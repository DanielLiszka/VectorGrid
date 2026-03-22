# KVDF GPU-native optimization plan addendum

This addendum supplements `kvdf_gpu_optimization_plan.md`.

It focuses on **what I would add or tighten** after a second pass through:

- `kvdf.rs`
- `kvdf_gpu_kernel.rs`
- `composite_validation_workflow.rs`
- adjacent implementation patterns in `gpu_backtest_kernel.rs`

The goal remains the same: **higher warm-path performance, less copy/allocation overhead, and stronger correctness guardrails without removing features or changing route semantics**.

---

## What I would add to the original plan

The main additions are:

1. **Persistent KVDF device scratch and allocation reuse** inside `KvdfCudaRuntime`.
2. **Stronger `validate_inputs(...)` checks** for combo bounds, time ranges, and stride correctness.
3. **Explicit market-residency and combo-residency keys** so copy elision stays correct.
4. **Named flag constants** instead of the current raw bit operations.
5. **Optional async + pinned transfer path** once the simpler reuse work is measured.
6. **Optional CUDA graph replay** for repeated same-shape KVDF launches.
7. **A shared-runtime alignment path** so KVDF can eventually reuse the same residency/caching ideas that already exist in `gpu_backtest_kernel.rs`.

The single most important item above is **scratch reuse**. The original plan emphasized close/log-return/device residency, but the second pass makes it clearer that KVDF currently also pays repeated **device allocation churn** for close, log returns, combo arrays, and output buffers on every evaluation call.

---

## 1. Highest-value addition: persistent KVDF scratch reuse

## Why this matters

`kvdf_gpu_kernel.rs` currently caches only:

- CUDA context
- module
- stream

but it **does not** cache the hot buffers used by `eval_kvdf_metrics_gpu_kernel_device_rows(...)`.

That means each call currently redoes all of the following:

- allocate/upload `d_close`
- allocate `d_lr`
- allocate/upload `d_combo_fast`
- allocate/upload `d_combo_slow`
- allocate/upload `d_combo_dti`
- allocate/upload `d_combo_start`
- allocate/upload `d_combo_end`
- allocate `d_out`
- allocate a fresh host metrics buffer on readback

Even before full copy elision, simply **reusing capacities and buffers** will reduce allocator churn and steady-state launch overhead.

## Recommended code edits

### `kvdf_gpu_kernel.rs`: add reusable scratch storage

```rust
struct KvdfKernelScratch {
    d_close: Option<DeviceBuffer<f32>>,
    d_lr: Option<DeviceBuffer<f32>>,
    d_combo_fast: Option<DeviceBuffer<i32>>,
    d_combo_slow: Option<DeviceBuffer<i32>>,
    d_combo_dti: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,

    // Reused host readback storage.
    host_metrics: Vec<f32>,

    // Optional correctness-preserving residency tags.
    last_close_key: Option<KvdfMarketResidencyKey>,
    last_combo_key: Option<KvdfComboResidencyKey>,
}

impl KvdfKernelScratch {
    fn new() -> Self {
        Self {
            d_close: None,
            d_lr: None,
            d_combo_fast: None,
            d_combo_slow: None,
            d_combo_dti: None,
            d_combo_start: None,
            d_combo_end: None,
            d_out: None,
            host_metrics: Vec::new(),
            last_close_key: None,
            last_combo_key: None,
        }
    }

    fn ensure_len_f32(buf: &mut Option<DeviceBuffer<f32>>, len: usize) -> Result<(), String> {
        let need_new = buf.as_ref().map_or(true, |b| b.len() < len);
        if need_new {
            *buf = Some(unsafe { DeviceBuffer::<f32>::uninitialized(len) }
                .map_err(|e| e.to_string())?);
        }
        Ok(())
    }

    fn ensure_len_i32(buf: &mut Option<DeviceBuffer<i32>>, len: usize) -> Result<(), String> {
        let need_new = buf.as_ref().map_or(true, |b| b.len() < len);
        if need_new {
            *buf = Some(unsafe { DeviceBuffer::<i32>::uninitialized(len) }
                .map_err(|e| e.to_string())?);
        }
        Ok(())
    }

    fn ensure_host_metrics_len(&mut self, len: usize) {
        if self.host_metrics.len() < len {
            self.host_metrics.resize(len, 0.0);
        }
    }
}
```

