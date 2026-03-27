# Double MA CUDA Orchestration Evaluation Plan

Date: 2026-03-26

## Goal

Measure and, if justified by evidence, reduce the Rust-side orchestration cost involved in launching the Double MA CUDA optimization path.

This document converts the generic CUDA profiling guidance into a repo-specific plan for the current Double MA runtime.

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

The kernel runtime already records limited timing:

- `gpu_indicator_prep_ms`
- `h2d_upload_ms`

That data comes from `KernelExecutorTimingMs` in `gpu_backtest_kernel.rs`.

Current limitation:

- this timing is harvested into `HybridTimingMs`
- it is surfaced mainly through the hybrid path
- it does not currently provide a complete orchestration breakdown for the exact `GpuKernel` path

So the repo already has useful timing hooks, but they are not yet sufficient to answer the orchestration question cleanly.

### 4. CUDA Graphs exist in the runtime, but the current helpers do not measure them well

The runtime already has graph capture / replay support in `gpu_backtest_kernel.rs`.

Graph use is currently gated by runtime conditions, including:

- pipeline profiling must be off
- `VECTORBT_KERNEL_USE_CUDA_GRAPHS=1`
- `include_all` / score-mode constraints
- no host-filter mode
- no heatmap

Current profiling helpers work against this goal:

- `scripts/profile_gpu_backtest_nsys.ps1` forces `VECTORBT_KERNEL_USE_CUDA_GRAPHS=0`
- `scripts/profile_gpu_backtest_ncu.ps1` forces `VECTORBT_KERNEL_USE_CUDA_GRAPHS=0`
- `profile_gpu_dynamic_runner.rs` currently hardcodes `heatmap_bins=64`

So a proper graph A/B experiment is not yet represented by the stock runner/script combination.

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

### 4. Current scripts are acceptable for coarse capture, not for the final orchestration study

The existing Nsight scripts are useful as a starting point, but they are currently missing key capabilities for this task:

- no NVTX capture range workflow
- graphs forced off
- runner shape not graph-eligible
- no explicit warmup/steady-iteration separation

So they should be treated as baseline helpers, not the final profiling interface.

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

- [ ] Add or verify exact-path timing for the plain `GpuKernel` route.
- [ ] Add NVTX ranges at engine and kernel-executor boundaries.
- [ ] Add runner flags for warmup iterations, heatmap bins, graph mode, and JSON output.
- [ ] Add script flags so Nsight helpers do not force graphs off when graph A/B is the target.

### Nsight Systems checklist

- [ ] Do a lean pass first.
- [ ] Use warm steady-state capture, not a cold first-use iteration.
- [ ] Inspect CUDA API time, queue time, sync calls, memcpy structure, and launch gaps.
- [ ] Only if host overhead is visible, run a host-backtrace pass.
- [ ] Save the `.nsys-rep` path and an exported stats summary.

### Graph A/B checklist

- [ ] Use the same dataset and search shape.
- [ ] Disable heatmap.
- [ ] Keep `include_all=false`.
- [ ] Keep score mode off unless the graph path explicitly supports it.
- [ ] Run graphs off and graphs on with the same warmup/measurement procedure.
- [ ] Compare warm wall time and CUDA API time, not just kernel runtime.

### Improvement checklist

- [ ] If cold-init dominates, prioritize runtime reuse or a dedicated GPU worker.
- [ ] If launch API time dominates, prioritize graph replay or fewer submissions.
- [ ] If sync time dominates, remove blocking structure before touching kernel math.
- [ ] If H2D dominates, prioritize cache reuse, batching, or pinned staging.
- [ ] If host finalize dominates, optimize result materialization before kernel work.
- [ ] If kernel runtime still dominates after orchestration cleanup, switch to Nsight Compute work.

## Exact NVTX Insertion Checklist

Use this checklist for the first instrumentation pass.

### Tauri and engine boundaries

- [ ] `src-tauri/src/commands_optimize.rs`
  - [ ] range: `tauri_double_ma_request`
- [ ] `crates/app_core/src/double_ma.rs`
  - [ ] range: `double_ma_opt_total`
  - [ ] range: `route_resolve`
  - [ ] range: `result_finalize`

### Kernel-path boundaries

- [ ] `crates/app_core/src/double_ma.rs`
  - [ ] range: `gpu_kernel_dispatch`
  - [ ] range: `hybrid_phase1` when applicable
- [ ] `crates/app_core/src/gpu_backtest_kernel.rs`
  - [ ] range: `kernel_executor_total`
  - [ ] range: `runtime_init`
  - [ ] range: `set_current`
  - [ ] range: `gpu_indicator_prep`
  - [ ] range: `h2d_upload`
  - [ ] range: `graph_capture`
  - [ ] range: `graph_replay`
  - [ ] range: `launch_submit`
  - [ ] range: `sync_and_decode`
  - [ ] range: `host_finalize`

### Instrumentation acceptance checks

- [ ] NVTX can be disabled by env flag for low-friction benchmarking.
- [ ] Range names are stable and short.
- [ ] Range placement does not create one marker per tiny row-level operation.
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
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerIterations 3 -DynamicStrategyMode off -OutputPrefix target/nsight/double_ma_orch_baseline
```

Current limitations:

- no NVTX capture-range workflow
- graphs forced off by script
- runner heatmap makes the shape graph-ineligible anyway

### D. Target command after NVTX + runner/script upgrades: lean Nsight Systems

```powershell
nsys profile `
  --trace=cuda,nvtx,osrt `
  --capture-range=nvtx `
  --nvtx-capture "double_ma_opt_total@engine" `
  --nvtx-domain-include engine,gpu_kernel `
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
  --graphs off `
  --dynamic-mode off `
  --adx-mode off
```

### E. Target command after NVTX + runner/script upgrades: host-backtrace Nsight Systems

```powershell
nsys profile `
  --trace=cuda,nvtx,osrt `
  --capture-range=nvtx `
  --nvtx-capture "double_ma_opt_total@engine" `
  --nvtx-domain-include engine,gpu_kernel `
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

| Run ID | runtime_init_ms | route_resolve_ms | cpu_indicator_prep_ms | gpu_indicator_prep_ms | h2d_upload_ms | launch_submit_ms | sync_decode_ms | host_finalize_ms |
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
- domain: `gpu_kernel`
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

- runtime init ms
- set-current ms
- gpu-indicator-prep ms
- H2D upload ms
- graph capture ms
- graph replay count
- launch submit ms
- explicit sync ms
- D2H decode / materialize ms
- host finalize ms

This is the fastest path to a precise answer without starting inside Nsight every time.

Definition of done:

- [ ] `GpuKernel` result surfaces a timing payload
- [ ] cold-init and warm executor costs are distinguishable
- [ ] timings are available in the direct runner output

### Task 2. Add NVTX domains/ranges in app-core

Without NVTX, Nsight Systems can still show CUDA APIs, but it will not map cleanly onto the engine phases we care about.

NVTX is the most useful missing implementation step for this specific review.

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

- [ ] runner can produce a graph-eligible shape
- [ ] script can choose graphs on or off
- [ ] script can choose warmup and measurement counts
- [ ] runner can emit machine-readable output

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

If host orchestration is proven meaningful, evaluate these in this order:

1. Stabilize runtime reuse.
   - avoid repeated cold init on different host threads
2. Remove or reduce unnecessary syncs.
3. Reuse scratch buffers and precomputed host/device state more aggressively.
4. Enable CUDA Graph replay for stable repeated launch shapes.
5. Reduce launch count if the timeline still shows many tiny submits.
6. Consider pinned staging / async copy improvements only if copies are still visible after the structural fixes.

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
