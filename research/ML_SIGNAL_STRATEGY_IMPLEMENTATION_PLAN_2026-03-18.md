# ML Signal Strategy Implementation Plan

Last updated: 2026-03-18
Scope: `ta_desktop_demo` backend + frontend + research tooling
Goal: add a new supervised ML strategy family that coexists with the current deterministic optimization/backtest workflows.

## 1. Executive Summary

This plan adds a new product path alongside the current rule/grid optimizer:

- Path A: keep the current deterministic optimization and validation stack unchanged.
- Path B: add a supervised ML signal strategy family that produces bar-level probabilities or scores, maps them into target positions, and evaluates those positions with the existing simulator and validation stack.

The correct implementation target for this repo is not "replace the backtester with ML."
The correct implementation target is:

- compute bar-level features from candles
- build offline supervised datasets
- train and version external models
- run inference in a repo-owned runtime path
- convert scores to target positions
- reuse current validation, cross-ticker, holdout, and artifact flows where possible

This plan keeps only the parts that held up against the current code review and repo architecture.

## 2. Why This Feature Is Worth Adding

The repo already has strong capabilities in:

- candle ingestion and caching
- CPU and CUDA indicator computation
- strategy-local signal generation
- vectorized and iterative validation
- cross-ticker validation
- artifact persistence
- an existing but narrower ML-adjacent subsystem (`ml_confidence`)

Those pieces are enough to support a new ML signal family if the implementation is disciplined.

Expected value:

- introduce a new nonlinear signal option without destabilizing the current optimizer
- reuse the repo's strongest component, the simulator/validation stack
- allow ML to be compared directly against deterministic baselines under the same validation rules
- preserve CPU fallback and keep CUDA optional

## 3. Repo Constraints That Must Shape The Design

This implementation must respect the repo's current architectural and operational rules.

### 3.1 Strategy-local architecture wins

`AGENTS.md` explicitly prefers strategy-local organization for new backends and validation routes.

That means the implementation should not start as a giant generic `ml/*` subsystem that everything depends on.

Preferred shape:

- add one new strategy family or profile for ML scoring
- keep its request/config, runtime scoring, and validation adapter grouped together
- extract shared primitives only when they are clearly cross-cutting

Shared ML infrastructure is acceptable for:

- model metadata/schema/versioning
- score calibration
- dataset partition helpers
- artifact/report plumbing

But the first production path should still read as a concrete strategy family, not as a generic framework.

### 3.2 CPU-only builds must remain healthy

Current feature wiring keeps CUDA optional through:

- `crates/app_core/Cargo.toml`
- `src-tauri/Cargo.toml`

The ML path must:

- build and run in CPU-only mode
- avoid introducing a hard CatBoost/XGBoost/CUDA runtime dependency into default app startup
- keep offline training tooling separable from core runtime builds

### 3.3 Reuse the simulator, do not fork it

Current validation already supports desired-position vectors through strategy-local series generation and iterative simulation:

- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

The ML path should reuse that pattern by introducing a clean "scores -> desired positions -> existing simulator" route instead of inventing a second trade engine.

## 4. Valid Architecture To Implement

## 4.1 Product shape

Implement a new strategy family:

- working name: `MlSignal`

Responsibilities:

- define the feature schema
- generate bar-level features from candles
- load a trained model bundle
- score each eligible bar
- map scores into target positions
- pass positions into the existing validation and execution machinery

### 4.2 System flow

Recommended v1 flow:

1. candles loaded from existing data path
2. feature computation on CPU or CUDA using current indicator stack
3. offline dataset export for train/calibration/test
4. offline model training outside the latency-sensitive app path
5. saved model bundle + schema + calibration artifact
6. runtime scoring in Rust host
7. score-to-position policy mapping
8. existing vectorized/iterative validation on desired positions
9. artifact persistence and comparison against deterministic baselines

### 4.3 What stays out of scope for v1

- RL or path-dependent policy learning
- ML-driven exit learning
- replacing existing strategy families
- mandatory GPU-resident inference
- broad generic multi-model orchestration framework
- auto-enabling live trading without explicit strategy-level opt-in

## 5. Recommended Initial Scope

Keep the first production slice narrow.

Recommended v1:

- Universe: liquid U.S. equities subset
- Timeframe: daily
- Forecast horizon: fixed horizon such as `H = 10`
- Model outputs: `P_long`, `P_short`
- Runtime policy: long / short / flat target positions
- Exit policy: still owned by deterministic policy logic around score streams
- Release mode: research/shadow first, then validation-only, then optional live-readiness review

Reason:

- smallest leakage surface
- easiest train/calibrate/test split discipline
- lowest runtime complexity
- easiest apples-to-apples comparison with the existing stack

## 6. Model and Runtime Direction

## 6.1 Recommended model direction for v1

Use a supervised tabular model family first.

Recommended default:

- two binary classifiers:
- model L predicts long-edge probability
- model S predicts short-edge probability

CatBoost is a reasonable first research target because:

- feature families here are mostly dense numeric tabular inputs
- inference integration from Rust is feasible through external model apply paths
- trees avoid a heavy scaling/normalization burden

However, CatBoost integration must be treated as an external runtime/tooling project, not as a small repo-local extension.

## 6.2 Runtime output contract

The runtime scorer should expose:

- `p_long: Option<f64>`
- `p_short: Option<f64>`
- `raw_long: Option<f64>`
- `raw_short: Option<f64>`
- model/schema version identifiers
- first valid bar index

The runtime should not emit direct order actions.

It should emit score streams that feed a deterministic mapping layer.

Current repo truthfulness note:

