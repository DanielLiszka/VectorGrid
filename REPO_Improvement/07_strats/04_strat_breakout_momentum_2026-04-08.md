# BreakoutMomentum Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for `BreakoutMomentum` as a strategy-local non-options surface that currently routes through the Double-MA family while carrying its own data prerequisites, ADX-sensitive GPU route behavior, and validation/report expectations.

## Scope Boundaries

In scope:

- BreakoutMomentum route policy and data requirements
- strategy-specific frontend request shaping and capability exposure
- ADX-sensitive route changes
- validation/report parity for BreakoutMomentum

Out of scope:

- CompositeTrend and RegimeHybrid specifics
- standalone composite CPU families
- options workflows

## Current User-Facing Behavior

BreakoutMomentum appears as its own strategy kind in the optimize and validation flows, but it reuses the Double-MA execution family. When ADX is off, the target route is `GpuKernel`; when ADX is on, the route becomes `GpuHybridKernel`; and `CpuOnly` remains available as a fallback or explicit request. Unlike simpler MA-only profiles, BreakoutMomentum also depends on high/low/volume-aware inputs.

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

## Current Verification Surface

- `web/src/strategies/compositeProfileGpuPolicy.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- tests in `crates/app_core/src/composite_profile_route.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

## Strengths Already Present

- BreakoutMomentum already has an explicit route policy instead of an opaque generic fallback
- the strategy benefits from the Double-MA family rather than duplicating a full execution stack
- frontend route preview and backend route selection already exist
- there are route-evidence helpers for GPU/path verification

## Weaknesses / Failure Modes

- BreakoutMomentum ownership is split across strategy UI, route-policy helpers, Double-MA execution, and validation surfaces
- ADX toggles can silently change both route and advanced-control availability
- the strategy depends on high/low/volume support, which is easy to under-document
- frontend preview and backend truth can drift if one side changes first

## Regressions To Avoid

- showing full kernel support when ADX or data prerequisites force hybrid or CPU behavior
- hiding explicit CPU fallback behavior from users who request it
- weakening high/low/volume prerequisite handling
- changing route policy without updating both preview helpers and route-evidence tooling
- losing validation/report parity while improving optimization copy

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because BreakoutMomentum’s product posture depends on truthful GPU-route ownership and host/device boundary discipline.
- [Polygon flat-file minute aggregates](https://massive.com/docs/flat-files/stocks/minute-aggregates) is relevant because the strategy depends on high/low/volume-style market data fields.
- [TypeScript narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html) is relevant because frontend route preview should stay explicit and exhaustive over strategy/ADX/data-condition branches.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because preview and actual execution are separated by a command boundary.

## Proposed Improvement Approach

1. Keep BreakoutMomentum strategy-local and document it as a routed Double-MA family member instead of a standalone optimizer stack.
2. Make data prerequisites explicit alongside route policy so users understand why the strategy is not interchangeable with pure MA profiles.
3. Keep one authoritative route matrix and verify frontend preview against backend runtime selection.
4. Document which capabilities are:
   - always available
   - route-sensitive
   - disabled by ADX or missing data prerequisites
5. Keep validation and artifact/report behavior tied to the route actually used.

## Checklist

- [ ] Document BreakoutMomentum route behavior for ADX off, ADX on, and CPU fallback.
- [ ] Document high/low/volume prerequisites and how they affect availability.
- [ ] Ensure frontend route preview stays aligned with `composite_profile_route.rs`.
- [ ] Keep advanced-control and export messaging route-sensitive.
- [ ] Add or maintain route-evidence coverage for BreakoutMomentum cases.
- [ ] Verify validation summaries and route notes match the runtime route actually used.
- [ ] Preserve strategy-local ownership instead of hiding rules in generic helpers.
- [ ] Avoid adding new implicit special cases without tests and route evidence.

## Verification Plan

- run `npm test -- src/strategies/compositeProfileGpuPolicy.test.ts`
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/validationRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run targeted `cargo test -p ta_app_core composite_profile_route`
- run the route-matrix/profile helper when route semantics change materially

## Rollout / Fallback / Compatibility Notes

- preserve the current ADX-off `GpuKernel`, ADX-on `GpuHybridKernel`, and CPU fallback posture unless product policy changes deliberately
- if route visibility improves in the UI, keep actual backend route semantics unchanged unless explicitly documented
- preserve existing data-prerequisite checks rather than hiding them behind generic unsupported-state messaging

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_DATE.md`
