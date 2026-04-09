# VectorGrid Math-Direction Final Memo

Date: 2026-04-08  
Status: **adopt negative-result direction**  
Audience: repo-internal research triage

## Executive summary

The strongest math-forward contribution for this repository is no longer a broad positive theorem. After screening candidate direction families, attacking the leading positive path with repo-grounded counterexamples, and comparing that narrowed positive path against a limitation-style result, the best outcome is:

> **pursue a negative-result / limitation-result direction** showing that there is no credible **structure-free universal shortcut theorem** for the current VectorGrid exact-search class.

In plain terms: broad exact pruning, naive fidelity shortcuts, and any universal “beat exact brute force” theorem are not defensible on the current repo without adding explicit extra structure. The repo exposes a finite but heterogeneous exact-search family with route-dependent admissibility, strategy-family heterogeneity, exact-control restrictions, and stage-dependent validation behavior. That is strong enough to support a useful CS limitation theorem with design consequences, and stronger than forcing a weak positive theorem that collapses to config normalization or a tiny restricted family. [Repo-1][Repo-2][Repo-3][Repo-4][Repo-5][Repo-6][Repo-7][Repo-8][Repo-9][Repo-10][Repo-12]

---

## 1. Final adopted outcome

**Adopted outcome:** pursue a **negative-result direction**.

Why:

1. The initial positive leader was a correctness-style structure-extraction theorem for request/IR/route semantics.
2. A repo-grounded attack shows that broad route equality and broad control admissibility are broken by dynamic preview, backend rewriting, composite/hybrid route distinctions, and separate validation semantics.
3. The remaining positive theorem can still survive, but only as a **narrow correctness-only fallback**.
4. The negative-result path is broader, cleaner, and more honest about what the repo actually supports.

---

## 2. The screened direction families

This is the final ordering **after** the counterexample attack, not the optimistic first-pass ordering.

### 1) Negative-result / impossibility-style direction **(adopted)**
Best final fit. Strong repo grounding. Survives the counterexample attack.

### 2) Correctness-only compiler / IR result **(fallback)**
Still viable, but only after narrowing to conservative route/control soundness on a restricted current-snapshot lane.

### 3) Distortion-aware multi-fidelity screening **(gated follow-up)**
Mathematically plausible only if the `GpuVdps` ladder yields a stable audited distortion envelope.

### 4) Batch-delayed budgeted optimization theory **(deferred)**
Possible in abstraction, but weak repo-specificity and high reviewer risk.

### 5) Restricted certified elimination / exact pruning **(kill)**
Should be killed immediately as a theorem target.

---

## 3. Why the broad positive theorem path weakened

The broad structure-extraction theorem looked strongest at first because the repo already has:

- explicit request normalization and candle-field gating,
- composite route preview,
- exact-kernel-only control restrictions,
- dynamic strategy preview and backend rewriting,
- a real `GpuVdps` fidelity ladder,
- and a clean exact-grid baseline. [Repo-2][Repo-3][Repo-4][Repo-5][Repo-6][Repo-7][Repo-9]

But the attack phase found real breaks:

### 3.1 Dynamic route resolution is not purely static
`DynamicDoubleMa` is not just a parser-normalization problem. Capability text already describes it as a builder/preview surface that validates a `strategy_ir`, previews the resolved backend, and then runs through the normal optimizer path. Exact controls only follow exact-compatible `dynamic_strategy_v1` GPU-kernel shapes. [Repo-6]

### 3.2 Exact shortlist/score controls are route- and expansion-dependent
The backend requires exact-kernel routes for `extra_shortlist_objectives`, `shortlist_confidence_mode`, `shortlist_union_mode`, and `shortlist_slice_mode`, and rejects multi-slice expanded runs (`variant_pairs > 1`) on those lanes. [Repo-7]

### 3.3 Composite families do not share one single route semantics
Actual composite routing is strategy- and ADX-dependent:
- `CompositeTrend`: GPU kernel when ADX is off, hybrid when ADX is on.
- `BreakoutMomentum`: GPU kernel when ADX is off, hybrid when ADX is on.
- `RegimeHybrid`: hybrid in its current ADX-bearing form.
- CPU fallback remains part of the product contract. [Repo-2][Repo-4]

### 3.4 Validation is not the same semantics as optimization
Validation has its own backend-resolution logic and explicitly falls back to CPU when GPU validation runtime is not implemented for that case. A single theorem over “the route” across both optimization and validation is therefore false in the broad form. [Repo-8]

