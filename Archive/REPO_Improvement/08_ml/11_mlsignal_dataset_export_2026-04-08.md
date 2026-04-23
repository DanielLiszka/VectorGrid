# ML Signal Dataset Export Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for ML Signal dataset export so feature-dataset generation, split manifests, ticker holdout rules, and artifact linkage remain explicit, reproducible, and safe for the current non-options workflow.

## Scope Boundaries

In scope:

- ML Signal dataset export request/result contract
- split-plan and ticker-holdout behavior
- export manifest and artifact linkage
- frontend stage-1 UX and backend export semantics

Out of scope:

- score-cache generation details
- policy validation or policy sweep details
- model-training bundle internals

## Current User-Facing Behavior

Users can choose an output CSV path, MA source, horizon bars, optional split-plan settings, and optional ticker-holdout settings, then export an ML feature dataset from the currently loaded candles. The frontend sends `data_id`, `output_path`, `ma_source`, `horizon_bars`, `round_trip_cost_bps`, and optional `split_plan` / `ticker_holdout`. The backend requires a loaded dataset, a non-empty output path, enough candles for warmup plus horizon labeling, and then writes the dataset CSV plus sidecars such as `.feature_schema.json`, optional `.split_manifest.json`, and `.export_manifest.json`.

## Current Code Surface

- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/mlSignalService.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- ML workspace component tests
- tests in `crates/app_core/src/ml_signal.rs`
- artifact/history tests that cover saved ML run payloads

## Strengths Already Present

- dataset export is already a dedicated stage with its own request/result path
- optional split-plan and ticker-holdout controls already exist in the UI
- backend export logic already guards against not-enough-candles conditions
- artifact persistence already links dataset export output into the broader workflow

## Weaknesses / Failure Modes

- split-plan and ticker-holdout semantics are powerful enough to need clearer documentation
- output path, export manifest, split manifest, and artifact fields are not yet owned by one explicit contract doc
- stage success/failure messaging can be rich but scattered
- dataset identity and exported feature-schema expectations are easy to infer incorrectly from the UI alone
- label generation depends on cost-sensitive settings such as `round_trip_cost_bps`, which means strategy cost controls can change exported target labels even when the candle data is unchanged

## Regressions To Avoid

- breaking plain dataset export when split-plan fields are left blank
- silently changing split or ticker-holdout semantics
- hiding “not enough candles” style failures behind generic status text
- losing artifact linkage between export and later ML Signal stages
- broadening export scope into a new training pipeline instead of improving the current stage
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn cross-validation guidance](https://scikit-learn.org/1.6/modules/cross_validation.html) is relevant because export-stage split planning should preserve time-aware training/validation assumptions.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because the optional split manifest is explicitly about ordered splits.
- [RFC 4180](https://datatracker.ietf.org/doc/html/rfc4180) is relevant because the exported feature dataset is a CSV contract, not just an internal buffer.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for formalizing export request/result and manifest fields.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because export-stage semantics cross a command boundary and need visible failure discipline.

## Proposed Improvement Approach

1. Document the dataset-export request contract, including required fields, defaults, and stage-owned optional fields.
2. Make split-plan and ticker-holdout behavior explicit, including when each is absent vs active.
3. Document the returned export fields:
   - output path
   - row count
   - feature count
   - split count
   - export/split manifest paths
4. Document the actual exported CSV column groups and label semantics, including the effect of cost-sensitive labeling inputs.
5. Keep run-artifact persistence and UI stage messaging tied to the export contract.
6. Expand tests around split-plan presence/absence, ticker holdout, and failure modes.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document required export request fields and defaults.
- [x] Document optional split-plan fields and their activation rules.
- [x] Document optional ticker-holdout fields and their normalization behavior.
- [x] Document returned export/result fields and manifest paths.
- [x] Document emitted sidecar naming and version constants.
- [x] Document the actual exported CSV column groups and label semantics.
- [x] Keep UI stage text aligned with backend export outcomes.
- [x] Preserve artifact linkage from export into later ML Signal stages.
- [x] Add or maintain tests for no-split, split-plan, and ticker-holdout export paths.
- [x] Keep export-stage scope bounded to the current workflow.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm test -- src/services/mlSignalService.test.ts`
- run ML workspace component tests for stage-1 behavior
- run targeted Rust ML-signal export tests in `ta_app_core`
- manually verify one export without split plan and one export with split plan if semantics change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current plain-export behavior when optional split fields are blank
- keep split-plan and ticker-holdout behavior opt-in and explicit
- if manifest fields expand, preserve existing artifact fields until compatibility is proven
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/07_strats/08_strat_ml_signal_2026-04-08.md`
- `REPO_Improvement/08_ml/16_mlsignal_score_cache_build_2026-04-08.md`
- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`

## Execution Notes

- Closed by making the stage-1 export contract visible in the ML Signal panel instead of leaving split-plan and holdout semantics implicit.
- `MlSignalWorkspacePanel.vue` now summarizes:
  - whether the export is a plain dataset or an active split-plan export
  - the normalized ticker-holdout rule
  - the currently attached dataset-export sidecars (`feature_schema`, `export_manifest`, optional `split_manifest`)
- `mlSignalService.test.ts` now covers the no-split path and proves that ticker-holdout residue is normalized before the export request is sent.
- `MlSignalWorkspacePanel.test.ts` now verifies the export-contract summary, including split-plan text, normalized holdout text, and attached sidecar basenames.
- Artifact linkage remains unchanged: exported dataset artifacts still flow through `save_run_artifact`, current-run hydration, and later ML Signal stages.
- `REPO_MAP.md` was synced so contributors can find the dataset-export contract behavior quickly.
- Stable fixture / replay payload:
  - request-side fields:
    - `output_path = C:\\data\\ml_dataset.csv`
    - split plan blank for plain export, or `252/63/63` with `gap=10`, `step=63` for manifest export
    - `ticker_holdout = { modulus: 5, residue: 2 }` after normalization
  - artifact-side fields:
    - `dataset_export_path = C:\\data\\ml_dataset.csv`
    - `feature_schema_path = C:\\data\\ml_dataset.feature_schema.json`
    - `export_manifest_path = C:\\data\\ml_dataset.export_manifest.json`
    - optional `split_manifest_path = C:\\data\\ml_dataset.split_manifest.json`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MlSignalWorkspacePanel.test.ts src/services/mlSignalService.test.ts src/services/historyService.test.ts src/stores/optimizeStore.actions.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
