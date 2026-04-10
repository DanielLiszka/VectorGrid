# ML Signal Export Manifests Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for ML Signal export-manifest artifacts so dataset exports, split manifests, and related sidecars remain explicit, discoverable, and contract-stable.

## Scope Boundaries

In scope:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- export manifest and split manifest generation, persistence, and presentation

Out of scope:

- model-bundle manifests under the ML confidence model directory
- generic run artifact manifests unrelated to ML Signal dataset export
- options-related manifest surfaces

## Current User-Facing Behavior

The ML Signal export path already produces more than a CSV:

- export manifest
- optional split manifest
- feature schema sidecar
- summary display of the generated artifact paths

That is a strong provenance story, but only if the manifest family is treated as a first-class contract instead of a set of loosely related side files.

## Current Code Surface

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- ML Signal unit tests in `crates/app_core/src/ml_signal.rs`
- `MlSignalWorkspacePanel.test.ts`
- `mlSignalService.test.ts`
- saved-run detail coverage in `run_artifacts.rs`

## Strengths Already Present

- manifests are already named artifacts, not hidden implementation details
- the frontend already carries manifest paths in the ML artifact payload
- the workspace summary already exposes manifest-related information
- saved-run detail can preserve related ML attachment paths

## Weaknesses / Failure Modes

- the repo has several ML-adjacent manifest concepts, which can cause naming confusion
- manifest creation, attachment, and display are split across backend, service, and artifact layers
- it is easy to show a manifest path without stating what the manifest guarantees
- optional split-manifest behavior can be hard to distinguish from always-generated export manifest behavior

## Regressions To Avoid

- changing manifest naming or meaning without synchronized UI/service/artifact updates
- letting split-manifest semantics blur with export-manifest semantics
- losing manifest paths during saved-run reopen or artifact persistence
- reducing manifest visibility while adding more ML sidecar complexity
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the manifests should be treated as versioned contract artifacts rather than incidental logs.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because manifest payloads move through JSON-based storage and artifact attachment layers.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because manifest paths and metadata cross the frontend/backend boundary in export responses.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because the manifests describe a CSV-backed ML dataset export.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because manifest-path state is presented across the workspace and saved-run reopen flows.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Treat export-manifest artifacts as a distinct ML sidecar family with explicit roles.
2. Document which sidecars are always generated and which are conditional.
3. Keep manifest naming, attachment, and display consistent across:
   - fresh export
   - score-cache generation follow-ons
   - saved-run reopen
4. Add tests for manifest-path attachment and display.
5. Prefer clearer manifest provenance over more condensed summary text.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the export manifest and split manifest roles clearly.
- [x] Add or tighten tests for manifest generation and path attachment.
- [x] Audit frontend summary text so users know which manifest they are looking at.
- [x] Confirm saved-run reopen preserves relevant manifest paths.
- [x] Keep manifest naming/version semantics stable unless a deliberate migration exists.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `crates/app_core/src/ml_signal.rs` tests covering export-manifest and split-manifest output
- run `MlSignalWorkspacePanel.test.ts`
- run `mlSignalService.test.ts`
- run relevant saved-artifact tests if manifest attachment behavior changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current manifest file roles and naming unless there is a strong compatibility plan
- keep manifest visibility explicit in the workspace and saved-run detail
- if a cleanup obscures manifest provenance, restore the clearer path-by-path presentation
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by making manifest-family roles explicit in the ML Signal panel instead of leaving them split across stage notes and artifact JSON.
- `MlSignalWorkspacePanel.vue` now:
  - states that dataset export always writes the CSV, feature schema, and `.export_manifest.json`
  - states that `.split_manifest.json` is conditional on split-plan activation
  - exposes `datasetExportPath`, `exportManifestPath`, and `splitManifestPath` directly in the current-artifact section with open/reveal affordances
- `mlSignalService.test.ts` now asserts that `exportManifestPath`, `splitManifestPath`, and `scoreManifestPath` survive validation-stage artifact carry-forward.
- `MlSignalWorkspacePanel.test.ts` now proves the panel shows the export/split manifest basenames and the stage-owned attachment summary.
- `REPO_MAP.md` was synced so contributors can find the manifest-role contract quickly.
- Stable fixture / replay payload:
  - `dataset_export_path = C:\\data\\ml_dataset.csv`
  - `export_manifest_path = C:\\data\\ml_dataset.export_manifest.json`
  - `split_manifest_path = C:\\data\\ml_dataset.split_manifest.json`
  - `score_manifest_path = C:\\data\\score_cache.manifest.json`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MlSignalWorkspacePanel.test.ts src/services/mlSignalService.test.ts src/services/historyService.test.ts src/stores/optimizeStore.actions.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_2026-04-08.md`
- `REPO_Improvement/08_ml/12_mlsignal_feature_schema_2026-04-09.md`
- `REPO_Improvement/08_ml/13_mlsignal_split_plan_2026-04-09.md`
- `REPO_Improvement/06_artifacts/23_mlsignal_report_path_semantics_2026-04-09.md`
