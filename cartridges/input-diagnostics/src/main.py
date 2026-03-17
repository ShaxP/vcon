import vcon


def _bar(value, x, y, w, h, color_pos, color_neg):
    # Draw neutral baseline.
    vcon.graphics.rect(x, y, w, h, (48, 56, 72, 255), filled=True)

    mid = x + (w / 2.0)
    fill = (w / 2.0) * max(-1.0, min(1.0, value))
    if fill > 0:
        vcon.graphics.rect(mid, y, fill, h, color_pos, filled=True)
    elif fill < 0:
        vcon.graphics.rect(mid + fill, y, -fill, h, color_neg, filled=True)


class InputDiagnostics(vcon.Game):
    def on_render(self, alpha):
        move_x = vcon.input.axis("move_x")
        move_y = vcon.input.axis("move_y")
        look_x = vcon.input.axis("look_x")
        look_y = vcon.input.axis("look_y")
        dpad_x = vcon.input.axis("dpad_x")
        dpad_y = vcon.input.axis("dpad_y")
        trigger_l = vcon.input.axis("trigger_l")
        trigger_r = vcon.input.axis("trigger_r")

        a_down = vcon.input.action_pressed("A")
        b_down = vcon.input.action_pressed("B")
        x_down = vcon.input.action_pressed("X")
        y_down = vcon.input.action_pressed("Y")
        l1_down = vcon.input.action_pressed("L1")
        r1_down = vcon.input.action_pressed("R1")
        l2_down = vcon.input.action_pressed("L2")
        r2_down = vcon.input.action_pressed("R2")
        dpad_up = vcon.input.action_pressed("DPadUp")
        dpad_down = vcon.input.action_pressed("DPadDown")
        dpad_left = vcon.input.action_pressed("DPadLeft")
        dpad_right = vcon.input.action_pressed("DPadRight")
        start_down = vcon.input.action_pressed("Start")
        select_down = vcon.input.action_pressed("Select")

        connected = vcon.input.action_pressed("ControllerConnectedState")
        just_connected = vcon.input.action_pressed("ControllerConnected")
        just_disconnected = vcon.input.action_pressed("ControllerDisconnected")
        just_reconnected = vcon.input.action_pressed("ControllerReconnected")

        vcon.graphics.clear((16, 20, 28, 255))
        vcon.graphics.text("Input Diagnostics", 24, 24, size=28, color=(255, 255, 255, 255))
        vcon.graphics.text(
            f"controller: {'connected' if connected else 'disconnected'}",
            24,
            52,
            size=16,
            color=(240, 246, 255, 255),
        )

        if just_reconnected:
            event_text = "event: reconnected"
        elif just_connected:
            event_text = "event: connected"
        elif just_disconnected:
            event_text = "event: disconnected"
        else:
            event_text = "event: none"
        vcon.graphics.text(event_text, 360, 52, size=16, color=(240, 246, 255, 255))

        vcon.graphics.text(f"move_x: {move_x:+.2f}", 24, 82, size=18, color=(220, 230, 255, 255))
        _bar(move_x, 200, 84, 280, 18, (84, 200, 132, 255), (255, 142, 96, 255))
        vcon.graphics.text(f"move_y: {move_y:+.2f}", 24, 108, size=18, color=(220, 230, 255, 255))
        _bar(move_y, 200, 110, 280, 18, (84, 200, 132, 255), (255, 142, 96, 255))

        vcon.graphics.text(f"look_x: {look_x:+.2f}", 24, 138, size=18, color=(220, 230, 255, 255))
        _bar(look_x, 200, 140, 280, 18, (84, 200, 132, 255), (255, 142, 96, 255))
        vcon.graphics.text(f"look_y: {look_y:+.2f}", 24, 164, size=18, color=(220, 230, 255, 255))
        _bar(look_y, 200, 166, 280, 18, (84, 200, 132, 255), (255, 142, 96, 255))

        vcon.graphics.text(f"dpad_x: {dpad_x:+.0f}", 24, 194, size=18, color=(220, 230, 255, 255))
        vcon.graphics.text(f"dpad_y: {dpad_y:+.0f}", 170, 194, size=18, color=(220, 230, 255, 255))
        vcon.graphics.text(
            f"dpad: U={int(dpad_up)} D={int(dpad_down)} L={int(dpad_left)} R={int(dpad_right)}",
            300,
            194,
            size=14,
            color=(220, 230, 255, 255),
        )

        vcon.graphics.text(
            f"triggers: L={trigger_l:+.2f} ({'down' if l2_down else 'up'})  R={trigger_r:+.2f} ({'down' if r2_down else 'up'})",
            24,
            218,
            size=14,
            color=(220, 230, 255, 255),
        )

        vcon.graphics.text(
            f"ABXY: A={int(a_down)} B={int(b_down)} X={int(x_down)} Y={int(y_down)}",
            24,
            240,
            size=16,
            color=(255, 230, 168, 255),
        )
        vcon.graphics.text(
            f"shoulders: L1={int(l1_down)} R1={int(r1_down)}",
            24,
            262,
            size=16,
            color=(255, 230, 168, 255),
        )
        vcon.graphics.text(
            f"menu: Start={int(start_down)} Select={int(select_down)}",
            24,
            284,
            size=18,
            color=(255, 230, 168, 255),
        )

        button_color = (255, 210, 120, 255) if a_down else (88, 98, 118, 255)
        vcon.graphics.circle(560, 250, 24, button_color, filled=True)
        vcon.graphics.text("A", 553, 244, size=14, color=(24, 24, 24, 255))

        return None


cartridge = vcon.Cartridge(InputDiagnostics())
