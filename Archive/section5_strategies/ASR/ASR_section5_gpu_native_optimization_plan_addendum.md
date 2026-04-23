# ASR section-5 GPU-native optimization plan — addendum and corrections

This addendum supplements the original ASR section-5 plan and focuses on things that were either under-emphasized, missing, or useful as optional phase-2 work.

The main theme is unchanged:

- keep the current ASR feature surface intact
- improve warm-path and repeated-run performance
- remove unnecessary host/device copies
- use uninitialized memory only where overwrite is guaranteed
- tighten correctness guards so GPU speedups do not come from undefined behavior or silent fallback

Files covered again in this addendum:

- `asr.rs`
- `asr_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

---

# What I would improve in the original plan

## 1. I would strengthen the cache-correctness story

In the original plan I used a sampled `SeriesCacheKey { ptr, len, first_bits, mid_bits, last_bits }` as a sketch for resident market-series reuse.

That is useful as a conceptual example, but for production correctness it is not my preferred final design.

Why:

- pointer identity alone is not enough if the caller mutates a slice in place
- a sample-based fingerprint can miss content changes
- stale device caches are correctness bugs, not just performance bugs

### What I would do instead

Use one of these, in order of preference:

1. **explicit immutable generation tokens from the caller**
2. **stable `Arc`-backed identities for precomputed signal sets**
3. **a full-content hash computed once at signal construction time**
4. only as a debug/dev fallback, pointer+sample fingerprints

For correctness-sensitive paths, I would not rely on sampled content fingerprints as the sole cache key.

### Safer pattern

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AsrGeneration(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MarketCacheKey {
    generation: AsrGeneration,
    len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SignalCacheKey {
    generation: AsrGeneration,
    rows: usize,
    cols: usize,
}
```

Then the runtime cache becomes explicit and safe:

```rust
struct AsrCudaRuntime {
    // existing fields ...
    market_key: Option<MarketCacheKey>,
    fast_sq_key: Option<SignalCacheKey>,
    fast_mom_key: Option<SignalCacheKey>,
    slow_ang_key: Option<SignalCacheKey>,
    slow_er_key: Option<SignalCacheKey>,
    slow_z_key: Option<SignalCacheKey>,
    kaufman_long_key: Option<SignalCacheKey>,
    kaufman_short_key: Option<SignalCacheKey>,
}
```

### Why this matters

The original plan already improved **allocation reuse**. This change improves **copy avoidance without correctness risk**.

That matters because repeated H2D uploads of unchanged ASR matrices can dominate total GPU-path cost.

---

## 2. I would add strict combo-range and launch-parameter validation

The current `validate_inputs(...)` and `validate_inputs_device_rows(...)` check lengths, but not all value ranges.

That means bad combo values can still reach the kernel.

### Missing validations I would add

- `fast_row_index[i]` must be in `[0, fast_rows)`
- `slow_row_index[i]` must be in `[0, slow_rows)`
- `start_t[i]` and `end_t[i]` must satisfy the kernel’s window contract
- all `usize -> i32` and `usize -> u32` launch conversions should be checked, not cast with `as`

This is primarily a correctness change, but it also improves benchmark trust because invalid inputs fail fast on the host instead of producing GPU-only undefined behavior.

### Suggested helper

