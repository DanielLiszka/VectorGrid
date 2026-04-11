# options_gpu_finalist_boundary_direction_FINAL

## Executive summary

The repo should **not** try to make finalist validation “GPU everywhere” next.

The right next move is a **narrow, measurement-gated boundary reduction**:

1. **Split the current CPU finalist exactify bucket** into:
   - retained-finalist non-extreme summary-lane recomputation
   - retained-finalist reporting enrichment (especially exact median fields)
   - retained-finalist exact `extreme` replay
2. **Only if the non-extreme summary portion is still materially large**, add a **finalist-only GPU rescore** for the already-compiled baseline / holdout / conservative / harsher lanes **after `top_k` truncation**.
3. Keep **sealed ordering, verdict derivation, median enrichment, and all exact `extreme` replay on CPU in the first wave**.

That is the best fit for the current repo because the attached evidence says the removable cost is **not** “all finalist work,” and the largest removable slice is **exactify**, not sealed finalize. On the attached SPY gate artifact, the finalist CPU tail is modest; on the attached QQQ gate artifact, `gpu_exactify_us` is essentially the same size as `gpu_tensor_score_us`, while `gpu_sealed_stage_us` stays much smaller. The next implementation dollar belongs in the **summary-compatible exactify slice**, not in a full-device finalist rewrite.

## Current GPU vs CPU boundary

### What the repo already does on GPU

The current shipped route already performs broad tuple evaluation on GPU for:

- search-lane ranking aggregates
- holdout-lane ranking aggregates
- conservative holdout stress aggregates
- harsher holdout stress aggregates

That boundary is implemented in `crates/app_core/src/options_expression.rs` and `crates/app_core/src/options_expression_gpu.rs`.

At a high level:

- `gpu_scoring::rank_compiled_tuples_gpu(...)` launches the broad GPU path.
- `score_options_expression_tensors_gpu_multi_range(...)` and `score_options_expression_tensors_gpu_range(...)` provide the device scoring entrypoints.
- the GPU path returns compact tuple aggregates through `OptionsGpuTupleAggregate`.
- after ranking, the repo truncates to `req.top_k.max(1)`.

### What still returns to CPU

After broad GPU ranking, the retained tuples still pass through CPU-side finalist work:

- `finalize_ranked_results(...)`
- `exactify_ranked_results(...)`
- `apply_sealed_finalist_stage(...)`
- `evaluate_exact_stress_for_split(...)` for exact `extreme` stress

This is the current hybrid boundary.

### Why the boundary still exists

The current broad GPU aggregate is intentionally thin. `OptionsGpuTupleAggregate` only carries:

- `valid_trades`
- `wins`
- `total_pnl`
- `total_premium`

That is enough to rebuild ranking-critical metrics such as expectancy, invalid rate, win rate, and total PnL, but it is **not** enough to recover exact `median_return_on_premium`. The current GPU reconstruction path therefore cannot fully replace the CPU exact path for final retained-tuple reporting semantics.

### What the attached timing evidence says

The attached artifacts show that the remaining CPU finalist boundary is real, but uneven:

| Attached artifact | `gpu_tensor_score_us` | `gpu_exactify_us` | `gpu_sealed_stage_us` | Takeaway |
| --- | ---: | ---: | ---: | --- |
| SPY DTE7 gate (`2026-04-07T20-41-53`) | 1190 | 232 | 101 | CPU finalist tail exists, but is modest |
| SPY cached compare (`spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features`) | 1206 | 62 | 18 | On the best warm SPY shape, the tail is very small |
| QQQ DTE7 gate (`2026-04-08T09-50-23`) | 1731 | 1765 | 242 | Exactify becomes first-order; sealed stage does not |

That timing pattern is the key reason the recommendation is **narrower than “move finalist validation to GPU.”**

## Final recommendation

### Adopted direction

Adopt a **two-step, measurement-gated retained-finalist boundary reduction**:

#### Step A — split the current exactify bucket before moving the boundary

Refactor the current finalist exactify path so the repo can separately measure:

- retained-finalist non-extreme summary-lane recomputation
- retained-finalist reporting enrichment, especially exact median fields
- retained-finalist exact `extreme` replay

The current `gpu_exactify_us` bucket is too coarse. Without this split, the repo risks “optimizing exactify” while mostly moving around CPU work that either cannot or should not leave the host.

#### Step B — if justified, offload only the summary-compatible retained-finalist slice

If Step A shows that the non-extreme summary portion is materially large on the canonical workloads, add a **finalist-only GPU rescore** for retained tuples after `top_k` truncation.

That finalist-only GPU rescore should cover only the already-compiled summary lanes used by `exactify_ranked_results(...)`:

- search baseline
- holdout baseline
- holdout conservative stress
- holdout harsher stress

It should **not** move these in the first wave:

- sealed-finalist stage ordering logic
- sealed baseline/conservative/harsher recomputation
- exact `extreme` stress replay
- verdict derivation
- exact median enrichment

### Why this wins

#### Why it beats keeping the current hybrid boundary unchanged

Doing nothing is defensible on the warm SPY slice, but the attached QQQ gate artifact shows that `gpu_exactify_us` can be first-order and roughly equal to tensor-score time. That makes the boundary worth revisiting.

#### Why it beats a more aggressive full-device finalist design

A near-full-device finalist rewrite would push the repo toward exactly the kind of irregular replay ownership it has been trying to keep on CPU. It would also make semantic parity and debugging harder while attacking the wrong current cost center: sealed finalize is not the dominant slice in the attached evidence.

#### Why it beats moving sealed logic first

The timing evidence says the largest removable slice is **exactify**, not sealed stage. On QQQ, `gpu_sealed_stage_us` is only 242 µs while `gpu_exactify_us` is 1765 µs. So the next work should target the exactify boundary, not sealed-stage ordering.

### Practical implementation shape

The first-wave implementation should treat retained-finalist work as three different classes:

1. **Summary-compatible ranking-critical metrics**
   - good GPU candidate
   - exact from small aggregate state
   - already compiled into dense tensors
2. **Reporting enrichment**
   - especially exact `median_return_on_premium`
   - keep on CPU first
3. **Irregular replay / exact oracle work**
   - exact `extreme` stress
   - keep on CPU

That keeps dense work on GPU and branchy work on CPU, which is consistent with the repo’s core design doctrine.

## What remains unchanged

The following should stay unchanged in the first wave:

- the current artifact-driven options workflow
- the current parent-family scope
- the current structure-family scope
- CPU fallback and CPU-oracle behavior
- the current `top_k` containment boundary
- the current `sealed_finalist_count` containment boundary
- broad GPU ranking over search / holdout / conservative / harsher lanes
- exact `extreme` stress staying CPU-side
- sealed-finalist ordering and verdict derivation staying CPU-side in wave one
- the current frontend workflow placement and command boundary

The containment model is already correct. The change is to do less CPU work **inside** that model, not to widen the model.

## Deferred ideas

The following ideas should be deferred rather than implemented now:

1. **GPU sealed-stage rescore for baseline / conservative / harsher sealed lanes**
   - only revisit after the non-extreme exactify split proves worth moving
2. **Approximate or sketch-based median-on-device work**
   - not needed for the first wave
   - likely better handled later under a separate low-state-metric review
3. **CUDA runtime micro-architecture or SASS work for this lane**
   - only revisit if the finalist boundary stops being the real bottleneck after the split
4. **Any change to `top_k` or `sealed_finalist_count` as the main performance lever**
   - those are product containment knobs, not the right first fix here

## Rejected ideas

The following directions should be rejected for now:

1. **Near-full-device finalist validation**
   - especially anything that tries to push raw path replay or exact `extreme` evaluation onto GPU
2. **Broad-pass device-state widening as the main mechanism**
   - do not bloat the wide GPU ranking contract just to improve retained-finalist work
3. **Changing finalist budgets to hide CPU cost**
   - that would be a false performance win
4. **A new workflow stage for options validation**
   - out of scope
5. **“GPU everywhere” as a principle**
   - the repo should stay GPU-first, not GPU-dogmatic

## Concrete code touch points by file and symbol

| File | Symbols / areas | Expected change |
| --- | --- | --- |
| `crates/app_core/src/options_expression.rs` | `gpu_scoring::rank_compiled_tuples_gpu(...)` | After `top_k` truncation, optionally call a finalist-only GPU summary rescore path instead of sending all non-extreme exactify work directly to CPU |
| `crates/app_core/src/options_expression.rs` | `finalize_ranked_results(...)` | Split orchestration into summary-compatible exactify, CPU enrichment, exact `extreme`, and sealed-stage finalize |
| `crates/app_core/src/options_expression.rs` | `exactify_ranked_results(...)` | Break apart current CPU bundle into smaller helpers so the removable non-extreme slice is explicit |
| `crates/app_core/src/options_expression.rs` | `apply_sealed_finalist_stage(...)` | Leave CPU-owned in wave one; only minor interface adjustments if timing or staging changes |
| `crates/app_core/src/options_expression.rs` | `evaluate_compiled_tuple_for_split(...)` | Keep as CPU oracle / enrichment path; may become the source for median-only or CPU-enrichment helpers |
| `crates/app_core/src/options_expression.rs` | `evaluate_exact_stress_for_split(...)` | Keep CPU-owned and untouched except for clearer timing boundaries |
| `crates/app_core/src/options_expression.rs` | `metrics_from_aggregate(...)` | Keep broad-path role; consider a retained-finalist-specific variant only if parity requires stricter aggregate output types |
| `crates/app_core/src/options_expression_gpu.rs` | `score_options_expression_tensors_gpu_range(...)` | Basis for retained-finalist GPU summary rescore |
| `crates/app_core/src/options_expression_gpu.rs` | `score_options_expression_tensors_gpu_multi_range(...)` | Basis for retained-finalist multi-lane summary rescore |
| `crates/app_core/src/options_expression_gpu.rs` | `OptionsGpuTupleAggregate` / output-buffer path | If needed for retained-finalist parity, add a finalist-only stricter aggregate path without widening the broad ranking contract |
| `crates/app_core/src/options_types.rs` | `OptionsExpressionFeasibilityTimingStats` | Add finer-grained finalist timing spans so summary-exactify vs CPU enrichment vs exact `extreme` are reported separately |
| `src-tauri/src/commands_options_expression.rs` | feasibility/report plumbing | Propagate any new timing fields into preview/headless reporting without changing workflow shape |

