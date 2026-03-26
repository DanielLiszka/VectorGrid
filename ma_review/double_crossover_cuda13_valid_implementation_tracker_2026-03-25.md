# Double Crossover CUDA 13.x Valid Implementation Tracker

Last updated: 2026-03-25

## Purpose

This tracker covers the subset of ideas from
`ma_review/double_crossover_cuda13_review.md`
that are worth implementing or at least A/B testing against the current `double_ma`
GPU product path.

This is **not** a generic CUDA 13 feature-adoption checklist.
It is a repo-grounded plan for the current `double_cross_backtest_tm_flex_*`
kernel family and its surrounding runtime.

## Current Repo / Machine State

- The current product path is the widened single-pass Double MA-family GPU route.
- The active hot kernel family is the `double_cross_backtest_tm_flex_*` wrapper set in
  `crates/app_core/src/kernels/double_crossover.cu`, loaded by
  `crates/app_core/src/gpu_backtest_kernel.rs`.
- The current shell still resolves `nvcc` to CUDA 13.0:
  - `nvcc --version` -> `release 13.0, V13.0.88`
  - `CUDA_PATH` -> `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.0`
- CUDA 13.2 is also installed locally:
  - `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2`
  - direct `v13.2\bin\nvcc.exe --version` reports `release 13.2, V13.2.51`
- Local CUDA 13.2 headers do expose `cub::DeviceTopK`:
  - `v13.2\include\cccl\cub\device\device_topk.cuh`

## Accepted Scope

These are the ideas worth pursuing in this tracker:

1. Shared-memory register spilling A/B tests on the current hot `double_cross_backtest_tm_flex_*` wrappers.
2. Removal of the extra `ret1` preprocessing / storage path by recomputing per-step arithmetic return from `lr_t` inside the hot kernel.
3. A CUDA 13.2 `cub::DeviceTopK` experiment on the **row-shortlist** stage only.
4. Lower-priority profiling-gated experiments:
   - adaptive slow-row dedup
   - warp-aggregated heatmap atomics
   - tiled transpose for `transpose_row_to_tm`
5. CUDA 13.1+ tooling adoption for optimization work:
   - better sanitizer / memcheck usage
   - better Nsight profiling usage

## Explicit Non-Goals

These are out of scope unless a later document reopens them:

- Rewriting the kernel around CUDA Tile.
- Replacing the fully fused pair-scoring selectors with a generic `DeviceTopK` path.
- Adopting CUDA 13 host-side runtime wrappers (`cuda::stream`, `cuda::event`, etc.) as a performance project for this kernel file.
- Green contexts / static SM partitioning as a source-level patch for `double_crossover.cu`.
- Large structural kernel rewrites before finishing the smaller measured experiments below.

## Canonical Code Surfaces

Open these first before broader search:

- `ma_review/double_crossover_cuda13_review.md`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/benches/double_ma_opt.rs`
- `ma_review/double_ma_single_pass_valid_implementation_tracker_2026-03-24.md`

Representative hotspot anchors:

- current hot wrapper family:
  - `double_cross_backtest_tm_flex_f32_*` in `double_crossover.cu`
- current return preprocessing:
  - `compute_log_and_simple_returns_f32` in `double_crossover.cu`
  - `ensure_log_and_simple_returns` in `gpu_backtest_kernel.rs`
- current row-shortlist / selector kernels:
  - `select_topk_metric_rows_*`
  - `select_topk_vemr_metric_rows_*`
  - `select_topk_from_pairs_tm_f32*_v2`
- current heatmap update:
  - `heatmap_update_scores_tm_f32`
- current transpose:
  - `transpose_row_to_tm`

## Implementation Touchpoints

Use these file groups while executing the checklist:

### Phase 0-3 core backend files

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/benches/double_ma_opt.rs`

