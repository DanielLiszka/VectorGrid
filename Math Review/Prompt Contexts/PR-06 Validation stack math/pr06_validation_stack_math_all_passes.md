# PR-06 Validation stack math — pass 1, pass 2, pass 3

This document bundles the three-pass mathematical analysis for `PR-06`, scoped to the uploaded prompt and the owned files.

---

## Governing scope

```md
## `PR-06` Validation stack math

Owned problem:
the full validation ranking and verdict stack, including single-candidate
statistics, reopt/regime summaries, and cross-ticker aggregation

Source packets:
`MR-08`, `MR-09`, `MR-10`, `MR-13`

Primary ownership files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`

Context files:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`

Own these symbols or decisions:

- robust-score and verdict contract
- PSR or DSR semantics
- bootstrap, CSCV/PBO, and superiority-test math
- surrogate slice-return assumptions
- reopt walk-forward summaries
- regime or bucket summaries
- cross-ticker aggregation and verdict rebuild math

Open problem:

- characterize a validation stack whose verdicts remain mathematically coherent
  under dependence, reruns, regime slicing, and cross-ticker aggregation
```

---

# First pass

## Core first-pass claim

There is no credible path to a single scalar sufficient statistic. The current stack is already a layered, search-relative, order-sensitive functional.

The practical ordering scalar is the repeatedly folded `robust_score`, but the run verdict is built by a different contract from hard-gate counts, approval counts, first-approved rank, top-candidate diagnostics, and optional global PBO and SPA. Reopt, regime, perturb, and bootstrap are currently auxiliary summary surfaces, not direct verdict gates.

So the first-pass mathematical target is not “find one final score.” It is:

- separate **gate-verdict state** from **auxiliary summary state**,
- characterize the exact layers that feed ranking and verdict,
- prove what can and cannot be sufficient.

## 1. Formal problem statement

Let `C` be the finite candidate set, `D0` the base dataset, and `D×` the optional cross-ticker rerun dataset family.

For each candidate `c in C`, the validation stack computes path-dependent evidence on `D0`, optional rerun evidence on `D×`, and then applies a layered map

`E(c; C, D0, D×) -> T(c) -> R(c) -> V(C)`

where:

- `T(c)` contains candidate-local and search-relative validation statistics,
- `R(c)` is the ranking score actually used to order candidates,
- `V(C)` is the run-level verdict built from the ordered candidate list plus global tests.

The mathematical problem is to describe the smallest retained state under which:

1. equivalent retained state implies identical candidate ordering and identical run verdict,
2. every approximation exposes a measurable instability contract,
3. family-specific regime and slice constructions remain semantically distinguishable,
4. cross-ticker and CSCV dependence are not silently treated as iid evidence.

## 2. Notation

Let

- `M*(c)` be the in-sample metrics actually used by the score,
- `W(c)` be walk-forward summary stats,
- `P(c; C)` be probability metrics `(psr, dsr, track_record_bars, minTRL)`,
- `C_loc(c; C)` be CSCV candidate-local outputs,
- `C_glob(C)` be CSCV global outputs `(pbo, lambdas, degradation, threshold hit rate)`,
- `X(c; D×)` be cross-ticker summary stats,
- `Aux(c)` be auxiliary summaries `(reopt, regime, perturb, bootstrap)`.

Define the base risk score:

`rho0(c) = rho_risk(M*(c), W(c))`.

Define the CSCV fold:

`rho1(c) = min(rho0(c), cscv_oos_metric_p50(c))` when the candidate-local CSCV metric exists and is finite, otherwise `rho1(c) = rho0(c)`.

Define the cross fold:

`rho2(c) = min(rho1(c), cross_score(c))` when the cross score exists and is finite, otherwise `rho2(c) = rho1(c)`.

The practical ranking score is `rho2(c)`.

Define the hard-gate pass indicator:

`G(c) = 1` iff the configured PSR, DSR, CSCV-rank, and CSCV-metric thresholds all pass under current missing-value semantics.

Define approval:

`A(c) = G(c) and track_record_bars(c) >= minTRL(c)`,

with the first-pass caveat that `minTRL` semantics would need to be made explicit.

Define the run verdict:

`V(C) = nu(order_by_rho2(C), {G(c), A(c)}, C_glob(C), SPA(C))`.

## 3. Fixed invariants and interface constraints

1. CSCV and cross-ticker folds are lower-envelope folds. They can only lower the running score, never raise it.
2. Run verdict is not a pure function of the scalar score.
3. DSR, CSCV rank statistics, and cross Pareto are search-relative, not candidate-local.
4. Probability gates live in `[0, 1]`.
5. Cross-ticker gating is multivariate, not reducible to cross score alone.
6. Preserved-finalists mode preserves base-run semantics for `candidates_tested`.
7. Reopt, regime, perturb, and bootstrap are auxiliary under the current verdict contract.
8. Regime support labels are shared, but regime partitions are family-specific.

## 4. Strongest plausible positive result

The strongest plausible positive result is a layered sufficiency theorem, not a scalar sufficiency theorem.

**Theorem candidate.**
Fix:

