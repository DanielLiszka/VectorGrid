# PR-09 options evaluation and options GPU economics — three-pass answer set

This document packages the first-pass, second-pass, and third-pass answers for `PR-09` into one markdown artifact.

## Shared prompt scope block

```md
## `PR-09` Options evaluation and options GPU economics

Owned problem:
options split rigor, ranking utility, options-specific GPU chunking or cache
economics, and exactify-reduction math

Source packets:
`MR-16`

Primary ownership files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_types.rs`

Context files:

- `crates/app_core/src/options_data_store.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/bin/ta_options_expression_compare.rs`
- `src-tauri/src/bin/ta_options_expression_feasibility.rs`
- `src-tauri/src/bin/ta_options_expression_smoke.rs`
- `src-tauri/src/bin/ta_options_parent_artifact.rs`

Own these symbols or decisions:

- options split design and leakage control
- options ranking utility and adequacy thresholds
- signal-manifest timing semantics
- fill-model and price-semantics compatibility
- options-specific GPU runtime cache usage and chunk planning
- device-side retained summary design

Open problem:

- characterize an options-evaluation functional on sparse, invalid, path-coupled
  candidate tuples that preserves ranking integrity while reducing exactify cost

Why this is hard here:

- options tuples are sparse, partially invalid, timing-sensitive, and dependent
  on fill-model and price-semantics choices
- options GPU economics are not the same as repo-wide runtime economics because
  candidate validity and chunk shape are part of the mathematical problem

Mathematical object:

- a tuple-evaluation map from sparse option-event sequences to ranked candidate
  outputs, together with a device-side retained-summary contract and an
  exactification operator

Notation seed:

- let `\omega \in \Omega` denote an options tuple candidate
- let `I(\omega) \in \{0,1\}` be the validity indicator under timing and
  fill-model rules
- let `U(\omega)` be the exact tuple utility used for finalist ranking
- let `S_{\mathrm{gpu}}(\omega)` be the device-side retained summary
- let `E(\omega)` be the exactification operator; any reduction in exactify
  volume must control
  `U(\omega^\star) - U(\widehat{\omega}^\star)`

Primary questions:

- what split and leakage-control rule is mathematically sound for the current
  event structure?
- what retained device summary is sufficient for stable ranking under sparse or
  partially invalid tuples?
- when can exactification volume be reduced without changing finalist order?

Required deliverables:

- formal tuple-evaluation and exactification model
- candidate sufficiency theorem for device-side summaries
- counterexamples for invalid timing or fill-model assumptions
- complexity and exactify-volume tradeoffs

Acceptance bar:

- every exactify-reduction claim must preserve final ranking exactly or provide
  an explicit finalist-regret bound

Verification hooks:

- feasibility and smoke harness parity
- compare-artifact parity
- timing/fill-model falsifiers

Non-goals:

- do not redesign generic runtime cache or execution economics; that belongs to
  `PR-08`
