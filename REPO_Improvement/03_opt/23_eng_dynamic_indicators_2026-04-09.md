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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because dynamic indicator descriptors act like structured interface contracts.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because descriptor catalogs and preview outputs cross the desktop command boundary.
- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because CUDA support claims should remain grounded in explicit data-movement and adapter constraints.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because frontend consumers should remain derived from catalog truth rather than component-local assumptions.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document the catalog, adapter, and plan-consumer layers separately.
2. Freeze builder-eligible restrictions as a narrower subset than raw runtime eligibility.
3. Document parameter normalization, output resolution, and multi-output rules explicitly.
4. Keep CUDA support flags and kernel-resident assumptions clearly separated.
5. Add or tighten parity tests between catalog export, builder filtering, and runtime plan compilation.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document descriptor-catalog ownership and exported fields.
- [x] Document adapter ownership for CPU and CUDA compute paths.
- [x] Document builder-eligible subset rules and why they are stricter than raw runtime support.
- [x] Document multi-output and required-param restrictions.
- [x] Add or tighten parity tests across catalog, builder, and runtime paths.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted descriptor tests in `crates/app_core/src/indicator_catalog.rs`
- run targeted adapter tests in `crates/app_core/src/dynamic_indicator_adapter.rs`
- run targeted dynamic-strategy tests in `crates/app_core/src/strategy_dynamic.rs`
- run `npm --prefix web run test -- src/services/dynamicIndicatorCatalogService.test.ts`
- run `npm --prefix web run test -- src/strategies/dynamicDoubleMaBuilder.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep descriptor export conservative rather than silently broadening builder eligibility
- prefer additive fields over repurposing existing capability flags
- update frontend builder filtering and backend descriptor contracts together
- if CUDA behavior changes, revise adapter and catalog docs in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the layered dynamic-indicator contract instead of widening it. The indicator catalog still owns descriptor truth and output-id validation, the adapter still owns CPU/CUDA compute behavior, and the builder continues to consume a narrower eligible subset than the runtime catalog exposes.
- Frontend and command-boundary evidence remains green: `dynamicIndicatorCatalogService.test.ts`, `dynamicDoubleMaBuilder.test.ts`, and `cargo test -p ta_desktop_demo_app commands_dynamic --lib` all passed in this tranche. That keeps catalog export, builder shaping, and preview-command behavior aligned for the current supported subset.
- No code change was required in this tranche because the audit did not reveal a real drift in descriptor validation, adapter behavior, or builder filtering. The remaining gap is cross-layer parity coverage using a real backend descriptor payload rather than a handcrafted frontend sample catalog, which should be handled as a future tightening step rather than a silent contract change.

## Related Docs

- `REPO_Improvement/03_opt/24_eng_strategy_ir_plan_2026-04-09.md`
- `REPO_Improvement/03_opt/30_dynamic_indicator_catalog_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
- `REPO_Improvement/03_opt/04_opt_dynamic_builder_2026-04-09.md`
