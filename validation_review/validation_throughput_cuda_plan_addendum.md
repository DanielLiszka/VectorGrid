# Validation throughput and fallback behavior plan — addendum

Date: 2026-03-22  
Applies to: `double_ma_validate.rs`, `validation_pipeline.rs`, `commands_validation.rs`, `validation_common.rs`, `validationRequestBuilder.ts`, `validationRequestBuilder.test.ts`, `ta_double_ma_validation_cost.rs`, `double_ma_validation_workflow.rs`, `composite_validation_workflow.rs`

This addendum strengthens the main plan with a few things that were either under-emphasized or not called out explicitly enough in the first document.

## What changes from the first plan

The core priority order does **not** change:

1. exact-combo GPU validation instead of sparse-grid expansion
2. multi-range batching for walk-forward / CSCV / SPA / reopt
3. masking non-gating post-pass analytics on preserved-finalists reruns
4. series/state reuse on CPU tails
5. honest benchmark attribution

Those are still the right first moves.

What this addendum adds is:

- one important **correctness + determinism** issue around `serde_json::to_string(...)` equality/keying
- one correction to the earlier `SeriesCache` sketch so it does not accidentally encourage cloning large `CandidateSeries`
- one extra **allocation-reuse** improvement for SPA bootstrap that mirrors the trade-bootstrap scratch change
- a few optional but high-upside **backend-adjacent CUDA** improvements that go beyond the original prompt while staying aligned with your performance/correctness goals

---

## 1. Important missed item: `serialized_eq(...)` is both a performance cost and a correctness risk

### Why this matters

In `double_ma_validate.rs`, the GPU-uniformity gate currently uses:

```rust
fn serialized_eq<T: Serialize>(a: &T, b: &T) -> Result<bool, String> {
    Ok(serde_json::to_string(a).map_err(|e| e.to_string())?
        == serde_json::to_string(b).map_err(|e| e.to_string())?)
}
```

That has two problems:

1. **Performance**: it allocates and serializes repeatedly in the GPU prep loop.
2. **Correctness / determinism risk**: for map-containing values such as `Option<HashMap<String, f64>>`, plain `serde_json::to_string(...)` is not a stable canonical identity because `HashMap` iteration order is not a semantic ordering.

That means two logically equal param maps can produce different serialized strings if insertion/iteration order differs. In practice, that can:

- reject GPU fixed-candidate validation even when candidates are semantically uniform
- create unstable candidate keys for cache/matching helpers
- make tests more fragile than they should be

### Where to fix it

At minimum:

- `double_ma_validate.rs::serialized_eq`
- `double_ma_validate.rs::try_prepare_gpu_fixed_candidate_validation`
- `double_ma_validate.rs` places that generate candidate keys with `serde_json::to_string(&params)`
- `double_ma_validate.rs` determinism tests that compare serialized params directly

### Recommended fix order

#### Preferred fix

If the involved types can reasonably derive or implement structural equality, use direct comparisons instead of JSON serialization.

That is the best fix for both speed and correctness.

#### Safe fallback fix

If direct equality is not available today, replace plain string serialization with a **canonical key helper** that sorts object keys before stringifying.

### Code-edit shape

**Add a canonical helper in `double_ma_validate.rs` now, or better, move it into shared app-core validation utilities and use it from both app-core and Tauri-side validation code.**

```rust
use serde::Serialize;
use serde_json::Value;

fn canonicalize_json_value(v: &Value) -> Value {
    match v {
        Value::Array(xs) => Value::Array(xs.iter().map(canonicalize_json_value).collect()),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            let mut out = serde_json::Map::with_capacity(map.len());
            for key in keys {
                if let Some(v2) = map.get(key) {
                    out.insert(key.clone(), canonicalize_json_value(v2));
                }
            }
            Value::Object(out)
        }
        _ => v.clone(),
    }
}

fn canonical_json_key<T: Serialize>(value: &T) -> Result<String, String> {
    let v = serde_json::to_value(value).map_err(|e| e.to_string())?;
    serde_json::to_string(&canonicalize_json_value(&v)).map_err(|e| e.to_string())
}

fn canonical_eq<T: Serialize>(a: &T, b: &T) -> Result<bool, String> {
    Ok(canonical_json_key(a)? == canonical_json_key(b)?)
}
```

