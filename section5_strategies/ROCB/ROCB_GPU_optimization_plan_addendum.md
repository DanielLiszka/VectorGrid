# ROCB GPU optimization plan addendum

This addendum patches a few gaps in the main ROCB plan and adds a small set of optional, higher-upside changes that still preserve correctness and existing features.

It is still scoped to the same ROCB review surface:

- `rocb.rs`
- `rocb_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

The main plan is still the base implementation document. This addendum exists to:

1. correct a few under-specified or missing items
2. add a couple of low-risk CPU-side improvements I should have called out explicitly
3. list optional changes that go slightly beyond the original prompt, but are still performance-first and parity-safe

---

## What I would change in the main plan immediately

These are the most important additions.

### 1. Add full ROCB GPU combo/input bounds validation

The current `validate_inputs(...)` checks non-empty length, pointer non-null, basic shape non-zero, and combo vector length agreement. It does **not** validate these correctness-critical conditions:

- `channel_stride >= cols`
- `atr_stride >= cols`
- every `channel_row_index` is in `0..channel_rows`
- every `atr_row_index` is in `0..atr_rows`
- every `start_t` / `end_t` is non-negative
- every `start_t` / `end_t` stays within the series length
- `start_t` and `end_t` obey the kernel's window semantics

That belongs in the implementation plan because it affects both correctness and debuggability.

### 2. Replace every unchecked `usize -> i32` kernel-launch cast

`launch_compute_log_returns_f32(...)` already uses `i32::try_from(len)`. The surrounding ROCB kernel-launch path still uses unchecked `as i32` casts for:

- `signals.channel_rows`
- `signals.channel_stride`
- `signals.atr_rows`
- `signals.atr_stride`
- `combo_count`
- `signals.cols`
- `ROCB_METRICS_STRIDE`

That is a correctness hole, not just a style issue. It should be fixed in both the legacy wrapper path and the new uploaded-input hot path.

### 3. Use a `period - 1` fast path for scalar `first_valid`

`RocbPeriodSweepDevice::first_valid_for_period(...)` already encodes the expected warmup boundary as `period - 1`. The scalar host-returning ROCB helpers still scan for the first finite value with `first_finite_idx_local(...)` even though the expected first valid bar is already known.

That is low risk and worth adding.

### 4. Avoid full tail zero-fill in the CPU desired-position builder

My earlier `build_rocb_desired_positions_into(...)` still did `out.resize(n, 0)`, which preserves reuse but still pays a full zero-fill. That misses the allocation/prefix-init guidance you explicitly asked me to emphasize.

The better version is:

- `out.clear()`
- `out.reserve(n - out.capacity())` if needed
- `out.resize(start_t.min(n), 0)`
- `push(pos)` for every bar in `start_t..n`

That keeps the required zero prefix, avoids a full-tail memset, and remains fully safe Rust.

### 5. Cache indicator-name resolution and avoid heap params where easy

In `rocb.rs`, both `compute_rocb_breakout_signals(...)` and `compute_rocb_risk_signals(...)` repeat:

- `resolve_param_key(...)`
- `resolve_output_id(...)`
- heap-allocation of a one-element `Vec<IndicatorParamKV>`

That is not the biggest cost in the ROCB stack, but it is real avoidable CPU overhead on the scalar/reference path. It is worth fixing because the change is small and does not alter behavior.

### 6. Add GPU validation route assertions, not just route printing

The ROCB validation benches should not only print `backend_used`, `backend_route`, and `backend_fallback_reason`. They should assert the expected route in the dedicated ROCB GPU workflow cases, just as the optimizer GPU Criterion path already does.

Otherwise you can accidentally benchmark a fallback path and not notice quickly.

---

## Code changes to add to the main plan

# 1) `rocb_gpu_kernel.rs`

## A. Strengthen `validate_inputs(...)`

Add structural validation plus combo-bounds validation.

> Note on `end_t`: the host and kernel must agree on whether it is exclusive (`[start_t, end_t)`) or inclusive (`[start_t, end_t]`). The snippet below assumes **exclusive** `end_t`. If the kernel uses inclusive semantics, change the `end_u > n` and ordering checks accordingly.

```rust
fn checked_i32(value: usize, label: &str) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("ROCB GPU {label} exceeds i32 range"))
}

