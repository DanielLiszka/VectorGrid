# Double MA CUDA Orchestration Evaluation Plan

Date: 2026-03-26
Updated: 2026-03-27

## Goal

Measure and, if justified by evidence, reduce the Rust-side orchestration cost involved in launching the Double MA CUDA optimization path.

This document converts the generic CUDA profiling guidance into a repo-specific plan for the current Double MA runtime.

## Implementation Status

Completed on 2026-03-27:

- Exact `GpuKernel` runs now surface an optional `gpu_kernel_timing_ms` payload in `BackendOptimizationResult`.
- That payload currently exposes:
  - `route_wall_ms`
  - `runtime_init_ms` when available
  - `set_current_ms` when available
  - `gpu_indicator_prep_ms` when available
  - `h2d_upload_ms` when available
  - `launch_submit_us` when available
  - `d2h_copy_ms` when available
  - `host_decode_ms` when available
  - `explicit_sync_ms` when available
  - `host_finalize_ms` when available
  - `graph_capture_ms` when available
  - `graph_replay_count` when available
- `launch_submit_us` is intentionally in microseconds because the measured submit path is much smaller than the millisecond-scale D2H/materialization buckets.
- `launch_submit_us` is intended to capture the executor-visible CUDA launch API time in this route and its instrumented helper stack.
- The exact-route finalize boundary was corrected so `host_finalize_ms` no longer double-counts the earlier D2H/materialization window.
- A whole-process `xlarge` Nsight Systems pass now confirms that this workload is not launch-bound:
  - `cuLaunchKernel` total API time is about 0.297 ms across 14 launches
  - the main backtest kernel accounts for about 376.9 ms of GPU time
  - the large `cuMemcpyDtoHAsync_v2` API bucket is where the host first blocks after that GPU work, not evidence of a large actual D2H payload
- The direct profiling runner now exposes:
  - warmup iterations
  - measured iterations via `--measure-iterations` or `--iterations`
  - heatmap bins
  - device id
  - objective selection
  - explicit `fast_ma` / `slow_ma` selection
  - candle-shape selection, including the benchmark-like `synth` generator
  - CUDA graph mode via `--cuda-graphs` or `--graphs`
  - NVTX mode via `--nvtx`
  - JSON-line output via `--emit-json`
- For hot ALMA product-path screening, the direct runner is now the preferred first perf surface instead of Criterion because it can reproduce the shape in a single process:
  - build once with `cargo build -p ta_app_core --release --features cuda,cuda-backtest-kernel --bin profile_gpu_dynamic_runner`
  - then run `target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload large --fast-ma alma --slow-ma alma --candle-shape synth --objective sharpe --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 64 --emit-json`
  - use Criterion only for isolated confirmation once a candidate already looks promising on that direct-runner path
- The Nsight helper scripts now expose the knobs needed for graph A/B and warmup control:
  - warmup iterations
  - heatmap bins
  - device id
  - CUDA graph mode
  - NVTX capture-range mode
- The Nsight Systems helper now also accepts an explicit `-NsysExePath` override and precreates the local `Temp\\nvidia\\nsight_systems` directory before launching `nsys`.
  - Current local result:
    - `2023.3.3` still produces usable CUDA captures but no NVTX tables from this Rust path
    - `2025.3.2`, `2025.6.3`, and `2026.2.1` all fail unelevated during their local agent/session startup on this machine before a capture completes
    - `2025.3.2` and `2026.2.1` both reproduce the deeper session-manager symptom on CLI-side probes:
      - `2025.3.2` cannot open `SessionManager-NSys-2025.3.2` under `LOCALAPPDATA\\Temp\\nvidia\\nsight_systems`
      - `2026.2.1` cannot open `SessionManager-NSys-2026.2.1` under the same root, even on `nsys sessions list`
    - profiler-side verbose logging now shows the bounded newer-CLI failure pattern:
      - the spawned `--start-agent` process fails with `Access is denied` while reopening the `create-agent-...` memory-mapped file
      - the created file ACL grants `SYSTEM` and `Administrators`, but not the current unelevated user SID
    - rerunning from an elevated PowerShell fixes the `2026.2.1` startup blocker:
      - a minimal `--trace=none` smoke test now completes and writes `target\\nsight\\nsys_2026_smoke_elevated.nsys-rep`
      - a whole-process Double MA runner capture now completes and writes `target\\nsight\\nvtx_2026_elevated_whole.nsys-rep`
    - even under an elevated `2026.2.1` capture, `nsys stats --report nvtx_sum --report nvtx_gpu_proj_sum` still reports no NVTX data from the current Rust path
    - rebuilding the runner with `nvtx-profiling` still does not produce NVTX tables under elevated `2026.2.1`
      - on this machine, that raw-dylib path also requires `C:\\Program Files\\NVIDIA Corporation\\NvToolsExt\\bin\\x64` on `PATH` just to start cleanly
      - the installed standalone `nvToolsExt64_1.dll` is the legacy 2016 package from `C:\\Program Files\\NVIDIA Corporation\\NvToolsExt\\bin\\x64`
  - Interpretation:
    - the newer-profiler startup blocker is a local privilege/tooling issue and is now confirmed to be solvable by an elevated shell on this machine
    - the remaining NVTX visibility problem is no longer just profiler startup; it is now bounded to the current Rust NVTX emission / runtime interop path because both `2023.3.3` and elevated `2026.2.1` still record zero NVTX tables
- NVTX ranges now exist in app-core behind `VECTORBT_ENABLE_NVTX=1`, and the wrapper uses runtime-loaded NVTX symbols instead of static toolkit linkage.
- The Tauri command path now also emits a `tauri_double_ma_request` NVTX range, so the app boundary is instrumented even though the local Nsight Systems install still does not expose NVTX records from this Rust path.
- The default single-pass product-row policy for the exact `GpuKernel` route now uses the 10-metric reduced contract instead of widening to the full 17-metric contract.
  - This keeps the route single-pass for the normal Sharpe/Volatility product family while avoiding the extra width that was not required to skip finalist replay.
  - Current validation:
    - targeted replay-vs-reduced parity tests still pass
    - `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed
    - `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product` improved from about 85.49 ms to about 81.98 ms
    - the replay-backed comparison path `.../search_plus_finalist_recompute` still sits around 167.29 ms on the same shape, which reinforces the single-pass preference
- Packed top-k materialization now reuses `HostTransferScratchF32` instead of a throwaway pageable `Vec<f32>`, which means the exact `GpuKernel` shortlist/materialization path can use pinned D2H staging when `VECTORBT_KERNEL_PINNED_D2H=1`.
  - Current validation:
    - focused global-top-k and product-row tests still pass
    - `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed after the change
    - `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product` stayed at about 81.85 ms, which is within noise and therefore not a regression
