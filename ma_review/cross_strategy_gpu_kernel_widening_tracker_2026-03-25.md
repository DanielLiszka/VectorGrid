# Cross-Strategy GPU-Kernel Widening Tracker

Last updated: 2026-03-25

## Purpose

This tracker is a follow-on to the closed Double MA single-pass tracker in
`ma_review/double_ma_single_pass_valid_implementation_tracker_2026-03-24.md`.

Its purpose is narrower and more additive:

1. Widen the richer single-pass feature surface only to strategies/routes that truly support the exact `GpuKernel` path.
2. Separately preserve or widen full native candidate-return support across strategies via `include_all`.
3. Avoid forcing non-Double-MA strategies into the Double MA metric schema.

This document does **not** reopen the completed Double MA Phase 1-5 checklist.

## Current Repo State

- The Double MA family already has a widened always-on single-pass product path.
- The closed Double MA tracker already added the richer score / shortlist / neighborhood feature set on the exact shared `GpuKernel` route.
- Cross-strategy parity is already partially solved:
  - broad `include_all` support exists across many strategies
  - exact `GpuKernel` Double-MA-family routes already inherit most of the newer feature set
- The main remaining gap on the exact-kernel side is `DynamicDoubleMa`.
- The main remaining gap across the broader strategy surface is consistent full native metric return and result-shape clarity, not replay removal.

## Additive Scope

This tracker follows the already-agreed scope:

- Keep the richer single-pass optimization feature surface scoped to strategies/routes that support the exact `GpuKernel` path.
- Preserve or widen the ability to return the full native candidate result matrix across strategies via `include_all`.
- Define cross-strategy parity as "full native metric return for that route", not "make every strategy emit the Double MA 17-row metric contract".

That means:

- `GpuKernel` exact routes are the right target for widened single-pass metric / score / shortlist support.
- CPU, hybrid, and standalone GPU routes should still expose full native per-candidate results where feasible.
- Standalone strategy-local GPU routes in `composite_cpu.rs` should not be forced into the Double MA result contract unless there is a strong product reason.

## Explicit Non-Goals

These are out of scope for this tracker unless a later document reopens them:

- Reopening the closed Double MA Phase 1-5 checklist.
- Forcing `GpuHybridKernel` or CPU fallback routes to claim exact parity with the exact global-top-k `GpuKernel` route.
- Forcing standalone `composite_cpu.rs` strategies to adopt the Double MA shortlist / neighborhood / score-observation schema.
- Introducing hidden env-gated runtime behavior instead of request/UI-visible capabilities.
- Reintroducing finalist replay or adding second-pass behavior to preserve a fake notion of parity.

## Current Strategy Surface

| Strategy group | Current route family | Full native candidate return via `include_all` | Inherits widened single-pass feature set | Current status |
| --- | --- | --- | --- | --- |
| `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct` | exact shared `GpuKernel` when selected | Yes | Yes | Complete for this workstream. |
| `CompositeTrend`, `BreakoutMomentum` with ADX off | exact shared `GpuKernel` | Yes | Yes when they resolve to exact `GpuKernel` | Treat as verification targets, not greenfield widening. |
| `DynamicDoubleMa` | dynamic builder preview + `GpuKernel(dynamic_strategy_v1)` or fallback | Yes | Partial only | Main remaining exact-kernel widening target. |
| `CompositeTrend`, `BreakoutMomentum` with ADX on | `GpuHybridKernel` / CPU fallback | Yes | No guaranteed parity | Not in scope for rich feature parity in this tracker. |
| `RegimeHybrid` current ADX-bearing form | `GpuHybridKernel` / CPU fallback | Yes | No guaranteed parity | Not in scope for rich feature parity in this tracker. |
| `Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet`, `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, `Cdmc` | standalone unified-GPU / `composite_cpu.rs` strategy-local routes | Yes | No | Focus on full native metric-return parity and result-surface consistency. |
| `CompositeCpu` | CPU route | Yes | No | Full native candidate return only. |
| `CompositeMo`, `MlSignal` | separate workflows | No | No | Out of scope. |

## Code Surfaces To Treat As Canonical

Open these first for any implementation work under this tracker:

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/optimizer/src/lib.rs`

