# Dynamic Indicator Catalog Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the dynamic-indicator catalog so registry export, builder eligibility, output typing, and parameter-schema truth remain explicit, stable, and safe for strategy-builder flows.

## Scope Boundaries

In scope:

- backend export of dynamic-strategy-eligible indicator descriptors
- frontend catalog normalization and caching
- builder-side eligibility filtering for dynamic Double-MA construction
- output descriptors, parameter descriptors, and GPU preference hints

Out of scope:

- moving-average catalog behavior
- dynamic-strategy runtime execution beyond contract implications
- options-related indicator workflows

## Current User-Facing Behavior

The repo already exposes a dynamic-indicator catalog that is broader than the builder’s final admissible set. The backend exports descriptors for indicators marked `dynamic_strategy_eligible`, including category, input kind, CPU/CUDA support, GPU-kernel hints, outputs, params, and notes.

The frontend then normalizes that payload and the dynamic builder applies a second eligibility filter. Builder eligibility currently requires that an indicator be dynamic-strategy-eligible, expose at least one floating-valued output, and avoid required params that cannot be safely defaulted.

That makes the dynamic-indicator catalog a two-stage contract: a raw exported descriptor set and a narrower builder-eligible subset. Confusing those two layers would create either missing options in the builder or illegal options that cannot be executed safely.

## Current Code Surface

- `crates/app_core/src/indicator_catalog.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dynamicIndicatorCatalogService.ts`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `web/src/services/optimizeRequestBuilder.ts`

## Current Verification Surface

- `crates/app_core/src/indicator_catalog.rs`
- `web/src/services/dynamicIndicatorCatalogService.test.ts`
- `web/src/strategies/dynamicDoubleMaBuilder.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`

## Strengths Already Present

- the backend already exports a rich indicator descriptor model instead of ad hoc UI rows
- the export already filters to `dynamic_strategy_eligible` indicators
- outputs and params already carry typed descriptor structure
- the builder already enforces an additional admissibility layer instead of blindly trusting the raw catalog
- the frontend normalization path already deduplicates ids, params, and outputs

## Weaknesses / Failure Modes

- contributors can assume “catalog entry exists” means “builder should expose it,” which is not currently true
- output-count and output-descriptor consistency can drift if normalization or registry export changes
- GPU support, VRAM support, and speedup preference can be read as stronger guarantees than they actually are
- enum/default/required-param behavior is subtle and can break builder eligibility if not documented
- frontend cache behavior can mask descriptor drift in tests unless cache reset is explicit

## Regressions To Avoid

- exposing non-builder-safe indicators in the dynamic builder
- breaking alignment between `output_count` and the actual `outputs` array
- losing required/default/enum param semantics during normalization
- using GPU preference hints as a substitute for backend legality
- dropping lower-level descriptor fields that future runtime or UI surfaces may already depend on
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the indicator descriptor payload already behaves like a structured schema contract.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the builder should consume a single normalized catalog source instead of fragmenting eligibility logic across components.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the builder and optimize flow need predictable typed shared state around normalized descriptor data.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the dynamic-indicator catalog is a backend-owned contract consumed across the Tauri boundary.
- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because exported GPU/VRAM hints should remain truthful capability signals rather than optimistic guesses.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Freeze the distinction between exported dynamic-indicator descriptors and builder-eligible dynamic indicators.
2. Document each descriptor family separately: identity/category, input-kind/capability, outputs, params, and notes.
3. Treat output typing and required-param defaultability as first-class eligibility rules rather than incidental builder heuristics.
4. Keep builder filtering explicit and local, while keeping raw descriptor normalization reusable.
5. Add or tighten tests whenever new indicator kinds, output kinds, or param kinds are introduced.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the raw exported descriptor contract and the narrower builder-eligible subset.
- [ ] Document output-count, output-descriptor, and floating-output eligibility rules.
- [ ] Document required/default/enum param behavior and defaultability rules.
- [ ] Document capability and preference flags without overstating legality.
- [ ] Add or tighten tests for catalog normalization and builder eligibility.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted catalog tests in `crates/app_core/src/indicator_catalog.rs`
- run `npm --prefix web run test -- src/services/dynamicIndicatorCatalogService.test.ts`
- run `npm --prefix web run test -- src/strategies/dynamicDoubleMaBuilder.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the raw exported descriptor shape until all builder and request consumers migrate together
- prefer adding new descriptor fields over changing existing semantics
- if builder eligibility expands later, document the new admissibility rule explicitly rather than silently widening the set
- keep unsupported-but-exported indicators out of the builder until runtime legality and defaults are verified together
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/04_opt_dynamic_builder_2026-04-09.md`
- `REPO_Improvement/03_opt/23_eng_dynamic_indicators_2026-04-09.md`
- `REPO_Improvement/03_opt/24_eng_strategy_ir_plan_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
