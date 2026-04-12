# Options CUDA Runtime / SASS Direction — FINAL

Date: 2026-04-12  
Status: final consolidation for Prompt 14; implementation-ready repo direction

## 1. Executive summary

The current options-expression lane should **not** move into CUDA-runtime restructuring or SASS/PTX work now.

The current repo evidence still says:

- the options lane is already a real **GPU-ranked hybrid** workflow, not a missing-GPU workflow;
- Prompt 13 already landed the highest-ROI end-to-end optimization by decomposing and shrinking cold replay/build cost;
- the current pass-2 SPY/QQQ evidence still does **not** make the runtime/kernel layer first-order enough to justify lower-level CUDA work;
- the remaining issues look more like **cold-path economics, provider-slice overlap/eligibility, and finalist/coverage boundaries** than like kernel micro-architecture.

**Adopted direction:**

- **No runtime-structure implementation now**
- **No SASS/PTX implementation now**
- **Default defer for Prompt 14**
- Keep only an **optional, bounded profiling checklist** as a falsification path if someone explicitly wants to prove the current defer decision wrong

That means Prompt 14 closes as a **defer-by-default decision**, not as a code-change prompt.

---

## 2. Current CUDA/runtime evidence

### 2.1 What the repo already completed

The current options lane already has the important product pieces in place:

- artifact-driven options-expression workflow is shipped
- broad tuple ranking already runs on the GPU compiled-tensor path
- Prompt 12 already narrowed the GPU/CPU finalist boundary
- Prompt 13 already decomposed end-to-end preview cost and reduced the dominant cold replay/build bucket
- replay/build timing now exists at the Tauri feasibility boundary
- the GPU runtime already includes chunk planning, cached shared/outcome uploads, and reusable output buffers
- CPU fallback/oracle behavior remains intact

So Prompt 14 is **not** arriving at a repo that still needs basic GPU enablement. It is arriving after the high-ROI first wave has already landed.

### 2.2 Authoritative current cost snapshot

Use the Prompt 13 **pass-2** reruns as the current decision surface.

| Root | replay_build_ms | total_preview_ms | gpu_ranking_us | gpu_tensor_score_us | gpu_exactify_us | gpu_sealed_stage_us | tuple_chunk_count | kernel_launch_count | output_buffer_reuses | output_buffer_allocations |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| SPY pass-2 | 172 | 312 | 1,691 | 1,405 | 191 | 95 | 1 | 4 | 3 | 1 |
| QQQ pass-2 | 131 | 383 | 4,061 | 1,937 | 1,838 | 286 | 1 | 4 | 3 | 1 |

Immediate implications:

- replay/build is still large in cold preview terms;
- the full GPU ranking stage is still only **low single-digit milliseconds**;
- the runtime shape is already small and disciplined;
- the current options lane is **not** obviously launch-bound, chunk-bound, or copy-bound from these canonical runs.

### 2.3 Current quality / promotion context

The current non-runtime story is still stronger than the runtime story.

#### SPY pass-2

- coverage adequacy verdict: `holdout_coverage_weak`
- non-promotable: `false`
- dominant invalid reason: `entry_past_cutoff`
- dominant invalid reason share: about `76.6%`
- holdout candidate coverage rate: `0.0%`

This does **not** look like a runtime-shaped miss.

#### QQQ pass-2

- coverage adequacy verdict: `weak_dataset_overlap`
- non-promotable: `true`
- dominant invalid reason: `contract_not_eligible_as_of_event`
- dominant invalid reason share: about `89.4%`
- search overlap rate: about `23.0%`

This is even more clearly **provider-slice / overlap / eligibility-bound**, not runtime-bound.

### 2.4 Current runtime shape in code

The current runtime already does the right broad things for this repo stage:

- computes a chunk plan from available memory
- caches shared tuple-offset uploads
- caches outcome tensors
- reuses output buffers
- launches a single simple scoring kernel over tuple chunks and event ranges
- synchronizes and copies back only compact aggregate arrays

Key touchpoints:

- `crates/app_core/src/options_expression_gpu.rs`
  - `OptionsGpuChunkPlan::for_inputs`
  - `OptionsExpressionGpuRuntime::score_ranges`
  - `ensure_shared_inputs_chunk`
  - `ensure_outcome_inputs`
  - `ensure_output_buffers`
  - `ReusableOutputBuffers::copy_to_host`
  - `inspect_options_expression_gpu_plan`
