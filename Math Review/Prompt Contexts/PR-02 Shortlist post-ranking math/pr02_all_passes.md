
# PR-02 Shortlist post-ranking math — all three passes

This document bundles the first-pass, second-pass, and third-pass answers for `PR-02`.

The third pass is the strongest surviving implementation-ready result and supersedes any conflicting broader claims from the first pass.

---


# First-pass answer

The first pass proposed the broadest plausible repo-fit thesis:

- exact replay-free reranking looked plausible for effective-score evaluation, union reranking, and confidence reranking;
- slice robustness, omission outputs, benchmark-relative slice summaries, and neighborhood robustness looked exact only if the relevant slice-local or neighborhood-local state had already been retained;
- score-first was treated as mostly out of scope because the current blockers exclude score mode, extra shortlist heads, union, confidence, and slice surfaces.

The first-pass formalization modeled the post-score pipeline as a factorization into seed shortlist, union rerank, confidence rerank, and slice rerank, with neighborhood robustness attached afterward. It proposed stagewise sufficient statistics:

- union: shortlisted objective score matrix plus tie-breaks;
- confidence: Sharpe, observation count, peer distribution, effective score, and drawdown;
- slice ordering: a compact tuple involving worst slice, omission summaries, positive-slice fraction, mean slice score, full-score tie-break, and drawdown;
- benchmark-relative slice summaries: the vector of slice excess returns;
- neighborhood robustness: the local response surface around the winner.

The first pass also proposed a family of impossibility results:

- full-period arithmetic aggregates do not identify slice-local robustness outputs;
- winner-only retained summaries do not identify neighborhood robustness;
- DSR is shortlist-conditional rather than candidate-local.

The main positive conclusion of the first pass was that union and confidence were algebraically identifiable from already retained shortlist summaries, while slice and neighborhood outputs were not.

## Main first-pass counterexample family

A simple separating family used two candidates with the same full-period arithmetic aggregates but different time allocations across slices, such as:

- \(A=(a,a,-a,-a)\),
- \(B=(a,-a,a,-a)\).

These paths share many full-period aggregate statistics but differ under half-slice decomposition, proving that arithmetic aggregates alone cannot identify slice robustness.

The first pass also noted that arithmetic-return slice summaries differ from exact compounded pathwise PnL, so benchmark-relative exactness needed to be interpreted carefully.

## Main unresolved questions

The first pass left open:

- whether “exactness” meant ordering only or full diagnostic parity;
- whether benchmark-relative slices were intended as arithmetic proxy objects or exact pathwise PnL objects;
- whether DSR’s peer universe was intentionally shortlist-derived;
- and whether slice boundaries should remain shortlist-global via the common-start rule.


---


# Second-pass answer

The second pass attacked the first pass and narrowed the claim sharply.

## Main corrections introduced in the second pass

1. **No candidate-local universal summary**
   - DSR depends on shortlist-level peer mean, peer std, and peer count.
   - Pareto, epsilon-front, and weighted-rank union modes depend on shortlist-global structure.
   - Therefore exactness is shortlist-level, not candidate-local.

2. **Metric-contract assumptions had to be updated**
   - The old 10-row reduced picture was no longer the normal exact contract.
   - The current code uses 17-row exact metrics by default and 22-row metrics for the trade-extended exact contract.

3. **Confidence tie-break semantics were subtler than first pass allowed**
   - `EffectiveSampleLcb10` has a pair-aware score path in some contexts.
   - But the actual confidence comparator ties break using plain `effective_score(...)`, not the pair-aware path.
   - So any theorem that assumed a pair-aware tie-break in confidence reranking was proving the wrong object.

4. **Slice boundaries are shortlist-global**
   - `common_start` is the maximum valid start across shortlisted rows.
   - Adding or removing one late-warmup candidate can move everyone’s slice boundaries.
   - That kills candidate-local slice summaries as universally reusable exact objects.

5. **Diagnostic parity is much stronger than order parity**
   - Current slice diagnostics expose per-slice labels, ranges, scores, metrics, omission summaries, benchmark-relative summaries, and neighborhood outputs.
   - A compact ranking tuple may preserve order, but not those diagnostics.

6. **Neighborhood robustness is not a function of the winner row**
   - It is attached only after a separate local-stability follow-up around the final winner.

## Strongest surviving second-pass result

After adversarial attack, the second pass narrowed the strongest repo-fit claim to:

