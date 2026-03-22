# ASR strategy — section 5 GPU-native optimization and validation plan

## Scope

This document is a **section-5, one-strategy-only** review and implementation plan for the **ASR** strategy, using the review-map’s “Strategy-local GPU-native optimization and validation routes” bundle and incorporating the allocation/copy-elision concerns that are directly implicated in the current ASR CUDA route.

Covered files in this plan:

- `asr.rs`
- `asr_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

Review-map asks covered here:

- **strategy-local hot-path review**
- **CPU/GPU payoff review**
- **staging/fallback review**
- **per-strategy benchmark review**

Additional allocation/copy-elision asks covered here:

- safe `MaybeUninit` adoption where every element is guaranteed to be overwritten before any read
- host-readback copy elision
- avoidable-versus-unavoidable copy-boundary review
- limiting unnecessary host->device and device->device copies
- reusing buffers instead of allocating per call

---

## What this ASR bundle owns

### `asr.rs`

Strategy-local signal generation, period-sweep routing, device-row ownership, and CPU-reference parity surfaces.

Key surfaces:

- `AsrPeriodSweepDevice`
- `compute_indicator_f64`
- `compute_asr_fast_signals_cuda_device`
- `compute_asr_slow_signals_cuda_device`
- `compute_asr_kaufman_stops_cuda_device`
- `compute_asr_ttm_squeeze_sweep_cuda_device`
- `compute_asr_squeeze_momentum_sweep_cuda_device`
- `compute_asr_linearreg_angle_sweep_cuda_device`
- `compute_asr_er_sweep_cuda_device`
- `compute_asr_zscore_sweep_cuda_device`
- `build_asr_desired_positions`
- `simulate_from_desired`

### `asr_gpu_kernel.rs`

Strategy-local CUDA scoring entrypoints and CUDA runtime reuse surface.

Key surfaces:

- `AsrCudaRuntime`
- `compute_log_returns_f32`
- `launch_pack_rows_u64_ptr_f32`
- `eval_asr_metrics_gpu_kernel`
- `eval_asr_metrics_gpu_kernel_device_rows`

### `gpu_native_expansion_opt.rs`

Strategy-local optimizer benchmark surface. In the current provided file, ASR is **not covered at all**.

### `composite_validation_workflow.rs`

Strategy-local validation benchmark surface. In the current provided file, ASR **is covered**, but the GPU route is not enforced strongly enough.

---

## Current ASR GPU route summary

ASR already has a legitimate GPU-native strategy surface:

- `asr.rs` can build **device-native period sweeps**
- `AsrPeriodSweepDevice` can expose **per-period row pointers**
- `asr_gpu_kernel.rs` can score ASR combos through a dedicated `asr_backtest_metrics_f32` kernel

That is good architecture for a strategy-local GPU route.

The current performance issue is not “ASR has no CUDA path.” The issue is that the current CUDA path still pays too much **staging cost** before the scoring kernel actually runs.

---

## Confirmed issues, likely issues, and open questions

## Confirmed issues

### C1. Repeated market-series rebuild and upload in both ASR CUDA entrypoints  
**Timing boundary:** `app-path`, `validation-cost harness`  
**File/function:** `asr_gpu_kernel.rs::eval_asr_metrics_gpu_kernel`, `asr_gpu_kernel.rs::eval_asr_metrics_gpu_kernel_device_rows`

Both entrypoints do this every call:

- allocate `close_f32`
- compute `lr_f32`
- upload `close_f32`
- upload `lr_f32`

That is repeated work for warm optimization/validation runs against the same candle set.

This is a confirmed optimization target.

---

### C2. The “device rows” path is not actually row-pointer-native end to end  
**Timing boundary:** `app-path`, `validation-cost harness`  
**File/function:** `asr_gpu_kernel.rs::eval_asr_metrics_gpu_kernel_device_rows`

The current device-row entrypoint:

- uploads row-pointer arrays
- allocates five dense matrix scratch buffers
- launches `launch_pack_rows_u64_ptr_f32` five times
- only then launches the ASR scoring kernel

So the intended advantage of `AsrPeriodSweepDevice::row_ptr_for_period(...)` is mostly lost at the final scoring boundary.

This is the biggest confirmed staging loss in the ASR CUDA route.

---

### C3. Host output readback is zero-initialized even though the device copy fully overwrites it  
**Timing boundary:** `app-path`  
**File/function:** `asr_gpu_kernel.rs::eval_asr_metrics_gpu_kernel`, `asr_gpu_kernel.rs::eval_asr_metrics_gpu_kernel_device_rows`

Both entrypoints currently do:

```rust
let mut host_out = vec![0.0f32; out_len];
d_out.copy_to(&mut host_out)?;
```

The zero fill is unnecessary if `copy_to(...)` fully initializes the destination.

This is a safe `MaybeUninit` candidate.

---

### C4. GPU route capability is narrower than the CPU reference path  
**Timing boundary:** `route matrix`, `app-path`  
**File/function:** `asr_gpu_kernel.rs::{eval_asr_metrics_gpu_kernel, eval_asr_metrics_gpu_kernel_device_rows}`, `asr.rs::simulate_from_desired`

GPU scoring rejects:

- execution overlays
- stop-loss
- take-profit

The CPU reference path supports those behaviors.

This is not a correctness bug by itself. It is a **route boundary** that must stay explicit in callsites and benchmarks.

---

### C5. `compute_asr_kaufman_stops_cuda_device` repeats wrapper/setup work  
**Timing boundary:** `app-path`, `validation-cost harness`  
**File/function:** `asr.rs::compute_asr_kaufman_stops_cuda_device`

The function constructs a `CudaKaufmanstop` wrapper inside a closure and invokes that closure twice, once for `"long"` and once for `"short"`, while also duplicating parameter vector construction.

This is not the largest ASR bottleneck, but it is real avoidable warm-path work.

---

### C6. Non-default z-score settings leave the direct native sweep path  
**Timing boundary:** `route matrix`, `app-path`  
**File/function:** `asr.rs::compute_asr_zscore_sweep_cuda_device`

Only `z_devtype == 0 && z_ma_type == "sma"` stays on the current direct/native z-score sweep path.

Other settings route through the generic period-sweep CUDA dispatch.

That is a valid current constraint, but it is a strategy-local payoff cliff that should be visible in benchmarks and route expectations.

---

### C7. ASR optimizer throughput is not benchmarked in the provided Criterion surface  
**Timing boundary:** `Criterion`  
**File/function:** `gpu_native_expansion_opt.rs`

`base_req(...)` sets `asr: None`, and only other strategies are registered in the benchmark group.

This means the provided optimizer benchmark file cannot currently serve as evidence for ASR GPU-native optimizer payoff.

---

### C8. ASR validation GPU benches do not fail on silent fallback  
**Timing boundary:** `validation-cost harness`  
**File/function:** `composite_validation_workflow.rs::bench_composite_validation_gpu_workflow`

The ASR GPU validation workflow prints route metadata, but does not require:

- `backend_used == GPU`
- `backend_fallback_reason == None`

So a benchmark labeled `gpu_kernel/asr/...` can still silently measure a fallback route.

---

## Likely issues / lower-confidence issues

### L1. Wrapper construction cost for direct CUDA indicator wrappers may be non-trivial  
**Timing boundary:** `app-path`  
**Files/functions:** ASR direct sweep wrappers in `asr.rs`

`CudaTtmSqueeze::new(...)`, `CudaSqueezeMomentum::new(...)`, `CudaLinearregAngle::new(...)`, `CudaEr::new(...)`, `CudaZscore::new(...)`, and `CudaKaufmanstop::new(...)` may or may not be cheap.

This is worth profiling before introducing persistent wrapper caching. It is lower priority than staging-copy fixes.

---

### L2. Request cloning inside benchmark closures may slightly contaminate measured cost  
**Timing boundary:** `Criterion`, `validation-cost harness`  
**Files:** `gpu_native_expansion_opt.rs`, `composite_validation_workflow.rs`

Both benchmark files clone sizable request structures inside the measured closure.

This is probably not the dominant cost for large workloads, but it is still timing-boundary noise.

I would treat it as a secondary cleanup, not a first-wave optimization.

---

## Open questions to answer with measurement

### O1. How much of the current device-row path cost is from full-matrix repacking versus kernel scoring?
Use before/after timing around:

- row-pointer upload
- pack kernel launches
- scoring kernel launch
- D2H decode/readback

### O2. Does the direct z-score wrapper already support more `ma_type` / `devtype` combinations than the current gate allows?
The direct z-score function already accepts the config values. The current gate may be conservative rather than fundamental.

This should be validated with parity tests before expanding support.

### O3. Is wrapper construction cost worth caching?
Only measure this after fixing market-series reuse and row-compaction. Those are much more likely to dominate.

---

# Strategy-local hot-path review

## 1. `asr_gpu_kernel.rs` hot path

This is the main hot-path file for ASR GPU scoring.

### Current dense path cost shape

`eval_asr_metrics_gpu_kernel` currently does all of the following per call:

- allocates `close_f32`
- allocates `lr_f32`
- uploads both
- uploads five indicator matrices
- uploads two Kaufman-stop vectors
- uploads four combo vectors
- allocates `d_out`
- synchronizes
- allocates a zero-filled host readback vector
- copies device output back to host
- decodes host rows into `Metrics`

This is correct, but it is allocation-heavy and copy-heavy.

### Current device-row path cost shape

`eval_asr_metrics_gpu_kernel_device_rows` currently does all of the following per call:

- allocates `close_f32`
- allocates `lr_f32`
- uploads both
- uploads five row-pointer arrays
- allocates five dense packed matrices
- launches five pack kernels
- uploads four combo vectors
- allocates `d_out`
- synchronizes
- allocates a zero-filled host readback vector
- copies output back to host

This means the “device rows” path still does dense materialization every call.

### Memory traffic intuition

Let:

- `N` = number of bars
- `F` = fast sweep rows
- `S` = slow sweep rows
- `C` = combo count

Then the current **dense path** stages approximately:

- host->device market series: `2 * N * sizeof(f32)`
- host->device indicator matrices: `(2F + 3S + 2) * N * sizeof(f32)`
  - `+2` is Kaufman long + short
- host->device combo vectors: `4 * C * sizeof(i32)`
- device->host output: `C * stride * sizeof(f32)`

The current **device-row path** adds another expensive layer:

- host->device row-pointer arrays
- device->device dense pack writes for `2F + 3S` rows
- device->device reads during those pack kernels
- then scoring

That is why the device-row path is currently under-delivering on its promise.

---

## 2. `asr.rs` hot path

### What is already good

- `AsrPeriodSweepDevice` is exactly the kind of strategy-local abstraction you want.
- The direct device sweep functions are a strong foundation.
- `row_ptr_for_period(...)` is already present, which means the strategy side has the right shape for pointer-native GPU scoring.

### Current hot-path inefficiencies or route cliffs

1. `compute_asr_kaufman_stops_cuda_device` repeats wrapper construction and parameter setup.
2. `compute_indicator_f64` will try CUDA when descriptors prefer it, but silently falls back to CPU if CUDA computation fails.
3. `compute_asr_zscore_sweep_cuda_device` leaves the current direct path for non-default z-score settings.

These are smaller than the `asr_gpu_kernel.rs` staging costs, but they matter for route transparency and measured payoff.

---

# CPU/GPU payoff review

## Cases where ASR should remain GPU-preferred

ASR should remain **GPU-preferred** for workloads that have all of the following:

- execution does **not** require overlays
- execution does **not** require stop-loss / take-profit
- candidate count is large enough for kernel scoring to amortize setup
- candles are reused across iterations or validation windows
- ASR sweeps are reused enough that device staging reuse matters

This is especially true for:

- optimizer throughput on large MA pair grids
- validation runs with many candidates
- repeated warm-path reruns on the same candle set

## Cases where ASR should remain CPU-preferred for now

ASR should remain **CPU-preferred** when the requested route requires:

- execution overlays
- stop-loss
- take-profit

Those are current feature blockers for the ASR GPU scoring kernel and are explicitly supported by the CPU reference route.

Do **not** paper over this with soft labeling. Treat it as an explicit route capability boundary.

## Intermediate case: GPU-eligible but less efficient than ideal

Some ASR runs are still GPU-eligible but may underperform the best route because of staging waste:

- non-default z-score settings that leave the direct/native z-score path
- device-row runs where most time is spent repacking full dense matrices
- short or small combo batches where the current staging cost dominates

That does **not** mean “ASR should move back to CPU.” It means the current GPU route needs tighter staging and more honest benchmarking.

---

# Staging/fallback review

## Staging waste in `asr_gpu_kernel.rs`

### Avoidable copies / allocations

1. `close_f64 -> close_f32` allocation every call
2. log-return allocation every call
3. `DeviceBuffer::from_slice(...)` for `d_close` every call
4. `DeviceBuffer::from_slice(...)` for `d_lr` every call
5. `DeviceBuffer::from_slice(...)` for combos every call
6. `DeviceBuffer::<f32>::uninitialized(out_len)` every call
7. zero-filled `Vec<f32>` for host output every call
8. five full dense pack buffers in the device-row path every call
9. five full pack launches in the device-row path every call

### Unavoidable boundaries

Some copies are still real boundaries and should not be “optimized away” in a misleading way:

- device output must eventually be read back to decode host-side `Metrics`
- if the scoring kernel ABI remains dense-row-major, a dense representation must exist somewhere before launch
- host-side benchmark request construction remains separate from kernel staging

The goal is to remove the avoidable copies, not to pretend all boundaries disappear.

## Fallback and route cliffs in `asr.rs`

1. CPU fallback inside `compute_indicator_f64` can erase expected CUDA payoff if it happens unexpectedly.
2. Non-default z-score settings currently move off the direct/native path.
3. Overlay / stop / take-profit configs must remain CPU-only until parity exists.

---

# Per-strategy benchmark review

## `gpu_native_expansion_opt.rs`

Current state:

- benchmarks KQSR, CARM, KVDF, CDMC
- does **not** benchmark ASR

Implication:

- this file cannot currently support any claim about ASR optimizer payoff

Additional nuance:

- GPU route probe happens before the benchmark loop, so the GPU path is effectively warmed before measurement
- the CPU-side cases do not have a matching explicit warm-up

That is not necessarily wrong, but it means the measured number is closer to **steady-state warm GPU path** than to cold-start path.

## `composite_validation_workflow.rs`

Current state:

- ASR validation groups exist
- route metadata is printed
- route correctness is not enforced strongly enough
- ASR candidates are all built from `AsrStrategyParams::default()`

Implications:

- benchmark labels can overstate what route actually ran
- the current ASR validation benchmarks do not cover the direct-zscore vs generic-zscore split
- the benchmark surface also does not exercise GPU-blocked ASR configs as explicit negative cases

---

# Comprehensive file-by-file code edit plan

---

## File 1: `asr_gpu_kernel.rs`

## P0-A. Add a reusable runtime cache for market series, combos, output buffers, and pack scratch

### Why

This removes repeated allocation and upload churn from warm ASR GPU runs.

### Core design

Extend `AsrCudaRuntime` so it owns reusable host/device scratch instead of only owning `context`, `module`, and `stream`.

### Suggested struct expansion

```rust
use cust::memory::{CopyDestination, DeviceBuffer, DeviceCopy};
use std::mem::{ManuallyDrop, MaybeUninit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SeriesCacheKey {
    ptr: usize,
    len: usize,
    first_bits: u64,
    mid_bits: u64,
    last_bits: u64,
}

impl SeriesCacheKey {
    fn from_close_f64(values: &[f64]) -> Self {
        let len = values.len();
        Self {
            ptr: values.as_ptr() as usize,
            len,
            first_bits: values.first().copied().unwrap_or(0.0).to_bits(),
            mid_bits: values.get(len / 2).copied().unwrap_or(0.0).to_bits(),
            last_bits: values.last().copied().unwrap_or(0.0).to_bits(),
        }
    }
}

struct AsrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,

    market_key: Option<SeriesCacheKey>,
    close_host_scratch: Vec<f32>,
    lr_host_scratch: Vec<f32>,

    d_close: Option<DeviceBuffer<f32>>,
    d_lr: Option<DeviceBuffer<f32>>,

    d_kaufman_long: Option<DeviceBuffer<f32>>,
    d_kaufman_short: Option<DeviceBuffer<f32>>,

    d_combo_fast: Option<DeviceBuffer<i32>>,
    d_combo_slow: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,

    d_fast_sq_rows: Option<DeviceBuffer<u64>>,
    d_fast_mom_rows: Option<DeviceBuffer<u64>>,
    d_slow_ang_rows: Option<DeviceBuffer<u64>>,
    d_slow_er_rows: Option<DeviceBuffer<u64>>,
    d_slow_z_rows: Option<DeviceBuffer<u64>>,

    d_fast_sq_pack: Option<DeviceBuffer<f32>>,
    d_fast_mom_pack: Option<DeviceBuffer<f32>>,
    d_slow_ang_pack: Option<DeviceBuffer<f32>>,
    d_slow_er_pack: Option<DeviceBuffer<f32>>,
    d_slow_z_pack: Option<DeviceBuffer<f32>>,

    host_fast_sq_scratch: Vec<f32>,
    host_fast_mom_scratch: Vec<f32>,
    host_slow_ang_scratch: Vec<f32>,
    host_slow_er_scratch: Vec<f32>,
    host_slow_z_scratch: Vec<f32>,
}
```

### Helper functions to add

```rust
fn upload_slice_reuse<T: DeviceCopy>(
    slot: &mut Option<DeviceBuffer<T>>,
    src: &[T],
) -> Result<(), String> {
    match slot {
        Some(buf) if buf.len() == src.len() => {
            buf.copy_from(src).map_err(|e| e.to_string())?;
        }
        _ => {
            *slot = Some(DeviceBuffer::from_slice(src).map_err(|e| e.to_string())?);
        }
    }
    Ok(())
}

