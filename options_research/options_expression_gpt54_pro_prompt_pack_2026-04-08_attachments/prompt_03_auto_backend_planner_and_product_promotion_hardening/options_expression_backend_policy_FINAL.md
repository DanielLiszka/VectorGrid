# Options Expression Backend Policy — Final Direction

Date: 2026-04-09  
Status: final consolidation for Prompt 3 (`Auto-Backend, Planner, And Product-Promotion Hardening`)

## 1. Executive summary

The current options-expression `auto` backend policy is **directionally correct but operationally one step too optimistic**. The shipped SPY and bounded QQQ sign-off slices already clear the current live `auto` floors and both currently recommend `gpu_compiled_tensor_hybrid`, while the older SPY gate artifact shows that the repo has already moved past the earlier, more conservative promotion posture. At the same time, runtime `Auto` still decides from workload heuristics alone and only discovers planner-fit failure *after* attempting the GPU path.

**Final recommendation:** implement **one narrow shared soft planner-preflight hook for runtime `Auto`**. Reuse the existing planner inspection logic and telemetry already exposed by `options_expression_gpu.rs`, and let runtime `Auto` consult that planner-fit result before it chooses to attempt the GPU path. Keep the current numeric workload thresholds unchanged. Keep CPU fallback unchanged. Keep the measured-speed promotion gate **out** of runtime `Auto`.

| Bucket | Decision |
|---|---|
| Implement now | Shared soft planner-preflight gate for runtime `Auto` |
| Defer | Threshold retuning, benchmark-derived routing classifier, GPU finalist-boundary changes, SASS/PTX work |
| Reject now | Reintroducing measured-speed gating into runtime `Auto`, manual-toggle-first as the main answer, broad routing redesign, split-policy reopen |

## 2. Current auto-backend evidence summary

### 2.1 What the repo has already completed

The current repo already ships an artifact-driven options-expression system with CPU reference scoring, CUDA compiled-tensor scoring, holdout/sealed/stress validation lanes, artifact persistence, compare surfaces, tuple drilldown, and event-path replay. The future roadmap explicitly treats the product as a **GPU-first**, artifact-driven optimizer and says GPU promotion hardening should make `auto` aggressively useful *without* turning it into blind GPU forcing.  
**Source anchors:**  
- `options_expression_future_scope_roadmap_2026-04-07.md:40-62, 66-85, 556-566, 858-888`  
- `options_expression_optimizer_viable_plan_2026-04-02.md:1671-1682, 1707-1709, 1845-1854`

### 2.2 What current code says

Current runtime `auto` is governed by these floors in `crates/app_core/src/options_expression.rs`:

- candidate density floor: `0.25`
- invalid-slot ceiling: `0.75`
- compiled tuple floor: `256`
- search-event floor: `8`
- projected event-tuple floor: `10_000`
- measured GPU speedup floor: `1.25x` *(promotion-only, not runtime `Auto`)*

The actual decision split is important:

1. `evaluate_auto_backend_decision(...)` can include the measured-speed gate.
2. `runtime_auto_backend_decision(...)` deliberately calls it with `None`, so runtime `Auto` does **not** use the measured-speed promotion gate.
3. `rank_compiled_tuples(...)` uses runtime `Auto` eligibility to decide whether to attempt GPU; under `Auto`, it still attempts GPU first whenever those heuristic gates pass, and only then falls back to CPU if the GPU path errors.
4. `analyze_options_expression_feasibility_blocking(...)` already does a planner probe and already demotes `auto_backend_decision` to CPU if planner probing or GPU ranking fails.

That means the repo already has a **feasibility/runtime semantic gap**: feasibility knows planner-fit truth earlier than runtime route selection does.  
**Source anchors:**  
- `crates/app_core/src/options_expression.rs:40-49`  
- `crates/app_core/src/options_expression.rs:1914-1987`  
- `crates/app_core/src/options_expression.rs:2948-2988`  
- `crates/app_core/src/options_expression.rs:3692-3745`  
- `crates/app_core/src/options_expression.rs:3936-3955`

### 2.3 What planner telemetry already exists

The GPU planner already exposes exactly the signals runtime `Auto` is missing:

