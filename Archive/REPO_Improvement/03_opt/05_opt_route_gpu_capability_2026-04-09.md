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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because route capability ultimately becomes a backend-to-frontend contract.
- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU route claims should stay grounded in verification, profiling, and correct host-device distinctions rather than marketing shorthand.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because capability prose should derive from canonical state and backend evidence, not component-local assumptions.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because route metadata fields behave like a structured compatibility contract.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document one route-capability matrix tagged by authority: frontend advisory, request-builder legality, and backend result truth.
2. Centralize route labels and backend alias handling where possible.
3. Treat backend route metadata as the authoritative source for exported diagnostics and reporting.
4. Add parity fixtures comparing frontend expectations with backend route outcomes.
5. Make export, shortlist, and metric-surface promises explicitly route-dependent.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the route-capability matrix and authority layers.
- [x] Document composite-profile route policy including ADX-sensitive branches.
- [x] Document export and metric-surface capability by route.
- [x] Add or tighten parity tests between frontend capability text and backend route results.
- [x] Document fallback reason handling as a first-class UX contract.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run targeted Rust route and result-contract tests
- run targeted validation report tests if route metadata shapes change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current backend result fields until all consumers migrate together
- prefer additive route metadata over silent reinterpretation of existing labels
- keep frontend capability prose conservative if there is any uncertainty
- if a route loses capability, surface the fallback reason explicitly instead of hiding it
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the current route-capability matrix instead of broadening it. `strategyOptimizationCapabilities.test.ts` and `compositeProfileGpuPolicy.test.ts` already prove the current authority split: frontend capability summaries are advisory, composite-route previews are ADX-sensitive, export-all is disabled on hybrid or CPU routes, and metric-surface availability remains route-dependent rather than generic “GPU” marketing.
- Added a direct parity test in `strategyOptimizationCapabilities.test.ts` tying advisory capability flags to real request-builder behavior. The new matrix verifies that `supportsExportAll` agrees with `buildDoubleMaOptimizationRequest()` acceptance for representative kernel, hybrid, and CPU composite routes, so capability prose and request legality no longer drift as separate test islands.
- Revalidated request-builder truth and export legality through `optimizeRequestBuilder.test.ts`, including Auto-route normalization for the composite trio and rejection of export-all on hybrid or CPU fallback routes. Revalidated panel-facing route text through `OptimizeWorkspacePanel.test.ts`, which exercises the hybrid composite route preview and legacy backend alias normalization.
- Kept backend-route truth grounded in the existing Rust and service surface. `optimizeService.test.ts` continues to verify parity metadata and surfaced fallback reasons, while the repo already contains backend route-diagnostics coverage in the double-MA and composite Rust modules for deeper route behavior.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts src/strategies/compositeProfileGpuPolicy.test.ts src/components/OptimizeWorkspacePanel.test.ts src/services/optimizeService.test.ts src/services/optimizeRequestBuilder.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
- `REPO_Improvement/05_validation/08_validation_route_display_2026-04-08.md`
- `REPO_Improvement/04_results/09_result_surface_contract_2026-04-08.md`