- A low-risk launch-bounds experiment on the dominant lockstep M7 no-risk kernel path was tested and then reverted.
  - The experiment tightened the relevant specializations to `__launch_bounds__(128, 8)` in an attempt to trade codegen toward lower register pressure.
  - It did not improve the warm runner path and pushed `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product` up to about 83.57 ms, so it was removed.
- A host-side launch-shape sweep using the existing `VECTORBT_KERNEL_BLOCK_X` override was also tested as a possible low-risk improvement.
  - One exploratory `xlarge` plus `heatmap_bins=64` sweep temporarily suggested that `block_x=160` might help.
  - The confirmation pass did not hold up:
    - `large` plus `heatmap_bins=64`: auto about 84 ms vs `160` about 82 ms
    - `xlarge` plus `heatmap_bins=64`: auto about 348 ms vs `160` about 347 ms
    - `xlarge` plus `heatmap_bins=0`: auto about 347 ms vs `160` about 1004 ms
  - Conclusion: there is no stable low-risk host-side launch-shape override to promote into the default chooser right now.
- A dispatch A/B using the existing env gates also confirmed that the current `lockstep_m7` fast path remains the best of the low-risk selection options tested.
  - disabling `VECTORBT_KERNEL_M7_SPECIALIZE` regressed relative to the default dispatch
  - disabling `VECTORBT_KERNEL_LOCKSTEP_TIME` regressed further
- The remaining measurement checklist is now materially complete for the current route:
  - warm graph A/B on `xlarge` with `heatmap_bins=0` showed no meaningful steady-state win; warm medians stayed about 327 ms with graphs off and on
  - a fresh lean warm Nsight Systems pass on the `xlarge` runner with `heatmap_bins=64` again showed kernel dominance and tiny physical memops
  - a pipeline-profile run on the same product-like shape showed about 98 percent of executor time inside `tile_score`, almost all of it in the backtest kernel itself
  - a direct Nsight Compute pass on the dominant backtest kernel showed:
    - about 70 registers per thread
    - theoretical occupancy about 58.3 percent and achieved occupancy about 50.1 percent
    - compute throughput about 61.9 percent vs memory throughput about 17.2 percent
    - a dominant L1TEX scoreboard stall reason, estimated at about 47 percent of average issue latency
  - an app-path faceoff run on a real CSV-backed `GpuKernel` request showed harness wall time effectively matching engine wall time, so there is no new warm-path evidence that a dedicated worker thread would beat the current app shell on steady-state cost alone
- The Nsight Compute helper script now resolves the local `2026.1.0` install path in addition to older layouts.
- `scripts/run_gpu_perf_gate.ps1` now waits for benchmark/profiler processes to drain before and after each benchmark/profile segment instead of assuming `cargo bench` has fully settled when it returns.
  - This was added because the local Criterion path can leave a spawned benchmark executable alive after the parent command exits, which contaminates follow-on measurements if it is not drained first.
  - Current validation:
    - script parses cleanly
    - `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` completed successfully after the hardening change
- The env-gated `VECTORBT_KERNEL_DISPATCH` log now emits the exact row-contract indicators needed for the current kernel-body track:
  - `metrics_count`
  - `tile_metrics_stride`
  - `use_lockstep_m10_product`
  - `decode_without_recompute`
- A first current-branch CUDA kernel-body change is now landed on the hot ALMA/ALMA product shape.
  - Added zero-commission specializations for the existing exact 10-row lockstep no-risk regime-off product kernels in the release `sm_89` cubin:
    - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_nocomm`
    - `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_eps0_nocomm`
  - Rust dispatch now prefers those kernels only when all of the following are true:
    - the route is already on the exact 10-row lockstep product path
    - no stop-loss or take-profit path is active
    - effective commission is exactly zero
    - `eps_zero_mode` selects the matching `_eps0_nocomm` variant when applicable
  - Intent:
    - keep the same single-pass product work and same result contract
    - remove zero-commission transition work from the hot exact-product kernels without changing nonzero-commission behavior
    - keep broader dispatch policy unchanged and preserve fallback behavior on PTX-only paths
  - Current validation:
    - targeted reduced-row parity tests still pass
    - focused global-top-k and dynamic GPU parity tests still pass
    - the exact-fee-stress GPU rerun test still passes on a nonzero-commission route
    - isolated Criterion reruns for `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product` landed around `77.45 ms` to `80.23 ms` and then `78.71 ms` to `79.03 ms`
    - `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed after isolated rerun from idle
    - release `sm_89` cubin inspection confirms the new `_nocomm` symbols are present
    - generated PTX does not carry those `m10` specializations, so PTX fallback continues to use the existing kernels rather than losing functionality
  - Interpretation:
    - this is the first currently-kept kernel-body optimization on the branch that improved the product benchmark while preserving the existing single-pass product contract
    - the win is narrower than the earlier orchestration work and stays entirely inside kernel/runtime cost, not host launch cost
    - the remaining optimization track, if more work is warranted, stays inside CUDA kernel-body work rather than host orchestration
    - if deeper CUDA work continues, the next likely wins are still per-thread instruction / latency reductions on the small-grid ALMA product path
