# OX-01 Sensitivity, neighborhood geometry, and local fragility math

This document consolidates my first-pass, second-pass, and third-pass answers for the uploaded OX-01 overlay prompt.

Prompt basis: the uploaded `PROMPT.md` defines OX-01 as a sensitivity / neighborhood-geometry overlay, lists the most relevant repo surfaces, and requires the third pass to produce implementation-facing implications of a new lemma, counterexample family, or bound without redesigning adjacent PR ownership surfaces.

## Repo anchors used

- `double_ma.rs`: lines 476-559, 2045-2187, 2951-2955, 3178-3275, 4697-4953, 7701-7725
- `composite_cpu.rs`: lines 11100-11297, 11855-12099
- `composite_mo.rs`: lines 107-125, 409-435, 1213-1570
- `options_expression.rs`: lines 521-625
- `ml_signal.rs`: lines 1556-1624, 3738-3769, 3982-4041
- `validation_pipeline.rs`: lines 438-561, 1528-1646

---

## First-pass answer

I treated the uploaded overlay as attaching most naturally to the `double_ma.rs` ownership surface first, then `composite_cpu.rs`, with smaller but still real spillover into `composite_mo.rs`, `options_expression.rs`, `ml_signal.rs`, and `validation_pipeline.rs`.

The strongest first-pass finding is that `double_ma.rs` already contains a substantial part of the OX-01 machinery. It has a dedicated `DoubleMaLocalStabilityRequest`, `LocalStabilityPoint`, `LocalStabilitySummary`, and `LocalStabilityResult`, so the codebase already knows how to evaluate an exact local lattice around a chosen winner. It also already defines `ShortlistNeighborhoodRobustnessSummary`, with fields such as `plateau_score`, `peak_delta`, `basin_floor_stability`, `roughness_rms_delta`, and `curvature_penalty`. So the repo is not merely selecting a best point; it is already trying to describe the local surface around that point.

The missing piece is directional exactness. The current robustness summary is mostly scalar. It tells you whether a winner is broadly robust, but it can hide anisotropy. A point can be very stable along `slow_len` and very fragile along `fast_len`, and a scalar blend can still label it “moderate.” The natural OX-01 addition is to expose per-axis local differences

- `Δ_f^- = F(f-h,s) - F(f,s)`
- `Δ_f^+ = F(f+h,s) - F(f,s)`
- `Δ_s^- = F(f,s-h) - F(f,s)`
- `Δ_s^+ = F(f,s+h) - F(f,s)`

and, when both neighbors exist,

- `Δ_f^2 = F(f-h,s) - 2F(f,s) + F(f+h,s)`
- `Δ_s^2 = F(f,s-h) - 2F(f,s) + F(f,s+h)`.

Those are already cheap because `summarize_shortlist_neighborhood_robustness` is already scanning the axial neighbors.

The first useful lemma candidate is this:

**Path-invariant plateau lemma.** On any neighborhood cell where the induced post-warmup desired-position path is unchanged, every path-dependent metric and therefore every local difference vanishes exactly. In this repo, true plateaus are often exact path-invariant cells rather than merely “similar-score” regions.

This matters because `double_ma.rs` is not optimizing a smooth analytical surface. Its evaluation path includes `trade_on_next_bar`, execution overlays, stop-loss / take-profit logic, and Kaufman-stop exits. A one-step MA perturbation can move a crossover by one bar; that can change whether an entry is delayed, a stop is hit, or a flip occurs at all. So the real local geometry is piecewise-flat cells separated by path-change cliffs.

That gives the first counterexample family: construct candles where the fast and slow averages are nearly tied over a narrow window, then turn on cooldown, persistence, or stop logic. A ±1 period change shifts a crossover by one bar, and that single bar changes realized trades materially. In this family, the search lattice has unstable cliffs rather than a smooth ridge.

