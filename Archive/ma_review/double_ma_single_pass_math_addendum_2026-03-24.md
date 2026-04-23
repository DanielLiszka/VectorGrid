# Double MA Single-Pass Math Addendum
Date: 2026-03-24

This note is a **math-first addendum** to the two earlier markdown documents. It focuses on mathematically strong, additive-only ways to extract more useful work from the current single backtest optimization pass with minimal marginal cost.

It does **not** replace the earlier repo-grounded evaluation. It extends it.

## How this addendum relates to the prior two documents

- `double_ma_single_pass_extra_work_evaluation_2026-03-23.md` is still the **main repo-grounded baseline**: current state, ranked recommendation list, broad implementation order, and the first formal pass over the original brief.
- `double_ma_single_pass_validation_pullforward_2026-03-24.md` is a **focused supplement** on which validation-like features can be moved earlier into optimization without adding another replay/pass.
- **This document** is the **deeper math addendum**: it concentrates on exact row algebra, tiny extra sufficient statistics, metric-surface geometry, and the remaining parts of the original brief that were still only partially formalized.

So the right way to use the prior documents is: **use both**. The later validation note does **not** supersede the earlier main evaluation. This addendum sits on top of both.

---

## Visible current state this addendum relies on

The math below is grounded in the uploaded files.

1. The current preferred GPU product/search row for the Sharpe/Volatility family is the 10-row reduced contract in `gpu_backtest_kernel.rs:136-148`:
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

2. The host decode already reconstructs several derived metrics from that row in `gpu_backtest_kernel.rs:764-831`, including:
   - `sharpe = mean / std`
   - `sortino = mean / downside_deviation`
   - `calmar`
   - `martin_ratio`

3. The single-pass CUDA kernel already carries the relevant running state in `double_crossover.cu:3506-3519` and updates it in `double_crossover.cu:3566-3607`:
   - signed position `pos`
   - `log_eq`, `log_peak`, `max_log_dd`
   - `trades`
   - `sum_abs_pos`, `sum_pos`
   - `mean`, `m2`
   - downside accumulation
   - ulcer accumulation
   - positive and negative return mass for exact `omega`

4. The row write path in `double_crossover.cu:3729-3773` emits the current 10-row product contract in one pass.

5. The reduction path is already device-centric in `gpu_backtest_kernel.rs:4220-4405`, while device-side top-k scoring currently uses `objective_score_f32(...)` and `passes_constraints_f32(...)` in `double_crossover.cu:4418-4509`.

6. Only `min_trades` and `max_drawdown` are currently exposed as hard constraints in the uploaded code, via `double_ma.rs:176-180`, `double_ma.rs:793-810`, and `double_crossover.cu:4491-4509`.

7. GPU preflight / routing still blocks `Sortino` and `Omega` even though the current row already supports their decode, via `gpu_backtest_kernel.rs:3583-3614`, `gpu_backtest_kernel.rs:6396-6424`, `double_ma.rs:471-482`, and `double_ma.rs:9609-9612`.

Where I mention `validation_pipeline.rs` or the TypeScript request builder, that part is based on your description rather than direct file inspection here.

---

## The remaining mathematical opportunity space

The biggest remaining opportunity is **not** “more replay-like validation inside optimization.”
It is three layers of algebra on top of the current architecture:

1. **Row algebra** over the current 10-row output and known pair parameters.
2. **Tiny extra online sufficient statistics** inside the same bar walk.
3. **Metric-surface geometry** over the already-computed candidate matrix.

Those are the places where there is still material mathematical headroom.

---

## 1) Exact row algebra that is still not fully mined

These ideas do **not** require another historical pass.
Most of them do **not** require widening the current 10-row default output.

### 1.1 Active-time normalization family

This is the strongest remaining exact family that was not fully formalized in the earlier notes.

The kernel defines per-bar step return as

```text
step_r = 0                  if pos == 0
step_r = pos * ret1[t]      otherwise
```

