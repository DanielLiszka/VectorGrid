# MlSignal bundle 10 — addendum for missed items, optional improvements, and deeper performance/correctness hardening

Date: 2026-04-01  
Companion to: `mlsignal_bundle10_full_plan_2026-04-01.md`

This addendum focuses on items that are worth adding to the original plan after a second pass over the source files. The goal is still the same:

- improve **correctness**,
- improve **warm-path performance**,
- improve **artifact/replay determinism**,
- avoid removing features or breaking the current workflow.

It is intentionally biased toward changes that are safe, incremental, and compatible with the existing `MlSignal` flow.

---

## What this addendum changes relative to the original plan

There are **six additions** I would explicitly merge into the original plan:

1. fix the **one-sided / long-only score-cache bug**,
2. fix **saved-run loading dropping ML artifact fields**,
3. expose and validate **score coverage / score scope**,
4. stop overstating **track-record length** in ML validation statistics,
5. cache **timestamp-index maps and log returns** on the warm path,
6. reduce **per-candidate allocations** during validation and policy sweep.

There are also several optional improvements beyond the review-map prompt that are worth keeping on your radar.

---

# 1. Important items that should be added to the main plan

## 1.1 Fix one-sided / long-only score-cache handling

### Why this matters

The current CSV reader accepts files that contain only one side of the score stream, because it only requires *at least one* of `p_long`, `p_short`, `raw_long`, or `raw_short`. But the actual policy mapper requires **both** long and short values, because `score_target(...)` immediately unwraps both sides with `?`, even when the strategy is long-only. Meanwhile, the frontend is already serializing `long_only` into the strategy config that accompanies ML validation requests. fileciteturn4file0 fileciteturn4file4

That creates a correctness bug:

- a long-only run with only `p_long` available is currently treated like “no usable score row,”
- the request shape implies long-only is supported,
- the backend behavior is stricter than the request contract.

### What to add to the main plan

Add a new Phase 0 item:

- **make score-side requirements explicit**:
  - `long_only = true` may run with `p_long` only,
  - `long_only = false` requires both long and short score streams,
  - raw-only inputs remain rejected unless you intentionally add raw-margin mode later.

### Recommended backend edit

Replace the current `score_target(...)` logic with side-aware handling.

```rust
fn score_target(point: Option<ScorePoint>, params: &MlSignalPolicyParams) -> Option<i32> {
    let point = point?;

    let p_long = point.p_long.filter(|v| v.is_finite());
    let p_short = point.p_short.filter(|v| v.is_finite());

    if params.strategy.long_only {
        let p_long = p_long?;
        if p_long >= params.t_long {
            return Some(1);
        }
        return Some(0);
    }

    let p_long = p_long?;
    let p_short = p_short?;

    if p_long >= params.t_long && (p_long - p_short) >= params.delta {
        return Some(1);
    }
    if p_short >= params.t_short && (p_short - p_long) >= params.delta {
        return Some(-1);
    }
    Some(0)
}
```

### Tests to add

```rust
#[test]
fn long_only_validation_accepts_long_side_only_scores() {
    // build a score cache with p_long only
    // run validation with strategy.long_only = true
    // assert that candidates are not treated as all-missing
}

#[test]
fn two_sided_validation_rejects_missing_short_side_scores() {
    // build a score cache with p_long only
    // run validation with strategy.long_only = false
    // assert an explicit error instead of silent flat behavior
}
```

---

## 1.2 Fix `loadSavedRun()` dropping ML artifact fields

### Why this matters

`workflowArtifactsStore.ts` is capable of storing both `datasetMeta` and the normalized `mlSignal` artifact block on `currentRun`. But `historyService.ts` currently drops those fields when loading a saved artifact into runtime state: it passes only `ts`, `artifact_path`, `report_xlsx_path`, `csv_path`, `tags`, `req`, and `res`. That means a loaded saved run can lose ML-specific artifact context even though the store is designed to hold it. fileciteturn4file5 fileciteturn4file6