- `crates/app_core/src/kernels/options_expression.cu`
  - `options_expression_score_split_f64`

The kernel itself is structurally simple:

- one thread maps to one tuple
- each thread walks the selected event range
- it accumulates six scalar outputs per tuple

That simplicity is one more reason not to jump into SASS heroics before proving the layer actually owns enough wall time.

### 2.5 Current boundary outside the kernel body

The broad tuple set is already GPU-ranked, but the repo still retains a CPU finalist boundary:

- `exactify_ranked_results(...)`
- `apply_sealed_finalist_stage(...)`
- exact extreme-stress replay

Those are still much more plausible remaining performance questions than SASS-level instruction tuning.

### 2.6 Repo policy context

The current prompt pack and Prompt 13 implementation guide already lean the same way:

- Prompt 14 is later than Prompt 13 and should be used only as a **falsification audit**
- runtime/SASS work remains deferred unless current evidence clearly rebuts the Prompt 13 conclusion
- README-level repo policy already says to use perf gates and Nsight before drawing conclusions from one-off CUDA timings

---

## 3. Final recommendation

### 3.1 Adopted direction

**Adopt an explicit defer-by-default posture for Prompt 14.**

Concretely:

1. **Do not implement CUDA-runtime restructuring now.**
2. **Do not implement SASS/PTX work now.**
3. **Do not treat SPY’s remaining miss as runtime-bound.**
4. Keep only a **bounded profiling checklist** available if someone explicitly wants to challenge the defer decision.

### 3.2 Repo decision in one sentence

**Prompt 14 should not lead to code changes now; at most it should exist as an optional measure-first audit, and the repo should otherwise move on.**

### 3.3 Why this is the winning direction

It beats the alternatives because the current authoritative evidence still says:

- replay/build remains materially larger than the scorer;
- the runtime shape is already compact;
- current misses are better explained by coverage, overlap, eligibility, and finalist-boundary issues;
- no profiler-backed options-kernel evidence currently shows a bottleneck large enough to justify runtime or SASS work.

### 3.4 SPY’s remaining miss classification

From the current evidence set, SPY’s remaining promotion-gate miss is **not runtime-bound**.

Best current classification:

- **primarily coverage/quality-bound**
- with a smaller finalist-boundary tail
- **not diagnosable as a SASS/runtime problem from the attached evidence**

### 3.5 QQQ classification

QQQ’s current limitation is even less ambiguous:

- **provider-slice / overlap / eligibility-bound**
- explicitly not a kernel-runtime story

---

## 4. What remains unchanged

Keep all of the following unchanged for now:

- current artifact-driven options workflow
- current Prompt 13 replay/build-first end-to-end posture
- current hybrid finalist boundary
- current CPU fallback/oracle behavior
- current stress-lane semantics
- current auto-backend behavior
- current options kernel and runtime structure
- current chunk planner, upload caches, and output-buffer reuse logic
- current `top_k` and `sealed_finalist_count` containment boundary

This final Prompt 14 decision is intentionally conservative: it preserves what is already working.

---

## 5. Deferred ideas

These are not adopted now, but remain reopenable if later evidence changes.

### 5.1 Optional profiling-only falsification audit

Allowed only if someone explicitly wants to test whether the current defer decision is wrong.

Scope:

- no runtime code changes
- no kernel code changes
- no SASS/PTX hand work
- only bounded capture and measurement

### 5.2 Runtime-structure tuning, but only if reopened later

This includes ideas like:

- different chunking policy
- different upload/copy behavior
- different output-copy contract
- launch-shape changes

These are **deferred**, not rejected forever.

### 5.3 True SASS/PTX investigation, but only if reopened later

This is the furthest-deferred lane.

It should not reopen until runtime-structure work is either:

- clearly unnecessary, or
- tried and proven insufficient,

**and** profiler evidence still shows the options kernel/runtime layer is first-order.

---

## 6. Rejected ideas

Reject these for the current options lane now:

- direct SASS/PTX hand optimization based on curiosity alone
- speculative launch/chunk/copy rewrites without profiler evidence
- attributing SPY’s remaining miss to the CUDA runtime by default
- generic CUDA folklore as a substitute for repo-specific measurement
- a broad “GPU everything” rewrite of finalist validation as a Prompt 14 outcome
- treating microbenchmark-style kernel numbers as more important than end-to-end preview timings

These are low-ROI for the current repo state.

---

## 7. Concrete code touch points by file and symbol

## 7.1 Current measurement and end-to-end context surfaces

### `src-tauri/src/commands_options_expression.rs`

Use these as the authoritative timing and replay/build context surface:

- `AnalyzeOptionsExpressionFeasibilityResponse`
- `OptionsExpressionReplayBuildTimingBreakdown`
- `build_options_expression_replay_inputs_with_timing`
- `analyze_options_expression_feasibility_blocking_impl`

These are the first places to re-check before blaming the CUDA layer.

## 7.2 Current broad ranking vs finalist boundary

### `crates/app_core/src/options_expression.rs`

Main touchpoints:

- `prepare_options_expression_run`
- `build_gpu_flat_score_inputs`
- `gpu_scoring::rank_compiled_tuples_gpu`
- `rank_compiled_tuples`
- `exactify_ranked_results`
- `apply_sealed_finalist_stage`
- `finalize_ranked_results`

These define the current hybrid boundary and should be read before any lower-level CUDA hypothesis.

## 7.3 Current CUDA runtime surface

### `crates/app_core/src/options_expression_gpu.rs`

Main touchpoints:

- `OptionsGpuChunkPlan::for_inputs`
- `OptionsExpressionGpuRuntime::new`
- `OptionsExpressionGpuRuntime::kernel_caps`
- `OptionsExpressionGpuRuntime::ensure_shared_inputs_chunk`
- `OptionsExpressionGpuRuntime::ensure_outcome_inputs`
- `OptionsExpressionGpuRuntime::ensure_output_buffers`
- `OptionsExpressionGpuRuntime::score_ranges`
- `ReusableOutputBuffers::copy_to_host`
- `score_options_expression_tensors_gpu_multi_range`
- `inspect_options_expression_gpu_plan`

These are **measurement-only touchpoints for now**, not implementation targets.

## 7.4 Current kernel surface

### `crates/app_core/src/kernels/options_expression.cu`

- `options_expression_score_split_f64`

This is the only kernel that matters for Prompt 14, and it remains read-mostly unless the defer decision is later overturned.

## 7.5 Verification helper

### `scripts/run_gpu_perf_gate.ps1`

This is relevant only if a runtime or kernel change is ever retained later.

For the current adopted direction, no direct modification is needed.

---

## 8. Profiling / measurement order

This section is the only active follow-up path left open by this document.

## 8.1 Default posture

**Do nothing by default.**

Prompt 14 does not automatically trigger a profiling run.

## 8.2 If someone explicitly wants a falsification audit

Run measurement in this order.

### Step 1 — re-anchor current repo timings

Confirm the current options lane still shows the same broad shape:

- `replay_build_ms`
- `total_preview_ms`
- `gpu_ranking_us`
- `gpu_tensor_score_us`
- `gpu_exactify_us`
- `gpu_sealed_stage_us`
- `gpu_tuple_chunk_count`
- `gpu_kernel_launch_count`
- `gpu_output_buffer_reuses`
- `gpu_output_buffer_allocations`

If this already shows the runtime layer is still tiny, stop.

### Step 2 — Nsight Systems, bounded capture only

Capture only a representative options-ranking region.

Use bounded collection so the trace answers:

- how much wall time is still replay/build
- how much is prepare
- how much is kernel execution
- how much is CUDA API / queue / synchronization / copy behavior
- whether the runtime layer is truly first-order

Do **not** use an unbounded trace that is dominated by unrelated initialization.

### Step 3 — Only if Systems shows the runtime layer is material

Run **Nsight Compute** on `options_expression_score_split_f64`.

Collect enough to answer:

- achieved occupancy / latency-hiding picture
- register / local-memory pressure
- memory-access shape and likely coalescing quality
- warp stall shape
- whether there is one stable, actionable bottleneck

### Step 4 — Decide in this order

1. no action
2. runtime-structure tuning
3. only then, and only rarely, SASS/PTX investigation

Never invert that order.

---

## 9. Verification order

## 9.1 For the adopted direction now

