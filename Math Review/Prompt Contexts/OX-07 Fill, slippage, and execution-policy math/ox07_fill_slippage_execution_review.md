# OX-07 Fill, Slippage, and Execution-Policy Math Review

Scope note: this document compiles my first-pass, second-pass, and third-pass answers for the OX-07 overlay review over the attached repo surfaces:
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_execution.rs`

I did not have the matching `PR-*` core prompt, so each pass stays inside the OX-07 overlay boundary.

---

## First pass

I only had the OX-07 overlay prompt, not the matching core `PR-*` first-pass prompt, so I applied the first-pass overlay directly to the attached surfaces and kept the review inside those boundaries.

Bottom line: yes, a more principled fill/slippage/execution-policy model would materially improve realism here, and in `options_expression.rs` it is likely to change ranking, not just reporting.

## 1. The biggest miss is in options ranking

The current options stack is already an execution model, just an implicit one.

`options_types.rs` hard-codes search axes and stress heuristics with fixed arrays: delays `[0,1,5]`, DTE buckets `[1,3,5]`, hold buckets `[5,15,30,60]`, target returns `[0.25,0.50,1.0]`, stops `[0.25,0.50]`, and default fill model `AggregateResearch` (`options_types.rs:39-77`). Those are fine as exploratory axes, but once they feed ranking they become assumptions about execution realism.

The stronger issue is in `options_expression.rs`. Aggregate pricing is:
- baseline entry = `close`
- conservative entry = `max(high, close)`
- harsher/extreme entry = `1.02` / `1.05` times that
- baseline exit = `close`
- conservative exit = `min(low, close)`
- harsher/extreme exit = `0.98` / `0.95` times that (`options_expression.rs:698-714`)

That is already a fill-distortion model, but it is fixed-percentage and bar-extrema based, not volatility-, liquidity-, or staleness-aware.

Two especially important findings:

First, `DerivedMidSnapshot` is routed through the same aggregate entry/exit/path logic as `MinuteAggregate`, not through quote-aware mid logic (`options_expression.rs:740-765`, `options_expression.rs:856-883`). If “derived mid snapshot” is meant to be quote-like, the implementation is mathematically closer to another OHLC heuristic than to mid-based execution.

Second, the tuple filter checks lag, premium, volume, and open interest, but spread or quote staleness are not used in the actual candidate rejection path (`options_expression.rs:1076-1195`). Yet the data model already has `quote_age_ms`, and the invalid-reason enum already has `SpreadTooWide` (`options_types.rs:221-236`, `options_types.rs:360-377`). That is a strong sign that the code knows these are important but is not yet using them.

This matters because ranking is driven by `expectancy_per_premium`, and `robust_score` is just an affine combination of invalid-rate-adjusted expectancy lanes (`options_expression.rs:4466-4490`). So optimistic fill math feeds directly into ordering.

## 2. The current stop/target handling is too exact

`options_expression.rs` sets
- `target_price = entry * (1 + target_return)`
- `stop_price = entry * (1 - stop_return)` (`options_expression.rs:1262-1267`, `4040-4050`)

and then, when a bar range touches a barrier, it exits at the exact threshold price, not at a degraded realized fill (`options_expression.rs:1317-1337`, `1425-1443`, `1585-1608`, `4144-4167`).

That is a hidden assumption that barrier crossing implies threshold execution. In real options markets, especially on wider spreads, a stop touch is often worse than the nominal stop, and a target touch does not imply a fill at the threshold.

The biggest ranking distortion appears in two families:

Wide-spread / stale-quote contracts. A tuple can look attractive under `QuoteMid` or close-based aggregate entry, but ask-on-entry and bid-on-exit can erase the edge.

Same-bar double-touch bars. If the bar high crosses target and the low crosses stop in the same interval, the current logic resolves the outcome deterministically from the observed boolean hits, not from a path model or a conservative interval bound. That can flip tuple ranking even when the underlying economic edge is negligible.

## 3. Live execution is also using a hidden deterministic policy

`commands_execution.rs` submits market orders for flattening and for the normal delta path (`commands_execution.rs:1513-1536`, `1734-1745`). For new openings, bracket stop/take prices are anchored to `last_close`, not to an estimated fill price (`commands_execution.rs:1538-1585`).

The execution loop is:

- compute desired position from the strategy
- fetch current position quantity
- set `delta_qty = target_position_qty - current_position_qty`
- if nonzero, submit a market order (`commands_execution.rs:1658-1778`)

That means the owned policy is effectively:

“rebalance immediately to the target with a market order, and if opening from flat, attach brackets around the previous close.”

That is a valid product choice, but mathematically it is a strong fill model. It ignores spread, queueing delay, volatility, open orders, and partial fills.

There is already machinery to normalize order snapshots with `filled_qty` and `filled_avg_price` (`commands_execution.rs:1380-1417`), but those quantities do not feed back into the delta computation in `execute_strategy_signal_blocking` (`commands_execution.rs:1658-1778`). So the code has fill-state observability, but not fill-state-aware policy.

One more important policy effect: the live runner only keeps the latest pending bar while a prior run is still inflight (`commands_execution.rs:2011-2033`, `2052-2206`). That is a constrained decision policy, not just plumbing. In fast conditions it compresses multiple state transitions into “latest bar wins,” while the eventual order still uses market execution and, for new openings, `last_close`-anchored bracket math.

## 4. ML policy evaluation is cost-aware only in a fixed-threshold sense

`ml_signal.rs` uses a deadband
\[
\text{deadband} = \max\!\left(2\,\text{cost\_frac},\ 0.25\cdot \frac{\text{ATR}_h}{\text{open}_{t+1}}\right)
\]
via `sample_deadband` (`ml_signal.rs:930-942`).

Then signal decisions are made by fixed thresholds:
- long if `p_long >= t_long` and `p_long - p_short >= delta`
- short if `p_short >= t_short` and `p_short - p_long >= delta` (`ml_signal.rs:3524-3539`)

After that, `fill_requested_targets` and `fill_desired_positions` apply deterministic bar-boundary overlays such as trade-on-next-bar, cooldown, persistence, max-hold, and flip rules (`ml_signal.rs:3600-3697`).

That is not wrong, but it is another implicit execution model: friction is represented by a fixed deadband and deterministic state transitions, not by state-dependent fill uncertainty.

A further subtlety: validation neutralizes some execution overlays by forcing `allow_flip=true`, `cooldown_bars=0`, and `signal_persistence_bars=1` before replay (`ml_signal.rs:3701-3752`). So policy ranking is not fully aligned with the exact runtime overlays that govern live behavior.

## 5. New lemma / bound / counterexample obligations introduced by this overlay

A clean lemma candidate is:

\[
U_j = (1-\mathrm{invalid}_j)\cdot \frac{\mathrm{PNL}_j}{\mathrm{Premium}_j},
\]

matching the code’s `lane_utility = (1-invalid_rate) * expectancy_per_premium` (`options_expression.rs:4474-4475`).

With robust score

\[
R = 0.45U_{\text{search}} + 0.25U_{\text{holdout}} + 0.15U_{\text{stress}} + 0.10U_{\text{harsher}} + 0.05U_{\text{extreme}},
\]

matching `options_expression.rs:4486-4490`, an execution-aware nonnegative tax \(C_j\) changes the ranking objective by

\[
R' = R - \sum_j w_j (1-\mathrm{invalid}_j)\frac{C_j}{\mathrm{Premium}_j}.
\]

So omitted execution cost is not cosmetic realism. It enters the ranking objective linearly.

A new counterexample family is:
- contracts with acceptable volume and open interest,
- but stale quotes and wide spreads,
- where close- or mid-based entry makes expectancy positive,
- while realizable ask-entry / bid-exit or stale-quote rejection makes expectancy negative.

A useful new live bound is:
- if the runner remains inflight while bars arrive and only the latest is retained, then the bracket anchor error from using `last_close` instead of estimated fill must scale with both latency and realized volatility over skipped bars.

## 6. What I would change first

If this is a first-pass review and the goal is the smallest set of changes with the highest decision value, the priorities are:

1. In `options_types.rs` / `options_expression.rs`, add an explicit execution distortion model rather than fixed heuristics:
   \[
   p_{\mathrm{fill}} = p_{\mathrm{ref}} + s(q,\sigma,\ell,a),
   \]
   where \(q\) is size, \(\sigma\) is volatility, \(\ell\) is liquidity, and \(a\) is quote age.
2. Operationalize spread/staleness. `SpreadTooWide` and `quote_age_ms` should become real filters or penalties, not dormant schema fields.
3. In `commands_execution.rs`, anchor live brackets to an estimated fill price, not `last_close`, and reconcile open orders / partial fills before computing a fresh market delta.

Overall first-pass verdict:
- `options_expression.rs` / `options_types.rs`: highest-value target for this overlay; current fixed heuristics can materially mis-rank tuples.
- `commands_execution.rs`: highest live-risk target; current policy is simple and coherent, but mathematically too exact about fills.
- `ml_signal.rs`: second-order but real; the policy sweep should become uncertainty-aware if its rankings are going to guide live decisions.


---

## Second pass

Using only the attached OX-07 overlay, I treated this as a second-pass delta on the same four owned surfaces rather than widening to an unseen `PR-*` core.

### Second-pass verdict

The first-pass priority order still stands: `options_expression.rs` is the main ranking-quality problem, `commands_execution.rs` is the main live-policy problem, and `ml_signal.rs` is a secondary fixed-threshold issue. What changes on second pass is the bias diagnosis. The code is internally exact for its own heuristic model, but the overlay changes the target notion of exactness. The problem is not just “missing slippage.” It is that the owned math hard-codes execution semantics in places that now matter for ranking.

One first-pass point I would weaken: `neutralize_ml_signal_execution_overlays` does not look like a mismatch bug by itself. In `ml_signal.rs`, `fill_requested_targets` and `fill_desired_positions` already encode `trade_on_next_bar`, cooldown, persistence, max-hold, and flip behavior before the simulator runs, so neutralization looks more like de-duplication than omission. The remaining ML issue is different: `sample_deadband` is a fixed function of round-trip cost and ATR, labels are built from that deadband, and `score_target` still uses fixed `t_long`, `t_short`, and `delta`, so the policy remains uncertainty-blind near decision boundaries.

### Where the overlay most changes the math

The strongest change is in the options stack. In `options_types.rs:39-77` and `106-153`, the default search axes and semantics map are fixed. In `options_expression.rs:698-714`, `740-765`, and `856-883`, `MinuteAggregate` and `DerivedMidSnapshot` both flow through the same aggregate `close/high/low` stress transforms. At the same time, `options_types.rs:221-236` and `360-377` already expose `SpreadTooWide` and `quote_age_ms`, but `select_candidate` in `options_expression.rs:1076-1195` only gates lag, premium, volume, and open interest before counting a trade as valid. That means execution feasibility is under-modeled in exactly the quantity that later drives ranking: `robust_score` at `options_expression.rs:4466-4490` is a weighted function of `(1 - invalid_rate) * expectancy_per_premium`. So execution realism is not just a late P&L adjustment here. It should change both validity and payoff before ranking.

A concise way to see the ranking sensitivity is:

\[
R_i = \sum_j w_j (1-\nu_{ij}) E_{ij},
\]

where \(E_{ij}\) is lane expectancy-per-premium and \(\nu_{ij}\) is invalid rate. If an execution-aware model only adds a nonnegative tax \(d_{ij}\) and leaves invalid rate fixed, then

\[
R_i' = R_i - \sum_j w_j (1-\nu_{ij}) d_{ij}.
\]

So two tuples flip order exactly when the raw score gap is smaller than the differential weighted tax. And in this codebase the true effect is usually larger than that, because spread/staleness rules would also increase \(\nu_{ij}\), not just reduce \(E_{ij}\).

### New lemma candidate

A new lemma the first pass did not make explicit is sealed-stage irrelevance:

If a fill/slippage correction is introduced only in sealed evaluation or post-ranking exactification, it cannot improve tuple selection.

That is exact under the current implementation. `_sealed` is ignored by `robust_score`; ranking is done off pre-sealed `robust_score`; results are sorted and truncated to `top_k`; only then are finalists finalized. So any execution-aware math that matters for ranking quality must enter the search / holdout / stress / harsher / extreme lanes before truncation, not only in sealed metrics. This is the most important second-pass consequence of the overlay, because it narrows where an execution-aware fix would actually have decision value.

### New adversarial construction

The most important counterexample family is now clearer than it was in the first pass: same-bar double-touch bars. In `options_expression.rs:1317-1335`, `1425-1442`, `1585-1608`, and `4144-4161`, if a bar hits both target and stop, the code resolves the trade as a stop because the branch is effectively “if stop hit, use stop, else use target.” So for a long entry at 1.00 with stop 0.75 and target 1.25, any next bar with range `[0.70, 1.30]` becomes a stop-out in the current model, even though the true intrabar path could have reached target first. That means the current execution model is not uniformly optimistic. It is optimistic on some entry assumptions, but adversarial on same-bar exit ordering. Ranking distortion is therefore two-sided, not just “too favorable fills.”

This also changes how I would talk about the aggregate stress lanes. Right now `Baseline/Conservative/Harsher/Extreme` are fixed `close/high/low` distortions with 2% and 5% multipliers. On second pass, the right interpretation is not “four arbitrary severity presets.” It is “four candidate execution envelopes whose distortion should depend on volatility, liquidity, and quote age.” That is a more principled use of machinery the code already has.

### New bound in live execution

The cleanest exact bound is in `commands_execution.rs`. `execute_strategy_signal_blocking` computes

\[
\Delta q = q_{\text{target}} - q_{\text{current}}
\]

and submits a market order. On a fresh opening, `alpaca_order_payload` anchors bracket stop/take prices to `last_close`, not to any estimated or realized fill. If the actual fill is \(p_f\), the anchor close is \(p_c\), the stop fraction is \(s\), and the take fraction is \(t\), then for a long entry the bracket misplacement is

\[
|(1-s)(p_f-p_c)| \quad \text{for the stop,}
\]
\[
|(1+t)(p_f-p_c)| \quad \text{for the take.}
\]

So the take-profit error is at least the fill-close gap itself. The inflight runner then keeps only the latest pending bar while a prior run is still in flight, which is a coherent control choice, but another hidden execution-policy assumption rather than a neutral transport detail.

### Comparison of alternatives

For this second pass, I would reject two weaker fixes.

A sealed-stage-only slippage correction is low value, because the lemma above says it cannot improve pre-sealed selection.

A flat P&L haircut is better than nothing, but it still misses invalid-rate changes and same-bar path ambiguity.

The best fit to the current ownership is an execution-envelope model that reuses the existing lanes but makes them explicit functions of uncertainty:

\[
p_{\mathrm{fill}} = p_{\mathrm{ref}} + s(q,\sigma,\ell,a),
\]

with \(q\) size, \(\sigma\) volatility, \(\ell\) liquidity, and \(a\) quote age.

For `commands_execution.rs`, the highest-value same-boundary change is smaller and more concrete: anchor brackets to estimated or realized fill, and reconcile open orders / partial fills before computing a fresh `delta_qty`. For `ml_signal.rs`, the right second-pass change is margin robustness: fixed thresholds and delta should clear an execution-uncertainty margin derived from the same deadband logic, rather than acting as if decisions near the boundary were equally reliable.

So the second-pass ordering is: pre-sealed execution-aware envelopes in `options_expression.rs` / `options_types.rs` first, fill-anchored live brackets and partial-fill reconciliation in `commands_execution.rs` second, uncertainty-aware threshold margins in `ml_signal.rs` third.


---

## Third pass

Using the OX-07 third-pass overlay, and still without the matching `PR-*` core prompt, I’m keeping implementation ownership inside the same four files: `crates/app_core/src/options_expression.rs`, `crates/app_core/src/options_types.rs`, `src-tauri/src/commands_execution.rs`, and `crates/app_core/src/ml_signal.rs`.

### Third-pass verdict

Yes. On third pass, the overlay stops being just a realism critique and turns into a short implementation program. The important change from second pass is that execution semantics are not merely a reporting concern here. They already participate in tuple validity, payoff measurement, and live order construction, so the right implementation target is the owned math before ranking and submission, not a late reporting layer.

### 1. Put the new lemma where ranking actually happens

The strongest implementation lemma remains:

> If a fill/slippage correction is applied only in sealed evaluation or post-ranking exactification, it cannot improve tuple selection.

That is exact in the current code. `robust_score` ignores sealed metrics by construction in `options_expression.rs:4466-4490`, and there is an explicit test asserting that sealed metrics do not affect tuple ranking in `options_expression.rs:8335-8382`.

So the implementation implication is immediate: any execution-aware correction that is meant to improve selection quality must enter the pre-ranked lane metrics — search, holdout, stress, harsher, extreme — before shortlist truncation. The practical hook points are the trade-outcome builders and lane aggregators in the scalar evaluation paths and the compiled long-single path, not a later finalist/report-only step.

If that is ignored, the code can become “more realistic” in final reports while remaining unchanged in what it actually selects.

### 2. Fix `DerivedMidSnapshot` before adding more math

The cleanest local patch is semantic consistency.

Right now, `options_types.rs:112-145` and the compatibility test at `options_types.rs:1579-1595` only allow `AggregateResearch` for `DerivedMidSnapshot`. Then `options_expression.rs:740-765` and `options_expression.rs:856-883` route `DerivedMidSnapshot` through the same aggregate close/high/low path as `MinuteAggregate`. There is even a test explicitly locking in that aggregate behavior at `options_expression.rs:6874-6904`.

If the name `DerivedMidSnapshot` is meant literally, that is the wrong owned math. The third-pass implementation should therefore be:

1. Change the compatibility matrix so `DerivedMidSnapshot` supports `QuoteMid` and, if desired, `QuoteConservative`.
2. Route `DerivedMidSnapshot` through `quote_entry_price`, `quote_exit_price`, and mid-based path logic rather than aggregate OHLC logic.
3. Extend `OptionsStaticFilterBundle` in `options_types.rs:294-315` with optional `max_quote_age_ms` and `max_relative_spread`, defaulting to `None` so the current request shape remains compatible.
4. Enforce those filters inside `select_candidate` in `options_expression.rs:990-1195` using the already-available `quote_age_ms` field from `options_types.rs:375` and the already-declared but currently unused `SpreadTooWide` invalid reason from `options_types.rs:221-236`.

That is attractive because it does not require inventing a new ownership surface. The schema already has `bid`, `ask`, `mid`, `quote_age_ms`, and `SpreadTooWide`; the implementation is just not consuming them yet.

### 3. Make same-bar barrier resolution explicit and shared

The main counterexample family from second pass was same-bar double-touch: bars whose high crosses target and low crosses stop. The current code resolves those implicitly with stop-first logic.

You can see that in the scalar paths:
- `options_expression.rs:1317-1335`
- `options_expression.rs:1425-1442`
- `options_expression.rs:1585-1608`

and again in the compiled long-single path:
- `options_expression.rs:4144-4161`

The implementation-facing implication is not just “choose a better policy.” It is “extract one barrier-resolution helper and call it everywhere,” because otherwise any future realism improvement will silently break scalar/compiled parity.

The bounded third-pass move is:

```rust
resolve_barrier_exit(
    stop_hit,
    target_hit,
    stop_px,
    target_px,
    fallback_px,
    policy,
)
```

and use that helper in every scalar and compiled barrier site.

The minimum safe version keeps the current conservative stop-first rule but makes it explicit and testable. The more principled follow-on is an ambiguity-aware policy for double-touch bars. I would not change that policy in three different call sites by hand.

### 4. `commands_execution.rs` already has the plumbing to stop assuming perfect fills

`commands_execution.rs` normalizes `filled_qty`, `filled_avg_price`, and broker order status in `1380-1417`. It can fetch recent orders in `1220-1278`, fetch open orders by symbol in `1280-1287`, and it already has safety actions that reason about open orders in `2465-2555`.

But the live submission path still does this:

- compute `delta_qty = target_position_qty - current_position_qty` from the current position only (`1658-1695`)
- submit a market order
- for a fresh opening, attach bracket prices anchored to `last_close` rather than to a fill estimate or realized fill (`1538-1585`, `1734-1745`)

So the implementation-facing third-pass change is local and concrete:

1. Before submitting a live order, query open orders for the symbol and refuse or reconcile if an order is still open or partially filled.
2. For new openings, stop anchoring protection to `last_close`.
3. Submit the entry order first, fetch recent orders, identify the returned `order_id`, normalize its snapshot, and when `filled_avg_price` is available, place protection using the realized fill anchor.
4. If realized fill cannot be retrieved quickly enough, prefer a warning or no protective precision over a knowingly mis-anchored bracket.

That directly targets the current anchor bound:
- `|stop_error| = (1-s) |p_fill - p_anchor|`
- `|take_error| = (1+t) |p_fill - p_anchor|`

with `p_anchor = last_close` in the current code.

There is one more interaction to preserve. The live runner keeps only the latest pending bar while an earlier run is still in flight (`2011-2033`, `2052-2206`), and `pending_bar_keeps_latest_timestamp` at `3404-3445` explicitly tests that policy. I would keep that queueing policy for now, but I would gate resubmission on open-order state; otherwise the queueing simplification and the market-order simplification compound each other.

I am not fully certain the exact broker workflow you want for post-fill protection is the same as Alpaca’s one-shot bracket semantics, but the file already has enough order-state plumbing that the change can be implemented locally if that is the chosen product behavior.

### 5. Keep the ML change bounded

`ml_signal.rs` is still a secondary target.

`sample_deadband` is fixed at

\[
\max\!\left(2\,\text{cost\_frac},\ 0.25 \cdot \frac{\text{ATR}_h}{\text{open}_{t+1}}\right)
\]

in `ml_signal.rs:930-942`.

But `score_target` still acts on fixed `t_long`, `t_short`, and `delta` in `ml_signal.rs:3524-3539`, and the requested/desired positions are then deterministically overlaid in `ml_signal.rs:3600-3697`.

I would not force an arbitrary mapping from price deadband to probability gap without calibration. The bounded third-pass implementation inside this file is instead:

- keep `score_target` structurally simple,
- but add a threshold-brittleness penalty in candidate evaluation,
- so a policy is discounted if its robust score collapses under small perturbations of `t_long`, `t_short`, or `delta`.

That keeps ownership inside `ml_signal.rs` and adds uncertainty-awareness at the decision boundary without inventing a fake probability-to-price conversion.

### 6. Patch order

If I were sequencing implementation inside the current ownership boundary, I would do it in this order:

1. `options_types.rs` / `options_expression.rs`: make `DerivedMidSnapshot` actually mid-aware and activate spread/staleness gating.
2. `options_expression.rs`: extract one shared barrier-resolution helper and update scalar + compiled paths together.
3. `commands_execution.rs`: reconcile open orders before resubmission and anchor protection to realized fills rather than `last_close`.
4. `ml_signal.rs`: add a threshold-brittleness penalty rather than a forced deadband-to-probability translation.

That is the smallest implementation program that matches the overlay’s new lemma, counterexample, and bound while staying inside the owned surfaces.

