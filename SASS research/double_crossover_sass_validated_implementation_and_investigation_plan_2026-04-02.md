# `double_crossover` SASS-validated implementation and investigation plan

Date: 2026-04-02

## Purpose

This document converts the validated parts of `double_crossover_performance_findings_and_code_suggestions.md` into a repo-specific implementation and investigation plan.

It is intentionally narrower than a general CUDA optimization roadmap. It focuses on findings that were confirmed against:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `CUDA kernel review/double_crossover_sm89_cuobjdump_2026-04-01.sass.txt`
- `CUDA kernel review/double_crossover_sm89_cuobjdump_symbols_2026-04-01.txt`

The optional section at the end covers lower-confidence or still-unproven ideas that may still be worth exploring after the confirmed work is closed.

## Scope

This plan covers:

- exact-wrapper specialization and stride handling for the `m17` / `m22` `double_crossover` families
- row-reducer stride assumptions vs padded exact rows
- runtime selection between row-first and experimental score-first top-k flows
- pair-path score-first vs fake-scored selector ambiguity
- optional late-stage follow-up on copy helpers, `m17` fixed-row packing, and deeper runtime profiling

This plan does not authorize:

- broad PTX or SASS surgery
- user-facing feature removal
- removing CPU fallback
- changing benchmark stories without parity and perf evidence

## Evidence status

### Confirmed

These are grounded directly in current source and generated SASS:

1. The non-windowed exact `m17` / `m22` wrappers and lockstep exact `m17` / `m22` wrappers discard `metrics_out_stride` and call the body without fixed-width specialization in the generic cases.
2. `windowed_m22` already uses a specialized `FIXED_M = 22` body and honors `metrics_out_stride`.
3. Generic `windowed_m17` does not mirror `windowed_m22`; it still uses the unspecialized body.
4. `store_metrics_row_fixed_f32` has a packed path for `N == 22` only; `N == 17` falls through to scalar writes.
5. The row-reducer kernels currently assume dense `M` stride, not `metrics_stride`.
6. `select_topk_scored_pairs_tm_f32` and `_v2` are not truly pre-scored selectors; they still compute `objective_score_f32(...)` from metrics.
7. A true score-array pair path already exists via:
   - `score_metric_rows_f32`
   - `select_topk_scored_candidates_tm_f32`
   - `gather_topk_scored_scalar_pairs_tm_f32`
8. The runtime still has a row-first global top-k path as the default/canonical route.
9. The runtime also has an experimental score-first global top-k branch, but it is env-gated and blocked for many request shapes.

### Confirmed with qualification

These are reasonable and repo-grounded, but still need runtime evidence before they should be treated as the main bottleneck:

1. The wrapper specialization and stride mismatch are likely real performance costs.
2. The row-first reducer path likely duplicates score work relative to a score-once path.
3. The pair gather path is likely output-heavy enough to justify keeping scalar-score routes compact as long as possible.

### Not yet first-class evidence

These are still worth revisiting later, but they should not drive the first implementation wave:

1. exact instruction-count tables that depend on artifacts not stored in the repo
2. any claim that a specific wrapper family is definitely the hottest in live workloads
3. any conclusion that the bottleneck has already shifted from backtest body to reducers without Nsight evidence

## Success criteria

The confirmed work is successful if it produces all of the following:

1. The exact `m17` / `m22` wrapper families have a coherent specialization and stride contract.
2. Padded exact rows can be consumed safely because every row consumer understands `metrics_stride`.
3. The code clearly distinguishes row-first top-k from true score-first top-k.
4. The code clearly distinguishes fake-scored pair selectors from true score-array pair selection.
5. SASS rechecks show the expected specialization effects on the targeted wrappers.
6. GPU perf gates or targeted benchmarks show that the changes are neutral or beneficial on the benchmark-critical `double_ma` surfaces.

## Guardrails and invariants

1. Keep CPU fallback behavior unchanged.
2. Do not silently change metric semantics or output ordering.
3. Do not enable padded exact rows until row readers are made stride-aware.
4. Do not promote score-first top-k as the default path without parity and benchmark evidence.
5. Do not remove the existing row-first path until the score-first path is proven on the real `double_ma` surfaces.
6. Do not infer live bottlenecks from static SASS alone.
7. Serialize benchmarks and profiling runs.
8. Run GPU perf gates for CUDA-kernel changes when GPU is available.

