# Standalone Unified-GPU Easy Extras Tracker

Last updated: 2026-03-26

## Purpose

This tracker is a follow-on to the closed cross-strategy widening tracker in
`ma_review/cross_strategy_gpu_kernel_widening_tracker_2026-03-25.md`.

Its scope is intentionally narrow:

1. Add the easy additive single-pass extras to the standalone unified-GPU strategy routes.
2. Reuse existing host/result-layer helpers wherever possible.
3. Avoid any redesign that would force these routes into the Double MA exact-kernel contract.

This tracker is specifically for the standalone unified-GPU strategies that run through
`crates/app_core/src/composite_cpu.rs`, such as:

- `Asr`
- `Vcc`
- `Vemr`
- `Rocb`
- `Vcet`
- `Mkcs`
- `Kqsr`
- `Carm`
- `Kvdf`
- `Cdmc`

It does not reopen the closed Double MA single-pass tracker and it does not claim parity with the
Double MA `GpuKernel` shortlist/slice/observation-context surface.

## Agreed Additive Scope

The target "easy / good candidates" for this tracker are:

1. `all_scores` for standalone unified-GPU `include_all=true` runs.
2. Host-side `extra_shortlist_objectives` plus `shortlist_union_mode` for standalone unified-GPU
   routes when full candidate rows are already materialized.
3. Host-side `shortlist_confidence_mode` reranking on the retained shortlist rows, using
   route-local observation-context reconstruction rather than a second optimization pass.

The point of this tracker is to extend useful result/ranking behavior without:

- widening the GPU kernel metric contract for these strategies
- rewriting the strategy-local GPU evaluators
- introducing a second pass
- pretending these routes expose the full Double MA exact-kernel diagnostics surface

## Current Repo State

### What already exists

- Standalone unified-GPU strategies already normalize `top_k` and `include_all` through
  `normalize_topk_include_all(...)` in `crates/app_core/src/composite_cpu.rs`.
- Those routes already use `StreamAggregator`, preserve `best_score` and `top_scores`, and return
  the full native candidate rows when `include_all=true`.
- `BackendOptimizationResult` and the shared backend-result builder already have fields for:
  - `all_scores`
  - `shortlist_slice_diagnostics`
  - `score_observation_context`
- Existing host helpers already exist for the core shortlist behaviors:
  - `build_exact_extra_shortlist_rows(...)` in `crates/app_core/src/double_ma.rs`
  - `rerank_shortlist_by_union_mode(...)` in `crates/app_core/src/gpu_backtest_kernel.rs`
  - `rerank_shortlist_by_confidence_for_host(...)` in `crates/app_core/src/gpu_backtest_kernel.rs`

### Current gaps

- Standalone unified-GPU result assembly repeatedly sets:
  - `all_scores: None`
  - `shortlist_slice_diagnostics: None`
  - `score_observation_context: None`
- `CompositeCpuRequest` in `crates/app_core/src/composite_cpu.rs` does not currently carry:
  - `extra_shortlist_objectives`
  - `shortlist_union_mode`
  - `shortlist_confidence_mode`
- Frontend capability copy in `web/src/utils/strategyOptimizationCapabilities.ts` currently treats
  these routes as `Top-K + include-all` only, not as routes with any optional shortlist extras.
- The current confidence rerank helper assumes one shared observation context for the whole
  shortlist. That is fine for the exact Double MA kernel path, but the standalone unified-GPU
  routes generally need row-local context reconstruction because their effective warmup/start bars
  vary with the candidate params.

## Key Design Decision

This tracker keeps the work in the host/result layer as much as possible.

That means:

- `all_scores` should be added without touching the device evaluator shape.
- Union shortlist support should first piggyback on `include_all=true`, because the full candidate
  rows are already present on the host in that mode.
- Confidence rerank should operate on the retained shortlist only, and should reconstruct the
  needed observation context per returned row rather than adding a new GPU metric surface.

## Explicit Non-Goals

These are out of scope for this tracker:

- forcing standalone unified-GPU routes into the Double MA 17-row / 22-row metric contract
- adding `shortlist_slice_mode`, neighborhood robustness, or `score_observation_context` parity to
  these routes
- adding device-side multi-head top-k for the standalone unified-GPU routes
- enabling extra shortlist union on `include_all=false` by introducing a new multi-head streaming
  reduction layer
- introducing a second optimization pass or replay pass
- changing `include_all` defaults
- hiding new behavior behind env vars

## Implementation Strategy

### Phase 0: Baseline Audit and Helper Reuse Plan