fn validate_combo_bounds(
    signals: &RocbGpuDeviceSignalMatrices<'_>,
    combos: &RocbGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;

    if signals.channel_stride < n {
        return Err("ROCB GPU channel stride is smaller than series length".to_string());
    }
    if signals.atr_stride < n {
        return Err("ROCB GPU ATR stride is smaller than series length".to_string());
    }

    for (i, &row) in combos.channel_row_index.iter().enumerate() {
        let row_u = usize::try_from(row)
            .map_err(|_| format!("ROCB GPU channel_row_index[{i}] is negative"))?;
        if row_u >= signals.channel_rows {
            return Err(format!(
                "ROCB GPU channel_row_index[{i}]={} out of bounds for {} rows",
                row, signals.channel_rows
            ));
        }
    }

    for (i, &row) in combos.atr_row_index.iter().enumerate() {
        let row_u = usize::try_from(row)
            .map_err(|_| format!("ROCB GPU atr_row_index[{i}] is negative"))?;
        if row_u >= signals.atr_rows {
            return Err(format!(
                "ROCB GPU atr_row_index[{i}]={} out of bounds for {} rows",
                row, signals.atr_rows
            ));
        }
    }

    for i in 0..combos.start_t.len() {
        let start_u = usize::try_from(combos.start_t[i])
            .map_err(|_| format!("ROCB GPU start_t[{i}] is negative"))?;
        let end_u = usize::try_from(combos.end_t[i])
            .map_err(|_| format!("ROCB GPU end_t[{i}] is negative"))?;

        if start_u >= n {
            return Err(format!(
                "ROCB GPU start_t[{i}]={} out of bounds for len {}",
                combos.start_t[i], n
            ));
        }
        if end_u > n {
            return Err(format!(
                "ROCB GPU end_t[{i}]={} out of bounds for len {}",
                combos.end_t[i], n
            ));
        }
        if start_u >= end_u {
            return Err(format!(
                "ROCB GPU invalid eval window at combo {i}: start_t={} end_t={} (expected start_t < end_t for exclusive end)",
                combos.start_t[i], combos.end_t[i]
            ));
        }
    }

    Ok(())
}

