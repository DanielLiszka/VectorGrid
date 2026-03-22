# Composite Route Policy and Request Shaping Valid Implementation Plan

Date: 2026-03-21

## Purpose

This document is the implementation-facing follow-up to
`research/composite_route_policy_request_shaping_optimization_plan_v2.md`.

It includes only the guide components that remain valid after repo-grounded
review against the current codebase. It also separates:

- core implementation work that should be planned and executed
- optional or gated work that should be done last, if at all
- invalid recommendations from the source guide that should not be implemented

## Source Review Summary

The source guide was directionally strong on frontend drift, capability
coverage gaps, and request-shaping waste. The main corrections from the review
were:

1. The current product optimization route for `CompositeTrend`,
   `RegimeHybrid`, and `BreakoutMomentum` is the `double_ma` family, not
   `CompositeCpu`.
2. CPU heatmap support must remain intact for the composite trio on the
   double-MA path.
3. `composite_profile_route.rs` is not the active backend route owner for the
   current optimization path.
4. The Rust label refactor proposed in the source guide should not be treated
   as part of the main implementation.

## Verified Constraints

- Current route owner for the composite trio:
  `web/src/services/optimizeService.ts`,
  `web/src/services/optimizeRequestBuilder.ts`,
  `crates/app_core/src/double_ma.rs`
- Composite route-policy helper still matters for profile identity and
  auxiliary composite surfaces:
  `crates/app_core/src/composite_profile_route.rs`
- Frontend capability summaries are user-visible and directly affect exposed
  controls:
  `web/src/utils/strategyOptimizationCapabilities.ts`,
  `web/src/components/AdvancedWorkspacePanel.vue`,
  `web/src/components/OptimizeWorkspacePanel.vue`
- Current backend guardrails that the frontend should mirror:
  `crates/app_core/src/double_ma.rs`,
  `crates/app_core/src/gpu_backtest_kernel.rs`

## Implementation Goals

1. Remove confirmed frontend route-policy drift for the composite trio.
2. Make request-builder guardrails depend on the resolved route target instead
   of the raw requested backend string.
3. Fix user-visible capability exposure so it matches current supported
   optimizer behavior.
4. Remove avoidable request-shaping allocations and repeated parsing in the
   request builders.
5. Add regression coverage for the known drift points and guardrail matrix.

## Non-Goals

- No new strategies.
- No broad UI redesign.
- No route-policy redesign centered on `CompositeCpu`.
- No unsafe or uninitialized-memory work in this frontend/request-builder
  surface.
- No backend request schema expansion unless separately proven safe.

## Core Implementation Plan

## Phase 1: Canonical Composite Route Preview in the Frontend

The frontend should have one shared composite route-policy helper for
`CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`.

This helper should represent the composite profile target route, not the full
runtime-effective backend. It must stay explicit about that distinction because
full runtime routing still depends on MA support, CUDA availability, and
backend fallback behavior.

### Required outcomes

- Centralize requested-backend normalization for the composite trio.
- Centralize ADX-sensitive route targeting for the composite trio.
- Preserve current alias handling for legacy backend vocabulary.
- Expose enough information for:
  - `exportAllCsvPath` gating
  - kernel-only restriction gating
  - capability summaries
  - UI execution summaries

### Primary files

- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### Checklist

- [ ] Add a shared composite requested-backend normalizer in
      `web/src/strategies/compositeProfileGpuPolicy.ts`.
- [ ] Add a composite route preview type in
      `web/src/strategies/compositeProfileGpuPolicy.ts`.
- [ ] Keep the preview scoped to composite route policy only; do not claim it
      is the full runtime-effective backend.
- [ ] Preserve alias support for `GpuOnly` and `GpuHybridKernel`.
- [ ] Keep blank or unspecified backend behavior consistent with current
      product defaults unless a separate UI-default change is intentionally
      approved.
- [ ] Do not serialize any new preview-only fields into the backend request.

## Phase 2: Route-Aware Guardrails in `optimizeRequestBuilder.ts`

The current builder resolves route shape late and applies some guardrails using
the raw requested backend string. That is the main correctness drift to remove.

### Required outcomes

- Compute the route preview before composite-specific guardrails run.
- Gate kernel-only restrictions from the resolved composite route target, not
  from the requested backend token.
- Gate `exportAllCsvPath` from the resolved route target and final payload
  intent.
- Mirror known backend guardrails through named constants.