- A second low-risk kernel-body follow-up is now kept on the same hot path.
  - Change:
    - the hot exact backtest body and regime helpers now use an explicit read-only load helper for one-dimensional scalar inputs such as `lr`, `ret1`, `regime_filter`, and the period arrays
    - this matches the existing read-only-load pattern already used for the MA matrices and keeps all result semantics unchanged
  - Current validation:
    - `double_ma::tests::gpu_kernel_product_reduced_rows_matches_replay_contract_for_sharpe`: pass
    - `gpu_backtest_kernel::tests::dynamic_double_ma_program_gpu_kernel_matches_cpu_plain_sma_metrics`: pass
    - isolated single-process ALMA/synth runner confirmation on `target/release/profile_gpu_dynamic_runner.exe`
      - prior baseline band: about `84/77/77/80/78 ms`
      - kept-change confirmation bands: about `83/77/77/78/76 ms` and `77/77/76/77/77 ms`
  - Interpretation:
    - this is a small but repeatable non-regressing improvement on the single-process product-like runner path
    - it is narrower than the zero-commission specialization win and should still be treated as a modest kernel-body cleanup rather than a first-order shift
    - a later isolated Criterion confirmation is still desirable, but the change already clears the repo's functionality bar and the current measurement-hygiene bar
- Mandatory kernel-symbol lookup errors now include the symbol name in `gpu_backtest_kernel.rs`.
  - This was added after one transient `named symbol not found` perf-gate run gave poor debugging value.
  - A fresh rebuild cleared that transient failure and the GPU perf gate completed successfully afterward.
- A follow-on branch-specialization attempt for the same exact product path was rejected and reverted.
  - Tested idea:
    - specialize away the runtime `compact_output` and valid-pair-map branches for the exact 10-row ALMA product path
  - Result:
    - targeted GPU tests still passed
    - isolated Criterion rerun stayed in the same band, about `79.74 ms` to `81.04 ms`
    - no meaningful improvement, so the extra specialization was removed
- A follow-on default-product-flags specialization inside the existing exact `m10` wrapper symbols was also rejected and reverted.
  - Tested idea:
    - keep the existing kernel symbol names, but route the common exact-product flag combination through a compile-time-specialized inner path instead of the current runtime-flag path
  - Result:
    - compile succeeded, but exact-path GPU tests did not
    - replay-backed parity hit `named symbol not found`
    - dynamic GPU parity hit `an illegal memory access was encountered`
  - Conclusion:
    - do not revisit this in-wrapper flag-specialization idea until the symbol/runtime failure mode is understood
    - the branch was restored immediately to the last known-good exact `m10` state

Still not implemented:

- a profiler-visible NVTX capture-range workflow on the local machine, because:
  - Nsight Systems `2023.3.3` captures CUDA but still emits no NVTX tables from this Rust path
  - Nsight Systems `2026.2.1` now captures successfully from an elevated shell, but it also emits no NVTX tables from this Rust path
  - rebuilding the runner with `nvtx-profiling` does not change that outcome under elevated `2026.2.1`
  - the `nvtx-profiling` raw-dylib path additionally depends on `C:\\Program Files\\NVIDIA Corporation\\NvToolsExt\\bin\\x64` being present on `PATH` for a clean process startup on this machine

## Scope

The target path is the Double MA optimization launch flow on the Rust side:

- `src-tauri/src/commands_optimize.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`

The question is not "what is total optimization runtime?".

The question is:

1. How much wall time is spent on host-side orchestration around GPU work?
2. Which parts of that host time are structural and worth optimizing?
3. If optimization is not worthwhile, can we still bound the orchestration cost precisely enough to stop guessing?

## Important Boundary Definition

There is no single universally correct number for "launch cost" unless the boundary is defined first.

For this repo, use these boundaries:

- App boundary:
  - Tauri command entry through app-core return.
  - Includes `spawn_blocking`, app-state lookup, progress plumbing, and engine work.
- Engine boundary:
  - `ta_app_core::run_double_ma_optimization_blocking_with_candles(...)` through the returned `BackendOptimizationResult`.
  - This is the main target for orchestration review.
- Kernel-executor boundary:
  - The host work inside `gpu_backtest_kernel.rs` that prepares device inputs, uploads buffers, submits kernels, synchronizes, and materializes outputs.
- GPU execution:
  - Actual device kernel runtime and device-side memory operations.

When this document says "Rust orchestration cost", it means the host-side cost inside the engine boundary, excluding pure device execution time and excluding unrelated Tauri/UI overhead unless stated otherwise.

## Current Runtime Facts In This Repo

### 1. Tauri is not the right first profiling target

The product command runs through `tauri::async_runtime::spawn_blocking(...)` in `src-tauri/src/commands_optimize.rs`.

That means a full-app trace mixes:

- Tauri command overhead
- state lookup / candle retrieval
- blocking-pool scheduling
- engine work
- GPU orchestration

If the goal is to isolate Double MA engine orchestration, start with the direct runner in `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`, not the desktop app.

### 2. Cold-start cost and steady-state cost are different

`gpu_backtest_kernel.rs` caches `CudaKernelRuntime` in thread-local storage:

- `CUDA_KERNEL_RUNTIME`
- lazy `CudaKernelRuntime::new(device_id)`

That runtime creation includes:

- CUDA init
- device lookup
- context creation
- module load / PTX or cubin load
- stream creation

This is real cost, but it is not steady-state launch overhead.

Because the Tauri path uses a blocking thread pool, thread hopping can cause runtime reuse to be less stable than a dedicated worker model. That must be measured separately from steady-state launch submission cost.

### 3. The repo already has partial internal timing

The kernel runtime now records exact-route timing in `KernelExecutorTimingMs` and surfaces it through `gpu_kernel_timing_ms`.

Current state:

- exact `GpuKernel` results expose a separate `gpu_kernel_timing_ms` payload
- the payload now includes launch-submit, D2H API bucket, sync, and output-finalize splits
- the hybrid path still uses `HybridTimingMs`
- `launch_submit_us` is precise enough to answer the original orchestration question for this route
- `d2h_copy_ms` must be interpreted carefully: on this workload it is primarily the first host blocking point after queued GPU work, not proof of a large physical D2H transfer

So the repo now has timing hooks that are sufficient to bound Rust launch orchestration cost on the exact `GpuKernel` path.

### 4. CUDA Graphs exist in the runtime, but the current helpers do not measure them well

The runtime already has graph capture / replay support in `gpu_backtest_kernel.rs`.

Graph use is currently gated by runtime conditions, including:

- pipeline profiling must be off
- `VECTORBT_KERNEL_USE_CUDA_GRAPHS=1`
- `include_all` / score-mode constraints
- no host-filter mode
- no heatmap

Current profiling helper status:

- the runner now exposes warmup, heatmap-bin, device-id, and CUDA-graph controls
- the Nsight helper scripts now expose those runner controls and can run with graphs on, off, or inherited
- the scripts still default to graphs off, which preserves prior behavior but means graph A/B remains an opt-in measurement step

So a proper graph A/B experiment is now possible with the stock runner/script combination, but only when the caller explicitly requests graph-friendly settings such as `heatmap_bins=0`.

## What Is Valid From The Generic Guide

These parts of the guide are valid for this repo and should be adopted:

1. Use Nsight Systems first.
   - This is the correct first tool for host launch gaps, syncs, memcpys, queue time, and launch structure.
2. Use Nsight Compute second.
   - Only after Systems shows which kernels are still worth deeper kernel analysis.
3. Use disassembly last.
   - `cuobjdump` / `nvdisasm` are not the first answer to an orchestration question.
4. Treat repeated allocations, small memcpys, and unnecessary syncs as orchestration debt.
5. Use a lean first profiling pass.
   - Avoid high-overhead capture modes until the first timeline pass identifies the structural issue.
6. Do a host-backtrace pass only after the lean pass suggests host overhead.
7. Do one CUDA Graph A/B experiment if the steady-state shape is stable and many short launches remain.

## Repo-Specific Corrections To The Generic Guide

These corrections are necessary for this codebase.

### 1. Do not interpret total host time as pure launch cost

In this repo, host-side time can include:

- request normalization
- period expansion
- route selection
- regime planning
- CPU precompute for staged indicators
- result aggregation / finalize
- Tauri-side dispatch overhead

So "host NVTX range time minus GPU projected time" is not automatically "Rust launch overhead". It is only "host-side non-GPU time inside the chosen boundary".

### 2. Separate cold-init cost from steady-state orchestration cost

Measure these separately:

- cold init
  - first use of runtime on a thread
- warm steady-state
  - repeated launches after runtime reuse is established

If these are mixed together, the result is not actionable.

### 3. Use the direct runner before the Tauri app

The first orchestration review should use:

```powershell
cargo run -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel -- --strategy double_ma --workload xlarge --iterations 5 --dynamic-mode off --adx-mode off
```

Then ignore iteration 1 and treat iterations 2+ as the first steady-state approximation.

### 4. Current scripts are acceptable for the first orchestration study, but not as the only interface

The existing Nsight script is now good enough for the first lean orchestration pass:

- graphs can be forced on or off
- runner warmup and steady-state iteration control are available
- graph-eligible shapes are possible when `heatmap_bins=0`
- whole-process direct-runner captures are stable on the local Nsight Systems install

Remaining limitation:

- local Nsight Systems 2023.3.3 is still not recording NVTX data from the current Rust path, so NVTX-gated capture remains blocked for now
- direct `nsys` CLI invocations are still the better interface when a capture needs uncommon backtrace or reporting options

## Canonical Cost Decomposition For This Review

Use the following decomposition.

### A. Cold runtime cost

- CUDA init
- context creation
- module load / JIT load
- stream creation

### B. Warm engine orchestration cost

- request / route normalization
- period / combo shaping
- regime preparation
- indicator preparation on host or device
- H2D upload
- graph capture or graph replay submission
- kernel launch submission
- explicit host synchronization
- D2H result materialization / decode
- host finalize / top-k / result assembly

### C. GPU execution cost

- device kernel runtime
- device-side memory ops
- queueing delay attributable to prior device work

### D. App-shell cost

- Tauri command / state / thread-pool overhead

The optimization target is section B.

## Current Measurement Surfaces Already Available

### Built-in engine/runtime timing

Available today:

- `BackendOptimizationResult.runtime_ms`
- `HybridTimingMs`
- `KernelExecutorTimingMs`
- optional pipeline profile via `VECTORBT_KERNEL_PROFILE_PIPELINE`
- optional finalist profile via `VECTORBT_KERNEL_FINALIST_PROFILE`

Important caveat:

- `runtime_ms` is broad engine time, not pure orchestration
- pipeline profiling changes runtime behavior enough that it should be treated as directional instrumentation, not ground truth
- pipeline profiling also disables graph usage in the current runtime

### External profiling

Available today:

- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- direct runner: `profile_gpu_dynamic_runner`

### Existing architectural notes worth remembering

- `validation_review/validation_throughput_cuda_plan_addendum.md` already treats CUDA Graphs as a second-stage optimization after the work shape is cleaned up
- `research/DOUBLE_MA_SEARCH_PRODUCT_GAP_REVIEW_2026-03-22.md` already frames some product-path overhead as an orchestration problem
- `ma_review/double_crossover_cuda13_valid_implementation_tracker_2026-03-25.md` already documents the general Nsight hygiene and local tool locations

## Execution Checklist

This section is the minimum operator checklist for turning the review into action.

### Preflight checklist

- [ ] Confirm the target is Double MA, not the Tauri shell or a composite profile.
- [ ] Confirm no other benchmark or profiler run is active.
- [ ] Confirm the measurement is isolated.
  - A single `cargo bench` invocation can appear as both `cargo.exe` and the spawned benchmark executable at the same time.
  - That is expected.
  - What is not acceptable is overlapping benchmark commands, overlapping profiler commands, or more than one benchmark executable from separate runs.
  - Any sample captured during overlapping perf work should be treated as contaminated and excluded from optimization decisions.
- [ ] Decide whether the target metric is:
  - app-shell cost
  - engine-side warm orchestration cost
  - cold-init cost
- [ ] Record GPU model, build variant, and whether `cuda-backtest-kernel` is enabled.
- [ ] Decide whether the run is:
  - graphs off baseline
  - graphs on A/B
  - pipeline profile
  - Nsight Systems lean
  - Nsight Systems host-backtrace
  - Nsight Compute follow-up

### Baseline checklist

- [ ] Run the direct runner for at least 5 iterations.
- [ ] Record iteration 1 separately as cold-start.
- [ ] Compute median warm runtime from iterations 2-5.
- [ ] Confirm `backend_used=Gpu` and expected route.
- [ ] Confirm the test shape is graph-ineligible or graph-eligible on purpose, not by accident.

### Instrumentation checklist