This is separate from rerun correctness. Even before rerunning, the currently loaded workflow state is incomplete.

### What to add to the main plan

Under the `historyService.ts` section, add:

- **preserve `dataset_meta` and `ml_signal` blocks when loading saved runs**, not just when persisting new ones.

### Recommended edit

In `loadSavedRun(...)`, change the `applyWorkflowArtifactsPayload(...)` call to carry the original artifact blocks forward.

```ts
applyWorkflowArtifactsPayload(
  {
    currentRun: {
      ts,
      artifact_path: nextPath,
      report_xlsx_path: reportXlsxPath,
      csv_path: csvPath,
      dataset_meta: record.dataset_meta ?? null,
      ml_signal: record.ml_signal ?? null,
      tags,
      req: runReq,
      res: runRes,
    },
    currentValidation: validationRes
      ? {
          ts,
          req: validationReq,
          res: validationRes,
        }
      : null,
  },
  "vue",
);
```

### Tests to add

```ts
it("loadSavedRun preserves dataset_meta and ml_signal blocks", async () => {
  // mock load_run_artifact with dataset_meta + ml_signal present
  // call loadSavedRun(path)
  // assert workflowArtifacts.currentRun?.datasetMeta is not null
  // assert workflowArtifacts.currentRun?.mlSignal is not null
});
```

---

## 1.3 Add score coverage and score scope to the contract

### Why this matters

The research harness currently produces two materially different score artifacts:

- `train_catboost.py` writes `validation_scores.csv`, and those rows come from the **calibration mask** only, because the score frame is built from `frame.loc[calibration_mask, ...]`, then merged and saved. fileciteturn4file8
- `evaluate_walkforward.py` scores the dataset and writes `walkforward_test_scores.csv` after restricting to walk-forward test slices. fileciteturn4file10

The app-side score-cache builder, however, only reports `matched_rows` and `skipped_rows`, and the UI currently treats the result as a generic score-cache success. It does not distinguish:

- full-dataset scores,
- calibration-only scores,
- walk-forward-test-only scores,
- sparse custom score CSVs. fileciteturn4file0 fileciteturn4file4

That is a correctness and UX problem because policy validation semantics depend on score coverage.

### What to add to the main plan

Add a new contract-hardening item:

- **persist score coverage and score scope** into the score-cache file, manifest, artifact block, and UI.

Recommended new fields:

- `score_scope`: `"full_dataset" | "calibration_slice" | "walkforward_test" | "custom_partial"`
- `coverage_ratio`
- `first_scored_timestamp`
- `last_scored_timestamp`
- `matched_rows`
- `bars_scored`
- `duplicate_timestamps`

### Recommended Rust additions

```rust
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlSignalScoreScope {
    FullDataset,
    CalibrationSlice,
    WalkforwardTest,
    CustomPartial,
}

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
    pub bars_scored: usize,
    pub coverage_ratio: f64,
    pub score_scope: MlSignalScoreScope,
    pub duplicate_timestamps: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_timestamp: Option<i64>,
}
```

### Recommended frontend additions

Update `mlSignalScoreMeta` so the panel clearly tells the user what was built.

Example:

```ts
patchOptimizeState({
  mlSignalScoreMeta: [
    `Score cache: ${matchedRows.toFixed(0)} matched rows`,
    `coverage ${(coverageRatio * 100).toFixed(1)}%`,
    scoreScope,
    duplicateTimestamps > 0 ? `duplicates ${duplicateTimestamps}` : "",
    `-> ${outputPath}`,
  ]
    .filter((part) => part)
    .join(" • "),
});
```

### Tests to add

- build from a short calibration-only CSV and assert `coverage_ratio < 1.0`,
- build from a full-dataset score CSV and assert `coverage_ratio ~= 1.0`,
- surface the returned scope and coverage in `mlSignalScoreMeta`.

---

## 1.4 Track-record length should not equal “number of rows with any score”

### Why this matters

`validate_ml_signal_candidates(...)` currently computes:

- `score_obs = count(point.is_some())`,
- then assigns that same value to every candidate as `track_record_bars`,
- then passes it into the probability-metric pipeline for PSR/DSR handling. fileciteturn4file0

That is probably too generous, because it does **not** distinguish between:

- bars with a score but no active position,
- bars where the candidate is actually long or short,
- sparse score caches with only partial coverage.

Even if you keep the current metric family, `track_record_bars` should be candidate-specific and based on actual active exposure or at least actual non-flat desired positions.

### What to add to the main plan

Add one validation-fidelity item:

- compute `track_record_bars` from candidate activity, not raw score presence.

### Recommended edit

After building `desired`, compute the candidate-specific active count.

```rust
let active_bars = desired.iter().filter(|&&pos| pos != 0).count();

out.push(MlSignalValidationCandidateResult {
    params,
    in_sample: metrics,
    iterative_in_sample: None,
    psr: None,
    dsr: None,
    track_record_bars: active_bars.max(2),
    min_track_record_length_bars: None,
    validation_verdict: ValidationCandidateVerdict::default(),
    robust_score,
    robust_pareto: false,
});
```

If you want a stricter metric later, add a distinct field such as:

- `score_rows_present`,
- `active_bars`,
- `position_changes`,
- `trade_count_estimate`.

### Tests to add

- one high-threshold candidate with very few active bars,
- one low-threshold candidate with many active bars,
- assert `track_record_bars` differs across candidates.

---

## 1.5 Cache timestamp-index maps and log returns on the warm path

### Why this matters

Two warm-path helpers are being recomputed repeatedly today:

1. both score-cache build and score-cache load rebuild `timestamp -> index` maps from `candles.timestamp`, and
2. validation recomputes `log_returns_from_close(&candles.close)` on every validation/sweep call. fileciteturn4file0

This is separate from parsed score-cache memoization. Even if you memoize the JSON parse, you are still paying these per-run setup costs.

### What to add to the main plan

Add a “warm dataset context” item:

- cache `timestamp_to_index` and `log_returns` alongside loaded candle data.

### Recommended direction

At the `AppState` / loaded-data-context layer, add memoized derived fields:

- `timestamp_index: HashMap<i64, usize>`
- `log_returns: Vec<f64>`

Then use them from `commands_ml_signal.rs` / `ml_signal.rs`.

### Suggested shape

```rust
#[derive(Clone)]
pub struct LoadedMlWarmContext {
    pub timestamp_to_index: Arc<HashMap<i64, usize>>,
    pub log_returns: Arc<Vec<f64>>,
}
```

### Why this is safe

These are pure derived values from the already loaded candle set. They are deterministic and invalidated whenever the dataset context changes.

---

## 1.6 Reuse candidate buffers during validation and sweep

### Why this matters

For every candidate, the current validation path allocates:

- `requested = vec![None; n]`,
- `desired = vec![0i32; n]`,

inside `build_desired_positions(...)`, then feeds the result into `simulate_iterative_from_signals(...)`. During larger sweeps, that means repeated allocation and initialization of large vectors per candidate. fileciteturn4file0

This is a real performance surface because policy sweep cost is dominated by repeated candidate evaluation.

### What to add to the main plan

Add a sweep-performance item:

- reuse per-candidate work buffers instead of allocating fresh vectors each time.

### Recommended refactor

Turn `build_desired_positions(...)` into a fill-into-buffer API.

```rust
fn build_desired_positions_into(
    score_points: &[Option<ScorePoint>],
    params: &MlSignalPolicyParams,
    requested: &mut [Option<i32>],
    desired: &mut [i32],
) {
    requested.fill(None);
    desired.fill(0);

    let n = score_points.len();
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

    let mut current = 0i32;
    let mut held_bars = 0usize;
    for bar in 0..n {
        // existing state machine here
        desired[bar] = current;
        if current != 0 {
            held_bars = held_bars.saturating_add(1);
        }
    }
}
```

And in validation:

```rust
let n = score_points.len();
let mut requested = vec![None; n];
let mut desired = vec![0i32; n];

for params in candidates {
    build_desired_positions_into(score_points, &params, &mut requested, &mut desired);
    let metrics = simulate_iterative_from_signals(
        &lr,
        OhlcSlice {
            open: &candles.open,
            high: &candles.high,
            low: &candles.low,
            close: &candles.close,
        },
        &desired,
        start_t.min(lr.len().saturating_sub(1)),
        &params.strategy,
        iterative,
    );
    // ...
}
```

### Why this is worth doing

This does not change policy semantics. It only reduces allocation churn in one of the most obvious O(candidates × bars) paths.

---

# 2. Optional changes beyond the prompt, but still worth considering

These are not required to satisfy the review-map prompt, but they are good optional extensions if your priority remains performance and correctness.

## 2.1 Add a sorted-CSV fast path for score-cache alignment

### Why

The research harness writes stable, sorted score outputs (`validation_scores.csv` sorted by row/timestamp; walk-forward outputs are naturally timestamp ordered). fileciteturn4file8 fileciteturn4file10

Instead of always using a `HashMap` alignment path, detect whether parsed score rows are already monotonic by timestamp and use a two-pointer merge against the candle timestamp vector.

### Benefit

- less hashing,
- less memory churn,
- simpler duplicate detection,
- easier coverage accounting.

### Suggested policy

- fast path: monotonic timestamps -> two-pointer merge,
- fallback path: unsorted input -> current map-based alignment with duplicate counting.

---

## 2.2 Collapse pattern-hit storage into a flatter layout

### Why

`pattern_recognition` currently becomes:

- `Vec<Vec<u8>>` hit matrix,
- then `Vec<Vec<usize>>` prefix counts,
- then per-row string emission during export. fileciteturn4file0

That is easy to reason about, but it is memory-heavy and allocation-heavy.

### Optional direction

Move to one of these:

- flat row-major `Vec<u8>` plus row offsets,
- flat prefix-count matrix,
- bit-packed hit storage if you want to squeeze memory further.

### Constraint

Do this only after the correctness fixes land, because it is more invasive than the prefix-sum return math.

---

## 2.3 Distinguish “calibration-only” vs “full-dataset” score generation in the research harness

### Why

Right now, the offline harness gives you:

- calibration-slice scores from training,
- walk-forward-test scores from evaluation,
- but no clearly named “full-dataset scored CSV for app-side policy work.” fileciteturn4file8 fileciteturn4file10 fileciteturn4file12

### Optional improvement

Add a harness step or flag such as:

```powershell
python score_full_dataset.py `
  --dataset C:\data\ml_dataset.csv `
  --bundle-dir C:\data\ml_signal\bundle_dev
```

or extend `evaluate_walkforward.py` with:

- `--emit-full-dataset-scores`

so the app has an explicit full-coverage input artifact.

### Benefit

This makes app-side validation semantics much clearer and avoids conflating calibration-tuning inputs with full-dataset policy validation.

---

## 2.4 Debounce model-status refresh in `MlSignalWorkspacePanel.vue`

### Why

The panel currently watches `mlSignalModelBundleId` and refreshes model status whenever the value changes. That means a manual text edit can trigger a Tauri request on every keystroke. fileciteturn4file4

### Optional edit

- debounce the watcher, or
- refresh on blur / explicit button click,
- keep immediate refresh only for dropdown selection.

### Priority

Low. This is worth doing for polish, not because it is a major bottleneck.

---

## 2.5 Add a bounded-top-k path for large sweeps

### Why

The current sweep flow evaluates every candidate, stores all candidate results, sorts the full vector, then truncates to `top_k`. fileciteturn4file0

The validation cost still dominates, but once you cross into large candidate counts, storing and sorting the full result set is unnecessary when the user only wants the best few.

### Optional improvement

- keep the full evaluation loop,
- use a bounded min-heap or `select_nth_unstable_by(...)` instead of fully sorting all candidates.

