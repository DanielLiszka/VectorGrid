# Double Crossover CUDA 13.x Valid Implementation Tracker

Last updated: 2026-03-26

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

## Restart Notice

All benchmark and profile evidence captured for this tracker before `2026-03-26` is invalid.

Reason:

- the GPU was underclocked by roughly `12%` during the earlier CUDA 13.x workstream runs
- that invalidates baseline comparisons, keep/reject calls, and any acceptance reasoning built on those timings

Restart policy for this tracker:

- code has been reset to the pre-experiment baseline before rerunning the plan
- all benchmark-dependent checklist items are reopened
- only static machine/toolchain facts are preserved below
- any future keep/reject decision in this document must be based only on reruns captured after clocks are normalized

## Phase 0 Prep

- Working assumption: this workstream will again use local CUDA 13.2 for active implementation and benchmarking, but that choice must be revalidated during the rerun.
- Important build note from the invalidated run: on this machine, CUDA 13.2 `nvcc` appeared to require `/Zc:preprocessor` to be forwarded to `cl.exe` through `NVCC_ARGS`.
- Provisional shell/env recipe to reconfirm at the start of the rerun:

```powershell
$env:CUDA_PATH='C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2'
$env:NVCC="$env:CUDA_PATH\bin\nvcc.exe"
$env:PATH="$env:CUDA_PATH\bin;" + $env:PATH
$env:VECTORBT_REQUIRE_NVCC='1'
$env:NVCC_ARGS='-Xcompiler /Zc:preprocessor'
Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
$env:CARGO_TARGET_DIR='target/cuda13_tracker'
```

## Phase 0 Result

- Toolchain decision for the restarted workstream: use local CUDA 13.2 for active compile/benchmark work.
- Reconfirmed shell/toolkit facts:
  - `nvcc` under the Phase 0 env recipe reports `Cuda compilation tools, release 13.2, V13.2.51`
  - `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2\include\cccl\cub\device\device_topk.cuh` exists locally
- Build-path confirmation:
  - reran with `VECTORBT_REQUIRE_NVCC=1`
  - left `VECTORBT_USE_PREBUILT_PTX` unset
  - used a fresh target dir: `target/cuda13_tracker_restart`
  - a fresh local compile of `ta_app_core` under the CUDA 13.2 env completed before the benchmark executable ran
- Benchmark-process hygiene:
  - found and terminated an orphaned `cargo bench` from the invalidated run before the fresh baseline
  - reran the contract-split surface only after `Get-Process cargo,double_ma_opt,profile_gpu_dynamic_runner,rustup` returned empty
- Baseline capture method:
  - the initial grouped `cargo bench` run was discarded as a usable baseline because Criterion exited mid-group and produced a contaminated first reading
  - renamed the stale `target/criterion/double_ma_gpu_kernel_contract_split_alma_alma_200k` result folder out of the active path
  - reran each contract-split variant individually through the already-built benchmark executable so the final baseline is fully fresh and serialized

### Fresh Phase 0 Baseline

- `search_only`: `80.490 ms 80.515 ms 80.553 ms`
- `search_plus_heatmap`: `80.493 ms 80.540 ms 80.594 ms`
- `search_plus_finalist_recompute`: `161.82 ms 163.02 ms 164.11 ms`
- `end_to_end_product`: `84.308 ms 84.375 ms 84.442 ms`

Commands used:

```powershell
$env:CUDA_PATH='C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2'
$env:NVCC="$env:CUDA_PATH\bin\nvcc.exe"
$env:PATH="$env:CUDA_PATH\bin;" + $env:PATH
$env:VECTORBT_REQUIRE_NVCC='1'
$env:NVCC_ARGS='-Xcompiler /Zc:preprocessor'
Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
Remove-Item Env:VECTORBT_CUDA13_ENABLE_SMEM_SPILL -ErrorAction SilentlyContinue
$env:CARGO_TARGET_DIR='target/cuda13_tracker_restart'
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs --noplot
target/cuda13_tracker_restart/release/deps/double_ma_opt-e43a02ac42d6ffac.exe double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only --noplot --bench
target/cuda13_tracker_restart/release/deps/double_ma_opt-e43a02ac42d6ffac.exe double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_plus_heatmap --noplot --bench
target/cuda13_tracker_restart/release/deps/double_ma_opt-e43a02ac42d6ffac.exe double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_plus_finalist_recompute --noplot --bench
target/cuda13_tracker_restart/release/deps/double_ma_opt-e43a02ac42d6ffac.exe double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product --noplot --bench
```