- [x] Add or verify exact-path timing for the plain `GpuKernel` route.
- [x] Add NVTX ranges at engine and kernel-executor boundaries.
- [x] Add runner flags for warmup iterations, heatmap bins, graph mode, and JSON output.
- [x] Add script flags so Nsight helpers do not force graphs off when graph A/B is the target.

### Nsight Systems checklist

- [x] Do a lean pass first.
- [x] Use warm steady-state capture, not a cold first-use iteration.
- [x] Inspect CUDA API time, queue time, sync calls, memcpy structure, and launch gaps.
- [ ] Only if host overhead is visible, run a host-backtrace pass.
- [x] Save the `.nsys-rep` path and an exported stats summary.

### Graph A/B checklist

- [x] Use the same dataset and search shape.
- [x] Disable heatmap.
- [x] Keep `include_all=false`.
- [x] Keep score mode off unless the graph path explicitly supports it.
- [x] Run graphs off and graphs on with the same warmup/measurement procedure.
- [x] Compare warm wall time and CUDA API time, not just kernel runtime.

### Improvement checklist

- [ ] If cold-init dominates, prioritize runtime reuse or a dedicated GPU worker.
- [ ] If launch API time dominates, prioritize graph replay or fewer submissions.
- [ ] If sync time dominates, remove blocking structure before touching kernel math.
- [ ] If H2D dominates, prioritize cache reuse, batching, or pinned staging.
- [ ] If host finalize dominates, optimize result materialization before kernel work.
- [x] If kernel runtime still dominates after orchestration cleanup, switch to Nsight Compute work.

### Current viable optimization checklist

Use this checklist after the 2026-03-27 orchestration pass.

Measured state for the current warm `xlarge` Double MA `GpuKernel` route:

- `launch_submit_us` is only in the tens of microseconds
- `cuLaunchKernel` is a tiny share of total CUDA API time
- the dominant backtest kernel is the main GPU cost
- the large `cuMemcpyDtoHAsync_v2` API bucket is the first host wait point, not a large physical D2H transfer

That means further performance work should currently focus on the following paths, in this order:

- [x] Run Nsight Compute on the dominant backtest kernel, not on the full app shell.
- [x] Capture Launch Statistics, occupancy, memory workload, scheduler / warp-state, and source correlation for the dominant kernel.
- [x] Record whether the main bottleneck is register pressure, memory behavior, stall reasons, or wasted work in the kernel body.
- [x] If the main backtest kernel still accounts for more than 70 percent of warm engine runtime, treat kernel/runtime work as the primary optimization track.
- [x] Inspect the queued device-side tail around reduction, shortlist, top-k, and finalist work rather than treating the D2H API bucket as a copy-bandwidth problem.
- [x] Use pipeline profiling or Systems summaries to determine whether `tile_finalize` or `final_finalize` is materially extending the host wait at the D2H API site.
- [x] Prefer device-resident shortlist and finalist handling until the smallest practical host payload is ready.
- [x] Only treat copy tuning as a real target if `cuda_gpu_mem_time_sum` shows non-trivial physical H2D or D2H memop time.
- [x] Probe low-risk host-side launch-shape overrides before changing the default chooser.
- [x] Reject launch-shape tuning as a default change unless it helps both heatmap and no-heatmap steady-state shapes.
- [x] A/B the current dispatch against `VECTORBT_KERNEL_M7_SPECIALIZE=0` and `VECTORBT_KERNEL_LOCKSTEP_TIME=0` before changing the selector; the default `lockstep_m7` route remained the best of the low-risk options tested.
- [x] If app-path cold-start or p95 latency matters, measure whether `spawn_blocking` plus thread-local `CudaKernelRuntime` reuse is unstable on the Tauri route.
- [x] If thread hopping causes repeated runtime construction, design a dedicated GPU worker thread per device before changing product-path code.
- [x] Compare any dedicated-worker design against the same cold and warm baseline before adopting it.
- [x] Move into CUDA-kernel body changes aimed at reducing register pressure and L1TEX scoreboard stalls in the dominant backtest kernel.
- [x] Prefer narrow exact-product specializations that preserve the same single-pass work, such as zero-commission wrapper variants, before considering any functionality-reducing shortcuts.
- [ ] Keep NVTX visibility work as a tooling task, but do not block kernel/runtime optimization on it.

## Exact NVTX Insertion Checklist

Use this checklist for the first instrumentation pass.

### Tauri and engine boundaries

- [x] `src-tauri/src/commands_optimize.rs`
  - [x] range: `tauri_double_ma_request`
- [x] `crates/app_core/src/double_ma.rs`
  - [x] range: `double_ma_opt_total`
  - [x] range: `route_resolve`
  - [x] range: `gpu_kernel_dispatch`
  - [x] range: `result_finalize`
  - [x] range: `hybrid_plan`
  - [x] range: `hybrid_prepare`
  - [x] range: `hybrid_transfer`
  - [x] range: `hybrid_kernel`
  - [x] range: `hybrid_fallback_gpu_sweep`
  - [x] range: `hybrid_fallback_cpu`

### Kernel-path boundaries

- [x] `crates/app_core/src/double_ma.rs`
  - [x] range: `gpu_kernel_dispatch`
  - [x] range: `hybrid_*` phase ranges when applicable
- [x] `crates/app_core/src/gpu_backtest_kernel.rs`
  - [x] range: `kernel_executor_total`
  - [x] range: `executor_impl`
  - [x] range: `runtime_init`
  - [x] range: `set_current`
  - [x] range: `backtest_core`
  - [x] range: `graph_capture`
  - [x] range: `gpu_indicator_prep`
  - [x] range: `h2d_upload`
  - [x] range: `graph_replay`
  - [x] range: `launch_submit`
  - [x] range: `sync_and_decode`
  - [x] range: `host_finalize`

### Instrumentation acceptance checks

- [x] NVTX can be disabled by env flag (`VECTORBT_ENABLE_NVTX`) for low-friction benchmarking.
- [x] Range names are stable and short.
- [x] Range placement does not create one marker per tiny row-level operation.
- [ ] Lean Nsight capture overhead stays acceptable after adding the ranges.

## Command Templates

Use these command templates as the default operator workflow.

### A. Works today: warm baseline through the direct runner