## Implementation and investigation phases

## Phase 0: Baseline and proof surfaces

Objective: lock the baseline before changing the CUDA contracts.

Tasks:

- Record the current wrapper contract for:
  - `double_cross_backtest_tm_flex_f32_m17*`
  - `double_cross_backtest_tm_flex_f32_m22*`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m17*`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m22*`
  - generic `double_cross_backtest_tm_flex_f32_windowed_m17`
  - generic `double_cross_backtest_tm_flex_f32_windowed_m22`
- Record the current SASS store-width counts for representative wrappers:
  - `double_cross_backtest_tm_flex_f32_m17`
  - `double_cross_backtest_tm_flex_f32_m22`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m17`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m22`
  - `double_cross_backtest_tm_flex_f32_windowed_m17`
  - `double_cross_backtest_tm_flex_f32_windowed_m22`
- Record the current route diagnostics for:
  - row-first global top-k
  - score-first global top-k, if explicitly enabled
- Record which benchmark surfaces will be used as the primary truth set.

Recommended benchmark surfaces:

- `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs`
- shortlist-heavy `xlarge`
- one compact valid-pair route that exercises `metrics_pad8_enabled` and `tile_metrics_stride`

Deliverable:

- a short baseline note or append-only run log with:
  - branch / commit
  - env flags
  - metrics contract
  - route diagnostics
  - benchmark surfaces

## Phase 1: Normalize exact-wrapper specialization and stride contracts

Objective: make the exact wrapper families internally consistent before changing reducers.

Target areas:

- non-windowed `m17` exact wrappers
- non-windowed `m22` exact wrappers
- lockstep `m17` exact wrappers
- lockstep `m22` exact wrappers
- generic `windowed_m17`

Required changes:

1. Stop discarding `metrics_out_stride` in the targeted exact wrappers.
2. Where the wrapper hardcodes `M = 17` or `M = 22`, instantiate the body with the corresponding fixed-width specialization rather than the generic form.
3. Keep `windowed_m22` as the reference shape.
4. Make `windowed_m17` consistent with the specialized intent of the exact-family wrappers.
5. Prefer a small macro or helper pattern so the specialization contract is declared once rather than hand-edited across many wrappers.

Implementation note:

- This phase is structural. It should not by itself enable padded exact rows globally if the reducers still assume dense `M` stride.

Exit condition:

- every targeted exact wrapper either:
  - intentionally stays generic with a documented reason, or
  - honors `metrics_out_stride` and uses the intended fixed-width specialization

## Phase 2: Make row consumers stride-aware

Objective: remove the dense-`M` row-reader assumption before padded exact rows are treated as canonical.

Target kernels:

- `score_metric_rows_f32`
- `select_topk_metric_rows_f32`
- `select_topk_metric_rows_from_candidates_f32`
- `select_topk_metric_rows_f32_v2`
- `select_topk_metric_rows_from_candidates_f32_v2`
- `gather_topk_metric_rows_f32`
- `pack_metric_rows_f32`

Required changes:

1. Add a `metrics_stride` parameter to every row kernel that reads `metrics_rows`.
2. Use `metrics_stride > 0 ? metrics_stride : M` semantics inside the kernels.
3. Preserve dense-row behavior for callers that still pass `metrics_stride == M`.
4. Update the Rust launch sites to pass the current `tile_metrics_stride`.
5. Re-check any host-side assumptions that copy or decode top-k metrics rows.

Critical guardrail:

- Do not enable padded exact rows on a route unless every row consumer on that route is stride-aware.

Exit condition:

- no row kernel in the `double_ma` path computes row bases as `row * M` unless it explicitly operates on a dense buffer by design

## Phase 3: Clarify the pair-path score-first contract

Objective: stop conflating fake-scored pair selection with true score-array pair selection.

Confirmed current state:

