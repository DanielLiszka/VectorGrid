# Double Crossover Next-Phase Performance Implementation Plan

Date: 2026-03-30

Scope: repo-grounded implementation plan for the realistic remaining performance opportunities after the current M17 / M22 specialization phase is effectively complete.

This document starts a new architecture phase. It is not a continuation of the old "add another kernel variant" track.

Primary sources for this plan:

- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`

## Purpose

The previous phase extracted the obvious wins:

- fixed-contract `m17` and `m22` kernels
- lockstep and windowed specialization coverage
- dispatch cleanup
- targeted layout / pointer-walk improvements
- profiling-backed rejection of several tempting but unhelpful micro-optimizations

That work materially improved the full-row exact surfaces. The remaining performance cost is no longer explained by a missing wrapper or a simple launch tweak. The next credible gains require changing how the optimizer moves data through the backtest-to-shortlist pipeline.

This plan captures the realistic remaining options and orders them by expected value and implementation risk.

This is a performance-only plan. It is not permission to remove existing features, reduce supported request surfaces, or narrow current product semantics as a shortcut.

## Current Repo Baseline

Current supported full-row exact contracts:

- `full17`
- `trade22`

Current product/UI contract:

- the UI exposes `auto`, `full_17`, and `trade_22`
- `trade_22` is opt-in, not the global default
- explicit metric contracts currently require the exact `GpuKernel` route on the current Double MA / double-crossover path
- DynamicDoubleMa can request explicit contracts only on exact-compatible `GpuKernel(dynamic_strategy_v1)` shapes

Current fresh benchmark anchor on the live tree:

- `double_ma`
- `alma/alma`
- `large/58300pairs`
- `objective=Sharpe`
- `heatmap_bins=0`

Measured on 2026-03-30:

- `full17`: `74 / 75 / 76 ms`, `75.0 ms` mean
- `trade22`: `76 / 76 / 76 ms`, `76.0 ms` mean

Current tracker-backed hotspot summary, based on prior profiling in the current tracker:

- the shortlist-heavy `xlarge` surfaces are still dominated by the main all-pairs backtest kernel
- replay-windowed exact reruns are still material, but secondary
- Nsight Compute still points to:
  - poor global load sector utilization
  - poor global store sector utilization
  - L1TEX scoreboard stalls
  - dependency / latency pressure rather than a simple occupancy-only problem

Current strategic implication:

- the obvious route-matrix and wrapper work is already done
- the remaining opportunity space is now split between:
  - bounded CUDA-body work on the still-dominant hot kernels
  - a larger architecture change if bounded body-local work stops paying off

## Goals

1. Improve the exact shortlist-heavy `xlarge` GPU path without regressing the stable `small` / `large` product anchors.
2. Preserve the current `auto`, `full17`, and `trade22` product semantics while changing internal GPU dataflow.
3. Reduce or eliminate repeated row reread / rescoring on the dominant exact search path.
4. Keep finalist `17` / `22` metric materialization exact and authoritative.
5. Land any new architecture behind a safe fallback until it is fully proven.

## Non-Goals

- changing the UI default metric contract from `auto`
- making `trade22` the global default
- replacing CPU fallback behavior
- removing existing functionality as a shortcut to gain speed
- broad PTX/SASS surgery as the primary optimization path
- retrying the previously rejected reducer-chain score-carry design
- broadening the architecture to `include_all`, export, heatmap, or union/confidence surfaces before the exact route proves out

## Success Criteria

This phase is successful when:

- the next-phase work measurably beats the current row-first exact path on the hot `xlarge` shortlist surfaces
- `small` and `large` do not regress materially
- finalist `full17` and `trade22` rows remain identical to the current exact replay path
- the old row-first path remains available until the new path is proven
- any remaining work is clearly categorized as either a measured follow-up or an intentional defer

## Feature Preservation Contract

The next-phase work is internal optimization work only.

That means:

- no feature removal as a performance shortcut
- no silent narrowing of supported routes or request surfaces
- no change to the meaning of `auto`, `full17`, or `trade22`
- no change to CPU fallback behavior
- no change to finalist exact metric semantics
- no forced migration of `include_all`, export, shortlist-union, confidence-rerank, or heatmap surfaces onto a new fast path before parity is proven
- if a new fast path does not yet support a currently supported surface, the current row-first path remains the required fallback for that surface

## Why The Previous "One-Pass" Attempt Was Rejected

The first reducer-chain score-carry experiment was measured and rejected:

- `large/58300pairs`: flat to slightly favorable
- `xlarge/370750pairs`: catastrophic regression

The important lesson is not "one-pass is bad." The real lesson is:

- carrying objective scores through the existing top-k reducer chain is the wrong architecture
- future one-pass work should not start by bolting more side buffers onto the current reducer path

This plan therefore excludes "retry the same score-carry reducer path with minor tweaks."

## What Is Realistically Still In Play

### 1. Bounded Lockstep Full-Row CUDA-Body Pass

This is the most realistic remaining low- to medium-risk option.

Idea:

- stay inside the current retained lockstep `m17` / `m22` families
- use fresh Nsight guidance to target coalescing and dependency-chain cleanup
- avoid changing the reducer architecture or request semantics

Why this is realistic:

- the shortlist-heavy `xlarge` surfaces are still dominated by the main all-pairs lockstep kernels
- those kernels still show poor global load/store sector utilization
- those kernels still show L1TEX scoreboard stalls without spilling

What makes it bounded:

- no route expansion
- no metric-contract change
- no new one-pass contract
- no retry of already rejected shuffle/broadcast, dedup, or generic launch-shape probes

Likely target class:

- source-level access-pattern and dependency cleanup inside the retained lockstep full-row kernels
- only where it differs from already rejected lockstep pointer-walk / broadcast ideas

### 2. Bounded Windowed Full-Row CUDA-Body Pass

This is realistic, but it should remain secondary to the lockstep shortlist hotspot.

Idea:

- stay on the retained `WALK_SERIES_PTRS && !LOCKSTEP_TIME && !FAST_WARP_DEDUP` exact/windowed path
- continue only with body-local passes that preserve the kept pointer-walk and steady-state/tail wins

Why this is realistic:

- the windowed path already produced real wins from body-local changes
- the current clean Nsight story still points at CUDA-body cost rather than replay plumbing on that path

What makes it risky:

- several nearby experiments already regressed hard
- some probes looked good on one surface and failed on another
- every candidate must be cross-checked on both direct exact and replay-backed surfaces

### 3. Backtest-Local Top-K Or Tile-Local Shortlist Emission

This is the strongest remaining option.

Idea:

- stop writing full metric rows for every pair in the search stage
- compute the ranking signal while the backtest result is still hot
- emit only a compact shortlist payload for each tile
- materialize full `17` / `22` rows only for survivors during exact replay

What this changes:

- the search stage becomes "score-first"
- the exact stage remains "metrics-row authoritative"
- the reducer chain merges scored candidates, not full rows

Why this is realistic:

- it directly attacks the current repeated row reread / rescoring path
- it reduces metric-row stores on the dominant all-pairs pass
- it preserves exact replay, which is already the repo's correctness-preserving fallback

What makes it high-risk:

- it changes the optimizer dataflow, not just a kernel body
- shortlist union / confidence / extra-objective surfaces must be staged carefully
- it needs a clean fallback to the current row-first path while the new path is proving out

### 4. Split Search Payload From Exact Output Payload

This is closely related to item 3, but it should be treated as an explicit contract change rather than an accidental consequence.

Idea:

- formalize that the search stage and the exact output stage have different payload needs
- let the search stage emit the minimum payload needed to rank and merge candidates
- let the exact stage emit full `Metrics` rows only for finalists, reruns, or export surfaces

Candidate search payloads:

- `(score, pair_idx)`
- `(score, compact_row_idx)`
- `(score, pair_idx, feasibility_bit)`
- `(score, pair_idx, small metadata needed for exact replay routing)`

Why this is realistic:

- the current code already conceptually distinguishes product search rows from exact rows
- the current repo already accepts different metric widths by contract

Why it matters:

- it avoids forcing `trade22` or `full17` width on the search stage just because the exact output eventually needs those rows
- it keeps the UI / product contract unchanged while changing internal device dataflow

### 5. Packed Score/Pair Outputs And Score-Only Reducers

This is a narrower architectural option than full backtest-local top-k, and may be a safer first implementation slice.

Idea:

- keep the shortlist pipeline on device
- replace the current row-based reducer chain with reducers that merge compact `(score, id)` items
- defer full metric-row materialization until after the best candidates are known

Why this is realistic:

- it still attacks repeated rescoring
- it is smaller than fully embedding top-k inside every backtest kernel body
- it avoids writing and then rereading full metric rows for the common exact shortlist path

Why it is not the same as the rejected experiment:

- the rejected path threaded score side buffers through the current row reducers
- this option changes the reducer payload itself
- the architectural center shifts from "rows plus attached score" to "scored candidates first"

### 6. Optional `feasible_out` / Feasibility-Mask Fusion

This remains realistic, but it should come after score-first work, not before it.

Idea:

- emit a cheap feasibility side output during backtest finalization
- constrained selectors consume that mask rather than recomputing constraint checks from metric rows

Recommended scope if attempted:

- exact `GpuKernel` path only
- full-row contracts only
- constrained selectors only

Why it is realistic:

- constrained selectors do repeatedly call `passes_constraints_f32(...)`
- it is still part of the row-reread overhead on constrained exact runs

Why it is lower priority:

- the current dominant costs are still in the main backtest/search stage
- the implementation surface is broader than score-only work
- reduced-row and log-space semantics make it easy to get subtly wrong if widened too early

### 7. Secondary Surface Alignment After The Main Architecture Lands

These are realistic follow-ups, not starting points:

- heatmap score updates
- `include_all` / host-filter paths
- export-all CSV path
- extra shortlist heads and union modes
- shortlist confidence rerank stages

These should not define the first implementation slice. They should adopt the new path only after the primary exact route proves out.

## What Should Explicitly Be Excluded From The Next Phase

Do not treat these as the next obvious step:

- retrying reducer-chain score-carry fusion
- more wrapper-matrix expansion in the current M17 / M22 family
- generic `lockstep + fast_warp_dedup` retry
- more launch-bounds / block-size / simple occupancy probes
- direct PTX/SASS surgery as the primary optimization method
- changing the UI or backend default from `auto` to `trade22`
- broad `windowed` scratch-layout experiments without a new dataflow model

These either already regressed in measurement or are lower-probability than the architectural options above.

## Recommended Direction

Recommended next-phase priority:

1. Run one more bounded lockstep full-row CUDA-body pass on the dominant shortlist kernels.
2. If that stalls, run one more bounded windowed CUDA-body pass on the retained exact/windowed path.
3. Only if the bounded body-local passes stop paying off, move to a score-first exact shortlist architecture.
4. Defer feasibility fusion until the score-first path proves out.

The most defensible near-term slice is therefore not "everything one-pass." It is:

- first: one bounded body-local pass on the dominant lockstep shortlist kernels
- second: one bounded body-local pass on the retained windowed exact path
- third: if the above stalls, an exact-path-only score-first architecture slice with:
  - primary objective only
  - `include_all=false`
  - no export path
  - no host-filter path
  - no extra shortlist heads
  - no union/confidence rerank
  - full-row exact replay preserved for finalist materialization

## Proposed Architecture

### Search Stage

Current:

- backtest writes full metric rows for all pairs
- selectors reread rows
- reducers rescore rows
- final shortlist rows are gathered

Proposed:

- backtest computes ranking signal immediately
- search stage emits compact scored candidates
- reducers merge only compact scored candidates
- only shortlisted finalists are replayed or materialized as full `17` / `22` rows

### Exact Stage

Current:

- exact and search share the same row-first backbone

Proposed:

- search path becomes compact and score-first
- exact replay remains row-authoritative
- exact replay fills `Metrics` / trade-summary fields only for winners

### Fallback Contract

The current row-first path remains available until the new path is proven.

That means:

- no public request-schema changes
- no UI changes required for phase 1
- no change to `auto` / `full17` / `trade22` semantics
- no change to CPU fallback
- any currently supported surface not yet adopted by the new fast path continues to use the current row-first path

## Per-File Edit Map

`crates/app_core/src/kernels/double_crossover.cu`:

- consider one more bounded lockstep-body coalescing / dependency pass on the retained hot kernels
- consider one more bounded windowed-body pass only on the retained kept pointer-walk path
- add compact scored-candidate output path for the search stage
- add score-first candidate reducers or backtest-local shortlist emitters
- keep full-row exact replay kernels authoritative for finalist materialization
- add a narrow feasibility side-output only if the score-first path succeeds first

`crates/app_core/src/gpu_backtest_kernel.rs`:

- keep route logging and benchmark visibility clear for any bounded body-local pass
- add scratch buffers for compact search outputs
- add host-side dispatch between current row-first path and new score-first path
- keep the fallback row-first route available behind explicit gating
- update merge/gather logic to support compact scored candidates

`crates/app_core/src/double_ma.rs`:

- keep exact-route semantics and tests aligned
- ensure metric-contract meaning stays unchanged even if the search stage narrows internally

`web/src/services/optimizeRequestBuilder.ts`:

- likely no phase-1 change
- if later phases expose a new advanced toggle, keep `auto` / `full_17` / `trade_22` semantics stable

`web/src/components/AdvancedWorkspacePanel.vue`:

- likely no phase-1 change
- do not silently remap the metric-contract defaults as part of performance work

### Exact Function Touchpoints

Phase 0 baseline / measurement:

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
  - `parse_metric_contract`
  - `parse_shortlist_slice_mode`
  - `parse_eval_range`
  - `backend_for_strategy`
- `crates/app_core/src/double_ma.rs`
  - `run_double_ma_optimization_blocking_with_candles`
  - `run_double_ma_optimization_blocking_with_candles_and_eval_range`
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - `optimize_double_ma_gpu_kernel_impl`

Phase 1 bounded lockstep body pass:

- `crates/app_core/src/kernels/double_crossover.cu`
  - `double_cross_backtest_tm_flex_f32_body`
  - `store_metrics_row_fixed_f32`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - `optimize_double_ma_gpu_kernel_impl`

Phase 2 bounded windowed body pass:

- `crates/app_core/src/kernels/double_crossover.cu`
  - `double_cross_backtest_tm_flex_f32_body`
  - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`
  - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm`
  - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk_eps0_nocomm`
  - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk_eps0_nocomm`
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - `optimize_double_ma_gpu_kernel_impl`

Phase 3 minimal score-first architecture slice:

- `crates/app_core/src/kernels/double_crossover.cu`
  - `objective_score_f32`
  - `select_topk_from_pairs_tm_f32`
  - `select_topk_from_candidates_tm_f32`
  - `select_topk_metric_rows_f32`
  - `select_topk_metric_rows_from_candidates_f32`
  - `gather_topk_pairs_tm_f32`
  - `gather_topk_metric_rows_f32`
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - `search_metrics_count_for_optimization_with_contract`
  - `exact_eval_metrics_count_with_contract`
  - `reduce_metric_rows_topk_on_device`
  - `merge_current_topk_rows_into_global_head`
  - `select_tile_topk_rows_for_objective`
  - `optimize_double_ma_gpu_kernel_impl`
- `crates/app_core/src/double_ma.rs`
  - `run_double_ma_optimization_blocking_with_candles_and_eval_range`

Phase 4 global-head and replay-backed expansion:

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - `reduce_metric_rows_topk_on_device`
  - `merge_current_topk_rows_into_global_head`
  - `select_tile_topk_rows_for_objective`
  - `build_valid_pair_index_map_i32`
  - `build_valid_pair_local_rows_i32`
  - `decode_tm_flex_metrics_row`
- `crates/app_core/src/double_ma.rs`
  - exact shortlist / metric-contract tests around `trade22` and shortlist-slice surfaces

Phase 5 optional feasibility fusion:

- `crates/app_core/src/kernels/double_crossover.cu`
  - `passes_constraints_f32`
  - `select_topk_from_pairs_tm_f32_v2`
  - `select_topk_from_candidates_tm_f32_v2`
  - `select_topk_metric_rows_f32_v2`
  - `select_topk_metric_rows_from_candidates_f32_v2`
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - constrained selector loading and launch sites inside `optimize_double_ma_gpu_kernel_impl`

Phase 6 broaden-or-stop surfaces:

- `crates/app_core/src/double_ma.rs`
  - request/result surfaces for include-all, shortlist-union, confidence, and exact replay
- `web/src/services/optimizeRequestBuilder.ts`
  - `buildDoubleMaOptimizationRequest`
- `web/src/components/AdvancedWorkspacePanel.vue`
  - `metricContractStateNote`

## Phase Plan

### Phase 0: Guardrails And Measurement

Goal:

- freeze the current baseline and make the new phase measurable

Primary files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`