### Phase 2 dynamic/runtime spillover files

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_result.rs`
- any dynamic uploaded-batch helpers in `crates/app_core/src/gpu_backtest_kernel.rs`

### Phase 5 tooling / profiling references

- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- any existing pipeline profiling toggles already in `gpu_backtest_kernel.rs`

### Representative tests

- targeted CUDA/CPU parity tests colocated in `crates/app_core/src/gpu_backtest_kernel.rs`
- any Double MA route tests in `crates/app_core/src/double_ma.rs`
- any benchmark harness assertions or helper tests in `crates/app_core/benches/double_ma_opt.rs`

## Practical Interpretation

- The guide is directionally valid, but only a subset maps cleanly onto the current repo.
- The best immediate CUDA 13-specific experiment is shared-memory register spilling on the actual hot wrappers that the runtime currently launches.
- The best non-13.x optimization idea in the guide is removing `ret1`.
- `DeviceTopK` is now a valid experiment on this machine because CUDA 13.2 is installed locally and the headers are present, but it should remain scoped to the row-shortlist stage until proven.
- Heatmap atomics and transpose work should be profiling-gated because the current benchmark history suggests those stages are not primary bottlenecks.

## Rollout Policy

- No feature lands permanently just because it is "new in CUDA 13".
- Every change in this tracker must be justified by:
  - correctness parity,
  - no replay/pass reintroduction,
  - benchmark or profile evidence,
  - and acceptable default-path cost.
- For pure experiments, temporary compile-time / source-gated A/B branches are acceptable during development.
- No hidden runtime env toggle should remain if a feature is accepted into the normal product path.
- If a feature remains useful only for measurement, keep it as a bench/dev aid rather than product behavior.

## Performance Acceptance

- Primary KPI: `end_to_end_product` on the main contract-split benchmark surface.
- Secondary KPI: `search_only` as first-pass sweep cost.
- For always-on accepted changes:
  - prefer `<= 5%` regression on `end_to_end_product`
  - anything materially above that needs a stronger justification than "new CUDA feature"
- For opt-in or experimental branches:
  - small enabled cost is acceptable
  - feature-off path should remain effectively flat
- Reject or redesign any change that:
  - reintroduces a second pass,
  - causes a large structural regression,
  - or shifts the bottleneck into host transfer / decode without compensating benefit

## Evidence Log Requirements

For each accepted or rejected experiment, record:

- toolkit version used (`13.0` vs `13.2`)
- source variant / guard used
- targeted kernels or selector stage
- correctness checks run
- benchmark command
- benchmark result
- gate results
- keep / reject decision and reason

## Detailed Checklist

### Phase 0: Toolchain Activation and Baseline

- [ ] Decide whether this tracker will actively switch local builds/benchmarks to CUDA 13.2, or remain on CUDA 13.0 while staging source changes.
- [ ] If using CUDA 13.2 for this workstream, record the exact shell/env recipe needed to make `nvcc` and `CUDA_PATH` point to `v13.2`.
- [ ] Confirm the build path used by this repo is actually compiling the kernel with the intended toolkit during experiments.
- [ ] Confirm local CUDA 13.2 header availability for `cub::DeviceTopK`.
- [ ] Capture a fresh serialized baseline for the current contract-split benchmark before any CUDA 13 experiment starts.
- [ ] Reconfirm that no benchmark process is already alive before each benchmark run.

### Phase 1: Shared-Memory Register Spilling Experiment

- [ ] Target the current hot wrapper family first, not the older `strategy_program_double_ma_*` kernels.
- [ ] Add an isolated experiment branch / macro / guarded source variant for shared-memory spilling on:
  - `double_cross_backtest_tm_flex_f32_m7`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0`
- [ ] Capture `ptxas -v` output before/after the spill pragma.
- [ ] Record spill loads/stores, static shared-memory use, and any occupancy changes.
- [ ] Run the contract-split benchmark against the spill-enabled build.
- [ ] Run the GPU perf gate against the spill-enabled build.
- [ ] Keep the spill change only if the measured default-path improvement is real and repeatable.
- [ ] If successful, evaluate whether the same treatment is worth extending to secondary backtest kernels.

### Phase 2: `ret1` Removal Experiment

- [ ] Confirm the current `ret1` usage sites in `double_crossover.cu` and all runtime plumbing that currently caches / uploads `d_ret1`.
- [ ] Design the smallest viable experiment that replaces `ret1[t]` with `expm1f(lr_t)` in the hot double-cross path.
- [ ] Remove or bypass the extra `ret1` output from `compute_log_and_simple_returns_f32` for the experiment branch.
- [ ] Update `ensure_log_and_simple_returns` and related scratch allocation logic consistently for the experiment branch.
- [ ] Update uploaded dynamic-batch paths that currently store `d_ret1`.
- [ ] Preserve correctness for both normal and dynamic Double MA-family GPU routes.
- [ ] Add targeted CPU/GPU parity checks for metric rows impacted by `step_r`.
- [ ] Benchmark the experiment against the current baseline.
- [ ] Keep the change only if deleting the buffer/pass beats the extra `expm1f` math.

