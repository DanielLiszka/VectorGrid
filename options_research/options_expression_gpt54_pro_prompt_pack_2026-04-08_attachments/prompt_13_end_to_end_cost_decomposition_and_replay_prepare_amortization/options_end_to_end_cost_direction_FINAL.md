# Options End-To-End Cost Direction — FINAL

Date: 2026-04-11  
Status: final Prompt 13 consolidation decision for end-to-end cost decomposition and replay/prepare amortization inside the current `options_expression` workflow

## 1. Executive summary

The final Prompt 13 decision is a **revise-but-not-overturn** result.

The first-pass answer was directionally right that the next performance dollar still belongs **above the scorer**, not inside lower-level CUDA work. The second pass narrowed that conclusion.

The adopted direction is:

**reduce cold-path replay/build cost first, then only pursue conditional first-preview prepare dedup where semantic identity is provable.**

More concretely:

- **Adopt now:** finer replay/build decomposition inside `build_options_expression_replay_inputs(...)`, followed by one narrow cold-path reduction pass against the largest removable replay/build slice.
- **Defer:** conditional first-preview prepare dedup inside `prepare_options_expression_run(...)` only where baseline / conservative / harsher lanes can safely share structure or layout without collapsing real stress semantics.
- **Reject for now:** GPU runtime micro-architecture as the primary next step, and especially SASS/PTX-level work, because the current end-to-end evidence does not show scorer/runtime cost as first-order.

This is the right repo decision because the attached April 11 reruns show:

- cold preview is still dominated by `replay_build_ms`,
- cold prepare is the next meaningful bucket,
- GPU tensor scoring is already small,
- Prompt 12 already split out and reduced the fake finalist-overwrite ambiguity,
- and warm previews are already excellent.

So the repo should treat Prompt 13 as a **cold-preview / reopen-latency** question, not a “more CUDA by default” question.

## 2. Current end-to-end cost decomposition

### 2.1 What the repo already completed

The repo already has the important pieces that Prompt 13 depends on:

- artifact-driven options-expression follow-up work is shipped,
- the compiled-tensor GPU scorer is real and productized enough to clear the current live `auto` gates on bounded SPY and QQQ evidence,
- replay/build timing is already split from app-core feasibility timing,
- prepare timing is already split into tuple compile, candidate compile, and exit-summary compile lanes,
- Prompt 12 already split the retained-finalist tail into tensor score, exactify sub-buckets, and sealed-stage sub-buckets,
- process-local parent-signal and imported-dataset replay caches already exist,
- persisted signal-manifest replay artifacts already exist,
- structural candidate and exit-summary prepare caches already exist,
- and GPU runtime telemetry already exposes chunk count, launch count, upload/cache reuse, output-buffer reuse, and device-memory-fit notes.

That means the repo is **not** choosing between “build the GPU scorer” and “build caches.” It already has both. The remaining question is where the next **user-visible** end-to-end win still lives.

### 2.2 What the April 11 evidence says

#### Cold preview evidence

| Root | Replay/build cold | Prepare cold | GPU tensor score | GPU exactify | GPU sealed stage | Total preview cold |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| SPY | 258 ms | 21 ms | 1,236 us | 232 us | 107 us | 432 ms |
| QQQ | 198 ms | 62 ms | 2,801 us | 1,868 us | 274 us | 504 ms |

#### Warm preview evidence

| Root | Replay/build warm | Total preview warm | Structural cache | Summary cache |
| --- | ---: | ---: | --- | --- |
| SPY | 1 ms | 7 ms | 3 hits / 0 misses | 3 hits / 0 misses |
| QQQ | 1 ms | 24 ms | 3 hits / 0 misses | 3 hits / 0 misses |

#### Additional relevant observations