Then in `try_prepare_gpu_fixed_candidate_validation(...)`, precompute the first candidate’s keys once instead of serializing repeatedly:

```rust
let first_strategy_key = canonical_json_key(first_strategy)?;
let first_halftrend_key = canonical_json_key(first_halftrend)?;
let first_fast_params_key = canonical_json_key(&first_fast_params)?;
let first_slow_params_key = canonical_json_key(&first_slow_params)?;

for cand in &req.candidates {
    if canonical_json_key(&cand.strategy)? != first_strategy_key {
        return Err(
            "GPU fixed-candidate validation currently requires a uniform StrategyConfig"
                .to_string(),
        );
    }
    if canonical_json_key(&cand.halftrend_filter)? != first_halftrend_key {
        return Err(
            "GPU fixed-candidate validation currently requires a uniform halftrend filter"
                .to_string(),
        );
    }
    if canonical_json_key(&cand.fast_ma_params.as_ref().or(req.fast_ma_params.as_ref()))?
        != first_fast_params_key
    {
        return Err(
            "GPU fixed-candidate validation currently requires uniform fast MA params"
                .to_string(),
        );
    }
    if canonical_json_key(&cand.slow_ma_params.as_ref().or(req.slow_ma_params.as_ref()))?
        != first_slow_params_key
    {
        return Err(
            "GPU fixed-candidate validation currently requires uniform slow MA params"
                .to_string(),
        );
    }
}
```

### Better long-term version

If these types are under your control, the best end-state is:

- derive `PartialEq` where possible for config types
- use direct equality for strategy/filter objects
- compare optional param maps structurally rather than through JSON
- keep canonical JSON only for logs / debug / cross-module fallback IDs

### Secondary places to tighten

The same issue exists in plain `serde_json::to_string(&params)` key generation in `double_ma_validate.rs` for:

- reopt-walk-forward finalist keys
- SPA candidate keys
- the determinism test that currently compares serialized params

Those should also use the same canonical helper or a stable candidate ID.

---

## 2. Small correction to the first plan: do not clone `CandidateSeries` in the cache path

The earlier MD used a sketch like this:

```rust
base_series.push(cache.get_or_build(params)?.clone());
```

That sketch was meant to show reuse, but it is not the version you should actually implement if `CandidateSeries` owns large vectors.

### Why this matters

If `CandidateSeries` cloning deep-copies:

- fast MA series
- slow MA series
- halftrend trend series
- Kaufman stop series

then the cache can accidentally turn into a memory amplifier instead of an optimization.

### Correct implementation shape

Keep the cache owning the built series and reuse **references**.

```rust
let mut base_scored: Vec<(usize, Metrics, f64)> = Vec::with_capacity(finalists.len());
for (idx, params) in finalists.iter().enumerate() {
    let series = cache.get_or_build(params)?;
    let metrics = eval_metrics_window(series, series.start_t, series.n);
    let score = double_ma_validation_robust_score(&metrics, None);
    base_scored.push((idx, metrics, score));
}
```

For the scenario pass, do the same thing:

```rust
for (idx, params) in finalists.iter().enumerate() {
    let base_series = cache.get_or_build(params)?;
    let scenario = friction_stress_scenarios(&params.strategy)[scenario_idx];

    let mut stressed_cfg = base_series.cfg.clone();
    stressed_cfg.fee_bps = scenario.fee_bps;
    stressed_cfg.spread_bps = scenario.spread_bps;
    stressed_cfg.slippage_bps = scenario.slippage_bps;

    let metrics = eval_metrics_window_with_cfg(
        base_series,
        base_series.start_t,
        base_series.n,
        &stressed_cfg,
    );
    let score = double_ma_validation_robust_score(&metrics, None);
    scored.push((idx, metrics, score));
}
```

### Recommended cache ownership pattern

One of these is safer than cloning owned series values around:

- `HashMap<String, CandidateSeries<'a>>` returning `&CandidateSeries<'a>`
- `HashMap<String, Box<CandidateSeries<'a>>>` if borrow ergonomics are easier that way
- `HashMap<String, Arc<CandidateSeries<'a>>>` only if shared ownership is genuinely needed

The first option is probably enough here.

---

## 3. Extra missed allocation win: reuse stationary-bootstrap index scratch in SPA too

The first document called out scratch reuse for `compute_trade_bootstrap_stats(...)`, but the same allocation pattern also appears in `compute_spa_validation_stats(...)`.

