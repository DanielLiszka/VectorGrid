
# VCC strategy-local GPU-native optimization and validation plan

## Scope

This document follows the review bundle for **“## 5. Strategy-local GPU-native optimization and validation routes”** and stays limited to the **VCC strategy family** and the four files you provided:

- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

It also follows the prompt constraints from the map:

- stay benchmark-first and parity-preserving
- focus on strategy-local staging waste, CPU/GPU payoff, fallback transparency, validation throughput, and benchmark fidelity
- prefer smaller parity-safe changes before large refactors
- only use uninitialized allocation where every element is guaranteed to be written before any read

## Reviewed surface

The VCC hot path is split into two different families:

1. **Reference / parity path in `vcc.rs`**
   - Computes fast/trend signals via dynamic indicator dispatch.
   - Can route through CUDA, but the public helpers still materialize host `Vec<f64>` series.
   - This path is useful for CPU reference, parity diagnostics, and caller simplicity.

2. **GPU-native metric path in `vcc_gpu_kernel.rs`**
   - Accepts device row pointers or host row-major matrices for:
     - fast squeeze rows
     - fast momentum rows
     - trend rows
   - Builds contiguous device matrices and launches `vcc_backtest_metrics_f32`.
   - This is the strategy-local CUDA execution path that matters most for optimizer and validation throughput.

The benchmark files show that VCC currently has **some** validation-cost coverage but **no dedicated optimizer Criterion coverage**, so the current repo state is not strong enough to declare VCC “CPU-preferred” or “GPU-preferred” on measured workloads yet.

---

## Executive summary

The highest-value VCC CUDA improvement is:

1. **Cache price-state device buffers and packed signal matrices inside `VccCudaRuntime`** so repeated calls with the same candles and same selected rows do **not** rebuild:
   - `close_f32`
   - `lr_f32`
   - `lr_f64`
   - packed fast squeeze matrix
   - packed fast momentum matrix
   - packed trend matrix
   - combo scratch/output buffers

2. **Expose VCC route/fallback metadata** in `vcc.rs` so device-input fallback and CUDA-host fallback do not stay silent.

3. **Add real VCC optimizer and heavier validation benches** before touching the kernel ABI.

4. **Only after those three land**, consider higher-risk work:
   - direct row-pointer kernel consumption
   - prepared device price-state injection
   - fusing fast-signal staging paths

The current VCC GPU metric function is paying a repeated cold staging tax on every invocation. That is the first-order issue. Everything else is secondary until that is fixed and measured.

---

## Current hot-path data movement

### Current behavior in `vcc_gpu_kernel.rs :: eval_vcc_metrics_gpu_kernel_device_rows_detailed`

Per invocation, the function currently does all of the following again:

1. build `close_f32` on the host
2. build `lr_f32` on the host
3. build `lr_f64` on the host
4. upload all three price/log-return arrays to the device
5. allocate device buffers for packed fast/trend matrices
6. upload row-pointer lists for fast squeeze / fast momentum / trend
7. launch `pack_rows_u64_ptr_f32` up to three times
8. upload combo vectors
9. allocate output buffers
10. launch `vcc_backtest_metrics_f32`
11. synchronize
12. allocate host output vectors
13. copy results back to the host
14. decode metrics rows

### Desired warm-path behavior after the first pass of edits

For repeated evaluation on the **same candles** and the **same selected VCC rows**, the warm path should reduce to:

1. upload combo vectors only
2. launch `vcc_backtest_metrics_f32`
3. synchronize
4. copy outputs back
5. decode metrics rows

That change alone removes the biggest avoidable H→D traffic and allocation churn without changing strategy logic or kernel semantics.

---

## 1. Surface understanding

### `vcc.rs`

The file owns:

- `VccStrategyParams`
- period resolution and validation
- CPU/GPU host-series signal helpers
- device sweep helpers for:
  - `ttm_squeeze`
  - `squeeze_momentum`
  - `ttm_trend`
- parity helpers:
  - `vcc_momentum_roundtrip_parity`
  - `vcc_momentum_roundtrip_is_fp32_stable`
- CPU desired-position builder
- a strong set of ignored GPU diagnostics

Important behavior:

- `compute_indicator_f64_with_warmup` prefers CUDA only when the descriptor and backend say it should, but silently falls back to CPU if the CUDA call fails.
- `compute_vcc_ttm_squeeze_sweep_cuda_direct`, `compute_vcc_squeeze_momentum_sweep_cuda_direct`, and `compute_vcc_ttm_trend_sweep_cuda_direct` first try device-input routes and silently fall back to host-input wrapper routes on failure.
- `compute_vcc_fast_signals` and `compute_vcc_trend_signals` always return host `Vec<f64>` series even when the internal compute path used CUDA.

### `vcc_gpu_kernel.rs`

The file owns:

- strategy-local CUDA runtime bootstrap
- input validation for VCC device rows
- row-pointer packing via `pack_rows_u64_ptr_f32`
- launch of `vcc_backtest_metrics_f32`
- decode of strategy-local GPU metrics

Important behavior:

- It uses a thread-local `VCC_GPU_RUNTIME`, but that runtime only stores:
  - CUDA context
  - PTX module
  - stream
- It does **not** store resident price buffers, packed matrices, combo scratch, or reusable host readback buffers.

### `gpu_native_expansion_opt.rs`

The file is a Criterion optimizer harness for other GPU-native strategies:

- `kqsr`
- `carm`
- `kvdf`
- `cdmc`

Important behavior:

- It already has a good route-probe/assert pattern for GPU-native strategies.
- It does **not** include VCC at all.
- That means there is no VCC optimizer-side Criterion evidence in the provided benchmark harness.

### `composite_validation_workflow.rs`

The file is a validation-cost harness that includes VCC in a generic profile closure.

Important behavior:

- VCC appears in `bench_profile("vcc", 50_000, make_vcc_candidates(8))`.
- The GPU profile benches print `backend_used`, `backend_route`, and `fallback`, but they do **not** assert that the route stayed on GPU.
- The heavy dedicated GPU workflow is ASR-only.
- `make_vcc_candidates(n)` repeats only a small set of period combinations for larger `n`, which can under-stress VCC row-pack and matrix-width costs.

---

## 2. Findings

### F1 — repeated staging and allocation in the VCC GPU metric path

- **Status:** confirmed issue
- **Severity:** high
- **Type:** performance
- **Location:** `vcc_gpu_kernel.rs :: eval_vcc_metrics_gpu_kernel_device_rows_detailed`
- **Timing boundary affected:** app-path, validation-cost harness, Criterion warm path
- **Why it matters:** the function rebuilds and reuploads price/log-return state, repacks matrices, reallocates combo buffers, reallocates outputs, and reallocates host readback vectors on every call. That is avoidable repeated work in the core strategy-local CUDA path.
- **Recommended change:** persist price-state device buffers, packed signal matrices, combo scratch buffers, and host readback vectors in `VccCudaRuntime`. Recompute/repack only when the input identity changes.

### F2 — three host passes and multiple zero-filled buffers for price-state prep

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** performance
- **Location:** `vcc_gpu_kernel.rs :: eval_vcc_metrics_gpu_kernel_device_rows_detailed`, `compute_log_returns_f32`, `compute_log_returns_f64`
- **Timing boundary affected:** app-path
- **Why it matters:** the current flow does:
  - one pass to build `close_f32`
  - one pass to build `lr_f32`
  - one pass to build `lr_f64`
  - zero-fills the `lr_*` vectors before overwriting most elements
- **Recommended change:** replace that with a single host pass that builds all three outputs together using `Vec::with_capacity` and `push`, or route through `alloc_policy` prefix-init helpers if they already exist in the repo.

### F3 — silent fallback hides whether VCC is really using the intended GPU path

- **Status:** confirmed issue
- **Severity:** high
- **Type:** benchmark-fidelity / correctness
- **Location:** 
  - `vcc.rs :: compute_indicator_f64_with_warmup`
  - `vcc.rs :: compute_vcc_ttm_squeeze_sweep_cuda_direct`
  - `vcc.rs :: compute_vcc_squeeze_momentum_sweep_cuda_direct`
  - `vcc.rs :: compute_vcc_ttm_trend_sweep_cuda_direct`
- **Timing boundary affected:** route-matrix, Criterion, validation-cost harness
- **Why it matters:** these functions silently downgrade from:
  - device-input CUDA → host-input CUDA wrapper, or
  - CUDA host output → CPU
  without surfacing the fallback reason. That makes route claims and benchmark labels less trustworthy.
