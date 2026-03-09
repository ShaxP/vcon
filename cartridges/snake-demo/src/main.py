import vcon

GRID_COLS = 60
GRID_ROWS = 32

_snake = []
_snake_prev = []
_direction = (1, 0)
_next_direction = (1, 0)
_food = (0, 0)
_rng = 1337
_score = 0
_best_score = 0
_tick_accumulator = 0.0
_tick_seconds = 0.08
_frame = 0
_game_over = False
_paused = False
_a_prev = False
_start_prev = False
_pause_prev = False
_fps_instant = 0.0
_fps_smooth = 0.0


def _reset_game():
    global _snake, _snake_prev, _direction, _next_direction
    global _score, _tick_accumulator, _tick_seconds, _game_over, _paused
    global _a_prev, _start_prev, _pause_prev, _fps_instant, _fps_smooth
    head_x = GRID_COLS // 2
    head_y = GRID_ROWS // 2
    _snake = [(head_x, head_y), (head_x - 1, head_y), (head_x - 2, head_y)]
    _snake_prev = list(_snake)
    _direction = (1, 0)
    _next_direction = (1, 0)
    _score = 0
    _tick_accumulator = 0.0
    _tick_seconds = 0.045
    _game_over = False
    _paused = False
    _a_prev = False
    _start_prev = False
    _pause_prev = False
    _fps_instant = 0.0
    _fps_smooth = 0.0
    _spawn_food()


def _next_rand():
    global _rng
    _rng = (_rng * 1664525 + 1013904223) & 0xFFFFFFFF
    return _rng


def _spawn_food():
    global _food
    occupied = set(_snake)
    while True:
        x = _next_rand() % GRID_COLS
        y = _next_rand() % GRID_ROWS
        if (x, y) not in occupied:
            _food = (x, y)
            return


def _is_opposite(a, b):
    return a[0] == -b[0] and a[1] == -b[1]


def _read_direction():
    move_x = vcon.input.axis("move_x")
    move_y = vcon.input.axis("move_y")

    if abs(move_x) >= abs(move_y) and abs(move_x) > 0.35:
        return (1, 0) if move_x > 0 else (-1, 0)
    if abs(move_y) > 0.35:
        return (0, 1) if move_y > 0 else (0, -1)
    return None


def _compute_layout():
    surface_w, surface_h = vcon.graphics.surface_size()

    pad_x = max(16, int(surface_w * 0.03))
    hud_top = max(64, int(surface_h * 0.11))
    hud_bottom = max(42, int(surface_h * 0.07))
    available_w = max(200, surface_w - (pad_x * 2))
    available_h = max(160, surface_h - hud_top - hud_bottom)

    cell_size = max(8, int(min(available_w / GRID_COLS, available_h / GRID_ROWS)))
    board_w = cell_size * GRID_COLS
    board_h = cell_size * GRID_ROWS
    board_x = int((surface_w - board_w) / 2)
    board_y = hud_top + int((available_h - board_h) / 2)

    return {
        "surface_w": surface_w,
        "surface_h": surface_h,
        "cell_size": cell_size,
        "board_x": board_x,
        "board_y": board_y,
        "board_w": board_w,
        "board_h": board_h,
    }


def _step_snake():
    global _snake, _snake_prev, _direction, _next_direction
    global _score, _best_score, _tick_seconds, _game_over

    if _game_over:
        return

    _snake_prev = list(_snake)

    if not _is_opposite(_direction, _next_direction):
        _direction = _next_direction

    head_x, head_y = _snake[0]
    next_head = (head_x + _direction[0], head_y + _direction[1])

    out_of_bounds = (
        next_head[0] < 0
        or next_head[0] >= GRID_COLS
        or next_head[1] < 0
        or next_head[1] >= GRID_ROWS
    )
    if out_of_bounds or next_head in _snake:
        _game_over = True
        _best_score = max(_best_score, _score)
        return

    _snake.insert(0, next_head)
    if next_head == _food:
        _score += 1
        _best_score = max(_best_score, _score)
        _tick_seconds = max(0.02, _tick_seconds - 0.008)
        _spawn_food()
    else:
        _snake.pop()


def on_boot():
    _reset_game()
    return None


def on_update(dt_fixed):
    global _frame, _tick_accumulator, _next_direction, _a_prev, _start_prev
    global _pause_prev, _paused, _fps_instant, _fps_smooth
    _frame += 1
    _fps_instant = 1.0 / max(dt_fixed, 1e-6)
    if _fps_smooth <= 0.0:
        _fps_smooth = _fps_instant
    else:
        _fps_smooth = (_fps_smooth * 0.9) + (_fps_instant * 0.1)

    desired = _read_direction()
    if desired is not None and not _is_opposite(desired, _direction):
        _next_direction = desired

    a_now = vcon.input.action_pressed("A")
    start_now = vcon.input.action_pressed("Start")
    pause_now = vcon.input.action_pressed("Pause")
    restart_pressed = (a_now and not _a_prev) or (start_now and not _start_prev)
    pause_pressed = pause_now and not _pause_prev
    _a_prev = a_now
    _start_prev = start_now
    _pause_prev = pause_now

    if pause_pressed and not _game_over:
        _paused = not _paused

    if _game_over and restart_pressed:
        _reset_game()
        return None

    if _paused:
        return None

    _tick_accumulator += dt_fixed
    while _tick_accumulator >= _tick_seconds:
        _tick_accumulator -= _tick_seconds
        _step_snake()

    return None


