# Options Validation Math Direction — Final

Date: 2026-04-11
Status: final consolidation for Prompt 11 (`Validation Frontier And Confidence Math Without Extra Stages`)

## 1. Executive summary

The best next step is **not** another validation stage, another ranking functional, or pseudo-confidence math built from incomplete state.

The best next step is a **frontend-first, zero-extra-state, scope-labeled validation-frontier layer** derived from the tuple metrics the repo already persists:

- exact support / coverage context
- adjacent-lane drift deltas
- monotone stress-deterioration flags
- retained-scope regret
- retained-scope dominance tags

This should sit **next to** the current verdicts and winner/frontier surfaces, not replace them.

The repo should keep all of the following unchanged in this slice:

- `robust_score`
- broad tuple ordering
- `top_k` / `sealed_finalist_count` policy
- `inferred_validation_verdict()` / `is_stress_fragile()` as the coarse language layer
- compact `OptionsExpressionArtifactSummary` scalar-max / count posture
- CPU exactification / sealed finalist replay boundary

The key product rule is scope honesty:

- some statistics are trustworthy only **within retained `top_k`**
- some are trustworthy only **within sealed finalists**
- none of the proposed frontier statistics should be presented as global statements about all tuples in the full search

## 2. Current validation frontier surface

The repo already has a real embedded validation surface.

### Backend tuple surface already present

`OptionsExpressionTupleResult` already carries:

- `selection_metrics`
- `holdout_metrics`
- `sealed_evaluation_metrics`
- `stress_metrics`
- `sealed_stress_metrics`
- `harsher_stress_metrics`
- `sealed_harsher_stress_metrics`
- `extreme_stress_metrics`
- `sealed_extreme_stress_metrics`
- `validation_verdict`
- `robust_score`

Those metrics are built on `OptionsExpressionMetrics`, which already stores:

- `valid_trades`
- `invalid_events`
- `invalid_rate`
- `expectancy_per_trade`
- `expectancy_per_premium`
- `median_return_on_premium`
- `win_rate`
- `total_pnl_per_contract`

### Current verdict logic

The current verdict layer is intentionally coarse and should stay that way.

Let

- `E_base(i)` = strongest validation base lane for tuple `i`
- `E_stress(i)` = strongest available validation stress lane for tuple `i`

Current stress fragility is effectively:

```text
stress_fragile(i) =
  [E_stress(i) < 0]
  OR
  [E_base(i) - E_stress(i) >= 0.03]
```

And current verdict promotion is effectively:

```text
if sealed validation exists:
  sealed_finalist unless stress_fragile
else if holdout validation exists:
  holdout_supported unless stress_fragile
else:
  search_only
```

That is a good coarse user-facing language layer. It is not an adequacy layer, not a frontier geometry layer, and not a confidence interval.

### Current ranking and trust boundary

The repo already makes the most important scope boundary explicit in code, even if the UI does not label it strongly enough yet.

Both CPU and GPU routes:

1. rank tuples broadly
2. truncate to retained `top_k`
3. exactify retained rows
4. run sealed finalist evaluation only inside `sealed_finalist_count`

That means:

- any frontier or dominance statistic over holdout lanes is trustworthy only within retained `top_k`
- any sealed frontier or dominance statistic is trustworthy only within sealed finalists
- nothing in this prompt should pretend to describe the full discarded search tail

### Current UI and artifact surface already present

The current shipped options surface already includes:

- explicit validation verdicts
- selected-tuple validation frontier rows
- validation winners
- winner disagreement reporting
- reopened-artifact summary cues
- compact artifact summary counts such as validated rows and stress-fragile rows

So the remaining gap is **not** “show users more tables.”

The remaining gap is:

- better adequacy context
- better adjacent-lane drift interpretation
- better scope-labeled frontier geometry
- less ambiguity about whether the selected tuple still sits near the visible validated frontier

### Important caveat: `confidence_tag`

`confidence_tag` exists on `OptionsSignalEvent`, not on `OptionsExpressionTupleResult`.

