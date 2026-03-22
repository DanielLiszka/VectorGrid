
# KQSR GPU-native optimization and copy-elision implementation plan

This document is a **strategy-local** review and implementation plan for the **KQSR** family only.

It follows the review frame from **`gpt54_feature_review_map.md` bundle 5** (“Strategy-local GPU-native optimization and validation routes”) and explicitly adopts the **allocation/copy-elision discipline** from **bundle 16** where it directly applies to `kqsr_gpu_kernel.rs`.

## Review frame used

### Surface under review

KQSR’s strategy-local GPU-native optimization and validation path, with emphasis on:

- strategy-local staging or upload waste
- CPU-reference prep that can erase GPU benefit
- per-strategy fallback behavior and route transparency
- per-strategy benchmark fidelity for optimization and validation
- safe use of uninitialized memory and copy-elision where the write-before-read guarantee is real

### Files reviewed

Primary files:

- `kqsr.rs`
- `kqsr_gpu_kernel.rs`

Supporting benchmark / validation files:

- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

Adjacent guidance explicitly used:

- `gpt54_feature_review_map.md` bundle 5 review structure
- `gpt54_feature_review_map.md` bundle 16 copy-elision / `MaybeUninit` rules
- Adjacent shared patterns mentioned in the map (for example shared `alloc_policy` helpers and resident GPU series patterns), but **this plan only makes claims grounded in the attached files**

### Hard scope guard

This plan does **not** propose:

- new strategy features
- new UI surfaces
- broad architecture rewrites
- GPU-expansion claims without route/fallback and benchmark context

It focuses on smaller, parity-safe changes first, then clearly labels higher-risk follow-ons.

---

# 1. Surface understanding

KQSR currently has two important cost surfaces.

1. **Signal production in `kqsr.rs`**
   - `compute_kqsr_fast_signals(...)`
   - `compute_kqsr_slow_signals(...)`
   - `compute_kqsr_fast_signals_f32(...)`
   - `compute_kqsr_slow_signals_f32(...)`

   These functions assemble the fast and slow KQSR signal families by repeatedly dispatching dynamic indicators (`kdj`, `srsi`, `ift_rsi`, `qqe`, `prb`). The CUDA-aware path tries `compute_dynamic_indicator_cuda_with_inputs(..., HostF32)` and silently falls back to CPU when that fails.

2. **Device metrics evaluation in `kqsr_gpu_kernel.rs`**
   - `eval_kqsr_metrics_gpu_kernel_device_rows(...)`

   This function already accepts **device pointers** for the row-major signal matrices, but it still:
   - uploads `close_f32` every call
   - allocates `d_lr` every call
   - recomputes log returns every call
   - uploads combo vectors every call
   - allocates `d_out` every call
   - zero-fills a host output vector before `copy_to(...)`

That means KQSR is not yet a fully warm, resident, reuse-heavy GPU path. It is a mixed path with real CUDA work, but also repeated staging that can eat into or erase the expected GPU advantage on repeated optimization/validation workloads.

## Timing boundaries used in this review

Every performance statement in this plan is assigned to one or more of these timing boundaries:

- **app-path**: actual runtime work inside the KQSR signal/kernel route
- **Criterion**: `gpu_native_expansion_opt.rs`
- **validation-cost**: `composite_validation_workflow.rs`
- **route-matrix**: route/fallback visibility and benchmark route attribution
- **profiling-only**: things best confirmed by Nsight or similar, even if likely from source inspection

---

# 2. Findings

Each finding uses the requested format from the map prompt.

## Finding 1

- **Severity:** high
- **Type:** performance
- **Location:** `kqsr_gpu_kernel.rs` → `eval_kqsr_metrics_gpu_kernel_device_rows`
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** The KQSR kernel path uploads `close_f32`, allocates `d_lr`, launches `compute_log_returns_f32`, uploads all combo arrays, allocates `d_out`, synchronizes, then reads back the full metrics matrix on every call. This is repeated per-evaluation staging on a path that is likely exercised many times during optimization and validation.
- **Recommended change:** Split the current function into:
  1. a **convenience wrapper** (keep the current API), and
  2. a **warm-path API** that accepts uploaded/resident series and uploaded/resident combos.

## Finding 2

- **Severity:** high
- **Type:** performance
- **Location:** `kqsr.rs` → `compute_indicator_f32`
- **Timing boundary affected:** app-path, Criterion, validation-cost
- **Why it matters:** CPU fallback for `compute_indicator_f32(...)` currently routes through `compute_indicator_f64(...)` and then converts the entire `Vec<f64>` to `Vec<f32>`. That is avoidable full-vector work in the exact scenario where the supposedly GPU-oriented path is falling back.
- **Recommended change:** Add a direct `series_to_f32(...)` helper and let the CPU fallback produce `Vec<f32>` directly.

## Finding 3

- **Severity:** medium
- **Type:** performance
- **Location:** `kqsr.rs` → `compute_kqsr_fast_signals`, `compute_kqsr_slow_signals`, `compute_kqsr_fast_signals_f32`, `compute_kqsr_slow_signals_f32`
- **Timing boundary affected:** app-path, Criterion
- **Why it matters:** KQSR issues duplicate indicator dispatches for multi-output families:
  - `srsi` twice (`k` and `d`)
  - `qqe` twice (`fast` and `slow`)
  - `prb` twice (`upper` and `lower`)

  The duplicate dispatch at the KQSR call-site is confirmed. Whether that fully duplicates underlying heavy computation depends on the adjacent dynamic-indicator implementation, which is not in the attached file set. At minimum, the repeated dispatch and output resolution are real.
- **Recommended change:** Treat multi-output dispatch collapse as a stage-2 optimization. Do the file-local cleanup now, then optionally extend the adjacent dynamic-indicator layer to return multiple outputs in one call.

## Finding 4

- **Severity:** medium
- **Type:** performance
- **Location:** `kqsr.rs` → first-valid detection in fast/slow signal builders
- **Timing boundary affected:** app-path, Criterion
- **Why it matters:** `first_valid` is currently derived by scanning each vector separately with `position(...)` and then taking the max. That means multiple full passes over long signal vectors where one pass is enough.
- **Recommended change:** Replace the repeated `position(...)` calls with a single-pass “find first finite in all required series” helper.

## Finding 5

- **Severity:** medium
- **Type:** performance
- **Location:** `kqsr.rs` → `build_kqsr_desired_positions`
- **Timing boundary affected:** app-path, validation-cost
- **Why it matters:** The CPU desired-position builder repeatedly uses `.get(...).copied().unwrap_or(...)` inside the main loop. For the normal case, the lengths are aligned and the indices are known. This adds repeated bounds/option work in a strategy-local hot loop.
- **Recommended change:** Keep the output buffer zero-initialized (that is required behavior), but switch to direct indexing after up-front length sanity assumptions (`debug_assert_eq!` or explicit checks if you want them in release too).

## Finding 6

- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `gpu_native_expansion_opt.rs`
- **Timing boundary affected:** Criterion
- **Why it matters:** KQSR’s GPU benchmark includes small, medium, and large pair grids, but the CPU side only includes small and medium. That makes the CPU/GPU crossover ambiguous.
- **Recommended change:** Add a KQSR-only CPU benchmark case list that matches the GPU case list exactly.

## Finding 7

- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `composite_validation_workflow.rs`
- **Timing boundary affected:** validation-cost
- **Why it matters:** KQSR only appears at `50_000` candles / `8` candidates in the validation benchmark profile list, while some other strategy families already have heavier `100_000` / `32` coverage. That is not enough evidence to decide how KQSR behaves on larger validation workloads.
- **Recommended change:** Add at least `100_000 / 32` KQSR profile coverage to both CPU and GPU validation benches. Consider `100_000 / 64` once the `32` case is stable.

## Finding 8

- **Severity:** medium
- **Type:** route-fidelity / benchmark-fidelity
- **Location:** `kqsr.rs` → `compute_indicator_f64`, `compute_indicator_f32`
- **Timing boundary affected:** route-matrix, Criterion
- **Why it matters:** The CUDA path falls back silently if the call fails or if prepared CUDA inputs are missing. That means a “GPU” benchmark can still spend meaningful time doing CPU signal prep without any strategy-local visibility.
- **Recommended change:** Add optional KQSR route accounting or debug logging for indicator dispatch results. This is not a raw speedup, but it is necessary for trustworthy benchmark interpretation.

## Finding 9

- **Severity:** medium
- **Type:** performance / copy-elision
- **Location:** `kqsr_gpu_kernel.rs` → host metrics readback
- **Timing boundary affected:** app-path, Criterion
- **Why it matters:** `host_metrics` is allocated as `vec![0.0f32; out_len]` immediately before `d_out.copy_to(...)`. If the device writes all elements, the prefill is wasted.
- **Recommended change:** Replace the zero-filled host buffer with the shared uninitialized host-readback helper from `alloc_policy` (or the shared project helper already used elsewhere), but only after confirming the kernel writes every element of every row on every path.

## Finding 10

- **Severity:** low
- **Type:** correctness / testing
- **Location:** `kqsr_gpu_kernel.rs` → `validate_inputs`
- **Timing boundary affected:** unknown
- **Why it matters:** The current validation checks pointer non-nullness, lengths, and basic shapes, but not combo bounds against `fast_rows`, `slow_rows`, and `cols`. Invalid indices can produce bad reads or undefined device behavior.
- **Recommended change:** Add host-side combo bounds validation before upload / launch.

## Finding 11

- **Severity:** low
- **Type:** testing
- **Location:** `kqsr.rs` tests
- **Timing boundary affected:** route-matrix, Criterion interpretation
- **Why it matters:** The current CUDA test is only a smoke test. It does not prove CPU/GPU parity for KQSR signals or fallback parity for the new direct-`f32` CPU path.
- **Recommended change:** Add CPU-vs-CPU(`f32`) parity tests and CPU-vs-GPU parity tests for fast/slow signals.

## Finding 12

- **Severity:** low
- **Type:** profiling-only
- **Location:** `kqsr_gpu_kernel.rs` → runtime/kernel launch setup
- **Timing boundary affected:** profiling-only, app-path
- **Why it matters:** Repeated capability probing (`detect_device_launch_caps`, `detect_kernel_launch_caps`) and repeated function lookup are likely small compared with transfers, but they are still repeated warm-path work.
- **Recommended change:** Cache launch-cap metadata in the runtime after the first launch. This is lower priority than eliminating repeated uploads and readback prefills.

---

# 3. Safe optimization opportunities

This section focuses on smaller, parity-safe changes first.

---

## 3.1 `kqsr.rs` — direct, low-risk improvements

### Goals

- remove the unnecessary CPU `f64 -> f32` round-trip
- reduce repeated vector scans for `first_valid`
- tighten the CPU desired-position loop
- add test coverage so later GPU-side work is safer

## 3.1.1 Add direct CPU-to-`f32` conversion

### Why this change is safe

The CPU path is already producing a fully materialized `DynamicIndicatorComputeOutput`. The only change is to convert that output directly to `Vec<f32>` instead of going through `Vec<f64>` first.

### Recommended code edit

Add this helper next to `series_to_f64(...)`:

