# Artifact Library Index Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the non-options artifact library index so backend summary contracts, family-specific library consumers, sorting/count semantics, and future consolidation decisions stay explicit and stable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/services/historyService.ts`
- artifact library indexing for run, meta, stage5, and policy-eval families

Out of scope:

- options-expression artifact indexing
- artifact replay payload details
- low-level artifact deletion and tag-update mechanics except where they depend on index semantics

## Current User-Facing Behavior

The backend already exposes an aggregate `ArtifactLibrarySnapshot` for non-run artifact families such as saved meta, saved Stage 5, and saved policy-evaluation results. The frontend currently uses that snapshot inside the Meta workspace, while run artifacts remain a separate history-driven surface fed by `list_run_artifacts`.

This means the product already has two consumers of artifact indexing:

- History for saved runs
- Meta for aggregate artifact families

Those two consumers are adjacent but not unified, and the difference is already part of the current UX.

## Current Code Surface

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/stores/historyStore.search.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- `src-tauri/src/run_artifacts.rs` tests

## Strengths Already Present

- the backend already has a canonical aggregate library snapshot for non-run families
- Meta-family refresh and selection-retention behavior already exist
- saved-run indexing is already stable in the separate History surface
- family-specific labels and empty states are already explicit in the Meta workspace
- index consumers already normalize backend field names before display

## Weaknesses / Failure Modes

- the repo currently has two adjacent indexing models rather than one explicit overall contract
- field-name normalization such as `policyEvals` versus `policy_evals` can drift
- selection retention on refresh is subtle and easy to break
- future consolidation pressure could blur run artifacts with meta-family artifacts before the contract is written down

## Regressions To Avoid

- resetting selected artifacts on refresh
- drifting sorting/count semantics between backend index output and frontend consumers
- accidentally pulling options-expression rows into the non-options library plan
- making History and Meta disagree on what constitutes the latest or most relevant artifact
- changing aggregate library field names without updating the consumers together

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because `ArtifactLibrarySnapshot` already behaves like a structured backend index contract.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because normalized family slices depend on stable JSON object/array semantics.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because index-level identity, sorting, and diff behavior benefit from deterministic structural assumptions.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because History and Meta are two consumers of shared artifact state, not isolated widgets.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because current refresh/selection-retention guarantees are already tested through stateful service flows.

## Proposed Improvement Approach

1. Treat `ArtifactLibrarySnapshot` as the canonical backend aggregate-library contract for non-run families.
2. Document explicitly that History and Meta are separate consumers of different artifact slices today.
3. Freeze sorting, count, and selection-retention semantics for the current consumers.
4. Decide in the doc whether future consolidation should:
   - keep run artifacts separate
   - or promote a unified non-options library index
5. Expand tests around family refresh and selection retention before any UI consolidation work.

## Checklist

- [ ] Document the canonical backend aggregate-library fields and family slices.
- [ ] Document how History and Meta currently consume different artifact sets.
- [ ] Document sorting/count and refresh-selection semantics for aggregate families.
- [ ] Document field-name normalization rules between backend and frontend.
- [ ] Decide whether unified non-options indexing is a later goal or explicitly out of scope for now.
- [ ] Add or tighten tests for family refresh, selection retention, and normalization.
- [ ] Keep options-expression indexing excluded from this non-options doc.

## Verification Plan

- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/historyStore.search.test.ts`
- run `npm --prefix web test -- src/stores/historyStore.actions.test.ts`
- run targeted `run_artifacts.rs` index/list tests

## Rollout / Fallback / Compatibility Notes

- preserve current split ownership between History and Meta until a deliberate consolidation plan exists
- keep backend aggregate-library field names stable or migrate consumers in the same change
- prefer additive family-index hardening over UI consolidation
- continue excluding options-specific library surfaces from this non-options program

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/06_artifacts/26_history_service_store_2026-04-08.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