- the shipped desktop path currently validates from score-cache files using probability-threshold policies
- raw margins are still useful as diagnostics, but they are not the intended desktop validation contract
- the Python research harness may emit both calibration-slice and walk-forward artifacts, and those scopes should not be treated as interchangeable

## 6.3 Position mapping contract

Recommended mapping:

- target `+1` if `p_long >= t_long` and `p_long - p_short >= delta`
- target `-1` if `p_short >= t_short` and `p_short - p_long >= delta`
- target `0` otherwise

V1 should also support:

- max hold bars
- opposite-signal exit
- flat-signal exit
- optional cooldown bars
- optional persistence bars

Those policies fit the repo better than asking the model to learn exits.

## 6.4 Decisions To Lock Now

The earlier version of this plan left too many implementation choices open.
Lock these defaults now unless there is a deliberate later decision to revise them.

- Strategy kind name: `MlSignal`
- Product scope for v1: research + validation only
- Offline training runtime: Python harness under `research/ml_signal/`
- Python dependency set: `catboost`, `scikit-learn`, `polars` or `pandas`, `pyarrow`, `numpy`, `pydantic`
- Runtime rollout step 1: ship score-cache-backed validation
- Runtime rollout step 2: add native Rust model scoring against the same bundle format
- Native model format default: CatBoost `.cbm`

Truthfulness guardrail for current readers:

- `validation_scores.csv` from the research harness is a calibration-slice artifact
- `walkforward_test_scores.csv` is a Python-side walk-forward proxy artifact
- neither file by itself is proof of desktop validation/policy-sweep parity unless the desktop path is exercised separately against the same contract
- Calibration format default: repo-owned JSON containing sigmoid or isotonic mappings
- Frontend scope for v1: dataset export, model status, score-cache generation, validation, artifact review
- Explicitly out of scope for v1: training models from the Tauri UI

This removes the biggest ambiguity from the earlier draft:
training is offline, validation is the first in-app consumer, and native Rust inference is a second step that must preserve the same bundle contract.

## 7. Feature Plan

The feature set should be compact and schema-versioned.

## 7.1 Feature principles

- use normalized and relative features instead of raw absolute price
- prefer features already available in `vector-ta` and current indicator plumbing
- preserve a stable feature order
- keep the training and runtime schema identical
- avoid aggressive search over feature hyperparameters in v1

## 7.2 V1 feature blocks

### A. Candlestick pattern block

The repo's current `vector-ta` version provides `pattern_recognition` support with pattern IDs and matrix outputs.

Use:

- raw pattern hit flags per bar
- rolling counts over short windows
- bars-since-last-hit features
- bullish/bearish aggregate counts

Important repo note:

- current dependency evidence points to a 61-pattern matrix, not 63
- the exact count must be read from the active `vector-ta` build/runtime, not hard-coded in the app

### B. Candle geometry block

Include normalized bar-shape features such as:

- body percent
- absolute body fraction of range
- upper wick fraction
- lower wick fraction
- close location in range
- gap percent
- true range percent

These are cheap, explainable, and cross-asset friendly.

### C. Trend block

Leverage the repo's existing trend primitives:

- ALMA bank at fixed windows
- ATR-normalized spread and slope measures
- ADX / DI strength and direction measures

Do not turn v1 into an optimizer over ALMA parameters.
Fix the trend bank definition and optimize the downstream policy around the scores.

### D. Regime / volatility / momentum block

Include a compact set:

- short-horizon log returns
- realized volatility windows
- ATR relative to close
- rolling range position
- breakout distance normalized by ATR
- relative volume where volume is meaningful

## 7.3 Feature implementation requirement

Every feature must have:

- exact formula
- input requirements
- warmup rule
- first-valid rule
- NaN/invalid handling rule
- schema name and stable index

## 7.4 Canonical V1 Feature Schema

The feature spec below is the canonical v1 contract.
If a later implementation wants to add or remove features, that must create a new schema version.

### 7.4.1 Common rules

- Runtime numeric type in Rust: `f64`
- Export numeric type: `f64` in Rust, optionally downcast inside the Python trainer only
- Label type: `u8`
- Score type: `f64`
- `eps = 1e-12`
- `H` is the forecast horizon in bars
- `H2 = max(1, floor(H / 2))`
- `W1 = H`
- `W2 = 3H`
- `W3 = 10H`
- Rolling-count windows are trailing inclusive windows ending at bar `t`
- Range-position windows are trailing inclusive windows ending at bar `t`
- Breakout windows use the trailing window ending at `t - 1`
- `first_valid_index` for a scored/exported row is the maximum first-valid requirement across all features
- No exported or scored row at or after `first_valid_index` may contain `NaN` or `inf`
- Any zero-denominator normalization uses `max(abs(denominator), eps)`

### 7.4.2 Pattern-feature ordering contract

The repo must not hard-code a pattern count.
It must read the active `pattern_ids` from the current `vector-ta` runtime and persist them into the schema artifact.

Given ordered `pattern_ids = [p0, p1, ... pN]`, emit features in this exact order:

1. For each pattern `p` in `pattern_ids`:
- `pat::{p}::hit`
- `pat::{p}::count_3`
- `pat::{p}::count_5`
- `pat::{p}::count_h`
- `pat::{p}::count_2h`
- `pat::{p}::bars_since_last_clip_4h`
2. Aggregate windows in this exact order for each window `3`, `5`, `h`, `2h`:
- `pat_agg::bull_count_{window}`
- `pat_agg::bear_count_{window}`
- `pat_agg::neutral_count_{window}`
- `pat_agg::bull_minus_bear_{window}`

