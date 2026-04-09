# Validation Context Docks Plan

Last updated: 2026-04-08

## Purpose

Define the current validation-context dock contract so the shell can keep showing compact validation summaries and quick handoff actions without turning the dock into a competing validation workspace or drifting from the real compatibility contract.

## Scope Boundaries

In scope:

- `web/src/components/ValidationContextDock.vue`
- validation summary fields currently shown in the dock
- dock visibility and workflow-step handoff behavior
- frontend/Rust compatibility information the dock ought to summarize truthfully

Out of scope:

- validation workspace editing controls
- report export behavior
- strategy-specific validation backend internals beyond current compatibility checks

## Current User-Facing Behavior

The app already exposes a dedicated validation context dock. It is only shown in the workflow workspace on the `validate` results tab, not in `ml-signal` mode. It shows validation meta, current table row count, cross-ticker summary, compact gate summary, status line, and validation preview text, plus one action that takes the user back to the validation workspace. The actual compatibility contract already lives underneath that summary surface: the builder emits `validation_context`, the frontend blocks mismatched dataset/timeframe/strategy before invoke, and Rust re-checks those assumptions.

## Current Code Surface

- `web/src/App.vue`
  - controls current dock mounting conditions
- `web/src/components/ValidationContextDock.vue`
  - renders current dock summary state and workflow-step handoff
- `web/src/services/validationRequestBuilder.ts`
  - builds `validation_context` with dataset/timeframe/strategy identity
- `web/src/services/validationService.ts`
  - enforces frontend compatibility/mismatch rules before validation invoke
- `src-tauri/src/commands_validation.rs`
  - re-validates strategy allowlists and dataset/timeframe compatibility on the Rust side
- `web/src/stores/validationStore.ts`
  - owns the current summary text the dock displays

## Current Verification Surface

- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`

## Strengths Already Present

- the dock is already intentionally narrow and easy to understand
- it reuses current store truth instead of duplicating validation logic
- the real compatibility contract is already checked in both frontend and Rust layers
- the handoff back to the validation workspace is explicit

## Weaknesses / Failure Modes

- the dock contract is almost entirely implicit in the component
- the dock only mirrors text today and does not expose explicit compatibility/staleness state
- some displayed fields are very compressed, so wording changes could degrade clarity quickly
- there is no dedicated dock test coverage today

## Regressions To Avoid

- letting the dock become a second editing surface for validation
- showing stale or misleading validation summaries that diverge from builder/service truth
- removing the workflow-step handoff and forcing manual navigation back to validation
- moving authority for compatibility checks out of the frontend/Rust validation pipeline into the dock
- making thin or legacy artifact metadata fail earlier than today without an explicit degraded-state message

## External Research Summary

- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the dock should remain a projection of centralized validation state
- [Pinia: State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because the dock depends on `validationStore` rather than component-local logic
- [MDN: ARIA `status` Role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because the dock communicates changing summary state to the user
- [WAI-ARIA APG: Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
  - relevant because the dock routes back into a specific workflow/result surface instead of replacing it

## Proposed Improvement Approach

1. Treat the validation dock as a compact context surface with explicit limits:
   - summarize
   - signal freshness/status
   - provide one handoff action
2. Document the fields it is allowed to show today:
   - `validationMeta`
   - `tableRowCount`
   - `crossMeta`
   - compact gate summary
   - `statusLine`
   - `validationOutputPreview`
3. Add explicit compatibility/staleness wording sourced from the same builder/service truth instead of inventing separate dock logic.
4. Add dedicated dock tests rather than relying only on builder/service/panel coverage.

## Checklist

- [ ] Document current validation dock fields and current handoff behavior.
- [ ] Document that the dock is summary-only and not a validation-edit surface.
- [ ] Document how dataset/timeframe/strategy compatibility truth reaches the dock indirectly today.
- [ ] Document responsive/visibility expectations for the dock.
- [ ] Add or tighten tests if dock visibility or displayed summary semantics change.

## Verification Plan

- run `npm --prefix web test -- src/services/validationRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/validationService.test.ts`
- run `npm --prefix web test -- src/components/ValidationWorkspacePanel.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve the current summary-only posture of the dock
- if the dock is redesigned, keep the existing “Open validation” handoff until a replacement navigation pattern exists
- keep dock copy compact; avoid multi-line complexity that belongs in the workspace panel

## Related Docs

- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/21_validation_store_2026-04-08.md`
- `REPO_Improvement/04_results/18_context_docks_2026-04-08.md`
