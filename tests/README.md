# Test Layout

This repository uses two Rust test styles:

- Unit tests (inline in source files)
  - `vcon-engine/src/*.rs`
  - `vcon-runtime/src/python_host.rs`

- Integration tests (crate-level `tests/` directories)
  - `vcon-engine/tests/foundation_smoke.rs`
  - `vcon-runtime/tests/runtime_smoke.rs`
  - `vcon-runtime/tests/determinism_replay.rs`
  - `vcon-runtime/tests/controller_hotplug.rs`
  - `vcon-runtime/tests/physics_box2d_replay.rs`
  - `vcon-runtime/tests/input_diagnostics_smoke.rs`
  - `vcon-runtime/tests/render_golden.rs`
  - `vcon-runtime/tests/performance_smoke.rs`
  - `vcon-runtime/tests/save_smoke.rs`
  - `vcon-pack/tests/validate_smoke.rs`

Run all tests from workspace root:

```bash
cargo test --workspace
```
