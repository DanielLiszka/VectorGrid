# CatBoost ML Signal Improvement Plan

Last updated: 2026-04-24

## Purpose

This document turns the CatBoost / ML Signal audit into an implementation plan.
It focuses on improving the current `MlSignal` CatBoost model training and
inference workflow, while keeping Stage 5 ML confidence separate because Stage 5
currently uses in-repo logistic / lightweight GBDT-stump models, not CatBoost.

The primary goal is to make CatBoost model bundles reproducible, verifiable,
and safer to use in desktop validation and policy sweep workflows.

## Implementation Status

Implemented in this pass:

- Reproducible training entrypoint:
  `scripts/train_ml_signal_catboost.py`
- Default training config:
  `configs/ml_signal_catboost_default.json`
- Manifest v2 fields for CatBoost, model hashes, schema hashes, score scope,
  calibration, training data, and training provenance.
- Strict native scoring checks for feature-schema hash, model feature count,
  calibration contract, score-scope policy, and path traversal.
- Native score-scope selection and walk-forward / calibration-slice filtering
  from split manifests.
- Native library discovery policies for production, developer debug, and legacy
  replay, with SHA-256 provenance for loaded libraries.
- Score-cache and saved-run provenance for bundle/model/schema/export/native
  hashes, score-scope policy, and raw/effective score observations.
- Frontend controls and artifact summaries for native score scope and new
  CatBoost provenance fields.
- A shared Rust feature-row emitter used by both dataset export and native
  inference, with parity coverage between CSV rows and in-memory CatBoost
  rows.
- Native CatBoost prediction batching with request/UI/smoke-harness override,
  score-cache provenance, and cancellation checks between batches.
- Generated strict-v2 bundle contract fixture tests that do not require a
  local CatBoost Python/package install.
- Focused quality gate script:
  `scripts/run_ml_signal_quality_gate.ps1`
- Release-manifest failure when the packaged Windows CatBoost native library is
  missing, unless explicitly overridden for diagnostics.

Remaining or intentionally deferred:

- CI/CD hook integration is intentionally excluded from this implementation
  pass.
- Committed real `.cbm` fixture models are still absent because CatBoost is not
  available in this environment; tests cover contract logic without requiring
  CatBoost to be installed.
- Native cancellation can stop between feature-generation/model-loading/batched
  prediction boundaries, but an in-flight CatBoost C API call cannot be
  interrupted by this Rust layer.
- Benchmark-based batch-size tuning was not run because the machine had
  background CPU/GPU-intensive work.

## Current State Summary

The repo currently supports a bundle-based CatBoost inference workflow:

- The desktop app exports an `MlSignal` feature dataset from loaded candles.
- CatBoost training happens outside the repo and outside the app.
- A model bundle is placed under `app_local_data_dir/models/ml_signal/<bundle_id>/`.
- The bundle manifest points to `model_long.cbm`, `model_short.cbm`, optional
  calibration metadata, feature-schema sidecars, split-plan sidecars, and
  score-contract metadata.
- Native inference loads CatBoost's `catboostmodel` dynamic library through
  `libloading`, applies the long and short `.cbm` models, writes a score cache,
  and validation / policy sweep consume that score cache.

The current shipped Windows desktop build also includes an app-level CPU
CatBoost fallback library at:

- `src-tauri/resources/native/catboostmodel.dll`

The live code owners are:

- Core ML Signal implementation: `crates/app_core/src/ml_signal.rs`
- Tauri command boundary: `src-tauri/src/commands_ml_signal.rs`
- Saved-run artifact summaries: `src-tauri/src/run_artifacts.rs`
- Frontend ML Signal service: `web/src/services/mlSignalService.ts`
- Frontend saved-run hydration: `web/src/services/historyService.ts`
- Frontend artifact normalization: `web/src/stores/workflowArtifactsStore.ts`
- Frontend ML Signal panel: `web/src/components/MlSignalWorkspacePanel.vue`
- Native build helper: `scripts/build_catboost_native_windows.ps1`
- Native smoke helper: `scripts/run_ml_signal_native_smoke.ps1`
- Native smoke binary: `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`
- Dataset export binary: `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`