```rust
#[cfg(feature = "cuda")]
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

Then replace the fallback path in `compute_indicator_f32(...)` with a direct CPU compute:

```rust
#[cfg(feature = "cuda")]
fn compute_indicator_f32(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f32>, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("KQSR indicator '{indicator_id}' is not registered for dynamic dispatch")
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

### Expected impact

- Removes a full extra allocation and cast pass on CPU fallback.
- Makes mixed-path or fallback-heavy KQSR runs cheaper.
- Improves both optimization and validation routes when CUDA indicator dispatch is unavailable or partially blocked.

---

## 3.1.2 Replace repeated `first_valid` scans with one-pass helpers

### Why this change is safe

The current logic is “max of the first finite index across a fixed set of series.” That can be computed with a single forward pass while preserving the exact meaning.

### Recommended code edit

Add helpers:

```rust
fn first_valid_quad_f64(a: &[f64], b: &[f64], c: &[f64], d: &[f64]) -> Option<usize> {
    let n = a.len().min(b.len()).min(c.len()).min(d.len());
    let (mut ia, mut ib, mut ic, mut id) = (None, None, None, None);

    for idx in 0..n {
        if ia.is_none() && a[idx].is_finite() {
            ia = Some(idx);
        }
        if ib.is_none() && b[idx].is_finite() {
            ib = Some(idx);
        }
        if ic.is_none() && c[idx].is_finite() {
            ic = Some(idx);
        }
        if id.is_none() && d[idx].is_finite() {
            id = Some(idx);
        }

        if let (Some(a0), Some(b0), Some(c0), Some(d0)) = (ia, ib, ic, id) {
            return Some(a0.max(b0).max(c0).max(d0));
        }
    }

    None
}

#[cfg(feature = "cuda")]
fn first_valid_quad_f32(a: &[f32], b: &[f32], c: &[f32], d: &[f32]) -> Option<usize> {
    let n = a.len().min(b.len()).min(c.len()).min(d.len());
    let (mut ia, mut ib, mut ic, mut id) = (None, None, None, None);

    for idx in 0..n {
        if ia.is_none() && a[idx].is_finite() {
            ia = Some(idx);
        }
        if ib.is_none() && b[idx].is_finite() {
            ib = Some(idx);
        }
        if ic.is_none() && c[idx].is_finite() {
            ic = Some(idx);
        }
        if id.is_none() && d[idx].is_finite() {
            id = Some(idx);
        }

        if let (Some(a0), Some(b0), Some(c0), Some(d0)) = (ia, ib, ic, id) {
            return Some(a0.max(b0).max(c0).max(d0));
        }
    }

    None
}
```

Then use them in the fast/slow builders.

For example, replace:

```rust
let first_valid = first_finite_idx_local(&j)
    .zip(first_finite_idx_local(&srsi_k))
    .zip(first_finite_idx_local(&srsi_d))
    .zip(first_finite_idx_local(&ift_rsi))
    .map(|(((a, b), c), d)| a.max(b).max(c).max(d))
    .ok_or_else(|| "KQSR fast signals are all non-finite".to_string())?;
```

with:

```rust
let first_valid = first_valid_quad_f64(&j, &srsi_k, &srsi_d, &ift_rsi)
    .ok_or_else(|| "KQSR fast signals are all non-finite".to_string())?;
```

And do the same for:
- `compute_kqsr_slow_signals(...)`
- `compute_kqsr_fast_signals_f32(...)`
- `compute_kqsr_slow_signals_f32(...)`

### Expected impact

- Reduces memory traversal over large vectors.
- Low risk, easy to test, and helpful in both CPU and mixed GPU/CPU signal prep.

---

## 3.1.3 Tighten `build_kqsr_desired_positions(...)` without changing semantics

### Important copy-elision note

**Keep**:

```rust
let mut desired = vec![0i32; n];
```

This is **not** a candidate for uninitialized allocation. The zeros are behaviorally observable:
- all entries before `start_t` must remain zero
- `start_t >= n` returns the prefilled vector
- any skipped tail or untouched elements must remain zero

### Why this loop still deserves cleanup

Even though the output buffer must stay zero-initialized, the loop body can be simplified and sped up by avoiding repeated `get(...).unwrap_or(...)` calls on aligned series.

### Recommended code edit

```rust
pub fn build_kqsr_desired_positions(
    close: &[f64],
    signals: &KqsrSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &KqsrStrategyParams,
) -> Vec<i32> {
    let n = close.len();
    let mut desired = vec![0i32; n]; // keep zero-init: observable behavior
    if start_t >= n {
        return desired;
    }

    debug_assert_eq!(signals.j.len(), n);
    debug_assert_eq!(signals.srsi_k.len(), n);
    debug_assert_eq!(signals.srsi_d.len(), n);
    debug_assert_eq!(signals.ift_rsi.len(), n);
    debug_assert_eq!(signals.qqe_fast.len(), n);
    debug_assert_eq!(signals.qqe_slow.len(), n);
    debug_assert_eq!(signals.prb_upper.len(), n);
    debug_assert_eq!(signals.prb_lower.len(), n);

    let j = &signals.j;
    let srsi_k = &signals.srsi_k;
    let srsi_d = &signals.srsi_d;
    let ift_rsi = &signals.ift_rsi;
    let qqe_fast = &signals.qqe_fast;
    let qqe_slow = &signals.qqe_slow;
    let prb_upper = &signals.prb_upper;
    let prb_lower = &signals.prb_lower;

    let trade_offset = usize::from(strategy.trade_on_next_bar);
    let neutral = 50.0;
    let mut pos = 0i32;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close[sig_t];
        let j_now = j[sig_t];
        let j_prev = j[prev_t];
        let srsi_k_now = srsi_k[sig_t];
        let srsi_k_prev = srsi_k[prev_t];
        let srsi_d_now = srsi_d[sig_t];
        let ift_now = ift_rsi[sig_t];
        let qqe_fast_now = qqe_fast[sig_t];
        let qqe_slow_now = qqe_slow[sig_t];
        let prb_upper_now = prb_upper[sig_t];
        let prb_lower_now = prb_lower[sig_t];

        let prb_frac = prb_band_fraction(close_now, prb_lower_now, prb_upper_now);
        let prb_long_ok = !prb_frac.is_finite() || prb_frac <= cfg.prb_threshold;
        let prb_short_ok = !prb_frac.is_finite() || prb_frac >= (1.0 - cfg.prb_threshold);

        let long_entry = j_prev.is_finite()
            && j_now.is_finite()
            && j_prev < cfg.oversold
            && j_now >= cfg.oversold
            && srsi_k_now.is_finite()
            && srsi_d_now.is_finite()
            && srsi_k_prev.is_finite()
            && srsi_k_now > srsi_d_now
            && srsi_k_now > srsi_k_prev
            && ift_now.is_finite()
            && ift_now > 0.0
            && qqe_fast_now.is_finite()
            && qqe_slow_now.is_finite()
            && qqe_fast_now > qqe_slow_now
            && prb_long_ok;

        let short_entry = !strategy.long_only
            && j_prev.is_finite()
            && j_now.is_finite()
            && j_prev > cfg.overbought
            && j_now <= cfg.overbought
            && srsi_k_now.is_finite()
            && srsi_d_now.is_finite()
            && srsi_k_prev.is_finite()
            && srsi_k_now < srsi_d_now
            && srsi_k_now < srsi_k_prev
            && ift_now.is_finite()
            && ift_now < 0.0
            && qqe_fast_now.is_finite()
            && qqe_slow_now.is_finite()
            && qqe_fast_now < qqe_slow_now
            && prb_short_ok;

        if pos == 0 {
            if long_entry {
                pos = 1;
            } else if short_entry {
                pos = -1;
            }
        } else if pos > 0 {
            let mut exit_now = (qqe_fast_now.is_finite()
                && qqe_slow_now.is_finite()
                && qqe_fast_now < qqe_slow_now)
                || (j_now.is_finite() && j_now >= neutral);
            if short_entry {
                if strategy.allow_flip {
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
            let mut exit_now = (qqe_fast_now.is_finite()
                && qqe_slow_now.is_finite()
                && qqe_fast_now > qqe_slow_now)
                || (j_now.is_finite() && j_now <= neutral);
            if long_entry {
                if strategy.allow_flip {
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

### Expected impact

- Small-to-medium CPU-side win.
- No semantic change if the series lengths are aligned as expected.
- Good cleanup before deeper GPU path tuning.

---

## 3.1.4 Add parity tests before larger changes

### Recommended test additions in `kqsr.rs`

Add a small comparison helper:

```rust
#[cfg(feature = "cuda")]
fn assert_series_close_f64_f32(label: &str, lhs: &[f64], rhs: &[f32], atol: f64, rtol: f64) {
    assert_eq!(lhs.len(), rhs.len(), "length mismatch for {label}");
    for (i, (l, r)) in lhs.iter().zip(rhs.iter()).enumerate() {
        let r64 = *r as f64;
        if !l.is_finite() || !r64.is_finite() {
            assert_eq!(
                l.is_finite(),
                r64.is_finite(),
                "finite/non-finite mismatch for {label} at {i}: lhs={l} rhs={r64}"
            );
            continue;
        }
        let diff = (l - r64).abs();
        let tol = atol.max(rtol * l.abs());
        assert!(
            diff <= tol,
            "{label} mismatch at {i}: lhs={l} rhs={r64} diff={diff} tol={tol}"
        );
    }
}
```

Then add two tests.

### Test 1: CPU `f32` path matches CPU `f64` path

```rust
#[cfg(feature = "cuda")]
#[test]
fn compute_kqsr_f32_cpu_path_matches_f64_reference() {
    let candles = make_trending_candles(512);
    let cfg = KqsrStrategyParams::default();
    let periods = resolve_kqsr_periods(&cfg, 12, 34);

    let fast_f64 = compute_kqsr_fast_signals(
        &candles,
        "close",
        &Backend::CpuOnly,
        periods,
        &cfg,
        None,
    )
    .expect("cpu fast f64");

    let slow_f64 = compute_kqsr_slow_signals(
        &candles,
        "close",
        &Backend::CpuOnly,
        periods,
        None,
    )
    .expect("cpu slow f64");

    let fast_f32 = compute_kqsr_fast_signals_f32(
        &candles,
        "close",
        &Backend::CpuOnly,
        periods,
        &cfg,
        None,
    )
    .expect("cpu fast f32");

    let slow_f32 = compute_kqsr_slow_signals_f32(
        &candles,
        "close",
        &Backend::CpuOnly,
        periods,
        None,
    )
    .expect("cpu slow f32");

    assert_series_close_f64_f32("j", &fast_f64.j, &fast_f32.j, 1e-5, 1e-5);
    assert_series_close_f64_f32("srsi_k", &fast_f64.srsi_k, &fast_f32.srsi_k, 1e-5, 1e-5);
    assert_series_close_f64_f32("srsi_d", &fast_f64.srsi_d, &fast_f32.srsi_d, 1e-5, 1e-5);
    assert_series_close_f64_f32("ift_rsi", &fast_f64.ift_rsi, &fast_f32.ift_rsi, 1e-5, 1e-5);

    assert_series_close_f64_f32("qqe_fast", &slow_f64.qqe_fast, &slow_f32.qqe_fast, 1e-5, 1e-5);
    assert_series_close_f64_f32("qqe_slow", &slow_f64.qqe_slow, &slow_f32.qqe_slow, 1e-5, 1e-5);
    assert_series_close_f64_f32("prb_upper", &slow_f64.prb_upper, &slow_f32.prb_upper, 1e-5, 1e-5);
    assert_series_close_f64_f32("prb_lower", &slow_f64.prb_lower, &slow_f32.prb_lower, 1e-5, 1e-5);
}
```

### Test 2: GPU signal path matches CPU reference

```rust
#[cfg(feature = "cuda")]
#[test]
fn compute_kqsr_gpu_signals_match_cpu_reference() {
    if !cuda_available() {
        return;
    }

    let candles = make_trending_candles(1024);
    let cfg = KqsrStrategyParams::default();
    let periods = resolve_kqsr_periods(&cfg, 12, 34);
    let prepared = PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
        .expect("cuda inputs");

    let fast_cpu = compute_kqsr_fast_signals(
        &candles,
        "close",
        &Backend::CpuOnly,
        periods,
        &cfg,
        None,
    )
    .expect("cpu fast");

    let slow_cpu = compute_kqsr_slow_signals(
        &candles,
        "close",
        &Backend::CpuOnly,
        periods,
        None,
    )
    .expect("cpu slow");

    let fast_gpu = compute_kqsr_fast_signals_f32(
        &candles,
        "close",
        &Backend::GpuOnly { device_id: 0 },
        periods,
        &cfg,
        Some(&prepared),
    )
    .expect("gpu fast");

    let slow_gpu = compute_kqsr_slow_signals_f32(
        &candles,
        "close",
        &Backend::GpuOnly { device_id: 0 },
        periods,
        Some(&prepared),
    )
    .expect("gpu slow");

    assert_series_close_f64_f32("j", &fast_cpu.j, &fast_gpu.j, 1e-4, 1e-4);
    assert_series_close_f64_f32("srsi_k", &fast_cpu.srsi_k, &fast_gpu.srsi_k, 1e-4, 1e-4);
    assert_series_close_f64_f32("srsi_d", &fast_cpu.srsi_d, &fast_gpu.srsi_d, 1e-4, 1e-4);
    assert_series_close_f64_f32("ift_rsi", &fast_cpu.ift_rsi, &fast_gpu.ift_rsi, 1e-4, 1e-4);

    assert_series_close_f64_f32("qqe_fast", &slow_cpu.qqe_fast, &slow_gpu.qqe_fast, 1e-4, 1e-4);
    assert_series_close_f64_f32("qqe_slow", &slow_cpu.qqe_slow, &slow_gpu.qqe_slow, 1e-4, 1e-4);
    assert_series_close_f64_f32("prb_upper", &slow_cpu.prb_upper, &slow_gpu.prb_upper, 1e-4, 1e-4);
    assert_series_close_f64_f32("prb_lower", &slow_cpu.prb_lower, &slow_gpu.prb_lower, 1e-4, 1e-4);
}
```

### Expected impact

- Enables safe refactoring of both CPU fallback and GPU staging.
- Gives you confidence that KQSR’s FP32 path remains acceptable before deeper changes.

---

## 3.1.5 Optional route accounting (recommended for benches, not required for phase 1)

If you want strategy-local route evidence without changing the external optimization APIs yet, add an optional internal summary struct and thread it through `compute_indicator_f32(...)` and `compute_indicator_f64(...)` as `Option<&mut KqsrIndicatorRouteSummary>`.

Suggested shape:

```rust
#[cfg(feature = "cuda")]
#[derive(Default, Debug, Clone)]
pub struct KqsrIndicatorRouteSummary {
    pub requested_gpu: bool,
    pub gpu_successes: usize,
    pub cpu_fallbacks: usize,
    pub fallback_reasons: Vec<String>,
}
```

Use it only in debug/bench call paths at first. Do not widen the public surface unless the benchmark reporting will actually consume it.

---

## 3.1.6 Optional stage-2 follow-on: collapse duplicate multi-output dispatches

This is an important optimization opportunity, but it likely needs adjacent support from the dynamic-indicator dispatch layer.

### Confirmed from attached files

The KQSR call-site definitely dispatches:
- `srsi` twice
- `qqe` twice
- `prb` twice

### Safer implementation order

1. Land the file-local changes first.
2. Benchmark again.
3. If signal staging is still dominant, extend the adjacent dynamic-indicator layer to return multiple outputs from one compute.
4. Then update KQSR to consume the grouped outputs.

### Example end-state API shape (adjacent work, not in the attached files)

```rust
let qqe = compute_indicator_outputs_f32(
    candles,
    ma_source,
    backend,
    "qqe",
    &[qqe_fast_output.as_str(), qqe_slow_output.as_str()],
    qqe_params.as_slice(),
    prepared_cuda_inputs,
)?;
let qqe_fast = qqe[qqe_fast_output.as_str()].clone();
let qqe_slow = qqe[qqe_slow_output.as_str()].clone();
```

Do **not** block the phase-1 plan on this; it is more intrusive than the direct fixes above.

---

## 3.2 `kqsr_gpu_kernel.rs` — the main warm-path improvement area

### Goals

- stop re-uploading invariant series data on every evaluation
- stop re-uploading combo vectors on every evaluation
- reuse device output scratch
- remove wasted host zero-fill before readback
- strengthen input validation
- preserve the current public API while introducing a better warm path

## 3.2.1 Keep the current entrypoint, but refactor it into a wrapper

### Current cold/convenience path

Today:

- `signals.close_f32` is uploaded every call
- log returns are recomputed every call
- combo vectors are uploaded every call
- output buffer is allocated every call
- host readback buffer is zero-filled every call

### Proposed shape

Keep the current entrypoint:

```rust
pub fn eval_kqsr_metrics_gpu_kernel_device_rows(...)
```

but make it a wrapper around a new uploaded/resident path.

### New types to add

```rust
pub struct KqsrGpuSignalDeviceMatrices {
    pub cols: usize,
    pub j_row_major_ptr: u64,
    pub fast_rows: usize,
    pub fast_stride: usize,
    pub srsi_k_row_major_ptr: u64,
    pub srsi_d_row_major_ptr: u64,
    pub ift_rsi_row_major_ptr: u64,
    pub qqe_fast_row_major_ptr: u64,
    pub qqe_slow_row_major_ptr: u64,
    pub slow_rows: usize,
    pub slow_stride: usize,
    pub prb_upper_row_major_ptr: u64,
    pub prb_lower_row_major_ptr: u64,
}

impl<'a> KqsrGpuDeviceSignalMatrices<'a> {
    fn device_view(&self) -> KqsrGpuSignalDeviceMatrices {
        KqsrGpuSignalDeviceMatrices {
            cols: self.cols,
            j_row_major_ptr: self.j_row_major_ptr,
            fast_rows: self.fast_rows,
            fast_stride: self.fast_stride,
            srsi_k_row_major_ptr: self.srsi_k_row_major_ptr,
            srsi_d_row_major_ptr: self.srsi_d_row_major_ptr,
            ift_rsi_row_major_ptr: self.ift_rsi_row_major_ptr,
            qqe_fast_row_major_ptr: self.qqe_fast_row_major_ptr,
            qqe_slow_row_major_ptr: self.qqe_slow_row_major_ptr,
            slow_rows: self.slow_rows,
            slow_stride: self.slow_stride,
            prb_upper_row_major_ptr: self.prb_upper_row_major_ptr,
            prb_lower_row_major_ptr: self.prb_lower_row_major_ptr,
        }
    }
}

pub struct KqsrGpuUploadedSeries {
    pub cols: usize,
    d_close: DeviceBuffer<f32>,
    d_lr: DeviceBuffer<f32>,
}

pub struct KqsrGpuUploadedComboBatch {
    pub len: usize,
    d_fast_row_index: DeviceBuffer<i32>,
    d_slow_row_index: DeviceBuffer<i32>,
    d_start_t: DeviceBuffer<i32>,
    d_end_t: DeviceBuffer<i32>,
}
```

### Why this design is worth it

- preserves the current wrapper API
- creates a true warm path for repeated evaluations
- makes the avoidable upload boundaries explicit
- avoids “magical” caching based on slice identity or hashing

---

## 3.2.2 Add upload helpers

### Upload market series once per dataset

```rust
pub fn upload_kqsr_series(
    device_id: u32,
    close_f32: &[f32],
) -> Result<KqsrGpuUploadedSeries, String> {
    if close_f32.is_empty() {
        return Err("KQSR GPU kernel requires non-empty close series".to_string());
    }

    KQSR_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, KqsrCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "KQSR GPU runtime missing".to_string())?;
        rt.set_current()?;

        let d_close = DeviceBuffer::from_slice(close_f32).map_err(|e| e.to_string())?;
        let mut d_lr =
            unsafe { DeviceBuffer::<f32>::uninitialized(close_f32.len()) }.map_err(|e| e.to_string())?;

        launch_compute_log_returns_f32(rt, device_id, &d_close, &mut d_lr, close_f32.len())?;
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        Ok(KqsrGpuUploadedSeries {
            cols: close_f32.len(),
            d_close,
            d_lr,
        })
    })
}
```

### Upload combo batch once per candidate set

```rust
pub fn upload_kqsr_combo_batch(
    device_id: u32,
    signals: &KqsrGpuSignalDeviceMatrices,
    combos: &KqsrGpuComboBatch<'_>,
) -> Result<KqsrGpuUploadedComboBatch, String> {
    validate_signal_device_view(signals)?;
    validate_combo_vectors(signals, combos)?;

    KQSR_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, KqsrCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "KQSR GPU runtime missing".to_string())?;
        rt.set_current()?;

        Ok(KqsrGpuUploadedComboBatch {
            len: combos.fast_row_index.len(),
            d_fast_row_index: DeviceBuffer::from_slice(combos.fast_row_index).map_err(|e| e.to_string())?,
            d_slow_row_index: DeviceBuffer::from_slice(combos.slow_row_index).map_err(|e| e.to_string())?,
            d_start_t: DeviceBuffer::from_slice(combos.start_t).map_err(|e| e.to_string())?,
            d_end_t: DeviceBuffer::from_slice(combos.end_t).map_err(|e| e.to_string())?,
        })
    })
}
```

### Important note

This does **not** eliminate the first host→device copy if your source data begins on the host. It eliminates the **repeated** copy when the same data is used for many evaluations.

That distinction matters:
- first upload: usually unavoidable
- repeated upload of unchanged data: avoidable and worth fixing

---

## 3.2.3 Add a reusable output scratch buffer

If a shared scratch helper (for example from a nearby GPU runtime module) is already public, prefer using that. If not, a small local scratch holder is fine.

### Local fallback version

```rust
struct KqsrScratch {
    d_out: Option<DeviceBuffer<f32>>,
    out_cap: usize,
}