fn ensure_uninit_device_len<T: DeviceCopy>(
    slot: &mut Option<DeviceBuffer<T>>,
    len: usize,
) -> Result<&mut DeviceBuffer<T>, String> {
    let needs_alloc = slot.as_ref().map_or(true, |buf| buf.len() != len);
    if needs_alloc {
        *slot = Some(
            unsafe { DeviceBuffer::<T>::uninitialized(len) }
                .map_err(|e| e.to_string())?,
        );
    }
    Ok(slot.as_mut().expect("slot initialized"))
}

fn fill_close_and_log_returns_f32(
    close_f64: &[f64],
    close_out: &mut [f32],
    lr_out: &mut [f32],
) {
    debug_assert_eq!(close_f64.len(), close_out.len());
    debug_assert_eq!(close_f64.len(), lr_out.len());

    if close_f64.is_empty() {
        return;
    }

    close_out[0] = close_f64[0] as f32;
    lr_out[0] = 0.0;

    for t in 1..close_f64.len() {
        let p = close_f64[t];
        let pm = close_f64[t - 1];

        close_out[t] = p as f32;
        lr_out[t] = if p.is_finite() && pm.is_finite() && p > 0.0 && pm > 0.0 {
            (p.ln() - pm.ln()) as f32
        } else {
            0.0
        };
    }
}

