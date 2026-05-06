# GPU Follow-Up Acceleration

Date: 2026-05-06
Status: researched implementation plan
Risk: high
Primary value: accelerate post-optimization research actions without overstating end-to-end GPU claims

## Summary

GPU Follow-Up Acceleration should target what users do after a grid optimization: fixed-candidate validation, candidate detail, local stability, revalidation windows, campaign follow-up runs, and repeated operations on the same dataset. This aligns with VectorGrid's strongest differentiator: warm app-path execution with resident market data and fast Double-MA GPU routes.

This should come after route metadata, validation evidence, and CPU-tail diagnostics are consistent. The first implementation should be a measurement and eligibility layer, then one narrow GPU follow-up route for eligible Double-MA finalist sets. CUDA Graph experiments are plausible later for stable repeated launch topologies, but only after correctness, residency, and workload shape are proven.

## External Research Basis

- CUDA C++ Best Practices Guide: https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html
  - NVIDIA emphasizes minimizing host/device transfers and keeping data on device across multiple kernels when possible.
- CUDA Graphs in the CUDA C++ Programming Guide: https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html#cuda-graphs
  - CUDA Graphs let a fixed set of operations and dependencies be defined separately from execution and launched repeatedly. They are useful only when the workflow topology is stable enough.
- CUDA Graphs special topic: https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/cuda-graphs.html
  - The graph update limitations section makes clear that topology changes require re-instantiation and that changing pointer/resource handles can be difficult. VectorGrid graph capture must have strict invalidation rules.
- CUDA stream-ordered memory allocator: https://docs.nvidia.com/cuda/cuda-runtime-api/group__CUDART__MEMORY__POOLS.html
  - Stream-ordered allocation and memory pools can reduce synchronization and allocation overhead, but require feature detection and correct stream lifetime discipline.
- Nsight Systems User Guide: https://docs.nvidia.com/nsight-systems/UserGuide/index.html
  - Nsight Systems can capture CUDA API calls, GPU memory operations, and kernel execution on a timeline. It is the right first profiler for route-level CPU/GPU interaction.
- Nsight Compute CLI: https://docs.nvidia.com/nsight-compute/NsightComputeCli/index.html
  - Nsight Compute CLI is appropriate after a specific kernel bottleneck is identified.
- CUDA pinned memory guidance: https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html#pinned-memory
  - Pinned memory can improve transfer bandwidth but is scarce and must be tested rather than overused.
- Compute Sanitizer: https://docs.nvidia.com/compute-sanitizer/ComputeSanitizer/index.html
  - Compute Sanitizer provides memcheck, racecheck, initcheck, and synccheck. Correctness-sensitive CUDA changes should run sanitizer checks before performance claims.

## Current Repo Anchors

- GPU kernel plumbing: `crates/app_core/src/gpu_backtest_kernel.rs`
- Double-MA execution: `crates/app_core/src/double_ma.rs`
- Double-MA validation: `crates/app_core/src/double_ma_validate.rs`
- Composite validation: `crates/app_core/src/composite_validate.rs`
- GPU route policy helpers: `crates/app_core/src/composite_profile_route.rs`
- Progress hooks and campaign VRAM prefetch: `crates/app_core/src/progress.rs`, `src-tauri/src/commands_campaign.rs`
- Validation route UI: `web/src/services/validationRequestBuilder.ts`, `web/src/services/validationService.ts`
- Follow-up benchmark: `crates/app_core/benches/double_ma_followup_workflow.rs`
- Existing GPU docs:
  - `docs/long_running_feature_tasks/04_gpu_validation_route_productization.md`
  - `docs/long_running_feature_tasks/14_gpu_validation_nonbenchmark_productization.md`
  - `docs/feature_expansion_implementation_plans_2026-04-30/10_gpu_cuda_performance_expansion.md`
- Release/benchmark authority: `docs/verification_benchmark_release_authority.md`

## Existing Behavior To Preserve

- CPU-only builds must work without CUDA.
- CPU fallback must remain supported.
- GPU route labels must remain truthful.
- Existing validation evidence and route metadata remain authoritative.
- Performance claims must include timing boundary, dataset identity, hardware identity, and route/cpu-tail metadata.
- Benchmarks and profiling must be serialized.

## Candidate Follow-Up Workloads

| Workload | Value | First eligibility |
|---|---|---|
| Fixed-candidate validation | high | Double-MA, uniform config, GPU route already supported |
| Candidate detail | high | selected Double-MA finalists with resident candles/derived series |
| Local stability | high | bounded neighborhood around selected Double-MA candidate |
| Revalidation windows | high | repeated fixed-candidate windows with stable module set |
| Campaign follow-up | medium-high | repeated datasets with explicit residency diagnostics |
| Options follow-up | later | only after options CPU/GPU parity and price-semantics contracts |
| CompositeMo follow-up | later | CPU-bound until a separate GPU route proposal exists |

