# Double-MA Validation Command Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Double-MA-family validation command path so request admission, backend-route reporting, cross-ticker filtering, and result shaping stay stable across frontend preview and backend execution.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_validation.rs`
- `crates/app_core/src/double_ma_validate.rs`
- Double-MA-family validation command routing for `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`, `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`
- backend-route and fallback reporting returned to the frontend

Out of scope:

- composite validation command ownership
- validation panel presentation details
- options-related validation flows

## Current User-Facing Behavior

The frontend routes Double-MA-like strategies through `run_double_ma_validation`, carrying a `validation_context` that protects dataset, timeframe, and strategy-kind compatibility. The backend returns not only candidate results but also route metadata such as `backend_used`, `backend_route`, and optional `backend_fallback_reason`, which the frontend turns into route summaries and hints.

The current path already supports CPU, `GpuKernel`, and `GpuHybridKernel`-style routing, including route-note behavior when hybrid kernels are disabled and the system falls back to `GpuKernel` or CPU. Cross-ticker preserved-finalists mode also recomputes verdicts after filtering, so the returned recommendation surface reflects the post-filtered finalist set rather than the pre-filtered run.

## Current Code Surface

- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `web/src/services/validationService.ts`
- `web/src/services/validationRequestBuilder.ts`

## Current Verification Surface

- `src-tauri/src/commands_validation.rs` tests
- `crates/app_core/src/double_ma_validate.rs` tests
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`

## Strengths Already Present

- the Double-MA command boundary already enforces `validation_context` compatibility
- backend-route and fallback reporting are already first-class result fields
- Double-MA GPU and hybrid route behavior already have strong backend test coverage
- preserved-finalists cross reruns already trigger verdict recomputation
- the frontend already surfaces route-note/fallback distinctions instead of silently collapsing them

## Weaknesses / Failure Modes

- Double-MA-family routing now covers several strategy aliases and profile variants, so allowed-kind logic is easy to drift
- route-note versus fallback semantics can be blurred if frontend summaries change independently of backend fields
- GPU, hybrid, and CPU paths can diverge in behavior while still looking equivalent from the panel if the returned route strings are not treated as a contract
- cross-ticker preserved-finalists logic adds a second-stage filtering step that can be overlooked during refactors

## Regressions To Avoid

- breaking `validation_context` checks for dataset, timeframe, or strategy-kind compatibility
- changing backend-route strings or fallback-note semantics without updating frontend expectations
- losing verdict recomputation after cross-ticker preserved-finalists filtering
- routing Double-MA-family aliases through the wrong validation command
- weakening GPU/hybrid route coverage while leaving the preview text unchanged
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because typed command results must remain a trustworthy contract back to the frontend.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because frontend route summaries should remain derived from returned command payloads rather than reimplemented heuristics.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because route and verdict expectations are exercised through store/service-driven flows.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU route reporting and fallback behavior should stay evidence-backed and not overclaim unsupported execution paths.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because user-visible route and fallback language should stay testable at the panel layer.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Freeze `run_double_ma_validation` as a typed contract with explicit guarantees for context validation, backend-route reporting, candidate verdicts, and run verdicts.
2. Document the allowed Double-MA-family strategy kinds and keep frontend builder expectations aligned with backend acceptance.
3. Distinguish clearly between:
   - requested route
   - actual backend route used
   - route note
   - hard fallback reason
4. Keep preserved-finalists cross filtering and verdict recomputation documented as part of the command contract.
5. Add or tighten tests for representative CPU, GPU, hybrid, alias-strategy, and cross-filtered runs.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the accepted Double-MA-family strategy kinds for this command.
- [x] Document `validation_context` requirements and rejection behavior.
- [x] Document backend-route, route-note, and fallback fields returned by the command.
- [x] Document preserved-finalists cross-ticker filtering and verdict recomputation behavior.
- [x] Add or tighten tests for alias-strategy acceptance and mismatch rejection.
- [x] Add or tighten tests for GPU, hybrid, and CPU route reporting parity.
- [x] Add or tighten tests for post-cross-filter verdict recomputation.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `commands_validation.rs` tests
- run targeted `double_ma_validate.rs` tests
- run `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/validationService.test.ts`
- manually verify route summaries for one CPU path, one GPU path, and one hybrid-note path from the validation tab
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve current route-string semantics unless a coordinated rename is performed across backend and frontend
- keep alias-strategy support explicit rather than implicit
- prefer additive route explanation over changing backend selection behavior without evidence
- if command behavior changes, update builder previews and service summaries in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Verified the Double-MA command contract against the current backend and frontend surfaces without widening route policy. `commands_validation.rs` already covers strategy-kind allowlists, dataset/timeframe rejection, preserved-finalists cross reruns, verdict recomputation, and representative route-preserving request behavior.
- Acceptance evidence for closure: `cargo test -p ta_desktop_demo_app commands_validation --lib`, `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts`, `npm --prefix web run lint`, and `npm --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/09_eng_validation_verdicts_2026-04-09.md`