- current device total/free/usable memory
- memory-budget source (`free_memory_headroom` vs `total_memory_fallback`)
- whether the outcome tensor fits
- whether the chunk plan fits
- planner notes
- chunk counts/sizes and device-byte estimates

`OptionsGpuChunkPlan::for_inputs(...)` already computes these fit decisions from **current free memory plus headroom**, and `inspect_options_expression_gpu_plan(...)` already exports them through `OptionsGpuRuntimeCacheUsage`.  
**Source anchors:**  
- `crates/app_core/src/options_expression_gpu.rs:36-57`  
- `crates/app_core/src/options_expression_gpu.rs:110-124`  
- `crates/app_core/src/options_expression_gpu.rs:179-272`  
- `crates/app_core/src/options_expression_gpu.rs:540-555`  
- `crates/app_core/src/options_expression_gpu.rs:634-664`  
- `crates/app_core/src/options_expression_gpu.rs:759-763`  
- `crates/app_core/src/options_types.rs:827-870`

### 2.4 What the current sign-off artifacts show

Supporting local artifacts:

- [Current SPY summary](sandbox:/mnt/data/OUTPUT~1.JSO)
- [Current QQQ summary](sandbox:/mnt/data/OUTPUT~2.JSO)
- [Older SPY gate summary](sandbox:/mnt/data/SPY_PH~1.JSO)

| Workload | Candidate density | Invalid-slot rate | `auto` recommendation | Planner fit booleans | Estimated preview wall speedup vs CPU |
|---|---:|---:|---|---|---:|
| SPY current sign-off | 52.47% | 47.53% | `gpu_compiled_tensor_hybrid` | outcome=`true`, chunk=`true` | 1.33x |
| QQQ current sign-off | 25.13% | 74.87% | `gpu_compiled_tensor_hybrid` | outcome=`true`, chunk=`true` | 1.55x |
| Older SPY gate artifact | 52.47% | 47.53% | `cpu_reference` | outcome=`true`, chunk=`true` | 1.33x |

Interpretation:

- The shipped SPY and QQQ envelope already works under current code.
- The current floors are **not obviously too conservative**.
- The remaining weakness is not “GPU is not viable.”  
  It is: runtime `Auto` still cannot use planner-fit truth until after it has already chosen to try GPU.

### 2.5 What is already closed and should stay closed

Prompt 2 is closed. The split-assignment ablation found that `session_chronological` did **not** produce a cross-root executable-coverage win; `event_chronological` remains the default, and split assignment remains a manual lever. That should **not** be reopened as an auto-backend policy dependency.  
**Source anchors:**  
- `options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md:523-537`  
- `crates/app_core/src/options_types.rs:100-101`

### 2.6 Bottom-line read on current policy

The current policy is **roughly correct strategically**, but **slightly too optimistic operationally**. It is no longer “GPU available = GPU recommended,” which was the old problem. Now the residual issue is narrower:

> runtime `Auto` still treats heuristic eligibility as sufficient to *attempt* GPU, even though the repo already has planner-fit truth that can sometimes say “not this time” before launch.

## 3. Final recommendation

### Implement one narrow change: a shared soft planner-preflight hook for runtime `Auto`

When `backend_preference == Auto` and CUDA is available:

1. Evaluate the current heuristic gates exactly as today.
2. **Only if those heuristic gates pass**, perform a lightweight planner inspect on the prepared baseline GPU inputs.
3. If planner inspection fails, or if the planner indicates `gpu_outcome_tensor_fits_in_memory == false` or `gpu_chunk_plan_fits_in_memory == false`, route directly to CPU and record the planner reason in the runtime auto-decision path.
4. If planner inspection passes, proceed with the current GPU attempt path.
5. Keep the existing reactive GPU->CPU fallback path in place for anything the preflight could not predict.

This is intentionally a **soft** planner-backed route alignment step, not a claim that planner preflight makes runtime fallback impossible.

### Why this is the right size of change

This beats a pure threshold tweak because the current thresholds already match the shipped envelope, and both current sign-off roots pass them. Threshold retuning would still be blind to the thing heuristics cannot know: **current free-memory fit at the time of launch**.

This also beats a broader routing redesign because the repo already has the planner logic, the telemetry surface, the feasibility-side demotion behavior, and unit-test seams for planner failure. The missing piece is not a new classifier. It is a narrow route-selection coherence fix.