Deliverables:

- define benchmark anchors for:
  - `alma/alma small/1590pairs`
  - `alma/alma large/58300pairs`
  - `alma/alma xlarge/370750pairs`
  - shortlist-heavy `xlarge` slices
  - direct exact `eval_range` slices
- define the phase-1 exact-route scope explicitly:
  - primary objective only
  - no extra shortlist heads
  - no export
  - no include-all
  - no host-filter
- record current fallback behavior and dispatch logs

Acceptance:

- baseline timings and route logs are recorded before any new architecture work

Done when:

- the benchmark matrix below is populated with fresh serial numbers on an idle GPU
- the phase-1 target surface is frozen before any performance patch lands

### Phase 1: Bounded Lockstep Full-Row CUDA-Body Pass

Goal:

- take one more profiling-driven pass at the dominant shortlist lockstep kernels without changing the current row-first architecture

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

Deliverables:

- identify one bounded source-level coalescing / dependency-chain hypothesis for the retained lockstep `m17` / `m22` hotspot kernels
- implement only that bounded pass
- measure on both `full17` and `trade22`
- reject immediately if the change repeats any already-rejected shuffle, broadcast, dedup, or pointer-walk pattern

Current status update, March 30, 2026:

- fresh Nsight Compute import on the hot `lockstep_time_m17` row-first kernel still pointed at weak store utilization, including about `4.0 B/sector` global-store efficiency and no spill-driven story
- first bounded hypothesis, an alignment-guarded packed `full17` row store, was rejected:
  - `alma/alma`, `xlarge`, `full17` moved from `289 / 297 / 294 ms` to `294 / 294 / 299 ms`
