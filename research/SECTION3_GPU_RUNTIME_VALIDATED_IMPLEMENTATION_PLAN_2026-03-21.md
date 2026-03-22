# Section 3 GPU Runtime Validated Implementation Plan

This document replaces the unvalidated portions of:

- `research/section3_gpu_runtime_comprehensive_optimization_plan (1).md`
- `research/section3_gpu_runtime_plan_addendum.md`

It is limited to repo-backed work for:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`

It keeps only the guide content that is supported by the current codebase, and it excludes or defers claims that are contradicted, overstated, or still measurement-dependent.

## Validation Summary

### Valid and ready to plan

1. `gpu_backtest_kernel.rs` does repeated resident-series upload synchronization today.
2. `dynamic_indicator_adapter.rs` eagerly converts and uploads all OHLCV fields on rebuild.
3. `strategy_dynamic.rs` eagerly creates prepared CUDA inputs even for CPU-only routed dynamic staging under CUDA builds.
4. `strategy_dynamic.rs` performs clone-heavy per-call dynamic indicator caching.
5. `strategy_dynamic.rs` recompiles plans more times than necessary across preview and runnable-plan preparation.
6. `strategy_dynamic.rs` always builds a CPU context for runnable plans, even when the resolved route is GPU.
7. `dynamic_indicator_adapter.rs` couples `CudaRuntime` lifetime to dataset lifetime per device.
8. `dynamic_indicator_adapter.rs` recomputes `first_valid` by scanning every time.
9. `dynamic_indicator_adapter.rs` silently falls back from prepared-device CUDA to host-backed CUDA.
10. `gpu_backtest_kernel.rs` repeatedly resolves module functions and recomputes per-kernel launch caps in hot paths.
11. `state.rs` always inserts a fresh base dataset on repeated loads of the same CSV.
12. `state.rs` retention is currently fixed at one base dataset plus one derived dataset.
13. `data_cache.rs` already has `SourceIdentity`, but it is not comparable and there is no identity-threaded `load_or_build_with_identity(...)`.
14. `ta_app_path_faceoff.rs` is useful, but it is a harness, not the full Tauri command path.

### Valid, but lower-confidence or later-phase

1. Small LRU expansion for resident GPU data and prepared CUDA datasets.
2. Configurable base/derived retention windows.
3. Lazy CPU-context construction for runnable dynamic plans.
4. Reducing host-side staging in `upload_double_ma_dynamic_program_batch`.
5. Replacing string/JSON dynamic cache keys with a structured hashed key.

These are structurally reasonable, but their payoff depends more heavily on real workload alternation and benchmark evidence.

### Exclude or explicitly defer

1. Do not treat `vram_ma.rs` as a direct optimization target. It is only a thin re-export seam here.
2. Do not describe GPU program or ABI upload reuse as an open question. `strategy_gpu_runtime.rs` already caches uploaded executables by plan fingerprint, device, and ABI version.
3. Do not describe `ta_app_path_faceoff.rs` as the full truth source for production behavior. It bypasses the actual Tauri command boundary.
4. Do not claim faceoff records only wall-clock time. It already captures `runtime_ms_wall` plus the full `BackendOptimizationResult`, which includes `runtime_ms` and may include `hybrid_timing_ms`.
5. Do not prioritize adding a `field_mask` to `PreparedCudaDatasetSignature` before a lazy/partial-field redesign. Today every resident prepared entry still materializes all OHLCV fields.
6. Do not prioritize process-wide cache-scope rewrites, shared pinned upload arenas, or on-disk cache format changes without fresh measurement evidence.

## Repo-Grounded Implementation Plan

## 1. GPU Runtime and Residency

### 1.1 Batch resident market uploads

Problem:

- `ensure_resident_market_series(...)` uploads `close`, `high`, `low`, `volume`, and source overrides through helpers that create a new `AsyncPinnedUploadGuard` and finalize it immediately.
- `AsyncPinnedUploadGuard::finalize()` synchronizes the stream when pinned buffers were used.

Implementation:

- Add a batched resident-upload helper in `gpu_backtest_kernel.rs`.
- Use one upload guard for all new series needed during a resident rebuild or resident miss-fill.
- Finalize once after the batch finishes.

Expected result:

- Lower cold-switch cost.
- Lower first-hit penalty for optional `high`/`low`/`volume`/derived-source uploads.
- Fewer avoidable stream synchronizations.

### 1.2 Keep safe uninitialized conversion helpers local and reusable

Problem:

- `upload_f32_source_to_device(...)` still uses `collect::<Vec<f32>>()` in the non-pinned `f64 -> f32` path.
- `simple_returns_from_log_returns_f32(...)` still allocates via iterator collection.

Implementation:

- Add small `alloc_uninit_f32(...)`-based conversion helpers in `gpu_backtest_kernel.rs`.
- Use them only where every element is fully written before any read.

Expected result:

- Small but safe host-side allocation cleanup in hot conversion paths.

### 1.3 Cache resolved kernel functions and kernel launch caps

Problem:

- Hot kernel paths repeatedly call `module.get_function(...)`.
- They also recompute `detect_kernel_launch_caps(...)` for those resolved functions.

Implementation:

- Extend `CudaKernelRuntime` to cache:
  - immutable `Function` handles
  - per-function `KernelLaunchCaps`
- Populate them once in `CudaKernelRuntime::new(...)`.

Expected result:

- Lower control overhead in warm runs.
- Less repeated symbol lookup noise in short and medium GPU runs.

### 1.4 Defer resident-market LRU until after the first measured wins

Current state:

- `resident_market` is a single-entry `Option<ResidentMarketSeriesCache>`.

Plan:

- Keep single-entry behavior for the first implementation wave.
- Revisit a tiny LRU only after batch-upload and state-layer dataset reuse are measured.

Reason:

- The single-entry design is real.
- The claim that it is the next best optimization is workload-dependent.

### 1.5 Revisit reset-time synchronization only after eviction policy changes are measured

Problem:

- `reset_resident_market_cache(...)` synchronizes the stream before dropping the resident entry.
- That is correct today, but it may become an avoidable warm/cold spike once resident cache capacity or eviction policy changes.

Implementation:

- Keep the current conservative behavior in the first implementation wave.
- If a tiny resident-market LRU is added later, revisit whether synchronization can be limited to true eviction and in-flight safety boundaries instead of every logical resident reset.

Expected result:

- Better control over switch-time latency once multi-entry residency exists.

Reason for deferral:

- This is a valid performance surface, but it is secondary to batched uploads and state-layer dataset reuse.

## 2. Dynamic Indicator Prepared Inputs and Dynamic Strategy Staging

### 2.1 Delay prepared CUDA input construction until the first GPU-routed slot

Problem:

- `prepare_dynamic_indicator_slot_series_with_controls(...)` eagerly calls `PreparedCudaIndicatorInputs::from_candles(...)` before it knows whether any slot will use `GpuSingle` or `GpuKernel`.
- CPU-only routed plans under CUDA builds still pay prepared-input setup cost.

Implementation:

- Change `strategy_dynamic.rs` to hold `Option<PreparedCudaIndicatorInputs>`.
- Construct it lazily on the first GPU-routed slot.
- Preserve current CPU fallback behavior when CUDA prep or CUDA compute fails.

Expected result:

- CPU-only dynamic staging no longer touches CUDA prep.
- Lower cold and warm staging cost for CPU-dominant plans.

### 2.2 Split device runtime lifetime from prepared dataset lifetime

Problem:

- `PreparedCudaIndicatorInputs::from_candles_on_device(...)` rebuilds the `CudaRuntime` and the dataset-specific buffers together whenever the signature changes.

Implementation:

- Refactor `dynamic_indicator_adapter.rs` to keep:
  - one runtime per device
  - a separate dataset cache per device
- Rebuild dataset-specific host/device buffers on signature change without recreating the runtime.

Expected result:

- Less runtime-setup churn when switching datasets on the same device.

### 2.3 Replace eager OHLCV preparation with on-demand field preparation

Problem:

- Rebuild currently converts and uploads `open`, `high`, `low`, `close`, and `volume` unconditionally.

Implementation:

- Refactor `ResidentCudaIndicatorDataset` and `PreparedCudaIndicatorInputs` so field materialization is driven by actual `IndicatorInputKind` needs.
- Preserve current API behavior at call sites by providing accessor methods that lazily ensure the required field exists.
- Keep derived-source overrides lazy as they are today.

Expected result:

- Less host conversion and H2D work for `Slice`, `HighLow`, and other narrower input sets.

Note:

- A `field_mask` in the dataset signature becomes relevant only after this redesign. It is not a standalone priority item.

### 2.4 Cache `first_valid` results by input-kind

Problem:

- `PreparedCudaIndicatorInputs::first_valid_for_input_kind(...)` rescans the prepared series every time.

Implementation:

- Add memoized `first_valid` storage per `IndicatorInputKind` inside prepared inputs.
- Populate lazily on first request.

Expected result:

- Small cumulative win for repeated indicator dispatch on the same prepared dataset.

### 2.5 Make prepared-device fallback explicit in runtime attribution

Problem:

- `compute_dynamic_indicator_cuda_with_inputs(...)` silently falls back from prepared-device CUDA to host-backed CUDA.

Implementation:

- Preserve the fallback for correctness.
- Add explicit attribution, logging, or counters that record:
  - prepared-device path used
  - prepared-device path failed
  - host-backed CUDA fallback used

Expected result:

- Better route transparency for debugging and benchmarking.

### 2.6 Remove clone-heavy dynamic staging payload copies

Problem:

- Both CPU and CUDA staging paths keep a per-call `HashMap<String, DynamicIndicatorComputeOutput>`.
- Cache hits and cache inserts clone owned `Vec<f64>` payloads.

Implementation:

- Convert the local cache values to shared ownership, for example `Arc<DynamicIndicatorComputeOutput>`.
- Keep the public return shape stable if needed by cloning only once at the final boundary, or refactor direct consumers to accept shared outputs.

Expected result:

- Less repeated copying of large indicator result vectors within one staging pass.

### 2.7 Collapse repeated plan compilation

Problem:

- `preview_dynamic_double_ma_strategy_ir(...)` compiles at least once and may recompile on fallback.
- `prepare_runnable_dynamic_double_ma_plan(...)` then recompiles CPU and may compile GPU again.
- `StrategyHandle::from_plan(...)` compiles GPU predicate program and ABI from the GPU plan.

Implementation:

- Introduce a small compile bundle helper that returns the preview plus any compiled CPU and GPU plans already proven valid.
- Reuse those compiled plans when building the runnable plan and GPU handle.

Expected result:

- Lower repeated compile and validation overhead during dynamic-plan setup.

### 2.8 Replace per-call string/JSON cache keys with a structured key

Problem:

- `dynamic_indicator_cache_key(...)` serializes params to JSON and builds a long string key even though the dedupe cache is scoped to a single staging pass.

Implementation:

- Replace the current string key with a structured key type or a stable hashed key built from:
  - dataset signature
  - route
  - source indicator id
  - output id
  - normalized params
- Keep the existing dedupe behavior unchanged.

Expected result:

- Lower per-slot cache-key construction overhead during CPU and CUDA staging passes.

### 2.9 Reframe lazy CPU-context construction as a second-wave change

Problem:

- `prepare_runnable_dynamic_double_ma_plan(...)` always builds `cpu_context`.
- Current consumers dereference `runtime_plan.cpu_context` directly.

Implementation:

- Do not do this in the first wave.
- After the earlier wins land, change `RunnableDynamicDoubleMaPlan` to support lazy CPU-context construction or a `OnceLock`-backed context.

Expected result:

- Avoid eager CPU-side staging for GPU-resolved plans.

Reason for deferral:

- This is valid work, but it is not a local-only edit.

## 3. State, Data Identity, and App-Path Measurement

### 3.1 Reuse identical base dataset loads

Problem:

- `AppState::load_price_data(...)` always inserts a fresh base dataset and a new `data-N` id even when the same CSV is loaded again unchanged.

Implementation:

- Extend `LoadedDataContext` with `SourceIdentity`.
- Add a state-layer lookup that reuses an existing base dataset when:
  - `csv_path` matches
  - timeframe is `None`
  - `SourceIdentity` matches
- Update last-access tracking on reuse.

Expected result:

- Better warm reuse for repeated load/open flows.
- Fewer avoidable dataset-id churn events that invalidate pointer-signature-based caches.

### 3.2 Thread precomputed identity through `data_cache`

Problem:

- `load_price_data(...)` has no way to pass a precomputed identity into `data_cache::load_or_build(...)`.

Implementation:

- Derive `PartialEq, Eq` for `SourceIdentity`.
- Add `load_or_build_with_identity(...)` alongside the current `load_or_build(...)`.
- Use the identity-threaded version when state has already computed identity for reuse checks.

Expected result:

- Cleaner state/data-cache integration.
- Avoids redundant source-identity work on the reuse path.

### 3.3 Treat retention widening as configurable and evidence-driven

Problem:

- Retention is hard-coded at one base plus one derived dataset.

Implementation:

- Make retention limits configurable by environment or config.
- Keep defaults conservative unless fresh measurements justify widening.

Expected result:

- Better chance of preserving warm data across common alternation patterns without forcing memory growth unconditionally.

### 3.4 Improve faceoff timing semantics without overstating what the harness is

Problem:

- `ta_app_path_faceoff.rs` is useful but currently blends harness wall time with the richer timing data already embedded in `BackendOptimizationResult`.

Implementation:

- Keep `runtime_ms_wall`.
- Add explicit serialized trial timing fields that separate:
  - harness wall time
  - engine-reported runtime from `result.runtime_ms`
  - any available hybrid timing fields already present in `result.hybrid_timing_ms`
- Add a short note in the artifact schema or README that this harness is same-thread app-path-like measurement, not the full Tauri command boundary.

Expected result:

- Better timing-boundary clarity without misrepresenting the harness.

## 4. Explicitly Deferred Items

These items should remain out of the first validated implementation pass:

1. Process-wide cache-scope rewrites for thread-local CUDA caches.
2. Shared pinned upload arenas across all GPU upload-heavy paths.
3. On-disk cache format redesign.
4. `vram_ma.rs`-specific optimization work.
5. Any work justified only by “this might dominate” language without benchmark evidence.

## Execution Order

## Wave 1: immediate, low-risk, repo-backed

1. Batch resident uploads in `gpu_backtest_kernel.rs`.
2. Add safe uninitialized conversion helpers in `gpu_backtest_kernel.rs`.
3. Delay prepared CUDA input creation in `strategy_dynamic.rs`.
4. Add prepared-device fallback attribution in `dynamic_indicator_adapter.rs`.
5. Reduce clone-heavy dynamic staging in `strategy_dynamic.rs`.
6. Add `SourceIdentity` equality plus base-load reuse in `state.rs` and `data_cache.rs`.
7. Improve `ta_app_path_faceoff.rs` trial timing fields using already-available timing data.

## Wave 2: still grounded, broader edits

1. Split `CudaRuntime` lifetime from dataset lifetime in `dynamic_indicator_adapter.rs`.
2. Add lazy field preparation for prepared CUDA inputs.
3. Memoize `first_valid` results.
4. Collapse repeated dynamic-plan compilation.
5. Replace dynamic string/JSON cache keys with a structured hashed key.
6. Cache resolved kernel functions and launch caps in `CudaKernelRuntime`.

## Wave 3: evidence-driven follow-ons

1. Configurable retention windows.
2. Small per-device LRU for prepared CUDA datasets.
3. Small resident-market LRU in `gpu_backtest_kernel.rs`.
4. Revisit resident reset-time synchronization if resident-market eviction policy changes.
5. Lazy CPU-context construction for runnable dynamic plans.
6. Reduced host staging in `upload_double_ma_dynamic_program_batch(...)`.

## Comprehensive Checklist

## Progress snapshot

- Completed:
  - resident upload batching and hot-path host conversion cleanup in `gpu_backtest_kernel.rs`
  - lazy prepared CUDA creation in dynamic staging
  - prepared-device fallback attribution
  - structured dynamic staging cache keys and lower-copy dedupe
  - runtime/dataset cache split for prepared CUDA inputs
  - memoized `first_valid` storage
  - compile-bundle reuse for dynamic preview/runnable plan creation
  - same-source base dataset reuse plus identity-threaded cache loading
  - configurable base/derived retention limits
  - lazy `cpu_context` construction for runnable dynamic plans
  - faceoff timing artifact separation and compatibility fields
  - selective prepared-field materialization for dynamic staging, direct dynamic CUDA entry, and selected `double_ma` regime-device builders
  - broader prepared-field adoption for the `CDMC` composite execution/validation family
  - broader prepared-field adoption across `ASR`, `VCC`, `VEMR`, `VCET`, and adjacent source-plus-HLC composite execution/validation wrappers
  - broader prepared-field adoption across the remaining `ROCB`, `MKCS`, `KQSR`, `CARM`, and `KVDF` composite optimization/validation wrappers, plus the remaining targeted `CDMC` wrapper cleanup
  - mixed input-kind regression coverage for selective prepared CUDA field materialization
  - baseline capture for CPU-only routed dynamic staging under CUDA builds
- Still open:
  - a non-regressing design for any `CudaKernelRuntime` function-handle cache
  - prepared-input tiny LRU follow-on, only if warm alternation data justifies the extra state
  - resident-market tiny LRU follow-on, only if warm alternation data justifies the extra state
- Current verification note:
  - `cargo test --workspace --no-fail-fast` passes on March 22, 2026; the workspace is green with non-blocking warnings still present
  - `run_workflow_gate.ps1 -Mode candidate` passes after the optional composite selective-prep follow-on work, with runtime `1701 ms` versus the `1744 ms` baseline
  - `run_gpu_perf_gate.ps1 -Mode candidate` passes after the optional composite selective-prep follow-on work, with `double_ma/gpu_kernel/sma_sma/200k/small` at `146.18989 ms` versus the `155.288513 ms` baseline and the dynamic-strategy ratio check still passing
  - `run_cpu_perf_gate.ps1 -Mode candidate` still fails on the existing `double_ma/cpu/sma_sma/200k/small` criterion benchmark (`230.765982 ms` versus `125.30684 ms` baseline), while the dynamic-strategy ratio check still passes (`1.199878` versus `1.070352`)
  - `ta_app_path_faceoff` cold/warm/reload artifacts were captured at `target/section3_faceoff_app_path_gpu_kernel_cold.json`, `target/section3_faceoff_app_path_gpu_kernel_fresh.json`, and `target/section3_faceoff_app_path_gpu_kernel_fresh_reload2.json`; dataset load was `2.7846 ms` cold, `3.1947 ms` warm, and `2.8558 ms` on repeated reload, while measured harness medians were `4923.0714 ms`, `5111.9316 ms`, and `5033.7356 ms`
  - `cargo bench -p ta_app_core --bench dynamic_strategy_cpu --features cuda -- "dynamic_strategy_cpu/staging_cuda_build/50k/cpu_route_slot_prep"` recorded a CPU-only routed dynamic staging mean of `370.659 µs` with estimates at `target/criterion/dynamic_strategy_cpu_staging_cuda_build_50k/cpu_route_slot_prep/new/estimates.json`
  - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "gpu_kernel_upload_batch_only"` recorded `upload_double_ma_dynamic_program_batch(...)` at about `198.089 ms` mean, while `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "gpu_kernel_warm_uploaded"` recorded warm uploaded execution at about `144.477 ms` mean; current evidence says upload staging is still a first-order cost surface
  - an attempted raw cached-function-handle implementation in `gpu_backtest_kernel.rs` regressed `run_gpu_perf_gate.ps1 -Mode candidate` to about `270.5 ms` on `double_ma/gpu_kernel/sma_sma/200k/small` and was reverted
  - targeted route-summary checks now pass again: `cargo test -p ta_app_core capability_report_marks_gpu_supported_for_plain_sma_request --lib`, `cargo test -p ta_app_core preview_dynamic_double_ma_strategy_ir_keeps_gpu_backend_for_plain_v1_shape --lib`, and `cargo test -p ta_app_core preview_dynamic_double_ma_strategy_ir_falls_back_to_cpu_for_halftrend_shape --lib`

