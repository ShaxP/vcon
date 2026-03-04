"""Deterministic physics API backed by runtime fixed-step integration."""

_gravity = (0.0, 0.0)
_desired_bodies = {}
_runtime_bodies = {}


def set_gravity(x, y):
    global _gravity
    _gravity = (float(x), float(y))


def upsert_body(
    name,
    x,
    y,
    vx=0.0,
    vy=0.0,
    radius=16.0,
    dynamic=True,
    restitution=0.5,
):
    key = str(name)
    if not key:
        raise ValueError("body name must be non-empty")

    radius = float(radius)
    if radius <= 0.0:
        raise ValueError("radius must be greater than 0")

    restitution = float(restitution)
    if restitution < 0.0 or restitution > 1.0:
        raise ValueError("restitution must be in [0.0, 1.0]")

    _desired_bodies[key] = {
        "name": key,
        "x": float(x),
        "y": float(y),
        "vx": float(vx),
        "vy": float(vy),
        "radius": radius,
        "dynamic": bool(dynamic),
        "restitution": restitution,
    }


def remove_body(name):
    _desired_bodies.pop(str(name), None)
    _runtime_bodies.pop(str(name), None)


def body(name):
    key = str(name)
    value = _runtime_bodies.get(key) or _desired_bodies.get(key)
    if value is None:
        return None
    return dict(value)


def list_bodies():
    return [dict(_runtime_bodies[name]) for name in sorted(_runtime_bodies)]


def _export_runtime_state():
    ordered = sorted(_desired_bodies)
    return {
        "gravity": _gravity,
        "bodies": [dict(_desired_bodies[name]) for name in ordered],
    }


def _set_runtime_state(gravity, bodies):
    global _gravity, _runtime_bodies
    gx, gy = gravity
    _gravity = (float(gx), float(gy))
    next_runtime = {}
    for body in bodies:
        name = str(body["name"])
        next_runtime[name] = {
            "name": name,
            "x": float(body["x"]),
            "y": float(body["y"]),
            "vx": float(body["vx"]),
            "vy": float(body["vy"]),
            "radius": float(body["radius"]),
        }
    _runtime_bodies = next_runtime