Pattern value rules:

- `hit` is `1.0` when the pattern fires on bar `t`, otherwise `0.0`
- count features are integer counts emitted as `f64`
- `bars_since_last_clip_4h` is the number of bars since the most recent hit of the same pattern, clipped to `4H`, with `4H` used when no prior hit exists inside the lookback
- bullish/bearish/neutral aggregate counts are based on the pattern metadata classification returned by the active pattern catalog

### 7.4.3 Fixed numeric feature list

Emit the following fixed numeric features after all pattern features.

Geometry block:

- `geom::body_pct = (close_t - open_t) / max(abs(open_t), eps)`
- `geom::abs_body_frac = abs(close_t - open_t) / max(high_t - low_t, eps)`
- `geom::upper_wick_frac = (high_t - max(open_t, close_t)) / max(high_t - low_t, eps)`
- `geom::lower_wick_frac = (min(open_t, close_t) - low_t) / max(high_t - low_t, eps)`
- `geom::close_location = (close_t - low_t) / max(high_t - low_t, eps)`
- `geom::gap_pct = (open_t - close_{t-1}) / max(abs(close_{t-1}), eps)`
- `geom::true_range_pct = TR_t / max(abs(close_{t-1}), eps)`

Trend block:

- `trend::alma_h`
- `trend::alma_3h`
- `trend::alma_10h`
- `trend::alma_h_minus_alma_3h_atr_h = (ALMA_H_t - ALMA_3H_t) / max(ATR_H_t, eps)`
- `trend::alma_3h_minus_alma_10h_atr_h = (ALMA_3H_t - ALMA_10H_t) / max(ATR_H_t, eps)`
- `trend::alma_h_slope_half_h_atr_h = (ALMA_H_t - ALMA_H_{t-H2}) / max(ATR_H_t, eps)`
- `trend::alma_3h_slope_h_atr_h = (ALMA_3H_t - ALMA_3H_{t-H}) / max(ATR_H_t, eps)`
- `trend::close_minus_alma_3h_atr_h = (close_t - ALMA_3H_t) / max(ATR_H_t, eps)`
- `trend::adx_2h`
- `trend::di_spread_norm_2h = (+DI_2H_t - -DI_2H_t) / max(+DI_2H_t + -DI_2H_t, eps)`

Regime / volatility / momentum block:

- `mom::logret_1 = ln(close_t / close_{t-1})`
- `mom::logret_2 = ln(close_t / close_{t-2})`
- `mom::logret_3 = ln(close_t / close_{t-3})`
- `mom::logret_5 = ln(close_t / close_{t-5})`
- `mom::logret_h = ln(close_t / close_{t-H})`
- `vol::realized_vol_h = sqrt(mean((logret_1)^2 over last H bars))`
- `vol::realized_vol_3h = sqrt(mean((logret_1)^2 over last 3H bars))`
- `vol::atr_h_over_close = ATR_H_t / max(abs(close_t), eps)`
- `range::pos_h = (close_t - rolling_low_H_inc_t) / max(rolling_high_H_inc_t - rolling_low_H_inc_t, eps)`
- `range::pos_3h = (close_t - rolling_low_3H_inc_t) / max(rolling_high_3H_inc_t - rolling_low_3H_inc_t, eps)`
- `breakout::above_roll_high_h_atr_h = max(0, close_t - rolling_high_H_excl_t) / max(ATR_H_t, eps)`
- `breakout::below_roll_low_h_atr_h = max(0, rolling_low_H_excl_t - close_t) / max(ATR_H_t, eps)`
- `vol::volume_over_sma20 = volume_t / max(SMA(volume, 20)_t, eps)`

### 7.4.4 Warmup and missing-data rules

- `geom::gap_pct` and `geom::true_range_pct` first become valid at `t = 1`
- log-return features first become valid at their lookback
- `vol::volume_over_sma20` first becomes valid at `t = 19`
- `trend::alma_10h` first becomes valid only after the underlying ALMA implementation is valid for `10H`
- `trend::alma_h_slope_half_h_atr_h` requires both `ALMA_H_t` and `ALMA_H_{t-H2}`
- `trend::alma_3h_slope_h_atr_h` requires both `ALMA_3H_t` and `ALMA_3H_{t-H}`
- `range::pos_h` requires at least `H` bars
- `range::pos_3h` requires at least `3H` bars
- breakout features require at least `H + 1` bars because the comparison window excludes the current bar
- pattern rolling counts require their respective windows
- before `first_valid_index`, the extractor may use `NaN` internally, but exported/scored rows before that index must be dropped
- after `first_valid_index`, any invalid feature is a hard error, not a silent fill

### 7.4.5 Dataset columns and ordering

The offline dataset export must emit columns in this exact order:

1. `ticker_id`
2. `timestamp`
3. `split_id`
4. `sample_weight`
5. all features in schema order
6. `y_long`
7. `y_short`

`split_id`, `ticker_id`, and `timestamp` are metadata columns, not training features.

## 8. Label Plan

V1 labeling should remain fixed-horizon and deterministic.

Recommended label timing:

- signal formed at bar close `t`
- assumed entry at `open[t+1]`
- label measured at `open[t+H+1]`

Recommended targets:

- `y_long = 1` when forward long return exceeds a deadband
- `y_short = 1` when forward short return exceeds a deadband

Recommended deadband inputs:

- transaction costs
- volatility-aware threshold such as ATR-relative deadband

Rows near zero edge should be dropped from the training target for the corresponding model.

This is the cleanest label design for the current repo because it aligns with vectorized research and avoids path-dependent label complexity.

