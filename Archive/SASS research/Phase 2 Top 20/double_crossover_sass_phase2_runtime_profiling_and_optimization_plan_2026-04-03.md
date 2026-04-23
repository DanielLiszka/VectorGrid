# `double_crossover` SASS Phase 2 Runtime Profiling and Optimization Plan

Date: 2026-04-03

## Purpose

This document is the phase-2 continuation of:

- `SASS research/double_crossover_performance_findings_and_code_suggestions.md`
- `SASS research/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`
- `orchestration_review/double_ma_cuda_orchestration_evaluation_plan_2026-03-26.md`
- `orchestration_review/double_ma_cuda_orchestration_nsys_findings_2026-03-27.md`

Phase 1 closed the strongest static-contract questions around exact-wrapper specialization, `metrics_stride`, and packed-store recovery for the `m17` / `m22` families. Phase 2 shifts the work from static-only evidence to runtime attribution and profiler-guided optimization.

This plan is intentionally not a blanket hand-SASS roadmap. The current repo evidence does not justify that. The goal here is to:

1. measure where the live exact `GpuKernel` time still goes,
2. decide which source/runtime changes are still worth attempting,
3. collect research-grade artifacts that support a SASS-guided source-refactor story rather than a generic "compiler bad" story.

## Phase 1 Handoff

Treat the following as already established unless new runtime evidence disproves it:

1. Exact wrapper normalization changed the emitted store mix for `m17`, `m22`, `lockstep_time_m17`, `lockstep_time_m22`, and `windowed_m17`.
2. The row-first global top-k path remains the canonical live path.
3. The current narrow exact score-first path is functional but slower than row-first on the admitted `large` surfaces.
4. Prior orchestration work already showed that the remaining xlarge cost is not primarily Rust launch submission and not primarily physical D2H transfer bandwidth.
5. The next optimization loop should be driven by runtime profiling, not by another broad static disassembly pass.

Primary handoff artifacts:

- `SASS research/double_crossover_targeted_store_counts_2026-04-02.txt`
- `SASS research/double_crossover_runtime_verification_summary_2026-04-02.txt`
- `SASS research/double_crossover_sm89_cuobjdump_2026-04-02.sass.txt`
- `SASS research/double_crossover_sm89_cuobjdump_symbols_2026-04-02.txt`

## Current Status Update, 2026-04-03

The first runtime-attribution bundle for Phase 2 is now captured.

New summary docs:

- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_gpt54_pro_prompt_pack_2026-04-03.md`

Completed artifacts:

- direct-runner baselines:
  - `target/double_crossover_phase2_large_full17_row_first_2026-04-03.jsonl`
  - `target/double_crossover_phase2_large_trade22_row_first_2026-04-03.jsonl`
  - `target/double_crossover_phase2_xlarge_shortlist_full17_row_first_2026-04-03.jsonl`
  - `target/double_crossover_phase2_xlarge_shortlist_trade22_row_first_2026-04-03.jsonl`
- built-in pipeline logs:
  - `target/double_crossover_phase2_large_full17_pipeline_2026-04-03.log`
  - `target/double_crossover_phase2_large_trade22_pipeline_2026-04-03.log`
- Nsight Systems:
  - `target/nsight/double_crossover_phase2_large_full17.nsys-rep`
  - `target/nsight/double_crossover_phase2_large_full17_stats.txt`
  - `target/nsight/double_crossover_phase2_large_trade22.nsys-rep`
  - `target/nsight/double_crossover_phase2_large_trade22_stats.txt`
- Nsight Compute:
  - `target/nsight/double_crossover_phase2_large_full17_backtest.ncu-rep`
  - `target/nsight/double_crossover_phase2_large_trade22_backtest.ncu-rep`
  - `target/nsight/double_crossover_phase2_xlarge_reducers.ncu-rep`
  - `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`

Current measured conclusions:

1. the live exact `large` surfaces are still dominated by the main `lockstep_time_m17` / `lockstep_time_m22` backtest kernels
2. the built-in pipeline and Nsight Systems evidence do not support a reducer-first or finalize-first product story on those large exact surfaces
3. fresh Nsight Compute does not show a spill-driven story and still points primarily at long-scoreboard and access-shape pressure
4. `trade22` now shows materially better global-store bytes-per-sector than `full17`, but both are still well below ideal sector utilization
5. the bounded `xlarge` reducer capture (`gather_topk_pairs_tm_f32`) was measurable but still small enough that it does not presently justify jumping straight to a reducer-first rewrite

Immediate use of this plan should therefore be:

1. feed the captured bundle to GPT-5.4 Pro using the new prompt pack
2. let that review choose the next bounded source/runtime intervention
3. only then resume code changes

## Scope

This phase covers:

- the live `double_ma` exact `GpuKernel` route for `double_crossover`
- the main backtest body, row reducers, pair reducers, gather helpers, and compact-output behavior
- `full17` and `trade22` exact contracts
- row-first vs experimental score-first control comparisons
- fast-warp-dedup and reducer-shape specialization experiments
- runtime attribution using:
  - built-in pipeline timing
  - direct-runner JSON timing
  - Nsight Systems
  - Nsight Compute
  - optional CUPTI SASS metrics follow-up if the first two profiler layers leave unresolved attribution gaps, later completed on 2026-04-05 as a viable helper-and-attribution branch that refined the large exact lockstep story without changing product priority

This phase does not authorize:

- blanket PTX or SASS rewriting
- removing CPU fallback
- promoting score-first as the default path without new evidence
- broad host-orchestration speculation that ignores the existing March Nsight findings
- changing benchmark or product stories without parity and perf verification

## Research and Product Objectives

The work should satisfy both of these goals at the same time:

### Product goal

Continue improving the real `double_ma` GPU path where evidence supports it, while preserving parity and the current default route behavior.

### Research goal

Produce a clean attribution trail showing when static SASS evidence should trigger:

- source/layout refactors,
- reducer/dataflow changes,
- runtime-route changes,
- or only narrow low-level tuning.

## Carry-Forward Threads From Prior Research

This document is meant to be the phase-2 continuation point, but the older March 30 and April 1 research docs still contain live threads that should remain visible here.

### High-priority carry-forward threads

These should be treated as active follow-ons once the first profiler pass is complete:

1. compact exact search-stage redesign that avoids writing and rereading full metric rows before final winners are known
2. device-side finalist materialization or replay-input emission so exact replay does not depend on the current host-first finalist rebuild
3. shortlist-slice exact reruns and `trade22` tail replay correctness on any retained compact path
4. route-adoption decisions for:
   - heatmap
   - include-all
   - export-all
   - shortlist union
   - shortlist confidence rerank
5. constrained exact-route evaluation of whether `feasible_out` or a similar feasibility side output is worth the added complexity

### Medium-priority carry-forward threads

These should stay visible but should not jump ahead of the main profiling-backed architecture work:

1. one more bounded lockstep full-row body pass, but only if fresh profiling points back to the retained exact lockstep hotspot families
2. `TM`-native MA preparation beyond `sma` / `alma`
3. multi-stream or double-buffered orchestration and stronger device-resident caches for search and replay reuse

### Diagnostic-only carry-forward threads

These remain explicitly secondary unless profiler evidence justifies them:

1. real cubin-vs-PTX-JIT A/B on retained hot kernels
2. narrow PTX/SASS/codegen audit only after a compiler miss is proven
3. any renewed `windowed` body work only if a retained benchmark surface makes it worthwhile and replay-backed sensitivity is acceptable

## Current Working Hypotheses

These hypotheses should guide phase-2 measurement, but they are not yet conclusions:

1. The xlarge exact row-first path is still primarily dominated by the main backtest kernel, not by host launch overhead and not by physical copy bandwidth.
2. The `large` exact path may expose a higher relative share for row reduction, pair reduction, or gather cost than xlarge does.
3. Score-first does not currently win because the existing implementation still carries the wrong live route shape and not because the score-once idea is universally bad.
4. Fast-warp-dedup is likely distribution-sensitive and should be treated as a conditional specialization, not a universal default.
5. If row or pair gather becomes visible in Nsight Compute, the next likely product win is source/runtime restructuring rather than further static backtest-body store work.
6. If reduction or finalist materialization becomes visible, the best phase-2 architecture candidate is a compact exact search stage that keeps scored candidates compact and leaves exact `full17` / `trade22` replay authoritative for final rows.
7. If a compact path is retained, shortlist-slice reruns, `trade22` replay tails, and downstream route-adoption decisions must be treated as first-class verification, not as afterthoughts.

## Repo Tooling and Constraints

Phase 2 must use the repo's existing measurement surfaces before adding new infrastructure:

- `target/release/profile_gpu_dynamic_runner.exe`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`

