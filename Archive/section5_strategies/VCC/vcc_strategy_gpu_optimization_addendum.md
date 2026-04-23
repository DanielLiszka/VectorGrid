# VCC GPU optimization addendum and corrections

This addendum supplements `vcc_strategy_gpu_optimization_plan.md`.

It focuses on **additional performance/correctness improvements**, **items I would strengthen or revise in the original plan**, and **optional changes that go a bit beyond the exact prompt scope** while still staying aligned with your goal:

- improve CUDA performance
- reduce unnecessary memory copies
- use uninitialized memory only where safe
- preserve correctness and current features
- avoid functionality regressions

---

## What I would add to the original plan

The original plan is still directionally correct, but I would add the following priorities.

### New highest-priority missed item

**Split the current VCC GPU evaluation into a true _metrics-only_ path and a _detailed_ path.**

Right now:

- `eval_vcc_metrics_gpu_kernel_device_rows_detailed(...)` allocates `d_slice_out`, copies it back, allocates `host_slice_out`, and decodes step-return sums.
- `eval_vcc_metrics_gpu_kernel_device_rows(...)` just calls the detailed path and discards the extra slice metrics.

That means the basic API is still paying for:

- device slice buffer allocation
- D→H copy of slice metrics
- host slice vector allocation
- decode work for values it never returns

This is a real missed optimization opportunity because it is present in the provided code today.

### New correctness hardening item

**Add strict range validation for combo indices and time windows.**

The current input validator checks vector lengths, but it does **not** validate:

- `fast_row_index[i]` in `[0, fast_rows)`
- `trend_row_index[i]` in `[0, trend_rows)`
- `start_t[i]`, `metric_start_t[i]`, `end_t[i]` being non-negative and ordered
- `end_t[i] <= cols`

That leaves room for invalid batches to reach the kernel.

### New cache-safety item

**Do not rely only on pointer+len identity for cache reuse unless you also control mutation semantics.**

The earlier plan mentioned this risk, but I would now make it a first-class design rule:

- best: explicit dataset/generation ID
- acceptable fallback: pointer+len plus debug checksum/assertions
- least safe: pointer+len only

For performance work that introduces caching, this is the main correctness trap.

### New benchmark-fidelity item

**Add explicit cold vs warm vs “same candles/new rows” benchmark modes.**

The current benches already do a route probe before the measured body, which implicitly warms some state. That is useful, but it means you still need separate labels if you want to isolate:

1. cold runtime creation
2. warm same-dataset same-row-set reuse
3. warm same-dataset new-row-set repack cost
4. warm same-dataset new-combo-only cost

---

## Additional concrete improvements

## 1. Stop copying slice metrics back in the metrics-only API

### Why this matters

The current public “basic” function:

```rust
pub fn eval_vcc_metrics_gpu_kernel_device_rows(...) -> Result<Vec<Metrics>, String>
```

still calls the detailed function and drops the extra outputs.

That means the common path may still pay for:

- `d_slice_out`
- `host_slice_out`
- `copy_to(&mut host_slice_out)`
- `step_return_sum`/`step_return_sumsq` decode

If your optimizer/validation code only needs `Metrics`, this is a direct avoidable cost.

### Recommended edit

Introduce an internal execution mode:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VccGpuEvalMode {
    MetricsOnly,
    Detailed,
}
```

Then split the common executor so the D→H slice readback happens only in `Detailed`.

### Safe first-pass implementation

Keep the kernel ABI unchanged in phase 1.

That means:

- still allocate `d_slice_out` if the kernel requires a valid pointer
- **skip copying `d_slice_out` back to host** in `MetricsOnly`
- **skip allocating/initializing `host_slice_out`** in `MetricsOnly`
- decode only the main metrics rows

### Example structure

```rust
#[derive(Clone, Debug)]
pub struct VccGpuMetricEval {
    pub metrics: Metrics,
    pub step_return_sum: f64,
    pub step_return_sumsq: f64,
}

