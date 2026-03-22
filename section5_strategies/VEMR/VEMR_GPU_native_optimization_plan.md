
# VEMR strategy-local GPU-native optimization and validation plan

## Scope and prompt alignment

This document follows the review shape from **`## 5. Strategy-local GPU-native optimization and validation routes`** in `gpt54_feature_review_map.md`, restricted to the **VEMR** strategy family only. It also pulls in the map’s **allocation policy / copy-elision** guidance because VEMR’s current hot path includes host readback buffers and full-buffer zero fills that can be removed safely when every element is overwritten before any read.

This plan is intentionally limited to the attached files:

- `vemr.rs`
- `vemr_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

Where an edit needs a missing adjacent callsite, I call that out explicitly instead of pretending the attached files are sufficient by themselves.

## Surface under review

The VEMR surface has two major performance layers:

1. **Indicator prep / reference-path prep** in `vemr.rs`, where periods are resolved, indicator sweeps are built, CPU/GPU prep helpers are selected, and CPU reference entry/exit logic is defined.
2. **Strategy-local metrics evaluation** in `vemr_gpu_kernel.rs`, where device-resident indicator row-major buffers are consumed by the VEMR GPU metrics kernel, optionally reduced to top-k, and decoded back to host.

The two benchmark files in scope matter for measurement quality:

- `gpu_native_expansion_opt.rs` is the **Criterion optimizer** surface.
- `composite_validation_workflow.rs` is the **Criterion validation-cost** surface.

## Files reviewed

- `vemr.rs`
- `vemr_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`
- `gpt54_feature_review_map.md` (prompt/rules only)

## Timing boundaries used in this document

I refer to the same timing boundaries the review map asks for:

- **warm app-path runtime**
- **Criterion benchmark**
- **validation-cost harness**
- **route-matrix / route-summary evidence**
- **profiling-only / Nsight**

## Executive summary

If you only do three things first, do these in order:

1. **Stop recomputing the Bollinger sweep three times** in the VEMR CUDA prep path by introducing a bundled `upper/middle/lower` sweep result and wiring the VEMR GPU caller to use it once.
2. **Stop re-uploading `close` and re-deriving log returns every VEMR GPU evaluation** when the price series is already device-resident or stable across repeated validation/eval calls.
3. **Add real VEMR-specific Criterion optimizer and validation benchmark coverage** so you can prove the payoff and guard the route against silent CPU fallback.

Those three changes attack the largest confirmed waste in the attached VEMR surface. The next tier is scratch-buffer reuse, host readback copy-elision via `MaybeUninit`/shared alloc helpers, and VEMR-specific benchmark shapes that actually exercise VEMR’s top-k path rather than accidentally benchmarking a full-readback path.

---

# 1. Surface understanding

## 1.1 `vemr.rs`

`vemr.rs` currently contains:

- VEMR parameter validation and period resolution.
- Device matrix ownership wrappers:
  - `VemrDeviceMatrix`
  - `VemrPeriodSweepDevice`
- Direct CUDA sweep helpers for:
  - z-score
  - Bollinger Bands
  - ATR
- Fallback / generic dynamic-indicator CUDA helpers.
- CPU / host-path signal builders:
  - `compute_vemr_fast_signals`
  - `compute_vemr_risk_signals`
- CPU reference execution logic:
  - `compute_vemr_start_t`
  - `build_vemr_desired_positions`

The most important performance observation here is that the direct Bollinger helper already computes **all three outputs** in one call, but the public API shape still exposes them separately.

## 1.2 `vemr_gpu_kernel.rs`

`vemr_gpu_kernel.rs` currently contains:

- `VemrGpuDeviceSignalMatrices`
- `VemrGpuComboBatch`
- `VemrCudaRuntime`
- top-k reduction helper logic
- `eval_vemr_metrics_gpu_kernel_device_rows`

The key performance observations here are:

- the runtime caches **context/module/stream**, but not scratch buffers,
- `close` still arrives as a **host slice** even though the indicator matrices are already device-side,
- log returns are recomputed inside every evaluation call,
- host readback buffers are zero-filled before `copy_to(...)` overwrites them.

## 1.3 `gpu_native_expansion_opt.rs`

This benchmark file currently covers:

- KQSR
- CARM
- KVDF
- CDMC

It does **not** currently include VEMR optimizer benches, even though this bundle is strategy-local and VEMR-specific.

## 1.4 `composite_validation_workflow.rs`

This benchmark file does include a VEMR validation profile slot, but:

- it does not currently assert that the VEMR GPU route stayed on GPU,
- it does not include a larger VEMR profile case matching the larger 100k/32 coverage used for other strategies,
- it uses a generic synthetic candle shape, which is serviceable for broad validation throughput, but is not obviously VEMR-shaped.

---

# 2. Findings

## Finding 1 — confirmed issue

- **Status:** confirmed issue
- **Severity:** high
- **Type:** performance
- **Location:** `vemr.rs` — `compute_vemr_bollinger_output_sweep_cuda_direct`, `compute_vemr_bollinger_output_sweep_cuda_device`, `compute_vemr_bollinger_upper_sweep_cuda_device`, `compute_vemr_bollinger_middle_sweep_cuda_device`, `compute_vemr_bollinger_lower_sweep_cuda_device`
- **Timing boundary affected:** warm app-path runtime, Criterion benchmark, validation-cost harness
- **Why it matters:**
  - The direct Bollinger helper calls `CudaBollingerBands::bollinger_bands_batch_*` once and receives `(upper, middle, lower)`.
  - The public VEMR API still exposes **three separate single-output sweep helpers**.
  - That is the exact shape that causes repeated work if the VEMR GPU orchestration asks for upper/middle/lower independently.
  - Even if an adjacent caller is not attached, the current API shape makes the repeated-computation footgun very easy and very likely.
- **Recommended change:**
  - Introduce a bundled CUDA Bollinger sweep result that owns all three output matrices in one object.
  - Keep the single-output helpers only as compatibility wrappers, but update the actual VEMR GPU callsite to use the bundled helper exactly once per request.

## Finding 2 — confirmed issue

- **Status:** confirmed issue
- **Severity:** high
- **Type:** performance
- **Location:** `vemr_gpu_kernel.rs` — `VemrGpuDeviceSignalMatrices`, `launch_compute_log_returns_f32`, `eval_vemr_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** warm app-path runtime, validation-cost harness
- **Why it matters:**
  - The VEMR GPU wrapper already consumes device pointers for z-score, Bollinger, and ATR matrices.
  - But it still takes `close_f32: &[f32]`, uploads `close` on every call with `DeviceBuffer::from_slice`, allocates `d_lr`, and launches a separate log-return kernel every time.
  - This is the most obvious avoidable **host→device copy boundary** left in the VEMR GPU path.
  - It will disproportionately hurt repeated validation windows, fixed-candidate reruns, and any warm-path workload where the price series is unchanged but candidate rows vary.
- **Recommended change:**
  - Extend `VemrGpuDeviceSignalMatrices` so it can accept:
    - an optional device-resident `close` pointer, and
    - an optional precomputed device-resident log-return pointer.
  - Only fall back to host upload / on-demand log-return generation when device pointers are not available.

## Finding 3 — confirmed issue

- **Status:** confirmed issue
- **Severity:** high
- **Type:** performance
- **Location:** `vemr_gpu_kernel.rs` — `VEMR_GPU_RUNTIME`, `VemrCudaRuntime`, `eval_vemr_metrics_gpu_kernel_device_rows`, `reduce_vemr_topk_on_device`
- **Timing boundary affected:** warm app-path runtime, Criterion benchmark, validation-cost harness
- **Why it matters:**
  - The current runtime cache preserves only:
    - CUDA context
    - module
    - stream
  - It does **not** preserve:
    - `d_close`
    - `d_lr`
    - combo buffers
    - metrics output buffer
    - top-k reduction buffers
  - So repeated evaluations still pay repeated device allocations, despite already having a per-device runtime cache.
