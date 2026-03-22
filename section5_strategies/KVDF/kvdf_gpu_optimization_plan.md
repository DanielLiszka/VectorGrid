# KVDF GPU-native optimization and validation plan

## Scope and review frame

This document is scoped to the **Bundle 5: strategy-local GPU-native optimization and validation routes** surface from `gpt54_feature_review_map.md`, narrowed to **KVDF only**.

### What this bundle owns

- strategy-local GPU-native optimization logic
- strategy-local CPU reference vs GPU scoring parity surfaces
- per-strategy staging costs, fp32-stability constraints, and route blockers
- strategy-specific validation cost differences that can be lost inside broad composite reviews

### Performance review focus

- one strategy at a time, not all strategies in one review
- strategy-local staging or upload waste
- CPU-reference prep that can erase GPU benefit
- per-strategy fallback reasons and fp32-stability blockers
- strategy-specific validation throughput, not just optimizer throughput
- whether KVDF should remain GPU-preferred or CPU-preferred on current measured workloads

### Ask for

- strategy-local hot-path review
- CPU/GPU payoff review
- staging/fallback review
- per-strategy benchmark review

### Files reviewed

- `gpt54_feature_review_map.md`
- `kvdf.rs`
- `kvdf_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

### Constraints carried forward from the review map

- stay grounded in the provided files
- prefer smaller, parity-safe changes before larger refactors
- separate confirmed issue, likely issue, open question, and optimization opportunity
- identify the relevant timing boundary for each material performance claim
- prefer existing perf-gate and benchmark surfaces before proposing entirely new instrumentation
- prefer shared helpers from `alloc_policy.rs` over new ad hoc unsafe allocation code
- only use uninitialized allocation when every element is guaranteed to be written before any read

---

## Surface understanding

KVDF currently spans **two separate hot paths** in the provided files:

1. `kvdf.rs` prepares indicator-driven signal surfaces for KVDF.
2. `kvdf_gpu_kernel.rs` launches the final CUDA scoring kernel over device-resident row-major matrices.

The current implementation already does one important thing correctly: **CUDA runtime/context/module creation is cached per thread/device** in `kvdf_gpu_kernel.rs`, so the main remaining performance issues are not cold-start context creation. The bigger remaining cost centers are:

- repeated **host/device staging** during GPU scoring
- repeated **GPU-to-host HostF32 materialization** during signal preparation
- **HostF32 -> Vec<f64> widening** on a GPU-prepared path
- repeated per-call **tiny heap allocations** and output-id resolution work inside KVDF signal prep
- incomplete **KVDF-specific validation evidence** at larger candidate/workload shapes

In practical terms, the likely current KVDF GPU route still pays too much CPU-side prep overhead before it ever reaches the final CUDA metrics kernel.

---

## Current hot-path map

### 1. `kvdf.rs` current prep path

`compute_indicator_f64(...)` tries CUDA when the backend prefers GPU and prepared CUDA inputs are available, but it requests `CudaOutputTarget::HostF32`, then converts that output into a fresh `Vec<f64>`.

That means the current GPU-prepared signal path can still look like this:

1. indicator computed on CUDA
2. result copied back to host as `Vec<f32>`
3. result widened to `Vec<f64>`
4. downstream path may later re-narrow or restage to `f32` for GPU scoring

That is exactly the kind of strategy-local prep overhead that can erase GPU benefit.

### 2. `kvdf_gpu_kernel.rs` current scoring path

`eval_kvdf_metrics_gpu_kernel_device_rows(...)` receives device pointers for the large indicator matrices, but it still:

1. uploads `signals.close_f32` into a fresh `DeviceBuffer`
2. allocates a fresh `d_lr`
3. launches `compute_log_returns_f32`
4. uploads combo index arrays into fresh device buffers
5. allocates a fresh device output buffer
6. copies metrics back into a zero-filled host `Vec<f32>`

This means the row-major indicator matrices are already resident on device, but the market close series and combo arrays are still staged every call.

### 3. Benchmark surfaces

`gpu_native_expansion_opt.rs` already gives KVDF dedicated CPU and GPU optimizer Criterion coverage. That is a good foundation.

`composite_validation_workflow.rs` includes KVDF in the broad 50k/8 profile runs, but the more meaningful GPU workflow case at 100k/64 is ASR-only, and the larger profile expansion only covers CARM and CDMC. That is the main KVDF validation-evidence gap.

---

## Findings

## Confirmed issues

### Finding 1

- **Severity:** high
- **Type:** performance
- **Location:** `kvdf.rs` -> `compute_indicator_f64`
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** the GPU-prepared path explicitly asks for `CudaOutputTarget::HostF32`, then converts that host `Vec<f32>` into a fresh `Vec<f64>`. For a GPU-targeted KVDF path, that introduces both a device-to-host materialization and an immediate widening pass on every indicator output.
- **Recommended change:** add a dedicated **KVDF GPU-prep `f32` surface** that returns `Vec<f32>` (and eventually optional device outputs) rather than forcing all GPU-prepared work through `Vec<f64>`.

### Finding 2

- **Severity:** high
- **Type:** performance
- **Location:** `kvdf_gpu_kernel.rs` -> `eval_kvdf_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** the final KVDF metrics kernel already consumes device-resident row-major indicator matrices, but the close series is still uploaded from host and log returns are still recomputed on every call.
- **Recommended change:** extend the KVDF kernel input surface so the caller can optionally provide **device-resident close** and **device-resident log returns**, with the current host-slice path preserved as a fallback.

### Finding 3