impl AsrCudaRuntime {
    fn ensure_market_series(&mut self, close_f64: &[f64]) -> Result<(), String> {
        let key = SeriesCacheKey::from_close_f64(close_f64);
        if self.market_key == Some(key) {
            return Ok(());
        }

        self.close_host_scratch.resize(close_f64.len(), 0.0);
        self.lr_host_scratch.resize(close_f64.len(), 0.0);
        fill_close_and_log_returns_f32(
            close_f64,
            &mut self.close_host_scratch,
            &mut self.lr_host_scratch,
        );

        upload_slice_reuse(&mut self.d_close, &self.close_host_scratch)?;
        upload_slice_reuse(&mut self.d_lr, &self.lr_host_scratch)?;
        self.market_key = Some(key);
        Ok(())
    }
}
```

### Why this is good

- combines `close_f32` conversion and log-return generation into a single host pass
- removes repeated `Vec` allocation for both series
- removes repeated `DeviceBuffer::from_slice(...)` allocation/upload when the candle set is unchanged
- sets up the runtime for later scratch reuse everywhere else

### Important safety note

Pointer-based caching can become stale if a caller mutates the same allocation in place and leaves the first/middle/last sentinels unchanged. If that is realistic in your caller graph, plumb an explicit `market_series_key` from the callsite instead of relying only on slice identity + sentinels.

---

## P0-B. Replace zero-filled host output readback with safe uninitialized readback

### Why

`copy_to(...)` fully initializes the destination. Zero-filling first is wasted work.

### Code snippet

```rust
fn copy_device_to_vec_uninit<T: DeviceCopy>(
    src: &DeviceBuffer<T>,
    len: usize,
) -> Result<Vec<T>, String> {
    let mut raw: Vec<MaybeUninit<T>> = Vec::with_capacity(len);
    unsafe { raw.set_len(len) };

    let dst: &mut [T] = unsafe {
        std::slice::from_raw_parts_mut(raw.as_mut_ptr() as *mut T, len)
    };

    src.copy_to(dst).map_err(|e| e.to_string())?;

    let mut raw = ManuallyDrop::new(raw);
    let ptr = raw.as_mut_ptr() as *mut T;
    let len = raw.len();
    let cap = raw.capacity();

    Ok(unsafe { Vec::from_raw_parts(ptr, len, cap) })
}
```

### Replace this

```rust
let mut host_out = vec![0.0f32; out_len];
d_out.copy_to(&mut host_out).map_err(|e| e.to_string())?;
```

### With this

```rust
let host_out = copy_device_to_vec_uninit(&d_out, out_len)?;
```

### If you already have an `alloc_policy` helper

If the repo already exposes a shared helper for this pattern, prefer the shared helper instead of duplicating the unsafe locally.

---

## P0-C. Centralize ASR GPU route-blocker logic instead of repeating it inline

### Why

Right now the GPU blocker logic lives inline in both ASR kernel entrypoints. It should also be reusable from benchmarks and route tests.

### Add to `asr.rs`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsrGpuKernelBlocker {
    ExecutionOverlays,
    StopLoss,
    TakeProfit,
}

pub fn asr_gpu_kernel_blocker(strategy: &StrategyConfig) -> Option<AsrGpuKernelBlocker> {
    if strategy.uses_execution_overlays() {
        Some(AsrGpuKernelBlocker::ExecutionOverlays)
    } else if strategy.stop_loss_pct().is_some() {
        Some(AsrGpuKernelBlocker::StopLoss)
    } else if strategy.take_profit_pct().is_some() {
        Some(AsrGpuKernelBlocker::TakeProfit)
    } else {
        None
    }
}

pub fn asr_uses_direct_zscore_cuda_fast_path(cfg: &AsrStrategyParams) -> bool {
    cfg.z_devtype == 0 && cfg.z_ma_type.eq_ignore_ascii_case("sma")
}
```

