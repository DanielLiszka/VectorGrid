# OX-06 Tail-risk, drawdown-path, and rare-event math review

Treating `OX-06` as the standalone overlay target, because the paired core `PR-*` prompt is not present in the bundle. The overlay asks for a tail-risk and rare-event lens over five owned surfaces: `composite_gpu_metric_decode.rs`, `iterative_sim.rs`, `validation_pipeline.rs`, `options_expression.rs`, and `options_expression_gpu.rs`. fileciteturn17file0

---

## First-pass answer

Treating `OX-06` itself as the first-pass prompt, since the paired core `PR-*` prompt is not in the bundle. The strongest first-pass conclusion is that this codebase is **under-measuring path-dependent downside mainly because it is under-retaining it**, not because the runtime paths cannot see it. The overlay’s ask is exactly to look for path-shape and rare-event summaries that can be added cheaply without widening ownership, and these files do expose that opportunity. fileciteturn17file0

The clearest gap is in `iterative_sim.rs`. The iterative validator already computes path-dependent drawdown ingredients such as running peak, drawdown, and ulcer accumulation, but the returned `Metrics` still hard-code several tail/path fields to zero: `downside_frequency`, `downside_severity`, `kappa_tau`, `long_mean_return`, `short_mean_return`, `underwater_fraction`, and `max_underwater_duration_bars`. So the validation path is observing downside shape but then throwing away exactly the quantities the overlay wants to preserve. fileciteturn19file14

`validation_pipeline.rs` then ranks candidates using only a narrow risk summary. `robust_score_from_metrics` depends on Sharpe-like, max-drawdown-like, ulcer-like, Martin-like, and downside-vol-ratio-like terms, and `risk_warning_flags_from_metrics` only emits warnings for high ulcer, weak Martin, and high downside-vol ratio. The walk-forward summaries retain medians and worst cases for Sharpe, drawdown, PnL, downside-vol ratio, ulcer, Martin, and trades, but nothing about underwater persistence or CVaR-style tail means. Perturbation stats are similarly limited to mean/worst Sharpe, max drawdown, PnL, and positive-PnL fraction. fileciteturn19file15 fileciteturn24file9

There is one existing tail-aware surface in `validation_pipeline.rs`: trade bootstrap already keeps simulated PnL and max-drawdown samples and returns quantiles such as `pnl_p05` and `max_dd_p95`, including stationary-bootstrap variants. But even there, the retained summary stops at quantiles; it does not compute expected shortfall / CVaR, drawdown-duration tails, or clustered-loss summaries. Since the code already builds bootstrap summaries and already has in-place quantile helpers, adding CVaR-style summaries would be a low-marginal-cost extension rather than a redesign. fileciteturn20file14 fileciteturn24file7

The options path shows the same pattern. `options_expression.rs` exact evaluation tracks path extrema per trade through path highs/lows and stop/target exit reasons. But the summary-cache path explicitly says it only needs PnL, premium, and return, meaning the path replay computes adverse-path information and then discards it before ranking. Downstream tuple ranking is then based on lane utility `(1 - invalid_rate) * expectancy_per_premium`, blended across search / holdout / stress lanes, so a rare catastrophic-left-tail tuple can still look identical to a smooth one if those two aggregates match. fileciteturn19file18 fileciteturn21file13

On the GPU side, the retained options aggregate is intentionally tiny: valid count, win count, total PnL, total premium, sum of return-on-premium, and sum of squared return-on-premium. The device-side per-tuple output is only 24 bytes. That makes `OX-06`’s low-cost tail-summary question especially relevant here: adding one or two extra floats would be a bounded extension of an existing compact-state design, not a conceptual departure. fileciteturn23file13 fileciteturn23file14

`composite_gpu_metric_decode.rs` is not the main blocker. It already supports a 19-field non-double-MA row contract with optional tail fields for `downside_frequency`, `downside_severity`, `long_mean_return`, `short_mean_return`, `kappa_tau`, `underwater_fraction`, and `max_underwater_duration_bars`; if omitted, they default to zero. So the decoder is already prepared to accept richer tail state. The missing work is upstream production and retention of those fields, not decode shape. fileciteturn19file4 fileciteturn19file17

### First-pass lemma candidate

**Lemma candidate — current robust ranking is non-identifiable with respect to drawdown persistence.** Under the present summary map, two candidates with identical values of `(sharpe_like, max_dd_like, ulcer_like, martin_like, downside_vol_ratio_like)` receive the same robust score even if they have very different underwater fraction or longest underwater spell. This is not because the underlying path lacks that information; in the iterative path the drawdown process is already traversed bar by bar, but those persistence fields are returned as zeros. So the non-identifiability is induced by the retained summary, not by the simulation semantics. fileciteturn19file15 fileciteturn19file14