### Add runtime-owned scratch and cached device launch caps

```rust
struct KvdfCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    device_caps: DeviceLaunchCaps,
    scratch: KvdfKernelScratch,
}

impl KvdfCudaRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let context = Arc::new(Context::new(device).map_err(|e| e.to_string())?);
        let module = load_double_crossover_module(device)?;
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;
        let device_caps = detect_device_launch_caps(device);

        Ok(Self {
            context,
            module,
            stream,
            device_caps,
            scratch: KvdfKernelScratch::new(),
        })
    }
}
```

### Convert the eval path to reuse scratch buffers

```rust
let scratch = &mut rt.scratch;

KvdfKernelScratch::ensure_len_f32(&mut scratch.d_lr, signals.cols)?;
KvdfKernelScratch::ensure_len_i32(&mut scratch.d_combo_fast, combo_count)?;
KvdfKernelScratch::ensure_len_i32(&mut scratch.d_combo_slow, combo_count)?;
KvdfKernelScratch::ensure_len_i32(&mut scratch.d_combo_dti, combo_count)?;
KvdfKernelScratch::ensure_len_i32(&mut scratch.d_combo_start, combo_count)?;
KvdfKernelScratch::ensure_len_i32(&mut scratch.d_combo_end, combo_count)?;
KvdfKernelScratch::ensure_len_f32(&mut scratch.d_out, out_len)?;
scratch.ensure_host_metrics_len(out_len);

let d_lr = scratch.d_lr.as_mut().unwrap();
let d_combo_fast = scratch.d_combo_fast.as_mut().unwrap();
let d_combo_slow = scratch.d_combo_slow.as_mut().unwrap();
let d_combo_dti = scratch.d_combo_dti.as_mut().unwrap();
let d_combo_start = scratch.d_combo_start.as_mut().unwrap();
let d_combo_end = scratch.d_combo_end.as_mut().unwrap();
let d_out = scratch.d_out.as_mut().unwrap();
```

## Rollout order for this change

1. Reuse only `d_lr`, combo buffers, `d_out`, and host metrics.
2. Then add persistent `d_close` reuse.
3. Then add **true copy elision** using an explicit close-residency key or device-native close input.

That order keeps the first patch smaller and easier to validate.

---

## 2. Important correctness addition: residency keys for safe copy elision

## Why this matters

Buffer reuse and copy elision are **not** the same thing.

- **Reuse** means you keep the same allocation around and overwrite it with new data.
- **Copy elision** means you skip the overwrite because you know the device buffer already contains the right data.

Skipping a host-to-device close upload based only on `len` is unsafe. Two different candle windows can have the same length.

## Recommended code edits

### Add explicit residency keys

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KvdfMarketResidencyKey {
    pub series_id: u64,
    pub cols: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KvdfComboResidencyKey {
    pub batch_id: u64,
    pub combo_count: usize,
}
```

### Extend inputs to carry those keys when the caller can provide them

```rust
pub struct KvdfGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub close_key: Option<KvdfMarketResidencyKey>,
    pub cols: usize,
    // ... existing fields ...
}

pub struct KvdfGpuComboBatch<'a> {
    pub fast_row_index: &'a [i32],
    pub slow_row_index: &'a [i32],
    pub dti_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
    pub combo_key: Option<KvdfComboResidencyKey>,
}
```

### Only skip uploads if the key matches

```rust
let must_upload_close = scratch.last_close_key != signals.close_key;
if must_upload_close {
    KvdfKernelScratch::ensure_len_f32(&mut scratch.d_close, signals.cols)?;
    scratch
        .d_close
        .as_mut()
        .unwrap()
        .copy_from(signals.close_f32)
        .map_err(|e| e.to_string())?;
    scratch.last_close_key = signals.close_key;
}

