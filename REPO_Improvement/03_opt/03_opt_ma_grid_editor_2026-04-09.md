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

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because this surface is a dense input editor with staged draft values and commit semantics.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the editor relies on shared store state and should avoid hidden component-local truth.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because request-shaping and grid-state behavior need strong store-aware tests.
- [WAI Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because this editor is part of a tabbed shell and should remain predictable within the workspace model.

## Proposed Improvement Approach

1. Document range editing, MA selection, and param-sweep editing as three related but distinct responsibilities.
2. Freeze the current combination rules between pair ranges and MA param sweeps.
3. Document where pair and combo counts are computed and which layer is authoritative.
4. Make backend-specific MA support messaging and builder rejection rules explicit in one plan.
5. Expand sweep-plus-output legality tests if current coverage is not enough.

## Checklist

- [ ] Document the fast/slow pair-range contract.
- [ ] Document MA-type capability hints and backend-aware labeling.
- [ ] Document per-side parameter state, sweep state, and restore behavior.
- [ ] Document how combo counts are derived and surfaced.
- [ ] Add or tighten tests for sweep plus export or shortlist legality.

## Verification Plan

- run `npm --prefix web test -- src/components/OptimizeWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/optimizeStore.actions.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.bench.test.ts`

## Rollout / Fallback / Compatibility Notes

- keep the current catalog-driven editor model intact
- prefer clearer ownership and more tests before extracting subcomponents
- preserve current draft-and-commit behavior for queued input writes
- if combo preview rules change, update UI wording and builder tests in the same change

## Related Docs

- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/02_data/03_csv_load_contract_2026-04-08.md`
