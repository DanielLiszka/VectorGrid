# Options Stress Scope Freeze - Detailed Implementation Guide

Date: 2026-04-10
Status: implemented and signed off on 2026-04-10 from the valid portions of Prompt 7 (`options_stress_scope_direction_FINAL.md`)

## Purpose

This document converts the validated residue of:

- [options_stress_scope_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_07_stress_family_expansion_constraints/options_stress_scope_direction_FINAL.md)

into one narrow, implementation-ready plan.

The key point is:

- Prompt 7 did not justify a fifth generic stress family
- Prompt 7 did not justify promoting `extreme` into a new always-on broad compiled lane
- Prompt 7 did not justify treating quote-aware spread or liquidity shock as Prompt 7 stress-family work
- Prompt 7 did justify freezing the current stress-family boundary and making that decision explicit in tracking/docs if desired

This guide exists to make that outcome explicit, additive, and easy to verify without reopening:

- GPU-first broad-path design
- current artifact schema
- current reopened-artifact surfaces
- current frontend reporting surfaces
- current validation-depth and realism-lane scope boundaries

This document is not a master consolidation plan for every options-expression Markdown file in `options_research/`.
It is a validated implementation tracker for the narrow subset of Prompt 7 that legitimately survives into follow-up work.

Use this together with:

- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md)
- [options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md)
- [options_expression_support_aware_validation_semantics_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_support_aware_validation_semantics_implementation_guide_2026-04-10.md)

Quick pointers:

