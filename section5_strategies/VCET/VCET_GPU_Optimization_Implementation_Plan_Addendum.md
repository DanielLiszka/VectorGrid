# VCET GPU optimization plan addendum

This addendum supplements `VCET_GPU_Optimization_Implementation_Plan.md`.

It focuses on things that I would tighten, make more explicit, or add as optional extensions after a second pass through the provided VCET files:

- `vcet.rs`
- `vcet_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

It stays aligned with the repo review posture in `gpt54_feature_review_map.md`: optimization-first, benchmark-first, parity-preserving, and copy-elision-aware.

---

## 1. What I would add or tighten from the first plan

The first plan was directionally correct: the largest immediate wins are still staged-copy reduction, device-resident reuse, duplicate-work removal, and benchmark coverage.

After re-checking the files, the main additions I would make are:

1. **Strengthen GPU input validation much more aggressively before launch.**
2. **Encode device-memory lifetime safety into the VCET batch types instead of passing only raw `u64` row pointers.**
3. **Add a reusable VCET runtime scratch layer, not just a one-off uploaded batch API.**
4. **Split VCET benchmarking into cold upload, warm eval, and end-to-end surfaces.**
5. **Consider a bigger optional ABI improvement: eliminate row-pointer table uploads entirely by using contiguous matrices plus row-stride metadata.**
6. **Be more explicit about which allocations must remain initialized and which are safe for uninitialized allocation / reuse.**

---

## 2. Additional confirmed or near-confirmed issues I would add

### A. `validate_inputs` is too weak for correctness and defensive performance

In `vcet_gpu_kernel.rs`, `validate_inputs(...)` checks series non-emptiness, length matching, and combo-vector length matching, but it does **not** currently validate:

- `signals.cols <= i32::MAX`
- `signals.vidya_rows <= i32::MAX`
- `signals.ce_rows <= i32::MAX`
- `combo_count <= i32::MAX`
- `vidya_row_index[i] >= 0 && < vidya_rows`
- `ce_row_index[i] >= 0 && < ce_rows`
- `start_t[i] >= 0`
- `end_t[i] >= 0`
- `start_t[i] < end_t[i]`
- `end_t[i] <= cols`
- row-pointer entries are nonzero

That matters because the hot path later casts several `usize` values to `i32` and uploads `start_t` / `end_t` / row-index vectors directly into the kernel launch surface. Right now those casts and values rely on upstream discipline rather than local enforcement.

### Recommended code edit

Replace the current validator with a strict validator that checks both **shape** and **value ranges** before any upload or launch:

```rust
fn validate_inputs(
    signals: &VcetGpuDeviceSignalMatrices<'_>,
    combos: &VcetGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("VCET GPU kernel requires non-empty series".to_string());
    }
    if n > i32::MAX as usize {
        return Err("VCET GPU cols exceed i32 kernel limits".to_string());
    }
    if signals.close_f32.len() != n {
        return Err("VCET GPU close length mismatch".to_string());
    }

    if signals.vidya_rows == 0 || signals.ce_rows == 0 {
        return Err("VCET GPU requires at least one VIDYA row and one CE row".to_string());
    }
    if signals.vidya_rows > i32::MAX as usize || signals.ce_rows > i32::MAX as usize {
        return Err("VCET GPU row counts exceed i32 kernel limits".to_string());
    }
    if signals.vidya_rows_ptr.len() != signals.vidya_rows
        || signals.ce_long_rows_ptr.len() != signals.ce_rows
        || signals.ce_short_rows_ptr.len() != signals.ce_rows
    {
        return Err("VCET GPU row-pointer vector length mismatch".to_string());
    }
    if signals.vidya_rows_ptr.iter().any(|&p| p == 0)
        || signals.ce_long_rows_ptr.iter().any(|&p| p == 0)
        || signals.ce_short_rows_ptr.iter().any(|&p| p == 0)
    {
        return Err("VCET GPU row-pointer table contains a null device pointer".to_string());
    }

    let combos_len = combos.vidya_row_index.len();
    if combos.ce_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("VCET GPU combo vector length mismatch".to_string());
    }
    if combos_len > i32::MAX as usize {
        return Err("VCET GPU combo count exceeds i32 kernel limits".to_string());
    }

    for i in 0..combos_len {
        let v = combos.vidya_row_index[i];
        let c = combos.ce_row_index[i];
        let s = combos.start_t[i];
        let e = combos.end_t[i];

        if v < 0 || v as usize >= signals.vidya_rows {
            return Err(format!(
                "VCET GPU vidya_row_index out of bounds at combo {}: {} not in [0,{})",
                i, v, signals.vidya_rows
            ));
        }
        if c < 0 || c as usize >= signals.ce_rows {
            return Err(format!(
                "VCET GPU ce_row_index out of bounds at combo {}: {} not in [0,{})",
                i, c, signals.ce_rows
            ));
        }
        if s < 0 || e < 0 {
            return Err(format!(
                "VCET GPU start_t/end_t must be non-negative at combo {}: start={} end={}",
                i, s, e
            ));
        }
        if s >= e {
            return Err(format!(
                "VCET GPU start_t must be < end_t at combo {}: start={} end={}",
                i, s, e
            ));
        }
        if e as usize > n {
            return Err(format!(
                "VCET GPU end_t out of bounds at combo {}: end={} cols={}",
                i, e, n
            ));
        }
    }

    Ok(())
}
```

### Why this belongs in the performance plan

This is first a correctness fix, but it is also a performance guard. Invalid combos are much cheaper to reject on CPU than to upload and crash, diverge, or corrupt a CUDA launch.

---

### B. `VcetGpuDeviceSignalMatrices` does not encode device-owner lifetime safety

The current `VcetGpuDeviceSignalMatrices` carries only:

- host `close_f32`
- `cols`
- three raw pointer tables: `vidya_rows_ptr`, `ce_long_rows_ptr`, `ce_short_rows_ptr`
- row counts

It does **not** own or borrow the underlying `VcetOwnedDeviceMatrix` objects that actually keep those row pointers alive.

That makes the current type easy to misuse: a caller can build pointer tables from device matrices, drop the matrices, and still hold a `VcetGpuDeviceSignalMatrices` value that looks valid on the Rust side.

### Recommended code edit

Add an owning uploaded signal batch for the hot path, and keep the current borrowed struct only as a lower-level escape hatch:

```rust
#[cfg(feature = "cuda")]
pub struct UploadedVcetSignalBatch {
    device_id: u32,
    cols: usize,

