# Advanced Search Objective Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to objective, score-mode, hurdle, and metric-contract controls so the advanced optimization surface remains precise, non-duplicative, and aligned with request-builder legality.

## Scope Boundaries

In scope:

- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/stores/advancedStore.ts`
- objective and mode interactions across advanced and optimize stores
- metric-contract and `includeTradeMetrics` behavior
- request-builder normalization for objective-related controls

Out of scope:

- shortlist export and union controls beyond their interaction with objective semantics
- backend metric computation internals
- options-related advanced controls

## Current User-Facing Behavior

The advanced panel already owns objective, score mode, hurdle tau, metric contract, and `includeTradeMetrics`. Those controls are route-aware and already show state notes that explain many of the current legality rules.

The request builder already normalizes primary objective, optional score transforms, metric-contract overrides, and trade-metric behavior across several route families. It also enforces constraints such as `full_17` being incompatible with `includeTradeMetrics=true`.

There is still drift risk because objective and mode also appear in optimize-panel or optimize-store state. `optimizeService.ts` reads those values from the advanced store, while the optimize panel and optimize store still carry copies that appear user-facing.

## Current Code Surface

- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/stores/advancedStore.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/historyService.ts`

## Current Verification Surface

- `web/src/components/AdvancedWorkspacePanel.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/historyService.test.ts`

## Strengths Already Present

- advanced objective-related controls are already grouped together in one panel
- legality notes already explain many conditions to the user
- request-builder validation is already strict for score mode and metric-contract combinations
- advanced store normalization already exists for many fields and aliases
- explicit metric-contract selection is already a first-class concept

## Weaknesses / Failure Modes

- objective and mode ownership can drift between advanced and optimize stores
- users can be shown duplicated fields whose true owner is unclear
- saved-run or snapshot hydration can preserve older duplicated values
- metric-contract precedence and `includeTradeMetrics` interaction are subtle enough to deserve a dedicated contract
- score-mode semantics can be accidentally weakened by UI cleanup if not frozen in a plan

## Regressions To Avoid

- objective or mode drift between optimize and advanced surfaces
- breaking `full_17`, `trade_22`, and `includeTradeMetrics` precedence rules
- weakening “objective-only” prerequisites for exact-route advanced controls
- losing history or snapshot compatibility for objective-related fields
- changing UI notes without synchronized request-builder validation
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because these are controlled advanced inputs with draft and commit semantics.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the main risk here is state ownership drift across stores.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because store ownership and hydration behavior need strong regression tests.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because these controls become structured optimization payload fields with compatibility expectations.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Choose and document a single owner for objective and mode.
2. Separate primary objective, score transform, hurdle, and metric-contract semantics clearly.
3. Freeze the precedence rules between `metricContract` and `includeTradeMetrics`.
4. Audit saved-run and snapshot hydration for objective-related fields.
5. Expand tests around duplicated ownership and round-trip behavior if needed.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document ownership of objective and mode across stores and panels.
- [x] Document score-mode semantics and prerequisites.
- [x] Document metric-contract precedence and `includeTradeMetrics` interaction.
- [x] Audit snapshot and history round-trip behavior for these fields.
- [x] Add or tighten tests for drift and precedence bugs.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current request-builder legality even if store ownership is cleaned up
- prefer store-ownership clarification before changing user-visible control layout
- if duplicated fields are removed later, migrate snapshot or history behavior deliberately
- keep UI guidance conservative when route-specific legality is uncertain
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed the real ownership gap instead of only documenting it. `objective` and `mode` were still user-editable in `OptimizeWorkspacePanel.vue` through `optimizeStore`, while `optimizeService.ts` reads the authoritative optimization request fields from `advancedStore`. `optimizeStore.ts` and `advancedStore.ts` now mirror `objective` and `mode` into each other for both direct user edits and legacy snapshot hydration, which removes the drift path without widening scope into a larger store redesign.
- Revalidated the rest of the advanced objective contract instead of rewriting it. `AdvancedWorkspacePanel.vue`, `optimizeRequestBuilder.ts`, and `optimizeService.ts` already enforced score-mode prerequisites, metric-contract precedence, and `includeTradeMetrics` interactions; those behaviors remain unchanged and are now backed by explicit parity tests.
- Added direct store-level regression coverage for both directions of the sync contract. `optimizeStore.actions.test.ts` now proves optimize-surface changes and optimize snapshot hydration update the advanced-store authority fields, and the new `advancedStore.actions.test.ts` proves advanced-surface edits and advanced snapshot hydration keep optimize-surface summary fields aligned.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/AdvancedWorkspacePanel.test.ts src/stores/optimizeStore.actions.test.ts src/stores/advancedStore.actions.test.ts src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/03_opt/06_opt_run_plan_readiness_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
