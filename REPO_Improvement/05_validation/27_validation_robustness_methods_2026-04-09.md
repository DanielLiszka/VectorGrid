# Validation Robustness Methods Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the validation robustness-method stack so walk-forward, re-opt walk-forward, perturbation, trade bootstrap, CSCV/PBO, SPA, and related gating semantics stay explicit, comparable, and truthful across frontend and backend surfaces.

## Scope Boundaries

In scope:

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

Out of scope:

- basic candidate-table rendering
- artifact-library replay flows
- options-related validation

## Current User-Facing Behavior

The validation workspace exposes a large robustness stack: walk-forward, finalist re-opt walk-forward, perturbation, trade bootstrap, CSCV/PBO, SPA, cross-ticker reruns, and probability-style hard gates such as PSR, DSR, CSCV thresholds, and max PBO. The builder and service already warn that some of these methods are omitted or reduced under certain routes, especially during cross reruns or some composite-family validations.

The backend then folds these methods into candidate verdicts and run-level recommendations. That means the user-facing panel is already acting as a control surface for a complex method stack, not just a thin trigger UI.

## Current Code Surface

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

## Current Verification Surface

- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`
- `crates/app_core/src/validation_pipeline.rs` tests
- strategy-specific validation tests in `double_ma_validate.rs` and `composite_validate.rs`

## Strengths Already Present

- the method stack is already broader and more explicit than a simple holdout rerun
- builder previews already surface several omitted-module and route-restriction warnings
- backend verdict logic already consumes CSCV, SPA, and track-record-style evidence explicitly
- deterministic tests already exist for some robustness primitives such as SPA determinism and minimum track-record behavior
- the validation panel already groups many of these controls into one analyst workflow

## Weaknesses / Failure Modes

- the method stack is large enough that users can confuse “enabled in UI” with “fully applied on every route”
- some method interactions are subtle:
  - cross reruns omit modules
  - GPU routes may support subsets differently
  - approval logic distinguishes warnings from failures
- current summaries are spread across preview text, warnings, shortlist meta, and recommendation text
- method defaults can silently reshape the product if they drift

## Regressions To Avoid

- letting unsupported methods appear fully active on routes that omit or degrade them
- changing default robustness posture without updating preview text and tests
- weakening CSCV, SPA, or track-record logic while keeping old recommendation language
- hiding omitted-module consequences during cross or composite special-case routes
- making frontend previews less specific than backend behavior
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [TimeSeriesSplit — scikit-learn](https://scikit-learn.org/1.0/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because chronological validation discipline is the baseline reference for split-based robustness evaluation.
- [The Probability of Backtest Overfitting — SSRN](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253) is relevant because CSCV/PBO is a core part of the repo’s robustness-method framing.
- [A Test for Superior Predictive Ability — SSRN](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=264569) is relevant because Hansen SPA is explicitly represented in the repo’s validation pipeline.
- [The Sharp Razor / Minimum Track Record Length](https://www.researchgate.net/publication/256035273_The_Sharp_Razor_Deflating_the_Sharpe_Ratio_by_Asking_for_a_Minimum_Track_Record_Length) is relevant because track-record sufficiency already affects the final recommendation path.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the method stack’s visible contract is currently exercised through frontend builder/service tests as well as backend tests.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Document the robustness stack as a layered method taxonomy instead of one undifferentiated control mass.
2. Distinguish:
   - primary split methods
   - finalist-focused methods
   - bootstrap/significance methods
   - hard-gate thresholds
   - cross-dataset extensions
3. Make route- and mode-specific omissions explicit in the contract.
4. Keep frontend previews, backend verdict logic, and report language aligned to the same method vocabulary.
5. Expand tests around route-specific method omission and recommendation parity.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current robustness method taxonomy and default posture.
- [x] Document which methods are always available versus route-conditional.
- [x] Document omitted/degraded method behavior for cross reruns and constrained composite routes.
- [x] Document how hard gates differ from warning-only methods in the final recommendation.
- [x] Add or tighten tests for route-specific omitted-method summaries.
- [x] Add or tighten tests for method-default behavior and recommendation parity.
- [x] Add or tighten tests for CSCV, SPA, and track-record interactions at the run-verdict layer.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/validationService.test.ts`
- run targeted `validation_pipeline.rs` tests
- run representative `double_ma_validate.rs` and `composite_validate.rs` tests
- manually verify preview summaries for one standard run, one cross run, and one constrained composite route
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve the current method vocabulary unless a coordinated rename/update is made everywhere
- prefer additive clarification over silently changing the enabled-by-default posture
- if a route begins supporting more methods, update preview warnings and tests in the same change
- keep omitted-method messaging explicit even when the route still returns a successful validation result
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the robustness-method stack as a layered contract instead of changing defaults casually. The builder/service tests still prove route-specific omitted-method summaries, while the backend tests now lock CSCV, SPA, track-record, and run-level recommendation interactions.
- Acceptance evidence for closure: `cargo test -p ta_app_core validation_pipeline --lib`, `cargo test -p ta_app_core validation_stats --lib`, `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`, plus `cargo test -p ta_desktop_demo_app commands_validation --lib`.

## Related Docs

- `REPO_Improvement/05_validation/09_eng_validation_verdicts_2026-04-09.md`
- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/26_cross_ticker_finalist_logic_2026-04-09.md`