- second bounded hypothesis, a `trade22`-only aligned `float4` plus `float2` tail store path in `store_metrics_row_fixed_f32<22>`, was kept:
  - `alma/alma`, `xlarge`, `trade22` moved from `307 / 310 / 309 ms` to `301 / 304 / 305 ms`
  - `alma/alma`, `large`, `trade22` now measures `73 / 73 / 74 ms`
  - `alma/alma`, `small`, `trade22` now measures `46 / 46 / 50 ms`
- the retained change is intentionally narrow:
  - `full17` code returned to the scalar baseline path
  - `trade22` keeps the wider-store fast path only when the row start is `16`-byte aligned
- the row-first product gate still passes after the retained change

Acceptance:

- measurable win on shortlist-heavy `xlarge` lockstep surfaces
- no material regression on `small` or `large`
- no route or metric-contract semantic drift

Done when:

- one bounded lockstep-body hypothesis has either been kept with evidence or rejected with evidence
- the next step is not another guess in the same already-rejected probe family

### Phase 2: Bounded Windowed Full-Row CUDA-Body Pass

Goal:

- take one more profiling-driven pass at the retained kept windowed exact path without changing the current row-first architecture

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

Deliverables:

- identify one bounded source-level hypothesis for the retained windowed `WALK_SERIES_PTRS && !LOCKSTEP_TIME && !FAST_WARP_DEDUP` path
- implement only that bounded pass
- measure on both direct exact and replay-backed exact surfaces
- reject immediately if the change only helps one surface and regresses the other

