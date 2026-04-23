# Double MA CUDA Orchestration Nsight Systems Findings

Date: 2026-03-27
Profiler: Nsight Systems 2023.3.3.42 initially, with elevated 2026.2.1 follow-up
Status: updated after elevated 2026.2.1 whole-process NVTX reevaluation

## Run Shape

This was a first small-workload direct-runner capture to validate the current orchestration workflow, not the final xlarge steady-state study.

Command path used:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload small -RunnerWarmupIterations 1 -RunnerIterations 1 -RunnerHeatmapBins 0 -NvtxMode on -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/nvtx_whole_process_test
```

Primary report:

- `target/nsight/nvtx_whole_process_test.nsys-rep`

## What Worked

- Whole-process direct-runner capture is working on the local Nsight Systems install.
- The helper script now produces a usable `.nsys-rep` in `CaptureMode none`.
- CUDA summary reports are usable from the CLI.

## What Did Not Work

- NVTX-gated capture is still blocked locally.
- `nsys stats` reports `does not contain NV Tools Extension (NVTX) data` for the current Rust NVTX path.
- That means the current local workflow cannot yet use `--capture-range=nvtx` or NVTX-to-GPU projection reports as the primary measurement surface.

## Preliminary Findings

From `cuda_api_sum` and `cuda_kern_exec_sum` on this small run:

- `cuMemcpyDtoHAsync_v2` dominated CUDA API time at about 73.7 ms, roughly 88.6% of total CUDA API time.
- `cuLaunchKernel` was about 0.217 ms total across 12 launches, which is not consistent with a launch-bound steady-state path in this sample.
- `double_cross_backtest_tm_flex_f32` dominated GPU kernel time at about 73.7 ms.
- The small top-k/finalization kernels showed very large queue time because they sat behind the main backtest kernel, not because host launch submission was expensive.
- `cuStreamSynchronize` and `cuMemAllocAsync` are visible secondary costs, but they are much smaller than the main D2H copy plus kernel runtime in this sample.

Interpretation:

- This first small sample does not support the theory that Rust launch submission is the primary bottleneck.
- The stronger immediate suspects are:
  - D2H/result materialization cost
  - host finalize cost
  - or the main backtest kernel itself

## Caveats

- This was a small workload, not the target xlarge run.
- This was a whole-process runner capture, not an NVTX-bounded steady-state slice.
- The current report is still coarse because NVTX visibility is unresolved.

## Exact Internal Timing Follow-up

The executor now surfaces exact timing fields for:

- `route_wall_ms`
- `runtime_init_ms`
- `gpu_indicator_prep_ms`
- `h2d_upload_ms`
- `launch_submit_us`
- `d2h_copy_ms`
- `host_decode_ms`
- `explicit_sync_ms`
- `host_finalize_ms`
- `graph_capture_ms`
- `graph_replay_count`

Warm xlarge direct-runner samples with `heatmap_bins=0` and NVTX disabled for low overhead now show:

- graphs off:
  - measured `route_wall_ms`: 364 ms, 367 ms, 366 ms
  - measured `launch_submit_us`: 33 us, 31 us, 54 us
  - measured `d2h_copy_ms`: 354 ms, 358 ms, 357 ms
- graphs on:
  - measured `route_wall_ms`: 370 ms, 367 ms, 364 ms
  - measured `launch_submit_us`: 49 us, 65 us, 45 us
  - measured `d2h_copy_ms`: 361 ms, 358 ms, 355 ms
  - `graph_replay_count=1` on measured iterations

Interpretation:

- The first executor-visible host blocking bucket still appears at the D2H API call site on this xlarge shape.
- The current executor-visible host-side launch submission bucket is tiny:
  - graphs off warm median `launch_submit_us` is about 33 us
  - graphs on warm median `launch_submit_us` is about 49 us
- That launch bucket is four orders of magnitude smaller than the warm D2H bucket on this shape, so Rust-side kernel submission is not the primary remaining cost.
- `host_decode_ms` is currently sub-millisecond on these runs after integer-millisecond rounding.
- `host_finalize_ms` is also effectively zero after the timing boundary was corrected, so the previous apparent finalize cost was instrumentation overlap rather than real post-materialization work.
- The current warm graph A/B does not show a meaningful end-to-end win on this xlarge shape; route-wall and D2H remain in the same general band with or without graphs.
- Graph replay is therefore no longer the first optimization target for this workload.

## Follow-on Optimization Attempt

Based on the measurements above, the first implemented optimization attempt was not more launch tuning. It was narrowing the default single-pass product row contract for the exact `GpuKernel` path:

- before this change, the default Sharpe/Volatility product policy widened the search rows to the full 17-metric contract in order to avoid finalist replay
- after this change, the same policy widens only to the 10-metric reduced product contract, which is still sufficient to decode the normal product-facing metrics without a finalist replay

Validation:

- targeted unit coverage still passes for the search-metrics policy and replay-vs-reduced parity
- `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed after the change