let must_upload_combo = scratch.last_combo_key != combos.combo_key;
if must_upload_combo {
    d_combo_fast.copy_from(combos.fast_row_index).map_err(|e| e.to_string())?;
    d_combo_slow.copy_from(combos.slow_row_index).map_err(|e| e.to_string())?;
    d_combo_dti.copy_from(combos.dti_row_index).map_err(|e| e.to_string())?;
    d_combo_start.copy_from(combos.start_t).map_err(|e| e.to_string())?;
    d_combo_end.copy_from(combos.end_t).map_err(|e| e.to_string())?;
    scratch.last_combo_key = combos.combo_key;
}
```

## Recommendation

Do **not** skip uploads without an explicit caller-provided key or a device-native input path.

That is a performance feature, but it is also a correctness boundary.

---

## 3. Important correctness addition: strengthen `validate_inputs(...)`

## Why this matters

The current validation surface is too thin for a GPU wrapper that receives raw device pointers and row indices.

The function should reject bad inputs **before** launching CUDA when any of the following occur:

- row index out of bounds
- `start_t > end_t`
- `end_t > cols`
- stride smaller than `cols`
- `usize -> i32` narrowing overflow

These checks are cheap and materially improve correctness and diagnosability.

## Recommended code edits

### `kvdf_gpu_kernel.rs`: extend validation

```rust
fn to_i32_checked(name: &str, v: usize) -> Result<i32, String> {
    i32::try_from(v).map_err(|_| format!("KVDF GPU {name} exceeds i32 range: {v}"))
}

fn validate_inputs(
    signals: &KvdfGpuDeviceSignalMatrices<'_>,
    combos: &KvdfGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("KVDF GPU kernel requires non-empty series".to_string());
    }

    let combo_count = combos.start_t.len();
    if combos.fast_row_index.len() != combo_count
        || combos.slow_row_index.len() != combo_count
        || combos.dti_row_index.len() != combo_count
        || combos.end_t.len() != combo_count
    {
        return Err("KVDF GPU combo arrays must have equal lengths".to_string());
    }

    if signals.slow_stride < n || signals.fast_stride < n || signals.dti_stride < n {
        return Err("KVDF GPU row-major stride must be >= cols".to_string());
    }

    if signals.slow_rows == 0 || signals.fast_rows == 0 || signals.dti_rows == 0 {
        return Err("KVDF GPU matrix row counts must be > 0".to_string());
    }

    for (i, &row) in combos.fast_row_index.iter().enumerate() {
        if row < 0 || (row as usize) >= signals.fast_rows {
            return Err(format!("KVDF fast_row_index[{i}] out of range: {row}"));
        }
    }
    for (i, &row) in combos.slow_row_index.iter().enumerate() {
        if row < 0 || (row as usize) >= signals.slow_rows {
            return Err(format!("KVDF slow_row_index[{i}] out of range: {row}"));
        }
    }
    for (i, &row) in combos.dti_row_index.iter().enumerate() {
        if row < 0 || (row as usize) >= signals.dti_rows {
            return Err(format!("KVDF dti_row_index[{i}] out of range: {row}"));
        }
    }

    for i in 0..combo_count {
        let start_t = combos.start_t[i];
        let end_t = combos.end_t[i];
        if start_t < 0 || end_t < 0 {
            return Err(format!("KVDF combo[{i}] has negative time range"));
        }
        let start_u = start_t as usize;
        let end_u = end_t as usize;
        if start_u > end_u {
            return Err(format!("KVDF combo[{i}] has start_t > end_t"));
        }
        if end_u > n {
            return Err(format!("KVDF combo[{i}] end_t exceeds cols"));
        }
    }

    // Also guard every narrowing conversion up front.
    let _ = to_i32_checked("cols", signals.cols)?;
    let _ = to_i32_checked("slow_rows", signals.slow_rows)?;
    let _ = to_i32_checked("fast_rows", signals.fast_rows)?;
    let _ = to_i32_checked("dti_rows", signals.dti_rows)?;
    let _ = to_i32_checked("slow_stride", signals.slow_stride)?;
    let _ = to_i32_checked("fast_stride", signals.fast_stride)?;
    let _ = to_i32_checked("dti_stride", signals.dti_stride)?;
    let _ = to_i32_checked("combo_count", combo_count)?;

    Ok(())
}
```

## Why I would do this early

This is low-risk and protects all later performance work.

---

## 4. Missed cleanup: replace raw flag bits with named constants

## Why this matters

The current wrapper builds flags like this:

```rust
if strategy.long_only {
    flags |= 1u32 << 0;
}
if !strategy.allow_flip {
    flags |= 1u32 << 1;
}
if strategy.trade_on_next_bar {
    flags |= 1u32 << 2;
}
flags |= 1u32 << 4;
```

The unnamed `1u32 << 4` is especially fragile.

This is not just style. It is a **CPU/GPU parity risk**. Any drift in bit meaning will silently change behavior.

## Recommended code edits

```rust
const FLAG_LONG_ONLY: u32 = 1 << 0;
const FLAG_DISABLE_FLIP: u32 = 1 << 1;
const FLAG_TRADE_ON_NEXT_BAR: u32 = 1 << 2;
const FLAG_KVDF_ENABLE_STRATEGY_LOGIC: u32 = 1 << 4; // rename to the true semantic meaning

