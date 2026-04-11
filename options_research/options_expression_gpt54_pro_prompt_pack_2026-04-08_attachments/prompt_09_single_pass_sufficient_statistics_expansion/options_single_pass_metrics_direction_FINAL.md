# Options Single-Pass Metrics Direction — Final

Date: 2026-04-11  
Status: final consolidation answer for Prompt 9

## 1. Executive summary

The repo should adopt a **minimal exact two-accumulator expansion** of the per-tuple aggregate state used by the options-expression GPU path.

Adopt now:

- carry `sum_return_on_premium = Σ r_i`
- carry `sumsq_return_on_premium = Σ r_i^2`
- expose two new exact metrics in `OptionsExpressionMetrics`:
  - `mean_return_on_premium`
  - `return_volatility_on_premium`
- keep `median_return_on_premium` **exactified-only**
- keep `robust_score(...)` **unchanged** for now

Do not adopt in this first wave:

- downside-deviation accumulators
- profit-factor / gross-profit / gross-loss families
- bounded quantile or histogram sketches
- any path-dependent metric

This is the strongest repo-fit move because it closes the current gap between:

- premium-weighted return information already available in `expectancy_per_premium`
- and trade-level return-distribution information that the broad GPU path currently lacks

while preserving the current compiled-tensor shape, CPU exactification boundary, and artifact-driven workflow.

In short: **widen the GPU aggregate state by exactly two floating-point accumulators, add exactly two trade-level return moments, and stop there unless later ranking-math work proves more state is worth it.**

---

## 2. Current metrics and implied aggregate state

### Current shipped metrics surface

`OptionsExpressionMetrics` currently exposes eight metrics:

- `valid_trades`
- `invalid_events`
- `invalid_rate`
- `expectancy_per_trade`
- `expectancy_per_premium`
- `median_return_on_premium`
- `win_rate`
- `total_pnl_per_contract`

On the CPU path, `compute_metrics(...)` calculates these from the realized trade outcomes. It explicitly materializes the full vector of `return_on_premium` values and computes an exact median from that vector.

### Current exact CPU sufficient statistics

For a split/lane with total event count `E`, valid trade count `n`, invalid count `k = E - n`, win count `w`, trade-level PnL values `pnl_i`, paid premium values `prem_i`, and trade-level returns

`r_i = pnl_i / prem_i`,

the current CPU path effectively computes:

- `P = Σ pnl_i`
- `Q = Σ prem_i`
- `w = count(pnl_i > 0)`
- `median(r_i)` from the full realized return vector

and derives:

- `invalid_rate = k / E`
- `expectancy_per_trade = P / n`
- `expectancy_per_premium = P / Q`
- `win_rate = w / n`
- `total_pnl_per_contract = P`
- `median_return_on_premium = median(r_i)`

### Current GPU aggregate state

The broad GPU path does **not** carry the full CPU metric state. `OptionsGpuTupleAggregate` currently retains only:

- `valid_trades`
- `wins`
- `total_pnl`
- `total_premium`

That is the compact state the broad compiled-tensor pass emits per tuple per lane.

Inside `options_expression.rs`, the GPU-side reduction is then widened only to the local helper `TupleAggregateMetrics`, which carries:

- `valid_trades`
- `invalid_events`
- `win_count`
- `total_pnl_per_contract`
- `total_premium_paid`

and `metrics_from_aggregate(...)` reconstructs the remaining metrics from that state.

### What the current aggregate state already implies

For the broad GPU path, the current mergeable sufficient-statistics state is:

`S0 = (n, w, P, Q)`

with `k` recovered from split size as:

`k = E - n`

That state is already exact for:

- `invalid_rate`
- `expectancy_per_trade`
- `expectancy_per_premium`
- `win_rate`
- `total_pnl_per_contract`

It is **not** exact for anything that depends on the distribution of `r_i`, especially:

- `median_return_on_premium`
- any quantile
- any trade-level dispersion measure
- any downside-only return moment

That is the core mathematical gap.

---

## 3. Final recommended metric expansion