## Product Shape

Expose this as a route-aware diagnostics and acceleration mode, not a blanket "GPU is faster" toggle.

UI elements:

- Follow-up route preview:
  - eligible
  - partially eligible
  - CPU preferred
  - unavailable
- Residency diagnostics:
  - dataset hash
  - resident buffer class
  - last upload time
  - derived series resident/missing
  - H2D/D2H copy counts where measured
- CPU-tail display:
  - module
  - reason
  - duration
  - whether expected/avoidable
- Repeated follow-up controls:
  - selected finalist set
  - window plan
  - stability neighborhood
  - run count
  - compare CPU route
- Proof output:
  - route metadata
  - timing boundary
  - hardware identity
  - dataset identity
  - kernel/materialization/copy counts

## Profiling Ladder

Use this order for any route behavior change:

1. CPU oracle and deterministic candidate identity tests.
2. CPU-only build check.
3. CUDA correctness check for changed kernels or memory lifetimes.
4. Nsight Systems timeline to find transfer, synchronization, launch, CPU-tail, or kernel bottlenecks.
5. Nsight Compute only if the bottleneck is inside a kernel.
6. Serialized benchmark gate under the repo timing boundary.
7. Proof packet saved under `output/`.

Do not run Nsight Compute first. Kernel metrics are not enough if the workflow is dominated by host/device transfer, materialization, synchronization, or CPU-tail modules.

## Proposed Contracts

### GpuFollowUpEligibilityV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GpuFollowUpEligibilityV1 {
    pub schema_version: u16,
    pub workflow: String,
    pub strategy_kind: String,
    pub eligible: bool,
    pub eligibility_class: String,
    pub target_route: Option<String>,
    pub fallback_route: String,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}
```

### GpuFollowUpResidencySummaryV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GpuFollowUpResidencySummaryV1 {
    pub schema_version: u16,
    pub dataset_hash: Option<String>,
    pub device_id: Option<u32>,
    pub residency_tier: Option<String>,
    pub candles_resident: bool,
    pub derived_series_resident: bool,
    pub candidate_buffer_resident: bool,
    pub h2d_copy_count: Option<u64>,
    pub d2h_copy_count: Option<u64>,
    pub kernel_launch_count: Option<u64>,
    pub cpu_tail_ms: Option<f64>,
    #[serde(default)]
    pub cpu_tail_reasons: Vec<String>,
}
```

### GpuFollowUpRunSummaryV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GpuFollowUpRunSummaryV1 {
    pub schema_version: u16,
    pub workflow: String,
    pub requested_backend: String,
    pub actual_route: String,
    pub timing_boundary: String,
    pub candidate_count: usize,
    pub window_count: Option<usize>,
    pub wall_ms: Option<f64>,
    pub residency: Option<GpuFollowUpResidencySummaryV1>,
    #[serde(default)]
    pub warnings: Vec<String>,
}
```

### GpuFollowUpProofPacketV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GpuFollowUpProofPacketV1 {
    pub schema_version: u16,
    pub workflow: String,
    pub timing_boundary: String,
    pub hardware_summary: Option<String>,
    pub driver_runtime_summary: Option<String>,
    pub dataset_hash: Option<String>,
    pub candidate_count: usize,
    pub route: String,
    pub cpu_oracle_status: String,
    pub sanitizer_status: Option<String>,
    pub nsight_systems_artifact: Option<String>,
    pub nsight_compute_artifact: Option<String>,
    pub benchmark_artifact: Option<String>,
    #[serde(default)]
    pub caveats: Vec<String>,
}
```

## Additive Boundary

- No route behavior changes before measurement.
- No new performance claims without proof output.
- No CUDA Graph adoption before stable repeated workflow shape is proven.
- No CPU fallback removal.
- No options GPU follow-up in first slice.
- No broad generic CUDA abstraction.
- No benchmarks while another GPU-intensive task is active.
- No CUDA memory-pool or pinned-memory changes without feature detection and before/after measurement.
- No graph capture across code that allocates, frees, or changes topology unpredictably.

## Implementation Phases

### Phase 1: Inventory And Eligibility

- [ ] Inventory current follow-up paths: validation, drilldown, local stability, campaign prefetch, and benches.
- [ ] Add `GpuFollowUpEligibilityV1` builder for Double-MA follow-up workloads.
- [ ] Add frontend route preview for selected candidate/detail/stability actions.
- [ ] Add tests for eligible, partially eligible, CPU-preferred, and unavailable states.

### Phase 2: Residency And CPU-Tail Diagnostics