### Then use it in `asr_gpu_kernel.rs`

```rust
if let Some(blocker) = crate::asr::asr_gpu_kernel_blocker(strategy) {
    return Err(format!(
        "ASR GPU kernel route blocked by strategy config: {:?}",
        blocker
    ));
}
```

### Then use it in benchmarks

- to assert that “GPU ASR” benchmark cases are actually GPU-eligible
- to create explicit negative/fallback cases if desired

---

## P0-D. Actually use the reusable runtime buffers in both entrypoints

### Dense path integration sketch

Inside `eval_asr_metrics_gpu_kernel(...)`, replace the current fresh-allocation flow with:

```rust
rt.set_current()?;
rt.ensure_market_series(signals.close_f64)?;

upload_slice_reuse(&mut rt.d_combo_fast, combos.fast_row_index)?;
upload_slice_reuse(&mut rt.d_combo_slow, combos.slow_row_index)?;
upload_slice_reuse(&mut rt.d_combo_start, combos.start_t)?;
upload_slice_reuse(&mut rt.d_combo_end, combos.end_t)?;

let d_out = ensure_uninit_device_len(&mut rt.d_out, out_len)?;
```

For the dense indicator matrices and Kaufman-stop vectors, do the same pattern instead of fresh `DeviceBuffer::from_slice(...)` every call:

```rust
upload_slice_reuse(&mut rt.d_fast_sq_pack, signals.fast_squeeze_row_major)?;
upload_slice_reuse(&mut rt.d_fast_mom_pack, signals.fast_momentum_row_major)?;
upload_slice_reuse(&mut rt.d_slow_ang_pack, signals.slow_angle_row_major)?;
upload_slice_reuse(&mut rt.d_slow_er_pack, signals.slow_er_row_major)?;
upload_slice_reuse(&mut rt.d_slow_z_pack, signals.slow_z_row_major)?;
upload_slice_reuse(&mut rt.d_kaufman_long, signals.kaufman_long)?;
upload_slice_reuse(&mut rt.d_kaufman_short, signals.kaufman_short)?;
```

### Device-row path integration sketch

Do the same for:

- `d_fast_sq_rows`
- `d_fast_mom_rows`
- `d_slow_ang_rows`
- `d_slow_er_rows`
- `d_slow_z_rows`
- `d_combo_*`
- `d_out`

This keeps both ASR entrypoints on the same reuse discipline.

---

## P1-A. Compact device-row packing to only the rows referenced by the current combo batch

### Why

This is the biggest improvement you can make **without changing the scoring kernel ABI**.

Right now the device-row path packs all `fast_rows` and all `slow_rows` even if the current combo batch only touches a subset.

If the batch uses:

- `F_used` out of `F` fast rows
- `S_used` out of `S` slow rows

then the pack work can shrink from:

- `2F + 3S` rows

to:

- `2F_used + 3S_used` rows

### Core helper: remap combo row indices to a compact used-row domain

```rust
fn remap_combo_rows(
    combo_rows: &[i32],
    total_rows: usize,
) -> Result<(Vec<i32>, Vec<usize>), String> {
    let mut old_to_new = vec![-1i32; total_rows];
    let mut unique_rows = Vec::new();
    let mut remapped = Vec::with_capacity(combo_rows.len());

    for &row in combo_rows {
        if row < 0 || row as usize >= total_rows {
            return Err(format!(
                "ASR GPU combo row index out of range: row={} total_rows={}",
                row, total_rows
            ));
        }

        let slot = &mut old_to_new[row as usize];
        if *slot < 0 {
            *slot = i32::try_from(unique_rows.len())
                .map_err(|_| "ASR GPU unique row count exceeds i32".to_string())?;
            unique_rows.push(row as usize);
        }
        remapped.push(*slot);
    }

    Ok((remapped, unique_rows))
}

fn gather_row_ptrs(src: &[u64], used_rows: &[usize]) -> Vec<u64> {
    used_rows.iter().map(|&row| src[row]).collect()
}
```

### Device-row integration sketch

```rust
let (combo_fast_remapped, used_fast_rows) =
    remap_combo_rows(combos.fast_row_index, signals.fast_rows)?;
let (combo_slow_remapped, used_slow_rows) =
    remap_combo_rows(combos.slow_row_index, signals.slow_rows)?;

let fast_sq_rows = gather_row_ptrs(signals.fast_squeeze_rows, &used_fast_rows);
let fast_mom_rows = gather_row_ptrs(signals.fast_momentum_rows, &used_fast_rows);
let slow_ang_rows = gather_row_ptrs(signals.slow_angle_rows, &used_slow_rows);
let slow_er_rows = gather_row_ptrs(signals.slow_er_rows, &used_slow_rows);
let slow_z_rows = gather_row_ptrs(signals.slow_z_rows, &used_slow_rows);

upload_slice_reuse(&mut rt.d_fast_sq_rows, &fast_sq_rows)?;
upload_slice_reuse(&mut rt.d_fast_mom_rows, &fast_mom_rows)?;
upload_slice_reuse(&mut rt.d_slow_ang_rows, &slow_ang_rows)?;
upload_slice_reuse(&mut rt.d_slow_er_rows, &slow_er_rows)?;
upload_slice_reuse(&mut rt.d_slow_z_rows, &slow_z_rows)?;

upload_slice_reuse(&mut rt.d_combo_fast, &combo_fast_remapped)?;
upload_slice_reuse(&mut rt.d_combo_slow, &combo_slow_remapped)?;
upload_slice_reuse(&mut rt.d_combo_start, combos.start_t)?;
upload_slice_reuse(&mut rt.d_combo_end, combos.end_t)?;

let compact_fast_rows = used_fast_rows.len();
let compact_slow_rows = used_slow_rows.len();

let fast_mat_len = compact_fast_rows
    .checked_mul(signals.cols)
    .ok_or_else(|| "ASR GPU compact fast matrix overflow".to_string())?;
let slow_mat_len = compact_slow_rows
    .checked_mul(signals.cols)
    .ok_or_else(|| "ASR GPU compact slow matrix overflow".to_string())?;

let d_fast_sq = ensure_uninit_device_len(&mut rt.d_fast_sq_pack, fast_mat_len)?;
let d_fast_mom = ensure_uninit_device_len(&mut rt.d_fast_mom_pack, fast_mat_len)?;
let d_slow_ang = ensure_uninit_device_len(&mut rt.d_slow_ang_pack, slow_mat_len)?;
let d_slow_er = ensure_uninit_device_len(&mut rt.d_slow_er_pack, slow_mat_len)?;
let d_slow_z = ensure_uninit_device_len(&mut rt.d_slow_z_pack, slow_mat_len)?;

launch_pack_rows_u64_ptr_f32(rt, rt.d_fast_sq_rows.as_ref().unwrap(), compact_fast_rows, signals.cols, d_fast_sq)?;
launch_pack_rows_u64_ptr_f32(rt, rt.d_fast_mom_rows.as_ref().unwrap(), compact_fast_rows, signals.cols, d_fast_mom)?;
launch_pack_rows_u64_ptr_f32(rt, rt.d_slow_ang_rows.as_ref().unwrap(), compact_slow_rows, signals.cols, d_slow_ang)?;
launch_pack_rows_u64_ptr_f32(rt, rt.d_slow_er_rows.as_ref().unwrap(), compact_slow_rows, signals.cols, d_slow_er)?;
launch_pack_rows_u64_ptr_f32(rt, rt.d_slow_z_rows.as_ref().unwrap(), compact_slow_rows, signals.cols, d_slow_z)?;
```