def on_render(alpha):
    layout = _compute_layout()
    cell_size = layout["cell_size"]
    board_x = layout["board_x"]
    board_y = layout["board_y"]
    board_w = layout["board_w"]
    board_h = layout["board_h"]

    vcon.graphics.clear((10, 14, 20, 255))

    vcon.graphics.rect(
        board_x - 4,
        board_y - 4,
        board_w + 8,
        board_h + 8,
        (90, 110, 130, 255),
        filled=False,
        thickness=3.0,
    )
    vcon.graphics.rect(board_x, board_y, board_w, board_h, (16, 24, 28, 255), filled=True)

    food_inset = max(2, cell_size // 5)
    food_x = board_x + (_food[0] * cell_size)
    food_y = board_y + (_food[1] * cell_size)
    vcon.graphics.rect(
        food_x + food_inset,
        food_y + food_inset,
        cell_size - (food_inset * 2),
        cell_size - (food_inset * 2),
        (230, 70, 80, 255),
    )

    blend = 0.0 if _game_over else min(1.0, _tick_accumulator / max(_tick_seconds, 1e-6))
    for idx, part in enumerate(_snake):
        prev_part = _snake_prev[idx] if idx < len(_snake_prev) else part
        interp_x = prev_part[0] + ((part[0] - prev_part[0]) * blend)
        interp_y = prev_part[1] + ((part[1] - prev_part[1]) * blend)
        x = board_x + (interp_x * cell_size)
        y = board_y + (interp_y * cell_size)
        color = (90, 220, 140, 255) if idx == 0 else (50, 165, 105, 255)
        inset = max(1, cell_size // (8 if idx == 0 else 6))
        vcon.graphics.rect(x + inset, y + inset, cell_size - (inset * 2), cell_size - (inset * 2), color)

    title_y = max(16, board_y - 62)
    stats_y = title_y + 34
    info_y = board_y + board_h + 12
    vcon.graphics.text("SNAKE DEMO", board_x, title_y, size=30, color=(235, 245, 255, 255))
    vcon.graphics.text(
        f"Score: {_score}   Best: {_best_score}   Speed: {int((1.0 / _tick_seconds) + 0.5)}",
        board_x,
        stats_y,
        size=18,
        color=(180, 220, 255, 255),
    )
    vcon.graphics.text(
        f"FPS: {_fps_instant:.1f} ({_fps_smooth:.1f})",
        board_x + board_w - 260,
        stats_y,
        size=18,
        color=(210, 230, 245, 255),
    )
    vcon.graphics.text(
        "Move: Arrows/WASD | Pause: P | Restart: Space/Enter",
        board_x,
        info_y,
        size=16,
        color=(180, 200, 210, 255),
    )

    if _game_over:
        panel_w = min(440, max(300, int(board_w * 0.42)))
        panel_h = min(140, max(110, int(board_h * 0.20)))
        panel_x = board_x + (board_w - panel_w) / 2.0
        panel_y = board_y + (board_h - panel_h) / 2.0
        vcon.graphics.rect(panel_x, panel_y, panel_w, panel_h, (0, 0, 0, 200), filled=True)
        vcon.graphics.rect(
            panel_x, panel_y, panel_w, panel_h, (220, 100, 90, 255), filled=False, thickness=2.0
        )
        vcon.graphics.text("GAME OVER", panel_x + 72, panel_y + 24, size=28, color=(255, 210, 210, 255))
        vcon.graphics.text(
            "Press Space or Enter to restart",
            panel_x + 28,
            panel_y + panel_h - 44,
            size=18,
            color=(240, 240, 240, 255),
        )

    if _paused and not _game_over:
        panel_w = min(320, max(220, int(board_w * 0.30)))
        panel_h = min(112, max(88, int(board_h * 0.15)))
        panel_x = board_x + (board_w - panel_w) / 2.0
        panel_y = board_y + (board_h - panel_h) / 2.0
        vcon.graphics.rect(panel_x, panel_y, panel_w, panel_h, (0, 0, 0, 200), filled=True)
        vcon.graphics.rect(
            panel_x, panel_y, panel_w, panel_h, (90, 170, 230, 255), filled=False, thickness=2.0
        )
        vcon.graphics.text("PAUSED", panel_x + 56, panel_y + 20, size=30, color=(220, 240, 255, 255))
        vcon.graphics.text(
            "Press P to resume",
            panel_x + 30,
            panel_y + panel_h - 34,
            size=16,
            color=(220, 230, 240, 255),
        )

    return None


def on_event(event):
    return None


def on_shutdown():
    return None
