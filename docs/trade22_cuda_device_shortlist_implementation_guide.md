# Trade22 CUDA Device Shortlist Implementation Guide

Created: 2026-04-26
Status: implementation planning guide

This document scopes the next CUDA performance investigation around the
Trade22 exact metric surface. The goal is to determine whether moving more
shortlist composition, score-mode ranking, and compact decision extraction onto
the GPU produces a measurable improvement without regressing the current exact
kernel route.

## Intent

Use the existing Trade22 matrix as the canonical rich exact metric surface, then
extract more decision value from that surface before host materialization.

The target shape is:

```text
candidate path walk
  -> Trade22 metric row on device
  -> device-side constraints
  -> device-side objective heads / score heads
  -> compact device-side shortlist composition where it pays
  -> bounded D2H rows and diagnostics
```

The work is only successful if it has correctness evidence and benchmark
evidence. A faster micro-path is not enough if default routes slow down, route
fallback behavior changes, CPU-only builds break, or shortlist semantics drift.

## Current Evidence Snapshot

Trade22 production is already fused into the path walk:

- `crates/app_core/src/double_ma.rs` defines `DoubleMaMetricContract::Trade22`.
- `crates/app_core/src/gpu_backtest_kernel.rs` promotes exact rows to
  `TRADE_METRICS_COUNT = 22` when `metric_contract=Trade22` or
  `include_trade_metrics=true`.
- `crates/app_core/src/kernels/double_crossover.cu` accumulates
  `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, and
  `gross_loss` during the same candidate path walk.

The current CUDA path already does some post-row extraction on device:

- Primary top-k selection runs on device for the global top-k path.
- Metric-only constraints run on device when the v2 selector path is available.
- Extra shortlist objective heads are selected on device from the same tile
  metric rows and merged into compact per-head global buffers.

The current CUDA path still has host-side shortlist composition:

- Extra-head rows are materialized to host before final union mode reranking.
- `shortlist_union_mode` is applied in Rust through
  `rerank_shortlist_by_union_mode`.
- `shortlist_confidence_mode` is applied in Rust through
  `rerank_shortlist_by_confidence`.
- `score_mode` currently forces `device_include_all`, and advanced shortlist
  union/confidence requests reject active `score_mode`.
- `shortlist_slice_mode` performs bounded exact GPU replays over slice ranges.
- `shortlist_neighborhood_radius` currently calls local stability after
  optimization and recomputes the neighborhood surface.

## Source Truth

Primary implementation files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - host CUDA orchestration, scratch buffers, route diagnostics, top-k merging,
    shortlist materialization, final reranks, and timing telemetry.
- `crates/app_core/src/kernels/double_crossover.cu`
  - metric production kernels, selector kernels, gather kernels, and any new
    device-side score/union kernels.
- `crates/app_core/src/double_ma.rs`
  - request contract, score-mode definitions, CPU score functions, route
    validation, follow-up cache, local stability, and high-level tests.

Secondary files:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/double_ma_test_support.rs`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`

## Non-Goals

- Do not replace the Trade22 path-walk kernel.
- Do not add more fields to Trade22 unless the new field is a fixed-state
  online metric that is naturally accumulated during the bar walk.
- Do not make CUDA mandatory for normal builds or tests.
- Do not remove CPU fallback or host fallback paths.
- Do not move slice robustness into a fake metric-row calculation. Slice
  robustness needs slice-local path metrics and can continue to replay unless a
  measured alternative exists.
- Do not assume neighborhood robustness is free from current rows. The optimizer
  currently tiles the metric matrix and does not retain the full exact grid as a
  reusable resident matrix after final top-k selection.

## Non-Regression Policy

Do not make a new CUDA decision path default until all of these are true:

- CPU-only builds still pass without CUDA tooling.
- Existing `Full17` and `Trade22` metric contract tests still pass.
- Existing route diagnostics remain accurate.
- Existing default GPU top-k results are unchanged, except for documented and
  explicitly requested ranking-mode changes.
- Default GPU perf gate cases do not regress beyond their configured budgets.
- Advanced shortlist cases have measurable improvement on at least one
  representative workload.
- D2H bytes and host-finalize time do not increase on default routes.
- Any new CUDA kernel compiles to fresh PTX locally before handoff.

Performance experiments must be serialized. Before running benchmarks,
profilers, Nsight, CUPTI, or Criterion, verify no other benchmark/profiling
process is active.

## Viability Threshold

Treat this as an experiment until the data proves otherwise.

A change is viable if it meets at least one of these criteria without default
regressions:

- Reduces advanced exact-kernel optimization wall time by at least 5% on a
  workload using `Trade22`, extra shortlist objectives, and union or confidence
  reranking.
- Reduces measured host finalize time for shortlist union/confidence by at
  least 25% on the same workload.
- Reduces D2H traffic for advanced shortlist extraction by at least 25% while
  preserving output parity.

If the improvement is below those thresholds, keep the path behind an opt-in
environment flag or do not ship it.

## Phase 0: Baseline And Instrumentation

Purpose: measure whether host-side union/confidence work is large enough to
justify CUDA implementation.

Tasks:

- [ ] Build a repeatable advanced exact-kernel workload with:
  - `metric_contract=Trade22`
  - `top_k > 0`
  - `include_all=false`
  - `extra_shortlist_objectives` with two or three heads
  - each `shortlist_union_mode`
  - `shortlist_confidence_mode=psr` and `dsr`
- [ ] Capture baseline timings with `VECTORBT_KERNEL_PROFILE_PIPELINE=1`.
- [ ] Record:
  - total optimization time
  - `host_finalize_shortlist_union_ms`
  - `host_finalize_shortlist_confidence_ms`
  - `host_finalize_shortlist_slice_ms`
  - D2H copy time
  - host decode time
  - route diagnostics
  - top row identity and metrics
- [ ] Add missing telemetry only if current timing output cannot isolate the
  relevant cost.
- [ ] Save the baseline command lines and results under `output/` if the run is
  used to justify implementation.

Suggested baseline commands:

```powershell
powershell -File scripts/run_workflow_gate.ps1 -Mode candidate
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