## 9. Data Split and Evaluation Rules

## 9.1 Split rules

Use only time-ordered splits.

Recommended baseline:

- train: 5 years
- calibration: 1 year
- test: 1 year
- retrain cadence: quarterly

Also test:

- rolling train windows
- expanding windows

### 9.2 Leakage controls

Must-have rules:

- embargo/gap of at least `H` bars between train and downstream split boundaries
- no future-derived labels in training features
- no using the same rows for early stopping, calibration, and final testing
- ticker holdout slice inside each evaluation roll

### 9.3 Success criteria

Call the path successful only if all are true:

- beats deterministic baseline on untouched future window
- still works on ticker holdout slice
- calibrated probabilities are stable across multiple rolls
- model health is not confined to one narrow market regime
- score-to-policy thresholds are not wildly unstable across rolls

## 10. Artifact Design

The ML path needs its own first-class artifact family.

## 10.1 Model bundle contents

Each trained bundle should include:

- model file(s)
- feature schema JSON
- label config JSON
- calibration artifact
- training metadata
- dataset partition metadata
- metrics summary
- model runtime identifier

Suggested contents:

- `model_long.*`
- `model_short.*`
- `feature_schema.json`
- `label_schema.json`
- `calibration_long.json`
- `calibration_short.json`
- `train_metadata.json`
- `metrics.json`

## 10.2 Score cache artifact

The repo needs a dedicated score-cache artifact for policy evaluation.

Suggested contents:

- strategy scope
- model bundle reference
- dataset identity
- timeframe
- timestamp vector
- `p_long`
- `p_short`
- optional raw scores
- first valid index
- score generation metadata

The score cache is the key enabler for fast threshold/hold/exit sweeps without re-running model inference every time.

## 10.3 Run artifact integration

Validation artifacts for ML strategies should include:

- model bundle identity
- score-cache identity when used
- policy thresholds
- mapping config
- cross-ticker summary
- walk-forward summary
- probability metrics where applicable

## 10.4 Artifact JSON Skeletons

These are the starting point for the Rust structs and JSON contract.

Model bundle manifest:

```json
{
  "bundle_kind": "vectorgrid_ml_signal_bundle",
  "bundle_version": 1,
  "bundle_id": "mlsig_us_daily_h10_2026q1_cbm_v1",
  "strategy_kind": "MlSignal",
  "created_utc": "2026-03-18T20:30:00Z",
  "runtime": {
    "scoring_mode": "score_cache_then_native",
    "model_family": "catboost_binary_pair",
    "native_format": "cbm",
    "requires_volume": true
  },
  "schema": {
    "feature_schema_version": "ml_signal_v1_daily_h10",
    "horizon_bars": 10,
    "feature_count": 405,
    "pattern_ids": ["cdl_doji", "cdl_hammer"]
  },
  "models": {
    "long": {
      "path": "model_long.cbm",
      "prediction_type": "RawFormulaVal"
    },
    "short": {
      "path": "model_short.cbm",
      "prediction_type": "RawFormulaVal"
    }
  },
  "calibration": {
    "long_path": "calibration_long.json",
    "short_path": "calibration_short.json"
  },
  "label": {
    "horizon_bars": 10,
    "entry_bar": "next_open",
    "exit_bar": "open_t_plus_h_plus_1",
    "cost_basis_bps": 10
  },
  "training": {
    "trainer": "research/ml_signal/train_catboost.py",
    "dataset_fingerprint": "sha256:...",
    "train_window": "2018-01-01..2022-12-31",
    "calibration_window": "2023-01-01..2023-12-31",
    "test_window": "2024-01-01..2024-12-31"
  }
}
```

Score-cache artifact:

```json
{
  "kind": "vectorgrid_ml_signal_score_cache",
  "version": 1,
  "ts": "2026-03-18T20:45:00Z",
  "strategy_kind": "MlSignal",
  "bundle_id": "mlsig_us_daily_h10_2026q1_cbm_v1",
  "feature_schema_version": "ml_signal_v1_daily_h10",
  "dataset_meta": {
    "csv_path": "C:\\data\\spy.csv",
    "timeframe": "1d",
    "source_content_fingerprint": "sha256:..."
  },
  "validation_context": {
    "strategy_kind": "MlSignal",
    "csv_path": "C:\\data\\spy.csv",
    "timeframe": "1d",
    "label": "SPY"
  },
  "first_valid_index": 101,
  "bars_scored": 2516,
  "score_columns": {
    "timestamp": [1704067200000],
    "p_long": [0.61],
    "p_short": [0.12],
    "raw_long": [0.48],
    "raw_short": [-0.31]
  }
}
```

Run-artifact ML extension block:

```json
{
  "kind": "vectorgrid_run_artifact",
  "strategy_kind": "MlSignal",
  "ml_signal": {
    "bundle_id": "mlsig_us_daily_h10_2026q1_cbm_v1",
    "feature_schema_version": "ml_signal_v1_daily_h10",
    "score_cache_path": "C:\\Users\\...\\vectorgrid_ml_signal_score_2026-03-18T20-45-00.json",
    "policy": {
      "t_long": 0.62,
      "t_short": 0.60,
      "delta": 0.15,
      "max_hold_bars": 10,
      "cooldown_bars": 2,
      "persistence_bars": 1,
      "long_only": false,
      "allow_flip": true
    }
  }
}
```

## 10.5 Storage and Lifecycle Policy

Use these locations and failure rules.

