# MlSignal bundle 10 — comprehensive implementation plan, code-edit guide, and tracking checklist

Date: 2026-04-01  
Scope: `MlSignal` dataset export, score-cache build/alignment, validation, policy sweep, artifact persistence/replay, and external research-bundle contract  
Primary goal: make the current `MlSignal` workflow correct, replayable, and materially faster **without** deepening the “`MlSignal` as just another optimizer strategy” bridge.

---

## Decision summary

My recommendation is:

1. **Do not deepen the current “`MlSignal` as strategyKind inside Optimize” bridge.**
   Keep it as a temporary compatibility path for now.

2. **Fix correctness and contract drift first.**
   The highest-priority fixes are:
   - calibrated score-column ingestion,
   - raw-score fallback removal,
   - ignored policy fields (`cooldown`, `signal persistence`, `allow_flip`),
   - saved-run replay/rerun correctness.

3. **Then optimize the hot path.**
   The safest backend wins are:
   - rolling-prefix feature math,
   - cheaper score-cache alignment,
   - parsed-score-cache memoization,
   - lighter artifact serialization.

4. **Persist the full ML pipeline state, not just the score-cache stage.**
   Dataset export outputs need to be carried into the run artifact, or later stages lose schema/manifests and replay becomes fragile.

5. **Add direct `MlSignal` benchmarks before doing larger refactors.**
   The current “ML” Criterion surface is not actually timing the `MlSignal` workflow.

6. **Defer the full top-level mode split until replay and artifact identity are correct.**
   I still think the separate mode is the right steady state, but I would *not* make that the first implementation step.

---

## Surface under review

This surface covers the full `MlSignal` workflow as it exists today:

- export a feature dataset from candles,
- generate a score cache from externally scored CSV data,
- validate one or more score-to-position policies,
- run bounded policy sweeps over cached scores,
- persist/reload ML-specific artifacts through history/workflow state,
- connect the app-side workflow to the offline research bundle (`train_catboost.py`, `calibrate.py`, `evaluate_walkforward.py`, `gate_report.py`).

It also includes the current shell bridge where `MlSignal` is mounted as a special `strategyKind` inside the Optimize surface.

---

## Files reviewed

