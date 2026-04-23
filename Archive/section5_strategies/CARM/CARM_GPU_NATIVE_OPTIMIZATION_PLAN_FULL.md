# CARM GPU-native optimization and validation plan

## Scope and framing

This document is a CARM-only implementation plan built from the `## 5. Strategy-local GPU-native optimization and validation routes` bundle in `gpt54_feature_review_map.md`.

It follows the bundle’s requested review shape:

- **strategy-local hot-path review**
- **CPU/GPU payoff review**
- **staging/fallback review**
- **per-strategy benchmark review**

It covers the provided files:

- `carm_gpu_kernel.rs`
- `carm.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

The plan is intentionally optimization-first, parity-preserving, and benchmark-aware.

## Important constraint

The Rust-side CARM GPU entrypoint is provided, but the actual CUDA kernel implementation for `carm_backtest_metrics_f32` is **not** included in the files you attached. That means:

- the Rust-side staging and runtime changes below are high-confidence and directly grounded in the provided code
- low-level CUDA body changes are included as a **conditional follow-up section**, not as confirmed findings

Where a snippet depends on APIs not fully visible in the provided files, I call that out explicitly.

## Executive summary

For the CARM strategy, the biggest near-term performance wins are not speculative math changes inside the unseen kernel body. They are the host/runtime issues directly visible in the attached files:

1. `carm_gpu_kernel.rs` re-uploads `close_f32`, re-allocates combo/output buffers, recomputes log-returns, and allocates a zero-filled host metrics buffer on every call.
2. `carm.rs` has a CUDA-preferred path that silently falls back to CPU, and its `compute_indicator_host_f32(...)` CPU fallback currently goes through `compute_indicator_f64(...)` and then casts back to `f32`, which adds unnecessary allocations and conversions.
3. `gpu_native_expansion_opt.rs` is strong on GPU-route assertions, but the CARM request shaping can be made more faithful to the validation bench and more useful for steady-state measurement.
4. `composite_validation_workflow.rs` labels GPU validation groups as GPU, but does not assert `BackendUsed::Gpu` for the measured runs, which allows silent CPU fallback to contaminate CARM GPU validation numbers.

If you want the fastest path to measurable improvement, implement the changes in this order:

1. **Reuse device buffers and host readback storage in `carm_gpu_kernel.rs`.**
2. **Skip close upload and log-return recomputation when the input close slice is unchanged.**
3. **Replace `f64 -> f32` fallback conversion in `carm.rs` with direct CPU-to-`f32` conversion.**
4. **Add a native `CarmSignalsF32` path so the GPU route can stay off the `f64` surface.**
5. **Make all GPU-labeled CARM validation benches fail fast on fallback.**
6. **Add dedicated CARM validation groups that reflect CARM-like price action and candidate diversity.**
7. **Only after that, consider bigger structural work like device-side top-k or a true multi-output OTTO path.**

---

## What the current files show

### `carm_gpu_kernel.rs`

The current runtime path does the following inside `eval_carm_metrics_gpu_kernel_device_rows(...)`:

- validates shapes and blocks overlays / stop-loss / take-profit
- allocates and uploads `close_f32` every call
- allocates `d_lr` every call
- launches `compute_log_returns_f32` every call
- uploads four combo arrays every call
- allocates `d_out` every call
- synchronizes after the kernel
- allocates `vec![0.0f32; out_len]` every call
- copies the full output to host
- decodes every metrics row on CPU

Relevant visible areas:

- `carm_gpu_kernel.rs:209-221`
- `carm_gpu_kernel.rs:223-320`

### `carm.rs`

The current CARM signal path has these important traits:

- CUDA is preferred when the dynamic indicator descriptor says CUDA is supported and `prepared_cuda_inputs` exists.
- If the CUDA attempt fails, fallback is silent.
- `compute_indicator_host_f32(...)` falls back by calling `compute_indicator_f64(...)` and then casting `Vec<f64>` to `Vec<f32>`.
- OTTO is requested twice, once for `hott` and once for `lott`.
- The file already exposes `*_f32` helpers for mixed, ROCR, and OTTO signals, but it does **not** expose a full `CarmSignalsF32` aggregate.

Relevant visible areas:

- `carm.rs:293-385`
- `carm.rs:478-565`
- `carm.rs:615-749`

### `gpu_native_expansion_opt.rs`

The CARM optimization benchmark already does an important thing correctly:

- it probes the route and asserts that a GPU-labeled run actually used the GPU

That is good and should remain.

However:

- CARM-specific request shaping is still inherited from `base_req(...)`, which uses `fast_ma_types = ["sma"]` and `slow_ma_types = ["ema"]`
- the validation bench uses `ema` / `ema` for CARM
- only the smallest case is explicitly route-probed before the group; larger shapes are not warmed independently before measurement
- `heatmap_bins: Some(64)` may be unnecessary if the benchmark goal is route timing rather than product-output timing

Relevant visible areas:

- `gpu_native_expansion_opt.rs:16-52`
- `gpu_native_expansion_opt.rs:185-195`
- `gpu_native_expansion_opt.rs:253-325`

### `composite_validation_workflow.rs`

The GPU validation workflow prints the backend route, but does not assert it during measured runs.

That means a group named `composite_validate/gpu_kernel/carm/...` can silently bench CPU fallback.

Also:

- the generic validation candle generator is used for all strategies
- there is no dedicated, heavier CARM GPU validation group analogous to the top-level ASR GPU validation group
- current `make_carm_candidates(...)` varies fast/slow lengths but not much of CARM’s own internal parameter surface

Relevant visible areas:

- `composite_validation_workflow.rs:262-294`
- `composite_validation_workflow.rs:723-823`

---

# 1. Strategy-local hot-path review

## Primary hot path: `carm_gpu_kernel.rs`

### Current hot-path problems

#### 1. Repeated close upload and repeated log-return computation

The same candle series is often reused across many optimization or validation calls. Right now, every call does:

- `DeviceBuffer::from_slice(signals.close_f32)`
- `DeviceBuffer::<f32>::uninitialized(signals.cols)` for `d_lr`
- `launch_compute_log_returns_f32(...)`

That guarantees unnecessary H2D traffic and an extra kernel launch on steady-state runs.

#### 2. Per-call combo/output allocations

Each call allocates fresh device buffers for:

- `fast_row_index`
- `otto_row_index`
- `start_t`
- `end_t`
- output metrics

The combo arrays still need to be copied each call unless you change the upstream API, but the allocations do **not** need to happen each call.

#### 3. Per-call zero-initialized host readback buffer

`let mut host_metrics = vec![0.0f32; out_len];`

That forces a full host-side zero-fill before `d_out.copy_to(...)` overwrites every element.

For a POD type like `f32`, that initialization is wasted work.

#### 4. Full device-to-host metrics readback even when the caller may only need ranking / top-k

This is a larger structural item because the current function signature returns `Vec<Metrics>`. The full readback is therefore expected.

Still, it is worth calling out as the largest remaining memory-copy cost after the steady-state upload fixes.

---

## Code-edit plan for `carm_gpu_kernel.rs`

### A. Introduce reusable device buffers inside the runtime

This is the most important concrete change.

Add a small helper that owns a `DeviceBuffer<T>` and only reallocates when the requested length changes.

```rust
use cust::memory::{CopyDestination, DeviceBuffer, DeviceCopy};

