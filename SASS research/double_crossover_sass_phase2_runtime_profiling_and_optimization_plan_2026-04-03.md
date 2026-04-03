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

The Phase 2 runtime-attribution bundle is now complete for both the retained `large` surfaces and the shortlist-heavy `xlarge` truth surfaces.

Current summary docs:

- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_response_and_task_tracker_2026-04-03.md`
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
  - `target/double_crossover_phase2_xlarge_full17_pipeline_2026-04-03.log`
  - `target/double_crossover_phase2_xlarge_trade22_pipeline_2026-04-03.log`
- Nsight Systems:
  - `target/nsight/double_crossover_phase2_large_full17.nsys-rep`
  - `target/nsight/double_crossover_phase2_large_full17.provenance.json`
  - `target/nsight/double_crossover_phase2_large_full17_stats.txt`
  - `target/nsight/double_crossover_phase2_large_trade22.nsys-rep`
  - `target/nsight/double_crossover_phase2_large_trade22.provenance.json`
  - `target/nsight/double_crossover_phase2_large_trade22_stats.txt`
  - `target/nsight/double_crossover_phase2_xlarge_full17.nsys-rep`
  - `target/nsight/double_crossover_phase2_xlarge_full17.provenance.json`
  - `target/nsight/double_crossover_phase2_xlarge_full17_stats.txt`
  - `target/nsight/double_crossover_phase2_xlarge_trade22.nsys-rep`
  - `target/nsight/double_crossover_phase2_xlarge_trade22.provenance.json`
  - `target/nsight/double_crossover_phase2_xlarge_trade22_stats.txt`
- Nsight Compute:
  - `target/nsight/double_crossover_phase2_large_full17_backtest.ncu-rep`
  - `target/nsight/double_crossover_phase2_large_full17_backtest.provenance.json`
  - `target/nsight/double_crossover_phase2_large_full17_backtest_raw.csv`
  - `target/nsight/double_crossover_phase2_large_trade22_backtest.ncu-rep`
  - `target/nsight/double_crossover_phase2_large_trade22_backtest.provenance.json`
  - `target/nsight/double_crossover_phase2_large_trade22_backtest_raw.csv`
  - `target/nsight/double_crossover_phase2_xlarge_full17_backtest.ncu-rep`
  - `target/nsight/double_crossover_phase2_xlarge_full17_backtest.provenance.json`
  - `target/nsight/double_crossover_phase2_xlarge_full17_backtest_raw.csv`
  - `target/nsight/double_crossover_phase2_xlarge_trade22_backtest.ncu-rep`
  - `target/nsight/double_crossover_phase2_xlarge_trade22_backtest.provenance.json`
  - `target/nsight/double_crossover_phase2_xlarge_trade22_backtest_raw.csv`
  - `target/nsight/double_crossover_phase2_xlarge_full17_reducers.ncu-rep`
  - `target/nsight/double_crossover_phase2_xlarge_full17_reducers.provenance.json`
  - `target/nsight/double_crossover_phase2_xlarge_full17_reducers_raw.csv`
  - `target/nsight/double_crossover_phase2_xlarge_trade22_reducers.ncu-rep`
  - `target/nsight/double_crossover_phase2_xlarge_trade22_reducers.provenance.json`
  - `target/nsight/double_crossover_phase2_xlarge_trade22_reducers_raw.csv`
  - `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.json`
  - `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`

Current measured conclusions:

1. the live exact `large` surfaces are still dominated by the main `lockstep_time_m17` / `lockstep_time_m22` backtest kernels
2. the shortlist-heavy `xlarge` surfaces remain backtest-family dominated, with the same lockstep families as the largest absolute hotspot
3. the surfaced secondary `xlarge` GPU cost is the `windowed_m17` / `windowed_m22` shortlist-slice replay family, not the reducer/search-stage kernels
4. fresh Nsight Compute still does not show a spill-driven story and still points primarily at long-scoreboard and access-shape pressure on the retained hot kernels
5. `trade22` continues to preserve materially better store width than `full17`, but both remain below ideal sector utilization
6. reducer/search-stage kernels remain too small in whole-process time on the measured truth surfaces to justify a reducer-first rewrite
7. the shortlist-heavy `xlarge` route has a real `final_finalize` / `host_finalize` slice, and the current code shows that this includes shortlist-slice rerank and replay work

Stage B decision from the completed bundle:

- branch label: `bounded_lockstep_pass`
- immediate implication: keep the next product-facing code pass inside the retained lockstep body first
- follow-on implication: if that pass stalls, the next architecture target should be shortlist-slice replay and diagnostics efficiency rather than reducer-first work

Stage C retained change from the chosen branch:

- retained change: hoist lockstep `fast_row_base` / `slow_row_base` addressing and use a narrow `ld_tm_row_base(...)` helper inside `double_cross_backtest_tm_flex_f32_body`
- retained ownership surface: `crates/app_core/src/kernels/double_crossover.cu`
- direct-runner A/B outcome: clear win on `full17`, mixed but non-blocking on shortlist-heavy `trade22`
- authoritative gate outcome: `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed and `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` passed
- post-change static refresh: `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_2026-04-03.sass.txt`