Measured effect on the ALMA contract-split benchmark family:

- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`
  - before: about 85.49 ms
  - after: about 81.98 ms
  - delta: about -4.17 percent
- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_plus_finalist_recompute`
  - current: about 167.29 ms

Interpretation:

- The one-pass product route is now materially faster on this benchmark shape.
- The replay-backed finalist path remains roughly 2x the cost of the new one-pass `end_to_end_product` path on the same workload.
- This supports the policy direction that the default product route should stay single-pass whenever the reduced single-pass row contract preserves the needed result semantics.

## Xlarge Whole-Process Nsight Confirmation

Whole-process xlarge capture:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerWarmupIterations 1 -RunnerIterations 1 -RunnerHeatmapBins 0 -NvtxMode off -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/xlarge_orchestration_off
```

Primary report:

- `target/nsight/xlarge_orchestration_off.nsys-rep`

Key CLI summaries:

- `cuda_api_sum`
  - `cuMemcpyDtoHAsync_v2`: about 376.9 ms, about 90.4% of CUDA API time
  - `cuLaunchKernel`: about 0.297 ms total across 14 launches
- `cuda_gpu_kern_sum`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`: about 376.9 ms, about 97.7% of GPU kernel time
- `cuda_gpu_mem_time_sum`
  - actual device-to-host memcopy time: about 1.7 us total

Interpretation:

- The large `cuMemcpyDtoHAsync_v2` API bucket on this xlarge run is not evidence of a large physical D2H transfer.
- The actual GPU D2H memop time is tiny; the host API call is simply where the submit thread first waits for the long-running backtest kernel and the queued reduction kernels ahead of the copy.
- This aligns with the internal timing result: the `d2h_copy_ms` bucket is the first host-visible blocking point after GPU work, not a direct measure of copy bandwidth pressure.
- Combined with `launch_submit_us` in the tens of microseconds, this means the remaining performance problem is not Rust orchestration overhead.
- If performance work continues from here, the priority should shift to GPU kernel/runtime work or deeper device-resident pipeline changes, not launch submission tuning.

## Warm Graph A/B Follow-up

Warm xlarge direct-runner comparison with `heatmap_bins=0`, `include_all=false`, `objective=Sharpe`, two warmup iterations, and five measured iterations:

- graphs off
  - measured `wall_ms`: 320 ms, 331 ms, 332 ms, 327 ms, 327 ms
  - measured `route_wall_ms`: 320 ms, 331 ms, 331 ms, 327 ms, 327 ms
  - measured `launch_submit_us`: 31 us, 30 us, 37 us, 29 us, 31 us
- graphs on
  - measured `wall_ms`: 324 ms, 330 ms, 327 ms, 325 ms, 327 ms
  - measured `route_wall_ms`: 324 ms, 330 ms, 327 ms, 325 ms, 327 ms
  - measured `launch_submit_us`: 40 us, 44 us, 43 us, 40 us, 42 us
  - `graph_replay_count=1` on all measured iterations

Interpretation:

- Warm median wall time stayed about 327 ms with graphs off and on.
- The current exact `GpuKernel` xlarge shape is therefore not getting a meaningful steady-state win from CUDA Graph replay.
- Graphs are not the next optimization target for this route.

## Pipeline Profile Follow-up

Warm xlarge pipeline-profile run on the product-like shape (`heatmap_bins=64`, `topk=50`, `sma/sma`) produced:

- total: about 323.2 ms
- `host_prep`: about 0.002 ms
- `tile_prepare`: about 5.27 ms
- `tile_score`: about 317.8 ms
  - `backtest`: about 317.7 ms
  - `heatmap`: about 0.14 ms
  - `score_other`: about 0.002 ms
- `tile_finalize`: about 0.07 ms
- `final_finalize`: about 0.004 ms

Interpretation:

- Roughly 98 percent of the executor-visible time is inside `tile_score`.
- Almost all of `tile_score` is the main backtest kernel, not heatmap update or host-side finalize work.
- The queued device-side tail around finalize/reduction exists, but it is not a first-order bottleneck on this shape.

## Fresh Lean Nsight Systems Pass

Fresh lean warm whole-process capture:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 64 -NvtxMode on -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/xlarge_lean_nvtx_20260327
```

Primary report:

- `target/nsight/xlarge_lean_nvtx_20260327.nsys-rep`

Key CLI summaries:

- `cuda_api_sum`
  - `cuStreamSynchronize`: about 340.1 ms, about 91.4% of CUDA API time
  - `cuLaunchKernel`: about 0.242 ms total across 18 launches
  - `cuMemcpyDtoH_v2`: about 0.083 ms total across 3 calls
- `cuda_gpu_kern_sum`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`: about 330.2 ms, about 97.4% of GPU kernel time
- `cuda_gpu_mem_time_sum`
  - actual H2D memop time: about 63.6 us total
  - actual D2H memop time: about 7.5 us total
  - actual D2D memop time: about 2.9 us total