fn kvdf_kernel_flags(strategy: &StrategyConfig) -> u32 {
    let mut flags = FLAG_KVDF_ENABLE_STRATEGY_LOGIC;
    if strategy.long_only {
        flags |= FLAG_LONG_ONLY;
    }
    if !strategy.allow_flip {
        flags |= FLAG_DISABLE_FLIP;
    }
    if strategy.trade_on_next_bar {
        flags |= FLAG_TRADE_ON_NEXT_BAR;
    }
    flags
}
```

## Follow-up

Add one parity test that checks the CPU reference and GPU route for each relevant flag combination:

- default
- `long_only = true`
- `allow_flip = false`
- `trade_on_next_bar = true`
- combinations of the above

---

## 5. Better uninitialized-memory adoption: split safe cases from unsafe cases

The original plan already called out zero-fill removal, but I would make the safety rules more explicit.

## Safe now

### Device buffers whose entire contents are kernel-written before any read

These are already good candidates:

- `d_lr`
- `d_out`

as long as the launch always covers the full logical output.

### Host readback buffer for `d_out`

This is safe **only after** you verify that every output cell is written by the kernel.

## Recommended code edits

### Replace zero-filled readback with an uninitialized helper

If you already have a shared helper in `alloc_policy.rs`, prefer that.

If you do not, the local pattern should still be hidden behind one helper rather than duplicated inline.

```rust
fn alloc_uninit_copy_f32(len: usize) -> Vec<f32> {
    let mut v: Vec<std::mem::MaybeUninit<f32>> = Vec::with_capacity(len);
    unsafe {
        v.set_len(len);
        std::mem::transmute::<Vec<std::mem::MaybeUninit<f32>>, Vec<f32>>(v)
    }
}
```

Then:

```rust
let host_metrics = alloc_uninit_copy_f32(out_len);
d_out.copy_to(host_metrics.as_mut_slice()).map_err(|e| e.to_string())?;
```

## Not safe to change blindly

Do **not** remove fills from any buffer whose initial contents are behaviorally observable before overwrite.

Examples:

- warmup-prefilled series using `NaN`
- host metrics buffers that may only be partially overwritten
- any host-side matrix builder that copies only a suffix or tile subset

That distinction should stay explicit in the document and in the implementation checklist.

---

## 6. Optional but strong improvement: support device-native close and precomputed device log returns

This is still one of the best copy-elision targets.

## Recommended code edits

### Replace host-only close input with a more flexible input enum

```rust
pub enum KvdfCloseInput<'a> {
    Host(&'a [f32]),
    Device { ptr: u64, len: usize },
}

pub enum KvdfLogReturnInput {
    ComputeOnDevice,
    Device { ptr: u64, len: usize },
}

pub struct KvdfGpuDeviceSignalMatrices<'a> {
    pub close: KvdfCloseInput<'a>,
    pub lr: KvdfLogReturnInput,
    pub cols: usize,
    // existing matrix pointers...
}
```

### Match on the close/lr source in the eval path

```rust
let close_ptr = match signals.close {
    KvdfCloseInput::Host(src) => {
        KvdfKernelScratch::ensure_len_f32(&mut scratch.d_close, src.len())?;
        let d_close = scratch.d_close.as_mut().unwrap();
        d_close.copy_from(src).map_err(|e| e.to_string())?;
        d_close.as_device_ptr().as_raw()
    }
    KvdfCloseInput::Device { ptr, len } => {
        if len != signals.cols {
            return Err("KVDF device close len mismatch".to_string());
        }
        ptr
    }
};

