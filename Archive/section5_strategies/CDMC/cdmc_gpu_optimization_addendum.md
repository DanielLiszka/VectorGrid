# CDMC GPU optimization plan — addendum and optional improvements

This addendum is meant to sit next to `cdmc_gpu_optimization_plan.md`.

It answers three questions:

1. **Did the original plan miss any first-order issues?**
2. **What additional changes are worth adding now?**
3. **What optional changes go beyond the original prompt scope but still fit the goal of more speed, less copying, and preserved correctness?**

The short answer is:

- The original plan already captured the **highest-payoff first-order wins**.
- I do **not** think it missed a larger win than the current top items:
  - removing the extra host-side MAMA/FAMA pass
  - keeping CDMC prep device-native
  - reusing `close` / `lr` / combo / output buffers
  - making validation route assertions strict
- There **are** several worthwhile additions, mostly in four buckets:
  - kernel-launch metadata caching
  - extra correctness guards around cached device state and combo indices
  - small but safe host-path cost reductions
  - optional profiling-driven improvements that go beyond the original bundle

---

# 1. What I would add to the original plan

## A. Cache kernel metadata and launch-cap inputs inside `CdmcCudaRuntime`

### Why this matters

In `cdmc_gpu_kernel.rs`, each call currently does all of the following for both the log-return kernel and the main CDMC metrics kernel:

- `module.get_function(...)`
- `Device::get_device(device_id)`
- `detect_device_launch_caps(...)`
- `detect_kernel_launch_caps(...)`
- `choose_launch_shape_1d_from_candidates(...)`

That is not the biggest bottleneck compared with repeated H2D/D2H copies, but once you remove the bigger copy/allocation waste, this becomes the next layer of repeated warm-path overhead.

### Why I did not elevate it to the main plan

Because it is a **second-order** improvement. It is not bigger than removing repeated `close` upload and repeated `d_lr` allocation.

### Recommended addition

Store device launch caps and kernel launch caps in `CdmcCudaRuntime` after first use.

You do **not** need to store raw CUDA function handles if that becomes lifetime-annoying. It is enough to cache the expensive capability detection and still fetch the function each call.

### Example sketch

```rust
struct CdmcKernelLaunchCache {
    device_caps: Option<DeviceLaunchCaps>,
    log_returns_caps: Option<KernelLaunchCaps>,
    metrics_caps: Option<KernelLaunchCaps>,
}

struct CdmcCudaRuntime {
    device_id: u32,
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    resident_close: Option<CdmcResidentMarketSeries>,
    scratch: CdmcCudaScratch,
    launch_cache: CdmcKernelLaunchCache,
}

impl CdmcCudaRuntime {
    fn device_caps(&mut self) -> Result<DeviceLaunchCaps, String> {
        if let Some(caps) = self.launch_cache.device_caps {
            return Ok(caps);
        }
        let device = Device::get_device(self.device_id).map_err(|e| e.to_string())?;
        let caps = detect_device_launch_caps(device);
        self.launch_cache.device_caps = Some(caps);
        Ok(caps)
    }

    fn log_returns_launch(
        &mut self,
        len: usize,
    ) -> Result<(cust::function::Function, LaunchShape1d), String> {
        let kernel = self
            .module
            .get_function("compute_log_returns_f32")
            .map_err(|e| e.to_string())?;
        let device_caps = self.device_caps()?;
        let kernel_caps = if let Some(caps) = self.launch_cache.log_returns_caps {
            caps
        } else {
            let caps = detect_kernel_launch_caps(&kernel, device_caps);
            self.launch_cache.log_returns_caps = Some(caps);
            caps
        };
        let launch = choose_launch_shape_1d_from_candidates(
            len,
            device_caps,
            kernel_caps,
            &[64u32, 128, 256, 512],
        );
        Ok((kernel, launch))
    }

    fn metrics_launch(
        &mut self,
        combo_count: usize,
    ) -> Result<(cust::function::Function, LaunchShape1d), String> {
        let kernel = self
            .module
            .get_function("cdmc_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let device_caps = self.device_caps()?;
        let kernel_caps = if let Some(caps) = self.launch_cache.metrics_caps {
            caps
        } else {
            let caps = detect_kernel_launch_caps(&kernel, device_caps);
            self.launch_cache.metrics_caps = Some(caps);
            caps
        };
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            device_caps,
            kernel_caps,
            &[32u32, 64, 128, 256],
        );
        Ok((kernel, launch))
    }
}
```

