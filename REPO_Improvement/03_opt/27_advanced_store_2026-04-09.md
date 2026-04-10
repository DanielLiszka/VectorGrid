# Advanced Store Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the advanced store so objective and mode ownership, shortlist state, VDPS state, Composite MO search-space state, and export-path helpers remain explicit and maintainable.

## Scope Boundaries

In scope:

- `web/src/stores/advancedStore.ts`
- ownership of advanced-search inputs
- alias normalization for advanced fields
- export browsing and Composite MO search-space reset helpers
- snapshot and history rehydration for advanced state

Out of scope:

- advanced panel presentation details by themselves
- strategy-store ownership
- backend request legality beyond the store contract

## Current User-Facing Behavior

The advanced store already owns the main advanced-search state surface for the product. That includes objective, mode, MA source, metric-contract controls, shortlist controls, top-K and heatmap controls, export-all path state, VDPS settings, and the large Composite MO search-space surface.

The store also carries real behavior. It normalizes legacy mode tokens such as `GpuProgressive` back into `Grid`, performs alias mapping for several snake_case field ids, supports export-path browsing, and exposes a reset action for Composite MO search space. Snapshot rehydration already restores a broad portion of this state.

Because this store is both large and behavior-bearing, it needs its own implementation plan distinct from the advanced panel and request builder.

## Current Code Surface

- `web/src/stores/advancedStore.ts`
- `web/src/stores/advancedStore.actions.test.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/services/optimizeService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/advancedService.ts`

## Current Verification Surface

- `web/src/stores/advancedStore.actions.test.ts`
- `web/src/components/AdvancedWorkspacePanel.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/historyService.test.ts`

## Strengths Already Present

- advanced-store ownership is already centralized
- alias normalization for advanced field ids already exists
- export browsing and MO search-space reset behavior already live in the store boundary
- snapshot and history rehydration already exist
- the store already groups diverse advanced-search families under one canonical surface

## Weaknesses / Failure Modes

- the store mixes several advanced-search families and can become a generic catch-all without documentation
- ownership boundaries between optimize store and advanced store still have drift risk
- alias normalization and legacy mode collapsing are easy to break accidentally
- export browsing and MO reset behavior are real actions but easy to forget during refactors
- snapshot compatibility can drift if field groups are reorganized without a contract

## Regressions To Avoid

- losing alias normalization for advanced field ids
- breaking `GpuProgressive*` to `Grid` compatibility behavior
- moving request-building or legality logic into the store
- breaking export-path browsing or Composite MO search-space reset helpers
- changing snapshot replay behavior without coordinated updates
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because this store is a canonical advanced-search ownership boundary.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because alias normalization and behavioral helpers are store-level regression surfaces.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because advanced-store ownership should stay explicit relative to optimize and strategy stores.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because snapshot and history payloads behave like serialized state contracts.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document the advanced store by state family rather than one flat field list.
2. Freeze alias normalization and legacy mode compatibility as explicit behavior.
3. Keep request building and legality in the service or builder layer, not the store.
4. Document export browsing and MO reset helpers as part of the store contract.
5. Add or tighten tests around behavior-bearing helpers and round-trip state replay.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document advanced-store ownership by state family.
- [x] Document alias normalization and legacy mode compatibility.
- [x] Document export-path browsing and MO search-space reset behavior.
- [x] Document snapshot and history replay expectations.
- [x] Add or tighten tests for store behavior and ownership boundaries.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/advancedStore.actions.test.ts`
- run `npm --prefix web run test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current alias and legacy mode compatibility until migrations are deliberate
- keep the store as the owner of advanced state, not advanced legality
- prefer grouping and documentation before splitting the store
- update snapshot and history consumers together if field groups move
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Tightened the direct advanced-store regression surface so it now covers the behavior-bearing helpers the plan calls out. `advancedStore.actions.test.ts` now directly proves legacy `GpuProgressive*` snapshot normalization back to `Grid`, export-path browsing via `browseExportAll()`, and Composite MO search-space reset via `resetMoSearchSpace()`, in addition to the existing optimize-store parity checks.
- Closed the replay-normalization gap in `historyService.ts`. Saved-run rehydration now routes `objective` and `mode` through `advancedStore.applyLegacySnapshot(...)` instead of directly mutating the store, which means legacy approximate-mode aliases are normalized during replay and parity back into `optimizeStore` remains store-owned.
- Revalidated the replay path with the existing saved-run tests: `historyService.test.ts` now covers a saved conventional run whose stored mode is `GpuProgressive`, and the restored stores correctly normalize that back to `Grid`.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/services/historyService.test.ts src/stores/strategyStore.actions.test.ts src/stores/advancedStore.actions.test.ts src/services/dynamicIndicatorCatalogService.test.ts src/strategies/dynamicDoubleMaBuilder.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/28_recommended_defaults_2026-04-09.md`