fn validate_inputs(
    signals: &RocbGpuDeviceSignalMatrices<'_>,
    combos: &RocbGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("ROCB GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f32.len() != n {
        return Err("ROCB GPU close length mismatch".to_string());
    }
    if signals.upper_row_major_ptr == 0
        || signals.lower_row_major_ptr == 0
        || signals.atr_row_major_ptr == 0
    {
        return Err("ROCB GPU signal pointer is null".to_string());
    }
    if signals.channel_rows == 0
        || signals.channel_stride == 0
        || signals.atr_rows == 0
        || signals.atr_stride == 0
    {
        return Err("ROCB GPU signal matrix shape is invalid".to_string());
    }

    let combos_len = combos.channel_row_index.len();
    if combos.atr_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("ROCB GPU combo vector length mismatch".to_string());
    }

    validate_combo_bounds(signals, combos)?;
    Ok(())
}
```

Why this should be added:

- it prevents malformed combo data from silently reaching the kernel
- it makes debugging much easier when a caller miscomputes row indices or eval windows
- it protects any future resident/reused combo-upload path from reusing bad batches

## B. Replace all lossy launch casts with checked conversions

Use the helper above in the launch path.

```rust
let mut channel_rows_i = checked_i32(signals.channel_rows, "channel row count")?;
let mut channel_stride_i = checked_i32(signals.channel_stride, "channel stride")?;
let mut atr_rows_i = checked_i32(signals.atr_rows, "ATR row count")?;
let mut atr_stride_i = checked_i32(signals.atr_stride, "ATR stride")?;
let mut combo_count_i = checked_i32(combo_count, "combo count")?;
let mut t_i = checked_i32(signals.cols, "series length")?;
let mut out_stride_i = checked_i32(ROCB_METRICS_STRIDE, "metrics stride")?;
```

Do this in:

- the legacy `eval_rocb_metrics_gpu_kernel_device_rows(...)` path
- the new uploaded-input path from the main plan

## C. Reuse `Function` handles and launch caps in `RocbCudaRuntime`

This is optional but easy and belongs in the runtime helper if you touch it anyway.

```rust
struct RocbCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    compute_lr_fn: cust::function::Function<'static>,
    rocb_metrics_fn: cust::function::Function<'static>,
    device_caps: crate::gpu_backtest_kernel::DeviceLaunchCaps,
    compute_lr_caps: crate::gpu_backtest_kernel::KernelLaunchCaps,
    rocb_metrics_caps: crate::gpu_backtest_kernel::KernelLaunchCaps,
    scratch: RocbGpuScratch,
}
```

Because `cust::Function` lifetime details depend on how you retain the module, you may need a slightly different ownership shape in your actual repo. The point is:

- do not repeatedly call `module.get_function(...)` on the hot path if you can cache it safely
- do not repeatedly call `detect_device_launch_caps(...)` for the same runtime/device
- do not repeatedly recompute kernel launch caps for the same kernel/module/device pair

This will not be the top gain, but it is a good cleanup once the uploaded-input path exists.

## D. Prefer the shared allocation helper before local unsafe host-uninit code

The base plan already emphasized this, but I would make it even stricter:

1. first choice: repo-wide `alloc_policy` / shared copy-elision helper
2. second choice: host buffer reuse with no unsafe
3. last choice: local `MaybeUninit<f32>` readback helper

If you do adopt a local helper, keep it in one place and add a targeted test that it never exposes uninitialized elements before the device copy completes.

---

# 2) `rocb.rs`

## A. Add a fast path for `first_valid`

This is a clean low-risk CPU optimization.

```rust
fn first_valid_or_expected(values: &[f64], expected: usize) -> Option<usize> {
    if let Some(&v) = values.get(expected) {
        if v.is_finite() {
            return Some(expected);
        }
    }
    values.iter().position(|v| v.is_finite())
}
```

Use it here:

```rust
pub fn compute_rocb_breakout_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    channel_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<RocbBreakoutSignals, String> {
    let ids = rocb_resolved_indicator_ids()?;
    let params = [IndicatorParamKV {
        key: ids.donchian_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(channel_period)),
    }];

    let upper = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "donchian",
        ids.donchian_upper_output.as_str(),
        &params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let lower = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "donchian",
        ids.donchian_lower_output.as_str(),
        &params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    if upper.len() != lower.len() {
        return Err("ROCB donchian output length mismatch".to_string());
    }

    let expected = usize::from(channel_period.saturating_sub(1));
    let first_valid = first_valid_or_expected(&upper, expected)
        .zip(first_valid_or_expected(&lower, expected))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "ROCB donchian outputs are all non-finite".to_string())?;

    Ok(RocbBreakoutSignals {
        upper,
        lower,
        first_valid,
    })
}

