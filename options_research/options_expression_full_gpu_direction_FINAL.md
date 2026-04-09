# Options Expression Full-GPU Direction — Final

Date: 2026-04-08  
Status: final consolidation for Prompt 1 (`Next Full-GPU Productization Direction`)

Implementation guide for the validated portions of this memo:

- [options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md)

## 1. Executive summary

The next implementation step should **not** be more GPU planner work, more CUDA-runtime work, more SASS/PTX work, or broader realism/scope expansion.

The best next move is:

**narrow overlap-aware coverage-quality hardening at the replay/compile boundary, plus minimal default-profile clamps inside the current aggregate lane.**

In practice, that means:

1. **Detect and surface parent-vs-options overlap weakness before scoring** so the system stops treating bad slice alignment as a GPU problem.
2. **Use the current invalid-reason telemetry to tighten the current default aggregate profile only where the evidence already says it is weak**, especially around late-entry failure and marginal strike/moneyness coverage.
3. **Leave the GPU scorer, planner, runtime chunking/buffer reuse, artifact workflow, and current scope freeze largely unchanged.**

This is the smallest change that attacks the real remaining product risk:

- the GPU scorer is already real
- the runtime path is already viable
- the current lane already clears live GPU gates on representative bounded slices
- but compiled coverage is still fragile enough that promotion quality depends too much on slice curation and overlap luck

So the repo should now **improve workload quality before widening computation**.

---

## 2. Current repo evidence summary

### 2.1 What is already shipped

The repo already has a real artifact-driven options-expression workflow, not a paper design. The shipped baseline includes:

- artifact-driven options follow-up over saved directional artifacts
- supported parent families:
  - `DoubleMa`
  - `CompositeTrend`
  - `RegimeHybrid`
  - `BreakoutMomentum`
- supported structure families:
  - `long_single`
  - `debit_vertical`
  - `long_pair`
- bounded Polygon/Massive `1m` aggregate provider download
- quote-aware and derived-mid CSV import lanes
- CPU reference scoring
- CUDA compiled-tensor scoring
- holdout / sealed-finalist / multi-lane robustness metrics
- saved artifacts, compare surfaces, tuple drilldown, and event-path replay

The current future roadmap is also explicit that scope should deepen **inside the current workflow**, not widen into more parent families, more structure families, or a general options platform. The roadmap specifically keeps additional parent families, dedicated options mode expansion, and direct quote-aware provider expansion out of priority scope. It also states that the default direction is to improve optimization and embedded validation quality without destroying the performance model.

### 2.2 What current code already solved

The current live backend policy is already much more permissive than the earlier conservative experiment-only posture. In `options_expression.rs`, the runtime `auto` path is currently keyed to:

- minimum candidate density: `25%`
- maximum invalid-slot rate: `75%`
- minimum compiled tuples: `256`
- minimum search events: `8`
- minimum projected event-tuples: `10,000`
- measured promotion-speed floor: `1.25x` when measured-speedup gating is used

At the same time, the current backend already includes:

- cached structural candidate compilation
- cached exit summaries
- prebuilt flat GPU score tensors during prepare
- shared-input cache reuse
- outcome-input cache reuse
- reusable tuple-sized output buffers
- free-memory-aware GPU chunk planning
- gate-driven `auto`
- replay/input caching around parent-signal and imported options datasets

So the repo is not waiting on first-wave planner or scorer infrastructure.

### 2.3 What the bounded provider-backed evidence says

The attached and referenced local evidence points to a clear pattern:

#### Canonical SPY slice

The authoritative SPY DTE7 provider-backed promotion evidence is a **good scorer proof**:

- candidate density: about `0.5247`
- invalid-slot rate: about `0.4753`
- repeated hybrid-ranking GPU speedup vs CPU: about `5.32x`
- invalid reasons are dominated by:
  - `entry_past_cutoff`
  - `no_strike_in_tolerance`

But as a full product proof it is still narrow:

- only `18` total events
- only `10` search events
- attached holdout candidate/outcome coverage is `0`

So SPY proves the scorer path can win, but it does **not** by itself prove robust coverage quality across representative slices.

#### Bounded QQQ slice