impl KqsrScratch {
    fn new() -> Self {
        Self {
            d_out: None,
            out_cap: 0,
        }
    }

    fn ensure_out_len(&mut self, len: usize) -> Result<(), String> {
        if self.out_cap < len || self.d_out.is_none() {
            self.d_out =
                Some(unsafe { DeviceBuffer::<f32>::uninitialized(len) }.map_err(|e| e.to_string())?);
            self.out_cap = len;
        }
        Ok(())
    }

    fn out_mut(&mut self) -> &mut DeviceBuffer<f32> {
        self.d_out.as_mut().expect("scratch buffer initialized")
    }
}
```

Then extend the runtime:

```rust
struct KqsrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    scratch: KqsrScratch,
    // optionally cache launch-cap metadata here after the first launch
}
```

Initialize it in `KqsrCudaRuntime::new(...)`:

```rust
Ok(Self {
    context,
    module,
    stream,
    scratch: KqsrScratch::new(),
})
```

### Why this is worth it

For repeated evaluations, output allocation becomes pure churn. Reusing `d_out` removes an allocation on every launch.

---

## 3.2.4 Add a true warm-path evaluation function

### Suggested new entrypoint

```rust
pub fn eval_kqsr_metrics_gpu_kernel_uploaded(
    device_id: u32,
    series: &KqsrGpuUploadedSeries,
    signals: &KqsrGpuSignalDeviceMatrices,
    combos: &KqsrGpuUploadedComboBatch,
    strategy: &StrategyConfig,
    kqsr: &KqsrStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_signal_device_view(signals)?;

    if combos.len == 0 {
        return Ok(Vec::new());
    }

    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "KQSR GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    if series.cols != signals.cols {
        return Err("KQSR uploaded series / signal column mismatch".to_string());
    }

    let out_len = combos
        .len
        .checked_mul(KQSR_METRICS_STRIDE)
        .ok_or_else(|| "KQSR GPU metrics buffer overflow".to_string())?;

    KQSR_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, KqsrCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "KQSR GPU runtime missing".to_string())?;
        rt.set_current()?;
        rt.scratch.ensure_out_len(out_len)?;

        let d_out = rt.scratch.out_mut();

        let kernel = rt
            .module
            .get_function("kqsr_backtest_metrics_f32")
            .map_err(|e| format!("KQSR get kqsr_backtest_metrics_f32: {e}"))?;

        // Keep the current launch-shape logic here.
        // Cache capability metadata after the first call if practical.

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
        let oversold = kqsr.oversold.clamp(0.0, 100.0) as f32;
        let overbought = kqsr.overbought.clamp(0.0, 100.0) as f32;
        let prb_threshold = kqsr.prb_threshold.clamp(0.0, 1.0) as f32;

        let mut close_ptr = series.d_close.as_device_ptr().as_raw();
        let mut lr_ptr = series.d_lr.as_device_ptr().as_raw();
        let mut t_i = i32::try_from(signals.cols).map_err(|_| "KQSR cols exceeds i32 range".to_string())?;
        let mut j_ptr = signals.j_row_major_ptr;
        let mut fast_rows_i =
            i32::try_from(signals.fast_rows).map_err(|_| "KQSR fast_rows exceeds i32 range".to_string())?;
        let mut fast_stride_i =
            i32::try_from(signals.fast_stride).map_err(|_| "KQSR fast_stride exceeds i32 range".to_string())?;
        let mut srsi_k_ptr = signals.srsi_k_row_major_ptr;
        let mut srsi_d_ptr = signals.srsi_d_row_major_ptr;
        let mut ift_ptr = signals.ift_rsi_row_major_ptr;
        let mut qqe_fast_ptr = signals.qqe_fast_row_major_ptr;
        let mut qqe_slow_ptr = signals.qqe_slow_row_major_ptr;
        let mut slow_rows_i =
            i32::try_from(signals.slow_rows).map_err(|_| "KQSR slow_rows exceeds i32 range".to_string())?;
        let mut slow_stride_i =
            i32::try_from(signals.slow_stride).map_err(|_| "KQSR slow_stride exceeds i32 range".to_string())?;
        let mut prb_upper_ptr = signals.prb_upper_row_major_ptr;
        let mut prb_lower_ptr = signals.prb_lower_row_major_ptr;
        let mut combo_fast_ptr = combos.d_fast_row_index.as_device_ptr().as_raw();
        let mut combo_slow_ptr = combos.d_slow_row_index.as_device_ptr().as_raw();
        let mut combo_start_ptr = combos.d_start_t.as_device_ptr().as_raw();
        let mut combo_end_ptr = combos.d_end_t.as_device_ptr().as_raw();
        let mut combo_count_i =
            i32::try_from(combos.len).map_err(|_| "KQSR combo_count exceeds i32 range".to_string())?;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut oversold_f = oversold;
        let mut overbought_f = overbought;
        let mut prb_threshold_f = prb_threshold;
        let mut out_stride_i = i32::try_from(KQSR_METRICS_STRIDE)
            .map_err(|_| "KQSR metrics stride exceeds i32 range".to_string())?;
        let mut out_ptr = d_out.as_device_ptr().as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut j_ptr as *mut _ as *mut c_void,
                &mut fast_rows_i as *mut _ as *mut c_void,
                &mut fast_stride_i as *mut _ as *mut c_void,
                &mut srsi_k_ptr as *mut _ as *mut c_void,
                &mut srsi_d_ptr as *mut _ as *mut c_void,
                &mut ift_ptr as *mut _ as *mut c_void,
                &mut qqe_fast_ptr as *mut _ as *mut c_void,
                &mut qqe_slow_ptr as *mut _ as *mut c_void,
                &mut slow_rows_i as *mut _ as *mut c_void,
                &mut slow_stride_i as *mut _ as *mut c_void,
                &mut prb_upper_ptr as *mut _ as *mut c_void,
                &mut prb_lower_ptr as *mut _ as *mut c_void,
                &mut combo_fast_ptr as *mut _ as *mut c_void,
                &mut combo_slow_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut oversold_f as *mut _ as *mut c_void,
                &mut overbought_f as *mut _ as *mut c_void,
                &mut prb_threshold_f as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];

            rt.stream
                .launch(
                    &kernel,
                    (/* existing grid_x */, 1, 1),
                    (/* existing block_x */, 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        rt.stream.synchronize().map_err(|e| e.to_string())?;

        // Prefer the shared alloc_policy helper here once confirmed available.
        let mut host_metrics: Vec<f32> = alloc_uninit_copy(out_len);
        d_out
            .copy_to(host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out = Vec::with_capacity(combos.len);
        for row in host_metrics.chunks_exact(KQSR_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }

        Ok(out)
    })
}
```

### Important note about `alloc_uninit_copy(...)`

Per the map’s bundle-16 rules, only use uninitialized host readback when **every element is guaranteed to be written before any read**. Before making this change:

1. confirm `kqsr_backtest_metrics_f32` writes all `KQSR_METRICS_STRIDE` elements for every combo row on every execution path
2. keep the existing zero-filled host vector if there is any partial-write path

If the shared `alloc_policy` helper is not already reachable from this module, prefer making it reachable centrally rather than adding local unsafe code here.

---

## 3.2.5 Make the old API delegate to the new warm path

Keep the current function as a convenience wrapper:

```rust
pub fn eval_kqsr_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &KqsrGpuDeviceSignalMatrices<'_>,
    combos: &KqsrGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    kqsr: &KqsrStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;

    let series = upload_kqsr_series(device_id, signals.close_f32)?;
    let combo_batch = upload_kqsr_combo_batch(device_id, &signals.device_view(), combos)?;

    eval_kqsr_metrics_gpu_kernel_uploaded(
        device_id,
        &series,
        &signals.device_view(),
        &combo_batch,
        strategy,
        kqsr,
    )
}
```

### Why this matters

- no existing callers break
- warm-path users can move over incrementally
- cold-path behavior remains simple and correct

---

## 3.2.6 Strengthen host-side input validation

### Why this matters

Bounds bugs are cheap to catch on the host and expensive or dangerous on the device.

### Recommended code edit

Split the current validation into:
- signal matrix validation
- combo vector validation
- combo bounds validation

```rust
fn validate_signal_device_view(signals: &KqsrGpuSignalDeviceMatrices) -> Result<(), String> {
    if signals.cols == 0 {
        return Err("KQSR GPU kernel requires non-empty series".to_string());
    }
    if signals.j_row_major_ptr == 0
        || signals.srsi_k_row_major_ptr == 0
        || signals.srsi_d_row_major_ptr == 0
        || signals.ift_rsi_row_major_ptr == 0
        || signals.qqe_fast_row_major_ptr == 0
        || signals.qqe_slow_row_major_ptr == 0
        || signals.prb_upper_row_major_ptr == 0
        || signals.prb_lower_row_major_ptr == 0
    {
        return Err("KQSR GPU signal pointer is null".to_string());
    }
    if signals.fast_rows == 0
        || signals.fast_stride == 0
        || signals.slow_rows == 0
        || signals.slow_stride == 0
    {
        return Err("KQSR GPU signal matrix shape is invalid".to_string());
    }
    Ok(())
}