struct ReusableDeviceBuffer<T: DeviceCopy> {
    buf: Option<DeviceBuffer<T>>,
    len: usize,
}

impl<T: DeviceCopy> Default for ReusableDeviceBuffer<T> {
    fn default() -> Self {
        Self { buf: None, len: 0 }
    }
}

impl<T: DeviceCopy> ReusableDeviceBuffer<T> {
    fn len(&self) -> usize {
        self.len
    }

    fn ensure_len(&mut self, len: usize) -> Result<(), String> {
        if self.len == len && self.buf.is_some() {
            return Ok(());
        }

        self.buf = if len == 0 {
            None
        } else {
            Some(unsafe { DeviceBuffer::<T>::uninitialized(len) }.map_err(|e| e.to_string())?)
        };
        self.len = len;
        Ok(())
    }

    fn copy_from_slice(&mut self, src: &[T]) -> Result<(), String> {
        self.ensure_len(src.len())?;
        if let Some(buf) = self.buf.as_mut() {
            buf.copy_from(src).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn as_ref(&self) -> &DeviceBuffer<T> {
        self.buf.as_ref().expect("device buffer not allocated")
    }

    fn as_mut(&mut self) -> &mut DeviceBuffer<T> {
        self.buf.as_mut().expect("device buffer not allocated")
    }
}
```

### B. Extend `CarmCudaRuntime` with reusable scratch and a cached close identity

```rust
struct CarmCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,

    d_close: ReusableDeviceBuffer<f32>,
    d_lr: ReusableDeviceBuffer<f32>,
    d_combo_fast: ReusableDeviceBuffer<i32>,
    d_combo_otto: ReusableDeviceBuffer<i32>,
    d_combo_start: ReusableDeviceBuffer<i32>,
    d_combo_end: ReusableDeviceBuffer<i32>,
    d_out: ReusableDeviceBuffer<f32>,

    host_metrics: Vec<f32>,

    cached_close_host_ptr: usize,
    cached_close_len: usize,
}
```

Initialize the fields in `new(...)`:

```rust
Ok(Self {
    context,
    module,
    stream,
    d_close: ReusableDeviceBuffer::default(),
    d_lr: ReusableDeviceBuffer::default(),
    d_combo_fast: ReusableDeviceBuffer::default(),
    d_combo_otto: ReusableDeviceBuffer::default(),
    d_combo_start: ReusableDeviceBuffer::default(),
    d_combo_end: ReusableDeviceBuffer::default(),
    d_out: ReusableDeviceBuffer::default(),
    host_metrics: Vec::new(),
    cached_close_host_ptr: 0,
    cached_close_len: 0,
})
```

### C. Change `launch_compute_log_returns_f32(...)` to take `&Module` and `&Stream`, not `&mut CarmCudaRuntime`

This reduces borrow friction and makes it much easier to use the runtime-owned reusable buffers.

```rust
fn launch_compute_log_returns_f32(
    module: &Module,
    stream: &Stream,
    device_id: u32,
    d_close: &DeviceBuffer<f32>,
    d_lr: &mut DeviceBuffer<f32>,
    len: usize,
) -> Result<(), String> {
    if len == 0 {
        return Ok(());
    }

    let kernel = module
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

    let mut close_ptr = d_close.as_device_ptr().as_raw();
    let mut len_i = i32::try_from(len)
        .map_err(|_| "CARM GPU close length exceeds i32 range".to_string())?;
    let mut lr_ptr = d_lr.as_device_ptr().as_raw();

    unsafe {
        let args: &mut [*mut c_void] = &mut [
            &mut close_ptr as *mut _ as *mut c_void,
            &mut len_i as *mut _ as *mut c_void,
            &mut lr_ptr as *mut _ as *mut c_void,
        ];
        stream
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

### D. Add a helper that skips close upload and log-return recomputation when the close series is unchanged

For immutable candle series in optimizer / validator workflows, pointer+length identity is a practical steady-state cache key.

```rust
impl CarmCudaRuntime {
    fn ensure_close_and_log_returns(
        &mut self,
        device_id: u32,
        close: &[f32],
    ) -> Result<(), String> {
        let same_close = self.cached_close_host_ptr == close.as_ptr() as usize
            && self.cached_close_len == close.len()
            && self.d_close.len() == close.len()
            && self.d_lr.len() == close.len();

        if same_close {
            return Ok(());
        }

        self.d_close.copy_from_slice(close)?;
        self.d_lr.ensure_len(close.len())?;
        launch_compute_log_returns_f32(
            &self.module,
            &self.stream,
            device_id,
            self.d_close.as_ref(),
            self.d_lr.as_mut(),
            close.len(),
        )?;

        self.cached_close_host_ptr = close.as_ptr() as usize;
        self.cached_close_len = close.len();
        Ok(())
    }
}
```

### E. Reuse the host readback buffer and avoid zero-initialization

For `f32`, it is reasonable to reserve capacity once and set the length unsafely just before `copy_to(...)`, because the copy fully overwrites the buffer before any read.

Add this helper:

```rust
impl CarmCudaRuntime {
    fn ensure_host_metrics_len(&mut self, len: usize) {
        if self.host_metrics.capacity() < len {
            self.host_metrics.reserve_exact(len - self.host_metrics.capacity());
        }
        unsafe {
            self.host_metrics.set_len(len);
        }
    }
}
```

**Guardrail:** only do this for plain-old-data readback types like `f32` or `i32`. Do **not** use this pattern for `Metrics` or other non-POD types.

### F. Update `eval_carm_metrics_gpu_kernel_device_rows(...)` to use the reusable buffers

Replace the current per-call allocation block with something like this:

```rust
rt.ensure_close_and_log_returns(device_id, signals.close_f32)?;

rt.d_combo_fast.copy_from_slice(combos.fast_row_index)?;
rt.d_combo_otto.copy_from_slice(combos.otto_row_index)?;
rt.d_combo_start.copy_from_slice(combos.start_t)?;
rt.d_combo_end.copy_from_slice(combos.end_t)?;
rt.d_out.ensure_len(out_len)?;
rt.ensure_host_metrics_len(out_len);

let d_close = rt.d_close.as_ref();
let d_lr = rt.d_lr.as_ref();
let d_combo_fast = rt.d_combo_fast.as_ref();
let d_combo_otto = rt.d_combo_otto.as_ref();
let d_combo_start = rt.d_combo_start.as_ref();
let d_combo_end = rt.d_combo_end.as_ref();
let d_out = rt.d_out.as_ref();

let mut close_ptr = d_close.as_device_ptr().as_raw();
let mut lr_ptr = d_lr.as_device_ptr().as_raw();
let mut combo_fast_ptr = d_combo_fast.as_device_ptr().as_raw();
let mut combo_otto_ptr = d_combo_otto.as_device_ptr().as_raw();
let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
let mut out_ptr = d_out.as_device_ptr().as_raw();

// launch kernel exactly as today

rt.stream.synchronize().map_err(|e| e.to_string())?;
d_out
    .copy_to(rt.host_metrics.as_mut_slice())
    .map_err(|e| e.to_string())?;

let mut out = Vec::with_capacity(combo_count);
for row in rt.host_metrics.chunks_exact(CARM_METRICS_STRIDE) {
    out.push(decode_metrics_row(row));
}
Ok(out)
```

### G. Optional: pre-size the decoded `Vec<Metrics>` without zero-initialization

The current `Vec::with_capacity(combo_count)` plus `push(...)` is already fine. I would **not** force an unsafe uninitialized `Vec<Metrics>` here because the decode is cheap compared with device staging and readback, and `Metrics` is not obviously POD.

### H. Add low-cost unit tests in `carm_gpu_kernel.rs`

Even without a real CUDA device path, you can add guard tests for the pure-Rust validation and blocker logic.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    static CLOSE: [f32; 8] = [100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0];
    static FAST_IDX: [i32; 2] = [0, 1];
    static OTTO_IDX: [i32; 2] = [0, 1];
    static START_T: [i32; 2] = [2, 2];
    static END_T: [i32; 2] = [7, 7];

    fn valid_signals() -> CarmGpuDeviceSignalMatrices<'static> {
        CarmGpuDeviceSignalMatrices {
            close_f32: &CLOSE,
            cols: CLOSE.len(),
            mixed_rows: 2,
            mixed_stride: CLOSE.len(),
            coppock_row_major_ptr: 1,
            ao_row_major_ptr: 2,
            rocr_row_major_ptr: 3,
            fast_rows: 2,
            fast_stride: CLOSE.len(),
            apo_row_major_ptr: 4,
            otto_hott_row_major_ptr: 5,
            otto_lott_row_major_ptr: 6,
            otto_rows: 2,
            otto_stride: CLOSE.len(),
        }
    }

    fn valid_combos() -> CarmGpuComboBatch<'static> {
        CarmGpuComboBatch {
            fast_row_index: &FAST_IDX,
            otto_row_index: &OTTO_IDX,
            start_t: &START_T,
            end_t: &END_T,
        }
    }