The bounded QQQ provider-backed rerun is the better product-risk signal:

- candidate density: about `0.2513`
- invalid-slot rate: about `0.7487`
- recommended backend: GPU
- best available backend: GPU
- hybrid-ranking GPU speedup vs CPU: about `2.17x` in the attached preview, with the gate summary reporting a stronger repeated-speedup result
- dominant invalid reasons:
  - `contract_not_eligible_as_of_event`
  - `entry_past_cutoff`
  - much smaller `no_price_row_after_entry`
  - very small `no_strike_in_tolerance`

This is the key current signal:

- the GPU path is already viable enough to route work
- the remaining fragility is mostly **coverage quality**, not scorer existence

#### End-to-end preview cost context

The timing artifacts also say the scorer is not the main remaining end-to-end problem on these slices.

Attached preview timing is dominated by replay/build, not tensor scoring:

- QQQ attached preview:
  - `replay_build_ms`: `153`
  - `total_preview_ms`: `332`
  - `gpu_tensor_score_us`: `1731`
- SPY attached preview:
  - `replay_build_ms`: `198`
  - `total_preview_ms`: `330`
  - `gpu_tensor_score_us`: `1206`

That is an important reason **not** to spend the next cycle on lower-level CUDA micro-architecture or SASS.

### 2.4 What this means

The repo has already crossed the line where the right question is no longer:

- “can the GPU scorer win?”

The real remaining question is:

- “how do we make the compiled workload consistently good enough that the full-GPU lane is product-trustworthy without broadening scope?”

That question is currently driven more by:

- overlap quality
- late-entry quality
- marginal strike/moneyness quality
- split-level coverage adequacy

than by raw CUDA throughput.

---

## 3. Final recommendation

## Adopted direction

**Implement overlap-aware coverage-quality hardening at the replay/compile boundary, then apply only minimal default-profile clamps that are already supported by the invalid telemetry.**

This direction has two parts.

### 3.1 Part A — make overlap quality a first-class preflight decision

Before or during feasibility build, explicitly compute and surface:

- parent-event coverage inside the options dataset window
- split-level event coverage inside the options dataset window
- whether search / holdout / sealed folds still have executable opportunity after dataset overlap is applied
- whether contract-reference as-of timing makes the slice structurally weak
- whether `contract_not_eligible_as_of_event` is dominating because the slice itself is mispaired, not because the compiled scorer is weak

When those signals are pathological, the system should:

- fail fast, or
- issue strong non-promotable warnings in feasibility/artifact output

instead of silently feeding a weak slice into the normal path and treating the result as a GPU-quality statement.

### 3.2 Part B — clamp only the weak default profile pieces already identified by telemetry

Inside the current aggregate lane, use the existing invalid telemetry to tighten the default profile narrowly, not to redesign the search space.

The current evidence already says the weak points are:

- late-entry / lag tolerance quality
- weak strike/moneyness profiles on some slices
- slice-overlap / eligibility mismatch

So the next intervention should be limited to things like:

- stronger late-entry quality guidance or normalization
- de-emphasizing obviously weak strike/moneyness defaults only if they improve event coverage instead of merely improving density ratios
- improving profile recommendations when the feasibility report shows chronic overlap or cutoff failure

This is **not** a call for a broad tuple-grid redesign, new axes, or realism-lane expansion.

---

## 4. Explicitly retained current strengths

These parts should stay unchanged for this pass.

### 4.1 Keep the current product boundary

Keep all of the following frozen:

- current artifact-driven workflow
- current parent-family surface
- current structure-family surface
- current History follow-up host
- current aggregate lane as the canonical hot path
- CPU fallback/oracle behavior
- bounded provider-backed download posture
- quote-aware / derived-mid as secondary realism lanes

### 4.2 Keep the current GPU scorer/runtime architecture

Do **not** make the current runtime layer the next battleground.

The existing runtime already has the right first-wave shape:

- free-memory-aware chunk planning
- shared-input cache reuse
- outcome-input cache reuse
- reusable output buffers
- current launch/chunk structure

This layer should remain stable unless and until coverage quality stops being first-order.

### 4.3 Keep the current auto/planner policy steady for now