Repo-specific operating constraints:

1. Serialize all benchmark and profiling runs. Do not run overlapping `cargo`, Criterion, `nsys`, or `ncu` processes.
2. Launch Nsight profiling from an elevated PowerShell shell when the local install requires it.
3. Prefer the direct runner plus `--emit-json` for first-pass steady-state screening, but do not treat it as the authoritative regression gate by itself.
4. Keep the current authority model:
   - `scripts/run_workflow_gate.ps1`, `scripts/run_cpu_perf_gate.ps1`, and `scripts/run_gpu_perf_gate.ps1` remain the authoritative gates
   - `scripts/run_gpu_alma_contract_rows_perf_gate.ps1` remains targeted supporting evidence, not the default PR authority
   - Nsight wrappers remain profiling helpers, not pass/fail gates
5. Use perf gates after candidate code changes; do not rely on profiler-only wins.
6. Keep artifacts and command provenance.
7. For complex `Nsight Compute` kernel-name regex filters on Windows, prefer direct `ncu.exe` via `-NcuExePath` instead of `ncu.bat`, because `cmd`-level parsing can break grouped regex filters.

Machine-specific notes already captured by the repo:

1. `scripts/profile_gpu_backtest_nsys.ps1` already handles native-safe path conversion, temp-dir creation, and whole-process fallback when bounded capture fails.
2. Newer local Nsight Systems installs on this machine may fail unelevated with an access-denied create-agent/session-manager startup error.
3. The repo helper already sets `NSYS_NVTX_PROFILER_REGISTER_ONLY=0` when bounded NVTX capture is requested.
4. Current profiling support is still primarily Nsight-based; a branch-local CUPTI helper now exists under [cupti_sass_probe.rs](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/bin/cupti_sass_probe.rs) and [run_cupti_sass_probe.ps1](C:/Rust%20Projects/ta_desktop_demo/scripts/run_cupti_sass_probe.ps1). The 2026-04-05 branch initially failed, but the root cause was a helper-side `FlushData` ABI bug and the large exact captures now succeed in [cupti_sass_metric_followup_findings_2026-04-05.md](C:/Rust%20Projects/ta_desktop_demo/CUDA%20kernel%20review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md).

## Primary Truth Surfaces

Phase 2 should not broaden the surface area until these are measured well:

| Surface | Why it matters | Primary tool |
|---|---|---|
| `double_ma`, `alma/alma`, `large`, `heatmap_bins=0`, `metric_contract=full17` | narrow exact row-first vs score-first control surface | direct runner + Nsight |
| `double_ma`, `alma/alma`, `large`, `heatmap_bins=0`, `metric_contract=trade22` | narrow exact trade-metric control surface | direct runner + Nsight |
| `double_ma`, `alma/alma`, `xlarge`, `shortlist_slice_mode=halves`, `metric_contract=full17` | shortlist-heavy route that remained on canonical row-first | direct runner + Nsight |
| `double_ma`, `alma/alma`, `xlarge`, `shortlist_slice_mode=halves`, `metric_contract=trade22` | shortlist-heavy trade-metric route | direct runner + Nsight |
| `double_ma/gpu_kernel_contract_rows/alma_alma/trending_200k/large/58300pairs/{auto,full17,trade22}` | controlled Criterion-backed contract-row comparison | contract-row perf gate + targeted Nsight |

Optional second-wave surfaces only after the truth set is stable:

- one compact-valid-pair route where `launch_pairs < pairs`
- one constraints-on-device route
- one fast-warp-dedup-on route with large enough pair count to plausibly matter

