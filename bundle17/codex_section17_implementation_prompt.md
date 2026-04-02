# Codex implementation prompt — Section 17 dynamic GPU runtime executable-cache and runtime reuse

## Mission

Implement only the remaining repo-valid Section 17 work for dynamic GPU executable-cache identity, handle-time reuse, and runtime-reuse evidence quality.

This is not a broad runtime rewrite.

Priority order:

1. fix executable identity and handle-time reuse first
2. remove the remaining duplicate plan-compilation work second
3. improve cache-attribution evidence third
4. defer broader cache-scope or LRU policy changes unless fresh evidence justifies them

---

## Read first

Before editing code, read:

1. `bundle17/section17_dynamic_gpu_runtime_reuse_plan.md`
2. `gpt54_feature_review_map.md` section `## 17. Dynamic GPU runtime executable-cache and runtime reuse`
3. `research/SECTION3_GPU_RUNTIME_VALIDATED_IMPLEMENTATION_PLAN_2026-03-21.md`
4. `research/section3_gpu_runtime_plan_addendum.md`

Trust the current code and the repo-state corrections below over older review wording.

---

## Repo-state validation and scope corrections

### Already true in this repo

- [x] `strategy_gpu_runtime.rs` already has a real executable cache and a thread-local uploaded-program cache.
- [x] Executable cache hits already repopulate uploaded-program residency for the current thread.
- [x] `strategy_gpu_program.rs` is already a generic GPU predicate-program compiler. Do not spend this bundle on first generalizing it.
- [x] `strategy_dynamic.rs` already reuses preview results within a request and already uses lazy CPU-context construction for GPU-resolved runnable plans.
- [x] `dynamic_indicator_adapter.rs` already handles runtime-versus-dataset reuse, selective OHLCV preparation, and prepared-device fallback counters.
- [x] `standalone_gpu_runtime.rs` already provides reusable host/device buffer helpers. It is not the main open performance surface for Section 17.
- [x] `ta_app_path_faceoff.rs` already labels itself as an app-path harness rather than the full Tauri command boundary.
- [x] `double_ma_opt.rs` already owns the right compile/upload/warm-lookup microbench surfaces.

### Still-open repo-valid work

- [ ] `StrategyHandle::from_plan(...)` still recompiles GPU program and ABI on every identical handle build before executable/upload reuse can help.
- [ ] `GpuStrategyExecutableCacheKey` is still derived from `plan.ir_fingerprint_hex`, which is weaker than full compiled executable identity.
- [ ] `resolve_dynamic_double_ma_strategy_ir_preview(...)` still does a redundant second CPU-plan compile on GPU-to-CPU downgrade paths.
- [ ] `prepare_runnable_dynamic_double_ma_plan(...)` still eagerly compiles a CPU fallback plan even when the resolved backend is GPU.
- [ ] Section 17 evidence binaries still do not surface executable-cache stats clearly enough to prove warm-path reuse.

### Explicit defer / exclude items

- [x] Do not re-implement executable upload caching.
- [x] Do not widen this bundle into `state.rs` or `data_cache.rs`.
- [x] Do not attempt a process-wide uploaded-program cache rewrite in the first pass.
- [x] Do not treat cross-call dynamic-indicator result reuse as a first-wave requirement.
- [x] Do not turn `ta_app_path_faceoff.rs` into the authority for compile/upload microbench claims.
- [x] Do not broaden runtime support beyond the current DoubleMa-v1 dynamic execution gate in this pass.

---

## File scope

### Primary implementation files

- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

### Inspect-only / conditional surfaces

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `crates/app_core/benches/double_ma_opt.rs`
- `scripts/run_gpu_perf_gate.ps1`

### Keep unchanged unless evidence forces it

- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- broad `gpu_backtest_kernel.rs` kernel-function rewrite surfaces

---

## Hard constraints

- Do not convert this into a large cache-architecture refactor.
- Preserve current route policy and capability decisions.
- Keep CPU-only builds working.
- If cache identity changes, keep invalidation rules explicit and obvious in code.
- Do not break the current GPU perf gate text parser unless it is updated in lockstep.
- Never run multiple benchmarks concurrently.

### Evidence-boundary guard

Treat the measurement surfaces as:

- `double_ma_opt.rs` for compile/upload/warm-cache micro-overhead
- `profile_gpu_dynamic_runner.rs` for steady-state runtime plus Section 17 cache-attribution JSON
- `ta_app_path_faceoff.rs` for app-path timing and route summaries, not executable-cache authority

