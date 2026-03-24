# Double MA single-pass optimization: validation pull-forward opportunities

## Current state used for this addendum

This addendum is based on the uploaded files plus the concrete validation-stage inventory in your note.

Directly visible in the uploaded repo state:

- The GPU backend defines a 10-field product-reduced row and makes it the default for the Sharpe/Volatility product family via `kernel_product_reduced_rows_enabled_for_objective(...)` and `search_metrics_count_for_optimization(...)` in `gpu_backtest_kernel.rs:136-148`, `gpu_backtest_kernel.rs:3346-3357`, and `gpu_backtest_kernel.rs:3583-3614`.
- The current reduced search/product row is still keyed around `pnl`, `trades`, `max_dd`, `mean`, `std`, `exposure`, `net_exposure`, `downside_deviation`, `ulcer_index`, and `omega`, with derived decode of `sharpe`, `sortino`, `calmar`, and `martin_ratio` in `gpu_backtest_kernel.rs:764-831`.
- Device-side row reduction and top-k already exist in `gpu_backtest_kernel.rs:4220-4398` and `double_crossover.cu:4997-5279`.
- Current on-device hard constraints are still narrow, and the fallback is `host_filter_mode`, which forces tile copies and CPU-side decode/filtering in `gpu_backtest_kernel.rs:6733-6752` and `gpu_backtest_kernel.rs:7685-7728`.
- The main single-pass flex kernel already carries substantial running state in registers, including Welford moments, downside mass, ulcer mass, drawdown state, trade count, and exposure state in `double_crossover.cu:3434-3765`.
- `Sortino` and `Omega` are still blocked as GPU kernel objectives even though the backend can already request 10-field rows for them: `gpu_backtest_kernel.rs:3583-3614`, `gpu_backtest_kernel.rs:6388-6424`, `double_ma.rs:471-482`, and `double_ma.rs:9609-9612`.
- The uploaded Double MA validation implementation already contains finalist-only regime-slice robustness, GPU slice-sum helpers for SPA-like downstream analytics, and friction-stress summaries in `double_ma_validate.rs:599-637`, `double_ma_validate.rs:2071-2141`, `double_ma_validate.rs:2141-2195`, `double_ma_validate.rs:2307-2421`, and `double_ma_validate.rs:2470-2611`.

Not directly visible in uploaded files, but supplied by you as current downstream context:

- validation workflow toggles such as `walk_forward`, `reopt_walk_forward`, `spa`, `perturb`, `trade_bootstrap`, `cross_ticker`, and `cscv_pbo`
- hard gates such as `min_psr`, `min_dsr`, `min_cscv_oos_rank_frac`, `min_cscv_oos_metric`, and `max_pbo`
- probability metrics, robust score / robust Pareto marking, and validation verdict logic in `validation_pipeline.rs`

I did **not** directly inspect `validation_pipeline.rs` or `validationRequestBuilder.ts` here, so any statement about their exact internal formulas is an inference from your note rather than a visible repo fact.

---

## Bottom line

Yes: **some validation features can be pulled into the single optimization pass**, but only the ones that are compressible into low-dimensional online statistics of the already-walked return path, or the ones that can be computed from the already-produced metric matrix without another bar replay.

The clean dividing line is:

- **Good pull-forward candidates:** same-path online summaries, fixed-bucket summaries, checkpoint/slice summaries, higher moments, stress sensitivities, and matrix-neighborhood robustness.
- **Bad pull-forward candidates:** anything that requires alternate datasets, alternate parameter reruns, resampling, benchmark/bootstrap p-values, or re-optimizing inside windows.

So the answer is **yes, but selectively**.

The highest-value first-wave validation-aware additions are:

1. **PSR ingredients and PSR itself** from extra online moments.
2. **Perturb / local-stability proxies from the already-computed metric matrix**.
3. **Fixed-slice time robustness** from one-pass checkpoint snapshots.
4. **Low/mid/high volatility bucket robustness** using a precomputed dataset-wide volatility-state array.
5. **Friction elasticity / stressed final-PnL proxies** using cost-event counts.

