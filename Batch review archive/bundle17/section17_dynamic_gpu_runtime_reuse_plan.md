# Section 17 Implementation Plan

## Dynamic GPU runtime executable-cache and runtime reuse

This plan is grounded in the current repo state for Section 17 of `gpt54_feature_review_map.md`.

It narrows the work to the code that still matters today:

- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`

It uses the older runtime research docs only as background:

- `research/SECTION3_GPU_RUNTIME_VALIDATED_IMPLEMENTATION_PLAN_2026-03-21.md`
- `research/section3_gpu_runtime_plan_addendum.md`
- `research/DYNAMIC_GPU_STRATEGY_EXECUTION_IMPLEMENTATION_PLAN_2026-03-18.md`

Those documents already describe work that has since landed. This Section 17 plan keeps only the still-valid follow-up work for executable identity, handle-time reuse, and evidence quality.

## Current repo-state validation

### Already implemented and not Section 17 open work

- [x] A real executable cache already exists in `strategy_gpu_runtime.rs`, with process-wide executable entries and a thread-local uploaded-program cache.
- [x] Cache hits already repopulate the per-thread uploaded-program cache before use.
- [x] `strategy_gpu_program.rs` already compiles generic GPU predicate programs from compiled plans; this bundle is not about first generalizing the compiler.
- [x] `strategy_dynamic.rs` already reuses resolved preview state within a request.
- [x] `strategy_dynamic.rs` already uses lazy CPU context construction for GPU-resolved runnable plans.
- [x] `dynamic_indicator_adapter.rs` already separates runtime lifetime from dataset lifetime, selectively prepares required OHLCV fields, and memoizes `first_valid`.
- [x] `dynamic_indicator_adapter.rs` already records prepared-device hits and fallbacks.
- [x] `standalone_gpu_runtime.rs` already provides reusable host/device buffer helpers. It is not the main hot-path gap for this section.
- [x] `ta_app_path_faceoff.rs` already states that it is an app-path harness, not the full Tauri command boundary.
- [x] `double_ma_opt.rs` already contains the right microbench surfaces for compile/upload/warm-lookup work. Section 17 should use those benches instead of overloading app-path binaries for micro-overhead claims.

### Still-open repo-valid problems

- [ ] `StrategyHandle::from_plan(...)` still recompiles the GPU predicate program and ABI on every identical handle build before the executable/upload caches can help.
- [ ] `GpuStrategyExecutableCacheKey` is still keyed from `plan.ir_fingerprint_hex`, which is IR identity, not full compiled executable identity.
- [ ] `resolve_dynamic_double_ma_strategy_ir_preview(...)` still has a redundant CPU-plan compile when GPU capability is pre-downgraded to CPU.
- [ ] `prepare_runnable_dynamic_double_ma_plan(...)` still eagerly compiles a CPU fallback plan even when the resolved backend is GPU and the CPU path may never run.
- [ ] Executable-cache and prepared-device fallback stats exist in library code, but the Section 17 evidence binaries do not surface them clearly enough to prove warm-path reuse.
- [ ] `profile_gpu_dynamic_runner.rs` is still better at steady-state route timing than at explicit cold-versus-warm executable-cache attribution.

### Explicit non-goals for the first implementation wave

- [x] Do not re-implement executable upload caching that already exists.
- [x] Do not widen this bundle back into `state.rs` or `data_cache.rs`; that work belongs to earlier data/cache ownership bundles.
- [x] Do not turn this into a process-wide cache-scope rewrite unless fresh evidence shows thread-hops are the real blocker.
- [x] Do not broaden the runtime beyond the current DoubleMa-v1 dynamic execution gate in this bundle.
- [x] Do not treat cross-call dynamic-indicator result reuse or device-resident slot-result caching as a first-wave requirement.
- [x] Do not use `ta_app_path_faceoff.rs` as the authority for compile/upload microbench claims.

## Implementation strategy

### Phase 1: executable identity and handle-time reuse

Goal:

- make warm-path reuse start earlier than uploaded-program lookup
- prevent cache aliasing based on IR-only identity

Checklist:

- [ ] Add a compiled executable identity that is derived from compiled GPU ABI/program shape rather than only `ir_fingerprint_hex`.
- [ ] Thread that executable identity through `CompiledGpuPredicateProgram`, `CompiledGpuPredicateProgramAbi`, `GpuStrategyExecutableCacheKey`, and uploaded-program metadata.
- [ ] Keep `STRATEGY_GPU_EXECUTABLE_ABI_VERSION` as an explicit invalidation input; do not bump versions casually.
- [ ] Add a compiled GPU program/ABI bundle cache, or an equivalent lazy shared bundle path, so repeated identical `StrategyHandle::from_plan(...)` calls do not rebuild the same program and ABI every time.
- [ ] Keep the public `StrategyHandle` behavior stable for current callers.
- [ ] Add stats for compiled-bundle cache hits and misses, separate from uploaded-executable hits and misses.
- [ ] Add tests proving identical GPU plans reuse the compiled bundle and uploaded executable.
- [ ] Add tests proving materially different compiled GPU shapes do not alias the same executable identity.

Notes:

- If route overrides can influence GPU slot shape, executable identity must account for that.
- This phase is the core Section 17 work. If it does not land, warm-path wins will still be partially hidden behind repeated handle compilation.

### Phase 2: preview/runnable-plan reuse cleanup

Goal:

- remove duplicate plan compilation that still survives after the earlier Section 3 work

Checklist:

- [ ] Remove the redundant second CPU compile in `resolve_dynamic_double_ma_strategy_ir_preview(...)`.
- [ ] Stop eagerly compiling a CPU fallback plan in `prepare_runnable_dynamic_double_ma_plan(...)` when preview resolved to GPU.
- [ ] Make CPU fallback plan compilation lazy alongside the existing lazy CPU-context path.
- [ ] Preserve current fallback reasons, preview output, and runnable-plan behavior.
- [ ] Add regression coverage for GPU-resolved runnable plans not compiling CPU fallback state until it is actually requested.
- [ ] Add regression coverage for pre-downgraded CPU preview paths remaining correct after the compile cleanup.

Notes:

- This phase should stay local to `strategy_dynamic.rs`, `strategy_plan.rs`, and the shared plan/handle types.
- Avoid changing route policy, validation messages, or frontend behavior.

### Phase 3: evidence-surface and attribution cleanup

Goal:

- make Section 17 measurable without conflating compile/upload, warm reuse, and app-path timing

Checklist:

- [ ] Extend `profile_gpu_dynamic_runner.rs` JSON mode to emit executable-cache deltas per phase: compiled-bundle hits/misses, uploaded-executable hits/misses, entry counts, and prepared-device hit/fallback counts.
- [ ] If cold-versus-warm executable-cache modes are added, serialize them explicitly so artifacts cannot be misread.
- [ ] Keep the current text-mode output backward compatible, or update `run_gpu_perf_gate.ps1` in lockstep if text output changes.
- [ ] Add helper tests for runner JSON shape, boundary labels, and any parser-sensitive output.
- [ ] Keep `ta_app_path_faceoff.rs` boundary notes accurate; only extend it if a deliberate Section 17 dynamic-mode artifact is needed.
- [ ] If `ta_app_path_faceoff.rs` is touched, avoid adding fields that duplicate the same outer wall time under new names without clarifying semantics.

Notes:

- `profile_gpu_dynamic_runner.rs` is the right place for dynamic runtime warm/cold attribution.
- `ta_app_path_faceoff.rs` remains useful for app-path timing and route summaries, but not for executable-cache micro-overhead authority.

### Optional later phase: broader reuse policies only after evidence

Checklist:

- [ ] Consider bounded cache retention or a tiny LRU only after measured evidence shows real multi-plan churn pressure.
- [ ] Consider thread-stable or process-wide uploaded-program reuse only after evidence shows thread-hops are defeating warm-path payoff.
- [ ] Consider cross-call dynamic-indicator result reuse only after executable identity and handle-time reuse are fixed and measured.
- [ ] Consider a later bridge from `PreparedCudaIndicatorInputs` to already-resident device fields only if post-fix profiling still shows duplicated OHLCV upload churn.

These are intentionally not first-wave requirements.

## File ownership

### Primary implementation files

- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

### Inspect-only or lightly-touched supporting files

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `crates/app_core/benches/double_ma_opt.rs`
- `scripts/run_gpu_perf_gate.ps1`

### Out-of-scope for this section unless evidence forces escalation

- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- broad `gpu_backtest_kernel.rs` kernel-lookup redesign
- frontend request-builder or UI work

## Verification checklist

### Behavior and regression coverage

- [ ] Run targeted `ta_app_core` tests for `strategy_gpu_runtime.rs`.
- [ ] Run targeted `ta_app_core` tests for `strategy_dynamic.rs`.
- [ ] Run targeted `ta_app_core` tests for `dynamic_indicator_adapter.rs` if evidence/output counters or cache-attribution paths are touched.
- [ ] Add or update tests for executable identity separation and compiled-bundle reuse.
- [ ] Add or update tests for lazy CPU fallback plan compilation.
- [ ] Add or update tests for `profile_gpu_dynamic_runner` JSON artifact shape if new stats are emitted.

### Build and bench surfaces

- [ ] `cargo test -p ta_app_core --lib`
- [ ] `cargo test -p ta_app_core --bench double_ma_opt --no-run --features cuda,cuda-backtest-kernel`
- [ ] `cargo test -p ta_app_core --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel`
- [ ] `cargo test -p ta_desktop_demo_app --bin ta_app_path_faceoff --features cuda,cuda-backtest-kernel` if that binary changes

### Performance and route evidence

- [ ] Use the Section 17 Criterion labels in `double_ma_opt.rs` for compile/upload/warm-lookup claims.
- [ ] Run `profile_gpu_dynamic_runner` in release with `--emit-json` for cache-attribution evidence after implementation.
- [ ] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` after performance-sensitive changes land.
- [ ] If backend selection or route guarantees change, pair the timing run with `ta_profile_route_matrix --strict`.

### Measurement rules

- [ ] Do not run multiple benchmarks at the same time.
- [ ] Do not treat `ta_app_path_faceoff` as the authoritative compile/upload microbench.
- [ ] Do not treat the GPU perf gate's single dynamic ratio check as the only Section 17 evidence.

## Acceptance criteria

- [ ] Repeated identical `StrategyHandle::from_plan(...)` builds no longer recompile the GPU program/ABI on the hot path.
- [ ] Executable cache identity is based on compiled executable shape, not only raw IR fingerprint.
- [ ] Uploaded-program reuse still works and remains parity-safe.
- [ ] Preview and runnable-plan preparation no longer do avoidable CPU-plan recompiles.
- [ ] Section 17 runner artifacts clearly distinguish cache-hit behavior from generic steady-state runtime.
- [ ] No CPU-only build is broken.
- [ ] No route-policy or capability regression is introduced.

## Recommended implementation order

1. Land executable-identity and compiled-bundle reuse in `strategy_gpu_runtime.rs`.
2. Clean up duplicate CPU-plan compilation in `strategy_dynamic.rs`.
3. Surface cache stats in `profile_gpu_dynamic_runner.rs`.
4. Update tests and verification surfaces.
5. Re-measure with the existing Criterion and GPU gate tooling.
