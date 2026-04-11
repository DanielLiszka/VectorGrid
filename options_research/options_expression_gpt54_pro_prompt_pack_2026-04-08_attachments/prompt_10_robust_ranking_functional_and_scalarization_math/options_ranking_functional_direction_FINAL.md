# Options Ranking Functional Direction — Final

Date: 2026-04-11
Scope: Prompt 10 final consolidation for VectorGrid's current `options_expression` lane

## 1. Executive summary

### Adopted direction

Revise the current `robust_score` rather than replacing the ranking system.

The best current repo-fit change is a **lane-local coverage-adjusted weighted expectancy scalar**:

```text
c(m) = clamp(1.0 - m.invalid_rate, 0.0, 1.0)
e(m) = m.expectancy_per_premium

S = search
H = holdout.unwrap_or(S)
C = stress.unwrap_or(S)
U = harsher_stress.unwrap_or(C)
X = extreme_stress.unwrap_or(U)

robust_score_v2 =
    0.45 * c(S) * e(S)
  + 0.25 * c(H) * e(H)
  + 0.15 * c(C) * e(C)
  + 0.10 * c(U) * e(U)
  + 0.05 * c(X) * e(X)
```

### Why this wins now

- It fixes a real inconsistency in shipped code: current ranking uses **lane-specific expectancies** but only **search-lane coverage**.
- It preserves the current scalar, GPU-friendly, artifact-driven design.
- It requires **no new aggregate state**, **no schema widening**, and **no extra replay-heavy stage**.
- It keeps the current boundary that matters: **broad ranking** and **exactified reranking** stay scalar, while **sealed-finalist ordering** stays validation-centric and separate.

### High-level decision

- **Implement now:** lane-local coverage-adjusted revision of `robust_score`
- **Defer:** shrinkage/confidence/dispersion-aware ranking until Prompt 9-style extra state exists
- **Reject:** Pareto, dominance-heavy, or sealed-in-first-pass ranking changes

---

## 2. Current ranking math

### Current metric surface

`OptionsExpressionMetrics` currently exposes:

- `valid_trades`
- `invalid_events`
- `invalid_rate`
- `expectancy_per_trade`
- `expectancy_per_premium`
- `median_return_on_premium`
- `win_rate`
- `total_pnl_per_contract`

`OptionsExpressionTupleResult` already separates:

- search / selection metrics
- holdout metrics
- sealed evaluation metrics
- stress metrics
- sealed stress metrics
- harsher stress metrics
- sealed harsher stress metrics
- extreme stress metrics
- sealed extreme stress metrics
- `validation_verdict`
- `robust_score`

Validation verdict logic is already explicit and should stay so: rows can become `StressFragile` when the strongest stressed validation lane turns negative or drops by at least `0.03` in `expectancy_per_premium`.

### Current scalar

The shipped `robust_score` is:

```text
coverage = 1 - search.invalid_rate

search_score   = search.expectancy_per_premium
holdout_score  = holdout.expectancy_per_premium          or search_score
stress_score   = stress.expectancy_per_premium           or search_score
harsher_score  = harsher.expectancy_per_premium          or stress_score
extreme_score  = extreme.expectancy_per_premium          or harsher_score

robust_score =
    coverage * (
        0.45 * search_score
      + 0.25 * holdout_score
      + 0.15 * stress_score
      + 0.10 * harsher_score
      + 0.05 * extreme_score
    )
```

Sealed metrics are intentionally excluded from this first-pass tuple-ranking scalar.

### Current ranking flow

The repo currently uses three distinct ranking or ordering points:

1. **Broad pre-`top_k` ranking**
   - CPU path computes per-split metrics directly
   - GPU path reconstructs reduced lane metrics from `OptionsGpuTupleAggregate` via `metrics_from_aggregate(...)`
   - broad ranking uses `robust_score`

2. **Exactified reranking on retained tuples**
   - recomputes selection, holdout, stress, harsher stress, and extreme stress
   - recomputes `robust_score`
   - resorts the retained tuples

