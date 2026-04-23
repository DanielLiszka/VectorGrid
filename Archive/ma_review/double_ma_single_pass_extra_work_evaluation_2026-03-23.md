# Double MA Single-Pass Extra-Work Evaluation
Date: 2026-03-23

All recommendations below are additive and backward-compatible. They preserve the current 10-row single-pass product path, the existing exported product fields, the existing single-objective modes, the current diagnostic replay baseline / benchmark variants, and CPU fallback behavior. New capabilities are assumed to be opt-in by request, env flag, or explicit mode selection.

## Current State

### What is directly visible in the uploaded files

1. The current preferred `Sharpe` / `Volatility` GPU-kernel product route is the 10-row single-pass path, not the old replay-default path.
   - `gpu_backtest_kernel.rs:136-148` defines the current 10-row contract:
     - `0 pnl`
     - `1 trades`
     - `2 max_dd`
     - `3 mean`
     - `4 std`
     - `5 exposure`
     - `6 net_exposure`
     - `7 downside_deviation`
     - `8 ulcer_index`
     - `9 omega`
   - `search_metrics_count_for_optimization(...)` widens `Sharpe` / `Volatility` to `PRODUCT_REDUCED_METRICS_COUNT = 10` when `VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS` is unset, via `kernel_product_reduced_rows_enabled_for_objective(...)` in `gpu_backtest_kernel.rs:3346-3364` and `gpu_backtest_kernel.rs:3583-3614`.
   - Finalist replay is skipped whenever the current GPU product metrics can already be decoded from the search row, via `can_decode_current_gpu_product_metrics_without_recompute(...)` and the recompute gate in `gpu_backtest_kernel.rs:3618-3620` and `gpu_backtest_kernel.rs:8463-8479`.

2. The contract-split benchmark intentionally preserves legacy rows for the search-only / replay-diagnostic variants.
   - In `double_ma_opt.rs:619-655`, the benchmark family `double_ma/gpu_kernel_contract_split/alma_alma/200k` uses four variants.
   - `search_only`, `search_plus_heatmap`, and `search_plus_finalist_recompute` pass `Some(false)` into `run_with_product_reduced_rows_env(...)`, while `end_to_end_product` leaves the default behavior active.
   - That means the benchmark is deliberately comparing a forced legacy search contract against the default product route, rather than benchmarking a 10-row search-only mode as the default control.

3. Current benchmark evidence says the product route is effectively one-pass on the main ALMA / ALMA surface.
   - The brief and tracker describe the current benchmark surface as `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/...`, `objective=Sharpe`, `top_k=50`, `heatmap_bins=64`.
   - The current reported timings in `double_ma_single_pass_extra_work_research_brief_2026-03-23.md` and `double_ma_validated_implementation_tracking_plan_2026-03-23.md` are approximately:
     - preferred path: `search_only ≈ 77.23 ms`, `search_plus_heatmap ≈ 77.24 ms`, `search_plus_finalist_recompute ≈ 151.14 ms`, `end_to_end_product ≈ 77.13 ms`
     - forced legacy baseline: `search_only ≈ 77.23 ms`, `search_plus_heatmap ≈ 77.26 ms`, `search_plus_finalist_recompute ≈ 151.88 ms`, `end_to_end_product ≈ 152.01 ms`

4. The backend already computes enough exact running state to support more objectives and constraints than are currently exposed.
   - The main time-walk kernel in `double_crossover.cu:3434-3765` already carries:
     - `log_eq`, `log_peak`, `max_log_dd`
     - `trades`
     - `sum_abs_pos`, `sum_pos`
     - Welford `mean` / `m2`
     - downside accumulation
     - ulcer accumulation
     - `sum_pos_r`, `sum_neg_abs_r` for exact `omega`
   - The current write path emits the 10-row product contract without needing a second pass in `double_crossover.cu:3747-3774`.

5. The current kernel reduction path is already device-centric.
   - Dataset-keyed returns reuse already exists through `ensure_log_and_simple_returns(...)` in `gpu_backtest_kernel.rs:3155-3173`.
   - Device-side row reduction already exists in `reduce_metric_rows_topk_on_device(...)` in `gpu_backtest_kernel.rs:4220-4398`.
   - The corresponding top-k kernels already exist in `double_crossover.cu:4997-5279`.
   - The global tile-to-global-top-k merge path is already wired in `gpu_backtest_kernel.rs:8171-8197` and finalized in `gpu_backtest_kernel.rs:8382-8450`.

6. Current device-side constraints are much narrower than the row state.
   - The public constraint struct in `double_ma.rs:176-180` only exposes `min_trades` and `max_drawdown`.
   - The host-side `metrics_pass_constraints(...)` helper in `double_ma.rs:793-807` only checks those two constraints.
   - The device-side constraint helper `passes_constraints_f32(...)` in `double_crossover.cu:4491-4509` also only checks those two constraints.
   - `gpu_backtest_kernel.rs:6732-6752` shows that unsupported device-side filtering leads toward `host_filter_mode`, and `gpu_backtest_kernel.rs:7685-7728` shows the consequence: full tile copy + decode on the host before filtering.

7. `Sortino` and `Omega` are already mathematically present but still route-blocked.
   - CPU-side objective semantics already support them in `double_ma.rs:471-482`.
   - `search_metrics_count_for_optimization(...)` already maps both to full rows in `gpu_backtest_kernel.rs:3596-3604`.
   - The GPU-kernel route still rejects them in `gpu_backtest_kernel.rs:6396-6424`.
   - Preflight still blocks them in `double_ma.rs:9609-9612`, with a test confirming that blocker in `double_ma.rs:11397-11418`.
   - `objective_score_f32(...)` in `double_crossover.cu:4418-4457` currently supports only:
     - `pnl`
     - `sharpe`
     - `-max_dd`
     - `calmar`
     - `-volatility`