- exact replay-free reranking is currently defensible only for **union** and **confidence**;
- even there, the sufficient statistics are **shortlist-level**, not candidate-local;
- exact slice, omission, benchmark-relative slice, and neighborhood outputs still require replay or stronger retained slice-local / neighborhood-local state.

## Main adversarial constructions

The second pass used several edge cases:

- a candidate with fixed Sharpe/obs but different DSR under different peer shortlists;
- a fixed candidate whose slice diagnostics change when a late-warmup peer shifts `common_start`;
- arithmetic-proxy slice returns that match while compounded pathwise PnL differs;
- `EffectiveSampleLcb10` tie situations where the actual comparator is not pair-aware.

## Revised conclusion

The second pass concluded that there is no single minimal retained summary for all PR-02 outputs. The exact frontier is narrower and more conditional than the first pass suggested.


---


# Third-pass answer

According to the current repo files from April 18, 2026, the strongest implementation-ready result for `PR-02` is route-aware and stage-specific:

> There is **no single universal retained summary** that exactly reproduces the whole shortlist post-ranking subsystem under the current code surface. Exact replay-free post-ranking is currently provable only **stagewise**. Exact replay-free reranking is available for **union** and **confidence** from **shortlist-level retained summaries** that preserve the actual current comparators. Exact **slice robustness**, **omission summaries**, **benchmark-relative slice summaries**, and **neighborhood robustness** require the final shortlist partition plus slice-local or neighborhood-local retained state; otherwise replay is necessary.

A second third-pass correction is that the current code does **not** implement one universal composition
`seed -> union -> confidence -> slice -> neighborhood`. The exact GPU route and the dynamic exact-compatible route compose the post-score stages differently, so any exactness theorem must be indexed by route.

## 1. Final formalization

Let a request be

\[
R = (\rho,\ \theta,\ \xi,\ U,\ Q,\ H,\ B,\ N),
\]

where:

- \(\rho\) is the backend route, restricted here to the exact-compatible PR-02 routes:
  - \(\rho = K\): classic exact `GpuKernel` host-finalize path;
  - \(\rho = D\): exact-compatible `DynamicDoubleMa` GPU path.
- \(\theta\) is the finite candidate grid.
- \(\xi\) is the immutable evaluation context:
  `first_valid`, `regime_first_valid`, `eval_start`, `eval_end`, `total_bars`, `trade_on_next_bar`, objective, and score mode.
- \(U\) is an optional union mode.
- \(Q\) is an optional confidence mode (`PSR` or `DSR`).
- \(H\) is an optional slice mode (`Halves` or `Quartiles`).
- \(B\) is an optional slice benchmark.
- \(N\) is an optional neighborhood configuration \((radius, step)\).

For each candidate \(c\), let \(X(c)\) denote its full path-dependent evidence. The exact PR-02 object is a route-indexed output

\[
\mathcal O_\rho(R;X)=\big(\Pi_\rho(R;X),\ D_\rho(R;X)\big),
\]

where \(\Pi_\rho\) is the accepted shortlist order and \(D_\rho\) is the attached diagnostics.

The exactness target is:

\[
\widehat{\mathcal O}_\rho(R;S)=\mathcal O_\rho(R;X),
\]

where \(S\) is retained post-score state. The problem is to identify the smallest admissible \(S\) for exactness, or to prove that no such summary exists under the current constraints.

### Route-aware stage semantics

The stage semantics are part of the mathematical object.

#### Route \(K\): exact `GpuKernel` host-finalize path

Let \(B^{seed}\) be the materialized shortlist rows if `extra_shortlist_objectives` is active; otherwise let the current `result.top` list be the seed shortlist.

Under the current code:

- union rerank runs on `materialized_shortlist_rows` and **does overwrite** that materialized shortlist;
- confidence rerank runs on the current materialized shortlist but **does not overwrite** it;
- slice rerank uses the materialized shortlist when it exists, otherwise `result.top`;
- neighborhood robustness is attached afterward from a separate local-stability follow-up around the final winner.

So if extra shortlist heads are active, slice sees the union output but not the confidence output. If extra shortlist heads are absent, slice sees `result.top`, which may already reflect confidence.

#### Route \(D\): exact-compatible dynamic GPU path

Let \(B^{seed}\) again be the exact extra-shortlist seed if extra heads are active.

Under the current code:

- union rerank runs on `materialized_shortlist_rows` but does **not** write back to it;
- confidence rerank uses `materialized_shortlist_rows` when it exists, else `result.top`;
- slice rerank uses `materialized_shortlist_rows` when it exists, else `result.top`.

Therefore, when extra shortlist heads are present, union, confidence, and slice all read the same seed shortlist rather than feeding one another. The accepted order is whatever the **last enabled stage** writes into `result.top`. This kills any route-independent composition theorem.

## 2. Final theorem and lemma list

### Theorem T1: No universal route-independent stage composition theorem exists

Under the current code, there is no single formula of the form

\[
\Pi = H \circ Q \circ U \circ B^{seed}
\]

that is valid for all exact-compatible PR-02 routes. The classic exact GPU route and the dynamic exact-compatible route compose the stages differently, and the difference is behaviorally material.

### Theorem T2: No universal candidate-local sufficient statistic exists for all current PR-02 outputs

There is no per-candidate summary \(S(c)\) that exactly determines all of:
union reranking, confidence reranking, slice reranking, omission summaries, benchmark-relative slice summaries, and neighborhood robustness.

The obstruction has three independent causes:

1. **shortlist-global dependence**: DSR and the nontrivial union modes depend on the whole shortlist;
2. **partition dependence**: slice ranges depend on the whole shortlisted set through the common start time;
3. **local-surface dependence**: neighborhood robustness depends on a later local-stability point cloud around the winner.

### Theorem T3: Exact union reranking is identifiable from shortlist-level retained summaries

Fix a shortlist \(C=\{c_1,\dots,c_n\}\) and union objectives \(o_0,\dots,o_p\). Define the shortlisted objective-score matrix

\[
A_{ij}=\mathrm{objective\_score}(M(c_i),o_j).
\]

A sufficient statistic for exact union reranking is:

- the full matrix \(A\),
- the union mode,
- the per-candidate `max_dd` tie-break vector,
- and any induced shortlist-global structures the mode recomputes from \(A\) (front ranks, dense ranks, or epsilon scales).

This is exact for:
- lexicographic union,
- Pareto union,
- epsilon-front union,
- weighted-rank union.

But it is **not** candidate-local: for Pareto, epsilon-front, and weighted-rank, the output depends on relationships among the shortlisted rows.

### Theorem T4: Exact confidence reranking is identifiable from shortlist-level retained summaries

Fix a shortlist \(C\). For each candidate \(c\), define

- \(s(c)=\mathrm{sharpe}(c)\),
- \(n(c)=\mathrm{obs}(c)\), where `obs` is the current `shortlist_candidate_track_record_bars(...)`,
- \(t(c)\) = the **actual** tie-break scalar used by the comparator,
- \(d(c)=\mathrm{maxdd}(c)\).

A sufficient statistic for exact confidence reranking is:

- candidate-local: \((s(c),n(c),t(c),d(c))\) for all \(c \in C\),
- shortlist-global: peer mean, peer std, and peer count over the finite Sharpe values in \(C\).

The current comparator is:

1. descending confidence score (PSR or DSR),
2. descending \(t(c)\),
3. ascending \(d(c)\).

For current code, the tie-break scalar is **plain**
`effective_score(metrics, objective, score_mode)`.
That matters because for `EffectiveSampleLcb10` the plain `effective_score` falls back to the objective score rather than the pair-aware effective-sample score.

### Theorem T5: Exact slice-order parity requires final-partition slice-local state

Let the active slice mode have \(k \in \{2,4\}\) slices. Let the final shortlist-global ranges be
\(\mathcal I=\{I_1,\dots,I_k\}\), where the start is the common start time induced by the shortlisted rows.

For each candidate \(c\), let the slice scores be \(s_i(c)\), and define the omission scores

\[
\omega_j(c)=\frac{\sum_{i=1}^k s_i(c)-s_j(c)}{k-1}.
\]

A sufficient statistic for exact **slice-order parity** is the current rerank tuple

\[
R_{slice}(c)=
\big(
\min_i s_i(c),\ 
\min_j \omega_j(c),\ 
\frac1k \sum_i \mathbf 1\{s_i(c)\ge 0\},\
-\big(\max_j\omega_j(c)-\min_j\omega_j(c)\big),\
\frac1k \sum_i s_i(c),\
t_{full}(c),\
-d(c)
\big),
\]

where \(t_{full}(c)\) is the full-row effective score tie-break scalar and \(d(c)=\mathrm{maxdd}(c)\).

