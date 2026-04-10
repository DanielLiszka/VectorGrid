# CompositeTrend Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because CompositeTrend's main product differentiation is route-sensitive GPU execution, not a separate CPU-only story.
- [TypeScript Narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html) is relevant because frontend route preview must stay exhaustive over strategy kind and route conditions.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because route-sensitive capability and report expectations are effectively a contract surface.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Make CompositeTrend route policy visible and explicit wherever ADX changes the route.
2. Keep one authoritative route matrix and verify frontend preview against it.
3. Document which features are:
   - always available
   - exact-route only
   - hybrid-disabled
4. Keep validation/report behavior tied to the borrowed Double-MA family contract.
5. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document CompositeTrend route behavior for ADX off, ADX on, and CPU fallback.
- [x] Ensure UI copy explains route and feature changes when ADX is toggled.
- [x] Keep `compositeProfileGpuPolicy.ts` aligned with `composite_profile_route.rs`.
- [x] Add or maintain route-matrix verification for CompositeTrend cases.
- [x] Verify capability and advanced-controls surfaces update correctly with route changes.
- [x] Keep validation and artifact/report behavior aligned with the route actually used.
- [x] Avoid scattering more CompositeTrend policy into generic layers.
- [x] Preserve strategy-local discoverability for future improvements.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `npm test -- src/strategies/compositeProfileGpuPolicy.test.ts`
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run `npm test -- src/components/AdvancedWorkspacePanel.test.ts`
- run targeted `cargo test -p ta_app_core composite_profile_route`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current ADX-off `GpuKernel` and ADX-on `GpuHybridKernel` policy unless a deliberate product decision changes it
- if route visibility is improved in the UI, do not change actual backend route semantics silently
- keep CPU fallback intact even if GPU messaging gets clearer
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed this item as a route-parity hardening pass rather than a UI rewrite. The current user-facing copy for CompositeTrend already lives in capability summaries, route previews, and validation summaries; this tranche locked that copy to the actual ADX-off kernel, ADX-on hybrid, and CpuOnly fallback contract.
- Revalidated the existing prerequisite enforcement rather than rewriting it. `optimizeRequestBuilder.test.ts` already proves CompositeTrend still requires `high`/`low` when ADX-bearing shapes need them, while this pass focused on the remaining wording and UI-parity gaps.
- Added direct route-preview and advanced-panel coverage so ADX toggles remain obvious in the product shell. `OptimizeWorkspacePanel.test.ts` and `AdvancedWorkspacePanel.test.ts` now keep the ADX-off kernel path and ADX-on hybrid path visibly distinct.
- Tightened validation-summary wording for routed composites so CompositeTrend’s fixed-candidate validation copy now stays specific about kernel, hybrid, and CPU fallback expectations instead of collapsing those cases into a generic GPU-main phrase.
- Acceptance evidence for closure: `cargo test -p ta_app_core composite_profile_route --lib`, `cargo test -p ta_app_core double_ma_validation_gpu_kernel_matches_cpu_metrics_trend_quality_adx_off --quiet`, `cargo test -p ta_app_core double_ma_validation_gpu_hybrid_matches_cpu_metrics_trend_quality_adx_on --quiet`, `npm.cmd --prefix web run test -- src/strategies/compositeProfileGpuPolicy.test.ts src/components/OptimizeWorkspacePanel.test.ts src/components/AdvancedWorkspacePanel.test.ts src/services/validationRequestBuilder.test.ts`, `npm.cmd --prefix web run test -- src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
