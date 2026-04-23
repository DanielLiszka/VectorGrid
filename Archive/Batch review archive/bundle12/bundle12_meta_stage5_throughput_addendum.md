# Bundle 12 Addendum: extra performance / correctness improvements beyond the original plan

This addendum supplements `bundle12_meta_stage5_throughput_plan.md`.

It focuses on:
- items I would now add or tighten after a second pass over the code
- one important open correctness question that deserved explicit callout
- optional extensions that go slightly beyond the narrow Bundle 12 prompt while still staying performance-first and parity-safe

It does **not** replace the original plan. Treat it as:
- **add these safe wins to Phase 1 / Phase 2**
- **treat the optional items as stretch work only after measurement**

---

## 1. What I would add immediately to the original plan

### A. Early semantic prefilter and single-pass partition during `run_meta_selection_blocking`

## Why this matters

The original plan already recommended a stronger filename prefilter and buffered parsing. I would now add a second safe filter layer:

- do **not** keep every non-meta JSON artifact in memory until later
- as soon as a parsed JSON is available, drop it unless it is a candidate run artifact for the requested strategy / tag / validation shape
- partition directly into `train_artifacts` and `holdout_artifacts` during the scan instead of building `artifacts` first and splitting later

Right now the command:
1. scans all non-meta `.json`
2. parses them into `artifacts: Vec<Value>`
3. only later partitions into `train_artifacts` / `holdout_artifacts`
4. only later performs the real semantic filtering in `meta_select_from_artifacts`

That means even after the filename prefilter, unrelated run artifacts for other strategies, tags, or invalid validation shapes still consume parse time, allocation, and memory retention.

## Recommended change

Add a relaxed prefilter that reuses the current command-layer matching logic as early as possible.

### Suggested code shape

```rust
fn artifact_matches_meta_req_relaxed(artifact: &Value, req: &MetaSelectionRequest) -> bool {
    let kind = artifact.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    if !is_run_artifact_kind(kind) {
        return false;
    }

    let strat = artifact
        .pointer("/run/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| artifact.pointer("/run/req/strategyKind").and_then(|v| v.as_str()))
        .unwrap_or("");
    if strat != req.strategy_kind {
        return false;
    }

    if let Some(tag) = req
        .required_tag
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !artifact_has_tag(artifact, tag) {
            return false;
        }
    }

    artifact.pointer("/validation/res/candidates").is_some()
}
```

Then in `run_meta_selection_blocking`:

```rust
let mut train_artifacts: Vec<Value> = Vec::new();
let mut holdout_artifacts: Vec<Value> = Vec::new();

for (i, p) in paths.iter().enumerate() {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let v = match read_json_value(p) {
        Some(v) => v,
        None => continue,
    };

    if !artifact_matches_meta_req_relaxed(&v, &req) {
        continue;
    }

    let goes_to_holdout = holdout_tag
        .as_deref()
        .map(|tag| artifact_has_tag(&v, tag))
        .unwrap_or(false);

    if goes_to_holdout {
        holdout_artifacts.push(v);
    } else {
        train_artifacts.push(v);
    }

    ta_app_core::progress::emit_double_ma_progress(
        Some(&sink),
        i + 1,
        paths.len(),
        "meta_read",
    );
}
```

If `req.timeframe` is empty, infer timeframe from the already-pruned `train_artifacts`, then do one final retain on both vectors.

## Why this is better than the original wording

This is stronger than the original filename-only improvement because it reduces:
- parse retention of irrelevant run artifacts
- peak memory in the command
- duplicate semantic filtering later

This is still parity-safe because it only discards artifacts that the command would ultimately ignore anyway.

---

### B. Do not silently swallow meta-scan failures in ML status

## Why this matters

`ml_confidence_model_status_blocking` currently does:

```rust
let metas = load_meta_artifacts_blocking(app).unwrap_or_default();
```

That means a meta scan failure is silently converted into “0 available meta artifacts / 0 available labeled rows”.

This is not just an observability issue. It can change user-visible behavior:
- stale detection can be underreported
- retrain recommendation can be skipped incorrectly
- the frontend can present a model as fresh enough when the supporting scan actually failed

## Recommended change

Keep status non-fatal, but surface the scan failure explicitly.

### Suggested payload addition