### Backend / Tauri
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`

### Frontend workflow and replay
- `web/src/stores/optimizeStore.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/services/mlSignalService.ts`
- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

### Benchmark
- `crates/app_core/benches/e2e_workflow_ml.rs`

### Research harness / bundle contract
- `research/ml_signal/train_catboost.py`
- `research/ml_signal/calibrate.py`
- `research/ml_signal/evaluate_walkforward.py`
- `research/ml_signal/gate_report.py`
- `research/ml_signal/README.md`

### Context docs
- `ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`
- `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md`
- `gpt54_feature_review_map.md`

---

# 1. Surface understanding

The current `MlSignal` product path is already a distinct workflow in practice:

1. **Dataset export** happens through `export_ml_signal_dataset_with_candles(...)`.
2. **Score-cache build** happens through `build_ml_signal_score_cache_from_csv(...)`.
3. **Validation** maps cached score streams into desired positions and reuses the existing simulator path.
4. **Policy sweep** expands a Cartesian product of policy parameters, validates candidates, and truncates to `top_k`.
5. **Artifacts and replay** are routed through `mlSignalService.ts`, `workflowArtifactsStore.ts`, and `historyService.ts`.
6. **The UI bridge** mounts `MlSignalWorkspacePanel.vue` as a special case under `OptimizeWorkspacePanel.vue` when `strategyKind === "MlSignal"`.

That means the repo is already treating `MlSignal` as its own workflow **semantically**, even though some shell vocabulary still frames it as a conventional strategy option.

The most important technical observations are:

- the backend hot path is mostly in `ml_signal.rs`,
- the replay/persistence drift is mostly in `mlSignalService.ts` + `historyService.ts` + `workflowArtifactsStore.ts`,
- the external Python harness currently has a **bundle contract** with the app, but it is not yet strong enough to guarantee parity in all places,
- the current benchmark surface does **not** directly measure the real `MlSignal` workflow.

---

# 2. Findings

## Finding 1 — calibrated columns are ignored and raw fallback is unsafe

- **Severity:** critical  
- **Type:** correctness / parity  
- **Location:** `crates/app_core/src/ml_signal.rs` (`read_score_rows_csv`, `score_target`); `research/ml_signal/calibrate.py`; `research/ml_signal/evaluate_walkforward.py`  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  `calibrate.py` writes `p_long_cal` / `p_short_cal`, and `evaluate_walkforward.py` uses calibrated probabilities when calibration exists. The app-side CSV reader does **not** recognize those calibrated names and silently falls back to `p_long`, `p_short`, and eventually `raw_long`, `raw_short`. That means app-side validation can diverge from the calibrated research bundle and can accidentally apply probability thresholds to raw-margin columns.
- **Recommended change:**  
  Add explicit score-space handling:
  - prefer calibrated probability columns first,
  - then uncalibrated probability columns,
  - **reject raw-only caches in v1** instead of silently applying probability thresholds to raw margins,
  - persist `score_space` into the score-cache file/manifest and surface it into artifacts/UI.

---

## Finding 2 — app-side policy mapping ignores serialized strategy fields

- **Severity:** high  
- **Type:** correctness  
- **Location:** `crates/app_core/src/ml_signal.rs` (`build_desired_positions`); `web/src/services/mlSignalService.ts` (`buildMlStrategyConfig`)  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  The frontend sends `allow_flip`, `cooldown_bars`, and `signal_persistence_bars` through `buildMlStrategyConfig()`, and the panel explicitly tells the user that execution settings come from Strategy Controls. The current backend policy mapping uses only:
  - `trade_on_next_bar`,
  - `long_only`,
  - `max_hold_bars`,
  - `exit_on_flat`,
  - `exit_on_opposite`.

  So the user sees settings in the UI that the backend does not actually honor.
- **Recommended change:**  
  Either:
  - implement `allow_flip`, `cooldown_bars`, and `signal_persistence_bars` in `build_desired_positions`, **or**
  - remove/hide those settings from the `MlSignal` path until supported.

  I recommend implementing them now, because the settings are already being serialized and surfaced.

---

## Finding 3 — saved ML reruns do not replay the saved validation/sweep request

- **Severity:** high  
- **Type:** correctness / replay  
- **Location:** `web/src/services/historyService.ts` (`rerunSavedRun`, `applySavedRun`, `updateStoresFromRunRequest`)  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  For `MlSignal`, `rerunSavedRun()` currently rehydrates the store and then only calls `generateMlSignalScoreCache()`. It does **not** restore and rerun the saved validation request or the saved sweep request from the artifact’s `validation.req`. That means:
  - rerunning a saved validation artifact does not actually rerun validation,
  - rerunning a saved sweep artifact does not rerun the sweep,
  - replay is incomplete and user expectations are violated.
- **Recommended change:**  
  Restore the saved ML validation/sweep request into the store and rerun the full pipeline:
  - optionally rebuild the score cache,
  - then rerun single validation or policy sweep depending on the saved `validation.req` shape.

---

## Finding 4 — dataset export artifacts are not persisted, so later ML artifacts lose provenance

- **Severity:** high  
- **Type:** correctness / maintainability / replay  
- **Location:** `web/src/services/mlSignalService.ts` (`exportMlSignalDataset`, `buildMlSignalArtifactPayload`); `web/src/stores/workflowArtifactsStore.ts`  
- **Timing boundary affected:** app-path / replay  
- **Why it matters:**  
  Dataset export is the first stage of the ML workflow, but `exportMlSignalDataset()` currently does not persist a workflow artifact. That means later stages (score cache build, validation, sweep) have no reliable artifact-carried linkage to:
  - `feature_schema_path`,
  - `export_manifest_path`,
  - `split_manifest_path`,
  - exported dataset path.

  The current `ml_signal` artifact block is therefore incomplete.
- **Recommended change:**  
  Persist dataset export as an artifact-backed ML stage and carry those paths forward into later ML artifacts.

---

## Finding 5 — score-cache parse/alignment is repeatedly paid on warm follow-up actions

- **Severity:** medium  
- **Type:** performance  
- **Location:** `crates/app_core/src/ml_signal.rs` (`load_score_cache`, validation path, sweep path); `src-tauri/src/commands_ml_signal.rs`; `web/src/services/mlSignalService.ts`  
- **Timing boundary affected:** warm app-path  
- **Why it matters:**  
  Validation and sweep both reload and reparse the score-cache JSON from disk. The frontend already uses a “build cache -> validate -> sweep” flow, so reparsing the same JSON file on every follow-up action wastes time on the warm path.
- **Recommended change:**  
  Memoize the parsed score-cache file keyed by path + mtime + file length, validate manifest identity, and align from the memoized structure on warm follow-up runs.

---

## Finding 6 — export hot path rescans rolling windows on every row

- **Severity:** high  
- **Type:** performance  
- **Location:** `crates/app_core/src/ml_signal.rs` (`export_ml_signal_dataset_with_candles`)  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  The exporter recomputes:
  - `log_ret_2`,
  - `log_ret_3`,
  - `log_ret_5`,
  - `log_ret_h`,
  - `realized_vol_h`,
  - `realized_vol_3h`

  by slicing and rescanning returns for every row. This is pure repeated work.
- **Recommended change:**  
  Precompute:
  - prefix sums of returns,
  - prefix sums of squared returns,
  - rolling RMS helpers for realized vol,

  then derive each window statistic in O(1) per row.

---

## Finding 7 — export path has additional avoidable work beyond the rolling-window rescans

- **Severity:** medium  
- **Type:** performance  
- **Location:** `crates/app_core/src/ml_signal.rs` (`feature_frame`, `export_ml_signal_dataset_with_candles`, `build_ml_signal_score_cache_from_csv`)  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  Additional repeated or avoidable work includes:
  - linear `split_windows.iter().find(...)` per exported row,
  - fresh `Vec<String>` allocation per row,
  - repeated string churn (`req.data_id.clone()`, `split_id.clone()`),
  - pretty-printed score-cache JSON on the hot path,
  - hash-map-then-sort cache alignment when a cheaper aligned vector is sufficient.
- **Recommended change:**  
  - precompute split membership by row,
  - reuse the row buffer,
  - write the large score-cache JSON in compact form,
  - fill aligned score rows directly by candle index,
  - keep manifest pretty, cache compact.

---

## Finding 8 — duplicate timestamps are silently “last row wins”

- **Severity:** medium  
- **Type:** correctness / testing  
- **Location:** `crates/app_core/src/ml_signal.rs` (`build_ml_signal_score_cache_from_csv`)  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  The score-cache builder uses a `HashMap<i64, MlSignalScoreCacheRow>`, so duplicate timestamps overwrite earlier rows without explicit reporting beyond implicit last-write-wins behavior. This is fragile and obscures whether the input scorer emitted duplicate rows.
- **Recommended change:**  
  Count duplicates explicitly and persist `duplicate_timestamps` into the result and manifest. If the workflow is intentionally single-ticker-only in v1, say that explicitly in the contract instead of relying on timestamp uniqueness by convention.

---

## Finding 9 — score-cache and run artifacts do not carry enough identity metadata

- **Severity:** medium  
- **Type:** correctness / replay / maintainability  
- **Location:** `crates/app_core/src/ml_signal.rs` (score-cache manifest); `web/src/services/mlSignalService.ts`; `web/src/stores/workflowArtifactsStore.ts`; `web/src/services/historyService.ts`  
- **Timing boundary affected:** app-path / replay  
- **Why it matters:**  
  The current score-cache manifest and `ml_signal` artifact block do not persist enough ML-specific provenance to make replay robust. Missing or weak fields include:
  - score space (`probability` vs `raw_margin`),
  - duplicate timestamp count,
  - dataset export paths/manifests,
  - build action kind,
  - stronger dataset identity / validation context,
  - run-stage classification.
- **Recommended change:**  
  Extend both the manifest and the frontend artifact block with explicit ML-stage metadata.

---

## Finding 10 — frontend sweep preflight is missing

- **Severity:** medium  
- **Type:** performance / UX / correctness  
- **Location:** `web/src/services/mlSignalService.ts`; `web/src/components/MlSignalWorkspacePanel.vue`  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  The backend caps sweep expansion at 20,000 candidates, but the frontend does not estimate the Cartesian product in advance. The user only learns they exceeded the cap after invoking the command.
- **Recommended change:**  
  Add sweep-size preflight and disable the sweep button when the estimated count exceeds the backend limit.

---

## Finding 11 — the current benchmark file is not measuring the `MlSignal` workflow

- **Severity:** medium  
- **Type:** benchmark-fidelity  
- **Location:** `crates/app_core/benches/e2e_workflow_ml.rs`  
- **Timing boundary affected:** Criterion  
- **Why it matters:**  
  The current “ML” e2e benchmark is benchmarking:
  - Composite CPU optimization,
  - composite validation,
  - meta selection,
  - holdout evaluation,
  - Stage 5 ML confidence training/apply.

  It is **not** benchmarking:
  - dataset export,
  - score-cache build,
  - cache load/alignment,
  - ML validation,
  - policy sweep.
- **Recommended change:**  
  Add direct `MlSignal` Criterion coverage for the actual workflow stages.

---

## Finding 12 — the external walk-forward harness is useful, but it is not app-policy parity

- **Severity:** medium  
- **Type:** correctness / maintainability / research-contract  
- **Location:** `research/ml_signal/evaluate_walkforward.py`; `research/ml_signal/gate_report.py`; `research/ml_signal/README.md`  
- **Timing boundary affected:** research harness / unknown  
- **Why it matters:**  
  The Python walk-forward path currently applies per-row threshold logic and uses `r_long` / `r_short` label-return columns. It does **not** reproduce the desktop policy mapper or the iterative simulator semantics:
  - no `max_hold_bars` state machine,
  - no `exit_on_flat`,
  - no `exit_on_opposite` + `allow_flip` interplay,
  - no cooldown / persistence,
  - no iterative stop/take-profit handling.
- **Recommended change:**  
  Treat the Python walk-forward report as a **research proxy**, not as app-validation parity. Document that explicitly in the README and gate report. Do not claim app-policy equivalence until a parity path exists.

---

## Finding 13 — shell drift is real, but a full mode split should not be step 1

- **Severity:** medium  
- **Type:** maintainability / workflow model  
- **Location:** `web/src/stores/optimizeStore.ts`; `web/src/components/OptimizeWorkspacePanel.vue`; `web/src/components/MlSignalWorkspacePanel.vue`; `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md`  
- **Timing boundary affected:** app-path / workflow state  
- **Why it matters:**  
  `MlSignal` is already a staged alternative workflow in both copy and code path, but it still rides on `strategyKind === "MlSignal"` and shared optimize actions. That drift will continue to confuse users and contributors.
- **Recommended change:**  
  Keep the current bridge for this implementation pass, but add the metadata and replay correctness needed to make a later mode split straightforward and safe.

---

## Open questions that should be answered before merge

### Open question A — `common.py` feature reconstruction/parity
`common.py` was not attached. If `build_feature_frame(bundle)` recomputes any feature math instead of selecting exported feature columns directly, verify parity with the Rust exporter—especially the realized-vol definitions.

### Open question B — do you want raw-margin policy support in v1?
My recommendation is **no**. Keep v1 probability-only:
- calibrated probability columns preferred,
- uncalibrated probability columns acceptable,
- raw-only caches rejected.

### Open question C — should rerun always rebuild the score cache?
My recommendation is:
- **default:** rebuild the score cache during ML rerun for determinism,
- **later option:** add “reuse saved cache if identity matches” as an explicit fast-path.

---

# 3. Safe optimization opportunities

These changes are high-value and low-risk:

1. **Prefix-sum feature math** in `ml_signal.rs`.
2. **Memoized parsed score-cache JSON** on the warm validation/sweep path.
3. **Split-membership precomputation** instead of per-row window scans.
4. **Compact JSON for large score-cache files** while keeping manifest JSON pretty.
5. **Frontend sweep preflight** before invoking the backend.
6. **Artifact action-kind + dataset-export persistence** to make replay deterministic.
7. **Explicit score-space handling** and rejection of raw-only caches.

---

# 4. Higher-risk optimization opportunities

These are worthwhile, but should wait until the correctness and replay contract are stable:

1. **Batched/deduplicated indicator extraction** across `pattern_recognition`, ALMA, ADX, ATR passes.
2. **Binary/columnar score-cache format** instead of JSON.
3. **Full shell mode separation** with `MlSignal` as a top-level workflow mode.
4. **Native parity walk-forward CLI** that reuses app-side validation logic for the research harness.

---

# 5. Benchmark and perf-gate review

The current benchmark situation for this bundle is incomplete.

## What exists today
- `e2e_workflow_ml.rs` exists, but it is measuring the **meta / Stage 5 ML confidence** flow, not the `MlSignal` flow.

## What is missing
Add direct timing surfaces for:
- dataset export only,
- score-cache build only,
- validation (cold cache parse),
- validation (warm cache parse),
- sweep at multiple candidate counts,
- artifact persistence/reload cost.

## Recommendation
Do **not** do broad performance refactors until you have at least:
- one export benchmark,
- one validation cold-vs-warm benchmark,
- one sweep expansion benchmark.

---

# 6. Test review

## Existing useful coverage
There is already decent smoke coverage for:
- score-target threshold behavior,
- score-cache build,
- validation/sweep basic success,
- request payload shape,
- artifact persistence wiring.

## Highest-priority missing tests

### Backend tests
- calibrated-column preference (`p_long_cal` / `p_short_cal`),
- raw-only score-cache rejection,
- duplicate timestamp counting,
- policy mapping honoring `cooldown_bars`, `signal_persistence_bars`, and `allow_flip`,
- export prefix-math parity,
- manifest/identity validation.

### Frontend tests
- export stage artifact persistence,
- ML artifact replay restoring thresholds and sweep axes,
- ML rerun executing validation/sweep, not just cache generation,
- sweep preflight limit behavior,
- `action_kind`-based restoration instead of shape heuristics.

### Research harness tests / checks
- README examples match actual column expectations,
- gate report explicitly says “proxy / not simulator parity”,
- calibrated-score preference is documented.

---

# 7. Suggested improvement plan

## Phase 0 — lock the contract
Implement first:
- score-space enum,
- calibrated column preference,
- raw-only cache rejection,
- policy-field support (`allow_flip`, `cooldown`, `persistence`),
- artifact action kinds.

## Phase 1 — fix replay and artifact state
Implement next:
- persist dataset export artifacts,
- extend `ml_signal` artifact block,
- restore ML validation/sweep requests on apply/load/rerun,
- rerun full ML artifacts deterministically.

## Phase 2 — optimize the hot path
Implement next:
- prefix-sum exporter math,
- split-membership precomputation,
- compact score-cache JSON,
- memoized parsed score-cache loads.

## Phase 3 — add direct benchmarks
Implement next:
- direct `MlSignal` Criterion surfaces,
- cold vs warm validation evidence,
- sweep-scaling evidence.

## Phase 4 — documentation and research harness truthfulness
Implement next:
- probability-column contract in README,
- proxy-walk-forward note in README + gate report,
- implementation-plan doc updates.

## Phase 5 — revisit shell split
After Phases 0–4 land and replay is trustworthy:
- decide whether to promote `MlSignal` to a top-level mode.

---

# 8. Detailed implementation plan by file

---

## A. `crates/app_core/src/ml_signal.rs`

### Goals
1. freeze score-space semantics,
2. remove raw fallback,
3. speed up dataset export,
4. improve score-cache identity,
5. support the policy settings already being serialized by the frontend.

---

### A1. Add explicit score-space handling

#### What to change
Add a new enum and propagate it through CSV parsing, cache file serialization, manifest serialization, and cache loading.

```rust
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlSignalScoreSpace {
    Probability,
    RawMargin,
}

