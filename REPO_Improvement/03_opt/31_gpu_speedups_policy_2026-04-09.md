# GPU Speedups Policy Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the GPU speedups policy so the repo keeps a clear separation between backend routing preference, exported numeric telemetry, and UI explanation of GPU-versus-CPU guidance.

## Scope Boundaries

In scope:

- embedded speedup-policy parsing in `ma_policy`
- Tauri export of `gpu_speedups.csv` rows
- frontend normalization and cache behavior for GPU speedup telemetry
- optimize and catalog consumers that use speedup-derived preference hints

Out of scope:

- raw benchmark generation methodology
- runtime route legality outside preference implications
- options-related GPU policy surfaces

## Current User-Facing Behavior

The repo already uses one CSV source in two different ways. In app-core, membership in the embedded `gpu_speedups.csv` table acts as a boolean preference signal for kernel-preferred routing. In Tauri and the frontend, the same CSV is exposed as a numeric table containing `speedup`, `cpu_ms`, and `cuda_ms` for display and explanation.

This means the speedup table is both policy input and user-facing telemetry, but not in the same form. Backend route policy consumes the table as a boolean preference overlay, while the frontend may show the numeric values directly. That distinction is important because table membership is not the same thing as hard capability and numeric speedup values are not the same thing as route legality.

Without a dedicated plan, it is easy for future work to overstate what the CSV proves or to let backend and frontend interpretations drift apart.

## Current Code Surface

- `crates/app_core/src/ma_policy.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/gpuSpeedupsService.ts`
- `web/src/services/optimizeService.ts`
- `crates/app_core/src/strategy_plan.rs`

## Current Verification Surface

- `crates/app_core/src/ma_policy.rs`
- `web/src/services/gpuSpeedupsService.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/movingAverageCatalogService.test.ts`

## Strengths Already Present

- backend policy already uses an embedded table rather than relying on an external file at runtime
- Tauri already exports numeric rows so the UI can explain the preference source instead of hiding it
- frontend normalization already preserves id, name, speedup, CPU ms, and CUDA ms fields explicitly
- app-core already separates hard kernel support from speedup-based preference
- tests already cover key routing and normalization behavior

## Weaknesses / Failure Modes

- contributors can mistake CSV membership for a hard capability flag
- the same data source supports both policy and display, which invites semantic drift
- missing or malformed CSV rows could distort UI explanation even if backend policy still falls back safely
- speedup values can be treated as universally valid across datasets, devices, or strategy contexts when they are really heuristic guidance
- there is no dedicated implementation doc freezing the “preference, not legality” rule

## Regressions To Avoid

- treating the speedup table as a substitute for `supports_gpu_kernel_ma`
- changing CSV interpretation in backend or frontend without synchronized updates
- dropping numeric telemetry fields that the UI uses to explain preference hints
- letting missing CSV content silently invert or flatten preference behavior
- presenting speedup guidance as a guarantee rather than a measured heuristic
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU speedup claims should remain measurement-backed and context-aware rather than treated as universal truths.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the numeric telemetry crosses from backend to frontend as a structured contract.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because UI consumers should share one normalized speedup table instead of re-parsing or reinterpreting raw values independently.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because speedup guidance is deterministic contract data that should be validated with focused normalization and consumer tests.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the exported speedup-table rows behave like a stable structured compatibility surface.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Freeze the separation between hard capability, policy preference, and numeric telemetry.
2. Document the exact backend rule: table membership influences preference only; it does not manufacture capability.
3. Document the exact frontend rule: numeric speedup rows are explanatory telemetry and should not silently override backend truth.
4. Keep CSV parsing logic simple and deterministic, with clear skip behavior for malformed rows.
5. Add or tighten tests whenever the CSV schema or policy interpretation changes.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document hard capability versus speedup-derived preference versus numeric telemetry.
- [x] Document the backend embedded-table rule and fallback behavior.
- [x] Document the frontend normalization contract for numeric speedup rows.
- [x] Document UI guidance limits so speedup hints are not overstated.
- [x] Add or tighten tests when the CSV schema or interpretation changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `ma_policy` tests in `crates/app_core/src/ma_policy.rs`
- run `npm --prefix web run test -- src/services/gpuSpeedupsService.test.ts`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/services/movingAverageCatalogService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current CSV row shape until all telemetry consumers migrate together
- prefer additive CSV columns over changing the meaning of existing ones
- keep backend routing conservative if speedup data is unavailable or incomplete
- if new policy thresholds are introduced later, document them separately from the raw telemetry export
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the separation between hard kernel capability, speedup-derived preference, and numeric telemetry. `gpuSpeedupsService.test.ts`, `movingAverageCatalogService.test.ts`, and `optimizeService.test.ts` remain green, which keeps the frontend explanatory telemetry aligned with the current backend preference model.
- No code change was required in this tranche because the current policy/telemetry separation is already implemented. The important outcome here is verified preservation of the rule that speedup-table membership influences preference hints but does not manufacture runtime legality.

## Related Docs

- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/03_opt/29_ma_catalog_registry_2026-04-09.md`
- `REPO_Improvement/03_opt/30_dynamic_indicator_catalog_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