Do **not** spend the next cycle retuning the live `auto` policy again.

The current code and current roadmap/viable-plan status already say the repo has:

- moved to gate-driven `auto`
- aligned preview/runtime `auto`
- rechecked representative SPY and bounded QQQ slices
- restored `auto` as the normal current posture where the data supports it

Changing thresholds again now would mostly risk turning weak compiled coverage into a routing “win.”

### 4.4 Keep the current GPU/CPU finalist boundary for now

Broad ranking on GPU plus retained-finalist exactification / sealed replay on CPU should remain unchanged on this pass.

That boundary is still a real later question, but the current evidence says it is not the first-order blocker yet.

---

## 5. Explicitly deferred ideas

These ideas are not wrong forever. They are just not the next move.

### 5.1 Deferred: planner / promotion hardening

Defer any fresh attempt to rework:

- density thresholds
- invalid-rate thresholds
- workload floors
- measured-speedup gates
- auto-backend messaging

unless the coverage-quality work first shows that the current slices are still being misrouted after overlap/default hardening.

### 5.2 Deferred: GPU/CPU finalist-boundary work

Defer:

- moving exactification onto GPU
- moving more sealed/finalist work onto GPU
- trying to remove the retained-finalist CPU replay boundary

until the repo proves that compiled coverage quality, not slice quality, is still the remaining product bottleneck.

### 5.3 Deferred: replay/build amortization as the primary Prompt 1 answer

End-to-end timing clearly shows replay/build is expensive, and that is a real later performance question. But for **Prompt 1**, it is not the best next **full-GPU productization direction**.

Why it is deferred rather than adopted here:

- it improves preview speed
- but it does not directly solve the main remaining product-trust problem
- the current fragility still shows up as weak compiled coverage and overlap-sensitive invalid reasons

So replay/prepare amortization belongs more naturally under the later end-to-end cost question.

### 5.4 Deferred: runtime micro-architecture and SASS/PTX investigation

Defer:

- launch-shape heroics
- output-copy micro-optimizations
- SASS/PTX work
- occupancy-driven kernel surgery

because the attached timings show the tensor scorer is already a tiny share of real preview time on the representative slices.

---

## 6. Explicitly rejected ideas

### 6.1 Rejected: broad “search-shape redesign”

Do not turn the next step into:

- a new axis family
- a big tuple-grid rewrite
- a new selector regime
- or a disguised structure-family expansion

That is too broad relative to the current evidence.

### 6.2 Rejected: planner-first or threshold-first “productization”

Do not lower thresholds, widen promotion rules, or rewrite `auto` again just to make more slices look GPU-worthy.

That would be fake progress if the underlying compiled coverage is still floor-hugging.

### 6.3 Rejected: realism/provider expansion as the next answer

Do not make the next step:

- direct quote-aware provider download
- quote-native hot-path work
- realism-lane expansion
- broader data-lane policy changes

The roadmap already keeps those secondary, and the current invalid evidence does not require them to explain the problem.

### 6.4 Rejected: declaring the lane “done” because SPY proves the scorer

SPY proves the compiled-tensor scorer can win. It does **not** prove that the full lane is fully productized across representative provider-backed slices.

### 6.5 Rejected: density cosmetics

Reject any intervention that “improves” density or invalid-slot rate by:

- removing hard cases until split coverage worsens
- shrinking the job until it barely resembles the current product path
- dropping problematic folds without surfacing why
- or changing defaults in a benchmark-specific way that does not generalize

---

## 7. Concrete code touch points by file and symbol

## 7.1 Primary implementation touch points

### `src-tauri/src/commands_options_expression.rs`

Primary symbol:

- `build_options_expression_replay_inputs(...)`

Why this is the main boundary:

- it already owns parent-artifact replay input reconstruction
- it already owns options-dataset replay input reconstruction
- it already owns parent-signal and imported-dataset caching
- it already determines the inputs that feasibility preview and real runs consume

What to add here:

- explicit overlap-quality diagnostics between:
  - parent event timestamps / folds
  - dataset coverage window
  - contract-reference as-of window
- split-level preflight counts for:
  - search / holdout / sealed events inside dataset overlap
  - overlap-starved or structurally non-promotable slices
