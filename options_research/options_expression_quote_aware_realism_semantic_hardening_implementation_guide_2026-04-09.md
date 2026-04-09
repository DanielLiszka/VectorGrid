# Options Expression Quote-Aware Realism Semantic Hardening — Detailed Implementation Guide

Date: 2026-04-09  
Status: implemented and signed off from the validated portions of Prompt 4 (`quote_aware_realism_direction_FINAL.md`)

## Purpose

This document converts the validated residue of:

- [quote_aware_realism_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_04_quote_aware_realism_productization/quote_aware_realism_direction_FINAL.md)

into one narrow, implementation-ready plan.

The key point is:

- Prompt 4 did **not** justify direct quote-aware Polygon/Massive provider-download expansion
- Prompt 4 did justify a strict semantic-hardening pass around the **already shipped** realism-lane posture:
  - aggregate remains the canonical provider-download and hot-path lane
  - quote-aware and derived-mid remain first-class product lanes
  - `TopOfBookQuote` remains the only true quote-side execution semantics
  - `DerivedMidSnapshot` remains first-class in product posture, but aggregate-style in backend execution

This guide exists to make that posture explicit, durable, test-locked, and easy to maintain without reopening scope.

This document is **not** a master consolidation plan for every sibling Markdown file in `options_research/`.
It is a validated implementation tracker for the narrow subset of those documents that legitimately survives into this specific Prompt 4 follow-up pass.

Use this together with:

- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md)
- [options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md)

Quick pointers:

- objective and allowed scope: [Section 1](#1-objective)
- validated baseline: [Section 2](#2-validated-current-baseline)
- accepted Prompt 4 carry-forward: [Section 3](#3-accepted-carry-forward-from-prompt-4)
- implementation phases: [Section 7](#7-implementation-phases)
- full checklist: [Section 10](#10-implementation-checklist)
- verification plan: [Section 11](#11-verification-plan)

---

## 1. Objective

The next valid Prompt 4 pass should do one thing only:

**lock the current realism-lane product posture in code, tests, copy, and artifact semantics without broadening provider scope or changing execution semantics.**

That objective breaks down into:

1. **Semantic lock-in**
   - keep aggregate as the canonical provider-download lane
   - keep direct Polygon/Massive download aggregate-only
   - keep quote-aware and derived-mid as first-class product lanes
   - keep `TopOfBookQuote` as the only true quote-side execution semantics
   - keep `DerivedMidSnapshot` explicitly non-quote-side in backend execution

2. **Product-surface clarity**
   - make lane semantics obvious in the existing options workflow
   - prevent users from inferring that derived-mid is equivalent to quote-side execution
   - prevent users from inferring that direct provider download is already quote-aware

3. **Scope freeze**
   - do not add direct quote-aware Polygon/Massive provider download
   - do not broaden runtime semantics
   - do not turn Prompt 4 into a new data-plane or execution-plane project

This guide is **not** a quote-native expansion guide and is **not** a provider-download implementation plan.

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not goals.

- [x] The repo currently supports three price-semantics families:
  - `MinuteAggregate`
  - `DerivedMidSnapshot`
  - `TopOfBookQuote`
- [x] The frontend currently exposes three realism lanes mapped from those semantics:
  - `aggregate`
  - `derived_mid`
  - `quote_aware`
- [x] Direct Polygon/Massive provider download is aggregate-only in:
  - [commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [x] Normalized CSV import already accepts quote-aware and derived-mid semantics in:
  - [commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
  - [ta_options_dataset_import.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_dataset_import.rs)
- [x] `TopOfBookQuote` is the only semantics family that currently supports quote-side fill models in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] `MinuteAggregate` and `DerivedMidSnapshot` currently allow only `aggregate_research` in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Runtime execution is currently:
  - aggregate-style for `MinuteAggregate`
  - aggregate-style for `DerivedMidSnapshot`
  - quote-side for `TopOfBookQuote`
  in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] The options panel/store already treat quote-aware and derived-mid as first-class product lanes through:
  - lane chooser
  - lane product status
  - lane-based dataset filtering
  - fill-model guidance/coercion
  - aggregate-only provider-download gating
  - artifact comparison/reporting
- [x] Artifact and dataset surfaces already persist `price_semantics`; the frontend derives lane labels from that persisted semantics rather than storing an independent canonical `realism_lane` field.

Important nuance:

- Prompt 4 does **not** prove that direct quote-aware provider download is never worth building
- Prompt 4 does prove that the current repo posture is coherent enough to preserve as-is
- the remaining valid work is mostly semantic-hardening, test-hardening, and UI/provenance clarity

---

## 3. Accepted Carry-Forward From Prompt 4

Only the following Prompt 4 conclusions are accepted into this guide as implementation work.

### 3.1 Accepted

- [x] Keep aggregate `1m` as the canonical provider-download lane and optimizer hot path.
- [x] Keep quote-aware and derived-mid as first-class realism lanes in the existing options workflow.
- [x] Keep direct Polygon/Massive provider download aggregate-only for now.
- [x] Keep `TopOfBookQuote` as the only semantics family that supports quote-side fills.
- [x] Keep `DerivedMidSnapshot` as a first-class lane/provenance concept while explicitly preserving aggregate-style runtime execution.
- [x] Keep artifact, dataset, and UI labeling derived from `price_semantics`.
- [x] Treat the remaining valid Prompt 4 work as semantic lock-in and clarity hardening, not scope expansion.

### 3.2 Not accepted as implementation work

These ideas may appear in the Prompt 4 memo as discussion or deferred alternatives, but they are not work items here:

- direct quote-aware Polygon/Massive provider download
- new quote-native dataset contract design
- new quote-native runtime/backtest path for derived-mid
- broad provider benchmarking to justify new quote surfaces
- new workflow stages
- widening the realism-lane set beyond the current three lanes
- treating derived-mid as execution-equivalent to quote-aware

---

## 4. Source Validation Filter

This guide is intentionally narrower than the sibling options documents in this folder.

It should be read as:

- a validated implementation subset
- not a wholesale merger of every idea present in every sibling document
- and not authorization to reopen broader options-roadmap decisions already frozen elsewhere

### Accepted into this guide

- code-grounded current truth from:
  - [commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
  - [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- current test surfaces in:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
  - [optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
  - [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)
- aligned supporting context from:
  - [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
  - [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- the validated residue of Prompt 4 in:
  - [quote_aware_realism_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_04_quote_aware_realism_productization/quote_aware_realism_direction_FINAL.md)

### Explicitly excluded

- any statement that turns the Prompt 4 recommendation into “proved forever truth”
- any direct provider-download expansion
- any new quote-native execution semantics for `DerivedMidSnapshot`
- any reopening of broader future-roadmap scope
- any prompt-pack or roadmap material that would widen the current realism-lane scope beyond what current code and the validated Prompt 4 memo support
- any work that depends on:
  - new strategy families
  - new structure families
  - new workflow stages
  - CUDA/runtime/planner changes

### Practical rule

If a statement is:

- already true in code, mark it as baseline `[x]`
- a bounded semantic-hardening or clarity-hardening task, mark it as remaining `[ ]`
- speculative, broadening, or deferred, exclude it from this guide

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This guide is additive and non-destructive.

It may add:

- stronger semantic tests
- stronger UI/copy clarity
- stronger artifact/dataset provenance assertions
- stronger import/download lane guardrails

It may not remove or narrow:

- the current aggregate lane
- the current quote-aware lane
- the current derived-mid lane
- the current provider-download path
- the current import path
- the current artifact/reporting surface

### Explicit non-goals

Do not use this pass to:

- implement direct quote-aware Polygon/Massive provider download
- add a new quote-native downloader contract
- change `TopOfBookQuote` execution semantics
- change `DerivedMidSnapshot` into quote-side execution
- add a new realism lane
- remove derived-mid as a first-class lane
- collapse quote-aware back into a hidden advanced-import niche
- change GPU routing, planner behavior, or runtime policy

---

## 6. Target End State

When this pass is done, a user should be able to tell, from the existing options workflow:

- which realism lane they are using
- whether the lane supports direct Polygon/Massive download or only CSV import
- which fill models are compatible with that lane
- whether the lane is aggregate-style or quote-side in backend execution
- whether a reopened artifact was run on aggregate, derived-mid, or quote-aware semantics

And a maintainer should be able to change the current options realism-lane code without accidentally:

- allowing quote-side fills on non-quote semantics
- erasing the aggregate-only direct-download boundary
- making derived-mid look execution-equivalent to true quote-aware fills
- creating inconsistent lane labels between dataset manifests, artifacts, and the options panel

---

## 7. Implementation Phases

## 7.1 Phase 1 — Backend Semantic Contract Audit

Goal:

- confirm the backend semantic matrix is explicit and sufficiently locked

Primary files:

- [src-tauri/src/commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

Required outcomes:

- direct provider download remains aggregate-only
- importer semantics normalization remains explicit for all three supported semantics
- fill-model compatibility matrix remains explicit and tested
- runtime pricing-path split remains explicit and tested:
  - `MinuteAggregate` -> aggregate-style
  - `DerivedMidSnapshot` -> aggregate-style
  - `TopOfBookQuote` -> quote-side

Acceptable code changes:

- stricter tests
- clearer error strings
- clearer helper naming or comments where ambiguity exists

Not acceptable:

- any semantics broadening
- any downloader broadening
- any new quote-runtime path for derived-mid

## 7.2 Phase 2 — Frontend/Product-Surface Semantic Hardening

Goal:

- ensure the current UI makes the lane boundary explicit and stable

Primary files:

- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

Required outcomes:

- lane chooser remains explicit
- lane labels remain derived from `price_semantics`
- aggregate lane messaging clearly states it supports both direct provider download and CSV import
- quote-aware and derived-mid messaging clearly states they are currently CSV-import lanes
- provider-download action remains gated to aggregate lane
- fill-model guidance remains coherent with backend semantics
- derived-mid wording does not imply true quote-side execution

Acceptable code changes:

- copy clarifications
- small labeling changes
- stronger normalization or helper naming
- targeted test additions

Not acceptable:

- new workflow stages
- hidden lane behavior changes
- UI broadening that implies new backend support

## 7.3 Phase 3 — Artifact And Dataset Provenance Lock-In

Goal:

- ensure saved data and reopened artifacts cannot drift into inconsistent realism-lane labeling

Primary files:

- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

Required outcomes:

- artifact summaries continue to carry `price_semantics`
- dataset manifests continue to carry `price_semantics`
- frontend lane/product labels derive from persisted semantics rather than introducing a second canonical field
- comparison/reporting views stay aligned with that rule

Acceptable changes:

- targeted tests
- type normalization
- clearer helper naming/documentation

Not acceptable:

- introducing a second canonical persisted lane identity unless absolutely required by a future separate plan

## 7.4 Phase 4 — Verification And No-Scope-Creep Sign-Off

Goal:

- prove the current realism-lane posture is coherent without accidentally expanding scope

Required outcomes:

- no direct quote-aware provider-download work is added
- no new runtime semantics are introduced
- no lane is removed or hidden
- tests and UI behavior confirm the current posture rather than drifting away from it

This phase is successful even if the resulting code diff is small.  
The point is to harden a valid shipped posture, not to force feature growth.

---

## 8. File Ownership Map

### Backend semantic ownership

- downloader/import semantics:
  - [commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
- imported dataset contract validation:
  - [options_data_store.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_data_store.rs)
- price semantics and fill-model matrix:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- runtime pricing-path semantics:
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

### Frontend semantic ownership

- normalization and summary parsing:
  - [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- lane mapping, gating, and import/download orchestration:
  - [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- lane UI and explanatory copy:
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

### Primary test ownership

- backend semantics tests:
  - [commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
  - [options_data_store.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_data_store.rs)
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- frontend store/service/panel tests:
  - [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
  - [optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
  - [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)

---

## 9. Acceptance, Rejection, And Stop Rules

### Accept this pass if

- the semantic matrix is explicit and test-locked
- the UI copy and gating remain unambiguous
- derived-mid is clearly first-class but not misrepresented as quote-side execution
- direct provider download remains clearly aggregate-only
- no scope-expansion work sneaks in

### Reject changes in this pass if

- they add direct quote-aware provider download
- they implicitly broaden `DerivedMidSnapshot` into quote-side execution
- they weaken the distinction between quote-aware and derived-mid
- they introduce a second conflicting canonical lane identity
- they remove or demote quote-aware or derived-mid as first-class lanes

### Stop rules

Stop this pass once:

- semantic tests are sufficient
- UI/product wording is sufficiently explicit
- artifact/dataset semantics remain coherent
- no new unresolved ambiguity remains around the current three lanes

Do **not** keep expanding the pass just to make the diff larger.

---

## 10. Implementation Checklist

### 10.1 Baseline and scope freeze

- [x] The current repo posture has been validated:
  - aggregate provider download is canonical
  - quote-aware and derived-mid are already first-class lanes
  - `TopOfBookQuote` is the only quote-side execution semantics
  - `DerivedMidSnapshot` is first-class but aggregate-style in execution
- [x] This pass is limited to semantic hardening and clarity hardening.
- [x] Direct quote-aware provider download is explicitly excluded from this guide.
- [x] No new realism lane or workflow stage is in scope.

### 10.2 Backend semantic audit

- [x] Confirm the downloader remains aggregate-only and that this is explicit in code/tests.
- [x] Confirm importer semantics normalization remains explicit for:
  - `minute_aggregate`
  - `derived_mid_snapshot`
  - `top_of_book_quote`
- [x] Confirm the fill-model compatibility matrix is explicitly tested and sufficient.
- [x] Confirm runtime pricing-path semantics are explicitly tested and sufficient.
- [x] Add or tighten backend tests only where a semantic ambiguity is still unguarded.

### 10.3 Frontend/product-surface hardening

- [x] Audit lane chooser, lane product status, and related panel copy for semantic clarity.
- [x] Audit store gating so direct provider download remains aggregate-lane only.
- [x] Audit fill-model guidance/coercion so it stays aligned with backend semantics.
- [x] Tighten frontend labels/copy only if they currently overstate derived-mid realism or understate aggregate-only download limits.
- [x] Add or tighten frontend tests where current semantics are not locked clearly enough.

### 10.4 Provenance and reporting hardening

- [x] Confirm dataset summaries, artifact summaries, and reopened artifact views all remain anchored to `price_semantics`.
- [x] Confirm no new conflicting canonical `realism_lane` persistence is introduced.
- [x] Tighten summary parsing or lane-derivation helpers only if ambiguity is found.

### 10.5 Sign-off

- [x] Run targeted backend semantics tests.
- [x] Run targeted frontend/store/service/panel tests.
- [x] Run typecheck/build for the touched surface.
- [x] Record the sign-off outcome in this document:
  - either “no code changes needed; current semantics already sufficiently locked”
  - or “narrow semantic-hardening changes shipped”

### 10.6 Documentation and routing sync

- [x] Update this guide after implementation so completed tasks are crossed from `[ ]` to `[x]`.
- [x] Update [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md) only if routing or the recommended entry-point list changed during implementation.
- [x] Update the Prompt 4 prompt-pack or attachment guidance only if the implementation materially changed the repo truth that Prompt 4 relies on.
- [x] Record any deliberately rejected scope-expansion ideas encountered during implementation in the sign-off notes.

---

## 11. Verification Plan

This pass should prefer semantic verification over benchmark-heavy verification.

### Backend

- `cargo test -q -p ta_app_core options_ --lib`
- targeted semantics coverage in:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

### Frontend

- `npm run typecheck`
- `npm run build`
- targeted tests in:
  - [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
  - [optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
  - [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)

### Verification focus

Verify these facts, not just raw green test counts:

- aggregate lane still supports direct provider download
- derived-mid and quote-aware still behave as CSV-import lanes
- derived-mid still rejects quote-side fill models
- quote-aware still accepts quote-side fill models
- runtime semantics remain aggregate-style for aggregate + derived-mid and quote-side for quote-aware
- UI/product wording does not overclaim derived-mid behavior

### Verification executed

- [x] `cargo test -q -p ta_app_core options_ --lib`
- [x] `cargo test -q -p ta_desktop_demo_app commands_options_ --lib`
- [x] `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
- [x] `npm run typecheck`
- [x] `npm run build`
- [x] `npm.cmd run test -- src/components/OptionsExpressionPanel.test.ts src/stores/optionsExpressionStore.test.ts src/services/optionsExpressionService.test.ts`
  - result: `3` files, `52` tests passed

---

## 12. Final Sign-Off Template

Use this section when implementation is complete.

### Outcome

- [ ] No code changes were needed; the current repo already satisfied the valid Prompt 4 residue.
- [x] Narrow semantic-hardening changes shipped.

### Final decision

- [x] Keep current realism-lane posture unchanged.
- [x] Keep direct quote-aware Polygon/Massive provider download deferred.
- [x] Keep quote-aware and derived-mid first-class in product posture.
- [x] Keep `DerivedMidSnapshot` explicitly non-quote-side in execution semantics.

### Notes

- Backend hardening shipped in:
  - [commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
  - [options_data_store.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_data_store.rs)
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- Frontend/product-surface hardening shipped in:
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)
  - [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
  - [optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
- Specific semantic/provenance guardrails added or tightened:
  - aggregate-only provider-download invariants are now locked by tests
  - minute-aggregate imports explicitly reject `aggregate_price_field != 'close'`
  - `top_of_book_quote` imports require quote-side columns and populated quote-side rows
  - reopened artifact inspection replay now refuses dataset hash drift, price-semantics drift, and parent-artifact drift
  - runtime tests now explicitly lock:
    - `DerivedMidSnapshot` -> aggregate-style pricing/path semantics
    - `TopOfBookQuote` -> quote-side pricing/path semantics
- Copy clarifications shipped:
  - quote-aware and derived-mid lanes now describe new dataset acquisition as import-first today
  - quote-aware is called out as the only quote-side execution lane
  - derived-mid is called out as first-class but explicitly aggregate-style and non-quote-side
- Deliberately rejected scope expansion during this pass:
  - no direct quote-aware Polygon/Massive provider download
  - no new quote-native runtime path for derived-mid
  - no new realism lane or workflow stage