in `double_crossover.cu:3585-3600`, while exposure is computed as `sum_abs_pos / n` in `double_crossover.cu:3731` from the accumulated `sum_abs_pos` state in `double_crossover.cu:3511` and `double_crossover.cu:3604-3606`.

That means the current row mixes:

- return quality while active, and
- the choice to be flat often.

Those are both useful, but they are different decisions. The current row is therefore rich enough to support a second family of **active-time-normalized** metrics.

Let

- `e = exposure`
- `μ = mean`
- `d = downside_deviation`
- `σ = std`

Then, under the visible zero-padding construction of `step_r`, the following are exact or nearly exact:

#### Exact active mean

\[
\mu_{act} = \frac{\mu}{\max(e,\varepsilon)}
\]

This is exact because flat bars contribute zero to the mean.

#### Exact active downside deviation

Because downside deviation is accumulated from squared negative `step_r` values and divided by the full observation count before the square root in `double_crossover.cu:3735-3736`, active-bar downside deviation is

\[
d_{act} = \frac{d}{\sqrt{\max(e,\varepsilon)}}
\]

#### Exact active Sortino

\[
Sortino_{act} = \frac{\mu_{act}}{d_{act}}
= \frac{\mu}{d\,\sqrt{\max(e,\varepsilon)}}
\]

This is exact under the visible zero-padding semantics.

#### Active Sharpe and active volatility

These are exact **if** pair-specific observation count is supplied or reconstructed from the known pair parameters and evaluation range.
That is feasible because the kernel’s start/end logic depends on `period_f`, `period_s`, `trade_offset`, and evaluation bounds in `double_crossover.cu:3486-3490`.

With total observation count `n` and active-bar count `a = e n`, the active sample variance is

\[
\sigma_{act}^2
= \frac{(n-1)\sigma^2 + n\mu^2 - a\left(\mu/e\right)^2}{\max(a-1,1)}
\]

so active Sharpe is

\[
Sharpe_{act} = \frac{\mu/e}{\sigma_{act}}
\]

If `n` is not carried explicitly, the large-`n` approximation is still extremely close on the 200k-bar benchmark surface, but that approximation should be labeled as such.

#### Why this matters

The current row already lets you distinguish between:

- strong candidates that are flat a lot but good when active, and
- candidates that look similar only because they are always in the market.

That opens a new exact composite family such as

\[
S = \alpha\,Sharpe + \beta\,Sortino_{act} - \gamma\,|net\_exposure|/e
\]

or

\[
S = \alpha\,\mu_{act} + \beta\,\log(\Omega+\varepsilon) - \gamma\,Ulcer
\]

with almost no marginal cost.

**Class:** exact / low-risk / mostly row-only

---

### 1.2 Drawdown-shape metrics: chronicity, not just magnitude

The current row already contains both:

- `max_dd`
- `ulcer_index`

and the kernel visibly accumulates both max drawdown and squared drawdown mass in `double_crossover.cu:3570-3583`, with final write in `double_crossover.cu:3730` and `double_crossover.cu:3737-3738`.

That means you can score **drawdown shape**, not just drawdown size.

A particularly useful exact derived scalar is drawdown chronicity:

\[
C_{dd} = \frac{Ulcer}{\max(MaxDD,\varepsilon)}
\]

Properties:

- `0 <= C_dd <= 1` when both are measured on the same path.
- low values correspond to brief/spiky pain,
- high values correspond to broad/persistent pain.

The inverse quantity

\[
R_{rec} = \frac{MaxDD}{\max(Ulcer,\varepsilon)}
\]

is a recovery-efficiency style diagnostic.

This is valuable because two candidates with identical `max_dd` can have very different “underwater shape.”
One may suffer one sharp event and recover quickly; the other may grind underwater for a long time.
The latter should usually be penalized more heavily.

A very practical composite is

\[
S = base\_score - \lambda\,C_{dd}
\]

or equivalently to add `C_dd` as a soft penalty or secondary tiebreaker.

