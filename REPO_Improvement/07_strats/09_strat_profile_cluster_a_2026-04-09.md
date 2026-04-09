# Profile Cluster A Strategy Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the first standalone profile cluster inside the non-options strategy surface: `Asr`, `Vcc`, `Vemr`, `Rocb`, and `Vcet`.

## Scope Boundaries

In scope:

- `crates/app_core/src/asr.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/vcet.rs`
- the corresponding `*_gpu_kernel.rs` files
- shared orchestration in `crates/app_core/src/composite_cpu.rs` and `crates/app_core/src/composite_validate.rs`
- frontend/backend capability and request surfaces that expose these profile families

Out of scope:

- `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`
- `CompositeMo`
- options-related strategies

## Current User-Facing Behavior

This cluster already exists as real product-facing strategy kinds, not just internal experiments. `Asr`, `Vcc`, `Vemr`, `Rocb`, and `Vcet` can be selected from the optimize workflow, run through the shared composite CPU product route, emit optimization results through the standard result surfaces, and participate in validation.

Current behavior already includes:

- strategy-local parameter sets and validation rules
- route-aware CPU and GPU-kernel execution where supported
- route diagnostics and fallback notes on some GPU paths
- reuse of the standard top-k, shortlist, artifact, and validation workflow instead of a special side channel

## Current Code Surface

- `crates/app_core/src/asr.rs`
- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/vcet.rs`
- `crates/app_core/src/vcet_gpu_kernel.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

## Current Verification Surface

- tests embedded in `crates/app_core/src/composite_cpu.rs`
- tests embedded in `crates/app_core/src/composite_validate.rs`
- module-local tests inside the profile files listed above
- `crates/app_core/benches/asr_opt.rs`
- `crates/app_core/benches/rocb_opt.rs`
- `crates/app_core/benches/vcc_opt.rs`
- `crates/app_core/benches/vcet_opt.rs`
- `crates/app_core/benches/vemr_opt.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`

## Strengths Already Present

- each family already has its own strategy-local core file instead of hiding entirely inside one generic engine
- the cluster already has broader benchmark coverage than the less mature standalone-profile families
- GPU-kernel execution is already implemented for several members of the cluster
- route diagnostics already exist for fallback and mixed-route cases
- the cluster already reuses the standard optimization and validation workflow rather than a disconnected prototype path

## Weaknesses / Failure Modes

- the product route is still orchestrated through very large shared files in `composite_cpu.rs` and `composite_validate.rs`
- strategy-local logic, route notes, and validation behavior can drift because ownership is split between profile files and shared orchestration
- route diagnostics are largely string-based and can become stale if runtime decisions change
- profile-specific benchmark depth is uneven even within this stronger cluster
- capability summaries in the frontend can overstate or understate what the route layer actually does

## Regressions To Avoid

- changing GPU-kernel fallback behavior without updating route diagnostics and tests
- letting validation semantics drift from optimization semantics for the same profile
- breaking standard result, shortlist, or artifact expectations while changing one profile family
- flattening these strategy families back into opaque shared code where ownership becomes harder to trace
- weakening benchmark or parity coverage for mature profiles while extending new ones

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because these profile families already exercise mixed CPU/GPU execution and should keep device-path behavior explicit and measurable.
- [Criterion.rs Getting Started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because this cluster already has dedicated bench surfaces and should keep benchmark-backed regressions practical.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because these strategy families are exposed through typed command boundaries and frontend request builders.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because request/result/report payloads for this cluster behave like structured contracts across multiple layers.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because capability and request-shaping drift often appears first in store/service level tests rather than at the kernel layer.

## Proposed Improvement Approach

1. Treat Cluster A as a semi-productized standalone-profile tier with explicit ownership rules.
2. Document each family's route model, validation behavior, and benchmark coverage in one place.
3. Tighten the boundary between strategy-local files and shared orchestration in `composite_cpu.rs` / `composite_validate.rs`.
4. Keep route diagnostics truthful and test-backed.
5. Use the cluster as the reference standard for how standalone profile families should look before expanding the less mature cluster.

## Checklist

- [ ] Document the exact membership of Cluster A and why these families are grouped together.
- [ ] Document route, fallback, and validation expectations for each member of the cluster.
- [ ] Document which behaviors are strategy-local versus shared in `composite_cpu.rs` and `composite_validate.rs`.
- [ ] Add or tighten tests for route diagnostics and fallback notes.
- [ ] Add or tighten capability-summary and request-builder tests for these profile kinds.
- [ ] Review whether benchmark coverage should be normalized across all five cluster members.
- [ ] Keep artifact/result expectations aligned with the rest of the standard optimize workflow.

## Verification Plan

- run targeted tests in `crates/app_core/src/composite_cpu.rs`
- run targeted tests in `crates/app_core/src/composite_validate.rs`
- run the dedicated benches already present for `Asr`, `Rocb`, `Vcc`, `Vcet`, and `Vemr`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/utils/strategyOptimizationCapabilities.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current strategy ids and frontend surface names
- prefer additive ownership clarification and test hardening before deeper refactors
- if shared orchestration is reduced later, move behavior into strategy-local files without changing result or validation semantics
- use this cluster's stronger benchmark/test posture as the minimum bar for future standalone-profile changes

## Related Docs

- `REPO_Improvement/07_strats/06_strat_composite_cpu_2026-04-09.md`
- `REPO_Improvement/07_strats/12_eng_composite_validation_2026-04-08.md`
- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/13_strategy_caps_surface_2026-04-08.md`