```powershell
Remove-Item Env:VECTORBT_KERNEL_PROFILE_PIPELINE -ErrorAction SilentlyContinue
Remove-Item Env:VECTORBT_KERNEL_FINALIST_PROFILE -ErrorAction SilentlyContinue
Remove-Item Env:VECTORBT_KERNEL_USE_CUDA_GRAPHS -ErrorAction SilentlyContinue
Remove-Item Env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL -ErrorAction SilentlyContinue

cargo run -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel -- --strategy double_ma --workload xlarge --iterations 5 --dynamic-mode off --adx-mode off
```

Record:

- iteration 1 wall/runtime
- median iterations 2-5 wall/runtime
- backend route

### B. Works today: directional pipeline split

```powershell
$env:VECTORBT_KERNEL_PROFILE_PIPELINE='1'
Remove-Item Env:VECTORBT_KERNEL_USE_CUDA_GRAPHS -ErrorAction SilentlyContinue

cargo run -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel -- --strategy double_ma --workload xlarge --iterations 3 --dynamic-mode off --adx-mode off

Remove-Item Env:VECTORBT_KERNEL_PROFILE_PIPELINE -ErrorAction SilentlyContinue
```

Use only for directional phase balance, not graph A/B truth.

### C. Works today: coarse Nsight Systems via existing helper

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -NvtxMode on -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/double_ma_orch_baseline
```

Current limitations:

- still a coarse wrapper over the full `nsys` CLI surface
- local NVTX trace visibility is unresolved, so `-CaptureMode nvtx` is still experimental on this machine
- lean and host-backtrace captures should still be validated directly against the emitted `.nsys-rep`

### D. Target command after NVTX visibility is resolved: lean Nsight Systems

```powershell
nsys profile `
  --trace=cuda,nvtx,osrt `
  --capture-range=nvtx `
  --nvtx-capture "double_ma_opt_total@engine" `
  --nvtx-domain-include engine,kernel `
  --sample=none `
  --cpuctxsw=none `
  --cuda-graph-trace=graph `
  --output target/nsight/double_ma_orch_lean `
  target/release/profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --workload xlarge `
  --warmup-iterations 2 `
  --measure-iterations 1 `
  --heatmap-bins 0 `
  --nvtx on `
  --graphs off `
  --dynamic-mode off `
  --adx-mode off
```

### E. Target command after NVTX visibility is resolved: host-backtrace Nsight Systems

```powershell
nsys profile `
  --trace=cuda,nvtx,osrt `
  --capture-range=nvtx `
  --nvtx-capture "double_ma_opt_total@engine" `
  --nvtx-domain-include engine,kernel `
  --sample=process-tree `
  --backtrace=fp `
  --cudabacktrace=kernel:5000,memory:5000,sync:5000 `
  --osrt-backtrace-threshold=50000 `
  --output target/nsight/double_ma_orch_host `
  target/release/profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --workload xlarge `
  --warmup-iterations 2 `
  --measure-iterations 1 `
  --heatmap-bins 0 `
  --nvtx on `
  --graphs off `
  --dynamic-mode off `
  --adx-mode off
```

Switch `--backtrace=fp` to `--backtrace=dwarf` if frame pointers are not usable.

### F. Target command after runner upgrades: graph A/B

Graphs off:

```powershell
target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload xlarge --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 0 --graphs off --dynamic-mode off --adx-mode off --emit-json
```

Graphs on:

```powershell
target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload xlarge --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 0 --graphs on --dynamic-mode off --adx-mode off --emit-json
```