#[derive(Clone, Debug)]
struct ParsedScoreCsv {
    rows: Vec<ParsedScoreRow>,
    score_space: MlSignalScoreSpace,
    duplicate_timestamps: usize,
}

#[derive(Clone, Debug)]
struct LoadedScoreCache {
    score_space: MlSignalScoreSpace,
    points: Vec<Option<ScorePoint>>,
    manifest: MlSignalScoreCacheManifest,
}
```

#### Replace the current header-index helper
The current helper returns the first matching header it sees in file order, which is wrong for “prefer calibrated columns first” behavior.

```rust
fn find_header_index_preferred(headers: &StringRecord, names: &[&str]) -> Option<usize> {
    for wanted in names {
        if let Some(idx) = headers
            .iter()
            .position(|header| header.trim().eq_ignore_ascii_case(wanted))
        {
            return Some(idx);
        }
    }
    None
}
```

#### Parse calibrated columns first, then probability columns, then raw margins

```rust
fn read_score_rows_csv(input_path: &str) -> Result<ParsedScoreCsv, String> {
    let file = File::open(input_path).map_err(|e| format!("open score csv: {e}"))?;
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::new(file));
    let headers = reader
        .headers()
        .map_err(|e| format!("read score csv headers: {e}"))?
        .clone();

    let ts_idx = find_header_index_preferred(&headers, &["timestamp", "ts", "time"])
        .ok_or_else(|| "score csv requires a timestamp column".to_string())?;

    let p_long_idx = find_header_index_preferred(
        &headers,
        &["p_long_cal", "p_long", "prob_long", "long_probability"],
    );
    let p_short_idx = find_header_index_preferred(
        &headers,
        &["p_short_cal", "p_short", "prob_short", "short_probability"],
    );
    let raw_long_idx = find_header_index_preferred(
        &headers,
        &["raw_long", "score_long", "margin_long"],
    );
    let raw_short_idx = find_header_index_preferred(
        &headers,
        &["raw_short", "score_short", "margin_short"],
    );

    let score_space = if p_long_idx.is_some() || p_short_idx.is_some() {
        MlSignalScoreSpace::Probability
    } else if raw_long_idx.is_some() || raw_short_idx.is_some() {
        MlSignalScoreSpace::RawMargin
    } else {
        return Err(
            "score csv requires probability columns or raw margin columns".to_string(),
        );
    };

    let mut rows = Vec::new();
    for (line_idx, row) in reader.records().enumerate() {
        let record = row.map_err(|e| format!("read score csv row {}: {e}", line_idx + 2))?;
        let timestamp = parse_i64_cell(&record, Some(ts_idx))
            .map_err(|e| format!("score csv row {} timestamp: {e}", line_idx + 2))?
            .ok_or_else(|| format!("score csv row {} missing timestamp", line_idx + 2))?;

        let p_long = parse_f64_cell(&record, p_long_idx)
            .map_err(|e| format!("score csv row {} p_long: {e}", line_idx + 2))?;
        let p_short = parse_f64_cell(&record, p_short_idx)
            .map_err(|e| format!("score csv row {} p_short: {e}", line_idx + 2))?;
        let raw_long = parse_f64_cell(&record, raw_long_idx)
            .map_err(|e| format!("score csv row {} raw_long: {e}", line_idx + 2))?;
        let raw_short = parse_f64_cell(&record, raw_short_idx)
            .map_err(|e| format!("score csv row {} raw_short: {e}", line_idx + 2))?;

        if p_long.is_none() && p_short.is_none() && raw_long.is_none() && raw_short.is_none() {
            continue;
        }

        rows.push(ParsedScoreRow {
            timestamp,
            p_long,
            p_short,
            raw_long,
            raw_short,
        });
    }

    Ok(ParsedScoreCsv {
        rows,
        score_space,
        duplicate_timestamps: 0,
    })
}
```

#### Reject raw-only caches in v1 validation/sweep
This is safer than silently using raw margins with probability thresholds.

```rust
fn score_target(point: Option<ScorePoint>, params: &MlSignalPolicyParams) -> Option<i32> {
    let point = point?;
    let p_long = point.p_long.filter(|v| v.is_finite())?;
    let p_short = point.p_short.filter(|v| v.is_finite())?;

    if p_long >= params.t_long && (p_long - p_short) >= params.delta {
        return Some(1);
    }
    if !params.strategy.long_only
        && p_short >= params.t_short
        && (p_short - p_long) >= params.delta
    {
        return Some(-1);
    }
    Some(0)
}
```

And in the loader/validator:

```rust
pub fn run_ml_signal_validation_blocking_with_candles(
    req: MlSignalValidationRequest,
    candles: &Candles,
    cancel: &AtomicBool,
) -> Result<MlSignalValidationResult, String> {
    let loaded = load_score_cache(&req.score_cache_path, &req.data_id, candles)?;
    if loaded.score_space != MlSignalScoreSpace::Probability {
        return Err(
            "raw-margin-only score caches are not supported by MlSignal probability policies; provide p_long/p_short or calibrated probability columns"
                .to_string(),
        );
    }

    validate_ml_signal_candidates(
        req.data_id,
        req.score_cache_path,
        &loaded.points,
        req.candidates,
        candles,
        &req.iterative,
        req.min_psr,
        req.min_dsr,
        cancel,
    )
}
```

---

### A2. Count duplicates explicitly during score-cache build

#### What to change
Replace the timestamp->row `HashMap` with an aligned vector keyed by candle index.

```rust
pub fn build_ml_signal_score_cache_from_csv(
    req: MlSignalScoreCacheBuildRequest,
    candles: &Candles,
    cancel: &AtomicBool,
) -> Result<MlSignalScoreCacheBuildResult, String> {
    let parsed = read_score_rows_csv(&req.input_path)?;
    let timestamp_to_index: HashMap<i64, usize> = candles
        .timestamp
        .iter()
        .enumerate()
        .map(|(idx, ts)| (*ts, idx))
        .collect();

    let mut aligned: Vec<Option<MlSignalScoreCacheRow>> = vec![None; candles.timestamp.len()];
    let mut skipped_rows = 0usize;
    let mut duplicate_timestamps = 0usize;

    for row in parsed.rows {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let Some(&idx) = timestamp_to_index.get(&row.timestamp) else {
            skipped_rows = skipped_rows.saturating_add(1);
            continue;
        };
        if aligned[idx].is_some() {
            duplicate_timestamps = duplicate_timestamps.saturating_add(1);
        }
        aligned[idx] = Some(MlSignalScoreCacheRow {
            timestamp: row.timestamp,
            p_long: row.p_long,
            p_short: row.p_short,
            raw_long: row.raw_long,
            raw_short: row.raw_short,
        });
    }

    let scores: Vec<MlSignalScoreCacheRow> = aligned.into_iter().flatten().collect();

    // ...persist score_space + duplicate_timestamps into file and manifest...
}
```

#### Extend the manifest/result structs

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalScoreCacheManifest {
    pub version: u32,
    pub kind: String,
    pub strategy_kind: String,
    pub data_id: String,
    pub input_path: String,
    pub feature_schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_schema_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_bundle_id: Option<String>,
    pub matched_rows: usize,
    pub skipped_rows: usize,
    pub duplicate_timestamps: usize,
    pub bars_scored: usize,
    pub score_space: MlSignalScoreSpace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_context: Option<ValidationArtifactContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_timestamp: Option<i64>,
}
```

```rust
#[derive(Clone, Debug, Serialize)]
pub struct MlSignalScoreCacheBuildResult {
    pub data_id: String,
    pub input_path: String,
    pub output_path: String,
    pub manifest_path: String,
    pub matched_rows: usize,
    pub skipped_rows: usize,
    pub duplicate_timestamps: usize,
    pub score_space: MlSignalScoreSpace,
    pub model_bundle_id: Option<String>,
    pub version: u32,
}
```

---

### A3. Memoize parsed score-cache files on the warm path

This is one of the most valuable warm-path wins.

```rust
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ScoreCacheFileMemoKey {
    path: String,
    file_len: u64,
    modified_ms: u128,
}

static SCORE_CACHE_FILE_MEMO: OnceLock<RwLock<HashMap<ScoreCacheFileMemoKey, Arc<MlSignalScoreCacheFile>>>> =
    OnceLock::new();

fn score_cache_memo() -> &'static RwLock<HashMap<ScoreCacheFileMemoKey, Arc<MlSignalScoreCacheFile>>> {
    SCORE_CACHE_FILE_MEMO.get_or_init(|| RwLock::new(HashMap::new()))
}

fn score_cache_file_key(path: &str) -> Result<ScoreCacheFileMemoKey, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("stat score cache: {e}"))?;
    let modified = metadata
        .modified()
        .map_err(|e| format!("read score cache mtime: {e}"))?
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("score cache mtime before epoch: {e}"))?
        .as_millis();

    Ok(ScoreCacheFileMemoKey {
        path: path.trim().to_string(),
        file_len: metadata.len(),
        modified_ms: modified,
    })
}

fn read_score_cache_file(path: &str) -> Result<Arc<MlSignalScoreCacheFile>, String> {
    let key = score_cache_file_key(path)?;
    if let Some(hit) = score_cache_memo()
        .read()
        .map_err(|_| "score cache memo poisoned".to_string())?
        .get(&key)
        .cloned()
    {
        return Ok(hit);
    }

    let file = File::open(path).map_err(|e| format!("open score cache: {e}"))?;
    let parsed: MlSignalScoreCacheFile = serde_json::from_reader(BufReader::new(file))
        .map_err(|e| format!("parse score cache json: {e}"))?;
    let parsed = Arc::new(parsed);

    score_cache_memo()
        .write()
        .map_err(|_| "score cache memo poisoned".to_string())?
        .insert(key, parsed.clone());

    Ok(parsed)
}
```

#### Align with a merge walk instead of a hash map
Because both candle timestamps and persisted scores are sorted.

