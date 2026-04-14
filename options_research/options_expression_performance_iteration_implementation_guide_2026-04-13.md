# Options Expression Performance Iteration - Active Implementation Guide

Date: 2026-04-13  
Status: active planning and working tracker for independent iteration on current `options_expression` performance

Use this document when the goal is:

- independently improve the shipped options backtest optimization performance
- focus primarily on improving existing CUDA performance or expanding the amount of useful work performed in CUDA
- remain willing to improve CPU-side performance when the current CPU/CUDA gap is too narrow to ignore CPU hot paths
- do so one measured iteration at a time
- keep a live checklist and findings log while the work is in progress

This guide is intentionally different from the broader prompt-pack docs in `options_research/`.
Those documents narrow the strategic choices.
This document is the active execution tracker for actually doing the work.

Use this together with:

- [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)
- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [options_expression_gpt54_pro_prompt_pack_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08.md)
- [options_end_to_end_cost_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_13_end_to_end_cost_decomposition_and_replay_prepare_amortization/options_end_to_end_cost_direction_FINAL.md)
- [options_end_to_end_cost_implementation_guide_2026-04-11.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_13_end_to_end_cost_decomposition_and_replay_prepare_amortization/options_end_to_end_cost_implementation_guide_2026-04-11.md)
- [options_gpu_finalist_boundary_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_12_gpu_resident_finalist_exactification_and_embedded_validation_boundary/options_gpu_finalist_boundary_direction_FINAL.md)
- [options_single_pass_metrics_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_09_single_pass_sufficient_statistics_expansion/options_single_pass_metrics_direction_FINAL.md)
- [options_ranking_functional_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_10_robust_ranking_functional_and_scalarization_math/options_ranking_functional_direction_FINAL.md)
- [options_validation_math_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_11_validation_frontier_and_confidence_math_without_extra_stages/options_validation_math_direction_FINAL.md)
- [options_cuda_runtime_sass_direction_FINAL.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_14_cuda_runtime_and_sass_level_micro_architecture_audit/options_cuda_runtime_sass_direction_FINAL.md)
- [options_expression_cuda_profiling_playbook_2026-04-13.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_cuda_profiling_playbook_2026-04-13.md)

Quick pointers:

- objective and scope guard: [Section 1](#1-objective)
- starting point and prior findings: [Section 2](#2-repo-grounded-starting-point)
- priority order: [Section 3](#3-priority-order-for-iteration)
- working loop: [Section 4](#4-iteration-loop)
- measurement package: [Section 5](#5-measurement-package)
- profiling stack: [Section 5.4](#54-profiling-stack)
- tool selection matrix: [Section 5.5](#55-tool-selection-matrix)
- execution checklist: [Section 6](#6-execution-checklist)
- findings log: [Section 7](#7-performance-findings-log)
- external reference log: [Section 8](#8-external-reference-log)
- immediate next action: [Section 9](#9-immediate-next-action)

---

## 1. Objective

The objective of this guide is:

**improve current options-expression end-to-end performance with evidence-backed, parity-safe iterations, while keeping a live record of what was measured, changed, accepted, deferred, or rejected.**

That objective has five hard constraints:

1. **Local evidence is authoritative.**
   Use repo-local timings, artifacts, proof harnesses, and tests as the primary decision surface.

2. **One optimization family at a time.**
   Do not mix replay/build reduction, prepare dedup, finalist-boundary work, ranking-math changes, and low-level CUDA tuning into one pass.

3. **Correctness and product semantics are fixed unless the iteration explicitly targets them.**
   Preserve CPU fallback/oracle behavior, artifact semantics, current workflow shape, and validation meaning.

4. **Web search is secondary, not primary.**
   Use external references only after local evidence narrows the question enough that outside math, CUDA, or profiling guidance is worth consulting.

5. **CUDA-first, not CUDA-dogmatic.**
   Prefer improvements that strengthen the existing CUDA path or move more useful work into CUDA, but allow CPU-side hot-path work when it lowers end-to-end wall time or makes the CUDA route materially more worthwhile.

### Explicit non-goals for this guide

Do not use this tracker to justify:

- new workflow stages
- new structure families
- new parent strategy families
- generic options-platform expansion
- abandoning the current CUDA-first product direction
- speculative SASS/PTX work without local profiler evidence
- broad UI expansion detached from measured performance or decision quality

---

## 2. Repo-Grounded Starting Point

These are the current repo-grounded priors that should be treated as starting assumptions, not as permanently true facts.

### 2.1 Confirmed prior decisions

- [x] Prior options-performance work already narrowed the likely next end-to-end target to cold replay/build cost before lower-level CUDA work.
- [x] Prior options-performance work explicitly deferred runtime-structure and SASS/PTX optimization unless later evidence reopens it.
- [x] Prior options-performance work treated finalist-boundary reduction as a narrow, measurement-gated lane, not a "GPU everything" lane.
- [x] Prior options evidence also implies a practical warning: the current CPU/CUDA performance gap is narrow enough that CPU-side improvement can still be high-ROI if it reduces non-CUDA overhead or improves total preview/run economics.
- [x] Prior options-math work identified three still-legitimate math-heavy surfaces:
  - sufficient-statistics widening
  - ranking functional revision
  - validation-frontier / adequacy math

### 2.2 Current practical hypothesis order

Until current `HEAD` evidence proves otherwise, the likely value order is:

1. cold replay/build reduction
2. conditional prepare dedup
3. narrow finalist-boundary reduction
4. bounded CPU hot-path improvement that either lowers total wall time or widens the practical payoff of the CUDA path
5. math-side improvement that increases decision quality at near-constant hot-path cost
6. runtime/SASS work only if current evidence decisively shifts there

### 2.3 Current "do not assume" list

Do not assume the following without rerunning current evidence:

- that April 11-12 findings still hold exactly on current `HEAD`
- that current canonical SPY/QQQ workloads still exhibit the same dominant bucket
- that any old gate result still reflects the current code path
- that a CUDA-only intervention is automatically better than a CPU-side intervention on the same workflow
- that a math-side improvement is worth shipping before a current performance bottleneck is remeasured

---

## 3. Priority Order For Iteration

This is the default order unless current measurements force a change.

## 3.1 Lane A - Cold replay/build reduction

Primary files:

- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [src-tauri/src/bin/ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)

What qualifies:

- replay/build timing decomposition
- cold cache reuse improvement
- safe persisted cache reuse
- reducing redundant replay/input-materialization work

What does not qualify:

- weakening provenance or identity checks
- changing tuple ranking
- changing current validation/finalist semantics

## 3.2 Lane B - Conditional prepare dedup

Primary files:

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

Enter this lane only if:

- Lane A has already been rerun on current `HEAD`
- one Lane A pass is already accepted or rejected
- prepare is now the dominant removable bucket

## 3.3 Lane C - Finalist-boundary reduction

Primary files:

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)

Enter this lane only if:

- broad ranking remains healthy
- exactify/finalist work is now materially first-order
- the retained-summary-compatible slice is measurable and large enough to matter

## 3.4 Lane D - Sufficient-statistics widening

Primary files:

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [crates/app_core/src/kernels/options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)

Enter this lane only if:

- the performance bottleneck is reasonably under control, or
- the goal of the iteration is explicitly decision-quality math rather than raw runtime

## 3.5 Lane E - CPU hot-path improvement

Primary files vary by the measured bottleneck, but likely include:

- [src-tauri/src/commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)

Enter this lane only if:

- the measured CPU/CUDA gap remains narrow on the current workload,
- the dominant removable cost is still on the CPU side,
- or a CPU-side reduction is the clearest way to improve end-to-end economics for the CUDA route

Good targets:

- replay/build CPU work
- prepare-time CPU work
- finalist exactify CPU work
- CPU-side metric or summary work that currently blunts the value of GPU ranking

Bad targets:

- CPU-only redesigns that weaken the CUDA-first product posture
- CPU work chosen purely because it is easier to implement than the measured higher-value target

## 3.6 Lane F - Ranking functional revision

Primary files:

- [crates/app_core/src/options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)

Enter this lane only if:

- current ranking asymmetry or score quality is the target,
- the change can remain scalar and GPU-friendly,
- and the sufficient-statistics state is already adequate for the intended revision

## 3.7 Lane G - Validation-frontier math

Primary files:

- [crates/app_core/src/options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [web/src/components/OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- [web/src/stores/optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [web/src/services/optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)

Enter this lane only if:

- the goal is better interpretation/selection trust, not lower runtime,
- and the work stays zero-extra-stage and scope-labeled

## 3.8 Deferred lane - Runtime/SASS work

Primary files:

- [crates/app_core/src/options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [crates/app_core/src/kernels/options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)

Stay deferred unless:

- current replay/build and safe prepare work stop dominating,
- runtime/kernel/copy behavior becomes first-order in current evidence,
- and bounded profiler evidence says the layer is worth reopening

---

## 4. Iteration Loop

Every active pass should follow this loop.

1. **Freeze the question.**
   State exactly which lane is being investigated and what would count as success.

2. **Capture a before baseline.**
   Use the current canonical options proof surfaces before changing code.

3. **Pick one narrow intervention.**
   Implement one reduction, one math change, or one boundary change. CUDA-first is preferred, but CPU-side work is valid when it is the measured bottleneck or the best way to improve end-to-end economics.

4. **Run parity and timing verification.**
   Keep the timing boundary honest and compare the same workload shape before/after.

5. **Record the result in the log.**
   Include what was measured, what changed, what improved, what regressed, and whether the pass was accepted.

6. **Decide the next lane.**
   Continue only if the evidence points to a clear next highest-value target.

### Practical rules

- Run one benchmark/proof/profile workflow at a time.
- Do not overlap perf runs.
- Do not trust one-off timing anecdotes.
- Do not move into the next lane until the current lane has an explicit keep/defer/reject decision.

---

## 5. Measurement Package

This is the default evidence package to reuse across iterations.

## 5.1 Canonical commands

Primary options proof/gate surface:

```powershell
powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1
```

Primary options phase05 wrapper:

```powershell
powershell -File scripts/run_options_expression_phase05.ps1
```

Options-specific Nsight Systems wrapper:

```powershell
powershell -File scripts/profile_options_expression_nsys.ps1
```

Options-specific Nsight Compute wrapper:

```powershell
powershell -File scripts/profile_options_expression_ncu.ps1
```

Options-specific Compute Sanitizer wrapper:

```powershell
powershell -File scripts/profile_options_expression_compute_sanitizer.ps1
```

Wrapper workload override:

- every current options wrapper accepts `-RequestArgs`
- those flags are forwarded directly to `ta_options_expression_feasibility`
- use this when the default Phase05 grid is too small for representative CUDA profiling
- when `-RequestArgs` contains literal `--foo` style flags, prefer direct PowerShell invocation
  with `&` and splatted parameters instead of `powershell -File ... -RequestArgs '--foo' ...`
- when a profiler run is pinned to a specific options dataset, prefer passing an explicit
  `-ArtifactPath` if the newest saved run artifact on disk may target a different root symbol
- current profiler wrappers now stage Cargo-style runtime DLL search paths before launching the
  release feasibility binary directly; keep that behavior because `cargo run` adds those paths
  automatically but Nsight / Compute Sanitizer direct launches do not

Bounded GPU scorer bench:

```powershell
cargo bench -p ta_app_core --bench options_expression_gpu_score --features cuda,cuda-backtest-kernel
```

Core backend confidence:

```powershell
cargo test -p ta_app_core options_ --lib
cargo test -p ta_desktop_demo_app commands_options_expression --lib
```

CUDA-enabled compile confidence when needed:

```powershell
cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel
```

## 5.2 Metrics to capture each time

For runtime/perf passes, capture at minimum:

- `replay_build_ms`
- replay/build sub-buckets when available
- `total_preview_ms`
- warm preview timing
- prepare cost buckets
- `gpu_ranking_us`
- `gpu_tensor_score_us`
- `gpu_exactify_us`
- `gpu_sealed_stage_us`
- `gpu_tuple_chunk_count`
- `gpu_kernel_launch_count`
- `gpu_output_buffer_reuses`
- CPU vs CUDA preview/run speedup when both paths are available on the same workload
- candidate density
- invalid-slot rate
- coverage / adequacy verdicts
- current backend used / recommendation

For decision-quality math passes, capture at minimum:

- retained `top_k` composition deltas
- validation-winner changes
- sealed-finalist membership changes
- validation-verdict changes
- artifact-summary or frontier-surface interpretation changes

## 5.3 Timing-boundary discipline

Every finding must say which timing boundary it belongs to:

- end-to-end preview/run wall time
- feasibility/replay-build time
- prepare time
- broad GPU ranking time
- finalist/exactify time
- profiling-only diagnostic time

Do not mix those together in one conclusion.

## 5.4 Profiling Stack

Use this stack in order unless current evidence gives a strong reason not to.

1. **NVTX instrumentation**
   - required before deeper Nsight work
   - current top-level options capture names:
     - `options_expression_feasibility_total`
     - `options_expression_run_total`
     - `tauri_options_expression_feasibility_request`
     - `tauri_options_expression_request`

2. **Nsight Systems**
   - first profiler for options CUDA work
   - use it to answer:
     - whether host replay/build still dominates
     - whether upload/copy/sync behavior is visible in wall time
     - whether kernel launch shape is suspicious

3. **Nsight Compute**
   - only after Systems identifies the kernel/range worth drilling into
   - default kernel target for the current options scorer:
     - `options_expression_score_split_f64`
   - use `-RequestArgs` if you need a denser feasibility workload than the default script grid

4. **Compute Sanitizer**
   - use before keeping aggressive CUDA-side changes
   - especially valuable before or after widening CUDA-resident work

5. **WPA / ETW CPU sampling and GPUView**
   - use when the CPU/CUDA gap is still narrow
   - especially valuable for:
     - replay/build CPU hot paths
     - prepare/exactify CPU hot paths
     - WDDM scheduling or queue-shape questions on Windows

6. **CUPTI or automated profiler capture**
   - only if repeated scripted profiling becomes important enough to justify custom tooling

Important Windows rule:

- run Nsight Compute, Nsight Systems, and CUDA profiling helpers from an elevated PowerShell session when local attach/capture permissions are unstable on this machine

## 5.5 Tool Selection Matrix

| Question | Primary tool | Secondary tool | Current repo entrypoint |
| --- | --- | --- | --- |
| Where does end-to-end time go? | Nsight Systems | phase05 summary JSON | `scripts/profile_options_expression_nsys.ps1` |
| Is host replay/build still dominant? | phase05 summary JSON + Nsight Systems | WPA CPU sampling | `scripts/run_options_expression_phase05.ps1` |
| Which CUDA kernel or range is actually hot? | Nsight Systems | Nsight Compute | `scripts/profile_options_expression_nsys.ps1` then `scripts/profile_options_expression_ncu.ps1` |
| Is the options scorer kernel inefficient? | Nsight Compute | source inspection of `options_expression.cu` | `scripts/profile_options_expression_ncu.ps1` |
| Are CUDA-side changes still correct? | Compute Sanitizer | targeted tests | `scripts/profile_options_expression_compute_sanitizer.ps1` |
| Is Windows scheduling or CPU overhead masking CUDA gains? | WPA / ETW | GPUView | manual Windows profiling flow |
| Do we need custom automated profiler capture? | CUPTI | existing Nsight scripts | defer until repeated profiling is clearly valuable |

---

## 6. Execution Checklist

Use this section as the active checklist for the next several passes.

## 6.1 Baseline reset

- [x] Reconfirm the current lane being investigated.
- [x] Reconfirm the canonical workload(s) for the pass.
- [x] Capture a before baseline on current `HEAD`.
- [x] Store or link the before artifact/run output.
- [x] Record the before state in the findings log.
- [x] Record which evidence source was used: summary JSON, Nsight Systems, Nsight Compute, Compute Sanitizer, WPA, GPUView, or another bounded source.

## 6.2 Lane A - Cold replay/build

- [x] Rerun current canonical options proof output on current `HEAD`.
- [x] Check whether replay/build is still the dominant cold bucket.
- [x] Verify replay/build sub-buckets are visible and trustworthy.
- [x] Identify the single largest removable replay/build slice.
- [x] Implement exactly one replay/build reduction family.
- [x] Rerun the same canonical proof pair after the change.
- [x] Compare cold replay/build before/after.
- [x] Compare total cold preview before/after.
- [x] Compare warm preview before/after.
- [x] Compare candidate density, invalid-slot rate, and coverage verdicts before/after.
- [x] Accept, defer, or reject the replay/build change explicitly.

## 6.3 Lane B - Conditional prepare dedup

- [x] Confirm prepare is now the dominant removable bucket.
- [x] Prove the intended dedup path preserves lane semantics.
- [x] Reject any blanket baseline/conservative/harsher sharing by assumption.
- [x] Implement one narrow dedup target only.
- [x] Re-run canonical proof pair.
- [x] Recheck tuple quality and verdict semantics.
- [x] Accept, defer, or reject the prepare-dedup change explicitly.

## 6.4 Lane C - Finalist-boundary reduction

- [ ] Split exactify/finalist timing if the current bucket is still too coarse.
- [ ] Confirm the removable summary-compatible finalist slice is materially large.
- [ ] Keep sealed ordering, exact extreme, and CPU oracle behavior fixed unless explicitly reopening them.
- [ ] Implement one retained-finalist boundary reduction only if justified.
- [ ] Re-run canonical proof pair.
- [ ] Check retained ordering, finalist membership, and validation verdict parity.
- [ ] Accept, defer, or reject the finalist-boundary change explicitly.

## 6.5 Lane D - Sufficient-statistics widening

- [ ] Confirm the change is mathematically exact or explicitly scoped as approximate.
- [ ] Keep state growth intentionally small.
- [ ] Re-run CPU vs GPU parity on widened metrics.
- [ ] Verify artifact compatibility when schemas widen.
- [ ] Confirm no meaningful hot-path runtime regression beyond the agreed budget.
- [ ] Accept, defer, or reject the state-widening change explicitly.

## 6.6 Lane E - CPU hot-path improvement

- [ ] Confirm the measured CPU/CUDA gap is narrow enough that CPU-side work is worth prioritizing.
- [ ] Confirm the target CPU work improves end-to-end economics or the practical payoff of the CUDA route.
- [ ] Reject CPU-side changes that weaken the CUDA-first product posture.
- [ ] Re-run bounded canonical workloads after the change.
- [ ] Compare total wall time and CPU vs CUDA speedup before/after.
- [ ] Accept, defer, or reject the CPU hot-path change explicitly.

## 6.7 Lane F - Ranking functional revision

- [ ] State the exact ranking inconsistency being fixed.
- [ ] Keep the revision scalar and GPU-friendly unless explicitly reopening that design.
- [ ] Re-run bounded canonical workloads.
- [ ] Compare retained ordering, validation winners, and sealed outcomes.
- [ ] Reject the change if it only games coverage without improving validated quality.

## 6.8 Lane G - Validation-frontier math

- [ ] Keep the work zero-extra-stage unless explicitly reopening that constraint.
- [ ] Label retained-scope versus sealed-scope math honestly.
- [ ] Do not change tuple ordering or finalist policy by accident.
- [ ] Verify reopened artifacts and fresh runs derive the same interpretation.

## 6.9 Deferred lane guard

- [ ] Do not reopen runtime/SASS work without new local evidence.
- [ ] If reopened, capture bounded profiling evidence first.
- [ ] Reject low-level CUDA work if end-to-end share remains too small.

## 6.10 Profiling workflow guard

- [x] Prefer NVTX + Nsight Systems before Nsight Compute.
- [x] Prefer Nsight Compute only after a hot kernel/range is identified.
- [x] Run Compute Sanitizer before accepting aggressive CUDA-side work.
- [x] Keep the repo-local profiler wrappers able to launch the release feasibility binary directly on Windows without relying on `cargo run`.
- [ ] Use WPA/GPUView when the CPU/CUDA gap remains narrow and Windows host scheduling may matter.
- [x] Keep profiler captures bounded to one workload and one hypothesis at a time.

## 6.11 Larger-workload CUDA decomposition

- [x] Confirm the workload is large enough that CUDA work is again worth isolating.
- [x] Keep ranking semantics, retained ordering, and CPU-oracle behavior fixed.
- [x] Increase useful work on device one bounded step at a time.
- [x] Re-run both a canonical dense workload and a larger workload after each CUDA-side change.
- [x] Compare kernel launches, tensor-score time, accelerated preview time, and warm preview time separately.
- [x] Accept the change only if parity stays intact and end-to-end preview economics improve on at least one representative workload family.

---

## 7. Performance Findings Log

Use this as the live record of measured performance findings and iteration decisions.
Append entries. Do not rewrite old findings unless the old finding was factually wrong.

| Date | Status | Lane | Profiler / evidence | Workload / run id | Command / artifact | Key metrics | Finding | Decision / next step |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-04-11 | prior baseline | A | phase05 summary | canonical SPY/QQQ reruns from Prompt 13 | Prompt 13 pass-2 summaries | cold replay/build still first-order; warm path already strong; GPU scorer share small | Replay/build remained the highest-value next target above runtime/SASS work | Treat replay/build as the default first lane until current `HEAD` disproves it |
| 2026-04-12 | prior baseline | deferred low-level | phase05 summary + prompt consolidation | canonical SPY/QQQ reruns from Prompt 14 | Prompt 14 final direction doc | runtime share still too small; one chunk; four launches; output buffers reused | Runtime/SASS/PTX work did not justify implementation | Keep runtime/SASS deferred by default |
| 2026-04-13 | planning | guide creation | document planning | n/a | this guide | active tracker created; no current-`HEAD` rerun yet | Independent iteration path is defined, but baseline must be refreshed on current code before choosing the next pass | Run current canonical options proof pair and log the new baseline |
| 2026-04-13 | accepted | A | phase05 summary JSON | dense SPY Phase05 feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) | `output/options_expression_phase05_dense_current/2026-04-13T13-51-11/phase05_summary.json` vs `output/options_expression_phase05_dense_binary_cache_steady/2026-04-13T14-09-28/phase05_summary.json` | cold persisted dataset cache lookup `163287us -> 47389us`; cold replay/build `167ms -> 50ms`; total preview `314ms -> 236ms`; density and invalid-slot rate unchanged at `61.1%` / `38.9%` | The dominant cold replay/build slice was persisted dataset cache deserialize cost in `src-tauri/src/commands_options_expression.rs`. Replacing JSON replay-cache deserialize with a binary-safe `bincode` cache format cut that slice by about 71% and reduced cold end-to-end preview time by about 25% without changing options semantics. | Keep the binary-safe replay cache change. Leave warm-preview remeasurement as a follow-up, not a blocker for accepting the cold-path win. |
| 2026-04-13 | accepted | deferred low-level / next-lane evidence | Nsight Systems + Nsight Compute + Compute Sanitizer | same dense SPY Phase05 workload | `target/nsight/options_expression_nsys_dense_current_whole.nsys-rep`; `target/nsight/options_expression_ncu_dense_app_only_2026-04-13.ncu-rep`; `target/nsight/options_expression_compute_sanitizer_dense_2026-04-13.memcheck.log` | Nsight Compute on `options_expression_score_split_f64`: `grid=32`, `block=64`, `waves/SM=0.01`, theoretical occupancy `100%`, achieved occupancy about `4.2%`, `No Eligible=97.18%`, `Eligible Warps/Scheduler=0.03`, `Memory Throughput=33.51 GB/s` / `3.15%` peak, `Compute Throughput=0.28%`, dominant stall `L1TEX scoreboard` about `79.7%`; Compute Sanitizer `ERROR SUMMARY: 0 errors` | Current scorer-kernel behavior is not a classic full-GPU saturation problem. The kernel is launch-size constrained and latency-bound, with too little parallel work per launch and very low achieved occupancy. The next CUDA win is more likely to come from increasing useful work per launch or changing the decomposition boundary than from blind SASS/PTX micro-tuning. | Keep broad runtime/SASS work deferred. Reopen CUDA work only as a bounded “more useful work per launch / larger grid / decomposition” investigation with repeatable profiler evidence. |
| 2026-04-13 | rejected | deferred low-level | phase05 summary JSON + bounded env override probe | same dense SPY Phase05 workload | `output/options_expression_phase05_dense_block32/2026-04-13T14-15-25/phase05_summary.json` and reverted local candidate-list experiment | fixed env override `VECTORBT_KERNEL_BLOCK_X=32` produced one promising scorer timing (`gpu_ranking_us ≈ 1537`), but widening the default scorer candidate list to admit `32` regressed a follow-up default run to `gpu_ranking_us ≈ 2233` and failed the local speedup gate | A manual block-size override is not strong enough evidence to ship a heuristic change. The small-workload launch shape is sensitive enough that simply widening the candidate list is not a safe default improvement. | Revert the candidate-list experiment. Keep the `block_x=32` observation as profiler context only. |
| 2026-04-13 | accepted | profiling workflow guard | direct launch repro + repo-local profiler smoke runs | bounded SPY feasibility smoke workload (`delay=0,1`, `hold=1..4`, `target/stop=0.1,0.2`) | `target/nsight/options_expression_ncu_script_smoke_fixed_spy_2026-04-13.ncu-rep`; `target/nsight/options_expression_nsys_script_smoke_fixed_spy_2026-04-13.nsys-rep`; `target/nsight/options_expression_compute_sanitizer_script_smoke_fixed_spy_2026-04-13.memcheck.log` | direct release launch previously failed with `0xC0000139`; after switching NVTX to lazy runtime loading and staging Cargo-style runtime DLL paths in the profiler wrappers, all three wrappers emitted their expected report/log artifacts on the same host | The broken Nsight Compute wrapper was really a Windows direct-launch environment problem. `cargo run` worked because it injects runtime DLL search paths that direct profiler launches were missing; the profiler wrappers now reproduce that environment and no longer depend on manual `ncu.exe` workarounds. | Keep the runtime-DLL PATH staging in the profiler wrappers. When profiling a pinned dataset, pass `-ArtifactPath` explicitly if the latest saved run artifact may target a different root symbol than the dataset. |
| 2026-04-13 | accepted | B | phase05 summary JSON + targeted `options_expression` tests | dense SPY Phase05 feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) | `output/options_expression_phase05_dense_current_head/2026-04-13T15-03-44/phase05_summary.json` vs `output/options_expression_phase05_dense_summary_bulk_long_single/2026-04-13T15-18-16/phase05_summary.json`; `cargo test -p ta_app_core options_expression --lib` | cold total preview `211ms -> 193ms`; replay/build stayed flat at about `49-50ms`; exit-summary compile total `30ms -> 25ms`; warm preview held at `11ms`; density / invalid-slot rate stayed `61.1%` / `38.9%` | The canonical Phase05 feasibility path is `LongSingle`, and prepare time was still paying repeated per-summary row scans inside `compile_exit_summary_cache`. Bulk-compiling each LongSingle hold path once and filling the target/stop matrix from that single scan cut cold preview by about `8.5%` without changing cache semantics, candidate coverage, or non-LongSingle behavior. | Keep the LongSingle bulk summary compiler. Do not generalize cross-stress sharing by assumption; if another pass is needed, target candidate-coverage limits or reopen CUDA only through a separately measured “more useful work per launch” change. |
| 2026-04-13 | accepted | B | phase05 summary JSON + targeted `options_expression` tests | same dense SPY Phase05 feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) | `output/options_expression_phase05_dense_summary_bulk_long_single/2026-04-13T15-18-16/phase05_summary.json` vs `output/options_expression_phase05_dense_summary_bulk_all_holds/2026-04-13T15-28-43/phase05_summary.json`; `cargo test -p ta_app_core options_expression --lib` | cold total preview `193ms -> 170ms`; replay/build stayed flat at `50ms -> 49ms`; exit-summary compile total `25ms -> 9ms`; warm preview held at `11ms`; density / invalid-slot rate stayed `61.1%` / `38.9%` | The first LongSingle prepare optimization still rescanned the same candidate path once per hold. Sweeping the LongSingle candidate path once across all holds and finalizing hold cutoffs in that single pass removed most of the remaining prepare-time row-replay overhead while keeping per-summary outcomes identical to the oracle. | Keep the all-holds LongSingle summary compiler as the current prepare baseline. The next pass should not stay in the same prepare bucket unless new evidence shows another hidden summary-compile cost. |
| 2026-04-13 | accepted | deferred low-level / diagnosis | phase05 summary JSON + bounded current-`HEAD` timing split | same dense SPY Phase05 feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) | `output/options_expression_phase05_dense_device_timing/2026-04-13T16-00-00/phase05_summary.json`; `cargo test -p ta_app_core options_expression --lib`; `cargo test -p ta_app_core options_types --lib`; `cargo check -p ta_desktop_demo_app --bin ta_options_expression_feasibility --features cuda,cuda-backtest-kernel` | cold total preview `115ms`; `prepare_us=1796`; `gpu_device_resolution_us=99125`; `gpu_plan_probe_us=320`; `gpu_ranking_us=1291`; `report_finalize_us=78` | Adding explicit feasibility sub-buckets showed that prepare, CPU ranking, and report finalization were already small. The dominant cold cost on current `HEAD` was the preferred-CUDA-device resolution path, not the options scorer math. | Keep the timing fields. Reopen low-level CUDA work only around this now-measured cold-start path rather than around prepare or scorer arithmetic. |
| 2026-04-13 | accepted | deferred low-level / bounded runtime follow-up | phase05 summary JSON + Nsight Systems + targeted tests | same dense SPY Phase05 feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) | `target/nsight/options_expression_nsys_dense_all_holds_dataset_2026-04-13.nsys-rep`; `output/options_expression_phase05_dense_device_resolver_lightweight/2026-04-13T16-35-00/phase05_summary.json`; `output/options_expression_phase05_dense_device_resolver_cubin/2026-04-13T17-05-00/phase05_summary.json`; `cargo test -p ta_app_core options_expression --lib`; `cargo check -p ta_desktop_demo_app --bin ta_options_expression_feasibility --features cuda,cuda-backtest-kernel` | old split: `gpu_device_resolution_us=99125`, `gpu_plan_probe_us=320`, cold preview `115ms`; lightweight resolver: `gpu_device_resolution_us=12628`, `gpu_plan_probe_us=87844`, cold preview `118ms`; resolver + `sm_89` cubin fast path: `gpu_device_resolution_us=11863`, `gpu_plan_probe_us=85013`, cold preview `113ms`, warm preview `5ms` | Replacing the preferred-device resolver's heavyweight `vector_ta::cuda_available()` gate removed about `87ms` from explicit device resolution, but the cold cost largely migrated into the first real options GPU runtime bring-up. Adding an `sm_89` cubin fast path for the options-expression kernel shaved that runtime bring-up modestly, leaving the remaining cold bucket dominated by one-time CUDA runtime/module initialization rather than by the scorer kernel or CPU-side math. | Keep the lightweight preferred-device resolver and the options-expression `sm_89` cubin fast path. Treat further wins as architectural only: a process-wide shared runtime and startup/idle prewarm are the remaining plausible paths, but they are deferred until the team is willing to trade a larger concurrency/lifecycle change for cold-start latency. |
| 2026-04-13 | accepted | deferred low-level / architectural cold-start | phase05 summary JSON + targeted runtime reuse test + desktop crate checks + GPU perf gate | same dense SPY Phase05 feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) with headless prewarm enabled | `output/options_expression_phase05_dense_device_resolver_cubin/2026-04-13T17-05-00/phase05_summary.json` vs `output/options_expression_phase05_dense_prewarm_runtime/2026-04-13T16-24-40/phase05_summary.json`; `cargo test -p ta_app_core prewarm_runtime_reuses_process_wide_entry_across_threads --lib --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app`; `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` | cold total preview `113ms -> 16ms`; replay/build held at `9ms -> 9ms`; `gpu_device_resolution_us 11863 -> 0`; `gpu_plan_probe_us 85013 -> 45`; warm preview held at `5ms`; density / invalid-slot rate stayed `61.1%` / `38.9%` | The remaining dominant cold bucket was architectural, not mathematical. Replacing the thread-local options runtime cache with a process-wide per-device runtime store and adding bounded startup/headless prewarm removed almost all one-time CUDA runtime/module bring-up from the canonical dense preview without changing tuple coverage, ranking behavior, or the existing warm path. | Keep the process-wide runtime reuse, the Tauri startup prewarm, and the headless `--prewarm-gpu-runtime` hook. Lane priority now shifts away from CUDA cold-start work; any further performance pass should justify itself either as CPU-side replay/build reduction or as a larger-workload CUDA decomposition win. |
| 2026-04-13 | accepted | deferred low-level / larger-workload CUDA decomposition | phase05 summary JSON + targeted `options_expression` CUDA tests | dense SPY Phase05 feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) | `output/options_expression_phase05_dense_fused_compare_legacy/2026-04-13T16-54-36/phase05_summary.json` vs `output/options_expression_phase05_dense_fused_compare_combined/2026-04-13T16-55-45/phase05_summary.json`; `cargo test -p ta_app_core fused_gpu_search_holdout_stress_aggregates_match_exact_cpu_aggregates --lib --features cuda,cuda-backtest-kernel`; `cargo test -p ta_app_core options_expression --lib --features cuda,cuda-backtest-kernel` | `gpu_kernel_launch_count 4 -> 1`; `gpu_tensor_score_us 2072 -> 1794`; `gpu_ranking_us 2202 -> 1889`; cold preview `75ms -> 73ms`; warm preview held at `12ms`; accelerated ranking median `769us -> 699us` | The first higher-work-per-launch win was to fuse baseline search, baseline holdout, conservative holdout, and harsher holdout into a single combined CUDA scorer. That did not materially change replay/build, but it removed three kernel launches and improved broad GPU-side ranking throughput without changing coverage, tuple density, or retained semantics. | Keep the combined fused scorer as the larger-workload CUDA baseline. If another CUDA pass is justified, target work that reduces host-side post-score cost or device-to-host transfer rather than reopening generic launch-shape tuning. |
| 2026-04-13 | accepted | deferred low-level / larger-workload CUDA decomposition | phase05 summary JSON + targeted GPU/CPU top-k parity tests + desktop crate checks + GPU perf gate | dense SPY feasibility workload and a widened larger workload (`delay=0,1,2,3`, `hold=1..32`, `target=0.1..0.8`, `stop=0.1..0.4`) | dense: `output/options_expression_phase05_dense_fused_compare_combined/2026-04-13T16-55-45/phase05_summary.json` vs `output/options_expression_phase05_dense_fused_compare_combined_scalarized/2026-04-13T17-07-57/phase05_summary.json`; large: `output/options_expression_phase05_large_fused_compare_combined/2026-04-13T16-58-24/phase05_summary.json` vs `output/options_expression_phase05_large_fused_compare_combined_scalarized_rerun/2026-04-13T17-07-38/phase05_summary.json`; `cargo test -p ta_app_core gpu_combined_ranking_matches_cpu_topk_on_small_surface --lib --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app`; `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` | dense: cold preview `73ms -> 69ms`, warm preview `12ms -> 11ms`, estimated accelerated backend preview `7ms -> 5ms`, accelerated ranking median `699us -> 645us`; large: cold preview `89ms -> 85ms`, warm preview `21ms -> 19ms`, estimated accelerated backend preview `12ms -> 10ms`; kernel-side `gpu_tensor_score_us` rose (`1794 -> 1927` dense, `1794 -> 2135` large) because the scorer now writes the ranking scalar on device | Emitting the robust ranking scalar from the combined CUDA kernel and only materializing full metric structs for retained `top_k` rows improved end-to-end accelerated preview economics on both the canonical dense workload and the widened larger workload. The kernel itself got slightly heavier, but the host-side post-score reduction was large enough to win overall. | Keep the combined-kernel scalarization plus lazy host `top_k` materialization. The remaining larger-workload CUDA opportunity is now device-resident shortlist/gather or further host-side replay/build reduction, not broader scorer fusion. |
| 2026-04-13 | rejected | deferred low-level / larger-workload CUDA decomposition | phase05 summary JSON + targeted `options_expression` CUDA tests + GPU perf gate | dense SPY feasibility workload and the widened larger workload (`delay=0,1,2,3`, `hold=1..32`, `target=0.1..0.8`, `stop=0.1..0.4`) | dense: `output/options_expression_phase05_dense_fused_compare_combined_scalarized/2026-04-13T17-07-57/phase05_summary.json` vs `output/options_expression_phase05_dense_fused_compare_combined_shortlist_gather/2026-04-13T18-11-06/phase05_summary.json`; large: `output/options_expression_phase05_large_fused_compare_combined_scalarized_rerun/2026-04-13T17-07-38/phase05_summary.json` vs `output/options_expression_phase05_large_fused_compare_combined_shortlist_gather/2026-04-13T18-11-24/phase05_summary.json`; `cargo test -p ta_app_core fused_gpu_search_holdout_stress_aggregates_match_exact_cpu_aggregates --lib --features cuda,cuda-backtest-kernel`; `cargo test -p ta_app_core gpu_combined_ranking_matches_cpu_topk_on_small_surface --lib --features cuda,cuda-backtest-kernel`; `cargo test -p ta_app_core options_expression --lib --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app`; `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` | dense: cold preview `69ms -> 189ms`, warm preview `11ms -> 10ms`, accelerated backend preview stayed `5ms`, accelerated ranking median regressed `645us -> 1110us`; large: cold preview `85ms -> 178ms`, warm preview held `19ms`, accelerated backend preview moved `10ms -> 9ms`, but accelerated ranking median regressed `1006us -> 1598us` | The extra device-resident shortlist/gather stage added enough launch, synchronization, and retained-row compaction cost that it lost on the current representative workload family. Millisecond-level preview noise on the widened workload was not strong enough evidence to offset the much worse ranking latency and sharply worse cold path. | Reject and revert the shortlist/gather path. Keep scalarization plus lazy host `top_k` materialization as the accepted CUDA baseline, and only revisit device-side gather if a later multi-chunk workload proves device-to-host transfer is again first-order. |
| 2026-04-13 | accepted | deferred low-level / larger-workload CUDA decomposition | phase05 summary JSON + targeted `options_expression` CUDA tests + desktop crate checks + GPU perf gate | dense SPY feasibility workload and the widened larger workload (`delay=0,1,2,3`, `hold=1..32`, `target=0.1..0.8`, `stop=0.1..0.4`) | dense: `output/options_expression_phase05_dense_fused_compare_combined_scalarized/2026-04-13T17-07-57/phase05_summary.json` vs `output/options_expression_phase05_dense_scalar_only_stablesort/2026-04-13T18-49-51/phase05_summary.json`; large: `output/options_expression_phase05_large_fused_compare_combined_scalarized_rerun/2026-04-13T17-07-38/phase05_summary.json` vs `output/options_expression_phase05_large_scalar_only_stablesort/2026-04-13T18-50-28/phase05_summary.json`; `cargo test -p ta_app_core fused_gpu_search_holdout_stress_aggregates_match_exact_cpu_aggregates --lib --features cuda,cuda-backtest-kernel`; `cargo test -p ta_app_core gpu_combined_ranking_matches_cpu_topk_on_small_surface --lib --features cuda,cuda-backtest-kernel`; `cargo test -p ta_app_core options_expression --lib --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app`; `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` | dense: cold preview held `69ms -> 70ms`, warm preview improved `11ms -> 10ms`, estimated accelerated backend preview improved `5ms -> 4ms`, accelerated ranking median improved `645us -> 543us`, `gpu_ranking_us` improved `2023 -> 1717`; large: cold/warm preview held `85ms -> 85ms` / `19ms -> 19ms`, estimated accelerated backend preview improved `10ms -> 9ms`, accelerated ranking median improved `1006us -> 918us`, while direct `gpu_ranking_us` rose slightly `2248 -> 2344`; repo GPU perf gate stayed green | The combined scorer was still carrying an extra search-expectancy scalar output and host copy that no longer affected ranking decisions after the shortlist/gather rollback. Removing that redundant scalar from `options_expression.cu` and `options_expression_gpu.rs` kept the one-pass design intact and improved end-to-end accelerated preview/ranking behavior on both representative workloads, even though the planner telemetry did not expose the extra host-copy cost directly. | Keep the redundant combined-scalar removal. The next pass should move away from this CUDA path unless a new one-pass change has similarly strong end-to-end evidence; prefer replay/build or prepare-path reductions next. |
| 2026-04-13 | rejected | A | phase05 summary JSON + targeted desktop options-expression tests | dense SPY feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) | `output/options_expression_phase05_dense_scalar_only_stablesort/2026-04-13T18-49-51/phase05_summary.json` vs `output/options_expression_phase05_dense_dataset_cache_readbytes/2026-04-13T18-59-34/phase05_summary.json`; `cargo test -p ta_desktop_demo_app commands_options_expression --lib` | cold replay/build regressed `48ms -> 58ms`; persisted dataset cache lookup regressed `46266us -> 56292us`; cold total preview regressed `70ms -> 79ms`; warm preview held at `10ms` | Replacing the persisted dataset cache's `BufReader` + `bincode::deserialize_from` path with `std::fs::read` + `bincode::deserialize` made the cold-path dataset replay slower on the real 4.4 MB cache artifact. The extra contiguous-buffer allocation/copy cost outweighed any trait-call savings from slice-based deserialization on this host. | Reject and revert the read-bytes loader idea. If replay/build is reopened again, require a materially different approach such as an archived / mmap-friendly cache format rather than another buffered-read micro-variant. |
| 2026-04-13 | rejected | B | phase05 summary JSON + targeted `options_expression` CUDA tests | dense SPY feasibility workload (`delay=0,1,2,3`, `hold=1..32`, `target/stop=0.1..0.4`) | `output/options_expression_phase05_dense_scalar_only_stablesort/2026-04-13T18-49-51/phase05_summary.json` vs `output/options_expression_phase05_dense_long_single_hitmask_free/2026-04-13T19-03-50/phase05_summary.json`; `cargo test -p ta_app_core options_expression --lib --features cuda,cuda-backtest-kernel` | cold preview regressed `70ms -> 71ms`; warm preview held at `10ms`; estimated accelerated backend preview regressed `4ms -> 5ms`; accelerated ranking median regressed `543us -> 624us`; prepare time stayed effectively flat at `13ms -> 13ms` | Removing the per-row `Vec<bool>` hit-mask allocations inside the LongSingle all-holds summary compiler looked plausible on paper, but it did not improve the real phase05 prepare path and slightly worsened the end-to-end accelerated ranking/preview path. The extra recomputation outweighed the saved short-lived allocations on this workload. | Reject and revert the hit-mask-allocation change. If prepare is revisited again, require a stronger one-pass restructuring such as aligned multi-stress compilation rather than another micro-allocation tweak. |
| 2026-04-13 | rejected | B | phase05 summary JSON + targeted `options_expression` CUDA tests | dense SPY feasibility workload and widened larger workload (`delay=0,1,2,3`, `hold=1..32`, dense `target/stop=0.1..0.4`, large `target=0.1..0.8`, `stop=0.1..0.4`) | dense: `output/options_expression_phase05_dense_scalar_only_stablesort/2026-04-13T18-49-51/phase05_summary.json` vs `output/options_expression_phase05_dense_long_single_multi_stress/2026-04-13T19-14-34/phase05_summary.json`; large: `output/options_expression_phase05_large_scalar_only_stablesort/2026-04-13T18-50-28/phase05_summary.json` vs `output/options_expression_phase05_large_long_single_multi_stress/2026-04-13T19-14-55/phase05_summary.json`; `cargo test -p ta_app_core options_expression --lib --features cuda,cuda-backtest-kernel` | dense: cold preview `70ms -> 71ms`, warm preview `10ms -> 11ms`, accelerated backend preview `4ms -> 7ms`, accelerated ranking median `543us -> 618us`; large: cold preview `85ms -> 86ms`, warm preview `19ms -> 22ms`, accelerated backend preview `9ms -> 12ms`, accelerated ranking median `918us -> 964us` | Broadening the accepted LongSingle all-holds compiler into an aligned multi-stress summary sweep did increase useful work per prepare pass, but it also forced enough extra state/update work that both representative workloads regressed. The existing per-stress prepare split is still the better end-to-end baseline on current `HEAD`. | Reject and revert the aligned multi-stress summary compiler. Do not reopen this bucket again without a materially different one-pass formulation or profiler evidence that stress-bound prepare has become first-order. |
| 2026-04-13 | rejected | deferred low-level / larger-workload CUDA decomposition | phase05 summary JSON + targeted `options_expression` CUDA tests + desktop crate checks | dense SPY feasibility workload and widened larger workload (`delay=0,1,2,3`, `hold=1..32`, dense `target/stop=0.1..0.4`, large `target=0.1..0.8`, `stop=0.1..0.4`) | dense: `output/options_expression_phase05_dense_scalar_only_stablesort/2026-04-13T18-49-51/phase05_summary.json` vs `output/options_expression_phase05_dense_scalar_rankonly_exactify/2026-04-13T19-30-18/phase05_summary.json`; large: `output/options_expression_phase05_large_scalar_only_stablesort/2026-04-13T18-50-28/phase05_summary.json` vs `output/options_expression_phase05_large_scalar_rankonly_exactify/2026-04-13T19-31-33/phase05_summary.json`; `cargo test -p ta_app_core options_expression --lib --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`; `cargo check -p ta_desktop_demo_app` | dense: cold/warm preview held at `70ms` / `10ms`, but accelerated backend preview regressed `4ms -> 5ms` and accelerated ranking median regressed `543us -> 921us`; large: cold preview improved `85ms -> 84ms`, warm preview improved `19ms -> 18ms`, but accelerated ranking median regressed `918us -> 1544us` and direct `gpu_ranking_us` regressed `2344 -> 2551` | Keeping only rank-order scalars on device and reconstructing retained rows through an exactify pass looked attractive as a stricter one-pass ranking path, but the follow-on retained-row reconstruction and host-side sort cost dominated any scalar-copy savings. The current combined path already sits near the useful one-pass boundary for this workload family. | Reject and revert the rank-only retained-row exactify path. Keep the combined scalar-only stable-sort baseline; reopen CUDA only for a larger workload where device-to-host tuple materialization becomes first-order again. |
| 2026-04-13 | rejected | deferred low-level / launch-shape probe | phase05 summary JSON with env override, using fresh prewarmed post-revert baselines | dense SPY feasibility workload and widened larger workload (`delay=0,1,2,3`, `hold=1..32`, dense `target/stop=0.1..0.4`, large `target=0.1..0.8`, `stop=0.1..0.4`) | dense baseline rerun: `output/options_expression_phase05_dense_post_revert_prewarm_rerun/2026-04-13T19-41-20/phase05_summary.json` vs `output/options_expression_phase05_dense_block32_post_revert/2026-04-13T19-42-13/phase05_summary.json`; large baseline rerun: `output/options_expression_phase05_large_post_revert_prewarm/2026-04-13T19-41-30/phase05_summary.json` vs `output/options_expression_phase05_large_block32_post_revert/2026-04-13T19-42-23/phase05_summary.json`; env override `VECTORBT_KERNEL_BLOCK_X=32` | dense: cold preview `74ms -> 70ms`, direct `gpu_ranking_us 2118 -> 1660`, `gpu_tensor_score_us 2013 -> 1569`, but accelerated ranking median moved `652us -> 733us`; large: cold preview `85ms -> 86ms`, direct `gpu_ranking_us 1969 -> 2374`, `gpu_tensor_score_us 1859 -> 2267`, accelerated ranking median `1034us -> 1036us` | For the current one-pass combined scorer, forcing `block_x=32` still helps the smaller dense workload, but the widened larger workload loses enough kernel throughput that there is no safe cross-workload heuristic win. The launch-shape sensitivity remains real, but the evidence is still too workload-specific to ship a default change. | Reject the block-size heuristic follow-up for now. Keep the default chooser unchanged and treat `VECTORBT_KERNEL_BLOCK_X=32` as profiler-only context, not product policy. |
| YYYY-MM-DD | pending / accepted / deferred / rejected | A/B/C/D/E/F/G | summary JSON / Systems / Compute / Sanitizer / WPA / GPUView / other | workload name | command or artifact path | summarize the important numbers only | one concrete interpretation | exact keep/defer/reject and next action |

### Log-writing rules

- Keep each entry to one decision.
- Quote the most important numbers, not every number.
- Always say whether the result changed the lane priority.
- If the finding depends on a local code change, include the file family in the entry text.

---

## 8. External Reference Log

Use this only when local evidence has already narrowed the question enough that external references are justified.

| Date | Question | Source type | Link or identifier | Repo-specific takeaway | Action taken |
| --- | --- | --- | --- | --- | --- |
| 2026-04-13 | confirm current Nsight Compute launch semantics on Windows before changing the wrapper | official docs | https://docs.nvidia.com/nsight-compute/pdf/NsightComputeCli.pdf | `application-only` is valid on Windows, `check-exit-code` defaults to enabled, and config-file lookup happens from the current working directory / Windows app-data locations. The wrapper fix should control launch environment directly instead of depending on ad hoc local shell state. | keep |
| YYYY-MM-DD | narrow question only | docs / paper / profiling guide / math reference | URL or short citation | one repo-grounded implication | keep / defer / reject |

### External-reference rules

- Use primary documentation or primary research where possible.
- Prefer CUDA/Nsight official docs for low-level runtime questions.
- Prefer mathematically explicit sources for ranking/statistics questions.
- Do not log generic blog posts unless they provided a concrete, testable repo implication.

---

## 9. Immediate Next Action

The immediate next action for active work under this guide is:

1. keep the accepted Lane A replay-cache change as the current cold-path baseline
2. keep the accepted Lane B LongSingle all-holds exit-summary compiler as the current prepare-path baseline
3. keep the process-wide options-expression runtime reuse and startup/headless prewarm path as the current cold-start baseline
4. keep the accepted larger-workload CUDA baseline:
   - fused search/holdout/stress scoring in one launch
   - device-side robust-score scalarization for the combined path
   - no redundant search-expectancy scalar output or host copy in the combined path
   - lazy host materialization of retained `top_k` rows only
5. treat candidate coverage and `entry_past_cutoff` pressure, not scorer throughput or CUDA bring-up, as the main product blocker on the canonical dense workload
6. if another performance pass is justified, prefer CPU-side replay/build or other host-side end-to-end reductions before reopening generic kernel-level CUDA work
7. if another CUDA pass is justified, do not reopen shortlist/gather on the current workload family; require a materially larger multi-chunk workload and fresh profiler evidence before revisiting device-side retained-row gather
8. do not reopen aligned multi-stress LongSingle prepare fusion or rank-only retained-row exactify on the current workload family; both have now been measured and rejected on current `HEAD`
9. do not ship a block-size heuristic change for the current one-pass combined scorer without a cross-workload win; `VECTORBT_KERNEL_BLOCK_X=32` remains profiler-only context

The correct current stance is:

- Lane A produced an accepted end-to-end win and should remain landed
- Lane B also produced an accepted end-to-end win and should remain landed
- the current larger-workload CUDA baseline is now the combined fused scorer plus device-side scalarization and lazy retained-row materialization
- the current larger-workload CUDA baseline also removes the redundant combined search-expectancy output/copy that no longer contributes to ranking decisions
- the current canonical dense baseline is now the all-holds LongSingle prepare path plus process-wide runtime reuse and startup/headless prewarm, with cold preview at about `16ms` and warm preview at about `5ms`
- repo-local profiler wrappers are operational again on this Windows host and no longer require manual `ncu.exe` launch workarounds
- current feasibility timing now proves the remaining dense-workload cost is mostly replay/build plus small host-side overheads, not scorer math or one-time CUDA bring-up
- the process-wide prewarm/shared-runtime architecture change is now landed and validated; the next CUDA step must be an evidence-backed “more useful work per launch” win on a workload where device work is again first-order
- the current larger-workload CUDA work has already taken the low-risk fusion and scalarization wins, and the first device-resident shortlist/gather follow-up has now been rejected on the representative workload family
- the next two nearby one-pass follow-ups have also now been rejected on current `HEAD`: aligned multi-stress LongSingle prepare fusion and rank-only retained-row exactify
- the current combined scorer still shows launch-shape sensitivity, but a fresh `block_x=32` probe remains workload-specific and is not strong enough to turn into another shipped heuristic
- further blanket baseline/conservative/harsher prepare sharing is still unjustified without separate proof
- broad runtime/SASS work remains deferred unless a later larger workload makes the device layer first-order again

---

## 10. Acceptance And Stop Rules

Keep these rules active across all passes.

### Accept a pass only if

- the timing boundary and decision scope were clear before implementation
- the after-state improves the intended metric materially
- warm behavior does not regress unacceptably
- tuple quality or artifact semantics do not drift unintentionally
- the pass yields a clear keep/defer/reject outcome

### Stop a pass if

- the apparent win is only bucket reshuffling
- the change weakens provenance, identity, or oracle behavior
- the change forces unrelated scope expansion
- the measurement surface is too noisy or too different from the baseline to support a conclusion
- the pass cannot explain why it improved or regressed

### Default rejection triggers

- "more CUDA" without end-to-end evidence
- "CPU only" when the actual goal is to improve the CUDA-first product path and CPU work does not support that goal
- "more metrics" without a defined product or ranking use
- "more caching" without an identified dominant removable bucket
- "more realism" when the task is performance

---

## 11. Sign-Off Template

Use this wording pattern whenever a pass closes:

- lane investigated:
- before baseline:
- intervention:
- after result:
- parity outcome:
- decision:
- next lane:

Example shape:

- lane investigated: cold replay/build
- before baseline: replay/build dominated cold preview on current `HEAD`
- intervention: persisted dataset replay reuse
- after result: cold replay/build and total preview dropped materially; warm path held
- parity outcome: density, invalid-slot rate, and validation semantics unchanged
- decision: accepted
- next lane: rerun baseline and decide whether replay/build still dominates or prepare is now the next target