```rust
fn checked_i32(value: usize, what: &str) -> Result<i32, String> {
    i32::try_from(value)
        .map_err(|_| format!("ASR GPU {} exceeds i32 range: {}", what, value))
}

fn checked_u32(value: usize, what: &str) -> Result<u32, String> {
    u32::try_from(value)
        .map_err(|_| format!("ASR GPU {} exceeds u32 range: {}", what, value))
}

fn validate_combo_ranges(
    cols: usize,
    fast_rows: usize,
    slow_rows: usize,
    combos: &AsrGpuComboBatch<'_>,
    end_is_exclusive: bool,
) -> Result<(), String> {
    for (i, &row) in combos.fast_row_index.iter().enumerate() {
        if row < 0 || (row as usize) >= fast_rows {
            return Err(format!(
                "ASR GPU fast_row_index[{}] out of range: {} (rows={})",
                i, row, fast_rows
            ));
        }
    }

    for (i, &row) in combos.slow_row_index.iter().enumerate() {
        if row < 0 || (row as usize) >= slow_rows {
            return Err(format!(
                "ASR GPU slow_row_index[{}] out of range: {} (rows={})",
                i, row, slow_rows
            ));
        }
    }

    for i in 0..combos.start_t.len() {
        let s = combos.start_t[i];
        let e = combos.end_t[i];
        if s < 0 || e < 0 {
            return Err(format!(
                "ASR GPU combo window[{}] contains negative bounds: start={} end={}",
                i, s, e
            ));
        }
        let s = s as usize;
        let e = e as usize;

        if end_is_exclusive {
            if s >= cols || e > cols || s >= e {
                return Err(format!(
                    "ASR GPU combo window[{}] invalid for exclusive end: start={} end={} cols={}",
                    i, s, e, cols
                ));
            }
        } else if s >= cols || e >= cols || s > e {
            return Err(format!(
                "ASR GPU combo window[{}] invalid for inclusive end: start={} end={} cols={}",
                i, s, e, cols
            ));
        }
    }

    Ok(())
}
```

### Important note

I do **not** have the CUDA kernel source in the provided files, only the Rust launch site and PTX embedding route. So the exact `start_t/end_t` contract must match the real kernel ABI. If `end_t` is exclusive, use the exclusive branch above; if inclusive, adjust accordingly.

---

## 3. I would make “skip unchanged signal upload” a first-class optimization

The original plan reused device buffers, which is good, but that alone does not skip the actual H2D copies for unchanged ASR indicator matrices.

That is an important missing optimization.

### Why it matters

For repeated ASR optimization and validation runs, the following data often stays unchanged across many combo batches:

- `fast_squeeze_row_major`
- `fast_momentum_row_major`
- `slow_angle_row_major`
- `slow_er_row_major`
- `slow_z_row_major`
- `kaufman_long`
- `kaufman_short`

If those are recomputed once and then reused across combo partitions, the best warm-path optimization is not just “reuse the buffer.” It is “reuse the already-uploaded device contents.”

### Pattern to add

```rust
fn upload_signal_if_changed<T: DeviceCopy>(
    slot: &mut Option<DeviceBuffer<T>>,
    slot_key: &mut Option<SignalCacheKey>,
    new_key: SignalCacheKey,
    src: &[T],
) -> Result<(), String> {
    if *slot_key == Some(new_key) {
        return Ok(());
    }
    upload_slice_reuse(slot, src)?;
    *slot_key = Some(new_key);
    Ok(())
}
```

### Practical guidance

- use this only when the key is correctness-safe
- keep the old always-upload path as fallback when no safe key is available
- do not silently assume host slices are immutable unless that is actually guaranteed by the caller

### Recommended priority

This is a **P0/P1-level warm-path improvement** if your ASR optimizer or validator uses repeated combo batches against stable signal matrices.

---

## 4. I would sort compacted row subsets and add a “compact only when it helps” heuristic

The original plan already proposed compaction of device-row packing to only the used rows.

I would add two refinements:

1. **sort the used rows ascending** before gathering row pointers
2. **only compact when the used-row fraction is materially below full coverage**

### Why sorting helps

If used rows are gathered in first-seen combo order, source-row reads can become effectively random.

If used rows are sorted ascending, the pack kernel is much more likely to read source rows in contiguous order, which should help memory access behavior.

### Why a heuristic helps

If a combo batch references 95% of fast rows and 97% of slow rows, compaction adds CPU remap/gather work for little memory reduction.

### Suggested helper