Current status update, March 30, 2026:

- one bounded windowed-body hypothesis was tried and rejected:
  - split the no-risk pointer-walk loop into a `ret1-present` fast path and a fallback path to remove per-bar `ret1_ptr != nullptr` checks on the hot full-row windowed reruns
- the direct exact `eval_range=[1000,199000)` surface improved:
  - `alma/alma`, `xlarge`, `full17` moved from `258 / 279 / 257 ms` to `238 / 239 / 241 ms`
  - `alma/alma`, `xlarge`, `trade22` moved from `260 / 266 / 259 ms` to `258 / 249 / 249 ms`
- the replay-backed exact surface regressed and forced immediate rollback:
  - `alma/alma`, `xlarge`, `full17`, `shortlist_slice_mode=halves` moved from `371 / 371 / 370 ms` to `386 / 392 / 387 ms`
- the slice was reverted immediately after that replay-backed regression
- current conclusion:
  - the retained windowed pointer-walk family can still produce isolated wins on direct exact reruns
  - it no longer looks attractive as a general next step because replay-backed exact sensitivity remains high

Acceptance:

- measurable win on direct exact or replay-backed exact windowed surfaces without regressing the other surface
- no material regression on `small` or `large`
- no metric-contract semantic drift

Done when:

- one bounded windowed-body hypothesis has either been kept with evidence or rejected with evidence
- the team can say clearly whether body-local windowed work still looks promising

