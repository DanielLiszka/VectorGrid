# `double_crossover_phase2_external_review_findings_2026-04-03.md`

## Date

2026-04-03

## Purpose

This memo resolves the next-step question for the `ta_desktop_demo` repo on the active `double_crossover` / `double_ma` exact `GpuKernel` path after the April 2 static-contract pass and the April 3 Phase 2 runtime bundle.

Fixed constraints:

- keep CPU fallback
- keep `auto` / `full17` / `trade22` product semantics unchanged
- keep exact finalist semantics authoritative
- do not treat blanket PTX/SASS rewriting as the next phase

This memo distinguishes direct evidence from the uploaded artifacts from explicit inference.

## Current evidence summary

### Direct evidence

- The April 2 wave already closed the strongest static-contract issues around exact-wrapper specialization, `metrics_stride`, and packed-store recovery. The handoff explicitly says Phase 1 closed those questions, and the targeted store-count artifact confirms that `m17`, `lockstep_time_m17`, and `windowed_m17` moved from scalar-only stores to mixed `STG.E.64` / `STG.E.128`, while `m22`, `lockstep_time_m22`, and `windowed_m22` share the recovered packed-store profile.  
  **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 14-18, 24-30); `double_crossover_targeted_store_counts_2026-04-02.txt` (lines 1-15).

- The current measured truth surfaces remain on the canonical row-first exact route. The April 3 runtime findings record `74.8 ms` for `large/full17`, `77.0 ms` for `large/trade22`, `452.8 ms` for shortlist-heavy `xlarge/full17`, and `441.8 ms` for shortlist-heavy `xlarge/trade22`. Route state on all four surfaces remains `row_first_global_topk`, `compact_valid_pair_output=true`, and `global_topk_mode=true`, with score-first disabled.  
  **Evidence:** `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 52-66).

- On the measured `large` exact surfaces, the current route is still overwhelmingly backtest-body bound. `tile_score` remains about `95.5%` to `96.8%` of total pipeline time on `large/full17` and `95.7%` to `96.6%` on `large/trade22`. Nsight Systems then shows `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm` and `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm` taking about `95.5%` and `95.3%` of traced GPU kernel time, while `gather_topk_pairs_tm_f32` and `pack_metric_rows_f32` are only microseconds.  
  **Evidence:** `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 68-123); `double_crossover_phase2_large_full17_stats.txt`; `double_crossover_phase2_large_trade22_stats.txt`.

- The Nsight Compute captures do not support a spill-driven story. The hot `large/full17` kernel averages `68.887 ms`, `55` registers/thread, `29.66%` active warps, `5.67 / 32 B` global-store bytes/sector, and no local/shared spilling. The hot `large/trade22` kernel averages `71.760 ms`, `64` registers/thread, `29.64%` active warps, `10.35 / 32 B` global-store bytes/sector, and no local/shared spilling. Both still show long-scoreboard pressure and weak sector efficiency.  
  **Evidence:** `double_crossover_phase2_ncu_summary_2026-04-03.txt`; `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 125-179).

- The only surfaced `xlarge` reducer-side NCU result is `gather_topk_pairs_tm_f32` at `3.029 ms`, about `0.7%` of the `452.8 ms` `xlarge/full17` wall time. That directly weakens pair-gather-first as the next product target.  
  **Evidence:** `double_crossover_phase2_ncu_summary_2026-04-03.txt`; `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 181-207).

- Current route-selection constraints are broader than the runtime note alone suggests. In `crates/app_core/src/gpu_backtest_kernel.rs`, `score_first_topk_blockers` and `should_use_score_first_topk_mode` block score-first not only on env state but also on score mode, extra shortlist heads, confidence mode, union mode, shortlist slicing, non-full-row metric contracts, constraints, eval ranges, precomputed MA paths, and nonzero heatmap bins (`gpu_backtest_kernel.rs:4247-4338`). `global_topk_mode` and route diagnostics are built separately in `build_gpu_kernel_route_diagnostics` (`gpu_backtest_kernel.rs:4344-4499`).

