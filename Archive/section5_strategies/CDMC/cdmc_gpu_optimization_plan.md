# CDMC strategy-local GPU-native optimization and validation plan

Prepared from `gpt54_feature_review_map.md`, scoped to **Bundle 5: Strategy-local GPU-native optimization and validation routes**, and limited to the **CDMC strategy**.

This document intentionally follows the bundle prompt structure and adds an implementation plan, concrete code-edit sketches, memory/copy-elision guidance, benchmark updates, and a detailed implementation checklist.

---

## Surface under review

This review covers the CDMC strategy’s strategy-local GPU-native optimization and validation route, with emphasis on:

- strategy-local hot-path cost
- CPU/GPU payoff and staging overhead
- fallback transparency and parity safety
- validation throughput, not just optimizer throughput
- allocation discipline, unnecessary copies, and safe use of uninitialized memory where possible

The concrete surface here is the combination of:

- device-native CDMC signal preparation in `cdmc.rs`
- CDMC CUDA metrics evaluation in `cdmc_gpu_kernel.rs`
- CDMC optimizer benchmarks in `gpu_native_expansion_opt.rs`
- CDMC validation benchmarks in `composite_validation_workflow.rs`

---

## Files reviewed

Primary files reviewed in this pass:

- `gpt54_feature_review_map.md` (bundle scope and prompt rules)
- `cdmc.rs`
- `cdmc_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

Important line anchors used throughout this plan:

- Bundle 5 scope and prompt rules: `gpt54_feature_review_map.md:235-273`, `gpt54_feature_review_map.md:725-818`
- CDMC device-prep and host/GPU helper paths: `cdmc.rs:335-558`, `cdmc.rs:561-799`, `cdmc.rs:814-925`, `cdmc.rs:957-1020`
- CDMC CUDA metrics launcher/runtime: `cdmc_gpu_kernel.rs:24-342`
- CDMC optimizer benchmark surface: `gpu_native_expansion_opt.rs:16-52`, `gpu_native_expansion_opt.rs:209-219`, `gpu_native_expansion_opt.rs:253-325`, `gpu_native_expansion_opt.rs:363-425`
- CDMC validation benchmark surface: `composite_validation_workflow.rs:297-329`, `composite_validation_workflow.rs:632-823`

---

## Extra context

The bundle prompt for this review says to focus on:

- one strategy at a time
- strategy-local staging or upload waste
- CPU-reference prep that can erase GPU benefit
- per-strategy fallback reasons and fp32-stability blockers
- strategy-specific validation throughput
- whether the strategy should remain CPU-preferred on current measured workloads

The prompt rules also require:

- separating confirmed issues from likely issues, open questions, and optimization opportunities
- identifying the relevant timing boundary for each claim
- preferring parity-safe, benchmark-backed changes before broad refactors
- using existing benchmark harnesses before inventing new instrumentation

That is the structure used below.

---

# 1. Surface understanding

CDMC already has the right **basic building blocks** for a strong GPU-native route.

In `cdmc.rs`, you already expose device-oriented helpers for:

- DPO sweeps via `compute_cdmc_dpo_sweep_cuda_device(...)` (`cdmc.rs:400-418`)
- Correlation-cycle output sweeps via `compute_cdmc_correlation_cycle_output_sweep_cuda_device(...)` (`cdmc.rs:421-444`)
- Decycler sweeps via `compute_cdmc_decycler_sweep_cuda_device(...)` (`cdmc.rs:447-469`)
- MAMA/FAMA device outputs via `compute_cdmc_mama_pair_cuda_device(...)` (`cdmc.rs:472-558`)

That means CDMC is **not** missing a GPU route. The main remaining problem is that the existing route still pays too much in:

- repeated host/device staging
- redundant uploads
- repeated per-call scratch allocation
- host-side reference work that partially defeats the device-native path
- benchmark/reporting gaps that can hide fallback or route drift

`cdmc_gpu_kernel.rs` then consumes device pointers for the signal matrices and launches `cdmc_backtest_metrics_f32` (`cdmc_gpu_kernel.rs:175-342`). That is a good shape for repeated multi-combo evaluation, but the current launcher still performs avoidable work around the kernel:

- it re-uploads `close` every call (`cdmc_gpu_kernel.rs:211`)
- it re-allocates `d_lr` every call (`cdmc_gpu_kernel.rs:212-214`)
- it re-uploads combo arrays every call (`cdmc_gpu_kernel.rs:216-223`)
- it allocates a fresh `d_out` every call (`cdmc_gpu_kernel.rs:224-225`)
- it allocates a fresh zero-filled host readback buffer every call (`cdmc_gpu_kernel.rs:331-334`)

At the benchmark level, the optimizer harness already does the right thing by probing the route and asserting GPU use in the CDMC GPU benchmark (`gpu_native_expansion_opt.rs:274-325`). The validation harness prints the route but does **not** assert it (`composite_validation_workflow.rs:776-805`), which makes validation performance regressions easier to miss.

### Bottom-line interpretation

- **For repeated multi-combo optimization and validation:** CDMC should remain **GPU-preferred**, but only if the implementation stays device-native and stops redoing host/device staging every call.
- **For one-off host-oriented signal generation:** the current helper path can still be CPU-competitive or CPU-preferred because some GPU-preferred codepaths still read back to host and/or re-run host reference work.

---

# 2. Findings

## Confirmed issues

### Finding 1

- **Status:** confirmed issue
- **Severity:** high
- **Type:** performance
- **Location:** `cdmc.rs` → `compute_cdmc_mama_pair_cuda_device(...)`
- **Timing boundary affected:** app-path / Criterion / validation-cost
- **Why it matters:** the function computes device MAMA/FAMA into `d_mama` and `d_fama`, but then immediately allocates `host_mama` and `host_fama` and calls `mama_batch_into_host_f32(...)` only to discover `first_output_valid` (`cdmc.rs:485-536`). That is a second full-series MAMA/FAMA pass after the device result already exists.
- **Recommended change:** split this into:
  1. an **immediate local optimization** that caches `CudaMama`, device scratch, and host scratch to reduce repeated allocation cost, and
  2. a **preferred follow-up** that removes the host-side pass entirely by surfacing `first_output_valid` from the device path or proving an analytic warmup rule.

### Finding 2

- **Status:** confirmed issue
- **Severity:** high
- **Type:** performance / fallback-transparency
- **Location:** `cdmc.rs` → `compute_indicator_f64(...)`, `compute_mama_pair_f64(...)`, `compute_decycler_f64(...)`, `compute_cdmc_signals(...)`
- **Timing boundary affected:** app-path / validation-cost / Criterion
- **Why it matters:** the “GPU-preferred” host path still requests `CudaOutputTarget::HostF32` and widens to `Vec<f64>` (`cdmc.rs:588-597`, `cdmc.rs:683-694`), and failures silently fall through to CPU (`cdmc.rs:586-600`, `cdmc.rs:626-647`, `cdmc.rs:671-696`). That means some GPU-requested CDMC work is still paying D2H readback, fp32→fp64 widening, and silent fallback.
- **Recommended change:** stop optimizing this path as the primary GPU route. Use it only for CPU/reference/debug flows. Add a device-native prepared-signal object in `cdmc.rs` and route GPU optimization/validation through that object instead.

### Finding 3

- **Status:** confirmed issue
- **Severity:** high
- **Type:** performance
- **Location:** `cdmc_gpu_kernel.rs` → `CdmcGpuDeviceSignalMatrices`, `launch_compute_log_returns_f32(...)`, `eval_cdmc_metrics_gpu_kernel_device_rows(...)`
- **Timing boundary affected:** Criterion / validation-cost / app-path
- **Why it matters:** every kernel evaluation uploads `close` from host into `d_close`, allocates `d_lr`, launches a separate log-return kernel, then launches the main metrics kernel (`cdmc_gpu_kernel.rs:211-214`). This discards the benefits of existing device-resident signal matrices for repeated evaluation on the same candles.
- **Recommended change:** change the CDMC GPU signal input surface so callers can pass **device close** and optionally **device log returns**. Keep host `close_f32` only as a fallback. Add resident host-upload caching in the runtime for the cases where only host close is available.

### Finding 4

- **Status:** confirmed issue
- **Severity:** high
- **Type:** performance / allocation-waste
- **Location:** `cdmc_gpu_kernel.rs` → `eval_cdmc_metrics_gpu_kernel_device_rows(...)`
- **Timing boundary affected:** Criterion / validation-cost
- **Why it matters:** every call allocates fresh buffers for combo indices, time windows, output, and host readback (`cdmc_gpu_kernel.rs:216-225`, `cdmc_gpu_kernel.rs:331-334`). That is exactly the kind of repeated scratch churn that grows with repeated validation folds and repeated Criterion iterations.
- **Recommended change:** add reusable CDMC scratch buffers to `CdmcCudaRuntime` for:
  - `d_lr`
  - combo buffers (`d_combo_dpo`, `d_combo_cycle`, `d_combo_decycler`, `d_combo_start`, `d_combo_end`)
  - `d_out`
  - host readback

### Finding 5

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** benchmark-fidelity / fallback-transparency
- **Location:** `composite_validation_workflow.rs`
- **Timing boundary affected:** validation-cost / Criterion
- **Why it matters:** the validation GPU benchmark prints backend route and fallback metadata but does not assert GPU execution (`composite_validation_workflow.rs:776-805`). That means CDMC validation could silently fall back to CPU and still appear to be a valid GPU timing sample.
- **Recommended change:** make validation use the same route-assertion discipline that the optimizer benchmark already uses (`gpu_native_expansion_opt.rs:288-295`, `gpu_native_expansion_opt.rs:312-319`).

### Finding 6

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** performance / allocation-waste
- **Location:** `cdmc.rs` → `build_cdmc_desired_positions(...)`
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:** the function allocates `vec![0i32; n]` (`cdmc.rs:822`) even though all elements from `start_t..n` are deterministically overwritten by the main loop (`cdmc.rs:835-924`). The zero-filled tail is not needed.
- **Recommended change:** allocate with capacity, zero only the observable prefix `[0..start_t)`, and `push(...)` the tail values. This is safe and avoids full-buffer initialization.

### Finding 7

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `gpu_native_expansion_opt.rs` and `composite_validation_workflow.rs`
- **Timing boundary affected:** Criterion / validation-cost
- **Why it matters:** the optimizer CDMC request inherits `fast_ma_types = ["sma"]` and `slow_ma_types = ["ema"]` from `base_req(...)` (`gpu_native_expansion_opt.rs:31-32`), while CDMC validation candidates use `ema` / `ema` (`composite_validation_workflow.rs:304-305`). That means optimize-vs-validate comparisons for CDMC are not shape-aligned.
- **Recommended change:** explicitly align CDMC optimizer and validation benchmark MA-type shapes. Either make both `ema/ema`, or add a dedicated aligned CDMC benchmark pair while preserving the existing general profile.

### Finding 8

- **Status:** confirmed issue
- **Severity:** low
- **Type:** performance / maintainability
- **Location:** `cdmc.rs` → `compute_cdmc_signals(...)`, `compute_decycler_f64(...)`
- **Timing boundary affected:** app-path
- **Why it matters:** there are repeated tiny heap allocations for fixed-size param vectors such as `dpo_params`, `cycle_params`, and `decycler` params (`cdmc.rs:720-733`, `cdmc.rs:673-682`). This is not a major bottleneck, but it is avoidable.
- **Recommended change:** replace these fixed small `Vec`s with fixed arrays where possible.

## Likely issues

### Finding 9

- **Status:** likely issue
- **Severity:** medium
- **Type:** performance
- **Location:** `cdmc.rs` → `compute_cdmc_signals(...)`
- **Timing boundary affected:** app-path / validation-cost
- **Why it matters:** `correlation_cycle` is requested twice with the same params, once for `angle` and once for `state` (`cdmc.rs:744-763`). If the dynamic-indicator layer does not internally share computation, this is duplicate work.
- **Recommended change:** verify whether the dynamic indicator layer can compute both outputs in one pass. If not, add a combined multi-output path in the adjacent indicator-dispatch layer and update CDMC to use it.

### Finding 10

- **Status:** likely issue
- **Severity:** medium
- **Type:** performance / validation-throughput
- **Location:** `cdmc_gpu_kernel.rs` + adjacent validation caller path
- **Timing boundary affected:** validation-cost
- **Why it matters:** for walk-forward validation, CDMC row-index combos are often stable while only `start_t` / `end_t` change. The current API re-uploads all five combo arrays every call (`cdmc_gpu_kernel.rs:216-223`).
- **Recommended change:** second-phase optimization: make row-index buffers resident across repeated validation folds and only refresh the window buffers when the candidate set is unchanged.

## Open questions

### Open question 1

- **Status:** open question
- **Severity:** medium
- **Type:** parity / performance
- **Location:** `cdmc.rs` → MAMA warmup handling
- **Timing boundary affected:** app-path / Criterion / validation-cost
- **Why it matters:** the correct `first_output_valid` for CUDA MAMA/FAMA is the only thing currently forcing the extra host-side pass in `compute_cdmc_mama_pair_cuda_device(...)`. The provided files do not prove whether this can be derived analytically or must be surfaced by the lower-level CUDA MAMA implementation.
- **Recommended change:** verify in the lower-level MAMA implementation whether `first_output_valid` is deterministic from `first_valid` and sample count. If yes, remove the host pass. If not, surface it from the lower-level device API.

### Open question 2

- **Status:** open question
- **Severity:** medium
- **Type:** profiling-only
- **Location:** `cdmc_gpu_kernel.rs` → `cdmc_backtest_metrics_f32` kernel body (not provided)
- **Timing boundary affected:** profiling-only / Nsight
- **Why it matters:** I can review launcher/orchestration waste from the provided Rust files, but I cannot confirm warp divergence, coalescing, occupancy, or whether precomputed `d_lr` is actually better than inline log-return computation without the kernel body.
- **Recommended change:** do not change kernel-side data layout or inline-return logic until you capture Nsight evidence after the launcher-side waste is removed.

---

# 3. File-by-file implementation plan

## A. `cdmc.rs`

### A1. Introduce an explicit device-native CDMC prepared-signal object

**Why:** `compute_cdmc_signals(...)` is still a host-oriented path. For GPU optimization/validation you already have the primitives needed to keep CDMC signals on device. Make that path explicit so callers stop routing through `HostF32 -> f64` host vectors.

**Relevant code:** `cdmc.rs:400-558`, `cdmc.rs:709-799`

### Recommended edit

Add a dedicated prepared object for fixed-candidate validation and other device-native flows:

```rust
#[cfg(feature = "cuda")]
pub struct CdmcGpuPreparedSignals {
    pub dpo: CdmcPeriodSweepDevice,
    pub cycle_angle: CdmcPeriodSweepDevice,
    pub cycle_state: CdmcPeriodSweepDevice,
    pub mama: CdmcDeviceMatrix,
    pub fama: CdmcDeviceMatrix,
    pub decycler: CdmcPeriodSweepDevice,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
pub fn prepare_cdmc_gpu_validation_signals(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    periods: CdmcResolvedPeriods,
    cfg: &CdmcStrategyParams,
) -> Result<CdmcGpuPreparedSignals, String> {
    let dpo = compute_cdmc_dpo_sweep_cuda_device(
        prepared_inputs,
        periods.dpo,
        periods.dpo,
        1,
    )?;

    let cycle_angle = compute_cdmc_correlation_cycle_output_sweep_cuda_device(
        prepared_inputs,
        &["angle"],
        periods.correlation_cycle,
        periods.correlation_cycle,
        1,
    )?;

    let cycle_state = compute_cdmc_correlation_cycle_output_sweep_cuda_device(
        prepared_inputs,
        &["state"],
        periods.correlation_cycle,
        periods.correlation_cycle,
        1,
    )?;

    let ((mama, fama), mama_first_valid) =
        compute_cdmc_mama_pair_cuda_device(prepared_inputs, cfg)?;

    let decycler = compute_cdmc_decycler_sweep_cuda_device(
        prepared_inputs,
        periods.decycler,
        periods.decycler,
        1,
    )?;

    let first_valid = [
        dpo.first_valid_for_period(periods.dpo),
        cycle_angle.first_valid_for_period(periods.correlation_cycle),
        cycle_state.first_valid_for_period(periods.correlation_cycle),
        mama_first_valid,
        decycler.first_valid_for_period(periods.decycler),
    ]
    .into_iter()
    .max()
    .unwrap_or(mama_first_valid);

    Ok(CdmcGpuPreparedSignals {
        dpo,
        cycle_angle,
        cycle_state,
        mama,
        fama,
        decycler,
        first_valid,
    })
}
```

### Why this is valuable

This makes the intended GPU path explicit:

- signal prep stays on device
- only metric outputs come back to host
- GPU optimization and GPU validation stop “accidentally” using the host-oriented helper path

### Call-site guidance

Any adjacent caller currently doing “GPU-preferred `compute_cdmc_signals(...)` + host vectors + later GPU scoring” should be changed to:

1. build `PreparedCudaIndicatorInputs`
2. call `prepare_cdmc_gpu_validation_signals(...)`
3. pass device pointers directly into `cdmc_gpu_kernel.rs`

---

### A2. Add an immediate MAMA runtime cache, then remove the host pass in a follow-up

**Why:** the current MAMA device path is correct in shape but still wastes work. Some of that waste can be removed entirely in `cdmc.rs`; the last piece likely needs lower-level MAMA support.

**Relevant code:** `cdmc.rs:472-558`, `cdmc.rs:615-658`

### Immediate local edit (safe first wave)

Cache the following per device:

- `CudaMama`
- `d_inv_dp`
- `d_mama`
- `d_fama`
- one-element fast/slow limit device buffers
- optional host scan scratch, if you still need the host pass temporarily

Illustrative structure:

```rust
#[cfg(feature = "cuda")]
struct CdmcMamaRuntime {
    cuda: CudaMama,
    len_capacity: usize,
    d_inv_dp: Option<cust::memory::DeviceBuffer<f32>>,
    d_mama: Option<cust::memory::DeviceBuffer<f32>>,
    d_fama: Option<cust::memory::DeviceBuffer<f32>>,
    d_fast_limit: Option<(u32, cust::memory::DeviceBuffer<f32>)>,
    d_slow_limit: Option<(u32, cust::memory::DeviceBuffer<f32>)>,
    host_mama: Vec<f32>,
    host_fama: Vec<f32>,
}
```

Then move `CudaMama::new(...)`, the uninitialized device allocations, and the one-element limit allocations out of the hot call path when the device and capacity are unchanged.

### Preferred follow-up (best payoff)

Once the lower-level CUDA MAMA path can provide `first_output_valid`, replace this block entirely:

```rust
let mut host_mama = vec![f32::NAN; prices.len()];
let mut host_fama = vec![f32::NAN; prices.len()];
cuda.mama_batch_into_host_f32(
    prepared_inputs.source(),
    &sweep,
    &mut host_mama,
    &mut host_fama,
)?;
let mama_first_valid = host_mama.iter().position(|value| value.is_finite());
let fama_first_valid = host_fama.iter().position(|value| value.is_finite());
```

with a returned `first_output_valid` from the device-native path.

### Important safety note

Do **not** replace `host_mama` / `host_fama` with uninitialized host buffers **unless** you confirm that `mama_batch_into_host_f32(...)` writes every element deterministically before any read. Right now the NaN fill is behaviorally observable because the scan depends on non-written prefix values staying non-finite.

---

### A3. Keep `compute_cdmc_signals(...)` as a host/reference path, not the primary GPU path

**Why:** it is still useful for CPU reference, tests, and debugging, but it should not remain the primary way to do GPU CDMC work.

**Relevant code:** `cdmc.rs:561-799`

### Recommended edits

1. Keep `compute_cdmc_signals(...)` for CPU/reference flows.
2. Add a comment at the top of the function making that role explicit.
3. Route GPU optimization/validation through the prepared device object from **A1**.
4. Stop spending time trying to over-optimize the host-oriented GPU-preferred fallback path beyond small low-risk fixes.

Suggested comment:

```rust
/// Host/reference CDMC signal path.
///
/// Use this for CPU execution, debugging, and parity/reference checks.
/// GPU optimization/validation should prefer the device-native CDMC prep
/// helpers above and pass device pointers into `cdmc_gpu_kernel.rs`.
```

---

### A4. Remove full zero-fill in `build_cdmc_desired_positions(...)`

**Why:** the current code zero-fills the entire output vector even though only the prefix needs to be zero-initialized.

**Relevant code:** `cdmc.rs:814-925`

### Recommended edit

This version is safe, avoids `unsafe`, and removes unnecessary tail initialization:

```rust
pub fn build_cdmc_desired_positions(
    close: &[f64],
    signals: &CdmcSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &CdmcStrategyParams,
) -> Vec<i32> {
    let n = close.len();
    if start_t >= n {
        return vec![0i32; n];
    }

    let trade_offset = if strategy.trade_on_next_bar { 1usize } else { 0usize };
    let phase_gate = cfg.phase_threshold.abs();
    let mut pos = 0i32;

    let mut desired = Vec::with_capacity(n);
    desired.resize(start_t, 0i32);

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let dpo_now = signals.dpo.get(sig_t).copied().unwrap_or(f64::NAN);
        let dpo_prev = signals.dpo.get(prev_t).copied().unwrap_or(f64::NAN);
        let angle_now = signals.cycle_angle.get(sig_t).copied().unwrap_or(f64::NAN);
        let state_now = signals.cycle_state.get(sig_t).copied().unwrap_or(f64::NAN);
        let mama_now = signals.mama.get(sig_t).copied().unwrap_or(f64::NAN);
        let fama_now = signals.fama.get(sig_t).copied().unwrap_or(f64::NAN);
        let decycler_now = signals.decycler.get(sig_t).copied().unwrap_or(f64::NAN);
        let decycler_prev = signals.decycler.get(prev_t).copied().unwrap_or(f64::NAN);

        let bullish_phase = angle_now.is_finite() && angle_now >= phase_gate;
        let bearish_phase = angle_now.is_finite() && angle_now <= -phase_gate;
        let trend_regime = !state_now.is_finite() || state_now >= 0.0;
        let reversion_regime = state_now.is_finite() && state_now < 0.0;
        let trend_up = mama_now.is_finite() && fama_now.is_finite() && mama_now > fama_now;
        let trend_down = mama_now.is_finite() && fama_now.is_finite() && mama_now < fama_now;
        let decycler_rising =
            decycler_now.is_finite() && decycler_prev.is_finite() && decycler_now > decycler_prev;
        let decycler_falling =
            decycler_now.is_finite() && decycler_prev.is_finite() && decycler_now < decycler_prev;
        let dpo_cross_up =
            dpo_now.is_finite() && dpo_prev.is_finite() && dpo_prev <= 0.0 && dpo_now > 0.0;
        let dpo_cross_down =
            dpo_now.is_finite() && dpo_prev.is_finite() && dpo_prev >= 0.0 && dpo_now < 0.0;

        let long_entry = (trend_regime
            && trend_up
            && decycler_rising
            && dpo_now.is_finite()
            && dpo_now > 0.0
            && bullish_phase)
            || (reversion_regime && trend_up && dpo_cross_up && bullish_phase);

        let short_entry = !strategy.long_only
            && ((trend_regime
                && trend_down
                && decycler_falling
                && dpo_now.is_finite()
                && dpo_now < 0.0
                && bearish_phase)
                || (reversion_regime && trend_down && dpo_cross_down && bearish_phase));

        if pos == 0 {
            if long_entry {
                pos = 1;
            } else if short_entry {
                pos = -1;
            }
        } else if pos > 0 {
            let opposing_votes = usize::from(trend_down)
                + usize::from(decycler_falling)
                + usize::from(dpo_now.is_finite() && dpo_now < 0.0)
                + usize::from(bearish_phase);
            let mut exit_now = opposing_votes >= 2;
            if short_entry {
                if strategy.allow_flip {
                    pos = -1;
                    desired.push(pos);
                    continue;
                }
                exit_now = true;
            }
            if exit_now {
                pos = 0;
            }
        } else {
            let opposing_votes = usize::from(trend_up)
                + usize::from(decycler_rising)
                + usize::from(dpo_now.is_finite() && dpo_now > 0.0)
                + usize::from(bullish_phase);
            let mut exit_now = opposing_votes >= 2;
            if long_entry {
                if strategy.allow_flip {
                    pos = 1;
                    desired.push(pos);
                    continue;
                }
                exit_now = true;
            }
            if exit_now {
                pos = 0;
            }
        }

        desired.push(pos);
    }

