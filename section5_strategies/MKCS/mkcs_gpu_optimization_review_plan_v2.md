
# MKCS strategy-local GPU-native optimization and validation plan

## Scope

This document is intentionally scoped to **Section 5: strategy-local GPU-native optimization and validation routes** from `gpt54_feature_review_map.md`, and only to the **MKCS** strategy. It follows the bundle’s required focus areas:

- strategy-local hot-path review
- CPU/GPU payoff review
- staging/fallback review
- per-strategy benchmark review

It also applies the allocation/copy-elision guidance from the map: prefer the shared allocation helpers where they already exist, only use uninitialized memory when every element is guaranteed to be overwritten before any read, and keep full-fill allocations where the fill value is observable behavior.

## Files covered

- `mkcs.rs`
- `mkcs_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

## Surface understanding

### What the current MKCS path is doing

At a high level, the current MKCS GPU route is:

1. `mkcs.rs`
   - resolves stddev and Donchian period mappings
   - computes stddev and Donchian surfaces
   - for CUDA sweeps, it currently requests **HostF32** rows one period at a time, appends them into a host `row_major` buffer, then uploads the completed matrix back to device
   - also contains the CPU reference desired-position logic

2. `mkcs_gpu_kernel.rs`
   - validates the device-row signal matrices and combo vectors
   - uploads `close_f32`
   - allocates `d_lr` and launches `compute_log_returns_f32`
   - uploads combo arrays every call
   - allocates `d_out` every call
   - synchronizes and copies all metrics back to host
   - decodes metrics rows into `Vec<Metrics>`

3. `gpu_native_expansion_opt.rs`
   - currently benchmarks KQSR, CARM, KVDF, and CDMC
   - **does not currently benchmark MKCS**

4. `composite_validation_workflow.rs`
   - does include a generic `bench_profile("mkcs", 50_000, make_mkcs_candidates(8))`
   - does **not** include a dedicated heavier MKCS GPU validation group analogous to the ASR `100k/64candidates` and `192` in-sample cases

### Current copy boundaries and hot-path waste

The main avoidable copy and allocation boundaries in the uploaded MKCS files are:

- **GPU -> host -> host append -> GPU** during sweep construction in `mkcs.rs`
- **host close -> device** on every `eval_mkcs_metrics_gpu_kernel_device_rows(...)` call
- **host combos -> device** on every kernel call
- **zero-filled host metrics buffer** even though the device overwrites every element
- **f32 -> f64 conversion** for CUDA indicator outputs even when the next consumer is a GPU path or only parity measurement

### What is missing from the provided files

The provided files include the **host-side launcher** for `mkcs_backtest_metrics_f32`, but they do **not** include the device kernel implementation itself. That means this plan focuses on:

- host-side staging
- upload reuse
- allocation reuse
- validation coverage
- benchmark coverage
- safe parity-preserving changes

It does **not** try to claim device-code occupancy, warp-divergence, shared-memory, or coalescing improvements inside the actual CUDA kernel body because that kernel source is not in the provided files.

---

## Findings

## Confirmed issues

### 1) High: MKCS sweep construction does repeated GPU->host->GPU round-trips

- **Severity:** high  
- **Type:** performance  
- **Location:** `mkcs.rs::compute_indicator_period_sweep_cuda_host_rows`  
- **Timing boundary affected:** app-path, Criterion, validation-cost  
- **Why it matters:** this helper requests `CudaOutputTarget::HostF32` for each period, receives a fresh host row, appends it into a host `row_major` vector, then uploads the completed matrix back to the device. That means the same logical sweep pays:
  - one device-to-host transfer per period
  - one host-to-host append per period
  - one final host-to-device transfer for the full matrix  
- **Recommended change:** add a **device-native sweep path** as the long-term fix. As the short-term fix, at minimum remove avoidable per-period host allocations in this helper and reuse the parameter vector.

### 2) High: the kernel launcher re-uploads close, recomputes log returns, re-uploads combo arrays, and reallocates scratch buffers every call

- **Severity:** high  
- **Type:** performance  
- **Location:** `mkcs_gpu_kernel.rs::eval_mkcs_metrics_gpu_kernel_device_rows`  
- **Timing boundary affected:** app-path, Criterion, validation-cost  
- **Why it matters:** every invocation currently:
  - allocates and uploads `d_close`
  - allocates `d_lr`
  - launches `compute_log_returns_f32`
  - allocates and uploads four combo buffers
  - allocates `d_out`
  - synchronizes
  - allocates and zero-fills `host_metrics`
  - copies all metrics back to host  
  This is exactly the sort of repeated setup cost that can erase GPU advantage when candidate batches are re-run over the same candles.  
- **Recommended change:** add **MKCS-local scratch reuse** inside `MkcsCudaRuntime`, and add a public **uploaded combo batch** path for callers that reuse the same combos across many invocations.

### 3) Medium: the host readback buffer is zero-filled even though the device fully overwrites it

- **Severity:** medium  
- **Type:** performance  
- **Location:** `mkcs_gpu_kernel.rs::eval_mkcs_metrics_gpu_kernel_device_rows`  
- **Timing boundary affected:** app-path, Criterion, validation-cost  
- **Why it matters:** `let mut host_metrics = vec![0.0f32; out_len];` pays a full host memory fill before `d_out.copy_to(...)` overwrites every element. This is a safe copy-elision candidate because `f32` has no drop glue and the device readback fully populates the destination.  
- **Recommended change:** use a shared allocation helper from `alloc_policy` if one already exists for this pattern; otherwise, use a narrowly scoped POD-only uninitialized `Vec<f32>` helper or a reusable scratch `host_out` buffer.

### 4) Medium: `compute_mkcs_breakout_signals` pays two full Donchian fetches

- **Severity:** medium  
- **Type:** performance  
- **Location:** `mkcs.rs::compute_mkcs_breakout_signals`  
- **Timing boundary affected:** app-path  
- **Why it matters:** upper and lower Donchian outputs are fetched separately. That means repeated descriptor resolution, parameter building, dispatch, and host materialization for one logical breakout surface.  
- **Recommended change:** if the dynamic indicator adapter can be extended, add a **combined multi-output Donchian fetch** for MKCS. If not, at least cache the resolved keys/output IDs and reuse the period parameter allocation.

### 5) Medium: benchmark coverage is incomplete for MKCS optimization

- **Severity:** medium  
- **Type:** benchmark-fidelity  
- **Location:** `gpu_native_expansion_opt.rs`  
- **Timing boundary affected:** Criterion  
- **Why it matters:** the uploaded optimizer benchmark file has no MKCS request builder, no MKCS synthetic candles, and no MKCS CPU/GPU benchmark functions. Without that, there is no direct Criterion evidence for MKCS optimizer performance.  
- **Recommended change:** add MKCS CPU and GPU benches, plus phase-split microbenches for:
  - sweep prep only
  - kernel scoring only
  - full end-to-end route

### 6) Medium: MKCS validation coverage is too light compared with ASR

- **Severity:** medium  
- **Type:** benchmark-fidelity  
- **Location:** `composite_validation_workflow.rs`  
- **Timing boundary affected:** validation-cost  
- **Why it matters:** MKCS appears in the generic `50k/8-candidate` profile loops, but it does not have a dedicated `100k/64-candidate` GPU validation group or the `192-candidate` in-sample stress case that ASR has. That makes it harder to detect staging or VRAM cliffs for MKCS validation.  
- **Recommended change:** add dedicated MKCS validation groups mirroring the ASR shape.

## Likely issues

### 7) Medium: CUDA signal-prep fallback is silent at the `compute_indicator_f64` boundary

- **Severity:** medium  
- **Type:** correctness / maintainability  
- **Location:** `mkcs.rs::compute_indicator_f64`  
- **Timing boundary affected:** app-path, route-matrix  
- **Why it matters:** the function tries CUDA first and falls back to CPU if the CUDA call fails, but the current file-level API does not preserve the fallback reason. This can hide “GPU requested, CPU actually used” cliffs in route summaries unless a higher-level caller is already capturing this elsewhere.  
- **Recommended change:** either:
  - add a route-aware result type, or
  - log the fallback reason at this boundary, or
  - expose a `_with_route(...)` variant so benchmark and route-summary layers can preserve the failure reason

### 8) Medium: CPU reference desired-position logic has avoidable per-bar overhead

- **Severity:** medium  
- **Type:** performance  
- **Location:** `mkcs.rs::build_mkcs_desired_positions`  
- **Timing boundary affected:** app-path, validation-cost  
- **Why it matters:** the loop currently does:
  - repeated `.get(...).copied().unwrap_or(f64::NAN)` access
  - two divisions per bar through `stddev_ratio(...)`
  - repeated config/strategy field loads  
  This is not the top MKCS bottleneck, but it is a CPU-reference hot path and can dilute GPU payoff if the strategy still uses CPU prep or fallback runs.  
- **Recommended change:** use direct indexed access after validating lengths, compare against `abs(close) * threshold` instead of dividing, and hoist strategy/config flags out of the loop.

### 9) Medium: fp32 boundary behavior around thresholds is a parity-risk surface

- **Severity:** medium  
- **Type:** parity  
- **Location:** CPU reference logic in `mkcs.rs` vs GPU launcher in `mkcs_gpu_kernel.rs`  
- **Timing boundary affected:** app-path, validation-cost  
- **Why it matters:** the CPU reference logic uses `f64` comparisons against `compression_threshold` and `reexpansion_threshold`, while the GPU launcher explicitly passes the thresholds to the kernel as `f32`. Threshold-crossing strategies often show edge-case parity drift exactly at `<=` and `>=` boundaries.  
- **Recommended change:** add focused CPU/GPU parity tests around `threshold ± ε`, and keep threshold changes separate from performance edits.

## Open questions

### 10) Open: device kernel implementation is not provided

- **Severity:** low  
- **Type:** profiling-only / open-question  
- **Location:** `mkcs_backtest_metrics_f32` device code is not in the uploaded files  
- **Timing boundary affected:** profiling-only  
- **Why it matters:** this prevents reliable recommendations about:
  - register pressure
  - divergence
  - shared-memory usage
  - read coalescing
  - occupancy  
- **Recommended change:** treat this document as the **host-side and benchmark-side plan**. If the staging changes still leave MKCS underperforming, inspect the actual CUDA kernel source or PTX next.

---

## Safe optimization opportunities

These are the changes I would implement first because they are high-value and parity-safe.

### Priority P0

1. Reuse MKCS runtime scratch in `mkcs_gpu_kernel.rs`
2. Reuse combo device buffers and add an uploaded-combo public API
3. Replace zero-filled host readback with uninitialized or reusable POD host scratch
4. Add MKCS end-to-end optimizer benches in `gpu_native_expansion_opt.rs`
5. Add dedicated MKCS validation groups in `composite_validation_workflow.rs`
6. Add parity tests before more invasive CUDA changes

### Priority P1

1. Cache MKCS indicator key/output resolution in `mkcs.rs`
2. Reuse the period parameter vector in `compute_indicator_period_sweep_cuda_host_rows`
3. Tighten CPU desired-position loop indexing and threshold math
4. Add a route-aware or fallback-aware signal compute path

### Priority P2

1. Add a public uploaded-batch evaluation API for MKCS scoring
2. Add phase-split Criterion benches for sweep prep and score-only costs
3. Add device-native close reuse or explicit uploaded-close API if the same candle series is used across many MKCS evaluations

---

## Higher-risk optimization opportunities

These should come **after** the safe changes are measured.

### 1) Device-native sweep construction

This is the biggest structural fix, but it likely needs an adjacent-file change in the dynamic indicator adapter to expose a device-native output and/or a device-to-device copy helper that can write directly into a row-major destination matrix.

### 2) Combined multi-output Donchian fetch

This is likely worthwhile, but it also depends on adjacent dynamic indicator dispatch surfaces not provided here.

### 3) Fusing log-return computation into the MKCS metrics kernel

This may remove a launch and one device scratch allocation, but it changes the kernel contract and can complicate parity and diagnosis. I would not do this before the host-side reuse work is in place and measured.

---

## File-by-file implementation plan

## 1) `mkcs.rs`

### Goals for this file

- reduce host-side churn in sweep prep
- reduce repeated descriptor/key/output lookups
- tighten CPU reference loop
- prepare for eventual device-native sweep support
- improve route/fallback transparency if desired

### 1.1 Cache MKCS indicator keys and output IDs once

This is not the biggest win, but it is cheap, safe, and removes repeated descriptor scans and temporary `Vec<String>` allocations.

```rust
use std::sync::OnceLock;