The items that should **not** be pulled wholesale into the optimization pass are:

- full `reopt_walk_forward`
- trade bootstrap
- cross-ticker validation
- full CSCV/PBO
- full SPA bootstrap p-values

Those remain downstream validation work.

---

## The right mathematical test: online compressibility

A validation analytic is a strong single-pass candidate if it can be written as

```text
V(candidate) = F( sum_t phi(x_t, state_t, bucket_t) )
```

or, more generally,

```text
V(candidate) = F( A_1, A_2, ..., A_k )
```

where the `A_i` are a **small** set of online-updatable accumulators or checkpoint snapshots gathered during the same full-history bar walk.

This criterion immediately partitions the downstream validation families.

### Pull-forwardable exactly or nearly exactly

- confidence metrics based on return moments
- fixed-window / fixed-slice consistency metrics
- exogenous-bucket robustness metrics
- turnover / friction sensitivity metrics
- drawdown-duration / underwater diagnostics
- local parameter-surface robustness from the metric matrix

### Not pull-forwardable exactly without effectively reintroducing another engine

- resampling methods
- alternate-dataset methods
- reoptimization-in-window methods
- large combinatorial split methods when you want the full exact statistic, not a proxy

That is why the best path is **not** “put validation into optimization.”
It is “pull forward the subset of validation that is online-compressible and high leverage.”

---

## Validation family by validation family

| Validation family | Can single-pass optimization absorb it? | Best additive form |
|---|---:|---|
| `walk_forward` | Partially | exact fixed-slice / checkpoint OOS proxies |
| `reopt_walk_forward` | No, not exactly | none; keep downstream |
| `spa` | Partially | exact slice sums / beat-rate proxies, not bootstrap p-values |
| `perturb` | Yes, in a different form | matrix-neighborhood robustness over the parameter surface |
| `trade_bootstrap` | No, not exactly | PSR / LCB / jackknife-style surrogates |
| `cross_ticker` | No | keep downstream |
| `cscv_pbo` | Partially | coarse slice cross-validation proxies, not full PBO |
| `min_psr` | Yes | exact PSR gate after adding skew/kurtosis moments |
| `min_dsr` | Partially | host-side DSR from in-pass ingredients, likely not first-wave device gate |
| regime-slice robustness | Partially | exact volatility-bucket robustness first; trend/range split only as a later approximation |
| friction-stress summaries | Partially | exact cost elasticity / stressed final-PnL; full path-risk stress remains downstream |

---

## Ranked validation-aware opportunities

### 1) Add PSR ingredients, PSR, and a Sharpe lower confidence bound

**Class:** exact / low-risk  
**Validation mapping:** `min_psr`, partial bridge to `min_dsr`  
**Why it is first:** it gives a real validation-grade confidence signal with very little extra state.

The current kernel already maintains `n`, `mean`, and `m2`-style variance state in the bar loop (`double_crossover.cu:3434-3765`).
The natural extension is to also maintain online third and fourth centered moments, `m3` and `m4`, using a standard Pébay/Welford update.

That gives exact in-pass estimates of:

- skewness
- kurtosis
- PSR
- one-sided lower confidence bounds for Sharpe

A standard PSR form is:

```text
PSR(SR*) = Φ( ((SR_hat - SR*) * sqrt(n - 1))
              / sqrt(1 - γ3 * SR_hat + ((γ4 - 1) / 4) * SR_hat^2) )
```

where:

- `SR_hat = μ / σ`
- `γ3` = sample skewness
- `γ4` = sample kurtosis
- `SR*` = benchmark Sharpe threshold

A particularly useful derived scalar is the lower confidence bound:

```text
LCB_α(SR) = SR_hat - z_(1-α) * SE_SR
```

with