### Why this should stay narrower than the first-pass version

The first-pass answer was directionally right but still sounded too close to “planner-backed route truth.” That is too strong. Planner fit is a point-in-time estimate based on current free memory and headroom. The implementation should therefore remain:

- advisory for runtime `Auto`
- non-invasive for forced GPU
- and non-destructive to the existing reactive CPU fallback path

## 4. Explicitly retained current strengths

Keep all of the following unchanged:

1. **Current numeric `auto` floors**  
   Keep `25%` density, `75%` invalid-slot ceiling, `256` compiled tuples, `8` search events, and `10,000` projected event-tuples unchanged for now.  
   **Source anchor:** `crates/app_core/src/options_expression.rs:40-49`

2. **Runtime vs promotion-gate separation**  
   Keep the measured-speed gate out of runtime `Auto`. The existing unit test explicitly protects that separation.  
   **Source anchor:** `crates/app_core/src/options_expression.rs:5078-5142`

3. **CPU oracle / fallback contract**  
   CPU remains the correctness oracle, forced debug path, and fallback lane.  
   **Source anchors:**  
   - `options_expression_optimizer_viable_plan_2026-04-02.md:1707-1709`  
   - `options_expression_future_scope_roadmap_2026-04-07.md:879-888`

4. **Free-memory-aware planner telemetry**  
   Keep the current free-memory + headroom planner model and its surfaced telemetry.  
   **Source anchors:** `crates/app_core/src/options_expression_gpu.rs:179-272, 540-555, 634-664`

5. **Current default split policy**  
   Keep `event_chronological` as default.  
   **Source anchors:**  
   - `crates/app_core/src/options_types.rs:100-101`  
   - `options_expression_split_assignment_ablation_implementation_guide_2026-04-09.md:533-537`

6. **Artifact-driven workflow and current product scope**  
   Do not widen families, workflow stages, or realism lanes here.  
   **Source anchors:**  
   - `options_expression_future_scope_roadmap_2026-04-07.md:40-62, 66-85, 556-566`  
   - `options_expression_optimizer_viable_plan_2026-04-02.md:1845-1854`

## 5. Explicitly deferred ideas

These are plausible later, but not the right next move now:

1. **Threshold recalibration from a broader benchmark set**  
   The roadmap keeps this as a future F3 option, but the current sign-off pair is already good enough that threshold churn now would mostly be noise-chasing.  
   **Source anchors:** `options_expression_future_scope_roadmap_2026-04-07.md:556-566, 858-888`

2. **Benchmark-derived or learned workload classifier**  
   Defer until there is evidence that heuristics + planner preflight still misroute representative workloads.

3. **GPU finalist-boundary expansion**  
   That is a different runtime-boundary question. It should not be bundled into auto-policy hardening.

4. **SASS/PTX-level tuning**  
   Defer until end-to-end cost decomposition shows the GPU scorer itself is the first-order bottleneck.

5. **Any split-policy reopen**  
   Prompt 2 already closed this.

6. **Any scope-widening realism or structure-family work**  
   Not a Prompt 3 problem.

## 6. Explicitly rejected ideas

Reject these as the next change:

1. **Reintroducing measured-speed gating into runtime `Auto`**  
   The current runtime-vs-promotion split is intentional and already unit-tested. Shipping a planner hardening change by smuggling the measured-speed gate into runtime would be the wrong fix.  
   **Source anchor:** `crates/app_core/src/options_expression.rs:5078-5142`

2. **A threshold-only fix**  
   A threshold tweak cannot see live memory fit, which is the one thing the planner already knows and the runtime route still ignores.

3. **A broad routing redesign now**  
   The repo already has enough machinery. A larger classifier or multi-tier route model is unnecessary until the narrow coherence fix is proven insufficient.

4. **Manual-user-toggle-first as the main policy answer**  
   The shipped posture already defaults to `auto`; the product question is how to make `auto` safer, not how to push responsibility back to the user.

5. **Reopening Prompt 2 as part of Prompt 3**  
   The split-ablation guide is already signed off; it is not a live blocker for backend policy.

## 7. Concrete code touch points by file and symbol

### 7.1 `crates/app_core/src/options_expression.rs`