## Artifact Plan

Phase 2 should generate an append-only artifact set rather than mixing everything into one file. Recommended outputs:

- `SASS research/double_crossover_phase2_runtime_baseline_2026-04-03.txt`
- `SASS research/double_crossover_phase2_nsys_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_ncu_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_experiment_log_2026-04-03.md`
- `SASS research/double_crossover_phase2_static_dynamic_attribution_notes_2026-04-03.md`
- `target/nsight/*.nsys-rep`
- `target/nsight/*.ncu-rep`
- `target/nsight/*.provenance.json`
- `target/*.jsonl` from direct-runner `--emit-json`

## Measurement Order

Follow this order unless hard evidence justifies skipping a step:

1. direct-runner baseline refresh
2. built-in pipeline profile and dispatch/timing refresh
3. whole-process Nsight Systems attribution
4. kernel-filtered Nsight Compute attribution
5. targeted intervention
6. perf-gate verification
7. narrow static SASS refresh only for the kernels touched by the intervention

## Phase 0: Baseline Lock

Objective: refresh the exact baseline after Phase 1 and before new code changes.

Tasks:

1. Confirm no benchmark or profiler process is running.
2. Record:
   - branch / commit
   - GPU model and driver
   - CUDA build mode
   - profiler versions used
3. Refresh direct-runner baselines for the primary truth surfaces with `--emit-json`.
4. Record route diagnostics:
   - `gpu_kernel.path`
   - `metrics_count`
   - `tile_metrics_stride`
   - whether score-first was enabled or blocked
5. Keep score-first disabled by default during baseline capture.

Suggested commands:

```powershell
target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload large --fast-ma alma --slow-ma alma --candle-shape synth --objective sharpe --metric-contract full17 --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 0 --dynamic-mode off --cuda-graphs off --emit-json

target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload large --fast-ma alma --slow-ma alma --candle-shape synth --objective sharpe --metric-contract trade22 --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 0 --dynamic-mode off --cuda-graphs off --emit-json

target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload xlarge --fast-ma alma --slow-ma alma --candle-shape synth --objective sharpe --metric-contract full17 --shortlist-slice-mode halves --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 0 --dynamic-mode off --cuda-graphs off --emit-json

target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload xlarge --fast-ma alma --slow-ma alma --candle-shape synth --objective sharpe --metric-contract trade22 --shortlist-slice-mode halves --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 0 --dynamic-mode off --cuda-graphs off --emit-json
```

Exit condition:

- a new baseline note exists with route, timing, and env evidence for the truth surfaces

## Phase 1: Internal Timing and Dispatch Refresh

Objective: use the repo's built-in timing and dispatch outputs to decide where external profiling should focus first.

Tasks:

1. Re-run the primary truth surfaces with:
   - `VECTORBT_KERNEL_PROFILE_PIPELINE=1`
   - `VECTORBT_KERNEL_LOG_DISPATCH=1`
2. Capture:
   - `route_wall_ms`
   - `gpu_indicator_prep_ms`
   - `h2d_upload_ms`
   - `launch_submit_us`
   - `d2h_copy_ms`
   - `host_decode_ms`
   - `explicit_sync_ms`
   - `host_finalize_ms`
   - `graph_replay_count`
3. Capture pipeline-profile buckets where available:
   - `host_prep`
   - `tile_prepare`
   - `tile_score`
   - `backtest`
   - `heatmap`
   - `tile_finalize`
   - `final_finalize`
4. Confirm the active dispatch for each surface:
   - exact `m17` / `m22`
   - lockstep vs non-lockstep
   - fast-warp-dedup on/off
   - compact output on/off

Suggested commands:

```powershell
$env:VECTORBT_KERNEL_PROFILE_PIPELINE='1'
$env:VECTORBT_KERNEL_LOG_DISPATCH='1'
target/release/profile_gpu_dynamic_runner.exe --strategy double_ma --workload large --fast-ma alma --slow-ma alma --candle-shape synth --objective sharpe --metric-contract full17 --warmup-iterations 2 --measure-iterations 5 --heatmap-bins 0 --dynamic-mode off --cuda-graphs off --emit-json
Remove-Item Env:VECTORBT_KERNEL_PROFILE_PIPELINE -ErrorAction SilentlyContinue
Remove-Item Env:VECTORBT_KERNEL_LOG_DISPATCH -ErrorAction SilentlyContinue
```

