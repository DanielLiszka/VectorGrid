# Optimize Service Progress Plan

Last updated: 2026-04-08

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

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because optimize progress is built around command invocation boundaries
- [Pinia: Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because the optimize service and store actions form a shared state-transition layer
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because progress state is shared UI state and should remain centralized
- [MDN: ARIA `status` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because optimize progress text is surfaced in user-visible status regions

## Proposed Improvement Approach

1. Document the optimize progress lifecycle from pre-run to final idle.
2. Separate panel-local progress from task-center lifecycle tracking.
3. Define the current meaning of:
   - `progressMeta`
   - `statusLine`
   - completion notes
   - cancellation text
4. Tie timing summaries and follow-up notes to the actual phases already measured in code.

## Checklist

- [ ] Document the initial running-state transition.
- [ ] Document current run-phase timing fields and their meaning.
- [ ] Document completion status-line generation and backend summary dependence.
- [ ] Document artifact-save warning notes and validation follow-up notes.
- [ ] Document GPU-refresh follow-up behavior.
- [ ] Document cancel and failure behavior.
- [ ] Document the relationship between optimize-panel progress and task-center progress.
- [ ] Add or tighten tests if progress wording or sequencing changes.

## Verification Plan

- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/stores/optimizeStore.actions.test.ts`
- run `npm --prefix web test -- src/tauri/instrumentation.test.ts`
- run `npm --prefix web test -- src/tauri/instrumentation.mockIPC.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current progress semantics unless a full status-model rewrite is intentional
- keep panel progress additive and truthful rather than overly terse
- if task-center and optimize-panel semantics are changed, update both in the same work item

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/03_opt/25_optimize_store_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