let lr_ptr = match signals.lr {
    KvdfLogReturnInput::ComputeOnDevice => {
        KvdfKernelScratch::ensure_len_f32(&mut scratch.d_lr, signals.cols)?;
        let d_lr = scratch.d_lr.as_mut().unwrap();
        launch_compute_log_returns_f32_raw(rt, close_ptr, d_lr, signals.cols)?;
        d_lr.as_device_ptr().as_raw()
    }
    KvdfLogReturnInput::Device { ptr, len } => {
        if len != signals.cols {
            return Err("KVDF device lr len mismatch".to_string());
        }
        ptr
    }
};
```

## Recommended rollout

1. Add the enum surface.
2. Keep host close as the default path.
3. Start using device close only from callers that already own resident series.
4. Use precomputed device log returns only after parity is established.

---

## 7. Optional warm-path improvement: async and pinned transfers

This goes beyond the strict strategy-local prompt, but it is a sensible next layer once scratch reuse lands.

## When it is worth it

Use this only if profiling still shows transfer latency after:

- scratch reuse
- host->device copy elision for close/combos
- no extra HostF32 -> `Vec<f64>` conversions on the GPU prep path

## Candidate changes

### Async device allocations and copies on the KVDF stream

```rust
let d_out = if kvdf_async_alloc_enabled() {
    unsafe { DeviceBuffer::<f32>::uninitialized_async(out_len, &rt.stream) }
} else {
    unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }
}.map_err(|e| e.to_string())?;
```

### Pinned host readback buffer

Depending on what `cust` and your local helpers expose, use a pinned/locked host buffer for `host_metrics` so the device-to-host copy avoids pageable memory.

## Important caution

Pinned buffers can improve transfer speed, but they also:

- complicate ownership/lifetimes
- increase pressure on host memory
- are easy to overuse

So I would treat this as **optional** rather than default.

---

## 8. Optional warm-path improvement: CUDA graph replay for stable KVDF shapes

This is a good beyond-scope idea because the adjacent `gpu_backtest_kernel.rs` already uses graph replay concepts for repeated launches.

## When it makes sense

Only consider this when all of the following are true:

- combo count is stable across repeated runs
- row counts/strides are stable
- the close/lr path is already resident or scratch-backed
- you have parity tests in place

## Why it can help

Graph replay can reduce launch overhead for repeated warm-path sequences such as:

1. compute log returns
2. launch `kvdf_backtest_metrics_f32`
3. copy out metrics

## Recommendation

Do **not** start here.

This should be a third-wave optimization after:

- scratch reuse
- device-native close/lr support
- stronger validation and parity coverage

---

## 9. Optional shared-runtime alignment: KVDF should eventually hook into the same residency model as `gpu_backtest_kernel.rs`

This is beyond the strict strategy-local prompt, but it is the cleanest long-term direction.

The adjacent shared GPU runtime already has patterns for:

- resident market series
- buffer reuse
- optional async allocation
- graph replay
- uninitialized host readback helpers

Rather than building a permanently bespoke KVDF cache, the cleaner direction is:

1. land a local KVDF scratch implementation first
2. measure it
3. if it works well, factor common pieces into shared helpers

That avoids a premature broad refactor, but still gives you a migration path toward one coherent CUDA residency model.

---

## 10. Small but useful addition in `kvdf.rs`: go beyond fixed arrays and remove per-call owned strings where practical

The original plan already suggested replacing outer `Vec<IndicatorParamKV>` containers with fixed arrays.

I would add one more note:

- if `IndicatorParamKV` forces owned `String` keys and enum strings, fixed arrays only remove the **container** allocation, not the inner string churn.

## Optional beyond-scope change

If adjacent dispatch code can tolerate it, add a borrowed parameter representation such as:

```rust
pub struct IndicatorParamRef<'a> {
    pub key: &'a str,
    pub value: IndicatorParamValueRef<'a>,
}
```

or use `Cow<'static, str>` for keys/output identifiers.

This is broader than KVDF alone, so it belongs in the optional bucket, but it is the correct long-term answer if tiny dynamic-indicator allocations become a repeated theme across GPU-native strategies.

