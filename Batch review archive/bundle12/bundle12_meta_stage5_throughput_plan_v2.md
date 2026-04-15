
# Bundle 12 Implementation Plan: Meta-selection and Stage 5 ML confidence throughput

## Surface under review

This pass is limited to the Bundle 12 scope from `gpt54_feature_review_map.md`: artifact scan/load, grouping, holdout evaluation, Stage 4 / Stage 5 model-status/train/apply/policy-evaluation throughput, and saved-artifact hydration around the meta pipeline. It intentionally avoids broad Meta UX redesign, new ML product features, or architectural rewrites.

## Files reviewed

### Primary files
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`

### Supporting benchmark / workflow files
- `crates/app_core/benches/meta_select_workflow.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

## Scope guard and implementation posture

Keep this work:
- optimization-first
- benchmark-first
- parity-preserving
- focused on throughput, blocking cost, status drift, and saved-artifact hydration

Do **not** turn this pass into:
- generic Meta UX cleanup
- new workflow panels
- new Stage 5 product features
- broad ML platform redesign
- new caching layers without invalidation rules and perf evidence

---

# 1. Surface understanding

The current meta / Stage 5 pipeline has three distinct cost centers:

1. **Artifact fanout and JSON churn**
   - `run_meta_selection_blocking` scans the runs directory, reads/parses many JSON files, and only later filters by kind/scope.
   - `load_meta_artifacts_with_paths_blocking` and `load_meta_artifacts_blocking` eagerly parse all meta JSON files for status, Stage 5 auto-train, and policy evaluation.

2. **Repeated meta / ML orchestration**
   - The frontend hot path does `runMeta -> refresh saved artifacts -> mlRefresh -> maybe mlTrain -> mlRefresh -> maybe mlApply`.
   - The backend Stage 5 hot path does `try_load_model -> status scan -> maybe load metas -> train -> status scan again -> score groups`.

3. **Repeated label extraction / model training work**
   - `ml_confidence.rs` does a target-resolution pass and then a dataset-build pass.
   - `commands_meta.rs` does a holdout-row detection pass and then a row-count pass.
   - policy evaluation retrains from scratch for every prefix of the meta history.

The core in-memory meta scoring logic is generally acceptable. The highest-leverage wins are in the app-path around scan/load, IPC payload size, and duplicated orchestration.

---

# 2. Executive summary

## Highest-priority safe wins

1. **Stop fully parsing obviously non-run JSON files in `run_meta_selection_blocking`.**
2. **Stop deserializing and returning the full saved ML model on every status refresh.**
3. **Stop sending full meta-group payloads over IPC for `mlApply`; send only the seven ML feature fields.**
4. **Stop doing immediate duplicate status refreshes after training when the train response already has the required metadata.**
5. **Stop awaiting saved-artifact list refreshes on the hot user-visible path.**
6. **Fuse duplicated scope/label scans in both `ml_confidence.rs` and `commands_meta.rs`.**
7. **Make selected-group holdout evaluation allocate against the selected subset, not the full group count.**

## Medium-priority improvements

8. **Precompute / reuse representative param signatures in `meta_select.rs` to reduce repeated canonicalization.**
9. **Extend benchmarks so Bundle 12 measures status/train/apply/policy-eval cost rather than only meta-select and holdout.**

## Higher-risk but high-payoff improvements

10. **Introduce prepared-row / prefix training for policy evaluation.**
11. **Introduce a summary manifest / index for run and meta artifacts.**
12. **Optionally add a process-level summary/model cache with explicit invalidation by mtime.**

---

# 3. Findings

## Finding 1 — Run-artifact scan fanout reads and parses unrelated JSON files

- **Severity:** high
- **Type:** performance
- **Location:** `src-tauri/src/commands_meta.rs::run_meta_selection_blocking`
- **Timing boundary affected:** app-path
- **Why it matters:** the current scan excludes only `vectorbt_meta_*` / `vectorgrid_meta_*` file names and then fully reads/parses every remaining JSON file into `serde_json::Value`. That means Stage 5 artifacts, policy-evaluation artifacts, batch summaries, and other non-run JSON files still pay the read + parse + allocation cost on every meta run.
- **Recommended change:** add a stronger filename prefilter immediately, then switch to buffered `serde_json::from_reader` to reduce transient string allocation. Treat an on-disk artifact index/manifest as a second-phase optimization.

### Current issue shape

The scan currently does:

- enumerate all `.json`
- skip only `vectorbt_meta_*` / `vectorgrid_meta_*`
- `read_to_string`
- `serde_json::from_str::<Value>`
- later pass the full `Vec<Value>` into `meta_select_from_artifacts`, where `artifact_matches_request` finally filters by run kind, strategy, timeframe, and tag

That is the right correctness behavior, but it is not the right app-path work shape once the run directory contains more than just run artifacts.

### Recommended code edits

#### A. Add explicit non-run filename filtering in `commands_meta.rs`

```rust
fn is_known_non_run_artifact_filename(name: &str) -> bool {
    let n = name.trim();
    n.starts_with("vectorbt_meta_")
        || n.starts_with("vectorgrid_meta_")
        || n.starts_with("vectorbt_stage5_")
        || n.starts_with("vectorgrid_stage5_")
        || n.starts_with("vectorbt_ml_confidence_eval_")
        || n.starts_with("vectorgrid_ml_confidence_eval_")
        || n.starts_with("vectorbt_batch_summary_")
        || n.starts_with("vectorgrid_batch_summary_")
}
```

Then change the directory scan:

```rust
for entry in entries {
    let entry = entry.map_err(|e| e.to_string())?;
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some("json") {
        continue;
    }
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        continue;
    };
    if is_known_non_run_artifact_filename(name) {
        continue;
    }
    paths.push(path);
}
```

#### B. Stop allocating a full intermediate string for every scanned file

```rust
use std::fs::File;
use std::io::BufReader;

fn read_json_value(path: &Path) -> Option<Value> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).ok()
}
```

Use it in `run_meta_selection_blocking` and in the meta loaders:

```rust
let v = match read_json_value(p) {
    Some(v) => v,
    None => continue,
};
artifacts.push(v);
```

### Rollout note

This is a safe first patch because it does **not** change selection semantics. It only avoids obviously irrelevant file parses and lowers per-file allocation pressure.

---

## Finding 2 — `ml_confidence_model_status_blocking` loads the full model and returns the full model body

- **Severity:** high
- **Type:** performance
- **Location:** `src-tauri/src/commands_meta.rs::MlConfidenceModelStatus`, `ml_confidence_model_status_blocking`, `web/src/services/metaService.ts::formatMlStatus`, `maybeRunMetaMlAutopilot`
- **Timing boundary affected:** app-path
- **Why it matters:** status refresh currently:
  - loads the full saved model JSON
  - deserializes the full `MlConfidenceModel`
  - scans/parses meta artifacts to compute freshness / available rows
  - returns the full model body in the status payload  
  The frontend status rendering only needs label thresholds, health, state, and training metadata. It does **not** need model weights or trees on each refresh.
- **Recommended change:** introduce a **summary-only** status path and a lightweight model summary type. Keep full-model load separate.

### Recommended code edits

#### A. Add a public model summary type in `ml_confidence.rs`

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceModelSummary {
    pub version: u32,
    pub scope: MlConfidenceScope,
    #[serde(default = "default_model_kind")]
    pub model_kind: MlConfidenceModelKind,
    pub label: MlConfidenceLabelConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MlConfidenceModelMeta>,
}

impl From<&MlConfidenceModel> for MlConfidenceModelSummary {
    fn from(model: &MlConfidenceModel) -> Self {
        Self {
            version: model.version,
            scope: model.scope.clone(),
            model_kind: model.model_kind.clone(),
            label: model.label.clone(),
            meta: model.meta.clone(),
        }
    }
}
```

#### B. Change the status payload to carry a summary, not the full model

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
}
```

#### C. Add a summary loader instead of full model deserialization

```rust
fn try_load_ml_confidence_model_summary(path: &Path) -> Result<ta_app_core::MlConfidenceModelSummary, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| e.to_string())
}
```

Use it in `ml_confidence_model_status_blocking`:

```rust
let mut model_summary: Option<ta_app_core::MlConfidenceModelSummary> = None;
let mut error: Option<String> = None;
let mut label_cfg = MlConfidenceLabelConfig::default();

if exists {
    bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len() as usize;
    match try_load_ml_confidence_model_summary(&path) {
        Ok(summary) => {
            label_cfg = summary.label.clone();
            model_summary = Some(summary);
        }
        Err(e) => error = Some(e),
    }
}
```

Then build state from `model_summary.as_ref()` instead of full model.

#### D. Frontend changes in `metaService.ts`

