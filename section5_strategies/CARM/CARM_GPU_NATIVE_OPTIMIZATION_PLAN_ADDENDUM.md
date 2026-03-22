# CARM GPU-native optimization plan addendum

This addendum supplements `CARM_GPU_NATIVE_OPTIMIZATION_PLAN_FULL.md`.

It focuses on three things:

1. concrete items that are worth adding to the original plan
2. a few places where the original snippets can be tightened or corrected
3. optional performance/correctness work that goes beyond the strict scope of the original prompt bundle, while still preserving features and behavior

The emphasis remains the same: **performance, optimization, and correctness without removing features or breaking functionality**.

---

## Executive addendum summary

The most important additional items are:

1. **Prevent duplicate failed CUDA attempts in `compute_indicator_host_f32(...)`.** The current fallback path can try CUDA twice before finally using CPU.
2. **Strengthen `validate_inputs(...)`.** It currently validates shape presence, but not row-index bounds, time-range bounds, stride adequacy, or `usize -> i32` launch-argument safety.
3. **Do not rely on pointer+length alone for close-series cache reuse unless immutability is guaranteed.** That is fast, but it is not a fully correct cache key if the underlying slice can be mutated in place.
4. **Separate “steady-state compute cost” from “request clone/setup cost” in the benches.** The current validation benches clone substantial request objects inside the timed path.
5. **Improve the original scratch-buffer snippet in two places.**
   - the host readback `reserve_exact(...)` helper should be corrected
   - the device scratch helper should ideally become **grow-only capacity reuse**, not exact-length reuse, where the local `cust` API makes that ergonomic
6. **Add a per-request CUDA failure short-circuit.** Once a CARM signal-prep request has already proven that its CUDA route is failing, the remaining indicator calls should not keep re-attempting CUDA.
7. **Add explicit cold-vs-warm runtime reset hooks.** That will make benchmark interpretation much cleaner and will also help with error recovery.

---

# 1. Additional confirmed items worth adding to the plan

## A. `compute_indicator_host_f32(...)` can currently re-attempt a failing CUDA route

This is a real issue in the attached `carm.rs` flow.

Current behavior:

- `compute_indicator_host_f32(...)` prefers CUDA and tries `compute_dynamic_indicator_cuda_with_inputs(...)`
- if that fails, it calls `compute_indicator_f64(...)`
- `compute_indicator_f64(...)` also prefers CUDA and can try `compute_dynamic_indicator_cuda_with_inputs(...)` again

So one failing GPU path can be attempted **twice** before the code finally falls back to CPU.

That means the original plan should be strengthened in one specific way:

- the `f32` helper should fall back **directly** to CPU
- it should not fall back by calling another helper that re-enters the GPU preference logic

### Recommended edit

Keep the original “direct CPU-to-`f32` conversion” recommendation, but make its motivation explicit:

- it avoids `Vec<f64>` allocation and cast-back
- it prevents a **duplicate failed CUDA attempt**
- it makes fallback timing and trace data cleaner

A good implementation shape is:

```rust
fn series_to_f32(out: DynamicIndicatorComputeOutput) -> Vec<f32> {
    match out.series {
        DynamicIndicatorSeries::F64(v) => v.into_iter().map(|x| x as f32).collect(),
        DynamicIndicatorSeries::I32(v) => v.into_iter().map(|x| x as f32).collect(),
        DynamicIndicatorSeries::Bool(v) => {
            v.into_iter().map(|x| if x { 1.0 } else { 0.0 }).collect()
        }
    }
}
```

```rust
#[cfg(feature = "cuda")]
fn compute_indicator_host_f32(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f32>, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("CARM indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    let prefer_gpu = backend_prefers_gpu(backend)
        && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);

    if prefer_gpu {
        if let Some(prepared) = prepared_cuda_inputs {
            if let Ok(values) = compute_dynamic_indicator_cuda_with_inputs(
                prepared,
                indicator_id,
                Some(output_id),
                params,
                Kernel::Auto,
                vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
            ) {
                return cuda_series_to_host_f32(values);
            }
        }
    }

    let out = compute_dynamic_indicator_cpu(
        candles,
        ma_source,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
    )
    .map_err(|e| e.to_string())?;

    Ok(series_to_f32(out))
}
```

That is the safer and faster fallback structure.

---

## B. `validate_inputs(...)` should do more than shape checks