fn validate_combo_vectors(
    signals: &KqsrGpuSignalDeviceMatrices,
    combos: &KqsrGpuComboBatch<'_>,
) -> Result<(), String> {
    let combos_len = combos.fast_row_index.len();
    if combos.slow_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("KQSR GPU combo vector length mismatch".to_string());
    }

    for i in 0..combos_len {
        let fast_idx = combos.fast_row_index[i];
        let slow_idx = combos.slow_row_index[i];
        let start_t = combos.start_t[i];
        let end_t = combos.end_t[i];

        if fast_idx < 0 || fast_idx as usize >= signals.fast_rows {
            return Err(format!("KQSR GPU fast_row_index out of range at combo {i}: {fast_idx}"));
        }
        if slow_idx < 0 || slow_idx as usize >= signals.slow_rows {
            return Err(format!("KQSR GPU slow_row_index out of range at combo {i}: {slow_idx}"));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("KQSR GPU start/end_t must be non-negative at combo {i}"));
        }

        let start_t = start_t as usize;
        let end_t = end_t as usize;

        if start_t >= end_t {
            return Err(format!(
                "KQSR GPU start_t must be < end_t at combo {i}: start={start_t} end={end_t}"
            ));
        }
        if end_t > signals.cols {
            return Err(format!(
                "KQSR GPU end_t exceeds cols at combo {i}: end={end_t} cols={}",
                signals.cols
            ));
        }
    }

    Ok(())
}