- **Recommended change:**
  - Add a `VemrCudaScratch` block inside `VemrCudaRuntime`.
  - Reuse exact-shape buffers on repeated calls first.
  - If you already have sub-slice copy helpers elsewhere, grow-only reuse is even better, but exact-shape reuse is still a strong first pass.

## Finding 4 — confirmed issue

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** performance
- **Location:** `vemr_gpu_kernel.rs` — `reduce_vemr_topk_on_device`, `eval_vemr_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** warm app-path runtime, Criterion benchmark, validation-cost harness
- **Why it matters:**
  - These host vectors are currently fully prefilled even though the device fully overwrites them:
    - `host_combo_tags`
    - `host_top_metrics`
    - `host_out`
  - This is exactly the case where the review map says copy-elision / `MaybeUninit` is appropriate: every element is written before any read.
- **Recommended change:**
  - Replace those zero-filled vectors with either:
    - the shared helper from `alloc_policy.rs` (preferred if available in your repo), or
    - a local `MaybeUninit` readback helper.
  - Do **not** remove full-fill where the fill value is observable behavior; this recommendation applies only to readback buffers whose entire contents come from `copy_to(...)`.

## Finding 5 — confirmed issue

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `gpu_native_expansion_opt.rs`
- **Timing boundary affected:** Criterion benchmark
- **Why it matters:**
  - This file has CPU/GPU optimizer benches for KQSR, CARM, KVDF, and CDMC.
  - It currently has **no VEMR optimizer benchmark coverage at all**.
  - That means the strategy-local review has no direct Criterion optimizer evidence for VEMR from the attached benchmark file.
- **Recommended change:**
  - Add:
    - a VEMR-specific candle generator,
    - a VEMR request builder,
    - a CPU VEMR optimizer bench,
    - a GPU VEMR optimizer bench.
  - Add two GPU VEMR benchmark shapes:
    - **device top-k path**
    - **full-readback / include-all or heatmap path**

## Finding 6 — confirmed issue

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `gpu_native_expansion_opt.rs` — `base_req`
- **Timing boundary affected:** Criterion benchmark
- **Why it matters:**
  - `base_req` sets:
    - `top_k = Some(20)`
    - `include_all = Some(false)`
    - `heatmap_bins = Some(64)`
  - The VEMR GPU kernel uses device-side top-k only when:
    - `!include_all`
    - `heatmap_bins == 0`
    - `topk_select < combo_count`
    - `topk_select <= VEMR_DEVICE_TOPK_LIMIT`
  - So a VEMR benchmark that blindly reuses `base_req` will not exercise the pure device top-k path.
- **Recommended change:**
  - Add explicit VEMR request builders for:
    - `topk/no_heatmap`
    - `full-readback/heatmap`
  - Benchmark both so you can see whether the staged top-k path is actually paying off.

## Finding 7 — confirmed issue

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `composite_validation_workflow.rs` — `bench_composite_validation_gpu_workflow`, `bench_profile`
- **Timing boundary affected:** validation-cost harness, route-matrix / route-summary evidence
- **Why it matters:**
  - The VEMR GPU validation profile is present:
    - `bench_profile("vemr", 50_000, make_vemr_candidates(8));`
  - But the benchmark:
    - only prints backend route/fallback,
    - does not assert that VEMR stayed on GPU,
    - does not provide the larger 100k/32 VEMR case that would expose scaling behavior.
- **Recommended change:**
  - Make VEMR’s validation benchmark explicitly assert `BackendUsed::Gpu`.
  - Add at least:
    - `50k / 32 candidates`
    - `100k / 32 candidates`
  - Keep the existing 50k / 8 case for continuity.

## Finding 8 — confirmed issue

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** performance
- **Location:** `vemr.rs` — `compute_vemr_fast_signals`
- **Timing boundary affected:** warm app-path runtime, validation-cost harness
- **Why it matters:**
  - The CPU/reference-path fast signal builder performs separate calls for:
    - z-score
    - Bollinger upper
    - Bollinger middle
    - Bollinger lower
  - Even ignoring the missing adjacent adapter context, this is at least repeated dynamic dispatch / repeated descriptor resolution.
  - It is very likely also repeated indicator compute on the host path unless the adapter caches internally.
- **Recommended change:**
  - First-pass safe change:
    - cache descriptor keys/ids,
    - replace heap `Vec` param lists with fixed arrays.
  - Second-pass change if adjacent adapter supports it:
    - add a multi-output Bollinger host helper so the host path does not dispatch the same indicator three times.

## Finding 9 — likely issue

- **Status:** likely issue
- **Severity:** medium
- **Type:** performance
- **Location:** `vemr.rs` — `compute_vemr_zscore_sweep_cuda_direct`, `compute_vemr_bollinger_output_sweep_cuda_direct`, `compute_vemr_atr_sweep_cuda_direct`
- **Timing boundary affected:** warm app-path runtime, validation-cost harness
- **Why it matters:**
  - These helpers construct wrapper objects on every call:
    - `CudaZscore::new(...)`
    - `CudaBollingerBands::new(...)`
    - `CudaAtr::new(...)`
  - If those constructors do anything non-trivial (module load, stream creation, state prep), wrapper re-init could become meaningful in repeated validation / optimization loops.
  - The attached files do not include wrapper internals, so the magnitude is still uncertain.
- **Recommended change:**
  - Measure constructor cost first.
  - If non-trivial, cache wrapper/runtime objects in the adjacent VEMR orchestration layer or a strategy-local runtime cache.

## Finding 10 — confirmed issue

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** performance
- **Location:** `vemr_gpu_kernel.rs` — `launch_compute_log_returns_f32`, `eval_vemr_metrics_gpu_kernel_device_rows`, `reduce_vemr_topk_on_device`
- **Timing boundary affected:** warm app-path runtime, validation-cost harness
- **Why it matters:**
  - Device launch caps and kernel launch caps are re-detected on every evaluation call.
  - That is not the dominant cost compared with host→device staging and allocations, but it is still unnecessary repeated control-path work inside a cached runtime.
- **Recommended change:**
  - Cache device launch caps in `VemrCudaRuntime`.
  - If the `cust` function handles / kernel-cap descriptors are safely reusable, cache them or their derived launch metadata too.
  - Do this only after the larger staging/allocation fixes are in place.

## Finding 11 — confirmed route split, impact open

- **Status:** open question
- **Severity:** medium
- **Type:** performance
- **Location:** `vemr.rs` — `compute_vemr_zscore_sweep_cuda_device`
- **Timing boundary affected:** warm app-path runtime, Criterion benchmark, validation-cost harness
- **Why it matters:**
  - VEMR gets the direct CUDA z-score sweep only when:
    - `cfg.z_devtype == 0`
    - `cfg.z_ma_type == "sma"` (case-insensitive)
  - Other z-score configurations fall back to the generic dynamic sweep path.
  - That is an explicit performance cliff by configuration.
- **Recommended change:**
  - Benchmark default and non-default z-score configurations separately before deciding whether to add another direct specialization.
  - Do not broaden the direct path until you have benchmark evidence that non-default configurations matter in real workloads.

## Finding 12 — open question

- **Status:** open question
- **Severity:** medium
- **Type:** parity
- **Location:** `vemr.rs` — `build_vemr_desired_positions`; `vemr_gpu_kernel.rs` — `eval_vemr_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** unknown
- **Why it matters:**
  - The CPU reference path is `f64`-based.
  - The GPU metrics path consumes `f32` price/indicator data and downcasts:
    - commission
    - `z_entry`
    - `z_exit`
    - `atr_stop_mult`
  - Threshold-adjacent behavior is exactly where parity drift can flip a trade.
  - The actual CUDA kernel body is not attached, so parity can only be treated as an open question, not resolved.
- **Recommended change:**
  - Add targeted parity tests around:
    - `require_band_touch`
    - `confirm_bars > 0`
    - `allow_flip`
    - `trade_on_next_bar`
    - boundary-touch values around `z_entry` and `z_exit`

---

# 3. Safe optimization opportunities

This section is the concrete implementation plan. The snippets are designed to stay inside the attached-file surface where possible. I note explicitly when a change requires a missing adjacent callsite.