- the candidate set,
- the family tag for each candidate,
- a deterministic tie-break key,
- a fixed base dataset and fixed cross dataset family,
- exact module outputs for metrics, walk-forward, CSCV, SPA, and cross reruns,
- explicit missingness indicators.

Then ranking and run verdict are deterministic functions of retained candidate state plus retained global state.

This is the strongest plausible result because it mirrors the actual interfaces while refusing to collapse the stack into one score.

## 5. Strongest plausible impossibility result

No single scalar “final score,” and no candidate-local finite statistic independent of search-space context, can be sufficient for the current verdict stack.

Reasons:

1. `build_run_verdict` depends on more than score.
2. DSR, CSCV, and cross Pareto are peer-relative.
3. Tie order matters.
4. Regime summaries are not family-invariant.

So the strongest plausible impossibility result is:

- no scalar sufficient statistic,
- no candidate-local sufficient statistic,
- no family-agnostic regime sufficient statistic.

## 6. Candidate theorem or lemma list

1. Lower-envelope monotonicity lemma.
2. Fold Lipschitz lemma for `min`.
3. Threshold-margin stability lemma.
4. Peer-dependence lemma.
5. Tie-order necessity lemma.
6. CSCV conditional exactness lemma.
7. Regime-family non-equivalence lemma.
8. Cross-censoring non-identifiability lemma.

## 7. Explicit counterexample family

Take two candidates `a` and `b` with equal folded score, identical hard-gate pass, but different approval status or different top-candidate warning surfaces.

Because order among equal scores affects which candidate is “top,” two runs with the same unordered evidence multiset can produce different final recommendations.

So the verdict is not score-sufficient and not unordered-set-sufficient.

## 8. Lower-bound / identifiability angle

The sharpest obstruction is success-only cross-ticker aggregation.

If only successful reruns contribute to cross summary state, then from retained cross summary alone one cannot tell whether:

- the candidate succeeded everywhere, or
- the candidate succeeded only on a strict subset and failed elsewhere.

That is an identifiability failure, not a small-bias nuisance.

## 9. Unresolved questions that block implementation

1. What is the semantic denominator for cross-ticker evidence?
2. Is input order part of the contract?
3. Which CSCV slice builders are exact and which are reconstructed?
4. Do auxiliary summaries ever gate verdict?
5. What dependence class is assumed for SPA and bootstrap?
6. Which peer set defines rerun-time DSR, CSCV, and Pareto?
7. How, if at all, should family-specific regime semantics be normalized?

## First-pass bottom line

- Yes to a coherent layered state and margin-based stability theorem.
- No to a single scalar or candidate-local sufficient statistic.
- No to exactness claims for non-enumerated CSCV masks or success-only cross aggregation without an explicit approximation contract.
- No to a family-agnostic regime statistic without the family tag and partition rule.

---

# Second pass

## Core second-pass correction

The first-pass formalization was too strong. What survives is much narrower:

- there is no broad sufficient-statistics theorem for the full stack,
- there is no persistent repo object called `final_score`,
- the strongest surviving result is a **stage-indexed deterministic replay theorem** plus a few **conditional stability lemmas**.

The repo stores a single mutable `robust_score` channel that is overwritten in place:

1. base robust score from metrics and walk-forward,
2. CSCV fold via `min`,
3. cross fold via `min`.

The run verdict does not read that number directly. It reads:

- the ordered candidate list induced by it,
- candidate verdict objects,
- base-run global CSCV/PBO,
- base-run SPA.

## 1. Explicit failure modes

### F1. A canonical tie-break was smuggled into pass 1

The repo does not have a canonical tie-break key. Stable score-only sorting means equal-score candidates preserve input order. The verdict is therefore a functional of the input permutation.

### F2. Approval was misstated

Approval is not merely “hard-gate pass plus enough bars.”

It is:

`passed and minTRL exists and track_record_bars.max(2) >= minTRL`.

So `minTRL = None` is verdict-relevant, not merely missing metadata.

### F3. Top-candidate PSR and DSR are output fields, not independent verdict gates

They are reported and preserved, but their direct numerical values are not separately used to decide the final recommendation after the candidate verdicts already exist.

### F4. The first-pass base/final score distinction was mathematical notation, not retained repo state

The code stores only the current mutable `robust_score` channel.

### F5. CSCV missingness is asymmetric

If a candidate is selected in zero CSCV splits, `oos_metric_p50 = None`, so CSCV folding leaves its `robust_score` unchanged unless a hard CSCV gate is explicitly requested.

That means “no CSCV candidate-local evidence” is not automatically penalized.

### F6. CSCV exactness was overstated

Exactness holds only for the implemented CSCV functional, conditional on:

- the realized mask set,
- exact per-slice moments,
- the same return convention,
- the same rank and quantile convention.

### F7. Best-effort CSCV sampling has no useful deterministic small error from mask count alone

A nearest-rounded median is discontinuous. One omitted mask can move the candidate median by order `U - L` over a bounded range `[L, U]`.

### F8. Cross-ticker aggregation is not an approximation to full rerun robustness

It is success-only by design. Failed reruns leave no penalty record.