3. **Sealed-finalist ordering**
   - explicit sealed ordering by:
     - sealed evaluation metrics
     - sealed stress metrics
     - sealed harsher stress metrics
     - sealed extreme stress metrics
     - then `robust_score`
     - then `pre_sealed_rank`

This split is correct and should remain intentional.

### Current strengths

- Scalar and easy to reason about
- Cheap on CPU and GPU
- Compatible with the current compiled-tensor shape
- Already test-covered
- Keeps sealed data out of broad ranking
- Preserves a clear boundary between search ranking and finalist validation ordering

### Current weakness that actually matters

The main weakness is not “it is scalar.” The main weakness is that it is **asymmetrically scalarized**.

The score uses:

- lane-specific `expectancy_per_premium` for search, holdout, stress, harsher, and extreme lanes
- but only **search-lane** `invalid_rate` as the coverage penalty applied to the whole sum

That means current code implicitly treats holdout/stress lane expectancy as important evidence, while treating holdout/stress lane coverage as irrelevant evidence. That is the strongest inconsistency in the current math.

### Additional current limitation

The GPU aggregate path currently carries only:

- `valid_trades`
- `wins`
- `total_pnl`
- `total_premium`

So `metrics_from_aggregate(...)` necessarily reconstructs a reduced surface and zeros `median_return_on_premium`. That makes richer stability-aware replacements premature unless extra aggregate state is deliberately added first.

---

## 3. Final recommended functional

### The recommendation

Keep the current scalar family, keep the current weights, keep the current fallback ladder, and keep sealed metrics out of first-pass ranking.

Only change the score so that **each resolved lane contributes its own coverage-adjusted utility proxy**.

### Recommended implementation

Define a helper:

```text
lane_utility(m) = clamp(1.0 - m.invalid_rate, 0.0, 1.0) * m.expectancy_per_premium
```

Resolve lanes with the current fallback structure:

```text
S = search
H = holdout.unwrap_or(S)
C = stress.unwrap_or(S)
U = harsher_stress.unwrap_or(C)
X = extreme_stress.unwrap_or(U)
```

Then score with:

```text
robust_score_v2 =
    0.45 * lane_utility(S)
  + 0.25 * lane_utility(H)
  + 0.15 * lane_utility(C)
  + 0.10 * lane_utility(U)
  + 0.05 * lane_utility(X)
```

### Why this is the right revision

This should be understood as a **minimal scalar correction**, not a new theory of ranking.

It keeps all the current product assumptions:

- still scalar
- still weighted by current lane importance
- still fallback-friendly when some lanes are unavailable
- still cheap to compute from current metrics
- still compatible with GPU pre-`top_k` ranking and CPU exactification

But it removes the strongest asymmetry in the shipped formula.

### Where this functional should apply

Use the same scalar family at both of these stages:

- broad pre-`top_k` ranking
- exactified reranking on retained tuples

Do **not** use it to replace sealed-finalist ordering. Sealed-finalist ordering should remain the current explicit validation-centric hierarchy.

### What this is not

This is **not**:

- a new Pareto system
- a dominance-frontier system
- a shrinkage-aware confidence bound
- a dispersion-aware stability score
- a reason to pull sealed data into broad ranking

It is a strict revision of the current scalar, sized to the current repo.

---

## 4. Required inputs / state for that functional

### Inputs consumed

`robust_score_v2` consumes only fields that the repo already has per lane:

- `invalid_rate`
- `expectancy_per_premium`

That is enough for:

- CPU broad ranking
- GPU broad ranking after `metrics_from_aggregate(...)`
- exactified reranking

### State impact

No new state is required in:

- `OptionsExpressionMetrics`
- `OptionsGpuTupleAggregate`
- `TupleAggregateMetrics`
- saved artifact row schemas
- frontend result payloads

### Important implication

Because the broad GPU path already reconstructs both:

- `invalid_rate`
- `expectancy_per_premium`

from current aggregate state, the recommended functional can be adopted **without widening the GPU aggregate output**.

### What is still not available from current state

The repo does **not** currently have enough aggregate state for principled:

- shrinkage by variance
- lower-confidence bounds on expectancy
- downside volatility penalties
- quantile- or median-aware pre-`top_k` GPU ranking

