# Virtual Console V1 Execution Plan

Status: Proposed  
Derived from: `docs/virtual-console-plan.md`  
Created: 2026-02-28

## 1. Delivery Approach

- Build V1 in five milestones aligned to the architecture plan: Foundation, Core Engine, Physics + SDK, Packaging Tooling, Hardening.
- Gate progress using explicit exit criteria per milestone.
- Keep runtime determinism and sandbox restrictions as non-negotiable quality bars throughout development.

## 2. Milestone Plan

### Milestone 1: Foundation

#### Objectives
- Stand up repository structure and baseline runtime boot path.
- Define cartridge manifest schema and policy checks.
- Establish sandbox boundaries for Python 3.12 game execution.

#### Tasks
- Create package/module layout:
  - `vcon-runtime`
  - `vcon-engine`
  - `vcon-sdk`
  - `vcon-pack`
- Implement manifest parsing and validation for required keys in `vcon.toml`.
- Implement cartridge boot lifecycle: load manifest, resolve entrypoint, initialize runtime host.
- Create sandbox policy layer:
  - block network access
  - restrict imports to approved SDK surface
  - block arbitrary third-party dependencies
- Define per-game save namespace layout and quota metadata model.

#### Deliverables
- Runtime boots a minimal cartridge and calls `on_boot()` and `on_shutdown()`.
- Manifest validation returns actionable errors.
- Baseline sandbox policy is testable and enabled by default.

#### Exit Criteria
- All foundation tests pass (manifest validation + sandbox boundary smoke tests).
- Sample cartridge runs without direct access to prohibited imports/network APIs.

### Milestone 2: Core Engine

#### Objectives
- Implement deterministic game loop, rendering path, input system, audio scaffolding, and storage primitives.
- Implement the initial SDK rendering primitives (`vcon.graphics`) using an engine-owned draw command buffer.

#### Tasks
- Build fixed timestep scheduler for `on_update(dt_fixed)` with interpolation for `on_render(alpha)`.
- Implement Scene + Node core model for game objects and hierarchy updates.
- Add `moderngl` render backend targeting `1280x800 @ 60 FPS`.
- Implement rendering primitive command model first:
  - Engine-side `DrawCommand` queue and validation
  - SDK-side `vcon.graphics` calls (`clear`, `line`, `rect`, `circle`, `sprite`, `text`)
  - Ordered command execution during `on_render(alpha)`
- Implement input mapping for:
  - dual sticks
  - dpad
  - A/B/X/Y
  - L1/R1/L2/R2
  - Start/Select
- Implement controller compatibility layer:
  - normalize controller backends to canonical virtual-console inputs
  - support profile mappings for common controllers and Steam Deck target profile
  - handle hot-plug/unplug and reconnect events
- Build input diagnostics sample cartridge to visualize raw and mapped input state.
- Add initial audio mixer API hooks.
- Implement save API primitives:
  - `save.write`
  - `save.read`
  - `save.list_slots`

#### Deliverables
- A playable sample scene updates and renders deterministically.
- A sample cartridge that draws exclusively via `vcon.graphics` primitives.
- Input events are accessible via SDK API (`action_pressed`, `axis`).
- Input diagnostics cartridge available for compatibility verification.
- Save slots persist data with per-game namespace isolation.

#### Exit Criteria
- Loop determinism test harness passes basic replay consistency checks.
- Rendering primitive tests pass (command ordering, parameter validation, and snapshot sanity checks).
- Input mapping tests pass for desktop and Steam Deck profile.
- Controller compatibility checks pass for hot-plug/reconnect and profile mapping correctness.
- Save CRUD tests pass with quota enforcement.

### Milestone 3: Physics + SDK Stabilization

#### Objectives
- Integrate Box2D and freeze initial SDK lifecycle/input/save contracts for V1.