- `nvtx_gpu_proj_sum`
  - skipped; local Nsight Systems 2023.3.3 still reports no NVTX data from the current Rust path

Interpretation:

- This repeat lean pass reaches the same conclusion as the earlier xlarge run: actual memops are tiny and the main backtest kernel is the dominant cost.
- The host-visible blocking point moved from the D2H API bucket in the earlier pass to `cuStreamSynchronize` in this run, but the meaning is the same: that API site is where the host first waits for the already-queued long-running kernel sequence.
- NVTX visibility is still a tooling compatibility problem, not a blocker to classification.

## Elevated 2026.2.1 Whole-Process NVTX Lean Revisit

Warm xlarge direct-runner reevaluation under elevated whole-process `2026.2.1`:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -NsysExePath "C:\Program Files\NVIDIA Corporation\Nsight Systems 2026.2.1\target-windows-x64\nsys.exe" -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 64 -NvtxMode on -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/xlarge_lean_nvtx_20260327_revisit
```

Primary artifacts:

- `target/nsight/xlarge_lean_nvtx_20260327_revisit.nsys-rep`
- `target/nsight/xlarge_lean_nvtx_20260327_revisit_stats.csv`

Key CLI summaries:

- `cuda_api_sum`
  - `cuStreamSynchronize`: about 758.8 ms, about 95.8% of CUDA API time
  - `cuLaunchKernel`: about 0.498 ms total across 50 launches
- `cuda_gpu_kern_sum`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_nocomm`: about 731.4 ms, about 96.5% of GPU kernel time
- `cuda_gpu_mem_time_sum`
  - actual H2D memop time: about 67.8 us total
  - actual D2H memop time: about 23.2 us total
  - actual D2D memop time: about 8.8 us total
- `nvtx_gpu_proj_sum`
  - `double_ma_opt_total`: range about 257.9 ms, projected GPU about 257.4 ms
  - `kernel::backtest_core`: range about 257.0 ms, projected GPU about 256.8 ms
  - `kernel::sync_and_decode`: range about 498.1 ms, projected GPU only about 0.0002 ms

Interpretation:

- The engine boundary now projects almost entirely to GPU work on this warm runner shape.
- The host-only gap inside `double_ma_opt_total` is sub-millisecond, which is stronger evidence than the earlier coarse non-NVTX pass.
- `kernel::sync_and_decode` is overwhelmingly a host wait / host-side non-GPU bucket at the chosen boundary, not hidden GPU execution and not Rust launch submission.
- The `kernel::launch_submit` NVTX range should not be read as launch overhead just because it projects to long GPU work; that range encloses enqueue, and Nsight projects the descendant GPU work into it.

## Elevated 2026.2.1 Whole-Process NVTX Graph Revisit

Warm xlarge graph off/on reevaluation under elevated whole-process `2026.2.1`, with `heatmap_bins=0` and identical warmup/measurement shape:

- graphs off
  - report: `target/nsight/xlarge_graph_off_nvtx_revisit.nsys-rep`
  - stats: `target/nsight/xlarge_graph_off_nvtx_revisit_stats.csv`
  - `double_ma_opt_total`: about 252.9 ms range time vs about 252.4 ms projected GPU
  - `kernel::backtest_core`: about 252.1 ms range time vs about 251.9 ms projected GPU
  - `kernel::sync_and_decode`: about 487.7 ms range time vs about 0.00007 ms projected GPU
  - `cuLaunchKernel`: about 0.517 ms total
- graphs on
  - report: `target/nsight/xlarge_graph_on_nvtx_revisit.nsys-rep`
  - stats: `target/nsight/xlarge_graph_on_nvtx_revisit_stats.csv`
  - `double_ma_opt_total`: about 248.8 ms range time vs about 248.2 ms projected GPU
  - `kernel::backtest_core`: about 247.9 ms range time vs about 247.3 ms projected GPU
  - `kernel::sync_and_decode`: about 478.7 ms range time vs about 0.00007 ms projected GPU
  - graph APIs are visible but tiny:
    - `cuGraphInstantiate_v2`: about 0.283 ms
    - `cuGraphLaunch`: about 0.118 ms
    - `cuStreamBeginCapture_v2`: about 0.265 ms

Interpretation:

- Projected GPU time is effectively unchanged with graphs off vs on on the current steady-state xlarge no-heatmap shape.
- Host-only gap remains sub-millisecond in both runs.
- The earlier graph conclusion still stands, but it is now backed by NVTX projection instead of only wall time and internal timing.
- Graph replay remains low priority for this route.

## Nsight Compute Follow-up

Direct Nsight Compute report:

- `target/nsight/xlarge_ncu_20260327.ncu-rep`

Dominant kernel:

- `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`

Key findings from Launch Statistics / Occupancy / Speed of Light / Memory / Scheduler / Warp State:

- duration: about 368.95 ms
- registers per thread: 70
- block size: 128
- waves per SM: about 4.29
- theoretical occupancy: about 58.3%
- achieved occupancy: about 50.1%
- compute throughput: about 61.9%
- memory throughput: about 17.2%
- L2 hit rate: about 96.4%
- local/shared spilling requests: 0
- eligible warps per scheduler: about 1.34
- active warps per scheduler: about 6.03
- dominant stall guidance: L1TEX scoreboard dependency, estimated at about 47.3% of average issue latency

Interpretation:

- This kernel is compute-heavier than memory-heavier on the current shape.
- The main structural limiter in the measured launch is register pressure, which caps occupancy well below the hardware ceiling.
- The main performance suspect inside the kernel body is latency waiting on L1TEX-backed data dependencies, not launch overhead and not obvious spill traffic.
- The next real optimization track, if more work is warranted, is CUDA kernel-body work aimed at register pressure and memory-access latency.

## App-Path Warm Check

Real CSV-backed app-path faceoff run:

```powershell
cargo run -p ta_desktop_demo_app --release --features cuda,cuda-backtest-kernel --bin ta_app_path_faceoff -- --csv data/SPY.csv --output-json target/faceoff/app_path_faceoff_spy.json --cache-dir target/faceoff/cache --backend gpu-kernel --fast-ma sma --slow-ma sma --objective sharpe --top-k 50 --heatmap-bins 64 --fast-period 5:500:1 --slow-period 10:1000:1 --warmup-trials 1 --measured-trials 3
```

Median measured timing summary from `target/faceoff/app_path_faceoff_spy.json`:

- harness wall: about 10340.4 ms
- engine runtime: about 10340.0 ms
- exact `GpuKernel` route wall: about 10340.0 ms
- `launch_submit_us`: about 308 us
- `h2d_upload_ms`: about 7958 ms

Interpretation:

- On this real-dataset app path, the outer app-shell timing is effectively the same as engine timing; there is no warm steady-state evidence that the Tauri shell itself is the primary issue.
- `launch_submit_us` is still tiny relative to total runtime.
- This one run does not prove a dedicated worker thread is useless for every cold-start or p95 scenario, but it does not justify that design from current warm-path evidence alone.

## App-Path Whole-Process NVTX Revisit

The earlier isolated `target/app_path_revisit/...` rebuild failure was real, but it turned out not to be a product-path regression in the app shell.

Root cause:

- the stale `target/app_path_revisit/release/ta_app_path_faceoff.exe` build had fallen back to `double_crossover.prebuilt.ptx`
- that fallback PTX did not contain `compute_log_and_simple_returns_f32`
- the runtime then failed later with `named symbol not found`, which made the app-path revisit look like a live workload regression when it was actually a stale CUDA artifact problem
- `crates/app_core/build.rs` now validates required PTX symbols on both fresh `nvcc` output and prebuilt fallback copies so this class of mismatch fails at build time instead of later at runtime

Repaired fresh build:

- isolated target dir: `target/app_path_revisit_fresh`
- build shape: `VECTORBT_REQUIRE_NVCC=1` with the current `build.rs` NVCC path
- real-data control validation succeeded first on the same SMA/SPY route via the app-core CLI helper
- the fresh profiler-visible app-shell artifacts are now:
  - `target/faceoff/app_path_faceoff_spy_revisit_fresh.json`
  - `target/faceoff/app_path_faceoff_spy_revisit_fresh_nvtxdyn_profiled.json`
  - `target/nsight/app_path_faceoff_spy_revisit_fresh_nvtxdyn.nsys-rep`
  - `target/nsight/app_path_faceoff_spy_revisit_fresh_nvtxdyn_stats.csv`

Important boundary note:

- this `ta_app_path_faceoff` harness exercises `AppState` lookup plus the same blocking engine call used by the app shell
- it does not traverse the literal `run_double_ma_optimization` `spawn_blocking` wrapper, so it does not emit the `tauri_double_ma_request` range
- it is still the current profiler-visible app-shell measurement surface on this branch

Key CLI summaries from `target/nsight/app_path_faceoff_spy_revisit_fresh_nvtxdyn_stats.csv`:

- `cuda_api_sum`
  - `cuMemcpyHtoD_v2`: about 8855.2 ms total across 36 calls
  - `cuStreamSynchronize`: about 6820.2 ms total across 20 calls
  - `cuLaunchKernel`: about 1.494 ms total across 98 launches