However, this statistic is sufficient only for **order**. It is not sufficient for full slice-diagnostic parity.

### Theorem T6: Exact slice-summary parity requires stronger slice-local retained state

To meet the PR-02 verification hooks on slice-summary parity and omission-summary parity, the retained state must determine, for each shortlisted candidate:

- the final common-start slice ranges,
- the per-slice labels and bar ranges,
- the per-slice scores,
- the per-slice metrics rows,
- the omission summary tuple,
- and, if a benchmark is active, the benchmark-relative tuple.

Without state of that strength, exact parity of current slice diagnostics is not guaranteed.

### Theorem T7: Exact benchmark-relative slice parity is exact only for the repo’s arithmetic-return proxy semantics

The current benchmark-relative slice summary is defined on slice excess returns

\[
x_i(c)=\widehat a_i(c)-b_i,
\]

where \(b_i\) is the benchmark arithmetic slice return and
\(\widehat a_i(c)\) is the candidate arithmetic return proxy reconstructed from the slice metrics row via:

- `derived_mean(metrics) * obs` when volatility is nontrivial;
- equal-step inversion from slice `pnl` when volatility is near zero.

Therefore:
- exact retained-state parity is possible for the **current arithmetic proxy object**;
- exact retained-state parity is **not** a theorem about exact compounded pathwise slice PnL.

### Theorem T8: Neighborhood robustness is non-identifiable from shortlisted rows alone

The current neighborhood robustness summary is attached only after a separate call to
`compute_double_ma_local_stability_blocking_with_candles(...)` around the winner. Its summary depends on the whole local point cloud and includes plateau support, basin-floor stability, roughness, curvature, rank-correlation agreement, and explanation geometry.

Therefore no winner-only retained summary derived from the shortlisted rows alone can identify exact neighborhood robustness.

### Lemma L1: Omission is affine in slice scores

For fixed slice labels and fixed \(k\),

\[
\omega_j=\frac{\sum_i s_i-s_j}{k-1}.
\]

This makes omission summaries easy to approximate once slice-score errors are controlled.

### Lemma L2: Benchmark-relative omission is affine in slice excess returns

The benchmark-relative omission summary is the same affine map applied to the excess-return vector \(x\).

## 3. Proof sketch / proof plan

### T1: Route asymmetry
Read the exact GPU host-finalize path and the dynamic exact-compatible path side by side.
In the exact GPU path, union overwrites the materialized shortlist before later stages, but confidence does not write back to it; slice then consumes that materialized shortlist when it exists. In the dynamic path, union also fails to write back, so union, confidence, and slice can all reread the same seed shortlist. This is a direct code-surface proof.

### T2: No universal candidate-local statistic
Use three separations:
1. DSR changes when the peer shortlist changes while a candidate’s local metrics stay fixed.
2. Slice ranges move when a late-warmup peer enters the shortlist, even if a candidate’s path is unchanged.
3. Neighborhood robustness is computed from a later local point cloud rather than from the base winner row.

### T3: Union exactness
The union comparator is a pure function of the shortlisted objective-score matrix and the `max_dd` tie-break. Once the induced ranks/fronts/epsilon scales are available, no path replay remains.

### T4: Confidence exactness
The comparator is a pure function of the PSR/DSR scalar, the tie-break scalar, and `max_dd`.
PSR/DSR are functions of Sharpe, observation count, and peer statistics. Hence those values form a sufficient statistic.
The `EffectiveSampleLcb10` caveat follows from reading the actual tie-break scalar used by the comparator.

### T5–T6: Slice exactness
Once slice-local metrics rows on the final partition are retained, the current slice order and diagnostics are algebraic functions of those rows.
Without them, exactness fails because full-period arithmetic aggregates do not determine slice-local replay outputs and do not determine the shortlist-global partition.

### T7: Arithmetic proxy versus compounded path
Construct two different within-slice paths with the same arithmetic sum but different compounded PnL.
This proves that exact arithmetic-proxy parity is a weaker statement than exact pathwise PnL parity.

### T8: Neighborhood non-identifiability
Read the neighborhood summary fields and the local-stability follow-up. Because the summary depends on neighboring points and alternate score surfaces, a winner-only retained row is too small to identify it.

## 4. Sufficient statistics, or a proof they do not exist

## 4.1. What exactly exists

### Exact union rerank
Minimal implementation-ready sufficient statistic:

\[
S^{union} = \big(A,\ d,\ mode\big),
\]

where \(A\) is the shortlisted objective-score matrix and \(d\) is the `max_dd` vector.

For epsilon-front and weighted-rank, either retain \(A\) and recompute the induced scales/ranks, or retain those induced structures themselves.

### Exact confidence rerank
Minimal implementation-ready sufficient statistic:

\[
S^{conf}=
\left(
\{(s(c),n(c),t(c),d(c)) : c\in C\},
\mu_C,\sigma_C,|C|_{finite}
\right).
\]

This is enough for exact PSR/DSR confidence reranking under current code semantics.

### Exact slice-order parity only
Minimal sufficient statistic after slice replay already happened:

\[
S^{slice,order} = \{ R_{slice}(c) : c \in C\}.
\]

This is enough only for exact final slice order, not for diagnostic parity.

### Exact slice-diagnostic parity
A sufficient statistic is any state equivalent to:

- final slice ranges and labels,
- per-candidate per-slice metrics rows,
- per-candidate per-slice scores,
- omission summaries,
- benchmark-relative summaries when active.

### Exact neighborhood parity
A sufficient statistic is the local-stability result itself or a mathematically proved equivalent compression of it.
No smaller sufficient statistic is currently identified.

## 4.2. What provably does not exist

There is no:

- universal candidate-local summary for all PR-02 outputs;
- exact slice summary from full-period arithmetic aggregates alone;
- exact neighborhood summary from the winner row alone;
- exact DSR summary from candidate-local data alone.

## 5. Exactness conditions and equivalence classes

Two retained states are **union-equivalent** iff they induce the same union comparator outcome for every shortlisted pair. Concretely, it is enough that they induce the same:

- shortlisted objective score rows,
- front ranks / epsilon-front structure / weighted-rank penalties,
- and `max_dd` tie-breaks.

Two retained states are **confidence-equivalent** iff they induce the same:

- confidence score for every shortlisted candidate,
- tie-break scalar \(t(c)\),
- and `max_dd`.

Two retained states are **slice-order-equivalent** iff they induce the same \(R_{slice}(c)\) tuple for every shortlisted candidate on the same final partition.

Two retained states are **slice-diagnostic-equivalent** iff they induce the same:

- slice ranges and labels,
- slice scores,
- slice metrics rows,
- omission tuple,
- and benchmark-relative tuple when present.

Two retained states are **neighborhood-equivalent** only if they induce the same neighborhood summary fields. No smaller canonical equivalence class is currently proved.

## 6. Error, regret, or bias bounds for every non-exact claim

The third-pass result makes only three approximation claims, and each comes with an explicit contract.

### 6.1. Scalar comparator stability
If a scalar sorting key \(f(c)\) is approximated by \(\hat f(c)\) with

\[
\|f-\hat f\|_\infty \le \varepsilon,
\]

then every pairwise comparison with true gap \(>2\varepsilon\) is preserved.

For lexicographic comparators, this must hold at the first decisive component.

### 6.2. Slice-score approximation contract
If slice scores are approximated with

\[
\|\hat s - s\|_\infty \le \varepsilon,
\]

then:

- each omission score changes by at most \(\varepsilon\);
- mean slice score changes by at most \(\varepsilon\);
- omission spread changes by at most \(2\varepsilon\);
- positive-slice fraction is exact whenever every true slice score satisfies
  \(|s_i|>\varepsilon\).

This follows from the affine omission formula.

### 6.3. Benchmark-relative slice approximation contract
If slice excess returns are approximated with

\[
\|\hat x - x\|_\infty \le \varepsilon,
\]

then:

- mean excess-return error is at most \(\varepsilon\);
- best and worst excess-return errors are at most \(\varepsilon\);
- best and worst omission-excess errors are at most \(\varepsilon\);
- omission-excess spread error is at most \(2\varepsilon\);
- beat rate is exact whenever every true excess return satisfies
  \(|x_i - 10^{-12}| > \varepsilon\).

### 6.4. No general approximation bound for union fronts or weighted ranks
Under current Pareto, epsilon-front, and weighted-rank semantics, there is no useful universal rank-loss bound from candidate-local score perturbations alone. Infinitesimal perturbations can change front membership, dense ranks, or epsilon-front assignments when the shortlist is near a dominance or score-tie boundary.

