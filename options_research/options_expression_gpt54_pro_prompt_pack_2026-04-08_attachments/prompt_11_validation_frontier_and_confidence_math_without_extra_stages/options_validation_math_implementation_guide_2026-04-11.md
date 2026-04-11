# Options Validation Math - Detailed Implementation Guide

Date: 2026-04-11
Status: ready for implementation from the validated Prompt 11 residue; this guide is intentionally limited to the valid, additive, zero-extra-state slice and does not authorize ranking, verdict, schema, or workflow changes

## Purpose

This document converts the valid implementation residue of:

- [options_validation_math_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_11_validation_frontier_and_confidence_math_without_extra_stages/options_validation_math_direction_FINAL.md)

into one narrow, implementation-ready plan for the current repo state.

The key point is:

- Prompt 11 does justify more decision-grade validation interpretation
- Prompt 11 does not justify another validation stage
- Prompt 11 does not justify another ranking functional
- Prompt 11 does not justify changing `inferred_validation_verdict()` or `is_stress_fragile()`
- Prompt 11 does not justify widening backend payloads, artifact summary schemas, or GPU aggregate state
- Prompt 11 does not justify global frontier claims across the discarded search tail

The valid adopted direction is:

- stay frontend-first in the first slice
- stay zero-extra-state
- derive new math from already-retained tuple rows only
- keep the current selected-tuple validation/frontier surfaces and deepen them
- add explicit adjacent-lane drift
- add tri-state monotonicity cues
- add selected-tuple retained-scope regret
- strengthen scope labeling so users can tell what is retained-scope versus sealed-finalist-scope

This guide exists to make that work explicit, additive, testable, and easy to verify without reopening:

- Prompt 10 ranking math
- Prompt 5 artifact-summary coherence
- shortlist or finalist-budget policy
- GPU runtime scope
- backend verdict semantics

Quick pointers:

- validated current baseline: [Section 2](#2-validated-current-baseline)
- accepted Prompt 11 carry-forward: [Section 3](#3-accepted-carry-forward-from-prompt-11)
- final adopted implementation slice: [Section 6](#6-final-adopted-implementation-slice)
- implementation phases: [Section 9](#9-implementation-phases)
- full checklist: [Section 10](#10-implementation-checklist)
- verification plan: [Section 11](#11-verification-plan)

---

## 1. Objective

The next valid Prompt 11 pass should do one thing only:

**deepen selected-tuple validation interpretation using already-retained tuple rows, without changing ranking, verdicting, artifact-summary shape, or workflow scope.**

That objective breaks down into:

1. **Interpretation depth**
   - make current validation lanes easier to reason about
   - show how the selected tuple moves across validation and realism ladders
   - show how far the selected tuple sits behind the visible retained frontier

2. **Scope honesty**
   - make it explicit when a statistic is only true within retained `top_k`
   - make it explicit when a statistic is only true within sealed finalists
   - never present retained-scope math as if it describes the full search

3. **Zero-extra-state discipline**
   - derive from fields already present on retained tuple rows
   - do not add new backend accumulators
   - do not add a new artifact summary family

4. **UI-local posture**
   - keep the first slice centered on the selected tuple detail panel
   - avoid adding global badges or table columns until the selected-tuple version proves useful

This guide is not:

- a confidence-interval guide
- a ranking-functional guide
- a verdict rewrite guide
- a GPU/runtime guide
- a shortlist-budget guide
- an artifact-summary widening guide

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not as goals.

- [x] `OptionsExpressionMetrics` already stores `valid_trades`, `invalid_events`, `invalid_rate`, and can derive total-event and support-rate context in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs#L471).
- [x] `OptionsExpressionTupleResult` already stores the retained lane surfaces needed for Prompt 11, including:
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
- [x] `inferred_validation_verdict()` and `is_stress_fragile()` already exist and should remain the coarse language layer in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs#L584).
- [x] The current selected-tuple panel already exposes support context via `selectedTupleSupportRows` in [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue#L1648).
- [x] The current selected-tuple panel already exposes a validation-frontier table via `selectedTupleValidationFrontierRows` in [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue#L1679).
- [x] The current UI already exposes winner alignment/disagreement surfaces in [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue#L1868).
- [x] Reopened-artifact summary/header surfaces already expose derived metrics, support rows, and coverage rows in:
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue#L1373)
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue#L1399)
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue#L1463)
- [x] The canonical retained-row source for live runs and reopened artifacts is `activeRankedResults` in [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts#L359).
- [x] `OptionsExpressionArtifactSummary` is intentionally compact and scalar-max/count oriented in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs#L76).

Important nuance:

- the repo is not missing validation-frontier reporting
- the repo is not missing support/coverage context entirely
- the remaining valid Prompt 11 gap is deeper interpretation and clearer scope labeling on top of the retained rows the repo already has

---

## 3. Accepted Carry-Forward From Prompt 11

Only the following Prompt 11 conclusions are accepted into this guide as implementation work.

### 3.1 Accepted

- [x] Keep Prompt 11 out of ranking math.
- [x] Keep `robust_score`, tuple ordering, retained `top_k`, sealed-finalist budgeting, and the CPU finalist boundary unchanged.
- [x] Keep `inferred_validation_verdict()` and `is_stress_fragile()` unchanged as the coarse language layer.
- [x] Improve interpretation from already-retained tuple rows rather than adding a new stage.
- [x] Add adjacent-lane drift beyond the existing delta-vs-selection surface.
- [x] Add tri-state monotonicity cues for the validation stress ladders.
- [x] Add selected-tuple retained-scope regret derived from raw retained rows.
- [x] Strengthen explicit scope labels for any new frontier math.
- [x] Treat support/adequacy as a refinement of already-shipped surfaces, not as a wholly new feature family.

### 3.2 Accepted, but only as deferred or secondary

- [x] Selected-tuple retained-scope dominance is mathematically defensible.
- [x] Dominance should be treated as secondary to drift, regret, and scope labeling.
- [x] Dominance should not be part of the minimum required first slice.

### 3.3 Rejected as implementation work

These ideas may appear in the response document, but they are not valid tasks here:

- any change to `robust_score`
- any change to `validation_verdict`
- any change to `inferred_validation_verdict()`
- any change to `is_stress_fragile()`
- any new validation mode or execution stage
- any backend schema widening in the first slice
- any artifact-summary frontier replacement
- any global regret or global dominance claims
- any count-only or pseudo-confidence interval on expectancy
- any reuse of `confidence_tag` as tuple-level validation certainty
- any change to `top_k` or `sealed_finalist_count`

---

## 4. Source Validation Filter

This guide is intentionally narrower than the response document in the same folder.

It should be read as:

- a validated implementation subset
- not a wholesale merger of every idea in the response
- and not authorization to reopen adjacent prompt families already resolved elsewhere

### Accepted into this guide

- code-grounded current truth from:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
  - [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- the valid Prompt 11 residue:
  - adjacent-lane drift
  - monotonicity cues
  - retained-scope regret
  - stronger scope labeling
  - refined support/adequacy presentation

### Explicitly excluded

- anything already fully shipped as Prompt 5 artifact-summary coherence
- anything that depends on summary-only artifact maxima to make tuple-coherent claims
- anything that depends on filtered display rows instead of raw retained rows
- anything that widens backend types or Tauri payloads in the first slice
- dominance as a mandatory first-slice feature

### Practical rule

If a statement is:

- already true in code, mark it as baseline `[x]`
- a narrow selected-tuple interpretation improvement, keep it
- broader, more opaque, or dependent on another unresolved prompt line, exclude or defer it

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This guide is additive and non-destructive.

It may add:

- new frontend-derived drift rows
- new frontend-derived monotonicity cues
- new frontend-derived regret rows
- explicit retained-scope / sealed-scope labels
- tighter wording around support/adequacy context
- focused frontend tests

It may not remove or narrow:

- current selected-tuple validation badges
- current support rows
- current validation frontier table
- current winner alignment/disagreement surfaces
- current artifact summary payloads
- current backend ranking or verdict behavior

### Explicit non-goals

Do not use this pass to:

- change `OptionsExpressionMetrics`
- change `OptionsExpressionTupleResult`
- change `OptionsExpressionArtifactSummary`
- change `summarize_options_expression_artifact(...)`
- change `rank_compiled_tuples_cpu(...)`
- change `gpu_scoring::rank_compiled_tuples_gpu(...)`
- change `exactify_ranked_results(...)`
- change `apply_sealed_finalist_stage(...)`
- change `robust_score(...)`
- add a result-table column
- add an artifact-library row badge based on retained-scope regret
- add a compare-matrix field based on retained-scope regret
- persist derived Prompt 11 math into artifacts in the first slice

---

## 6. Final Adopted Implementation Slice

### 6.1 Minimum required slice

Implement one narrow, selected-tuple interpretation layer in the options-expression panel.

That minimum slice contains:

1. **Refined support/adequacy wording**
   - keep the existing support rows
   - clarify what they mean
   - do not market them as a new feature family

2. **Adjacent-lane drift**
   - selection to holdout
   - holdout to sealed
   - holdout-family stress drift
   - sealed-family stress drift

3. **Tri-state monotonicity cues**
   - `monotone`
   - `non_monotone`
   - `insufficient_lanes`

4. **Selected-tuple retained-scope regret**
   - computed from raw retained rows only
   - clearly scope-labeled

5. **Scope labels**
   - `Within retained top_k only`
   - `Within sealed finalists only`

### 6.2 Deferred, not required in the first slice

The following is valid but not part of the minimum required implementation:

- selected-tuple retained-scope dominance

If dominance is adopted later, it must remain:

- selected-tuple only
- scope-labeled
- derived from raw retained rows
- clearly secondary to drift and regret

### 6.3 Surfaces that should remain unchanged in the first slice

Keep unchanged:

- the result table
- artifact library rows
- artifact comparison matrix
- summary-only artifact header math
- backend payload shape

The first slice should improve the selected-tuple detail experience, not spread partial Prompt 11 math everywhere at once.

---

## 7. Mathematical Definitions To Implement

This section intentionally narrows the response document into exact, first-slice formulas.

### 7.1 Support and event context

For any metrics object `m`:

```text
n(m) = valid_trades(m) + invalid_events(m)
coverage(m) = valid_trades(m) / n(m)
```

Implementation rule:

- derive this from already-loaded tuple metrics only
- use it for explanation and labeling only
- do not treat it as an EV confidence interval

### 7.2 Cross-phase validation drift

These are not stress-ladder comparisons. They are validation promotion comparisons.

If the required adjacent lanes exist:

```text
Delta_sel_to_hold(i) = E_hold(i) - E_sel(i)
Delta_hold_to_sealed(i) = E_sealed(i) - E_hold(i)
```

Implementation rule:

- compute only when both adjacent lanes are present
- omit the row otherwise
- do not rename cumulative deltas as adjacent drift

### 7.3 Holdout-family stress drift

Use only adjacent observed pairs:

```text
Delta_hold_to_stress(i) = E_stress(i) - E_hold(i)
Delta_stress_to_harsher(i) = E_harsher(i) - E_stress(i)
Delta_harsher_to_extreme(i) = E_extreme(i) - E_harsher(i)
```

Implementation rule:

- compute each row only if both lanes exist
- omit absent pairs
- do not backfill missing lanes with cumulative differences

### 7.4 Sealed-family stress drift

Use the sealed analogue:

```text
Delta_sealed_to_stress(i) = E_sealed_stress(i) - E_sealed(i)
Delta_sealed_stress_to_harsher(i) = E_sealed_harsher(i) - E_sealed_stress(i)
Delta_sealed_harsher_to_extreme(i) = E_sealed_extreme(i) - E_sealed_harsher(i)
```

Implementation rule:

- only show these for selected tuples that actually have the relevant sealed lanes

### 7.5 Monotonicity

Define two separate families:

- holdout family:
  - `holdout`
  - `stress`
  - `harsher`
  - `extreme`
- sealed family:
  - `sealed`
  - `sealed_stress`
  - `sealed_harsher`
  - `sealed_extreme`

For each family:

- if fewer than two supported lanes exist, return `insufficient_lanes`
- otherwise return `monotone` when each observed adjacent pair is non-increasing within a tiny floating-point epsilon
- otherwise return `non_monotone`

Implementation rule:

- use a tiny numeric epsilon only to suppress float noise
- do not reuse the `0.03` stress-fragility threshold here
- monotonicity is a stability-shape cue only, not a promotion rule

### 7.6 Regret

For a lane `L`, define:

```text
S_L = { j in activeRows : lane L is supported on tuple j }
regret_L(i) = max_{j in S_L} E_L(j) - E_L(i)
```

First-slice regret rows should be limited to:

- retained holdout regret
- retained stress regret
- sealed regret
- sealed stress regret

Implementation rules:

- compute from raw `activeRows`, never from `displayedRowsWithIndex`
- require the selected tuple to support the lane
- require at least one comparator in the relevant scope
- label retained rows as `Within retained top_k only`
- label sealed rows as `Within sealed finalists only`

### 7.7 Dominance

Dominance is not part of the minimum required slice.

If revisited later, define it separately and keep it selected-tuple only.

---

## 8. Concrete Code Touch Points

### 8.1 Primary implementation file

The primary file for the first slice is:

- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

Relevant existing symbols:

- `selectedTupleValidation`
- `selectedTupleSupportRows`
- `selectedTupleValidationFrontierRows`
- `winnerDisagreementRows`
- `winnerDisagreementSummary`
- `activeRows`

Recommended new computed/helper surfaces in this file:

- `selectedTupleValidationScopeRows`
- `selectedTupleValidationDriftRows`
- `selectedTupleValidationMonotonicityRows`
- `selectedTupleValidationRegretRows`

These names are suggestions, not a required public API.

### 8.2 Store touch points

Primary store source:

- [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts#L359)

Expected change level:

- likely none
- use `activeRankedResults` as the canonical retained-row source
- avoid adding new persistent store state unless the UI truly needs a small display toggle

### 8.3 Service touch points

Primary service file:

- [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)

Expected change level:

- none in the first slice
- no payload normalization changes should be required

### 8.4 Backend non-change points

These files are reference surfaces, not expected first-slice edit targets:

- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)

The guide should stay honest:

- if implementation starts requiring edits there, stop and reassess whether the work has drifted outside the valid Prompt 11 slice

### 8.5 Test touch points

Primary first-slice test files:

- [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)
- [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts), only if a small helper or store-level derivation is introduced

---

## 9. Implementation Phases

### Phase 1 - refine existing support and scope context

Goal:

- make the current support rows and selected-tuple validation area easier to interpret

Tasks:

- keep `selectedTupleSupportRows`
- add explicit scope copy near the new interpretation block
- make it clear that retained-scope math is not global search math

### Phase 2 - add adjacent-lane drift

Goal:

- show how the selected tuple changes across adjacent validation transitions

Tasks:

- implement cross-phase validation drift rows
- implement holdout-family stress drift rows
- implement sealed-family stress drift rows
- omit rows when adjacent pairs do not exist

### Phase 3 - add tri-state monotonicity cues

Goal:

- show whether the observed validation stress ladder degrades smoothly

Tasks:

- implement family-specific monotonicity helpers
- return `insufficient_lanes` when the family is too sparse
- present monotonicity as a cue, not a promotion rule

### Phase 4 - add selected-tuple retained-scope regret

Goal:

- show how far the selected tuple sits behind the visible retained frontier

Tasks:

- compute regret from raw `activeRows`
- keep lane scopes explicit
- avoid filtered-table contamination

### Phase 5 - optional secondary pass for dominance

Goal:

- only revisit if the first slice still leaves real ambiguity

Tasks:

- keep selected-tuple only
- keep scope-labeled
- do not spread dominance into summary-only surfaces

This phase is explicitly optional and should not block completion of the core Prompt 11 slice.

---

## 10. Implementation Checklist

### 10.1 Baseline truths already in place

- [x] Current selected-tuple support rows already exist.
- [x] Current selected-tuple validation-frontier rows already exist.
- [x] Winner alignment/disagreement reporting already exists.
- [x] Reopened-artifact support/coverage surfaces already exist.
- [x] Raw retained rows are already available through `activeRankedResults`.
- [x] Backend metrics already include the counts needed for support/event context.
- [x] Artifact summaries are already intentionally compact and should remain so.

### 10.2 Required implementation tasks

- [ ] Keep the first slice frontend-first and zero-extra-state.
- [ ] Keep ranking, truncation, finalist budgeting, and verdict semantics unchanged.
- [ ] Keep artifact summary schemas unchanged in the first slice.
- [ ] Use `activeRows` / `activeRankedResults` as the canonical source for retained-scope Prompt 11 math.
- [ ] Do not derive new Prompt 11 statistics from `displayedRowsWithIndex` or any filtered/sorted visible subset.
- [ ] Keep the existing validation-frontier table as the baseline surface rather than replacing it.
- [ ] Add explicit cross-phase drift rows for `selection -> holdout` and `holdout -> sealed` when both adjacent lanes exist.
- [ ] Add explicit holdout-family stress drift rows for each observed adjacent pair.
- [ ] Add explicit sealed-family stress drift rows for each observed adjacent pair.
- [ ] Keep the drift rows mathematically adjacent rather than cumulative.
- [ ] Add tri-state monotonicity helpers for the holdout family.
- [ ] Add tri-state monotonicity helpers for the sealed family.
- [ ] Use `insufficient_lanes` rather than treating missing lanes as failure.
- [ ] Add selected-tuple retained holdout regret when holdout metrics exist.
- [ ] Add selected-tuple retained stress regret when stress metrics exist.
- [ ] Add selected-tuple sealed regret when sealed metrics exist.
- [ ] Add selected-tuple sealed stress regret when sealed stress metrics exist.
- [ ] Label every new regret row with the honest scope of the comparison set.
- [ ] Refine support/adequacy wording without pretending it is a wholly new metric family.
- [ ] Add explicit retained-scope / sealed-scope labels near the new interpretation block.
- [ ] Keep the result table unchanged in the first slice.
- [ ] Keep artifact library rows unchanged in the first slice.
- [ ] Keep compare-matrix fields unchanged in the first slice.
- [ ] Keep `optionsExpressionService.ts` normalization unchanged unless implementation proves otherwise.
- [ ] Keep `optionsExpressionStore.ts` changes minimal or avoid them entirely.
- [ ] Keep Rust backend files unchanged unless the work has clearly drifted outside the valid slice.

### 10.3 Deferred or secondary tasks

- [ ] Revisit selected-tuple retained-scope dominance only after the core slice is implemented and reviewed.
- [ ] If dominance is adopted later, keep it selected-tuple only.
- [ ] If dominance is adopted later, keep it scope-labeled and derived from raw retained rows only.
- [ ] If dominance is adopted later, do not add it to summary-only artifact surfaces in the first follow-up.

### 10.4 Required test tasks

- [ ] Add panel tests for cross-phase drift rows.
- [ ] Add panel tests for holdout-family stress drift rows.
- [ ] Add panel tests for sealed-family stress drift rows.
- [ ] Add panel tests for monotonicity states: `monotone`, `non_monotone`, and `insufficient_lanes`.
- [ ] Add panel tests proving regret uses raw retained rows rather than the filtered display table.
- [ ] Add panel tests proving sealed-scope regret only uses sealed finalists.
- [ ] Add panel tests proving scope labels appear anywhere new Prompt 11 math is shown.
- [ ] Add or adjust tests only in store/service layers if a helper truly moves there.

---

## 11. Verification Plan

### Step 1 - frontend unit and component tests

Run at minimum:

```powershell
npm --prefix web run test
```

Target files of interest:

- `web/src/components/OptionsExpressionPanel.test.ts`
- `web/src/stores/optionsExpressionStore.test.ts` only if touched

Primary assertions:

- adjacent drift uses the correct adjacent lane pairs
- monotonicity returns the right tri-state for sparse and full lane families
- regret is derived from raw retained rows, not the visible filtered table
- scope labels are present and unambiguous

### Step 2 - frontend type safety

Run:

```powershell
npm --prefix web run typecheck
```

Expected result:

- no payload-type widening required
- no accidental service/store contract drift

### Step 3 - backend no-regression smoke

Because the valid first slice is frontend-first, backend regression checks are narrow:

```powershell
cargo test -q -p ta_desktop_demo_app commands_options_expression --lib
cargo test -q -p ta_app_core options_ --lib
```

Expected result:

- no backend semantic changes
- no artifact-summary schema changes

### Step 4 - live vs reopened parity smoke

Check both:

- a fresh live options-expression run
- a reopened artifact with the same retained rows loaded

Expected result:

- the same selected tuple yields the same Prompt 11 derived math in both cases

### Step 5 - UX sanity check

Review the selected-tuple panel manually.

The new interpretation layer should:

- add clarity
- not bury the existing frontier table
- not read as a global frontier claim
- not overwhelm the panel with too many badges or rows

---

## 12. Stop Conditions

Stop before implementation if any of the following becomes true.

1. The first slice starts requiring backend payload changes, artifact-summary changes, or GPU aggregate-state changes.
2. The only way to compute the new math is from filtered display rows rather than raw retained rows.
3. The implementation starts changing ranking, truncation, finalist selection, or verdict semantics.
4. The UI cannot clearly distinguish retained-scope claims from sealed-finalist-scope claims.
5. The selected-tuple panel becomes less readable because the new math crowds out the already-useful frontier table.
6. Dominance starts pulling the slice away from the clearer and higher-priority drift/regret interpretation work.

---

## 13. Summary Decision

The valid Prompt 11 implementation direction is:

- implement a selected-tuple, frontend-first, zero-extra-state interpretation layer
- keep the work grounded in raw retained rows
- add adjacent drift, tri-state monotonicity, retained-scope regret, and strong scope labels
- treat support/adequacy as refinement, not as a brand-new feature family
- keep dominance out of the minimum first slice

That is additive, detailed enough to implement directly, and narrow enough to avoid reopening settled backend and CUDA-first decisions.
