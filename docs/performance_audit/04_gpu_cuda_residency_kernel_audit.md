# GPU CUDA Residency And Kernel Audit

Date: 2026-05-06
Status: audit plan

## Goal

Maximize GPU-path performance by measuring complete route behavior, not only kernel time. The audit should expose residency, transfers, launches, synchronization, materialization, CPU tails, and kernel bottlenecks.

## Scope

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/*.cu`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_campaign.rs`
- GPU perf scripts and profiling scripts

## Existing GPU Authorities

- `scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`
- `scripts/run_gpu_followup_perf_gate.ps1 -Mode candidate`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
- `scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate`

## Profiling Order

1. Correctness/parity tests.
2. Compute Sanitizer for memory/race/uninitialized risk.
3. Nsight Systems for workflow timeline.
4. Nsight Compute for kernel metrics after timeline identifies kernel bottleneck.
5. Serialized perf gate candidate run.

Nsight output is diagnostic, not benchmark authority.

## Residency Audit Matrix

| Surface | Residency questions |
|---|---|
| Double-MA optimization | Are price/indicator/candidate buffers resident across measured trials? |
| Validation | Are fixed candidates rerun on GPU, and which modules materialize to CPU? |
| Candidate follow-up | Are candles/derived series reused for drilldown/local stability? |
| Campaign | Does VRAM prefetch stage once and reuse across campaign rows? |
| Options expression | Which scoring paths are GPU eligible, and where are CPU tails required? |
| Reports/artifacts | Are large GPU results materialized only once? |

## Required GPU Route Diagnostics

Every GPU performance proof should record:

- requested backend;
- actual backend;
- route;
- fallback reason;
- residency tier;
- device id/name;
- driver/runtime versions where available;
- dataset hash;
- candidate count;
- window count when validating;
- H2D copy count;
- D2H copy count;
- kernel launch count;
- CPU-tail modules;
- CPU-tail milliseconds;
- materialization reason.

## CUDA Optimization Hypotheses

Do not implement these until measured.

| Hypothesis | Evidence needed |
|---|---|
| Hidden D2H materialization dominates follow-up | Nsight Systems copy timeline plus route diagnostics |
| Kernel launch overhead dominates repeated small follow-ups | Nsight Systems launch timeline; candidate for CUDA Graph spike |
| Host allocation/staging dominates | CPU timing plus transfer timeline |
| Kernel occupancy/memory access dominates | Nsight Compute after workflow timeline |
| Pinned memory helps transfer path | before/after transfer benchmark, bounded buffers |
| Stream-ordered allocator helps | allocation timeline and feature support |
| CPU-tail validation modules dominate | validation phase timing and route category summary |

## CUDA Graph Spike Criteria

Only consider CUDA Graphs when:

- candidate count is fixed;
- validation window topology is fixed;
- kernel sequence is stable;
- buffer lifetimes are stable;
- graph update does not re-instantiate every run;
- CPU oracle parity exists;
- route diagnostics can report graph active/inactive.

Graph invalidation:

- changed candidate count;
- changed module set;
- changed window plan;
- changed buffer ownership or pointer layout;
- device/runtime mismatch;
- graph update failure;
- feature flag disabled.

## Measurement Checklist

- [ ] Run plan-only or inventory modes where available.
- [ ] Record current route metadata for Double-MA optimization.
- [ ] Record current route metadata for validation-heavy profile.
- [ ] Run GPU follow-up gate on candidate detail/local stability workloads.
- [ ] Run Nsight Systems on one representative Double-MA workflow.
- [ ] Run Nsight Systems on one validation-heavy workflow.
- [ ] Run Nsight Compute only for identified kernel bottleneck.
- [ ] Store profiles under `output/performance_audit/<date>/profiles/`.
- [ ] Summarize copy/launch/materialization counts.

## Verification

- CPU fallback:
  - `cargo check -p ta_app_core`
- GPU behavior:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`
  - `powershell -File scripts/run_gpu_followup_perf_gate.ps1 -Mode candidate`
- Options:
  - `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate`

## Regression Watchpoints

- CPU-only build must not require CUDA.
- Route metadata must expose fallback and CPU tails.
- Benchmark/profile sessions must be serialized.
- Profiler overhead must not become benchmark evidence.
- GPU claims must not hide host materialization or CPU modules.
- CUDA Graphs must not become default without invalidation evidence.

## Definition Of Done

- GPU routes have residency and CPU-tail attribution.
- Top GPU bottlenecks are classified as transfer, launch, synchronization, materialization, CPU tail, or kernel.
- Any CUDA optimization has before/after evidence and CPU/reference parity.