- **Severity:** medium
- **Type:** performance
- **Location:** `kvdf_gpu_kernel.rs` -> `eval_kvdf_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** combo index arrays are uploaded into fresh `DeviceBuffer`s on every call. That is confirmed staging work. Whether it is the dominant bottleneck depends on how much reuse the caller gets across optimization and validation passes.
- **Recommended change:** after close/log-return residency is fixed, add either:
  - an optional device-combo input path, or
  - a caller-owned prepared combo batch surface for repeated scoring

### Finding 4

- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `composite_validation_workflow.rs` -> GPU validation benchmark groups
- **Timing boundary affected:** validation-cost
- **Why it matters:** the GPU validation benchmarks print the route summary, but they do **not assert** that the backend used is actually GPU. A silent CPU fallback could therefore contaminate the benchmark result without failing the run.
- **Recommended change:** add the same style of explicit `BackendUsed::Gpu` assertions that already exist in `gpu_native_expansion_opt.rs`.

### Finding 5

- **Severity:** medium
- **Type:** performance
- **Location:** `kvdf_gpu_kernel.rs` -> host readback of `host_metrics`
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** the code zero-fills `host_metrics` before `copy_to(...)` overwrites the entire buffer.
- **Recommended change:** move this readback through a shared `alloc_policy` uninitialized-host helper, but only after confirming the CUDA kernel writes **every output element** for every scored combo.

### Finding 6

- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `gpu_native_expansion_opt.rs` and `composite_validation_workflow.rs`
- **Timing boundary affected:** Criterion, validation-cost
- **Why it matters:** KVDF optimizer benchmarking is stronger than KVDF validation benchmarking. The optimizer file has dedicated KVDF CPU/GPU groups at 200k candles, but validation still lacks a KVDF-specific 100k/64 GPU workflow case and a larger KVDF candidate-count shape.
- **Recommended change:** add a dedicated **KVDF 100k/64 GPU validation workflow group** and at least one **larger KVDF profile case**.

### Finding 7

- **Severity:** medium
- **Type:** correctness / route-fidelity
- **Location:** `kvdf.rs` -> `compute_indicator_f64`
- **Timing boundary affected:** route-matrix, Criterion, validation-cost
- **Why it matters:** CUDA indicator failures silently fall back to CPU. That keeps the feature working, but it hides whether a supposedly GPU-preferred KVDF run actually prepared indicator surfaces on GPU or CPU.
- **Recommended change:** add a **KVDF prep trace summary** that records per-indicator backend used and fallback reason.

### Finding 8

- **Severity:** low
- **Type:** performance
- **Location:** `kvdf.rs` -> `compute_kvdf_slow_signals`, `compute_kvdf_shared_signals`, `compute_kvdf_dti_signal`, `compute_kvdf_fast_signals`
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** these functions build several tiny `Vec<IndicatorParamKV>` allocations and re-resolve indicator output ids on every call.
- **Recommended change:** cache resolved output ids with `OnceLock`, and replace the outer `Vec` parameter containers with fixed arrays/slices.

## Likely issues

### Finding 9

- **Severity:** high
- **Type:** performance
- **Location:** `kvdf.rs` -> repeated Keltner / FVG / DevStop output computation
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** KVDF asks for Keltner three times, FVG trailing stop twice, and DevStop twice. If the dynamic indicator layer computes each requested output independently, that multiplies work. If it already caches internally, the risk is lower.
- **Recommended change:** verify whether the adjacent dynamic-indicator layer deduplicates/multi-emits outputs. If not, add family-level multi-output helpers.

### Finding 10

- **Severity:** medium
- **Type:** performance
- **Location:** `composite_validation_workflow.rs` -> KVDF validation groups
- **Timing boundary affected:** validation-cost
- **Why it matters:** the generic synthetic candles may not reflect the flow/gap structure that exercises KVDF signal surfaces well, especially FVG-related warmup and branch patterns.
- **Recommended change:** add a **KVDF-shaped candle generator** for KVDF-specific validation groups, mirroring the strategy-specific optimizer benchmark shape.

## Open questions

### Finding 11

- **Severity:** medium
- **Type:** open question
- **Location:** adjacent caller(s) not provided
- **Timing boundary affected:** unknown
- **Why it matters:** the provided files do not show the upstream code that builds `KvdfGpuDeviceSignalMatrices`. That missing context determines whether `close_f32` and combo arrays are already reused or rebuilt per fold/call.
- **Recommended change:** inspect the upstream KVDF GPU row-builder before committing to device-combo caching design.

### Finding 12

- **Severity:** medium
- **Type:** open question
- **Location:** adjacent CUDA module / alloc helper not provided
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** safe use of uninitialized host readback depends on the CUDA kernel writing every output element. The kernel body is not provided here.
- **Recommended change:** confirm full-write behavior before replacing zero-filled host readback.

---

## Safe optimization opportunities

These are the changes I would implement first.

1. **Cache output ids and stop rebuilding the same output-id strings every call.**
2. **Replace outer parameter `Vec`s with fixed arrays/slices** to remove avoidable tiny heap allocations.
3. **Add a KVDF-specific `f32` prep path** for GPU-bound workflows while keeping the existing `f64` path for CPU/reference use.
4. **Add a KVDF prep trace** so you can see whether indicator families used CUDA or CPU.
5. **Add device-resident `close` and `lr` support** to the GPU kernel interface.
6. **Add checked `usize -> i32` conversions** in the kernel wrapper instead of unchecked `as i32` casts.
7. **Use shared uninitialized-host readback helpers** for `host_metrics` once the full-write guarantee is verified.
8. **Assert GPU route usage in GPU validation benches.**
9. **Add KVDF-specific 100k/64 validation workflow coverage.**
10. **Add explicit route-blocker tests** for stop-loss/take-profit/overlay rejection paths.

---

## Higher-risk optimization opportunities

These are worthwhile, but I would only take them after the safe changes above are measured.

1. **Direct `DeviceF32` indicator output path** for KVDF row construction.
   - This would skip HostF32 readback entirely for GPU row-builders.
   - It requires adjacent row-builder changes not shown in the provided files.

2. **Family-level multi-output indicator APIs** for Keltner/FVG/DevStop.
   - This is likely one of the biggest remaining KVDF prep wins if the current dynamic-indicator layer recomputes each output independently.
   - It requires adjacent dynamic-indicator changes.

3. **Prepared device combo batches** for repeated validation/optimization scoring.
   - Useful if the same candidate/index vectors are reused across folds or passes.
   - Do not build this until you confirm the caller reuse pattern.

4. **Pinned host readback buffers** for metrics if DtoH readback remains measurable after the larger copy eliminations.

---

## Detailed implementation plan by file

# 1) `kvdf.rs`

## Goals

- keep the CPU/reference path correct and easy to reason about
- stop forcing GPU-prepared signal work through `Vec<f64>`
- reduce repeated tiny allocation work in the per-combo KVDF prep path
- make fallback behavior visible
- create a clean surface that upstream GPU row-builders can consume

## Recommended edits

### Edit 1.1: cache resolved output ids with `OnceLock`

`resolve_output_id(...)` is pure lookup work against a registered descriptor surface. KVDF repeatedly resolves the same output ids on every signal-prep call.

Use a small cached bundle:

```rust
use std::sync::OnceLock;