`composite_cpu.rs` is the next most relevant surface because it adds a second sensitivity layer: prescan admissibility. Its stage-0 prescan can narrow the `(fast_len, slow_len)` set to `allowed_pairs` around prescan winners, optionally augmented by predictive rescue. That means sensitivity is no longer only about the final score surface; it is also about whether prescan retained the right neighborhood at all. The cheapest new summary there is prescan neighborhood recall or allowed-pair coverage around the eventual winner.

`composite_mo.rs` shows the same prescan geometry even more explicitly. It already exposes `candidate_pairs`, `pair_density`, and per-type candidate telemetry, and it can rewrite the search domains to categorical candidate sets. That makes OX-01’s admissibility geometry fully concrete: exactness after prescan is exactness on a pruned subset, not on the full lattice.

`options_expression.rs` and `ml_signal.rs` are different. They are not smooth score-surface problems so much as staircase and assignment-boundary problems. `options_expression.rs` assigns search, holdout, and sealed-test folds using `floor(total * ratio)` plus clamping, so its split surface is piecewise constant with abrupt jumps. `ml_signal.rs` uses stable-hash ticker holdout and integer rolling windows, and its policy sweep then ranks by `robust_score`. For both files, the right cheap sensitivity summary is not a derivative but a boundary margin: how many events, sessions, bars, or residue classes away the next split or verdict flip lies.

`validation_pipeline.rs` fits the same threshold story. Its verdict logic hard-gates on approval, minimum track record, and SPA significance. The right OX-01 summary there is gate margin, not raw score alone.

So the first-pass verdict is:

- `double_ma.rs` already partially implements the overlay, but it underreports directional fragility.
- `composite_cpu.rs` is the next-best attachment because prescan pruning creates a second local-fragility layer.
- `composite_mo.rs` makes prescan admissibility explicit through domain rewriting and pair-density telemetry.
- `options_expression.rs`, `ml_signal.rs`, and `validation_pipeline.rs` are mostly staircase / threshold problems, so boundary-margin diagnostics are more appropriate than derivative language.

---

## Second-pass answer

Keeping the same ownership choice, the second pass changes the review in one important way: it forces a distinction between three different notions of exactness.

The first notion is **full-lattice exactness**. That is the raw score functional the optimizer evaluates at each parameter point.

The second notion is **common-start neighborhood exactness**. In `double_ma.rs`, `compute_double_ma_local_stability_blocking_with_candles` does not evaluate each neighbor at its own period-specific warmup boundary. Instead, it builds the whole radius/step neighborhood, takes the maximum period in that neighborhood, and chooses a single `start_t` from that shared maximum so all local points are directly comparable on the same suffix of the series. `composite_cpu.rs` does the same thing for its local stability path. So the local finite differences computed from `LocalStabilityResult.points` are exact for a common-start neighborhood functional, not automatically for the raw full-grid functional.

That gives the second-pass lemma:

**Common-start neighborhood lemma.** Let `F` be the raw search-lattice score and let `F̃` be the neighborhood score induced by evaluating every point in a radius-`r`, step-`h` local block from the same `start_t`. Then the first and second differences derived from `LocalStabilityResult` are exact differences of `F̃`, not necessarily of `F`. The mismatch between `F` and `F̃` is confined to the warmup prefix excluded by the shared start.

That is precisely the kind of exactness shift the core prompt would likely miss.

The third notion is **prescan-pruned exactness**. `composite_cpu.rs` can use a stage-0 double-MA prescan with `top_k`, `window`, and optional predictive rescue to build `allowed_pairs`, and the later search is then exact only on that admissible subset. `composite_mo.rs` goes further: it can rewrite its domains to categorical candidate sets and retain only `allowed_pairs_by_type`. Once that rewrite happens, later search is exact only on the pruned categorical subset.

That yields the next lemma:

**Admissibility lemma.** After prescan, `CompositeCpu` and `CompositeMo` are globally exact only on the retained candidate subset. If the true full-lattice optimum lies outside the prescan windows and rescue seeds, later exact evaluation cannot recover it.