---

## 11. Optional CPU-path cleanup in `kvdf.rs`

This is lower priority than CUDA-path work, but it is still worth noting.

`build_kvdf_desired_positions(...)` repeatedly does:

- `.get(sig_t)`
- `.copied()`
- `.unwrap_or(f64::NAN)`

inside the hot loop.

That is safe and readable, but it adds repeated bounds checks and fallback branches.

## Optional cleanup

After one up-front equal-length validation, use direct indexing in the hot region.

```rust
let close_now = close[sig_t];
let keltner_upper = signals.keltner_upper[sig_t];
let keltner_middle = signals.keltner_middle[sig_t];
// ...
```

I would only do this after higher-value CUDA work, and only if the CPU fallback still matters materially in benchmarks.

---

## 12. Extra benchmark and test work I would add

## A. KVDF GPU validation benchmark parity with ASR benchmark shape

Add:

- `composite_validate/gpu_kernel/kvdf/100k/64candidates`
- optionally `composite_validate/gpu_kernel/kvdf/100k/192candidates`

The current coverage includes KVDF at `50k/8`, but the larger dedicated GPU validation case is still ASR-only.

## B. Route assertion in GPU validation benches

After the route probe, assert the backend is actually GPU.

```rust
assert!(
    matches!(route_probe.backend_used, ta_app_core::BackendUsed::Gpu),
    "expected KVDF GPU validation benchmark to stay on GPU, got {:?} (fallback={})",
    route_probe.backend_used,
    route_probe.backend_fallback_reason.as_deref().unwrap_or("<none>")
);
```

## C. Full-write test for `d_out`

Before switching host readback to uninitialized allocation, add a test that proves the KVDF kernel writes every metrics slot.

A simple strategy:

1. fill device output with a sentinel
2. launch kernel
3. copy back
4. assert no sentinel remains in any logical output slot

## D. Stronger blocked-route tests

Add tests that verify the GPU path rejects:

- execution overlays
- stop-loss
- take-profit

and that the fallback/error text stays stable enough to diagnose route blockers.

## E. Scratch-reuse correctness test

Add a test that launches the same-shape KVDF batch repeatedly, with:

- same close key / same combo key
- different close key / same shape
- same close key / different combo key

and verifies that reuse does not corrupt results.

---

## Revised implementation order

## Phase 1: safest and highest ROI

- [ ] Strengthen `validate_inputs(...)`
- [ ] Replace raw flag bits with named constants
- [ ] Cache `device_caps` in `KvdfCudaRuntime`
- [ ] Reuse `d_lr`, combo buffers, `d_out`, and host metrics scratch
- [ ] Add GPU validation route assertions
- [ ] Add KVDF 100k/64 GPU validation benchmark case

## Phase 2: copy-elision and prep cleanup

- [ ] Add residency keys for safe upload skipping
- [ ] Add device-native close input
- [ ] Add optional device-native log-return input
- [ ] Add KVDF-specific GPU prep trace summary
- [ ] Keep GPU-bound signal prep in `f32` as long as possible

## Phase 3: optional warm-path work

- [ ] Async allocations/copies where the API supports them cleanly
- [ ] Pinned host readback if DtoH remains measurable
- [ ] CUDA graph replay for stable KVDF shapes
- [ ] Investigate shared-runtime integration with the broader residency model

## Phase 4: broader optional cleanup

- [ ] Borrowed parameter surface in dynamic indicator dispatch
- [ ] CPU desired-position loop micro-optimization if still measurable
- [ ] Multi-output dynamic indicator family APIs if adjacent files confirm recomputation waste

---

## Additional checklist to append to the original plan

### KVDF runtime and scratch

- [ ] Add `KvdfKernelScratch` to `KvdfCudaRuntime`
- [ ] Reuse `d_lr`
- [ ] Reuse `d_combo_fast`
- [ ] Reuse `d_combo_slow`
- [ ] Reuse `d_combo_dti`
- [ ] Reuse `d_combo_start`
- [ ] Reuse `d_combo_end`
- [ ] Reuse `d_out`
- [ ] Reuse host metrics readback storage
- [ ] Cache `device_caps` in runtime
- [ ] Measure warm-path before/after allocation reuse

