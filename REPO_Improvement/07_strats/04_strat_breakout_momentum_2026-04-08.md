# BreakoutMomentum Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because BreakoutMomentum’s product posture depends on truthful GPU-route ownership and host/device boundary discipline.
- [Polygon flat-file minute aggregates](https://massive.com/docs/flat-files/stocks/minute-aggregates) is relevant because the strategy depends on high/low/volume-style market data fields.
- [TypeScript narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html) is relevant because frontend route preview should stay explicit and exhaustive over strategy/ADX/data-condition branches.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because preview and actual execution are separated by a command boundary.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Keep BreakoutMomentum strategy-local and document it as a routed Double-MA family member instead of a standalone optimizer stack.
2. Make data prerequisites explicit alongside route policy so users understand why the strategy is not interchangeable with pure MA profiles.
3. Keep one authoritative route matrix and verify frontend preview against backend runtime selection.
4. Document which capabilities are:
   - always available
   - route-sensitive
   - disabled by ADX or missing data prerequisites
5. Keep validation and artifact/report behavior tied to the route actually used.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document BreakoutMomentum route behavior for ADX off, ADX on, and CPU fallback.
- [x] Document high/low/volume prerequisites and how they affect availability.
- [x] Ensure frontend route preview stays aligned with `composite_profile_route.rs`.
- [x] Keep advanced-control and export messaging route-sensitive.
- [x] Add or maintain route-evidence coverage for BreakoutMomentum cases.
- [x] Verify validation summaries and route notes match the runtime route actually used.
- [x] Preserve strategy-local ownership instead of hiding rules in generic helpers.
- [x] Avoid adding new implicit special cases without tests and route evidence.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `npm test -- src/strategies/compositeProfileGpuPolicy.test.ts`
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/validationRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run targeted `cargo test -p ta_app_core composite_profile_route`
- run the route-matrix/profile helper when route semantics change materially
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current ADX-off `GpuKernel`, ADX-on `GpuHybridKernel`, and CPU fallback posture unless product policy changes deliberately
- if route visibility improves in the UI, keep actual backend route semantics unchanged unless explicitly documented
- preserve existing data-prerequisite checks rather than hiding them behind generic unsupported-state messaging
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed this item by hardening the concrete BreakoutMomentum contract instead of altering the strategy surface. The route-preview and capability layers already treated BreakoutMomentum as a Double-MA-routed profile with ADX-sensitive kernel vs hybrid behavior; this tranche turned the remaining assumptions into explicit regression tests.
- Revalidated the existing high/low/volume prerequisite enforcement rather than replacing it. `optimizeRequestBuilder.test.ts` already keeps missing-volume and missing-high/low failure cases explicit; this pass focused on the remaining route-preview and report-summary drift risks.
- Added direct kernel-route UI coverage for BreakoutMomentum in `OptimizeWorkspacePanel.test.ts` and `AdvancedWorkspacePanel.test.ts`, so the ADX-off kernel path stays visibly different from hybrid/CPU fallback cases and exact-kernel controls cannot silently drift.
- Tightened validation-summary wording for routed composites so BreakoutMomentum ADX-on winners stay tied to `GpuHybridKernel` wording instead of collapsing into the plainer generic GPU-main wording used for exact-kernel cases.
- Acceptance evidence for closure: `cargo test -p ta_app_core composite_profile_route --lib`, `cargo test -p ta_app_core double_ma_validation_gpu_kernel_matches_cpu_metrics_breakout_momentum_adx_off --quiet`, `cargo test -p ta_app_core double_ma_validation_gpu_hybrid_matches_cpu_metrics_breakout_momentum_adx_on --quiet`, `npm.cmd --prefix web run test -- src/strategies/compositeProfileGpuPolicy.test.ts src/components/OptimizeWorkspacePanel.test.ts src/components/AdvancedWorkspacePanel.test.ts src/services/validationRequestBuilder.test.ts`, `npm.cmd --prefix web run test -- src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_2026-04-08.md`