### Current pattern

Inside `compute_spa_validation_stats(...)`:

```rust
let indices = stationary_bootstrap_indices(slice_count, block_length, &mut rng);
```

That allocates a fresh `Vec<usize>` on every bootstrap sample.

### Recommended edit

Generalize the helper into an `..._into(...)` form and reuse the same scratch vector in both:

- `compute_trade_bootstrap_stats(...)`
- `compute_spa_validation_stats(...)`

### Code-edit shape

```rust
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

    let restart_prob = (1.0 / mean_block.max(1) as f64).clamp(1e-6, 1.0);
    out.reserve(n);

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

Then in SPA:

```rust
let mut indices_scratch: Vec<usize> = Vec::with_capacity(slice_count);
for _ in 0..bootstrap_samples {
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    stationary_bootstrap_indices_into(
        slice_count,
        block_length,
        &mut rng,
        &mut indices_scratch,
    );

    let mut boot_stat = 0.0_f64;
    for diff in &diffs {
        let mean = sample_mean(diff);
        let std = sample_std(diff, mean).max(1e-12);
        let mut centered_sum = 0.0_f64;
        for &sample_idx in &indices_scratch {
            centered_sum += diff[sample_idx] - mean;
        }
        let centered_mean = centered_sum / slice_count as f64;
        let t_star = (sqrt_n * centered_mean / std).max(0.0);
        if t_star > boot_stat {
            boot_stat = t_star;
        }
    }
    if boot_stat >= observed_stat {
        exceed = exceed.saturating_add(1);
    }
}
```

This is a clean allocation-reuse win and should not change semantics.

---

## 4. Additional optional cleanup in `compute_spa_validation_stats(...)`: flatten the diff matrix

### Why this matters

Today SPA builds:

```rust
let mut diffs: Vec<Vec<f64>> = Vec::with_capacity(candidate_count);
```

That means:

- one heap allocation per candidate for the diff vector
- worse cache locality during the bootstrap loop
- repeated nested-vector bounds and pointer chasing

### Better shape

Store a single row-major diff buffer:

- shape: `[candidate_idx * slice_count + slice_idx]`
- plus `means[candidate_idx]`
- plus `inv_stds[candidate_idx]`

### Code-edit shape

```rust
let mut diffs_flat = vec![0.0_f64; candidate_count * slice_count];
let mut means = vec![0.0_f64; candidate_count];
let mut inv_stds = vec![0.0_f64; candidate_count];
let mut t_stats = vec![0.0_f64; candidate_count];