```rust
fn align_score_points(
    cache: &MlSignalScoreCacheFile,
    candles: &Candles,
) -> Vec<Option<ScorePoint>> {
    let mut out = vec![None; candles.timestamp.len()];
    let mut score_idx = 0usize;

    for (bar_idx, ts) in candles.timestamp.iter().copied().enumerate() {
        while score_idx < cache.scores.len() && cache.scores[score_idx].timestamp < ts {
            score_idx = score_idx.saturating_add(1);
        }
        if score_idx < cache.scores.len() && cache.scores[score_idx].timestamp == ts {
            let row = &cache.scores[score_idx];
            out[bar_idx] = Some(ScorePoint {
                p_long: row.p_long,
                p_short: row.p_short,
                raw_long: row.raw_long,
                raw_short: row.raw_short,
            });
        }
    }

    out
}
```

---

### A4. Replace repeated rolling rescans with prefix-sum helpers

#### New helpers

```rust
fn prefix_sum(values: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; values.len() + 1];
    for (idx, value) in values.iter().copied().enumerate() {
        out[idx + 1] = out[idx] + value;
    }
    out
}

fn prefix_sum_sq(values: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; values.len() + 1];
    for (idx, value) in values.iter().copied().enumerate() {
        out[idx + 1] = out[idx] + value * value;
    }
    out
}

fn trailing_sum(prefix: &[f64], end_exclusive: usize, len: usize) -> f64 {
    if end_exclusive == 0 || len == 0 {
        return 0.0;
    }
    let end = end_exclusive.min(prefix.len().saturating_sub(1));
    let start = end.saturating_sub(len);
    prefix[end] - prefix[start]
}

fn trailing_rms(prefix_sq: &[f64], end_exclusive: usize, len: usize) -> f64 {
    if end_exclusive == 0 || len == 0 {
        return 0.0;
    }
    let end = end_exclusive.min(prefix_sq.len().saturating_sub(1));
    let start = end.saturating_sub(len);
    let n = end.saturating_sub(start);
    if n == 0 {
        return 0.0;
    }
    ((prefix_sq[end] - prefix_sq[start]) / n as f64).sqrt()
}
```

#### Use them in the export loop

```rust
let log_returns = log_returns_from_close(&candles.close);
let log_ret_prefix = prefix_sum(&log_returns);
let log_ret_sq_prefix = prefix_sum_sq(&log_returns);

for t in frame.first_valid_bar..=last_label_bar {
    let end = t + 1;

    let log_ret_1 = log_returns[t];
    let log_ret_2 = trailing_sum(&log_ret_prefix, end, 2);
    let log_ret_3 = trailing_sum(&log_ret_prefix, end, 3);
    let log_ret_5 = trailing_sum(&log_ret_prefix, end, 5);
    let log_ret_h = trailing_sum(&log_ret_prefix, end, h);

    let realized_vol_h = trailing_rms(&log_ret_sq_prefix, end, h);
    let realized_vol_3h = trailing_rms(&log_ret_sq_prefix, end, h.saturating_mul(3));

    // ...
}
```

> Note: this also aligns the implementation with the March 18 feature-spec language for realized vol as RMS of log returns. If `common.py` recomputes features, verify it matches.

---

### A5. Precompute split membership and reuse the row buffer

#### Precompute split-membership once

```rust
fn build_split_membership(
    last_row_inclusive: usize,
    split_windows: &[MlSignalSplitWindow],
) -> Vec<usize> {
    let mut out = vec![usize::MAX; last_row_inclusive + 1];
    for (idx, window) in split_windows.iter().enumerate() {
        let start = window.train_start.min(out.len());
        let end = window.test_end.min(out.len());
        for slot in &mut out[start..end] {
            *slot = idx;
        }
    }
    out
}
```

#### Reuse the row buffer
Move the record allocation outside the row loop.

```rust
let split_membership = build_split_membership(last_label_bar, &split_windows);
let mut record: Vec<String> = Vec::with_capacity(4 + feature_names.len() + 6);

for t in frame.first_valid_bar..=last_label_bar {
    record.clear();

    let split_idx = split_membership[t];
    let split_id = if split_idx != usize::MAX {
        split_windows[split_idx].split_id.as_str()
    } else {
        ""
    };

    record.push(candles.timestamp[t].to_string());
    record.push(req.data_id.clone());
    record.push(split_id.to_string());
    record.push("1.0".to_string());

    // ...append features...

    writer
        .write_record(record.iter().map(String::as_str))
        .map_err(|e| format!("write dataset row: {e}"))?;
}
```

---

### A6. Honor `allow_flip`, `cooldown_bars`, and `signal_persistence_bars`

This is the minimum backend policy logic I would implement to match the frontend contract.

```rust
fn build_desired_positions(
    score_points: &[Option<ScorePoint>],
    params: &MlSignalPolicyParams,
) -> Vec<i32> {
    let n = score_points.len();
    let mut requested = vec![None; n];

    for (idx, point) in score_points.iter().copied().enumerate() {
        let target = score_target(point, params);
        let apply_idx = if params.strategy.trade_on_next_bar {
            idx.saturating_add(1)
        } else {
            idx
        };
        if apply_idx < n {
            requested[apply_idx] = target;
        }
    }

    let persistence_bars = params.strategy.signal_persistence_bars.max(1);
    let cooldown_bars = params.strategy.cooldown_bars;
    let allow_flip = params.strategy.allow_flip;

    let mut desired = vec![0i32; n];
    let mut current = 0i32;
    let mut held_bars = 0usize;
    let mut cooldown_remaining = 0usize;

    let mut candidate_target = 0i32;
    let mut candidate_streak = 0usize;

    for bar in 0..n {
        if current != 0 && params.max_hold_bars > 0 && held_bars >= params.max_hold_bars {
            current = 0;
            held_bars = 0;
            cooldown_remaining = cooldown_bars;
        }

        let incoming = requested[bar].flatten().unwrap_or(0);

        if incoming == candidate_target {
            candidate_streak = candidate_streak.saturating_add(1);
        } else {
            candidate_target = incoming;
            candidate_streak = 1;
        }

        let persisted_target = if candidate_streak >= persistence_bars {
            candidate_target
        } else {
            0
        };

        if current == 0 && cooldown_remaining > 0 {
            cooldown_remaining = cooldown_remaining.saturating_sub(1);
            desired[bar] = 0;
            continue;
        }

        if current == 0 {
            if persisted_target != 0 {
                current = persisted_target;
                held_bars = 0;
            }
        } else if persisted_target == 0 {
            if params.exit_on_flat {
                current = 0;
                held_bars = 0;
                cooldown_remaining = cooldown_bars;
            }
        } else if persisted_target != current && params.exit_on_opposite {
            if allow_flip {
                current = persisted_target;
                held_bars = 0;
            } else {
                current = 0;
                held_bars = 0;
                cooldown_remaining = cooldown_bars;
            }
        }

        desired[bar] = current;
        if current != 0 {
            held_bars = held_bars.saturating_add(1);
        }
    }

    desired
}
```

---

### A7. Write the large score-cache JSON in compact form

```rust
fn write_json_file_compact<T: Serialize>(path: &str, value: &T, context: &str) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let file = File::create(path).map_err(|e| format!("create {context}: {e}"))?;
    serde_json::to_writer(BufWriter::new(file), value)
        .map_err(|e| format!("write {context}: {e}"))
}
```

Use:
- `write_json_file(...)` for manifests (pretty),
- `write_json_file_compact(...)` for the score-cache payload file.

---

### A8. Add backend tests

Add at least these tests to `ml_signal.rs`:

1. `score_csv_prefers_calibrated_probability_columns`
2. `raw_only_score_csv_is_rejected_for_validation`
3. `duplicate_timestamps_are_counted`
4. `desired_positions_respect_cooldown_persistence_and_allow_flip`
5. `score_cache_manifest_persists_score_space`
6. `rolling_prefix_math_matches_previous_export_values`

Example skeleton:

```rust
#[test]
fn score_csv_prefers_calibrated_probability_columns() {
    // write csv with p_long, p_short, p_long_cal, p_short_cal
    // assert cache manifest score_space == Probability
    // assert parsed p_long/p_short match calibrated columns
}
```

---

## B. `src-tauri/src/commands_ml_signal.rs`

### Goals
1. validate context earlier,
2. propagate stronger ML score-cache identity,
3. keep Tauri command semantics explicit.

---

### B1. Extend score-cache build request with validation context