8. The current heatmap path is intentionally simple.
   - `double_crossover.cu:4552-4609` performs one score per cell using `atomicMax`.
   - The host finalization path in `gpu_backtest_kernel.rs:8433-8460` only materializes one scalar heatmap value per cell.

9. The validation layer already computes much richer robustness diagnostics, but only after optimization.
   - `double_ma_validate.rs:2307-2421` computes a substantial regime-slice robustness summary using volatility tertiles and trend/range buckets.
   - `double_ma_validate.rs:2470-2588` computes friction-stress summaries by re-evaluating finalists under stressed cost scenarios.
   - Those files are useful because they show what the downstream product actually values, but they also show that not all validation logic belongs inside the optimization pass.

### Inferences that follow from the visible state

1. The main optimization opportunity has moved from “remove replay” to “increase useful work density inside the current single pass”.
   - This is the central state change.
   - The key observation is not that 10 rows are free in the abstract.
   - It is that the current benchmark shows almost no measurable gap between forced-legacy `search_only` and default `end_to_end_product`, while the forced replay path remains expensive.

2. The safest additions are not new per-bar statistics.
   - The safest additions are score / constraint / shortlist semantics that reuse the existing 10-row matrix and existing reduction machinery.

3. The most dangerous regressions are still:
   - extra per-thread registers in the dominant backtest kernel
   - row-width growth that increases tile footprint enough to change tiling / occupancy behavior
   - unsupported constraints that accidentally push filtering back to full-tile host decode

4. Because the benchmark harness intentionally preserves forced-legacy rows for `search_only`, “nearly free extra work” should be interpreted as:
   - extra work on the default 10-row product path is currently cheap enough that it does not visibly move the `end_to_end_product` timing envelope on this surface
   - not as a claim that every possible extra metric will always be free across every MA family and every tile regime

---

## Ranked Recommendation List

### Summary table

| Rank | Proposal | Class | Expected value | Perf risk | Impl complexity | Main cost vector |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Unblock direct GPU `Sortino` and `Omega` objectives | Exact / low-risk | Very high | Low | Low | Reduction-only / routing |
| 2 | Expand device-side hard constraints on current row state | Exact / low-risk | Very high | Low | Low-Med | Reduction-only; lowers host readback risk |
| 3 | Add opt-in composite score families over the current row | Exact / low-risk | Very high | Low | Med | Reduction-only |
| 4 | Add exact multi-head top-k + shortlist union from the same matrix | Exact / medium-risk | High | Low | Med | Extra reductions; tiny shortlist readback |
| 5 | Keep shortlist formation device-first; use host rerank only on a tiny union shortlist | Exact / medium-risk | High | Low | Med | Shrinks D2H to shortlist-sized traffic |
| 6 | Add derived metric vocabulary before adding new carried state | Exact / low-risk | Medium-High | Very low | Low | No new row width |
| 7 | Add one minimal stability summary family: half-split robustness | Exact / medium-risk | Medium | Med | Med | Register growth in backtest kernel |
| 8 | Add trade-level metrics only as opt-in extended work, not as first-wave default state | Exact / medium-risk | Medium | Med-High | Med-High | Register growth + row growth if exported |
| 9 | Add richer heatmap aggregates only after shortlist/ranking work lands | Exact / optional | Low-Med | Low-Med | Med | Atomics / extra heatmap buffers |
| 10 | Pareto / weighted-rank / epsilon-front exploration on full-grid device output | Approximate / optional | Medium | Low runtime, higher semantic risk | Med-High | Semantics / explainability more than FLOPs |

### 1) Unblock direct GPU `Sortino` and `Omega` objectives
**Class:** exact / low-risk

**Why it ranks first**

This is the cleanest “already-paid-for” addition in the repo.

The kernel already carries the exact sufficient statistics needed for both objectives:
- `mean` and `downside_deviation` for `Sortino`
- `sum_pos_r` and `sum_neg_abs_r` for exact `omega`

The CPU semantics already exist. The search metric-count policy already widens to full rows for these objectives. The missing part is route unification, not metric accumulation.

**Visible code basis**
- CPU objective semantics: `double_ma.rs:471-482`
- metric-count policy: `gpu_backtest_kernel.rs:3583-3614`
- current route rejection: `gpu_backtest_kernel.rs:6396-6424`
- preflight blocker: `double_ma.rs:9609-9612`
- current device score function: `double_crossover.cu:4418-4457`

**Recommended implementation shape**
- Extend `objective_score_f32(...)` with:
  - `objective == 5 -> Sortino`
  - `objective == 6 -> Omega`
- Reuse the same clamp / zero-denominator semantics already used in CPU finalization and host decode.
- Update the `objective_i` mapping in `gpu_backtest_kernel.rs`.
- Remove the corresponding route and preflight blockers in `double_ma.rs` and `gpu_backtest_kernel.rs`.

**Cost model**
- `Δregs`: none in the backtest kernel
- `Δflops`: negligible; only extra arithmetic in the reduction score function
- `Δglobal_writes`: none
- `Δatomics`: none
- `Δd2h`: none

**Expected performance effect**
- Essentially zero on the current benchmark surface
- High product value because it closes an obvious exactness / API mismatch

**Important note**
- This should not change existing single-objective behavior.
- It should add new objective modes only.

---

### 2) Expand device-side hard constraints on current row state
**Class:** exact / low-risk

**Why it ranks second**

This is both a feature win and a performance-protection win.

Right now, the code already has a narrow device-side constraint lane, and the runtime structure makes it clear that unsupported constraint filtering pushes work toward host-side full-tile decode. That is exactly the wrong direction if the goal is to increase useful work density without increasing readback.

**Visible code basis**
- public constraint surface: `double_ma.rs:176-180`
- host constraint helper: `double_ma.rs:793-807`
- device-side constraint helper: `double_crossover.cu:4491-4509`
- device-vs-host filtering split: `gpu_backtest_kernel.rs:6732-6752`
- host-filter decode path: `gpu_backtest_kernel.rs:7685-7728`

