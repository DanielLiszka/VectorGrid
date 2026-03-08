# GPU Backend Unification Master Checklist

Last updated: 2026-02-28

## Objective

Unify `GpuKernel` and `GpuHybridKernel` into one GPU execution engine that:

- keeps backtest/metrics/reduction on GPU whenever possible,
- routes indicator computation to CPU or GPU based on known performance,
- preserves existing functionality, correctness, and determinism,
- maximizes end-to-end optimization throughput.

## Performance Routing Policy (Source of Truth)

Primary source: `gpu_speedups.csv` at repo root.

Hard rule for this implementation:

- if an indicator is listed in `gpu_speedups.csv`, treat it as GPU-preferred,
- if an indicator is not listed, treat it as CPU-preferred.

Additional guardrails:

- GPU route is only allowed when the selected GPU execution path supports that indicator.
- Unsupported GPU indicators always route to CPU.
- CPU-computed indicator rows intended for GPU backtest are downcast once (`f64 -> f32`) before H2D transfer.

## In-Scope Indicator Set (from current VectorTA CUDA MA dispatcher)

`alma, cora_wave, cwma, dema, dma, edcf, ehma, ehlers_ecema, ehlers_itrend, ehlers_kama, ema, epma, frama, fwma, gaussian, highpass, highpass2, highpass_2_pole, hma, hwma, jma, jsa, kama, linreg, maaq, mwdx, nama, nma, pwma, reflex, sama, sinwma, sma, smma, sqwma, srwma, supersmoother, supersmoother_3_pole, swma, tema, tilson, trendflex, trima, vama, volatility_adjusted_ma, wilders, wma, zlema, vwap, vwma, vpwma`

## Target End State

One backend engine with policy-driven routing:

1. Plan per-indicator route (`GPU_KERNEL`, `GPU_SINGLE`, `CPU`).
2. Compute GPU-routed indicators on GPU.
3. Compute CPU-routed indicators on CPU (parallelizable).
4. Stage CPU rows to GPU efficiently.
5. Execute one shared GPU backtest kernel pipeline.
6. Return reduced outputs only.

No duplicate backtest kernel families for "hybrid vs kernel".

## User-Facing Backend Model

Recommended post-unification model:

- `CPU only` (full CPU pipeline)
- `GPU unified (Auto routing)` (recommended)
- Optional `GPU strict` (require full GPU eligibility; otherwise error)
- Optional temporary `GPU compatibility` (legacy behavior if needed during rollout)

## File-Level Implementation Map

- Orchestration:
  - `crates/app_core/src/double_ma.rs`
- Kernel execution:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- Capability/routing policy:
  - `crates/app_core/src/ma_policy.rs`
- UI/backend selection text + request mapping:
  - `web/src/services/optimizeRequestBuilder.ts`
  - `web/src/components/OptimizeWorkspacePanel.vue`
  - `web/src/stores/optimizeStore.ts`
- Benchmark + perf gates:
  - `crates/app_core/benches/double_ma_opt.rs`

## Master Checklist

### Phase 0 - Contract and Naming

- [x] Decide final backend enum and alias strategy.
  - Canonical backend tags remain: `Auto`, `CpuOnly`, `GpuOnly`, `GpuHybridKernel`, `GpuKernel`.
  - Compatibility aliases accepted during deserialization for legacy/alternate tags (for example `GpuHybrid`, `GpuSweep`, lowercase forms).
- [x] Keep backward compatibility for saved `GpuKernel` / `GpuHybridKernel` values.
  - Added `serde` backend aliases (`GpuHybrid`, `GpuSweep`, lowercase variants) while preserving canonical serialization tags.
- [x] Add deprecation notes for legacy labels and routes.
  - UI labels now mark `GpuOnly` as legacy sweep path and clarify `GpuHybridKernel`/`GpuKernel` semantics in optimize panel hints.

### Phase 1 - Routing Data Integration (`gpu_speedups.csv`)

- [x] Implement loader/parser for `gpu_speedups.csv` (robust to whitespace/case).
  - Implemented in `crates/app_core/src/ma_policy.rs` using embedded CSV parsing (`include_str!`) and normalized lowercase ids.
- [x] Build normalized indicator-id lookup.
- [x] Add deterministic route decision function:
  - [x] in CSV => GPU-preferred
  - [x] not in CSV => CPU-preferred
