# Double MA Backtest Performance Unexplored Research

Date: 2026-04-01

Status: repo-grounded research inventory for still-open performance work on the `double_ma` CUDA backtest optimization path and its Rust-side orchestration.

## Scope

This memo is intentionally narrower than "all GPU work in the repo".

It focuses on the `double_ma` family because:

- the product headline benchmarks are anchored on `Double MA`, `ALMA/ALMA`, exact-grid GPU runs in `README.md`
- the repo's strongest public-facing performance claims are still tied to that family
- the current CUDA review corpus is deepest there

Evidence:

- `README.md:11-18`
- `README.md:42-47`
- `README.md:49-55`

This memo covers:

- CUDA kernel-body opportunities that are still open, lightly explored, or only explored indirectly
- Rust-side orchestration opportunities in `gpu_backtest_kernel.rs` and nearby `double_ma` runtime code
- build, PTX, cubin, JIT, and SASS-level questions that are still worth investigating
- measurement gaps that prevent the repo from declaring a lane truly exhausted

This memo does not treat already-measured rejects as fresh opportunities.

## Reviewed Corpus

Primary docs:

- `CUDA kernel review/double_crossover_performance_review_final.md`
- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md`
- `CUDA kernel review/double_crossover_m17_m22_valid_implementation_plan_2026-03-28.md`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `CUDA kernel review/double_crossover_shadow_10row_reduced_payload_experiment_plan_2026-03-30.md`
- `ma_review/double_crossover_cuda13_valid_implementation_tracker_2026-03-25.md`
- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md`
- `ma_review/double_ma_single_pass_valid_implementation_tracker_2026-03-24.md`
- `REPO_MAP.md`
- `AGENTS.md`

Primary live-code surfaces:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/build.rs`

## What "Unexplored" Means Here

In this memo, an item counts as still open if it falls into one of these buckets:

1. It has not been tried in the current repo.
2. It exists in code as a latent toggle or fallback path, but has not been benchmarked or promoted as a first-class strategy.
3. It was only explored through a nearby architecture, not through the specific design that now looks credible.
4. It was blocked by the current Rust/PTX/module architecture rather than disproven on performance grounds.
5. It has open checklist items or unresolved verification surfaces in the current review docs.

## Executive Summary

The current repo evidence says the obvious low-risk wrapper work is mostly done.

The strongest remaining performance questions are now:

1. Can the `double_ma` exact search path stop writing and rereading full metric rows before the final winners are known?
2. Can the Rust orchestration stop behaving like a single-stream, row-first pipeline and become a pipelined compact-candidate engine?
3. Can more of the MA preparation path stay TM-native and resident on device, especially beyond the current `sma` / `alma` fast path?
4. Is there a real cubin-vs-PTX-JIT gap on the advertising-hot kernels, or is the bottleneck still above the compiler layer?
5. Which latent runtime toggles already present in `gpu_backtest_kernel.rs` are real wins, and which are just unvalidated scaffolding?

The current docs already state that broad PTX/SASS surgery should not be the primary optimization path. That is still the right posture. But a narrow PTX/cubin/SASS audit is still underexplored and worth doing as a diagnostic branch.

Evidence:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:28-30`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:61-76`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:92-94`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:184-205`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:878-879`

## What Is Already Largely Explored

These areas should not be relitigated as if they were untouched:

- fixed-contract `m17` / `m22` wrapper expansion on the shipped row-first exact path
- the first wave of lockstep and windowed specialization coverage
- broad launch-bounds / occupancy-only tuning
- generic `lockstep + fast_warp_dedup`
- broad warp-uniform `lr` / `ret1` shuffle helpers in the old form
- the first row-first reducer-chain `score_out` carry design
- the first compact score-first prototype inside the current global-head shape
- the first shadow reduced-payload lane as a direct replacement for the shipped reduced row-first path

Evidence:

- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md:86-99`
- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md:175-205`
- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md:1444-1449`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:120-132`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:300-312`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:648-669`
- `CUDA kernel review/double_crossover_shadow_10row_reduced_payload_experiment_plan_2026-03-30.md:70-145`

## Highest-Value Unexplored Work

### 1. A genuinely different exact search-stage dataflow

This is still the most important open problem.

What has been shown already:

- the current row-first full-row exact path is still dominant on shortlist-heavy `xlarge`
- the first score-first prototype lost on `large` and `xlarge`
- the lesson from the docs is not "score-first is impossible"
- the lesson is "the first reducer shape was wrong"