### First-pass counterexample families

**Counterexample family A — same drawdown depth, different recovery shape.** Construct two equity curves with the same final PnL, same max drawdown, and even similar ulcer index, but one recovers quickly after each loss while the other stays underwater for long contiguous stretches. The current robust score and warning flags do not see that distinction, because underwater persistence is absent from both iterative output and ranking inputs. fileciteturn19file14 fileciteturn19file15

**Counterexample family B — same options expectancy, different rare-loss tail.** Construct two options tuples with the same invalid rate and expectancy per premium, but one has many small wins plus occasional severe stop-out losses, while the other has moderate symmetric outcomes. Current lane utility ties them if those two aggregates match, because the ranking summary retains no left-tail excursion or stop-cluster proxy even though exact replay already computes path highs/lows. fileciteturn19file18 fileciteturn21file13

### First-pass bound

**Bound — adding a small tail-retention payload is cheap in both CPU and GPU summaries.** For options GPU ranking, current retained output is 24 bytes per tuple. Adding one or two extra floats raises that to only 28 or 32 bytes per tuple, still `O(tuple count)` and independent of event count. For validation bootstrap, CVaR-style stats can be computed from vectors already materialized in memory using one quantile threshold plus one extra linear pass, with no asymptotic memory increase. fileciteturn23file14 fileciteturn24file7

### First-pass recommendation

Within the current file boundaries, the highest-leverage fixes are retained-summary fixes, not replay redesigns:

- In `iterative_sim.rs`, stop zeroing the cheapest path-persistence fields: `downside_frequency`, `downside_severity`, `underwater_fraction`, and `max_underwater_duration_bars` first, then `long_mean_return`, `short_mean_return`, and possibly `kappa_tau`. fileciteturn19file14
- In `validation_pipeline.rs`, promote tail summaries from quantile-only toward quantile-plus-tail-mean, especially bootstrap PnL expected shortfall and drawdown-tail means. fileciteturn20file14
- In `options_expression.rs` and `options_expression_gpu.rs`, retain at least one left-tail or adverse-path proxy in the compact summary instead of only mean/variance-style return aggregates. fileciteturn19file18 fileciteturn23file14

So the first-pass verdict is: **yes, the owned problem is under-measuring tail/path downside; the highest-value fixes are small retained-summary additions on paths that already observe the relevant information.**

---

## Second-pass answer

Again treating `OX-06` as the second-pass target in isolation, because the paired core `PR-*` second-pass prompt is not in the bundle. Under this overlay, the main second-pass change is: the current code is often exact for a **reduced retained statistic**, not exact for **tail-sufficient ranking**. fileciteturn17file0

In `iterative_sim.rs`, the bar walk already updates running peak, drawdown, ulcer accumulation, and negative-return ingredients, but the returned `Metrics` still hard-code `downside_frequency`, `downside_severity`, `kappa_tau`, `long_mean_return`, `short_mean_return`, `underwater_fraction`, and `max_underwater_duration_bars` to zero. So the simulation semantics are richer than the exported tail semantics. `composite_gpu_metric_decode.rs` already knows how to decode exactly those optional tail fields when present, which means the missing exactness is mostly an upstream retention problem, not a decoder-boundary problem. fileciteturn19file14 fileciteturn19file4

In `validation_pipeline.rs`, ranking exactness factors through a very small projection. `robust_score_from_metrics` only depends on Sharpe-like, max-drawdown-like, ulcer-like, Martin-like, and downside-vol-ratio-like terms, and `risk_warning_flags_from_metrics` only looks at ulcer, Martin, and downside-vol ratio. Trade bootstrap already materializes tail sample vectors but only retains quantiles, not expected shortfall or drawdown-duration tails. Under `OX-06`, the exactness statement therefore has to be weakened from “validation is exact for downside robustness” to “validation is exact for this explicit low-dimensional projection of downside robustness.” fileciteturn19file15 fileciteturn20file14

`options_expression.rs` makes the same issue even sharper. Exact trade replay tracks path extrema and stop/target exits, but the summary-cache path explicitly says it only needs PnL, premium, and return. The robust scorer itself depends only on per-lane `invalid_rate` and `expectancy_per_premium`, and there is a test proving that equal invalid rates reduce the score to a weighted expectancy blend. So second-pass exactness here is not “exact tail-aware tuple ranking”; it is “exact aggregate-first ranking for the lane-utility projection the code currently chose.” fileciteturn19file18 fileciteturn21file13 fileciteturn21file17

### Second-pass lemmas

**Validation projection lemma.** If two candidates induce the same tuple `(sharpe_like, max_dd_like, ulcer_like, martin_like, downside_vol_ratio_like)`, then `robust_score_from_metrics` and `risk_warning_flags_from_metrics` treat them as identical, regardless of any difference in underwater fraction, recovery duration, or clustered-loss structure. fileciteturn19file15

