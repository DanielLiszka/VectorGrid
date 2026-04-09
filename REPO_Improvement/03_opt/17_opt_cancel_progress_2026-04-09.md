# Optimization Cancel Progress Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to cancellation and progress plumbing so run cancellation intent, legacy fallback behavior, event instrumentation, and user-visible progress remain truthful across the stack.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_optimize.rs` cancellation commands
- `web/src/services/runControlService.ts`
- progress-event instrumentation in `web/src/tauri/instrumentation.ts`
- task-store progress application and cancellation-state behavior

Out of scope:

- redesigning the whole task-center system
- background task domains unrelated to optimization semantics
- options-related runtime controls

## Current User-Facing Behavior

The app already supports explicit current-run cancellation. On the backend, `cancel_current_run` and `cancel_double_ma_optimization` both request cancellation through the app-level run state, while `cancel_background_task` targets background-task state separately.

On the frontend, `runControlService.ts` already treats `cancel_current_run` as the preferred command, requests `cancel_background_task` in parallel, and falls back to `cancel_double_ma_optimization` if the preferred current-run command is unavailable. The task store immediately marks tracked tasks as cancelling while the request is in flight and restores them to running if cancellation cannot be requested.

Progress is already instrumented through the Tauri event system. `commands_optimize.rs` emits `double_ma_progress`, `web/src/tauri/instrumentation.ts` subscribes to that channel, and `taskStore.ts` converts payloads into overall progress, phase changes, timeline entries, and active-task progress updates.

## Current Code Surface

- `src-tauri/src/commands_optimize.rs`
- `src-tauri/tests/command_runtime_wiring.rs`
- `web/src/services/runControlService.ts`
- `web/src/services/runControlService.test.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/tauri/events.ts`
- `web/src/stores/taskStore.ts`

## Current Verification Surface

- `src-tauri/tests/command_runtime_wiring.rs`
- `web/src/services/runControlService.test.ts`
- `web/src/tauri/instrumentation.test.ts`
- `web/src/stores/taskStore.lifecycle.test.ts`
- `web/src/utils/statusPresentation.test.ts`

## Strengths Already Present

- current-run cancellation already has a preferred command and a legacy fallback path
- frontend task state already reflects cancellation intent immediately
- progress events are already centralized on a shared channel
- event instrumentation already normalizes and distributes progress payloads
- backend runtime-wiring tests already verify cancellation commands toggle cancel intent

## Weaknesses / Failure Modes

- cancellation semantics span backend commands, frontend fallback logic, instrumentation, and task-store behavior
- `double_ma_progress` is now a cross-cutting event for more than one domain, which can be confusing without explicit documentation
- failure restoration behavior for cancelling tasks is subtle and easy to regress
- event-subscription behavior is best-effort and deserves explicit contract documentation
- users can be shown progress and cancellation posture that depends on several loosely coupled layers

## Regressions To Avoid

- removing the preferred `cancel_current_run` path or its legacy fallback behavior unintentionally
- leaving tasks stuck in cancelling state when no cancellation command succeeds
- breaking `double_ma_progress` event subscription or normalization
- losing phase-change visibility in the task center and timeline
- allowing backend cancel intent and frontend cancel posture to drift
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because cancellation begins as a command-boundary request.
- [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) is relevant because progress is emitted from Rust into the frontend event system.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because cancellation and progress behavior are tightly bound to store state and are prone to regressions without state-aware tests.
- [Vitest Mocking](https://vitest.dev/guide/mocking.html) is relevant because the current cancellation tests depend on mocked command outcomes and deferred promise timing.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Freeze current-run cancellation, background cancellation, and legacy fallback behavior as an explicit contract.
2. Document the full progress path from backend `ProgressSink` emission to task-store updates.
3. Make the shared use of `double_ma_progress` explicit so contributors do not assume it is Double-MA-only forever.
4. Strengthen tests around task restoration when cancellation fails.
5. Keep status-presentation and task-store semantics aligned with cancellation and progress truth.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document backend cancellation commands and their state targets.
- [ ] Document frontend current-run cancellation, fallback, and background-cancel behavior.
- [ ] Document the `double_ma_progress` event path end to end.
- [ ] Add or tighten tests for cancellation failure restoration and progress-phase handling.
- [ ] Document cross-layer ownership for progress versus cancellation posture.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app command_runtime_wiring`
- run `npm --prefix web run test -- src/services/runControlService.test.ts`
- run `npm --prefix web run test -- src/tauri/instrumentation.test.ts`
- run `npm --prefix web run test -- src/stores/taskStore.lifecycle.test.ts`
- run `npm --prefix web run test -- src/utils/statusPresentation.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current preferred-plus-legacy cancellation model until all callers are unified
- keep `double_ma_progress` stable until all consumers migrate together
- prefer more explicit docs and tests over silent changes to cancel semantics
- if event channels are renamed later, migrate instrumentation and task-store handling in one change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/14_opt_double_ma_command_2026-04-09.md`
- `REPO_Improvement/03_opt/15_opt_composite_cpu_command_2026-04-09.md`
- `REPO_Improvement/03_opt/16_opt_composite_mo_command_2026-04-09.md`
- `REPO_Improvement/01_shell/33_shell_task_lifecycle_summary_2026-04-09.md`