- `compact_valid_pair_output` is already live behavior, not an unused next-step toggle. `should_use_compact_valid_pair_output(...)` currently returns `true` unconditionally (`gpu_backtest_kernel.rs:5668-5675`).

- Shortlist confidence, union, and slice are exact-route features with backend and expansion constraints in `crates/app_core/src/double_ma.rs`. They currently require the exact `GpuKernel` route and reject multi-slice expanded runs (`double_ma.rs:6779-6865`).

- The current full-row exact route decodes current product metrics without replay by default. `can_decode_current_gpu_product_metrics_without_recompute(metrics_count)` returns `true` for the current product row widths (`gpu_backtest_kernel.rs:5663-5665`). The explicit full-GPU finalist recompute path exists (`gpu_backtest_kernel.rs:11203-11415`), but it is only taken when the current payload cannot directly decode current product metrics. The downstream shortlist rerank path only materializes separate shortlist rows when needed (`gpu_backtest_kernel.rs:15059-15420`).

- The relevant current code surfaces remain concentrated in:
  - `crates/app_core/src/kernels/double_crossover.cu`
    - `store_metrics_row_fixed_f32` (`3485`)
    - `double_cross_backtest_tm_flex_f32_body` (`3593`)
    - `gather_topk_pairs_tm_f32` (`8425`)
    - `select_topk_scored_pairs_tm_f32` (`8492`)
    - `select_topk_scored_pairs_tm_f32_v2` (`8567`)
    - `select_topk_scored_candidates_tm_f32` (`8670`)
    - `gather_topk_scored_scalar_pairs_tm_f32` (`8771`)
    - `score_metric_rows_f32` (`8847`)
    - `select_topk_metric_rows_f32` (`9015`)
    - `select_topk_metric_rows_f32_v2` (`9221`)
    - `gather_topk_metric_rows_f32` (`9556`)
    - `pack_metric_rows_f32` (`9610`)
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

### Inference boundary

- The April 3 plan explicitly called for whole-process Nsight Systems on both shortlist-heavy `xlarge` surfaces and for xlarge-vs-large NCU comparison, but the uploaded findings note only surfaces `large` Nsight Systems summaries plus one `xlarge/full17` gather capture. The repo therefore has decisive current attribution for `large`, but only partial surfaced attribution for the actual shortlist-heavy `xlarge` product route.  
  **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 328-399, 596-620); `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 95-123, 181-207).

## What the Phase 2 bundle proves

1. **The current `large` exact route is still dominated by the main lockstep backtest kernels.**  
   This is direct evidence. The pipeline profile, Nsight Systems summaries, and Nsight Compute captures all point to the same hotspot family.  
   **Evidence:** `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 68-179).

2. **Reducer-first or gather-first is not the best-supported immediate product move.**  
   The surfaced xlarge reducer result is too small in absolute time to justify leading with pair-gather work. This does not prove reducers are irrelevant everywhere, but it does weaken them as the first product target.  
   **Evidence:** `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 181-207).

3. **The current score-first implementation should not be promoted.**  
   The narrow exact score-first branch is real and functional, but it lost badly to row-first on admitted exact surfaces, and the March architecture note also records a rejected packed global-head extension. This is direct evidence against the current implementation family as a shippable next move.  
   **Evidence:** `double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md` (lines 38-44, 234-257, 291-305); `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 648-669).

4. **April 2 store-width recovery landed, but it did not eliminate the remaining hot-kernel inefficiency story.**  
   The store mix changed materially, and `trade22` now shows materially better global-store bytes/sector than `full17`, but both hot kernels still remain well below ideal sector utilization.  
   **Evidence:** `double_crossover_targeted_store_counts_2026-04-02.txt` (lines 1-15); `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 125-179).

5. **The next decision is no longer “more wrapper cleanup” or “generic launch tuning.”**  
   The Phase 2 plan explicitly rejects blanket hand-SASS as the current mode and frames the remaining choice as bounded body-local work versus a deeper search-stage/runtime architecture change.  
   **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 16-20, 69-81, 99-105); `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 28-32, 71-76).