- Model bundles: `app_local_data_dir()/ml_signal/bundles/<bundle_id>/`
- Score caches: `app_local_data_dir()/runs/vectorgrid_ml_signal_score_<ts>.json`
- Validation artifacts: keep using `app_local_data_dir()/runs/vectorgrid_run_<ts>.json`
- Offline dataset exports: user-selected output directory, never implicit app-local data only
- Bundle identity key: `bundle_id`
- Schema identity key: `feature_schema_version`
- Cache identity key: bundle ID + schema version + dataset fingerprint + timeframe + row count

Hard-fail conditions:

- missing manifest
- missing model file
- missing calibration file when calibration is marked required
- feature count mismatch
- pattern-ID mismatch
- horizon mismatch
- dataset fingerprint mismatch for a claimed reusable score cache

Forbidden silent fallbacks:

- do not fall back from calibrated scores to raw scores without an explicit bundle flag
- do not fall back from native model scoring to a stale score cache
- do not fall back from a schema mismatch to partial-feature scoring
- do not silently coerce missing volume data to zero for `volume_over_sma20`

## 11. Proposed Repo Layout

This should be strategy-first, not framework-first, and it needs a concrete file-touch map.

## 11.1 Backend files to create

Core crate:

- `crates/app_core/src/ml_signal/mod.rs`
- `crates/app_core/src/ml_signal/request.rs`
- `crates/app_core/src/ml_signal/feature_schema.rs`
- `crates/app_core/src/ml_signal/features.rs`
- `crates/app_core/src/ml_signal/labels.rs`
- `crates/app_core/src/ml_signal/dataset_export.rs`
- `crates/app_core/src/ml_signal/model_bundle.rs`
- `crates/app_core/src/ml_signal/score_cache.rs`
- `crates/app_core/src/ml_signal/calibration.rs`
- `crates/app_core/src/ml_signal/runtime.rs`
- `crates/app_core/src/ml_signal/signal_policy.rs`
- `crates/app_core/src/ml_signal/validate.rs`
- `crates/app_core/src/ml_signal/tests.rs`

Tauri app:

- `src-tauri/src/commands_ml_signal.rs`

Research/offline tooling:

- `research/ml_signal/README.md`
- `research/ml_signal/pyproject.toml`
- `research/ml_signal/train_catboost.py`
- `research/ml_signal/calibrate.py`
- `research/ml_signal/evaluate_walkforward.py`

## 11.2 Backend files to modify

- `crates/app_core/src/lib.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_validation.rs`

Modify shared files only for one of these reasons:

- expose the new request/result types
- add artifact listing/loading support
- reuse cross-ticker helpers already living in `commands_validation.rs`

Do not bury `MlSignal` inside the existing Double-MA or Composite validation request types.

## 11.3 Frontend files to create

- `web/src/services/mlSignalRequestBuilder.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/stores/mlSignalStore.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `web/src/services/mlSignalRequestBuilder.test.ts`
- `web/src/services/mlSignalService.test.ts`

## 11.4 Frontend files to modify

- `web/src/tauri/commands.ts` via generator only
- `web/src/stores/taskStore.ts`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/services/historyService.ts`
- `web/src/services/analysisService.ts`
- `web/src/services/metaService.ts` only if `MlSignal` gets dedicated artifact tabs
- `web/src/services/validationService.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/stores/strategyStore.ts`
- `web/src/stores/validationStore.ts`
- `web/src/stores/optimizeStore.ts` only if the shared strategy selector must expose `MlSignal`
- `web/src/services/optimizeRequestBuilder.ts` and `web/src/services/optimizeService.ts` only if `MlSignal` is intentionally routed through shared optimize UX

## 11.5 Shared helpers that may remain shared

If needed, shared ML helpers may live under `crates/app_core/src/ml_shared/`, but only for:

- schema version helpers
- calibration primitives
- artifact metadata structs
- split helpers

The actual feature extractor, score cache, policy mapping, and validation adapter remain under `ml_signal/`.

## 11.6 Tauri command surface

Add a dedicated ML command family instead of overloading current optimization commands.

Recommended commands:

- `list_ml_signal_model_bundles`
- `ml_signal_model_status`
- `export_ml_signal_dataset`
- `generate_ml_signal_score_cache`
- `run_ml_signal_validation`

Do not add a Tauri `train_ml_signal_model` command in v1.
Training remains offline.

After adding commands:

- export them from `src-tauri/src/commands.rs`
- register them in `src-tauri/src/lib.rs`
- regenerate `web/src/tauri/commands.ts` with `npm run gen:commands`
- update `web/src/stores/taskStore.ts` `PROGRESS_ELIGIBLE_COMMANDS`
- update `web/src/stores/taskStore.ts` `CANCEL_COMMAND_BY_COMMAND`

## 11.7 Concrete request and response contracts

`export_ml_signal_dataset` request:

- `data_id: string`
- `export_dir: string`
- `horizon_bars: usize`
- `schema_version: string`
- `label_config`
- `split_config`
- `universe_label: string`
- `validation_context`

`export_ml_signal_dataset` response:

- `dataset_path: string`
- `schema_path: string`
- `rows_exported: usize`
- `first_valid_index: usize`
- `dataset_fingerprint: string`

`ml_signal_model_status` request:

- `bundle_id: string` or `manifest_path: string`

`ml_signal_model_status` response:

- `state: Missing | Blocked | Warning | Healthy`
- `bundle_id: string`
- `feature_schema_version: string`
- `horizon_bars: usize`
- `pattern_count: usize`
- `native_runtime_available: bool`
- `issues: string[]`

`generate_ml_signal_score_cache` request:

- `data_id: string`
- `bundle_ref`
- `backend`
- `validation_context`
- `force_refresh: bool`

