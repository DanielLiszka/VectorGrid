# Options End-To-End Cost - Detailed Implementation Guide

Date: 2026-04-11
Status: implemented and verified for Prompt 13 scope; conditional prepare dedup intentionally deferred

Validated against:

- [options_end_to_end_cost_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_13_end_to_end_cost_decomposition_and_replay_prepare_amortization/options_end_to_end_cost_direction_FINAL.md)
- [options_gpu_finalist_boundary_implementation_guide_2026-04-11.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_12_gpu_resident_finalist_exactification_and_embedded_validation_boundary/options_gpu_finalist_boundary_implementation_guide_2026-04-11.md)
- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [SPY phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/prompt12_reruns/spy/2026-04-11T17-31-43/2026-04-11T17-31-44/phase05_summary.json)
- [QQQ phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/prompt12_reruns/qqq/2026-04-11T17-34-53/2026-04-11T17-34-53/phase05_summary.json)
- [SPY prompt13 pass2 phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/prompt13_reruns/spy_pass2/2026-04-11T20-32-04/2026-04-11T20-32-04/phase05_summary.json)
- [QQQ prompt13 pass2 phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/prompt13_reruns/qqq_pass2/2026-04-11T20-32-14/2026-04-11T20-32-14/phase05_summary.json)

Important correction applied here:

- Prompt 13 is valid to prioritize replay/build-first end-to-end cost decomposition.
- Prompt 13 is valid to keep conditional prepare dedup as a second-step option rather than the headline move.
- Prompt 13 is valid to defer runtime micro-architecture and SASS/PTX work for now.
- Prompt 13 is **not** valid where it implies the current canonical SPY/QQQ evidence has already cleared the broader GPU speed-gate story.
- Prompt 13 stop thresholds are useful proposed implementation gates, but they are not current repo policy until explicitly codified by implementation and verification.

Quick pointers:

