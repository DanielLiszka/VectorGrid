# VCET GPU-native optimization and validation implementation plan

## Scope

This document is a VCET-only implementation plan derived from the review guidance in `gpt54_feature_review_map.md`, specifically:

- **Bundle 5: Strategy-local GPU-native optimization and validation routes**
- **Bundle 16: Allocation policy, prefix initialization, and copy-elision discipline**
- The general review prompt sections:
  1. Surface understanding
  2. Findings
  3. Safe optimization opportunities
  4. Higher-risk optimization opportunities
  5. Benchmark and perf-gate review
  6. Test review
  7. Suggested improvement plan

This plan is intentionally limited to the files you provided:

- `vcet.rs`
- `vcet_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

It also uses one adjacent reference pattern from `gpu_backtest_kernel.rs` because the review map explicitly allows adjacent hot-path files when GPU buffer reuse and copy-elision are implicated.

## What this document is optimizing for

1. Reducing unnecessary host/device copy boundaries.
2. Eliminating avoidable zero-fill allocations.
3. Preventing host readback + re-upload “bounce” paths in GPU-native VCET setup.
4. Separating **cold upload cost** from **warm repeated evaluation cost**.
5. Preserving parity and route transparency while improving throughput.
6. Adding benchmark coverage for VCET so changes can be justified with measurement rather than guesswork.

## Executive summary

The most important VCET changes are **not** inside the kernel arithmetic first. The biggest wins in the provided files are in **setup-path staging**, **copy-elision**, and **benchmark coverage**.

### Highest-priority changes

1. **Stop the host-output-to-device bounce in `compute_vcet_vidya_signal_cuda_device`.**  
   If `DeviceF32` output is unavailable, do not silently accept `HostF32` and re-upload it. That path is functionally convenient but performance-hostile for a GPU-native route.

2. **Add a reusable uploaded VCET batch in `vcet_gpu_kernel.rs`.**  
   Right now the hot path re-uploads:
   - close prices
   - row-pointer arrays
   - combo arrays  
   and re-computes log returns on every call.  
   A two-step `upload -> eval` API is the biggest direct way to cut redundant H2D traffic.

3. **Use shared uninitialized host readback for `host_metrics`.**  
   `d_out.copy_to(...)` fully overwrites the destination, so `vec![0.0; out_len]` is pure waste.

4. **Remove duplicate Chandelier dispatch in `compute_vcet_chandelier_signals`.**  
   The CPU/reference path currently resolves the same indicator twice, once for `long_stop` and once for `short_stop`.

5. **Add VCET-specific optimizer and validation benchmarks.**  
   The provided benchmark files currently cover other strategies, but not VCET. Without that coverage, you cannot honestly measure whether VCET should be GPU-preferred on current workloads.

---

## 1. Surface understanding

### Current VCET surface split

The current VCET implementation is split across two distinct surfaces:

#### `vcet.rs`
This file owns:

- parameter defaults and validation
- fast/slow period mapping into VIDYA + Chandelier periods
- CPU/reference indicator acquisition via dynamic indicator dispatch
- GPU-oriented indicator-device preparation helpers
- reference signal logic via `build_vcet_desired_positions`

Important functions:

- `compute_indicator_f64` (`vcet.rs`, ~363-405)
- `compute_vcet_vidya_signals` (`vcet.rs`, ~429-471)
- `compute_vcet_chandelier_signals` (`vcet.rs`, ~473-534)
- `compute_vcet_vidya_signal_cuda_device` (`vcet.rs`, ~537-588)
- `compute_vcet_chandelier_signal_cuda_device` (`vcet.rs`, ~591-618)
- `build_vcet_desired_positions` (`vcet.rs`, ~641-740)

#### `vcet_gpu_kernel.rs`
This file owns the strategy-local GPU metrics scoring route:

- runtime/module/stream setup
- kernel launch for log-return generation
- kernel launch for `vcet_backtest_metrics_f32`
- host readback and decode of metrics rows

Important functions:

- `launch_compute_log_returns_f32` (`vcet_gpu_kernel.rs`, ~113-160)
- `eval_vcet_metrics_gpu_kernel_device_rows` (`vcet_gpu_kernel.rs`, ~166-314)

#### Bench files
The two provided bench files are the default evidence path the map asks us to use before inventing new measurement surfaces:

- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

At the moment, both are useful as patterns but neither currently measures VCET.

### Current hot-path shape

A typical GPU-native VCET evaluation currently looks like:

1. Build or fetch VCET indicator outputs.
2. Materialize row-pointer arrays on host.
3. In `eval_vcet_metrics_gpu_kernel_device_rows`:
   - upload `close_f32`
   - allocate `d_lr`
   - compute log returns on device
   - upload row-pointer arrays
   - upload combo arrays
   - allocate `d_out`
   - launch kernel
   - synchronize
   - read full output buffer back to host
   - decode metrics on CPU

That is functional, but it is not yet optimized for repeated evaluation on the same candle set and candidate family.

### Timing boundaries used in this plan

To stay aligned with the map, every performance recommendation is framed against one of these timing boundaries:

- **app-path**: actual runtime work in the VCET execution path
- **Criterion**: benchmark harness timing inside the provided Criterion files
- **validation-cost harness**: validation workflow cost inside `composite_validation_workflow.rs`
- **route-matrix / route-summary evidence**: whether route probes and fallback summaries expose the right execution route
- **profiling-only**: good idea but not confirmed from the provided files alone

---

## 2. Findings

This section follows the map’s requested format.

### Confirmed issue 1
- **Severity:** high
- **Type:** performance / fallback-transparency
- **Location:** `vcet.rs` → `compute_indicator_f64`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  When backend is not CPU-only, `compute_indicator_f64` tries CUDA with `HostF32`. If CUDA succeeds, it still pays a host materialization plus `f32 -> f64` conversion. If CUDA fails, it silently falls back to CPU indicator computation. This means a GPU-preferred call path can degrade to CPU work without any visibility in this function.
- **Recommended change:**  
  Keep this function for CPU/reference surfaces, but add an explicit route mode:
  - permissive: CUDA host output allowed, CPU fallback allowed
  - strict: CUDA required or error  
  Do **not** use this helper for GPU-native staging if the end result must remain device-resident.

### Confirmed issue 2
- **Severity:** high
- **Type:** performance
- **Location:** `vcet.rs` → `compute_vcet_chandelier_signals`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  The function resolves Chandelier metadata once, but then runs `compute_indicator_f64` twice: once for `long_stop`, once for `short_stop`. That is duplicate indicator dispatch for a logically paired output.
- **Recommended change:**  
  Introduce a paired-output path so both stops are retrieved together where possible. At minimum, centralize shared metadata and treat the pair as one logical Chandelier evaluation surface.

### Confirmed issue 3
- **Severity:** high
- **Type:** performance / route-correctness
- **Location:** `vcet.rs` → `compute_vcet_vidya_signal_cuda_device`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  The current code requests `DeviceF32`. If that fails, it retries as `HostF32`, then converts that host result back to a device buffer via `VcetOwnedDeviceMatrix::from_host_row`. That is the worst of both worlds for a GPU-native path: host materialization plus re-upload.
- **Recommended change:**  
  Split the behavior:
  - `compute_vcet_vidya_signal_cuda_device_strict`: device output required, otherwise return error
  - optional permissive helper for non-hot-path tooling  
  The strict helper should be the default for GPU-native VCET evaluation.

### Confirmed issue 4
- **Severity:** high
- **Type:** performance
- **Location:** `vcet_gpu_kernel.rs` → `eval_vcet_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  Every call performs fresh uploads for:
  - `close_f32`
  - `vidya_rows_ptr`
  - `ce_long_rows_ptr`
  - `ce_short_rows_ptr`
  - `combo` arrays  
  and fresh allocation for:
  - `d_lr`
  - `d_out`  
  and then recomputes log returns. This is the clearest repeated staging waste in the provided VCET GPU path.
