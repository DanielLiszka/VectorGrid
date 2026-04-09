# Options Expression Artifact-Summary Coherence — Detailed Implementation Guide

Date: 2026-04-09  
Status: implementation guide derived from the validated portions of Prompt 5 (`options_optimization_depth_direction_FINAL.md`)

## Purpose

This document converts the validated residue of:

- [options_optimization_depth_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_05_optimization_depth_expansion_inside_the_current_pass/options_optimization_depth_direction_FINAL.md)

into one narrow, implementation-ready plan.

The key point is:

- Prompt 5 did **not** justify another broad hot-path metric expansion
- Prompt 5 did justify one bounded reporting-depth pass:
  - make saved-artifact summaries tuple-coherent
  - surface retained leader identity and disagreement explicitly
  - do this from already-saved retained tuple rows

This guide exists to make that work explicit, additive, testable, and easy to verify without reopening:

- GPU-state width
- ranking math
- finalist exactification
- workflow scope
- realism-lane scope

This document is **not** a master consolidation plan for every sibling Markdown file in `options_research/`.
It is a validated implementation tracker for the narrow subset of those documents that legitimately survives into this specific Prompt 5 follow-up pass.

Use this together with:

- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md)
- [options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md)
- [options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md)

Quick pointers:

- objective and allowed scope: [Section 1](#1-objective)
- validated baseline: [Section 2](#2-validated-current-baseline)
- accepted Prompt 5 carry-forward: [Section 3](#3-accepted-carry-forward-from-prompt-5)
- implementation phases: [Section 7](#7-implementation-phases)
- full checklist: [Section 11](#11-implementation-checklist)
- verification plan: [Section 12](#12-verification-plan)

---

## 1. Objective

The next valid Prompt 5 pass should do one thing only:

**make saved-artifact summary surfaces tuple-coherent by attaching compact retained-leader and disagreement facts to artifact summaries, without widening hot-path state or changing ranking behavior.**

That objective breaks down into:

1. **Summary-coherence hardening**
   - stop treating lane-wise summary maxima as if they imply one real tuple
   - expose which retained tuple actually leads each lane
   - expose whether leadership is concentrated or fragmented

2. **Artifact-surface trust hardening**
   - improve artifact list rows
   - improve compare rows
   - improve reopened-artifact header summaries
   - do this without forcing the user to load row-level detail first

3. **No hot-path expansion**
   - do not widen `OptionsGpuTupleAggregate`
   - do not add a replay or exactification pass
   - do not change `robust_score`
   - do not add a new workflow stage

This guide is **not** a metric-expansion guide, **not** a ranking-math guide, and **not** a CUDA/runtime guide.

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not goals.

- [x] The options-expression workflow is already shipped as an artifact-driven optimization + embedded-validation lane.
- [x] Each retained tuple row already stores rich lane-level state, including:
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
  - `pre_sealed_rank`
  - `sealed_final_rank`
- [x] Live and reopened tuple inspection already expose winner/frontier and disagreement logic from full retained rows in:
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [x] `OptionsExpressionArtifactSummary` currently exposes lane-wise maxima and counts, including:
  - `best_robust_score`
  - `best_selection_expectancy_per_premium`
  - `best_holdout_expectancy_per_premium`
  - `best_sealed_expectancy_per_premium`
  - stress-lane maxima
  - validation counts
  - sealed finalist counts
  in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs).
- [x] `summarize_options_expression_artifact(...)` currently computes those maxima independently from `artifact.selection_metrics`, so different summary maxima can already come from different retained tuples.
- [x] Artifact compare and artifact meta-analysis surfaces still consume summary fields rather than full retained rows by default in:
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [x] Prompt 4 is now closed, so Prompt 5 work must preserve the current realism-lane posture:
  - aggregate remains the canonical provider-download lane
  - direct quote-aware provider download remains deferred
  - `DerivedMidSnapshot` remains aggregate-style
  - `TopOfBookQuote` remains the only quote-side execution lane

Important nuance:

- the repo is **not** missing row-level depth
- the repo is **not** missing validation-frontier reporting
- the remaining valid Prompt 5 gap is specifically **artifact-summary coherence**

---

## 3. Accepted Carry-Forward From Prompt 5

Only the following Prompt 5 conclusions are accepted into this guide as implementation work.

### 3.1 Accepted

- [x] Do not do another broad hot-path metric expansion as the default next Prompt 5 move.
- [x] Treat the remaining valid work as summary-coherence hardening, not GPU-state expansion.
- [x] Add a compact retained leader/disagreement summary to artifact summary payloads.
- [x] Derive that compact summary from already-saved retained tuple rows only.
- [x] Use the new summary primarily in:
  - artifact library rows
  - compare rows
  - reopened-artifact summary/header surfaces
- [x] Keep richer row-level chips, badges, and deltas client-side once full retained rows are loaded.
- [x] Preserve current maxima fields for backward compatibility and fast sorting, but stop treating them as decision-grade tuple-coherent summaries.

### 3.2 Not accepted as implementation work

These ideas may appear in the Prompt 5 memo as discussion or deferred alternatives, but they are not work items here:

- widening `OptionsGpuTupleAggregate`
- broad one-pass metric expansion
- exact or approximate GPU-path median restoration
- `robust_score` rewrite
- shrinkage-aware ranking changes
- new exactification or replay pass for report-only fields
- large persisted per-tuple context bundles
- full retained-frontier tables embedded directly into artifact summaries
- new workflow stages
- new realism lanes
- new strategy families

---

## 4. Source Validation Filter

This guide is intentionally narrower than the sibling options documents in this folder.

It should be read as:

- a validated implementation subset
- not a wholesale merger of every idea present in every sibling document
- and not authorization to reopen broader options-roadmap decisions already frozen elsewhere

### Accepted into this guide

- code-grounded current truth from:
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
  - [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- current tuple/frontier reporting and artifact comparison behavior
- aligned supporting context from:
  - [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
  - [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
  - [options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md)
- the validated residue of Prompt 5 in:
  - [options_optimization_depth_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_05_optimization_depth_expansion_inside_the_current_pass/options_optimization_depth_direction_FINAL.md)

### Explicitly excluded

- any statement that turns the Prompt 5 recommendation into “proved highest-ROI truth”
- any hot-path metric expansion
- any ranking-functional rewrite
- any new replay or exactification work
- any new data-plane or realism-lane work
- any work that depends on:
  - CUDA/runtime/planner changes
  - new parent families
  - new structure families
  - new workflow stages

### Practical rule

If a statement is:

- already true in code, mark it as baseline `[x]`
- a bounded summary/reporting hardening task, mark it as remaining `[ ]`
- speculative, broadening, or dependent on another unresolved prompt line, exclude it from this guide

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This guide is additive and non-destructive.

It may add:

- compact leader identity fields
- compact disagreement booleans
- compact finalist-movement facts for one retained leader
- clearer artifact-row and compare-row chips/badges
- narrow backend and frontend tests for summary coherence

It may not remove or narrow:

- current maxima summary fields
- current row-level tuple inspection
- current validation frontier surfaces
- current artifact reopen flow
- current realism-lane posture
- current GPU route

### Explicit non-goals

Do not use this pass to:

- change `robust_score`
- change tuple ordering logic in the engine
- widen GPU scorer outputs
- add a new replay or exactification pass
- persist large per-row context bundles
- build a new options validation stage
- reopen Prompt 4 realism decisions
- change provider-download scope

---

## 6. Target End State

When this pass is done, a user should be able to tell, from the existing options workflow:

- which retained tuple actually leads:
  - selection
  - robust score
  - holdout
  - sealed
  - stress
- whether one tuple leads across most relevant lanes or whether leadership is fragmented
- whether the robust leader survives into sealed finalists and, if so, how it finished
- whether an artifact’s summary is concentrated or mixed before opening row-level detail

And a maintainer should be able to change artifact-summary code without accidentally:

- implying that independent maxima describe one real tuple
- duplicating full tuple payloads into artifact summaries
- moving summary concerns into the GPU/runtime layer
- introducing nondeterministic leader selection

---

## 7. Implementation Phases

## 7.1 Phase 1 — Backend Summary Contract Design

Goal:

add a compact retained-leader/disagreement contract to `OptionsExpressionArtifactSummary`.

### Recommended shape

Use a compact nested summary struct or an equivalently compact flat field set inside:

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)

Preferred semantics:

- `search_leader_tuple_index`
  - retained tuple index with the highest `selection_metrics.expectancy_per_premium`
- `robust_leader_tuple_index`
  - retained tuple index with the highest `robust_score`
- `holdout_leader_tuple_index`
  - retained tuple index with the highest holdout EV, when holdout exists
- `sealed_leader_tuple_index`
  - retained tuple index with the highest sealed EV, when sealed exists
- `stress_leader_tuple_index`
  - retained tuple index with the highest stress EV, when stress exists
- `distinct_leader_count`
  - count of unique retained tuple indices across the present leaders
- `search_vs_robust_disagree`
- `robust_vs_holdout_disagree`, when holdout exists
- `robust_vs_sealed_disagree`, when sealed exists
- `robust_leader_reached_sealed_finalist`
- `robust_leader_sealed_final_rank`, when present

### Deterministic tie-breaking rule

When two retained rows tie on the chosen metric:

1. prefer the lower retained tuple index
2. do not introduce fuzzy tie-break heuristics
3. do not use timestamps or incidental serialization order beyond the existing retained-row order

That keeps summary derivation deterministic and easy to test.

### Scope guard

This contract must remain:

- identity-oriented
- compact
- derived from already-saved rows

It must **not** duplicate full metric payloads already present elsewhere.

---

## 7.2 Phase 2 — Backend Summary Derivation

Goal:

derive the new compact leader/disagreement summary inside `summarize_options_expression_artifact(...)`.

### Required behavior

- compute everything from `artifact.selection_metrics`
- never inspect the full compiled candidate grid
- never trigger replay
- never trigger exactification
- remain `O(retained tuple count)`

### Required invariants

- existing `best_*` maxima remain present
- new leader indices correspond to the same retained row set the maxima were derived from
- absent lanes stay `None` / omitted
- disagreement booleans are only emitted when both compared lanes exist
- `distinct_leader_count` counts only present leader identities

### Strongly recommended helper shape

Add one or two local helper functions in:

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)

for:

- best-retained-row lookup by metric extractor
- compact disagreement summary derivation

Do **not** move this into app-core unless shared ownership becomes clearly necessary.

---

## 7.3 Phase 3 — Frontend Contract Normalization

Goal:

carry the new artifact summary fields through the frontend transport and store layers cleanly.

### Required touch points

- [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)

### Required behavior

- normalize new leader/disagreement fields without changing existing artifact summary semantics
- keep naming consistent with current frontend conventions
- preserve backward compatibility when older artifacts do not have the new fields yet

### Backward-compatibility rule

Old artifacts must still load cleanly.

That means:

- nullable fields normalize to `null` or `undefined`
- disagreement booleans default conservatively
- frontend UI must tolerate absent leader summary data gracefully

---

## 7.4 Phase 4 — Existing UI Surface Hardening

Goal:

use the new compact summary on summary-only surfaces without duplicating the already-rich row-level frontier UI.

### Required touch point

- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

### Required surfaces

1. **Artifact list / artifact library rows**
   - compact chip or label for concentrated vs fragmented leadership
   - optional leader identity shorthand like `Sel #2 · Robust #1`

2. **Artifact compare rows**
   - disagreement badge
   - distinct leader count
   - optional robust-leader finalist movement fact

3. **Reopened-artifact summary/header**
   - compact retained leadership summary before the user opens row-level detail

### UI scope guard

Do **not**:

- clone the full tuple frontier table into the artifact summary area
- create another “validation stage”
- add a large dashboard
- make the header denser than the existing compare/reopen surface can carry

The purpose is summary trust, not UI sprawl.

---

## 7.5 Phase 5 — Narrow Test Hardening

Goal:

lock in the new summary behavior with backend and frontend tests.

### Backend test requirements

In [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs):

- synthetic artifact where:
  - one tuple leads selection
  - another leads robust
  - another leads holdout or sealed
- assert:
  - leader tuple indices
  - disagreement booleans
  - distinct leader count
  - robust-leader finalist movement facts
- assert retained-row-only scope

### Frontend test requirements

In:

- [optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
- [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
- [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)

test:

- summary normalization for new fields
- graceful handling of old artifacts with absent fields
- compare/header rendering of concentrated vs fragmented leadership

---

## 7.6 Phase 6 — Documentation And Routing Sync

Goal:

make the new implementation pass discoverable in the same way as the other options guides.

### Required routing touch point

- [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)

### Optional supporting touch point

- the Prompt 5 final memo, if you want a pointer from the decision document into the guide

This is optional because the implementation guide itself is the authoritative execution tracker.

---

## 8. File Ownership Map

### Primary implementation files

- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - artifact summary contract
  - artifact summary derivation
  - backend tests for leader/disagreement summary

- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
  - transport normalization

- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
  - store exposure and fallback handling

- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - artifact list / compare / reopened-artifact summary rendering

### Primary verification files

- [web/src/services/optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
- [web/src/stores/optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
- [web/src/components/OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)

### Intentionally untouched files

These should stay unchanged for the adopted direction unless a helper extraction becomes clearly necessary:

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

`options_types.rs` is intentionally untouched unless a shared Rust-side summary type becomes unavoidable. That is not the preferred first move.

---

## 9. Acceptance, Rejection, And Stop Rules

## 9.1 Accept the pass if all of the following are true

- new summary fields are derived in `O(retained tuple count)`
- no GPU tensor width changes occur
- no replay or exactification pass is added
- old artifacts still load cleanly
- artifact list / compare / reopened-artifact summary surfaces become clearly more tuple-coherent
- the implementation remains inside the current workflow and realism-lane scope

## 9.2 Reject or roll back any part of the pass if it does any of the following

- widens hot-path GPU state
- changes ranking math
- adds a report-only replay
- duplicates large per-tuple metric bundles into artifact summaries
- makes the UI denser without improving summary trust
- creates a dependency on unresolved Prompt 9 / 10 / 11 work

## 9.3 Stop rule

If the pass starts drifting into:

- metric expansion
- ranking-functional redesign
- large dashboard work
- or another workflow stage

stop and reject the drift. That is outside the valid Prompt 5 residue.

---

## 10. Sign-Off Target

The correct sign-off for this pass is:

- artifact summaries remain backward-compatible
- artifact summary surfaces become tuple-coherent enough to trust before drilldown
- row-level frontier detail remains the richer surface
- GPU/runtime behavior is unchanged
- Prompt 5 closes as a reporting-depth pass, not a hot-path expansion pass

---

## 11. Implementation Checklist

## 11.1 Baseline And Scope Freeze

- [x] Prompt 4 reality is already closed and must remain frozen.
- [x] Current live/reopened tuple frontier reporting already exists.
- [x] Current artifact summaries still use independent lane-wise maxima without retained leader identity.
- [x] Current valid Prompt 5 residue is summary coherence, not hot-path metric expansion.
- [x] Current scope remains additive and must not reopen realism, ranking, or runtime questions.

## 11.2 Backend Contract And Derivation

- [ ] Add compact retained leader/disagreement fields to `OptionsExpressionArtifactSummary`.
- [ ] Implement deterministic retained-row leader lookup helpers in `summarize_options_expression_artifact(...)`.
- [ ] Derive `distinct_leader_count` from present retained leaders only.
- [ ] Derive disagreement booleans only when the compared lanes exist.
- [ ] Preserve existing `best_*` maxima fields unchanged for backward compatibility.
- [ ] Keep all derivation strictly retained-row-only and `O(retained tuple count)`.

## 11.3 Frontend Transport And Store

- [ ] Normalize the new artifact summary fields in `optionsExpressionService.ts`.
- [ ] Thread the new fields through `optionsExpressionStore.ts`.
- [ ] Keep older artifact payloads loading safely with null/undefined fallbacks.

## 11.4 Existing UI Surface Hardening

- [ ] Add compact retained-leader coherence cues to artifact list/library rows.
- [ ] Add disagreement / distinct-leader summary cues to artifact compare rows.
- [ ] Add compact retained-leader summary cues to reopened-artifact header or summary surfaces.
- [ ] Keep row-level chips/frontier detail derived from loaded tuples, not persisted summary bundles.
- [ ] Avoid UI sprawl or duplicate frontier tables.

## 11.5 Tests

- [ ] Add backend unit coverage for synthetic fragmented-leadership artifacts.
- [ ] Add service normalization coverage for the new fields.
- [ ] Add store fallback coverage for missing older-artifact fields.
- [ ] Add panel coverage for fragmented vs concentrated leader rendering.

## 11.6 Verification

- [ ] Run `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`.
- [ ] Run `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`.
- [ ] Run `npm run typecheck`.
- [ ] Run `npm run build`.
- [ ] Run targeted frontend tests for options panel/service/store.
- [ ] Manually compare two saved artifacts with fragmented maxima and confirm the summary now exposes real retained leadership.
- [ ] Reopen one saved artifact and confirm the header/summary cues agree with the actual retained tuple rows.

## 11.7 Documentation And Routing Sync

- [x] Add this guide to `REPO_MAP.md`.
- [ ] Optionally add a pointer from the Prompt 5 final memo to this guide.
- [ ] Mark this guide signed off once the above verification passes.

---

## 12. Verification Plan

Run verification in this order.

### 12.1 Backend verification

Required:

```powershell
cargo test -q -p ta_desktop_demo_app commands_options_expression --lib
```

If helper extraction unexpectedly touches shared Rust-side types:

```powershell
cargo test -q -p ta_app_core options_ --lib
```

### 12.2 CUDA build compatibility

Required:

```powershell
cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel
```

This pass should not affect CUDA logic directly, but the CUDA-enabled app build should stay green.

### 12.3 Frontend verification

Required:

```powershell
npm run typecheck
npm run build
npm run test -- src/components/OptionsExpressionPanel.test.ts src/stores/optionsExpressionStore.test.ts src/services/optionsExpressionService.test.ts
```

### 12.4 Manual artifact verification

Do two quick manual checks:

1. **Saved-artifact compare check**
   - pick two artifacts with fragmented lane maxima
   - confirm the compare surface now shows retained leader identity and disagreement cues

2. **Reopened-artifact coherence check**
   - load one artifact
   - confirm the reopened summary/header cues match the actual retained rows visible after load

---

## 13. Final Sign-Off Template

Use this wording pattern when the pass is complete:

- artifact-summary coherence hardening is implemented and signed off
- retained leader/disagreement summary is now derived from already-saved retained rows
- summary-only artifact surfaces no longer imply a synthetic “best-of-everything” tuple as strongly as before
- existing maxima fields remain for compatibility and fast sorting
- row-level frontier and validation depth stay the richer drilldown surface
- GPU/runtime behavior, ranking math, and realism-lane posture remain unchanged