That in turn changes the adversarial family. The right counterexample is not just a noisy or rough surface. It is a thin diagonal ridge or isolated spike that lies outside the union of prescan windows around the prescan winners. Objective-only prescan can prune it away; coarse predictive rescue can miss it if the spike lies between coarse stride points. Under the core prompt alone, one might still speak loosely about “the optimum.” OX-01 requires the sharper statement that any later optimum is conditional on prescan recall.

Back in `double_ma.rs`, the second pass also changes how I read the existing neighborhood summary. The code already computes scalar roughness and curvature penalties, but scalarization is isotropic. A one-axis ridge can still be hidden inside a moderate aggregate label. So the cheapest missing exact quantities are still the per-axis one-step differences and the per-axis centered curvatures, plus a small anisotropy ratio like

`K = (max(|Δ_f^-|, |Δ_f^+|) + ε) / (max(|Δ_s^-|, |Δ_s^+|) + ε)`.

Those would be exact on the current common-start local functional and would cost essentially nothing beyond the existing neighborhood scan.

The path-discontinuity argument also becomes sharper in the second pass. `double_ma.rs` is evaluating a trading state machine with `trade_on_next_bar`, execution overlays, Kaufman-stop exits, and stop / take-profit handling. That means the correct local picture is not “small parameter change implies small score change,” but “piecewise-flat path cells separated by execution cliffs.” OX-01 upgrades the adversarial story from vague noise to concrete path-change boundaries.

The other files reinforce the staircase view:

- `options_expression.rs` uses `floor(total * ratio)` plus clamping and session bucketing. Its split map is a staircase in ratio space.
- `ml_signal.rs` uses stable-hash ticker holdout and integer rolling windows. Its fragility is measured in bars, residue classes, or policy ticks to the next assignment change.
- `validation_pipeline.rs` computes SPA `p_value = exceed / bootstrap_samples` and passes only when `p_value <= 0.05`. So its p-value resolution is exactly `1 / bootstrap_samples`. At the clamp floor of 100 bootstrap samples, that resolution is `0.01`.

So the second-pass verdict is:

- `double_ma.rs` and the implemented `composite_cpu.rs` local-stability paths provide exact differences on a common-start neighborhood surface, not necessarily on the raw full-grid surface.
- `composite_cpu.rs` and especially `composite_mo.rs` are exact only on the admissible subset after prescan.
- `options_expression.rs`, `ml_signal.rs`, and `validation_pipeline.rs` are dominated by staircase boundaries and gate margins rather than smooth curvature.

---

## Third-pass answer

Keeping the same ownership choice, the third pass turns the review into a patch plan. The highest-confidence implementation target is still `double_ma.rs`, because that file already owns all three ingredients needed for OX-01 to become concrete product behavior:

1. it owns the local-neighborhood evaluation request / result types,
2. it already computes the local neighborhood from a single shared `start_t`, and
3. it already attaches a neighborhood robustness summary to the winning shortlist candidate.

That means the best third-pass move is **not** to design a new cross-repo fragility framework. It is to extend the existing `double_ma.rs` summary so it reports exact axiswise local geometry at zero additional backtest cost.

### Third-pass lemma and its implementation consequence

The new implementation lemma is:

**Zero-rerun common-start axis lemma.** Because `compute_double_ma_local_stability_blocking_with_candles` already evaluates every neighbor in a radius/step block from one shared `start_t`, any one-step axial difference, centered axial second difference, and axiswise plateau width computed from `LocalStabilityResult.points` is an exact summary of that already-materialized local surface. No extra grid search is required.

Implementation consequence: the cheapest high-value patch is to enrich `ShortlistNeighborhoodRobustnessSummary` rather than adding a new evaluator.

### What I would add in `double_ma.rs`

I would extend the neighborhood summary with explicit axiswise fields. Conceptually:

- `fast_left_delta`, `fast_right_delta`
- `slow_down_delta`, `slow_up_delta`
- `fast_curvature`, `slow_curvature`
- `fast_axis_half_width`, `slow_axis_half_width`
- `anisotropy_ratio`
- `dominant_fragility_axis`
- `cliff_flag`

The formulas are direct:

- `Δ_f^- = S(f-h,s) - S(f,s)`
- `Δ_f^+ = S(f+h,s) - S(f,s)`
- `κ_f = S(f-h,s) - 2S(f,s) + S(f+h,s)`
- `Δ_s^- = S(f,s-h) - S(f,s)`
- `Δ_s^+ = S(f,s+h) - S(f,s)`
- `κ_s = S(f,s-h) - 2S(f,s) + S(f,s+h)`

and

- `anisotropy_ratio = (max(|Δ_f^-|, |Δ_f^+|) + ε) / (max(|Δ_s^-|, |Δ_s^+|) + ε)`.

`fast_axis_half_width` and `slow_axis_half_width` can be computed from the already-available axial points by walking outward until the drop from base exceeds a tolerance `τ`. A reasonable first choice is `τ = 0.05 * score_scale`, where `score_scale` is the same normalization already used by the summary.

This patch lives exactly where the code already computes `roughness_rms_delta` and `curvature_penalty`, so it is implementation-local and does not widen ownership.

### Why this is the right patch

The current summary already identifies neighbors, roughness, curvature, and plateau score. But it still hides one-axis ridges. OX-01 asks us to identify whether the optimum is a plateau, ridge, or cliff on the discrete lattice. A scalar summary cannot do that reliably. Axiswise deltas and centered curvatures can.

This patch also fits the existing output path. `run_double_ma_optimization_blocking_with_candles` already computes a `DoubleMaLocalStabilityRequest`, evaluates the local block around the best result, and then calls `summarize_shortlist_neighborhood_robustness` before attaching the result to diagnostics. So adding fields to the robustness summary automatically enriches the already-existing optimization response.

### The third-pass counterexample and the cheapest boundary proxy

The new counterexample family is the one-bar execution-boundary family: price paths where a ±1 change in `fast_len` or `slow_len` shifts a crossover by one bar, and that single-bar change causes a different stop-loss, take-profit, Kaufman-stop, or delayed-entry outcome.

`double_ma.rs` already makes this family realistic because the evaluation path includes:

- `trade_on_next_bar`,
- execution overlays,
- Kaufman-stop handling,
- stop-loss / take-profit exits.