What has not yet been fully explored:

- backtest-local shortlist emission where the kernel emits compact scored candidates as the primary artifact
- a score-first path that does not immediately fall back into the old row-oriented global-head merge shape
- a compact candidate path that keeps exact replay authoritative but avoids the current host-centric finalist rebuild
- a compact path that is designed first for the advertising-hot exact route instead of trying to satisfy all secondary surfaces up front

Why it still matters:

- the current exact route still writes full metric rows for all pairs during search
- the current docs still identify repeated row reread / rescoring as the remaining architectural target
- the current score-first branch in code is still explicitly experimental and heavily blocked

The current code-level blockers are also informative. Score-first is still disabled whenever any of these are active:

- non-full-row metric contracts
- constraints
- `eval_range`
- score modes
- extra shortlist heads
- shortlist confidence / union / slice modes
- precomputed matrices
- heatmap output

That means the repo has not yet really explored score-first on the richer exact surfaces that matter to advanced `double_ma` usage.

Evidence:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:184-205`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:213-255`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:628-669`
- `crates/app_core/src/gpu_backtest_kernel.rs:4077-4139`
- `crates/app_core/src/gpu_backtest_kernel.rs:12360-12384`

Research questions:

- What is the smallest exact search payload that still preserves final-top-k parity on the full-row exact route?
- Should the compact payload be `(score, pair_idx)`, `(score, pair_idx, feasibility_bit)`, or `(score, pair_idx, tiny replay metadata)`?
- Can the global-head phase be rewritten around compact candidates instead of metric rows without reproducing the rejected prototype?
- Can shortlist-heavy `xlarge` become faster without regressing the shipped `large` benchmark anchor?

### 2. Exact finalist handling that is not host-first

The current score-first prototype still becomes host-centric at finalist materialization time.

The active code path:

- copies shortlisted `fast` / `slow` rows back to host
- rebuilds finalist params on host
- calls replay from host-side finalists
- re-sorts exact rows on host

That means the prototype still pays for:

- device-to-host pair transfer
- host finalist reconstruction
- host-side post-processing before the exact route is fully settled

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:14813-14888`

What is still unexplored:

- keeping compact finalists on device until exact replay launch is fully parameterized
- device-side packing of exact replay inputs
- direct replay launch from compact candidate buffers rather than a host rebuilt params vector
- keeping final exact candidate order on device until the final authoritative replay is complete

Research questions:

- Can the replay path consume compact device-side finalist identities directly?
- Can the host become a launch coordinator rather than a finalist rebuilder?
- How much of the current score-first loss is really a host-finalist-materialization problem rather than a kernel problem?

## CUDA Kernel-Side Opportunities That Are Still Open

### 3. One more bounded lockstep full-row body pass, but only on the actual remaining hotspots

The next-phase plan still leaves room for one more bounded body-local pass driven by fresh Nsight evidence.

This is not a license to retry the already rejected lockstep micro-ideas. It is a narrower question:

- is there still one coalescing or dependency-chain cleanup that differs materially from the rejected shuffle / broadcast / dedup probes?

Evidence:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:136-162`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:561-571`

What is still open:

- write-path restructuring that attacks the still-poor store-sector utilization on the retained hot kernels
- load-address simplification that is different from the already rejected lockstep pointer-walk and warp-broadcast forms
- route-specific body cleanup for the exact `m17` / `m22` lockstep families actually used in the headline benchmarks

### 4. TM-native MA generation beyond `sma` / `alma`

The Rust runtime still treats only `sma` and `alma` as the default fast path for TM-native preparation.

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:3624-3635`
- `crates/app_core/src/gpu_backtest_kernel.rs:11807-11809`
- `crates/app_core/src/gpu_backtest_kernel.rs:12708-12723`
- `crates/app_core/src/gpu_backtest_kernel.rs:12746-12793`

What this means today:

- `sma` / `alma` can go through `compute_period_ma_tm_into(...)`
- other MA types still compute row-major output and then launch `transpose_row_to_tm`

What is still underexplored:

- extending the TM-native path to more kernel-capable MA families that matter for real product surfaces
- deciding whether the best route is more `vector_ta` support, more `VramMaComputer` coverage, or a fused transpose-elimination path in the current CUDA body
- benchmarking the transpose-elimination value on non-ALMA double-ma surfaces instead of inferring from the current ALMA anchor only

Why it matters:

- even if the marketing benchmark is `alma/alma`, the `double_ma` family performance story is stronger if the orchestrator does not collapse outside the narrow fast-path pair

### 5. Device-side feasibility side outputs on the exact route

The current docs leave `feasible_out` explicitly open.

Evidence:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:262-286`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:726-751`
- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md:1446-1449`

