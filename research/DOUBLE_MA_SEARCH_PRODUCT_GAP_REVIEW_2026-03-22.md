# Double MA Search-Only vs Product-Path Gap Review
## Date: 2026-03-22

## Purpose

This note summarizes the current code-backed explanation for the remaining gap between:

- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only`
- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product`

It is intended to be handed to a stronger follow-up model for deeper analysis, especially around kernel math, launch-shape economics, and whether some of the optimization can be reformulated as a mathematical problem rather than only a code-structure problem.

## Executive summary

The gap is real, but it is not "just the second pass."

There are three distinct costs:

1. `search_only` is already slower than the idealized target because the first pass is still doing substantial per-pair work.
2. `end_to_end_product` restores both finalist recompute and heatmap work, so the benchmark pair is not isolating recompute alone.
3. The finalist recompute is architecturally correct in one key sense, but still too generic and fixed-cost heavy for only about 50 finalists.

My current view:

- The search path is still primarily a kernel problem.
- The product-path delta is primarily a finalist-recompute orchestration problem, with heatmap as a secondary product-contract tax.
- A naive one-pass full-metrics sweep is probably the wrong default.
- A redesigned two-pass architecture still looks best, but only if pass 2 becomes a true finalist executor with much stronger device-resident reuse.

## Primary code surfaces

- Benchmark split:
  - `crates/app_core/benches/double_ma_opt.rs:573-645`
- Double-MA GPU route entry:
  - `crates/app_core/src/double_ma.rs:10776-10809`
- Search-pass and product-pass runtime:
  - `crates/app_core/src/gpu_backtest_kernel.rs:6123-8233`
- Exact finalist recompute path:
  - `crates/app_core/src/gpu_backtest_kernel.rs:4365-5527`
  - `crates/app_core/src/gpu_backtest_kernel.rs:5530-5598`
- Kernel contract and metric-row behavior:
  - `crates/app_core/src/kernels/double_crossover.cu:3427-3756`
  - `crates/app_core/src/kernels/double_crossover.cu:4538-4596`
  - `crates/app_core/src/kernels/double_crossover.cu:4921-4954`
- Host stream aggregator:
  - `crates/optimizer/src/stream.rs:1-316`

## What the benchmark pair actually measures

The split benchmark does not isolate only finalist recompute.

In `crates/app_core/benches/double_ma_opt.rs:587-615`, both variants start from the same base request:

- `objective = Sharpe`
- `top_k = 50`
- `include_all = false`
- `heatmap_bins = 64`

Then the benchmark mutates only one request:

- `search_only` sets `heatmap_bins = 0` at `crates/app_core/benches/double_ma_opt.rs:613-614`
- `search_only` also disables finalist recompute through `VECTORBT_KERNEL_FINALIST_RECOMPUTE=0` at `crates/app_core/benches/double_ma_opt.rs:625-630`
- `end_to_end_product` leaves `heatmap_bins = 64`
- `end_to_end_product` enables finalist recompute at `crates/app_core/benches/double_ma_opt.rs:638-643`

Therefore the measured gap is:

- `heatmap cost`
- plus `finalist full-metrics recompute cost`
- plus any indirect effects from disabling graph replay when heatmap is enabled

It is not a pure "recompute tax" benchmark.

## Search-only is not a minimal objective pass

The current GPU search contract is reduced relative to full metrics, but still non-trivial.

`search_metrics_count_for_optimization` in `crates/app_core/src/gpu_backtest_kernel.rs:3350-3377` returns:

- `1` metric for `Pnl`
- `5` metrics for `Sharpe` and `Volatility`
- `3` metrics for `MaxDrawdown` and `Calmar`

For the benchmark in question, `Sharpe` means `metrics_count = 5`.

That matters because the search kernel still computes enough state to produce:

- log equity or pnl
- trades
- max drawdown
- mean
- std

This is visible in the kernel body in `crates/app_core/src/kernels/double_crossover.cu:3430-3439` and `crates/app_core/src/kernels/double_crossover.cu:3749-3756`.