- `cuda_gpu_kern_sum`
  - `double_cross_backtest_tm_flex_f32_lockstep_time`: about 8553.7 ms total across 7 instances
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_nocomm`: about 3765.6 ms
  - `sma_batch_from_prefix_f64_tm`: about 1821.0 ms
- `cuda_gpu_mem_time_sum`
  - actual H2D memop time: about 1.168 ms total
  - actual D2H memop time: about 15.1 us total
- `nvtx_gpu_proj_sum`
  - `double_ma_opt_total`: about 15919.4 ms range time vs about 15711.2 ms projected GPU
  - `engine::gpu_kernel_dispatch`: about 15712.1 ms range time vs about 15704.0 ms projected GPU
  - `kernel::backtest_core`: about 15675.0 ms range time vs about 15392.8 ms projected GPU
  - `kernel::h2d_upload`: about 8853.6 ms range time vs about 0.011 ms projected GPU
  - `kernel::sync_and_decode`: about 9875.2 ms range time vs about 0.424 ms projected GPU
  - `kernel::launch_submit`: about 0.506 ms range time, while descendant GPU work projects through the range as expected

Interpretation:

- The repaired app-shell revisit again shows that the outer app-shell boundary and the engine boundary move together closely on the real CSV-backed SMA route.
- `launch_submit_us` remains tiny; the app-shell revisit does not reopen Rust launch submission as a first-order target.
- The huge app-path `h2d_upload_ms` / `cuMemcpyHtoD_v2` bucket is not physical copy bandwidth cost. Actual GPU H2D time is only about 1.168 ms total, so that bucket is mostly host waiting at the upload API site while queued GPU work drains.
- The dominant cost is still the long-running GPU kernel sequence plus host blocking at synchronization/copy API sites, not warm app-shell orchestration.
- The only stricter follow-on that remains possible here would be a literal `run_double_ma_optimization` command-wrapper study if a future cold-path or p95 investigation specifically needs the `spawn_blocking` envelope.

## Follow-on Optimization: Packed Top-K D2H Scratch

The next low-risk optimization after the one-pass product-row change was to replace the throwaway pageable host buffer used for packed top-k materialization with reusable `HostTransferScratchF32`, allowing pinned D2H staging on the exact `GpuKernel` route when enabled.

Validation:

- focused global-top-k and heatmap/result-surface tests still pass
- `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed after the change
- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`
  - current: about 81.85 ms
  - delta vs previous optimized baseline: within noise, no regression

Interpretation:

- The scratch-path change is safe and keeps the exact-route materialization path aligned with the repo's pinned-D2H policy.
- It does not change the current conclusion that the dominant remaining work is inside the main backtest kernel.

## Accepted Kernel-Body Follow-up: Fast-Row Dedup On The Exact 10-Row Product Kernels

The current kept kernel-body change on the branch is a narrow specialization inside the already-selected hot product kernels rather than another global dispatch heuristic.

Change:

- add exact zero-commission release-cubin variants for the existing exact 10-row lockstep no-risk regime-off product kernels:
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_nocomm`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_eps0_nocomm`
- dispatch prefers those kernels only when:
  - the route is already on the exact 10-row lockstep product path
  - the live product/search row contract is exactly the reduced 10-row contract
  - no stop-loss or take-profit path is active
  - effective commission is zero
- nonzero-commission paths keep using the existing kernels
- PTX fallback also keeps using the existing kernels because the checked-in PTX does not carry the exact `m10` specializations

Intent:

- preserve the same single-pass product behavior and result contract
- keep the optimization targeted to the measured hot ALMA/ALMA product path
- remove zero-commission transition work from that hot kernel body without changing the paid-commission path

Validation:

- `double_ma::tests::gpu_kernel_product_reduced_rows_matches_replay_contract_for_sharpe`: pass
- `double_ma::tests::gpu_kernel_global_topk_matches_host_merge_path`: pass
- `gpu_backtest_kernel::tests::dynamic_double_ma_program_gpu_kernel_matches_cpu_plain_sma_metrics`: pass
- `double_ma::tests::gpu_kernel_exact_fee_stress_pnl_matches_rerun`: pass
- isolated Criterion rerun:
  - `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`
  - first rerun: about `77.45 ms` to `80.23 ms`
  - confirmation rerun: about `78.71 ms` to `79.03 ms`
- `scripts/run_gpu_perf_gate.ps1 -Mode candidate`: pass
- release `sm_89` cubin inspection confirms the `_nocomm` kernels are present
- the initial post-change perf-gate failure was transient; after a fresh isolated rebuild, the exact small Criterion case and the full GPU perf gate both passed

Interpretation:

- this is the first validated CUDA kernel-body improvement on the current branch
- the win is modest but real relative to the immediately previous `~79.7 ms` to `~80.5 ms` tier
- the change stays within the repo’s preferred one-pass product policy rather than reintroducing replay or widening rows

## Accepted Low-Risk Follow-up: Explicit Read-Only Scalar Loads

Next, I tested a very narrow hot-loop cleanup:

- add an explicit read-only load helper for one-dimensional scalar inputs
- use it in the hot exact backtest body for:
  - `lr`
  - `ret1`
  - `fast_periods`
  - `slow_periods`
- use it in the regime helpers for `regime_filter`

Intent:

- align those scalar reads with the same explicit read-only access pattern already used for the MA matrices
- reduce read-latency pressure without changing row contracts, dispatch policy, or product semantics

Validation:

- `double_ma::tests::gpu_kernel_product_reduced_rows_matches_replay_contract_for_sharpe`: pass
- `gpu_backtest_kernel::tests::dynamic_double_ma_program_gpu_kernel_matches_cpu_plain_sma_metrics`: pass
- isolated single-process ALMA/synth runner, same command shape used for hot-path screening:
  - previous measured band: about `84/77/77/80/78 ms`
  - first kept-change run: about `83/77/77/78/76 ms`
  - confirmation run: about `77/77/76/77/77 ms`

Interpretation:

- this is a small but repeatable non-regressing improvement on the isolated single-process runner path
- the effect is modest and should be treated as a cleanup-level kernel-body win, not a major step change
- because measurement hygiene is currently prioritized over spinning up fresh Criterion runs, the current evidence is runner-based rather than Criterion-based

## Current Artifact Note For The Exact Product Kernels

The current release artifact has an important split:

- the release `sm_89` cubin contains:
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_eps0`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_nocomm`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_eps0_nocomm`
- the generated PTX fallback does not carry the exact `m10` specializations