**Keep unchanged**
- `OPTIONS_AUTO_GPU_MIN_*` constants  
  `:40-49`

**Primary edit surface**
- `evaluate_auto_backend_decision(...)`  
  `:1914-1981`
- `runtime_auto_backend_decision(...)`  
  `:1983-1987`
- `rank_compiled_tuples(...)`  
  `:2948-2988`
- `analyze_options_expression_feasibility_blocking(...)`  
  `:3692-3955`

**Recommended change shape**
- Add one shared helper near the current auto-decision code, for example:
  - `runtime_auto_backend_decision_with_planner_probe(...)`
  - or `auto_backend_decision_after_planner_preflight(...)`
- Have it:
  - run the current heuristic decision first
  - optionally call the GPU planner inspect when heuristics pass and `Auto` is the backend
  - append planner failure reasons to the same `OptionsExpressionAutoBackendDecision.reasons` surface
- Use the helper in both:
  - runtime route selection (`rank_compiled_tuples`)
  - feasibility/report shaping (`analyze_options_expression_feasibility_blocking`)
  so the report/runtime split cannot drift again

**Existing tests to preserve or extend**
- planner-failure demotion path  
  `:4872-4921`
- runtime-vs-promotion gate separation  
  `:5078-5142`

### 7.2 `crates/app_core/src/options_expression_gpu.rs`

**Existing planner telemetry surface to reuse**
- `OptionsGpuRuntimeCacheUsage`  
  `:36-57`
- `OptionsGpuChunkPlan`  
  `:110-124`
- `OptionsGpuChunkPlan::for_inputs(...)`  
  `:179-272`
- `score_ranges(...)` planner telemetry export  
  `:540-555`
- `inspect_plan(...)`  
  `:634-664`
- `inspect_options_expression_gpu_plan(...)`  
  `:759-763`

**Recommended change shape**
- Reuse `inspect_options_expression_gpu_plan(...)` exactly or wrap it with a tiny helper.
- Do **not** create a second planner concept.
- If needed, add a tiny repo-local helper that translates `OptionsGpuRuntimeCacheUsage` into a runtime auto-decision verdict without widening the GPU runtime API.

### 7.3 `crates/app_core/src/options_types.rs`

**Likely no schema change needed**
- `OptionsExpressionAutoBackendDecision`  
  `:827-832`
- `OptionsExpressionFeasibilityReport`  
  `:835-870`

The current `recommended_backend`, `gpu_eligible`, and `reasons` fields are already enough for the narrow policy change.

**Keep unchanged**
- default backend is still `Auto`  
  `:84-85`
- default split mode is still `EventChronological`  
  `:100-101`

### 7.4 `src-tauri/src/commands_options_expression.rs`

**Current surfaces already sufficient**
- `AnalyzeOptionsExpressionFeasibilityResponse`  
  `:39-42`
- `OptionsExpressionArtifactSummary`  
  `:58-94`
- artifact summary request split-mode surfacing  
  `:778-796`
- feasibility wrapper timing surface  
  `:1049-1071`

No large frontend/API redesign should be required for this change. The main value is that the existing feasibility and artifact surfaces will start telling a more route-coherent story with the same shapes.

## 8. Benchmark matrix

Use the smallest matrix that can validate the change without reopening unrelated questions.

### 8.1 Representative sign-off matrix

Run both representative roots:

- SPY bounded DTE7 sign-off slice
- QQQ bounded DTE7 sign-off slice

For each root, run:

- cold `Auto`
- warm `Auto`
- forced `GpuCompiledTensor`
- forced `CpuReference`

### 8.2 Record these outputs for every run

From feasibility and full-run results, record:

- `backend_used`
- `backend_fallback_reason`
- `auto_backend_decision.recommended_backend`
- `auto_backend_decision.gpu_eligible`
- `auto_backend_decision.reasons`
- `cache_stats.gpu_outcome_tensor_fits_in_memory`
- `cache_stats.gpu_chunk_plan_fits_in_memory`
- `cache_stats.gpu_memory_budget_source`
- `cache_stats.gpu_plan_notes`
- `timings.cpu_ranking_us`
- `timings.gpu_ranking_us`
- `timings.gpu_speedup_vs_cpu`
- `replay_build_ms`
- `total_preview_ms`

