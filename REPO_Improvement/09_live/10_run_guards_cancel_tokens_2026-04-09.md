# Run Guards Cancel Tokens Plan

Last updated: 2026-04-10

## Purpose

Define the current cancellation and single-run guard contracts across desktop runs and the live execution runner so future improvements do not weaken mutual exclusion, cooperative cancellation, or operator-visible status truth.

## Scope Boundaries

In scope:

- `AppState` run guard and cancellation token behavior
- background-task guard behavior in Tauri state
- live execution runner abort/generation logic
- frontend/status expectations that depend on these guards
- cancellation and stale-runtime protections for live/runtime tasks

Out of scope:

- optimization algorithm internals
- live broker order semantics beyond runner lifecycle control
- options-related task control

## Current User-Facing Behavior

The app already treats long-running work as guarded and cancellable rather than freely concurrent. Current behavior includes:

- a global run guard for backend workflows so only one protected run occupies the primary run slot at a time
- cooperative cancellation through shared atomic flags rather than preemptive teardown of arbitrary work
- a separate but conceptually similar guard for background-task surfaces
- a live execution runner that uses its own abort marker plus generation checks to prevent stale async work from mutating current runner state
- frontend task/run state that assumes cancellation is best-effort and status updates reflect the currently active generation only

This means “cancel” currently means “request cooperative stop and ignore stale work,” not “kill every in-flight computation instantly.”

## Current Code Surface

- `src-tauri/src/state.rs`
  - owns `RunGuard`, primary run-state atomics, cancellation token access, background-task guards, and live-candle-stream slot state
- `src-tauri/src/commands.rs`
  - exposes run-cancel style commands for non-live workflows
- `src-tauri/src/commands_execution.rs`
  - owns `RunnerAbortHandle`, runner generation tracking, in-flight/pending-bar logic, and stale-generation protections
- `src-tauri/src/lib.rs`
  - registers the relevant command surfaces and shared app state
- `web/src/stores/taskStore.ts`
  - represents protected long-running tasks in the frontend shell
- `web/src/stores/executionStore.ts`
  - reflects live runner busy/active state and user-facing blocked reasons
- `web/src/services/executionService.ts`
  - interprets runner status/events and exposes them to the frontend
- `web/src/components/DesktopTitlebar.vue`
  - guards app close while protected tasks or the live runner are active

## Current Verification Surface