### Phase 3: `cub::DeviceTopK` Row-Shortlist Experiment

- [ ] Reconfirm the local CUDA 13.2 toolkit path and header availability before coding this experiment.
- [ ] Keep the experiment scoped to the row-shortlist selector family first:
  - `select_topk_metric_rows_*`
  - `select_topk_vemr_metric_rows_*`
- [ ] Do **not** replace the fully fused pair-scoring selectors in the first pass of this experiment.
- [ ] Measure whether the row-shortlist stage is large enough in the pipeline profile to justify the swap.
- [ ] Implement a clean A/B path comparing the current row-shortlist selector against `cub::DeviceTopK`.
- [ ] Validate correctness of selected rows / scores versus the existing selector path.
- [ ] Benchmark the full contract-split surface, not just a microkernel.
- [ ] Keep the change only if the full optimization path benefits, not just an isolated selector microbenchmark.

### Phase 4: Profiling-Gated Lower-Priority Experiments

- [ ] Adaptive slow-row dedup:
  - confirm there is a realistic tile-shape window where slow-row duplicates per warp are common enough to matter
  - prototype only after higher-priority experiments are finished
- [ ] Warp-aggregated heatmap atomics:
  - use current pipeline profile or benchmark evidence to decide if `tile_heatmap` is worth touching
  - avoid this work if heatmap remains within noise of `search_only`
- [ ] Tiled transpose:
  - use pipeline profile to decide whether `transpose_row_to_tm` is visible enough to justify a shared-memory rewrite
- [ ] If any Phase 4 experiment is attempted, treat it as optional and profiling-justified rather than mandatory.

### Phase 5: Tooling / Profiling Hygiene

- [ ] Add a written workflow for using CUDA 13.1+ memcheck / sanitizer support against this kernel file.
- [ ] Add a written workflow for using Nsight Compute / Nsight Systems on the current double-cross launch path.
- [ ] Record the exact commands used for any accepted benchmark/profile evidence.
- [ ] Record whether profiling was performed under CUDA 13.0 or 13.2.

### Phase 6: Verification and Closure

- [ ] Run targeted Rust/CUDA parity tests for any accepted kernel/runtime change.
- [ ] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` after each accepted change.
- [ ] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` before closing the tracker.
- [ ] Keep all benchmarks serialized; never launch overlapping bench/test processes.
- [ ] Record final benchmark and gate evidence directly in this document.
- [ ] Record a final keep/reject table for every experiment in scope.
- [ ] Close the tracker only after accepted changes are either:
  - merged into the normal path, or
  - explicitly documented as rejected / kept as development-only experiments

## Recommended Execution Order

1. Phase 0 toolchain activation and baseline
2. Phase 1 shared-memory spilling
3. Phase 2 `ret1` removal
4. Phase 3 `DeviceTopK`
5. Phase 4 optional profiling-gated experiments
6. Phase 5 tooling / profiling hygiene
7. Phase 6 verification and closure

## Notes On Current Prioritization

- Shared-memory spilling is the best immediate CUDA 13 experiment because it matches the actual hot wrapper family already selected by the runtime.
- `ret1` removal is probably the highest-value pure performance idea in the review, even though it is not tied to a new CUDA 13 API.
- `DeviceTopK` is now worth testing on this machine because CUDA 13.2 is physically installed and the headers exist locally, but it is still a scoped experiment, not an assumed upgrade.
- Heatmap atomics and transpose tuning should remain conditional on measured profile evidence.
- CUDA Tile remains a reject/defer item for this repo and should not appear as an open implementation task in this tracker.

## Closure Criteria

This tracker is only complete when all of the following are true:

- The active toolchain choice for the workstream (`13.0` or `13.2`) was recorded explicitly.
- Shared-memory spilling was either accepted or rejected with benchmark evidence.
- `ret1` removal was either accepted or rejected with correctness and benchmark evidence.
- `DeviceTopK` was either accepted or rejected with full-path evidence, not just microbenchmark evidence.
- Any attempted Phase 4 experiment was justified by profile evidence and documented as keep/reject.
- GPU perf/workflow gate results were recorded for the accepted final state.
- No accepted change is left as an undocumented experiment-only patch.
