"""Input API backed by runtime-injected frame state."""

_axes = {}
_actions = {}


def _set_runtime_state(axes, actions):
    global _axes, _actions
    _axes = dict(axes)
    _actions = dict(actions)


def action_pressed(name):
    return bool(_actions.get(name, False))


def axis(name):
    value = float(_axes.get(name, 0.0))
    if value > 1.0:
        return 1.0
    if value < -1.0:
        return -1.0
    return value