### G. Current or later: Nsight Compute follow-up

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerIterations 3 -DynamicStrategyMode off -OutputPrefix target/nsight/double_ma_orch_ncu
```

Use this only after Systems shows remaining time is still device-dominant.

## Decision Thresholds

These are heuristic decision gates, not hard laws. They exist to force consistent decisions instead of vague interpretations.

### Cold-init gates

- If iteration 1 warm-minus-cold delta is greater than 10 percent of warm median or greater than 50 ms:
  - treat runtime init as a first-class cost bucket
  - do not mix it into steady-state launch conclusions

### Host-orchestration gates

- If warm host-side non-GPU time inside the engine boundary is greater than 15 percent of warm engine runtime:
  - treat orchestration as material
- If it is greater than 25 percent:
  - treat orchestration as a primary optimization candidate

### Launch-submit gates

- If `cudaLaunchKernel` / `cuLaunchKernel` API time is greater than 10 percent of warm engine runtime:
  - investigate submission structure
- If launch API time is greater than kernel time for the repeated steady-state slice:
  - treat the path as launch-bound until proven otherwise

### Sync gates

- If explicit sync APIs or visible host waits exceed 10 percent of warm engine runtime:
  - prioritize sync removal or restructuring before kernel work

### Copy gates

- If H2D + D2H combined exceed 10 percent of warm engine runtime:
  - treat transfer reduction or reuse as material
- If they exceed 20 percent:
  - treat transfer structure as a primary target

### Graph A/B gates

- If graph replay reduces warm wall time or CUDA API time by at least 10 percent:
  - graph enablement is worth implementing or expanding
- If graph replay improves total warm wall time by less than 3 percent:
  - deprioritize graphs unless it materially improves CPU availability for other reasons

### Kernel-dominance gates

- If device kernel time remains greater than 70 percent of warm engine runtime after obvious orchestration issues are removed:
  - switch attention to Nsight Compute and kernel math

## Result Capture Template

Every run in this review should fill in a table like this.

### Baseline and A/B summary

| Run ID | Target | Route | Heatmap | Graphs | Warmup iters | Measured iters | Cold wall ms | Warm median wall ms | Cold engine ms | Warm median engine ms | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fill me | `double_ma` | `GpuKernel` | `64` or `0` | `off/on` | `N` | `N` |  |  |  |  |  |

### Nsight Systems summary

| Run ID | API ms | Queue ms | Kernel ms | Memcpy ms | Sync ms | Host non-GPU ms | Launch-bound? | Sync-bound? | Copy-bound? | Kernel-bound? |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fill me |  |  |  |  |  |  | yes/no | yes/no | yes/no | yes/no |

### Internal timing summary

| Run ID | runtime_init_ms | route_resolve_ms | cpu_indicator_prep_ms | gpu_indicator_prep_ms | h2d_upload_ms | launch_submit_us | sync_decode_ms | host_finalize_ms |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fill me |  |  |  |  |  |  |  |  |

## Performance Decision Tree

Use this decision tree after the first complete measurement round.

- If cold-init is the dominant delta:
  - prioritize runtime reuse
  - consider a dedicated GPU worker thread
  - do not spend time on graphs first
- If launch API time dominates steady-state:
  - prioritize graph A/B
  - then reduce launch count or submission structure
- If sync time dominates:
  - remove syncs or change blocking structure before kernel work
- If H2D or D2H dominates:
  - prioritize caching, reuse, and transfer consolidation
- If host finalize dominates:
  - optimize aggregation and result materialization
- If none of the above dominates and kernel time is still dominant:
  - move to kernel profiling

## Recommended Evaluation Ladder

### Phase 0. Get a clean steady-state baseline

Run the direct runner, not the app, for several iterations:

```powershell
cargo run -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel -- --strategy double_ma --workload xlarge --iterations 5 --dynamic-mode off --adx-mode off
```

Record:

- iteration 1 wall/runtime
- median of iterations 2-5 wall/runtime
- backend route

Purpose:

- establish cold vs warm gap before any Nsight work

Execution checks:

- [ ] Use `double_ma`
- [ ] Use at least 5 iterations
- [ ] Keep a written cold vs warm split
- [ ] Confirm route is the intended one

### Phase 1. Use the built-in pipeline profile as a rough directional split

Run a warm direct-runner capture with:

```powershell
$env:VECTORBT_KERNEL_PROFILE_PIPELINE='1'
cargo run -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel -- --strategy double_ma --workload xlarge --iterations 3 --dynamic-mode off --adx-mode off
Remove-Item Env:VECTORBT_KERNEL_PROFILE_PIPELINE -ErrorAction SilentlyContinue
```

Use this only to learn whether time clusters around:

- host_prep
- tile_prepare
- tile_score
- tile_finalize
- final_finalize

Do not use this as the final truth for graph-vs-no-graph launch cost because this mode changes runtime behavior and disables graph usage.

Execution checks:

- [ ] Save the stdout profile dump
- [ ] Record whether tile score or host finalize dominates
- [ ] Do not compare this directly to graph-enabled runs

### Phase 2. Add NVTX before serious Nsight Systems work

This is the first implementation task that should be done before relying on an Nsight Systems orchestration study.

Add NVTX domains and ranges around these phases:

- domain: `engine`
  - `double_ma_opt_total`
  - `route_resolve`
  - `cpu_indicator_precompute`
  - `gpu_kernel_dispatch`
  - `result_finalize`
- domain: `kernel`
  - `kernel_executor_total`
  - `runtime_init`
  - `set_current`
  - `gpu_indicator_prep`
  - `h2d_upload`
  - `graph_capture`
  - `graph_replay`
  - `launch_submit`
  - `sync_and_decode`
  - `host_finalize`

The point is not to create many tiny markers everywhere.

The point is to make Nsight Systems capable of answering:

- how much time is outside the GPU timeline but inside the engine
- where the major host-side buckets live
- whether graph replay collapses host submission cost

Execution checks:

- [ ] Add engine domain
- [ ] Add kernel domain
- [ ] Make instrumentation env-gated
- [ ] Verify the runner still executes normally with NVTX disabled

### Phase 3. Extend the runner before the final orchestration study

The runner should be extended with these flags:

- `--warmup-iterations N`
- `--measure-iterations N`
- `--heatmap-bins N`
- `--include-all on|off`
- `--graphs on|off|inherit`
- `--emit-json`
- `--steady-only on|off`

Why:

- current runner shape is not graph-eligible because `heatmap_bins=64`
- orchestration review needs repeatable steady-state shape control
- JSON output makes regression comparison easier than scraping formatted stdout

Execution checks:

- [ ] Add warmup flag
- [ ] Add heatmap flag
- [ ] Add graph flag
- [ ] Add JSON output
- [ ] Preserve current default behavior unless flags opt in

### Phase 4. Run a lean Nsight Systems pass on a steady-state iteration

After NVTX exists, do a lean Systems run on the runner.

Target:

- direct runner
- warm steady-state iteration
- CPU sampling off
- CUDA + NVTX + OS runtime only

Use this pass to answer:

- is CUDA API time large relative to actual kernel time?
- are there large queue gaps?
- are there repeated `cudaLaunchKernel` / `cuLaunchKernel` clusters?
- are there repeated sync calls?
- are there repeated small copies?
- are graphs absent or ineffective?

Recommended report families to read first:

- `cuda_api_sum`
- `cuda_kern_exec_sum`
- `cuda_gpu_kern_sum`
- `cuda_gpu_mem_time_sum`
- `cuda_api_sync`
- NVTX-to-GPU projected summaries if capture ranges are in place

Execution checks:

- [ ] Export stats immediately after the lean run
- [ ] Classify the run as launch/sync/copy/kernel bound
- [ ] Save the stats output beside the `.nsys-rep`

### Phase 5. Run a host-backtrace Nsight Systems pass

Only if Phase 4 suggests host-side overhead is meaningful:

- enable CPU sampling
- enable CUDA API backtraces
- keep the same narrow warm capture range

This pass should identify which Rust call sites own:

- repeated launch submission
- repeated synchronization
- repeated allocation
- repeated result materialization

Execution checks:

- [ ] Save at least one screenshot or exported trace note showing the dominant host stack
- [ ] Write down the top 3 host call sites before any optimization work starts

### Phase 6. Do one CUDA Graph A/B experiment

Only after the run shape is graph-eligible and steady:

- same dataset
- same ranges
- same top-k
- `include_all=false`
- no heatmap
- same kernel sequence
- graphs off vs graphs on

Measure:

- steady-state wall time
- Nsight Systems CUDA API time
- queue time
- launch count behavior
- host time outside projected GPU work

Interpretation:

- if API time drops materially with graphs on, launch orchestration is a real bottleneck
- if total wall time barely moves, the bottleneck is elsewhere

Execution checks:

- [ ] Use the same search shape for both runs
- [ ] Keep heatmap off in both runs
- [ ] Compare warm medians, not one-shot values
- [ ] Record both API delta and total wall delta

### Phase 7. Use Nsight Compute only after Systems

If Systems shows the GPU kernels still dominate after orchestration is cleaned up, then use:

- `scripts/profile_gpu_backtest_ncu.ps1`
- or a direct `ncu` invocation against the runner / benchmark

This phase is for kernel math, memory behavior, scheduler stalls, and source correlation.

It is not the first answer to a launch-cost question.

Execution checks:

- [ ] Only do this after Systems classification is written down
- [ ] Narrow to the hottest kernel family
- [ ] Keep notes separate from orchestration notes

## Implementation Tasks That Are Worth Doing

These are the valid implementation tasks implied by the generic guide and grounded in the current repo.

### Task 1. Add exact-path timing output for the plain `GpuKernel` route

Current issue:

- the runtime has partial executor timing
- the result model mostly surfaces that timing via `HybridTimingMs`

Add a dedicated timing payload for the exact `GpuKernel` path so the normal Double MA route reports:

- route wall ms
- runtime init ms
- set-current ms
- gpu-indicator-prep ms
- H2D upload ms
- launch-submit us
- graph capture ms
- graph replay count
- D2H copy ms
- host decode ms
- explicit sync ms
- host finalize ms

This is the fastest path to a precise answer without starting inside Nsight every time.

Definition of done:

- [x] `GpuKernel` result surfaces a timing payload
- [x] cold-init and warm executor costs are distinguishable
- [x] timings are available in the direct runner output

### Task 2. Add NVTX domains/ranges in app-core

Without NVTX, Nsight Systems can still show CUDA APIs, but it will not map cleanly onto the engine phases we care about.

NVTX is now in place in the code. The remaining work for this task is to make those ranges profiler-visible on the local Nsight Systems install, then validate whether the current range set is sufficient.

Definition of done:

- [ ] Nsight Systems can capture a single engine range by NVTX
- [ ] engine and kernel ranges are both visible
- [ ] the added markers are stable enough for repeat captures

### Task 3. Extend the runner and scripts for orchestration studies

Minimum changes:

- make the runner graph-eligible when asked
- expose warmup and steady-state iteration control
- allow heatmap off
- stop forcing graphs off in the scripts when the user wants a graph A/B
- optionally add a dedicated Nsight Systems helper script just for orchestration review

Definition of done:

- [x] runner can produce a graph-eligible shape
- [x] script can choose graphs on or off
- [x] script can choose warmup and measurement counts
- [x] runner can emit machine-readable output

### Task 4. Separate cold-init and warm steady-state in reporting

Every report for this review should carry:

- cold iteration cost
- warm median cost
- cold-minus-warm delta

This must be standard output, not an afterthought.

Definition of done:

- [ ] every baseline capture includes cold and warm
- [ ] graphs off/on reports use the same format
- [ ] the result tables in this document can be filled without ad hoc extra notes

### Task 5. Consider a dedicated GPU worker thread only if profiling justifies it

This repo currently relies on `spawn_blocking` on the Tauri side and thread-local runtime caches on the engine side.

If profiling shows that thread hopping causes repeated runtime construction or poor cache reuse, then a dedicated GPU worker model becomes a valid optimization candidate:

- one long-lived worker thread per device
- one long-lived `CudaKernelRuntime` per worker
- channel-based request submission from the app

This is a plausible orchestration optimization, but it is not justified unless measurement shows thread-pool behavior is actually visible in the cost.

Definition of done:

- [ ] evidence shows thread reuse is unstable or expensive
- [ ] a dedicated worker design exists on paper before code work starts
- [ ] the optimization is measured against the same warm baseline

## Optimizations That Should Be Considered Only After Measurement

The current measured `xlarge` Double MA state does not justify more launch-path micro-optimization.

For this workload, evaluate follow-on optimization work in this order:

1. Improve the dominant GPU kernel/runtime path.
   - use Nsight Compute on the main backtest kernel first
2. Reduce queued device-side tail work before host materialization.
   - inspect reduction, shortlist, top-k, and finalist stages
3. Stabilize runtime reuse only if the app path shows cold-path thread hopping.
   - avoid repeated cold init on different host threads
4. Reuse scratch buffers and precomputed host/device state more aggressively if profiling shows they still matter.
5. Revisit CUDA Graph replay only if a different smaller-launch workload shows real launch-bound behavior.
6. Consider pinned staging / async copy improvements only if actual GPU memop time, not just host API wait time, becomes material.

## What Not To Do First

Do not start with:

- Nsight Compute deep dives
- SASS disassembly
- generic CUDA Graph rollout everywhere
- large refactors to validation/product paths
- speculative pinned-memory work

None of those answers the current question as quickly as:

- clean steady-state baseline
- NVTX
- lean Nsight Systems
- one graph A/B

## Concrete Success Criteria

This review is complete when all of the following are true:

1. We can report cold vs warm engine runtime separately.
2. We can report a host-side orchestration breakdown for the exact `GpuKernel` path without relying only on ad hoc profiler sessions.
3. A lean Nsight Systems capture identifies whether the path is:
   - launch-bound
   - sync-bound
   - copy-bound
   - kernel-bound
4. A graph A/B result exists for a graph-eligible steady-state shape.
5. If orchestration dominates, there is a ranked optimization list backed by evidence.
6. If orchestration does not dominate, we can say so with measurements and stop treating it as an open suspicion.

## Recommended Filename Follow-On Documents

If this review turns into implementation work, add follow-on documents here rather than expanding this note indefinitely:

- `double_ma_cuda_orchestration_nvtx_plan_YYYY-MM-DD.md`
- `double_ma_cuda_orchestration_nsys_findings_YYYY-MM-DD.md`
- `double_ma_cuda_orchestration_graph_ab_YYYY-MM-DD.md`

## Short Version

The generic guide is directionally correct, but the repo-specific valid plan is:

1. Profile the direct app-core runner first, not Tauri.
2. Separate cold-init cost from warm steady-state cost.
3. Add NVTX around engine and kernel-executor phases.
4. Extend the runner/scripts so graph-eligible steady-state runs are possible.
5. Use Nsight Systems first.
6. Use one CUDA Graph A/B test after the run shape is stable.
7. Only then decide whether the right optimization is:
   - better runtime reuse
   - fewer syncs / copies
   - CUDA Graph replay
   - or simply accepting that orchestration is not the real bottleneck
