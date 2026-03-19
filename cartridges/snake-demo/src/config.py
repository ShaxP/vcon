from domain import GridPosition

GRID_COLUMNS = 60
GRID_ROWS = 32

INITIAL_SEED = 1337
INITIAL_SNAKE_SPEED = 50
MAX_SNAKE_SPEED = 100
SNAKE_SPEED_INCREMENT = 5

INPUT_AXIS_THRESHOLD = 0.35

RIGHT = GridPosition(1, 0)
LEFT = GridPosition(-1, 0)
DOWN = GridPosition(0, 1)
UP = GridPosition(0, -1)