```rust
#[derive(Debug, Clone, Serialize)]
pub struct MlConfidenceModelStatus {
    pub exists: bool,
    pub model_path: String,
    pub bytes: usize,
    pub available_meta_artifacts: usize,
    pub available_labeled_rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_meta_ts_available: Option<String>,
    pub stale: bool,
    pub retrain_recommended: bool,
    pub state: MlConfidenceModelState,
    pub legacy_model: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_healthy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrain_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_summary: Option<ta_app_core::MlConfidenceModelSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_scan_error: Option<String>,
}
```

Then:

```rust
let (available_meta_artifacts, newest_meta_ts_available, available_labeled_rows, meta_scan_error) =
    match load_meta_artifacts_blocking(app) {
        Ok(metas) => {
            let summary = summarize_meta_artifacts_for_scope(&metas, &scope, &label_cfg);
            (
                summary.available_meta_artifacts,
                summary.newest_meta_ts_available,
                summary.available_labeled_rows,
                None,
            )
        }
        Err(e) => (0, None, 0, Some(e)),
    };
```

Frontend status formatting should append something like:

```ts
const metaScanError = normalizeText(status.meta_scan_error)
if (metaScanError) parts.push(`meta_scan_error=${metaScanError}`)
```

## Priority

High. This is a correctness-preserving improvement that also helps future perf work because it prevents false-good status baselines.

---

### C. Remove the duplicate full-model load in Stage 5 before status is even computed

## Why this matters

The original plan called out duplicate status scans, but the code has an even earlier duplication:

```rust
let model_path = ml_confidence_model_path(&scope, &app)?;
let mut model = try_load_ml_confidence_model(&model_path);

let mut status = ml_confidence_model_status_blocking(scope.clone(), &app)?;
```

So the Stage 5 path can:
- parse the full model once via `try_load_ml_confidence_model`
- then parse it again inside `ml_confidence_model_status_blocking`
- then possibly parse or load it again later for actual scoring

## Recommended change

Compute status first using the summary-only path, then load the full model only when actually needed.

### Suggested code shape

```rust
let model_path = ml_confidence_model_path(&scope, &app)?;
let mut status = ml_confidence_model_status_blocking(scope.clone(), &app)?;
let mut model: Option<MlConfidenceModel> = None;

let should_train = req.auto_train_model
    && (!status.exists
        || status.error.is_some()
        || (req.auto_retrain_stale && status.retrain_recommended));

if should_train {
    // train path
} else if status.exists && status.error.is_none() {
    model = try_load_ml_confidence_model(&model_path);
}
```

If auto-train fails and fallback to an existing model is needed, load the full model only in that branch.

## Priority

High. This belongs with the summary-only status patch.

---

### D. Stage 5 auto-train should update frontend ML status state when it changes the model

## Why this matters

The Stage 5 backend result already exposes:
- `model_path`
- `model_state`
- `model_healthy`
- `model_trained_now`

But the frontend `runStage5()` flow renders the Stage 5 result and refreshes the saved Stage 5 artifact list; it does **not** refresh or update `LAST_META_ML_STATUS` afterward.

That means a Stage 5 run that auto-trained or fixed a stale model can leave the ML status strip/buttons stale until the user manually refreshes ML status.

## Recommended change

Do one of these:

### Safe option 1
After `runStage5()`, trigger a **deferred** `mlRefresh()` only when the model actually changed.

```ts
export async function runStage5(options: MetaStage5RunOptions = {}): Promise<void> {
  try {
    // ... invoke and render ...

    const result = asRecord(response.result) ?? {}
    const modelTrainedNow = normalizeBoolean(result.model_trained_now)

    if (modelTrainedNow) {
      const scope = currentMetaScope()
      scheduleAsyncRefresh(() => mlRefresh({ scope }))
    }

    setStatusLine(
      responseStage5Path(response)
        ? `Stage 5 complete. Saved: ${responseStage5Path(response)}`
        : 'Stage 5 complete.'
    )
  } catch (error) {
    setErrorStatus(error)
  }
}
```

### Better option 2
Extend `RunMetaStage4Response` to include a `ml_status` summary payload and update frontend state directly without an extra command.

That is cleaner, but more invasive.

## Priority

Medium-high. This is a correctness / state-drift fix that also prevents unnecessary manual refresh behavior.

---

### E. Optimistically upsert saved-artifact rows before deferred refresh

## Why this matters

The original plan recommended deferring:
- `refreshSavedInternal`
- `refreshStage4SavedInternal`
- `refreshPolicyEvalSavedInternal`