pub fn compute_rocb_risk_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    atr_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<RocbRiskSignals, String> {
    let ids = rocb_resolved_indicator_ids()?;
    let params = [IndicatorParamKV {
        key: ids.atr_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(atr_period)),
    }];

    let atr = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "atr",
        ids.atr_output.as_str(),
        &params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let expected = usize::from(atr_period.saturating_sub(1));
    let first_valid = first_valid_or_expected(&atr, expected)
        .ok_or_else(|| "ROCB ATR output is all non-finite".to_string())?;

    Ok(RocbRiskSignals { atr, first_valid })
}
```

## B. Fix the CPU desired-position builder to avoid tail zero-fill

This is the version I would use instead of the earlier `resize(n, 0)` variant.

```rust
pub fn build_rocb_desired_positions_into(
    out: &mut Vec<i32>,
    close: &[f64],
    breakout: &RocbBreakoutSignals,
    risk: &RocbRiskSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &RocbStrategyParams,
) {
    let n = close.len();
    out.clear();

    if start_t >= n {
        out.resize(n, 0);
        return;
    }

    out.reserve(n.saturating_sub(out.capacity()));
    out.resize(start_t, 0);

    let trade_offset = if strategy.trade_on_next_bar { 1usize } else { 0usize };
    let max_trade_bars = usize::from(cfg.max_trade_bars.max(1));
    let mut pos: i32 = 0;
    let mut bars_in_trade: usize = 0;
    let mut entry_price = f64::NAN;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close.get(sig_t).copied().unwrap_or(f64::NAN);
        let upper_prev = breakout.upper.get(prev_t).copied().unwrap_or(f64::NAN);
        let lower_prev = breakout.lower.get(prev_t).copied().unwrap_or(f64::NAN);
        let atr_now = risk.atr.get(sig_t).copied().unwrap_or(f64::NAN);

        let long_breakout =
            close_now.is_finite() && upper_prev.is_finite() && close_now > upper_prev;
        let short_breakout =
            close_now.is_finite() && lower_prev.is_finite() && close_now < lower_prev;

        if pos == 0 {
            bars_in_trade = 0;
            entry_price = f64::NAN;
            if long_breakout {
                pos = 1;
                bars_in_trade = 1;
                entry_price = close_now;
            } else if !strategy.long_only && short_breakout {
                pos = -1;
                bars_in_trade = 1;
                entry_price = close_now;
            }
        } else if pos > 0 {
            let stop_dist = if atr_now.is_finite() {
                cfg.atr_stop_mult * atr_now
            } else {
                f64::NAN
            };
            let stop_hit = entry_price.is_finite()
                && stop_dist.is_finite()
                && close_now.is_finite()
                && close_now <= entry_price - stop_dist;
            let channel_exit =
                close_now.is_finite() && upper_prev.is_finite() && close_now <= upper_prev;
            let mut exit_now = stop_hit || channel_exit || bars_in_trade >= max_trade_bars;

            if !strategy.long_only && short_breakout {
                if strategy.allow_flip {
                    pos = -1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    out.push(pos);
                    continue;
                }
                exit_now = true;
            }

            if exit_now {
                pos = 0;
                bars_in_trade = 0;
                entry_price = f64::NAN;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        } else {
            let stop_dist = if atr_now.is_finite() {
                cfg.atr_stop_mult * atr_now
            } else {
                f64::NAN
            };
            let stop_hit = entry_price.is_finite()
                && stop_dist.is_finite()
                && close_now.is_finite()
                && close_now >= entry_price + stop_dist;
            let channel_exit =
                close_now.is_finite() && lower_prev.is_finite() && close_now >= lower_prev;
            let mut exit_now = stop_hit || channel_exit || bars_in_trade >= max_trade_bars;

            if long_breakout {
                if strategy.allow_flip {
                    pos = 1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    out.push(pos);
                    continue;
                }
                exit_now = true;
            }

            if exit_now {
                pos = 0;
                bars_in_trade = 0;
                entry_price = f64::NAN;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        }

        out.push(pos);
    }
}
```

Why I prefer this over the earlier version:

- only the prefix before `start_t` is zero-initialized
- every value from `start_t..n` is produced by the algorithm and appended once
- it removes a full-vector tail memset without any unsafe code

## C. Cache resolved ROCB indicator IDs

This is small, easy, and makes the scalar/reference path cheaper.

```rust
use std::sync::OnceLock;

#[derive(Clone, Debug)]
struct RocbResolvedIndicatorIds {
    donchian_period_key: String,
    donchian_upper_output: String,
    donchian_lower_output: String,
    atr_period_key: String,
    atr_output: String,
}

static ROCB_RESOLVED_IDS: OnceLock<Result<RocbResolvedIndicatorIds, String>> = OnceLock::new();

fn rocb_resolved_indicator_ids() -> Result<&'static RocbResolvedIndicatorIds, String> {
    match ROCB_RESOLVED_IDS.get_or_init(|| {
        Ok(RocbResolvedIndicatorIds {
            donchian_period_key: resolve_param_key(
                "donchian",
                &["period", "length", "timeperiod"],
            )?,
            donchian_upper_output: resolve_output_id(
                "donchian",
                &["upperband", "upper", "value"],
            )?,
            donchian_lower_output: resolve_output_id(
                "donchian",
                &["lowerband", "lower"],
            )?,
            atr_period_key: resolve_param_key("atr", &["period", "length", "timeperiod"] )?,
            atr_output: resolve_output_id("atr", &["value", "atr"] )?,
        })
    }) {
        Ok(ids) => Ok(ids),
        Err(err) => Err(err.clone()),
    }
}
```

Notes:

- this only pays the descriptor resolution cost once per process
- it removes repeated lowercase-string building and lookup work from the hot scalar path
- it pairs well with one-element stack arrays instead of heap `Vec`s for params

## D. Optional ROCB-local multi-output Donchian dispatch

This should be treated as an **open question**, not a confirmed change. If `dynamic_indicator_adapter` or the adjacent dispatch layer can emit both Donchian upper and lower from one computation, then `compute_rocb_breakout_signals(...)` can remove one indicator dispatch.

Do not implement this until the adjacent adapter confirms one-call multi-output support.

---

# 3) `gpu_native_expansion_opt.rs`

The original plan already adds ROCB benches. Two additions make it better:

## A. Add a warm-state microbench that bypasses upload cost

This is beyond the default Criterion harness and should be marked clearly as a **microbench**, not your main perf gate.

Why it is useful:

- the main ROCB benches should continue to measure the real route
- the warm-state microbench isolates the upside of the new uploaded-input entrypoint
- it tells you whether the residency work actually pays off before full integration

Suggested shape:

- build candles once
- build Donchian/ATR sweep matrices once
- upload market once
- upload combo batch once
- benchmark only `eval_rocb_metrics_gpu_kernel_uploaded(...)`

This should not replace the existing route-level bench. It should supplement it.

## B. Add a bench case that reuses the same combo batch repeatedly

The uploaded-combo optimization only matters if reuse actually happens. Add one repeated-finalist shape where the same small ROCB batch is scored many times.

That is especially relevant for validation/finalist re-ranking flows.

---

# 4) `composite_validation_workflow.rs`

## A. Assert GPU route for the dedicated ROCB GPU workflow

Keep printing route info, but also assert it.

```rust
assert!(
    matches!(route_probe.backend_used, ta_app_core::BackendUsed::Gpu),
    "expected ROCB GPU validation route, got backend={:?}, route={:?}, fallback={:?}",
    route_probe.backend_used,
    route_probe.backend_route,
    route_probe.backend_fallback_reason,
);
```

## B. Add one heavier ROCB candidate-count shape that stresses reuse

Add at least one dedicated ROCB validation profile like:

- `50_000` candles / `64` candidates
- `100_000` candles / `64` candidates
- one in-sample-only larger batch to stress upload reuse and output readback

The reason is simple: ROCB currently has only light validation presence. A heavier case is where the new market/combo residency changes should show up.

---

## Optional changes that go beyond the original prompt

These are not required to implement the base plan, but they are strong candidates if you want to squeeze more out of ROCB without dropping features.

### 1. Reuse already-resident `close` from adjacent GPU-prep/runtime surfaces

This is likely the biggest optional win beyond the attached-file-only scope.

The broader GPU runtime path already has resident-market machinery and dataset-keyed returns reuse in `gpu_backtest_kernel.rs`. If `PreparedCudaIndicatorInputs` or an adjacent resident-market cache already owns a device-resident close series, the best ROCB design is to consume that directly instead of uploading `close_f32` again.

Practical shape:

- extend ROCB uploaded-market creation to accept either host `close_f32` or an existing device pointer/buffer
- only compute `d_lr` if the cached market bundle does not already provide it
- attach an invalidation key based on `device_id`, series length, and source identity

This is a bigger integration change than the base plan, but it removes a producer-to-destination copy boundary entirely.

### 2. Use pinned host memory for metrics readback if readback stays material

Only do this after the uploaded-input and scratch-reuse path lands and after you measure.

Possible improvement path:

- use a page-locked host buffer for metrics readback
- switch to async D2H copy if your `cust` layer or raw CUDA wrapper supports it cleanly
- synchronize once at the end of the stream work

This is higher complexity than plain buffer reuse, so it is not a phase-1 task. But if ROCB output readback remains measurable after the obvious wins, pinned readback is a valid next step.

### 3. Add a caller-provided metrics sink API

Instead of always returning `Vec<Metrics>`, add an internal helper that writes into a caller-provided `Vec<Metrics>` or a mutable row buffer.

Why it can help:

- it removes repeated `Vec<Metrics>` allocation in repeated validation loops
- it lets the caller keep a reusable result buffer next to the reusable combo batch
- it keeps the public convenience API intact while giving hot paths a cheaper internal entrypoint

Suggested shape:

```rust
pub fn eval_rocb_metrics_gpu_kernel_uploaded_into(
    device_id: u32,
    market: &RocbGpuUploadedMarket,
    signals: &RocbGpuDeviceSignalMatrices<'_>,
    combos: &RocbGpuUploadedCombos,
    strategy: &StrategyConfig,
    rocb: &RocbStrategyParams,
    out: &mut Vec<Metrics>,
) -> Result<(), String>
```

### 4. Candidate ordering / dedup experiments

This is an experiment, not a guaranteed win.

Potential idea:

- sort combos by `(channel_row_index, atr_row_index, start_t, end_t)` before upload
- measure whether kernel-side locality improves enough to matter
- optionally deduplicate truly identical combos before scoring, then expand results back to original order

Only do this with measurement. Without the kernel source or Nsight evidence, this stays speculative.

### 5. Warm-state and cold-state benchmarks should be kept separate

This is not a code optimization, but it prevents bad decisions.

You should explicitly keep two ROCB benchmark classes:

- real route benchmarks that include prep and route machinery
- warm-state/uploaded-input microbenches that isolate resident ROCB scoring

If those are mixed together, it becomes hard to tell whether a regression came from the kernel, the prep path, or the residency/cache boundary.

### 6. Add an Nsight checklist only after the base changes land

Do not start here. But after phase 1/2 changes land, an Nsight pass can validate whether remaining time is in:

- host-to-device upload
- device kernel runtime
- device-to-host copy
- launch overhead
- row-major matrix access patterns

This is worthwhile only after the obvious staging waste is removed.

---

## Revised implementation priority

I would now prioritize ROCB work in this exact order:

1. add benchmark coverage and route assertions for ROCB
2. add combo/window bounds validation and checked `i32` conversions
3. land uploaded-market and uploaded-combo reuse
4. reuse `d_out` plus reusable host readback buffer
5. add scalar `first_valid` fast path
6. fix CPU desired-position builder to avoid tail zero-fill
7. cache resolved ROCB indicator IDs and switch one-param vectors to stack arrays
8. only then consider pinned host readback or direct reuse of broader resident-market state

That ordering gives you:

- correctness protection first
- biggest warm-path wins next
- CPU cleanup after the GPU route is stable
- higher-complexity optional wins only after the easy work is measured

---

## Checklist delta to append to the main plan

### Correctness and guardrails

- [ ] Add `validate_combo_bounds(...)` to `rocb_gpu_kernel.rs`.
- [ ] Validate `channel_stride >= cols` and `atr_stride >= cols`.
- [ ] Validate every `channel_row_index` is within `channel_rows`.
- [ ] Validate every `atr_row_index` is within `atr_rows`.
- [ ] Validate every `start_t` and `end_t` is non-negative.
- [ ] Validate every eval window matches kernel semantics (`start_t < end_t` for exclusive-end windows, or the correct inclusive variant).
- [ ] Replace all lossy `as i32` launch casts with `checked_i32(...)` conversions.
- [ ] Add a test that oversized dimensions fail with a clear `i32 range` error.

### Low-risk CPU wins

- [ ] Add `first_valid_or_expected(...)`.
- [ ] Use `period - 1` as the expected warmup index for scalar Donchian/ATR helpers.
- [ ] Switch one-element param vectors in scalar ROCB helpers to stack arrays.
- [ ] Add a file-local `OnceLock` cache for resolved ROCB indicator IDs.
- [ ] Replace `resize(n, 0)` in the in-place desired-position builder with prefix-only zero init plus `push(...)`.
- [ ] Add a test that zeros are still present before `start_t`.
- [ ] Add a test that `start_t >= n` still returns all zeros.

### Bench and workflow improvements

- [ ] Add GPU route assertions to dedicated ROCB validation benches.
- [ ] Add a heavier ROCB validation profile with larger candidate count.
- [ ] Add a warm-state ROCB microbench that exercises the uploaded-input entrypoint directly.
- [ ] Add a repeated-same-combos ROCB bench to measure combo-upload reuse.

### Optional advanced work

- [ ] Evaluate whether ROCB can consume an already-resident device `close` series from adjacent runtime/prep surfaces.
- [ ] Measure whether metrics readback remains material after scratch reuse.
- [ ] Only if needed, prototype pinned host metrics readback.
- [ ] Only if needed, add a caller-provided `Vec<Metrics>` sink API for repeated validation loops.
- [ ] Only if measurement justifies it, test combo sorting/dedup for locality.
- [ ] Only after the base path is stable, run Nsight to classify the remaining bottleneck.

---

## Bottom line

The main plan was directionally right, but I would definitely add these missing items before implementation starts:

- full combo/window bounds validation
- checked `usize -> i32` conversions everywhere in the ROCB kernel launch path
- scalar `first_valid` fast path
- prefix-only zero init in the CPU desired-position builder
- cached ROCB indicator-name resolution
- route assertions for ROCB GPU validation benches

The optional changes with the best upside beyond the original prompt are:

1. consuming an already-resident device `close` from broader GPU runtime/prep state
2. pinned host readback if readback remains measurable after reuse changes
3. a caller-provided result sink for repeated validation loops

Those are the changes I would layer on top of the original Markdown plan.
