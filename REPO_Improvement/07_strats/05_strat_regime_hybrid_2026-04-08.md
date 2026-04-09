# RegimeHybrid Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for `RegimeHybrid` as a strategy-local non-options surface that currently routes through the Double-MA family, is hybrid-GPU-first in its current form, and needs explicit ownership for route, validation, and report behavior.

## Scope Boundaries

In scope:

- RegimeHybrid route policy
- strategy-specific frontend request shaping and capability exposure
- current hybrid-GPU target and CPU fallback
- validation/report parity for RegimeHybrid

Out of scope:

- CompositeTrend and BreakoutMomentum specifics
- speculative no-ADX future variants as current product scope
- options workflows

## Current User-Facing Behavior

RegimeHybrid is presented as its own strategy kind, but it currently reuses the Double-MA execution family. In its present ADX-bearing form, the target route is `GpuHybridKernel`, with `CpuOnly` remaining the fallback. Unlike CompositeTrend or BreakoutMomentum, the repo does not currently position RegimeHybrid as a normal `GpuKernel` strategy in the shipped product path.

## Current Code Surface

- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/strategies/compositeProfileGpuPolicy.test.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

## Current Verification Surface

- `web/src/strategies/compositeProfileGpuPolicy.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- tests in `crates/app_core/src/composite_profile_route.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

## Strengths Already Present

- RegimeHybrid already has an explicit product route target instead of accidental fallback behavior
- route preview and backend route-selection logic already exist
- the strategy can reuse the Double-MA family instead of duplicating infrastructure
- GPU-kernel flavor work exists on the backend, which keeps future route evidence grounded

## Weaknesses / Failure Modes

- it is easy to over-promise full-kernel support even though the current product target is hybrid plus CPU fallback
- strategy-local ownership is split across route-policy helpers, Double-MA execution, kernel flavors, and validation surfaces
- future no-ADX discussions could leak into current product semantics if the doc stays vague
- frontend preview and backend runtime truth can drift

## Regressions To Avoid

- presenting RegimeHybrid as full `GpuKernel` when the current shipped target is hybrid
- hiding CPU fallback behavior
- mixing future variant ideas into current capability messaging
- changing route semantics without route-evidence and validation-surface updates
- weakening current validation/report parity while pursuing route cleanup

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because RegimeHybrid’s route posture is a correctness-and-boundary problem, not just a UI label.
- [TypeScript narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html) is relevant because frontend preview logic should stay exhaustive over the current hybrid-only product rule.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because preview and runtime route selection are separated by a command boundary.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for keeping route-sensitive capability and result expectations contractual.

## Proposed Improvement Approach

1. Document RegimeHybrid as a routed Double-MA family strategy with a current hybrid-GPU target, not as a generic composite strategy.
2. Make the current product rule explicit:
   - `GpuHybridKernel` in the present ADX-bearing form
   - `CpuOnly` fallback
   - no normal `GpuKernel` promise today
3. Keep future no-ADX or alternate effective variants labeled as future-only so they do not contaminate shipped capability messaging.
4. Keep validation, artifact, and report behavior aligned with the actual route used.

## Checklist

- [ ] Document the current RegimeHybrid route contract and its limits.
- [ ] Ensure frontend preview matches backend runtime truth for hybrid vs CPU fallback.
- [ ] Keep future no-ADX/full-kernel ideas out of the current shipped contract.
- [ ] Verify validation summaries and route notes match the runtime path.
- [ ] Maintain route-evidence tooling for RegimeHybrid cases.
- [ ] Preserve strategy-local discoverability across route helpers and presets.
- [ ] Add or maintain tests when route semantics or kernel flavors change.
- [ ] Avoid generic capability copy that over-promises support.

## Verification Plan

- run `npm test -- src/strategies/compositeProfileGpuPolicy.test.ts`
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/validationRequestBuilder.test.ts`
- run targeted `cargo test -p ta_app_core composite_profile_route`
- run route/profile evidence helpers when RegimeHybrid route behavior changes

## Rollout / Fallback / Compatibility Notes

- preserve the current hybrid-first plus CPU-fallback contract unless a deliberate route policy change lands
- keep any future no-ADX/full-kernel exploration documented separately from the current strategy contract
- if UI messaging gets sharper, do not silently change backend semantics at the same time

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_DATE.md`