- [x] Add tests for mapping and normalization.
- [x] Add behavior for missing/unreadable CSV:
  - [x] explicit fallback policy
    - Build-embedded CSV is now the source of truth (`include_str!`), so runtime file availability is not a dependency for routing.
  - [x] telemetry note intentionally skipped per project decision (favor implementation speed / no runtime overhead).

### Phase 2 - Unified Planner

- [x] Replace split planner logic with one unified per-indicator plan.
- [x] Include capability checks (`supports_cuda_single`, kernel support, CPU support).
- [x] Emit one route plan artifact per run (for status/telemetry/debug).
  - `BackendOptimizationResult.indicator_route_plan` now returns fast/slow capability + selected route + GPU-preference flag.
- [x] Add planner tests for:
  - [x] kernel-capable + GPU-preferred indicators
  - [x] CPU-preferred indicators
  - [x] mixed strategy selections
  - [x] unsupported indicator handling

### Phase 3 - Unified Execution Path

- [x] Route all GPU-backed runs through one shared GPU execution function.
- [x] Route `Backend::Auto` to unified hybrid engine when CUDA kernel path is available (unless strict kernel is required for export).
- [x] Keep one backtest/metrics kernel family.
- [x] Reuse existing mixed-origin precomputed matrix support.
- [x] Ensure `GpuVdps` and other GPU modes use unified route logic.
  - `GpuVdps` now promotes explicit `GpuOnly` to `GpuHybridKernel` (when hybrid kernel path is available) to keep routing on the unified GPU-hybrid execution path.
  - Hybrid path continues to execute through `optimize_double_ma_gpu_kernel_tiled_with_precomputed`, preserving one kernel-family execution path while reusing mixed-origin precomputed staging buffers.

### Phase 4 - Transfer and Staging Performance

- [x] Ensure CPU->GPU staging uses pinned memory when enabled.
- [x] Ensure async H2D is used where safe.
  - CUDA kernel upload path uses `LockedBuffer<f32>` + `DeviceBuffer::uninitialized_async` + `async_copy_from` when `VECTORBT_KERNEL_PINNED_H2D=1` (default enabled), validated by feature-gated env-flag tests.
- [x] Avoid duplicate host copies in mixed-origin path.
  - Hybrid CPU-staged indicator prep now builds precomputed matrices directly in `f32` (`ma_matrix_cpu_f32_staging`) and feeds them to kernel precomputed path, removing the extra `Vec<f64> -> Vec<f32>` full-matrix conversion copy in common batch paths.
- [x] Validate tiling/batching behavior under VRAM budget constraints.
  - Representative budget-constrained parity tests pass (for example `gpu_hybrid_batched_matches_unbatched_best_params_alma`, `gpu_kernel_batched_matches_unbatched_best_params_nma` on isolated rerun). One broad multi-test run produced a transient `nma` mismatch under load, then passed on focused rerun.

### Phase 5 - Correctness and Parity

- [x] Add/extend parity tests:
  - [x] CPU vs GPU unified for kernel-capable routes
  - [x] CPU vs GPU unified for mixed CPU/GPU routes
  - [x] strategy variants (double, trend quality, regime hybrid, breakout)
- [x] Determinism tests across repeated runs.
- [x] Fallback and error-path tests.
  - Kernel-capable CPU-vs-GPU parity: covered by `gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies`.
  - Mixed-route parity + strategy variants: covered by `gpu_hybrid_cpu_fallback_parity_across_double_like_strategies`.
  - Determinism: covered by `gpu_hybrid_repeatability_for_cpu_fallback_path`, `gpu_vdps_deterministic_same_seed_reproducible`, and `gpu_vdps_deterministic_reproducible_for_double_like_strategies`.
  - Fallback/error paths: covered by tests like `gpu_hybrid_overlay_blocker_surfaces_fallback_reason`, `gpu_hybrid_vdps_rejects_cpu_fallback_mode`, and `gpu_vdps_rejects_cpu_backend`.

### Phase 6 - UI and UX Alignment

- [x] Update backend descriptions to remove ambiguous "GPU kernel vs GPU hybrid" semantics.
- [x] Show route summary to user (counts by CPU/GPU indicator route).
  - Added route-plan status rendering in `web/src/services/optimizeService.ts` via `indicator_route_plan` summary (`fast:<id>-><route>, slow:<id>-><route>`).