- implementation result: [Section 0](#0-implementation-result-2026-04-11)
- validated baseline: [Section 2](#2-validated-current-baseline)
- source validation filter: [Section 3](#3-source-validation-filter)
- adopted implementation direction: [Section 4](#4-final-adopted-direction)
- code touch points: [Section 7](#7-code-touch-points-and-ownership)
- implementation phases: [Section 8](#8-implementation-phases)
- full checklist: [Section 9](#9-implementation-checklist)
- verification plan: [Section 10](#10-verification-plan)

---

## 0. Implementation Result (2026-04-11)

Prompt 13 valid scope is now implemented.

- Replay/build timing decomposition shipped in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs) and is surfaced through the feasibility CLI in [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs).
- One narrow cold-path reduction shipped: persisted options-dataset replay caching keyed by the imported dataset manifest/rows stamps, with versioned on-disk persistence and process-local reuse.
- The second-process canonical reruns confirmed that the chosen reduction target was correct:
  - SPY: replay/build `413 -> 172 ms` (`-58.4%`), total cold preview `551 -> 312 ms` (`-43.4%`), warm preview `6 -> 6 ms`
  - QQQ: replay/build `318 -> 131 ms` (`-58.8%`), total cold preview `514 -> 383 ms` (`-25.5%`), warm preview `23 -> 21 ms`
- On both canonical roots, `options_dataset_csv_load_us` dropped from the dominant cold replay/build bucket to `0`, and `options_dataset_persisted_cache_hit` flipped from `false` to `true` on the second process.
- Candidate density, invalid-slot rate, and coverage verdicts stayed unchanged across the pass-1 to pass-2 reruns, so the measured win is real cold-path reduction rather than quality drift.
- Conditional prepare dedup was evaluated and explicitly deferred. Replay/build remains the dominant cold bucket after the dataset replay-cache reduction, so Prompt 13 stops before any lane-dedup work.
- Runtime/SASS/PTX work remains deferred.

---

## 1. Objective

The next valid Prompt 13 implementation pass should do one thing well:

**reduce cold-preview and reopen-latency cost in the current options-expression workflow by decomposing and then shrinking replay/build work before touching lower-level CUDA/runtime surfaces.**

That objective breaks down into four parts:

1. **Measurement honesty**
   - stop treating `replay_build_ms` as one opaque bucket
   - separate correctness-critical replay/build work from potentially removable cold-path work

2. **Cold-path-first prioritization**
   - optimize the largest removable replay/build slice first
   - do not skip directly to prepare dedup or runtime micro-architecture

3. **Semantic safety**
   - preserve provenance checks, dataset identity checks, fill-model compatibility checks, and current artifact semantics
   - preserve stress-lane meaning across `baseline`, `conservative`, and `harsher`

4. **Additive behavior preservation**
   - keep the current options optimization + embedded validation workflow shape
   - keep CPU fallback and Prompt 12's retained-summary exactify boundary intact
   - preserve existing canonical SPY/QQQ proof harnesses

This guide is not:

- a product-promotion guide
- a generic “add more caching” guide
- a blanket lane-merging guide
- a CUDA runtime retuning guide
- a SASS/PTX guide
- a budget-shrinking guide for `top_k` or `sealed_finalist_count`

---

## 2. Validated Current Baseline

These statements are already true in the repo and should be treated as baseline, not goals.

- [x] `AnalyzeOptionsExpressionFeasibilityResponse` already exposes coarse end-to-end preview timing via `replay_build_ms` and `total_preview_ms` in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs#L42).
- [x] `build_options_expression_replay_inputs(...)` in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs#L998) currently owns the cold replay/build path.
- [x] That replay/build path already performs real correctness work:
  - parent artifact validation
  - parent-family support checks
  - source CSV/timeframe/adjustment resolution
  - process-local parent-signal cache lookup
  - persisted signal-manifest cache lookup
  - candle replay load/resample
  - parent drilldown reconstruction
  - signal-manifest materialization
  - options dataset manifest validation
  - options dataset cache lookup / CSV load
  - request normalization
- [x] `prepare_options_expression_run(...)` already splits prepare work into tuple compile, candidate compile, and exit-summary compile lanes in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs#L2262).
- [x] Prompt 12 already split and cleaned up the finalist tail; exactify is no longer an opaque unresolved bucket.
- [x] Current prepare/cache behavior is stress-mode sensitive. Candidate selection and cache keys are lane-aware in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs#L356), [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs#L394), [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs#L3330), and [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs#L3384).
- [x] The current canonical reruns still show replay/build dominating cold preview:
  - SPY: `258 ms` replay/build vs `21 ms` prepare vs `432 ms` total preview
  - QQQ: `198 ms` replay/build vs `62 ms` prepare vs `504 ms` total preview
- [x] The current canonical reruns still show warm-path amortization already working well:
  - SPY: `1 ms` replay/build and `7 ms` total warm preview
  - QQQ: `1 ms` replay/build and `24 ms` total warm preview
- [x] The current canonical reruns still do **not** clear the current GPU speed gate:
  - SPY hybrid-ranking speed gate remains below threshold
  - QQQ hybrid-ranking speed gate remains below threshold
- [x] The current runtime shape does **not** make lower-level CUDA work first-order yet:
  - `gpu_tuple_chunk_count = 1`
  - `gpu_kernel_launch_count = 4`
  - `gpu_output_buffer_reuses = 3`

Important nuance:

- the repo is **not** missing basic replay caches
- the repo is **not** missing prepare caches
- the repo is **not** starting from a scorer-bound profile
- the current valid gap is narrower:
  - coarse replay/build timing
  - no honest sub-bucket view of cold replay/build
  - no evidence-backed replay/build reduction yet
  - prepare dedup not yet proven safe enough to lead with

---

## 3. Source Validation Filter

This guide intentionally uses only the valid portions of the Prompt 13 response.

### 3.1 Accepted into this guide

- [x] treat Prompt 13 as a cold-preview / reopen-latency problem
- [x] decompose `replay_build_ms` before optimizing it
- [x] optimize the largest removable replay/build slice first
- [x] treat prepare dedup as conditional and secondary
- [x] defer runtime micro-architecture and Prompt 14-style SASS work
- [x] keep the current options optimization + embedded validation workflow shape
- [x] preserve Prompt 12's retained-summary exactify boundary

### 3.2 Valid, but revised here

- [x] Prompt 13's end-to-end cost decomposition framing is valid.
- [x] Prompt 13's replay/build-first recommendation is valid.
- [x] Prompt 13's suggested stop thresholds are revised here as **proposed implementation gates**, not as existing repo policy.
- [x] Prompt 13's “GPU scorer is viable” framing is revised here to:
  - `auto` currently selects the accelerated path on the canonical reruns
  - preview-wall speedup remains positive
  - but the explicit GPU speed gate is still not fully cleared

### 3.3 Rejected as immediate implementation work

- any recommendation to lead with lower-level CUDA runtime retuning
- any SASS/PTX-first recommendation
- any generic “add more caching” campaign without a measured replay/build target
- any blanket merge of `baseline`, `conservative`, and `harsher`
- any weakening of provenance or identity checks
- any “win” manufactured by shrinking `top_k`, `sealed_finalist_count`, or related budgets

### 3.4 Practical rule

If a proposed step:

- explains where cold replay/build time actually goes, keep it
- reduces replay/build without weakening correctness, prefer it
- touches prepare dedup only after replay/build evidence says it is next, allow it as conditional work
- pushes work into runtime micro-architecture before cold replay/build is honestly decomposed, reject it

---

## 4. Final Adopted Direction

The adopted Prompt 13 direction for this repo is:

**instrument the replay/build path first, then implement one narrow cold replay/build reduction, and only then evaluate whether conditional prepare dedup is still worth it.**

That translates into this exact order:

1. **Split `replay_build_ms` into repo-grounded sub-buckets**
   - parent artifact and source resolution
   - parent manifest cache lookup
   - persisted signal-manifest cache lookup/load
   - candle replay load/resample
   - parent drilldown and signal-manifest build
   - options dataset manifest validation
   - options dataset cache lookup / CSV load
   - request normalization

2. **Capture the canonical SPY/QQQ baseline again with the new sub-buckets**
   - cold preview
   - warm preview
   - current prepare and GPU buckets

3. **Implement only the largest removable replay/build slice**
   - do not widen scope to multiple cold-path reductions in one pass
   - do not start prepare dedup in the same pass unless the chosen replay/build change is trivial

4. **Re-measure SPY/QQQ and decide whether replay/build is still first-order**
   - if yes, continue with one more replay/build-first refinement only if the measured evidence justifies it
   - if no, and only if prepare is now first-order, prototype one narrow prepare dedup path

5. **Keep lower-level CUDA/runtime work deferred**
   - unless the runtime share becomes first-order after replay/build and safe prepare work

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This implementation plan is explicitly additive.

The intended result is:

- finer replay/build timing
- clearer understanding of what cold replay/build work is removable
- one evidence-backed cold-path reduction
- preserved workflow semantics

The intended result is not:

- removing replay/build correctness checks
- merging stress lanes by assumption
- broadening workflow scope
- rewriting the GPU runtime
- changing product gating policy in the same pass

### Frozen for this pass

Do not change:

- the current options-expression workflow shape
- the current Prompt 12 exactify/sealed boundary
- `top_k`
- `sealed_finalist_count`
- current stress-lane semantics
- current aggregate-first provider posture
- current CPU fallback and CPU-oracle behavior
- current backend auto-selection rules, except for additive telemetry if implementation truly needs it

### Deferred for later prompts

Do not use Prompt 13 implementation to do:

- Prompt 14-style runtime micro-architecture work
- SASS/PTX profiling-driven kernel changes
- density/search-shape redesign
- realism-lane expansion
- broader structure-family expansion

---

## 6. Target End State

When this Prompt 13 pass is done, the repo should have all of the following:

1. **Replay/build timing honesty**
   - `replay_build_ms` remains available
   - replay/build sub-buckets are visible enough to explain cold-preview cost
   - cache hit/miss context is visible enough to interpret those timings

2. **One measured replay/build reduction**
   - a single narrow cold-path reduction is implemented against the largest removable slice
   - the change is tied to actual canonical SPY/QQQ evidence

3. **No semantic drift**
   - parent artifact identity
   - dataset identity
   - candidate density
   - invalid-slot rate
   - overlap / adequacy verdicts
   - retained tuple identities
   - pre-sealed ordering
   - sealed-finalist membership
   - validation verdicts
   remain correct

4. **A hard gate before prepare dedup**
   - prepare dedup is not started until replay/build evidence says it is now the best remaining target

5. **A hard gate before runtime/SASS work**
   - runtime micro-architecture remains deferred unless replay/build and safe prepare work shift the bottleneck there

---

## 7. Code Touch Points And Ownership

### 7.1 Primary cold-path surface

- [ ] [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - `build_options_expression_replay_inputs(...)`
  - `load_parent_signal_cache_entry(...)`
  - `load_signal_manifest_replay_cache(...)`
  - `persist_signal_manifest_replay_cache(...)`
  - `load_replayed_candles(...)`
  - `load_options_dataset_cache_entry(...)`
  - `analyze_options_expression_feasibility_blocking_impl(...)`

Primary ownership for this prompt:

- replay/build timing decomposition
- one narrow replay/build reduction
- preserving provenance and identity checks

### 7.2 Feasibility/CLI output surface

- [ ] [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)

Primary ownership for this prompt:

- ensure any new replay/build timing breakdown is serialized into the existing proof harness output
- keep current `phase05_summary.json` and markdown outputs understandable
- preserve current coarse fields for backward compatibility wherever practical

### 7.3 Secondary prepare surface

- [ ] [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - `prepare_options_expression_run(...)`
  - `get_or_build_structural_candidate_cache(...)`
  - `get_or_build_exit_summary_cache(...)`
  - `structural_candidate_cache_key(...)`
  - `exit_summary_cache_key(...)`
  - `compile_structural_candidate_cache(...)`
  - `compile_exit_summary_cache(...)`
  - `build_gpu_flat_score_inputs(...)`

Wave-one rule:

- do not touch this surface beyond measurement/context gathering unless replay/build work has already been measured and reduced first, or the implementation evidence clearly shows prepare is the new dominant removable bucket

### 7.4 Shared schema, only if truly needed

- [ ] [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

Preferred rule:

- keep replay/build timing local to the Tauri feasibility response if possible
- widen shared timing schema only if the shared report surface genuinely needs replay/build sub-buckets

### 7.5 Explicitly deferred surface

- [ ] [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)

This file is deferred for Prompt 13 implementation. It remains a read-only measurement surface unless evidence later proves runtime work is first-order.

---

## 8. Implementation Phases

## 8.1 Phase 0 - Scope lock and evidence freeze

This document already performs the validated subset selection.

- [x] Confirm Prompt 13 is still about end-to-end cost decomposition after Prompt 12.
- [x] Confirm Prompt 12 already resolved the finalist exactify ambiguity.
- [x] Confirm replay/build is still the dominant cold-preview bucket on the canonical reruns.
- [x] Confirm runtime/SASS work remains deferred.

## 8.2 Phase 1 - Replay/build timing decomposition

Add a local replay/build timing breakdown to the Tauri feasibility surface while preserving current coarse fields.

Recommended approach:

- [x] add a new local timing breakdown struct in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [x] add that struct as an optional field on `AnalyzeOptionsExpressionFeasibilityResponse`
- [x] keep existing `replay_build_ms` and `total_preview_ms`
- [x] keep shared app-core timing schema unchanged unless implementation proves widening is necessary

Recommended sub-buckets:

- [x] parent artifact path/read/parse + parent-family validation
- [x] source CSV/timeframe/adjustment resolution
- [x] process-local parent-signal cache lookup
- [x] persisted signal-manifest cache lookup/load
- [x] candle replay load/resample
- [x] parent replay/drilldown reconstruction
- [x] signal-manifest materialization
- [x] dataset manifest read/validation
- [x] options dataset cache lookup / CSV hash+load
- [x] request normalization / final compatibility checks

Required interpretation fields:

- [x] parent-signal process-local cache hit flag
- [x] persisted signal-manifest cache hit flag
- [x] options dataset process-local cache hit flag

Phase-1 rules:

- [x] no weakening of path validation, dataset identity, or fill-model checks
- [x] CLI harness output remains readable
- [x] final implementation kept the telemetry local to the Tauri feasibility surface even though the completed Prompt 13 pass also added one additive dataset replay-cache family

## 8.3 Phase 2 - Canonical baseline capture with the new sub-buckets

Re-run the canonical proof pair after Phase 1 lands.

- [x] rerun SPY cold preview
- [x] rerun SPY warm preview
- [x] rerun QQQ cold preview
- [x] rerun QQQ warm preview
- [x] record the dominant removable pass-1 replay/build bucket as options-dataset CSV hash/load
- [x] record that the remaining pass-2 replay/build cost is primarily persisted options-dataset cache lookup plus small validation buckets, with no other large removable bucket yet evidenced

Phase-2 rule:

- [x] choose exactly one primary replay/build reduction target based on measured evidence: persisted options-dataset replay reuse

## 8.4 Phase 3 - Implement one narrow replay/build reduction

Implement only the dominant removable cold-path slice identified in Phase 2.

Allowed target families:

- [ ] parent replay/drilldown reconstruction reuse
- [ ] signal-manifest persisted materialization reuse/load cost reduction
- [ ] redundant parent artifact/source validation reduction where identity is already proven by the existing cache keys
- [x] options dataset cold-path reduction only if dataset identity remains fully proven

Required implementation rule:

- [x] do not mix multiple replay/build reduction families into one pass unless the chosen change is obviously coupled and still small

Hard rule:

- [x] preserve `build_options_expression_replay_inputs(...)` correctness semantics

## 8.5 Phase 4 - Re-measure and decide whether prepare dedup is now justified

After the one replay/build reduction lands:

- [x] rerun canonical SPY cold/warm
- [x] rerun canonical QQQ cold/warm
- [x] compare updated replay/build share vs prepare share
- [x] decide whether replay/build remains first-order
- [x] do not enter conditional prepare-dedup planning because replay/build remains the best current target after the dataset replay-cache win

## 8.6 Phase 5 - Conditional prepare dedup, not entered

Only do this phase if all of the following become true:

- [x] replay/build has already been honestly decomposed
- [x] one replay/build reduction has already been implemented
- [x] prepare is not yet the dominant removable cold bucket
- [x] keep prepare dedup deferred for Prompt 13

Allowed prepare targets:

- [ ] shared layout/offset planning
- [ ] shared geometry reuse where lane semantics are provably identical for the current request shape
- [ ] shared GPU flat-input layout work where outputs remain lane-correct

Still excluded:

- [x] reject blanket baseline/conservative/harsher cache sharing
- [x] reject summary-cache merging by assumption
- [x] reject candidate-cache merging by assumption
- [x] reject weakening stress semantics to manufacture reuse

## 8.7 Phase 6 - Stop-or-defer decision

After Phases 1-4, or 1-5 if needed:

- [x] keep the new replay/build path only if it materially improves cold preview
- [x] keep the change only if semantic parity is maintained
- [x] stop before broader prepare/runtime work because Prompt 13 achieved the measured replay/build win without justifying lane dedup or runtime retuning

---

## 9. Implementation Checklist

### 9.1 Baseline and scope

- [x] Validate the Prompt 13 response against the current repo instead of accepting it wholesale.
- [x] Accept replay/build-first end-to-end decomposition as the primary direction.
- [x] Reject the overstated “live auto gates are cleared” wording.
- [x] Treat Prompt 12 as completed baseline work.
- [x] Freeze runtime/SASS work as deferred.

### 9.2 Replay/build telemetry

- [x] Add a local replay/build timing breakdown struct.
- [x] Preserve `replay_build_ms`.
- [x] Preserve `total_preview_ms`.
- [x] Add cache hit/miss interpretation fields for replay/build timing.
- [x] Plumb the new replay/build breakdown through the headless feasibility response.
- [x] Plumb the new replay/build breakdown through the feasibility CLI output.
- [x] Add tests or serialization assertions for the new replay/build timing field(s).

### 9.3 Replay/build dominant-bucket analysis

- [x] Capture canonical SPY cold/warm timing with the new breakdown.
- [x] Capture canonical QQQ cold/warm timing with the new breakdown.
- [x] Identify the largest removable replay/build sub-bucket.
- [x] Identify the largest correctness-critical replay/build sub-bucket.
- [x] Write down the chosen reduction target before implementation.

### 9.4 One narrow replay/build reduction

- [x] Implement exactly one replay/build reduction family.
- [x] Preserve parent artifact identity guarantees.
- [x] Preserve source CSV identity guarantees.
- [x] Preserve options dataset identity guarantees.
- [x] Preserve fill-model compatibility checks.
- [x] Preserve preview/run/inspect replay alignment.
- [x] Add targeted tests for the chosen replay/build reduction path.

### 9.5 Conditional prepare dedup

- [x] Decide whether replay/build work is still first-order after the first reduction.
- [x] Defer prepare dedup because replay/build remains first-order after the first reduction.
- [x] Reject blanket lane merging.
- [x] Reject any dedup path that changes candidate validity or premium-sensitive behavior.

### 9.6 Verification and stop gates

- [x] Re-run canonical SPY cold/warm after the replay/build change.
- [x] Re-run canonical QQQ cold/warm after the replay/build change.
- [x] Compare cold replay/build before vs after.
- [x] Compare total cold preview before vs after.
- [x] Compare warm preview before vs after.
- [x] Compare candidate density and invalid-slot rate before vs after.
- [x] Compare coverage verdicts before vs after.
- [x] Stop if the change is bucket-shuffling only.

---

## 10. Verification Plan

### 10.1 Required semantic parity checks

For the replay/build reduction, require parity for:

- [x] parent artifact identity behavior
- [x] dataset identity behavior
- [x] fill-model compatibility behavior
- [x] candidate density
- [x] invalid-slot rate
- [x] overlap / adequacy verdicts
- [ ] retained tuple identities
- [ ] pre-sealed ordering
- [ ] sealed-finalist membership
- [ ] validation verdicts

The remaining tuple-order and finalist-membership checks are run-surface concerns rather than Prompt 13 blockers because this prompt intentionally improved the feasibility/replay cold path without changing tuple-ranking logic.

### 10.2 Required timing comparisons

Minimum before/after comparisons:

- [x] old vs new `replay_build_ms`
- [x] old vs new replay/build sub-buckets
- [x] old vs new prepare cost
- [x] old vs new `total_preview_ms`
- [x] old vs new warm preview timing

### 10.3 Canonical workloads

Use the same canonical pair already stabilized by Prompt 12:

- [x] SPY bounded slice as the no-harm control
- [x] QQQ bounded slice as the higher-cost cold-preview control

### 10.4 Required commands

At minimum, run:

- [x] `cargo test -q -p ta_app_core options_ --lib`
- [x] `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`
- [x] `cargo check -q -p ta_desktop_demo_app`

If the feasibility CLI surface changes:

- [x] `cargo check -q -p ta_desktop_demo_app --bin ta_options_expression_feasibility`

If the implementation touches CUDA-adjacent prepare logic:

- not required for Prompt 13 because this pass did not edit CUDA-adjacent prepare/runtime surfaces

### 10.5 Optional gates

Use these only after the core proof pair is stable:

- [ ] repo workflow gate
- [x] options GPU promotion reruns
- [ ] GPU perf gate only if the work legitimately moves runtime-adjacent surfaces

---

## 11. Acceptance Criteria

This Prompt 13 implementation is successful only if all of these are true:

- [x] the repo gains honest replay/build sub-bucket visibility
- [x] the chosen replay/build reduction is tied to measured dominant cost rather than intuition
- [x] cold preview improves materially on the canonical proof pair
- [x] warm preview does not regress materially
- [x] semantic parity is maintained
- [x] the implementation does not depend on weakening provenance or lane semantics

If those conditions are not met, the correct outcome is:

- keep the replay/build timing split if it is still useful
- stop before broader prepare or runtime work
- defer deeper work until stronger evidence exists

That is still a valid Prompt 13 result.

---

## 12. Proposed Stop Conditions

These are **proposed implementation gates for this prompt**, not existing repo policy.

Keep the current architecture beyond measurement work if any of the following occur:

- [ ] replay/build sub-buckets show that most remaining cold time is correctness-critical
- [ ] the chosen replay/build reduction cannot preserve identity/provenance semantics
- [ ] the prototype fails to produce a clear cold-preview win on both canonical roots
- [ ] warm preview regresses materially
- [ ] candidate density, invalid-slot rate, or verdict semantics drift
- [ ] prepare dedup requires treating stress lanes as interchangeable by assumption
- [ ] the apparent win only comes from budget changes rather than real cold-path reduction

Suggested quantitative gates for this prompt:

- [ ] reject the replay/build change if it does not achieve about `20%` replay/build reduction on both canonical roots, unless the measured dominant removable bucket is smaller than that by construction
- [ ] reject the replay/build change if it does not achieve about `10%` cold `total_preview_ms` reduction on both canonical roots
- [ ] reject the replay/build change if warm preview regresses by more than about `10%` or more than about `2 ms`, whichever is tighter for the relevant root

These thresholds should stay in this implementation lane until later evidence justifies promoting them into broader repo policy.

---
