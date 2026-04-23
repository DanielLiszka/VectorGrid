# PR-05 Composite search and utility math — all passes

Based on the uploaded `PROMPT.md` for `PR-05`. fileciteturn16file0

---

## First-pass answer

### First-pass framing

Treat `PR-05` as a discrete-search geometry problem over repo-defined metrics, not as a metric-redesign problem.

### First-pass result

The strongest repo-fit first pass was:

1. **Single-objective `CompositeCpu`** should use the repo’s existing signed objective scalarization: reward metrics are ranked directly, risk metrics are negated, and non-finite scores are clamped to `-∞`.
2. **`CompositeMo`** should be modeled as a constrained two-objective maximize-both problem after polarity normalization.
3. **Exact-grid and candidate-set equivalence** can only be claimed relative to the declared evaluated domain.
4. **Prescan** can only be exact if its filtered candidate domain contains the true winner or true Pareto set.
5. **Heatmaps** are sampled-bin telemetry, not faithful surrogates for unsampled regions.

### First-pass formal object

Let `G` be the discrete candidate lattice, `M(g)` the metric vector, `σ` the scalarization, and `P*` the exact Pareto set.

The first-pass candidate contracts were:

- exact search,
- winner-preserving approximation,
- Pareto-cover approximation,
- regret-bounded approximation,
- otherwise heuristic.

### First-pass positive result

If two procedures enumerate the same candidate set and use the same scalarization, they preserve winner parity up to ties.

### First-pass impossibility result

Without candidate inclusion or extra regularity assumptions, no strict prescan can guarantee preservation of the full-grid winner or front on arbitrary finite lattices.

### First-pass unresolved questions

- What exact notion of rank stability matters: winner, top-k, shortlist, or Pareto?
- What structure, if any, justifies prescan regret bounds?
- What exact public semantics should heatmaps claim?
- Is the authoritative object the base-stage winner or a reranked surfaced winner?

---

## Second-pass answer

### Second-pass attack

The first-pass model was too coarse because the repo does not expose only a single ranking on a raw lattice.

The surviving object is layered:

`raw candidates -> repair -> repaired unique domain -> evaluation -> base score/frontier -> optional rerank -> surfaced winner`.

### Second-pass breakpoints

1. **Scalarization is not globally unique.** `StreamAggregator` supports `with_score_fn` and `with_pair_score_fn`, so ranking can override the default objective score.
2. **Surfaced `CompositeCpu` winners need not maximize the primary objective.** Union rerank and confidence rerank can overwrite the best row after base aggregation.
3. **Candidate-set exactness is only exact on the repaired unique image of the supplied candidates.**
4. **`exact_grid_used=true` is not the same as full-space exact.** `CompositeMo` labels both full-space exact-grid and explicit-candidate exact-grid as `ExactGrid`.
5. **All-infeasible `CompositeMo` output is not a normal Pareto front.** It collapses to the epsilon-min-violation tie set.
6. **Prescan is weaker than first pass suggested.** Predictive rescue adds seeds, but later route constraints and narrowing can still change the effective domain.
7. **Heatmap summaries are push-weighted sample summaries, not unique-candidate summaries.**
8. **`CompositeMo.mode` is ignored**, so no theorem should depend on it.
9. **Telemetry `actual_evaluations` excludes the final Trade22 enrichment pass.**

### Strongest surviving second-pass result

The only strong exact theorem that survived was a **base-stage theorem**:

- if two methods enumerate the same repaired domain,
- use the same declared base ranker,
- and do not apply different rerank stages,

then they agree on the best-score tie set.

Everything stronger required narrowing the claim to a declared repaired domain and a declared ranking functional.

---

## Third-pass answer

This is the strongest implementation-ready mathematical result that survives the second pass.

### 1. Final formalization

#### 1.1 Declared domains

Let `G0` be the raw decoded candidate set for a request.

Let `ρ` be the repo repair map, including decode repair and structural repair such as enforcing `fast_len < slow_len` where applicable.

For any raw candidate domain `D ⊆ G0`, define the **repaired unique image**

`U(D) = { ρ(x) : x in D }_unique`.

Every exactness statement in `PR-05` must be about a declared repaired domain `U(D)`, not about the unrepaired raw domain.

This is required because `CompositeMo` exact candidate-set search repairs every supplied candidate before evaluation and deduplicates repaired points through cache-backed batch evaluation. fileciteturn26file9 fileciteturn26file18

#### 1.2 Search-domain classes

There are only three exact-domain classes worth naming:

- **Full repaired domain**: `D = G0`, exact on `U(G0)`.
- **Explicit repaired candidate domain**: exact on `U(C)` for an explicit candidate list `C`.
- **Filtered repaired domain**: exact on `U(F(G0))` for a filter `F`, such as prescan-allowed pairs.

`CompositeMo` already uses both the full-space and explicit-candidate exact-grid plans, and labels both with the same runtime label `ExactGrid`, so the domain class must be tracked separately from the execution label. fileciteturn26file1 fileciteturn26file2

#### 1.3 Evaluation map

For each repaired candidate `u in U(D)`, let

`E(u) = (M(u), c(u), payload(u))`

where:

- `M(u)` is the metric vector,
- `c(u)` is the nonnegative constraint-violation vector,
- `payload(u)` is any route-specific output used later.

Metric meaning is not redesigned here.

#### 1.4 Base ranking functionals

For single-objective `CompositeCpu`, the **default base ranker** is the repo’s signed objective score

- `Sharpe -> sharpe`
- `Sortino -> sortino`
- `Calmar -> calmar`
- `Omega -> omega`
- `Pnl -> pnl`
- `Volatility -> -volatility`
- `MaxDrawdown -> -max_dd`

with non-finite values clamped to `-∞` in aggregation. `CompositeCpu::objective_score` matches this polarity, and `StreamAggregator` applies the same finite-value convention when no custom score override is installed. fileciteturn20file8 fileciteturn24file14

For single-objective math, define this default ranker as `s_o(u)`.

For `CompositeMo`, the optimizer crate stores objectives already normalized to “maximize both,” and constrained domination is:

- feasible dominates infeasible,
- among infeasible points, lower violation dominates,
- among feasible points, standard two-objective maximize-both Pareto dominance applies. fileciteturn27file12 fileciteturn27file9 fileciteturn27file3

#### 1.5 Declared ranker versus surfaced ranker

The critical surviving distinction is:

- **base ranker** `s` or base Pareto operator `Π`, used by the search stage;
- **surfaced reranker** `R`, optionally applied after base search.

For `CompositeCpu`, the base search uses the default objective score unless a custom stream score override is explicitly installed. But the support crate allows `with_score_fn` and `with_pair_score_fn`, so exactness theorems must be parameterized by the declared ranker, not hard-coded to the objective alone. fileciteturn20file3 fileciteturn20file16

The surfaced `CompositeCpu` result may then apply:

- `R_id`: no rerank,
- `R_union`: exact extra-objective shortlist build plus optional union rerank,
- `R_conf`: confidence rerank on the existing shortlist.

`R_union` requires `include_all=true` and uses exact `all` rows to build the extra shortlist before resetting `best_params`, `best_metrics`, `best_score`, and `top`. `R_conf` reranks `result.top` and then also rewrites those surfaced fields. fileciteturn29file2 fileciteturn29file3 fileciteturn18file15

So the authoritative mathematical object is not just `(U(D), s)` but `(U(D), s, R)`.

#### 1.6 Prescan filter

Let `F_prescan` denote the stage-0 filter induced by `CompositeGpuPrescanConfig`.

For `CompositeCpu`, prescan is an optional stage-0 Double-MA run that can narrow `(fast_len, slow_len)` before the composite search. Predictive rescue adds seed pairs through `predictive_prescan_seed_pairs(...)`, parameterized by `predictive_top_k` and `predictive_stride`. Explicit prescan can also alter backend behavior because the base `CompositeCpu` `GpuKernel` route does not support prescan-subset `allowed_pairs`. fileciteturn25file5 fileciteturn25file7 fileciteturn25file9 fileciteturn25file3

For `CompositeMo`, `CompositeGpuPrescanConfig` exposes `top_k`, `window`, `predictive_top_k`, `predictive_stride`, `device_id`, `require_gpu`, `objectives`, `ma_pairs`, and `mode`. Predictive rescue is explicitly documented as adding top-K MA pairs on a coarse MA grid. Prescan may span multiple MA type pairs, and fixed-type requests must include the base pair in `ma_pairs`. fileciteturn25file13 fileciteturn28file16 fileciteturn28file15

Prescan therefore defines a **filtered domain** `U(F_prescan(G0))`, not a theorem of exactness.

#### 1.7 Heatmap object

For a fixed bin layout `H`, the stream heatmap stores per-bin:

- sampled max score,
- sampled sum,
- sampled count,

