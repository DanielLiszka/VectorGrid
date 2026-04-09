# Advanced Shortlist Diagnostics Plan

Last updated: 2026-04-08

## Purpose

Plan improvements to the non-options advanced-shortlist diagnostic surface so advanced search/export controls are explained consistently, route prerequisites are truthful, and exact-compatible vs host-side rerank behavior is not conflated.

## Scope Boundaries

In scope:

- advanced shortlist/exact diagnostic controls
- prerequisite messaging
- request-builder enforcement vs backend behavior
- export and rerank diagnostic visibility

Out of scope:

- options optimization diagnostics
- redesigning unrelated optimize controls
- inventing new shortlist features

## Current User-Facing Behavior

The advanced workspace exposes shortlist and exact-oriented controls, with helper text that tries to explain when certain features are legal or meaningful. The request builder and backend also enforce prerequisites, so the visible hints and actual behavior can diverge when the rules evolve.

## Current Code Surface

- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/services/optimizeRequestBuilder.ts`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_result.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/components/AdvancedWorkspacePanel.test.ts`

## Current Verification Surface

- `web/src/components/AdvancedWorkspacePanel.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- backend shortlist/exact-output tests in `ta_app_core`

## Strengths Already Present

- advanced controls are already visible instead of hidden behind undocumented flags
- request-builder coverage already exists for important combinations
- backend output logic already distinguishes shortlist-oriented behavior from basic optimization output
- the UI already gives users some route-sensitive guidance

## Weaknesses / Failure Modes

- prerequisite logic is duplicated across UI hints, request-builder validation, and backend route handling
- similar terms can mean different things across exact-kernel, exact-compatible, and host-side rerank flows
- non-default combinations can leave stale or misleading clearable values in the UI
- users can misunderstand whether a diagnostic control affects ranking, export, rerun cost, or all three

## Regressions To Avoid

- hiding advanced controls that are currently useful
- allowing invalid advanced combinations through the request builder
- collapsing exact-oriented modes into one label that loses route differences
- breaking export behavior or shortlist count assumptions
- changing advanced defaults without documenting the impact on existing workflows

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces that backend legality still matters even when the UI provides fast hints.
- [Vue Forms](https://vuejs.org/guide/essentials/forms) is relevant because advanced controls depend on keeping form state and derived legality synchronized.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting a structured advanced-control contract if the payload shape is formalized later.

## Proposed Improvement Approach

1. Enumerate every advanced shortlist-related control and map it to the exact backend/output behavior it influences.
2. Separate exact-kernel, exact-compatible, and host-side shortlist/rerank semantics in both copy and tests.
3. Make hard prerequisites explicit, including `top_k`, export dependencies, scoring mode, and any route-family limitations.
4. Define how stale advanced values are cleared, preserved, or surfaced when the route changes.
5. Add cross-layer parity coverage so UI diagnostics, request-builder validation, and backend handling stay aligned.

## Checklist

- [ ] Inventory all advanced shortlist/exact-related controls and payload fields.
- [ ] Document each control’s effect on ranking, export, rerun behavior, and output shape.
- [ ] Separate exact-kernel vs host-side rerank semantics in user-facing copy.
- [ ] Document hard prerequisites and route-family limitations.
- [ ] Define stale-value behavior when route or mode changes.
- [ ] Add tests for non-default advanced combinations and clearable-state transitions.
- [ ] Add parity checks between UI diagnostics, request builder, and backend behavior.
- [ ] Preserve existing safe advanced defaults unless there is a documented improvement reason.

## Verification Plan

- run `npm test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run targeted backend shortlist/exact-output tests in `ta_app_core`
- run a manual optimize flow with advanced shortlist controls if payload semantics change

## Rollout / Fallback / Compatibility Notes

- preserve currently legal advanced workflows
- prefer clearer copy and stricter parity over removing controls
- if terminology changes, keep legacy wording visible in release notes or inline helper text until users can adjust

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_DATE.md`
- `REPO_Improvement/03_opt/17_opt_cancel_progress_DATE.md`
- `REPO_Improvement/05_validation/18_csv_shortlist_export_DATE.md`