`generate_ml_signal_score_cache` response:

- `score_cache_path: string`
- `bars_scored: usize`
- `first_valid_index: usize`
- `bundle_id: string`
- `feature_schema_version: string`

`run_ml_signal_validation` request:

- `data_id: string`
- `bundle_ref`
- `score_source: generate | cached`
- `score_cache_path?: string`
- `policy`
- `backend`
- `walk_forward`
- `reopt_walk_forward: null` in v1
- `spa`
- `perturb`
- `trade_bootstrap`
- `cross_ticker`
- `cscv_pbo`
- `seed`
- `validation_context`

`run_ml_signal_validation` response:

- `ts: string`
- `artifact_path: string`
- `score_cache_path: string`
- `req`
- `res`

## 11.8 Validation route decision

The current frontend/backend split is:

- Double-like strategies -> `run_double_ma_validation`
- Composite-like strategies -> `run_composite_validation`

`MlSignal` should be a third route:

- `MlSignal` -> `run_ml_signal_validation`

Backend integration rule:

- `src-tauri/src/commands_validation.rs` may keep shared cross-ticker helpers
- the actual ML entrypoint and normalization logic should live in `src-tauri/src/commands_ml_signal.rs`
- the core request/result types should live in `crates/app_core/src/ml_signal/`

This keeps ownership obvious and avoids contaminating current validation request types with ML-only fields.

## 12. Concrete Backend Integration Plan

## 12.1 Phase 1: Define contracts only

Add the core Rust types without model integration:

- ML strategy request
- feature schema structs
- label config structs
- model bundle metadata structs
- score stream structs
- policy config structs

Outputs of this phase:

- compile-time shape exists
- no training/inference yet
- no frontend exposure yet

## 12.2 Phase 2: Feature extraction engine

Implement bar-level feature extraction from `Candles`.

Requirements:

- CPU implementation first
- CUDA acceleration optional behind existing feature flags
- first-valid computation
- deterministic schema ordering
- unit tests for shape, warmup, NaN handling, and schema stability

Touchpoints likely needed:

- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/indicator_catalog.rs`
- new `ml_signal` feature modules

## 12.3 Phase 3: Label generation and offline dataset export

Implement:

- fixed-horizon label generator
- split generator
- export to a stable training format

Recommendation:

- keep training export outside latency-sensitive app code paths
- write a headless CLI or dedicated offline command for dataset generation

Export format for v1:

- simple, explicit, stable
- columnar preferred if dependency cost is acceptable
- otherwise CSV/JSON plus schema metadata as a first pass

Concrete default:

- emit `dataset.parquet`
- emit adjacent `feature_schema.json`
- emit adjacent `split_manifest.json`
- emit adjacent `export_manifest.json`

## 12.4 Phase 4: Offline training harness

Implement training outside the main runtime path.

Requirements:

- train long and short models separately
- support calibration stage
- emit reproducible metadata
- reject unhealthy models
- version the feature schema

Training should likely live in:

- a dedicated Rust binary wrapper around external tooling
- or a companion script directory invoked from research tooling

The runtime app should consume trained bundles, not own the full model-training loop.

Concrete default:

- Python training harness under `research/ml_signal/`
- Rust owns dataset export
- Python owns model fitting, calibration, metrics generation, and bundle assembly

## 12.5 Phase 5: Runtime scorer

Implement model-bundle loading and scoring in the backend.

Requirements:

- lazy loading
- explicit error surfaces
- chunked scoring
- CPU-safe fallback
- stable schema/version checks
- calibrated and raw score outputs

This phase must not silently score with a mismatched feature schema.

Concrete rollout:

1. score-cache generation path that computes features and applies a bundle in one blocking command
2. validation path that can reuse that score cache
3. later native scoring reuse in live/shadow contexts after packaging is proven stable

## 12.6 Phase 6: Score-to-position policy adapter

Implement a deterministic policy layer that maps scores to desired positions.

Requirements:

- threshold mapping
- delta gate
- max hold support
- cooldown support
- persistence support
- flip and long-only compatibility where needed

This layer is the bridge that lets the ML path reuse the existing validation machinery.

## 12.7 Phase 7: Validation integration

Integrate the ML strategy family into validation.

Requirements:

- vectorized validation route
- iterative validation compatibility
- cross-ticker validation compatibility
- artifact persistence
- benchmark comparability with existing deterministic strategies

Key requirement:

- avoid bolting ML onto `double_ma` or `composite_validate` in a way that obscures ownership
- prefer a distinct validation adapter that reuses shared validation primitives

Concrete backend choice:

- add `MlSignalValidationRequest`
- add `MlSignalValidationResult`
- add `run_ml_signal_validation_blocking_with_candles(...)`
- reuse `iterative_sim.rs` and shared validation statistics helpers
- reuse cross-ticker orchestration patterns from `src-tauri/src/commands_validation.rs`

## 12.8 Phase 8: Tauri/frontend integration

Expose the ML strategy path through explicit product surfaces.

Needed additions:

- request builder
- capability summary
- training/status/report controls
- validation workflow support
- artifact browsing

UI should make clear:

- this is a different strategy family
- model bundle and feature schema versions matter
- CPU-only fallback exists
- score-cache and validation artifacts are inspectable

Concrete frontend choice:

- show `MlSignal` as a strategy family with its own workspace/store
- let the workspace call `generate_ml_signal_score_cache` and `run_ml_signal_validation`
- use `historyService.ts` and `workflowArtifactsSync.ts` to surface score-cache and validation artifacts
- do not route `MlSignal` through the current optimization form unless that is a deliberate product decision

## 12.9 Phase 9: Live execution review

Do not wire live execution by default.

Only review live readiness after:

- backtest success
- walk-forward success
- ticker-holdout success
- artifact and model lifecycle are stable

If live execution is later enabled, `commands_execution.rs` will need a new strategy family branch rather than reusing the current Double-MA/Composite assumptions.

## 13. Frontend Plan

## 13.1 New strategy workspace surface

Add a new strategy section for ML signal research and validation.

Inputs:

- model bundle
- feature schema version
- horizon
- policy thresholds
- hold/cooldown/persistence settings
- long-only / allow-flip options where appropriate

Outputs:

- score summary
- validation summary
- cross-ticker summary
- calibration summary
- artifact references

## 13.2 Request-builder discipline

Follow the repo's pattern:

- strict input normalization
- explicit validation errors
- deterministic request serialization
- tests near the request builder

Relevant current patterns:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`