## 3.1 `vemr.rs` — bundled Bollinger sweeps

### Why this should be first

This is the cleanest high-payoff edit in the attached VEMR surface:

- the work is already available in one direct CUDA call,
- the current API shape encourages waste,
- the change is local and low-risk.

### Proposed new type

```rust
#[cfg(feature = "cuda")]
pub struct VemrBollingerSweepDevice {
    pub upper: VemrPeriodSweepDevice,
    pub middle: VemrPeriodSweepDevice,
    pub lower: VemrPeriodSweepDevice,
}
```

### Proposed new bundled helper

```rust
#[cfg(feature = "cuda")]
pub fn compute_vemr_bollinger_sweeps_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrBollingerSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);

    let sweep = BollingerBandsBatchRange {
        period: (
            usize::from(start),
            usize::from(end),
            usize::from(step),
        ),
        devup: (cfg.bb_mult, cfg.bb_mult, 0.0),
        devdn: (cfg.bb_mult, cfg.bb_mult, 0.0),
        matype: ("sma".to_string(), "sma".to_string(), 0),
        devtype: (0, 0, 0),
    };

    let wrapper = CudaBollingerBands::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.source_first_valid();

    let (upper, middle, lower) = wrapper
        .bollinger_bands_batch_from_device_ptr(
            prepared_inputs.source_device().buffer().as_device_ptr(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| wrapper.bollinger_bands_batch_dev(prepared_inputs.source(), &sweep))
        .map_err(|e| e.to_string())?;

    let mk = |matrix: VemrDeviceMatrix| VemrPeriodSweepDevice {
        matrix,
        period_start: start,
        period_end: end,
        period_step: step,
    };

    Ok(VemrBollingerSweepDevice {
        upper: mk(VemrDeviceMatrix::from_bollinger(upper)),
        middle: mk(VemrDeviceMatrix::from_bollinger(middle)),
        lower: mk(VemrDeviceMatrix::from_bollinger(lower)),
    })
}
```

### Compatibility wrappers

If other code still expects the old single-output helpers, keep them as thin wrappers:

```rust
#[cfg(feature = "cuda")]
pub fn compute_vemr_bollinger_upper_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    Ok(
        compute_vemr_bollinger_sweeps_cuda_device(
            prepared_inputs,
            cfg,
            period_start,
            period_end,
            period_step,
            device_id,
        )?
        .upper,
    )
}
```

Repeat for `middle` and `lower`.

### Important note

This does **not** remove the waste by itself unless the actual VEMR GPU orchestration callsite stops invoking upper/middle/lower separately. That adjacent callsite is not attached, so the implementation plan must include:

- adding the new bundled helper here, and
- replacing the three-call pattern in the missing orchestrator with one bundled call.

## 3.2 `vemr.rs` — cache indicator keys/ids once

This is not as large as the staging fixes, but it is safe, local, and reduces repeated string work in the reference path.

### Proposed cache type

```rust
use std::sync::OnceLock;

struct VemrIndicatorKeys {
    z_period: String,
    z_ma_type: String,
    z_nbdev: String,
    z_devtype: String,
    z_output: String,
    bb_period: String,
    bb_devup: String,
    bb_devdn: String,
    bb_upper: String,
    bb_middle: String,
    bb_lower: String,
    atr_period: String,
    atr_output: String,
}

fn vemr_indicator_keys() -> Result<&'static VemrIndicatorKeys, String> {
    static KEYS: OnceLock<Result<VemrIndicatorKeys, String>> = OnceLock::new();

    KEYS.get_or_init(|| {
        Ok(VemrIndicatorKeys {
            z_period: resolve_param_key("zscore", &["period", "length", "timeperiod"])?,
            z_ma_type: resolve_param_key("zscore", &["ma_type", "matype", "ma"])?,
            z_nbdev: resolve_param_key("zscore", &["nbdev", "stddev", "stdev", "deviation"])?,
            z_devtype: resolve_param_key("zscore", &["devtype", "deviation_type", "dev_type"])?,
            z_output: resolve_output_id("zscore", &["value", "zscore"])?,
            bb_period: resolve_param_key("bollinger_bands", &["period", "length", "timeperiod"])?,
            bb_devup: resolve_param_key("bollinger_bands", &["devup", "upper_mult", "upper"])?,
            bb_devdn: resolve_param_key("bollinger_bands", &["devdn", "lower_mult", "lower"])?,
            bb_upper: resolve_output_id("bollinger_bands", &["upper_band", "upper", "up"])?,
            bb_middle: resolve_output_id("bollinger_bands", &["middle_band", "middle", "mid", "basis"])?,
            bb_lower: resolve_output_id("bollinger_bands", &["lower_band", "lower", "down"])?,
            atr_period: resolve_param_key("atr", &["period", "length", "timeperiod"])?,
            atr_output: resolve_output_id("atr", &["value", "atr"])?,
        })
    })
    .as_ref()
    .map_err(Clone::clone)
}
```

### Usage in `compute_vemr_fast_signals`

```rust
pub fn compute_vemr_fast_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &VemrStrategyParams,
    z_period: u16,
    bb_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<VemrFastSignals, String> {
    let keys = vemr_indicator_keys()?;

    let z_params = [
        IndicatorParamKV {
            key: keys.z_period.clone(),
            value: IndicatorParamValue::Int(i64::from(z_period)),
        },
        IndicatorParamKV {
            key: keys.z_ma_type.clone(),
            value: IndicatorParamValue::EnumString(cfg.z_ma_type.clone()),
        },
        IndicatorParamKV {
            key: keys.z_nbdev.clone(),
            value: IndicatorParamValue::Float(cfg.z_nbdev),
        },
        IndicatorParamKV {
            key: keys.z_devtype.clone(),
            value: IndicatorParamValue::Int(i64::from(cfg.z_devtype)),
        },
    ];

    let bb_params = [
        IndicatorParamKV {
            key: keys.bb_period.clone(),
            value: IndicatorParamValue::Int(i64::from(bb_period)),
        },
        IndicatorParamKV {
            key: keys.bb_devup.clone(),
            value: IndicatorParamValue::Float(cfg.bb_mult),
        },
        IndicatorParamKV {
            key: keys.bb_devdn.clone(),
            value: IndicatorParamValue::Float(cfg.bb_mult),
        },
    ];

    let zscore = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "zscore",
        keys.z_output.as_str(),
        &z_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let bb_upper = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "bollinger_bands",
        keys.bb_upper.as_str(),
        &bb_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let bb_middle = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "bollinger_bands",
        keys.bb_middle.as_str(),
        &bb_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let bb_lower = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "bollinger_bands",
        keys.bb_lower.as_str(),
        &bb_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    // existing length/first_valid logic stays unchanged
}
```

### Why arrays instead of `Vec`

The current code uses heap-allocated `Vec`s for tiny fixed-size parameter lists. These can become stack arrays immediately because the callee takes `&[IndicatorParamKV]`.

Use the same pattern in `compute_vemr_risk_signals`:

```rust
let atr_params = [IndicatorParamKV {
    key: keys.atr_period.clone(),
    value: IndicatorParamValue::Int(i64::from(atr_period)),
}];
```

## 3.3 `vemr.rs` — optional `MaybeUninit` for `build_vemr_desired_positions`

This is **not** a top-three priority change. Do it after the GPU staging fixes.

### Why it is safe here

`build_vemr_desired_positions` writes:

- `0` for the prefix before `start_t`, and
- exactly one value for every `t in start_t..n`

So a partial-prefix-init + tail overwrite strategy is safe.

### Why it is lower priority

This saves one zero-fill over `n` elements on the CPU reference path. That is smaller than:

- triple Bollinger recompute,
- per-call `close` upload,
- per-call `d_lr` generation,
- repeated device allocations.

### Suggested helper

If `alloc_policy.rs` already exposes a helper for prefix-init / overwrite-only vectors, use that instead of local unsafe code.

If not, a local version can look like this:

```rust
use std::mem::MaybeUninit;

fn assume_init_vec<T>(mut v: Vec<MaybeUninit<T>>) -> Vec<T> {
    let ptr = v.as_mut_ptr() as *mut T;
    let len = v.len();
    let cap = v.capacity();
    std::mem::forget(v);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}
```

### Sketch inside `build_vemr_desired_positions`

```rust
pub fn build_vemr_desired_positions(
    close: &[f64],
    fast: &VemrFastSignals,
    risk: &VemrRiskSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &VemrStrategyParams,
) -> Vec<i32> {
    use std::mem::MaybeUninit;

    let n = close.len();
    if n == 0 {
        return Vec::new();
    }

    let mut desired: Vec<MaybeUninit<i32>> = Vec::with_capacity(n);
    unsafe { desired.set_len(n); }

    let prefix = start_t.min(n);
    for slot in &mut desired[..prefix] {
        slot.write(0);
    }

    if start_t >= n {
        return assume_init_vec(desired);
    }

    // existing loop logic...
    // replace:
    //     desired[t] = value;
    // with:
    //     desired[t].write(value);

    // IMPORTANT:
    // every continue path must write desired[t] before continuing.

    assume_init_vec(desired)
}
```

### Recommendation

Treat this as **phase 2**, not phase 1.

---

## 3.4 `vemr_gpu_kernel.rs` — accept device-resident `close` and optional precomputed `lr`

This is the largest strategy-local staging improvement after the bundled Bollinger sweep.

### Proposed input struct expansion

```rust
pub struct VemrGpuDeviceSignalMatrices<'a> {
    pub close_host_f32: Option<&'a [f32]>,
    pub close_device_ptr: Option<u64>,
    pub lr_device_ptr: Option<u64>,

    pub cols: usize,

    pub zscore_row_major_ptr: u64,
    pub zscore_rows: usize,
    pub zscore_stride: usize,

    pub bb_upper_row_major_ptr: u64,
    pub bb_middle_row_major_ptr: u64,
    pub bb_lower_row_major_ptr: u64,
    pub bb_rows: usize,
    pub bb_stride: usize,

    pub atr_row_major_ptr: u64,
    pub atr_rows: usize,
    pub atr_stride: usize,
}
```

### Updated validation

```rust
fn validate_inputs(
    signals: &VemrGpuDeviceSignalMatrices<'_>,
    combos: &VemrGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("VEMR GPU kernel requires non-empty series".to_string());
    }

    let has_host_close = signals
        .close_host_f32
        .map(|v| v.len() == n)
        .unwrap_or(false);
    let has_device_close = signals.close_device_ptr.unwrap_or(0) != 0;

    if !has_host_close && !has_device_close {
        return Err("VEMR GPU close input missing".to_string());
    }

    if let Some(v) = signals.close_host_f32 {
        if v.len() != n {
            return Err("VEMR GPU close length mismatch".to_string());
        }
    }

    if signals.zscore_row_major_ptr == 0
        || signals.bb_upper_row_major_ptr == 0
        || signals.bb_middle_row_major_ptr == 0
        || signals.bb_lower_row_major_ptr == 0
        || signals.atr_row_major_ptr == 0
    {
        return Err("VEMR GPU signal pointer is null".to_string());
    }

    // existing shape and combo checks stay the same
    Ok(())
}
```

### Make log-return launch accept a raw device pointer

This avoids forcing `close` through a temporary `DeviceBuffer<f32>` when `close` is already device-resident.

```rust
fn launch_compute_log_returns_f32(
    rt: &mut VemrCudaRuntime,
    device_id: u32,
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

    let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
    let device_caps = detect_device_launch_caps(device);
    let launch = choose_launch_shape_1d_from_candidates(
        len,
        device_caps,
        detect_kernel_launch_caps(&kernel, device_caps),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr_raw;
    let mut len_i =
        i32::try_from(len).map_err(|_| "VEMR GPU close length exceeds i32 range".to_string())?;
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

### Integration inside `eval_vemr_metrics_gpu_kernel_device_rows`

This is the core change:

```rust
let close_ptr = if let Some(ptr) = signals.close_device_ptr {
    ptr
} else {
    let close = signals
        .close_host_f32
        .ok_or_else(|| "VEMR GPU close input missing".to_string())?;

    let d_close = rt.scratch.ensure_close_exact(close.len())?;
    d_close.copy_from(close).map_err(|e| e.to_string())?;
    d_close.as_device_ptr().as_raw()
};

let lr_ptr = if let Some(ptr) = signals.lr_device_ptr {
    ptr
} else {
    let d_lr = rt.scratch.ensure_lr_exact(signals.cols)?;
    launch_compute_log_returns_f32(rt, device_id, close_ptr, d_lr, signals.cols)?;
    d_lr.as_device_ptr().as_raw()
};
```

### Adjacent wiring required

This change becomes much more valuable if the missing VEMR orchestration layer passes:

- `prepared_cuda_inputs.close_device().buffer().as_device_ptr()`
- and eventually a cached/precomputed VEMR log-return buffer

instead of always materializing host `close_f32`.

---

## 3.5 `vemr_gpu_kernel.rs` — add scratch-buffer reuse to `VemrCudaRuntime`

### Proposed runtime and scratch structs

```rust
struct VemrCudaScratch {
    close: Option<DeviceBuffer<f32>>,
    lr: Option<DeviceBuffer<f32>>,

    combo_z: Option<DeviceBuffer<i32>>,
    combo_bb: Option<DeviceBuffer<i32>>,
    combo_atr: Option<DeviceBuffer<i32>>,
    combo_start: Option<DeviceBuffer<i32>>,
    combo_end: Option<DeviceBuffer<i32>>,

    out: Option<DeviceBuffer<f32>>,

    combo_tags: Option<DeviceBuffer<i32>>,
    cand_a: Option<DeviceBuffer<i32>>,
    cand_b: Option<DeviceBuffer<i32>>,
    top_combo: Option<DeviceBuffer<i32>>,
    top_dummy: Option<DeviceBuffer<i32>>,
    top_metrics: Option<DeviceBuffer<f32>>,

    combo_tags_host: Vec<i32>,
    combo_tags_len: usize,
}

impl Default for VemrCudaScratch {
    fn default() -> Self {
        Self {
            close: None,
            lr: None,
            combo_z: None,
            combo_bb: None,
            combo_atr: None,
            combo_start: None,
            combo_end: None,
            out: None,
            combo_tags: None,
            cand_a: None,
            cand_b: None,
            top_combo: None,
            top_dummy: None,
            top_metrics: None,
            combo_tags_host: Vec::new(),
            combo_tags_len: 0,
        }
    }
}