and finalizes to max / mean / count. Merge is exact only when layouts match; otherwise a nonempty/nonempty mismatch invalidates the heatmap. These are push-weighted sample summaries, not unique-candidate summaries. fileciteturn20file5 fileciteturn20file16 fileciteturn20file18

---

### 2. Final theorem and lemma list

#### Lemma 1 — Base-stage exactness on a declared repaired domain

If two procedures enumerate the same repaired domain `U(D)`, use the same declared base ranker `s`, and either both use `R_id` or both apply the same rerank operator to the same exact information set, then they return the same maximizing tie set for single-objective search.

#### Lemma 2 — Candidate-set exactness is exact on the repair quotient

`candidate_set_mo_search_with_stats(C, ...)` is exact on `U(C)`, not on raw `C` as distinct points. This follows because candidates are repaired chunk-by-chunk before evaluation, then deduplicated by cache. fileciteturn26file9 fileciteturn27file13

#### Lemma 3 — `ExactGrid` is an execution label, not a full-space certificate

In `CompositeMo`, both `ExactGridPlan::FullSpace` and `ExactGridPlan::ExplicitCandidates` surface as sampler label `ExactGrid`. Therefore `exact_grid_used=true` certifies exactness only on the declared repaired search domain, not automatically on the original full request space. fileciteturn26file1 fileciteturn26file2

#### Lemma 4 — All-infeasible `CompositeMo` output is a min-violation tie set

If no feasible points exist, `pareto_front_constrained_indices` returns every point whose violation lies within epsilon of the minimum violation. This is not a usual objective-space Pareto front. fileciteturn27file9

#### Lemma 5 — Heatmap exactness is only sampled-bin exactness

For a fixed layout, heatmap max/mean/count are exact summaries of pushed samples only. No stronger claim about unsampled quality, unique-candidate occupancy, or frontier geometry follows. fileciteturn20file5 fileciteturn20file16

#### Lemma 6 — Union rerank can be exact; confidence rerank generally cannot

If `CompositeCpu` uses exact `all` rows, `include_all=true`, and the declared surfaced operator is the repo’s union-shortlist functional, then the surfaced union-reranked result is exact relative to that rerank functional. But confidence rerank operates on the existing shortlist `result.top`, so it is exact only relative to that shortlist, not generally to the full repaired domain. fileciteturn29file2 fileciteturn29file3 fileciteturn18file15

#### Lemma 7 — Prescan has no exactness theorem without a certificate

A prescan-restricted search is exact only if there is an inclusion certificate proving that every target winner/frontier point lies in the filtered repaired domain. Without that certificate, the method is heuristic.

#### Lemma 8 — `CompositeMo.mode` carries no search semantics today

`CompositeMoRequest.mode` is currently ignored, and a test asserts parity between `mode=None` and `mode=CoarseToFine`. So no theorem may depend on `CompositeMo.mode` until that interface changes. fileciteturn28file3 fileciteturn28file5 fileciteturn28file10

---

### 3. Proof sketch / proof plan

- **Lemma 1**: identical declared domain plus identical ranker induces identical score preorder; identical rerank on identical information preserves identity.
- **Lemma 2**: explicit candidate search repairs each candidate, then `evaluate_batch_with_cache` deduplicates repaired values by hashed value equality before counting actual evaluations.
- **Lemma 3**: direct inspection of the `CompositeMo` exact-grid branch shows both full-space and explicit-candidate plans mapped to the same sampler label.
- **Lemma 4**: direct inspection of constrained Pareto logic shows all-feasible and no-feasible branches are distinct, with the latter returning epsilon-min-violation ties.
- **Lemma 5**: heatmap update is by push aggregation of max / sum / count, and merge either merges these exact sampled aggregates or invalidates the heatmap on incompatible layouts.
- **Lemma 7**: adversarially place the full-grid winner outside the filtered domain while making all observed points identical.

---

### 4. Sufficient statistics, or proof they do not exist

#### 4.1 What is sufficient

For **single-objective base-stage exactness** on a declared repaired domain `U(D)`, the sufficient statistic is:

- the maximizing score value,
- a deterministic representative or maximizing tie set,
- and a top-k heap if only top-k output is needed.

For **union-reranked surfaced exactness**, the sufficient statistic is larger:

- exact `all` rows for the declared domain,
- the primary objective,
- the extra objective list,
- the declared union mode.

That is exactly why the repo requires `include_all=true` for standalone extra-objective union handling. fileciteturn29file1 fileciteturn29file2

#### 4.2 What is not sufficient

There is no finite sufficient statistic under current constraints for:

- unsampled heatmap interpretation,
- unique-candidate occupancy from heatmap alone,
- full-domain confidence-rerank exactness from shortlist-only data,
- or full-space exactness from `exact_grid_used=true` alone.

In particular, `(bin max, bin mean, bin count)` is not sufficient to recover within-bin ranking law or frontier structure, and confidence rerank on `result.top` cannot certify full-domain optimality unless the full-domain optimum is already known to lie in that shortlist.

---

### 5. Exactness conditions and equivalence classes

Define the following exactness classes.

#### 5.1 Domain classes

- **E-full**: exact on `U(G0)`.
- **E-explicit**: exact on `U(C)` for an explicit candidate list `C`.
- **E-filtered**: exact on `U(F(G0))` for a declared filter `F`.

#### 5.2 Surfaced-result classes

- **B-exact**: base-stage result is exact on the declared domain under the declared base ranker.
- **S-exact**: surfaced result is exact because rerank is identity or has complete exact information.
- **S-shortlist-exact**: surfaced result is exact only relative to the exact shortlist it reranked.

#### 5.3 Equivalence relations

Two raw domains `D1, D2` are equivalent for exactness if `U(D1) = U(D2)`.

Two heatmap traces are equivalent only if they induce the same pushed sample multiset under the same layout.

This repair-quotient view is mandatory for `CompositeMo` candidate-set exactness. fileciteturn26file9 fileciteturn26file18

---

### 6. Error, regret, or bias bounds for every non-exact claim

This is the key third-pass requirement.

#### 6.1 Prescan and predictive rescue

There is **no deterministic ex-ante regret bound supported by the repo contract today**.

Therefore the only allowed contract is:

- **status**: heuristic filter,
- **deterministic bound**: none,
- **required audit metrics on exact audit surfaces**:
  - score regret `Δ_score = s(w_full) - s(w_filtered) >= 0`,
  - rank gap `Δ_rank = rank_full(w_filtered) - 1`,
  - winner recall `1{w_full in U(F(G0))}`,
  - Pareto recall / coverage against the full repaired front.

No stronger contract may be claimed without adding a separate structural assumption package.

#### 6.2 Random / TPE / QMC / NSGA2 samplers in `CompositeMo`

These are also **non-exact with no deterministic repo-backed ex-ante regret bound**.

Allowed contract:

- **status**: heuristic sampler,
- **deterministic bound**: none,
- **reported telemetry**: requested evaluations, actual evaluations, unique evaluations, duplicates avoided,
- **required audit on bounded exact surfaces**: full-grid score/frontier gap when the exact domain is small enough. fileciteturn25file19 fileciteturn26file3 fileciteturn26file16

#### 6.3 Confidence rerank

Confidence rerank is exact only on its input shortlist.

Allowed contract:

- **status**: `S-shortlist-exact`,
- **deterministic full-domain bound**: none,
- **shortlist-domain regret**: zero by definition on the reranked shortlist,
- **full-domain audit**: compare against confidence rerank applied to exact all-rows when available.

#### 6.4 Heatmap interpretation beyond sampled bins

Allowed contract:

- **status**: exact sampled-bin summary,
- **bias outside sampled semantics**: unbounded.

So any product language stronger than “sampled max/mean/count by bin” must be rejected.

---

### 7. Time, memory, and retained-state complexity

Let `N = |U(D)|` be the declared repaired exact domain size.

#### 7.1 Single-objective exact base search

- time: `Θ(N * eval_cost)`,
- top-k memory: `O(k)`,
- include-all memory: `O(N)` if `include_all=true`,
- heatmap memory: `O(Bf * Bs)` bins with max/sum/count backing arrays.

This follows from the streaming aggregator structure. fileciteturn24file14 fileciteturn20file18

#### 7.2 `CompositeMo` exact explicit-candidate search

For raw candidate list size `|C|`:

- requested work is `|C|`,
- actual unique evaluations are `|U(C)|`,
- duplicate avoidance is explicitly tracked,
- frontier memory is `O(|frontier|)`,
- cache memory is `O(|U(C)|)`. fileciteturn26file9 fileciteturn27file13

#### 7.3 Prescan cost

Prescan adds a stage-0 Double-MA run plus filtered-domain construction. For `CompositeCpu`, explicit prescan can also change backend route eligibility because the base `GpuKernel` path rejects prescan-subset `allowed_pairs`. fileciteturn25file5 fileciteturn25file3

#### 7.4 `CompositeMo` enrichment tail