Exit condition:

- each truth surface has a refreshed internal-timing summary and confirmed dispatch path

## Phase 2: Nsight Systems Attribution

Objective: classify end-to-end wall time into GPU execution, host wait points, materialization, and NVTX-visible ranges before drilling into single kernels.

Tasks:

1. Use whole-process capture first.
2. Use NVTX-enabled capture when available.
3. Prefer elevated `2026.x` Nsight Systems if the local machine requires it.
4. Capture at least one `large` exact surface and one `xlarge` shortlist-heavy surface for each contract family.
5. Record:
   - `cuda_api_sum`
   - `cuda_gpu_kern_sum`
   - `cuda_gpu_mem_time_sum`
   - `nvtx_sum`
   - `nvtx_gpu_proj_sum`
6. Confirm whether the dominant host-side wait point is still only the first blocking API boundary after GPU work rather than physical D2H cost.
7. Confirm whether `double_ma_opt_total` still projects almost entirely to GPU work on the chosen exact surfaces.

Suggested commands:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload large -RunnerMetricContract full17 -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -NvtxMode on -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/double_crossover_phase2_large_full17

powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload large -RunnerMetricContract trade22 -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -NvtxMode on -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/double_crossover_phase2_large_trade22

powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerMetricContract full17 -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -NvtxMode on -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/double_crossover_phase2_xlarge_full17

powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerMetricContract trade22 -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -NvtxMode on -CaptureMode none -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/double_crossover_phase2_xlarge_trade22
```

Optional bounded-capture follow-up after whole-process success:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerMetricContract full17 -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -NvtxMode on -CaptureMode nvtx -NvtxCapture double_ma_opt_total -DynamicStrategyMode off -CudaGraphsMode off -OutputPrefix target/nsight/double_crossover_phase2_xlarge_full17_bounded
```

Exit condition:

- a findings note exists showing where host waits occur and which NVTX-visible ranges are GPU-dominant vs host-only

## Phase 3: Nsight Compute Deep Dive

Objective: attribute the GPU-dominant portion to specific kernels and decide whether the next optimization target is the backtest body, reducers, or gathers.

Tasks:

1. Start with the dominant exact backtest kernel family.
2. Then collect targeted runs for:
   - row-score and row-select kernels
   - pair-select and pair-gather kernels
   - gather / pack helpers if they become visible
3. Record at minimum:
   - registers per thread
   - theoretical and achieved occupancy
   - memory vs compute throughput
   - dominant stall reasons
   - branch efficiency or predication symptoms when relevant
4. Keep contract-specific captures separate:
   - `full17`
   - `trade22`
5. Compare `large` vs `xlarge` because the dominant kernel mix may differ.

Suggested commands:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload large -RunnerMetricContract full17 -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -DynamicStrategyMode off -CudaGraphsMode off -UseKernelNameFilter:$true -KernelName "regex:^double_cross_backtest_tm_flex_f32" -OutputPrefix target/nsight/double_crossover_phase2_large_full17_backtest

powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload large -RunnerMetricContract trade22 -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -DynamicStrategyMode off -CudaGraphsMode off -UseKernelNameFilter:$true -KernelName "regex:^double_cross_backtest_tm_flex_f32" -OutputPrefix target/nsight/double_crossover_phase2_large_trade22_backtest

powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 -Target runner -RunnerStrategy double_ma -RunnerWorkload xlarge -RunnerMetricContract full17 -RunnerWarmupIterations 2 -RunnerIterations 1 -RunnerHeatmapBins 0 -DynamicStrategyMode off -CudaGraphsMode off -UseKernelNameFilter:$true -KernelName "regex:^(score_metric_rows_f32|select_topk_metric_rows_f32|select_topk_metric_rows_f32_v2|gather_topk_metric_rows_f32|gather_topk_pairs_tm_f32|select_topk_scored_candidates_tm_f32|gather_topk_scored_scalar_pairs_tm_f32)$" -OutputPrefix target/nsight/double_crossover_phase2_xlarge_reducers
```

Exit condition:

- a kernel-attribution note exists with clear evidence about whether the next optimization target is:
  - backtest core,
  - row reducer path,
  - pair reducer path,
  - gather / pack output handling,
  - or route-level materialization policy

## Phase 4: Controlled Intervention Matrix

Objective: run only evidence-backed interventions and keep the experiment matrix small enough to interpret.

Interventions should be chosen from the profiler evidence, not precommitted. Candidate classes:

### If the backtest body still dominates

Investigate:

- fast-warp-dedup routing and thresholds
- warp-uniform load/broadcast opportunities
- invalid-row materialization and compact-output behavior
- exact-contract-specific control-flow trimming that does not reopen Phase 1 issues
- backtest-body address-generation or store-shape cleanup only if Nsight Compute still points there

### If row reducers become material

Investigate:

- true score-once row flow on admitted narrow exact surfaces
- constrained score-generation support instead of raw-row `v2` selection
- dense-only gather fast paths only if they show up as real cost

### If pair reducers or gathers become material

Investigate:

- routes that only need `(fast, slow, score)` and can stay on scalar-score gather longer
- compact-valid-pair output heuristics
- fake-scored selector cleanup vs true score-array pair selection
- distribution-aware fast-warp-dedup enablement

### If host wait still dominates but physical memops remain tiny

Investigate:

- deeper device-resident pipeline shape
- synchronization boundaries
- result materialization policy

Explicit non-goal for this phase:

- do not reopen broad "maybe host orchestration is the bottleneck" work unless Nsight contradicts the March findings

Preferred architecture lane if the profiles justify it:

- keep the narrow exact search stage on compact scored candidates rather than full metric rows,
- keep device-side finalist descriptors or replay inputs compact as long as possible,
- preserve exact replay as the authority for final `full17` / `trade22` rows,
- only broaden that architecture after it proves itself on the admitted narrow exact surfaces.

Exit condition:

- each accepted code change can be tied to one profiler-backed hypothesis and one retained benchmark or perf-gate result

## Phase 4A: Compact Exact Search-Stage Architecture

Objective: if profiling points at reduction, finalist materialization, or replay setup overhead, redesign the narrow exact path so search stays compact while exact replay remains authoritative.

Key questions:

1. What is the minimum compact payload that still preserves exact replay semantics?
2. Should the payload be:
   - `(score, pair_idx)`
   - `(score, pair_idx, feasibility_bit)`
   - `(score, pair_idx, tiny replay metadata)`
   - or a compact replay-descriptor struct
3. Can finalist identities stay on device until replay launch is fully parameterized?
4. Can replay consume compact device-side finalist identities directly?

Required guardrails:

1. final `full17` / `trade22` rows stay replay-authoritative
2. ordering and tie behavior remain deterministic
3. host finalist reconstruction is reduced only with parity-safe evidence

Exit condition:

- either a narrow compact exact path is implemented and verified on admitted surfaces, or the repo has enough evidence to defer it cleanly

## Phase 4B: Route Broadening and Constrained Surfaces

Objective: after a retained compact or score-once path exists, decide where it should and should not be adopted.

Required evaluations:

1. shortlist-slice exact reruns
2. `trade22` tail-metric correctness after replay
3. constrained exact surfaces and whether `feasible_out` is worth adding
4. whether heatmap should adopt the new path
5. whether include-all and export-all should adopt the new path
6. whether shortlist union and confidence rerank should adopt the new path

Guardrail:

- do not broaden the route into these surfaces just because the narrow exact surface improved

Exit condition:

- each downstream route is either adopted or explicitly deferred with evidence

## Phase 4C: Runtime Architecture Follow-ons

Objective: keep the next architecture lanes visible without letting them preempt the primary profiling-backed work.

Candidate follow-ons:

1. `TM`-native MA expansion beyond `sma` / `alma`
2. multi-stream or double-buffered tiling
3. stronger device-resident caches for:
   - period banks
   - TM MA tiles
   - replay-side reuse
   - shortlist replay reuse

Guardrail:

- only promote these after the first retained compact/replay architecture evidence exists, or if phase-2 profiling clearly shows they are the better next move

Exit condition:

- these lanes are either scheduled as the next concrete phase or explicitly held behind stronger evidence

## Phase 5: Optional CUPTI SASS-Metric Follow-up

Objective: only if Nsight Systems and Nsight Compute still leave ambiguous attribution inside control-heavy kernels, add a more research-oriented static+dynamic attribution layer.

Feasibility and first large-surface capture completed 2026-04-05.

Outcome:

1. A branch-local helper and wrapper script were implemented:
   - [cupti_sass_probe.rs](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/bin/cupti_sass_probe.rs)
   - [run_cupti_sass_probe.ps1](C:/Rust%20Projects/ta_desktop_demo/scripts/run_cupti_sass_probe.ps1)
2. Local inventory succeeded on `RTX 4090` / `AD102`.
3. NVIDIA's bundled `sass_metrics` sample succeeded locally.
4. The first repo-local helper attempt failed at `cuptiSassMetricsFlushData`, but the root cause was a helper-side `CUpti_SassMetricsFlushData_Params.struct_size` mismatch.
5. After correcting that bug, repo-local captures succeeded on the retained `large/full17` and `large/trade22` exact routes in [cupti_sass_metric_followup_findings_2026-04-05.md](C:/Rust%20Projects/ta_desktop_demo/CUDA%20kernel%20review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md).
6. Region aggregation against refreshed static maps shows that `large/trade22` remains dominated by the expected `m22_r3` main-loop region, while `large/full17` collapses into a later `m17` control-heavy numeric/call cluster (`0x3770-0x61a0`) rather than into entry/setup or writeback.
7. The branch is now viable for research/tooling attribution work, but it still does not change any product optimization priority.

Candidate follow-up:

1. Classify static disassembly regions into:
   - store-heavy exact-row writeback
   - address generation
   - predication / control-flow
   - score computation
   - reducer duplication
2. Use CUPTI SASS metrics or equivalent runtime instruction-region evidence to validate which static regions actually dominate the live workloads.
3. Publish the result as a research artifact, not as a prerequisite for product optimization.

Guardrail:

- do not block product optimization on this phase if Nsight already gives a clear enough answer

Exit condition:

- either:
  - the repo has a useful static+dynamic attribution note, or
  - there is enough evidence that CUPTI would not materially change the next product decisions

Recorded result:

- the branch no longer fails at feasibility, and the interpreted large exact result refines the intra-symbol `full17` / `trade22` story without promoting a new product-facing branch

## Phase 6: Verification and Closure

Objective: keep only changes that remain parity-safe and useful on the actual surfaces that motivated the work.

Required verification after each accepted intervention:

1. targeted parity tests for the touched route
2. direct-runner rerun on the motivating surface
3. `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
4. `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` when exact contract-row changes are involved
5. a narrow Nsight refresh if the retained change targeted a kernel body or reducer body

