# Options Single-Pass Metrics - Detailed Implementation Guide

Date: 2026-04-11
Status: implemented and verified for the valid portions of Prompt 9 (`options_single_pass_metrics_direction_FINAL.md`)

## Purpose

This document converts the validated residue of:

- [options_single_pass_metrics_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_09_single_pass_sufficient_statistics_expansion/options_single_pass_metrics_direction_FINAL.md)

into one narrow, implementation-ready plan.

The key point is:

- Prompt 9 does justify a real backend/runtime change
- Prompt 9 does justify a small exact widening of the per-tuple aggregate state
- Prompt 9 does not justify widening workflow stages, ranking math, stress-family breadth, or structure-family breadth
- Prompt 9 does not justify sketches, path-dependent metrics, or replay-heavy post-processing

The valid adopted direction is:

- add exactly two mergeable accumulators to the aggregate path:
  - `sum_return_on_premium`
  - `sumsq_return_on_premium`
- expose exactly two new exact metrics in `OptionsExpressionMetrics`:
  - `mean_return_on_premium`
  - `return_volatility_on_premium`
- keep `median_return_on_premium` exactified-only
- keep `robust_score(...)` unchanged in this implementation wave

This guide exists to make that work explicit, additive, testable, and easy to verify without reopening:

- structure-family scope
- stress-family scope
- workflow shape
- ranking-functional math
- replay-heavy validation depth

This document is not a master consolidation plan for every file copied into the Prompt 9 folder.
It is a validated implementation tracker for the narrow subset of Prompt 9 that legitimately survives into implementation work.

Use this together with:

- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [options_stress_scope_freeze_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_07_stress_family_expansion_constraints/options_stress_scope_freeze_implementation_guide_2026-04-10.md)
- [options_structure_scope_freeze_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_08_bounded_structure_family_expansion_or_freeze/options_structure_scope_freeze_implementation_guide_2026-04-10.md)
- [options_expression_support_aware_validation_semantics_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_support_aware_validation_semantics_implementation_guide_2026-04-10.md)

Quick pointers:

