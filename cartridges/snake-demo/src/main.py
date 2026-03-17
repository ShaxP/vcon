import vcon

GRID_COLUMNS = 60
GRID_ROWS = 32
INITIAL_SEED = 1337
INITIAL_SNAKE_SPEED = 50
MAX_SNAKE_SPEED = 100
SNAKE_SPEED_INCREMENT = 5


class Layout:
    def __init__(
        self,
        surface_width: int,
        surface_height: int,
        cell_size: int,
        board_x: int,
        board_y: int,
        board_width: int,
        board_height: int,
    ):
        self.surface_width = surface_width
        self.surface_height = surface_height
        self.cell_size = cell_size
        self.board_x = board_x
        self.board_y = board_y
        self.board_width = board_width
        self.board_height = board_height


class DeterministicRng:
    def __init__(self, seed: int):
        self.seed = seed

    def next_value(self) -> int:
        self.seed = (self.seed * 1664525 + 1013904223) & 0xFFFFFFFF
        return self.seed


class Snake:
    def __init__(self, start_x: int, start_y: int):
        self.body = [(start_x, start_y), (start_x - 1, start_y), (start_x - 2, start_y)]
        self.direction = (1, 0)
        self.queued_direction = (1, 0)
        self.speed_cells_per_second = INITIAL_SNAKE_SPEED

    @staticmethod
    def is_opposite(first_direction, second_direction) -> bool:
        return (
            first_direction[0] == -second_direction[0]
            and first_direction[1] == -second_direction[1]
        )

    def queue_direction(self, desired_direction) -> None:
        if desired_direction is None:
            return
        if not self.is_opposite(desired_direction, self.direction):
            self.queued_direction = desired_direction

    def prepare_move(self) -> None:
        if not self.is_opposite(self.direction, self.queued_direction):
            self.direction = self.queued_direction

    def next_head_position(self):
        head_x, head_y = self.body[0]
        return (head_x + self.direction[0], head_y + self.direction[1])

    def move_forward(self, next_head, grew: bool) -> None:
        self.body.insert(0, next_head)
        if not grew:
            self.body.pop()

    def increase_speed(self) -> None:
        self.speed_cells_per_second = min(
            MAX_SNAKE_SPEED,
            self.speed_cells_per_second + SNAKE_SPEED_INCREMENT,
        )

    @property
    def move_interval_seconds(self) -> float:
        return 1.0 / max(1, self.speed_cells_per_second)