### Recommended helper split in `options_expression.rs`

The current code likely wants an explicit separation such as:

- `exactify_ranked_results_summary_lanes_*`
- `enrich_ranked_results_reporting_fields_*`
- `exactify_ranked_results_extreme_only_*`

The exact names can vary, but the boundary should become explicit in code.

## Benchmark and parity plan

### Phase 0 — instrumentation-only split

Before moving any work, split the current exactify timing into:

- non-extreme retained-finalist summary exactify
- retained-finalist CPU reporting enrichment
- retained-finalist exact `extreme` replay

Run this on:

- attached SPY DTE7 gate workload
- attached QQQ DTE7 gate workload
- attached SPY cached compare workload

Goal:

- confirm how much of today’s `gpu_exactify_us` is actually summary-compatible and worth moving

### Phase 1 — finalist-only GPU summary prototype

Prototype a retained-finalist GPU rescore for only:

- search baseline
- holdout baseline
- holdout conservative stress
- holdout harsher stress

Keep CPU for:

- median enrichment
- exact `extreme`
- sealed stage

### Phase 2 — strict parity validation

For retained finalists only, require parity against the current CPU oracle for:

- retained tuple identities
- pre-sealed ordering
- `selection_metrics` except any explicitly deferred reporting-only field during the GPU rescore stage
- `holdout_metrics`
- `stress_metrics`
- `harsher_stress_metrics`
- `robust_score`
- `validation_verdict`
- final artifact leaders after CPU enrichment is complete

`median_return_on_premium` must still match the current CPU artifact output after the CPU enrichment step.

### Phase 3 — performance validation

Measure at least:

- old vs new retained-finalist summary-exactify time
- old vs new total finalist CPU time
- old vs new hybrid ranking time
- warm end-to-end preview wall time
- warm end-to-end full run wall time

Success should be judged primarily on the bounded QQQ workload, with canonical SPY as the “do no harm” control.

### Minimum success bar

Treat the change as worthwhile only if all of these are true:

- the summary-compatible retained-finalist CPU slice drops materially on QQQ-like workloads
- warm end-to-end preview or run wall time improves measurably
- SPY does not regress meaningfully
- no product-facing result semantics drift

## Regression risks

1. **Semantic drift from broad-vs-finalist aggregate differences**
   - especially if retained-finalist GPU output uses less precise state than CPU exactify
2. **Hidden dependence on reporting-only fields**
   - especially `median_return_on_premium`
3. **Extra launch overhead dominating small workloads**
   - especially on the warm SPY slice where the current finalist tail can already be tiny
4. **Memory or planner regression**
   - if the finalist pass allocates wider output state than necessary
5. **Code-boundary confusion**
   - if exactify, enrichment, extreme, and sealed finalize remain tangled after refactor
6. **Fake wins from bucket reshuffling**
   - if microsecond timing improves but real preview/run wall time does not

## Stop conditions

Keep the current hybrid boundary if any of the following happen:

1. **Instrumentation shows the removable summary-compatible slice is too small to matter**
   - especially if it stays small on both canonical SPY and bounded QQQ
2. **The prototype changes product behavior**
   - retained tuple identities
   - pre-sealed rank order
   - sealed finalist membership or final ranks
   - validation verdicts
   - final reported metrics after CPU enrichment
3. **The change requires widening the broad GPU ranking contract**
   - this would make the fix worse than the current boundary
4. **Warm end-to-end improvement is negligible**
   - especially if QQQ-like preview/run wall time does not improve clearly
5. **SPY regresses more than it improves**
   - especially on the canonical warm slice
6. **The implementation only helps by shrinking `top_k` or `sealed_finalist_count`**
   - that is not an acceptable primary fix

If any of those conditions hold, the repo should keep the current hybrid boundary and treat the remaining finalist CPU work as acceptable product cost.