For targeted local runs, keep environment overrides explicit:

```powershell
$env:VECTORBT_KERNEL_PROFILE_PIPELINE="1"
$env:VECTORBT_KERNEL_GLOBAL_TOPK="1"
$env:VECTORBT_KERNEL_FORCE_LEGACY_TOPK="0"
```

Acceptance:

- [ ] A clear baseline exists before kernel changes.
- [ ] The measured host-side shortlist cost is large enough to justify CUDA
  work, or the effort stops here.

## Phase 1: Preserve Current Device Multi-Head Extraction

Purpose: make sure future changes do not weaken what already works.

Tasks:

- [ ] Add or tighten tests proving extra shortlist heads still select on the
  exact global top-k kernel path.
- [ ] Assert that requests with `extra_shortlist_objectives` still reject
  incompatible host-filter, export, include-all, and active score-mode paths.
- [ ] Preserve per-head global buffers in `ExtraShortlistHeadState`.
- [ ] Preserve device-side constraints for the v2 selector path.

Acceptance:

- [ ] Existing row-first global top-k tests still pass.
- [ ] Extra-head output parity matches the current host-finalized result.
- [ ] Route diagnostics still identify `row_first_global_topk` when expected.

## Phase 2: Device Score Heads For Row-Only Score Modes

Purpose: avoid `device_include_all` and host scoring for score modes that are
pure functions of completed metric rows.

Initial scope:

- `BalancedRiskAdjusted10`
- `AdversarialEnvelope10`
- `SupportShrunk10`
- `ActiveSortino10`
- `OmegaVsSharpe10`
- `RobustMargin10`

Defer initially:

- `EffectiveSampleLcb10`, because it needs pair observation context in addition
  to the metric row.

Tasks:

- [ ] Add a CUDA score-mode enum that mirrors `DoubleMaScoreMode` for row-only
  modes.
- [ ] Add device helper functions in `double_crossover.cu` that match the Rust
  score functions in `double_ma.rs`.
- [ ] Add a selector path that scores metric rows by score mode without forcing
  `device_include_all`.
- [ ] Keep the new score-mode selector behind an opt-in flag until parity and
  performance are proven.
- [ ] Add direct CUDA tests comparing device scores to Rust score functions for
  representative metric rows.
- [ ] Add end-to-end tests proving top-k identity matches the host score-mode
  path for each row-only score mode.

Acceptance:

- [ ] Row-only score modes no longer require full host materialization on the
  experimental path.
- [ ] Top-k order matches the host implementation for deterministic fixtures.
- [ ] No default route changes until the GPU perf gate passes.

## Phase 3: Device-Side Compact Union Composition

Purpose: reduce host-side union reranking and D2H work after multiple objective
heads have already been extracted from the same metric surface.

Preferred implementation order:

1. Device dedup of compact per-head top-k rows.
2. Device lexicographic union ranking.
3. Device weighted-rank union ranking.
4. Device Pareto and epsilon-front ranking only if Phase 0 shows enough host
   cost to justify the more complex O(k^2) logic.

Tasks:

- [ ] Add a compact union scratch buffer sized from `top_k` and head count, not
  from full grid size.
- [ ] Add a device kernel that merges primary and extra-head compact rows and
  deduplicates by `(fast_len, slow_len)`.
- [ ] Add a device ranking kernel for lexicographic union mode.
- [ ] Add a device ranking kernel for weighted-rank union mode.
- [ ] Copy back only the final compact union rows needed by the result.
- [ ] Keep the existing Rust `rerank_shortlist_by_union_mode` as the oracle and
  fallback.
- [ ] Add parity tests for all supported union modes, including duplicate rows
  across heads and tie-breakers.

Acceptance:

- [ ] Device union output matches the host oracle on deterministic fixtures.
- [ ] D2H row count is no larger than the current compact materialization path.
- [ ] The path falls back to host union if required CUDA symbols are missing.
- [ ] The path remains opt-in until GPU perf evidence clears the viability
  threshold.