What is still unexplored:

- a full-row exact-path-only feasibility mask emitted from the backtest finalizer
- constrained selector kernels that consume the mask directly instead of re-reading row metrics
- whether this helps enough on constrained exact surfaces to justify the added surface area

Important note:

- this is still lower priority than getting the compact score-first architecture right
- but it remains genuinely open and has not been closed by measurement

### 6. On-device exact replay input emission

This is related to Sections 1 and 2, but it is a distinct kernel-side design question.

What is still open:

- kernels that emit compact replay-ready finalist descriptors instead of generic row-first buffers
- replay-stage kernels that can consume pair descriptors, not only materialized metric rows
- a split where search kernels stay compact while exact replay kernels remain row-authoritative

This would reduce the amount of host-side coordination needed after tile scoring.

## Rust Orchestration Opportunities That Are Still Open

### 7. Multi-stream orchestration and double-buffered tiling

The current runtime is still built around one non-blocking CUDA stream.

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:1563-1616`

The current tile loop still performs a lot of serialized work on that stream:

- period uploads
- MA prep
- transpose
- score-stage launch
- D2H copy for some paths
- sync points for graph capture, pinned transfers, profiling, and decode

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:12659-12793`
- `crates/app_core/src/gpu_backtest_kernel.rs:13931-14036`
- `crates/app_core/src/gpu_backtest_kernel.rs:3951-3986`

What is still underexplored:

- double-buffered tile orchestration with one stream preparing the next tile while another finishes scoring or replay setup on the current tile
- separate H2D, compute, and D2H streams with events rather than one serialized stream
- rethinking scratch ownership to allow pipeline overlap without reintroducing correctness hazards

Why it matters:

- the current code already has the right conceptual stages
- it does not yet exploit overlap aggressively

### 8. Caching period banks and MA tiles, not just market series and valid pair maps

The runtime already caches:

- resident market series
- valid pair maps
- scratch buffers

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:1563-1573`
- `crates/app_core/src/gpu_backtest_kernel.rs:1677-1723`
- `crates/app_core/src/gpu_backtest_kernel.rs:3683-3690`

What does not appear to be cached at the same level:

- per-slice period banks as device-resident stable buffers
- reusable MA TM tiles across exact replay shapes on the same dataset
- replay-side indicator tiles keyed by `(dataset, ma_id, params, period_slice, eval_range)`

The current loop still rebuilds host period vectors and uploads them synchronously per tile:

- `crates/app_core/src/gpu_backtest_kernel.rs:12668-12688`

What is still unexplored:

- persistent device period banks
- cached replay tiles for shortlist reruns
- explicit replay caches for the most common exact `full17` / `trade22` follow-up surfaces
- stronger reuse between the initial search pass and the exact replay path

### 9. Systematic evaluation of the latent orchestration toggles

The live runtime exposes several performance toggles:

- `VECTORBT_KERNEL_USE_CUDA_GRAPHS`
- `VECTORBT_KERNEL_ASYNC_ALLOC`
- `VECTORBT_KERNEL_PINNED_H2D`
- `VECTORBT_KERNEL_PINNED_D2H`
- `VECTORBT_KERNEL_PERSIST_BUFFERS`
- `VECTORBT_KERNEL_GLOBAL_TOPK`
- `VECTORBT_KERNEL_VALID_PAIR_MAP`
- `VECTORBT_KERNEL_TARGET_WAVES`
- `VECTORBT_KERNEL_BLOCK_X`

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:3613-3668`
- `crates/app_core/src/gpu_backtest_kernel.rs:4056-4080`
- `crates/app_core/src/gpu_backtest_kernel.rs:478-488`
- `crates/app_core/src/gpu_backtest_kernel.rs:5173-5180`

What is still underexplored:

- a disciplined matrix that says which of these are actually wins on the current `double_ma` exact route
- which combinations matter on `small`, `large`, and shortlist-heavy `xlarge`
- whether any of them should become route-specific defaults instead of generic hidden toggles

Current examples of "present but not yet a closed strategy":

