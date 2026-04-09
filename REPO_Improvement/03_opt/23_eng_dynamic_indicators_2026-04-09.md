# Dynamic Indicators Engine Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the dynamic-indicator engine so descriptor validation, runtime compute adapters, and strategy-plan consumers remain clearly separated and do not drift.

## Scope Boundaries

In scope:

- dynamic-indicator descriptor cataloging
- CPU and CUDA compute adapters
- plan-time consumption of dynamic indicators
- runtime restrictions that matter to the builder and GPU plans

Out of scope:

- the frontend builder subset policy by itself
- non-dynamic indicator registries
- options-related indicator work

## Current User-Facing Behavior

Dynamic indicators already participate in the Dynamic Double MA path through a layered system. Only indicators marked `dynamic_strategy_eligible` are exported through the dynamic catalog path. Those descriptors include input kind, output descriptors, parameter schemas, CPU and CUDA support flags, VRAM support flags, and speedup-derived GPU preference.

At runtime, the repo already separates descriptor validation from execution. Descriptor normalization and output or parameter validation live in the indicator catalog layer. Actual compute flows through the dynamic-indicator adapter, which supports CPU computation, CUDA computation with prepared per-device uploads, and chained slice-input behavior.

This layered design already exists, but it needs a dedicated implementation plan because contributors can easily confuse catalog truth, builder-eligible subsets, and runtime compute truth.

## Current Code Surface

- `crates/app_core/src/indicator_catalog.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `web/src/services/dynamicIndicatorCatalogService.ts`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`

## Current Verification Surface

- `crates/app_core/src/indicator_catalog.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `web/src/services/dynamicIndicatorCatalogService.test.ts`
- `web/src/strategies/dynamicDoubleMaBuilder.test.ts`

## Strengths Already Present

- descriptor export is already restricted to dynamic-strategy-eligible indicators
- validation of params and output refs is already centralized in the catalog layer
- CPU and CUDA compute paths already have a dedicated adapter boundary
- device-slice chaining for slice-input indicators is already explicit
- builder restrictions are already narrower than raw runtime eligibility, which is the right direction

## Weaknesses / Failure Modes

- catalog truth, builder truth, and runtime compute truth can be conflated easily
- required-param and multi-output restrictions are subtle and easy to weaken accidentally
- CUDA support flags can be mistaken for kernel-resident or builder-safe support
- there is no dedicated plan freezing the adapter-versus-catalog boundary
- descriptor drift can break builder and runtime paths differently

## Regressions To Avoid

- relaxing multi-output `output_id` requirements
- exposing non-eligible indicators to the builder path
- assuming the adapter borrows kernel-resident VRAM buffers when it does not
- weakening param normalization or case-insensitive output resolution
- letting builder-eligible filtering diverge from documented runtime constraints

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because dynamic indicator descriptors act like structured interface contracts.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because descriptor catalogs and preview outputs cross the desktop command boundary.
- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because CUDA support claims should remain grounded in explicit data-movement and adapter constraints.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because frontend consumers should remain derived from catalog truth rather than component-local assumptions.

## Proposed Improvement Approach

1. Document the catalog, adapter, and plan-consumer layers separately.
2. Freeze builder-eligible restrictions as a narrower subset than raw runtime eligibility.
3. Document parameter normalization, output resolution, and multi-output rules explicitly.
4. Keep CUDA support flags and kernel-resident assumptions clearly separated.
5. Add or tighten parity tests between catalog export, builder filtering, and runtime plan compilation.

## Checklist

- [ ] Document descriptor-catalog ownership and exported fields.
- [ ] Document adapter ownership for CPU and CUDA compute paths.
- [ ] Document builder-eligible subset rules and why they are stricter than raw runtime support.
- [ ] Document multi-output and required-param restrictions.
- [ ] Add or tighten parity tests across catalog, builder, and runtime paths.

## Verification Plan

- run targeted descriptor tests in `crates/app_core/src/indicator_catalog.rs`
- run targeted adapter tests in `crates/app_core/src/dynamic_indicator_adapter.rs`
- run targeted dynamic-strategy tests in `crates/app_core/src/strategy_dynamic.rs`
- run `npm --prefix web test -- src/services/dynamicIndicatorCatalogService.test.ts`
- run `npm --prefix web test -- src/strategies/dynamicDoubleMaBuilder.test.ts`

## Rollout / Fallback / Compatibility Notes

- keep descriptor export conservative rather than silently broadening builder eligibility
- prefer additive fields over repurposing existing capability flags
- update frontend builder filtering and backend descriptor contracts together
- if CUDA behavior changes, revise adapter and catalog docs in the same change

## Related Docs

- `REPO_Improvement/03_opt/24_eng_strategy_ir_plan_2026-04-09.md`
- `REPO_Improvement/03_opt/30_dynamic_indicator_catalog_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
- `REPO_Improvement/03_opt/04_opt_dynamic_builder_2026-04-09.md`