Additional required pattern:

- `web/scripts/generate-tauri-commands.mjs` is the source of truth for `web/src/tauri/commands.ts`
- no manual edits to `web/src/tauri/commands.ts`

## 13.3 Artifact review UX

The UI should support reviewing:

- model bundle metadata
- score-cache summaries
- validation artifacts
- walk-forward comparisons
- deterministic baseline comparisons

## 14. Verification Plan

Every phase needs explicit verification.

## 14.1 Unit tests

Add unit tests for:

- schema ordering stability
- first-valid calculations
- label generation correctness
- score-to-position mapping
- calibration application
- version mismatch rejection

## 14.2 Regression tests

Add regression tests for:

- CPU feature extraction parity across repeated runs
- policy mapping determinism
- artifact serialization/deserialization
- request-builder normalization
- stale score-cache rejection
- stale model-bundle rejection
- missing calibration-file rejection
- pattern-ID mismatch rejection

## 14.3 Validation tests

Add tests that prove:

- ML desired-position series run through vectorized validation
- iterative validation works on the same signals
- cross-ticker validation applies to ML candidates
- run artifacts preserve model/policy metadata

## 14.4 Benchmarks

Add benchmarks for:

- feature extraction throughput
- runtime scoring throughput
- score-cache generation throughput
- policy sweep over cached scores

## 14.5 Research acceptance runbook

Before promoting beyond research mode, run:

1. deterministic baseline benchmark
2. ML strategy benchmark
3. untouched future-window validation
4. ticker holdout validation
5. calibration quality check
6. threshold-stability check across rolls

## 15. Risks

## 15.1 Architecture risk

Risk:

- building a generic ML framework that does not fit the repo's strategy-local architecture

Mitigation:

- ship one concrete ML strategy family first

## 15.2 Leakage risk

Risk:

- train/calibration/test contamination

Mitigation:

- strict time splits
- embargo gaps
- independent ticker holdouts
- explicit artifact metadata for split windows

## 15.3 Runtime risk

Risk:

- model integration becomes brittle or hard to package

Mitigation:

- keep training offline
- keep runtime model loading explicit and versioned
- maintain CPU-safe fallback

## 15.4 Product risk

Risk:

- ML looks better only in-sample or in one regime

Mitigation:

- require future-window and ticker-holdout wins before promotion

## 15.5 Complexity risk

Risk:

- score-cache and policy layers grow into a second validation engine

Mitigation:

- reuse the current simulator and validation stack
- keep score-to-position mapping deterministic and thin

## 16. Recommended Implementation Order

1. Define strategy-local type contracts and schema/versioning.
2. Implement CPU feature extraction and schema-stability tests.
3. Implement fixed-horizon labels and offline dataset export.
4. Implement offline training harness and model bundle format.
5. Implement runtime model loading and scoring.
6. Implement score-to-position policy adapter.
7. Integrate ML strategy validation with vectorized and iterative engines.
8. Add score-cache artifacts and threshold-sweep tooling.
9. Add Tauri/frontend support.
10. Run walk-forward, ticker-holdout, and deterministic-baseline comparisons.
11. Review live-execution readiness only after research success.

## 17. Implementation Checklist

## 17.1 Contracts and structure

- [ ] Create `crates/app_core/src/ml_signal/` module family.
- [ ] Add request/config structs for the ML strategy family.
- [ ] Add schema/version metadata types.
- [ ] Add model-bundle metadata types.
- [ ] Add score-cache metadata types.
- [ ] Add score stream and policy config types.
- [ ] Add serialization/deserialization tests for all public structs.
- [ ] Add manifest examples that match Rust structs exactly.

## 17.2 Feature extraction

- [ ] Define v1 feature schema names and stable order.
- [ ] Persist `pattern_ids` into the feature schema instead of hard-coding a pattern count.
- [ ] Implement CPU feature extraction from `Candles`.
- [ ] Implement first-valid and warmup computation.
- [ ] Add pattern-recognition feature extraction based on active `vector-ta` pattern IDs.
- [ ] Add candle-geometry features.
- [ ] Add ALMA/ATR/ADX trend features.
- [ ] Add regime/volatility/momentum features.
- [ ] Add unit tests for shape, ordering, and NaN handling.
- [ ] Add throughput benchmark for CPU extraction.

## 17.3 Optional CUDA acceleration

- [ ] Identify which v1 features already have CUDA-backed primitives.
- [ ] Add CUDA extraction path behind existing feature flags.
- [ ] Preserve CPU fallback for every CUDA-only feature opportunity.
- [ ] Add CPU vs CUDA parity tests where practical.
- [ ] Benchmark CUDA extraction before expanding scope.

