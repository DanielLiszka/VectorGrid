# Optimization MA Grid Editor Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the moving-average range and parameter editor so sweep construction, MA capability hints, pair counts, and legality checks remain accurate and usable across route families.

## Scope Boundaries

In scope:

- fast and slow MA range controls
- MA type selection and capability hints
- per-side MA parameter editors and parameter sweep controls
- pair and combination preview behavior tied to the request builder

Out of scope:

- non-MA advanced controls
- strategy-family parameter sections owned by the strategy panel
- backend kernel implementation changes

## Current User-Facing Behavior

The optimize panel already exposes fast and slow MA ranges plus MA type selection. The UI is catalog-driven and backend-aware, so users see compatibility hints and GPU speedup notes for selected MA types.

The panel also supports per-side MA parameters and parameter sweeps. Those parameter sweeps multiply the search-space size, and the request builder already enforces guardrails around combinations that are incompatible with export or exact-route features. The product therefore already behaves like a real grid editor rather than a simple pair-range form.

Pair-count and combo-count messaging exists today, but the logic is distributed between UI drafts, catalog services, and request-builder validation.

## Current Code Surface

- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/services/gpuSpeedupsService.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/stores/optimizeStore.ts`
- `src-tauri/src/commands_data.rs`

## Current Verification Surface

- `web/src/stores/optimizeStore.actions.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeRequestBuilder.bench.test.ts`
- `web/src/components/OptimizeWorkspacePanel.test.ts`

## Strengths Already Present

- MA editing is already catalog-driven rather than hard-coded by component
- backend-aware MA support hints already exist in the panel
- parameter sweeps already participate in legality validation instead of being ignored
- pair-count and variant-count thinking is already present in the UX
- current tests already cover a large amount of request-builder behavior

## Weaknesses / Failure Modes

- grid-range editing, MA capability hints, and parameter-sweep editing are still tightly interleaved in one large component
- combo-count truth depends on several layers and is easy to desynchronize
- unsupported MA shapes can still become a UX trap if the panel explanation and builder rejection diverge
- there is no single implementation plan freezing how MA ranges and MA param sweeps combine
- export and shortlist legality can be hard to understand when sweeps are involved

## Regressions To Avoid

- allowing unsupported MA shapes onto a backend route that cannot execute them
- losing hydration or restore behavior for MA parameter values or sweeps
- weakening sweep-output guardrails
- making pair or combo preview numbers less truthful
- changing current backend-aware hint behavior without synchronized tests
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because this surface is a dense input editor with staged draft values and commit semantics.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the editor relies on shared store state and should avoid hidden component-local truth.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because request-shaping and grid-state behavior need strong store-aware tests.
- [WAI Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because this editor is part of a tabbed shell and should remain predictable within the workspace model.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document range editing, MA selection, and param-sweep editing as three related but distinct responsibilities.
2. Freeze the current combination rules between pair ranges and MA param sweeps.
3. Document where pair and combo counts are computed and which layer is authoritative.
4. Make backend-specific MA support messaging and builder rejection rules explicit in one plan.
5. Expand sweep-plus-output legality tests if current coverage is not enough.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the fast/slow pair-range contract.
- [x] Document MA-type capability hints and backend-aware labeling.
- [x] Document per-side parameter state, sweep state, and restore behavior.
- [x] Document how combo counts are derived and surfaced.
- [x] Add or tighten tests for sweep plus export or shortlist legality.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/OptimizeWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/optimizeStore.actions.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.bench.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the current catalog-driven editor model intact
- prefer clearer ownership and more tests before extracting subcomponents
- preserve current draft-and-commit behavior for queued input writes
- if combo preview rules change, update UI wording and builder tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Tightened the MA grid editor evidence at the panel level instead of widening the runtime path. `OptimizeWorkspacePanel.test.ts` now proves the live editor restores seeded fast/slow MA parameter values and sweep ranges into the rendered UI and writes edited sweep values back into `optimizeStore`.
- The new tests exposed a real hydration bug in `OptimizeWorkspacePanel.vue`: one MA side could sync schema-default values back into the store before the opposite side had initialized its UI state, clobbering seeded params or sweeps. `buildSideMaParamsAndSweeps()` now preserves store-seeded values as its fallback until side-specific UI state exists, which keeps fast/slow restore behavior truthful and eliminates the cross-side wipe risk.
- Reused the existing request-builder and optimize-service coverage rather than duplicating legality logic in the component tests. `optimizeRequestBuilder.test.ts` and `optimizeService.test.ts` continue to prove MA param/sweep request encoding and downstream legality, while the new panel tests close the remaining UI restore/writeback gap.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/OptimizeWorkspacePanel.test.ts src/stores/optimizeStore.actions.test.ts src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/02_data/03_csv_load_contract_2026-04-08.md`
