# Foundation + Core Engine Documentation

Status: Implemented through Milestone 5 (with defined hardware/backend gaps)  
Last updated: 2026-03-04

## Purpose
This document captures the shipped implementation state after Milestone 5 work:
- Milestone 1 foundation (manifest/sandbox/boot path)
- deterministic runtime loop wiring
- SDK-driven rendering command pipeline
- input state injection and profile mapping baseline
- audio mixer scaffolding
- deterministic packaging/validation tooling
- hardening coverage for sandbox, render golden snapshots, frame pacing smoke checks, and save corruption recovery

It replaces the earlier "foundation only" baseline.

## Module Boundaries

### `vcon-engine`
Core engine logic shared by runtime and tooling.

- [lib.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/lib.rs)
  - Exposes public engine APIs (`Manifest`, boot reports, render/input/audio/scene primitives).

- [manifest.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/manifest.rs)
  - Parses `vcon.toml` into `Manifest`.
  - Validates required keys and base constraints.

- [sandbox.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/sandbox.rs)
  - Enforces baseline policy checks:
    - blocked `network` permission
    - blocked network-related imports
    - blocked non-SDK imports
    - blocked dynamic import patterns (`__import__`, `importlib.import_module`)

- [storage.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/storage.rs)
  - Builds per-game save namespace using manifest `id`.
  - Applies slot path safety checks.

- [render.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/render.rs)
  - Defines validated `DrawCommand` model and `FrameCommandBuffer`.
  - Implements software rendering for:
    - `clear`
    - `line`
    - `rect`
    - `circle`
    - `sprite` (PPM texture assets)
    - `text` (built-in font atlas)
  - Supports frame dump to PPM for determinism and snapshot checks.

- [input.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/input.rs)
  - Defines canonical `InputFrame` action/axis state.
  - Includes deterministic scripted input source.

- [input_mapping.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/input_mapping.rs)
  - Maps raw gamepad state into canonical actions/axes.
  - Provides `Desktop` and `SteamDeck` deadzone profiles.

- [audio.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/audio.rs)
  - Implements mixer scaffolding:
    - queue SFX/music requests
    - activate voices on flush
    - stop voice/all voices

- [scene.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/scene.rs)
  - Implements Scene + Node hierarchy model.
  - Supports DFS update ordering and branch enable/disable.

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
  - Calls `boot_cartridge` and runs runtime loop with configurable:
    - frame count
    - fixed timestep
    - surface resolution
    - input source (`none`, `scripted`, `gamepad`)
    - scripted input seed (`--input-seed`) for replay scenarios
    - optional final frame dump path

- [python_host.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/python_host.rs)
  - Embeds Python with `pyo3`.
  - Installs runtime import guard for sandbox policy at execution time.
  - Injects runtime input state into `vcon.input`.
  - Injects save runtime state into `vcon.save`.
  - Runs render command lifecycle per frame:
    - `vcon.graphics.begin_frame()`
    - game `on_render(alpha)`
    - `vcon.graphics.drain_commands()`
    - engine-side validation/execution
  - Executes callbacks:
    - `on_boot()`
    - `on_update(dt_fixed)` for `N` frames
    - `on_render(alpha)` for `N` frames
    - `on_shutdown()`

- [gamepad.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/gamepad.rs)
  - Provides a file-backed gamepad adapter for diagnostics/testing.
  - Maps parsed state through engine input profiles.

### `vcon-pack`
Packaging and validation CLI for cartridge distribution checks.

- [main.rs](/Users/shahram/source/repos/codex/vcon/vcon-pack/src/main.rs)
  - `build` command emits deterministic `.vcon` bundles with a format version marker.
  - `validate` command accepts either a cartridge directory or `.vcon` bundle and checks:
    - manifest + sdk compatibility
    - permission policy
    - disallowed dependency files
    - disallowed import roots across Python sources
    - entrypoint/assets presence

### `vcon-sdk`
Shipped Milestone 2 API surface for cartridges.

- [__init__.py](/Users/shahram/source/repos/codex/vcon/vcon-sdk/vcon/__init__.py)
- [graphics.py](/Users/shahram/source/repos/codex/vcon/vcon-sdk/vcon/graphics.py)
  - Command-buffer API (`clear`, `line`, `rect`, `circle`, `sprite`, `text`)
  - Per-frame buffer lifecycle (`begin_frame`, `drain_commands`)
- [input.py](/Users/shahram/source/repos/codex/vcon/vcon-sdk/vcon/input.py)
  - `axis(name)` and `action_pressed(name)` accessors
- [save.py](/Users/shahram/source/repos/codex/vcon/vcon-sdk/vcon/save.py)
  - `write`, `read`, `list_slots`
  - quota-aware atomic write path