    #[test]
    fn validate_inputs_accepts_basic_valid_shapes() {
        assert!(validate_inputs(&valid_signals(), &valid_combos()).is_ok());
    }

    #[test]
    fn validate_inputs_rejects_mismatched_combo_lengths() {
        let combos = CarmGpuComboBatch {
            fast_row_index: &FAST_IDX,
            otto_row_index: &OTTO_IDX[..1],
            start_t: &START_T,
            end_t: &END_T,
        };
        assert!(validate_inputs(&valid_signals(), &combos).is_err());
    }
}
```

### Expected payoff from the `carm_gpu_kernel.rs` edits

#### High confidence

- lower steady-state latency for repeated runs on the same candles
- lower `cudaMalloc` / `cudaFree` churn
- lower H2D traffic on repeated same-series runs
- lower CPU-side allocation and memset pressure

#### Medium confidence

- improved stability of benchmark numbers because warm steady-state work is cleaner

#### Not solved yet

- full output readback cost
- unseen kernel-body inefficiencies

---

# 2. CPU/GPU payoff review

## Current payoff profile

CARM is a strategy where GPU payoff can be erased by host-side prep if you are not careful.

The visible erosion points are:

1. host close upload every call
2. log-return kernel launch every call even when the input series has not changed
3. silent fallback from CUDA indicator compute to CPU
4. CPU fallback path that currently allocates `Vec<f64>` before converting to `Vec<f32>`
5. possible duplicate OTTO work

## Recommended backend stance

### Keep CARM GPU-preferred when all of the following are true

- the same candle series is reused across many runs
- `prepared_cuda_inputs` exists and actually works
- candidate / combo counts are medium or large
- overlays / stop-loss / take-profit are not requested

### Prefer CPU or at least re-measure before forcing GPU when any of these are true

- tiny candidate counts
- one-off runs with no warm reuse
- missing prepared CUDA inputs
- repeated indicator CUDA fallback for one or more of CARM’s component indicators
- routes that need unsupported overlays or execution modifiers

## Concrete `carm.rs` edits to improve payoff

### A. Add direct CPU-to-`f32` conversion so fallback does not detour through `Vec<f64>`

Current path in `compute_indicator_host_f32(...)`:

- try CUDA HostF32
- on failure, call `compute_indicator_f64(...)`
- allocate `Vec<f64>`
- cast to `Vec<f32>`

You can replace that with a direct CPU conversion helper.

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

Then rewrite `compute_indicator_host_f32(...)` like this:

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
            let out = compute_dynamic_indicator_cuda_with_inputs(
                prepared,
                indicator_id,
                Some(output_id),
                params,
                Kernel::Auto,
                vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
            );
            if let Ok(values) = out {
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

This is a simple, safe improvement and avoids one full temporary `Vec<f64>` on every CPU fallback.

### B. Add a full `CarmSignalsF32` aggregate and a `compute_carm_signals_f32(...)` API

You already expose the `*_f32` building blocks. The missing piece is a single aggregate return type that lets the GPU path stay on `f32` surfaces end to end.

Add this near the other signal structs:

```rust
#[cfg(feature = "cuda")]
#[derive(Clone, Debug)]
pub struct CarmSignalsF32 {
    pub coppock: Vec<f32>,
    pub ao: Vec<f32>,
    pub rocr: Vec<f32>,
    pub apo: Vec<f32>,
    pub otto_hott: Vec<f32>,
    pub otto_lott: Vec<f32>,
    pub first_valid: usize,
}
```

Then add this function:

```rust
#[cfg(feature = "cuda")]
pub fn compute_carm_signals_f32(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: CarmResolvedPeriods,
    cfg: &CarmStrategyParams,
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CarmSignalsF32, String> {
    let mixed = compute_carm_mixed_signals_f32(
        candles,
        ma_source,
        backend,
        periods,
        prepared_cuda_inputs,
    )?;
    let rocr = compute_carm_rocr_signal_f32(
        candles,
        ma_source,
        backend,
        periods,
        prepared_cuda_inputs,
    )?;
    let otto = compute_carm_otto_signals_f32(
        candles,
        ma_source,
        backend,
        periods,
        cfg,
        prepared_cuda_inputs,
    )?;

    let rocr_first_valid = first_finite_idx_local_f32(&rocr)
        .ok_or_else(|| "CARM ROCR signal is all non-finite".to_string())?;

    let first_valid = mixed
        .first_valid
        .max(rocr_first_valid)
        .max(otto.first_valid);

    Ok(CarmSignalsF32 {
        coppock: mixed.coppock,
        ao: mixed.ao,
        rocr,
        apo: mixed.apo,
        otto_hott: otto.hott,
        otto_lott: otto.lott,
        first_valid,
    })
}
```

### Why this matters

It gives the CARM GPU route a single obvious `f32` signal-prep API. That reduces the chance that upstream code accidentally grabs `f64` signals and then converts or restages them.

### C. Add a traced variant for indicator fallback visibility

This is optional but strongly recommended. It lets you preserve the existing public API while exposing route metadata for benches and diagnostics.

```rust
#[cfg(feature = "cuda")]
#[derive(Clone, Debug, Default)]
struct CarmIndicatorRouteTrace {
    preferred_gpu: bool,
    attempted_cuda: bool,
    used_cuda: bool,
    fallback_reason: Option<String>,
}
```

Then create a traced helper:

```rust
#[cfg(feature = "cuda")]
fn compute_indicator_host_f32_with_trace(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<(Vec<f32>, CarmIndicatorRouteTrace), String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("CARM indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    let prefer_gpu = backend_prefers_gpu(backend)
        && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);

    let mut trace = CarmIndicatorRouteTrace {
        preferred_gpu: prefer_gpu,
        ..Default::default()
    };

    if prefer_gpu {
        trace.attempted_cuda = true;
        if let Some(prepared) = prepared_cuda_inputs {
            match compute_dynamic_indicator_cuda_with_inputs(
                prepared,
                indicator_id,
                Some(output_id),
                params,
                Kernel::Auto,
                vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
            ) {
                Ok(values) => {
                    trace.used_cuda = true;
                    return Ok((cuda_series_to_host_f32(values)?, trace));
                }
                Err(e) => {
                    trace.fallback_reason = Some(e.to_string());
                }
            }
        } else {
            trace.fallback_reason = Some("prepared CUDA inputs missing".to_string());
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

    Ok((series_to_f32(out), trace))
}
```

Keep the current signature as a thin wrapper:

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
    compute_indicator_host_f32_with_trace(
        candles,
        ma_source,
        backend,
        indicator_id,
        output_id,
        params,
        prepared_cuda_inputs,
    )
    .map(|(values, _)| values)
}
```

This keeps current behavior while making it possible for a future route-summary surface to say exactly why CARM indicator prep fell back.

### D. OTTO multi-output optimization

The current file computes OTTO twice:

- once for `hott`
- once for `lott`

If your dynamic-indicator adapter can compute both OTTO outputs in a single call, you should add a CARM-specific paired-output helper and use it in both `compute_carm_otto_signals(...)` and `compute_carm_otto_signals_f32(...)`.

Because the multi-output adapter API is not shown in the attached files, I am not giving a direct compile-ready snippet here. The implementation direction is:

- compute OTTO once
- extract both `hott` and `lott`
- avoid a second dispatch and second host/device output materialization

If your adapter cannot do that today, leave the current dual call in place and prioritize the runtime fixes first.

### E. Add more CARM logic tests in `carm.rs`

The current tests cover parameter rejection, one desired-position behavior case, and a GPU smoke test.

Add threshold and execution-mode tests like these:

```rust
#[test]
fn build_carm_desired_positions_respects_trade_on_next_bar() {
    let close = vec![100.0, 101.0, 102.0, 103.0, 104.0];
    let signals = CarmSignals {
        coppock: vec![f64::NAN, 0.4, 0.8, 0.9, 0.8],
        ao: vec![f64::NAN, 0.2, 0.5, 0.4, 0.3],
        rocr: vec![f64::NAN, 1.01, 1.02, 1.03, 1.01],
        apo: vec![f64::NAN, 0.1, 0.2, 0.2, 0.1],
        otto_hott: vec![f64::NAN, 100.0, 100.4, 101.0, 101.4],
        otto_lott: vec![f64::NAN, 101.0, 101.7, 102.1, 102.0],
        first_valid: 1,
    };

    let mut strategy = StrategyConfig::default();
    strategy.trade_on_next_bar = true;
    strategy.allow_flip = false;

    let cfg = CarmStrategyParams::default();
    let desired = build_carm_desired_positions(&close, &signals, 1, &strategy, &cfg);

    assert_eq!(desired[1], 0);
    assert_eq!(desired[2], 1);
}
```

```rust
#[test]
fn build_carm_desired_positions_respects_long_only() {
    let close = vec![100.0, 99.0, 98.0, 97.0, 96.0];
    let signals = CarmSignals {
        coppock: vec![f64::NAN, -0.3, -0.5, -0.7, -0.8],
        ao: vec![f64::NAN, -0.2, -0.4, -0.5, -0.6],
        rocr: vec![f64::NAN, 0.99, 0.98, 0.97, 0.96],
        apo: vec![f64::NAN, -0.1, -0.2, -0.3, -0.4],
        otto_hott: vec![f64::NAN, 100.1, 99.8, 99.0, 98.4],
        otto_lott: vec![f64::NAN, 99.8, 99.0, 98.2, 97.6],
        first_valid: 1,
    };

    let mut strategy = StrategyConfig::default();
    strategy.long_only = true;
    strategy.allow_flip = false;

    let cfg = CarmStrategyParams::default();
    let desired = build_carm_desired_positions(&close, &signals, 1, &strategy, &cfg);

    assert!(desired.iter().all(|&p| p >= 0));
}
```

These tests strengthen the parity surface even before you add a full CPU-vs-GPU metrics fixture elsewhere.

---

# 3. Staging and fallback review

## Memory-copy reduction plan

### Current copy surfaces

#### Host to device

- `close_f32`
- `fast_row_index`
- `otto_row_index`
- `start_t`
- `end_t`

#### Device to host

- full metrics matrix

### What to remove first

#### Remove repeated `close_f32` upload on same-series runs

Do this with `cached_close_host_ptr + cached_close_len` in the runtime.

#### Remove repeated `d_lr` allocation and log-return recompute on same-series runs

Treat `d_lr` as derived state of `close_f32`. If the close series did not change, `d_lr` is still valid.

#### Remove per-call device allocations for combo/output buffers

The combo arrays still need copies unless you change the caller contract, but the allocation can be reused.

#### Remove zero-filling of `host_metrics`

Use `Vec::with_capacity + set_len` for POD readback buffers.

## Uninitialized-memory policy

### Use uninitialized memory here

- `DeviceBuffer::<f32>::uninitialized(...)` for reusable GPU scratch
- `Vec<f32>` host readback buffer that is fully overwritten by `copy_to(...)` before any read

### Do **not** use uninitialized memory here

- `Vec<Metrics>`
- any non-POD type with a destructor
- any host buffer that may be partially filled and then observed on error

## Fallback transparency plan

### Problem

Indicator CUDA fallback is currently silent in `carm.rs`.

### Fix

- add traced indicator helpers for `HostF32`
- optionally add traced `f64` helpers too if you want full route visibility on CPU-reference paths
- surface fallback reasons in debug logs, route summaries, or benches that focus on CARM prep cost

## Route blockers that should remain explicit

The GPU kernel currently rejects:

- execution overlays
- stop-loss
- take-profit

Keep that blocker explicit. Do **not** silently downgrade to a GPU route that ignores those fields.

What you should improve is the visibility of that blocker in route reporting and tests.

---

# 4. Per-strategy benchmark review

## `gpu_native_expansion_opt.rs`

### Goals for the CARM optimization bench

- assert GPU route for every GPU-labeled measurement
- warm each shape before timing it
- keep the request shape faithful to the CARM validation route
- avoid extra output shaping work if it is not part of the benchmark question

## Code-edit plan for `gpu_native_expansion_opt.rs`

### A. Add a small helper for GPU-route assertions

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn assert_gpu_backend(
    route_label: &str,
    backend_used: BackendUsed,
    backend_route: Option<&str>,
    backend_fallback_reason: Option<&str>,
) {
    assert!(
        matches!(backend_used, BackendUsed::Gpu),
        "expected GPU backend for {}, got backend={:?}, route={:?}, fallback={:?}",
        route_label,
        backend_used,
        backend_route,
        backend_fallback_reason,
    );
}
```

Use it both for the probe and for each measured result.

### B. Make the CARM benchmark request CARM-specific instead of inheriting the generic MA-type mix

Current `base_req(...)` uses `sma` / `ema`. Validation CARM uses `ema` / `ema`.

Override that in `make_carm_req(...)`:

```rust
fn make_carm_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.carm = Some(CarmStrategyParams::default());
    req.fast_ma_types = vec!["ema".to_string()];
    req.slow_ma_types = vec!["ema".to_string()];
    req.fast_range = fast_range;
    req.slow_range = slow_range;

    // Optional: disable heatmap work for route-only perf benches.
    req.heatmap_bins = None;

    req
}
```

If you want one benchmark to reflect true product-output behavior and another to reflect raw route cost, keep two CARM request builders:

- `make_carm_req_product_fidelity(...)`
- `make_carm_req_perf_only(...)`

### C. Warm each case independently before timing it

Right now only the initial route probe is explicit. You should warm each shape before Criterion measures that shape.

Update the loop in `bench_gpu_cases(...)` like this:

```rust
for (label, fast_range, slow_range) in cases.iter().copied() {
    let req = build_req(Backend::GpuKernel { device_id: 0 }, fast_range, slow_range);

    let warm = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
        req.clone(),
        candles,
        &cancel,
        None,
    )
    .expect("gpu warm run");

    assert_gpu_backend(
        route_label,
        warm.backend_used,
        warm.backend_route.as_deref(),
        warm.backend_fallback_reason.as_deref(),
    );

    group.bench_function(label, |b| {
        b.iter(|| {
            let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                req.clone(),
                candles,
                &cancel,
                None,
            )
            .expect("gpu bench run");

            assert_gpu_backend(
                route_label,
                res.backend_used,
                res.backend_route.as_deref(),
                res.backend_fallback_reason.as_deref(),
            );
        })
    });
}
```

### D. Optional: add a dedicated steady-state CARM group

If you want to explicitly measure the payoff of the new runtime reuse, add a CARM-only benchmark that repeatedly runs the exact same request shape on the exact same candle series after one warm-up.

That makes improvements from close/log-return reuse much easier to see.

Suggested label:

- `carm/gpu_kernel/200k/steady_state_large`

---

## `composite_validation_workflow.rs`

### Goals for the CARM validation bench

- do not allow silent CPU fallback in GPU-labeled groups
- include a dedicated CARM GPU validation profile, not only generic profile coverage
- use candle shapes that actually exercise CARM-like momentum/regime behavior
- optionally diversify a few CARM-native params so validation covers more of the strategy-local route shape

## Code-edit plan for `composite_validation_workflow.rs`

### A. Import and use `BackendUsed`

Add `BackendUsed` to the imports if it is not already there:

```rust
use ta_app_core::{
    BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeParams,
    CompositeValidationRequest, CscvPboConfig, PerturbConfig, TradeBootstrapConfig,
    WalkForwardConfigOptional,
};
```

### B. Add the same GPU assertion helper

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn assert_gpu_backend(
    route_label: &str,
    backend_used: BackendUsed,
    backend_route: Option<&str>,
    backend_fallback_reason: Option<&str>,
) {
    assert!(
        matches!(backend_used, BackendUsed::Gpu),
        "expected GPU backend for {}, got backend={:?}, route={:?}, fallback={:?}",
        route_label,
        backend_used,
        backend_route,
        backend_fallback_reason,
    );
}
```