### F9. Cross Pareto is misaligned with the full cross gate

Pareto uses only `(sharpe_median, max_dd_worst)`, while the actual gate can also depend on dataset count, score, and positive-PnL fraction.

### F10. Preserved-finalists mode makes rerun evidence subset-relative

Rerun DSR, CSCV, and pass/fail can depend on the preserved finalist subset, not just on `(candidate, dataset)`.

### F11. Post-cross run verdict is hybrid

After cross filtering, the repo rebuilds the run verdict on the filtered candidate list but reuses base-run global CSCV/PBO, base-run SPA, and the original base-run `candidates_tested`.

### F12. Regime summaries are descriptive, family-specific, and currently verdict-extrinsic

They do not belong in a minimal verdict sufficiency theorem.

## 2. Adversarial constructions

### A1. Equal-score top-warning flip

Two equal-score candidates with different top-candidate warnings can flip the final recommendation depending on input order.

### A2. Zero-selection CSCV paradox

A candidate with no CSCV selections can keep a strong base score, while another candidate with many selections and poor OOS CSCV metric gets penalized. The first can outrank the second.

### A3. Pareto domination by an ineligible cross candidate

A candidate can be rejected as non-Pareto because another candidate dominates it on Pareto coordinates even if that dominating candidate fails the actual cross gate.

### A4. Preserved-finalists rerun dependence

The same candidate on the same cross dataset can pass in preserved-finalists mode and fail in full-pool mode because peer-relative statistics change.

### A5. One-mask median instability

One omitted mask can move the CSCV median by order `U - L`.

## 3. Corrected theorem or lemma candidates

1. Stage-indexed deterministic replay theorem.
2. No candidate-local sufficient statistic theorem.
3. Conditional folded-score stability lemma.
4. Threshold-margin lemma with missingness.
5. CSCV conditional exactness lemma.
6. Cross censoring impossibility lemma.
7. Family-tag necessity lemma for regime summaries.

## 4. Weakest assumptions under which anything survives

1. The theorem must be stage-indexed.
2. Order or tie-breaking input must be retained.
3. CSCV exactness must be conditional on the realized mask set.
4. Cross exactness must assume either all reruns succeeded or failures are explicitly retained.
5. Stability bounds must be stated on the actually consumed derived statistics.
6. Auxiliary surfaces must stay out of the minimal verdict theorem.

## 5. Exact vs approximate vs heuristic

### Exact implementation claims

- score folding by `min`,
- hard-gate logic,
- approval logic,
- run verdict rebuild from ordered candidates plus base-run globals,
- descriptive regime summaries within-family.

### Approximate claims that survive only with explicit contracts

- rank stability under direct folded-score error bounds,
- gate stability under direct threshold-margin bounds.

### Heuristic only

- inferential calibration of PSR/DSR under real dependence,
- stationary bootstrap calibration without a dependence model,
- full attempted-panel robustness from success-only cross summaries,
- family-agnostic regime robustness.

## 6. Revised costs

- final run verdict replay: `O(N)` time, `O(N)` state,
- ranking plus cross-filter replay: `O(ND + N^2 + N log N)` time, `O(ND)` exact state for full panel,
- CSCV exact replay from slice stats: `O(KNS)` time, combinatorial in full enumeration,
- auxiliary summaries do not strengthen the current verdict theorem.

## 7. Dependency burden

The burden on adjacent prompts is semantic:

- downstream consumers must not treat `robust_score` as a calibrated universal final score,
- they must distinguish base, CSCV-folded, and cross-folded stages,
- they must respect success-only cross and preserved-finalists mode as interface facts.

## Second-pass bottom line

The strongest surviving repo-fit result is:

1. exact deterministic replay only as a stage-indexed functional,
2. no candidate-local, family-agnostic, order-free sufficient statistic,
3. conditional stability only under direct stage-level error bounds,
4. cross exactness fails under current retained state,
5. regime evidence is family-specific and verdict-extrinsic.

---

# Third pass

## Strongest implementation-ready mathematical result

The strongest result that survives the second pass is a **Stage-Indexed Ordered Replay Contract** for `PR-06`.

This contract is implementation-ready because it tells us exactly what the stack is allowed to mean mathematically, what is exact, what is only heuristic, what state is minimally required for exact replay, and what claims must be killed during consolidation.

The contract has three layers:

1. **decision-critical exact replay layer**  
   ranking, gating, approval, cross filtering, and run-verdict rebuild,

2. **conditional exactness layer**  
   CSCV/PBO, PSR/DSR, and cross summaries only for the implemented functionals under explicit state conditions,

3. **descriptive auxiliary layer**  
   reopt, regime, perturb, and bootstrap summaries, which are owned surfaces but are not part of the current minimal verdict theorem.

The crucial prompt-specific correction is:

- there is no separate persistent repo object `final_score`;
- there is one mutable score channel, `robust_score`,
- it is overwritten by CSCV folding and then overwritten again by cross folding.

Mathematically, if one wants a “final score,” it is merely the current stage value of that channel, not an independent calibrated quantity.

## 1. Final formalization