#[derive(Debug)]
struct KvdfOutputIds {
    keltner_upper: String,
    keltner_middle: String,
    keltner_lower: String,
    vwap_value: String,
    dti_value: String,
    devstop_value: String,
    fvg_upper_ts: String,
    fvg_lower_ts: String,
}

fn kvdf_output_ids() -> &'static KvdfOutputIds {
    static IDS: OnceLock<KvdfOutputIds> = OnceLock::new();
    IDS.get_or_init(|| KvdfOutputIds {
        keltner_upper: resolve_output_id("keltner", &["upper_band", "upperband", "upper"])
            .expect("KVDF keltner upper output id"),
        keltner_middle: resolve_output_id("keltner", &["middle_band", "middleband", "middle"])
            .expect("KVDF keltner middle output id"),
        keltner_lower: resolve_output_id("keltner", &["lower_band", "lowerband", "lower"])
            .expect("KVDF keltner lower output id"),
        vwap_value: resolve_output_id("vwap", &["value", "vwap"])
            .expect("KVDF VWAP output id"),
        dti_value: resolve_output_id("dti", &["value", "dti"])
            .expect("KVDF DTI output id"),
        devstop_value: resolve_output_id("devstop", &["value"])
            .expect("KVDF DevStop output id"),
        fvg_upper_ts: resolve_output_id("fvg_trailing_stop", &["upper_ts", "upper"])
            .expect("KVDF FVG upper trailing-stop output id"),
        fvg_lower_ts: resolve_output_id("fvg_trailing_stop", &["lower_ts", "lower"])
            .expect("KVDF FVG lower trailing-stop output id"),
    })
}
```

Then replace repeated per-call `resolve_output_id(...)` usage with:

```rust
let ids = kvdf_output_ids();
```

### Edit 1.2: replace outer parameter `Vec`s with fixed arrays/slices

This is a modest improvement, but it is free and safe.

```rust
let keltner_params = [
    IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(periods.keltner)),
    },
    IndicatorParamKV {
        key: "multiplier".to_string(),
        value: IndicatorParamValue::Float(cfg.keltner_mult),
    },
    IndicatorParamKV {
        key: "ma_type".to_string(),
        value: IndicatorParamValue::EnumString("ema".to_string()),
    },
];

let keltner_upper = compute_indicator_f64(
    candles,
    ma_source,
    backend,
    "keltner",
    ids.keltner_upper.as_str(),
    &keltner_params,
    #[cfg(feature = "cuda")]
    prepared_cuda_inputs,
)?;
```

Do the same for:

- `devstop_long_params`
- `devstop_short_params`
- `vwap_params`
- `dti_params`
- `fvg_params`

### Edit 1.3: add a KVDF `f32` prep surface for GPU-bound workflows

Keep the current `compute_kvdf_signals(...) -> KvdfSignals` API for CPU/reference use. Add a **parallel GPU-prep surface** rather than mutating the existing CPU-friendly signature.

Recommended new types:

```rust
#[cfg(feature = "cuda")]
#[derive(Clone, Debug)]
pub struct KvdfSignalsF32 {
    pub keltner_upper: Vec<f32>,
    pub keltner_middle: Vec<f32>,
    pub keltner_lower: Vec<f32>,
    pub vwap: Vec<f32>,
    pub dti: Vec<f32>,
    pub devstop_long: Vec<f32>,
    pub devstop_short: Vec<f32>,
    pub fvg_upper_ts: Vec<f32>,
    pub fvg_lower_ts: Vec<f32>,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug)]
pub struct KvdfPrepTraceEntry {
    pub indicator_id: &'static str,
    pub output_id: String,
    pub backend_used: &'static str,
    pub fallback_reason: Option<String>,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug, Default)]