Decision rule:

- reject any change that only improves a synthetic or unsupported path while regressing the canonical row-first surfaces

## Decision Table for What to Do Next

| Evidence outcome | Next action |
|---|---|
| xlarge exact path remains overwhelmingly backtest-core dominated | keep work in the backtest body and route policy, not in reducer micro-optimizations |
| `large` exact path exposes visible row-reducer share | prototype score-once row selection on admitted exact surfaces only |
| pair gather, row gather, or finalist materialization shows up materially in Nsight Compute | prioritize compact exact search-stage redesign and only then test dense-only copy helpers |
| fast-warp-dedup helps only on selected distributions | keep or improve runtime gating rather than making it default |
| host-visible wait remains large but actual memops are tiny | focus on synchronization and materialization shape, not bandwidth folklore |
| NCU shows store-path issue reappearing only on one exact family | do a narrow static SASS refresh for that family and only then consider codegen-level investigation |

## Extensive Checklist

### A. Baseline and measurement hygiene

- [ ] Confirm no benchmark or profiler process is active.
- [ ] Record branch, commit, GPU, driver, profiler versions.
- [ ] Refresh direct-runner JSON baselines for `large` `full17`.
- [ ] Refresh direct-runner JSON baselines for `large` `trade22`.
- [ ] Refresh direct-runner JSON baselines for shortlist-heavy `xlarge` `full17`.
- [ ] Refresh direct-runner JSON baselines for shortlist-heavy `xlarge` `trade22`.

