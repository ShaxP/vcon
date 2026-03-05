import vcon
from vcon import save


def on_boot():
    state = save.read("state")
    if not isinstance(state, dict):
        state = {"counter": 0}

    counter = int(state.get("counter", 0)) + 1
    save.write("state", {"counter": counter})
