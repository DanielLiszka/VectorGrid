# Double-MA Family Plan

Last updated: 2026-04-08

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

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because this family is the main GPU-sensitive execution lane and should keep correctness, profiling, and host/device data-flow discipline explicit.
- [scikit-learn: TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because this family's value is not only faster optimization but trustworthy time-ordered validation.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the family already emits rich request/result/report payloads that benefit from explicit contract thinking.

## Proposed Improvement Approach

1. Write down the Double-MA family contract in one place:
   - supported routes
   - exact vs approximate modes
   - result guarantees
   - validation/report expectations
2. Separate family-level ownership from overlay-specific policy.
3. Keep exact evaluation as the reference surface for parity and fidelity checks.
4. Add or tighten family-level docs/tests whenever a new overlay, metric contract, or route policy is added.

## Checklist

- [ ] Document what belongs to the Double-MA family vs overlay-specific policy.
- [ ] Document exact-grid, approximate, CPU, hybrid, and kernel route expectations.
- [ ] Clarify which result/report surfaces are guaranteed across the whole family.
- [ ] Keep validation compatibility and artifact/report behavior tied to the family contract.
- [ ] Add references to the fidelity audit harness wherever approximate paths are discussed.
- [ ] Review whether `double_ma.rs` should be split further by ownership area without losing discoverability.
- [ ] Keep frontend request-shaping and backend route behavior aligned.
- [ ] Avoid introducing new hidden special cases into the family without tests.

## Verification Plan

- run `cargo test -p ta_app_core`
- run targeted Double-MA optimization and validation tests
- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run `npm test -- src/services/validationRequestBuilder.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve the current exact path as the truth anchor
- prefer strategy-local docs and helpers over adding more generic abstraction first
- if family ownership is refactored, keep route/report behavior stable and test-covered

## Related Docs

- `REPO_Improvement/03_opt/19_eng_dm_backend_routing_2026-04-08.md`
- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`
- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