### Phase 3: Minimal Score-First Exact Shortlist Slice

Goal:

- prove that a compact scored-candidate path can beat the current row-first reducer path on the exact route

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`

Deliverables:

- add compact score-first search output
- add a compact score-only reducer path
- keep final full-row materialization via exact replay
- keep the current row-first path behind a switch or narrow host decision gate
- land the new scratch / reducer flow as a parallel path first, not an in-place mutation of the current row-first reducers

Current status update, March 30, 2026:

- a narrow experimental score-first exact path now exists in the repo
- it is parked behind explicit `VECTORBT_KERNEL_SCORE_FIRST_TOPK=1` opt-in and defaults off
- the experimental path now uses a real tile-level scored-candidate reducer instead of the older faux row-first shortlist step
- parity is now covered on the narrow exact route for both `full17` and `trade22`
- fresh measured outcome on the retained prototype is still mixed and not shippable:
  - `alma/alma`, `small`, `full17`: about `114 to 118 ms` score-first versus about `151 to 152 ms` row-first
  - `alma/alma`, `large`, `full17`: about `149 ms` score-first versus about `77 to 78 ms` row-first
  - `alma/alma`, `xlarge`, `full17`: about `369 to 376 ms` score-first versus about `289 to 296 ms` row-first
- route diagnostics confirm the experimental branch is actually active:
  - `gpu_kernel.path=score_first_global_topk`
  - `gpu_kernel.score_first_mode=true`
- that means Phase 3 is still not “ship the current prototype”; it remains “replace or substantially redesign the current prototype so it can actually beat the row-first path on `large` and `xlarge`”

Initial route scope for the first slice:

- classic non-dynamic exact `double_ma` `GpuKernel` route first
- DynamicDoubleMa exact-compatible `GpuKernel(dynamic_strategy_v1)` routes deferred until the classic path has parity and a measured win

Recommended payload for the first slice:

- `(score, compact_pair_idx)` or `(score, pair_idx)`

Recommended exclusions:

- no feasibility mask yet
- no extra shortlist heads yet
- no union/confidence rerank yet
- no DynamicDoubleMa widening yet

Acceptance:

- improved `xlarge` exact shortlist surfaces
- no regression on `small`
- no semantic drift in final metric rows for winners

Done when:

- the minimal score-first slice has either landed with parity and performance wins or been rejected as not viable
- the old row-first path remains callable and comparable

### Phase 4: Expand To Global Head And Extra Exact Surfaces

Goal:

- make the score-first path cover the common exact shortlist route, not just a narrow smoke path

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`