## What remains ambiguous

- **`xlarge` end-to-end dominance is still only partially surfaced.**  
  The plan expected whole-process `xlarge` Nsight Systems attribution for both contracts. The uploaded findings note does not surface those summaries. The current claim that shortlist-heavy `xlarge` is still dominated end-to-end by the same backtest core is therefore still partly an inference from prior work plus the April 3 `large` results.  
  **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 337-367, 610-611); `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 95-123, 181-207).

- **“Pair gather is small” is not the same claim as “all xlarge reducer/materialization work is small.”**  
  The surfaced xlarge capture only reports `gather_topk_pairs_tm_f32`. It does not close the broader xlarge row-select / reducer / shortlist materialization stack.  
  **Evidence:** `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 181-207); `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 398-408).

- **Exact replay / finalist materialization matters more as an architecture guardrail than as a currently measured dominant cost on the live full-row truth surfaces.**  
  Current full-row `full17` / `trade22` surfaces decode current product metrics without replay, and slice/confidence/union rerank only materialize separate shortlist rows when needed. Any future compact search-stage design still has to preserve replay-authoritative final rows and downstream rerank semantics, but that is a contract requirement for future architecture work, not a directly measured dominant cost on the current full-row surfaces.  
  **Evidence:** `gpu_backtest_kernel.rs:5663-5665`, `gpu_backtest_kernel.rs:11203-11415`, `gpu_backtest_kernel.rs:15059-15420`.

- **Score-first design limitations are not the same as score-first implementation limitations.**  
  Current code documents that the experimental score-first path still relies on metric-scored pair selection to produce compact per-tile candidates, and the April 2 validation note separately documents that `select_topk_scored_pairs_tm_f32*` are not truly pre-scored selectors. The measured loss is therefore direct evidence against the current implementation, not definitive evidence against the broader compact scored-candidate architecture.  
  **Evidence:** `gpu_backtest_kernel.rs:12747-12755`; `double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md` (lines 234-257, 291-305).

- **Distribution-aware specialization remains unresolved.**  
  The Phase 2 plan keeps fast-warp-dedup and related specialization as a conditional lane, and the runtime findings note explicitly says the current bundle does not prove that distribution-aware specialization is unimportant.  
  **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 91, 159-169, 416-441, 581-584); `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 220-227).

## Recommended next steps, ranked

1. **Complete the missing shortlist-heavy `xlarge` attribution bundle before any new product code change.**  
   This is the most defensible immediate move. It is measurement-first, not open-ended. It targets the actual product truth surfaces that still matter most and resolves the main remaining uncertainty in the current artifact set.  
   **Why first:** the current uploaded summaries are decisive for `large`, but not yet fully surfaced for `xlarge`.  
   **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 201-212, 239-279, 337-399, 596-620); `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 52-66, 181-207).

2. **Conditional next code step: one more bounded lockstep backtest-body pass.**  
   This becomes the best code intervention **if** completed `xlarge` whole-process attribution still resolves to the same backtest-core dominance seen on `large`. The touch set should stay narrow: `crates/app_core/src/kernels/double_crossover.cu` around `store_metrics_row_fixed_f32` (`3485`), `double_cross_backtest_tm_flex_f32_body` (`3593`), and the emitted hot lockstep exact families rooted in that body; keep `crates/app_core/src/gpu_backtest_kernel.rs` changes limited to route logging or measurement continuity.  
   **Why second:** it is the highest-value bounded code path that preserves current route semantics and exact finalist authority.  
   **Evidence:** `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 136-163, 872-879, 896-898); `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 416-425); `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 213-218).