### 3.5 The cost/fidelity story is not theorem-ready
The repo does have `GpuVdps` telemetry, including top-\(k\) overlap and best-score regret against a stronger baseline, which is useful. But the broader research artifacts argue for measurement-first and explicitly avoid reopening code without more attribution; they do not supply a theorem-ready fidelity invariant. [Repo-7][Repo-10][Repo-11][Repo-12][Repo-13]

**Bottom line:** the positive theorem survives only after severe narrowing.

---

## 4. Exact formal target for the adopted direction

Let \(\mathcal V\) be the class of optimization instances generated by the current VectorGrid exact-search surface.

Each instance \(I \in \mathcal V\) contains:

- a finite feasible set \(X_I\), obtained after request normalization, range expansion, field gating, and route gating;
- an exact objective \(f_I : X_I \to \mathbb R\), where \(f_I(x)\) is the exact score induced by the repo’s exact evaluation path for candidate \(x\);
- optionally, a family of cheaper fidelities \(\{f_{I,\alpha}\}_{\alpha \in A_I}\), such as the existing runged `GpuVdps` surface;
- route/admissibility metadata \(r_I(x)\), encoding which backend/control lanes are actually admissible;
- optionally, stage-specific validation behavior \(v_I(x)\), which may differ from optimization semantics.

The assumptions on \(\mathcal V\) are intentionally weak:

- \(X_I\) is finite;
- strategy families may differ substantially;
- routes and admissibility may depend on strategy shape and preview/runtime state;
- no admissible global upper bounds are assumed;
- no global smoothness, convexity, Lipschitzness, or monotonicity is assumed;
- no distortion bound between fidelities and exact scores is assumed unless explicitly added.

### Proposed limitation theorem

> **No structure-free universal shortcut theorem for VectorGrid exact search.**  
> Over the class \(\mathcal V\), under the weak assumptions above:
>
> 1. **Safe elimination impossibility.** Any algorithm that eliminates at least one candidate without exact evaluation is not universally correct on \(\mathcal V\). There exists another indistinguishable instance in \(\mathcal V\) on which the eliminated candidate is the unique exact optimum.
>
> 2. **Naive fidelity shortcut impossibility.** Any algorithm that skips an exact evaluation by trusting a cheaper fidelity without an explicit distortion condition is not universally correct on \(\mathcal V\). There exists another instance in \(\mathcal V\) with the same low-fidelity observations but a different exact ranking.
>
> 3. **Universal improvement target is wrong.** Therefore, “beat exact brute force” is not the correct universal theorem target for this repo’s exact-search class. Exhaustive exact evaluation is the only structure-free universally correct baseline. Any positive theorem must add explicit structure.

This is the right formal target because the repo already presents itself as an **exact-grid optimization** system, and the baseline exhaustive search really is explicit in the optimizer crate. [Repo-1][Repo-9]

---

## 5. Assumptions: what survives and what is rejected

### 5.1 Repo-supported assumptions

These are supported directly by the repository and should be treated as green assumptions.

- The exact-search surface is finite and explicit.
- The product is organized around exact-grid optimization and validation, with `DoubleMa` as the mature GPU path.
- `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` route through the `DoubleMa` family rather than the old `CompositeCpu` route.
- Composite routing is strategy- and ADX-dependent.
- Candle-field gating is explicit in request shaping.
- Dynamic preview and route-specific control admissibility are explicit.
- Validation semantics are distinct enough to include their own backend fallback behavior.
- `GpuVdps` already reports overlap/regret telemetry rather than assuming fidelity agreement.
- The repo already has concrete evidence that a “score-first” exact-search variant can lose badly to the row-first exact route on clean exact surfaces. [Repo-1][Repo-2][Repo-3][Repo-4][Repo-5][Repo-6][Repo-7][Repo-8][Repo-10][Repo-12]

### 5.2 Plausible but unverified assumptions

These are usable only if the paper formalizes and tests them.

- The repo-induced counterexample family can be stated cleanly enough to look native rather than artificial.
- A restricted family of current instances can be turned into a clear indistinguishability construction.
- `GpuVdps` distortions may be stable enough on some audited subfamily to justify a later multi-fidelity theorem.
- A conservative route/control lattice might still be strong enough for a narrow correctness paper.

### 5.3 Rejected assumptions

These should be explicitly rejected as theorem inputs for the main paper.