That would require extra aggregate state such as second moments, downside moments, or bounded sketches. Until that exists, any confidence or dispersion penalty would be heuristic rather than strongly grounded.

### Practical consequence for `median_return_on_premium`

Keep `median_return_on_premium` out of broad ranking for now.

It can remain:

- exact on CPU exactification paths
- display-level information
- possible future ranking input only after a deliberate state-expansion decision

---

## 5. What remains unchanged

The following should remain unchanged in this prompt’s implementation scope.

### Base weighting policy

Keep the current lane weights:

- `0.45` search
- `0.25` holdout
- `0.15` stress
- `0.10` harsher stress
- `0.05` extreme stress

### Missing-lane fallback structure

Keep the current lane-resolution ladder:

- holdout falls back to search
- stress falls back to search
- harsher falls back to stress
- extreme falls back to harsher

The revision changes **how lane utility is formed**, not the fallback topology.

### Sealed-stage boundary

Keep sealed metrics out of broad tuple ranking.

Keep sealed-finalist ordering explicitly separate and validation-centric.

### Validation verdict logic

Do not change:

- `OptionsExpressionTupleResult::inferred_validation_verdict()`
- `OptionsExpressionTupleResult::is_stress_fragile()`
- `OPTIONS_VALIDATION_STRESS_DROP_THRESHOLD`

### GPU and CPU staging model

Keep the current hybrid structure:

- broad tuple ranking can happen on GPU
- retained tuple exactification stays CPU-backed where needed
- sealed-finalist evaluation remains separate
- CPU fallback/oracle behavior remains intact

### Contracts and payload shapes

No contract change is required for:

- Tauri commands
- artifact summary structures
- frontend tuple/result row shapes

The score semantics change, but the row shape does not.

---

## 6. Deferred alternatives

These ideas are still plausible, but they should be deferred rather than bundled into the current change.

### A. Adequacy-aware shrinkage by `valid_trades`

Example family:

```text
q(n) = n / (n + tau)
robust_shrink = sum_i w_i * q(n_i) * c_i * e_i
```

Why defer:

- introduces a tuning constant
- partly double-counts adequacy already reflected by `invalid_rate`
- still lacks dispersion information
- is harder to defend than the simpler lane-local coverage revision

### B. Dispersion-aware or confidence-bound ranking

Examples:

- lower-confidence-bound expectancy
- downside-risk penalties
- semi-variance penalties
- bounded quantile-sketch penalties

Why defer:

- current aggregate state is too thin
- needs Prompt 9-style state growth first
- otherwise becomes pseudo-rigorous rather than rigorous

### C. Tie-break redesign

A future follow-up could replace the current search-only secondary tiebreak with something like:

1. holdout expectancy when present
2. stress expectancy when present
3. selection expectancy
4. `valid_trades`

Why defer:

- it is not needed to land the core scalar fix
- it is a separate ranking-policy decision
- it should be evaluated only if `robust_score_v2` still leaves obvious search-biased ties

### D. Metric expansion beyond current state

Prompt 9 may still justify richer low-cost aggregate state later. That should be evaluated independently rather than smuggled into this scalarization change.

---

## 7. Rejected alternatives

### A. Keep the current `robust_score` unchanged

Rejected for now.

Reason:

- the asymmetry is real
- the fix is small
- current code already carries lane-local `invalid_rate`
- leaving the asymmetry in place is harder to justify than correcting it

### B. Pull sealed metrics into broad pre-`top_k` ranking

Rejected.

Reason:

- breaks the current search/validation boundary
- risks leakage of finalist-stage evidence into broad ranking
- contradicts the repo’s current explicit separation and tests

### C. Use the same scalar for sealed-finalist ordering

Rejected.

Reason:

- sealed ordering should stay explicitly validation-centric
- current code already encodes the right ordering policy
- replacing it with one scalar would reduce interpretability

### D. Dominance-aware or min-lane ranking

Examples:

- `min(u(S), u(H), u(C), u(U), u(X))`
- weighted Tchebycheff ranking
- skyline / non-dominated sorting

Rejected.

Reason:

- over-penalizes one weak lane
- adds conceptual complexity without product fit
- does not suit the current scalar GPU-first shortlist pipeline

