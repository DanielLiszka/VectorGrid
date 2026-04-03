# `double_crossover` Phase 2 External Review Response and Task Tracker

Date: 2026-04-03

## Purpose

This document converts `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md` into a repo-owned Phase 2 task tracker.

It is not a replacement for `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`.

Its job is narrower:

1. record which parts of the external review are adopted into the repo plan,
2. record which parts remain inference or conservative policy rather than proved fact,
3. turn the adopted guidance into ordered tasks and decision gates for the next Phase 2 loop.

## Relationship To Existing Docs

Use this tracker together with:

- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `SASS research/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`

The current authority model remains unchanged:

- this tracker organizes the next tasks
- the Phase 2 plan remains the main implementation and profiling roadmap
- direct runner and profiler artifacts remain evidence
- `scripts/run_gpu_perf_gate.ps1 -Mode candidate` remains the authoritative post-change performance gate

## Coverage Of The External Review

This tracker is meant to absorb the valid, action-driving parts of the external review without duplicating its full prose.

Coverage summary:

1. retained `large` exact-route conclusions are absorbed under `External Review Items Adopted Into The Repo Plan`
2. incomplete shortlist-heavy `xlarge` closure is absorbed under `External Review Items Kept As Provisional` and `Stage A`
3. the external review's branch recommendation between a bounded lockstep pass and a compact exact search-stage lane is absorbed under `Phase 2 Working Policy From This Tracker` and `Stage B`
4. score-first deferral, reducer-first caution, and rejection of broad SASS reopening are absorbed under `External Review Items Adopted Into The Repo Plan` and `Immediate Rejection Criteria`
5. the external review's request for a measurement-first decision loop is absorbed as repo policy, but is explicitly labeled here as policy rather than already-proved fact

What this tracker intentionally does not duplicate:

1. the full evidence narrative already written in `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
2. the full procedural profiling roadmap already written in `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
3. the broader research-framing ranking, except where it changes immediate product tasks

## External Review Items Adopted Into The Repo Plan

The following conclusions are accepted as sufficiently supported by the current repo evidence:

1. The April 2 SASS-guided source wave closed the strongest wrapper, `metrics_stride`, and packed-store recovery questions for the retained exact families.
2. The current measured `large/full17` and `large/trade22` exact surfaces are still dominated by the main `double_cross_backtest_tm_flex_f32_lockstep_time_*` kernels.
3. The current Phase 2 runtime bundle does not support a spill-driven story for the retained hot kernels.
4. The next product step should stay evidence-first and should not reopen blanket PTX or SASS rewriting.
5. The current score-first implementation should remain deferred.
6. The surfaced `xlarge` `gather_topk_pairs_tm_f32` result is too small by itself to justify a reducer-first rewrite as the next default move.

## External Review Items Kept As Provisional

The following statements should be treated as working policy or inference, not as repo-proved fact:

1. "Finish missing `xlarge` attribution before any new product code change."
   This is a conservative and defensible process decision, but it is still a decision rather than a fact already proved by the repo.
2. "Reducer or replay-side work is effectively ruled out on shortlist-heavy `xlarge`."
   The current surfaced bundle does not prove that. It only proves that the surfaced `gather_topk_pairs_tm_f32` capture is small.
3. "The same backtest-core dominance seen on `large` is already proved end to end on the shortlist-heavy `xlarge` truth surfaces."
   Current repo evidence makes that plausible, but the surfaced `xlarge` attribution is still incomplete.

## Additional Valid Review Points Retained As Background

The external review also made two valid points that remain important, even though they are not immediate checklist items:

1. The strongest product-facing question is no longer "more generic SASS analysis" but "which bounded intervention does the runtime evidence justify next."
2. The strongest research-facing framing remains hybrid rather than blanket hand-SASS:
   - static plus dynamic attribution remains the most credible research layer
   - SASS-guided source-refactor logic remains a strong supporting framing
   - any compact exact search-stage work should preserve replay-authoritative finalists rather than replace exact final-row authority

These points are retained here as guardrails for future writeups and implementation decisions, not as separate short-term tasks.

## Phase 2 Working Policy From This Tracker

Adopt the following policy for the next loop:

1. Treat missing shortlist-heavy `xlarge` attribution as the next blocking measurement gap.
2. Do not create a new parallel optimization plan; feed all new work back into the existing Phase 2 plan.
3. Keep the next decision bounded to one of these outcomes:
   - one more lockstep backtest-body pass,
   - compact exact search-stage architecture work with replay-authoritative finalists,
   - measurement extension if the new `xlarge` evidence is still ambiguous.