## Goals

- Make CatBoost model training reproducible from exported datasets.
- Make bundle contents self-validating before inference.
- Prevent silent schema, calibration, or score-scope drift.
- Reduce duplicated feature-generation logic.
- Improve cancellation and long-run behavior during native scoring.
- Harden native library discovery for production.
- Preserve replay compatibility for old saved runs and legacy bundles.
- Keep CPU-only desktop builds working without CUDA tooling.

## Non-Goals

- Do not replace Stage 5 ML confidence with CatBoost in this plan.
- Do not make `MlSignal` part of normal optimizer drilldown/local-stability views.
- Do not require CUDA for normal desktop CatBoost inference.
- Do not remove score-CSV ingestion; it remains useful for externally generated
  scores and comparison workflows.
- Do not break existing `native_cpu` bundle inference.

## Design Principles

- Prefer explicit artifacts over inferred state.
- Fail hard on correctness mismatches and warn only for known exploratory modes.
- Keep bundle metadata relative where possible so bundles are portable.
- Keep runtime scoring deterministic and auditable.
- Treat feature schema, calibration state, score scope, and native library
  identity as operational contracts.
- Keep the UI honest about whether scores are walk-forward test, full dataset,
  calibration slice, or custom partial.

## Implementation Checklist

### Phase 0 - Baseline and Fixtures

- [x] Add at least one tiny synthetic `MlSignal` fixture dataset for deterministic
      training and inference tests.
- [x] Add a fixture bundle layout under a test-only fixture directory.
- [x] Document whether fixture `.cbm` files are real CatBoost models or mocked
      only for manifest validation.
- [ ] Capture current native scoring behavior with the existing smoke binary.
- [ ] Record the current packaged `catboostmodel` library size and build source
      tag in release documentation or a generated manifest.
- [x] Confirm CPU-only build/test lanes do not require CUDA.

Acceptance criteria:

- Developers can run a deterministic test fixture without private market data.
- The fixture is small enough to keep in repo or generated quickly by a script.
- The current behavior is captured before hardening begins.

Suggested verification:

- `cargo test -p ta_app_core ml_signal --lib`
- `cargo test -p ta_desktop_demo_app --lib commands_ml_signal`
- `npm --prefix web run test -- src/services/mlSignalService.test.ts src/components/MlSignalWorkspacePanel.test.ts`

### Phase 1 - Reproducible CatBoost Training Pipeline

- [x] Add a training script entrypoint, preferably under `scripts/`, for example:
      `scripts/train_ml_signal_catboost.py`.
- [ ] Add a lockfile or documented Python environment for the training script.
- [x] Make the script consume:
      - exported dataset CSV
      - `*.feature_schema.json`
      - `*.split_manifest.json`
      - training config JSON
      - output bundle directory
- [x] Train separate long and short binary CatBoost models.
- [x] Use split windows from the export sidecar instead of random row splits.
- [x] Train only on train windows, tune / early-stop on calibration windows, and
      reserve walk-forward test windows for final evidence.
- [x] Save `.cbm` models using CatBoost binary format.
- [x] Emit bundle-relative paths in `manifest.json`.
- [x] Emit `training_config` with all CatBoost parameters, seed, feature list,
      label policy, split policy, class-balance policy, and package versions.
- [x] Emit `training_windows_summary` with train/calibration/test window counts,
      row counts, timestamp ranges, and ticker-holdout settings.
- [x] Emit model-quality metrics for long and short models separately:
      - train metrics
      - calibration metrics
      - walk-forward test metrics
      - AUC
      - Brier
      - precision / recall at configured thresholds
      - positive / negative counts
- [x] Emit feature importance artifacts.
- [x] Emit model file hashes for `model_long.cbm` and `model_short.cbm`.
- [x] Emit the exact source feature-schema hash and export-manifest hash.
- [x] Emit score-contract metadata that states expected score scope and
      probability semantics.
- [x] Add a README section or script help text with the end-to-end workflow:
      export dataset, train bundle, install bundle, run native smoke, build
      score cache, validate, sweep.

Acceptance criteria:

- A developer can train a new CatBoost bundle from app-exported data without
  reading source code.
- The bundle contains enough metadata to explain how it was trained.
- The desktop app can list and classify the resulting bundle.
- Re-running training with the same seed and inputs produces comparable metrics
  and identical or intentionally explained artifact metadata.

Suggested CatBoost defaults to start with:

- `loss_function=Logloss`
- `eval_metric=AUC` or `Logloss`, decided explicitly in config
- `random_seed` fixed
- `task_type=CPU` for the default path
- `od_type=Iter`
- `od_wait` configured through training config
- `use_best_model=true` when an eval set is present
- conservative depth and learning rate until enough evidence exists

### Phase 2 - Bundle Manifest Contract v2

- [x] Introduce a manifest contract version for stricter bundles. Keep existing
      bundles readable as v1 / legacy.
- [x] Add explicit manifest fields:
      - `bundle_schema_version`
      - `catboost_version`
      - `catboost_model_interface_version` or native-library build tag
      - `model_long_sha256`
      - `model_short_sha256`
      - `feature_schema_sha256`
      - `feature_schema_hash`
      - `export_manifest_sha256`
      - `source_export_manifest_hash`
      - `score_contract_version`
      - `score_scope_policy`
      - `probability_semantics`
      - `calibration_required`
      - `calibration_method_long`
      - `calibration_method_short`
      - `training_data_id`
      - `training_timeframe`
      - `training_symbol_scope`
      - `created_at_utc`
      - `trained_by`
- [x] Keep paths relative to the bundle directory unless an absolute path is
      explicitly needed for local experiments.
- [x] Reject path traversal in manifest child paths.
- [x] Add manifest validation that distinguishes:
      - fatal correctness issues
      - warning-only recommended metadata gaps
      - replay-compatible legacy gaps
- [x] Add frontend status text that surfaces fatal vs warning state clearly.
- [x] Add tests for v1 compatibility and v2 stricter validation.

Acceptance criteria:

- Healthy v2 bundles have no recommended-metadata warnings.
- Legacy bundles still list and can be used when they satisfy the old minimum
  contract.
- Malformed paths or missing model files cannot silently proceed to inference.