### Kernel launch changes

Use the compact row counts instead of the full sweep row counts:

```rust
let mut fast_rows_i = compact_fast_rows as i32;
let mut slow_rows_i = compact_slow_rows as i32;
```

### Why this matters

This keeps the existing `asr_backtest_metrics_f32` kernel ABI intact while still removing a large portion of device-row staging waste.

---

## P1-B. Mirror row compaction in the dense host-row-major path

### Why

The dense entrypoint can also avoid uploading unused rows when the combo batch references only a subset of rows.

### Helper

```rust
fn pack_selected_rows_f32(
    src_row_major: &[f32],
    cols: usize,
    used_rows: &[usize],
    out: &mut Vec<f32>,
) {
    out.clear();
    out.reserve(used_rows.len() * cols);

    for &row in used_rows {
        let base = row * cols;
        out.extend_from_slice(&src_row_major[base..base + cols]);
    }
}
```

### Dense-path usage

Use the same `remap_combo_rows(...)` helper as above.

If `used_fast_rows.len() < signals.fast_rows`, compact and upload only the selected fast rows:

```rust
pack_selected_rows_f32(
    signals.fast_squeeze_row_major,
    signals.cols,
    &used_fast_rows,
    &mut rt.host_fast_sq_scratch,
);
pack_selected_rows_f32(
    signals.fast_momentum_row_major,
    signals.cols,
    &used_fast_rows,
    &mut rt.host_fast_mom_scratch,
);

upload_slice_reuse(&mut rt.d_fast_sq_pack, &rt.host_fast_sq_scratch)?;
upload_slice_reuse(&mut rt.d_fast_mom_pack, &rt.host_fast_mom_scratch)?;
upload_slice_reuse(&mut rt.d_combo_fast, &combo_fast_remapped)?;
```

Do the same for slow rows.

### Recommended policy

Only compact when it is actually useful.

A simple first rule is:

- compact if `used_rows.len() < total_rows`
- otherwise upload the original row-major buffer directly

A slightly more conservative rule is:

- compact only if `used_rows.len() * 10 <= total_rows * 9`

That avoids paying host-side repack work when the batch already touches nearly all rows.

---

## P1-C. Remove `compute_log_returns_f32` allocation in favor of in-place fill

### Why

Once `AsrCudaRuntime` has `lr_host_scratch`, the standalone allocation-returning helper becomes unnecessary on the hot path.

### Recommendation

Keep `compute_log_returns_f32(...)` only if you still need it for tests or non-hot-path utilities. Otherwise replace hot-path usage with `fill_close_and_log_returns_f32(...)`.

---

## P1-D. Use reusable combo/output buffers everywhere

### Why

Combos and output are also repeatedly allocated.

### Recommendation

Replace all per-call combo allocations like this:

```rust
let d_combo_fast = DeviceBuffer::from_slice(combos.fast_row_index)?;
```

with reusable uploads:

```rust
upload_slice_reuse(&mut rt.d_combo_fast, combos.fast_row_index)?;
```

And replace:

```rust
let d_out = unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }?;
```

with:

```rust
let d_out = ensure_uninit_device_len(&mut rt.d_out, out_len)?;
```

This is small but worthwhile because these paths run frequently in optimization and validation loops.

---

## P2-A. Higher-ceiling follow-up: add a truly row-pointer-native ASR scoring kernel

### Why

The compact-pack fix above is the best same-ABI improvement, but it still packs dense matrices.

The long-term best route is to let the ASR kernel read device row pointers directly and skip packing altogether.

### Requires work outside the provided files

This requires updating the CUDA source/PTX that currently exposes `asr_backtest_metrics_f32` and `pack_rows_u64_ptr_f32`.

### Suggested future kernel shape

```cpp
extern "C" __global__
void asr_backtest_metrics_rows_f32(
    const float* close,
    const float* lr,
    int t_count,

    const uint64_t* fast_sq_rows,
    const uint64_t* fast_mom_rows,
    int fast_rows,

    const uint64_t* slow_ang_rows,
    const uint64_t* slow_er_rows,
    const uint64_t* slow_z_rows,
    int slow_rows,

    uint64_t kaufman_long_row,
    uint64_t kaufman_short_row,

    const int* combo_fast_idx,
    const int* combo_slow_idx,
    const int* combo_start_t,
    const int* combo_end_t,
    int combo_count,

    unsigned int flags,
    float commission,
    float angle_min,
    float angle_mr,
    int use_er_filter,
    float er_min,
    float er_mr,
    float z_entry,
    float z_exit,
    int confirm_bars,
    int mom_fade_bars,
    int max_trade_bars,
    int max_mr_trade_bars,

    int out_stride,
    float* out
);
```

### Rust-side launch boundary

The Rust-side API can stay very close to the current `eval_asr_metrics_gpu_kernel_device_rows(...)`, but without the five dense pack buffers and five pack launches.

### Why not do this first?

Because the compact-pack approach is smaller, safer, and already likely to pay off significantly.

---

## P2-B. Higher-ceiling follow-up: compute log returns on device once per resident close series

### Why

If the upstream ASR route already has close on device, host-side log-return computation is no longer ideal.

### Recommended staging

1. keep the P0 market-residency/cache patch first
2. then add a small CUDA kernel to compute log returns from resident `d_close`
3. only refresh `d_lr` when the market-series cache key changes

### Why this is not P0

It requires additional PTX/kernel-source work and is not needed to get a meaningful first round of gains.

---

## File 2: `asr.rs`

## P0-E. Hoist Kaufman-stop wrapper construction and deduplicate parameter setup

### Why

`compute_asr_kaufman_stops_cuda_device(...)` constructs wrapper/setup work twice through the closure-based flow.

### Suggested refactor