Use:
- `status.model_summary`
- `status.model_healthy`

instead of:
- `status.model`
- `status.model.meta`

Example:

```ts
function formatMlStatus(
  status: Record<string, unknown>,
  scope: { strategy_kind: string; timeframe: string }
): string {
  const summary = asRecord(status.model_summary)
  const label = asRecord(summary?.label)
  const modelMeta = asRecord(summary?.meta)

  const healthy = status.model_healthy === false || modelMeta?.healthy === false
    ? 'UNHEALTHY'
    : status.model_healthy === true || modelMeta?.healthy === true
      ? 'healthy'
      : 'unknown'

  // ... keep the rest of the formatting logic the same
}
```

### Rollout note

This is one of the biggest app-path wins because it reduces:
- disk read cost
- deserialization cost
- IPC payload size
- frontend parse/GC cost

without changing the saved model file format.

---

## Finding 3 — `mlApply` sends full meta groups over IPC even though only seven numeric features are needed

- **Severity:** high
- **Type:** performance
- **Location:** `web/src/services/metaService.ts::mlApply`, `src-tauri/src/commands_meta.rs::apply_ml_confidence_model`, `crates/app_core/src/ml_confidence.rs::predict_ml_confidence_for_group`
- **Timing boundary affected:** app-path
- **Why it matters:** the frontend currently passes the entire `groups` array from the current meta result into the Tauri command. Those group objects can contain `train_rows` and `holdout_rows`, which makes the IPC payload far larger than required. The ML apply path only needs:
  - `samples`
  - `pass_rate`
  - `robust_score_p10`
  - `robust_score_median`
  - `max_dd_p90`
  - `cross_score_median`
  - `meta_score`
- **Recommended change:** introduce a minimal ML feature DTO and apply against that instead of full `Value` groups.

### Recommended code edits

#### A. Add a typed feature payload in `ml_confidence.rs`

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceGroupFeatures {
    pub samples: f64,
    pub pass_rate: f64,
    pub robust_score_p10: f64,
    pub robust_score_median: f64,
    pub max_dd_p90: f64,
    #[serde(default)]
    pub cross_score_median: f64,
    pub meta_score: f64,
}

impl MlConfidenceGroupFeatures {
    fn to_vec(&self) -> Option<Vec<f64>> {
        let x = vec![
            self.samples,
            self.pass_rate,
            self.robust_score_p10,
            self.robust_score_median,
            self.max_dd_p90,
            self.cross_score_median,
            self.meta_score,
        ];
        if x.iter().all(|v| v.is_finite()) {
            Some(x)
        } else {
            None
        }
    }
}

