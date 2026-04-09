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

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because stage-0 prescan only justifies itself if it preserves clear compute/memory tradeoffs rather than opaque overhead.
- [rayon iterator docs](https://docs.rs/rayon/latest/rayon/iter/index.html) are relevant because prescan and subsequent search phases rely on safe data-parallel evaluation patterns.
- [Optuna `TPESampler` docs](https://optuna.readthedocs.io/en/v2.10.1/reference/generated/optuna.samplers.TPESampler.html) are relevant because prescan narrowing must not be allowed to obscure the semantics of the downstream sampler family.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because prescan configuration is part of the structured CompositeMo request contract.
- [serde_json `Value` docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because request/config payloads and diagnostics pass through structured JSON-like values.

## Proposed Improvement Approach

1. Freeze prescan as an explicit domain-narrowing phase, not an invisible optimization trick.
2. Document how objective-only narrowing, predictive rescue, and fixed seed pairs combine.
3. Keep backend inheritance and route-note behavior explicit.
4. Add tests that show domain narrowing effects and predictive rescue behavior directly.
5. Keep request-builder defaults and backend defaults aligned.

## Checklist

- [ ] Document the prescan request contract and default semantics.
- [ ] Document how predictive rescue interacts with objective-only prescan behavior.
- [ ] Document inherited backend and explicit backend behavior for prescan.
- [ ] Add or tighten tests for predictive seed-pair generation.
- [ ] Add or tighten tests for domain narrowing and top-k/window behavior.
- [ ] Add or tighten frontend tests for CompositeMo prescan request shaping.
- [ ] Add or tighten capability-summary tests for prescan-related product messaging.
- [ ] Keep route-note or diagnostics coverage aligned with real prescan behavior.

## Verification Plan

- run targeted tests in `crates/app_core/src/composite_mo.rs`
- run targeted tests in `crates/app_core/src/prescan_predictive.rs`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run targeted Tauri optimize-command tests for CompositeMo requests
- manually inspect one CompositeMo request with prescan enabled and one without prescan

## Rollout / Fallback / Compatibility Notes

- preserve prescan as optional and explicit
- avoid changing narrowing semantics without matching diagnostics and tests
- keep inherited backend behavior stable unless a product decision changes it
- if predictive rescue expands, land verification before changing defaults

## Related Docs

- `REPO_Improvement/07_strats/07_strat_composite_mo_2026-04-09.md`
- `REPO_Improvement/03_opt/16_opt_composite_mo_command_2026-04-08.md`
- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-08.md`
- `REPO_Improvement/07_strats/21_optimizer_grid_stream_2026-04-09.md`