So it is parent/event metadata, not a tuple-confidence estimator. It should not be overloaded into validation certainty for tuples in this prompt.

## 3. Final recommended derived validation statistics

### Winning direction

Adopt one compact package of **tuple-coherent derived validation statistics** computed from already-retained tuple rows.

That package has five parts.

---

### 3.1 Validation support / adequacy context

For any lane `ℓ`, define:

```text
n_ℓ(i) = valid_trades_ℓ(i) + invalid_events_ℓ(i)
coverage_ℓ(i) = valid_trades_ℓ(i) / n_ℓ(i) = 1 - invalid_rate_ℓ(i)
```

Surface, at minimum:

- lane event count `n_ℓ`
- `valid_trades_ℓ`
- `coverage_ℓ`

Recommended display targets:

- strongest validation base lane
- strongest validation stress lane
- sealed base / sealed stress when present

Why this matters:

- the current verdict tells the user **what lane exists**
- support / coverage context tells the user **how much actual evidence is behind it**

This is exact and zero-extra-state.

---

### 3.2 Adjacent-lane drift vector

The current panel already shows delta versus selection. That is useful, but it is incomplete.

Add adjacent-lane drift explicitly.

For tuple `i`, define:

```text
Δ_sel→hold(i)   = E_hold(i)   - E_sel(i)
Δ_hold→sealed(i)= E_sealed(i) - E_hold(i)
Δ_base→stress(i)= E_base(i)   - E_stress(i)
Δ_base→harsher(i)= E_base(i)  - E_harsher(i)
Δ_base→extreme(i)= E_base(i)  - E_extreme(i)
```

with sealed analogues when sealed lanes exist:

```text
Δ_sealed→sealed_stress(i)
Δ_sealed→sealed_harsher(i)
Δ_sealed→sealed_extreme(i)
```

Why this matters:

- users need search-to-holdout drift, not just holdout-versus-selection cells
- users need holdout-to-sealed drift, because sealed is the actual finalist check
- users need base-to-stress drop magnitude in adjacent terms, not only a coarse fragile/not-fragile badge

This is exact and zero-extra-state.

---

### 3.3 Monotone stress-deterioration flags

The repo currently has a strong coarse fragility rule, but not a monotonicity rule.

Add separate monotonicity checks for holdout-family and sealed-family stress ladders.

For holdout-family lanes:

```text
M_hold(i) =
  [E_hold(i) >= E_stress(i)]
  AND [E_stress(i) >= E_harsher(i)]
  AND [E_harsher(i) >= E_extreme(i) - ε]
```

For sealed-family lanes:

```text
M_sealed(i) =
  [E_sealed(i) >= E_sealed_stress(i)]
  AND [E_sealed_stress(i) >= E_sealed_harsher(i)]
  AND [E_sealed_harsher(i) >= E_sealed_extreme(i) - ε]
```

Use a tiny numeric epsilon only to avoid floating-point noise, not to redefine the 0.03 stress-fragility threshold.

Why this matters:

- a tuple that degrades smoothly under harsher realism is easier to trust than a tuple whose lanes bounce around non-monotonically
- non-monotone deterioration is often a better “inspect this more closely” cue than raw drop size alone

This is exact and zero-extra-state.

---

### 3.4 Retained-scope regret

Let `T_hold` be the set of retained tuples with holdout metrics, taken from the raw retained result rows, not the currently filtered/sorted visible table.

Define:

```text
regret_hold(i) = max_{j ∈ T_hold} E_hold(j) - E_hold(i)
regret_stress(i) = max_{j ∈ T_hold} E_stress(j) - E_stress(i)
```

For sealed finalists `F_sealed`, define:

```text
regret_sealed(i) = max_{j ∈ F_sealed} E_sealed(j) - E_sealed(i)
regret_sealed_stress(i) = max_{j ∈ F_sealed} E_sealed_stress(j) - E_sealed_stress(i)
```

Why this matters:

- the current panel can tell the user who wins a lane
- regret tells the user how far the selected tuple sits behind that visible frontier
- this answers the practical question: “Is the search winner still close to the validated winner, or did validation move the frontier materially?”