    debug_assert_eq!(desired.len(), n);
    desired
}
```

This is a straightforward win because it keeps only the observable prefix zero-initialized.

---

### A5. Small fixed-size allocation cleanup

**Why:** these are micro-optimizations, not primary wins, but they are safe and free.

**Relevant code:** `cdmc.rs:673-682`, `cdmc.rs:720-733`

### Recommended edit

Replace fixed tiny `Vec` params with arrays:

```rust
let dpo_params = [IndicatorParamKV {
    key: "period".to_string(),
    value: IndicatorParamValue::Int(i64::from(periods.dpo)),
}];

let cycle_params = [
    IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(periods.correlation_cycle)),
    },
    IndicatorParamKV {
        key: "threshold".to_string(),
        value: IndicatorParamValue::Float(9.0),
    },
];
```

And similarly in `compute_decycler_f64(...)`:

```rust
let params = [
    IndicatorParamKV {
        key: "hp_period".to_string(),
        value: IndicatorParamValue::Int(i64::from(periods.decycler)),
    },
    IndicatorParamKV {
        key: "k".to_string(),
        value: IndicatorParamValue::Float(0.707),
    },
];
```

---

## B. `cdmc_gpu_kernel.rs`

### B1. Expand the input surface to accept device close / device log returns

**Why:** the current API forces a host `close_f32` slice, which in turn forces H2D upload and log-return recomputation every call.

**Relevant code:** `cdmc_gpu_kernel.rs:24-39`, `cdmc_gpu_kernel.rs:175-214`

### Recommended edit

Refactor the signal container like this:

```rust
pub struct CdmcGpuDeviceSignalMatrices<'a> {
    pub cols: usize,

    // Fallback path only.
    pub close_host_f32: Option<&'a [f32]>,

    // Preferred GPU path.
    pub close_device_ptr: Option<u64>,
    pub lr_device_ptr: Option<u64>,

    pub dpo_row_major_ptr: u64,
    pub dpo_rows: usize,
    pub dpo_stride: usize,

    pub cycle_angle_row_major_ptr: u64,
    pub cycle_state_row_major_ptr: u64,
    pub cycle_rows: usize,
    pub cycle_stride: usize,

    pub mama_ptr: u64,
    pub fama_ptr: u64,

    pub decycler_row_major_ptr: u64,
    pub decycler_rows: usize,
    pub decycler_stride: usize,
}
```

And update validation to enforce one of the valid market-series shapes:

```rust
fn validate_inputs(
    signals: &CdmcGpuDeviceSignalMatrices<'_>,
    combos: &CdmcGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("CDMC GPU kernel requires non-empty series".to_string());
    }

    if let Some(close) = signals.close_host_f32 {
        if close.len() != n {
            return Err("CDMC GPU close length mismatch".to_string());
        }
    }

    if signals.close_host_f32.is_none() && signals.close_device_ptr.is_none() {
        return Err("CDMC GPU requires close_host_f32 or close_device_ptr".to_string());
    }

    if let Some(ptr) = signals.close_device_ptr {
        if ptr == 0 {
            return Err("CDMC GPU close_device_ptr is null".to_string());
        }
    }
    if let Some(ptr) = signals.lr_device_ptr {
        if ptr == 0 {
            return Err("CDMC GPU lr_device_ptr is null".to_string());
        }
    }

    // Existing signal-matrix checks stay unchanged...
    // Existing combo-length checks stay unchanged...

    Ok(())
}
```

### Why this matters

This change is what unlocks the rest of the launcher-side improvements.

---

### B2. Add reusable runtime scratch and resident host-upload caching

**Why:** fresh allocation per kernel call is one of the main avoidable costs in `cdmc_gpu_kernel.rs`.

**Relevant code:** `cdmc_gpu_kernel.rs:49-75`, `cdmc_gpu_kernel.rs:211-225`, `cdmc_gpu_kernel.rs:331-334`

### Recommended edit

Add runtime scratch and optional resident host-uploaded market series:

```rust
struct CdmcResidentMarketSeries {
    host_ptr: usize,
    len: usize,
    d_close: DeviceBuffer<f32>,
    d_lr: DeviceBuffer<f32>,
}

