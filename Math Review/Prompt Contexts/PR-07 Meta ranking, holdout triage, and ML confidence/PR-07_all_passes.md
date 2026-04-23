# PR-07 — Meta ranking, holdout triage, and ML confidence

Compiled markdown bundle of the three-pass analysis.

Prompt source followed: `PROMPT.md`. fileciteturn11file0

---

# First-pass answer

This is the first-pass answer only.

```md
## `PR-07` Meta ranking, holdout triage, and ML confidence

Owned problem:
how fixed validation outputs are turned into meta ranking, holdout admission, and
ML-confidence features or labels

Source packets:
`MR-11`, `MR-12`

Primary ownership files:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_campaign.rs`

Context files:

- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/validation_common.rs`

Own these symbols or decisions:

- meta-score weighting on fixed validation outputs
- shortlist admission and holdout triage heuristics
- final-score blending after holdout evidence exists
- ML-confidence feature use, target design, and calibration

Open problem:

- characterize how to combine fixed validation outputs into meta ranking,
  holdout triage, and confidence estimates under selection bias and repeated
  screening

Why this is hard here:

- meta selection acts on already filtered candidates rather than an unbiased
  sample
- holdout triage and final-score blending can create feedback between selection
  policy and observed evidence quality

Mathematical object:

- a staged selection functional that maps validation outputs into meta scores,
  holdout allocation decisions, and calibrated confidence targets

Notation seed:

- let `Y_i` denote the fixed validation-output vector for candidate `i`
- let `m_i = \mu(Y_i)` be the meta score
- let `h_i \in \{0,1\}` be the holdout-allocation decision
- let `q_i = \Pr(Z_i = 1 \mid \phi(Y_i))` be the calibrated confidence target
- the core question is whether there exists a stable staged rule
  `(m_i, h_i, q_i) = \Gamma(Y_i)` under the repo's selection bias

Primary questions:

- what combination rule is stable under selection bias and repeated filtering?
- what holdout triage rule is mathematically defensible under limited budget?
- which confidence features remain identifiable after upstream screening?

Required deliverables:

- formal staged-selection model
- candidate lemma set for bias-aware ranking or triage
- counterexamples showing failure of naive point-estimate weighting
- complexity and sample-efficiency consequences

Acceptance bar:

- every triage or blending proposal must state its bias assumptions explicitly

Verification hooks:

- workflow-regression parity
- holdout allocation sensitivity checks
- calibration drift falsifiers

Non-goals:

- do not redesign validation primitive definitions, verdict logic, or
  cross-ticker math; that belongs to `PR-06`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat `ta_workflow_regression.rs` as the workflow-baseline evidence surface
- if a proposed change requires new validation outputs, record that as a
  dependency on `PR-06`
```

The key first-pass conclusion was: the right estimand is **selection-conditional**, not unconditional. Train-stage `meta_select_from_artifacts` owns shortlist/grouping and `meta_score`, while later holdout logic optionally triages only some groups, adds holdout aggregates, rewrites `final_score`, and reorders groups. The default train score blends lower-tail robust score, median robust score, cross score, coverage, drawdown, and tail-gap, with hard gates on robust lower tail, coverage, and drawdown. The ML model uses a fixed seven-feature group summary and holdout-derived labels. So the mathematical object was not “estimate true quality over the raw search space,” but “define a stable staged rule over already-screened groups under explicit bias assumptions.” fileciteturn17file10 fileciteturn17file17 fileciteturn19file11

## First-pass conclusions in compact form

1. **Formal problem statement**  
   I modeled PR-07 as a staged functional \(\Gamma=(\mu,\eta,Q,B)\) mapping fixed validation outputs into train-stage score \(m_i\), holdout allocation \(h_i\), ML confidence \(q_i\), and post-holdout score \(F_i\).

2. **Notation**  
   I introduced the raw search space, grouped state space, empirical train/holdout measures, and the path functional from raw validation outputs to grouped states.

3. **Fixed invariants and interface constraints**  
   I treated train-stage ranking, holdout-stage augmentation, and the seven-feature ML surface as fixed interfaces. The train stage owns `meta_score`; holdout is later. The feature vector is fixed-width and includes `meta_score`. fileciteturn17file10 fileciteturn19file11

4. **Strongest plausible positive result**  
   I proposed a narrow positive theorem: under fixed grouping, fixed survival policy, exchangeability, and positive holdout support, there exists a stable **selection-conditional** staged rule over the already-screened population.

5. **Strongest plausible impossibility result**  
   I argued there can be no unconditional, policy-invariant exact rule without modeling upstream screening and holdout-admission propensities.

6. **Candidate theorem list**  
   I proposed:
   - a selection-conditional reduction lemma,
   - a summary sufficiency criterion,
   - a top-\(B\) exploit lemma,
   - a positivity lemma for calibration,
   - and a no-common-scale additive-blend theorem.

7. **Explicit counterexample family**  
   I gave a family where all groups have identical latent utility but selective holdout plus additive blending double-counts signal and structurally favors evaluated groups.

8. **Lower-bound / identifiability angle**  
   I argued that if holdout propensity \(\pi(x)=0\) on a feature region, then calibration is impossible there. “No exploration, no calibration off-support.”

9. **Unresolved blockers**  
   I flagged the need to pin down:
   - the exact ML target,
   - the exact holdout utility,
   - a common-scale final score,
   - exchangeability assumptions,
   - and the right dependence unit for dataset-pass labels.