- **Recommended change:**  
  Add a reusable uploaded-batch API so repeated evaluation can reuse resident device buffers instead of rebuilding everything every time.

### Confirmed issue 5
- **Severity:** medium
- **Type:** performance / copy-elision
- **Location:** `vcet_gpu_kernel.rs` → `eval_vcet_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  `host_metrics` is allocated as `vec![0.0f32; out_len]`, then `d_out.copy_to(...)` fully overwrites it. The zero-fill is wasted work.
- **Recommended change:**  
  Replace it with the shared uninitialized host allocation helper already used in adjacent GPU hot paths.

### Confirmed issue 6
- **Severity:** medium
- **Type:** performance
- **Location:** `vcet.rs` → `compute_vcet_vidya_signals`, `compute_vcet_chandelier_signals`, `compute_vcet_vidya_signal_cuda_device`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  Fixed-size parameter lists are built with `Vec<IndicatorParamKV>` even though each call has only 3 entries. This adds a heap allocation in repeated setup paths.
- **Recommended change:**  
  Replace those `Vec` allocations with fixed-size arrays and pass slices of those arrays.

### Confirmed issue 7
- **Severity:** medium
- **Type:** performance / maintainability
- **Location:** `vcet.rs` → `resolve_output_id`, `resolve_param_key`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  VCET repeatedly scans descriptor metadata and allocates temporary lowercase vectors even though VIDYA and Chandelier metadata are effectively static within a process lifetime.
- **Recommended change:**  
  Cache resolved metadata using `OnceLock<Result<..., String>>` so lookup cost is paid once per process.

### Likely issue 8
- **Severity:** medium
- **Type:** performance
- **Location:** `vcet.rs` → `compute_vcet_chandelier_signal_cuda_device`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  `CudaChandelierExit::new(device_id as usize)` is called on each invocation. The provided files do not show whether this constructor is cheap or heavy. If it loads modules or allocates internal state, it is avoidable repeated setup.
- **Recommended change:**  
  Add a thread-local per-device handle cache if constructor cost is non-trivial.

### Likely issue 9
- **Severity:** medium
- **Type:** performance
- **Location:** `vcet_gpu_kernel.rs` → `launch_compute_log_returns_f32`, `eval_vcet_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  `Device::get_device(device_id)`, `detect_device_launch_caps`, and `module.get_function(...)` happen in the hot path. The staging waste is bigger today, but once uploads are reduced this repeated launch-shape discovery will matter more.
- **Recommended change:**  
  Defer until after batch upload reuse. Then cache stable launch metadata or resolved kernels inside `VcetCudaRuntime` if the `cust` API makes that safe and ergonomic.

### Open question 10
- **Severity:** medium
- **Type:** performance
- **Location:** `vcet_gpu_kernel.rs` signal row-pointer interface
- **Timing boundary affected:** app-path
- **Why it matters:**  
  The kernel currently receives row-pointer arrays (`u64` device addresses). If all indicator rows are always contiguous inside one matrix, a base pointer + stride would be cheaper than uploading pointer tables. The provided files do not prove that contiguity is always available because `VcetOwnedDeviceMatrix` can come from multiple ownership forms.
- **Recommended change:**  
  Keep the pointer-table interface for now. Revisit only if later profiling shows pointer-table upload or pointer-chasing is material.

### Confirmed issue 11
- **Severity:** high
- **Type:** benchmark-fidelity
- **Location:** `gpu_native_expansion_opt.rs`
- **Timing boundary affected:** Criterion
- **Why it matters:**  
  This optimizer benchmark file currently includes KQSR, CARM, KVDF, and CDMC, but no VCET path.
- **Recommended change:**  
  Add CPU and GPU VCET benchmark entries using the same route-probe and Criterion-group pattern.

### Confirmed issue 12
- **Severity:** high
- **Type:** benchmark-fidelity
- **Location:** `composite_validation_workflow.rs`
- **Timing boundary affected:** validation-cost harness
- **Why it matters:**  
  This validation benchmark file currently profiles base composite, ASR, VCC, VEMR, ROCB, MKCS, KQSR, CARM, KVDF, and CDMC, but no VCET.
- **Recommended change:**  
  Add VCET candidate generation plus CPU and GPU validation profile entries. Add one larger dedicated GPU VCET workflow group as well.

### Confirmed issue 13
- **Severity:** medium
- **Type:** testing / parity
- **Location:** `vcet.rs` tests and `vcet_gpu_kernel.rs` tests
- **Timing boundary affected:** route-matrix / app-path guard safety
- **Why it matters:**  
  Current VCET tests are narrow. They do not cover:
  - threshold equality behavior
  - `trade_on_next_bar` start-index behavior
  - long-only no-flip behavior
  - input validation mismatches in the GPU kernel path
  - overlay blocker behavior
- **Recommended change:**  
  Add those tests before larger refactors.

---

## 3. Safe optimization opportunities

These are the changes I would implement first because they are the most parity-safe and give the best chance of measurable improvement.

### Safe opportunity A — replace heap-allocated param vectors with stack arrays

This is small, safe, and local.

#### Current pattern
```rust
let params = vec![
    IndicatorParamKV { ... },
    IndicatorParamKV { ... },
    IndicatorParamKV { ... },
];
```

#### Recommended pattern
```rust
let params = [
    IndicatorParamKV {
        key: short_key,
        value: IndicatorParamValue::Int(i64::from(periods.vidya_short)),
    },
    IndicatorParamKV {
        key: long_key,
        value: IndicatorParamValue::Int(i64::from(periods.vidya_long)),
    },
    IndicatorParamKV {
        key: alpha_key,
        value: IndicatorParamValue::Float(alpha),
    },
];
```

Use `params.as_slice()` exactly as today.

**Why first:** zero behavior change, minimal code churn, removes tiny repeated heap allocations.

---

### Safe opportunity B — switch host readback to shared uninitialized allocation

Use the same shared helper pattern already present in adjacent GPU code.

#### Current code in `vcet_gpu_kernel.rs`
```rust
let mut host_metrics = vec![0.0f32; out_len];
d_out
    .copy_to(host_metrics.as_mut_slice())
    .map_err(|e| e.to_string())?;
```

#### Recommended edit
```rust
use crate::alloc_policy::alloc_uninit_copy;

// ...

let mut host_metrics: Vec<f32> = alloc_uninit_copy(out_len);
d_out
    .copy_to(host_metrics.as_mut_slice())
    .map_err(|e| e.to_string())?;
```

**Why safe:** every element is written by `copy_to(...)` before any read.

---

### Safe opportunity C — cache descriptor metadata