**Best additive constraint set**

#### 2A. Cheap constraints that do not need more than the current 7-row search contract
These should land first.

1. `min_sharpe`
   - derives from `mean / std`
   - uses row slots 3 and 4

2. `min_calmar`
   - important correction: this is **not** actually 10-row-only
   - the GPU score path already computes exact `calmar` from the reduced search row by converting search-space `log_eq` / `max_log_dd` in `objective_score_f32(...)`
   - so `min_calmar` is a cheap 3-row / 7-row-compatible hard constraint

3. `max_volatility`
   - direct from `std`

4. `min_exposure`
5. `max_exposure`
   - direct from `exposure`

6. `max_abs_net_exposure`
   - direct from `abs(net_exposure)`

#### 2B. 10-row-only hard constraints
These are effectively free on the current default product path for `Sharpe` / `Volatility`, because that route already emits 10 rows.

1. `max_downside_deviation`
2. `max_ulcer_index`
3. `min_omega`
4. `min_sortino`

**Default hard vs soft split**
- Hard by default:
  - `min_trades`
  - `max_drawdown`
  - `min_calmar`
  - `max_volatility`
  - `min_exposure` / `max_exposure`
  - `max_abs_net_exposure`
  - `max_ulcer_index`
- Hard only when the user truly means it:
  - `min_sharpe`
  - `min_sortino`
  - `min_omega`
- Better as soft penalties unless explicitly requested as hard:
  - “prefer higher Sharpe”
  - “prefer lower drawdown below the hard cap”
  - “prefer moderate exposure”
  - “prefer more trades once the floor is met”

**Reasoning**
- Risk caps and exposure bounds are acceptability constraints.
- Ratio floors are noisier and more selection-bias-prone; they should usually become hard only after `min_trades` or similar sample-quality floors are already active.

**Cost model**
- `Δregs`: none in the backtest kernel if constraints use existing row values
- `Δflops`: tiny in reduction kernels
- `Δglobal_writes`: none
- `Δatomics`: none
- `Δd2h`: can decrease materially by avoiding unsupported host filtering

**Expected performance effect**
- Little or no direct speedup when constraints are absent
- High upside when constraints are present, because exact constrained top-k stays on device

**Implementation touchpoints**
- Extend `DoubleMaConstraints` in `double_ma.rs`
- Extend `metrics_pass_constraints(...)`
- Extend `passes_constraints_f32(...)`
- Thread new constraint args through `gpu_backtest_kernel.rs:6732-6752` and reduction launch plumbing

---

### 3) Add opt-in composite score families over the current row
**Class:** exact / low-risk

**Why it ranks third**

The row already contains enough exact information to rank candidates in a much more product-useful way than “Sharpe only”, without touching the bar walk. This is the highest-value mathematical addition after direct objective parity and richer constraints.

The critical design choice is to keep composite scoring:
- additive
- opt-in
- scalarized enough for exact device top-k
- based on fixed transforms / aspiration levels, not fragile run-relative normalization by default

**Do this in two families**

#### Family A: constrained normalized additive utility
This should be the first composite family.

Let `g_i(m)` be monotone transformed utilities and `p_j(m)` be soft penalties.

A practical form is:

```text
S_add(m) =
    w_sh * u_sh(sharpe)
  + w_so * u_so(sortino)
  + w_ca * u_ca(calmar)
  + w_om * u_om(omega)
  + w_ma * u_ma(martin_ratio)
  + w_tr * u_tr(trades)
  - w_dd * p_dd(max_dd)
  - w_ul * p_ul(ulcer_index)
  - w_ex * p_ex(exposure, net_exposure)
```

A good GPU-cheap transform set is:

```text
u_sh(x) = clip(asinh(x / s_sh), -c, c)
u_so(x) = clip(asinh(x / s_so), -c, c)
u_ca(x) = clip(asinh(x / s_ca), -c, c)
u_ma(x) = clip(asinh(x / s_ma), -c, c)
u_om(x) = clip(log(max(x, eps)), -c, c)

p_dd(x) = clip(log1p(x / t_dd), 0, c)
p_ul(x) = clip(log1p(x / t_ul), 0, c)

u_tr(x) = min(1, log1p(x) / log1p(t_tr))

p_ex(expo, net) =
    relu(e_min - expo) / t_ex +
    relu(expo - e_max) / t_ex +
    relu(abs(net) - n_max) / t_net
```

**Why this family is good here**
- exact scalar objective
- robust to outliers through `asinh` / `log1p`
- monotone in the right directions
- cheap to compute in the existing score function
- compatible with device top-k

**What not to do by default**
- do not normalize with run-wide z-scores in the first version
- that adds another layer of reduction semantics and also makes the score depend on the candidate distribution of the current run, which is not always desirable

#### Family B: augmented Tchebycheff / worst-deficit scalarization
This should be the second composite family.

Let transformed utilities `q_i(m) in [0,1]` and aspiration levels `a_i in [0,1]`.

Define deficiencies:

```text
d_i(m) = max(0, a_i - q_i(m))
```

Then score by:

```text
S_tch(m) = -max_i (lambda_i * d_i(m)) - eps * sum_i (lambda_i * d_i(m)) + beta * q_primary(m)
```

**Why this family is good here**
- much less compensatory than a weighted sum
- strongly punishes a candidate whose weakest dimension is bad
- better aligned with “balanced good candidate” product intent
- still cheap: just `max`, `sum`, clamps, and a few transforms

**When to use it**
- when the user wants “no glaring weakness” rather than “best average score”
- especially good when `max_dd`, `ulcer_index`, and `omega` must all be decent simultaneously

**Recommended first-wave composite modes**
1. `BalancedRiskAdjusted7`
   - only uses values available from the 7-row contract
   - intended for lighter reduced-row modes if needed

2. `BalancedRiskAdjusted10`
   - uses the full 10-row family
   - intended for the current default 10-row product path

