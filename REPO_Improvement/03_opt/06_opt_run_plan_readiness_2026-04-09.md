# Optimization Run Plan Readiness Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to run-plan and readiness messaging so pre-run guidance, in-run progress, and footer status remain truthful and do not rely on stale snapshot-era fields.

## Scope Boundaries

In scope:

- `web/src/stores/optimizeStore.ts`
- `web/src/services/optimizeService.ts`
- `web/src/components/RunFooterPanel.vue`
- optimize-panel summary fields related to readiness and plan text

Out of scope:

- redesigning the task-center system
- changing backend optimization execution
- validation-specific readiness outside optimize-run boundaries

## Current User-Facing Behavior

The application already surfaces optimization progress and status through `statusLine`, `progressMeta`, busy flags, cancel state, and footer behavior. Those values are actively driven by the optimize service during runs.

At the same time, the store also carries `runPlanMeta`, `runReadinessHint`, and `runReadinessBadges`. Those fields appear in the footer and store shape, but they currently look much more like legacy or snapshot-era display fields than live recomputed readiness truth. They are initialized and hydrated from snapshots, but no comparable live recomputation path is obvious.

This means the app already has real progress truth, but readiness truth may be weaker than the product presentation suggests.

## Current Code Surface

- `web/src/stores/optimizeStore.ts`
- `web/src/services/optimizeService.ts`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/stores/taskStore.ts`

## Current Verification Surface

- `web/src/components/RunFooterPanel.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/stores/domainStores.test.ts`
- `web/src/stores/optimizeStore.actions.test.ts`

## Strengths Already Present

- live optimization progress already exists and is service-driven
- run and cancel busy semantics are already wired into the product
- the footer already centralizes primary optimization action posture
- readiness-related display fields are at least centralized in the store rather than scattered everywhere
- test coverage already exists for footer and optimize service behavior

## Weaknesses / Failure Modes

- some readiness fields appear stale or snapshot-driven instead of live computed
- the product can imply stronger readiness truth than it actually has
- footer, panel, and task-center language can drift if readiness and progress semantics are not separated
- there is no dedicated contract saying what counts as pre-run readiness versus in-run status
- stale display fields can survive across restored snapshots

## Regressions To Avoid

- presenting stale readiness hints as live truth
- breaking current run, cancel, or post-run busy behavior
- making the footer contradict the optimize panel or task center
- removing useful progress detail while trying to clean up readiness messaging
- coupling footer copy too tightly to snapshot data

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because readiness and progress semantics should derive from shared state, not component-local copy.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because this is primarily a store-ownership and derived-state problem.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because readiness drift is easiest to catch with store-aware component tests.
- [Vitest Mocking](https://vitest.dev/guide/mocking.html) is relevant because optimize-service-driven status transitions should remain easy to simulate in tests.

## Proposed Improvement Approach

1. Separate pre-run readiness, run-plan summary, and in-run progress as distinct concepts.
2. Audit `runPlanMeta`, `runReadinessHint`, and `runReadinessBadges` and decide whether to compute them live or demote them.
3. Keep `statusLine` and `progressMeta` as the live in-run truth unless a stronger model replaces them.
4. Align footer posture with optimize-panel and task-center semantics.
5. Add explicit tests for no-data, route-blocked, busy, and post-run states.

## Checklist

- [ ] Document which status fields are live, derived, or snapshot-only today.
- [ ] Document the difference between readiness and progress.
- [ ] Decide whether legacy readiness fields should be computed, demoted, or removed from authority.
- [ ] Add or tighten tests for common readiness states.
- [ ] Align footer copy and panel copy to the same semantics.

## Verification Plan

- run `npm --prefix web test -- src/components/RunFooterPanel.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/stores/domainStores.test.ts`
- run `npm --prefix web test -- src/stores/optimizeStore.actions.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current primary run and cancel behavior while auditing readiness messaging
- prefer demotion of stale fields over pretending they are live truth
- keep snapshot compatibility until a deliberate migration removes or recalculates old readiness fields
- if readiness becomes live-computed later, update all shell consumers together

## Related Docs

- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/01_shell/33_shell_task_lifecycle_summary_2026-04-09.md`
- `REPO_Improvement/01_shell/06_shell_run_footer_2026-04-08.md`
- `REPO_Improvement/01_shell/26_runtime_task_store_2026-04-08.md`