    // Device-resident shared market data.
    d_close: DeviceBuffer<f32>,
    d_lr: DeviceBuffer<f32>,

    // Device-resident pointer tables for the current ABI.
    d_vidya_rows: DeviceBuffer<u64>,
    d_ce_long_rows: DeviceBuffer<u64>,
    d_ce_short_rows: DeviceBuffer<u64>,

    // Keep owners alive so row pointers stay valid.
    vidya_owners: Vec<VcetOwnedDeviceMatrix>,
    ce_long_owners: Vec<VcetOwnedDeviceMatrix>,
    ce_short_owners: Vec<VcetOwnedDeviceMatrix>,

    vidya_rows: usize,
    ce_rows: usize,
}
```

Then expose:

```rust
pub fn upload_vcet_signal_batch(
    device_id: u32,
    close_f32: &[f32],
    vidya_rows: Vec<VcetOwnedDeviceMatrix>,
    ce_long_rows: Vec<VcetOwnedDeviceMatrix>,
    ce_short_rows: Vec<VcetOwnedDeviceMatrix>,
) -> Result<UploadedVcetSignalBatch, String>
```

This prevents dangling-pointer misuse and gives you a natural place to cache `d_close`, `d_lr`, and the row-pointer tables.

---

### C. The first plan should have been more explicit about a reusable runtime scratch layer

The first plan called for an uploaded VCET batch, which is still correct.

What I would add now is: **also build a small VCET-specific scratch/cache layer inside `VcetCudaRuntime`**, because the adjacent shared GPU path already uses this model effectively.

The current `VcetCudaRuntime` stores only:

- `context`
- `module`
- `stream`

By comparison, the adjacent `gpu_backtest_kernel.rs` runtime holds reusable scratch buffers, launch caps, and resident-market data. VCET does not need all of that complexity, but it **does** need a minimal version of it.

### Recommended code edit

```rust
struct VcetKernelScratch {
    d_out: Option<DeviceBuffer<f32>>,
    d_combo_vidya: Option<DeviceBuffer<i32>>,
    d_combo_ce: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,

