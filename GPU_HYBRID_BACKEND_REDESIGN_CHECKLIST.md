# GPU Hybrid Backend Redesign Checklist

Last updated: 2026-02-28

## Purpose

Define a concrete implementation plan to replace the current "GPU only" behavior (GPU MA compute + CPU evaluation) with a performance-oriented hybrid model:

- compute GPU-capable indicators on GPU,
- compute CPU-only indicators on CPU in parallel,
- transfer CPU-produced indicator rows to GPU,
- run backtest/metrics/reduction on GPU kernel,
- return only reduced outputs to host.

This preserves full indicator coverage while keeping the expensive evaluation stage on GPU.

## Current State (as implemented)

- `Backend::CpuOnly`:
  - indicator + evaluation path is CPU/f64.
  - key code: `crates/app_core/src/double_ma.rs` (`ma_matrix_cpu_f64`, `eval_metrics_f64_with_exit_mode`).
- `Backend::GpuOnly`:
  - indicator MA sweep on GPU, copied to host (`Vec<f32>`), evaluation on CPU/f32.
  - key code: `crates/app_core/src/double_ma.rs` (`ma_matrix_cuda_host_f32`, `optimize_double_ma_gpu_sweep_tiled`).
- `Backend::GpuKernel`:
  - VRAM-resident path, indicator + backtest kernel on GPU.
  - key code: `crates/app_core/src/gpu_backtest_kernel.rs`.
- `Backend::GpuHybridKernel`:
  - hybrid contract/wiring is available end-to-end (frontend + request + Rust enum + CLI).
  - mixed-origin execution for `double_ma` is now wired:
    - non-kernel-capable MA side(s) are precomputed on CPU (`f64`), downcast to `f32`,
    - staged into the GPU kernel path as precomputed row-major matrices,
    - backtest/metrics/reduction still execute on the existing GPU kernel family.

UI wording has been updated to make this distinction explicit:
- `GPU indicators + CPU backtest`
- `GPU hybrid (phase 1: kernel path)`
- `GPU kernel (full GPU/VRAM path)`

## Target Architecture

Add a new backend mode (name TBD, suggested: `GpuHybridKernel`) with this pipeline:

1. Partition indicators by capability:
   - group A: GPU-kernel-capable indicators
   - group B: CUDA-capable but not kernel-capable (optional intermediate policy)
   - group C: CPU-only indicators
2. Launch GPU indicator computation for group A on CUDA stream(s).
3. Launch CPU computation for group C concurrently (thread pool).
4. Convert CPU indicator output to `f32` and upload to VRAM using pinned host buffers + async H2D copy.
5. Synchronize stream/event dependencies.
6. Execute GPU backtest kernel over fully prepared device indicator matrices.
7. Copy only top-k/heatmap/export outputs to host.

## Precision Policy

- `CpuOnly` remains f64 end-to-end.
- `GpuKernel` and `GpuHybridKernel` run indicator/eval kernels in f32.
- For hybrid:
  - CPU-only indicators should be computed in f64 on CPU (existing logic),
  - downcast once to f32 before H2D upload,
  - avoid f32->f64 round-trips in the hot path.

Rationale: transfering f64 doubles PCIe traffic and is usually worse than one CPU-side downcast.

## Backend Naming / UX Policy

- Keep wire/backend enum stability where possible to avoid breaking saved configs.
- UI labels must describe execution semantics, not hardware availability only.
- Recommended final user-facing labels:
  - `CPU only (indicators + backtest on CPU)`
  - `GPU indicators + CPU backtest` (compat mode)
  - `GPU hybrid (mixed indicators + GPU backtest)` (new)
  - `GPU kernel (full GPU/VRAM path)`

## Implementation Plan

### Phase 0 - Contract + Naming

- [x] Decide final backend enum variant name (`GpuHybridKernel`).
- [x] Add/update backend docs in:
  - [x] `README.md`
  - [x] `ARCHITECTURE_NOTES.md`