```rust
#[cfg(feature = "cuda")]
fn build_kaufmanstop_params(
    strategy: &StrategyConfig,
    direction: &str,
) -> Vec<IndicatorParamKV> {
    vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.kaufman_stop_period().max(1) as i64),
        },
        IndicatorParamKV {
            key: "mult".to_string(),
            value: IndicatorParamValue::Float(strategy.kaufman_stop_mult()),
        },
        IndicatorParamKV {
            key: "direction".to_string(),
            value: IndicatorParamValue::EnumString(direction.to_string()),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(strategy.kaufman_stop_ma_type().to_string()),
        },
    ]
}

#[cfg(feature = "cuda")]
pub fn compute_asr_kaufman_stops_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    strategy: &StrategyConfig,
    device_id: u32,
) -> Result<AsrKaufmanStopsDevice, String> {
    let wrapper = CudaKaufmanstop::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.high_low_first_valid();
    let output_id = find_dynamic_indicator_descriptor("kaufmanstop")
        .and_then(|d| d.outputs.first())
        .map(|o| o.id.clone());

    let sweep_for_direction =
        |direction: &str| vector_ta::indicators::kaufmanstop::KaufmanstopBatchRange {
            period: (
                strategy.kaufman_stop_period().max(1) as usize,
                strategy.kaufman_stop_period().max(1) as usize,
                0,
            ),
            mult: (
                strategy.kaufman_stop_mult(),
                strategy.kaufman_stop_mult(),
                0.0,
            ),
            direction: (direction.to_string(), direction.to_string(), 0.0),
            ma_type: (
                strategy.kaufman_stop_ma_type().to_string(),
                strategy.kaufman_stop_ma_type().to_string(),
                0.0,
            ),
        };

    let compute_direction = |direction: &str| -> Result<AsrOwnedDeviceMatrix, String> {
        let sweep = sweep_for_direction(direction);
        wrapper
            .kaufmanstop_batch_dev_from_device_inputs(
                prepared_inputs.high_device().buffer(),
                prepared_inputs.low_device().buffer(),
                prepared_inputs.len(),
                first_valid,
                &sweep,
            )
            .map(|(matrix, _)| AsrOwnedDeviceMatrix::from_raw(matrix))
            .or_else(|_| {
                let params = build_kaufmanstop_params(strategy, direction);
                if let Some(output_id) = output_id.as_deref() {
                    compute_indicator_f32_cuda_device(
                        prepared_inputs,
                        "kaufmanstop",
                        output_id,
                        params.as_slice(),
                    )
                    .map(|(matrix, _)| AsrOwnedDeviceMatrix::from_dispatch(matrix))
                } else {
                    let candidates = [
                        IndicatorInputKind::HighLow,
                        IndicatorInputKind::Candles,
                        IndicatorInputKind::Ohlc,
                    ];
                    let mut last_err: Option<String> = None;
                    for kind in candidates {
                        match compute_indicator_f32_cuda_device_direct(
                            prepared_inputs,
                            "kaufmanstop",
                            None,
                            kind,
                            params.as_slice(),
                        ) {
                            Ok((matrix, _)) => {
                                return Ok(AsrOwnedDeviceMatrix::from_dispatch(matrix));
                            }
                            Err(err) => last_err = Some(err),
                        }
                    }
                    Err(last_err.unwrap_or_else(|| {
                        "ASR kaufman-stop CUDA device dispatch failed for all input kinds"
                            .to_string()
                    }))
                }
            })
    };

    let long = compute_direction("long")?;
    let short = compute_direction("short")?;
    // ... keep existing shape checks and first_valid logic ...
}
```

### Why this matters

- avoids repeated wrapper construction
- centralizes Kaufman parameter generation
- keeps fallback behavior intact
- reduces duplicated work without changing route semantics

---

## P0-F. Use the z-score fast-path helper everywhere the route assumption matters

### Why

The current direct/native z-score eligibility rule is strategy-local and easy to forget.

### Recommended code change

Replace the inline gate in `compute_asr_zscore_sweep_cuda_device(...)`:

```rust
let sweep = if cfg.z_devtype == 0 && cfg.z_ma_type.eq_ignore_ascii_case("sma") {
```

with:

```rust
let sweep = if asr_uses_direct_zscore_cuda_fast_path(cfg) {
```

### Why this matters

- prevents benchmark drift
- keeps route assumptions centralized
- makes it easy to build route-aware benchmark sets

---

## P1-E. Keep route boundaries explicit instead of broadening GPU support prematurely

### Recommendation

Do **not** broaden the ASR GPU scoring kernel to overlays / stop-loss / take-profit in the same patch set as the staging fixes.

That is a larger parity surface and should stay separate from the current performance cleanup.

---

## P1-F. Optional later step: widen direct z-score eligibility only after parity tests

### Why

The direct z-score function already accepts `ma_type` and `devtype`.

That suggests the current direct/native gate may be conservative rather than fundamental.

### Recommended process

1. add parity tests comparing direct/native versus generic CUDA z-score sweeps for:
   - `sma/devtype=0`
   - `ema/devtype=0`
   - `ema/devtype=1`
   - any other in-use combinations
2. if parity is acceptable, widen `asr_uses_direct_zscore_cuda_fast_path(...)`
3. benchmark before/after

This is explicitly a **second-wave** change, not part of the first safety-focused patch set.

---

## File 3: `gpu_native_expansion_opt.rs`

## P0-G. Add real ASR optimizer benchmarks

### Why

This file currently provides no ASR optimizer evidence.

### Imports to extend

```rust
use ta_app_core::{
    AsrStrategyParams, Backend, BackendUsed, CarmStrategyParams, CdmcStrategyParams,
    CompositeCpuRequest, CompositeSignalMode, KqsrStrategyParams, KvdfStrategyParams,
};
```

### Add an ASR-focused synthetic candle generator

Use a candle set that mixes trend and mean-reversion regimes so ASR’s state machine is not benchmarked on a trivial monotonic series.

```rust
fn make_asr_regime_candles(len: usize) -> Candles {
    let len = len.max(400);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol = Vec::with_capacity(len);

    let mut px = 100.0f64;
    for i in 0..len {
        ts.push(i as i64);

        let regime = (i / 250) % 4;
        let drift = match regime {
            0 => 0.0006,  // trend up
            1 => 0.0,     // mean-reverting chop
            2 => -0.0005, // trend down
            _ => 0.0,     // high-vol chop
        };
        let osc = match regime {
            1 => (i as f64 * 0.11).sin() * 0.010,
            3 => (i as f64 * 0.21).sin() * 0.015,
            _ => (i as f64 * 0.05).sin() * 0.003,
        };

        px *= 1.0 + drift + osc;
        let next = px * (1.0 + (i as f64 * 0.07).cos() * 0.0015);

        open.push(px * 0.9995);
        high.push(px.max(next) * 1.0015);
        low.push(px.min(next) * 0.9985);
        close.push(px);
        vol.push(1_000.0 + ((i % 17) as f64) * 40.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

### Add an ASR request builder

```rust
fn make_asr_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req.fast_ma_types = vec!["ema".to_string()];
    req.slow_ma_types = vec!["ema".to_string()];
    req.asr = Some(AsrStrategyParams::default());
    req
}
```

### Tighten the generic GPU benchmark assertion while you are there

`bench_gpu_cases(...)` already checks `backend_used == GPU`. For strategy-local GPU-native benchmarks, tighten it to also require `backend_fallback_reason == None`:

```rust
assert!(
    matches!(probe.backend_used, BackendUsed::Gpu)
        && probe.backend_fallback_reason.is_none(),
    "expected pure GPU backend for {} probe, got backend={:?}, route={:?}, fallback={:?}",
    route_label,
    probe.backend_used,
    probe.backend_route,
    probe.backend_fallback_reason
);
```

Do the same inside the measured loop result assertion.

### Add CPU and GPU ASR benches

```rust
fn bench_asr_cpu(c: &mut Criterion) {
    if !bench_target_enabled("asr") {
        return;
    }
    let candles = make_asr_regime_candles(200_000);
    bench_cpu_cases(c, "asr/cpu/200k", &candles, make_asr_req);
}