```rust
fn remap_combo_rows_sorted(
    combo_rows: &[i32],
    total_rows: usize,
) -> Result<(Vec<i32>, Vec<usize>), String> {
    let mut used_rows: Vec<usize> = Vec::with_capacity(combo_rows.len());
    for &row in combo_rows {
        if row < 0 || (row as usize) >= total_rows {
            return Err(format!(
                "ASR GPU combo row index out of range: row={} total_rows={}",
                row, total_rows
            ));
        }
        used_rows.push(row as usize);
    }

    used_rows.sort_unstable();
    used_rows.dedup();

    let mut inverse = vec![-1i32; total_rows];
    for (new_idx, &old_idx) in used_rows.iter().enumerate() {
        inverse[old_idx] = i32::try_from(new_idx)
            .map_err(|_| format!("ASR GPU compact row index exceeds i32 range: {}", new_idx))?;
    }

    let mut remapped = Vec::with_capacity(combo_rows.len());
    for &row in combo_rows {
        remapped.push(inverse[row as usize]);
    }

    Ok((remapped, used_rows))
}

fn should_compact_rows(used_rows: usize, total_rows: usize) -> bool {
    if total_rows == 0 {
        return false;
    }
    let frac = used_rows as f64 / total_rows as f64;
    frac <= 0.85
}
```

### Practical route policy

- if `should_compact_rows(...)` is true, compact and repack only the used rows
- otherwise, skip compaction and use the full row set
- once you have a true pointer-native ASR kernel, this heuristic becomes less important

---

## 5. I would add a small ASR kernel-phase microbenchmark and timing instrumentation

The existing optimization and validation benches are useful, but they are not enough to explain *why* one route is slower.

What is still missing is a small strategy-local measurement surface for the ASR GPU route itself.

### What to time

In `asr_gpu_kernel.rs`, add opt-in timing around:

- market-series prep / upload
- dense signal upload
- row-pointer upload
- row packing
- scoring kernel launch
- D2H readback
- decode to `Metrics`

### Low-risk host-side timing pattern

```rust
#[derive(Default, Debug, Clone)]
struct AsrGpuPhaseTimings {
    market_upload_ms: f64,
    dense_signal_upload_ms: f64,
    row_ptr_upload_ms: f64,
    row_pack_ms: f64,
    kernel_ms: f64,
    readback_ms: f64,
    decode_ms: f64,
}

fn elapsed_ms(start: std::time::Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000.0
}
```

Then, under a debug/perf feature:

```rust
#[cfg(feature = "asr-cuda-phase-timing")]
let mut timings = AsrGpuPhaseTimings::default();

#[cfg(feature = "asr-cuda-phase-timing")]
let t0 = std::time::Instant::now();
rt.ensure_market_series(signals.close_f64)?;
#[cfg(feature = "asr-cuda-phase-timing")]
timings.market_upload_ms = elapsed_ms(t0);
```

This is not perfect GPU timing, but it is still valuable for before/after comparisons.

### Why this matters

Without phase timings, it is easy to claim “kernel optimization” when the real win came from fewer copies or fewer allocations.

---

# Additional in-scope code edits I would add

## A. Add a convenience builder for device-native ASR signal bundles

This is partly ergonomics, partly correctness.

Right now the provided files expose the pieces needed for a device-row route, but not an obvious convenience builder that makes the device-native route the easy route.

That means callsites can accidentally fall back to dense host materialization even when the signals already exist on device.

### Suggested helper direction

In `asr.rs`, add helpers to collect row pointers from `AsrPeriodSweepDevice` and `AsrKaufmanStopsDevice`.

```rust
#[cfg(feature = "cuda")]
pub fn collect_device_row_ptrs(sweep: &AsrPeriodSweepDevice) -> Result<Vec<u64>, String> {
    let mut out = Vec::with_capacity(sweep.matrix.rows);
    let stride_bytes = (sweep.matrix.cols)
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or_else(|| "ASR row-stride overflow".to_string())? as u64;

    for row in 0..sweep.matrix.rows {
        let byte_off = (row as u64)
            .checked_mul(stride_bytes)
            .ok_or_else(|| "ASR row pointer overflow".to_string())?;
        let ptr = sweep.matrix.device_ptr
            .checked_add(byte_off)
            .ok_or_else(|| "ASR row pointer add overflow".to_string())?;
        if ptr == 0 {
            return Err(format!("ASR device row pointer is null at row {}", row));
        }
        out.push(ptr);
    }
    Ok(out)
}
```

