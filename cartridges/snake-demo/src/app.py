from config import DOWN, INPUT_AXIS_THRESHOLD, LEFT, RIGHT, UP
from domain import InputState
from model import GameSession
from renderer import SnakeRenderer

import vcon


class InputTracker:
    def __init__(self):
        self._was_a_pressed = False
        self._was_start_pressed = False
        self._was_pause_pressed = False

    def capture(self) -> InputState:
        is_a_pressed = vcon.input.action_pressed("A")
        is_start_pressed = vcon.input.action_pressed("Start")
        is_pause_pressed = vcon.input.action_pressed("Pause")

        restart_pressed = (is_a_pressed and not self._was_a_pressed) or (
            is_start_pressed and not self._was_start_pressed
        )
        pause_toggled = is_pause_pressed and not self._was_pause_pressed

        self._was_a_pressed = is_a_pressed
        self._was_start_pressed = is_start_pressed
        self._was_pause_pressed = is_pause_pressed

        return InputState(
            desired_direction=self.read_desired_direction(),
            restart_pressed=restart_pressed,
            pause_toggled=pause_toggled,
        )

    @staticmethod
    def read_desired_direction():
        move_axis_x = vcon.input.axis("move_x")
        move_axis_y = vcon.input.axis("move_y")

        if abs(move_axis_x) >= abs(move_axis_y) and abs(move_axis_x) > INPUT_AXIS_THRESHOLD:
            return RIGHT if move_axis_x > 0 else LEFT
        if abs(move_axis_y) > INPUT_AXIS_THRESHOLD:
            return DOWN if move_axis_y > 0 else UP
        return None


class SnakeAppContext:
    def __init__(self):
        self.session = GameSession()
        self.renderer = SnakeRenderer()
        self.input_tracker = InputTracker()
        self.input_state = InputState(
            desired_direction=None,
            restart_pressed=False,
            pause_toggled=False,
        )

    def begin_frame(self, dt_seconds: float) -> None:
        self.session.record_frame_timing(dt_seconds)
        self.input_state = self.input_tracker.capture()


class PlayingState(vcon.fsm.State):
    name = "playing"

    def update(self, dt_seconds: float):
        frame_input = self.context.input_state
        self.context.session.snake.queue_direction(frame_input.desired_direction)

        if frame_input.pause_toggled:
            self.machine.change_state(PausedState(self.context, self.machine))
            return

        self.context.session.accumulated_time += dt_seconds
        while self.context.session.accumulated_time >= self.context.session.snake.move_interval_seconds:
            self.context.session.accumulated_time -= self.context.session.snake.move_interval_seconds
            if not self.context.session.step_snake():
                self.machine.change_state(GameOverState(self.context, self.machine))
                return

    def render(self, alpha: float):
        self.context.renderer.render_world(self.context.session)


class PausedState(vcon.fsm.State):
    name = "paused"

    def update(self, dt_seconds: float):
        if self.context.input_state.pause_toggled:
            self.machine.change_state(PlayingState(self.context, self.machine))

    def render(self, alpha: float):
        self.context.renderer.render_world(self.context.session)
        self.context.renderer.render_paused_overlay()


class GameOverState(vcon.fsm.State):
    name = "game_over"

    def update(self, dt_seconds: float):
        if self.context.input_state.restart_pressed:
            self.context.session.reset_run()
            self.machine.change_state(PlayingState(self.context, self.machine))

    def render(self, alpha: float):
        self.context.renderer.render_world(self.context.session)
        self.context.renderer.render_game_over_overlay()


class SnakeDemo(vcon.Game):
    def __init__(self):
        self.context = SnakeAppContext()
        self.machine = vcon.fsm.StateMachine(self.context)

    def on_boot(self):
        self.context.session.reset_run()
        self.machine.change_state(PlayingState(self.context, self.machine))
        print(f"Snake demo render backend: {vcon.graphics.render_backend()}")

    def on_update(self, dt_fixed):
        self.context.begin_frame(dt_fixed)
        self.machine.update(dt_fixed)

    def on_render(self, alpha):
        self.machine.render(alpha)

    def on_event(self, event):
        self.machine.on_event(event)

    def on_shutdown(self):
        pass


cartridge = vcon.Cartridge(SnakeDemo())