Interpretation:

- on this local `sm_89` machine, release benchmarking can exercise the exact `m10` cubin variants
- on PTX fallback paths, the code preserves functionality by falling back to the existing kernels instead of requiring the new symbols
- if more CUDA optimization work continues, a fresh Nsight Compute pass should be taken on the actual `_nocomm` cubin specialization rather than reusing older reports

## Diagnostic Hardening For Symbol Lookup Failures

One transient GPU perf-gate run failed with only:

- `named symbol not found`

That was not enough to identify which mandatory kernel symbol lookup had failed.

Change:

- mandatory `get_function(...)` calls in `gpu_backtest_kernel.rs` now include the symbol name in the returned error string

Result:

- the next isolated rebuild and rerun succeeded
- the exact small Criterion case succeeded
- the full GPU perf gate succeeded

Interpretation:

- this is mainly a debugging hardening change, not a performance change
- it reduces the chance of wasting time on generic CUDA-driver lookup errors in future kernel-body work

## Rejected Follow-up: Compact-Output / Pair-Map Branch Specialization

Next, I tested a tighter specialization that tried to remove the always-true runtime branches around `compact_output` and `pair_index_map` on the exact 10-row ALMA product path.

Result:

- targeted GPU parity tests still passed
- isolated Criterion rerun for `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product` stayed in the same band:
  - about `79.74 ms` to `81.04 ms`
- there was no meaningful improvement beyond the current kept product-path win

Conclusion:

- that branch-specialization candidate was reverted
- the current validated product-path win remains the zero-commission specialization only

## Rejected Follow-up: In-Wrapper Default Product-Flags Specialization

Next, I tested a safer variant of the earlier flag-specialization idea:

- keep the existing exact `m10` kernel symbol names
- inside those existing wrapper symbols, route the common product flag combination through a compile-time-specialized inner path
- keep the current runtime-flag path as the fallback for every other flag combination

Intent:

- reduce per-bar runtime flag work on the hot exact 10-row ALMA product path
- avoid adding any new exported kernel symbols
- preserve the same result contract and fallback behavior

Result:

- compile succeeded
- the change was not safe:
  - `double_ma::tests::gpu_kernel_product_reduced_rows_matches_replay_contract_for_sharpe`
    - failed with `named symbol not found`
  - `gpu_backtest_kernel::tests::dynamic_double_ma_program_gpu_kernel_matches_cpu_plain_sma_metrics`
    - failed with `an illegal memory access was encountered`

Conclusion:

- this candidate was reverted immediately
- the branch was revalidated on the last known-good exact `m10` state after the revert
- do not revisit this in-wrapper flag-specialization path until the symbol/runtime failure mode is understood

## Rejected Low-Risk Follow-ups

Two additional low-risk ideas were tested and rejected after measurement.

### 1. Tightened launch-bounds hint on the dominant lockstep M7 no-risk kernels

Experiment:

- change `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`
- change `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0`
- from `__launch_bounds__(128)` to `__launch_bounds__(128, 8)`

Intent:

- encourage codegen toward lower register use after Nsight Compute showed the dominant kernel was register-limited

Observed result:

- warm runner path did not show a useful improvement
- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`
  - regressed to about 83.57 ms
  - previous optimized baseline was about 81.85 ms

Conclusion:

- the launch-bounds change was reverted
- this is not a safe optimization for the current branch

### 2. Host-side block-size override sweep

Experiment:

- use the existing `VECTORBT_KERNEL_BLOCK_X` override to probe whether the default backtest launch-shape chooser was missing an easy win

Exploratory sweep on `xlarge` plus `heatmap_bins=64`:

- `96`: about 983 ms median route wall
- `128`: about 949 ms
- `160`: about 311 ms
- `192`: about 314 ms
- `224`: about 319 ms
- `256`: about 321 ms
- `320`: about 317 ms
- `384`: about 313 ms

That first pass made `160` look promising, but the confirmation pass did not hold up:

- `large` plus `heatmap_bins=64`
  - auto: about 84 ms
  - `160`: about 82 ms
- `xlarge` plus `heatmap_bins=64`
  - auto: about 348 ms
  - `160`: about 347 ms
- `xlarge` plus `heatmap_bins=0`
  - auto: about 347 ms
  - `160`: about 1004 ms

Conclusion:

- there is no stable low-risk host-side launch-shape override to promote into the default chooser
- launch-shape tuning should stay out of the default path unless it proves safe across both heatmap and no-heatmap steady-state shapes

### 3. Dispatch selector A/B with existing env gates

Experiment:

- keep the kernel code unchanged
- compare the current default dispatch against:
  - `VECTORBT_KERNEL_M7_SPECIALIZE=0`
  - `VECTORBT_KERNEL_LOCKSTEP_TIME=0`

Observed result:

- the default route continued to dispatch `lockstep_m7`
- disabling M7 specialization regressed relative to that default
- disabling lockstep time regressed further

Conclusion:

- there is no measurement support for changing the current dispatch selector toward the non-M7 or non-lockstep paths
- the remaining optimization work is now beyond low-risk host-side heuristics and into broader kernel-body changes

## Regression Check After Revert

After removing the failed launch-bounds experiment, the current branch again passed:

- `scripts/run_gpu_perf_gate.ps1 -Mode candidate`

Current gate summary:

- `gpu_criterion_mean_not_regressed_10pct`: pass
- `gpu_dynamic_strategy_ratio_not_regressed_20pct`: pass
- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`: about `79.71 ms` to `79.92 ms`
- `VECTORBT_KERNEL_DISPATCH=1` logging now emits `metrics_count`, `tile_metrics_stride`, `use_lockstep_m10_product`, and `decode_without_recompute` so future perf passes can prove the live row contract directly
- `scripts/run_gpu_perf_gate.ps1` now waits for stray benchmark/profiler processes to drain before and after each segment
  - verification rerun completed successfully and left no lingering `cargo` or `double_ma_opt` process behind

Interpretation:

- the branch is again in a clearly non-regressing state
- a targeted kernel-body change produced a measurable product-path improvement without changing the one-pass product policy
- the remaining optimization track is still CUDA kernel-body work, but it is no longer just a future plan item; it has one validated landed change
- measurement hygiene matters here:
  - a single `cargo bench` command can legitimately appear as both `cargo.exe` and the spawned benchmark executable
  - the spawned benchmark executable can also linger after the parent command returns; the GPU perf gate now drains that process explicitly before moving on
  - only one perf command should be active at a time
  - if overlapping benchmark or profiler work is observed, treat that sample as contaminated and do not use it for optimization decisions
  - for hot ALMA product-path iteration, prefer the single-process direct runner first:
    - `target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload large --fast-ma alma --slow-ma alma --candle-shape synth --objective sharpe --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 64 --emit-json`
    - this avoids Criterion’s parent/child process behavior during quick candidate screening

## NVTX Tooling Follow-up

I reran the local NVTX/tooling investigation after the code-side ranges were already in place.

Observed result:

- Nsight Systems `2023.3.3` still produces usable CUDA captures on the runner path, but `nsys stats` continues to report that the resulting `.nsys-rep` contains no NVTX data
- rebuilding the runner with the `nvtx-profiling` feature did not change that outcome on `2023.3.3`
- newer installed CLIs (`2025.3.2`, `2025.6.3`, and `2026.2.1`) do not currently complete a local capture unelevated on this machine; all fail during their local agent/session startup before profiling begins
  - `2025.3.2` and `2026.2.1` both show the deeper session-manager symptom on CLI-side probes:
    - `2025.3.2` tries to open `C:\Users\dlisz\AppData\Local\Temp\nvidia\nsight_systems\SessionManager-NSys-2025.3.2`
    - `2026.2.1` tries to open `C:\Users\dlisz\AppData\Local\Temp\nvidia\nsight_systems\SessionManager-NSys-2026.2.1`
    - those storage opens fail before any capture starts
  - profiler-side verbose logging on `2025.3.2` and `2026.2.1` now shows the concrete local access failure:
    - the spawned `--start-agent` process fails with `Access is denied` while reopening the `create-agent-...` memory-mapped file
    - the created file ACL grants `SYSTEM` and `Administrators`, but not the current unelevated user SID
    - the follow-on `SessionManager-NSys-*` storage open then fails as a consequence
  - `2025.6.3` shows the same broad pattern for its own per-version session-manager storage path
  - direct `--start-agent` probing also hangs, which is consistent with the profile command timing out while waiting for the local agent path rather than the target app itself
