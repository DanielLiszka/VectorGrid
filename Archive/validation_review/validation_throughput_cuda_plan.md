# Validation throughput and fallback behavior review plan

Date: 2026-03-22  
Bundle target: **`## 4. Validation throughput and fallback behavior`** from the attached GPT-5.4 review map  
Secondary lens applied: allocation/copy-elision discipline from the same map’s allocation guidance  
Primary goal: improve **validation throughput**, **CUDA-path efficiency**, **fallback transparency**, and **benchmark fidelity** without changing validation semantics unless explicitly called out.

## Scope guard

This plan stays inside the review-map posture:

- optimization-first
- benchmark-first
- parity-preserving
- smaller safe changes before larger refactors
- no new product features unless they are directly required to remove measured validation cost or fallback waste

## Files reviewed

- `gpt54_feature_review_map.md`
- `double_ma_validate.rs`
- `validation_pipeline.rs`
- `commands_validation.rs`
- `validation_common.rs`
- `validationRequestBuilder.ts`
- `validationRequestBuilder.test.ts`
- `ta_double_ma_validation_cost.rs`
- `double_ma_validation_workflow.rs`
- `composite_validation_workflow.rs`

## Short executive summary

The highest-value improvements are:

1. **Stop evaluating sparse finalist sets as implicit fast×slow grids on GPU.** The current “fixed-candidate GPU validation” path still expands sparse finalist sets into unique fast/slow ranges and then evaluates a grid-shaped route. This is the single most important CUDA-side inefficiency in the supplied files.
2. **Batch validation ranges instead of re-entering the GPU path once per walk-forward window, CSCV slice, SPA slice, or reopt range.** The current orchestration repeatedly rebuilds host maps and clones metrics per range.
3. **Turn hidden finalist analytics into an internal execution mask.** Preserved-finalists cross reruns currently still pay for unconditional post-pass CPU analytics (`regime_slice_robustness` and `friction_stress`) even though the command layer and frontend summaries describe a lighter rerun shape.
4. **Reuse series/state aggressively on CPU tails.** Once the GPU main route is fast, CPU follow-up work becomes the next bottleneck. `friction_stress` is the clearest example.
5. **Eliminate avoidable copies.** The easiest win outside the core CUDA path is cross-ticker candle alignment: full-range alignment currently still clones all OHLCV vectors.
6. **Add measurement fields that make validation-cost claims honest.** The existing validation-cost harness is the right evidence boundary, but it does not currently expose grid-expansion ratio, auxiliary finalist analytics time, or range-batch counts.

---

## 1. Surface understanding

This validation surface is split across three layers:

- **Core Double-MA validation execution** in `double_ma_validate.rs`, including the current GPU-capable fixed-candidate main route plus CPU tails and follow-up analytics.
- **Shared validation reductions and statistics** in `validation_pipeline.rs`, including walk-forward aggregation, CSCV/PBO, SPA, perturb, bootstrap, run verdicts, and probability metrics.
- **Application-level orchestration** in `commands_validation.rs`, `validation_common.rs`, and `validationRequestBuilder.ts`, which shapes cross-ticker reruns, request payloads, user-facing route summaries, and measurement surfaces.

From the supplied code, the current GPU path works roughly like this:

1. `run_double_ma_validation_blocking_with_candles(...)` tries to prepare a GPU fixed-candidate validation plan.  
   Evidence: `double_ma_validate.rs::run_double_ma_validation_blocking_with_candles`, `::try_prepare_gpu_fixed_candidate_validation`.
2. `try_prepare_gpu_fixed_candidate_validation(...)` enforces uniform MA types, strategy config, halftrend config, and MA param maps across candidates, computes per-candidate `start_t`, builds unique `fast_periods` and `slow_periods`, and then calls `gpu_fixed_candidate_metric_map(...)`.  
   Evidence: `double_ma_validate.rs::try_prepare_gpu_fixed_candidate_validation`.
3. `gpu_fixed_candidate_metric_map(...)` does **not** execute exact finalist pairs. It routes through GPU optimization-style entrypoints using the unique fast and slow period lists, then reconstructs a host `HashMap<(u16,u16), Metrics>`.  
   Evidence: `double_ma_validate.rs::gpu_fixed_candidate_metric_map`.
4. Walk-forward, CSCV, SPA, and GPU reopt each call back into the GPU metric map one range at a time, then rebuild host maps or clone metrics into new host structures.  
   Evidence: `double_ma_validate.rs::compute_gpu_walk_forward_stats_for_candidates`, `::compute_gpu_cscv_slice_stats_for_candidates`, `::compute_gpu_spa_slice_sums_for_candidates`, `::compute_gpu_reopt_walk_forward_stats_for_candidates`.
5. After candidate results are produced, the validation pass unconditionally computes `regime_slice_robustness` and `friction_stress` for top finalists.  
   Evidence: `double_ma_validate.rs::run_double_ma_validation_blocking_with_candles` after the `run_validation_candidates(...)` call.
6. Cross-ticker reruns are orchestrated in `commands_validation.rs`, with preserved-finalists mode described as keeping only the verdict-relevant stack, but the base validation function still performs unconditional post-pass finalist analytics.  
   Evidence: `commands_validation.rs::apply_cross_ticker_validation_double_ma`, `validationRequestBuilder.ts::buildValidationCapabilitySummary`.

That combination means the current throughput cliff is **not** just “GPU vs CPU.” It is:

- GPU main route over a larger-than-needed candidate shape,
- repeated GPU re-entry for many evaluation ranges,
- plus CPU-only follow-up modules,
- plus hidden CPU finalist analytics that are not fully reflected in route summaries.

---

## 2. Findings

### Finding 1 — fixed-candidate GPU validation still executes grid-shaped work

- **Severity:** critical
- **Type:** performance / benchmark-fidelity
- **Location:** `double_ma_validate.rs::gpu_fixed_candidate_metric_map` (approx. lines 148-367), `double_ma_validate.rs::try_prepare_gpu_fixed_candidate_validation` (approx. lines 922-1166)
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:**
  - The GPU path is advertised and used as fixed-candidate validation.
  - In practice, the preparation step collects unique `fast_periods` and `slow_periods`, then routes through optimization-style GPU entrypoints that evaluate a grid shape derived from those periods.
  - A sparse finalist set such as `(9, 60), (15, 95), (27, 140), (41, 180)` can become a much larger implicit period grid once `fast_periods` and `slow_periods` are expanded.
  - This is the most important CUDA-side waste in the supplied files because it multiplies device-side work, host-side result shaping, and benchmark misattribution.
- **Recommended change:**
  - Add a new **exact-combo fixed-candidate GPU backend entry** that accepts an explicit `combos: &[DoubleMaParams]` list rather than inferring work shape from the Cartesian product of `fast_periods × slow_periods`.
  - In `double_ma_validate.rs`, refactor the GPU prep path around a `PreparedGpuValidationPlan` that preserves combo order and exposes `evaluated_pairs` back to the caller.
  - Until the adjacent backend helper exists, keep a host-side dense-combo fallback that fills a row-major `[range][combo]` buffer from the current GPU map path. That does not solve the device-side overwork, but it removes some host overhead immediately.

**Implementation notes**

The adjacent CUDA/runtime files that likely need the new exact-combo helper are not attached here, but the current call sites make the required contract clear. The implementation should live behind the current GPU runtime boundary, probably in the existing Double-MA GPU runtime/helper layer used by `gpu_fixed_candidate_metric_map(...)`.