### Why this helps

- makes the device-native route easy to use correctly
- reduces duplicate pointer-arithmetic code outside `asr.rs`
- makes future pointer-native kernels easier to adopt

---

## B. Add a debug-only full-overwrite check for uninitialized output buffers

The original plan already recommended using uninitialized host memory for readback where the kernel fully overwrites output.

I still agree with that.

But for correctness, I would add a one-time debug guard that proves the kernel really does write every output slot.

### Safer debug pattern

In debug builds or under a perf-validation feature:

1. allocate `d_out` with a sentinel value
2. launch the kernel
3. copy back
4. assert no sentinel remains

### Example

```rust
#[cfg(feature = "asr-debug-full-output-check")]
fn make_sentinel_vec(len: usize) -> Vec<f32> {
    vec![f32::from_bits(0x7fc0_0001); len]
}

#[cfg(feature = "asr-debug-full-output-check")]
fn assert_no_sentinel(data: &[f32]) -> Result<(), String> {
    let sentinel = f32::from_bits(0x7fc0_0001);
    if let Some((idx, _)) = data.iter().enumerate().find(|(_, v)| v.to_bits() == sentinel.to_bits()) {
        return Err(format!(
            "ASR GPU output slot {} was not overwritten by the kernel",
            idx
        ));
    }
    Ok(())
}
```

This is slower and should not stay on in release benches, but it makes the `MaybeUninit` path much safer to adopt with confidence.

---

## C. Add checked launch-dimension construction

Instead of hand-writing many `as i32` conversions inline, centralize them.

### Suggested helper

```rust
struct AsrLaunchDims {
    t_i: i32,
    fast_rows_i: i32,
    fast_stride_i: i32,
    slow_rows_i: i32,
    slow_stride_i: i32,
    combo_count_i: i32,
    out_stride_i: i32,
    block_x: u32,
    grid_x: u32,
}

fn build_launch_dims(
    cols: usize,
    fast_rows: usize,
    slow_rows: usize,
    combo_count: usize,
    out_stride: usize,
) -> Result<AsrLaunchDims, String> {
    let block_x = asr_eval_block_x(combo_count);
    let combo_u32 = checked_u32(combo_count, "combo_count")?;
    let grid_x = combo_u32.div_ceil(block_x);

    Ok(AsrLaunchDims {
        t_i: checked_i32(cols, "cols")?,
        fast_rows_i: checked_i32(fast_rows, "fast_rows")?,
        fast_stride_i: checked_i32(cols, "fast_stride")?,
        slow_rows_i: checked_i32(slow_rows, "slow_rows")?,
        slow_stride_i: checked_i32(cols, "slow_stride")?,
        combo_count_i: checked_i32(combo_count, "combo_count")?,
        out_stride_i: checked_i32(out_stride, "out_stride")?,
        block_x,
        grid_x,
    })
}
```

That reduces repetitive unsafe-ish conversions and makes the launch contract easier to audit.

---

## D. Add ASR optimizer benchmark variants for strategy-flag combinations

The original plan added ASR optimizer coverage and generic-zscore coverage.

I would add one more layer for **strategy flags that remain supported on GPU**:

- `long_only = true`
- `allow_flip = false`
- `trade_on_next_bar = true`

These flags change branch behavior inside the ASR kernel and should be covered at least once in optimizer and validation benches.

### Suggested helper in `gpu_native_expansion_opt.rs`

```rust
fn make_asr_req_with_strategy(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
    strategy: StrategyConfig,
    asr: ta_app_core::AsrStrategyParams,
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req.strategy = strategy;
    req.asr = Some(asr);
    req
}
```

Then add at least one alternate benchmark group such as:

- `asr/gpu_kernel/200k/default`
- `asr/gpu_kernel/200k/long_only`
- `asr/gpu_kernel/200k/no_flip`
- `asr/gpu_kernel/200k/trade_next_bar`

### Why this matters

It checks that your performance gains are not only true for the default `StrategyConfig`.

---

# Optional improvements that go beyond the original prompts

These are real ideas, but they are either broader than section 5, higher risk, or dependent on APIs not shown in the provided files.

---

## O1. Pinned/page-locked host memory for uploads and readback

If your `cust` version supports page-locked host buffers and async copies, this is a worthwhile follow-up.

### Why it can help

- faster H2D and D2H transfers than pageable host memory
- cleaner overlap opportunities with stream-based execution
- especially useful if the output readback volume remains large

### Caution

- API availability varies by crate version
- pinned memory is a finite system resource
- this is an optimization on top of copy elimination, not a substitute for it

### Recommendation

Do this **after** you remove redundant copies. Do not optimize a copy that should not exist in the first place.

---

## O2. Async copy + overlap on the same stream

Today the path is effectively:

1. upload
2. launch
3. synchronize
4. copy back
5. decode

If your runtime and wrapper APIs support it, a later phase can move to stream-aware async H2D/D2H and reduce explicit synchronization points.

This is most useful once:

- device buffers are persistent
- signal uploads are skipped when unchanged
- pinned memory is available

Otherwise the simpler sync model is easier to reason about.

---

## O3. True device-side top-k / score-only mode

This is the largest optional optimization beyond the current prompt.

### Why it matters

If the optimizer only needs ranking information for many combos, reading back a full `Metrics` row for every combo can be more data than necessary.

A higher-ceiling design is:

- compute a scalar objective on device
- do top-k selection or threshold filtering on device
- only read back the finalists or reduced metrics

### Why I did not put this in the original plan as first-wave work

- it changes the surrounding API/contracts more than the copy-elision work does
- it requires more parity and UX verification
- it is a bigger design step than the current ASR files alone can justify

Still, if ASR search spaces get much larger, this can become a major win.

---

## O4. Evaluate whether `thread_local!` runtime caching is the right long-term policy

`ASR_GPU_RUNTIME` is currently thread-local.

That is simple and safe, but it also means:

- each host thread can have its own CUDA runtime/cache state
- module JIT, market cache, and signal caches may be duplicated per thread
- VRAM usage can scale with host thread count

### Optional follow-up

Consider whether a process-global per-device runtime cache would serve production workloads better.

### Caution

This is a concurrency design decision, not just a micro-optimization.

I would **not** do this before the warm-path copy eliminations land, but it is worth revisiting after measurement.

---

## O5. Kernel-function lookup caching

`module.get_function("asr_backtest_metrics_f32")` and `module.get_function("pack_rows_u64_ptr_f32")` are currently done at runtime call sites.

This may be tiny, but once the larger bottlenecks are fixed, it can become measurable.

If the `cust` lifetimes work cleanly for your version, cache resolved kernel handles inside `AsrCudaRuntime`.

I consider this **low priority**.

---

## O6. Block-size and occupancy tuning for `asr_backtest_metrics_f32`

The current block heuristic is:

- 64 for small combo counts
- 128 for medium
- 256 otherwise

That is a sensible start, but it is still just a heuristic.

### Optional next step

Once copies are reduced, measure:

- 64 / 128 / 256 / 512 block sizes
- occupancy and achieved throughput
- whether compacted row sets change the optimal launch configuration

### Caution

I could not inspect the actual CUDA kernel source in the provided files, so I cannot make strong claims about register pressure or shared-memory usage. Treat this as a profiling task, not a certainty.

---

## O7. Middle-ground pack fusion if a true pointer-native kernel is delayed

If you cannot yet move all the way to a row-pointer-native ASR scoring kernel, there is a medium step:

- reduce the number of pack launches
- fuse multiple pack operations where practical
- or reuse previously packed matrices when the compact row subsets repeat