The current `validate_inputs(...)` protects against null pointers, empty series, and mismatched combo-vector lengths.

That is necessary, but it is not sufficient.

The following additional checks are worth adding for correctness and kernel safety:

- `mixed_stride >= cols`
- `fast_stride >= cols`
- `otto_stride >= cols`
- every `fast_row_index[k]` is within `[0, fast_rows)`
- every `otto_row_index[k]` is within `[0, otto_rows)`
- every `start_t[k]` and `end_t[k]` is non-negative
- every `start_t[k] <= end_t[k]`
- every `end_t[k] < cols`
- every `usize` that is later cast to `i32` fits within `i32`

Without these checks, invalid data can survive the Rust boundary and reach the kernel launch with values that can cause OOB reads, invalid indexing, or undefined behavior.

### Recommended stricter validation helper

```rust
fn checked_i32(name: &str, value: usize) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("CARM GPU {name} exceeds i32 range"))
}

fn validate_inputs(
    signals: &CarmGpuDeviceSignalMatrices<'_>,
    combos: &CarmGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("CARM GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f32.len() != n {
        return Err("CARM GPU close length mismatch".to_string());
    }

    if signals.mixed_rows == 0
        || signals.fast_rows == 0
        || signals.otto_rows == 0
        || signals.mixed_stride == 0
        || signals.fast_stride == 0
        || signals.otto_stride == 0
    {
        return Err("CARM GPU signal matrix shape is invalid".to_string());
    }

    if signals.mixed_stride < n || signals.fast_stride < n || signals.otto_stride < n {
        return Err("CARM GPU stride is smaller than cols".to_string());
    }

    if signals.coppock_row_major_ptr == 0
        || signals.ao_row_major_ptr == 0
        || signals.rocr_row_major_ptr == 0
        || signals.apo_row_major_ptr == 0
        || signals.otto_hott_row_major_ptr == 0
        || signals.otto_lott_row_major_ptr == 0
    {
        return Err("CARM GPU signal pointer is null".to_string());
    }

    checked_i32("cols", n)?;
    checked_i32("mixed_rows", signals.mixed_rows)?;
    checked_i32("mixed_stride", signals.mixed_stride)?;
    checked_i32("fast_rows", signals.fast_rows)?;
    checked_i32("fast_stride", signals.fast_stride)?;
    checked_i32("otto_rows", signals.otto_rows)?;
    checked_i32("otto_stride", signals.otto_stride)?;

    let combos_len = combos.fast_row_index.len();
    if combos.otto_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("CARM GPU combo vector length mismatch".to_string());
    }

    checked_i32("combo_count", combos_len)?;

    for (k, &row) in combos.fast_row_index.iter().enumerate() {
        if row < 0 || row as usize >= signals.fast_rows {
            return Err(format!(
                "CARM GPU fast_row_index[{k}]={} is out of range for fast_rows={}",
                row, signals.fast_rows
            ));
        }
    }

    for (k, &row) in combos.otto_row_index.iter().enumerate() {
        if row < 0 || row as usize >= signals.otto_rows {
            return Err(format!(
                "CARM GPU otto_row_index[{k}]={} is out of range for otto_rows={}",
                row, signals.otto_rows
            ));
        }
    }

    for (k, (&start_t, &end_t)) in combos
        .start_t
        .iter()
        .zip(combos.end_t.iter())
        .enumerate()
    {
        if start_t < 0 || end_t < 0 {
            return Err(format!(
                "CARM GPU combo[{k}] has negative start/end: start_t={}, end_t={}",
                start_t, end_t
            ));
        }

        let start_u = start_t as usize;
        let end_u = end_t as usize;

        if start_u > end_u {
            return Err(format!(
                "CARM GPU combo[{k}] has start_t > end_t: {} > {}",
                start_u, end_u
            ));
        }
        if end_u >= n {
            return Err(format!(
                "CARM GPU combo[{k}] end_t={} is out of range for cols={}",
                end_u, n
            ));
        }
    }

    Ok(())
}
```

### Performance note

This validation adds CPU work, but the work is linear in combo count and usually much cheaper than a failed or invalid GPU run.

If you later find that this validation cost is measurable on very large repeated batches, the right optimization is **not** to remove the checks. The right optimization is to validate once upstream and pass a prevalidated batch type into the kernel entrypoint.

---

## C. Prefer explicit cache invalidation or a stable series identity over pointer+length reuse

