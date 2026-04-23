# VEMR GPU-native optimization plan — addendum and missed items

This addendum captures improvements I would add to the main VEMR GPU-native optimization plan after a second audit of the attached files. It focuses on practical, parity-preserving additions that further reduce unnecessary copies, improve warm-path reuse, and tighten correctness guards.

## Repo-state note

This addendum should now be read against the current repo state, not the older
Section 5 baseline.

As of the current `composite_cpu.rs` standalone unified-GPU route:

- `include_all=true` already returns `all_scores` alongside the full native candidate rows
- the requested standalone `GpuKernel` route already supports host-side:
  - `extra_shortlist_objectives`
  - `shortlist_union_mode`
  - `shortlist_confidence_mode`
- those features are exposed in the frontend advanced controls for the supported standalone routes

What still does **not** exist here is Double-MA exact-kernel parity:

- no shortlist slice / benchmark / neighborhood diagnostics parity
- no `score_observation_context`
- no attempt to force VEMR into the Double-MA exact-kernel metric schema

So the recommendations below should be read as **strategy-local runtime / residency /
copy-elision improvements on top of that already-expanded product surface**, not as a plan to add
the recently completed host/result-layer extras.

## 1. What I would explicitly add to the main plan

### 1.1 Strengthen `validate_inputs` with row-index and range validation

The current VEMR GPU input validation checks:

- non-empty series
- `close_f32.len() == cols`
- non-null device pointers for z-score, Bollinger, and ATR matrices
- non-zero row counts and strides
- equal combo-vector lengths

But it does **not** currently validate that:

- `z_row_index < zscore_rows`
- `bb_row_index < bb_rows`
- `atr_row_index < atr_rows`
- `start_t` / `end_t` are non-negative
- `start_t <= end_t`
- `end_t <= cols`

That is a correctness gap first, and a performance/debuggability issue second. If an upstream batch builder ever passes a bad row index or a bad time window, the GPU kernel may perform out-of-bounds reads or produce undefined results.

### Recommended edit in `vemr_gpu_kernel.rs`

```rust
fn validate_inputs(
    signals: &VemrGpuDeviceSignalMatrices<'_>,
    combos: &VemrGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("VEMR GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f32.len() != n {
        return Err("VEMR GPU close length mismatch".to_string());
    }
    if signals.zscore_row_major_ptr == 0
        || signals.bb_upper_row_major_ptr == 0
        || signals.bb_middle_row_major_ptr == 0
        || signals.bb_lower_row_major_ptr == 0
        || signals.atr_row_major_ptr == 0
    {
        return Err("VEMR GPU signal pointer is null".to_string());
    }
    if signals.zscore_rows == 0
        || signals.bb_rows == 0
        || signals.atr_rows == 0
        || signals.zscore_stride == 0
        || signals.bb_stride == 0
        || signals.atr_stride == 0
    {
        return Err("VEMR GPU signal matrix shape is invalid".to_string());
    }

    let combos_len = combos.z_row_index.len();
    if combos.bb_row_index.len() != combos_len
        || combos.atr_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("VEMR GPU combo vector length mismatch".to_string());
    }

    for i in 0..combos_len {
        let z = combos.z_row_index[i];
        let bb = combos.bb_row_index[i];
        let atr = combos.atr_row_index[i];
        let start = combos.start_t[i];
        let end = combos.end_t[i];

        if z < 0 || usize::try_from(z).ok().map_or(true, |v| v >= signals.zscore_rows) {
            return Err(format!("VEMR GPU z_row_index out of bounds at combo {}", i));
        }
        if bb < 0 || usize::try_from(bb).ok().map_or(true, |v| v >= signals.bb_rows) {
            return Err(format!("VEMR GPU bb_row_index out of bounds at combo {}", i));
        }
        if atr < 0 || usize::try_from(atr).ok().map_or(true, |v| v >= signals.atr_rows) {
            return Err(format!("VEMR GPU atr_row_index out of bounds at combo {}", i));
        }
        if start < 0 || end < 0 {
            return Err(format!("VEMR GPU eval range is negative at combo {}", i));
        }
        let start_u = usize::try_from(start).map_err(|_| "start_t conversion failed".to_string())?;
        let end_u = usize::try_from(end).map_err(|_| "end_t conversion failed".to_string())?;
        if start_u > end_u || end_u > n {
            return Err(format!("VEMR GPU eval range invalid at combo {}", i));
        }
    }

    Ok(())
}
```

