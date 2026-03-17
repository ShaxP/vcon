import vcon


class SaveSmoke(vcon.Game):
    def __init__(self):
        self.state = {"counter": 0}

    def on_boot(self):
        loaded = vcon.save.read("state")
        if isinstance(loaded, dict):
            self.state = loaded

    def on_update(self, dt_fixed):
        self.state["counter"] = int(self.state.get("counter", 0)) + 1
        vcon.save.write("state", self.state)

    def on_render(self, alpha):
        vcon.graphics.clear((8, 12, 20, 255))
        vcon.graphics.text(
            f"counter: {self.state['counter']}",
            24,
            24,
            size=24,
            color=(255, 255, 255, 255),
        )


cartridge = vcon.Cartridge(SaveSmoke())