- do not redesign generic validation stack math; that belongs to `PR-06`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly review persisted versus inferred validation verdicts
- keep options-specific GPU economics separate from repo-wide runtime economics
```

---

## First-pass answer

I treated `PR-09` as the first-pass mathematical answer, with the uploaded prompt and the copied repo files as the fixed source of truth.

### 1. Formal problem statement

Fix a provenance-locked instance
\[
P=(\text{parent id/hash},\ \text{signal-manifest id},\ \text{dataset hash},\ \text{price semantics},\ \text{provider provenance},\ \text{contract-reference-as-of}),
\]
because the repo treats drift in those fields as a different replay problem, not as harmless variation. Let \(E=\{e_1,\dots,e_n\}\) be the options signal events after split assignment. Each event carries a decision timestamp, earliest tradable timestamp, side, raw underlier price, optional base exit timestamp, split label, and a persisted `session_id`; today that `session_id` is a UTC-day label, not an exchange-local session key. The run request defines a finite tuple grid \(\Omega\) over delay, DTE bucket, moneyness bucket, hold, target, stop, exit template, and fill model, with structure family fixed per request. The mathematical problem is to evaluate each \(\omega\in\Omega\) exactly enough to rank candidates by the repo’s robust pre-sealed utility, then order finalists by sealed metrics, while minimizing exactification work. Any reduction in exactify volume is admissible only if it preserves the final ranking exactly or comes with an explicit regret or rank-instability bound. The canonical validation verdict is not the persisted artifact field; it is the verdict inferred from the available metrics, because artifacts are refreshed that way on load.

### 2. Notation for the search space, state space, path functional, ranking functional, and admissible approximations

Let the structural axis be
\[
A=\Delta\times D\times M,
\]
where \(\Delta\) is delay minutes, \(D\) is DTE bucket, and \(M\) is moneyness bucket. Let the exit axis be
\[
B=H\times T\times Z\times \{\chi\}\times \{\phi\},
\]
where \(H\) is hold minutes, \(T\) target-return bucket or `None`, \(Z\) stop-return bucket or `None`, \(\chi\) is the fixed exit-template family, and \(\phi\) is the fixed fill model. Then
\[
\Omega=A\times B.
\]

Let the evaluation lanes be
\[
L=\{\mathrm{S},\mathrm{H},\mathrm{C},\mathrm{Hh},\mathrm{X}\},
\]
with
- \(\mathrm{S}=(\text{Search},\text{Baseline})\),
- \(\mathrm{H}=(\text{Holdout},\text{Baseline})\),
- \(\mathrm{C}=(\text{Holdout},\text{Conservative})\),
- \(\mathrm{Hh}=(\text{Holdout},\text{Harsher})\),
- \(\mathrm{X}=(\text{Holdout},\text{Extreme})\),

and sealed-finalist analogues
\[
L_{\mathrm{sealed}}=\{\mathrm{T},\mathrm{TC},\mathrm{THh},\mathrm{TX}\}
\]
on the `SealedTest` split. This matches the repo’s two-stage ordering: robust score for pre-sealed ranking, then sealed metrics for finalist ordering.

For each lane \(\ell\in L\) and event \(e\), define the structural selector
\[
C_\ell(e,a)\in\mathcal C\cup\{\bot\},
\]
where \(C_\ell(e,a)=\bot\) means the candidate is structurally invalid for that event and lane. Structural invalidity covers the repo’s enumerated reasons such as no eligible contract as of event, no expiration bucket, no strike, missing companion leg, premium/volume/open-interest gates, entry past lag cutoff, and incompatible price semantics. Because premium filtering uses lane-dependent entry prices, \(C_\ell\) is lane-dependent, not purely geometric.

Define the state space
\[
X_\ell(e,\omega)=\big(C_\ell(e,a),\ \tau_{\text{entry}},\ \tau_{\text{cutoff}},\ \Pi_\ell(e,\omega)\big),
\]
where \(\Pi_\ell(e,\omega)\) is the aligned finite path of row data from entry to cutoff, including the companion leg when the structure is multi-leg. This path depends on the exit template: `FixedHold`, `FixedHoldOrTargetStop`, `EndOfDayOrTargetStop`, or `BaseExitOrTargetStop`; the last one depends on the parent-derived `base_exit_timestamp` in the signal manifest.

Define the exact event-level path functional
\[
Y_\ell(e,\omega)=
\begin{cases}
\bot, & C_\ell(e,a)=\bot \text{ or the realized path cannot be evaluated},\\
(\mathrm{pnl},\mathrm{prem},\mathrm{ret}), & \text{otherwise},
\end{cases}
\]
where \(\mathrm{ret}=\mathrm{pnl}/\mathrm{prem}\). Then \(I_\ell(e,\omega)=1_{\{Y_\ell(e,\omega)\neq \bot\}}\). The split-lane metric map is
\[
M_\ell(\omega)=\Phi\big(N_\ell,\{Y_\ell(e,\omega):e\in E_\ell,\ I_\ell(e,\omega)=1\}\big),
\]
where \(N_\ell=|E_\ell|\) is the total number of events in that lane’s split. This matches the repo metric contract:
\[
\text{invalid\_events}=N_\ell-\text{valid\_trades},
\qquad
\text{invalid\_rate}=\frac{\text{invalid\_events}}{N_\ell},
\qquad
\text{expectancy\_per\_premium}=\frac{\text{total pnl}}{\text{total premium}}.
\]

Define the lane utility
\[
u_\ell(\omega)=\bigl(1-\mathrm{invalid\_rate}(M_\ell(\omega))\bigr)\cdot \mathrm{expectancy\_per\_premium}(M_\ell(\omega)).
\]
Then the exact robust ranking functional is
\[
U^\star(\omega)=0.45u_{\mathrm S}(\omega)+0.25u_{\mathrm H}(\omega)+0.15u_{\mathrm C}(\omega)+0.10u_{\mathrm{Hh}}(\omega)+0.05u_{\mathrm X}(\omega),
\]
with the repo’s missing-lane fallback rule when a lane is absent. Sealed metrics are excluded from this robust score. Finalist ordering is a second lexicographic functional \(V^\star\) that compares sealed evaluation, then sealed stress, then sealed harsher stress, then sealed extreme stress, then robust score, then pre-sealed rank.

Admissible approximations are summaries \(\widetilde S(\omega)\) that induce lower and upper bounds
\[
\underline U(\omega)\le U^\star(\omega)\le \overline U(\omega).
\]
For a shortlisted set \(Q\subseteq\Omega\), define exactification regret bound
\[
\mathcal R(Q)=\max_{\omega\notin Q}\overline U(\omega)-\max_{\hat\omega\in Q}\underline U(\hat\omega).
\]
If \(\mathcal R(Q)<0\), then exactifying only \(Q\) preserves the true winner exactly.

Two provisional ranking functionals already exist implicitly in the repo:
\[
\widetilde U_{\mathrm{cpu}}=0.45u_{\mathrm S}+0.25u_{\mathrm H}+0.30u_{\mathrm C},
\]
because CPU pre-top-\(k\) ranking omits harsher and extreme lanes, and
\[
\widetilde U_{\mathrm{gpu}}=0.45u_{\mathrm S}+0.25u_{\mathrm H}+0.15u_{\mathrm C}+0.15u_{\mathrm{Hh}},
\]
because the GPU provisional rank includes harsher but omits exact extreme stress. These identities matter for exactify safety.

### 3. Fixed invariants and interface constraints

The first invariant is provenance lock. The problem is defined only relative to a fixed parent artifact, signal manifest, dataset hash, semantics, provider surface, and contract-reference timestamp.

The second invariant is price-semantics compatibility. `MinuteAggregate` and `DerivedMidSnapshot` support only `AggregateResearch`; `TopOfBookQuote` supports the quote-aware fill models. Runtime semantics are not symmetric: `DerivedMidSnapshot` is evaluated with the same aggregate-style entry, exit, and path-range logic as `MinuteAggregate`, while `TopOfBookQuote` uses quote-side entry/exit prices and collapses the path range to a point \((px,px)\).

The third invariant is signal timing. If `trade_on_next_bar` is true, the manifest’s decision index is the prior bar and earliest tradable is the entry bar; `base_entry_timestamp` is set to earliest tradable, `base_exit_timestamp` comes from the parent trade when available, and `session_id` is currently a UTC-day persistence label.

The fourth invariant is split construction. Events are first chronologically ordered by `earliest_tradable_timestamp` and `event_id`. `EventChronological` partitions that total order. `SessionChronological` instead partitions contiguous blocks of persisted session ids, with fallback to a UTC-day surrogate when session ids are missing, and with fallback back to event chronology when there are fewer than three sessions. So the strongest mathematically sound leakage notion available today is leakage across persisted session-equivalence classes, not across exchange-local sessions.

The fifth invariant is verdict semantics. `validation_verdict` and `robust_score` are persisted snapshots; the canonical object is `inferred_validation_verdict()`, which recomputes the verdict from supported holdout or sealed metrics plus stress fragility.

The sixth invariant is that options-specific GPU economics are local to this owned surface. The GPU summary stores per-tuple aggregates, outcome tensors are flattened from exact CPU-prepared outcomes, chunk planning budgets against those outcome tensors plus shared-offset layouts, and the combined scorer counts distinct shared layouts across baseline, conservative, and harsher lanes.

### 4. Strongest plausible positive result

The strongest repo-fit positive result was a three-part statement.

First, exact tuple evaluation factorizes without loss:
\[
Y_\ell(e,(a,b))=
\begin{cases}
\bot,& C_\ell(e,a)=\bot,\\
\Psi_\ell(e,a,b,C_\ell(e,a)),& C_\ell(e,a)\neq\bot.
\end{cases}
\]
That is, structural selection depends only on event and structural coordinates, while the exit-path evaluation depends only on the chosen candidate plus exit coordinates.

Second, for the current robust ranking functional, the minimal per-lane sufficient statistic is
\[
S_{\mathrm{rank},\ell}(\omega)=\bigl(n_\ell,\ P_\ell,\ Q_\ell,\ N_\ell\bigr),
\]
where \(n_\ell\) is valid-trade count, \(P_\ell\) total pnl, \(Q_\ell\) total premium, and \(N_\ell\) total split-event count. That is enough because \(U^\star\) uses each lane only through invalid rate and expectancy per premium. For the current sealed finalist comparator, add wins:
\[
S_{\mathrm{cmp},\ell}(\omega)=\bigl(n_\ell,\ W_\ell,\ P_\ell,\ Q_\ell,\ N_\ell\bigr).
\]
Median return and return volatility are not needed for current ordering. So median exactification can always be delayed without changing present ranking semantics.

Third, exactify drift decomposes cleanly.

For the current CPU provisional rank,
\[
\Delta_{\mathrm{cpu}}(\omega)
:=U^\star(\omega)-\widetilde U_{\mathrm{cpu}}(\omega)
=0.10\bigl(u_{\mathrm{Hh}}(\omega)-u_{\mathrm C}(\omega)\bigr)+0.05\bigl(u_{\mathrm X}(\omega)-u_{\mathrm C}(\omega)\bigr).
\]

For the current GPU provisional rank,
\[
\Delta_{\mathrm{gpu}}(\omega)
:=U^\star(\omega)-\widetilde U_{\mathrm{gpu}}(\omega)
=0.05\bigl(u_{\mathrm X}(\omega)-u_{\mathrm{Hh}}(\omega)\bigr).
\]

So the strongest plausible positive theorem was:

> If a shortlist \(Q\) is chosen together with valid per-tuple bounds on the omitted-lane drift, and every pruned tuple satisfies
> \[
> \overline U(\omega)<\max_{\hat\omega\in Q}\underline U(\hat\omega),
> \]
> then exactifying only \(Q\) preserves final robust ranking exactly.
> Under the current GPU scorer this requires bounding only the extreme-vs-harsher gap; under the current CPU provisional scorer it requires bounding both harsher-vs-stress and extreme-vs-stress gaps.

### 5. Strongest plausible impossibility result

No device-side retained summary that omits the exact extreme-stress lane, or an interval certificate for that lane, can preserve the final robust ranking for all tuples. The reason is structural: the final robust score depends on \(u_{\mathrm X}\), while the current provisional summaries do not determine it.

There is also a split-level impossibility: no theorem can claim exchange-session leakage control from the current `SessionChronological` mode, because the persisted `session_id` is only a UTC-day label.

And there is a semantics-level impossibility: no fill-model-agnostic theorem can cover `MinuteAggregate`, `DerivedMidSnapshot`, and `TopOfBookQuote` simultaneously, because their runtime entry/exit/path semantics are different.

### 6. Candidate theorem or lemma list

1. **Structural/exit factorization lemma.**  
   For each lane \(\ell\), exact tuple evaluation factors through a structural selector \(C_\ell(e,a)\) and an exit-path operator \(\Psi_\ell\).

2. **Persisted-session split theorem.**  
   Among chronology-preserving rules available on the current event surface, `SessionChronological` is the maximal rule that avoids splitting persisted session-equivalence classes across folds.

3. **Provenance invariance lemma.**  
   Exact evaluation is defined only on a fixed provenance class \(P\).

4. **Lane sufficiency lemma for robust ranking.**  
   \(S_{\mathrm{rank},\ell}=(n_\ell,P_\ell,Q_\ell,N_\ell)\) is sufficient for current robust ranking on lane \(\ell\).

5. **Comparator sufficiency lemma for finalist ordering.**  
   \(S_{\mathrm{cmp},\ell}=(n_\ell,W_\ell,P_\ell,Q_\ell,N_\ell)\) is sufficient for current sealed lane comparison.

6. **Median irrelevance lemma.**  
   Under the current repo ordering functions, median-return enrichment is ranking-neutral.

7. **CPU drift decomposition proposition.**  
   \[
   \Delta_{\mathrm{cpu}}=0.10(u_{\mathrm{Hh}}-u_{\mathrm C})+0.05(u_{\mathrm X}-u_{\mathrm C}).
   \]

8. **GPU drift decomposition proposition.**  
   \[
   \Delta_{\mathrm{gpu}}=0.05(u_{\mathrm X}-u_{\mathrm{Hh}}).
   \]

9. **Verdict canonicalization lemma.**  
   The formal validation verdict is `inferred_validation_verdict()`, not the persisted field.

10. **Non-identifiability proposition for omitted lanes.**  
    No summary that omits exact extreme-lane information can recover final robust ranking for all tuples.

### 7. One explicit counterexample family

Take two long-single tuples, \(\omega_A\) and \(\omega_B\), identical on all search, holdout-baseline, conservative, and harsher summaries, and suppose both survive current GPU provisional ranking with the same retained summary.

On one holdout event, both have entry price \(1.00\), no target, and stop threshold \(0.24\). Let the cutoff close be \(1.00\) in both cases, so if no stop is hit they exit flat. Let the path low after entry be:
- for \(\omega_A\): \(0.2500\),
- for \(\omega_B\): \(0.2550\).

Then:
- conservative low is unchanged, so both stay above stop;
- harsher low multiplies by \(0.98\), so both still stay above stop;
- extreme low multiplies by \(0.95\), so
  \[
  0.2500\times 0.95=0.2375<0.24
  \]
  for \(\omega_A\), but
  \[
  0.2550\times 0.95=0.24225>0.24
  \]
  for \(\omega_B\).

So \(\omega_A\) suffers an extreme-lane stop hit and \(\omega_B\) does not. Their provisional GPU score is identical, but their exact final robust scores differ after extreme exactification.

### 8. One lower-bound, identifiability, or impossibility angle

There is an information-theoretic lower bound against over-compressed retained summaries.

For the current ranking functions, a small summary is enough because the score ignores medians. But exact median recovery is impossible from a fixed set of aggregate moments such as count, wins, total pnl, total premium, \(\sum r\), and \(\sum r^2\). For \(m\ge 3\) valid returns, there exist different return multisets with the same count, wins, sum, and sum of squares but different medians.

### 9. Unresolved questions that block implementation

The biggest blocker is objective identity: is the canonical top-\(k\) the top-\(k\) under exact \(U^\star\), or the top-\(k\) under the current provisional scorer that is later exactified?

The second blocker is leakage semantics: does “session leakage” mean leakage across persisted UTC-day labels, or across exchange-local sessions?

The third blocker is lane-dependent validity: because premium filters are applied using lane-dependent entry prices, structural validity itself changes with stress lane. That is mathematically coherent, but it must be affirmed as intentional.

The fourth blocker is semantics identity for `DerivedMidSnapshot`: should it remain aggregate-style, or is it intended to represent quote-mid behavior? The current runtime says aggregate-style.

The fifth blocker is verdict authority: should persisted verdicts ever be treated as canonical in artifacts, or are they always cache fields to be refreshed?

The sixth blocker is the acceptable contract for non-exact pruning: top-1 regret, top-\(k\) membership instability, finalist regret, or sealed-final-rank instability.

---

## Second-pass answer

I treated the second pass as an adversarial attack on the leading formalization.

### 1. Explicit failure modes

The first-pass formalization overstated three things.

First, it treated structural validity as lane-dependent in general. That is false at repo scope. Structural candidate selection is parameterized by `stress_mode`, but under the current semantics that only changes candidate validity for some structure families. For `LongSingle`, and likewise `LongPair` under current entry semantics, the selected candidate identity is stress-invariant; for debit verticals, stress can change the effective premium and therefore flip filter validity. So any theorem that says “structural validity is lane-dependent” must be weakened to “possibly lane-dependent, family-specific, and not universal.”

Second, the first pass implicitly treated `exactify_ranked_results` as if it repaired all provisional GPU summaries. It does not. The current GPU path keeps the GPU-provided search, holdout, conservative, and harsher non-median aggregates as authoritative and exactifies only what is missing later: median enrichment and exact extreme-stress metrics. So any pruning theorem that relies on “GPU shortlist first, exactify later will repair any aggregate drift” is mathematically wrong for the current code.

Third, the first pass ignored mixed-precision drift. `GpuFlatScoreInputs` are flattened for the GPU scorer, `OptionsGpuTupleAggregate` stores `total_pnl`, `total_premium`, `sum_return_on_premium`, and `sumsq_return_on_premium` in `f32`, and the combined kernel emits robust scores before host exactify. Therefore the GPU path is not just “exact score without extreme”; it is “exact score with omitted extreme only if one additionally assumes no relevant `f32` ranking drift.”

A fourth failure mode is finite-grid tie handling. GPU shortlist ordering uses `robust_score`, then search expectancy, then tuple index. CPU sorting is based on `robust_score` and search expectancy, and exact total-order claims need a canonical tuple key.

A fifth failure mode is top-\(k\) versus top-1 confusion. The repo does not just need the winner. It truncates to `top_k`, exactifies that shortlist, and then evaluates sealed metrics only for `sealed_finalist_count` finalists. So a winner-only regret statement is too weak.

A sixth failure mode is session leakage overclaim. The split logic only justifies “don’t cut through persisted session labels,” and even that only if `session_id` is a chronological partition key.

A seventh failure mode is conflating ranking utility with run promotability. Coverage adequacy and overlap health are computed separately from tuple ranking. A run can be non-promotable because search overlap is missing even though the tuple utility functional is still well defined.

### 2. Adversarial constructions or separating examples

#### A. Omitted-extreme separating example

Take two tuples with identical exact search, holdout, conservative, and harsher lane aggregates, but different extreme outcomes on a single holdout event. Since the GPU combined scorer only materializes baseline, conservative, and harsher holdout aggregates before shortlist formation, the two tuples are indistinguishable at shortlist time, yet their final robust scores differ after exact extreme injection.

#### B. Mixed-precision near-tie separation

Take two tuples \(\omega_1,\omega_2\) whose exact pre-extreme robust scores differ by less than the cumulative `f32` rounding error in the carried aggregates. Because GPU aggregates store pnl, premium, and return moments in `f32`, there exists a finite-grid near-tie regime where GPU order can flip relative to exact host aggregation even with identical extreme lanes.

#### C. Tie-break insufficiency example

Suppose two tuples have exactly equal robust score and equal search expectancy. The GPU comparator breaks the tie by tuple index. Any proposed sufficient statistic that omits canonical tuple identity cannot recover the total order used by the implementation.

#### D. Structure-family separation

Take a `LongSingle` tuple and a debit-vertical tuple with identical delay/DTE/moneyness coordinates. Under current semantics, stress-mode changes candidate selection only for the debit vertical, because the selection-time premium depends on stressed companion pricing there, while `LongSingle` selection uses stress-invariant entry pricing. Therefore a theorem that assumes a single structure-agnostic factorization across all families is false.

#### E. Session-label noncontiguity pathology

Let sorted events have persisted labels \(A,B,A\). A session-based splitter that groups by label identity rather than contiguous runs can merge noncontiguous blocks into one abstract “session,” which is not a real session partition. The current theorem survives only if each session label appears in a single chronological block.

#### F. Top-\(k\) boundary pathology

Let tuple \(k\) and tuple \(k+1\) be separated by less than the omitted-extreme plus floating-point uncertainty. A pruning theorem that protects only the winner can still change the membership of the retained `top_k`, which changes what gets exactified and what can ever become a sealed finalist.

### 3. Corrected theorem or lemma candidates

#### Lemma 1: Family-conditional structural/exit factorization

For a fixed structure family \(F\), exact tuple evaluation factorizes through structural selection and exit-path evaluation only after specifying whether \(F\) is stress-invariant at candidate selection time.

For `LongSingle` and `LongPair` under the current semantics, structural candidate identity is stress-invariant, so lane differences live in exit summaries only.

For debit verticals, structural validity can vary by stress lane, so factorization must be stated lane-by-lane.

#### Lemma 2: Median irrelevance survives

For the current repo ordering functions, median-return enrichment is ranking-neutral. The GPU exactify stage enriches medians after shortlist formation without changing the retained non-median aggregates.

#### Lemma 3: Exact sufficient state for current ranking requires a tie key

If lane aggregates are exact, then the current robust score is determined by
\[
(n_\ell,P_\ell,Q_\ell,N_\ell)_{\ell \in L}
\]
for the relevant lanes, and the current sealed-final comparator is determined by
\[
(n_\ell,W_\ell,P_\ell,Q_\ell,N_\ell)_{\ell \in L_{\mathrm{sealed}}}
\]
plus a canonical tuple key for tie resolution. Without the canonical key, the theorem is only about score equality, not total order.

#### Lemma 4: Current GPU retained summary is only approximately sufficient

The current GPU retained state is sufficient for exact ranking only under an extra arithmetic assumption:
\[
\varepsilon_{\mathrm{fp}}(\omega)=0
\]
for all shortlisted tuples, or at least small enough that no relevant pairwise margin is crossed.

#### Proposition 5: Exactify reduction needs a top-\(k\) margin certificate, not just winner regret

A non-exact pruning rule preserves current repo behavior only if it certifies that no tuple outside the retained shortlist can cross into the retained pre-sealed top-\(k\) after adding omitted extreme and numerical correction terms. A top-1 certificate is not enough.

#### Lemma 6: Persisted verdicts are non-canonical

The mathematically meaningful verdict object is the inferred verdict from supported validation lanes, not the persisted verdict field.

### 4. Weakest assumptions under which the result survives

The strongest surviving result needs the following minimal assumptions.

First, fix a single structure family, and if the theorem uses structural stress invariance, restrict it to a family where that is actually true under current semantics.

Second, require a canonical total-order key, for example tuple index in compiled order.

Third, require the session-id contract to be: each persisted `session_id` appears in one chronological block.

Fourth, require one of two arithmetic assumptions:
\[
\varepsilon_{\mathrm{fp}}=0
\]
for CPU or exact host-aggregate reasoning, or
\[
\min_{i \le k,\ j > k}\bigl(U_i-U_j\bigr) > 2\varepsilon_{\mathrm{fp}} + \varepsilon_{\mathrm{extreme}} + \varepsilon_{\mathrm{tie}}
\]
for GPU shortlist preservation, where \(\varepsilon_{\mathrm{extreme}}\) bounds omitted-extreme drift and \(\varepsilon_{\mathrm{tie}}\) covers tie-key ambiguity.

Fifth, if the claim is about final artifact stability, protect pre-sealed top-\(k\) membership, not just the top-1 winner.

### 5. Distinction between exact, approximate, and heuristic claims

#### Exact

It is exact that current robust ranking ignores sealed metrics.

It is exact that current GPU exactify injects medians and exact extreme later, while preserving retained GPU expectancy aggregates for already-retained search/holdout/stress/harsher lanes.

It is exact that only configured sealed finalists receive sealed evaluation metrics.

It is exact that persisted validation verdicts are not authoritative once refreshed from inferred support.

#### Approximate

Any claim that current GPU search-state is “sufficient” for exact ranking is approximate unless it includes an explicit floating-point margin term.

Any claim that GPU shortlist formation preserves final ranking is approximate unless it includes an omitted-extreme bound.

#### Heuristic

The current small-surface GPU/CPU parity tests support confidence, but they are empirical evidence, not a theorem over the full finite grid.

The runtime auto-backend thresholds and promotability thresholds are operational heuristics or product rules, not components of the mathematical ranking functional.

### 6. Revised time, memory, and retained-state costs

The first-pass answer understated what has to be counted.

The structural cache cost is
\[
\Theta\!\big(E\,|\Delta|\,|D|\,|M|\big)
\]
candidate slots plus invalid-reason storage.

The exit-summary cost is
\[
\Theta\!\big(E\,|\Delta|\,|D|\,|M|\,|H|\,|T|\,|Z|\big)
\]
outcome slots, because the exit cache stores outcomes over the structural candidate slot crossed with the exit axis.

The current device retained state per tuple is not just a scalar score. `OptionsGpuTupleAggregate` carries two counts and four `f32` accumulators per lane. Across the GPU shortlist stage that means four such lanes before exact extreme on the current combined path: search baseline, holdout baseline, holdout conservative, and holdout harsher, plus a robust score buffer and chunk/output-buffer overhead.

More importantly, exactness cost is not finished after GPU shortlist formation. Current exactify still performs host-side median enrichment and exact extreme evaluation on retained tuples, and sealed evaluation later runs only on the configured finalist subset.

### 7. Dependency burden on other PR prompts

This second-pass result has low redesign burden if kept narrow.

It touches `PR-06` only as an interface fact: the canonical validation verdict is inferred from supported lanes, not persisted.

It touches `PR-08` only as an interface fact: GPU chunk planning, cache reuse, and memory budgeting are options-specific here and should stay separate from repo-wide runtime economics.

It does not require redesigning context surfaces. The surviving result lives inside the owned decisions: split rigor, ranking utility, signal timing semantics, fill-model/price-semantics compatibility, GPU retained summary design, and exactify-reduction math.

### Narrowed strongest surviving repo-fit result

After the adversarial pass, the strongest surviving result was not a broad pruning theorem.

It was this:

For the current repo, the only robust exact statement is that median-return replay is dispensable for ranking, while any further exactify reduction must be structure-family-conditional and must carry an explicit margin contract covering three hazards at once: omitted extreme-stress contribution, mixed-precision GPU aggregate drift, and finite-grid tie resolution. Without that contract, nontrivial pre-exactify pruning remains heuristic.

---

## Third-pass answer

This is the strongest implementation-ready mathematical result for `PR-09` that survives the second pass and stays inside the owned surface.

### 1. Final formalization

Define a fixed run instance
\[
\mathcal P=(\mathcal R,\mathcal M,\mathcal D,\kappa),
\]
where:

- \(\mathcal R\) is the normalized run request: structure family, delay/DTE/moneyness axes, hold/target/stop axes, exit template, fill model, filters, `top_k`, `sealed_finalist_count`, split ratios, and split-assignment mode.
- \(\mathcal M\) is the options signal manifest after projection from the parent artifact, including `decision_timestamp`, `earliest_tradable_timestamp`, `base_exit_timestamp`, and persisted `session_id`.
- \(\mathcal D\) is the imported options dataset together with the replay-locked provenance class: parent id/hash, signal-manifest id, dataset id/hash, price semantics, provider provenance, and contract-reference-as-of.
- \(\kappa:\Omega\to\{0,\dots,|\Omega|-1\}\) is the canonical compile-order key induced by compiled tuple order. This is the tie key required for exact total order.

Let
\[
\Omega=\Delta\times D\times M\times H\times T\times Z
\]
denote the finite tuple grid, with structure family, exit template, and fill model fixed by \(\mathcal R\).

Sort events by
\[
(\text{earliest\_tradable\_timestamp},\ \text{event\_id}).
\]
Let
\[
\sigma:E\to\{\mathrm S,\mathrm H,\mathrm T\}
\]
be the split map returned by either `EventChronological` or `SessionChronological`.

There are two split semantics classes:

1. **Event chronology.**  
   \(\sigma\) partitions the globally sorted event order directly.

2. **Persisted-session chronology.**  
   \(\sigma\) partitions chronological runs of persisted `session_id` labels, with UTC-day fallback when labels are missing. This is a guarantee about persisted labels, not exchange-local sessions.

Let the pre-sealed ranking lanes be
\[
L_{\mathrm{rank}}=\{\mathrm S,\mathrm H,\mathrm C,\mathrm{Hh},\mathrm X\},
\]
and the sealed finalist lanes be
\[
L_{\mathrm{sealed}}=\{\mathrm T,\mathrm{TC},\mathrm{THh},\mathrm{TX}\}.
\]

For each tuple \(\omega\in\Omega\), event \(e\), lane \(\ell\), semantics class \(s\), and structure family \(F\), define:

- structural selection \(C_{F,s,\ell}(e,\omega)\in\mathcal C\cup\{\bot\}\),
- exact realized trade outcome \(Y_{F,s,\ell}(e,\omega)\in\{\bot\}\cup\mathbb R^3\), where
  \[
  Y=(\mathrm{pnl},\mathrm{prem},\mathrm{ret}),\qquad \mathrm{ret}=\mathrm{pnl}/\mathrm{prem}.
  \]

From these, define exact lane aggregates
\[
A_\ell(\omega)=\bigl(n_\ell,\ w_\ell,\ P_\ell,\ Q_\ell,\ R_\ell,\ S_\ell,\ m_\ell,\ N_\ell\bigr),
\]
where:

- \(n_\ell\): valid-trade count,
- \(w_\ell\): win count,
- \(P_\ell\): total pnl,
- \(Q_\ell\): total premium,
- \(R_\ell=\sum r_i\),
- \(S_\ell=\sum r_i^2\),
- \(m_\ell\): median return on premium,
- \(N_\ell\): total event count in the lane split.

The exact robust lane utility is
\[
u_\ell(\omega)=
\begin{cases}
\frac{n_\ell}{N_\ell}\frac{P_\ell}{Q_\ell}, & N_\ell>0,\ Q_\ell>0,\\
0,&\text{otherwise.}
\end{cases}
\]

Define fallback-completed lane utilities
\[
\bar u_{\mathrm S}=u_{\mathrm S},
\]
\[
\bar u_{\mathrm H}=
\begin{cases}
u_{\mathrm H},&\text{if holdout exists},\\
u_{\mathrm S},&\text{otherwise,}
\end{cases}
\]
\[
\bar u_{\mathrm C}=
\begin{cases}
u_{\mathrm C},&\text{if conservative holdout exists},\\
u_{\mathrm S},&\text{otherwise,}
\end{cases}
\]
\[
\bar u_{\mathrm{Hh}}=
\begin{cases}
u_{\mathrm{Hh}},&\text{if harsher holdout exists},\\
\bar u_{\mathrm C},&\text{otherwise,}
\end{cases}
\]
\[
\bar u_{\mathrm X}=
\begin{cases}
u_{\mathrm X},&\text{if extreme holdout exists},\\
\bar u_{\mathrm{Hh}},&\text{otherwise.}
\end{cases}
\]

Then the exact pre-sealed robust score is
\[
U^\star(\omega)=
0.45\bar u_{\mathrm S}
+0.25\bar u_{\mathrm H}
+0.15\bar u_{\mathrm C}
+0.10\bar u_{\mathrm{Hh}}
+0.05\bar u_{\mathrm X}.
\]

The exact sealed-final comparator is the lexicographic order
\[
C^\star(\omega_i,\omega_j)
\]
obtained by comparing, in order:

1. sealed baseline metrics,
2. sealed conservative metrics,
3. sealed harsher metrics,
4. sealed extreme metrics,
5. robust score \(U^\star\),
6. compile key \(\kappa\).

For a single lane metric block with exact aggregate \(A=(n,w,P,Q,\dots)\), the comparator key is
\[
K(A)=
\begin{cases}
\left(\frac{P}{Q},\ \frac{P}{n},\ \frac{w}{n},\ n\right),& n>0,\ Q>0,\\
\bot,& n=0\text{ or unsupported.}
\end{cases}
\]

Coverage adequacy is a separate run-level functional
\[
G(\mathcal P),
\]
built from overlap rates, candidate/outcome coverage, and dominant invalid reasons. It affects promotability, not tuple ordering.

The canonical verdict functional is
\[
V(\omega)=\texttt{inferred\_validation\_verdict}(\omega),
\]
not the persisted verdict snapshot.

### 2. Final theorem or lemma list

#### Theorem 1: Provenance-lock theorem

All exact ranking, verdict, and replay claims are defined only within a fixed provenance class \(\mathcal D\). If parent id/hash, signal manifest id, dataset id/hash, price semantics, provider provenance, or contract-reference-as-of changes, the mathematical problem instance changes.

#### Theorem 2: Split theorem

Under current code, the strongest exact leakage-control statement is:

- `EventChronological` prevents fold interleaving only in the global event order.
- `SessionChronological` prevents cuts through chronological runs of persisted session labels.

It is **not** an exchange-session theorem. It is a persisted-session-label theorem.

#### Theorem 3: Semantics-class theorem

There are exactly two relevant price-semantics classes for `PR-09`.

1. **Quote class**
   \[
   s=\texttt{TopOfBookQuote}.
   \]
   In this class, entry price, exit price, and path range are all stress-invariant. Therefore for every structure family \(F\),
   \[
   A_{\mathrm H}=A_{\mathrm C}=A_{\mathrm{Hh}}=A_{\mathrm X},
   \]
   and likewise on sealed-test lanes,
   \[
   A_{\mathrm T}=A_{\mathrm{TC}}=A_{\mathrm{THh}}=A_{\mathrm{TX}}.
   \]

2. **Aggregate class**
   \[
   s\in\{\texttt{MinuteAggregate},\ \texttt{DerivedMidSnapshot}\}.
   \]
   In this class, path ranges and exit pricing vary by stress mode, and `DerivedMidSnapshot` belongs to the same mathematical class as `MinuteAggregate` under current code.

This theorem is the key surviving exact simplifier.

#### Theorem 4: Structural-equivalence theorem

Let \(F\) be the structure family and \(s\) the semantics class.

- If \(s=\texttt{TopOfBookQuote}\), then structural candidate selection is stress-invariant for **all** \(F\in\{\texttt{LongSingle},\texttt{LongPair},\texttt{DebitVertical}\}\).
- If \(s\) is aggregate-style, then structural candidate selection is stress-invariant for
  \[
  F\in\{\texttt{LongSingle},\texttt{LongPair}\},
  \]
  but can vary by stress lane for
  \[
  F=\texttt{DebitVertical}.
  \]

So structural cache sharing across stress lanes is exact for:
- all quote-semantics runs,
- aggregate-style `LongSingle`,
- aggregate-style `LongPair`.

It is not exact in general for aggregate-style `DebitVertical`.

#### Theorem 5: Exact sufficient statistics for pre-sealed ranking

Define the required exact lane set
\[
L^{\mathrm{req}}_{\mathrm{rank}}(s)=
\begin{cases}
\{\mathrm S,\mathrm H\},& s=\texttt{TopOfBookQuote},\\
\{\mathrm S,\mathrm H,\mathrm C,\mathrm{Hh},\mathrm X\},& s\text{ aggregate-style}.
\end{cases}
\]
(If holdout is absent, the set collapses to \(\{\mathrm S\}\).)

Then the exact sufficient statistic for pre-sealed ranking is
\[
S^{\mathrm{rank}}_{\mathrm{exact}}(\omega)
=
\kappa(\omega)\ \oplus\ 
\bigl((n_\ell,P_\ell,Q_\ell,N_\ell)\bigr)_{\ell\in L^{\mathrm{req}}_{\mathrm{rank}}(s)}.
\]

This statistic is sufficient and minimal up to deterministic transforms:
- \(m_\ell\), \(R_\ell\), and \(S_\ell\) are not required for current ranking,
- omitting any required lane in the aggregate class destroys exact sufficiency,
- omitting \(\kappa\) destroys exact total-order sufficiency.

#### Theorem 6: Exact sufficient statistics for sealed finalist ordering

Define the required exact sealed lane set
\[
L^{\mathrm{req}}_{\mathrm{sealed}}(s)=
\begin{cases}
\{\mathrm T\},& s=\texttt{TopOfBookQuote},\\
\{\mathrm T,\mathrm{TC},\mathrm{THh},\mathrm{TX}\},& s\text{ aggregate-style}.
\end{cases}
\]
(If sealed-test is absent, the set is empty.)

Then the exact sufficient statistic for sealed finalist ordering is
\[
S^{\mathrm{sealed}}_{\mathrm{exact}}(\omega)
=
\kappa(\omega)\ \oplus\
\bigl((n_\ell,w_\ell,P_\ell,Q_\ell)\bigr)_{\ell\in L^{\mathrm{req}}_{\mathrm{sealed}}(s)}.
\]

Again this is sufficient and minimal up to deterministic transforms. Median, mean-return, variance, and invalid-event counts are not required for the current sealed comparator.

#### Theorem 7: Median irrelevance theorem

Under the current repo ordering functions, median-return exactification is ranking-irrelevant. Therefore median replay can always be deferred until after pre-sealed top-\(k\) selection without changing:

- pre-sealed ranking,
- finalist membership,
- sealed finalist ordering.

#### Theorem 8: Current CPU/GPU exactness classes

Let \(\widetilde U_{\mathrm{cpu}}\) denote the current CPU provisional pre-truncation score and \(\widetilde U_{\mathrm{gpu}}\) the current GPU provisional pre-truncation score.

1. **Quote semantics.**
   In the quote class,
   \[
   \widetilde U_{\mathrm{cpu}} = U^\star
   \]
   exactly, because conservative, harsher, and extreme lanes collapse to holdout baseline.
   Also,
   \[
   \widetilde U_{\mathrm{gpu}} = U^\star
   \]
   at the mathematical lane level; only floating-point screening error remains.

2. **Aggregate semantics.**
   In the aggregate class,
   \[
   U^\star-\widetilde U_{\mathrm{cpu}}
   =0.10(u_{\mathrm{Hh}}-u_{\mathrm C})+0.05(u_{\mathrm X}-u_{\mathrm C}),
   \]
   and
   \[
   U^\star-\widetilde U_{\mathrm{gpu}}
   =0.05(u_{\mathrm X}-u_{\mathrm{Hh}}).
   \]

So the current CPU/GPU top-\(k\) truncation is exact in the quote class and only approximate in the aggregate class.

#### Theorem 9: Omitted-extreme impossibility theorem

In the aggregate class, no retained summary that omits exact extreme-lane ranking state can be universally sufficient for exact pre-sealed ranking. There exist tuples with identical retained non-extreme state and different exact extreme contributions.

#### Theorem 10: Interval-certificate theorem for top-\(k\)

Let \(\widetilde U(\omega)\) be any provisional score and let \(B(\omega)\ge 0\) satisfy
\[
|U^\star(\omega)-\widetilde U(\omega)|\le B(\omega).
\]
Define
\[
L(\omega)=\widetilde U(\omega)-B(\omega),\qquad
U(\omega)=\widetilde U(\omega)+B(\omega).
\]

For a provisional top-\(k\) set \(K\subseteq\Omega\), exact top-\(k\) membership is preserved if
\[
\min_{\omega\in K}L(\omega) > \max_{\eta\notin K}U(\eta).
\]

Exact order inside \(K\) is preserved if, after provisional sorting,
\[
L(\omega_i)>U(\omega_j)\qquad\text{for all }i<j,\ \omega_i,\omega_j\in K.
\]

This is the only admissible non-exact pruning theorem under the prompt’s acceptance bar.

### 3. Proof sketch or proof plan

#### For Theorem 3

Read the price-path semantics directly from the runtime definitions.

- In `TopOfBookQuote`, entry and exit use quote-side functions, and path range is \((px,px)\), independent of `AggregateStressMode`.
- In `MinuteAggregate` and `DerivedMidSnapshot`, entry/exit/path use stress-mode-dependent high/low perturbations.

Therefore quote semantics collapse all stress lanes exactly, while aggregate-style semantics do not.

#### For Theorem 4

Inspect `select_candidate` through `position_entry_prices`.

- `LongSingle`: no companion-leg stress term at selection.
- `LongPair`: companion entry uses stress-invariant entry pricing.
- `DebitVertical`: in aggregate-style semantics the companion leg contributes via stressed exit pricing, so selection-time premium can change by lane.
- In quote semantics, the companion-leg quote-side pricing ignores stress mode, restoring stress invariance even for `DebitVertical`.

#### For Theorems 5 and 6

Show that the comparator formulas use only:
- \(n,P,Q,N\) for robust ranking,
- \(n,w,P,Q\) for sealed ordering,
plus \(\kappa\) for total order.

Then construct separating examples:
- same \(n,P,Q,N\) but different medians: no ranking change,
- same robust/sealed metrics but different \(\kappa\): order changes,
- same non-extreme lanes but different extreme lane in aggregate semantics: exact robust order changes.

#### For Theorem 8

Substitute the lane-collapse identities from Theorem 3 into the robust-score fallback chain.

- In the quote class, all stress utilities equal holdout baseline, so both current CPU and current GPU formulas reduce to the exact formula.
- In the aggregate class, subtract the provisional formulas from the exact formula and simplify algebraically.

#### For Theorem 10

This is a standard interval-separation proof. If every retained tuple’s lower bound exceeds every pruned tuple’s upper bound, no pruned tuple can belong to the exact top-\(k\). Pairwise interval separation inside the retained set gives exact order preservation.

### 4. Sufficient statistics, or proof they do not exist under the stated constraints

#### Exact sufficient statistics that do exist

For exact pre-sealed ranking:

- quote semantics:
  \[
  \kappa \oplus (n,P,Q,N)_{\mathrm S,\mathrm H}
  \]
- aggregate semantics:
  \[
  \kappa \oplus (n,P,Q,N)_{\mathrm S,\mathrm H,\mathrm C,\mathrm{Hh},\mathrm X}
  \]

For exact sealed finalist ordering:

- quote semantics:
  \[
  \kappa \oplus (n,w,P,Q)_{\mathrm T}
  \]
- aggregate semantics:
  \[
  \kappa \oplus (n,w,P,Q)_{\mathrm T,\mathrm{TC},\mathrm{THh},\mathrm{TX}}
  \]

#### Exact sufficient statistics that do not exist under current constraints

Under the current retained GPU summary contract:

- non-extreme aggregate lanes are carried in mixed precision,
- exact extreme is absent,
- medians are absent until enrichment.

Therefore **no universal exact sufficient statistic for aggregate-class pre-sealed ranking exists under the current retained state**. The missing extreme lane alone kills exact sufficiency. The mixed-precision path kills exactness even if extreme were ignored.

#### Ranking-only state vs report-complete state

Current `OptionsGpuTupleAggregate` carries more than ranking needs:

- needed for current ranking: \(n,P,Q\) and split event count \(N\) supplied externally,
- additionally carried today: \(w,R,S\).

This extra state is useful for reporting and downstream diagnostics, but not required for current ranking. Median still cannot be reconstructed from it.

### 5. Exactness conditions and equivalence classes

There are four exactness layers.

#### Layer A: Provenance exactness

All claims assume fixed replay provenance. If provenance drifts, the theorem is void.

#### Layer B: Split exactness

All session-based leakage claims are with respect to persisted session labels, not exchange sessions.

#### Layer C: Lane-equivalence exactness

There are three relevant equivalence classes.

1. **Full lane equivalence**
   \[
   A_{\mathrm H}=A_{\mathrm C}=A_{\mathrm{Hh}}=A_{\mathrm X}
   \]
   and similarly on sealed lanes. This holds exactly in quote semantics.

2. **Structural-only equivalence**
   Structural candidate selection is the same across stress lanes, but outcomes are not. This holds for aggregate-style `LongSingle` and `LongPair`.

3. **No cross-stress structural equivalence**
   This is the aggregate-style `DebitVertical` case.

#### Layer D: Arithmetic exactness

Even with lane-equivalent summaries, exact ranking still requires either:
- exact arithmetic, or
- a certified interval envelope for mixed-precision drift.

### 6. Error, regret, or bias bounds for every non-exact claim

#### Claim A: Using current GPU ranking state in the quote class

This is exact at the lane-model level and approximate only because of mixed precision.

Let \(\varepsilon_{\mathrm{fp}}(\omega)\) be a certified bound on the absolute robust-score error induced by carrying GPU aggregates in mixed precision for tuple \(\omega\). Then
\[
|U^\star(\omega)-\widetilde U_{\mathrm{gpu}}(\omega)|\le \varepsilon_{\mathrm{fp}}(\omega).
\]

This is the full non-exact contract in the quote class.

#### Claim B: Using current GPU ranking state in the aggregate class

Let \(B_{\mathrm X}(\omega)\) be any certified bound on
\[
|u_{\mathrm X}(\omega)-u_{\mathrm{Hh}}(\omega)|.
\]
Then
\[
|U^\star(\omega)-\widetilde U_{\mathrm{gpu}}(\omega)|
\le \varepsilon_{\mathrm{fp}}(\omega)+0.05\,B_{\mathrm X}(\omega).
\]

If no finite \(B_{\mathrm X}\) is available from retained state, the only exact admissible choice is
\[
B_{\mathrm X}(\omega)=+\infty,
\]
which forbids pruning from that summary alone.

#### Claim C: Using current CPU ranking state in the aggregate class

Let
\[
B_{\mathrm{Hh-C}}(\omega)\ge |u_{\mathrm{Hh}}(\omega)-u_{\mathrm C}(\omega)|,
\qquad
B_{\mathrm{X-C}}(\omega)\ge |u_{\mathrm X}(\omega)-u_{\mathrm C}(\omega)|.
\]
Then
\[
|U^\star(\omega)-\widetilde U_{\mathrm{cpu}}(\omega)|
\le 0.10\,B_{\mathrm{Hh-C}}(\omega)+0.05\,B_{\mathrm{X-C}}(\omega).
\]

Again, without finite certified bounds, exact pre-truncation pruning is not allowed.

#### Claim D: Median omission

This one is exact:
\[
\Delta_{\mathrm{median}}(\omega)=0
\]
for all current ranking and finalist-ordering functions.

#### Claim E: Any empirical parity claim

If a compare harness is used to estimate
\[
\widehat\varepsilon_{\mathrm{fp}}
=
\max_{\omega\in\mathcal S}
|U^\star_{\mathrm{cpu}}(\omega)-\widetilde U_{\mathrm{gpu}}(\omega)|,
\]
then the claim is only surface-local. Its contract is:

- valid only on the audited surface \(\mathcal S\),
- invalidated by semantics, structure-family, kernel, accumulator, or comparator changes.

### 7. Time, memory, and retained-state complexity

Let
\[
N_E=|E|,
\quad
N_A=|\Delta||D||M|,
\quad
N_B=|H||T||Z|,
\quad
N_\Omega=N_A N_B.
\]

#### Structural candidate slots

\[
\Theta(N_E N_A).
\]

#### Exit-summary outcome slots

\[
\Theta(N_E N_A N_B).
\]

This is the dominant exact evaluation lattice.

#### Current combined GPU output working-set cost

The current planner uses:

- per shared-input tuple bytes:
  \[
  2\cdot \texttt{u32}=8\text{ bytes},
  \]
- per aggregate-lane output bytes:
  \[
  2\cdot \texttt{i32}+4\cdot \texttt{f32}=24\text{ bytes},
  \]
- one extra `f64` robust score:
  \[
  8\text{ bytes}.
  \]

So the current combined GPU per-tuple working-set cost is
\[
8\cdot L_{\mathrm{shared}}
+
24\cdot L_{\mathrm{agg}}
+
8,
\]
where \(L_{\mathrm{shared}}\) is the number of distinct shared-input layouts and \(L_{\mathrm{agg}}\) is the number of aggregate output lanes.

For the current combined path:
- \(L_{\mathrm{agg}}=4\) (search, holdout, conservative, harsher),
- \(L_{\mathrm{shared}}=1\) for the current prepared lane family.

Hence current per-tuple device working-set cost is
\[
8+24\cdot 4+8=112\text{ bytes},
\]
excluding outcome tensors.

#### Exact ranking state size targets

If one were to carry exact ranking-only state:

- quote semantics exact rank state:
  2 required pre-sealed lanes,
- aggregate semantics exact rank state:
  5 required pre-sealed lanes.

So, relative to the current 4-lane GPU combined path:

- quote semantics admits an exact lane collapse below current 4-lane output,
- aggregate semantics exactness needs one more lane than current GPU if exact extreme is carried.

#### Outcome-tensor economics

Current combined GPU planning budgets against the sum of baseline, conservative, and harsher outcome tensors.

Mathematically:

- in quote semantics, all stress-lane outcome tensors are equivalent, so a future exact design could collapse them to the baseline tensor,
- in aggregate semantics, conservative/harsher/extreme outcome tensors are genuinely distinct in general.

This is an owned, options-specific GPU economics result, not a repo-wide runtime result.

### 8. File and symbol impact map

This section names the file/symbol surfaces implicated by the mathematical result if consolidated.

#### `crates/app_core/src/options_expression.rs`

Core ranking and exactify surface:

- `assign_splits`
- `select_candidate`
- `position_entry_prices`
- `position_exit_prices`
- `option_path_range_for_row`
- `evaluate_trade_summary`
- `evaluate_trade_detail`
- `compute_metrics`
- `RetainedMetricState`
- `RetainedSummaryState`
- `exactify_retained_summary_metrics`
- `exactify_retained_summary_enrichment`
- `exactify_retained_extreme_stress_metrics`
- `exactify_ranked_results`
- `apply_sealed_finalist_stage`
- `robust_score`
- `assign_sealed_final_ranks`
- `build_overlap_window_summary`
- `build_coverage_adequacy_summary`
- `gpu_scoring::aggregate_from_gpu`
- `gpu_scoring::compare_gpu_shortlist_indices`
- `gpu_scoring::build_ranked_result_from_gpu_aggregates`

#### `crates/app_core/src/options_expression_gpu.rs`

Options-specific GPU economics surface:

- `OptionsGpuTupleAggregate`
- `OptionsGpuRuntimeCacheUsage`
- `OptionsGpuCombinedTensorRun`
- `distinct_shared_input_lane_count`
- `OptionsGpuChunkPlan::for_layout`
- `OptionsGpuChunkPlan::for_inputs`
- `OptionsGpuChunkPlan::for_combined_inputs`

#### `crates/app_core/src/options_signal_manifest.rs`

Timing and session semantics surface:

- `session_id_from_timestamp`
- manifest builders
- `trade_on_next_bar`
- `base_exit_timestamp` projection

#### `crates/app_core/src/options_types.rs`

Formal contract surface:

- `OptionsPriceSemantics`
- `OptionsFillModelFamily`
- `OptionsStructureFamily`
- `OptionsSplitAssignmentMode`
- `OptionsExpressionMetrics`
- `OptionsExpressionTupleResult`
- `OptionsExpressionTupleResult::inferred_validation_verdict`
- `OptionsExpressionRunRequest`
- `OptionsExpressionCoverageAdequacySummary`

#### `src-tauri/src/commands_options_expression.rs`

Interface constraints only:

- replay provenance validation
- artifact verdict refresh
- summary/report surfaces

#### Harness surfaces

- `ta_options_expression_compare.rs`: parity and delta auditing
- `ta_options_expression_feasibility.rs`: feasibility, cache, and memory-budget falsifiers
- `ta_options_expression_smoke.rs`: end-to-end invariants

### 9. Verification obligations and falsifiers

#### Obligation 1: Quote-class lane collapse

For `TopOfBookQuote`, verify on every structure family that:
\[
A_{\mathrm H}=A_{\mathrm C}=A_{\mathrm{Hh}}=A_{\mathrm X}
\]
and sealed analogues, tuple by tuple.

**Falsifier:** any tuple whose conservative/harsher/extreme aggregate differs from baseline holdout under quote semantics.

#### Obligation 2: Aggregate-class structural equivalence

For aggregate-style `LongSingle` and `LongPair`, verify that structural candidate identity is the same across baseline/conservative/harsher/extreme lanes.

**Falsifier:** any event/tuple whose selected contract or invalid reason differs by stress lane in those families.

#### Obligation 3: Aggregate-style `DebitVertical` non-equivalence

Verify that the code path continues to allow lane-dependent selection there.

**Falsifier:** a code change that makes aggregate-style debit-vertical selection stress-invariant without updating the theorem set.

#### Obligation 4: Median irrelevance

Verify that no ranking comparator reads `median_return_on_premium`.

**Falsifier:** any future comparator that uses median before top-\(k\) selection or sealed ranking.

#### Obligation 5: Exact total-order key

Verify that exact total order is still compile-order-tied:
- either explicit tuple index tie-break,
- or stable-sort preservation of compile order.

**Falsifier:** unstable sorting or a new tie key.

#### Obligation 6: Mixed-precision envelope

If GPU aggregates are relied upon without overwrite, compare harnesses must produce a certified \(\varepsilon_{\mathrm{fp}}\) on the audited surface.

**Falsifier:** any compare run showing a top-\(k\) boundary crossing larger than the declared margin contract.

#### Obligation 7: Current exactify semantics

Verify that GPU exactify:
- enriches medians,
- injects exact extreme,
- does not overwrite retained non-median GPU aggregates that are already present.

**Falsifier:** any change that silently overwrites those aggregates or relies on them as if they were repaired.

#### Obligation 8: Verdict canonicalization

Artifacts loaded through command surfaces must refresh to inferred verdicts.

**Falsifier:** persisted verdict snapshots being treated as authoritative.

#### Obligation 9: Feasibility/smoke harnesses

The feasibility and smoke harnesses remain the operational falsifiers for:
- candidate density,
- overlap adequacy,
- GPU memory budget fit,
- outcome-tensor fit,
- cache reuse,
- end-to-end tuple inspection invariants.

### 10. Dependencies that must be reconciled during consolidation

#### `PR-06` interface dependency

Only one question matters here: the canonical verdict surface must remain inferred-from-supported-metrics. If `PR-06` changes verdict semantics, the third-pass theorem set must be re-derived.

#### `PR-08` interface dependency

Any exact device-summary redesign or lane-collapse exploitation changes the options-specific chunk-plan arithmetic and cache-usage accounting. That is allowed here only as an interface dependency. It is not permission to redesign generic runtime economics.

#### Command/harness dependency

If summary contracts change, then:
- replay provenance checks,
- compare-artifact delta views,
- feasibility preview reports,
- smoke invariants

must be updated in lockstep.

### 11. Kill criteria

Abandon this third-pass result, or re-derive it from scratch, if any of the following become false.

1. `TopOfBookQuote` stops being stress-invariant.
2. `DerivedMidSnapshot` stops sharing aggregate-style semantics with `MinuteAggregate`.
3. `robust_score` changes weights or fallback semantics.
4. sealed finalist ordering changes comparator fields.
5. medians enter ranking or finalist ordering.
6. compile order ceases to be the canonical tie key.
7. persisted `session_id` semantics change materially.
8. GPU mixed-precision screening is used as if exact without a certified interval envelope.
9. a new structure family or fill model is added.
10. any proposal claims exact aggregate-class pruning while still omitting exact extreme and without an interval certificate.

## Final strongest surviving implementation-ready result

The strongest surviving result is not a universal pruning theorem. It is a semantics-stratified exactness contract.

### Exact result A: quote semantics are rank-collapsible

For `TopOfBookQuote`, stress lanes collapse exactly. Therefore:

- current CPU pre-top-\(k\) truncation is exact,
- current GPU pre-top-\(k\) truncation is exact up to mixed-precision error,
- median replay is ranking-irrelevant,
- extreme replay is ranking-irrelevant,
- exact pre-sealed ranking needs only search and holdout baseline lane state,
- exact sealed finalist ordering needs only sealed baseline lane state.

This is the strongest nontrivial exact reduction result that survives all attacks.

### Exact result B: aggregate semantics admit only median-free unconditional reduction

For `MinuteAggregate` and `DerivedMidSnapshot`:

- median replay is always safe to defer,
- exact top-\(k\) pruning is impossible from the current retained summary because exact extreme is absent,
- exact pre-sealed ranking requires exact extreme lane state as well as search/holdout/conservative/harsher state,
- current CPU and GPU provisional truncation are only admissible with an explicit interval certificate.

### Implementation-ready non-exact contract

If a provisional scorer with score \(\widetilde U\) is used before exactification, then every pruning claim must carry a tuple-local interval
\[
[\,\widetilde U(\omega)-B(\omega),\ \widetilde U(\omega)+B(\omega)\,]
\]
and must pass the top-\(k\) interval-separation test
\[
\min_{\omega\in K}(\widetilde U-B)
>
\max_{\eta\notin K}(\widetilde U+B).
\]

For the current GPU path:

- quote class:
  \[
  B(\omega)=\varepsilon_{\mathrm{fp}}(\omega),
  \]
- aggregate class:
  \[
  B(\omega)=\varepsilon_{\mathrm{fp}}(\omega)+0.05\,B_{\mathrm X}(\omega).
  \]

If \(B_{\mathrm X}\) is unavailable, pruning is not exact and must be rejected.

That is the strongest implementation-ready mathematical result that fits the actual repo, stays within `PR-09`, respects the non-goals, and meets the prompt’s acceptance bar.