- [x] Ensure UI/backend labels stay explicit and non-ambiguous.

### Phase 1 - Request/Config Plumbing

- [x] Extend backend normalization/validation to include new backend variant:
  - [x] `web/src/services/optimizeRequestBuilder.ts`
  - [x] `crates/app_core/src/double_ma.rs` backend resolution
- [x] Add persistence compatibility for history/snapshots:
  - [x] `web/src/services/historyService.ts`
  - [x] `web/src/stores/optimizeStore.ts`
- [x] Add tests for request-building and store snapshots.
  - [x] request-building test for `GpuHybridKernel`
  - [x] store snapshot compatibility test

### Phase 2 - Capability Partition Planner

- [x] Implement planner that partitions requested indicators into GPU-capable and CPU-only sets.
- [x] Reuse existing MA capability sources:
  - [x] `supports_cuda_single`
  - [x] `supports_cuda_kernel`
  - [x] per-strategy indicator capability rules
- [x] Add deterministic planner tests (same input -> same partition decisions).

### Phase 3 - Concurrent Indicator Build

- [x] Introduce a hybrid-prep orchestrator in `ta_app_core`:
  - [x] spawn CPU indicator jobs (Rayon/threadpool) - phase-1 placeholder branch in hybrid runner
  - [x] launch GPU indicator jobs on CUDA stream - phase-1 placeholder branch in hybrid runner
- [x] Ensure cancellation checks and progress reporting are preserved.
- [x] Add explicit timing counters for:
  - [x] CPU indicator prep
  - [x] GPU indicator prep
  - [x] H2D upload
  - [x] kernel eval
  - Note: synthetic spin-loop placeholders were removed. Mixed-origin CPU prep time is now measured in hybrid flow; H2D remains tracked in-kernel and is still reported as `0` at the outer hybrid orchestrator level.

### Phase 4 - Transfer Path (CPU -> GPU)

- [ ] Add pinned host staging buffers for CPU-computed indicator rows.
- [ ] Use async H2D copies with stream/event synchronization.
- [x] Validate memory budget behavior under VRAM constraints (tiling + batching still works).
  - Added CUDA-gated hybrid parity test: `gpu_hybrid_batched_matches_unbatched_best_params_alma`.
- [ ] Avoid extra host copies (single downcast+pack path).
  - Progress: mixed-origin indicator rows now stage through kernel tile buffers for both fast/slow sides when precomputed matrices are provided.
  - Progress: kernel path now supports pinned+async H2D uploads behind `VECTORBT_KERNEL_PINNED_H2D` (fallback to pageable copy when pinned allocation is unavailable) for close/MA-source/volume/high/low/regime host series.
  - Progress: host->device upload setup is centralized (`upload_market_series_to_device`) and now performs one synchronization per upload bundle instead of one sync per series.
  - Progress: pinned path now downcasts directly into page-locked buffers (no intermediate `Vec<f32>` copy in pinned mode).

### Phase 5 - GPU Kernel Integration

- [x] Extend kernel input assembly to accept mixed-origin indicator matrices.
- [x] Guarantee identical matrix layout for kernel regardless of source.
- [x] Keep one backtest kernel family (no duplicate strategy-specific kernels).
- [x] Preserve existing top-k/heatmap/export semantics.
  - Implemented via optional `PrecomputedHostMaMatrix` inputs in `crates/app_core/src/gpu_backtest_kernel.rs` and hybrid precompute routing in `crates/app_core/src/double_ma.rs`.

### Phase 6 - Correctness/Determinism Gates

- [ ] Add backend parity tests (CPU vs Hybrid vs GpuKernel where supported):
  - [x] double MA baseline
  - [x] trend quality
  - [x] regime hybrid
  - [x] breakout momentum