### Adopted direction

Add exactly two new trade-level return metrics:

1. `mean_return_on_premium`
2. `return_volatility_on_premium`

These should be computed from two new mergeable accumulators over realized trade returns `r_i`:

- `R1 = Σ r_i`
- `R2 = Σ r_i^2`

and derived as:

- `mean_return_on_premium = μ_r = R1 / n`
- `return_volatility_on_premium = σ_r = sqrt(max(R2 / n - μ_r^2, 0))`

These metrics are:

- exact from small sufficient statistics
- mergeable across event partitions
- compatible with the current GPU-first compiled pass shape
- immediately useful as reportable context
- plausibly useful for later Prompt 10 / Prompt 11 ranking or acceptance math

### Why this wins now

This pair is the best first-wave expansion because it adds the **minimum non-redundant trade-level return family**.

The repo already has:

- a premium-weighted mean-like signal: `expectancy_per_premium`
- an order-statistic on trade-level return: `median_return_on_premium`

But it does **not** have:

- the trade-weighted mean of return-on-premium
- any trade-level dispersion measure on the same return domain

That means the current metrics surface mixes two incompatible viewpoints:

- premium-weighted mean return
- median of trade-level returns

without exposing the simplest trade-level moment pair that would let engineers and users interpret the return distribution more coherently.

The adopted pair fixes exactly that hole.

### What this does not try to do

This change does **not**:

- replace `robust_score(...)`
- remove or demote `median_return_on_premium`
- widen the workflow
- add another validation stage
- reopen structure-family or realism-lane scope
- introduce sketches or approximate quantile machinery

It is a narrow sufficient-statistics improvement only.

---

## 4. Required accumulator/state vector for the adopted metrics

### Minimal adopted state vector

The recommended widened per-tuple per-lane state is:

`S1 = (n, w, P, Q, R1, R2)`

where:

- `n = valid_trades`
- `w = wins`
- `P = Σ pnl_i`
- `Q = Σ premium_i`
- `R1 = Σ r_i`
- `R2 = Σ r_i^2`
- `k = E - n` still comes from the split event count

### Derived metric formulas

For `n > 0`:

- `invalid_rate = (E - n) / E`
- `expectancy_per_trade = P / n`
- `expectancy_per_premium = P / Q` if `Q > 0`, else `0`
- `win_rate = w / n`
- `total_pnl_per_contract = P`
- `mean_return_on_premium = R1 / n`
- `return_volatility_on_premium = sqrt(max(R2 / n - (R1 / n)^2, 0))`

For `n == 0`, all return-domain means and dispersions remain `0.0`, matching the current repo posture for empty-valid-trade splits.

### Exactness and mergeability

This adopted pair is:

- **exact**, not approximate
- **mergeable**, because sums of first and second moments are associative
- **path-order insensitive**, so it does not need replay ordering
- **compatible with current lane partitioning**, because it can be accumulated independently for search, holdout, stress, and harsher lanes

### State-budget effect

Current GPU aggregate output per tuple:

- `u32 valid_trades`
- `u32 wins`
- `f32 total_pnl`
- `f32 total_premium`

That is **16 bytes per tuple**.

Recommended widened GPU aggregate output per tuple:

- current 16 bytes
- plus `f32 sum_return_on_premium`
- plus `f32 sumsq_return_on_premium`

That is **24 bytes per tuple**.

So the first-wave widening cost is:

- **+8 bytes per tuple**
- **+64 KiB** at the current hard chunk cap of `8,192` tuples

This is still modest enough to count as near-constant-cost relative to the current planner and outcome-tensor footprint.

### Rust contract widening

This change widens both:

- `OptionsGpuTupleAggregate`
- `OptionsExpressionMetrics`

Recommended new `OptionsExpressionMetrics` fields:

- `mean_return_on_premium: f64`
- `return_volatility_on_premium: f64`

Recommended compatibility rule:

- add `#[serde(default)]` to the new fields so older saved artifacts continue to deserialize cleanly

---

## 5. Metrics that should remain unchanged