**Class:** exact / low-risk / row-only

---

### 1.3 Downside-share and asymmetry metrics from the current row

The current row already contains enough information to create useful asymmetry indicators without adding trade-level state.

A simple one is downside-share:

\[
Q_{down} = \frac{downside\_deviation}{\max(std,\varepsilon)}
\]

This is not a probability, and the numerators/denominators are not normalized identically, so it should be described as a **shape proxy**, not a canonical textbook ratio.
But it is exact as a row-derived scalar and has real value:

- larger values mean a greater share of volatility is downside-shaped,
- smaller values mean more upside symmetry or more benign dispersion.

Another useful exact row-derived asymmetry feature is the gap between `log(omega)` and a symmetric-risk score such as Sharpe:

\[
\Delta_{\Omega,SR} = \log(\Omega + \varepsilon) - \alpha\,Sharpe
\]

for some fixed scaling `α`.

This is best treated as a **composite feature**, not a standalone objective, but it can highlight candidates whose upside/downside mass is meaningfully better than what a near-Gaussian symmetric-volatility view would suggest.

**Class:** exact / low-risk / row-only

---

### 1.4 Adversarial weight-envelope composite scoring

The earlier document already compared weighted sums, Tchebycheff, geometric, harmonic, epsilon-constraint, and Pareto-based approaches.
The mathematically strongest remaining extension is to treat user preference uncertainty itself as an optimization object.

Instead of one fixed weight vector `w`, choose a small finite set of plausible weight vectors `Λ = {w^(1), ..., w^(H)}` over transformed metrics `z(m)`, and score each candidate by its **worst** performance across those preference settings:

\[
S_{env}(m) = \min_{h \in \{1,\dots,H\}} \langle w^{(h)}, z(m) \rangle
\]

This is an adversarial or envelope-style robust score.

It is mathematically attractive because it prefers candidates that remain strong under several plausible user priorities rather than candidates that win only under one brittle weighting.

For this codebase it is also cheap:

- the expensive work is the bar walk,
- the current row matrix is only `N * 10 * 4` bytes,
- for `N ≈ 58,300`, one full scan of the 10-row matrix is only about `2.33 MiB`.

So even `H = 4..8` extra score heads is tiny compared with the single-pass backtest over `T = 200,000` bars.

This gives a strong exact design for “multi-objective robustness” without needing exact full Pareto extraction over the whole grid.

**Recommendation:** use `S_env` as a device or host-shortlist scalar mode, and use the union of the corresponding head shortlists as the exact multi-objective candidate set.

**Class:** exact / low-risk runtime / medium implementation

---

### 1.5 Exact commission-stress final PnL from the current row

This idea was introduced in the validation pull-forward note, but it is important enough to keep in the core math set.

The optimization kernel receives a single scalar `commission` via `KernelStrategyParams` in `gpu_backtest_kernel.rs:650-657` and `gpu_backtest_kernel.rs:681-684`, converts it to `log_comm = log(1 - commission)` in `double_crossover.cu:3431`, and applies it on trade events in `double_crossover.cu:3548-3561` and related risk-path branches. The exported row already contains `trades` in `double_crossover.cu:3753`.

If the signal path itself does not depend on cost level, then changing the scalar commission from `c0` to `c1` changes final log equity by

\[
\log(1 + pnl_{c1})
=
\log(1 + pnl_{c0})
+
E\big(\log(1-c_1)-\log(1-c_0)\big)
\]

where `E` is the exact internal cost-event count.

If exported `trades` matches cost-event semantics, then the current row already supports exact stressed final PnL.
If not, the clean additive extension is a dedicated latent `cost_event_count` accumulator.

This is a major result because it means a meaningful slice of friction stress can be computed with no replay.
It also supports a very cheap exact penalty such as

\[
Gap_{fee}(c_1) = pnl_{c0} - pnl_{c1}
\]

or a commission elasticity score.

**Class:** exact / low-risk for final-PnL stress / low implementation if event semantics are aligned