**Code-edit shape (new local plan object)**

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[derive(Clone)]
struct PreparedGpuValidationPlan {
    combo_pairs: Vec<(u16, u16)>,
    combos: Vec<ta_strategies::double_ma::DoubleMaParams>,
    pair_to_idx: std::collections::HashMap<(u16, u16), usize>,
    fast_periods: Vec<u16>,
    slow_periods: Vec<u16>,
    start_ts: Vec<usize>,
    fast_ma_type: String,
    slow_ma_type: String,
    first_fast_params: Option<std::collections::HashMap<String, f64>>,
    first_slow_params: Option<std::collections::HashMap<String, f64>>,
    first_strategy: StrategyConfig,
    first_halftrend: Option<HalfTrendRegimeFilter>,
    regime_filter: Option<Vec<f64>>,
    device_id: u32,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
}
```

**Adjacent backend contract to add**

```rust
// Adjacent backend/helper contract (likely in the current GPU runtime layer).
// This is not in the attached files, so treat the names as implementation shape.
pub struct FixedCandidateBatchRequest<'a> {
    pub candles: &'a vector_ta::utilities::data_loader::Candles,
    pub combos: &'a [ta_strategies::double_ma::DoubleMaParams],
    pub fast_periods: &'a [u16],
    pub slow_periods: &'a [u16],
    pub eval_ranges: &'a [(usize, usize)], // empty => full range
    pub fast_ma_type: &'a str,
    pub slow_ma_type: &'a str,
    pub ma_source: &'a str,
    pub first_fast_params: Option<&'a std::collections::HashMap<String, f64>>,
    pub first_slow_params: Option<&'a std::collections::HashMap<String, f64>>,
    pub regime_filter: Option<&'a [f64]>,
    pub strategy: &'a StrategyConfig,
    pub halftrend: Option<&'a HalfTrendRegimeFilter>,
    pub device_id: u32,
    pub fast_alma_offset: f64,
    pub fast_alma_sigma: f64,
    pub slow_alma_offset: f64,
    pub slow_alma_sigma: f64,
}

pub struct FixedCandidateBatchResult {
    pub metrics: Vec<ta_strategies::double_ma::Metrics>, // row-major [range][combo]
    pub backend_used: BackendUsed,
    pub backend_route: Option<String>,
    pub backend_fallback_reason: Option<String>,
    pub evaluated_pairs: usize,
}
```

**Immediate host-side cleanup even before the backend helper exists**

```rust
#[derive(Clone)]
struct DenseRangeMetrics {
    ranges: Vec<(usize, usize)>,
    combo_pairs: Vec<(u16, u16)>,
    metrics: Vec<Metrics>, // row-major [range_idx * combo_count + combo_idx]
}

impl DenseRangeMetrics {
    #[inline]
    fn combo_count(&self) -> usize {
        self.combo_pairs.len()
    }

    #[inline]
    fn idx(&self, range_idx: usize, combo_idx: usize) -> usize {
        range_idx * self.combo_count() + combo_idx
    }

    #[inline]
    fn metric(&self, range_idx: usize, combo_idx: usize) -> &Metrics {
        &self.metrics[self.idx(range_idx, combo_idx)]
    }
}
```

**Why this matters for copy-elision**

The dense row-major buffer can safely use **uninitialized allocation** if every slot is guaranteed to be written before any read. This is a good fit for the new exact-combo multi-range result buffer and a bad fit for buffers whose default contents are semantically observed.

- **Safe uninitialized candidate:** dense `[range][combo]` metric buffer.
- **Not safe here:** `spread_ratio` initialized with `NaN`, mask arrays initialized with `false`, state arrays initialized with `u8::MAX`, or any prefix array whose fill value is observable before overwrite.

Prefer the existing shared allocation helpers in the repo’s `alloc_policy` module if they already expose an uninitialized-vector helper. If not, add a single helper there rather than scattering local unsafe code across validation files.

---

### Finding 2 — GPU validation re-enters the metric path once per range and rebuilds host maps every time

- **Severity:** high
- **Type:** performance
- **Location:** `double_ma_validate.rs::compute_gpu_walk_forward_stats_for_candidates`, `::compute_gpu_cscv_slice_stats_for_candidates`, `::compute_gpu_spa_slice_sums_for_candidates`, `::compute_gpu_reopt_walk_forward_stats_for_candidates`
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:**
  - The same exact-candidate set is evaluated over many ranges.
  - Each range currently calls `gpu_fixed_candidate_metric_map(...)` separately.
  - Each of those calls rebuilds a `HashMap<(u16, u16), Metrics>` on the host and then clones or looks up results again.
  - The GPU runtime may reuse price data residency, but the validation layer still pays repeated host orchestration and result materialization costs.
- **Recommended change:**
  - Build a **range schedule once**.
  - Fill a dense row-major result buffer once.
  - Consume that buffer for walk-forward, CSCV, SPA, and reopt instead of rebuilding `HashMap<usize, HashMap<(u16,u16), Metrics>>` or per-slice maps.
  - Stage 1: use the current map-returning GPU call internally but immediately flatten into dense storage.
  - Stage 2: replace it with a true multi-range exact-combo backend call.

**Code-edit shape (range schedule + dense fill)**

```rust
fn collect_walk_forward_ranges(
    n: usize,
    start_ts: &[usize],
    cfg: &WalkForwardConfigOptional,
) -> (Vec<Vec<usize>>, Vec<(usize, usize)>) {
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);

    let mut candidate_windows: Vec<Vec<usize>> = Vec::with_capacity(start_ts.len());
    let mut starts: Vec<usize> = Vec::new();

    for &start_t in start_ts {
        let mut windows = Vec::new();
        let mut t0 = start_t;
        while t0 + 2 <= n {
            let end = t0.saturating_add(window).min(n);
            if end <= t0 + 1 {
                break;
            }
            windows.push(t0);
            starts.push(t0);
            if end == n {
                break;
            }
            t0 = t0.saturating_add(step);
        }
        candidate_windows.push(windows);
    }

    starts.sort_unstable();
    starts.dedup();
    let ranges = starts.into_iter().map(|t0| (t0, t0.saturating_add(window).min(n))).collect();
    (candidate_windows, ranges)
}
```

**Dense fill from the current GPU map path**

```rust
fn build_dense_metrics_with_current_gpu_map(
    req: &DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    plan: &PreparedGpuValidationPlan,
    ranges: &[(usize, usize)],
) -> Result<DenseRangeMetrics, String> {
    let combo_count = plan.combo_pairs.len();
    let mut metrics = Vec::with_capacity(ranges.len().saturating_mul(combo_count));

    for &(start, end) in ranges {
        let (metric_map, ..) = gpu_fixed_candidate_metric_map(
            req,
            candles,
            &plan.fast_periods,
            &plan.slow_periods,
            &plan.fast_ma_type,
            &plan.slow_ma_type,
            plan.first_fast_params.as_ref(),
            plan.first_slow_params.as_ref(),
            plan.regime_filter.as_deref(),
            &plan.first_strategy,
            &plan.first_halftrend,
            &plan.combos,
            plan.device_id,
            plan.fast_alma_offset,
            plan.fast_alma_sigma,
            plan.slow_alma_offset,
            plan.slow_alma_sigma,
            Some((start, end)),
        )?;

        for &pair in &plan.combo_pairs {
            let metric = metric_map.get(&pair).cloned().ok_or_else(|| {
                format!(
                    "missing GPU metrics for pair ({}, {}) in range {}..{}",
                    pair.0, pair.1, start, end
                )
            })?;
            metrics.push(metric);
        }
    }

    Ok(DenseRangeMetrics {
        ranges: ranges.to_vec(),
        combo_pairs: plan.combo_pairs.clone(),
        metrics,
    })
}
```

**Micro-optimization to include now**

The current `gpu_fixed_candidate_metric_map(...)` clones `Metrics` out of `res.all.as_ref()` even though `res` is owned. Move metrics out of the GPU result instead of cloning them.

```rust
let res = crate::double_ma::run_double_ma_optimization_blocking_with_candles(...)?;
let backend_used = res.backend_used;
let backend_route = res.backend_route.clone();
let backend_fallback_reason = res.backend_fallback_reason.clone();
let mut metric_by_pair = std::collections::HashMap::with_capacity(combos.len());
if let Some(all) = res.all {
    for (params, metric) in all {
        metric_by_pair.insert((params.fast_len, params.slow_len), metric);
    }
}
```

That is a small but real host-side copy reduction.

---

### Finding 3 — the full-range GpuHybridKernel validation route still goes through the optimizer-shaped API

- **Severity:** medium
- **Type:** performance / maintainability
- **Location:** `double_ma_validate.rs::gpu_fixed_candidate_metric_map` full-range `Backend::GpuHybridKernel` branch
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:**
  - The full-range hybrid path constructs a synthetic `DoubleMaRequest` and calls `run_double_ma_optimization_blocking_with_candles(...)`.
  - That means validation inherits optimizer-oriented request shaping (`top_k`, `include_all`, grid ranges, route notes) instead of a slimmer fixed-candidate validation boundary.
  - It makes it harder to expose true evaluated-pair counts and harder to reason about validation cost independently of optimizer shape.
- **Recommended change:**
  - Once the exact-combo backend entry exists, use it for both `eval_range: None` and `eval_range: Some(...)`.
  - Remove the full-range optimizer-shaped detour from the validation path.

**Code-edit direction**

- Replace the full-range `run_double_ma_optimization_blocking_with_candles(...)` branch in `gpu_fixed_candidate_metric_map(...)` with the new fixed-candidate batch API.
- Preserve the current `backend_used`, `backend_route`, and `backend_fallback_reason` fields.
- Add `evaluated_pairs` to the optional execution profile so the validation-cost harness can report the true work shape.

---

### Finding 4 — `candidate_start_t` recomputes shared regime and Kaufman-stop work per candidate even though the GPU path already requires uniform configs

- **Severity:** high
- **Type:** performance
- **Location:** `double_ma_validate.rs::candidate_start_t`, `double_ma_validate.rs::try_prepare_gpu_fixed_candidate_validation`
- **Timing boundary affected:** app-path
- **Why it matters:**
  - The GPU fixed-candidate path rejects mixed `StrategyConfig`, mixed halftrend config, and mixed MA param maps.
  - Under those uniformity requirements, `compute_combined_regime_filter(...)` and `compute_kaufman_stop_series(...)` do not need to be recomputed once per candidate just to derive `start_t`.
  - That is avoidable CPU work on every GPU validation request with more than one finalist.
- **Recommended change:**
  - Precompute a single shared `regime_first_valid` and `kaufman_first_valid` for the validated uniform strategy/filter bundle.
  - Compute each candidate’s `start_t` from `base_first_valid`, MA warmup, trade offset, and those shared first-valid indices only.

**Code-edit shape**

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[derive(Clone, Debug, Default)]
struct SharedStartGate {
    regime_first_valid: Option<usize>,
    kaufman_first_valid: Option<usize>,
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_shared_start_gate(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    strategy: &StrategyConfig,
    halftrend: Option<&HalfTrendRegimeFilter>,
) -> Result<(SharedStartGate, Option<Vec<f64>>), String> {
    let n = candles.close.len();
    let mut gate = SharedStartGate::default();

    let regime_filter = compute_combined_regime_filter(candles, ma_source, strategy, halftrend)?;
    if let Some(trend) = regime_filter.as_ref() {
        if trend.len() != n {
            return Err("regime trend series length mismatch".to_string());
        }
        gate.regime_first_valid = Some(
            first_finite_idx(trend)
                .ok_or_else(|| "regime trend series all non-finite".to_string())?,
        );
    }

    if let Some(ks) = crate::double_ma::compute_kaufman_stop_series(candles, strategy)?.as_ref() {
        let fv = first_finite_idx(&ks.long)
            .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?
            .max(
                first_finite_idx(&ks.short)
                    .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?,
            );
        gate.kaufman_first_valid = Some(fv);
    }

    Ok((gate, regime_filter))
}

#[inline]
fn candidate_start_t_from_shared(
    base_first_valid: usize,
    params: &DoubleMaValidationParams,
    gate: &SharedStartGate,
) -> usize {
    let trade_offset = if params.strategy.trade_on_next_bar { 1usize } else { 0usize };
    let ma_warmup = base_first_valid
        .saturating_add((params.fast_len as usize).max(params.slow_len as usize).saturating_sub(1))
        .saturating_add(trade_offset);

    ma_warmup
        .max(gate.regime_first_valid.unwrap_or(0))
        .max(gate.kaufman_first_valid.unwrap_or(0))
}
```

