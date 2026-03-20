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

## Current Status

Current capabilities:

- `train_catboost.py`
  - loads the exported dataset CSV plus `feature_schema.json`
  - uses the exported `keep_long` / `keep_short` masks
  - uses the exported `sample_weight`
  - uses the split manifest when present, otherwise falls back to a simple time-ordered split
  - trains CatBoost long/short classifiers
  - writes `model_long.cbm`, `model_short.cbm`, `validation_scores.csv`, `metrics.json`, `train_metadata.json`, and `manifest.json`
- `calibrate.py`
  - fits sigmoid calibration from validation raw scores
  - writes `calibration.json`
  - updates `manifest.json`
- `evaluate_walkforward.py`
  - loads the saved bundle
  - scores the dataset once and evaluates each split-window test slice when a split manifest is present
  - applies optional calibration
  - writes `walkforward_test_scores.csv`, `walkforward_rolls.json`, and `walkforward_summary.json`

Still open:

- direct app invocation from the Python harness for score-cache generation and policy sweeps
- native Rust model scoring inside the desktop app