No code change is adopted, so verification is simple:

1. keep current Prompt 13 pass-2 evidence as the decision anchor
2. keep current repo perf-gate posture unchanged
3. keep current CPU/GPU parity expectations unchanged

## 9.2 If a profiling-only audit is run

Verification order:

1. preserve existing feasibility JSON output and timing semantics
2. collect bounded Nsight Systems evidence
3. only if warranted, collect Nsight Compute evidence
4. write down whether the evidence changes the repo decision
5. stop if it does not

## 9.3 If runtime work is ever reopened later

Use this order before keeping any change:

1. CPU parity first
   - ranked tuple identities
   - selection metrics
   - holdout / sealed / stress metrics
   - verdicts
2. Options feasibility reruns on the current SPY/QQQ anchors
3. GPU perf gate
4. Repo quality gate as appropriate
5. end-to-end preview timing comparison, not just kernel timings

## 9.4 Invariants that must not drift

Any future runtime/kernel change must preserve:

- candidate density
- invalid-slot rate and dominant invalid reasons unless intentionally changed elsewhere
- overlap and adequacy verdicts
- retained `top_k` ordering unless intentionally changed by a separate ranking decision
- sealed-finalist membership
- validation verdicts
- CPU-only behavior

If a runtime tweak changes those, it is no longer a Prompt 14-only change.

---

## 10. Stop conditions

Reject or continue deferring SASS-level work while any of the following remain true:

1. **Replay/build still dominates the cold path**
2. **GPU ranking remains only a low-single-digit-millisecond stage**
3. **`gpu_tuple_chunk_count` stays at `1` on representative workloads**
4. **kernel launch count remains tiny**
5. **output buffers are already being effectively reused**
6. **no Nsight Systems capture shows the runtime layer as first-order wall time**
7. **no Nsight Compute capture shows one stable, actionable kernel bottleneck**
8. **the likely end-to-end upside is too small to justify the engineering risk**
9. **the remaining issue is better explained by coverage, overlap, eligibility, or finalist-boundary economics**

### 10.1 Explicit reopen conditions for runtime-structure work

Reopen runtime-structure tuning only if **all** of the following become true:

- Nsight Systems shows runtime/kernel/copy/sync behavior is materially visible in wall time
- representative workloads stop looking like the current one-chunk, low-launch shape
- replay/build and other larger non-runtime buckets are no longer the main explanation
- the expected end-to-end win is meaningful

### 10.2 Explicit reopen conditions for SASS/PTX work

Reopen true SASS/PTX work only if **all** of the following become true:

- runtime-structure work is now first-order or has already been exhausted
- Nsight Systems still shows a material runtime/kernel share
- Nsight Compute shows a stable kernel bottleneck on `options_expression_score_split_f64`
- the likely upside is large enough to matter in end-to-end previews/runs
- the repo can explain the change in product terms, not just microbenchmark terms

If those conditions are not met, Prompt 14 stays closed.

---

## Final repo decision

**Final adopted decision:** keep Prompt 14 deferred.

For the current options-expression lane:

- **implementation now:** no
- **profiling doc only:** optional, bounded, and only if explicitly requested
- **default action:** no further runtime/SASS work

The next code-changing performance dollar still belongs **outside** SASS.

---

## Source basis

This consolidation is grounded in the current Prompt 14 attachment set, especially:

- `options_expression_gpt54_pro_prompt_pack_2026-04-08.md`
- `options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_expression_future_scope_roadmap_2026-04-07.md`
- `options_end_to_end_cost_implementation_guide_2026-04-11.md`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `src-tauri/src/commands_options_expression.rs`
- Prompt 13 SPY/QQQ pass-2 phase05 summaries
- `README.md`
- `REPO_MAP.md`
- `AGENTS.md`

### Optional external profiling references

If a bounded falsification audit is later run, these are the current tool references that best match the intended measurement order:

- NVIDIA Nsight Systems User Guide: https://docs.nvidia.com/nsight-systems/UserGuide/index.html
- NVIDIA Nsight Systems Analysis Guide: https://docs.nvidia.com/nsight-systems/AnalysisGuide/index.html
- NVIDIA Nsight Compute Profiling Guide: https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html
- NVIDIA CUDA C++ Best Practices Guide: https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/