- **Recommended change:** split these helpers into “attempt device/GPU path” and “fallback path” branches that return route metadata. Keep the existing public signatures as thin compatibility wrappers if needed.

### F4 — VCC has no optimizer Criterion coverage in the provided benchmark harness

- **Status:** confirmed issue
- **Severity:** high
- **Type:** benchmark-fidelity
- **Location:** `gpu_native_expansion_opt.rs`
- **Timing boundary affected:** Criterion
- **Why it matters:** there is no `make_vcc_req`, no VCC candle generator, and no VCC CPU/GPU benchmark groups. You cannot make a benchmark-backed CPU/GPU payoff claim for VCC from this file set.
- **Recommended change:** add VCC-specific request builders and Criterion groups that mirror the existing KQSR/CARM/KVDF/CDMC harness pattern.

### F5 — VCC validation coverage exists, but the GPU route is not asserted

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** benchmark-fidelity / testing
- **Location:** `composite_validation_workflow.rs :: bench_composite_validation_gpu_workflow`
- **Timing boundary affected:** validation-cost harness
- **Why it matters:** GPU validation groups print route information, but do not assert that `backend_used` stayed on GPU. A fallback can therefore be benchmarked under a GPU-labeled group.
- **Recommended change:** add route assertions after the probe and inside the benchmark iteration for all GPU-labeled VCC groups.

### F6 — VCC validation shape is too light for the staging problem you actually need to measure

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `composite_validation_workflow.rs`
- **Timing boundary affected:** validation-cost harness
- **Why it matters:** VCC only appears in the generic `bench_profile("vcc", 50_000, make_vcc_candidates(8))` path. The heavier dedicated GPU cases are ASR-only, so VCC staging and row-pack reuse are not being stress-tested the same way.
- **Recommended change:** add a dedicated heavy VCC GPU validation group and at least one larger VCC profile case (`100k / 32` or similar) so the cache/reuse work has a benchmark target.

### F7 — `make_vcc_candidates(n)` under-stresses unique-row packing for larger candidate counts

- **Status:** likely issue
- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `composite_validation_workflow.rs :: make_vcc_candidates`
- **Timing boundary affected:** validation-cost harness
- **Why it matters:** `fast_len = 8 + (i % 12)` and `slow_len = fast_len + 24` means larger candidate counts cycle through the same narrow period set. For VCC, that can understate the cost of packing/selecting unique rows because the candidate count grows faster than row diversity.
- **Recommended change:** add a second helper such as `make_vcc_candidates_wide(n)` for heavy VCC validation groups so unique-row pressure scales more realistically.

### F8 — VCC parity knowledge exists, but it is still diagnostic-only

- **Status:** confirmed issue
- **Severity:** medium-high
- **Type:** parity / testing
- **Location:** `vcc.rs :: ignored diagnostic tests`
- **Timing boundary affected:** profiling-only, validation-cost harness
- **Why it matters:** the file already has strong diagnostics for:
  - CPU vs CUDA host series
  - device sweeps vs CPU series
  - short-combo parity
  - validation batch parity
  - momentum roundtrip stability
  but they are ignored/log-only. That means the exact parity surface you are about to optimize is not guarded by a regression test.
- **Recommended change:** promote one small VCC GPU metric parity case and one momentum roundtrip case into assertive, non-ignored tests before making the larger hot-path changes.

### F9 — fixed-size param vectors and full zero-fill in `build_vcc_desired_positions` are safe but not optimal

- **Status:** optimization opportunity
- **Severity:** low
- **Type:** performance
- **Location:** 
  - `vcc.rs :: compute_vcc_fast_signals`
  - `vcc.rs :: compute_vcc_trend_signals`
  - `vcc.rs :: build_vcc_desired_positions`
- **Timing boundary affected:** app-path, CPU reference path
- **Why it matters:** these helpers:
  - allocate small heap `Vec`s for fixed-size parameter lists
  - zero-fill the entire `desired` vector even though the tail `[start_t..n)` is always overwritten in the loop
- **Recommended change:** replace fixed-size `Vec`s with stack arrays, and consider a prefix-init helper for `desired` only if `alloc_policy.rs` already provides the right utility. Do not add new local unsafe here as a first step.

### F10 — row-pack kernel/block-size tuning is still an open profiling question

- **Status:** open question
- **Severity:** low
- **Type:** performance
- **Location:** `vcc_gpu_kernel.rs :: vcc_eval_block_x`, `launch_pack_rows_u64_ptr_f32`
- **Timing boundary affected:** profiling-only
- **Why it matters:** block sizes may not be ideal for all VCC combo counts, but there is no Nsight evidence in the provided files. Tuning them before fixing staging reuse would be premature.
- **Recommended change:** leave launch geometry alone until the cache/reuse work lands and you have a stable warm-path baseline.

---

## 3. Safe optimization opportunities

These are the changes I would do first.

### A. Persist VCC price-state device buffers

**Goal:** remove repeated H→D uploads of close/log-return arrays when the candle series is unchanged.

**Safe because:** it does not change strategy logic, only buffer lifetime.

**Implementation sketch:**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HostSliceKey {
    ptr: usize,
    len: usize,
}

impl HostSliceKey {
    fn of<T>(slice: &[T]) -> Self {
        Self {
            ptr: slice.as_ptr() as usize,
            len: slice.len(),
        }
    }
}

struct CachedPriceState {
    key: Option<HostSliceKey>,
    d_close: Option<DeviceBuffer<f32>>,
    d_lr: Option<DeviceBuffer<f32>>,
    d_lr64: Option<DeviceBuffer<f64>>,
}

