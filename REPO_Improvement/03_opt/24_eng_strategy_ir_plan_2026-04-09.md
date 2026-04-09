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

- [ ] Document supported schema versions and compatibility behavior.
- [ ] Document static profiles and graph-size limits.
- [ ] Document CPU versus GPU-unified plan compilation behavior.
- [ ] Document the reduced GPU-v1 subset and its purpose.
- [ ] Add or tighten parity tests across builder, preview, and plan compilation.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

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

## Related Docs

- `REPO_Improvement/03_opt/04_opt_dynamic_builder_2026-04-09.md`
- `REPO_Improvement/03_opt/23_eng_dynamic_indicators_2026-04-09.md`
- `REPO_Improvement/03_opt/30_dynamic_indicator_catalog_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
