# Section 3 GPU Runtime / Residency Plan Addendum

This addendum captures additional findings after a second pass through the actual section-3 primary files.

It is intentionally narrower than the main plan and focuses on items that are either:

1. genuinely missing or under-emphasized in the first document, or
2. useful optional follow-ons that remain performance-first and correctness-preserving.

---

## 1) Additional high-value items to add to the main plan

### A. Delay `PreparedCudaIndicatorInputs` construction until the first actual GPU-routed dynamic indicator slot

### Why this should be added

`prepare_dynamic_indicator_slot_series_with_controls()` currently builds `PreparedCudaIndicatorInputs::from_candles(candles, ma_source)` before it knows whether any slot will actually use a CUDA route.

That means a plan that is entirely CPU-routed can still pay:

- f64 -> f32 host conversion for OHLCV
- CUDA runtime creation / acquisition
- device uploads for OHLCV
- source override preparation

This is especially wasteful for dynamic plans that compile under CUDA builds but resolve many slots to CPU.

### Current shape (simplified)

```rust
let prepared_cuda_inputs =
    match PreparedCudaIndicatorInputs::from_candles(candles, ma_source) {
        Ok(v) => v,
        Err(_) => {
            return prepare_dynamic_indicator_slot_series_cpu_with_controls(
                plan, candles, ma_source, cancel, progress,
            );
        }
    };

for slot in &plan.indicator_plan {
    let computed = match slot.route {
        IndicatorRouteKind::Cpu => compute_dynamic_indicator_cpu(...),
        IndicatorRouteKind::GpuSingle | IndicatorRouteKind::GpuKernel => {
            compute_dynamic_indicator_cuda_with_inputs(&prepared_cuda_inputs, ...)
        }
    };
}
```

### Recommended edit

Construct prepared CUDA inputs lazily only when the first GPU-routed slot is encountered.

```rust
let mut prepared_cuda_inputs: Option<PreparedCudaIndicatorInputs> = None;

for slot in &plan.indicator_plan {
    let computed = match slot.route {
        IndicatorRouteKind::Cpu => compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            source_indicator_id,
            slot.output_id.as_deref(),
            slot.params.as_slice(),
            Kernel::Auto,
        )
        .map_err(|e| e.to_string())?,
        IndicatorRouteKind::GpuSingle | IndicatorRouteKind::GpuKernel => {
            if prepared_cuda_inputs.is_none() {
                prepared_cuda_inputs = Some(
                    PreparedCudaIndicatorInputs::from_candles(candles, ma_source)
                        .map_err(|e| e.to_string())?,
                );
            }

            let prepared = prepared_cuda_inputs.as_ref().expect("just inserted");
            match compute_dynamic_indicator_cuda_with_inputs(
                prepared,
                source_indicator_id,
                slot.output_id.as_deref(),
                slot.params.as_slice(),
                Kernel::Auto,
                vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
            ) {
                Ok(out) => cast_cuda_output_to_cpu_series(out)?,
                Err(_) => compute_dynamic_indicator_cpu(
                    candles,
                    ma_source,
                    source_indicator_id,
                    slot.output_id.as_deref(),
                    slot.params.as_slice(),
                    Kernel::Auto,
                )
                .map_err(|e| e.to_string())?,
            }
        }
    };

    // existing validation / cache insertion
}
```

### Expected impact

- removes completely unnecessary CUDA prep on CPU-only or CPU-dominant dynamic plans
- reduces cold and warm dynamic-plan staging latency
- preserves all existing fallback behavior

### Risk

Low. This is a control-flow optimization, not a route-policy change.

---

### B. Split CUDA runtime lifetime from prepared dataset lifetime in `dynamic_indicator_adapter.rs`

### Why this should be added

`PreparedCudaIndicatorInputs::from_candles_on_device()` currently rebuilds both:

- the per-device `CudaRuntime`
- and the dataset-specific host/device vectors

when the cached entry for a device is replaced.

That couples a long-lived device/runtime object to short-lived dataset identity. Reusing the runtime even when the dataset changes should reduce warm-path setup jitter and repeated initialization cost.

### Current shape (simplified)

```rust
thread_local! {
    static PREPARED_CUDA_INPUT_CACHE: RefCell<HashMap<usize, ResidentCudaIndicatorDataset>> =
        RefCell::new(HashMap::new());
}

if rebuild {
    let runtime = Arc::new(CudaRuntime::new(device_id)?);
    // convert OHLCV to f32
    // upload OHLCV
    cache.insert(device_id, ResidentCudaIndicatorDataset { runtime, ... });
}
```

### Recommended edit

Use a two-level structure:

- per-device runtime cache
- per-device dataset cache (single-entry or small LRU)