### Priority

**Medium.** Add it after scratch reuse and resident market-series caching.

---

## B. Tighten combo-index and window validation in `cdmc_gpu_kernel.rs`

### Why this matters

`validate_inputs(...)` currently checks lengths and null pointers, but it does **not** verify that:

- `dpo_row_index[i]` is inside `0..dpo_rows`
- `cycle_row_index[i]` is inside `0..cycle_rows`
- `decycler_row_index[i]` is inside `0..decycler_rows`
- `start_t[i]` / `end_t[i]` are valid against `signals.cols`
- `start_t[i] < end_t[i]`

That is more of a **correctness and safety** omission than a raw performance omission, but it matters because stale or malformed combo batches can produce:

- invalid device reads
- misleading benchmark results
- hard-to-debug parity drift
- crashes that only show up after adding stateful runtime caches

### Recommended addition

Do not necessarily put the full O(combo_count) validation on every hot timed path. Instead:

- add a strict validator for pre-flight checks
- use it in tests, debug builds, and validation-benchmark route probes
- optionally keep only lightweight checks in the steady-state hot path

### Example sketch

```rust
fn validate_combo_ranges(
    signals: &CdmcGpuDeviceSignalMatrices<'_>,
    combos: &CdmcGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;

    for i in 0..combos.dpo_row_index.len() {
        let dpo_row = combos.dpo_row_index[i];
        let cycle_row = combos.cycle_row_index[i];
        let decycler_row = combos.decycler_row_index[i];
        let start_t = combos.start_t[i];
        let end_t = combos.end_t[i];

        if dpo_row < 0 || (dpo_row as usize) >= signals.dpo_rows {
            return Err(format!("CDMC GPU dpo_row_index[{i}] out of range: {dpo_row}"));
        }
        if cycle_row < 0 || (cycle_row as usize) >= signals.cycle_rows {
            return Err(format!("CDMC GPU cycle_row_index[{i}] out of range: {cycle_row}"));
        }
        if decycler_row < 0 || (decycler_row as usize) >= signals.decycler_rows {
            return Err(format!(
                "CDMC GPU decycler_row_index[{i}] out of range: {decycler_row}"
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("CDMC GPU start/end contains negative value at combo {i}"));
        }

        let start_u = start_t as usize;
        let end_u = end_t as usize;
        if start_u >= end_u || end_u > n {
            return Err(format!(
                "CDMC GPU invalid window at combo {i}: start_t={start_t}, end_t={end_t}, n={n}"
            ));
        }
    }

    Ok(())
}
```

### Priority

**Medium-high for correctness.** I would add it before more aggressive cache/state reuse.

---

## C. Do not key resident host-upload cache only by `host_ptr + len`

### Why this matters

In the original plan, I suggested a `resident_close` cache keyed by host pointer and length. That is useful, but it is **not safe enough as the only key** if the caller can mutate the underlying candle buffer in place while retaining the same allocation.

That means a pure pointer-based reuse rule can accidentally keep stale device `d_close` / `d_lr` while the host slice contents have changed.

### Better rule

Use one of these instead:

1. **Preferred:** pass an explicit stable cache token from the caller.
2. **Second-best:** only enable host-side cache reuse when the caller supplies a trusted immutable request/session identity.
3. **Fallback:** if no explicit cache key is available, skip host-side resident reuse and use only the `close_device_ptr`/`lr_device_ptr` path for true reuse.

### Recommended API addition