fn eval_vcc_metrics_gpu_kernel_core(
    device_id: u32,
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcc: &VccStrategyParams,
    mode: VccGpuEvalMode,
) -> Result<Vec<VccGpuMetricEval>, String> {
    // ... existing validation and runtime lookup ...

    let need_slice_host_readback = matches!(mode, VccGpuEvalMode::Detailed);

    // Launch kernel exactly as today.

    rt.stream.synchronize().map_err(|e| e.to_string())?;

    let host_out = rt.combo_scratch.host_out_f32_mut(out_len)?;
    d_out.copy_to(host_out).map_err(|e| e.to_string())?;

    let slice_host: Option<&[f64]> = if need_slice_host_readback {
        let host_slice_out = rt.combo_scratch.host_slice_out_f64_mut(slice_out_len)?;
        d_slice_out.copy_to(host_slice_out).map_err(|e| e.to_string())?;
        Some(host_slice_out)
    } else {
        None
    };

    let mut metrics = Vec::with_capacity(combo_count);
    for i in 0..combo_count {
        let base = i * out_stride;
        let (step_return_sum, step_return_sumsq) = match slice_host {
            Some(slice) => {
                let sb = i * 2;
                (slice[sb], slice[sb + 1])
            }
            None => (0.0, 0.0),
        };

        metrics.push(VccGpuMetricEval {
            metrics: decode_non_double_ma_gpu_metrics_row(&host_out[base..base + out_stride]),
            step_return_sum,
            step_return_sumsq,
        });
    }

    Ok(metrics)
}

pub fn eval_vcc_metrics_gpu_kernel_device_rows_detailed(...) -> Result<Vec<VccGpuMetricEval>, String> {
    eval_vcc_metrics_gpu_kernel_core(..., VccGpuEvalMode::Detailed)
}