This is exact and zero-extra-state, but **scope-limited**:

- holdout/stress regret is trustworthy only within retained `top_k`
- sealed regret is trustworthy only within sealed finalists

---

### 3.5 Retained-scope dominance tags

Within retained holdout-capable rows, define a simple dominance relation.

Tuple `a` dominates tuple `b` when:

```text
E_hold(a)      >= E_hold(b)
E_stress(a)    >= E_stress(b)
coverage_hold(a) >= coverage_hold(b)
```

with at least one strict inequality.

Within sealed finalists, use the sealed analogue:

```text
E_sealed(a)         >= E_sealed(b)
E_sealed_stress(a)  >= E_sealed_stress(b)
coverage_sealed(a)  >= coverage_sealed(b)
```

again with at least one strict inequality.

Why this matters:

- regret is scalar and intuitive
- dominance tells the user whether another retained candidate is simply better on the currently visible validated surface
- it stays interpretable because it uses only three dimensions:
  - validated EV
  - stressed EV
  - coverage

This is exact and zero-extra-state, but also **scope-limited**:

- holdout dominance is retained-`top_k` only
- sealed dominance is sealed-finalists only
- it must not mix sealed and non-sealed lanes in one relation

## 4. Zero-extra-state vs extra-state classification

### Adopt now — zero-extra-state

These should be implemented now because they require no new reruns, no new GPU aggregate width, and no new artifact schema in the first slice:

- support / adequacy context from `valid_trades`, `invalid_events`, `invalid_rate`
- adjacent-lane drift deltas
- monotone stress-deterioration flags
- retained-scope regret
- retained-scope dominance tags

### Defer — small extra state may be worthwhile later

These are mathematically legitimate, but they require more state than the current lane guarantees cheaply enough:

- per-lane second moments of return-on-premium or PnL for actual EV uncertainty estimates
- downside moments or semivariance for sharper fragility interpretation
- bounded return sketches / histograms for cheap tail or quantile summaries

These may be worth revisiting only after the zero-extra-state package proves real product value.

### Reject now — misleading or too costly

These should be rejected for this prompt:

- bootstrap or resampling-based confidence intervals
- empirical-Bayes or lower-confidence-bound EV math derived only from current count fields
- pseudo-confidence derived from `confidence_tag`
- artifact-summary frontier statistics built from scalar maxima across different tuples
- any metric whose practical effect is to rescore tuple ordering rather than improve interpretation
- any proposal that needs another heavyweight execution stage

## 5. What remains unchanged

The following should remain unchanged in this slice.

### Verdict logic stays coarse

Keep:

- `inferred_validation_verdict()`
- `is_stress_fragile()`

Treat them as the language layer.

The new statistics should **complement** them, not silently rewrite them.

### Ranking stays fixed

Keep:

- `robust_score`
- broad ranking order
- retained `top_k` truncation
- sealed finalist promotion budget
- CPU exactification and finalist evaluation boundary

Prompt 10 is already closed. This prompt should not sneak in ranking changes.

### Artifact summary stays compact

Keep `OptionsExpressionArtifactSummary` compact and scalar-max/count oriented in the first slice.

Do **not** try to encode tuple-coherent frontier geometry into artifact-summary maxima.

If later product review wants compact counts such as:

- monotone validated tuple count
- retained-dominated tuple count

those can be revisited later. They are not needed for the first slice.

### No new workflow branch

Do not create:

- a new validation mode
- a new heavyweight report family
- a new backend replay stage

This is an interpretation/depth improvement inside the current embedded validation surface.

## 6. Deferred ideas

The following ideas are reasonable, but should wait.

### A. Backend-owned helper fields

If the frontend-first slice proves useful, the repo may later promote some derived fields into backend-owned tuple helpers for artifact/export parity.

Candidates:

- canonical support rows
- canonical adjacent-lane drift values
- canonical monotonicity flags

This is not required now.

### B. Compact summary counts