```rust
pub struct CdmcGpuDeviceSignalMatrices<'a> {
    pub cols: usize,

    pub close_host_f32: Option<&'a [f32]>,
    pub close_host_cache_key: Option<u64>,

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

Then in the runtime:

```rust
struct CdmcResidentMarketSeries {
    cache_key: u64,
    len: usize,
    d_close: DeviceBuffer<f32>,
    d_lr: DeviceBuffer<f32>,
}
```

And only reuse when both key and length match:

```rust
let reuse = match (signals.close_host_cache_key, rt.resident_close.as_ref()) {
    (Some(key), Some(cached)) => cached.cache_key == key && cached.len == signals.cols,
    _ => false,
};
```

### Why this is important

This is a **correctness guardrail** I would absolutely add to the plan.

### Priority

**High if you implement host-side resident caching.**

---

## D. Reduce `first_valid` scanning work in `compute_cdmc_signals(...)`

### Why this matters

The original plan focused on the GPU-native path, which is where the biggest wins are. But the host/reference path also does avoidable scanning work when building `first_valid`:

```rust
let first_valid = first_finite_idx_local(&dpo)
    .zip(first_finite_idx_local(&cycle_angle))
    .zip(first_finite_idx_local(&cycle_state))
    .zip(first_finite_idx_local(&mama))
    .zip(first_finite_idx_local(&fama))
    .zip(first_finite_idx_local(&decycler))
```

For DPO, correlation-cycle, and decycler, the warmup may already be deterministic from the selected periods. If that is true for the actual CPU dispatch behavior, you do not need to scan those arrays at all.

### Safer version

Keep scanning only the uncertain pieces:

- scan `mama`
- scan `fama`
- derive `dpo`, `cycle`, `decycler` analytically if parity confirms that behavior

### Example sketch

```rust
fn deterministic_cdmc_first_valid(periods: CdmcResolvedPeriods) -> usize {
    usize::from(periods.dpo.saturating_sub(1))
        .max(usize::from(periods.correlation_cycle.saturating_sub(1)))
        .max(usize::from(periods.decycler.saturating_sub(1)))
}