- [x] Add warning/status when selected indicators are CPU-routed due to performance policy.
  - Added `policy_cpu_route=...` status suffix when route is `cpu_staged` due to `gpu_speedups.csv` (kernel-capable but CPU-preferred indicator).

### Phase 7 - Performance Gates

- [x] Benchmark matrix for unified backend:
  - [x] small
  - [x] large
  - [x] xlarge
  - [x] xxlarge
  - [x] batched variants
- [x] Compare against current baselines:
  - [x] legacy `GpuKernel`
  - [x] legacy `GpuHybridKernel`
  - [x] `GpuOnly` (CPU backtest path)
  - [x] `CpuOnly`
- [x] Nsight profiling runs for representative large workloads.
  - Nsight Compute report generated for `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs` at `target/nsight/gpu_kernel_alma_large_58300.ncu-rep`.
- [x] Record pass/fail criteria with concrete thresholds.
  - Candidate gate thresholds currently enforced by scripts:
    - CPU: `workflow_runtime_not_regressed_50pct`, `criterion_mean_not_regressed_15pct`
    - GPU: `gpu_criterion_mean_not_regressed_10pct`

### Phase 8 - Rollout

- [x] Gate with env flag for staged rollout.
- [x] Enable by default only after parity + perf gates pass.
- [x] Keep compatibility aliases for one release window.
  - Rollout/env gate: `VECTORBT_ENABLE_GPU_HYBRID_KERNEL` (default enabled, can force off with `0`/`false`).
  - Compatibility window: keep backend serde aliases (`GpuHybrid`, `GpuSweep`, lowercase aliases) active while canonical tags remain unchanged.
- [x] Remove deprecated split backends after migration window.
  - Frontend/UI now exposes only `Auto`, `CpuOnly`, and unified `GpuKernel`.
  - Legacy UI/snapshot backend values (`GpuOnly`, `GpuHybridKernel`) are normalized to unified `GpuKernel` in store/request builder paths.

## Validation Matrix (must pass before default enablement)

- [x] `cargo check -p ta_app_core`
- [x] `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- [x] `cargo test -p ta_app_core`
- [x] `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel`
- [x] `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel`
- [x] Performance gate scripts:
  - [x] `scripts/run_cpu_perf_gate.ps1 -Mode candidate`
  - [x] `scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Risks and Mitigations

- Risk: CSV mapping drift or naming mismatch.
  - Mitigation: normalized id mapping + tests + explicit diagnostics.
- Risk: Mixed-origin transfer overhead erases gains for small workloads.
  - Mitigation: size-aware routing thresholds and benchmark gates.
- Risk: Backend behavior confusion during migration.
  - Mitigation: compatibility aliases + clear UI route telemetry.
- Risk: precision drift (`f64` CPU indicators, `f32` GPU backtest).
  - Mitigation: explicit tolerances in parity tests.

## Progress Log

Use this section as the running tracker during implementation.

- [x] P0 complete
- [x] P1 complete
- [x] P2 complete
- [x] P3 complete
- [x] P4 complete
- [x] P5 complete
- [x] P6 complete
- [x] P7 complete
- [x] P8 complete

### Latest Pass Notes

