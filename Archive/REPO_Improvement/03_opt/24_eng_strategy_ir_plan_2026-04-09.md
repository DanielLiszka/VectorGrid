# Strategy IR Plan Compilation Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to strategy IR validation and plan compilation so schema compatibility, static profiles, dynamic-indicator routing, and the reduced GPU-v1 subset remain explicit and stable.

## Scope Boundaries

In scope:

- `strategy_ir.rs` schema and validation rules
- `strategy_plan.rs` compilation and route planning
- dynamic preview and product-capability checks
- frontend builder parity with the accepted IR subset

Out of scope:

- non-IR strategy request surfaces
- backend indicator compute internals beyond plan-level implications
- options-related planning systems

## Current User-Facing Behavior

The Dynamic Double MA path already depends on a real strategy IR contract. The repo supports schema version 2 while still accepting version 1 as compatible within the supported range. The IR currently supports five static profiles, graph-size limits, dynamic-indicator references, and execution-policy embedding.

The backend already compiles IRs into plans that resolve to either CPU or GPU-unified execution. Those plans depend on route policy, indicator eligibility, and a deliberately reduced dynamic GPU v1 subset. That reduced subset only allows certain floating single-series outputs, bounded graph sizes, and other restrictions needed to keep the GPU-unified path product-safe.

The frontend builder already operates against that reality through the preview path, but the relationship between builder output, schema compatibility, and plan compilation still needs a dedicated implementation plan.

## Current Code Surface

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `src-tauri/src/commands_dynamic.rs`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `web/src/services/optimizeService.ts`

## Current Verification Surface

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `src-tauri/src/commands_dynamic.rs`
- `web/src/strategies/dynamicDoubleMaBuilder.test.ts`

## Strengths Already Present

- schema versioning and compatibility checks already exist
- static profile support is already explicit in the IR layer
- plan compilation already separates CPU and GPU-unified paths
- the reduced GPU-v1 subset is already enforced rather than implied
- frontend builder tests already exist for the constrained IR path

## Weaknesses / Failure Modes

- schema versioning, IR validation, and plan compilation are tightly coupled but not yet documented as one lifecycle
- builder output can drift from accepted backend IR if the contract is not frozen
- contributors can broaden GPU-v1 support accidentally by relaxing one layer but not the others
- route-policy changes can quietly change compiled-plan determinism
- there is no dedicated implementation plan for the reduced GPU-v1 subset

## Regressions To Avoid

- schema drift without explicit compatibility handling
- builder IR that preview accepts but plan compilation later rejects unexpectedly
- silent broadening or narrowing of the GPU-v1 subset
- breaking static-profile compatibility while editing dynamic-indicator rules
- weakening plan determinism through implicit route-policy changes
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the strategy IR is effectively a schema-governed contract.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because preview and formatting commands expose IR behavior across the desktop boundary.
- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because the GPU-v1 subset should stay tied to explicit capability constraints, not wishful generalization.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because frontend builder and preview state should remain subordinate to backend schema truth.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document the IR lifecycle from schema validation through plan compilation.
2. Freeze the current schema-version compatibility policy and static-profile set.
3. Document the reduced GPU-v1 subset as an explicit product contract.
4. Add parity checks between builder output, preview acceptance, and plan compilation.
5. Treat route-policy changes as compatibility-affecting plan changes, not incidental refactors.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document supported schema versions and compatibility behavior.
- [x] Document static profiles and graph-size limits.
- [x] Document CPU versus GPU-unified plan compilation behavior.
- [x] Document the reduced GPU-v1 subset and its purpose.
- [x] Add or tighten parity tests across builder, preview, and plan compilation.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted schema and validation tests in `crates/app_core/src/strategy_ir.rs`
- run targeted plan-compilation tests in `crates/app_core/src/strategy_plan.rs`
- run targeted preview and capability tests in `crates/app_core/src/strategy_dynamic.rs`
- run `cargo test -p ta_desktop_demo_app commands_dynamic`
- run `npm --prefix web run test -- src/strategies/dynamicDoubleMaBuilder.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current schema compatibility until an explicit version migration is introduced
- prefer additive preview detail over silently broadening the accepted subset
- keep builder constraints conservative if backend support is uncertain
- update builder, preview, and plan docs together when schema or route support changes
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the current IR lifecycle across schema validation, backend preview, and product-safe plan compilation. The existing `strategy_ir.rs`, `strategy_plan.rs`, and `strategy_dynamic.rs` tests still cover supported schema versions, static-profile builders, route-aware compilation, and the reduced GPU-v1 subset restrictions described in this plan.
- The command-boundary preview evidence remains current: `cargo test -p ta_desktop_demo_app commands_dynamic --lib` passed and continues to prove that JSON preview parsing and product preview behavior stay wired through the Tauri layer.
- No code change was required in this tranche because the current builder and backend preview/plan stack already implement the documented schema compatibility and reduced GPU-v1 subset. The remaining known gap is a true end-to-end parity test that feeds frontend-emitted IR through the backend preview/compile path in one assertion.

## Related Docs

- `REPO_Improvement/03_opt/04_opt_dynamic_builder_2026-04-09.md`
- `REPO_Improvement/03_opt/23_eng_dynamic_indicators_2026-04-09.md`
- `REPO_Improvement/03_opt/30_dynamic_indicator_catalog_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
