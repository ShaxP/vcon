"""Graphics command buffer API.

Cartridges call these functions during `on_render`; runtime drains commands
for engine-side validation and submission.
"""

_frame_commands = []
_surface_width = 1280
_surface_height = 800


def _set_runtime_state(surface_width, surface_height):
    global _surface_width, _surface_height
    _surface_width = int(surface_width)
    _surface_height = int(surface_height)


def begin_frame():
    _frame_commands.clear()


def drain_commands():
    commands = list(_frame_commands)
    _frame_commands.clear()
    return commands


def clear(color):
    _frame_commands.append({"kind": "clear", "color": tuple(color)})


def line(x1, y1, x2, y2, color, thickness=1.0):
    _frame_commands.append(
        {
            "kind": "line",
            "x1": float(x1),
            "y1": float(y1),
            "x2": float(x2),
            "y2": float(y2),
            "color": tuple(color),
            "thickness": float(thickness),
        }
    )


def rect(x, y, w, h, color, filled=True, thickness=1.0):
    _frame_commands.append(
        {
            "kind": "rect",
            "x": float(x),
            "y": float(y),
            "w": float(w),
            "h": float(h),
            "color": tuple(color),
            "filled": bool(filled),
            "thickness": float(thickness),
        }
    )


def circle(x, y, r, color, filled=True, thickness=1.0):
    _frame_commands.append(
        {
            "kind": "circle",
            "x": float(x),
            "y": float(y),
            "r": float(r),
            "color": tuple(color),
            "filled": bool(filled),
            "thickness": float(thickness),
        }
    )


def sprite(asset_id, x, y, rotation=0.0, scale=1.0, color=(255, 255, 255, 255)):
    _frame_commands.append(
        {
            "kind": "sprite",
            "asset_id": str(asset_id),
            "x": float(x),
            "y": float(y),
            "rotation": float(rotation),
            "scale": float(scale),
            "color": tuple(color),
        }
    )


def text(value, x, y, size=16.0, color=(255, 255, 255, 255)):
    _frame_commands.append(
        {
            "kind": "text",
            "value": str(value),
            "x": float(x),
            "y": float(y),
            "size": float(size),
            "color": tuple(color),
        }
    )


def surface_width():
    return _surface_width


def surface_height():
    return _surface_height


def surface_size():
    return (_surface_width, _surface_height)