Use it after every route probe and every measured validation call in GPU groups.

### C. Add a CARM-specific candle generator

The generic synthetic candles are useful for broad workflow coverage, but they are not strategy-local enough for CARM.

Add a CARM momentum/regime generator similar to the one already used in `gpu_native_expansion_opt.rs`:

```rust
fn make_carm_momentum_candles(len: usize) -> Candles {
    let len = len.max(260);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let px = if i < 70 {
            100.0 + (i as f64 * 0.12).sin() * 0.8 + i as f64 * 0.05
        } else if i < 140 {
            103.5 + (i - 70) as f64 * 0.21 + ((i - 70) as f64 * 0.16).sin() * 0.7
        } else if i < 200 {
            118.0 - (i - 140) as f64 * 0.18 + ((i - 140) as f64 * 0.22).cos() * 0.9
        } else {
            107.0 + (i - 200) as f64 * 0.14 + ((i - 200) as f64 * 0.14).sin() * 0.6
        };
        open.push(px * 0.998);
        high.push(px * 1.004);
        low.push(px * 0.996);
        close.push(px);
        vol.push(1_100.0 + (i % 17) as f64 * 28.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

### D. Add an optional diversified CARM candidate builder

The current `make_carm_candidates(...)` only varies fast/slow lengths.

For a dedicated CARM validation bench, you can add mild variation in a few CARM-native parameters while staying close to defaults.

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_carm_candidates_diverse(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 10u16.saturating_add(i_u16.saturating_mul(2));
        let slow_len = fast_len.saturating_add(20);

        let mut carm = CarmStrategyParams::default();
        carm.rocr_period = carm.rocr_period.saturating_add(i_u16 % 3);
        carm.otto_period = carm.otto_period.saturating_add(i_u16 % 4);
        carm.otto_threshold = 0.6 + (i % 4) as f64 * 0.1;

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
            carm: Some(carm),
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

If you want to preserve current benchmark continuity, keep the old `make_carm_candidates(...)` and use this new version only in a dedicated CARM profile.

### E. Add a dedicated CARM GPU validation group

A focused CARM GPU validation group is worth having because the top-level GPU workflow group is currently ASR-only.

Suggested implementation:

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_carm_gpu_validation_profile(c: &mut Criterion) {
    let candles = make_carm_momentum_candles(100_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_carm_candidates_diverse(64);

    let mut group = c.benchmark_group("composite_validate/gpu_kernel/carm/100k/64candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let base = CompositeValidationRequest {
        data_id: "bench-gpu-carm".to_string(),
        backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: candidates.clone(),
        walk_forward: Some(WalkForwardConfigOptional {
            window_bars: 20_000,
            step_bars: 20_000,
        }),
        reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
            window_bars: 30_000,
            step_bars: 20_000,
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

    let probe = ta_app_core::run_composite_validation_blocking_with_candles(
        base.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("carm gpu validation route probe");

    assert_gpu_backend(
        "composite_validate/gpu_kernel/carm/100k/64candidates",
        probe.backend_used,
        probe.backend_route.as_deref(),
        probe.backend_fallback_reason.as_deref(),
    );

    let warm = ta_app_core::run_composite_validation_blocking_with_candles(
        base.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("carm gpu validation warm run");

    assert_gpu_backend(
        "composite_validate/gpu_kernel/carm/100k/64candidates",
        warm.backend_used,
        warm.backend_route.as_deref(),
        warm.backend_fallback_reason.as_deref(),
    );

    group.bench_function("wf_reopt_cscv_spa", |b| {
        b.iter(|| {
            let res = ta_app_core::run_composite_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("carm gpu validation bench run");

            assert_gpu_backend(
                "composite_validate/gpu_kernel/carm/100k/64candidates",
                res.backend_used,
                res.backend_route.as_deref(),
                res.backend_fallback_reason.as_deref(),
            );
        })
    });

    group.finish();
}
```