- [ ] Add non-benchmark residency probe fields where safe.
- [ ] Count or report H2D/D2H copies and kernel launches for selected follow-up route.
- [ ] Attribute CPU tails by module and reason.
- [ ] Surface diagnostics in validation/detail UI and reports only where measured.
- [ ] Preserve optional fields for CPU-only builds.
- [ ] Mark diagnostics as measured, inferred, or unavailable.
- [ ] Record whether resident buffers are reusable across follow-up actions or only within one run.

### Phase 3: Double-MA Fixed-Finalist Follow-Up Route

- [ ] Select one bounded workflow, preferably fixed-candidate validation or local stability for Double-MA.
- [ ] Add CPU oracle tests.
- [ ] Add GPU route behind existing CUDA feature flags.
- [ ] Preserve exact candidate identity and ordering.
- [ ] Fail closed to CPU with explicit route note when eligibility fails.
- [ ] Add proof packet generation for local benchmark/profiling runs.
- [ ] Preserve artifact replay compatibility with optional route fields.

### Phase 4: Follow-Up Workflow Benchmark

- [ ] Extend or add Criterion follow-up benchmark around:
  - upload/residency setup
  - repeated validation/detail/stability
  - result materialization
- [ ] Record route, dataset hash, candidate count, and hardware.
- [ ] Do not compare against VectorBT or public README claims unless timing boundary matches.
- [ ] Store output under `output/` per repo convention.

### Phase 5: CUDA Graph Spike

Only after Phase 4 identifies launch overhead:

- [ ] Identify a stable repeated launch topology.
- [ ] Prototype CUDA Graph capture/instantiate/launch for that path.
- [ ] Add graph invalidation rules for changed candidate count, window count, buffer pointer, or module set.
- [ ] Compare graph versus non-graph route under the same timing boundary.
- [ ] Keep spike behind feature flag until parity and perf evidence are stable.

Graph invalidation triggers:

- candidate count changes;
- validation window count or boundaries change;
- module set changes;
- buffer addresses or allocation ownership changes;
- device id changes;
- CUDA runtime/driver incompatibility;
- kernel parameter layout changes;
- graph update returns non-success.

### Phase 6: Memory Pool And Transfer Spike

- [ ] Inventory current allocations and host/device transfers in the selected follow-up path.
- [ ] Test pinned memory only for bounded transfer buffers.
- [ ] Test stream-ordered memory allocation only when the CUDA device reports support.
- [ ] Add fallback to existing allocation behavior.
- [ ] Compare transfer counts and wall time before adopting.

## Frontend Scope

Recommended files:

- `web/src/utils/gpuFollowUpModel.ts`
- `web/src/services/validationService.ts`
- `web/src/services/analysisService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.vue`
- tests in `validationService.test.ts`, `analysisService.test.ts`, and new model tests

UI copy must use:

- "GPU follow-up eligible"
- "GPU main with CPU tail"
- "CPU preferred"
- "not measured"
- "resident evidence unavailable"

Avoid:

- "end-to-end GPU" unless every relevant stage is measured and GPU-resident.
- "faster" unless backed by a current benchmark.

## Verification Plan

CPU-only:

- `cargo check -p ta_app_core`
- `cargo test -p ta_app_core --lib`
- `cargo check -p ta_desktop_demo_app`

Frontend:

- `npm --prefix web run test -- validationService`
- `npm --prefix web run test -- analysisService`
- `npm --prefix web run test -- gpuFollowUpModel`
- `npm --prefix web run typecheck`

GPU when behavior changes and hardware is available:

- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- Nsight Systems before Nsight Compute for workflow-level bottlenecks.
- Compute Sanitizer for correctness-sensitive CUDA changes.
- Record all proof packet fields before writing release-facing docs.

Performance:

- Serialize benchmark/profiling runs.
- Verify no other benchmark/profiling process is active before starting.
- Record profiler overhead caveats.

## Regression Watchpoints

- CPU-only build must not depend on CUDA headers, libs, PTX, or runtime.
- GPU route must preserve finalist ordering and fixed-candidate identity.
- Route labels must expose CPU tails.
- Residency diagnostics must distinguish measured, inferred, and unavailable.
- Pinned memory must not be overused.
- CUDA Graphs must be invalidated when topology changes.
- Benchmark claims must not mix warm app-path, Criterion, profiler, and public README timing boundaries.
- CUDA Graph speedups must not be claimed if graph re-instantiation happens every run.
- Memory pools and pinned buffers must have clear lifetime and cleanup ownership.

## Definition Of Done

- Users can see whether a follow-up action is GPU eligible before running.
- Follow-up results include route, residency, and CPU-tail diagnostics where measured.
- One bounded Double-MA follow-up workflow is accelerated with CPU oracle parity.
- CPU fallback and CPU-only builds remain clean.
- Any performance claim has serialized proof output and a clear timing boundary.
- CUDA Graph work remains a separate measured spike, not a default route.