struct MkcsIndicatorIds {
    stddev_period_key: String,
    stddev_output: String,
    donchian_period_key: String,
    donchian_upper_output: String,
    donchian_lower_output: String,
}

fn mkcs_indicator_ids() -> Result<&'static MkcsIndicatorIds, String> {
    static IDS: OnceLock<Result<MkcsIndicatorIds, String>> = OnceLock::new();

    IDS.get_or_init(|| {
        Ok(MkcsIndicatorIds {
            stddev_period_key: resolve_param_key("stddev", &["period", "length", "timeperiod"])?,
            stddev_output: resolve_output_id("stddev", &["value", "stddev", "stdev"])?,
            donchian_period_key: resolve_param_key("donchian", &["period", "length", "timeperiod"])?,
            donchian_upper_output: resolve_output_id("donchian", &["upperband", "upper", "value"])?,
            donchian_lower_output: resolve_output_id("donchian", &["lowerband", "lower"])?,
        })
    })
    .as_ref()
    .map_err(|e| e.clone())
}
```

Then update call sites like this:

```rust
pub fn compute_mkcs_compression_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    stddev_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<MkcsCompressionSignals, String> {
    let ids = mkcs_indicator_ids()?;
    let params = vec![IndicatorParamKV {
        key: ids.stddev_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(stddev_period)),
    }];

    let stddev = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "stddev",
        ids.stddev_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let first_valid = first_finite_idx_local(&stddev)
        .ok_or_else(|| "MKCS stddev output is all non-finite".to_string())?;

    Ok(MkcsCompressionSignals { stddev, first_valid })
}
```

### 1.2 Reuse the parameter vector in the host-row sweep helper

The current helper allocates a new `Vec<IndicatorParamKV>` and a new `period_key.to_string()` on every period. That is small compared with the transfer cost, but it is still unnecessary.

```rust
#[cfg(feature = "cuda")]
fn compute_indicator_period_sweep_cuda_host_rows(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: &str,
    period_key: &str,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    base_params: &[IndicatorParamKV],
) -> Result<MkcsPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let rows = usize::from(((end - start) / step) + 1);
    let cols = prepared_inputs.len();
    let total = rows
        .checked_mul(cols)
        .ok_or_else(|| format!("MKCS CUDA sweep overflow for indicator '{indicator_id}'"))?;

    let mut row_major = Vec::<f32>::with_capacity(total);

    let mut params = Vec::<IndicatorParamKV>::with_capacity(base_params.len() + 1);
    params.extend_from_slice(base_params);
    params.push(IndicatorParamKV {
        key: period_key.to_string(),
        value: IndicatorParamValue::Int(i64::from(start)),
    });

    for period in (start..=end).step_by(usize::from(step)) {
        if let Some(period_param) = params.last_mut() {
            period_param.value = IndicatorParamValue::Int(i64::from(period));
        }

        let out = compute_dynamic_indicator_cuda_with_inputs(
            prepared_inputs,
            indicator_id,
            Some(output_id),
            params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .map_err(|e| e.to_string())?;

        let mut row = cuda_series_to_host_f32(out)?;
        if row.len() != cols {
            return Err(format!(
                "MKCS CUDA host sweep column mismatch for indicator '{indicator_id}' period={period}: expected {cols}, got {}",
                row.len()
            ));
        }

        row_major.append(&mut row);
    }

    debug_assert_eq!(row_major.len(), total);

    let matrix = MkcsDeviceMatrix::from_raw(
        prepared_inputs
            .runtime()
            .upload_f32(row_major.as_slice())
            .map_err(|e| e.to_string())?,
        rows,
        cols,
    );

    Ok(MkcsPeriodSweepDevice {
        matrix,
        period_start: start,
        period_end: end,
        period_step: step,
    })
}
```

**Why this is worth doing even though it is not the main win:** it is effectively free risk-wise, and it keeps the temporary allocation count from exploding before you land the larger device-native sweep change.

### 1.3 Tighten the CPU desired-position reference path

This is not the core CUDA optimization, but it is a real CPU hot path and is safe to improve.

Key changes:

- use direct indexed access after up-front length validation
- compare `abs(stddev)` against `abs(close) * threshold` instead of dividing
- hoist flags and thresholds out of the loop

```rust
#[inline(always)]
fn abs_le_threshold(stddev: f64, close: f64, threshold: f64) -> bool {
    stddev.is_finite()
        && close.is_finite()
        && stddev.abs() <= close.abs().max(1.0e-12) * threshold
}

#[inline(always)]
fn abs_ge_threshold(stddev: f64, close: f64, threshold: f64) -> bool {
    stddev.is_finite()
        && close.is_finite()
        && stddev.abs() >= close.abs().max(1.0e-12) * threshold
}

