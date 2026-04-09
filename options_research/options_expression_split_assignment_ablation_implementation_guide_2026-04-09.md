# Options Expression Split-Assignment Ablation — Detailed Implementation Guide

Date: 2026-04-09  
Status: implementation guide derived from the validated portions of Prompt 2 (`options_expression_density_direction_FINAL.md`)

## Purpose

This document converts the validated residue of:

- [options_expression_density_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_02_candidate_density_invalid_slots_and_search_shape_direction/options_expression_density_direction_FINAL.md)

into one narrow, implementation-ready plan.

The key point is:

- Prompt 2 did **not** justify another shipped default search-shape change
- Prompt 2 did justify one bounded, already-supported experiment:
  - `EventChronological` versus `SessionChronological`

This guide exists to make that experiment reproducible, well-scoped, and easy to reject if it does not produce a real cross-root win.

This document is **not** a master consolidation plan for every sibling Markdown file in `options_research/`.
It is a validated implementation tracker for the narrow subset of those documents that legitimately survives into this specific Prompt 2 follow-up pass.

Use this together with:

- [options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md)
- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)

Quick pointers:

- objective and allowed scope: [Section 1](#1-objective)
- validated baseline: [Section 2](#2-validated-current-baseline)
- accepted Prompt 2 carry-forward: [Section 3](#3-accepted-carry-forward-from-prompt-2)
- implementation phases: [Section 7](#7-implementation-phases)
- full checklist: [Section 11](#11-implementation-checklist)
- verification plan: [Section 12](#12-verification-plan)

---

## 1. Objective

The next valid Prompt 2 pass should do one thing only:

**run a one-factor split-assignment ablation on the fixed SPY/QQQ sign-off pair without changing shipped defaults or broadening the current options workflow.**

That objective breaks down into:

1. **Measurement-only ablation**
   - compare `EventChronological`
   - compare `SessionChronological`
   - hold everything else constant

2. **Decision-grade sign-off**
   - decide whether split assignment materially improves split-level executable coverage
   - reject the experiment if it only improves cosmetic density or invalid-rate metrics

3. **No default change without proof**
   - do not change:
     - delay defaults
     - DTE defaults
     - moneyness defaults
     - lag defaults
     - runtime `auto`
   - unless the fixed pair shows a real cross-root win

This guide is **not** a search-grid redesign guide and is **not** a default-tightening guide.

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not goals.

- [x] The overlap-quality hardening pass is already implemented and signed off in [options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md).
- [x] The repo already ships overlap-window, split invalid-reason, and adequacy verdict reporting in:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [x] The current defaults are already owned in code:
  - `delay_minutes = [0, 1, 5]`
  - `dte_buckets = [1, 3, 5]`
  - `moneyness_buckets = [ATM, OTM1, OTM2, ITM1]`
  - `max_entry_lag_minutes = 5`
  - `split_assignment_mode = EventChronological`
  in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Split assignment already exists end to end:
  - request type in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - core execution in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - Tauri surfaces in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - headless harness in [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)
  - script forwarding in [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)
  - frontend controls in [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- [x] Artifact summaries already expose `request_split_assignment_mode` in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs).
- [x] The fixed sign-off pair is already known:
  - SPY bounded DTE7
  - QQQ bounded DTE7
- [x] The current live `auto` route already stands on both sign-off roots without retuning.

Important nuance:

- Prompt 2 does **not** reopen the overlap-quality pass
- Prompt 2 only carries forward one remaining bounded experiment

---

## 3. Accepted Carry-Forward From Prompt 2

Only the following Prompt 2 conclusions are accepted into this guide as implementation work.

### 3.1 Accepted

- [x] Keep current shipped defaults unchanged for now.
- [x] Do not ship another default search-shape change now.
- [x] Treat `EventChronological` versus `SessionChronological` as the single remaining serious one-factor ablation.
- [x] Treat the split-mode ablation as:
  - measurement-only first
  - non-default
  - cross-root
  - easy to reject
- [x] Keep provider-slice overlap / contract-reference timing truth visible rather than “fixing” it by default pruning.

### 3.2 Not accepted as implementation work

These ideas may appear in the Prompt 2 memo as discussion or rejected alternatives, but they are not work items here:

- lag clamp
- dropping the `5m` delay bucket
- DTE clamp
- moneyness clamp
- broader tuple-grid redesign
- parent-event filtering
- provider-slice curation disguised as a search-default change
- runtime `auto` retuning
- GPU planner/runtime changes
- finalist-boundary changes

---

## 4. Source Validation Filter

This guide is intentionally narrower than the sibling options documents in this folder.

It should be read as:

- a validated implementation subset
- not a wholesale merger of every idea present in every sibling document
- and not authorization to reopen broader options-roadmap decisions already frozen elsewhere

### Accepted into this guide

- code-grounded current truth from:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)
- signed-off overlap-quality results from:
  - [options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md)
  - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/spy_warm/2026-04-08T21-03-50/phase05_summary.json)
  - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/qqq_warm/2026-04-08T21-04-35/phase05_summary.json)
- the validated residue of Prompt 2 in:
  - [options_expression_density_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_02_candidate_density_invalid_slots_and_search_shape_direction/options_expression_density_direction_FINAL.md)

### Explicitly excluded

- any statement that turns the Prompt 2 recommendation into “proved truth”
- any default-profile redesign
- any roadmap-level scope expansion
- any work that depends on:
  - new parent families
  - new structure families
  - new workflow stages
  - CUDA runtime/SASS work

### Practical rule

If a statement is:

- already true in code, mark it as baseline `[x]`
- a bounded experiment still worth trying, mark it as remaining `[ ]`
- speculative, stale, or broadening, exclude it from this guide

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This guide is additive and non-destructive.

It may add:

- repeatable ablation procedure
- optional convenience plumbing for self-labeling ablation outputs
- optional reporting polish if the experiment survives

It may not remove or narrow:

- the current options workflow
- the current defaults
- the current GPU route
- the current overlap/adequacy reporting

### Explicit non-goals

Do not use this pass to:

- change `default_delay_minutes()`
- change `default_dte_buckets()`
- change `default_moneyness_buckets()`
- change `OptionsStaticFilterBundle::default()`
- change `default_split_assignment_mode()`
- change `auto` thresholds
- redesign tuple axes
- re-center provider datasets
- filter parent events out of the benchmark pair
- retune planner/runtime behavior
- add new frontend workflow stages

---

## 6. Benchmark Freeze And Input Lock

This guide depends on a strict one-factor comparison.

### 6.1 Fixed benchmark pair

Use the same sign-off roots already frozen by the overlap-quality pass:

1. SPY bounded DTE7 provider-backed slice
2. QQQ bounded DTE7 provider-backed slice

Representative current warm evidence:

- SPY warm:
  - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/spy_warm/2026-04-08T21-03-50/phase05_summary.json)
- QQQ warm:
  - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/qqq_warm/2026-04-08T21-04-35/phase05_summary.json)

### 6.2 Inputs that must remain locked

For every ablation run, keep all of the following constant:

- parent artifact path
- options dataset id
- backend preference
- delay buckets
- DTE buckets
- moneyness buckets
- hold minutes
- target/stop buckets
- fill model
- filter bundle
- top-k
- sealed finalist count
- warm repeat count
- benchmark iteration count
- output mode (`preview only` or `preview + full run`)

Only change:

- `split_assignment_mode`

### 6.3 Allowed split modes

Only compare:

- `event_chronological`
- `session_chronological`

Do not introduce a third split mode in this pass.

---

## 7. Implementation Phases

## 7.1 Phase 0 — Freeze The Decision Envelope

Before any code change or new benchmark run:

1. freeze the current default-policy surfaces
2. freeze the fixed SPY/QQQ pair
3. freeze the interpretation rule:
   - no shipped default change without a cross-root win

### File touchpoints

- guide/docs only

---

## 7.2 Phase 1 — Confirm Existing Surfaces Instead Of Rebuilding Them

The repo already supports split-assignment comparison.

### What is already owned

- request schema:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- split assignment logic:
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- artifact summary surfacing:
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- script forwarding:
  - [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)

### Implementation rule

Do **not** add new core runtime plumbing unless the existing surfaces prove insufficient.

### Only known ergonomic gap

The preview-only `phase05_summary.json` output is not fully self-describing about split mode. That is a convenience issue, not a capability issue.

---

## 7.3 Phase 2 — Run The One-Factor Split Ablation

Run the fixed pair twice per split mode:

1. SPY cold `event_chronological`
2. SPY cold `session_chronological`
3. QQQ cold `event_chronological`
4. QQQ cold `session_chronological`
5. repeat the same pair warm

### Required command shape

Use the existing phase05 script with explicit output directories.

Recommended pattern:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_options_expression_phase05.ps1 `
  -ArtifactPath "<artifact>" `
  -DatasetId "<dataset-id>" `
  -SplitAssignmentMode event_chronological `
  -Cuda `
  -CompareBackends `
  -WarmRepeats 0 `
  -BenchmarkIterations 3 `
  -OutDir output/options_expression_split_assignment_ablation/<root>_event_cold
```

Repeat with:

- `session_chronological`
- `WarmRepeats 1`
- separate output roots

### Important execution rule

Serialize the runs. Do not overlap them.

---

## 7.4 Phase 3 — Compare Decision-Grade Metrics, Not Just Density

Compare the following for each root and split mode:

- `candidate_density`
- `invalid_slot_rate`
- split overlap rates
- split candidate coverage rates
- split baseline-outcome coverage rates
- dominant invalid reasons overall
- dominant invalid reasons by split
- `best_available_backend`
- `auto_backend_decision`
- `cpu_ranking_us`
- `gpu_ranking_us`
- `gpu_speedup_vs_cpu`
- `replay_build_ms`
- `total_preview_ms`

### Interpretation rule

Do not count the experiment as a win if it only improves:

- `candidate_density`
- or `invalid_slot_rate`

without improving:

- holdout candidate coverage
- holdout baseline-outcome coverage
- sealed candidate coverage
- sealed baseline-outcome coverage

---

## 7.5 Phase 4 — Optional Convenience Plumbing Only If Needed

Only if the preview outputs prove awkward to compare repeatedly, allow a very narrow convenience pass.

### Allowed convenience work

- better split-mode labeling in phase05 outputs
- paired-run wrapper or compare helper
- small report header additions that embed split mode into the summary output

### Primary touchpoints if needed

- [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)
- [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)

### Explicit rule

Do not change core scoring, defaults, or frontend workflow just to make the ablation easier to read.

---

## 7.6 Phase 5 — Final Decision

At the end of the ablation, choose exactly one outcome:

1. **Reject**
   - keep defaults unchanged
   - stop here

2. **Defer**
   - keep defaults unchanged
   - keep split-mode experiment available for later

3. **Promote split mode**
   - only if it wins on both roots
   - only if it improves split-level executable coverage without masking structural weakness

### Strong default expectation

The expected outcome should still be:

- reject or defer

unless the experiment produces a clear cross-root win.

---

## 8. File Ownership Map

### Core execution

- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - current default policy
  - split-assignment request field

- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - `assign_splits(...)`
  - feasibility and run execution
  - session-grouping behavior

### Tauri and artifact summary

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - persisted request
  - artifact summary split-mode reporting

### Scripts and headless tooling

- [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)
- [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)

### Frontend

Only relevant if later reporting polish is justified:

- [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

---

## 9. Acceptance, Rejection, And Stop Rules

## 9.1 Accept promotion only when

All of the following are true:

1. split-level candidate coverage improves on both SPY and QQQ
2. split-level baseline-outcome coverage improves on both SPY and QQQ
3. the improvement is not just a density/invalid-rate cosmetic win
4. QQQ does not remain weak for the same overlap / eligibility reasons
5. preview time does not regress meaningfully
6. no `auto` retuning is required

## 9.2 Reject when

Any of the following occur:

- the change only helps SPY
- the change only improves density/invalid rate
- the change just moves starvation between folds
- QQQ remains structurally weak for the same overlap / eligibility reasons
- preview time regresses materially
- the result would force default retuning outside split assignment

## 9.3 Stop here rule

If the ablation does not satisfy Section 9.1, stop and keep:

- current defaults
- current overlap-quality reporting
- current artifact/request surfaces

Do not force a default change from ambiguous results.

## 9.4 Implementation Outcome (2026-04-09)

The ablation in this guide has now been executed and signed off.

Executed outputs:

- [spy_event_cold](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation/spy_event_cold/2026-04-08T23-57-45/phase05_summary.json)
- [spy_session_cold](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation/spy_session_cold/2026-04-08T23-58-28/phase05_summary.json)
- [qqq_event_cold](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation/qqq_event_cold/2026-04-08T23-59-05/phase05_summary.json)
- [qqq_session_cold](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation/qqq_session_cold/2026-04-08T23-59-45/phase05_summary.json)
- [spy_event_warm](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation/spy_event_warm/2026-04-09T00-00-38/phase05_summary.json)
- [spy_session_warm](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation/spy_session_warm/2026-04-09T00-01-21/phase05_summary.json)
- [qqq_event_warm](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation/qqq_event_warm/2026-04-09T00-02-00/phase05_summary.json)
- [qqq_session_warm](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation/qqq_session_warm/2026-04-09T00-02-39/phase05_summary.json)

Observed decision-grade outcome:

- [x] `session_chronological` did **not** produce a cross-root executable-coverage win.
- [x] SPY improved holdout candidate/baseline-outcome coverage under `session_chronological` (`0.0000 -> 0.4286`) but search coverage weakened (`0.7000 -> 0.6667`).
- [x] QQQ improved search candidate coverage under `session_chronological` (`0.7143 -> 0.8333`) but holdout and sealed candidate/baseline-outcome coverage weakened (`0.5500 -> 0.5238`, `0.5556 -> 0.5263`).
- [x] Density and invalid-slot rate remained unchanged on both roots, so there was no decision-grade coverage improvement hiding behind a density win.
- [x] Adequacy verdicts did not improve:
  - SPY remained `holdout_coverage_weak`
  - QQQ remained `weak_dataset_overlap`

Final decision:

- [x] **Reject promotion** of `session_chronological` as a shipped default.
- [x] Keep `event_chronological` as the default.
- [x] Keep split assignment available as an existing non-default/manual lever.

Additive convenience follow-up completed:

- [x] Feasibility outputs now self-label split mode via `request_split_assignment_mode` in:
  - [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)
  - [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)
- [x] Verification sample:
  - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation_label_check/spy_session_cold/2026-04-09T00-07-43/phase05_summary.json)
  - [phase05_summary.md](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_split_assignment_ablation_label_check/spy_session_cold/2026-04-09T00-07-43/phase05_summary.md)

---

## 10. Proposed Test And Verification Surfaces

### Existing Rust tests already relevant

- session grouping tests in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- artifact summary exposure tests in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)

### Existing frontend tests already relevant

- [optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
- [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
- [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)

### New tests only if code changes

Add new tests only if:

- phase05 summary labeling changes
- artifact summary/report formatting changes
- frontend comparison/reporting changes

Do not create new test churn for a pure measurement-only pass.

---

## 11. Implementation Checklist

Checklist semantics:

- `[x]` already true / already completed / already frozen
- `[ ]` remaining work for this guide

### Validated baseline and frozen constraints

- [x] The overlap-quality hardening pass is already implemented and signed off.
- [x] The current default search-shape surfaces are already owned in code.
- [x] Split assignment is already configurable end to end.
- [x] Artifact summaries already expose split assignment mode.
- [x] This guide does not authorize another shipped default search-shape change by default.
- [x] The fixed benchmark pair is SPY bounded DTE7 plus QQQ bounded DTE7.

### Phase 0 — Decision freeze

- [x] Freeze the current default delay, DTE, moneyness, lag, and split-mode surfaces as baseline.
- [x] Freeze the fixed SPY/QQQ sign-off pair as the decision envelope.
- [x] Freeze the one-factor rule: only `split_assignment_mode` may vary.

### Phase 1 — Existing-surface confirmation

- [x] Confirm the request schema already carries `split_assignment_mode`.
- [x] Confirm `assign_splits(...)` already implements both split modes.
- [x] Confirm the phase05 script already forwards `-SplitAssignmentMode`.
- [x] Confirm artifact summaries already expose `request_split_assignment_mode`.
- [x] Record that any remaining gap is convenience/self-labeling, not missing execution plumbing.

### Phase 2 — One-factor ablation execution

- [x] Run SPY cold with `event_chronological`.
- [x] Run SPY cold with `session_chronological`.
- [x] Run QQQ cold with `event_chronological`.
- [x] Run QQQ cold with `session_chronological`.
- [x] Run SPY warm with `event_chronological`.
- [x] Run SPY warm with `session_chronological`.
- [x] Run QQQ warm with `event_chronological`.
- [x] Run QQQ warm with `session_chronological`.
- [x] Keep all other request inputs locked.
- [x] Keep runs serialized.

### Phase 3 — Comparison and interpretation

- [x] Compare density and invalid-slot rate for all eight runs.
- [x] Compare split overlap rates for all eight runs.
- [x] Compare split candidate coverage rates for all eight runs.
- [x] Compare split baseline-outcome coverage rates for all eight runs.
- [x] Compare dominant invalid reasons overall and by split.
- [x] Compare backend decision and speed metrics.
- [x] Decide whether any observed improvement is decision-grade or cosmetic only.

### Phase 4 — Optional convenience plumbing

- [x] Only if preview outputs are too awkward to compare, decide whether small split-mode labeling/plumbing is warranted.
- [x] If such plumbing is added, keep it confined to script/report ergonomics.
- [x] Do not change core runtime logic, defaults, or frontend workflow as part of convenience work.

### Phase 5 — Final sign-off

- [x] Decide one outcome only: reject, defer, or promote.
- [x] Reject by default unless the experiment wins on both SPY and QQQ.
- [x] Keep defaults unchanged if the experiment is ambiguous.
- [x] Record the final decision in a follow-up sign-off note or by updating this guide.

---

## 12. Verification Plan

### Required benchmark runs

Use the existing harness:

- [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)

Recommended output naming:

- `output/options_expression_split_assignment_ablation/spy_event_cold`
- `output/options_expression_split_assignment_ablation/spy_session_cold`
- `output/options_expression_split_assignment_ablation/qqq_event_cold`
- `output/options_expression_split_assignment_ablation/qqq_session_cold`
- `output/options_expression_split_assignment_ablation/spy_event_warm`
- `output/options_expression_split_assignment_ablation/spy_session_warm`
- `output/options_expression_split_assignment_ablation/qqq_event_warm`
- `output/options_expression_split_assignment_ablation/qqq_session_warm`

### Required metric comparison

Compare:

- `candidate_density`
- `invalid_slot_rate`
- split overlap rates
- split candidate coverage rates
- split baseline-outcome coverage rates
- dominant invalid reasons overall and by split
- `best_available_backend`
- `auto_backend_decision`
- `cpu_ranking_us`
- `gpu_ranking_us`
- `gpu_speedup_vs_cpu`
- `replay_build_ms`
- `total_preview_ms`

### Required local verification if code changes

Only if code changes are made:

- `cargo test -q -p ta_app_core options_ --lib`
- `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`
- `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`

Only if frontend changes are made:

- `npm run typecheck`
- `npm run test -- optionsExpressionService.test.ts optionsExpressionStore.test.ts OptionsExpressionPanel.test.ts`

---

## 13. Source Basis

Primary source set for this guide:

- [options_expression_density_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_02_candidate_density_invalid_slots_and_search_shape_direction/options_expression_density_direction_FINAL.md)
- [options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md)
- [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/spy_warm/2026-04-08T21-03-50/phase05_summary.json)
- [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/qqq_warm/2026-04-08T21-04-35/phase05_summary.json)
- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)