Let `C` be the candidate set and let `Pi_t` denote the **ordered** candidate list entering stage `t`.

Let `D0` be the base dataset and `D× = {d1, ..., dJ}` the attempted cross-ticker dataset family.

For each candidate `c`, define the following stage-local objects.

### Stage 0: local path evidence

`L(c)` contains:

- in-sample metrics `M(c)`,
- optional iterative metrics `M_it(c)`,
- optional walk-forward summary `W(c)`,
- family tag `fam(c)`.

Define the base score channel:

`S0(c) = rho_base(c) := rho_risk(M_used(c), W(c))`

where `M_used(c)` is iterative metrics when present and otherwise in-sample metrics.

### Stage 1: probability metrics and candidate hard-gate verdict

Let

- `obs(c)` be the requested sample observation count with runtime fallback `max(requested_obs, 2)` inside PSR/DSR,
- `peer_mean(C)`, `peer_std(C)`, and `trials(C)` be the peer Sharpe moments used by DSR,
- `minTRL(c)` be the output of the implemented minimum-track-record formula.

Define

`P(c; C) = (psr(c), dsr(c), track_record_bars(c), minTRL(c))`.

Define the candidate hard-gate predicate `H(c)` as the output of the implemented candidate verdict object:

- PSR and DSR thresholds use `unwrap_or(0.0)`,
- CSCV rank and CSCV metric thresholds fail if CSCV is requested but unavailable,
- min-track-record shortfall contributes a **warning**, not a fail reason.

Thus

`H(c) = 1{ fail_reasons(c) is empty }`.

Define the approval predicate:

`A(c) = H(c) and minTRL(c) exists and track_record_bars(c).max(2) >= minTRL(c)`.

This is exact current behavior.

### Stage 2: CSCV/PBO layer

Let `M` denote the realized CSCV mask set after sanitization and possible best-effort sampling.

For each candidate, let

`Q(c; C, M) = (selected_frac(c), oos_rank_frac_p50(c), oos_metric_p50(c))`.

Let the global CSCV output be

`G_cscv(C, M) = (pbo, lambdas, degradation, threshold_hit_rate, skipped_reason, combinations)`.

Define the CSCV score fold:

`S1(c) = F_cscv(S0(c), Q(c))`

with exact repo semantics

- `S1(c) = min(S0(c), oos_metric_p50(c))` when `oos_metric_p50(c)` exists and is finite,
- otherwise `S1(c) = S0(c)`.

This is a lower-envelope fold, not a separate score family.

### Stage 3: cross-ticker success-only aggregation layer

For each attempted rerun dataset `dj` and candidate `c`, let `Y(c, dj)` denote the rerun outcome on that dataset.

Current orchestration retains only successful reruns in the cross accumulator. So define the retained success-only rerun relation

`R_succ(c) = { dj in D× : validation_passed(c, dj) = true }`.

From that retained success-only panel, define cross summary

`X(c; R_succ) = (datasets, sharpe_median, sharpe_worst, max_dd_median, max_dd_worst, pnl_median, pnl_worst, trades_median, trades_worst, pnl_positive_frac, cross_score)`,

where

`cross_score(c) = sharpe_median(c) - 2 * max_dd_worst(c)`.

Define the cross Pareto flag on the coordinates `(sharpe_median, max_dd_worst)` only.

Define the cross fold:

`S2(c) = F_cross(S1(c), X(c))`

with exact repo semantics

- `S2(c) = min(S1(c), cross_score(c))` when `cross_score(c)` exists and is finite,
- otherwise `S2(c) = S1(c)`.

Define the cross gate `G×(c)` by the configured cross gate fields:

- enabled,
- require_pareto,
- min_datasets,
- min_score,
- min_sharpe_median,
- min_pnl_positive_frac,
- optional max_dd_worst.

### Stage 4: run-verdict rebuild

Let `Pi_final` be the ordered candidate list after current-stage filtering and stable descending score sort.

Let `G_base` denote the base-run globals that are reused in the final verdict rebuild:

- `candidates_tested_base`,
- base-run `cscv_pbo`,
- base-run `spa`.

Then the final run verdict is the deterministic functional

`V_run = Nu(Pi_final, G_base, hard_gates)`.

Critically, after cross filtering the repo rebuilds the run verdict on the filtered candidate list but keeps:

- the original base-run `candidates_tested`,
- the base-run global CSCV/PBO object,
- the base-run SPA object.

So the post-cross run verdict is a hybrid functional, not a pure function of filtered-candidate evidence alone.

## 2. Final theorem and lemma list

### T1. Stage-indexed ordered replay theorem

Fix:

- the ordered stage-entry candidate list at every stable-sort stage,
- the exact candidate-local post-stage fields,
- the realized CSCV mask set,
- the cross-rerun candidate subset selection rule,
- the exact success-only cross accumulators,
- the reused base-run globals `(candidates_tested_base, cscv_pbo_base, spa_base)`.

Then every downstream stage output is deterministic, including:

- folded score channel,
- candidate hard-gate verdicts,
- approval set,
- cross-filter survivor set,
- final ordered candidate list,
- final run verdict.