- SPY cold preview is roughly **59.7% replay/build** and **4.9% prepare**. GPU ranking is under **0.4%** of preview wall time.
- QQQ cold preview is roughly **39.3% replay/build** and **12.3% prepare**. GPU ranking is still under **1.0%** of preview wall time.
- SPY exactify summary-overwrite time is effectively gone (`1 us`), with the remaining exactify cost mostly enrichment (`193 us`) and a small exact `extreme` tail (`33 us`).
- QQQ exactify summary-overwrite time is also effectively gone (`0 us`), with the remaining exactify cost now mostly enrichment (`762 us`) plus exact `extreme` (`1,096 us`).
- Both reruns stay in a healthy runtime shape from the CUDA side: `gpu_tuple_chunk_count = 1`, `gpu_kernel_launch_count = 4`, and `gpu_output_buffer_reuses = 3`.

### 2.3 What those numbers mean

These numbers make four things clear.

**First, the scorer is no longer the first-order preview bottleneck.**  
Even on the weaker QQQ slice, GPU tensor scoring is only a few milliseconds, while replay/build and prepare together remain in the hundreds of milliseconds.

**Second, warm-path amortization is already strong.**  
The warm SPY and QQQ reruns collapse to `7 ms` and `24 ms`, with perfect structural and summary cache hit behavior in the captured samples. That is not the signature of a repo that is still missing generic cache reuse.

**Third, the remaining open problem is cold preview / reopen / first-preview cost.**  
That is where users will still feel delay, and that is where end-to-end optimization work can still pay off.

**Fourth, the current evidence does not justify leading with CUDA runtime internals.**  
One chunk, four launches, reused output buffers, and tiny scorer share are not the profile of a kernel/runtime layer screaming for immediate launch-shape or SASS work.

### 2.4 Coupling constraints that matter

The second pass changed the recommendation because the remaining costs are **not all independently removable**.

#### Replay/build is coupled to provenance and correctness

`build_options_expression_replay_inputs(...)` currently performs real correctness work, not just avoidable glue:

- parent artifact validation,
- parent-family support checks,
- source CSV/timeframe/adjustment reconstruction,
- parent replay cache lookup,
- persisted signal-manifest replay-cache lookup,
- underlying candle replay load/resample,
- parent drilldown reconstruction,
- signal-manifest projection,
- dataset manifest validation,
- dataset path validation,
- dataset hash/load lookup,
- fill-model compatibility checks,
- and request normalization.

A large `replay_build_ms` number does **not** automatically mean all of that time is removable.

#### Lane duplication is partly semantic, not purely accidental

`prepare_options_expression_run(...)` builds three broad prepared lanes:

- `baseline`
- `conservative`
- `harsher`

That duplication is not just cosmetic. Candidate and summary cache keys include stress-lane identity, and structural candidate compilation itself passes `stress_mode` into candidate selection. In the current runtime, entry semantics can differ across stress modes, which means eligibility and premium-sensitive filtering can legitimately differ too.

So the repo should **not** treat baseline / conservative / harsher as a blanket merge target.

The right interpretation is:

- broad lane merging is unsafe by default,
- summary-cache merging is unsafe by default,
- candidate-cache merging is unsafe by default,
- but **conditional** reuse of shared structure/layout work may still be worthwhile where semantics are provably identical.

That is a much narrower target than “remove lane duplication” in the abstract.

## 3. Final recommendation

### 3.1 Adopted direction

The adopted Prompt 13 direction is:

**cold-path replay/build decomposition and minimization first, followed by conditional first-preview prepare dedup only where semantic identity is provable.**

That should be implemented in this order.

#### Phase A — add finer replay/build decomposition

Before any larger optimization, split `replay_build_ms` into sub-buckets inside `build_options_expression_replay_inputs(...)` so the repo can see where cold-path time really goes.

At minimum, split:

- parent artifact read + validation,
- underlying replay/candle load + resample,
- parent replay/drilldown reconstruction,
- signal-manifest cache hit/load/project,
- options dataset manifest validation,
- options dataset CSV hash/load,
- and request normalization / compatibility checks.

This is the minimum honesty pass required before another optimization claim.

#### Phase B — optimize the largest removable cold replay/build slice

Once the sub-buckets exist, optimize the single largest safe cold-path slice.