impl CachedPriceState {
    fn ensure(
        &mut self,
        close_f64: &[f64],
    ) -> Result<(&DeviceBuffer<f32>, &DeviceBuffer<f32>, &DeviceBuffer<f64>), String> {
        let key = HostSliceKey::of(close_f64);
        let needs_refresh = self.key != Some(key);

        if needs_refresh {
            let (close_f32, lr_f32, lr_f64) = build_price_state_host(close_f64);

            match self.d_close.as_mut() {
                Some(buf) if buf.len() == close_f32.len() => {
                    buf.copy_from(close_f32.as_slice()).map_err(|e| e.to_string())?;
                }
                _ => {
                    self.d_close = Some(DeviceBuffer::from_slice(close_f32.as_slice()).map_err(|e| e.to_string())?);
                }
            }

            match self.d_lr.as_mut() {
                Some(buf) if buf.len() == lr_f32.len() => {
                    buf.copy_from(lr_f32.as_slice()).map_err(|e| e.to_string())?;
                }
                _ => {
                    self.d_lr = Some(DeviceBuffer::from_slice(lr_f32.as_slice()).map_err(|e| e.to_string())?);
                }
            }

            match self.d_lr64.as_mut() {
                Some(buf) if buf.len() == lr_f64.len() => {
                    buf.copy_from(lr_f64.as_slice()).map_err(|e| e.to_string())?;
                }
                _ => {
                    self.d_lr64 = Some(DeviceBuffer::from_slice(lr_f64.as_slice()).map_err(|e| e.to_string())?);
                }
            }

            self.key = Some(key);
        }

        Ok((
            self.d_close.as_ref().unwrap(),
            self.d_lr.as_ref().unwrap(),
            self.d_lr64.as_ref().unwrap(),
        ))
    }
}
```

**Notes:**

- This “last value wins” cache is enough for the first pass.
- If the surrounding call path can reuse the same host slice identity, this eliminates almost all repeated price-state uploads.
- If the same host allocation can be mutated in place, pointer/len is not sufficient. In that case, add a caller-supplied dataset identity later.

---

### B. Replace three host passes with one prefix-safe builder

**Goal:** avoid repeated zero-fill and extra passes before the first upload.

**Safe because:** every pushed element is initialized before read.

**Implementation sketch:**

```rust
#[inline]
fn build_price_state_host(close_f64: &[f64]) -> (Vec<f32>, Vec<f32>, Vec<f64>) {
    let n = close_f64.len();
    let mut close_out = Vec::with_capacity(n);
    let mut lr32_out = Vec::with_capacity(n);
    let mut lr64_out = Vec::with_capacity(n);

    if n == 0 {
        return (close_out, lr32_out, lr64_out);
    }

    close_out.push(close_f64[0] as f32);
    lr32_out.push(0.0);
    lr64_out.push(0.0);

    for t in 1..n {
        let p = close_f64[t];
        let pm = close_f64[t - 1];
        close_out.push(p as f32);

        let lr64 = if p.is_finite() && pm.is_finite() && p > 0.0 && pm > 0.0 {
            p.ln() - pm.ln()
        } else {
            0.0
        };

        lr64_out.push(lr64);
        lr32_out.push(lr64 as f32);
    }

    (close_out, lr32_out, lr64_out)
}
```

**Notes:**

- This replaces `close_f32` collection plus both `compute_log_returns_*` passes in the VCC GPU path.
- It is also a better fit for future `alloc_policy` prefix-init helpers if you already have them.

---

### C. Persist packed fast/trend matrices across repeated calls

**Goal:** avoid row-pointer upload and repack when the selected rows have not changed.

**Safe because:** it only changes buffer lifetime and reuse.

**Implementation sketch:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
enum PackedMatrixKey {
    DevicePtrs {
        rows: usize,
        cols: usize,
        row_ptrs: Vec<u64>,
    },
    HostRows {
        rows: usize,
        cols: usize,
        host: HostSliceKey,
    },
}

struct CachedPackedMatrix {
    key: Option<PackedMatrixKey>,
    d_row_ptrs: Option<DeviceBuffer<u64>>,
    d_matrix: Option<DeviceBuffer<f32>>,
}

impl CachedPackedMatrix {
    fn ensure_from_device_rows(
        &mut self,
        rt: &mut VccCudaRuntime,
        row_ptrs: &[u64],
        rows: usize,
        cols: usize,
    ) -> Result<&DeviceBuffer<f32>, String> {
        let key = PackedMatrixKey::DevicePtrs {
            rows,
            cols,
            row_ptrs: row_ptrs.to_vec(),
        };

        if self.key.as_ref() != Some(&key) {
            let total = rows
                .checked_mul(cols)
                .ok_or_else(|| "VCC packed matrix overflow".to_string())?;

            match self.d_row_ptrs.as_mut() {
                Some(buf) if buf.len() == row_ptrs.len() => {
                    buf.copy_from(row_ptrs).map_err(|e| e.to_string())?;
                }
                _ => {
                    self.d_row_ptrs = Some(DeviceBuffer::from_slice(row_ptrs).map_err(|e| e.to_string())?);
                }
            }

            match self.d_matrix.as_mut() {
                Some(buf) if buf.len() == total => {}
                _ => {
                    self.d_matrix = Some(
                        unsafe { DeviceBuffer::<f32>::uninitialized(total) }
                            .map_err(|e| e.to_string())?
                    );
                }
            }

            launch_pack_rows_u64_ptr_f32(
                rt,
                self.d_row_ptrs.as_ref().unwrap(),
                rows,
                cols,
                self.d_matrix.as_mut().unwrap(),
            )?;

            self.key = Some(key);
        }

        Ok(self.d_matrix.as_ref().unwrap())
    }

    fn ensure_from_host_rows(
        &mut self,
        host_rows: &[f32],
        rows: usize,
        cols: usize,
    ) -> Result<&DeviceBuffer<f32>, String> {
        let key = PackedMatrixKey::HostRows {
            rows,
            cols,
            host: HostSliceKey::of(host_rows),
        };

        if self.key.as_ref() != Some(&key) {
            match self.d_matrix.as_mut() {
                Some(buf) if buf.len() == host_rows.len() => {
                    buf.copy_from(host_rows).map_err(|e| e.to_string())?;
                }
                _ => {
                    self.d_matrix = Some(DeviceBuffer::from_slice(host_rows).map_err(|e| e.to_string())?);
                }
            }

            self.key = Some(key);
        }

        Ok(self.d_matrix.as_ref().unwrap())
    }
}
```

**Notes:**

- This is the main staging win after price-state caching.
- It is especially valuable for repeated validation or optimizer calls with the same set of rows.
- Because the device-row key stores the exact row-pointer list, there is no hash-collision risk.

---

### D. Reuse combo scratch buffers and host readback buffers

**Goal:** avoid allocating `DeviceBuffer` and host `Vec` outputs on every call.

**Safe because:** the buffers are still fully written before any read.

**Implementation sketch:**

```rust
struct VccComboScratch {
    combo_len: usize,
    out_stride: usize,
    d_fast_idx: Option<DeviceBuffer<i32>>,
    d_trend_idx: Option<DeviceBuffer<i32>>,
    d_start_t: Option<DeviceBuffer<i32>>,
    d_metric_start_t: Option<DeviceBuffer<i32>>,
    d_end_t: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    d_slice_out: Option<DeviceBuffer<f64>>,
    host_out: Vec<f32>,
    host_slice_out: Vec<f64>,
}

impl VccComboScratch {
    fn ensure_shape(&mut self, combo_len: usize, out_stride: usize) -> Result<(), String> {
        let out_len = combo_len
            .checked_mul(out_stride)
            .ok_or_else(|| "VCC out buffer overflow".to_string())?;
        let slice_len = combo_len
            .checked_mul(2)
            .ok_or_else(|| "VCC slice-out buffer overflow".to_string())?;

        macro_rules! ensure_dev_buf {
            ($slot:expr, $ty:ty, $len:expr) => {
                if !matches!($slot.as_ref(), Some(buf) if buf.len() == $len) {
                    $slot = Some(
                        unsafe { DeviceBuffer::<$ty>::uninitialized($len) }
                            .map_err(|e| e.to_string())?
                    );
                }
            };
        }

        ensure_dev_buf!(self.d_fast_idx, i32, combo_len);
        ensure_dev_buf!(self.d_trend_idx, i32, combo_len);
        ensure_dev_buf!(self.d_start_t, i32, combo_len);
        ensure_dev_buf!(self.d_metric_start_t, i32, combo_len);
        ensure_dev_buf!(self.d_end_t, i32, combo_len);
        ensure_dev_buf!(self.d_out, f32, out_len);
        ensure_dev_buf!(self.d_slice_out, f64, slice_len);

        if self.host_out.len() != out_len {
            self.host_out.resize(out_len, 0.0);
        }
        if self.host_slice_out.len() != slice_len {
            self.host_slice_out.resize(slice_len, 0.0);
        }

        self.combo_len = combo_len;
        self.out_stride = out_stride;
        Ok(())
    }

    fn upload(&mut self, combos: &VccGpuComboBatch<'_>) -> Result<(), String> {
        self.d_fast_idx
            .as_mut()
            .unwrap()
            .copy_from(combos.fast_row_index)
            .map_err(|e| e.to_string())?;
        self.d_trend_idx
            .as_mut()
            .unwrap()
            .copy_from(combos.trend_row_index)
            .map_err(|e| e.to_string())?;
        self.d_start_t
            .as_mut()
            .unwrap()
            .copy_from(combos.start_t)
            .map_err(|e| e.to_string())?;
        self.d_metric_start_t
            .as_mut()
            .unwrap()
            .copy_from(combos.metric_start_t)
            .map_err(|e| e.to_string())?;
        self.d_end_t
            .as_mut()
            .unwrap()
            .copy_from(combos.end_t)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
```

**Extra micro-optimization:**

If `combos.start_t` and `combos.metric_start_t` point to the same slice, you can reuse the same device buffer pointer for both kernel args and skip one upload. Many of the VCC tests do exactly that.

---

### E. Keep public VCC APIs stable, but split internal route/fallback helpers

**Goal:** make fallback visible without forcing a broad public API break.

**Safe because:** existing callers can keep the old return types.

**Implementation sketch:**

```rust
#[cfg(feature = "cuda")]
#[derive(Clone, Debug, Default)]
pub struct VccCudaRouteMeta {
    pub attempted_gpu: bool,
    pub used_gpu: bool,
    pub used_device_inputs: bool,
    pub fallback_reason: Option<String>,
}

#[cfg(feature = "cuda")]
pub struct VccPeriodSweepWithRoute {
    pub sweep: VccPeriodSweepDevice,
    pub route: VccCudaRouteMeta,
}
```

Then convert the private direct helpers to return route metadata:

```rust
#[cfg(feature = "cuda")]
fn compute_vcc_ttm_squeeze_sweep_cuda_direct_with_route(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VccStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VccPeriodSweepWithRoute, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let sweep = TtmSqueezeBatchRange {
        length: (usize::from(start), usize::from(end), usize::from(step)),
        bb_mult: (cfg.squeeze_bb_mult, cfg.squeeze_bb_mult, 0.0),
        kc_high: (1.0, 1.0, 0.0),
        kc_mid: (cfg.squeeze_kc_mult, cfg.squeeze_kc_mult, 0.0),
        kc_low: (2.0, 2.0, 0.0),
    };

    let wrapper = CudaTtmSqueeze::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.hlc_first_valid();

    match wrapper.ttm_squeeze_batch_dev_from_device_inputs(
        prepared_inputs.high_device().buffer(),
        prepared_inputs.low_device().buffer(),
        prepared_inputs.close_device().buffer(),
        prepared_inputs.len(),
        first_valid,
        &sweep,
    ) {
        Ok((_momentum, squeeze)) => Ok(VccPeriodSweepWithRoute {
            sweep: VccPeriodSweepDevice {
                matrix: VccOwnedDeviceMatrix::from_raw(squeeze),
                period_start: start,
                period_end: end,
                period_step: step,
            },
            route: VccCudaRouteMeta {
                attempted_gpu: true,
                used_gpu: true,
                used_device_inputs: true,
                fallback_reason: None,
            },
        }),
        Err(device_err) => {
            let (_momentum, squeeze) = wrapper
                .ttm_squeeze_batch_dev(
                    prepared_inputs.high(),
                    prepared_inputs.low(),
                    prepared_inputs.close(),
                    &sweep,
                )
                .map_err(|host_err| {
                    format!(
                        "ttm_squeeze device-input path failed: {}; host-input fallback also failed: {}",
                        device_err,
                        host_err
                    )
                })?;

            Ok(VccPeriodSweepWithRoute {
                sweep: VccPeriodSweepDevice {
                    matrix: VccOwnedDeviceMatrix::from_raw(squeeze),
                    period_start: start,
                    period_end: end,
                    period_step: step,
                },
                route: VccCudaRouteMeta {
                    attempted_gpu: true,
                    used_gpu: true,
                    used_device_inputs: false,
                    fallback_reason: Some(device_err.to_string()),
                },
            })
        }
    }
}
```

Then keep the existing public helper:

```rust
#[cfg(feature = "cuda")]
pub fn compute_vcc_ttm_squeeze_sweep_cuda_device(...) -> Result<VccPeriodSweepDevice, String> {
    compute_vcc_ttm_squeeze_sweep_cuda_direct_with_route(...).map(|x| x.sweep)
}
```

This gives you a parity-safe route transparency layer without breaking existing callers.

---

### F. Replace fixed-size param `Vec`s with stack arrays

**Goal:** remove tiny heap allocations from reference/parity helpers.

**Safe because:** the parameter counts are static in these helpers.

**Implementation sketch:**

```rust
pub fn compute_vcc_fast_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &VccStrategyParams,
    squeeze_period: u16,
    momentum_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<VccFastSignals, String> {
    let squeeze_params = [
        IndicatorParamKV {
            key: "length".to_string(),
            value: IndicatorParamValue::Int(i64::from(squeeze_period.max(1))),
        },
        IndicatorParamKV {
            key: "bb_mult".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
    ];

    let momentum_params = [
        IndicatorParamKV {
            key: "length_bb".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.momentum_bb_period.max(1))),
        },
        IndicatorParamKV {
            key: "mult_bb".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
        IndicatorParamKV {
            key: "length_kc".to_string(),
            value: IndicatorParamValue::Int(i64::from(momentum_period.max(1))),
        },
        IndicatorParamKV {
            key: "mult_kc".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_kc_mult),
        },
    ];

    let squeeze_state = compute_indicator_f64_with_warmup(
        candles,
        ma_source,
        backend,
        "ttm_squeeze",
        "squeeze",
        squeeze_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let momentum = compute_indicator_f64_with_warmup(
        candles,
        ma_source,
        backend,
        "squeeze_momentum",
        "momentum",
        momentum_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    // existing logic...
}
```

Do the same for `compute_vcc_trend_signals`.

---

### G. Use prefix-init for `desired` only if `alloc_policy.rs` already has the right helper

**Goal:** avoid zero-filling the tail of `desired` when `[start_t..n)` is always overwritten.

**Safe only if:** you already have repo-standard helpers for uninitialized vectors and prefix initialization.

**Reasoning:**

`build_vcc_desired_positions` currently does:

```rust
let mut desired = vec![0i32; n];
```

But the behavior only requires:

- `[0..start_t)` = `0`
- `[start_t..n)` = written exactly once in the main loop

So the full zero-fill is not strictly necessary.

**Recommended approach:**

- If `alloc_policy.rs` already has `MaybeUninit` helpers for `i32` vectors, use them.
- If not, leave this function alone in the first pass.
- Do **not** add local one-off unsafe here before the larger GPU-path fixes land.

**Illustrative shape only:**

```rust
// Pseudocode only — use alloc_policy helpers if they already exist in the repo.
let mut desired = alloc_policy::make_uninit_i32_vec(n);
alloc_policy::init_prefix_i32(&mut desired, 0..start_t.min(n), 0);

for t in start_t..n {
    let out_pos = if strategy.long_only && pos < 0 { 0 } else { pos };
    alloc_policy::write_i32(&mut desired, t, out_pos);
}

let desired = unsafe { alloc_policy::assume_init_i32_vec(desired) };
```

---

## 4. Higher-risk optimization opportunities

These can be worthwhile, but only after the safe staging/cache work and benchmark additions are done.

### H1. Teach the kernel to consume row remap tables directly

**Idea:** instead of packing selected rows into contiguous scratch matrices, pass:

- base device matrix pointer
- stride
- row remap tables

Then the kernel resolves `base + remap[idx] * stride`.

**Why it is attractive:**

- removes three pack kernels
- removes three packed matrix scratch buffers
- reduces first-call staging cost further

**Why it is higher risk:**

- changes kernel ABI
- can hurt memory coalescing if row selection is sparse or irregular
- requires more careful benchmarking than the cache-only path

**Recommendation:** only consider this if the warm-path cache is not enough and first-call staging still dominates your real workload.

---

### H2. Accept prepared device price-state instead of host close slices

**Idea:** extend the VCC GPU metric API to accept an optional prepared device price-state:

```rust
pub struct VccPreparedPriceState {
    pub close_ptr: u64,
    pub lr_f32_ptr: u64,
    pub lr_f64_ptr: u64,
    pub len: usize,
}
```

Then `eval_vcc_metrics_gpu_kernel_device_rows_detailed` can bypass host price conversion/upload entirely when the caller already owns resident device data.

**Why it is attractive:**

- aligns with the device-input design already used by the VCC sweep helpers
- removes even the first-call host price upload for callers that already prepared device candles

**Why it is higher risk:**

- widens the public API
- requires more integration work in adjacent call sites that were not provided here

**Recommendation:** make this a phase-3 improvement, not the first patch.

---

### H3. Fuse or narrow fast-signal sweep APIs only after parity is locked down

**Idea:** reduce unused byproducts across:
- `ttm_squeeze`
- `squeeze_momentum`

**Why it is attractive:**
- VCC currently calls separate sweep paths and only keeps a subset of the generated outputs.

**Why it is higher risk:**
- your diagnostics already suggest that implementation details around squeeze/momentum parity matter
- a superficially “obvious” fusion can create subtle parity drift

**Recommendation:** do not attempt this until:
1. the staging cache exists,
2. VCC has optimizer Criterion coverage,
3. one VCC GPU parity test is no longer ignored.

---

## 5. Benchmark and perf-gate review

## Current benchmark quality

### What is good already

- `gpu_native_expansion_opt.rs` already has a strong route-probe and route-assert pattern for GPU-native optimizer benches.
- `composite_validation_workflow.rs` already prints route metadata.
- Both files use realistic candle counts rather than toy inputs.

### What is missing for VCC

1. **No VCC optimizer Criterion groups exist.**
2. **GPU validation route is not asserted in the generic VCC path.**
3. **VCC does not have a dedicated heavier GPU validation group.**
4. **The current VCC candidate generator repeats a narrow period set for large `n`, which can hide row-pack costs.**
5. **The current generic synthetic candles are fine for broad validation, but not ideal for strategy-specific VCC stress.**

## Recommended benchmark changes

### `gpu_native_expansion_opt.rs`

Add:

- `make_vcc_breakout_candles(len)`
- `make_vcc_req(backend, fast_range, slow_range)`
- `bench_vcc_cpu`
- `bench_vcc_gpu_kernel`

### `composite_validation_workflow.rs`

Add:

- route assertions for all GPU-labeled VCC benches
- `make_vcc_validation_candles(len)` for dedicated VCC groups
- `make_vcc_candidates_wide(n)` for heavy VCC candidate sets
- a dedicated VCC GPU validation group similar in shape to the existing ASR heavy group
- at least one larger VCC profile case such as `100k / 32 candidates`

## Important timing-boundary note

The route-probe call outside the Criterion measurement already warms the runtime. That is actually useful here because the primary VCC work item is a **warm-path staging reuse** improvement. Do not compare route-probe timing to benchmark timing as if they represent the same boundary.

If you later want cold-start evidence, add a separate named bench or profiling script. Do not contaminate the existing warm-path Criterion groups.

---

## 6. Test review

## What the current tests already tell you

`vcc.rs` already contains useful diagnostics for:

- CPU vs CUDA host-series differences
- device sweeps vs CPU series
- multi-row sweep parity
- short-combo parity
- validation-batch parity
- optimizer-shape parity
- momentum roundtrip stability

That is good diagnostic coverage.

## What is still missing

### T1. a non-ignored momentum roundtrip gate

Add a small test that asserts a known VCC fixture is FP32-stable.

Example shape:

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[test]
fn vcc_momentum_roundtrip_fixture_is_fp32_stable() {
    let candles = make_trending_candles(4096);
    let cfg = VccStrategyParams::default();
    let prepared =
        PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();

    let fast = compute_vcc_fast_signals(
        &candles,
        "close",
        &Backend::CpuOnly,
        &cfg,
        8,
        8,
        Some(&prepared),
    )
    .unwrap();

    let parity = vcc_momentum_roundtrip_parity(&fast.momentum);
    assert_eq!(parity.sign_mismatch_count, 0);
    assert_eq!(parity.delta_sign_mismatch_count, 0);
}
```

If this exact fixture is not stable today, keep the diagnostic but choose a measured stable fixture and gate that one.

### T2. a small GPU metric parity test

Add a non-ignored batch test with one or two combos and explicit tolerances.

Use the same fixture family as your existing diagnostics, but turn one into a real assertion.

Example shape:

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[test]
fn vcc_gpu_metrics_match_cpu_small_batch() {
    // Build the same small-batch setup as the existing diagnostic.
    // Then assert trades exactly and pnl within measured tolerance.

    let cpu_metrics = /* existing CPU path */;
    let gpu_metrics = /* existing GPU path */;

    assert_eq!(cpu_metrics.trades, gpu_metrics[0].trades);
    assert!(
        (cpu_metrics.pnl - gpu_metrics[0].pnl).abs() <= VCC_GPU_PNL_TOLERANCE,
        "cpu pnl={} gpu pnl={}",
        cpu_metrics.pnl,
        gpu_metrics[0].pnl
    );
}
```

Use a tolerance derived from the current diagnostic baseline, not an invented number.

### T3. a staging-cache reuse test

Once the cache exists, add a test that calls the VCC GPU metric function twice with the same inputs and asserts identical results. If you expose cache hit counters behind `cfg(test)`, you can also assert reuse happened.

Do not make cache-hit counters public in production unless you need them.

---

## 7. Suggested improvement plan

## Phase 1 — add warm-path staging reuse in `vcc_gpu_kernel.rs`

**Why first:** this is the biggest confirmed app-path waste and does not need a strategy rewrite.

Steps:

1. add `CachedPriceState`
2. add `CachedPackedMatrix` for:
   - fast squeeze
   - fast momentum
   - trend
3. add `VccComboScratch`
4. extend `VccCudaRuntime` to own those caches
5. replace per-call buffer creation in `eval_vcc_metrics_gpu_kernel_device_rows_detailed` with `ensure_*` methods
6. keep kernel ABI unchanged
7. validate parity using the existing diagnostics before moving on

## Phase 2 — make VCC route/fallback visible in `vcc.rs`

**Why second:** VCC benchmarking is much more useful once you can tell whether the intended route was actually used.

Steps:

1. add `VccCudaRouteMeta`
2. split private direct sweep helpers into:
   - device-input attempt
   - host-input fallback
3. capture fallback reason strings instead of dropping them
4. optionally add route metadata to the private `IndicatorSeriesF64`
5. keep existing public helpers as compatibility wrappers that discard metadata

## Phase 3 — add VCC optimizer Criterion groups

**Why third:** you need optimizer evidence before claiming CPU/GPU payoff.

Steps:

1. add `make_vcc_breakout_candles`
2. add `make_vcc_req`
3. add `bench_vcc_cpu`
4. add `bench_vcc_gpu_kernel`
5. add both to `criterion_group!`

## Phase 4 — strengthen VCC validation-cost coverage

**Why fourth:** validation throughput is explicitly in scope for this bundle.

Steps:

1. assert GPU route in GPU-labeled validation groups
2. add `make_vcc_validation_candles`
3. add `make_vcc_candidates_wide`
4. add one dedicated heavier VCC GPU validation group
5. add a larger VCC profile case in both CPU and GPU profiles

## Phase 5 — only then consider higher-risk kernel/interface work

Possible phase-5 items:

- direct row-remap kernel path
- prepared device price-state input
- fast-signal API fusion/narrowing
- launch geometry retuning

---

## 8. File-by-file implementation plan and code snippets

## 8.1 `crates/app_core/src/vcc_gpu_kernel.rs`

### Goals

- remove repeated H→D price uploads
- remove repeated row packing when selected rows are unchanged
- reuse combo scratch and outputs
- reduce host-side zero-fill
- keep kernel ABI unchanged in phase 1

### Recommended struct edits

Add cache fields to `VccCudaRuntime`:

```rust
struct VccCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    price_state: CachedPriceState,
    fast_squeeze_cache: CachedPackedMatrix,
    fast_momentum_cache: CachedPackedMatrix,
    trend_cache: CachedPackedMatrix,
    combo_scratch: VccComboScratch,
}
```

Update `new`:

```rust
Ok(Self {
    context,
    module,
    stream,
    price_state: CachedPriceState {
        key: None,
        d_close: None,
        d_lr: None,
        d_lr64: None,
    },
    fast_squeeze_cache: CachedPackedMatrix {
        key: None,
        d_row_ptrs: None,
        d_matrix: None,
    },
    fast_momentum_cache: CachedPackedMatrix {
        key: None,
        d_row_ptrs: None,
        d_matrix: None,
    },
    trend_cache: CachedPackedMatrix {
        key: None,
        d_row_ptrs: None,
        d_matrix: None,
    },
    combo_scratch: VccComboScratch {
        combo_len: 0,
        out_stride: 0,
        d_fast_idx: None,
        d_trend_idx: None,
        d_start_t: None,
        d_metric_start_t: None,
        d_end_t: None,
        d_out: None,
        d_slice_out: None,
        host_out: Vec::new(),
        host_slice_out: Vec::new(),
    },
})
```

### Replace the current eval body with cache-aware flow

Core shape:

```rust
pub fn eval_vcc_metrics_gpu_kernel_device_rows_detailed(
    device_id: u32,
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcc: &VccStrategyParams,
) -> Result<Vec<VccGpuMetricEval>, String> {
    validate_inputs_device_rows(signals, combos)?;
    let combo_count = combos.fast_row_index.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }

    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "VCC GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_stride = NON_DOUBLE_MA_GPU_METRICS_STRIDE;

    VCC_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            let rt = VccCudaRuntime::new(device_id)?;
            runtimes.insert(device_id, rt);
        }

        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "VCC GPU runtime missing".to_string())?;

        rt.set_current()?;

        let (d_close, d_lr, d_lr64) = rt.price_state.ensure(signals.close_f64)?;

        let d_fast_sq = match signals.fast_squeeze_host_row_major {
            Some(host_rows) => rt
                .fast_squeeze_cache
                .ensure_from_host_rows(host_rows, signals.fast_rows, signals.cols)?,
            None => rt.fast_squeeze_cache.ensure_from_device_rows(
                rt,
                signals.fast_squeeze_rows,
                signals.fast_rows,
                signals.cols,
            )?,
        };

        let d_fast_mom = match signals.fast_momentum_host_row_major {
            Some(host_rows) => rt
                .fast_momentum_cache
                .ensure_from_host_rows(host_rows, signals.fast_rows, signals.cols)?,
            None => rt.fast_momentum_cache.ensure_from_device_rows(
                rt,
                signals.fast_momentum_rows,
                signals.fast_rows,
                signals.cols,
            )?,
        };

        let d_trend = match signals.trend_host_row_major {
            Some(host_rows) => rt
                .trend_cache
                .ensure_from_host_rows(host_rows, signals.trend_rows, signals.cols)?,
            None => rt.trend_cache.ensure_from_device_rows(
                rt,
                signals.trend_rows_ptr,
                signals.trend_rows,
                signals.cols,
            )?,
        };

        rt.combo_scratch.ensure_shape(combo_count, out_stride)?;
        rt.combo_scratch.upload(combos)?;

        // Use cached device buffers from rt.combo_scratch here.
        // Launch kernel unchanged.
        // Synchronize once.
        // Copy into rt.combo_scratch.host_out / host_slice_out.
        // Decode into Vec<VccGpuMetricEval>.
    })
}
```