- global Lipschitzness,
- global convexity or smoothness,
- broad monotonicity,
- universally admissible bounds for pruning,
- universal rank preservation across fidelities,
- claims whose substance depends on one specific CUDA kernel implementation.

Those assumptions are exactly what make positive pruning or optimistic-search theorems possible in the literature, and exactly what this repo does not support in general. [Lit-1][Lit-2][Lit-4][Lit-8]

---

## 6. Why this still counts as useful CS

This is only useful if it is written as a **boundary theorem with constructive consequences**, not as a generic restatement of No Free Lunch.

The useful CS claim is:

1. this repository induces a concrete class of **heterogeneous, route-dependent, exact black-box search problems**;
2. for that class, broad safe-pruning and broad fidelity-shortcut theorems fail unless explicit structure is added;
3. therefore, future positive work must come from one of two places:
   - a **restricted-family structural assumption** (for example admissible bounds or an audited distortion envelope), or
   - a **correctness-and-systems result** (for example a verified extractor / conservative route-control lattice).

That is more than “nothing works.” It is a precise explanation of **why the tempting universal target is wrong** and **what shape a valid theorem must take instead**. [Repo-3][Repo-5][Repo-6][Repo-7][Repo-8][Lit-1][Lit-6][Lit-7]

---

## 7. Minimum proof obligations

The proof burden for the adopted path is modest but real.

### 7.1 Core proof obligations

1. **Define the class \(\mathcal V\)**  
   It must clearly reflect the repo:
   - finite exact candidate sets,
   - route-dependent admissibility,
   - optional lower fidelities,
   - heterogeneous strategy families.