Most likely candidates are:

- broadening reuse around parent replay/drilldown reconstruction,
- reducing repeated cold-path source validation work where the current provenance model already proves identity,
- or improving persisted replay/materialization reuse across preview / rerun / inspect entry points.

The rule is simple:

- preserve provenance checks,
- preserve dataset identity checks,
- preserve signal-manifest determinism,
- and only reduce work that is actually redundant under those constraints.

#### Phase C — only then test conditional first-preview prepare dedup

If replay/build work remains materially large after Phase B, the next in-scope step is **conditional** dedup inside `prepare_options_expression_run(...)`.

This should be limited to cases where the repo can prove that baseline / conservative / harsher lanes share:

- the same slot geometry,
- the same candidate validity surface,
- or the same layout/offset work,

without weakening the current stress semantics.

Examples of acceptable targets:

- reusing shared layout hashing / offset planning,
- reusing identical per-lane candidate geometry when request filters make lane divergence impossible,
- reusing derived GPU flat-input layout work when the actual outcome tensors must still remain separate.

Examples of unacceptable targets:

- silently sharing lane outputs when stress-mode entry/exit semantics differ,
- merging baseline and stressed summary caches by assumption,
- weakening premium-sensitive filtering just to manufacture reuse.

### 3.2 Why this beats the alternatives

#### Versus GPU runtime micro-architecture first

That direction is plausible in the abstract, but it loses on current evidence.

The attached reruns do not show:

- chunk explosion,
- launch-count explosion,
- upload thrash,
- memory-fit failure,
- or scorer-share dominance.

So runtime work is currently a **defer** question, not an **implement now** question.

#### Versus “keep current implementation and stop”

That is the strongest conservative alternative, and it is defensible because warm previews are already excellent and cold previews remain sub-half-second on the canonical pair.

It loses only because there is still one narrow, evidence-backed place left to look for meaningful user-visible improvement: first preview / reopen / cold-start latency.

The repo should not keep optimizing indefinitely, but it also has not exhausted that specific surface yet.

#### Versus generic “add more caching”

That direction is too broad and too sloppy.

The warm-path evidence already proves that the repo’s current replay and prepare reuse is doing real work. The next pass must be **cold-path specific** and **provenance-safe**, not a generic cache campaign.

## 4. What remains unchanged

The following should remain unchanged for this pass:

- the current artifact-driven options workflow shape,
- the current supported parent-family surface,
- the current supported structure-family surface,
- the current aggregate-first provider posture,
- the current CPU fallback / CPU-oracle posture,
- the current Prompt 12 retained-summary reuse boundary,
- the current sealed-stage ownership boundary,
- the current exact CPU `extreme` lane,
- the current `top_k` and `sealed_finalist_count` semantics,
- the current gate-driven `auto` backend policy,
- and the current CUDA runtime contract unless later measurements prove it has become first-order.

This pass should also **not** smuggle in:

- density/search-shape redesign,
- split-assignment default changes,
- realism-lane expansion,
- or broader workflow/product-scope changes.

## 5. Deferred ideas

The following are worth keeping alive, but not as the first implementation move.

### 5.1 Conditional prepare/lane dedup beyond the smallest safe slice

Only revisit this after replay/build sub-buckets show that cold prepare is still a real user-visible share and after the repo proves semantic identity where reuse is proposed.

### 5.2 GPU runtime micro-architecture work

Only revisit this if, after replay/build and safe prepare work, the scorer/runtime layer becomes a first-order share or the runtime shape degrades beyond the current one-chunk / four-launch / reused-buffer profile.

### 5.3 Broader prepared-run or artifact-level reuse layers

Only revisit this if the current process-local and persisted replay caches still leave a large cold-path burden that is clearly redundant and correctness-safe to amortize.

### 5.4 Prompt 14-style CUDA runtime or SASS work

Only revisit this after end-to-end evidence says the runtime layer is now the real bottleneck and profiler evidence confirms that lower-level work is justified.

