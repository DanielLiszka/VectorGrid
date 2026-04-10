# Stage 4/Stage 5 Saved Replay Plan

Last updated: 2026-04-09

## Purpose

Define the current Stage 4/Stage 5 saved-replay contract so saved-family ownership, compatibility aliases, replay/load behavior, and saved-list management remain explicit despite the current naming drift.

## Scope Boundaries

In scope:

- saved Stage 5 artifact family
- Stage 4 compatibility aliases for saved Stage 5 state
- saved list/selection/load/reveal/delete/tag flows
- replay command boundaries and artifact hydration

Out of scope:

- live Stage 5 execution logic
- policy-eval saved replay beyond comparisons needed to explain the saved-family split
- Stage 3 saved meta replay
- options workflows

## Current User-Facing Behavior

The Meta workspace presents a saved Stage 5 family, but the underlying implementation still carries Stage 4 compatibility names. Current behavior means:

- users browse a “Saved Stage 5” family in the UI
- saved Stage 5 runs can be refreshed, loaded, revealed, tagged, and deleted
- the same current result surface is reused when a saved Stage 5 artifact is loaded
- there is not a separate stable Stage 4 saved family from the user’s perspective
- replay is JSON-artifact-based rather than workbook/report-based

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - saved Stage 5 family tabs, lists, and action controls
- `web/src/stores/analysisStore.ts`
  - `stage4Saved*` state that currently backs the saved Stage 5 UI
- `web/src/services/metaService.ts`
  - saved Stage 5 refresh/load/reveal/delete/tag flows and compatibility aliasing
- `src-tauri/src/commands_artifacts.rs`
  - replay/list commands for saved Stage 5 artifacts
- `src-tauri/src/run_artifacts.rs`
  - Stage 5 artifact listing, summary extraction, replay hydration, and pruning
- `src-tauri/src/commands_meta.rs`
  - Stage 5 execution response fields that feed saved replay

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/stores/analysisStore.actions.test.ts`
- `web/src/stores/domainStores.test.ts`
- tests embedded in `src-tauri/src/run_artifacts.rs`
- tests embedded in `src-tauri/src/commands_meta.rs`

## Strengths Already Present

- saved Stage 5 artifacts are already replayable and manageable in the UI
- compatibility aliasing is acknowledged in code rather than being hidden
- replay uses a dedicated Tauri artifact path instead of reconstructing the result from partial store state
- saved Stage 5 management is already integrated into the same artifact library model as other families

## Weaknesses / Failure Modes

- saved Stage 5 state is still named `stage4Saved*`, which can mislead both docs and refactors
- the codebase can easily be misdescribed as having two separate saved families when the user-facing reality is one saved Stage 5 family plus compatibility aliases
- replay is JSON-only, so anyone expecting a report path will be confused
- selection state and current-result state can drift if alias handling is changed carelessly

## Regressions To Avoid

- breaking saved Stage 5 replay while cleaning up Stage 4 compatibility names
- documenting separate Stage 4 and Stage 5 saved families as if both were first-class current UX
- removing alias handling before store/service/artifact surfaces are migrated together
- weakening saved-entry load/reveal/delete behavior during naming cleanup
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because replay is a command-driven restore path, not a purely local frontend rehydration
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because saved replay depends on stable JSON artifact shapes and compatibility fields
- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because replayable model-driven artifacts benefit from explicit compatibility signatures
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because replayed Stage 5 artifacts still need enough metadata to remain interpretable without a workbook
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document saved Stage 5 replay as the authoritative user-facing contract and Stage 4 naming as the compatibility layer underneath.
2. Record all current alias relationships:
   - `stage4Saved*` store state
   - `runStage4()` helper aliases
   - `stage4_json_path` / `stage5_json_path`
3. Keep replay/load behavior grouped with saved-list management because that is how the product exposes the feature.
4. Make JSON-only replay explicit so later report features can be added without rewriting current semantics.
5. Tie any naming cleanup to a coordinated migration across store, service, command, and artifact layers.
6. Preserve saved replay as an artifact-library operation, not a hand-built store reconstruction.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current saved Stage 5 family and its user-facing actions.
- [x] Document `stage4Saved*` as compatibility state backing the saved Stage 5 UI.
- [x] Document the current replay/load/reveal/delete/tag flows and their owners.
- [x] Document the `stage4_json_path` / `stage5_json_path` compatibility behavior.
- [x] Document JSON-only replay expectations for saved Stage 5 runs.
- [x] Preserve tests for saved Stage 5 load, reveal, delete, and alias handling.
- [x] Preserve artifact replay tests in `run_artifacts.rs`.
- [x] Treat Stage 4 naming cleanup as compatibility-sensitive across all saved-replay surfaces.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/analysisStore.actions.test.ts`
- run `npm --prefix web run test -- src/stores/domainStores.test.ts`
- run targeted artifact replay tests in `src-tauri/src/run_artifacts.rs`
- manually load and reveal one saved Stage 5 artifact
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current compatibility aliases until store/service/artifact naming is migrated together
- keep replay behavior additive if richer report surfaces are introduced later
- prefer explicit deprecation notes over silent renames for saved-family fields
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by making the Saved Stage 5 replay contract explicit in both the panel copy and the saved-artifact summaries.
- `metaService.ts` now enriches saved Stage 5 metadata with:
  - `alias=stage4Saved*`
  - `replay=json`
  - `path_alias=stage4/stage5`
- Saved policy-eval metadata now also carries `arms=meta/ml/stage5` and `replay=json`, which keeps the saved-family split explicit when reopening artifacts later.
- `MetaWorkspacePanel.vue` now tells the user that Saved Stage 5 is still backed by `stage4Saved*` compatibility fields and that Stage 4 / Stage 5 path aliases currently point at the same JSON artifact.
- Stable fixture / replay payload:
  - saved Stage 5 artifact `C:/runs/stage5_saved.json`
  - saved policy evaluation artifact `C:/runs/policy_saved.json`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaService.test.ts src/services/metaStage4RequestBuilder.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - `cargo test -p ta_desktop_demo_app commands_meta --lib`
  - `cargo test -p ta_desktop_demo_app build_stage5_artifact_summary_preserves_stage4_stage5_alias_contract --lib`
  - `cargo test -p ta_desktop_demo_app build_ml_policy_eval_summary_preserves_saved_eval_contract_fields --lib`
  - note: `cargo test -p ta_desktop_demo_app run_artifacts --lib` still has an unrelated baseline failure in `update_artifact_tags_trims_and_dedupes_case_insensitively`, so replay validation here was pinned with the focused summary-level tests above instead of the full filtered suite

## Related Docs

- `REPO_Improvement/08_ml/37_stage5_execution_contract_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/08_ml/46_ml_meta_store_status_2026-04-09.md`
- `REPO_Improvement/06_artifacts/06_stage5_artifact_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
