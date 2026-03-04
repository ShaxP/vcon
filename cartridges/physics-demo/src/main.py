import vcon

_ticks = 0
_collisions = 0


def on_boot():
    vcon.physics.set_gravity(0.0, 0.0)



def on_update(dt_fixed):
    global _ticks
    _ticks += 1

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



def on_event(event):
    global _collisions
    if event.get("type") == "physics.collision":
        _collisions += 1



def on_render(alpha):
    vcon.graphics.clear((8, 10, 18, 255))

    left = vcon.physics.body("left")
    right = vcon.physics.body("right")

    if left:
        vcon.graphics.circle(left["x"], left["y"], left["radius"], (255, 140, 100, 255))
    if right:
        vcon.graphics.circle(right["x"], right["y"], right["radius"], (120, 220, 255, 255))

    vcon.graphics.text(f"ticks: {_ticks}", 24, 24, size=20, color=(255, 255, 255, 255))
    vcon.graphics.text(
        f"collisions: {_collisions}",
        24,
        50,
        size=20,
        color=(255, 230, 150, 255),
    )



def on_shutdown():
    return None