### Priority

Medium-low. I would do buffer reuse first.

---

## 2.6 Stream candidate generation instead of materializing the whole sweep

### Why

`run_ml_signal_policy_sweep_blocking_with_candles(...)` currently expands the full Cartesian product into `Vec<MlSignalPolicyParams>` before validation. fileciteturn4file0

### Optional improvement

Generate candidates lazily and validate them as you go. This reduces peak memory and combines naturally with:

- reusable candidate buffers,
- bounded-top-k selection.

### Priority

Medium-low. Good follow-on after the current correctness fixes.

---

## 2.7 Add a compressed or binary companion score-cache format later

### Why

JSON is fine for visibility and debugging, but it is not ideal for repeated parse-heavy workflows.

### Optional direction

Once the current contract stabilizes, consider:

- compact JSON as the default persisted form,
- optional `.json.gz`,
- or a binary/columnar companion format for warm reuse.

### Constraint

Do **not** do this before the score-space and coverage semantics are frozen.

---

# 3. Revised priority order

If I were updating the original plan today, I would tighten the priority stack to this:

## P0 — correctness blockers

1. calibrated score preference + raw-only rejection,  
2. one-sided / long-only score handling,  
3. preserve `ml_signal` + `dataset_meta` on saved-run load,  
4. rerun the full ML artifact, not just cache generation,  
5. add score coverage / score scope metadata.

## P1 — warm-path performance wins

1. prefix-sum export math,  
2. parsed score-cache memoization,  
3. cached timestamp-index maps + cached log returns,  
4. candidate buffer reuse,  
5. split-membership precompute,  
6. compact cache JSON.

## P2 — validation fidelity hardening

1. candidate-specific `track_record_bars`,  
2. better sparse-score warnings,  
3. explicit score-scope UI labeling,  
4. stronger stale-cache identity checks.

## P3 — optional refinements

1. sorted-CSV fast path,  
2. flattened pattern storage,  
3. bounded top-k selection,  
4. lazy sweep generation,  
5. debounced model-status refresh,  
6. later binary score-cache format.

---

# 4. Checklist additions to merge into the original plan

## Backend

- [ ] Accept long-only score caches that contain only `p_long` when `strategy.long_only = true`.
- [ ] Reject two-sided ML validation when the short-side score stream is missing.
- [ ] Add `score_scope` to the score-cache payload and manifest.
- [ ] Add `coverage_ratio` to the score-cache payload and manifest.
- [ ] Cache `timestamp_to_index` alongside loaded candle data.
- [ ] Cache `log_returns` alongside loaded candle data.
- [ ] Reuse `requested` / `desired` buffers across ML candidate evaluations.
- [ ] Make `track_record_bars` candidate-specific instead of using one global scored-row count.

## Frontend / replay

- [ ] Preserve `dataset_meta` when loading saved ML artifacts.
- [ ] Preserve `ml_signal` when loading saved ML artifacts.
- [ ] Surface score coverage and score scope in `mlSignalScoreMeta`.
- [ ] Warn when score coverage is sparse or clearly slice-specific.
- [ ] Debounce or explicit-trigger bundle-status refresh.

## Research harness

- [ ] Document the difference between `validation_scores.csv` and `walkforward_test_scores.csv` more explicitly.
- [ ] Add an explicit full-dataset score export path if app-side full-coverage validation is desired.
- [ ] Record score scope in research outputs where possible.

---

# 5. Final recommendation

The original plan is still directionally right, but I would explicitly add the six items above before calling it complete.

The **two most important missed items** are:

1. **one-sided / long-only score handling**, and  
2. **saved-run loading preserving ML artifact fields**.

The **best extra performance additions** are:

1. **warm-context caching of timestamp-index maps and log returns**, and  
2. **reusable candidate buffers during validation/sweep**.

The **best optional extension beyond the prompt** is:

- **make score coverage / score scope first-class**, because it closes a real contract gap between the app workflow and the research harness.