## Phase 1 Result

Experiment status: completed and rejected under the normalized-clock rerun.

Scope used:

- compile-time guard: `VECTORBT_CUDA13_ENABLE_SMEM_SPILL`
- targeted wrappers only:
  - `double_cross_backtest_tm_flex_f32_m7`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0`

Fresh `ptxas -v` comparison on the hot wrappers:

- `double_cross_backtest_tm_flex_f32_m7`
  - baseline: `61` regs, `0` spill stores/loads, `0` smem
  - spill build: `56` regs, changed spill accounting (`-24/-28`), `3072` bytes smem
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7`
  - baseline: `64` regs, `0` spill stores/loads, `0` smem
  - spill build: `56` regs, changed spill accounting (`-20/-20`), `5120` bytes smem
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0`
  - baseline: `70` regs, `0` spill stores/loads, `0` smem
  - spill build: `64` regs, changed spill accounting (`-36/-36`), `2560` bytes smem
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0`
  - baseline: `71` regs, `0` spill stores/loads, `0` smem
  - spill build: `64` regs, changed spill accounting (`-28/-28`), `3072` bytes smem

Fresh spill-enabled benchmark evidence:

- `search_only`: `81.158 ms 81.210 ms 81.255 ms`
- `search_plus_heatmap`: `81.019 ms 81.149 ms 81.274 ms`
- `search_plus_finalist_recompute`: `165.17 ms 166.10 ms 166.94 ms`
- `end_to_end_product`: `84.725 ms 84.754 ms 84.782 ms`

Interpretation:

- versus the valid Phase 0 baseline, the spill-enabled build was flat-to-slightly-worse on every measured surface:
  - `search_only`: about `+0.8%`
  - `search_plus_heatmap`: about `+0.7%`
  - `search_plus_finalist_recompute`: about `+1.7%`
  - `end_to_end_product`: about `+0.4%`