This is the strongest exact theorem that survives.

### T2. No candidate-local sufficient statistic theorem

There is no candidate-local, family-agnostic, order-free sufficient statistic for the full current stack.

Necessary missing ingredients are:

- input order for tie preservation,
- peer-set context for DSR and CSCV ranks,
- peer-set context for cross Pareto,
- reused base-run globals in the post-cross verdict rebuild,
- family tag for regime semantics,
- missingness indicators such as `minTRL = None` and `oos_metric_p50 = None`.

### T3. Mutable score-channel lemma

The repo has one mutable score channel. There is no independent persistent `final_score`.

The channel evolves as

`S0 -> S1 -> S2`

through lower-envelope folds only.

Therefore any consumer that wants score provenance must treat stage identity as part of the mathematical object.

### T4. Approval-non-equivalence lemma

`passed` and `approved` are not equivalent.

A candidate can satisfy `passed = true` and still fail approval because:

- `minTRL = None`, or
- `track_record_bars.max(2) < minTRL`.

This is verdict-critical.

### T5. CSCV conditional exactness lemma

CSCV candidate and global outputs are exact for the implemented CSCV functional if and only if:

- every candidate has exactly the sanitized slice count,
- `slice_bars >= 2`,
- the realized mask set is fixed,
- slice summaries match the implemented return convention,
- tie handling and nearest-rounded quantile conventions are fixed.

If sampling is used, exactness is only conditional on the realized sampled mask set.

### T6. Cross success-only exactness lemma

Cross summary and cross fold are exact only for the **success-only** functional currently implemented.

They are not exact summaries of full attempted-dataset robustness unless every attempted rerun succeeded or failure evidence is explicitly retained.

### T7. Hybrid post-cross verdict lemma

The final post-cross run verdict is exactly a hybrid functional of:

- the filtered ordered candidate list,
- the original base-run `candidates_tested`,
- the base-run global CSCV/PBO object,
- the base-run SPA object.

So “same filtered list” is not sufficient for “same final run verdict.”

### T8. Family-tag necessity lemma for regime summaries

Regime summaries are exact descriptive outputs only relative to family-specific partition rules.

A regime summary without the family tag is semantically incomplete.

### T9. Margin-based rank stability lemma

If direct derived folded scores satisfy

`|S_t_tilde(c_i) - S_t(c_i)| <= eps_i`

and every relevant pairwise score gap satisfies

`S_t(c_i) - S_t(c_j) > eps_i + eps_j`,

then the descending order at stage `t` is preserved, provided no tie boundary is crossed.

### T10. Threshold-margin with existence lemma

A threshold-based gate is stable only if:

- the statistic exists in both exact and approximate states,
- the statistic-threshold margin exceeds the approximation error.

If the existence indicator itself can change, no gate-stability guarantee is valid.

## 3. Proof sketch / proof plan

### For T1

Proceed by structural composition.

1. Each stage is deterministic given its ordered inputs and explicit configuration.
2. Stable score sorts preserve prior order among equal keys, so stage-entry order is part of the sufficient replay state.
3. CSCV outputs are deterministic given exact slice summaries, realized masks, and convention choices.
4. Cross outputs are deterministic given the success-only rerun relation and cross gate configuration.
5. The final verdict builder is deterministic given the ordered candidate list and base-run globals.

Compose the stage maps.

### For T2

Use four separating families:

1. equal-score tie-order counterexample,
2. `passed` but not `approved` via `minTRL = None`,
3. same filtered candidate list but different base-run SPA,
4. same success-only cross summary but different attempted-panel failure structure.

Each shows that a candidate-local order-free statistic cannot be sufficient.

### For T3

Direct from the implementation contract:

- base risk score is written into `robust_score`,
- CSCV folds into the same field,
- cross folds into the same field.

No separate retained scalar survives.

### For T5

Show that CSCV is a deterministic functional of per-candidate per-slice summaries and the realized mask set. Exactness to the current implementation follows when all conventions match. If sampled masks differ, equality to full enumeration is not guaranteed.

### For T6

Construct two attempted rerun panels with the same successful reruns but different omitted failures. Since the retained success-only state is identical, any full-panel functional that depends on failures cannot be identified.

### For T9 and T10

Use monotonicity of stable sort under key perturbations and threshold comparison under margin conditions. The missingness clause is necessary because `None` and `Some(x)` are not small perturbations of each other.

## 4. Sufficient statistics, or proof they do not exist

## 4.1 What does not exist

The following do not exist under the current constraints:

1. a scalar sufficient statistic for the full stack,
2. a candidate-local sufficient statistic for the full stack,
3. an unordered multiset sufficient statistic for the full stack,
4. a family-agnostic regime sufficient statistic,
5. a filtered-list-only sufficient statistic for the final post-cross verdict.

## 4.2 Minimal exact replay state that does exist

A minimal exact replay state for the decision-critical layer is stage-indexed and ordered.

### Base decision state

For each candidate:

- family tag,
- current stage score channel,
- candidate verdict object `(passed, fail_reasons, warning_flags)`,
- `psr`, `dsr`,
- `track_record_bars`,
- `minTRL` existence and value,
- candidate-local CSCV outputs,
- current cross summary and cross Pareto flag,
- parameter identity key if order needs to be reconstructed across joins.

Global:

- hard-gate thresholds,
- base-run CSCV/PBO object,
- base-run SPA object,
- base-run `candidates_tested`,
- cross mode and preserved-finalists selection rule.

### Exact CSCV replay addendum

To replay CSCV exactly at the statistical level rather than merely replay stored outputs, retain:

- sanitized slice count,
- `slice_bars`,
- realized mask set,
- per-candidate per-slice summaries,
- the exact ranking and quantile conventions.

### Exact full-panel cross replay addendum

To replay a full attempted-dataset robustness functional, retain:

- attempted dataset count,
- per-candidate per-dataset attempted outcomes, including failures.

Current repo state does not retain this, so only the success-only functional is exact.

## 5. Exactness conditions and equivalence classes

## 5.1 Base-risk equivalence class

Two candidates are base-risk equivalent if they have the same metrics tuple used by `rho_risk`, the same walk-forward fallback resolutions, and the same missingness pattern.

Then their base score channel values agree exactly.

## 5.2 Probability-metric equivalence class

Two candidate sets are probability-equivalent if, for each candidate, they induce the same:

- candidate Sharpe,
- observation count after fallback,
- peer mean,
- peer standard deviation,
- trial count.

Then PSR, DSR, and minTRL agree exactly for the implemented formulas.

This is exact for the implementation, not a claim of external inferential calibration.

## 5.3 CSCV equivalence class

Two CSCV states are exactly equivalent if they agree on:

- sanitized slices,
- `slice_bars`,
- realized masks,
- per-candidate per-slice summaries,
- rank averaging convention,
- nearest-rounded quantile convention.

Then candidate-local CSCV medians and global CSCV/PBO outputs agree exactly.

## 5.4 Cross equivalence class

Two cross states are exactly equivalent for the current implementation if they agree on:

- the success-only retained rerun outcomes,
- the resulting cross summaries,
- the Pareto points,
- the gate configuration.

This does **not** imply equivalence for any full attempted-panel notion of robustness.

## 5.5 Final run-verdict equivalence class

Two final states are verdict-equivalent if they agree on:

- the final ordered candidate list,
- candidate verdict objects,
- candidate approval-critical fields,
- base-run CSCV/PBO object,
- base-run SPA object,
- base-run `candidates_tested`.

Then `build_run_verdict` returns exactly the same output.

## 6. Error, regret, or bias bounds for every non-exact claim

This section is the explicit approximation contract.

## 6.1 Rank-instability contract

For any stage using derived folded scores `S_t(c)`:

If an approximation supplies direct score bounds

`|S_t_tilde(c_i) - S_t(c_i)| <= eps_i`,

then the relative order of `c_i` above `c_j` is guaranteed to be preserved whenever

`S_t(c_i) - S_t(c_j) > eps_i + eps_j`.

If the inequality fails, rank flips are admissible.

If ties are present in the exact state or can be created by approximation, the current stack is order-sensitive and no order guarantee is valid without retaining prior order.

## 6.2 Gate-stability contract

For any thresholded statistic `z(c)` with threshold `tau`:

If the statistic exists in both exact and approximate states and

`|z_tilde(c) - z(c)| <= eps(c)`,

then the gate outcome is preserved whenever

`|z(c) - tau| > eps(c)`.

If the existence indicator can differ between exact and approximate states, no stability guarantee is valid.

This matters for:

- PSR,
- DSR,
- CSCV rank median,
- CSCV metric median,
- `minTRL` existence.

## 6.3 CSCV sampling contract

There is no deterministic small-error guarantee from mask count alone.

Assume a bounded candidate-local CSCV OOS metric range `[L, U]`. Then, from omitted-mask information alone, the only universal deterministic bound is

`|Delta oos_metric_p50| <= U - L`.

Likewise, since CSCV OOS rank fraction lives in `[0, 1]`,

`|Delta oos_rank_frac_p50| <= 1`.

These bounds are sharp in the finite-grid nearest-rounded-median setting.

Therefore the only strong exact contract is:

- exact conditional replay on the realized mask set.

Anything stronger is invalid without an actual sampling-theoretic bound.

## 6.4 Zero-selection CSCV contract

If `selected_frac = 0`, the candidate-local CSCV medians are `None`.

Current exact semantics are:

- no candidate-local CSCV fold is applied,
- any requested CSCV hard gate can still fail because missing candidate-local CSCV fields are treated as non-passing.

There is no approximation here. This is an exact behavioral branch.

## 6.5 Cross success-only bias contract

Relative to any full attempted-panel cross robustness functional, current cross aggregation is success-only and therefore censored.

If attempted count `A`, successful count `S`, and failure count `F = A - S` are known, and if the downstream metric lives in a bounded range `[L, U]`, then the universal deterministic bound on omitted-failure bias for any order-statistic-based scalar is only

`|bias| <= U - L`.

That includes medians and worsts once omitted failures can alter order statistics.

