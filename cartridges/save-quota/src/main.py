import vcon


class SaveQuota(vcon.Game):
    def on_update(self, dt_fixed):
        # Intentionally exceed 1 MB quota.
        vcon.save.write("state", {"blob": "x" * (2 * 1024 * 1024)})


cartridge = vcon.Cartridge(SaveQuota())