#[derive(Default)]
struct CdmcCudaScratch {
    lr_capacity: usize,
    combo_capacity: usize,
    out_capacity: usize,

    d_lr: Option<DeviceBuffer<f32>>,
    d_combo_dpo: Option<DeviceBuffer<i32>>,
    d_combo_cycle: Option<DeviceBuffer<i32>>,
    d_combo_decycler: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,

    // Conservative reusable host readback buffer.
    // If your repo already has a shared alloc_policy helper for uninitialized
    // device readback, use that helper here instead of zero-filling growth.
    host_metrics: Vec<f32>,
}

struct CdmcCudaRuntime {
    device_id: u32,
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    resident_close: Option<CdmcResidentMarketSeries>,
    scratch: CdmcCudaScratch,
}
```

And add capacity-management helpers:

```rust
impl CdmcCudaScratch {
    fn ensure_lr_capacity(&mut self, len: usize) -> Result<(), String> {
        if self.lr_capacity >= len {
            return Ok(());
        }
        self.d_lr = Some(
            unsafe { DeviceBuffer::<f32>::uninitialized(len) }
                .map_err(|e| e.to_string())?,
        );
        self.lr_capacity = len;
        Ok(())
    }

    fn ensure_combo_capacity(&mut self, combo_count: usize) -> Result<(), String> {
        if self.combo_capacity >= combo_count {
            return Ok(());
        }
        self.d_combo_dpo = Some(
            unsafe { DeviceBuffer::<i32>::uninitialized(combo_count) }
                .map_err(|e| e.to_string())?,
        );
        self.d_combo_cycle = Some(
            unsafe { DeviceBuffer::<i32>::uninitialized(combo_count) }
                .map_err(|e| e.to_string())?,
        );
        self.d_combo_decycler = Some(
            unsafe { DeviceBuffer::<i32>::uninitialized(combo_count) }
                .map_err(|e| e.to_string())?,
        );
        self.d_combo_start = Some(
            unsafe { DeviceBuffer::<i32>::uninitialized(combo_count) }
                .map_err(|e| e.to_string())?,
        );
        self.d_combo_end = Some(
            unsafe { DeviceBuffer::<i32>::uninitialized(combo_count) }
                .map_err(|e| e.to_string())?,
        );
        self.combo_capacity = combo_count;
        Ok(())
    }

