# Meta Artifact Replay Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to saved-meta artifact replay so typed replay loading, report-open behavior, selection retention, and Meta-workspace restoration remain explicit and dependable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- saved-meta replay/load/reveal/open-report/delete flows

Out of scope:

- Stage 5 and policy-eval replay families
- artifact indexing beyond the saved-meta family slice
- options-related artifact replay

## Current User-Facing Behavior

Saved Meta currently supports refresh, load, open report, reveal file, tag updates, and delete. Loading a saved meta artifact uses a typed replay command, restores the current meta-selection result state, refreshes the saved list, and returns the user to the Meta results tab. This is not workflow hydration; it is typed render-state restoration for the Meta workspace.

Saved Meta is also the only aggregate artifact family that currently exposes an `Open report` action, so preserving report-path behavior here is especially important.

## Current Code Surface

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.test.ts`

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `src-tauri/src/run_artifacts.rs` replay tests

## Strengths Already Present

- replay already uses a typed response rather than raw artifact JSON
- saved-meta refresh and selection retention are already implemented
- report-path behavior is already distinct from reveal/load behavior
- the Meta workspace already has a dedicated saved-meta family surface
- family actions are already explicitly tested

## Weaknesses / Failure Modes

- replay, reveal, open-report, and delete are adjacent enough to blur without a written contract
- report-path availability is special to this family and could be lost during library refactors
- refresh can accidentally clear selection if identity semantics drift
- replay can silently depend on raw artifact shape if the typed response stops being treated as authoritative

## Regressions To Avoid

- dropping `report_xlsx_path` from saved-meta state
- breaking the distinction between load/replay and reveal/open-report
- clearing the selected saved meta after refresh
- making saved-meta replay depend on raw artifact shape rather than the typed replay response
- removing the only aggregate-family report-open path by accident
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because the typed replay response is effectively a schema-governed contract for Meta restoration.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because replay payload stability depends on predictable JSON response semantics.
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because saved-meta replay is a command-mediated frontend/backend restoration path.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because replay restores shared Meta workspace state, not a local-only panel cache.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the important guarantees are selection retention and correctly restored state after typed replay.

## Proposed Improvement Approach

1. Freeze saved-meta replay as a typed restoration contract: replay payload, report path, selection retention, and action semantics.
2. Distinguish clearly between:
   - replay/load
   - reveal file
   - open report
   - delete
3. Keep typed replay authoritative over raw artifact internals.
4. Treat report-path preservation as a first-class saved-meta requirement.
5. Expand tests around refresh retention, empty payload handling, and report-open behavior.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the saved-meta typed replay response and how the frontend consumes it.
- [ ] Document the distinction between replay/load, reveal, and open-report actions.
- [ ] Document selection-retention behavior during refresh.
- [ ] Document report-path persistence and why this family exposes `Open report`.
- [ ] Add or tighten tests for typed replay loading, empty-payload handling, and report-open flows.
- [ ] Add or tighten tests for refresh preserving the selected saved-meta artifact.
- [ ] Keep replay explicitly separate from workflow hydration semantics.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run targeted `run_artifacts.rs` replay tests
- manually verify refresh, load, open-report, reveal, and delete flows inside Saved Meta
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve typed replay response ownership in the backend
- keep report-open support explicit for Saved Meta until a broader aggregate-report policy exists
- prefer additive replay hardening over changing family actions
- if replay payload fields change, update Meta service and tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/31_stage5_result_surface_2026-04-08.md`