## Phase 4: Device-Side Confidence Reranking

Purpose: evaluate whether PSR/DSR reranking can move to device without changing
semantics.

This phase depends on Phase 0. If confidence reranking is not a measured cost,
defer it.

Tasks:

- [ ] Decide whether peer mean/std should be computed on device or copied as a
  compact precomputed scalar pair from host.
- [ ] Match `validation_stats::compute_psr_dsr` semantics exactly or keep the
  host path.
- [ ] Add a compact confidence score buffer for materialized shortlist rows.
- [ ] Add a device confidence ranking path for `psr` and `dsr`.
- [ ] Keep the host confidence reranker as the oracle and fallback.

Acceptance:

- [ ] PSR/DSR order matches host for deterministic fixtures.
- [ ] Numeric differences are bounded and cannot flip documented tie-breakers.
- [ ] The path improves measured advanced shortlist workload time.

## Phase 5: Neighborhood Robustness From Existing Rows

Purpose: investigate whether neighborhood robustness can reuse already evaluated
rows in cases where the required local surface is available.

This is lower priority than score heads and union composition.

Current limitation:

- The optimizer computes tiled metric rows and retains compact top-k heads, not
  the full exact grid. For most normal runs, the neighborhood around the final
  best candidate is not resident after optimization finalization.

Potential approaches:

- Retain a bounded device-side neighborhood buffer only after a candidate is
  known.
- Perform a second device pass over only the requested neighborhood.
- Use existing local stability recompute when the grid surface was not retained.

Tasks:

- [ ] Measure current local-stability recompute cost before changing it.
- [ ] Do not add full-grid retention just for neighborhood diagnostics unless
  memory and timing evidence supports it.
- [ ] If implemented, keep the feature bounded by radius and step and preserve
  the existing local-stability summary as the oracle.

Acceptance:

- [ ] No additional full-grid memory residency is introduced on default routes.
- [ ] Neighborhood diagnostics match existing local-stability summaries.
- [ ] Any new path is skipped when the local surface is not already available or
  cheap to recompute.

## Testing Plan

Add tests close to the changed surface.

Rust unit and integration coverage:

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - device score helpers versus Rust score helpers
  - compact union dedup and ranking parity
  - CUDA symbol fallback behavior
- `crates/app_core/src/double_ma.rs`
  - end-to-end request behavior
  - top-k identity parity for `Trade22`
  - union/confidence result parity
  - route diagnostic expectations
- `web/src/services/optimizeRequestBuilder.test.ts`
  - request shaping remains unchanged unless a new explicit knob is added
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
  - capability text reflects opt-in or default status accurately

Correctness commands:

```powershell
cargo test -p ta_app_core
powershell -File scripts/run_workspace_tests_stable.ps1
```

CUDA correctness commands when a GPU is available:

```powershell
cargo test -p ta_app_core --features cuda,cuda-backtest-kernel
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

For low-level CUDA edits, compile fresh PTX before handoff. If Nsight or CUDA
profiling is needed and attach permissions fail, rerun from elevated
PowerShell.

## Performance Plan

Run measurements in this order:

1. Baseline current main branch behavior.
2. Enable the experimental path through an environment flag.
3. Run the same workload with identical inputs and no overlapping benchmarks.
4. Compare route diagnostics, top rows, D2H timing, host finalize timing, and
   wall time.
5. Run the default GPU perf gate before making the path default.

Minimum evidence to promote a path:

- Baseline and candidate outputs for the same workload.
- Route diagnostics showing the intended device path.
- Top-k parity or documented requested ranking-mode change.
- Timing evidence for the advanced workload.
- Default GPU perf gate pass.
- CPU-only safety pass for non-CUDA build behavior.

## Rollback And Feature Gating

Every experimental path should have a narrow rollback point.

- Use an opt-in environment flag while evaluating performance.
- Keep host implementations as the semantic oracle.
- Fall back to host when CUDA symbols are unavailable.
- Fall back to host when unsupported score modes, union modes, or metric strides
  are requested.
- Do not remove host-finalize timing telemetry after moving logic to device.

Suggested experimental flags:

- `VECTORBT_KERNEL_DEVICE_SCORE_MODE`
- `VECTORBT_KERNEL_DEVICE_SHORTLIST_UNION`
- `VECTORBT_KERNEL_DEVICE_SHORTLIST_CONFIDENCE`

Do not make these default until the viability threshold is met and the relevant
perf gates pass.

## Completion Checklist

- [ ] Phase 0 baseline proves this work is worth doing.
- [ ] Device score heads match Rust score functions.
- [ ] Device compact union matches host union reranking.
- [ ] Confidence reranking is either implemented with parity or explicitly
  deferred based on measurements.
- [ ] Neighborhood robustness is either improved with bounded evidence or
  explicitly deferred.
- [ ] Default GPU perf gates pass.
- [ ] CPU-only safety remains intact.
- [ ] Documentation and capability copy match the final shipped behavior.