Goal:

- Confirm the smallest code path that can implement the three easy candidates with minimal churn.

Implementation notes:

- Audit every repeated `BackendOptimizationResult` construction in
  `crates/app_core/src/composite_cpu.rs`.
- Decide whether to:
  - add one local `composite_cpu.rs` result-construction helper, or
  - reuse `crates/app_core/src/double_ma_result.rs` more directly.
- Confirm whether `build_exact_extra_shortlist_rows(...)` can be reused as-is by widening
  visibility, or whether it should move to a small shared host-shortlist helper surface.
- Confirm whether the confidence rerank formula should be extracted to a per-row helper instead of
  reusing the current shared-context wrapper as-is.

Deliverable:

- One clear implementation path with minimal duplicate edits across the standalone strategy routes.

### Phase 1: `all_scores` for `include_all=true`

Goal:

- When a standalone unified-GPU strategy returns `all`, also return `all_scores` for those same
  rows.

Why this is easy:

- `all` already exists.
- `objective_score(...)` already exists in `crates/app_core/src/composite_cpu.rs`.
- No request-shape or kernel-shape change is required.

Recommended implementation:

1. Add a shared local helper in `crates/app_core/src/composite_cpu.rs` that builds
   `BackendOptimizationResult` consistently for these routes.
2. Compute:
   - `all_scores = all.as_ref().map(|rows| rows.iter().map(|(_, m)| objective_score(m, objective)).collect())`
3. Route all repeated standalone unified-GPU result construction through that helper.

Validation:

- Representative include-all GPU smoke tests should assert:
  - `all` exists
  - `all_scores` exists
  - `all_scores.len() == all.len()`
- At least 2 representative standalone strategies should be covered.

### Phase 2: Host-Side Extra Shortlist Heads and Union Rerank

Goal:

- Add `extra_shortlist_objectives` and optional `shortlist_union_mode` to standalone unified-GPU
  routes without adding a new GPU reduction path.

Scope rule:

- In this tracker, support this only when `include_all=true`.
- Reason: the full candidate metric rows are already on the host in that mode, so the union can be
  built cheaply and exactly without redesigning `StreamAggregator`.

Recommended implementation:

1. Add request fields to `CompositeCpuRequest`:
   - `extra_shortlist_objectives: Option<Vec<ObjectiveKind>>`
   - `shortlist_union_mode: Option<DoubleMaShortlistUnionMode>`
2. Reuse or extract `build_exact_extra_shortlist_rows(...)` from `crates/app_core/src/double_ma.rs`.
3. Reuse `rerank_shortlist_by_union_mode(...)` from
   `crates/app_core/src/gpu_backtest_kernel.rs`.
4. Apply the behavior after the normal `OptimizationResult` is finalized:
   - build the union shortlist from `result.all`
   - if `shortlist_union_mode` is set, rerank that union shortlist
   - update `best_params`, `best_metrics`, `best_score`, `top`, and `top_scores`
   - preserve `all` and `all_scores` unchanged

Request-builder rules:

- Require `top_k > 0`
- Require `include_all=true`
- Reject on routes that are not standalone unified-GPU
- Continue to reject on CPU-only / hybrid / exact-Double-MA-incompatible routes

Frontend/product surface:

- Show `extra_shortlist_objectives` and `shortlist_union_mode` for standalone unified-GPU routes
  only.
- Add route note text that this is a host-side union over the returned full candidate set, not the
  exact Double MA kernel union path.
- Make the `include_all=true` requirement explicit in the UI note and request-builder errors.

Validation:

- Request-builder tests for accept/reject rules
- Backend tests that compare the returned shortlist against an exhaustive host union built from the
  same `all` rows
- At least 1 representative standalone strategy GPU route and 1 negative route case

### Phase 3: Host-Side Confidence Rerank on Retained Shortlist Rows

Goal:

- Add `shortlist_confidence_mode` for standalone unified-GPU routes without requiring full-grid
  observation-context export or a second pass.

Key constraint:

- The current helper `rerank_shortlist_by_confidence_for_host(...)` assumes one shared observation
  context for the whole shortlist.
- That assumption is too strong for many standalone unified-GPU strategies, because candidate
  warmup/start bars vary with the candidate params.

Recommended implementation:

1. Add `shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>` to
   `CompositeCpuRequest`.
2. Extract the row-level confidence computation from the current helper path in
   `crates/app_core/src/gpu_backtest_kernel.rs` into a reusable per-row helper.