The following should remain unchanged in this implementation wave:

### Keep the existing shipped metrics

Keep all current fields in `OptionsExpressionMetrics`:

- `valid_trades`
- `invalid_events`
- `invalid_rate`
- `expectancy_per_trade`
- `expectancy_per_premium`
- `median_return_on_premium`
- `win_rate`
- `total_pnl_per_contract`

### Keep `median_return_on_premium`, but keep it exactified-only

Do **not** remove `median_return_on_premium`.

But do **not** attempt to make the broad GPU path approximate it with a sketch in this first wave.

The correct repo posture is:

- broad GPU ranking may continue to carry `median_return_on_premium = 0.0` in aggregate-derived placeholder form
- retained tuples and sealed finalists continue to get the exact CPU median from `compute_metrics(...)`

That preserves semantic honesty and avoids fake parity between broad GPU ranking outputs and retained-tuple exact metrics.

### Keep `robust_score(...)` unchanged

Do **not** change the ranking functional in this metric expansion pass.

The new metrics are worth computing now, but ranking-math changes belong to the later Prompt 10 decision. The repo should first prove that the added state is cheap, exact, and stable before any scalarization change is attempted.

---

## 6. Deferred metric ideas

These ideas remain alive, but they should **not** be part of the adopted first-wave state widening.

### A. `downside_deviation_on_premium`

Mathematically plausible state:

- `D2 = Σ min(r_i, 0)^2`

Derived metric:

- `downside_deviation_on_premium = sqrt(D2 / n)`

Status:

- exact
- mergeable
- cheap in absolute terms
- **defer**

Reason to defer:

It is the strongest false-cheap extension. It adds a third accumulator and downside-specific semantics before the repo has actually decided that downside-aware ranking or acceptance math is worth adopting.

Only reopen this if Prompt 10 or Prompt 11 later concludes that a downside-sensitive scalarization or confidence penalty is materially better than what current metrics support.

### B. Gross positive / gross negative payoff families

Examples:

- gross positive PnL
- gross negative PnL
- profit factor
- average win / average loss

Status:

- exact with a few extra accumulators
- potentially useful
- **defer**

Reason to defer:

These metrics are less aligned with the current repo’s return-on-premium framing than the adopted `R1` / `R2` pair, and they do not close the most important current gap as directly.

### C. Free reporting-only deltas from existing lane metrics

Examples:

- `coverage_rate = 1 - invalid_rate`
- `holdout_delta_expectancy_per_premium`
- `stress_drop_expectancy_per_premium`
- `harsher_drop_expectancy_per_premium`

Status:

- derivable from already-computed lane metrics
- no new GPU state required
- **defer from this change**

Reason to defer:

These are useful, but they belong more naturally to reporting / validation-frontier work than to the narrow sufficient-statistics widening decision in this prompt.

### D. Average premium context metrics

Example:

- `average_premium_paid_per_trade = Q / n`

Status:

- free from current state
- **defer**

Reason to defer:

This is more contextual than decision-critical. It may be worth adding later for UI or CSV context, but it is not the highest-value metric expansion for this repo now.

---

## 7. Rejected metric ideas

These should be treated as out of scope or actively rejected for this first implementation wave.

### A. Median / quantile sketches in the broad GPU pass

Rejected now:

- bounded histogram approximations
- streaming quantile summaries
- sketch-based median parity attempts

Reason:

The broad GPU path would start reporting approximate distribution statistics that disagree with the exact CPU metrics used later for retained tuples and sealed finalists. That is exactly the kind of subtle semantic mismatch this repo should avoid.

### B. Path-dependent metrics

Rejected now:

- max drawdown
- underwater duration
- time to recovery
- MFE / MAE distributions
- any order-sensitive path statistic

Reason:

These are not small sufficient-statistics problems. They are replay/path problems and do not belong in this hot aggregate widening pass.

### C. Higher moments beyond variance as first-wave additions

Rejected now:

- skewness
- kurtosis
- higher-order moment families

Reason:

Even though some are algebraically decomposable with additional accumulators, they are weaker product metrics than the adopted first and second moments and much easier to misuse on small trade counts.