- objective and allowed scope: [Section 1](#1-objective)
- validated current baseline: [Section 2](#2-validated-current-baseline)
- accepted Prompt 9 carry-forward: [Section 3](#3-accepted-carry-forward-from-prompt-9)
- implementation phases: [Section 7](#7-implementation-phases)
- full checklist: [Section 11](#11-implementation-checklist)
- verification plan: [Section 12](#12-verification-plan)

---

## 1. Objective

The valid Prompt 9 follow-up should do one thing only:

**widen the current options-expression sufficient-statistics surface by the smallest exact return-moment pair that materially improves the broad GPU aggregate path without changing workflow shape or ranking math.**

That objective breaks down into:

1. **Aggregate-state widening**
   - keep the current compact aggregate path
   - add exactly two new return-moment accumulators:
     - `sum_return_on_premium`
     - `sumsq_return_on_premium`

2. **Metric-surface widening**
   - add exactly two new exact metrics:
     - `mean_return_on_premium`
     - `return_volatility_on_premium`

3. **Semantic containment**
   - keep `median_return_on_premium` exactified-only
   - keep `robust_score(...)` unchanged
   - keep sketches, downside moments, profit-factor families, and path-dependent metrics out of this pass

This guide is not a ranking-math guide, not a validation-stage guide, and not a generic GPU telemetry expansion guide.
It is a narrow sufficient-statistics implementation plan.

---

## 2. Validated Current Baseline

These statements are already true in the current repo and should be treated as baseline, not goals.

- [x] `OptionsExpressionMetrics` currently exposes eight fields in [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs).
- [x] `compute_metrics(...)` on the CPU path computes full exact metrics and materializes the `return_on_premium` vector to derive an exact median in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] `OptionsGpuTupleAggregate` currently retains only `valid_trades`, `wins`, `total_pnl`, and `total_premium` in [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs).
- [x] `TupleAggregateMetrics` and `metrics_from_aggregate(...)` reconstruct a reduced exact-from-aggregate metric set and intentionally set `median_return_on_premium` to `0.0` on the aggregate path in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] The current GPU ranking path already uses aggregate-derived metrics for search, holdout, conservative stress, and harsher stress lanes in [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs).
- [x] The current CUDA output width is 16 bytes per tuple in [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs#L131).
- [x] The current kernel already accumulates `valid`, `wins`, `total_pnl`, and `total_premium` per tuple in [options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu).
- [x] Prompt 7 and Prompt 8 have already frozen stress-family and structure-family breadth, so Prompt 9 must stay inside the current lane rather than solving the problem by widening product scope.

Important nuance:

- the current broad GPU path does not need more workflow stages
- the current gap is specifically distribution-aware trade-return information that is exact from small state
- the recommended first wave is intentionally smaller than many mathematically possible expansions

---

## 3. Accepted Carry-Forward From Prompt 9

Only the following Prompt 9 conclusions are accepted into this guide as implementation work.

### 3.1 Accepted

- [x] Add `sum_return_on_premium = Σ r_i` to the aggregate state.
- [x] Add `sumsq_return_on_premium = Σ r_i^2` to the aggregate state.
- [x] Add `mean_return_on_premium` to `OptionsExpressionMetrics`.
- [x] Add `return_volatility_on_premium` to `OptionsExpressionMetrics`.
- [x] Compute both metrics exactly on the CPU path from realized outcomes.
- [x] Compute both metrics exactly on the aggregate path from mergeable sufficient statistics.
- [x] Keep `median_return_on_premium` exactified-only.
- [x] Keep `robust_score(...)` unchanged in this wave.
- [x] Preserve backward-compatible artifact deserialization when the new metric fields appear.

### 3.2 Not accepted as implementation work

These ideas may appear in the Prompt 9 memo as discussion or deferred alternatives, but they are not work items here:

- downside deviation in this first wave
- gross-profit / gross-loss / profit-factor families in this first wave
- histogram or quantile sketches
- median approximation in the broad GPU pass
- path-dependent metrics such as drawdown, recovery, MFE, or MAE
- ranking-functional changes
- workflow-stage changes
- structure-family or stress-family expansion

---

## 4. Source Validation Filter

This guide is intentionally narrower than the full set of files present in the Prompt 9 folder.

It should be read as:

- a validated implementation subset
- not a wholesale merger of every copied baseline file in the folder
- and not authorization to treat every mathematically possible accumulator as equally worth implementing

Folder-local note:

- this folder includes the Prompt 9 final response plus copied repo files and baseline references used by the prompt
- only the final response plus current repo code are authoritative for deciding what should actually be implemented now

### Accepted into this guide

- code-grounded current truth from:
  - [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
  - [crates/app_core/src/kernels/options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)
  - [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- aligned supporting context from:
  - [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
  - [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- the validated residue of Prompt 9 in:
  - [options_single_pass_metrics_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_09_single_pass_sufficient_statistics_expansion/options_single_pass_metrics_direction_FINAL.md)

### Explicitly excluded

- any ranking-math change that consumes the new metrics immediately
- any metric that is not exact from small mergeable state
- any sketch-heavy approximation for median or quantiles in this first wave
- any path-order-sensitive statistic
- any product-surface widening used to justify the metric work

### Practical rule

If a statement is:

- already true in code, mark it as baseline `[x]`
- a narrow implementation task needed to add the adopted two-accumulator expansion, mark it as remaining and unchecked in the checklist
- a deferred or speculative expansion beyond the adopted pair, exclude it from this guide

---

## 5. Additive Posture And Explicit Non-Goals

### Additive posture

This guide is additive and non-destructive.

It may add:

- two new aggregate accumulators
- two new exact metrics
- narrow CPU and GPU parity tests
- backward-compatible serde defaults
- current-host contract threading needed to keep the service layer aligned with the widened backend metric contract

It may not remove or narrow:

- current shipped metrics
- current CPU exactification boundary
- current workflow shape
- current stress-family or structure-family set
- current artifact-driven reopen flow

### Explicit non-goals

Do not use this pass to:

- change `robust_score(...)`
- approximate the median in the broad GPU path
- add downside-deviation or profit-factor families in this first wave
- widen artifact summaries or UI tables by default
- add a new validation stage
- add path-dependent metrics
- widen stress lanes or structure families

---

## 6. Target End State

When this pass is complete, the repo should have:

- exact `mean_return_on_premium` and `return_volatility_on_premium` on the CPU path
- exact `mean_return_on_premium` and `return_volatility_on_premium` on the broad aggregate path
- unchanged `median_return_on_premium` semantics:
  - exact on CPU exactification
  - `0.0` placeholder on aggregate-derived broad GPU metrics
- unchanged `robust_score(...)`
- unchanged workflow shape
- backward-compatible artifact loads

And a maintainer should be able to say:

- the broad GPU aggregate is now richer by exactly two return-moment accumulators
- the first-wave state widening remains bounded and exact
- no sketch, replay, or ranking-math drift was introduced by Prompt 9

---

## 7. Implementation Phases

## 7.1 Phase 1 - Metrics Contract Widening

Goal:

extend the shared metric struct with the adopted two-moment pair while preserving backward compatibility.

### Required touch point

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

### Required behavior

- add `mean_return_on_premium: f64`
- add `return_volatility_on_premium: f64`
- add `#[serde(default)]` to both new fields so older artifacts deserialize cleanly

### Scope guard

Do not remove or reinterpret any existing metric fields in this phase.

---

## 7.2 Phase 2 - CPU Exact Metric Path

Goal:

compute the new metrics exactly from realized outcomes on the CPU oracle path.

### Required touch point

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

### Required behavior

In `compute_metrics(...)`:

- accumulate `R1 = Σ return_on_premium`
- accumulate `R2 = Σ return_on_premium^2`
- derive:
  - `mean_return_on_premium = R1 / n`
  - `return_volatility_on_premium = sqrt(max(R2 / n - mean^2, 0))`
- for `valid_trades == 0`, return `0.0` for both new fields

### Scope guard

Do not change median semantics or ranking math in this phase.

---

## 7.3 Phase 3 - Aggregate Metrics Helper Widening

Goal:

make the local aggregate representation capable of reconstructing the new metrics exactly.

### Required touch point

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

### Required behavior

Widen `TupleAggregateMetrics` with:

- `sum_return_on_premium: f64`
- `sumsq_return_on_premium: f64`

Update `metrics_from_aggregate(...)` so it:

- derives the two new metrics exactly
- keeps `median_return_on_premium = 0.0` on the aggregate path
- preserves current empty-valid-trade behavior

---

## 7.4 Phase 4 - GPU Aggregate And Buffer Widening

Goal:

carry the new return-moment sums through the existing CUDA output path.

### Required touch points

- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [crates/app_core/src/kernels/options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)

### Required behavior

Widen `OptionsGpuTupleAggregate` with:

- `sum_return_on_premium: f32`
- `sumsq_return_on_premium: f32`

Widen `ReusableOutputBuffers` with matching device and host buffers.

Update:

- `copy_to_host()`
- `aggregates_prefix()`
- `per_tuple_output_device_bytes()`
- the kernel argument list and output writes

### Intended first-wave output-width target

- current: 16 bytes per tuple
- after widening: 24 bytes per tuple

### Scope guard

If the implementation pressure pushes the output width beyond 24 bytes per tuple in this wave, stop and reevaluate.

---

## 7.5 Phase 5 - GPU Ranking Integration

Goal:

thread the widened aggregate through the existing broad GPU lane reconstruction without changing ranking semantics.

### Required touch point

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

### Required behavior

Update:

- `gpu_scoring::aggregate_from_gpu(...)`
- the aggregate-to-metrics reconstruction for:
  - search
  - holdout
  - conservative stress
  - harsher stress

### Explicit non-change

- leave `robust_score(...)` unchanged

---

## 7.6 Phase 6 - Fixture, Serialization, And Command-Test Hygiene

Goal:

keep explicit struct literals, artifact fixtures, and command-side tests compiling and deserializing correctly.

### Required touch points

- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

### Required behavior

- update all explicit `OptionsExpressionMetrics { ... }` test literals
- add or adjust older-artifact deserialization coverage for the new fields
- keep artifact summary behavior unchanged unless a later product step explicitly chooses to surface the new metrics there

---

## 7.7 Phase 7 - Current-Host Contract Parity

Goal:

keep the web-side metric summary type aligned with the widened backend metric struct, without turning this into a UI redesign.

### Candidate touch points

- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [web/src/services/optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)

### Required additions

- add the two new fields to `OptionsExpressionMetricsSummary`
- normalize the two new fields in `normalizeMetrics(...)`

### Scope guard

Do not widen tables, sort controls, or artifact-summary displays by default in this phase.

This phase should thread the fields through the service layer now while keeping presentation unchanged.
That keeps reopened artifact payloads, future Prompt 10 / 11 work, and frontend-side debugging aligned with the widened backend contract without forcing any table or ranking UI expansion in this wave.

---

## 8. File Ownership Map

### Primary implementation files

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - metric struct widening and serde defaults

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - CPU formulas
  - aggregate reconstruction
  - GPU aggregate mapping

- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
  - aggregate struct widening
  - output buffer widening
  - output-width accounting

- [crates/app_core/src/kernels/options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)
  - kernel-side accumulation and output layout

### Primary verification / fixture files

- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

### Optional current-host parity files

- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [web/src/services/optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)

### Intentionally untouched files

These should stay unchanged for the adopted direction unless a very small compatibility edit becomes clearly necessary:

- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)

---

## 9. Acceptance, Rejection, And Stop Rules

## 9.1 Accept the pass if all of the following are true

- the aggregate state is widened by exactly the adopted two-accumulator pair
- `mean_return_on_premium` and `return_volatility_on_premium` are exact on both CPU and aggregate paths
- `median_return_on_premium` remains exactified-only
- `robust_score(...)` is unchanged
- older artifacts still deserialize cleanly
- the widened CUDA output remains within the intended first-wave state budget

## 9.2 Reject any part of the pass if it does any of the following

- adds sketch-based median or quantile approximation
- adds path-dependent metrics
- widens ranking math in the same pass
- widens output state beyond the intended first-wave budget without separate justification
- turns Prompt 9 into a generic distributional telemetry project

## 9.3 Stop rule

If work starts drifting into:

- ranking-functional changes
- median approximation
- downside-only accumulator families
- path replay or path statistics
- UI redesign

stop and reject the drift.
That is outside the valid Prompt 9 residue.

---

## 10. Sign-Off Target

The correct sign-off for this pass is:

- the broad GPU aggregate is widened by exactly two return-moment accumulators
- `mean_return_on_premium` and `return_volatility_on_premium` are implemented exactly
- `median_return_on_premium` remains exactified-only
- `robust_score(...)` remains unchanged
- workflow shape and breadth freezes remain unchanged

---

## 11. Implementation Checklist

## 11.1 Validated Current Baseline

- [x] `OptionsExpressionMetrics` currently exposes the shipped compact metric set.
- [x] `compute_metrics(...)` already computes exact CPU metrics and exact median from realized return vectors.
- [x] `OptionsGpuTupleAggregate` currently carries only `valid_trades`, `wins`, `total_pnl`, and `total_premium`.
- [x] `metrics_from_aggregate(...)` already reconstructs the reduced metric surface and zeros the median on aggregate-derived metrics.
- [x] The broad GPU ranking path already reconstructs search, holdout, conservative stress, and harsher stress lane metrics from aggregate outputs.
- [x] Prompt 7 and Prompt 8 already freeze stress-family and structure-family breadth.

## 11.2 Adopted Prompt 9 Decisions

- [x] Add `sum_return_on_premium = Σ r_i`.
- [x] Add `sumsq_return_on_premium = Σ r_i^2`.
- [x] Add `mean_return_on_premium`.
- [x] Add `return_volatility_on_premium`.
- [x] Keep `median_return_on_premium` exactified-only.
- [x] Keep `robust_score(...)` unchanged.
- [x] Reject sketches and path-dependent metrics in this first wave.

## 11.3 Backend Implementation

- [x] Add `mean_return_on_premium` and `return_volatility_on_premium` to `OptionsExpressionMetrics`.
- [x] Add `#[serde(default)]` to both new metric fields.
- [x] Update `compute_metrics(...)` to accumulate `R1` and `R2` and derive the two new metrics.
- [x] Widen `TupleAggregateMetrics` with `sum_return_on_premium` and `sumsq_return_on_premium`.
- [x] Update `metrics_from_aggregate(...)` to derive the two new metrics exactly.
- [x] Keep aggregate-derived `median_return_on_premium` at `0.0`.
- [x] Widen `OptionsGpuTupleAggregate` with the two new accumulators.
- [x] Widen `ReusableOutputBuffers`, host buffers, copyback, and `aggregates_prefix()`.
- [x] Update `per_tuple_output_device_bytes()` from 16 to 24.
- [x] Widen the CUDA kernel output layout and accumulation logic in `options_expression.cu`.
- [x] Update `aggregate_from_gpu(...)` so the widened GPU outputs map into the widened aggregate helper.
- [x] Confirm search, holdout, conservative stress, and harsher stress aggregate reconstruction all carry the new fields.

## 11.4 Compatibility And Fixture Hygiene

- [x] Update explicit `OptionsExpressionMetrics { ... }` literals in app-core tests.
- [x] Update explicit `OptionsExpressionMetrics { ... }` literals in Tauri command tests.
- [x] Add or update older-artifact deserialization coverage so missing new fields default cleanly.
- [x] Keep artifact-summary behavior unchanged in this wave.

## 11.5 Current-Host Contract Parity

- [x] Thread `mean_return_on_premium` and `return_volatility_on_premium` through `OptionsExpressionMetricsSummary` in `web/src/services/optionsExpressionService.ts`.
- [x] Add normalization coverage for the two new fields in `web/src/services/optionsExpressionService.test.ts`.
- [x] Do not widen panel tables, artifact summaries, or ranking UI by default in this pass.

## 11.6 Verification

- [x] Add CPU formula tests for obvious nonzero mean and variance cases.
- [x] Add empty-valid-trade and single-trade edge-case tests.
- [x] Add aggregate-equivalence tests proving `metrics_from_aggregate(...)` matches `compute_metrics(...)` for the new fields.
- [x] Add GPU aggregate buffer or prefix tests for the new accumulators.
- [x] Run `cargo test -q -p ta_app_core options_ --lib`.
- [x] Run `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`.
- [x] Run `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`.
- [x] If current-host contract parity is touched, run `npm run typecheck`.
- [x] Recheck that widened output bytes and planner telemetry stay within the intended first-wave budget.

---

## 12. Verification Plan

Run verification in this order.

### 12.1 CPU formula verification

Required:

- prove `mean_return_on_premium` by hand on small synthetic outcome vectors
- prove `return_volatility_on_premium` by hand on small synthetic outcome vectors
- cover zero-valid and single-valid-trade edge cases

### 12.2 Aggregate-equivalence verification

Required:

- build equivalent widened `TupleAggregateMetrics`
- verify `metrics_from_aggregate(...)` matches `compute_metrics(...)` for:
  - `mean_return_on_premium`
  - `return_volatility_on_premium`
- explicitly allow median mismatch because aggregate-derived median remains `0.0`

### 12.3 CUDA path verification

Required:

```powershell
cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel
```

And, when the CUDA build environment is available:

- verify widened kernel outputs copy back correctly
- verify `per_tuple_output_device_bytes()` rises from 16 to 24 exactly

### 12.4 Rust test verification

Required:

```powershell
cargo test -q -p ta_app_core options_ --lib
cargo test -q -p ta_desktop_demo_app commands_options_expression --lib
```

### 12.5 Optional current-host verification

```powershell
npm run typecheck
```

### 12.6 Cost guard verification

Before sign-off, confirm:

- no workflow-stage changes were introduced
- no ranking-functional changes were introduced
- aggregate output width is still within the intended first-wave budget
- no planner or canonical feasibility telemetry shows an unexplained regression large enough to invalidate the “near-constant-cost” claim

---

## 13. Final Sign-Off Template

Use this wording pattern when the pass is complete:

- single-pass sufficient-statistics widening is implemented and signed off
- the broad GPU aggregate now carries `Σ r_i` and `Σ r_i^2`
- `mean_return_on_premium` and `return_volatility_on_premium` are now exact on both CPU and aggregate paths
- `median_return_on_premium` remains exactified-only
- `robust_score(...)`, workflow shape, and breadth freezes remain unchanged
