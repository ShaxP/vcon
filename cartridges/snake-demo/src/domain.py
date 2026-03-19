class GridPosition:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

    def translated(self, direction):
        return GridPosition(self.x + direction.x, self.y + direction.y)

    def __eq__(self, other) -> bool:
        return isinstance(other, GridPosition) and self.x == other.x and self.y == other.y

    def __hash__(self) -> int:
        return hash((self.x, self.y))


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


class InputState:
    def __init__(
        self,
        desired_direction,
        restart_pressed: bool,
        pause_toggled: bool,
    ):
        self.desired_direction = desired_direction
        self.restart_pressed = restart_pressed
        self.pause_toggled = pause_toggled