```text
SE_SR^2 ≈ (1 - γ3 * SR_hat + ((γ4 - 1) / 4) * SR_hat^2) / (n - 1)
```

That is often even easier to tune than a probability threshold.

**Why this is so strong**

- It directly aligns with your downstream `min_psr` gate.
- It is not a toy proxy; it is a real statistical confidence measure.
- It needs no replay, no slice matrix, no alternate scenario, and no row widening if used only for device score / gate logic.

**Performance profile**

- extra state: `m3`, `m4` and perhaps a couple of temporaries
- extra memory traffic: none if not exported
- occupancy risk: low
- host readback: none if only used for ranking/gating

**Recommendation**

- first deploy as an **opt-in score / gate**
- promote to a hard device-side gate only after parity testing against the downstream validation formula

**Important caveat**

I cannot see the exact `validation_pipeline.rs` PSR/DSR formula from the uploaded files, so formula-level parity with downstream validation is an inference target, not a visible repo fact.

---

### 2) Add parameter-surface perturb robustness from the already-computed metric matrix

**Class:** exact over the metric matrix / low bar-loop cost / medium implementation  
**Validation mapping:** `perturb` surrogate  
**Why it is second:** it captures a large part of perturb-style validation without any new bar walk.

The key observation is that the optimizer already evaluates a dense MA-parameter surface.
A perturb test is asking, in effect:

```text
If I nudge (fast_len, slow_len), does the strategy collapse?
```

But those nudged neighbors are already present in the optimization matrix.
So instead of rerunning perturbed candidates downstream, the optimizer can measure local robustness directly from the same matrix.

Let `S(i,j)` be the chosen scalar score at grid point `(i,j)` and `N_r(i,j)` a small neighborhood such as a 3x3 or radius-2 stencil.
Useful local robustness summaries are:

```text
plateau_q(i,j) = Q_q { S(u,v) : (u,v) in N_r(i,j) }
basin_min(i,j) = min { S(u,v) : (u,v) in N_r(i,j) }
roughness(i,j) = S(i,j) - median { S(u,v) : (u,v) in N_r(i,j) }
plateau_count(i,j) = count { S(u,v) >= S(i,j) - δ }
```

A perturb-aware score can then be:

```text
S_pert(i,j) = α * S(i,j) + (1 - α) * plateau_q(i,j) - λ * roughness(i,j)
```

or, in more conservative form,

```text
S_rob(i,j) = basin_min(i,j)
```

This is mathematically very attractive because it attacks the real failure mode of overfit grid spikes: narrow, brittle peaks.

**Why this is especially good here**

- it uses the same already-computed optimization surface
- it adds no second historical replay
- it is often more relevant to MA-length robustness than generic random perturbation

**Best placement**

- exact device-tile stencil if you are willing to add tile-neighborhood logic
- otherwise host/device-hybrid on a union shortlist

**Performance profile**

- extra bar-loop cost: none
- extra memory traffic: reads of local neighbor scores from the metric matrix
- occupancy risk: none in the backtest loop itself
- host readback: optional; depends on whether you do the stencil on device or on a shortlist

**Implementation caution**

This is not free from a systems perspective because the current path uses compact valid-pair indexing and tile-local reduction machinery (`gpu_backtest_kernel.rs:4220-4398`, `gpu_backtest_kernel.rs:6733-6752`, `double_crossover.cu:4997-5279`).
So the math is easy; the main work is the indexing and placement.

---

### 3) Add exact fixed-slice time robustness via checkpoint snapshots, not a replay

**Class:** exact / medium-risk  
**Validation mapping:** `walk_forward`, `spa`, and CSCV-like OOS consistency proxies  
**Why it is third:** it is the best way to get real out-of-time information into the optimizer without rerunning ranges.

The validation code already shows two important facts:

1. downstream validation cares about slice/window behavior
2. GPU validation currently obtains slice stats by **multi-range reruns** for finalists (`double_ma_validate.rs:599-637`)