- targeted unit tests in `src-tauri/src/commands_execution.rs`
- `src-tauri/tests/mock_runtime_smoke.rs`
- `src-tauri/tests/command_runtime_wiring.rs`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`

## Strengths Already Present

- primary run state already uses explicit guard objects instead of ad hoc booleans scattered across commands
- cancellation is already modeled as cooperative state shared across the runtime
- the live runner already protects against stale async completions via generation checks
- close-guard UX already respects the same protected-task model instead of bypassing it
- runner status already exposes `inflight`, counts, and config metadata rather than only a binary on/off flag

## Weaknesses / Failure Modes

- the repo uses more than one guard/cancel idiom, which makes it easy to document them too vaguely
- cooperative cancellation can be misunderstood as immediate termination
- stale-generation protection is subtle and easy to break during refactors
- primary run guards, background-task guards, and live-runner guards are related but not interchangeable
- frontend busy/block reasons can drift from backend mutual-exclusion truth if not explicitly tied to current guard behavior

## Regressions To Avoid

- allowing concurrent protected runs in the same slot without an explicit design change
- replacing cooperative cancellation with partial behavior that leaves status surfaces lying to the user
- removing or weakening runner generation checks
- clearing guarded-close protections while tasks or runners remain active
- making cancel behavior appear instantaneous when it is still best-effort/cooperative
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tokio `JoinHandle` docs](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)
  - relevant because the repo combines async task handles with cooperative state rather than treating abort alone as full business-logic cancellation
- [Rust `AtomicBool`](https://doc.rust-lang.org/std/sync/atomic/struct.AtomicBool.html)
  - relevant because cancellation and guarded state currently rely on atomic flags
- [Tauri state management](https://v2.tauri.app/develop/state-management/)
  - relevant because the app’s guarded run/runtime state is owned on the Rust side and projected into the frontend
- [Tokio task module](https://docs.rs/tokio/latest/tokio/task/)
  - relevant because stale async completions and cancellation behavior need to be understood in the context of spawned tasks
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document the current guard model explicitly:
   - primary run guard
   - background-task guard
   - live-runner abort/generation guard
2. Keep the semantics precise: cancellation is cooperative, generation checks prevent stale mutations, and guard objects own exclusivity.
3. Tie frontend blocked/busy/close-guard behavior back to the authoritative backend guard model.
4. Preserve the existing distinction between “request stop,” “runner no longer active,” and “all async work fully settled.”
5. Add regression coverage before changing any guard or cancel token shape.
6. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the primary `AppState` run guard and cancel token semantics.
- [x] Document background-task guard behavior separately from the primary run slot.
- [x] Document live-runner abort handling and generation checks.
- [x] Document the difference between cooperative cancellation and immediate task termination.
- [x] Document which frontend status surfaces depend on these guards.
- [x] Add or tighten tests when guard ownership, cancellation, or generation logic changes.
- [x] Keep close-guard UX aligned with protected-task truth.
- [x] Review any future concurrency broadening against the current single-slot assumptions.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run targeted tests in `src-tauri/src/commands_execution.rs`
- run `cargo test -p ta_desktop_demo_app --test command_runtime_wiring`
- run `cargo test -p ta_desktop_demo_app --test mock_runtime_smoke`
- run `npm --prefix web run test -- src/stores/executionStore.actions.test.ts`
- run `npm --prefix web run test -- src/components/DesktopTitlebar.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current single-slot and generation-based safety first
- if cancellation behavior is made more structured later, keep current user-facing expectations explicit during migration
- do not remove guarded-close behavior until an equivalent protected-shutdown story exists
- keep cooperative cancel semantics documented even if lower-level task primitives change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on verified current guard behavior without widening concurrency semantics.
- Tightened the background-task contract where it had been weakest:
  - [commands_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_data.rs) now routes Polygon background download/top-off commands through `BackgroundTaskState` so reentry is rejected and the shared cancel token is available to the blocking download path
  - [polygon.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/polygon.rs) now accepts an optional cancel flag and checks it cooperatively during chunked download/top-off work
  - [state.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/state.rs) now has direct guard tests for both the primary run slot and background-task slot
- Acceptance evidence now covers the three guard layers this doc owns:
  - primary/background cancellation commands in [command_runtime_wiring.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/tests/command_runtime_wiring.rs)
  - managed-state initialization and cleared cancel tokens in [mock_runtime_smoke.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/tests/mock_runtime_smoke.rs)
  - live-runner abort and generation protections in [commands_execution.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_execution.rs)
  - background-task exclusivity and drop-reset behavior in [commands_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_data.rs) and [state.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/state.rs)
  - frontend cancellation posture and fallback behavior in [runControlService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/runControlService.test.ts)
  - close-guard truth in [DesktopTitlebar.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.test.ts)
- Kept scope intentionally additive:
  - no primary run-slot ownership changed
  - no cancel command names changed
  - no runner generation logic changed
- Acceptance evidence:
  - `cargo test -p ta_desktop_demo_app --lib background_download_slot_rejects_reentry`
  - `cargo test -p ta_desktop_demo_app --lib state_guard_tests`
  - `cargo test -p ta_desktop_demo_app --test command_runtime_wiring`
  - `cargo test -p ta_desktop_demo_app --test mock_runtime_smoke`
  - `npm.cmd --prefix web run test -- src/services/runControlService.test.ts src/stores/executionStore.actions.test.ts src/components/DesktopTitlebar.test.ts`
  - `npm.cmd --prefix web run lint`
  - `set NODE_OPTIONS=--max-old-space-size=4096 && npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/01_shell/05_shell_task_center_2026-04-08.md`
- `REPO_Improvement/01_shell/33_shell_task_lifecycle_summary_2026-04-08.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
