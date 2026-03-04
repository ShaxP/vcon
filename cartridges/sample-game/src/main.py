import vcon

_frame = 0
_player_x = 380.0
_player_y = 160.0
_boost = False


def on_boot():
    return None


def on_update(dt_fixed):
    global _frame, _player_x, _boost
    _frame += 1

    speed = 220.0
    _player_x += vcon.input.axis("move_x") * speed * dt_fixed
    _player_x = max(0.0, min(1200.0, _player_x))

    _boost = vcon.input.action_pressed("A")
    return None


def on_render(alpha):
    vcon.graphics.clear((12, 16, 28, 255))
    vcon.graphics.rect(100, 120, 220, 140, (240, 96, 64, 255), filled=True)

    tint = (255, 220, 120, 255) if _boost else (255, 255, 255, 255)
    vcon.graphics.sprite("hero", x=_player_x, y=_player_y, scale=2.0, color=tint)

    vcon.graphics.text(f"Frame: {_frame}", 24, 24, size=24, color=(255, 255, 255, 255))
    vcon.graphics.text(
        f"move_x: {vcon.input.axis('move_x'):+.2f}",
        24,
        56,
        size=20,
        color=(180, 220, 255, 255),
    )
    vcon.graphics.text(
        f"A: {'down' if vcon.input.action_pressed('A') else 'up'}",
        24,
        84,
        size=20,
        color=(255, 210, 160, 255),
    )
    return None


def on_event(event):
    return None


def on_shutdown():
    return None
