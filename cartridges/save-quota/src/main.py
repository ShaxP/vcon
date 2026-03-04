import vcon


def on_boot():
    return None


def on_update(dt_fixed):
    # Intentionally exceed 1 MB quota.
    vcon.save.write("state", {"blob": "x" * (2 * 1024 * 1024)})


def on_render(alpha):
    return None


def on_shutdown():
    return None
