# Optimization Route GPU Capability Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to route-capability truthfulness so frontend GPU capability messaging, backend route diagnostics, and export or metric-surface promises remain aligned.

## Scope Boundaries

In scope:

- frontend capability summaries and route notes
- backend route and fallback metadata
- composite-profile route policy and export distinctions
- route truth for metric-surface and shortlist capabilities

Out of scope:

- kernel implementation optimization
- non-optimization GPU features
- options-related GPU routing

## Current User-Facing Behavior

The frontend already explains route capability in strategy-aware prose. Users see whether a selected strategy can use the global backend selector, whether export or exact shortlist controls are available, and how composite-profile ADX settings affect route previews.

Rust remains the true route authority. Backend results already carry route fields such as backend route, fallback reason, route diagnostics, indicator route plan, residency information, and metric-surface metadata. The composite trio already distinguishes pure kernel versus hybrid or CPU fallback behavior, and export capability depends on the exact kernel-capable route rather than GPU wording alone.

The current product therefore already has a real GPU-capability model, but it is split between advisory frontend text and authoritative backend metadata.

## Current Code Surface

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/app_core/src/composite_profile_route.rs`

## Current Verification Surface

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `crates/app_core/src/double_ma_backend.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`

## Strengths Already Present

- route diagnostics already exist in backend result contracts
- frontend already surfaces route-aware capability prose instead of generic GPU hype
- composite-profile route policy is already explicit and ADX-sensitive
- export capability is already tied to actual route conditions
- metric-surface support is already called out as part of capability summaries

## Weaknesses / Failure Modes

- frontend capability prose can drift from backend truth
- route labels and alias rules are distributed across several frontend and backend files
- hybrid routes are easy to over-describe as “GPU” if the exact restrictions are not frozen
- there is no single capability matrix distinguishing advisory UI text from backend authority
- route-note drift can become user-facing confusion in results and validation flows

## Regressions To Avoid

- promising export support on hybrid or CPU fallback routes
- hiding backend fallback reasons from the product
- treating advisory capability prose as more authoritative than Rust route results
- losing route metadata needed by validation or reporting surfaces
- weakening composite-profile ADX route truthfulness

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because route capability ultimately becomes a backend-to-frontend contract.
- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU route claims should stay grounded in verification, profiling, and correct host-device distinctions rather than marketing shorthand.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because capability prose should derive from canonical state and backend evidence, not component-local assumptions.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because route metadata fields behave like a structured compatibility contract.

## Proposed Improvement Approach

1. Document one route-capability matrix tagged by authority: frontend advisory, request-builder legality, and backend result truth.
2. Centralize route labels and backend alias handling where possible.
3. Treat backend route metadata as the authoritative source for exported diagnostics and reporting.
4. Add parity fixtures comparing frontend expectations with backend route outcomes.
5. Make export, shortlist, and metric-surface promises explicitly route-dependent.

## Checklist

- [ ] Document the route-capability matrix and authority layers.
- [ ] Document composite-profile route policy including ADX-sensitive branches.
- [ ] Document export and metric-surface capability by route.
- [ ] Add or tighten parity tests between frontend capability text and backend route results.
- [ ] Document fallback reason handling as a first-class UX contract.

## Verification Plan

- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run targeted Rust route and result-contract tests
- run targeted validation report tests if route metadata shapes change

## Rollout / Fallback / Compatibility Notes

- preserve current backend result fields until all consumers migrate together
- prefer additive route metadata over silent reinterpretation of existing labels
- keep frontend capability prose conservative if there is any uncertainty
- if a route loses capability, surface the fallback reason explicitly instead of hiding it

## Related Docs

- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
- `REPO_Improvement/05_validation/08_validation_route_display_2026-04-08.md`
- `REPO_Improvement/04_results/09_result_surface_contract_2026-04-08.md`