The bottom-line first-pass claim was therefore: an exact theory exists only for a **selection-conditional staged problem**; the current quantile summaries, clustered grouping, selective holdout blending, and seven-feature ML map are exact only under explicit assumptions, and otherwise are heuristics or approximations. fileciteturn17file7

---

# Second-pass answer

Here is the second-pass answer.

The first-pass answer was too optimistic in one decisive way: it treated meta ranking as if each group had a mostly self-contained train state. That is false in the repo. Train-stage `pass_rate` is `datasets_covered / dataset_universe`, and `dataset_universe` is global over the whole screened meta run. So even before holdout, a group’s score is already a **globally coupled** functional, not a function of its own rows alone. Any exact theorem of the form \(m_i=\mu(T_i)\) is therefore not repo-faithful. The minimally correct object is global. fileciteturn17file11

## Second-pass explicit failure modes

1. **Failure of group-local sufficiency**  
   Because `pass_rate` uses the global denominator, adding an unrelated surviving artifact can change every group’s `meta_score` even when that group’s own rows are unchanged. fileciteturn17file11

2. **Failure of weak survival conditioning**  
   Conditioning only on “survival” is too weak. The screened population changes materially with `strict_shortlist` and `allow_shortlist_fallback`. Hard strict mode can exclude artifacts entirely; fallback can intentionally re-admit the broader set to avoid empty meta runs. fileciteturn18file5 fileciteturn18file6

3. **Failure of common-scale final ranking under selective holdout**  
   Post-holdout ordering is evaluated-first and only then score-based. Therefore admission itself changes rank, and final ordering is policy-dependent whenever only a strict subset is evaluated. fileciteturn20file2

4. **Failure of a fixed ML estimand under `Auto`**  
   `Auto` target selection is prefix-dependent. With gate-only history it resolves to holdout-gate; once dataset rows exist it resolves to holdout-dataset-pass. So `Auto` is not one estimand. fileciteturn19file4

5. **Failure of scope-only pooling exactness**  
   ML history is pooled only by normalized `(strategy_kind,timeframe)` scope. It does not condition on meta weights, grouping mode, shortlist/fallback, or missingness convention. So pooled `meta_score` values need not share one semantics. fileciteturn19file5 fileciteturn19file14

6. **Failure of seven-feature sufficiency via aliasing**  
   Missing `cross_score_median` is filled as `0.0`, so “missing” and “true zero” are aliased. Exact sufficiency fails unless one assumes those states have identical target behavior. fileciteturn14file17

7. **Failure of independence assumptions in holdout**  
   Holdout batching deduplicates representatives and fans one candidate out to multiple groups through `candidate_targets`, so holdout evidence is cross-group dependent. fileciteturn16file0 fileciteturn16file9

8. **Failure of naive cluster Lipschitz intuition**  
   Clustered grouping is floor-bucket quantization plus capped representatives. A simple bucket-radius continuity claim is not exact without within-cell oscillation and representative-coverage assumptions. fileciteturn17file14 fileciteturn18file8

## Second-pass corrected result

The strongest surviving repo-fit result was narrowed to:

- **exact**: a deterministic staged functional indexed by a **fixed policy bundle**;
- **exact**: train-stage ranking is a **global** functional of the full screened artifact bundle;
- **exact only when fully evaluated**: post-holdout scalar ranking;
- **exact only for fixed targets**: ML confidence with a frozen target family and compatibility class;
- **heuristic**: `Auto` targets, scope-only incompatible pooling, and partial-holdout common-scale ranking;
- **approximate**: clustered grouping, but only with an explicit instability contract.

The second-pass bottom line was therefore: the strongest honest theorem is about a **frozen policy-indexed deterministic workflow**, not an abstract unbiased experiment, and every clustered, selective-holdout, or pooled-history claim must carry an explicit bias or approximation contract. fileciteturn17file7

---

# Third-pass answer

Here is the third-pass answer.

I followed the prompt scope in `PROMPT.md`. fileciteturn11file0

```md
## `PR-07` Meta ranking, holdout triage, and ML confidence

Owned problem:
how fixed validation outputs are turned into meta ranking, holdout admission, and
ML-confidence features or labels

Source packets:
`MR-11`, `MR-12`

Primary ownership files:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_campaign.rs`

Context files:

- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/validation_common.rs`

Own these symbols or decisions:

- meta-score weighting on fixed validation outputs
- shortlist admission and holdout triage heuristics
- final-score blending after holdout evidence exists
- ML-confidence feature use, target design, and calibration

Open problem:

- characterize how to combine fixed validation outputs into meta ranking,
  holdout triage, and confidence estimates under selection bias and repeated
  screening

Why this is hard here:

- meta selection acts on already filtered candidates rather than an unbiased
  sample
- holdout triage and final-score blending can create feedback between selection
  policy and observed evidence quality

Mathematical object:

- a staged selection functional that maps validation outputs into meta scores,
  holdout allocation decisions, and calibrated confidence targets

Notation seed:

- let `Y_i` denote the fixed validation-output vector for candidate `i`
- let `m_i = \mu(Y_i)` be the meta score
- let `h_i \in \{0,1\}` be the holdout-allocation decision
- let `q_i = \Pr(Z_i = 1 \mid \phi(Y_i))` be the calibrated confidence target
- the core question is whether there exists a stable staged rule
  `(m_i, h_i, q_i) = \Gamma(Y_i)` under the repo's selection bias

Primary questions:

- what combination rule is stable under selection bias and repeated filtering?
- what holdout triage rule is mathematically defensible under limited budget?
- which confidence features remain identifiable after upstream screening?

Required deliverables:

- formal staged-selection model
- candidate lemma set for bias-aware ranking or triage
- counterexamples showing failure of naive point-estimate weighting
- complexity and sample-efficiency consequences

Acceptance bar:

- every triage or blending proposal must state its bias assumptions explicitly

Verification hooks:

- workflow-regression parity
- holdout allocation sensitivity checks
- calibration drift falsifiers

Non-goals:

- do not redesign validation primitive definitions, verdict logic, or
  cross-ticker math; that belongs to `PR-06`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat `ta_workflow_regression.rs` as the workflow-baseline evidence surface
- if a proposed change requires new validation outputs, record that as a
  dependency on `PR-06`
```

