# ML Signal Research Harness

This folder is the offline research harness for the `MlSignal` strategy path.

Current scope:

- train external long/short tabular models from exported ML datasets
- calibrate model outputs into score/probability columns
- evaluate both proxy and rolling walk-forward rolls outside the Tauri runtime
- assemble model bundles that the app can inspect and later score natively

## Desktop Runtime Recommendation

The current desktop-native recommendation is:

- prefer `native_cpu` for bundle-backed runtime scoring
- treat `auto` and `native_cuda` as experimental
- keep score-cache import available as the compatibility fallback

Reason:

- CPU-native CatBoost inference is implemented and validated in the app/runtime path
- Windows GPU-native CatBoost builds are still upstream-fragile on the current CUDA 13.x stack
- the desktop workflow does not need GPU inference to remain fast enough for the current `MlSignal` scope
- packaged Windows builds now ship an app-level CPU CatBoost DLL fallback at `$RESOURCE/native/catboostmodel.dll`
- the normal desktop panel only offers `native_cpu`; experimental GPU-native backend values are kept replay-compatible for older saved runs and internal testing

## Environment

Create a virtual environment and install the pinned research dependencies:

```powershell
cd research/ml_signal
python -m venv .venv
.venv\Scripts\Activate.ps1
pip install -e .
```

## Expected Workflow

1. Export a dataset from the app using the `MlSignal` workspace.
2. Train the long and short models:

```powershell
python train_catboost.py `
  --dataset C:\data\ml_dataset.csv `
  --out-dir C:\data\ml_signal\bundle_dev
```

3. Calibrate score outputs:

```powershell
python calibrate.py `
  --bundle-dir C:\data\ml_signal\bundle_dev `
  --scores-csv C:\data\ml_signal\bundle_dev\validation_scores.csv `
  --method isotonic
```

4. Run walk-forward evaluation.

Preferred research path: rolling mode retrains and recalibrates per split window.

```powershell
python evaluate_walkforward.py `
  --dataset C:\data\ml_dataset.csv `
  --bundle-dir C:\data\ml_signal\bundle_dev `
  --mode rolling `
  --calibration-method isotonic `
  --artifact-suffix rolling
```

Legacy compatibility path: proxy mode reuses the saved bundle models and slices test windows without retraining.

```powershell
python evaluate_walkforward.py `
  --dataset C:\data\ml_dataset.csv `
  --bundle-dir C:\data\ml_signal\bundle_dev `
  --mode proxy `
  --artifact-suffix proxy
```

Use distinct bundle directories or distinct `--artifact-suffix` values if you want to keep both proxy and rolling outputs. The walk-forward scripts and downstream reports otherwise write fixed filenames and later runs will overwrite earlier artifacts.

5. Generate a research gate report:

```powershell
python gate_report.py `
  --bundle-dir C:\data\ml_signal\bundle_dev
```

6. Compare the ML walk-forward summary against a deterministic baseline summary:

```powershell
python compare_baseline.py `
  --ml-summary C:\data\ml_signal\bundle_dev\walkforward_summary.json `
  --baseline-summary C:\data\deterministic\baseline_summary.json
```

Or generate a built-in baseline directly from the same exported dataset and bundle sidecars:

```powershell
python compare_baseline.py `
  --ml-summary C:\data\ml_signal\bundle_dev\walkforward_summary.json `
  --built-in-baseline logistic_regression `
  --dataset C:\data\ml_dataset.csv `
  --bundle-dir C:\data\ml_signal\bundle_dev
```

7. Compare walk-forward scores across holdout and non-holdout ticker slices:

```powershell
python holdout_report.py `
  --scores C:\data\ml_signal\bundle_dev\walkforward_test_scores.csv `
  --modulus 5 `
  --residue 0
```

8. Review policy-threshold stability across one or more sweep/validation artifacts:

```powershell
python stability_report.py `
  --artifacts C:\data\ml_signal\run_a\policy_sweep.json C:\data\ml_signal\run_b\policy_sweep.json `
  --top-n 3
```

9. Review validation-slice calibration quality by split, holdout, and top ticker slices:

```powershell
python calibration_slice_report.py `
  --scores-csv C:\data\ml_signal\bundle_dev\validation_scores.csv `
  --top-tickers 25
```

10. Review rolling per-roll calibration diagnostics when using rolling walk-forward mode:

```powershell
python rolling_calibration_report.py `
  --bundle-dir C:\data\ml_signal\bundle_dev `
  --artifact-suffix rolling
```

11. Aggregate multiple bundle summaries by exported holdout cohort:

```powershell
python holdout_bundle_report.py `
  --bundle-dirs C:\data\ml_signal\bundle_holdout_true C:\data\ml_signal\bundle_holdout_false
```

## Native Bundle Layout For Desktop CPU Inference

Preferred operator layout for a bundle that will be scored natively by the desktop app:

- `manifest.json`
- `model_long.cbm`
- `model_short.cbm`
- `feature_schema.json`
- `export_manifest.json`
- optional `split_manifest.json`
- optional `calibration.json`
- `native/catboostmodel.dll` on Windows

Preferred manifest shape:

- keep model/schema/export paths relative to the bundle directory
- keep `native_library_path` relative to the bundle directory

Example:

```json
{
  "long_model_path": "model_long.cbm",
  "short_model_path": "model_short.cbm",
  "feature_schema_path": "feature_schema.json",
  "export_manifest_path": "export_manifest.json",
  "native_library_path": "native/catboostmodel.dll"
}
```

The runtime native loader currently searches in this order:

1. explicit UI/CLI override path
2. `CATBOOST_MODEL_INTERFACE_LIB`
3. `CATBOOST_MODEL_INTERFACE_DIR`
4. bundle directory
5. bundle `native/`
6. bundle `lib/`
7. executable directory
8. executable `native/`
9. executable `lib/`
10. current working directory
11. current working directory `native/`
12. bare platform default filename such as `catboostmodel.dll`

Operational guidance:

- prefer shipping the CPU CatBoost library inside the bundle under `native/`
- use the UI `Native library override` only for local debugging or temporary operator overrides
- leave the GPU device field blank unless you are explicitly testing an experimental GPU-native CatBoost library

## Current Status

Current capabilities:

- `train_catboost.py`
  - loads the exported dataset CSV plus `feature_schema.json`
  - uses the exported `keep_long` / `keep_short` masks
  - uses the exported `sample_weight`
  - uses the split manifest when present, otherwise falls back to a simple time-ordered split
  - trains CatBoost long/short classifiers
  - writes calibration scores with stable row identity (`row_index`, `ticker_id`, `split_id`, optional `ticker_holdout`)
  - copies bundle-side `feature_schema.json`, `export_manifest.json`, and `split_manifest.json` when present
  - writes reproducibility metadata including the copied sidecar paths, training config, and Python/dependency versions
  - writes `model_long.cbm`, `model_short.cbm`, `validation_scores.csv`, `metrics.json`, `train_metadata.json`, and `manifest.json`
- `calibrate.py`
  - fits `sigmoid` or `isotonic` calibration from validation raw scores
  - handles side-specific validation rows when `keep_long` and `keep_short` diverge
  - writes `calibration.json`
  - rewrites `validation_scores.csv` with preferred calibrated probability columns `p_long_cal` / `p_short_cal`
  - updates `manifest.json` and `train_metadata.json` with additive calibration summaries
- `evaluate_walkforward.py`
  - loads the saved bundle plus bundle-side copied schema/export/split manifests when present
  - supports `--mode proxy` and `--mode rolling`
  - supports `--calibration-method auto|sigmoid|isotonic|none` for rolling mode
  - supports `--artifact-suffix` so multiple walk-forward modes can coexist in one bundle directory
  - `proxy` mode scores the dataset once with the saved bundle models, applies optional bundle calibration, and evaluates each split-window test slice
  - `rolling` mode retrains long/short models on each split-window train slice, fits per-roll calibration on that split-window calibration slice, and scores only that split-window test slice
  - persists the replayed training contract source and replay config into `walkforward_summary.json`
  - persists per-roll long/short calibration diagnostics into `walkforward_rolls.json`
  - writes `walkforward_test_scores.csv`, `walkforward_rolls.json`, `walkforward_tickers.json`, and `walkforward_summary.json`
- `gate_report.py`
  - reads `metrics.json`, `walkforward_summary.json`, `walkforward_rolls.json`, and `manifest.json`
  - uses `metrics.json` AUCs for `proxy` mode and rolling calibration diagnostics for `rolling` mode
  - applies repo-oriented research gates for scored rows, positive-roll rate, active share, and calibration AUC
  - surfaces calibrated logloss/Brier when available
  - folds in `baseline_comparison.json`, `holdout_report.json`, and `stability_report.json` when they are present
  - writes `gate_report.json` and `gate_report.md`
- `compare_baseline.py`
  - compares ML walk-forward summary metrics against a deterministic baseline summary JSON
  - can also generate built-in `logistic_regression` or `class_prior` baselines directly from the exported dataset and bundle sidecars
  - writes `baseline_comparison.json` and `baseline_comparison.md`
- `holdout_report.py`
  - partitions `walkforward_test_scores.csv` into holdout and non-holdout groups
  - supports either an explicit holdout column or the same stable hash modulus/residue rule used by the Rust exporter
  - writes `holdout_report.json` and `holdout_report.md`
- `stability_report.py`
  - reads one or more ML sweep/validation artifact JSON files
  - summarizes how often the same top policy reappears and how tightly key thresholds cluster
  - writes `stability_report.json` and `stability_report.md`
- `calibration_slice_report.py`
  - reads `validation_scores.csv`
  - prefers calibrated probability columns `p_long_cal` / `p_short_cal` when they are present
  - summarizes long/short calibration quality overall, by split, by holdout when present, and for the largest ticker slices
  - includes Brier plus raw-vs-preferred probability comparisons
  - writes `validation_slice_metrics.json`, `validation_slice_metrics.csv`, and `validation_slice_metrics.md`
- `rolling_calibration_report.py`
  - reads `walkforward_summary.json` and `walkforward_rolls.json`
  - summarizes per-roll long/short calibration diagnostics, including AUC/logloss/Brier deltas
  - writes `rolling_calibration_report.json`, `rolling_calibration_report.csv`, and `rolling_calibration_report.md`
- `baseline_walkforward.py`
  - generates built-in research baselines against the exported dataset using the same split windows as the ML bundle
  - currently supports `logistic_regression` and `class_prior`
  - writes built-in baseline summary/score artifacts inside the bundle directory
- `holdout_bundle_report.py`
  - reads one or more bundle directories with `walkforward_summary.json`
  - groups bundle-level results by exported holdout cohort (`true`, `false`, `unknown`)
  - writes `holdout_bundle_summary.json`, `holdout_bundle_summary.md`, and `holdout_bundle_rows.csv`

Still open:

- direct app invocation from the Python harness for score-cache generation and policy sweeps
- automatic ticker-holdout reporting still requires either a preserved holdout column or explicit modulus/residue inputs
- native in-app threshold-stability review is still a separate step from the Python research reports

## Score Contract Notes

- `validation_scores.csv` is a calibration-slice artifact produced from the calibration rows used by the offline training harness.
- After `calibrate.py` runs, `p_long_cal` / `p_short_cal` are the preferred probability columns for `validation_scores.csv`.
- `raw_long` / `raw_short` remain diagnostic raw-margin columns; they are not the preferred desktop validation contract.
- `walkforward_test_scores.csv` is still a research artifact. Check `walkforward_summary.json.evaluation_mode` to determine whether it came from `proxy` or `rolling` evaluation.
- When you keep multiple mode variants, the artifact suffix becomes part of the filename, for example `walkforward_summary.proxy.json` or `walkforward_summary.rolling.json`.
- `proxy` mode is a one-shot fixed-model approximation. It is useful for continuity with older bundles, but it is not a no-future-leak rolling evaluation.
- `rolling` mode retrains and recalibrates per split window. It is the preferred offline research path when causal walk-forward behavior matters.
- Desktop `MlSignal` validation/policy sweep is still a separate score-cache-backed runtime surface and should be reviewed separately when parity matters.
