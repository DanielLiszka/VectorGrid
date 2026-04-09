# Double MA GPU Kernel VRAM Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Double-MA GPU kernel VRAM path so staged and resident cache behavior, prefetch budgeting, and device-buffer ownership stay explicit and regression-resistant.

## Scope Boundaries

In scope:

- resident and staged market-series caches
- campaign VRAM prefetch and promotion behavior
- VRAM MA inputs and kernel-execution buffer ownership
- route notes and diagnostics related to GPU-resident execution

Out of scope:

- frontend GPU capability prose outside VRAM-specific truth
- non-Double-MA GPU kernels
- options-related GPU work

## Current User-Facing Behavior

The Double-MA GPU route already has more than one memory posture. Some runs are fully GPU-resident, while others stage or skip residency depending on dataset, device, and budget conditions. That residency posture later surfaces through route diagnostics and result metadata.

Backend code already maintains staged and resident market-series caches keyed by candle-dataset signatures. Campaign-level prefetch can stage data in advance, and staged entries can be promoted into the active resident cache when the subsequent run uses the same dataset and device context.

Kernel MA execution then consumes VRAM-oriented inputs through `VramMaInputs` and the `compute_period_ma*_into` path. This means the VRAM layer is already an important systems contract, not just a performance detail.

## Current Code Surface

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/vram_ma.rs`
- `src-tauri/src/commands_campaign.rs`
- `crates/app_core/src/double_ma_result.rs`

## Current Verification Surface

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`

## Strengths Already Present

- the repo already has explicit staged and resident VRAM cache concepts
- staged-to-resident promotion is already tested
- kernel execution already uses VRAM-specific input structs rather than accidental reuse of CPU paths
- residency summaries already flow into backend result metadata
- campaign prefetch already exists as a first-class path instead of ad hoc warmup

## Weaknesses / Failure Modes

- VRAM residency behavior spans several backend layers and is easy to misunderstand without a dedicated plan
- device gating, dataset signature matching, and budget checks are subtle enough to drift
- extra host-device copies can quietly degrade the value of the resident path without changing visible route labels
- required candle-field assumptions for resident caches are easy to miss
- ownership of resident device buffers is implicit in code but not yet explicit in the improvement program

## Regressions To Avoid

- adding unnecessary H2D or D2H transfers on the resident path
- breaking staged-to-resident promotion
- weakening budget, device, or source gating for prefetched data
- misreporting residency posture in backend result metadata
- allowing required-candle-field mismatches into VRAM cache reuse

## External Research Summary

- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because resident device execution only matters if transfers, staging, and buffer reuse remain explicit and measured.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because prefetch and residency summaries ultimately surface through long-lived backend state and result contracts.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because residency and route diagnostics are serialized result fields consumed outside the kernel implementation.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because campaign prefetch and optimization runs cross separate command boundaries that must preserve truthful residency semantics.

## Proposed Improvement Approach

1. Document resident versus staged VRAM cache ownership and lifecycle explicitly.
2. Freeze the staged-to-resident promotion contract, including dataset-signature and device matching.
3. Document required candle fields and skip reasons for resident-path eligibility.
4. Make transfer minimization a first-class regression concern in this implementation plan.
5. Add or tighten tests around budget gating, promotion, and result-surface residency reporting where needed.

## Checklist

- [ ] Document staged and resident cache lifecycles.
- [ ] Document campaign prefetch promotion conditions and skip reasons.
- [ ] Document VRAM MA input ownership and kernel-execution buffer expectations.
- [ ] Document required candle fields and dataset-signature matching.
- [ ] Add or tighten tests for promotion, budgeting, and residency summaries.

## Verification Plan

- run targeted GPU cache and promotion tests in `crates/app_core/src/gpu_backtest_kernel.rs`
- run targeted Double-MA GPU-kernel parity tests in `crates/app_core/src/double_ma.rs`
- run targeted report or artifact tests if residency fields change
- run the GPU performance gate when VRAM-path behavior changes on a machine with GPU access

## Rollout / Fallback / Compatibility Notes

- preserve current skip-safe fallbacks when resident execution is unavailable
- prefer additive diagnostics before changing residency policy
- keep backend result residency summaries stable until all consumers migrate together
- if budgeting rules change, update campaign prefetch and runtime-path docs together

## Related Docs

- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/03_opt/21_eng_dm_vdps_fidelity_2026-04-09.md`
- `REPO_Improvement/03_opt/22_eng_dm_output_contract_2026-04-09.md`
- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