Target files:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`

### Phase 3 - Hard Schema Enforcement Before Inference

- [x] Compute the actual `feature_schema.json` hash during native scoring.
- [x] Compare the actual hash to:
      - `manifest.source_feature_schema_hash`
      - request `feature_schema_hash`, when present
      - score-contract feature schema metadata, when present
- [x] Hard-error on mismatch for v2 bundles.
- [x] Keep legacy warning behavior only for v1 bundles with missing metadata.
- [x] Verify `schema.feature_names` exactly match runtime generated feature names.
- [x] Verify CatBoost model float feature count exactly matches schema feature
      count for both long and short models before scoring.
- [x] Persist the verified schema hash and mismatch status in score-cache
      manifest output.
- [x] Add tests for:
      - matching schema hash
      - missing schema hash in legacy bundle
      - mismatched source schema hash
      - long model feature-count mismatch (covered by runtime guard; real
        CatBoost fixture still pending)
      - short model feature-count mismatch (covered by runtime guard; real
        CatBoost fixture still pending)

Acceptance criteria:

- A stale or mismatched feature schema cannot produce a score cache without a
  clear failure.
- The score cache records the schema hash that was actually used.

Target code areas:

- `build_ml_signal_native_score_cache(...)`
- `read_feature_schema_from_path(...)`
- `write_ml_signal_score_cache(...)`
- bundle status classification helpers

### Phase 4 - Calibration Contract Hardening

- [x] Make calibration behavior explicit in the manifest:
      - absent calibration is allowed only when `calibration_required=false`
      - required calibration must include method and parameters per side
      - long and short calibration states are tracked independently
- [x] Treat declared-but-missing calibration files as fatal for v2 bundles.
- [x] Treat malformed calibration files as fatal for v2 bundles.
- [x] Treat unsupported calibration methods as fatal for v2 bundles.
- [x] If raw sigmoid fallback is used for legacy bundles, persist:
      - `calibration_state=uncalibrated_sigmoid_fallback`
      - `native_fallback_reason`
      - per-side fallback detail
- [x] Add calibration evidence to score cache and saved run artifacts.
- [x] Add UI copy that surfaces calibrated vs uncalibrated probability state.
- [x] Add tests for:
      - sigmoid calibration
      - isotonic calibration
      - missing required calibration
      - malformed calibration
      - legacy fallback with persisted warning

Acceptance criteria:

- Probability-threshold validation never mistakes uncalibrated raw-margin sigmoid
  scores for calibrated probabilities.
- Users can see calibration state in the current artifact and score-cache summary.

Target code areas:

- `apply_calibration_probability(...)`
- `manifest_calibration_state(...)`
- `build_ml_signal_native_score_cache(...)`
- `MlSignalScoreCacheSummary`
- `MlSignalScoreCacheManifest`
- `MlSignalWorkspacePanel.vue`

### Phase 5 - Score Scope and Walk-Forward Enforcement

- [x] Add a request-level score-scope override for native bundle inference.
- [x] Add manifest `score_scope_policy` with allowed values:
      - `walkforward_test_required`
      - `full_dataset_allowed`
      - `custom_partial_allowed`
      - `calibration_slice_allowed`
- [x] Make v2 training bundles default to `walkforward_test_required`.
- [x] If native scoring produces a full-dataset or custom-partial cache, require
      an explicit request flag or developer-mode setting.
- [x] Use split manifest windows to restrict native score-cache rows to
      walk-forward test intervals when requested.
- [x] Persist raw and effective score-observation counts.
- [x] Keep existing warning flags for overlap-adjusted walk-forward evidence.
- [x] Add frontend controls or status text that makes score scope explicit before
      validation.
- [x] Add tests for:
      - native walk-forward score-cache generation
      - native full-dataset generation with explicit opt-in
      - rejection when full-dataset generation is disallowed
      - missing split manifest under walk-forward-required policy

Acceptance criteria:

- Default native CatBoost validation is based on walk-forward test rows when
  bundle metadata supports it.
- Calibration-slice or full-dataset scores are still possible only through an
  explicit, visible choice.

Target code areas:

- `MlSignalScoreScope`
- `build_ml_signal_native_score_cache(...)`
- `infer_score_scope(...)`
- `load_ml_signal_overlap_metadata(...)`
- validation warning flags
- frontend score-cache request builder in `mlSignalService.ts`

### Phase 6 - Single Feature-Frame Emitter

- [x] Refactor feature generation so dataset export and native inference share
      one row emitter.
- [x] Define an internal row representation that can write:
      - CSV feature rows for training export
      - transposed `Vec<Vec<f32>>` rows for CatBoost C API inference
      - optional debug JSON / parity fixtures
- [x] Preserve current feature ordering exactly.
- [x] Preserve current timestamp and warmup semantics exactly.
- [x] Avoid string round-trips for native inference.
- [x] Add a test that exports a small dataset and compares feature values with
      the in-memory native inference rows within float tolerance.
- [x] Add a schema-generation test that asserts `feature_names_for_schema(...)`
      matches emitted columns.
- [x] Add a regression test for first valid bar and last label bar.

Acceptance criteria:

- Adding or modifying an `MlSignal` feature requires changing one emitter, not
  two paths.
- Feature parity between training export and native inference is tested.

Target code areas:

- `feature_frame(...)`
- `feature_names_for_schema(...)`
- `build_ml_signal_inference_feature_rows(...)`
- `export_ml_signal_dataset_with_candles(...)`

### Phase 7 - Native Inference Cancellation and Batching

- [x] Honor cancellation during:
      - feature frame construction
      - feature row emission
      - model loading boundary
      - long model prediction
      - short model prediction
      - score-cache write
- [x] Add batched CatBoost scoring for very large datasets, with a configurable
      default batch size.
- [x] Add memory accounting or simple row-count diagnostics to native score-cache
      output.
- [ ] Add a cancellation test around a fake or injectable scoring backend.
- [x] Persist the batch-size setting in score-cache/saved-run provenance so
      tuning evidence can be compared later.

Acceptance criteria:

- User cancellation can stop long native score-cache generation without waiting
  for all scoring work to finish.
- Large score-cache generation has observable progress or at least bounded
  memory diagnostics.

Target code areas:

- `build_ml_signal_native_score_cache(...)`
- `CatBoostModelHandle::calc_raw_transposed(...)`
- `generate_ml_signal_score_cache(...)`
- task cancellation state in `AppState`

### Phase 8 - Native Library Discovery Hardening

- [x] Split library discovery policy into:
      - production desktop policy
      - developer / smoke-test policy
      - legacy replay policy
- [x] Production policy should prefer:
      1. bundle-local declared library path, if allowed
      2. explicit UI/developer override when supplied
      3. bundle-local default paths and packaged app resource fallback from the
         Tauri command boundary
- [x] Avoid `cwd` and bare library-name search in production scoring.
- [x] Keep env-var and broad search support in smoke/debug tools.
- [x] Record native library identity in score-cache metadata:
      - resolved path
      - SHA-256
      - CatBoost tag or build metadata if available
      - CPU vs CUDA capability if available
- [x] Add tests for candidate ordering under each policy.
- [x] Add release manifest checks that fail when packaged CatBoost fallback is
      missing for Windows release variants that expect it.

Acceptance criteria:

- Production desktop scoring cannot accidentally load an unrelated
  `catboostmodel.dll` from cwd or PATH-like lookup behavior.
- Score caches identify the native library that produced them.

Target code areas:

- `build_catboost_library_candidates(...)`
- `CatBoostApi::load(...)`
- `src-tauri/src/commands_ml_signal.rs`
- `scripts/generate_release_manifest.ps1`
- `scripts/run_ml_signal_native_smoke.ps1`

### Phase 9 - Score Cache and Artifact Provenance Upgrades

- [x] Add score-cache manifest fields for:
      - bundle manifest path
      - bundle manifest hash
      - model file hashes
      - native library hash
      - feature schema hash
      - calibration state per side
      - score scope policy
      - training config hash
      - score generation timestamp
- [x] Add saved-run artifact fields mirroring the same provenance.
- [x] Update reopen/apply/rerun behavior to preserve new fields.
- [ ] Add validation-time checks that loaded score cache provenance matches the
      active dataset/timeframe context when validation context is present.
- [x] Add frontend artifact summaries for:
      - bundle id
      - schema hash
      - score scope
      - calibration state
      - native backend used
      - native library hash
      - model hash short IDs
- [ ] Add tests for saved-run hydration with new provenance fields.

Acceptance criteria:

- A saved ML Signal run can be audited without manually opening every sidecar.
- Reopening a run does not lose CatBoost provenance fields.

Target code areas:

- `MlSignalScoreCacheSummary`
- `MlSignalScoreCacheManifest`
- `buildMlSignalArtifactPayload(...)`
- workflow artifacts store normalization
- `MlSignalWorkspacePanel.vue`

### Phase 10 - Verification, CI Hooks, and Docs

- [x] Add a deterministic training smoke that can run without private data.
- [ ] Add a native inference smoke that can run from a generated fixture bundle.
- [x] Add a score-cache validation smoke.
- [x] Add frontend tests for strict bundle status and provenance summaries.
- [x] Add docs for:
      - training workflow
      - bundle manifest contract
      - calibration file contract
      - score-scope policy
      - native library discovery policy
      - troubleshooting native CatBoost loading
- [x] Add release-check coverage for packaged native library presence.
- [x] Decide whether this deserves a dedicated script, for example:
      `scripts/run_ml_signal_quality_gate.ps1`.

Suggested gate contents:

- `cargo test -p ta_app_core ml_signal --lib`
- `cargo test -p ta_desktop_demo_app --lib commands_ml_signal`
- targeted Tauri run-artifact tests for ML Signal saved-run provenance and
  hydration
- `npm --prefix web run test -- src/services/mlSignalService.test.ts src/components/MlSignalWorkspacePanel.test.ts`
- Native smoke with fixture bundle, when fixture `.cbm` files are available

Acceptance criteria:

- Developers have a single high-signal command for ML Signal / CatBoost changes.
- Release checks cover packaged native runtime prerequisites.

## Detailed Implementation Notes

### Training Script Contract

The training script should avoid being a loose research notebook. It should be a
repeatable artifact builder with a stable CLI.

Proposed command shape:

```powershell
python scripts/train_ml_signal_catboost.py `
  --dataset-csv C:\data\ml_signal_dataset.csv `
  --feature-schema C:\data\ml_signal_dataset.feature_schema.json `
  --split-manifest C:\data\ml_signal_dataset.split_manifest.json `
  --training-config configs\ml_signal_catboost_default.json `
  --bundle-dir C:\models\ml_signal\bundle_20260424_a
```

