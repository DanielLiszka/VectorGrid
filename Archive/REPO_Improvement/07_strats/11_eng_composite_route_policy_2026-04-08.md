# Composite Route Policy Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because these route decisions materially change host/device execution behavior and should stay evidence-backed.
- [TypeScript Narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html) is relevant because frontend route-preview code should be exhaustive and explicit.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because route preview and actual execution are separated by a command boundary and need a stable shared contract.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

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
5. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the route contract for CompositeTrend, RegimeHybrid, and BreakoutMomentum.
- [x] Define backend runtime route selection as the source of truth.
- [x] Keep frontend preview and capability surfaces explicitly synchronized to that truth.
- [x] Add or maintain route-matrix evidence for supported route combinations.
- [x] Review route-sensitive validation and report behavior so it is not left implicit.
- [x] Make route-change review require code, tests, and evidence updates together.
- [x] Preserve strategy-local discoverability instead of broad generic routing layers.
- [x] Avoid route-policy prose that can become stale without code/test linkage.
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
- run targeted `cargo test -p ta_app_core composite_profile_route`
- run the route-matrix binary when route semantics or performance claims change materially
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current product route targets unless there is a deliberate policy change
- if route evidence becomes more formalized, keep the lightweight unit tests too
- do not hide CPU fallback behavior while improving GPU route clarity
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed this shared policy slice by tightening the route contract in the code and the user-facing preview together. `web/src/components/OptimizeWorkspacePanel.vue` now reflects the exact route-sensitive feature policy for CPU, hybrid GPU, and kernel GPU composite paths, while the route-policy helper and capability tests lock the same distinctions in code.
- Added route-policy evidence in `web/src/strategies/compositeProfileGpuPolicy.test.ts`, `web/src/utils/strategyOptimizationCapabilities.test.ts`, `web/src/components/OptimizeWorkspacePanel.test.ts`, and `crates/app_core/src/composite_profile_route.rs` so frontend preview, capability copy, and backend target labels all assert the same current route matrix.
- This gives the composite-route contract one practical authority chain: backend target labels as runtime truth, mirrored by frontend route-preview/capability helpers, and enforced by tests on both sides.
- Acceptance evidence captured for this closure: `npm.cmd --prefix web run test -- src/strategies/compositeProfileGpuPolicy.test.ts src/utils/strategyOptimizationCapabilities.test.ts src/components/OptimizeWorkspacePanel.test.ts`, `cargo test -p ta_app_core composite_profile_route --lib`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/07_strats/03_strat_composite_trend_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
