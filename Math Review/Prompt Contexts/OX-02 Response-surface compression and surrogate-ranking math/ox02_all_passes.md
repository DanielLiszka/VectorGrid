# OX-02 Response-surface compression and surrogate-ranking math

This document packages the first-pass, second-pass, and third-pass answers for the uploaded `OX-02` overlay prompt.

The overlay asks for a response-surface compression lens over these repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/meta_select.rs`

The prompt specifically asks for reductions that cut evaluations or improve ranking quality **without replacing exact final scoring with a generic black-box optimizer**.

Code references below use the uploaded local repo copies and are written as `path:line-line`.

---

## First-pass answer

Because the core `PR-*` prompt was not provided, this is an overlay-only first pass rather than an attached-core answer.

My first-pass verdict is yes, but the exploitable structure is **not** one global low-rank surrogate over the whole mixed search space. The repo already shows a safer pattern: compress first, then replay exact scoring on a reduced candidate universe.

In `double_ma.rs`, ranking already behaves like a low-dimensional local surrogate problem. The file defines multiple score modes (`BalancedRiskAdjusted10`, `AdversarialEnvelope10`, `SupportShrunk10`, `ActiveSortino10`, `EffectiveSampleLcb10`, `OmegaVsSharpe10`, `RobustMargin10`) plus shortlist union modes (`Lexicographic`, `Pareto`, `EpsilonFront`, `WeightedRank`) (`double_ma.rs:193-218`). `effective_score(...)` routes ranking through either a score-mode function or the base objective (`double_ma.rs:1334-1376`). The same file also computes exact local neighborhood summaries such as `plateau_score`, `roughness_stability`, `curvature_stability`, and `agreement_floor` (`double_ma.rs:1903-2188`), and then runs an exact local replay around the chosen winner (`double_ma.rs:7695-7725`). `CoarseToFine` is another direct sign of local lattice regularity: the search first reduces the grid, then reopens an exact fine window around the coarse winner (`double_ma.rs:6247-6403`).

In `composite_mo.rs`, `gpu_prescan` already implements staged compression. It prescans MA pairs, expands windows around seeded winners, optionally adds predictive rescue pairs, rewrites `fast_len` and `slow_len` into discrete candidate sets, and then searches either an exact explicit candidate set or a fallback sampler (`composite_mo.rs:1188-1578`, `composite_mo.rs:2805-2989`). After search, Pareto survivors are re-evaluated exactly with CPU `Trade22` metrics (`composite_mo.rs:3048-3132`). That is exactly the repo-fit pattern the overlay is looking for.

In `meta_select.rs`, grouped clustering is already a response-surface compression layer. `MetaGroupingMode` defaults to `Clustered`, `MetaClusterConfig` quantizes integer and float parameters, and nearby parameterizations can collapse into a single grouped region (`meta_select.rs:123-190`, `meta_select.rs:1179-1224`). The aggregate owns regional train-stage quantities such as `samples`, `datasets_covered`, `robust_score_p10`, `tail_gap`, and `meta_score`, while `final_score` is explicitly left to a later holdout stage (`meta_select.rs:240-330`, `meta_select.rs:680-775`). The included regression test confirms that nearby settings merge into one region with multiple representatives (`meta_select.rs:2183-2196`).

In `mo.rs`, the exactness boundary is already clear. `candidate_set_mo_search_with_stats(...)` evaluates an explicit candidate set in chunks, applies repair, caches duplicates, and keeps search exact on the supplied reduced set (`mo.rs:2409-2462`). The TPE path is explicitly approximate: it caps `max_model_points`, ranks the working subset with constrained Pareto fronts and crowding distance, and only then builds good/bad per-parameter models (`mo.rs:1189-1236`, `mo.rs:1982-2077`). So the best reduction target is the explicit candidate set, not the final scorer.

### New lemma candidate

A useful new lemma is a **local elite-preservation lemma**.

Let `U` be the exact local neighborhood returned by `compute_double_ma_local_stability_blocking_with_candles`, let `f` be the exact score used in that neighborhood, and let `\tilde f` be any local surrogate fitted only on `U`. If

```text
sup_{g in U} |f(g) - \tilde f(g)| <= ε
```

and the elite margin

```text
γ := min_{g in E, h not in E} (f(g) - f(h))
```

satisfies `γ > 2ε`, then elite membership is preserved on `U`.

The implementation relevance is that `double_ma.rs` already computes the exact neighborhood diagnostics needed to decide whether such a local surrogate is even admissible (`double_ma.rs:1903-2188`, `double_ma.rs:4697-4765`, `double_ma.rs:7695-7725`).

### New bound

A clean first-pass bound is the **head-union bound** for `build_exact_extra_shortlist_rows(...)`. If there are `m` distinct heads after deduplication and each contributes at most `k` rows, then the deduplicated union has size at most `mk` (`double_ma.rs:2437-2497`). That makes the cost of exact replay over the union linear in the number of heads and `top_k`.

### New counterexample family

The main counterexample family is a **thin diagonal ridge**. If the true good region lies near a relation like `slow_len ≈ 2 * fast_len + c`, then axis-aligned windows and coarse stride rescue can miss it even when local windows look reasonable. `composite_mo.rs`'s predictive rescue helps, but it does not prove coverage of such a ridge (`composite_mo.rs:1423-1456`).

A second counterexample is the **chimera shortlist miss**: a candidate that ranks just outside the top-`k` set on every head, but would win under later Pareto-aware or weighted reranking. Because `build_exact_extra_shortlist_rows(...)` unions only per-head top-`k` sets, such a candidate can be pruned before any later exact step sees it (`double_ma.rs:2437-2497`).

### First-pass bottom line

The repo already contains a good compression stack, but it is **local, staged, and exact-at-the-end**. The right first-pass recommendation is not “add a global surrogate optimizer.” It is:

- use `double_ma` neighborhood summaries only as local compression or certification tools
- use `composite_mo` prescan to create a much smaller explicit candidate universe
- use `meta_select` clustering as region proposal rather than pointwise truth
- use `mo.rs` exact candidate-set search whenever prescan has already made the candidate universe small enough

---

## Second-pass answer

The second-pass correction is stricter: the safe exactness claim is **not** “exact over the full original lattice after surrogate help.” The correct claim is **exact final scoring over a reduced candidate universe**.

In `double_ma.rs`, not all score modes are equally surrogate-friendly. `score_mode_needs_pair_observation_context(...)` marks `EffectiveSampleLcb10` as requiring pair observation context, and `score_mode_score_fn(...)` returns `None` for that mode while returning pure metric score functions for the others (`double_ma.rs:1334-1364`). So a metric-only local surrogate is admissible only for the metric-only score modes, not uniformly for every ranking mode.

In `composite_mo.rs`, once `gpu_prescan` is enabled, exactness is already relative to a pruned space. Prescan expands seed windows, optionally adds predictive rescue pairs, rewrites `fast_len` and `slow_len` into categorical candidate sets, and then either builds an `ExplicitCandidates` exact-grid plan or falls back to a sampler (`composite_mo.rs:1188-1578`, `composite_mo.rs:2805-2989`). After that, Pareto survivors are re-evaluated exactly with CPU `Trade22` metrics (`composite_mo.rs:3048-3132`). The telemetry comments even separate search-phase evaluations from the final CPU enrichment pass (`composite_mo.rs:434-458`).

In `meta_select.rs`, clustered mode changes the semantic unit from a point to a region. `MetaGroupingMode` defaults to `Clustered`, `MetaClusterConfig` quantizes parameter values, and the aggregate owns `meta_score` while `final_score` stays optional until later holdout enrichment (`meta_select.rs:123-190`, `meta_select.rs:240-330`). So grouped output is region-level evidence, not exact pointwise ranking.

In `mo.rs`, the contrast is explicit. `candidate_set_mo_search_with_stats(...)` is exact on the supplied candidate set (`mo.rs:2409-2462`). TPE, by contrast, truncates the working subset at `max_model_points` and ranks that subset with constrained non-dominated fronts and crowding distance before fitting probabilistic models (`mo.rs:1189-1236`, `mo.rs:1982-2077`). So once an explicit reduced candidate universe exists, exact candidate-set search is mathematically preferable to TPE.

### Second-pass lemma

The best second-pass lemma remains the **local elite-preservation lemma**, but now with a sharper exactness reading: it can only justify local surrogate ranking **inside an already exact local replay neighborhood**, never as a replacement for the final scorer.

### Second-pass bounds

The **head-union bound** in `double_ma.rs` becomes more important in second pass form. `build_exact_extra_shortlist_rows(...)` preserves every candidate that is top-`k` for at least one head, but everything outside that union is permanently invisible to later exact reranking (`double_ma.rs:2437-2497`). So exactness here means “exact on the head-union,” not “exact on the full original shortlist.”

A stronger reduction bound comes from `composite_mo.rs`. Once prescan has built `allowed_pairs_by_type`, the exact search space factors into:

```text
N_explicit = sum_{b in B} |A_tau(b)| <= |B| * max_tau |A_tau|
```

where `B` is the exact reduced grid over the non-MA axes and `A_tau` is the allowed pair set for MA-type pair `tau` (`composite_mo.rs:2805-2928`). This is the real complexity reduction in the repo: replace the original full product over `fast_len` and `slow_len` with a smaller reduced base grid times a bounded per-type allowed-pair set.

### Second-pass counterexamples

The thin diagonal ridge still matters, because prescan windows and rescue stride are mostly axis-aligned (`composite_mo.rs:1423-1456`).

The chimera shortlist miss still matters, because per-head top-`k` union can discard a candidate that is never individually elite but is globally excellent after downstream multi-head reranking (`double_ma.rs:2437-2497`).

A third second-pass counterexample is the **bucket-boundary inversion** in `meta_select.rs`: clustered grouping quantizes nearby params, and if score changes sharply across a quantization boundary, a high-quality region can still misorder the best individual point. The included test that collapses two nearby settings into one grouped region is good evidence that grouped output must be interpreted regionally (`meta_select.rs:1179-1224`, `meta_select.rs:2183-2196`).

### Second-pass bottom line

The right second-pass interpretation is:

- local interpolation is only defensible inside exact neighborhood replay in `double_ma`
- `gpu_prescan` + explicit candidates in `composite_mo` changes the exactness claim to exact-on-pruned-space
- clustered `meta_select` output should be treated as region proposal, not exact point ranking
- once explicit candidates exist, `mo.rs` exact candidate-set search should dominate TPE

---

## Third-pass answer

This is an overlay-only third pass, because the core `PR-*` prompt is still not present. The third-pass obligation is implementation-facing: turn the new lemma, bound, and counterexamples into concrete changes **inside the current ownership boundaries**, without redesigning adjacent surfaces.

### 1. `double_ma.rs`: turn neighborhood summaries into an admission contract

The most valuable third-pass change is to promote the existing neighborhood summaries from explanation-only outputs into a bounded **local-surrogate admission gate**.

The file already gives the raw ingredients:

- exact score modes and shortlist modes (`double_ma.rs:193-218`)
- score-mode dispatch and the special-case pair-context requirement for `EffectiveSampleLcb10` (`double_ma.rs:1334-1364`)
- exact neighborhood diagnostics `plateau_score`, `agreement_floor`, `roughness_stability`, and `curvature_stability` (`double_ma.rs:1903-2188`)
- exact local replay around the chosen winner (`double_ma.rs:7695-7725`)

The narrow implementation patch is:

1. Add a local helper inside `double_ma.rs`, something like `admit_local_surrogate(...) -> LocalSurrogateAdmission`.
2. Reject immediately when `score_mode_needs_pair_observation_context(score_mode)` is true, because `EffectiveSampleLcb10` is not metric-only (`double_ma.rs:1334-1364`).
3. Fit only a tiny surrogate on the exact replay neighborhood, such as a bilinear or quadratic patch in `(fast_len, slow_len)`.
4. Compute an empirical residual bound `eps_hat` on the already exact local points.
5. Compute an exact elite margin `gamma_hat` on that same neighborhood.
6. Admit the local surrogate only when `gamma_hat > 2 * eps_hat` and when the existing exact diagnostics clear fixed thresholds, for example high `plateau_score`, high `agreement_floor`, and high roughness/curvature stability.
7. Use the admitted surrogate only for **local ordering, tie-breaking, or early-stop replay inside that neighborhood**. Keep the final chosen row and final metric surface exact.

This is the code-level implementation of the local elite-preservation lemma. It does not require new ownership, a new optimizer, or a new API surface. It only changes how an already-owned exact neighborhood is interpreted.

The counterexample implication is equally concrete: if the local neighborhood is rough, curved, or low-agreement, the thin-ridge failure mode is live, so the surrogate must be disabled and the current exact replay left unchanged.

### 2. `double_ma.rs`: add union telemetry and a bounded instability response

`build_exact_extra_shortlist_rows(...)` constructs a per-head top-`k` union with deduplication by parameter key (`double_ma.rs:2437-2497`). The head-union bound says `|union| <= |heads| * k`, but the chimera counterexample says this union can still miss globally good rows.

The implementation-facing implication is not “replace the union.” It is:

- expose `head_count`, `per_head_top_k`, `union_size`, and `union_overlap_ratio`
- expose whether the winning row was present in multiple heads or only one head
- if overlap is very low, mark the shortlist as unstable
- optionally trigger one bounded exact follow-up that already belongs to this file: local replay around the current best row

That is enough to make the bound operational. A low-overlap union is exactly the case where the chimera risk is highest, so the file should surface that fact explicitly.

### 3. `composite_mo.rs`: keep all compression on the proposal side

The best third-pass rule for `CompositeMo` is simple: **all new compression should happen before `mo.rs` sees the candidate universe**.

Why this is already repo-fit:

- `gpu_prescan` seeds MA pairs, expands local windows, optionally adds predictive rescue pairs, and rewrites `fast_len`/`slow_len` into categorical candidate sets (`composite_mo.rs:1188-1578`)
- when the reduced candidate universe is small enough, `ExactGridPlan::ExplicitCandidates` is built and searched exactly (`composite_mo.rs:2805-2989`)
- search survivors are then re-evaluated with exact CPU `Trade22` metrics (`composite_mo.rs:3048-3132`)

So the implementation-facing action is:

- keep exact final scoring unchanged
- keep the exact-candidate path preferred whenever prescan made it feasible
- do **not** add a learned model between prescan and exact replay

The useful patch here is to strengthen the prescan-side coverage against the thin diagonal ridge counterexample. The smallest bounded change is to add a tiny **ridge rescue** around prescan seeds before domain rewrite. Today, rescue grows mostly axis-aligned windows around seeds (`composite_mo.rs:1423-1456`). A small fixed offset set such as

```text
(+1,+2), (-1,-2), (+1,+1), (-1,-1)
```

scaled by the period step, would cover common sloped ridges without changing `mo.rs`, without widening ownership, and without turning prescan into a generic optimizer.

The second implementation patch is telemetry. `CompositeMo` already records `candidate_pairs`, `pair_density`, `exact_grid_used`, and `exact_grid_total_combinations` (`composite_mo.rs:1516-1580`, `composite_mo.rs:3048-3132`). It should also record a simple **reduction factor**:

```text
original_valid_pairs / candidate_pairs
```

so that future tuning is driven by actual prescan compression quality.

### 4. `mo.rs`: keep exact candidate-set search as the reduced-space default

The implementation implication for `mo.rs` is mostly policy, not redesign.

`candidate_set_mo_search_with_stats(...)` is already exact on the supplied candidate set and already caches duplicates (`mo.rs:2409-2462`). TPE is already approximate because it truncates to `max_model_points` and ranks within that capped subset before fitting models (`mo.rs:1189-1236`, `mo.rs:1982-2077`).

So the third-pass change is simply to make the preference explicit in comments, tests, or call-site assertions:

- if the caller already has explicit candidates, search them exactly
- only use TPE when no exact reduced candidate universe exists yet

That preserves the ownership boundary perfectly. `mo.rs` remains the exact reduced-space engine and the approximate full-space fallback.

### 5. `meta_select.rs`: make clustered output behave like region proposals, not point claims

`meta_select.rs` already distinguishes train-stage `meta_score` from later `final_score`, already stores `representatives`, and already defaults to `Clustered` grouping (`meta_select.rs:123-190`, `meta_select.rs:240-330`, `meta_select.rs:680-775`).

The implementation-facing implication of the bucket-boundary counterexample is:

- when `grouping_mode == Clustered`, downstream handoff should use the stored `representatives`, not only the canonical `params`
- keep `meta_score` ownership in this file exactly as it is
- do not let clustered groups be interpreted as exact point winners before holdout or downstream replay

The smallest internal patch is to add one more evidence flag for **cluster heterogeneity**. This can be computed from information the file already owns:

- spread of `robust_score` values inside `train_rows`
- disagreement between the canonical median representative and the other stored representatives
- unusually wide within-group quantized span

If that spread is large, the group should still be allowed to rank, but it should be marked as region-like rather than point-like.

This patch stays entirely inside `meta_select.rs`. It does not change holdout ownership, scoring ownership, or adjacent workflow boundaries.

### 6. Tests that directly encode the new math

The third pass should end with very targeted tests rather than broad architecture work.

In `double_ma.rs`:

- a stable local neighborhood test where the local surrogate is admitted
- a rough or highly curved neighborhood test where it is rejected
- a test proving rejection for `EffectiveSampleLcb10`
- a union telemetry test asserting `union_size <= head_count * top_k`

In `composite_mo.rs`:

- a synthetic diagonal-ridge prescan test where ridge rescue improves coverage
- a regression test keeping `exact_grid_used == true` when the candidate set is small enough

In `meta_select.rs`:

- a clustered group with high within-group score spread that emits the heterogeneity flag
- a test ensuring clustered output still leaves `final_score == None` before holdout

In `mo.rs`:

- a narrow regression test confirming that explicit candidate search preserves `exact_grid_used == true`
- optionally, a comparison test showing that candidate-set exact search and TPE can disagree on the same reduced candidate set, reinforcing the policy boundary

### Third-pass bottom line

The third-pass answer is not “build a global surrogate layer.” It is:

- certify local compression only inside `double_ma`'s already exact local replay
- keep `CompositeMo` compression entirely on the prescan/proposal side
- keep `mo.rs` exact on explicit reduced candidate sets
- treat `meta_select` clustered groups as region proposals with representatives, not as exact point winners

That is the narrowest implementation path that honors the overlay prompt, preserves exact final scoring, and stays inside the current ownership surfaces.

---

## Compact overall conclusion

Across all three passes, the stable answer is:

1. The repo **does** have exploitable low-dimensional and lattice structure.
2. The right reduction is **proposal compression plus exact replay**, not global surrogate optimization.
3. The best new math is local and certifying:
   - a local elite-preservation lemma
   - a head-union size bound
   - counterexamples based on diagonal ridges, chimera shortlist misses, and bucket-boundary inversions
4. The implementation path is therefore small and local:
   - admission gates and telemetry in `double_ma.rs`
   - stronger prescan coverage in `composite_mo.rs`
   - exact candidate-set preference in `mo.rs`
   - region-proposal discipline in `meta_select.rs`

