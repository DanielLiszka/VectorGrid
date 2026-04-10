# Validation Stats Core Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the core validation statistics layer so probability gates, PSR/DSR calculations, and shared numeric sanitation remain explicit and consistent across backend verdicts, exports, and frontend summaries.

## Scope Boundaries

In scope:

- `crates/app_core/src/validation_stats.rs`
- probability-gate sanitation
- PSR/DSR computation and gating helpers
- shared statistical inputs consumed by validation verdict logic

Out of scope:

- higher-level run-verdict aggregation already covered by the verdict doc
- panel rendering details
- options-related statistical logic

## Current User-Facing Behavior

Users see PSR/DSR thresholds as validation controls, row-level table fields, and run-level recommendation context. Under the hood, the shared stats layer sanitizes probability thresholds into `[0, 1]`, computes PSR/DSR from Sharpe-oriented inputs, and decides whether configured probability gates pass.

These helpers are small, but they define core semantics that propagate into candidate verdicts, shortlist selection, workbook output, and frontend summaries.

## Current Code Surface

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_reports_validation_common.rs`

## Current Verification Surface

- `crates/app_core/src/validation_stats.rs` tests
- `crates/app_core/src/validation_pipeline.rs` tests
- strategy-specific validation tests that exercise probability-gate behavior
- report/export tests that depend on verdict fields derived from these stats

## Strengths Already Present

- probability-gate sanitation is already centralized
- PSR/DSR computation is already shared instead of duplicated per strategy
- gating semantics are already explicit and test-covered
- downstream verdict logic already consumes these outputs consistently
- row/export/report surfaces already expose the resulting values

## Weaknesses / Failure Modes

- these small helpers can be changed casually even though they shape multiple surfaces
- PSR/DSR semantics can drift from UI labels if not treated as a contract
- sanitization rules and gating rules are easy to conflate
- downstream users may assume “missing” and “failed” are identical when they are not

## Regressions To Avoid

- changing probability-gate clamping semantics without updating control/summary expectations
- altering PSR/DSR computation assumptions silently
- making gating logic inconsistent between candidate verdicts and exported/report surfaces
- collapsing missing-stat and below-threshold behavior into one undocumented branch
- duplicating probability-gate logic outside the shared stats layer
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [The Sharp Razor / Minimum Track Record Length](https://www.researchgate.net/publication/256035273_The_Sharp_Razor_Deflating_the_Sharpe_Ratio_by_Asking_for_a_Minimum_Track_Record_Length) is relevant because the repo’s statistical posture already incorporates track-record sufficiency alongside Sharpe-derived measures.
- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because PSR/DSR and gate outputs become structured contract fields in results, reports, and artifacts.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because absent-versus-present numeric fields affect how downstream surfaces interpret missing stats.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because frontend summaries/tests rely on deterministic statistical fields coming from shared backend logic.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because the user-visible consequence of these helpers should remain observable through the validation workflow.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Freeze the stats core as the sole owner of PSR/DSR sanitation and gating semantics.
2. Document the distinction between:
   - sanitizing configured thresholds
   - computing PSR/DSR values
   - deciding gate pass/fail
3. Keep downstream verdict/export consumers aligned to these meanings.
4. Add targeted tests around edge cases such as `NaN`, absent values, and mixed threshold configurations.
5. Treat field meaning as compatibility-sensitive because it drives multiple user-facing surfaces.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the shared PSR/DSR helper contract and inputs.
- [x] Document threshold sanitation and gate-evaluation rules.
- [x] Document how missing stats should be interpreted downstream.
- [x] Add or tighten tests for clamping, `NaN`, and absent-value behavior.
- [x] Add or tighten tests for gate pass/fail combinations.
- [x] Verify downstream verdict/report consumers still interpret these fields consistently.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `validation_stats.rs` tests
- run targeted `validation_pipeline.rs` tests
- run representative strategy validation tests that exercise probability gates
- run report tests that depend on PSR/DSR presence and recommendation behavior
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- keep statistical semantics centralized here rather than duplicating them in verdict or report layers
- prefer additive edge-case tests over semantic rewrites
- if PSR/DSR field meaning changes, update control labels, summaries, and report wording together
- treat absent-value handling as contract behavior, not an implementation accident
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Strengthened the shared stats-layer contract in `validation_stats.rs` with explicit tests for bounded PSR/DSR outputs, threshold sanitation, and missing-stat behavior when probability gates are configured. This keeps the low-level statistical contract aligned with the downstream verdict layer.
- Acceptance evidence for closure: `cargo test -p ta_app_core validation_stats --lib`, `cargo test -p ta_app_core validation_pipeline --lib`, and the downstream report/export suites that consume the resulting verdict fields.

## Related Docs

- `REPO_Improvement/05_validation/09_eng_validation_verdicts_2026-04-09.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/27_validation_robustness_methods_2026-04-09.md`
- `REPO_Improvement/05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md`