3. `SharpeWithGuardrails`
   - mostly Sharpe, but with smooth penalties on drawdown / ulcer / omega / trade scarcity / exposure violations
   - easiest product-facing default

**Cost model**
- `Δregs`: none in the backtest kernel if the score is computed only in reduction / selection
- `Δflops`: tiny relative to the bar walk
- `Δglobal_writes`: none
- `Δatomics`: none unless used in richer heatmaps
- `Δd2h`: none

**Key implementation choice**
- Composite score should live in the score / reduction layer, not in the stored 10-row output
- that keeps the base row stable and pushes the new capability into the cheapest part of the pipeline

**Touchpoints**
- `double_crossover.cu:4418-4457` for score evaluation
- `gpu_backtest_kernel.rs:4220-4398` for device reductions
- `double_crossover.cu:4997-5279` for top-k kernels
- optionally `double_crossover.cu:4552-4609` for heatmap score modes

---

### 4) Add exact multi-head top-k + shortlist union from the same matrix
**Class:** exact / medium-risk

**Why it ranks fourth**

The most interesting opportunity is not just “one better scalar score”. It is “one expensive bar walk, several exact shortlists”.

Given `N ≈ 58,300` and `K = 50`, this is already in the regime where extra row reductions are very cheap relative to the bar walk.

**Recommended exact design**
Run **separate exact device reductions** for a small head set, for example:
- `Sharpe`
- `Omega`
- `Calmar`
- `BalancedRiskAdjusted10`
- optionally `Pnl` or `Martin` if/when exposed

Then optionally form:
- per-head exact top-k lists
- the deduplicated union of those lists
- a final reranked shortlist over that union

**Why separate reductions are the right first design**
- they reuse the existing kernels almost unchanged
- they keep each head exact
- they avoid inflating the backtest kernel
- the read volume is tiny

For intuition:
- one 10-row metric matrix over `58,300` candidates is only about `2.22 MiB`
- four separate head reductions read about `8.9 MiB` total from that matrix
- that is tiny compared with the cost of walking `200,000` bars per candidate in the main kernel

The scratch buffers are also tiny:
- `ceil(58,300 / 512) = 114` blocks
- candidate list length per reduction is about `114 * 50 = 5,700`
- that is only tens of kilobytes of integer scratch

**Why not fused multi-head first**
A fused multi-head reducer saves only a few extra MiB of metric-row reads on this benchmark, but costs:
- more code complexity
- more shared-memory and register pressure in the reduction kernels
- more brittle maintenance

That trade does not look favorable for the first implementation wave.

**Why not exact full Pareto on device first**
Exact Pareto extraction over `N = 58,300` and roughly 5-10 objectives is not the right first move:
- semantics are unstable unless objective directions and tolerances are very carefully defined
- exact front size can become large
- exact implementation cost is much higher than “separate exact heads + union”

**Best additive product behavior**
Add an opt-in mode that returns:
- the existing primary `best` / `top`
- plus `extra_shortlists` for auxiliary heads
- plus an optional `shortlist_union` for downstream rerank / explanation

**Cost model**
- `Δregs`: none in the dominant backtest kernel
- `Δflops`: small and isolated in the reduction stage
- `Δglobal_writes`: small extra shortlist buffers
- `Δatomics`: none
- `Δd2h`: only if the union shortlist is materialized to host; still tiny at `O(H*K)`

**Touchpoints**
- `gpu_backtest_kernel.rs:4220-4398`
- `gpu_backtest_kernel.rs:8171-8197`
- `gpu_backtest_kernel.rs:8382-8450`
- `double_crossover.cu:4997-5279`

---

### 5) Keep shortlist formation device-first; use host rerank only on a tiny union shortlist
**Class:** exact / medium-risk

**Why it ranks fifth**

This is the best architecture split for the current codebase.

The core rule should be:

- if a shortlist semantic is a scalar function of the existing row, do it entirely on device
- if a shortlist semantic is non-scalar, exploratory, or interaction-heavy, do a tiny host rerank over a device-formed union shortlist
- never fall back to full tile host decode unless the user explicitly asked for `include_all` / export behavior

**Best split**

#### On device
These belong entirely on device:
- existing single-objective exact top-k
- `Sortino` / `Omega` exact top-k
- composite scalar exact top-k
- exact hard filtering on metrics already present in the row
- per-head shortlist extraction
- union shortlist formation

#### On host, but only after device narrowing
These belong on host over a tiny shortlist:
- weighted-rank aggregation
- Pareto-like explanation modes
- interactive weight changes
- explainability / reason codes / final presentation ordering

**Why this is mathematically right**
If a constraint is semantically hard, and it is not enforced before top-k, the returned `K` can fail to contain the true best feasible candidates.

So:
- hard constraints must be enforced before or during device top-k
- soft / exploratory semantics can be deferred to host once the feasible union shortlist is small

**Why this is architecturally right**
`gpu_backtest_kernel.rs:7685-7728` shows the wrong pattern:
- copy full tile
- decode full tile
- host filter afterward

The right additive direction is:
- extend the device semantics that already exist
- shrink host work to `O(K)` or `O(H*K)`

**Recommended additive mode**
Add a hybrid multi-objective shortlist mode with:
- device hard filters
- device per-head top-k
- device or host union dedup
- host rerank only on union

This keeps current exact product behavior untouched while adding a much stronger “discovery” mode.

---

### 6) Add derived metric vocabulary before adding new carried state
**Class:** exact / low-risk

**Why it ranks sixth**

The repo already contains more useful exact metrics than it currently exploits.

Before adding any new backtest-kernel state, the backend should expose the zero-new-state derivatives that are already available from the current row.

**Highest-value derived metrics / diagnostics from the current row**

1. `sortino = mean / downside_deviation`
   - already derivable
   - already supported on CPU

2. `calmar = pnl / max_dd`
   - already derivable
   - already used in current objective scoring

