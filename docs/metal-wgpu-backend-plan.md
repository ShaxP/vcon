# Metal/wgpu Render Backend Plan

Status: Proposed  
Branch: `codex/metal-wgpu-backend-plan-v2`  
Created: 2026-03-09

## 1. Problem Statement

The current `moderngl` path in `vcon-runtime` runs as a Python post-process pass over CPU-rendered RGBA buffers. This adds Python/GIL work and extra buffer traffic in the hot frame loop, and it depends on OpenGL driver/runtime behavior that is unstable on modern macOS setups.

We need a first-class Rust-native GPU path using `wgpu` (Metal on macOS) while keeping deterministic software rendering as fallback and CI baseline.

## 2. Current-State Constraints

- Runtime backend selection is `auto | software | moderngl | wgpu` in [`vcon-runtime/src/render_backend.rs`](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/render_backend.rs), where `moderngl` is now deprecated and routed to software fallback.
- Windowing and present path are currently `minifb` copy-to-window in [`vcon-runtime/src/window_runtime.rs`](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/window_runtime.rs).
- Engine draw commands are executed by `SoftwareFrame` in [`vcon-engine/src/render.rs`](/Users/shahram/source/repos/codex/vcon/vcon-engine/src/render.rs).
- Python host expects per-frame RGBA bytes via `FrameObserver` in [`vcon-runtime/src/python_host.rs`](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/python_host.rs).

Implication: the quickest safe migration is to replace only presentation/post-process first (software render retained), then optionally move primitive rasterization to GPU in a second stage.

## 3. Target Architecture

### Backend Modes

- Add `wgpu` backend request and active mode:
  - requested: `auto | software | moderngl | wgpu`
  - active: `software | wgpu` (`moderngl` request remains CLI-compatible but deprecated)
- `auto` policy:
  - macOS: prefer `wgpu` when adapter + surface initialize succeeds (Metal backend)
  - other platforms: prefer `wgpu` if supported, else software
  - fallback chain: `wgpu -> software`

### Runtime Ownership Split

- `RenderExecutor` remains source of truth for command execution and final RGBA pixels.
- New `WgpuPresenter` owns swapchain/surface, pipeline, textures, and frame pacing integration.
- Window runtime migrates from `minifb` blit to `winit` + `wgpu` surface present.
- Python host contracts remain unchanged for stage 1.

### Data Flow (Stage 1)

1. Python cartridge emits draw commands.
2. `SoftwareFrame` executes commands to RGBA buffer (deterministic baseline unchanged).
3. `WgpuPresenter` uploads RGBA to a GPU texture and presents via fullscreen pass.
4. `dump_frame` continues using CPU RGBA path.

This de-risks migration by removing `moderngl` lag/compat issues without rewriting render semantics.

## 4. Implementation Plan

## Phase 0: Backend Scaffolding (1-2 days)

- Add dependencies to [`vcon-runtime/Cargo.toml`](/Users/shahram/source/repos/codex/vcon/vcon-runtime/Cargo.toml): `wgpu`, `winit`, and `pollster` (or async executor choice).
- Extend CLI `RenderBackendArg` in [`vcon-runtime/src/main.rs`](/Users/shahram/source/repos/codex/vcon/vcon-runtime/src/main.rs) with `Wgpu`.
- Extend backend enums and selection logic in `render_backend.rs`.
- Add capability probe:
  - request high-performance adapter
  - verify texture upload + render pipeline creation
  - emit explicit fallback reasons

Exit criteria:
- `vcon-runtime --render-backend wgpu` resolves to `wgpu` or explicit software fallback reason.

## Phase 1: WGPU Window + Present Path (2-4 days)

- Replace `minifb` window path with `winit` event loop integration in `window_runtime.rs`.
- Implement `WgpuPresenter` module:
  - surface creation and reconfigure on resize
  - sampler + bind group + fullscreen triangle pipeline
  - staging/upload path for RGBA frame data each tick
