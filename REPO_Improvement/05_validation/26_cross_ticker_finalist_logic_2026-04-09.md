# Cross-Ticker Finalist Logic Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to cross-ticker finalist logic so preserved-finalists selection, dataset alignment, cap behavior, gating, and post-filter verdict recomputation stay explicit and defensible.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- cross-ticker preserved-finalists and lightweight modes
- finalist-cap behavior and post-filter verdict recomputation

Out of scope:

- baseline single-dataset validation without cross reruns
- validation table rendering details
- options-related flows

## Current User-Facing Behavior

Cross-ticker validation is not a naive rerun of the full validation stack. The backend sanitizes the selected dataset list, aligns cross datasets to the base horizon, applies shared aggregation/gating, and in preserved-finalists mode limits the cross-rerun set to a small finalist cap. The frontend surfaces this through cross summaries, warnings about omitted modules, and shortlist/rule text that changes when cross behavior is in play.

The current command path also recomputes the run verdict after cross filtering. That is important: if cross reruns remove previously approved finalists, the reported `approved_candidate_count` and final recommendation change to match the filtered set rather than the original pre-cross output.

## Current Code Surface

- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`

## Current Verification Surface

- `src-tauri/src/commands_validation.rs` tests
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`

## Strengths Already Present

- cross-dataset alignment to the base horizon is already explicit
- preserved-finalists mode already has a fixed cap and deterministic filtering path
- post-cross-filter verdict recomputation already exists
- frontend summaries already distinguish lightweight and preserved-finalists semantics
- cross gates and cross pareto logic already have a shared backend layer

## Weaknesses / Failure Modes

- cross behavior is complex enough that users can over-assume it is “full validation on more datasets”
- cap behavior can look arbitrary unless documented clearly
- omitted modules during cross reruns can be forgotten if warnings and summaries drift
- alignment and filtering rules are backend-only today unless explicitly documented
- the same cross summary is used to explain several distinct things:
  - dataset count
  - mode
  - gate behavior
  - omitted modules

## Regressions To Avoid

- changing the preserved-finalists cap or selection order without updating warnings and tests
- failing to recompute verdicts after cross filtering
- breaking base-horizon alignment for cross datasets
- silently expanding cross reruns to modules that are currently intentionally omitted
- making frontend summaries imply stronger cross guarantees than the backend actually provides
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [TimeSeriesSplit — scikit-learn](https://scikit-learn.org/1.0/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because cross-dataset evaluation still has to respect chronological structure and comparable horizons.
- [The Probability of Backtest Overfitting — SSRN](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253) is relevant because cross-ticker filtering is part of the repo’s broader attempt to reduce overconfident finalist selection.
- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because cross-ticker effects show up directly in dense result tables that need to remain interpretable.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because current cross-mode behavior is already validated through builder and service tests.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because cross-mode warnings and row-level consequences should remain visible in user-observable flows.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Document cross-ticker logic as a two-mode contract:
   - lightweight reruns
   - preserved-finalists reruns
2. Make finalist-cap behavior, selection order, and post-filter verdict recomputation explicit.
3. Document base-horizon alignment and omitted-module behavior as first-class semantics.
4. Keep frontend cross summaries aligned with the exact backend rules they are describing.
5. Expand tests around cap behavior, alignment behavior, and recomputed run verdicts.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the preserved-finalists versus lightweight cross modes.
- [ ] Document finalist-cap behavior and selection-order rules.
- [ ] Document base-horizon alignment for cross datasets.
- [ ] Document which validation modules are omitted during cross reruns and why.
- [ ] Document post-cross-filter verdict recomputation behavior.
- [ ] Add or tighten tests for finalist-cap behavior and recomputed approvals.
- [ ] Add or tighten tests for frontend cross summaries and omitted-module warnings.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `commands_validation.rs` cross-ticker tests
- run `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/validationService.test.ts`
- manually verify one preserved-finalists run and one lightweight run in the validation tab
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve current preserved-finalists cap unless there is a deliberate product change
- prefer better explanation over broader cross execution promises
- if omitted modules change, update preview text, summary text, and tests together
- keep cross behavior additive and explicit rather than silently “upgrading” it
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/05_validation/24_validation_common_backend_2026-04-09.md`
- `REPO_Improvement/05_validation/09_eng_validation_verdicts_2026-04-09.md`
- `REPO_Improvement/05_validation/27_validation_robustness_methods_2026-04-09.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