### Tighten scalar conversion safety

Right now several values are cast to `i32` with `as`. Use checked conversions for consistency with `launch_pack_rows_u64_ptr_f32`:

```rust
let mut t_i =
    i32::try_from(signals.cols).map_err(|_| "VCC GPU cols exceed i32 range".to_string())?;
let mut fast_rows_i =
    i32::try_from(signals.fast_rows).map_err(|_| "VCC GPU fast rows exceed i32 range".to_string())?;
let mut fast_stride_i =
    i32::try_from(signals.cols).map_err(|_| "VCC GPU fast stride exceeds i32 range".to_string())?;
let mut trend_rows_i =
    i32::try_from(signals.trend_rows).map_err(|_| "VCC GPU trend rows exceed i32 range".to_string())?;
let mut trend_stride_i =
    i32::try_from(signals.cols).map_err(|_| "VCC GPU trend stride exceeds i32 range".to_string())?;
let mut combo_count_i =
    i32::try_from(combo_count).map_err(|_| "VCC GPU combo count exceeds i32 range".to_string())?;
let mut out_stride_i =
    i32::try_from(out_stride).map_err(|_| "VCC GPU out stride exceeds i32 range".to_string())?;
```

This is not the main performance change, but it is a worthwhile correctness hardening while you are already touching the launch code.

### Optional micro-optimization for repeated start vectors

If the caller commonly sets `start_t == metric_start_t`, reuse the same buffer:

```rust
let same_start = std::ptr::eq(combos.start_t.as_ptr(), combos.metric_start_t.as_ptr())
    && combos.start_t.len() == combos.metric_start_t.len();

if same_start {
    self.d_start_t
        .as_mut()
        .unwrap()
        .copy_from(combos.start_t)
        .map_err(|e| e.to_string())?;
    // Skip d_metric_start_t upload and pass the same ptr to both kernel args.
} else {
    // Upload both.
}
```

This is optional, but cheap and parity-safe.

---

## 8.2 `crates/app_core/src/vcc.rs`

### Goals

- preserve current public behavior
- surface route/fallback information
- remove avoidable tiny heap allocations
- add one or two real parity gates
- optionally use prefix-init for `desired` if shared helpers already exist

### Route-metadata addition

A minimal internal route metadata layer is enough for phase 2.

```rust
#[cfg(feature = "cuda")]
#[derive(Clone, Debug, Default)]
pub struct VccCudaRouteMeta {
    pub attempted_gpu: bool,
    pub used_gpu: bool,
    pub used_device_inputs: bool,
    pub fallback_reason: Option<String>,
}
```

Then add it to the private series wrapper:

```rust
struct IndicatorSeriesF64 {
    values: Vec<f64>,
    warmup: Option<usize>,
    #[cfg(feature = "cuda")]
    route: Option<VccCudaRouteMeta>,
}
```

### Capture fallback reason in `compute_indicator_f64_with_warmup`

```rust
fn compute_indicator_f64_with_warmup(
    candles: &Candles,
    ma_source: &str,
    _backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<IndicatorSeriesF64, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("VCC indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    #[cfg(feature = "cuda")]
    {
        let prefer_gpu = backend_prefers_gpu(_backend)
            && descriptor.prefers_gpu_by_speedup
            && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);

        let mut route = None;

        if prefer_gpu {
            match prepared_cuda_inputs {
                Some(prepared) => {
                    match compute_dynamic_indicator_cuda_with_inputs(
                        prepared,
                        indicator_id,
                        Some(output_id),
                        params,
                        Kernel::Auto,
                        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                    ) {
                        Ok(values) => {
                            let warmup = values.warmup;
                            return Ok(IndicatorSeriesF64 {
                                values: cuda_series_to_f64(values)?,
                                warmup,
                                route: Some(VccCudaRouteMeta {
                                    attempted_gpu: true,
                                    used_gpu: true,
                                    used_device_inputs: true,
                                    fallback_reason: None,
                                }),
                            });
                        }
                        Err(err) => {
                            route = Some(VccCudaRouteMeta {
                                attempted_gpu: true,
                                used_gpu: false,
                                used_device_inputs: true,
                                fallback_reason: Some(err.to_string()),
                            });
                        }
                    }
                }
                None => {
                    route = Some(VccCudaRouteMeta {
                        attempted_gpu: true,
                        used_gpu: false,
                        used_device_inputs: false,
                        fallback_reason: Some("prepared CUDA inputs missing".to_string()),
                    });
                }
            }
        }

        let cpu = compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            indicator_id,
            Some(output_id),
            params,
            Kernel::Auto,
        )
        .map_err(|e| e.to_string())?;

        return Ok(IndicatorSeriesF64 {
            values: series_to_f64(cpu),
            warmup: cpu.warmup,
            route,
        });
    }

    let cpu = compute_dynamic_indicator_cpu(
        candles,
        ma_source,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
    )
    .map_err(|e| e.to_string())?;

    Ok(IndicatorSeriesF64 {
        values: series_to_f64(cpu),
        warmup: cpu.warmup,
    })
}
```

If you do not want `route` to live inside `IndicatorSeriesF64`, return a tuple instead. The key point is to stop dropping fallback details.

### Keep public API stable with compatibility wrappers

Private helpers can return metadata while public wrappers keep the old signatures:

```rust
#[cfg(feature = "cuda")]
pub fn compute_vcc_ttm_trend_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VccPeriodSweepDevice, String> {
    compute_vcc_ttm_trend_sweep_cuda_direct_with_route(
        prepared_inputs,
        period_start,
        period_end,
        period_step,
        device_id,
    )
    .map(|x| x.sweep)
}
```

### Fixed-size param arrays

Apply the stack-array change to:

- `compute_vcc_fast_signals`
- `compute_vcc_trend_signals`

This is a small safe cleanup and it reduces tiny heap churn in CPU/reference paths.

### Optional prefix-init in `build_vcc_desired_positions`

This is lower priority than the GPU-path changes, but it is a legitimate copy-elision opportunity.

Do it only if `alloc_policy.rs` already has a standard helper for prefix-init and `assume_init`. Otherwise leave the current `vec![0i32; n]`.

### Test plan inside `vcc.rs`

Convert one diagnostic into a real gate. My recommendation:

1. keep the long ignored diagnostics
2. add **one** non-ignored FP32 roundtrip gate
3. add **one** non-ignored small GPU metric parity gate

That gives you a minimal regression net before you touch the warm path.

---

## 8.3 `crates/app_core/benches/gpu_native_expansion_opt.rs`

### Goals

- add actual optimizer Criterion evidence for VCC
- reuse the existing route-probe/assert pattern
- use a VCC-specific candle shape that creates compression/release/trend regimes

### Import addition

Add `VccStrategyParams` to the top-level imports.

```rust
use ta_app_core::{
    Backend, BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeCpuRequest,
    CompositeSignalMode, KqsrStrategyParams, KvdfStrategyParams, VccStrategyParams,
};
```

### Add a VCC candle generator

Use alternating low-vol / breakout / low-vol / breakdown regimes so VCC logic is actually exercised.

```rust
fn make_vcc_breakout_candles(len: usize) -> Candles {
    let len = len.max(320);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let mut px = 100.0f64;
    for i in 0..len {
        ts.push(i as i64);

        let regime = (i / 96) % 4;
        let (drift, wave_amp, span) = match regime {
            0 => (0.015, 0.08, 0.22),  // compressed up drift
            1 => (0.17, 0.45, 1.10),   // breakout / expansion
            2 => (-0.015, 0.08, 0.22), // compressed down drift
            _ => (-0.18, 0.48, 1.15),  // breakdown / expansion
        };

        let wave = (i as f64 * 0.12).sin() * wave_amp
            + (i as f64 * 0.035).cos() * wave_amp * 0.35;

        px += drift + wave * 0.08;

        open.push(px - drift * 0.25);
        high.push(px + span);
        low.push(px - span);
        close.push(px + wave * 0.02);
        vol.push(1_100.0 + (i % 23) as f64 * 31.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

### Add `make_vcc_req`

```rust
fn make_vcc_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.vcc = Some(VccStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}
```

### Add VCC bench groups

```rust
fn bench_vcc_cpu(c: &mut Criterion) {
    if !bench_target_enabled("vcc") {
        return;
    }
    let candles = make_vcc_breakout_candles(200_000);
    bench_cpu_cases(c, "vcc/cpu/200k", &candles, make_vcc_req);
}