## 6. Rejected ideas

The following should be rejected for this Prompt 13 pass.

### 6.1 SASS/PTX-first work

This is fake progress for the current evidence profile.

### 6.2 Broad “more CUDA” heroics

The scorer is already viable. The current bottleneck is still above it.

### 6.3 Generic caching as the headline recommendation

The warm path already proves the repo has meaningful reuse. The remaining work must be cold-path specific.

### 6.4 Weakening provenance or identity checks for speed

Any cold-path improvement that relies on stale parent artifacts, stale datasets, or looser source identity checks should be rejected.

### 6.5 Blanket merging of baseline / conservative / harsher lanes

Current lane duplication is partly semantic. Default merging is not an acceptable shortcut.

### 6.6 Manufacturing wins by shrinking budgets

Do not use `top_k`, `sealed_finalist_count`, or other budget knobs to fake an end-to-end performance win for this prompt.

## 7. Concrete code touch points by file and symbol

### 7.1 Immediate implementation surface

#### `src-tauri/src/commands_options_expression.rs`

Primary touch points:

- `build_options_expression_replay_inputs(...)`
- `load_parent_signal_cache_entry(...)`
- `load_signal_manifest_replay_cache(...)`
- `load_options_dataset_cache_entry(...)`
- `load_replayed_candles(...)`
- `analyze_options_expression_feasibility_blocking_impl(...)`

Primary responsibilities for this pass:

- add replay/build sub-bucket timing,
- preserve provenance/identity checks,
- identify the largest removable cold sub-bucket,
- and keep preview/run/inspect replay behavior aligned.

#### `crates/app_core/src/options_expression.rs`

Primary touch points:

- `prepare_options_expression_run(...)`
- `get_or_build_structural_candidate_cache(...)`
- `get_or_build_exit_summary_cache(...)`
- `structural_candidate_cache_key(...)`
- `exit_summary_cache_key(...)`
- `compile_structural_candidate_cache(...)`
- `compile_exit_summary_cache(...)`
- `build_gpu_flat_score_inputs(...)`

Primary responsibilities for this pass:

- preserve current prepare semantics,
- prototype only narrow and provably safe first-preview dedup,
- and keep baseline / conservative / harsher lane meaning explicit.

### 7.2 Additive telemetry surface, only if needed

#### `crates/app_core/src/options_types.rs`

Touch only if the repo decides that richer replay/build timing must be carried through shared artifact/report surfaces rather than staying Tauri-local.

Primary touch point:

- `OptionsExpressionFeasibilityTimingStats`

Preferred rule:

- keep replay/build timing local to the preview response if possible,
- widen shared timing schema only if the reporting surface truly needs it.

### 7.3 Explicitly deferred surface

#### `crates/app_core/src/options_expression_gpu.rs`

Current deferred touch points:

- `OptionsGpuChunkPlan::for_inputs(...)`
- `OptionsExpressionGpuRuntime::score_ranges(...)`
- `OptionsGpuRuntimeCacheUsage`

These should remain measurement-only touch points for now, not the first implementation target.

## 8. Benchmark order

Use the same canonical pair already stabilized by the Prompt 12 work.

### Step 1 — freeze the baseline

Reproduce the current SPY and QQQ cold + warm preview pair with the existing feasibility/gate harness so the before-state is pinned.

Capture at minimum:

- `replay_build_ms`
- `total_preview_ms`
- summed prepare cost
- `cpu_ranking_us`
- `gpu_ranking_us`
- `gpu_tensor_score_us`
- `gpu_exactify_us`
- `gpu_sealed_stage_us`
- structural and summary cache hits/misses
- GPU chunk count, launch count, and output-buffer reuse

### Step 2 — add replay/build sub-buckets with no behavior change

Re-run the same SPY and QQQ pair after replay/build timing decomposition lands.

Goal:

- identify which cold sub-bucket actually dominates,
- and determine whether it is reusable, reducible, or correctness-critical.

### Step 3 — implement one narrow replay/build reduction