This is a safe, parity-preserving CPU reduction and should land early.

---

### Finding 5 — unconditional finalist analytics still run after the main candidate pass

- **Severity:** high
- **Type:** performance / route-fidelity
- **Location:** `double_ma_validate.rs::run_double_ma_validation_blocking_with_candles`, `commands_validation.rs::apply_cross_ticker_validation_double_ma`, `validationRequestBuilder.ts::buildValidationCapabilitySummary`
- **Timing boundary affected:** app-path / validation-cost / route-summary
- **Why it matters:**
  - After candidate results are computed, the base validation function always evaluates:
    - `regime_slice_robustness` for up to `DEFAULT_REGIME_SLICE_FINALISTS`
    - `friction_stress` for up to `DEFAULT_FRICTION_STRESS_FINALISTS`
  - Preserved-finalists cross reruns are described in the command layer and frontend summaries as a verdict-relevant subset that omits walk-forward, reopt, SPA, perturb, and trade bootstrap.
  - Those summaries do not reflect the unconditional post-pass finalist analytics that still run today.
- **Recommended change:**
  - Introduce an internal **auxiliary execution mask**.
  - Default base validation to `full()`.
  - Use `cross_preserved_finalists()` when cross reruns only need verdicts and cross metrics.
  - Measure those auxiliary blocks separately and expose them in `ValidationExecutionProfile` and the validation-cost harness.

**Code-edit shape**

```rust
#[derive(Clone, Copy, Debug)]
struct ValidationAuxWorkMask {
    regime_slice_robustness: bool,
    friction_stress: bool,
}

impl ValidationAuxWorkMask {
    fn full() -> Self {
        Self {
            regime_slice_robustness: true,
            friction_stress: true,
        }
    }

    fn cross_preserved_finalists() -> Self {
        Self {
            regime_slice_robustness: false,
            friction_stress: false,
        }
    }
}
```

**Suggested integration point in `run_double_ma_validation_blocking_with_candles(...)`**

```rust
pub fn run_double_ma_validation_blocking_with_candles_with_mask(
    mut req: DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
    aux_mask: ValidationAuxWorkMask,
) -> Result<DoubleMaValidationResult, String> {
    // existing body with the unconditional finalist analytics replaced by masked blocks
}

pub fn run_double_ma_validation_blocking_with_candles(
    req: DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<DoubleMaValidationResult, String> {
    run_double_ma_validation_blocking_with_candles_with_mask(
        req,
        candles,
        cancel,
        progress,
        ValidationAuxWorkMask::full(),
    )
}
```

**Command-layer use for preserved-finalists cross reruns**

Because `commands_validation.rs` already deliberately strips non-gating modules from preserved-finalists cross reruns, it should call the masked entry so the backend runtime matches the described rerun shape.

---

### Finding 6 — `friction_stress` rebuilds full candidate series for every finalist and every stress scenario

- **Severity:** high
- **Type:** performance
- **Location:** `double_ma_validate.rs::compute_friction_stress_summaries`, `double_ma_validate.rs::build_candidate_series`
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:**
  - `friction_stress` currently builds the base series once per finalist and then rebuilds another full series for each stress scenario.
  - The stressed scenarios only change `fee_bps`, `spread_bps`, and `slippage_bps`.
  - Those fields do not change the MA vectors, halftrend regime filter, or Kaufman-stop series.
- **Recommended change:**
  - Build the base `CandidateSeries` once per finalist.
  - Add `eval_metrics_window_with_cfg(...)` so the same signal/state series can be re-evaluated under modified cost config without rebuilding indicators.
  - Reuse a local `SeriesCache` for any post-pass modules that still need the same base finalist series.

**Code-edit shape: series cache**

```rust
struct SeriesCache<'a> {
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    base_first_valid: usize,
    ma_source: &'a str,
    fast_ma_params_req: Option<&'a std::collections::HashMap<String, f64>>,
    slow_ma_params_req: Option<&'a std::collections::HashMap<String, f64>>,
    by_key: std::collections::HashMap<String, CandidateSeries<'a>>,
}

impl<'a> SeriesCache<'a> {
    fn new(
        candles: &'a vector_ta::utilities::data_loader::Candles,
        lr: &'a [f64],
        base_first_valid: usize,
        ma_source: &'a str,
        fast_ma_params_req: Option<&'a std::collections::HashMap<String, f64>>,
        slow_ma_params_req: Option<&'a std::collections::HashMap<String, f64>>,
    ) -> Self {
        Self {
            candles,
            lr,
            base_first_valid,
            ma_source,
            fast_ma_params_req,
            slow_ma_params_req,
            by_key: std::collections::HashMap::new(),
        }
    }

    fn get_or_build(
        &mut self,
        params: &DoubleMaValidationParams,
    ) -> Result<&CandidateSeries<'a>, String> {
        let key = serde_json::to_string(params).unwrap_or_else(|_| {
            format!(
                "{}:{}:{}:{}",
                params.fast_ma_type,
                params.fast_len,
                params.slow_ma_type,
                params.slow_len
            )
        });
        if !self.by_key.contains_key(&key) {
            let built = build_candidate_series(
                self.candles,
                self.lr,
                self.base_first_valid,
                self.ma_source,
                self.fast_ma_params_req,
                self.slow_ma_params_req,
                params,
            )?;
            self.by_key.insert(key.clone(), built);
        }
        Ok(self.by_key.get(&key).expect("series cache entry"))
    }
}
```

