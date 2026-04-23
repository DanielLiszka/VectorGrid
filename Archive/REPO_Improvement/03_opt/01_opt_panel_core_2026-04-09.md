# Optimization Panel Core Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the main optimization panel so it remains the stable orchestration shell for conventional optimization workflows without drifting into duplicated state, stale summary fields, or broken mount behavior.

## Scope Boundaries

In scope:

- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/stores/optimizeStore.ts`
- `web/src/services/optimizeService.ts`
- shared shell-mount and workflow-tab behavior that directly affects the optimize panel

Out of scope:

- redesigning advanced or strategy panels wholesale
- ML Signal workspace behavior outside optimize-panel boundaries
- changing backend optimization algorithms

## Current User-Facing Behavior

The optimize panel is the conventional optimization shell for the application. It exposes strategy selection, preset application, backend selection, MA sweep controls, dynamic-builder previews, GPU status affordances, and run or validate actions. The panel intentionally remains the normal optimization surface even when `DynamicDoubleMa` is selected.

The panel already branches behavior by strategy family. Strategies like `CompositeMo` hide global backend or mode controls when the strategy owns its own path. `DynamicDoubleMa` exposes its constrained builder and raw JSON preview while still living inside the same optimize shell. Stale `MlSignal` state does not hijack the conventional optimize surface.

The store and panel also carry legacy summary fields such as `strategyHint`, `pairsPreview`, `runPlanMeta`, and readiness text. Some of those are still useful in the UI, but not all appear to be live computed contracts rather than snapshot-era fields.

## Current Code Surface

- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/stores/optimizeStore.ts`
- `web/src/services/optimizeService.ts`
- `web/src/composables/useShellMount.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/OptimizeWorkspacePanel.test.ts`
- `web/src/stores/optimizeStore.actions.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/components/RunFooterPanel.test.ts`

## Strengths Already Present

- the repo already has a clear conventional optimize shell rather than many competing entry points
- the panel already respects strategy-owned route differences such as `CompositeMo`
- dynamic-builder functionality already coexists with normal optimization controls instead of forking the product
- mount behavior already uses the shared shell-mount composable
- the panel already has targeted tests

## Weaknesses / Failure Modes

- orchestration concerns and search-space editing concerns are still mixed in one very large component
- some summary and readiness fields appear to be legacy or snapshot-shaped rather than trustworthy live signals
- objective and mode have drift risk because they appear in both optimize-side and advanced-side surfaces
- the component is large enough that feature work can accidentally change unrelated behavior
- there is no dedicated implementation doc freezing what the optimize panel owns versus what advanced and strategy panels own

## Regressions To Avoid

- breaking shell mount or tab visibility behavior
- letting the optimize panel drift away from the canonical conventional optimization workflow
- reintroducing ML Signal panel hijack behavior into the conventional optimize shell
- preserving stale readiness or summary fields as if they were live truth
- breaking dynamic builder restore and preview behavior
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because the optimize panel is primarily a high-density form surface.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because panel state should remain store-driven rather than duplicated across components.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the panel already depends on a store-plus-service orchestration model.
- [WAI Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because the optimize panel lives inside the shell’s tabbed workspace model and should remain predictable within that structure.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Freeze the optimize panel as the conventional orchestration shell, not the owner of every optimization detail.
2. Separate shell responsibilities, live status responsibilities, and search-space editing responsibilities in the implementation plan.
3. Audit legacy summary fields and either compute them from live state or demote them from authoritative status.
4. Document which controls belong in optimize, which belong in advanced, and which belong in strategy.
5. Strengthen mount-scope and restore tests before large refactors.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the optimize panel’s ownership boundaries.
- [x] Document which summary fields are live truth versus legacy display fields.
- [x] Document strategy-family-specific panel behavior such as `CompositeMo` and `DynamicDoubleMa`.
- [x] Identify candidate splits inside the large component without changing behavior first.
- [x] Add or tighten tests for mount, restore, and conventional-shell behavior.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/OptimizeWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/optimizeStore.actions.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/components/RunFooterPanel.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the existing optimize shell as the default optimization entry point
- prefer documentation and state-audit work before component extraction
- if summary fields are demoted, keep current UI text until all consumers are updated together
- maintain compatibility with persisted snapshot fields until migrations are explicit
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Kept this slice focused on the panel-core contract rather than widening into request-building or backend-routing changes. `OptimizeWorkspacePanel.vue` remains the conventional optimization shell, while strategy-owned route differences such as `CompositeMo` and `DynamicDoubleMa` continue to branch inside that shell instead of forking the workflow surface.
- Tightened the panel evidence with a new component test in `web/src/components/OptimizeWorkspacePanel.test.ts` proving the panel renders live store-driven status and readiness fields (`strategyHint`, `pairsPreview`, `progressMeta`, `statusLine`, `runDisabled`, `validateDisabled`) directly from store truth. This makes it explicit that those fields are the active panel contract, while older snapshot-era fields like `runPlanMeta` are not part of the current core render path.
- Revalidated the existing strategy-family behavior instead of changing UI structure prematurely. The optimize panel tests still prove backend/mode visibility for `DoubleMa`, strategy-owned execution for `CompositeMo`, constrained builder handling for `DynamicDoubleMa`, persisted builder-state restore, and conventional-shell behavior even when stale `MlSignal` state is present.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/OptimizeWorkspacePanel.test.ts src/stores/optimizeStore.actions.test.ts src/services/optimizeService.test.ts src/components/RunFooterPanel.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/06_opt_run_plan_readiness_2026-04-09.md`
- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
