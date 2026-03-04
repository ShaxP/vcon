use std::collections::{HashMap, HashSet};

use crate::scene::{NodeId, PhysicsBody2D, PhysicsBodyKind, SceneGraph};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsVec2 {
    pub x: f64,
    pub y: f64,
}

impl PhysicsVec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionEvent {
    pub a: NodeId,
    pub b: NodeId,
}

#[derive(Debug, Clone)]
struct BodyState {
    kind: PhysicsBodyKind,
    position: PhysicsVec2,
    velocity: PhysicsVec2,
    radius: f64,
    restitution: f64,
}

#[derive(Debug, Clone)]
pub struct PhysicsWorld {
    gravity: PhysicsVec2,
    bodies: HashMap<NodeId, BodyState>,
    active_collisions: HashSet<(NodeId, NodeId)>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            gravity: PhysicsVec2::new(0.0, 0.0),
            bodies: HashMap::new(),
            active_collisions: HashSet::new(),
        }
    }

    pub fn set_gravity(&mut self, gravity: PhysicsVec2) {
        self.gravity = gravity;
    }

    pub fn gravity(&self) -> PhysicsVec2 {
        self.gravity
    }

    pub fn sync_from_scene(&mut self, scene: &SceneGraph) {
        let mut seen = HashSet::new();

        for node in scene.nodes() {
            let Some(body) = node.physics_body.as_ref() else {
                continue;
            };

            seen.insert(node.id);
            let entry = self.bodies.entry(node.id).or_insert_with(|| BodyState {
                kind: body.kind,
                position: PhysicsVec2::new(node.transform.x, node.transform.y),
                velocity: PhysicsVec2::new(body.velocity_x, body.velocity_y),
                radius: body.radius,
                restitution: body.restitution,
            });

            entry.kind = body.kind;
            entry.position = PhysicsVec2::new(node.transform.x, node.transform.y);
            entry.velocity = PhysicsVec2::new(body.velocity_x, body.velocity_y);
            entry.radius = body.radius;
            entry.restitution = body.restitution;
        }

        self.bodies.retain(|id, _| seen.contains(id));
        self.active_collisions
            .retain(|(a, b)| seen.contains(a) && seen.contains(b));
    }

    pub fn step(&mut self, dt_fixed: f64) -> Vec<CollisionEvent> {
        for body in self.bodies.values_mut() {
            if matches!(body.kind, PhysicsBodyKind::Dynamic) {
                body.velocity.x += self.gravity.x * dt_fixed;
                body.velocity.y += self.gravity.y * dt_fixed;
                body.position.x += body.velocity.x * dt_fixed;
                body.position.y += body.velocity.y * dt_fixed;
            }
        }

        let mut ids = self.bodies.keys().copied().collect::<Vec<_>>();
        ids.sort_by_key(|id| id.0);

        let mut collisions_now = HashSet::new();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a_id = ids[i];
                let b_id = ids[j];
                if self.resolve_collision(a_id, b_id) {
                    collisions_now.insert((a_id, b_id));
                }
            }
        }

        let mut events = Vec::new();
        for pair in &collisions_now {
            if !self.active_collisions.contains(pair) {
                events.push(CollisionEvent {
                    a: pair.0,
                    b: pair.1,
                });
            }
        }
        events.sort_by_key(|event| (event.a.0, event.b.0));

        self.active_collisions = collisions_now;
        events
    }

    pub fn apply_to_scene(&self, scene: &mut SceneGraph) {
        for (id, state) in &self.bodies {
            let _ = scene.update_physics_state(
                *id,
                state.position.x,
                state.position.y,
                state.velocity.x,
                state.velocity.y,
            );
        }
    }

    fn resolve_collision(&mut self, a_id: NodeId, b_id: NodeId) -> bool {
        let Some(a_state) = self.bodies.get(&a_id).cloned() else {
            return false;
        };
        let Some(b_state) = self.bodies.get(&b_id).cloned() else {
            return false;
        };

        let dx = b_state.position.x - a_state.position.x;
        let dy = b_state.position.y - a_state.position.y;
        let radius_sum = a_state.radius + b_state.radius;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq > radius_sum * radius_sum {
            return false;
        }

        let dist = dist_sq.sqrt();
        let (nx, ny) = if dist > 1e-9 {
            (dx / dist, dy / dist)
        } else {
            (1.0, 0.0)
        };

        let penetration = (radius_sum - dist).max(0.0);

        let mut a_new = a_state.clone();
        let mut b_new = b_state.clone();

        match (a_state.kind, b_state.kind) {
            (PhysicsBodyKind::Dynamic, PhysicsBodyKind::Dynamic) => {
                a_new.position.x -= nx * penetration * 0.5;
                a_new.position.y -= ny * penetration * 0.5;
                b_new.position.x += nx * penetration * 0.5;
                b_new.position.y += ny * penetration * 0.5;
            }
            (PhysicsBodyKind::Dynamic, PhysicsBodyKind::Static) => {
                a_new.position.x -= nx * penetration;
                a_new.position.y -= ny * penetration;
            }
            (PhysicsBodyKind::Static, PhysicsBodyKind::Dynamic) => {
                b_new.position.x += nx * penetration;
                b_new.position.y += ny * penetration;
            }
            (PhysicsBodyKind::Static, PhysicsBodyKind::Static) => {}
        }

        let rvx = b_state.velocity.x - a_state.velocity.x;
        let rvy = b_state.velocity.y - a_state.velocity.y;
        let vel_along_normal = rvx * nx + rvy * ny;

        if vel_along_normal < 0.0 {
            let e = ((a_state.restitution + b_state.restitution) * 0.5).clamp(0.0, 1.0);
            let inv_mass_a = if matches!(a_state.kind, PhysicsBodyKind::Dynamic) {
                1.0
            } else {
                0.0
            };
            let inv_mass_b = if matches!(b_state.kind, PhysicsBodyKind::Dynamic) {
                1.0
            } else {
                0.0
            };

            let denom = inv_mass_a + inv_mass_b;
            if denom > 0.0 {
                let j = -(1.0 + e) * vel_along_normal / denom;
                let impulse_x = j * nx;
                let impulse_y = j * ny;

                if inv_mass_a > 0.0 {
                    a_new.velocity.x -= impulse_x * inv_mass_a;
                    a_new.velocity.y -= impulse_y * inv_mass_a;
                }
                if inv_mass_b > 0.0 {
                    b_new.velocity.x += impulse_x * inv_mass_b;
                    b_new.velocity.y += impulse_y * inv_mass_b;
                }
            }
        }

        self.bodies.insert(a_id, a_new);
        self.bodies.insert(b_id, b_new);
        true
    }
}