**Code-edit shape: cost-only re-evaluation**

```rust
#[inline]
fn eval_metrics_window_with_cfg(
    series: &CandidateSeries<'_>,
    start_t: usize,
    end_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    let n = series.n;
    let end_t = end_t.min(n);
    if end_t < 2 || start_t >= end_t {
        return Metrics::default();
    }

    let lr = &series.lr[..end_t];
    let close = &series.close[..end_t];
    let fast = &series.fast[..end_t];
    let slow = &series.slow[..end_t];
    let ht2 = series.halftrend_trend.as_ref().map(|h| &h[..end_t]);

    crate::double_ma::eval_metrics_f64_with_exit_mode(
        lr,
        close,
        fast,
        slow,
        ht2,
        start_t,
        cfg,
        series.kaufman_stops.as_ref(),
        true,
    )
}
```

**Code-edit shape: friction summary rewrite**

```rust
fn compute_friction_stress_summaries(
    candles: &vector_ta::utilities::data_loader::Candles,
    lr: &[f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params_req: Option<&std::collections::HashMap<String, f64>>,
    slow_ma_params_req: Option<&std::collections::HashMap<String, f64>>,
    finalists: &[DoubleMaValidationParams],
) -> Result<Vec<Option<FrictionStressStats>>, String> {
    if finalists.is_empty() {
        return Ok(Vec::new());
    }

    let mut cache = SeriesCache::new(
        candles,
        lr,
        base_first_valid,
        ma_source,
        fast_ma_params_req,
        slow_ma_params_req,
    );

    let mut base_series = Vec::with_capacity(finalists.len());
    for params in finalists {
        base_series.push(cache.get_or_build(params)?.clone());
    }

    // base ranking
    let mut base_scored: Vec<(usize, Metrics, f64)> = Vec::with_capacity(finalists.len());
    for (idx, series) in base_series.iter().enumerate() {
        let metrics = eval_metrics_window(series, series.start_t, series.n);
        let score = double_ma_validation_robust_score(&metrics, None);
        base_scored.push((idx, metrics, score));
    }

    // scenario ranking: no series rebuilds, only cfg changes
    // ... same ranking logic as today but using eval_metrics_window_with_cfg(...)

    todo!("replace current repeated build_candidate_series(...) calls with cfg-only re-evaluation");
}
```

This is one of the safest and highest-value CPU-tail optimizations in the supplied files.

---

### Finding 7 — `regime_slice_robustness` allocates and rescans more than necessary

- **Severity:** medium
- **Type:** performance / allocation-discipline
- **Location:** `double_ma_validate.rs::compute_regime_slice_robustness_stats`, `::compute_step_returns_for_series`, `::rolling_volatility_tertiles`, `::summarize_regime_bucket`
- **Timing boundary affected:** app-path
- **Why it matters:**
  - This block only runs for a few finalists, so it is not the first optimization target.
  - But it currently allocates several `Vec`s of full candle length:
    - `step_returns`
    - `active_flags`
    - `trade_entries`
    - `spread_ratio`
    - five full-length boolean masks
    - plus `states`, `prefix`, `prefix_sq`, and `raw` in the volatility helper
  - It also rescans the full length several times through `summarize_regime_bucket(...)`.
- **Recommended change:**
  - Keep the percentile/tertile logic, but remove the five full-length mask arrays.
  - Use a two-pass approach:
    1. compute thresholds
    2. stream once through the series and update five bucket accumulators directly
  - Do **not** use uninitialized allocation here for arrays whose sentinel values are behaviorally meaningful (`NaN`, `false`, `u8::MAX`).

**Sketch of the lower-allocation accumulator approach**

```rust
#[derive(Default, Clone)]
struct BucketAccum {
    bars: usize,
    active_bars: usize,
    trades: usize,
    eq: f64,
    peak: f64,
    max_dd: f64,
    ulcer_sum_sq: f64,
    sum_neg_sq: f64,
    samples: Vec<f64>,
}

fn update_bucket(acc: &mut BucketAccum, step_r: f64, active: bool, trade_entry: bool) {
    acc.bars += 1;
    if active {
        acc.active_bars += 1;
    }
    if trade_entry {
        acc.trades += 1;
    }
    acc.eq *= (1.0 + step_r).max(1.0e-12);
    if acc.eq > acc.peak {
        acc.peak = acc.eq;
    }
    let dd = (acc.peak - acc.eq) / acc.peak.max(1.0e-12);
    if dd > acc.max_dd {
        acc.max_dd = dd;
    }
    acc.ulcer_sum_sq += dd * dd;
    if step_r < 0.0 {
        acc.sum_neg_sq += step_r * step_r;
    }
    acc.samples.push(step_r);
}
```

This is a worthwhile second-pass optimization once the GPU work-shape issues are addressed.

---

### Finding 8 — CPU follow-up work rebuilds finalist series repeatedly because there is no post-pass series cache

- **Severity:** medium
- **Type:** performance
- **Location:** `double_ma_validate.rs::run_double_ma_validation_blocking_with_candles`, `::compute_trade_bootstrap_for_candidate`, `::compute_cpu_perturb_stats_for_candidate`
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:**
  - The non-GPU path already reuses `series` inside the candidate closure for several modules.
  - After the main pass finishes, post-pass modules rebuild finalists again.
  - Even on the GPU path, CPU-tail follow-up work still reconstructs series on demand.
- **Recommended change:**
  - Use the `SeriesCache` pattern for post-pass finalist modules.
  - Keep perturb as a separate CPU-tail path, because perturbed candidates change the strategy config and may require new series.
  - Reuse the cache for:
    - `regime_slice_robustness`
    - `friction_stress`
    - CPU SPA fallback
    - CPU reopt-walk-forward finalist prep
    - trade bootstrap when using the base candidate rather than a perturbed candidate

**Practical note**

This is especially important after the GPU main path gets faster, because otherwise the validation wall-clock shifts into repeated CPU series rebuilds that were previously hidden by GPU route overhead.

---

### Finding 9 — `compute_trade_bootstrap_stats` allocates a new stationary index vector on every bootstrap simulation

- **Severity:** medium
- **Type:** performance / allocation-discipline
- **Location:** `validation_pipeline.rs::compute_trade_bootstrap_stats`, `validation_pipeline.rs::stationary_bootstrap_indices`
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:**
  - For stationary bootstrap mode, every simulation allocates a new `Vec<usize>` from `stationary_bootstrap_indices(...)`.
  - This sits inside the innermost bootstrap loop.
  - The code is deterministic now; the optimization should keep that property.
- **Recommended change:**
  - Add a reusable `TradeBootstrapScratch` struct.
  - Change `stationary_bootstrap_indices(...)` into `stationary_bootstrap_indices_into(...)`.
  - Keep the public wrapper for compatibility and measurement parity.

**Code-edit shape**

```rust
#[derive(Default)]
pub struct TradeBootstrapScratch {
    indices: Vec<usize>,
    pnls: Vec<f64>,
    dds: Vec<f64>,
    stationary_pnls: Vec<f64>,
    stationary_dds: Vec<f64>,
}

fn stationary_bootstrap_indices_into(
    n: usize,
    mean_block: usize,
    rng: &mut StdRng,
    out: &mut Vec<usize>,
) {
    out.clear();
    if n == 0 {
        return;
    }
    out.reserve(n.saturating_sub(out.capacity()));
    let restart_prob = (1.0 / mean_block.max(1) as f64).clamp(1e-6, 1.0);
    let mut idx = rng.gen_range(0..n);
    out.push(idx);
    while out.len() < n {
        if rng.gen::<f64>() < restart_prob {
            idx = rng.gen_range(0..n);
        } else {
            idx = (idx + 1) % n;
        }
        out.push(idx);
    }
}
```

**Wrapper-preserving public API**