Immediate post-Stage C follow-on:

- next branch: shortlist-slice replay and diagnostics efficiency, not `compact_exact_search_stage`
- primary ownership surface: `crates/app_core/src/gpu_backtest_kernel.rs`
- primary rationale:
  - shortlist-heavy `xlarge` still shows a real `final_finalize` / `host_finalize` slice
  - current source shows that this slice includes shortlist rerank and replay work
  - Stage C already retained one more bounded lockstep-body win, so the next evidence-backed question is whether shortlist-slice finalization can be reduced before any larger architecture rewrite
- first requirement: instrument `final_finalize` more finely before changing replay or rerank behavior

Execution outcome:

- completed instrumentation landed in:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- initial shortlist-heavy reruns showed the replay-dominant problem that justified the replay-side follow-on:
  - `full17`: `297.40 ms` with `none`, `379.20 ms` with `halves`, `986.80 ms` with serialized `quartiles`
  - `trade22`: `319.60 ms` with `none`, `396.20 ms` with `halves`, `599.00 ms` with serialized `quartiles`
- the initial runner-visible shortlist sub-buckets showed replay dominating the surfaced shortlist cost:
  - `full17` `halves`: shortlist slice `60.40 ms`, replay `60.40 ms`
  - `trade22` `halves`: shortlist slice `58.40 ms`, replay `58.40 ms`
- corrected pipeline-profile captures showed record assembly, sort, and diagnostics remained trace-level compared with replay
- one focused post-instrumentation Nsight Systems rerun on `xlarge/full17/halves` confirms the replay link:
  - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`
  - `51.595 ms` total across `6` instances
  - `14.3%` of traced GPU kernel time in `target/nsight/double_crossover_phase2_shortlist_full17_halves_post_instrumentation_stats.txt`
- targeted replay-kernel follow-on work is now complete:
  - targeted Nsight Compute captures exist for `windowed_m17` and `windowed_m22` on the shortlist-heavy `halves` surface
  - both kernels currently profile as tiny launches with block `(64, 1, 1)`, grid `(1, 1, 1)`, achieved occupancy about `2.78%`, and only `1` to `2` achieved active warps per SM
  - live dispatch logs showed the original `halves` route executing `6` replay launches per request: `2x50` shortlist rows and `4x8` diagnostic rows
  - the `4x8` diagnostic tail is explained by the current `halves` path still computing quartiles diagnostics for the diagnostic subset
- retained bounded replay-batching prototype:
  - `crates/app_core/src/gpu_backtest_kernel.rs` now batches shortlist replay rows across slice ranges before the `windowed` launch in `eval_double_ma_batch_gpu_kernel_impl(...)`
  - `crates/app_core/src/kernels/double_crossover.cu` now accepts optional per-row replay ranges on the `windowed` wrappers through a launch-time dispatch helper that keeps non-windowed hot kernels on the old compile-time path
- retained post-change reruns on shortlist-heavy `xlarge`, `alma/alma`, `synth` now show:
  - `full17`: `296.00 ms` with `none`, `342.80 ms` with `halves`, `349.00 ms` with `quartiles`
  - `trade22`: `308.20 ms` with `none`, `359.00 ms` with `halves`, `365.00 ms` with `quartiles`
- the retained runner-visible shortlist sub-buckets collapsed to about `28 ms` replay on both contracts and both replay modes
- live dispatch logs for the retained implementation confirm the new launch shape:
  - `batched_windowed_ranges=true range_count=2 launch_pairs=100`
  - `batched_windowed_ranges=true range_count=4 launch_pairs=32`
- verification after retaining the replay-batching prototype passed:
  - targeted replay-path tests for range-major ordering and `trade22` tail-metric preservation
  - `scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