Deliverables:

- integrate compact score-first merging into global top-k head handling
- support shortlist-slice and replay-backed exact surfaces
- support both `full17` and `trade22` exact finalist materialization

Acceptance:

- same top-k identities and final metrics as current exact path
- measurable improvement on shortlist-heavy `xlarge` surfaces

Done when:

- the score-first path covers the common exact shortlist route, not just a narrow smoke path
- shortlist-slice / replay-backed exact surfaces are either adopted or explicitly deferred with evidence

### Phase 5: Optional Feasibility Fusion

Goal:

- remove repeated device-side constraint recomputation where profiling says it matters

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`

Deliverables:

- add optional `feasible_out` or feasibility-mask emission
- wire constrained selectors to consume that mask on the new score-first path
- limit the first implementation to full-row exact contracts

Acceptance:

- no constraint-parity regressions
- measured improvement on constrained exact surfaces

Done when:

- `feasible_out` has either proven its value on constrained exact surfaces or been explicitly deferred again

### Phase 6: Broaden Or Stop

Goal:

- decide whether the new architecture should expand to secondary surfaces

Primary files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`

Deliverables:

Candidates:

- heatmap path
- include-all path
- export-all path
- union/confidence rerank stages

Decision rule:

- only broaden if profiling says those stages are now material
- preserve the current exact-route and single-expanded-GPU-kernel constraints on shortlist union / confidence / slice surfaces unless a later phase intentionally proves and adopts a wider contract
- otherwise stop and keep the win localized to the exact shortlist route

Done when:

- each secondary surface is explicitly categorized as adopt now, defer, or reject
- the plan closes without leaving ambiguous “maybe later” surfaces on the critical path

## Verification Plan

### Benchmark Command Matrix

All commands below assume:

- serial execution only
- warmed runs on an idle GPU
- release build already available at `target\release\profile_gpu_dynamic_runner.exe`