- `select_topk_scored_pairs_tm_f32*` still compute objective scores from metrics inside the selector.
- `select_topk_scored_candidates_tm_f32` is the true pre-scored selector.
- `gather_topk_scored_scalar_pairs_tm_f32` is the true scalar-score gather.

Required work:

1. Audit the current pair-path call sites and document which routes really need:
   - `(fast, slow, full metrics row)`
   - `(fast, slow, score)` only
2. Rename or comment the fake-scored pair selectors so future contributors do not misread them.
3. For routes that only need `(fast, slow, score)`, prefer the true score-array path.
4. Keep full-metric pair gather only where the route really consumes the metrics immediately.

Exit condition:

- the code and docs clearly distinguish:
  - score-from-metrics selectors
  - true pre-scored selectors

## Phase 4: Score-first row top-k investigation and possible implementation

Objective: improve or close the gap between the default row-first global-topk path and the existing experimental score-first branch.

Important current state:

- score-first global top-k already exists in the runtime
- it is not the default
- it is blocked by many request shapes:
  - extra shortlist heads
  - shortlist confidence
  - shortlist union
  - shortlist slicing
  - constraints
  - eval range
  - precomputed MA paths
  - non-full metrics contracts
  - heatmap

Investigation questions:

1. Is the current experimental score-first path missing only reducer plumbing, or does it still need a better exact search-stage contract?
2. Would adding a constrained score-generation kernel materially simplify the current row-first `v2` selector path?
3. Which blockers are essential and which are just implementation gaps?

Possible implementation path:

1. Add a constrained score-generation kernel for rows, or equivalent score-once support.
2. Use scored-row selection for the narrow exact surfaces where the blockers are absent.
3. Keep the row-first path as the fallback and baseline.
4. Measure score-first only on request shapes it is actually designed to support.

Exit condition:

- either:
  - the score-first row path is measurably useful on narrow exact surfaces, or
  - the repo has enough evidence to stop prioritizing it

## Phase 5: Optional fixed-width output helpers after contract cleanup

Objective: only after Phases 1 and 2, consider the bounded output-path helpers that are still plausible but not required for correctness.

Candidate work:

1. Add a dedicated `N == 17` packed store path in `store_metrics_row_fixed_f32`.
2. Add fixed-width copy helpers for `m17` / `m22` row copies in:
   - `gather_topk_metric_rows_f32`
   - `pack_metric_rows_f32`
   - any equivalent gather helper that remains hot after Phases 1 through 4

Why optional:

- these ideas are plausible and consistent with the validated findings
- but they are less fundamental than fixing specialization and stride contracts

Exit condition:

- only pursue this phase if the earlier phases leave a measurable output-side cost worth attacking

## Phase 6: Verification, parity, and benchmark closure

Objective: prove the contract changes did not break semantics and decide whether they are worth keeping.

Required checks:

1. CPU/GPU parity on representative `double_ma` exact surfaces.
2. `full17` and `trade22` output contract checks where applicable.
3. Route diagnostics confirm the expected path:
   - row-first global top-k
   - score-first global top-k when intentionally enabled
4. SASS spot checks for the exact wrappers after specialization changes.
5. GPU perf gate on candidate mode.
6. One focused benchmark pass on:
   - `large/58300pairs`
   - shortlist-heavy `xlarge`
   - a padded-row route if enabled

Decision rule:

- keep only changes that are parity-safe and neutral-to-positive on the benchmark-critical surfaces

## Extensive checklist

### A. Repo safety and setup

- [ ] Read the current guide in `SASS research`.
- [ ] Reconfirm the validated findings against current source before editing.
- [ ] Ensure no other benchmark or profiling run is active.
- [ ] Record the baseline env flags for any score-first or stride experiments.
- [ ] Decide the benchmark surfaces that will be treated as the truth set.

### B. Exact-wrapper audit