    // Optional reusable host readback buffer.
    host_metrics: Vec<f32>,
}

impl VcetKernelScratch {
    fn ensure_len_f32(
        buf: &mut Option<DeviceBuffer<f32>>,
        len: usize,
        stream: &Stream,
    ) -> Result<(), String> {
        if buf.as_ref().map(|b| b.len()) != Some(len) {
            if let Some(old) = buf.take() {
                old.drop_async(stream).map_err(|e| e.to_string())?;
            }
            let next = unsafe { DeviceBuffer::<f32>::uninitialized_async(len, stream) }
                .map_err(|e| e.to_string())?;
            *buf = Some(next);
        }
        Ok(())
    }

    fn ensure_len_i32(
        buf: &mut Option<DeviceBuffer<i32>>,
        len: usize,
        stream: &Stream,
    ) -> Result<(), String> {
        if buf.as_ref().map(|b| b.len()) != Some(len) {
            if let Some(old) = buf.take() {
                old.drop_async(stream).map_err(|e| e.to_string())?;
            }
            let next = unsafe { DeviceBuffer::<i32>::uninitialized_async(len, stream) }
                .map_err(|e| e.to_string())?;
            *buf = Some(next);
        }
        Ok(())
    }

    fn ensure_host_metrics_len(&mut self, len: usize) {
        if self.host_metrics.len() != len {
            self.host_metrics = alloc_uninit_copy(len);
        }
    }
}

struct VcetCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    scratch: VcetKernelScratch,
}
```

### Why this matters

Without scratch reuse, even after you add an uploaded signal batch, you still keep paying per-call allocation churn for:

- `d_out`
- combo arrays
- host readback buffer

This is exactly the kind of avoidable warm-path overhead the broader runtime bundle warns about.

---

### D. The first plan should have separated VCET benchmark surfaces more explicitly

The provided benchmark files already print route metadata for other strategy cases, and the review map stresses timing-boundary discipline.

For VCET, I would now recommend **three** benchmark surfaces, not one:

1. **End-to-end request benchmark**  
   Measures the real application surface with request construction, route selection, indicator prep, uploads, execution, and result shaping.

2. **Cold upload benchmark**  
   Uploads a fresh VCET batch each iteration, then launches once.

3. **Warm eval benchmark**  
   Uploads once outside the timed loop, then repeatedly launches on the already-uploaded batch.

### Why this matters

Without a dedicated warm-eval benchmark, you cannot tell whether VCET is slow because of:

- GPU scoring,
- setup-path indicator staging,
- H2D uploads,
- or readback/allocation churn.

### Recommended Criterion structure

```rust
fn bench_vcet_gpu_warm_eval_only(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_synth_candles(100_000);
    let strategy = StrategyConfig::default();
    let vcet = ta_app_core::VcetStrategyParams::default();

    let uploaded = prepare_uploaded_vcet_batch(&candles, 0)
        .expect("upload vcet batch");
    let combos = make_vcet_combo_batch(64);

    // Prime JIT / runtime / kernels before timing.
    let _ = eval_uploaded_vcet_metrics_gpu_kernel(
        &uploaded,
        &combos,
        &strategy,
        &vcet,
    )
    .expect("warmup eval");

    let mut group = c.benchmark_group("vcet/gpu/warm_eval_only/100k/64candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    group.bench_function("warm_eval", |b| {
        b.iter(|| {
            let res = eval_uploaded_vcet_metrics_gpu_kernel(
                &uploaded,
                &combos,
                &strategy,
                &vcet,
            )
            .expect("warm eval");
            criterion::black_box(res);
        })
    });

    group.finish();
}
```

### Note

If the broader composite request types in your repo do not yet expose a `vcet` field in the bench helpers, that wiring remains a prerequisite outside these four files.

---

## 3. Optional but high-value changes that go beyond the original prompt scope

These are **not** the first things I would implement, but they are good optional follow-ons if the in-scope fixes are done and VCET remains important.

### Optional extension 1 — eliminate row-pointer table uploads entirely

This is the biggest structural improvement I would consider beyond the original prompt scope.

Right now the VCET GPU ABI uses three host-side arrays of raw device row pointers:

- `vidya_rows_ptr: &[u64]`
- `ce_long_rows_ptr: &[u64]`
- `ce_short_rows_ptr: &[u64]`

Those arrays are uploaded every evaluation call in `eval_vcet_metrics_gpu_kernel_device_rows(...)`.

### Better design

If the upstream indicator rows are stored contiguously and uniformly shaped, change the kernel ABI from:

- pointer table + row count

into:

- base device pointer
- row stride in elements or bytes
- row count
- row index arrays in combos

### Example redesign

```rust
pub struct VcetGpuContiguousSignalMatrices {
    pub close_f32: &'a [f32],
    pub cols: usize,

