# Validation Panel Core Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the core validation panel so configuration controls, route/evidence summaries, execution actions, and dense review output remain coherent and trustworthy as one analyst-facing workspace.

## Scope Boundaries

In scope:

- `web/src/components/ValidationWorkspacePanel.vue`
- validation-stage readiness, summary chips, and core panel layout
- primary validation action placement and core control grouping
- panel coupling to `validationStore`, `validationService`, and workflow state

Out of scope:

- low-level request-shaping details already covered by the request-builder doc
- workbook rendering internals
- options-related validation flows

## Current User-Facing Behavior

The validation panel already acts as a full workspace, not a single button plus report link. It exposes candidate/admission controls, module toggles, cross-dataset selection, column visibility, export actions, and a dense result table. It also publishes route summary, warning/skipped summaries, cross summary, execution preview text, and a primary inline `Run validation` action.

Current behavior already distinguishes readiness states through `useUiReadiness`, shows summary chips describing stage, route, row count, and cross coverage, and keeps execution preview/admission comparison visible before or after a run. The panel therefore serves as both configuration surface and evidence surface.

## Current Code Surface

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/stores/validationStore.ts`
- `web/src/services/validationService.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/composables/useUiReadiness.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/stores/validationStore.actions.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/validationRequestBuilder.test.ts`

## Strengths Already Present

- the panel already keeps the primary validation action inline and prominent
- configuration, evidence, and export surfaces already coexist in one panel
- summary chips and route/execution preview text already reduce hidden state
- the panel already supports dense review via persisted table columns and virtualization
- store/service ownership is already separated enough to avoid component-only business logic

## Weaknesses / Failure Modes

- the panel is broad enough that small layout changes can quietly hide critical control families
- readiness, preview, and post-run summary state are easy to blur without a dedicated panel-level contract
- there are many toggles and numeric inputs, so grouping and hierarchy matter more than raw presence
- export/report actions can look available even when the evidence path is not obvious
- there is no dedicated implementation doc freezing the panel as a full validation workspace rather than a thin launcher

## Regressions To Avoid

- burying the primary `Run validation` action behind lower-signal controls
- hiding route, warning, or skipped-module summaries that currently explain the validation result
- weakening the linkage between preview state and post-run state
- reducing visibility of export/report actions that analysts already rely on
- moving significant validation logic back into the component from the store/service layers

## External Research Summary

- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because validation is one panel in the broader results-tab workflow and still needs a self-contained role.
- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because the validation panel includes an interactive dense table alongside controls.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because route summaries, warning summaries, and execution previews are dynamic advisory output.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the panel should remain a reader/writer of shared validation state rather than a second state owner.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the panel contract is mostly about deterministic state, action placement, and rendered readiness.

## Proposed Improvement Approach

1. Freeze Validation as a full workspace with four responsibilities: configuration, preview, execution, and review.
2. Document the major control groups and which summary surfaces must stay visible near them.
3. Keep route/execution/admission preview text as part of the core panel contract rather than optional supporting copy.
4. Preserve the separation between panel layout, store state, service execution, and request building.
5. Add or tighten tests around core action placement, visible control groups, summary chips, and readiness transitions.

## Checklist

- [ ] Document the core validation workspace responsibilities and panel sections.
- [ ] Document the visibility requirements for route, admission, warning, and cross summaries.
- [ ] Document how readiness states affect panel output and action availability.
- [ ] Document the core export/report action cluster and its intended placement.
- [ ] Add or tighten tests for core action placement, summary visibility, and readiness behavior.

## Verification Plan

- run `npm --prefix web test -- src/components/ValidationWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/validationStore.actions.test.ts`
- run `npm --prefix web test -- src/services/validationService.test.ts`
- manually verify pre-run preview, post-run summaries, and dense table visibility in the validation tab

## Rollout / Fallback / Compatibility Notes

- preserve the current panel-wide workflow instead of splitting it into disconnected micro-surfaces without a clear migration
- keep panel logic thin relative to the store/service boundary
- prefer additive clarity improvements over removing existing validation evidence
- if panel grouping changes later, update readiness and export/action tests in the same change

## Related Docs

- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/21_validation_store_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/03_validate_results_table_2026-04-09.md`
