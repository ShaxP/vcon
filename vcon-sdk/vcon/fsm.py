"""Optional finite state machine helpers for cartridges."""


class State:
    """Base class for a cartridge state."""

    name = "state"

    def __init__(self, context, machine):
        self.context = context
        self.machine = machine

    def enter(self, previous_state_name=None):
        return None

    def exit(self, next_state_name=None):
        return None

    def update(self, dt_seconds: float):
        return None

    def render(self, alpha: float):
        return None

    def on_event(self, event):
        return None


class StateMachine:
    """Minimal cartridge-side state machine with explicit transitions."""

    def __init__(self, context):
        self.context = context
        self.current_state = None
        self.current_state_name = None

    def change_state(self, next_state) -> None:
        previous_state_name = self.current_state_name
        if self.current_state is not None:
            self.current_state.exit(next_state.name if next_state is not None else None)
        self.current_state = next_state
        self.current_state_name = next_state.name if next_state is not None else None
        if self.current_state is not None:
            self.current_state.enter(previous_state_name)

    def update(self, dt_seconds: float) -> None:
        if self.current_state is None:
            return
        self.current_state.update(dt_seconds)

    def render(self, alpha: float) -> None:
        if self.current_state is None:
            return
        self.current_state.render(alpha)

    def on_event(self, event) -> None:
        if self.current_state is None:
            return
        self.current_state.on_event(event)
