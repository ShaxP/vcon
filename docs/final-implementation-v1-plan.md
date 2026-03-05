# Virtual Console V1 Final Implementation Plan

Status: Proposed  
Branch: `codex/final-implementation-v1`  
Created: 2026-03-05  
Scope: Close all remaining V1 delivery gaps identified after Milestones 1-5.

## Progress Update (2026-03-05)

- Stream A phase 1 implemented:
  - Runtime render backend abstraction added (`auto`, `software`, `moderngl` request modes).
  - `moderngl` capability probe with automatic fallback to software backend.
  - Frame execution routed through a backend executor boundary.
  - Runtime reporting now exposes requested/active backend and fallback reason.
- Remaining Stream A work:
  - Actual windowed GPU presentation path and parity validation harness.

## 1. Goal

Ship a true V1 release candidate by closing current functional and validation gaps while preserving deterministic behavior, sandbox guarantees, and packaging reproducibility.

## 2. Gap Summary (Current State)

1. Windowed GPU backend is not integrated (`moderngl` target remains planned).
2. Controller mapping is limited to `move_x`, `move_y`, `A`, `Start`.
3. No real OS-level controller hot-plug/reconnect behavior.
4. Physics layer is custom and deterministic, but planned Box2D integration is not done.
5. Audio is scaffolding only; no device playback backend.
6. Steam Deck performance validation is smoke-level only, not hardware-verified release validation.

## 3. Delivery Streams

## Stream A: Render Backend Completion

### Objectives
- Integrate a windowed `moderngl` render backend.
- Keep software renderer as deterministic CI backend and fallback mode.

### Tasks
- Add backend abstraction (`software`, `moderngl`) in runtime.
- Implement `moderngl` presentation path at `1280x800 @ 60 FPS`.
- Ensure draw-command parity between software and GPU backends.
- Add headless fallback behavior when GPU/context init fails.

### Exit Criteria
- Runtime can launch in windowed mode with `moderngl`.
- Golden snapshot suite remains stable in software backend.
- Backend parity tests pass for primitive command ordering and parameter handling.

## Stream B: Input/Controller Completion

### Objectives
- Expand canonical input map to planned V1 control surface.
- Implement real controller event backend with connect/disconnect handling.

### Tasks
- Expand mappings for dual sticks, dpad, A/B/X/Y, L1/R1/L2/R2, Start/Select.
- Implement controller backend abstraction (scripted, file, OS-native).
- Add hot-plug and reconnect state transitions with debounce and stale-state reset.
- Extend diagnostics cartridge to visualize full control set and connection events.

### Exit Criteria
- Action/axis tests pass for Desktop and Steam Deck profiles across full mapping matrix.
- Integration tests cover connect, disconnect, reconnect, and profile remap paths.
- Diagnostics cartridge confirms mapped-state correctness for all controls.

## Stream C: Physics Finalization (Box2D)

### Objectives
- Replace or encapsulate current custom physics core with Box2D-backed simulation.
- Preserve deterministic fixed-step behavior and event semantics.

### Tasks
- Integrate Box2D into fixed-step update loop.
- Port scene-physics sync and collision event dispatch to Box2D.
- Maintain stable `vcon.physics` SDK API contracts.
- Add determinism replay tests focused on physics-heavy scenes.

### Exit Criteria
- Physics demo passes with stable collision behavior under Box2D.
- Determinism replay tests pass for seeded multi-run physics scenarios.
- No SDK signature breaks in documented V1 APIs.

## Stream D: Audio Backend Completion

### Objectives
- Move from mixer scaffolding to actual device playback backend.

### Tasks
- Implement runtime audio device output path.
- Preserve queueing/mixing semantics currently exercised by tests.
- Add runtime health reporting for buffer underrun/overrun metrics.
- Add integration smoke test for audio initialization and playback path.

### Exit Criteria
- Runtime initializes audio backend reliably on supported desktop environment.
- Audio playback smoke tests pass without destabilizing frame pacing.
- Existing audio unit tests continue to pass.

## Stream E: Release Validation and Hardening Closure

### Objectives
- Complete acceptance validation on target hardware/profile.
- Close remaining open hardening and regression risks.

### Tasks
- Run full acceptance suite: sandbox, determinism, render, input, physics, storage, packaging, audio.
- Execute hardware validation pass on Steam Deck profile (frame pacing + controller behavior).
- Expand sandbox bypass regression set (dynamic import and runtime escape attempts).
- Produce final release-readiness report with pass/fail per category.

### Exit Criteria
- Acceptance suite passes all categories on CI.
- Steam Deck validation report confirms performance target and control compatibility.
- No critical security/sandbox bypass issues remain open.

## 4. Sequencing

1. Stream A foundation (backend abstraction) and Stream B mapping expansion in parallel.
2. Stream C Box2D integration after fixed-step and scene contracts are locked.
3. Stream D audio backend after render/input stabilization to reduce debugging overlap.
4. Stream E validation/hardening after Streams A-D merge.

## 5. Test Plan Additions

- `vcon-runtime/tests/backend_parity.rs` for software vs GPU command parity.
- `vcon-runtime/tests/controller_hotplug.rs` for connect/reconnect behavior.
- `vcon-runtime/tests/physics_box2d_replay.rs` for seeded deterministic physics replay.
- `vcon-runtime/tests/audio_playback_smoke.rs` for device initialization/playback path.
- Extend `tests/README.md` with new suite categories and execution notes.

## 6. Definition Of Done (Final V1)

1. Windowed `moderngl` backend shipped with software fallback retained.
2. Full controller mapping and hot-plug/reconnect behavior implemented and test-backed.
3. Box2D-backed physics integrated with deterministic replay coverage.
4. Audio device backend shipped with smoke/integration coverage.
5. Acceptance and hardware validation reports complete and passing for release criteria.