pub fn eval_vcc_metrics_gpu_kernel_device_rows(...) -> Result<Vec<Metrics>, String> {
    eval_vcc_metrics_gpu_kernel_core(..., VccGpuEvalMode::MetricsOnly)
        .map(|rows| rows.into_iter().map(|row| row.metrics).collect())
}
```

### Phase-2 variant

If you later confirm the kernel can safely accept a null or sentinel pointer for `slice_out`, then you can also:

- skip `d_slice_out` allocation in `MetricsOnly`
- skip the device write itself

That is a kernel-ABI change, so I would keep it phase 2.

---

## 2. Add strict combo/time validation

### Why this matters

`validate_inputs_device_rows(...)` currently validates only lengths and matrix shapes.

It does not validate row index or time-window contents.

That means malformed combo batches can still reach the kernel.

### Recommended edit

Add a second validator:

```rust
fn validate_combo_ranges(
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
) -> Result<(), String> {
    let cols_i32 = i32::try_from(signals.cols)
        .map_err(|_| "VCC GPU cols exceed i32 range".to_string())?;
    let fast_rows_i32 = i32::try_from(signals.fast_rows)
        .map_err(|_| "VCC GPU fast_rows exceed i32 range".to_string())?;
    let trend_rows_i32 = i32::try_from(signals.trend_rows)
        .map_err(|_| "VCC GPU trend_rows exceed i32 range".to_string())?;

    for i in 0..combos.fast_row_index.len() {
        let fast_idx = combos.fast_row_index[i];
        let trend_idx = combos.trend_row_index[i];
        let start_t = combos.start_t[i];
        let metric_start_t = combos.metric_start_t[i];
        let end_t = combos.end_t[i];

        if fast_idx < 0 || fast_idx >= fast_rows_i32 {
            return Err(format!(
                "VCC GPU fast_row_index[{i}]={} out of range [0,{})",
                fast_idx, fast_rows_i32
            ));
        }
        if trend_idx < 0 || trend_idx >= trend_rows_i32 {
            return Err(format!(
                "VCC GPU trend_row_index[{i}]={} out of range [0,{})",
                trend_idx, trend_rows_i32
            ));
        }
        if start_t < 0 || metric_start_t < 0 || end_t < 0 {
            return Err(format!(
                "VCC GPU combo window[{i}] has negative index: start={} metric_start={} end={}",
                start_t, metric_start_t, end_t
            ));
        }
        if start_t > metric_start_t || metric_start_t > end_t {
            return Err(format!(
                "VCC GPU combo window[{i}] is unordered: start={} metric_start={} end={}",
                start_t, metric_start_t, end_t
            ));
        }
        if end_t > cols_i32 {
            return Err(format!(
                "VCC GPU combo window[{i}] end_t={} exceeds cols={}",
                end_t, cols_i32
            ));
        }
    }

    Ok(())
}
```

### Performance-safe deployment options

You have three reasonable choices:

1. always-on validation for production safety
2. `cfg(debug_assertions)` only
3. always-on for external/public entrypoints, debug-only for internal hot paths

My preference here is **always-on for the public entrypoint**, because the loop is linear in combo count and cheap compared with the kernel work it protects.

---

## 3. Strengthen cache identity and invalidation

### Why this matters

If you cache by host pointer + length only, then this sequence can go wrong:

1. caller reuses the same `Vec` allocation
2. caller mutates values in place
3. cached VCC buffers appear valid by address/len
4. stale device data gets reused

That is a correctness bug, not just a cache miss issue.

### Recommended hierarchy

### Best option

Add explicit epochs/IDs to `VccGpuDeviceSignalMatrices`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VccCacheEpoch(pub u64);

pub struct VccGpuDeviceSignalMatrices<'a> {
    pub close_f64: &'a [f64],
    pub close_epoch: Option<VccCacheEpoch>,
    pub cols: usize,
    pub fast_rows: usize,
    pub trend_rows: usize,
    pub fast_squeeze_rows: &'a [u64],
    pub fast_squeeze_epoch: Option<VccCacheEpoch>,
    pub fast_squeeze_host_row_major: Option<&'a [f32]>,
    pub fast_momentum_rows: &'a [u64],
    pub fast_momentum_epoch: Option<VccCacheEpoch>,
    pub fast_momentum_host_row_major: Option<&'a [f32]>,
    pub trend_rows_ptr: &'a [u64],
    pub trend_epoch: Option<VccCacheEpoch>,
    pub trend_host_row_major: Option<&'a [f32]>,
}
```

Then prefer the explicit epoch in cache keys.

### Acceptable fallback

When no epoch is available, use pointer+len keys but add debug assertions/checksums:

```rust
#[cfg(debug_assertions)]
fn sample_checksum_f32(x: &[f32]) -> u64 {
    let mut acc = 0u64;
    let stride = (x.len() / 8).max(1);
    let mut i = 0usize;
    while i < x.len() {
        acc ^= u64::from(x[i].to_bits()).rotate_left((i % 63) as u32);
        i = i.saturating_add(stride);
    }
    acc ^ (x.len() as u64)
}
```

This should not be your correctness foundation, but it is a good guardrail while developing the cache.

### Important recommendation

If the same logical signal matrices already have a natural owner object upstream, put the epoch there and flow it into VCC instead of computing hashes locally.

---

## 4. Call out the thread-local cache limitation explicitly

### Why this matters

`VCC_GPU_RUNTIME` is thread-local.

That means:

- reuse is per-thread, not process-wide
- Criterion may show strong wins even if production call sites hop threads
- tests/benches can inherit warm state from previous work on the same thread

### Recommended addition to the plan

Document the assumption clearly:

> VCC warm-path cache hits are thread-affine under the current runtime design.

### Recommended helper

Add a test/bench-only reset function:

```rust
#[cfg(any(test, feature = "bench-debug"))]
pub fn clear_vcc_gpu_runtime_cache(device_id: Option<u32>) {
    VCC_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        match device_id {
            Some(id) => {
                runtimes.remove(&id);
            }
            None => {
                runtimes.clear();
            }
        }
    });
}
```

This gives you deterministic cold-start measurement and cleaner test isolation.

### Optional beyond-scope extension

If production profiling later shows thread hopping is common, then consider a process-global cache/pool.

I would **not** do that first.

---

## 5. Use async, stream-bound allocation/upload where your `cust` version supports it

### Why this matters

`VccCudaRuntime` already uses a non-blocking stream, but the current VCC code still performs synchronous-looking allocations and uploads inside the hot path.

Even when overlap is limited, it is better to keep staging work attached to the intended stream.

### Recommended policy

- first allocation on a known stream: prefer async allocation/upload APIs if your current `cust` version supports them
- steady-state reuse: prefer resize-once and reuse
- only synchronize once per call, after the final kernel launch, unless correctness requires an earlier sync

### Recommended practical rule

Because `cust` APIs vary a bit by version, implement this conservatively:

1. use cached buffers first
2. for first allocation, use stream-bound async APIs where available
3. otherwise keep current sync upload, but still retain the cache so the cost disappears on the warm path

### What I would not do yet

- multiple streams for VCC packing/eval
- host-side thread fanout to feed a single strategy-local kernel
- speculative overlap work before the cache is benchmarked

---

## 6. Replace zero-filled host readbacks with repo-standard uninitialized helpers when safe

### Why this matters

The current code does:

```rust
let mut host_out = vec![0.0f32; out_len];
d_out.copy_to(&mut host_out)?;
let mut host_slice_out = vec![0.0f64; slice_out_len];
d_slice_out.copy_to(&mut host_slice_out)?;
```

Those fills are unnecessary **if** `copy_to(...)` fully writes every element.

### What I would change from the original plan

The original plan talked about host-buffer reuse. I would add a stricter copy-elision option:

- use a shared `alloc_policy` helper for uninitialized host readback buffers if available
- otherwise keep the reuse cache but do not introduce ad-hoc unsafe unless you can unit-test it heavily

### Safe adoption order

1. **reuse buffers first**
2. **switch to repo helper** if `alloc_policy.rs` already provides one
3. only add local `MaybeUninit` code if the shared helper does not exist and you can fully test it

### Suggested abstraction

```rust
struct HostReadbackScratch {
    out_f32: Vec<f32>,
    slice_f64: Vec<f64>,
}

impl HostReadbackScratch {
    fn ensure_out_f32(&mut self, len: usize) -> &mut [f32] {
        if self.out_f32.len() != len {
            self.out_f32.resize(len, 0.0);
        }
        self.out_f32.as_mut_slice()
    }

    fn ensure_slice_f64(&mut self, len: usize) -> &mut [f64] {
        if self.slice_f64.len() != len {
            self.slice_f64.resize(len, 0.0);
        }
        self.slice_f64.as_mut_slice()
    }
}
```

Then later swap the implementation behind `ensure_*` to an `alloc_policy` helper without changing the call sites.

---

## 7. Reuse kernel function lookups inside the eval path

### Why this matters

The current code resolves:

- `pack_rows_u64_ptr_f32` on every pack call
- `vcc_backtest_metrics_f32` on every eval call

This is small compared with staging today, but after caching lands it becomes a more noticeable micro-cost.

### Safe first-pass edit

Do not fight lifetimes by forcing long-lived stored `Function<'_>` handles if that becomes awkward.

Instead, just resolve once per eval and pass the handle into the helper:

```rust
fn launch_pack_rows_u64_ptr_f32_with_kernel(
    stream: &Stream,
    kernel: &cust::function::Function<'_>,
    row_ptrs: &DeviceBuffer<u64>,
    rows: usize,
    cols: usize,
    out_row_major: &mut DeviceBuffer<f32>,
) -> Result<(), String> {
    // existing body, but no module.get_function lookup here
}
```