### Why I would add this

This is the cleanest missed correctness safeguard. It prevents hard-to-debug GPU failures and does not reduce feature coverage.

---

### 1.2 Add a bridge-stage resident `close` / `lr` cache inside `VemrCudaRuntime`

The main plan already proposed extending `VemrGpuDeviceSignalMatrices` with optional device-resident `close` and `lr` pointers. I would still keep that as the **end-state API**.

But I would add an intermediate bridge implementation too:

- if callers still only have `close_f32: &[f32]`, cache an uploaded `d_close` and derived `d_lr` inside `VemrCudaRuntime`
- reuse those buffers across repeated evaluations as long as the underlying series has not changed

This gives immediate warm-path benefit without requiring every upstream caller to switch to device-resident close/lr on day one.

### Suggested runtime extension

```rust
struct VemrSeriesCacheKey {
    len: usize,
    fingerprint: u64,
}

struct VemrResidentSeries {
    key: VemrSeriesCacheKey,
    d_close: DeviceBuffer<f32>,
    d_lr: DeviceBuffer<f32>,
}

struct VemrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    resident_series: Option<VemrResidentSeries>,
    scratch: VemrCudaScratch,
    launch_caps: DeviceLaunchCaps,
    fn_caps: VemrFunctionCaps,
}
```

### Fingerprint helper

```rust
fn fingerprint_f32_slice(values: &[f32]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    values.len().hash(&mut h);
    for &v in values.iter().step_by((values.len() / 64).max(1)) {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}
```

### Resident series helper

```rust
fn ensure_resident_close_lr(
    rt: &mut VemrCudaRuntime,
    close_f32: &[f32],
) -> Result<(&DeviceBuffer<f32>, &DeviceBuffer<f32>), String> {
    let key = VemrSeriesCacheKey {
        len: close_f32.len(),
        fingerprint: fingerprint_f32_slice(close_f32),
    };

    let needs_refresh = rt
        .resident_series
        .as_ref()
        .map(|cached| cached.key.len != key.len || cached.key.fingerprint != key.fingerprint)
        .unwrap_or(true);

    if needs_refresh {
        let d_close = DeviceBuffer::from_slice(close_f32).map_err(|e| e.to_string())?;
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(close_f32.len()) }
            .map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, 0, &d_close, &mut d_lr, close_f32.len())?;
        rt.resident_series = Some(VemrResidentSeries { key, d_close, d_lr });
    }

    let resident = rt
        .resident_series
        .as_ref()
        .ok_or_else(|| "VEMR resident series cache missing".to_string())?;
    Ok((&resident.d_close, &resident.d_lr))
}
```

### Why I would add this

This lets you harvest most of the warm-path gain now, before every upstream caller is refactored to pass device pointers.

---

### 1.3 Prefer grow-only scratch reuse over exact-shape reuse

The main plan proposed exact-shape scratch reuse. I would refine that recommendation:

- use **grow-only capacity reuse** as the default
- do not shrink on smaller requests
- only reallocate when `requested > capacity`

This is more robust for real workloads because candidate counts and `top_k` values can vary slightly across optimizer and validation calls. Exact-shape reuse still reallocates too often.

### Suggested helper shape

```rust
impl VemrCudaScratch {
    fn ensure_len_f32(
        slot: &mut Option<DeviceBuffer<f32>>,
        required: usize,
    ) -> Result<(), String> {
        let needs_alloc = slot.as_ref().map(|buf| buf.len() < required).unwrap_or(true);
        if needs_alloc {
            *slot = Some(
                unsafe { DeviceBuffer::<f32>::uninitialized(required) }
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }

    fn ensure_len_i32(
        slot: &mut Option<DeviceBuffer<i32>>,
        required: usize,
    ) -> Result<(), String> {
        let needs_alloc = slot.as_ref().map(|buf| buf.len() < required).unwrap_or(true);
        if needs_alloc {
            *slot = Some(
                unsafe { DeviceBuffer::<i32>::uninitialized(required) }
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }
}
```