for (cand_idx, candidate) in candidate_slice_returns.iter().enumerate() {
    let row = &candidate[..slice_count];
    let dst = &mut diffs_flat[cand_idx * slice_count..(cand_idx + 1) * slice_count];
    for slice_idx in 0..slice_count {
        dst[slice_idx] = row[slice_idx] - benchmark[slice_idx];
    }

    let mean = sample_mean(dst);
    let std = sample_std(dst, mean).max(1e-12);
    let inv_std = 1.0 / std;
    let t = (sqrt_n * mean * inv_std).max(0.0);

    means[cand_idx] = mean;
    inv_stds[cand_idx] = inv_std;
    t_stats[cand_idx] = t;
}
```

Then the bootstrap loop works over slices of `diffs_flat`.

### Why I would treat this as optional, not first-pass

This is a real allocation/cache improvement, but it is still behind the larger wins from exact-combo GPU work shape, multi-range batching, and scratch reuse.

---

## 5. Minor but real missed item: avoid the double-allocation path in CPU SPA slice sums

`slice_return_sums_for_series(...)` currently gets its output by calling `slice_return_stats_for_series(...)` and then allocating another `Vec<f64>` during `.map(|entry| entry.sum).collect()`.

That is fine functionally, but it is still an avoidable extra allocation on CPU fallback paths.

### Better helper shape

```rust
fn slice_return_sums_for_series_into(
    series: &CandidateSeries<'_>,
    start_t: usize,
    slice_bars: usize,
    slices: usize,
    out: &mut Vec<f64>,
) {
    out.clear();
    out.resize(slices, 0.0);
    if slices == 0 || slice_bars == 0 {
        return;
    }

    let end_t = start_t
        .saturating_add(slice_bars.saturating_mul(slices))
        .min(series.n);
    if end_t <= start_t + 1 {
        return;
    }

    let trade_offset = if series.cfg.trade_on_next_bar { 1usize } else { 0usize };
    let mut pos: i32 = 0;

    for t in start_t..end_t {
        let slice_idx = (t - start_t) / slice_bars;
        if slice_idx >= slices {
            break;
        }

        let t_sig = t.saturating_sub(trade_offset);
        let f = series.fast[t_sig];
        let s = series.slow[t_sig];

        let mut sign: i32 = 0;
        if f.is_finite() && s.is_finite() {
            let eps = if series.cfg.eps_rel > 0.0 {
                series.cfg.eps_rel * 1.0_f64.max(s.abs())
            } else {
                0.0
            };
            sign = (f > s + eps) as i32 - (f < s - eps) as i32;
            if series.cfg.long_only && sign < 0 {
                sign = 0;
            }
        }

        if let Some(ht) = series.halftrend_trend.as_ref() {
            let v = ht[t_sig];
            if !v.is_finite() {
                sign = 0;
            } else if v < 0.5 {
                sign = sign.max(0);
            } else {
                sign = sign.min(0);
            }
        }

        if sign != pos {
            if pos == 0 || sign == 0 {
                pos = if sign == 0 { 0 } else { sign };
            } else if !series.cfg.allow_flip {
                pos = 0;
            } else {
                pos = sign;
            }
        }

        let lr_t = series.lr[t];
        let step_r = if pos == 0 {
            0.0
        } else {
            (pos as f64) * lr_t.exp_m1()
        };
        out[slice_idx] += step_r;
    }
}
```

That lets you reuse a scratch `Vec<f64>` in CPU SPA preparation paths.

---

## 6. Additional cross-ticker cleanup: dataset dedupe should use normalized paths

`validation_common.rs::sanitize_cross_ticker_config(...)` currently dedupes datasets using the trimmed raw path string:

```rust
seen.insert(d.csv_path.trim().to_string())
```

That can miss duplicate datasets that differ only by:

- slash direction
- drive-letter casing
- path casing on case-insensitive environments

### Why this matters

It is a small thing, but it can cause:

- duplicate cross-ticker reruns
- misleading dataset counts
- wasted load/build work

### Recommended edit

Move a shared path-normalization helper into a small common utility and use it in both:

- `commands_validation.rs`
- `validation_common.rs`

### Code-edit shape

```rust
fn normalize_path_key(raw: &str) -> String {
    raw.trim().replace('/', "\\").to_ascii_lowercase()
}