```rust
#[cfg(feature = "cuda")]
#[derive(Debug)]
struct PreparedCudaDeviceState {
    runtime: Arc<CudaRuntime>,
    dataset: Option<ResidentCudaIndicatorDataset>,
}

#[cfg(feature = "cuda")]
thread_local! {
    static PREPARED_CUDA_DEVICE_CACHE: RefCell<HashMap<usize, PreparedCudaDeviceState>> =
        RefCell::new(HashMap::new());
}
```

Then only rebuild `dataset` when the signature changes; leave `runtime` intact.

### Optional extension

If you later move to a small LRU, keep:

- one runtime per device
- N dataset entries per device

rather than N runtimes.

### Expected impact

- smoother warm behavior when datasets change frequently
- less repeated CUDA runtime/context setup work
- cleaner separation between runtime and residency concerns

### Risk

Low to medium. Mostly a local structural cleanup.

---

### C. Cache resolved CUDA function handles and launch-cap metadata inside `CudaKernelRuntime`

### Why this should be added

`optimize_double_ma_gpu_kernel_impl()` repeatedly resolves kernel symbols via `bt_module.get_function(...)` and then recomputes launch-cap metadata for those functions during the optimization path.

Examples include:

- `compute_log_returns_f32`
- `transpose_row_to_tm`
- `double_cross_backtest_tm_flex_f32`
- `double_cross_backtest_tm_flex_f32_windowed`
- optional `m7` / lockstep / fast-warp variants
- `fill_f32`

Those lookups are not the dominant cost versus full kernel launches, but they are still avoidable warm-path overhead and they make the runtime path noisier than it needs to be.

### Recommended edit

Extend `CudaKernelRuntime` with resolved function handles and precomputed launch-cap metadata.

```rust
struct ResolvedKernelFns {
    compute_lr: Function,
    transpose: Function,
    backtest: Function,
    backtest_windowed: Function,
    backtest_m7: Option<Function>,
    backtest_lockstep: Option<Function>,
    backtest_lockstep_m7: Option<Function>,
    backtest_fast_warp: Option<Function>,
    fill_f32: Function,
}

struct ResolvedKernelCaps {
    compute_lr: KernelLaunchCaps,
    transpose: KernelLaunchCaps,
    backtest: KernelLaunchCaps,
    backtest_windowed: KernelLaunchCaps,
    backtest_m7: Option<KernelLaunchCaps>,
    backtest_lockstep: Option<KernelLaunchCaps>,
    backtest_lockstep_m7: Option<KernelLaunchCaps>,
    backtest_fast_warp: Option<KernelLaunchCaps>,
    fill_f32: KernelLaunchCaps,
}

struct CudaKernelRuntime {
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    kernels: ResolvedKernelFns,
    kernel_caps: ResolvedKernelCaps,
    // existing fields...
}
```

Populate those once in `CudaKernelRuntime::new(device_id)`.

Then the hot path becomes:

```rust
let compute_lr = &rt.kernels.compute_lr;
let compute_lr_launch_caps = rt.kernel_caps.compute_lr;
let transpose = &rt.kernels.transpose;
let backtest = &rt.kernels.backtest;
```

### Expected impact

- lower warm-path control overhead
- fewer symbol-resolution calls in hot execution
- more stable timing in short/medium runs

### Risk

Low. This is runtime caching of immutable module metadata.

---

### D. Add an explicit “CPU-only dynamic staging should not touch CUDA” test

### Why this should be added

Once the lazy-preparation change above is implemented, it should be pinned with a regression test. Right now the current function shape makes it easy to reintroduce eager CUDA prep later.

### Recommended test

Add a test that:

- compiles a dynamic plan where all indicator slots resolve to `IndicatorRouteKind::Cpu`
- runs `prepare_dynamic_indicator_slot_series_with_controls`
- asserts no CUDA-prepared-input cache entry was created

A minimal test helper could expose cache size under `#[cfg(test)]`.

```rust
#[cfg(all(feature = "cuda", test))]
pub(crate) fn prepared_cuda_input_cache_len() -> usize {
    PREPARED_CUDA_INPUT_CACHE.with(|cell| cell.borrow().len())
}
```

Then:

```rust
#[test]
fn cpu_only_dynamic_staging_does_not_create_prepared_cuda_inputs() {
    reset_test_cuda_input_cache();
    // build CPU-routed plan
    // stage indicators
    assert_eq!(prepared_cuda_input_cache_len(), 0);
}
```

### Expected impact

Not a runtime improvement by itself, but it protects a useful warm-path optimization.

### Risk

Low.

---

## 2) Optional changes that go slightly beyond the original prompt but still fit your goals

These are still aligned with performance/correctness, but they are a little farther from the exact section-3 prompt center.