pub fn compute_cdmc_signals(...) -> Result<CdmcSignals, String> {
    // ... compute vectors ...

    let structural_first_valid = deterministic_cdmc_first_valid(periods);
    let mama_first = first_finite_idx_local(&mama)
        .ok_or_else(|| "CDMC MAMA outputs are all non-finite".to_string())?;
    let fama_first = first_finite_idx_local(&fama)
        .ok_or_else(|| "CDMC FAMA outputs are all non-finite".to_string())?;

    let first_valid = structural_first_valid.max(mama_first.max(fama_first));

    Ok(CdmcSignals {
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

### Caveat

Only do this after confirming CPU dispatch actually matches the deterministic warmup rule for these indicators.

### Priority

**Low to medium.** Useful, but not a top-3 win.

---

## E. Cache descriptor/key/output resolution with `OnceLock`

### Why this matters

`resolve_output_id(...)` and `resolve_param_key(...)` walk the dynamic indicator registry and allocate/clones strings on every call.

That is not massive, but it is repeated in the CDMC hot surface:

- DPO sweep
- correlation-cycle sweep
- decycler sweep
- host signal generation

### Recommended addition

If the dynamic indicator registry is effectively process-static after startup, cache these lookups once.

### Example sketch

```rust
#[cfg(feature = "cuda")]
struct CdmcDispatchIds {
    dpo_period_key: String,
    dpo_output_id: String,
    cycle_period_key: String,
    cycle_angle_output_id: String,
    cycle_state_output_id: String,
    decycler_period_key: String,
    decycler_output_id: String,
}

#[cfg(feature = "cuda")]
fn cdmc_dispatch_ids() -> &'static CdmcDispatchIds {
    use std::sync::OnceLock;

    static IDS: OnceLock<CdmcDispatchIds> = OnceLock::new();
    IDS.get_or_init(|| CdmcDispatchIds {
        dpo_period_key: resolve_param_key("dpo", &["period", "length", "timeperiod"])
            .expect("cdmc dpo period key"),
        dpo_output_id: resolve_output_id("dpo", &["value", "dpo"])
            .expect("cdmc dpo output id"),
        cycle_period_key: resolve_param_key(
            "correlation_cycle",
            &["period", "length", "timeperiod"],
        )
        .expect("cdmc cycle period key"),
        cycle_angle_output_id: resolve_output_id("correlation_cycle", &["angle"])
            .expect("cdmc cycle angle output id"),
        cycle_state_output_id: resolve_output_id("correlation_cycle", &["state"])
            .expect("cdmc cycle state output id"),
        decycler_period_key: resolve_param_key("decycler", &["hp_period", "period", "length"])
            .expect("cdmc decycler period key"),
        decycler_output_id: resolve_output_id("decycler", &["value", "decycler"])
            .expect("cdmc decycler output id"),
    })
}
```

### Priority

**Low.** Nice cleanup, not a major measured win.

---

# 2. Optional changes that go beyond the original prompt scope

These are not the first changes I would implement, but they are valid performance ideas that fit your goals.

## Optional 1. Sort or bucket combos for better kernel locality

### Idea

If the kernel walks large rows of DPO / cycle / decycler matrices, combo order may affect memory locality and branch similarity. If you sort combos by row indices and optionally by `(start_t, end_t)`, you may improve cache behavior and reduce divergence.

### Sketch

```rust
let mut order: Vec<usize> = (0..combo_count).collect();
order.sort_unstable_by_key(|&i| {
    (
        combos.dpo_row_index[i],
        combos.cycle_row_index[i],
        combos.decycler_row_index[i],
        combos.start_t[i],
        combos.end_t[i],
    )
});
```

Then upload the permuted combo arrays, run the kernel, and restore result order on host.

### When to try it

Only after the simpler copy/allocation wins land, and only if profiling shows the kernel is memory-latency sensitive.

### Risk

Medium. Requires result-order restoration and more parity testing.

---

## Optional 2. Compress repeated windows across validation folds

### Idea

If many candidate batches reuse the same `(start_t, end_t)` windows, you can upload unique windows once and let combos reference a small window-index buffer instead of full `start_t` and `end_t` arrays every time.

### Why it is beyond scope

This is a larger batching format change. It goes beyond simple scratch reuse.

### Risk

Medium to high.

---

## Optional 3. Use pinned host memory and async D2H only after reuse is in place

### Idea

Once `d_out` is reused and the batch shape is stable, you can allocate pinned host metrics scratch and perform async D2H on the same stream. That can overlap better with downstream work or with future multi-stream expansion.

### Why it is not first-wave

Pinned memory and async copy complexity is not worth it until:

- repeated H2D close upload is gone
- `d_lr` is reused
- combo/output scratch reuse is in place
- correctness tests are solid

### Suggested rule

Do not do this before the simpler wins.

---

## Optional 4. Audit upstream `PreparedCudaIndicatorInputs` reuse

### Why this matters

The files under review show CDMC using `PreparedCudaIndicatorInputs`, which is good. But the provided files do **not** show whether upstream optimization/validation orchestration reuses those prepared inputs or rebuilds them more often than necessary.

If an adjacent caller recreates `PreparedCudaIndicatorInputs::from_candles_on_device(...)` repeatedly for the same candle set or fold, that could wipe out part of the strategy-local gain.

### What to check in adjacent files

Look for whether prepared inputs are created:

- once per candle set
- once per validation fold
- once per candidate batch
- once per candidate (bad)

### Priority

High as an audit, but it requires adjacent files beyond the current bundle surface.

---

## Optional 5. Kernel-body questions to leave for Nsight, not for first implementation

Do **not** rush into these without kernel-body profiling:

- remove `close_ptr` entirely if the kernel only needs returns
- compute log returns inline instead of using `d_lr`
- transpose signal matrices if current row-major access is poorly coalesced
- fuse auxiliary kernels
- switch combo layout from SoA to AoS or vice versa for memory-coalescing reasons

These are real ideas, but they are **profiling-only** until you see evidence.

---

# 3. Corrections or refinements to the original plan

## Refinement 1

The original host-side resident `close` cache suggestion should be tightened:

- **Do not** rely on `host_ptr + len` alone.
- Use an explicit cache key, or disable host-side reuse when no trusted key exists.

## Refinement 2

If you add uninitialized host readback, prefer a shared `alloc_policy` helper if one already exists in the repo. If not, use conservative reusable `Vec<f32>` scratch first.

## Refinement 3

If you cache kernel launch metadata, caching the capability results is safer and simpler than trying to over-engineer raw CUDA-function lifetime management.

## Refinement 4

If you add combo-range validation, decide intentionally where it lives:

- always-on preflight in outer orchestration
- debug-only local validation in the kernel wrapper
- benchmark probe validation but not timed-iteration validation

That choice affects whether the guardrail is “free enough” for production hot paths.

---

# 4. Updated implementation order

This is the order I would use after including the addendum items.

## Tier 1 — still the best first-wave changes

1. Reuse `d_close` / `d_lr` / combo / `d_out` / host readback scratch in `cdmc_gpu_kernel.rs`
2. Keep GPU CDMC prep device-native and stop routing GPU work through host-vector helpers
3. Add strict GPU route assertions in CDMC validation benchmarks
4. Fix the extra host-side MAMA/FAMA pass, or at least cache its runtime/scratch while it remains

## Tier 2 — add these immediately after

5. Add explicit cache-key-safe resident host upload reuse
6. Add combo-range/window validation strategy
7. Cache device caps and kernel caps inside `CdmcCudaRuntime`
8. Reduce full zero-fill in `build_cdmc_desired_positions(...)`

## Tier 3 — useful but smaller

9. Cache descriptor/output/key resolution with `OnceLock`
10. Reduce `first_valid` scan work if deterministic warmup rules are confirmed
11. Align optimizer/validation benchmark shapes and optionally separate cold vs warm measurements

## Tier 4 — profiling-driven extras

12. Sort/bucket combos for locality
13. Compress repeated windows / stable row-index uploads
14. Pinned host memory + async D2H
15. Kernel-layout or fusion changes after Nsight evidence

---

# 5. Additional checklist items to append to the original checklist

## `cdmc_gpu_kernel.rs`

- [ ] Cache `DeviceLaunchCaps` in `CdmcCudaRuntime`
- [ ] Cache `KernelLaunchCaps` for `compute_log_returns_f32`
- [ ] Cache `KernelLaunchCaps` for `cdmc_backtest_metrics_f32`
- [ ] Decide whether kernel-function handles themselves should be cached or re-fetched
- [ ] Add strict combo-row and window validation path
- [ ] Decide whether that strict validation runs always, debug-only, or only in preflight probes
- [ ] Add an explicit `close_host_cache_key` or equivalent trusted cache identity
- [ ] Disable host-side resident close reuse when no trusted cache key is available
- [ ] Add tests that mutate host-close contents across calls and confirm cache invalidation works

## `cdmc.rs`

- [ ] Consider replacing repeated output/key resolution with a `OnceLock` CDMC descriptor cache
- [ ] Confirm whether DPO / correlation-cycle / decycler first-valid positions are deterministic in the CPU path
- [ ] If confirmed, stop scanning those vectors when computing `first_valid`

## Benchmarks / perf gates

- [ ] Decide whether you want warm-steady-state timing only, or both cold-first-run and warm timing
- [ ] If warm timing is the target, add an explicit untimed warm-up comment or warm-up call in the CDMC benchmarks
- [ ] If first-hit latency matters, add a separate cold-path measurement strategy instead of mixing it with Criterion steady-state samples

## Tests

- [ ] Add a test that invalid combo row indices are rejected cleanly
- [ ] Add a test that invalid `start_t` / `end_t` windows are rejected cleanly
- [ ] Add a test that resident host-upload cache is bypassed when cache key is absent
- [ ] Add a test that resident host-upload cache refreshes when cache key changes
- [ ] Add a test that cached launch metadata does not change output correctness across repeated runs

---

# 6. Final takeaway

I do **not** think the original plan missed a bigger performance lever than the ones already identified.

The most important additions are:

1. **cache-key-safe host upload reuse**
2. **combo-range/window validation guardrails**
3. **kernel metadata / launch-cap caching**
4. **optional host-path scan reduction and descriptor caching**

If you only add a few things to the original plan, make them those four.
