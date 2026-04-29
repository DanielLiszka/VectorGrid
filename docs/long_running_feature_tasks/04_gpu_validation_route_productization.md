# GPU Validation Route Productization

Status: proposed
Risk: high
Primary value: make validation GPU routing faster, more transparent, and harder to misinterpret

## Summary

Some validation paths still fall back to CPU even when a GPU backend is requested, and several route/residency/timing fields are planned but not fully productized across UI, artifacts, and reports. This task improves GPU validation capability and reporting in isolated slices.

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

- [ ] Inventory requested backend, actual backend, route, fallback reason, residency tier, and CPU-tail fields currently emitted.
- [ ] Add missing fields to validation summaries as optional fields.
- [ ] Add UI labels for fallback and CPU-tail work.
- [ ] Add XLSX/report fields for route, residency, fallback, and CPU-tail timing where data exists.
- [ ] Add tests proving missing fields in legacy artifacts do not break hydration.

### Phase 2: Validation Feature Matrix

- [ ] Build a strategy-by-module matrix for in-sample metrics, walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturbation, trade bootstrap, cross-ticker, aux analytics, and reports.
- [ ] Mark each cell as GPU main, GPU auxiliary CPU-tail, CPU-only fallback, or unsupported.
- [ ] Surface the matrix in docs and, where useful, UI capability summaries.
- [ ] Add tests that request-builder route copy matches backend support.

### Phase 3: One GPU Validation Capability Slice

- [ ] Pick a single strategy/module pair with clear value and bounded scope.
- [ ] Add backend parity tests against CPU output.
- [ ] Add route/fallback metadata tests.
- [ ] Measure before/after on a representative workload.
- [ ] Update capability matrix after implementation.

### Phase 4: Performance Evidence

- [ ] Add timing split fields for context discovery, module load, upload, kernel execution, materialization, and CPU-tail work where practical.
- [ ] Add candidate perf gate coverage for the touched route.
- [ ] Store proof output under the existing output/profiling convention.
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

- [ ] `cargo test -p ta_app_core double_ma_validate`
- [ ] `cargo test -p ta_app_core composite_validate`
- [ ] `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts src/components/ValidationWorkspacePanel.test.ts`
- [ ] `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- [ ] `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate` when GPU is available.
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