- CUDA graphs are implemented but narrowly gated to no-heatmap, no-host-filter, full reusable tile shapes
- pinned H2D and D2H are enabled by default, but there is no repo-level evidence that they have been fully benchmarked as an optimization strategy rather than accepted as plumbing
- async alloc and buffer persistence exist, but they are not documented as measured headline wins

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:12647-12655`
- `crates/app_core/src/gpu_backtest_kernel.rs:13931-13971`

### 10. Launch-shape and tile-shape auto-tuning instead of hand-scored heuristics

The runtime already has launch-shape heuristics and env overrides.

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:523-657`
- `crates/app_core/src/gpu_backtest_kernel.rs:478-488`

What is still open:

- persistent per-kernel auto-tuned launch selections by device, workload tier, and metric contract
- distinct launch policies for:
  - row-first exact backtest
  - score-first compact search
  - replay-windowed exact reruns
- tile-shape auto-tuning keyed by `(device, MA family, metric contract, shortlist mode)`

Why this still matters:

- the current heuristics are explicit but still generic
- the code comments already acknowledge that backtest kernels are sensitive to block shape on mid-sized ALMA workloads

### 11. A Rust-side host shim for CUB or other external device-selection primitives

The CUDA 13 tracker explicitly says `cub::DeviceTopK` is available locally but not viable inside the current PTX-module + Rust-driver architecture without a new host-side CUDA shim.

Evidence:

- `ma_review/double_crossover_cuda13_valid_implementation_tracker_2026-03-25.md:463-467`

That means:

- `DeviceTopK` is not disproven on performance grounds
- it is blocked by orchestration architecture

This is still open if, and only if, the team is willing to add:

- a new CUDA host-side shim layer
- a stronger cubin-first or C++ runtime boundary for compact candidate selection

That makes it a real unexplored avenue, but not a cheap one.

## PTX, Cubin, JIT, and SASS-Level Work That Is Still Open

### 12. A real cubin-vs-PTX-JIT A/B on the hot exact kernels

The runtime currently:

- tries `sm_89` cubin first for Ada
- otherwise falls back to PTX JIT with `DetermineTargetFromContext` and `OptLevel(O2)`

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:1636-1661`

The build path currently:

- emits PTX with `-O3`
- best-effort builds a single `sm_89` cubin
- otherwise falls back to prebuilt PTX
- exposes `NVCC_ARGS`, but there is no repo-level evidence of a systematic `ptxas` / build-flag sweep on the live advertising kernels

Evidence:

- `crates/app_core/build.rs:34-43`
- `crates/app_core/build.rs:86-133`
- `crates/app_core/build.rs:164-223`

What is still open:

- a clean three-way A/B on the real hot `double_ma` exact kernels:
  - prebuilt PTX JIT
  - locally built PTX JIT
  - offline `sm_89` cubin
- measurement of whether the advertising-hot kernels materially benefit from offline `ptxas`
- verification of whether the current JIT path is leaving performance on the table or not

### 13. Full SASS inspection was never actually closed

The original cubin review explicitly states that full SASS disassembly was not available because `nvdisasm` / `cuobjdump` were missing.

Evidence:

- `CUDA kernel review/double_crossover_performance_review_final.md:12-18`
- `CUDA kernel review/double_crossover_performance_review_final.md:89-91`

That means the following are still unexplored in a strict sense:

- the actual instruction mix of the retained hot `m17` / `m22` exact kernels
- whether `ptxas` is already doing the expected vectorized stores and predication cleanup
- whether the poor load/store sector utilization is mostly source-level layout or partly an instruction-selection issue

### 14. Narrow PTX/SASS intervention only after a compiler miss is proven

The repo is right not to make PTX/SASS surgery the mainline plan.

But a narrow diagnostic branch is still open if all of these are true:

1. the hot kernel is identified cleanly with Nsight
2. the cubin-vs-PTX-JIT A/B shows a reproducible gap or disassembly shows an obvious compiler miss
3. the miss is tiny and architecture-specific
4. source-level CUDA cannot coax the same result out cleanly

Potential examples:

- register-allocation pathologies that only appear in one exact full-row wrapper
- missed vector store formation on the retained `trade22` path
- obviously suboptimal predication or branch lowering in one specialized wrapper

This remains diagnostic work, not the default engineering path.

Evidence:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:92-94`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:878-879`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:949`

## Unclosed Measurement and Verification Surfaces

The current next-phase plan still has open checklist items. Those are not just bookkeeping gaps. They are unresolved research surfaces.

Open exact-route verification surfaces:

- extend the new path through shortlist-slice exact reruns
- confirm `trade22` tail metrics still materialize correctly after finalist replay
- re-benchmark shortlist-heavy `xlarge`
- capture Nsight Systems on the kept path
- capture Nsight Compute on the new dominant kernel if stage dominance changes
- decide whether constrained exact routes are hot enough to justify `feasible_out`
- benchmark constrained exact surfaces if feasibility fusion is attempted
- decide whether heatmap, `include_all`, export, and union/confidence rerank should adopt a new path

Evidence:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md:923-942`

These open items matter because the current repo cannot yet claim that:

- the new architecture is exhausted
- feasibility fusion is disproven
- secondary exact surfaces are understood well enough to accept or defer permanently

The older valid-implementation plan also reinforces that several lanes were never disproven so much as superseded or deferred:

- side outputs such as `score_out` / `feasible_out`
- compile-time promotion of host-fixed flags
- `windowed` full-row coverage only if benchmark evidence justifies it
- exact-route trigger and symbol-loading verification as first-class work, not only implementation detail

Evidence:

- `CUDA kernel review/double_crossover_m17_m22_valid_implementation_plan_2026-03-28.md:67-87`
- `CUDA kernel review/double_crossover_m17_m22_valid_implementation_plan_2026-03-28.md:165-222`
- `CUDA kernel review/double_crossover_m17_m22_valid_implementation_plan_2026-03-28.md:249-288`

## Prioritized Research Backlog

### Priority 1

- Re-attack exact search-stage dataflow with a design that is compact from the start, not row-first with attached score buffers.
- Keep exact replay authoritative, but stop rebuilding finalists on host if possible.
- Use `double_ma` exact `full17` / `trade22` shortlist-heavy `xlarge` as the truth surface.

### Priority 2

- Run one more bounded lockstep full-row CUDA-body pass only if it is clearly different from the rejected shuffle / broadcast / dedup probes.
- Focus on store efficiency and dependency chains, not on generic occupancy hunting.

### Priority 3

- Generalize TM-native MA preparation beyond `sma` / `alma`.
- Treat transpose elimination as a first-class orchestration and kernel-performance problem.

### Priority 4

- Design a multi-stream, double-buffered tile pipeline on the Rust side.
- Overlap H2D, MA prep, score stage, and D2H or replay setup where correctness allows.

### Priority 5

- Run a disciplined study of CUDA graphs, pinned H2D/D2H, async alloc, buffer persistence, and launch-shape overrides on the current advertising route.

### Priority 6

- Add a cubin-vs-PTX-JIT benchmark lane and full SASS disassembly workflow for the retained hot kernels.
- Only escalate to hand PTX/SASS edits if a tiny miss is proven.

### Priority 7

- Decide whether a Rust-side CUB or CUDA host shim is worth introducing for compact candidate selection primitives.

## Adjacent Feature Opportunities

These are not the same thing as the main performance backlog above.

They are included here because they are small or medium-scope additions that reuse the current `double_ma` one-pass row surfaces, and therefore may create tangible product value without requiring a second data pass or a large kernel redesign.

They are also not really covered by `gpt54_feature_review_map.md`, because that file is explicitly scoped to optimization/performance review and says not to use it for broad feature ideation.

### The governing rule

The best additive feature rule from the current repo evidence is:

- keep the default search row fixed when the desired quantity is already reconstructible from the current row, pair params, and run constants
- prefer reduction-layer or decode-layer additions before spending new live kernel-state budget
- treat `trade22` as an opt-in richer exact/reporting surface, not as the default search payload

Evidence:

- `gpt54_feature_review_map.md:17`
- `gpt54_feature_review_map.md:809-810`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:695-716`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:783-786`

### Highest-value small feature additions from current rows

#### 1. Direct GPU `Sortino` and `Omega`

This is the clearest low-scope win.

The current notes explicitly say both are already mathematically supported by the current GPU rows, but are still route-blocked by preflight/routing and objective gating rather than missing state.

Evidence:

- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:59-70`
- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:107-109`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:34-40`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:56-57`

Why it belongs here:

- it is a user-visible feature
- it should be mostly routing / reduction-layer work
- it does not require widening the default row

#### 2. Active-time-normalized metrics and score families

The current row already carries enough information to derive active-time-normalized versions of mean and downside risk, including active Sortino.

The repo already has helper code for this family.

Evidence:

- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:98-132`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:724-726`
- `crates/app_core/src/double_ma.rs:851-881`

Small-scope feature forms:

- display-only derived metrics
- optional rerank score
- opt-in objective family for advanced users

#### 3. Drawdown-shape and asymmetry metrics from current rows

The current row already supports exact row-derived path-shape features such as drawdown chronicity and downside-share.

The repo already has helper code for these too.

Evidence:

- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:179-193`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:226-250`
- `crates/app_core/src/double_ma.rs:885-900`

Good candidate uses:

- display metric
- rerank-only feature
- soft guardrail inside a composite score

These are better first additions than widening the row for richer trade-level state.

#### 4. Exposure decomposition from current rows

The current rows already contain `exposure` and `net_exposure`, which support exact derived long exposure, short exposure, and directional-bias features.

Evidence:

- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:1159-1160`
- `crates/app_core/src/double_ma.rs:904-921`