The original full plan suggested pointer+length identity as a practical way to skip repeated `close_f32` upload and log-return recomputation.

That works **only** if the close slice is effectively immutable across the repeated calls.

If the same `Vec<f32>` storage is reused and its contents change in place, then:

- pointer is unchanged
- length is unchanged
- cache appears valid
- but the resident device copy is stale

That is a correctness risk.

### Better approach: explicit close identity

Add an optional caller-provided identity token that changes whenever the close series changes.

```rust
pub struct CarmGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub close_cache_key: Option<u64>,
    pub cols: usize,
    pub mixed_rows: usize,
    pub mixed_stride: usize,
    pub coppock_row_major_ptr: u64,
    pub ao_row_major_ptr: u64,
    pub rocr_row_major_ptr: u64,
    pub fast_rows: usize,
    pub fast_stride: usize,
    pub apo_row_major_ptr: u64,
    pub otto_hott_row_major_ptr: u64,
    pub otto_lott_row_major_ptr: u64,
    pub otto_rows: usize,
    pub otto_stride: usize,
}
```

Then in the runtime:

```rust
struct CarmCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    // existing scratch fields...
    cached_close_key: Option<u64>,
}
```

And use it like this:

```rust
impl CarmCudaRuntime {
    fn ensure_close_and_log_returns(
        &mut self,
        device_id: u32,
        close: &[f32],
        close_cache_key: Option<u64>,
    ) -> Result<(), String> {
        let can_reuse = match (self.cached_close_key, close_cache_key) {
            (Some(prev), Some(cur)) => prev == cur,
            _ => false,
        };

        if can_reuse {
            return Ok(());
        }

        // upload + recompute log returns here

        self.cached_close_key = close_cache_key;
        Ok(())
    }
}
```

### Safe fallback policy

If no stable identity key is available, prefer this policy:

- **always** re-upload and recompute
- or use pointer+length reuse only behind a very explicit “immutable input contract” comment and benchmark-only configuration

That is better than silently returning stale results.

---

## D. Add a per-request CUDA failure short-circuit for CARM signal prep

CARM signal prep can make multiple indicator calls in one request:

- `coppock`
- `ao`
- `apo`
- `rocr`
- `otto/hott`
- `otto/lott`

If the first CUDA-backed indicator attempt fails due to a route-wide reason, it is wasteful to keep trying CUDA for the remaining indicators in the same CARM request.

Examples of route-wide failure reasons:

- missing `prepared_cuda_inputs`
- invalid prepared-device state
- device/context failure
- incompatible runtime/kernel state

### Recommended execution-context helper

```rust
#[cfg(feature = "cuda")]
#[derive(Clone, Debug, Default)]
struct CarmGpuAttemptState {
    allow_cuda: bool,
    first_fallback_reason: Option<String>,
}

#[cfg(feature = "cuda")]
impl CarmGpuAttemptState {
    fn new() -> Self {
        Self {
            allow_cuda: true,
            first_fallback_reason: None,
        }
    }

    fn record_failure(&mut self, reason: impl Into<String>) {
        if self.first_fallback_reason.is_none() {
            self.first_fallback_reason = Some(reason.into());
        }
        self.allow_cuda = false;
    }
}
```

Then thread that through the `*_f32` helpers:

```rust
#[cfg(feature = "cuda")]
fn compute_indicator_host_f32_with_state(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
    gpu_state: &mut CarmGpuAttemptState,
) -> Result<Vec<f32>, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("CARM indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    let prefer_gpu = gpu_state.allow_cuda
        && backend_prefers_gpu(backend)
        && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);

    if prefer_gpu {
        match prepared_cuda_inputs {
            Some(prepared) => {
                match compute_dynamic_indicator_cuda_with_inputs(
                    prepared,
                    indicator_id,
                    Some(output_id),
                    params,
                    Kernel::Auto,
                    vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                ) {
                    Ok(values) => return cuda_series_to_host_f32(values),
                    Err(e) => gpu_state.record_failure(e.to_string()),
                }
            }
            None => gpu_state.record_failure("prepared CUDA inputs missing"),
        }
    }

    let out = compute_dynamic_indicator_cpu(
        candles,
        ma_source,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
    )
    .map_err(|e| e.to_string())?;

    Ok(series_to_f32(out))
}
```

### Why this helps

It preserves behavior but avoids repeated failing CUDA dispatches inside one CARM request.