Representative verification files:

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`

## Primary Workstreams

### Workstream A: Exact `GpuKernel` Widening

Goal:

- Finish widening the richer single-pass feature set to the remaining exact-kernel-capable route shape that still lags: `DynamicDoubleMa`.

Key blocker evidence already present in code:

- `effective_sample_lcb_10` is explicitly rejected for `DynamicDoubleMa`.
- `extra_shortlist_objectives` are explicitly rejected for `DynamicDoubleMa`.
- `shortlist_confidence_mode` is explicitly rejected for `DynamicDoubleMa`.
- `shortlist_union_mode` is explicitly rejected for `DynamicDoubleMa`.
- `shortlist_slice_mode` is explicitly rejected for `DynamicDoubleMa`.
- `include_trade_metrics` is explicitly rejected for `DynamicDoubleMa`.

All of those checks currently live in `crates/app_core/src/double_ma.rs`.

Required outcome:

- When `DynamicDoubleMa` resolves to a provably exact single expanded `GpuKernel(dynamic_strategy_v1)` run shape, it should be allowed to opt into the same compatible advanced score / shortlist / trade-metric features as the static Double-MA-family exact route.
- When it does not resolve to that shape, the route should continue to reject or fall back clearly.

### Workstream B: Full Native Candidate-Return Parity

Goal:

- Make sure all strategy routes that claim `include_all` support can return their full native per-candidate result surface coherently, without requiring the Double MA schema.

This is broader than Workstream A and applies to:

- exact `GpuKernel` routes
- hybrid routes where `include_all` is already supported
- standalone unified-GPU strategy-local routes in `composite_cpu.rs`
- CPU routes that already claim `include_all`

Required outcome:

- `include_all=true` should mean "return the full native candidate result set for this route" consistently.
- Capability summaries and frontend copy should match reality.
- Result consumers should not assume the Double MA score-observation / shortlist diagnostics surface exists everywhere.

### Workstream C: Capability / UI / Result-Surface Clarity

Goal:

- Make the product-facing capability summaries and request/UI behavior reflect the real split:
  - rich exact-kernel single-pass features for exact `GpuKernel` routes
  - full native candidate return broadly where `include_all` exists

Required outcome:

- No hidden env-only widening behavior.
- No frontend implication that every strategy supports the full Double MA advanced feature set.
- No backend ambiguity about what "include_all" guarantees on each route family.

## Rollout Policy

- No widening feature should ship as a hidden runtime env toggle.
- If a widened feature changes semantics rather than simply exposing richer results, it should be a normal request/UI option, not an always-on silent change.
- Feature-off cost should remain effectively equivalent to the current feature-off path.
- Static exact `GpuKernel` routes should remain the reference implementation for richer single-pass behavior.
- `DynamicDoubleMa` should only inherit richer features when the runtime can prove the same exact single expanded route assumptions that the static path relies on.
- Standalone strategy-local GPU routes should prioritize native metric-return parity first; richer shortlist/diagnostic parity should only follow if their route contract supports it cleanly.

## Implementation Touchpoints

Use these file groups when executing the checklist:

### Phase 0-2 core backend files

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/strategy_dynamic.rs`

