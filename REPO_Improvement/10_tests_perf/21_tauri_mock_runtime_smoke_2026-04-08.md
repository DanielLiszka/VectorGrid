# Tauri Mock Runtime Smoke Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because command/runtime wiring is the eventual production path these tests support indirectly
- [Tauri: State Management](https://v2.tauri.app/develop/state-management/)
  - relevant because the smoke layer is really about managed-state wiring, not UI behavior
- [Rust testing book section](https://doc.rust-lang.org/book/ch11-00-testing.html)
  - relevant because this layer is a targeted test boundary and should stay intentionally scoped
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Document the exact boundary of this smoke layer:
   - minimal mock app
   - managed-state wiring
   - cancellation/runtime flags
2. Keep the mock builder in sync with managed-state additions that matter to command/runtime smoke behavior.
3. Add only a small number of safe `AppHandle`-oriented smokes if needed, without turning this into a desktop integration suite.
4. Keep broader parity, plugin, and frontend bridge testing in their existing layers.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current mock app builder and the exact state it wires.
- [ ] Document what these tests do cover and do not cover.
- [ ] Document current cancel-token/runtime assertions.
- [ ] Keep the mock builder aligned with managed-state additions that affect command/runtime smoke.
- [ ] Add or tighten tests only within this narrow scope unless another test layer is explicitly being extended.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `cargo test --test mock_runtime_smoke`
- run `cargo test --test command_runtime_wiring`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- keep this layer intentionally small and fast
- if broader Tauri behavior must be tested, add or extend a different test layer instead of overloading this one
- update the mock builder in the same change as relevant managed-state additions
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/04_fe_tauri_bridge_tests_2026-04-08.md`
- `REPO_Improvement/01_shell/34_frontend_tauri_bridge_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