Implement only the largest safe cold-path reduction.

Re-run the same SPY and QQQ pair cold and warm.

Goal:

- prove a real user-visible cold-preview win,
- while preserving warm-path behavior and semantic parity.

### Step 4 — only if needed, prototype conditional prepare dedup

If replay/build remains materially large, add one narrow prepare-dedup experiment.

Re-run the same SPY and QQQ pair again.

Goal:

- prove a further cold-preview win,
- without collapsing baseline / conservative / harsher semantics.

### Step 5 — only then reconsider runtime profiling

Only after Steps 1–4 should the repo ask whether `options_expression_gpu.rs` or Prompt 14-style profiling is worth reopening.

## 9. Verification order

### 9.1 Correctness and cache/provenance verification first

Run targeted backend tests covering:

- parent replay / signal-manifest cache correctness,
- imported-dataset cache correctness,
- cache invalidation on changed parent artifact or dataset identity,
- and deterministic replay/build behavior across repeated preview / rerun / inspect paths.

### 9.2 Standard backend options verification

At minimum:

- `cargo test -q -p ta_app_core options_ --lib`
- `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`
- `cargo check -q -p ta_desktop_demo_app`

### 9.3 CUDA build verification when relevant

If any change touches prepare or scorer-adjacent paths:

- `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`

### 9.4 Canonical SPY/QQQ proof reruns

Re-run the same narrow proof harness used for the current options GPU promotion and Prompt 12 reruns.

Compare before vs after on:

- cold preview timing,
- warm preview timing,
- cache hits/misses,
- candidate density,
- invalid-slot rate,
- top retained tuple identities,
- pre-sealed ordering,
- sealed-finalist membership,
- final validation verdicts,
- and saved-artifact semantic summaries.

### 9.5 Stop before lower-level profiling unless the evidence moves

Do not start Nsight or SASS-level profiling as part of this Prompt 13 implementation unless the end-to-end proof pair newly shows the runtime layer becoming first-order.

## 10. Stop conditions

Stop before implementation, or revert after a prototype, if any of the following happen.

### 10.1 Cold replay/build turns out to be mostly correctness-critical

If replay/build sub-buckets show that most remaining cold time sits inside provenance checks, replay reconstruction, or dataset validation that cannot be safely amortized, stop rather than weaken the safety model.

### 10.2 The prototype does not produce a material cold-preview win

Reject the work if it does **not** deliver at least:

- about a **20% reduction in replay/build time**, and
- about a **10% reduction in cold `total_preview_ms`**

on **both** canonical roots, while preserving semantics.

### 10.3 Warm behavior regresses

Reject the work if warm preview regresses materially on either root. As a practical rule, do not accept more than:

- roughly **10% warm regression**, or
- more than **2 ms** additional warm preview time,

whichever is tighter for the relevant root.

### 10.4 Candidate or ranking semantics drift

Reject the work if it changes any of the following without an explicit non-performance justification:

- candidate density,
- invalid-slot rate,
- overlap / adequacy verdicts,
- top retained tuple identities,
- pre-sealed ordering,
- sealed-finalist membership,
- validation verdicts,
- or saved-artifact summary meaning.

### 10.5 Prepare dedup requires semantic shortcuts

Reject any prepare/lane dedup proposal that only works by pretending baseline / conservative / harsher lanes are interchangeable when the current runtime says they are not.

### 10.6 The remaining bottleneck shifts below the scorer threshold for user-visible gain

If the replay/build and safe prepare passes together do not produce a clear user-visible win, stop there and keep the current implementation rather than continuing into runtime/SASS churn.

---

## Final decision

**Implement now:** cold-path replay/build decomposition and one narrow cold replay/build reduction pass.  
**Defer:** conditional first-preview prepare dedup, then runtime micro-architecture only if the bottleneck truly shifts there.  
**Reject for now:** SASS/PTX work, broad generic caching campaigns, and blanket lane merging.

That is the narrowest defensible Prompt 13 direction for the current VectorGrid options-expression lane.