4. Keep score-first, broad reducer-first work, and broad specialization work out of scope unless new measurement evidence promotes them.

## Task Tracker

### Stage A: Complete Missing `xlarge` Attribution

- [x] Re-run whole-process Nsight Systems on `double_ma`, `alma/alma`, `xlarge`, `shortlist_slice_mode=halves`, `metric_contract=full17`.
- [x] Re-run whole-process Nsight Systems on `double_ma`, `alma/alma`, `xlarge`, `shortlist_slice_mode=halves`, `metric_contract=trade22`.
- [x] Run Nsight Compute on the shortlist-heavy `xlarge/full17` surface with a hot-backtest regex targeting `^double_cross_backtest_tm_flex_f32`.
- [x] Run Nsight Compute on the shortlist-heavy `xlarge/trade22` surface with a hot-backtest regex targeting `^double_cross_backtest_tm_flex_f32`.
- [x] Run Nsight Compute on the shortlist-heavy `xlarge/full17` surface with the reducer and row-select regex set covering `score_metric_rows_f32`, `select_topk_metric_rows_f32`, `select_topk_metric_rows_f32_v2`, `gather_topk_metric_rows_f32`, `gather_topk_pairs_tm_f32`, `select_topk_scored_candidates_tm_f32`, `gather_topk_scored_scalar_pairs_tm_f32`, and `pack_metric_rows_f32`.
- [x] Run the same reducer and row-select Nsight Compute capture on shortlist-heavy `xlarge/trade22`.
- [x] Refresh built-in pipeline and dispatch logs on both shortlist-heavy `xlarge` truth surfaces with `VECTORBT_KERNEL_PROFILE_PIPELINE=1` and `VECTORBT_KERNEL_LOG_DISPATCH=1`.
- [x] Record route diagnostics for both shortlist-heavy `xlarge` truth surfaces, including `gpu_kernel.path`, `metrics_count`, `tile_metrics_stride`, `decode_without_recompute`, `compact_valid_pair_output`, and score-first blockers.
- [x] Append the new artifact paths and provenance to the main Phase 2 plan.
- [x] Write a short surfaced-results note that compares `large` vs `xlarge` attribution rather than leaving the `xlarge` profiler outputs implicit.

### Stage B: Decision Gate After `xlarge` Attribution

- [x] Decide whether shortlist-heavy `xlarge` is still overwhelmingly dominated by the same lockstep backtest families seen on `large`.
- [x] Decide whether any reducer, selector, pack, rerank, or replay-adjacent kernels are materially large in absolute time on shortlist-heavy `xlarge`.
- [x] Decide whether the new `xlarge` hot kernels show the same primary signature as the current `large` NCU bundle: long scoreboard pressure, weak bytes-per-sector, and no spill story.
- [x] Decide whether distribution-aware specialization should stay deferred or move into a bounded follow-up experiment lane.
- [x] Record the branch decision in the tracker using exactly one of these labels:
  - `bounded_lockstep_pass`
  - `compact_exact_search_stage`
  - `measurement_extension`

### Stage B Result

Recorded branch label: `bounded_lockstep_pass`

Repo-grounded rationale:

1. Whole-process Nsight Systems on shortlist-heavy `xlarge` keeps the lockstep backtest kernels as the largest absolute hotspot.
2. The surfaced secondary `xlarge` GPU cost is the `windowed_m17` / `windowed_m22` shortlist-slice replay family, not the reducer/search-stage kernels.
3. The reducer/search-stage kernels stay microseconds in whole-process Nsight Systems on the measured truth surfaces, so `compact_exact_search_stage` is not the first promoted branch from the current bundle.
4. The `xlarge` route does have a real `final_finalize` / `host_finalize` slice, but current code shows that this includes shortlist-slice rerank and replay work. That should become the next architecture candidate only if the bounded lockstep pass stalls.

Additional repo note:

- The profiler wrapper scripts had to be corrected before the retained `xlarge` captures were trustworthy. The successful reruns now pass through `fast-ma`, `slow-ma`, `candle-shape`, and `shortlist-slice-mode`, so the stored artifacts match the intended `alma/alma`, `synth`, shortlist-heavy truth surfaces.

### Stage C: Conditional Implementation Lane 1

Only start this lane if Stage B resolves to `bounded_lockstep_pass`.

