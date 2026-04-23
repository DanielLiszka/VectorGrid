# Dynamic Double-MA Strategy Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the dynamic Double-MA surface that compiles strategy IR into runnable plans, stages dynamic indicators, and conditionally preserves exact-GPU execution where product constraints allow it.

## Scope Boundaries

In scope:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `src-tauri/src/ma_registry.rs`

Out of scope:

- static Double-MA-only request paths already covered elsewhere
- CompositeCpu and CompositeMo strategy ownership
- options-related strategy work

## Current User-Facing Behavior

The repo already supports a dynamic Double-MA variant that sits on top of the broader Double-MA execution family. It can:

- carry a `dynamic_strategy_ir` in the Double-MA request
- resolve indicator descriptors and output references through the indicator catalog
- compile a dynamic plan into CPU and, when supported, GPU-backed execution
- run strict exact-GPU eligibility checks before allowing exact-kernel-only features
- emit route notes when dynamic execution forces a non-exact or staged path

This means dynamic Double-MA is already a real product surface, but one that is constrained by runtime-plan, indicator-route, and backend-support rules.

## Current Code Surface

- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `src-tauri/src/commands_dynamic.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`

## Current Verification Surface

- `web/src/strategies/dynamicDoubleMaBuilder.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- tests in `crates/app_core/src/strategy_dynamic.rs`
- tests in `crates/app_core/src/indicator_catalog.rs`
- tests in `crates/app_core/src/double_ma.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`

## Strengths Already Present

- dynamic plans already compile through a strategy-IR and compiled-plan layer instead of one-off condition handling
- the frontend already has a dedicated builder/preview surface instead of hand-editing raw IR
- indicator output validation already exists
- CPU fallback and lazy CPU plan/context preparation are already explicit
- exact-GPU restrictions are already guarded rather than implied
- route notes already disclose some dynamic execution limitations

## Weaknesses / Failure Modes

- the surface spans builder text, preview/command validation, indicator cataloging, GPU preparation, runtime-plan compilation, and route-note reporting
- backend GPU-v1 support is intentionally narrow: Double-MA profile only, plain crossover exits, and tightly constrained indicator slots/operands
- exact-GPU eligibility and product feature eligibility are easy to drift apart
- dynamic indicator support can widen without corresponding catalog/runtime-plan tests
- lazy CPU and cached GPU preparation introduce stateful complexity
- route notes are present, but the contract is distributed across several layers

## Regressions To Avoid

- allowing unsupported dynamic indicators or outputs into GPU-backed plans
- widening the GPU-v1 envelope accidentally through generic Double-MA changes
- silently degrading exact-GPU features without route-note coverage
- breaking CPU fallback plan/context generation
- widening the dynamic surface without catalog/runtime-plan verification
- letting frontend request shaping imply support that backend runtime plans do not actually provide
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [OnceLock in `std::sync`](https://doc.rust-lang.org/beta/std/sync/struct.OnceLock.html?search=std%3A%3Avec) is relevant because the dynamic execution path already depends on explicit lazy initialization and cache-like semantics that should stay disciplined.
- [rayon iterator docs](https://docs.rs/rayon/latest/rayon/iter/index.html) are relevant because dynamic and Double-MA execution surfaces depend on data-parallel evaluation patterns that must remain deterministic and race-free.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because the dynamic path only earns its complexity if it preserves coherent GPU execution and host/device staging discipline.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because dynamic IR and catalog metadata flow through structured JSON-like request/config payloads.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the dynamic request/catalog/runtime-plan boundary is effectively a schema contract even if not yet formalized as one.
- [Calling Rust from the frontend | Tauri](https://v2.tauri.app/develop/calling-rust/) is relevant because the dynamic builder/preview path already relies on a dedicated Tauri command boundary.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat dynamic Double-MA as a constrained product surface layered on top of the Double-MA family, not as a free-form generic strategy runtime.
2. Freeze the boundaries between:
   - frontend builder/preview
   - request IR
   - indicator catalog resolution
   - compiled runtime plan
   - exact-GPU eligibility
   - CPU fallback eligibility
3. Expand verification where product-facing exact-route features depend on dynamic plan capabilities.
4. Keep route-note and capability reporting tied to actual runtime-plan support.
5. Keep dynamic indicator growth catalog-driven and test-backed.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the end-to-end dynamic Double-MA pipeline from request IR to runtime plan.
- [x] Document the dedicated builder/preview path and its contract with backend validation.
- [x] Document exact-GPU eligibility vs CPU fallback semantics for dynamic plans.
- [x] Document catalog/output validation requirements for dynamic indicators.
- [x] Add or tighten tests for unsupported indicator/output rejection.
- [x] Add or tighten tests for dynamic route notes and exact-kernel feature gating.
- [x] Add or tighten tests for lazy CPU fallback plan/context behavior.
- [x] Add parity coverage between frontend request shaping and backend runtime-plan support.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `crates/app_core/src/double_ma.rs`
- run targeted tests in `crates/app_core/src/strategy_dynamic.rs`
- run targeted tests in `crates/app_core/src/indicator_catalog.rs`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- manually inspect one dynamic plan that stays exact-GPU-capable and one that falls back to a non-exact route
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current constrained product posture; do not silently widen dynamic support
- keep CPU fallback intact even if GPU support expands
- if new dynamic indicators are added, land catalog, runtime-plan, and route-note coverage together
- prefer additive capability reporting over hiding unsupported dynamic-plan features
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed this tranche without widening the dynamic surface. The existing preview-and-runtime contract in `strategy_dynamic.rs`, `double_ma.rs`, `commands_dynamic.rs`, and the frontend builder path already carried the real product semantics; this pass kept that contract explicit in the public repo instead of adding another runtime layer.
- Added contributor-facing truth anchors in `REPO_MAP.md` so `DynamicDoubleMa` is now described in one public-facing place as a constrained builder + preview surface that re-enters the normal Double-MA optimizer path and still intentionally lacks fixed-candidate validation replay.
- Revalidated the current dynamic guardrails rather than expanding them. The sampled backend/frontend evidence confirms preview-resolved CPU fallback, exact-route-only shortlist/trade-metric gates, and the current constrained GPU-v1 envelope all remain enforced at the request, preview, and runtime-plan layers.
- No dynamic indicator catalog or runtime-plan behavior was broadened. The outcome here is stronger contract evidence and contributor-facing clarity for the current constrained product surface, not a feature expansion.
- Acceptance evidence for closure: `cargo test -p ta_app_core preview_dynamic_double_ma_strategy_ir_keeps_gpu_backend_for_plain_v1_shape`, `cargo test -p ta_app_core request_dynamic_strategy_ir_cpu_composite_profiles_accept_trade_metrics`, `npm.cmd --prefix web run test -- src/strategies/dynamicDoubleMaBuilder.test.ts`, `npm.cmd --prefix web run test -- src/services/optimizeService.test.ts -t "applies preview-resolved CPU fallback before enforcing exact-kernel-only dynamic controls"`, `npm.cmd --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`, `npm.cmd --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/04_opt_dynamic_builder_2026-04-08.md`
- `REPO_Improvement/03_opt/23_eng_dynamic_indicators_2026-04-08.md`
- `REPO_Improvement/03_opt/24_eng_strategy_ir_plan_2026-04-08.md`
- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