Then include it in the Criterion group list.

### F. Add assertions to the existing generic GPU profile bench too

Inside the existing `bench_profile(...)` closure, after `route_probe`, add:

```rust
assert_gpu_backend(
    &format!(
        "composite_validate/gpu_kernel/{}/{}/{}candidates",
        name, candles_len, candidate_count
    ),
    route_probe.backend_used,
    route_probe.backend_route.as_deref(),
    route_probe.backend_fallback_reason.as_deref(),
);
```

Then assert the backend inside the measured closure as well.

---

# 5. Additional optimizations worth considering after the core patches

## A. Device-side top-k / shortlist instead of full metrics readback

### Why it matters

The current CARM kernel path returns a full `Vec<Metrics>`, so it must copy every metrics row from device to host.

If the optimizer only needs a shortlist, a device-side top-k or reduction path can remove the largest remaining D2H copy.

### Why this is phase 2

It changes interfaces, result shaping, and probably upstream optimizer integration. It is a bigger change than the buffer-reuse fixes.

### Recommendation

Do this only after you have measured steady-state CARM again with the low-risk changes in place.

## B. Device-resident CARM indicator outputs

The cleanest long-term CARM GPU path is:

- prepare indicator outputs directly on device
- build `CarmGpuDeviceSignalMatrices` from device-resident outputs
- never round-trip those series through host memory

