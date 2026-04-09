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

## External Research Summary

- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU speedup claims should remain measurement-backed and context-aware rather than treated as universal truths.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the numeric telemetry crosses from backend to frontend as a structured contract.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because UI consumers should share one normalized speedup table instead of re-parsing or reinterpreting raw values independently.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because speedup guidance is deterministic contract data that should be validated with focused normalization and consumer tests.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the exported speedup-table rows behave like a stable structured compatibility surface.

## Proposed Improvement Approach

1. Freeze the separation between hard capability, policy preference, and numeric telemetry.
2. Document the exact backend rule: table membership influences preference only; it does not manufacture capability.
3. Document the exact frontend rule: numeric speedup rows are explanatory telemetry and should not silently override backend truth.
4. Keep CSV parsing logic simple and deterministic, with clear skip behavior for malformed rows.
5. Add or tighten tests whenever the CSV schema or policy interpretation changes.

## Checklist

- [ ] Document hard capability versus speedup-derived preference versus numeric telemetry.
- [ ] Document the backend embedded-table rule and fallback behavior.
- [ ] Document the frontend normalization contract for numeric speedup rows.
- [ ] Document UI guidance limits so speedup hints are not overstated.
- [ ] Add or tighten tests when the CSV schema or interpretation changes.

## Verification Plan

- run targeted `ma_policy` tests in `crates/app_core/src/ma_policy.rs`
- run `npm --prefix web test -- src/services/gpuSpeedupsService.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/services/movingAverageCatalogService.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve the current CSV row shape until all telemetry consumers migrate together
- prefer additive CSV columns over changing the meaning of existing ones
- keep backend routing conservative if speedup data is unavailable or incomplete
- if new policy thresholds are introduced later, document them separately from the raw telemetry export

## Related Docs

- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/03_opt/29_ma_catalog_registry_2026-04-09.md`
- `REPO_Improvement/03_opt/30_dynamic_indicator_catalog_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