pub fn build_mkcs_desired_positions(
    close: &[f64],
    compression: &MkcsCompressionSignals,
    breakout: &MkcsBreakoutSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &MkcsStrategyParams,
) -> Vec<i32> {
    let n = close.len();
    debug_assert_eq!(compression.stddev.len(), n);
    debug_assert_eq!(breakout.upper.len(), n);
    debug_assert_eq!(breakout.lower.len(), n);

    let mut desired = vec![0i32; n];
    if start_t >= n {
        return desired;
    }

    let trade_offset = if strategy.trade_on_next_bar { 1usize } else { 0usize };
    let long_only = strategy.long_only;
    let allow_flip = strategy.allow_flip;
    let compression_threshold = cfg.compression_threshold;
    let reexpansion_threshold = cfg.reexpansion_threshold;

    let mut pos: i32 = 0;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close[sig_t];
        let close_prev = close[prev_t];
        let stddev_prev = compression.stddev[prev_t];
        let stddev_now = compression.stddev[sig_t];
        let upper_prev = breakout.upper[prev_t];
        let lower_prev = breakout.lower[prev_t];

        let compressed_prev = abs_le_threshold(stddev_prev, close_prev, compression_threshold);
        let reexpanded_now = abs_ge_threshold(stddev_now, close_now, reexpansion_threshold);

        let long_breakout =
            compressed_prev && close_now.is_finite() && upper_prev.is_finite() && close_now > upper_prev;
        let short_breakout =
            compressed_prev && close_now.is_finite() && lower_prev.is_finite() && close_now < lower_prev;

        if pos == 0 {
            if long_breakout {
                pos = 1;
            } else if !long_only && short_breakout {
                pos = -1;
            }
        } else if pos > 0 {
            let mut exit_now =
                reexpanded_now || (close_now.is_finite() && upper_prev.is_finite() && close_now <= upper_prev);

            if !long_only && short_breakout {
                if allow_flip {
                    pos = -1;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }

            if exit_now {
                pos = 0;
            }
        } else {
            let mut exit_now =
                reexpanded_now || (close_now.is_finite() && lower_prev.is_finite() && close_now >= lower_prev);

            if long_breakout {
                if allow_flip {
                    pos = 1;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }

            if exit_now {
                pos = 0;
            }
        }

        desired[t] = pos;
    }

    desired
}
```

### 1.4 Long-term fix: device-native sweep path

This is the best structural fix, but it likely needs adjacent dynamic-indicator support not shown in the uploaded files.

The end-state shape should be:

- request `CudaOutputTarget::DeviceF32`
- receive a device-owned row
- write or copy that row into a contiguous device matrix
- return `MkcsPeriodSweepDevice` without ever materializing the sweep on host

**Illustrative shape only** — adapt this to the actual device-series type exposed by your dynamic indicator adapter:

```rust
// PSEUDOCODE / ADJACENT-API SHAPE
#[cfg(feature = "cuda")]
fn compute_indicator_period_sweep_cuda_device_rows(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: &str,
    period_key: &str,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    base_params: &[IndicatorParamKV],
) -> Result<MkcsPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let rows = usize::from(((end - start) / step) + 1);
    let cols = prepared_inputs.len();
    let total = rows.checked_mul(cols).ok_or_else(|| "MKCS sweep overflow".to_string())?;

    let mut matrix_owner = prepared_inputs.runtime().alloc_device_f32(total)?;
    let mut params = Vec::<IndicatorParamKV>::with_capacity(base_params.len() + 1);
    params.extend_from_slice(base_params);
    params.push(IndicatorParamKV {
        key: period_key.to_string(),
        value: IndicatorParamValue::Int(i64::from(start)),
    });

    for (row_idx, period) in (start..=end).step_by(usize::from(step)).enumerate() {
        params.last_mut().unwrap().value = IndicatorParamValue::Int(i64::from(period));

        let out = compute_dynamic_indicator_cuda_with_inputs(
            prepared_inputs,
            indicator_id,
            Some(output_id),
            params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::DeviceF32,
        )
        .map_err(|e| e.to_string())?;

        let row_owner = match out.series {
            DynamicIndicatorCudaSeries::DeviceF32(owner) => owner,
            DynamicIndicatorCudaSeries::HostF32(_) => {
                return Err("MKCS expected CUDA DeviceF32 output".to_string());
            }
        };

        prepared_inputs.runtime().copy_device_to_device_row(
            &row_owner,
            &mut matrix_owner,
            row_idx,
            cols,
        )?;
    }

    Ok(MkcsPeriodSweepDevice {
        matrix: MkcsDeviceMatrix::from_raw(matrix_owner, rows, cols),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}
```

**Important:** do not try to fake this with unsafe host-pointer tricks. Either use a real device-output API or keep the current host path until that API exists.

### 1.5 What not to change in `mkcs.rs`

Do **not** uninitialize `desired` in `build_mkcs_desired_positions`. The zero prefix is semantically visible output, and many indices may intentionally remain zero.

---

## 2) `mkcs_gpu_kernel.rs`

### Goals for this file

- reuse device scratch across repeated calls
- eliminate repeated device allocations where possible
- eliminate zero-fill host readback
- add a true uploaded-combo path
- improve host-side validation before launching the kernel

### 2.1 Add reusable scratch to `MkcsCudaRuntime`

Current `MkcsCudaRuntime` only stores context/module/stream. Extend it with a reusable scratch struct.

```rust
#[derive(Default)]
struct MkcsGpuScratch {
    close_host_ptr: usize,
    close_len: usize,
    d_close: Option<DeviceBuffer<f32>>,
    d_lr: Option<DeviceBuffer<f32>>,

    combo_len: usize,
    d_combo_stddev: Option<DeviceBuffer<i32>>,
    d_combo_breakout: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,

    out_len: usize,
    d_out: Option<DeviceBuffer<f32>>,
    host_out: Vec<f32>,
}

impl MkcsGpuScratch {
    fn ensure_close_uploaded(&mut self, close: &[f32]) -> Result<(), String> {
        let ptr = close.as_ptr() as usize;
        let needs_upload =
            self.d_close.is_none() || self.close_len != close.len() || self.close_host_ptr != ptr;

        if needs_upload {
            self.d_close = Some(DeviceBuffer::from_slice(close).map_err(|e| e.to_string())?);
            self.close_len = close.len();
            self.close_host_ptr = ptr;
        }

        Ok(())
    }

    fn ensure_lr(&mut self, len: usize) -> Result<(), String> {
        let needs_realloc = self.d_lr.as_ref().map_or(true, |buf| buf.len() < len);
        if needs_realloc {
            self.d_lr = Some(
                unsafe { DeviceBuffer::<f32>::uninitialized(len) }
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }

    fn ensure_i32_buffer(slot: &mut Option<DeviceBuffer<i32>>, len: usize) -> Result<(), String> {
        let needs_realloc = slot.as_ref().map_or(true, |buf| buf.len() < len);
        if needs_realloc {
            *slot = Some(
                unsafe { DeviceBuffer::<i32>::uninitialized(len) }
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }

    fn ensure_combo_buffers(&mut self, len: usize) -> Result<(), String> {
        Self::ensure_i32_buffer(&mut self.d_combo_stddev, len)?;
        Self::ensure_i32_buffer(&mut self.d_combo_breakout, len)?;
        Self::ensure_i32_buffer(&mut self.d_combo_start, len)?;
        Self::ensure_i32_buffer(&mut self.d_combo_end, len)?;
        self.combo_len = len;
        Ok(())
    }

    fn upload_combo_data(&mut self, combos: &MkcsGpuComboBatch<'_>) -> Result<(), String> {
        let len = combos.stddev_row_index.len();
        self.ensure_combo_buffers(len)?;

        self.d_combo_stddev
            .as_mut()
            .unwrap()
            .copy_from(combos.stddev_row_index)
            .map_err(|e| e.to_string())?;
        self.d_combo_breakout
            .as_mut()
            .unwrap()
            .copy_from(combos.breakout_row_index)
            .map_err(|e| e.to_string())?;
        self.d_combo_start
            .as_mut()
            .unwrap()
            .copy_from(combos.start_t)
            .map_err(|e| e.to_string())?;
        self.d_combo_end
            .as_mut()
            .unwrap()
            .copy_from(combos.end_t)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn ensure_out(&mut self, out_len: usize) -> Result<(), String> {
        let needs_realloc = self.d_out.as_ref().map_or(true, |buf| buf.len() < out_len);
        if needs_realloc {
            self.d_out = Some(
                unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }
                    .map_err(|e| e.to_string())?,
            );
        }
        self.out_len = out_len;

        if self.host_out.capacity() < out_len {
            self.host_out.reserve(out_len - self.host_out.capacity());
        }
        unsafe {
            self.host_out.set_len(out_len);
        }

        Ok(())
    }
}

struct MkcsCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    scratch: MkcsGpuScratch,
}
```

Update `new(...)`:

```rust
Ok(Self {
    context,
    module,
    stream,
    scratch: MkcsGpuScratch::default(),
})
```

### 2.2 Change log-return launch to use raw pointers, not borrowed buffers

This avoids borrow-checker pain once the buffers live inside runtime scratch.

```rust
fn launch_compute_log_returns_f32_raw(
    rt: &mut MkcsCudaRuntime,
    device_id: u32,
    close_ptr: u64,
    lr_ptr: u64,
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

    let mut close_ptr = close_ptr;
    let mut len_i =
        i32::try_from(len).map_err(|_| "MKCS GPU close length exceeds i32 range".to_string())?;
    let mut lr_ptr = lr_ptr;

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

### 2.3 Add stronger host-side combo validation before launch

This is a correctness improvement and helps avoid launching invalid kernels.

```rust
fn validate_inputs(
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("MKCS GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f32.len() != n {
        return Err("MKCS GPU close length mismatch".to_string());
    }
    if signals.stddev_row_major_ptr == 0
        || signals.upper_row_major_ptr == 0
        || signals.lower_row_major_ptr == 0
    {
        return Err("MKCS GPU signal pointer is null".to_string());
    }
    if signals.stddev_rows == 0
        || signals.stddev_stride == 0
        || signals.breakout_rows == 0
        || signals.breakout_stride == 0
    {
        return Err("MKCS GPU signal matrix shape is invalid".to_string());
    }

    let combos_len = combos.stddev_row_index.len();
    if combos.breakout_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("MKCS GPU combo vector length mismatch".to_string());
    }

    for i in 0..combos_len {
        let s_row = combos.stddev_row_index[i];
        let b_row = combos.breakout_row_index[i];
        let start_t = combos.start_t[i];
        let end_t = combos.end_t[i];

        if s_row < 0 || (s_row as usize) >= signals.stddev_rows {
            return Err(format!(
                "MKCS GPU stddev row index out of range at combo {i}: {s_row}"
            ));
        }
        if b_row < 0 || (b_row as usize) >= signals.breakout_rows {
            return Err(format!(
                "MKCS GPU breakout row index out of range at combo {i}: {b_row}"
            ));
        }
        if start_t < 0 || end_t < start_t || (end_t as usize) >= n {
            return Err(format!(
                "MKCS GPU time range invalid at combo {i}: start_t={start_t}, end_t={end_t}, len={n}"
            ));
        }
    }

    Ok(())
}
```

Also replace unchecked `as i32` conversions with `i32::try_from(...)` and return explicit errors.

### 2.4 Rework the evaluation path to reuse scratch

This is the core performance change in the launcher.

```rust
pub fn eval_mkcs_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    mkcs: &MkcsStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.stddev_row_index.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }

    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "MKCS GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_len = combo_count
        .checked_mul(MKCS_METRICS_STRIDE)
        .ok_or_else(|| "MKCS GPU metrics buffer overflow".to_string())?;

    MKCS_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, MkcsCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "MKCS GPU runtime missing".to_string())?;
        rt.set_current()?;

        rt.scratch.ensure_close_uploaded(signals.close_f32)?;
        rt.scratch.ensure_lr(signals.cols)?;
        rt.scratch.upload_combo_data(combos)?;
        rt.scratch.ensure_out(out_len)?;

        let close_ptr = rt
            .scratch
            .d_close
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw() as u64;
        let lr_ptr = rt
            .scratch
            .d_lr
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw() as u64;

        launch_compute_log_returns_f32_raw(rt, device_id, close_ptr, lr_ptr, signals.cols)?;

        let kernel = rt
            .module
            .get_function("mkcs_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let device_caps = detect_device_launch_caps(device);
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
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

        let mut close_ptr = close_ptr;
        let mut lr_ptr = lr_ptr;
        let mut stddev_ptr = signals.stddev_row_major_ptr;
        let mut stddev_rows_i = i32::try_from(signals.stddev_rows)
            .map_err(|_| "MKCS GPU stddev_rows exceeds i32 range".to_string())?;
        let mut stddev_stride_i = i32::try_from(signals.stddev_stride)
            .map_err(|_| "MKCS GPU stddev_stride exceeds i32 range".to_string())?;
        let mut upper_ptr = signals.upper_row_major_ptr;
        let mut lower_ptr = signals.lower_row_major_ptr;
        let mut breakout_rows_i = i32::try_from(signals.breakout_rows)
            .map_err(|_| "MKCS GPU breakout_rows exceeds i32 range".to_string())?;
        let mut breakout_stride_i = i32::try_from(signals.breakout_stride)
            .map_err(|_| "MKCS GPU breakout_stride exceeds i32 range".to_string())?;
        let mut combo_stddev_ptr = rt.scratch.d_combo_stddev.as_ref().unwrap().as_device_ptr().as_raw();
        let mut combo_breakout_ptr = rt.scratch.d_combo_breakout.as_ref().unwrap().as_device_ptr().as_raw();
        let mut combo_start_ptr = rt.scratch.d_combo_start.as_ref().unwrap().as_device_ptr().as_raw();
        let mut combo_end_ptr = rt.scratch.d_combo_end.as_ref().unwrap().as_device_ptr().as_raw();
        let mut combo_count_i = i32::try_from(combo_count)
            .map_err(|_| "MKCS GPU combo_count exceeds i32 range".to_string())?;
        let mut t_i = i32::try_from(signals.cols)
            .map_err(|_| "MKCS GPU len exceeds i32 range".to_string())?;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut compression_threshold_f = mkcs.compression_threshold as f32;
        let mut reexpansion_threshold_f = mkcs.reexpansion_threshold as f32;
        let mut out_stride_i = MKCS_METRICS_STRIDE as i32;
        let mut out_ptr = rt.scratch.d_out.as_ref().unwrap().as_device_ptr().as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut stddev_ptr as *mut _ as *mut c_void,
                &mut stddev_rows_i as *mut _ as *mut c_void,
                &mut stddev_stride_i as *mut _ as *mut c_void,
                &mut upper_ptr as *mut _ as *mut c_void,
                &mut lower_ptr as *mut _ as *mut c_void,
                &mut breakout_rows_i as *mut _ as *mut c_void,
                &mut breakout_stride_i as *mut _ as *mut c_void,
                &mut combo_stddev_ptr as *mut _ as *mut c_void,
                &mut combo_breakout_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut compression_threshold_f as *mut _ as *mut c_void,
                &mut reexpansion_threshold_f as *mut _ as *mut c_void,
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

        let host_metrics = &mut rt.scratch.host_out[..out_len];
        rt.scratch
            .d_out
            .as_ref()
            .unwrap()
            .copy_to(host_metrics)
            .map_err(|e| e.to_string())?;

        let mut out = Vec::<Metrics>::with_capacity(combo_count);
        for row in host_metrics.chunks_exact(MKCS_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }

        Ok(out)
    })
}
```

### 2.5 Add a true uploaded-combo API

The runtime scratch reuse above eliminates repeated **allocation**, but it still copies combo data host->device each call. If the optimizer or validation path reuses the same combos many times, add a public upload-once type.

```rust
pub struct MkcsUploadedComboBatch {
    pub len: usize,
    d_stddev_row_index: DeviceBuffer<i32>,
    d_breakout_row_index: DeviceBuffer<i32>,
    d_start_t: DeviceBuffer<i32>,
    d_end_t: DeviceBuffer<i32>,
}

impl MkcsUploadedComboBatch {
    pub fn from_host(combos: &MkcsGpuComboBatch<'_>) -> Result<Self, String> {
        let len = combos.stddev_row_index.len();
        if combos.breakout_row_index.len() != len
            || combos.start_t.len() != len
            || combos.end_t.len() != len
        {
            return Err("MKCS GPU combo vector length mismatch".to_string());
        }

        Ok(Self {
            len,
            d_stddev_row_index: DeviceBuffer::from_slice(combos.stddev_row_index)
                .map_err(|e| e.to_string())?,
            d_breakout_row_index: DeviceBuffer::from_slice(combos.breakout_row_index)
                .map_err(|e| e.to_string())?,
            d_start_t: DeviceBuffer::from_slice(combos.start_t)
                .map_err(|e| e.to_string())?,
            d_end_t: DeviceBuffer::from_slice(combos.end_t)
                .map_err(|e| e.to_string())?,
        })
    }
}
```

Then add a second eval entry point:

```rust
pub fn eval_mkcs_metrics_gpu_kernel_uploaded_rows(
    device_id: u32,
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsUploadedComboBatch,
    strategy: &StrategyConfig,
    mkcs: &MkcsStrategyParams,
) -> Result<Vec<Metrics>, String> {
    // same body as the main evaluator, but use the pre-uploaded combo device buffers
}
```

Keep the current API as a convenience wrapper:

```rust
pub fn eval_mkcs_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    mkcs: &MkcsStrategyParams,
) -> Result<Vec<Metrics>, String> {
    let uploaded = MkcsUploadedComboBatch::from_host(combos)?;
    eval_mkcs_metrics_gpu_kernel_uploaded_rows(device_id, signals, &uploaded, strategy, mkcs)
}
```

### 2.6 Keep uninitialized memory usage narrow and safe

Use uninitialized memory only in these cases:

- `DeviceBuffer::<f32>::uninitialized(...)` for `d_lr` and `d_out` — already safe because kernels overwrite them
- `DeviceBuffer::<i32>::uninitialized(...)` for combo buffers — safe if immediately `copy_from(...)` fills them
- reusable `Vec<f32>` host readback buffer — safe only because `copy_to(...)` fills every element before any read

Do **not** use uninitialized memory for:

- `Vec<Metrics>`
- any buffer that may be partially read before being fully written
- semantically visible zero-prefix series like `desired`

---

## 3) `gpu_native_expansion_opt.rs`

### Goals for this file

- add MKCS end-to-end CPU/GPU optimizer benches
- add MKCS phase-split benches so you can measure staging separately from kernel scoring
- ensure route-probe output is present for MKCS too

### 3.1 Add MKCS request builder and synthetic candles

```rust
use ta_app_core::{
    Backend, BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeCpuRequest,
    CompositeSignalMode, KqsrStrategyParams, KvdfStrategyParams, MkcsStrategyParams,
};

fn make_mkcs_squeeze_candles(len: usize) -> Candles {
    let len = len.max(320);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);

        let phase = i % 180;
        let (drift, amp) = if phase < 50 {
            (0.0, 0.05)          // tight squeeze
        } else if phase < 90 {
            (0.03 * (phase - 50) as f64, 0.08) // expansion
        } else if phase < 130 {
            (1.2 + 0.14 * (phase - 90) as f64, 0.20) // breakout trend
        } else {
            (6.8 - 0.12 * (phase - 130) as f64, 0.11) // fade / reset
        };

        let base = 100.0 + drift + ((i as f64) * 0.17).sin() * amp;
        open.push(base * 0.999);
        high.push(base * 1.004);
        low.push(base * 0.996);
        close.push(base);
        vol.push(1_200.0 + (phase as f64) * 3.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_mkcs_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.mkcs = Some(MkcsStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}
```

### 3.2 Add MKCS end-to-end CPU and GPU benches

```rust
fn bench_mkcs_cpu(c: &mut Criterion) {
    if !bench_target_enabled("mkcs") {
        return;
    }
    let candles = make_mkcs_squeeze_candles(200_000);
    bench_cpu_cases(c, "mkcs/cpu/200k", &candles, make_mkcs_req);
}

fn bench_mkcs_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("mkcs") {
        return;
    }
    let candles = make_mkcs_squeeze_candles(200_000);
    bench_gpu_cases(
        c,
        "mkcs/gpu_kernel/200k",
        "mkcs/gpu_kernel/200k",
        &candles,
        make_mkcs_req,
    );
}
```

Add them to `criterion_group!`:

```rust
criterion_group!(
    gpu_native_expansion_benches,
    bench_mkcs_cpu,
    bench_mkcs_gpu_kernel,
    bench_kqsr_cpu,
    bench_kqsr_gpu_kernel,
    bench_carm_cpu,
    bench_carm_gpu_kernel,
    bench_kvdf_cpu,
    bench_kvdf_gpu_kernel,
    bench_cdmc_cpu,
    bench_cdmc_gpu_kernel
);
```

### 3.3 Add phase-split MKCS microbenches

This is extremely important for separating:

- sweep prep cost
- kernel scoring cost
- full route cost

Use public MKCS helpers directly.

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use ta_app_core::dynamic_indicator_adapter::PreparedCudaIndicatorInputs;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use ta_app_core::mkcs::{
    compute_mkcs_donchian_output_sweep_cuda_device,
    compute_mkcs_stddev_sweep_cuda_device,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use ta_app_core::mkcs_gpu_kernel::{
    eval_mkcs_metrics_gpu_kernel_device_rows, MkcsGpuComboBatch, MkcsGpuDeviceSignalMatrices,
};

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_mkcs_gpu_phases(c: &mut Criterion) {
    if !bench_target_enabled("mkcs") || !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_mkcs_squeeze_candles(200_000);
    let prepared =
        PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).expect("mkcs prepared");

    let close_f32: Vec<f32> = candles.close.iter().map(|&v| v as f32).collect();
    let mkcs_cfg = MkcsStrategyParams::default();

    let mut prep_group = c.benchmark_group("mkcs/gpu_phases/prep/200k");
    prep_group.measurement_time(Duration::from_secs(8));
    prep_group.sample_size(10);

    prep_group.bench_function("stddev_sweep_5_80", |b| {
        b.iter(|| {
            let _ = compute_mkcs_stddev_sweep_cuda_device(&prepared, 5, 80, 1)
                .expect("stddev sweep");
        })
    });

    prep_group.bench_function("donchian_upper_sweep_20_220", |b| {
        b.iter(|| {
            let _ = compute_mkcs_donchian_output_sweep_cuda_device(
                &prepared,
                &["upperband", "upper", "value"],
                20,
                220,
                1,
            )
            .expect("upper sweep");
        })
    });

    prep_group.bench_function("donchian_lower_sweep_20_220", |b| {
        b.iter(|| {
            let _ = compute_mkcs_donchian_output_sweep_cuda_device(
                &prepared,
                &["lowerband", "lower"],
                20,
                220,
                1,
            )
            .expect("lower sweep");
        })
    });

    prep_group.finish();

    let stddev = compute_mkcs_stddev_sweep_cuda_device(&prepared, 5, 80, 1)
        .expect("stddev ready");
    let upper = compute_mkcs_donchian_output_sweep_cuda_device(
        &prepared,
        &["upperband", "upper", "value"],
        20,
        220,
        1,
    )
    .expect("upper ready");
    let lower = compute_mkcs_donchian_output_sweep_cuda_device(
        &prepared,
        &["lowerband", "lower"],
        20,
        220,
        1,
    )
    .expect("lower ready");

    let combo_count = 256usize;
    let stddev_row_index: Vec<i32> = (0..combo_count)
        .map(|i| (i % stddev.matrix.rows) as i32)
        .collect();
    let breakout_row_index: Vec<i32> = (0..combo_count)
        .map(|i| (i % upper.matrix.rows) as i32)
        .collect();
    let start_t: Vec<i32> = vec![64; combo_count];
    let end_t: Vec<i32> = vec![(candles.close.len() - 1) as i32; combo_count];

    let signals = MkcsGpuDeviceSignalMatrices {
        close_f32: close_f32.as_slice(),
        cols: close_f32.len(),
        stddev_row_major_ptr: stddev.matrix.device_ptr,
        stddev_rows: stddev.matrix.rows,
        stddev_stride: stddev.matrix.cols,
        upper_row_major_ptr: upper.matrix.device_ptr,
        lower_row_major_ptr: lower.matrix.device_ptr,
        breakout_rows: upper.matrix.rows,
        breakout_stride: upper.matrix.cols,
    };
    let combos = MkcsGpuComboBatch {
        stddev_row_index: stddev_row_index.as_slice(),
        breakout_row_index: breakout_row_index.as_slice(),
        start_t: start_t.as_slice(),
        end_t: end_t.as_slice(),
    };

    let mut score_group = c.benchmark_group("mkcs/gpu_phases/score/200k");
    score_group.measurement_time(Duration::from_secs(8));
    score_group.sample_size(10);

    score_group.bench_function("score_256candidates", |b| {
        b.iter(|| {
            let _ = eval_mkcs_metrics_gpu_kernel_device_rows(
                0,
                &signals,
                &combos,
                &StrategyConfig::default(),
                &mkcs_cfg,
            )
            .expect("mkcs score");
        })
    });

    score_group.finish();
}
```

Add it to the criterion group if you want the phase-split evidence in the same bench target.

### 3.4 Why these MKCS benches matter

Without these benches, you cannot answer these basic questions for MKCS:

- is sweep prep the real bottleneck?
- is kernel scoring already fast enough?
- do scratch-reuse changes improve score-only cost, end-to-end cost, or both?
- is MKCS GPU-preferred only at large pair counts, or already at medium?

---

## 4) `composite_validation_workflow.rs`

### Goals for this file

- bring MKCS validation coverage closer to ASR
- expose validation throughput cliffs for MKCS
- keep route-probe output visible

### 4.1 Add dedicated MKCS `100k/64-candidate` and heavy in-sample groups

Mirror the existing ASR group.

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_mkcs_validation_gpu_workflow(c: &mut Criterion) {
    let candles = make_synth_candles(100_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_mkcs_candidates(64);

    let mut group = c.benchmark_group("composite_validate/gpu_kernel/mkcs/100k/64candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let base = CompositeValidationRequest {
        data_id: "bench-gpu-mkcs".to_string(),
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
    .expect("mkcs gpu validation route probe");

    println!(
        "benchmark route composite_validate/gpu_kernel/mkcs/100k/64candidates: backend_used={:?} backend_route={} fallback={}",
        route_probe.backend_used,
        route_probe.backend_route.as_deref().unwrap_or("<none>"),
        route_probe
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("<none>")
    );

    group.bench_function("wf_cscv", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("mkcs gpu wf+cscv validate");
        })
    });

    let mut vram_heavy = base.clone();
    vram_heavy.candidates = make_mkcs_candidates(192);
    vram_heavy.max_candidates = vram_heavy.candidates.len();
    vram_heavy.walk_forward = None;
    vram_heavy.cscv_pbo = None;

    group.bench_function("in_sample_192candidates", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                vram_heavy.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("mkcs gpu in-sample validate");
        })
    });

    group.finish();
}
```

Then add it to `criterion_group!`:

```rust
criterion_group!(
    benches,
    bench_composite_validation_workflow,
    bench_composite_validation_gpu_workflow,
    bench_mkcs_validation_gpu_workflow
);
```

### 4.2 Expand generic profile coverage for MKCS

Also add MKCS to the larger generic profiles so you can compare it against the existing CARM/CDMC 100k shapes.

```rust
bench_cpu_profile("mkcs", 100_000, make_mkcs_candidates(32));
```

and in the GPU profile block:

```rust
bench_profile("mkcs", 100_000, make_mkcs_candidates(32));
```

### 4.3 Optional cleanup: factor the ASR and MKCS dedicated groups through one helper

This is not a performance change by itself, but it is worth doing to reduce benchmark-shape drift between strategies.

---

## Benchmark and perf-gate review

## What is already good

- both benchmark files print route information
- GPU route probes already assert GPU use in the optimizer bench
- the validation bench already distinguishes CPU and GPU profile groups

## What is currently missing for MKCS

### In `gpu_native_expansion_opt.rs`

Missing:

- MKCS end-to-end CPU optimizer benchmark
- MKCS end-to-end GPU optimizer benchmark
- MKCS prep-only microbench
- MKCS score-only microbench

### In `composite_validation_workflow.rs`

Missing:

- dedicated MKCS `100k/64-candidate` GPU validation benchmark
- dedicated MKCS `192-candidate` in-sample stress benchmark
- larger `100k/32-candidate` CPU/GPU profile entries for MKCS

## Recommended benchmark order

### Step 1: establish baseline

Run baseline MKCS benches before editing:

- end-to-end CPU optimize
- end-to-end GPU optimize
- sweep prep only
- score only
- validation `50k/8`
- validation `100k/64`
- validation `192` in-sample

### Step 2: land `mkcs_gpu_kernel.rs` scratch reuse

Re-run the score-only bench first. If that does not improve, the bottleneck is probably not the kernel launcher.

### Step 3: land `mkcs.rs` sweep prep changes

Re-run the prep-only benches. This should move prep cost even before the full device-native sweep change.

### Step 4: land device-native sweep support when adjacent APIs are ready

Re-run prep-only and end-to-end benches. This is where the large prep-side win should appear.

## Suggested evidence boundaries

Keep conclusions tied to the right timing boundary:

- **Criterion:** use the new MKCS benches in `gpu_native_expansion_opt.rs` and `composite_validation_workflow.rs`
- **validation-cost:** use the dedicated MKCS validation groups
- **app-path:** only infer after the Criterion surfaces improve
- **profiling-only:** inspect device kernel only if host-side changes still leave MKCS slow

---

## Test review and additions

## What exists today

`mkcs.rs` currently has:

- parameter validation tests
- a basic desired-position behavior test
- a GPU smoke test that checks signal lengths/finiteness

That is not enough for parity-safe CUDA optimization.

## Add these tests in `mkcs.rs`

### 1) Threshold-edge behavior tests

Add explicit tests around compression and re-expansion thresholds.

```rust
#[test]
fn build_mkcs_desired_positions_threshold_edges_are_stable() {
    let cfg = MkcsStrategyParams {
        compression_threshold: 0.01,
        reexpansion_threshold: 0.03,
        ..MkcsStrategyParams::default()
    };

    let close = vec![100.0, 100.0, 100.0, 100.0, 100.0];
    let compression = MkcsCompressionSignals {
        stddev: vec![f64::NAN, 1.0, 0.999999999, 3.0, 3.000000001],
        first_valid: 1,
    };
    let breakout = MkcsBreakoutSignals {
        upper: vec![f64::NAN, 100.1, 99.9, 99.9, 99.9],
        lower: vec![f64::NAN,  99.9, 99.8, 99.8, 99.8],
        first_valid: 1,
    };

    let strategy = StrategyConfig::default();
    let desired = build_mkcs_desired_positions(&close, &compression, &breakout, 2, &strategy, &cfg);

    // fill in expected behavior for exact and epsilon-edge cases
    assert_eq!(desired.len(), close.len());
}
```

### 2) Flag-matrix behavior tests

Add explicit tests for:

- `long_only = true`
- `allow_flip = false`
- `trade_on_next_bar = true`

### 3) CPU vs GPU signal parity tests

Use the existing public signal functions.

```rust
#[cfg(feature = "cuda")]
#[test]
fn compute_mkcs_signals_gpu_matches_cpu_within_tolerance() {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_trending_candles(1024);
    let prepared = PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
        .expect("cuda inputs");

    let cpu_compression = compute_mkcs_compression_signals(
        &candles,
        "close",
        &crate::double_ma::Backend::CpuOnly,
        20,
        None,
    )
    .expect("cpu compression");

    let gpu_compression = compute_mkcs_compression_signals(
        &candles,
        "close",
        &crate::double_ma::Backend::GpuOnly { device_id: 0 },
        20,
        Some(&prepared),
    )
    .expect("gpu compression");

    assert_eq!(cpu_compression.stddev.len(), gpu_compression.stddev.len());

    for (cpu, gpu) in cpu_compression.stddev.iter().zip(gpu_compression.stddev.iter()) {
        if cpu.is_finite() && gpu.is_finite() {
            assert!(
                (cpu - gpu).abs() <= 1.0e-4,
                "stddev mismatch: cpu={cpu}, gpu={gpu}"
            );
        }
    }
}
```

Repeat the same pattern for Donchian upper/lower.

## Add these tests in `mkcs_gpu_kernel.rs`

### 4) Input validation tests

These do not require actual CUDA execution.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_inputs_rejects_combo_length_mismatch() {
        let close = vec![100.0f32, 101.0, 102.0, 103.0];
        let signals = MkcsGpuDeviceSignalMatrices {
            close_f32: close.as_slice(),
            cols: close.len(),
            stddev_row_major_ptr: 1,
            stddev_rows: 1,
            stddev_stride: close.len(),
            upper_row_major_ptr: 1,
            lower_row_major_ptr: 1,
            breakout_rows: 1,
            breakout_stride: close.len(),
        };
        let combos = MkcsGpuComboBatch {
            stddev_row_index: &[0, 0],
            breakout_row_index: &[0],
            start_t: &[1, 1],
            end_t: &[3, 3],
        };

        let err = validate_inputs(&signals, &combos).unwrap_err();
        assert!(err.contains("length mismatch"));
    }

    #[test]
    fn validate_inputs_rejects_out_of_range_rows() {
        let close = vec![100.0f32, 101.0, 102.0, 103.0];
        let signals = MkcsGpuDeviceSignalMatrices {
            close_f32: close.as_slice(),
            cols: close.len(),
            stddev_row_major_ptr: 1,
            stddev_rows: 1,
            stddev_stride: close.len(),
            upper_row_major_ptr: 1,
            lower_row_major_ptr: 1,
            breakout_rows: 1,
            breakout_stride: close.len(),
        };
        let combos = MkcsGpuComboBatch {
            stddev_row_index: &[1],
            breakout_row_index: &[0],
            start_t: &[1],
            end_t: &[3],
        };

        let err = validate_inputs(&signals, &combos).unwrap_err();
        assert!(err.contains("row index out of range"));
    }
}
```

### 5) Add an overlay-blocker test if the `StrategyConfig` fields are directly constructible

If `StrategyConfig` allows easy construction of stop-loss / take-profit / overlay settings, add a test that asserts the exact blocker string before any device work begins.

---

## Allocation and copy-elision plan

## Safe uninitialized memory opportunities

These are the places where uninitialized memory is justified.

### Safe

- `DeviceBuffer::<f32>::uninitialized(len)` for `d_lr`
- `DeviceBuffer::<f32>::uninitialized(out_len)` for `d_out`
- `DeviceBuffer::<i32>::uninitialized(len)` for combo buffers that are immediately filled via `copy_from(...)`
- host `Vec<f32>` readback scratch when `copy_to(...)` overwrites every element
- reusable host `Vec<f32>` scratch with `set_len(out_len)` for the same reason

### Not safe / not appropriate

- `desired` in `build_mkcs_desired_positions`
- any buffer whose prefix zeros or NaNs are behaviorally observable
- any buffer of non-POD values without full overwrite guarantees
- any destination whose producer might early-return before full population

## Copy-elision priorities

### Priority 1: remove the biggest unnecessary copies

1. **GPU -> host -> host append -> GPU** in sweep prep
2. repeated **host close -> device** upload
3. repeated **host combo -> device** upload
4. zero-fill of host readback before D2H copy

### Priority 2: remove smaller host churn

1. repeated parameter `Vec` allocation in sweep loops
2. repeated output/key resolution
3. repeated scalar conversions and checked field access in CPU reference loop

## Recommended helper policy

Prefer the repo’s shared allocation helper from `alloc_policy` if it already exposes:

- POD uninitialized host vectors
- matrix builders
- debug-poison helpers for testing

Only fall back to local `unsafe` helpers if that shared helper is not available to these files. If you do add a local helper, keep it:

- type-specific (`f32`, `i32`) instead of generic
- documented as POD-only
- used only at full-overwrite boundaries

---

## Suggested implementation order

## Phase 0: baseline and guardrails

1. Add MKCS optimizer and validation benches first
2. Record baseline numbers
3. Add missing parity tests before deeper CUDA edits

## Phase 1: launcher-side reuse

1. Extend `MkcsCudaRuntime` with `MkcsGpuScratch`
2. Reuse `d_close`, `d_lr`, combo buffers, `d_out`, and host readback
3. Add row/time-range validation
4. Re-run score-only and end-to-end MKCS benches

**Expected result:** score-only cost should improve first.

## Phase 2: sweep-side cleanup

1. Cache indicator IDs
2. Reuse period params in the host-row sweep helper
3. Tighten CPU reference desired-position loop
4. Re-run prep-only and end-to-end benches

**Expected result:** small-to-moderate prep improvement even before device-native sweeps.

## Phase 3: device-native sweeps

1. Extend adjacent dynamic indicator support for `DeviceF32` sweeps
2. Add `compute_indicator_period_sweep_cuda_device_rows(...)`
3. Use it for stddev and Donchian surfaces
4. Re-run prep-only and end-to-end benches

**Expected result:** this should produce the biggest prep-side win.

## Phase 4: uploaded combos and deeper route work

1. Add `MkcsUploadedComboBatch`
2. Use the uploaded path from repeated validation/optimization loops where possible
3. Add score-only bench with uploaded combos

**Expected result:** lower repeated-call H2D cost when the same combos are re-run.

## Phase 5: only if still needed

1. inspect the actual device kernel implementation
2. consider fusing log-return generation
3. consider combined Donchian multi-output fetch

---

## Expected payoff by change

| Change | Likely payoff | Risk | Notes |
|---|---:|---:|---|
| `mkcs_gpu_kernel.rs` scratch reuse | High | Low | Best short-term win |
| uninitialized/reusable host readback | Small-Medium | Low | Easy and safe |
| combo upload-once API | Medium | Low-Medium | Strong for repeated validation loops |
| parameter/key caching in `mkcs.rs` | Small | Low | Worth doing but not enough alone |
| CPU desired-position loop tightening | Small-Medium | Low | Helps CPU fallback/reference cost |
| device-native sweep path | High | Medium | Best prep-side fix, needs adjacent support |
| combined Donchian multi-output fetch | Medium | Medium | Needs adjacent dispatch support |
| fused log-return kernel | Unknown-Medium | Medium-High | Do later, only with evidence |

---

## Detailed implementation checklist

## `mkcs.rs`

- [ ] Add `MkcsIndicatorIds` cache with `OnceLock`
- [ ] Replace repeated `resolve_param_key(...)` and `resolve_output_id(...)` calls with cached values
- [ ] Add checked `rows * cols` overflow handling in `compute_indicator_period_sweep_cuda_host_rows`
- [ ] Reuse the `params` vector inside the sweep loop instead of reallocating it every period
- [ ] Keep `row_major` as `Vec::with_capacity(total)`; do **not** zero-fill it
- [ ] Add `debug_assert_eq!(row_major.len(), total)` after the sweep loop
- [ ] Tighten `build_mkcs_desired_positions`:
  - [ ] hoist flags and thresholds out of the loop
  - [ ] replace `stddev_ratio(...)` division with `abs(stddev) <= abs(close) * threshold`
  - [ ] replace `.get(...).copied().unwrap_or(...)` with direct indexed access after length validation
- [ ] Add CPU threshold-edge tests
- [ ] Add flag-matrix behavior tests (`long_only`, `allow_flip`, `trade_on_next_bar`)
- [ ] Add CPU vs GPU signal parity tests for stddev
- [ ] Add CPU vs GPU signal parity tests for Donchian upper/lower
- [ ] Optional: add a `_with_route(...)` or logging variant for indicator fallback visibility
- [ ] Optional/adjacent-file: add device-native sweep support for `DeviceF32`

## `mkcs_gpu_kernel.rs`

- [ ] Extend `MkcsCudaRuntime` with `MkcsGpuScratch`
- [ ] Add reusable `d_close`
- [ ] Add reusable `d_lr`
- [ ] Add reusable combo buffers
- [ ] Add reusable `d_out`
- [ ] Add reusable host `Vec<f32>` readback scratch
- [ ] Change log-return launch helper to use raw pointers or another borrow-checker-safe form
- [ ] Replace unchecked `as i32` conversions with `i32::try_from(...)`
- [ ] Add host-side row-index validation
- [ ] Add host-side `start_t` / `end_t` range validation
- [ ] Keep overlay/SL/TP blocker check before any real device work
- [ ] Replace zero-filled `host_metrics` allocation with reusable/uninitialized POD scratch
- [ ] Add unit test for combo length mismatch
- [ ] Add unit test for row-index out-of-range
- [ ] Add unit test for invalid time ranges
- [ ] Add overlay-blocker unit test if `StrategyConfig` construction is straightforward
- [ ] Add `MkcsUploadedComboBatch`
- [ ] Add `eval_mkcs_metrics_gpu_kernel_uploaded_rows(...)`
- [ ] Keep the current API as a wrapper around the uploaded path

## `gpu_native_expansion_opt.rs`

- [ ] Import `MkcsStrategyParams`
- [ ] Add `make_mkcs_squeeze_candles(...)`
- [ ] Add `make_mkcs_req(...)`
- [ ] Add `bench_mkcs_cpu(...)`
- [ ] Add `bench_mkcs_gpu_kernel(...)`
- [ ] Add MKCS benches to the `criterion_group!`
- [ ] Add phase-split prep-only MKCS bench group
- [ ] Add phase-split score-only MKCS bench group
- [ ] Optional: add uploaded-combo score-only bench after that API exists
- [ ] Keep route-probe printing and backend assertions for MKCS

## `composite_validation_workflow.rs`

- [ ] Add dedicated `composite_validate/gpu_kernel/mkcs/100k/64candidates` group
- [ ] Add dedicated MKCS `in_sample_192candidates` stress case
- [ ] Add `bench_cpu_profile("mkcs", 100_000, make_mkcs_candidates(32))`
- [ ] Add `bench_profile("mkcs", 100_000, make_mkcs_candidates(32))`
- [ ] Keep route-probe printing for MKCS validation
- [ ] Optional: factor ASR and MKCS dedicated GPU validation groups through one helper to prevent benchmark-shape drift

## Benchmark / validation process

- [ ] Record baseline MKCS optimizer CPU times
- [ ] Record baseline MKCS optimizer GPU times
- [ ] Record baseline MKCS prep-only times
- [ ] Record baseline MKCS score-only times
- [ ] Record baseline MKCS validation `50k/8`
- [ ] Record baseline MKCS validation `100k/64`
- [ ] Record baseline MKCS validation `192` in-sample
- [ ] Re-run all MKCS benches after Phase 1
- [ ] Re-run all MKCS benches after Phase 2
- [ ] Re-run all MKCS benches after Phase 3
- [ ] Compare score-only improvement separately from end-to-end improvement
- [ ] Verify no benchmark route changed unexpectedly
- [ ] Verify fallback reasons stay visible in benchmark output where applicable

## Parity / correctness guardrails

- [ ] Confirm CPU vs GPU stddev parity within chosen tolerance
- [ ] Confirm CPU vs GPU Donchian parity within chosen tolerance
- [ ] Confirm trade count parity on representative MKCS combo sets
- [ ] Confirm threshold-edge behavior around `compression_threshold ± ε`
- [ ] Confirm threshold-edge behavior around `reexpansion_threshold ± ε`
- [ ] Confirm `long_only` behavior
- [ ] Confirm `allow_flip` behavior
- [ ] Confirm `trade_on_next_bar` behavior
- [ ] Confirm overlay/SL/TP blocker behavior remains explicit and stable

---

## Bottom line

The highest-confidence, parity-safe MKCS improvements in the uploaded files are **not** deeper device-math changes first. They are:

1. **stop redoing uploads and allocations in `mkcs_gpu_kernel.rs`**
2. **measure MKCS directly in the optimizer and validation benchmarks**
3. **remove sweep-side host churn in `mkcs.rs` now**
4. **replace the sweep round-trip with a true device-native path when the adjacent indicator adapter can support it**

If you implement only one thing first, implement the **runtime scratch reuse + reusable host readback + stronger benchmark coverage**. If you implement the second thing after that, implement the **device-native sweep path**.


---

# V2 addendum: corrections, missed items, and optional performance work

This addendum is the authoritative update to the original plan. Implement this section together with the original document, and treat the items explicitly marked **supersedes v1** as replacing the earlier suggestion.

## Why this addendum exists

After re-checking `mkcs.rs`, `mkcs_gpu_kernel.rs`, `gpu_native_expansion_opt.rs`, and `composite_validation_workflow.rs`, there are a few important improvements and a couple of corrections worth making before implementation:

1. one of the proposed reuse keys in v1 was too weak for correctness (`close.as_ptr()` + len is not a safe identity)
2. the uploaded-combo API in v1 needs explicit device affinity
3. the benchmark plan should cover **steady-state vs cold-path** separately
4. MKCS validation candidate generation currently varies only MA lengths, not MKCS-local thresholds / mapping flags
5. there are a few additional high-value optional changes that go beyond the strict Section 5 prompt but still align with your stated goal: **better performance, fewer copies, and preserved functionality**

---

## A. Corrections to the original plan

## A.1 Supersedes v1: do **not** key close-buffer reuse only by host pointer + length

The original sketch in v1 used:

- `close_host_ptr`
- `close_len`

to decide whether `d_close` could be reused.

That is **not strong enough** as a correctness key. A caller can reuse the same backing allocation and mutate values in place while the pointer and length remain unchanged. In that case, the GPU upload would be skipped incorrectly and stale market data would be used.

### Recommended correction

Use one of these instead:

1. **best:** explicit caller-provided immutable identity / generation key
2. **next best:** a device-resident close / log-return handle that already implies data identity
3. **fallback:** content hash, but only if there is no practical way to provide a generation key

### Revised public shape

```rust
pub struct MkcsGpuMarketSeries<'a> {
    pub len: usize,

    // Stable identity chosen by the caller. Must change whenever the underlying
    // market data content changes, even if the slice pointer is reused.
    pub market_series_key: u64,

    // Host close is optional when the caller already has device-resident close.
    pub close_host_f32: Option<&'a [f32]>,

    // Optional device-resident close series. If present, it should be preferred
    // over host upload.
    pub close_device_ptr: Option<u64>,

    // Optional precomputed device-resident log-return series.
    pub lr_device_ptr: Option<u64>,

    // Device ownership for close_device_ptr / lr_device_ptr when present.
    pub device_id: Option<u32>,
}

pub struct MkcsGpuDeviceSignalMatrices<'a> {
    pub market: MkcsGpuMarketSeries<'a>,

    // Device that owns the row-major signal matrices.
    pub signal_device_id: u32,

    pub cols: usize,
    pub stddev_row_major_ptr: u64,
    pub stddev_rows: usize,
    pub stddev_stride: usize,
    pub upper_row_major_ptr: u64,
    pub lower_row_major_ptr: u64,
    pub breakout_rows: usize,
    pub breakout_stride: usize,
}
```

### Revised scratch for market-series reuse

```rust
#[derive(Default)]
struct MkcsGpuScratch {
    market_series_key: u64,
    close_len: usize,
    d_close: Option<DeviceBuffer<f32>>,
    d_lr: Option<DeviceBuffer<f32>>,
    lr_from_market_series_key: u64,

    combo_len: usize,
    d_combo_stddev: Option<DeviceBuffer<i32>>,
    d_combo_breakout: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,

    out_len: usize,
    d_out: Option<DeviceBuffer<f32>>,
    host_out: Vec<f32>,
}

impl MkcsGpuScratch {
    fn ensure_market_series(
        &mut self,
        market: &MkcsGpuMarketSeries<'_>,
    ) -> Result<(u64, u64), String> {
        if market.len == 0 {
            return Err("MKCS GPU market series must be non-empty".to_string());
        }

        // Fast path 1: caller already supplies device-resident close and lr.
        if let (Some(close_ptr), Some(lr_ptr)) = (market.close_device_ptr, market.lr_device_ptr) {
            return Ok((close_ptr, lr_ptr));
        }

        // Fast path 2: caller supplies device close but not device lr.
        if let Some(close_ptr) = market.close_device_ptr {
            let need_lr = self.d_lr.is_none()
                || self.close_len != market.len
                || self.lr_from_market_series_key != market.market_series_key;
            if need_lr {
                let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(market.len) }
                    .map_err(|e| e.to_string())?;
                // launch log-return kernel using the supplied close pointer
                // and d_lr.as_device_ptr().as_raw()
                self.d_lr = Some(d_lr);
                self.lr_from_market_series_key = market.market_series_key;
                self.close_len = market.len;
            }
            let lr_ptr = self
                .d_lr
                .as_ref()
                .ok_or_else(|| "MKCS GPU missing d_lr after ensure_market_series".to_string())?
                .as_device_ptr()
                .as_raw() as u64;
            return Ok((close_ptr, lr_ptr));
        }

        // Host-upload path.
        let close = market
            .close_host_f32
            .ok_or_else(|| "MKCS GPU requires host close_f32 or device close pointer".to_string())?;

        if close.len() != market.len {
            return Err(format!(
                "MKCS GPU host close length mismatch: expected {}, got {}",
                market.len,
                close.len()
            ));
        }

        let need_close_upload = self.d_close.is_none()
            || self.close_len != market.len
            || self.market_series_key != market.market_series_key;

        if need_close_upload {
            self.d_close = Some(DeviceBuffer::from_slice(close).map_err(|e| e.to_string())?);
            self.market_series_key = market.market_series_key;
            self.close_len = market.len;
            self.lr_from_market_series_key = 0;
        }

        let need_lr = self.d_lr.is_none()
            || self.close_len != market.len
            || self.lr_from_market_series_key != market.market_series_key;

        if need_lr {
            let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(market.len) }
                .map_err(|e| e.to_string())?;
            // launch log-return kernel against self.d_close + d_lr
            self.d_lr = Some(d_lr);
            self.lr_from_market_series_key = market.market_series_key;
        }

        let close_ptr = self
            .d_close
            .as_ref()
            .ok_or_else(|| "MKCS GPU missing d_close after ensure_market_series".to_string())?
            .as_device_ptr()
            .as_raw() as u64;
        let lr_ptr = self
            .d_lr
            .as_ref()
            .ok_or_else(|| "MKCS GPU missing d_lr after ensure_market_series".to_string())?
            .as_device_ptr()
            .as_raw() as u64;

        Ok((close_ptr, lr_ptr))
    }
}
```

### Why this is better

- preserves correctness under reused host allocations
- opens the door to **zero H2D close upload** when the caller already has device-resident close
- opens the door to **zero log-return launch** when the caller already has device-resident log returns

---

## A.2 Supersedes v1: `MkcsUploadedComboBatch` must be device-bound

The v1 upload-once sketch omitted explicit device affinity. That is dangerous because `DeviceBuffer` ownership is context/device-specific.

### Recommended correction

Bind uploaded combo batches to a device at creation time and reject cross-device use.

```rust
pub struct MkcsUploadedComboBatch {
    pub device_id: u32,
    pub len: usize,
    d_stddev_row_index: DeviceBuffer<i32>,
    d_breakout_row_index: DeviceBuffer<i32>,
    d_start_t: DeviceBuffer<i32>,
    d_end_t: DeviceBuffer<i32>,
}

impl MkcsUploadedComboBatch {
    pub fn from_host_on_device(
        device_id: u32,
        combos: &MkcsGpuComboBatch<'_>,
    ) -> Result<Self, String> {
        let len = combos.stddev_row_index.len();
        if combos.breakout_row_index.len() != len
            || combos.start_t.len() != len
            || combos.end_t.len() != len
        {
            return Err("MKCS GPU combo vector length mismatch".to_string());
        }

        MKCS_GPU_RUNTIME.with(|cell| {
            let mut runtimes = cell.borrow_mut();
            if !runtimes.contains_key(&device_id) {
                runtimes.insert(device_id, MkcsCudaRuntime::new(device_id)?);
            }
            let rt = runtimes
                .get_mut(&device_id)
                .ok_or_else(|| "MKCS GPU runtime missing".to_string())?;
            rt.set_current()?;

            Ok(Self {
                device_id,
                len,
                d_stddev_row_index: DeviceBuffer::from_slice(combos.stddev_row_index)
                    .map_err(|e| e.to_string())?,
                d_breakout_row_index: DeviceBuffer::from_slice(combos.breakout_row_index)
                    .map_err(|e| e.to_string())?,
                d_start_t: DeviceBuffer::from_slice(combos.start_t)
                    .map_err(|e| e.to_string())?,
                d_end_t: DeviceBuffer::from_slice(combos.end_t)
                    .map_err(|e| e.to_string())?,
            })
        })
    }
}
```

Then in the uploaded evaluator:

```rust
if combos.device_id != device_id {
    return Err(format!(
        "MKCS uploaded combo batch belongs to device {}, but eval requested device {}",
        combos.device_id, device_id
    ));
}
```

### Important correction to API design

Do **not** make the current `eval_mkcs_metrics_gpu_kernel_device_rows(...)` convenience path allocate a fresh `MkcsUploadedComboBatch` every call. That would erase the point of the scratch-reuse path.

Instead:

- keep the existing API as the **one-shot / scratch-upload** path
- add a second explicit **uploaded** path for callers that can reuse combos across many evaluations

---

## A.3 Supersedes v1: strengthen signal-matrix validation beyond non-zero shapes

The current file validates only:

- non-empty series
- close length match
- non-null pointers
- non-zero rows/strides
- combo vector length match

That leaves several correctness/perf cliffs uncovered.

### Add these additional checks

```rust
fn validate_inputs(
    device_id: u32,
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("MKCS GPU kernel requires non-empty series".to_string());
    }
    if signals.market.len != n {
        return Err(format!(
            "MKCS GPU market len mismatch: market.len={}, cols={}",
            signals.market.len, n
        ));
    }

    if signals.signal_device_id != device_id {
        return Err(format!(
            "MKCS GPU signal device mismatch: signal_device_id={}, requested_device={}",
            signals.signal_device_id, device_id
        ));
    }

    if let Some(market_device_id) = signals.market.device_id {
        if market_device_id != signals.signal_device_id {
            return Err(format!(
                "MKCS GPU market device mismatch: market_device_id={}, signal_device_id={}",
                market_device_id, signals.signal_device_id
            ));
        }
    }

    if signals.stddev_row_major_ptr == 0
        || signals.upper_row_major_ptr == 0
        || signals.lower_row_major_ptr == 0
    {
        return Err("MKCS GPU signal pointer is null".to_string());
    }

    if signals.stddev_rows == 0 || signals.breakout_rows == 0 {
        return Err("MKCS GPU row count must be > 0".to_string());
    }

    if signals.stddev_stride != n {
        return Err(format!(
            "MKCS GPU stddev stride mismatch: expected {}, got {}",
            n, signals.stddev_stride
        ));
    }
    if signals.breakout_stride != n {
        return Err(format!(
            "MKCS GPU breakout stride mismatch: expected {}, got {}",
            n, signals.breakout_stride
        ));
    }

    let combos_len = combos.stddev_row_index.len();
    if combos.breakout_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("MKCS GPU combo vector length mismatch".to_string());
    }

    for i in 0..combos_len {
        let s_row = combos.stddev_row_index[i];
        let b_row = combos.breakout_row_index[i];
        let start_t = combos.start_t[i];
        let end_t = combos.end_t[i];

        if s_row < 0 || (s_row as usize) >= signals.stddev_rows {
            return Err(format!("MKCS GPU stddev row index out of range at combo {i}: {s_row}"));
        }
        if b_row < 0 || (b_row as usize) >= signals.breakout_rows {
            return Err(format!("MKCS GPU breakout row index out of range at combo {i}: {b_row}"));
        }
        if start_t < 0 || end_t < start_t || (end_t as usize) >= n {
            return Err(format!(
                "MKCS GPU time range invalid at combo {i}: start_t={start_t}, end_t={end_t}, len={n}"
            ));
        }
    }

    Ok(())
}
```

### Why this matters

- catches device-affinity mistakes before kernel launch
- rejects mismatched row-major shapes that would otherwise become invalid reads
- improves route/fallback transparency because these failures become explicit, deterministic host-side errors

---

## B. Additional in-scope improvements I would add

## B.1 Add a **market-series residency path**, not just signal-matrix residency

The current `MkcsGpuDeviceSignalMatrices` already passes device pointers for the signal matrices, but still forces host `close_f32` in the provided file. The biggest missing performance step after scratch reuse is to let the caller provide:

- device close pointer
- optionally device log-return pointer
- device identity

This is separate from sweep optimization and is worth doing even if the device-native sweep API takes longer.

### Implementation direction

- keep the current host-close path as the fallback
- add the `MkcsGpuMarketSeries` shape above
- prefer `close_device_ptr` when available
- prefer `lr_device_ptr` when available
- only upload host close when no device close is supplied

### Expected payoff

- removes one H2D upload per repeated evaluation
- removes one kernel launch per repeated evaluation when `lr_device_ptr` is supplied
- aligns MKCS more closely with the resident-market-series posture mentioned elsewhere in the optimization map

---

## B.2 Add a **GPU-ready output split** in `mkcs.rs`

Right now `compute_indicator_f64(...)` routes CUDA to `HostF32` and then expands to `Vec<f64>`. That is fine for CPU-reference surfaces, but it is the wrong shape when the next consumer is the MKCS GPU kernel.

### Better split

Use two explicit paths:

1. **CPU/reference path**
   - returns `Vec<f64>`
   - used for CPU execution and parity tests

2. **GPU-prep path**
   - returns either `Vec<f32>` or a device matrix / device series handle
   - used only by the MKCS GPU evaluator and benchmark prep paths

### Suggested local helper split

```rust
#[cfg(feature = "cuda")]
fn compute_indicator_cuda_host_f32(
    prepared: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
) -> Result<Vec<f32>, String> {
    let out = compute_dynamic_indicator_cuda_with_inputs(
        prepared,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
    )
    .map_err(|e| e.to_string())?;

    match out.series {
        DynamicIndicatorCudaSeries::HostF32(v) => Ok(v),
        DynamicIndicatorCudaSeries::DeviceF32(_) => {
            Err("MKCS expected CUDA HostF32 output".to_string())
        }
    }
}
```

Then keep `compute_indicator_f64(...)` for CPU/reference work only.

### Why this matters

The current design pays an unnecessary `f32 -> f64` expansion when the next stage is GPU scoring or parity-only benchmarking. This is not the biggest bottleneck, but it is still wasted memory traffic and host work.

---

## B.3 Add a **combined Donchian path** for both CPU and GPU, not just GPU

The original plan already suggested combined Donchian output as an optional GPU-side improvement. I would broaden that slightly:

- if the dynamic indicator adapter can return multiple outputs from a single Donchian invocation, use it for **both** CPU and GPU
- if it cannot today, add that as an adjacent API task

### Why this matters

`compute_mkcs_breakout_signals(...)` currently performs two separate logical fetches using the same params and indicator id. That repeats dispatch and materialization work for one strategy-local concept.

Even if the GPU-native path ends up using device rows directly, the CPU fallback path still benefits from a combined multi-output fetch.

---

## B.4 Add **steady-state vs cold-path** benchmark separation

The benchmark files already do route probes and repeated iterations, which is good. But for MKCS specifically, I would explicitly separate:

- **cold-path**: first call that includes runtime init / module load / first upload
- **steady-state**: repeated call after one warm-up iteration

### Why this matters

After you add scratch reuse, the steady-state numbers will improve more than the cold-path numbers. If you only look at one blended figure, the real benefit can be hidden.

### Suggested Criterion shape

For the new MKCS phase benches:

```rust
// Before the benchmark group:
let _warm_stddev = compute_mkcs_stddev_sweep_cuda_device(&prepared, 5, 80, 1)
    .expect("mkcs warm stddev");
