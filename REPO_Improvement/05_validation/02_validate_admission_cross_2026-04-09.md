# Validation Admission And Cross-Dataset Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation admission and cross-dataset behavior so shortlist entry rules, admission comparison, cross-selection, and cross-summary messaging stay explicit and analytically trustworthy.

## Scope Boundaries

In scope:

- admission-policy controls and preview text
- cross-dataset enablement, selection, and summary behavior
- preserved-finalists versus lightweight cross semantics
- store/service behavior that feeds admission and cross summaries

Out of scope:

- low-level report workbook formatting
- validation result-table column formatting
- options-related validation workflows

## Current User-Facing Behavior

The validation workspace already exposes admission policy as a visible control and can preview how current admission differs from legacy behavior. It surfaces overlap and head composition in the admission comparison text, which means admission is already treated as user-visible evidence rather than hidden implementation detail.

Cross-dataset behavior is also already explicit. Users can enable cross validation, select datasets, search/filter them, choose a mode, and see summary text describing requested mode, selected counts, and omitted modules. Post-run summaries already explain when cross reruns were unavailable or when preserved-finalists semantics changed which modules ran.

That makes admission and cross-dataset logic part of the analyst-facing contract, not just the backend payload.

## Current Code Surface

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/validationService.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/stores/validationStore.ts`
- `web/src/services/dataService.ts`
- `src-tauri/src/commands_validation.rs`

## Current Verification Surface

- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- targeted validation command tests in `src-tauri/src/commands_validation.rs`

## Strengths Already Present

- admission policy is already visible and previewable in the UI
- admission provenance already survives into the request contract
- cross-dataset selection already has search, counts, and helper actions
- post-run summaries already explain omitted modules for cross reruns
- composite and double-like validation routes already surface cross semantics in tests

## Weaknesses / Failure Modes

- admission policy and cross behavior are both subtle enough that generic wording can mislead users
- preserved-finalists behavior is powerful but easy to overstate if summary text weakens
- cross-selection state can drift from visible rows if filter and selection helpers regress
- admission comparison can become stale if it is not rebuilt when current run or policy changes
- there is no dedicated doc freezing admission and cross as one analyst-facing decision surface

## Regressions To Avoid

- hiding or weakening admission comparison text when policy differs from legacy
- silently changing cross mode semantics without updating summaries and tests
- breaking cross-selection counts, filter behavior, or visible-selection helpers
- losing omitted-module explanations for cross reruns
- letting admission/cross summary text diverge from the actual emitted request

## External Research Summary

- [TimeSeriesSplit | scikit-learn](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because chronology-aware evaluation and held-out comparison semantics are central to validation framing.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because admission and cross-selection state are shared workspace state, not isolated widget state.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because admission and cross behavior depend on deterministic store/service state transitions.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because admission provenance and cross selection ultimately cross the backend boundary as structured validation request data.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because admission provenance and cross-dataset config already behave like a nested structured contract.

## Proposed Improvement Approach

1. Treat admission policy and cross-dataset selection as one evidence family inside validation, because both determine what gets evaluated and how broadly it is stress-tested.
2. Freeze admission preview text and cross-summary text as first-class analyst guidance.
3. Keep preserved-finalists versus lightweight cross semantics explicit in both preview and post-run summaries.
4. Keep selection helpers and counts honest relative to visible filtered rows.
5. Add or tighten tests around policy preview, cross-mode summaries, and filtered cross selection.

## Checklist

- [ ] Document admission policy control behavior and preview semantics.
- [ ] Document cross enablement, mode selection, and dataset-selection rules.
- [ ] Document preserved-finalists versus lightweight cross summary language.
- [ ] Document visible-row filtering and cross-selection helper behavior.
- [ ] Add or tighten tests for admission preview, cross summaries, and selection counts.

## Verification Plan

- run `npm --prefix web test -- src/components/ValidationWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/validationService.test.ts`
- run `npm --prefix web test -- src/services/validationRequestBuilder.test.ts`
- run targeted validation command tests where cross-mode semantics or admission provenance changes

## Rollout / Fallback / Compatibility Notes

- preserve current admission provenance richness and cross summary detail until all consumers migrate together
- if admission defaults change later, update comparison text and tests in the same change
- prefer explicit omitted-module explanations over silent capability downgrades
- keep preserved-finalists semantics conservative unless backend/runtime support expands intentionally

## Related Docs

- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/21_validation_store_2026-04-08.md`
- `REPO_Improvement/02_data/21_polygon_selection_ux_2026-04-09.md`