- stronger warnings or hard failures when overlap weakness makes feasibility results misleading

Secondary symbols worth touching:

- `load_replayed_candles(...)`
- `AnalyzeOptionsExpressionFeasibilityResponse`
- any artifact/summary path that should persist the new preflight diagnostics

### `crates/app_core/src/options_expression.rs`

Primary symbols:

- `select_candidate(...)`
- `compile_structural_candidate_cache(...)`
- `assign_splits(...)`

Why these matter:

- `select_candidate(...)` is exactly where the current dominant invalid reasons are produced:
  - `ContractNotEligibleAsOfEvent`
  - `EntryPastCutoff`
  - `NoStrikeInTolerance`
- `compile_structural_candidate_cache(...)` is where structural candidate coverage is formed across the fixed tuple axis
- `assign_splits(...)` is where split policy affects whether later folds even remain meaningful once options coverage is considered

What to change:

- preserve current structural compilation
- add or expose stronger diagnostics around invalid concentration by:
  - split
  - delay bucket
  - DTE bucket
  - moneyness bucket
- optionally add narrow profile-quality clamps only if they are justified by the same invalid telemetry across the benchmark pair

What to leave steady:

- `evaluate_auto_backend_decision(...)`
- `runtime_auto_backend_decision(...)`
- the current GPU ranking integration
- the current retained-finalist exactification flow

### `crates/app_core/src/options_types.rs`

Primary symbols:

- `OptionsStaticFilterBundle`
- `OptionsCandidateInvalidReason`
- `OptionsCandidateInvalidReasonCount`
- `OptionsExpressionFeasibilityEventCoverage`
- `OptionsExpressionFeasibilityReport`

Why these matter:

- the current repo already has most of the contract surface needed for this direction
- if one new overlap-quality or split-quality section is needed in the feasibility report, it belongs here
- if the repo wants to distinguish:
  - weak overlap
  - weak split coverage
  - weak candidate quality
  that contract boundary should be explicit here rather than inferred only in UI text

Likely additions:

- a small overlap-quality summary payload
- possibly split-level “coverage adequacy” or “overlap adequacy” fields
- no broad request-surface expansion

## 7.2 Explicit no-change touch points for this pass

### `crates/app_core/src/options_expression_gpu.rs`

Symbols to leave alone unless a benchmark later proves otherwise:

- `OptionsGpuChunkPlan::for_inputs(...)`
- `ensure_shared_inputs_chunk(...)`
- `ensure_outcome_inputs(...)`
- `ensure_output_buffers(...)`
- `score_ranges(...)`

These already reflect the current intended runtime posture and are not the first-order problem right now.

---

## 8. Benchmark and validation plan

## 8.1 Baseline benchmark set

Use a **narrow fixed matrix** first.

### Required benchmark pair

1. **Canonical SPY DTE7 provider-backed slice**
2. **Bounded QQQ DTE7 provider-backed slice**

These are the right pair because:

- SPY is the good scorer proof
- QQQ is the better fragility proof

### Optional negative control

If available from the existing viable-plan evidence, also keep one explicit bad-overlap control rerun:

- legacy-parent / short-options-window QQQ-style mismatch

That negative control is valuable because it proves the new preflight actually catches the class of failure it is supposed to catch.

## 8.2 Measurements to record before and after the change

For each slice, record:

- `candidate_density`
- `invalid_slot_rate`
- invalid-reason counts
- axis coverage by:
  - delay
  - DTE
  - moneyness
- split-level event coverage:
  - events with candidates
  - events with baseline outcomes
  - events with conservative outcomes
  - events with harsher outcomes
- `best_available_backend`
- `auto_backend_decision`
- `cpu_ranking_us`
- `gpu_ranking_us`
- `gpu_speedup_vs_cpu`
- `replay_build_ms`
- `total_preview_ms`

Run both:

- cold preview
- warm preview

so the result cannot be explained away as cache luck.

## 8.3 Acceptance criteria

Adopt this direction if the same benchmark pair shows **real coverage improvement**, not just cleaner ratios.

The preferred acceptance signals are:

1. On bounded QQQ:
   - candidate density improves by about **5 percentage points or more**, **or**
   - invalid-slot rate drops by about **5 percentage points or more**
2. On both slices:
   - `total_events_with_candidates` does not fall
   - `total_events_with_*_outcomes` does not fall
3. Where split coverage is currently pathological:
   - the new preflight flags the slice explicitly instead of silently treating it as healthy
4. Preview cost does not regress materially:
   - no large cold-preview regression
   - no meaningful warm-preview regression
5. The backend still clears the current live `auto` policy without threshold relaxation

## 8.4 Failure signals

Reject or stop early if any of the following appears:

- density improves but split-level event coverage gets worse
- invalid-slot rate improves only because the profile pruned hard but useful cases
- the change requires new gate relaxation to still route GPU
- replay/build or preview wall time regresses materially
- SPY remains good but QQQ stays floor-hugging in the same way
- overlap diagnostics show the real issue is mostly benchmark curation, not engine quality

---

## 9. Verification plan

## 9.1 Backend contract verification

Add targeted Rust coverage around the new preflight behavior:

- overlap-quality diagnostics
- split-level overlap adequacy
- non-promotable overlap cases
- persistence/serialization of any new feasibility report fields

Primary verification target files:

- `src-tauri/src/commands_options_expression.rs`
- `crates/app_core/src/options_types.rs`

## 9.2 Feasibility/report verification

Verify that feasibility preview and saved artifacts now make these distinctions explicit:

- healthy slice
- weak overlap
- split-starved slice
- chronically late-entry/lag-weak slice
- strike-window weak slice

The result should be visible without forcing the user to infer it from raw invalid reasons alone.

## 9.3 Regression verification

The following must remain stable:

- CPU reference route
- GPU scoring route
- current retained-finalist exactification
- current artifact shape except for explicitly added diagnostic fields
- current aggregate-lane compatibility behavior
- current GPU runtime telemetry

## 9.4 Benchmark verification order

Recommended order:

1. run the fixed SPY/QQQ benchmark pair cold
2. run the pair warm
3. compare density / invalid / split-coverage deltas
4. compare preview/runtime deltas
5. only if coverage is now materially healthier, consider any later planner or GPU-boundary work

## 9.5 What not to do during verification

Do not:

- move thresholds during the same evaluation
- change the benchmark slices during the same evaluation
- mix this change with quote-aware realism changes
- mix this change with GPU runtime micro-architecture work
- claim success from one slice only

---

## 10. Rejection criteria and stop conditions

## 10.1 Adopt

Adopt this direction if:

- the benchmark pair shows real coverage improvement
- overlap-pathology slices become clearly diagnosable
- the repo does not need fresh planner/gate relaxation
- the current GPU scorer remains the best available backend on the same representative slices

## 10.2 Defer

Defer this direction if:

- a slightly broader provider-backed matrix shows current defaults are already comfortably robust
- or the overlap diagnostics prove most current pain is benchmark curation, not code behavior

In that case, the next question shifts away from coverage hardening and toward:

- end-to-end replay/build amortization
- or the GPU/CPU finalist boundary

## 10.3 Reject

Reject this direction if:

- it turns into broad tuple-grid redesign
- it requires scope expansion
- it mainly improves ratios while hurting event coverage
- it materially worsens preview time
- or it simply repackages planner/promotion changes as “coverage work”

---

## Short final verdict

The compiled-tensor GPU scorer is already good enough to stop being the main question.

The next full-GPU productization step should therefore be:

**make slice quality and compiled coverage quality explicit and harder to misuse, then tighten only the weak default aggregate profile pieces already exposed by the invalid telemetry.**

That is the highest-ROI next move that stays faithful to the current VectorGrid options lane.

---

## Source basis

Primary source set used for this consolidation:

- `options_expression_gpt54_pro_prompt_pack_2026-04-08.md`
- `options_expression_future_scope_roadmap_2026-04-07.md`
- `options_expression_optimizer_viable_plan_2026-04-02.md`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_options_expression.rs`
- `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`
- `output/options_expression_phase05_gpu_gate/2026-04-08T09-50-23/2026-04-08T09-50-23/phase05_summary.json`