## Validation and setup

- [x] Record a baseline with `src-tauri/src/bin/ta_app_path_faceoff.rs` for same-dataset warm runs.
- [x] Record a baseline for repeated reload of the same CSV through `AppState::load_price_data(...)`.
- [x] Record a baseline for CPU-only routed dynamic staging under CUDA builds.
- [x] Confirm no benchmark or profiling process is already running before perf work.

## Wave 1 checklist

- [x] Add a resident-upload batch helper in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [x] Refactor `ensure_resident_market_series(...)` to use one upload guard per rebuild or miss-fill batch.
- [x] Add local uninitialized `f64 -> f32` conversion helpers in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [x] Replace iterator-collection allocation in `simple_returns_from_log_returns_f32(...)`.
- [x] Refactor `crates/app_core/src/strategy_dynamic.rs` so prepared CUDA inputs are created lazily on first GPU-routed slot.
- [x] Add a regression test proving CPU-only dynamic staging does not create prepared CUDA inputs.
- [x] Add explicit attribution for prepared-device fallback in `crates/app_core/src/dynamic_indicator_adapter.rs`.
- [x] Convert dynamic staging dedupe caches to shared ownership or an equivalent one-copy design.
- [x] Derive `PartialEq, Eq` for `src-tauri/src/data_cache.rs::SourceIdentity`.
- [x] Add `load_or_build_with_identity(...)` in `src-tauri/src/data_cache.rs`.
- [x] Extend `src-tauri/src/state.rs::LoadedDataContext` with `SourceIdentity`.
- [x] Add identical-base-load reuse in `src-tauri/src/state.rs`.
- [x] Add tests for same-source base-load reuse and access-sequence updates.
- [x] Extend `src-tauri/src/bin/ta_app_path_faceoff.rs` artifact timing fields using `BackendOptimizationResult`.
- [x] Document that faceoff is not the full Tauri command path.