Proposed output:

```text
bundle_20260424_a/
  manifest.json
  model_long.cbm
  model_short.cbm
  calibration.json
  feature_schema.json
  export_manifest.json
  split_manifest.json
  metrics.json
  feature_importance_long.csv
  feature_importance_short.csv
  training_config.json
```

The manifest should remain the desktop-facing source of truth. The other files
may contain richer detail, but the manifest must have enough summary metadata
for the app to reject unsafe bundles without loading all sidecars.

### Label and Split Policy

The existing export writes these target-related columns:

- `r_long`
- `r_short`
- `y_long`
- `y_short`
- `keep_long`
- `keep_short`

The training script should define and persist:

- whether `keep_long` / `keep_short` filter rows
- whether labels are binary from `y_long` / `y_short`
- whether sample weights are used
- class-balance strategy
- deadband and transaction-cost assumptions
- which split windows are train, calibration, and test

The script should not randomly split rows by default. Time and walk-forward
boundaries are part of the model contract.

### Calibration File Contract

Suggested `calibration.json` shape:

```json
{
  "version": 1,
  "long": {
    "method": "sigmoid",
    "coef": 1.0,
    "intercept": 0.0,
    "source": "calibration_windows",
    "brier_before": 0.21,
    "brier_after": 0.19
  },
  "short": {
    "method": "isotonic",
    "x_thresholds": [-2.0, -1.0, 0.0, 1.0, 2.0],
    "y_thresholds": [0.02, 0.08, 0.22, 0.54, 0.83],
    "source": "calibration_windows",
    "brier_before": 0.24,
    "brier_after": 0.20
  }
}
```