pub fn upsert_scene_body(
    scene: &mut SceneGraph,
    node: NodeId,
    x: f64,
    y: f64,
    body: PhysicsBody2D,
) -> Result<(), crate::scene::SceneError> {
    scene.set_node_transform(node, x, y, 0.0, 1.0, 1.0)?;
    scene.set_physics_body(node, body)
}

#[cfg(test)]
mod tests {
    use crate::scene::{NodeId, PhysicsBody2D, PhysicsBodyKind, SceneGraph};

    use super::{PhysicsVec2, PhysicsWorld};

    #[test]
    fn collisions_emit_once_while_contact_persists() {
        let mut scene = SceneGraph::new();
        let root = scene.root();
        let a = scene.add_node(root, "a").expect("node a");
        let b = scene.add_node(root, "b").expect("node b");

        scene
            .set_node_transform(a, 0.0, 0.0, 0.0, 1.0, 1.0)
            .expect("a transform");
        scene
            .set_node_transform(b, 10.0, 0.0, 0.0, 1.0, 1.0)
            .expect("b transform");
        scene
            .set_physics_body(
                a,
                PhysicsBody2D {
                    kind: PhysicsBodyKind::Dynamic,
                    radius: 8.0,
                    velocity_x: 120.0,
                    velocity_y: 0.0,
                    restitution: 1.0,
                },
            )
            .expect("a body");
        scene
            .set_physics_body(
                b,
                PhysicsBody2D {
                    kind: PhysicsBodyKind::Static,
                    radius: 8.0,
                    velocity_x: 0.0,
                    velocity_y: 0.0,
                    restitution: 1.0,
                },
            )
            .expect("b body");

        let mut world = PhysicsWorld::new();
        world.set_gravity(PhysicsVec2::new(0.0, 0.0));

        world.sync_from_scene(&scene);
        let first = world.step(1.0 / 60.0);
        world.apply_to_scene(&mut scene);

        world.sync_from_scene(&scene);
        let second = world.step(1.0 / 60.0);

        assert_eq!(first.len(), 1, "first contact should emit one event");
        assert!(second.is_empty(), "persistent contact should not re-emit");
    }

    #[test]
    fn fixed_step_is_deterministic() {
        fn run() -> (f64, f64) {
            let mut scene = SceneGraph::new();
            let root = scene.root();
            let a = scene.add_node(root, "a").expect("node a");
            let b = scene.add_node(root, "b").expect("node b");

            scene
                .set_node_transform(a, -40.0, 0.0, 0.0, 1.0, 1.0)
                .expect("a transform");
            scene
                .set_node_transform(b, 0.0, 0.0, 0.0, 1.0, 1.0)
                .expect("b transform");
            scene
                .set_physics_body(
                    a,
                    PhysicsBody2D {
                        kind: PhysicsBodyKind::Dynamic,
                        radius: 8.0,
                        velocity_x: 180.0,
                        velocity_y: 0.0,
                        restitution: 0.8,
                    },
                )
                .expect("a body");
            scene
                .set_physics_body(
                    b,
                    PhysicsBody2D {
                        kind: PhysicsBodyKind::Static,
                        radius: 8.0,
                        velocity_x: 0.0,
                        velocity_y: 0.0,
                        restitution: 0.8,
                    },
                )
                .expect("b body");

            let mut world = PhysicsWorld::new();
            world.set_gravity(PhysicsVec2::new(0.0, 0.0));

            for _ in 0..120 {
                world.sync_from_scene(&scene);
                let _ = world.step(1.0 / 60.0);
                world.apply_to_scene(&mut scene);
            }

            let node = scene.node(NodeId(1)).expect("node a exists");
            (node.transform.x, node.transform.y)
        }

        assert_eq!(run(), run());
    }
}