A better additive optimization-stage design is to collect **checkpoint snapshots** during the main full-history pass.

Let checkpoints be `τ_1 < τ_2 < ... < τ_K`.
At each checkpoint, save cumulative additive statistics such as:

```text
A(τ) = (
    n,
    mean, m2,
    sum_neg_sq,
    sum_pos_r, sum_neg_abs_r,
    trades,
    log_eq,
    exposure_abs_sum,
    exposure_signed_sum
)
```

Then segment aggregates between checkpoints are recovered by differencing cumulative snapshots.
For example, if you instead materialize cumulative raw sums `s1`, `s2`, `sneg2`, etc., then for segment `k`:

```text
Δs1_k = s1(τ_k) - s1(τ_{k-1})
Δs2_k = s2(τ_k) - s2(τ_{k-1})
Δn_k  = n(τ_k)  - n(τ_{k-1})
```

and

```text
μ_k = Δs1_k / Δn_k
σ_k^2 = (Δs2_k - Δs1_k^2 / Δn_k) / (Δn_k - 1)
Sortino_k = μ_k / sqrt(Δsneg2_k / Δn_k)
Omega_k = Δspos_k / max(Δsnegabs_k, ε)
PnL_k = exp(Δlog_eq_k) - 1
```

From those exact slice metrics, you can build high-value robustness scalars:

```text
worst_slice_score = min_k S_k
positive_slice_frac = (1/K) * Σ 1{S_k >= 0}
late_early_gap = S_last - S_first
jackknife_min = min_k S_{-k}
```

where `S_{-k}` is the score recomputed after removing slice `k`.

This is a very strong bridge to validation because it gives:

- OOS consistency information
- single-bad-segment sensitivity
- slice hit-rate vs benchmark
- SPA-lite and CSCV-lite inputs

without another historical bar walk.

**Why checkpoint snapshots are better than duplicated per-slice accumulators**

For the first-wave version, checkpoint snapshots reduce bar-loop overhead: most bars only update the normal cumulative stats, and only boundary bars snapshot state.
That is much cheaper than keeping `K` fully duplicated live accumulators per bar.

**Best first variants**

- **half split:** lowest risk, easiest
- **quartiles:** best balance of information and cost
- **leave-one-slice-out jackknife on quartiles:** very high value once quartiles exist

**What remains impossible from snapshots alone**

Max drawdown and ulcer over arbitrary noncontiguous unions are not exactly composable from raw additive moments alone.
So the first-wave slice/jackknife score family should focus on metrics that are additive-composable:

- pnl / log equity
- mean / variance / Sharpe
- downside deviation / Sortino
- omega
- trades
- exposure

---

### 4) Pull forward the low/mid/high volatility part of regime-slice robustness

**Class:** exact / medium-risk  
**Validation mapping:** regime-slice robustness  
**Why it is fourth:** unlike trend/range splitting, the validation code’s volatility buckets are dataset-wide, not candidate-specific.

This is a key point from the uploaded validation file.
`compute_regime_slice_robustness_stats(...)` in `double_ma_validate.rs:2307-2421` uses five buckets:

- low volatility
- mid volatility
- high volatility
- trend
- range

But the first three come from `rolling_volatility_tertiles(series.lr, ...)` in `double_ma_validate.rs:2141-2195`, and that depends only on the dataset return series, not on the candidate.

That means you can precompute a device array:

```text
vol_state[t] ∈ {low, mid, high}
```

once per dataset and then maintain three bucketed accumulator sets during the same optimization pass.

For each bucket `b`, maintain additive stats such as:

```text
n_b, s1_b, s2_b, sneg2_b, trades_b, logeq_b, exposure_b
```

Then derive:

```text
Sharpe_b, Sortino_b, Omega_b, PnL_b
worst_bucket_sharpe = min_b Sharpe_b
stable_bucket_count = Σ 1{PnL_b >= 0 and Sharpe_b >= 0}
bucket_gap = max_b Sharpe_b - min_b Sharpe_b
```