Search telemetry `actual_evaluations` excludes the final CPU `Trade22` Pareto enrichment pass, so end-to-end work is:

`search work + pareto enrichment tail`.

That distinction must remain explicit. fileciteturn25file19 fileciteturn26file2

---

### 8. File and symbol impact map

- **`crates/app_core/src/composite_cpu.rs`**
  - `objective_score`
  - `gpu_prescan`
  - `apply_requested_standalone_union_shortlist`
  - `apply_requested_standalone_confidence_rerank`
  - base `GpuKernel` rejection of prescan-subset `allowed_pairs`

- **`crates/app_core/src/composite_mo.rs`**
  - `CompositeGpuPrescanConfig`
  - `CompositeMoRequest.mode` ignored semantics
  - `ExactGridPlan`
  - prescan telemetry
  - search telemetry
  - prescan-pair rejection counters
  - final Trade22 Pareto enrichment

- **`crates/optimizer/src/stream.rs`**
  - `StreamAggregator`
  - `with_score_fn`
  - `with_pair_score_fn`
  - heatmap push / merge / finalize

- **`crates/optimizer/src/mo.rs`**
  - `candidate_set_mo_search_with_stats`
  - `exact_grid_mo_search_with_stats`
  - constrained domination
  - `pareto_front_constrained_indices`
  - `MoSearchStats`

- **`crates/optimizer/src/grid.rs` / `lib.rs`**
  - exact single-objective objective polarity and result containers

- **`crates/strategies/src/composite_cpu/mod.rs`**
  - path-functional semantics boundary only; not a redesign target for metric meaning.

---

### 9. Verification obligations and falsifiers

#### 9.1 Exactness obligations

Every exactness claim must name:

- the raw domain,
- the repaired exact domain,
- the declared base ranker,
- the surfaced rerank operator,
- and whether the claim is `B-exact`, `S-exact`, or `S-shortlist-exact`.

#### 9.2 Falsifiers

A proposal fails if any of the following occur:

- a full-grid audit winner is absent from the prescan-filtered repaired domain,
- a claimed full-space exact result is only exact on `U(C)` for explicit candidates,
- a confidence-reranked surfaced winner differs from the full-domain confidence optimum,
- a heatmap is interpreted as an unsampled surface estimate,
- telemetry `actual_evaluations` is treated as end-to-end work.

Specific repo-backed falsifiers already exist:

- `CompositeMo.mode` changing nothing,
- fixed-type `gpu_prescan.ma_pairs` excluding the base pair and correctly erroring,
- heatmap invalidation on nonempty mismatched layouts. fileciteturn28file5 fileciteturn28file15 fileciteturn20file18

---

### 10. Dependencies that must be reconciled during consolidation

These are interfaces, not redesign permission.

1. **Metric semantics (`PR-01`)**: `PR-05` must consume metric polarity, not redefine metric meaning.
2. **Shortlist / surfaced-result policy**: the product must distinguish base exactness from surfaced reranked exactness.
3. **Telemetry policy**: exact-domain kind and search-versus-enrichment work should remain distinguishable.
4. **Backend route policy**: prescan can change route admissibility for `CompositeCpu` because `allowed_pairs` is not supported on its base `GpuKernel` path.
5. **CompositeMo workflow identity**: `CompositeMo` is its own search-space / sampler workflow, not “CompositeCpu with more knobs.” fileciteturn25file10

---

### 11. Kill criteria

Reject any implementation or product claim that does any of the following:

- claims full-space exactness without naming the declared repaired domain,
- equates `exact_grid_used=true` with full request-space exactness,
- claims deterministic prescan regret bounds without extra structure,
- conflates base-stage exact winners with surfaced reranked winners,
- treats confidence rerank as full-domain exact without a shortlist inclusion certificate,
- treats heatmap max/mean/count as sufficient for unsampled landscape claims,
- uses `CompositeMo.mode` as if it already changes search geometry,
- uses search `actual_evaluations` as total end-to-end work.

---

## Final bottom line

The strongest implementation-ready mathematical result for `PR-05` is a **taxonomy with precise exactness boundaries**:

- exactness is always relative to a **declared repaired domain**,
- ranking is always relative to a **declared base ranker**,
- surfaced results are always relative to a **declared rerank operator**,
- prescan and non-exact samplers are **heuristic unless accompanied by an explicit certificate**, and otherwise may only ship with **ex-post audit contracts**,
- heatmaps are **exact sampled-bin summaries and nothing more**.

That is the strongest repo-fit result that survives the third pass.