So even before product-only work is added, `search_only` is not a trivial score-only kernel.

## Root-cause review: remaining search-only gap

The main reason `search_only` is still relatively slow is that the first pass is already dominated by the core backtest loop, not by host-side aggregation.

### What is not the main culprit

- `crates/optimizer/src/stream.rs` is not the 70 ms problem.
- In this GPU route, the host aggregator only sees selected rows and final decoded winners, not all 58,300 pairs.
- The heavier work is in the GPU path before the host aggregator is involved.

### What still costs real time in search-only

1. The search kernel still performs real statistical accumulation per pair.
   - See `crates/app_core/src/kernels/double_crossover.cu:3506-3756`.
   - Even reduced rows still walk the full time axis and accumulate drawdown, trades, mean, and std behavior.

2. The search route still maintains top-k machinery.
   - Global top-k mode is enabled under common search conditions in `crates/app_core/src/gpu_backtest_kernel.rs:6478-6494`.
   - The route allocates candidate/top buffers in `crates/app_core/src/gpu_backtest_kernel.rs:6532-6555`.
   - It later packs rows for host copy and decodes them on the CPU in `crates/app_core/src/gpu_backtest_kernel.rs:8124-8148`.

3. The search kernel contract for `Sharpe` does not map to the most specialized full-metric path.
   - Search rows use reduced row width and log-space search outputs.
   - See `crates/app_core/src/kernels/double_crossover.cu:3749-3756`.
   - Rust then decodes search-space rows differently in `crates/app_core/src/gpu_backtest_kernel.rs:649-744`.

### Practical interpretation

The current `search_only` gap versus a more aspirational `~75 ms` target should be treated as a first-pass kernel/codegen/execution-shape problem, not mainly a second-pass problem.

## Root-cause review: product-path gap

The product gap is primarily the finalist recompute branch, with heatmap as a secondary but real cost.

### Recompute trigger

After the reduced-search result is finalized, the product path conditionally re-enters a full-metrics GPU pass when:

- `include_all == false`
- `metrics_count < METRICS_COUNT`
- there are no host precomputed matrices
- finalist recompute is enabled

See `crates/app_core/src/gpu_backtest_kernel.rs:8205-8225`.

### Why the recompute is necessary

The search pass emits reduced rows. The product path still wants full metrics for:

- `best`
- `top_k`
- downstream product/reporting behavior

This is not fake work. It is a real product contract.

### Why the current recompute is still too expensive

The current recompute path is correct in one important way:

- it recomputes exact finalists, not the whole original grid

The code deduplicates finalist pairs in `crates/app_core/src/gpu_backtest_kernel.rs:5545-5564`, builds unique fast/slow period subsets in `crates/app_core/src/gpu_backtest_kernel.rs:5566-5577`, and then runs an exact-combo executor through `eval_double_ma_batch_gpu_kernel_impl` at `crates/app_core/src/gpu_backtest_kernel.rs:5579-5596`.

Within the exact-combo executor:

- it rebuilds per-tile exact pair maps in `crates/app_core/src/gpu_backtest_kernel.rs:5158-5200`
- it still recomputes `d_lr` and `d_ret1` for the whole time series in `crates/app_core/src/gpu_backtest_kernel.rs:4797-4817`
- it still regenerates MA tiles for the finalist period subsets
- it launches full-M7 kernels for a very small pair count
- it synchronizes and copies compact rows back to host in `crates/app_core/src/gpu_backtest_kernel.rs:5486-5499`

So the issue is not "it reruns too many pairs."

The issue is:

- the second pass is exact in pair count
- but still too generic in execution shape and orchestration
- so the fixed overhead is too large relative to only about 50 winners

## Small-finalist GPU execution-shape review

The current second pass is likely underfilling the GPU.

The exact-combo path explicitly allows a low-pair M7 fast path in `crates/app_core/src/gpu_backtest_kernel.rs:5262-5273`.