3. In `crates/app_core/src/composite_cpu.rs`, add a narrow per-strategy observation-context
   builder for retained shortlist rows:
   - derive the candidate's effective start bars from the already-available
     `resolve_*_periods(...)` + `compute_*_start_t(...)` helpers
   - use current candle count and `trade_on_next_bar`
   - do this only for the retained shortlist rows, not for the full grid
4. Rerank:
   - `result.top` by default
   - or the already-built union shortlist if Phase 2 features are also active
5. Update:
   - `best_params`
   - `best_metrics`
   - `best_score`
   - `top`
   - `top_scores`
6. Leave:
   - `all`
   - `all_scores`
   unchanged

Request-builder rules:

- Require `top_k > 0`
- Allow with `include_all=false`
- If union shortlist is also requested, apply confidence rerank after the union shortlist is
  formed
- Reject on routes that are not standalone unified-GPU

Frontend/product surface:

- Show `shortlist_confidence_mode` for standalone unified-GPU routes
- Make the route note explicit:
  - reranks the returned shortlist only
  - does not imply Double MA kernel score-observation parity
  - does not add slice/neighborhood diagnostics

Validation:

- Request-builder accept/reject tests
- Backend tests that compare the reranked shortlist against a manual per-row confidence sort for
  retained rows
- Cover at least 2 standalone unified-GPU strategies with different start-bar logic

## Canonical Files To Touch

### Backend

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/optimizer/src/lib.rs`

### Frontend

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.test.ts`

### Representative tests

- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `crates/app_core/src/composite_cpu.rs`

## Performance and Validation Policy

- Feature-off behavior must remain effectively unchanged.
- No new benchmark or test should be run in parallel with another benchmark.
- Before any benchmark/perf-gate run, check that no stale benchmark processes are active.
- Performance acceptance rule:
  - feature-off / disabled path: treat any regression above normal noise as unacceptable; target
    effectively flat behavior and use a hard stop at roughly `<= 3%`
  - feature-on / enabled path: allow a bounded localized regression only on the enabled feature
    path, with a hard ceiling of `<= 15%`
- The `15%` ceiling applies to the feature path that explicitly requested the added behavior, not to
  the default standalone unified-GPU optimization path.
- Minimum validation before closure:
  - targeted Rust tests for each phase
  - targeted frontend tests for capability/request/UI updates
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- If any shared CUDA/runtime helper changes land in `gpu_backtest_kernel.rs`, also run:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

Expected perf posture:

- Phase 1 should be effectively free unless `include_all=true`.
- Phase 2 should add cost only when `include_all=true` plus union-shortlist features are enabled.
- Phase 3 should add cost only when confidence rerank is enabled, and only on retained shortlist
  rows.
- Any measurement above the `<= 15%` enabled-path ceiling should be treated as a redesign signal,
  not something to quietly accept.

## Detailed Checklist

### Phase 0: Audit and helper plan

- [x] Reconfirm every standalone unified-GPU route in `crates/app_core/src/composite_cpu.rs` still
      returns `all` on `include_all=true`.
- [x] Reconfirm all repeated standalone result-construction blocks currently set `all_scores: None`.
- [x] Decide whether to add one local result-construction helper in `composite_cpu.rs` or reuse
      `double_ma_result.rs` directly.
- [x] Decide whether `build_exact_extra_shortlist_rows(...)` should be widened for reuse or moved
      to a shared host-shortlist helper.
- [x] Decide whether the confidence formula should be extracted to a reusable row-level helper.

### Phase 1: `all_scores`

- [x] Add one shared result-construction helper for standalone unified-GPU routes.
- [x] Populate `all_scores` when `all` is present.
- [x] Keep `all_scores=None` when `include_all=false`.
- [x] Update representative GPU include-all tests to assert `all_scores` presence and length.
- [x] Update any frontend result-consumer tests if they currently assume `all_scores` is absent on
      these routes.

### Phase 2: extra shortlist objectives and union rerank

- [x] Add `extra_shortlist_objectives` to `CompositeCpuRequest`.
- [x] Add `shortlist_union_mode` to `CompositeCpuRequest`.
- [x] Reuse or extract `build_exact_extra_shortlist_rows(...)`.
- [x] Reuse `rerank_shortlist_by_union_mode(...)`.
- [x] Restrict this path to standalone unified-GPU routes with `include_all=true`.
- [x] Recompute `best_*`, `top`, and `top_scores` from the union shortlist.
- [x] Preserve `all` and `all_scores` unchanged.
- [x] Update `strategyOptimizationCapabilities.ts` for these routes.
- [x] Update `AdvancedWorkspacePanel.vue` copy/visibility for these routes.
- [x] Update `optimizeRequestBuilder.ts` validation:
      - [x] require `top_k > 0`
      - [x] require `include_all=true`
      - [x] reject CPU/hybrid/non-standalone routes
