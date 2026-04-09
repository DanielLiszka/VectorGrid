# Optimization Strategy Backend Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the strategy-to-backend routing model so strategy selection, backend aliases, route previews, and command dispatch remain truthful across frontend and backend layers.

## Scope Boundaries

In scope:

- strategy selector behavior in `OptimizeWorkspacePanel.vue`
- backend capability summaries in `strategyOptimizationCapabilities.ts`
- frontend backend alias normalization
- route selection across Double-MA, Composite CPU, and Composite MO families

Out of scope:

- redesigning backend execution engines
- options-related strategy routing
- ML Signal strategy-family work

## Current User-Facing Behavior

The frontend already filters the conventional optimization strategy list and excludes `MlSignal` from this panel. It includes currently supported and staged strategy families, including newer families that are not yet fully surfaced everywhere.

Backend behavior is already family-dependent. The frontend normalizes legacy tokens like `GpuOnly` and `GpuHybridKernel` into the product-facing `GpuKernel` backend. Composite-profile strategies use ADX-aware route previews. Request building and service dispatch already split the product into the Double-MA path, Composite CPU path, and Composite MO path.

Users therefore already see route-aware behavior, but the contract is still spread across selector options, capability-summary prose, request-builder rules, and backend command selection.

## Current Code Surface

- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `crates/app_core/src/composite_profile_route.rs`

## Current Verification Surface

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `web/src/stores/optimizeStore.actions.test.ts`

## Strengths Already Present

- the repo already has route-aware frontend messaging rather than pretending all strategies use one backend
- backend alias normalization already protects the product from old tokens
- service dispatch already distinguishes major optimization families
- composite-profile routing already incorporates ADX-sensitive route policy
- request-builder tests already cover a substantial amount of legality logic

## Weaknesses / Failure Modes

- routing truth is distributed across several files and not frozen in one implementation plan
- staged strategy families can drift between selector exposure, capability summaries, and backend legality
- legacy backend aliases can reappear if normalization rules drift
- frontend capability text is advisory while Rust is authoritative, which makes parity drift a real risk
- there is no single strategy/backend matrix document for contributors

## Regressions To Avoid

- resurfacing legacy backend tokens in the product UI
- misrouting a strategy family to the wrong backend command
- letting frontend summaries promise a route the backend will reject
- losing ADX-sensitive route preview correctness for composite profiles
- coupling fixed-path strategies too tightly to the global backend selector

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because route decisions ultimately become command-boundary contracts.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because selector state and capability summaries should derive from canonical store-backed rules.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because strategy and backend routing truth should remain centralized and reusable.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because request payloads and route metadata effectively act as structured contracts across layers.

## Proposed Improvement Approach

1. Produce one explicit strategy-family-to-backend matrix and treat it as the authoritative human-readable reference.
2. Separate global-backend strategies from fixed-path or strategy-owned-backend strategies.
3. Centralize backend alias normalization and ensure all consumers use it.
4. Add parity checks between capability prose, request-builder routing, service dispatch, and Rust route results.
5. Keep advisory UI messaging subordinate to backend truth.

## Checklist

- [ ] Document every strategy family and its optimization command path.
- [ ] Document backend alias normalization rules and legacy-token compatibility.
- [ ] Document which families respect the global backend selector and which do not.
- [ ] Add selector-to-command parity tests where coverage is thin.
- [ ] Document composite-profile route policy and export limitations clearly.

## Verification Plan

- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/stores/optimizeStore.actions.test.ts`
- run targeted Rust tests covering composite route policy if route behavior changes

## Rollout / Fallback / Compatibility Notes

- preserve legacy backend token compatibility at the boundary even if the UI hides those tokens
- prefer additive route metadata and parity tests over changing visible selector behavior first
- treat backend truth as authoritative if capability prose and backend execution disagree
- keep staged families explicit rather than half-hidden

## Related Docs

- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/03_opt/08_strategy_family_params_2026-04-09.md`
- `REPO_Improvement/03_opt/04_opt_dynamic_builder_2026-04-09.md`
- `REPO_Improvement/05_validation/08_validation_route_display_2026-04-08.md`