For v2 bundles:

- if `calibration_required=true`, the file must exist and parse
- each side must have a supported method
- fallback to raw sigmoid must be fatal unless explicitly allowed by contract

For v1 bundles:

- missing calibration can continue as a compatibility fallback
- the fallback must be persisted in score-cache metadata

### Score Scope Policy

The app should stop treating score scope as a passive warning only. For
production-quality model evaluation:

- `walkforward_test` is the preferred validation evidence
- `calibration_slice` is useful for model tuning evidence, not final validation
- `full_dataset` can be useful for exploration but risks leakage
- `custom_partial` is acceptable only when the user knows what rows were scored

Suggested policy defaults:

- v2 bundle default: `walkforward_test_required`
- legacy bundle default: preserve current warning behavior
- CSV score import: infer from `score_scope` column or filename, then warn
- native scoring: use split manifest to emit walk-forward test rows unless
  explicitly overridden

### Native Library Policy

The current broad lookup is useful while wiring the feature, but production
should avoid accidental local DLL resolution. Proposed policy:

```text
Production desktop:
  bundle declared path -> explicit UI override -> bundle defaults / packaged resource

Smoke/debug:
  explicit override -> env vars -> bundle dirs -> process dirs -> cwd -> bare name

Legacy replay:
  use recorded path if still present, otherwise bundle defaults / process-relative
  compatibility lookup, otherwise clear failure with remediation text
```

The exact ordering can be adjusted, but it should be explicit and tested.

### Stage 5 ML Confidence Boundary

Stage 5 ML confidence should remain separately documented and tested. It is not
CatBoost today. It currently trains from saved meta artifacts and produces a
small JSON model with health/staleness metadata.

Recommended Stage 5 follow-ups outside the CatBoost path:

- [ ] Keep model health gates strict when `require_healthy_model=true`.
- [ ] Preserve source-meta exclusion during auto-train to avoid train/score
      leakage.
