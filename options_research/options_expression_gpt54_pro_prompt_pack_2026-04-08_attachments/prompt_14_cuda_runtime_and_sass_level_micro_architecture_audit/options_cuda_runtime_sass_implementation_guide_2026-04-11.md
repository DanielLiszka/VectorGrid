# Options CUDA Runtime / SASS - Implementation Guide

Date: 2026-04-11
Status: validated defer-by-default guide created; no code implementation authorized by current evidence

Validated against:

- [options_cuda_runtime_sass_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_14_cuda_runtime_and_sass_level_micro_architecture_audit/options_cuda_runtime_sass_direction_FINAL.md)
- [options_end_to_end_cost_implementation_guide_2026-04-11.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_13_end_to_end_cost_decomposition_and_replay_prepare_amortization/options_end_to_end_cost_implementation_guide_2026-04-11.md)
- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)
- [SPY prompt13 pass2 phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/prompt13_reruns/spy_pass2/2026-04-11T20-32-04/2026-04-11T20-32-04/phase05_summary.json)
- [QQQ prompt13 pass2 phase05_summary.json](/C:/Rust%20Projects/ta_desktop_demo/output/prompt13_reruns/qqq_pass2/2026-04-11T20-32-14/2026-04-11T20-32-14/phase05_summary.json)
- [run_options_expression_gpu_promotion_gate.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_gpu_promotion_gate.ps1)
- [profile_gpu_backtest_nsys.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/profile_gpu_backtest_nsys.ps1)
- [profile_gpu_backtest_ncu.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/profile_gpu_backtest_ncu.ps1)

Important correction applied here:

- Prompt 14 does **not** justify a new runtime/kernel implementation lane right now.
- Prompt 14 does **not** justify SASS/PTX work right now.
- Prompt 14 is valid only as:
  - an explicit defer decision
  - plus an optional bounded profiling / falsification path if someone later wants to challenge that decision
- This guide therefore tracks the valid Prompt 14 outcome:
  - preserve the current CUDA runtime and kernel surfaces unchanged
  - define when profiling is allowed
  - define what evidence would be required before any runtime or SASS work could reopen

Quick pointers:

- decision summary: [Section 1](#1-decision-summary)
- valid scope filter: [Section 3](#3-valid-scope-filter)
- code touch points: [Section 6](#6-code-touch-points-and-ownership)
- implementation phases: [Section 7](#7-implementation-phases)
- full checklist: [Section 8](#8-implementation-checklist)
- verification / reopen criteria: [Section 9](#9-verification-and-reopen-plan)

---

## 1. Decision Summary

The valid Prompt 14 outcome for this repo is:

**do not implement CUDA runtime restructuring or SASS/PTX work now.**

The current repo evidence still supports all of the following:

- Prompt 13 already landed the highest-ROI end-to-end optimization by shrinking cold replay/build cost.
- The current pass-2 SPY/QQQ evidence still does not make the runtime/kernel layer first-order enough to justify lower-level CUDA work.
- The current misses are more plausibly explained by replay/build economics, coverage quality, dataset overlap, eligibility, and the existing finalist boundary than by kernel micro-architecture.
- The current runtime shape is already compact:
  - `gpu_tuple_chunk_count = 1`
  - `gpu_kernel_launch_count = 4`
  - reusable output buffers are already present

That means the only valid implementation posture here is:

1. freeze runtime/kernel code changes for this prompt
2. preserve the current hybrid boundary and artifact workflow
3. optionally allow a bounded profiler-driven falsification audit later
4. reopen runtime or SASS work only if explicit evidence clears a stricter gate

This is an implementation guide only in the sense of **how to operationalize the defer decision safely**.

It is not a code-change implementation guide.

---

## 2. Current Evidence Snapshot

Use the Prompt 13 pass-2 reruns as the current decision anchor.

### SPY pass-2

- `replay_build_ms = 172`
- `total_preview_ms = 312`
- `gpu_ranking_us = 1691`
- `gpu_tensor_score_us = 1405`
- `gpu_exactify_us = 191`
- `gpu_sealed_stage_us = 95`
- promotion gate still fails
- coverage adequacy verdict remains `holdout_coverage_weak`
- dominant invalid reason remains `entry_past_cutoff`

### QQQ pass-2

- `replay_build_ms = 131`
- `total_preview_ms = 383`
- `gpu_ranking_us = 4061`
- `gpu_tensor_score_us = 1937`
- `gpu_exactify_us = 1838`
- `gpu_sealed_stage_us = 286`
- promotion gate passes
- coverage adequacy verdict remains `weak_dataset_overlap`
- dominant invalid reason remains `contract_not_eligible_as_of_event`

Immediate implication:

- the current kernel/runtime layer is still a low-single-digit-millisecond component inside a workflow whose more important remaining issues are elsewhere

That is not a sufficient runtime/SASS justification.

---

## 3. Valid Scope Filter

This guide intentionally keeps only the valid parts of the Prompt 14 response.

### 3.1 Accepted into this guide

- [x] make Prompt 14 a defer-by-default decision
- [x] preserve the current runtime and kernel code unchanged
- [x] allow only an optional bounded profiling path
- [x] require stronger reopen evidence before any runtime work
- [x] require even stronger reopen evidence before any SASS/PTX work
- [x] keep the current artifact-driven workflow and CPU fallback intact

### 3.2 Valid, but revised here

- [x] the current runtime shape is compact enough that immediate runtime work is not justified
- [x] SPY's remaining miss should not be assumed runtime-bound
- [x] QQQ's remaining limitation is more clearly overlap / eligibility shaped than runtime shaped
- [x] a profiling checklist is useful only as an optional falsification path, not as an active implementation lane

### 3.3 Rejected as current implementation work

- any runtime-structure rewrite now
- any chunk-plan rewrite now
- any host/device copy rewrite now
- any launch-shape rewrite now
- any SASS/PTX or hand-tuned kernel work now
- any new GPU-only finalist-boundary redesign under Prompt 14
- any gate retuning under Prompt 14
- any attempt to treat Prompt 14 as authorization for broad profiler infrastructure work

### 3.4 Practical rule

If a proposed Prompt 14 step:

- preserves the current runtime/kernel surfaces and only documents guardrails, keep it
- adds an optional bounded profiling checklist, keep it
- starts changing runtime or SASS code before strong evidence exists, reject it

---

## 4. Additive Posture And Non-Goals

### Additive posture

This guide is additive only in documentation and process control.

The intended result is:

- a clear record that Prompt 14 does not authorize code changes now
- a bounded profiling path if someone later wants to challenge that decision
- explicit reopen criteria and stop conditions

The intended result is not:

- changing the kernel
- changing the runtime
- changing finalist validation
- changing product gating policy
- broadening options scope

### Frozen for this pass

Do not change:

- [x] [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [x] [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [x] [options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)
- [x] [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [x] current `top_k` / `sealed_finalist_count`
- [x] current hybrid finalist boundary
- [x] current auto-backend policy
- [x] current artifact-driven workflow shape

### Explicit non-goals

Prompt 14 should not be used to do:

- replay/build optimization
- prepare-cache redesign
- density/search redesign
- overlap-quality fixes
- finalist-boundary redesign
- quote/realism expansion

Those belong elsewhere.

---

## 5. Target End State

When the valid Prompt 14 follow-up is complete, the repo should have all of the following:

1. **A locked defer decision**
   - runtime and kernel code remain unchanged
   - SASS/PTX work remains deferred

2. **A bounded profiling option**
   - if someone explicitly wants to challenge the defer decision, there is a narrow checklist to do so

3. **Clear reopen criteria**
   - runtime work cannot reopen without profiler-backed runtime first-order evidence
   - SASS cannot reopen without stronger kernel-specific evidence still

4. **No accidental scope creep**
   - Prompt 14 does not silently become a new implementation lane

---

## 6. Code Touch Points And Ownership

These surfaces are relevant for Prompt 14, but they are read-only by default.

### 6.1 End-to-end measurement anchor

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - `AnalyzeOptionsExpressionFeasibilityResponse`
  - `OptionsExpressionReplayBuildTimingBreakdown`
  - `analyze_options_expression_feasibility_blocking_impl(...)`

Ownership for Prompt 14:

- use as the current cost-share anchor before blaming the CUDA layer

### 6.2 Hybrid boundary context

- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
  - `prepare_options_expression_run(...)`
  - `build_gpu_flat_score_inputs(...)`
  - `rank_compiled_tuples(...)`
  - `exactify_ranked_results(...)`
  - `apply_sealed_finalist_stage(...)`
  - `finalize_ranked_results(...)`

Ownership for Prompt 14:

- contextual reading only
- do not modify under this prompt unless Prompt 14 is explicitly reopened by evidence

### 6.3 CUDA runtime surface

- [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
  - `OptionsGpuChunkPlan::for_inputs(...)`
  - `OptionsExpressionGpuRuntime::ensure_shared_inputs_chunk(...)`
  - `OptionsExpressionGpuRuntime::ensure_outcome_inputs(...)`
  - `OptionsExpressionGpuRuntime::ensure_output_buffers(...)`
  - `OptionsExpressionGpuRuntime::score_ranges(...)`
  - `ReusableOutputBuffers::copy_to_host(...)`
  - `inspect_options_expression_gpu_plan(...)`

Ownership for Prompt 14:

- read-only unless reopen criteria are met

### 6.4 Kernel surface

- [options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)
  - `options_expression_score_split_f64`

Ownership for Prompt 14:

- read-only unless reopen criteria are met

### 6.5 Existing profiling scripts

- [profile_gpu_backtest_nsys.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/profile_gpu_backtest_nsys.ps1)
- [profile_gpu_backtest_ncu.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/profile_gpu_backtest_ncu.ps1)

Important note:

- these scripts are currently oriented around `profile_gpu_dynamic_runner` and double-MA runtime profiling
- they are useful as profiler workflow references
- they are **not** an automatic options-expression profiling harness
- Prompt 14 does not authorize building a new options-specific profiler wrapper now

---

## 7. Implementation Phases

## 7.1 Phase 0 - Scope lock

- [x] Confirm Prompt 13 is completed baseline work.
- [x] Confirm current Prompt 14 output is a defer decision, not a code-change proposal.
- [x] Confirm runtime/SASS remains deferred by current evidence.

## 7.2 Phase 1 - Freeze code changes

This is the main valid implementation phase for Prompt 14.

- [x] make no runtime code edits
- [x] make no kernel code edits
- [x] make no SASS/PTX changes
- [x] make no boundary/gate changes under Prompt 14

Phase-1 success condition:

- [x] the repo remains unchanged in runtime behavior because Prompt 14 did not justify intervention

## 7.3 Phase 2 - Optional profiling prerequisites

Only enter this phase if someone explicitly wants to challenge the defer decision.

- [ ] confirm no other benchmark or profiler processes are running
- [ ] use an elevated PowerShell terminal before Nsight or CUDA profiling
- [ ] re-anchor the current SPY/QQQ options-expression feasibility evidence first
- [ ] refuse profiling if the current anchor still already makes runtime clearly non-first-order

## 7.4 Phase 3 - Optional bounded profiling

Only if Phase 2 is justified:

- [ ] capture a bounded Nsight Systems trace around a representative options-expression feasibility run
- [ ] measure wall-time share for:
  - replay/build
  - prepare
  - kernel execution
  - CUDA API / synchronization / copies
- [ ] stop if Systems still shows runtime/kernel share is too small
- [ ] only if Systems says runtime share is material, capture Nsight Compute on the options kernel
- [ ] use Nsight Compute only to answer:
  - occupancy / latency hiding
  - register / local-memory pressure
  - memory-access shape
  - stable stall reasons

## 7.5 Phase 4 - Reopen decision

Only if profiling evidence is strong:

- [ ] decide whether runtime-structure work should reopen
- [ ] separately decide whether SASS/PTX work should reopen
- [ ] do not conflate these two decisions

Hard rule:

- [ ] SASS/PTX cannot reopen just because runtime work might reopen

## 7.6 Phase 5 - Stop-or-defer

- [ ] stop if runtime is still not first-order
- [ ] stop if the likely win is too small in end-to-end terms
- [ ] stop if the evidence is coverage/overlap/finalist shaped instead of runtime shaped

---

## 8. Implementation Checklist

### 8.1 Baseline and scope

- [x] Validate the Prompt 14 response against the current repo rather than accepting it wholesale.
- [x] Accept defer-by-default as the core valid Prompt 14 direction.
- [x] Reject immediate runtime-structure implementation.
- [x] Reject immediate SASS/PTX implementation.
- [x] Treat Prompt 13 as completed baseline work.

### 8.2 Freeze decision

- [x] Keep `options_expression_gpu.rs` unchanged under Prompt 14.
- [x] Keep `options_expression.cu` unchanged under Prompt 14.
- [x] Keep `commands_options_expression.rs` unchanged under Prompt 14.
- [x] Keep current product gating policy unchanged under Prompt 14.

### 8.3 Optional profiling prerequisites

- [ ] Confirm no active benchmark/profiler overlap before profiling.
- [ ] Use an elevated PowerShell terminal for Nsight/CUDA profiling.
- [ ] Re-anchor current SPY/QQQ pass-2 feasibility evidence first.
- [ ] Confirm profiling is still worth doing after re-anchor.

### 8.4 Optional bounded profiling

- [ ] Run a bounded Nsight Systems capture only if explicitly requested.
- [ ] Stop if Systems does not show a material runtime/kernel wall-time share.
- [ ] Run Nsight Compute only if Systems shows the runtime layer is first-order enough to justify it.
- [ ] Record one stable actionable bottleneck before considering any implementation.

### 8.5 Runtime reopen gate

- [ ] Reopen runtime-structure work only if Systems shows runtime/kernel/copy/sync behavior is materially visible in wall time.
- [ ] Reopen runtime-structure work only if replay/build and other larger buckets are no longer the dominant explanation.
- [ ] Reopen runtime-structure work only if the expected end-to-end win is meaningful.

### 8.6 SASS reopen gate

- [ ] Reopen SASS/PTX work only if runtime-structure work is first-order or exhausted.
- [ ] Reopen SASS/PTX work only if Nsight Compute shows a stable kernel bottleneck on `options_expression_score_split_f64`.
- [ ] Reopen SASS/PTX work only if the likely upside is meaningful in end-to-end preview/run terms.

### 8.7 Stop conditions

- [ ] Stop if replay/build still dominates the cold path.
- [ ] Stop if GPU ranking remains a low-single-digit-millisecond stage on representative workloads.
- [ ] Stop if `gpu_tuple_chunk_count` stays `1` and launch count stays tiny.
- [ ] Stop if output-buffer reuse already looks effective.
- [ ] Stop if the evidence remains coverage / overlap / eligibility / finalist shaped.

---

## 9. Verification And Reopen Plan

### 9.1 Current verification anchors

The current defer decision should continue to rely on:

- [x] Prompt 13 SPY pass-2 summary
- [x] Prompt 13 QQQ pass-2 summary
- [x] current runtime shape in `options_expression_gpu.rs`
- [x] current kernel shape in `options_expression.cu`

### 9.2 Current accepted interpretation

- [x] SPY remaining miss is not proven runtime-bound
- [x] QQQ remaining limitation is more clearly overlap / eligibility bound
- [x] the runtime layer is not currently first-order enough to justify code changes

### 9.3 Allowed re-anchor commands

Use these before any future profiling discussion:

- [ ] `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`
- [ ] `cargo check -q -p ta_desktop_demo_app --bin ta_options_expression_feasibility`
- [ ] rerun the canonical SPY feasibility command through [run_options_expression_gpu_promotion_gate.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_gpu_promotion_gate.ps1)
- [ ] rerun the canonical QQQ feasibility command through [run_options_expression_gpu_promotion_gate.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_gpu_promotion_gate.ps1)

### 9.4 Profiling command policy

- [x] do not run `profile_gpu_backtest_nsys.ps1` blindly for Prompt 14
- [x] do not run `profile_gpu_backtest_ncu.ps1` blindly for Prompt 14
- [x] if profiling is explicitly requested later, either:
  - run a bounded manual Nsight capture around the options-expression feasibility command
  - or create a separate profiling-only doc / harness proposal first

### 9.5 Acceptance criteria for keeping Prompt 14 closed

Prompt 14 should remain closed if all of the following stay true:

- [x] no profiler evidence shows the runtime/kernel layer as first-order
- [x] replay/build and non-runtime issues remain the larger explanation
- [x] no stable actionable kernel bottleneck is demonstrated
- [x] likely end-to-end upside remains too small

---

## 10. Final Acceptance Criteria

This Prompt 14 guide is successful only if all of these are true:

- [x] it keeps the repo from starting low-ROI runtime/SASS work prematurely
- [x] it clearly distinguishes defer, profiling-only, runtime-reopen, and SASS-reopen
- [x] it preserves current runtime/kernel code unless strong evidence appears
- [x] it keeps Prompt 14 subordinate to end-to-end evidence rather than CUDA folklore

If those conditions are not met, the correct outcome is:

- keep Prompt 14 as a decision memo only
- do not create a runtime/SASS implementation lane

That is still a valid Prompt 14 result.

---