    fn ensure_out_capacity(&mut self, out_len: usize) -> Result<(), String> {
        if self.out_capacity < out_len {
            self.d_out = Some(
                unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }
                    .map_err(|e| e.to_string())?,
            );
            self.out_capacity = out_len;
        }
        if self.host_metrics.len() < out_len {
            self.host_metrics.resize(out_len, 0.0);
        }
        Ok(())
    }

    fn upload_combos(&mut self, combos: &CdmcGpuComboBatch<'_>) -> Result<(), String> {
        let combo_count = combos.dpo_row_index.len();
        self.ensure_combo_capacity(combo_count)?;

        self.d_combo_dpo
            .as_mut()
            .unwrap()
            .copy_from(combos.dpo_row_index)
            .map_err(|e| e.to_string())?;
        self.d_combo_cycle
            .as_mut()
            .unwrap()
            .copy_from(combos.cycle_row_index)
            .map_err(|e| e.to_string())?;
        self.d_combo_decycler
            .as_mut()
            .unwrap()
            .copy_from(combos.decycler_row_index)
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
}
```

### Important note on host readback

If your repo already has shared `alloc_policy` helpers for safe uninitialized host readback, use those instead of the conservative `Vec<f32>` scratch above. The review map explicitly says to prefer shared `alloc_policy` helpers before adding new local unsafe allocation code.

---

### B3. Change log-return launching to work from raw device close pointers

**Why:** once `close_device_ptr` becomes supported, the current `launch_compute_log_returns_f32(...)` signature is too narrow because it expects a `DeviceBuffer<f32>`.

**Relevant code:** `cdmc_gpu_kernel.rs:122-169`

### Recommended edit

Refactor it to accept a raw close pointer:

```rust
fn launch_compute_log_returns_f32_raw(
    rt: &mut CdmcCudaRuntime,
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

    let device = Device::get_device(rt.device_id).map_err(|e| e.to_string())?;
    let device_caps = detect_device_launch_caps(device);
    let launch = choose_launch_shape_1d_from_candidates(
        len,
        device_caps,
        detect_kernel_launch_caps(&kernel, device_caps),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr_raw;
    let mut len_i =
        i32::try_from(len).map_err(|_| "CDMC GPU close length exceeds i32 range".to_string())?;
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

---

### B4. Resolve the market-series path once per call, then reuse scratch everywhere else

**Why:** this is the core change that removes repeated H2D upload and scratch churn from the CDMC metrics path.

**Relevant code:** `cdmc_gpu_kernel.rs:175-342`

### Recommended edit

Inside `eval_cdmc_metrics_gpu_kernel_device_rows(...)`, replace the current `d_close` / `d_lr` / combo / `d_out` allocation block with logic like this:

```rust
let (close_ptr_raw, lr_ptr_raw) = if let Some(lr_ptr) = signals.lr_device_ptr {
    let close_ptr = signals
        .close_device_ptr
        .ok_or_else(|| "CDMC GPU requires close_device_ptr when lr_device_ptr is provided".to_string())?;
    (close_ptr, lr_ptr)
} else if let Some(close_ptr) = signals.close_device_ptr {
    rt.scratch.ensure_lr_capacity(signals.cols)?;
    let d_lr = rt.scratch.d_lr.as_mut().unwrap();
    launch_compute_log_returns_f32_raw(rt, close_ptr, d_lr, signals.cols)?;
    (close_ptr, d_lr.as_device_ptr().as_raw())
} else {
    let close = signals
        .close_host_f32
        .ok_or_else(|| "CDMC GPU requires close_host_f32 or close_device_ptr".to_string())?;

    let host_ptr = close.as_ptr() as usize;
    let reuse = rt
        .resident_close
        .as_ref()
        .map(|cached| cached.host_ptr == host_ptr && cached.len == close.len())
        .unwrap_or(false);

    if !reuse {
        let d_close = DeviceBuffer::from_slice(close).map_err(|e| e.to_string())?;
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(close.len()) }
            .map_err(|e| e.to_string())?;
        let close_ptr = d_close.as_device_ptr().as_raw();
        launch_compute_log_returns_f32_raw(rt, close_ptr, &mut d_lr, close.len())?;
        rt.resident_close = Some(CdmcResidentMarketSeries {
            host_ptr,
            len: close.len(),
            d_close,
            d_lr,
        });
    }

    let cached = rt.resident_close.as_ref().unwrap();
    (
        cached.d_close.as_device_ptr().as_raw(),
        cached.d_lr.as_device_ptr().as_raw(),
    )
};

rt.scratch.upload_combos(combos)?;
rt.scratch.ensure_out_capacity(out_len)?;

let d_combo_dpo = rt.scratch.d_combo_dpo.as_ref().unwrap();
let d_combo_cycle = rt.scratch.d_combo_cycle.as_ref().unwrap();
let d_combo_decycler = rt.scratch.d_combo_decycler.as_ref().unwrap();
let d_combo_start = rt.scratch.d_combo_start.as_ref().unwrap();
let d_combo_end = rt.scratch.d_combo_end.as_ref().unwrap();
let d_out = rt.scratch.d_out.as_mut().unwrap();
```

Then use `close_ptr_raw`, `lr_ptr_raw`, the cached combo buffers, and the cached output buffer when launching the main kernel.

After kernel completion:

```rust
rt.stream.synchronize().map_err(|e| e.to_string())?;
let host_metrics = &mut rt.scratch.host_metrics[..out_len];
d_out.copy_to(host_metrics).map_err(|e| e.to_string())?;

let mut out: Vec<Metrics> = Vec::with_capacity(combo_count);
for row in host_metrics.chunks_exact(CDMC_METRICS_STRIDE) {
    out.push(decode_metrics_row(row));
}
```

### What this removes

From the repeated hot path, this removes or amortizes:

- repeated `close` upload
- repeated `d_lr` allocation
- repeated combo-buffer allocation
- repeated output-buffer allocation
- repeated host readback allocation

---

### B5. Optional second-phase combo packing

**Why:** once the simpler reuse work is done, you can reduce launch arguments and H2D traffic further.

**Recommended second-phase edit (higher risk):** replace five separate combo buffers with one AoS buffer:

```rust
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CdmcComboDevice {
    dpo_row_index: i32,
    cycle_row_index: i32,
    decycler_row_index: i32,
    start_t: i32,
    end_t: i32,
}
```

This requires kernel signature changes and should only be done after parity tests and launcher-side reuse are in place.

---

## C. `gpu_native_expansion_opt.rs`

### C1. Align the CDMC benchmark request shape with validation

**Why:** optimizer and validation are currently not MA-type aligned for CDMC.

**Relevant code:** `gpu_native_expansion_opt.rs:16-52`, `gpu_native_expansion_opt.rs:209-219`

### Recommended edit

Override CDMC MA types explicitly:

```rust
fn make_cdmc_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.cdmc = Some(CdmcStrategyParams::default());

    // Align CDMC optimizer benchmark shape with validation.
    req.fast_ma_types = vec!["ema".to_string()];
    req.slow_ma_types = vec!["ema".to_string()];

    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}
```

### Why this matters

This gives you an apples-to-apples CDMC optimize-vs-validate story without changing the generic shared benchmark defaults for other strategies.

### Optional improvement

If you still want the old CDMC benchmark shape, add a second explicit `make_cdmc_req_sma_ema(...)` variant and benchmark both. That is better than quietly inheriting cross-strategy defaults.

---

### C2. Keep the existing GPU route assertion pattern

**Why:** this file already does the right thing and should be treated as the benchmark-fidelity reference for CDMC.

**Relevant code:** `gpu_native_expansion_opt.rs:274-325`

### Action

Do **not** weaken this route assertion. Reuse the same discipline in validation.

---

## D. `composite_validation_workflow.rs`

### D1. Add explicit GPU assertion for CDMC validation timing samples

**Why:** validation currently prints GPU route metadata but does not fail on fallback.

**Relevant code:** `composite_validation_workflow.rs:776-805`

### Recommended edit

Immediately after each GPU route probe, assert GPU backend use:

```rust
assert!(
    matches!(route_probe.backend_used, ta_app_core::BackendUsed::Gpu),
    "expected GPU backend for composite_validate/gpu_kernel/{}/{}/{}candidates, got backend={:?}, route={:?}, fallback={:?}",
    name,
    candles_len,
    candidate_count,
    route_probe.backend_used,
    route_probe.backend_route,
    route_probe.backend_fallback_reason,
);
```

Do the same for the initial fixed ASR benchmark and any dedicated CDMC validation benchmark group.

---

### D2. Add a dedicated CDMC validation profile instead of only relying on the generic profile loop

**Why:** CDMC already has a dedicated optimizer benchmark. It should also have a dedicated validation benchmark group with explicit route assertion and CDMC-shaped inputs.

**Relevant code:** `composite_validation_workflow.rs:297-329`, `composite_validation_workflow.rs:723-823`

### Recommended edit

Add a dedicated CDMC candle builder and a CDMC validation group. For example:

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_cdmc_cycle_candles(len: usize) -> Candles {
    let len = len.max(260);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let angle = i as f64 * 0.11;
        let trend = if i < len / 2 {
            i as f64 * 0.035
        } else {
            (len - i) as f64 * 0.028
        };
        let px = 100.0 + angle.sin() * 3.6 + (angle * 0.43).cos() * 1.1 + trend;
        open.push(px * 0.9988);
        high.push(px * 1.0038);
        low.push(px * 0.9962);
        close.push(px);
        vol.push(1_250.0 + (i % 11) as f64 * 42.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

Then add a dedicated benchmark group, for example:

```rust
let candles = make_cdmc_cycle_candles(100_000);
let candidates = make_cdmc_candidates(32);

let base = CompositeValidationRequest {
    data_id: "bench-gpu-cdmc".to_string(),
    backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
    ma_source: "close".to_string(),
    fast_ma_params: None,
    slow_ma_params: None,
    candidates: candidates.clone(),
    walk_forward: Some(WalkForwardConfigOptional {
        window_bars: 10_000,
        step_bars: 10_000,
    }),
    reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
        window_bars: 15_000,
        step_bars: 10_000,
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
```

Then probe + assert GPU route + benchmark exactly as done in the optimizer benchmark.

### Why this matters

This gives CDMC its own dedicated validation timing surface instead of relying only on the broad generic per-strategy profile loop.

---

# 4. Safe uninitialized-memory / copy-elision matrix

This section is here because your request explicitly called out minimizing copies and using uninitialized memory where possible.

## Safe to keep uninitialized today

### 1. Device-only working buffers in `cdmc.rs`

These are already correctly using uninitialized device memory and should stay that way:

- `d_inv_dp` (`cdmc.rs:491-492`)
- `d_mama` (`cdmc.rs:497-498`)
- `d_fama` (`cdmc.rs:499-500`)

They are safe candidates because they are write-only device outputs before first read.

### 2. Device-only working buffers in `cdmc_gpu_kernel.rs`

These are also appropriate uninitialized device buffers:

- `d_lr` (`cdmc_gpu_kernel.rs:212-214`) — provided the log-return kernel overwrites the entire range
- `d_out` (`cdmc_gpu_kernel.rs:224-225`) — provided the metrics kernel overwrites every metrics slot

These should remain uninitialized and, after the refactor, should be reused rather than reallocated.

## Safe to optimize without `unsafe`

### 3. `build_cdmc_desired_positions(...)`

Use the `Vec::with_capacity + resize(prefix, 0) + push(tail)` pattern from **A4**. This avoids full initialization without requiring `MaybeUninit`.

## Only safe if a shared host-readback helper exists

### 4. Host metrics readback in `cdmc_gpu_kernel.rs`

Current code:

```rust
let mut host_metrics = vec![0.0f32; out_len];
d_out.copy_to(host_metrics.as_mut_slice())?;
```

This is a legitimate target for uninitialized host allocation **if** your repo already has a safe shared helper for “device fully overwrites destination before any read”. The review map suggests such helpers may exist in `alloc_policy.rs`, but that file was not provided here.

**Recommended rule:**

- If you have a shared helper, use it.
- If you do not, keep a reusable conservative `Vec<f32>` scratch until you can wire the helper in.

## Do **not** switch to uninitialized yet

### 5. `host_mama` / `host_fama` in `compute_cdmc_mama_pair_cuda_device(...)`

Current code:

```rust
let mut host_mama = vec![f32::NAN; prices.len()];
let mut host_fama = vec![f32::NAN; prices.len()];
```

Do not replace these with uninitialized buffers unless you verify that `mama_batch_into_host_f32(...)` writes the entire buffer before any scan or read. Right now the NaN prefix is semantically important.

### 6. Any host-side NaN-prefix indicator series

If any future refactor still depends on “unwritten prefix remains NaN until scanned”, keep explicit prefix initialization or use a prefix-init helper. Do not blindly switch such buffers to uninitialized memory.

---

# 5. Safe optimization opportunities

These are the changes I would do first.

## Safe opportunity 1: move GPU validation/optimization onto the explicit device-native signal path

- **Files:** `cdmc.rs` + adjacent caller updates
- **Why safe:** you already have device-prep helpers; this is mostly route cleanup
- **Expected payoff:** high
- **Risk:** low to medium, mostly parity wiring

## Safe opportunity 2: add `CdmcCudaRuntime` scratch reuse and resident close/lr caching

- **Files:** `cdmc_gpu_kernel.rs`
- **Why safe:** behavior stays the same; only allocation/upload repetition is removed
- **Expected payoff:** high for Criterion and validation-cost paths
- **Risk:** low if parity tests are added

## Safe opportunity 3: zero only the observable prefix in `build_cdmc_desired_positions(...)`

- **Files:** `cdmc.rs`
- **Why safe:** output semantics remain identical
- **Expected payoff:** medium on repeated CPU-side validation/reference paths
- **Risk:** low

## Safe opportunity 4: align CDMC benchmark shapes and assert GPU route in validation

- **Files:** `gpu_native_expansion_opt.rs`, `composite_validation_workflow.rs`
- **Why safe:** measurement-only changes
- **Expected payoff:** high confidence in perf evidence
- **Risk:** low

## Safe opportunity 5: replace fixed tiny `Vec` param allocations with arrays

- **Files:** `cdmc.rs`
- **Why safe:** no behavior change
- **Expected payoff:** low but free
- **Risk:** low

---

# 6. Higher-risk optimization opportunities

These are worth considering only after the safe changes above land and parity is well-covered.

## Higher-risk opportunity 1: remove precomputed `d_lr` and compute returns inline in the kernel

- **Files:** `cdmc_gpu_kernel.rs` + CUDA kernel body (not provided)
- **Potential benefit:** removes one kernel launch and one device buffer
- **Why it is higher risk:** may increase per-thread arithmetic or reduce clarity; cannot be judged correctly without kernel-body profiling

## Higher-risk opportunity 2: pack combo fields into an AoS buffer

- **Files:** `cdmc_gpu_kernel.rs` + CUDA kernel body
- **Potential benefit:** fewer kernel args, fewer H2D copies, better locality per combo
- **Why it is higher risk:** requires kernel signature changes and parity retesting

## Higher-risk opportunity 3: cache row-index buffers across repeated validation folds

- **Files:** `cdmc_gpu_kernel.rs` + adjacent validation caller
- **Potential benefit:** more validation throughput when candidate set stays fixed across folds
- **Why it is higher risk:** needs a stable candidate identity and more stateful runtime handling

## Higher-risk opportunity 4: fully eliminate the host MAMA/FAMA scan

- **Files:** `cdmc.rs` + lower-level MAMA implementation
- **Potential benefit:** large
- **Why it is higher risk:** correctness depends on a verified first-valid rule or lower-level API support

---

# 7. Benchmark and perf-gate review

## What is already good

### Optimizer benchmark discipline is strong

`gpu_native_expansion_opt.rs` already does the right things for CDMC:

- probes the route (`gpu_native_expansion_opt.rs:274-280`)
- prints backend route/fallback metadata (`gpu_native_expansion_opt.rs:281-287`)
- asserts GPU use before timing (`gpu_native_expansion_opt.rs:288-295`)
- asserts GPU use inside timed iterations (`gpu_native_expansion_opt.rs:312-319`)

This should remain your reference harness style.

## What needs to improve

### Validation benchmark must assert GPU backend

The validation harness only prints route info (`composite_validation_workflow.rs:783-794`) and then proceeds to benchmark. That is not strict enough for a strategy-local GPU route audit.

### CDMC needs a dedicated validation profile

The generic profile loop includes CDMC (`composite_validation_workflow.rs:820-822`), but it is not enough for strategy-local review because:

- it uses generic candles
- it has no CDMC-specific route assertion
- it is easier for CDMC regressions to hide inside the broader composite profile loop

### Benchmark shapes need alignment

CDMC optimizer and validation MA-type shapes should be made explicitly consistent.

## Timing-boundary guidance

- The launcher and scratch-reuse changes in `cdmc_gpu_kernel.rs` target **Criterion** and **validation-cost** boundaries.
- The host-pass reduction in `compute_cdmc_mama_pair_cuda_device(...)` affects **app-path**, **Criterion**, and **validation-cost**.
- Route assertions affect **benchmark-fidelity** only; they do not speed anything up, but they protect measurement integrity.

---

# 8. Test review

## Existing coverage visible in the provided files

In `cdmc.rs`, current tests cover:

- param validation (`cdmc.rs:957-966`)
- one desired-position logic example (`cdmc.rs:968-988`)
- one GPU smoke test for signal generation (`cdmc.rs:990-1020`)

That is useful, but not enough for the refactors above.

## Missing tests you should add

### 1. CPU-vs-GPU signal parity test

Add a direct CPU vs GPU comparison for `compute_cdmc_signals(...)`:

```rust
#[cfg(feature = "cuda")]
#[test]
fn compute_cdmc_signals_gpu_matches_cpu_within_tolerance() {
    if !cuda_available() {
        return;
    }

    let candles = make_trending_candles(2_048);
    let cfg = CdmcStrategyParams::default();
    let periods = resolve_cdmc_periods(&cfg, 12, 34);

    let cpu = compute_cdmc_signals(
        &candles,
        "close",
        &Backend::CpuOnly,
        periods,
        &cfg,
        None,
    )
    .expect("cpu cdmc signals");

    let prepared = PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
        .expect("cuda inputs");

    let gpu = compute_cdmc_signals(
        &candles,
        "close",
        &Backend::GpuOnly { device_id: 0 },
        periods,
        &cfg,
        Some(&prepared),
    )
    .expect("gpu cdmc signals");

    fn assert_close(a: &[f64], b: &[f64], atol: f64, rtol: f64) {
        assert_eq!(a.len(), b.len());
        for (idx, (&lhs, &rhs)) in a.iter().zip(b.iter()).enumerate() {
            if !lhs.is_finite() || !rhs.is_finite() {
                assert_eq!(lhs.is_finite(), rhs.is_finite(), "finite mismatch at {idx}");
                continue;
            }
            let tol = atol + rtol * lhs.abs().max(rhs.abs());
            assert!(
                (lhs - rhs).abs() <= tol,
                "mismatch at {idx}: lhs={lhs}, rhs={rhs}, tol={tol}"
            );
        }
    }

    assert_eq!(cpu.first_valid, gpu.first_valid);
    assert_close(&cpu.dpo, &gpu.dpo, 1e-4, 1e-4);
    assert_close(&cpu.cycle_angle, &gpu.cycle_angle, 1e-4, 1e-4);
    assert_close(&cpu.cycle_state, &gpu.cycle_state, 1e-4, 1e-4);
    assert_close(&cpu.mama, &gpu.mama, 1e-4, 1e-4);
    assert_close(&cpu.fama, &gpu.fama, 1e-4, 1e-4);
    assert_close(&cpu.decycler, &gpu.decycler, 1e-4, 1e-4);
}
```

### 2. Threshold-edge desired-position tests

Add tests specifically for:

- `phase_threshold` just below / at / just above the gate
- `dpo_prev == 0.0` and `dpo_now > 0.0`
- `dpo_prev == 0.0` and `dpo_now < 0.0`
- `trade_on_next_bar = true`
- `allow_flip = true`

These are the conditions most likely to show CPU/GPU `f32` vs `f64` parity drift in practice.

### 3. GPU fallback/blocker tests

Add tests that confirm `eval_cdmc_metrics_gpu_kernel_device_rows(...)` rejects overlays / stop-loss / take-profit exactly as intended (`cdmc_gpu_kernel.rs:187-195`).

### 4. Scratch-reuse repeatability test

After adding scratch reuse in `cdmc_gpu_kernel.rs`, add a test that runs the same GPU evaluation twice and asserts identical metrics on both runs. This is important because stateful runtime caches can cause subtle stale-buffer bugs.

### 5. Validation benchmark route test

If you keep CDMC inside the broad validation profile loop, add a unit/integration check that the CDMC GPU validation profile actually reports `BackendUsed::Gpu`.

---

# 9. Suggested improvement plan

This section follows the prompt requirement for a concrete improvement plan.

## Phase 1 — low-risk, high-payoff local edits

1. **`cdmc_gpu_kernel.rs`: add scratch reuse and resident close/lr caching.**
   - This is the biggest local win entirely inside the provided files.
   - It reduces repeated allocation and repeated H2D upload.

2. **`cdmc.rs`: add explicit `CdmcGpuPreparedSignals` for device-native callers.**
   - This clarifies the intended GPU path and stops future route drift.

3. **`cdmc.rs`: remove full zero-fill in `build_cdmc_desired_positions(...)`.**
   - Easy, safe, and measurable on repeated CPU/reference paths.

4. **`composite_validation_workflow.rs`: add route assertions and a dedicated CDMC validation profile.**
   - This improves measurement trust immediately.

5. **`gpu_native_expansion_opt.rs`: align the CDMC benchmark MA-type shape.**
   - This fixes cross-harness comparability.

## Phase 2 — reduce MAMA prep cost without widening scope too much

6. **`cdmc.rs`: add `CudaMama` and scratch reuse for the device MAMA path.**
   - This is the best local improvement available even if the final host pass cannot be removed yet.

7. **`cdmc.rs`: keep the host pass only as a temporary correctness crutch.**
   - If lower-level support is available, remove it.

## Phase 3 — parity and validation hardening

8. Add CPU-vs-GPU signal parity tests.
9. Add threshold-edge decision tests.
10. Add scratch-reuse repeatability tests.
11. Add GPU fallback/blocker tests.

## Phase 4 — only after the above lands cleanly

12. Consider AoS combo packing.
13. Consider caching row indices across repeated validation folds.
14. Consider kernel-side inline log-return computation only if Nsight shows it helps.

---

# 10. Detailed implementation checklist

Use this as the implementation tracker.

## `cdmc.rs`

- [ ] Add a new `CdmcGpuPreparedSignals` struct.
- [ ] Add `prepare_cdmc_gpu_validation_signals(...)`.
- [ ] Update comments/docstrings to distinguish host/reference CDMC signal generation from the preferred device-native CDMC path.
- [ ] Keep `compute_cdmc_signals(...)` as the CPU/reference/debug path.
- [ ] Replace tiny fixed-size param `Vec`s with arrays in:
  - [ ] `compute_cdmc_signals(...)`
  - [ ] `compute_decycler_f64(...)`
- [ ] Refactor `build_cdmc_desired_positions(...)` to:
  - [ ] allocate with capacity
  - [ ] initialize only the prefix with zeros
  - [ ] `push(...)` tail values instead of indexing a fully zero-filled buffer
- [ ] Add a cached MAMA runtime structure for:
  - [ ] `CudaMama`
  - [ ] `d_inv_dp`
  - [ ] `d_mama`
  - [ ] `d_fama`
  - [ ] one-element fast/slow limit buffers
  - [ ] temporary host scan scratch if still needed
- [ ] Keep `host_mama` / `host_fama` NaN-filled unless the lower-level API is proven to fully overwrite them.
- [ ] Investigate whether `first_output_valid` can be surfaced directly from the lower-level CUDA MAMA path.
- [ ] If yes, remove the extra `mama_batch_into_host_f32(...)` pass.
- [ ] If no, keep the host pass but make it allocation-reused rather than freshly allocated.
- [ ] Verify whether `correlation_cycle` angle/state can be computed in one pass in the adjacent indicator layer.

## `cdmc_gpu_kernel.rs`

- [ ] Change `CdmcGpuDeviceSignalMatrices` so it can accept:
  - [ ] `close_host_f32: Option<&[f32]>`
  - [ ] `close_device_ptr: Option<u64>`
  - [ ] `lr_device_ptr: Option<u64>`
- [ ] Update `validate_inputs(...)` for the new market-series rules.
- [ ] Add `device_id` to `CdmcCudaRuntime`.
- [ ] Add `resident_close: Option<CdmcResidentMarketSeries>` to `CdmcCudaRuntime`.
- [ ] Add `scratch: CdmcCudaScratch` to `CdmcCudaRuntime`.
- [ ] Add reusable scratch buffers for:
  - [ ] `d_lr`
  - [ ] `d_combo_dpo`
  - [ ] `d_combo_cycle`
  - [ ] `d_combo_decycler`
  - [ ] `d_combo_start`
  - [ ] `d_combo_end`
  - [ ] `d_out`
  - [ ] `host_metrics`
- [ ] Refactor `launch_compute_log_returns_f32(...)` into a raw-pointer variant.
- [ ] In `eval_cdmc_metrics_gpu_kernel_device_rows(...)`, resolve market series in this order:
  - [ ] use `lr_device_ptr` if already provided
  - [ ] otherwise use `close_device_ptr` and reusable `d_lr`
  - [ ] otherwise upload `close_host_f32` once and cache the resident pair
- [ ] Replace per-call combo buffer allocations with scratch-buffer reuse.
- [ ] Replace per-call output buffer allocation with scratch-buffer reuse.
- [ ] Replace per-call host metrics allocation with reusable scratch.
- [ ] If the repo already has a shared alloc-policy helper for safe uninitialized host readback, switch host metrics scratch to that helper.
- [ ] Add a second-phase design note for possible AoS combo packing.
- [ ] Add a second-phase design note for caching stable row-index buffers across validation folds.

## `gpu_native_expansion_opt.rs`

- [ ] Override CDMC MA types explicitly in `make_cdmc_req(...)`.
- [ ] Choose one benchmark alignment policy and document it:
  - [ ] `ema/ema` for both optimize and validate
  - [ ] or two explicit CDMC benchmark variants if both shapes matter
- [ ] Keep the existing route-probe assertion behavior intact.
- [ ] Re-run the CDMC benchmark after the launcher refactor and compare:
  - [ ] small case
  - [ ] medium case
  - [ ] large case

## `composite_validation_workflow.rs`

- [ ] Add explicit GPU assertions after every GPU route probe used for timing.
- [ ] Add a dedicated CDMC validation benchmark group.
- [ ] Add a CDMC-shaped candle builder for validation.
- [ ] Ensure CDMC validation uses the same MA-type shape as the CDMC optimizer benchmark.
- [ ] Keep the broad generic profile loop, but do not rely on it as the only CDMC validation evidence.
- [ ] Re-run CDMC validation benchmarks after the launcher refactor and compare:
  - [ ] 50k / 8 candidates
  - [ ] 100k / 32 candidates
  - [ ] any dedicated CDMC profile you add

## Tests

- [ ] Add CPU-vs-GPU CDMC signal parity test.
- [ ] Add threshold-edge tests for desired-position logic.
- [ ] Add `trade_on_next_bar` parity coverage.
- [ ] Add `allow_flip` parity coverage.
- [ ] Add GPU blocker/fallback tests for overlays / stop-loss / take-profit.
- [ ] Add repeated-run scratch-reuse tests for `cdmc_gpu_kernel.rs`.
- [ ] Add a validation-route test that fails if CDMC validation silently falls back to CPU.

## Performance verification

- [ ] Re-run `gpu_native_expansion_opt` CDMC benches before/after the launcher changes.
- [ ] Re-run `composite_validation_workflow` CDMC benches before/after the launcher changes.
- [ ] Confirm route metadata still reports GPU for intended CDMC cases.
- [ ] Confirm no parity regressions against CPU reference paths.
- [ ] Only after the above, decide whether AoS combo packing or inline log-return computation is worth pursuing.

---

# Final recommendation order

If you want the shortest path to meaningful improvement, do the work in this order:

1. `cdmc_gpu_kernel.rs` scratch reuse + resident `close` / `lr`
2. `cdmc.rs` explicit device-native prepared CDMC signals
3. `composite_validation_workflow.rs` route assertion + dedicated CDMC validation profile
4. `gpu_native_expansion_opt.rs` CDMC benchmark shape alignment
5. `cdmc.rs` desired-position tail copy-elision
6. `cdmc.rs` cached MAMA runtime
7. lower-level MAMA first-valid surfacing to delete the extra host pass
8. only then consider AoS combo packing or kernel-side log-return changes

That sequence stays faithful to the review-map prompt: smaller parity-safe changes first, measured surfaces first, and strategy-local GPU payoff before broader redesign.