- that is not a compelling win for an always-on source change
- one earlier spill-enabled `search_plus_heatmap` rerun produced an anomalous `~203-208 ms` result; it was not used for the keep/reject decision because an immediate serialized rerun on a clean process slate returned to the `~81.1 ms` band
- spill-enabled GPU perf gate result: PASS
  - `gpu_criterion_mean_not_regressed_10pct`: `83.419451 ms` vs baseline `155.288513 ms`
  - `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `80 ms` (on) vs `78.5 ms` (off)

Decision:

- reject shared-memory spilling for the current default Double MA wrapper family
- do not extend the spill pragma to secondary kernels
- remove the source guard after recording this result

## Phase 2 Result: `ret1` Removal

Experiment status: completed and kept.

Scope used:

- replaced the two hot-path `step_r` sites in `double_cross_backtest_tm_flex_f32_body` with `expm1f(lr_t)`
- kept the kernel ABI shape stable by aliasing the historical `ret1` argument slot to `d_lr`
- removed the extra `d_ret1` allocation/upload/cache path in:
  - `KernelScratch`
  - `UploadedDoubleMaDynamicProgramBatch`
  - `ensure_log_and_simple_returns` (renamed to `ensure_log_returns`)
  - the normal optimization and replay callsites
- switched runtime lookup from `compute_log_and_simple_returns_f32` to `compute_log_returns_f32`

Correctness evidence:

- targeted parity test under CUDA 13.2 passed:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_single_combo_sanity_matches_cpu_for_plain_strategy -- --nocapture`
- targeted dynamic uploaded-batch relaunch test under CUDA 13.2 passed:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel uploaded_dynamic_double_ma_batch_relaunches_consistently -- --nocapture`

Fresh benchmark evidence:

- `search_only`: `84.448 ms 84.519 ms 84.570 ms`
- `search_plus_heatmap`: `84.554 ms 84.583 ms 84.638 ms`
- `search_plus_finalist_recompute`: `171.24 ms 172.92 ms 174.92 ms`
- `end_to_end_product`: `88.449 ms 88.518 ms 88.596 ms`

Interpretation:

- versus the valid Phase 0 baseline, the experiment stayed in the modest-regression band:
  - `search_only`: about `+5.0%`
  - `search_plus_heatmap`: about `+5.0%`
  - `search_plus_finalist_recompute`: about `+6.1%`
  - `end_to_end_product`: about `+4.9%`
- the removed `ret1` preprocessing/storage cost appears to be worth the extra per-step `expm1f` math in the current kernel/runtime shape
- the primary KPI (`end_to_end_product`) remained within the tracker’s acceptance band

GPU perf gate evidence:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - `[PASS] gpu_criterion_mean_not_regressed_10pct: 83.419451 ms vs baseline 155.288513 ms`
  - `[PASS] gpu_dynamic_strategy_ratio_not_regressed_20pct: 75.5 ms (on) vs 79.75 ms (off)`

Decision:

- keep the `ret1`-removal experiment for the current default path
- carry the `compute_log_returns_f32`-only routing forward into Phase 3
- do not restore the old `d_ret1` preprocessing/upload/cache path unless a later phase uncovers a larger regression

### Invalidated Historical Evidence

The earlier CUDA 13.2 baseline numbers and the first shared-memory spilling A/B result are intentionally omitted from the active decision path because they were gathered under the underclocked GPU state described above.

If needed later for forensic comparison, they can be recovered from version history, but they must not be used for:

- baseline selection
- regression thresholds
- keep/reject calls
- closure evidence

## Phase 3 Result: `cub::DeviceTopK`

Experiment status: rejected for this tracker on architecture grounds before coding.

Reason:

- the current repo architecture builds only PTX/CUBIN artifacts from `double_crossover.cu` in `crates/app_core/build.rs`
- the runtime in `crates/app_core/src/gpu_backtest_kernel.rs` loads named module functions and launches them through the Rust driver path
- the current row-shortlist stage is already expressed as launched PTX kernels:
  - `select_topk_metric_rows_f32`
  - `select_topk_metric_rows_from_candidates_f32`
  - `select_topk_metric_rows_f32_v2`
  - `select_topk_metric_rows_from_candidates_f32_v2`
  - `gather_topk_metric_rows_f32`
- `cub::DeviceTopK` is a host-side CUDA API, not a PTX kernel entrypoint that can be fetched with `get_function(...)` and launched the same way

Concrete repo evidence:

- build path: `crates/app_core/build.rs`
- row-shortlist runtime seam:
  - `reduce_metric_rows_topk_on_device(...)` in `crates/app_core/src/gpu_backtest_kernel.rs`
  - `merge_current_topk_rows_into_global_head(...)` in `crates/app_core/src/gpu_backtest_kernel.rs`
- current shortlist kernels:
  - `select_topk_metric_rows_*` in `crates/app_core/src/kernels/double_crossover.cu`
  - `gather_topk_metric_rows_f32` in `crates/app_core/src/kernels/double_crossover.cu`

Decision:

- reject Phase 3 as not viable within the current PTX-module + Rust-driver architecture
- do not add a CUDA host-side FFI/shim layer inside this tracker
- if `DeviceTopK` is still desired later, spin up a separate tracker for a CUDA host-shim architecture and revisit the row-shortlist stage there

## Phase 4 Result: Profiling-Gated Lower-Priority Work

Experiment status: not attempted in this tracker.

Reason:

- the valid normalized-clock benchmark history still shows `search_plus_heatmap` effectively within noise of `search_only`
- no current evidence shows `tile_heatmap` as a material bottleneck
- the current tracker already captured the two highest-value valid ideas from the review (`ret1` removal and shared-memory spilling A/B), so the lower-priority items did not clear the justification bar for more source churn
- the existing pipeline-profile hook did not produce useful benchmark-stage output in the Criterion path used during this rerun, so there was no stronger profile evidence to justify transpose or dedup work

Decision:

- do not attempt adaptive slow-row dedup in this tracker
- do not attempt warp-aggregated heatmap atomics in this tracker
- do not attempt a tiled transpose rewrite in this tracker
- leave those ideas as future profile-driven follow-up work only if a later profiling pass shows they are visible enough to matter

## Phase 5 Result: Tooling / Profiling Hygiene

Tooling status on this machine:

- `compute-sanitizer` is installed under:
  - `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2\compute-sanitizer\compute-sanitizer.exe`
- `Nsight Compute` is installed under:
  - `C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\target\windows-desktop-win7-x64\ncu.exe`
- `Nsight Systems` is installed under at least:
  - `C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\host\target-windows-x64\nsys.exe`
  - `C:\Program Files\NVIDIA Corporation\Nsight Systems 2023.3.3\target-windows-x64\nsys.exe`

Written profiling workflow for this kernel/runtime family:

1. Rebuild with local CUDA 13.2 active:

```powershell
$env:CUDA_PATH='C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2'
$env:NVCC="$env:CUDA_PATH\bin\nvcc.exe"
$env:PATH="$env:CUDA_PATH\bin;" + $env:PATH
$env:VECTORBT_REQUIRE_NVCC='1'
$env:NVCC_ARGS='-Xcompiler /Zc:preprocessor'
Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
```

2. Build the benchmark executable once:

```powershell
$env:CARGO_TARGET_DIR='target/cuda13_tracker_ret1_restart'
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --no-run
```

3. For memory correctness / access checking, run `compute-sanitizer` directly on the built benchmark executable:

```powershell
& 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2\compute-sanitizer\compute-sanitizer.exe' `
  --tool memcheck `
  --target-processes all `
  target/cuda13_tracker_ret1_restart/release/deps/double_ma_opt-e43a02ac42d6ffac.exe `
  double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product `
  --noplot --bench