### A. Add benchmark modes that distinguish:

- process-cold
- thread-warm / cache-warm
- dataset-warm but runtime-recreated
- fully warm

### Why

`ta_app_path_faceoff.rs` is a strong app-path harness, but same-thread repeated runs can make thread-local runtime reuse look better than real request scheduling if the desktop app ever changes thread affinity. A small mode split would make results easier to interpret.

### Suggested artifact change

Add fields like:

```rust
#[derive(Debug, Serialize)]
struct TrialArtifact {
    trial_index: usize,
    runtime_ms_wall: f64,
    cache_scope: &'static str, // "process_cold" | "thread_warm" | "app_warm"
    result: ta_app_core::BackendOptimizationResult,
}
```

This does not change production behavior; it improves evidence quality.

### Risk

Low.

---

### B. Introduce a reusable pinned host slab / staging arena shared across GPU upload-heavy paths

### Why

If both `gpu_backtest_kernel.rs` and `dynamic_indicator_adapter.rs` keep growing their own upload/staging helpers, you may end up with duplicated temporary host buffers and duplicated host-side conversion logic. A shared pinned staging arena could:

- reuse one or a few host slabs
- support async H2D copies better
- centralize unsafe/uninitialized memory rules

### Possible shape

```rust
struct HostUploadArena {
    f32_slab: PinnedHostBuffer<f32>,
    i32_slab: PinnedHostBuffer<i32>,
}
```

Use it only behind very tight APIs so uninitialized regions cannot be observed before write completion.

### Why this is optional

It is useful, but it introduces more unsafe surface and should come after the simpler lazy-prep / runtime-caching wins.

### Risk

Medium.

---

### C. Revisit cache scope: thread-local versus process-level runtime/data residency

### Why

Both the kernel runtime and prepared CUDA input cache are thread-local today. That is safe and simple, but it can undercut warm-path wins when requests move across worker threads.

### Why this is beyond the original prompt

Changing cache scope has concurrency and teardown implications. It is not the first thing to change.

### Suggested direction

Do this only after:

1. faceoff artifacts clearly separate thread-warm from broader app-warm behavior
2. single-thread warm-path wins are already captured
3. tests exist for correct invalidation / device affinity / teardown

### Risk

Medium to high.

---

### D. Consider a field-aware / source-aware on-disk cache format only if app-path evidence says load/decode is still a dominant bottleneck

### Why

If `data_cache.rs` remains a meaningful portion of end-to-end wall time even after in-memory identity reuse is fixed, a more efficient disk format could help. But this is further from the current hot path than the other items above.

### Recommendation

Do not prioritize this until app-path artifacts prove dataset load/decode remains material after the in-memory and runtime-residency fixes.

### Risk

Medium.

---

## 3) Things I would *not* promote yet

### A. Aggressive `strategy_gpu_program.rs` cloning/fingerprint micro-optimization

There are some obvious owned-string / vector clones when compiling the GPU predicate program and ABI. But based on the files in scope, the larger question is whether the adjacent runtime layer is already caching executables by `plan_fingerprint_hex`. Without that adjacent file, I would treat deeper optimization here as an open question.

### B. Broad process-wide residency rewrite

Useful in theory, but too large and too risky compared with the smaller wins above.

### C. New feature-level route policy changes

Out of scope unless app-path measurements show a confirmed routing bug or setup cliff.

---

## 4) Updated recommended priority order

1. **Lazy `PreparedCudaIndicatorInputs` construction in `strategy_dynamic.rs`**
2. **Split `CudaRuntime` lifetime from dataset lifetime in `dynamic_indicator_adapter.rs`**
3. **Cache resolved CUDA functions + launch caps in `CudaKernelRuntime`**
4. **Add regression test ensuring CPU-only dynamic staging does not touch CUDA**
5. **Expand `ta_app_path_faceoff.rs` artifact modes only if you need clearer warm/cold attribution**
6. **Consider shared pinned host upload arena only after the simpler wins land**
7. **Revisit process-level cache scope only after benchmark evidence justifies it**

---

## 5) Net assessment

The first MD plan covered most of the big-ticket issues correctly.

The main things worth adding are:

- a **missed eager-CUDA-prep cost** in dynamic indicator staging
- a **runtime-versus-dataset cache separation** in the prepared-input adapter
- a **missed warm-path symbol-resolution overhead** in `gpu_backtest_kernel.rs`
- a **test lock** to ensure CPU-only dynamic staging stays CUDA-free

Those additions stay aligned with your stated goal:

- fewer unnecessary memory copies
- fewer repeated conversions/uploads
- more stable warm performance
- better correctness and benchmark fidelity
- no feature removal and no route-capability regression