struct VemrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    scratch: VemrCudaScratch,
}
```

### Runtime construction

```rust
Ok(Self {
    context,
    module,
    stream,
    scratch: VemrCudaScratch::default(),
})
```

### Exact-shape first pass

This is the lowest-risk reuse model and works well when shapes are stable across repeated calls.

```rust
impl VemrCudaScratch {
    fn ensure_f32_exact(
        slot: &mut Option<DeviceBuffer<f32>>,
        len: usize,
    ) -> Result<&mut DeviceBuffer<f32>, String> {
        let need_new = slot.as_ref().map(|b| b.len() != len).unwrap_or(true);
        if need_new {
            *slot = Some(
                unsafe { DeviceBuffer::<f32>::uninitialized(len) }
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(slot.as_mut().unwrap())
    }

    fn ensure_i32_exact(
        slot: &mut Option<DeviceBuffer<i32>>,
        len: usize,
    ) -> Result<&mut DeviceBuffer<i32>, String> {
        let need_new = slot.as_ref().map(|b| b.len() != len).unwrap_or(true);
        if need_new {
            *slot = Some(
                unsafe { DeviceBuffer::<i32>::uninitialized(len) }
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(slot.as_mut().unwrap())
    }

    fn ensure_close_exact(&mut self, len: usize) -> Result<&mut DeviceBuffer<f32>, String> {
        Self::ensure_f32_exact(&mut self.close, len)
    }

    fn ensure_lr_exact(&mut self, len: usize) -> Result<&mut DeviceBuffer<f32>, String> {
        Self::ensure_f32_exact(&mut self.lr, len)
    }

    fn ensure_out_exact(&mut self, len: usize) -> Result<&mut DeviceBuffer<f32>, String> {
        Self::ensure_f32_exact(&mut self.out, len)
    }

    fn ensure_combo_z_exact(&mut self, len: usize) -> Result<&mut DeviceBuffer<i32>, String> {
        Self::ensure_i32_exact(&mut self.combo_z, len)
    }

    // repeat for combo_bb, combo_atr, combo_start, combo_end, combo_tags,
    // cand_a, cand_b, top_combo, top_dummy, top_metrics
}
```

### Populate combo buffers from scratch instead of allocating every call

```rust
{
    let d_combo_z = rt.scratch.ensure_combo_z_exact(combo_count)?;
    d_combo_z.copy_from(combos.z_row_index).map_err(|e| e.to_string())?;
}
{
    let d_combo_bb = rt.scratch.ensure_combo_bb_exact(combo_count)?;
    d_combo_bb.copy_from(combos.bb_row_index).map_err(|e| e.to_string())?;
}
{
    let d_combo_atr = rt.scratch.ensure_combo_atr_exact(combo_count)?;
    d_combo_atr.copy_from(combos.atr_row_index).map_err(|e| e.to_string())?;
}
{
    let d_combo_start = rt.scratch.ensure_combo_start_exact(combo_count)?;
    d_combo_start.copy_from(combos.start_t).map_err(|e| e.to_string())?;
}
{
    let d_combo_end = rt.scratch.ensure_combo_end_exact(combo_count)?;
    d_combo_end.copy_from(combos.end_t).map_err(|e| e.to_string())?;
}
let d_out = rt.scratch.ensure_out_exact(out_len)?;
```

### Cache combo tags across calls

`combo_tags` is deterministic: `1..=combo_count`.

```rust
fn ensure_combo_tags(
    rt: &mut VemrCudaRuntime,
    combo_count: usize,
) -> Result<&DeviceBuffer<i32>, String> {
    if rt.scratch.combo_tags_len != combo_count {
        rt.scratch.combo_tags_host.clear();
        rt.scratch.combo_tags_host.reserve(combo_count);

        for idx in 0..combo_count {
            rt.scratch.combo_tags_host.push(
                i32::try_from(idx + 1)
                    .map_err(|_| "VEMR GPU combo index exceeds i32 range".to_string())?,
            );
        }

        let d_combo_tags = rt.scratch.ensure_combo_tags_exact(combo_count)?;
        d_combo_tags
            .copy_from(rt.scratch.combo_tags_host.as_slice())
            .map_err(|e| e.to_string())?;

        rt.scratch.combo_tags_len = combo_count;
    }

    Ok(rt.scratch.combo_tags.as_ref().unwrap())
}
```

### Result

This removes a large amount of repeated allocation churn from the warm VEMR path without changing the math.

---

## 3.6 `vemr_gpu_kernel.rs` — host readback copy-elision with `MaybeUninit`

### Why this is safe

These buffers are immediately and fully populated by `copy_to(...)`:

- full metrics readback
- top-k combo-tag readback
- top-k metric-row readback

That means a zero-filled `vec![...]` is not required.

### Preferred path

If your repo already has shared helpers in `alloc_policy.rs` for readback-into-uninit, use them.

### Local helper if needed

```rust
use std::mem::MaybeUninit;

fn alloc_uninit_host_vec<T>(len: usize) -> Vec<MaybeUninit<T>> {
    let mut out = Vec::with_capacity(len);
    unsafe { out.set_len(len); }
    out
}

fn assume_init_vec<T>(mut v: Vec<MaybeUninit<T>>) -> Vec<T> {
    let ptr = v.as_mut_ptr() as *mut T;
    let len = v.len();
    let cap = v.capacity();
    std::mem::forget(v);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

fn readback_f32(buf: &DeviceBuffer<f32>, len: usize) -> Result<Vec<f32>, String> {
    let mut raw = alloc_uninit_host_vec::<f32>(len);
    let dst = unsafe {
        std::slice::from_raw_parts_mut(raw.as_mut_ptr() as *mut f32, len)
    };
    buf.copy_to(dst).map_err(|e| e.to_string())?;
    Ok(assume_init_vec(raw))
}

fn readback_i32(buf: &DeviceBuffer<i32>, len: usize) -> Result<Vec<i32>, String> {
    let mut raw = alloc_uninit_host_vec::<i32>(len);
    let dst = unsafe {
        std::slice::from_raw_parts_mut(raw.as_mut_ptr() as *mut i32, len)
    };
    buf.copy_to(dst).map_err(|e| e.to_string())?;
    Ok(assume_init_vec(raw))
}
```

### Replace current readbacks

#### Current

```rust
let mut host_combo_tags = vec![0i32; topk_select];
let mut host_top_metrics = vec![0.0f32; topk_select.saturating_mul(VEMR_METRICS_STRIDE)];
d_top_combo.copy_to(&mut host_combo_tags).map_err(|e| e.to_string())?;
d_top_metrics.copy_to(&mut host_top_metrics).map_err(|e| e.to_string())?;
```

#### Proposed

```rust
let host_combo_tags = readback_i32(&d_top_combo, topk_select)?;
let host_top_metrics =
    readback_f32(&d_top_metrics, topk_select.saturating_mul(VEMR_METRICS_STRIDE))?;
```

#### Current full metrics readback

```rust
let mut host_out = vec![0.0f32; out_len];
d_out.copy_to(&mut host_out).map_err(|e| e.to_string())?;
```

#### Proposed

```rust
let host_out = readback_f32(d_out, out_len)?;
```

### Important guardrail

Use this only where every element is guaranteed to be written before any read.

---

## 3.7 `vemr_gpu_kernel.rs` — cache device launch capability metadata

This is a smaller safe follow-up after the larger staging fixes.

### Suggested direction

Add cached device launch caps to `VemrCudaRuntime`:

```rust
struct VemrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    scratch: VemrCudaScratch,
    device_caps: crate::gpu_backtest_kernel::DeviceLaunchCaps,
}
```

Initialize it once in `new(...)`.

Then reuse `rt.device_caps` instead of:

```rust
let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
let device_caps = detect_device_launch_caps(device);
```

Keep kernel-cap detection per function for the first pass unless you already have a pattern for caching it elsewhere.

---

## 3.8 `gpu_native_expansion_opt.rs` — add real VEMR optimizer coverage

This file needs VEMR coverage before deeper profiling.

### Imports

Add `VemrStrategyParams`:

```rust
use ta_app_core::{
    Backend, BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeCpuRequest,
    CompositeSignalMode, KqsrStrategyParams, KvdfStrategyParams, VemrStrategyParams,
};
```

### Add a VEMR-shaped candle generator

The existing file uses strategy-shaped candle builders. VEMR should get the same treatment.

```rust
fn make_vemr_mean_reversion_candles(len: usize) -> Candles {
    let len = len.max(320);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64 * 60_000);

        let angle = i as f64 * 0.14;
        let anchor = 100.0 + ((i / 80) as f64 * 0.6).sin() * 1.4;
        let excursion = angle.sin() * 3.1 + (angle * 0.37).cos() * 1.2;
        let shock = match i % 160 {
            28..=34 => -2.0,
            90..=97 => 2.3,
            _ => 0.0,
        };

        let px = anchor + excursion + shock;
        open.push(px * 0.999);
        high.push(px * 1.0045);
        low.push(px * 0.9955);
        close.push(px);
        vol.push(1_200.0 + (i % 19) as f64 * 35.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

### Add VEMR request builders

Baseline request:

```rust
fn make_vemr_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.vemr = Some(VemrStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}
```

Top-k path:

```rust
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
```

Full-readback / heatmap path:

```rust
fn make_vemr_req_full(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = make_vemr_req(backend, fast_range, slow_range);
    req.top_k = Some(20);
    req.include_all = Some(true);
    req.heatmap_bins = Some(64);
    req
}
```

### Add CPU bench

```rust
fn bench_vemr_cpu(c: &mut Criterion) {
    if !bench_target_enabled("vemr") {
        return;
    }
    let candles = make_vemr_mean_reversion_candles(200_000);
    bench_cpu_cases(c, "vemr/cpu/200k", &candles, make_vemr_req);
}
```

### Add GPU benches

Top-k route:

```rust
fn bench_vemr_gpu_kernel_topk(c: &mut Criterion) {
    if !bench_target_enabled("vemr") {
        return;
    }
    let candles = make_vemr_mean_reversion_candles(200_000);
    bench_gpu_cases(
        c,
        "vemr/gpu_kernel/topk/200k",
        "vemr/gpu_kernel/topk/200k",
        &candles,
        make_vemr_req_topk,
    );
}
```

Full-readback route:

```rust
fn bench_vemr_gpu_kernel_full(c: &mut Criterion) {
    if !bench_target_enabled("vemr") {
        return;
    }
    let candles = make_vemr_mean_reversion_candles(200_000);
    bench_gpu_cases(
        c,
        "vemr/gpu_kernel/full/200k",
        "vemr/gpu_kernel/full/200k",
        &candles,
        make_vemr_req_full,
    );
}
```

### Update `criterion_group!`

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
    bench_vemr_cpu,
    bench_vemr_gpu_kernel_topk,
    bench_vemr_gpu_kernel_full
);
```

### Optional branch-diverse VEMR candidate shape

If you want a more representative VEMR-specific optimizer benchmark, add a second VEMR builder that varies branch-driving flags without introducing route blockers:

- `require_band_touch`
- `confirm_bars`
- `allow_flip`
- `trade_on_next_bar`

That should be a second benchmark shape, not the only one, so your baseline remains stable.

---

## 3.9 `composite_validation_workflow.rs` — tighten VEMR validation benchmark fidelity

### Add CUDA availability guard

The GPU optimizer bench already guards on CUDA availability; the GPU validation bench should do the same.

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_composite_validation_gpu_workflow(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    // existing code...
}
```

### Add a VEMR-shaped validation candle generator

The current `make_synth_candles` is trend-ish and generic. Add a VEMR-specific generator and use it for VEMR cases.

```rust
fn make_vemr_validation_candles(len: usize) -> Candles {
    let len = len.max(320);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);

        let angle = i as f64 * 0.12;
        let anchor = 100.0 + ((i / 120) as f64 * 0.5).sin() * 1.0;
        let excursion = angle.sin() * 2.8 + (angle * 0.29).cos() * 1.1;
        let shock = match i % 180 {
            36..=42 => -1.9,
            104..=110 => 2.2,
            _ => 0.0,
        };

        let px = anchor + excursion + shock;
        open.push(px * 0.999);
        high.push(px * 1.004);
        low.push(px * 0.996);
        close.push(px);
        vol.push(1_050.0 + (i % 17) as f64 * 22.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

### Update `bench_profile` signature

Add an explicit GPU expectation flag and use the VEMR candle shape when `name == "vemr"`.

```rust
let mut bench_profile = |
    name: &str,
    candles_len: usize,
    candidates: Vec<CompositeParams>,
    expect_gpu: bool
| {
    let candidate_count = candidates.len();
    let candles = if name == "vemr" {
        make_vemr_validation_candles(candles_len)
    } else {
        make_synth_candles(candles_len)
    };

    let cancel = AtomicBool::new(false);

    // existing group setup...

    let route_probe = ta_app_core::run_composite_validation_blocking_with_candles(
        base.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("gpu profile validation route probe");

    println!(
        "benchmark route composite_validate/gpu_kernel/{}/{}/{}candidates: backend_used={:?} backend_route={} fallback={}",
        name,
        candles_len,
        candidate_count,
        route_probe.backend_used,
        route_probe.backend_route.as_deref().unwrap_or("<none>"),
        route_probe
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("<none>")
    );

    if expect_gpu {
        assert!(
            matches!(route_probe.backend_used, ta_app_core::BackendUsed::Gpu),
            "expected GPU backend for composite_validate/gpu_kernel/{}/{}/{}candidates, got backend={:?}, route={:?}, fallback={:?}",
            name,
            candles_len,
            candidate_count,
            route_probe.backend_used,
            route_probe.backend_route,
            route_probe.backend_fallback_reason
        );
    }

    group.bench_function("wf_reopt_cscv_spa", |b| {
        b.iter(|| {
            let res = ta_app_core::run_composite_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("gpu profile wf+reopt+cscv+spa validate");

            if expect_gpu {
                assert!(
                    matches!(res.backend_used, ta_app_core::BackendUsed::Gpu),
                    "expected GPU backend for composite_validate/gpu_kernel/{}/{}/{}candidates, got backend={:?}, route={:?}, fallback={:?}",
                    name,
                    candles_len,
                    candidate_count,
                    res.backend_used,
                    res.backend_route,
                    res.backend_fallback_reason
                );
            }
        })
    });

    group.finish();
};
```

### Update VEMR invocations

```rust
bench_profile("base_composite", 50_000, make_candidates(8), false);
bench_profile("asr", 50_000, make_asr_candidates(8), true);
bench_profile("vcc", 50_000, make_vcc_candidates(8), true);
bench_profile("vemr", 50_000, make_vemr_candidates(8), true);
bench_profile("vemr", 50_000, make_vemr_candidates(32), true);
bench_profile("vemr", 100_000, make_vemr_candidates(32), true);
bench_profile("rocb", 50_000, make_rocb_candidates(8), true);
bench_profile("mkcs", 50_000, make_mkcs_candidates(8), true);
bench_profile("kqsr", 50_000, make_kqsr_candidates(8), true);
bench_profile("carm", 50_000, make_carm_candidates(8), true);
bench_profile("kvdf", 50_000, make_kvdf_candidates(8), true);
bench_profile("cdmc", 50_000, make_cdmc_candidates(8), true);
bench_profile("carm", 100_000, make_carm_candidates(32), true);
bench_profile("cdmc", 100_000, make_cdmc_candidates(32), true);
```

### Why add both `50k/32` and `100k/32`

- `50k / 8` preserves the current benchmark.
- `50k / 32` exposes candidate-count scaling without bar-count scaling.
- `100k / 32` exposes both.

For VEMR-specific validation throughput, those are the minimum useful scaling checkpoints.

---

# 4. Higher-risk optimization opportunities

These should come after the safe plan above.

## 4.1 Fuse log-return generation into the main VEMR metrics kernel

### Upside

- eliminates one launch,
- eliminates one intermediate buffer,
- reduces control overhead further.

### Risk

- larger kernel surface,
- more parity/debug complexity,
- harder to isolate if metrics drift.

### Recommendation

Only consider this after:

- bundled Bollinger sweep,
- resident `close` / `lr`,
- scratch reuse,
- VEMR benchmark coverage

have already been implemented and measured.

## 4.2 Cache / reuse indicator wrapper objects in `vemr.rs`

### Upside

If `CudaZscore::new`, `CudaBollingerBands::new`, and `CudaAtr::new` are non-trivial, caching them could shave warm-path overhead.

### Risk

- wrapper ownership/lifetime complexity,
- possible thread/device affinity complexity,
- missing attached context.

### Recommendation

Profile constructor cost first; do not front-load this change.

## 4.3 Change the top-k gather interface so combo tags are implicit

### Upside

Could remove:

- host combo-tag generation
- device combo-tag upload
- some scratch usage

### Risk

Requires CUDA kernel signature/body changes not present in the attached files.

### Recommendation

Treat this as a later kernel-surface cleanup, not a first-wave optimization.

## 4.4 Expand direct z-score specialization beyond the default config

### Upside

Can reduce performance cliffs for non-default `z_ma_type` / `z_devtype` cases.

### Risk

- more specialized code paths,
- more parity surface,
- unclear real-world payoff without benchmark evidence.

### Recommendation

Only add another specialization if your new VEMR benchmarks show a real workload need.

---

# 5. Benchmark and perf-gate review

## 5.1 What should be measured first

Do **not** jump straight to Nsight. The review map is explicit that existing perf-gate / benchmark surfaces should be the first evidence path.

For VEMR, the correct order is:

1. **Criterion optimizer**
   - CPU baseline
   - GPU top-k/no-heatmap
   - GPU full-readback/heatmap
2. **Criterion validation**
   - 50k / 8
   - 50k / 32
   - 100k / 32
3. **Warm app-path** (adjacent app-path harness not attached)
4. **Nsight** only after those first three show a stable target

## 5.2 Benchmark-shape corrections required

### Optimizer (`gpu_native_expansion_opt.rs`)

You need **two** VEMR GPU benchmark shapes, not one:

- `topk/no_heatmap`
- `full/heatmap`

Otherwise you cannot tell whether a VEMR GPU speedup is coming from:

- the actual metrics path,
- the device-topk reduction,
- or a full-readback shape that may dominate small jobs.

### Validation (`composite_validation_workflow.rs`)

You need:

- explicit GPU-route assertions for VEMR,
- VEMR-shaped candles,
- larger VEMR scaling cases.

Otherwise the benchmark can silently turn into a CPU benchmark or a non-representative synthetic case.

## 5.3 Success criteria for the first implementation wave

Use concrete structural success criteria, not vague “faster” goals.

### After phase 1, these should be true

- exactly **one** Bollinger sweep launch/build per VEMR request
- zero host→device `close` upload when device `close` is already available
- zero on-demand log-return recompute when device `lr` is already available
- zero repeated device allocations on warm repeated VEMR eval with identical shapes
- VEMR optimizer benches exist in Criterion
- VEMR validation benches assert GPU route

### After phase 2, these should also be true

- host readback buffers do not zero-fill before `copy_to(...)`
- VEMR validation includes at least `50k/32` and `100k/32`
- VEMR benchmark candles are strategy-shaped instead of only generic synthetic drift

## 5.4 What not to over-interpret

- A speedup from the VEMR top-k benchmark does **not** automatically mean VEMR full-readback workloads improved.
- A speedup on generic synthetic candles does **not** automatically translate to mean-reversion-shaped workloads.
- A speedup in Criterion does **not** automatically imply warm app-path improvement if orchestration and data residency are unchanged.

---

# 6. Test review

## 6.1 Unit tests to add in `vemr.rs`

### A. `build_vemr_desired_positions` branch coverage

Add targeted tests for:

- `confirm_bars = 0` vs `confirm_bars > 0`
- `require_band_touch = true/false`
- `allow_flip = true/false`
- `trade_on_next_bar = true/false`
- `long_only = true`

Skeleton:

```rust
#[test]
fn build_vemr_desired_positions_respects_confirm_bars_and_band_touch() {
    let close = vec![100.0, 98.0, 95.0, 96.0, 99.0, 101.0];
    let fast = VemrFastSignals {
        zscore: vec![f64::NAN, -2.2, -2.1, -1.5, 0.7, 2.2],
        bb_upper: vec![f64::NAN, 101.0, 100.0, 99.5, 100.0, 100.5],
        bb_middle: vec![f64::NAN, 99.5, 98.5, 98.0, 98.5, 99.0],
        bb_lower: vec![f64::NAN, 98.5, 95.5, 95.0, 96.0, 97.0],
        first_valid: 1,
    };
    let risk = VemrRiskSignals {
        atr: vec![f64::NAN, 1.0, 1.0, 1.0, 1.0, 1.0],
        first_valid: 1,
    };

    let mut cfg = VemrStrategyParams::default();
    cfg.confirm_bars = 1;
    cfg.require_band_touch = true;

    let strategy = StrategyConfig::default();
    let desired = build_vemr_desired_positions(&close, &fast, &risk, 1, &strategy, &cfg);

    // Fill this with the exact expected sequence from the CPU reference path.
    assert_eq!(desired.len(), close.len());
}
```

### B. Period mapping tests

You already have one positive mapping test. Add cases for:

- mapped fast disabled
- mapped slow disabled
- explicit periods forced to at least 1

## 6.2 Tests to add in `vemr_gpu_kernel.rs`

### A. Input validation tests

Test the new host/device `close` selection rules:

- host only valid
- device only valid
- both absent invalid
- host length mismatch invalid

### B. Readback helper tests

If you add local `MaybeUninit` readback helpers, add small unit tests for the helper conversion functions so their invariants are documented.

## 6.3 Integration / parity tests to add in adjacent orchestration code

These require missing adjacent files, so they cannot be fully written here, but they should be added next to the VEMR optimization/validation route that uses the kernel.

### A. CPU vs GPU metric parity

Use a small fixed VEMR candidate batch and compare CPU and GPU metrics within tolerances.

### B. Threshold-adjacent parity

Construct prices and indicators that sit very near:

- `z_entry`
- `z_exit`
- band-touch edges
- ATR stop distance

This is where `f64` vs `f32` drift is most likely to show up.

### C. Route-blocker tests

Add explicit tests that VEMR GPU route blocks when:

- execution overlays are enabled
- stop loss is enabled
- take profit is enabled

Those blockers are present in the attached kernel wrapper and deserve test coverage.

---

# 7. Suggested improvement plan

## Phase 0 — baseline measurement before edits

Do this before touching code so you can prove the payoff.

### Baseline runs

- `gpu_native_expansion_opt.rs`
  - add temporary VEMR benches first if needed just to capture a baseline
- `composite_validation_workflow.rs`
  - existing `vemr 50k / 8`
- warm app-path
  - adjacent harness not attached; run from your existing app-path benchmark if available

### Record

- backend used
- backend route
- fallback reason
- bars
- candidate count
- top-k/include-all/heatmap shape
- whether `close` was host or device resident
- whether `lr` was precomputed or per-call

## Phase 1 — biggest VEMR wins

Implement in this order:

1. `vemr.rs`
   - add bundled Bollinger sweep helper
2. adjacent VEMR orchestration (not attached)
   - replace three separate Bollinger calls with one bundled call
3. `vemr_gpu_kernel.rs`
   - extend input struct for optional device `close` / device `lr`
4. `vemr_gpu_kernel.rs`
   - add scratch reuse for `close`, `lr`, combo buffers, metrics buffer
5. `gpu_native_expansion_opt.rs`
   - add real VEMR optimizer benches
6. `composite_validation_workflow.rs`
   - add VEMR GPU assertions and larger VEMR cases

## Phase 2 — copy-elision / minor hot-path cleanup

1. `vemr_gpu_kernel.rs`
   - replace zero-filled readback buffers with uninitialized readback helper
2. `vemr.rs`
   - cache indicator keys/ids
   - replace tiny heap `Vec`s with arrays
3. `vemr.rs`
   - optional prefix-init / `MaybeUninit` desired-position builder if profiling says it matters

## Phase 3 — parity and route hardening

1. add CPU/GPU parity tests
2. add threshold-adjacent parity tests
3. add route-blocker tests
4. make VEMR benchmark failures loud when GPU unexpectedly falls back

## Phase 4 — only if measurement justifies it

1. cache indicator wrappers
2. cache launch metadata more aggressively
3. fuse log-return generation into the main kernel
4. consider broader z-score specialization

---

# 8. File-by-file edit checklist

## 8.1 `vemr.rs`

### Required edits

- add `VemrBollingerSweepDevice`
- add `compute_vemr_bollinger_sweeps_cuda_device(...)`
- keep compatibility wrappers for upper/middle/lower if other code still references them
- add `OnceLock` indicator key/id cache
- replace tiny `Vec` param lists with arrays
- optionally convert `build_vemr_desired_positions` to prefix-init / overwrite-only allocation

### Notes

- the bundled Bollinger helper is the most important edit in this file
- the actual benefit requires updating the missing adjacent callsite

## 8.2 `vemr_gpu_kernel.rs`

### Required edits

- expand `VemrGpuDeviceSignalMatrices` with:
  - `close_host_f32: Option<&[f32]>`
  - `close_device_ptr: Option<u64>`
  - `lr_device_ptr: Option<u64>`
- update `validate_inputs(...)`
- change `launch_compute_log_returns_f32(...)` to accept a raw device pointer
- add `VemrCudaScratch`
- reuse `close`, `lr`, combo, metrics, and top-k buffers
- cache combo tags
- replace zero-filled readback buffers with uninitialized readback helper or shared alloc helper

### Notes

- these edits are where most unnecessary memory copies are currently concentrated

## 8.3 `gpu_native_expansion_opt.rs`

### Required edits

- import `VemrStrategyParams`
- add VEMR candle generator
- add:
  - `make_vemr_req`
  - `make_vemr_req_topk`
  - `make_vemr_req_full`
- add:
  - `bench_vemr_cpu`
  - `bench_vemr_gpu_kernel_topk`
  - `bench_vemr_gpu_kernel_full`
- update `criterion_group!`

### Notes

- do not reuse the `base_req` shape blindly for VEMR GPU if you want to measure device top-k

## 8.4 `composite_validation_workflow.rs`

### Required edits

- add CUDA availability guard for the GPU validation bench
- add VEMR-shaped validation candles
- update `bench_profile` to accept `expect_gpu`
- assert GPU route for VEMR in both route probe and benchmark iteration
- add `vemr 50k/32`
- add `vemr 100k/32`

### Optional edits

- add a second VEMR candidate builder that varies branch-driving flags for a more representative strategy-local workload

---

# 9. Detailed implementation checklist

Use this as the bottom-up tracking sheet.

## Phase 0 — baseline

- [ ] Record current VEMR validation benchmark numbers for `50k / 8`
- [ ] Add temporary local logging to identify whether `close` is host-staged or device-resident today
- [ ] Record whether current VEMR validation route ever falls back unexpectedly
- [ ] Record current allocation counts / signs of repeated device allocations if you already have instrumentation

## Phase 1 — `vemr.rs`

- [ ] Add `VemrBollingerSweepDevice`
- [ ] Add `compute_vemr_bollinger_sweeps_cuda_device(...)`
- [ ] Keep compatibility wrappers for upper/middle/lower or remove them after all callsites are migrated
- [ ] Identify the missing adjacent VEMR callsite that currently asks for upper/middle/lower separately
- [ ] Replace that three-call pattern with the new bundled helper
- [ ] Re-run VEMR optimizer/validation baseline after this change only
- [ ] Confirm only one Bollinger sweep is executed per VEMR request

## Phase 1 — `vemr_gpu_kernel.rs`

- [ ] Expand `VemrGpuDeviceSignalMatrices` to carry optional host/device `close`
- [ ] Add optional device `lr` pointer to `VemrGpuDeviceSignalMatrices`
- [ ] Update `validate_inputs(...)` for the new close-input modes
- [ ] Change `launch_compute_log_returns_f32(...)` to accept a raw device pointer
- [ ] Add `VemrCudaScratch`
- [ ] Reuse `close` scratch buffer
- [ ] Reuse `lr` scratch buffer
- [ ] Reuse `combo_z`
- [ ] Reuse `combo_bb`
- [ ] Reuse `combo_atr`
- [ ] Reuse `combo_start`
- [ ] Reuse `combo_end`
- [ ] Reuse `out`
- [ ] Reuse `combo_tags`
- [ ] Reuse `cand_a`
- [ ] Reuse `cand_b`
- [ ] Reuse `top_combo`
- [ ] Reuse `top_dummy`
- [ ] Reuse `top_metrics`
- [ ] Cache combo tags across identical `combo_count`
- [ ] Wire the new `close_device_ptr` path from the missing adjacent orchestrator
- [ ] Wire the new `lr_device_ptr` path if your orchestration already has a reusable log-return buffer
- [ ] Re-run warm repeated VEMR eval and confirm allocations disappear on stable shapes

## Phase 1 — `gpu_native_expansion_opt.rs`

- [ ] Import `VemrStrategyParams`
- [ ] Add `make_vemr_mean_reversion_candles(...)`
- [ ] Add `make_vemr_req(...)`
- [ ] Add `make_vemr_req_topk(...)`
- [ ] Add `make_vemr_req_full(...)`
- [ ] Add `bench_vemr_cpu(...)`
- [ ] Add `bench_vemr_gpu_kernel_topk(...)`
- [ ] Add `bench_vemr_gpu_kernel_full(...)`
- [ ] Update `criterion_group!`
- [ ] Run the new VEMR CPU benchmark
- [ ] Run the new VEMR GPU top-k benchmark
- [ ] Run the new VEMR GPU full-readback benchmark
- [ ] Compare top-k vs full-readback behavior explicitly

## Phase 1 — `composite_validation_workflow.rs`

- [ ] Add CUDA availability guard to the GPU validation bench
- [ ] Add `make_vemr_validation_candles(...)`
- [ ] Update `bench_profile(...)` to accept `expect_gpu`
- [ ] Assert GPU route in route probe for VEMR
- [ ] Assert GPU route inside benchmark iteration for VEMR
- [ ] Add `vemr 50k / 32`
- [ ] Add `vemr 100k / 32`
- [ ] Run all VEMR validation cases and record route/fallback output
- [ ] Confirm no silent CPU fallback remains for VEMR benchmark cases

## Phase 2 — copy elision and minor cleanup

- [ ] Prefer shared `alloc_policy.rs` helper for readback-into-uninit if available
- [ ] Otherwise add local `MaybeUninit` readback helper in `vemr_gpu_kernel.rs`
- [ ] Replace zero-filled `host_combo_tags`
- [ ] Replace zero-filled `host_top_metrics`
- [ ] Replace zero-filled `host_out`
- [ ] Add `OnceLock` indicator key/id cache in `vemr.rs`
- [ ] Replace tiny `Vec` param lists with arrays
- [ ] Benchmark again after these cleanup changes
- [ ] Only then decide whether `build_vemr_desired_positions` needs prefix-init / `MaybeUninit`

## Phase 3 — tests and parity

- [ ] Add `build_vemr_desired_positions` test for `confirm_bars`
- [ ] Add `build_vemr_desired_positions` test for `require_band_touch`
- [ ] Add `build_vemr_desired_positions` test for `allow_flip`
- [ ] Add `build_vemr_desired_positions` test for `trade_on_next_bar`
- [ ] Add input-validation tests for new host/device close modes in `vemr_gpu_kernel.rs`
- [ ] Add CPU vs GPU metric parity integration test for VEMR
- [ ] Add threshold-adjacent parity test around `z_entry` / `z_exit`
- [ ] Add route-blocker tests for overlays / stop-loss / take-profit
- [ ] Re-run VEMR benchmarks after adding tests to ensure no accidental perf regression

## Phase 4 — optional / measurement-gated

- [ ] Profile wrapper-constructor cost for `CudaZscore`, `CudaBollingerBands`, `CudaAtr`
- [ ] Cache wrapper/runtime objects if constructor cost is real
- [ ] Cache launch metadata more aggressively if control overhead is still visible
- [ ] Consider fusing log-return generation into the main VEMR kernel only if staging reuse is still insufficient
- [ ] Benchmark non-default z-score configs
- [ ] Only then consider broadening direct z-score specialization

## Sign-off checks

- [ ] VEMR Bollinger sweeps are bundled and not recomputed three times
- [ ] VEMR GPU eval can consume device `close`
- [ ] VEMR GPU eval can consume device `lr`
- [ ] Warm repeated VEMR eval reuses scratch buffers
- [ ] Host readback buffers no longer zero-fill before full overwrite
- [ ] VEMR optimizer Criterion coverage exists
- [ ] VEMR validation Criterion coverage asserts GPU route
- [ ] VEMR has at least one VEMR-shaped optimizer candle generator
- [ ] VEMR has at least one VEMR-shaped validation candle generator
- [ ] CPU/GPU parity tests cover threshold edges
- [ ] Route-blocker behavior is tested
- [ ] Post-change benchmark numbers are recorded side-by-side with baseline