### Sample cartridges
- `cartridges/sample-game`
  - Exercises rendering primitives + asset sprite + input access.
- `cartridges/input-diagnostics`
  - Visualizes mapped axis/action state.
- `cartridges/save-smoke`
  - Validates save read/write persistence flow.
- `cartridges/save-quota`
  - Validates quota enforcement failure behavior.
- `cartridges/save-recovery`
  - Validates corrupt-slot recovery and quarantine behavior.

## Milestone 5 Status Snapshot

### Completed
- Deterministic fixed-step update loop (`dt_fixed`) with repeatable replay test.
- Seeded deterministic replay path for scripted input (`--input-seed`) with audit tests.
- SDK render command pipeline implemented and validated.
- Software render backend executing draw commands in submission order.
- Render golden snapshot checks for sample and diagnostics cartridges.
- Input API (`axis`, `action_pressed`) available in SDK.
- Input diagnostics cartridge added.
- Audio mixer API scaffolding implemented.
- Save API primitives (`write`, `read`, `list_slots`) implemented with quota checks.
- Save corruption recovery semantics (quarantine + rewrite) with integration tests.
- Runtime and static sandbox hardened against dynamic import bypass patterns.
- Steam Deck-profile performance smoke budget check added.

### Partially complete
- Render backend target exists as software rasterizer and PPM dump path.
  - Planned `moderngl`/windowed backend is not yet integrated.
- Input profile support exists for `Desktop` and `SteamDeck`.
  - Current mapped controls are `move_x`, `move_y`, `A`, `Start` only.
  - Full dual-stick/dpad/ABXY/LR/Start/Select map is pending.
- Gamepad support is file-backed and deterministic for tests.
  - Real hot-plug/reconnect backend handling is pending.

### Remaining gaps
- No `moderngl` windowed GPU backend yet (software rasterizer only).
- Input/controller map is intentionally narrow in current implementation.
- No real OS-level controller hot-plug/reconnect handling yet.
- Audio is queue/mixer scaffolding only (no device playback backend).

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
- `--width`, `--height`: render surface dimensions
- `--input-source`: `none`, `scripted`, or `gamepad`
- `--input-seed`: deterministic seed for scripted input stream
- `--dump-frame`: write final frame to `.ppm`
- `--windowed`: run live loop in an OS window until closed (Esc exits)
- `--windowed-target-fps`: windowed present target (default `60`)
- `--windowed-max-frames`: optional frame cap for windowed mode
- `--window-title`: title used for the windowed runtime window

Current output includes lifecycle invocation and update/render call counts.

Windowed snake demo:
```bash
cargo run -p vcon-runtime -- \
  --cartridge cartridges/snake-demo \
  --sdk-root vcon-sdk \
  --windowed \
  --width 1024 \
  --height 720
```

### `vcon-pack validate`
Command:
```bash
cargo run -p vcon-pack -- validate --cartridge cartridges/sample-game
```

### `vcon-pack build`
Command:
```bash
cargo run -p vcon-pack -- build --cartridge cartridges/sample-game --output /tmp/sample.vcon
```

## Test Coverage

Unit tests:
- [manifest.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/manifest.rs)
- [sandbox.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/sandbox.rs)
- [storage.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/storage.rs)
- [host.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/host.rs)
- [render.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/render.rs)
- [input.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/input.rs)
- [input_mapping.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/input_mapping.rs)
- [audio.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/audio.rs)
- [scene.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/scene.rs)
- [python_host.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/python_host.rs)
- [gamepad.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/gamepad.rs)

Integration tests:
- [foundation_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-engine/tests/foundation_smoke.rs)
- [runtime_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/tests/runtime_smoke.rs)
- [determinism_replay.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/tests/determinism_replay.rs)
- [input_diagnostics_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/tests/input_diagnostics_smoke.rs)
- [render_golden.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/tests/render_golden.rs)
- [performance_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/tests/performance_smoke.rs)
- [save_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-runtime/tests/save_smoke.rs)
- [validate_smoke.rs](/Users/shahram/source/repos/codex/vcon/vcon-pack/tests/validate_smoke.rs)

Test index:
- [tests/README.md](/Users/shahram/source/repos/codex/vcon/tests/README.md)

Run all tests:
```bash
cargo test --workspace
```

## Current Limits
- No `moderngl` windowed GPU backend yet (software rasterizer only).
- Input/controller map is intentionally narrow in current implementation.
- No real OS-level controller hot-plug/reconnect handling yet.
- Audio is queue/mixer scaffolding only (no device playback backend).

## Next Work (from current baseline)
- Integrate windowed render backend and pacing checks for `1280x800 @ 60`.
- Expand canonical input map to full virtual-console control set.
- Add real controller backend with hot-plug/reconnect semantics.
- Integrate Box2D and finalize Milestone 3 lifecycle/API stabilization.
