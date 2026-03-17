import vcon


class PhysicsDemo(vcon.Game):
    def __init__(self):
        self.ticks = 0
        self.collisions = 0

    def on_boot(self):
        vcon.physics.set_gravity(0.0, 0.0)

    def on_update(self, dt_fixed):
        self.ticks += 1

        vcon.physics.upsert_body(
            "left",
            x=220.0,
            y=220.0,
            vx=180.0,
            vy=0.0,
            radius=18.0,
            dynamic=True,
            restitution=1.0,
        )
        vcon.physics.upsert_body(
            "right",
            x=320.0,
            y=220.0,
            vx=0.0,
            vy=0.0,
            radius=18.0,
            dynamic=False,
            restitution=1.0,
        )

    def on_event(self, event):
        if event.get("type") == "physics.collision":
            self.collisions += 1

    def on_render(self, alpha):
        vcon.graphics.clear((8, 10, 18, 255))

        left = vcon.physics.body("left")
        right = vcon.physics.body("right")

        if left:
            vcon.graphics.circle(left["x"], left["y"], left["radius"], (255, 140, 100, 255))
        if right:
            vcon.graphics.circle(right["x"], right["y"], right["radius"], (120, 220, 255, 255))

        vcon.graphics.text(f"ticks: {self.ticks}", 24, 24, size=20, color=(255, 255, 255, 255))
        vcon.graphics.text(
            f"collisions: {self.collisions}",
            24,
            50,
            size=20,
            color=(255, 230, 150, 255),
        )


cartridge = vcon.Cartridge(PhysicsDemo())
