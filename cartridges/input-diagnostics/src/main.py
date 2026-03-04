import vcon


def _bar(value, x, y, w, h, color_pos, color_neg):
    # Draw neutral baseline.
    vcon.graphics.rect(x, y, w, h, (48, 56, 72, 255), filled=True)

    mid = x + (w / 2.0)
    fill = (w / 2.0) * max(-1.0, min(1.0, value))
    if fill > 0:
        vcon.graphics.rect(mid, y, fill, h, color_pos, filled=True)
    elif fill < 0:
        vcon.graphics.rect(mid + fill, y, -fill, h, color_neg, filled=True)


def on_boot():
    return None


def on_update(dt_fixed):
    return None


def on_render(alpha):
    move_x = vcon.input.axis("move_x")
    move_y = vcon.input.axis("move_y")
    a_down = vcon.input.action_pressed("A")
    start_down = vcon.input.action_pressed("Start")

    vcon.graphics.clear((16, 20, 28, 255))
    vcon.graphics.text("Input Diagnostics", 24, 24, size=28, color=(255, 255, 255, 255))

    vcon.graphics.text(f"move_x: {move_x:+.2f}", 24, 76, size=18, color=(220, 230, 255, 255))
    _bar(move_x, 200, 78, 320, 20, (84, 200, 132, 255), (255, 142, 96, 255))

    vcon.graphics.text(f"move_y: {move_y:+.2f}", 24, 112, size=18, color=(220, 230, 255, 255))
    _bar(move_y, 200, 114, 320, 20, (84, 200, 132, 255), (255, 142, 96, 255))

    vcon.graphics.text(f"A: {'down' if a_down else 'up'}", 24, 156, size=18, color=(255, 230, 168, 255))
    vcon.graphics.text(
        f"Start: {'down' if start_down else 'up'}",
        24,
        184,
        size=18,
        color=(255, 230, 168, 255),
    )

    button_color = (255, 210, 120, 255) if a_down else (88, 98, 118, 255)
    vcon.graphics.circle(560, 168, 28, button_color, filled=True)
    vcon.graphics.text("A", 552, 161, size=16, color=(24, 24, 24, 255))

    return None


def on_event(event):
    return None


def on_shutdown():
    return None