## 17.4 Labels and splits

- [ ] Implement fixed-horizon next-open label generation.
- [ ] Add configurable cost/deadband handling.
- [ ] Drop ambiguous rows according to target-specific deadband rules.
- [ ] Add time-split generator with embargo/gap support.
- [ ] Add ticker-holdout slice generator.
- [ ] Add tests for label alignment and split leakage guards.

## 17.5 Offline dataset export

- [ ] Add offline dataset export command or binary.
- [ ] Export `dataset.parquet`.
- [ ] Export `feature_schema.json`.
- [ ] Export `split_manifest.json`.
- [ ] Export `export_manifest.json`.
- [ ] Persist feature schema with every exported dataset.
- [ ] Persist split metadata with every exported dataset.
- [ ] Add reproducibility metadata for source data and timeframe.
- [ ] Add smoke test for export on a small fixture dataset.
- [ ] Add failure-mode tests for unwritable export path and missing candle fields.

## 17.6 Training harness

- [ ] Choose v1 external training runtime and pin versions.
- [ ] Add `research/ml_signal/pyproject.toml`.
- [ ] Add Python trainer entrypoint.
- [ ] Add Python calibration entrypoint.
- [ ] Add bundle assembly step.
- [ ] Train separate long and short models.
- [ ] Add calibration stage and artifact outputs.
- [ ] Emit training metrics and health flags.
- [ ] Reject unhealthy models.
- [ ] Add model-bundle directory/version format.
- [ ] Add research runbook command examples.
- [ ] Add fixture bundle for backend smoke tests.

## 17.7 Runtime scoring

- [ ] Implement model-bundle loading in Rust.
- [ ] Enforce schema/version compatibility checks.
- [ ] Implement chunked scoring.
- [ ] Expose calibrated and raw outputs.
- [ ] Add score-cache generation command.
- [ ] Add missing-model and stale-cache hard errors.
- [ ] Add runtime smoke tests with a small fixture bundle.
- [ ] Benchmark scoring throughput.

## 17.8 Policy adapter

- [ ] Implement `p_long/p_short -> target position` mapping.
- [ ] Add threshold, delta, hold, cooldown, and persistence options.
- [ ] Add long-only and flip-policy compatibility rules.
- [ ] Add unit tests for all policy edge cases.

## 17.9 Validation integration

- [ ] Add ML strategy family validation adapter.
- [ ] Add dedicated `run_ml_signal_validation` command instead of overloading existing validation commands.
- [ ] Reuse vectorized validation path on desired-position series.
- [ ] Reuse iterative validation path on the same signals.
- [ ] Add cross-ticker validation support.
- [ ] Persist model and policy metadata into run artifacts.
- [ ] Add regression tests for validation output structure.
- [ ] Add regression tests proving `MlSignal` artifacts remain loadable through history/workflow views.

## 17.10 Score-cache and threshold sweeps

- [ ] Define score-cache artifact schema.
- [ ] Implement score-cache generation.
- [ ] Implement score-cache loading/reuse.
- [ ] Add score-cache invalidation on bundle/schema/dataset mismatch.
- [ ] Add threshold/hold/exit sweeps over cached scores.
- [ ] Add benchmark comparing cached-score sweeps vs fresh inference.

## 17.11 Frontend/Tauri

- [ ] Add `src-tauri/src/commands_ml_signal.rs`.
- [ ] Add Tauri commands for model-bundle listing/status.
- [ ] Add Tauri commands for dataset export and score-cache generation.
- [ ] Add Tauri command for ML validation.
- [ ] Export new commands from `src-tauri/src/commands.rs`.
- [ ] Register new commands in `src-tauri/src/lib.rs`.
- [ ] Regenerate `web/src/tauri/commands.ts` with `npm run gen:commands`.
- [ ] Update `web/src/stores/taskStore.ts` progress-eligible command set.
- [ ] Update `web/src/stores/taskStore.ts` cancel-command mapping.
- [ ] Add frontend request builder for the ML strategy family.
- [ ] Add frontend service for model status / dataset export / score-cache / validation.
- [ ] Add strategy-specific store and workspace panel.
- [ ] Add strategy capability summary in the UI.
- [ ] Add validation workflow UI for ML strategy runs.
- [ ] Add artifact review UI for model/score-cache/validation artifacts.
- [ ] Add request-builder and service tests.
- [ ] Add history/workflow artifact parsing for `MlSignal`.

## 17.12 Research gates

- [ ] Compare ML strategy against deterministic baseline on untouched future window.
- [ ] Compare ML strategy on ticker holdout slice.
- [ ] Evaluate calibration quality across rolls.
- [ ] Evaluate threshold stability across rolls.
- [ ] Document failure cases and regimes where ML degrades.
- [ ] Keep live execution disabled until all research gates pass.

## 17.13 Documentation and operations

- [ ] Add `research/ml_signal/README.md` with end-to-end commands.
- [ ] Document model-bundle directory layout.
- [ ] Document score-cache invalidation rules.
- [ ] Document how to rotate or delete stale bundles safely.
- [ ] Document how to regenerate frontend Tauri command bindings.
- [ ] Document CPU-only fallback expectations.

## 18. Exit Criteria

This feature is ready to leave research-only status only when:

- the ML strategy family builds cleanly in CPU-only mode
- feature schema and model bundles are versioned and enforced
- the validation path is artifact-backed and test-covered
- deterministic baselines are available in the same workflows
- future-window and ticker-holdout results are acceptable
- threshold/policy behavior is stable enough to be explainable

Until then, this should remain a research-grade strategy option, not a default product path.