pub struct KvdfPrepTrace {
    pub entries: Vec<KvdfPrepTraceEntry>,
}
```

Add an `f32` CPU conversion helper:

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

Then add a GPU-preferred helper that returns both values and trace information:

```rust
#[cfg(feature = "cuda")]
fn compute_indicator_f32_for_gpu(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &'static str,
    output_id: &str,
    params: &[IndicatorParamKV],
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<(Vec<f32>, KvdfPrepTraceEntry), String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("KVDF indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    let prefer_gpu = backend_prefers_gpu(backend)
        && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);

    let mut fallback_reason: Option<String> = None;

    if prefer_gpu {
        match prepared_cuda_inputs {
            Some(prepared) => match compute_dynamic_indicator_cuda_with_inputs(
                prepared,
                indicator_id,
                Some(output_id),
                params,
                Kernel::Auto,
                vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
            ) {
                Ok(out) => match out.series {
                    DynamicIndicatorCudaSeries::HostF32(v) => {
                        return Ok((
                            v,
                            KvdfPrepTraceEntry {
                                indicator_id,
                                output_id: output_id.to_string(),
                                backend_used: "cuda",
                                fallback_reason: None,
                            },
                        ));
                    }
                    DynamicIndicatorCudaSeries::DeviceF32(_) => {
                        fallback_reason = Some(
                            "received DeviceF32 on KVDF HostF32 prep path".to_string(),
                        );
                    }
                },
                Err(e) => {
                    fallback_reason = Some(e.to_string());
                }
            },
            None => {
                fallback_reason = Some("prepared CUDA inputs missing".to_string());
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

    Ok((
        series_to_f32(out),
        KvdfPrepTraceEntry {
            indicator_id,
            output_id: output_id.to_string(),
            backend_used: "cpu",
            fallback_reason,
        },
    ))
}
```

Then build the new surface:

```rust
#[cfg(feature = "cuda")]
pub fn compute_kvdf_signals_f32_for_gpu(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KvdfResolvedPeriods,
    cfg: &KvdfStrategyParams,
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<(KvdfSignalsF32, KvdfPrepTrace), String> {
    let ids = kvdf_output_ids();
    let mut trace = KvdfPrepTrace::default();

    let keltner_params = [
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.keltner)),
        },
        IndicatorParamKV {
            key: "multiplier".to_string(),
            value: IndicatorParamValue::Float(cfg.keltner_mult),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString("ema".to_string()),
        },
    ];

    let (keltner_upper, tr0) = compute_indicator_f32_for_gpu(
        candles,
        ma_source,
        backend,
        "keltner",
        ids.keltner_upper.as_str(),
        &keltner_params,
        prepared_cuda_inputs,
    )?;
    trace.entries.push(tr0);

    // repeat for keltner_middle, keltner_lower, vwap, dti, devstop_long,
    // devstop_short, fvg_upper_ts, fvg_lower_ts

    let first_valid = [
        keltner_upper.iter().position(|v| v.is_finite()),
        // include all other surfaces here
    ]
    .into_iter()
    .flatten()
    .max()
    .ok_or_else(|| "KVDF GPU-prepared signals are all non-finite".to_string())?;

    Ok((
        KvdfSignalsF32 {
            keltner_upper,
            keltner_middle: vec![],
            keltner_lower: vec![],
            vwap: vec![],
            dti: vec![],
            devstop_long: vec![],
            devstop_short: vec![],
            fvg_upper_ts: vec![],
            fvg_lower_ts: vec![],
            first_valid,
        },
        trace,
    ))
}
```

Notes:

- The above is the **shape** to implement; fill in the remaining series exactly as the existing `compute_kvdf_*` helpers do.
- Keep the current `f64` API for CPU/reference paths so you do not break existing CPU logic.
- The intended upstream migration is: **GPU row-builders use `KvdfSignalsF32`; CPU/reference paths keep using `KvdfSignals`.**

### Edit 1.4: optionally track `first_valid` during conversion instead of rescanning later

This is secondary, but cheap.

Instead of converting then scanning later, you can compute `first_valid` while building the output vector. This is especially useful for the new `f32` prep path.

A minimal helper pattern:

```rust
fn first_finite_idx_f32(values: &[f32]) -> Option<usize> {
    values.iter().position(|v| v.is_finite())
}
```

Then compute `first_valid` once in the new `KvdfSignalsF32` builder rather than rescanning the same vectors again elsewhere.

### Edit 1.5: keep the current `compute_indicator_f64` path, but make it explicitly CPU/reference oriented

Once the new `f32` GPU-prep surface exists, the existing `compute_indicator_f64(...)` can stay as the reference path. That avoids a broad API break.

I would also add a doc comment clarifying this intent:

```rust
/// CPU/reference-oriented indicator surface for KVDF.
/// GPU-bound row builders should prefer `compute_kvdf_signals_f32_for_gpu`.
```

## Recommended tests for `kvdf.rs`

### Test 1: `KvdfSignalsF32` parity vs existing CPU/reference path

Use the same candles and params, then compare:

- `compute_kvdf_signals(...)` converted to `f32`
- `compute_kvdf_signals_f32_for_gpu(...)`

Suggested tolerance policy:

- start with `max_abs_diff <= 1e-4` on most surfaces
- permit a slightly looser tolerance on the more numerically sensitive ones if needed
- document any per-indicator tolerance widening rather than hiding it

### Test 2: prep trace visibility

Add a test proving that when the backend prefers GPU but prepared CUDA inputs are missing, the new trace records a CPU fallback with a reason.

### Test 3: preserve current smoke test

Keep `compute_kvdf_signals_gpu_smoke_runs_with_all_indicator_surfaces()` as a broad integration smoke test even after the new `f32` surface lands.

---

# 2) `kvdf_gpu_kernel.rs`

## Goals

- stop restaging market series that should already be resident
- stop recomputing log returns when precomputed device-resident values are available
- reduce per-call staging and metadata lookup overhead
- use uninitialized memory safely where every element is guaranteed to be written
- tighten validation and blocker tests

## Recommended edits

### Edit 2.1: extend `KvdfGpuDeviceSignalMatrices` to optionally accept device-resident close/log-return inputs

Current struct:

```rust
pub struct KvdfGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    // ...
}
```

Recommended shape:

```rust
pub struct KvdfGpuDeviceSignalMatrices<'a> {
    pub close_f32: Option<&'a [f32]>,
    pub close_device_ptr: u64,
    pub lr_device_ptr: u64,
    pub cols: usize,
    pub dti_rows: usize,
    pub dti_stride: usize,
    pub keltner_upper_row_major_ptr: u64,
    pub keltner_middle_row_major_ptr: u64,
    pub keltner_lower_row_major_ptr: u64,
    pub vwap_row_major_ptr: u64,
    pub dti_row_major_ptr: u64,
    pub slow_rows: usize,
    pub slow_stride: usize,
    pub devstop_long_row_major_ptr: u64,
    pub devstop_short_row_major_ptr: u64,
    pub fvg_upper_ts_row_major_ptr: u64,
    pub fvg_lower_ts_row_major_ptr: u64,
    pub fast_rows: usize,
    pub fast_stride: usize,
}
```

Validation should then enforce:

- either `close_device_ptr != 0` or `close_f32.is_some()`
- `lr_device_ptr` may be zero, in which case log returns are computed from whichever close source is available

### Edit 2.2: change log-return launcher to accept a raw device pointer

That lets you compute log returns from either:

- an uploaded `DeviceBuffer<f32>` created locally from `close_f32`, or
- an existing device-resident close pointer supplied by the caller

```rust
fn launch_compute_log_returns_f32_raw(
    rt: &mut KvdfCudaRuntime,
    close_ptr_raw: u64,
    d_lr: &mut DeviceBuffer<f32>,
    len: usize,
) -> Result<(), String> {
    if len == 0 {
        return Ok(());
    }

    let kernel = rt
        .module
        .get_function("compute_log_returns_f32")
        .map_err(|e| e.to_string())?;

    let launch = choose_launch_shape_1d_from_candidates(
        len,
        rt.device_caps,
        detect_kernel_launch_caps(&kernel, rt.device_caps),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr_raw;
    let mut len_i = i32::try_from(len)
        .map_err(|_| "KVDF GPU close length exceeds i32 range".to_string())?;
    let mut lr_ptr = d_lr.as_device_ptr().as_raw();

    unsafe {
        let args: &mut [*mut c_void] = &mut [
            &mut close_ptr as *mut _ as *mut c_void,
            &mut len_i as *mut _ as *mut c_void,
            &mut lr_ptr as *mut _ as *mut c_void,
        ];
        rt.stream
            .launch(
                &kernel,
                (launch.grid_x.max(1), 1, 1),
                (launch.block_x.max(1), 1, 1),
                0,
                args,
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
```

### Edit 2.3: cache `device_caps` in `KvdfCudaRuntime`

Current code re-gets the device and re-detects launch caps in both the log-return launch and the main kernel launch.

Recommended runtime shape:

```rust
struct KvdfCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    device_caps: crate::gpu_backtest_kernel::DeviceLaunchCaps,
}

impl KvdfCudaRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let device_caps = detect_device_launch_caps(device);
        let context = Arc::new(Context::new(device).map_err(|e| e.to_string())?);
        let module = load_double_crossover_module(device)?;
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;

        Ok(Self {
            context,
            module,
            stream,
            device_caps,
        })
    }
}
```

This is a small win, but it is clean and low risk.

### Edit 2.4: use checked integer conversions instead of unchecked casts

Current code uses several `as i32` conversions:

- `signals.cols as i32`
- `signals.dti_rows as i32`
- `signals.dti_stride as i32`
- `signals.slow_rows as i32`
- `signals.slow_stride as i32`
- `signals.fast_rows as i32`
- `signals.fast_stride as i32`
- `combo_count as i32`

Replace them with `i32::try_from(...)` and return a descriptive error if they overflow.

Example:

```rust
let mut t_i = i32::try_from(signals.cols)
    .map_err(|_| "KVDF GPU cols exceeds i32 range".to_string())?;
let mut dti_rows_i = i32::try_from(signals.dti_rows)
    .map_err(|_| "KVDF GPU dti_rows exceeds i32 range".to_string())?;
// ... repeat for the rest
```

This is mostly correctness, but it is the right cleanup for a kernel wrapper.

### Edit 2.5: resolve close/lr pointers once, then launch the kernel

Recommended local pattern in `eval_kvdf_metrics_gpu_kernel_device_rows(...)`:

```rust
let mut owned_close: Option<DeviceBuffer<f32>> = None;
let close_ptr = if signals.close_device_ptr != 0 {
    signals.close_device_ptr
} else {
    let host_close = signals
        .close_f32
        .ok_or_else(|| "KVDF GPU close source missing".to_string())?;
    let d_close = DeviceBuffer::from_slice(host_close).map_err(|e| e.to_string())?;
    let ptr = d_close.as_device_ptr().as_raw();
    owned_close = Some(d_close);
    ptr
};

let mut owned_lr: Option<DeviceBuffer<f32>> = None;
let lr_ptr = if signals.lr_device_ptr != 0 {
    signals.lr_device_ptr
} else {
    let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(signals.cols) }
        .map_err(|e| e.to_string())?;
    launch_compute_log_returns_f32_raw(rt, close_ptr, &mut d_lr, signals.cols)?;
    let ptr = d_lr.as_device_ptr().as_raw();
    owned_lr = Some(d_lr);
    ptr
};
```

Then use `close_ptr` and `lr_ptr` in the main kernel argument list.

This gives you a **backward-compatible fallback path** while enabling the caller to skip both the close upload and log-return recomputation when those are already resident.

### Edit 2.6: use shared uninitialized-host readback for `host_metrics`

Current code:

```rust
let mut host_metrics = vec![0.0f32; out_len];
d_out
    .copy_to(host_metrics.as_mut_slice())
    .map_err(|e| e.to_string())?;
