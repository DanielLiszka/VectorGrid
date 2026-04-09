# Composite Validation Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for non-options composite-strategy validation so route notes, fallback semantics, candidate handling, and report expectations remain truthful across the composite-profile and composite-CPU families.

## Scope Boundaries

In scope:

- composite strategy validation route behavior
- validation request and execution summaries
- route-sensitive fallback messaging
- report/export implications of composite validation routes

Out of scope:

- options validation
- standalone workbook-layout quality
- broad optimization route policy outside validation concerns

## Current User-Facing Behavior

Composite strategy validation is not one single route. `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` are treated as Double-MA-family fixed-candidate validation routes. `CompositeMo` currently uses a CPU validation route, and plain `CompositeCpu` GPU fixed-candidate validation has narrower constraints. The frontend already surfaces route notes and warnings, but the rules are distributed across request builders, services, backend handlers, and report builders.

## Current Code Surface

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_cpu.rs`
- `src-tauri/src/commands_reports_validation_common.rs`

## Current Verification Surface

- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`
- validation workspace component tests
- Rust validation command tests
- validation report tests in `src-tauri/tests/validation_xlsx_report_tests.rs`

## Strengths Already Present

- composite validation already has meaningful route notes instead of silent fallback
- frontend preview and post-run execution summaries already exist
- backend strategy routing and validation helpers already enforce important distinctions
- report builders already preserve some route/fallback explanation in exported workbooks

## Weaknesses / Failure Modes

- the validation matrix across composite families is complex and easy to simplify incorrectly
- route notes live in several places and can drift
- GPU-capable composite validation is narrower than optimize-route messaging might suggest
- report/export behavior depends on route and fallback semantics that are not owned by one document today

## Regressions To Avoid

- over-promising GPU validation support for composite families
- hiding CPU or mixed-route fallbacks that users need to understand
- changing validation route semantics without updating report wording
- weakening current strategy-specific distinctions in the name of generic cleanup
- breaking current fixed-candidate validation flows for composite profiles

## External Research Summary

- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because validation is a time-ordered contract, not just a post-processing step.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because preview and actual validation execution are separated by a command boundary.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU-validation route claims need evidence-backed host/device boundary discipline.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting route- and family-sensitive validation payloads and summaries.

## Proposed Improvement Approach

1. Publish one composite validation matrix that distinguishes:
   - composite-profile strategies routed through Double-MA validation
   - `CompositeMo` CPU-route validation
   - plain `CompositeCpu` fixed-candidate GPU constraints
2. Keep route-note ownership explicit across request preview, post-run summaries, and report export.
3. Document what is product policy vs technical limitation vs measured CPU preference.
4. Add parity checks so validation summaries and report wording stay synchronized with backend truth.

## Checklist

- [ ] Document the current composite validation matrix by strategy family.
- [ ] Separate preview-only guidance from backend enforcement and exported route notes.
- [ ] Document current CPU-preferred, mixed-route, and GPU-eligible cases explicitly.
- [ ] Verify `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` follow the routed Double-MA validation contract.
- [ ] Verify `CompositeMo` and plain `CompositeCpu` constraints are not conflated.
- [ ] Add or maintain tests for route-note parity across preview, run summary, and report export.
- [ ] Preserve current validation/report parity while improving readability.
- [ ] Avoid collapsing family-specific rules into generic language that becomes false.

## Verification Plan

- run `npm test -- src/services/validationRequestBuilder.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- run validation workspace tests
- run targeted Rust validation command tests
- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`

## Rollout / Fallback / Compatibility Notes

- preserve current family-specific validation routes unless a deliberate backend change lands
- if route-note copy changes, keep exported workbook wording aligned in the same change
- prefer clearer family-specific truth over generic “GPU validation supported” messaging

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`
- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_2026-04-08.md`
