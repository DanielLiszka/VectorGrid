# Run Control Cancel Bridge Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the run-control cancel bridge so foreground-run cancellation, background-task cancellation, progress-event handling, and task-center state transitions remain explicit, truthful, and regression-resistant.

## Scope Boundaries

In scope:

- Tauri cancel commands for foreground and background work
- `AppState` and `BackgroundTaskState` cancellation behavior
- frontend run-control service behavior, including legacy fallback
- task-center progress and cancelling-state handling for optimize-family commands

Out of scope:

- per-strategy cancel semantics inside individual engine loops
- unrelated live-execution stop controls
- options-related run-control surfaces

## Current User-Facing Behavior

The repo already exposes a unified cancel entrypoint for optimize-family work, but it is not a true per-run tokenized cancellation system. The frontend requests cancellation through `requestCancelCurrentRun`, which attempts background-task cancellation in parallel, then tries `cancel_current_run`, and finally falls back to the legacy `cancel_double_ma_optimization` command if the newer foreground command is unavailable.

At the backend boundary, both `cancel_current_run` and the legacy Double-MA cancel command currently map to the same `AppState::request_cancel()` atomic flag. Background work has a separate cancellation path. Progress events continue to flow through the shared `double_ma_progress` payload, and the task center owns most of the visible lifecycle and cancellation posture rather than `optimizeService` itself.

That means the user-visible cancel experience depends on a three-layer bridge: frontend service behavior, Tauri command availability, and backend atomic-flag semantics. If any one layer drifts, the UI can show cancelling state that does not correspond to actual runtime behavior.

## Current Code Surface

- `src-tauri/src/commands_optimize.rs`
- `src-tauri/src/state.rs`
- `web/src/services/runControlService.ts`
- `web/src/services/optimizeService.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/tauri/instrumentation.ts`
- `crates/app_core/src/progress.rs`

## Current Verification Surface

- `web/src/services/runControlService.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/stores/taskStore.lifecycle.test.ts`
- `web/src/stores/taskStorePerformance.test.ts`
- `src-tauri/tests/command_runtime_wiring.rs`
- targeted cancellation tests in `crates/app_core/src/double_ma.rs`

## Strengths Already Present

- the repo already has a single frontend cancel helper for mapped run commands
- legacy fallback behavior already exists instead of assuming all runtimes have the new command surface
- task-center lifecycle state already records cancelling posture separately from running
- shared `double_ma_progress` events already provide a common progress surface across multiple optimize-family backends
- backend state already enforces a single foreground-run slot with a shared atomic cancel token

## Weaknesses / Failure Modes

- the system still presents a unified cancel surface on top of a single-slot foreground cancellation model
- progress and task lifecycle ownership is split between optimize service, instrumentation, and task store, which is easy to misunderstand
- fallback behavior can regress quietly if the newer and legacy command surfaces stop behaving identically
- cancelling state can become misleading if command availability fails and task restoration is incomplete
- the current bridge can be over-described as finer-grained than it really is

## Regressions To Avoid

- removing or bypassing the legacy fallback before all supported runtimes converge
- renaming `double_ma_progress` or changing its payload semantics without synchronized consumers
- breaking task restoration when cancel commands fail
- implying per-run targeted cancellation when the backend still uses a shared single-run cancel flag
- weakening `try_begin_run()` single-run protection while changing cancel behavior
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the cancel bridge depends on stable command-boundary contracts.
- [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) is relevant because progress and lifecycle truth reach the UI through emitted runtime events.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because cancelability and run exclusivity rely on shared backend state instead of isolated command-local flags.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because task lifecycle and cancel posture are centralized store concerns rather than component-local behavior.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the core correctness questions are state-transition and fallback-path behaviors that should stay deterministic in tests.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document the full cancel bridge as one pipeline: mapped command selection, fallback behavior, Tauri command handling, backend atomic cancellation, emitted progress, and task-store lifecycle updates.
2. Freeze the distinction between foreground cancellation, background-task cancellation, and legacy fallback semantics.
3. Keep the single-foreground-run limitation explicit so UI or doc changes do not overpromise finer-grained cancellation.
4. Treat task restoration after failed cancel requests as a first-class compatibility requirement.
5. Add or tighten tests whenever cancel-command names, payloads, or task-center lifecycle rules change.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document foreground, background, and legacy cancel paths separately.
- [ ] Document `double_ma_progress` payload expectations and task-center ownership.
- [ ] Document the single-foreground-run limitation and shared cancel-token model.
- [ ] Document task restoration behavior when cancellation commands fail.
- [ ] Add or tighten tests for fallback behavior, lifecycle status transitions, and progress compatibility.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/runControlService.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run targeted task lifecycle tests in `web/src/stores/taskStore.lifecycle.test.ts`
- run targeted task throughput tests in `web/src/stores/taskStorePerformance.test.ts`
- run `cargo test -p ta_desktop_demo_app command_runtime_wiring`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the legacy cancel fallback until all supported runtimes no longer need it
- keep progress payload shape stable until every consumer migrates together
- prefer additive lifecycle metadata over repurposing existing task statuses
- if the app later moves to per-run tokens, document the model change explicitly rather than quietly changing existing semantics
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/12_opt_service_progress_2026-04-08.md`
- `REPO_Improvement/03_opt/17_opt_cancel_progress_2026-04-09.md`
- `REPO_Improvement/03_opt/25_optimize_store_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