```

Recommended direction:

```rust
let mut host_metrics = alloc_policy::host_readback_uninit_f32(out_len)
    .map_err(|e| e.to_string())?;
d_out
    .copy_to(host_metrics.as_mut_init_slice())
    .map_err(|e| e.to_string())?;
let host_metrics = unsafe { host_metrics.assume_init_vec() };
```

Important notes:

- The helper name above is illustrative. Use the existing helper surface in `alloc_policy.rs` if it already exists.
- Do **not** hand-roll a local `set_len` / `MaybeUninit` pattern in this file if the shared helper already exists or can be added centrally.
- Only do this once you confirm the kernel writes the full metrics row for every combo.

### Edit 2.7: plan a phase-2 prepared combo path, but do not start there

A future prepared-combo surface could look like:

```rust
pub struct KvdfGpuPreparedComboBatch {
    pub fast_row_index_ptr: u64,
    pub slow_row_index_ptr: u64,
    pub dti_row_index_ptr: u64,
    pub start_t_ptr: u64,
    pub end_t_ptr: u64,
    pub len: usize,
}
```

Then add an overload:

```rust
pub fn eval_kvdf_metrics_gpu_kernel_prepared_rows(
    device_id: u32,
    signals: &KvdfGpuDeviceSignalMatrices<'_>,
    combos: &KvdfGpuPreparedComboBatch,
    strategy: &StrategyConfig,
    kvdf: &KvdfStrategyParams,
) -> Result<Vec<Metrics>, String>
```

I would only implement this after measuring the gains from close/lr residency.

## Recommended tests for `kvdf_gpu_kernel.rs`

### Test 1: early return on empty combos

This is easy to test without real CUDA work.

### Test 2: blocker path fires before any CUDA launch

Add a unit test that verifies the function rejects unsupported execution overlays / stop-loss / take-profit **before** doing any device launch work.

Because the exact `StrategyConfig` setter surface is not shown in the provided files, implement this using whatever field or builder makes `uses_execution_overlays()` / `stop_loss_pct()` / `take_profit_pct()` return a value in your local repo.

### Test 3: validation accepts device-close mode without host close slice

Once `close_device_ptr` / `lr_device_ptr` exists, add a validation-level test proving that a device-resident path can skip `close_f32`.

---

# 3) `gpu_native_expansion_opt.rs`

## Goals

- make CPU vs GPU KVDF payoff easier to compare at the same search sizes
- preserve the existing route-assert discipline
- expose any new KVDF prep-trace summary once it exists

## Recommended edits

### Edit 3.1: add a large CPU KVDF case to mirror the GPU case

Current CPU cases:

```rust
let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
    ("small", (5, 20, 1), (20, 80, 1)),
    ("medium", (5, 30, 1), (30, 120, 1)),
];
```

Recommended:

```rust
let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
    ("small/975pairs", (5, 20, 1), (20, 80, 1)),
    ("medium/2365pairs", (5, 30, 1), (30, 120, 1)),
    ("large/13385pairs", (5, 80, 1), (20, 220, 1)),
];
```

This makes CPU/GPU comparison much cleaner.

### Edit 3.2: keep the existing route assertions exactly as strict as they are now

This file is already doing the right thing by:

- probing the route once
- printing the route summary
- asserting `BackendUsed::Gpu`
- asserting GPU usage inside the measured iterations too

Do not loosen that.

### Edit 3.3: optionally print the new KVDF prep trace summary in the route probe

Once the result surface exposes a KVDF prep trace summary, print it in the same route-probe block. Keep it outside the timed loop.

Illustrative pattern:

```rust
if let Some(kvdf_trace) = probe.kvdf_prep_trace.as_ref() {
    println!("kvdf prep trace: {:?}", kvdf_trace);
}
```

The exact field path depends on the adjacent result type not provided here.

### Edit 3.4: keep KVDF-specific candles

This file already uses `make_kvdf_flow_candles(200_000)` for KVDF optimizer benchmarking. Keep that. It is a better strategy-local shape than a generic synth series.

---

# 4) `composite_validation_workflow.rs`

## Goals

- make KVDF validation throughput measurable at a meaningful GPU workload shape
- stop silent CPU fallback from contaminating GPU validation benchmarks
- use KVDF-shaped candles for KVDF-specific validation groups
- extend the existing benchmark matrix without inventing a new benchmark family

## Recommended edits

### Edit 4.1: assert `BackendUsed::Gpu` in the GPU validation route probes

Import `BackendUsed`:

```rust
use ta_app_core::{
    BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeParams,
    CompositeValidationRequest, CscvPboConfig, PerturbConfig, TradeBootstrapConfig,
    WalkForwardConfigOptional,
};
```

Then add assertions after the route probe in GPU groups:

```rust
assert!(
    matches!(route_probe.backend_used, BackendUsed::Gpu),
    "expected GPU backend for composite_validate/gpu_kernel/kvdf/100k/64candidates, got backend={:?}, route={:?}, fallback={:?}",
    route_probe.backend_used,
    route_probe.backend_route,
    route_probe.backend_fallback_reason,
);
```

Do this for:

- the existing ASR GPU workflow group
- the generic GPU `bench_profile(...)` groups
- the new KVDF-specific GPU workflow group below

### Edit 4.2: add a KVDF-specific GPU workflow group at 100k / 64 candidates

Mirror the ASR workflow shape, but use KVDF candidates.

Recommended additional group:

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_composite_validation_gpu_workflow_kvdf(c: &mut Criterion) {
    let candles = make_kvdf_flow_candles(100_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_kvdf_candidates(64);

    let mut group = c.benchmark_group("composite_validate/gpu_kernel/kvdf/100k/64candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let base = CompositeValidationRequest {
        data_id: "bench-gpu-kvdf".to_string(),
        backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: candidates.clone(),
        walk_forward: Some(WalkForwardConfigOptional {
            window_bars: 20_000,
            step_bars: 20_000,
        }),
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: Some(CscvPboConfig {
            slices: 8,
            max_combinations: 2_000,
            threshold: 0.0,
        }),
        seed: 123,
        max_candidates: candidates.len(),
        validation_engine: ta_app_core::ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    };

    let route_probe = ta_app_core::run_composite_validation_blocking_with_candles(
        base.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("gpu kvdf validation route probe");

    assert!(
        matches!(route_probe.backend_used, BackendUsed::Gpu),
        "expected GPU backend for KVDF validation probe, got backend={:?}, route={:?}, fallback={:?}",
        route_probe.backend_used,
        route_probe.backend_route,
        route_probe.backend_fallback_reason,
    );

    group.bench_function("wf_cscv", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("gpu kvdf wf+cscv validate");
        })
    });

    group.finish();
}
```