fn validate_inputs(
    signals: &KqsrGpuDeviceSignalMatrices<'_>,
    combos: &KqsrGpuComboBatch<'_>,
) -> Result<(), String> {
    if signals.close_f32.len() != signals.cols {
        return Err("KQSR GPU close length mismatch".to_string());
    }
    let view = signals.device_view();
    validate_signal_device_view(&view)?;
    validate_combo_vectors(&view, combos)
}
```

### Expected impact

- Minimal overhead relative to the work being launched
- Stronger correctness guarantees
- Better error messages during benchmarking and validation

---

## 3.2.7 Cache launch-cap metadata after the first launch

### Why this is secondary

This is not the main win. The main wins are:
- resident series
- resident combos
- reusable output buffer
- uninitialized host readback where safe

### Still worth doing

If `detect_device_launch_caps(...)` and `detect_kernel_launch_caps(...)` are cheap enough, this may barely matter. But if you are chasing repeated warm-path overhead, cache the metadata in `KqsrCudaRuntime` after the first successful probe.

### Implementation guidance

Use the exact concrete types returned by your existing helpers. The field names below are placeholders:

```rust
struct KqsrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    scratch: KqsrScratch,
    // replace these with the actual return types from your helper functions:
    device_caps: Option<DeviceCapsType>,
    compute_lr_caps: Option<KernelCapsType>,
    metrics_caps: Option<KernelCapsType>,
}
```

Do not over-engineer this before the larger staging wins land.

---

## 3.2.8 Add a small validation test for bounds handling

Add a simple host-side unit test in `kqsr_gpu_kernel.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_inputs_rejects_out_of_bounds_combo_indices() {
        let close = vec![1.0f32; 32];
        let signals = KqsrGpuDeviceSignalMatrices {
            close_f32: &close,
            cols: 32,
            j_row_major_ptr: 1,
            fast_rows: 2,
            fast_stride: 32,
            srsi_k_row_major_ptr: 1,
            srsi_d_row_major_ptr: 1,
            ift_rsi_row_major_ptr: 1,
            qqe_fast_row_major_ptr: 1,
            qqe_slow_row_major_ptr: 1,
            slow_rows: 2,
            slow_stride: 32,
            prb_upper_row_major_ptr: 1,
            prb_lower_row_major_ptr: 1,
        };

        let combos = KqsrGpuComboBatch {
            fast_row_index: &[2], // invalid: only 0..1 is allowed
            slow_row_index: &[0],
            start_t: &[0],
            end_t: &[31],
        };

        assert!(validate_inputs(&signals, &combos).is_err());
    }
}
```

This is cheap protection against a nasty class of GPU-side failures.

---

## 3.3 `gpu_native_expansion_opt.rs` — make KQSR’s benchmark evidence stronger

### Goals

- make CPU and GPU KQSR case grids comparable
- add KQSR-specific signal-build evidence
- separate “optimizer end-to-end” from “signal staging cost”

## 3.3.1 Add matched CPU cases for KQSR

The current CPU helper only runs:

- `small`
- `medium`

while KQSR GPU runs:

- `small/975pairs`
- `medium/2365pairs`
- `large/13385pairs`

### Recommended code edit

Add a case-aware CPU helper:

```rust
type BenchCase = (&'static str, (u32, u32, u32), (u32, u32, u32));

fn bench_cpu_cases_with_cases<F>(
    c: &mut Criterion,
    group_name: &str,
    candles: &Candles,
    cases: &[BenchCase],
    build_req: F,
) where
    F: Fn(Backend, (u32, u32, u32), (u32, u32, u32)) -> CompositeCpuRequest,
{
    let cancel = AtomicBool::new(false);

    let mut group = c.benchmark_group(group_name);
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = build_req(Backend::CpuOnly, fast_range, slow_range);
        group.bench_function(label, |b| {
            b.iter(|| {
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req.clone(),
                    candles,
                    &cancel,
                    None,
                )
                .expect("cpu bench run");
            })
        });
    }

    group.finish();
}
```

Then add a KQSR-specific case list:

```rust
const KQSR_CASES: &[BenchCase] = &[
    ("small/975pairs", (5, 20, 1), (20, 80, 1)),
    ("medium/2365pairs", (5, 30, 1), (30, 120, 1)),
    ("large/13385pairs", (5, 80, 1), (20, 220, 1)),
];
```

And update only the KQSR CPU benchmark:

```rust
fn bench_kqsr_cpu(c: &mut Criterion) {
    if !bench_target_enabled("kqsr") {
        return;
    }
    let candles = make_kqsr_oscillating_candles(200_000);
    bench_cpu_cases_with_cases(c, "kqsr/cpu/200k", &candles, KQSR_CASES, make_kqsr_req);
}
```

### Why this matters

This gives you the actual CPU/GPU crossover picture for KQSR instead of an asymmetric benchmark.

---

## 3.3.2 Add KQSR signal-build benchmarks

The current bench file is useful for end-to-end optimizer measurements, but it does not tell you how much of KQSR’s time is signal staging versus kernel evaluation.

### Recommended additions

Add two KQSR-specific signal-build groups:

1. **CPU fast+slow signal build**
2. **GPU fast+slow signal build**
   - one case reusing `PreparedCudaIndicatorInputs`
   - one case preparing CUDA inputs every iteration

### Suggested benchmark sketch

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_kqsr_signal_build(c: &mut Criterion) {
    if !bench_target_enabled("kqsr") || !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_kqsr_oscillating_candles(200_000);
    let cfg = KqsrStrategyParams::default();
    let periods = ta_app_core::kqsr::resolve_kqsr_periods(&cfg, 20, 60);

    let mut cpu_group = c.benchmark_group("kqsr/signal_build/cpu/200k");
    cpu_group.measurement_time(Duration::from_secs(6));
    cpu_group.sample_size(10);
    cpu_group.bench_function("fast_plus_slow", |b| {
        b.iter(|| {
            let _fast = ta_app_core::kqsr::compute_kqsr_fast_signals(
                &candles,
                "close",
                &Backend::CpuOnly,
                periods,
                &cfg,
                None,
            )
            .expect("cpu fast");

            let _slow = ta_app_core::kqsr::compute_kqsr_slow_signals(
                &candles,
                "close",
                &Backend::CpuOnly,
                periods,
                None,
            )
            .expect("cpu slow");
        })
    });
    cpu_group.finish();

    let prepared = ta_app_core::dynamic_indicator_adapter::PreparedCudaIndicatorInputs::from_candles_on_device(
        &candles,
        "close",
        0,
    )
    .expect("cuda inputs");

    let mut gpu_group = c.benchmark_group("kqsr/signal_build/gpu/200k");
    gpu_group.measurement_time(Duration::from_secs(6));
    gpu_group.sample_size(10);

    gpu_group.bench_function("reuse_prepared_inputs", |b| {
        b.iter(|| {
            let _fast = ta_app_core::kqsr::compute_kqsr_fast_signals_f32(
                &candles,
                "close",
                &Backend::GpuOnly { device_id: 0 },
                periods,
                &cfg,
                Some(&prepared),
            )
            .expect("gpu fast");

            let _slow = ta_app_core::kqsr::compute_kqsr_slow_signals_f32(
                &candles,
                "close",
                &Backend::GpuOnly { device_id: 0 },
                periods,
                Some(&prepared),
            )
            .expect("gpu slow");
        })
    });

    gpu_group.bench_function("prepare_each_iter", |b| {
        b.iter(|| {
            let prepared = ta_app_core::dynamic_indicator_adapter::PreparedCudaIndicatorInputs::from_candles_on_device(
                &candles,
                "close",
                0,
            )
            .expect("cuda inputs");

            let _fast = ta_app_core::kqsr::compute_kqsr_fast_signals_f32(
                &candles,
                "close",
                &Backend::GpuOnly { device_id: 0 },
                periods,
                &cfg,
                Some(&prepared),
            )
            .expect("gpu fast");

            let _slow = ta_app_core::kqsr::compute_kqsr_slow_signals_f32(
                &candles,
                "close",
                &Backend::GpuOnly { device_id: 0 },
                periods,
                Some(&prepared),
            )
            .expect("gpu slow");
        })
    });

    gpu_group.finish();
}
```

### Why this matters

This benchmark tells you whether the real bottleneck is:

- dynamic indicator prep
- prepared-input staging
- or the later backtest kernel

That is exactly the evidence needed before doing a more invasive multi-output or fully device-resident signal refactor.

---

## 3.3.3 Add route visibility if you thread route stats later

Once KQSR route accounting exists, print it in the KQSR bench output. Even a simple summary like:

- requested GPU: yes
- actual GPU outputs: 8 / 8
- CPU fallbacks: 0

would make the benchmark much easier to trust.

---

## 3.4 `composite_validation_workflow.rs` — give KQSR heavier validation coverage

### Goals

- make KQSR validation evidence large enough to be meaningful
- keep the current validation workflow harness as the default evidence path
- avoid inventing a totally new benchmark before using the existing one better

## 3.4.1 Add `100_000 / 32` KQSR to CPU and GPU validation profiles

### Recommended code edit

In the CPU bench list, add:

```rust
bench_cpu_profile("kqsr", 100_000, make_kqsr_candidates(32));
```

So the CPU tail becomes:

```rust
bench_cpu_profile("base_composite", 50_000, make_candidates(8));
bench_cpu_profile("asr", 50_000, make_asr_candidates(8));
bench_cpu_profile("vcc", 50_000, make_vcc_candidates(8));
bench_cpu_profile("vemr", 50_000, make_vemr_candidates(8));
bench_cpu_profile("rocb", 50_000, make_rocb_candidates(8));
bench_cpu_profile("mkcs", 50_000, make_mkcs_candidates(8));
bench_cpu_profile("kqsr", 50_000, make_kqsr_candidates(8));
bench_cpu_profile("carm", 50_000, make_carm_candidates(8));
bench_cpu_profile("kvdf", 50_000, make_kvdf_candidates(8));
bench_cpu_profile("cdmc", 50_000, make_cdmc_candidates(8));
bench_cpu_profile("kqsr", 100_000, make_kqsr_candidates(32));
bench_cpu_profile("carm", 100_000, make_carm_candidates(32));
bench_cpu_profile("cdmc", 100_000, make_cdmc_candidates(32));
```

In the GPU bench list, add:

```rust
bench_profile("kqsr", 100_000, make_kqsr_candidates(32));
```

So the GPU tail becomes:

```rust
bench_profile("base_composite", 50_000, make_candidates(8));
bench_profile("asr", 50_000, make_asr_candidates(8));
bench_profile("vcc", 50_000, make_vcc_candidates(8));
bench_profile("vemr", 50_000, make_vemr_candidates(8));
bench_profile("rocb", 50_000, make_rocb_candidates(8));
bench_profile("mkcs", 50_000, make_mkcs_candidates(8));
bench_profile("kqsr", 50_000, make_kqsr_candidates(8));
bench_profile("carm", 50_000, make_carm_candidates(8));
bench_profile("kvdf", 50_000, make_kvdf_candidates(8));
bench_profile("cdmc", 50_000, make_cdmc_candidates(8));
bench_profile("kqsr", 100_000, make_kqsr_candidates(32));
bench_profile("carm", 100_000, make_carm_candidates(32));
bench_profile("cdmc", 100_000, make_cdmc_candidates(32));
```

### Why this matters

This is the minimum heavier validation case that brings KQSR into parity with the heavier strategy examples already present.

---

## 3.4.2 Optional stage-2 validation addition: `100_000 / 64`

Once `100_000 / 32` is stable and fast enough for the bench budget, add:

```rust
bench_cpu_profile("kqsr", 100_000, make_kqsr_candidates(64));
bench_profile("kqsr", 100_000, make_kqsr_candidates(64));
```

Do this only after confirming the runtime budget is acceptable.

---

## 3.4.3 Keep route probe output, and extend it if KQSR route stats exist

The current route probes are useful. Once KQSR-specific route accounting exists, print it there too.

This keeps the existing validation harness as the evidence path instead of inventing a new one too early.

---

# 4. Higher-risk optimization opportunities

These are real opportunities, but they should come **after** the safe plan above.

## 4.1 Multi-output indicator dispatch collapse

This is likely important if signal staging remains dominant after phase 1, but it requires adjacent dispatch-layer changes.

Risk:
- broader API surface change
- more moving pieces
- more parity requirements

Reward:
- can materially reduce KQSR signal build cost

## 4.2 Fully device-resident signal outputs from indicator generation to KQSR kernel

Today KQSR requests CUDA `HostF32` indicator outputs. If the broader pipeline can generate the KQSR signal families and keep them device-resident through matrix formation and metrics evaluation, that would remove additional host round-trips.

Risk:
- much larger ownership / lifetime changes
- harder benchmark attribution
- broader changes outside the attached file set

## 4.3 Reduced readback surface (objective-only or top-k device reduction)

If the downstream caller only needs:
- objective values
- top-k
- or a reduced metric subset

then full `Vec<Metrics>` readback is more data than necessary.

Risk:
- API change beyond KQSR-only warm path
- downstream compatibility changes

This is a good later optimization only if profiling shows readback bandwidth is dominating.

## 4.4 CUDA graph capture for repeated fixed-shape launch sequences

Only worth considering after:
- series uploads are resident
- combo uploads are resident
- scratch reuse exists
- route behavior is stable

Otherwise you risk graph-capturing a wasteful sequence.

---

# 5. Benchmark and perf-gate review

This section follows the prompt’s requested benchmark-fidelity focus.

## 5.1 Current evidence quality

### Good

- `gpu_native_expansion_opt.rs` already checks actual backend route before running the GPU benchmark.
- `composite_validation_workflow.rs` already prints route probe output and measures realistic validation workflows.
- KQSR already has both CPU and GPU benchmark surfaces in the repo-facing harnesses you attached.

### Not yet good enough for KQSR-specific conclusions

- CPU and GPU optimization cases are not matched for KQSR at the large pair grid.
- KQSR validation coverage is too light relative to other strategy families.
- There is no KQSR-specific signal-build benchmark in the attached harnesses.
- There is no KQSR-local indicator route accounting, so a “GPU” benchmark can still hide CPU indicator prep.

## 5.2 What to treat as the default evidence path

Before adding any brand-new harness, use these in order:

1. `gpu_native_expansion_opt.rs` with matched CPU/GPU cases
2. `composite_validation_workflow.rs` with heavier KQSR cases
3. optional KQSR signal-build microbench inside `gpu_native_expansion_opt.rs`

