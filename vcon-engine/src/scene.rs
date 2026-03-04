use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsBodyKind {
    Static,
    Dynamic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsBody2D {
    pub kind: PhysicsBodyKind,
    pub radius: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub restitution: f64,
}

impl Default for PhysicsBody2D {
    fn default() -> Self {
        Self {
            kind: PhysicsBodyKind::Dynamic,
            radius: 16.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            restitution: 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transform2D {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneNode {
    pub id: NodeId,
    pub name: String,
    pub enabled: bool,
    pub transform: Transform2D,
    pub physics_body: Option<PhysicsBody2D>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct SceneGraph {
    nodes: HashMap<NodeId, SceneNode>,
    root: NodeId,
    next_id: u32,
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneGraph {
    pub fn new() -> Self {
        let root = NodeId(0);
        let mut nodes = HashMap::new();
        nodes.insert(
            root,
            SceneNode {
                id: root,
                name: "root".to_owned(),
                enabled: true,
                transform: Transform2D::default(),
                physics_body: None,
                parent: None,
                children: Vec::new(),
            },
        );

        Self {
            nodes,
            root,
            next_id: 1,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn add_node(
        &mut self,
        parent: NodeId,
        name: impl Into<String>,
    ) -> Result<NodeId, SceneError> {
        let parent_node = self
            .nodes
            .get_mut(&parent)
            .ok_or(SceneError::ParentMissing(parent))?;

        let id = NodeId(self.next_id);
        self.next_id += 1;

        parent_node.children.push(id);
        self.nodes.insert(
            id,
            SceneNode {
                id,
                name: name.into(),
                enabled: true,
                transform: Transform2D::default(),
                physics_body: None,
                parent: Some(parent),
                children: Vec::new(),
            },
        );

        Ok(id)
    }

    pub fn node(&self, id: NodeId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }

    pub fn set_enabled(&mut self, id: NodeId, enabled: bool) -> Result<(), SceneError> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(SceneError::NodeMissing(id))?;
        node.enabled = enabled;
        Ok(())
    }

    pub fn set_node_transform(
        &mut self,
        id: NodeId,
        x: f64,
        y: f64,
        rotation: f64,
        scale_x: f64,
        scale_y: f64,
    ) -> Result<(), SceneError> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(SceneError::NodeMissing(id))?;
        node.transform = Transform2D {
            x,
            y,
            rotation,
            scale_x,
            scale_y,
        };
        Ok(())
    }

    pub fn set_physics_body(&mut self, id: NodeId, body: PhysicsBody2D) -> Result<(), SceneError> {
        if body.radius <= 0.0 {
            return Err(SceneError::InvalidPhysicsBody(
                "physics body radius must be greater than 0".to_owned(),
            ));
        }
        if !(0.0..=1.0).contains(&body.restitution) {
            return Err(SceneError::InvalidPhysicsBody(
                "physics body restitution must be within [0.0, 1.0]".to_owned(),
            ));
        }

        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(SceneError::NodeMissing(id))?;
        node.physics_body = Some(body);
        Ok(())
    }

    pub fn clear_physics_body(&mut self, id: NodeId) -> Result<(), SceneError> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(SceneError::NodeMissing(id))?;
        node.physics_body = None;
        Ok(())
    }

    pub fn update_physics_state(
        &mut self,
        id: NodeId,
        x: f64,
        y: f64,
        velocity_x: f64,
        velocity_y: f64,
    ) -> Result<(), SceneError> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(SceneError::NodeMissing(id))?;
        node.transform.x = x;
        node.transform.y = y;
        if let Some(body) = node.physics_body.as_mut() {
            body.velocity_x = velocity_x;
            body.velocity_y = velocity_y;
        }
        Ok(())
    }

    pub fn nodes(&self) -> impl Iterator<Item = &SceneNode> {
        self.nodes.values()
    }

    pub fn update_order(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.collect_enabled_dfs(self.root, &mut out);
        out
    }

    fn collect_enabled_dfs(&self, id: NodeId, out: &mut Vec<NodeId>) {
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        if !node.enabled {
            return;
        }

        out.push(id);
        for child in &node.children {
            self.collect_enabled_dfs(*child, out);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error("parent node missing: {0:?}")]
    ParentMissing(NodeId),
    #[error("scene node missing: {0:?}")]
    NodeMissing(NodeId),
    #[error("invalid physics body: {0}")]
    InvalidPhysicsBody(String),
}

#[cfg(test)]
mod tests {
    use super::{NodeId, PhysicsBody2D, PhysicsBodyKind, SceneGraph};

    #[test]
    fn builds_hierarchy_and_preserves_update_order() {
        let mut scene = SceneGraph::new();
        let root = scene.root();
        let a = scene.add_node(root, "a").expect("node a");
        let b = scene.add_node(root, "b").expect("node b");
        let a1 = scene.add_node(a, "a1").expect("node a1");

        let order = scene.update_order();
        assert_eq!(order, vec![root, a, a1, b]);
    }

    #[test]
    fn disabled_branch_is_skipped() {
        let mut scene = SceneGraph::new();
        let root = scene.root();
        let a = scene.add_node(root, "a").expect("node a");
        let a1 = scene.add_node(a, "a1").expect("node a1");

        scene.set_enabled(a, false).expect("disable a");

        let order = scene.update_order();
        assert_eq!(order, vec![root]);
        assert!(scene.node(a1).is_some());
    }

    #[test]
    fn missing_parent_fails() {
        let mut scene = SceneGraph::new();
        let err = scene
            .add_node(NodeId(777), "bad")
            .expect_err("missing parent should fail");
        assert!(err.to_string().contains("parent node missing"));
    }

    #[test]
    fn supports_physics_component_assignment() {
        let mut scene = SceneGraph::new();
        let node = scene.add_node(scene.root(), "physics").expect("node");

        scene
            .set_physics_body(
                node,
                PhysicsBody2D {
                    kind: PhysicsBodyKind::Dynamic,
                    radius: 12.0,
                    velocity_x: 20.0,
                    velocity_y: -5.0,
                    restitution: 0.8,
                },
            )
            .expect("assign body");

        let body = scene
            .node(node)
            .expect("node exists")
            .physics_body
            .as_ref()
            .expect("body exists");
        assert_eq!(body.radius, 12.0);
    }
}