3. **Conditional alternative code path: compact exact search-stage architecture with replay-authoritative finalists.**  
   This becomes the best code intervention **if** completed `xlarge` attribution shows material row-select / materialization / stage-shape cost rather than pure backtest-core dominance. The relevant symbols are in `crates/app_core/src/kernels/double_crossover.cu`: `gather_topk_pairs_tm_f32` (`8425`), `select_topk_scored_candidates_tm_f32` (`8670`), `gather_topk_scored_scalar_pairs_tm_f32` (`8771`), `score_metric_rows_f32` (`8847`), `select_topk_metric_rows_f32` (`9015`), `select_topk_metric_rows_f32_v2` (`9221`), `gather_topk_metric_rows_f32` (`9556`), and `pack_metric_rows_f32` (`9610`), with route integration in `crates/app_core/src/gpu_backtest_kernel.rs` and exact-route semantics in `crates/app_core/src/double_ma.rs`.  
   **Why third:** it is the strongest alternative architecture lane, but it is higher-risk and should only move ahead after the missing `xlarge` attribution says the dataflow is the real next target.  
   **Evidence:** `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 350-376, 395-456, 719-724); `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 455-489).

## Preferred immediate intervention

**Recommendation: measurement-first, not code-first.**

Do not start a new product code change in `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, or `crates/app_core/src/double_ma.rs` until the missing shortlist-heavy `xlarge` attribution is completed and summarized.

The immediate work should instead do all of the following on the existing truth surfaces:

- whole-process Nsight Systems on `xlarge/full17` and `xlarge/trade22`
- xlarge hot-backtest Nsight Compute on `regex:^double_cross_backtest_tm_flex_f32`
- xlarge row-select / reducer-group Nsight Compute on the planned reducer regex set
- xlarge pipeline/dispatch refresh with `VECTORBT_KERNEL_PROFILE_PIPELINE=1` and `VECTORBT_KERNEL_LOG_DISPATCH=1`

The measurement pass should inspect these symbols first:

- hot backtest family
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`

- reducer / gather / scored-candidate family in `crates/app_core/src/kernels/double_crossover.cu`
  - `score_metric_rows_f32`
  - `select_topk_metric_rows_f32`
  - `select_topk_metric_rows_f32_v2`
  - `gather_topk_metric_rows_f32`
  - `gather_topk_pairs_tm_f32`
  - `select_topk_scored_candidates_tm_f32`
  - `gather_topk_scored_scalar_pairs_tm_f32`
  - `pack_metric_rows_f32`

- route-control and contract logic in `crates/app_core/src/gpu_backtest_kernel.rs`
  - `score_first_topk_blockers`
  - `should_use_score_first_topk_mode`
  - `build_gpu_kernel_route_diagnostics`
  - `should_use_compact_valid_pair_output`

- request-surface constraints in `crates/app_core/src/double_ma.rs`
  - shortlist confidence / union / slice exact-route checks
  - metric-contract exact-route checks

This is the shortest path to a defensible product decision on the actual shortlist-heavy surfaces.

## Rejected or deferred directions

- **Reducer-first or pair-gather-first rewrite:** deferred. The surfaced xlarge `gather_topk_pairs_tm_f32` result is too small in absolute time to justify leading with it.  
  **Evidence:** `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 181-207).

- **Broad rollout of the current score-first prototype:** rejected. The current implementation remains slower than row-first on admitted exact surfaces, and the packed global-head extension already regressed and was reverted.  
  **Evidence:** `double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md` (lines 291-305); `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 648-669).

- **Treating `compact_valid_pair_output` as the missing product lever:** rejected. It is already always enabled.  
  **Evidence:** `gpu_backtest_kernel.rs:5668-5675`.

- **Reopening the April 2 wrapper / `metrics_stride` / packed-store wave as the main task:** rejected. That work is already complete enough to have handed off into runtime attribution.  
  **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 14-20, 24-30); `double_crossover_targeted_store_counts_2026-04-02.txt` (lines 1-15).

- **Generic launch-only, occupancy-only, or blanket PTX/SASS tactics:** rejected. The Phase 2 plan excludes them, and the new profiler bundle does not newly justify them.  
  **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 16-20, 99-105); `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 28-32, 71-76).

- **Windowed-body work as the immediate next product move:** deferred. It remains secondary because a recent direct exact win regressed the replay-backed shortlist surface and was rolled back.  
  **Evidence:** `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 563-626).