That is exactly in line with the map’s “use existing perf gates and benchmark scripts first” rule.

## 5.3 What conclusions should wait until after phase 1

Do **not** make a final claim about whether KQSR should be GPU-preferred or CPU-preferred on current workloads until you have:

- matched CPU/GPU pair grids in `gpu_native_expansion_opt.rs`
- at least `100k / 32` KQSR validation coverage
- route visibility for indicator dispatch (or strong confidence that all KQSR indicator legs stayed on GPU)

---

# 6. Test review

## Current state

What exists now in `kqsr.rs`:
- parameter validation test
- one desired-position behavior test
- one CUDA smoke test

That is helpful, but it is not enough for the refactor plan above.

## Minimum test set before the deeper GPU refactor

### In `kqsr.rs`
- CPU `f32` path vs CPU `f64` reference
- GPU `f32` signal path vs CPU `f64` reference

### In `kqsr_gpu_kernel.rs`
- combo bounds validation test
- optional uploaded-series/uploaded-combo reuse consistency test later

## Optional later tests

If you implement the uploaded warm path:
- run the same uploaded series + same uploaded combo batch twice and assert stable metrics
- run old wrapper vs new warm-path API and assert equal metrics

Those tests are especially valuable before and after introducing scratch reuse.

---

# 7. Suggested improvement plan

This is the implementation order I recommend.

## Phase 0 — add tests first

1. Add the `kqsr.rs` parity helpers/tests
2. Add the `kqsr_gpu_kernel.rs` combo-bounds validation test

Why first:
- it lowers the risk of all later edits
- it makes the fallback and FP32 changes safer

## Phase 1 — low-risk file-local wins

### `kqsr.rs`
- add `series_to_f32(...)`
- change `compute_indicator_f32(...)` to use direct CPU compute
- replace repeated `first_valid` scans with one-pass helpers
- tighten `build_kqsr_desired_positions(...)` indexing

### `kqsr_gpu_kernel.rs`
- add stronger combo bounds validation
- add output scratch reuse
- switch `host_metrics` to shared uninitialized host readback helper **only after confirming full write coverage**

Expected result:
- immediate reduction in avoidable CPU prep and allocation churn
- no major public API changes yet

## Phase 2 — explicit warm path for repeated GPU evaluation

### `kqsr_gpu_kernel.rs`
- add `KqsrGpuSignalDeviceMatrices`
- add `KqsrGpuUploadedSeries`
- add `KqsrGpuUploadedComboBatch`
- add `upload_kqsr_series(...)`
- add `upload_kqsr_combo_batch(...)`
- add `eval_kqsr_metrics_gpu_kernel_uploaded(...)`
- keep the old wrapper API delegating to the new path

Expected result:
- biggest warm-path improvement in repeated optimization/validation workloads

## Phase 3 — improve benchmark evidence

### `gpu_native_expansion_opt.rs`
- add matched KQSR CPU case list including the large case
- add KQSR signal-build microbench

### `composite_validation_workflow.rs`
- add KQSR `100k / 32`
- optionally add KQSR `100k / 64`

Expected result:
- trustworthy KQSR-specific CPU/GPU payoff evidence

## Phase 4 — optional route instrumentation

### `kqsr.rs`
- add optional indicator route summary capture
- print it in KQSR-specific bench/debug paths

Expected result:
- clearer route-matrix evidence
- easier interpretation of “GPU” benchmark runs

## Phase 5 — only if phase 3 still shows signal staging dominating

- pursue multi-output dispatch collapse
- consider broader device-resident signal output plumbing

Do not do this until the safer plan above is measured.

---

# 8. Avoidable vs unavoidable copy-boundary map

This section is explicitly aligned with the bundle-16 review rules.

| Location | Current behavior | Avoidable? | Recommendation |
|---|---|---:|---|
| `kqsr_gpu_kernel.rs` `d_close = DeviceBuffer::from_slice(signals.close_f32)` | uploads close series every evaluation | **Yes, when series is reused** | move to `upload_kqsr_series(...)` and keep resident per dataset |
| `kqsr_gpu_kernel.rs` `d_lr` + `compute_log_returns_f32` every evaluation | recomputes log returns every evaluation | **Yes, when series is reused** | compute once during series upload |
| `kqsr_gpu_kernel.rs` combo uploads | uploads combo indices/start/end every evaluation | **Yes, when candidate batch is reused** | move to `upload_kqsr_combo_batch(...)` |
| `kqsr_gpu_kernel.rs` `d_out` allocation | allocates device output every evaluation | **Yes** | reuse scratch `d_out` buffer |
| `kqsr_gpu_kernel.rs` `vec![0.0f32; out_len]` before `copy_to(...)` | full host zero-fill before device overwrite | **Yes, if kernel fully writes all elements** | use shared uninitialized host readback helper |
| `kqsr.rs` CPU fallback `f64 -> f32` | full-vector cast after CPU compute | **Yes** | direct `series_to_f32(...)` |
| `kqsr.rs` `desired = vec![0i32; n]` | zero-fill of desired positions | **No** | keep as-is; zeros are observable behavior |
| `kqsr.rs` indicator outputs | externally produced vectors returned from dispatch layer | **Not from this file alone** | focus on eliminating duplicate dispatch / reuse, not local `MaybeUninit` |
| signal matrix device pointers in `kqsr_gpu_kernel.rs` | already passed as device pointers | **Already good** | keep zero-copy behavior here |

## Important practical rule

Not every copy is an allocation bug.

Examples:
- Host→device upload of a series that exists only on the host **once** per dataset is usually unavoidable.
- Host→device upload of the same unchanged series for every evaluation is avoidable.
- Device→host readback of full metrics is unavoidable if the caller truly needs every metric row on the host.
- The zero-fill of the host readback vector before the device overwrites it is avoidable.

---

# 9. Open questions to verify during implementation

These are the main things the attached files do not fully answer.

## 9.1 Does the KQSR metrics kernel write the full output row on every path?

This must be confirmed before switching `host_metrics` to uninitialized allocation.

## 9.2 Does the adjacent dynamic-indicator layer already cache or reuse work across different `output_id` requests?

The KQSR call-site definitely issues duplicate dispatches, but the exact extra cost depends on the implementation behind those calls.

## 9.3 Is a shared scratch helper already public enough to reuse?

If there is already a public shared scratch type in the adjacent GPU runtime code, use that instead of creating a KQSR-only scratch holder.

---

# 10. Acceptance criteria

These are the success criteria I would use for each phase.

## Phase 1 acceptance

- all new parity tests pass
- `compute_kqsr_fast_signals_f32(..., Backend::CpuOnly, ...)` matches CPU `f64` reference within tolerance
- `build_kqsr_desired_positions(...)` behavior unchanged on existing tests
- no public API break

## Phase 2 acceptance

- old wrapper path still works unchanged
- uploaded warm-path API produces identical metrics to the wrapper path
- repeated evaluation on the same series/candidate batch no longer re-uploads close or combos
- `d_out` allocation is not repeated for same-or-smaller output sizes

## Phase 3 acceptance

- CPU and GPU KQSR optimizer benches include the same pair-grid cases
- validation workflow includes KQSR `100k / 32`
- benchmark output is sufficient to decide whether KQSR should be CPU- or GPU-preferred on the measured cases

## Phase 4 acceptance

- strategy-local route output clearly distinguishes requested GPU vs actual GPU success vs CPU fallback
- KQSR benchmark output is no longer ambiguous when signal prep falls back

---

# 11. Implementation checklist

Use this section as the tracking list while you apply the changes.

## `kqsr.rs`

- [ ] Add `series_to_f32(...)`
- [ ] Change `compute_indicator_f32(...)` to use direct CPU compute instead of `compute_indicator_f64(...).map(...)`
- [ ] Add `first_valid_quad_f64(...)`
- [ ] Add `first_valid_quad_f32(...)`
- [ ] Replace repeated `first_finite_idx_local(...)` chains in:
  - [ ] `compute_kqsr_fast_signals(...)`
  - [ ] `compute_kqsr_slow_signals(...)`
  - [ ] `compute_kqsr_fast_signals_f32(...)`
  - [ ] `compute_kqsr_slow_signals_f32(...)`
- [ ] Tighten `build_kqsr_desired_positions(...)`:
  - [ ] keep `desired = vec![0i32; n]`
  - [ ] add series-length assertions or checks
  - [ ] switch loop body to direct indexing
- [ ] Add test helper `assert_series_close_f64_f32(...)`
- [ ] Add CPU `f32` path parity test
- [ ] Add GPU signal parity test
- [ ] Optional: add `KqsrIndicatorRouteSummary`
- [ ] Optional: thread route accounting through indicator helpers for bench/debug builds
- [ ] Optional later: plan adjacent multi-output indicator dispatch support

## `kqsr_gpu_kernel.rs`

- [ ] Add `KqsrGpuSignalDeviceMatrices`
- [ ] Add `device_view()` on `KqsrGpuDeviceSignalMatrices<'_>`
- [ ] Add `KqsrGpuUploadedSeries`
- [ ] Add `KqsrGpuUploadedComboBatch`
- [ ] Add `upload_kqsr_series(...)`
- [ ] Add `upload_kqsr_combo_batch(...)`
- [ ] Add `KqsrScratch`
- [ ] Add reusable `d_out` scratch allocation
- [ ] Add `eval_kqsr_metrics_gpu_kernel_uploaded(...)`
- [ ] Make `eval_kqsr_metrics_gpu_kernel_device_rows(...)` delegate to the new warm path
- [ ] Split validation into:
  - [ ] `validate_signal_device_view(...)`
  - [ ] `validate_combo_vectors(...)`
  - [ ] wrapper `validate_inputs(...)`
- [ ] Add bounds checks for:
  - [ ] `fast_row_index`
  - [ ] `slow_row_index`
  - [ ] `start_t`
  - [ ] `end_t`
- [ ] Convert unchecked `as i32` casts to `i32::try_from(...)` where practical
- [ ] Confirm full device write coverage for output rows
- [ ] Replace `vec![0.0f32; out_len]` with shared uninitialized host readback helper only after that confirmation
- [ ] Optional: cache launch-cap metadata in runtime
- [ ] Add combo-bounds validation unit test
- [ ] Optional later: add wrapper-vs-uploaded parity test
- [ ] Optional later: add repeated-uploaded-launch consistency test

## `gpu_native_expansion_opt.rs`

- [ ] Add `BenchCase` alias (or equivalent)
- [ ] Add `bench_cpu_cases_with_cases(...)`
- [ ] Add `KQSR_CASES` with small / medium / large pair grids
- [ ] Update `bench_kqsr_cpu(...)` to use matched cases
- [ ] Keep existing GPU KQSR bench unchanged unless you also want case helper reuse
- [ ] Add KQSR signal-build CPU microbench
- [ ] Add KQSR signal-build GPU microbench
- [ ] Add GPU signal-build split:
  - [ ] `reuse_prepared_inputs`
  - [ ] `prepare_each_iter`
- [ ] Add new benchmark(s) to the `criterion_group!`

## `composite_validation_workflow.rs`

- [ ] Add CPU KQSR validation profile at `100_000 / 32`
- [ ] Add GPU KQSR validation profile at `100_000 / 32`
- [ ] Optional: add CPU/GPU KQSR `100_000 / 64`
- [ ] Keep existing route probe printing
- [ ] Optional: print KQSR indicator route summary when available
- [ ] Re-run the validation workflow benches and compare KQSR against current `50_000 / 8`

## Verification and rollout