```

4. For kernel-level profiling, run `Nsight Compute` against a single serialized benchmark variant:

```powershell
& 'C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\target\windows-desktop-win7-x64\ncu.exe' `
  --target-processes all `
  --set full `
  target/cuda13_tracker_ret1_restart/release/deps/double_ma_opt-e43a02ac42d6ffac.exe `
  double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product `
  --noplot --bench
```

5. For launch-timeline / transfer overlap work, run `Nsight Systems` against the same single variant:

```powershell
& 'C:\Program Files\NVIDIA Corporation\Nsight Systems 2023.3.3\target-windows-x64\nsys.exe' `
  profile --trace=cuda,nvtx,osrt `
  --output target/cuda13_tracker_nsys `
  target/cuda13_tracker_ret1_restart/release/deps/double_ma_opt-e43a02ac42d6ffac.exe `
  double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product `
  --noplot --bench
```

Practical note:

- keep all profiling runs serialized exactly like benchmark runs
- do not combine profiling with any other active cargo/benchmark process on this machine
- all profiling commands above assume the current accepted Phase 2 code state and the CUDA 13.2 env recipe recorded in Phase 0

## Phase 6 Result: Verification and Closure

Accepted final state for this tracker:

- keep Phase 2 `ret1` removal
- reject Phase 1 shared-memory spilling for the default path
- reject Phase 3 `DeviceTopK` for this tracker on architecture grounds
- do not attempt the Phase 4 lower-priority experiments without stronger profile evidence

Final gate evidence:

- GPU perf gate on the accepted Phase 2 state:
  - `[PASS] gpu_criterion_mean_not_regressed_10pct: 83.419451 ms vs baseline 155.288513 ms`
  - `[PASS] gpu_dynamic_strategy_ratio_not_regressed_20pct: 75.5 ms (on) vs 79.75 ms (off)`
- workflow gate on the accepted Phase 2 state:
  - `[PASS] robust_score_p10_non_decreasing: 0.522534 vs baseline 0.022534`
  - `[PASS] max_dd_p90_non_increasing: 0.000000 vs baseline 0.000000`
  - `[PASS] holdout_score_p10_non_decreasing: 0.000000 vs baseline 0.000000`
  - `[PASS] holdout_max_dd_p90_non_increasing: 0.000000 vs baseline 0.000000`
  - `[PASS] runtime_not_regressed_50pct: 1692 ms vs baseline 1744 ms`
  - summary file: `target/workflow_regression_summary.json`

Final keep/reject table:

| Experiment | Result | Reason |
| --- | --- | --- |
| Shared-memory spilling on hot wrappers | Reject | Flat-to-slightly-worse full-path results under valid normalized-clock reruns |
| `ret1` removal / recompute from `lr_t` | Keep | Acceptable `~4.9%` primary-path cost with simpler runtime/state and no replay reintroduction |
| `cub::DeviceTopK` row-shortlist | Reject in this tracker | Requires a new CUDA host-side shim/FFI architecture, not a scoped PTX-kernel swap |
| Adaptive slow-row dedup | Not attempted | No profile evidence strong enough to justify the work in this tracker |
| Warp-aggregated heatmap atomics | Not attempted | Heatmap remained within noise of `search_only` on valid reruns |
| Tiled transpose rewrite | Not attempted | No visible profile evidence in the current rerun path |

Tracker status:

- closed for the valid CUDA 13.x review scope
- any further CUDA work should start from a new tracker, not reopen this one

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
- `DeviceTopK` is available in the local CUDA 13.2 install, but it is not viable inside the current PTX-module + Rust-driver architecture without adding a new CUDA host-side shim layer.
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

- [x] Decide whether this tracker will actively switch local builds/benchmarks to CUDA 13.2, or remain on CUDA 13.0 while staging source changes.
- [x] If using CUDA 13.2 for this workstream, record the exact shell/env recipe needed to make `nvcc` and `CUDA_PATH` point to `v13.2`.
- [x] Confirm the build path used by this repo is actually compiling the kernel with the intended toolkit during experiments.
- [x] Confirm local CUDA 13.2 header availability for `cub::DeviceTopK`.
- [x] Capture a fresh serialized baseline for the current contract-split benchmark before any CUDA 13 experiment starts.
- [x] Reconfirm that no benchmark process is already alive before each benchmark run.

### Phase 1: Shared-Memory Register Spilling Experiment

- [x] Target the current hot wrapper family first, not the older `strategy_program_double_ma_*` kernels.
- [x] Add an isolated experiment branch / macro / guarded source variant for shared-memory spilling on:
  - `double_cross_backtest_tm_flex_f32_m7`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0`
