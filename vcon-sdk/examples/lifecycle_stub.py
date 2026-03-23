"""Lifecycle callback stub for VCON SDK V2."""

import vcon


class PhysicsStub(vcon.Game):
    def on_boot(self) -> None:
        vcon.physics.set_gravity(0.0, 0.0)

    def on_update(self, dt_fixed) -> None:
        # Keep a body alive in physics world.
        vcon.physics.upsert_body("player", x=100.0, y=100.0, radius=12.0, dynamic=True)

    def on_render(self, alpha) -> None:
        vcon.graphics.clear((20, 24, 36, 255))
        player = vcon.physics.body("player")
        if player:
            vcon.graphics.circle(player["x"], player["y"], player["radius"], (120, 230, 255, 255))

    def on_event(self, event) -> None:
        if event.get("type") == "physics.collision":
            pass

    def on_shutdown(self) -> None:
        pass

cartridge = vcon.Cartridge(PhysicsStub())