3. `martin_ratio = pnl / ulcer_index`
   - already derivable
   - already exported downstream

4. `abs_net_exposure = abs(net_exposure)`
   - zero-cost and useful as both score penalty and hard bound

5. `long_exposure = 0.5 * (exposure + net_exposure)`
6. `short_exposure = 0.5 * (exposure - net_exposure)`
   - these are exact consequences of the current exposure bookkeeping for `pos in {-1, 0, 1}`
   - they are useful for side-balance constraints and diagnostics
   - they do **not** require widening the 10-row contract

7. `directional_bias = abs(net_exposure) / max(exposure, eps)`
   - useful diagnostic for “mostly one-sided” strategies

**Recommendation**
- add these first to the score / constraint vocabulary
- do not widen the row for them
- only add explicit exported fields later, and only behind additive / opt-in surfaces if compatibility matters

---

### 7) Add one minimal stability summary family: half-split robustness
**Class:** exact / medium-risk

**Why it ranks seventh**

The validation file makes it clear that the downstream product cares about robustness, but the current validation summaries are too heavy to move wholesale into the optimization pass.

The best first stability addition is therefore not full validation logic.
It is one very cheap, exact, in-pass summary.

**Recommended scheme: first half / second half**
This is the best stability segmentation under a strict state budget.

**Why this is the best first choice**
- no new input series
- deterministic split
- directly measures out-of-time degradation
- much cheaper than thirds
- much simpler than regime-conditioned or drawdown-conditioned buckets
- more universally meaningful than long-only vs short-only, which can collapse for long-only strategies

**What to accumulate**
Do not carry full duplicate diagnostics unless needed.
Carry only the minimal duplicated accumulators required to derive a small summary set, for example per half:
- `log_eq`
- `mean`, `m2`
- `sum_neg_sq`
- `ulcer_sum_sq`
- `max_log_dd`
- `trades`

From that, derive:
- half `pnl`
- half `sharpe`
- half `sortino`
- half `ulcer_index`
- half `max_dd`

Then build one or two stability scalars such as:
- `worst_half_sharpe = min(sharpe_1, sharpe_2)`
- `half_gap_sharpe = abs(sharpe_1 - sharpe_2)`
- `worst_half_pnl = min(pnl_1, pnl_2)`

**How to use it**
- default: soft penalty or secondary ranking key
- not default hard filter
- optional display metric if exposed downstream

**Why not thirds first**
Thirds are information-richer, but they roughly triple the duplicated accumulator footprint and are much more likely to move register pressure.

**Why not regime buckets first**
The validation layer already shows that a richer regime summary wants:
- step-return arrays
- volatility tertiles
- spread-threshold logic
- trend/range bucketing

That is too expensive and too specialized for a first in-pass addition.

---

### 8) Add trade-level metrics only as opt-in extended work, not as first-wave default state
**Class:** exact / medium-risk

**Why it ranks eighth**

Metrics such as:
- gross profit
- gross loss
- win count
- loss count
- profit factor
- expectancy
- average holding time

are all useful, but they are **not** as free as the current row-derived family.

They require trade-level bookkeeping, not just bar-return bookkeeping.

**Why that matters in this kernel**
The current backtest kernel already tracks:
- position
- trade count
- trade-level log return only in the risk-managed path

But it does **not** currently maintain the full per-trade aggregates needed for exact trade-distribution metrics across all exit paths.

To do this correctly across:
- `allow_flip`
- `trade_on_next_bar`
- stop-loss / take-profit
- regime modes
- long-only vs signed exposure

you need additional state such as:
- current trade PnL
- current trade holding bars
- sum of winning trades
- sum of losing trades
- win / loss counts

That is a meaningful register and branching increase.

**Recommendation**
- do not spend the first-wave performance budget here
- if these metrics become important, add them only in one of two forms:
  1. an explicit opt-in extended row mode with the first 10 fields frozen
  2. finalist-only recompute / validation diagnostics, not full-grid default state

**Best candidate if this becomes necessary later**
The most useful later extension would likely be the trade quartet:
- `gross_profit`
- `gross_loss`
- `wins`
- `losses`

because that supports:
- profit factor
- win rate
- expectancy
- average win / loss

without needing many more exported fields.

---

### 9) Add richer heatmap aggregates only after shortlist / ranking work lands
**Class:** exact / optional

**Why it ranks ninth**

The current heatmap cost appears tiny on the benchmark surface, but heatmap enrichment still has lower product value than better constraints, better scores, and better shortlist formation.

**Useful exact additions**
- count of valid candidates per cell
- count of constraint-passing candidates per cell
- mean score per cell
- best composite score per cell

**Why this is lower priority**
- it does not improve candidate selection as directly as device-side constraints or composite scoring
- it increases atomics and heatmap buffer size
- it is mostly a visualization / exploratory product feature

**Implementation note**
If this lands, do not overload the current single-value heatmap contract by default.
Add it as:
- extra heatmap planes
- or a parallel opt-in heatmap payload

---

### 10) Pareto / weighted-rank / epsilon-front exploration on full-grid device output
**Class:** approximate / optional

**Why it is last**

These are interesting semantics, but they are not the best first additive move.

**Weighted ranks**
- very good scale-robustness
- but global rank aggregation is less natural on device than scalar top-k
- much better as a host rerank on a tiny union shortlist

**Exact Pareto**
- mathematically attractive
- but exact full-grid Pareto is not the best first architecture here

**Recommended use**
- keep these as optional host-side rerank modes over the union shortlist from Recommendation 4
- do not make them the first device-level semantics

---

## Math / Optimization Problems

### 1) Best practical composite score families over the current metric set

Let the exact per-candidate metric vector be

```text
m = (
    pnl,
    sharpe,
    max_dd,
    downside_dev,
    ulcer,
    omega,
    martin,
    trades,
    exposure,
    net_exposure
)
```

and define the “good direction” map