Inside the eval path:

```rust
let pack_kernel = rt
    .module
    .get_function("pack_rows_u64_ptr_f32")
    .map_err(|e| format!("VCC GPU missing pack_rows_u64_ptr_f32 kernel: {e}"))?;

let metrics_kernel = rt
    .module
    .get_function("vcc_backtest_metrics_f32")
    .map_err(|e| format!("VCC GPU missing vcc_backtest_metrics_f32 kernel: {e}"))?;
```

Then reuse `pack_kernel` for all three pack launches.

---

## 8. Add benchmark modes that separate cache layers

### Why this matters

Once you introduce caching, a single “GPU benchmark” label is not enough.

You need to know whether the speedup came from:

- device price-state reuse
- packed row reuse
- combo scratch reuse
- all of the above together

### Recommended labels in `gpu_native_expansion_opt.rs`

For VCC, add four benchmark variants for at least one candle size:

- `cold/small`
- `warm_same_rows/small`
- `warm_new_rows/small`
- `warm_new_combos_only/small`

### Recommended labels in `composite_validation_workflow.rs`

For VCC, add at least:

- `gpu_kernel/vcc/50k/8candidates/warm`
- `gpu_kernel/vcc/100k/32candidates/warm`
- `gpu_kernel/vcc/100k/32candidates/cold`
- `gpu_kernel/vcc/100k/32candidates/wide_rows`

### Implementation pattern

Use the cache-clear helper for cold runs:

```rust
group.bench_function("cold/small", |b| {
    b.iter(|| {
        ta_app_core::clear_vcc_gpu_runtime_cache(Some(0));
        let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
            req.clone(),
            candles,
            &cancel,
            None,
        )
        .expect("vcc cold gpu bench run");
        assert!(matches!(res.backend_used, BackendUsed::Gpu));
    })
});
```

For warm row reuse:

```rust
group.bench_function("warm_same_rows/small", |b| {
    let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
        req.clone(),
        candles,
        &cancel,
        None,
    )
    .expect("warmup run");

    b.iter(|| {
        let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
            req.clone(),
            candles,
            &cancel,
            None,
        )
        .expect("vcc warm gpu bench run");
        assert!(matches!(res.backend_used, BackendUsed::Gpu));
    })
});
```

For warm new-row-set runs, alternate between two candidate/request shapes that keep the same candles but change row selections.

---

## 9. Add VCC “parameter-shape” benchmark coverage, not just period coverage

### Why this matters

The current VCC candidate builder uses `VccStrategyParams::default()` for every candidate.

But the kernel behavior depends on additional VCC config flags/values such as:

- `require_momentum_accel`
- `confirm_bars`
- `max_trade_bars`

These directly influence branch behavior and trade hold duration in the kernel.

If you benchmark only default params, you can miss performance cliffs tied to control-flow divergence.

### Recommended new candidate helper

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_vcc_candidates_shape_mix(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);

        let mut vcc = ta_app_core::VccStrategyParams::default();
        vcc.require_momentum_accel = (i % 2) == 0;
        vcc.confirm_bars = 1u16.saturating_add(i_u16 % 4);
        vcc.max_trade_bars = 8u16.saturating_add(i_u16 % 32);

        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: Some(vcc),
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}
```

### Usage recommendation

Keep two VCC benchmark families:

1. **period-scaling family** — isolates matrix width/row count effects
2. **param-shape family** — isolates kernel branch behavior

---

## 10. Make the route metadata richer than just “fallback happened”

### Why this matters

If you add route metadata, you should distinguish between:

- GPU not attempted because descriptor policy blocked it
- GPU not attempted because prepared CUDA inputs were absent
- GPU attempted and device-input path failed
- GPU attempted and host-input fallback succeeded
- GPU attempted and full CUDA path failed, CPU fallback used

That gives you real route evidence instead of a single undifferentiated fallback string.

### Recommended enum

```rust
#[cfg(feature = "cuda")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VccCudaRouteKind {
    CpuOnly,
    GpuDeviceInputs,
    GpuHostInputs,
    CpuFallbackNoPreparedInputs,
    CpuFallbackDescriptorPolicy,
    CpuFallbackCudaError,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug, Default)]
