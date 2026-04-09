# CompositeMo Prescan Search Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the CompositeMo stage-0 prescan path so domain narrowing, predictive rescue, and downstream multi-objective search remain explicit and testable.

## Scope Boundaries

In scope:

- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `crates/app_core/src/composite_cpu.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- CompositeMo prescan configuration, predictive seed-pair generation, and search narrowing

Out of scope:

- overall CompositeMo sampler semantics not specific to prescan
- validation/report exports
- options-related search

## Current User-Facing Behavior

CompositeMo already supports an optional stage-0 prescan intended to narrow MA domains before multi-objective search. The current request surface can express:

- top-k prescan results
- prescan window size
- predictive rescue parameters
- explicit or inherited backend choice
- objective modes for the prescan
- optional fixed MA-pair seeds

This means prescan is already a real product-facing search behavior, not merely an internal heuristic.

## Current Code Surface

- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `crates/app_core/src/composite_cpu.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `src-tauri/src/commands_optimize.rs`

## Current Verification Surface

- tests in `crates/app_core/src/composite_mo.rs`
- tests in `crates/app_core/src/prescan_predictive.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- Tauri optimize-command tests covering CompositeMo request normalization
- `crates/app_core/benches/dynamic_strategy_cpu.rs`

## Strengths Already Present

- prescan is already explicitly modeled in the request contract
- predictive rescue is already separated into its own helper logic
- prescan can inherit backend preferences instead of inventing a totally separate route policy
- top-k/window/stride controls are already explicit
- CompositeMo and CompositeCpu already share some prescan-related concepts rather than duplicating everything ad hoc
- the backend already records meaningful prescan telemetry instead of hiding narrowing behavior

## Weaknesses / Failure Modes

- prescan narrowing can change downstream search behavior in ways users do not easily see
- predictive rescue and objective-only narrowing can interact subtly
- inherited backend selection can be confusing if route notes are weak
- request-builder and backend defaults can drift
- prescan is easy to overfit as a performance shortcut if the narrowed domain contract is not documented
- predictive rescue currently has less direct dedicated test surface than the main CompositeMo path

## Regressions To Avoid

- silently changing how prescan narrows domains
- letting predictive rescue widen or bias the search without explicit documentation and tests
- breaking inherited backend semantics
- producing CompositeMo payloads where prescan options are accepted but ignored
- conflating prescan seeds with final search results
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because stage-0 prescan only justifies itself if it preserves clear compute/memory tradeoffs rather than opaque overhead.
- [rayon iterator docs](https://docs.rs/rayon/latest/rayon/iter/index.html) are relevant because prescan and subsequent search phases rely on safe data-parallel evaluation patterns.
- [Optuna `TPESampler` docs](https://optuna.readthedocs.io/en/v2.10.1/reference/generated/optuna.samplers.TPESampler.html) are relevant because prescan narrowing must not be allowed to obscure the semantics of the downstream sampler family.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because prescan configuration is part of the structured CompositeMo request contract.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because request/config payloads and diagnostics pass through structured JSON-like values.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Freeze prescan as an explicit domain-narrowing phase, not an invisible optimization trick.
2. Document how objective-only narrowing, predictive rescue, and fixed seed pairs combine.
3. Keep backend inheritance and route-note behavior explicit.
4. Add tests that show domain narrowing effects and predictive rescue behavior directly.
5. Keep request-builder defaults and backend defaults aligned.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the prescan request contract and default semantics.
- [ ] Document how predictive rescue interacts with objective-only prescan behavior.
- [ ] Document inherited backend and explicit backend behavior for prescan.
- [ ] Add or tighten tests for predictive seed-pair generation.
- [ ] Add or tighten tests for domain narrowing and top-k/window behavior.
- [ ] Add or tighten frontend tests for CompositeMo prescan request shaping.
- [ ] Add or tighten capability-summary tests for prescan-related product messaging.
- [ ] Keep route-note or diagnostics coverage aligned with real prescan behavior.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `crates/app_core/src/composite_mo.rs`
- run targeted tests in `crates/app_core/src/prescan_predictive.rs`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run targeted Tauri optimize-command tests for CompositeMo requests
- manually inspect one CompositeMo request with prescan enabled and one without prescan
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve prescan as optional and explicit
- avoid changing narrowing semantics without matching diagnostics and tests
- keep inherited backend behavior stable unless a product decision changes it
- if predictive rescue expands, land verification before changing defaults
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/07_strats/07_strat_composite_mo_2026-04-09.md`
- `REPO_Improvement/03_opt/16_opt_composite_mo_command_2026-04-08.md`
- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-08.md`
- `REPO_Improvement/07_strats/21_optimizer_grid_stream_2026-04-09.md`
