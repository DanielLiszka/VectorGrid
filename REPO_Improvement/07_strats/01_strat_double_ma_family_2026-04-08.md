# Double-MA Family Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the Double-MA strategy family as the central non-options execution family spanning exact optimization, route-aware GPU execution, validation, and artifact/report integration.

## Scope Boundaries

In scope:

- core Double-MA optimization and validation surfaces
- Double-MA overlays that share the same execution family
- exact vs approximate search posture
- backend-route ownership
- validation/report/artifact expectations for this family

Out of scope:

- standalone composite CPU families
- Composite MO
- options workflows

## Current User-Facing Behavior

The Double-MA family is the main optimization lane. It exposes CPU, Auto, and GPU-oriented execution, top-k/include-all outputs, exact-grid and approximate modes, validation follow-through, and the largest share of saved optimization/report artifacts. Several overlay strategies also route through this family instead of owning fully separate optimization stacks.

## Current Code Surface

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/double_ma_strategy.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `crates/app_core/src/strategy_presets.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

## Current Verification Surface

- tests embedded in `crates/app_core/src/double_ma.rs`
- tests embedded in `crates/app_core/src/double_ma_validate.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs` tests

## Strengths Already Present

- the repo already treats Double-MA as a first-class execution family rather than a toy example
- optimization, validation, and result/report surfaces are already connected
- the family supports both exact and cheaper search paths, which enables evidence-driven comparison
- many newer composite profile routes already reuse this family rather than duplicating infrastructure

## Weaknesses / Failure Modes

- `double_ma.rs` is a very large ownership surface with route policy, search logic, scoring, result shaping, and GPU coordination mixed together
- it is easy to blur "exact truth surface" and "production default route" in documentation and UI
- overlay strategies that reuse this family can hide where strategy-specific policy actually lives
- validation expectations, metric-surface contracts, and artifact/report guarantees are spread across multiple files

## Regressions To Avoid

- breaking the exact-grid anchor while improving approximate or GPU paths
- moving strategy-family rules into generic layers that make ownership harder to follow
- changing backend-route semantics without updating validation, artifact, and capability surfaces
- letting overlay strategies drift from the family contract they rely on
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because this family is the main GPU-sensitive execution lane and should keep correctness, profiling, and host/device data-flow discipline explicit.
- [scikit-learn: TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because this family's value is not only faster optimization but trustworthy time-ordered validation.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the family already emits rich request/result/report payloads that benefit from explicit contract thinking.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Write down the Double-MA family contract in one place:
   - supported routes
   - exact vs approximate modes
   - result guarantees
   - validation/report expectations
2. Separate family-level ownership from overlay-specific policy.
3. Keep exact evaluation as the reference surface for parity and fidelity checks.
4. Add or tighten family-level docs/tests whenever a new overlay, metric contract, or route policy is added.
5. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document what belongs to the Double-MA family vs overlay-specific policy.
- [x] Document exact-grid, approximate, CPU, hybrid, and kernel route expectations.
- [x] Clarify which result/report surfaces are guaranteed across the whole family.
- [x] Keep validation compatibility and artifact/report behavior tied to the family contract.
- [x] Add references to the fidelity audit harness wherever approximate paths are discussed.
- [x] Review whether `double_ma.rs` should be split further by ownership area without losing discoverability.
- [x] Keep frontend request-shaping and backend route behavior aligned.
- [x] Avoid introducing new hidden special cases into the family without tests.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `cargo test -p ta_app_core`
- run targeted Double-MA optimization and validation tests
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run `npm test -- src/services/validationRequestBuilder.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current exact path as the truth anchor
- prefer strategy-local docs and helpers over adding more generic abstraction first
- if family ownership is refactored, keep route/report behavior stable and test-covered
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Kept this family slice additive and evidence-first. No Double-MA family refactor landed here; instead, the family contract is now explicitly reinforced by request-builder, capability-summary, validation-summary, and backend route-helper tests that cover the routed overlay strategies using the family.
- Tightened frontend/backend alignment for the family’s routed composite members. `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` now have locked parity around CPU fallback summaries, hybrid-vs-kernel route messaging, and composite route-preview expectations across `optimizeRequestBuilder`, `validationRequestBuilder`, `strategyOptimizationCapabilities`, and `compositeProfileGpuPolicy`.
- Added a backend helper regression in `crates/app_core/src/composite_profile_route.rs` proving the default family surface stays on the `CompositeCpu`/`CpuOnly` label path unless explicit overlay flags promote it, which protects the family’s baseline route vocabulary for future contributors.
- Kept the exact-anchor posture intact. This tranche did not widen approximate behavior or route semantics; it only hardened the contract around which family members retain exact-kernel controls, which ones stay hybrid or CPU-routed, and how those facts are surfaced.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/services/optimizeRequestBuilder.test.ts src/utils/strategyOptimizationCapabilities.test.ts src/services/validationRequestBuilder.test.ts src/strategies/compositeProfileGpuPolicy.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, `cargo test -p ta_app_core composite_profile_route --lib`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/03_opt/19_eng_dm_backend_routing_2026-04-08.md`
- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`
- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
