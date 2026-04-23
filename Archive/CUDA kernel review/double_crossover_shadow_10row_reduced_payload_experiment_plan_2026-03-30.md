# Double Crossover Shadow 10-Row Reduced-Payload Experiment Plan

Date: 2026-03-30

Status: canonical experimental planning document for a dev-only shadow search architecture.

Archival note, 2026-03-31: the shadow reduced-payload code path was removed from the active codebase after clean serial benchmarks and profiling showed it lost to the shipped row-first reduced path. This document is retained as evidence and experiment history.

Scope: repo-grounded implementation plan for an isolated classic `double_ma` GPU experiment that tests whether a lighter internal search payload can beat the current row-first reduced exact baseline without changing shipped application behavior.

This document is the canonical plan for the shadow reduced-payload experiment lane.

The older filename `CUDA kernel review/double_crossover_shadow_reduced_payload_search_experiment_plan_2026-03-30.md` is retained only as a pointer to this file.

It is intentionally separate from:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md`

Those documents track the shipped path and the completed M17 / M22 phase. This document tracks a parallel experiment only.

## Reviewed Corpus

This plan was tightened against the current repo state and the existing markdown review corpus, not just against one prior draft.

Primary reviewed documents:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md`
- `CUDA kernel review/double_crossover_performance_review_final.md`
- `ma_review/double_crossover_cuda13_review.md`
- `ma_review/double_crossover_cuda13_valid_implementation_tracker_2026-03-25.md`
- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md`
- `ma_review/double_ma_single_pass_validation_pullforward_2026-03-24.md`
- `ma_review/double_ma_single_pass_valid_implementation_tracker_2026-03-24.md`

Reviewed but mostly excluded from this plan because they are cross-strategy or feature-widening work rather than core shadow-search performance work:

- `ma_review/cross_strategy_gpu_kernel_widening_tracker_2026-03-25.md`
- `ma_review/standalone_unified_gpu_easy_extras_tracker_2026-03-26.md`

## Purpose

The current evidence says:

- the shipped row-first path is still the correct production choice
- the obvious low-risk tuning work on the current row-first architecture is largely exhausted
- recent score-first extensions inside the current reducer shape regressed and were reverted
- the remaining credible win is not "more local tuning on the same pipeline," but "a different search-stage dataflow"

This experiment exists to answer one falsifiable question:

- can a shadow reduced-payload search architecture beat the current reduced exact GPU baseline that resolves to `10` rows on common `Sharpe` surfaces?

This is not a product change. It is an evidence-backed experiment lane.

## Current Baseline

Benchmark truth surface for the first wave:

- strategy: classic `double_ma`
- fast/slow MA: `alma/alma`
- workload: `large` (`58,300` valid pairs)
- objective: `Sharpe`
- exact `GpuKernel`
- `include_all=false`
- `heatmap_bins=0`
- no constraints
- `metric_contract=auto`

Current live-tree observation on that surface:

- `auto` resolves to `metrics_count=10`
- fresh serial `small` row-first reduced `auto`: `73 / 65 / 62 ms`
- fresh serial `large` row-first reduced `auto`: `70 / 67 / 67 ms`
- fresh serial `xlarge` row-first reduced `auto`: `220 / 219 / 220 ms`
- fresh serial `small` shadow scalar-score `auto` with reduced replay: `115 / 115 / 111 ms`
- fresh serial `large` shadow scalar-score `auto` with reduced replay: `146 / 146 / 147 ms`
- fresh serial `xlarge` shadow scalar-score `auto` with reduced replay: `278 / 277 / 295 ms`
- recent serial `auto` run: `73 / 74 / 86 ms`
- earlier near-idle serial `auto` run: `73 / 73 / 72 ms`
- recent serial `full17` run: `78 / 78 / 77 ms`
- recent serial `trade22` run: `76 / 75 / 77 ms`

Interpretation:

- on the current row-first architecture, `10`, `17`, and `22` rows land in roughly the same timing band on this exact large surface
- row width alone is therefore not the main remaining bottleneck
- the shadow lane must beat the current reduced row-first baseline, not just `full17` or `trade22`

## Implementation Status

Phase 0 scaffolding is implemented in the live repo:

- runner flag: `--shadow-reduced-payload 1|0|on|off|true|false`
- env gate used by the kernel runtime: `VECTORBT_KERNEL_SHADOW_REDUCED_PAYLOAD`
- current state: explicit runner-only activation surface exists, and the first real shadow lane now runs on the narrow reduced exact surface only
- current route diagnostics now report:
  - `gpu_kernel.shadow_reduced_payload_env_enabled`
  - `gpu_kernel.shadow_reduced_payload_mode`
  - `gpu_kernel.shadow_reduced_payload_blockers`
- current behavior when requested:
  - narrow reduced exact surface: shadow scalar-score search path with exact finalist replay
  - unsupported shapes such as `full17`: explicit row-first fallback with shape blockers like `metrics_contract_not_reduced_auto`

Phase 0 verification completed on the kept scaffolding:

- targeted tests for shadow flag defaults, blocker behavior, and route diagnostics
- release build of `profile_gpu_dynamic_runner`
- runner smoke with `--shadow-reduced-payload 1`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

Phase 1 and Phase 2 have now been exercised on the first real shadow slice:

- separate shadow scratch buffer is live via `scratch.d_tile_scores`
- the first real shadow payload is scalar Sharpe score plus compact pair identity
- the shadow replay path was then narrowed to use reduced-authority replay on reduced `auto` surfaces instead of always forcing a full `17`-row exact pass
- parity on the narrow reduced exact surface is now restored after fixing a host integration bug where the shadow backtest kernels were still writing to `scratch.d_tile` while the reducer read `scratch.d_tile_scores`
- targeted parity tests and route-diagnostic tests now pass
- the kept shipped path remains row-first because the shadow scalar-score slice regressed on every serial benchmark that matters:
  - `small`: row-first `73 / 65 / 62 ms` vs shadow `115 / 115 / 111 ms`
  - `large`: row-first `70 / 67 / 67 ms` vs shadow `146 / 146 / 147 ms`
  - `xlarge`: row-first `220 / 219 / 220 ms` vs shadow `278 / 277 / 295 ms`

Interpretation:

- the first true reduced-payload scalar shadow lane is now correctness-safe
- the reduced-replay follow-on materially narrowed the gap on `xlarge`, but the lane is still slower than the current reduced row-first baseline on every tested workload
- it is therefore rejected as the winning design for this phase, while remaining dev-only scaffolding for any later shadow variants

Currently supported surfaces that must remain on the existing row-first route unless and until a later shadow variant proves parity and speed:

- all shipped application surfaces
- all non-`auto` metric-contract surfaces such as `full17` and `trade22`
- any constrained, heatmap, shortlist-union, shortlist-confidence, shortlist-slice, export, `include_all`, precomputed-matrix, eval-range, or dynamic-strategy shape

Phase 3 has now exercised three real shortlist-emission variants:

- the first shadow shortlist-emission slice was the scalar score plus compact pair-identity lane:
  - `double_cross_backtest_tm_flex_f32_shadow_thread_top2_scores`
- the second shortlisted follow-on was the per-thread reduced-row shortlist kernel:
  - `double_cross_backtest_tm_flex_f32_shadow_thread_top2_reduced_rows`
- the latest bounded follow-on was a block-local compact scored-pair shortlist kernel:
  - `double_cross_backtest_tm_flex_f32_shadow_block_topk_scores`
- all three variants kept finalist exact replay authoritative and remained confined to the narrow reduced shadow surface
- the shadow lane still remains dev-only and opt-in behind `VECTORBT_KERNEL_SHADOW_REDUCED_PAYLOAD`
- the shadow gate is intentionally narrower:
  - regime-filtered shapes are blocked
  - stop-loss / take-profit shapes are blocked
  - the current shadow kernels ignore those inputs and must not silently route them

Fresh serial measurements on the rejected block-local compact scored-pair slice:

- route diagnostics confirmed `gpu_kernel.path=shadow_reduced_payload` and `dispatch=shadow_block_topk_scores`
- `small`:
  - row-first `81 / 76 / 73 ms`
  - shadow block-local scored pairs `144 / 140 / 143 ms`
- `large`:
  - row-first `75 / 76 / 76 ms`
  - shadow block-local scored pairs `1042 / 435 / 434 ms`
- `xlarge`:
  - row-first `245 / 249 ms`
  - shadow block-local scored pairs `4647 / 2320 ms`

Interpretation:

- the block-local scored-pair follow-on regressed on every benchmark surface that matters
- the dominant added cost was host-side finalist materialization:
  - `large` shadow `d2h_copy` climbed to roughly `342-864 ms`
  - `xlarge` shadow `d2h_copy` climbed to roughly `2207-4531 ms`
- it is therefore rejected as a winning Phase 3 design
- the current checked-in shadow lane is back on the earlier scalar-score baseline
- the shipped path remains row-first

The current shadow experiment has therefore rejected:

- the earlier scalar-score reduced replay slice as a replacement architecture
- the per-thread reduced-row shortlist slice as a replacement architecture
- the block-local compact scored-pair shortlist slice as a replacement architecture

Phase 4 has now exercised the first pair-order / locality A/B on the kept thread-top2 scored-candidate slice:

- current kept shadow ordering: warmup-bucketed compact valid-pair map
- tested shadow-only locality alternative: dense `pf-major` compact valid-pair map
- the dense `pf-major` variant preserved winner parity but regressed sharply where it matters:
  - `small`: warmup-bucketed `123 / 115 / 116 ms` vs dense `114 / 113 / 112 ms`
  - `large`: warmup-bucketed `142 / 141 / 141 ms` vs dense `341 / 337 / 335 ms`
  - `xlarge`: warmup-bucketed `304 / 303 / 305 ms` vs dense `1819 / 1841 / 1827 ms`
- the regression was dominated by finalist materialization, not kernel launch cost:
  - `large` dense `d2h_copy` climbed to roughly `253-259 ms`
  - `xlarge` dense `d2h_copy` climbed to roughly `1712-1733 ms`

Interpretation:

- preserving dense `pf-major` locality was a reasonable hypothesis, but on this shadow architecture it is decisively worse than the existing warmup-bucketed ordering
- the pair-order / locality lane is therefore now evidence-backed and rejected as a winning direction for this shadow experiment
- the kept shadow baseline remains the warmup-bucketed thread-top2 scored-candidate slice

The remaining credible lanes are now the heavier conditional alternatives in this document, not more shortlist-emission reshaping or pair-order relabeling inside the same reducer chain.

Fresh retained-slice profiling on the actual kept shadow baseline is now captured:

- Nsight Systems artifacts:
  - `target/nsight/shadow_thread_top2_scores_xlarge.nsys-rep`
  - `target/nsight/shadow_thread_top2_scores_xlarge.sqlite`
- Nsight Systems xlarge hot-kernel summary on `alma/alma`, `Sharpe`, `auto`, `--shadow-reduced-payload 1`, `--shadow-pair-order warmup_bucketed`:
  - `double_cross_backtest_tm_flex_f32_shadow_thread_top2_scores`: `205.25 ms`
  - authoritative replay `double_cross_backtest_tm_flex_f32`: `74.14 ms`
  - `alma_batch_tiled_f32_2x_tile256_tm`: `32.02 ms` total across 4 launches
  - reducer kernels were negligible by comparison
- Nsight Compute artifact:
  - `target/nsight/shadow_thread_top2_scores_xlarge.ncu-rep`
- Nsight Compute summary on `double_cross_backtest_tm_flex_f32_shadow_thread_top2_scores`:
  - duration: `219.48 ms`
  - registers per thread: `46`
  - theoretical occupancy: `83.33%`
  - achieved occupancy: `66.08%`
  - SM busy: `54.03%`
  - memory throughput: `56.91 GB/s`
  - L1/TEX hit rate: `57.48%`
  - L2 hit rate: `96.41%`
  - global-load sector utilization: about `19.3 / 32 B`
  - global-store sector utilization: about `16.0 / 32 B`
  - L1TEX scoreboard stall pressure: about `7.7 cycles`, about `55.3%` of the `14.0` average cycles between issued instructions
  - no local-memory spilling and no shared-memory spilling
  - waves per SM: `2.26`, with partial-wave tail-risk still called out at up to `33.3%`

Interpretation:

- the kept shadow baseline is still dominated by the search-stage shadow score kernel itself, not by shortlist reduction primitives
- the retained shadow slice still loses to the reduced row-first baseline where it matters, so it remains a dev-only experimental reference path rather than a candidate shipped architecture
- the fresh profiler evidence does not support more reducer-adjacent work as the next move:
  - a shadow-only `DeviceTopK` host-shim is not justified because the reducer portion is already negligible next to the search-stage kernel
  - shadow-only shared-memory spill experiments or metric-family split work are not justified because the retained shadow kernel is not spill-driven and does not show high register pressure
  - tiny aux-buffer diagnostics are not justified as a performance experiment because they do not attack the dominant cost center
  - `full17` / `trade22` compatibility broadening is not justified while the reduced shadow lane still loses on `large` and `xlarge`

## Feature Preservation Contract

This experiment is performance-only internal work.

It does not permit:

- UI changes
- Tauri command changes
- normal application-route wiring
- changes to the meaning of `auto`, `full17`, or `trade22`
- removal or narrowing of existing functionality
- changes to CPU fallback behavior
- changes to final exact replay semantics
- silent feature loss on any currently supported row-first surface

If the experiment does not support a currently supported surface, the current row-first path remains the required fallback for that surface.

## What "Exhaustive" Means For This Experiment Lane

The goal is not to blindly implement every old idea.

The goal is to exhaust every still-feasible performance opportunity that:

- is relevant to a dev-only reduced-payload shadow search lane
- is not already implemented in the shipped path
- is not already measured and rejected on materially equivalent architecture
- can be tested without mutating shipped behavior

This document therefore separates ideas into:

- must-test first-wave experiments
- conditional second-wave experiments
- conditional third-wave experiments
- explicit exclusions because they are already implemented, already rejected, or out of scope

## Hypothesis

The strongest remaining architecture claim is:

- the search stage should emit only the minimum payload needed to compete candidates
- full metrics rows should not be the primary search artifact
- finalists should still replay through the current authoritative exact path

If that claim cannot beat the current reduced row-first baseline on `large` and `xlarge`, then the current row-first design remains the correct long-term answer for this codebase.

## Architecture Target

The shadow path should stay parallel to the current shipped path.

Preferred shape:

1. backtest search stage computes only the ranking payload needed for shortlist competition
2. search stage emits compact scored candidates instead of full search rows
3. reducers merge compact candidates only
4. finalists replay through the existing authoritative exact path
5. final returned metrics remain identical to the current authoritative path

The architectural center should be:

- reduced search payload
- exact replay authoritative for final rows

It should not be:

- "rows plus attached score side-buffers" threaded through the old reducer chain

## First-Wave Must-Test Experiments

These are the realistic still-feasible first-wave experiments that should be tested before declaring the lane exhausted.

### 1. Minimal Compact Scored-Candidate Search

Purpose:

- prove the lane can operate without full search rows as the primary artifact

Preferred payload:

- `(score, pair_id)`

Allowed variants:

- `(score, pair_idx)`
- `(score, compact_pair_idx)`
- `(score, pair_id, tiny replay-routing metadata)` only if strictly necessary

Not allowed in the first slice:

- full metrics rows as the main shadow reducer payload
- score side-buffers piggybacked onto the current row reducers

### 2. Tile-Local Shortlist Or Backtest-Local Top-K Emission

Purpose:

- test the best still-feasible architecture idea from the broader performance review corpus

Meaning:

- shortlist candidates while the backtest result is still hot
- emit a tile-local shortlist instead of a global full-row matrix

Why it belongs in the first-wave backlog:

- it directly attacks repeated row writes and rereads
- it was identified as the strongest remaining route in the next-phase tracker
- it has not yet been exhaustively tested as a separate shadow architecture

### 3. Split Search Payload From Exact Output Payload

Purpose:

- formalize that search and final output have different payload needs

Minimum requirement:

- search stage uses reduced compact payload only
- exact replay remains responsible for finalist `10` / `17` / `22` materialization

This is not optional. It is the core design boundary of the experiment lane.

## Conditional Second-Wave Experiments

These should be tested only after the first-wave compact search lane is running and benchmarked.

### 4. Compact Candidate Payload Variants

Candidate variants:

- `(score, pair_id)` baseline
- `(score, pair_id, feasibility_bit)` if hard constraints are brought into scope later
- `(score, pair_id, tiny aux stats)` only if replay count or constraint cost justifies it
- reduced metrics payload instead of pure score-only payload

Guardrail:

- prefer tiny aux metadata over widening the default reduced search contract

This comes directly from the single-pass math/addendum guidance: preserve the reduced core contract and use a very small auxiliary payload only if needed.

### 5. Compact Candidate Merge Variants

Allowed follow-on variants:

- tile-local top-k emission followed by global compact merge
- alternate stable `pair_id` encodings
- compact pair-code or compact row identifiers

Guardrail:

- do not retry the already rejected packed global-head shape from the current experimental score-first path as if it were new evidence

### 6. Pair-Map Ordering / Locality Variants

Conditional idea from the older CUDA review:

- preserve dense `pf`-major locality
- optionally test coarse `start_t` bucketing if the shadow path still shows lockstep tail-wave or locality problems

This is second-wave only because:

- it is only relevant after the shadow path exists
- it should be driven by shadow-path Nsight evidence, not by the old row-first hotspot alone

## Conditional Third-Wave Experiments

These are still-feasible but should remain conditional because they add more architecture or runtime complexity.

### 7. `cub::DeviceTopK` On Compact Candidate Buffers

Status from prior review corpus:

- identified as a promising final-shortlist primitive
- rejected in the older CUDA13 tracker for the current PTX-module + Rust-driver architecture because it requires a host-side CUDA shim

What that means here:

- do not treat `DeviceTopK` as a phase-1 kernel swap
- do include it as a valid third-wave experiment if the shadow lane already produces a compact device candidate buffer and the team is willing to add a dedicated CUDA host-shim architecture for this experiment

### 8. Shadow-Specific `ret1` Removal

Status from prior review corpus:

- already accepted on the current shipped path

What that means here:

- do not retest it blindly as a "new idea"
- do retest it only if the shadow path reintroduces a separate `ret1` stream or preprocessing dependency

### 9. Shadow-Specific Shared-Memory Spilling Or Metric-Family Split

Status from prior review corpus:

- shared-memory spilling was rejected for the current default wrapper family
- metric-family splitting was suggested only if register pressure remains high

What that means here:

- do not schedule either as first-wave work
- do keep them as conditional shadow-only experiments if Nsight Compute on the new shadow kernels shows a genuinely different register/shared-memory profile

### 10. Multi-Head Exact Top-K / Union Shortlist On Top Of The Shadow Lane

Status from prior review corpus:

- multi-head union shortlist is already implemented for the current product path

What that means here:

- do not rebuild it as if it were missing
- do treat it as a later compatibility question for the shadow lane if the scalar reduced-payload path wins first

### 11. Tiny Aux-Buffer Diagnostics Or Robustness Metadata

Potential later additions drawn from the single-pass math and validation pull-forward notes:

- small checkpoint snapshots
- small volatility-bucket summaries
- small neighborhood / plateau diagnostics

Guardrail:

- these are not first-wave performance experiments
- only consider them after the core reduced-payload lane wins and only if they can ride a tiny aux payload rather than widen the default reduced search contract

## Explicit Exclusions

These should be documented as excluded so the shadow lane does not waste time relitigating dead ends.

### Already Rejected On Materially Equivalent Architecture

- reducer-chain `score_out` / score-carry fusion on top of the current row-first reducers
- packed pair-code global-head merge on top of the current experimental score-first exact path
- block-local reduced-row shortlist emission on top of the current shadow row-reducer chain
- generic combined `lockstep + fast_warp_dedup`
- broad lockstep warp-uniform `lr` / `ret1` shuffle helper revisits
- the already rejected family of small lockstep/windowed micro-tuning probes from the M17 / M22 tracker

### Already Implemented On The Shipped Path

- `ret1` removal on the current default path
- current exact multi-head shortlist / union-shortlist functionality
- current checkpoint / shortlist-slice / neighborhood robustness features
- current volatility-bucket and validation-side robustness surfaces

These are relevant background, but not "missing experiments" for this shadow lane.

### Out Of Scope For The First Shadow Lane

- UI or product exposure
- `include_all`
- export
- heatmap
- shortlist union / confidence / slice rerank
- DynamicDoubleMa
- non-`Sharpe` scalar surfaces before the narrow exact surface proves out
- broad PTX/SASS patching as the main optimization method
- warp-aggregated heatmap atomics

## Likely Code Touchpoints

Required:

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

Likely only if parity or narrow helper exposure needs it:

- `crates/app_core/src/double_ma.rs`

Preferred ownership split:

- runner gating and diagnostics in `profile_gpu_dynamic_runner.rs`
- shadow route gate, scratch allocation, compact reducer flow, and replay handoff in `gpu_backtest_kernel.rs`
- shadow kernel bodies or wrappers in `double_crossover.cu`
- targeted parity tests in `double_ma.rs` or `gpu_backtest_kernel.rs`

## Benchmark And Profiling Matrix

All performance work in this lane must use serialized runs only.

### Baseline Commands

`small`:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload small --objective sharpe --metric-contract auto --iterations 3 --warmup-iterations 1 --heatmap-bins 0
```

