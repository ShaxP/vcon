import vcon


class SampleGame(vcon.Game):
    def __init__(self):
        self.frame = 0
        self.player_x = 380.0
        self.player_y = 160.0
        self.boost = False

    def on_update(self, dt_fixed):
        self.frame += 1

        speed = 220.0
        self.player_x += vcon.input.axis("move_x") * speed * dt_fixed
        self.player_x = max(0.0, min(1200.0, self.player_x))

        self.boost = vcon.input.action_pressed("A")

    def on_render(self, alpha):
        vcon.graphics.clear((12, 16, 28, 255))
        vcon.graphics.rect(100, 120, 220, 140, (240, 96, 64, 255), filled=True)

        tint = (255, 220, 120, 255) if self.boost else (255, 255, 255, 255)
        vcon.graphics.sprite("hero", x=self.player_x, y=self.player_y, scale=2.0, color=tint)

        vcon.graphics.text(f"Frame: {self.frame}", 24, 24, size=24, color=(255, 255, 255, 255))
        vcon.graphics.text(
            f"move_x: {vcon.input.axis('move_x'):+.2f}",
            24,
            56,
            size=20,
            color=(180, 220, 255, 255),
        )
        vcon.graphics.text(
            f"A: {'down' if vcon.input.action_pressed('A') else 'up'}",
            24,
            84,
            size=20,
            color=(255, 210, 160, 255),
        )


cartridge = vcon.Cartridge(SampleGame())