This is a small, parity-safe CPU-path cleanup.

#### Recommended helper types for `vcet.rs`
```rust
use std::sync::OnceLock;

#[derive(Clone)]
struct VcetVidyaMeta {
    short_key: String,
    long_key: String,
    alpha_key: String,
    output_id: String,
}

#[derive(Clone)]
struct VcetChandelierMeta {
    period_key: String,
    mult_key: String,
    use_close_key: String,
    long_output: String,
    short_output: String,
}

fn vcet_vidya_meta() -> Result<&'static VcetVidyaMeta, String> {
    static META: OnceLock<Result<VcetVidyaMeta, String>> = OnceLock::new();
    match META.get_or_init(|| {
        Ok(VcetVidyaMeta {
            short_key: resolve_param_key("vidya", &["short_period", "short", "fast"])?,
            long_key: resolve_param_key("vidya", &["long_period", "long", "slow"])?,
            alpha_key: resolve_param_key("vidya", &["alpha"])?,
            output_id: resolve_output_id("vidya", &["values", "value", "vidya"])?,
        })
    }) {
        Ok(meta) => Ok(meta),
        Err(err) => Err(err.clone()),
    }
}

fn vcet_chandelier_meta() -> Result<&'static VcetChandelierMeta, String> {
    static META: OnceLock<Result<VcetChandelierMeta, String>> = OnceLock::new();
    match META.get_or_init(|| {
        Ok(VcetChandelierMeta {
            period_key: resolve_param_key("chandelier_exit", &["period", "length", "timeperiod"])?,
            mult_key: resolve_param_key("chandelier_exit", &["mult", "multiplier"])?,
            use_close_key: resolve_param_key("chandelier_exit", &["use_close", "close"])?,
            long_output: resolve_output_id("chandelier_exit", &["long_stop", "long"])?,
            short_output: resolve_output_id("chandelier_exit", &["short_stop", "short"])?,
        })
    }) {
        Ok(meta) => Ok(meta),
        Err(err) => Err(err.clone()),
    }
}
```

#### Then in `compute_vcet_vidya_signals`
```rust
let meta = vcet_vidya_meta()?;
let params = [
    IndicatorParamKV {
        key: meta.short_key.clone(),
        value: IndicatorParamValue::Int(i64::from(periods.vidya_short)),
    },
    IndicatorParamKV {
        key: meta.long_key.clone(),
        value: IndicatorParamValue::Int(i64::from(periods.vidya_long)),
    },
    IndicatorParamKV {
        key: meta.alpha_key.clone(),
        value: IndicatorParamValue::Float(alpha),
    },
];

let values = compute_indicator_f64(
    candles,
    ma_source,
    backend,
    "vidya",
    meta.output_id.as_str(),
    params.as_slice(),
    #[cfg(feature = "cuda")]
    prepared_cuda_inputs,
)?;
```

Same idea for `compute_vcet_chandelier_signals` and `compute_vcet_vidya_signal_cuda_device`.

**Why safe:** metadata is static for process lifetime; caching only removes repeated lookup and temporary allocation.

---

### Safe opportunity D — add strict device-only VIDYA helper

This removes the host bounce from the GPU-native path without changing the permissive helper used elsewhere.

#### Recommended edit in `vcet.rs`
```rust
#[cfg(feature = "cuda")]
pub fn compute_vcet_vidya_signal_cuda_device_strict(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    periods: VcetResolvedPeriods,
    alpha: f64,
) -> Result<VcetOwnedDeviceMatrix, String> {
    let meta = vcet_vidya_meta()?;
    let params = [
        IndicatorParamKV {
            key: meta.short_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.vidya_short)),
        },
        IndicatorParamKV {
            key: meta.long_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.vidya_long)),
        },
        IndicatorParamKV {
            key: meta.alpha_key.clone(),
            value: IndicatorParamValue::Float(alpha),
        },
    ];

    let descriptor = find_dynamic_indicator_descriptor("vidya").ok_or_else(|| {
        "VCET indicator 'vidya' is not registered for dynamic dispatch".to_string()
    })?;

    let out = compute_dynamic_indicator_cuda_direct_with_inputs(
        prepared_inputs,
        "vidya",
        Some(meta.output_id.as_str()),
        descriptor.input_kind,
        params.as_slice(),
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| format!("VCET VIDYA DeviceF32 output unavailable: {e}"))?;

    cuda_series_to_device_f32(out)
}
```

#### Then preserve the old helper only if you still need a permissive path
```rust
#[cfg(feature = "cuda")]
pub fn compute_vcet_vidya_signal_cuda_device_permissive(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    periods: VcetResolvedPeriods,
    alpha: f64,
) -> Result<VcetOwnedDeviceMatrix, String> {
    if let Ok(v) = compute_vcet_vidya_signal_cuda_device_strict(prepared_inputs, periods, alpha) {
        return Ok(v);
    }

    let meta = vcet_vidya_meta()?;
    let params = [
        IndicatorParamKV {
            key: meta.short_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.vidya_short)),
        },
        IndicatorParamKV {
            key: meta.long_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.vidya_long)),
        },
        IndicatorParamKV {
            key: meta.alpha_key.clone(),
            value: IndicatorParamValue::Float(alpha),
        },
    ];
    let descriptor = find_dynamic_indicator_descriptor("vidya").ok_or_else(|| {
        "VCET indicator 'vidya' is not registered for dynamic dispatch".to_string()
    })?;

    let host_out = compute_dynamic_indicator_cuda_direct_with_inputs(
        prepared_inputs,
        "vidya",
        Some(meta.output_id.as_str()),
        descriptor.input_kind,
        params.as_slice(),
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
    )
    .map_err(|e| e.to_string())?;

    cuda_series_to_host_device_f32(host_out)
}
```

**Recommendation:** use the strict helper in the GPU-native hot path.

---

### Safe opportunity E — add a reusable uploaded VCET batch

This is the most important strategy-local hot-path change.

The best structure is to mirror the adjacent `upload -> eval` pattern used in the Double-MA GPU program path rather than trying to cram cache signatures into the existing wrapper immediately.

#### Recommended new types in `vcet_gpu_kernel.rs`
```rust
use crate::alloc_policy::alloc_uninit_copy;

pub struct UploadedVcetBatch {
    device_id: u32,
    cols: usize,
    combo_count: usize,
    d_close: DeviceBuffer<f32>,
    d_lr: DeviceBuffer<f32>,
    d_vidya_rows: DeviceBuffer<u64>,
    d_ce_long_rows: DeviceBuffer<u64>,
    d_ce_short_rows: DeviceBuffer<u64>,
    d_combo_vidya: DeviceBuffer<i32>,
    d_combo_ce: DeviceBuffer<i32>,
    d_combo_start: DeviceBuffer<i32>,
    d_combo_end: DeviceBuffer<i32>,
    d_out: DeviceBuffer<f32>,
}

impl UploadedVcetBatch {
    #[inline]
    pub fn combo_count(&self) -> usize {
        self.combo_count
    }

    #[inline]
    fn out_len(&self) -> usize {
        self.combo_count * VCET_METRICS_STRIDE
    }
}
```