pub fn sanitize_cross_ticker_config(cross_cfg: &mut CrossTickerConfig) {
    cross_cfg.datasets.retain(|d| !d.csv_path.trim().is_empty());
    let mut seen: HashSet<String> = HashSet::with_capacity(cross_cfg.datasets.len() * 2);
    cross_cfg
        .datasets
        .retain(|d| seen.insert(normalize_path_key(&d.csv_path)));

    // existing gate sanitization unchanged
}
```

This is small, safe, and keeps the cross-ticker surface from doing duplicate work.

---

## 7. Optional backend-adjacent CUDA changes that go beyond the original prompt

These are **not** the first things I would implement, but they are good optional next steps once the core plan is done.

## 7A. Return a compact validation metric DTO instead of the full `Metrics` payload where possible

### Why this matters

Many validation modules do not need every field in the full `Metrics` struct.

For example:

- walk-forward uses sharpe / max-dd / pnl / volatility / ulcer / martin / trades
- CSCV slice reconstruction uses only a subset
- SPA slice paths ideally need direct slice outputs, not full metrics

If the device currently copies back a larger struct than the validation layer actually needs, D2H cost is higher than necessary.

### Recommended shape

Keep full `Metrics` for the main in-sample candidate results if that is still the public contract, but allow the multi-range validation path to use a narrower internal result type.

```rust
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ValidationMetricLite {
    sharpe: f64,
    max_dd: f64,
    pnl: f64,
    volatility: f64,
    ulcer_index: f64,
    martin_ratio: f64,
    trades: u32,
}
```

### Why I would keep `f64`

Because your stated priority includes correctness/parity. Dropping to `f32` would save more bytes, but it is a separate parity decision.

---

## 7B. Pinned host staging buffers + async copies for range-batched GPU validation

### When this becomes worthwhile

After exact-combo + multi-range batching exist, the next likely overhead is host-device staging for:

- combo lists
- range lists
- result buffers

### Optional implementation direction

In the adjacent CUDA runtime/backend:

- allocate pinned host buffers for the recurrent validation result path
- use async H2D/D2H transfers on a dedicated stream
- overlap host post-processing of batch `N` with copy or kernel work for batch `N+1` when the execution model allows it

### Why I would classify it as optional

Because the current attached files do not show the runtime allocation/copy implementation details. This is very likely a real opportunity, but it belongs just outside the supplied file set.

---

## 7C. Persistent device scratch / memory pool for repeated validation batches

If the adjacent backend still allocates/frees device buffers per validation call, add a prepared-plan scratch layer or use a device memory pool so repeated validation batches can reuse:

- combo buffers
- range buffers
- output buffers
- temporary reduction scratch

This pairs very naturally with the exact-combo multi-range plan.

---

## 7D. CUDA Graph capture for repeated validation launch shapes

Once the validation shape is stable for a prepared plan:

- same dataset
- same combo count
- same range-batch shape
- same kernel sequence

CUDA Graph capture can remove repeated launch overhead.

This is a good optional second-stage improvement **after** the work shape is cleaned up. It is not worth doing before exact-combo and batching because you would be graphing a suboptimal workload.

---

## 7E. Device-resident prepared-plan indicator/state cache

This is the most ambitious optional item.

### Idea

For a prepared validation plan, keep device-resident state for anything that is identical across all validation ranges, such as:

- price/source arrays already resident in the GPU runtime
- regime filter state if uniform
- Kaufman stop state if uniform
- potentially MA outputs for the exact finalized combo set

### Why it can matter

If the current per-range GPU path still recomputes indicator state from scratch for every range, then exact-combo + multi-range batching still leaves performance on the table.

### Why I rank it later

This touches adjacent backend/runtime code not included here, and the payoff depends heavily on what the current kernel is already caching or reusing.

---

## 8. Optional larger-scope host-side changes that still fit your goals

## 8A. `CandlesView<'a>` or equivalent borrowed subrange view

The earlier plan already removed the full-range no-op copy in cross-ticker alignment.

A larger optional step is introducing a borrowed candle view type so **all** subrange alignment/slicing can avoid cloning OHLCV vectors.

### Value

- removes more memory traffic than the no-op borrow alone
- helps cross-ticker, walk-forward prep, and any other subrange-heavy path

### Cost

- broader signature churn across helpers that currently take owned `Candles`

That makes it a good optional refactor, not a first-pass change.

---

## 8B. Deterministic CPU-tail parallelism after caching

Once series/state reuse is in place, some CPU-only post-pass work becomes embarrassingly parallel across finalists:

- `regime_slice_robustness`
- `friction_stress`
- perhaps some non-RNG auxiliary summaries

### Safe way to do it

- keep output ordering stable
- keep RNG-dependent modules single-threaded unless their RNG partitioning is designed explicitly
- bound thread count to avoid fighting the GPU runtime or saturating memory bandwidth

This is a valid optional improvement once the bigger structural waste is gone.

---

## 8C. Stable candidate IDs generated once at request ingress

The first plan already suggested moving away from canonical JSON for identity in cross-ticker flows.

The stronger optional version is:

- generate a stable candidate ID once at request ingress
- carry it through validation, cross reruns, SPA/reopt labeling, and tests
- stop regenerating textual keys in every downstream phase

That improves:

- performance
- determinism
- logging readability
- test robustness

---

## 9. Measurement additions I would add to the cost/bench layer

These are not algorithm changes, but they would make your performance work easier to trust.

Add the following to `ta_double_ma_validation_cost.rs` and the relevant workflow benches:

1. **requested_candidates**
2. **unique_candidates**
3. **evaluated_pairs_on_device**
4. **grid_expansion_ratio = evaluated_pairs_on_device / unique_candidates**
5. **range_batches** and **total_ranges**
6. **main_gpu_validation_ms**
7. **gpu_result_materialization_ms**
8. **aux_finalist_analytics_ms**
9. **cpu_tail_ms**
10. **cross_ticker_alignment_copy_ms**
11. **cold_vs_warm_run flag** when resident caches materially affect timing
12. **estimated D2H metric bytes** and **estimated H2D combo/range bytes** for the new batched path

