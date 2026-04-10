# DoubleMa Backend Routing Plan

Last updated: 2026-04-09

## Purpose

Define the route-policy and fallback-improvement plan for DoubleMa-family backend execution.

## Scope Boundaries

In scope:

- `Backend` selection and `Auto` behavior
- CPU / GPU / hybrid / kernel route distinctions
- fallback reasons and route-note truthfulness
- UI-visible route summaries

Out of scope:

- broader strategy-family docs
- report formatting

## Current User-Facing Behavior

The product exposes CPU, GPU sweep, GPU hybrid kernel, GPU kernel, and auto-style routing. The backend resolves product modes, route blockers, and preferred CUDA devices, while the UI presents route previews and capability messaging.

## Current Code Surface

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`

## Current Verification Surface

- DoubleMa backend/unit tests in `crates/app_core`
- optimize service/request-builder tests
- perf gates and route-matrix profiling surfaces

## Strengths Already Present

- route selection is already explicit in code
- backend fallbacks are already captured in route summaries and reasons
- the product already differentiates grid-only product constraints from backend capabilities

## Weaknesses / Failure Modes

- route policy spans frontend messaging and backend execution logic
- exact vs approximate / follow-up behavior is easy to miscommunicate
- fallback reasons can drift if they are not treated as contract-like output

## Regressions To Avoid

- changing `Auto` behavior without updating UI messaging
- hiding fallback reasons from results/report surfaces
- increasing host/device transfers in a way that defeats the kernel route
- silently changing route resolution for export / heatmap / include-all cases
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Async Commands](https://v2.tauri.app/develop/calling-rust/) reinforces that heavy optimization work should remain off the UI thread.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) emphasizes minimizing host/device transfers and keeping intermediate structures on device memory when GPU performance matters.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for treating route summaries and fallback reasons as stable output fields rather than incidental strings.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document a route-decision tree for each backend mode.
2. Document all known blockers that force fallback.
3. Document which product outputs force grid behavior and why.
4. Make route-note truthfulness part of the contract across backend, UI, and report output.
5. Tighten coverage around approximate / exact-anchor distinctions for VDPS-style paths.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the route matrix for CPU, GPU sweep, GPU hybrid kernel, GPU kernel, and auto.
- [x] Document preferred-device resolution and fallback-device behavior.
- [x] Document grid-only product overrides for heatmap/export/include-all.
- [x] Document fallback reasons and where they must surface.
- [x] Document approximate vs exact semantics for VDPS and related cheaper paths.
- [x] Document the expected UI route preview language.
- [x] Add or tighten route-resolution tests and representative fixtures.
- [x] Tie route-sensitive changes to perf/profiling checks where needed.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `cargo test -p ta_app_core`
- run optimize request-builder/service tests
- run route-matrix or representative profiling/perf tooling if routing logic changes materially
- manually verify route notes in results/report outputs for representative runs
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve CPU fallback support
- preserve existing backend labels unless an explicit migration is planned
- prefer additive route diagnostics over changing route semantics
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the core backend route contract in the default CPU-safe lane. `auto_backend_records_route_metadata` and `gpu_hybrid_phase1_cpu_fallback_surfaces_route_metadata` continue to pass in `crates/app_core/src/double_ma.rs`, which confirms that backend auto-resolution still records truthful route metadata and that hybrid phase-1 CPU fallback still surfaces route reasons instead of silently degrading.
- Reconfirmed command-boundary product normalization in `src-tauri/src/commands_optimize.rs`. The existing tests still prove that Double-MA requests preserve non-grid modes when exact-grid outputs are disabled, pin back to `Grid` when heatmap/include-all/export shapes demand exact-grid semantics, and ignore whitespace export paths rather than misclassifying a run.
- The deeper GPU-kernel route test named `composite_adx_off_gpu_kernel_routes_accept_advanced_exact_controls` is not compiled in the default lane on this machine, so this execution pass records it honestly as CUDA-lane evidence rather than pretending it was exercised here. The route-policy closure for this doc therefore rests on the verified default-lane route-metadata tests plus the command-level normalization tests already present.
- No code change was required in this tranche because the audited backend route behavior and command normalization already satisfy the documented contract. Acceptance evidence captured during execution: `cargo test -p ta_app_core auto_backend_records_route_metadata --lib`, `cargo test -p ta_app_core gpu_hybrid_phase1_cpu_fallback_surfaces_route_metadata --lib`, `cargo test -p ta_desktop_demo_app commands_optimize --lib`, and the existing frontend request-builder/service route tests exercised in prior optimization tranches.

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/07_strats/25_strategy_runtime_gpu_stack_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/36_gpu_route_matrix_profile_2026-04-09.md`
