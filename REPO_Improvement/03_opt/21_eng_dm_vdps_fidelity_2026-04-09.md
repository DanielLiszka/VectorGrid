# Double MA VDPS Fidelity Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Double-MA VDPS path so its approximate nature, rung behavior, reproducibility, and quality-check telemetry remain explicit and non-misleading.

## Scope Boundaries

In scope:

- `GpuVdps` request behavior for the Double-MA family
- `VdpsConfig` and rung behavior
- quality-check telemetry, overlap, and regret reporting
- command-boundary mode normalization that constrains VDPS when grid outputs are requested

Out of scope:

- exact-grid optimization behavior
- fidelity audit harness interpretation
- options-related approximate search

## Current User-Facing Behavior

VDPS is already exposed as an alternate optimization mode for Double-MA-family requests, but it is an approximate staged search rather than an exact full-grid enumeration. The backend validates a `VdpsConfig`, evaluates rung fractions, promotes survivors, can inject random or elite mutation seeds, and can optionally refill from GPU-oriented candidate pools.

The backend also already emits telemetry about the approximate search. That telemetry can include overlap and regret quality checks, evaluation counts, survivor counts, runtime totals, and rung-level details. The frontend and artifacts can therefore distinguish VDPS from exact search when they choose to surface that information.

At the same time, the product already protects some exact-only outputs. Command-level normalization forces `Grid` when include-all, heatmap, or export-all outputs are requested, so VDPS cannot silently pretend to support exact-grid output shapes through those routes.

## Current Code Surface

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`

## Current Verification Surface

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`

## Strengths Already Present

- VDPS already has a dedicated backend configuration and helper module
- approximate search already exposes telemetry instead of hiding behind exact-looking results
- deterministic seeding and quality-check support already exist
- command-level normalization already blocks unsupported exact-grid output combinations
- rung behavior and candidate promotion are already explicit in code

## Weaknesses / Failure Modes

- users and contributors can still overread VDPS as an exact search substitute if the contract is not documented clearly
- rung, refill, and quality-check semantics are subtle and spread across multiple backend files
- reproducibility can drift if deterministic settings are treated casually
- product-mode normalization and builder legality need to stay aligned with backend behavior
- quality-check telemetry is only valuable if it remains stable and correctly interpreted

## Regressions To Avoid

- presenting VDPS as exact or equivalent to full-grid truth
- allowing exact-grid-only outputs under VDPS through mode drift
- losing reproducibility or deterministic-seed behavior
- dropping overlap or regret quality-check fields silently
- weakening rung or refill semantics without synchronized docs and tests
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because approximate GPU-oriented search should stay grounded in explicit quality and transfer considerations rather than vague speed claims.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because VDPS configuration and telemetry cross a command boundary into product behavior.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because frontend mode and telemetry state should stay derived from canonical request and result contracts.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because VDPS telemetry behaves like a structured result contract consumed by multiple layers.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Freeze VDPS as an approximate search family with explicit non-goals.
2. Document rung, survivor, refill, and deterministic-seed semantics clearly.
3. Document the quality-check telemetry fields and how they should be interpreted.
4. Keep exact-only output protections explicit at the command and builder layers.
5. Add or tighten tests around determinism, quality-check emission, and mode normalization if needed.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document VDPS as approximate, not exact.
- [ ] Document `VdpsConfig` fields and rung behavior.
- [ ] Document overlap and regret quality-check telemetry.
- [ ] Document exact-output restrictions that force `Grid` instead of VDPS.
- [ ] Add or tighten tests for determinism and telemetry stability.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted VDPS tests in `crates/app_core/src/double_ma.rs`
- run targeted helper tests in `crates/app_core/src/double_ma_vdps.rs`
- run `cargo test -p ta_desktop_demo_app commands_optimize`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current exact-only protections for include-all, heatmap, and export-all output shapes
- prefer clearer wording before broadening or narrowing VDPS eligibility
- keep telemetry fields stable until all downstream consumers migrate together
- if quality-check behavior changes, update fidelity-oriented docs and artifacts in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/14_opt_double_ma_command_2026-04-09.md`
- `REPO_Improvement/03_opt/20_eng_dm_gpu_kernel_vram_2026-04-09.md`
- `REPO_Improvement/03_opt/22_eng_dm_output_contract_2026-04-09.md`
- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