That helps both performance and diagnosability.

---

## E. Add cold/warm runtime reset hooks

Once runtime scratch reuse lands, the CARM GPU path will have meaningful warm-state behavior.

That is good for production performance, but it also means you should have an explicit way to reset the cache when you want to measure cold-start behavior or recover from a hard CUDA error.

### Recommended helper

```rust
pub fn clear_carm_gpu_runtime_cache(device_id: Option<u32>) {
    CARM_GPU_RUNTIME.with(|cell| {
        let mut map = cell.borrow_mut();
        if let Some(id) = device_id {
            map.remove(&id);
        } else {
            map.clear();
        }
    });
}
```

### Uses

- measure cold vs warm behavior intentionally
- reset per-device scratch after a fatal benchmark failure
- make unit/integration tests deterministic

This is a small utility but it helps a lot once caching exists.

---

# 2. Corrections and refinements to the original snippets

## A. Correct the `host_metrics.reserve_exact(...)` helper

The original full plan used this idea:

```rust
if self.host_metrics.capacity() < len {
    self.host_metrics.reserve_exact(len - self.host_metrics.capacity());
}
unsafe {
    self.host_metrics.set_len(len);
}
```

That should be tightened.

`Vec::reserve_exact(additional)` is based on the vector’s **current length**, not on its capacity.

So the safer version is:

```rust
impl CarmCudaRuntime {
    fn ensure_host_metrics_len(&mut self, len: usize) {
        self.host_metrics.clear();
        if self.host_metrics.capacity() < len {
            self.host_metrics.reserve_exact(len);
        }
        unsafe {
            self.host_metrics.set_len(len);
        }
    }
}
```

This preserves the intended effect:

- no zero-fill before D2H copy
- capacity reuse when possible
- correct capacity growth semantics

### Guardrail

This remains appropriate only because the element type is `f32`, which has no destructor and is fully overwritten before read.

Do not generalize this pattern carelessly to non-POD / drop-bearing types.

---

## B. Improve the reusable device-buffer idea from exact-size reuse to grow-only reuse when practical

The original full plan showed an exact-length `ReusableDeviceBuffer<T>` helper.

That is still useful, but the best version is slightly better:

- keep the allocation if the requested length is **smaller than or equal to** the current capacity
- only reallocate when the requested length **exceeds** the current capacity

That avoids churn when workloads bounce between small, medium, and large cases.

### Better conceptual shape

```rust
struct ReusableDeviceBuffer<T: DeviceCopy> {
    buf: Option<DeviceBuffer<T>>,
    cap: usize,
    logical_len: usize,
}
```

```rust
impl<T: DeviceCopy> ReusableDeviceBuffer<T> {
    fn ensure_capacity(&mut self, min_len: usize) -> Result<(), String> {
        if self.cap >= min_len && self.buf.is_some() {
            return Ok(());
        }

        self.buf = if min_len == 0 {
            None
        } else {
            Some(unsafe { DeviceBuffer::<T>::uninitialized(min_len) }
                .map_err(|e| e.to_string())?)
        };
        self.cap = min_len;
        self.logical_len = min_len;
        Ok(())
    }
}
```

### Important implementation note

Whether this can be written cleanly depends on what slice-copy APIs your local `cust` version exposes for copying a shorter host slice into a larger device allocation.

So the practical recommendation is:

- **preferred:** grow-only reuse if your `cust` version supports copying into a logical prefix cleanly
- **acceptable fallback:** exact-size reuse as already described in the full plan

Exact-size reuse is still a clear win over per-call fresh allocation.

---

## C. Use checked conversion for every `usize -> i32` kernel-launch argument

The attached `carm_gpu_kernel.rs` currently converts several `usize` values to `i32` with `as i32`.

That should be tightened for correctness.

Examples currently affected:

- `signals.cols as i32`
- `signals.mixed_rows as i32`
- `signals.mixed_stride as i32`
- `signals.fast_rows as i32`
- `signals.fast_stride as i32`
- `signals.otto_rows as i32`
- `signals.otto_stride as i32`
- `combo_count as i32`
- `CARM_METRICS_STRIDE as i32`

### Recommended helper

```rust
fn checked_i32(name: &str, value: usize) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("CARM GPU {name} exceeds i32 range"))
}
```

Then use it before building the launch argument list.

This is cheap and protects you from silent narrowing on very large inputs.

---