- Keep input mapping behavior equivalent to existing key mapping.
- Ensure fixed-step loop still controlled by runtime, not by vsync-driven simulation.

Exit criteria:
- `--windowed --render-backend wgpu` runs at `1280x800` and visually matches software output.
- Escape-to-exit behavior preserved.

## Phase 2: Remove Python ModernGL Hot Path (1-2 days)

- Deprecate `moderngl` request path and force software fallback with explicit reason.
- Remove embedded Python shader module from `render_backend.rs`.
- Update docs and runtime reporting to mark `wgpu` as preferred backend.

Exit criteria:
- No Python-side GL dependency needed for default runtime path.

## Phase 3: Validation + Performance Hardening (2-3 days)

- Add/extend tests:
  - backend selection tests in `render_backend.rs`
  - windowed smoke test for `wgpu` (headless-safe skip when unavailable)
  - parity test: command ordering and output checksums between `software` and `wgpu` presentation input
- Add simple frame timing telemetry in runtime report:
  - CPU render time
  - upload + present time
  - dropped/pacing anomalies count
- Run acceptance suite and document results.

Exit criteria:
- Stable frame pacing target (`1280x800 @ 60`) on macOS test machine.
- Golden/software tests remain deterministic.

## 5. Risks and Mitigations

- Event loop integration complexity (`winit` vs current loop model).
  - Mitigation: keep simulation/update ownership in runtime and drive render from controlled ticks.
- GPU availability variance in CI/headless.
  - Mitigation: software remains default for CI tests; `wgpu` tests are capability-gated.
- Texture upload bandwidth overhead (full-frame upload every tick).
  - Mitigation: start with safe full upload; optimize later with persistent mapped/staging buffers.

## 6. Explicit Non-Goals (This Migration)

- Rewriting draw-command rasterization to GPU primitives.
- Shader effects pipeline redesign.
- Changing SDK graphics command semantics.

These can be a follow-up after `wgpu` presentation is stable.

## 7. Proposed Task Breakdown

1. Create `vcon-runtime/src/wgpu_presenter.rs` and compile-guarded constructor tests.
2. Add backend enum/CLI wiring and probe updates.
3. Migrate `window_runtime` to `winit` + `WgpuPresenter`.
4. Keep software execution path and frame dump behavior unchanged.
5. Remove/deprecate `moderngl` path and docs once parity/perf criteria pass.

## 8. Ticketized Checklist

- [x] M0-01 Create planning branch and architecture plan doc.
- [x] M0-02 Define staged migration constraints and non-goals.
- [x] M0-03 Add `wgpu`/`winit`/`pollster` dependencies in runtime crate.
- [x] M0-04 Extend CLI `--render-backend` to include `wgpu`.
- [x] M0-05 Extend runtime backend request/active enums for `wgpu`.
- [x] M0-06 Implement backend probe ordering (`wgpu` first, then software fallback).
- [x] M0-07 Add unit tests for new selection logic and fallback reason formatting.
- [x] M0-08 Run `cargo check -p vcon-runtime` and fix compile warnings/errors.
- [x] M1-01 Introduce `WgpuPresenter` module with init and present API.
- [x] M1-02 Port window runtime from `minifb` to `winit` event loop.
- [x] M1-03 Preserve current keyboard mappings and escape-to-exit behavior.
- [x] M1-04 Handle surface resize/reconfigure safely.
- [x] M1-05 Add windowed `wgpu` smoke test (capability-gated).
- [x] M2-01 Feature-gate or deprecate `moderngl` path. (`moderngl` deprecated with explicit software fallback.)
- [x] M2-02 Remove embedded Python GL shader module.
- [x] M2-03 Update runtime/backend docs to make `wgpu` default recommendation.
- [x] M3-01 Add backend parity checks for command-ordering and output checksum.
- [x] M3-02 Add frame timing telemetry for CPU render and present phases.
- [x] M3-03 Run acceptance suite and record release-readiness notes.