### Phase 1-4 frontend and product-surface files

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/stores/advancedStore.ts`

### Phase 3 standalone strategy parity files

- `crates/app_core/src/composite_cpu.rs`
- `crates/optimizer/src/lib.rs`
- `crates/app_core/src/double_ma_result.rs`

### Representative test surfaces

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- any route-specific tests already colocated in the touched backend files

## Detailed Checklist

### Phase 0: Baseline Audit

- [ ] Reconfirm the closed Double MA tracker remains complete and is not being reopened.
- [ ] Reconfirm the current strategy capability matrix in `web/src/utils/strategyOptimizationCapabilities.ts`.
- [ ] Reconfirm current route policy in `crates/app_core/src/composite_profile_route.rs`.
- [ ] Reconfirm the current `DynamicDoubleMa` rejection points in `crates/app_core/src/double_ma.rs`.
- [ ] Reconfirm how `BackendOptimizationResult` differs between `double_ma.rs` and `composite_cpu.rs`.
- [ ] Record the exact current support matrix in this tracker once the audit is complete.

### Phase 1: Exact `GpuKernel` Static-Route Verification

- [ ] Re-verify that `CompositeTrend` with ADX off inherits the widened single-pass feature set when it resolves to exact `GpuKernel`.
- [ ] Re-verify that `BreakoutMomentum` with ADX off inherits the widened single-pass feature set when it resolves to exact `GpuKernel`.
- [ ] Add or tighten targeted backend-route tests proving those routes behave like the static Double MA-family exact path for the supported feature set.
- [ ] Add or tighten frontend capability/request tests so ADX-off versus ADX-on behavior is described accurately.
- [ ] Confirm there are no stale route guards or UI caveats still implying those exact ADX-off routes are second-class.
- [ ] Record the verified ADX-off exact-kernel support matrix directly in this tracker before moving to `DynamicDoubleMa`.

### Phase 2: `DynamicDoubleMa` Exact-Route Widening

- [ ] Audit the exact preconditions under which `DynamicDoubleMa` can prove a single expanded exact `GpuKernel(dynamic_strategy_v1)` run.
- [ ] Separate "exact route proven" from "dynamic builder used" in request validation so compatible features are not rejected merely because the request is dynamic.
- [ ] Enable `effective_sample_lcb_10` for the exact compatible `DynamicDoubleMa` route.
- [ ] Enable `extra_shortlist_objectives` for the exact compatible `DynamicDoubleMa` route.
- [ ] Enable `shortlist_confidence_mode` for the exact compatible `DynamicDoubleMa` route.
- [ ] Enable `shortlist_union_mode` for the exact compatible `DynamicDoubleMa` route.
- [ ] Enable `shortlist_slice_mode` and neighborhood settings for the exact compatible `DynamicDoubleMa` route.
- [ ] Enable `include_trade_metrics` for the exact compatible `DynamicDoubleMa` route.
- [ ] Preserve explicit rejection or fallback for any `DynamicDoubleMa` shape that is not a single expanded exact-kernel run.
- [ ] Add targeted backend tests for each newly-enabled `DynamicDoubleMa` feature path.
- [ ] Add targeted request-builder / optimize-service tests covering the new `DynamicDoubleMa` request acceptance surface.
- [ ] Record which exact-route predicates were sufficient for `DynamicDoubleMa` widening and which shapes remain intentionally rejected.

### Phase 3: Full Native Candidate-Return Audit Across Strategies

- [ ] Audit every strategy family that advertises `supportsIncludeAll: true` and confirm the result payload truly carries the full native metric rows for that route.
- [ ] Audit whether `all_scores` should be populated consistently on broader route families or remain intentionally absent where no coherent score list exists.
- [ ] Audit whether any route currently claims `include_all` in the UI while silently omitting strategy-native metrics that should be present.
- [ ] Reconfirm the current large-grid `include_all` hard limits and ensure their errors are consistent across route families.
- [ ] Add or tighten representative `include_all` smoke tests for standalone unified-GPU strategy routes in `crates/app_core/src/composite_cpu.rs`.
- [ ] Add or tighten representative CPU-vs-GPU include-all parity tests where the route family already supports both.
- [ ] Document any intentionally route-specific omissions rather than hiding them behind generic product language.
- [ ] Record a per-strategy include-all audit table in this tracker showing: claimed support, observed returned fields, limits, and any intentional omissions.

### Phase 4: Capability and Result-Surface Cleanup

- [ ] Update `web/src/utils/strategyOptimizationCapabilities.ts` so the execution/output summaries match the widened reality after Phase 1-3 changes.
- [ ] Update request-builder and service-layer validation so unsupported advanced features fail clearly on non-exact routes.
- [ ] Ensure frontend advanced controls are shown only where the route family truly supports them.
- [ ] Ensure result consumers do not assume `shortlist_slice_diagnostics`, `score_observation_context`, or related exact-kernel-only fields exist on every route.
- [ ] Add/update tests for capability descriptions, request shaping, and result handling after any widening changes land.
- [ ] Recheck all user-facing copy so "include_all" and "advanced single-pass features" are not conflated.

### Phase 5: Verification, Performance, and Closure

- [ ] Run targeted Rust tests for any widened exact-kernel route changes.
- [ ] Run targeted frontend tests for request-building and capability summaries.
- [ ] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` after any exact-kernel widening change.
- [ ] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` before closing this tracker.
- [ ] If any benchmark is needed, keep all benchmark runs serialized and precheck that no `cargo` / benchmark process is already active.
- [ ] Record benchmark and gate evidence directly in this document before closing it.
- [ ] Record the final support matrix after widening, including what remains intentionally unsupported.
- [ ] Close this tracker only after there are no hidden env-gated widening features left in scope and the frontend/backend capability surfaces agree.

## Validation Standards

### Correctness

- Preserve the existing widened Double MA default path unless a later tracked change explicitly alters it.
- Preserve CPU fallback behavior and existing route-selection semantics.
- Do not silently widen unsupported routes; either prove exact compatibility or reject clearly.
- Do not force standalone strategies to emit fields whose semantics only exist on the Double MA exact-kernel route.

### Performance

- Reject any change that reintroduces replay or extra-pass behavior on the widened exact `GpuKernel` path.
- Treat `run_gpu_perf_gate.ps1 -Mode candidate` as mandatory for exact-kernel widening work.
- Prefer representative targeted strategy runs over broad speculative benchmarking.
- Never run multiple benchmarks at the same time.
- Preserve the default widened Double MA product path performance unless a later tracked change explicitly targets it.
- For exact-route widening work, reject any default-off regression that is clearly attributable to widened feature selection logic rather than the feature itself.
- For newly-enabled opt-in features, small opt-in cost is acceptable; large structural regressions or replay-like behavior are not.

### Product Surface

- No hidden runtime env gates for shipped widening behavior.
- Optional advanced features are acceptable when they are real request/UI features.
- Feature-off cost should remain effectively equivalent to a hardcoded feature-off path.
- Capability summaries, request validation, and actual backend support must agree before closure.

## Closure Criteria

This tracker is only complete when all of the following are true:

- The exact-kernel widening target set is either implemented or explicitly documented as intentionally out of scope.
- `DynamicDoubleMa` no longer has stale blanket rejections where an exact compatible route can be proven.
- The broad `include_all` product surface has been audited and any route-specific limitations are documented clearly.
- Frontend capability summaries match the actual backend support matrix.
- GPU perf/workflow gates are recorded clean after the widening work.
- No hidden runtime env-only widening behavior remains in scope.

## Suggested Implementation Order

1. Phase 0 audit
2. Phase 1 exact static-route verification
3. Phase 2 `DynamicDoubleMa`
4. Phase 3 full native candidate-return audit
5. Phase 4 capability/result-surface cleanup
6. Phase 5 verification and closure

## Notes

- Broadening `include_all` is mostly a consistency and audit problem, not a replay-removal problem.
- The most important remaining exact-kernel widening target is `DynamicDoubleMa`.
- Hybrid routes remain a separate future track unless a later document explicitly reopens them.