### Guardrails to preserve or add

- `MAX_HEATMAP_BINS = 512`
- `MAX_INCLUDE_ALL_ROWS = 2_000_000`
- `MAX_GPU_KERNEL_TOP_K = 256` when `includeAll=false` and the route is kernel

### Important parity rules

- `exportAllCsvPath` stays disabled for hybrid and CPU routes.
- `exportAllCsvPath` remains `Grid`-only.
- `exportAllCsvPath` remains incompatible with `includeAll=true`.
- `exportAllCsvPath` remains incompatible with MA-parameter sweep optimization.
- CPU heatmap support must remain available.
- CPU `topK` and `includeAll` support must remain available.

### Primary files

- `web/src/services/optimizeRequestBuilder.ts`
- `crates/app_core/src/gpu_backtest_kernel.rs` as backend-reference-only
- `crates/app_core/src/double_ma.rs` as backend-reference-only

### Checklist

- [ ] Add named frontend guardrail constants near the top of
      `web/src/services/optimizeRequestBuilder.ts`.
- [ ] Resolve the composite route preview before kernel-only guardrails run in
      `buildDoubleMaOptimizationRequest`.
- [ ] Make kernel-only overlay restrictions use the resolved route target.
- [ ] Make `exportAllCsvPath` use the resolved route target rather than the raw
      requested backend string.
- [ ] Add frontend preflight for `topK > 256` when the resolved route is
      kernel and `includeAll=false`.
- [ ] Keep CPU heatmap support intact.
- [ ] Keep CPU `topK` and `includeAll` support intact.
- [ ] Keep legacy backend alias normalization intact during the transition.

## Phase 3: Capability-Layer Correctness

The current capability layer is missing real supported kinds and is too static
for the composite trio.

The most important user-visible correctness fix is:

- the composite trio should only advertise `exportAll` when the route target is
  kernel-capable
- missing double-MA overlay kinds should not be classified as `Unknown`

### Required outcomes

- Expand `StrategyOptimizationKind` to include the missing double-MA overlay
  kinds.
- Route `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` through
  composite route-preview logic.
- Keep heatmap, `topK`, and `includeAll` available for CPU composite-trio
  requests.
- Propagate route inputs from the current UI to the capability summary helper.

### Missing supported kinds to add

- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`

### Primary files

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/stores/optimizeStore.ts` as reference

### Checklist

- [ ] Add `Pit`, `Set`, `Mts`, `Orc`, and `Marrt` to
      `StrategyOptimizationKind`.
- [ ] Replace partial repeated double-MA-family checks with a shared family set.
- [ ] Add optional composite route inputs to
      `describeStrategyOptimizationCapabilities(...)`.
- [ ] Make `supportsExportAll` route-aware for the composite trio.
- [ ] Keep `supportsHeatmap=true` for explicit `CpuOnly` requests on the
      composite trio.
- [ ] Keep `supportsTopK=true` for explicit `CpuOnly` requests on the
      composite trio.
- [ ] Keep `supportsIncludeAll=true` for explicit `CpuOnly` requests on the
      composite trio.
- [ ] Update `web/src/components/AdvancedWorkspacePanel.vue` to pass
      `requestedBackend`, `trendUseAdx`, and `breakoutUseAdx`.
- [ ] Update `web/src/components/OptimizeWorkspacePanel.vue` to pass the same
      route inputs where capability text is shown.

## Phase 4: Request-Builder Warm-Path Cleanup

These items are valid because they remove confirmed wasted work in the current
builders. They are still optimization opportunities rather than benchmarked
hotspots, so they should remain surgical and parity-safe.

### Required outcomes

- Replace range-array materialization with arithmetic range parsing/counting.
- Reuse parsed ranges where later domain construction currently reparses them.
- Remove full support-map cloning on the common path.
- Split common strategy parsing from kind-specific strategy shaping.
- Add capped multiplication for combination guardrails.
- Canonicalize MA IDs once in the support-lookup path.