- objective and allowed scope: [Section 1](#1-objective)
- validated current baseline: [Section 2](#2-validated-current-baseline)
- accepted Prompt 7 carry-forward: [Section 3](#3-accepted-carry-forward-from-prompt-7)
- implementation phases: [Section 7](#7-implementation-phases)
- full checklist: [Section 11](#11-implementation-checklist)
- reopen criteria: [Section 12](#12-reopen-criteria)

---

## 1. Objective

The valid Prompt 7 follow-up should do one thing only:

**freeze the generic options stress-family surface at the current boundary while preserving the current GPU-first broad-path shape.**

That objective breaks down into:

1. **Stress-family boundary freeze**
   - keep `baseline`, `conservative`, and `harsher` as the broad compiled robustness lanes
   - keep `extreme` as the retained-tuple / sealed-finalist exact lane
   - do not add a fifth generic lane now

2. **Scope separation**
   - keep quote-aware spread or liquidity shock ideas out of Prompt 7 generic stress-family work
   - route those ideas, if reopened later, through realism-lane or validation-depth scope instead

3. **Implementation discipline**
   - avoid widening runtime, schema, UI, and reporting surfaces without concrete decision value
   - keep the current options workflow focused on useful robustness work that fits the current tensor/runtime economics

This guide is not a new runtime plan.
It is a scope-control and follow-through document for an already-existing design boundary.

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not goals.

- [x] The current broad compiled stress path already covers `baseline`, `conservative`, and `harsher` in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] `extreme` already exists as an exact retained-tuple / sealed-finalist stress lane rather than a broad always-on compiled lane in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] Tuple rows already persist separate stress, harsher-stress, and extreme-stress outputs in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Artifact summaries already expose separate stress, harsher-stress, and extreme-stress fields in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs).
- [x] Frontend reopen and tuple surfaces already understand and render stress, harsher-stress, and extreme-stress fields in [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts) and [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue).
- [x] Current validation semantics already consume strongest stressed metrics rather than requiring a new stress-lane family to become useful in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] The current options workflow already has sufficient generic stress-family breadth for present scope.

Important nuance:

- the repo does expose all four named robustness surfaces
- but the always-broad compiled path is effectively `baseline`, `conservative`, and `harsher`
- while `extreme` is intentionally attached later at retained-tuple / sealed-finalist evaluation

That nuance matters because Prompt 7 is valid only if it preserves that split.

---

## 3. Accepted Carry-Forward From Prompt 7

Only the following Prompt 7 conclusions are accepted into this guide as implementation-relevant work.

### 3.1 Accepted

- [x] Freeze the generic stress-family set now.
- [x] Do not add a fifth generic stress lane.
- [x] Do not add a "slightly harsher than harsher" lane for symmetry.
- [x] Keep `baseline`, `conservative`, and `harsher` as the broad compiled robustness lanes.
- [x] Keep `extreme` as the retained-tuple / sealed-finalist exact lane.
- [x] Keep quote-aware spread or liquidity shock ideas out of Prompt 7 generic stress-family expansion.
- [x] Treat any later quote-aware or provider-semantic-specific shock work as realism-lane or validation-depth work instead.
- [x] Keep implementation effort focused on decision value rather than lane proliferation.

### 3.2 Not accepted as implementation work

These ideas may appear in the Prompt 7 memo as discussion or future possibilities, but they are not work items here:

- a fifth generic stress family
- broad compiled promotion of `extreme`
- a new stress lane added only for symmetry or visual completeness
- quote-aware liquidity or spread shock folded into the generic stress-family set
- widened tuple schema or artifact summary schema for another generic lane
- widened frontend reporting surfaces for a new generic stress family

---

## 4. Source Validation Filter

This guide is intentionally narrower than the sibling options documents in this folder.

It should be read as:

- a validated implementation subset
- not a wholesale merger of every idea present in every sibling document
- and not authorization to widen stress families just because a future note mentions a hypothetical lane

Folder-local note:

- this folder currently contains the Prompt 7 final decision memo and this implementation guide only
- therefore the folder-local validation question is whether this guide faithfully implements the valid residue of that final memo without widening scope

### Accepted into this guide

- code-grounded current truth from:
  - [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
  - [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- aligned supporting context from:
  - [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
  - [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- the validated residue of Prompt 7 in:
  - [options_stress_scope_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_07_stress_family_expansion_constraints/options_stress_scope_direction_FINAL.md)

### Explicitly excluded

- any argument that widens scope without concrete decision value
- any claim that a fifth lane is justified by symmetry alone
- any move to promote `extreme` into a broad always-on lane without new evidence
- any attempt to move realism-lane or quote-aware disagreement work into Prompt 7
- any change that would widen GPU/runtime tensors, artifact summaries, or UI tables for another generic lane

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

- explicit documentation language that the current stress-family surface is frozen
- routing pointers so future contributors do not reopen the question casually
- narrow future-reopen criteria

It may not remove or narrow:

- current stress, harsher-stress, or extreme-stress reporting already present in backend or UI
- current artifact summary fields
- current validation semantics already implemented in Prompt 6
- current GPU-first broad-path design

### Explicit non-goals

Do not use this pass to:

- add a fifth generic stress lane
- widen `AggregateStressMode`
- widen broad compiled caches or GPU buffers for another generic lane
- promote `extreme` into the broad ranking path
- redesign `robust_score`
- widen artifact summary contracts
- widen frontend tables or filters for another generic stress family
- move quote-aware or realism-lane disagreement work into Prompt 7

---

## 6. Target End State

When this pass is complete, a maintainer should be able to tell, without ambiguity:

- the generic options stress-family boundary is currently frozen
- the broad compiled path remains `baseline`, `conservative`, and `harsher`
- `extreme` remains exact retained-tuple / sealed-finalist work
- quote-aware shock ideas do not belong to Prompt 7 unless the scope is explicitly reopened
- future work should spend decision-value budget elsewhere before adding another generic lane

And a contributor should not be able to accidentally justify runtime widening by pointing at Prompt 7 alone.

---

## 7. Implementation Phases

## 7.1 Phase 1 - Decision Capture In This Folder

Goal:

record the validated Prompt 7 outcome in a checklist-backed implementation guide.

### Required touch point

- this file in `prompt_07_stress_family_expansion_constraints/`

### Required behavior

- capture the valid result as a scope freeze
- explicitly state that no runtime expansion is recommended now
- separate valid carry-forward from deferred or rejected ideas

---

## 7.2 Phase 2 - Optional Roadmap Language Sync

Goal:

make the freeze explicit in the standing options roadmap if stronger discoverability is desired.

### Candidate touch point

- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)

### Allowed additions

- sharpen wording that the current practical stress-family set is frozen
- clarify that the broad compiled path is `baseline` / `conservative` / `harsher`
- clarify that `extreme` remains retained/exact rather than broad-path by default

### Scope guard

Do not use this phase to rewrite the roadmap around a new stress family.

---

## 7.3 Phase 3 - Optional Repo Routing Sync

Goal:

make the Prompt 7 freeze decision discoverable from repo routing material.

### Candidate touch point

- [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)

### Allowed additions

- a short routing note pointing options contributors to this guide when stress-family expansion questions arise

### Scope guard

Do not turn `REPO_MAP.md` into a second roadmap document.

---

## 7.4 Phase 4 - No Runtime Expansion

Goal:

leave runtime and UI contracts unchanged for Prompt 7.

### Files intentionally untouched for the adopted direction

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

### Required behavior

- do not widen any stress-lane enums, tuple fields, summary fields, or UI surfaces
- do not change runtime ranking, exactification, or finalist staging for Prompt 7

---

## 8. File Ownership Map

### Primary implementation files for this guide

- [options_stress_scope_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_07_stress_family_expansion_constraints/options_stress_scope_direction_FINAL.md)
- [options_stress_scope_freeze_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_07_stress_family_expansion_constraints/options_stress_scope_freeze_implementation_guide_2026-04-10.md)

### Optional supporting routing/docs files

- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)

### Future reopen touch points only

These are not current implementation targets.
They matter only if the stress-family question is explicitly reopened later.

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)

---

## 9. Acceptance, Rejection, And Stop Rules

## 9.1 Accept the pass if all of the following are true

- the current stress-family boundary is documented clearly enough to avoid accidental scope drift
- no runtime or schema widening was introduced for Prompt 7
- `extreme` remains exact retained-tuple / sealed-finalist work
- quote-aware shock ideas remain outside Prompt 7 generic stress-family expansion

## 9.2 Reject any part of the pass if it does any of the following

- adds a fifth generic stress lane
- widens broad compiled tensors or GPU work for another generic lane
- promotes `extreme` into a new broad always-on path
- widens artifact summaries or frontend surfaces for another generic lane
- reframes realism-lane disagreement work as Prompt 7 generic stress-family work

## 9.3 Stop rule

If work starts drifting into:

- new stress-family design
- broad-path tensor redesign
- quote-aware realism semantics
- validation-depth expansion

stop and reject the drift.
That is outside the valid Prompt 7 residue.

---

## 10. Sign-Off Target

The correct sign-off for this pass is:

- the generic options stress-family boundary is frozen
- no fifth generic lane is being implemented
- `extreme` remains exact retained-tuple / sealed-finalist work
- current backend, artifact, and frontend surfaces remain unchanged for Prompt 7
- any future lane expansion must clear the reopen criteria in this guide
- full Prompt 7 implementation means policy capture and any chosen doc-routing sync only, not new runtime work

---

## 11. Implementation Checklist

## 11.1 Validated Current Baseline

- [x] The broad compiled options stress path already covers `baseline`, `conservative`, and `harsher`.
- [x] `extreme` already exists as retained-tuple / sealed-finalist exact evaluation rather than a broad always-on lane.
- [x] Tuple result contracts already carry stress, harsher-stress, and extreme-stress outputs.
- [x] Artifact summaries already expose stress, harsher-stress, and extreme-stress reporting fields.
- [x] Frontend reopen and tuple-detail surfaces already understand stress, harsher-stress, and extreme-stress outputs.

## 11.2 Adopted Prompt 7 Decisions

- [x] Freeze the generic stress-family set now.
- [x] Do not add a fifth generic stress lane.
- [x] Do not add a symmetry-only intermediate or harsher-than-harsher lane.
- [x] Keep `baseline`, `conservative`, and `harsher` as the broad compiled robustness lanes.
- [x] Keep `extreme` as the retained-tuple / sealed-finalist exact lane.
- [x] Keep quote-aware spread/liquidity shock work outside Prompt 7 generic stress-family scope.

## 11.3 Current Implementation Scope

- [x] Prompt 7 does not require backend runtime changes now.
- [x] Prompt 7 does not require tuple schema changes now.
- [x] Prompt 7 does not require artifact summary changes now.
- [x] Prompt 7 does not require frontend surface expansion now.
- [x] Prompt 7 does not require GPU-path widening now.

## 11.4 Remaining Policy-Sync Decisions

- [x] Create this implementation guide in the Prompt 7 folder.
- [x] Sharpen [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md) so the freeze is more explicit.
- [x] Add a short routing pointer in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md) for future stress-family questions.
- [x] Add a backlink from [options_stress_scope_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_07_stress_family_expansion_constraints/options_stress_scope_direction_FINAL.md) to this implementation guide.

## 11.5 Explicit Non-Work Items

- [x] Do not add another generic stress-lane enum or mode now.
- [x] Do not widen broad compiled candidate caches for another generic lane now.
- [x] Do not promote `extreme` into the broad ranking path now.
- [x] Do not widen artifact summary best-value or presence fields for another lane now.
- [x] Do not widen frontend tables, compare surfaces, or filter vocabulary for another generic lane now.

---

## 12. Reopen Criteria

Do not reopen Prompt 7 unless all of the following become true:

1. **Distinct failure mode**
   - repeated real-data use shows a failure mode that `baseline`, `conservative`, `harsher`, and `extreme` do not already expose clearly enough

2. **Decision relevance**
   - the proposed lane materially changes:
   - robust winner ordering
   - retained `top_k` composition
   - sealed-finalist promotion or demotion
   - or final validation verdicts

3. **Deterministic semantics**
   - the proposed lane has clear semantics across supported price semantics and fill-model assumptions

4. **Tensor-economics fit**
   - the proposed lane fits the current tensor/runtime model or a bounded retained-tuple exact path without disproportionate schema, planner, artifact, or UI sprawl

5. **Beats simpler alternatives**
   - the proposed lane is more useful than improving:
   - validation clarity
   - robustness-frontier reporting
   - or aggregate-vs-derived-mid-vs-quote-aware disagreement surfacing

If those conditions are not met, the correct decision remains:

> keep the current stress-family surface frozen and spend the next decision-value dollar elsewhere.

---

## 13. Verification Plan

### 13.1 For the adopted Prompt 7 direction

No runtime verification is required, because the validated outcome is not a runtime expansion.

Minimum acceptable verification for this guide is:

- confirm the repo already exposes the current stress-lane surfaces described above
- confirm the guide does not authorize new runtime work that exceeds the validated Prompt 7 residue
- confirm the only remaining unchecked items are policy-sync decisions rather than hidden backend/frontend implementation work

### 13.2 If Prompt 7 is explicitly reopened later

Use this order:

1. backend unit/build checks
   - `cargo test -q -p ta_app_core options_ --lib`
   - `cargo check -q -p ta_desktop_demo_app`
2. CUDA build compatibility if broad-lane or scorer contracts change
   - `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
3. artifact-summary and reopened-artifact coherence review
4. frontend review only after backend proof exists

---

## 14. Final Sign-Off Template

Use this wording pattern if the remaining optional doc-sync items are completed:

- Prompt 7 is signed off as a stress-family scope-freeze decision
- no fifth generic stress lane is being implemented
- the broad compiled path remains `baseline`, `conservative`, and `harsher`
- `extreme` remains retained-tuple / sealed-finalist exact work
- any future lane expansion requires a separate evidence-backed reopen