### Copy-elision correctness

- [ ] Introduce `KvdfMarketResidencyKey`
- [ ] Introduce `KvdfComboResidencyKey`
- [ ] Skip close uploads only when the market key matches
- [ ] Skip combo uploads only when the combo key matches
- [ ] Never infer data equality from shape alone
- [ ] Add tests for same-key reuse vs changed-key refresh

### Input validation

- [ ] Verify combo array lengths match
- [ ] Verify row indices are non-negative
- [ ] Verify fast indices are `< fast_rows`
- [ ] Verify slow indices are `< slow_rows`
- [ ] Verify dti indices are `< dti_rows`
- [ ] Verify `start_t <= end_t`
- [ ] Verify `end_t <= cols`
- [ ] Verify all strides are `>= cols`
- [ ] Replace unchecked `as i32` casts with checked conversions

### Flag / kernel-contract safety

- [ ] Replace raw flag shifts with named constants
- [ ] Give bit 4 a semantic name
- [ ] Add CPU/GPU parity tests for each relevant flag combination
- [ ] Document the host/kernel flag contract near the wrapper

### Uninitialized memory and readback

- [ ] Keep `d_lr` and `d_out` uninitialized
- [ ] Verify the kernel writes every output slot before removing host zero-fill
- [ ] Route host readback through a shared uninitialized-copy helper if available
- [ ] Do not remove fills from prefix-observable or partially written buffers

### KVDF prep-path cleanup

- [ ] Keep GPU-bound signal prep in `f32`
- [ ] Avoid `HostF32 -> Vec<f64>` unless CPU/reference code truly needs it
- [ ] Add per-indicator prep trace / fallback summary
- [ ] Replace outer parameter `Vec`s with fixed arrays/slices
- [ ] Cache output-id resolution with `OnceLock`
- [ ] Consider borrowed param keys/values only after the local wins land

### Benchmark and validation coverage

- [ ] Assert GPU route usage in KVDF GPU validation benchmarks
- [ ] Add `kvdf/100k/64candidates` GPU validation group
- [ ] Optionally add `kvdf/100k/192candidates` GPU validation group
- [ ] Add a kernel full-write test for metrics output
- [ ] Add blocked-route tests for overlays/stop-loss/take-profit
- [ ] Add scratch-reuse correctness regression test

### Optional warm-path work

- [ ] Evaluate async alloc/copy path only after scratch reuse is measured
- [ ] Evaluate pinned host readback only if DtoH remains material
- [ ] Evaluate CUDA graph replay only for stable KVDF shapes
- [ ] Consider shared residency/runtime integration only after local KVDF fixes prove out

---

## Bottom line

If I were tightening the implementation plan, I would elevate the following three items to near-top priority:

1. **persistent scratch-buffer reuse in `kvdf_gpu_kernel.rs`**
2. **stronger pre-launch validation for indices/ranges/strides**
3. **explicit residency keys so copy elision stays correct**

Those three changes improve performance and correctness together, and they do it without removing features or requiring a broad redesign.

---

## 13. Further corrections after another source pass

These are the items I would explicitly add to the addendum above.

## A. Real CPU/GPU correctness mismatch: `dti_threshold` semantics differ today

This is the biggest correctness issue I did not call out strongly enough in the earlier plan.

### Current mismatch

In `kvdf.rs`, CPU desired-position generation uses the threshold directly:

```rust
let dti_up = dti_now.is_finite() && dti_now >= cfg.dti_threshold;
let dti_down = dti_now.is_finite() && dti_now <= -cfg.dti_threshold;
```

In `kvdf_gpu_kernel.rs`, the GPU wrapper normalizes it with `abs()`:

```rust
let dti_threshold = kvdf.dti_threshold.abs() as f32;
```

So negative user inputs can produce **different CPU and GPU behavior**.

### Recommended fix

Normalize once through a shared helper and make validation reject non-positive values.

```rust
pub fn validate_kvdf_params(cfg: &KvdfStrategyParams) -> Result<(), String> {
    // ... existing checks ...
    if !cfg.dti_threshold.is_finite() || cfg.dti_threshold <= 0.0 {
        return Err("KVDF dti_threshold must be > 0 and finite.".to_string());
    }
    Ok(())
}

#[inline]
pub fn normalized_kvdf_dti_threshold(cfg: &KvdfStrategyParams) -> f64 {
    cfg.dti_threshold.abs()
}
```

