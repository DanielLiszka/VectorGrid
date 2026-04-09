# Advanced Shortlist Export Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to shortlist, export-all, confidence, union, slice, and neighborhood controls so advanced optimization legality remains comprehensible and exact-route promises stay correct.

## Scope Boundaries

In scope:

- advanced shortlist controls in `AdvancedWorkspacePanel.vue`
- export-all CSV contract
- request-builder legality for exact-kernel and standalone unified-GPU shortlist behavior
- backend shortlist-result and export behavior for optimization

Out of scope:

- validation-side shortlist export or XLSX reporting
- backend optimization algorithm changes
- options-related export surfaces

## Current User-Facing Behavior

The advanced panel already exposes top-K, include-all, heatmap, export-all, extra shortlist objectives, confidence rerank, union rerank, slice diagnostics, slice benchmark, and neighborhood settings. The UI already explains many route-specific prerequisites through state notes.

The request builder already enforces different legality rules for exact Double-MA kernel routes versus standalone unified-GPU host-side rerank routes. Export-all is its own optimize-side contract: it requires `mode=Grid`, `includeAll=false`, no MA parameter sweeps, a route that supports export, and no conflicting shortlist extras. Exact-kernel shortlist features have their own prerequisite matrix.

Backend behavior already computes exact shortlist diagnostics and GPU export CSV output on supported routes. That makes this surface a real product contract rather than a speculative UI.

## Current Code Surface

- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/stores/advancedStore.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_result.rs`

## Current Verification Surface

- `web/src/components/AdvancedWorkspacePanel.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `crates/app_core/src/double_ma.rs`

## Strengths Already Present

- advanced legality notes are already present in the UI
- request-builder guardrails are already strict and specific
- the product already distinguishes exact-kernel shortlist semantics from standalone host-side reranking
- export-all is already treated as a real contract with explicit restrictions
- backend result surfaces already carry relevant shortlist diagnostics

## Weaknesses / Failure Modes

- the legality matrix is large enough that it is easy for contributors to misunderstand
- shortlist controls and export-all semantics live close together but are not the same feature family
- stale advanced values can be visually hidden yet still relevant unless clearing rules are explicit
- save and restore behavior for advanced shortlist fields needs a stronger explicit contract
- users can confuse optimize-side export-all with validation/report export if documentation is weak

## Regressions To Avoid

- conflating exact-kernel shortlist semantics with standalone unified-GPU host-side behavior
- allowing invalid shortlist-plus-export combinations through the request builder
- losing clearability of stale advanced values when route support changes
- weakening export-all restrictions around mode, include-all, or MA param sweeps
- blending optimize-side export-all with validation-side report export in the docs or UI

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because this surface uses many conditional advanced controls with draft-state behavior.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because advanced control truth should remain canonical and serializable.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because this feature family is mostly legality and restore behavior, which benefits from store-aware tests.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because these advanced controls become structured payload fields with strong compatibility expectations.

## Proposed Improvement Approach

1. Build one explicit legality matrix for shortlist and export controls, split by route family.
2. Document optimize-side export-all separately from validation/report export features.
3. Freeze stale-value clearing and restore behavior for advanced shortlist fields.
4. Keep UI state notes synchronized with request-builder legality.
5. Add or tighten tests around restore, hidden stale values, and conflicting control combinations.

## Checklist

- [ ] Document route-specific legality for all shortlist and export controls.
- [ ] Document optimize-side export-all separately from validation/report export.
- [ ] Document stale-value clearing and restore rules.
- [ ] Add or tighten tests for conflicting advanced-control combinations.
- [ ] Add parity checks between UI guidance and request-builder behavior.

## Verification Plan

- run `npm --prefix web test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run targeted Rust optimization tests if shortlist or export result surfaces change

## Rollout / Fallback / Compatibility Notes

- preserve current strict legality rather than silently broadening advanced controls
- prefer clearer notes and explicit matrices before changing UI exposure
- keep optimize-side export-all and validation-side export clearly separated in names and docs
- if hidden stale fields are auto-cleared later, make the behavior explicit and test it

## Related Docs

- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-09.md`
- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/04_results/14_result_export_csv_2026-04-08.md`
