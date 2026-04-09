# Profile Cluster B Strategy Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the second standalone profile cluster inside the non-options strategy surface: `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, and `Cdmc`.

## Scope Boundaries

In scope:

- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/cdmc.rs`
- the corresponding `*_gpu_kernel.rs` files
- shared orchestration in `crates/app_core/src/composite_cpu.rs` and `crates/app_core/src/composite_validate.rs`
- frontend/backend capability and request surfaces that expose these profile families

Out of scope:

- Cluster A standalone profiles (`Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet`)
- `CompositeMo`
- options-related strategies

## Current User-Facing Behavior

This cluster is also already user-visible through the standard optimize workflow. `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, and `Cdmc` can be requested through the normal product path, produce standard optimization outputs, and participate in validation.

Current behavior already includes:

- strategy-local parameter validation and signal-generation logic
- mixed CPU/GPU-kernel execution in the families that already support it
- route diagnostics for fallback-sensitive families such as `Kqsr` and `Kvdf`
- reuse of the existing result and validation surfaces rather than a sidecar research-only lane

## Current Code Surface

- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/cdmc.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

## Current Verification Surface

- tests embedded in `crates/app_core/src/composite_cpu.rs`
- tests embedded in `crates/app_core/src/composite_validate.rs`
- module-local tests inside the profile files listed above
- `crates/app_core/benches/mkcs_opt.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`

## Strengths Already Present

- each family already owns real strategy-local computation code instead of being implied by config flags alone
- the cluster already participates in the same optimize/validate/artifact workflow as the rest of the product
- route diagnostics exist for some members and are already asserted in integration tests
- GPU-kernel execution is already implemented for multiple families in the cluster
- the cluster is large enough to justify a shared improvement doc instead of scattering one-off notes

## Weaknesses / Failure Modes

- benchmark coverage is visibly thinner and less even than Cluster A
- some of the cluster's quality signal comes from diagnostic-heavy integration tests rather than cleaner dedicated bench surfaces
- orchestration still runs through the same very large shared files, so ownership can blur quickly
- route diagnostic behavior is partially text-driven and can drift from the runtime path
- capability-summary copy can lag actual route behavior for the less frequently exercised profile kinds

## Regressions To Avoid

- changing fallback or route-diagnostic behavior without synchronized tests
- weakening validation parity while tuning optimization paths
- breaking less frequently exercised strategy kinds because shared orchestration changes were only tested against the more popular profiles
- silently changing benchmark posture or removing useful diagnostic coverage without replacement
- merging cluster-specific logic into generic helpers that obscure family ownership

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because this cluster depends on mixed CPU/GPU execution and route fallback clarity.
- [Criterion.rs Getting Started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because this cluster needs more deliberate benchmark coverage and clearer regression baselines.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because these strategies are still part of the standard command-driven product surface, not isolated binaries.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because request/result/report payloads for these families are shared structured contracts.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because frontend capability and visibility regressions around lower-traffic strategy kinds benefit from focused, user-visible checks.

## Proposed Improvement Approach

1. Treat Cluster B as the second standalone-profile tier with explicit maturity gaps relative to Cluster A.
2. Document which members already have strong route and validation coverage and which still lean on shared integration tests.
3. Keep strategy-local ownership explicit while narrowing the amount of hidden policy inside the giant composite orchestrators.
4. Raise coverage and diagnostics quality before expanding the cluster's public surface further.
5. Keep the cluster fully compatible with the shared optimize/validate workflow while improving the weakest verification gaps first.

## Checklist

- [ ] Document the exact membership of Cluster B and why these families are grouped together.
- [ ] Document route, fallback, and validation expectations for each member of the cluster.
- [ ] Document current benchmark gaps and diagnostic-only coverage gaps.
- [ ] Add or tighten route-diagnostic tests where runtime fallback is important.
- [ ] Add or tighten capability-summary and request-builder tests for these profile kinds.
- [ ] Review whether additional dedicated benches are needed beyond `mkcs_opt.rs`.
- [ ] Keep result, validation, and artifact behavior aligned with the standard product workflow.

## Verification Plan

- run targeted tests in `crates/app_core/src/composite_cpu.rs`
- run targeted tests in `crates/app_core/src/composite_validate.rs`
- run the existing `mkcs_opt.rs` bench and any relevant strategy-local benches added later
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/utils/strategyOptimizationCapabilities.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current strategy ids and frontend labels
- prefer additive hardening and ownership cleanup before broadening user-facing promises for the cluster
- if benchmark coverage is expanded, do it without altering current route semantics
- keep the shared product workflow intact even if more strategy-local code is extracted over time

## Related Docs

- `REPO_Improvement/07_strats/06_strat_composite_cpu_2026-04-09.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_2026-04-08.md`
- `REPO_Improvement/07_strats/09_strat_profile_cluster_a_2026-04-09.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