That is good. I would now improve it one step further:

- immediately insert or update the just-created artifact entry in the local saved-list cache
- then run the real list refresh in the background as reconciliation

This gives:
- instant saved-list responsiveness
- no blocking list IPC on the hot path
- no temporary “my run finished but the saved list did not update” gap

## Recommended change

### Example for meta runs

```ts
function upsertSavedMetaFromRunResponse(resp: Record<string, unknown>): void {
  const result = asRecord(resp.result) ?? {}
  const holdout = asRecord(result.holdout) ?? {}
  const entry = {
    path: normalizePath(resp.meta_json_path),
    report_xlsx_path: normalizePath(resp.report_xlsx_path),
    ts: normalizeText(resp.ts),
    strategy_kind: normalizeText(result.strategy_kind),
    timeframe: normalizeText(result.timeframe),
    required_tag: normalizeText(asRecord(resp.req)?.required_tag),
    groups: asRecordArray(result.groups).length,
    artifacts_used: Math.max(0, Math.floor(Number(result.artifacts_used || 0))),
    holdout_datasets: Math.max(0, Math.floor(Number(holdout.datasets || 0))),
    tags: [],
    bytes: Math.max(0, Math.floor(Number(resp.meta_json_bytes || 0)))
  }

  const path = normalizePath(entry.path)
  SAVED_META = [
    entry,
    ...SAVED_META.filter((row) => normalizePath(row.path) !== path)
  ]
  updateMetaSavedState(path)
}
```

Then in `runMeta()`:

```ts
renderMetaSelection(response)
upsertSavedMetaFromRunResponse(response)
scheduleAsyncRefresh(() => refreshSavedInternal(normalizePath(response.meta_json_path)))
await maybeRunMetaMlAutopilot()
```

Use the same pattern for Stage 5 and policy-eval saved lists.

## Priority

Medium-high. This is a strong frontend responsiveness improvement and stays within current functionality.

---

## 2. Important open question I should have called out explicitly

### Mixed-history `Auto` target semantics in `ml_confidence.rs`

## Why it matters

The current `Auto` target resolution is scope-wide:
- if **any** scoped meta artifact has usable `holdout_rows`, use `HoldoutDatasetPass`
- otherwise use `HoldoutGate`

That means on a mixed history:
- newer metas with `holdout_rows`
- older metas with only aggregate `holdout`

training can switch entirely to dataset-pass mode and skip the older aggregate-labeled history.

The same scope-wide preference also influences command-side label counting for status freshness.

## Why I am not recommending a first-patch semantic change

A “hybrid” policy is not obviously free of side effects because it would mix:
- per-group labels
- per-dataset labels

into one model-training surface.

That might improve sample efficiency, or it might change the model target in a way that breaks comparability.

## What I would add to the plan

### Add an explicit decision checkpoint

- first, preserve current behavior in the safe throughput refactor
- second, add tests that lock in current mixed-history behavior
- third, evaluate one of these optional paths:

#### Option A — explicit hybrid backfill mode
Use dataset rows where available and aggregate labels only for artifacts without `holdout_rows`.

#### Option B — versioned `Auto` behavior
Keep current `Auto` semantics for compatibility, and add an explicit future target mode if benchmarked value is strong.

#### Option C — status-only hybrid counting
Keep training semantics unchanged, but report status availability more transparently:
- `available_dataset_rows`
- `available_group_rows`
- `preferred_target`

## Recommendation

Treat this as an **open correctness / training-quality question**, not as a Phase 1 throughput patch.

---

## 3. Optional additions that go slightly beyond the original prompt scope

These are still performance / correctness oriented, but they extend a bit beyond the narrowest Bundle 12 reading.

### Optional A — Cap policy-eval warning growth

## Why this matters

`run_ml_confidence_policy_evaluation_blocking` can push one warning per failed artifact during rolling-prefix evaluation. On a large history with many failures, the saved evaluation artifact can grow substantially and the preview path can become noisy and slower than necessary.

## Recommended change

```rust
const MAX_POLICY_EVAL_WARNINGS: usize = 25;
let mut suppressed_training_failure_warnings = 0usize;

if warnings.len() < MAX_POLICY_EVAL_WARNINGS {
    warnings.push(format!(
        "Skipped artifact {} due to training failure: {err}",
        scoped[idx].get("ts").and_then(|v| v.as_str()).unwrap_or("unknown")
    ));
} else {
    suppressed_training_failure_warnings =
        suppressed_training_failure_warnings.saturating_add(1);
}
```