    pub vidya_base_ptr: u64,
    pub ce_long_base_ptr: u64,
    pub ce_short_base_ptr: u64,

    pub row_stride_elems: usize,
    pub vidya_rows: usize,
    pub ce_rows: usize,
}
```

Then the kernel can compute:

```text
row_ptr = base_ptr + row_index * row_stride_bytes
```

instead of dereferencing uploaded row-pointer tables.

### Why it is attractive

- removes three H2D pointer-table uploads
- reduces host work creating pointer arrays
- simplifies lifetime reasoning if the batch owns one contiguous allocation per family
- works naturally with an uploaded-batch API

### Why it is optional / higher risk

It changes the kernel ABI and likely the upstream indicator batching layout.

I would do this **after** the safer fixes unless profiling shows row-pointer staging is still material.

---

### Optional extension 2 — use pinned host memory and async copies for large cold batches

The first plan mentioned pinned async upload as an optional follow-on. I would now make that recommendation more concrete.

If cold upload still dominates after residency reuse and scratch reuse, add a feature-gated path that:

- uses page-locked host memory for large readback buffers,
- uses async H2D upload for large cold `close_f32` / combo arrays,
- uses async D2H readback for metrics,
- synchronizes only at the end of the batch evaluation boundary.

### Important guardrail

Do this only after simpler reuse is in place. Pinned-memory code is more invasive and should not be the first optimization step.

### Sketch

```rust
// Pseudocode only; actual type depends on the CUDA wrapper layer you use.
struct VcetPinnedHostBuffers {
    metrics: PinnedHostBuffer<f32>,
}