fn bench_vcc_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("vcc") {
        return;
    }
    let candles = make_vcc_breakout_candles(200_000);
    bench_gpu_cases(
        c,
        "vcc/gpu_kernel/200k",
        "vcc/gpu_kernel/200k",
        &candles,
        make_vcc_req,
    );
}
```

Then add them to `criterion_group!`:

```rust
criterion_group!(
    gpu_native_expansion_benches,
    bench_vcc_cpu,
    bench_vcc_gpu_kernel,
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

### What not to change yet

- do not add a brand-new benchmark framework
- do not add synthetic cold-start timing to this file yet
- do not change the route-probe/assert pattern; it is already good

---

## 8.4 `crates/app_core/benches/composite_validation_workflow.rs`

### Goals

- assert the actual GPU route for VCC-labeled validation groups
- add heavier VCC validation cases
- use more VCC-appropriate candle data in dedicated VCC groups
- increase period diversity for larger VCC candidate sets

### Import addition

Add `BackendUsed` if you want a shorter assertion.

```rust
use ta_app_core::{
    BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeParams, CompositeValidationRequest,
    CscvPboConfig, PerturbConfig, TradeBootstrapConfig, WalkForwardConfigOptional,
};
```

### Add a small GPU-route assertion helper

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn assert_gpu_route(
    label: &str,
    backend_used: BackendUsed,
    backend_route: &Option<String>,
    backend_fallback_reason: &Option<String>,
) {
    assert!(
        matches!(backend_used, BackendUsed::Gpu),
        "expected GPU backend for {}, got backend={:?}, route={:?}, fallback={:?}",
        label,
        backend_used,
        backend_route,
        backend_fallback_reason
    );
}
```

Use it after route probes and inside benchmark iterations.

### Add a VCC-specific validation candle generator

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_vcc_validation_candles(len: usize) -> Candles {
    let len = len.max(8_192);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let mut px = 100.0f64;
    for i in 0..len {
        ts.push(i as i64);

        let regime = (i / 128) % 4;
        let (drift, noise, span) = match regime {
            0 => (0.010, 0.05, 0.18),
            1 => (0.140, 0.35, 1.00),
            2 => (-0.010, 0.05, 0.18),
            _ => (-0.145, 0.38, 1.05),
        };

        let wave = (i as f64 * 0.09).sin() * noise
            + (i as f64 * 0.027).cos() * noise * 0.4;

        px += drift + wave * 0.06;

        open.push(px - drift * 0.2);
        high.push(px + span);
        low.push(px - span);
        close.push(px + wave * 0.02);
        vol.push(1_000.0 + (i % 17) as f64 * 19.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

### Add a wider VCC candidate generator for heavier cases

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_vcc_candidates_wide(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 6u16.saturating_add(i_u16 % 40);
        let slow_len = fast_len.saturating_add(18u16 + ((i_u16 / 2) % 36));

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
            vcc: Some(ta_app_core::VccStrategyParams::default()),
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

This keeps the strategy config stable but widens period diversity.

### Tighten the existing GPU profile closure

Inside the existing `bench_profile` closure, change the route probe block to assert GPU usage:

```rust
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
    route_probe.backend_fallback_reason.as_deref().unwrap_or("<none>")
);