```text
good =
    (+ pnl, + sharpe, - max_dd, - downside_dev, - ulcer,
     + omega, + martin, + trades, target exposure, target net_exposure)
```

where exposure and net exposure are not naturally one-sided and usually want band or target semantics rather than “bigger is always better”.

#### Family A: constrained normalized additive utility
This is the best first scalar family.

Form:

```text
S_add(m) = sum_i w_i * g_i(m_i) - sum_j lambda_j * h_j(m)
```

with:
- `g_i` = monotone clipped transforms
- `h_j` = hinge or band penalties

A good transform discipline is:
- use `asinh(x / s)` for signed, heavy-tailed “good when larger” ratios
- use `log(max(x, eps))` for positive ratios centered around 1, such as `omega`
- use `log1p(x / t)` for positive costs such as drawdown, downside deviation, or ulcer
- use saturating floors for `trades`
- use band penalties for `exposure` and `net_exposure`

This family is:
- monotone in the correct directions
- exact and deterministic
- cheap on GPU
- interpretable
- stable for exact top-k

Its weakness is compensability:
a very strong metric can offset a weak one unless the penalty weights are chosen carefully.

#### Family B: augmented Tchebycheff / worst-deficit score
This is the best second scalar family.

Let `q_i(m)` be transformed utilities in `[0,1]` and aspiration levels `a_i`.
Define deficiencies:

```text
d_i(m) = max(0, a_i - q_i(m))
```

Then score by:

```text
S_tch(m) = -max_i(lambda_i * d_i(m))
           - eps * sum_i(lambda_i * d_i(m))
           + beta * q_primary(m)
```

This family is:
- less compensatory than a weighted sum
- closer to “balanced robust candidate” selection
- still cheap enough for device scoring

Its weakness is parameterization:
it requires aspiration levels, and those levels must be meaningful.

#### Family C: geometric mean after positivity transforms
Form:

```text
S_geo(m) = prod_i z_i(m) ^ w_i
```

with `z_i(m) in (0, 1]`.

This strongly rewards balance, but only after careful positivity transforms.
It is acceptable as an optional family, but not my first recommendation because:
- it is more fragile around zeros
- it is less intuitive to tune than the additive family
- it still needs the same normalization discipline

#### Family D: harmonic mean
Form:

```text
S_harm(m) = 1 / sum_i w_i / z_i(m)
```

This is even more imbalance-averse than the geometric mean.
In practice, it is too brittle for first-wave use because near-zero transformed components dominate too hard.

#### Family E: weighted ranks
Form:

```text
S_rank(m) = -sum_i w_i * rank_i(m)
```

This is very robust to scale differences and outliers.
The problem is not mathematics.
The problem is placement:
global rank aggregation is not the cheapest device semantic here.

Recommendation:
- use weighted ranks only on a small union shortlist on the host

#### Family F: epsilon-constraint / lexicographic
This is not a single smooth scalar, but it is one of the best practical multi-objective designs.

Example:

```text
maximize sharpe
subject to:
    omega >= 1.15
    max_dd <= 0.20
    trades >= 20
tie-break:
    calmar, then pnl
```

This is:
- mathematically clean
- easy to explain
- cheap to implement
- exact on device if the constraints are available there

It is one of the best actual product modes for this repo.

#### Family G: Pareto-shortlist then rerank
This is best treated as:
- optional
- host-side on a tiny shortlist
- not the first global exact device mode

#### Best recommendation for this repo
My recommendation is:

1. **Default composite family:** constrained normalized additive utility
2. **Second exact family:** augmented Tchebycheff / worst-deficit scalarization
3. **Non-scalar multi-objective mode:** epsilon-constraint / lexicographic
4. **Host-only optional rerank:** weighted ranks on union shortlist

#### Normalization guidance
Do not make the first implementation depend on run-relative z-scores or percentiles of the full candidate matrix.
That makes the score:
- depend on the candidate population
- harder to reason about across benchmark surfaces
- more complex than needed

Prefer:
- fixed scales
- aspiration levels
- user-specified thresholds
- clipped transforms

#### Denominator / infinity handling
Use the same semantics already present in CPU finalization and host decode:
- `Sortino`: clamp with sign-based saturation when downside deviation is zero
- `Calmar`: clamp with sign-based saturation when drawdown is zero
- `Martin`: same
- `Omega`: clamp to `[0, 1e6]` using a small epsilon

That avoids semantic drift between CPU and GPU paths.

---

### 2) Best exact multi-objective top-k approach from one metric matrix

Assume the single pass emits one exact metric matrix `X in R^(N x 10)`.

The core point is:
there is no unique “multi-objective top-k” without specifying semantics.

So there are really three different problems:

1. exact top-k for **multiple scalar objectives**
2. exact top-k for **one composite scalar objective**
3. exact shortlist for **non-scalar exploratory semantics**

#### Option A: separate exact reductions
For `H` scalar heads `s_h(x)`, run:

```text
I_h = TopK(s_h(X), K)
```

independently for each head.

**Cost**
- `H` passes over the row matrix
- very small scratch
- no effect on the main bar walk

For the current benchmark:
- `N ≈ 58,300`
- `K = 50`
- blocks of `512` items imply about `114` first-stage blocks
- candidate scratch length is about `5,700` rows per head

This is a very favorable regime.

**Recommendation**
This is the best exact design for returning multiple useful shortlists.

#### Option B: one exact composite reduction
If the user wants one “best balanced” list, then define a scalar composite `S(m)` and run one exact top-k over it.

This is the best exact design for one multi-objective mode.

#### Option C: fused multi-head reduction
This saves some row rereads but increases reduction-kernel complexity.
Given the tiny size of the metric matrix relative to the bar walk, it is not the best first implementation.

#### Option D: exact Pareto-front then rerank
This is not the right first exact design for this repo.
The cost and semantic instability are not justified relative to the value of separate exact heads.