### E. Confidence or dispersion penalties from current state alone

Rejected for now.

Reason:

- current state does not contain enough information
- any such penalty would be underidentified
- better to defer until the repo intentionally widens state

### F. Primary ranking by `median_return_on_premium`

Rejected for broad ranking now.

Reason:

- not available from current GPU aggregate state
- currently zeroed in `metrics_from_aggregate(...)`
- would bias broad ranking toward CPU-only or exactified contexts

---

## 8. Concrete code touch points by file and symbol

### `crates/app_core/src/options_expression.rs`

Primary implementation file.

#### Required changes

- `robust_score(...)` (around line `3283`)
  - replace the current search-coverage-times-weighted-expectancy formula
  - add a small helper such as:

```rust
fn lane_utility(metrics: &OptionsExpressionMetrics) -> f64
```

- `exactify_ranked_results(...)` (around line `2509`)
  - no structural rewrite required
  - keep the current call site shape, but it will now use the revised scalar

- `gpu_scoring::rank_compiled_tuples_gpu(...)` (around line `2664`)
  - keep the current aggregate reconstruction path
  - the broad GPU ranking call to `robust_score(...)` picks up the new semantics automatically

- broad GPU ranking call site (around line `2786`)
  - no state change; only scalar semantics change

- `rank_compiled_tuples_cpu(...)` (around line `2868`)
  - same semantic change on CPU broad ranking path

#### Optional later follow-up, not part of the core adoption

- `sort_ranked_results(...)` (around line `2357`)
  - current secondary tiebreak remains search expectancy
  - leave unchanged in this patch unless follow-up evidence justifies a tie-break change

#### Test touch points

- `robust_score_excludes_sealed_metrics_from_tuple_ranking()` (around line `5145`)
  - keep this invariant

- `sealed_final_rank_uses_sealed_metrics_ordering()` (around line `5194`)
  - keep this invariant

- add new tests for:
  - same lane expectancies, worse holdout `invalid_rate` -> lower score
  - same search lane, worse stress `invalid_rate` -> lower score
  - fallback ladder preserves expected equality when a missing lane falls back
  - exactified reranking still works with the new scalar

### `crates/app_core/src/options_types.rs`

Mostly unchanged.

- `OptionsExpressionMetrics` (around line `471`)
  - no schema change

- `OptionsExpressionTupleResult` and validation verdict helpers (around lines `495`, `543`, `564`)
  - no logic change required for this prompt’s scope

Optional documentation-only touch:

- clarify in comments that `robust_score` is ranking-only and separate from `validation_verdict`

### `crates/app_core/src/options_expression_gpu.rs`

No aggregate-state change should be made.

- `OptionsGpuTupleAggregate` (around line `28`)
  - keep unchanged

This is a key constraint of the adopted direction.

### `src-tauri/src/commands_options_expression.rs`

Core runtime/API shape can remain unchanged.

- `summarize_options_expression_artifact(...)` (around line `634`)
  - no schema change required
  - `best_robust_score` will automatically reflect new semantics for newly produced artifacts

Optional rollout hardening:

- if the team decides that old-vs-new artifact score comparability is too risky, this is the place to surface an explicit score-semantics label in artifact summary metadata

### `web/src/components/OptionsExpressionPanel.vue`

No required contract change.

Optional only:

- refresh any help text or tooltip that describes robust-score semantics if such explanation is surfaced later

---

## 9. Verification order

### Step 1 — unit math verification

Add or update targeted Rust tests first.

Required assertions:

1. identical expectancies, worse holdout `invalid_rate` lowers `robust_score_v2`
2. identical expectancies, worse stress `invalid_rate` lowers `robust_score_v2`
3. sealed metrics still do not affect first-pass tuple ranking
4. missing-lane fallback preserves expected equality when the fallback lane is identical
5. sealed-finalist rank ordering remains driven by sealed metrics rather than the broad scalar

### Step 2 — CPU/GPU parity on retained semantics

Run the same prepared workload through:

- CPU broad ranking
- GPU broad ranking

Confirm that:

- broad pre-`top_k` ordering matches up to expected float noise and stage differences
- exactified reranking still only differs for the expected reasons, especially the addition of exact extreme stress

### Step 3 — canonical provider-backed proof slices

Rerun the current bounded canonical workloads, at minimum:

- SPY DTE7
- QQQ DTE7

Evaluate:

- `top_k` composition changes
- holdout-supported share
- sealed-finalist quality
- stressed finalist quality
- share or count of `StressFragile` rows
- whether newly promoted tuples are actually better on sealed evaluation or merely lower-invalidity search rows

### Step 4 — artifact and compare-surface smoke

Validate that:

- new artifacts save and reopen correctly
- artifact summaries still render correctly
- compare surfaces are still interpretable
- any pre/post semantic drift in `robust_score` is visible or at least consciously accepted

### Step 5 — performance sanity

Because the adopted direction is state-neutral, the performance check is simple:

- no new GPU aggregate width
- no new GPU output buffers
- no new launches
- no meaningful ranking-time or preview-time regression beyond noise

If the repo treats this as scorer-adjacent behavior, capture before/after proof output under `output/options_expression_phase05`.

---

## 10. Regression risks

### A. Heuristic-risk, not model-risk, remains

`c(m) * e(m)` is still a heuristic utility proxy, not a fully principled expected-utility model. The revision fixes the asymmetry, but it does not eliminate the fact that coverage-weighted expectancy is still a heuristic.

### B. Stress-lane over-penalty risk

If stress-lane `invalid_rate` partly reflects stricter semantics rather than true economic fragility, lane-local coverage weighting can over-penalize stressed lanes.

### C. Ranking churn across old vs new artifacts

Saved artifacts produced before the change retain old `robust_score` values. Newly produced artifacts will carry new semantics. That can create compare-surface drift across the rollout boundary.

### D. Residual search bias via tie-break

If `sort_ranked_results(...)` keeps the current search-only tiebreak, some search bias remains in exact ties or near-ties.

### E. Small-sample fragility remains unsolved

This change does not solve:

- variance blindness
- sample adequacy shrinkage
- uncertainty-aware ranking

Those remain future work if the repo chooses to widen state.

### F. Broad vs exactified rerank churn may increase slightly

Because holdout/stress invalidity now matters more directly, retained tuples may reorder differently after exactification. That is acceptable only if the reordered set improves or at least does not worsen finalist validation quality.

---

## 11. Stop conditions

Do **not** ship the ranking change if any of the following occur.

### A. No validation-quality win on canonical slices

If SPY/QQQ DTE7 reruns show that `robust_score_v2` does not improve, or clearly worsens, finalist quality on:

- sealed evaluation expectancy
- sealed stressed expectancy
- `StressFragile` share/count
- or other current validation-first comparison cues

then stop.

### B. The change only games coverage

If the new score mainly promotes tuples with lower invalidity but not better holdout or sealed outcomes, stop.

### C. GPU/CPU retained-order disagreement becomes noisier without payoff

If broad GPU ranking and CPU exactified reranking disagree more materially than before, and that disagreement does not buy better finalist validation quality, stop.

### D. The implementation pressures state widening

If landing the change starts to require widening:

- `OptionsGpuTupleAggregate`
- saved artifact schemas
- Tauri payloads
- or other hot-path device/host state

stop and re-scope. That is outside the adopted direction.

### E. Artifact compare coherence becomes unacceptable

If the team is not willing to tolerate old/new `robust_score` semantic drift across saved artifacts, and also does not want to label or version the semantic change, stop and defer until rollout semantics are decided.

### F. The repo cannot defend the heuristic any better than the status quo

If additional targeted reruns show that lane-local coverage weighting behaves erratically across representative workloads, do not stack more heuristics on top. Defer further ranking changes until richer small-state statistics are added.

---

## Final decision

**Implement a minimal revision, not a replacement.**

Adopt lane-local coverage-adjusted weighted expectancy for broad and exactified ranking, keep the current weights and fallback ladder, keep sealed-finalist ordering separate, keep state and payload shapes unchanged, and defer shrinkage/confidence/dominance ideas until the repo intentionally widens its aggregate state.