class SnakeGame:
    def __init__(self):
        self.rng = DeterministicRng(INITIAL_SEED)
        self.best_score = 0
        self.frame_count = 0
        self.fps_instant = 0.0
        self.fps_smoothed = 0.0
        self.reset()

    def reset(self) -> None:
        center_x = GRID_COLUMNS // 2
        center_y = GRID_ROWS // 2
        self.snake = Snake(center_x, center_y)
        self.food_position = (0, 0)
        self.score = 0
        self.accumulated_time = 0.0
        self.game_over = False
        self.paused = False

        self.was_a_pressed = False
        self.was_start_pressed = False
        self.was_pause_pressed = False

        self.spawn_food()

    def spawn_food(self) -> None:
        occupied_cells = set(self.snake.body)
        while True:
            candidate_x = self.rng.next_value() % GRID_COLUMNS
            candidate_y = self.rng.next_value() % GRID_ROWS
            candidate_position = (candidate_x, candidate_y)
            if candidate_position not in occupied_cells:
                self.food_position = candidate_position
                return

    def read_desired_direction(self):
        move_axis_x = vcon.input.axis("move_x")
        move_axis_y = vcon.input.axis("move_y")

        if abs(move_axis_x) >= abs(move_axis_y) and abs(move_axis_x) > 0.35:
            return (1, 0) if move_axis_x > 0 else (-1, 0)
        if abs(move_axis_y) > 0.35:
            return (0, 1) if move_axis_y > 0 else (0, -1)
        return None

    def is_out_of_bounds(self, position) -> bool:
        x, y = position
        return x < 0 or x >= GRID_COLUMNS or y < 0 or y >= GRID_ROWS

    def step(self) -> None:
        if self.game_over:
            return

        self.snake.prepare_move()
        next_head = self.snake.next_head_position()

        hit_boundary = self.is_out_of_bounds(next_head)
        hit_self = next_head in self.snake.body
        if hit_boundary or hit_self:
            self.game_over = True
            self.best_score = max(self.best_score, self.score)
            return

        ate_food = next_head == self.food_position
        self.snake.move_forward(next_head, grew=ate_food)

        if ate_food:
            self.score += 1
            self.best_score = max(self.best_score, self.score)
            self.snake.increase_speed()
            self.spawn_food()

    def update(self, dt_seconds: float) -> None:
        self.frame_count += 1
        self.fps_instant = 1.0 / max(dt_seconds, 1e-6)
        if self.fps_smoothed <= 0.0:
            self.fps_smoothed = self.fps_instant
        else:
            self.fps_smoothed = (self.fps_smoothed * 0.9) + (self.fps_instant * 0.1)

        desired_direction = self.read_desired_direction()
        self.snake.queue_direction(desired_direction)

        is_a_pressed = vcon.input.action_pressed("A")
        is_start_pressed = vcon.input.action_pressed("Start")
        is_pause_pressed = vcon.input.action_pressed("Pause")

        restart_pressed = (is_a_pressed and not self.was_a_pressed) or (
            is_start_pressed and not self.was_start_pressed
        )
        pause_toggled = is_pause_pressed and not self.was_pause_pressed

        self.was_a_pressed = is_a_pressed
        self.was_start_pressed = is_start_pressed
        self.was_pause_pressed = is_pause_pressed

        if pause_toggled and not self.game_over:
            self.paused = not self.paused

        if self.game_over and restart_pressed:
            self.reset()
            return

        if self.paused:
            return

        self.accumulated_time += dt_seconds
        while self.accumulated_time >= self.snake.move_interval_seconds:
            self.accumulated_time -= self.snake.move_interval_seconds
            self.step()

    def compute_layout(self) -> Layout:
        surface_width, surface_height = vcon.graphics.surface_size()

        horizontal_padding = max(16, int(surface_width * 0.03))
        hud_top_height = max(64, int(surface_height * 0.11))
        hud_bottom_height = max(42, int(surface_height * 0.07))
        available_width = max(200, surface_width - (horizontal_padding * 2))
        available_height = max(160, surface_height - hud_top_height - hud_bottom_height)

        cell_size = max(
            8,
            int(min(available_width / GRID_COLUMNS, available_height / GRID_ROWS)),
        )
        board_width = cell_size * GRID_COLUMNS
        board_height = cell_size * GRID_ROWS
        board_x = int((surface_width - board_width) / 2)
        board_y = hud_top_height + int((available_height - board_height) / 2)

        return Layout(
            surface_width=surface_width,
            surface_height=surface_height,
            cell_size=cell_size,
            board_x=board_x,
            board_y=board_y,
            board_width=board_width,
            board_height=board_height,
        )

    @staticmethod
    def draw_rounded_rect(x: float, y: float, width: float, height: float, radius: float, color) -> None:
        corner_radius = max(0.0, min(radius, width * 0.5, height * 0.5))
        if corner_radius <= 0.0:
            vcon.graphics.rect(x, y, width, height, color, filled=True)
            return

        center_width = width - (corner_radius * 2.0)
        center_height = height - (corner_radius * 2.0)
        if center_width > 0.0:
            vcon.graphics.rect(x + corner_radius, y, center_width, height, color, filled=True)
        if center_height > 0.0:
            vcon.graphics.rect(x, y + corner_radius, width, center_height, color, filled=True)

        vcon.graphics.circle(x + corner_radius, y + corner_radius, corner_radius, color, filled=True)
        vcon.graphics.circle(
            x + width - corner_radius, y + corner_radius, corner_radius, color, filled=True
        )
        vcon.graphics.circle(
            x + corner_radius, y + height - corner_radius, corner_radius, color, filled=True
        )
        vcon.graphics.circle(
            x + width - corner_radius,
            y + height - corner_radius,
            corner_radius,
            color,
            filled=True,
        )

    def draw_head_eyes(
        self,
        center_x: float,
        center_y: float,
        head_width: float,
        head_height: float,
    ) -> None:
        direction_x, direction_y = self.snake.direction
        if direction_x == 0 and direction_y == 0:
            direction_x = 1

        forward_offset = min(head_width, head_height) * 0.16
        side_offset = min(head_width, head_height) * 0.18
        eye_radius = max(1.0, min(head_width, head_height) * 0.10)
        pupil_radius = max(1.0, eye_radius * 0.45)

        perpendicular_x = -direction_y
        perpendicular_y = direction_x

        left_eye_x = (
            center_x
            + (direction_x * forward_offset)
            + (perpendicular_x * side_offset)
        )
        left_eye_y = (
            center_y
            + (direction_y * forward_offset)
            + (perpendicular_y * side_offset)
        )
        right_eye_x = (
            center_x
            + (direction_x * forward_offset)
            - (perpendicular_x * side_offset)
        )
        right_eye_y = (
            center_y
            + (direction_y * forward_offset)
            - (perpendicular_y * side_offset)
        )

        pupil_forward = eye_radius * 0.28
        for eye_x, eye_y in ((left_eye_x, left_eye_y), (right_eye_x, right_eye_y)):
            vcon.graphics.circle(eye_x, eye_y, eye_radius, (245, 255, 245, 255), filled=True)
            vcon.graphics.circle(
                eye_x + (direction_x * pupil_forward),
                eye_y + (direction_y * pupil_forward),
                pupil_radius,
                (20, 30, 20, 255),
                filled=True,
            )

    def render(self) -> None:
        layout = self.compute_layout()

        vcon.graphics.clear((10, 14, 20, 255))
        vcon.graphics.rect(
            layout.board_x - 4,
            layout.board_y - 4,
            layout.board_width + 8,
            layout.board_height + 8,
            (90, 110, 130, 255),
            filled=False,
            thickness=3.0,
        )
        vcon.graphics.rect(
            layout.board_x,
            layout.board_y,
            layout.board_width,
            layout.board_height,
            (16, 24, 28, 255),
            filled=True,
        )

        food_inset = max(2, layout.cell_size // 5)
        food_x = layout.board_x + (self.food_position[0] * layout.cell_size)
        food_y = layout.board_y + (self.food_position[1] * layout.cell_size)
        vcon.graphics.rect(
            food_x + food_inset,
            food_y + food_inset,
            layout.cell_size - (food_inset * 2),
            layout.cell_size - (food_inset * 2),
            (230, 70, 80, 255),
        )

        for index, segment in enumerate(self.snake.body):
            draw_x = layout.board_x + (segment[0] * layout.cell_size)
            draw_y = layout.board_y + (segment[1] * layout.cell_size)
            segment_color = (90, 220, 140, 255) if index == 0 else (50, 165, 105, 255)
            segment_direction = self.snake.direction

            if index == 0:
                if abs(segment_direction[0]) >= abs(segment_direction[1]):
                    head_width = layout.cell_size * 1.04
                    head_height = layout.cell_size * 0.86
                else:
                    head_width = layout.cell_size * 0.86
                    head_height = layout.cell_size * 1.04

                head_x = draw_x + ((layout.cell_size - head_width) * 0.5)
                head_y = draw_y + ((layout.cell_size - head_height) * 0.5)
                self.draw_rounded_rect(
                    head_x,
                    head_y,
                    head_width,
                    head_height,
                    min(head_width, head_height) * 0.24,
                    segment_color,
                )
                self.draw_head_eyes(
                    head_x + (head_width * 0.5),
                    head_y + (head_height * 0.5),
                    head_width,
                    head_height,
                )
                continue

            previous_segment = self.snake.body[index - 1]
            next_segment = self.snake.body[index + 1] if index + 1 < len(self.snake.body) else None
            to_previous = (previous_segment[0] - segment[0], previous_segment[1] - segment[1])
            to_next = (
                (next_segment[0] - segment[0], next_segment[1] - segment[1])
                if next_segment is not None
                else None
            )
            is_bend = (
                to_next is not None
                and to_previous[0] != 0
                and to_next[1] != 0
            ) or (
                to_next is not None
                and to_previous[1] != 0
                and to_next[0] != 0
            )

            if is_bend:
                center_x = draw_x + (layout.cell_size * 0.5)
                center_y = draw_y + (layout.cell_size * 0.5)
                core_size = layout.cell_size * 0.68
                core_x = center_x - (core_size * 0.5)
                core_y = center_y - (core_size * 0.5)
                self.draw_rounded_rect(
                    core_x,
                    core_y,
                    core_size,
                    core_size,
                    core_size * 0.38,
                    segment_color,
                )

                arm_length = layout.cell_size * 0.56
                arm_thickness = layout.cell_size * 0.62
                for direction_x, direction_y in (to_previous, to_next):
                    if direction_x != 0:
                        arm_width = arm_length
                        arm_height = arm_thickness
                        arm_x = center_x + (direction_x * (layout.cell_size * 0.25)) - (arm_width * 0.5)
                        arm_y = center_y - (arm_height * 0.5)
                    else:
                        arm_width = arm_thickness
                        arm_height = arm_length
                        arm_x = center_x - (arm_width * 0.5)
                        arm_y = center_y + (direction_y * (layout.cell_size * 0.25)) - (arm_height * 0.5)
                    self.draw_rounded_rect(
                        arm_x,
                        arm_y,
                        arm_width,
                        arm_height,
                        min(arm_width, arm_height) * 0.42,
                        segment_color,
                    )
                continue

            straight_direction = to_previous if to_previous != (0, 0) else (1, 0)
            body_length_factor = 1.06
            body_thickness_factor = 0.70
            if abs(straight_direction[0]) >= abs(straight_direction[1]):
                segment_width = layout.cell_size * body_length_factor
                segment_height = layout.cell_size * body_thickness_factor
            else:
                segment_width = layout.cell_size * body_thickness_factor
                segment_height = layout.cell_size * body_length_factor

            body_x = draw_x + ((layout.cell_size - segment_width) * 0.5)
            body_y = draw_y + ((layout.cell_size - segment_height) * 0.5)
            self.draw_rounded_rect(
                body_x,
                body_y,
                segment_width,
                segment_height,
                min(segment_width, segment_height) * 0.35,
                segment_color,
            )

        title_y = max(16, layout.board_y - 62)
        stats_y = title_y + 34
        info_y = layout.board_y + layout.board_height + 12
        vcon.graphics.text(
            "SNAKE DEMO",
            layout.board_x,
            title_y,
            size=30,
            color=(235, 245, 255, 255),
        )
        vcon.graphics.text(
            f"Score: {self.score}   Best: {self.best_score}   Speed: {self.snake.speed_cells_per_second}",
            layout.board_x,
            stats_y,
            size=18,
            color=(180, 220, 255, 255),
        )
        vcon.graphics.text(
            f"FPS: {self.fps_instant:.1f} ({self.fps_smoothed:.1f})",
            layout.board_x + layout.board_width - 260,
            stats_y,
            size=18,
            color=(210, 230, 245, 255),
        )
        vcon.graphics.text(
            "Move: Arrows/WASD | Pause: P | Restart: Space/Enter",
            layout.board_x,
            info_y,
            size=16,
            color=(180, 200, 210, 255),
        )

        if self.game_over:
            panel_width = min(440, max(300, int(layout.board_width * 0.42)))
            panel_height = min(140, max(110, int(layout.board_height * 0.20)))
            panel_x = layout.board_x + (layout.board_width - panel_width) / 2.0
            panel_y = layout.board_y + (layout.board_height - panel_height) / 2.0
            vcon.graphics.rect(panel_x, panel_y, panel_width, panel_height, (0, 0, 0, 200), filled=True)
            vcon.graphics.rect(
                panel_x,
                panel_y,
                panel_width,
                panel_height,
                (220, 100, 90, 255),
                filled=False,
                thickness=2.0,
            )
            vcon.graphics.text(
                "GAME OVER",
                panel_x + 72,
                panel_y + 24,
                size=28,
                color=(255, 210, 210, 255),
            )
            vcon.graphics.text(
                "Press Space or Enter to restart",
                panel_x + 28,
                panel_y + panel_height - 44,
                size=18,
                color=(240, 240, 240, 255),
            )

        if self.paused and not self.game_over:
            panel_width = min(320, max(220, int(layout.board_width * 0.30)))
            panel_height = min(112, max(88, int(layout.board_height * 0.15)))
            panel_x = layout.board_x + (layout.board_width - panel_width) / 2.0
            panel_y = layout.board_y + (layout.board_height - panel_height) / 2.0
            vcon.graphics.rect(panel_x, panel_y, panel_width, panel_height, (0, 0, 0, 200), filled=True)
            vcon.graphics.rect(
                panel_x,
                panel_y,
                panel_width,
                panel_height,
                (90, 170, 230, 255),
                filled=False,
                thickness=2.0,
            )
            vcon.graphics.text(
                "PAUSED",
                panel_x + 56,
                panel_y + 20,
                size=30,
                color=(220, 240, 255, 255),
            )
            vcon.graphics.text(
                "Press P to resume",
                panel_x + 30,
                panel_y + panel_height - 34,
                size=16,
                color=(220, 230, 240, 255),
            )

class SnakeDemo(vcon.Game):
    def __init__(self):
        self.game = SnakeGame()

    def on_boot(self):
        self.game.reset()
        print(f"Snake demo render backend: {vcon.graphics.render_backend()}")
        return None

    def on_update(self, dt_fixed):
        self.game.update(dt_fixed)
        return None

    def on_render(self, alpha):
        self.game.render()
        return None

    def on_event(self, event):
        return None

    def on_shutdown(self):
        return None


cartridge = vcon.Cartridge(SnakeDemo())