If reopened-artifact compare surfaces later need them, consider a small number of new compact counts in `OptionsExpressionArtifactSummary`, such as:

- validated-and-monotone tuple count
- retained non-dominated holdout tuple count

Only do this if the UI proves that compact summary counts help real decisions.

### C. Real uncertainty math after extra state exists

If the repo later adds mergeable second-moment or bounded-sketch state, revisit:

- EV uncertainty
- downside dispersion
- stronger acceptance thresholds

Until then, defer.

## 7. Rejected ideas

### A. Bootstrap / rerun-heavy confidence math

Rejected because it violates the runtime philosophy for this prompt and creates the appearance of rigor without a compelling product payoff.

### B. Count-only “confidence” on expectancy

Rejected because option-expression payoffs are asymmetric. `valid_trades`, `invalid_events`, and `invalid_rate` alone do not justify an EV confidence interval.

### C. Reusing `confidence_tag` as tuple confidence

Rejected because `confidence_tag` is event-level metadata, not tuple-level validation state.

### D. Artifact-summary frontier replacement

Rejected because `OptionsExpressionArtifactSummary` intentionally mixes scalar maxima that may come from different tuples. That is useful for compact summary cues, not for tuple-coherent frontier geometry.

### E. Changing shortlist or finalist policy in this prompt

Rejected for now.

The right fix is better scope labeling on the frontier the user can actually see, not widening `top_k` or `sealed_finalist_count` here.

### F. Global dominance or global regret claims

Rejected because the code truncates before exactification and sealed evaluation.

## 8. Concrete code touch points by file and symbol

### `crates/app_core/src/options_types.rs`

Primary symbols:

- `OptionsExpressionMetrics`
- `OptionsExpressionTupleResult`
- `OptionsExpressionTupleResult::strongest_validation_metrics()`
- `OptionsExpressionTupleResult::strongest_validation_stress_metrics()`
- `OptionsExpressionTupleResult::inferred_validation_verdict()`
- `OptionsExpressionTupleResult::is_stress_fragile()`
- `OPTIONS_VALIDATION_STRESS_DROP_THRESHOLD`

Recommended first-slice change:

- **no required schema widening**
- optional tiny helper functions only if the team wants one canonical backend/shared definition for:
  - support ratio
  - holdout-family monotonicity
  - sealed-family monotonicity

Recommended non-change:

- do not rewrite verdict semantics here
- do not add pseudo-confidence fields here yet

### `crates/app_core/src/options_expression.rs`

Primary symbols:

- `rank_compiled_tuples_cpu()`
- `gpu_scoring::rank_compiled_tuples_gpu()`
- `exactify_ranked_results()`
- `apply_sealed_finalist_stage()`
- `finalize_ranked_results()`
- `robust_score()`

Recommended first-slice change:

- none required for the winning direction

Optional later change:

- backend-shared monotonicity helpers if frontend/backend parity becomes important

Recommended non-change:

- keep ranking, truncation, exactification, and sealed evaluation shape unchanged

### `src-tauri/src/commands_options_expression.rs`

Primary symbols:

- `OptionsExpressionArtifactSummary`
- `summarize_options_expression_artifact()`

Recommended first-slice change:

- none required

Optional later change:

- add a very small number of compact summary counts only if the UI later proves they are needed

Recommended non-change:

- do not widen summary maxima into tuple-coherent frontier proxies

### `web/src/components/OptionsExpressionPanel.vue`

Primary existing symbols / computed surfaces:

- `selectedTupleValidationFrontierRows`
- `validationWinnersRows`
- `winnerDisagreementRows`
- `winnerDisagreementSummary`
- `activeRows`

This is the primary file for the winning slice.

Recommended additions:

- support / adequacy presentation next to the selected tuple frontier
- adjacent-lane drift rows in the selected tuple validation panel
- monotonicity badges for holdout-family and sealed-family stress ladders
- retained-scope regret for the selected tuple against the raw retained tuple set
- retained-scope dominance status for the selected tuple
- explicit scope labels such as:
  - `Within retained top_k only`
  - `Within sealed finalists only`

