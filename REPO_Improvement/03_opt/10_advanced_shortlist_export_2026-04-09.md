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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because this surface uses many conditional advanced controls with draft-state behavior.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because advanced control truth should remain canonical and serializable.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because this feature family is mostly legality and restore behavior, which benefits from store-aware tests.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because these advanced controls become structured payload fields with strong compatibility expectations.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Build one explicit legality matrix for shortlist and export controls, split by route family.
2. Document optimize-side export-all separately from validation/report export features.
3. Freeze stale-value clearing and restore behavior for advanced shortlist fields.
4. Keep UI state notes synchronized with request-builder legality.
5. Add or tighten tests around restore, hidden stale values, and conflicting control combinations.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document route-specific legality for all shortlist and export controls.
- [x] Document optimize-side export-all separately from validation/report export.
- [x] Document stale-value clearing and restore rules.
- [x] Add or tighten tests for conflicting advanced-control combinations.
- [x] Add parity checks between UI guidance and request-builder behavior.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run targeted Rust optimization tests if shortlist or export result surfaces change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current strict legality rather than silently broadening advanced controls
- prefer clearer notes and explicit matrices before changing UI exposure
- keep optimize-side export-all and validation-side export clearly separated in names and docs
- if hidden stale fields are auto-cleared later, make the behavior explicit and test it
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed the remaining stale-value clearability gap at the panel layer. `AdvancedWorkspacePanel.vue` now keeps `exportAllCsvPath` visible whenever a saved value exists, even on routes that no longer support export-all, and surfaces an explicit unsupported-route note instead of hiding the field outright. That matches the existing stale-value strategy already used for shortlist slice controls.
- Kept legality authoritative in the request builder rather than widening product behavior. `optimizeRequestBuilder.ts` and `optimizeService.test.ts` already covered the route-specific shortlist/export restrictions, including export-all vs extra-shortlist/union/confidence/slice conflicts and composite route export legality. The UI change only preserves visibility and clearability for stale values; it does not broaden the allowed request surface.
- Added direct panel coverage for the stale export-path case. `AdvancedWorkspacePanel.test.ts` now proves an unsupported route still renders the saved export path so the user can clear it, and it asserts the new unsupported-route copy. Existing builder and service tests continue to cover the deeper legality matrix for shortlist and export combinations.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/AdvancedWorkspacePanel.test.ts src/stores/optimizeStore.actions.test.ts src/stores/advancedStore.actions.test.ts src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-09.md`
- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/04_results/14_result_export_csv_2026-04-08.md`