impl VcetPinnedHostBuffers {
    fn ensure_metrics_len(&mut self, len: usize) -> Result<(), String> {
        if self.metrics.len() != len {
            self.metrics = PinnedHostBuffer::new(len)?;
        }
        Ok(())
    }
}
```

If your `cust` version does not expose a pinned host buffer abstraction directly, this becomes a later optional path via lower-level CUDA APIs.

---

### Optional extension 3 — preserve explicit route diagnostics when device-only prep fails

Once you split the VIDYA helper into strict and permissive forms, I would also preserve a structured reason for strict-route rejection.

For example:

```rust
#[derive(Debug, Clone)]
pub enum VcetGpuPrepError {
    DeviceVidyaUnavailable(String),
    DeviceChandelierUnavailable(String),
    UnsupportedStrategyOverlay,
    UnsupportedStopLoss,
    UnsupportedTakeProfit,
    InvalidSignalBatch(String),
}
```

That makes it easier for optimizer benches, validation benches, and route summaries to tell you **why** VCET did not stay on the GPU-native path.

This is not a product-surface feature; it is route/fallback transparency for debugging and benchmarking.

---

### Optional extension 4 — add no-poison tests if you adopt more `MaybeUninit`

If you take the uninitialized-allocation route further than just host readback, add debug-only poison checks similar in spirit to the adjacent ALMA tests.

That means:

- after any `alloc_uninit_copy(...)` readback path, assert all decoded rows are valid and no poison sentinel leaks,
- after any prefix-initialized matrix helper, assert the warmed prefix has the expected semantic fill (`NaN` or `0.0`) and the tail is fully overwritten,
- keep those checks debug-only so they do not affect release performance.

This is especially worthwhile if you add more local unsafe allocation or new shared alloc-policy helpers during the VCET refactor.

---

## 4. Things I would explicitly keep as-is unless profiling proves otherwise

### Keep initialized host vectors where the fill is part of observable behavior

I would **not** try to uninitialize or partially initialize these surfaces unless you can prove every read happens after overwrite:

- `build_vcet_desired_positions` output vector (`vec![0i32; n]` is semantically correct because zeros are visible before later positions are assigned)
- any CPU reference series where warmup `NaN` is observable behavior
- any fallback path that intentionally returns a fully materialized host series

### Do not remove the explicit sync before readback unless the async-copy semantics are audited

It is tempting to delete `rt.stream.synchronize()` before host readback, but I would not do that casually. If you move to async D2H on pinned memory, then audit and rework synchronization deliberately. Until then, the explicit sync is the safer correctness boundary.

### Do not fuse log-return computation into the main backtest kernel without evidence

The current VCET path computes log returns once into `d_lr` and then reuses them across all combo evaluations. That is often a good trade when many combos share the same price series.

So I would prioritize:

- reusing `d_lr`
- reusing `d_close`
- reusing `d_out`

before considering any kernel fusion that might recompute returns per combo.

---

## 5. Concrete code changes I would add to the implementation order

These steps should be inserted into the main implementation plan in this order.

### Added step A — strengthen `validate_inputs`

Implement the strict validation function shown above.

Add tests for:

- row-index out of bounds
- negative `start_t`
- `start_t >= end_t`
- `end_t > cols`
- null device pointer in row-pointer arrays
- `usize -> i32` overflow rejection

### Added step B — add `VcetKernelScratch`

Extend `VcetCudaRuntime` with:

- `launch_caps`
- `scratch`

Reuse:

- `d_out`
- combo arrays
- host metrics readback buffer

Use async uninitialized device allocation if your current runtime policy already allows it.

### Added step C — create an owning uploaded batch type

Prefer a batch object that keeps device owners alive and caches:

- `d_close`
- `d_lr`
- row-pointer tables
- row counts
- owner vectors for underlying signal rows

### Added step D — add a warm-eval microbench

Beyond the end-to-end request bench, add:

- `upload_only`
- `warm_eval_only`
- `end_to_end`

for VCET.

This is the cleanest way to know which optimization actually moved the intended timing boundary.

### Added step E — decide whether to keep pointer-table ABI or move to contiguous matrices

Do this after steps A-D.

If H2D pointer-table traffic still shows up in profiling, redesign the VCET kernel ABI around contiguous matrices plus base pointer + stride.

---

## 6. Expanded test additions I would append to the main checklist

### Input validation tests

```rust
#[test]
fn validate_inputs_rejects_negative_start_t() {
    let signals = VcetGpuDeviceSignalMatrices {
        close_f32: &[1.0, 2.0, 3.0],
        cols: 3,
        vidya_rows_ptr: &[0x1000],
        ce_long_rows_ptr: &[0x2000],
        ce_short_rows_ptr: &[0x3000],
        vidya_rows: 1,
        ce_rows: 1,
    };
    let combos = VcetGpuComboBatch {
        vidya_row_index: &[0],
        ce_row_index: &[0],
        start_t: &[-1],
        end_t: &[2],
    };
    let err = validate_inputs(&signals, &combos).unwrap_err();
    assert!(err.contains("non-negative"));
}

#[test]
fn validate_inputs_rejects_start_ge_end() {
    let signals = VcetGpuDeviceSignalMatrices {
        close_f32: &[1.0, 2.0, 3.0],
        cols: 3,
        vidya_rows_ptr: &[0x1000],
        ce_long_rows_ptr: &[0x2000],
        ce_short_rows_ptr: &[0x3000],
        vidya_rows: 1,
        ce_rows: 1,
    };
    let combos = VcetGpuComboBatch {
        vidya_row_index: &[0],
        ce_row_index: &[0],
        start_t: &[2],
        end_t: &[2],
    };
    let err = validate_inputs(&signals, &combos).unwrap_err();
    assert!(err.contains("start_t must be < end_t"));
}