### Additional benchmark cases worth adding

- sparse finalists where `|Uf × Us| >> |candidates|`
- preserved-finalists cross reruns with aux masking enabled vs disabled
- multi-range GPU batch vs one-range-at-a-time
- CPU SPA prep before vs after scratch reuse + flat diff matrix
- warm-cache validation vs cold-cache validation

---

## 10. Priority-adjusted “what I would do next” ranking

### Still first

1. exact-combo GPU validation
2. multi-range batching
3. preserved-finalists aux masking
4. shared start-gate precompute
5. series/state reuse for friction stress and related tails

### New item that should move up

6. replace `serialized_eq(...)` / raw JSON keying with canonical or structural identity helpers

That one is not as large a speed win as the GPU work-shape cleanup, but it is more important than I originally emphasized because it touches correctness and determinism.

### Then

7. scratch reuse for trade bootstrap **and** SPA bootstrap
8. no-op cross-ticker borrow + normalized dataset dedupe
9. compact row-major host buffers in SPA and other CPU helpers
10. measurement refinements

### Later optional layer

11. compact validation result DTOs
12. pinned host staging / async copies
13. device scratch pools / CUDA graphs
14. full borrowed `CandlesView<'a>` refactor
15. deterministic CPU-tail parallelism
16. device-resident prepared-plan indicator/state cache

---

## 11. Addendum checklist

### Correctness / determinism

- [ ] Replace `serialized_eq(...)` or its call sites with structural equality or canonical-key equality.
- [ ] Remove plain `serde_json::to_string(...)` identity comparisons for params in `double_ma_validate.rs`.
- [ ] Update the determinism test to compare canonical/stable candidate keys instead of raw `serde_json::to_string(...)`.
- [ ] Re-run GPU eligibility tests with logically equal param maps inserted in different key orders.
- [ ] Re-run preserved-finalists cross rerun tests after the keying change.

### CPU-tail allocation cleanup

- [ ] Correct the `SeriesCache` implementation so it returns references and does not clone large `CandidateSeries` values.
- [ ] Add `eval_metrics_window_with_cfg(...)` if not already added from the first plan.
- [ ] Rework `compute_friction_stress_summaries(...)` around cached base series references.
- [ ] Add `stationary_bootstrap_indices_into(...)`.
- [ ] Use the reusable stationary-index scratch in `compute_trade_bootstrap_stats(...)`.
- [ ] Use the reusable stationary-index scratch in `compute_spa_validation_stats(...)`.
- [ ] Add the optional flat row-major diff buffer in SPA if profiling still shows SPA prep overhead.
- [ ] Add `slice_return_sums_for_series_into(...)` or equivalent direct-sum helper.

### Cross-ticker cleanup

- [ ] Normalize dataset path keys during cross-ticker dedupe in `validation_common.rs`.
- [ ] Keep the no-op full-range borrow in alignment from the first plan.
- [ ] Verify duplicate datasets with slash/case differences no longer trigger duplicate reruns.

### Benchmark / telemetry

- [ ] Add `evaluated_pairs_on_device`.
- [ ] Add `grid_expansion_ratio`.
- [ ] Add `range_batches` and `total_ranges`.
- [ ] Add `aux_finalist_analytics_ms`.
- [ ] Add `cpu_tail_ms`.
- [ ] Add `cold_vs_warm_run` or equivalent cache-state labeling.
- [ ] Add a sparse-finalist benchmark case.
- [ ] Add a warm-vs-cold validation benchmark case.

### Optional advanced CUDA layer

- [ ] Decide whether the multi-range validation path can use a compact internal validation metric DTO.
- [ ] Inspect the adjacent backend for reusable device scratch / memory-pool support.
- [ ] Inspect the adjacent backend for pinned-host / async-copy opportunities.
- [ ] Evaluate CUDA Graph capture only after exact-combo + multi-range batching lands.
- [ ] Profile whether device-resident indicator/state caching is still worth doing after the first two stages.

### Optional larger refactors

- [ ] Decide whether a borrowed `CandlesView<'a>` refactor is worth the signature churn.
- [ ] Decide whether cached CPU-tail modules should parallelize per finalist.
- [ ] Decide whether stable candidate IDs should be generated once at request ingress and carried throughout the validation stack.