---

### 1.6 Effective-sample shrinkage and confidence-aware row scores

Another under-addressed math point is that pair-specific observation count differs with `(fast_len, slow_len)` through the kernel’s start logic in `double_crossover.cu:3486-3490`.

That means any estimator `\hat{q}` can be made more selection-robust by confidence shrinkage:

\[
S_{LCB}(\hat{q}) = \hat{q} - \frac{\lambda}{\sqrt{n_{obs}}}
\]

or more generally via a metric-specific lower-confidence bound.

For Sharpe this naturally becomes PSR/LCB, which was already emphasized in the validation note.
For other composite scores it becomes a generic way to prefer stable estimates over extreme but weakly supported ones.

This is cleaner than an arbitrary parameter penalty on `fast_len` or `slow_len`, because it penalizes the real statistical object: small effective sample size.

**Class:** exact / low-risk when `n_obs` is reconstructed from pair parameters; otherwise approximate if a global `n` is assumed

---

## 2) Tiny extra online state with high leverage

This is the next layer up: add a few more accumulators inside the same walk, but still avoid another pass.

### 2.1 Fixed-target partial moments: the best next exact risk family

The current kernel already computes zero-threshold downside mass and zero-threshold Omega via `sum_neg_sq`, `sum_pos_r`, and `sum_neg_abs_r` in `double_crossover.cu:3516-3519` and `double_crossover.cu:3593-3600`.

The mathematically strongest next exact family is to generalize that to a user-selected hurdle `τ`.

Define step-return lower and upper partial moments:

\[
LPM_1(\tau) = \sum_t \max(\tau - r_t, 0)
\]
\[
LPM_2(\tau) = \sum_t \max(\tau - r_t, 0)^2
\]
\[
UPM_1(\tau) = \sum_t \max(r_t - \tau, 0)
\]

Then you get, in the same pass:

\[
\Omega_{\tau} = \frac{UPM_1(\tau)}{LPM_1(\tau) + \varepsilon}
\]
\[
\kappa_1(\tau) = \frac{\mu - \tau}{LPM_1(\tau)/n}
\]
\[
\kappa_2(\tau) = \frac{\mu - \tau}{\sqrt{LPM_2(\tau)/n}}
\]

Why this is so good:

- it upgrades “risk around zero” into “risk around a hurdle,”
- it is still one-pass and exact,
- it is more aligned with real deployment requirements when a required return floor matters.

This is the cleanest next exact metric family after the current zero-threshold Sortino/Omega setup.

**Class:** exact / medium-risk / small register increase

---

### 2.2 Downside frequency and severity decomposition

Current downside deviation tells you aggregate downside magnitude, but not how that downside is distributed.

With a tiny additive state such as:

- `count_neg`
- `sum_neg_abs`
- optionally `count_pos`
- optionally `sum_pos_abs`

you get exact new scalars such as:

\[
p_- = \frac{count_{neg}}{n}
\]
\[
\bar{\ell} = \frac{sum_{neg\_abs}}{\max(count_{neg},1)}
\]
\[
LossPressure = p_- \cdot \bar{\ell}
\]

This lets you distinguish between:

- many small losses,
- few severe losses,
- symmetric noise,
- high-Omega candidates that still lose too often.

That information is often more product-useful than another raw volatility statistic.

**Class:** exact / medium-risk / very small state increase

---

### 2.3 Side-conditional long/short state

The loop already has signed position `pos` in `double_crossover.cu:3506` and uses it in the step-return update in `double_crossover.cu:3586`.
So a very natural additive extension is to accumulate separate long-side and short-side summaries, for example:

- `μ_long`, `μ_short`
- `e_long`, `e_short`
- `d_long`, `d_short`
- possibly `pnl_long`, `pnl_short`

This supports scores like

\[
S_{side-min} = \min(S_{long}, S_{short})
\]

or a softer balance penalty such as

\[
Penalty_{side} = \left| \frac{\mu_{long}}{e_{long}} - \frac{\mu_{short}}{e_{short}} \right|
\]