---

## Implementation checklist

### Phase 1 — executable identity and handle-time reuse

- [ ] Add a compiled executable identity that reflects GPU ABI/program shape, not only raw IR fingerprint.
- [ ] Thread that identity through `CompiledGpuPredicateProgram`, `CompiledGpuPredicateProgramAbi`, `GpuStrategyExecutableCacheKey`, and uploaded-program metadata.
- [ ] Keep `STRATEGY_GPU_EXECUTABLE_ABI_VERSION` as an explicit invalidation input.
- [ ] Add a compiled GPU program/ABI bundle cache, or an equivalent lazy/shared bundle path, so repeated identical `StrategyHandle::from_plan(...)` calls do not rebuild the same program and ABI every time.
- [ ] Keep the public `StrategyHandle` behavior stable for current callers while adding compiled-bundle reuse.
- [ ] Add stats for compiled-bundle cache hits and misses.
- [ ] Add regression coverage for identical-plan compiled-bundle reuse.
- [ ] Add regression coverage for distinct compiled GPU shapes not aliasing the same executable identity.

### Phase 2 — duplicate plan-compilation cleanup

- [ ] Remove the redundant second CPU-plan compile in `resolve_dynamic_double_ma_strategy_ir_preview(...)`.
- [ ] Make CPU fallback plan compilation lazy in `prepare_runnable_dynamic_double_ma_plan(...)` when preview resolved to GPU.
- [ ] Preserve current preview output, fallback reasons, and runnable-plan behavior.
- [ ] Add regression coverage for GPU-resolved runnable plans not compiling CPU fallback state until needed.
- [ ] Add regression coverage for CPU-fallback preview behavior after the cleanup.

### Phase 3 — runtime-reuse evidence and attribution

- [ ] Extend `profile_gpu_dynamic_runner.rs` JSON output with per-phase executable-cache and prepared-device dispatch stats.
- [ ] Keep text-mode output backward compatible, or update `run_gpu_perf_gate.ps1` in lockstep if output changes.
- [ ] Add helper tests for runner JSON shape and parser-sensitive output.
- [ ] Keep `ta_app_path_faceoff.rs` boundary notes accurate if it is touched.
- [ ] Avoid adding duplicate timing field names unless the semantic difference is explicit.

### Explicit later-only items

- [x] Process-wide uploaded-program cache scope changes remain deferred.
- [x] Tiny LRU policies remain deferred until evidence justifies them.
- [x] Cross-call dynamic-indicator result reuse remains deferred.
- [x] Any bridge that borrows already-resident device fields instead of duplicate OHLCV uploads remains deferred until first-wave executable-cache work is measured.
- [x] `standalone_gpu_runtime.rs` broad rewrites remain deferred.

---

## Verification checklist

- [ ] Run targeted `strategy_gpu_runtime.rs` tests.
- [ ] Run targeted `strategy_dynamic.rs` tests.
- [ ] Run targeted `dynamic_indicator_adapter.rs` tests if stats or evidence-path plumbing touches that file.
- [ ] `cargo test -p ta_app_core --lib`
- [ ] `cargo test -p ta_app_core --bench double_ma_opt --no-run --features cuda,cuda-backtest-kernel`
- [ ] `cargo test -p ta_app_core --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel`
- [ ] `cargo test -p ta_desktop_demo_app --bin ta_app_path_faceoff --features cuda,cuda-backtest-kernel` if that binary changes
- [ ] `cargo test -p ta_desktop_demo_app --bin ta_profile_route_matrix --features cuda,cuda-backtest-kernel` if route behavior or route-attribution output changes
- [ ] Use `double_ma_opt.rs` for compile/upload/warm-lookup evidence.
- [ ] Run `profile_gpu_dynamic_runner --emit-json` in release for Section 17 cache-attribution evidence.
- [ ] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` after performance-sensitive changes land.
- [ ] Pair with `ta_profile_route_matrix --strict` if route behavior changes.

---

## Acceptance criteria

- [ ] Repeated identical GPU handle builds no longer recompile program/ABI on the hot path.
- [ ] Executable cache identity is compiled-shape aware.
- [ ] Uploaded-program reuse still works correctly.
- [ ] Preview and runnable-plan preparation no longer do avoidable CPU-plan recompiles.
- [ ] Section 17 runner artifacts clearly surface cache-hit behavior.
- [ ] CPU-only builds still work.
- [ ] No route-policy or capability regression is introduced.