- rerunning from an elevated PowerShell changes the `2026.2.1` result materially:
  - `nsys profile --trace=none ... cmd /c exit 0` now succeeds and writes `target\nsight\nsys_2026_smoke_elevated.nsys-rep`
  - a whole-process Double MA runner capture with `--trace=cuda,nvtx` now succeeds and writes `target\nsight\nvtx_2026_elevated_whole.nsys-rep`
  - probing the installed `nvToolsExt64_1.dll` then exposed the concrete repo-side interop issue:
    - `GetProcAddress` succeeds for `nvtxRangePushEx` / `nvtxRangePop`
    - `GetProcAddress` fails for `nvtxDomainCreateA`, `nvtxDomainRangePushEx`, and `nvtxDomainRangePop`
    - the installed standalone `nvToolsExt64_1.dll` is therefore only exposing the global push/pop API on this machine
  - `crates\app_core\src\nvtx.rs` now falls back to default-domain push/pop ranges when those domain exports are absent instead of disabling NVTX entirely
  - after that change, the elevated whole-process capture `target\nsight\nvtx_2026_elevated_whole_fallback.nsys-rep` now produces real NVTX tables:
    - `nvtx_sum` contains ranges such as `:double_ma_opt_total`, `:engine::gpu_kernel_dispatch`, `:kernel::executor_impl`, and `:kernel::backtest_core`
    - `nvtx_gpu_proj_sum` now projects those ranges onto the CUDA work as expected
- the Nsight helper itself needed a separate repo-side repair before it could use the elevated `2026.2.1` path reliably:
  - direct PowerShell native invocation was still fragile with the space-containing target exe path and output prefix under `C:\Rust Projects\...`
  - `scripts\profile_gpu_backtest_nsys.ps1` now converts the target exe path and output prefix to native-safe short paths before invoking `nsys`
  - the helper now launches `nsys` via `Start-Process` with one composed native argument string instead of relying on PowerShell's direct native argument expansion for path-bearing arguments
  - after that repair, whole-process helper capture succeeds under elevated `2026.2.1` and writes `target\nsight\helper_2026_whole_process_smoke5.nsys-rep`
  - `nsys stats --report nvtx_sum --report nvtx_gpu_proj_sum` on that helper-produced report shows the same profiler-visible NVTX ranges as the manual whole-process run
- rebuilding the runner with `nvtx-profiling` under elevated `2026.2.1` is still not the preferred path
  - the raw-dylib build initially failed to start cleanly because `C:\Program Files\NVIDIA Corporation\NvToolsExt\bin\x64` was not on `PATH`
  - that path is no longer needed for profiler-visible NVTX in the normal release build because the runtime-loaded fallback now works
- the remaining bounded-capture failure then turned out to be a trigger-matching issue, not a missing-report quirk in the profiler itself
  - NVIDIA documents that Nsight Systems considers only registered NVTX strings for trigger matching by default
  - the current Rust NVTX path uses push/pop ranges with plain strings, not registered NVTX strings
  - a direct elevated bounded run succeeds immediately when `NSYS_NVTX_PROFILER_REGISTER_ONLY=0` is set:
    - `target\nsight\capture_range_registered0_smoke.nsys-rep`
  - the repo helper now sets that env var automatically when `-CaptureMode nvtx` is requested
  - after that helper-side fix, bounded helper capture succeeds directly and writes:
    - `target\nsight\helper_capture_range_fixed.nsys-rep`
  - `nsys stats --report nvtx_sum --report nvtx_gpu_proj_sum` on that report confirms that bounded capture now produces the same useful NVTX summaries as the whole-process path

Interpretation:

- the newer-profiler startup blocker is now confirmed to be an elevation / local tooling issue on this machine, not a repo issue
- the remaining NVTX visibility issue was not just local Nsight startup; it was also a Rust NVTX runtime interop problem on this branch
- the whole-process NVTX path is now fixed under elevated `2026.2.1`
- the repo-side follow-up was still worth doing:
  - the helper script now accepts an explicit `-NsysExePath`
  - the helper now precreates the local `Temp\\nvidia\\nsight_systems` directory before launching `nsys`
  - `crates\app_core\src\nvtx.rs` now tolerates older NVTX runtimes that only export the global push/pop API
- the practical local guidance is now clearer:
  - keep using an elevated shell for newer `nsys` on this machine
  - the repo helper now supports both whole-process and bounded NVTX capture on this machine
  - raw manual `nsys profile --capture-range=nvtx` calls still need `NSYS_NVTX_PROFILER_REGISTER_ONLY=0` unless the code moves to registered NVTX strings
  - clear `%TEMP%\\nvidia` state if the unelevated startup path needs to be revisited

## Next Steps

1. Treat Rust-side launch orchestration cost on this route as measured and bounded; it is in the tens of microseconds, not a first-order optimization target.
2. If performance work continues, stay on GPU kernel/runtime investigation and use the current exact 10-row specialization as the new baseline for further Nsight Compute-guided work.
3. Only add registered-string / real-domain NVTX support if raw manual `nsys profile --capture-range=nvtx` usage without the helper becomes important.
4. Only add a stricter literal `run_double_ma_optimization` command-wrapper study if a future cold-path or p95 investigation specifically needs the `spawn_blocking` envelope.