At the end:

```rust
if suppressed_training_failure_warnings > 0 {
    warnings.push(format!(
        "Suppressed {} additional training-failure warning(s).",
        suppressed_training_failure_warnings
    ));
}
```

## Risk

Very low. This is a saved-artifact size / clarity improvement.

---

### Optional B — Make XLSX export split or deferrable if profiling shows it matters

## Why this matters

`run_meta_selection_blocking` always:
1. builds XLSX bytes
2. writes the report
3. then writes the meta JSON

That is on the hot path of every meta run.

If report generation is a meaningful share of warm app-path latency, the cleanest optimization is not to remove reporting, but to decouple it.

## Two possible designs

### Design 1 — explicit `save_report_xlsx` flag
Default it to `true` for full compatibility.
Allow batch/autopilot/benchmark flows to set it to `false` when they only need the meta artifact.

### Design 2 — split report generation command
- `run_meta_selection` returns result + JSON artifact immediately
- `export_meta_report` is a follow-up command

## Recommendation

Do **not** do this first.

Only do it if profiling shows XLSX export is a real fraction of `runMeta` latency after the safe scan/status/apply wins land.

---

### Optional C — Training-loop allocation cleanup after prepared-row refactor

## Why this matters

If policy evaluation remains hot even after prepared-row prefix training, the next place to look is the model trainer itself:
- cross-validation repeatedly clones feature rows into train/test vectors
- fold assignment is recomputed by hashing `group_id`
- temporary vectors are rebuilt fold by fold

## Recommendation

Only after row-prep refactor lands:
- precompute fold ids once per row set
- reuse scratch buffers between folds
- consider using a fixed `[f64; 7]` feature row shape for ML internals

This is not the first place to optimize, but it becomes much more relevant if Stage 5 policy evaluation remains CPU-heavy after the row-preparation patch.

---

## 4. Revised priority stack

### Add these to Phase 1 / Phase 2

1. early semantic prefilter + single-pass partition during meta scan
2. surface `meta_scan_error` in ML status instead of swallowing failures
3. remove duplicate full-model load at Stage 5 start
4. sync frontend ML status after Stage 5 auto-train
5. optimistic saved-list upsert before deferred refresh

### Keep as open / optional

6. mixed-history `Auto` target semantics checkpoint
7. cap policy-eval warning growth
8. split/defer XLSX export if measured
9. trainer scratch-buffer reuse after prepared-row refactor

---

## 5. Addendum checklist

### Safe additions to implement soon
- [ ] Add early semantic prefilter during `run_meta_selection_blocking`
- [ ] Partition directly into `train_artifacts` / `holdout_artifacts` while scanning
- [ ] Re-run timeframe inference after the early-pruned scan
- [ ] Add `meta_scan_error` to `MlConfidenceModelStatus`
- [ ] Stop using `unwrap_or_default()` for failed meta scans in status
- [ ] Compute Stage 5 status before loading the full model
- [ ] Load the full model only if scoring or fallback actually needs it
- [ ] After `runStage5()`, refresh ML status only when the model changed
- [ ] Add optimistic local upsert for saved meta runs
- [ ] Add optimistic local upsert for saved Stage 5 runs
- [ ] Add optimistic local upsert for saved policy-eval runs

### Open-question / optional work
- [ ] Add explicit tests for mixed-history `Auto` target semantics
- [ ] Decide whether hybrid backfill is desired or whether current `Auto` semantics remain authoritative
- [ ] Cap policy-eval warning growth
- [ ] Measure XLSX export share in `runMeta` latency
- [ ] If export share is high, evaluate split/deferred report generation
- [ ] After prepared-row refactor, measure trainer allocation share before touching CV internals

### Validation steps
- [ ] Re-measure warm `runMeta` latency on a mixed-artifact runs directory
- [ ] Re-measure `ml_confidence_model_status` latency with summary-only status plus early error surfacing
- [ ] Re-measure `runStage5` when `model_trained_now=true`
- [ ] Verify saved lists update instantly even before deferred refresh returns
- [ ] Verify status UI does not silently report freshness from a failed meta scan
- [ ] Verify no semantic drift in meta selection, holdout scoring, or Stage 5 group ranking