## 1. Final formalization

The strongest implementation-ready result is a **policy-indexed staged functional**, not a universal, group-local, policy-invariant one.

Let

\[
\Pi = (\Pi_{\mathrm{tr}}, \Pi_{\mathrm{ho}}, \Pi_{\mathrm{ml}})
\]

be a fixed policy bundle.

### 1.1 Train-stage policy bundle

\[
\Pi_{\mathrm{tr}} =
(\text{request match},\text{validation-verdict filter},\text{shortlist mode},
\text{fallback mode},\text{grouping mode},\text{cluster config},
\text{representative cap},\text{min\_samples},\text{top\_k},\text{weights}).
\]

This is not optional bookkeeping. `MetaSelectionRequest` explicitly carries `strict_shortlist`, `allow_shortlist_fallback`, `grouping_mode`, `cluster_config`, `cluster_representatives`, `min_samples`, `top_k`, and the full train and holdout weight vector. Train-stage ownership is also explicit: `meta_select_from_artifacts` owns shortlist, grouping, and `meta_score`; holdout augmentation is later. Defaults are clustered grouping, `min_samples=30`, `top_k=25`, `strict_shortlist=true`, `allow_shortlist_fallback=false`, and the default train/holdout weights are hard-coded in `MetaScoreWeights`. fileciteturn17file10 fileciteturn18file16 fileciteturn17file5

Let \(A\) be the full matched artifact bundle. Under \(\Pi_{\mathrm{tr}}\), let

- \(U(A,\Pi_{\mathrm{tr}})\) be the screened dataset universe, i.e. the set of dataset keys that survive request matching, verdict filtering, and shortlist admission,
- \(R_g^{\mathrm{tr}}(A,\Pi_{\mathrm{tr}})\) be the admitted multiset of train rows for group \(g\),
- \(\kappa_{\Pi_{\mathrm{tr}}}\) be the exact or clustered grouping map.

The current train score is