- [x] Add backend tests comparing union results to exhaustive host union on representative
      standalone strategies.
- [x] Add request-builder and capability tests.

### Phase 3: shortlist confidence rerank

- [x] Add `shortlist_confidence_mode` to `CompositeCpuRequest`.
- [x] Extract reusable row-level confidence scoring from the current helper path.
- [x] Add retained-shortlist observation-context builders for representative standalone strategies.
- [x] Extend the pattern to the rest of the standalone unified-GPU strategy set.
- [x] Rerank retained shortlist rows by confidence.
- [x] Apply confidence rerank after union-shortlist rerank when both are requested.
- [x] Recompute `best_*`, `top`, and `top_scores` from the reranked shortlist.
- [x] Keep `all` and `all_scores` unchanged.
- [x] Update `strategyOptimizationCapabilities.ts` for standalone confidence support.
- [x] Update `AdvancedWorkspacePanel.vue` copy/visibility for standalone confidence support.
- [x] Update `optimizeRequestBuilder.ts` validation:
      - [x] require `top_k > 0`
      - [x] reject CPU/hybrid/non-standalone routes
- [x] Add backend parity tests against manual retained-shortlist confidence sorting.
- [x] Add request-builder and capability tests.

### Phase 4: closure

- [x] Run targeted Rust tests for all 3 phases.
- [x] Run targeted frontend tests for capability/request/UI changes.
- [x] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`.
- [x] If shared CUDA/runtime helpers changed, run
      `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- [x] Update this tracker with:
      - [x] implemented files
      - [x] validation commands
      - [x] perf notes
      - [x] final route/capability summary

## Completion Notes

### Implemented files

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.test.ts`

### Validation commands

- `cargo check -p ta_app_core --tests`
- `cargo check -p ta_app_core --tests --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel standalone_unified_gpu_shortlist_extras_require_include_all -- --nocapture --test-threads=1`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel vemr_gpu_kernel_supports_standalone_ -- --nocapture --test-threads=1`
- `npm test -- src/services/optimizeRequestBuilder.test.ts src/utils/strategyOptimizationCapabilities.test.ts src/components/AdvancedWorkspacePanel.test.ts`
- `npm run typecheck`
  - current status: still fails only on the pre-existing `validationRequestBuilder.ts` `replaceAll`
    TS-lib-target issue; the new standalone-shortlist fields are now type-correct
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

### Perf notes

- Feature-off/default-path behavior remained within the agreed posture:
  - workflow gate passed with `runtime_not_regressed_50pct: 1681ms vs baseline 1744ms`
  - GPU perf gate passed with `gpu_criterion_mean_not_regressed_10pct: 82.815344 ms vs baseline 155.288513 ms`
  - GPU dynamic-strategy check also passed: `75.25 ms on vs 80.75 ms off`
- The new standalone extras are opt-in and stay in the host/result layer:
  - `all_scores` only materializes when `include_all=true`
  - union shortlist support is restricted to `include_all=true`
  - confidence rerank runs only on retained shortlist rows
- No default-path widening of device metric rows or new second pass was introduced.

### Final route/capability summary

- Standalone unified-GPU routes now return `all_scores` alongside `all` on `include_all=true`.
- Standalone unified-GPU routes now accept:
  - `extra_shortlist_objectives`
  - `shortlist_union_mode`
  - `shortlist_confidence_mode`
- Scope remains intentionally narrower than the Double MA exact-kernel surface:
  - no slice/neighborhood diagnostics parity
  - no `score_observation_context`
  - no device-side multi-head top-k redesign
- Frontend capability copy and advanced controls now expose these standalone extras only on the
  requested `GpuKernel` preview, with explicit `includeAll=true` messaging for union-style extras.

## Closure Criteria

This tracker is complete when:

- standalone unified-GPU routes expose `all_scores` on `include_all=true`
- standalone unified-GPU routes support host-side extra shortlist objectives and union rerank in
  the agreed `include_all=true` scope
- standalone unified-GPU routes support host-side confidence rerank on retained shortlist rows
- frontend capability text and advanced controls match the actual support surface
- no hidden env-gated runtime behavior is required
- workflow/gate evidence is recorded here
- measured regressions stay within the policy above:
  - feature-off effectively flat / `<= 3%`
  - feature-on localized and `<= 15%`
