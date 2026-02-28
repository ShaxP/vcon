"""Graphics API placeholder.

The engine side will execute submitted draw commands in Milestone 2.
"""


def clear(color):
    return ("clear", color)


def line(x1, y1, x2, y2, color, thickness=1.0):
    return ("line", x1, y1, x2, y2, color, thickness)


def rect(x, y, w, h, color, filled=True, thickness=1.0):
    return ("rect", x, y, w, h, color, filled, thickness)


def circle(x, y, r, color, filled=True, thickness=1.0):
    return ("circle", x, y, r, color, filled, thickness)


def sprite(asset_id, x, y, rotation=0.0, scale=1.0, color=(255, 255, 255, 255)):
    return ("sprite", asset_id, x, y, rotation, scale, color)


def text(value, x, y, size=16.0, color=(255, 255, 255, 255)):
    return ("text", value, x, y, size, color)