fn bench_asr_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("asr") {
        return;
    }
    let candles = make_asr_regime_candles(200_000);
    bench_gpu_cases(
        c,
        "asr/gpu_kernel/200k",
        "asr/gpu_kernel/200k",
        &candles,
        make_asr_req,
    );
}
```

### Add them to the Criterion group

```rust
criterion_group!(
    gpu_native_expansion_benches,
    bench_asr_cpu,
    bench_asr_gpu_kernel,
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

### Recommended follow-up variant

Add a second ASR benchmark group with a non-default z-score configuration so you can compare:

- direct/native z-score route
- generic CUDA z-score route

That is useful once the first ASR benches are in place.

---

## P1-G. Be explicit about warm-path versus cold-path interpretation

### Why

`bench_gpu_cases(...)` does a route probe before the measured loop, which warms the GPU path.

### Recommendation

Either:

- keep that behavior and label the benchmark as steady-state / warm-path, or
- add equivalent warm-up for CPU, or
- split into `cold_probe` and `warm_steady_state` groups

This is not required for correctness, but it makes timing-boundary interpretation cleaner.

---

## File 4: `composite_validation_workflow.rs`

## P0-H. Make ASR GPU validation benchmarks fail on fallback

### Why

A benchmark labeled `gpu_kernel/asr/...` should not silently measure CPU or fallback behavior.

### Imports to extend

```rust
use ta_app_core::{
    AsrStrategyParams, BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeParams,
    CompositeValidationRequest, CscvPboConfig, PerturbConfig, TradeBootstrapConfig,
    WalkForwardConfigOptional,
};
```

### After `route_probe`, assert the route instead of only printing it

```rust
assert!(
    matches!(route_probe.backend_used, BackendUsed::Gpu)
        && route_probe.backend_fallback_reason.is_none(),
    "expected pure GPU route for composite_validate/gpu_kernel/asr/100k/64candidates, got backend={:?}, route={:?}, fallback={:?}",
    route_probe.backend_used,
    route_probe.backend_route,
    route_probe.backend_fallback_reason
);
```

Do the same inside the benchmark closure if you want the benchmark to fail on any runtime downgrade, not just on the initial probe.

---

## P0-I. Split ASR validation candidates into direct-zscore and generic-zscore sets

### Why

Right now ASR validation candidates are all built from `AsrStrategyParams::default()`, which only measures one ASR route profile.

### Candidate builder upgrade

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_asr_candidates_with_zscore_mode(
    n: usize,
    z_ma_type: &str,
    z_devtype: u8,
) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);

        let mut asr = AsrStrategyParams::default();
        asr.squeeze_period = 20u16.saturating_add(i_u16 % 5);
        asr.trend_period = 14u16.saturating_add(i_u16 % 7);
        asr.er_period = 20u16.saturating_add(i_u16 % 9);
        asr.z_period = 14u16.saturating_add(i_u16 % 11);
        asr.z_ma_type = z_ma_type.to_string();
        asr.z_devtype = z_devtype;

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
            asr: Some(asr),
            vcc: None,
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

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_asr_candidates(n: usize) -> Vec<CompositeParams> {
    make_asr_candidates_with_zscore_mode(n, "sma", 0)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_asr_candidates_generic_zscore(n: usize) -> Vec<CompositeParams> {
    make_asr_candidates_with_zscore_mode(n, "ema", 1)
}
```

### Then add both benchmark profiles

```rust
bench_profile("asr", 50_000, make_asr_candidates(8));
bench_profile("asr_generic_zscore", 50_000, make_asr_candidates_generic_zscore(8));
```

### Why this matters

It separates:

- native/direct z-score ASR route
- generic-period-sweep z-score ASR route

which is exactly the sort of strategy-local route difference the review-map wants surfaced.

---

## P1-H. Add an explicit ASR negative-route / blocker case if your validation API allows it

### Why

A benchmark suite is more trustworthy when it includes one explicit case that should **not** stay on GPU.

Possible negative ASR cases:

- `strategy.stop_loss_pct = Some(...)`
- `strategy.take_profit_pct = Some(...)`
- execution overlay enabled

Use the `asr_gpu_kernel_blocker(...)` helper from `asr.rs` to label these cases explicitly.

I would not time these as “GPU” benches; I would add them as route-matrix assertions or route-probe tests.

---

# Suggested implementation order

## Wave 1 — smallest, safest, highest ROI

1. Add `asr_gpu_kernel_blocker(...)` and `asr_uses_direct_zscore_cuda_fast_path(...)` in `asr.rs`
2. Hoist `CudaKaufmanstop` wrapper construction in `compute_asr_kaufman_stops_cuda_device(...)`
3. Expand `AsrCudaRuntime` scratch/cache fields
4. Add `upload_slice_reuse(...)`
5. Add `ensure_uninit_device_len(...)`
6. Add `fill_close_and_log_returns_f32(...)`
7. Add `copy_device_to_vec_uninit(...)`
8. Switch both ASR CUDA entrypoints to runtime-backed market-series reuse
9. Switch both entrypoints to uninitialized host readback
10. Reuse combo and output buffers

## Wave 2 — biggest same-ABI payoff

11. Add `remap_combo_rows(...)`
12. Compact the device-row path to only used fast/slow rows
13. Compact the dense path when selected rows are a strict subset
14. Re-benchmark optimizer and validation throughput

## Wave 3 — benchmark fidelity cleanup

15. Add ASR optimizer benches in `gpu_native_expansion_opt.rs`
16. Add ASR generic-zscore validation profile in `composite_validation_workflow.rs`
17. Make ASR GPU validation benches fail on fallback
18. Decide whether warm-path labeling needs to be more explicit

## Wave 4 — only after the above is stable

19. Consider a truly row-pointer-native ASR scoring kernel
20. Consider device-side log-return generation
21. Consider widening direct z-score eligibility if parity holds

---

# Testing and validation plan

## Unit-level tests / local tests

### 1. `remap_combo_rows(...)`

Test:

- empty input
- repeated indices
- full coverage
- sparse subset
- negative row index -> error
- out-of-range row index -> error

### 2. `pack_selected_rows_f32(...)`

Test:

- contiguous selected rows
- non-contiguous rows
- single row
- all rows
- no rows

### 3. `copy_device_to_vec_uninit(...)`

Test with a small known device buffer and verify exact equality after readback.

## Strategy parity tests

### Dense path parity

Compare:

- `eval_asr_metrics_gpu_kernel(...)`
- CPU reference metrics path

### Device-row path parity

Compare:

- `eval_asr_metrics_gpu_kernel_device_rows(...)`
- `eval_asr_metrics_gpu_kernel(...)`
- CPU reference metrics path

Test across:

- `long_only = true/false`
- `allow_flip = true/false`
- `trade_on_next_bar = true/false`
- representative `confirm_bars`
- representative `mom_fade_bars`
- `use_er_filter = true/false`
- default direct-zscore config
- generic-zscore config

## Route boundary tests

Verify that ASR GPU kernel rejects:

- execution overlays
- stop-loss
- take-profit

Verify that those same configs still succeed on the CPU reference route.

## Benchmark validation steps

### Optimizer (`gpu_native_expansion_opt.rs`)

Run before/after for:

- `asr/cpu/200k`
- `asr/gpu_kernel/200k`

Capture:

- time per iteration
- whether the route stayed GPU
- whether warm-path improvements increased the GPU lead

### Validation (`composite_validation_workflow.rs`)

Run before/after for:

- `composite_validate/gpu_kernel/asr/100k/64candidates`
- `composite_validate/gpu_kernel/asr/100k/192candidates`
- `composite_validate/gpu_kernel/asr_generic_zscore/50k/8candidates`

Capture:

- backend used
- fallback reason
- validation wall-clock
- any route regressions

## What success looks like

Success is not “the kernel changed.” Success is:

- fewer allocations in warm ASR GPU runs
- fewer H2D uploads on repeated candle sets
- smaller pack volume in device-row mode
- removal of unnecessary host zero-fills
- honest benchmark labels
- unchanged metric parity on supported GPU configs

---

# What I would **not** do in the first patch set

1. I would **not** broaden GPU support to overlays / stop-loss / take-profit in the same change set.
2. I would **not** rewrite ASR scoring semantics just to chase a theoretical speedup.
3. I would **not** assume non-default z-score direct support is safe without parity tests.
4. I would **not** optimize away copies that are still real producer->consumer boundaries.
5. I would **not** replace the CPU reference path; it is still the route-parity anchor.

---

# Recommended first commit boundaries

A clean implementation sequence would be:

## Commit 1 — route helpers + Kaufman-stop cleanup
- add `asr_gpu_kernel_blocker(...)`
- add `asr_uses_direct_zscore_cuda_fast_path(...)`
- refactor `compute_asr_kaufman_stops_cuda_device(...)`

## Commit 2 — ASR runtime reuse + uninitialized readback
- extend `AsrCudaRuntime`
- add upload/reuse helpers
- add in-place market-series fill
- add `copy_device_to_vec_uninit(...)`
- switch both ASR CUDA entrypoints to runtime-backed scratch

## Commit 3 — row compaction
- add `remap_combo_rows(...)`
- compact device-row path
- compact dense path when worthwhile

## Commit 4 — benchmark coverage
- add ASR optimizer benches
- tighten ASR validation GPU route assertions
- add generic-zscore ASR validation profile

## Commit 5 — optional PTX follow-up
- row-pointer-native ASR scoring kernel
- optional device-side log-return generation

---

# Detailed implementation checklist

## `asr.rs`

- [ ] Add `AsrGpuKernelBlocker`
- [ ] Add `pub fn asr_gpu_kernel_blocker(strategy: &StrategyConfig) -> Option<AsrGpuKernelBlocker>`
- [ ] Add `pub fn asr_uses_direct_zscore_cuda_fast_path(cfg: &AsrStrategyParams) -> bool`
- [ ] Replace the inline z-score gate with `asr_uses_direct_zscore_cuda_fast_path(cfg)`
- [ ] Add `build_kaufmanstop_params(...)`
- [ ] Hoist `CudaKaufmanstop::new(...)` out of the per-direction closure
- [ ] Reuse descriptor/output-id lookup in `compute_asr_kaufman_stops_cuda_device(...)`
- [ ] Keep existing fallback behavior intact after the refactor
- [ ] Add parity tests for the refactored Kaufman-stop path
- [ ] Add route-helper tests for overlays / stop-loss / take-profit

## `asr_gpu_kernel.rs` — runtime/cache foundation

- [ ] Extend `AsrCudaRuntime` with market-series cache fields
- [ ] Extend `AsrCudaRuntime` with reusable dense Kaufman-stop buffers
- [ ] Extend `AsrCudaRuntime` with reusable combo buffers
- [ ] Extend `AsrCudaRuntime` with reusable output buffer
- [ ] Extend `AsrCudaRuntime` with reusable row-pointer buffers
- [ ] Extend `AsrCudaRuntime` with reusable pack scratch buffers
- [ ] Add `SeriesCacheKey`
- [ ] Add `upload_slice_reuse(...)`
- [ ] Add `ensure_uninit_device_len(...)`
- [ ] Add `fill_close_and_log_returns_f32(...)`
- [ ] Add `ensure_market_series(...)`
- [ ] Replace per-call `close_f32` allocation in dense path
- [ ] Replace per-call `lr_f32` allocation in dense path
- [ ] Replace per-call `close_f32` allocation in device-row path
- [ ] Replace per-call `lr_f32` allocation in device-row path
- [ ] Replace per-call dense Kaufman-stop uploads with reusable buffers
- [ ] Replace per-call combo buffer allocation with reusable uploads
- [ ] Replace per-call output buffer allocation with reusable `d_out`
- [ ] Replace inline route-blocker logic with `crate::asr::asr_gpu_kernel_blocker(...)`

## `asr_gpu_kernel.rs` — uninitialized readback

- [ ] Add `copy_device_to_vec_uninit(...)`
- [ ] Replace zero-filled `host_out` in dense path
- [ ] Replace zero-filled `host_out` in device-row path
- [ ] Verify the kernel always writes the full output row for every combo before keeping this optimization
- [ ] Prefer a shared `alloc_policy` helper if one already exists in the repo

## `asr_gpu_kernel.rs` — row compaction

- [ ] Add `remap_combo_rows(...)`
- [ ] Add `gather_row_ptrs(...)`
- [ ] Add `pack_selected_rows_f32(...)`
- [ ] Compact fast-row usage in dense path
- [ ] Compact slow-row usage in dense path
- [ ] Compact fast-row usage in device-row path
- [ ] Compact slow-row usage in device-row path
- [ ] Remap combo fast indices to compact fast-row domain
- [ ] Remap combo slow indices to compact slow-row domain
- [ ] Launch pack kernels with compact row counts
- [ ] Pass compact `fast_rows_i` / `slow_rows_i` into the scoring kernel
- [ ] Add tests for repeated-row and sparse-row cases
- [ ] Add parity tests comparing compacted versus uncompacted launches

## `gpu_native_expansion_opt.rs`

- [ ] Import `AsrStrategyParams`
- [ ] Add `make_asr_regime_candles(...)`
- [ ] Add `make_asr_req(...)`
- [ ] Add `bench_asr_cpu(...)`
- [ ] Add `bench_asr_gpu_kernel(...)`
- [ ] Add both ASR benches to `criterion_group!(...)`
- [ ] Confirm the route probe for ASR stays on GPU
- [ ] Tighten `bench_gpu_cases(...)` to require `backend_fallback_reason == None`
- [ ] Decide whether to add a second `asr_generic_zscore` benchmark case
- [ ] Decide whether the benchmark should be explicitly labeled warm-path

## `composite_validation_workflow.rs`

- [ ] Import `AsrStrategyParams`
- [ ] Import `BackendUsed`
- [ ] Add `make_asr_candidates_with_zscore_mode(...)`
- [ ] Keep `make_asr_candidates(...)` as the direct-zscore default set
- [ ] Add `make_asr_candidates_generic_zscore(...)`
- [ ] Assert GPU backend after the ASR route probe
- [ ] Assert no fallback after the ASR route probe
- [ ] Consider asserting GPU backend inside the measured closure as well
- [ ] Add `bench_profile("asr_generic_zscore", ...)`
- [ ] Decide whether to add an explicit negative-route ASR test case outside the timed GPU group
- [ ] Re-run validation benchmarks after the runtime/cache patches land

## Parity / regression work

- [ ] Compare dense GPU ASR metrics against CPU reference metrics
- [ ] Compare device-row GPU ASR metrics against dense GPU ASR metrics
- [ ] Compare direct-zscore GPU ASR metrics against generic-zscore GPU ASR metrics where equivalent
- [ ] Verify long-only behavior
- [ ] Verify no-flip behavior
- [ ] Verify trade-on-next-bar behavior
- [ ] Verify confirm-bars behavior
- [ ] Verify momentum-fade behavior
- [ ] Verify ER-filter on/off behavior
- [ ] Verify blocked configs still route away from GPU scoring correctly

## Measurement / evidence

- [ ] Capture before/after ASR optimizer benchmark numbers
- [ ] Capture before/after ASR validation benchmark numbers
- [ ] Capture route/fallback metadata for all ASR benchmark cases
- [ ] Record whether improvements are warm-path only or cold-path as well
- [ ] Measure compact-row usage ratios (`used_fast_rows / fast_rows`, `used_slow_rows / slow_rows`)
- [ ] Decide whether the row-pointer-native kernel is still worth doing after the same-ABI compaction patch
- [ ] Only then consider device-side log-return generation
- [ ] Only then consider widening direct z-score eligibility

---

## Final recommendation

For the current ASR bundle, the best path is:

1. **keep ASR GPU-native**
2. **remove staging waste first**
3. **make benchmark labels honest**
4. **keep CPU-preferred behavior for blocked configs**
5. **only then consider higher-risk kernel ABI changes**

The biggest immediate win is **not** a new trading algorithm or a broad rewrite. It is the combination of:

- resident market-series reuse
- reusable combo/output buffers
- uninitialized host readback where safe
- compacting the device-row path to only the rows actually used by the combo batch
- adding real ASR benchmark coverage and route assertions

That gives you the highest chance of measurable speedup with the lowest parity risk.