```rust
pub fn compute_trade_bootstrap_stats(
    trades: &[Trade],
    cfg: &TradeBootstrapConfig,
    rng: &mut StdRng,
) -> TradeBootstrapStats {
    let mut scratch = TradeBootstrapScratch::default();
    compute_trade_bootstrap_stats_with_scratch(trades, cfg, rng, &mut scratch)
}
```

This is a clean low-risk allocation reduction.

---

### Finding 10 — cross-ticker alignment always clones OHLCV arrays, even when the aligned window is the full source range

- **Severity:** medium
- **Type:** performance / copy-elision
- **Location:** `commands_validation.rs::slice_candles`, `::align_cross_ticker_candles_to_base_horizon`
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:**
  - `align_cross_ticker_candles_to_base_horizon(...)` always returns an owned `Candles`.
  - Even when the cross dataset already matches the desired range (`start == 0 && end == cross_len`), the code still clones six vectors through `slice_candles(...)`.
  - This is a pure copy boundary with no computational value.
- **Recommended change:**
  - Return a borrowed-or-owned wrapper.
  - Only allocate and copy when a true subrange slice is needed.
  - Keep the current `slice_candles(...)` path for real subrange alignment.

**Code-edit shape**

```rust
enum AlignedCandles<'a> {
    Borrowed(&'a vector_ta::utilities::data_loader::Candles),
    Owned(vector_ta::utilities::data_loader::Candles),
}

impl<'a> AlignedCandles<'a> {
    fn as_ref(&self) -> &vector_ta::utilities::data_loader::Candles {
        match self {
            Self::Borrowed(c) => c,
            Self::Owned(c) => c,
        }
    }
}

fn align_cross_ticker_candles_to_base_horizon<'a>(
    base: &vector_ta::utilities::data_loader::Candles,
    cross: &'a vector_ta::utilities::data_loader::Candles,
) -> AlignedCandles<'a> {
    let base_len = base.close.len();
    let cross_len = cross.close.len();
    if base_len == 0 || cross_len == 0 {
        return AlignedCandles::Owned(slice_candles(cross, 0, cross_len));
    }

    let base_start = *base.timestamp.first().unwrap_or(&0);
    let base_end = *base.timestamp.last().unwrap_or(&0);
    let start = cross.timestamp.partition_point(|&ts| ts < base_start);
    let end = cross.timestamp.partition_point(|&ts| ts <= base_end);

    if start == 0 && end == cross_len {
        return AlignedCandles::Borrowed(cross);
    }
    if start < end {
        return AlignedCandles::Owned(slice_candles(cross, start, end));
    }
    if cross_len > base_len {
        return AlignedCandles::Owned(slice_candles(
            cross,
            cross_len.saturating_sub(base_len),
            cross_len,
        ));
    }
    AlignedCandles::Borrowed(cross)
}
```

**Call-site change**

```rust
let aligned = align_cross_ticker_candles_to_base_horizon(base_candles, &outcome.candles);
datasets_meta.push(ta_app_core::CrossTickerDatasetMeta {
    label: validation_common::dataset_label(csv_path, &ds.label),
    csv_path: csv_path.to_string(),
    bars: aligned.as_ref().close.len(),
});
let res2 = run_validate(req2, aligned.as_ref(), cancel, None)?;
```

This is a direct unnecessary-copy removal.

---

### Finding 11 — cross-ticker candidate matching still depends on canonical JSON serialization for identity

- **Severity:** medium
- **Type:** performance / maintainability
- **Location:** `commands_validation.rs::apply_cross_ticker_validation_generic`, `validation_common.rs::candidate_key_canonical`, `validation_common.rs::canonicalize_json_value`
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:**
  - Base candidates are keyed by canonical JSON strings.
  - Every rerun candidate also reserializes params to canonical JSON for matching.
  - This is not the dominant bottleneck today, but it becomes more visible once the GPU main route and cross preserved-finalists reruns get faster.
- **Recommended change:**
  - Phase 1: keep the generic canonical fallback, but sort `acc.trades` in place and tighten the obvious host-side costs in `validation_common.rs`.
  - Phase 2: add an optional **stable candidate key** field or trait method so matching does not require JSON canonicalization when the caller can supply an ID.

**Low-risk immediate edit in `validation_common.rs`**

```rust
pub fn finalize_cross_ticker_candidates<C: CrossTickerCandidate>(
    candidates: &mut [C],
    mut accs: Vec<CrossTickerAcc>,
) {
    for (i, cand) in candidates.iter_mut().enumerate() {
        let Some(acc) = accs.get_mut(i) else {
            continue;
        };
        let ds_n = acc.sharpes.len();
        if ds_n == 0 {
            continue;
        }

        acc.sharpes.sort_by(|a, b| a.total_cmp(b));
        acc.dds.sort_by(|a, b| a.total_cmp(b));
        acc.pnls.sort_by(|a, b| a.total_cmp(b));
        acc.trades.sort_unstable();

        let trades_median = acc.trades[acc.trades.len() / 2];
        let trades_worst = *acc.trades.first().unwrap_or(&0);

        let sharpe_median = median(&acc.sharpes);
        let max_dd_worst = worst_max(&acc.dds);
        let score = sharpe_median - 2.0 * max_dd_worst;
        cand.set_cross_ticker(CrossTickerStats {
            datasets: ds_n,
            sharpe_median,
            sharpe_worst: worst_min(&acc.sharpes),
            max_dd_median: median(&acc.dds),
            max_dd_worst,
            pnl_median: median(&acc.pnls),
            pnl_worst: worst_min(&acc.pnls),
            trades_median,
            trades_worst,
            pnl_positive_frac: acc.pnl_pos as f64 / ds_n as f64,
            score,
        });
        // ... rest unchanged
    }
}
```

That removes the extra clone/sort currently hidden in `median_u32(xs.to_vec())`.

**Higher-payoff follow-up (requires adjacent type support)**

Add a stable candidate key in the validation candidate types so cross-ticker matching does not rely on canonical JSON.

---

### Finding 12 — frontend request building does not dedupe duplicate finalists before validation

- **Severity:** medium
- **Type:** performance / route-shaping
- **Location:** `validationRequestBuilder.ts::buildValidationRequest`
- **Timing boundary affected:** app-path / route-summary
- **Why it matters:**
  - The frontend builder slices `topRows` and maps them directly into `candidates`.
  - There is no dedupe pass.
  - Duplicate finalists can therefore trigger duplicated validation work on CPU or GPU.
  - The CLI validation-cost tool does dedupe candidate keys, so frontend and headless validation currently have different throughput shaping.
- **Recommended change:**
  - Add a stable-candidate-key dedupe pass in the frontend builder.
  - Include strategy/halftrend/MA-param state in the key, not just lengths.
  - Also dedupe cross datasets by normalized path on the frontend side to reduce request payload churn before the backend sanitizes them again.

**Code-edit shape (candidate dedupe)**

```ts
function stableJson(value: unknown): string {
  if (value === null || value === undefined) return 'null'
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`
  if (typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([k, v]) => `${JSON.stringify(k)}:${stableJson(v)}`)
    return `{${entries.join(',')}}`
  }
  return JSON.stringify(value)
}

function stableCandidateKey(candidate: ValidationCandidateLike): string {
  return [
    candidate.fast_ma_type,
    candidate.fast_len,
    candidate.slow_ma_type,
    candidate.slow_len,
    stableJson(candidate.fast_ma_params),
    stableJson(candidate.slow_ma_params),
    stableJson(candidate.strategy),
    stableJson(candidate.halftrend_filter ?? candidate.halftrend),
    stableJson(candidate.asr),
    stableJson(candidate.vcc),
    stableJson(candidate.vemr),
    stableJson(candidate.rocb),
    stableJson(candidate.mkcs),
    stableJson(candidate.kqsr),
    stableJson(candidate.carm),
    stableJson(candidate.kvdf),
    stableJson(candidate.cdmc),
    stableJson(candidate.vcet),
    stableJson(candidate.macz),
    stableJson(candidate.ott),
    candidate.signal_mode ?? '',
  ].join('|')
}
```

**Apply the dedupe after the existing validity filter**

```ts
const seenCandidateKeys = new Set<string>()
const candidates = candidatesRaw
  .map((row): ValidationCandidateLike | null => {
    // existing mapping logic
  })
  .filter((candidate): candidate is ValidationCandidateLike => {
    // existing validity checks
  })
  .filter((candidate) => {
    const key = stableCandidateKey(candidate)
    if (seenCandidateKeys.has(key)) return false
    seenCandidateKeys.add(key)
    return true
  })
