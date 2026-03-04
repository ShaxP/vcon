"""Lifecycle callback stub for VCON SDK V1."""

import vcon


def on_boot():
    vcon.physics.set_gravity(0.0, 0.0)


def on_update(dt_fixed):
    # Keep a body alive in physics world.
    vcon.physics.upsert_body("player", x=100.0, y=100.0, radius=12.0, dynamic=True)


def on_render(alpha):
    vcon.graphics.clear((20, 24, 36, 255))
    player = vcon.physics.body("player")
    if player:
        vcon.graphics.circle(player["x"], player["y"], player["radius"], (120, 230, 255, 255))


def on_event(event):
    if event.get("type") == "physics.collision":
        pass


def on_shutdown():
    pass