This is a real validation-like robustness signal, not a vague proxy.

**Why this is better than trying to port the full current regime-slice package**

The validation implementation’s trend/range split uses a candidate-specific `spread_ratio` series and a candidate-specific median threshold (`double_ma_validate.rs:2071-2141` and `double_ma_validate.rs:2307-2421`).
That is much less attractive for first-wave in-pass work.

The volatility-bucket half is cleaner because:

- bucket labels are exogenous and dataset-wide
- there is no per-candidate threshold estimation problem
- the result is exact for the chosen bucket definition

**Performance profile**

- extra bar-loop state: three small bucketed accumulator families
- extra memory traffic: one bucket label read per bar
- occupancy risk: medium; this is why it ranks below PSR and matrix-neighborhood robustness

---

### 5) Add leave-one-slice-out jackknife robustness from the same slice snapshots

**Class:** exact / medium-risk  
**Validation mapping:** trade-bootstrap / CSCV-lite surrogate  
**Why it is fifth:** it gives a real robustness-to-one-bad-block signal while staying purely deterministic and additive.

Once fixed slices exist, a powerful derived family is:

```text
S_{-k} = score computed from all slices except k
```

Then define:

```text
jackknife_min = min_k S_{-k}
jackknife_mean = (1/K) * Σ_k S_{-k}
jackknife_drop = S_full - jackknife_min
jackknife_pass_frac = (1/K) * Σ_k 1{S_{-k} >= τ}
```

This is attractive because it answers a very validation-like question:

```text
Does the candidate still look good if any one temporal block is removed?
```

That is not full bootstrap, and it is not CSCV/PBO.
But it is a serious and useful robustness test that can be made exact from slice aggregates.

It is especially appealing for MA-parameter optimization because many fragile candidates are dominated by one lucky subperiod. `jackknife_min` exposes that quickly.

**Recommendation**

- do not make this the first feature
- add it as the first second-order derivative once checkpoint slices exist

---

### 6) Add friction elasticity and exact stressed final-PnL proxies

**Class:** exact / low-risk for final PnL, approximate for path-risk metrics  
**Validation mapping:** friction stress summaries  
**Why it is sixth:** the full downstream friction package is too heavy, but a meaningful subset is cheap.

The validation file’s friction-stress summary reranks finalists under stressed fees/spread/slippage by rerunning metrics (`double_ma_validate.rs:2470-2611`).
That full package does **not** belong in the default optimization pass.

However, a very useful exact subset does.

If the execution path does not itself depend on the friction level, and if the per-event cost enters through a log-cost factor `ℓ(c) = log(1 - c)`, then with `E` cost events:

```text
log_eq(c1) = log_eq(c0) + E * (ℓ(c1) - ℓ(c0))
```

So stressed final log equity and stressed final PnL are exactly recoverable from:

- base `log_eq`
- cost-event count `E`

This supports exact scalars like:

```text
stress_pnl_2x
stress_pnl_combined
friction_gap = pnl_base - pnl_stress
friction_slope ≈ -d(log_eq)/dc
```

These are not full friction-stress summaries, because:

- max drawdown under stress is not exactly recoverable from only total event count
- Sharpe / Sortino under stress are not exactly recoverable from only total event count
- scenario rank drift is not exactly recoverable without more detail

But the **final-PnL side** of stress is recoverable, and that alone is often enough to penalize high-turnover fragility earlier in optimization.

**Important implementation detail**

Use the same internal event counter that drives actual cost application, not an assumed interpretation of the exported `trades` field.
The current kernels visibly increment trade-like counters differently in different variants, so a dedicated internal `cost_event_count` is safer if exact stress-PnL is the goal.

**Best first use**

- soft penalty in a composite score
- secondary ranking key
- optional exact stress-PnL export for shortlisted candidates