- [ ] Add model-card style summaries for Stage 5 JSON models.
- [ ] Add more policy-evaluation evidence before replacing the lightweight
      model family.

## Risk Register

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Schema drift between training export and native inference | Wrong scores with no obvious crash | Hard schema-hash and feature-count checks |
| Calibration missing or malformed | Probability thresholds become misleading | Fatal v2 calibration policy and explicit legacy fallback metadata |
| Full-dataset scoring used as validation evidence | Leakage and overstated validation quality | Walk-forward default and explicit score-scope policy |
| Production loads wrong native library | Undefined behavior or incompatible scoring | Split production vs debug loader policies and persist library hash |
| Training script becomes a research-only artifact | Bundles cannot be reproduced | Stable CLI, config file, hashes, and generated manifest |
| Feature emitter refactor changes output | Model incompatibility | Parity tests comparing old export and new emitter |
| Native scoring remains uncancellable | Poor UX on large datasets | Cancellation checks and optional batching |
| Strict v2 validation breaks existing bundles | User disruption | Legacy v1 compatibility path with warnings |

## Verification Matrix

| Area | Tests / Commands |
| --- | --- |
| Core ML Signal score cache and validation | `cargo test -p ta_app_core ml_signal --lib` |
| Stage 5 ML confidence boundary | `cargo test -p ta_app_core ml_confidence --lib` |
| Tauri ML Signal command boundary | `cargo test -p ta_desktop_demo_app --lib commands_ml_signal` |
| Tauri ML confidence command boundary | `cargo test -p ta_desktop_demo_app --lib ml_confidence` |
| Frontend ML Signal request and UI contract | `npm --prefix web run test -- src/services/mlSignalService.test.ts src/components/MlSignalWorkspacePanel.test.ts` |
| Frontend type contract | `npm --prefix web run typecheck` |
| Bench target compile only, no benchmark execution | `cargo check -p ta_app_core --benches` |
| Focused non-benchmark ML Signal gate | `powershell -File scripts/run_ml_signal_quality_gate.ps1` |
| Native CatBoost smoke | `powershell -File scripts/run_ml_signal_native_smoke.ps1 ...` |
| Broad repo quality | `powershell -File scripts/run_repo_quality_gate.ps1` |

## Suggested Work Order

1. Add the reproducible training script and bundle manifest v2 contract.
2. Harden manifest and schema validation before touching scoring semantics.
3. Harden calibration behavior.
4. Add score-scope policy and walk-forward-native generation.
5. Refactor the feature emitter once tests protect the old behavior.
6. Harden loader policy and native library provenance.
7. Add cancellation and optional batching.
8. Promote the dedicated ML Signal quality gate.

This order prioritizes correctness and auditability before performance or UX
polish.

## Open Decisions

- [ ] Should v2 bundles require calibrated probabilities, or allow explicit
      uncalibrated sigmoid probabilities with a lower trust level?
- [ ] Should the default CatBoost training metric be AUC, Logloss, Brier, or a
      custom trading-aware metric used only for model selection?
- [ ] Should training use two independent binary models long/short forever, or
      evolve toward one multi-output contract with two score lanes?
- [ ] Should the app allow production users to override native library paths, or
      should that be developer-only?
- [ ] Should walk-forward test score-cache generation emit only test rows, or
      emit all rows with a `score_scope` / split annotation per row?
- [ ] Should fixture `.cbm` models be committed, generated in tests, or kept as
      optional local smoke artifacts?

## Done Definition

This initiative is complete when:

- A CatBoost bundle can be trained reproducibly from an exported dataset.
- The bundle can be installed and classified by the desktop app.
- Native scoring fails on schema or calibration mismatches instead of producing
  ambiguous probabilities.
- Default validation uses walk-forward test score scope when the bundle provides
  split metadata.
- Score caches and saved run artifacts contain enough provenance to audit model,
  schema, calibration, score scope, and native library identity.
- Focused Rust and frontend tests cover the strict path and legacy compatibility
  path.
- The release manifest confirms required CatBoost runtime resources are present.
