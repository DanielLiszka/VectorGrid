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

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because replay is a command-driven restore path, not a purely local frontend rehydration
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because saved replay depends on stable JSON artifact shapes and compatibility fields
- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because replayable model-driven artifacts benefit from explicit compatibility signatures
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because replayed Stage 5 artifacts still need enough metadata to remain interpretable without a workbook

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

## Checklist

- [ ] Document the current saved Stage 5 family and its user-facing actions.
- [ ] Document `stage4Saved*` as compatibility state backing the saved Stage 5 UI.
- [ ] Document the current replay/load/reveal/delete/tag flows and their owners.
- [ ] Document the `stage4_json_path` / `stage5_json_path` compatibility behavior.
- [ ] Document JSON-only replay expectations for saved Stage 5 runs.
- [ ] Preserve tests for saved Stage 5 load, reveal, delete, and alias handling.
- [ ] Preserve artifact replay tests in `run_artifacts.rs`.
- [ ] Treat Stage 4 naming cleanup as compatibility-sensitive across all saved-replay surfaces.

## Verification Plan

- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/analysisStore.actions.test.ts`
- run `npm --prefix web test -- src/stores/domainStores.test.ts`
- run targeted artifact replay tests in `src-tauri/src/run_artifacts.rs`
- manually load and reveal one saved Stage 5 artifact

## Rollout / Fallback / Compatibility Notes

- preserve current compatibility aliases until store/service/artifact naming is migrated together
- keep replay behavior additive if richer report surfaces are introduced later
- prefer explicit deprecation notes over silent renames for saved-family fields

## Related Docs

- `REPO_Improvement/08_ml/37_stage5_execution_contract_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/08_ml/46_ml_meta_store_status_2026-04-09.md`
- `REPO_Improvement/06_artifacts/06_stage5_artifact_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
