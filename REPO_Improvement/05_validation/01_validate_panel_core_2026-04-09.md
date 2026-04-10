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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because validation is one panel in the broader results-tab workflow and still needs a self-contained role.
- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because the validation panel includes an interactive dense table alongside controls.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because route summaries, warning summaries, and execution previews are dynamic advisory output.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the panel should remain a reader/writer of shared validation state rather than a second state owner.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the panel contract is mostly about deterministic state, action placement, and rendered readiness.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Freeze Validation as a full workspace with four responsibilities: configuration, preview, execution, and review.
2. Document the major control groups and which summary surfaces must stay visible near them.
3. Keep route/execution/admission preview text as part of the core panel contract rather than optional supporting copy.
4. Preserve the separation between panel layout, store state, service execution, and request building.
5. Add or tighten tests around core action placement, visible control groups, summary chips, and readiness transitions.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the core validation workspace responsibilities and panel sections.
- [x] Document the visibility requirements for route, admission, warning, and cross summaries.
- [x] Document how readiness states affect panel output and action availability.
- [x] Document the core export/report action cluster and its intended placement.
- [x] Add or tighten tests for core action placement, summary visibility, and readiness behavior.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/ValidationWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/validationStore.actions.test.ts`
- run `npm --prefix web run test -- src/services/validationService.test.ts`
- manually verify pre-run preview, post-run summaries, and dense table visibility in the validation tab
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve the current panel-wide workflow instead of splitting it into disconnected micro-surfaces without a clear migration
- keep panel logic thin relative to the store/service boundary
- prefer additive clarity improvements over removing existing validation evidence
- if panel grouping changes later, update readiness and export/action tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Tightened [ValidationWorkspacePanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/ValidationWorkspacePanel.test.ts) to cover the missing panel-level contract: no-data and no-run empty states, the ready-to-run preview state, disabled action clusters, and the ready-state summary/debug surface.
- Revalidated the existing store/service surfaces in [validationStore.actions.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/validationStore.actions.test.ts), [validationService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/validationService.test.ts), and [validationRequestBuilder.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/validationRequestBuilder.test.ts), which already covered the heavier execution and request branches this panel consumes.
- No production component change was required in this tranche because the current panel structure already satisfies the intended workspace contract once the missing regression coverage was added.

## Related Docs

- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/21_validation_store_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/03_validate_results_table_2026-04-09.md`