#### Best practical design for `N ≈ 58,300`, `K = 50`, `T = 200,000`
1. Separate exact device reductions for a small head set
2. Optional union shortlist
3. Exact composite rerank on the union shortlist, or exact global composite reduction if the final mode is scalar
4. Host Pareto / weighted-rank only on the union shortlist

This keeps the heavy work on device and the non-scalar semantics on a tiny host payload.

---

### 3) Best hard-constraint vs soft-penalty split

Let each candidate metric vector be `m`.
For a given condition `c(m)`, define:

- `H(c)` = semantic hardness
  - is violation truly unacceptable?
- `R(c)` = reliability
  - is the metric stable enough to threshold?
- `P(c)` = expected prune value
  - does it meaningfully reduce feasible candidates?
- `B(c)` = bias risk
  - does a hard threshold create high selection bias?

A simple decision rule is:

```text
hard filter if:
    H(c) + alpha * R(c) + beta * P(c) - gamma * B(c) > tau
else:
    soft penalty or diagnostic only
```

#### Good hard-filter candidates
These have:
- clear acceptability semantics
- exact computation from the current row
- low ambiguity

Examples:
- `min_trades`
- `max_drawdown`
- `max_volatility`
- exposure band constraints
- `max_abs_net_exposure`
- `max_ulcer_index`

#### Borderline hard-filter candidates
These are exact but noisier because they are ratios.

Examples:
- `min_sharpe`
- `min_sortino`
- `min_omega`
- `min_calmar`

Default guidance:
- make them hard only when explicitly requested
- otherwise keep them as soft penalties
- pair them with `min_trades` or another sample-quality floor

#### Diagnostic-only candidates
These are useful for explanation but usually not first-line filters:
- win rate
- profit factor
- average holding time
- friction stress summaries
- rich regime summaries

#### Exactness interaction
A truly hard constraint must be applied before top-k.
Otherwise, the returned top-k is not the exact best feasible set.

This is the strongest mathematical reason to move more hard constraints onto the device.

---

### 4) Value density of extra state under a no-regression target

Use the following qualitative cost model:

```text
DeltaCost
  ≈ a * DeltaRegs
  + b * DeltaBacktestFlops
  + c * DeltaRowBytes
  + d * DeltaReductionPasses
  + e * DeltaAtomics
  + f * DeltaD2H
```

#### Most dangerous classes on an Ada / RTX 4090-like device
1. **Register growth in the dominant backtest kernel**
   - can reduce active blocks per SM
   - can turn a “free extra metric” into a real runtime regression

2. **Row-width growth**
   - increases tile footprint
   - can change tiling decisions
   - can increase metric buffer traffic across all candidates

3. **Host-filter fallback / full tile D2H**
   - extremely bad value density
   - moves work in the wrong architectural direction

4. **Extra heatmap atomics**
   - lower risk than the three above, but still real

5. **Extra reduction-only passes**
   - lowest risk on this workload

#### Classification of proposed additions

| Addition | Class |
| --- | --- |
| `Sortino` / `Omega` objectives | reduction-only |
| `min_sharpe`, `min_calmar`, `max_volatility`, exposure constraints | reduction-only |
| `min_omega`, `max_ulcer`, `min_sortino` on the current 10-row product path | reduction-only |
| composite score on current row | reduction-only |
| multi-head exact top-k | reduction-only |
| richer heatmap planes | atomic-increasing / host-materialization-increasing |
| half-split stability summary | register-only, optionally row-width-increasing if exported |
| trade-level metrics | register-only and likely row-width-increasing if exported |

#### Best useful-work portfolio under strict no-regression
1. reduction-only additions first
2. internal-only derived metrics second
3. one minimal stability summary third
4. trade-level metrics later
5. optional heatmap enrichment after shortlist work

---

### 5) Minimal sufficient state for the richest derived metric family

There are two different optimization targets:

1. minimal state for exact exported product outputs
2. minimal state for exact ranking / filtering / composite scoring

Those targets are not identical.

#### Current latent running state
The kernel currently carries enough latent state to derive:
- `pnl`
- `max_dd`
- `sharpe`
- `sortino`
- `omega`
- `ulcer_index`
- `martin_ratio`
- `trades`
- `exposure`
- `net_exposure`

That is already a strong sufficient-statistics basis.

#### Current emitted 10-row state
The emitted 10-row row is close to a local optimum for downstream flexibility:
- one profit magnitude
- one trade activity count
- one peak-to-trough risk metric
- one symmetric dispersion pair (`mean`, `std`)
- one downside dispersion metric
- one path-risk metric (`ulcer`)
- one upside/downside asymmetry metric (`omega`)
- one participation pair (`exposure`, `net_exposure`)

That combination already supports exact downstream derivation of:
- `Sharpe`
- `Sortino`
- `Calmar`
- `Volatility`
- `Martin`

#### Recommendation
Keep the 10-row output fixed as the default contract.

#### Why
Because most of the next high-value features do **not** need more emitted rows:
- direct `Sortino`
- direct `Omega`
- composite scoring
- richer constraints
- multi-head top-k
- shortlist union

#### What should live only in registers or score functions
Examples:
- composite score values
- Tchebycheff deficits
- stability penalties
- shortlist membership bits
- derived exposures such as `long_exposure` / `short_exposure`
- any “explainability score” that does not need to be exported for every candidate

#### When widening becomes justified
Widen only if all three are true:
1. the new quantity cannot be reconstructed from the current 10 rows or existing latent state
2. it materially changes full-grid device filtering / ranking
3. it is worth the row-width cost on the benchmark surface

#### Best backward-compatible widening strategy if ever needed
If widening is necessary later:
- keep the first 10 row slots frozen
- append new fields only at the tail
- gate it behind an opt-in mode
- prefer optional sidecar buffers or extended payloads over redefining the base contract

---

### 6) Best stability segmentation under a fixed extra-state budget

#### Recommendation
Use **first half / second half** as the first exact segmentation scheme.