Baseline product anchors:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload small --objective sharpe --metric-contract full17 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload small --objective sharpe --metric-contract trade22 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload large --objective sharpe --metric-contract full17 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload large --objective sharpe --metric-contract trade22 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract full17 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract trade22 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
```

Current row-first exact anchors, March 30, 2026:

- `alma/alma`, `small`, `full17`: `44 / 44 / 44 ms`
- `alma/alma`, `small`, `trade22`: `46 / 46 / 50 ms`
- `alma/alma`, `large`, `full17`: `75 / 74 / 74 ms`
- `alma/alma`, `large`, `trade22`: `73 / 73 / 74 ms`
- `alma/alma`, `xlarge`, `full17`: `290 / 294 / 289 ms`
- `alma/alma`, `xlarge`, `trade22`: `301 / 304 / 305 ms`

Shortlist-heavy `xlarge` anchors:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract full17 --shortlist-slice-mode halves --iterations 3 --warmup-iterations 1 --heatmap-bins 0
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract trade22 --shortlist-slice-mode halves --iterations 3 --warmup-iterations 1 --heatmap-bins 0
```

Current row-first shortlist-heavy anchors, March 30, 2026:

- `alma/alma`, `xlarge`, `full17`, `shortlist_slice_mode=halves`: `371 / 371 / 370 ms`
- `alma/alma`, `xlarge`, `trade22`, `shortlist_slice_mode=halves`: `385 / 384 / 413 ms`

Direct exact `eval_range` anchors:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract full17 --eval-start 1000 --eval-end 199000 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract trade22 --eval-start 1000 --eval-end 199000 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
```

Current row-first direct exact `eval_range=[1000,199000)` anchors, March 30, 2026:

- `alma/alma`, `xlarge`, `full17`: `258 / 279 / 257 ms`
- `alma/alma`, `xlarge`, `trade22`: `260 / 266 / 259 ms`

Optional constrained / risk anchors when relevant:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract full17 --eval-start 1000 --eval-end 199000 --stop-loss-pct 0.04 --take-profit-pct 0.06 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract trade22 --eval-start 1000 --eval-end 199000 --stop-loss-pct 0.04 --take-profit-pct 0.06 --iterations 3 --warmup-iterations 1 --heatmap-bins 0
```

Required benchmarks:

- `alma/alma small/1590pairs`
- `alma/alma large/58300pairs`
- `alma/alma xlarge/370750pairs`
- shortlist-heavy `xlarge` halves surface
- direct exact `eval_range=[1000,199000)` surface

Required correctness checks:

- top-k identity parity versus the current exact route
- finalist metric parity for `full17`
- finalist metric parity for `trade22`
- no regression in trade-summary tail fields
- no constraint-parity drift if feasibility fusion is attempted
- no feature loss on any surface that remains mapped to the current row-first path

Required tooling:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- Nsight Systems on the new exact shortlist path
- Nsight Compute on the dominant hot kernel after the new path lands

## Risks And Guardrails

- Keep the current row-first path callable until the new path is proven on `small`, `large`, and `xlarge`.
- Do not couple this work to a metric-contract default change; `auto`, `full17`, and `trade22` must keep their current meaning.
- Do not broaden from the exact route into export or include-all surfaces before the exact route has a measured win.
- Do not remove or narrow current product functionality as a shortcut; unsupported new-path surfaces must fall back cleanly.
- Do not try to retrofit the new score-first flow by mutating the current row-oriented scratch/reducer path in place; land it as a parallel path first.
- Treat feasibility fusion as a second slice, not a prerequisite for the score-first path.
- Treat PTX/SASS analysis as diagnostic input only unless a tiny, architecture-specific compiler miss is proven and isolated.
- Reject any new implementation slice immediately if it reproduces the old pattern of looking flat on `large` while exploding on `xlarge`.

## Checklist

### Phase 0 Checklist

- [x] Record fresh baseline timings for `small`, `large`, and `xlarge` on `alma/alma`.
- [x] Record fresh shortlist-heavy `xlarge` timing on the current row-first path.
- [x] Record fresh direct exact `eval_range` timing on the current row-first path.
- [x] Freeze the architecture-slice scope to primary objective, exact route only, no extra heads, no export, and no include-all.
- [x] Record the feature-preservation boundary for the phase: which surfaces stay on the current row-first fallback until parity is proven.
- [x] Add or refresh route-selection diagnostics so the new path can be distinguished from the current row-first path.

### Phase 1 Checklist

- [x] Form one bounded lockstep full-row kernel-body hypothesis from fresh Nsight evidence.
- [x] Keep the hypothesis outside the already-rejected shuffle, broadcast, dedup, and lockstep pointer-walk families.
- [x] Measure the change on shortlist-heavy `xlarge` for both `full17` and `trade22`.
- [x] Recheck `small` and `large` after the lockstep-body change.
- [x] Revert immediately if the change is flat on `large` and loses on `xlarge`.

