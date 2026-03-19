"""VCON SDK package for object-oriented cartridge entrypoints."""

from abc import ABC

from . import audio, fsm, graphics, input, physics, save


class Game(ABC):
    """Base game contract for cartridges."""

    def on_boot(self):
        return None

    def on_update(self, dt_fixed: float):
        return None

    def on_render(self, alpha: float):
        return None

    def on_event(self, event: dict):
        return None

    def on_shutdown(self):
        return None


class Cartridge:
    """Runtime-facing cartridge wrapper around a game instance."""

    def __init__(self, game: Game):
        self.game = game

    def on_boot(self):
        return self.game.on_boot()

    def on_update(self, dt_fixed: float):
        return self.game.on_update(dt_fixed)

    def on_render(self, alpha: float):
        return self.game.on_render(alpha)

    def on_event(self, event: dict):
        return self.game.on_event(event)

    def on_shutdown(self):
        return self.game.on_shutdown()


__all__ = ["Cartridge", "Game", "audio", "fsm", "graphics", "input", "physics", "save"]