### 8.3 Synthetic planner-failure proof

Use the existing test seam that forces planner failure:

- `set_forced_options_expression_gpu_plan_error_for_test(...)`
- existing proof path at `crates/app_core/src/options_expression.rs:4872-4921`

Expected result after implementation:

- runtime `Auto` should choose CPU *before* launch when the planner error is injected
- forced GPU should still preserve its current error semantics

### 8.4 Success criteria

The change is successful only if all of the following are true:

1. SPY and QQQ current sign-off slices still choose GPU under `Auto`.
2. Synthetic planner failure causes runtime `Auto` to choose CPU without a post-attempt fallback path.
3. No measured-speed promotion logic leaks back into runtime `Auto`.
4. Median `total_preview_ms` under `Auto` does not regress by more than **5%** on the sign-off pair.
5. No new artifact/report ambiguity is introduced.

## 9. Verification and rollout plan

### 9.1 Backend verification

Run:

- `cargo test -q -p ta_app_core options_ --lib`
- `cargo check -q -p ta_desktop_demo_app`
- `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`

These are already the roadmap’s minimum verification expectations for backend/CUDA options changes.  
**Source anchor:** `options_expression_future_scope_roadmap_2026-04-07.md:980-991`

### 9.2 Targeted proof harness verification

Rerun the narrow proof harnesses the roadmap already names for CUDA options changes:

- `ta_options_expression_feasibility.rs`
- `run_options_expression_gpu_promotion_gate.ps1`

Capture before/after outputs under `output/options_expression_phase05` when planner/runtime behavior changes.  
**Source anchor:** `options_expression_future_scope_roadmap_2026-04-07.md:985-991`

### 9.3 Rollout posture

Roll out as a **backend-only coherence fix**:

1. land helper + tests
2. rerun SPY/QQQ sign-off matrix
3. keep frontend default at `auto` (already current posture)
4. do not change user-facing backend controls or workflow structure
5. inspect the first new saved artifacts for any unexpected increase in `backend_fallback_reason` or surprising CPU demotions

### 9.4 Artifact/reporting expectation after rollout

Saved artifacts and feasibility reports should continue to answer:

- what backend was recommended
- what backend actually ran
- whether fallback occurred
- whether planner fit was the blocking reason

The current schema and artifact summary surfaces are already shaped for that.  
**Source anchors:**  
- `crates/app_core/src/options_types.rs:827-870`  
- `src-tauri/src/commands_options_expression.rs:58-94, 1049-1071`

## 10. Rejection criteria and stop conditions

Reject or halt this change if any of the following occurs:

1. **False negative demotion on the representative pair**  
   After implementation, SPY or QQQ current sign-off runs under `Auto` get demoted to CPU, while forced GPU continues to run cleanly and remains the better backend.

2. **Measured-speed gate leaks into runtime `Auto`**  
   If implementation can only “work” by reusing `OPTIONS_AUTO_GPU_MIN_MEASURED_SPEEDUP` inside runtime route selection, stop. That would violate current intended semantics.

3. **No real trust gain, only overhead**  
   If the planner-preflight hook adds measurable preview cost but does not reduce avoidable GPU-attempt fallbacks (real or synthetic), stop.

4. **Preview wall time regresses materially**  
   If median `total_preview_ms` regresses by more than **5%** on both sign-off roots without a clear fallback-avoidance win, stop.

5. **Schema or UI churn becomes necessary**  
   If the change cannot be explained through the existing `reasons`, planner notes, and fallback surfaces, stop and keep the policy unchanged.

6. **The evidence remains purely synthetic**  
   If no representative workload ever exhibits the avoidable planner/runtime mismatch and the only proof remains a forced synthetic planner failure, do not widen the change into a larger routing rewrite. Keep it narrow or defer it.

7. **Broader product promotion still depends on one slice**  
   Even after this fix, broader product-promotion claims should remain blocked if:
   - only one canonical root stays healthy
   - planner notes are unstable across representative runs
   - or representative workloads still require runtime fallback under `Auto`

---

**Final decision:**  
Implement the **shared soft planner-preflight hook for runtime `Auto`**, keep the current numeric floors and runtime/promotion separation unchanged, and do not bundle that work with threshold retuning, split-policy reopening, or broader routing redesign.
