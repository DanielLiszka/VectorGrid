# Policy-Eval Artifact Replay Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to saved policy-evaluation artifact replay so typed JSON replay, family refresh/selection behavior, and action-surface expectations remain explicit and dependable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- saved policy-evaluation replay/load/reveal/delete flows

Out of scope:

- saved-meta report-open behavior
- Stage 5 replay alias debt
- options-related artifact replay

## Current User-Facing Behavior

Saved Policy Eval supports refresh, load, reveal, and delete, with no report-open action. Loading a saved policy-evaluation artifact restores the policy-evaluation result surface inside the Meta workspace, updates the selected saved item, and leaves the user in the relevant saved-family workflow.

This family is a JSON-result replay surface, not a workbook/report family. That distinction already matters for the current UX and should remain explicit.

## Current Code Surface

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.test.ts`

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `src-tauri/src/run_artifacts.rs` list/replay tests

## Strengths Already Present

- the family already uses a typed replay response
- refresh, selection, and action menus already exist
- no-report behavior is already intentional rather than accidental
- tests already cover family actions and typed replay loading
- backend summary fields already surface policy-eval-specific signals such as `best_policy`

## Weaknesses / Failure Modes

- policy-eval replay can be treated like a generic meta replay unless its JSON-only contract is explicit
- `policyEvals` versus `policy_evals` normalization can drift
- future library refactors could accidentally introduce report expectations for this family
- selection retention and result restoration can break quietly if the typed contract is not frozen

## Regressions To Avoid

- losing policy-eval-specific summary fields such as `best_policy`
- breaking `policyEvals` versus `policy_evals` normalization
- adding or implying report-open behavior where none currently exists
- clearing the selected policy-eval artifact on refresh
- making policy-eval replay depend on raw artifact structure instead of the typed response

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because policy-eval replay is a typed JSON-result contract, not an ad hoc payload.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because replay depends on stable JSON field semantics.
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because the saved policy-eval replay path is a command-mediated frontend/backend restoration flow.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because policy-eval replay restores shared Meta state and selected saved-family state.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because family refresh and selection guarantees are currently enforced through stateful service tests.

## Proposed Improvement Approach

1. Freeze policy-eval replay as a JSON-result replay contract with no workbook/report expectations.
2. Document the specific summary/result fields the UI actually needs to preserve.
3. Keep field-name normalization stable between backend and frontend.
4. Preserve family refresh and selection behavior during replay-related changes.
5. Expand tests around typed replay, field normalization, and no-report action constraints.

## Checklist

- [ ] Document the typed policy-eval replay response and consumed result fields.
- [ ] Document why this family is JSON-only and does not expose `Open report`.
- [ ] Document `policyEvals` versus `policy_evals` normalization rules.
- [ ] Document selection-retention behavior during refresh.
- [ ] Add or tighten tests for typed replay loading and summary-field preservation.
- [ ] Add or tighten tests for field-name normalization and no-report action behavior.
- [ ] Keep this family separate from workbook/report-oriented replay semantics.

## Verification Plan

- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/components/MetaWorkspacePanel.test.ts`
- run targeted `run_artifacts.rs` policy-eval list/replay tests
- manually verify refresh, load, reveal, and delete flows inside Saved Policy Eval

## Rollout / Fallback / Compatibility Notes

- preserve JSON-only replay expectations unless a real policy-eval report artifact is introduced
- keep frontend normalization aligned with backend field names
- prefer additive hardening over family-surface expansion
- if additional summary fields are surfaced later, keep typed replay backward compatible

## Related Docs

- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
- `REPO_Improvement/06_artifacts/06_stage5_artifact_replay_2026-04-09.md`
- `REPO_Improvement/08_ml/25_meta_replay_routes_2026-04-08.md`
- `REPO_Improvement/08_ml/34_policy_eval_outputs_2026-04-08.md`
