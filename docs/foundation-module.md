# Foundation Module Documentation

Status: Implemented (Milestone 1 baseline)  
Last updated: 2026-02-28

## Purpose
The foundation module establishes the minimum runnable Virtual Console platform:
- cartridge manifest parsing and validation
- sandbox policy checks
- boot path for cartridge loading
- Python lifecycle execution in runtime
- save namespace model
- packaging validation command

This is the current baseline before rendering/input/physics subsystems are added.

## Module Boundaries

### `vcon-engine`
Core foundation logic shared by runtime and tooling.

- [lib.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/lib.rs)
  - Exposes public foundation APIs (`Manifest`, `boot_cartridge`).

- [manifest.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/manifest.rs)
  - Parses `vcon.toml` into `Manifest`.
  - Validates required keys and base constraints.

- [sandbox.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/sandbox.rs)
  - Enforces baseline policy checks:
    - blocked `network` permission
    - blocked network-related imports
    - blocked non-SDK imports

- [storage.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/storage.rs)
  - Builds per-game save namespace using manifest `id`.
  - Applies slot path safety checks.

- [host.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/host.rs)
  - End-to-end boot path:
    - load and validate manifest
    - run policy checks
    - read entrypoint
    - detect lifecycle callback availability
    - initialize save namespace metadata

### `vcon-runtime`
Executable host for running cartridges.

- [main.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/main.rs)
  - CLI entrypoint.
  - Calls `boot_cartridge` and runs cartridge lifecycle loop.

- [python_host.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/python_host.rs)
  - Embeds Python with `pyo3`.
  - Installs runtime import guard for sandbox policy at execution time.
  - Executes callbacks:
    - `on_boot()`
    - `on_update(dt_fixed)` for `N` frames
    - `on_render(alpha)` for `N` frames
    - `on_shutdown()`

### `vcon-pack`
Validation CLI for cartridge foundation checks.

- [main.rs](/Users/shahram/source/repos/codex/vcon/vcon-pack/src/main.rs)
  - `validate` command checks manifest + permissions + entrypoint file presence.

### `vcon-sdk` (placeholder)
Current SDK placeholder for cartridge imports.

- [__init__.py](/Users/shahram/source/repos/codex/vcon/vcon-sdk/vcon/__init__.py)
- [graphics.py](/Users/shahram/source/repos/codex/vcon/vcon-sdk/vcon/graphics.py)
- [input.py](/Users/shahram/source/repos/codex/vcon/vcon-sdk/vcon/input.py)
- [save.py](/Users/shahram/source/repos/codex/vcon/vcon-sdk/vcon/save.py)

### `cartridges/sample-game`
Baseline cartridge used for smoke/integration testing.

- [vcon.toml](/Users/shahram/source/repos/codex/vcon/cartridges/sample-game/vcon.toml)
- [main.py](/Users/shahram/source/repos/codex/vcon/cartridges/sample-game/src/main.py)

## Runtime CLI

### `vcon-runtime`
Command:
```bash
cargo run -p vcon-runtime -- \
  --cartridge cartridges/sample-game \
  --saves-root /tmp/vcon/saves \
  --sdk-root vcon-sdk \
  --frames 60 \
  --dt-fixed 0.0166666667
```

Key options:
- `--cartridge`: cartridge root path
- `--saves-root`: base save directory root
- `--sdk-root`: SDK import root
- `--frames`: number of loop iterations
- `--dt-fixed`: fixed timestep passed to `on_update`

Current output includes lifecycle invocation and update/render call counts.

### `vcon-pack validate`
Command:
```bash
cargo run -p vcon-pack -- validate --cartridge cartridges/sample-game
```

## Test Coverage

Unit tests:
- [manifest.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/manifest.rs)
- [sandbox.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/sandbox.rs)
- [storage.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/storage.rs)
- [host.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/host.rs)
- [python_host.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/python_host.rs)

Integration tests:
- [foundation_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/tests/foundation_smoke.rs)
- [runtime_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/tests/runtime_smoke.rs)
- [validate_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-pack/tests/validate_smoke.rs)

Test index:
- [tests/README.md](/Users/shahram/source/repos/codex/vcon/tests/README.md)

Run all tests:
```bash
cargo test --workspace
```

## Current Limits
- No windowed rendering output yet.
- No real input backend yet.
- No audio subsystem yet.
- Save API persistence plumbing is namespace-level only (engine metadata foundation in place).
- SDK modules are placeholders pending full Milestone 2+ implementation.

## Next Work (after foundation)
- Draw command pipeline (`vcon.graphics` -> engine command buffer).
- Input backend + action/axis mapping.
- Real save slot I/O API.
- Deterministic loop expansion and render backend integration.