The attached `carm.rs` file only shows `HostF32` requests for CUDA indicator output, not a device-resident path.

If upstream currently routes the CARM GPU preparation through `HostF32`, that is a large opportunity.

However, the exact upstream builder code is not included in the files you attached, so this remains a phase-2 recommendation.

## C. Pinned host memory for output readback

If the full metrics readback remains necessary, pinned host memory may improve transfer behavior.

That is a second-order optimization compared with removing the readback entirely or reducing it.

## D. Kernel launch-config caching

I am **not** giving a direct code patch for storing `Function` handles because `cust::Function` lifetimes are often tied to the owning `Module`, which makes self-contained caching awkward.

You can still consider caching launch-cap metadata if your local `gpu_backtest_kernel` types are cheap to store and your `cust` setup makes it ergonomic.

This is optional and lower priority than the buffer and copy fixes.

---

# 6. Conditional follow-up for the actual CUDA kernel body

The following suggestions are valid only after you inspect the actual CUDA source or profile its PTX/SASS with Nsight Compute. They are **not** confirmed from the provided Rust files.

## A. Reduce per-thread repeated work inside the time loop

If each thread walks one combo across time:

- load `commission`, `flags`, and `otto_threshold` into registers once
- avoid repeated `fabsf(close)` plus division if possible
- compute `inv_close = 1.0f / max(abs(close), 1e-9f)` once per step and multiply

