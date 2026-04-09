# CompositeTrend Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for `CompositeTrend` as a strategy-local surface that currently routes through the Double-MA family while exposing route-sensitive GPU policy and validation behavior.

## Scope Boundaries

In scope:

- CompositeTrend route policy
- CompositeTrend-specific frontend request shaping and capability exposure
- ADX-sensitive route changes
- validation/report parity for CompositeTrend

Out of scope:

- RegimeHybrid and BreakoutMomentum specifics
- standalone composite CPU families
- options workflows

## Current User-Facing Behavior

CompositeTrend is presented as its own strategy kind, but it reuses the Double-MA optimization command path. When ADX is off, the product target route is `GpuKernel`; when ADX is on, the route becomes `GpuHybridKernel`; and `CpuOnly` remains the fallback/requested CPU path. These route choices affect export support and advanced exact-kernel shortlist controls.

## Current Code Surface

- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/strategies/compositeProfileGpuPolicy.test.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`

## Current Verification Surface

- `web/src/strategies/compositeProfileGpuPolicy.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/components/AdvancedWorkspacePanel.test.ts`
- tests in `crates/app_core/src/composite_profile_route.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`

## Strengths Already Present

- route policy already exists in both frontend preview and backend ownership code
- ADX-on vs ADX-off behavior is already explicit and tested
- CompositeTrend already benefits from the Double-MA family instead of owning a duplicated execution stack
- a route-matrix binary already exists for route verification evidence

## Weaknesses / Failure Modes

- CompositeTrend ownership is split across strategy UI, route-policy helpers, and Double-MA backend logic
- ADX toggles can silently change route and feature availability unless the UI makes that obvious
- export-all and exact-kernel shortlist availability are route-dependent and easy to miscommunicate
- backend route policy and frontend preview policy can drift if one side changes first

## Regressions To Avoid

- showing CompositeTrend as full-kernel-capable when ADX promotes it to hybrid
- hiding CPU fallback support from users who explicitly request it
- changing route policy without updating both route helpers and route-matrix verification
- losing validation/report parity when CompositeTrend borrows Double-MA surfaces

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because CompositeTrend's main product differentiation is route-sensitive GPU execution, not a separate CPU-only story.
- [TypeScript Narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html) is relevant because frontend route preview must stay exhaustive over strategy kind and route conditions.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because route-sensitive capability and report expectations are effectively a contract surface.

## Proposed Improvement Approach

1. Make CompositeTrend route policy visible and explicit wherever ADX changes the route.
2. Keep one authoritative route matrix and verify frontend preview against it.
3. Document which features are:
   - always available
   - exact-route only
   - hybrid-disabled
4. Keep validation/report behavior tied to the borrowed Double-MA family contract.

## Checklist

- [ ] Document CompositeTrend route behavior for ADX off, ADX on, and CPU fallback.
- [ ] Ensure UI copy explains route and feature changes when ADX is toggled.
- [ ] Keep `compositeProfileGpuPolicy.ts` aligned with `composite_profile_route.rs`.
- [ ] Add or maintain route-matrix verification for CompositeTrend cases.
- [ ] Verify capability and advanced-controls surfaces update correctly with route changes.
- [ ] Keep validation and artifact/report behavior aligned with the route actually used.
- [ ] Avoid scattering more CompositeTrend policy into generic layers.
- [ ] Preserve strategy-local discoverability for future improvements.

## Verification Plan

- run `npm test -- src/strategies/compositeProfileGpuPolicy.test.ts`
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run `npm test -- src/components/AdvancedWorkspacePanel.test.ts`
- run targeted `cargo test -p ta_app_core composite_profile_route`

## Rollout / Fallback / Compatibility Notes

- preserve current ADX-off `GpuKernel` and ADX-on `GpuHybridKernel` policy unless a deliberate product decision changes it
- if route visibility is improved in the UI, do not change actual backend route semantics silently
- keep CPU fallback intact even if GPU messaging gets clearer

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