If `A` is not retained, the bias is not just large; it is unidentifiable.

For positive-PnL fraction, if the current success-only fraction is

`p_succ = pos_success / S`,

and all omitted failures could be non-positive, then the corresponding full attempted-panel fraction could be as low as

`p_full = pos_success / A`.

So the optimism can be as large as

`p_succ - p_full = pos_success * F / (S * A) <= F / A`.

If `A` is unknown, this optimism is unidentifiable from current retained state.

## 6.6 PSR/DSR contract

PSR and DSR are exact for the implemented formulas.

But their interpretation as dependence-robust inferential confidence is heuristic under the current owned surface because:

- the formulas use sanitized/fixed skewness and kurtosis inputs,
- DSR is peer-relative,
- no dependence-calibration theorem is carried by the current contract.

So the mathematical contract is:

- exact implementation output,
- no external calibration guarantee claimed.

## 6.7 Bootstrap and auxiliary-summary contract

Trade bootstrap, stationary bootstrap, reopt, and regime summaries are exact as descriptive outputs of their implemented procedures, conditional on their inputs and random seeds.

But under the current stack they do not participate in the minimal verdict theorem.

Any attempt to reinterpret them as calibrated uncertainty or direct verdict gates would require a new theorem and therefore fails the current contract.

## 7. Time, memory, and retained-state complexity

Let:

- `N0` = base-run candidate count,
- `N1` = candidate count entering cross reruns,
- `J` = attempted cross datasets,
- `S` = CSCV slice count,
- `K` = realized CSCV combination count.

## 7.1 Final verdict replay only

If the ordered filtered candidate list is already present, exact replay of the final run verdict costs:

- time: `O(N1)`,
- retained state: `O(N1)`.

## 7.2 Score-fold and hard-gate replay

If candidate-local stage fields are already present:

- CSCV fold plus stable sort: `O(N0 log N0)`,
- hard-gate pass construction: `O(N0)`.

## 7.3 Exact CSCV replay from stored slice summaries

If exact per-candidate per-slice summaries and the realized mask set are retained:

- time: `O(K * N0 * S)`,
- retained state: `O(N0 * S + K)`.

If full enumeration is used, `K = binom(S, S/2)` and the cost is combinatorial.

If sampled masks are used, storing only the seed is enough for deterministic replay of the current sampler, but storing the realized mask set is the stronger theorem-level state.

## 7.4 Current cross success-only summary replay

If success-only rerun outcomes have already been accumulated:

- summarization time: `O(sum_i S_i log S_i + N1^2 + N1 log N1)`,
- retained state: `O(sum_i S_i)` for raw successful outcomes or `O(N1)` for the compressed summary.

The `N1^2` term is from cross Pareto computation.

## 7.5 Exact full-panel cross robustness

If one wanted exact full attempted-panel robustness rather than the current success-only functional, retained state would need to scale like:

- `O(N1 * J)`.

That state is not currently retained.

## 7.6 Preserved-finalists mode

Under preserved-finalists cross mode:

- `N1 <= 4`.

This reduces cost sharply, but it also changes semantics because rerun-time peer-relative quantities become subset-relative.

## 8. File and symbol impact map

## 8.1 `validation_pipeline.rs`

Decision-critical owner of:

- `robust_score_from_metrics`,
- `compute_probability_metrics`,
- `fold_cscv_metric`,
- `fold_cscv_into_robust_and_sort`,
- `apply_candidate_hard_gates`,
- `build_run_verdict`,
- walk-forward, reopt, perturb, and bootstrap summarizers.

This file defines the core replay contract.

## 8.2 `sharpe_stats.rs`

Owner of:

- `probabilistic_sharpe_ratio`,
- `deflated_sharpe_ratio`,
- `minimum_track_record_length`.

This file defines the exact implemented probability-metric surface.

## 8.3 `validation_stats.rs`

Owner of:

- probability-gate sanitization,
- bounded PSR/DSR wrappers.

This file fixes the missing-value and threshold semantics for probability gates.

## 8.4 `cscv_pbo.rs`

Owner of:

- `SliceReturnStats`,
- `CscvCandidatePboStats`,
- `CscvPboResult`,
- realized-mask CSCV/PBO evaluation,
- nearest-rounded quantile and rank-tie statistical contract.

This file defines the CSCV exactness conditions.

## 8.5 `validation_common.rs`

Owner of:

- `push_metrics`,
- `finalize_cross_ticker_candidates`,
- `cross_ticker_passes_gate`,
- cross Pareto summary logic.

This file defines the current success-only cross functional and multivariate cross gate.

## 8.6 `commands_validation.rs`

Owner of:

- cross-ticker orchestration,
- success-only rerun accumulation,
- preserved-finalists selection rule,
- post-cross run-verdict rebuild.

This file defines the hybrid post-cross verdict contract.

## 8.7 `double_ma_validate.rs`

Owner of:

- double-MA candidate-local evidence builders,
- family-specific regime construction,
- double-MA CSCV and SPA slice builders,
- base-run invocation of `build_run_verdict`.