So I would add one more cheap product-facing diagnostic: a **path-change proxy**, not an exact path-equality test. Exact path equality would require materializing full desired-position or trade-path traces for every local point, which is more invasive. But the metrics already carry `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, `gross_loss`, `max_dd`, `exposure`, and `net_exposure`. So a good third-pass compromise is to count how many one-step axial neighbors change any of these coarse path signatures.

For example:

- `changed_closed_trades_neighbors`
- `changed_exposure_neighbors`
- `changed_drawdown_neighbors`

These are not perfect same-path detectors, but they are cheap and very good cliff proxies. On a true flat plateau, they will usually stay unchanged. Near a path-change boundary, they often jump immediately.

### What I would not do in this pass

I would **not** try to redesign `composite_cpu.rs`, `composite_mo.rs`, `options_expression.rs`, `ml_signal.rs`, and `validation_pipeline.rs` around one shared sensitivity trait in the same patch. That would widen ownership and violate the third-pass constraint.

Instead:

- `composite_cpu.rs` should be a follow-on that copies the same axiswise local-summary math into its existing local-stability endpoint. It already uses the same common-start neighborhood pattern, so the math transfers directly.
- `composite_mo.rs` should get telemetry-only additions such as local prescan recall, rescue share, and best-window allowed-pair coverage. Its `candidate_pairs` and `pair_density` telemetry already make that natural.
- `options_expression.rs`, `ml_signal.rs`, and `validation_pipeline.rs` should get boundary-margin diagnostics later, not derivative-style fields.

### The narrow follow-on I would allow for `composite_cpu.rs`

If I were allowed one tiny follow-on inside the same general ownership family, it would be this: add a small helper local to `composite_cpu.rs` that mirrors the new `double_ma.rs` axis-difference calculations on top of `LocalStabilityResult`.

Why only that much?

Because `composite_cpu.rs` already has a local-stability endpoint and the same common-start logic, but its implementation coverage is profile-dependent. It supports local stability for the base path plus VEMR, VCC, ASR, and ROCB, while VCET, MKCS, KQSR, CARM, KVDF, and CDMC still return “not implemented yet.” So the safe implementation-facing conclusion is: keep the main OX-01 patch in `double_ma.rs`, and only mirror the zero-rerun local summaries into the already-supported `CompositeCpu` local-stability paths later.

### Small telemetry addition for prescan surfaces

The new admissibility lemma has one cheap implementation implication for prescan code: expose local retention coverage around the prescan seed.

For `composite_cpu.rs`, after `allowed_pairs` is constructed, a tiny telemetry field like

- `best_pair_window_coverage = kept_pairs_in_window / possible_pairs_in_window`

would immediately tell you whether prescan retained a healthy local basin around the prescan winner or just a narrow spike. That can be computed from the existing `pair_set` and `prescan_window` with no extra backtests.

For `composite_mo.rs`, the same idea should be telemetry-only and fit naturally beside the existing `candidate_pairs` and `pair_density` metrics.

### Boundary-margin diagnostics for the non-smooth files

I would leave these outside the current implementation boundary, but the third-pass answer should still say what the correct cheap summaries are:

- `options_expression.rs`: distance in events or sessions to the next split-boundary jump.
- `ml_signal.rs`: bars-to-next-window-flip and hash-residue distance for ticker holdout.
- `validation_pipeline.rs`: `p_value_resolution = 1 / bootstrap_samples`, `p_margin = 0.05 - p_value`, and minimum-track-record slack.

Those summaries are more faithful than derivatives because those files are staircase or hard-gate systems.

### Final third-pass verdict

The third-pass conclusion is simple: ship the OX-01 implementation first as a **`double_ma.rs` summary-enrichment patch**, not as a new optimizer.

That patch should:

- add exact axiswise first differences and centered second differences,
- add axiswise plateau widths,
- add an anisotropy label so ridges are visible,
- add a cheap path-change proxy based on existing metrics,
- keep the semantics explicit that these are summaries of the existing common-start local surface.

That satisfies the overlay’s theorem obligation, stays within the current ownership boundary, costs almost nothing computationally, and makes the current “plateau / robust / fragile” reporting materially more exact.

---

## Brief source mapping

- `double_ma.rs` local-stability request/result and robustness summary structs: lines 476-559.
- `double_ma.rs` robustness computation, including roughness and curvature: lines 2045-2187.
- `double_ma.rs` local-stability evaluator using shared neighborhood `start_t`: lines 4697-4953.
- `double_ma.rs` optimization path attaching local robustness to the winner: lines 7701-7725.
- `double_ma.rs` trade metric enrichment fields: lines 2951-2955 and 3398-3412.
- `double_ma.rs` execution discontinuities (`trade_on_next_bar`, Kaufman stops, stop / take-profit): lines 3178-3275.
- `composite_cpu.rs` stage-0 prescan and `allowed_pairs` narrowing: lines 11100-11297.
- `composite_cpu.rs` local-stability endpoint, shared-start logic, and unsupported profiles: lines 11855-12099.
- `composite_mo.rs` prescan config / predictive rescue / telemetry / domain rewrite: lines 107-125, 409-435, 1213-1570.
- `options_expression.rs` split assignment staircase: lines 521-625.
- `ml_signal.rs` stable-hash holdout, rolling split windows, robust-score ranking: lines 1556-1624, 3738-3769, 3982-4041.
- `validation_pipeline.rs` verdict hard gates and SPA p-value granularity: lines 438-561, 1528-1646.