**Options lane-utility lemma.** If two tuples match on `invalid_rate` and `expectancy_per_premium` on every scored lane, then current `robust_score` ties them, regardless of skew, CVaR, max adverse excursion, or stop-hit concentration. The equal-invalid-rate test makes that collapse explicit. fileciteturn21file13 fileciteturn21file17

### Second-pass adversarial constructions

- **Drawdown-persistence aliasing:** same Sharpe / max drawdown / ulcer / Martin / downside-vol ratio, different underwater duration. fileciteturn19file15 fileciteturn19file14
- **Rare-crash options aliasing:** same lane invalid rates and expectancy-per-premium, different rare crash profile. fileciteturn19file18 fileciteturn21file13
- **CVaR aliasing in bootstrap:** same quantiles, different mean loss in the worst tail. fileciteturn20file14

### Second-pass bounds

- In `iterative_sim.rs`, exposing persistence fields is an `O(1)`-state extension of a loop that already computes the needed ingredients. fileciteturn19file14
- In `validation_pipeline.rs`, CVaR-style tail means for PnL and drawdown need no new asymptotic storage, because the bootstrap vectors already exist. fileciteturn20file14 fileciteturn24file7
- In `options_expression_gpu.rs`, one added float would raise per-tuple retained output only from 24 bytes to 28 bytes; two would make it 32 bytes. fileciteturn23file14

So the second-pass verdict is: **under `OX-06`, the right theorem is not “the owned paths are already exact enough,” but “they are exact for a deliberately coarsened statistic.”** The overlay changes the target from exact replay alone to **tail-sufficient exact replay plus tail-sufficient retention**.

---

## Third-pass answer

Treating `OX-06` again as the standalone third-pass target, because the paired core `PR-*` third-pass prompt is not in the bundle. Under the third-pass overlay, the right deliverable is an implementation-facing patch plan that breaks the new aliasing lemmas **without leaving the five owned surfaces**. fileciteturn17file0

The key third-pass corollary is this: the missing tail fidelity is not blocked by simulation semantics; it is blocked by where the current code **drops** information. So the best implementation plan is to change the **producer and retention points that already exist**, not to add a new replay stage. `iterative_sim.rs` already walks the full drawdown path but zeros persistence fields on return. `validation_pipeline.rs` already stores the bootstrap and walk-forward aggregates needed for simple tail extensions. `options_expression.rs` already owns exact replay and exactify/finalization logic around ranked tuples, even though the compact ranking path collapses to lane utility. fileciteturn19file14 fileciteturn19file15 fileciteturn19file18 fileciteturn21file16

### Third-pass implementation implication 1 — producer-side fix in `iterative_sim.rs`

**Implementation corollary of the validation projection lemma:** if `iterative_sim.rs` keeps exporting zeros for persistence fields, any downstream tail-aware change in `validation_pipeline.rs` is mostly cosmetic. So the first patch has to be producer-side.

Inside the existing bar loop, add only `O(1)` accumulators:

- `downside_count` and `downside_sum_abs` for `downside_frequency` and `downside_severity`
- `underwater_bars`, `current_underwater_run`, and `max_underwater_run` for `underwater_fraction` and `max_underwater_duration_bars`
- optionally `long_sum/long_count` and `short_sum/short_count` for side-specific means

That stays inside the current loop that already computes peak, drawdown, max drawdown, ulcer accumulation, and exposure. The implementation consequence is direct: **export the path-shape fields the loop already observes instead of recomputing them later.** fileciteturn19file14

### Third-pass implementation implication 2 — summarize tails where vectors already exist in `validation_pipeline.rs`

`validation_pipeline.rs` is currently projection-heavy: robust score uses only Sharpe, max drawdown, ulcer, Martin, and downside-vol ratio, while perturb summaries retain only mean/worst Sharpe, drawdown, and PnL. That means the cheapest third-pass change is not a new validation subsystem; it is a few extra retained fields on summaries that are already being built. fileciteturn19file15 fileciteturn24file9

The direct file-local additions are:

1. **Bootstrap expected shortfall fields** in `compute_trade_bootstrap_stats`, such as:
   - `pnl_es_p05`: mean of bootstrap PnLs in the worst 5% tail
   - `max_dd_tail_mean_p95`: mean of bootstrap drawdowns in the worst 5% drawdown tail
   - stationary-bootstrap versions of the same fields

   This is cheap because the bootstrap path already materializes the PnL and drawdown sample vectors and already has an in-place quantile helper. The implementation is one threshold computation plus one extra linear pass per vector. fileciteturn20file14 fileciteturn24file7

