# Options Expression Support-Aware Validation Semantics — Detailed Implementation Guide

Date: 2026-04-10  
Status: implemented and signed off on 2026-04-10 from the validated portions of Prompt 6 (`options_validation_depth_direction_FINAL.md`)

## Purpose

This document converts the validated residue of:

- [options_validation_depth_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_06_embedded_validation_depth_expansion/options_validation_depth_direction_FINAL.md)

into one narrow, implementation-ready plan.

The key point is:

- Prompt 6 did **not** justify a new validation stage
- Prompt 6 did **not** justify a shortlist-budget rewrite
- Prompt 6 did justify one bounded semantic-hardening pass:
  - make validation verdicts support-aware instead of lane-presence-aware
  - recompute verdict semantics for reopened artifacts and summary counts
  - surface raw support counts and support ratios inside the current UI

This guide exists to make that work explicit, additive, testable, and easy to verify without reopening:

- workflow shape
- realism-lane scope
- GPU/runtime shape
- ranking math
- shortlist/finalist budgets

This document is **not** a master consolidation plan for every sibling Markdown file in `options_research/`.
It is a validated implementation tracker for the narrow subset of those documents that legitimately survives into this specific Prompt 6 follow-up pass.

Use this together with:

- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md)
- [options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md)
- [options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md)
- [options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md)

Quick pointers:

- objective and allowed scope: [Section 1](#1-objective)
- validated baseline: [Section 2](#2-validated-current-baseline)
- accepted Prompt 6 carry-forward: [Section 3](#3-accepted-carry-forward-from-prompt-6)
- implementation phases: [Section 7](#7-implementation-phases)
- full checklist: [Section 11](#11-implementation-checklist)
- verification plan: [Section 12](#12-verification-plan)

---

## 1. Objective

The next valid Prompt 6 pass should do one thing only:

**make options embedded-validation semantics support-aware, using counts already present in saved tuple metrics, while preserving the current workflow and validation families.**

That objective breaks down into:

1. **Verdict semantic hardening**
   - stop treating the presence of a holdout or sealed metric struct as sufficient validation support
   - require supported trades in the relevant lane before promoting the verdict
   - require supported base and stress lanes before calling a tuple stress-fragile

2. **Artifact truth refresh**
   - stop blindly trusting legacy persisted verdicts for reopened artifacts and summary counts
   - recompute current verdict semantics from current tuple metrics

3. **Current-host adequacy surfacing**
   - show raw support counts and support ratios in the current panel
   - do this without creating a new validation mode or a heavier artifact dashboard

This guide is **not** a frontier-math guide, **not** a ranking-functional guide, and **not** a runtime-budget guide.

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not goals.

- [x] The options-expression workflow is already shipped as an artifact-driven optimization + embedded-validation lane.
- [x] The repo already ships holdout, sealed-finalist, and multi-lane stress evaluation.
- [x] Tuple rows already persist the ingredients needed for support-aware semantics:
  - `valid_trades`
  - `invalid_events`
  - `invalid_rate`
  in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Verdict refresh already exists in the runtime path through `refresh_validation_verdicts(...)` in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] Tuple verdict inference is still presence-based today in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Frontend fallback verdict logic is also still presence-based in [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue).
- [x] Reopened artifact loading still returns persisted artifacts as-is in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs).
- [x] Prompt 5 is already closed:
  - compact artifact-summary coherence has already shipped
  - retained leader/disagreement summary already exists
  - Prompt 6 should build on that baseline rather than re-solving it
- [x] The current options workflow already has overlap/adequacy reporting and should keep that scope boundary.

Important nuance:

- the repo is **not** missing embedded validation lanes
- the repo is **not** missing row-level validation inspection
- the remaining valid Prompt 6 gap is specifically **support-aware validation semantics and current-host adequacy truthfulness**

---

## 3. Accepted Carry-Forward From Prompt 6

Only the following Prompt 6 conclusions are accepted into this guide as implementation work.

### 3.1 Accepted

- [x] Keep the current workflow and verdict enum shape.
- [x] Reinterpret validation support from existing metrics instead of adding a new heavyweight support payload first.
- [x] Make tuple verdicts require supported trades in the relevant validation lane, not merely present lane structs.
- [x] Require supported base and supported stress lanes before calling a tuple `stress_fragile`.
- [x] Recompute verdict semantics for reopened artifacts and artifact summaries instead of trusting legacy persisted verdicts blindly.
- [x] Surface raw support counts and support ratios inside the existing UI.
- [x] Keep the work compact, metrics-derived, and inside the current host.

### 3.2 Not accepted as implementation work

These ideas may appear in the Prompt 6 memo as discussion or deferred alternatives, but they are not work items here:

- a new validation mode or stage
- verdict-taxonomy expansion
- shortlist or finalist-budget rewrite
- heavier frontier/regret/dominance math
- bootstrap or replay-heavy confidence estimation
- new dashboard layers built on top of existing artifact/frontier surfaces
- Prompt 5 artifact-summary coherence reopening

---

## 4. Source Validation Filter

This guide is intentionally narrower than the sibling options documents in this folder.

It should be read as:

- a validated implementation subset
- not a wholesale merger of every idea present in every sibling document
- and not authorization to reopen broader options-roadmap decisions already frozen elsewhere

### Accepted into this guide

- code-grounded current truth from:
  - [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
  - [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- aligned supporting context from:
  - [options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md)
  - [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- the validated residue of Prompt 6 in:
  - [options_validation_depth_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_06_embedded_validation_depth_expansion/options_validation_depth_direction_FINAL.md)

### Explicitly excluded

- any statement that turns the Prompt 6 recommendation into “proved highest-ROI truth”
- any new validation stage
- any verdict-enum expansion
- any shortlist/finalist budget changes
- any replay-heavy confidence or bootstrap work
- any work that depends on:
  - new parent families
  - new structure families
  - CUDA/runtime/planner changes
  - Prompt 5 artifact-summary redesign

### Practical rule

If a statement is:

- already true in code, mark it as baseline `[x]`
- a bounded semantic-hardening or current-host adequacy task, mark it as remaining `[ ]`
- speculative, broadening, or dependent on another unresolved prompt line, exclude it from this guide

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This guide is additive and non-destructive.

It may add:

- support-aware metrics helper methods
- support-aware tuple verdict semantics
- recomputed artifact verdict counts on load/summarization
- compact support-count and support-rate cues in the existing UI
- narrow backend and frontend tests

It may not remove or narrow:

- the current verdict enum
- the current workflow host
- the current validation lane set
- the current artifact reopen flow
- the current Prompt 5 summary coherence work
- the current realism-lane posture
- the current GPU route

### Explicit non-goals

Do not use this pass to:

- change `robust_score`
- change shortlist/finalist defaults
- add a new validation stage
- widen GPU state
- add replay or bootstrap confidence work
- create a new artifact-level dashboard
- reopen Prompt 4 or Prompt 5 decisions

---

## 6. Target End State

When this pass is done, a user should be able to tell, from the existing options workflow:

- whether a tuple has **supported** holdout validation rather than merely a present holdout lane
- whether a tuple is a sealed finalist by budget and whether it has actual sealed validation support
- whether stress fragility reflects real supported validation/stress evidence rather than structural lane presence
- how many supported trades exist in:
  - holdout
  - sealed
  - strongest stressed lane
- what the support ratio is for those lanes inside the existing UI

And a maintainer should be able to change validation semantics without accidentally:

- promoting tuples because a metric struct exists but has zero supported trades
- calling a tuple stress-fragile when the stressed lane has no supported trades
- trusting stale persisted verdicts in reopened artifacts
- hiding support thinness behind opaque confidence labels

---

## 7. Implementation Phases

## 7.1 Phase 1 — Metrics Helper Semantics

Goal:

derive support-aware truth from the metrics the repo already stores.

### Required touch point

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

### Required additions

On `OptionsExpressionMetrics`, add compact helpers such as:

- `total_events()`  
  `valid_trades + invalid_events`
- `has_supported_trades()`  
  `valid_trades > 0`
- `support_rate()`  
  `valid_trades / total_events()` when `total_events() > 0`

These helpers should stay:

- deterministic
- side-effect free
- derived only from currently stored fields

Do **not** add a new persisted support payload in this phase.

---

## 7.2 Phase 2 — Tuple Verdict Semantics

Goal:

make tuple verdict inference support-aware while keeping the current enum.

### Required touch point

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

### Required behavior

Revise these methods:

- `strongest_validation_metrics()`
- `strongest_validation_stress_metrics()`
- `inferred_validation_verdict()`
- `is_stress_fragile()`

### Required semantic changes

- `holdout_supported` requires `holdout_metrics.has_supported_trades()`
- `sealed_finalist` requires supported sealed evaluation, not just `sealed_finalist == true`
- `sealed_finalist` as a budget flag stays separate from support-aware verdict meaning
- `stress_fragile` requires:
  - supported base validation metrics
  - supported stressed metrics
  - then existing fragility comparison logic
- fallback remains `search_only`

### Important rule

This pass changes **semantics**, not the enum itself.

Do **not** add new verdict variants.

---

## 7.3 Phase 3 — Runtime Refresh And Finalist Interaction

Goal:

make sure the runtime refresh path applies the new semantics cleanly after exactification and sealed-finalist staging.

### Required touch point

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

### Required behavior

Keep `refresh_validation_verdicts(...)` as the central post-processing seam, but confirm it now:

- applies the support-aware tuple semantics
- does not relabel zero-support sealed finalists as sealed-validated
- does not mark zero-support stressed lanes as fragile

### Scope guard

Do **not** redesign:

- `top_k`
- `sealed_finalist_count`
- the exactification boundary
- the stress lane set

This phase is semantic refresh only.

---

## 7.4 Phase 4 — Reopened Artifact Truth Refresh

Goal:

stop trusting stale persisted verdicts during reopen and summary derivation.

### Required touch points

- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)

### Required behavior

For reopened artifacts:

- recompute tuple `validation_verdict` on load using current semantics
- recompute artifact-level verdict counts from current tuple metrics/semantics
- recompute `top_validation_verdict` from the refreshed tuple rows

### Minimum acceptable scope

It is acceptable to keep the persisted artifact file unchanged on disk.

This pass only needs the loaded/in-memory artifact and artifact summary surfaces to reflect current semantics.

### Scope guard

Do **not** use this phase to widen `OptionsExpressionArtifactSummary` more than necessary.

Only add compact support-aware cues if they are directly needed by the current UI.

---

## 7.5 Phase 5 — Existing UI Surface Hardening

Goal:

surface raw support facts inside the current UI, not a new validation product.

### Required touch points

- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)

### Required surfaces

1. **Selected tuple validation summary**
   - compact support counts for strongest holdout/sealed/stress lanes
   - support-rate cue where useful

2. **Reopened artifact validation summary**
   - compact support-aware counts/ratios based on refreshed semantics

3. **Saved artifact comparison**
   - only minimal verdict truthfulness improvement if artifact-level cues become available
   - do not turn compare rows into another dashboard

### UI rule

Prefer:

- raw counts
- raw ratios
- updated wording

Over:

- opaque confidence scores
- new tone systems
- more validation categories

---

## 7.6 Phase 6 — Narrow Test Hardening

Goal:

lock in support-aware semantics with backend and frontend tests.

### Backend test requirements

In [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs):

- zero-support holdout lane stays `search_only`
- supported holdout lane promotes to `holdout_supported`
- zero-support sealed lane does not promote to `sealed_finalist`
- supported sealed lane promotes correctly
- zero-support stress lane does not produce `stress_fragile`

In [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs):

- refresh path preserves new semantics after exactification/finalist staging

In [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs):

- reopened artifact load recomputes tuple verdicts
- artifact summary counts reflect refreshed semantics rather than stale persisted verdicts

### Frontend test requirements

In:

- [web/src/services/optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
- [web/src/stores/optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
- [web/src/components/OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)

test:

- refreshed verdict semantics render correctly
- raw support counts/ratios appear where expected
- older artifacts still load without schema breakage

---

## 7.7 Phase 7 — Documentation And Routing Sync

Goal:

make this execution guide discoverable like the other options implementation passes.

### Required routing touch point

- [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)

### Optional supporting touch point

- the Prompt 6 final memo, if you want a pointer from the decision document into this guide

---

## 8. File Ownership Map

### Primary implementation files

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - support-aware metrics helpers
  - tuple verdict semantics

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - runtime verdict refresh consistency

- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - reopened artifact verdict refresh
  - artifact summary verdict/count recomputation

- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
  - any compact transport normalization needed for support-aware UI cues

- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
  - store threading only if needed

- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - selected tuple, reopened artifact, and compact compare-surface support cues

### Primary verification files

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [web/src/services/optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
- [web/src/stores/optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
- [web/src/components/OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)

### Intentionally untouched files

These should stay unchanged for the adopted direction unless a small helper extraction becomes clearly necessary:

- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)

This is intentional. Prompt 6 valid residue is semantic hardening, not GPU-path change.

---

## 9. Acceptance, Rejection, And Stop Rules

## 9.1 Accept the pass if all of the following are true

- tuple verdicts require supported validation, not mere lane presence
- stress fragility requires supported base and stress lanes
- reopened artifacts reflect current semantics without rewriting artifact files on disk
- current UI surfaces show raw support facts clearly enough for decision-grade use
- the implementation remains inside the current workflow and Prompt 5 baseline

## 9.2 Reject or roll back any part of the pass if it does any of the following

- adds a new validation stage
- adds a new verdict taxonomy
- widens GPU state
- adds replay-heavy confidence work
- changes shortlist/finalist budgets
- turns compare or reopened summary surfaces into a broader dashboard

## 9.3 Stop rule

If the pass starts drifting into:

- budget-policy redesign
- frontier-math expansion
- new confidence-taxonomy work
- or heavier artifact-dashboard work

stop and reject the drift. That is outside the valid Prompt 6 residue.

---

## 10. Sign-Off Target

The correct sign-off for this pass is:

- validation semantics become support-aware
- reopened artifacts and artifact summaries reflect current semantics
- the current UI shows raw support counts/ratios where they matter
- workflow shape, verdict enum, ranking math, and GPU/runtime behavior remain unchanged

---

## 11. Implementation Checklist

## 11.1 Baseline And Scope Freeze

- [x] Prompt 5 summary coherence work is already closed and must remain frozen.
- [x] Current workflow remains optimization + embedded validation only.
- [x] Current validation lanes already exist.
- [x] Current valid Prompt 6 residue is support-aware semantics, not new workflow or budget work.
- [x] Current scope remains additive and must not reopen realism, ranking, or runtime questions.

## 11.2 Metrics Helpers And Tuple Semantics

- [x] Add compact support-aware helper methods to `OptionsExpressionMetrics`.
- [x] Update `strongest_validation_metrics()` to prefer supported validation lanes only.
- [x] Update `strongest_validation_stress_metrics()` to prefer supported stress lanes only.
- [x] Update `inferred_validation_verdict()` to require supported holdout/sealed trades.
- [x] Update `is_stress_fragile()` to require supported base and stress lanes.
- [x] Preserve the existing verdict enum unchanged.

## 11.3 Runtime Refresh And Artifact Truth

- [x] Confirm `refresh_validation_verdicts(...)` applies the new semantics cleanly after exactification/finalist staging.
- [x] Recompute tuple verdicts when loading reopened artifacts.
- [x] Recompute artifact-level verdict counts from refreshed tuple semantics.
- [x] Recompute `top_validation_verdict` from refreshed tuple semantics.
- [x] Keep persisted artifact files backward-compatible and unchanged on disk.

## 11.4 Existing UI Surface Hardening

- [x] Add compact raw support counts/ratios to selected tuple validation surfaces.
- [x] Add compact raw support counts/ratios to reopened artifact validation surfaces.
- [x] Add only minimal support-aware compare-surface cues if needed.
- [x] Update wording so the UI no longer overclaims validation from mere lane presence.
- [x] Avoid dashboard sprawl or new stage-like UI.

## 11.5 Tests

- [x] Add backend unit coverage for zero-support holdout/sealed lanes.
- [x] Add backend unit coverage for zero-support stress lanes not triggering fragility.
- [x] Add runtime-path coverage for verdict refresh after exactification/finalist staging.
- [x] Add Tauri command coverage for reopened-artifact semantic refresh.
- [x] Add frontend coverage for support-aware wording and raw support cues.

## 11.6 Verification

- [x] Run `cargo test -q -p ta_app_core options_ --lib`.
- [x] Run `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`.
- [x] Run `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`.
- [x] Run `npm run typecheck`.
- [x] Run `npm run build`.
- [x] Run targeted frontend tests for options panel/service/store.
- [x] Reopen one saved artifact and confirm verdict semantics reflect support-aware logic.

## 11.7 Documentation And Routing Sync

- [x] Add this guide to `REPO_MAP.md`.
- [x] Optionally add a pointer from the Prompt 6 final memo to this guide.
- [x] Mark this guide signed off once the above verification passes.

---

## 12. Verification Plan

Run verification in this order.

### 12.1 Backend verification

Required:

```powershell
cargo test -q -p ta_app_core options_ --lib
cargo test -q -p ta_desktop_demo_app commands_options_expression --lib
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

### 12.4 Reopened-artifact verification

Executed via automated reopen-path coverage rather than a separate manual GUI pass:

1. `load_options_expression_artifact_blocking_headless_refreshes_tuple_verdicts` verifies a reopened artifact no longer trusts a stale persisted verdict
2. `summarize_options_expression_artifact_reports_validation_verdict_counts` verifies artifact summary counts/verdicts are recomputed from current support-aware semantics
3. `falls back to support-aware tuple validation semantics when verdicts are absent` verifies the current panel shows support-aware wording/cues when tuple verdicts are absent

### 12.5 Executed verification

Executed on 2026-04-10:

```powershell
cargo test -q -p ta_app_core options_ --lib
cargo test -q -p ta_desktop_demo_app commands_options_expression --lib
cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel
npm run typecheck
npm run build
npm run test -- src/components/OptionsExpressionPanel.test.ts src/stores/optionsExpressionStore.test.ts src/services/optionsExpressionService.test.ts
```

---

## 13. Final Sign-Off Template

Use this wording pattern when the pass is complete:

- support-aware validation semantics are implemented and signed off
- tuple verdicts now require supported validation lanes rather than mere lane presence
- stress fragility now requires supported base and stress lanes
- reopened artifacts and artifact summaries reflect current semantics
- the current UI now shows raw support facts without creating a new validation mode
- workflow shape, verdict enum, ranking math, and GPU/runtime behavior remain unchanged