This is especially useful when signed exposure and two-sided deployment are important, which the current contract already suggests via `net_exposure`.

**Class:** exact / medium-risk / moderate register increase

---

## 3) Metric-surface geometry: robustness without another bar walk

The earlier documents already argued that perturb robustness should be treated as a **surface** problem, not a rerun problem. That idea can be pushed further mathematically.

### 3.1 Curvature, basin, and plateau metrics

Once a scalar score `S(i,j)` exists on the `(fast_len, slow_len)` grid, the optimizer already has a geometric object.

Define local finite differences:

\[
g_f = S(i+1,j) - S(i-1,j)
\]
\[
g_s = S(i,j+1) - S(i,j-1)
\]
\[
h_{ff} = S(i+1,j) - 2S(i,j) + S(i-1,j)
\]
\[
h_{ss} = S(i,j+1) - 2S(i,j) + S(i,j-1)
\]

Useful exact neighborhood scalars include:

\[
Plateau_q(i,j) = Q_q\{S(u,v) : (u,v) \in N_r(i,j)\}
\]
\[
BasinMin(i,j) = \min\{S(u,v) : (u,v) \in N_r(i,j)\}
\]
\[
Roughness(i,j) = S(i,j) - median\{S(u,v) : (u,v) \in N_r(i,j)\}
\]
\[
Curv(i,j) = |h_{ff}| + |h_{ss}|
\]

A perturb-aware scalar can then be

\[
S_{rob}(i,j) = \alpha S(i,j) + \beta Plateau_{0.25}(i,j) - \gamma Curv(i,j) - \delta Roughness(i,j)
\]

This is mathematically serious because it penalizes narrow spikes and rewards broad basins.
It also uses the already-computed parameter surface, not a second historical run.

**Class:** exact over the matrix / medium implementation / zero bar-loop cost

---

### 3.2 Multi-score surface agreement

A particularly interesting extension is to compare local robustness across several score heads at once.

For example, let

- `S1 = Sharpe`
- `S2 = log(Omega)`
- `S3 = active Sortino`
- `S4 = chronicity-penalized composite`

Then define a local agreement score such as

\[
A(i,j) = \min_{h \in \{1,\dots,H\}} Plateau_q^{(h)}(i,j)
\]

or

\[
A(i,j) = -\max_h Curv^{(h)}(i,j)
\]

This is the surface analogue of the adversarial weight-envelope idea.
It prefers candidates that are not only locally stable, but locally stable across several views of “good.”

That is often a much better anti-overfit criterion than any single scalar objective alone.

**Class:** exact over the matrix / medium-risk / best on shortlist or tile-local neighborhoods

---

## 4) Approximate and optional ideas that are mathematically interesting but not first-wave

### 4.1 Streaming tail sketches for approximate VaR / CVaR

If you want tail-focused risk without replay, one optional path is a tiny per-candidate streaming quantile sketch such as a P² estimator or a very small fixed-marker sketch.

That can approximate lower-tail quantiles and CVaR-like quantities without storing the full return path.

This is mathematically useful, but it is no longer in the “obviously cheap” class. The register footprint and branchiness make it a second-wave or experimental feature.

**Class:** approximate / optional / medium-high performance risk

---

### 4.2 Epsilon-dominance or hypervolume-style shortlist metrics

Exact full Pareto extraction over the whole grid remains the wrong first move.
But on a tiny union shortlist, approximate multi-objective richness measures become practical:

- epsilon-dominance bucket counts
- 2D or 3D hypervolume proxies
- dominance counts restricted to the shortlist

These are valuable for explanation and interactive reranking, but they should remain a shortlist-stage feature, not a first-wave device-global optimization semantic.

**Class:** approximate or hybrid / optional

---

## 5) Formal answers to the original brief items that were still partly open

### 5.1 Best composite score families over the current metric set

The earlier write-up already recommended normalized additive utility and augmented Tchebycheff.
The strongest remaining additions are:

#### Family A: adversarial weight-envelope score

Use transformed metrics `z(m)` and a small set of plausible weight vectors `Λ`:

\[
S_{env}(m) = \min_{w \in \Lambda} \langle w, z(m) \rangle
\]

This is the best exact family when the real problem is uncertainty about the right weighting, not just uncertainty about the candidate.

#### Family B: active-quality plus path-shape score

A very practical exact family is

\[
S_{act-path}
=
\alpha\,Sortino_{act}
+
\beta\,\log(\Omega+\varepsilon)
-
\gamma\,\frac{Ulcer}{\max(MaxDD,\varepsilon)}
-
\delta\,\frac{|net\_exposure|}{\max(exposure,\varepsilon)}
\]

This uses only the current row plus the active-time derivations above.

#### Family C: hurdle-slack / robust-margin score

Let `m_i` be normalized constraint margins. Then use either

\[
S_{min} = \min_i m_i
\]

or a soft-min/log-slack variant.

This remains the best family when the user really wants “balanced candidates with no weak dimension.”

### 5.2 Best exact multi-objective top-k from one metric matrix

The best exact design remains:

1. compute several scalar heads from the same current metric matrix,
2. do exact top-k per head,
3. take the union,
4. deduplicate,
5. do any nonlocal or exploratory rerank only on that tiny union.

Why this is still the best answer:

- full exact Pareto over the entire grid is semantically elegant but operationally unnecessary,
- each extra head only scans roughly `2.33 MiB` on the 10-row current matrix,
- the bar walk remains the dominant cost.

The main addition here is that the heads should include not only raw metrics, but also:

- active-time-normalized heads,
- chronicity-penalized heads,
- envelope heads,
- optionally fee-stressed final-PnL heads.

### 5.3 Hard device-side constraints vs soft penalties

A concise rule is:

- **Hard gate** a condition when it is non-compensatory, reliably estimated, and failure should make a candidate categorically uninteresting.
- **Soft-penalize** a condition when it is a trade-off, noisy, or preference-dependent.
- **Diagnostic only** when it is expensive, nonlocal, or mainly explanatory.

Under that rule, the best default split is:

**Hard gates**
- `min_trades`
- `max_drawdown`
- `min_exposure`
- `max_abs_net_exposure`
- optionally `min_omega` once direct support is live

**Soft penalties**
- drawdown chronicity `Ulcer / MaxDD`
- active-time penalties or bonuses
- fee-stressed final-PnL gap
- downside-share / asymmetry penalties
- long/short imbalance

**Diagnostic / shortlist-stage**
- matrix-neighborhood robustness
- epsilon-dominance explanation
- richer validation-like robustness summaries

### 5.4 Highest-value extra state under a strict no-regression target

If the target is “do more useful work with the lowest chance of moving the ~77 ms envelope,” the best portfolio is:

1. **row-only algebra first**
   - active-time metrics
   - chronicity
   - asymmetry proxies
   - envelope composites
   - fee-stress final-PnL transform

2. **then the smallest new sufficient statistics**
   - target partial moments `LPM/UPM`
   - `count_neg`, `sum_neg_abs`
   - PSR moments if not already added

3. **then nonlocal matrix geometry**
   - plateau / basin / curvature on the metric matrix or a shortlist

This is the best risk-adjusted portfolio because it postpones new register pressure until the row-only and matrix-only gains are exhausted.

### 5.5 Where shortlist formation should happen

The clean mathematical split is:

- if the semantic depends only on one candidate’s row and fixed thresholds: **device**
- if the semantic depends on several scalar score heads from the same row: **device exact top-k + union**
- if the semantic depends on neighborhood geometry or global rank interactions: **shortlist-stage hybrid**
- if the semantic depends on alternate datasets or resampling: **downstream validation**

That split is now even stronger because active-time metrics, chronicity, envelope scoring, and fee-stress transforms all live in the first two categories.

