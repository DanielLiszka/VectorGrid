# Optimize Service Progress Plan

Last updated: 2026-04-09

## Purpose

Define the current optimize-service progress contract so run-time status, phase timing, artifact notes, validation follow-up notes, and cancel/error handling remain consistent between `optimizeService`, the optimize store, and task instrumentation.

## Scope Boundaries

In scope:

- `web/src/services/optimizeService.ts` progress and status behavior
- optimize panel `progressMeta` / `statusLine`
- timing summaries and post-run notes
- cancel/error handling during optimization

Out of scope:

- request-building rules themselves
- backend optimization kernels and algorithms
- options workflows

## Current User-Facing Behavior

When the user runs optimization, the optimize panel immediately switches into a running state with `progressMeta` and `statusLine` set to `Running optimization…`. After command completion, the service computes a backend summary, writes a completion status line, persists the run artifact, refreshes analysis, optionally auto-runs validation, optionally refreshes GPU status, and then rewrites `progressMeta` again with a fuller run-phase summary plus any notes such as `validation=ok`, `validation=cancelled`, or artifact-save warnings. If the run fails, the panel falls back to a simple error or `Cancelled.` status line.

## Current Code Surface

- `web/src/services/optimizeService.ts`
  - owns command invocation, `progressMeta` and `statusLine` updates, run-phase timing collection, artifact persistence notes, optional validation follow-up, and final optimize-panel output text
- `web/src/stores/optimizeStore.ts`
  - owns the optimize store fields that the service mutates, including `progressMeta`, `statusLine`, `runDisabled`, `canCancel`, and `validateDisabled`
- `web/src/stores/optimizeStore.actions.test.ts`
  - covers store actions delegating to optimize service
- `web/src/tauri/instrumentation.ts`
  - tracks command lifecycle separately in the task center, which makes it important to keep panel-local progress and global task progress conceptually aligned
- `crates/app_core/src/progress.rs`
- `src-tauri/src/commands_campaign.rs`
- optimization paths in `crates/app_core/src/double_ma.rs`, `crates/app_core/src/composite_cpu.rs`, and `crates/app_core/src/composite_mo.rs`
  - provide the underlying progress events and phases that the UI may later summarize or correlate

## Current Verification Surface

- `web/src/stores/optimizeStore.actions.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/tauri/instrumentation.test.ts`
- `web/src/tauri/instrumentation.mockIPC.test.ts`

## Strengths Already Present

- optimize-panel progress is already explicit instead of inferred from task center state
- the service already tracks multiple timing phases instead of only total wall time
- validation follow-up notes are already surfaced to the user after optimization
- artifact-save failures already degrade into progress notes instead of aborting the whole optimize flow

## Weaknesses / Failure Modes

- there are multiple progress layers: optimize panel, task center, backend progress events, and GPU refresh follow-up
- `statusLine` and `progressMeta` are easy to conflate even though they serve different purposes
- completion-state wording can drift if backend summary formatting, validation notes, and panel-local status updates are changed independently
- current DOM text mirroring and store patching happen together, which makes it easy to preserve one surface and forget the other

## Regressions To Avoid

- weakening the current immediate `Running optimization…` transition
- dropping artifact-save or validation follow-up notes from `progressMeta`
- changing cancel/error behavior so the optimize panel lies about terminal state
- desynchronizing panel-local status text from task-center lifecycle semantics
- silently removing GPU-refresh follow-up from the current post-run sequence
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because optimize progress is built around command invocation boundaries
- [Pinia: Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because the optimize service and store actions form a shared state-transition layer
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because progress state is shared UI state and should remain centralized
- [MDN: ARIA `status` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because optimize progress text is surfaced in user-visible status regions
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document the optimize progress lifecycle from pre-run to final idle.
2. Separate panel-local progress from task-center lifecycle tracking.
3. Define the current meaning of:
   - `progressMeta`
   - `statusLine`
   - completion notes
   - cancellation text
4. Tie timing summaries and follow-up notes to the actual phases already measured in code.
5. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the initial running-state transition.
- [x] Document current run-phase timing fields and their meaning.
- [x] Document completion status-line generation and backend summary dependence.
- [x] Document artifact-save warning notes and validation follow-up notes.
- [x] Document GPU-refresh follow-up behavior.
- [x] Document cancel and failure behavior.
- [x] Document the relationship between optimize-panel progress and task-center progress.
- [x] Add or tighten tests if progress wording or sequencing changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/stores/optimizeStore.actions.test.ts`
- run `npm --prefix web run test -- src/tauri/instrumentation.test.ts`
- run `npm --prefix web run test -- src/tauri/instrumentation.mockIPC.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current progress semantics unless a full status-model rewrite is intentional
- keep panel progress additive and truthful rather than overly terse
- if task-center and optimize-panel semantics are changed, update both in the same work item
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the optimize-service progress contract without changing production sequencing. `optimizeService.test.ts` already covers the immediate running transition, truthful command-boundary completion wording, artifact-save warnings, validation follow-up notes, cancel/failure posture, and the richer `progressMeta` timing summary.
- Kept panel-local progress and task-center instrumentation conceptually separate. The targeted test sweep confirmed `optimizeService.ts`, `optimizeStore.actions.test.ts`, `instrumentation.test.ts`, and `instrumentation.mockIPC.test.ts` still align on lifecycle boundaries without forcing one surface to impersonate the other.
- No product-code change was required in this tranche because the audit did not reveal a missing progress-phase or status-text regression; the current implementation already satisfies the plan’s behavior goals.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts src/utils/strategyOptimizationCapabilities.test.ts src/components/OptimizeWorkspacePanel.test.ts src/stores/optimizeStore.actions.test.ts src/tauri/instrumentation.test.ts src/tauri/instrumentation.mockIPC.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/03_opt/25_optimize_store_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