assert_gpu_route(
    &format!("composite_validate/gpu_kernel/{name}/{candles_len}/{candidate_count}candidates"),
    route_probe.backend_used,
    &route_probe.backend_route,
    &route_probe.backend_fallback_reason,
);
```

And inside the `group.bench_function` body:

```rust
group.bench_function("wf_reopt_cscv_spa", |b| {
    b.iter(|| {
        let res = ta_app_core::run_composite_validation_blocking_with_candles(
            base.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu profile wf+reopt+cscv+spa validate");

        assert_gpu_route(
            &format!("composite_validate/gpu_kernel/{name}/{candles_len}/{candidate_count}candidates"),
            res.backend_used,
            &res.backend_route,
            &res.backend_fallback_reason,
        );
    })
});
```

### Add a dedicated heavy VCC GPU validation group

Mirror the ASR heavy-group pattern but use VCC-specific candles/candidates:

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_vcc_validation_gpu_workflow(c: &mut Criterion) {
    let candles = make_vcc_validation_candles(100_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_vcc_candidates_wide(64);

    let mut group = c.benchmark_group("composite_validate/gpu_kernel/vcc/100k/64candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let base = CompositeValidationRequest {
        data_id: "bench-gpu-vcc".to_string(),
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

    let probe = ta_app_core::run_composite_validation_blocking_with_candles(
        base.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("vcc gpu validation route probe");

    println!(
        "benchmark route composite_validate/gpu_kernel/vcc/100k/64candidates: backend_used={:?} backend_route={} fallback={}",
        probe.backend_used,
        probe.backend_route.as_deref().unwrap_or("<none>"),
        probe.backend_fallback_reason.as_deref().unwrap_or("<none>")
    );

    assert_gpu_route(
        "composite_validate/gpu_kernel/vcc/100k/64candidates",
        probe.backend_used,
        &probe.backend_route,
        &probe.backend_fallback_reason,
    );

    group.bench_function("wf_cscv", |b| {
        b.iter(|| {
            let res = ta_app_core::run_composite_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("vcc gpu wf+cscv validate");

            assert_gpu_route(
                "composite_validate/gpu_kernel/vcc/100k/64candidates",
                res.backend_used,
                &res.backend_route,
                &res.backend_fallback_reason,
            );
        })
    });

    let mut vram_heavy = base.clone();
    vram_heavy.candidates = make_vcc_candidates_wide(192);
    vram_heavy.max_candidates = vram_heavy.candidates.len();
    vram_heavy.walk_forward = None;
    vram_heavy.cscv_pbo = None;

    group.bench_function("in_sample_192candidates", |b| {
        b.iter(|| {
            let res = ta_app_core::run_composite_validation_blocking_with_candles(
                vram_heavy.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("vcc gpu in-sample validate");

            assert_gpu_route(
                "composite_validate/gpu_kernel/vcc/100k/192candidates",
                res.backend_used,
                &res.backend_route,
                &res.backend_fallback_reason,
            );
        })
    });

    group.finish();
}
```

Then add it to the `criterion_group!` block alongside the existing workflow benches.

### Add a larger VCC profile case

Also add:

```rust
bench_cpu_profile("vcc", 100_000, make_vcc_candidates_wide(32));
```

and in the GPU profile section:

```rust
bench_profile("vcc", 100_000, make_vcc_candidates_wide(32));
```

This makes VCC comparable to the larger CARM/CDMC profile cases.

---

## 9. Uninitialized-memory and copy-elision policy for this patch set

This section is intentionally explicit because you asked to minimize unnecessary copies and use uninitialized memory where appropriate.

## Safe places to use uninitialized allocation

### 1. Device packed matrices written by pack kernels

Safe when:

- the buffer length is exact
- the pack kernel writes every element
- no read happens before the kernel completes

Examples:

- `d_fast_sq` when built from device row pointers
- `d_fast_mom` when built from device row pointers
- `d_trend` when built from device row pointers

These are already using uninitialized device buffers today, which is correct.

### 2. Device output buffers written by `vcc_backtest_metrics_f32`

Safe when:

- the kernel writes every row of:
  - `d_out`
  - `d_slice_out`

These are already uninitialized today, which is correct.

### 3. Host vectors built with `Vec::with_capacity` and `push`

Safe when:

- each element is pushed exactly once before any read

Examples:

- combined `build_price_state_host`
- candle generators already follow this pattern

This is the preferred first step for host-side copy-elision here.

## Safe only if shared `alloc_policy` helpers already exist

### 4. Prefix-init vectors where the prefix is observable but the tail is overwrite-only

Examples:

- `build_vcc_desired_positions`
  - prefix `[0..start_t)` must be zero
  - tail `[start_t..n)` is written in-loop

This is a good candidate for shared prefix-init helpers, but not for ad hoc local unsafe.

## Places to **avoid** uninitialized allocation in the first patch

### 5. Host readback vectors for `copy_to(...)`

Current code:

```rust
let mut host_out = vec![0.0f32; out_len];
d_out.copy_to(&mut host_out)?;
```

Could this be made uninitialized? Possibly, but only if you already have a proven, repo-standard helper that guarantees `copy_to` fully initializes the destination. With only the provided files, I would **not** introduce new unsafe host-readback code here yet.

**Better first step:** reuse the same host vectors across calls so you avoid repeated allocation.

### 6. Any vector whose fill value is observable before guaranteed overwrite

Examples:

- full-return vectors where the caller can read uninitialized prefix
- warmup-sensitive outputs that rely on a NaN/0 prefix
- `desired` if you are not explicitly initializing the prefix

## Unavoidable copy boundaries that should stay

These copies are not the real problem and should stay unless a later benchmark says otherwise:

- device → host output readback after kernel execution
- host → device combo upload when combo contents actually differ
- host → device contiguous matrix upload when the source truly lives on the host and is not reusable

Do not spend early effort trying to remove the unavoidable boundary before you remove the repeated avoidable ones.

---

## 10. Recommended order of commits

I would implement this as a sequence of small commits:

1. `vcc_gpu_kernel.rs`: add `build_price_state_host` and replace the three host passes
2. `vcc_gpu_kernel.rs`: add `CachedPriceState`
3. `vcc_gpu_kernel.rs`: add `CachedPackedMatrix`
4. `vcc_gpu_kernel.rs`: add `VccComboScratch` and reusable host readback vectors
5. `vcc.rs`: add route metadata for private sweep helpers and CUDA-host fallback
6. `gpu_native_expansion_opt.rs`: add VCC optimizer benches
7. `composite_validation_workflow.rs`: add route assertions and VCC heavy validation benches
8. `vcc.rs`: promote one or two VCC parity diagnostics into non-ignored tests
9. only after measuring, consider row-remap ABI or prepared device price-state

This gives you clean rollback points if a parity issue appears.

---

## 11. Detailed checklist

## A. `vcc_gpu_kernel.rs` hot-path reuse

- [ ] Add `HostSliceKey`
- [ ] Add `CachedPriceState`
- [ ] Add `CachedPackedMatrix`
- [ ] Add `VccComboScratch`
- [ ] Extend `VccCudaRuntime` to own all four cache groups
- [ ] Replace `close_f32` + `compute_log_returns_f32` + `compute_log_returns_f64` with a single `build_price_state_host`
- [ ] Reuse `DeviceBuffer<f32>` for `close`
- [ ] Reuse `DeviceBuffer<f32>` for `lr`
- [ ] Reuse `DeviceBuffer<f64>` for `lr64`
- [ ] Reuse `DeviceBuffer<u64>` for row-pointer uploads when lengths match
- [ ] Reuse `DeviceBuffer<f32>` for packed fast squeeze rows
- [ ] Reuse `DeviceBuffer<f32>` for packed fast momentum rows
- [ ] Reuse `DeviceBuffer<f32>` for packed trend rows
- [ ] Reuse combo buffers:
  - [ ] `fast_row_index`
  - [ ] `trend_row_index`
  - [ ] `start_t`
  - [ ] `metric_start_t`
  - [ ] `end_t`
- [ ] Reuse `d_out`
- [ ] Reuse `d_slice_out`
- [ ] Reuse host `Vec<f32>` output buffer
- [ ] Reuse host `Vec<f64>` slice-output buffer
- [ ] Add checked `i32::try_from(...)` conversions for launch scalars
- [ ] Optionally reuse a single start buffer when `start_t` and `metric_start_t` are the same slice
- [ ] Keep kernel ABI unchanged for phase 1
- [ ] Re-run existing VCC diagnostics after the change

## B. `vcc.rs` route visibility and light CPU-side cleanup

- [ ] Add `VccCudaRouteMeta`
- [ ] Add a private sweep-return wrapper carrying both:
  - [ ] `VccPeriodSweepDevice`
  - [ ] `VccCudaRouteMeta`
- [ ] Split `compute_vcc_ttm_squeeze_sweep_cuda_direct` into:
  - [ ] device-input attempt
  - [ ] host-input fallback
  - [ ] fallback-reason capture
- [ ] Split `compute_vcc_squeeze_momentum_sweep_cuda_direct` the same way
- [ ] Split `compute_vcc_ttm_trend_sweep_cuda_direct` the same way
- [ ] Capture fallback reason in `compute_indicator_f64_with_warmup`
- [ ] Keep current public function signatures as compatibility wrappers where practical
- [ ] Replace fixed-size param `Vec`s with arrays in `compute_vcc_fast_signals`
- [ ] Replace fixed-size param `Vec`s with arrays in `compute_vcc_trend_signals`
- [ ] Decide whether `build_vcc_desired_positions` should adopt a shared prefix-init helper
- [ ] Do **not** add new local unsafe in `build_vcc_desired_positions` unless it is routed through shared alloc helpers

## C. `gpu_native_expansion_opt.rs` optimizer evidence

- [ ] Import `VccStrategyParams`
- [ ] Add `make_vcc_breakout_candles`
- [ ] Add `make_vcc_req`
- [ ] Add `bench_vcc_cpu`
- [ ] Add `bench_vcc_gpu_kernel`
- [ ] Add both to `criterion_group!`
- [ ] Run the new VCC CPU bench
- [ ] Run the new VCC GPU bench
- [ ] Verify the GPU route probe asserts `BackendUsed::Gpu`
- [ ] Capture pre-cache baseline
- [ ] Capture post-cache result on the same machine

## D. `composite_validation_workflow.rs` validation-cost evidence

- [ ] Import `BackendUsed` or use fully-qualified GPU assertions
- [ ] Add `assert_gpu_route(...)`
- [ ] Assert GPU route after the VCC GPU route probe
- [ ] Assert GPU route inside the VCC GPU benchmark iteration
- [ ] Add `make_vcc_validation_candles`
- [ ] Add `make_vcc_candidates_wide`
- [ ] Add a dedicated heavy VCC GPU validation group
- [ ] Add `bench_cpu_profile("vcc", 100_000, make_vcc_candidates_wide(32))`
- [ ] Add `bench_profile("vcc", 100_000, make_vcc_candidates_wide(32))`
- [ ] Verify the heavy VCC GPU group still routes to GPU
- [ ] Compare pre-cache vs post-cache validation throughput

## E. Parity and regression protection

- [ ] Add one non-ignored momentum roundtrip gate
- [ ] Add one non-ignored small VCC GPU metric parity gate
- [ ] Keep the long ignored diagnostics for investigation workflows
- [ ] Record measured tolerance for the new parity gate rather than inventing one
- [ ] Re-run parity after every cache-related commit

## F. Measurement and rollout discipline

- [ ] Measure warm-path optimizer cost before code changes
- [ ] Measure warm-path validation cost before code changes
- [ ] Land phase-1 cache changes without changing kernel ABI
- [ ] Re-measure optimizer cost
- [ ] Re-measure validation cost
- [ ] Only after that, decide whether row-remap ABI changes are justified
- [ ] Only after that, decide whether prepared device price-state inputs are justified
- [ ] Document whether VCC is CPU-preferred or GPU-preferred **after** the new benches exist

## G. Explicit “do not do this first” guardrail

- [ ] Do **not** rewrite the VCC kernel ABI before the staging cache is measured
- [ ] Do **not** claim a VCC CPU/GPU preference before adding the missing optimizer bench
- [ ] Do **not** add broad product or strategy changes
- [ ] Do **not** add ad hoc unsafe host-readback code unless it routes through repo-standard alloc helpers
- [ ] Do **not** remove warmup-visible zero/NaN initialization where the value is part of behavior

---

## Final recommendation

If you want the highest return for the least parity risk, do this in order:

1. `vcc_gpu_kernel.rs` cache/reuse work
2. `gpu_native_expansion_opt.rs` VCC Criterion coverage
3. `composite_validation_workflow.rs` route assertions + heavier VCC groups
4. `vcc.rs` fallback metadata and one real parity gate
5. only then consider kernel ABI or deeper device-input changes

That sequence directly targets the biggest confirmed waste in the current VCC CUDA path while giving you the benchmark and parity evidence needed to keep the work honest.
