# GPU Validation Route Productization

Status: implemented for transparency/productization; no new GPU route behavior added in this slice
Risk: high
Primary value: make validation GPU routing faster, more transparent, and harder to misinterpret

## Summary

Validation already had mature GPU route/fallback behavior across Double-MA and composite fixed-candidate validation. This slice productizes the remaining low-regression transparency gaps without changing GPU execution behavior: result JSON now records requested backend and residency tier, route-detail UI includes requested/residency/CPU-tail metadata when present, and XLSX dashboard metadata includes requested backend and residency.

No GPU kernel, route policy, or performance-sensitive execution code was changed in this slice.

## Current Anchors

- Double-MA validation: `crates/app_core/src/double_ma_validate.rs`
- Composite validation: `crates/app_core/src/composite_validate.rs`
- GPU kernel plumbing: `crates/app_core/src/gpu_backtest_kernel.rs`
- Backend route policy: `crates/app_core/src/double_ma_backend.rs`
- Frontend validation request/reporting: `web/src/services/validationRequestBuilder.ts`, `web/src/services/validationService.ts`, `web/src/components/ValidationWorkspacePanel.vue`
- Performance plan: `docs/backtest_optimization_gpu_performance_plan.md`

## Additive Boundary

- CPU fallback must remain supported.
- GPU work must be feature-gated and build without CUDA on CPU-only machines.
- Reporting improvements should not change validation pass/fail logic.
- Route changes must include CPU/GPU parity tests before performance claims.

## Non-Goals

- No low-level CUDA kernel rewrite without profiling evidence.
- No removal of existing CPU validation paths.
- No broad strategy route policy rewrite in the first slice.

## Implementation Checklist

### Phase 1: Route Transparency Without Behavior Change

- [x] Inventory requested backend, actual backend, route, fallback reason, residency tier, and CPU-tail fields currently emitted.
- [x] Add missing requested-backend and residency-tier fields to validation results.
- [x] Add route-detail UI metadata for requested backend, residency, and measured Double-MA CPU-tail time when present.
- [x] Add XLSX/report metadata fields for requested backend, residency, route, fallback, and existing route details.
- [x] Preserve legacy hydration by treating absent fields as optional/fallback-derived in frontend/report code.

### Phase 2: Validation Feature Matrix

- [x] Build a strategy-by-module matrix for in-sample metrics, walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturbation, trade bootstrap, cross-ticker, aux analytics, and reports through the existing repo map, request-builder capability summaries, and route-note tests.
- [x] Mark route classes as GPU main, intentional auxiliary CPU-tail, CPU fallback, or unsupported through route strings, fallback reasons, route detail summaries, and report summaries.
- [x] Surface the matrix in docs and UI capability summaries.
- [x] Add tests that request-builder/route copy matches backend support through existing validation service and route-copy coverage.

### Phase 3: One GPU Validation Capability Slice

- [x] No new GPU capability was selected because the audit found the originally planned route behavior already implemented.
- [x] Existing CPU/GPU parity tests remain the authority for current GPU validation route behavior.
- [x] Add route/fallback/requested/residency metadata checks in smoke coverage.
- [ ] Measure before/after on a representative workload only when a future slice changes GPU validation execution behavior.
- [x] Update capability matrix/status after implementation.

### Phase 4: Performance Evidence

- [x] Preserve existing Double-MA validation execution profile fields including GPU materialization and CPU-tail time.
- [ ] Add candidate perf gate coverage for a touched route when a future route behavior change lands.
- [ ] Store proof output under the existing output/profiling convention when performance-sensitive route behavior changes.
- [ ] Use Nsight Systems for workflow timeline bottlenecks before Nsight Compute.
- [ ] Run Compute Sanitizer for correctness-sensitive CUDA changes.
- [ ] Use Nsight Systems to identify transfer, launch, synchronization, and CPU-tail bottlenecks before kernel-level tuning.
- [ ] Use Nsight Compute only after the bottleneck is known to be inside a CUDA kernel.
- [ ] Record profiler overhead caveats in proof notes when profiler settings materially change runtime.
- [ ] Run Compute Sanitizer `memcheck`, and add `racecheck` or `initcheck` when shared memory or uninitialized device-memory risk is present.

### Phase 5: CI And Artifact Evidence

- [ ] When GPU CI is available, upload route summaries, benchmark rows, and profiler notes as workflow artifacts.
- [ ] Keep GPU jobs optional or hardware-labeled so CPU-only CI remains green.
- [ ] Add a matrix dimension for backend mode only after the targeted local gates are stable.

## Validation Checklist

- [x] `cargo test -p ta_app_core --lib double_ma_validation_smoke_runs_and_returns_stats -- --test-threads=1`
- [x] `cargo test -p ta_app_core --lib composite_validation_smoke_runs_and_returns_stats -- --test-threads=1`
- [x] `npm --prefix web run test -- validationService ValidationWorkspacePanel`
- [ ] `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` only if a future CPU-sensitive validation execution change lands.
- [ ] `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate` when GPU behavior changes or GPU evidence is required.
- [ ] `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` for broad CUDA route changes.

## Regression Watchpoints

- CPU-only build must pass without CUDA installed.
- GPU path must not silently drop validation modules.
- Fallback reasons must be explicit when CPU is used.
- Performance claims require serialized benchmark runs.
- Profiler output must not be treated as a benchmark result unless profiler overhead is explicitly accounted for.

## Definition Of Done

- Users can see requested route, actual route, fallback reason, and CPU-tail work.
- Any new GPU validation capability has CPU/GPU parity tests.
- Reports and artifacts preserve route provenance.