### B. Internal timing refresh

- [ ] Re-run with `VECTORBT_KERNEL_PROFILE_PIPELINE=1`.
- [ ] Re-run with `VECTORBT_KERNEL_LOG_DISPATCH=1`.
- [ ] Capture route and pipeline timing summaries.
- [ ] Record active dispatch and exact contract for every truth surface.

### C. Nsight Systems

- [ ] Run whole-process NSYS on `large` `full17`.
- [ ] Run whole-process NSYS on `large` `trade22`.
- [ ] Run whole-process NSYS on `xlarge` `full17`.
- [ ] Run whole-process NSYS on `xlarge` `trade22`.
- [ ] Capture `cuda_api_sum`, `cuda_gpu_kern_sum`, `cuda_gpu_mem_time_sum`.
- [ ] Capture `nvtx_sum` and `nvtx_gpu_proj_sum` when available.
- [ ] Record whether elevated shell was required.

### D. Nsight Compute

- [ ] Capture dominant backtest kernel on `large` `full17`.
- [ ] Capture dominant backtest kernel on `large` `trade22`.
- [ ] Capture reducer/gather kernel group on at least one xlarge surface.
- [ ] Record occupancy, regs/thread, throughput, and stall reasons.

### E. Experiment selection

- [ ] Choose one next intervention from profiler evidence.
- [ ] Keep score-first disabled unless the experiment explicitly targets it.
- [ ] Use fast-warp-dedup only as a controlled A/B unless the data clearly justifies broader use.
- [ ] Keep compact-valid-pair output behavior observable in the logs.

### F. Verification

- [ ] Re-run the motivating direct-runner surface after the change.
- [ ] Run `scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- [ ] Run `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` when relevant.
- [ ] Save the rerun outputs and profiler provenance.

### G. Research closure

- [ ] Write the phase-2 static+dynamic attribution note.
- [ ] State clearly whether the next frontier is source/layout, reducer shape, route policy, or narrow low-level tuning.
- [ ] Do not claim a scheduler/codegen failure unless profiler-backed evidence still points there after the retained source/runtime changes.

### H. Carry-forward exact-path verification

- [ ] If a compact path is retained, extend it through shortlist-slice exact reruns.
- [ ] Confirm `trade22` tail metrics still materialize correctly after replay.
- [ ] Decide whether constrained exact routes are hot enough to justify `feasible_out` or similar feasibility side outputs.
- [ ] Decide whether heatmap should adopt the retained new path.
- [ ] Decide whether include-all and export-all should adopt the retained new path.
- [ ] Decide whether union and confidence rerank stages should adopt the retained new path.

### I. Deferred but tracked architecture branches

- [ ] Decide whether one more bounded lockstep-body pass is justified by fresh profiling.
- [ ] Keep `windowed` body work deferred unless a retained benchmark surface makes it worthwhile.
- [ ] Decide whether `TM`-native MA expansion beyond `sma` / `alma` should be the next concrete phase.
- [ ] Decide whether multi-stream / double-buffered orchestration should be the next concrete phase.
- [ ] Decide whether stronger replay-side or shortlist-reuse device caches are worth the added complexity.
- [ ] Add a cubin-vs-PTX-JIT A/B only if the retained hot kernels still justify compiler-layer suspicion.

## End State

Phase 2 is successful if it produces all of the following:

1. a refreshed runtime attribution baseline for the exact `double_crossover` path,
2. profiler-backed evidence for where the next real performance headroom sits,
3. at least one retained optimization that improves the motivating live surfaces or a defensible decision not to change code,
4. a clean artifact trail supporting both future product work and a research narrative about SASS-guided source optimization.
