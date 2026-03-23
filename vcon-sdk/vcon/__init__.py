"""VCON SDK package for object-oriented cartridge entrypoints."""

from abc import ABC

from . import audio, fsm, graphics, input, physics, save


class Game(ABC):
    """Base game contract for cartridges."""

    def on_boot(self) -> None:
        pass

    def on_update(self, dt_fixed: float) -> None:
        pass

    def on_render(self, alpha: float) -> None:
        pass

    def on_event(self, event: dict) -> None:
        pass

    def on_shutdown(self) -> None:
        pass


class Cartridge:
    """Runtime-facing cartridge wrapper around a game instance."""

    def __init__(self, game: Game):
        self.game = game

    def on_boot(self) -> None:
        self.game.on_boot()

    def on_update(self, dt_fixed: float) -> None:
        self.game.on_update(dt_fixed)

    def on_render(self, alpha: float) -> None:
        self.game.on_render(alpha)

    def on_event(self, event: dict) -> None:
        self.game.on_event(event)

    def on_shutdown(self) -> None:
        self.game.on_shutdown()


__all__ = ["Cartridge", "Game", "audio", "fsm", "graphics", "input", "physics", "save"]