Illustrative pattern:

```cpp
const float denom = fmaxf(fabsf(close_t), 1.0e-9f);
const float inv_close = 1.0f / denom;
const float otto_spread = (lott_t - hott_t) * inv_close;
```

## B. Consider kernel specialization for branch-heavy flags

If `long_only`, `allow_flip`, and `trade_on_next_bar` are causing divergence, consider separate kernel variants or template specializations for a few common flag combinations.

This is most useful if Nsight shows branch efficiency problems.

## C. Use read-only / cache-friendly access for signal matrices

The signal matrices appear read-only during backtest evaluation. If the kernel currently uses plain pointer loads, profile whether read-only cache access patterns help.

## D. Check register pressure and occupancy

CARM logic has multiple signal states, thresholds, and accumulated metrics. Occupancy can drop if the compiler keeps too much per-thread state live.

Use Nsight Compute to inspect:

- achieved occupancy
- register count
- local memory spills
- branch efficiency
- DRAM throughput vs arithmetic utilization

## E. Check memory layout assumptions

From the Rust side, the signals are passed as row-major pointers plus stride metadata. Verify that the kernel’s indexing pattern actually coalesces the dominant loads for the way you map threads to combos and time.

If each thread uses a different row index but marches through the same time index, signal-row-major layout may be fine. If not, you may want to evaluate a different packing strategy.

---

# 7. Measurement and acceptance criteria

## Measure before and after these patches

### Optimization bench

Use:

- `cargo bench --bench gpu_native_expansion_opt --features cuda,cuda-backtest-kernel`
- optionally restrict to CARM with your existing environment filter

### Validation bench

Use:

- `cargo bench --bench composite_validation_workflow --features cuda,cuda-backtest-kernel`

### Nsight / runtime instrumentation

Track:

- number of device allocations per steady-state iteration
- bytes uploaded H2D per steady-state iteration
- bytes read back D2H per steady-state iteration
- kernel launches per iteration
- backend route and fallback reason

## Acceptance criteria

### Must-have

- GPU-labeled CARM benches fail if they actually fall back to CPU
- steady-state CARM runs stop re-uploading unchanged close data
- steady-state CARM runs stop recomputing log returns when the close series is unchanged
- `host_metrics` no longer zero-initializes before full overwrite
- `compute_indicator_host_f32(...)` no longer detours through `Vec<f64>` on CPU fallback

### Strongly desired

- visible reduction in steady-state latency on medium and large CARM GPU benchmark cases
- reduced variance between benchmark samples
- no regression in CPU reference behavior or route blocker correctness

### Stretch

- a dedicated CARM GPU validation profile with stable GPU-route assertions
- a clean `CarmSignalsF32` path adopted by the GPU-prep caller

---

# 8. Implementation order

## Phase 1: low-risk, high-payoff

1. `carm_gpu_kernel.rs`: reusable device buffers
2. `carm_gpu_kernel.rs`: close/log-return reuse
3. `carm_gpu_kernel.rs`: reusable uninitialized host readback buffer
4. `carm.rs`: `series_to_f32(...)`
5. `carm.rs`: direct CPU fallback in `compute_indicator_host_f32(...)`
6. `gpu_native_expansion_opt.rs`: per-case warm run and route assertions
7. `gpu_native_expansion_opt.rs`: CARM MA-type alignment
8. `composite_validation_workflow.rs`: GPU-route assertions in every GPU-labeled bench

## Phase 2: medium-risk, still worthwhile

9. `carm.rs`: add `CarmSignalsF32` and `compute_carm_signals_f32(...)`
10. `composite_validation_workflow.rs`: add dedicated CARM GPU validation group
11. `composite_validation_workflow.rs`: add CARM-like candles and optional diversified CARM candidates
12. `carm.rs`: add traced indicator helper for fallback visibility

## Phase 3: larger structural work

13. upstream device-resident indicator outputs for CARM prep
14. OTTO paired-output or single-dispatch multi-output path
15. device-side shortlist / top-k reduction to avoid full metrics readback
16. kernel-body specialization after Nsight evidence

---

# 9. Detailed implementation checklist

## `carm_gpu_kernel.rs`

- [ ] Add `ReusableDeviceBuffer<T>` helper.
- [ ] Add runtime-owned reusable buffers for close, log returns, combo arrays, and output.
- [ ] Add `host_metrics: Vec<f32>` to the runtime.
- [ ] Add `cached_close_host_ptr` and `cached_close_len` to the runtime.
- [ ] Change `launch_compute_log_returns_f32(...)` to take `&Module` and `&Stream`.
- [ ] Add `ensure_close_and_log_returns(...)` to the runtime.
- [ ] Skip close upload when the host close slice pointer and length are unchanged.
- [ ] Skip log-return recomputation when the host close slice pointer and length are unchanged.
- [ ] Replace per-call `DeviceBuffer::from_slice(...)` for combos with reusable device buffers plus `copy_from(...)`.
- [ ] Replace per-call `DeviceBuffer::<f32>::uninitialized(out_len)` for output with runtime scratch reuse.
- [ ] Replace `vec![0.0f32; out_len]` with reusable host readback storage.
- [ ] Use `Vec::with_capacity + set_len` only for POD readback types.
- [ ] Keep `Vec<Metrics>` decode path safe; do not force uninitialized `Metrics`.
- [ ] Add pure-Rust tests for `validate_inputs(...)`.
- [ ] Add a regression test for combo-length mismatch.
- [ ] Add a test or route assertion for overlay/SL/TP blockers where practical.