```

**Cross-dataset dedupe**

```ts
function normalizeCrossDatasets(
  value: unknown,
  maxDatasets: number,
): Array<{ csv_path: string; label: string }> {
  if (maxDatasets <= 0 || !Array.isArray(value)) return []
  const out: Array<{ csv_path: string; label: string }> = []
  const seen = new Set<string>()
  for (const entry of value) {
    const row = asRecord(entry)
    if (!row) continue
    const csv_path = normalizeText(row.csv_path)
    if (!csv_path) continue
    const normalizedKey = csv_path.replaceAll('/', '\\').trim().toLowerCase()
    if (seen.has(normalizedKey)) continue
    seen.add(normalizedKey)
    out.push({ csv_path, label: normalizeText(row.label) })
    if (out.length >= maxDatasets) break
  }
  return out
}
```

This is a direct route-shaping improvement that can reduce validation work before it reaches Rust.

---

### Finding 13 — route summaries under-describe today’s preserved-finalists rerun work and should be updated once the aux mask lands

- **Severity:** medium
- **Type:** benchmark-fidelity / route-summary
- **Location:** `validationRequestBuilder.ts::buildValidationCapabilitySummary`, `validationRequestBuilder.test.ts`
- **Timing boundary affected:** route-summary / user-visible expectation shaping
- **Why it matters:**
  - The current summary for preserved-finalists double-like reruns says they keep CSCV/PBO + hard gates and omit walk-forward, SPA, reopt, perturb, and trade bootstrap.
  - That summary becomes accurate only after the backend mask actually disables unconditional post-pass finalist analytics.
  - Without that backend change, the frontend wording currently sounds lighter than the actual rerun path.
- **Recommended change:**
  - Land the backend aux mask first.
  - Then update the wording to explicitly say preserved-finalists reruns keep **CSCV/PBO + hard gates only** and omit the non-gating modules **including** `regime_slice_robustness` and `friction_stress`.

**Suggested summary text update**

```ts
warningNotes.push(
  'Preserved-finalists cross reruns keep CSCV/PBO + hard gates only; cross reruns omit walk-forward, SPA, reopt WF, perturb, trade bootstrap, regime-slice robustness, and friction stress.'
)
skippedNotes.push(
  'Cross reruns: preserved-finalists omit walk_forward + reopt_walk_forward + SPA + perturb + trade bootstrap + regime_slice_robustness + friction_stress'
)
```

**Tests to update**

- `validationRequestBuilder.test.ts::summarizes double-ma gpu preserved-finalists capability expectations`
- add a new duplicate-candidate-dedupe test
- add a new duplicate-cross-dataset-dedupe test

---

### Finding 14 — the validation-cost harness does not yet expose the most important work-shape fields

- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `ta_double_ma_validation_cost.rs`
- **Timing boundary affected:** validation-cost
- **Why it matters:**
  - This harness is the right current boundary for validation-cost claims.
  - But it currently reports candidate counts and wall times without exposing:
    - requested candidate count
    - unique fast periods
    - unique slow periods
    - evaluated GPU pairs
    - pair-expansion ratio
    - auxiliary finalist-analytics wall time
    - GPU range-eval call count
  - Without those, it is easy to claim improvement while still hiding grid-expansion or CPU-tail work.
- **Recommended change:**
  - Add an optional execution profile field to the validation result.
  - Thread those fields into the harness and print them in both JSON and stdout summaries.

**Suggested result profile**

```rust
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct ValidationExecutionProfile {
    pub requested_candidates: usize,
    pub unique_fast_periods: usize,
    pub unique_slow_periods: usize,
    pub evaluated_pairs: Option<usize>,
    pub pair_expansion_ratio: Option<f64>,
    pub gpu_range_eval_calls: Option<usize>,
    pub gpu_range_count: Option<usize>,
    pub aux_regime_slice_wall_ms: u64,
    pub aux_friction_stress_wall_ms: u64,
}
```

**Suggested harness additions**

```rust
#[derive(Serialize)]
struct ValidationCaseSummary {
    // existing fields...
    requested_candidates: Option<usize>,
    unique_fast_periods: Option<usize>,
    unique_slow_periods: Option<usize>,
    evaluated_pairs: Option<usize>,
    pair_expansion_ratio: Option<f64>,
    avg_aux_regime_slice_wall_ms: Option<u64>,
    avg_aux_friction_stress_wall_ms: Option<u64>,
    gpu_range_eval_calls: Option<usize>,
    gpu_range_count: Option<usize>,
}
```

**Why this should land before large CUDA refactors**

Because once exact-combo and multi-range batching land, this harness becomes the best place to prove the improvement at the correct timing boundary.

---

### Finding 15 — the Double-MA workflow Criterion bench is not the right boundary for CUDA validation claims yet

- **Severity:** low
- **Type:** benchmark-fidelity
- **Location:** `double_ma_validation_workflow.rs`, `composite_validation_workflow.rs`
- **Timing boundary affected:** Criterion vs validation-cost
- **Why it matters:**
  - `double_ma_validation_workflow.rs` is CPU-only in the supplied version.
  - `composite_validation_workflow.rs` does include GPU cases, but those are composite validation benches, not the primary Double-MA validation-cost evidence path.
  - The validation-cost CLI is the right current tool for Double-MA CUDA validation throughput.
- **Recommended change:**
  - Keep the current CPU Criterion bench as a CPU regression surface.
  - Add GPU sparse-finalist probes behind CUDA feature gates to both workflow benches, but treat them as supplementary evidence.
  - Use `ta_double_ma_validation_cost.rs` as the primary validation-cost proof for Double-MA CUDA work.

**Good additions to both benches**

- sparse-finalist candidate sets that make the grid-expansion ratio visible
- route probe printing that includes `evaluated_pairs` and `pair_expansion_ratio` if `execution_profile` is present
- preserved-finalists cross rerun coverage once the aux mask lands

---

### Open question — GPU CSCV and SPA slice helpers infer slice-level data from aggregate metrics

- **Severity:** medium
- **Type:** parity / benchmark-fidelity
- **Location:** `double_ma_validate.rs::compute_gpu_cscv_slice_stats_for_candidates`, `::compute_gpu_spa_slice_sums_for_candidates`
- **Timing boundary affected:** validation-cost / unknown
- **Why it matters:**
  - These helpers derive slice-level statistics from aggregate metrics (`sharpe`, `volatility`, etc.) rather than from explicit per-step slice returns.
  - That may be acceptable in practice, but it is also the kind of approximation that can produce parity drift on skewed return distributions.
- **Recommended change:**
  - Before heavily optimizing these paths, add a parity-focused validation-cost test case comparing GPU slice results against the CPU slice-return path on a representative dataset and a few strategy profiles.
  - If the approximation is good enough, keep it and optimize the orchestration around it.
  - If not, add a more direct slice-return output path in the adjacent GPU runtime.

---

## 3. Safe optimization opportunities

### Priority order

1. **Exact-combo GPU validation entry** with dense row-major host consumption.
2. **Multi-range batching** for walk-forward, CSCV, SPA, and reopt.
3. **Auxiliary execution mask** for preserved-finalists reruns.
4. **Shared start-gate precompute** for uniform GPU validation candidates.
5. **Series cache + cost-only re-evaluation** for `friction_stress` and other post-pass modules.
6. **Cross-ticker no-op alignment borrow** to eliminate needless OHLCV copying.
7. **Frontend candidate dedupe and cross-dataset dedupe** to reduce work before it hits Rust.
8. **Bootstrap scratch reuse** and in-place trade sorting.
9. **Micro cleanups**: move metrics instead of cloning, replace `BTreeSet` with `Vec + sort_unstable + dedup` where ordering only needs sorted uniqueness.

### Copy-elision and `MaybeUninit` rules for this surface

Use uninitialized allocation only where **every element is guaranteed to be written before any read**.

**Good candidates in the supplied files once refactored**

- Dense `[range][combo]` metric buffer for exact-combo GPU results.
- Dense per-scenario metrics buffer for friction-stress ranking if every slot is explicitly filled.

**Not good candidates here**

- `spread_ratio: vec![f64::NAN; n]` because `NaN` is meaningful sentinel state.
- `states: vec![u8::MAX; n]` because the sentinel is meaningful.
- `low_mask/high_mask/... = vec![false; n]` because the default is observed.
- Prefix arrays whose zero initialization is mathematically required and read before later positions are written.

**Rule of thumb for this bundle**

- Prefer **copy elimination** first.
- Then prefer **scratch reuse**.
- Then use **uninitialized allocation** only for fully overwritten dense buffers.

### Small safe micro-optimizations to apply opportunistically

- Move metrics out of owned GPU result containers instead of cloning them.
- Replace `BTreeSet` usage with `Vec + sort_unstable + dedup` where only sorted uniqueness is needed.
- Sort `acc.trades` in place in `validation_common.rs` and remove `median_u32(xs.to_vec())` cloning.
- Reuse scratch vectors in bootstrap and dense slice-stat helpers.

---

## 4. Higher-risk optimization opportunities

These are real opportunities, but I would land them only after the safe path above is measured.

### A. GPU-native perturb batching

- **Why risky:** perturb changes `StrategyConfig` per iteration, while the current GPU fixed-candidate path assumes a uniform strategy bundle.
- **Potential payoff:** large, if perturb remains expensive after the main GPU path is fixed.
- **Recommendation:** keep CPU-tail perturb for now; only revisit after exact-combo GPU validation and series-cache reductions are measured.

### B. GPU trade-sequence emission for bootstrap

- **Why risky:** current GPU validation returns aggregate metrics, not the closed-trade sequence required by bootstrap resampling.
- **Potential payoff:** moderate to high, depending on how large bootstrap workloads are in practice.
- **Recommendation:** treat this as a separate backend project after the validation-cost harness shows the remaining CPU bootstrap tail is worth pursuing.

### C. Direct device-side multi-slice CSCV / SPA outputs

- **Why risky:** parity details matter here, and the current helpers appear to infer slice stats from aggregate metrics.
- **Potential payoff:** moderate.
- **Recommendation:** gate it behind parity tests first.

### D. Cross-dataset parallel overlap for cross-ticker reruns

- **Why risky:** can complicate determinism, disk/cache behavior, and GPU residency.
- **Potential payoff:** uncertain until the rerun shape is cleaned up.
- **Recommendation:** keep cross reruns serial until the preserved-finalists rerun path is genuinely lightweight.

### E. Stable candidate IDs instead of canonical JSON keys

- **Why risky:** likely requires adjacent type/API changes across validation request/result types.
- **Potential payoff:** moderate host-side cleanup.
- **Recommendation:** worthwhile, but after the primary CUDA-path fixes.

---

## 5. Benchmark and perf-gate review

### Correct timing boundaries to use

- Use **`ta_double_ma_validation_cost.rs`** as the primary evidence path for Double-MA validation throughput.
- Use **Criterion workflow benches** as supplementary regression evidence.
- Do **not** mix GPU validation throughput claims with CPU-only workflow bench numbers.
- Treat route probes and fallback strings as supporting metadata, not as throughput proof.

### What to add to the validation-cost harness immediately

1. `requested_candidates`
2. `unique_fast_periods`
3. `unique_slow_periods`
4. `evaluated_pairs`
5. `pair_expansion_ratio`
6. `aux_regime_slice_wall_ms`
7. `aux_friction_stress_wall_ms`
8. `gpu_range_eval_calls`
9. `gpu_range_count`

### New benchmark cases to add

#### `double_ma_validation_workflow.rs`

- keep current CPU workflow coverage
- add CUDA-gated sparse-finalist bench
- add CUDA-gated preserved-finalists rerun bench

**Suggested sparse candidate generator**

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_sparse_gpu_candidates() -> Vec<DoubleMaValidationParams> {
    [(9u16, 60u16), (15, 95), (27, 140), (41, 180), (55, 220), (73, 260)]
        .into_iter()
        .map(|(fast_len, slow_len)| DoubleMaValidationParams {
            fast_ma_type: "alma".to_string(),
            slow_ma_type: "alma".to_string(),
            fast_len,
            slow_len,
            fast_ma_params: Some(std::collections::HashMap::from([
                ("offset".to_string(), 0.85),
                ("sigma".to_string(), 6.0),
            ])),
            slow_ma_params: Some(std::collections::HashMap::from([
                ("offset".to_string(), 0.85),
                ("sigma".to_string(), 6.0),
            ])),
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
        })
        .collect()
}
```

