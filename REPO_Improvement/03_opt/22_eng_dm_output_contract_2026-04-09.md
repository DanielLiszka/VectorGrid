# Double MA Output Contract Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Double-MA backend output contract so core fields, optional route metadata, metric-surface information, and heatmap auxiliaries remain stable and truthful across frontend and artifact consumers.

## Scope Boundaries

In scope:

- `BackendOptimizationResult` and related Double-MA result structures
- route, fallback, timing, residency, and metric-surface metadata
- heatmap auxiliary fields such as mean values and sample counts
- frontend and artifact consumers of the Double-MA result shape

Out of scope:

- result-table rendering details
- validation output contracts
- options-related result surfaces

## Current User-Facing Behavior

Double-MA-family optimization results already carry more than the basic optimizer output. The backend result includes best params and metrics, top results, optional all-rows output, heatmap data, and also route-oriented metadata such as backend route, fallback reason, indicator-route plan, residency, timings, follow-up cache key, shortlist diagnostics, score-observation context, and `metric_surface_kind`.

This metadata is not decorative. Frontend result handling, run-artifact persistence, validation/report generation, and metric-surface interpretation already depend on these fields. Heatmaps can also include auxiliary arrays such as `mean_values` and `sample_counts`, so the output contract is broader than a simple two-dimensional score grid.

Because the output shape is already used across layers, it needs a dedicated plan that distinguishes core fields from optional route- and contract-specific fields.

## Current Code Surface

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/services/optimizeService.ts`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/metric_surface.rs`

## Current Verification Surface

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_result.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `web/src/services/optimizeService.test.ts`

## Strengths Already Present

- the backend output contract is already rich and route-aware
- optional route and residency metadata already exist instead of being reconstructed heuristically
- metric-surface information is already carried explicitly
- artifacts and frontend services already consume a shared result shape
- heatmap auxiliary arrays already have real backend support and tests

## Weaknesses / Failure Modes

- the result shape is large enough that accidental field drift is easy
- there is no dedicated implementation doc distinguishing stable core fields from optional extensions
- frontend and artifact consumers can silently depend on optional fields without a clear compatibility plan
- `metric_surface_kind` and route metadata are critical but easy to treat as incidental
- heatmap auxiliary arrays can be broken by backend refactors that seem unrelated to UI consumers

## Regressions To Avoid

- renaming or dropping optional fields that downstream consumers already use
- misreporting `metric_surface_kind`
- weakening route or fallback truthfulness in result metadata
- breaking heatmap `mean_values` or `sample_counts`
- changing output-shape semantics without synchronized artifact and frontend updates

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because this result shape already behaves like a versioned structured contract.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because this contract crosses the backend boundary into frontend and artifact code.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because frontend consumers should stay driven by canonical result payloads rather than inferred field semantics.
- [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) is relevant indirectly because result and route metadata feed user-visible runtime behavior and follow-up flows.

## Proposed Improvement Approach

1. Split the output contract into stable core fields and optional route- or contract-specific extensions in the implementation plan.
2. Document all currently consumed optional fields explicitly.
3. Freeze `metric_surface_kind`, route, fallback, residency, and heatmap auxiliary semantics.
4. Add parity checks between backend result output and artifact/frontend consumers.
5. Treat contract-shape changes as compatibility work, not incidental backend cleanup.

## Checklist

- [ ] Document stable core result fields.
- [ ] Document optional route, fallback, residency, and timing fields.
- [ ] Document `metric_surface_kind` and score-observation context semantics.
- [ ] Document heatmap auxiliary arrays and their consumers.
- [ ] Add or tighten consumer-parity tests for result-shape compatibility.

## Verification Plan

- run targeted Double-MA result tests in `crates/app_core/src/double_ma.rs`
- run targeted result-construction tests in `crates/app_core/src/double_ma_result.rs`
- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`

## Rollout / Fallback / Compatibility Notes

- keep optional fields present and truthful until all known consumers migrate together
- prefer additive metadata over silent field repurposing
- if a field becomes deprecated, document the deprecation before removing it
- update artifact and metric-surface consumers in the same change as any output-contract shift

## Related Docs

- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/03_opt/21_eng_dm_vdps_fidelity_2026-04-09.md`
- `REPO_Improvement/04_results/09_results_analysis_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