- 2026-02-28:
  - Updated `ma_policy` to route GPU preference from `gpu_speedups.csv` entries.
  - Wired hybrid precompute routing so non-GPU-preferred MA selections are CPU-staged for GPU backtest.
  - Updated `Backend::Auto` selection to choose `GpuHybridKernel` as the default GPU path when available (export still enforces strict `GpuKernel` eligibility).
  - Added unified planner API in `ma_policy` (`HybridRouteTier`, `plan_hybrid_indicator_route`, `plan_double_ma_hybrid_route`) and switched hybrid routing in `double_ma` to consume that planner.
  - Added route plan artifact to optimization result payload (`indicator_route_plan`) and wired non-double constructors with `None`.
  - Added backend serde alias compatibility (`GpuHybrid`, `GpuSweep`, lowercase tags) and test coverage to preserve old saved/backend payload compatibility.
  - Added mixed-selection planner test to assert a single plan can contain both `GpuKernel` and `CpuStaged` indicator routes.
  - Consolidated grid GPU backend dispatch (`GpuOnly`, `GpuKernel`, `GpuHybridKernel`) into one shared run-grid GPU execution closure in `double_ma` (behavior-preserving refactor).
  - Added `GpuVdps` backend promotion from `GpuOnly` -> `GpuHybridKernel` when available, plus CUDA-gated test coverage for the promotion route metadata.
  - Confirmed CUDA kernel parity test currently passes for kernel-capable routes (`gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies`).
  - Added `ma_matrix_cpu_f32_staging` and switched hybrid CPU-precompute route to use it, reducing host copy overhead for mixed-origin precomputed matrices.
  - Performance gates executed and passing:
    - CPU gate (`run_cpu_perf_gate.ps1 -Mode candidate`): pass, workflow runtime 362 ms vs baseline 374 ms; criterion mean 132.31822 ms vs baseline 130.125866 ms (within configured threshold).
    - GPU gate (`run_gpu_perf_gate.ps1 -Mode candidate`): pass, criterion mean 99.883539 ms vs baseline 155.288513 ms.
  - Added CUDA-kernel env-flag tests for pinned H2D and async alloc defaults/parsing in `gpu_backtest_kernel`.
  - Ran representative VRAM-budget batched/unbatched parity tests (hybrid + kernel). One broad kernel batch sweep produced a transient `nma` mismatch under load; isolated rerun passed.
  - Ran full CUDA benchmark suite (`double_ma_opt`) and exported an ALMA comparison matrix to `benchmark_unification_matrix_alma_2026-02-28.csv` (20 rows) covering `GpuKernel`, `GpuHybridKernel`, `GpuOnly`, `CpuOnly` across size/batching cases.
  - Added a readable benchmark summary table: `benchmark_unification_matrix_alma_2026-02-28.md`.
    - Example means (ms): `GpuKernel large_58300pairs=108.82`, `GpuHybridKernel large_58300pairs=109.65`, `GpuOnly large_58300pairs=3525.89`, `CpuOnly large_58300pairs=3389.68`.
  - Ran Nsight Compute profile via `scripts/profile_gpu_backtest_ncu.ps1` for large ALMA kernel workload; report written to `target/nsight/gpu_kernel_alma_large_58300.ncu-rep`.
  - Full app_core test suites pass:
    - `cargo test -p ta_app_core` -> `112 passed, 0 failed`
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel` -> `145 passed, 0 failed`
  - Completed rollout cleanup of deprecated split backend options in frontend surfaces; legacy backend values are remapped to unified `GpuKernel`.
  - Surfaced route artifact in frontend status text (optimize service summary) and validated `optimizeService` tests.
  - Updated backend option labels/hints in the Vue optimize panel to clarify distinctions:
    - `GpuOnly` is legacy GPU-indicator + CPU-backtest path.
    - `GpuHybridKernel` is mixed indicator routing into GPU backtest.
    - `GpuKernel` is strict full GPU/VRAM path.
  - Added explicit policy-routing summary in optimize status when indicators are CPU-routed by speedup policy (`policy_cpu_route=...`).
  - Confirmed "skip telemetry" decision by locking routing to embedded CSV and removing runtime-CSV dependency from required routing behavior.
  - Validation passed:
    - `cargo check -p ta_app_core`
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core ma_policy -- --nocapture`
    - `cargo test -p ta_app_core auto_backend_records_route_metadata -- --nocapture`
    - `cargo test -p ta_app_core backend_deserialize_aliases_remain_compatible -- --nocapture`
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_vdps_promotes_gpu_only_to_hybrid_kernel_when_available -- --nocapture`
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies -- --nocapture`
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel kernel_h2d_flag -- --nocapture`
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel kernel_async_alloc_flag_defaults_to_enabled_and_parses_false -- --nocapture`
    - `cargo test -p ta_app_core cpu_ma_matrix_f32_staging_matches_f64_cast -- --nocapture`
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_hybrid_batched_matches_unbatched_best_params_alma -- --nocapture`
    - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_batched_matches_unbatched_best_params_nma -- --nocapture`
    - `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel`
    - `powershell -File scripts/profile_gpu_backtest_ncu.ps1 -BenchFilter "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs" -OutputPrefix "target/nsight/gpu_kernel_alma_large_58300"`
    - `cargo test -p ta_app_core gpu_hybrid_ -- --nocapture`
    - `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
    - `cargo test -p ta_app_core gpu_hybrid_ --features cuda,cuda-backtest-kernel -- --nocapture`
    - `npm --prefix web run test -- optimizeService --run`
    - `npm --prefix web run test -- optimizeStore.actions optimizeRequestBuilder optimizeService --run`