## `carm.rs`

- [ ] Add `series_to_f32(...)`.
- [ ] Rewrite `compute_indicator_host_f32(...)` so CPU fallback converts directly to `Vec<f32>`.
- [ ] Verify that no path still allocates `Vec<f64>` before converting to `Vec<f32>` in the fallback route.
- [ ] Add `CarmSignalsF32`.
- [ ] Add `compute_carm_signals_f32(...)`.
- [ ] Update GPU-prep callers to prefer `compute_carm_signals_f32(...)` where applicable.
- [ ] Add optional traced helper for `HostF32` indicator fallback visibility.
- [ ] Surface fallback reasons somewhere observable for CARM diagnostics.
- [ ] Evaluate whether OTTO can be produced as a paired output in one dispatch.
- [ ] If paired OTTO is available, update both `compute_carm_otto_signals(...)` and `compute_carm_otto_signals_f32(...)`.
- [ ] Add test for `trade_on_next_bar` behavior.
- [ ] Add test for `long_only` behavior.
- [ ] Add test for `allow_flip` behavior if you want stronger execution-mode coverage.
- [ ] Keep CPU reference semantics unchanged while optimizing prep.

## `gpu_native_expansion_opt.rs`

- [ ] Add `assert_gpu_backend(...)` helper.
- [ ] Use the helper on the initial route probe.
- [ ] Use the helper on every measured GPU result.
- [ ] Warm each case before Criterion starts timing it.
- [ ] Override `make_carm_req(...)` to use `ema` / `ema`.
- [ ] Decide whether CARM perf-only benches should set `heatmap_bins = None`.
- [ ] Optionally split product-fidelity vs perf-only CARM request builders.
- [ ] Optionally add a dedicated `steady_state_large` CARM GPU benchmark.
- [ ] Re-run CARM CPU vs GPU optimization benches and compare medium/large cases after the runtime changes.

## `composite_validation_workflow.rs`

- [ ] Import `BackendUsed` if needed.
- [ ] Add `assert_gpu_backend(...)` helper.
- [ ] Assert GPU backend for every GPU route probe.
- [ ] Assert GPU backend inside every GPU-labeled measured closure.
- [ ] Add a CARM-specific candle generator.
- [ ] Add a dedicated CARM GPU validation benchmark group.
- [ ] Decide whether to keep `make_carm_candidates(...)` unchanged for continuity.
- [ ] If needed, add `make_carm_candidates_diverse(...)` for a dedicated CARM profile.
- [ ] Warm the dedicated CARM validation case before timing it.
- [ ] Keep the current generic GPU profile bench, but hard-fail on fallback there too.
- [ ] Compare CARM validation behavior on generic synthetic candles vs CARM momentum candles.

## Measurement / profiling

- [ ] Record pre-change CARM GPU optimization times for small, medium, and large cases.
- [ ] Record pre-change CARM GPU validation times for the existing CARM groups.
- [ ] Re-run the same numbers after Phase 1.
- [ ] Check whether steady-state H2D traffic drops after close/log-return reuse.
- [ ] Check whether device allocation counts drop after reusable scratch lands.
- [ ] Check whether benchmark variance improves.
- [ ] Verify that GPU-labeled validation benches no longer bench CPU fallback silently.
- [ ] If results are good, move to Phase 2.
- [ ] If results are still disappointing, inspect the actual CUDA kernel body with Nsight.

## Safety / correctness guardrails

- [ ] Do not use uninitialized memory for non-POD host types.
- [ ] Do not silently ignore unsupported execution overlays or protective order settings.
- [ ] Do not switch CARM to GPU-preferred for tiny workloads without measurement.
- [ ] Do not merge OTTO paired-output logic until parity is verified.
- [ ] Do not claim kernel-body wins until you have Nsight evidence from the real CUDA source.

---

## Final recommendation

If you only implement a small subset of this document, implement these first:

1. reusable device buffers in `carm_gpu_kernel.rs`
2. close/log-return reuse in `carm_gpu_kernel.rs`
3. uninitialized reusable host readback for `f32`
4. direct CPU-to-`f32` fallback in `carm.rs`
5. GPU-route assertions in `composite_validation_workflow.rs`

Those five changes are the best balance of confidence, impact, and implementation cost from the files you provided.


---

# Appendix: suggested rollout and bench commands

## Suggested implementation rollout

1. Land the benchmark-fidelity patches first:
   - GPU assertion helper in `gpu_native_expansion_opt.rs`
   - GPU assertion helper in `composite_validation_workflow.rs`
   - explicit CARM MA-type alignment between the two benches
2. Land the `carm_gpu_kernel.rs` scratch-reuse patch second.
3. Land the `carm.rs` direct CPU-to-`f32` fallback patch third.
4. Land the `CarmSignalsF32` aggregation and route-trace additions fourth.
5. Land dedicated CARM validation groups only after the route assertions are already in place.

## Suggested bench commands

### Optimization bench

```bash
VECTORBT_GPU_NATIVE_BENCH_ONLY=carm   cargo bench --bench gpu_native_expansion_opt --features cuda,cuda-backtest-kernel
```

### Validation bench

```bash
cargo bench --bench composite_validation_workflow --features cuda,cuda-backtest-kernel
```

### Optional: dedicated CARM validation stress profile

If you gate the dedicated CARM validation profile behind an env var, a good pattern is:

```bash
VECTORBT_COMPOSITE_VALIDATE_ONLY=carm   cargo bench --bench composite_validation_workflow --features cuda,cuda-backtest-kernel
```

## Rollback guidance

If any patch regresses parity or introduces instability, roll back in this order:

1. disable dedicated CARM validation groups;
2. disable `CarmSignalsF32` adoption at the call site;
3. keep direct CPU-to-`f32` fallback;
4. keep benchmark assertion helpers;
5. revert only the runtime scratch reuse section last.

That ordering preserves benchmark truthfulness even if one of the deeper runtime changes needs to be retried.