#### `composite_validation_workflow.rs`

- add one composite GPU sparse-profile case with route probe output including execution profile
- keep current strategy-family GPU coverage, but add work-shape output if `execution_profile` becomes available

### Perf-gate note

Do not promote the exact-combo or multi-range work as “done” until the validation-cost harness can show:

- lower `evaluated_pairs`
- lower `pair_expansion_ratio`
- lower `gpu_range_eval_calls`
- lower `avg_main_validation_wall_ms`
- unchanged route/fallback behavior
- unchanged parity tests

---

## 6. Test review

### Existing useful coverage already present

From the attached files, there is already good coverage for:

- GPU vs CPU parity on several validation shapes in `double_ma_validate.rs`
- GPU fallback reasons for unsupported validation cases in `double_ma_validate.rs`
- preserved GPU route behavior in cross preserved-finalists reruns in `commands_validation.rs`
- frontend summary wording and backend-expectation shaping in `validationRequestBuilder.test.ts`

### Tests to add before or with the main changes

#### `double_ma_validate.rs`

1. **Exact-combo sparse-finalist work-shape test**
   - Build a sparse finalist set.
   - Assert metrics match the current path.
   - Assert `execution_profile.evaluated_pairs <= requested_candidates` once the exact-combo backend lands.
   - If the backend still stages unique MA periods separately, assert at least that pair evaluation no longer inflates to the full cross grid.

2. **Preserved-finalists aux-mask omission test**
   - Call the masked validation entry.
   - Assert `regime_slice_robustness` and `friction_stress` are not populated.
   - Assert verdict and cross-ticker-required metrics are still present.

3. **Friction stress series-reuse parity test**
   - Compare old rebuild-per-scenario results against new `eval_metrics_window_with_cfg(...)` results.
   - Use a few strategies and fee/spread/slippage combinations.

4. **Shared `start_t` precompute parity test**
   - Compare old `candidate_start_t(...)` result vs `candidate_start_t_from_shared(...)` for several candidates sharing one strategy bundle.

5. **Dense-range batch parity test**
   - Compare current per-range GPU map aggregation vs dense range buffer result consumption for walk-forward, CSCV, SPA, and reopt.

#### `validation_pipeline.rs`

6. **Bootstrap scratch determinism test**
   - Assert `compute_trade_bootstrap_stats(...)` and `compute_trade_bootstrap_stats_with_scratch(...)` are identical with the same seed.

7. **Stationary bootstrap scratch reuse smoke test**
   - Assert repeated calls with reused scratch do not leak previous state.

#### `commands_validation.rs`

8. **No-op alignment borrow test**
   - When base and cross ranges already match, assert the borrowed path is used.
   - If pointer identity is awkward to test, assert that the enum variant is `Borrowed`.

9. **Preserved-finalists masked rerun route test**
   - Assert reruns preserve GPU route and also skip the non-gating aux analytics when the new masked entry is used.

#### `validation_common.rs`

10. **In-place trade sort parity test**
    - Assert cross-ticker stats are unchanged after replacing `median_u32(xs.to_vec())` with in-place trade sorting.

#### `validationRequestBuilder.test.ts`

11. **Duplicate validation-candidate dedupe test**
    - Provide repeated top rows.
    - Assert the request contains only one logical candidate.

12. **Duplicate cross-dataset dedupe test**
    - Provide repeated `csv_path` entries with path-case/slash variations.
    - Assert only one normalized dataset survives.

13. **Preserved-finalists summary wording test**
    - Update the expected warning/skipped summary once the backend aux mask lands.

#### `ta_double_ma_validation_cost.rs`

14. **Execution-profile field propagation test**
    - Assert JSON output includes `evaluated_pairs`, `pair_expansion_ratio`, and aux wall-time fields when present.

---

## 7. Suggested improvement plan

### Phase 0 — instrumentation and truthfulness

**Goal:** make the current path measurable before changing behavior.

1. Add `ValidationExecutionProfile` to `DoubleMaValidationResult`.
2. Record:
   - requested candidates
   - unique fast/slow periods
   - evaluated pairs if known
   - pair-expansion ratio if known
   - GPU range-eval call count if known
   - aux finalist analytics wall time