### Primary files

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/core/ranges.ts`

### Checklist

- [ ] Add parsed integer-range helpers for the request builders.
- [ ] Replace range expansion in `buildDoubleMaOptimizationRequest`.
- [ ] Replace range expansion in `buildCompositeCpuOptimizationRequest`.
- [ ] Replace range expansion in `buildCompositeMoOptimizationRequest`.
- [ ] Reuse parsed ranges directly when building `CompositeMo` `IntRange`
      domains.
- [ ] Replace full MA support-map cloning with a lookup-oriented helper.
- [ ] Canonicalize MA IDs once for support lookups.
- [ ] Add capped multiplication for combo guardrails.
- [ ] Split common parsed execution/strategy fields from kind-specific parsing
      in `buildStrategyConfigFromInputs`.
- [ ] Keep the refactor surgical; do not rewrite per-kind strategy object
      literals unless a later benchmark proves they remain material.

## Phase 5: `tradeOnNext` Cleanup

`tradeOnNext` is currently a dead input on this optimization-builder surface.
The safe near-term move is documentation and explicit tests, not a runtime
behavior change.

### Required outcomes

- Keep runtime behavior unchanged for now.
- Make the fixed `true` behavior explicit in tests and planning notes.

### Primary files

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/components/StrategyWorkspacePanel.vue` as behavior reference

### Checklist

- [ ] Document in code comments or nearby planning notes that
      `trade_on_next_bar` is intentionally fixed to `true` on this path.
- [ ] Add a test that makes the fixed `true` behavior explicit.
- [ ] Do not wire `tradeOnNext` through to runtime behavior until backend
      semantics are separately confirmed.

## Phase 6: Regression Test Expansion

This work should land with the builder/capability changes, not as a later
cleanup.

### Frontend tests to add

- Composite route-target matrix:
  - `CompositeTrend` with ADX off
  - `CompositeTrend` with ADX on
  - `RegimeHybrid`
  - `BreakoutMomentum` with ADX off
  - `BreakoutMomentum` with ADX on
- Requested backend matrix:
  - `Auto`
  - `CpuOnly`
  - `GpuKernel`
  - legacy aliases still accepted
- `exportAllCsvPath` matrix:
  - allowed on kernel-targeted composite requests
  - rejected on hybrid-targeted composite requests
  - rejected on CPU requests
- Kernel-only restriction matrix:
  - restrictions apply on kernel target
  - restrictions do not trip only because a composite route preview resolves to
    hybrid
- Capability coverage:
  - missing overlay kinds no longer classify as `Unknown`
  - composite `supportsExportAll` follows route target
  - composite CPU requests still advertise heatmap support
- MA-id handling:
  - support lookup treats MA IDs consistently
- `tradeOnNext`:
  - fixed `true` behavior is explicit

### Optional small backend tests that are still valid

- precedence tests for `composite_profile_from_strategy(...)`

### Primary files

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- `web/src/strategies/compositeProfileGpuPolicy.test.ts`
- `crates/app_core/src/composite_profile_route.rs` for optional precedence tests

### Checklist

- [ ] Add composite route-target matrix tests to
      `web/src/services/optimizeRequestBuilder.test.ts`.
- [ ] Add composite capability-route tests to
      `web/src/utils/strategyOptimizationCapabilities.test.ts`.
- [ ] Add tests for the missing overlay kinds in
      `web/src/utils/strategyOptimizationCapabilities.test.ts`.
- [ ] Add tests for `exportAllCsvPath` kernel-vs-hybrid behavior.
- [ ] Add tests for `Auto` + composite route-target behavior.
- [ ] Add tests that confirm CPU composite requests still support heatmap.
- [ ] Add a test for fixed `trade_on_next_bar: true`.
- [ ] Add optional precedence tests in
      `crates/app_core/src/composite_profile_route.rs`.

## Gated but Valid Items

These items are valid, but they are not clean drop-in changes. They require a
coordinated decision or a broader audit first.

### Gated item 1: Fail-Closed Unknown Capability Behavior

This is reasonable, but only after auditing current kind normalization,
saved-state behavior, and the existing fallback-to-`DoubleMa` behavior in other
frontend surfaces.

### Gated item 2: Strict Unknown Backend Rejection

This is reasonable, but only after auditing all backend callers and preserving
legacy aliases.

### Gated checklist

- [ ] Audit strategy kind normalization before changing unknown-capability
      behavior.
- [ ] Audit backend caller vocabulary before making unknown backend tokens hard
      errors.

## Source-Guide Optional Items / Implement Last

These items were explicitly marked optional, late, or proof-dependent in the
source guide. They should be implemented only after the core phases are
complete.

### Optional item 1: Request-Builder Microbenchmark

Useful for evidence, but not required to implement the core correctness fixes.

### Optional item 2: Property-Based Arithmetic Counter Tests