`large`:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload large --objective sharpe --metric-contract auto --iterations 3 --warmup-iterations 1 --heatmap-bins 0
```

`xlarge`:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload xlarge --objective sharpe --metric-contract auto --iterations 3 --warmup-iterations 1 --heatmap-bins 0
```

### Shadow Commands

The exact switch name can change, but the shadow path must remain runner-only or env-gated.

Example shape:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --workload large --objective sharpe --metric-contract auto --iterations 3 --warmup-iterations 1 --heatmap-bins 0 --shadow-reduced-payload 1
```

### Optional Later Comparisons

Only after the reduced shadow lane wins on `auto`:

- rerun against `full17`
- rerun against `trade22`
- rerun replay-backed shortlist surfaces

### Nsight Requirements

After any kept slice:

- one Nsight Systems capture on the shadow path
- one Nsight Compute capture on the shadow path

Use those captures to decide whether to try:

- tile-local shortlist variants
- pair-order/locality variants
- shadow-only shared-memory spill or metric-family split

## Acceptance Criteria

Keep a slice only if all of the following are true:

- the slice stays isolated from shipped behavior
- best params match the reduced row-first baseline on the narrow exact surface
- top-k identity matches the reduced row-first baseline on the narrow exact surface
- finalist metrics after exact replay match the current authoritative path
- `large` improves
- `xlarge` improves
- `small` does not regress materially

## Rejection Criteria

Reject a slice immediately if:

- it only wins on `small`
- it loses on `large`
- it loses on `xlarge`
- it requires product-path changes to function
- it mutates the shipped row-first route instead of staying parallel
- it replays a previously rejected architecture under a new label

## Phase Order

### Phase 0: Harness And Baseline Lock

Goal:

- isolate the shadow trigger and lock the reduced baseline

### Phase 1: Minimal Compact Candidate Path

Goal:

- build the smallest working `(score, pair_id)` shadow path with authoritative exact replay

### Phase 2: First Benchmark Decision

Goal:

- decide whether the minimal compact-candidate lane is competitive

### Phase 3: Stronger Search-Stage Variants

Goal:

- test tile-local shortlist emission and other first real architectural alternatives if Phase 2 is at least promising

### Phase 4: Conditional Broadening

Goal:

- only after a clear reduced-lane win, decide whether the lane should absorb more complex ideas such as compact payload variants, `DeviceTopK`, or shadow-side compatibility with richer exact surfaces

## Checklist

### Phase 0 Checklist

- [x] Record fresh row-first reduced `auto` baselines for `small`, `large`, and `xlarge`.
- [x] Confirm that the narrow benchmark surface resolves `auto` to `metrics_count=10`.
- [x] Add a runner-only or env-gated shadow trigger.
- [x] Verify that the normal application route cannot reach the shadow lane.
- [x] Add explicit diagnostics that identify the shadow lane when active.
- [x] Record which currently supported surfaces must remain on the row-first fallback until parity is proven.

### Phase 1 Checklist

- [x] Add separate shadow scratch allocation without mutating the shipped row-first path.
- [x] Add a minimal compact scored-candidate payload such as `(score, pair_id)`.
- [x] Add a shadow reducer flow that merges compact candidates only.
- [x] Keep finalist exact replay on the current authoritative path.
- [x] Verify that the shadow lane returns valid winners on the narrow exact surface.
- [x] Verify that diagnostics correctly identify the shadow lane.

### Phase 2 Checklist

- [x] Verify best-params parity against the reduced row-first baseline.
- [x] Verify top-k identity parity against the reduced row-first baseline.
- [x] Verify finalist metrics parity after exact replay.
- [x] Benchmark `small` serially.
- [x] Benchmark `large` serially.
- [x] Benchmark `xlarge` serially.
- [x] Reject immediately if `large` or `xlarge` regresses.

### Phase 3 Checklist

- [x] Test a first non-scalar shortlist-emission variant and record the outcome.
- [x] Test tile-local shortlist or backtest-local top-k emission as a separate shadow variant.
- [x] Test whether search payload and exact output payload are still cleanly split under that variant.
- [x] Capture Nsight Systems on any kept Phase 3 slice.
- [x] Capture Nsight Compute on any kept Phase 3 slice.
- [ ] Keep the Phase 3 slice only if it beats the earlier shadow baseline and the row-first reduced baseline where it matters.

### Phase 4 Checklist

- [x] Decide whether compact candidate payload variants are justified.
- [x] Decide whether pair-order / locality variants are justified.
- [x] Decide whether a tiny aux buffer is justified.
- [x] Decide whether a shadow-specific `DeviceTopK` host-shim experiment is justified.
- [x] Decide whether shadow-only shared-memory spill or metric-family split is justified from real Nsight evidence.
- [x] Decide whether `full17` / `trade22` compatibility work is justified.
- [x] Do not broaden the lane until the reduced scalar shadow path has already won clearly.

### Explicit Exclusion Checklist

- [ ] Do not wire this lane into the UI.
- [ ] Do not wire this lane into the shipped application route.
- [ ] Do not remove or narrow existing functionality as a shortcut.
- [ ] Do not relabel previously rejected reducer-chain score-carry work as a new experiment.
- [ ] Do not retest already implemented shipped-path work unless the shadow lane reintroduces the underlying cost.
- [ ] Do not run benchmarks in parallel.
- [ ] Do not treat a `small`-only win as success.

## Definition Of Done

This experiment lane is done when one of these outcomes is reached:

1. A shadow reduced-payload search architecture clearly beats the current reduced row-first baseline on `large` and `xlarge`, preserves exact replay parity, and remains isolated from shipped behavior.
2. The credible reduced-payload variants have been tested and rejected with evidence, leaving the current row-first reduced path as the clear best design.

Either outcome is useful. The point of this document is to make that conclusion evidence-backed and final enough that the repo can stop relitigating the same architecture question.