3. Thread those fields into `ta_double_ma_validation_cost.rs`.
4. Update route probe printing in the workflow benches to include them if present.

### Phase 1 — remove hidden work and redundant CPU prep

**Goal:** land the safest changes first.

1. Add `ValidationAuxWorkMask` and masked validation entry.
2. Use the full mask for normal runs.
3. Use the stripped mask for preserved-finalists cross reruns.
4. Precompute the shared start gate once for GPU fixed-candidate validation.
5. Move metrics out of owned GPU result containers instead of cloning.
6. Apply the no-op cross-alignment borrow in `commands_validation.rs`.
7. Sort cross-ticker trades in place in `validation_common.rs`.

### Phase 2 — exact-combo host reshaping and dense result consumption

**Goal:** reduce host-side overhead before touching the adjacent backend.

1. Introduce `PreparedGpuValidationPlan`.
2. Introduce `DenseRangeMetrics`.
3. Replace per-window/per-slice nested `HashMap` aggregation with dense row-major buffers.
4. Replace `BTreeSet` uniqueness builders with `Vec + sort_unstable + dedup` where appropriate.
5. Add parity tests for dense-range consumption.

### Phase 3 — exact-combo CUDA backend path

**Goal:** remove the biggest device-side work inflation.

1. Add an adjacent exact-combo fixed-candidate GPU helper.
2. Switch `gpu_fixed_candidate_metric_map(...)` callers over to that helper.
3. Remove the full-range optimizer-shaped detour for `GpuHybridKernel` validation.
4. Expose `evaluated_pairs` back through the execution profile.
5. Measure with `ta_double_ma_validation_cost.rs`.

### Phase 4 — multi-range batching

**Goal:** stop re-entering the GPU path once per range.

1. Add an adjacent multi-range exact-combo GPU evaluation entry.
2. Batch all walk-forward, CSCV, SPA, and reopt ranges into one prepared execution.
3. Fill `DenseRangeMetrics` directly from that result.
4. Measure `gpu_range_eval_calls` reduction and main validation wall time.

### Phase 5 — CPU-tail shrinkage

**Goal:** make the remaining CPU work proportionate.

1. Add `SeriesCache`.
2. Rewrite `compute_friction_stress_summaries(...)` to reuse series and only change cost config.
3. Reuse cached finalist series for post-pass modules.
4. Add `TradeBootstrapScratch` and `_with_scratch(...)` path.
5. Optionally streamline `regime_slice_robustness` with streaming accumulators if it still shows up in profiling.

### Phase 6 — request shaping and frontend alignment

**Goal:** stop creating duplicate work and align summaries with actual backend behavior.

1. Deduplicate validation candidates in `validationRequestBuilder.ts`.
2. Deduplicate cross datasets by normalized path in `validationRequestBuilder.ts`.
3. Update preserved-finalists summary wording once the aux mask lands.
4. Add tests covering those changes.

### Phase 7 — only then consider higher-risk GPU extensions

1. GPU-native perturb with non-uniform strategy batches.
2. GPU trade-sequence emission for bootstrap.
3. Direct slice-return device outputs for CSCV/SPA.
4. Cross-dataset parallel overlap.

---

## 8. File-by-file implementation checklist

### `double_ma_validate.rs`

- [ ] Add `ValidationAuxWorkMask`.
- [ ] Add `ValidationExecutionProfile` and optional `execution_profile` result field.
- [ ] Add `PreparedGpuValidationPlan`.
- [ ] Add `DenseRangeMetrics`.
- [ ] Refactor GPU prep to compute a shared start gate once.
- [ ] Replace per-candidate `candidate_start_t(...)` recomputation with `candidate_start_t_from_shared(...)`.
- [ ] Change GPU map consumers to dense row-major range buffers.
- [ ] Move owned GPU metrics instead of cloning them.
- [ ] Add the masked validation entry and wire the default entry through `ValidationAuxWorkMask::full()`.
- [ ] Apply the aux mask to preserved-finalists cross reruns.
- [ ] Add `SeriesCache`.
- [ ] Add `eval_metrics_window_with_cfg(...)`.
- [ ] Rewrite `compute_friction_stress_summaries(...)` to reuse series.
- [ ] Reuse cached series for post-pass finalist modules where possible.
- [ ] Optionally streamline `compute_regime_slice_robustness_stats(...)` if profiling still shows it.
- [ ] Add parity tests for exact-combo work shape, masked aux omissions, and friction-stress reuse.

### `validation_pipeline.rs`

- [ ] Add `TradeBootstrapScratch`.
- [ ] Add `stationary_bootstrap_indices_into(...)`.
- [ ] Add `compute_trade_bootstrap_stats_with_scratch(...)` and keep the current public wrapper.
- [ ] Verify deterministic parity with the current bootstrap implementation.
- [ ] Consider generic slice-row helpers only after the main GPU work-shape cleanup lands.

### `commands_validation.rs`

- [ ] Replace always-owned aligned candles with `Borrowed` vs `Owned` aligned candles.
- [ ] Update call sites to use `aligned.as_ref()`.
- [ ] Route preserved-finalists cross reruns through the masked validation entry.
- [ ] Keep current route/fallback behavior unchanged while reducing hidden aux work.
- [ ] Add tests for borrowed no-op alignment and masked preserved-finalists reruns.

### `validation_common.rs`

- [ ] Sort `acc.trades` in place and remove extra trade median cloning.
- [ ] Keep canonical JSON fallback for candidate identity.
- [ ] Optionally add a stable-key fast path later if adjacent types can supply it.
- [ ] Add parity test for unchanged cross-ticker stats.

### `validationRequestBuilder.ts`

- [ ] Deduplicate validation candidates after the current validity filter.
- [ ] Use a stable canonical key that includes MA params, strategy, filter, and strategy-local config.
- [ ] Deduplicate cross datasets by normalized path.
- [ ] Update preserved-finalists summary wording after the aux mask lands.
- [ ] Keep current backend-selection policy unchanged unless the backend path truly changes.

### `validationRequestBuilder.test.ts`

- [ ] Add duplicate-candidate dedupe coverage.
- [ ] Add duplicate-cross-dataset dedupe coverage.
- [ ] Update preserved-finalists summary assertions.
- [ ] Add summary coverage if route text starts mentioning exact-combo GPU validation explicitly.

### `ta_double_ma_validation_cost.rs`

- [ ] Add execution-profile fields to the JSON report.
- [ ] Print requested candidates, evaluated pairs, and expansion ratio in stdout summaries.
- [ ] Print auxiliary finalist-analytics wall time separately from main validation wall time.
- [ ] Add range-eval call counts when available.
- [ ] Keep the validation-cost harness as the primary proof boundary for these changes.

### `double_ma_validation_workflow.rs`

- [ ] Keep the existing CPU workflow bench as CPU regression coverage.
- [ ] Add a CUDA-gated sparse-finalist GPU bench.
- [ ] Print execution-profile work-shape fields when present.
- [ ] Do not use this file alone to claim Double-MA CUDA validation wins.

### `composite_validation_workflow.rs`

- [ ] Add at least one sparse-finalist composite GPU bench that makes pair expansion visible.
- [ ] Print execution-profile work-shape fields when present.
- [ ] Keep current composite GPU coverage as supplementary evidence, not the primary Double-MA proof boundary.

### Cross-cutting validation before merge

- [ ] Re-run GPU vs CPU parity tests for main metrics.
- [ ] Re-run walk-forward parity tests.
- [ ] Re-run CSCV parity tests.
- [ ] Re-run SPA parity tests.
- [ ] Re-run reopt-walk-forward parity tests.
- [ ] Re-run route/fallback reason tests.
- [ ] Re-run preserved-finalists cross rerun tests.
- [ ] Capture before/after reports from `ta_double_ma_validation_cost.rs`.
- [ ] Confirm lower `evaluated_pairs` and lower `gpu_range_eval_calls` on sparse-finalist workloads.
- [ ] Confirm no route-summary drift remains in frontend tests.

---

## Final recommendation

If you only implement three things first, implement these in this order:

1. **Aux-mask + execution profile**  
   This fixes hidden work and makes measurement honest.

2. **Exact-combo GPU path**  
   This is the biggest CUDA throughput win in the supplied files.

3. **Multi-range batching**  
   This removes repeated GPU re-entry and host reshaping overhead across walk-forward, CSCV, SPA, and reopt.

Everything else should be measured relative to those three changes.