- [x] Add deterministic seed/repeatability checks for hybrid mode.
- [x] Add fallback behavior tests when CUDA unavailable or capability mismatches occur.
  - Added CUDA-gated guard test: kernel-capable `alma/alma` hybrid requests stay on GPU path with no route fallback.
  - Added CUDA-gated mixed-origin tests: `gpu_hybrid_cpu_only_ma_uses_gpu_kernel_with_precomputed_staging` and `gpu_hybrid_vdps_cpu_only_ma_stays_on_gpu_route`.

### Phase 7 - Performance Gates

- [x] Add benchmark group for new backend in `crates/app_core/benches/double_ma_opt.rs`.
- [x] Required benchmark matrix:
  - [x] `small`
  - [x] `large`
  - [x] `xlarge`
  - [x] `xxlarge`
  - [x] batched VRAM-budget variants
- [ ] Profile with Nsight (`ncu`) for representative xlarge/xxlarge runs.
- [ ] Accept/reject criteria:
  - [ ] no major regression vs current `GpuKernel` on kernel-supported selections,
  - [ ] clear win vs `GpuOnly` for mixed-capability indicator selections.
  - Progress (2026-02-28, representative large case):
    - `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs` median ~`118.60 ms`.
    - `double_ma/gpu_hybrid_kernel/alma_alma/200k/large/58300pairs` median ~`114.41 ms`.

### Phase 8 - Rollout Controls

- [x] Add env/feature flag to enable hybrid backend progressively.
- [x] Keep `GpuOnly` as fallback compatibility mode initially.
- [x] Add telemetry/status line that states actual executed path and fallback reason.
- [ ] Promote hybrid as default only after benchmark + parity gates pass.

## File-Level Work Map

Expected primary touchpoints:

- Backend orchestration:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- Capability policy:
  - `crates/app_core/src/ma_policy.rs`
- Frontend/backend request wiring:
  - `web/src/services/optimizeRequestBuilder.ts`
  - `web/src/services/optimizeService.ts`
  - `web/src/stores/optimizeStore.ts`
  - `web/src/components/OptimizeWorkspacePanel.vue`
- Bench/tests:
  - `crates/app_core/benches/double_ma_opt.rs`
  - `web/src/services/optimizeRequestBuilder.test.ts`
  - `web/src/services/optimizeService.test.ts`
  - `crates/app_core/src/double_ma.rs` tests

## Risk Register

- Precision drift risk (f64 CPU indicator -> f32 GPU kernel):
  - Mitigation: explicit tolerance policy in parity tests.
- PCIe overhead risk:
  - Mitigation: pinned buffers, async overlap, tile sizing, avoid f64 transfer.
- Complexity/maintenance risk:
  - Mitigation: keep a single kernel path, add planner abstraction, avoid per-indicator backend branching in orchestration code.
- Fallback confusion risk:
  - Mitigation: explicit status text with actual executed backend and fallback reason.

## Exit Criteria

- [x] New hybrid backend is selectable and documented.
- [ ] All correctness tests pass with defined tolerances.
- [ ] Performance gates and Nsight profiles recorded.
- [ ] No regression in existing `CpuOnly` and `GpuKernel` paths.
- [ ] User-visible backend descriptions accurately reflect execution semantics.

## Notes

- This plan intentionally avoids requiring changes in `vector-ta` for phase-1 rollout.
- If later needed, deeper VectorTA integration can further reduce CPU-side staging overhead.
- 2026-02-27 benchmark note (targeted Criterion samples):
  - `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs` observed ~`118.60 ms` median.
  - `double_ma/gpu_hybrid_kernel/alma_alma/200k/large/58300pairs` observed ~`114.41 ms` median.
  - `double_ma/cpu/alma_alma/200k/large/58300pairs` observed ~`3.42-3.65 s`.
  - Hybrid route smoke tests confirm GPU/no-fallback path for kernel-capable and mixed-origin selections; Nsight profiling is still required before promoting hybrid defaults.