#### Upload helper
```rust
pub fn upload_vcet_batch(
    device_id: u32,
    signals: &VcetGpuDeviceSignalMatrices<'_>,
    combos: &VcetGpuComboBatch<'_>,
) -> Result<UploadedVcetBatch, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.vidya_row_index.len();
    if combo_count == 0 {
        return Err("VCET uploaded batch requires at least one combo".to_string());
    }

    VCET_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, VcetCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "VCET GPU runtime missing".to_string())?;
        rt.set_current()?;

        let d_close = DeviceBuffer::from_slice(signals.close_f32).map_err(|e| e.to_string())?;
        let mut d_lr =
            unsafe { DeviceBuffer::<f32>::uninitialized(signals.cols) }.map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, device_id, &d_close, &mut d_lr, signals.cols)?;

        let d_vidya_rows =
            DeviceBuffer::from_slice(signals.vidya_rows_ptr).map_err(|e| e.to_string())?;
        let d_ce_long_rows =
            DeviceBuffer::from_slice(signals.ce_long_rows_ptr).map_err(|e| e.to_string())?;
        let d_ce_short_rows =
            DeviceBuffer::from_slice(signals.ce_short_rows_ptr).map_err(|e| e.to_string())?;

        let d_combo_vidya =
            DeviceBuffer::from_slice(combos.vidya_row_index).map_err(|e| e.to_string())?;
        let d_combo_ce =
            DeviceBuffer::from_slice(combos.ce_row_index).map_err(|e| e.to_string())?;
        let d_combo_start =
            DeviceBuffer::from_slice(combos.start_t).map_err(|e| e.to_string())?;
        let d_combo_end =
            DeviceBuffer::from_slice(combos.end_t).map_err(|e| e.to_string())?;

        let out_len = combo_count
            .checked_mul(VCET_METRICS_STRIDE)
            .ok_or_else(|| "VCET GPU metrics buffer overflow".to_string())?;
        let d_out =
            unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }.map_err(|e| e.to_string())?;

        Ok(UploadedVcetBatch {
            device_id,
            cols: signals.cols,
            combo_count,
            d_close,
            d_lr,
            d_vidya_rows,
            d_ce_long_rows,
            d_ce_short_rows,
            d_combo_vidya,
            d_combo_ce,
            d_combo_start,
            d_combo_end,
            d_out,
        })
    })
}
```

#### Eval helper
```rust
pub fn eval_uploaded_vcet_metrics_gpu_kernel(
    uploaded: &mut UploadedVcetBatch,
    strategy: &StrategyConfig,
    vcet: &VcetStrategyParams,
) -> Result<Vec<Metrics>, String> {
    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "VCET GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    VCET_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        let rt = runtimes
            .get_mut(&uploaded.device_id)
            .ok_or_else(|| "VCET GPU runtime missing".to_string())?;
        rt.set_current()?;

        let kernel = rt
            .module
            .get_function("vcet_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let device = Device::get_device(uploaded.device_id).map_err(|e| e.to_string())?;
        let device_caps = detect_device_launch_caps(device);
        let launch = choose_launch_shape_1d_from_candidates(
            uploaded.combo_count,
            device_caps,
            detect_kernel_launch_caps(&kernel, device_caps),
            &[32u32, 64, 128, 256],
        );

        let mut flags: u32 = 0;
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

        let commission = strategy.effective_commission().clamp(0.0, 0.95) as f32;

        let mut close_ptr = uploaded.d_close.as_device_ptr().as_raw();
        let mut lr_ptr = uploaded.d_lr.as_device_ptr().as_raw();
        let mut t_i = uploaded.cols as i32;
        let mut vidya_rows_ptr = uploaded.d_vidya_rows.as_device_ptr().as_raw();
        let mut ce_long_rows_ptr = uploaded.d_ce_long_rows.as_device_ptr().as_raw();
        let mut ce_short_rows_ptr = uploaded.d_ce_short_rows.as_device_ptr().as_raw();
        let mut vidya_rows_i = uploaded.d_vidya_rows.len() as i32;
        let mut ce_rows_i = uploaded.d_ce_long_rows.len() as i32;
        let mut combo_vidya_ptr = uploaded.d_combo_vidya.as_device_ptr().as_raw();
        let mut combo_ce_ptr = uploaded.d_combo_ce.as_device_ptr().as_raw();
        let mut combo_start_ptr = uploaded.d_combo_start.as_device_ptr().as_raw();
        let mut combo_end_ptr = uploaded.d_combo_end.as_device_ptr().as_raw();
        let mut combo_count_i = uploaded.combo_count as i32;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut require_vidya_slope_i = if vcet.require_vidya_slope { 1i32 } else { 0i32 };
        let mut max_trade_bars_i = i32::from(vcet.max_trade_bars.max(1));
        let mut out_stride_i = VCET_METRICS_STRIDE as i32;
        let mut out_ptr = uploaded.d_out.as_device_ptr().as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut vidya_rows_ptr as *mut _ as *mut c_void,
                &mut ce_long_rows_ptr as *mut _ as *mut c_void,
                &mut ce_short_rows_ptr as *mut _ as *mut c_void,
                &mut vidya_rows_i as *mut _ as *mut c_void,
                &mut ce_rows_i as *mut _ as *mut c_void,
                &mut combo_vidya_ptr as *mut _ as *mut c_void,
                &mut combo_ce_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut require_vidya_slope_i as *mut _ as *mut c_void,
                &mut max_trade_bars_i as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
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

        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut host_metrics: Vec<f32> = alloc_uninit_copy(uploaded.out_len());
        uploaded
            .d_out
            .copy_to(host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out = Vec::with_capacity(uploaded.combo_count);
        for row in host_metrics.chunks_exact(VCET_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}
```

#### Preserve the current convenience wrapper
```rust
pub fn eval_vcet_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &VcetGpuDeviceSignalMatrices<'_>,
    combos: &VcetGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcet: &VcetStrategyParams,
) -> Result<Vec<Metrics>, String> {
    let mut uploaded = upload_vcet_batch(device_id, signals, combos)?;
    eval_uploaded_vcet_metrics_gpu_kernel(&mut uploaded, strategy, vcet)
}
```

### Why this is the right shape

This gives you three benefits immediately:

1. **You keep backward compatibility** with the existing one-shot API.
2. **Callers that repeat evaluation can reuse uploads** explicitly.
3. **You separate upload cost from eval cost**, which makes both app-path optimization and benchmark attribution much cleaner.

---

## 4. Higher-risk optimization opportunities

These are good ideas, but I would not implement them before the safe items above.

### Higher-risk opportunity A — per-device cached Chandelier handle

If `CudaChandelierExit::new(...)` is expensive, cache it.

#### Optional helper in `vcet.rs`
```rust
#[cfg(feature = "cuda")]
fn with_vcet_chandelier<T>(
    device_id: u32,
    f: impl FnOnce(&mut CudaChandelierExit) -> Result<T, String>,
) -> Result<T, String> {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static VCET_CE_HANDLES: RefCell<HashMap<u32, CudaChandelierExit>> =
            RefCell::new(HashMap::new());
    }

    VCET_CE_HANDLES.with(|cell| {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(
                device_id,
                CudaChandelierExit::new(device_id as usize).map_err(|e| e.to_string())?,
            );
        }
        let cuda = map.get_mut(&device_id).expect("just inserted");
        f(cuda)
    })
}
```

