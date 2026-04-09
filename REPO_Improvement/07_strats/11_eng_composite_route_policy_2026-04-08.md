# Composite Route Policy Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for the composite profile route policy so frontend preview, backend execution selection, validation expectations, and route-evidence tooling all reflect one authoritative composite-route contract.

## Scope Boundaries

In scope:

- CompositeTrend, RegimeHybrid, and BreakoutMomentum route policy
- frontend route preview helpers
- backend route resolution helpers
- route-matrix evidence tooling
- route-sensitive capability/report behavior

Out of scope:

- detailed parameter tuning for each strategy
- standalone composite CPU families not using this policy
- options workflows

## Current User-Facing Behavior

Composite profile strategies present route-sensitive behavior based on strategy kind, requested backend, and ADX usage. The frontend normalizes route preview through `compositeProfileGpuPolicy.ts`, while the backend owns runtime route truth in `composite_profile_route.rs`. Additional route evidence exists in the `ta_profile_route_matrix` binary and related tests.

## Current Code Surface

- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`

## Current Verification Surface

- `web/src/strategies/compositeProfileGpuPolicy.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- tests in `crates/app_core/src/composite_profile_route.rs`
- route evidence from `src-tauri/src/bin/ta_profile_route_matrix.rs`

## Strengths Already Present

- the repo already has distinct frontend and backend route-policy helpers
- route policy is backed by tests instead of being implicit UI copy
- the route-matrix binary gives an evidence surface beyond unit tests
- the policy is already strategy-local rather than buried in a generic optimizer abstraction

## Weaknesses / Failure Modes

- frontend and backend policy still require deliberate synchronization
- route truth is split between preview helpers, capability summaries, and runtime execution
- report/export and advanced-controls behavior is route-sensitive but not owned by one explicit contract document
- route-matrix evidence exists but is not yet the obvious authority for future changes

## Regressions To Avoid

- frontend preview claiming a route the backend will not actually use
- backend route policy changing without updating feature/capability surfaces
- route evidence binaries drifting into optional background tooling that nobody checks
- turning route policy into opaque generic logic that hides strategy ownership

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because these route decisions materially change host/device execution behavior and should stay evidence-backed.
- [TypeScript Narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html) is relevant because frontend route-preview code should be exhaustive and explicit.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because route preview and actual execution are separated by a command boundary and need a stable shared contract.

## Proposed Improvement Approach

1. Declare one route-policy authority model:
   - backend runtime truth
   - frontend preview mirror
   - route-matrix evidence harness
2. Keep route-sensitive capability/report implications adjacent to route policy ownership.
3. Require route-policy changes to update:
   - backend helper
   - frontend helper
   - tests
   - route matrix or equivalent evidence
4. Keep the policy strategy-local and readable.

## Checklist

- [ ] Document the route contract for CompositeTrend, RegimeHybrid, and BreakoutMomentum.
- [ ] Define backend runtime route selection as the source of truth.
- [ ] Keep frontend preview and capability surfaces explicitly synchronized to that truth.
- [ ] Add or maintain route-matrix evidence for supported route combinations.
- [ ] Review route-sensitive validation and report behavior so it is not left implicit.
- [ ] Make route-change review require code, tests, and evidence updates together.
- [ ] Preserve strategy-local discoverability instead of broad generic routing layers.
- [ ] Avoid route-policy prose that can become stale without code/test linkage.

## Verification Plan

- run `npm test -- src/strategies/compositeProfileGpuPolicy.test.ts`
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run targeted `cargo test -p ta_app_core composite_profile_route`
- run the route-matrix binary when route semantics or performance claims change materially

## Rollout / Fallback / Compatibility Notes

- preserve current product route targets unless there is a deliberate policy change
- if route evidence becomes more formalized, keep the lightweight unit tests too
- do not hide CPU fallback behavior while improving GPU route clarity

## Related Docs

- `REPO_Improvement/07_strats/03_strat_composite_trend_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