- branch decision: keep the timing instrumentation and the retained replay-batching prototype
- follow-on implication: if more work is justified here, it is now a larger replay-side architecture / dataflow question beyond the first batching step rather than a local host-side cleanup or another kernel-local `windowed` pass

Stage C artifact additions:

- `target/double_crossover_stagec_large_full17_rowbase_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_trade22_rowbase_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_full17_rowbase_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_trade22_rowbase_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_full17_rowbase_reverted_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_trade22_rowbase_reverted_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_full17_rowbase_reverted_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_trade22_rowbase_reverted_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_trade22_rowbase_patchcheck_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_trade22_rowbase_revertcheck_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_trade22_rowbase_patchcheck_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_trade22_rowbase_revertcheck_2026-04-03.jsonl`
- `target/gpu_perf_summary.json`
- `target/gpu_alma_contract_rows_perf_summary.json`
- `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_2026-04-03.sass.txt`
- `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_symbols_2026-04-03.txt`
- `SASS research/double_crossover_phase2_stagec_bounded_lockstep_pass_log_2026-04-03.md`

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
  - optional CUPTI SASS metrics follow-up if the first two profiler layers leave unresolved attribution gaps

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
4. Current profiling support is Nsight-based; there is no existing first-class CUPTI helper script in the repo today.

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

## Immediate Post-Stage C Branch: Shortlist-Slice Replay And Diagnostics Efficiency

Objective: now that one more bounded lockstep-body pass has been retained, isolate and reduce the shortlist-slice `final_finalize` cost on shortlist-heavy `xlarge` without changing exact finalist authority.

Primary ownership surfaces:

1. `crates/app_core/src/gpu_backtest_kernel.rs`
2. shortlist-row materialization and host decode around `materialize_shortlist_rows`
3. shortlist replay and rerank flow around:
   - `rerank_shortlist_by_slice_robustness`
   - `compute_shortlist_slice_mode_records`
   - `host_finalize_shortlist_slice_ms`

Primary truth surfaces:

1. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=full17`, `shortlist_slice_mode=halves`
2. `double_ma`, `alma/alma`, `xlarge`, `candle_shape=synth`, `metric_contract=trade22`, `shortlist_slice_mode=halves`

Measurement-first sequence:

1. Add finer host-finalize timers around:
   - shortlist-row materialization
   - replay batch preparation
   - replay evaluation
   - slice-record computation
   - rerank and sorting
   - diagnostics assembly
2. Surface those timers in runner JSON and pipeline-style logs so they can be compared directly against `host_finalize_shortlist_slice_ms`.
3. Re-run the shortlist-heavy `xlarge` truth surfaces with:
   - `shortlist_slice_mode=none`
   - `shortlist_slice_mode=halves`
   - `shortlist_slice_mode=quartiles` if the extra pass remains cheap enough to interpret
4. Decide whether the dominant slice cost is:
   - row materialization
   - replay evaluation
   - sorting / rerank logic
   - diagnostics-only work
5. Only then choose one narrow intervention in `gpu_backtest_kernel.rs`.

Execution result:

1. The dominant surfaced slice cost is replay evaluation.
2. Row materialization, record assembly, sort, and diagnostics are too small to justify a retained local optimization.
3. A bounded replay-batching prototype around the replay `ranges.iter().enumerate()` loop was then implemented and retained while preserving exact `full17` / `trade22` row authority and current shortlist diagnostics semantics.
4. Post-implementation shortlist-heavy reruns show the retained replay fan-out reduction is material:
   - `full17`: `296.00 ms` with `none`, `342.80 ms` with `halves`, `349.00 ms` with `quartiles`
   - `trade22`: `308.20 ms` with `none`, `359.00 ms` with `halves`, `365.00 ms` with `quartiles`
5. The runner-visible shortlist bucket collapsed to about `28 ms` replay on both retained contracts and both replay modes, so `quartiles` remains diagnostic-only replay expansion but no longer creates the prior runtime blow-up.
6. Verification after retaining the prototype passed:
   - targeted replay-path tests for range-major ordering and `trade22` tail-metric preservation
   - `scripts/run_gpu_perf_gate.ps1 -Mode candidate`
   - `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`

Candidate interventions once attribution is clear:

1. Avoid materializing more shortlist rows than the active rerank or diagnostic limits actually need.
2. Separate diagnostics-only work from ranking-critical work so product ranking does not always pay for full diagnostics expansion.
3. Reuse replay-side data or replay descriptors when the current path rebuilds equivalent shortlist inputs, including multi-range batching on the shortlist replay path.
4. Reduce clone, sort, or duplicate slice-record work inside `rerank_shortlist_by_slice_robustness` and `compute_shortlist_slice_mode_records`.

Guardrails:

1. keep exact `full17` / `trade22` result authority unchanged
2. do not broaden into compact exact search-stage work until shortlist-slice attribution says this local branch is exhausted
3. do not treat diagnostics-only wins as product wins unless the live shortlist route also improves

Exit condition:

- the repo has a profiler-backed shortlist-slice attribution note and either:
  - one retained `gpu_backtest_kernel.rs` improvement, or
  - a clean deferral to the compact exact search-stage architecture lane

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

This is optional because the repo does not yet appear to have a CUPTI SASS-metrics integration surface.

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
| retained lockstep pass lands but shortlist-slice `final_finalize` remains material | instrument and optimize shortlist-slice replay / diagnostics in `gpu_backtest_kernel.rs` before attempting compact exact search-stage work |
| `large` exact path exposes visible row-reducer share | prototype score-once row selection on admitted exact surfaces only |
| pair gather, row gather, or finalist materialization shows up materially in Nsight Compute | prioritize compact exact search-stage redesign and only then test dense-only copy helpers |
| fast-warp-dedup helps only on selected distributions | keep or improve runtime gating rather than making it default |
| host-visible wait remains large but actual memops are tiny | focus on synchronization and materialization shape, not bandwidth folklore |
| NCU shows store-path issue reappearing only on one exact family | do a narrow static SASS refresh for that family and only then consider codegen-level investigation |

## Extensive Checklist

### A. Baseline and measurement hygiene

- [x] Confirm no benchmark or profiler process is active.
- [x] Record branch, commit, GPU, driver, profiler versions.
- [x] Refresh direct-runner JSON baselines for `large` `full17`.
- [x] Refresh direct-runner JSON baselines for `large` `trade22`.
- [x] Refresh direct-runner JSON baselines for shortlist-heavy `xlarge` `full17`.
- [x] Refresh direct-runner JSON baselines for shortlist-heavy `xlarge` `trade22`.

### B. Internal timing refresh

- [x] Re-run with `VECTORBT_KERNEL_PROFILE_PIPELINE=1`.
- [x] Re-run with `VECTORBT_KERNEL_LOG_DISPATCH=1`.
- [x] Capture route and pipeline timing summaries.
- [x] Record active dispatch and exact contract for every truth surface.

### C. Nsight Systems

- [x] Run whole-process NSYS on `large` `full17`.
- [x] Run whole-process NSYS on `large` `trade22`.
- [x] Run whole-process NSYS on `xlarge` `full17`.
- [x] Run whole-process NSYS on `xlarge` `trade22`.
- [x] Capture `cuda_api_sum`, `cuda_gpu_kern_sum`, `cuda_gpu_mem_time_sum`.
- [x] Capture `nvtx_sum` and `nvtx_gpu_proj_sum` when available.
- [x] Record whether elevated shell was required.

### D. Nsight Compute

- [x] Capture dominant backtest kernel on `large` `full17`.
- [x] Capture dominant backtest kernel on `large` `trade22`.
- [x] Capture reducer/gather kernel group on at least one xlarge surface.
- [x] Record occupancy, regs/thread, throughput, and stall reasons.

### E. Experiment selection

- [x] Choose one next intervention from profiler evidence.
- [x] Keep score-first disabled unless the experiment explicitly targets it.
- [x] Use fast-warp-dedup only as a controlled A/B unless the data clearly justifies broader use.
- [x] Keep compact-valid-pair output behavior observable in the logs.

### F. Verification

- [x] Re-run the motivating direct-runner surface after the change.
- [x] Run `scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- [x] Run `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` when relevant.
- [x] Save the rerun outputs and profiler provenance.