pub struct VccCudaRouteMeta {
    pub route_kind: Option<VccCudaRouteKind>,
    pub attempted_gpu: bool,
    pub used_gpu: bool,
    pub fallback_reason: Option<String>,
}
```

This is especially useful in bench logs and ignored parity diagnostics.

---

## 11. Optional beyond-scope extension: align VCC price-state with shared resident market caches

This goes beyond the narrow strategy-local prompt, but it is still a legitimate future optimization.

The broader repo already has a shared GPU runtime/resident-market pattern in the Double-MA family. If VCC eventually plugs into that same price-state residency layer, you can remove a second family of close/log-return staging.

I would **not** start here.

I would only consider it **after**:

1. VCC local cache is implemented
2. VCC optimizer + validation benches exist
3. VCC warm-path payoff is measured
4. parity tests are no longer diagnostic-only

This keeps the first patch small and strategy-local.

---

## What I would explicitly revise in the original checklist

Add these items near the top of the implementation tracker:

### New checklist items

- [ ] Split VCC GPU executor into `MetricsOnly` vs `Detailed` host-readback modes.
- [ ] In `MetricsOnly`, skip `host_slice_out` allocation and D→H copy.
- [ ] Add strict combo row/time range validation to the public VCC GPU entrypoint.
- [ ] Decide the cache identity policy: explicit epoch preferred, pointer+len only as fallback.
- [ ] Add `clear_vcc_gpu_runtime_cache()` for deterministic cold/warm bench coverage.
- [ ] Add `cold`, `warm_same_rows`, `warm_new_rows`, and `warm_new_combos_only` VCC bench labels.
- [ ] Add VCC “param-shape” candidate generation that varies `require_momentum_accel`, `confirm_bars`, and `max_trade_bars`.
- [ ] Resolve `pack_rows_u64_ptr_f32` only once per eval call, not once per pack launch.
- [ ] Upgrade host readback reuse to repo-standard uninitialized helpers if `alloc_policy.rs` already provides them.
- [ ] Document the thread-local nature of VCC runtime reuse so app-path claims are not overstated.

---

## Revised implementation order

If I were implementing this now, I would slightly reorder the original plan:

### Phase 1A — correctness + immediate copy-elision

1. add strict combo/time validation
2. split `MetricsOnly` vs `Detailed` readback behavior
3. add checked integer conversions for kernel launch scalars

### Phase 1B — buffer lifetime improvements

4. add price-state cache
5. add packed-matrix caches
6. add combo/output scratch reuse
7. reuse host readback buffers

### Phase 2 — route transparency

8. add richer route metadata enums/structs
9. preserve current public signatures via compatibility wrappers

### Phase 3 — benchmark fidelity

10. add VCC optimizer Criterion coverage
11. add dedicated heavier VCC GPU validation groups
12. add cold vs warm vs new-row-set modes
13. add param-shape VCC benches

### Phase 4 — optional deeper optimization

14. switch to async stream-bound allocation/upload where supported
15. replace host zero-fill readbacks with repo-standard uninitialized helpers
16. evaluate direct row-remap kernel ABI
17. evaluate shared resident-market integration

---

## Bottom line

The two most important additions are:

1. **stop copying detailed slice metrics back when the caller only wants `Metrics`**
2. **add strict range validation before the kernel to protect correctness while you make the hot path more stateful**

Those are the main things I would strengthen relative to the original document.
