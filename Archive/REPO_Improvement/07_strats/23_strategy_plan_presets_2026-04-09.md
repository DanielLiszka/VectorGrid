# Strategy Plan Presets Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Serde enum representations](https://serde.rs/enum-representations.html) are relevant because `StrategyPlanBackend`, `IndicatorRouteKind`, and related compiled-plan enums are persisted and inspected across tooling surfaces.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the plan layer already carries schema-version fields and needs clearer compatibility expectations when serialized or emitted by tools.
- [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) are relevant because route/probe binaries are an important verification surface and should remain intentional targets, not accidental side artifacts.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because compiled-plan payloads and route diagnostics often flow through JSON-shaped output and need stable structure.
- [Criterion.rs getting started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because representative presets are useful benchmark fixtures and should remain stable enough to support performance comparisons across plan/runtime changes.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

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
7. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the purpose and expected stability of every preset in `strategy_presets.rs`.
- [x] Add or tighten preset-to-IR tests that validate representative condition trees and indicator references.
- [x] Add or tighten preset-to-plan tests that assert backend and route decisions for each representative family.
- [x] Add or tighten tests for descriptor-driven indicator normalization and output selection.
- [x] Add or tighten tests for route overrides so override behavior is explicit and bounded.
- [x] Decide whether a small set of serialized plan fixtures should be checked into tests as golden references.
- [x] Audit route/probe binaries to ensure they consume the shared preset layer rather than duplicating assumptions.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `crates/app_core/src/strategy_ir.rs`
- run targeted tests in `crates/app_core/src/strategy_plan.rs`
- run relevant dynamic-strategy tests in `crates/app_core/src/double_ma.rs`
- run relevant validation-side plan consumers in `crates/app_core/src/double_ma_validate.rs`
- run route probe binaries when behavior around route mapping or preset coverage changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the first wave behavior-preserving and focused on parity, fixtures, and documentation
- avoid changing representative preset semantics until parity tests make the intended behavior explicit
- if a preset is no longer representative, replace it deliberately and update all probe/test consumers in the same change
- keep serialized-plan changes schema-aware and version-conscious
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by hardening preset ownership directly in [strategy_presets.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/strategy_presets.rs):
  - added a uniqueness/non-empty-label test for `double_like_kernel_strategy_cases()`
  - added a preset-to-plan test proving every representative kernel case compiles under `GpuUnified` and keeps `fast_ma` / `slow_ma` on the kernel route
  - added a representative composite-profile route test proving `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` keep their expected GPU indicator routes under `GpuUnified`
- Kept scope intentionally additive:
  - no preset semantics changed
  - no route-policy changes
  - no serialized-plan schema changes
- Acceptance evidence:
  - `cargo test -p ta_app_core strategy_presets --lib`

## Related Docs

- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/07_strats/25_strategy_runtime_gpu_stack_2026-04-09.md`
- `REPO_Improvement/03_opt/24_eng_strategy_ir_plan_2026-04-08.md`