## Wave 2 checklist

- [x] Split runtime cache from dataset cache in `crates/app_core/src/dynamic_indicator_adapter.rs`.
- [~] Refactor prepared CUDA inputs to materialize OHLCV fields on demand.
  Repo-wide adoption is now broad across the validated section-3 composite wrappers, but some generic or intentionally broad call sites still remain.
- [x] Preserve current call-site behavior through lazy accessors.
- [x] Add memoized `first_valid` storage by `IndicatorInputKind`.
- [x] Add tests covering mixed input kinds after lazy field preparation.
- [x] Add a compile bundle helper around `compile_strategy_plan(...)` usage in `crates/app_core/src/strategy_dynamic.rs`.
- [x] Remove redundant compile passes in preview and runnable-plan creation.
- [x] Replace `dynamic_indicator_cache_key(...)` string and JSON serialization with a structured or hashed key.
- [ ] Defer any `crates/app_core/src/gpu_backtest_kernel.rs::CudaKernelRuntime` function-handle cache until a design proves it does not regress the GPU perf gate.
- [x] Cache per-kernel `KernelLaunchCaps` at runtime creation.
- [ ] Add tests or assertions for cached handles only if a revived non-regressing handle-cache design is introduced.

## Wave 3 checklist

- [x] Add configurable base/derived retention limits in `src-tauri/src/state.rs`.
- [ ] Add a tiny per-device prepared-input LRU only after warm alternation data is collected.
- [ ] Add a tiny resident-market LRU only after warm alternation data is collected.
- [ ] Revisit whether `reset_resident_market_cache(...)` still needs the same synchronization pattern after any resident-market eviction redesign.
- [x] Prototype lazy `cpu_context` construction for `RunnableDynamicDoubleMaPlan`.
- [x] Update `crates/app_core/src/double_ma.rs` consumers to work with lazy CPU context.
- [x] Reassess `upload_double_ma_dynamic_program_batch(...)` with fresh profiling before changing staging shape.

## Verification checklist

- [x] Run targeted tests for `strategy_dynamic.rs`, `dynamic_indicator_adapter.rs`, `state.rs`, and `data_cache.rs`.
- [x] Run `cargo test --workspace`.
- [x] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` at minimum.
- [~] Run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` for performance-sensitive CPU-path changes.
- [x] If CUDA-path behavior changed and GPU is available, run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- [x] Re-run `ta_app_path_faceoff` baselines and compare warm, cold, and reload behavior.
- [x] Confirm route/fallback summaries remain correct after attribution changes.

## Acceptance Criteria

1. Resident market rebuilds no longer synchronize once per uploaded field.
2. CPU-only dynamic staging under CUDA builds does not create prepared CUDA inputs.
3. Prepared CUDA runtime objects are not recreated on every dataset change for the same device.
4. Repeated loads of the same unchanged CSV reuse the existing in-memory base dataset.
5. Faceoff artifacts clearly distinguish harness wall time from engine-reported timing.
6. No CPU-only build is broken.
7. Existing fallback behavior remains transparent and parity-safe.
