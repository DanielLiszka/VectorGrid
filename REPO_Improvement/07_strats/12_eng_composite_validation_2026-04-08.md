# Composite Validation Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because validation is a time-ordered contract, not just a post-processing step.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because preview and actual validation execution are separated by a command boundary.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU-validation route claims need evidence-backed host/device boundary discipline.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting route- and family-sensitive validation payloads and summaries.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Publish one composite validation matrix that distinguishes:
   - composite-profile strategies routed through Double-MA validation
   - `CompositeMo` CPU-route validation
   - plain `CompositeCpu` fixed-candidate GPU constraints
2. Keep route-note ownership explicit across request preview, post-run summaries, and report export.
3. Document what is product policy vs technical limitation vs measured CPU preference.
4. Add parity checks so validation summaries and report wording stay synchronized with backend truth.
5. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current composite validation matrix by strategy family.
- [x] Separate preview-only guidance from backend enforcement and exported route notes.
- [x] Document current CPU-preferred, mixed-route, and GPU-eligible cases explicitly.
- [x] Verify `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` follow the routed Double-MA validation contract.
- [x] Verify `CompositeMo` and plain `CompositeCpu` constraints are not conflated.
- [x] Add or maintain tests for route-note parity across preview, run summary, and report export.
- [x] Preserve current validation/report parity while improving readability.
- [x] Avoid collapsing family-specific rules into generic language that becomes false.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `npm test -- src/services/validationRequestBuilder.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- run validation workspace tests
- run targeted Rust validation command tests
- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current family-specific validation routes unless a deliberate backend change lands
- if route-note copy changes, keep exported workbook wording aligned in the same change
- prefer clearer family-specific truth over generic “GPU validation supported” messaging
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by making the composite validation matrix explicit in [REPO_MAP.md](/C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md): routed Double-MA composite profiles, plain `CompositeCpu`, `CompositeMo`, and standalone Cluster A/B families now have one shared truth surface describing which validation path they actually use.
- Tightened report/export parity in [validation_xlsx_report_tests.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/tests/validation_xlsx_report_tests.rs) by adding workbook assertions for:
  - `CompositeMo` CPU-route fallback notes
  - plain `CompositeCpu` GPU-route compatibility notes
- Existing preview and post-run parity remains the truth anchor and was intentionally preserved:
  - preview/request matrix in `web/src/services/validationRequestBuilder.test.ts`
  - post-run route summaries in `web/src/services/validationService.test.ts`
  - exported workbook route notes in `src-tauri/tests/validation_xlsx_report_tests.rs`
- Stable truth anchors:
  - routed Double-MA validation in `crates/app_core/src/double_ma_validate.rs`
  - composite validation helper/report wording in `src-tauri/src/commands_reports_validation_common.rs`
  - frontend preview and post-run summaries in `validationRequestBuilder.ts` and `validationService.ts`
- Acceptance evidence:
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
  - `npm.cmd --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`
- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_2026-04-08.md`