Then use it inside `compute_vcet_chandelier_signal_cuda_device`.

**Why higher-risk:** depends on the actual semantics and lifetime behavior of `CudaChandelierExit`.

---

### Higher-risk opportunity B — allow device-resident close input

If an upstream caller already has close prices resident on device, the VCET kernel path should be able to consume them directly instead of uploading `signals.close_f32` each time.

Potential future shape:
```rust
pub enum VcetCloseInput<'a> {
    Host(&'a [f32]),
    // Replace with the actual repo-wide device-slice type if one already exists.
    DevicePtr { ptr: u64, len: usize },
}
```

**Why higher-risk:** the required device-slice type is not visible in the provided files, so this should wait until the broader strategy-local call surface is in view.

---

### Higher-risk opportunity C — base pointer + stride instead of row-pointer tables

If all VCET signal rows are contiguous inside shared device matrices, you could replace row-pointer uploads with base pointer + row stride.

**Why higher-risk:** the provided files do not prove that all rows come from one contiguous owner; `VcetOwnedDeviceMatrix` explicitly supports multiple ownership forms.

---

### Higher-risk opportunity D — device-side shortlist/reduction

Today the kernel writes full metrics for every combo and the CPU decodes all of them. If the downstream consumer only needs top-k or filtered candidates, a device-side reduction could shrink readback.

**Why higher-risk:** this changes the data contract and may complicate parity debugging. Do it only after the simpler upload/readback waste is fixed.

---

## 5. Benchmark and perf-gate review

The review map is explicit: use existing benchmark and gate surfaces before inventing new instrumentation.

### Current benchmark evidence gap

The provided benchmark files do **not** currently benchmark VCET:

- `gpu_native_expansion_opt.rs` covers KQSR, CARM, KVDF, and CDMC.
- `composite_validation_workflow.rs` covers base composite plus ASR, VCC, VEMR, ROCB, MKCS, KQSR, CARM, KVDF, and CDMC.

That means there is currently **no in-file Criterion evidence** for:

- VCET optimizer throughput
- VCET GPU route usage
- VCET validation throughput
- VCET fallback behavior in validation

### What to add in `gpu_native_expansion_opt.rs`

> **Dependency note:** the snippets below assume `CompositeCpuRequest` already exposes `vcet: Option<VcetStrategyParams>` in the same way it already exposes `kqsr`, `carm`, `kvdf`, and `cdmc`.  
> If that field does not exist yet, that broader wiring is a prerequisite outside the provided files.

#### Step 1 — imports
Add:
```rust
use ta_app_core::VcetStrategyParams;
```

If `CompositeCpuRequest` has a `vcet` field, also add it to `base_req` alongside the other strategy option fields:
```rust
fn base_req(backend: Backend) -> CompositeCpuRequest {
    CompositeCpuRequest {
        backend,
        asr: None,
        vcc: None,
        vemr: None,
        rocb: None,
        mkcs: None,
        kqsr: None,
        carm: None,
        kvdf: None,
        cdmc: None,
        vcet: None,
        // ... rest unchanged
    }
}
```

#### Step 2 — add a VCET-specific candle generator
Use a series that actually forces trend shifts and stop behavior rather than a flat or purely oscillatory pattern.

```rust
fn make_vcet_trend_reversal_candles(len: usize) -> Candles {
    let len = len.max(320);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let px = if i < 80 {
            100.0 + i as f64 * 0.10 + (i as f64 * 0.11).sin() * 0.4
        } else if i < 160 {
            108.0 + (i - 80) as f64 * 0.18 + ((i - 80) as f64 * 0.15).cos() * 0.8
        } else if i < 240 {
            122.0 - (i - 160) as f64 * 0.22 + ((i - 160) as f64 * 0.21).sin() * 1.0
        } else {
            104.0 + (i - 240) as f64 * 0.12 + ((i - 240) as f64 * 0.09).cos() * 0.7
        };

        open.push(px * 0.9985);
        high.push(px * 1.0050);
        low.push(px * 0.9950);
        close.push(px);
        vol.push(1_300.0 + (i % 23) as f64 * 37.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

#### Step 3 — add VCET request builder
```rust
fn make_vcet_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.vcet = Some(VcetStrategyParams {
        require_vidya_slope: true,
        max_trade_bars: 96,
        ..VcetStrategyParams::default()
    });
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}
```

#### Step 4 — add CPU and GPU benchmark entries
```rust
fn bench_vcet_cpu(c: &mut Criterion) {
    if !bench_target_enabled("vcet") {
        return;
    }
    let candles = make_vcet_trend_reversal_candles(200_000);
    bench_cpu_cases(c, "vcet/cpu/200k", &candles, make_vcet_req);
}

fn bench_vcet_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("vcet") {
        return;
    }
    let candles = make_vcet_trend_reversal_candles(200_000);
    bench_gpu_cases(
        c,
        "vcet/gpu_kernel/200k",
        "vcet/gpu_kernel/200k",
        &candles,
        make_vcet_req,
    );
}
```

#### Step 5 — add to `criterion_group!`
```rust
criterion_group!(
    gpu_native_expansion_benches,
    bench_kqsr_cpu,
    bench_kqsr_gpu_kernel,
    bench_carm_cpu,
    bench_carm_gpu_kernel,
    bench_kvdf_cpu,
    bench_kvdf_gpu_kernel,
    bench_cdmc_cpu,
    bench_cdmc_gpu_kernel,
    bench_vcet_cpu,
    bench_vcet_gpu_kernel
);
```

### What to add in `composite_validation_workflow.rs`

> **Dependency note:** the snippets below assume `CompositeParams` already exposes `vcet: Option<VcetStrategyParams>`.  
> If that field does not exist yet, you need broader composite-strategy plumbing first.

#### Step 1 — imports
Add:
```rust
use ta_app_core::VcetStrategyParams;
```

#### Step 2 — update all `CompositeParams` constructors
If `CompositeParams` now includes a `vcet` field, every constructor in this file must add `vcet: None` except the VCET builder below.

For example:
```rust
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
    vcc: None,
    vemr: None,
    rocb: None,
    mkcs: None,
    kqsr: None,
    carm: None,
    kvdf: None,
    cdmc: None,
    vcet: None,
    macz: ta_app_core::composite_cpu::MaczGateParams::default(),
    ott: ta_app_core::composite_cpu::OttGateParams::default(),
    signal_mode: ta_app_core::CompositeSignalMode::HardGate,
    strategy: StrategyConfig::default(),
});
```

#### Step 3 — add VCET candidates
```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_vcet_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 10);
        let slow_len = fast_len.saturating_add(26);

        let mut vcet = VcetStrategyParams::default();
        vcet.require_vidya_slope = true;
        vcet.max_trade_bars = 72u16.saturating_add(i_u16 % 24);

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
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: Some(vcet),
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}
```

#### Step 4 — add CPU and GPU profile entries
Add VCET to the CPU profile list:
```rust
bench_cpu_profile("vcet", 50_000, make_vcet_candidates(8));
bench_cpu_profile("vcet", 100_000, make_vcet_candidates(32));
```

Add VCET to the GPU profile list:
```rust
bench_profile("vcet", 50_000, make_vcet_candidates(8));
bench_profile("vcet", 100_000, make_vcet_candidates(32));
```

#### Step 5 — add one dedicated large GPU VCET validation group
The ASR-specific large GPU workflow group is a good pattern. Do the same for VCET so you get strategy-specific validation-cost evidence at a meaningful size.

```rust
let vcet_candidates = make_vcet_candidates(64);
let mut vcet_group =
    c.benchmark_group("composite_validate/gpu_kernel/vcet/100k/64candidates");