### Phase 2 Checklist

- [x] Form one bounded windowed full-row kernel-body hypothesis from fresh Nsight evidence.
- [x] Keep the hypothesis inside the retained kept windowed pointer-walk family, not the already-rejected neighboring probes.
- [x] Measure the change on direct exact windowed surfaces.
- [x] Measure the change on replay-backed exact windowed surfaces.
- [x] Revert immediately if the change helps only one windowed surface and regresses the other.

### Phase 3 Checklist

- [x] Add compact score-first search output buffers to the exact path scratch.
- [x] Add a narrow score-only reducer path for compact candidates.
- [x] Keep final `full17` / `trade22` metric materialization on the existing exact replay path.
- [x] Gate the new path so the current row-first route remains available as a fallback.
- [x] Verify top-k identity parity on the minimal exact route slice.
- [x] Verify finalist metric parity on `full17`.
- [x] Verify finalist metric parity on `trade22`.
- [x] Benchmark `small`, `large`, and `xlarge` against the current row-first baseline.
- [x] Reject the slice immediately if `xlarge` regresses the way the old reducer-chain score-carry experiment did.

### Phase 4 Checklist

- [ ] Extend the compact score-first path through global head merge.
- [ ] Extend the path through shortlist-slice exact reruns.
- [ ] Confirm `trade22` tail metrics still materialize correctly after finalist replay.
- [ ] Re-benchmark shortlist-heavy `xlarge` surfaces.
- [ ] Capture Nsight Systems on the kept path.
- [ ] Capture Nsight Compute on the new dominant kernel if stage dominance changes.

### Phase 5 Checklist

- [ ] Decide whether constrained exact routes are hot enough to justify `feasible_out`.
- [ ] If yes, add a full-row-only feasibility side output.
- [ ] Teach constrained selectors to consume the feasibility mask on the new path.
- [ ] Verify constrained-result parity with the current path.
- [ ] Benchmark constrained exact surfaces before keeping the change.

### Phase 6 Checklist

- [ ] Decide whether heatmap scoring should adopt the new path.
- [ ] Decide whether include-all / export paths should adopt the new path.
- [ ] Decide whether union / confidence rerank stages should adopt the new path.
- [ ] Stop broadening the architecture once profiling says the secondary stages are not material.

### Explicit Do-Not-Retry Checklist

- [ ] Do not retry reducer-chain score-carry fusion as the next step.
- [ ] Do not retry broad launch-bounds, block-size, or occupancy-only tuning as the next step.
- [ ] Do not change the default metric contract from `auto` to `trade22` as part of this performance phase.
- [ ] Do not treat PTX/SASS surgery as the primary optimization path.

## Rejected-Ideas Appendix

These ideas were already measured and should not be treated as fresh opportunities unless a genuinely new theory appears:

- reducer-chain score-carry / `score_out` fusion on top of the current row-first reducers
- generic `lockstep + fast_warp_dedup`
- dedicated or forced dedup variants in the current form
- lockstep warp-uniform `lr` / `ret1` shuffle helpers
- lockstep regime-value broadcast probes
- lockstep pointer-walk addressing
- broad launch-bounds and block-size override hunting
- broad `#pragma unroll 1` / simple occupancy-only tuning
- padded full-row stride probes
- exact column-major scratch probes
- raw-pointer windowed load probes
- no-risk windowed pointer-walk `ret1_ptr` null-check loop split in the current form
- `trade22 -> full17 search + refill` contraction
- changing the default product/UI metric contract from `auto` to `trade22`
- alignment-guarded packed `full17` row stores in the current lockstep row-first exact path

If any of these are revisited later, the burden of proof should be high:

- a clearly different hypothesis
- a clearly different implementation shape
- fresh serial measurements on an idle GPU
- both direct exact and shortlist-heavy validation where relevant

## Definition Of Done

This next-phase plan is complete when:

- the bounded CUDA-body phase has either produced a retained win or been exhausted with evidence
- a score-first exact shortlist architecture has either landed with measured wins or been rejected with evidence
- `full17` and `trade22` finalist semantics remain unchanged
- the old row-first path remains available until the new path is proven
- any remaining work is either a clearly scoped follow-up or explicitly deferred
