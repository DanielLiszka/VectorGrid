# Options Expression Overlap-Quality Hardening — Detailed Implementation Guide

Date: 2026-04-08  
Status: implementation guide derived from the validated portions of `options_expression_full_gpu_direction_FINAL.md`

## Purpose

This document converts the validated direction in:

- [options_expression_full_gpu_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_full_gpu_direction_FINAL.md)

into an execution guide with:

- concrete file ownership
- exact implementation phases
- explicit non-goals
- a checklist
- verification and acceptance criteria

Use this together with:

- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)

Quick pointers:

- validated baseline and constraints: [Section 2](#2-validated-current-baseline)
- source-validation filter: [Section 2.1](#21-source-validation-filter)
- detailed implementation phases: [Section 6](#6-implementation-phases)
- full checklist: [Section 10](#10-implementation-checklist)
- verification plan: [Section 11](#11-verification-plan)

---

## 1. Objective

The next options-expression implementation pass should do one thing well:

**make parent-vs-options overlap quality and compiled-coverage adequacy explicit before the user treats a feasibility or artifact result as a statement about GPU quality.**

That objective has two parts:

1. **Coverage-quality hardening**
   - detect overlap weakness early
   - classify split starvation and dominant structural invalidity explicitly
   - persist those signals in feasibility and artifact outputs

2. **Minimal profile hardening**
   - only after the new diagnostics exist
   - only if the benchmark pair proves it
   - clamp or guide the current aggregate defaults narrowly where telemetry already identifies weak slices

This pass is **not** a new runtime/GPU architecture project and is **not** a scope-expansion project.

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not goals.

- [x] The options-expression workflow is already shipped as an artifact-driven follow-up lane.
- [x] Supported parent families already include `DoubleMa`, `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`.
- [x] Supported structure families already include `long_single`, `debit_vertical`, and `long_pair`.
- [x] CPU reference scoring and CUDA compiled-tensor scoring both exist.
- [x] Holdout, sealed-finalist, and multi-lane stress metrics already exist.
- [x] The current feasibility path already reports:
  - dataset coverage bounds
  - event coverage
  - invalid-reason totals
  - axis coverage
  - best available backend
  - auto-backend decision
- [x] The current runtime `auto` policy is already aligned with the current Phase `0.5` live gate in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] The current GPU scorer is already viable enough to route bounded provider-backed SPY and QQQ slices.
- [x] The main remaining product risk is not “does the GPU scorer exist?” but “is the compiled workload healthy enough that the result is trustworthy?”

Important nuance:

- the repo already has partial visibility into weak slices
- the gap is **stronger classification and promotion of that visibility**
- the gap is **not** “there are no diagnostics today”

## 2.1 Source Validation Filter

This guide is intentionally narrower than the other options documents in this folder.

### Accepted into this implementation guide

- validated current-state facts from:
  - [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
  - [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
  - [options_expression_full_gpu_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_full_gpu_direction_FINAL.md)
- current code-truth from:
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- currently shipped verification surfaces and phase05 evidence

### Explicitly excluded from this implementation guide

- stale artifact conclusions that no longer match the current live `auto` policy
- exploratory prompt-pack ideas that were never adopted into current scope
- broader scope expansion from the future roadmap
- any recommendation that depends on:
  - planner retuning
  - runtime/GPU micro-architecture work
  - new structure families
  - new parent families
  - new workflow stages

### Practical rule

If a statement from the sibling docs is:

- already true in code, it may appear here as a baseline `[x]`
- clearly useful but not yet implemented, it may appear here as a remaining `[ ]` task
- stale, exploratory, or out of scope, it should **not** appear here as implementation work

---

## 3. Implementation Scope For This Pass

### In scope

- overlap-window summary derived from parent events vs imported options dataset coverage
- split-level adequacy classification for:
  - search
  - holdout
  - sealed
- split-level invalid-reason breakdowns using existing structural candidate invalidity
- explicit feasibility verdicts for weak overlap / weak coverage / fragile slices
- persistence of the new coverage-quality signals in saved options artifacts
- minimal frontend surfacing in the existing options panel and artifact comparison flows
- conditional narrow default-profile guidance or clamps only if the benchmark pair proves they improve real coverage

### Out of scope

- new structure families
- new parent families
- new workflow stages beyond the current optimization + embedded validation lane
- direct quote-aware provider-download work
- runtime chunking changes
- GPU planner retuning
- SASS/PTX work
- moving more finalist exactification onto GPU
- broad search-grid redesign
- threshold lowering merely to make more workloads look GPU-worthy

### Additive posture

This plan is intentionally **additive**, not subtractive.

The intended result is:

- stronger classification
- stronger reporting
- optional narrow default guidance/clamps only if proven

The intended result is **not**:

- removing supported functionality
- narrowing the shipped workflow surface
- removing current realism lanes
- removing current parent or structure families
- downgrading GPU usage

The only allowed “tightening” is:

- conditional default de-emphasis or clamp behavior inside the current aggregate lane
- and only when it improves weak slices without reducing meaningful event/outcome coverage

---

## 4. Explicit Non-Goals

The following are intentionally frozen for this pass.

### 4.1 Keep the current product boundary

Do not change:

- the current unified History follow-up host
- the current parent-family surface
- the current structure-family surface
- the current Polygon/Massive aggregate-lane posture
- quote-aware and derived-mid as secondary realism lanes

### 4.2 Keep the current runtime and planner shape

Do not change:

- `OptionsGpuChunkPlan::for_inputs(...)`
- `ensure_shared_inputs_chunk(...)`
- `ensure_outcome_inputs(...)`
- `ensure_output_buffers(...)`
- `score_ranges(...)`

in [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs) unless a later profiling pass proves they are first-order bottlenecks.

### 4.3 Keep the current auto route steady

Treat the current `auto` thresholds as baseline for this pass. This guide is about coverage quality, not about moving the goalposts.

### 4.4 Keep the current GPU/CPU finalist boundary steady

Do not use this pass to:

- move finalist exactification onto GPU
- move sealed replay onto GPU
- remove CPU finalist replay

That is a later question if coverage-quality hardening proves insufficient.

---

## 5. Target End State

When this pass is done, a user should be able to tell, from the existing options workflow:

- whether the parent artifact meaningfully overlaps the options dataset window
- whether search / holdout / sealed folds still retain meaningful executable opportunity
- whether the slice is structurally weak because of:
  - overlap starvation
  - contract eligibility timing
  - late-entry failure
  - weak strike/moneyness coverage
- whether the result is merely GPU-eligible or actually coverage-adequate
- whether a saved artifact came from a healthy slice or from a fragile one

The user should not have to infer all of that from:

- raw invalid-reason totals alone
- backend choice alone
- or one strong SPY-style scorer result alone

---

## 6. Implementation Phases

## 6.1 Phase 0 — Freeze The Benchmark And Decision Envelope

Before editing behavior, freeze the evaluation pair and the interpretation rules.

### Required benchmark pair

1. SPY bounded DTE7 provider-backed slice
2. QQQ bounded DTE7 provider-backed slice

Representative current evidence:

- SPY evidence: [spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json)
- QQQ evidence: [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json)

### Implementation rule

Do not change all of the following in the same pass:

- overlap-quality reporting
- default-profile guidance/clamps
- live `auto` thresholds
- benchmark slices

This pass should isolate coverage-quality work from planner/routing churn.

### File touchpoints

- doc/reference only
- no code changes yet

---

## 6.2 Phase 1 — Add Explicit Coverage-Quality Report Types

Add new report types to [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs) so the current partial signals become explicit contract surfaces.

### New types to add

1. `OptionsExpressionOverlapWindowSummary`
   - purpose:
     - summarize parent-event overlap with dataset coverage bounds
   - recommended fields:
     - `total_events_in_dataset_window`
     - `search_events_in_dataset_window`
     - `holdout_events_in_dataset_window`
     - `sealed_test_events_in_dataset_window`
     - `total_events_before_dataset_window`
     - `total_events_after_dataset_window`
     - `search_events_before_dataset_window`
     - `search_events_after_dataset_window`
     - `holdout_events_before_dataset_window`
     - `holdout_events_after_dataset_window`
     - `sealed_test_events_before_dataset_window`
     - `sealed_test_events_after_dataset_window`
     - `total_overlap_rate`
     - `search_overlap_rate`
     - `holdout_overlap_rate`
     - `sealed_test_overlap_rate`

2. `OptionsExpressionSplitInvalidReasonSummary`
   - purpose:
     - summarize structural invalidity by split, not just overall
   - recommended fields:
     - `search_invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>`
     - `holdout_invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>`
     - `sealed_test_invalid_reasons: Vec<OptionsCandidateInvalidReasonCount>`

3. `OptionsExpressionCoverageAdequacyVerdict`
   - recommended variants:
     - `Healthy`
     - `WeakDatasetOverlap`
     - `SearchOverlapMissing`
     - `SearchCoverageWeak`
     - `HoldoutCoverageWeak`
     - `SealedCoverageWeak`
     - `ContractEligibilityFragile`
     - `LateEntryFragile`
     - `StrikeWindowFragile`
     - `MixedFragile`

4. `OptionsExpressionCoverageAdequacySummary`
   - purpose:
     - turn existing counts into a verdict the user can reason about
   - recommended fields:
     - `verdict: OptionsExpressionCoverageAdequacyVerdict`
     - `non_promotable: bool`
     - `reasons: Vec<String>`
     - `search_candidate_coverage_rate`
     - `holdout_candidate_coverage_rate`
     - `sealed_test_candidate_coverage_rate`
     - `search_baseline_outcome_coverage_rate`
     - `holdout_baseline_outcome_coverage_rate`
     - `sealed_test_baseline_outcome_coverage_rate`

### Existing types to extend

- `OptionsExpressionFeasibilityReport`
  - add:
    - `overlap_window`
    - `split_invalid_reasons`
    - `coverage_adequacy`

- `OptionsExpressionRunResult`
  - add only the minimum persisted fields needed for reopen/compare flows:
    - `overlap_window`
    - `coverage_adequacy`
  - keep `split_invalid_reasons` feasibility-first unless saved-artifact reopen genuinely needs it

- `OptionsExpressionArtifactV1`
  - persist only the minimum run-level coverage-quality fields required for later reopen/comparison

### Design rule

Do **not** duplicate fields already present in:

- `OptionsExpressionDiagnostics`
- `OptionsExpressionFeasibilityEventCoverage`

The new types should summarize and classify the existing data, not re-invent it.

### Primary file touchpoints

- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

---

## 6.3 Phase 2 — Derive Overlap And Adequacy In App-Core

Add the actual overlap/adequacy math to [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).

### Why app-core is the right home

The relevant inputs already exist here:

- normalized `OptionsSignalEventManifestV1`
- normalized `ImportedOptionsDataset`
- split-assigned events
- structural candidate invalid reasons
- event-coverage counts
- axis coverage

This avoids duplicating classification logic at the Tauri boundary.

### New helper responsibilities

Add small private helpers near the feasibility/report section:

1. overlap summary helper
   - example responsibility:
     - count manifest events inside / before / after `dataset.manifest.coverage_start` and `coverage_end`

2. split invalid-reason helper
   - example responsibility:
     - aggregate `baseline_candidate_cache.invalid_reasons` by `event.fold`

3. adequacy verdict helper
   - example responsibility:
     - derive a stable verdict from:
       - overlap summary
       - event coverage
       - dominant invalid reasons
   - keep this deterministic and explainable

### Recommended verdict policy

Use simple deterministic rules, not opaque scoring.

Recommended ordering:

1. `SearchOverlapMissing`
   - if search overlap is zero
   - this should be `non_promotable = true`

2. `WeakDatasetOverlap`
   - if total overlap or search overlap is materially low even before structural compilation

3. `SearchCoverageWeak`
   - if search events exist in-window but few reach executable candidates/outcomes

4. `HoldoutCoverageWeak`
   - if search looks healthy but holdout is meaningfully starved

5. `SealedCoverageWeak`
   - if search/holdout look healthy but sealed remains too weak to support the current embedded validation story

6. `ContractEligibilityFragile`
   - if `ContractNotEligibleAsOfEvent` is dominant

7. `LateEntryFragile`
   - if `EntryPastCutoff` is dominant

8. `StrikeWindowFragile`
   - if `NoStrikeInTolerance` is dominant

9. `MixedFragile`
   - if the slice is weak for more than one reason and no single verdict is dominant

10. `Healthy`
   - only when overlap, candidate coverage, and baseline-outcome coverage are all materially adequate

### Important implementation rule

Use the new verdict to improve reporting and trust. Do **not** let this silently mutate:

- `evaluate_auto_backend_decision(...)`
- `runtime_auto_backend_decision(...)`

in this pass.

### Primary file touchpoints

- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

### Existing symbols to use, not rewrite

- `assign_splits(...)`
- `compile_structural_candidate_cache(...)`
- `split_coverage_counts(...)`
- `analyze_options_expression_feasibility_blocking(...)`
- `run_options_expression_blocking(...)`

---

## 6.4 Phase 3 — Tauri Persistence And Summary Plumbing

Once the app-core fields exist, plumb them through the existing artifact and feasibility surfaces in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs).

### What to persist

Persist in the full saved artifact:

- overlap summary
- coverage adequacy summary
- optional split invalid-reason summary only if reopen/comparison truly needs it

Persist in the lightweight artifact summary at minimum:

- `coverage_adequacy_verdict`
- `coverage_non_promotable`
- `search_overlap_rate`
- `holdout_overlap_rate`
- `sealed_test_overlap_rate`

### What not to do here

Do not rebuild the overlap math in `build_options_expression_replay_inputs(...)`.

The Tauri layer should:

- call the app-core logic
- serialize the result
- reuse it in artifact summaries and list/open flows

Only use the Tauri boundary for:

- path validation
- run artifact loading
- dataset loading
- persistence

### Primary file touchpoints

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)

### Related command surfaces

- `analyze_options_expression_feasibility(...)`
- `run_options_expression(...)`
- artifact summary/list/open helpers

---

## 6.5 Phase 4 — Frontend Surfacing In The Existing Panel

Make the new signals visible in the current unified options panel without creating a new workflow.

### Minimum UI additions

1. Feasibility preview
   - show:
     - overlap summary
     - coverage adequacy verdict
     - non-promotable warning
     - split-local invalid dominance if useful

2. Reopened artifact view
   - show:
     - stored adequacy verdict
     - stored overlap rates
     - whether the artifact came from a weak slice

3. Saved-artifact comparison
   - add compact columns for:
     - adequacy verdict
     - non-promotable flag
     - search overlap rate
     - holdout overlap rate
     - sealed overlap rate

### Design rule

Do not add another dashboard layer.

Integrate the new signals into:

- the existing feasibility summary
- the existing reopened artifact detail
- the existing saved-artifact comparison table

### Primary file touchpoints

- [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

---

## 6.6 Phase 5 — Conditional Narrow Default-Profile Hardening

This phase is **conditional**. Do not do it until Phases 1-4 exist and the benchmark pair confirms a real improvement path.

### Allowed interventions

Only narrow interventions already implied by telemetry:

- stronger guidance around existing `delay_minutes` values that chronically produce `EntryPastCutoff`
- stronger guidance around existing `dte_buckets` / `moneyness_buckets` combinations that chronically produce `NoStrikeInTolerance`
- tighter use of existing `OptionsStaticFilterBundle` knobs only if the benchmark pair proves it:
  - `max_entry_lag_minutes`
  - `min_premium`
  - `max_premium`
  - `min_volume`
  - `min_open_interest`
- optional default clamp or de-emphasis of clearly weak profile pieces

### Forbidden interventions

Do not:

- redesign the tuple grid
- add new axes
- retune `auto`
- remove difficult cases just to improve ratios

### Adoption gate

Only land a clamp or default change if all of the following are true:

1. it improves weak-slice density or invalid-slot rate materially
2. it does **not** reduce split-level event/outcome coverage
3. it does **not** require threshold relaxation to keep GPU promoted
4. it generalizes across the fixed benchmark pair

### Primary file touchpoints

- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

---

## 7. File Ownership Map

### App-core

- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - new report structs and enums

- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - overlap summary math
  - split invalid-reason aggregation
  - adequacy verdict derivation
  - feasibility population
  - run-result population

### Tauri

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - persistence
  - artifact summary plumbing
  - command response serialization

### Frontend

- [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
  - type normalization

- [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
  - state and comparison shape

- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - actual user-visible surfacing

### Tests

- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - add unit tests near the current feasibility tests

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - add command serialization/persistence tests

- [optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)
- [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
- [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)

---

## 8. Acceptance And Stop Rules

## 8.1 Accept this pass when

All of the following are true:

1. weak-overlap slices are explicitly identified
2. healthy vs fragile slices are visible without inferring from raw invalid totals
3. saved artifacts preserve the new adequacy signal
4. SPY remains healthy
5. QQQ becomes more interpretable, even if it remains near the current gate edge
6. no runtime/planner/SASS work was required

## 8.2 Stop or defer if

Any of the following occur:

- the work starts drifting into grid redesign
- the work starts drifting into threshold relaxation
- the work starts drifting into replay/build amortization
- the work starts drifting into CUDA micro-architecture work
- the work improves ratios but harms split-level event/outcome coverage

## 8.3 Conditional clamp adoption rule

Only adopt Phase 5 if the benchmark pair shows a real improvement. Otherwise stop at Phases 1-4 and ship the diagnostics/reporting improvement alone.

---

## 9. Proposed Test Additions

### App-core tests

Add focused tests for:

- overlap summary when all events are in-window
- overlap summary when search is fully out-of-window
- overlap summary when only holdout or sealed is starved
- adequacy verdict for:
  - healthy slice
  - search overlap missing
  - holdout weak
  - contract eligibility fragile
  - late-entry fragile
  - strike-window fragile
- split invalid-reason aggregation by fold

Recommended location:

- alongside the existing feasibility tests in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

### Tauri tests

Add tests for:

- serialization of the new feasibility fields
- artifact persistence of the new fields
- artifact summary projection of the new compact coverage-quality columns

Recommended location:

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)

### Frontend tests

Add tests for:

- normalization of the new feasibility payload sections
- store exposure of adequacy verdict and overlap rates
- panel rendering of:
  - non-promotable warning
  - adequacy verdict
  - overlap summary
  - saved-artifact comparison columns

---

## 10. Implementation Checklist

Checklist semantics:

- `[x]` already true / already completed / already frozen for this pass
- `[ ]` remaining implementation work

### Validated baseline and frozen constraints

- [x] The implementation guide only carries forward code-grounded and validated portions of the sibling options docs.
- [x] The current shipped workflow, parent-family surface, structure-family surface, and GPU runtime/planner architecture are baseline, not work items in this pass.
- [x] The current live `auto` thresholds are frozen for this pass.
- [x] The benchmark pair for this pass is SPY bounded DTE7 plus bounded QQQ DTE7.
- [x] The current before-state evidence is pinned to:
  - [spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json)
  - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json)

### Phase 0 — Benchmark freeze

- [x] Freeze the SPY bounded DTE7 slice as the strong scorer control.
- [x] Freeze the bounded QQQ DTE7 slice as the fragility control.
- [x] Record the exact pre-change JSON/Markdown outputs used as before-state evidence.

### Phase 1 — Types and contracts

- [x] Add `OptionsExpressionOverlapWindowSummary` to [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Add `OptionsExpressionSplitInvalidReasonSummary` to [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Add `OptionsExpressionCoverageAdequacyVerdict` to [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Add `OptionsExpressionCoverageAdequacySummary` to [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] Extend `OptionsExpressionFeasibilityReport` with overlap and adequacy fields.
- [x] Extend run/artifact types with the minimum persisted adequacy fields needed for reopen/compare flows.

### Phase 2 — App-core derivation

- [x] Add overlap-window summary derivation in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] Add split invalid-reason aggregation in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] Add adequacy-verdict derivation in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] Populate the new fields in `analyze_options_expression_feasibility_blocking(...)`.
- [x] Populate the persisted run/artifact fields in `run_options_expression_blocking(...)`.
- [x] Keep `evaluate_auto_backend_decision(...)` and `runtime_auto_backend_decision(...)` behavior unchanged in this pass.

### Phase 3 — Tauri persistence

- [x] Plumb the new fields through [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs).
- [x] Persist the new fields in `OptionsExpressionArtifactV1`.
- [x] Add compact adequacy/overlap columns to `OptionsExpressionArtifactSummary`.
- [x] Keep replay-input reconstruction logic stable except for serialization/plumbing needs.

### Phase 4 — Frontend surfacing

- [x] Normalize the new feasibility/report fields in [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts).
- [x] Surface adequacy and overlap state in [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts).
- [x] Add a feasibility adequacy block to [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue).
- [x] Add reopened-artifact adequacy display to [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue).
- [x] Add compact adequacy/overlap columns to saved-artifact comparison in [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue).

### Phase 5 — Conditional default-profile hardening

- [x] Review the new benchmark outputs for chronic `EntryPastCutoff` weakness.
- [x] Review the new benchmark outputs for chronic `NoStrikeInTolerance` weakness.
- [x] Decide not to adopt a default guidance or clamp change in this pass because the late-entry weakness is not paired with a stable cross-root strike-window weakness.
- [x] Keep the current default profile unchanged because no candidate clamp was proven to improve weak slices without reducing split-level candidate/outcome coverage.
- [x] Keep `auto` thresholds unchanged as part of this pass.

Phase 5 sign-off outcome:

- SPY remains a healthy overlap slice with `52.5%` density, `47.5%` invalid-slot rate, `100.0%` search overlap, and a `holdout_coverage_weak` verdict driven by `entry_past_cutoff` (`76.6%` dominant-invalid share).
- QQQ remains an interpretable but fragile slice with `25.1%` density, `74.9%` invalid-slot rate, `23.0%` search overlap, `50.5%` total overlap, and a `weak_dataset_overlap` verdict driven primarily by `contract_not_eligible_as_of_event` (`89.4%` dominant-invalid share).
- `EntryPastCutoff` is reproducibly present in both roots and is now surfaced explicitly by the new adequacy reporting.
- `NoStrikeInTolerance` is meaningful in SPY but negligible in QQQ, so it is not a stable cross-root default-profile problem.
- The correct implementation result for this pass is to ship the new overlap/adequacy diagnostics and keep the current aggregate-lane defaults steady.

### Phase 6 — Verification and sign-off

- [x] Run Rust unit tests for app-core feasibility/reporting changes.
- [x] Run Rust tests for Tauri command serialization/persistence changes.
- [x] Run frontend typecheck and targeted options tests.
- [x] Run the fixed SPY benchmark pair cold.
- [x] Run the fixed QQQ benchmark pair cold.
- [x] Run the same pair warm.
- [x] Compare density, invalid-rate, overlap, and split-coverage deltas.
- [x] Confirm no meaningful preview-time regression.
- [x] Confirm the current live `auto` route still stands without retuning.

---

## 11. Verification Plan

### Required local verification

- `cargo test -q -p ta_app_core options_ --lib`
- `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`
- `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
- `npm run typecheck`
- `npm run test -- optionsExpressionService.test.ts optionsExpressionStore.test.ts OptionsExpressionPanel.test.ts`

### Required benchmark reruns

Use the existing scripts:

- [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)
- [run_options_expression_gpu_promotion_gate.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_gpu_promotion_gate.ps1)

Recommended sequence:

1. cold SPY feasibility/promotion run
2. cold QQQ feasibility/promotion run
3. warm SPY rerun
4. warm QQQ rerun

Executed sign-off evidence:

1. SPY cold:
   - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/spy_cold/2026-04-08T21-01-49/phase05_summary.json)
2. QQQ cold:
   - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/qqq_cold/2026-04-08T21-03-05/phase05_summary.json)
3. SPY warm:
   - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/spy_warm/2026-04-08T21-03-50/phase05_summary.json)
4. QQQ warm:
   - [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/qqq_warm/2026-04-08T21-04-35/phase05_summary.json)

Sign-off readout:

- SPY cold/warm remained stable on density, invalid rate, overlap, split coverage, and backend selection.
- QQQ cold/warm remained stable on density, invalid rate, overlap, split coverage, and backend selection.
- Warm reruns showed `100%` structural and summary cache reuse for both roots.
- No meaningful preview-time regression was introduced by the overlap-quality reporting and persistence changes.
- The live `auto` route still recommends the accelerated backend on both bounded roots without threshold retuning.

### Metrics to compare before vs after

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

---

## 12. Source Basis

Primary source set for this guide:

- [options_expression_full_gpu_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_full_gpu_direction_FINAL.md)
- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json)
- [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json)
- [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/spy_cold/2026-04-08T21-01-49/phase05_summary.json)
- [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/spy_warm/2026-04-08T21-03-50/phase05_summary.json)
- [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/qqq_cold/2026-04-08T21-03-05/phase05_summary.json)
- [phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/options_expression_overlap_quality_signoff/qqq_warm/2026-04-08T21-04-35/phase05_summary.json)