2. **Safe-elimination counterexample family**  
   Construct paired instances \(I, I' \in \mathcal V\) that are indistinguishable to any proposed structure-free elimination rule up to the point where the rule eliminates candidate \(x\), yet differ in whether \(x\) is optimal.

3. **Naive-fidelity counterexample family**  
   Construct paired instances with identical low-fidelity observations but different exact rankings.

4. **Corollary for universal theorem targets**  
   Show that without explicit additional structure, no universal shortcut theorem can dominate exhaustive exact evaluation in the sense of universal correctness.

### 7.2 What remains empirical only

The theorem itself does **not** need empirical validation to be logically correct.  
The empirical work is only there to show that the theorem’s class is **relevant to the repo**.

---

## 8. Minimum empirical obligations

The paper still needs a relevance package strong enough to prevent the reviewer reaction “this is just generic NFL.”

### 8.1 Heterogeneity audit
Demonstrate that the repo really spans distinct admissibility regimes:

- `CompositeTrend` route changes with ADX.
- `BreakoutMomentum` route changes with ADX.
- `RegimeHybrid` is hybrid-only in its current form.
- Candle-field requirements differ across strategy kinds.
- Dynamic preview and exact-control restrictions matter in practice. [Repo-2][Repo-4][Repo-5][Repo-6][Repo-7]

### 8.2 Validation divergence audit
Show that optimization and validation are not one shared semantic route. The explicit CPU fallback in validation is enough to motivate this. [Repo-8]

### 8.3 Fidelity-danger audit
Run a small, honest `GpuVdps` study across datasets / score modes / contracts and report:
- top-\(k\) overlap,
- best-score regret,
- baseline runtime,
- rung behavior.

The purpose is not to prove the theorem empirically; it is to show that fidelity agreement is treated in the repo as something to **measure**, not assume. [Repo-7]

### 8.4 Exact-baseline non-dominance example
Include one concrete example showing that “beat exact brute force” is already the wrong universal target on the repo’s own clean exact surfaces.

The strongest current evidence is the score-first branch losing to row-first on several exact surfaces, including shortlist-heavy `xlarge` cases. That does **not** prove row-first is always best, but it does directly refute the idea that a broad universal improvement claim is already available. [Repo-10][Repo-12]

### 8.5 Measurement-first posture
Preserve the repo’s own research discipline: do not reopen broad code work without stronger attribution. That posture strengthens the limitation framing because it shows the repo is already avoiding unjustified architecture claims. [Repo-10][Repo-11][Repo-12][Repo-13]

---

## 9. The kill test

Kill this direction if any of the following happens:

1. The theorem collapses into a generic No-Free-Lunch restatement that does not use a recognizably repo-grounded instance class.
2. The counterexample family cannot be stated in terms that clearly correspond to the repo’s actual search/admissibility surface.
3. The constructive corollary becomes too weak to inform the next positive direction.
4. The empirical relevance package cannot show real heterogeneity, real validation divergence, or real fidelity-risk in the current repo.

If that happens, close the math-heavy path rather than force the paper.

---

## 10. Implementation decision

**Do not reopen implementation now.**

This should be a **design-only / formalization-first** path.

What should happen next:

1. formalize the class \(\mathcal V\),
2. write the paired counterexample constructions,
3. assemble the minimal empirical relevance package,
4. only then decide whether a narrow positive follow-up should reopen implementation.

What should **not** reopen now:

- broad exact pruning,
- universal score-first search,
- universal fidelity screening,
- generic batch-bandit theory.

---

## 11. What was rejected and why

### 11.1 Broad exact pruning / certified elimination — **rejected**
No admissible bound family is repo-supported. No credible global smoothness or monotonicity structure exists across the shipped family. This direction depends too heavily on unrealistic assumptions. [Lit-2][Lit-8]

### 11.2 Broad distortion-aware multi-fidelity theorem — **deferred**
Potentially real, but only if the `GpuVdps` ladder yields a stable audited distortion envelope on a restricted family. Right now the repo supports telemetry, not a universal guarantee. [Repo-7][Lit-3][Lit-4]

### 11.3 Broad batch-delayed theory — **deferred**
The repo certainly has batch structure, but that alone does not yield a meaningful universal theorem. The closest literature assumes online stochastic/adversarial or GP-style structure that is not the default repo assumption. [Lit-3][Lit-5]

### 11.4 Broad compiler / IR semantics theorem — **rejected in broad form**
The positive theorem became too brittle under dynamic preview, route rewriting, exact-control restrictions, composite heterogeneity, and stage-specific validation semantics. It survives only as a narrow fallback.

---

## 12. The narrow positive fallback that still survives

If the team later wants a positive theorem, the only version that still looks publishable is:

> a **correctness-only compiler / IR result** for a restricted current-snapshot optimization lane, proving a **conservative route/control lattice** rather than exact final-route equality.

Concretely:

- optimization only, not validation;
- current-snapshot `DoubleMa` family only;
- actual candles or previewed runnable plan included in the semantics where needed;
- sound conservative classification of route/control admissibility;
- no cost theorem;
- no fidelity theorem;
- no broad cross-strategy completeness claim.

That is a legitimate fallback, but it is no longer the main recommendation. [Repo-5][Repo-6][Repo-7][Repo-8][Lit-6][Lit-7]

---

## 13. Repo-grounded evidence package behind the final decision

This is the practical evidence that pushed the memo toward the limitation result.

### 13.1 The repo is explicitly an exact-grid system
The README frames VectorGrid as a GPU-accelerated desktop app for **exact-grid** optimization and validation, and the optimizer crate includes a straightforward exhaustive `grid_search` baseline. [Repo-1][Repo-9]

### 13.2 Strategy families are heterogeneous
Composite strategies route through the `DoubleMa` family but differ by ADX use and profile kind, with CPU fallback still preserved. [Repo-2][Repo-3][Repo-4]

### 13.3 Request shaping already encodes route- and field-dependent feasibility
The request builder performs candle-field gating and exact-route-only control checks. [Repo-5]

### 13.4 Dynamic preview is real, not decorative
Capability text explicitly describes `DynamicDoubleMa` as previewing `strategy_ir` and making exact controls available only on exact-compatible dynamic GPU-kernel shapes. [Repo-6]

### 13.5 Validation semantics differ from optimization semantics
The validation path still contains explicit “GPU validation runtime not implemented yet; using CPU validation” fallbacks. [Repo-8]

### 13.6 The repo already has a concrete anti-universal example
The score-first branch lost to row-first on clean exact surfaces, including shortlist-heavy `xlarge` cases. That is strong evidence against chasing a universal “beat exact brute force” target before structure is formalized. [Repo-10][Repo-12]

### 13.7 The repo’s research process already favors measured limits over speculative branches
The replay-batching and descriptor-reuse sequence is exactly the kind of measurement-first discipline this paper should mirror. [Repo-11][Repo-12][Repo-13]

---

## 14. Final verdict

**Chosen direction:** negative-result / limitation-result  
**Exact formal target:** no structure-free universal shortcut theorem for VectorGrid exact search  
**Proof posture:** formalize repo-induced instance class + paired counterexample constructions  
**Empirical posture:** heterogeneity audit + validation divergence audit + fidelity-danger audit + exact-baseline non-dominance example  
**Implementation posture:** design-only for now

If the team wants one sentence to check into the repo:

> **The strongest math-forward path is a repo-grounded limitation theorem: without explicit added structure such as admissible bounds or audited distortion control, VectorGrid’s heterogeneous exact-search surface does not support a credible universal pruning, fidelity-shortcut, or “beat exact brute force” theorem.**

---

## References

### Repo references

- **[Repo-1]** `README.md` — product framing as GPU-accelerated exact-grid optimization and validation, current Double-MA / composite coverage.  
- **[Repo-2]** `AGENTS.md` — current route policy for `CompositeTrend`, `BreakoutMomentum`, `RegimeHybrid`, plus strategy-local architecture notes.  
- **[Repo-3]** `REPO_MAP.md` — routing doc describing Double-MA family ownership, `DynamicDoubleMa` caveat, and validation nuance.  
- **[Repo-4]** `crates/app_core/src/composite_profile_route.rs` — actual route mapping for composite profiles and ADX-driven GPU/hybrid behavior.  
- **[Repo-5]** `web/src/services/optimizeRequestBuilder.ts` — candle-field gating, composite route preview, exact-route-only control restrictions.  
- **[Repo-6]** `web/src/utils/strategyOptimizationCapabilities.ts` — capability text for `DynamicDoubleMa`, composite routes, and exact-compatible dynamic GPU-kernel controls.  
- **[Repo-7]** `crates/app_core/src/double_ma.rs` — exact-route restrictions for shortlist controls and score modes, `GpuVdps` telemetry structure, route/fallback handling.  
- **[Repo-8]** `crates/app_core/src/double_ma_validate.rs` — validation runtime route resolution and CPU fallback when GPU validation is unavailable.  
- **[Repo-9]** `crates/optimizer/src/grid.rs` — deterministic exhaustive exact-grid baseline.  
- **[Repo-10]** `double_crossover_phase2_external_review_findings_2026-04-03.md` — score-first losses, exact-route control constraints, and measurement-first recommendation.  
- **[Repo-11]** `double_crossover_phase2_replay_micro_attribution_2026-04-03.md` — replay bucket attribution after batching.  
- **[Repo-12]** `double_crossover_phase2_research_findings_2026-04-03.md` — methodology, retained wins, negative result on descriptor reuse, and current stop rule.  
- **[Repo-13]** `cupti_sass_metric_followup_findings_2026-04-05.md` — tooling/research value but no product-facing optimization priority change.

### Literature references

- **[Lit-1]** Wolpert, D. H., & Macready, W. G. *No Free Lunch Theorems for Optimization* (1997).  
  https://www.cs.ubc.ca/~hutter/earg/papers07/00585893.pdf

- **[Lit-2]** Jones, D. R., Perttunen, C. D., & Stuckman, B. E. *Lipschitzian Optimization Without the Lipschitz Constant* (DIRECT, 1993).  
  https://link.springer.com/content/pdf/10.1007/BF00941892.pdf

- **[Lit-3]** Li, L., Jamieson, K., DeSalvo, G., Rostamizadeh, A., & Talwalkar, A. *Hyperband: A Novel Bandit-Based Approach to Hyperparameter Optimization* (JMLR, 2018).  
  https://jmlr.org/papers/v18/16-558.html

- **[Lit-4]** Kandasamy, K., Dasarathy, G., Oliva, J. B., Schneider, J., & Póczos, B. *Gaussian Process Bandit Optimisation with Multi-fidelity Evaluations* (NeurIPS, 2016).  
  https://proceedings.neurips.cc/paper_files/paper/2016/file/605ff764c617d3cd28dbbdd72be8f9a2-Paper.pdf

- **[Lit-5]** Joulani, P., György, A., & Szepesvári, C. *Online Learning under Delayed Feedback* (ICML, 2013).  
  https://proceedings.mlr.press/v28/joulani13.html

- **[Lit-6]** The CompCert project — verified compilation and semantic preservation.  
  https://compcert.org/

- **[Lit-7]** Lopes, N. P., et al. *Alive2: Bounded Translation Validation for LLVM* (PLDI, 2021).  
  https://users.cs.utah.edu/~regehr/alive2-pldi21.pdf

- **[Lit-8]** Bubeck, S., Munos, R., Stoltz, G., & Szepesvári, C. *X-Armed Bandits* (JMLR, 2011).  
  https://www.jmlr.org/papers/volume12/bubeck11a/bubeck11a.pdf