- [ ] Audit generic `double_cross_backtest_tm_flex_f32_m17`.
- [ ] Audit generic `double_cross_backtest_tm_flex_f32_m22`.
- [ ] Audit non-windowed `m17_*` regime variants.
- [ ] Audit non-windowed `m22_*` regime variants.
- [ ] Audit generic `double_cross_backtest_tm_flex_f32_lockstep_time_m17`.
- [ ] Audit generic `double_cross_backtest_tm_flex_f32_lockstep_time_m22`.
- [ ] Audit lockstep `m17_*` regime variants.
- [ ] Audit lockstep `m22_*` regime variants.
- [ ] Audit generic `double_cross_backtest_tm_flex_f32_windowed_m17`.
- [ ] Confirm `double_cross_backtest_tm_flex_f32_windowed_m22` as the reference pattern.
- [ ] Document which wrappers still discard `metrics_out_stride`.
- [ ] Document which wrappers still omit fixed-width specialization.
- [ ] Decide whether to use a helper macro or helper template for wrapper normalization.

### C. Exact-wrapper implementation

- [ ] Update non-windowed `m17` wrappers to honor `metrics_out_stride`.
- [ ] Update non-windowed `m22` wrappers to honor `metrics_out_stride`.
- [ ] Update lockstep `m17` wrappers to honor `metrics_out_stride`.
- [ ] Update lockstep `m22` wrappers to honor `metrics_out_stride`.
- [ ] Update generic `windowed_m17` to use the intended specialization contract.
- [ ] Keep `windowed_m22` behavior unchanged unless a correctness issue is found.
- [ ] Rebuild the cubin after wrapper changes.
- [ ] Re-check targeted SASS symbols for store-width changes.

### D. Row-kernel stride audit

- [ ] Audit `score_metric_rows_f32` for dense `M` indexing.
- [ ] Audit `select_topk_metric_rows_f32` for dense `M` indexing.
- [ ] Audit `select_topk_metric_rows_from_candidates_f32` for dense `M` indexing.
- [ ] Audit `select_topk_metric_rows_f32_v2` for dense `M` indexing.
- [ ] Audit `select_topk_metric_rows_from_candidates_f32_v2` for dense `M` indexing.
- [ ] Audit `gather_topk_metric_rows_f32` for dense `M` indexing.
- [ ] Audit `pack_metric_rows_f32` for dense `M` indexing.
- [ ] Check whether any adjacent helper kernels read metric rows with the same dense-row assumption.

### E. Row-kernel stride implementation

- [ ] Add `metrics_stride` to `score_metric_rows_f32`.
- [ ] Add `metrics_stride` to `select_topk_metric_rows_f32`.
- [ ] Add `metrics_stride` to `select_topk_metric_rows_from_candidates_f32`.
- [ ] Add `metrics_stride` to `select_topk_metric_rows_f32_v2`.
- [ ] Add `metrics_stride` to `select_topk_metric_rows_from_candidates_f32_v2`.
- [ ] Add `metrics_stride` to `gather_topk_metric_rows_f32`.
- [ ] Add `metrics_stride` to `pack_metric_rows_f32`.
- [ ] Update Rust launch plumbing for every changed row kernel.
- [ ] Verify dense-row callers still work.
- [ ] Verify padded-row callers now work coherently.

### F. Runtime contract audit

- [ ] Reconfirm `effective_metrics_stride(...)` behavior.
- [ ] Reconfirm `kernel_metrics_pad8_enabled_for_pair(...)` behavior.
- [ ] Reconfirm where `tile_metrics_stride` is computed and passed.
- [ ] Reconfirm where row-first global top-k remains the default path.
- [ ] Reconfirm where score-first global top-k is enabled and blocked.
- [ ] Reconfirm which request shapes force row-first fallback.

### G. Score-first row investigation

- [ ] Re-evaluate `should_use_score_first_topk_mode(...)` blockers.
- [ ] Decide which blockers are structural versus temporary.
- [ ] Decide whether to add a constrained score-generation row kernel.
- [ ] If added, keep it behind the experimental score-first branch first.
- [ ] Benchmark score-first only on supported narrow exact surfaces.
- [ ] Compare row-first vs score-first on the same exact benchmark surface.

### H. Pair-path clarification