That is directionally correct, but it still looks like:

- tiny pair parallelism
- full time-axis walk
- non-trivial setup and synchronization around the kernel

This shape is much weaker than the main search sweep, which can amortize setup over the whole grid.

The likely consequence is:

- search pass: dominated by useful wide GPU work
- finalist pass: dominated by full-time-axis work plus fixed orchestration overhead on too few pairs

This is exactly the regime where "tiny exact recompute on GPU" can disappoint even when the arithmetic volume seems small.

## Heatmap review

The product benchmark restores heatmap work, and that is a real additional cost.

The heatmap path runs a dedicated kernel in `crates/app_core/src/gpu_backtest_kernel.rs:7246-7323` using the kernel implementation at `crates/app_core/src/kernels/double_crossover.cu:4538-4596`.

Separately, the runtime disables CUDA graph replay when heatmap is present:

- `crates/app_core/src/gpu_backtest_kernel.rs:6602-6608`

So `end_to_end_product` pays:

- direct heatmap fill/update/D2H work
- and loss of a graph-replay optimization opportunity

This is why the benchmark family should not be interpreted as "search pass versus same thing plus finalist pass."

## One-pass full-metrics vs two-pass reduced-search-plus-recompute

### Current recommendation

Keep the two-pass architecture, but redesign pass 2.

### Why naive one-pass full metrics is unattractive

One-pass M7 would force full metric work across all 58,300 pairs, not just the finalists.

That would:

- move full-metric cost into the dominant search sweep
- raise the floor of every optimization run
- likely hurt the benchmark that is supposed to showcase the search engine

### Why current two-pass still underperforms

Current two-pass only wins if pass 2 is cheap.

Right now pass 2 is not cheap enough because it still:

- rebuilds enough runtime state to look like a mini generic optimization run
- walks the full time axis again
- runs on a weak small-finalist execution shape

### Better target architecture

The better target is:

1. Search once with reduced rows.
2. Keep enough device-resident state or sufficient statistics alive.
3. Enrich only the finalists into full metrics without rebuilding a generic second optimization pipeline.

## Device-resident reuse / state carry-forward review

There is clear opportunity here.

### What is already reused

The runtime already keeps major resident buffers around within the optimization flow and clears them only later:

- see buffer lifetime behavior around `crates/app_core/src/gpu_backtest_kernel.rs:8239-8243`

So the main problem is not that buffers are dropped too early.

### What is not reused enough

The finalist path still recomputes or rebuilds:

- simple returns and log returns
- period tile setup
- exact pair maps
- enough launch scaffolding to behave like a second generic batch executor

### Best reuse opportunities

1. Keep `d_lr` and `d_ret1` from pass 1 and feed them directly into a finalist-only executor.
2. Keep finalist identities on device after top-k selection instead of decoding to host first and rebuilding pair maps later.
3. Reuse already prepared MA tiles or build a dedicated compact finalist MA view rather than running the same tile-oriented orchestration again.
4. Produce full metric rows directly into a compact finalist output buffer and copy one packed result block back to host once.

## What is probably not worth over-focusing on

- Host `StreamAggregator` in `crates/optimizer/src/stream.rs`
- Minor CPU-side decode costs by themselves
- Generalized "maybe top-k is slow" claims without separating them from the dominant kernel time

Those items matter at the margin, but they do not appear to be the main explanation for the current gap.

## Suggested benchmark cleanup

If this benchmark family is intended as a showcase, add one more split so the costs stop being conflated.

Recommended benchmark set:

1. `search_only`
   - reduced search rows
   - no heatmap
   - no finalist recompute
2. `search_plus_heatmap`
   - reduced search rows
   - heatmap on
   - no finalist recompute
3. `search_plus_finalist_recompute`
   - reduced search rows
   - no heatmap
   - finalist recompute on
4. `end_to_end_product`
   - reduced search rows
   - heatmap on
   - finalist recompute on

That would let future profiling answer:

- how much of the gap is heatmap
- how much is finalist recompute
- how much is the search pass itself

## Math problem prompt guide for a stronger follow-up model

Yes, there are real opportunities to present parts of this as mathematical problems rather than only software-review questions.

The sections below are the unsolved math-oriented questions that appear most relevant to the current code and benchmark behavior.

### Shared notation for prompting

Use the following symbols consistently in follow-up prompts:

- `N` = total candidate pairs
- `K` = finalist count
- `T` = candle count
- `M_search` = number of search metrics emitted in pass 1
- `M_full` = number of full product metrics emitted in pass 2
- `C_search(T)` = search-pass per-pair cost as a function of `T`
- `C_full(T)` = full-metric per-pair cost as a function of `T`
- `F_search` = fixed cost of the search pipeline excluding per-pair backtest work
- `F_recompute` = fixed cost of the finalist recompute pipeline excluding per-pair backtest work
- `U(K, T)` = utilization/throughput efficiency factor for the finalist executor
- `S` = carried-forward sufficient-statistics state from pass 1

For the benchmark under discussion, the approximate regime is:

- `N ~= 58,300`
- `K ~= 50`
- `T = 200,000`
- `M_search = 5` for `Sharpe`
- `M_full = METRICS_COUNT`

### Prompt 1: One-pass vs two-pass break-even analysis

Problem statement:

Determine when one-pass full metrics, current two-pass, or idealized two-pass is asymptotically and practically optimal.

Useful model:

- one-pass full metrics:
  - `Cost_one_pass = F_search_full + N * C_full(T)`
- current two-pass:
  - `Cost_two_pass_current = F_search + N * C_search(T) + F_recompute + K * C_full(T) / U(K, T)`
- idealized two-pass:
  - `Cost_two_pass_ideal = F_search + N * C_search(T) + F_recompute_ideal + K * C_full_ideal(T) / U_ideal(K, T)`

Prompt:

"Assume the current code is correct and the product contract must be preserved. Using the workload `N ~= 58,300`, `K ~= 50`, `T = 200,000`, derive the break-even conditions under which:

1. one-pass full metrics,
2. current reduced-search-plus-recompute,
3. an idealized low-fixed-cost finalist executor

are optimal.

Do not answer only qualitatively. Produce algebraic break-even inequalities in terms of `F_search`, `F_recompute`, `C_search(T)`, `C_full(T)`, and `U(K, T)`. Then interpret which parameters would have to change materially for one-pass to beat two-pass in this workload family."

### Prompt 2: Minimal sufficient-statistics derivation

Problem statement:

Determine whether the product metrics for finalists can be reconstructed from search-pass state plus a small number of extra accumulators, instead of replaying the full backtest.

Grounding in current code:

- The search kernel already accumulates pieces of:
  - log equity / pnl
  - trades
  - max drawdown
  - mean
  - std
- See:
  - `crates/app_core/src/kernels/double_crossover.cu:3506-3756`
  - `crates/app_core/src/gpu_backtest_kernel.rs:3350-3377`

Prompt:

"Treat the Double-MA trading loop as a state machine over time. Identify the minimal sufficient statistics that must be carried forward from pass 1 so that finalist full metrics can be reconstructed without replay, or prove that some metrics fundamentally require replay.

Use the current metric family in the codebase, including:

- pnl
- trades
- max drawdown
- mean
- std
- exposure
- net exposure
- downside deviation
- ulcer index
- martin ratio

For each metric, classify it as:

1. derivable from current search state,
2. derivable if a small additional accumulator set is added,
3. not derivable without replay under the current state-machine semantics.

If additional accumulators suffice, specify the exact accumulator set and update equations."

### Prompt 3: Search-pass objective reformulation for Sharpe

Problem statement:

The current `Sharpe` search uses reduced rows but still performs substantial arithmetic. Determine whether the ranking objective can be algebraically restructured into a cheaper first-pass form without changing top-k identity too much.

Grounding in current code:

- `Sharpe` uses `M_search = 5`
- search rows emit log-space row values for reduced search mode
- See:
  - `crates/app_core/src/gpu_backtest_kernel.rs:3350-3377`
  - `crates/app_core/src/kernels/double_crossover.cu:3749-3756`

Prompt:

"Analyze whether the current Sharpe-oriented search objective can be algebraically reformulated into a lower-arithmetic first pass.

Consider:

1. exact reformulations that preserve ranking exactly,
2. monotone transforms that preserve top-k identity,
3. bounds or surrogates that can be used for safe screening before exact finalist evaluation.

Give the strongest exact result you can. If exact reduction is impossible, derive the tightest admissible surrogate or bound family and explain what additional validation step would be needed to preserve correctness of final top-k output."

### Prompt 4: Approximate ranking with top-k stability guarantees

Problem statement:

Find whether approximate search metrics can reduce first-pass cost while preserving final top-k identity with provable or empirically controllable risk.

Prompt:

"Model the search stage as a top-k ranking problem over `N` candidates. Derive approximation or screening schemes that reduce arithmetic in pass 1 while preserving final top-k identity with either:

1. deterministic guarantees, or
2. explicit probabilistic guarantees.

Candidate directions:

- lower/upper bounds for Sharpe-like scores
- partial-window estimates with confidence bounds
- dominance screening
- two-level ranking with elite safety margin

For each candidate scheme, state:

- the mathematical guarantee,
- the failure mode,
- the expected cost reduction,
- whether it fits the current product requirement that final `best` and `top_k` remain exact."

### Prompt 5: Small-finalist occupancy and execution-shape model

Problem statement:

The current exact recompute uses very few pairs but a huge time dimension. Determine the best parallel decomposition mathematically before proposing new kernels.

Grounding in current code:

- exact-combo path:
  - `crates/app_core/src/gpu_backtest_kernel.rs:5158-5200`
  - `crates/app_core/src/gpu_backtest_kernel.rs:5262-5273`
  - `crates/app_core/src/gpu_backtest_kernel.rs:5486-5499`

Prompt:

"Assume `K ~= 50` finalist pairs and `T = 200,000` time steps. Model the expected GPU utilization if the kernel parallelizes primarily over pairs, then compare it against alternate decompositions:

1. pair-parallel
2. time-parallel within pair
3. segmented time reduction
4. cooperative block-per-pair or warp-per-segment layouts
5. hybrid pair-time tiling

Derive which decomposition should maximize effective throughput when fixed launch and synchronization overheads are included. Explain the break-even region where a tiny-pair replay becomes structurally inefficient and an alternate decomposition wins."

### Prompt 6: Can replay be replaced by segment algebra?

Problem statement:

If the trading state machine can be decomposed into segments, there may be a route to composable reductions instead of a second full replay.

Prompt:

"Treat each candidate strategy path as a sequence of position-state segments over time. Determine whether pnl, max drawdown, exposure-derived metrics, downside deviation, ulcer index, and martin ratio can be expressed as composable segment summaries.

If so, derive the segment algebra and show how segment summaries can be merged exactly.

If not, identify precisely which non-associative pieces of the metric/state update block exact composition and what minimal extra state would restore associativity."

### Prompt 7: Lower bound on unavoidable replay information

Problem statement:

There may be an information-theoretic reason some finalist metrics cannot be recovered from reduced rows.

Prompt:

"Given the current search-pass outputs and the current trading state-machine semantics, determine whether there exists an information-theoretic lower bound showing that exact finalist full-metric reconstruction is impossible without either:

1. replay,
2. a larger sufficient-statistics state,
3. storing additional path-dependent summaries.

State the argument formally. If impossibility holds, identify the minimal missing information."

### Prompt 8: Heatmap cost decomposition model

Problem statement:

The current benchmark pair bundles heatmap with finalist recompute. A stronger model can help quantify whether heatmap should remain in the default showcase contract.

Grounding in current code:

- heatmap kernel path:
  - `crates/app_core/src/gpu_backtest_kernel.rs:7246-7323`
  - `crates/app_core/src/kernels/double_crossover.cu:4538-4596`
- heatmap also disables graph replay:
  - `crates/app_core/src/gpu_backtest_kernel.rs:6602-6608`

Prompt:

"Model the total product-path cost as:

- search kernel cost
- top-k maintenance cost
- heatmap kernel cost
- graph-replay opportunity loss
- finalist recompute cost

Given this decomposition, determine how much heatmap cost can be tolerated before it meaningfully distorts the benchmark’s role as a search-engine showcase. Express the answer as a ratio or threshold relative to search-pass time."

### Prompt 9: Finalist executor fixed-cost minimization

Problem statement:

The current second pass is hurt by fixed overhead. Ask for a mathematical decomposition of which fixed terms matter most and how low they must go to make two-pass decisively superior.

Prompt:

"Decompose `F_recompute` into fixed-cost components such as:

- return-buffer setup
- MA tile setup
- exact pair-map construction
- kernel launch overhead
- synchronization cost
- D2H copy/decode cost

Then derive sensitivity of total product cost to each fixed term under `K ~= 50`, `T = 200,000`.

Which fixed terms must shrink the most to make two-pass decisively better than one-pass for this workload family?"

### Prompt 10: Product-contract preserving one-and-a-half-pass design

Problem statement:

There may be a hybrid design between full replay and no replay.

Prompt:

"Search for a mathematically valid 'one-and-a-half-pass' design:

- pass 1 computes reduced search rows plus a compact carried-forward state `S`
- pass 2 computes only the residual quantities needed for exact product metrics from `S`

Your goal is to minimize the dimensionality and update cost of `S` while preserving exact final `best` and `top_k` metrics.

Derive the smallest candidate `S` you can justify, and compare its asymptotic and practical cost against both current two-pass replay and naive one-pass full metrics."

### Prompt 11: Dominance, pruning, and admissible screening over the MA grid

Problem statement:

There may be exploitable structure in the `(fast_len, slow_len)` lattice that allows admissible pruning before full evaluation.

Prompt:

"Treat the search space as a partially ordered grid over `(fast_len, slow_len)` with `fast_len < slow_len`. Determine whether there exist admissible dominance rules, Lipschitz-style bounds, local smoothness assumptions, or monotonicity regions that allow exact pruning or aggressively safe screening.

If such structure exists, derive it formally. If not, state why the strategy dynamics destroy the needed regularity."

### Prompt 12: Full follow-up meta prompt

If only one prompt is used, prefer this one:

"Assume the current Double-MA GPU code is correct and the product contract must be preserved. Using the code and benchmark structure described in this note, analyze the problem as a set of math and systems questions:

1. derive break-even inequalities for one-pass vs current two-pass vs idealized two-pass,
2. identify the minimal sufficient statistics needed to avoid finalist replay or prove replay is unavoidable,
3. analyze whether Sharpe search can be algebraically restructured into a cheaper first pass,
4. model the utilization collapse of a finalist replay with `K ~= 50` and `T = 200,000`,
5. determine whether approximate screening or dominance rules can reduce first-pass arithmetic while preserving exact final top-k,
6. propose the strongest product-contract-preserving one-and-a-half-pass design you can justify.

Be explicit about which conclusions are formal, which are heuristic, and which depend on hardware-specific assumptions." 

## Bottom line

The code does not point to a simple bug.

It points to a system-level mismatch:

- pass 1 is still expensive because the search kernel itself is substantial
- pass 2 is exact but still too generic for only about 50 finalists
- the benchmark pair also bundles heatmap into the product side

The most promising next direction is not "revert to one-pass full metrics" and not "blame host aggregation."

It is:

- isolate heatmap from recompute in the benchmark family
- redesign pass 2 as a true compact finalist executor
- explore whether some of the full finalist metrics can be recovered from carried-forward sufficient statistics rather than a second full replay