## D. Replace magic flag bits with named constants

This is not a big performance change, but it is a correctness and maintenance win.

Current code uses literal bit positions like:

```rust
flags |= 1u32 << 4;
```

That is hard to audit against the CUDA side.

### Recommended edit

```rust
const FLAG_LONG_ONLY: u32 = 1u32 << 0;
const FLAG_NO_FLIP: u32 = 1u32 << 1;
const FLAG_TRADE_ON_NEXT_BAR: u32 = 1u32 << 2;
const FLAG_ENABLE_CARM_EXIT_VOTES: u32 = 1u32 << 4;
```

Then:

```rust
let mut flags = 0u32;
if strategy.long_only {
    flags |= FLAG_LONG_ONLY;
}
if !strategy.allow_flip {
    flags |= FLAG_NO_FLIP;
}
if strategy.trade_on_next_bar {
    flags |= FLAG_TRADE_ON_NEXT_BAR;
}
flags |= FLAG_ENABLE_CARM_EXIT_VOTES;
```

If the actual CUDA kernel source is available later, mirror these definitions there too.

---

# 3. Benchmark-plan additions that improve evidence quality

## A. Request clone cost should be measured explicitly

The validation benches repeatedly do:

- `base.clone()` inside the route probe or timed path
- `vram_heavy.clone()` inside the timed path

That clone work may or may not be material, depending on candidate count and request size.

If your goal is total end-to-end route timing, keeping clone cost is reasonable.

If your goal is *engine compute cost*, clone/setup cost can blur the result.

### Recommended add-on bench

Add a tiny request-clone benchmark so you can quantify this overhead separately:

```rust
fn bench_carm_validation_request_clone(c: &mut Criterion) {
    let base = CompositeValidationRequest {
        data_id: "bench-gpu-carm-clone".to_string(),
        backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: make_carm_candidates(64),
        walk_forward: Some(WalkForwardConfigOptional {
            window_bars: 10_000,
            step_bars: 10_000,
        }),
        reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
            window_bars: 15_000,
            step_bars: 10_000,
            top_n_candidates: 4,
        }),
        spa: Some(SpaConfigOptional {
            slices: 6,
            bootstrap_samples: 64,
            block_length: 2,
            benchmark: SpaBenchmarkKind::BuyAndHold,
        }),
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: Some(CscvPboConfig {
            slices: 6,
            max_combinations: 64,
            threshold: 0.0,
        }),
        seed: 321,
        max_candidates: 64,
        validation_engine: ta_app_core::ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    };

    c.bench_function("composite_validate/request_clone/carm/64candidates", |b| {
        b.iter(|| {
            let _ = base.clone();
        })
    });
}
```

### Interpretation rule

- if clone time is tiny relative to workflow time, keep the main benches as-is
- if clone time becomes meaningful, annotate the main benchmark results accordingly or restructure the measured path

This is a benchmark-fidelity improvement, not a production-code change.

---

## B. Separate cold-state and warm-state benchmark intent explicitly

Once buffer reuse is added, you will have at least two legitimate timing questions:

1. **cold CARM GPU path**
   - first call on a device/thread/runtime
   - includes allocation and first upload costs
2. **warm CARM GPU path**
   - repeated same-series evaluations
   - shows the benefit of scratch reuse and resident close/log-return state

Both are valid, but they should not be mixed accidentally.

### Recommended benchmark naming convention

Use explicit names such as:

- `carm/gpu_kernel/200k/cold_large`
- `carm/gpu_kernel/200k/warm_large`
- `composite_validate/gpu_kernel/carm/100k/64candidates/cold`
- `composite_validate/gpu_kernel/carm/100k/64candidates/warm`

And use `clear_carm_gpu_runtime_cache(...)` before cold measurements.

This makes the results easier to interpret and compare.

---

## C. Add a direct CARM prep microbench beyond the existing prompt scope

The original prompt bundle emphasized existing harnesses first, which is correct.

However, if you want to localize performance after the main fixes land, one useful optional microbench is:

- `compute_carm_mixed_signals_f32(...)`
- `compute_carm_rocr_signal_f32(...)`
- `compute_carm_otto_signals_f32(...)`
- `compute_carm_signals_f32(...)`
- `eval_carm_metrics_gpu_kernel_device_rows(...)` on prebuilt device signal matrices

This should not replace the composite benches.

It should exist only to answer targeted questions like:

- is the time still in signal prep?
- is the time in kernel staging?
- is OTTO still disproportionately expensive?
- did scratch reuse help the kernel path but not the prep path?

This is a helpful optional extension once the core benchmark gates are already aligned.

---

# 4. Additional correctness work that also protects performance

## A. Add boundary-equality parity tests, not just directional tests

The attached `build_carm_desired_positions(...)` uses conditions like:

- `coppock_now > 0.0`
- `coppock_now < 0.0`
- `rocr_now > 1.0`
- `rocr_now < 1.0`
- `otto_spread >= cfg.otto_threshold`
- `otto_spread <= -cfg.otto_threshold`

That means equality semantics matter.

### Add tests for exact boundaries

Add CPU-reference tests where the signals land exactly on the decision boundaries.

Examples worth covering:

- `rocr == 1.0`
- `apo == 0.0`
- `coppock == 0.0`
- `otto_spread == +threshold`
- `otto_spread == -threshold`

These are particularly important if fp32 GPU prep is feeding the strategy logic.

### Example test idea

```rust
#[test]
fn build_carm_desired_positions_respects_boundary_equality() {
    let close = vec![100.0, 100.0, 100.0, 100.0];
    let signals = CarmSignals {
        coppock: vec![f64::NAN, 0.0, 0.01, -0.01],
        ao: vec![f64::NAN, 0.0, 0.2, -0.2],
        rocr: vec![f64::NAN, 1.0, 1.01, 0.99],
        apo: vec![f64::NAN, 0.0, 0.1, -0.1],
        otto_hott: vec![f64::NAN, 100.0, 100.0, 100.0],
        otto_lott: vec![f64::NAN, 100.6, 101.0, 99.0],
        first_valid: 1,
    };

    let strategy = StrategyConfig::default();
    let cfg = CarmStrategyParams::default();
    let desired = build_carm_desired_positions(&close, &signals, 1, &strategy, &cfg);

    // Fill in exact assertions based on the intended equality semantics.
    assert_eq!(desired[1], 0);
}
```

This does not change functionality. It protects it.

---

## B. Audit the `otto_threshold.abs().max(0.6)` semantic mismatch

This is slightly outside the narrow performance scope, but it is important enough to call out.

In `compute_carm_otto_signals(...)` and `compute_carm_otto_signals_f32(...)`, the OTTO indicator parameter uses:

```rust
cfg.otto_threshold.abs().max(0.6)
```

But the strategy decision logic later compares `otto_spread` against the raw `cfg.otto_threshold`.

That means if a user sets `otto_threshold` below `0.6`, the indicator-prep and the decision logic are not obviously operating on the same effective threshold regime.

### Why this matters

- possible CPU/GPU parity confusion
- possible user-facing strategy-semantics confusion
- possible benchmark mismatch if some candidate builders use values below the clamp threshold

### Recommendation

Do one of these explicitly:

1. confirm this is intentional and document it
2. align the indicator-prep threshold and the strategy-decision threshold
3. add tests that lock the intended behavior down

Even if this stays unchanged, it should be **explicitly tested and documented**.

---

## C. Add reset-on-error behavior for runtime caches

If a CUDA launch or transfer fails in a way that may leave the runtime state questionable, it is safer to allow the caller or the error path to invalidate that runtime entry.

This is especially relevant once you add reusable buffers.

### Recommended policy

- for normal recoverable errors: return `Err(...)` as today
- for known fatal / context-related errors: clear the cached runtime for that device and force reinitialization on the next call

If your `cust` error taxonomy makes that easy, this is worth doing.

If not, the smaller first step is just to expose `clear_carm_gpu_runtime_cache(...)` and call it from tests or benchmark harnesses when needed.

---

# 5. Optional changes beyond the original prompt scope

These are not required to satisfy the original review bundle, but they are reasonable next steps if your main goal is maximum performance while preserving correctness.

## A. Introduce a prepared CARM GPU execution session

Right now the visible kernel entrypoint is a function-level API.

A stronger long-term structure is a session object that explicitly owns:

- resident close/log-return buffers
- reusable combo/output scratch
- maybe prepacked signal-matrix metadata
- route counters / cache-hit counters
- cold/warm lifecycle control

Conceptually:

```rust
pub struct CarmGpuExecSession {
    device_id: u32,
    runtime: CarmCudaRuntime,
}
```

Benefits:

- cleaner cache ownership
- easier cold/warm benchmark control
- fewer hidden side effects from thread-local state
- more obvious path to future async or pipelined execution

This is a design upgrade, not a required near-term patch.

---

## B. Use pinned host memory plus async copies only after the simple reuse fixes land

Pinned memory and async copy overlap can help, but they are not the first thing to do.

They become attractive only after:

- close reuse is already in place
- log-return reuse is already in place
- you have confirmed that D2H or H2D transfer is still material

If the full metrics readback remains mandatory, then these can help:

- pinned host buffer for `host_metrics`
- async D2H copy on the stream
- maybe async H2D combo upload if the API and ownership model make it clean

This is worthwhile as a **phase-2 transfer optimization**, not as the first patch.

---

## C. Consider combo-order locality optimization if profiling shows row-access inefficiency

This is a higher-level optional idea.

If each GPU thread evaluates one combo, and neighboring threads pull from widely scattered `fast_row_index` / `otto_row_index` rows, the kernel may get worse cache locality on the indicator matrices.

A possible optimization is:

- sort or bucket combos by `(fast_row_index, otto_row_index)` before launch
- preserve an inverse permutation if host output order must remain stable

Potential benefits:

- improved row locality
- less chaotic signal-matrix access pattern
- possibly less warp divergence if combo groups behave similarly

Potential costs:

- extra CPU preprocessing
- output reordering / inverse-permutation restoration
- more complex debugging

This should be done only after profiling, and only if the kernel-body access pattern supports it.

---

## D. Consider narrower combo-index representations only if transfer volume still matters

Today the combo arrays are `i32`:

- `fast_row_index`
- `otto_row_index`
- `start_t`
- `end_t`

If the real ranges are modest, you could consider narrower packed host-side representations for transfer efficiency.

Examples:

- `u16` row indices if row counts are always `< 65536`
- `u32` time indices if signedness is not needed
- a packed struct if the kernel is changed accordingly

This is very much an optional phase-3 change.

Do it only if transfer volume remains measurable after the simpler reuse fixes.

---

## E. Add tiny micro-caches for repeated descriptor/output resolution and parameter construction

This is a small optimization, but it is safe and sometimes worthwhile in repeated signal-prep flows.

Possible examples:

- cache resolved output IDs for:
  - `coppock/value`
  - `ao/value`
  - `apo/value`
  - `rocr/value`
  - `otto/hott`
  - `otto/lott`
- replace small `vec![...]` parameter construction with stack-backed storage or `SmallVec` if you already use it elsewhere

These are low-priority micro-optimizations.

They should come only after the main allocation/copy fixes.

---

## F. Add route telemetry counters for CARM specifically

A very practical optional enhancement is lightweight per-route counters behind a debug or benchmark feature.

Examples:

- close cache hits / misses
- log-return cache hits / misses
- device allocations this call
- H2D bytes this call
- D2H bytes this call
- indicator CUDA attempts / successes / failures
- CPU fallback count by indicator

This makes it much easier to prove that a patch actually removed the work you intended to remove.

It also reduces the chance of “improvements” that only move work around.

---

## G. Optional “do not pursue” list

These are the things I would **not** prioritize or would actively avoid.

### 1. Do not use pointer+length cache reuse as a hidden correctness assumption

Fine for controlled warm benchmarks.

Not fine as a silent correctness contract unless the input series is explicitly immutable.

### 2. Do not switch to mapped zero-copy host memory for the main close series

For this workload, resident device memory is the right direction.

Mapped host memory is usually the wrong answer for a repeatedly traversed backtest series.

### 3. Do not use uninitialized `Vec<Metrics>`

The decode work is cheap compared with transfer and launch cost.

Unsafe uninitialized host `Metrics` storage is not worth the maintenance and correctness risk.

### 4. Do not remove input validation just to save CPU cycles

If validation becomes measurable, move it earlier and validate once.

Do not remove it.

---

# 6. Additional checklist for implementation tracking

Use this together with the checklist in `CARM_GPU_NATIVE_OPTIMIZATION_PLAN_FULL.md`.

## `carm.rs`

