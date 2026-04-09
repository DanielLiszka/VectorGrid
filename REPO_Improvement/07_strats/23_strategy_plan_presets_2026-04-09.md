## Purpose

Define the improvement plan for the preset-to-IR-to-plan layer that establishes representative non-options strategy configurations and compiles them into backend-aware execution plans.

## Scope Boundaries

In scope:

- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/double_ma_test_support.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- supporting route-matrix and profile-runner bins that exercise the preset layer

Out of scope:

- frontend strategy-form UX
- options-related presets
- inventing new strategy families
- changing product route policy beyond clarifying and testing the existing preset/plan mapping

## Current User-Facing Behavior

This layer is mostly indirect, but it affects visible system behavior in important ways:

- representative strategy cases used in tests and route probes
- which indicators are planned as CPU, `GpuKernel`, or `GpuSingle`
- whether dynamic strategy plans are compiled consistently for the current strategy family
- whether route-matrix and benchmark bins reflect actual supported plan shapes

If presets drift from real route policy, users can end up seeing route diagnostics, capability summaries, or benchmark outputs that no longer represent the app’s intended behavior.

## Current Code Surface

- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/double_ma_test_support.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`

Key current ownership:

- `strategy_presets.rs` defines representative kernel-ready configs such as trend-quality, regime-hybrid, breakout, VFCT, PIT, SET, MTS, ORC, and the combined `double_like_kernel_strategy_cases()`
- `strategy_ir.rs` converts strategy configuration into a normalized, validated IR
- `strategy_plan.rs` compiles the IR into backend-aware indicator plans, condition groups, signal bindings, and route decisions

## Current Verification Surface

- unit tests in `crates/app_core/src/strategy_ir.rs`
- unit tests in `crates/app_core/src/strategy_plan.rs`
- dynamic strategy/runtime tests in `crates/app_core/src/double_ma.rs`
- validation-side dynamic strategy coverage in `crates/app_core/src/double_ma_validate.rs`
- route/probe bins:
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
  - `crates/app_core/src/bin/stage6b_kernel_report.rs`
  - `src-tauri/src/bin/ta_profile_route_matrix.rs`

## Strengths Already Present

- preset ownership is centralized instead of being rebuilt ad hoc in each test
- IR validation is explicit before plan compilation
- route decisions are encoded and serialized rather than staying implicit in runtime branches
- the current layer already supports legacy indicator kinds and descriptor-driven indicators
- route-probe bins give this layer a practical integration surface beyond unit tests

## Weaknesses / Failure Modes

- presets are validated mostly indirectly, which makes drift easy to miss
- the preset layer is small, but the semantic importance is high because it feeds route policy and GPU readiness checks
- plan serialization/versioning is present, but contract expectations are not written down in one place
- a change in default-route logic can alter runtime behavior far away from where the change was made
- route overrides can become more permissive or restrictive without a clear audit trail

## Regressions To Avoid

- drifting preset configs away from the route/probe expectations used in the repo’s benchmarks and diagnostics
- changing plan serialization or route encoding without deliberate compatibility review
- breaking descriptor-driven indicator normalization while preserving only legacy cases
- changing representative cases so tests keep passing but stop representing real product policy

## External Research Summary

- [Serde enum representations](https://serde.rs/enum-representations.html) are relevant because `StrategyPlanBackend`, `IndicatorRouteKind`, and related compiled-plan enums are persisted and inspected across tooling surfaces.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the plan layer already carries schema-version fields and needs clearer compatibility expectations when serialized or emitted by tools.
- [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) are relevant because route/probe binaries are an important verification surface and should remain intentional targets, not accidental side artifacts.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because compiled-plan payloads and route diagnostics often flow through JSON-shaped output and need stable structure.
- [Criterion.rs getting started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because representative presets are useful benchmark fixtures and should remain stable enough to support performance comparisons across plan/runtime changes.

## Proposed Improvement Approach

1. Treat presets and compiled strategy plans as an executable contract layer.
2. Add a clear parity chain for each representative preset:
   - preset config
   - IR validation
   - compiled plan
   - route report / probe expectation
3. Document what makes a preset “representative” so new contributors do not mutate them casually.
4. Add golden-style tests for a small subset of serialized plan artifacts:
   - backend
   - indicator routes
   - condition/group counts
   - signal bindings
5. Keep route-override testing explicit so override behavior does not silently redefine default-route policy.
6. Use the route/probe bins as contract verifiers, not just as ad hoc diagnostics.

## Checklist

- [ ] Document the purpose and expected stability of every preset in `strategy_presets.rs`.
- [ ] Add or tighten preset-to-IR tests that validate representative condition trees and indicator references.
- [ ] Add or tighten preset-to-plan tests that assert backend and route decisions for each representative family.
- [ ] Add or tighten tests for descriptor-driven indicator normalization and output selection.
- [ ] Add or tighten tests for route overrides so override behavior is explicit and bounded.
- [ ] Decide whether a small set of serialized plan fixtures should be checked into tests as golden references.
- [ ] Audit route/probe binaries to ensure they consume the shared preset layer rather than duplicating assumptions.

## Verification Plan

- run targeted tests in `crates/app_core/src/strategy_ir.rs`
- run targeted tests in `crates/app_core/src/strategy_plan.rs`
- run relevant dynamic-strategy tests in `crates/app_core/src/double_ma.rs`
- run relevant validation-side plan consumers in `crates/app_core/src/double_ma_validate.rs`
- run route probe binaries when behavior around route mapping or preset coverage changes

## Rollout / Fallback / Compatibility Notes

- keep the first wave behavior-preserving and focused on parity, fixtures, and documentation
- avoid changing representative preset semantics until parity tests make the intended behavior explicit
- if a preset is no longer representative, replace it deliberately and update all probe/test consumers in the same change
- keep serialized-plan changes schema-aware and version-conscious

## Related Docs

- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/07_strats/25_strategy_runtime_gpu_stack_2026-04-09.md`
- `REPO_Improvement/03_opt/24_eng_strategy_ir_plan_2026-04-08.md`