let _warm_upper = compute_mkcs_donchian_output_sweep_cuda_device(
    &prepared,
    &["upperband", "upper", "value"],
    20,
    220,
    1,
)
.expect("mkcs warm upper");
let _warm_lower = compute_mkcs_donchian_output_sweep_cuda_device(
    &prepared,
    &["lowerband", "lower"],
    20,
    220,
    1,
)
.expect("mkcs warm lower");

// For score-only:
let _warm_score = eval_mkcs_metrics_gpu_kernel_device_rows(
    0,
    &signals,
    &combos,
    &StrategyConfig::default(),
    &mkcs_cfg,
)
.expect("mkcs warm score");
```

Also set an explicit warm-up time:

```rust
group.warm_up_time(Duration::from_secs(2));
```

And if you add an uploaded-combo benchmark, benchmark both:

- `score_host_combos_reused_runtime_scratch`
- `score_uploaded_combos`

That makes it obvious how much H2D combo traffic still matters.

---

## B.5 Expand MKCS validation candidates to vary **MKCS-local** parameters

This is one of the biggest benchmark-fidelity gaps I missed in the first draft.

In `composite_validation_workflow.rs`, the current `make_mkcs_candidates(...)` varies only:

- `fast_len`
- `slow_len`

while `mkcs` itself stays at `MkcsStrategyParams::default()`.

That means the validation bench is not exercising:

- compression threshold variation
- re-expansion threshold variation
- `map_fast_to_stddev_period`
- `map_slow_to_breakout_lookback`
- direct `stddev_period` / `breakout_lookback` variation when mappings are disabled

### Better MKCS candidate generator

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_mkcs_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);

        let map_fast_to_stddev_period = i % 4 != 0;
        let map_slow_to_breakout_lookback = i % 5 != 0;

        let compression_threshold = match i % 4 {
            0 => 0.008,
            1 => 0.010,
            2 => 0.012,
            _ => 0.015,
        };
        let reexpansion_threshold = match i % 4 {
            0 => 0.020,
            1 => 0.030,
            2 => 0.040,
            _ => 0.050,
        };

        let mkcs = ta_app_core::MkcsStrategyParams {
            map_fast_to_stddev_period,
            map_slow_to_breakout_lookback,
            stddev_period: 12u16.saturating_add(i_u16 % 16),
            breakout_lookback: 18u16.saturating_add((i_u16 * 3) % 24),
            compression_threshold,
            reexpansion_threshold,
        };

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
            mkcs: Some(mkcs),
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

### Why this matters

- better parity coverage around fp32 thresholds
- more realistic MKCS-specific validation throughput measurement
- makes the validation benches more representative of actual MKCS workloads

---

## B.6 Add a **CPU-vs-GPU route threshold** only after measurement

The review map explicitly says the strategy-local review should consider whether the strategy should remain CPU-preferred on current measured workloads. If the new MKCS benches show:

- GPU wins only above a certain `(bars * candidates)` or `(bars * pair_count)` threshold
- CPU wins below that threshold because staging dominates

then it is worth encoding a route threshold rather than blindly preferring GPU whenever CUDA is available.

### What to add after the new benchmarks exist

A benchmark-backed threshold function in the route policy layer (adjacent files, not in the uploaded set), for example:

```rust
fn mkcs_should_prefer_gpu(
    bars: usize,
    candidate_count: usize,
    prepared_cuda_inputs_present: bool,
) -> bool {
    if !prepared_cuda_inputs_present {
        return false;
    }

    let work = bars.saturating_mul(candidate_count);
    work >= 2_000_000
}
```

**Do not hardcode the threshold above as-is.** Use the benchmark surfaces you add first.

---

## B.7 Cache launch-shape / device-cap decisions once per runtime

This is a smaller win, but it is real and safe.

The current `mkcs_gpu_kernel.rs` launch path repeatedly does:

- `Device::get_device(device_id)`
- `detect_device_launch_caps(...)`
- `module.get_function(...)`
- `detect_kernel_launch_caps(...)`
- `choose_launch_shape_1d_from_candidates(...)`

for both:

- `compute_log_returns_f32`
- `mkcs_backtest_metrics_f32`

on each evaluation.

### Safer minimal improvement

- cache device launch caps in `MkcsCudaRuntime`
- cache chosen block sizes after first evaluation for common kernels
- recompute only if there is a genuine reason to do so

Because storing borrowed `Function` handles inside the same struct as `Module` can be awkward depending on the `cust` version, I would not force a self-referential design. Caching the **device caps and selected block sizes** still removes some repeated host overhead without tricky ownership work.

---

## C. Additional correctness / parity tests I would add

## C.1 Regression test for incorrect close-cache reuse

This directly guards against the v1 mistake.

```rust
#[cfg(test)]
#[test]
fn mkcs_market_series_key_must_change_when_close_values_change() {
    // Build two host close vectors with the same allocation length and, if practical,
    // the same backing Vec reused across mutations. The test should assert that
    // reusing the same market_series_key would be invalid and that the caller must
    // provide a new key whenever content changes.
    //
    // This is primarily a host-side invariant test for the caching contract.
}
```

## C.2 Uploaded-combo wrong-device rejection test

```rust
#[cfg(test)]
#[test]
fn mkcs_uploaded_combo_batch_rejects_cross_device_eval() {
    // Upload combos on device 0, then attempt eval on device 1.
    // Assert explicit device mismatch error.
}
```

## C.3 Same-shape different-threshold parity tests

Because the benchmark candidate generator should start varying thresholds, add parity tests with:

- same candles
- same MA lengths
- different threshold pairs
- CPU vs GPU metrics comparison within tolerance

Use relative + absolute tolerance, not only a fixed absolute tolerance.

```rust
fn approx_eq_f64(cpu: f64, gpu: f64, abs_tol: f64, rel_tol: f64) -> bool {
    let diff = (cpu - gpu).abs();
    diff <= abs_tol.max(rel_tol * cpu.abs().max(gpu.abs()))
}
```

---

## D. Optional changes beyond the strict prompt scope

Everything below goes beyond the narrow “Section 5 / strategy-local review” prompt, but still fits your goal: improve performance and correctness without removing functionality.

These are **not** the first changes I would implement, but they are worth considering once the in-scope fixes are done.

## D.1 Device-side **objective-only / score-only** readback path

Today the MKCS GPU launcher reads back full metrics rows and decodes them to `Vec<Metrics>` for every combo.

If the optimization path only needs:

- objective score
- maybe trade count / guard metric
- maybe a few tie-breakers

then a separate score-only GPU path can reduce:

- device write volume
- D2H copy volume
- host decode cost

### Example shape

```rust
pub struct MkcsGpuObjectiveRow {
    pub objective: f32,
    pub net_profit: f32,
    pub trades: i32,
}