#### Tasks
- Integrate Box2D into engine update loop with fixed-step synchronization.
- Expose physics components in Scene + Node model.
- Finalize SDK callback lifecycle:
  - `on_boot()`
  - `on_update(dt_fixed)`
  - `on_render(alpha)`
  - `on_event(event)`
  - `on_shutdown()`
- Add developer-facing SDK reference stubs and examples.
- Add compatibility checks for `sdk_version` in manifest.

#### Deliverables
- Physics demo cartridge showing stable collision behavior.
- SDK V1 surface documented and version-gated.

#### Exit Criteria
- Physics correctness and stability tests pass.
- No breaking changes to SDK signatures after milestone completion.

### Milestone 4: Packaging Tooling (`vcon-pack`)

#### Objectives
- Provide a deterministic, validated cartridge packaging and verification workflow.

#### Tasks
- Implement `vcon-pack build` to package cartridge assets and code.
- Implement `vcon-pack validate` for:
  - manifest schema and key constraints
  - permission policy checks
  - disallowed dependency/import checks
- Define cartridge bundle format and version marker.
- Add reproducibility checks to ensure deterministic package output.

#### Deliverables
- CLI tool capable of building and validating distributable cartridges.
- Clear error reporting with line/key context for malformed manifests.

#### Exit Criteria
- Golden tests confirm deterministic package output for identical inputs.
- Invalid cartridges are consistently rejected with useful diagnostics.

### Milestone 5: Hardening And Release Readiness

#### Objectives
- Close determinism gaps, harden sandbox enforcement, and validate performance on Steam Deck target profile.

#### Tasks
- Run determinism audits under variable frame timings and seeded replay scenarios.
- Tighten sandbox to close bypass paths discovered during testing.
- Implement render snapshot/golden tests and tune frame pacing.
- Execute performance passes targeting stable `60 FPS` at `1280x800` on Steam Deck profile.
- Run final controller compatibility regression passes using the diagnostics cartridge and action-level tests.
- Add corruption handling tests for save system and recovery semantics.

#### Deliverables
- Release candidate runtime and SDK with validated constraints and performance profile.
- Test reports for sandbox, determinism, rendering, physics, input, and storage.

#### Exit Criteria
- Acceptance suite passes all categories defined in plan.
- Steam Deck target profile meets frame pacing/performance objective.
- No critical security/sandbox bypass issues remain open.

## 3. Cross-Cutting Workstreams

### Determinism
- Standardize random seeding strategy for engine and physics tests.
- Maintain replay test corpus from Milestone 2 onward.

### Security/Sandbox
- Treat denied capabilities as explicit policy with test coverage.
- Keep import allowlist and permission checks centrally versioned.

### Developer Experience
- Maintain one example cartridge per major feature area (input, save, physics, rendering).
- Keep SDK docs synchronized with shipped API surface each milestone.

## 4. Dependency Order

1. Manifest schema + runtime boot path before SDK stabilization.
2. Fixed timestep and scene model before physics integration.
3. Stable SDK surface before packaging validation rules are finalized.
4. Full feature coverage before hardening/performance lock.

## 5. Risks And Mitigations

- Risk: Non-deterministic behavior from physics/render coupling.
  - Mitigation: Keep physics strictly in fixed-step update and isolate rendering interpolation.
- Risk: Sandbox bypass via Python import/runtime edges.
  - Mitigation: Centralized allowlist + deny-by-default checks + regression tests.
- Risk: Performance regressions on Steam Deck profile late in cycle.
  - Mitigation: Introduce frame pacing benchmarks in Milestone 2 and enforce regression budgets.

## 6. Definition Of Done (V1)

- Runtime launches and executes packaged cartridges with required lifecycle callbacks.
- SDK supports defined input and save APIs with stable signatures.
- No network access or arbitrary third-party dependencies available to cartridges.
- Determinism and acceptance tests pass across render, physics, input, storage, and sandbox categories.
- Target performance objective (`1280x800 @ 60 FPS` on Steam Deck profile) is met.
