# Validation Results Table Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the validation results table so dense candidate-review output stays understandable, keyboard-friendly, and truthful about shortlist, gate, cross-ticker, and robustness semantics.

## Scope Boundaries

In scope:

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/composables/useVirtualTableRows.ts`

Out of scope:

- request-builder rules that decide which candidates enter validation
- workbook/report rendering
- options-specific validation surfaces

## Current User-Facing Behavior

The validation table only appears once validation reaches a ready/results state. It is a dense review surface driven by service-built `validationColumns` and `validationRows`, not by ad hoc component formatting. Current default columns include shortlist identity plus robustness/explanation columns such as `Robust`, `PSR`, `DSR`, `CSCV Rank`, `Gate`, `Robust Pareto`, `Cross`, `Cross Pareto`, `PnL`, `Sharpe`, `Max DD`, and `Trades`.

The table already supports persisted column visibility, restore-defaults behavior, and virtualized row rendering. The surrounding panel also publishes row count, visible-column count, and column-visibility help text so users can tighten the table without fully losing important explanatory columns.

## Current Code Surface

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/composables/useVirtualTableRows.ts`
- `web/src/utils/core/storageKeys.ts`

## Current Verification Surface

- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/stores/validationStore.actions.test.ts`

## Strengths Already Present

- the table is already service-derived instead of being manually reshaped inside the component
- persisted column visibility already exists and is restorable
- virtualization is already in place for denser result sets
- the table exposes gate and cross-ticker semantics instead of hiding them in secondary detail
- the component already summarizes row-count and visible-column state for the user

## Weaknesses / Failure Modes

- the table has enough columns that first-run comprehension can degrade if defaults are not carefully chosen
- shortlist-rule semantics live partly in row labels and partly in summary text, which can drift
- gate, pareto, and cross columns are easy to treat as optional visual noise even though they explain recommendation logic
- virtualization and persisted-column behavior make regressions easy to miss if only happy-path rendering is tested
- the current implementation lacks one dedicated contract doc for what must stay visible versus what is optional expert density

## Regressions To Avoid

- losing persisted column visibility or restore-default behavior
- changing shortlist/gate semantics without reflecting them in row-level presentation
- removing or obscuring `Gate`, `Robust Pareto`, or `Cross Pareto` context that explains why a candidate is kept or rejected
- making `tableVisible` or `tableRowCount` disagree with the actual rendered state
- weakening keyboard-friendly dense review by replacing the table with a purely decorative output surface
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because the validation table is an interactive dense review surface that must remain keyboard-navigable.
- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because validation is one tab in the broader results workflow and should preserve stable panel semantics while hosting a large table.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because rendered rows/columns should stay derived from shared store/service state rather than become component-owned business logic.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the most important table guarantees are state-driven and should remain testable through store/service inputs.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because dense-tabular flows benefit from user-observable tests around column visibility, empty states, and keyboard review behavior.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Freeze the validation table contract around three layers: row identity, explanatory metrics, and optional expert-density columns.
2. Document which columns are required for first-pass interpretation and which columns are safely user-hideable.
3. Keep column visibility persistence and virtualization as first-class behavior, not incidental implementation details.
4. Make row-level provenance clearer for shortlist rule, hard gates, robust pareto, and cross-ticker pareto semantics.
5. Add explicit coverage for branch states such as empty rows, restored columns, and mixed cross/non-cross result sets.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the canonical validation table column families and their intended meaning.
- [x] Document which columns should remain visible by default for first-run comprehension.
- [x] Document how persisted column state should behave across remounts and restore-defaults flows.
- [x] Document virtualization expectations and table-row identity invariants.
- [x] Document how shortlist, gate, pareto, and cross signals must remain legible at row level.
- [x] Add or tighten tests for persisted columns, restore defaults, and representative row rendering.
- [x] Add or tighten tests for mixed-result cases where cross-ticker columns are present for some runs and absent for others.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/ValidationWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/validationService.test.ts`
- run `npm --prefix web run test -- src/stores/validationStore.actions.test.ts`
- manually verify column hide/show, restore defaults, and high-row-count rendering in the validation tab
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve the current service-derived row/column contract first
- prefer additive clarity improvements over shrinking the result surface
- keep persisted column keys stable unless a coordinated migration is added
- if default columns change, update tests and user-facing hints in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the existing dense-table coverage in [ValidationWorkspacePanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/ValidationWorkspacePanel.test.ts), which now covers the ready-state summary shell plus persisted visible-column state and restore-default behavior.
- Revalidated [validationService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/validationService.test.ts) and [validationStore.actions.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/validationStore.actions.test.ts), which already exercise the service-derived row/column contract, mixed cross semantics, and the store-driven column/table metadata the panel consumes.
- No production table change was required in this tranche because the current result-table contract was already intact; the work here was to close the last panel-level regression gap and then verify the existing service/store path still upheld the documented behavior.

## Related Docs

- `REPO_Improvement/05_validation/01_validate_panel_core_2026-04-09.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/21_validation_store_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
