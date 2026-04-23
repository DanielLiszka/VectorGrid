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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

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
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the canonical backend aggregate-library fields and family slices.
- [x] Document how History and Meta currently consume different artifact sets.
- [x] Document sorting/count and refresh-selection semantics for aggregate families.
- [x] Document field-name normalization rules between backend and frontend.
- [x] Decide whether unified non-options indexing is a later goal or explicitly out of scope for now.
- [x] Add or tighten tests for family refresh, selection retention, and normalization.
- [x] Keep options-expression indexing excluded from this non-options doc.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/historyStore.search.test.ts`
- run `npm --prefix web run test -- src/stores/historyStore.actions.test.ts`
- run targeted `run_artifacts.rs` index/list tests
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve current split ownership between History and Meta until a deliberate consolidation plan exists
- keep backend aggregate-library field names stable or migrate consumers in the same change
- prefer additive family-index hardening over UI consolidation
- continue excluding options-specific library surfaces from this non-options program
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/06_artifacts/26_history_service_store_2026-04-08.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`

## Execution Notes

- Closed as an aggregate-library index hardening slice without consolidating History and Meta into one UI surface.
- Added a direct frontend regression test in `web/src/services/metaService.test.ts` proving aggregate library refresh still accepts backend `policy_evals` snake_case while preserving the selected policy-eval artifact in the current analysis store.
- Re-verified current split ownership: run artifacts remain History-owned, aggregate meta/stage5/policy-eval slices remain Meta-owned, and options-expression indexing stays out of scope for this non-options contract.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/services/metaService.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.