This is not as good as consuming row pointers directly, but it can still reduce launch overhead.

---

# Additional benchmark and test ideas

## 1. Add a small direct `asr_gpu_kernel` microbenchmark

The current benches go through larger optimization/validation flows.

Add a narrower benchmark that exercises only:

- dense upload path
- device-row path with full-row packing
- device-row path with compacted-row packing

That will show whether your changes helped the ASR kernel route itself, not just the surrounding workflow.

## 2. Add a “used row fraction” sweep benchmark

For the device-row path, benchmark several cases:

- 10% of rows used
- 25% of rows used
- 50% of rows used
- 90% of rows used
- 100% of rows used

This will tell you where the compaction heuristic should switch on/off.

## 3. Add a correctness matrix for GPU-supported strategy flags

Explicitly test parity for:

- `long_only`
- `allow_flip = false`
- `trade_on_next_bar`
- direct-zscore route
- generic-zscore route

## 4. Add a negative-route matrix for unsupported features

Keep explicit blocker tests for:

- overlays enabled
- stop-loss enabled
- take-profit enabled

Those should not silently masquerade as GPU-native ASR passes.

---

# Additional checklist items to append to the original checklist

## Cache correctness

- [ ] Replace sampled cache fingerprints with correctness-safe generation keys where possible
- [ ] Avoid skipping uploads unless the cache identity is guaranteed safe
- [ ] Document which cache entries are identity-safe and which still require unconditional upload

## `asr_gpu_kernel.rs` correctness hardening

- [ ] Add combo fast-row index range validation
- [ ] Add combo slow-row index range validation
- [ ] Add combo window validation matching the real kernel contract
- [ ] Replace unchecked `as i32`/`as u32` launch conversions with checked helpers
- [ ] Add a centralized `build_launch_dims(...)` helper

## Dense-signal copy elimination

- [ ] Add optional “skip unchanged signal upload” support for dense signal matrices
- [ ] Add optional “skip unchanged signal upload” support for Kaufman-stop vectors
- [ ] Keep the unconditional upload path as a correctness-safe fallback
- [ ] Measure copy-elimination benefit separately from allocation-reuse benefit

## Device-row compaction refinements

- [ ] Sort compacted used-row sets ascending before gathering row pointers
- [ ] Add a heuristic for when compaction should be skipped
- [ ] Benchmark the used-row coverage threshold where compaction stops helping

## Uninitialized-memory safety

- [ ] Add a debug-only sentinel/full-overwrite check for `d_out`
- [ ] Keep release builds on uninitialized output only after overwrite is proven

## Benchmarks

- [ ] Add a direct ASR GPU-kernel microbenchmark
- [ ] Add a used-row-fraction benchmark for the device-row path
- [ ] Add ASR optimizer benchmarks for `long_only`, `allow_flip = false`, and `trade_on_next_bar`
- [ ] Separate copy-elimination wins from kernel-math wins in benchmark notes

## Optional follow-up work

- [ ] Decide whether pinned/page-locked host memory is worth introducing
- [ ] Decide whether stream-async copies are worth introducing after copy elimination lands
- [ ] Decide whether process-global per-device runtime caching is worth exploring
- [ ] Decide whether device-side objective/top-k reduction is worth a later design pass
- [ ] Decide whether kernel-function lookup caching is measurable enough to matter
- [ ] Profile block-size sensitivity after the copy/staging fixes land

---

# Bottom line

The original plan still stands, but if I were improving it today I would make these additions explicit:

1. **do not stop at buffer reuse; skip unchanged uploads where safe**
2. **tighten host-side range validation before every kernel launch**
3. **use correctness-safe cache identities, not sampled fingerprints, for production caching**
4. **sort and threshold compacted row subsets instead of compacting blindly**
5. **add direct ASR kernel-phase measurement so copy wins and kernel wins do not get mixed together**
6. **treat pinned memory, async copies, global runtime caches, and device-side top-k as optional phase-2 work**

Those additions improve both performance and correctness without removing ASR features or weakening route transparency.
