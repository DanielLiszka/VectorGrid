# ML Signal Research Harness

This folder is the offline research harness for the `MlSignal` strategy path.

Current scope:

- train external long/short tabular models from exported ML datasets
- calibrate model outputs into score/probability columns
- evaluate walk-forward rolls outside the Tauri runtime
- assemble model bundles that the app can inspect and later score natively

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
  --scores-csv C:\data\ml_signal\bundle_dev\validation_scores.csv
```

4. Run walk-forward evaluation:

```powershell
python evaluate_walkforward.py `
  --dataset C:\data\ml_dataset.csv `
  --bundle-dir C:\data\ml_signal\bundle_dev
```

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

9. Review calibration quality by split, holdout, and top ticker slices:

```powershell
python calibration_slice_report.py `
  --scores-csv C:\data\ml_signal\bundle_dev\validation_scores.csv `
  --top-tickers 25
```

10. Aggregate multiple bundle summaries by exported holdout cohort:

```powershell
python holdout_bundle_report.py `
  --bundle-dirs C:\data\ml_signal\bundle_holdout_true C:\data\ml_signal\bundle_holdout_false
```

## Current Status

Current capabilities:

- `train_catboost.py`
  - loads the exported dataset CSV plus `feature_schema.json`
  - uses the exported `keep_long` / `keep_short` masks
  - uses the exported `sample_weight`
  - uses the split manifest when present, otherwise falls back to a simple time-ordered split
  - trains CatBoost long/short classifiers
  - writes calibration scores with stable row identity (`row_index`, `ticker_id`, `split_id`, optional `ticker_holdout`)
  - writes `model_long.cbm`, `model_short.cbm`, `validation_scores.csv`, `metrics.json`, `train_metadata.json`, and `manifest.json`
- `calibrate.py`
  - fits sigmoid calibration from validation raw scores
  - writes `calibration.json`
  - rewrites `validation_scores.csv` with preferred calibrated probability columns `p_long_cal` / `p_short_cal`
  - updates `manifest.json`
- `evaluate_walkforward.py`
  - loads the saved bundle
  - scores the dataset once and evaluates each split-window test slice when a split manifest is present
  - applies optional calibration
  - writes `walkforward_test_scores.csv`, `walkforward_rolls.json`, `walkforward_tickers.json`, and `walkforward_summary.json`
- `gate_report.py`
  - reads `metrics.json`, `walkforward_summary.json`, `walkforward_rolls.json`, and `manifest.json`
  - applies repo-oriented research gates for scored rows, positive-roll rate, active share, and calibration AUC
  - folds in `baseline_comparison.json`, `holdout_report.json`, and `stability_report.json` when they are present
  - writes `gate_report.json` and `gate_report.md`
- `compare_baseline.py`
  - compares ML walk-forward summary metrics against a deterministic baseline summary JSON
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
  - summarizes long/short calibration quality overall, by split, by holdout when present, and for the largest ticker slices
  - writes `validation_slice_metrics.json`, `validation_slice_metrics.csv`, and `validation_slice_metrics.md`
- `holdout_bundle_report.py`
  - reads one or more bundle directories with `walkforward_summary.json`
  - groups bundle-level results by exported holdout cohort (`true`, `false`, `unknown`)
  - writes `holdout_bundle_summary.json`, `holdout_bundle_summary.md`, and `holdout_bundle_rows.csv`

Still open:

- direct app invocation from the Python harness for score-cache generation and policy sweeps
- native Rust model scoring inside the desktop app
- deterministic baseline comparison is still manual
- automatic ticker-holdout reporting still requires either a preserved holdout column or explicit modulus/residue inputs
- native in-app threshold-stability review is still a separate step from the Python research reports

## Score Contract Notes

- `validation_scores.csv` is a calibration-slice artifact produced from the calibration rows used by the offline training harness.
- After `calibrate.py` runs, `p_long_cal` / `p_short_cal` are the preferred probability columns for `validation_scores.csv`.
- `raw_long` / `raw_short` remain diagnostic raw-margin columns; they are not the preferred desktop validation contract.
- `walkforward_test_scores.csv` is a walk-forward proxy artifact produced by the Python harness. It is useful for research review, but it is not itself a desktop validation or policy-sweep artifact.
- Desktop `MlSignal` validation/policy sweep is still a separate score-cache-backed runtime surface and should be reviewed separately when parity matters.
