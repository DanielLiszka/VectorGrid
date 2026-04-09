# Tauri Mock Runtime Smoke Plan

Last updated: 2026-04-08

## Purpose

Define the current Rust-side and frontend-adjacent mock-runtime smoke layer so Tauri-facing behavior can be verified quickly and truthfully without pretending that these tests are full desktop integration coverage.

## Scope Boundaries

In scope:

- `src-tauri/tests/support/mod.rs`
- `src-tauri/tests/mock_runtime_smoke.rs`
- `src-tauri/tests/command_runtime_wiring.rs`
- current managed-state mock runtime assumptions

Out of scope:

- full desktop-driver tests
- plugin/window setup coverage
- broad command-surface parity
- options-specific mock-runtime behavior

## Current User-Facing Behavior

Users do not see these tests directly, but they protect a narrow class of runtime safety: whether the minimal managed Tauri app state used by cancellation/runtime tests is wired correctly. The current Rust mock-runtime layer is intentionally tiny and state-focused. `build_mock_app()` wires only `AppState` and `BackgroundTaskState`; the smoke test checks both cancel tokens start clear, and `command_runtime_wiring.rs` checks the cancel commands flip the expected atomic flags. This is valuable, but it is not a substitute for real desktop integration or broad command-parity testing.

## Current Code Surface

- `src-tauri/tests/support/mod.rs`
  - mock app builder with current managed-state wiring
- `src-tauri/tests/mock_runtime_smoke.rs`
  - basic mock app/state smoke test
- `src-tauri/tests/command_runtime_wiring.rs`
  - verifies cancel command wiring against mock runtime state
- `src-tauri/src/lib.rs`
  - real app builder and command surface that the mock layer only partially mirrors

## Current Verification Surface

- `cargo test --test mock_runtime_smoke`
- `cargo test --test command_runtime_wiring`

## Strengths Already Present

- the mock runtime is intentionally narrow, which keeps these tests fast and stable
- the tests verify real managed-state wiring rather than only unit-level helper logic
- cancel-token wiring is already pinned by tests
- the mock layer is clearly separate from desktop/browser integration layers

## Weaknesses / Failure Modes

- it is easy to over-read these tests as broad command-surface or Tauri-builder coverage
- the mock app builder can drift if new managed state is added and these tests are not reviewed
- plugin/window/invoke-handler behavior is intentionally absent, which leaves a correctness gap if misunderstood
- the smoke layer currently verifies mostly cancellation/runtime state, not more general app-handle interactions

## Regressions To Avoid

- treating these tests as full command-surface parity
- coupling them to fragile window/plugin behavior that belongs in other layers
- letting managed-state additions drift out of sync with the mock builder when the smoke layer is supposed to cover them
- weakening current cancel-token wiring assertions

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because command/runtime wiring is the eventual production path these tests support indirectly
- [Tauri: State Management](https://v2.tauri.app/develop/state-management/)
  - relevant because the smoke layer is really about managed-state wiring, not UI behavior
- [Rust testing book section](https://doc.rust-lang.org/book/ch11-00-testing.html)
  - relevant because this layer is a targeted test boundary and should stay intentionally scoped

## Proposed Improvement Approach

1. Document the exact boundary of this smoke layer:
   - minimal mock app
   - managed-state wiring
   - cancellation/runtime flags
2. Keep the mock builder in sync with managed-state additions that matter to command/runtime smoke behavior.
3. Add only a small number of safe `AppHandle`-oriented smokes if needed, without turning this into a desktop integration suite.
4. Keep broader parity, plugin, and frontend bridge testing in their existing layers.

## Checklist

- [ ] Document the current mock app builder and the exact state it wires.
- [ ] Document what these tests do cover and do not cover.
- [ ] Document current cancel-token/runtime assertions.
- [ ] Keep the mock builder aligned with managed-state additions that affect command/runtime smoke.
- [ ] Add or tighten tests only within this narrow scope unless another test layer is explicitly being extended.

## Verification Plan

- run `cargo test --test mock_runtime_smoke`
- run `cargo test --test command_runtime_wiring`

## Rollout / Fallback / Compatibility Notes

- keep this layer intentionally small and fast
- if broader Tauri behavior must be tested, add or extend a different test layer instead of overloading this one
- update the mock builder in the same change as relevant managed-state additions

## Related Docs

- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/04_fe_tauri_bridge_tests_2026-04-08.md`
- `REPO_Improvement/01_shell/34_frontend_tauri_bridge_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