#[test]
fn validate_inputs_rejects_row_index_out_of_bounds() {
    let signals = VcetGpuDeviceSignalMatrices {
        close_f32: &[1.0, 2.0, 3.0],
        cols: 3,
        vidya_rows_ptr: &[0x1000],
        ce_long_rows_ptr: &[0x2000],
        ce_short_rows_ptr: &[0x3000],
        vidya_rows: 1,
        ce_rows: 1,
    };
    let combos = VcetGpuComboBatch {
        vidya_row_index: &[1],
        ce_row_index: &[0],
        start_t: &[0],
        end_t: &[2],
    };
    let err = validate_inputs(&signals, &combos).unwrap_err();
    assert!(err.contains("vidya_row_index out of bounds"));
}
```

### Lifetime / uploaded-batch tests

Add at least one GPU-only test that:

1. uploads a VCET batch,
2. runs it twice,
3. confirms the second run reuses resident buffers,
4. and confirms dropping the original source vectors does not invalidate the uploaded batch.

### Warm-vs-cold benchmark sanity tests

If you add the upload/eval split, verify:

- cold upload is materially slower than warm eval,
- repeated warm eval is stable across iterations,
- route probes still report `BackendUsed::Gpu` for the intended GPU VCET bench.

---

## 7. Addendum checklist

Append these items to the bottom of the main checklist.

### Validation and safety

- [ ] Add `validate_inputs` checks for `cols <= i32::MAX`
- [ ] Add `validate_inputs` checks for `vidya_rows <= i32::MAX`
- [ ] Add `validate_inputs` checks for `ce_rows <= i32::MAX`
- [ ] Add `validate_inputs` checks for `combo_count <= i32::MAX`
- [ ] Reject negative `start_t`
- [ ] Reject negative `end_t`
- [ ] Reject `start_t >= end_t`
- [ ] Reject `end_t > cols`
- [ ] Reject `vidya_row_index` out of bounds
- [ ] Reject `ce_row_index` out of bounds
- [ ] Reject null row-pointer entries in `vidya_rows_ptr`
- [ ] Reject null row-pointer entries in `ce_long_rows_ptr`
- [ ] Reject null row-pointer entries in `ce_short_rows_ptr`

### Lifetime safety

- [ ] Add an owning uploaded VCET signal-batch type
- [ ] Ensure the batch keeps the underlying `VcetOwnedDeviceMatrix` owners alive
- [ ] Keep the raw-pointer borrowed struct only as a lower-level/internal API if still needed

### Scratch and allocation reuse

- [ ] Add `VcetKernelScratch` to `VcetCudaRuntime`
- [ ] Reuse `d_out` across repeated evaluations
- [ ] Reuse combo device buffers across repeated evaluations when length matches
- [ ] Reuse the host metrics readback buffer across repeated evaluations
- [ ] Use `alloc_uninit_copy(...)` or the shared alloc-policy helper for fully overwritten host readback buffers
- [ ] Prefer async uninitialized device allocation where the current runtime policy already supports it

### Benchmarks

- [ ] Add a VCET end-to-end optimizer benchmark
- [ ] Add a VCET end-to-end validation benchmark
- [ ] Add a VCET `upload_only` microbench
- [ ] Add a VCET `warm_eval_only` microbench
- [ ] Prime the runtime once before timed warm-eval iterations
- [ ] Keep route-probe output for all VCET benchmark surfaces
- [ ] Confirm GPU route probe reports `BackendUsed::Gpu` for intended VCET GPU benches

### Optional higher-risk follow-ons

- [ ] Decide whether to keep row-pointer table ABI or move to contiguous matrix ABI
- [ ] If keeping row-pointer ABI, measure the upload cost explicitly
- [ ] If moving to contiguous matrix ABI, pass `base_ptr + row_stride` instead of uploading pointer tables
- [ ] Add pinned host buffers if cold H2D/D2H still dominate after reuse fixes
- [ ] Add debug-only no-poison tests if you adopt more `MaybeUninit`/prefix-init helpers

---

## Bottom line

I do not think the first plan was directionally wrong. The biggest improvements are still the same.

What I would change after a second pass is:

1. make the validation and lifetime-safety story much stricter,
2. make reusable scratch explicit in the runtime,
3. split the benchmark surfaces more clearly,
4. and keep one bigger optional redesign in reserve: replacing uploaded row-pointer tables with contiguous matrices plus row-stride metadata.

If you implement only the in-scope changes first, I would do them in this order:

1. strict validation and route transparency,
2. strict device-only VIDYA path,
3. uploaded batch + runtime scratch reuse,
4. uninitialized readback reuse,
5. VCET benchmark coverage,
6. then only afterwards evaluate the contiguous-matrix ABI redesign.