---

### 7) Add entry-margin / spread-ratio fragility metrics

**Class:** exact / low-risk  
**Validation mapping:** indirect bridge to regime-slice trend/range logic, perturb sensitivity, and friction sensitivity  
**Why it is seventh:** it is new, cheap, and operationally useful.

The uploaded validation code computes a candidate-specific spread ratio series:

```text
spread_ratio[t] = |fast[t] - slow[t]| / close[t]
```

in `double_ma_validate.rs:2071-2141`, and then uses it downstream in regime-slice logic.

A very cheap optimization-stage derivative is to track entry-side signal margin:

```text
m_entry = |fast - slow| / max(|price|, ε)
```

only when a trade is initiated, or perhaps on all active bars.

Useful derived scalars:

```text
mean_entry_margin
min_entry_margin
harmonic_entry_margin
active_mean_margin
```

These are not cosmetic. They are meaningful because a crossover system with tiny average entry margin is typically more vulnerable to:

- minor parameter perturbations
- slippage / spread inflation
- signal timing noise
- regime-threshold ambiguity

This makes entry margin a very good soft-robustness penalty.

**Why this is attractive**

- `fast`, `slow`, and price-like data are already in hand in the bar loop
- the state needed is tiny
- it does not widen the default row unless you choose to export it

---

### 8) Add underwater duration diagnostics

**Class:** exact / low-risk  
**Validation mapping:** not a direct existing validation feature, but very high-value shortlist information  
**Why it is eighth:** it adds path-quality information that drawdown magnitude alone misses.

The current kernels already track peak and drawdown state (`double_crossover.cu:3434-3765`).
So it is very cheap to also track:

```text
underwater_len_t = 0 if equity_t == peak_t else underwater_len_{t-1} + 1
```

and derive:

```text
max_underwater_duration
underwater_fraction
mean_underwater_duration (optional)
```

This is not a validation-stage metric you explicitly listed, but it is exactly the kind of additional useful work that improves shortlist quality with minimal additive cost.

Many strategies with acceptable `max_dd` are still poor finalists because they spend too much time underwater.
This metric catches that cheaply.

---

### 9) Add host-side DSR from in-pass ingredients, but do not make it the first device gate

**Class:** approximate / optional  
**Validation mapping:** `min_dsr`  
**Why it is ninth:** it is valuable, but it needs more modeling choices than PSR.

Once PSR ingredients exist, a DSR-style statistic can be formed as:

```text
DSR = PSR(SR_0)
```

where `SR_0` is a multiple-testing-adjusted benchmark Sharpe.
The practical issue is that `SR_0` depends on how you model the number of effective trials.
For a dense MA surface, raw pair count overstates the true trial count because nearby candidates are strongly correlated.

A much better approximation is:

```text
N_eff ≈ N / (ℓ_fast * ℓ_slow)
```

where `ℓ_fast` and `ℓ_slow` are correlation-length estimates of the score surface along the two parameter axes.
Those lengths can be estimated from the already-computed metric matrix.

This is mathematically attractive, but it is also more assumption-heavy than PSR.
So my recommendation is:

- compute PSR exactly first
- compute DSR host-side later from the same in-pass ingredients
- use `N_eff`, not raw grid size, if you want a surface-aware deflation model

This is a strong optional extension, not the first deployment target.

---

### 10) Add SPA-lite benchmark slice beat-rate and worst-slice excess return

**Class:** exact / medium-risk  
**Validation mapping:** `spa` bridge  
**Why it is tenth:** it becomes cheap once slice snapshots already exist.

The validation code already computes benchmark slice returns from the dataset (`double_ma_validate.rs:2615-2636`) and candidate slice sums on GPU for finalists (`double_ma_validate.rs:599-637`).

If the optimization pass captures fixed-slice candidate sums, you can compute exact benchmark-relative summary scalars such as:

```text
beat_frac = (1/K) * Σ 1{candidate_slice_return_k > benchmark_slice_return_k}
worst_excess = min_k (candidate_slice_return_k - benchmark_slice_return_k)
mean_excess = (1/K) * Σ (candidate_slice_return_k - benchmark_slice_return_k)
```

These are not SPA bootstrap p-values.
But they are genuine outperformance-consistency summaries and a very reasonable optimization-stage proxy.

---

## New ideas that were not in the prior write-up but now look especially good

### A) Use checkpoint snapshots, not duplicated live per-slice state

This is the best implementation trick in this whole addendum.

If the goal is half-split, quartile, or slice-jackknife robustness, the cheapest first-wave implementation is usually:

- keep one live cumulative accumulator set
- snapshot it at a few fixed boundaries
- derive slice stats from differences later

rather than keeping `K` live bucket copies per bar.

That materially improves the cost model.

### B) Treat perturb validation as a **surface** problem, not a rerun problem

This is the most important conceptual shift.

For MA-grid optimization, perturb robustness is already latent in the search surface.
That makes local plateau / basin scoring one of the highest-value additions in the whole system.

### C) Use volatility buckets before trend/range buckets

The uploaded validation code makes this clear.
The low/mid/high-volatility half is much easier to move forward exactly than the candidate-specific trend/range half.

### D) Use a dedicated internal `cost_event_count` if you want exact friction-PnL transforms

That avoids semantic ambiguity around the existing `trades` export.
This keeps the new stress logic exact even if exported trade semantics remain untouched.

### E) Add a tiny optional aux buffer instead of widening the default 10-row contract

For many of these features, the right architecture is:

- keep the base 10-row product contract unchanged
- compute extra scalars only when the run opts in
- store them in a parallel aux buffer or use them only transiently for scoring/filtering

This respects backward compatibility while still letting the optimizer do more useful work.

---

## Formal analysis: which validation features are truly one-pass-compatible?

### 1) PSR is genuinely one-pass-compatible

PSR depends on a finite set of return moments.
That makes it fundamentally compatible with a single exact pass.

Mathematically, PSR is a function of:

```text
n, μ, σ, skewness, kurtosis
```

and all of those are online statistics.
So PSR belongs in optimization if validation-aware early filtering is a priority.

### 2) DSR is only partially one-pass-compatible

Per-candidate moment ingredients are one-pass-compatible.
The multiple-testing correction is not purely per-candidate; it depends on a model of the search family.

Therefore the clean architecture is:

- compute the per-candidate ingredients in the pass
- apply the deflation model later, typically on host or on a small shortlisted set

### 3) Walk-forward is compatible only after you replace “many rerun windows” with “few fixed one-pass slices”

A full walk-forward pipeline involves multiple IS/OOS windows and often repeated scoring.
That is not what you want in the default optimization pass.

But fixed-slice summaries and leave-one-slice-out summaries capture the same mathematical theme:

```text
generalization across time blocks
```

So the right pull-forward is not exact walk-forward replication; it is **exact fixed-slice robustness**.

### 4) Reoptimization walk-forward is not a good single-pass target

This method changes the candidate-selection process across windows.
That is algorithmically different from evaluating one candidate once.

Trying to force this into the optimization pass would effectively rebuild a validation engine inside the search loop.
That is the wrong trade.

### 5) Full bootstrap is not one-pass-compatible, but deterministic jackknife-style block omission is

Bootstrap requires resampling or explicit trade lists.
That is not compatible with the strict low-cost single-pass target.

Block jackknife, by contrast, only needs deterministic block aggregates.
That is why it is the right bridge.

### 6) Full SPA and CSCV/PBO are not first-wave one-pass targets, but their fixed-slice sufficient statistics are

For chosen fixed slices, the optimization pass can absolutely produce exact per-slice candidate summaries.
What it should not do in the first wave is carry the full resampling/combinatorial machinery.