vcet_group.measurement_time(Duration::from_secs(8));
vcet_group.sample_size(10);

let vcet_base = CompositeValidationRequest {
    data_id: "bench-gpu-vcet".to_string(),
    backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
    ma_source: "close".to_string(),
    fast_ma_params: None,
    slow_ma_params: None,
    candidates: vcet_candidates.clone(),
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
    max_candidates: vcet_candidates.len(),
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
    vcet_base.clone(),
    &candles,
    &cancel,
    None,
)
.expect("gpu vcet validation route probe");

println!(
    "benchmark route composite_validate/gpu_kernel/vcet/100k/64candidates: backend_used={:?} backend_route={} fallback={}",
    route_probe.backend_used,
    route_probe.backend_route.as_deref().unwrap_or("<none>"),
    route_probe
        .backend_fallback_reason
        .as_deref()
        .unwrap_or("<none>")
);

vcet_group.bench_function("wf_cscv", |b| {
    b.iter(|| {
        let _ = ta_app_core::run_composite_validation_blocking_with_candles(
            vcet_base.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu vcet wf+cscv validate");
    })
});

vcet_group.finish();
```

### Benchmark acceptance criteria

After the benchmark edits above, you should have evidence for:

1. **VCET optimizer CPU vs GPU path**
2. **VCET validation CPU vs GPU path**
3. **Route probe evidence** for backend used / fallback reason
4. **Warm repeated app-path behavior** after upload reuse is introduced

---

## 6. Test review

The current tests in `vcet.rs` are useful but not sufficient for a refactor that touches GPU staging and signal equality behavior.

### Add these tests to `vcet.rs`

#### Test 1 — `compute_vcet_start_t` with `trade_on_next_bar`
```rust
#[test]
fn compute_vcet_start_t_honors_trade_on_next_bar() {
    let base = compute_vcet_start_t(2, 3, 1, 4, 5, false);
    let shifted = compute_vcet_start_t(2, 3, 1, 4, 5, true);
    assert_eq!(base, 5);
    assert_eq!(shifted, 6);
}
```

#### Test 2 — equality semantics matter
This guards the `>=` and `<=` logic that can become sensitive to `f32` rounding on the GPU path.

```rust
#[test]
fn build_vcet_desired_positions_treats_equal_thresholds_as_valid_signal() {
    let close = vec![10.0, 10.0, 10.0, 10.0];
    let vidya = VcetVidyaSignals {
        values: vec![10.0, 10.0, 10.0, 10.0],
        first_valid: 0,
    };
    let ce = VcetChandelierSignals {
        long_stop: vec![10.0, 10.0, 10.0, 10.0],
        short_stop: vec![10.0, 10.0, 10.0, 10.0],
        first_valid: 0,
    };
    let desired = build_vcet_desired_positions(
        &close,
        &vidya,
        &ce,
        0,
        &StrategyConfig {
            long_only: false,
            allow_flip: false,
            trade_on_next_bar: false,
            ..StrategyConfig::default()
        },
        &VcetStrategyParams {
            require_vidya_slope: false,
            ..VcetStrategyParams::default()
        },
    );

    assert_eq!(desired[0], 1);
}
```

#### Test 3 — long-only should not flip short
```rust
#[test]
fn build_vcet_desired_positions_long_only_does_not_flip_short() {
    let close = vec![10.0, 10.5, 11.0, 10.2, 9.8];
    let vidya = VcetVidyaSignals {
        values: vec![9.5, 10.0, 10.4, 10.4, 10.2],
        first_valid: 0,
    };
    let ce = VcetChandelierSignals {
        long_stop: vec![9.4, 9.8, 10.1, f64::NAN, f64::NAN],
        short_stop: vec![f64::NAN, f64::NAN, f64::NAN, 10.3, 10.0],
        first_valid: 0,
    };

    let desired = build_vcet_desired_positions(
        &close,
        &vidya,
        &ce,
        0,
        &StrategyConfig {
            long_only: true,
            allow_flip: true,
            trade_on_next_bar: false,
            ..StrategyConfig::default()
        },
        &VcetStrategyParams::default(),
    );

    assert_eq!(desired[1], 1);
    assert_eq!(desired[3], 0);
    assert!(desired.iter().all(|&v| v >= 0));
}
```

### Add these tests to `vcet_gpu_kernel.rs`

These tests are small but valuable because they guard the input-contract and blocker behavior around the GPU kernel path.

#### Test 1 — input length validation
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ta_strategies::double_ma::StrategyConfig;

    #[test]
    fn validate_inputs_rejects_combo_length_mismatch() {
        let signals = VcetGpuDeviceSignalMatrices {
            close_f32: &[1.0, 2.0, 3.0],
            cols: 3,
            vidya_rows_ptr: &[0x10],
            ce_long_rows_ptr: &[0x20],
            ce_short_rows_ptr: &[0x30],
            vidya_rows: 1,
            ce_rows: 1,
        };

        let combos = VcetGpuComboBatch {
            vidya_row_index: &[0, 0],
            ce_row_index: &[0],
            start_t: &[0, 0],
            end_t: &[2, 2],
        };

        let err = validate_inputs(&signals, &combos).unwrap_err();
        assert!(err.contains("combo vector length mismatch"));
    }
```

#### Test 2 — overlay blocker
```rust
    #[test]
    fn gpu_kernel_rejects_execution_overlays() {
        let signals = VcetGpuDeviceSignalMatrices {
            close_f32: &[1.0, 2.0, 3.0],
            cols: 3,
            vidya_rows_ptr: &[0x10],
            ce_long_rows_ptr: &[0x20],
            ce_short_rows_ptr: &[0x30],
            vidya_rows: 1,
            ce_rows: 1,
        };

        let combos = VcetGpuComboBatch {
            vidya_row_index: &[0],
            ce_row_index: &[0],
            start_t: &[0],
            end_t: &[2],
        };

        let strategy = StrategyConfig {
            // Fill with whichever overlay field or stop/take-profit setting
            // makes `uses_execution_overlays()` return true in your actual type.
            ..StrategyConfig::default()
        };

        let err = eval_vcet_metrics_gpu_kernel_device_rows(
            0,
            &signals,
            &combos,
            &strategy,
            &VcetStrategyParams::default(),
        )
        .unwrap_err();

        assert!(err.contains("without overlays/stop-loss/take-profit"));
    }
}
```

### Important “do not force unsafe uninitialized allocation” notes

Do **not** replace the following with uninitialized buffers:

1. `build_vcet_desired_positions`:
   ```rust
   let mut desired = vec![0i32; n];
   ```
   Keep this as-is. The zero prefix is part of observable behavior.

2. Any buffer where warmup values (`NaN`, `0`, etc.) are semantically visible before guaranteed overwrite.

3. `Vec<Metrics>` decode output:
   ```rust
   let mut out: Vec<Metrics> = Vec::with_capacity(combo_count);
   for row in host_metrics.chunks_exact(VCET_METRICS_STRIDE) {
       out.push(decode_metrics_row(row));
   }
   ```
   This is already fine.

---

## 7. File-by-file change plan

This section turns the findings into concrete implementation steps.

### A. `vcet.rs`

#### A1. Cache static descriptor metadata
Implement `vcet_vidya_meta()` and `vcet_chandelier_meta()` using `OnceLock<Result<..., String>>`.

#### A2. Replace small heap param vectors with arrays
Change these functions:

- `compute_vcet_vidya_signals`
- `compute_vcet_chandelier_signals`
- `compute_vcet_vidya_signal_cuda_device` (or its strict/permissive split)

from `Vec<IndicatorParamKV>` to fixed arrays.

#### A3. Split strict and permissive device helpers
Add:
- `compute_vcet_vidya_signal_cuda_device_strict`
- keep a permissive helper only if a non-hot-path caller actually needs it

Make the GPU-native VCET route use the strict helper.

#### A4. Consider thread-local Chandelier handle caching
Only after verifying `CudaChandelierExit::new(...)` is not trivial.

#### A5. Expand tests
Add the three tests listed above.

#### A6. Leave these areas unchanged
- `VcetOwnedDeviceMatrix::device_ptr()` keepalive match-arm pattern
- `build_vcet_desired_positions` zero-filled `desired`
- `VcetOwnedDeviceMatrix::from_host_row` unless/until the host fallback path is removed from hot use

---

### B. `vcet_gpu_kernel.rs`

#### B1. Import shared allocation helper
Add:
```rust
use crate::alloc_policy::alloc_uninit_copy;
```

#### B2. Add uploaded batch API
Implement:
- `UploadedVcetBatch`
- `upload_vcet_batch(...)`
- `eval_uploaded_vcet_metrics_gpu_kernel(...)`

#### B3. Keep one-shot wrapper for compatibility
Make the current public function a thin wrapper around the upload + eval flow.

#### B4. Eliminate zero-fill host readback
Use `alloc_uninit_copy(out_len)`.

#### B5. Defer runtime-function caching until after B2/B4
The upload reuse is much more important than caching `get_function(...)` or launch-shape discovery.

#### B6. Add small input/guard tests
Add the two tests above.

#### B7. Optional phase after B2
If repeated eval still shows H2D cost, consider pinned async upload for cold batches using the adjacent `gpu_backtest_kernel.rs` pattern.

---

### C. `gpu_native_expansion_opt.rs`

#### C1. Add VCET import
```rust
use ta_app_core::VcetStrategyParams;
```

#### C2. Add `vcet: None` to `base_req` if the request type supports it

#### C3. Add `make_vcet_trend_reversal_candles`

#### C4. Add `make_vcet_req`

#### C5. Add `bench_vcet_cpu`

#### C6. Add `bench_vcet_gpu_kernel`

#### C7. Add both to `criterion_group!`

#### C8. Use route probes exactly like existing GPU cases
Do not add a new benchmark style. Reuse the current pattern so route and fallback evidence stay consistent across strategies.

---

### D. `composite_validation_workflow.rs`

#### D1. Add VCET import
```rust
use ta_app_core::VcetStrategyParams;
```

#### D2. Add `vcet: None` to non-VCET candidate constructors if the type supports it

#### D3. Add `make_vcet_candidates`

#### D4. Add VCET CPU profile runs
```rust
bench_cpu_profile("vcet", 50_000, make_vcet_candidates(8));
bench_cpu_profile("vcet", 100_000, make_vcet_candidates(32));
```

#### D5. Add VCET GPU profile runs
```rust
bench_profile("vcet", 50_000, make_vcet_candidates(8));
bench_profile("vcet", 100_000, make_vcet_candidates(32));
```

#### D6. Add one dedicated large GPU VCET validation group
Mirror the ASR-specific group.

#### D7. Keep route probe printouts
These are important for fallback transparency and benchmark fidelity.

---

## Copy-boundary audit

This is the most important section if your goal is to minimize unnecessary memory copies.

| Location | Current behavior | Avoidable? | Recommended action |
|---|---|---:|---|
| `vcet.rs::compute_indicator_f64` | CUDA host output then `HostF32 -> Vec<f64>` conversion | Partly | Keep only for CPU/reference surfaces; not for GPU-native staging |
| `vcet.rs::compute_vcet_vidya_signal_cuda_device` | `DeviceF32` failure falls back to `HostF32`, then host result is uploaded back to device | **Yes** | Replace with strict device-only helper for the GPU-native path |
| `vcet.rs::compute_vcet_chandelier_signals` | Separate long and short dispatches | **Yes** | Treat the pair as one logical retrieval surface |
| `vcet_gpu_kernel.rs::eval_vcet_metrics_gpu_kernel_device_rows` | Upload `close_f32` every call | **Yes** for repeated runs | Move into reusable uploaded batch |
| `vcet_gpu_kernel.rs::eval_vcet_metrics_gpu_kernel_device_rows` | Recompute `d_lr` every call | **Yes** for repeated runs | Store `d_lr` in uploaded batch |
| `vcet_gpu_kernel.rs::eval_vcet_metrics_gpu_kernel_device_rows` | Upload row-pointer arrays every call | **Yes** for repeated runs | Store in uploaded batch |
| `vcet_gpu_kernel.rs::eval_vcet_metrics_gpu_kernel_device_rows` | Upload combo arrays every call | **Yes** for repeated runs | Store in uploaded batch |
| `vcet_gpu_kernel.rs::eval_vcet_metrics_gpu_kernel_device_rows` | Zero-fill `host_metrics` before `copy_to` | **Yes** | Use shared uninitialized host buffer |
| `vcet.rs::build_vcet_desired_positions` | Zero-fill `desired` | **No** | Keep as-is; zeros are observable behavior |
| `vcet.rs::VcetOwnedDeviceMatrix::from_host_row` | Host vector to device buffer copy | Only if hot path uses host fallback | Eliminate host fallback in the strict GPU route; otherwise this copy is unavoidable |

---

## MaybeUninit / uninitialized-allocation guidance

Per the map, use shared helpers and only adopt uninitialized allocation when every element is guaranteed to be written before any read.

### Safe to use shared uninitialized allocation
- `host_metrics` in `vcet_gpu_kernel.rs`
- `d_lr` and `d_out` are already using device-side uninitialized allocation appropriately

### Not safe or not useful
- `desired` in `build_vcet_desired_positions`
- any buffer whose prefix must be a visible `0.0` or `NaN`
- tiny fixed-size param lists (just use stack arrays, not `MaybeUninit`)

### Preferred policy
Do **not** add new local unsafe allocation logic in VCET unless the shared `alloc_policy` helper is insufficient. Use the same helper surface already adopted in adjacent GPU hot paths.

---

## Suggested rollout order

### Phase 1 — measurement coverage first
1. Add VCET optimizer bench entries in `gpu_native_expansion_opt.rs`
2. Add VCET validation bench entries in `composite_validation_workflow.rs`
3. Confirm route-probe output and record current CPU/GPU behavior

### Phase 2 — trivial parity-safe cleanup
4. Replace small param `Vec`s with arrays
5. Cache descriptor metadata with `OnceLock`
6. Replace zero-filled `host_metrics` with shared uninitialized allocation

### Phase 3 — remove worst GPU-native route waste
7. Split strict/permissive VIDYA device helper
8. Make the GPU-native path use the strict helper
9. Validate no hot path is still doing `HostF32 -> device` bounce

### Phase 4 — biggest app-path improvement
10. Add `UploadedVcetBatch`
11. Keep current one-shot wrapper as a compatibility layer
12. Update repeated call sites to reuse uploaded VCET batches

### Phase 5 — validation and regression hardening
13. Add equality / `trade_on_next_bar` / long-only tests in `vcet.rs`
14. Add GPU input-contract and blocker tests in `vcet_gpu_kernel.rs`
15. Rerun Criterion benches and compare route probe + runtime behavior

### Phase 6 — only if still needed
16. Evaluate caching `CudaChandelierExit`
17. Evaluate pinned async H2D cold uploads
18. Evaluate runtime kernel-handle / launch-shape caching
19. Revisit pointer-table vs contiguous-matrix API only if profiling demands it

---

## Expected payoff by change

| Change | Main benefit | Timing boundary |
|---|---|---|
| Strict VIDYA device helper | Removes host bounce in GPU-native setup | app-path |
| Uploaded VCET batch | Cuts repeated H2D traffic and repeated log-return work | app-path |
| Uninitialized `host_metrics` | Removes wasted host zero-fill | app-path |
| Param arrays | Removes tiny heap allocs in setup | app-path |
| Descriptor metadata cache | Cuts repeated string/descriptor scans | app-path |
| VCET optimizer bench | Gives actual route/payoff evidence | Criterion |
| VCET validation bench | Gives strategy-specific validation-cost evidence | validation-cost harness |

---

## Notes on what is unclear from the provided files

To stay honest and grounded, these are the things I cannot fully confirm from the provided files alone:

1. Whether `CompositeCpuRequest` and `CompositeParams` already expose `vcet` fields.
2. Whether `CudaChandelierExit::new(...)` is cheap or expensive.
3. Whether there is already a broader route-summary surface that should receive explicit VCET fallback reasons when device-only staging fails.
4. Whether all VCET signal rows could be represented as contiguous matrices instead of pointer tables.
5. Whether the broader caller can already preserve an uploaded VCET batch across repeated evals.

Those are all reasonable next-checks, but they live partly outside the provided files.

---

# Comprehensive implementation checklist

Use this as the tracking list while you implement.

## Bench coverage
- [ ] Add `VcetStrategyParams` import to `gpu_native_expansion_opt.rs`
- [ ] Add `vcet: None` to `base_req` if `CompositeCpuRequest` supports it
- [ ] Add `make_vcet_trend_reversal_candles`
- [ ] Add `make_vcet_req`
- [ ] Add `bench_vcet_cpu`
- [ ] Add `bench_vcet_gpu_kernel`
- [ ] Add both VCET benches to `criterion_group!`
- [ ] Confirm GPU route probe prints `backend_used=Gpu` for the VCET GPU bench once the route is wired
- [ ] Add `VcetStrategyParams` import to `composite_validation_workflow.rs`
- [ ] Add `vcet: None` to all non-VCET `CompositeParams` constructors if the type supports it
- [ ] Add `make_vcet_candidates`
- [ ] Add `bench_cpu_profile("vcet", ...)`
- [ ] Add `bench_profile("vcet", ...)`
- [ ] Add dedicated `composite_validate/gpu_kernel/vcet/100k/64candidates` group
- [ ] Confirm route probe output is visible for VCET validation benches

## `vcet.rs` cleanup
- [ ] Add `OnceLock`-based `vcet_vidya_meta()`
- [ ] Add `OnceLock`-based `vcet_chandelier_meta()`
- [ ] Replace small `Vec<IndicatorParamKV>` allocations with fixed arrays in:
  - [ ] `compute_vcet_vidya_signals`
  - [ ] `compute_vcet_chandelier_signals`
  - [ ] `compute_vcet_vidya_signal_cuda_device` / strict helper
- [ ] Add strict `compute_vcet_vidya_signal_cuda_device_strict`
- [ ] Restrict host-bounce fallback to an explicitly permissive helper only if needed
- [ ] Review whether any hot-path caller is still using the permissive helper
- [ ] Optionally add thread-local `CudaChandelierExit` cache if constructor cost is real
- [ ] Keep `VcetOwnedDeviceMatrix::device_ptr()` ownership-preserving pattern unchanged
- [ ] Keep `build_vcet_desired_positions` zero-filled `desired` unchanged

## `vcet_gpu_kernel.rs` hot-path work
- [ ] Import shared `alloc_uninit_copy`
- [ ] Add `UploadedVcetBatch`
- [ ] Add `upload_vcet_batch(...)`
- [ ] Add `eval_uploaded_vcet_metrics_gpu_kernel(...)`
- [ ] Keep `eval_vcet_metrics_gpu_kernel_device_rows(...)` as a wrapper
- [ ] Move repeated H2D uploads into the uploaded batch
- [ ] Precompute and store `d_lr` in the uploaded batch
- [ ] Allocate `d_out` once per uploaded batch
- [ ] Replace zero-filled `host_metrics` with `alloc_uninit_copy(out_len)`
- [ ] Verify `copy_to(...)` always fully overwrites the host buffer before any read
- [ ] Re-run benches after the batch refactor
- [ ] Only after that, consider cached launch metadata / function lookup

## Tests
- [ ] Add `compute_vcet_start_t_honors_trade_on_next_bar`
- [ ] Add `build_vcet_desired_positions_treats_equal_thresholds_as_valid_signal`
- [ ] Add `build_vcet_desired_positions_long_only_does_not_flip_short`
- [ ] Add `validate_inputs_rejects_combo_length_mismatch`
- [ ] Add GPU overlay/blocker-path test in `vcet_gpu_kernel.rs`
- [ ] Add any higher-level GPU-vs-CPU parity test once the broader scoring entry point is visible

## Copy-elision audit
- [ ] Confirm no zero-filled host readback buffer remains in VCET GPU hot paths
- [ ] Confirm no GPU-native hot path still does `HostF32 -> device` bounce for VIDYA
- [ ] Confirm repeated evaluation can reuse resident `close`, `lr`, row-pointer arrays, and combo arrays
- [ ] Confirm no unsafe uninitialized allocation was added where prefix values are behaviorally observable
- [ ] Confirm any remaining host/device copy boundary is truly unavoidable

## Validation before merge
- [ ] Run CPU and GPU VCET Criterion benches
- [ ] Capture route probe output for optimizer benches
- [ ] Capture route probe output for validation benches
- [ ] Compare pre/post runtime for repeated VCET evaluation
- [ ] Sanity-check parity behavior around equality thresholds
- [ ] Sanity-check long-only and `trade_on_next_bar` behavior
- [ ] Verify no regression in overlay blocker behavior
- [ ] Document any remaining fallback cases explicitly

## Stretch goals
- [ ] Cache `CudaChandelierExit` if constructor cost is measurable
- [ ] Add pinned async upload path for large cold batches if H2D is still dominant
- [ ] Revisit pointer-table API only if profiling shows it matters
- [ ] Consider device-side shortlist/reduction only after simpler wins are exhausted