Good candidate uses:

- opt-in filters
- advanced diagnostics
- composite penalties that discourage one-sided accidental bias

#### 5. Trade22-derived reporting and rerank features

The existing `trade22` tail already carries enough information for several small user-facing features without inventing a broader trade-metric family.

The current repo already exports or computes many of these downstream.

Evidence:

- `crates/app_core/src/gpu_backtest_kernel.rs:1178-1230`
- `crates/app_core/src/gpu_backtest_kernel.rs:17332-17359`
- `crates/app_core/src/double_ma.rs:24601`

Good candidate uses:

- `win_rate`
- `avg_win`
- `avg_loss`
- `profit_factor`
- `expectancy`

These should stay:

- opt-in
- `trade22`-only
- more useful for reporting / rerank than as default search objectives

### Existing code surfaces that already point in this direction

The repo is already moving toward richer row-derived scoring and shortlist semantics.

Existing examples include:

- current score modes:
  - `BalancedRiskAdjusted10`
  - `AdversarialEnvelope10`
  - `SupportShrunk10`
  - `ActiveSortino10`
  - `EffectiveSampleLcb10`
  - `OmegaVsSharpe10`
  - `RobustMargin10`
- shortlist confidence, union, and slice modes
- neighborhood / plateau / agreement summaries on shortlist geometry

Evidence:

- `crates/app_core/src/double_ma.rs:206-237`
- `crates/app_core/src/double_ma.rs:482-503`
- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:109-113`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:261-281`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:611-631`

That means the best next feature work is not "invent a huge new metric family." It is to finish surfacing the mathematically supported row-only families that the repo already partially acknowledges.

### Best implementation order for additive feature work

1. Unblock direct GPU `Sortino` / `Omega`.
2. Expose one current row-derived composite family as an experimental user-facing score mode.
3. Promote a small set of exact row-based constraints already supported by the current row or helper surfaces.
4. Add a compact display/rerank layer for active-time metrics and drawdown chronicity.
5. Add `trade22`-only reporting/rerank features such as `profit_factor` and `expectancy`.

Why this order:

- the first three are mostly reduction/routing/product-surface work
- they reuse current rows and existing helper math
- they preserve one-pass semantics
- they avoid spending new register/state budget before the row-algebra gains are exhausted

Evidence:

- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:1293-1301`
- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:1364-1380`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:667-682`

### What should stay out of the first feature wave

These are still real ideas, but they are not the right first tack-on features:

- broad new trade-level metric families
- walk-forward-like diagnostics in optimization
- rich regime-conditioned robustness summaries
- any feature that materially widens the default search row without proving it is not reconstructible

Evidence:

- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:1237-1241`
- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:1317-1321`
- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md:1335-1338`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md:704-716`

## Suggested Experiment Order

1. Baseline the exact `double_ma` advertising route on:
   - warm app-path benchmark
   - Criterion `large/58300pairs`
   - shortlist-heavy `xlarge`
2. Prototype a compact-candidate path that does not use the old row-first global-head structure.
3. Prototype device-side finalist replay input emission or at least remove the current host-first finalist rebuild.
4. In parallel, benchmark TM-native MA expansion beyond `sma` / `alma`.
5. Only after that, run cubin-vs-PTX-JIT and SASS audit work.

## Bottom Line

The repo has not run out of performance ideas for `double_ma`.

But the remaining credible opportunities are no longer "add one more wrapper" or "flip a simple launch flag".

The real still-open work is:

- changing the search-stage dataflow
- reducing host participation in the exact shortlist path
- broadening TM-native device-resident MA preparation
- pipelining the Rust orchestration
- and finally, only as a diagnostic branch, proving whether there is a meaningful cubin/PTX/SASS gap on the retained hot kernels

If this repo wants another step-function improvement on the `double_ma` family, it is more likely to come from a better compact exact-search architecture plus better Rust-side orchestration than from another round of local kernel micro-tuning alone.