### 7) Regime robustness is one-pass-compatible only for bucket definitions that are exogenous or cheaply computable online

- dataset-wide volatility state: yes
- market up/down state: yes
- externally supplied regime filter: yes
- candidate-specific median-threshold split over spread ratio: not first-wave friendly

This gives a clear ordering for regime-aware additions.

---

## What I would implement first

### Phase 1: very cheap, very high value

1. **PSR ingredients + PSR / LCB score**
2. **entry-margin fragility metric**
3. **underwater duration metrics**
4. **direct Sortino/Omega objective support** if you want validation-aware optimization to use the current metric family cleanly (`gpu_backtest_kernel.rs:6388-6424`, `double_ma.rs:9609-9612`)

This phase adds real shortlist quality with minimal cost and does not require changing the default row contract.

### Phase 2: the best validation bridge

5. **half-split checkpoint snapshots**
6. **quartile checkpoint snapshots**
7. **worst-slice / positive-slice-frac / jackknife_min**

This is the best path toward walk-forward / SPA / CSCV-lite behavior without a replay.

### Phase 3: regime-aware robustness

8. **low/mid/high vol bucket robustness**
9. **optional benchmark up/down capture buckets**

This gets you real validation-like robustness signals with exact same-pass computation.

### Phase 4: matrix-level perturb robustness

10. **local basin / plateau score over the metric matrix**
11. **perturb-aware union shortlist mode**

This is probably the highest product-value second-order improvement once the simpler statistics exist.

### Phase 5: optional host-side skepticism layer

12. **DSR from in-pass ingredients and a surface-aware `N_eff`**
13. **Pareto / lexicographic rerank over base score + PSR + slice robustness + perturb robustness**

---

## The shortlist architecture I now recommend

The best architecture after this review is:

### Device hard constraints

Only the constraints that are:

- exact
- cheap
- monotone
- clearly beneficial for readback avoidance

Examples after this addendum:

- `min_trades`
- `max_dd`
- `min_omega`
- `min_psr` (after parity validation)
- possibly `min_worst_half_score` for half-split mode

### Device exact score families

- existing single-metric score
- composite score over current metrics
- PSR / LCB score
- half-split / worst-slice score once enabled
- stressed-PnL penalty score

### Device or hybrid shortlist union

Take the union of top-k from several exact score families, for example:

- primary objective
- PSR / LCB
- worst-slice or jackknife score
- stressed-PnL score
- maybe basin/plateau perturb score

### Host rerank

Use host rerank only on the tiny union shortlist for:

- DSR
- weighted-rank aggregation
- optional Pareto / lexicographic validation-aware ordering

This is much better than trying to make one giant scalar do everything.

---

## What should stay out of the default 10-row contract

The current 10-row contract should stay fixed by default.

I do **not** recommend widening it for:

- per-slice arrays
- per-bucket arrays
- PSR moments
- neighborhood robustness fields
- friction scenario tables

Those should instead be added as:

- internal-only score/gate logic
- optional auxiliary buffers
- optional finalist/shortlist exports

That preserves backward compatibility and keeps the cost model clean.

---

## Final answer to the concrete question

Yes, a validation feature can absolutely be processed during the single backtest optimization pass.

The best ones are **not** the heavyweight downstream procedures themselves.
The best ones are the mathematically compressible cores of those procedures:

1. **PSR** from extra return moments
2. **fixed-slice OOS robustness** from checkpoint snapshots
3. **volatility-bucket robustness** from a precomputed dataset-wide bucket array
4. **friction elasticity / stressed final-PnL** from cost-event counts
5. **perturb robustness from local surface geometry** of the already-computed metric matrix

Those five additions are the strongest path to “more useful work during the same pass” under a strict no-regression philosophy.

The single most important practical recommendation is:

> Pull forward **PSR first**, then **checkpoint-slice robustness**, and treat **perturb validation as a surface-neighborhood problem**, not as a rerun problem.