- [x] Limit code changes to `crates/app_core/src/kernels/double_crossover.cu` around `store_metrics_row_fixed_f32` and `double_cross_backtest_tm_flex_f32_body`, plus any narrow wrapper or instrumentation support needed for the retained exact lockstep families.
- [x] Keep `crates/app_core/src/gpu_backtest_kernel.rs` changes measurement-oriented unless the retained body pass requires a narrow route or payload adjustment.
- [x] Avoid reopening score-first, reducer, or broad orchestration work inside this lane.
- [x] After implementation, refresh direct-runner baselines on the four truth surfaces.
- [x] After implementation, run `scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- [x] After implementation, run `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` as supporting evidence.
- [x] After implementation, perform a narrow SASS refresh only for the touched hot kernels.

### Stage C Result

Retained change:

1. Kept a bounded lockstep-body source change in `crates/app_core/src/kernels/double_crossover.cu`:
   - add `ld_tm_row_base(...)`
   - hoist `fast_row_base` / `slow_row_base`
   - use that row-base load path only on the retained `LOCKSTEP_TIME` exact-body branch
2. No route-policy, reducer, score-first, or `gpu_backtest_kernel.rs` behavior change was required.

Same-tree direct-runner evidence:

1. Initial four-surface A/B against a reverted same-tree build:
   - `large/full17`: `69.33 ms` patched vs `71.67 ms` reverted
   - shortlist-heavy `xlarge/full17`: `381.33 ms` patched vs `409.00 ms` reverted
   - `large/trade22`: first 3-iteration pass was noisy and inconclusive
   - shortlist-heavy `xlarge/trade22`: `409.00 ms` patched vs `414.00 ms` reverted on the first pass
2. Targeted 7-iteration recheck on the ambiguous `trade22` surfaces:
   - `large/trade22`: `70.00 ms` patched vs `71.86 ms` reverted
   - shortlist-heavy `xlarge/trade22`: `390.29 ms` patched vs `387.29 ms` reverted
3. Repo interpretation:
   - `full17` improvement is clear
   - `trade22` is mixed on shortlist-heavy `xlarge`, but the swing is small enough that it did not justify dropping the change before an authoritative gate run

Authoritative gate evidence:

1. `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed and wrote `target/gpu_perf_summary.json`.
2. `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` passed and wrote `target/gpu_alma_contract_rows_perf_summary.json`.
3. The specialized ALMA contract-row gate stayed green on:
   - `auto`
   - `full17`
   - `trade22`

Post-change static refresh:

1. Narrow cuobjdump refresh captured the touched hot kernels into:
   - `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_2026-04-03.sass.txt`
   - `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_symbols_2026-04-03.txt`
