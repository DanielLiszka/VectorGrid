# Stage 5 Execution Contract Plan

Last updated: 2026-04-09

## Purpose

Define the current Stage 5 execution contract so command routing, saved-artifact behavior, replay support, status updates, and Stage 4 compatibility aliases remain explicit and do not drift into a fictional separate Stage 4/Stage 5 implementation split.

## Scope Boundaries

In scope:

- `run_meta_stage5_ml_confidence`
- frontend `runStage5()` and current `runStage4()` aliasing
- Stage 5 saved-artifact creation
- saved Stage 5 list/load/reveal/delete behavior
- Stage 4 compatibility response fields and naming drift

Out of scope:

- detailed selection math already covered by the Stage 4 selection-logic doc
- policy evaluation
- Stage 3 report/workbook generation
- options workflows

## Current User-Facing Behavior

The user-facing workflow is explicitly Stage 5. Users can run Stage 5, inspect the preview, and manage saved Stage 5 results from the Meta workspace. Current behavior means:

- the main action is `Run Stage 5`
- the frontend `runStage4()` helper is only an alias to `runStage5()`
- running Stage 5 produces a saved JSON artifact, not a workbook-style report
- saved Stage 5 entries can be refreshed, loaded, revealed, tagged, and deleted
- the preview/status surfaces present Stage 5 language even though many underlying fields still use Stage 4 compatibility names

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - Stage 5 controls, saved Stage 5 lane, and preview/status display
- `web/src/stores/analysisStore.ts`
  - persisted Stage-5-related store fields that still carry `stage4` compatibility names
- `web/src/services/metaService.ts`
  - `runStage5()`
  - `runStage4()` alias
  - Stage 5 saved-entry management
  - replay/reveal/delete helpers
- `src-tauri/src/commands_meta.rs`
  - `run_meta_stage5_ml_confidence`
  - Stage 4 compatibility aliases
  - Stage 5 artifact payload writing
- `src-tauri/src/run_artifacts.rs`
  - Stage 5 artifact listing, summary extraction, replay hydration, and pruning
- `src-tauri/src/commands_artifacts.rs`
  - Tauri commands used for Stage 5 replay/listing operations

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/stores/domainStores.test.ts`
- tests embedded in `src-tauri/src/commands_meta.rs`
- tests embedded in `src-tauri/src/run_artifacts.rs`

## Strengths Already Present

- the user-facing contract is consistently described as Stage 5 even though compatibility aliases still exist underneath
- Stage 5 artifact generation is already durable and replayable
- the saved Stage 5 family is already integrated into the artifact library and replay commands
- frontend helpers already resolve either `stage5_json_path` or `stage4_json_path`, which preserves compatibility
- Stage 5 completion can trigger ML status refresh when the run materially changed model state

## Weaknesses / Failure Modes

- the execution contract is affected by deep naming drift:
  - backend structs still use Stage 4 names
  - store fields still use Stage 4 names
  - response payload carries both Stage 4 and Stage 5 paths
- Stage 5 has no workbook/report output comparable to Stage 3, which is a real product asymmetry that can be overlooked
- if docs imply that Stage 4 and Stage 5 are fully separate execution paths today, they will be wrong
- saved entry management is Stage-5-first in the UX but still keyed off Stage-4-named state in the implementation

## Regressions To Avoid

- breaking the current `stage5_json_path ?? stage4_json_path` compatibility behavior
- documenting Stage 5 as workbook/report-backed when it is currently JSON-artifact-only
- changing saved Stage 5 artifact shapes without updating replay, summary extraction, and UI management together
- hiding the fact that `runStage4()` currently delegates to `runStage5()`
- weakening saved-entry management flows during naming cleanup
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because Stage 5 execution is a structured command-boundary workflow with stable request/result expectations
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the saved Stage 5 artifact and replay payloads behave like versioned JSON contracts
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because Stage 5 outputs should remain interpretable even without a workbook-style report surface
- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because saved model-driven artifacts benefit from explicit, stable output signatures and compatibility expectations
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document Stage 5 execution as the authoritative user-facing contract and Stage 4 as the compatibility alias layer beneath it.
2. Record current response-field aliasing explicitly, including the shared-file path returned through both Stage 4 and Stage 5 fields.
3. Keep saved-artifact creation, replay, reveal, and delete behavior grouped in one execution doc so the full lifecycle is visible.
4. Call out the current report asymmetry:
   - Stage 3 has workbook/report output
   - Stage 5 currently has JSON artifacts only
5. Preserve model-status-refresh coupling as part of the execution contract because Stage 5 can update the ML status surface indirectly.
6. Tie the saved-entry UX to its current Stage-4-named store/state ownership so future renames do not break behavior accidentally.
7. Treat any future true split between Stage 4 and Stage 5 as a new contract, not as a retroactive reinterpretation of today’s code.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current public Stage 5 command and its frontend invocation path.
- [ ] Document that `runStage4()` currently delegates to `runStage5()`.
- [ ] Document the Stage 5 artifact payload kind and current response fields.
- [ ] Document the current `stage4_json_path` / `stage5_json_path` compatibility alias behavior.
- [ ] Document saved Stage 5 list/load/reveal/delete behavior and its current store ownership.
- [ ] Document the lack of workbook/report output for Stage 5.
- [ ] Document model-status refresh coupling after Stage 5 completion.
- [ ] Preserve tests for Stage 5 run, saved-entry update, replay, and artifact summary extraction.
- [ ] Keep Stage 5 docs aligned with artifact-library behavior in `run_artifacts.rs`.
- [ ] Treat Stage 4/Stage 5 renaming work as compatibility-sensitive.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/domainStores.test.ts`
- run targeted Stage 5 tests in `src-tauri/src/commands_meta.rs`
- run targeted Stage 5 artifact tests in `src-tauri/src/run_artifacts.rs`
- manually run Stage 5, then load and reveal the saved Stage 5 artifact
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current Stage 5 JSON artifact compatibility while the repo still carries Stage 4 compatibility aliases
- if Stage 5 later gains report/workbook outputs, document that as an additive extension rather than implying they exist now
- keep replay/list/delete flows working through any naming cleanup by preserving path alias handling
- prefer additive metadata over semantic changes to existing saved Stage 5 artifacts
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/36_stage4_selection_logic_2026-04-09.md`
- `REPO_Improvement/08_ml/38_ml_policy_eval_contract_2026-04-09.md`
- `REPO_Improvement/06_artifacts/06_stage5_artifact_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