\[
m_g
=
w_{p10}\,q_{0.10}(\text{robust})
+
w_{50}\,q_{0.50}(\text{robust})
+
w_{\times}\,q_{0.50}(\text{cross})
+
w_{\mathrm{pass}}\,
\frac{\#\text{datasets covered by }g}{|U(A,\Pi_{\mathrm{tr}})|}
-
w_{\mathrm{dd}}\,q_{0.90}(\text{dd})
+
w_{\mathrm{psr}}\,q_{0.50}(\text{psr})
+
w_{\mathrm{dsr}}\,q_{0.50}(\text{dsr})
-
w_{\mathrm{gap}}\,\text{tail\_gap},
\]

with hard train gates

\[
q_{0.10}(\text{robust}) > 0,\qquad
\text{pass\_rate} \ge 0.15,\qquad
q_{0.90}(\text{dd}) \le 0.80.
\]

Those are exact repo facts. Crucially, the pass-rate denominator is the **global screened dataset universe**, not a group-local quantity. That is what makes the exact train-stage object global. fileciteturn17file11 fileciteturn17file3 fileciteturn17file17

### 1.2 Holdout-stage policy bundle

\[
\Pi_{\mathrm{ho}} =
(\text{holdout tag},\text{subset-selection rule},\text{holdout mode},
\text{validation config},\text{holdout score weights},\text{final ordering rule}).
\]

`commands_meta.rs` optionally selects only a subset of groups for holdout using `holdout_eval_top_n`, `holdout_eval_explore_n`, and `holdout_eval_rank_by`. When ranking is `"auto"` or `"ml"`, it tries to use a healthy ML-confidence model; otherwise it falls back to meta score. Evaluated groups are then assigned holdout aggregates and a `final_score`; the final sort puts evaluated groups ahead of unevaluated groups, then sorts by `final_score`, with `meta_score` as secondary tie-break. fileciteturn20file1 fileciteturn20file2

Let \(H \subseteq G\) be the active holdout subset chosen by the stage-two selector. Let \(E\) be the bipartite representative-target graph built by deduplicating representatives across groups and recording `candidate_targets`. One representative candidate can target multiple groups. Holdout observations are therefore not independent across groups. fileciteturn16file0 fileciteturn16file9 fileciteturn16file8

For each active group \(g \in H\), let \(R_g^{\mathrm{ho}}\) be the multiset of per-dataset holdout rows and let

\[
h_g = (\text{datasets},\text{pass\_rate},\text{score\_median},\text{score\_p10},
\text{sharpe\_median},\text{max\_dd\_p90},\text{pnl\_median})
\]

be the holdout aggregate. The repo’s post-holdout score rewrite is

\[
f_g
=
m_g
+
w_{\mathrm{ho,pass}}\cdot h_g.\text{pass\_rate}
+
w_{\mathrm{ho,p10}}\cdot h_g.\text{score\_p10}
-
w_{\mathrm{ho,dd}}\cdot h_g.\text{max\_dd\_p90}
\]

for evaluated groups, and \(f_g=m_g\) otherwise. Final ordering is then lexicographic in

\[
(\mathbf 1\{g \text{ evaluated}\},\ f_g,\ m_g).
\]

That lexicographic order is exact for the repo. It is not a common-scale latent ranking when only a strict subset is evaluated. fileciteturn16file18

### 1.3 ML-confidence policy bundle

\[
\Pi_{\mathrm{ml}}
=
(\text{scope},\text{target family},\text{label config},\text{feature schema},
\text{history compatibility class},\text{model family},\text{calibration rule}).
\]

The current group feature vector is exactly

\[
\phi(g) =
(\text{samples},\text{pass\_rate},\text{robust\_score\_p10},
\text{robust\_score\_median},\text{max\_dd\_p90},\text{cross\_score\_median},
\text{meta\_score}),
\]

a seven-feature map. `cross_score_median` is `#[serde(default)]` and extraction fills a missing value as `0.0`. Thus “missing cross score” and “true zero cross score” are aliased by the present schema. fileciteturn19file11 fileciteturn19file9 fileciteturn14file17

History pooling currently keys only on normalized `(strategy_kind,timeframe)` scope. `summarize_meta_artifacts_for_scope` and downstream ML preparation do not condition on meta weights, grouping mode, shortlist/fallback semantics, or the missingness convention beyond whatever is already implicit in the serialized groups. `train_ml_confidence_model_from_meta_artifacts` simply prepares rows from scope-matching meta artifacts and trains on the chosen target. `Auto` target resolution is prefix-dependent: with only gate-style history it resolves to `HoldoutGate`; once dataset holdout rows appear in the prefix it resolves to `HoldoutDatasetPass`. fileciteturn19file5 fileciteturn19file14 fileciteturn19file8 fileciteturn19file4

Therefore the exact ML object is not “one unconditional confidence number.” The strongest exact object is

\[
q_{\Pi_{\mathrm{ml}}}(x)
=
\Pr_{\Pi_{\mathrm{ml}}}(Z_T=1\mid \phi(g)=x),
\]

for a **fixed** target family \(T \in \{\text{HoldoutGate},\text{HoldoutDatasetPass}\}\), a fixed feature schema, and a fixed history compatibility class.

## 2. Final theorem and lemma list

### Theorem 1 — Exact policy-indexed workflow theorem

For every fixed policy bundle \(\Pi\), the repo induces a deterministic staged functional

\[
\Gamma_\Pi(A)
=
\bigl(
\{m_g\}_{g\in G_\Pi},
H_\Pi,
\{h_g\}_{g\in H_\Pi},
\{f_g\}_{g\in G_\Pi},
q_{\Pi_{\mathrm{ml}}}
\bigr)
\]

from the full artifact bundle \(A\) to train scores, selected holdout subset, holdout aggregates, final scores, and ML-confidence model outputs. This theorem is exact.

### Theorem 2 — No exact group-local sufficient statistic for train-stage ranking

Under the current train-stage score, there is no exact sufficient statistic \(S_g\) depending only on group \(g\)’s admitted rows such that \(m_g = F(S_g)\) for all compatible artifact bundles.

### Theorem 3 — Common-scale final ranking exists only in the all-evaluated regime

If holdout evaluation is run for all groups, the post-holdout scalar \(f_g\) is an exact deterministic ranking score under fixed \(\Pi_{\mathrm{ho}}\). If holdout evaluation is run for a strict subset of groups, the repo’s final ordering is policy-indexed and is not an exact common-scale latent ranking over all groups.

### Theorem 4 — One-shot exploit-optimal triage theorem

Fix a target family \(T\), a feature schema, a compatibility class, and equal holdout cost per evaluated group. Let

\[
q_g = \Pr(Z_T=1\mid \phi(g),\Pi_{\mathrm{ml}})
\]

be the exact success probability for group \(g\). Then among all subsets \(H\) of size \(B\), the expected number of positives is maximized by choosing the top-\(B\) groups by \(q_g\).

### Corollary 4.1 — Surrogate triage regret bound

If a surrogate score \(\widehat q_g\) satisfies

\[
\sup_g |\widehat q_g - q_g| \le \varepsilon,
\]

and \(\widehat H_B\) is the top-\(B\) set under \(\widehat q\), then the exploit regret satisfies

\[
\sum_{g\in H_B^\star} q_g - \sum_{g\in \widehat H_B} q_g \le 2B\varepsilon.
\]

Any pairwise inversion requires a true probability gap at most \(2\varepsilon\).

### Theorem 5 — Exact ML-confidence theorem for fixed target families only

For fixed scope, fixed target family \(T\), fixed feature schema, fixed label config, and fixed history compatibility class \(C\), the ML pipeline defines an exact training map from prepared rows to a model summary and a calibrated predictor. `Auto` is not an exact target family; it is a target-selection heuristic over prefixes.

### Proposition 5.1 — Scope-only history pooling is exact only under compatibility collapse

If scope-matching history artifacts are pooled from multiple latent compatibility classes \(c\in C\), then the learned target is the mixture

\[
q_{\mathrm{mix}}(x)=\sum_{c\in C}\alpha_c(x)\,q_c(x).
\]

It equals the desired class-specific target \(q_{c^\star}(x)\) exactly iff either all active classes have the same conditional target at \(x\), or the mixture puts all mass on \(c^\star\).

### Theorem 6 — Missingness aliasing lower bound

Suppose two latent states \(s=0\) and \(s=\varnothing\) share the same observed seven-feature vector because missing `cross_score_median` is imputed as \(0.0\), but have different conditional label means \(\mu_0(x)\neq \mu_\varnothing(x)\). If the conditional mixing weights are \(\lambda\) and \(1-\lambda\), then any predictor using only the current observed feature vector has minimum conditional Brier loss

\[
\lambda(1-\lambda)\bigl(\mu_0(x)-\mu_\varnothing(x)\bigr)^2.
\]

Hence exact zero-bias prediction is impossible unless the two latent states have identical label means.

### Theorem 7 — Clustered-mode approximation theorem

Let \(u(x)\) denote the exact per-candidate utility under exact grouping. For a clustered cell \(C\), define

\[
\omega_{\mathrm{cell}}(C)=\sup_{x,x'\in C}|u(x)-u(x')|
\]

and let \(\varepsilon_{\mathrm{rep}}(C)\) be the approximation error from representing the cell with the stored representative subset. Then any cluster-level surrogate \(\widetilde u(C)\) obeys

\[
|\widetilde u(C)-u(x)| \le \omega_{\mathrm{cell}}(C)+\varepsilon_{\mathrm{rep}}(C)
\qquad \forall x\in C.
\]

Consequently, pairwise rank inversion between exact groups can occur only when the true gap is at most

\[
2\bigl(\omega_{\mathrm{cell}}+\varepsilon_{\mathrm{rep}}\bigr).
\]

This is the strongest non-exact clustered guarantee that survives the second pass.

## 3. Proof sketch / proof plan

**Theorem 1.** Determinism follows directly from the owned files. `meta_select_from_artifacts` is a deterministic scan/aggregate/sort over the full artifact bundle; it computes grouped summaries, gates, `meta_score`, warnings, and top-k truncation. `commands_meta.rs` then deterministically selects a holdout subset, builds representative-target batches, computes holdout aggregates, rewrites `final_score`, and resorts the groups. `ml_confidence.rs` deterministically prepares rows, resolves the target family, trains the chosen model family, optionally calibrates, and serializes the resulting model metadata. fileciteturn17file17 fileciteturn20file1 fileciteturn16file18 fileciteturn19file8

**Theorem 2.** The train-stage contradiction is immediate from the score definition: `pass_rate = datasets_covered / dataset_universe`, and `dataset_universe` is global. Keep \(R_g^{\mathrm{tr}}\) fixed and add an unrelated surviving artifact from a new dataset outside group \(g\); then `datasets_covered` is unchanged but `dataset_universe` changes, hence `pass_rate` and `meta_score` change. So no exact group-local statistic can recover the score. fileciteturn17file11 fileciteturn17file3

**Theorem 3.** If all groups are evaluated, evaluated-first sorting becomes vacuous and the final ranking is a scalar sort by \(f_g\) with deterministic tie-break. If only a strict subset is evaluated, the comparator uses evaluation status before score. Then there exist \(a,b\) with \(a\) evaluated, \(b\) unevaluated, and \(f_a < m_b\), yet \(a\) ranks above \(b\). Therefore no policy-invariant common-scale latent scalar can represent the produced order. fileciteturn20file2

**Theorem 4.** Under equal cost and one-shot binary reward, expected yield is \(\sum_{g\in H} q_g\). The maximizing subset of size \(B\) is the top-\(B\) set. This is a standard exchange argument: if \(i\notin H\), \(j\in H\), and \(q_i > q_j\), swapping \(j\) out for \(i\) increases expected yield.

**Corollary 4.1.** Let \(H_B^\star\) be the exact top-\(B\) set by \(q\) and \(\widehat H_B\) the top-\(B\) set by \(\widehat q\). For each selected element, the surrogate can overstate the chosen item by at most \(\varepsilon\) and understate the omitted optimal item by at most \(\varepsilon\), so total regret is bounded by \(2B\varepsilon\).

**Theorem 5 / Proposition 5.1.** `train_ml_confidence_model_from_meta_artifacts` and `train_ml_confidence_model_from_prepared_rows` are exact only after the target family is fixed. The prefix test shows `Auto` changes target semantics when the history prefix changes from gate-only to dataset-pass-inclusive. Scope-only pooling then implies that, unless all pooled artifacts share identical conditional target semantics at a given feature vector, the trained model learns a mixture target, not the desired class-specific one. fileciteturn19file8 fileciteturn19file4 fileciteturn19file5

**Theorem 6.** The current feature extractor maps both “missing cross score” and “true zero cross score” to the same observed \(x\). Conditional on that \(x\), the best possible single prediction is the conditional mean of the mixture. The minimum achievable Brier loss is the variance of the latent conditional means under the mixture, which is \(\lambda(1-\lambda)(\mu_0-\mu_\varnothing)^2\). fileciteturn14file17

**Theorem 7.** For any \(x\in C\),

\[
|\widetilde u(C)-u(x)|
\le
|\widetilde u(C)-u(C_{\mathrm{ref}})| + |u(C_{\mathrm{ref}})-u(x)|
\le
\varepsilon_{\mathrm{rep}}(C)+\omega_{\mathrm{cell}}(C),
\]

where \(C_{\mathrm{ref}}\) is any reference point in the cluster cell. The pairwise rank-instability corollary follows by applying the triangle inequality to both items in the pair. The code facts needed are: clustered mode quantizes parameters into canonical bucket keys; default buckets are integer 5 and float 0.25; and only a capped representative subset is retained. fileciteturn17file14 fileciteturn17file13 fileciteturn18file8

## 4. Sufficient statistics, or proof they do not exist

### 4.1 Group-local sufficient statistics do not exist for train-stage exactness

Under the current score, there is no exact group-local sufficient statistic. The proof is Theorem 2.

### 4.2 Minimal exact train-stage state

The minimal exact train-stage state is global:

\[
S_{\mathrm{tr}}
=
\Bigl(
U(A,\Pi_{\mathrm{tr}}),
\{R_g^{\mathrm{tr}}(A,\Pi_{\mathrm{tr}})\}_{g\in G},
\Pi_{\mathrm{tr}}
\Bigr).
\]

Equivalently, two artifact bundles \(A\) and \(A'\) are **train-equivalent** under \(\Pi_{\mathrm{tr}}\) if they induce the same screened dataset universe, the same admitted train-row multisets for every output group, and the same canonical grouping/tie-break keys. Train-equivalent bundles produce identical train-stage ranking outputs. This is the right exact sufficient statistic notion for PR-07.

### 4.3 Minimal exact holdout-stage state

The minimal exact holdout state is

\[
S_{\mathrm{ho}}
=
\Bigl(
H,\ E,\ \{R_g^{\mathrm{ho}}\}_{g\in H},\ \Pi_{\mathrm{ho}}
\Bigr),
\]

where \(H\) is the active subset, \(E\) is the representative-target graph, and \(R_g^{\mathrm{ho}}\) is the per-group holdout-row multiset. This must be global because deduplicated representatives fan one holdout candidate out to several groups. fileciteturn16file0 fileciteturn16file9

### 4.4 Minimal exact ML-confidence state

For fixed target family \(T\) and fixed compatibility class \(C\), the minimal exact ML state is the prepared labeled row dataset

\[
S_{\mathrm{ml}} = (D_{T,C},\Pi_{\mathrm{ml}}),
\]

where \(D_{T,C}\) is the multiset of `(group_id, x, y)` rows after scope filtering, target resolution, label filtering, and compatibility filtering. This is exact for a frozen target family. It is not exact for `Auto`. fileciteturn19file8 fileciteturn19file4

## 5. Exactness conditions and equivalence classes

The exact theorem survives only under the following conditions.

1. **Train-stage exactness**
   - fixed \(\Pi_{\mathrm{tr}}\),
   - exact grouping mode, or clustered mode treated only through its approximation contract,
   - fixed global screened dataset universe,
   - fixed shortlist/fallback semantics.  
   Exact grouping and clustered grouping are not equivalent; clustered mode quantizes parameters and can collapse distinct exact groups into one region. fileciteturn18file8 fileciteturn17file14

2. **Holdout-stage exactness**
   - fixed \(\Pi_{\mathrm{ho}}\),
   - exact representative decoding/batching graph,
   - common-scale final ranking only if all groups are evaluated or none are evaluated.  
   When only some groups are evaluated, the final order is exact only as a **policy-indexed order**, not as a latent scalar order. fileciteturn20file2

3. **ML-confidence exactness**
   - fixed normalized scope `(strategy_kind,timeframe)`,
   - fixed target family \(T\neq \text{Auto}\),
   - fixed seven-feature schema,
   - fixed missingness convention for `cross_score_median`,
   - fixed compatibility class \(C\),
   - fixed model-family/calibration configuration.  
   Under those conditions, `train_ml_confidence_model_from_meta_artifacts` and `predict_ml_confidence_for_group` define an exact map from prepared rows to model output. fileciteturn19file11 fileciteturn19file8

The relevant equivalence classes are:

- \(A \sim_{\mathrm{tr}} A'\): same train-stage exact state \(S_{\mathrm{tr}}\),
- \((A,B) \sim_{\mathrm{ho}} (A',B')\): same holdout-stage exact state \(S_{\mathrm{ho}}\),
- \(D \sim_{\mathrm{ml}} D'\): same prepared labeled rows under the same \(\Pi_{\mathrm{ml}}\),
- \(x \sim_{\Delta} x'\): same clustered quantization key under bucket config \(\Delta\). fileciteturn17file14

## 6. Error, regret, or bias bounds for every non-exact claim

### 6.1 Clustered grouping error contract

For clustered mode, define

\[
\Omega = \sup_C \omega_{\mathrm{cell}}(C),\qquad
E_{\mathrm{rep}} = \sup_C \varepsilon_{\mathrm{rep}}(C).
\]

Then

\[
|\widetilde u-u| \le \Omega + E_{\mathrm{rep}},
\]

and top-k instability can occur only at a boundary where the exact pairwise gap is at most

\[
2(\Omega + E_{\mathrm{rep}}).
\]

No stronger clustered exactness claim is admissible.

### 6.2 Surrogate triage regret contract

For any surrogate \(\widehat q\) used to rank holdout candidates under equal costs,

\[
\mathcal R_B(\widehat q)
=
\sum_{g\in H_B^\star} q_g - \sum_{g\in \widehat H_B} q_g
\le 2B\varepsilon
\quad\text{if}\quad
\sup_g |\widehat q_g-q_g| \le \varepsilon.
\]

This is the right non-exact contract for ML-based or heuristic ranking under a fixed exploit objective.

### 6.3 Compatibility-mixing bias contract

If scope-only pooling mixes latent compatibility classes \(c\in C\), then at feature vector \(x\),

\[
|q_{\mathrm{mix}}(x)-q_{c^\star}(x)|
\le
\sum_{c\in C}\alpha_c(x)\,|q_c(x)-q_{c^\star}(x)|.
\]

Therefore exactness requires either explicit compatibility filtering or a proof that the mixed classes are target-equivalent.

### 6.4 Missingness-aliasing irreducible error contract

If missing and true-zero cross-score states are aliased by the observed feature map, the minimum conditional Brier loss at aliased \(x\) is

\[
\lambda(1-\lambda)(\mu_0(x)-\mu_\varnothing(x))^2.
\]

This is an irreducible lower bound, not merely model misspecification.

### 6.5 Selective holdout ranking contract

There is **no** scalar approximation contract from partial-evaluation final order to a common latent ranking without extra assumptions beyond the repo. The only admissible statement is:

- exact as a policy-indexed lexicographic order,
- non-comparable as a common-scale latent score unless all groups are evaluated.

### 6.6 `Auto`-target contract

`Auto` has no exact fixed-target interpretation. The admissible contract is:

- exact only after resolving to a concrete target family on a concrete history prefix,
- otherwise heuristic. fileciteturn19file4

## 7. Time, memory, and retained-state complexity

### 7.1 Train-stage exact state

Retained exact state is

\[
O(N_{\mathrm{train\ rows}} + G + |U|),
\]

where \(N_{\mathrm{train\ rows}}\) is the number of admitted train rows, \(G\) the number of groups, and \(|U|\) the screened dataset universe. This is larger than \(O(G)\) precisely because pass-rate is globally coupled. fileciteturn17file11

Train-stage time is a single artifact scan plus per-group sorting/aggregation. A safe expression is

\[
O(N_{\mathrm{scan}} + \sum_g T_g),
\]

where \(T_g\) includes sorting representative params, sorting train rows, computing quantiles, and evaluating gates/score. The exact low-level cost of `quantile_in_place` need not be assumed here.

### 7.2 Holdout-stage exact state

Retained exact state is

\[
O(R + E + H),
\]

where

- \(R\) = number of active representatives,
- \(E\) = number of representative-to-group target edges,
- \(H\) = number of persisted holdout rows if `save_holdout_rows=true`.

This edge term is mathematically essential because one representative can update multiple groups. fileciteturn16file0 fileciteturn16file9 fileciteturn20file5

Holdout time is

\[
O(C_{\mathrm{validate}} + E + H),
\]

where \(C_{\mathrm{validate}}\) is the external validation cost of the selected holdout datasets and candidate batches. PR-07 owns the staging mathematics, not the validation-engine runtime.

### 7.3 ML-confidence state

Feature width is fixed at \(d=7\). Retained state is \(O(N_{\mathrm{rows}})\), where \(N_{\mathrm{rows}}\) is the number of prepared labeled rows. Under holdout-gate targets, \(N_{\mathrm{rows}}\) scales with labeled groups; under dataset-pass targets, it scales with valid per-dataset holdout rows and can be much larger. The tests show row multiplication on dataset-pass training, e.g. at least \(6\times 50\) rows in a small synthetic example. fileciteturn19file9 fileciteturn19file15

## 8. File and symbol impact map

### `crates/app_core/src/meta_select.rs`
Owned mathematical surface:
- `MetaScoreWeights`
- `MetaSelectionRequest`
- `MetaGroupingMode`
- `MetaClusterConfig`
- group-gate constants
- grouping key and quantization path
- shortlist semantics
- `meta_select_from_artifacts`
- `MetaCandidateAggregate` / `MetaSelectionResult` payload contract.  
This file owns the exact train-stage theorem and the clustered approximation contract. fileciteturn17file10 fileciteturn17file17 fileciteturn20file16

### `src-tauri/src/commands_meta.rs`
Owned mathematical surface:
- ML-based holdout subset selection
- `meta_candidate_group_features`
- history summarization for ML scope
- prepared holdout batch graph construction
- `apply_holdout_accumulators_to_result`
- `group_rank_score`
- `group_policy_eval_meta_score`.  
This file owns the selective-holdout theorem, the evaluated-first ordering fact, and the representative-target dependence graph. fileciteturn20file1 fileciteturn20file2 fileciteturn16file0

### `crates/app_core/src/ml_confidence.rs`
Owned mathematical surface:
- `MlConfidenceGroupFeatures`
- `FEATURE_NAMES`
- feature extraction semantics
- target-resolution semantics
- prepared-row training interface
- model-family and calibration semantics
- model metadata and health reporting.  
This file owns the fixed-target theorem, the compatibility-mixing bias contract, and the missingness-aliasing lower bound. fileciteturn19file11 fileciteturn19file9 fileciteturn19file8 fileciteturn19file17

### `src-tauri/src/commands_campaign.rs`
Owned interface surface:
- campaign train/holdout partitioning interface.  
The current campaign split is deterministic over dataset paths using FNV-1a hashing XORed with a seed tag, and `holdout_pct` is capped at 50%. For PR-07 this is an interface assumption, not a redesign target. fileciteturn20file8 fileciteturn20file18

## 9. Verification obligations and falsifiers

1. **Workflow-regression parity**  
   `ta_workflow_regression.rs` is the baseline evidence surface. Under exact grouping, fixed weights, no selective holdout, and fixed target family, any PR-07 consolidation must preserve top-group identity/order and stage-level workflow outputs within agreed tolerances. fileciteturn18file12

2. **Global-coupling falsifier**  
   Add an unrelated surviving dataset artifact that does not change group \(g\)’s own rows. If a proposed exact train theorem predicts unchanged `meta_score`, it is false, because `pass_rate` depends on the global screened dataset universe. fileciteturn17file11

3. **Selective-holdout falsifier**  
   Construct one evaluated group and one unevaluated group with \(f_{\mathrm{eval}} < m_{\mathrm{uneval}}\). If the evaluated group still ranks first, any common-scale ranking claim is false. The repo comparator does exactly this. fileciteturn20file2

4. **Compatibility-mixing falsifier**  
   Pool two scope-matching meta histories with different train-score semantics or grouping semantics. If the learned ML confidence differs materially when trained separately, scope-only exactness is false. Current code does not filter beyond scope. fileciteturn19file5 fileciteturn19file14

5. **Missingness falsifier**  
   Compare groups identical on observed seven features except that one has true zero cross score and the other has missing cross score. If realized labels differ, the seven-feature exactness claim is false. The extractor aliases them. fileciteturn14file17

6. **Cluster instability falsifier**  
   Run exact and clustered grouping on the same artifacts. If top-k drift exceeds the declared \(2(\Omega+E_{\mathrm{rep}})\) boundary, the clustered approximation contract is false. The exact-vs-clustered collapse test already demonstrates that clustered mode can merge distinct exact groups. fileciteturn18file8

7. **Selected-group parity obligation**  
   If selective holdout is enabled, selected groups must match the full-evaluation outputs they would have received under all-group evaluation. The existing selected-group parity test is exactly the right verification hook. fileciteturn16file10

8. **Calibration drift falsifier**  
   On fresh compatible meta artifacts, bucketed realized success rates must stay within tolerance of predicted confidence. If they do not, the model is stale or the compatibility class is too wide. Health metadata and stale-state handling already exist at the model-summary surface. fileciteturn19file17 fileciteturn19file2 fileciteturn19file0

## 10. Dependencies that must be reconciled during consolidation

1. **PR-06 provenance dependency**  
   Any claim about unconditional quality over the raw search space, or any exact correction for upstream search/selection bias, needs provenance not currently guaranteed by PR-07-owned surfaces. That remains a dependency on PR-06, not a PR-07 redesign.

2. **Compatibility-key dependency**  
   Exact ML confidence requires a compatibility class finer than normalized scope if train-score semantics, grouping mode, shortlist/fallback semantics, or missingness conventions differ across historical meta artifacts. If those semantics are not already recoverable from existing meta artifacts, exact ML-confidence claims must be downgraded to heuristic for pooled history.

3. **Missingness-contract dependency**  
   Exact seven-feature confidence requires either:
   - accepting the current convention `missing cross_score_median ≡ 0.0`, or
   - adding a missingness distinction inside the owned ML surface.  
   Without one of these, only the aliasing lower bound is exact.

4. **Cluster-bound dependency**  
   Any clustered-mode guarantee needs measured or assumed \(\omega_{\mathrm{cell}}\) and \(\varepsilon_{\mathrm{rep}}\). Without them, clustered mode remains heuristic.

5. **Exploration-objective dependency**  
   `holdout_eval_explore_n` can be justified exactly only for a separately stated support-preservation objective. Without an explicit future-learning objective, only the exploit slice has an exact optimality theorem.

## 11. Kill criteria

Kill the PR-07 mathematical consolidation if any of the following is demanded as an exact theorem:

1. **Universal group-local exactness**  
   Impossible under the current `pass_rate = datasets_covered / dataset_universe` design.

2. **Common-scale final ranking under partial holdout**  
   Impossible under the current evaluated-first comparator.

3. **Exact `Auto` target semantics**  
   Impossible because `Auto` is history-prefix-dependent.

4. **Exact scope-only history pooling**  
   Impossible unless compatibility collapse is proved.

5. **Exact clustered-mode equivalence without a bound**  
   Impossible without an explicit \(\omega_{\mathrm{cell}}+\varepsilon_{\mathrm{rep}}\) contract.

6. **Exact unconditional search-space quality**  
   Impossible without PR-06 provenance on upstream screening and multiplicity.

7. **Optimality of exploration under the one-shot exploit objective**  
   False unless exploration is assigned a different objective than immediate positive yield.

## Bottom line

The strongest implementation-ready mathematical result for `PR-07` is:

- **exact**: a fixed, deterministic, policy-indexed staged workflow theorem over the full artifact bundle;
- **exact**: no group-local sufficient statistic for train ranking under the current score;
- **exact**: one-shot holdout exploitation is top-\(B\) by fixed-target success probability;
- **exact**: ML confidence is exact only for a frozen target family, feature schema, and compatibility class;
- **non-exact but contractible**: clustered grouping and surrogate triage, with explicit rank-instability and regret bounds;
- **heuristic only**: `Auto` target semantics, scope-only incompatible history pooling, and partial-holdout “final ranking” interpreted as a common latent score.

That is the strongest repo-fit result that survives the second pass and is ready to guide consolidation without trespassing into PR-06 or PR-08. fileciteturn17file7