- **Broad fast-warp-dedup retry:** deferred as a conditional specialization question, not a default next product step.  
  **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 159-169, 416-441, 581-584).

## Required follow-up measurements

1. **Whole-process Nsight Systems on shortlist-heavy `xlarge/full17` and `xlarge/trade22`.**  
   Use the already planned truth surfaces from the Phase 2 plan. Capture `cuda_api_sum`, `cuda_gpu_kern_sum`, `cuda_gpu_mem_time_sum`, `nvtx_sum`, and `nvtx_gpu_proj_sum`.  
   **Question answered:** is shortlist-heavy `xlarge` still backtest-core dominated end to end, or do row-select / materialization / slice-rerank stages emerge as material?  
   **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 337-367).

2. **Hot-backtest Nsight Compute on shortlist-heavy `xlarge/full17` and `xlarge/trade22`.**  
   Use the Phase 2 plan’s `regex:^double_cross_backtest_tm_flex_f32` capture pattern.  
   **Question answered:** do the xlarge hot kernels show the same long-scoreboard / weak sector-utilization signature as the current `large` captures?  
   **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 369-399).

3. **Reducer / row-select Nsight Compute on shortlist-heavy `xlarge`.**  
   Use the planned regex covering `score_metric_rows_f32`, `select_topk_metric_rows_f32`, `select_topk_metric_rows_f32_v2`, `gather_topk_metric_rows_f32`, `gather_topk_pairs_tm_f32`, `select_topk_scored_candidates_tm_f32`, and `gather_topk_scored_scalar_pairs_tm_f32`.  
   **Question answered:** is any reducer/select kernel materially large on `xlarge`, or merely sector-inefficient?  
   **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (line 398).

4. **Pipeline and dispatch refresh on the shortlist-heavy `xlarge` truth surfaces.**  
   Re-run with `VECTORBT_KERNEL_PROFILE_PIPELINE=1` and `VECTORBT_KERNEL_LOG_DISPATCH=1`.  
   **Question answered:** does `tile_score` still dominate, and do route diagnostics remain `row_first_global_topk` with the same exact-contract dispatch?  
   **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 239-279, 287-326).

5. **Decision checkpoint after the missing `xlarge` results are surfaced.**  
   - If the dominant xlarge backtest kernel still accounts for the overwhelming majority of GPU kernel time and no replay/materialization component becomes comparably visible, proceed with the bounded lockstep pass.
   - If replay/materialization or row-selector cost becomes materially visible, skip reducer micro-work and move to the compact exact search-stage lane.

   This final branch decision is inference conditioned on the follow-up measurements above.

## Verification plan

### Measurement-phase verification

Keep the current truth set fixed:

- `double_ma`, `alma/alma`, `large`, `metric_contract=full17`
- `double_ma`, `alma/alma`, `large`, `metric_contract=trade22`
- `double_ma`, `alma/alma`, `xlarge`, `shortlist_slice_mode=halves`, `metric_contract=full17`
- `double_ma`, `alma/alma`, `xlarge`, `shortlist_slice_mode=halves`, `metric_contract=trade22`

Preserve route diagnostics across measurement runs:

- `gpu_kernel.path`
- `metrics_count`
- `tile_metrics_stride`
- score-first enabled vs blocked

### Conditional code-change verification

If the next step becomes a code change, verify in this order:

1. re-run the four truth surfaces above with the direct runner
2. run `scripts/run_gpu_perf_gate.ps1 -Mode candidate`
3. run `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` as supporting evidence, not as the sole authority
4. re-check CPU/GPU parity on representative exact `double_ma` surfaces
5. re-check `full17` and `trade22` finalist rows for exact identity
6. confirm route diagnostics stay on the intended path
7. do a narrow SASS refresh only for the touched hot kernels, not another broad cubin audit

### Success criteria

For any code change that follows the measurement step:

- measurable improvement on at least one shortlist-heavy `xlarge` truth surface
- no material regression on the sibling shortlist-heavy `xlarge` truth surface
- no material regression on the stable `large` anchor surfaces
- route diagnostics remain on the canonical exact path
- exact finalist rows remain identical for `full17` and `trade22`
- CPU fallback remains intact

## Rejection criteria

- Reject any **code-first** move taken before the missing shortlist-heavy `xlarge` attribution is completed and summarized.

- Reject any **reducer-first justification** that is still based only on `gather_topk_pairs_tm_f32` inefficiency. The surfaced absolute runtime is too small to carry that recommendation.  
  **Evidence:** `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 181-207).

- Reject a **bounded lockstep body pass** if the completed `xlarge` whole-process results do not keep the backtest core as the dominant stage, or if the resulting code change does not beat the current row-first shortlist-heavy `xlarge` surfaces without material regression on the stable `large` anchor surfaces.

- Reject any **score-first rollout** that depends on the current prototype or ignores current blocker logic. The current implementation family is already measured and not shippable.  
  **Evidence:** `gpu_backtest_kernel.rs:4247-4390`; `double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md` (lines 291-305); `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 648-669).

- Reject any change that narrows supported route surfaces, changes `auto` / `full17` / `trade22` semantics, removes CPU fallback, or changes exact finalist metric semantics.

- Reject any compact architecture slice that cannot keep replay-authoritative final `full17` / `trade22` rows and retain the current row-first path as a fallback while parity is still being proven.  
  **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 457-489); `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 350-376).

## Research framing

### Product recommendation

The product recommendation is **measurement-first**:

1. finish the missing shortlist-heavy `xlarge` attribution
2. then choose between one more bounded lockstep body pass and the compact exact search-stage architecture

That is the most defensible product sequence because the current uploaded summaries are decisive for `large` but only partially surfaced for the actual shortlist-heavy `xlarge` route.

### Research recommendation

The strongest research framing is now **static + dynamic attribution**, not a standalone “one more backtest tweak” story.

Ranked research framing:

1. **Static + dynamic attribution tool / method**  
   This is the strongest framing because the repo now has a clean chain from April 2 static/SASS contract repair to April 3 runtime attribution, with explicit separation between what is proved and what remains hypothesis.  
   **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 14-20, 239-279); `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 209-244).

2. **Replay-authoritative payload separation for exact search**  
   This is the strongest architecture research lane. The March plan already frames compact scored candidates plus replay-authoritative final rows as the real architectural alternative to the rejected score-carry reducer path.  
   **Evidence:** `double_crossover_next_phase_performance_implementation_plan_2026-03-30.md` (lines 350-376); `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 457-489).

3. **SASS-guided source-refactor oracle**  
   Still strong, but now secondary to the broader attribution method. The April 2 wave already showed the value of SASS-guided source corrections; the April 3 bundle adds the runtime decision layer that says when that method should stop and architecture work should begin.  
   **Evidence:** `double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md` (lines 33-44); `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 16-20, 69-81).

4. **Distribution-aware specialization study**  
   Still viable as a research direction, especially around fast-warp-dedup thresholds, but not yet the strongest product move.  
   **Evidence:** `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 159-169, 416-441, 581-584); `double_crossover_phase2_runtime_profiling_findings_2026-04-03.md` (lines 220-227).

5. **Narrow store-width recovery study**  
   Important historically, but no longer the live lead story. That wave already landed and is now part of the setup for runtime attribution rather than the main research frontier.  
   **Evidence:** `double_crossover_targeted_store_counts_2026-04-02.txt` (lines 1-15); `double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md` (lines 14-18).

### Bottom line

- **Immediate product action:** finish the missing shortlist-heavy `xlarge` attribution before touching product code again.
- **Most likely next code action if `xlarge` matches `large`:** one more bounded lockstep backtest-body pass.
- **Most likely next code action if `xlarge` exposes real reducer/materialization share:** move directly to the compact exact search-stage architecture.
- **Strongest research story now:** the repo’s static-plus-dynamic attribution method, with replay-authoritative payload separation as the strongest downstream architecture study.