pub fn eval_mkcs_objective_gpu_kernel_device_rows(
    device_id: u32,
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    mkcs: &MkcsStrategyParams,
) -> Result<Vec<MkcsGpuObjectiveRow>, String> {
    // Separate kernel or separate output mode.
}
```

### Why it is worth considering

For large candidate sets, readback and decode can become a material share of total end-to-end time. Keeping the full-metrics path for compatibility while adding a score-only fast path preserves features and reduces hot-path cost.

**Risk:** medium-high, because it touches optimizer integration and any caller assumptions about full metric availability.

---

## D.2 MKCS-local **surface cache**

If the same candles and the same period ranges are used repeatedly, you can cache:

- stddev sweep device matrix
- Donchian upper sweep device matrix
- Donchian lower sweep device matrix

keyed by something like:

- `market_series_key`
- `device_id`
- indicator id
- output id
- `(period_start, period_end, period_step)`

### Why this is valuable

This can remove repeated sweep prep entirely across:

- repeated optimizer calls on the same data
- repeated validation probes
- route probes that reuse the same candle series

### Why it is beyond scope

This starts to overlap the broader residency/cache bundle from the review map rather than staying purely inside MKCS-local code.

---

## D.3 Pinned host buffers + async copies

If the `cust` / runtime stack in your repo supports pinned host buffers cleanly, you can reduce some H2D / D2H overhead for the remaining copy boundaries by using pinned host memory for:

- `close_f32` upload when host-upload is still used
- metrics readback buffer

and moving to `async_copy_to` / `async_copy_from` on the same stream where safe.

### Why this can help

Page-locked host memory often improves transfer bandwidth and makes async copy behavior meaningful.

### Why it is not first priority

The larger wins are still:

- eliminate copies entirely when possible
- reuse device buffers
- avoid host round-trips in the first place

Pinned memory helps the **remaining** unavoidable copy boundaries.

---

## D.4 Multi-stream overlap for independent prep work

MKCS prep has three logically independent surfaces:

- stddev sweep
- Donchian upper sweep
- Donchian lower sweep

If the underlying CUDA prep APIs can operate on separate streams safely, you may be able to overlap some of that work.

### Why this is optional

- it depends on adjacent runtime / adapter design
- it adds synchronization complexity
- it only helps once the data is already resident and the prep APIs are stream-friendly

---

## D.5 CUDA Graphs for repeated same-shape scoring

If you end up with many repeated invocations of the same shape:

- same combo count
- same signal shapes
- same kernel sequence

then CUDA Graph capture/replay can reduce launch overhead.

### Why this is optional

- advanced
- dependent on driver / crate support
- not necessary until the larger staging issues are fixed

---

## D.6 MKCS-specific prep kernel

A more aggressive strategy-local extension would be a dedicated MKCS prep kernel that directly produces:

- stddev sweep
- upper Donchian sweep
- lower Donchian sweep

from device-resident candles, bypassing generic dynamic indicator dispatch overhead.

### Why this is attractive

- removes generic-dispatch overhead from the hottest MKCS prep path
- lets you control output layout exactly for the scoring kernel
- can avoid intermediate host materialization entirely

### Why this is late-phase work

- larger implementation surface
- higher parity risk
- adjacent to kernel-source work that is not in the provided file set

---

## E. Additional benchmark/process guidance

## E.1 Add throughput-derived reporting, not just elapsed time

For the new MKCS benches, record or print:

- bars/sec
- candidates/sec
- `(bars * candidates)/sec`
- if applicable, pair-bars/sec

That makes it easier to compare:

- score-only improvements
- full-route improvements
- CPU-preferred vs GPU-preferred crossover

## E.2 Keep one **steady-state** score-only bench that reuses everything possible

The most informative post-change benchmark is often:

- same signals
- same combos
- same runtime
- same device
- repeated score-only eval

That tells you whether the GPU kernel path itself is now healthy after staging work is removed.

## E.3 If staging is still dominant after Phase 3, use Nsight next, not more guesswork

At that point:

- use Nsight Systems to confirm copy/launch overlap and stream behavior
- only then inspect Nsight Compute or kernel internals if the kernel itself still looks expensive

This keeps the evidence boundary aligned with the review map.

---

## F. Additional checklist items

Add these to the bottom checklist from the original document.

## API / correctness corrections

- [ ] Replace any `close.as_ptr() + len` reuse key with an explicit `market_series_key`
- [ ] Extend `MkcsGpuDeviceSignalMatrices` with market-series identity and optional device-resident close/lr fields
- [ ] Validate `signal_device_id` against requested `device_id`
- [ ] Validate `market.device_id` against `signal_device_id` when device pointers are supplied
- [ ] Validate `stddev_stride == cols`
- [ ] Validate `breakout_stride == cols`
- [ ] Ensure uploaded combo batches are created on a specific device
- [ ] Ensure uploaded combo batches reject cross-device use
- [ ] Keep the current one-shot eval path separate from the uploaded-combo eval path

## Additional benchmark fidelity

- [ ] Add one warm-up iteration before each steady-state MKCS prep/score benchmark
- [ ] Add `warm_up_time(...)` to new MKCS Criterion groups
- [ ] Add separate score-only benchmark using uploaded combos
- [ ] Add throughput-derived reporting (`bars/sec`, `candidates/sec`, `bars*candidates/sec`)
- [ ] Expand `make_mkcs_candidates(...)` to vary MKCS-local params, not only MA lengths
- [ ] After collecting data, determine whether MKCS should remain CPU-preferred below a measured work-size threshold

## Additional parity / regression tests

- [ ] Add a regression test for stale close reuse when backing storage is reused
- [ ] Add a wrong-device uploaded-combo rejection test
- [ ] Add relative+absolute tolerance helpers for CPU/GPU parity checks
- [ ] Add same-shape different-threshold CPU/GPU parity tests
- [ ] Add a benchmark-route test or assertion once a CPU/GPU work-size threshold is introduced

## Optional later-phase work

- [ ] Evaluate score-only / objective-only readback path
- [ ] Evaluate MKCS-local device surface cache
- [ ] Evaluate pinned host buffers for remaining H2D / D2H copies
- [ ] Evaluate multi-stream overlap for stddev + Donchian prep
- [ ] Evaluate CUDA Graph replay for repeated same-shape scoring
- [ ] Evaluate a dedicated MKCS prep kernel only after staging bottlenecks are measured down

---

## Updated priority order after this addendum

If I were implementing from scratch after the second review pass, I would use this order:

1. **Benchmark coverage first**
   - add MKCS optimizer benches
   - add MKCS validation heavy groups
   - add MKCS-local candidate variation
   - add warm/steady-state split

2. **Correct and strengthen the runtime-reuse plan**
   - move from pointer-based reuse to `market_series_key`
   - add signal/market device-affinity validation
   - add scratch reuse for `d_close`, `d_lr`, combos, `d_out`, and host readback

3. **Add explicit uploaded-combo support**
   - device-bound upload-once API
   - keep one-shot path separate

4. **Reduce prep waste in `mkcs.rs`**
   - cached IDs
   - param reuse
   - CPU loop tightening
   - begin separating GPU-ready outputs from CPU-reference outputs

5. **Then attack the big prep boundary**
   - device-native sweep path
   - combined Donchian fetch
   - market-series residency path

6. **Only after the above**
   - route thresholds
   - score-only readback
   - pinned memory / async copies
   - Nsight-guided deeper kernel work

## Final addendum takeaway

The first plan already captured the biggest obvious staging costs, but after re-review I would make three important adjustments:

- treat **identity / residency correctness** as part of the optimization plan, not an afterthought
- make the benchmark story more explicitly **steady-state and MKCS-specific**
- keep the optional higher-ceiling work in reserve, especially **device-side score-only readback** and **surface caching**, because those are the changes most likely to matter after the easy copy/allocation wins are gone
