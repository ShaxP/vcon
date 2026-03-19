from config import (
    GRID_COLUMNS,
    GRID_ROWS,
    INITIAL_SEED,
    INITIAL_SNAKE_SPEED,
    MAX_SNAKE_SPEED,
    RIGHT,
    SNAKE_SPEED_INCREMENT,
)
from domain import GridPosition


class DeterministicRng:
    def __init__(self, seed: int):
        self.seed = seed

    def next_value(self) -> int:
        self.seed = (self.seed * 1664525 + 1013904223) & 0xFFFFFFFF
        return self.seed


class Snake:
    def __init__(self, start_x: int, start_y: int):
        self.body = [
            GridPosition(start_x, start_y),
            GridPosition(start_x - 1, start_y),
            GridPosition(start_x - 2, start_y),
        ]
        self.direction = RIGHT
        self.queued_direction = RIGHT
        self.speed_cells_per_second = INITIAL_SNAKE_SPEED

    @staticmethod
    def is_opposite(first_direction, second_direction) -> bool:
        return (
            first_direction.x == -second_direction.x
            and first_direction.y == -second_direction.y
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
        return self.body[0].translated(self.direction)

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


class GameSession:
    def __init__(self):
        self.rng = DeterministicRng(INITIAL_SEED)
        self.best_score = 0
        self.frame_count = 0
        self.fps_instant = 0.0
        self.fps_smoothed = 0.0

        self.snake = None
        self.food_position = GridPosition(0, 0)
        self.score = 0
        self.accumulated_time = 0.0

        self.reset_run()

    def reset_run(self) -> None:
        center_x = GRID_COLUMNS // 2
        center_y = GRID_ROWS // 2
        self.snake = Snake(center_x, center_y)
        self.food_position = GridPosition(0, 0)
        self.score = 0
        self.accumulated_time = 0.0
        self.spawn_food()

    def record_frame_timing(self, dt_seconds: float) -> None:
        self.frame_count += 1
        self.fps_instant = 1.0 / max(dt_seconds, 1e-6)
        if self.fps_smoothed <= 0.0:
            self.fps_smoothed = self.fps_instant
        else:
            self.fps_smoothed = (self.fps_smoothed * 0.9) + (self.fps_instant * 0.1)

    def spawn_food(self) -> None:
        occupied_cells = set(self.snake.body)
        while True:
            candidate = GridPosition(
                self.rng.next_value() % GRID_COLUMNS,
                self.rng.next_value() % GRID_ROWS,
            )
            if candidate not in occupied_cells:
                self.food_position = candidate
                return

    def is_out_of_bounds(self, position) -> bool:
        return (
            position.x < 0
            or position.x >= GRID_COLUMNS
            or position.y < 0
            or position.y >= GRID_ROWS
        )

    def step_snake(self) -> bool:
        self.snake.prepare_move()
        next_head = self.snake.next_head_position()

        hit_boundary = self.is_out_of_bounds(next_head)
        hit_self = next_head in self.snake.body
        if hit_boundary or hit_self:
            self.best_score = max(self.best_score, self.score)
            return False

        ate_food = next_head == self.food_position
        self.snake.move_forward(next_head, grew=ate_food)

        if ate_food:
            self.score += 1
            self.best_score = max(self.best_score, self.score)
            self.snake.increase_speed()
            self.spawn_food()

        return True