Then use the same helper in both CPU and GPU code:

```rust
let dti_threshold = normalized_kvdf_dti_threshold(cfg);
```

and

```rust
let dti_threshold = normalized_kvdf_dti_threshold(kvdf) as f32;
```

### Tests to add

```rust
#[test]
fn validate_kvdf_params_rejects_non_positive_dti_threshold() {
    let mut cfg = KvdfStrategyParams::default();
    cfg.dti_threshold = 0.0;
    assert!(validate_kvdf_params(&cfg).is_err());

    let mut cfg = KvdfStrategyParams::default();
    cfg.dti_threshold = -20.0;
    assert!(validate_kvdf_params(&cfg).is_err());
}
```

Also add one CPU-vs-GPU parity fixture using a non-default positive threshold, nonzero commission, and a non-default flag combination.

## B. Strengthen the existing KVDF GPU smoke test

The current test contains a weak assertion equivalent to “at least one element is finite or NaN,” which is not a useful guarantee.

Replace it with stronger checks:

```rust
assert!(signals.fvg_upper_ts.iter().any(|v| v.is_finite()));
assert!(signals.fvg_lower_ts.iter().any(|v| v.is_finite()));
assert!(signals
    .fvg_upper_ts
    .iter()
    .all(|v| v.is_finite() || v.is_nan()));
assert!(signals
    .fvg_lower_ts
    .iter()
    .all(|v| v.is_finite() || v.is_nan()));
```

If the prep-trace surface from the main plan is added, also assert that supported indicator families actually used CUDA in this test rather than silently falling back to CPU.

## C. Add a strict signal-length validation helper before direct indexing on CPU fallback

The earlier plan already suggested replacing repeated `get(...).unwrap_or(NaN)` lookups with direct indexing. I would make that explicit and tie it to a shared validator.

```rust
fn validate_kvdf_signal_lengths(close_len: usize, signals: &KvdfSignals) -> Result<(), String> {
    let lens = [
        signals.keltner_upper.len(),
        signals.keltner_middle.len(),
        signals.keltner_lower.len(),
        signals.vwap.len(),
        signals.dti.len(),
        signals.devstop_long.len(),
        signals.devstop_short.len(),
        signals.fvg_upper_ts.len(),
        signals.fvg_lower_ts.len(),
    ];

    if lens.iter().copied().all(|len| len == close_len) {
        Ok(())
    } else {
        Err("KVDF signal length mismatch".to_string())
    }
}
```

Then use direct indexing only after that invariant is checked.

## D. Strengthen GPU benchmark route checks beyond `backend_used == Gpu`

The optimizer benches already assert GPU usage. I would also assert that the fallback reason is absent:

```rust
assert!(
    probe.backend_fallback_reason.is_none(),
    "expected no fallback reason for GPU benchmark route, got {:?}",
    probe.backend_fallback_reason
);
```

Do the same in measured iterations where practical.

If your route strings are stable in the local repo, add a route-name assertion as well.

## E. Add explicit flag-coverage parity cases

The wrapper currently marshals:

- `long_only`
- `allow_flip`
- `trade_on_next_bar`
- `commission`
- `dti_threshold`

Those deserve explicit parity coverage, not just default-config coverage.

The minimum strong fixture is:

- one fixed candle set,
- one KVDF candidate,
- `long_only = true`,
- `allow_flip = false`,
- `trade_on_next_bar = true`,
- nonzero commission,
- non-default positive `dti_threshold`,
- CPU reference metrics vs GPU metrics with tolerances.

---

## 14. Items I would now elevate in priority

After the extra pass, my top five priorities would be:

1. **Fix `dti_threshold` CPU/GPU consistency immediately.**
2. **Strengthen `validate_inputs(...)` for row/time/stride correctness.**
3. **Add close/log-return residency and scratch reuse.**
4. **Strengthen the GPU smoke test and add flag-coverage parity fixtures.**
5. **Add no-fallback assertions to GPU benchmark paths.**

That ordering keeps correctness ahead of performance, while still targeting the highest-value warm-path wins.