#### Why it wins
- no new input series
- lowest additional control complexity
- good signal for temporal robustness
- much cheaper than thirds
- more universal than side-based segmentation
- much cheaper than regime-conditioned or drawdown-conditioned buckets

#### Ranking of segmentation families under a tight budget
1. first half / second half
2. long-only / short-only (only if two-sided strategy use is common)
3. thirds
4. regime-conditioned buckets
5. drawdown-conditioned buckets

#### How to use the result
- default: soft penalty or secondary rerank
- optional: display metric
- not default: hard filter

#### Reason
A stability summary is valuable, but it is still a robustness preference more often than a hard acceptability boundary.

---

### 7) Exact one-pass enrichment vs approximate screening

#### When exact one-pass enrichment dominates
Exact one-pass enrichment dominates when the desired addition:
- reuses already-carried state
- does not increase row width materially
- lives in the reduction layer
- or adds only a very small amount of extra register state

That includes:
- direct `Sortino`
- direct `Omega`
- composite scoring on current rows
- richer current-row constraints
- multi-head exact top-k

For this workload, those additions are strictly preferable to approximate screening because:
- the expensive bar walk is already being paid
- the added work is exact
- the added work is cheap
- approximate screening would only add selection error and pipeline complexity

#### When approximate screening still wins
Approximate or multi-fidelity screening still wins when the desired addition:
- materially increases backtest-kernel registers
- requires many new per-trade statistics
- requires many new bucketed summaries
- or effectively becomes a validation engine rather than an optimization score

That is why:
- trade-level metric families
- rich regime robustness
- friction stress
- walk-forward-style diagnostics

should not be pulled wholesale into the single optimization pass.

---

### 8) Where shortlist formation should happen

Let:
- `F` = feasible set after hard filters
- `s_h` = scalar score heads
- `U = union_h TopK_F(s_h, K_h)`

#### Best split

##### Device
Do on device:
- hard filters
- scalar top-k
- per-head exact shortlists
- union shortlist formation

##### Host
Do on host, but only on `U`:
- weighted-rank aggregation
- Pareto-like explanation modes
- interactive weight reranking
- presentation-specific ordering

#### Why this is best
Because the device is the right place for:
- exact scalar ranking
- exact hard filtering
- avoiding full-tile readback

And the host is the right place for:
- non-scalar or UI-driven semantics
- only after the candidate set is already tiny

#### How much information should be copied back
For exact product behavior:
- current default `best` / `top_k` rows only

For optional multi-objective behavior:
- per-head top-k rows
- or the deduplicated union shortlist rows

Do **not** copy back the full metric matrix unless the user explicitly asked for `include_all` / export behavior.

---

## Implementation Order

### Phase 1 — exact low-risk wins first
1. Enable direct GPU `Sortino` and `Omega` objectives
2. Extend device-side constraints using current row values
   - first 7-row / 3-row-compatible constraints
   - then 10-row-only constraints
3. Add one composite scalar family (`SharpeWithGuardrails` or `BalancedRiskAdjusted10`)

**Why first**
All three are reduction-layer additions with almost no backtest-kernel risk.

### Phase 2 — shortlist enrichment
4. Add exact multi-head top-k
5. Add optional shortlist union output
6. Add host rerank only on union shortlist for optional exploratory modes

**Why second**
This is where the main product discovery value appears without changing the core single-objective route.

### Phase 3 — minimal robustness enrichment
7. Add half-split stability summaries as internal scoring / diagnostic options

**Why third**
This is the first item that actually touches the backtest-kernel state budget.

### Phase 4 — opt-in extended metrics only if still justified
8. Consider trade-level metric family only if the earlier phases leave a clear product gap

### Phase 5 — optional visualization work
9. Add richer heatmap aggregates only after the shortlist and score work has landed

---

## Open Questions / Risks

1. **Composite-score parameterization risk**
   - the transforms and weights must be stable and explainable
   - fixed scales are safer than run-relative normalization for the first version

2. **Ratio-threshold reliability risk**
   - `min_sharpe`, `min_sortino`, `min_omega`, and `min_calmar` can be noisy if sample quality is weak
   - default guidance should therefore pair them with `min_trades`

3. **Register-pressure risk for stability and trade-level metrics**
   - the current benchmark suggests row-width growth from 7 to 10 is cheap on this surface
   - that does **not** prove that all new per-thread state is cheap
   - any stability or trade-level state should be benchmarked carefully

4. **Shortlist-semantic exactness risk**
   - a union shortlist is exact for each head
   - it is **not** automatically exact for every possible non-scalar multi-objective semantic
   - if the final behavior is a scalar composite, the exact top-k for that composite should still be computed globally on device

5. **Compatibility risk for widened outputs**
   - do not widen the default 10-row contract casually
   - if widening ever becomes necessary, freeze the first 10 slots and append only opt-in tail fields

6. **Heatmap scope risk**
   - richer heatmap outputs are easy to justify as “cheap”
   - but they do not improve shortlist quality as directly as the ranking / constraint work
   - they should not preempt higher-value optimization-stage additions

7. **Benchmark-meaning risk**
   - the contract-split benchmark deliberately forces legacy rows in the search-only variants
   - new opt-in features must not silently invalidate the interpretation of that benchmark family

---

## Bottom Line

The best additive path is **not** to add a new replay path and **not** to widen the default row immediately.

The best additive path is:

1. exploit the 10-row single-pass matrix much harder
2. move more exact filtering and ranking semantics onto the device
3. add composite and multi-head ranking modes that reuse the existing row
4. keep the host responsible only for small-shortlist rerank / explanation
5. spend new backtest-kernel state budget only after the reduction-layer gains are exhausted

In practical order, the highest-value portfolio is:

1. direct GPU `Sortino` / `Omega`
2. expanded device-side constraints
3. composite scalar modes over the current row
4. exact multi-head top-k + union shortlist
5. device-first shortlist / host-light rerank
6. half-split stability summary
7. trade-level metric families only later and only opt-in