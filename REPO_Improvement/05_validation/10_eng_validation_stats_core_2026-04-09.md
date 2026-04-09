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

## External Research Summary

- [The Sharp Razor / Minimum Track Record Length](https://www.researchgate.net/publication/256035273_The_Sharp_Razor_Deflating_the_Sharpe_Ratio_by_Asking_for_a_Minimum_Track_Record_Length) is relevant because the repo’s statistical posture already incorporates track-record sufficiency alongside Sharpe-derived measures.
- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because PSR/DSR and gate outputs become structured contract fields in results, reports, and artifacts.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because absent-versus-present numeric fields affect how downstream surfaces interpret missing stats.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because frontend summaries/tests rely on deterministic statistical fields coming from shared backend logic.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because the user-visible consequence of these helpers should remain observable through the validation workflow.

## Proposed Improvement Approach

1. Freeze the stats core as the sole owner of PSR/DSR sanitation and gating semantics.
2. Document the distinction between:
   - sanitizing configured thresholds
   - computing PSR/DSR values
   - deciding gate pass/fail
3. Keep downstream verdict/export consumers aligned to these meanings.
4. Add targeted tests around edge cases such as `NaN`, absent values, and mixed threshold configurations.
5. Treat field meaning as compatibility-sensitive because it drives multiple user-facing surfaces.

## Checklist

- [ ] Document the shared PSR/DSR helper contract and inputs.
- [ ] Document threshold sanitation and gate-evaluation rules.
- [ ] Document how missing stats should be interpreted downstream.
- [ ] Add or tighten tests for clamping, `NaN`, and absent-value behavior.
- [ ] Add or tighten tests for gate pass/fail combinations.
- [ ] Verify downstream verdict/report consumers still interpret these fields consistently.

## Verification Plan

- run targeted `validation_stats.rs` tests
- run targeted `validation_pipeline.rs` tests
- run representative strategy validation tests that exercise probability gates
- run report tests that depend on PSR/DSR presence and recommendation behavior

## Rollout / Fallback / Compatibility Notes

- keep statistical semantics centralized here rather than duplicating them in verdict or report layers
- prefer additive edge-case tests over semantic rewrites
- if PSR/DSR field meaning changes, update control labels, summaries, and report wording together
- treat absent-value handling as contract behavior, not an implementation accident

## Related Docs

- `REPO_Improvement/05_validation/09_eng_validation_verdicts_2026-04-09.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/27_validation_robustness_methods_2026-04-09.md`
- `REPO_Improvement/05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md`
