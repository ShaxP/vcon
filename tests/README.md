# Test Layout

This repository uses two Rust test styles:

- Unit tests (inline in source files)
  - `vcon-engine/src/*.rs`
  - `vcon-runtime/src/python_host.rs`

- Integration tests (crate-level `tests/` directories)
  - `vcon-engine/tests/foundation_smoke.rs`
  - `vcon-runtime/tests/runtime_smoke.rs`
  - `vcon-pack/tests/validate_smoke.rs`

Run all tests from workspace root:

```bash
cargo test --workspace
```
