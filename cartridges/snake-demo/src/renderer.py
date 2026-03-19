from config import GRID_COLUMNS, GRID_ROWS, RIGHT
from domain import GridPosition, Layout

import vcon


class SnakeRenderer:
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
    def draw_rounded_rect(
        x: float,
        y: float,
        width: float,
        height: float,
        radius: float,
        color,
    ) -> None:
        corner_radius = max(0.0, min(radius, width * 0.5, height * 0.5))
        if corner_radius <= 0.0:
            vcon.graphics.rect(x, y, width, height, color, filled=True)
            return

        center_width = width - (corner_radius * 2.0)
        center_height = height - (corner_radius * 2.0)
        if center_width > 0.0:
            vcon.graphics.rect(
                x + corner_radius,
                y,
                center_width,
                height,
                color,
                filled=True,
            )
        if center_height > 0.0:
            vcon.graphics.rect(
                x,
                y + corner_radius,
                width,
                center_height,
                color,
                filled=True,
            )

        vcon.graphics.circle(x + corner_radius, y + corner_radius, corner_radius, color, filled=True)
        vcon.graphics.circle(
            x + width - corner_radius,
            y + corner_radius,
            corner_radius,
            color,
            filled=True,
        )
        vcon.graphics.circle(
            x + corner_radius,
            y + height - corner_radius,
            corner_radius,
            color,
            filled=True,
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
        direction,
        center_x: float,
        center_y: float,
        head_width: float,
        head_height: float,
    ) -> None:
        direction_x = direction.x
        direction_y = direction.y
        if direction_x == 0 and direction_y == 0:
            direction_x = 1

        forward_offset = min(head_width, head_height) * 0.16
        side_offset = min(head_width, head_height) * 0.18
        eye_radius = max(1.0, min(head_width, head_height) * 0.10)
        pupil_radius = max(1.0, eye_radius * 0.45)

        perpendicular_x = -direction_y
        perpendicular_y = direction_x

        left_eye_x = center_x + (direction_x * forward_offset) + (perpendicular_x * side_offset)
        left_eye_y = center_y + (direction_y * forward_offset) + (perpendicular_y * side_offset)
        right_eye_x = center_x + (direction_x * forward_offset) - (perpendicular_x * side_offset)
        right_eye_y = center_y + (direction_y * forward_offset) - (perpendicular_y * side_offset)

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

    def draw_board(self, layout) -> None:
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

    def draw_food(self, layout, food_position) -> None:
        food_inset = max(2, layout.cell_size // 5)
        food_x = layout.board_x + (food_position.x * layout.cell_size)
        food_y = layout.board_y + (food_position.y * layout.cell_size)
        vcon.graphics.rect(
            food_x + food_inset,
            food_y + food_inset,
            layout.cell_size - (food_inset * 2),
            layout.cell_size - (food_inset * 2),
            (230, 70, 80, 255),
        )

    @staticmethod
    def is_bend(to_previous, to_next) -> bool:
        return (
            to_next is not None
            and to_previous.x != 0
            and to_next.y != 0
        ) or (
            to_next is not None
            and to_previous.y != 0
            and to_next.x != 0
        )

    def draw_snake_head(
        self,
        layout,
        direction,
        draw_x: float,
        draw_y: float,
        color,
    ) -> None:
        if abs(direction.x) >= abs(direction.y):
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
            color,
        )
        self.draw_head_eyes(
            direction,
            head_x + (head_width * 0.5),
            head_y + (head_height * 0.5),
            head_width,
            head_height,
        )

    def draw_snake_bend(
        self,
        layout,
        draw_x: float,
        draw_y: float,
        to_previous,
        to_next,
        color,
    ) -> None:
        if to_next is None:
            return

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
            color,
        )

        arm_length = layout.cell_size * 0.56
        arm_thickness = layout.cell_size * 0.62
        for direction in (to_previous, to_next):
            if direction.x != 0:
                arm_width = arm_length
                arm_height = arm_thickness
                arm_x = center_x + (direction.x * (layout.cell_size * 0.25)) - (arm_width * 0.5)
                arm_y = center_y - (arm_height * 0.5)
            else:
                arm_width = arm_thickness
                arm_height = arm_length
                arm_x = center_x - (arm_width * 0.5)
                arm_y = center_y + (direction.y * (layout.cell_size * 0.25)) - (arm_height * 0.5)
            self.draw_rounded_rect(
                arm_x,
                arm_y,
                arm_width,
                arm_height,
                min(arm_width, arm_height) * 0.42,
                color,
            )

    def draw_snake_straight(
        self,
        layout,
        draw_x: float,
        draw_y: float,
        straight_direction,
        color,
    ) -> None:
        body_length_factor = 1.06
        body_thickness_factor = 0.70
        if abs(straight_direction.x) >= abs(straight_direction.y):
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
            color,
        )

    def draw_snake(self, layout, snake) -> None:
        for index, segment in enumerate(snake.body):
            draw_x = layout.board_x + (segment.x * layout.cell_size)
            draw_y = layout.board_y + (segment.y * layout.cell_size)
            segment_color = (90, 220, 140, 255) if index == 0 else (50, 165, 105, 255)

            if index == 0:
                self.draw_snake_head(layout, snake.direction, draw_x, draw_y, segment_color)
                continue

            previous_segment = snake.body[index - 1]
            next_segment = snake.body[index + 1] if index + 1 < len(snake.body) else None
            to_previous = GridPosition(
                previous_segment.x - segment.x,
                previous_segment.y - segment.y,
            )
            to_next = (
                GridPosition(next_segment.x - segment.x, next_segment.y - segment.y)
                if next_segment is not None
                else None
            )

            if self.is_bend(to_previous, to_next):
                self.draw_snake_bend(layout, draw_x, draw_y, to_previous, to_next, segment_color)
                continue

            straight_direction = to_previous if to_previous != GridPosition(0, 0) else RIGHT
            self.draw_snake_straight(layout, draw_x, draw_y, straight_direction, segment_color)

    def draw_hud(self, layout, session) -> None:
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
            f"Score: {session.score}   Best: {session.best_score}   Speed: {session.snake.speed_cells_per_second}",
            layout.board_x,
            stats_y,
            size=18,
            color=(180, 220, 255, 255),
        )
        vcon.graphics.text(
            f"FPS: {session.fps_instant:.1f} ({session.fps_smoothed:.1f})",
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

    def render_world(self, session) -> None:
        layout = self.compute_layout()
        self.draw_board(layout)
        self.draw_food(layout, session.food_position)
        self.draw_snake(layout, session.snake)
        self.draw_hud(layout, session)

    def render_game_over_overlay(self) -> None:
        layout = self.compute_layout()
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

    def render_paused_overlay(self) -> None:
        layout = self.compute_layout()
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