- [ ] Rewrite `compute_indicator_host_f32(...)` so CPU fallback does **not** call `compute_indicator_f64(...)`.
- [ ] Verify that a failed CUDA attempt is no longer retried by the fallback path.
- [ ] Add a per-request CUDA attempt state so one route-wide CUDA failure disables further CUDA attempts for the same CARM signal-prep request.
- [ ] Add tests or tracing that confirm how many CUDA attempts occur during one CARM signal-prep request.
- [ ] Add explicit boundary-equality tests for `rocr == 1.0`, `apo == 0.0`, `coppock == 0.0`, and `otto_spread == ±threshold`.
- [ ] Audit and document whether `cfg.otto_threshold.abs().max(0.6)` is intentional.
- [ ] If intentional, add tests that lock the behavior down.

## `carm_gpu_kernel.rs`

- [ ] Strengthen `validate_inputs(...)` with row-index bounds checks.
- [ ] Strengthen `validate_inputs(...)` with `start_t/end_t` non-negative and in-range checks.
- [ ] Strengthen `validate_inputs(...)` with stride-adequacy checks (`stride >= cols`).
- [ ] Replace all `usize -> i32` kernel-launch argument casts with checked conversions.
- [ ] Add named constants for every flag bit sent to the kernel.
- [ ] Add a cache-reset helper such as `clear_carm_gpu_runtime_cache(...)`.
- [ ] Decide whether close/log-return reuse uses:
  - [ ] explicit cache key / generation token
  - [ ] immutable-slice contract
  - [ ] no reuse when identity is uncertain
- [ ] Avoid deploying pointer+length-only cache reuse as a silent correctness assumption in production.
- [ ] Improve reusable device buffers to grow-only capacity reuse if the local `cust` APIs make that practical.
- [ ] Correct the host readback helper so `reserve_exact(...)` is based on vector length semantics correctly.
- [ ] Add tests for invalid negative `start_t/end_t`.
- [ ] Add tests for out-of-range `fast_row_index` / `otto_row_index`.
- [ ] Add tests for `end_t >= cols` and `start_t > end_t`.

## `gpu_native_expansion_opt.rs`

- [ ] Add explicit cold vs warm naming if you intend to measure both.
- [ ] Add a steady-state-only CARM benchmark if you want to highlight buffer-reuse payoff.
- [ ] Consider a request-clone-only microbench if benchmark interpretation becomes ambiguous.
- [ ] Ensure any new warm benchmark uses a deliberate runtime state and not accidental cache carryover.

## `composite_validation_workflow.rs`

- [ ] Add a request-clone-only microbench or at least quantify clone overhead separately.
- [ ] Decide whether the main validation benches are meant to be:
  - [ ] end-to-end request+engine timing
  - [ ] engine-dominant timing
- [ ] Label that intent explicitly in benchmark names or comments.
- [ ] Use cache reset intentionally for cold validation measurements.
- [ ] Keep warm validation measurements explicitly warm.

## Optional phase-2 / phase-3 work

- [ ] Consider a prepared `CarmGpuExecSession` abstraction.
- [ ] Consider pinned host memory for metrics readback only after basic reuse wins are confirmed.
- [ ] Consider async copy overlap only after transfer remains measurable.
- [ ] Consider combo bucketing / ordering for row locality only after profiling.
- [ ] Consider narrower combo index representations only if transfer volume still matters.
- [ ] Consider tiny micro-caches for indicator-output ID resolution and parameter construction.
- [ ] Consider per-route telemetry counters for CARM cache hits, bytes moved, and fallback counts.

## Guardrails

- [ ] Do not remove overlays/SL/TP blockers unless you implement full parity support.
- [ ] Do not relax correctness validation just to reduce CPU work.
- [ ] Do not assume pointer+length is a sufficient cache key unless the caller guarantees immutability.
- [ ] Do not use uninitialized memory for non-POD/drop-bearing host types.
- [ ] Re-run CPU/GPU parity checks after every staging change, not just after kernel-body changes.

---

## Final addendum recommendation

If I were extending the original full plan, I would prioritize these additions in this order:

1. fix `compute_indicator_host_f32(...)` so it does not retry failed CUDA routes indirectly
2. strengthen `validate_inputs(...)` with range/stride/i32 checks
3. replace pointer+length reuse with an explicit close-series identity or safe invalidation policy
4. add per-request CUDA failure short-circuiting
5. add runtime cache reset helpers for clean cold/warm measurement and recovery
6. then decide whether to pursue phase-2 extras such as pinned-memory async copies, execution sessions, or locality-aware combo ordering

These additions keep the original plan fully aligned with your stated goal: **maximize performance and optimization, preserve correctness, and avoid breaking features or functionality**.