2. This refresh was intentionally narrow and limited to:
   - `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
   - `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`

Decision:

1. Keep the bounded lockstep row-base change.
2. Treat Stage C as complete for this branch.
3. If another product-facing pass is needed after this retained change, the next candidate should be shortlist-slice replay and diagnostics efficiency rather than a reducer-first or broad SASS reopening.

Planned next branch:

1. The next repo-owned follow-on is now documented in `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`.
2. That follow-on is:
   - instrument shortlist-slice `final_finalize`
   - separate replay, rerank, and diagnostics costs in `crates/app_core/src/gpu_backtest_kernel.rs`
   - then choose one narrow shortlist-slice intervention before reopening `compact_exact_search_stage`

### Stage D: Conditional Implementation Lane 2

Only start this lane if Stage B resolves to `compact_exact_search_stage`.

- [ ] Scope the architecture work around `crates/app_core/src/kernels/double_crossover.cu` symbols `gather_topk_pairs_tm_f32`, `select_topk_scored_candidates_tm_f32`, `gather_topk_scored_scalar_pairs_tm_f32`, `score_metric_rows_f32`, `select_topk_metric_rows_f32`, `select_topk_metric_rows_f32_v2`, `gather_topk_metric_rows_f32`, and `pack_metric_rows_f32`.
- [ ] Keep `crates/app_core/src/gpu_backtest_kernel.rs` responsible for route integration, diagnostics, replay authority, and fallback control.
- [ ] Keep `crates/app_core/src/double_ma.rs` responsible for exact-route surface constraints such as shortlist slice, shortlist confidence, shortlist union, and metric-contract gating.
- [ ] Preserve replay-authoritative final `full17` and `trade22` rows.
- [ ] Preserve the current row-first exact route as a fallback until parity and performance are both proven.
- [ ] Explicitly test shortlist-slice reruns, `trade22` replay tails, and downstream row-decoding behavior before broadening the route.
- [ ] After implementation, re-run the truth surfaces and the authoritative GPU perf gate before promoting the architecture change.

### Stage E: Conditional Measurement Extension

Only start this lane if Stage B resolves to `measurement_extension`.

- [ ] Decide which unresolved ambiguity remains after the new `xlarge` bundle.
- [ ] If the ambiguity is dynamic hotspot composition, consider CUPTI or a tighter Nsight Compute slice rather than guessing from static SASS.
- [ ] If the ambiguity is workload sensitivity, add exactly one bounded specialization study rather than reopening many branches in parallel.
- [ ] If the ambiguity is route-state gating, capture one focused diagnostic pass against the relevant `gpu_backtest_kernel.rs` blockers before proposing code changes.

## Immediate Success Criteria

Treat the next measurement loop as successful only if all of the following are true:

1. Both shortlist-heavy `xlarge` truth surfaces have surfaced Nsight Systems summaries.
2. Both shortlist-heavy `xlarge` truth surfaces have surfaced hot-backtest Nsight Compute summaries.
3. Both shortlist-heavy `xlarge` truth surfaces have surfaced reducer and row-select Nsight Compute summaries.
4. The tracker records one explicit Stage B branch label.
5. The repo has a written reason for why the next step is a bounded lockstep pass, compact exact search-stage work, or further measurement.

## Immediate Rejection Criteria

Reject the next step if any of the following are true:

1. It starts product code changes before the missing shortlist-heavy `xlarge` measurements are surfaced and summarized.
2. It promotes reducer-first work only because `gather_topk_pairs_tm_f32` looks inefficient without showing meaningful absolute time.
3. It promotes score-first by ignoring the current blocker logic and current measured losses.
4. It reopens broad PTX or SASS rewriting without a newly demonstrated codegen miss on a retained hot kernel.
5. It changes `auto`, `full17`, or `trade22` semantics, exact finalist semantics, or CPU fallback behavior.

## Implementation Touch Targets If Code Work Is Greenlit

The next code pass, if any, should remain concentrated in the existing ownership surfaces:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

The next code pass should not begin by broadening into unrelated frontend, Tauri, or workspace-crate surfaces.

## Logging Requirements

When the next loop is run, record all of the following in the repo:

- exact commands used
- profiler versions used
- elevated vs unelevated launch context for Nsight
- artifact file paths
- branch or commit ID
- route diagnostics for every truth surface
- final Stage B decision label

## Update Log

### 2026-04-03

- Created this tracker as the repo-owned response to `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`.
- Adopted the external review's main prioritization points for the retained `large` exact route.
- Kept the external review's stronger shortlist-heavy `xlarge` claims provisional pending surfaced whole-process and kernel-level attribution.
- Set the next blocking work to completion of the missing shortlist-heavy `xlarge` profiling bundle and an explicit Stage B branch decision.
- Completed the missing shortlist-heavy `xlarge` Nsight Systems, Nsight Compute, and built-in pipeline passes on the intended `alma/alma`, `synth`, `shortlist_slice_mode=halves` truth surfaces.
- Recorded Stage B as `bounded_lockstep_pass`.
- Kept shortlist-slice replay and diagnostics efficiency as the next architecture follow-on if the bounded lockstep pass stalls, instead of promoting reducer-first work.
- Retained a bounded lockstep-body row-base addressing change in `crates/app_core/src/kernels/double_crossover.cu` after same-tree direct-runner A/B and authoritative GPU perf-gate validation.
- Added the narrow post-change cuobjdump refresh for the touched `lockstep_time_m17` / `lockstep_time_m22` hot symbols.
- Added `SASS research/double_crossover_phase2_stagec_bounded_lockstep_pass_log_2026-04-03.md` to record the exact Stage C commands and saved artifacts.
- Moved the next planned work into the main Phase 2 plan as a shortlist-slice replay / diagnostics follow-on instead of reopening Stage D or Stage E here.

## Exit Condition For This Tracker

This tracker is complete when one of the following has happened:

1. the missing shortlist-heavy `xlarge` attribution is surfaced, the Stage B decision is recorded, and the next implementation lane is formally chosen, or
2. the new `xlarge` evidence proves that another measurement extension is required and that extension is explicitly scoped.

At that point, push the resulting branch decision back into `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` and continue from there rather than creating another parallel plan document.

Current state:

- This exit condition is now satisfied.
- Further execution should continue in `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` under the `bounded_lockstep_pass` branch.
- The immediate next planned branch there is the shortlist-slice replay / diagnostics measurement-and-intervention loop.