- [ ] Audit `select_topk_scored_pairs_tm_f32`.
- [ ] Audit `select_topk_scored_pairs_tm_f32_v2`.
- [ ] Audit `select_topk_scored_candidates_tm_f32`.
- [ ] Audit `gather_topk_scored_pairs_tm_f32`.
- [ ] Audit `gather_topk_scored_scalar_pairs_tm_f32`.
- [ ] Identify routes that only need `(fast, slow, score)`.
- [ ] Identify routes that immediately need full metric rows.
- [ ] Rename or document misleading fake-scored selector functions.
- [ ] Prefer the true score-array path where metrics are not immediately needed.

### I. Optional output-helper work

- [ ] Decide whether `N == 17` packed stores are still worth adding after wrapper normalization.
- [ ] If yes, add a dedicated `N == 17` packed store path.
- [ ] Decide whether fixed-width copy helpers are still worth adding after earlier phases.
- [ ] If yes, add copy helpers for `m17` / `m22`.
- [ ] Use those helpers only where aligned, fixed-width copies are actually plausible and measurable.

### J. Static verification

- [ ] Re-run SASS generation after changes.
- [ ] Re-check `m17` targeted store counts.
- [ ] Re-check `m22` targeted store counts.
- [ ] Re-check `lockstep_time_m17` targeted store counts.
- [ ] Re-check `lockstep_time_m22` targeted store counts.
- [ ] Re-check `windowed_m17` targeted store counts.
- [ ] Re-check `windowed_m22` targeted store counts.
- [ ] Save any new symbol-count summaries as repo artifacts if they are used in review docs.

### K. Functional verification

- [ ] Check compile-time CUDA symbol loading still succeeds.
- [ ] Check row-first route still executes.
- [ ] Check score-first route still executes when intentionally enabled.
- [ ] Check top-k output ordering and contents.
- [ ] Check compact-valid-pair output routes.
- [ ] Check `full17` outputs if modified surfaces touch them.
- [ ] Check `trade22` outputs if modified surfaces touch them.

### L. Benchmark and perf verification

- [ ] Run the relevant narrow benchmark surface before changes.
- [ ] Run the same benchmark surface after changes.
- [ ] Run shortlist-heavy `xlarge`.
- [ ] Run GPU perf gate in candidate mode for kernel changes.
- [ ] Compare route diagnostics to ensure the same logical path was exercised.
- [ ] Reject changes that only improve a synthetic or unsupported path.

## Optional late-stage investigations

These are allowed at the end, but they are intentionally lower priority than the validated work above.

### Optional 1: Re-derive the static instruction-count tables from stored repo artifacts

Reason:

- the earlier guide cites a counts artifact that is not present in the repo

Task:

- generate a fresh stored artifact for any numeric instruction-count tables that will be referenced in future docs

### Optional 2: Runtime/Nsight proof of where time actually goes after Phases 1 through 4

Reason:

- static SASS can justify code-shape changes, but not final bottleneck ranking

Task:

- collect Nsight Compute or equivalent evidence after the structural fixes land
- determine whether the dominant cost is:
  - backtest body
  - row reducers
  - pair gather
  - host/device orchestration

### Optional 3: Invalid-row materialization changes

Reason:

- the guide plausibly points to unnecessary invalid-row write cost
- but this is less fundamental than the contract fixes

Task:

- consider top-k-only modes that avoid carrying invalid rows forward when dense row identity is not required

### Optional 4: Retire or simplify fake-scored pair selectors

Reason:

- the names are misleading and may become dead weight once the true score-array path is adopted more broadly

Task:

- only revisit after the preferred pair-path contract is proven and benchmarked

## Recommended execution order

1. Phase 0 baseline capture
2. Phase 1 exact-wrapper normalization
3. Phase 2 row-consumer stride support
4. Phase 6 first parity and benchmark check
5. Phase 3 pair-path clarification
6. Phase 4 score-first row investigation
7. Phase 6 second parity and benchmark check
8. Phase 5 optional output helpers
9. Optional late-stage investigations

## Bottom line

The most defensible first wave is not hand-tuned PTX or broad SASS surgery.

It is:

1. fix the exact-wrapper specialization and stride contract
2. make row consumers stride-aware before padded rows are trusted
3. make the runtime distinction between row-first and score-first explicit
4. use the true score-array pair path where it is actually appropriate

That is the highest-confidence path supported by the current source and the generated `sm_89` artifacts.