### 5.6 Should the 10-row output stay fixed?

Yes, by default.

The current 10-row output looks close to the right default frontier for downstream flexibility.
It already supports a large derived family at host decode time, especially through `gpu_backtest_kernel.rs:764-831`.

The right widening rule is:

> only widen in an additive opt-in way when the new quantity is **not reconstructible** from the current row, pair parameters, and known global run constants, and when it supports at least one of:
>
> - a new user-visible objective,
> - a new exact hard device-side gate,
> - a new high-value downstream diagnostic.

Under that rule:

- active-time metrics do **not** justify default widening,
- chronicity does **not** justify default widening,
- fee-stressed final-PnL does **not** justify default widening if cost-event semantics already match,
- target partial moments **might** justify opt-in widening if users want host-visible `Omega_tau` / `Kappa_tau`,
- long/short decomposition **might** justify opt-in widening if it becomes a user-facing filter or reporting feature.

---

## 6) Ranked list of the best remaining math-first additions

### Tier 1: exact and very cheap

1. **Active-time normalization family**
   - biggest remaining exact row-algebra win
   - especially `active_mean`, `active Sortino`, and exposure-neutral composites

2. **Drawdown chronicity (`Ulcer / MaxDD`)**
   - pure row algebra
   - strong path-shape information

3. **Adversarial weight-envelope composite**
   - mathematically stronger than one arbitrary weighted sum
   - still cheap because it is reduction-only

4. **Exact commission-stress final-PnL transform**
   - highly useful validation bridge
   - exact under the current scalar commission model

### Tier 2: exact and still attractive

5. **Fixed-target partial moments (`Omega_tau`, `Kappa_tau`)**
   - best next exact risk family to add inside the bar walk

6. **Downside frequency/severity decomposition**
   - tiny state, real extra information

7. **Effective-sample shrinkage / LCB scoring beyond raw metrics**
   - especially important when comparing very long-period candidates

### Tier 3: exact but more architectural than algebraic

8. **Matrix-neighborhood curvature / plateau / basin scoring**
   - zero bar-loop cost
   - very strong perturb-style robustness signal

9. **Long/short side-conditioned metrics**
   - highly valuable if two-sided deployment quality matters

### Tier 4: approximate / optional

10. **Streaming tail sketches for VaR / CVaR**
11. **Shortlist-stage epsilon-dominance / hypervolume proxies**

---

## 7) What was still unaddressed in the original prompt and is now addressed here

Relative to the original research brief and the two earlier markdown notes, the biggest still-open mathematical items were:

1. **Exact row-derived metrics that had not yet been fully formalized**
   - active-time normalization
   - drawdown chronicity
   - downside-share / asymmetry proxies

2. **A stronger family for composite scoring under preference uncertainty**
   - adversarial weight-envelope scoring

3. **The best next exact carried risk family after current zero-threshold Sortino/Omega**
   - target partial moments and hurdle-based Omega/Kappa

4. **A sharper rule for whether the current 10-row output should widen**
   - keep default fixed; widen only when the metric is not reconstructible and has real gate/objective/report value

5. **A sharper distinction between bar-loop work, row algebra, and matrix geometry**
   - the remaining high-value work is mostly in row algebra and surface geometry before more live kernel state is justified

That is the main new content of this addendum.

---

## Bottom line

Yes: there is still meaningful mathematical headroom in the single optimization pass.

The highest-value remaining space is:

- **exact row algebra on the current 10-row output**,
- **a few more online sufficient statistics with very high leverage**, and
- **robustness geometry on the already-computed metric matrix**.

If the goal is “more useful work with minimal additive cost,” the next strongest moves are:

1. active-time-normalized score families,
2. drawdown chronicity and related path-shape penalties,
3. adversarial weight-envelope composites,
4. exact fee-stressed final-PnL transforms,
5. target partial moments for hurdle-based Omega/Kappa,
6. local surface-geometry robustness.

Those are the main mathematically serious opportunities still left after the earlier two notes.
