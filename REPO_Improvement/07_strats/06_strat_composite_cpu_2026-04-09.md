# CompositeCpu Strategy Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for `CompositeCpu` as the broad composite strategy surface that still owns many signal families, route diagnostics, and the currently implemented composite drilldown/local-stability follow-up behaviors.

## Scope Boundaries

In scope:

- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `src-tauri/src/commands_optimize.rs`
- CompositeCpu request, route, drilldown, and strategy-family handling

Out of scope:

- product-profile routes that intentionally present as `CompositeTrend`, `RegimeHybrid`, or `BreakoutMomentum`
- CompositeMo search behavior
- options-related strategies

## Current User-Facing Behavior

`CompositeCpu` remains the large strategy-local backend for many non-options composite profiles and indicator-gated strategy families. It owns:

- a large `CompositeCpuRequest` shape
- per-family signal resolution and validation
- route diagnostics for several profile families
- optional GPU prescan inputs
- drilldown and local-stability flows for the currently implemented family subset, with some family-specific followups still intentionally unavailable
- shortlist slice diagnostics and route-note carry-through

Even where some product-facing strategies now route through other visible strategy kinds, `CompositeCpu` still carries substantial strategy-local execution ownership.

## Current Code Surface

- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `src-tauri/src/commands_optimize.rs`

## Current Verification Surface

- tests in `crates/app_core/src/composite_cpu.rs`
- tests in `crates/app_core/src/composite_profile_route.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- route-matrix and optimization command tests in `src-tauri`
- `crates/app_core/benches/composite_cpu_opt.rs`

## Strengths Already Present

- CompositeCpu is already strategy-local instead of hidden inside a generic execution layer
- family-specific validation and route diagnostics already exist
- drilldown and local-stability outputs are already owned close to execution
- optional prescan and route-diagnostic behavior are explicit in the request/result surface
- GPU route awareness already exists alongside CPU fallback behavior
- the shared stream aggregator and engine heatmap support are already available under the hood for this surface

## Weaknesses / Failure Modes

- the file is extremely large and spans many families, modes, and optional behaviors
- strategy-family behavior can drift because the same surface owns many profile types
- route-diagnostic handling is repeated across families
- prescan/search/drilldown/local-stability concerns all meet in one backend
- frontend product capability text can under-describe backend engine behavior such as internal heatmap-capable paths
- frontend request shaping can obscure which parts are truly CompositeCpu-specific

## Regressions To Avoid

- pushing more CompositeCpu ownership into generic helpers where strategy-local behavior becomes harder to find
- breaking route diagnostics or shortlist-slice diagnostics for any supported family
- losing drilldown/local-stability parity with optimization outputs
- changing prescan behavior without clear request/result documentation
- regressing CPU fallback just because GPU-aware routes are improved
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [rayon iterator docs](https://docs.rs/rayon/latest/rayon/iter/index.html) are relevant because CompositeCpu evaluation remains a large data-parallel execution surface with many candidate loops.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because CompositeCpu increasingly interacts with GPU-aware prescan and mixed-route behavior even when CPU remains authoritative.
- [OnceLock in `std::sync`](https://doc.rust-lang.org/beta/std/sync/struct.OnceLock.html?search=std%3A%3Avec) is relevant because large strategy surfaces often accumulate caching/lazy-init behavior that must remain disciplined.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because CompositeCpu request and result surfaces bridge structured frontend/backend payloads.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because CompositeCpu effectively exposes a strategy-specific request/result contract that should remain stable.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Keep CompositeCpu strategy-local and explicit rather than diluting ownership into generic layers.
2. Split documentation of the surface by responsibility:
   - request/config
   - route diagnostics
   - drilldown
   - local stability
   - prescan/search hints
3. Freeze which user-visible families still depend on CompositeCpu.
4. Add verification where family-specific route diagnostics and drilldown outputs are easy to regress.
5. Reduce conceptual sprawl through documentation and test matrices before any code reorganization is attempted.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the CompositeCpu request/result responsibilities by sub-surface.
- [x] Document which non-options families still depend on CompositeCpu behavior directly.
- [x] Document route-diagnostic and shortlist-slice diagnostic expectations.
- [x] Add or tighten tests for drilldown and local-stability parity.
- [x] Add or tighten tests for family-specific route diagnostics.
- [x] Add or tighten request-builder tests where CompositeCpu-specific shaping is still user-visible.
- [x] Add or tighten capability-summary tests wherever frontend text intentionally simplifies backend behavior.
- [x] Keep CPU fallback and GPU-aware prescan behavior both explicit in the contract.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `crates/app_core/src/composite_cpu.rs`
- run targeted tests in `crates/app_core/src/composite_profile_route.rs`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run targeted Tauri optimize-command tests if CompositeCpu request wiring changes
- manually inspect one optimization result with route diagnostics and one drilldown/local-stability flow
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve CompositeCpu as a strategy-local ownership surface
- prefer additive clarity and narrower docs/tests over large refactors
- if a family migrates away from CompositeCpu, update request shaping, route diagnostics, and docs together
- keep CPU fallback behavior visible and stable
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Kept this CompositeCpu tranche additive and contract-first. No strategy/runtime refactor landed here; the work instead tightened the user-facing contract and analysis-path verification around the existing strategy-local surface.
- Expanded the contributor-facing truth anchors in `REPO_MAP.md` so CompositeCpu is explicitly documented as the owner of its request/result family, route diagnostics, and drilldown/local-stability follow-up path rather than being conflated with routed Double-MA profiles.
- Tightened the user-visible capability surface in `web/src/utils/strategyOptimizationCapabilities.ts` so CompositeCpu now advertises its route-diagnostic-aware drilldown and local-stability follow-ups instead of looking like a generic fixed path.
- Added frontend analysis regressions in `web/src/services/analysisService.test.ts` that prove CompositeCpu top-row selection routes through `compute_composite_cpu_drilldown` and `compute_composite_cpu_local_stability`, while preserving HalfTrend, MACZ, OTT, ASR, signal-mode, and backend-hint/backend contract fields.
- Added validation/report parity coverage for the plain CompositeCpu surface: `validationRequestBuilder.test.ts`, `validationService.test.ts`, and `commands_reports_validation_common.rs` now pin the product-default `CpuOnly` preview copy plus the GPU-route note that warns about HalfTrend-uniformity limits across finalists.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts src/services/optimizeRequestBuilder.test.ts src/services/analysisService.test.ts src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`, `cargo test -p ta_app_core composite_cpu --quiet`, `cargo test -p ta_desktop_demo_app commands_reports_validation_common --lib`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_2026-04-08.md`
- `REPO_Improvement/03_opt/15_opt_composite_cpu_command_2026-04-08.md`
- `REPO_Improvement/07_strats/20_strat_surface_matrix_2026-04-09.md`