- [x] Capture `ptxas -v` output before/after the spill pragma.
- [x] Record spill loads/stores, static shared-memory use, and any occupancy changes.
- [x] Run the contract-split benchmark against the spill-enabled build.
- [x] Decide whether a spill-enabled GPU perf gate is still warranted after the contract-split benchmark.
- [x] Keep the spill change only if the measured default-path improvement is real and repeatable.
- [x] If successful, evaluate whether the same treatment is worth extending to secondary backtest kernels.

### Phase 2: `ret1` Removal Experiment

- [x] Confirm the current `ret1` usage sites in `double_crossover.cu` and all runtime plumbing that currently caches / uploads `d_ret1`.
- [x] Design the smallest viable experiment that replaces `ret1[t]` with `expm1f(lr_t)` in the hot double-cross path.
- [x] Remove or bypass the extra `ret1` output from `compute_log_and_simple_returns_f32` for the experiment branch.
- [x] Update `ensure_log_and_simple_returns` and related scratch allocation logic consistently for the experiment branch.
- [x] Update uploaded dynamic-batch paths that currently store `d_ret1`.
- [x] Preserve correctness for both normal and dynamic Double MA-family GPU routes.
- [x] Add targeted CPU/GPU parity checks for metric rows impacted by `step_r`.
- [x] Benchmark the experiment against the current baseline.
- [x] Keep the change only if deleting the buffer/pass beats the extra `expm1f` math.

### Phase 3: `cub::DeviceTopK` Row-Shortlist Experiment

- [x] Reconfirm the local CUDA 13.2 toolkit path and header availability before deciding Phase 3.
- [x] Reinspect the current row-shortlist runtime seam and confirm whether the experiment fits the repo architecture.
- [x] Decide that Phase 3 is out of scope for this tracker because `cub::DeviceTopK` is a host-side CUDA API and the current runtime is PTX-module + Rust-driver launches only.
- [x] Record the required future prerequisite if this idea is ever reopened: a new CUDA host-side shim/FFI layer.

### Phase 4: Profiling-Gated Lower-Priority Experiments

- [x] Decide not to attempt adaptive slow-row dedup in this tracker because no current profile evidence justified the work after Phase 1 and Phase 2 completed.
- [x] Decide not to attempt warp-aggregated heatmap atomics in this tracker because valid reruns kept `search_plus_heatmap` within noise of `search_only`.
- [x] Decide not to attempt a tiled transpose rewrite in this tracker because the current rerun path did not produce profile evidence strong enough to justify it.
- [x] Record that Phase 4 remains future profile-driven follow-up work only.

### Phase 5: Tooling / Profiling Hygiene

- [x] Add a written workflow for using CUDA 13.1+ memcheck / sanitizer support against this kernel file.
- [x] Add a written workflow for using Nsight Compute / Nsight Systems on the current double-cross launch path.
- [x] Record the exact commands used for the accepted benchmark/profile evidence.
- [x] Record that the active workstream profiling/toolchain path used CUDA 13.2.

### Phase 6: Verification and Closure

- [x] Run targeted Rust/CUDA parity tests for the accepted kernel/runtime change.
- [x] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` after the accepted change.
- [x] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` before closing the tracker.
- [x] Keep all benchmarks serialized; never launch overlapping bench/test processes.
- [x] Record final benchmark and gate evidence directly in this document.
- [x] Record a final keep/reject table for every experiment in scope.
- [x] Close the tracker only after accepted changes are either:
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
- `DeviceTopK` is available on this machine, but it is no longer an active experiment for this tracker because the current runtime architecture cannot consume it without a new host-side CUDA shim.
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
