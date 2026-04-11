# Options GPU Finalist Boundary - Detailed Implementation Guide

Date: 2026-04-11
Status: validated first-wave implementation completed; exactify/sealed timing decomposition shipped, retained-summary reuse shipped, canonical SPY and QQQ phase05 reruns completed, and the second GPU finalist pass remains intentionally deferred because the wave-one reuse path preserved correctness without widening the broad GPU ranking contract

Verified with:

- `cargo test -q -p ta_app_core feasibility_timing_stats_default_prompt12_fields_to_none_and_omit_serialization --lib`
- `cargo test -q -p ta_app_core exactify_enriches_gpu_like_retained_rows_without_summary_overwrite --lib --features cuda,cuda-backtest-kernel`
- `cargo test -q -p ta_app_core options_expression_only_sealed_evaluates_configured_finalists --lib`
- `cargo test -q -p ta_app_core options_ --lib`
- `cargo test -q -p ta_app_core options_ --lib --features cuda,cuda-backtest-kernel`
- `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`
- `cargo test -q -p ta_desktop_demo_app summarize_options_expression_artifact_reports_validation_verdict_counts --lib`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_options_expression_gpu_promotion_gate.ps1 -ArtifactPath 'C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-07T03-56-14.824717100_00-00.json' -DatasetId 'spy_phase05_2026_03_31_to_2026_04_02_calls_645_665_dte7' -OutDir 'output/prompt12_reruns/spy' -WarmRepeats 1 -BenchmarkIterations 3`
- `powershell -ExecutionPolicy Bypass -File scripts/run_options_expression_gpu_promotion_gate.ps1 -ArtifactPath 'C:\Users\dlisz\AppData\Local\dev.vectoralpha.vectorgrid\runs\vectorgrid_run_2026-04-08T05-54-53.090196400_00-00.json' -DatasetId 'qqq_phase05_2026_01_26_to_2026_01_28_calls_625_640_dte7' -OutDir 'output/prompt12_reruns/qqq' -WarmRepeats 1 -BenchmarkIterations 3`

Observed workload outcome:

- SPY rerun emitted the new exactify/sealed sub-buckets and kept estimated backend preview wall speedup positive at `1.25x`, while the hybrid-ranking speed gate remained below the promotion threshold at `1.31x`.
- QQQ rerun emitted the new exactify/sealed sub-buckets and kept estimated backend preview wall speedup positive at `1.25x`, while the hybrid-ranking speed gate remained below the promotion threshold at `2.34x` versus the current `2.75x` bar.

## Purpose

This document converts the valid Prompt 12 implementation residue in:

- [options_gpu_finalist_boundary_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_12_gpu_resident_finalist_exactification_and_embedded_validation_boundary/options_gpu_finalist_boundary_direction_FINAL.md)

into one repo-grounded execution guide for the current options-expression workflow.

The main correction applied here is important:

- Prompt 12 is right that the next CUDA-first question is the current CPU finalist exactification boundary
- Prompt 12 is right that the first-wave target is exactify, not a broad sealed-stage rewrite
- Prompt 12 is right that exact `extreme` stress should stay CPU-side first
- Prompt 12 is right that `top_k` and `sealed_finalist_count` are containment knobs, not the primary fix
- Prompt 12 is **not** fully right to assume a new finalist-only GPU rescore is the default next step
- the current broad GPU pass already computes and stores most non-median summary metrics once
- therefore the first implementation branch should be:
  - split timing
  - split ownership in code
  - try to reuse existing GPU-computed retained-row summary metrics
  - only then consider a second GPU finalist pass if parity evidence says reuse is insufficient

Quick pointers:

- validated current baseline: [Section 2](#2-validated-current-baseline)
- source validation filter: [Section 3](#3-source-validation-filter)
- adopted implementation direction: [Section 4](#4-final-adopted-direction)
- code touch points: [Section 7](#7-code-touch-points-and-ownership)
- implementation phases: [Section 8](#8-implementation-phases)
- full checklist: [Section 9](#9-implementation-checklist)
- verification plan: [Section 10](#10-verification-plan)

---

## 1. Objective

The next valid Prompt 12 implementation pass should do one thing well:

**reduce unnecessary CPU work inside the current retained-finalist exactification boundary without widening workflow scope, without weakening CPU-oracle correctness, and without pretending that "more GPU" is automatically the right answer.**

That objective breaks down into four parts:

1. **Measurement honesty**
   - stop treating `gpu_exactify_us` as one opaque bucket
   - isolate which retained-row CPU work is actually removable

2. **Code-boundary clarity**
   - separate summary-lane overwrite work from exact-only enrichment
   - separate exactify work from sealed-stage work
   - keep exact `extreme` replay explicit

3. **GPU-first reuse before new GPU work**
   - prefer reusing already-computed retained GPU summary metrics
   - do not default to launching a second GPU finalist pass unless the evidence requires it

4. **Additive behavior preservation**
   - preserve the current options optimization + embedded validation workflow
   - preserve CPU fallback and CPU-oracle behavior
   - preserve current artifact semantics unless a field is explicitly staged as GPU-derived plus CPU-enriched

This guide is not:

- a new workflow-stage guide
- a new strategy-family guide
- a broad product-promotion guide
- a CUDA micro-architecture or SASS guide
- a sealed-stage rewrite guide
- a justification for shrinking `top_k` or `sealed_finalist_count`

---

## 2. Validated Current Baseline

These statements are already true in the repo and should be treated as baseline, not goals.

- [x] GPU broad ranking already exists in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs) via `gpu_scoring::rank_compiled_tuples_gpu(...)`.
- [x] The broad GPU pass already evaluates:
  - search baseline
  - holdout baseline
  - holdout conservative stress
  - holdout harsher stress
- [x] `OptionsGpuTupleAggregate` already carries:
  - `valid_trades`
  - `wins`
  - `total_pnl`
  - `total_premium`
  - `sum_return_on_premium`
  - `sumsq_return_on_premium`
  in [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs).
- [x] `metrics_from_aggregate(...)` already reconstructs:
  - invalid rate
  - expectancy per trade
  - expectancy per premium
  - mean return on premium
  - return volatility on premium
  - win rate
  - total PnL
  but still sets `median_return_on_premium` to `0.0` in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] The GPU path already writes those reconstructed summary metrics into `RankedCompiledTuple.result` before sort/truncate.
- [x] The current GPU path already sorts retained rows, resequences pre-sealed rank, and truncates to `req.top_k.max(1)` before final exactification.
- [x] `exactify_ranked_results(...)` currently overwrites retained-row:
  - `selection_metrics`
  - `holdout_metrics`
  - `stress_metrics`
  - `harsher_stress_metrics`
  and also computes `extreme_stress_metrics` on CPU in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs#L2605).
- [x] `apply_sealed_finalist_stage(...)` currently stays CPU-owned and computes:
  - sealed baseline metrics
  - sealed conservative metrics
  - sealed harsher metrics
  - sealed exact `extreme`
  in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs#L2550).
- [x] Current feasibility timing only exposes coarse finalist buckets:
  - `gpu_tensor_score_us`
  - `gpu_exactify_us`
  - `gpu_sealed_stage_us`
  in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs#L763).
- [x] Current Tauri/report plumbing already serializes those coarse timing fields through feasibility/report surfaces in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs).

Important nuance:

- the repo is **not** missing GPU summary-lane computation for retained rows
- the repo is **not** starting from a CPU-only finalist path
- the current valid gap is narrower:
  - coarse timing
  - tangled exactify ownership
  - unconditional CPU overwrite of summary metrics that the GPU path already computed once

---

## 3. Source Validation Filter

This guide is intentionally narrower than the final-direction markdown in the same folder.

### 3.1 Accepted into this guide

- [x] split coarse finalist timing before moving the boundary
- [x] keep exact `extreme` replay on CPU in wave one
- [x] keep sealed-stage logic CPU-owned in wave one
- [x] keep `top_k` and `sealed_finalist_count` unchanged as containment boundaries
- [x] reject near-full-device finalist replay
- [x] use the current SPY bounded slice as the "do no harm" control
- [x] use the current QQQ bounded slice as the first canonical "is the exactify tail material?" control

### 3.2 Valid, but revised here

- [x] The document's overall "narrow exactify-first boundary reduction" recommendation is valid.
- [x] The document's implied default mechanism is revised:
  - do **not** assume a second GPU finalist pass is step one
  - first try to reuse already-computed retained GPU summary metrics
  - only add a finalist-only GPU pass if reuse proves semantically insufficient and still worth it

### 3.3 Rejected as immediate implementation work

These ideas may appear in the sibling response document, but they are not valid first-wave tasks here:

- any claim that current GPU output only carries wins/PnL/premium-level state
- a default assumption that retained rows must be rescored on GPU after `top_k`
- widening the broad GPU ranking contract before measurement proves it is necessary
- moving sealed baseline/conservative/harsher recomputation to GPU in wave one
- moving exact `extreme` to GPU in wave one
- changing `top_k` or `sealed_finalist_count` to manufacture a performance win
- treating this prompt as authorization for SASS/PTX work

### 3.4 Practical rule

If a proposed step:

- clarifies the current CPU finalist boundary, keep it
- reuses already-computed retained GPU state, prefer it
- adds a second GPU pass only after parity evidence says reuse is not enough, allow it as conditional work
- broadens workflow scope, changes budgets, or rewrites sealed logic first, reject it

---

## 4. Final Adopted Direction

The adopted Prompt 12 direction for this repo is:

**instrument, refactor, and then attempt retained-row summary reuse before considering any new finalist-only GPU pass.**

That translates into this exact order:

1. **Split current exactify timing**
   - isolate CPU summary overwrite cost
   - isolate CPU exact-only enrichment cost
   - isolate CPU exact `extreme` cost

2. **Split current exactify code ownership without changing behavior**
   - make the removable summary overwrite slice explicit
   - make the exact-only slices explicit

3. **Prototype retained-row summary reuse from the already-populated GPU result rows**
   - do not overwrite `selection_metrics`, `holdout_metrics`, `stress_metrics`, and `harsher_stress_metrics` on CPU if the retained GPU-derived values meet parity requirements
   - keep CPU for exact-only fields and exact `extreme`

4. **Only if reuse fails for semantic reasons, and only if the timing split still shows material removable cost, prototype a second finalist-only GPU summary pass**
   - keep this behind a strict evidence gate
   - do not treat it as the default outcome

5. **Leave sealed-stage GPU reduction deferred**
   - unless the instrumentation unexpectedly proves sealed non-extreme work is first-order after exactify is cleaned up

This is the narrowest valid CUDA-first plan that stays additive and honest.

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This implementation plan is explicitly additive.

The intended result is:

- finer finalist timing
- clearer internal ownership boundaries
- less unnecessary CPU overwrite work inside the existing retained finalist path
- preserved artifact/report semantics

The intended result is not:

- removing CPU fallback
- removing CPU-oracle behavior
- removing the sealed stage
- removing exact `extreme`
- changing retained budgets
- changing workflow scope

### Frozen for this pass

Do not change:

- the current options optimization + embedded validation workflow shape
- supported parent families
- supported structure families
- current artifact-driven workflow placement
- `top_k`
- `sealed_finalist_count`
- ranking math and `robust_score`
- current frontend workflow layout
- current auto-backend promotion policy, except for additive timing/report fields

### Deferred for a later prompt family

Do not use Prompt 12 implementation to do:

- SASS/PTX tuning
- CUDA launch-shape retuning
- GPU memory-layout redesign
- generalized retained-row device-state expansion
- broad planner or runtime auto-policy changes

---

## 6. Target End State

When this pass is done, the repo should have all of the following:

1. **Code-visible ownership boundaries**
   - the code should clearly distinguish:
     - retained summary overwrite work
     - retained exact-only enrichment
     - retained exact `extreme`
     - sealed-stage work

2. **Timing-visible finalist cost decomposition**
   - the report should show which slice still consumes CPU time
   - the repo should no longer rely on one opaque `gpu_exactify_us` bucket

3. **A validated retained-row reuse path**
   - the repo should either:
     - successfully reuse retained GPU-derived summary metrics and reduce CPU overwrite cost
     - or prove that reuse is not semantically acceptable and stop there

4. **A strict stop gate before any second GPU finalist pass**
   - only if reuse fails and the remaining summary-compatible CPU slice still matters

5. **No semantic drift**
   - retained tuple identities
   - pre-sealed ordering
   - sealed finalist membership
   - final artifact metrics
   - validation verdicts
   must remain correct

---

## 7. Code Touch Points And Ownership

### 7.1 Core backend

- [x] [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - `gpu_scoring::GpuRankedTuplePass`
  - `gpu_scoring::rank_compiled_tuples_gpu(...)`
  - `exactify_ranked_results(...)`
  - `finalize_ranked_results(...)`
  - `apply_sealed_finalist_stage(...)`
  - `evaluate_compiled_tuple_for_split(...)`
  - `evaluate_exact_stress_for_split(...)`
  - `metrics_from_aggregate(...)`

### 7.2 GPU runtime surface

- [x] [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
  - `OptionsGpuTupleAggregate`
  - `score_options_expression_tensors_gpu_range(...)`
  - `score_options_expression_tensors_gpu_multi_range(...)`

Wave-one rule:

- [x] `OptionsGpuTupleAggregate` was intentionally not widened in wave one because retained-summary reuse proved sufficient for the adopted boundary reduction

### 7.3 Timing/report schema

- [x] [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - `OptionsExpressionFeasibilityTimingStats`

- [x] [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - report serialization/plumbing remained valid without a dedicated file edit because the feasibility report already serializes the timing struct wholesale

### 7.4 Tests

- [x] backend tests in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [x] command/report tests in [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)

---

## 8. Implementation Phases

## 8.1 Phase 0 - Source validation and scope lock

This document already performs the validated subset selection.

- [x] Confirm Prompt 12 is still about the current GPU/CPU finalist boundary.
- [x] Confirm the broad GPU pass already computes non-median summary metrics once.
- [x] Confirm the current invalid recommendation is "default to a second GPU finalist pass."
- [x] Lock wave one to additive boundary reduction only.

## 8.2 Phase 1 - Timing decomposition with no behavior change

Add finer-grained optional timing fields to `OptionsExpressionFeasibilityTimingStats` while preserving the existing coarse fields for backward compatibility.

Recommended new optional fields:

- [x] `gpu_exactify_summary_overwrite_us`
- [x] `gpu_exactify_summary_overwrite_ms`
- [x] `gpu_exactify_enrichment_us`
- [x] `gpu_exactify_enrichment_ms`
- [x] `gpu_exactify_extreme_us`
- [x] `gpu_exactify_extreme_ms`

Optional sealed split only if easy and behavior-neutral:

- [x] `gpu_sealed_non_extreme_us`
- [x] `gpu_sealed_non_extreme_ms`
- [x] `gpu_sealed_extreme_us`
- [x] `gpu_sealed_extreme_ms`

Phase-1 requirements:

- [x] keep existing `gpu_exactify_us` and `gpu_sealed_stage_us`
- [x] preserve the legacy parent buckets as umbrella totals while exposing the major sub-buckets underneath them
- [x] plumb new fields through feasibility/report output without breaking existing consumers

## 8.3 Phase 2 - Exactify helper split with no semantic change

Refactor `exactify_ranked_results(...)` into explicit helper slices before changing ownership.

Suggested internal split:

- [x] helper for retained summary overwrite work
- [x] helper for retained exact-only enrichment
- [x] helper for retained exact `extreme`
- [x] helper for recomputing retained `robust_score`, with validation verdict refresh remaining explicit after sealed-stage completion

The exact helper names can vary, but the boundary must be explicit in code.

Hard rule:

- [x] Phase 2 produced no intended user-visible behavior change beyond the additive timing detail

## 8.4 Phase 3 - Retained summary reuse prototype

This is the first real boundary-reduction phase.

The intended approach is:

- [x] identify the retained rows that came from the GPU path
- [x] keep or expose the already-computed retained summary metrics from that path
- [x] stop CPU-overwriting `selection_metrics`, `holdout_metrics`, `stress_metrics`, and `harsher_stress_metrics` for those rows when parity allows
- [x] keep CPU exact-only enrichment for fields that still require exact replay semantics
- [x] keep CPU exact `extreme`

Important implementation rule:

- [x] do not add a second GPU finalist pass in this phase

## 8.5 Phase 4 - Exact-only enrichment hardening

Once retained summary reuse exists, tighten the CPU exact-only part so it is explicit and auditable.

Minimum responsibilities to keep on CPU:

- [x] exact median-related enrichment if still required for product semantics
- [x] any exact-only field not reconstructible from existing GPU aggregates
- [x] exact `extreme` replay

Phase-4 rule:

- [x] if enrichment still needs full replay of the same lanes and erases most of the time win, record that honestly and stop before deeper CUDA work

## 8.6 Phase 5 - Conditional second GPU finalist pass

Wave-one decision:

- [x] Decide whether retained-summary reuse is sufficient before entertaining a second GPU finalist pass
- [x] Keep the second GPU finalist pass deferred because the retained-summary reuse path preserved parity and did not justify widening the broad GPU contract

Still excluded after the wave-one decision:

- [x] sealed-stage GPU rewrite
- [x] exact `extreme` GPU replay
- [x] broad GPU contract widening without proof

## 8.7 Phase 6 - Stop-or-promote decision

After Phases 1-4, or 1-5 if needed:

- [x] keep the new path only if real preview/run wall time improves
- [x] keep the new path only if semantic parity is maintained
- [x] keep the clearer timing and helper split, and stop before more CUDA work because deeper CUDA expansion is not yet justified by the current evidence

---

## 9. Implementation Checklist

### 9.1 Baseline and scope

- [x] Validate Prompt 12 against current code instead of copying the sibling markdown wholesale.
- [x] Confirm current broad GPU aggregates already include first and second moments for return-on-premium.
- [x] Confirm the current first-wave target is exactify, not sealed-stage GPU migration.
- [x] Confirm this plan is additive and preserves current workflow scope.

### 9.2 Timing schema

- [x] Add optional exactify sub-bucket timing fields to `OptionsExpressionFeasibilityTimingStats`.
- [x] Preserve existing coarse timing fields for backward compatibility.
- [x] Add optional sealed sub-bucket timing fields because the split stayed clean and low-risk.
- [x] Plumb new optional timing fields through report/command serialization.
- [x] Add tests for new timing-field serialization and defaults.

### 9.3 Backend refactor with no semantic change

- [x] Split `exactify_ranked_results(...)` into explicit helper slices.
- [x] Make retained summary overwrite work explicit.
- [x] Make retained exact-only enrichment work explicit.
- [x] Make retained exact `extreme` work explicit.
- [x] Keep sealed-stage work explicit and separate.
- [x] Verify helper split causes no user-visible behavior drift.

### 9.4 Retained summary reuse

- [x] Identify the retained rows whose current summary metrics came from GPU aggregation.
- [x] Prototype reuse of those retained GPU-derived summary metrics.
- [x] Stop CPU-overwriting non-median summary lanes when parity is acceptable.
- [x] Keep exact-only enrichment on CPU.
- [x] Keep exact `extreme` on CPU.
- [x] Recompute `robust_score` after enrichment and keep validation verdict refresh explicit at finalization.
- [x] Add targeted tests proving retained-row parity against the prior CPU-overwrite path.

### 9.5 Conditional fallback path

- [x] Decide that retained-summary reuse is sufficient for the current wave.
- [x] Defer a narrow retained-only second GPU summary pass because it is not yet justified.
- [x] Keep the rejection rule that any future second pass must not widen the broad GPU ranking contract.
- [x] Keep the rejection rule that any future second pass must beat the reuse path on QQQ-like workloads.

### 9.6 Verification and stop gates

- [x] Run retained-row parity checks against the current CPU oracle.
- [x] Compare SPY bounded warm behavior before/after.
- [x] Compare QQQ bounded warm behavior before/after.
- [x] Compare total preview wall time, not just micro-buckets.
- [x] Stop before deeper CUDA work if the result is only bucket-shuffling; current evidence shows additive preview-wall improvement but not enough to justify a second GPU finalist pass.

---

## 10. Verification Plan

### 10.1 Required semantic parity checks

For retained rows, require parity for:

- [x] retained tuple identities
- [x] pre-sealed ordering
- [x] `selection_metrics`
- [x] `holdout_metrics`
- [x] `stress_metrics`
- [x] `harsher_stress_metrics`
- [x] `robust_score`
- [x] sealed finalist membership
- [x] `validation_verdict`
- [x] final artifact leader rows after CPU enrichment completes

Special attention:

- [x] `median_return_on_premium` remains correct after the final CPU enrichment step

### 10.2 Required timing comparisons

Minimum before/after comparisons:

- [x] old vs new `gpu_exactify_us`
- [x] old vs new exactify sub-buckets
- [x] old vs new total finalist CPU time
- [x] old vs new total preview wall time
- [x] old vs new total full-run wall time if a full-run benchmark is practical for the repo perf gates in this pass

### 10.3 Canonical workloads

Use the same canonical evidence pair already referenced by Prompt 12:

- [x] SPY bounded slice as the no-harm control
- [x] QQQ bounded slice as the first canonical "exactify tail matters" control

Optional:

- [x] additional bounded workload not required for this wave because SPY and QQQ already cover the no-harm and tail-materiality controls

### 10.4 Command-level verification

At minimum, run:

- [x] `cargo test -q -p ta_app_core options_ --lib`
- [x] `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`

If the code path changes enough to justify it:

- [x] run the repo's candidate workflow gate
- [x] run the repo's GPU perf gate when a clean benchmark window is available

---

## 11. Acceptance Criteria

This Prompt 12 implementation is successful only if all of these are true:

- [x] the repo gains finer-grained visibility into exactify cost
- [x] the code boundary becomes easier to reason about
- [x] retained CPU overwrite work materially drops on QQQ-like workloads
- [x] SPY-like bounded workloads do not regress meaningfully
- [x] final artifact semantics remain intact
- [x] the implementation does not rely on shrinking retained budgets

If those conditions are not met, the correct outcome is:

- keep the timing split
- keep the helper split
- stop before deeper CUDA work

That is still a valid Prompt 12 result.

---

## 12. Explicit Stop Conditions

Keep the current hybrid boundary beyond measurement/refactor work if any of the following occur:

- the removable retained-summary CPU slice is too small to matter
- retained-summary reuse fails semantic parity
- the CPU exact-only enrichment slice still dominates after summary reuse
- a second GPU finalist pass would require widening the broad GPU ranking contract
- end-to-end preview/run time does not improve clearly on the canonical QQQ control
- SPY no-harm control regresses more than QQQ improves
- the apparent win only comes from changing `top_k` or `sealed_finalist_count`

If any stop condition trips, the repo should keep:

- clearer timing
- clearer code ownership
- the existing hybrid boundary

and defer further CUDA-first finalist work until a later prompt or stronger evidence.
