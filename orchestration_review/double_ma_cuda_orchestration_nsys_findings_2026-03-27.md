# Double MA CUDA Orchestration Nsight Systems Findings

Date: 2026-03-27
Profiler: Nsight Systems 2023.3.3.42
Status: preliminary

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

## Next Steps

1. Treat Rust-side launch orchestration cost on this route as measured and bounded; it is in the tens of microseconds, not a first-order optimization target.
2. If performance work continues, pivot to GPU kernel/runtime investigation, starting with the dominant backtest kernel and its surrounding queue structure.
3. Keep NVTX visibility on the local Nsight Systems install as a compatibility task, but do not block further performance work on it.
