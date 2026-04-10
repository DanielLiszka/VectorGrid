# ML Signal Model Bundle Inventory Plan

Last updated: 2026-04-09

## Purpose

Define the current ML Signal model-bundle inventory and status contract so manifest health, app-local storage layout, optional-bundle UX, and artifact/history retention remain explicit and auditable.

## Scope Boundaries

In scope:

- model-bundle manifest schema and health classification
- app-local bundle discovery and status lookups
- frontend bundle picker and status-line behavior
- artifact/history retention of bundle identity
- compatibility expectations between model bundles and feature-schema versions

Out of scope:

- model training pipelines
- score-cache CSV ingestion or score-cache file generation rules
- Stage 3 / Stage 4 meta-analysis
- non-ML-signal model registries

## Current User-Facing Behavior

The ML Signal workspace lets users refresh bundle options, pick a bundle from the app-local inventory, and refresh status for a specific bundle. The current UX intentionally treats bundles as optional for score-cache-backed validation, but when a bundle is selected the app surfaces its status and manifest path. Current behavior includes:

- bundle options are listed from the app-local models directory
- each option is summarized as `bundleId • family • state`
- per-bundle status is fetched on demand and rendered as a single status line
- bundle identity is carried into ML artifact payloads so saved runs can be inspected later
- missing manifests are reported as `missing` state rather than crashing the UI

## Current Code Surface

- `web/src/components/MlSignalWorkspacePanel.vue`
  - bundle picker, refresh actions, and current status line
- `web/src/services/mlSignalService.ts`
  - option-list normalization, status refresh, and artifact payload enrichment
- `web/src/stores/optimizeStore.ts`
  - persisted `mlSignalModelBundleId` and `mlSignalModelStatus`
- `src-tauri/src/commands_ml_signal.rs`
  - app-local models directory resolution, bundle listing, and status lookup
- `crates/app_core/src/ml_signal.rs`
  - `MlSignalModelBundleManifest`
  - `MlSignalModelBundleStatus`
  - `list_ml_signal_model_bundle_summaries()`
  - `classify_ml_signal_manifest()`
- `src-tauri/src/run_artifacts.rs`
  - retention of bundle identity in saved run details
- `web/src/services/historyService.ts`
  - replay/hydration of ML bundle-related artifact context

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `web/src/services/historyService.test.ts`
- ML-signal tests in `crates/app_core/src/ml_signal.rs`
- artifact/detail tests that touch ML payload hydration

## Strengths Already Present

- bundle health is already computed in one canonical backend classifier
- bundle discovery is constrained to an app-local models directory rather than scattered paths
- frontend does not crash when bundles are absent or optional
- manifest path, bundle id, and model family/format already exist as structured inventory fields
- saved artifacts already retain bundle identity alongside score-cache and schema metadata

## Weaknesses / Failure Modes

- user-facing bundle status is currently compressed into a single string, which can hide which part of the manifest is actually unhealthy
- manifest health semantics live in Rust, but the UI largely reduces them to one status label
- bundle inventory is related to, but distinct from, score-cache inventory; those surfaces are easy to conflate
- report/workbook surfaces do not currently serve as the main audit point for bundle health, so artifact/history inspection matters more than it might appear
- missing or stale bundle manifests can degrade to warning/missing states without a richer remediation surface

## Regressions To Avoid

- breaking the current app-local models directory layout
- weakening `feature_schema_version` compatibility checks
- treating bundle absence as a hard error when score-cache-backed validation is intentionally allowed without a selected bundle
- dropping bundle identity from saved artifacts or history detail views
- flattening manifest issues into an unstructured string without preserving the underlying issue list
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because stable model packaging benefits from explicit schema/signature thinking rather than opaque file drops
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because bundle inventories are more useful when they expose health, caveats, and compatibility metadata rather than only filenames
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the manifest is a versioned JSON contract
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because bundle discovery and status inspection are command-mediated desktop flows
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the bundle inventory as a first-class app-local registry with a concrete directory convention.
2. Record the manifest schema explicitly, including optional versus required child files.
3. Separate bundle list summaries from per-bundle status details so future UI work can stay structured.
4. Preserve the current optional-bundle posture while documenting exactly when the bundle still adds value.
5. Make feature-schema compatibility a visible contract concern instead of a hidden classifier detail.
6. Ensure artifact/history docs keep bundle id, manifest path, and score-cache/schema context aligned.
7. Expand tests if the bundle manifest or health states evolve.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the app-local models directory path and bundle folder convention.
- [x] Document the `manifest.json` schema and required fields.
- [x] Document optional `long_model_path`, `short_model_path`, and `calibration_path` semantics.
- [x] Document current health states and issue-generation rules.
- [x] Document current frontend summary labels and status-line behavior.
- [x] Document that bundle selection is optional for score-cache-backed validation.
- [x] Document feature-schema-version compatibility as a first-class invariant.
- [x] Document which bundle fields are persisted into artifacts and replayed later.
- [x] Add or maintain tests for missing manifests, warning states, and UI normalization.
- [x] Preserve backward compatibility for currently saved bundle-aware artifacts.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/mlSignalService.test.ts`
- run `npm --prefix web run test -- src/components/MlSignalWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run targeted ML-signal tests in `crates/app_core/src/ml_signal.rs`
- manually verify one healthy bundle, one warning bundle, and one missing bundle path through the UI/service boundary
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve the current bundle manifest layout and issue semantics unless a versioned manifest migration is introduced
- prefer richer issue display over silently relaxing manifest checks
- keep the “bundle optional” user message accurate while the backend still supports score-cache-backed validation without one
- if bundle metadata expands, keep current fields stable until older artifacts and UI surfaces are updated
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by making the bundle-inventory contract explicit in the Stage 2 panel rather than leaving it compressed into the status line alone.
- `MlSignalWorkspacePanel.vue` now explains that:
  - bundles come from the desktop app-local inventory
  - selection is optional for score-cache-backed validation
  - the status line should name the selected bundle state and manifest path when a bundle is chosen
- The underlying inventory/service contract was left unchanged; `mlSignalService.test.ts` remains the authority for list normalization and optional-bundle status refresh behavior.
- `REPO_MAP.md` was synced so contributors can locate the bundle inventory contract quickly.
- Stable fixture / replay payload:
  - `bundle_id = bundle-a`
  - status line `Model bundle: bundle-a • healthy • C:\\models\\ml_signal\\bundle-a\\manifest.json`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MlSignalWorkspacePanel.test.ts src/services/mlSignalService.test.ts src/services/historyService.test.ts src/stores/optimizeStore.actions.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_2026-04-08.md`
- `REPO_Improvement/08_ml/16_mlsignal_score_cache_build_2026-04-08.md`
- `REPO_Improvement/08_ml/17_mlsignal_score_cache_validation_2026-04-09.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