### Why I would add this

This mirrors the stronger warm-path reuse pattern already visible in `gpu_backtest_kernel.rs`, where scratch buffers are retained and grown as needed rather than recreated per call.

---

### 1.4 Cache function lookup and launch-cap metadata per runtime

The current VEMR path still performs repeated module symbol lookup and repeated launch-cap detection for:

- `compute_log_returns_f32`
- `vemr_backtest_metrics_f32`
- `select_topk_vemr_metric_rows_f32`
- `select_topk_vemr_metric_rows_from_candidates_f32`
- `gather_topk_metric_rows_f32`

Those costs are not as large as H2D uploads or scratch allocation, but they are pure repeated work.

The safe version of this is not to cache raw `Function<'_>` handles if lifetimes become awkward. The simpler stage is:

- cache `DeviceLaunchCaps` once at runtime construction
- cache per-kernel launch metadata or preferred block-size choices once
- optionally wrap symbol lookup behind a small runtime accessor so you do not redetect capabilities every call

### Suggested metadata cache

```rust
struct VemrFunctionCaps {
    compute_lr: KernelLaunchCaps,
    metrics: KernelLaunchCaps,
    topk_select: KernelLaunchCaps,
    topk_select_candidates: KernelLaunchCaps,
    gather_rows: KernelLaunchCaps,
}
```

### Why I would add this

This is low-risk cleanup after the bigger staging fixes. It helps the warm app-path without changing semantics.

---

### 1.5 Add explicit combo-tag caching

The current top-k path still needs a combo-tag buffer, and the cleanest staging policy is:

- store a grow-only device `combo_tags` buffer in scratch
- fill it only when `combo_count` grows or the current contents are invalid
- keep a small `combo_tags_len` field on the scratch/runtime

That saves one host allocation and one H2D upload per evaluation in the device-top-k path.

### Suggested helper

```rust
fn ensure_combo_tags(
    rt: &mut VemrCudaRuntime,
    combo_count: usize,
) -> Result<&DeviceBuffer<i32>, String> {
    VemrCudaScratch::ensure_len_i32(&mut rt.scratch.d_combo_tags, combo_count)?;
    let needs_fill = rt.scratch.combo_tags_len < combo_count;
    if needs_fill {
        let mut host = Vec::with_capacity(combo_count);
        host.extend((0..combo_count).map(|i| (i as i32) + 1));
        rt.scratch
            .d_combo_tags
            .as_mut()
            .unwrap()
            .copy_from(&host)
            .map_err(|e| e.to_string())?;
        rt.scratch.combo_tags_len = combo_count;
    }
    Ok(rt.scratch.d_combo_tags.as_ref().unwrap())
}
```

---

## 2. Additional copy-elision opportunities I would add

### 2.1 Make the top-k host readback buffers uninitialized

This one is directly in scope with bundle 16. The current code zero-fills `host_combo_tags` and `host_top_metrics` immediately before `copy_to(...)`, even though the device copy fully overwrites every element.

### Safe replacement

```rust
let mut host_combo_tags: Vec<i32> = alloc_uninit_copy(topk_select);
let mut host_top_metrics: Vec<f32> = alloc_uninit_copy(
    topk_select.saturating_mul(VEMR_METRICS_STRIDE)
);

d_top_combo.copy_to(&mut host_combo_tags).map_err(|e| e.to_string())?;
d_top_metrics.copy_to(&mut host_top_metrics).map_err(|e| e.to_string())?;
```

If your shared allocation helpers prefer an explicit `MaybeUninit` API instead of `alloc_uninit_copy`, use that. The key requirement is unchanged: only do this because `copy_to(...)` overwrites every element before any read.

---

### 2.2 Do not add broad `MaybeUninit` to behaviorally observed fills

I would also explicitly keep this caution in the main plan:

- `build_vemr_desired_positions` can use prefix-init + uninit-tail only if the tail is guaranteed overwritten before read
- any series whose warmup `NaN` or `0.0` is behaviorally observable must stay initialized in those regions
- do **not** replace every `vec![0; n]` mechanically

This matches the review-map rule that uninitialized allocation is safe only when every element is definitely written before read.

---

## 3. Benchmark and perf-gate additions I would now include

### 3.1 Add a VEMR optimizer bench that measures pure device-top-k separately from full-readback/heatmap

The current `gpu_native_expansion_opt.rs` base request sets:

- `top_k = Some(20)`
- `include_all = Some(false)`
- `heatmap_bins = Some(64)`

That does **not** represent the pure device-top-k VEMR path, because the VEMR device-top-k path is gated on `heatmap_bins == 0`.

I would add two explicit VEMR GPU benchmark shapes:

1. `topk_no_heatmap` — measures the intended reduced-readback path
2. `full_or_heatmap` — measures the broader result-shaping path

### Suggested request builders

```rust
fn make_vemr_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.vemr = Some(ta_app_core::VemrStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}

fn make_vemr_req_topk(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = make_vemr_req(backend, fast_range, slow_range);
    req.top_k = Some(20);
    req.include_all = Some(false);
    req.heatmap_bins = Some(0);
    req
}

fn make_vemr_req_heatmap(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = make_vemr_req(backend, fast_range, slow_range);
    req.top_k = Some(20);
    req.include_all = Some(false);
    req.heatmap_bins = Some(64);
    req
}
```

---

### 3.2 Add a benchmark that intentionally exercises VEMR GPU blockers

Because the VEMR GPU kernel currently rejects overlays / stop-loss / take-profit, I would add one small route bench or route test that deliberately trips those blockers. This is not to optimize the failure path; it is to make sure fallback visibility does not silently regress.

### Example negative-route case

```rust
fn make_vemr_blocked_req(backend: Backend) -> CompositeCpuRequest {
    let mut req = make_vemr_req(backend, (5, 20, 1), (20, 80, 1));
    let mut strategy = StrategyConfig::default();
    strategy.stop_loss = Some(0.02);
    req.strategy = strategy;
    req
}
```

Then assert either:

- the benchmark route probe shows fallback clearly, or
- the execution returns the expected blocker reason

---

### 3.3 Add a non-default z-score configuration benchmark

VEMR’s direct CUDA z-score specialization is only the fastest path for the default shape. I would add one benchmark with a non-default `z_ma_type` or `z_devtype` to quantify the cliff.

This does not mean you should optimize that path immediately. It means you should measure whether it is common enough to justify more specialization.

---

## 4. Optional changes beyond the original prompt scope

These are useful, but I would treat them as second-wave work after the main plan lands.

### 4.1 Align VEMR runtime patterns with `gpu_backtest_kernel` warm-path patterns

`gpu_backtest_kernel.rs` already shows a more mature warm-path model:

- resident market-series caching
- grow-only scratch buffers
- dataset-keyed reusable `d_lr` / `d_ret1`
- reusable host scratch / copy-elision helpers
- promoted single-pass product-path policy in the shared Double-MA backend

Separately, the repo now also exposes host/result-layer shortlist extras on the standalone
unified-GPU routes themselves. That newer product surface does not remove the value of this
runtime-alignment section; it just means the remaining gap is now more about residency, scratch,
and transfer discipline than about basic shortlist/result capabilities.

A longer-horizon improvement would be to make VEMR converge toward those conventions instead of growing a one-off local runtime design forever.

I would **not** turn this into a rewrite right away. But I would keep the local VEMR scratch/residency API shaped similarly so convergence later is easy.

---

### 4.2 Pinned host staging / async copies for unavoidable transfers

If profiling later shows H2D copies still dominate after reuse is in place, the next optional step is:

- pinned host memory for frequently uploaded slices
- async copies on the VEMR stream
- overlap upload and prep when safe

I would only do this after the simpler reuse and residency work. It adds complexity and depends on what `cust` and your current repo patterns support cleanly.

---

### 4.3 Host-sort removal only if the kernel already guarantees order

