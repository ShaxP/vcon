# Implementation Agent Contract (`agent.md`)

Status: Active  
Date: 2026-02-28

## 1) Mission
Build Virtual Console V1 end-to-end in this repository using the agreed stack:
- Rust core for runtime/engine/tooling
- Embedded Python 3.12 for game SDK surface
- Box2D for 2D physics
- Deterministic engine loop and sandbox-first model

## 2) Non-Negotiable Product Constraints
- Display target: `1280x800` at `60 FPS`
- Offline-first; no network access for cartridges
- No arbitrary third-party Python dependencies in cartridges
- Deterministic fixed-timestep gameplay behavior
- Per-game isolated save storage with quota enforcement

## 3) Responsibilities
The implementation agent will:
- Implement code, tests, and tooling (not only plans/docs)
- Keep milestones shippable incrementally
- Preserve deterministic and sandbox guarantees as first-class quality bars
- Report tradeoffs and blockers with concrete next actions

## 4) Out Of Scope (V1)
- Multiplayer/online gameplay
- Internet-connected cartridge behavior
- Cloud save sync infrastructure

## 5) Technical Direction
- `vcon-runtime` (Rust): desktop host + cartridge lifecycle
- `vcon-engine` (Rust): loop, render/input/audio/storage/scene/physics integration
- `vcon-sdk` (Python 3.12): public game-author API
- `vcon-pack` (Rust CLI): build/validate cartridge bundles

### Runtime boundary
- Rust owns execution model and policy enforcement
- Python code only interacts through SDK API surface
- Sandbox is deny-by-default

## 6) Delivery Sequence
1. Foundation
- Repository scaffolding and workspace setup
- Manifest schema and validation
- Cartridge boot path + lifecycle wiring
- Baseline sandbox restrictions

2. Core Engine
- Fixed timestep scheduler
- Scene + Node model
- Render/input/audio/save primitives

3. Physics + SDK Stabilization
- Box2D integration
- Stable lifecycle/input/save SDK contracts
- `sdk_version` compatibility enforcement

4. Packaging Tooling
- `vcon-pack build`
- `vcon-pack validate`
- Deterministic package output checks

5. Hardening
- Determinism replay audits
- Sandbox tightening
- Steam Deck performance pass

## 7) Quality Gates Per Milestone
- Required tests must exist and pass before advancing
- New public API changes require tests and docs updates
- Security/sandbox regressions block promotion
- Determinism regressions block promotion

## 8) Engineering Rules
- Keep modules small and explicit; favor composable interfaces
- Avoid hidden global state in loop/physics/storage paths
- Version public interfaces intentionally (manifest + SDK)
- Fail fast with actionable diagnostics (especially manifest/packaging)
- Do not weaken sandbox policy for convenience

## 9) Definition Of Done (V1)
- Runtime loads packaged cartridges and executes lifecycle callbacks
- SDK input/save APIs are stable and tested
- Sandbox constraints enforced (no network, no arbitrary deps)
- Determinism + acceptance suites pass
- Target performance bar met at `1280x800 @ 60 FPS`

## 10) Working Agreement
- Implementation starts from Milestone 1 unless reprioritized
- Changes are incremental and test-backed
- Any scope change is recorded with date and rationale
