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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because route decisions ultimately become command-boundary contracts.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because selector state and capability summaries should derive from canonical store-backed rules.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because strategy and backend routing truth should remain centralized and reusable.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because request payloads and route metadata effectively act as structured contracts across layers.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Produce one explicit strategy-family-to-backend matrix and treat it as the authoritative human-readable reference.
2. Separate global-backend strategies from fixed-path or strategy-owned-backend strategies.
3. Centralize backend alias normalization and ensure all consumers use it.
4. Add parity checks between capability prose, request-builder routing, service dispatch, and Rust route results.
5. Keep advisory UI messaging subordinate to backend truth.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every strategy family and its optimization command path.
- [x] Document backend alias normalization rules and legacy-token compatibility.
- [x] Document which families respect the global backend selector and which do not.
- [x] Add selector-to-command parity tests where coverage is thin.
- [x] Document composite-profile route policy and export limitations clearly.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/stores/optimizeStore.actions.test.ts`
- run targeted Rust tests covering composite route policy if route behavior changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve legacy backend token compatibility at the boundary even if the UI hides those tokens
- prefer additive route metadata and parity tests over changing visible selector behavior first
- treat backend truth as authoritative if capability prose and backend execution disagree
- keep staged families explicit rather than half-hidden
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Kept the routing model unchanged and tightened the selector-to-command evidence instead. The existing service and request-builder suites already prove major family dispatch across the Double-MA, Composite CPU, and Composite MO paths, plus backend alias normalization and ADX-sensitive composite routing.
- Added two panel-layer tests in `web/src/components/OptimizeWorkspacePanel.test.ts` to close the remaining display-truth gaps. One proves a legacy `GpuOnly` backend value is normalized into the product-facing unified GPU messaging instead of resurfacing the stale token. The other proves `CompositeTrend` with ADX enabled renders the hybrid route preview and export limitation text that the backend policy implies.
- Reused the existing deeper routing tests rather than duplicating backend logic in the component layer. `optimizeRequestBuilder.test.ts`, `optimizeService.test.ts`, and `optimizeStore.actions.test.ts` remain the authority for command-path parity, while the panel tests now prove the optimize shell displays that routing truth coherently.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/OptimizeWorkspacePanel.test.ts src/stores/optimizeStore.actions.test.ts src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/03_opt/08_strategy_family_params_2026-04-09.md`
- `REPO_Improvement/03_opt/04_opt_dynamic_builder_2026-04-09.md`
- `REPO_Improvement/05_validation/08_validation_route_display_2026-04-08.md`