Therefore any approximate union claim is admissible only under a **front/rank preservation condition**:
the approximation must preserve all induced front ranks, dense ranks, decisive lexicographic components, and final `max_dd` tie-breaks. Without that, the claim is only heuristic.

### 6.5. No approximation claim for neighborhood robustness
No approximation claim survives third pass for neighborhood robustness. Any attempt to approximate it without retaining the full local surface or a proved equivalent summary is heuristic and should be rejected.

## 7. Time, memory, and retained-state complexity

Let:

- \(n=|C|\): shortlist size,
- \(p\): number of union objectives,
- \(k \in \{2,4\}\): slice count,
- \(m \in \{17,22\}\): full metrics-row width.

The current normal full exact contract is \(m=17\), with \(m=22\) only for the trade-extended exact contract.

### Union exact rerank
- Retained state: \(O(np)\) objective scores + \(O(n)\) drawdowns.
- Memory: \(O(np)\).
- Time:
  - lexicographic: \(O(np + n\log n)\),
  - weighted-rank: typically \(O(p\,n\log n)\),
  - Pareto / epsilon-front: worst-case \(O(n^2 p)\) if fronts are recomputed naïvely.

### Confidence exact rerank
- Retained state: \(O(n)\) candidate tuples + \(O(1)\) peer summary.
- Memory: \(O(n)\).
- Time: \(O(n)\) to compute peer stats and confidence scalars, plus \(O(n\log n)\) for sorting.

### Slice exact order only
- Retained state after replay: \(O(n)\) scalar rerank tuples.
- Memory: \(O(n)\).
- Replay cost is not avoided unless slice-local values were already retained.

### Slice exact diagnostic parity
- Retained state: \(O(nkm)\) if retaining full per-slice metrics rows, plus \(O(nk)\) slice labels/ranges/scores.
- Memory: \(O(nkm)\).
- Current time shape: one batched exact replay over all shortlisted candidates across all active slices.

### Benchmark-relative slice parity
- Additional retained state: \(O(nk)\) slice excess returns, or \(O(n)\) if only the final benchmark-relative tuple is retained and diagnostic parity does not require raw slice excess values.

### Neighborhood exact parity
Let \(q\) be the number of local stability points (typically \((2r+1)^2\) before feasibility pruning).
- Retained state: at least \(O(qm)\) if retaining local point metrics rows plus current-score values.
- Memory: \(O(qm)\).
- No smaller proved exact summary exists at third pass.

## 8. File and symbol impact map

## Primary ownership files

### `crates/app_core/src/double_ma.rs`
Owns or materially constrains:

- `effective_score`
- `score_mode_score_fn`
- `score_mode_needs_pair_observation_context`
- request validation for:
  - `extra_shortlist_objectives`
  - `shortlist_confidence_mode`
  - `shortlist_union_mode`
  - `shortlist_slice_mode`
  - `shortlist_slice_benchmark`
  - shortlist neighborhood config
- dynamic exact-compatible stage sequencing
- local-stability follow-up:
  - `compute_double_ma_local_stability_blocking_with_candles`
  - `summarize_shortlist_neighborhood_robustness`
  - `summarize_shortlist_surface_agreement`
  - `summarize_shortlist_surface_explanation`

### `crates/app_core/src/gpu_backtest_kernel.rs`
Owns or materially constrains:

- `score_first_topk_blockers`
- metric-contract constants:
  - `BASE_METRICS_COUNT`
  - `PRODUCT_REDUCED_METRICS_COUNT`
  - `METRICS_COUNT`
  - `TRADE_METRICS_COUNT`
- `shortlist_candidate_track_record_bars`
- `effective_sample_lcb_10_pair_score`
- `kernel_effective_score_for_pair`
- `rerank_shortlist_by_union_mode`
- `rerank_shortlist_by_confidence`
- `shortlist_slice_ranges`
- `compute_shortlist_slice_omission_summary`
- `compute_shortlist_slice_benchmark_relative_summary`
- `shortlist_slice_candidate_return_sum`
- exact GPU stage sequencing in host finalize
- batched slice replay assembly

### `crates/app_core/src/validation_stats.rs`
Interface owner for:
- `compute_psr_dsr`

PR-02 may rely on that interface but should not redesign the underlying validation model.

### `crates/app_core/src/vemr_gpu_kernel.rs`
No direct third-pass redesign burden surfaced in the current proof. It remains an interface only.

## Context files