Useful after the arithmetic range refactor lands.

### Optional item 3: Mostly Allocation-Free Frozen Capability Tables

Low-risk frontend churn reduction, but lower value than the core correctness
work.

### Optional item 4: Sparse Payload Emission

Only if backend deserialize/default behavior is explicitly proven safe.

### Optional item 5: Generated Route/Capability Manifest from Rust

Long-term drift reduction idea, but too invasive for the current bundle.

### Optional item 6: Backend Route-Label Cleanup

Only revisit after the current frontend/request-shaping work is complete and
only with the active `double_ma` route owner in scope.

### Optional checklist

- [ ] Add a request-builder benchmark harness if later prioritization needs it.
- [ ] Add property-based tests for arithmetic counters after the refactor lands.
- [ ] Consider frozen capability tables only if reactive churn remains worth
      reducing.
- [ ] Consider sparse payload emission only after proving backend struct
      defaults and command-boundary compatibility.
- [ ] Consider a generated route/capability manifest only as a separate later
      project.
- [ ] Consider backend route-label cleanup only with `double_ma` route files in
      scope.

## Explicitly Excluded from Implementation

The following source-guide ideas should not be implemented from that guide as
written:

- Do not implement `supportsHeatmap: false` for explicit `CpuOnly` composite
  requests.
- Do not treat `crates/app_core/src/composite_profile_route.rs` as the current
  optimization route owner for the composite trio.
- Do not implement the source guide's Rust route-label refactor as a core part
  of this workstream.
- Do not add unsafe or uninitialized-memory work in this frontend/request
  shaping surface.

## Recommended Delivery Order

1. Phase 1: canonical composite route preview
2. Phase 2: route-aware guardrail fixes in the request builder
3. Phase 3: capability-layer correctness
4. Phase 4: request-builder warm-path cleanup
5. Phase 5: `tradeOnNext` cleanup
6. Phase 6: regression-test expansion
7. Optional items only after the above are complete

## Full Implementation Checklist

- [ ] Create the shared composite route preview helper in
      `web/src/strategies/compositeProfileGpuPolicy.ts`.
- [ ] Keep the preview scoped to composite route policy, not full runtime
      backend resolution.
- [ ] Preserve legacy backend aliases while centralizing requested-backend
      normalization.
- [ ] Resolve composite route preview before composite guardrails run in
      `web/src/services/optimizeRequestBuilder.ts`.
- [ ] Convert kernel-only guardrails to use the resolved route target.
- [ ] Convert `exportAllCsvPath` gating to use the resolved route target.
- [ ] Add named constants for heatmap, include-all, and kernel `topK`
      guardrails.
- [ ] Add frontend `topK <= 256` kernel-route preflight.
- [ ] Keep CPU heatmap support intact.
- [ ] Expand `StrategyOptimizationKind` with the missing double-MA overlay
      kinds.
- [ ] Replace repeated double-MA-family checks with a shared family helper.
- [ ] Add composite route inputs to
      `describeStrategyOptimizationCapabilities(...)`.
- [ ] Wire route inputs from `OptimizeWorkspacePanel.vue`.
- [ ] Wire route inputs from `AdvancedWorkspacePanel.vue`.
- [ ] Keep CPU composite-trio heatmap, `topK`, and `includeAll` capability
      support visible.
- [ ] Replace range-array expansion with arithmetic range helpers across all
      three builders.
- [ ] Reuse parsed ranges for `CompositeMo` domain construction.
- [ ] Replace full MA support-map cloning with a lookup-oriented helper.
- [ ] Canonicalize MA IDs once in the support-lookup path.
- [ ] Add capped multiplication for combo guardrails.
- [ ] Split common strategy parsing from kind-specific strategy shaping.
- [ ] Document the fixed `trade_on_next_bar: true` behavior.
- [ ] Add a fixed-true `tradeOnNext` regression test.
- [ ] Add route-target matrix tests for the composite trio.
- [ ] Add `exportAllCsvPath` route-target tests.
- [ ] Add `Auto` composite-route tests.
- [ ] Add capability tests for the missing overlay kinds.
- [ ] Add capability tests for composite `supportsExportAll`.
- [ ] Add capability tests that keep CPU heatmap support visible.
- [ ] Add optional precedence tests in
      `crates/app_core/src/composite_profile_route.rs`.
- [ ] Leave all optional items for last, and only after core correctness work is
      merged or at least fully implemented.