2. **Walk-forward and reopt-walk-forward persistence summaries**, once `iterative_sim.rs` exports them:
   - median / worst `underwater_fraction`
   - median / worst `max_underwater_duration_bars`

   The walk-forward and reopt-walk-forward code already accumulate vectors of forward Sharpe, drawdown, PnL, ulcer, Martin, and trades. Extending those structs by a couple more vectors stays squarely inside the current summarization pattern. fileciteturn24file8 fileciteturn24file19

3. **Warning-flag promotion before ranking promotion.** The safest third-pass change is to first expose new warning flags for persistence and tail means, then decide whether to widen `robust_score_from_metrics`. That breaks the “silent aliasing” failure mode without forcing an immediate scoring redesign. fileciteturn19file15

### Third-pass implementation implication 3 — fix pre-truncation aliasing in `options_expression.rs`

The most important implementation implication from the options lane-utility lemma is not just “add a new metric.” It is this:

**If ranking truncates to `top_k` before exact tail enrichment, then any tail-aware metric computed only during finalization cannot rescue a tuple that was dropped earlier.**

That is exactly what the current CPU ranking order does: it builds `ranked_results`, sorts, truncates to `top_k`, and only then calls `finalize_ranked_results`. At the same time, the exact trade evaluator already knows path highs/lows and stop/target exits, while the summary-cache path explicitly discards that detail. So the boundary-safe third-pass fix is:

1. keep a **bounded surplus shortlist** before finalization, not just `top_k`
2. exactify / enrich that surplus shortlist using the already-owned exact replay path
3. rerank with one new tail-aware field
4. only then truncate to final `top_k`

This change is local to `options_expression.rs` and does **not** require a new GPU kernel contract. It uses the exact replay logic the file already owns to break the rare-crash alias on a bounded set of candidates. fileciteturn21file16 fileciteturn19file18

The best boundary-safe tail fields to add here are the ones exact replay can produce cheaply during exactify, for example:

- `stop_hit_rate`
- normalized adverse excursion proxy from path lows/highs
- left-tail return proxy such as cumulative negative return-on-premium on exactified trades

The point is not to add a full new path representation. The point is to use one extra exact tail discriminator on the bounded shortlist that already gets replayed. fileciteturn19file18

### Third-pass implementation implication 4 — keep `options_expression_gpu.rs` stable for the immediate patch

`options_expression_gpu.rs` shows that the current compact output is only 24 bytes per tuple. That is useful as a bound: if the boundary later widens to the kernel-side contract, the next cheap move is one extra float or two. But for the **immediate** third-pass patch, you do not need to widen the GPU aggregate yet if you adopt the bounded-surplus exactify strategy in `options_expression.rs`. fileciteturn23file13 fileciteturn23file14

So the third-pass implementation stance for `options_expression_gpu.rs` is:

- **no required immediate change** for the boundary-safe patch
- keep the 24-byte aggregate as-is for now
- document that the next cheap widening, if later allowed, is one or two tail floats rather than a replay redesign

That keeps implementation ownership inside the current boundary while still using the GPU file to establish the cost bound.

### Third-pass implementation implication 5 — `composite_gpu_metric_decode.rs` is already aligned

No major third-pass work is required in `composite_gpu_metric_decode.rs`. It already accepts optional tail fields and defaults them to zero when absent. So the implementation implication here is simply: **do not redesign the decoder; fix upstream production and retention first.** If any upstream non-double-MA producer inside scope later begins emitting those tail fields, the decode pattern is already established. fileciteturn19file4 fileciteturn19file17

### Third-pass bound and concrete patch order

A good bounded patch order inside the owned surfaces is:

1. **`iterative_sim.rs`** — export persistence and downside-frequency/severity fields already visible in the path walk. fileciteturn19file14
2. **`validation_pipeline.rs`** — add bootstrap ES / tail-mean fields and persistence-based warning summaries. fileciteturn19file15 fileciteturn20file14
3. **`options_expression.rs`** — delay final truncation, exactify a bounded surplus shortlist, add one exact tail discriminator, rerank, then final-truncate. fileciteturn21file16 fileciteturn19file18
4. **`options_expression_gpu.rs`** — leave layout unchanged for now; only use it to document the low-cost widening bound. fileciteturn23file14
5. **`composite_gpu_metric_decode.rs`** — no redesign needed. fileciteturn19file4

So the third-pass verdict is: **the cheapest implementation that actually changes behavior is not “add more theory,” but “move tail information across the existing producer/truncation boundaries.”** The highest-value boundary-safe change is exporting persistence from `iterative_sim.rs`, adding ES-style summaries in `validation_pipeline.rs`, and changing `options_expression.rs` so exact tail enrichment happens before final truncation rather than after it.
