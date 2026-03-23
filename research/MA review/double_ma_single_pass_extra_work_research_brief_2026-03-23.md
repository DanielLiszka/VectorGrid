# Double MA Single-Pass Extra-Work Research Brief

Date: 2026-03-23

## Purpose

This document is a prompt-ready research brief for a stronger GPT-5.4 Pro model.

The current Double MA shared GPU backend has already moved from the old replay-backed product path to a 10-row single-pass product path with exact `omega`. That changes the optimization question materially.

The old question was:

- how do we reduce the replay burden in the old finalist-recompute path?

The new question is:

- given that modest additional work inside the same single pass appears to have trivial cost while occupancy headroom remains, what extra useful work should be added to the optimization stage?
- which additions are exact and high-value?
- which additions should be phrased as math / optimization problems rather than purely implementation tasks?

This brief is meant to give GPT-5.4 Pro the current repo state, the most relevant code surfaces, and a set of unresolved math-heavy design questions to solve.

---

## Additive-Only Constraint

All recommendations in response to this brief should be backward-compatible and additive in nature.

Do not propose removing, weakening, or narrowing any of the following:

- the current 10-row single-pass product path
- existing exported product metrics / fields
- existing single-metric ranking modes
- the current diagnostic replay baseline / benchmark variants
- CPU fallback behavior
- existing benchmark surfaces used for regression tracking

Preferred design style:

- add new objectives, scores, constraints, summaries, or reduction modes
- keep existing behavior available
- use env-gated, flag-gated, or explicitly opt-in modes when needed
- widen internal capabilities only in ways that preserve current outputs and compatibility

---

## Current Repo State

### Important state change

The current preferred Sharpe/Volatility Double MA GPU-kernel product path is:

- 10-row single-pass
- exact `omega`
- replay skipped

The current forced legacy diagnostic baseline is:

- 7-row search contract
- finalist replay enabled

Do not analyze this repo as if the old two-pass replay-backed path were still the default product route for this benchmark family. It is not.

### Current benchmark surface

Primary benchmark family:

- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/...`
- `objective=Sharpe`
- `top_k=50`
- `heatmap_bins=64`

Current recorded timings from `research/MA review/double_ma_validated_implementation_tracking_plan_2026-03-23.md`:

- preferred path, 10-row single-pass exact `omega`
  - `search_only`: `77.23 ms`
  - `search_plus_heatmap`: `77.24 ms`
  - `search_plus_finalist_recompute`: `151.14 ms`
  - `end_to_end_product`: `77.13 ms`
- forced legacy baseline, 7-row + replay
  - `search_only`: `77.23 ms`
  - `search_plus_heatmap`: `77.26 ms`
  - `search_plus_finalist_recompute`: `151.88 ms`
  - `end_to_end_product`: `152.01 ms`

Interpretation:

- the product path is now effectively one-pass
- adding the current extra product work for the preferred path is almost free on this benchmark surface
- this suggests the next performance question is not replay removal, but useful-work density inside the single pass

### Current 10-row metric contract

Current stored per-candidate row in `crates/app_core/src/gpu_backtest_kernel.rs:136-148`:

- `0`: pnl
- `1`: trades
- `2`: max_dd
- `3`: mean
- `4`: std
- `5`: exposure
- `6`: net_exposure
- `7`: downside_deviation
- `8`: ulcer_index
- `9`: omega

Derived downstream metrics already available from this row shape:

- `sharpe` from `mean / std`
- `calmar` from `pnl / max_dd`
- `volatility` from `std`
- `martin_ratio` from `pnl / ulcer_index`

Export headers in both `double_ma.rs` and `gpu_backtest_kernel.rs` already expose:

- `pnl`
- `sharpe`
- `max_dd`
- `downside_deviation`
- `ulcer_index`
- `omega`
- `martin_ratio`
- `trades`
- `exposure`
- `net_exposure`
- `score`

### Important current mismatch

The backend already has enough state to make `Sortino` and `Omega` highly plausible GPU objectives, but they are still blocked as kernel objectives.

Current evidence:

- `search_metrics_count_for_optimization(...)` maps `Sortino` and `Omega` to full rows in `crates/app_core/src/gpu_backtest_kernel.rs:3583-3614`
- the kernel route still rejects `ObjectiveKind::Sortino | ObjectiveKind::Omega` in `crates/app_core/src/gpu_backtest_kernel.rs:6396-6424`
- preflight still blocks them in `crates/app_core/src/double_ma.rs:9609-9612`
- CPU-side objective scoring already supports them in `crates/app_core/src/double_ma.rs:471-482`

### Current device-side reduction / filtering shape

The shared runtime already includes:

- dataset-keyed reusable return buffers via `ensure_log_and_simple_returns(...)` in `crates/app_core/src/gpu_backtest_kernel.rs:3155-3173`
- device-side row reduction via `reduce_metric_rows_topk_on_device(...)` in `crates/app_core/src/gpu_backtest_kernel.rs:4220-4398`
- device-side top-k kernels in `crates/app_core/src/kernels/double_crossover.cu:4997-5279`
- device-side heatmap score updates in `crates/app_core/src/kernels/double_crossover.cu:4552-4609`
- global tile-to-global-topk merge machinery in `crates/app_core/src/gpu_backtest_kernel.rs:6732-6752` and `crates/app_core/src/gpu_backtest_kernel.rs:8382-8450`

Current on-device hard constraints are still narrow:

- `min_trades`
- `max_drawdown`

Constraint logic is in `crates/app_core/src/kernels/double_crossover.cu:4491-4509`.

Important adjacent point:

- if constraints cannot be enforced on device, the runtime falls into `host_filter_mode`
- that path copies and decodes full tiles on the CPU side before filtering
- see `crates/app_core/src/gpu_backtest_kernel.rs:6732-6752` and `crates/app_core/src/gpu_backtest_kernel.rs:7685-7728`

This means better on-device constraints are not only a feature improvement; they are also an anti-readback optimization.

### Current single-pass kernel state

The main flex-metrics kernel already carries substantial exact running state in registers in `crates/app_core/src/kernels/double_crossover.cu:3434-3765`, including:

- position state
- log equity
- drawdown state
- trade count
- exposure sums
- Welford mean / variance state
- downside accumulation
- ulcer accumulation
- positive-return and negative-return mass for `omega`

This matters because it means many additional metrics or ranking rules are not new passes. They are extra arithmetic on top of values already touched in the current loop.

---

## High-Value Candidate Opportunity Set

This is the candidate set GPT-5.4 Pro should analyze and rank.

### A. Multi-objective ranking from the same 10-row pass

The most interesting direction is likely not just "extra metrics", but "better use of the metrics already available".

Potential products:

- top-k by Sharpe
- top-k by PnL
- top-k by Calmar
- top-k by Omega
- top-k by a composite score
- top-k by a robustness-aware score

Potential ranking modes:

- multiple separate exact top-k lists from the same metric rows
- one composite scalar score
- lexicographic ranking
- epsilon-constraint ranking
- Pareto shortlist followed by secondary ranking
- weighted rank aggregation

Why this looks promising:

- the expensive part is the bar walk
- once the 10-row metric rows exist, extra reduction work should be much cheaper than another optimization pass
- the current reduction infrastructure is already present

Relevant code:

- `crates/app_core/src/gpu_backtest_kernel.rs:4220-4398`
- `crates/app_core/src/gpu_backtest_kernel.rs:8171-8197`
- `crates/app_core/src/kernels/double_crossover.cu:4418-4457`
- `crates/app_core/src/kernels/double_crossover.cu:4997-5279`

### B. Composite scoring alongside single-metric ranking

This is a special case of multi-objective ranking, but important enough to isolate.

This should be treated as an additive capability, not a replacement for existing single-metric objective modes.

Real product use case:

- "find candidates with high Sharpe, but also acceptable drawdown, decent Omega, and not absurdly low trade count"

Possible composite families:

- weighted sum after scale normalization
- weighted geometric mean after positivity transforms
- weighted harmonic mean for "balanced" candidates
- weighted Tchebycheff / max-penalty formulations
- rank-based aggregation
- piecewise penalty functions
- constrained score where some metrics are hard filters and others are soft penalties

This problem is more mathematical than implementation-heavy. The implementation can be simple once the score family is chosen.

### C. Expand on-device hard constraints

The current 10-row pass already makes more constraint families possible than the runtime currently uses.

There are really two subfamilies here.

#### C1. Cheap 7-row constraints

These are attractive because they can often stay on the lighter search contract:

- `min_sharpe`
- `max_volatility`
- `min_exposure`
- `max_exposure`
- `max_abs_net_exposure`

These use fields already available in the current 7-row base contract:

- trades
- max_dd
- mean
- std
- exposure
- net_exposure

#### C2. 10-row risk constraints

These require the richer product row:

- `max_downside_deviation`
- `max_ulcer_index`
- `min_omega`
- `min_sortino` once objective support is added
- `min_calmar`

Important design question:

- which constraints should be hard device-side filters?
- which should be score penalties instead?

Relevant code:

- `crates/app_core/src/gpu_backtest_kernel.rs:6732-6752`
- `crates/app_core/src/kernels/double_crossover.cu:4491-4509`

Secondary question:

- for each candidate constraint, is it worth promoting the row width for that run, or should it be left as a score penalty instead?

### D. Enable direct GPU `Sortino` and `Omega` objectives

This is likely lower-risk than inventing a new metric family because the current kernel already accumulates the required state.

Important nuance:

- `objective_score_f32(...)` in `crates/app_core/src/kernels/double_crossover.cu:4418-4457` currently supports `pnl`, `sharpe`, `-max_dd`, `calmar`, and `-volatility`
- extending it to `Sortino` and `Omega` looks conceptually straightforward
- but the route / blocker logic also needs to be updated

Relevant code:

- `crates/app_core/src/double_ma.rs:9609-9612`
- `crates/app_core/src/double_ma.rs:11397-11418`
- `crates/app_core/src/gpu_backtest_kernel.rs:3583-3614`
- `crates/app_core/src/gpu_backtest_kernel.rs:6396-6424`
- `crates/app_core/src/kernels/double_crossover.cu:4418-4457`

### E. Add more exact metrics that piggyback on the same walk

Potential additions:

- gross profit
- gross loss
- win count
- loss count
- profit factor
- expectancy
- average trade return
- turnover
- average holding time
- time in market by side

These are attractive if they can be used for:

- device-side constraints
- composite scoring
- export / UI diagnostics

However, the right question is not "can they be computed?" but:

- which subset gives the best decision value per register cost?
- which subset should be exported vs used only internally for scoring / filtering?

### F. Add stability summaries inside the same pass

Potential summaries:

- first half vs second half
- first third / middle third / final third
- long-only vs short-only
- regime bucket A vs regime bucket B
- pre-trend vs post-trend

Why it matters:

- this turns optimization into partial validation
- the result can be a more robust shortlist without another replay pass

### G. Richer heatmap aggregates

Current heatmap path is essentially:

- one score per cell
- `atomicMax`

Potential richer heatmap outputs:

- best score
- mean score
- count of valid candidates
- count of constraint-passing candidates
- best composite score
- best risk-adjusted score

This is lower priority than multi-objective top-k, but it may be valuable for the product and not require another bar walk.

### H. Adjacent host-readback reduction opportunities

These are not "more useful work inside the pass", but they are closely related to useful-work density:

- reduce or bypass `host_filter_mode` full-tile decode when richer constraints can stay on device, while preserving current behavior
- minimize final packed-row host decode when device-side shortlist materialization is already available
- reduce heatmap host materialization cost if heatmap remains part of product runs

This is relevant because the best mathematical design may depend on where filtering and shortlist materialization happen.

---

## Core Implementation Question

The central question is not:

- "what else can we compute?"

The central question is:

- "what is the most valuable extra exact work that can be added while staying in one pass, keeping memory traffic low, and preserving the current ~77 ms product timing envelope?"

That should be treated as a joint:

- algorithm design problem
- metric design problem
- GPU cost-model problem

---

## Unsolved Math / Optimization Problems

These are the main math-heavy problems I want GPT-5.4 Pro to solve or at least formalize well.

### Problem 1: Optimal composite score design

Given a metric vector

`m = (pnl, sharpe, max_dd, downside_dev, ulcer, omega, martin, trades, exposure, net_exposure)`

design one or more scalar scores `S(m)` that:

- are monotone in the "good" directions
- penalize pathological candidates
- are robust to scale differences and outliers
- are GPU-cheap to evaluate
- are stable enough for exact top-k selection

Specific asks:

- compare weighted-sum, weighted-rank, geometric-mean, harmonic-mean, Tchebycheff, epsilon-constraint, and Pareto-based formulations
- recommend at least 2 concrete score families that are practical for this codebase
- explain how to normalize or transform each metric so the score is not dominated by one unit scale
- account for clamping / infinities / zero-denominator cases

### Problem 2: Exact multi-objective top-k from one metric matrix

Suppose the kernel already emits one 10-row metric matrix for all candidates.

Question:

- what is the most efficient exact way to extract multiple useful shortlists from that matrix?

Examples:

- top-k by Sharpe
- top-k by Omega
- top-k by Calmar
- top-k by composite score
- Pareto-like shortlist

Specific asks:

- compare separate reductions, fused multi-head reductions, and Pareto-shortlist-plus-rerank
- provide a cost model for each
- recommend the best design for `N ~= 58,300`, `K = 50`, `T = 200,000`
- identify when extra reduction work remains negligible relative to the bar walk

### Problem 3: Optimal hard-constraint vs soft-penalty split

Given candidate metrics and a user preference surface, determine which conditions should be:

- hard device-side filters
- soft penalties inside the objective score
- pure post-hoc diagnostics

Specific asks:

- derive a decision rule or heuristic based on expected prune rate, metric reliability, and selection bias
- discuss the interaction between hard filters and top-k exactness
- recommend a default split for this backtest optimization context

### Problem 4: Value density of extra state

Each additional exact metric or diagnostic consumes:

- registers
- arithmetic
- possibly row width
- possibly more reduction work

Define a cost model such as:

`cost ~= a * delta_regs + b * delta_flops + c * delta_global_writes + d * delta_atomics + e * delta_d2h`

and use it to rank candidate work items.

Specific asks:

- classify each proposed addition as register-only, reduction-only, row-width-increasing, or host-traffic-increasing
- estimate which class is most dangerous on an Ada-class GPU / RTX 4090-like device
- recommend the best "useful work portfolio" under a strict no-regression target
- distinguish especially between:
  - 7-row-compatible extra work
  - 10-row-only extra work
  - work that triggers host-filter fallback if not implemented carefully

### Problem 5: Minimal sufficient state for the richest derived metric family

There are two related but different optimization targets:

- a minimal state for exact product outputs
- a minimal state for exact ranking / filtering / composite scoring

Question:

- what is the smallest exact carried state that supports the largest practical family of derived metrics and ranking rules?

Specific asks:

- distinguish stored output rows from latent running state inside the kernel loop
- determine whether the current 10-row output is close to optimal for downstream flexibility
- identify if some new statistics should exist only in registers for scoring/filtering rather than being exported

### Problem 6: Best stability segmentation under a fixed extra-state budget

Assume we can afford only a small amount of additional exact state.

Question:

- which segmentation scheme gives the most robustness information per unit cost?

Candidate segmentation families:

- first half / second half
- thirds
- long-only / short-only
- regime-conditioned buckets
- drawdown-conditioned buckets

Specific asks:

- recommend the best segmentation scheme for this codebase
- justify it in terms of information value and GPU cost
- explain whether the result should be used as a hard filter, a composite penalty, or a secondary display-only metric

### Problem 7: Exact one-pass enrichment vs approximate screening

The current insight is that some extra work inside one pass appears nearly free.

Question:

- under that condition, when should we still prefer approximate screening or multi-fidelity evaluation over exact one-pass enrichment?

Specific asks:

- compare exact one-pass enrichment against approximate multi-fidelity screening for this workload
- explain when adding more exact work to the current pass dominates screening
- explain when screening still wins

### Problem 8: Where should shortlist formation happen?

Given that some constraint and score combinations can be enforced on device while others currently trigger host filtering:

- what is the mathematically and architecturally best split between:
  - on-device hard filtering
  - on-device top-k reduction
  - host-side decode / rerank / reporting

Specific asks:

- identify which shortlist semantics belong entirely on device
- identify which composite or Pareto-like semantics are still practical on device
- recommend how much information should be copied back for exact product behavior

---

## Suggested Prompts For GPT-5.4 Pro

### Prompt 1: Main research prompt

You are reviewing the current Double MA shared GPU backtest optimization backend in this repository.

Important current state:

- the old replay-backed product path is no longer the default for the Sharpe/Volatility benchmark family
- the preferred path is now a 10-row single-pass product path with exact omega
- current benchmark timings are about 77.2 ms for both search_only and end_to_end_product on the main 58,300-pair / 200k-bar benchmark surface
- this suggests that modest additional work inside the same single pass may be nearly free while occupancy headroom remains

Your task:

1. Analyze what additional exact useful work should be added to the optimization stage.
2. Prioritize opportunities that reuse the current bar walk and avoid a second pass.
3. Focus especially on:
   - multi-objective ranking
   - composite scoring
   - expanded device-side constraints
   - direct Sortino/Omega objective support
   - extra exact metrics or diagnostics that are cheap to accumulate
4. All recommendations must be additive and backward-compatible.
5. Do not propose removing existing functionality, narrowing the current product contract, or replacing existing single-objective modes.
6. Treat this as both a GPU cost-model problem and a mathematical objective-design problem.
7. Propose an implementation order ranked by expected value per performance risk.
8. Explicitly identify which proposals are:
   - exact and low-risk
   - exact but medium-risk
   - approximate / optional
9. Provide concrete code touchpoints in the referenced files.

Also solve or at least formalize the math problems listed in this brief, especially the composite-score and multi-objective top-k questions.

### Prompt 2: Composite score prompt

Given the current exact per-candidate metric family

- pnl
- sharpe
- max_dd
- downside_deviation
- ulcer_index
- omega
- martin_ratio
- trades
- exposure
- net_exposure

design GPU-cheap composite scores that would be better product objectives than ranking by Sharpe alone.

Requirements:

- mathematically well-behaved
- scale-robust
- resistant to domination by one metric
- suitable for exact top-k selection
- realistic for implementation on the current GPU backend

Compare at least:

- weighted normalized sum
- geometric mean
- harmonic mean
- weighted ranks
- Tchebycheff scalarization
- epsilon-constraint / lexicographic formulations
- Pareto-shortlist-then-rerank

Recommend the best 2-3 families for this repository and explain why.

### Prompt 3: Useful-work portfolio prompt

Assume the current one-pass kernel already carries significant state in registers and that adding some extra arithmetic is cheap as long as it does not materially increase memory traffic or trigger occupancy collapse.

Construct a "useful-work portfolio" for the next optimization pass:

- which extra metrics to add
- which extra objectives to add
- which extra constraints to enforce on device
- which extra reduction outputs to return

Rank the portfolio by value density:

- user value
- downstream validation value
- expected performance cost
- implementation complexity
- regression risk

### Prompt 4: State-minimization prompt

Find the minimal exact carried state that supports the richest practical downstream family of:

- ranking objectives
- hard constraints
- exported diagnostics

in this Double MA shared GPU backend.

Distinguish carefully between:

- latent running state inside the kernel loop
- emitted per-candidate row fields
- host-derived metrics

Recommend whether the current 10-row output should stay fixed or be widened in a backward-compatible way. Do not propose removing or redefining existing exported fields.

---

## Files To Provide As Direct Context

### Required

- `research/MA review/double_ma_single_pass_extra_work_research_brief_2026-03-23.md`
  - this brief
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - shared runtime policy, metric-count policy, returns reuse, top-k reduction, constraint placement
- `crates/app_core/src/kernels/double_crossover.cu`
  - actual single-pass kernel loop, current register state, objective score logic, constraint kernels, top-k kernels, heatmap kernel
- `crates/app_core/src/double_ma.rs`
  - route policy, objective blockers, CPU-side objective semantics, benchmark-adjacent tests
- `crates/app_core/benches/double_ma_opt.rs`
  - benchmark harness and current contract-split benchmark surface
- `research/MA review/double_ma_validated_implementation_tracking_plan_2026-03-23.md`
  - current benchmark evidence and validated backend state

### Optional but useful

- `crates/app_core/src/double_ma_validate.rs`
  - helpful if the model wants to reason about carrying more robustness / validation-style summaries into optimization
- `gpt54_feature_review_map.md`
  - useful only if the model also needs surrounding review-map context; not required for the core technical analysis

---

## Concrete Code Touchpoints

Use these as the fastest anchors into the repo:

- `crates/app_core/src/gpu_backtest_kernel.rs:136-148`
  - 10-row metric layout
- `crates/app_core/src/gpu_backtest_kernel.rs:3155-3173`
  - dataset-keyed returns reuse
- `crates/app_core/src/gpu_backtest_kernel.rs:3583-3614`
  - search metrics-count policy
- `crates/app_core/src/gpu_backtest_kernel.rs:3618-3620`
  - replay-skip predicate
- `crates/app_core/src/gpu_backtest_kernel.rs:4220-4398`
  - device-side row reduction helper
- `crates/app_core/src/gpu_backtest_kernel.rs:6396-6424`
  - current Sortino/Omega route rejection
- `crates/app_core/src/gpu_backtest_kernel.rs:6732-6752`
  - device-vs-host constraint and global-topk mode selection
- `crates/app_core/src/gpu_backtest_kernel.rs:7685-7728`
  - host-filter path when constraints are not enforced on device
- `crates/app_core/src/gpu_backtest_kernel.rs:8382-8450`
  - final global top-k decode and heatmap D2H path
- `crates/app_core/src/kernels/double_crossover.cu:3434-3765`
  - main flex-metrics time-walk and current carried state
- `crates/app_core/src/kernels/double_crossover.cu:4418-4457`
  - current objective score function for the shared kernel path
- `crates/app_core/src/kernels/double_crossover.cu:4491-4509`
  - current device-side constraint function
- `crates/app_core/src/kernels/double_crossover.cu:4552-4609`
  - current heatmap scoring kernel
- `crates/app_core/src/kernels/double_crossover.cu:4997-5279`
  - current row-selection / top-k kernels
- `crates/app_core/src/double_ma.rs:471-482`
  - CPU-side objective semantics
- `crates/app_core/src/double_ma.rs:9609-9612`
  - kernel blockers for Sortino/Omega
- `crates/app_core/src/double_ma.rs:11397-11418`
  - blocker test
- `crates/app_core/benches/double_ma_opt.rs:589-655`
  - contract-split benchmark definition
- `research/MA review/double_ma_validated_implementation_tracking_plan_2026-03-23.md:374-380`
  - benchmark capture table

---

## What I Want The Next Model To Deliver

The ideal response from GPT-5.4 Pro is another markdown document that:

- reflects the current single-pass 10-row repo state accurately
- does not reason from the old default replay-backed product path
- stays strictly additive and backward-compatible
- ranks the candidate extra-work ideas by value and risk
- gives mathematically serious guidance on composite scoring and multi-objective ranking
- recommends an implementation order
- clearly separates:
  - exact / low-risk opportunities
  - exact / medium-risk opportunities
  - approximate / optional opportunities
- references the most relevant code touchpoints directly
