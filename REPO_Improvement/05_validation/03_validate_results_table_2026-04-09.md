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

## External Research Summary

- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because the validation table is an interactive dense review surface that must remain keyboard-navigable.
- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because validation is one tab in the broader results workflow and should preserve stable panel semantics while hosting a large table.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because rendered rows/columns should stay derived from shared store/service state rather than become component-owned business logic.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the most important table guarantees are state-driven and should remain testable through store/service inputs.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because dense-tabular flows benefit from user-observable tests around column visibility, empty states, and keyboard review behavior.

## Proposed Improvement Approach

1. Freeze the validation table contract around three layers: row identity, explanatory metrics, and optional expert-density columns.
2. Document which columns are required for first-pass interpretation and which columns are safely user-hideable.
3. Keep column visibility persistence and virtualization as first-class behavior, not incidental implementation details.
4. Make row-level provenance clearer for shortlist rule, hard gates, robust pareto, and cross-ticker pareto semantics.
5. Add explicit coverage for branch states such as empty rows, restored columns, and mixed cross/non-cross result sets.

## Checklist

- [ ] Document the canonical validation table column families and their intended meaning.
- [ ] Document which columns should remain visible by default for first-run comprehension.
- [ ] Document how persisted column state should behave across remounts and restore-defaults flows.
- [ ] Document virtualization expectations and table-row identity invariants.
- [ ] Document how shortlist, gate, pareto, and cross signals must remain legible at row level.
- [ ] Add or tighten tests for persisted columns, restore defaults, and representative row rendering.
- [ ] Add or tighten tests for mixed-result cases where cross-ticker columns are present for some runs and absent for others.

## Verification Plan

- run `npm --prefix web test -- src/components/ValidationWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/validationService.test.ts`
- run `npm --prefix web test -- src/stores/validationStore.actions.test.ts`
- manually verify column hide/show, restore defaults, and high-row-count rendering in the validation tab

## Rollout / Fallback / Compatibility Notes

- preserve the current service-derived row/column contract first
- prefer additive clarity improvements over shrinking the result surface
- keep persisted column keys stable unless a coordinated migration is added
- if default columns change, update tests and user-facing hints in the same change

## Related Docs

- `REPO_Improvement/05_validation/01_validate_panel_core_2026-04-09.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/21_validation_store_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