The current top-k path sorts `top_rows` on host after readback. If the selected/gathered rows are already ordered by objective on device, that sort can be removed.

I would mark this as **measurement-gated and correctness-gated** because `top_k` is small and the current sort cost is probably minor compared with transfers.

---

### 4.4 Optional bridge to caller-provided generation IDs

If upstream code can cheaply provide a stable market-series generation or dataset revision ID, use that instead of a content fingerprint for resident `close/lr` caching. That avoids fingerprint cost and makes cache invalidation exact.

This is a nice beyond-prompt enhancement, but it depends on adjacent orchestration files that were not part of the VEMR bundle.

---

## 5. Additional tests I would add to the checklist

The main plan already had parity and route tests. I would add these too.

### 5.1 `validate_inputs` negative tests

Add unit tests that confirm the GPU path rejects:

- `z_row_index == zscore_rows`
- `bb_row_index == bb_rows`
- `atr_row_index == atr_rows`
- `start_t < 0`
- `end_t < start_t`
- `end_t > cols`

### 5.2 Resident close/lr cache invalidation tests

Add tests that confirm:

- same close series reuses resident buffers
- changed close series refreshes resident buffers
- same length but changed values invalidates correctly
- increased length grows buffers once and reuses thereafter

### 5.3 Device-top-k readback safety tests

If you switch to uninitialized host readback buffers, add a test that exercises the readback path in debug builds and confirms no poison/sentinel value is ever observed after `copy_to(...)`.

### 5.4 Route-assert tests in benchmark-like harnesses

Add small tests or debug asserts so VEMR “GPU” benches fail loudly when they silently become CPU/fallback benches.

---

## 6. Priority order for the addendum items

If you want the best extra payoff beyond the original plan, I would prioritize them in this order:

1. strengthen `validate_inputs`
2. add bridge-stage resident `close/lr` caching inside `VemrCudaRuntime`
3. switch scratch reuse from exact-shape to grow-only reuse
4. add top-k host readback copy-elision
5. cache function/launch metadata
6. expand VEMR benchmark coverage with `topk_no_heatmap`, blocker, and non-default z-score cases
7. only then consider pinned memory / async copy overlap / host-sort removal

---

## 7. Checklist additions

Add these items to the bottom checklist in the main plan.

### Correctness / safety

- [ ] `validate_inputs` rejects out-of-range `z_row_index`
- [ ] `validate_inputs` rejects out-of-range `bb_row_index`
- [ ] `validate_inputs` rejects out-of-range `atr_row_index`
- [ ] `validate_inputs` rejects negative `start_t` / `end_t`
- [ ] `validate_inputs` rejects `start_t > end_t`
- [ ] `validate_inputs` rejects `end_t > cols`

### Warm-path residency / reuse

- [ ] `VemrCudaRuntime` stores resident `d_close`
- [ ] `VemrCudaRuntime` stores resident `d_lr`
- [ ] resident close/lr cache invalidates correctly on changed input data
- [ ] scratch buffers are grow-only, not exact-shape-only
- [ ] combo-tag device buffer is cached and reused
- [ ] per-kernel launch metadata is cached once per runtime

### Copy-elision

- [ ] top-k host readback uses uninitialized buffers instead of zero-filled vectors
- [ ] all new uninitialized buffers are covered by overwrite-before-read reasoning
- [ ] debug/test coverage exists for any new uninitialized host buffer helper

### Benchmarks / gates

- [ ] VEMR optimizer Criterion bench exists
- [ ] VEMR optimizer bench includes `topk_no_heatmap`
- [ ] VEMR optimizer bench includes heatmap/full-readback shape
- [ ] VEMR validation bench asserts actual GPU route
- [ ] VEMR validation bench includes a larger scaling case
- [ ] VEMR blocker/fallback route case is benchmarked or route-tested
- [ ] VEMR non-default z-score route is benchmarked

### Tests

- [ ] unit tests cover invalid combo row indices
- [ ] unit tests cover invalid eval ranges
- [ ] unit tests cover resident close/lr cache reuse
- [ ] unit tests cover resident close/lr cache invalidation
- [ ] parity tests still pass after staging and copy-elision changes