pub fn predict_ml_confidence_for_features(
    model: &MlConfidenceModel,
    features: &MlConfidenceGroupFeatures,
) -> Option<f64> {
    let x0 = features.to_vec()?;
    let p_raw = match model.model_kind {
        MlConfidenceModelKind::Logistic => {
            if model.weights.len() != model.mean.len()
                || model.weights.len() != model.std.len()
                || model.weights.is_empty()
                || x0.len() != model.weights.len()
            {
                return None;
            }
            let mut z = model.bias;
            for j in 0..x0.len() {
                let x = (x0[j] - model.mean[j]) / model.std[j];
                z += model.weights[j] * x;
            }
            sigmoid(z)
        }
        MlConfidenceModelKind::GbdtStump => {
            let gbdt = model.gbdt.as_ref()?;
            predict_prob_gbdt_row(gbdt, &x0)?
        }
    };

    if let Some(cal) = model.calibration.as_ref() {
        Some(apply_calibration(p_raw, cal))
    } else {
        Some(p_raw)
    }
}
```

Keep the current `predict_ml_confidence_for_group` as a compatibility adapter:

```rust
pub fn predict_ml_confidence_for_group(model: &MlConfidenceModel, group: &Value) -> Option<f64> {
    let x0 = extract_features_from_meta_group(group)?;
    let features = MlConfidenceGroupFeatures {
        samples: x0[0],
        pass_rate: x0[1],
        robust_score_p10: x0[2],
        robust_score_median: x0[3],
        max_dd_p90: x0[4],
        cross_score_median: x0[5],
        meta_score: x0[6],
    };
    predict_ml_confidence_for_features(model, &features)
}
```

#### B. Change the command signature in `commands_meta.rs`

```rust
#[tauri::command]
pub async fn apply_ml_confidence_model(
    scope: MlConfidenceScope,
    groups: Vec<ta_app_core::MlConfidenceGroupFeatures>,
    app: tauri::AppHandle,
) -> Result<ApplyMlConfidenceResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let path = ml_confidence_model_path(&scope, &app)?;
        if !path.exists() {
            return Err("ML confidence model not found. Train it first.".to_string());
        }

        let model = {
            let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            let reader = std::io::BufReader::new(file);
            serde_json::from_reader::<_, MlConfidenceModel>(reader).map_err(|e| e.to_string())?
        };

        let confidences = groups
            .iter()
            .map(|g| ta_app_core::predict_ml_confidence_for_features(&model, g))
            .collect();

        Ok(ApplyMlConfidenceResponse {
            model_path: path.to_string_lossy().to_string(),
            confidences,
            warnings: Vec::new(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
```

#### C. Frontend `metaService.ts` should send only minimal features

```ts
function toMlFeatureGroup(group: Record<string, unknown>): Record<string, unknown> {
  return {
    samples: Number(group.samples || 0),
    pass_rate: Number(group.pass_rate || 0),
    robust_score_p10: Number(group.robust_score_p10 || 0),
    robust_score_median: Number(group.robust_score_median || 0),
    max_dd_p90: Number(group.max_dd_p90 || 0),
    cross_score_median: Number(group.cross_score_median || 0),
    meta_score: Number(group.meta_score || 0)
  }
}
```

Then in `mlApply`:

```ts
const featureGroups = groups.map(toMlFeatureGroup)

const raw = (await invokeCommand('apply_ml_confidence_model', {
  scope,
  groups: featureGroups
})) as unknown
```

### Rollout note

This change pays off immediately in:
- lower IPC payload size
- lower JSON serialization cost in the webview
- lower deserialization cost in Tauri
- lower GC pressure on apply/autopilot

This is a very strong safe win.

---

## Finding 4 — Frontend and backend both duplicate status/train/apply work

- **Severity:** high
- **Type:** performance
- **Location:** `web/src/services/metaService.ts::runMeta`, `maybeRunMetaMlAutopilot`, `mlTrain`; `src-tauri/src/commands_meta.rs::run_meta_stage4_experimental_blocking`
- **Timing boundary affected:** app-path
- **Why it matters:** there are multiple redundant steps in the user-visible hot path.
  - Frontend: `runMeta` awaits saved refresh and then `maybeRunMetaMlAutopilot`, which always calls `mlRefresh`; if training happens, `mlTrain` immediately calls `mlRefresh` again.
  - Backend Stage 5: `run_meta_stage4_experimental_blocking` loads the model, calls status, maybe loads/trains, and then calls status again.
- **Recommended change:** return summary/status payloads from train responses and build post-train status locally instead of immediately rescanning.

### Recommended code edits

#### A. Slim `TrainMlConfidenceResponse` and include a status payload

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TrainMlConfidenceResponse {
    pub model_path: String,
    pub model_bytes: usize,
    pub model_summary: ta_app_core::MlConfidenceModelSummary,
    pub metrics: MlConfidenceTrainMetrics,
    pub status: MlConfidenceModelStatus,
}
```

#### B. Build status once inside `train_ml_confidence_model`

Sketch:

```rust
#[tauri::command]
pub async fn train_ml_confidence_model(
    scope: MlConfidenceScope,
    cfg: Option<MlConfidenceTrainConfig>,
    app: tauri::AppHandle,
) -> Result<TrainMlConfidenceResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let cfg = cfg.unwrap_or_default();

        let metas = load_meta_artifacts_blocking(&app)?;
        let (model, metrics) =
            ta_app_core::train_ml_confidence_model_from_meta_artifacts(scope.clone(), &metas, cfg)?;

        let path = ml_confidence_model_path(&model.scope, &app)?;
        let bytes = serde_json::to_vec_pretty(&model).map_err(|e| e.to_string())?;
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

        let (available_meta_artifacts, newest_meta_ts_available, available_labeled_rows) =
            summarize_meta_artifacts_for_scope(&metas, &model.scope, &model.label);

        let model_summary = ta_app_core::MlConfidenceModelSummary::from(&model);
        let status = build_ml_confidence_status_from_summary(
            &path,
            bytes.len(),
            Some(model_summary.clone()),
            None,
            available_meta_artifacts,
            newest_meta_ts_available,
            available_labeled_rows,
        );

        Ok(TrainMlConfidenceResponse {
            model_path: path.to_string_lossy().to_string(),
            model_bytes: bytes.len(),
            model_summary,
            metrics,
            status,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
```

The exact helper shape is flexible. The important part is: **do not train, write, and then immediately call the full status path again**.

#### C. Frontend `mlTrain` should consume the train response directly

Add a helper:

```ts
function applyMlStatusPayload(status: Record<string, unknown>, scope: { strategy_kind: string; timeframe: string }): void {
  LAST_META_ML_STATUS = status
  patchAnalysisStore({
    metaMlStatus: formatMlStatus(status, scope)
  })
  updateMetaMlButtons()
}
```

Then:

```ts
export async function mlTrain(options: MetaMlActionOptions = {}): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope)
    if (!normalizeText(scope.timeframe)) {
      setStatusLine('Timeframe required for ML model training.')
      return
    }

    const raw = (await invokeCommand('train_ml_confidence_model', {
      scope,
      cfg: null
    })) as unknown
    const payload = asRecord(raw) ?? {}
    const status = asRecord(payload.status) ?? {}

    applyMlStatusPayload(status, scope)

    const store = getAnalysisStore()
    if (store?.metaUseMlConfidence && asRecordArray(asRecord(LAST_META_RUN?.result)?.groups).length > 0) {
      await mlApply({ scope })
      return
    }

    setStatusLine('ML model trained.')
  } catch (error) {
    setErrorStatus(error)
  }
}
```

This removes the immediate second `mlRefresh()` call.

#### D. Backend Stage 5 should not call `ml_confidence_model_status_blocking` twice

Instead:
- call it once before deciding whether to train
- after training, derive the post-train state from the just-produced model summary and the already-loaded meta scope summary

Pseudo-shape:

```rust
let pre_status = ml_confidence_model_status_blocking(scope.clone(), &app)?;

if should_train {
    let metas_with_paths = load_meta_artifacts_with_paths_blocking(&app)?;
    let (metas, excluded_source_count) =
        stage4_training_metas_excluding_source(metas_with_paths, &source_meta_path);

    let (trained_model, metrics) =
        ta_app_core::train_ml_confidence_model_from_meta_artifacts(scope.clone(), &metas, cfg)?;

    // write model once
    // build post-train status locally
    // do NOT call ml_confidence_model_status_blocking() again
}
```

### Rollout note

This is one of the biggest “feels faster” changes because it removes redundant full-scan calls on both frontend and backend paths.

---

## Finding 5 — Policy evaluation retrains from scratch over the entire prefix for every artifact

- **Severity:** high
- **Type:** performance
- **Location:** `src-tauri/src/commands_meta.rs::run_ml_confidence_policy_evaluation_blocking`
- **Timing boundary affected:** app-path
- **Why it matters:** policy evaluation loops over `scoped[..idx]` and calls `train_ml_confidence_model_from_meta_artifacts(...)` for each prefix. That preserves rolling-origin semantics, but it also repeats target detection, group feature extraction, row assembly, scaling, training, and calibration work from scratch on every step.
- **Recommended change:** introduce a prepared-row / prefix-training path so row extraction happens once and each iteration trains from prefix views of already prepared rows.

### Recommended code edits

## Phase 1 (safe): prepare rows once, still retrain each prefix

In `ml_confidence.rs`, expose a prepared dataset representation:

```rust
#[derive(Clone, Debug)]
pub struct MlConfidencePreparedRows {
    pub gate_rows: Vec<DatasetRow>,
    pub dataset_rows: Vec<DatasetRow>,
    pub newest_meta_ts: Option<String>,
    pub used_meta: usize,
}

pub fn prepare_ml_confidence_rows_from_meta_artifacts(
    scope: &MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: &MlConfidenceLabelConfig,
) -> Result<MlConfidencePreparedRows, String> {
    let build = build_candidate_rows_for_scope(scope, meta_artifacts, cfg);
    Ok(MlConfidencePreparedRows {
        gate_rows: build.gate_rows,
        dataset_rows: build.dataset_rows,
        newest_meta_ts: build.newest_meta_ts,
        used_meta: build.used_meta,
    })
}
```

Add a second trainer that works from rows directly:

```rust
pub fn train_ml_confidence_model_from_rows(
    scope: MlConfidenceScope,
    rows: &[DatasetRow],
    build: DatasetBuildSummary,
    cfg: MlConfidenceTrainConfig,
) -> Result<(MlConfidenceModel, MlConfidenceTrainMetrics), String> {
    // move the existing post-build training logic here
}
```

Then policy evaluation becomes:

```rust
let prepared_history = prepare_policy_eval_history(&scoped, &scope, &label_cfg)?;
let mut prefix_gate_rows: Vec<ta_app_core::DatasetRow> = Vec::new();
let mut prefix_dataset_rows: Vec<ta_app_core::DatasetRow> = Vec::new();

for idx in 0..prepared_history.len() {
    if idx == 0 {
        artifacts_skipped_no_history += 1;
        continue;
    }

    prefix_gate_rows.extend(prepared_history[idx - 1].gate_rows.iter().cloned());
    prefix_dataset_rows.extend(prepared_history[idx - 1].dataset_rows.iter().cloned());

    let use_dataset_rows = !prefix_dataset_rows.is_empty();
    let rows = if use_dataset_rows {
        &prefix_dataset_rows
    } else {
        &prefix_gate_rows
    };

    let build = DatasetBuildSummary {
        used_meta: idx,
        newest_meta_ts: prepared_history[idx - 1].ts.clone(),
        target: if use_dataset_rows {
            MlConfidenceTargetResolved::HoldoutDatasetPass
        } else {
            MlConfidenceTargetResolved::HoldoutGate
        },
    };

    let (model, metrics) = ta_app_core::train_ml_confidence_model_from_rows(
        scope.clone(),
        rows,
        build,
        train_cfg.clone(),
    )?;

    // evaluate candidates for scoped[idx]
}
```

## Phase 2 (higher payoff): optional prepared-row cache keyed by path/mtime

If policy evaluation is still too slow after Phase 1:
- persist a small sidecar cache keyed by artifact path + mtime + label config hash
- store only prepared rows / prepared group features, not raw artifacts
- invalidate on mtime or label threshold change

### Rollout note

This is the highest-value deeper refactor in Bundle 12. It is also the most invasive, so it belongs after the safe wins.

---

## Finding 6 — `Auto` target resolution and status row counting both do duplicated scope scans

- **Severity:** medium
- **Type:** performance
- **Location:** `crates/app_core/src/ml_confidence.rs::scope_has_holdout_rows`, `build_dataset_from_meta_artifacts`; `src-tauri/src/commands_meta.rs::scope_has_holdout_rows_for_ml`, `count_labeled_rows_in_meta`, `summarize_meta_artifacts_for_scope`
- **Timing boundary affected:** app-path
- **Why it matters:** both the core training path and the status path walk the same meta artifacts multiple times.
  - `ml_confidence.rs`: one pass to decide whether `holdout_rows` exists, then a second pass to actually build rows
  - `commands_meta.rs`: one pass to decide whether holdout rows exist, then another pass to count labeled rows
- **Recommended change:** turn both into single-pass builders that collect both aggregate-label rows and dataset-level rows during the same scope walk.

### Recommended code edits

#### A. Replace `scope_has_holdout_rows + build_dataset_from_meta_artifacts` with one builder

```rust
#[derive(Clone, Debug)]
struct DatasetBuildBuffers {
    gate_rows: Vec<DatasetRow>,
    dataset_rows: Vec<DatasetRow>,
    used_meta: usize,
    newest_meta_ts: Option<String>,
    warnings: Vec<String>,
}

fn build_candidate_rows_for_scope(
    scope: &MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: &MlConfidenceLabelConfig,
) -> DatasetBuildBuffers {
    let mut gate_rows = Vec::new();
    let mut dataset_rows = Vec::new();
    let mut used_meta = 0usize;
    let mut newest_meta_ts: Option<String> = None;
    let mut warnings = Vec::new();

    // one scope walk:
    // - push valid holdout aggregate labels into gate_rows
    // - push valid holdout_rows labels into dataset_rows
    // - update used_meta/newest_meta_ts/warnings once

    DatasetBuildBuffers {
        gate_rows,
        dataset_rows,
        used_meta,
        newest_meta_ts,
        warnings,
    }
}
```

Then inside `train_ml_confidence_model_from_meta_artifacts`:

```rust
let build = build_candidate_rows_for_scope(&scope, meta_artifacts, &cfg.label);

let (rows, target_used) = match cfg.target {
    MlConfidenceTarget::HoldoutGate => (build.gate_rows.clone(), MlConfidenceTarget::HoldoutGate),
    MlConfidenceTarget::HoldoutDatasetPass => (
        build.dataset_rows.clone(),
        MlConfidenceTarget::HoldoutDatasetPass,
    ),
    MlConfidenceTarget::Auto => {
        if !build.dataset_rows.is_empty() {
            (build.dataset_rows.clone(), MlConfidenceTarget::HoldoutDatasetPass)
        } else {
            (build.gate_rows.clone(), MlConfidenceTarget::HoldoutGate)
        }
    }
};
```

#### B. Replace the commands-side double scan with a single scope summary

```rust
#[derive(Default, Clone, Debug)]
struct MetaScopeSummary {
    available_meta_artifacts: usize,
    newest_meta_ts_available: Option<String>,
    labeled_rows_from_holdout: usize,
    labeled_rows_from_holdout_rows: usize,
}

fn summarize_meta_artifacts_for_scope(
    metas: &[Value],
    scope: &MlConfidenceScope,
    cfg: &MlConfidenceLabelConfig,
) -> MetaScopeSummary {
    let mut out = MetaScopeSummary::default();

    for meta in metas {
        // scope filter once
        // update meta count/newest
        // count both holdout aggregate labels and holdout_rows labels in the same pass
    }

    out
}
```

Then choose:

```rust
let labeled_rows = if summary.labeled_rows_from_holdout_rows > 0 {
    summary.labeled_rows_from_holdout_rows
} else {
    summary.labeled_rows_from_holdout
};
```

### Rollout note

This is a small but very defensible safe win.

---

## Finding 7 — Selected-group holdout evaluation still allocates for the full group count

- **Severity:** medium
- **Type:** performance
- **Location:** `src-tauri/src/commands_meta.rs::apply_holdout_evaluation`
- **Timing boundary affected:** app-path
- **Why it matters:** even when `holdout_eval_top_n` / `holdout_eval_explore_n` limit holdout evaluation to a subset of groups, the function still allocates:
  - `accs` sized to `result.groups.len()`
  - `holdout_rows` sized to `result.groups.len()`
  - `local` per dataset sized to `result.groups.len()`
  That reduces the benefit of selective evaluation.
- **Recommended change:** remap selected global indices into a dense local index space and allocate against the selected subset only.

### Recommended code edits

```rust
let selected_globals: Vec<usize> = if let Some(idxs) = selected_groups {
    idxs.to_vec()
} else {
    (0..result.groups.len()).collect()
};

if selected_globals.is_empty() {
    return Ok(());
}

let dense_of_global: HashMap<usize, usize> = selected_globals
    .iter()
    .enumerate()
    .map(|(dense, global)| (*global, dense))
    .collect();

let mut accs: Vec<HoldoutAcc> = vec![HoldoutAcc::default(); selected_globals.len()];
let mut holdout_rows: Vec<Vec<ta_app_core::HoldoutDatasetRow>> = if req.save_holdout_rows {
    vec![Vec::new(); selected_globals.len()]
} else {
    Vec::new()
};

let mut dense_idx_by_key: HashMap<String, usize> = HashMap::with_capacity(selected_globals.len());
for (dense, &global_idx) in selected_globals.iter().enumerate() {
    dense_idx_by_key.insert(result.groups[global_idx].key.clone(), dense);
}
```

Then per dataset:

```rust
let mut local: Vec<Vec<(f64, f64, f64, f64)>> = vec![Vec::new(); selected_globals.len()];
```

And when writing back:

```rust
for (dense_idx, rows) in local.into_iter().enumerate() {
    if rows.is_empty() {
        continue;
    }
    let global_idx = selected_globals[dense_idx];
    // accumulate into accs[dense_idx]
    // then write final results back to result.groups[global_idx]
}
```

### Rollout note

This is a straightforward safe change and should be implemented together with the scan/status fixes.

---

## Finding 8 — `meta_select.rs` recomputes canonical parameter signatures unnecessarily

- **Severity:** medium
- **Type:** performance
- **Location:** `crates/app_core/src/meta_select.rs::pick_representatives`, `canonical_json`
- **Timing boundary affected:** Criterion and app-path
- **Why it matters:** representative selection deduplicates by recomputing `canonical_json(p)` for each representative candidate, even though the parameter signature is stable and can be computed once when the candidate is first visited.
- **Recommended change:** precompute/store parameter signatures inside `GroupAcc.robust_params`.

### Recommended code edits

#### A. Replace `(f64, Value)` with a richer row type

```rust
#[derive(Clone, Debug)]
struct RobustParamRow {
    score: f64,
    param_sig: String,
    params: Value,
}

#[derive(Default)]
struct GroupAcc {
    params: Value,
    ma_source: String,
    feature_context: Option<Value>,
    run_context: Option<Value>,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    robust_scores: Vec<f64>,
    robust_params: Vec<RobustParamRow>,
    max_dds: Vec<f64>,
    cross_scores: Vec<f64>,
    psrs: Vec<f64>,
    dsrs: Vec<f64>,
    dataset_keys: HashSet<String>,
    train_rows: Vec<TrainDatasetRow>,
}
```

#### B. Push the signature once during candidate ingestion

```rust
let param_sig = canonical_json(params);

entry.robust_params.push(RobustParamRow {
    score: robust_score,
    param_sig,
    params: params.clone(),
});
```

#### C. Update representative selection to reuse the cached signature

```rust
fn pick_representatives(sorted: &[RobustParamRow], max: usize) -> Vec<Value> {
    let max = max.max(1);
    if sorted.is_empty() {
        return Vec::new();
    }

    let mid = quantile_index(sorted.len(), 0.5);
    let mut indices = Vec::with_capacity(sorted.len());
    indices.push(mid);
    for d in 1..sorted.len() {
        if mid >= d {
            indices.push(mid - d);
        }
        let hi = mid + d;
        if hi < sorted.len() {
            indices.push(hi);
        }
        if indices.len() >= sorted.len() {
            break;
        }
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for idx in indices {
        let Some(row) = sorted.get(idx) else { continue };
        if !seen.insert(row.param_sig.clone()) {
            continue;
        }
        out.push(row.params.clone());
        if out.len() >= max {
            break;
        }
    }

    if out.is_empty() {
        out.push(sorted[mid].params.clone());
    }

    out
}
```

### Rollout note

This is safe and self-contained. A deeper stable-hash redesign should wait until measurement says this remains a real hotspot.

---

## Finding 9 — Saved-artifact refreshes are awaited on the hot path

- **Severity:** medium
- **Type:** performance
- **Location:** `web/src/services/metaService.ts::runMeta`, `runStage5`, `runMlPolicyEvaluation`
- **Timing boundary affected:** app-path
- **Why it matters:** after each successful run, the frontend waits for a saved-artifact list refresh before the action fully completes. That adds an extra IPC + disk/list round trip to the visible path even though the user already has the result payload needed for immediate rendering.
- **Recommended change:** render immediately, then kick saved-artifact refresh into a non-blocking follow-up task.

### Recommended code edits

```ts
function scheduleAsyncRefresh(task: () => Promise<void>): void {
  void Promise.resolve()
    .then(task)
    .catch((error) => {
      console.warn('Deferred meta refresh failed:', error)
    })
}
```

Then:

```ts
export async function runMeta(options: MetaRunOptions = {}): Promise<void> {
  try {
    // ... existing request build and invoke ...
    renderMetaSelection(response)

    scheduleAsyncRefresh(() => refreshSavedInternal(normalizePath(response.meta_json_path)))

    await maybeRunMetaMlAutopilot()

    if (options.switchTab !== false) {
      selectMetaResultsTab()
    }
  } catch (error) {
    setErrorStatus(error)
  }
}
```

Similarly for Stage 5:

```ts
renderMetaStage4(response)
scheduleAsyncRefresh(() => refreshStage4SavedInternal(responseStage5Path(response)))
selectMetaResultsTab()
```

And policy evaluation:

```ts
renderMlPolicyEvaluation(response)
scheduleAsyncRefresh(() => refreshPolicyEvalSavedInternal(normalizePath(response.eval_json_path)))
```

### Rollout note

This is a safe UX/perf improvement because the result payload is already in hand.

---

## Finding 10 — Builder files are not hotspots, but they should avoid throughput-hostile input drift

- **Severity:** low
- **Type:** correctness / maintainability
- **Location:** `web/src/services/metaRequestBuilder.ts`, `web/src/services/metaStage4RequestBuilder.ts`
- **Timing boundary affected:** app-path (indirect)
- **Why it matters:** these builders are not hotspots, but bad frontend normalization can accidentally create more work:
  - `clusterFloatBucket` currently allows `0`, which effectively disables float quantization in clustered mode
  - Stage 5 thresholds are passed raw and only normalized on the backend
- **Recommended change:** keep the builders thin, but clamp obviously throughput-hostile values on the client too.

### Recommended code edits

#### A. `metaRequestBuilder.ts`

Current:
```ts
const floatBucketRaw = Number(inputs.clusterFloatBucket)
const floatBucket = Number.isFinite(floatBucketRaw) ? Math.max(0, floatBucketRaw) : 0.25
```

Recommended:
```ts
const floatBucketRaw = Number(inputs.clusterFloatBucket)
const floatBucket = Number.isFinite(floatBucketRaw) && floatBucketRaw > 0
  ? floatBucketRaw
  : 0.25
```

This prevents clustered mode from silently degenerating into “no float quantization”.

#### B. `metaStage4RequestBuilder.ts`

Add a shared helper:

```ts
function normalizeUnitIntervalOrPercent(value: unknown, fallback: number): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return fallback
  if (parsed > 1 && parsed <= 100) return Math.max(0, Math.min(1, parsed / 100))
  return Math.max(0, Math.min(1, parsed))
}
```

Use it for:
- `min_ml_confidence`
- `accept_confidence`
- `review_confidence`
- `max_accept_uncertainty`

Example:

```ts
min_ml_confidence: normalizeUnitIntervalOrPercent(inputs.minMlConfidence, 0.55),
accept_confidence: normalizeUnitIntervalOrPercent(inputs.acceptConfidence, 0.70),
review_confidence: normalizeUnitIntervalOrPercent(inputs.reviewConfidence, 0.55),
max_accept_uncertainty: normalizeUnitIntervalOrPercent(inputs.maxAcceptUncertainty, 0.45),
```

### Rollout note

These are small drift-prevention changes, not major perf work. Keep them low priority.

---

# 4. Safe optimization opportunities

## Safe opportunity A — Buffered JSON parsing everywhere in this bundle

Use `serde_json::from_reader(BufReader<File>)` instead of `read_to_string + from_str` in:
- `run_meta_selection_blocking`
- `load_meta_artifacts_with_paths_blocking`
- `try_load_ml_confidence_model`
- any new summary loader helpers

This is safe and reduces transient allocations.

## Safe opportunity B — Summary-only status and training response

Implement:
- `MlConfidenceModelSummary`
- `MlConfidenceModelStatus.model_summary`
- summary-based `formatMlStatus`
- `TrainMlConfidenceResponse.status`

This is likely the best safe user-visible latency win.

## Safe opportunity C — Minimal DTO for ML apply

This is safe because the scorer already only depends on seven numeric features. It reduces IPC size without changing model logic.

## Safe opportunity D — Single-pass scope summary / row build

This is safe because it preserves the same label thresholds and target logic while removing repeated traversals.

## Safe opportunity E — Dense selected-group holdout evaluation

This is safe because it only changes temporary allocation strategy, not evaluation semantics.

## Safe opportunity F — Precomputed representative signatures

This is safe because it preserves canonical equivalence while reducing repeated `canonical_json` calls during representative selection.

## Safe opportunity G — Defer saved-artifact list refreshes

This is safe because the UI already has the run response payload needed for immediate rendering.

---

# 5. Higher-risk optimization opportunities

## Higher-risk A — Prepared-row / prefix training for policy evaluation

This is the highest-payoff deeper refactor in this bundle. It should happen only after:
- scan/status/orchestration wins land
- policy eval is still measurably expensive
- parity tests are added

## Higher-risk B — Artifact manifest / summary index

A manifest could store:
- file path
- kind
- strategy kind
- timeframe
- tags
- ts
- file size / mtime

That would let the app skip most full JSON parses during scan/status/list operations. This is powerful, but it requires:
- invalidation rules
- lazy rebuild logic
- compatibility with older artifacts

## Higher-risk C — Process-level model / status cache

A cached status summary keyed by:
- scope
- model mtime
- runs dir mtime or relevant artifact mtimes

could remove repeated model load + summary scan work, but stale-cache bugs would be worse than a small perf miss. Only add this if measurement says the safe wins are not enough.

## Higher-risk D — Stable hashed group keys

A redesign from canonical JSON strings to a stable sorted hashed representation can pay off, but it is not the first thing to do. Only pursue it if:
- grouped-mode benches still show a clear key-generation hotspot
- the safe signature precompute is not enough

---

# 6. Benchmark and perf-gate review

## `crates/app_core/benches/meta_select_workflow.rs`

### Current value
Good for measuring:
- in-memory `meta_select_from_artifacts`
- exact vs clustered grouping

### Gap
It does **not** measure:
- directory scan/load
- JSON parsing
- scope filtering
- Stage 5 status/train/apply
- policy evaluation
- saved-artifact hydration

### Recommended edit

Add a second benchmark group with richer `feature_context` / `run_context` payloads so grouped key generation cost is more realistic.

```rust
fn make_context_heavy_artifacts(
    n_artifacts: usize,
    n_groups: usize,
    cands_per_artifact: usize,
) -> Vec<Value> {
    let mut out = Vec::with_capacity(n_artifacts);

    for i in 0..n_artifacts {
        let mut cands = Vec::with_capacity(cands_per_artifact);
        for j in 0..cands_per_artifact {
            let gid = (i + j) % n_groups;
            cands.push(json!({
                "params": {
                    "fast_len": 10 + gid,
                    "slow_len": 70 + gid,
                    "alpha": (gid as f64) * 0.125,
                    "beta": (gid as f64) * 0.250
                },
                "robust_score": 1.0 + gid as f64 * 0.01,
                "robust_pareto": true,
                "in_sample": { "max_dd": 0.10 + gid as f64 * 0.001 }
            }));
        }

        out.push(json!({
            "kind": "vectorgrid_run_artifact",
            "tags": ["bench"],
            "dataset_meta": { "label": format!("SYM{i:04}"), "timeframe": "1m", "csv_path": format!("SYM{i:04}.csv") },
            "run": {
                "req": {
                    "strategy_kind": "composite",
                    "feature_context": {
                        "family": "generic",
                        "bucket": i % 8,
                        "flags": { "trend": true, "vol": i % 2 == 0 }
                    },
                    "run_context": {
                        "variant": format!("v{}", i % 4),
                        "seed": i,
                        "regime": if i % 2 == 0 { "risk_on" } else { "risk_off" }
                    }
                }
            },
            "validation": {
                "req": {
                    "feature_context": {
                        "family": "generic",
                        "bucket": i % 8,
                        "flags": { "trend": true, "vol": i % 2 == 0 }
                    },
                    "run_context": {
                        "variant": format!("v{}", i % 4),
                        "seed": i,
                        "regime": if i % 2 == 0 { "risk_on" } else { "risk_off" }
                    }
                },
                "res": { "candidates": cands }
            }
        }));
    }

    out
}
```

Add:
- `clustered_context_rich`
- `exact_context_rich`

## `crates/app_core/benches/e2e_workflow_ml.rs`

### Current value
Good for Criterion-boundary end-to-end in-memory workflow:
- optimize
- validate
- meta-select
- holdout
- ML train/apply

### Gap
It still does **not** isolate policy-eval throughput.

### Recommended edit

Add a second benchmark family that specifically simulates Stage 5 history/prefix evaluation:

```rust
fn make_meta_history(meta_count: usize, groups_per_meta: usize) -> Vec<Value> {
    let mut metas = Vec::with_capacity(meta_count);
    for i in 0..meta_count {
        let mut groups = Vec::with_capacity(groups_per_meta);
        for g in 0..groups_per_meta {
            let good = (i + g) % 3 != 0;
            groups.push(json!({
                "samples": 40 + (g % 10),
                "pass_rate": if good { 0.75 } else { 0.30 },
                "robust_score_p10": if good { 0.12 } else { -0.08 },
                "robust_score_median": if good { 0.18 } else { -0.03 },
                "max_dd_p90": if good { 0.20 } else { 0.55 },
                "cross_score_median": if good { 0.06 } else { -0.02 },
                "meta_score": if good { 0.22 } else { -0.14 },
                "holdout": {
                    "datasets": 50,
                    "pass_rate": if good { 0.72 } else { 0.28 },
                    "score_p10": if good { 0.08 } else { -0.10 },
                    "score_median": if good { 0.12 } else { -0.04 },
                    "sharpe_median": if good { 1.0 } else { -0.4 },
                    "max_dd_p90": if good { 0.22 } else { 0.58 },
                    "pnl_median": if good { 1.0 } else { -1.0 }
                }
            }));
        }

        metas.push(json!({
            "kind": "vectorgrid_meta_selection",
            "ts": format!("2026-02-{:02}T00:00:00Z", (i % 28) + 1),
            "req": { "strategy_kind": "CompositeCpu", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeCpu", "timeframe": "1m", "groups": groups }
        }));
    }
    metas
}
```

Benchmark:
- rolling-prefix train/apply
- prepared-row prefix train/apply (after refactor)

That gives direct evidence for the policy-eval refactor.

## `src-tauri/src/bin/ta_workflow_regression.rs`

### Current value
Good deterministic gate for:
- optimize
- validate
- meta-select
- holdout

### Gap
It does **not** gate:
- ML train
- ML apply
- Stage 5 policy evaluation

### Recommended edit

Extend the breakdown schema conservatively:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct WorkflowStageBreakdown {
    optimize_ms_total: u64,
    validate_ms_total: u64,
    meta_select_ms: u64,
    holdout_ms: u64,
    ml_train_ms: u64,
    ml_apply_ms: u64,
}
```

Keep backward compatibility because `#[serde(default)]` is already present.

Add CLI flags:

```rust
#[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
include_ml: bool,

#[arg(long, default_value_t = 150.0)]
ml_train_stage_regression_pct: f64,

#[arg(long, default_value_t = 150.0)]
ml_apply_stage_regression_pct: f64,
```

Then after holdout:

```rust
let mut ml_train_ms = 0u64;
let mut ml_apply_ms = 0u64;

if cli.include_ml {
    let meta_artifact = json!({
        "kind": "vectorgrid_meta_selection",
        "ts": "reg_meta",
        "req": meta_req,
        "res": meta_res,
    });

    let scope = ta_app_core::MlConfidenceScope {
        strategy_kind: "CompositeCpu".to_string(),
        timeframe: "1m".to_string(),
    };

    let mut cfg = ta_app_core::MlConfidenceTrainConfig::default();
    cfg.label.min_holdout_datasets = holdout.len().max(1);

    let train_start = Instant::now();
    let (model, _metrics) = ta_app_core::train_ml_confidence_model_from_meta_artifacts(
        scope,
        &[meta_artifact.clone()],
        cfg,
    )?;
    ml_train_ms = train_start.elapsed().as_millis() as u64;

    let apply_start = Instant::now();
    if let Some(groups) = meta_artifact.pointer("/res/groups").and_then(|v| v.as_array()) {
        for g in groups {
            let _ = ta_app_core::predict_ml_confidence_for_group(&model, g);
        }
    }
    ml_apply_ms = apply_start.elapsed().as_millis() as u64;
}
```

Then gate them the same way as the other stages.

---

# 7. Test review

## Current strengths

### `meta_select.rs`
Coverage already exists for:
- grouping behavior
- strict shortlist behavior
- validation verdict filtering
- evidence flags
- clustered grouping
- legacy / forward-compatible grouping behavior

### `ml_confidence.rs`
Coverage already exists for:
- training from holdout aggregates
- training from `holdout_rows`
- GBDT training path
- prediction ordering sanity checks

### `commands_meta.rs`
Coverage exists for:
- Stage 5 source exclusion
- health gate behavior
- auto-train fallback behavior
- strategy holdout support routing

## Most important missing tests

### Missing test A — summary-only status payload
Add a unit or command-level test that verifies:
- status can be built from a summary-only model payload
- `model_summary` contains everything `formatMlStatus` needs
- full model body is not required

### Missing test B — minimal feature DTO apply
Add a test that:
- builds a small model
- predicts with `predict_ml_confidence_for_group`
- predicts with `predict_ml_confidence_for_features`
- asserts equality within tolerance

### Missing test C — fused auto-target dataset build
Add a test that:
- passes artifacts with both `holdout` and `holdout_rows`
- asserts `Auto` chooses dataset rows
- passes artifacts with only `holdout`
- asserts `Auto` falls back to aggregate labels
- asserts no behavior change relative to the current implementation

### Missing test D — dense selected-group holdout equivalence
Add a test that:
- runs holdout evaluation with a selected subset
- compares selected-group results before/after dense remapping
- asserts identical selected group scores / final scores / holdout rows

### Missing test E — deferred saved refresh does not block rendering
This belongs in the adjacent frontend service tests (not provided here), but it should verify:
- result rendering happens before refresh completion
- refresh failures do not wipe the just-rendered result

### Missing test F — policy eval prepared-row parity
After the prefix-training refactor:
- compare old policy-eval precision outputs vs new prepared-row outputs on the same synthetic meta history
- assert same best policy and same per-arm precision within tolerance

---

# 8. File-by-file implementation plan

## A. `crates/app_core/src/meta_select.rs`

### What to change now
1. Precompute/store exact parameter signatures for representative dedupe.
2. Leave grouping key architecture mostly unchanged in Phase 1.
3. Keep `canonical_json` as the correctness baseline.

### Why
This file is not the first throughput bottleneck, but it does have a safe micro-optimization that avoids redundant exact-parameter canonicalization.

### Concrete edits
- add `RobustParamRow`
- update `GroupAcc.robust_params`
- compute `param_sig` once per selected candidate
- update `pick_representatives`

### Snippet recap
Use the `RobustParamRow` patch from Finding 8.

### Do **not** do yet
- Do not replace grouping keys with a new hash scheme in the first patch.
- Do not remove canonical JSON until grouped-mode benches prove the safe micro-optimization is insufficient.

---

## B. `crates/app_core/src/ml_confidence.rs`

### What to change now
1. Add `MlConfidenceModelSummary`.
2. Add `MlConfidenceGroupFeatures` + `predict_ml_confidence_for_features`.
3. Fuse `Auto` target detection and dataset build into one scan.
4. Split the current trainer into:
   - row preparation
   - row-based training logic

### Why
This file is central to:
- summary-only status
- minimal-feature apply IPC
- policy-eval prefix training

### Recommended implementation order

#### Step 1 — add summary and feature DTO types
- no behavior change
- immediately useful to command/frontend changes

#### Step 2 — refactor the current training entrypoint
Move the “post-row-build” training code into a helper:

```rust
fn train_ml_confidence_model_from_rows_impl(
    scope: MlConfidenceScope,
    rows: &[DatasetRow],
    build: DatasetBuildSummary,
    cfg: MlConfidenceTrainConfig,
    mut warnings: Vec<String>,
) -> Result<(MlConfidenceModel, MlConfidenceTrainMetrics), String> {
    // move the current training body here
}
```

Then `train_ml_confidence_model_from_meta_artifacts` becomes:
- normalize scope
- build candidate rows once
- choose target
- call `train_ml_confidence_model_from_rows_impl`

#### Step 3 — expose a prepared-row public API
That is the hook `commands_meta.rs` will use for policy evaluation.

### Suggested skeleton

```rust
pub fn train_ml_confidence_model_from_meta_artifacts(
    scope: MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: MlConfidenceTrainConfig,
) -> Result<(MlConfidenceModel, MlConfidenceTrainMetrics), String> {
    let scope = normalize_scope(scope)?;

    let build = build_candidate_rows_for_scope(&scope, meta_artifacts, &cfg.label);
    let (rows, target_used) = resolve_rows_for_target(&build, &cfg.target);

    let build_summary = DatasetBuildSummary {
        used_meta: build.used_meta,
        newest_meta_ts: build.newest_meta_ts.clone(),
        target: target_used,
    };

    train_ml_confidence_model_from_rows_impl(
        scope,
        &rows,
        build_summary,
        cfg,
        build.warnings,
    )
}
```

### Do **not** do yet
- Do not make `DatasetRow` serialization part of the stable artifact contract.
- Keep prepared rows as an internal optimization structure unless a later benchmark proves on-disk caching is worth it.

---

## C. `src-tauri/src/commands_meta.rs`

This is the biggest file in this bundle and the one with the most app-path wins.

### Priority 1 edits
1. Add stronger filename prefilter in `run_meta_selection_blocking`.
2. Switch JSON parsing to buffered readers.
3. Introduce summary-only model status.
4. Slim `TrainMlConfidenceResponse`.
5. Change ML apply to minimal feature DTO.
6. Dense selected-group holdout evaluation allocations.
7. Stop double status scans in Stage 5.

### Priority 2 edits
8. Single-pass scope summary helper.
9. Prepared-row policy evaluation.

### Concrete change set

#### Change set C1 — safer scan path in `run_meta_selection_blocking`
- add `is_known_non_run_artifact_filename`
- use `from_reader`
- optionally extract a shared `read_json_value` helper

#### Change set C2 — summary-only status path
- update `MlConfidenceModelStatus`
- add summary loader
- build status from summary + scope meta summary

#### Change set C3 — `TrainMlConfidenceResponse` should stop shipping full weights/trees
The frontend does not consume the full model after training. Replace:
- `model: MlConfidenceModel`
with:
- `model_summary`
- `status`

#### Change set C4 — apply command should take minimal features
Use the DTO from `ml_confidence.rs`.

#### Change set C5 — Stage 5 auto-train should reuse loaded info
Do not do:
- initial status scan
- load metas
- train
- second status scan

Instead:
- initial status scan once
- if train needed, reuse the just-loaded metas and just-trained summary to build post-train state locally

#### Change set C6 — dense holdout evaluation
Use the dense remapping patch from Finding 7.

#### Change set C7 — prepared-row policy eval
Add this after the safe wins land.

### Example helper: status builder from summary

```rust
fn build_ml_confidence_status_from_summary(
    model_path: &Path,
    bytes: usize,
    summary: Option<ta_app_core::MlConfidenceModelSummary>,
    error: Option<String>,
    available_meta_artifacts: usize,
    newest_meta_ts_available: Option<String>,
    available_labeled_rows: usize,
) -> MlConfidenceModelStatus {
    let exists = model_path.exists();
    let stale = false; // fill from summary/meta comparison
    let retrain_recommended = false; // fill from summary/meta comparison

    let state = if !exists {
        MlConfidenceModelState::Missing
    } else if error.is_some() {
        MlConfidenceModelState::Blocked
    } else if summary.as_ref().and_then(|s| s.meta.as_ref()).map(|m| m.healthy) == Some(false) {
        MlConfidenceModelState::Blocked
    } else {
        MlConfidenceModelState::Healthy
    };

    MlConfidenceModelStatus {
        exists,
        model_path: model_path.to_string_lossy().to_string(),
        bytes,
        available_meta_artifacts,
        available_labeled_rows,
        newest_meta_ts_available,
        stale,
        retrain_recommended,
        state,
        legacy_model: summary.as_ref().and_then(|s| s.meta.as_ref()).is_none(),
        model_healthy: summary.as_ref().and_then(|s| s.meta.as_ref()).map(|m| m.healthy),
        retrain_reason: None,
        model_summary: summary,
        error,
    }
}
```

The exact stale/retrain logic should still mirror the current code; the point is to make it reusable and summary-based.

### Do **not** do yet
- Do not background the actual meta selection or Stage 5 scoring.
- Do not add a global cache before the summary-only status path and the scan fanout fix land.

---

## D. `web/src/services/metaService.ts`

### What to change now
1. Stop depending on `status.model`.
2. Use `status.model_healthy` and `status.model_summary`.
3. Stop calling `mlRefresh` immediately after `mlTrain`.
4. Send minimal ML feature DTOs in `mlApply`.
5. Defer saved-artifact refreshes in `runMeta`, `runStage5`, and `runMlPolicyEvaluation`.

### Concrete edits

#### D1 — use top-level status fields in autopilot
Current:
```ts
const modelMeta = asRecord(asRecord(status.model)?.meta) ?? {}
const modelHealthy = normalizeBoolean(modelMeta.healthy)
```

Recommended:
```ts
const modelHealthy = status.model_healthy === true
```

#### D2 — use train response status directly
See Finding 4.

#### D3 — minimal apply payload
See Finding 3.

#### D4 — defer saved refreshes
See Finding 9.

#### D5 — optional helper to keep status updates centralized

```ts
function applyMlStatusPayload(
  status: Record<string, unknown> | null,
  scope: { strategy_kind: string; timeframe: string }
): void {
  LAST_META_ML_STATUS = status ?? {}
  patchAnalysisStore({
    metaMlStatus: status ? formatMlStatus(status, scope) : '—'
  })
  updateMetaMlButtons()
}
```

Use this from:
- `mlRefresh`
- `mlTrain`
- `mlDelete`

### Why this matters
This file is where the user actually feels the latency. Even if the backend is fast enough, waiting on redundant refreshes and large IPC payloads makes the path feel slow.

### Do **not** do yet
- Do not redesign the Meta workspace.
- Do not move Stage 5 selection logic into the frontend.
- Do not add speculative caching in local storage for model/status objects.

---

## E. `web/src/services/metaRequestBuilder.ts`

### What to change now
Only one meaningful throughput-related correction:
- clamp `clusterFloatBucket` to a positive default in clustered mode

### Why
A zero or negative float bucket silently turns clustered mode into a more exact grouping shape, which can explode group count and reduce the value of clustered mode.

### Suggested edit

```ts
const floatBucketRaw = Number(inputs.clusterFloatBucket)
const floatBucket = Number.isFinite(floatBucketRaw) && floatBucketRaw > 0
  ? floatBucketRaw
  : 0.25
```

### What to leave alone
- `supportsHoldout(...)` is already doing the right kind of frontend gating.
- Do not add extra knobs or builder-side ML behavior here.

---

## F. `web/src/services/metaStage4RequestBuilder.ts`

### What to change now
Very little:
- add client-side unit-interval / percentage normalization so obvious bad inputs do not need backend cleanup

### Suggested edit

```ts
function normalizeUnitIntervalOrPercent(value: unknown, fallback: number): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return fallback
  if (parsed > 1 && parsed <= 100) return Math.max(0, Math.min(1, parsed / 100))
  return Math.max(0, Math.min(1, parsed))
}
```

Use for:
- `min_ml_confidence`
- `accept_confidence`
- `review_confidence`
- `max_accept_uncertainty`

### Why
This is not a hotspot, but it reduces frontend/backend drift and avoids unnecessary invalid-request cleanup work.

---

## G. `crates/app_core/benches/meta_select_workflow.rs`

### What to change now
- keep existing `exact` vs `clustered`
- add a second benchmark group for context-rich grouping shapes

### Why
The current bench likely understates the cost of grouped key generation because the synthetic params are very small.

### Suggested addition
Use the `make_context_heavy_artifacts(...)` pattern from the Benchmark section.

---

## H. `crates/app_core/benches/e2e_workflow_ml.rs`

### What to change now
- keep the existing full workflow bench
- add a second benchmark group for rolling-prefix Stage 5 / policy-eval-like throughput

### Why
Bundle 12 is about more than “can I train/apply once in-memory”. It also needs evidence for the repeated-prefix training path.

### Suggested addition
Use the `make_meta_history(...)` + rolling-prefix benchmark from the Benchmark section.

---

## I. `src-tauri/src/bin/ta_workflow_regression.rs`

### What to change now
- extend stage breakdown with `ml_train_ms` and `ml_apply_ms`
- add optional `--include_ml`
- add regression thresholds for the ML stages
- keep backward compatibility with old baseline JSON

### Why
This file is the best existing place to turn Bundle 12 into an actual regression gate instead of a one-off review memo.

### Suggested addition
Use the schema/CLI patch from the Benchmark section.

### What to defer
- full policy-eval gating in this binary can be a second step
- first, gate train/apply
- then decide whether policy eval should live here or in a dedicated gate

---

# 9. Suggested improvement plan

## Phase 1 — Safe, high-return app-path wins
Implement first:

1. `commands_meta.rs`
   - filename prefilter for non-run JSON
   - buffered JSON parsing
   - summary-only status
   - slim training response
   - dense holdout allocation remap

2. `ml_confidence.rs`
   - `MlConfidenceModelSummary`
   - `MlConfidenceGroupFeatures`
   - `predict_ml_confidence_for_features`

3. `metaService.ts`
   - use summary status fields
   - send minimal apply payload
   - stop immediate post-train `mlRefresh`
   - defer saved refreshes

### Acceptance criteria
- `runMeta` feels faster with a large runs directory
- `mlRefresh` returns faster and does not ship full weights/trees
- `mlApply` payload size is much smaller
- Stage 5 auto-train no longer does a redundant second status scan
- all existing behavior remains semantically identical

## Phase 2 — Single-pass builders
Implement next:

4. `ml_confidence.rs`
   - fuse `Auto` target detection + dataset build

5. `commands_meta.rs`
   - fuse holdout-row detection + labeled-row counting into one scope summary

### Acceptance criteria
- no behavioral drift in target selection
- status row counts stay unchanged
- row-build tests pass

## Phase 3 — Benchmark and gate expansion
Implement next:

6. `meta_select_workflow.rs`
   - add context-rich grouped bench

7. `e2e_workflow_ml.rs`
   - add Stage 5 prefix benchmark

8. `ta_workflow_regression.rs`
   - gate ML train/apply stage timings

### Acceptance criteria
- bundle has direct Criterion evidence for grouped/context-rich key cost
- bundle has direct Criterion evidence for prefix training cost
- workflow regression JSON includes ML stage fields when enabled

## Phase 4 — Deeper policy-eval refactor
Implement last:

9. `ml_confidence.rs` + `commands_meta.rs`
   - prepared-row API
   - prefix training from prepared rows

### Acceptance criteria
- same policy eval outcome as before
- materially lower policy-eval runtime
- no label leakage
- same rolling-origin semantics

## Phase 5 — Optional stretch
Only if Phase 1–4 are not enough:

10. artifact manifest / summary index  
11. process-level summary/model cache  
12. stable hashed group-key redesign

---

# 10. Implementation sequencing notes

## Best order inside each commit series

### Commit 1
- `MlConfidenceModelSummary`
- `MlConfidenceGroupFeatures`
- `predict_ml_confidence_for_features`

### Commit 2
- summary-only status in `commands_meta.rs`
- `metaService.ts` status consumers updated

### Commit 3
- slim `train_ml_confidence_model` response
- `metaService.ts::mlTrain` consumes returned status directly

### Commit 4
- minimal-feature `mlApply`
- frontend DTO mapping

### Commit 5
- stronger filename prefilter + buffered parse

### Commit 6
- dense holdout allocation remap

### Commit 7
- single-pass core dataset builder
- single-pass commands-side scope summary

### Commit 8
- benchmark expansions

### Commit 9
- prepared-row policy-eval refactor

This sequencing minimizes risk:
- first reduce payloads and duplicated work
- then refactor repeated scans
- then touch the deeper policy-eval core

---

# 11. Risks and mitigations

## Risk 1 — Frontend status rendering breaks when `status.model` disappears
**Mitigation:** land `model_summary` first, keep a temporary compatibility adapter for one commit if needed.

## Risk 2 — Builder-side clamping changes current user-entered edge-case behavior
**Mitigation:** keep builder normalization conservative and leave backend normalization authoritative.

## Risk 3 — Prepared-row refactor changes target selection or label counts
**Mitigation:** add parity tests comparing old vs new row counts and target resolution on the same synthetic fixtures.

## Risk 4 — Policy-eval refactor accidentally leaks future rows into earlier prefixes
**Mitigation:** append only rows from `idx - 1` when building prefix state for evaluating artifact `idx`.

## Risk 5 — Filename prefilter misses a legacy prefix
**Mitigation:** keep the filename filter narrow and safe in Phase 1; treat the artifact manifest as the robust long-term fix.

---

# 12. Completion checklist

## A. `crates/app_core/src/meta_select.rs`
- [ ] Add `RobustParamRow`
- [ ] Precompute `param_sig` once per selected candidate
- [ ] Update `pick_representatives` to use cached signatures
- [ ] Keep `canonical_json` as the correctness baseline
- [ ] Re-run existing `meta_select.rs` unit tests
- [ ] Add at least one test proving representative output is unchanged

## B. `crates/app_core/src/ml_confidence.rs`
- [ ] Add `MlConfidenceModelSummary`
- [ ] Add `MlConfidenceGroupFeatures`
- [ ] Add `predict_ml_confidence_for_features`
- [ ] Keep `predict_ml_confidence_for_group` as a compatibility adapter
- [ ] Refactor training logic into a row-based helper
- [ ] Fuse `Auto` target detection + row build into one scope walk
- [ ] Add parity tests for:
  - [ ] full-group vs minimal-feature prediction equality
  - [ ] `Auto` target behavior with/without `holdout_rows`
  - [ ] row counts before/after the single-pass refactor
- [ ] Keep saved model file format unchanged

## C. `src-tauri/src/commands_meta.rs`
- [ ] Add `is_known_non_run_artifact_filename`
- [ ] Apply stronger filename prefilter in `run_meta_selection_blocking`
- [ ] Switch scan/load helpers to buffered `from_reader`
- [ ] Replace full-model status loading with summary-only loading
- [ ] Update `MlConfidenceModelStatus` to use `model_summary`
- [ ] Slim `TrainMlConfidenceResponse`:
  - [ ] remove full model body
  - [ ] add `model_summary`
  - [ ] add `status`
- [ ] Change `apply_ml_confidence_model` to accept `Vec<MlConfidenceGroupFeatures>`
- [ ] Refactor Stage 5 auto-train to avoid immediate second status scan
- [ ] Replace double-pass scope summary with single-pass summary
- [ ] Implement dense selected-group holdout allocation remap
- [ ] Add tests for:
  - [ ] filename prefilter helper
  - [ ] summary-only status path
  - [ ] dense holdout remap parity
  - [ ] post-train status building without second status scan
- [ ] Measure run-meta path before/after on a runs directory containing mixed JSON artifacts

## D. `web/src/services/metaService.ts`
- [ ] Add `applyMlStatusPayload`
- [ ] Update `formatMlStatus` to use `status.model_summary`
- [ ] Update `maybeRunMetaMlAutopilot` to use `status.model_healthy`
- [ ] Update `mlTrain` to use the returned status directly
- [ ] Remove immediate post-train `mlRefresh`
- [ ] Add `toMlFeatureGroup`
- [ ] Update `mlApply` to send minimal feature DTOs
- [ ] Defer saved refreshes in:
  - [ ] `runMeta`
  - [ ] `runStage5`
  - [ ] `runMlPolicyEvaluation`
- [ ] Verify the UI still:
  - [ ] updates ML button enablement correctly
  - [ ] applies ML ranking correctly
  - [ ] preserves the just-rendered result even if deferred refresh fails

## E. `web/src/services/metaRequestBuilder.ts`
- [ ] Clamp `clusterFloatBucket` to a positive default
- [ ] Keep holdout support logic unchanged unless backend drift is found
- [ ] Add/update tests if a builder test file exists in the repo

## F. `web/src/services/metaStage4RequestBuilder.ts`
- [ ] Add `normalizeUnitIntervalOrPercent`
- [ ] Normalize `min_ml_confidence`
- [ ] Normalize `accept_confidence`
- [ ] Normalize `review_confidence`
- [ ] Normalize `max_accept_uncertainty`
- [ ] Keep Stage 4 alias compatibility unchanged

## G. `crates/app_core/benches/meta_select_workflow.rs`
- [ ] Add context-rich grouped benchmark fixtures
- [ ] Add `exact_context_rich`
- [ ] Add `clustered_context_rich`
- [ ] Compare grouped-mode timing before/after cached param signatures

## H. `crates/app_core/benches/e2e_workflow_ml.rs`
- [ ] Keep existing full workflow bench
- [ ] Add rolling-prefix Stage 5 / policy-eval-like benchmark
- [ ] Record metadata for the new bench family
- [ ] Compare before/after timings once prepared-row refactor lands

## I. `src-tauri/src/bin/ta_workflow_regression.rs`
- [ ] Extend `WorkflowStageBreakdown` with `ml_train_ms`
- [ ] Extend `WorkflowStageBreakdown` with `ml_apply_ms`
- [ ] Add `--include_ml`
- [ ] Add `ml_train_stage_regression_pct`
- [ ] Add `ml_apply_stage_regression_pct`
- [ ] Keep baseline JSON backward compatible
- [ ] Add ML stage budget checks
- [ ] Decide later whether policy eval belongs in this binary or in a dedicated gate

## J. Policy-evaluation deeper refactor
- [ ] Add prepared-row builder in `ml_confidence.rs`
- [ ] Add row-based public training helper
- [ ] Refactor `run_ml_confidence_policy_evaluation_blocking` to use prefix rows
- [ ] Add parity tests against the current implementation
- [ ] Re-benchmark policy eval before/after
- [ ] Only then consider on-disk prepared-row cache

## K. Validation and rollout
- [ ] Run Rust tests for touched crates/files
- [ ] Run the Meta / ML related frontend service tests if available
- [ ] Run `cargo bench --bench meta_select_workflow`
- [ ] Run `cargo bench --bench e2e_workflow_ml`
- [ ] Run `ta_workflow_regression` with and without `--include_ml`
- [ ] Capture before/after timings for:
  - [ ] meta scan/load
  - [ ] status refresh
  - [ ] train
  - [ ] apply
  - [ ] Stage 5 auto-train path
  - [ ] policy evaluation
- [ ] Verify no behavior drift in:
  - [ ] group ordering without holdout
  - [ ] holdout-adjusted ranking
  - [ ] model target selection
  - [ ] model health classification
  - [ ] Stage 5 selected groups
  - [ ] policy-eval best-policy result

---

# 13. Recommended “first patch” if you want the best return immediately

If you want the best payoff with the least risk, do these first:

1. `MlConfidenceModelSummary` + summary-only status  
2. slim `train_ml_confidence_model` response and stop post-train `mlRefresh`  
3. minimal-feature `mlApply` DTO  
4. deferred saved-artifact refreshes  
5. non-run filename prefilter + buffered parse  
6. dense selected-group holdout allocation remap  

That patch set is the strongest balance of:
- low risk
- clear app-path win
- no model/selection semantic change
- easy regression validation


---

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