- [ ] Run `kqsr.rs` tests after phase 1
- [ ] Run `kqsr_gpu_kernel.rs` tests after validation changes
- [ ] Verify old wrapper path still works before switching any caller to uploaded warm-path APIs
- [ ] Compare wrapper vs uploaded metrics on the same series/candidates
- [ ] Re-run `gpu_native_expansion_opt.rs`
- [ ] Re-run `composite_validation_workflow.rs`
- [ ] Confirm whether KQSR stays GPU-preferred or CPU-preferred on the measured workloads
- [ ] Only then decide whether to pursue multi-output dispatch collapse or deeper device-resident signal work

---

# 12. Bottom line

If you only implement the highest-value, lowest-risk subset first, do these in order:

1. direct CPU `series_to_f32(...)` fallback in `kqsr.rs`
2. one-pass `first_valid` helpers
3. tighter `build_kqsr_desired_positions(...)`
4. uploaded KQSR series + uploaded combo batches in `kqsr_gpu_kernel.rs`
5. reusable `d_out`
6. uninitialized host readback **only after full-write confirmation**
7. matched KQSR CPU/GPU optimizer benches
8. heavier KQSR validation coverage

That sequence should give you the cleanest path to less staging waste, fewer unnecessary copies, and much better evidence about KQSR’s real CUDA payoff.


---

# 10. Addendum — additional improvements, hardening, and beyond-scope-but-safe options

This addendum tightens the original plan in five areas that deserve more explicit treatment:

1. **stream-aware async upload/allocation/readback**
2. **pinned host readback when full readback is still required**
3. **stricter shape / overflow validation for correctness**
4. **optional stride padding and combo ordering for locality**
5. **API hardening around raw device pointers and helper reuse**

These additions do **not** replace the main plan. They extend it.

## 10.1 What the original plan already covered correctly

The main document already covered the highest-value KQSR issues:

- repeated `close_f32` upload and repeated log-return generation in `kqsr_gpu_kernel.rs`
- repeated combo uploads and repeated `d_out` allocation in `kqsr_gpu_kernel.rs`
- duplicate multi-output indicator dispatches in `kqsr.rs`
- fallback opacity in `compute_indicator_f32(...)` / `compute_indicator_f64(...)`
- CPU benchmark mismatch in `gpu_native_expansion_opt.rs`
- light KQSR coverage in `composite_validation_workflow.rs`
- host-side combo-bounds validation needs
- zero-filled host readback buffer before `copy_to(...)`

Those are still the primary phase-1 and phase-2 priorities.

## 10.2 Additional item A — make the KQSR warm path stream-aware and async where possible

### Why this deserves explicit inclusion

The original plan correctly proposed resident series, resident combos, and reusable `d_out`, but it did not go far enough in calling out that the **current KQSR code is using synchronous host-to-device upload and synchronous device allocation even though `KqsrCudaRuntime` already owns a non-blocking stream**.

That means even after introducing resident objects, the convenience path and first-upload path can still leave performance on the table unless they adopt the same stream-aware patterns already seen elsewhere in the repo.

### Confirmed source basis

- `KqsrCudaRuntime` already owns `Stream::new(StreamFlags::NON_BLOCKING, None)`.
- `eval_kqsr_metrics_gpu_kernel_device_rows(...)` currently uses `DeviceBuffer::from_slice(...)` and `DeviceBuffer::<f32>::uninitialized(...)` for close, combos, and `d_out`.
- Adjacent code already demonstrates `from_slice_async(...)`, `uninitialized_async(...)`, and pinned host async copies.

### Recommended code edits

#### 10.2.1 Import the pinned-host type and shared async-allocation gating

At the top of `kqsr_gpu_kernel.rs`, extend imports along these lines:

```rust
use cust::memory::{CopyDestination, DeviceBuffer, LockedBuffer};

use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates,
    detect_device_launch_caps,
    detect_kernel_launch_caps,
    load_double_crossover_module,
    kernel_async_alloc_enabled, // if public in your crate layout
};
```

If `kernel_async_alloc_enabled()` is not publicly exported from the shared module, mirror its repo-standard behavior with the smallest possible local helper instead of introducing a new policy.

#### 10.2.2 Add a small allocation helper for stream-aware device buffers

```rust
#[inline]
fn alloc_device_f32_for_stream(
    len: usize,
    stream: &Stream,
) -> Result<DeviceBuffer<f32>, String> {
    if kernel_async_alloc_enabled() {
        unsafe { DeviceBuffer::<f32>::uninitialized_async(len, stream) }
            .map_err(|e| e.to_string())
    } else {
        unsafe { DeviceBuffer::<f32>::uninitialized(len) }
            .map_err(|e| e.to_string())
    }
}
```

If you need the same logic for `i32`, add the parallel helper rather than duplicating the allocation branch everywhere.

#### 10.2.3 Make the upload helpers async on the runtime stream

Prefer this shape for the convenience wrapper and the uploaded-series/uploaded-combo helpers:

```rust
pub fn upload_kqsr_series(
    device_id: u32,
    close_f32: &[f32],
) -> Result<KqsrGpuUploadedSeries, String> {
    if close_f32.is_empty() {
        return Err("KQSR GPU kernel requires non-empty close series".to_string());
    }

    KQSR_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, KqsrCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "KQSR GPU runtime missing".to_string())?;
        rt.set_current()?;

        let d_close = unsafe { DeviceBuffer::from_slice_async(close_f32, &rt.stream) }
            .map_err(|e| e.to_string())?;
        let mut d_lr = alloc_device_f32_for_stream(close_f32.len(), &rt.stream)?;

        launch_compute_log_returns_f32(rt, device_id, &d_close, &mut d_lr, close_f32.len())?;
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        Ok(KqsrGpuUploadedSeries {
            cols: close_f32.len(),
            d_close,
            d_lr,
        })
    })
}
```

And for combos:

```rust
pub fn upload_kqsr_combo_batch(
    device_id: u32,
    signals: &KqsrGpuSignalDeviceMatrices,
    combos: &KqsrGpuComboBatch<'_>,
) -> Result<KqsrGpuUploadedComboBatch, String> {
    validate_signal_device_view(signals)?;
    validate_combo_vectors(signals, combos)?;

    KQSR_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, KqsrCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "KQSR GPU runtime missing".to_string())?;
        rt.set_current()?;

        let d_fast_row_index = unsafe {
            DeviceBuffer::from_slice_async(combos.fast_row_index, &rt.stream)
        }
        .map_err(|e| e.to_string())?;
        let d_slow_row_index = unsafe {
            DeviceBuffer::from_slice_async(combos.slow_row_index, &rt.stream)
        }
        .map_err(|e| e.to_string())?;
        let d_start_t = unsafe { DeviceBuffer::from_slice_async(combos.start_t, &rt.stream) }
            .map_err(|e| e.to_string())?;
        let d_end_t = unsafe { DeviceBuffer::from_slice_async(combos.end_t, &rt.stream) }
            .map_err(|e| e.to_string())?;

        rt.stream.synchronize().map_err(|e| e.to_string())?;

        Ok(KqsrGpuUploadedComboBatch {
            len: combos.fast_row_index.len(),
            d_fast_row_index,
            d_slow_row_index,
            d_start_t,
            d_end_t,
        })
    })
}
```

### Why this matters

Even if you still synchronize once at the end of the upload helper, this removes some unnecessary host-side blocking and keeps the KQSR code aligned with the stream-aware patterns that are already present elsewhere in the repo.

### Safety notes

- Keep synchronization in the upload helper until you are certain no caller can use the object before the upload/compute is complete.
- Do **not** rely on implicit synchronization from later operations to establish correctness.

## 10.3 Additional item B — replace pageable readback with pinned uninitialized host memory when readback is still required

### Why this deserves explicit inclusion

The original plan already called out that `vec![0.0f32; out_len]` is wasted before `copy_to(...)`. That is true, but there is a second improvement on top of that: **if KQSR still needs full metrics readback, use pinned host memory instead of pageable `Vec<f32>` for the device-to-host transfer**.

This is particularly relevant because the current KQSR path synchronizes and then performs a full metrics readback into pageable memory.

### Recommended code edit

Instead of this current shape:

```rust
rt.stream.synchronize().map_err(|e| e.to_string())?;
let mut host_metrics = vec![0.0f32; out_len];
d_out.copy_to(host_metrics.as_mut_slice()).map_err(|e| e.to_string())?;

let mut out: Vec<Metrics> = Vec::with_capacity(combo_count);
for row in host_metrics.chunks_exact(KQSR_METRICS_STRIDE) {
    out.push(decode_metrics_row(row));
}
```

prefer this:

```rust
let mut host_metrics: LockedBuffer<f32> =
    unsafe { LockedBuffer::uninitialized(out_len) }.map_err(|e| e.to_string())?;

unsafe {
    d_out
        .async_copy_to(host_metrics.as_mut_slice(), &rt.stream)
        .map_err(|e| e.to_string())?;
}

rt.stream.synchronize().map_err(|e| e.to_string())?;

let mut out: Vec<Metrics> = Vec::with_capacity(combo_count);
for row in host_metrics.as_slice().chunks_exact(KQSR_METRICS_STRIDE) {
    out.push(decode_metrics_row(row));
}
```

### Why this is better

- removes the wasted zero-fill
- can improve D2H throughput by using pinned host memory
- lets you decode directly from the pinned buffer without first copying into a second `Vec<f32>`

### Safety notes

- This is only safe because `async_copy_to(...)` fully overwrites the destination slice before any read and you synchronize before decoding.
- Keep a plain zero-filled `Vec` only if some code path reads the buffer before the device write completes (which would be a correctness bug anyway).

## 10.4 Additional item C — strengthen shape and overflow validation beyond what the original plan spelled out

The existing source validation checks only non-empty input, non-null pointers, non-zero shapes, and combo-length equality.

The main plan already added combo index and start/end checks. Two more checks should be added explicitly.

### 10.4.1 Require `stride >= cols`

Because the kernel takes `fast_stride` and `slow_stride` separately from `cols`, the most natural contract is that stride is the physical row width and must be at least the logical row width.

Add this to `validate_signal_device_view(...)`:

```rust
if signals.fast_stride < signals.cols {
    return Err(format!(
        "KQSR GPU fast_stride {} is smaller than cols {}",
        signals.fast_stride,
        signals.cols
    ));
}
if signals.slow_stride < signals.cols {
    return Err(format!(
        "KQSR GPU slow_stride {} is smaller than cols {}",
        signals.slow_stride,
        signals.cols
    ));
}
```

### 10.4.2 Check row-stride products for overflow even if you cannot validate the raw allocation size

Because the current signal matrix struct carries raw `u64` pointers rather than typed device buffers, you cannot prove the backing allocation length from this file alone. But you **can** still reject impossible or obviously unsafe arithmetic:

```rust
let _fast_elems = signals
    .fast_rows
    .checked_mul(signals.fast_stride)
    .ok_or_else(|| "KQSR GPU fast matrix rows*stride overflow".to_string())?;
let _slow_elems = signals
    .slow_rows
    .checked_mul(signals.slow_stride)
    .ok_or_else(|| "KQSR GPU slow matrix rows*stride overflow".to_string())?;
```

### 10.4.3 Convert every launch scalar cast to checked conversion

The main plan already proposed `i32::try_from(...)` for the obvious casts, but it is worth elevating this to a **must-do correctness item**. In the current source, `signals.cols`, `fast_rows`, `fast_stride`, `slow_rows`, `slow_stride`, and `combo_count` are cast with `as i32`.

Keep the launch block like this:

```rust
let mut t_i = i32::try_from(signals.cols)
    .map_err(|_| "KQSR cols exceeds i32 range".to_string())?;
let mut fast_rows_i = i32::try_from(signals.fast_rows)
    .map_err(|_| "KQSR fast_rows exceeds i32 range".to_string())?;
let mut fast_stride_i = i32::try_from(signals.fast_stride)
    .map_err(|_| "KQSR fast_stride exceeds i32 range".to_string())?;
let mut slow_rows_i = i32::try_from(signals.slow_rows)
    .map_err(|_| "KQSR slow_rows exceeds i32 range".to_string())?;
let mut slow_stride_i = i32::try_from(signals.slow_stride)
    .map_err(|_| "KQSR slow_stride exceeds i32 range".to_string())?;
let mut combo_count_i = i32::try_from(combos.len)
    .map_err(|_| "KQSR combo_count exceeds i32 range".to_string())?;
let mut out_stride_i = i32::try_from(KQSR_METRICS_STRIDE)
    .map_err(|_| "KQSR metrics stride exceeds i32 range".to_string())?;
```

### Why this matters

This is not speculative. Silent integer narrowing in launch arguments is a real correctness risk.

## 10.5 Additional item D — optional row-stride padding / alignment for KQSR matrices

### Why this is interesting

The kernel interface already takes `fast_stride` and `slow_stride` independently from `cols`. That strongly suggests the KQSR path can already tolerate padded rows if the matrix builders choose to provide them.

If Nsight or perf traces show row-start misalignment or poor global-load efficiency, **padding each row to an alignment boundary** can be a real win.

### Recommended scope

Do **not** make this a phase-1 change.

Do this only if:

- the warm-path upload cleanup is already done
- duplicate signal dispatch work is already reduced
- profiling still shows the kernel itself as the dominant cost

### Suggested shape

Use the shared alloc-policy helpers if they already exist and are public in your crate layout. The generic pattern is:

```rust
let padded_cols = cols.next_multiple_of(32); // or 64, benchmark-guided
let len = rows
    .checked_mul(padded_cols)
    .ok_or_else(|| "KQSR padded matrix len overflow".to_string())?;

// Pseudocode: adapt to actual alloc_policy helper names in your repo.
let mut matrix = make_uninit_matrix_f32(rows, padded_cols)?;
init_matrix_prefixes_f32(&mut matrix, rows, padded_cols, cols, f32::NAN)?;

for row in 0..rows {
    let src = &row_values[row * cols..(row + 1) * cols];
    let dst = &mut matrix[row * padded_cols..row * padded_cols + cols];
    dst.copy_from_slice(src);
}
```

Then pass `cols` as the logical time dimension and `padded_cols` as the physical stride.

### Safety rule

Only the padded tail may remain uninitialized if the kernel will never read it. If the kernel iterates to `stride` rather than `cols`, you must initialize the tail. In practice, KQSR should keep `cols` as the loop bound and `stride` only as row-step distance.

### Expected payoff

- possible better row-start alignment
- possible more regular memory coalescing
- especially relevant when many rows are scanned sequentially by the kernel

### Risk

- requires touching the matrix-builder side, not just `kqsr_gpu_kernel.rs`
- requires explicit parity tests because the physical layout changes

## 10.6 Additional item E — optional combo reordering for signal-row locality

### Why this is interesting

If many KQSR combos reuse the same signal rows with different `start_t` / `end_t` windows, processing combos in arbitrary order can cause avoidable cache churn on the device.

A stable reorder by row index can improve locality while preserving final output order via inverse permutation.

### Suggested host-side shape

```rust
#[derive(Clone)]
struct PermutedCombo {
    orig_idx: usize,
    fast: i32,
    slow: i32,
    start_t: i32,
    end_t: i32,
}

fn reorder_combos_for_locality(combos: &KqsrGpuComboBatch<'_>) -> Vec<PermutedCombo> {
    let mut rows: Vec<PermutedCombo> = (0..combos.fast_row_index.len())
        .map(|i| PermutedCombo {
            orig_idx: i,
            fast: combos.fast_row_index[i],
            slow: combos.slow_row_index[i],
            start_t: combos.start_t[i],
            end_t: combos.end_t[i],
        })
        .collect();

    rows.sort_by_key(|r| (r.fast, r.slow, r.start_t, r.end_t));
    rows
}
```

Upload the reordered combo vectors, run the kernel, then restore result order:

```rust
fn invert_permutation<T: Clone>(ordered: Vec<(usize, T)>) -> Vec<T> {
    let mut out: Vec<Option<T>> = vec![None; ordered.len()];
    for (orig_idx, value) in ordered {
        out[orig_idx] = Some(value);
    }
    out.into_iter().map(|v| v.expect("filled")).collect()
}
```

### When to try this

Try it only as an optional benchmark branch after the uploaded warm path exists.

### Risk

- extra host-side work for sorting and inverse permutation
- may not help if combo reuse is low

### Why it is still worth mentioning

This is a **feature-preserving** optimization: the public API and returned metrics do not change.

## 10.7 Additional item F — optional API hardening for raw device-pointer correctness

### Why this deserves mention

`KqsrGpuDeviceSignalMatrices` currently carries raw `u64` pointers for the indicator matrices. That is lightweight, but it weakens correctness guarantees:

- no device provenance
- no backing-allocation length proof
- easy accidental cross-device misuse
- no type distinction between row-major matrix pointers and arbitrary addresses

### Optional hardening path

Without changing the current public wrapper immediately, add an internal typed view for the warm path:

```rust
pub struct KqsrGpuSignalDeviceMatricesTyped {
    pub device_id: u32,
    pub cols: usize,
    pub fast_rows: usize,
    pub fast_stride: usize,
    pub slow_rows: usize,
    pub slow_stride: usize,
    pub j: cust::memory::DevicePointer<f32>,
    pub srsi_k: cust::memory::DevicePointer<f32>,
    pub srsi_d: cust::memory::DevicePointer<f32>,
    pub ift_rsi: cust::memory::DevicePointer<f32>,
    pub qqe_fast: cust::memory::DevicePointer<f32>,
    pub qqe_slow: cust::memory::DevicePointer<f32>,
    pub prb_upper: cust::memory::DevicePointer<f32>,
    pub prb_lower: cust::memory::DevicePointer<f32>,
}
```

Then keep the current raw-pointer struct as the compatibility wrapper that converts into the typed internal view.

### Why this helps

It does not magically prove allocation sizes, but it does make wrong-device and wrong-type mistakes harder.

## 10.8 Additional item G — explicitly prefer shared repo helpers before introducing KQSR-local unsafe code

The prompt map explicitly says to prefer shared helpers from `alloc_policy.rs` before adding new local unsafe allocation code.

The KQSR plan should therefore add this rule of implementation order:

1. **reuse shared alloc-policy helpers if public and suitable**
2. **reuse shared upload/readback helpers if public and suitable**
3. **only then add a KQSR-local helper**

Concrete candidates to check before adding local code:

- host uninitialized copy/readback helpers used by broader GPU paths
- stream-aware `DeviceBuffer::<T>::uninitialized_async(...)` gating patterns
- generic upload helpers for ephemeral `f32` series
- scratch-buffer helper types already exposed by nearby GPU modules

### Why this matters

This reduces maintenance cost and keeps KQSR aligned with the repo’s allocation-policy direction rather than growing a strategy-specific unsafe island.

## 10.9 Additional benchmark suggestions that go beyond the prompt but are still worth doing

These are optional, but they would make the performance story much stronger without changing product behavior.

### 10.9.1 Warm uploaded path vs convenience wrapper

Add a Criterion group that measures:

- wrapper path (upload everything every call)
- uploaded series + uploaded combos + reusable `d_out`
- uploaded series + uploaded combos + reusable `d_out` + pinned async readback

This quantifies exactly how much of the KQSR gain comes from copy-elision rather than kernel math.

### 10.9.2 Sync vs async upload/readback branch

If you make the async path optional at compile time or behind a small local flag, benchmark:

- sync upload/readback
- async upload/readback with single final synchronize

This helps validate whether the stream-aware path is actually winning in your environment.

### 10.9.3 Grouped combos vs original order

If you add locality-based combo ordering, benchmark:

- original candidate order
- grouped-by-row candidate order + inverse permutation

This reveals whether reordering is worthwhile for your real KQSR candidate shapes.

### 10.9.4 Padded stride vs compact stride

If you add padded row-major matrices, benchmark:

- compact `stride == cols`
- padded `stride == next_multiple_of(32 or 64)`

Use Nsight or another memory-efficiency diagnostic before keeping this permanently.

## 10.10 Updated implementation priority after this addendum

The priority stack becomes:

### Phase 1 — still first

- direct CPU-to-`f32` fallback in `kqsr.rs`
- route accounting for KQSR indicator dispatches
- combo bounds validation
- reusable `d_out`
- resident `d_close` / `d_lr`
- resident combo uploads

### Phase 1.5 — add now if touching the same code anyway

- `from_slice_async(...)` / `uninitialized_async(...)` where supported
- pinned uninitialized host readback with `LockedBuffer`
- `stride >= cols` validation
- `rows * stride` checked overflow validation
- complete `i32::try_from(...)` coverage for launch args

### Phase 2 — only after parity is stable

- multi-output indicator dispatch collapse in adjacent dispatch layer
- combo reordering for locality
- padded row strides
- typed internal device-pointer view

### Phase 3 — only if profiling still points there

- reduced readback surface (objective-only / top-k)
- full device-resident indicator output pipeline
- CUDA graph capture

## 10.11 Checklist addendum

Add these to the bottom checklist.

### `kqsr_gpu_kernel.rs` addendum checklist

- [ ] Import `LockedBuffer` for pinned host readback if available in current `cust` version
- [ ] Prefer `kernel_async_alloc_enabled()` or existing shared policy before adding any KQSR-local async-allocation switch
- [ ] Switch upload helpers from `from_slice(...)` to `from_slice_async(...)` where supported
- [ ] Switch reusable device allocations from `uninitialized(...)` to `uninitialized_async(...)` where supported
- [ ] Replace pageable host metrics readback with pinned uninitialized host readback when full readback remains necessary
- [ ] Decode metrics directly from the pinned host slice instead of copying through a second host vector
- [ ] Add `fast_stride >= cols` validation
- [ ] Add `slow_stride >= cols` validation
- [ ] Add `fast_rows.checked_mul(fast_stride)` overflow validation
- [ ] Add `slow_rows.checked_mul(slow_stride)` overflow validation
- [ ] Ensure every launch scalar uses checked conversion instead of `as i32`
- [ ] Keep the current wrapper API but make the uploaded warm path the performance-first route internally

### `kqsr.rs` addendum checklist

- [ ] Keep the duplicate-dispatch collapse on the roadmap, but do not block phase-1 work on adjacent dispatch-layer changes
- [ ] If signal matrices are later padded, ensure the matrix-builder side distinguishes logical `cols` from physical `stride`
- [ ] Add parity tests that explicitly cover any padded-row representation if introduced later

### Benchmark / validation addendum checklist

- [ ] Add a warm uploaded-path benchmark branch
- [ ] Add a pinned-readback benchmark branch if implemented
- [ ] Add a grouped-combo benchmark branch if combo reordering is implemented
- [ ] Add a padded-stride benchmark branch if padded rows are implemented
- [ ] Keep route summary output in all KQSR-specific GPU benchmark branches

## 10.12 Final recommendation on scope

If you want the **best performance-per-risk** path, stop at:

- resident `d_close` / `d_lr`
- resident combos
- reusable `d_out`
- direct CPU-to-`f32` fallback
- route accounting
- pinned uninitialized host readback
- full bounds / stride / overflow validation

That set keeps functionality intact, removes a large amount of avoidable staging and allocation churn, improves correctness, and does not require a broad strategy redesign.