Important implementation rule:

- regret and dominance must be computed from `activeRows`
- they must **not** be computed from `displayedRowsWithIndex` or any filtered/sorted visible subset

### `web/src/services/optionsExpressionService.ts`

Primary symbols:

- `OptionsExpressionTupleSummary`
- `OptionsExpressionArtifactSummary`
- normalization helpers for tuple and artifact summary payloads

Recommended first-slice change:

- none required if the slice stays frontend-derived

Optional later change:

- normalize any backend-owned helper fields if the repo later promotes them out of frontend-only derivation

### `web/src/stores/optionsExpressionStore.ts`

Primary symbols:

- `activeRankedResults`

Recommended first-slice change:

- likely none beyond any UI toggle state the panel may want
- use the existing raw retained results getter as the canonical source for retained-scope frontier math

## 9. Verification order

### Step 1 — backend helper tests only if helpers are added

If the repo adds optional helper functions in `options_types.rs`, add narrow unit tests for:

- support / coverage calculation
- holdout monotonicity true/false cases
- sealed monotonicity true/false cases
- missing-lane handling

### Step 2 — frontend-derived math tests

Add or extend panel/service/store tests so they prove:

- adjacent drift values use the correct lane pairs
- regret is computed from raw retained rows, not the filtered display table
- sealed regret/dominance is computed only inside sealed finalists
- holdout dominance does not mix sealed metrics
- scope labels appear anywhere regret/dominance is shown

### Step 3 — no-regression behavioral checks

Verify all of the following remain unchanged:

- tuple ordering
- `pre_sealed_rank`
- `sealed_final_rank`
- `validation_verdict`
- saved artifact summary fields in the first slice

### Step 4 — performance sanity check

Because the winning direction is zero-extra-state and frontend-first, the performance check is intentionally narrow:

- no new rerun
- no new exactification pass
- no measurable preview/run-time regression beyond noise
- no artifact-size increase in the first slice

### Step 5 — reopen parity

Verify that reopened artifacts and fresh runs derive the same frontier math from the same retained tuple rows.

## 10. Interpretability risks

### Risk 1 — scope leakage

The biggest risk is users reading retained-scope frontier math as if it were global.

Mitigation:

- put scope labels directly next to regret/dominance outputs
- never bury scope limits in tooltip-only copy

### Risk 2 — false rigor from small samples

Even exact coverage and regret can look more authoritative than they are when counts are tiny.

Mitigation:

- always pair frontier math with event count and valid-trade count
- avoid “confidence” language in the first slice

### Risk 3 — dominance that feels opaque

Dominance becomes hard to trust when too many dimensions are mixed together.

Mitigation:

- keep the dominance relation simple:
  - validated EV
  - stressed EV
  - coverage
- keep sealed and non-sealed dominance separate

### Risk 4 — monotonicity mistaken for quality

A tuple can deteriorate monotonically and still be bad.

Mitigation:

- present monotonicity as a stability shape cue, not as a promotion rule

### Risk 5 — filtered-table contamination

If regret or dominance is accidentally computed from the visible filtered table, the numbers become misleading.

Mitigation:

- derive from raw retained rows only
- cover this explicitly with tests

## 11. Stop conditions

Stop before implementation if any of the following becomes true.

1. The proposed slice stops being zero-extra-state and starts requiring new reruns, raw-path replay, or new GPU aggregate width.
2. The only persuasive next statistic is an EV confidence interval, but the repo still lacks second moments or bounded sketches to support it honestly.
3. The implementation begins to change ranking, truncation, finalist selection, or verdict semantics instead of only improving interpretation.
4. The UI cannot clearly and consistently label retained-scope versus finalist-scope frontier math.
5. The only way to compute the new statistics is from filtered display rows or artifact-summary maxima rather than raw retained tuple rows.
6. Review testing shows that the added math creates more user confusion than decision value.

## Source basis

This recommendation is grounded in the current prompt-pack and live repo surfaces, especially:

- `options_expression_gpt54_pro_prompt_pack_2026-04-08.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`
