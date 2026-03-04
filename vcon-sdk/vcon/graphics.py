"""Graphics command buffer API.

Cartridges call these functions during `on_render`; runtime drains commands
for engine-side validation and submission.
"""

_frame_commands = []


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