Then add it to the Criterion group.

### Edit 4.3: add a larger KVDF profile case

Add at least one of the following:

```rust
bench_cpu_profile("kvdf", 100_000, make_kvdf_candidates(32));
bench_profile("kvdf", 100_000, make_kvdf_candidates(32));
```

If your GPU has the headroom and the runtime is still manageable, also consider:

```rust
bench_profile("kvdf", 100_000, make_kvdf_candidates(64));
```

### Edit 4.4: use KVDF-shaped candles for KVDF-specific validation groups

This file currently uses a generic synthetic candle generator. Add a KVDF-shaped candle helper similar to the one in `gpu_native_expansion_opt.rs`.

Recommended local helper:

```rust
fn make_kvdf_flow_candles(len: usize) -> Candles {
    let len = len.max(280);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64 * 60_000);
        let px = if i < 90 {
            100.0 + (i as f64 * 0.08).sin() * 0.9 + i as f64 * 0.03
        } else if i < 170 {
            102.8 + (i - 90) as f64 * 0.17 + ((i - 90) as f64 * 0.11).cos() * 0.8
        } else if i < 230 {
            116.0 - (i - 170) as f64 * 0.22 + ((i - 170) as f64 * 0.17).sin() * 1.1
        } else {
            102.4 + (i - 230) as f64 * 0.12 + ((i - 230) as f64 * 0.18).cos() * 0.7
        };
        open.push(px * 0.9985);
        high.push(px * 1.0045);
        low.push(px * 0.9955);
        close.push(px);
        vol.push(1_500.0 + (i % 19) as f64 * 55.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

Use this helper in the KVDF-specific validation groups rather than the generic `make_synth_candles(...)`.

### Edit 4.5: optionally print KVDF prep-trace summaries in route probes

Once the adjacent result type exposes the KVDF prep trace, print it outside the measured loop.

---

## Benchmark and perf-gate review

## What is already good

- `gpu_native_expansion_opt.rs` already has **dedicated KVDF CPU and GPU optimizer benches**.
- it already performs a **route probe and GPU assertion**, which is exactly the right discipline for a GPU-native strategy benchmark
- `composite_validation_workflow.rs` already includes KVDF in the broader profile sweep, so the existing validation harness is the right place to extend coverage

## What is missing

1. **A KVDF-specific GPU validation workflow case at a meaningful size.**
2. **Explicit GPU route assertions in GPU validation benches.**
3. **A larger apples-to-apples CPU vs GPU comparison point for KVDF validation and optimizer work.**
4. **Visibility into whether KVDF indicator prep actually used CUDA or silently fell back to CPU.**

## Recommended measurement sequence

### Phase A: baseline

Run before code changes:

- `gpu_native_expansion_opt` with `VECTORBT_GPU_NATIVE_BENCH_ONLY=kvdf`
- `composite_validation_workflow` existing KVDF groups

Capture:

- backend used
- backend route
- fallback reason
- current optimizer timings
- current validation timings

### Phase B: low-risk cleanup only

After implementing:

- output-id caching
- fixed-array params
- checked kernel casts
- GPU validation route asserts
- benchmark expansion

Measure again. This isolates the “cheap cleanup” gain.

### Phase C: `KvdfSignalsF32` migration

After migrating the GPU row-builder to consume the new `f32` prep surface, measure again. This is likely the first materially visible win.

### Phase D: close/lr residency

After adding `close_device_ptr` / `lr_device_ptr` support and switching the caller to provide resident inputs, measure again.

### Phase E: optional combo residency

Only do this if Phase D still leaves staging as a meaningful share of runtime.

## Existing gate alignment

Use the existing Criterion and validation harnesses first. Only add new instrumentation if the current harnesses still leave ambiguity after the above changes.

---

## Test review

## Current strengths

- `kvdf.rs` already has parameter validation tests
- `kvdf.rs` already has a desired-position behavior test
- `kvdf.rs` already has a GPU smoke test that exercises the indicator surfaces

## Main gaps

1. **No direct parity fixture for a new KVDF `f32` prep surface.**
2. **No explicit prep-fallback visibility test.**
3. **No explicit KVDF GPU-kernel blocker-path test in `kvdf_gpu_kernel.rs`.**
4. **No explicit validation benchmark route assertion in GPU validation benches.**

## Recommended additions

### `kvdf.rs`

- `compute_kvdf_signals_f32_matches_reference_cpu_within_tolerance`
- `compute_kvdf_signals_f32_records_cpu_fallback_reason_when_cuda_inputs_missing`

### `kvdf_gpu_kernel.rs`

- `kvdf_gpu_kernel_returns_empty_for_empty_combo_batch`
- `kvdf_gpu_kernel_rejects_execution_blockers_before_launch`
- `kvdf_gpu_kernel_accepts_device_close_without_host_slice` once the new interface exists

### benchmark files

- assert GPU backend in all GPU validation probe sites
- keep route metadata printing intact
- optionally print the new KVDF prep trace when available

---

## Suggested implementation order

## Phase 0: baseline and route truth

1. benchmark current KVDF optimizer path
2. benchmark current KVDF validation path
3. record route summaries and fallback reasons

## Phase 1: low-risk code cleanup

1. cache output ids in `kvdf.rs`
2. replace outer param `Vec`s with fixed arrays/slices
3. add checked integer conversions in `kvdf_gpu_kernel.rs`
4. add GPU validation route assertions in `composite_validation_workflow.rs`
5. add larger KVDF benchmark shapes

## Phase 2: add the new `KvdfSignalsF32` prep path

1. add `series_to_f32(...)`
2. add `KvdfSignalsF32`
3. add `KvdfPrepTraceEntry` / `KvdfPrepTrace`
4. add `compute_indicator_f32_for_gpu(...)`
5. add `compute_kvdf_signals_f32_for_gpu(...)`
6. switch upstream KVDF GPU row-builders to this surface
7. measure again

## Phase 3: add close/log-return residency

1. extend `KvdfGpuDeviceSignalMatrices`
2. add raw-pointer log-return launcher
3. resolve close/lr pointers once per call
4. switch upstream caller to provide resident `close_device_ptr` and `lr_device_ptr` when available
5. measure again

## Phase 4: add uninitialized host readback

1. verify the metrics kernel writes every output element
2. route `host_metrics` allocation through shared `alloc_policy`
3. re-run tests and benchmarks

## Phase 5: optional higher-risk work

1. direct `DeviceF32` prep path
2. family-level multi-output indicator helpers
3. prepared device combo batches
4. pinned host readback buffers if still measurable

---

## Practical notes on uninitialized memory usage

These are the safe opportunities visible from the provided files.

## Safe now or already present

- `DeviceBuffer::<f32>::uninitialized(signals.cols)` for `d_lr` is already the right shape **if** the log-return kernel writes every element.
- `DeviceBuffer::<f32>::uninitialized(out_len)` for `d_out` is already the right shape **if** the metrics kernel writes every output element.

## Safe after verification

- host `host_metrics` readback can use uninitialized memory if the destination helper is implemented through the shared `alloc_policy` surface and the kernel is confirmed to fully overwrite it

## Lower-priority optional pattern

`build_kvdf_desired_positions(...)` could theoretically use a prefix-init / loop-fill pattern instead of `vec![0i32; n]`, because every element is ultimately written exactly once if you explicitly fill the prefix and loop-fill the suffix. I would **not** prioritize that change for a CUDA-focused KVDF pass; it is much lower leverage than the prep and staging fixes above.

---

## Acceptance criteria

Treat the plan as successful when the following are all true:

1. **KVDF optimizer and validation GPU benches still route to GPU** with explicit assertions.
2. **The GPU-prepared KVDF signal path no longer widens HostF32 into Vec<f64>** for GPU-bound workflows.
3. **The KVDF scoring kernel can consume resident close/log-return inputs** without forced per-call host uploads.
4. **KVDF prep fallback reasons are visible** when CUDA indicator prep is unavailable or fails.
5. **Larger KVDF validation workload shapes exist** in the existing validation harness.
6. **All new uninitialized memory uses are centrally justified** by full-write guarantees.
7. **Parity tests pass** for the new `f32` prep surface against the existing CPU/reference path.

---

## Detailed implementation checklist

### Baseline and evidence

- [ ] Run `gpu_native_expansion_opt` for KVDF only and capture the current route summary.
- [ ] Run `composite_validation_workflow` and capture the current KVDF validation route summary.
- [ ] Save the current optimizer and validation timings before any edits.
- [ ] Confirm whether current KVDF GPU validation runs ever fall back to CPU.

### `kvdf.rs` low-risk cleanup

- [ ] Add a `KvdfOutputIds` cache using `OnceLock`.
- [ ] Replace repeated `resolve_output_id(...)` calls with cached ids.
- [ ] Replace outer parameter `Vec`s with fixed arrays/slices in all KVDF signal builders.
- [ ] Keep the existing `compute_kvdf_signals(...)` API intact for CPU/reference paths.

### `kvdf.rs` new GPU-prep path

- [ ] Add `series_to_f32(...)`.
- [ ] Add `KvdfSignalsF32`.
- [ ] Add `KvdfPrepTraceEntry`.
- [ ] Add `KvdfPrepTrace`.
- [ ] Add `compute_indicator_f32_for_gpu(...)`.
- [ ] Add `compute_kvdf_signals_f32_for_gpu(...)`.
- [ ] Ensure the new path records whether each indicator used CUDA or CPU.
- [ ] Ensure fallback reasons are preserved in the trace.
- [ ] Compute `first_valid` for the new `f32` surface.
- [ ] Add doc comments clarifying CPU/reference vs GPU-prep entrypoints.

### `kvdf.rs` tests

- [ ] Add parity test for `KvdfSignalsF32` vs the existing CPU/reference signal path.
- [ ] Add fallback-trace visibility test.
- [ ] Keep the existing smoke test working.
- [ ] Verify tolerances are documented rather than silently widened.

### `kvdf_gpu_kernel.rs` interface and staging cleanup

- [ ] Change `close_f32` from required host slice to `Option<&[f32]>`.
- [ ] Add `close_device_ptr` to `KvdfGpuDeviceSignalMatrices`.
- [ ] Add `lr_device_ptr` to `KvdfGpuDeviceSignalMatrices`.
- [ ] Update validation to accept either a host close slice or a device close pointer.
- [ ] Add a raw-pointer log-return launcher.
- [ ] Resolve close/lr pointers once before launching the metrics kernel.
- [ ] Skip host close upload when `close_device_ptr` is provided.
- [ ] Skip log-return recomputation when `lr_device_ptr` is provided.
- [ ] Replace unchecked `as i32` casts with `i32::try_from(...)`.
- [ ] Cache `device_caps` inside `KvdfCudaRuntime`.

### `kvdf_gpu_kernel.rs` uninitialized memory discipline

- [ ] Confirm that `compute_log_returns_f32` writes every element of `d_lr`.
- [ ] Confirm that `kvdf_backtest_metrics_f32` writes every output element of `d_out`.
- [ ] Route host metrics readback through shared `alloc_policy` helper instead of `vec![0.0; out_len]`.
- [ ] Avoid introducing new local unsafe allocation code if the shared helper exists or can be added centrally.

### `kvdf_gpu_kernel.rs` tests

- [ ] Add an empty-combo early-return test.
- [ ] Add a blocker-path test for overlays / stop-loss / take-profit.
- [ ] Add a validation-path test for device-close mode once the new interface exists.

### `gpu_native_expansion_opt.rs`

- [ ] Add a large CPU KVDF case matching the GPU large-case shape.
- [ ] Rename CPU labels so they match GPU pair-count naming.
- [ ] Keep the existing GPU route assertions intact.
- [ ] Print the KVDF prep trace in the route probe once that metadata exists.
- [ ] Re-run KVDF-only optimizer benchmarks after each phase.

### `composite_validation_workflow.rs`

- [ ] Import `BackendUsed`.
- [ ] Assert GPU backend usage in all GPU validation route probes.
- [ ] Add a KVDF-specific GPU workflow benchmark at 100k / 64 candidates.
- [ ] Add at least one larger KVDF profile benchmark, preferably 100k / 32 and optionally 100k / 64.
- [ ] Add a KVDF-shaped candle generator for KVDF-specific validation groups.
- [ ] Use KVDF-shaped candles in the new KVDF-specific validation groups.
- [ ] Print the KVDF prep trace in route probes once available.

### Upstream caller migration (adjacent file(s) not provided)

- [ ] Switch the KVDF GPU row-builder to consume `KvdfSignalsF32` instead of the `f64` signal surface.
- [ ] Provide `close_device_ptr` when the close series is already resident.
- [ ] Provide `lr_device_ptr` when log returns are already resident or precomputed.
- [ ] Re-measure optimizer and validation timings after the migration.

### Optional higher-risk follow-ups

- [ ] Investigate whether the dynamic indicator adapter can emit multi-output families in one call.
- [ ] If not, design family-level Keltner/FVG/DevStop helpers.
- [ ] Consider direct `DeviceF32` indicator outputs for KVDF row builders.
- [ ] Consider prepared device combo batches if staging remains measurable.
- [ ] Consider pinned host readback buffers only if DtoH copy still shows up after the bigger fixes.

### Final sign-off

- [ ] Confirm that all GPU validation benches assert actual GPU usage.
- [ ] Confirm that the new `f32` prep path passes parity tests.
- [ ] Confirm that no new uninitialized allocation is used without a documented full-write guarantee.
- [ ] Confirm that optimizer and validation timings improved at the intended boundary.
- [ ] Confirm that no fallback visibility was lost.
- [ ] Confirm that CPU/reference behavior remained intact.

