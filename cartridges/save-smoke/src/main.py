import vcon

_state = {"counter": 0}


def on_boot():
    global _state
    loaded = vcon.save.read("state")
    if isinstance(loaded, dict):
        _state = loaded


def on_update(dt_fixed):
    _state["counter"] = int(_state.get("counter", 0)) + 1
    vcon.save.write("state", _state)


def on_render(alpha):
    vcon.graphics.clear((8, 12, 20, 255))
    vcon.graphics.text(f"counter: {_state['counter']}", 24, 24, size=24, color=(255, 255, 255, 255))


def on_shutdown():
    return None