### G. Research closure

- [x] Write the phase-2 static+dynamic attribution note.
- [x] State clearly whether the next frontier is source/layout, reducer shape, route policy, or narrow low-level tuning.
- [x] Do not claim a scheduler/codegen failure unless profiler-backed evidence still points there after the retained source/runtime changes.

### H. Immediate shortlist-slice replay follow-on

- [x] Add finer host-finalize timers around shortlist-row materialization, replay preparation, replay evaluation, slice-record computation, rerank/sort, and diagnostics assembly in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [x] Surface the new sub-timings in runner JSON output and the built-in pipeline/logging surfaces next to `host_finalize_shortlist_slice_ms`.
- [x] Re-run shortlist-heavy `xlarge` `full17` with `shortlist_slice_mode=none` and `shortlist_slice_mode=halves`.
- [x] Re-run shortlist-heavy `xlarge` `trade22` with `shortlist_slice_mode=none` and `shortlist_slice_mode=halves`.
- [x] Add `quartiles` reruns only if the additional timing surface remains interpretable and does not create excessive diagnostic-only noise.
- [x] Capture one focused post-instrumentation Nsight Systems rerun on the shortlist-heavy `xlarge` truth surface so the new host-side breakdown can be correlated with `final_finalize`.
- [x] Write a shortlist-slice attribution note that states whether the dominant cost is row materialization, replay evaluation, rerank/sort, or diagnostics-only work.
- [x] Resolve whether one narrow shortlist-slice intervention in `crates/app_core/src/gpu_backtest_kernel.rs` is justified before reopening `compact_exact_search_stage`.
- [x] Re-run direct-runner truth surfaces and both GPU perf gates after any retained shortlist-slice change.

### I. Targeted replay-kernel study

- [x] Capture targeted Nsight Compute on `double_cross_backtest_tm_flex_f32_windowed_m17*` for shortlist-heavy `xlarge/full17/halves`.
- [x] Capture targeted Nsight Compute on `double_cross_backtest_tm_flex_f32_windowed_m22*` for shortlist-heavy `xlarge/trade22/halves`.
- [x] Export importable per-kernel details for both replay captures.
- [x] Correlate the profiler captures with live dispatch logs from `VECTORBT_KERNEL_LOG_DISPATCH=1`.
- [x] Determine whether the retained replay cost is still a kernel-local tuning problem or a replay-shape / batching problem.
- [x] Record the outcome in `SASS research/double_crossover_phase2_windowed_replay_kernel_study_2026-04-03.md`.
- [x] Design and retain exactly one bounded replay-batching prototype around the replay `ranges.iter().enumerate()` launch loop in `eval_double_ma_batch_gpu_kernel_impl(...)` that preserves shortlist diagnostics semantics and exact `full17` / `trade22` row authority.
- [x] Re-run shortlist-heavy `xlarge` `none` / `halves` / `quartiles` truth surfaces on both contracts after retaining the replay-batching prototype.
- [x] Confirm live dispatch logs collapse the replay fan-out to one launch per replay mode on the retained path.
- [x] Re-run both authoritative GPU perf gates after retaining the replay-batching prototype.
- [x] Record the retained implementation in `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`.
- [x] Do not treat diagnostic replay elision as valid unless product requirements explicitly allow a diagnostics contract change.

### J. Carry-forward exact-path verification

- [ ] If a compact path is retained, extend it through shortlist-slice exact reruns.
- [ ] Confirm `trade22` tail metrics still materialize correctly after replay.
- [ ] Decide whether constrained exact routes are hot enough to justify `feasible_out` or similar feasibility side outputs.
- [ ] Decide whether heatmap should adopt the retained new path.
- [ ] Decide whether include-all and export-all should adopt the retained new path.
- [ ] Decide whether union and confidence rerank stages should adopt the retained new path.

### K. Deferred but tracked architecture branches

- [x] Decide whether one more bounded lockstep-body pass is justified by fresh profiling.
- [x] Keep `windowed` body work deferred unless a retained benchmark surface makes it worthwhile.
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