This file also defines the double-MA side of the family-specific regime semantics:
trend/range are split by the median of `spread_ratio = |fast - slow| / close`.

## 8.8 `composite_validate.rs`

Owner of:

- composite candidate-local evidence builders,
- composite CSCV and SPA slice builders,
- family-specific regime construction,
- base-run invocation of `build_run_verdict`.

This file defines the composite side of the regime split:
trend/range are split by the median of rolling trend-efficiency scores.

## 8.9 `validation_shared.rs`

Owner of shared validation types, summaries, and config surfaces that carry the contract across files.

## 9. Verification obligations and falsifiers

## 9.1 Order-sensitivity falsifier

Construct equal-score candidates with different top-candidate warning flags or different approval status.

If final recommendations can differ by input order, any claimed order-free sufficient statistic is false.

## 9.2 Approval falsifier

Construct a candidate with:

- `passed = true`,
- `minTRL = None`.

If any consumer marks that candidate as approved, it violates the current contract.

## 9.3 CSCV zero-selection falsifier

Construct a candidate with:

- `selected_frac = 0`,
- `oos_metric_p50 = None`.

If CSCV folding penalizes that candidate despite the current exact contract, the contract has been changed.

## 9.4 CSCV exactness falsifier

Compare two CSCV runs with different realized mask sets or different quantile/tie conventions.

If they are claimed to be exactly equivalent, the claim is false.

## 9.5 Cross identifiability falsifier

Construct two attempted rerun panels with identical successful reruns but different omitted failures.

If a theorem claims the retained success-only summary identifies full-panel robustness, it is false.

## 9.6 Hybrid verdict falsifier

Hold the filtered candidate list fixed and vary only:

- base-run SPA significance, or
- base-run CSCV/PBO.

If final run verdict changes, then any filtered-list-only theorem is false.

## 9.7 Family-regime falsifier

Construct equal-looking exported regime summaries from different families.

If a theorem treats them as semantically equivalent without family tag and partition rule, it is false.

## 10. Dependencies that must be reconciled during consolidation

These are interfaces, not permission to redesign adjacent prompts.

### Dependency 1: score provenance

Any downstream consumer must treat score provenance as part of the interface:

- base score channel,
- CSCV-folded score channel,
- cross-folded score channel.

Treating the final mutable `robust_score` as a calibrated universal score is invalid.

### Dependency 2: pass vs approval

Any downstream policy that consumes candidate status must distinguish:

- `passed` candidate,
- `approved` candidate.

These are not interchangeable.

### Dependency 3: cross mode provenance

Any downstream consumer must treat cross mode as part of the interface:

- lightweight cross,
- preserved-finalists cross.

The latter is subset-relative by construction.

### Dependency 4: CSCV provenance

Any downstream consumer must distinguish:

- full realized mask set,
- best-effort sampled mask set,
- skipped CSCV,
- zero-selection candidate-local CSCV.

### Dependency 5: family-tagged regime semantics

Any consumer of regime summaries must retain family provenance. Cross-family “trend/range robustness” is not currently a well-defined common statistic.

### Dependency 6: auxiliary-surface status

Reopt, regime, perturb, and bootstrap are currently descriptive auxiliary surfaces for the verdict stack. Any attempt to make them gating inputs would require a new theorem and a new approximation contract.

## 11. Kill criteria

The following claims or implementation assumptions must be rejected during consolidation.

1. **Kill scalar sufficiency.**  
   Any claim that one scalar score is sufficient for the full verdict stack.

2. **Kill candidate-local sufficiency.**  
   Any theorem or consumer that ignores peer-set dependence.

3. **Kill order-free replay.**  
   Any replay claim that ignores stable equal-score order.

4. **Kill `passed == approved`.**  
   Any consumer that conflates hard-gate pass with approval.

5. **Kill filtered-list-only final verdict.**  
   Any theorem that drops base-run global CSCV/PBO or SPA from the post-cross verdict.

6. **Kill exact full-panel cross robustness from success-only summaries.**  
   Without attempted-failure state, that claim is false.

7. **Kill small CSCV error guarantees from mask count alone.**  
   Nearest-rounded medians prevent such a theorem.

8. **Kill family-agnostic regime semantics.**  
   Without family tag and partition rule, the regime summary is incomplete.

9. **Kill rank-stability claims without tie and existence conditions.**  
   Missingness transitions and ties destroy naive continuity arguments.

10. **Kill calibrated-confidence claims for the mutable score channel.**  
    The current score is a lower envelope of heterogeneous stage summaries, not a standalone calibrated posterior.

## Third-pass bottom line

The strongest implementation-ready mathematical result for `PR-06` is:

- an **ordered, stage-indexed exact replay theorem** for the decision-critical stack,
- a **conditional exactness theorem** for CSCV and probability metrics only under explicit realized-state conditions,
- an explicit declaration that **cross-ticker aggregation is exact only for the current success-only functional**,
- an explicit partition between **decision-critical state** and **auxiliary descriptive state**,
- and a set of **kill criteria** that forbid stronger but false interpretations.

That is the strongest repo-fit mathematical result that survives the adversarial pass while staying inside the owned problem.
