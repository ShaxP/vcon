# Virtual Console V1 Plan

Status: Draft  
Last updated: 2026-02-28

## 1. Goals And Scope

### V1 Includes
- A virtual gaming console platform with fixed hardware-like constraints.
- A desktop runtime that loads and runs packaged game bundles ("cartridges").
- A sandboxed Python SDK for writing games.
- A deterministic engine loop suitable for consistent behavior across machines.

### V1 Excludes
- Multiplayer networking and online gameplay features.
- Open internet access for games.
- Arbitrary third-party Python dependencies inside cartridges.
- Cloud-sync infrastructure in the first release.

## 2. Fixed Virtual Hardware Spec

- Display: `1280x800` (16:10), target `60 FPS`.
- Controls: dual-stick + triggers layout (`left/right sticks`, `dpad`, `A/B/X/Y`, `L1/R1`, `L2/R2`, `Start`, `Select`).
- Audio: unlimited mixer channels for music and SFX in V1.

## 3. Runtime And Sandbox Model

- Python runtime pinned to `3.12`.
- Games import through SDK-facing APIs only.
- No network access in V1.
- No arbitrary third-party pip packages for game cartridges.
- Per-game isolated storage namespace with quota-managed save data.

## 4. Engine Architecture

### Core Components
- `vcon-runtime`: desktop console application, cartridge loader, lifecycle host.
- `vcon-engine`: internal subsystems (render/input/audio/scene/physics/storage/scheduler).
- `vcon-sdk`: public Python API used by game developers.
- `vcon-pack`: packaging and validation tool for cartridge bundles.

### Gameplay Model
- Scene + Node model as the official V1 authoring architecture.
- Box2D integration for 2D physics and collision handling.

### Rendering
- `moderngl`-based rendering pipeline.
- 2D-first with shader-capable backend.

### Storage
- Engine-managed save API with slots, metadata, and quotas.

## 5. Public Interfaces

### Manifest (`vcon.toml`) Keys
- `id`
- `name`
- `version`
- `entrypoint`
- `sdk_version`
- `assets_path`
- `save_quota_mb`
- `permissions`

### Lifecycle Callbacks (SDK)
- `on_boot()`
- `on_update(dt_fixed)`
- `on_render(alpha)`
- `on_event(event)`
- `on_shutdown()`

### Input And Save APIs (Initial)
- `input.action_pressed(name)`
- `input.axis(name) -> float`
- `save.write(slot: str, data: dict)`
- `save.read(slot: str) -> dict | None`
- `save.list_slots() -> list[str]`

### Input Compatibility Plan (V1)

Input backend and normalization:
- Use a controller backend with broad gamepad support and mapping database support.
- Normalize raw device input to the virtual console layout:
  - sticks: `left_x`, `left_y`, `right_x`, `right_y`
  - dpad: `up`, `down`, `left`, `right`
  - buttons: `A`, `B`, `X`, `Y`, `L1`, `R1`, `L2`, `R2`, `Start`, `Select`

Action/axis abstraction:
- Engine converts normalized input into action/axis bindings used by SDK.
- Deadzones and trigger thresholds are configurable with deterministic defaults.

Device profile handling:
- Support known profiles for Xbox, PlayStation, Switch Pro, and Steam Deck layouts.
- Include fallback generic mapping and per-device override support.
- Support hot-plug/unplug and reconnect handling.

Diagnostics and validation:
- Provide an input diagnostics cartridge that visualizes real-time button/axis states.
- Use diagnostics cartridge in manual validation on desktop and Steam Deck profiles.

### Rendering Primitives API (Initial)

Location and ownership:
- Engine implementation resides in `vcon-engine` render subsystem.
- Public game-facing API resides in `vcon-sdk` (`vcon.graphics` module).
- Python code submits draw commands; engine validates and executes them each frame.

Initial `vcon.graphics` surface:
- `clear(color: tuple[int, int, int, int])`
- `line(x1: float, y1: float, x2: float, y2: float, color: tuple[int, int, int, int], thickness: float = 1.0)`
- `rect(x: float, y: float, w: float, h: float, color: tuple[int, int, int, int], filled: bool = True, thickness: float = 1.0)`
- `circle(x: float, y: float, r: float, color: tuple[int, int, int, int], filled: bool = True, thickness: float = 1.0)`
- `sprite(asset_id: str, x: float, y: float, rotation: float = 0.0, scale: float = 1.0, color: tuple[int, int, int, int] = (255, 255, 255, 255))`
- `text(value: str, x: float, y: float, size: float = 16.0, color: tuple[int, int, int, int] = (255, 255, 255, 255))`

Command-buffer model:
- Draw calls append to a per-frame command list.
- Commands execute in submission order during `on_render(alpha)`.
- No direct GPU/context access is exposed to cartridges.

## 6. Testing And Acceptance Criteria

### Sandbox And Safety
- Verify blocked imports and restricted runtime boundaries.
- Validate cartridge permissions and manifest policy enforcement.

### Determinism
- Fixed-timestep behavior under variable frame timings.
- Seeded replay consistency for logic/physics where applicable.

### Engine Validation
- Render tests (golden or snapshot-based frame checks).
- Physics tests (collision correctness, stability).
- Input mapping tests (desktop and Steam Deck profile behavior).
- Controller compatibility tests (hot-plug, reconnect, profile mapping correctness).
- Save system tests (slot CRUD, corruption handling, quota enforcement).

### Performance Targets
- Maintain stable frame pacing at `1280x800 @ 60 FPS` on Steam Deck target profile.

## 7. Delivery Phases

1. Foundation
- Repository scaffolding, manifest schema, boot path, baseline sandbox.

2. Core Engine
- Scene graph, render path, input/audio, storage primitives.

3. Physics + SDK
- Box2D integration and public API stabilization.

4. Packaging Tooling
- `vcon-pack` build/validate flow and cartridge format enforcement.

5. Hardening
- Determinism audits, sandbox tightening, Steam Deck performance pass.

## 8. Open Questions / Change Log

Use this section as the running history of decisions and refinements.

### 2026-02-28
- Established V1 baseline architecture and fixed console profile.
- Locked primary display target to `1280x800`.
- Selected Scene + Node as the official V1 gameplay architecture.
- Chosen Box2D for built-in physics.
- Confirmed sandbox-first policy: no network, no arbitrary pip dependencies.
- Defined initial `vcon.graphics` rendering primitives and draw-command submission model.
- Added V1 input compatibility plan with profile normalization and diagnostics cartridge.

## 9. Update Process For Future Changes

1. Add a dated entry under `Open Questions / Change Log`.
2. Update affected sections in place (spec, APIs, tests, phases).
3. Keep superseded decisions as short historical notes, not deleted context.
4. Bump the `Last updated` date on each revision.

## Assumptions And Defaults

- Offline-first, single-player-first V1.
- Steam Deck is a primary target profile.
- Backward compatibility begins with exact runtime/SDK match in V1.