### D. Generic sketch-heavy distribution work

Rejected now:

- small histograms by default
- bucketed return surfaces per tuple
- anything that substantially widens output state for exploratory curiosity rather than clear decision value

Reason:

Prompt 9 is a small-state sufficient-statistics problem, not a broad distributional telemetry project.

---

## 8. Concrete code touch points by file and symbol

### `crates/app_core/src/options_types.rs`

Touch:

- `OptionsExpressionMetrics`

Changes:

- add `#[serde(default)] pub mean_return_on_premium: f64`
- add `#[serde(default)] pub return_volatility_on_premium: f64`

Also update helper constructors and tests in the local `tests` module, especially any `OptionsExpressionMetrics { ... }` literal that currently enumerates all fields.

### `crates/app_core/src/options_expression.rs`

Touch:

- `compute_metrics(...)`
- `TupleAggregateMetrics`
- `metrics_from_aggregate(...)`
- `gpu_scoring::aggregate_from_gpu(...)`
- local test helpers constructing `OptionsExpressionMetrics`

Changes:

- in `compute_metrics(...)`, accumulate `R1` and `R2` from `TradeOutcome.return_on_premium`
- widen `TupleAggregateMetrics` with:
  - `sum_return_on_premium`
  - `sumsq_return_on_premium`
- in `metrics_from_aggregate(...)`, derive the new mean and volatility metrics
- in `aggregate_from_gpu(...)`, map the widened GPU aggregate into the widened local aggregate
- update local tests such as:
  - `metrics_with_values(...)`
  - `robust_score_excludes_sealed_metrics_from_tuple_ranking()`
  - `sealed_final_rank_uses_sealed_metrics_ordering()`

Explicit non-change:

- leave `robust_score(...)` unchanged in this pass

### `crates/app_core/src/options_expression_gpu.rs`

Touch:

- `OptionsGpuTupleAggregate`
- `ReusableOutputBuffers`
- `copy_to_host()`
- `aggregates_prefix()`
- `per_tuple_output_device_bytes()`
- device/host output-buffer management used by `score_ranges(...)`

Changes:

- widen `OptionsGpuTupleAggregate` with two new compact floating-point outputs
- add matching device buffers and host vectors in `ReusableOutputBuffers`
- include those arrays in host copyback
- include them in `aggregates_prefix()` mapping
- update `per_tuple_output_device_bytes()` from 16 to 24

### `crates/app_core/src/kernels/options_expression.cu`

This file was not attached in the prompt bundle, but it is an implied required touchpoint.

Touch:

- the tuple-score kernel output layout

Changes:

- accumulate `Σ r_i`
- accumulate `Σ r_i^2`
- emit both compact outputs in lockstep with the widened Rust-side output buffers

### `src-tauri/src/commands_options_expression.rs`

Touch:

- test fixtures and any manual `OptionsExpressionMetrics` struct literals

Changes:

- update existing test data to include the new fields
- keep artifact-summary behavior unchanged unless later product work explicitly decides to surface the new metrics in summary tables or CSVs

---

## 9. Verification order

### 1. CPU formula tests first

Add unit tests around `compute_metrics(...)` using synthetic `TradeOutcome` vectors where:

- the mean return is easy to compute by hand
- the variance is nonzero and obvious
- empty-valid-trade and single-trade cases are covered

Goal:

- prove the formulas and edge handling before any GPU work

### 2. Aggregate equivalence tests second

For the same synthetic outcomes, construct the equivalent widened `TupleAggregateMetrics` and verify that:

- `metrics_from_aggregate(...)` matches `compute_metrics(...)`
- all fields match exactly except the deliberately exactified-only median behavior

Goal:

- prove the sufficient-statistics claim

### 3. GPU ↔ CPU parity tests third

Add parity tests over widened GPU aggregates for at least:

- search lane
- holdout lane
- conservative stress lane
- harsher stress lane

Goal:

- prove that widened GPU outputs decode to the same mean / volatility metrics the CPU path would produce from the same realized outcomes