### `crates/app_core/src/composite_cpu.rs`
Constraint only.
Its standalone shortlist flow mirrors union/confidence behavior and therefore must remain semantically aligned, but PR-02 should not redesign that surface from here.

### `crates/strategies/src/double_ma/mod.rs`
Constraint only.
Metric field meanings belong to `PR-01`, not `PR-02`.

## 9. Verification obligations and falsifiers

## 9.1. Verification obligations

### Route-aware order parity
1. **Exact GPU path**
   - union parity against host reconstruction,
   - confidence parity against manual PSR/DSR order,
   - slice parity using the actual exact GPU stage order.

2. **Dynamic exact-compatible path**
   - union parity against host reconstruction,
   - confidence parity against manual PSR/DSR order,
   - explicit route-order tests showing that union does not feed confidence/slice through the materialized shortlist when extra heads are active.

### Confidence edge cases
3. `EffectiveSampleLcb10` must verify the current tie-break scalar, not a hypothetical pair-aware tie-break.
4. DSR tests must verify peer-set dependence by changing the shortlist peer distribution while holding a candidate fixed.

### Slice and omission parity
5. Common-start slicing must be verified under heterogeneous warmups.
6. Omission summaries must be checked under ties and near-ties.
7. Halves and quartiles must both be verified.
8. Benchmark-relative parity must be checked for both `Flat` and `BuyAndHold`.

### Neighborhood parity
9. The stored neighborhood summary must match direct local-stability follow-up around the final winner.

## 9.2. Falsifiers

The third-pass result is false if any of the following happen:

1. the dynamic exact-compatible path is changed so union, confidence, and slice truly compose through one another;
2. the exact GPU path is changed so slice consumes the confidence output even when extra shortlist heads are active;
3. confidence reranking switches its tie-break from `effective_score(...)` to the pair-aware `kernel_effective_score_for_pair(...)`;
4. benchmark-relative slice semantics are redefined from arithmetic proxies to exact pathwise compounded slice PnL;
5. a smaller exact neighborhood sufficient statistic is proved.

## 10. Dependencies that must be reconciled during consolidation

1. **PR-01 metric semantics**
   - `derived_mean`, `pnl`, `volatility`, `max_dd`, `omega`, and related fields are treated as fixed interfaces here.
   - Benchmark-relative exactness depends on those semantics.

2. **PR-06 validation statistics**
   - DSR and PSR exactness depend on `compute_psr_dsr` exactly as implemented.
   - Any redesign there changes the confidence theorem.

3. **Local stability subsystem**
   - Neighborhood robustness exactness depends on the semantics of the local-stability engine and its alternate score surfaces.
   - PR-02 should treat that subsystem as an interface.

4. **Score-first work**
   - Current score-first blockers still exclude the common PR-02 shortlist surfaces.
   - Any future score-first extension that claims PR-02 parity must first reconcile those blockers and then restate the route theorem.

5. **Composite CPU shortlist mirror**
   - Union/confidence behavior in `composite_cpu.rs` should remain aligned, but PR-02 should not redesign that context surface.

## 11. Kill criteria

The implementation-ready PR-02 result should be rejected if consolidation asks it to support any of the following without first changing the underlying code semantics:

1. **A single universal minimal retained summary** for all current PR-02 outputs.
2. **A route-independent composition theorem** for union, confidence, slice, and neighborhood.
3. **Exact slice or omission parity from full-period arithmetic aggregates alone**.
4. **Exact benchmark-relative parity as exact pathwise compounded slice PnL**.
5. **Exact neighborhood parity from the winner row alone**.
6. **Exact DSR reranking from candidate-local summaries alone**.
7. **Confidence exactness for `EffectiveSampleLcb10` using a pair-aware tie-break** when the current comparator does not do that.
8. **Approximate union claims without a front/rank preservation contract**.
9. **Approximate neighborhood claims without a full local-surface error contract**.
10. **Any score-first claim that treats the current narrow score-first path as already covering the common exact PR-02 route**.

## Bottom line

The strongest surviving implementation-ready result is:

> PR-02 is not governed by one universal replay-free summary theorem. The current exact frontier is route-aware and stage-specific. Exact replay-free reranking is currently available for union and confidence from shortlist-level retained summaries that preserve the actual code comparators. Exact slice, omission, benchmark-relative slice, and neighborhood outputs still require final-partition slice-local or neighborhood-local retained state; otherwise replay is mathematically forced.

