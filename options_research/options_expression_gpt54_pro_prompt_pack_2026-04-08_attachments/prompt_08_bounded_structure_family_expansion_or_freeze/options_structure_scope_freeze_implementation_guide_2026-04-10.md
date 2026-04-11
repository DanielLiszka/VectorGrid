# Options Structure Scope Freeze - Detailed Implementation Guide

Date: 2026-04-10
Status: implemented and signed off on 2026-04-10 from the valid portions of Prompt 8 (`options_structure_scope_direction_FINAL (1).md`)

## Purpose

This document converts the validated residue of:

- [options_structure_scope_direction_FINAL (1).md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_08_bounded_structure_family_expansion_or_freeze/options_structure_scope_direction_FINAL%20(1).md)

into one narrow, implementation-ready plan.

The key point is:

- Prompt 8 did not justify a fourth fixed-geometry structure family
- Prompt 8 did not justify a broader bounded-family expansion program in the current cycle
- Prompt 8 did not justify any movement toward generic multi-leg breadth
- Prompt 8 did justify freezing the current structure-family boundary and making that decision explicit in tracking and routing if desired

This guide exists to make that outcome explicit, additive, and easy to verify without reopening:

- the current three-family backend contract
- the current closed frontend selector surface
- the current artifact and provenance shape
- the current unified History follow-up host
- the current performance-first roadmap posture

This document is not a master consolidation plan for every options-expression Markdown file in `options_research/`.
It is a validated implementation tracker for the narrow subset of Prompt 8 that legitimately survives into follow-up work.

Use this together with:

- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md)
- [options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md)
- [options_expression_support_aware_validation_semantics_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_support_aware_validation_semantics_implementation_guide_2026-04-10.md)
- [options_stress_scope_freeze_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_07_stress_family_expansion_constraints/options_stress_scope_freeze_implementation_guide_2026-04-10.md)

Quick pointers:

- objective and allowed scope: [Section 1](#1-objective)
- validated current baseline: [Section 2](#2-validated-current-baseline)
- accepted Prompt 8 carry-forward: [Section 3](#3-accepted-carry-forward-from-prompt-8)
- implementation phases: [Section 7](#7-implementation-phases)
- full checklist: [Section 11](#11-implementation-checklist)
- reopen criteria: [Section 12](#12-reopen-criteria)

---

## 1. Objective

The valid Prompt 8 follow-up should do one thing only:

**freeze the generic options structure-family surface at the current three-family boundary while preserving the current closed evaluator shape and unified follow-up workflow.**

That objective breaks down into:

1. **Structure-family boundary freeze**
   - keep `long_single`, `debit_vertical`, and `long_pair`
   - do not add a fourth fixed-geometry family now
   - do not carry family expansion as active near-term roadmap work

2. **Scope separation**
   - keep any future ladder-style or additional debit-family ideas dormant unless the repo explicitly reopens structure breadth later
   - keep generic multi-leg, mixed-expiry, rolling, assignment, and scripting work out of scope

3. **Implementation discipline**
   - avoid widening runtime, schema, UI, and reporting surfaces without concrete decision value
   - keep the current options workflow focused on deeper optimization, embedded validation, realism interpretation, and preserved performance

This guide is not a runtime expansion plan.
It is a scope-control and follow-through document for an already-existing design boundary.

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not goals.

- [x] The current shipped structure-family set is `long_single`, `debit_vertical`, and `long_pair` in [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md).
- [x] The roadmap already freezes new structure families as non-active near-term work in [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md).
- [x] The viable plan already constrains v1 to `long_single` plus bounded same-expiry fixed-geometry two-leg families in [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md).
- [x] `OptionsStructureFamily` is currently a closed three-variant enum in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] The evaluator still encodes structure semantics through explicit family branches in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] `SelectedCandidate` is still fundamentally modeled as primary leg plus optional companion leg in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] Artifact summaries already surface structure family as bounded provenance in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs).
- [x] The frontend store and panel already expose a closed three-option selector in [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts) and [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue).
- [x] The current host remains the unified History follow-up panel rather than a separate options workflow branch.

Important nuance:

- the repo does preserve a dormant future expansion envelope in the viable plan
- but that is not active implementation scope
- and the roadmap now clearly prioritizes depth inside the current lane over more expression-family breadth

That nuance matters because Prompt 8 is valid only if it preserves the freeze as the active direction.

---

## 3. Accepted Carry-Forward From Prompt 8

Only the following Prompt 8 conclusions are accepted into this guide as implementation-relevant work.

### 3.1 Accepted

- [x] Freeze structure-family breadth now.
- [x] Do not add a fourth fixed-geometry structure family now.
- [x] Keep `long_single`, `debit_vertical`, and `long_pair` as the current shipped structure-family set.
- [x] Keep the evaluator shape closed and explicitly branched rather than moving toward a generic leg graph.
- [x] Keep one structure family per job.
- [x] Keep same-expiry first-wave multi-leg support.
- [x] Keep deterministic strike-offset geometry from a base selector.
- [x] Keep same entry timestamp across legs in a bundle.
- [x] Keep rolling, assignment, exercise, generic multi-leg scripting, and new workflow branches out of scope.
- [x] Keep implementation effort focused on depth inside the current lane rather than breadth.

### 3.2 Not accepted as implementation work

These ideas may appear in the Prompt 8 memo as discussion or future possibilities, but they are not work items here:

- a fourth fixed-geometry family
- a broader bounded-family expansion program
- a generic multi-leg abstraction layer
- mixed-expiry calendar or diagonal work
- rolling logic
- assignment or exercise modeling
- a separate options workflow branch
- frontend pre-wiring for dormant future structure families

---

## 4. Source Validation Filter

This guide is intentionally narrower than the sibling options documents in this folder.

It should be read as:

- a validated implementation subset
- not a wholesale merger of every idea present in every sibling document
- and not authorization to widen structure families just because a future note mentions a possible ladder-style reopen candidate

Folder-local note:

- this folder currently contains the Prompt 8 final decision memo and this implementation guide only
- therefore the folder-local validation question is whether this guide faithfully implements the valid residue of that final memo without widening scope

### Accepted into this guide

- code-grounded current truth from:
  - [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
  - [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- aligned supporting context from:
  - [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
  - [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- the validated residue of Prompt 8 in:
  - [options_structure_scope_direction_FINAL (1).md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_08_bounded_structure_family_expansion_or_freeze/options_structure_scope_direction_FINAL%20(1).md)

### Explicitly excluded

- any argument that widens scope without concrete decision value
- any move to add a fourth family now
- any attempt to treat dormant ladder-style expansion as active roadmap work
- any change that would widen runtime branches, artifact summaries, or UI selectors for another family
- any move toward generic multi-leg scripting or a separate options branch

### Practical rule

If a statement is:

- already true in code, mark it as baseline `[x]`
- an optional documentation or routing clarification that makes the freeze more explicit, mark it as remaining `[ ]`
- speculative or dependent on a future scope reopen, exclude it from this guide

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This guide is additive and non-destructive.

It may add:

- explicit documentation language that the current structure-family surface is frozen
- routing pointers so future contributors do not casually reopen the structure-family question
- narrow future-reopen criteria

It may not remove or narrow:

- current `long_single`, `debit_vertical`, or `long_pair` functionality
- current artifact provenance fields
- current tuple inspection or event-path replay coverage for the shipped families
- current unified History follow-up host
- current CPU oracle / fallback behavior

### Explicit non-goals

Do not use this pass to:

- add a fourth structure family
- widen `OptionsStructureFamily`
- widen explicit evaluator branches for another family
- redesign `SelectedCandidate` into a generic leg-graph abstraction
- widen artifact summaries for new structure-family breadth
- widen frontend selectors, copy, or table vocabulary for another family
- create a separate options workflow branch
- move dormant future ladder ideas into active implementation work

---

## 6. Target End State

When this pass is complete, a maintainer should be able to tell, without ambiguity:

- the generic options structure-family boundary is currently frozen
- the current shipped structure-family set remains `long_single`, `debit_vertical`, and `long_pair`
- the evaluator remains a closed explicit-family surface rather than a generic multi-leg engine
- dormant ladder-style or additional debit-family ideas do not belong to active implementation unless the scope is explicitly reopened
- future work should spend decision-value budget elsewhere before adding another structure family

And a contributor should not be able to accidentally justify runtime widening by pointing at Prompt 8 alone.

---

## 7. Implementation Phases

## 7.1 Phase 1 - Decision Capture In This Folder

Goal:

record the validated Prompt 8 outcome in a checklist-backed implementation guide.

### Required touch point

- this file in `prompt_08_bounded_structure_family_expansion_or_freeze/`

### Required behavior

- capture the valid result as a structure-family freeze
- explicitly state that no runtime expansion is recommended now
- separate valid carry-forward from deferred or rejected ideas

---

## 7.2 Phase 2 - Optional Roadmap Language Sync

Goal:

make the structure-family freeze explicit in the standing options roadmap if stronger discoverability is desired.

### Candidate touch point

- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)

### Allowed additions

- sharpen wording that the current practical structure-family set is frozen
- clarify that `long_single`, `debit_vertical`, and `long_pair` remain the active shipped set
- clarify that any future structure-family reopen remains subordinate to depth work and performance preservation

### Scope guard

Do not use this phase to rewrite the roadmap around a new family.

---

## 7.3 Phase 3 - Optional Repo Routing Sync

Goal:

make the Prompt 8 freeze decision discoverable from repo routing material.

### Candidate touch point

- [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)

### Allowed additions

- a short routing note pointing options contributors to this guide when structure-family expansion questions arise

### Scope guard

Do not turn `REPO_MAP.md` into a second roadmap document.

---

## 7.4 Phase 4 - No Runtime Expansion

Goal:

leave runtime, schema, and UI contracts unchanged for Prompt 8.

### Files intentionally untouched for the adopted direction

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

### Required behavior

- do not widen the structure-family enum, selector, artifact summary, or evaluator logic
- do not change runtime pricing, candidate selection, tuple inspection, or event-path replay behavior for the shipped families

---

## 8. File Ownership Map

### Primary implementation files for this guide

- [options_structure_scope_direction_FINAL (1).md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_08_bounded_structure_family_expansion_or_freeze/options_structure_scope_direction_FINAL%20(1).md)
- [options_structure_scope_freeze_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_08_bounded_structure_family_expansion_or_freeze/options_structure_scope_freeze_implementation_guide_2026-04-10.md)

### Optional supporting routing/docs files

- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)

### Future reopen touch points only

These are not current implementation targets.
They matter only if the structure-family question is explicitly reopened later.

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

---

## 9. Acceptance, Rejection, And Stop Rules

## 9.1 Accept the pass if all of the following are true

- the current structure-family boundary is documented clearly enough to avoid accidental scope drift
- no runtime, schema, or UI widening was introduced for Prompt 8
- the current shipped family set remains unchanged
- dormant ladder-style or additional debit-family ideas remain outside active implementation scope

## 9.2 Reject any part of the pass if it does any of the following

- adds a fourth structure family
- widens the evaluator into a more generic leg abstraction
- widens artifact summaries or frontend selectors for another family
- reframes structure-family expansion as frontend or workflow work
- moves dormant future ladder-style ideas into active scope

## 9.3 Stop rule

If work starts drifting into:

- new family design
- generic multi-leg runtime design
- workflow-branch creation
- mixed-expiry or rolling semantics

stop and reject the drift.
That is outside the valid Prompt 8 residue.

---

## 10. Sign-Off Target

The correct sign-off for this pass is:

- the generic options structure-family boundary is frozen
- no fourth structure family is being implemented
- the current shipped set remains `long_single`, `debit_vertical`, and `long_pair`
- current backend, artifact, and frontend surfaces remain unchanged for Prompt 8
- any future family expansion must clear the reopen criteria in this guide
- full Prompt 8 implementation means policy capture and any chosen doc-routing sync only, not new runtime work

---

## 11. Implementation Checklist

## 11.1 Validated Current Baseline

- [x] The shipped structure-family set is `long_single`, `debit_vertical`, and `long_pair`.
- [x] The roadmap already treats broader structure-family expansion as non-active near-term work.
- [x] The viable plan already constrains v1 to `long_single` plus bounded same-expiry fixed-geometry two-leg families.
- [x] `OptionsStructureFamily` is currently a closed three-variant enum.
- [x] The evaluator still branches explicitly by structure family.
- [x] The frontend selector/store surface is currently closed to the three shipped families.
- [x] Structure family already exists as bounded provenance in saved artifacts and current UI surfaces.

## 11.2 Adopted Prompt 8 Decisions

- [x] Freeze structure-family breadth now.
- [x] Do not add a fourth fixed-geometry family now.
- [x] Keep `long_single`, `debit_vertical`, and `long_pair` as the active shipped set.
- [x] Keep the evaluator shape closed rather than moving toward a generic leg graph.
- [x] Keep rolling, assignment, exercise, generic multi-leg scripting, and new workflow branches out of scope.
- [x] Keep implementation effort focused on depth inside the current options lane.

## 11.3 Current Implementation Scope

- [x] Prompt 8 does not require backend runtime changes now.
- [x] Prompt 8 does not require enum or tuple schema changes now.
- [x] Prompt 8 does not require artifact summary changes now.
- [x] Prompt 8 does not require frontend selector or table expansion now.
- [x] Prompt 8 does not require GPU-path widening now.

## 11.4 Remaining Policy-Sync Decisions

- [x] Create this implementation guide in the Prompt 8 folder.
- [x] Sharpen [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md) so the structure-family freeze is more explicit.
- [x] Add a short routing pointer in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md) for future structure-family questions.
- [x] Add a backlink from [options_structure_scope_direction_FINAL (1).md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_08_bounded_structure_family_expansion_or_freeze/options_structure_scope_direction_FINAL%20(1).md) to this implementation guide.

## 11.5 Explicit Non-Work Items

- [x] Do not add another structure-family enum variant now.
- [x] Do not widen explicit evaluator branches for another family now.
- [x] Do not redesign `SelectedCandidate` into a generic multi-leg abstraction now.
- [x] Do not widen artifact summary provenance fields for another structure family now.
- [x] Do not widen frontend selector vocabulary or reporting surfaces for another family now.

---

## 12. Reopen Criteria

Do not reopen Prompt 8 unless all of the following become true:

1. **Priority criterion**
   - the current higher-ROI backlog in optimization depth, embedded validation depth, realism interpretation, and performance preservation has been substantially exhausted first

2. **Shape criterion**
   - the candidate family remains fixed-geometry, same-expiry in first-wave support, deterministic from a base selector, one-family-per-job, and free of rolling, assignment, and exercise semantics

3. **Runtime-fit criterion**
   - the candidate fits the current evaluator shape without forcing a generic leg-engine rewrite

4. **Economics criterion**
   - a dedicated feasibility spike shows that the candidate preserves the current promotion envelope for density, invalid-rate, cache reuse, and GPU scorer speedup

5. **Decision-value criterion**
   - the candidate materially changes robust finalists, holdout or sealed winners, or useful stress/validation outcomes beyond what the current three-family set already provides

If those conditions are not met, the correct decision remains:

> keep the current structure-family surface frozen and spend the next decision-value dollar elsewhere.

---

## 13. Verification Plan

### 13.1 For the adopted Prompt 8 direction

No runtime verification is required, because the validated outcome is not a runtime expansion.

Minimum acceptable verification for this guide is:

- confirm the repo already exposes the current three-family structure surface described above
- confirm the guide does not authorize new runtime work that exceeds the validated Prompt 8 residue
- confirm the only remaining unchecked items are policy-sync decisions rather than hidden backend or frontend implementation work

### 13.2 If Prompt 8 is explicitly reopened later

Use this order:

1. type and runtime fit check
2. candidate density and invalid-slot check
3. structural and summary cache reuse check
4. GPU-economics check
5. artifact-summary and frontend review only after backend proof exists

Representative commands and gates should be taken from the current viable plan and Phase 0.5 tooling rather than invented ad hoc during the reopen.

---

## 14. Final Sign-Off Template

Use this wording pattern if the remaining optional doc-sync items are completed:

- Prompt 8 is signed off as a structure-family scope-freeze decision
- no fourth structure family is being implemented
- the current shipped set remains `long_single`, `debit_vertical`, and `long_pair`
- the current evaluator shape remains a closed explicit-family surface
- any future family expansion requires a separate evidence-backed reopen