### 4. Artifact compatibility test fourth

Load an older options-expression artifact without the new metric fields and verify deserialization still succeeds because the new fields default to `0.0`.

Goal:

- preserve saved-artifact compatibility

### 5. Planner and memory regression check fifth

Verify that:

- `per_tuple_output_device_bytes()` rises from 16 to 24 exactly
- tuple chunk planning still fits the canonical DTE7 workload
- no chunk-plan regression or cache-reuse regression appears in feasibility telemetry

Goal:

- prove near-constant-cost really holds at the runtime level

### 6. End-to-end benchmark sixth

Run the smallest meaningful regression set:

- canonical options GPU scorer bench or feasibility preview on the current DTE7 slice
- broad GPU ranking path with unchanged `robust_score(...)`

Goal:

- confirm that the metric widening itself does not materially slow the workflow before any ranking-math work consumes the new metrics

---

## 10. Cost-risk checklist

### Hot-path cost

- extra GPU output width: **+8 bytes per tuple**
- extra kernel arithmetic: a few floating-point adds/multiplies per realized trade
- no new path replay
- no new workflow stage
- no new lanes

### Memory / planner risk

- output buffers widen from 16 to 24 bytes per tuple
- shared-input width is unchanged
- outcome tensor width is unchanged
- chunk planning must still fit without degrading the current canonical workloads

### Exactness risk

- the adopted metrics are exact from mergeable sufficient statistics
- no bounded sketch is introduced
- median remains exactified-only

### Product semantics risk

- `return_volatility_on_premium` must be documented as cross-trade dispersion of realized return-on-premium, not time-series volatility
- the repo should avoid immediately promoting the new metrics into ranking until Prompt 10 decides whether that is useful

### Backward compatibility risk

- adding fields to `OptionsExpressionMetrics` can break older saved artifacts unless `#[serde(default)]` is added to the new fields

### Testing burden

- moderate, because the widening touches:
  - CPU compute path
  - GPU aggregate struct
  - output buffers
  - test fixtures with explicit struct literals

But it does **not** require new workflow plumbing or a new product surface.

---

## 11. Stop conditions

Stop before shipping this change if any of the following becomes true:

1. **State growth stops being small.**  
   If the first wave requires more than the two adopted accumulators, stop and split the work.

2. **Per-tuple output width exceeds the intended first-wave budget.**  
   If the implementation pressure pushes the output layout beyond **24 bytes per tuple** in this wave, stop and reevaluate.

3. **The planner cost is not actually near-constant.**  
   If canonical feasibility previews or scorer benches regress by more than about **5–10%**, stop and treat the widening as unjustified until the regression is explained.

4. **GPU ↔ CPU parity is not tight enough.**  
   If widened GPU mean / volatility metrics do not match CPU exact metrics closely enough for retained tuples and lane comparisons to remain trustworthy, stop.

5. **The implementation tries to smuggle in median approximation.**  
   If the only way to make the change feel “complete” is to add a median or quantile sketch, stop and defer that separate problem.

6. **The new metrics immediately create ranking churn without a Prompt 10 decision.**  
   If the repo starts using the new metrics in `robust_score(...)` before proving that ranking changes help rather than destabilize selection, stop and keep them reporting-only.

7. **Backward compatibility is broken.**  
   If older options-expression artifacts cannot deserialize after the field addition, stop until `serde` defaults or a migration-safe decode path is in place.

---

## Final decision

The final repo decision is:

- **Adopt** a two-accumulator trade-return moment expansion: `Σ r_i`, `Σ r_i^2`
- **Expose** `mean_return_on_premium` and `return_volatility_on_premium`
- **Keep** `median_return_on_premium` exactified-only
- **Leave** `robust_score(...)` unchanged
- **Defer** downside deviation and gross-profit/gross-loss families
- **Reject** sketches and path-dependent metrics in this first wave

That is the smallest exact sufficient-statistics widening that materially improves the options-expression metrics surface while preserving the repo’s current GPU-first compiled evaluation shape.