The build request should carry the same dataset identity context that validation/sweep already carry.

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalScoreCacheBuildRequest {
    pub data_id: String,
    pub input_path: String,
    pub output_path: String,
    #[serde(default)]
    pub model_bundle_id: Option<String>,
    #[serde(default)]
    pub validation_context: Option<ValidationArtifactContext>,
}
```

### B2. Validate build context against the currently loaded dataset

```rust
#[tauri::command]
pub async fn generate_ml_signal_score_cache(
    req: MlSignalScoreCacheBuildRequest,
    state: tauri::State<'_, AppState>,
) -> Result<MlSignalScoreCacheBuildResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let loaded = state.get_loaded_data_context(&req.data_id)?;
    validate_ml_signal_context(req.validation_context.as_ref(), &loaded)?;
    let candles = state.get_candles(&req.data_id)?;

    tauri::async_runtime::spawn_blocking(move || {
        build_ml_signal_score_cache_from_csv(req, &candles, cancel.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

### B3. Keep model-status command as-is, but document it as metadata-only
Do **not** make model-status checks block score-cache validation in this pass. Keep it advisory unless you later add native bundle scoring.

---

## C. `web/src/services/mlSignalService.ts`

### Goals
1. persist the dataset export stage,
2. enrich ML artifact metadata,
3. preflight sweeps,
4. remove ambiguous artifact payloads.

---

### C1. Add ML artifact action kinds

Add an action kind that lives in the persisted artifact request, not necessarily in the backend request.

```ts
type MlSignalArtifactActionKind =
  | "dataset_export"
  | "score_cache_build"
  | "validation"
  | "policy_sweep";
```

### C2. Persist the dataset export stage
Right now dataset export updates UI text only. It should persist a run artifact.

```ts
export async function exportMlSignalDataset(): Promise<unknown> {
  const pinia = resolveRuntimePinia();
  if (!pinia) throw new Error("Pinia is not active.");

  const optimize = useOptimizeStore(pinia);
  const data = requireDataContext();
  const outputPath = normalizeText(optimize.mlSignalDatasetExportPath);
  if (!outputPath) throw new Error("Choose a dataset export CSV path first.");

  const req = {
    data_id: data.dataId,
    output_path: outputPath,
    ma_source: normalizeText(optimize.mlSignalMaSource) || "close",
    horizon_bars: Math.max(2, Math.floor(parseNumber(optimize.mlSignalHorizonBars, 10))),
    round_trip_cost_bps: currentRoundTripCostBps(),
    split_plan: /* existing logic */,
    ticker_holdout: /* existing logic */,
  };

  const artifactReq = {
    ...req,
    strategy_kind: "MlSignal",
    action_kind: "dataset_export" as const,
  };

  const runTs = new Date().toISOString();

  setMlBusy("Exporting ML feature dataset…");
  try {
    const res = await invokeCommand("export_ml_signal_dataset", { req });

    const persisted = await persistRunArtifact(artifactReq, res, runTs);
    applyVueRunArtifacts(runTs, artifactReq, res, persisted.path, persisted.csvPath);

    patchOptimizeState({
      mlSignalDatasetMeta: [
        `Dataset export: ${parseNumber((res as Record<string, unknown>)?.row_count, 0)} rows -> ${outputPath}`,
        basename(normalizeText((res as Record<string, unknown>)?.feature_schema_path)),
        basename(normalizeText((res as Record<string, unknown>)?.export_manifest_path)),
      ]
        .filter(Boolean)
        .join(" • "),
      statusLine: "ML feature dataset exported.",
    });

    return res;
  } finally {
    clearMlBusy();
  }
}
```

### C3. Merge previous ML artifact metadata forward
When score-cache build happens after dataset export, carry forward the dataset export fields.

```ts
function currentMlArtifactSnapshot(): Record<string, unknown> | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const artifacts = useWorkflowArtifactsStore(pinia);
  return artifacts.currentRun?.mlSignal
    ? (artifacts.currentRun.mlSignal as unknown as Record<string, unknown>)
    : null;
}

function buildMlSignalArtifactPayload(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq?: Record<string, unknown> | null,
  validationRes?: unknown,
): Record<string, unknown> {
  const runResRecord =
    runRes && typeof runRes === "object" ? (runRes as Record<string, unknown>) : null;
  const prev = currentMlArtifactSnapshot();

  return {
    action_kind: normalizeText(runReq.action_kind) || normalizeText(prev?.action_kind),
    dataset_path:
      normalizeText(runReq.output_path) && normalizeText(runReq.action_kind) === "dataset_export"
        ? normalizeText(runReq.output_path)
        : normalizeText(prev?.dataset_path),
    export_manifest_path:
      normalizeText(runResRecord?.export_manifest_path) || normalizeText(prev?.export_manifest_path),
    split_manifest_path:
      normalizeText(runResRecord?.split_manifest_path) || normalizeText(prev?.split_manifest_path),
    bundle_id:
      normalizeText(runReq.model_bundle_id) ||
      normalizeText(runResRecord?.model_bundle_id) ||
      normalizeText(prev?.bundle_id),
    feature_schema_path:
      normalizeText(runResRecord?.feature_schema_path) || normalizeText(prev?.feature_schema_path),
    feature_schema_version:
      normalizeText(runResRecord?.feature_schema_version ?? prev?.feature_schema_version),
    score_cache_path:
      normalizeText(validationReq?.score_cache_path) ||
      normalizeText(runResRecord?.output_path) ||
      normalizeText(prev?.score_cache_path),
    score_manifest_path:
      normalizeText(runResRecord?.manifest_path) || normalizeText(prev?.score_manifest_path),
    score_space:
      normalizeText(runResRecord?.score_space) || normalizeText(prev?.score_space),
    duplicate_timestamps:
      normalizeText(runResRecord?.duplicate_timestamps ?? prev?.duplicate_timestamps),
    policy:
      validationReq && validationRes ? extractMlPolicy(validationReq, validationRes) : prev?.policy ?? null,
    policy_sweep:
      validationReq && validationRes
        ? extractMlPolicySweepSummary(validationRes)
        : prev?.policy_sweep ?? null,
  };
}
```

### C4. Add sweep preflight
Block overly large sweeps before invoking the backend.

```ts
function estimateAxisCount(
  startRaw: unknown,
  endRaw: unknown,
  stepRaw: unknown,
): number {
  const start = Number(startRaw);
  const end = Number(endRaw);
  const step = Number(stepRaw);
  if (!Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(step)) return 1;
  if (step <= 0 || end < start) return 1;
  return Math.floor((end - start) / step + 1e-9) + 1;
}

function estimateMlSignalSweepSize(optimize: ReturnType<typeof useOptimizeStore>): number {
  const longCount = estimateAxisCount(
    optimize.mlSignalSweepLongStart,
    optimize.mlSignalSweepLongEnd,
    optimize.mlSignalSweepLongStep,
  );
  const shortCount = estimateAxisCount(
    optimize.mlSignalSweepShortStart,
    optimize.mlSignalSweepShortEnd,
    optimize.mlSignalSweepShortStep,
  );
  const deltaCount = estimateAxisCount(
    optimize.mlSignalSweepDeltaStart,
    optimize.mlSignalSweepDeltaEnd,
    optimize.mlSignalSweepDeltaStep,
  );
  const holdCount = estimateAxisCount(
    optimize.mlSignalSweepHoldStart,
    optimize.mlSignalSweepHoldEnd,
    optimize.mlSignalSweepHoldStep,
  );
  const flatCount = 1;
  const oppositeCount = 1;
  return longCount * shortCount * deltaCount * holdCount * flatCount * oppositeCount;
}
```

Use it in `runMlSignalPolicySweep()`:

```ts
const estimatedSweepSize = estimateMlSignalSweepSize(optimize);
if (estimatedSweepSize > 20_000) {
  patchOptimizeState({
    mlSignalSweepMeta: `ML policy sweep: estimated ${estimatedSweepSize} candidates exceeds the 20,000 backend cap`,
    statusLine: "ML policy sweep blocked: estimated candidate count is too large.",
  });
  throw new Error(
    `Estimated ML policy sweep size ${estimatedSweepSize} exceeds the 20,000 backend cap.`,
  );
}
```

### C5. Remove extra unknown fields from backend requests
Today `generateMlSignalScoreCache()` includes `strategy_kind: "MlSignal"` in the backend request even though the request struct does not use it. Keep that only in artifact metadata, not the invoke payload.

Also pass `validation_context: buildValidationContext()` on score-cache build so the cache manifest is tied to the same dataset/timeframe identity that validation and sweep already use.

---

## D. `web/src/services/mlSignalService.test.ts`

### Goals
1. cover dataset export artifact persistence,
2. cover sweep preflight blocking,
3. cover richer ML artifact payload persistence.

### Add tests

#### D1. dataset export persists artifact state
Verify `save_run_artifact` is called from `exportMlSignalDataset()`.

#### D2. sweep preflight blocks oversized expansion
Seed very large axes and assert that:
- `invokeCommand("run_ml_signal_policy_sweep", ...)` is **not** called,
- `mlSignalSweepMeta` contains the cap warning.

#### D3. score-cache build persists score-space and duplicates when returned
If you extend the backend response, assert those fields are forwarded into the artifact block and UI meta.

Example skeleton:

```ts
it("blocks oversized ML policy sweeps before backend invocation", async () => {
  seedStores();
  const optimize = useOptimizeStore(pinia);
  optimize.mlSignalSweepLongStart = "0.10";
  optimize.mlSignalSweepLongEnd = "0.90";
  optimize.mlSignalSweepLongStep = "0.01";
  optimize.mlSignalSweepShortStart = "0.10";
  optimize.mlSignalSweepShortEnd = "0.90";
  optimize.mlSignalSweepShortStep = "0.01";
  optimize.mlSignalSweepDeltaStart = "0.00";
  optimize.mlSignalSweepDeltaEnd = "0.50";
  optimize.mlSignalSweepDeltaStep = "0.01";
  optimize.mlSignalSweepHoldStart = "1";
  optimize.mlSignalSweepHoldEnd = "50";
  optimize.mlSignalSweepHoldStep = "1";

  await expect(runMlSignalPolicySweep()).rejects.toThrow(/exceeds the 20,000 backend cap/i);
  expect(invokeCommandMock).not.toHaveBeenCalledWith(
    "run_ml_signal_policy_sweep",
    expect.anything(),
  );
});
```

---

## E. `web/src/stores/workflowArtifactsStore.ts`

### Goals
1. persist more ML-stage metadata,
2. make restoration deterministic,
3. stop relying on shape heuristics later.

### Extend the ML artifact interface

```ts
export interface WorkflowMlSignalArtifact {
  actionKind: string;
  datasetPath: string;
  exportManifestPath: string;
  splitManifestPath: string;
  bundleId: string;
  scoreCachePath: string;
  scoreManifestPath: string;
  scoreSpace: string;
  duplicateTimestamps: string;
  featureSchemaPath: string;
  featureSchemaVersion: string;
  policy: Record<string, unknown> | null;
  policySweep: Record<string, unknown> | null;
}
```

### Update the normalizer

```ts
function normalizeMlSignalArtifact(
  value: unknown,
): WorkflowMlSignalArtifact | null {
  if (!value || typeof value !== "object") return null;
  const row = value as Record<string, unknown>;

  const out: WorkflowMlSignalArtifact = {
    actionKind: normalizeText(row.action_kind ?? row.actionKind),
    datasetPath: normalizeText(row.dataset_path ?? row.datasetPath),
    exportManifestPath: normalizeText(row.export_manifest_path ?? row.exportManifestPath),
    splitManifestPath: normalizeText(row.split_manifest_path ?? row.splitManifestPath),
    bundleId: normalizeText(row.bundle_id ?? row.bundleId),
    scoreCachePath: normalizeText(row.score_cache_path ?? row.scoreCachePath),
    scoreManifestPath: normalizeText(row.score_manifest_path ?? row.scoreManifestPath),
    scoreSpace: normalizeText(row.score_space ?? row.scoreSpace),
    duplicateTimestamps: normalizeText(row.duplicate_timestamps ?? row.duplicateTimestamps),
    featureSchemaPath: normalizeText(row.feature_schema_path ?? row.featureSchemaPath),
    featureSchemaVersion: normalizeText(row.feature_schema_version ?? row.featureSchemaVersion),
    policy:
      row.policy && typeof row.policy === "object"
        ? (row.policy as Record<string, unknown>)
        : null,
    policySweep:
      row.policy_sweep && typeof row.policy_sweep === "object"
        ? (row.policy_sweep as Record<string, unknown>)
        : row.policySweep && typeof row.policySweep === "object"
          ? (row.policySweep as Record<string, unknown>)
          : null,
  };

  if (
    !out.actionKind &&
    !out.datasetPath &&
    !out.exportManifestPath &&
    !out.splitManifestPath &&
    !out.bundleId &&
    !out.scoreCachePath &&
    !out.scoreManifestPath &&
    !out.scoreSpace &&
    !out.duplicateTimestamps &&
    !out.featureSchemaPath &&
    !out.featureSchemaVersion &&
    !out.policy &&
    !out.policySweep
  ) {
    return null;
  }

  return markRaw(out);
}
```

---

## F. `web/src/services/historyService.ts`

### Goals
1. restore ML validation/sweep inputs correctly,
2. rerun the full ML artifact pipeline,
3. stop using request-shape heuristics where action-kind metadata exists,
4. prepare for a later shell mode split.

---

### F1. Add helpers to restore ML validation request state

```ts
function applyMlSignalValidationReqToStore(req: Record<string, unknown>): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);

  const candidates = Array.isArray(req.candidates)
    ? (req.candidates as Array<Record<string, unknown>>)
    : [];
  const first = candidates[0] && typeof candidates[0] === "object"
    ? (candidates[0] as Record<string, unknown>)
    : null;

  if (normalizeText(req.score_cache_path)) {
    optimize.mlSignalScoreCachePath = normalizeText(req.score_cache_path);
  }

  if (first) {
    optimize.mlSignalLongThreshold = normalizeText(first.t_long);
    optimize.mlSignalShortThreshold = normalizeText(first.t_short);
    optimize.mlSignalDelta = normalizeText(first.delta);
    optimize.mlSignalMaxHoldBars = normalizeText(first.max_hold_bars);
    optimize.mlSignalExitOnFlat = Boolean(first.exit_on_flat);
    optimize.mlSignalExitOnOpposite = Boolean(first.exit_on_opposite);
  }

  if (req.min_psr != null) optimize.mlSignalMinPsr = normalizeText(req.min_psr);
  if (req.min_dsr != null) optimize.mlSignalMinDsr = normalizeText(req.min_dsr);

  optimize.strategyKind = "MlSignal";
  optimize.source = "vue";
  optimize.updatedAtUnixMs = Date.now();
}

function applyMlSignalSweepReqToStore(req: Record<string, unknown>): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);

  const axis = (value: unknown): Record<string, unknown> | null =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : null;

  const tLong = axis(req.t_long);
  const tShort = axis(req.t_short);
  const delta = axis(req.delta);
  const hold = axis(req.max_hold_bars);

  if (normalizeText(req.score_cache_path)) {
    optimize.mlSignalScoreCachePath = normalizeText(req.score_cache_path);
  }

  if (tLong) {
    optimize.mlSignalSweepLongStart = normalizeText(tLong.start);
    optimize.mlSignalSweepLongEnd = normalizeText(tLong.end);
    optimize.mlSignalSweepLongStep = normalizeText(tLong.step);
  }
  if (tShort) {
    optimize.mlSignalSweepShortStart = normalizeText(tShort.start);
    optimize.mlSignalSweepShortEnd = normalizeText(tShort.end);
    optimize.mlSignalSweepShortStep = normalizeText(tShort.step);
  }
  if (delta) {
    optimize.mlSignalSweepDeltaStart = normalizeText(delta.start);
    optimize.mlSignalSweepDeltaEnd = normalizeText(delta.end);
    optimize.mlSignalSweepDeltaStep = normalizeText(delta.step);
  }
  if (hold) {
    optimize.mlSignalSweepHoldStart = normalizeText(hold.start);
    optimize.mlSignalSweepHoldEnd = normalizeText(hold.end);
    optimize.mlSignalSweepHoldStep = normalizeText(hold.step);
  }

  if (req.top_k != null) optimize.mlSignalSweepTopK = normalizeText(req.top_k);
  if (req.min_psr != null) optimize.mlSignalMinPsr = normalizeText(req.min_psr);
  if (req.min_dsr != null) optimize.mlSignalMinDsr = normalizeText(req.min_dsr);

  optimize.strategyKind = "MlSignal";
  optimize.source = "vue";
  optimize.updatedAtUnixMs = Date.now();
}

function isMlSignalSweepRequest(req: Record<string, unknown> | null): boolean {
  return !!req && !Array.isArray(req.candidates) && (
    "t_long" in req ||
    "t_short" in req ||
    "delta" in req ||
    "max_hold_bars" in req
  );
}
```

### F2. Stop relying only on shape heuristics in `updateStoresFromRunRequest`
If the artifact has `ml_signal.action_kind`, use that first.

```ts
const actionKind = normalizeText(asRecord(record.ml_signal)?.action_kind);
const hasDatasetExportShape =
  actionKind === "dataset_export" ||
  (
    !actionKind &&
    (normalizeText(req.ma_source ?? req.maSource).length > 0 ||
      Number.isFinite(Number(req.horizon_bars ?? req.horizonBars)))
  );
```

### F3. Rerun the full saved ML artifact
This is the key replay fix.

```ts
export async function rerunSavedRun(path: string): Promise<void> {
  const loaded = await loadRunArtifact(path);
  const record = loaded.record;
  const runRecord = asRecord(record.run);
  const req = asRecord(runRecord?.req);
  if (!req) throw new Error("Saved run artifact is missing run.req.");

  const validationRecord = asRecord(record.validation);
  const validationReq = asRecord(validationRecord?.req);

  const csvPath =
    normalizeText(record.csv_path ?? record.csvPath) ||
    normalizeText(asRecord(record.data)?.csv_path);

  if (!csvPath) {
    throw new Error(
      "Saved ML run is missing data.csv_path. Load a dataset first, then rerun.",
    );
  }

  const base = await loadDataCsv(csvPath);
  let currentDataId = normalizeText(base.dataId);
  const timeframe = normalizeText(asRecord(record.dataset_meta)?.timeframe);

  if (timeframe) {
    const baseTf = normalizeText(base.meta.timeframe);
    if (!baseTf || baseTf !== timeframe) {
      const resampled = await applyDataTimeframe(timeframe, currentDataId);
      currentDataId = normalizeText(resampled.dataId) || currentDataId;
    }
  }

  updateStoresFromRunRequest(req, csvPath, timeframe);

  if (isMlSignalStrategyKind(req.strategy_kind ?? req.strategyKind)) {
    const pinia = resolveRuntimePinia();
    if (!pinia) throw new Error("Pinia is not active.");

    const optimizeStore = useOptimizeStore(pinia);
    optimizeStore.strategyKind = "MlSignal";

    const { generateMlSignalScoreCache, runMlSignalValidation, runMlSignalPolicySweep } =
      await import("./mlSignalService");

    if (validationReq) {
      if (isMlSignalSweepRequest(validationReq)) {
        applyMlSignalSweepReqToStore(validationReq);
      } else {
        applyMlSignalValidationReqToStore(validationReq);
      }
    }

    // Rebuild cache by default for deterministic reruns.
    await generateMlSignalScoreCache();

    if (validationReq) {
      if (isMlSignalSweepRequest(validationReq)) {
        await runMlSignalPolicySweep();
      } else {
        await runMlSignalValidation();
      }
    }

    return;
  }

  await runOptimization({ request: { ...req, data_id: currentDataId }, switchTab: true });
}
```

### F4. When loading/applying an ML artifact, set `strategyKind = "MlSignal"`
This keeps the current bridge usable until you do the later mode split.

---

## G. `web/src/stores/optimizeStore.ts`

### Goals
1. make the ML bridge explicit,
2. reduce string-literal drift,
3. avoid deepening the fake “MlSignal is a normal optimizer strategy” abstraction.

### G1. Centralize the strategy-kind check

```ts
function isMlSignalStrategyKind(value: unknown): boolean {
  return String(value || "").trim() === "MlSignal";
}
```

Use it in `runOptimization()` and `runValidation()`:

```ts
runOptimization(): void {
  if (isMlSignalStrategyKind(this.strategyKind)) {
    this.statusLine = "MlSignal uses score-cache generation instead of optimizer search.";
    void import("../services/mlSignalService").then(
      ({ generateMlSignalScoreCache }) => generateMlSignalScoreCache(),
    );
    return;
  }
  void runOptimizationAction();
}

runValidation(): void {
  if (isMlSignalStrategyKind(this.strategyKind)) {
    this.statusLine = "MlSignal uses score-cache-backed policy validation.";
    void import("../services/mlSignalService").then(
      ({ runMlSignalValidation }) => runMlSignalValidation(),
    );
    return;
  }
  void runValidationAction();
}
```

### G2. Do **not** add more optimizer-native ML state here right now
Avoid growing `optimizeStore.ts` into a pseudo-dedicated ML store during this pass. Fix contract and replay first.

---

## H. `web/src/components/OptimizeWorkspacePanel.vue`

### Goals
1. keep the current bridge explicit,
2. avoid implying that `MlSignal` is just another optimizer route.

### Recommended edit
Keep the existing special-case mount, but add a bridge note so the current UX is honest.

```vue
<section v-if="isMlSignalKind" class="pane">
  <div class="sectionEyebrow">Bridge mode</div>
  <div class="muted mono">
    MlSignal is currently mounted inside Optimize as a compatibility bridge.
    Saved ML artifacts and reruns should restore into this panel, but this is
    not the final shell model.
  </div>
</section>

<MlSignalWorkspacePanel v-if="isMlSignalKind" />
```

This is intentionally small. I would not do a full shell split in this patch.

---

## I. `web/src/components/MlSignalWorkspacePanel.vue`

### Goals
1. make sweep size visible before launch,
2. surface richer artifact state,
3. set the user’s expectations correctly about the current bridge.

### I1. Add computed sweep estimate

```ts
function axisCount(startRaw: unknown, endRaw: unknown, stepRaw: unknown): number {
  const start = Number(startRaw);
  const end = Number(endRaw);
  const step = Number(stepRaw);
  if (!Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(step)) return 1;
  if (step <= 0 || end < start) return 1;
  return Math.floor((end - start) / step + 1e-9) + 1;
}

const estimatedSweepSize = computed(() => {
  return (
    axisCount(mlSignalSweepLongStart.value, mlSignalSweepLongEnd.value, mlSignalSweepLongStep.value) *
    axisCount(mlSignalSweepShortStart.value, mlSignalSweepShortEnd.value, mlSignalSweepShortStep.value) *
    axisCount(mlSignalSweepDeltaStart.value, mlSignalSweepDeltaEnd.value, mlSignalSweepDeltaStep.value) *
    axisCount(mlSignalSweepHoldStart.value, mlSignalSweepHoldEnd.value, mlSignalSweepHoldStep.value)
  );
});

const sweepWithinLimit = computed(() => estimatedSweepSize.value <= 20_000);
```

Use it in the template:

```vue
<div class="muted mono">
  Estimated sweep size: {{ estimatedSweepSize }} candidate{{
    estimatedSweepSize === 1 ? "" : "s"
  }}
  <span v-if="!sweepWithinLimit"> • exceeds 20,000 backend cap</span>
</div>

<button
  type="button"
  class="primary"
  :disabled="!canRunSweep || !sweepWithinLimit"
  @click="void onRunPolicySweep()"
>
  Run policy sweep
</button>
```

### I2. Surface richer artifact metadata
Once the artifact block is extended, show:
- dataset export manifest,
- split manifest,
- score space,
- duplicate timestamp count.

### I3. Adjust copy to reduce shell-model confusion
I would change the opening copy slightly:

```vue
<div class="sectionEyebrow">MlSignal workflow</div>
<h3>Pattern recognition + ML signal workflow</h3>
<div class="muted">
  This workflow is currently mounted inside Optimize as a bridge. It exports
  datasets, builds score caches, validates score-to-position policies, and
  replays ML artifacts, but it is not a conventional optimizer search path.
</div>
```

---

## J. `crates/app_core/benches/e2e_workflow_ml.rs`

### Goals
1. stop using this filename as if it covered `MlSignal`,
2. add direct Criterion coverage for the actual `MlSignal` workflow.

### Recommendation
Either:
- rename the current file to make it clear it is a Stage 5/meta ML-confidence workflow bench, **or**
- keep it and add a second group in the same file for the actual `MlSignal` flow.

### Benchmark targets to add
1. `ml_signal/export_dataset`
2. `ml_signal/build_score_cache`
3. `ml_signal/validate_cold_cache`
4. `ml_signal/validate_warm_cache`
5. `ml_signal/policy_sweep_small`
6. `ml_signal/policy_sweep_large`

### Skeleton

```rust
fn bench_ml_signal_workflow(c: &mut Criterion) {
    let candles = make_synth_candles(10_000, 0.0001, 0.001, 0.0);
    let cancel = AtomicBool::new(false);

    c.bench_function("ml_signal/export_dataset", |b| {
        b.iter(|| {
            let req = MlSignalDatasetExportRequest {
                data_id: "bench".to_string(),
                output_path: temp_file_path("ml_dataset", "csv"),
                ma_source: "close".to_string(),
                horizon_bars: 10,
                round_trip_cost_bps: 4.0,
                split_plan: None,
                ticker_holdout: None,
            };
            export_ml_signal_dataset_with_candles(req, &candles, &cancel).expect("export");
        })
    });

    // Add cache build / validation / sweep benches here.
}
```

---

## K. `research/ml_signal/train_catboost.py`

### Goals
1. make the bundle contract more explicit,
2. tell the app and humans what score columns are expected.

### Add bundle score-contract metadata

```py
dump_json(
    out_dir / "manifest.json",
    {
        "bundle_id": bundle_id,
        "schema_version": 1,
        "feature_schema_version": int(bundle.schema.get("schema_version", 1) or 1),
        "model_family": "catboost_binary_pair",
        "model_format": "cbm",
        "long_model_path": "model_long.cbm",
        "short_model_path": "model_short.cbm",
        "notes": "Trained from exported MlSignal dataset CSV.",
        "score_contract": {
            "preferred_probability_columns": ["p_long_cal", "p_short_cal", "p_long", "p_short"],
            "raw_margin_columns": ["raw_long", "raw_short"],
            "validation_mode": "probability_pair_required",
        },
    },
)
```

This is not a hot-path optimization. It is a contract-clarity improvement that will make later app checks and debugging much easier.

---

## L. `research/ml_signal/calibrate.py`

### Goals
1. make calibration visible in the manifest,
2. make the intended preferred columns explicit.

### Update the manifest more fully

```py
if manifest_path.exists():
    with manifest_path.open("r", encoding="utf-8") as handle:
        manifest = json.load(handle)
    if isinstance(manifest, dict):
        manifest["calibration_path"] = "calibration.json"
        manifest["calibration_applied"] = True
        manifest["score_contract"] = {
            "preferred_probability_columns": ["p_long_cal", "p_short_cal"],
            "fallback_probability_columns": ["p_long", "p_short"],
            "raw_margin_columns": ["raw_long", "raw_short"],
            "validation_mode": "probability_pair_required",
        }
        with manifest_path.open("w", encoding="utf-8") as handle:
            json.dump(manifest, handle, indent=2, sort_keys=True)
            handle.write("\n")
```

---

## M. `research/ml_signal/evaluate_walkforward.py`

### Goals
1. stop implying app-policy parity,
2. make the proxy nature of the output explicit,
3. keep the harness useful without overselling it.

### Add explicit summary fields

```py
summary = {
    # existing fields...
    "policy_evaluation_mode": "proxy_threshold_label_return",
    "desktop_policy_parity": False,
    "desktop_parity_gaps": [
        "no max_hold_bars state machine",
        "no exit_on_flat / exit_on_opposite policy state",
        "no cooldown or persistence handling",
        "no iterative simulator stop/take-profit handling",
    ],
    # ...
}
```

### Keep the current script, but document it honestly
Do **not** pretend it is equivalent to desktop validation until a true parity path exists.

---

## N. `research/ml_signal/gate_report.py`

### Goals
1. reflect the proxy-evaluation truth in the generated report,
2. reduce the chance of people treating the research gate as app-validation parity.

### Add proxy/parity notes

```py
report = {
    # existing fields...
    "policy_evaluation_mode": walkforward.get("policy_evaluation_mode", "unknown"),
    "desktop_policy_parity": bool(walkforward.get("desktop_policy_parity", False)),
    "notes": [
        "This report is a research gate only. It does not replace desktop score-cache validation.",
        "The walk-forward summary may be generated from proxy threshold/label-return logic rather than the desktop iterative simulator.",
        "Ticker-holdout comparison remains a separate research step.",
        "Threshold-stability review still requires comparing multiple policy outputs across rolls.",
    ],
}
```

Also update the markdown output to say the same thing.

---

## O. `research/ml_signal/README.md`

### Goals
1. document the real bundle contract,
2. tell the user what the app expects,
3. avoid parity confusion.

### Add a “score-column contract” section

```md
## Score-column contract

The desktop `MlSignal` validation path expects a probability-pair score cache.

Preferred columns, in order:
1. `p_long_cal`, `p_short_cal`
2. `p_long`, `p_short`

Raw margin columns (`raw_long`, `raw_short`) may still be emitted for diagnostics, but
desktop score-cache validation should treat raw-only caches as unsupported unless an
explicit raw-margin policy mode is added later.
```

### Add a “walk-forward parity” section

```md
## Walk-forward parity note

`evaluate_walkforward.py` is currently a research proxy. It applies threshold logic to the
exported dataset and summarizes label-return outcomes, but it does not reproduce the desktop
iterative simulator or the full score-to-position policy state machine.

Use desktop `MlSignal` validation as the source of truth for final policy selection.
```

---

## P. `ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`

### Goal
Keep the implementation plan aligned with what you are actually shipping.

### Recommended doc updates
1. Add a short note under the rollout steps:
   - v1 app-side validation requires a **probability pair** cache,
   - raw-only caches are rejected,
   - dataset export + score-cache + validation artifacts must be replayable.

2. Add a short note that the current `strategyKind` bridge is temporary and should not be deepened.

---

## Q. `mlsignal_mode_separation_plan_2026-03-31.md`

### Goal
Make the later shell split depend on the right prerequisites.

### Add mode-split entry criteria
Before promoting `MlSignal` to a top-level mode, require:

- dataset export artifacts persist and restore correctly,
- score cache manifests carry score space and dataset identity,
- saved ML validation and sweep reruns are deterministic,
- history restoration can reopen ML artifacts into the correct working surface.

This keeps the later mode split from becoming a state-model bug amplifier.

---

# 9. Recommended implementation order (merge plan)

## Patch set 1 — correctness + replay contract
- `ml_signal.rs`: score-space enum, calibrated-column preference, raw-only rejection, duplicate timestamps.
- `mlSignalService.ts`: dataset export artifact persistence, action-kind metadata.
- `workflowArtifactsStore.ts`: richer `ml_signal` block.
- `historyService.ts`: restore/rerun validation/sweep correctly.
- `mlSignalService.test.ts`: replay + preflight coverage.

## Patch set 2 — policy semantics
- `ml_signal.rs`: implement `allow_flip`, `cooldown_bars`, `signal_persistence_bars`.
- `MlSignalWorkspacePanel.vue`: update copy if needed to explain effective policy fields.

## Patch set 3 — hot-path optimization
- `ml_signal.rs`: prefix math, split-membership precompute, row-buffer reuse, compact JSON, parsed-cache memoization.

## Patch set 4 — benchmark coverage
- `e2e_workflow_ml.rs`: add direct `MlSignal` benches or split the file.

## Patch set 5 — research harness truthfulness
- `README.md`, `evaluate_walkforward.py`, `gate_report.py`, `train_catboost.py`, `calibrate.py`.

## Patch set 6 — later shell decision
- only after the above is stable.

---

# 10. Recommended product stance on the UI question

You said you are on the fence about exposing ML mode as a discrete separate workflow versus keeping it as a strategy inside the unified UI.

My recommendation is:

## What to do now
**Do not** do the full shell split in this same implementation pass.

Why:
- replay is not correct yet,
- artifact metadata is incomplete,
- dataset export is not even persisted as an ML stage today,
- rerun semantics are still incomplete.

If you split the shell **before** fixing those, you risk moving state-model confusion into a larger surface area.

## What to do after the fixes land
Once:
- dataset export is persisted,
- validation/sweep rerun is correct,
- ML artifacts restore deterministically,
- score-space identity is explicit,

then the dedicated top-level `MlSignal` mode becomes much safer and much easier to justify.

## Bottom line
- **Near-term:** keep the current bridge, but make it honest and robust.
- **Steady state:** move to a discrete `MlSignal` mode.

That matches both the current code reality and the deferred separation plan without making the UI split the blocker for the core fixes.

---

# 11. Implementation checklist

## Backend: `crates/app_core/src/ml_signal.rs`
- [ ] Add `MlSignalScoreSpace`.
- [ ] Parse calibrated score columns first (`p_long_cal`, `p_short_cal`).
- [ ] Persist `score_space` into the score-cache payload and manifest.
- [ ] Reject raw-only caches in validation/sweep for v1.
- [ ] Count duplicate timestamps during cache build.
- [ ] Persist `duplicate_timestamps` into the score-cache result and manifest.
- [ ] Extend `MlSignalScoreCacheBuildRequest` / result / manifest to carry stronger ML identity metadata.
- [ ] Add memoized parsed score-cache file loading.
- [ ] Replace hash-map alignment on warm validation/sweep with a merge-walk over sorted timestamps.
- [ ] Replace repeated rolling-window rescans with return/squared-return prefix helpers.
- [ ] Reuse the CSV row buffer during dataset export.
- [ ] Precompute split membership instead of scanning windows per row.
- [ ] Write the large score-cache JSON in compact form.
- [ ] Implement `allow_flip` in the desired-position mapper.
- [ ] Implement `cooldown_bars` in the desired-position mapper.
- [ ] Implement `signal_persistence_bars` in the desired-position mapper.
- [ ] Add backend tests for calibrated-column preference.
- [ ] Add backend tests for raw-only rejection.
- [ ] Add backend tests for duplicate timestamp counting.
- [ ] Add backend tests for cooldown/persistence/allow_flip behavior.
- [ ] Add backend tests for score-space manifest serialization.
- [ ] Add backend tests for prefix-math parity.

## Tauri: `src-tauri/src/commands_ml_signal.rs`
- [ ] Extend score-cache build request to accept optional validation/dataset context.
- [ ] Validate score-cache build context against the loaded dataset.
- [ ] Keep model-status checks advisory for now.
- [ ] Thread through any new score-cache response fields needed by the frontend.

## Frontend service: `web/src/services/mlSignalService.ts`
- [ ] Persist dataset export as an artifact-backed ML stage.
- [ ] Add ML artifact `action_kind` metadata (`dataset_export`, `score_cache_build`, `validation`, `policy_sweep`).
- [ ] Merge previous ML artifact fields forward when later stages run.
- [ ] Persist `export_manifest_path` into the ML artifact block.
- [ ] Persist `split_manifest_path` into the ML artifact block.
- [ ] Persist `score_space` into the ML artifact block.
- [ ] Persist `duplicate_timestamps` into the ML artifact block.
- [ ] Remove extraneous unknown fields from backend invoke payloads.
- [ ] Add sweep-size preflight estimation.
- [ ] Block oversized sweeps before backend invocation.
- [ ] Improve `mlSignalDatasetMeta` with schema/manifest basenames.
- [ ] Improve `mlSignalScoreMeta` with score space / duplicates once backend returns them.
- [ ] Keep current-run snapshot fallback robust when validation starts without a prior score-cache build artifact.

## Frontend tests: `web/src/services/mlSignalService.test.ts`
- [ ] Add test that dataset export persists an artifact.
- [ ] Add test that oversize sweep is blocked before invoke.
- [ ] Add test that enriched ML artifact metadata is stored.
- [ ] Add test that score-cache response metadata appears in optimize meta strings.

## Workflow artifacts: `web/src/stores/workflowArtifactsStore.ts`
- [ ] Extend `WorkflowMlSignalArtifact` with dataset export fields.
- [ ] Extend `WorkflowMlSignalArtifact` with score-space and duplicate-count fields.
- [ ] Normalize `action_kind`.
- [ ] Normalize `export_manifest_path`.
- [ ] Normalize `split_manifest_path`.
- [ ] Keep old artifacts backward-compatible by treating missing fields as empty strings/nulls.

## History/replay: `web/src/services/historyService.ts`
- [ ] Stop relying only on request-shape heuristics for ML artifact restoration.
- [ ] Restore ML dataset-export paths from artifact metadata.
- [ ] Restore single-validation thresholds from saved `validation.req`.
- [ ] Restore sweep axes from saved `validation.req`.
- [ ] Restore ML gate fields (`min_psr`, `min_dsr`) from saved `validation.req`.
- [ ] Set `strategyKind = "MlSignal"` when loading/applying ML artifacts.
- [ ] Rerun the full ML pipeline, not just score-cache generation.
- [ ] Default ML rerun to rebuilding the score cache before validation/sweep.
- [ ] Add focused replay tests once corresponding test coverage exists in this area.

## Store bridge: `web/src/stores/optimizeStore.ts`
- [ ] Add a centralized `isMlSignalStrategyKind()` helper.
- [ ] Keep the special-case bridge explicit in `runOptimization()`.
- [ ] Keep the special-case bridge explicit in `runValidation()`.
- [ ] Do not expand optimizer-native ML state in this file beyond what is required for the current bridge.

## Optimize bridge UI: `web/src/components/OptimizeWorkspacePanel.vue`
- [ ] Keep the existing special-case mount.
- [ ] Add brief bridge copy so the current shell model is honest.
- [ ] Do not turn this patch into a full shell refactor.

## MlSignal panel UI: `web/src/components/MlSignalWorkspacePanel.vue`
- [ ] Add computed sweep-size estimate.
- [ ] Show estimated sweep size in the UI.
- [ ] Disable the sweep action when the estimated size exceeds the cap.
- [ ] Surface richer artifact metadata (export manifest, split manifest, score space, duplicates).
- [ ] Adjust opening copy so it reads as a workflow bridge rather than a normal optimizer path.
- [ ] Keep desktop-only behavior non-throwing.

## Benchmarking: `crates/app_core/benches/e2e_workflow_ml.rs`
- [ ] Add a direct dataset-export benchmark.
- [ ] Add a direct score-cache build benchmark.
- [ ] Add a direct validation benchmark.
- [ ] Add cold vs warm score-cache validation coverage.
- [ ] Add policy-sweep scaling coverage.
- [ ] Rename or clarify the current non-`MlSignal` benchmark scope if needed.

## Research harness: `research/ml_signal/train_catboost.py`
- [ ] Add `score_contract` metadata to the bundle manifest.
- [ ] Document preferred probability columns in the manifest.

## Research harness: `research/ml_signal/calibrate.py`
- [ ] Mark `calibration_applied` in the manifest.
- [ ] Persist preferred calibrated-column metadata in the manifest.

## Research harness: `research/ml_signal/evaluate_walkforward.py`
- [ ] Add `policy_evaluation_mode` to the summary.
- [ ] Add `desktop_policy_parity = false`.
- [ ] Add explicit `desktop_parity_gaps` summary fields.
- [ ] Keep current logic as a proxy until a parity path exists.

## Research harness: `research/ml_signal/gate_report.py`
- [ ] Surface `policy_evaluation_mode`.
- [ ] Surface `desktop_policy_parity`.
- [ ] Add explicit notes that the report does not replace desktop validation.

## Documentation: `research/ml_signal/README.md`
- [ ] Add a score-column contract section.
- [ ] State the preferred probability-column order.
- [ ] State that raw-only caches are unsupported in v1 validation.
- [ ] Add a walk-forward parity note.
- [ ] Tell users to use desktop validation as the source of truth for final policy choice.

## Context docs: `ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`
- [ ] Note that app-side validation is probability-cache based in v1.
- [ ] Note that raw-only caches are rejected.
- [ ] Note that the current optimize-surface bridge is temporary.

## Context docs: `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md`
- [ ] Add entry criteria for mode split:
  - [ ] dataset export artifact persistence is fixed,
  - [ ] ML validation/sweep replay is deterministic,
  - [ ] score-space identity is explicit,
  - [ ] history restore is correct.

## Deferred / later
- [ ] Decide whether to add a true desktop-parity research CLI or keep the Python harness explicitly proxy-only.
- [ ] Revisit the top-level mode split once replay/artifact identity is stable.
- [ ] Revisit binary/columnar score-cache formats only after the current JSON contract is correct and benchmarked.

---

# 12. Final recommendation

If you implement only three things from this document first, make them these:

1. **Fix score-space correctness**  
   calibrated-column preference, raw-only rejection, and no silent raw fallback.

2. **Fix replay correctness**  
   persist dataset export, carry ML metadata forward, restore/rerun saved validation and sweep requests.

3. **Fix the export and cache hot path**  
   prefix math + memoized parsed cache + sweep preflight.

That will give you the biggest improvement in trustworthiness, implementation clarity, and actual day-to-day usability before you decide whether the UI should become a dedicated top-level mode.
