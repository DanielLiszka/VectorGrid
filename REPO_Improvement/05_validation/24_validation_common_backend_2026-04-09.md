# Validation Common Backend Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the shared validation backend helpers so cross-ticker canonicalization, gate evaluation, candidate-key stability, and shared robust-score handling remain consistent across Double-MA and composite validation paths.

## Scope Boundaries

In scope:

- `src-tauri/src/validation_common.rs`
- shared cross-ticker config sanitation
- candidate-key canonicalization
- shared cross-ticker stats aggregation, gating, and pareto tagging

Out of scope:

- strategy-specific validation execution internals
- frontend validation presentation
- options-related validation code

## Current User-Facing Behavior

Users do not interact with `validation_common.rs` directly, but several visible behaviors depend on it being stable:

- duplicate cross datasets are deduped
- cross-ticker gates behave the same across validation families
- cross-ticker pareto flags are computed consistently
- robust-score ordering after cross aggregation is consistent
- preserved-finalists and lightweight cross flows operate on canonical candidate identity instead of unstable row order

That means this file is effectively a backend contract layer even though it is not exposed as its own command.

## Current Code Surface

- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

## Current Verification Surface

- `src-tauri/src/validation_common.rs` tests
- `src-tauri/src/commands_validation.rs` tests
- backend validation tests that depend on shared cross behavior

## Strengths Already Present

- shared cross-ticker sanitation is already centralized
- candidate-key canonicalization is already deterministic instead of relying on display order
- shared gate logic already covers dataset count, score, sharpe, pnl-positive fraction, max drawdown, and optional pareto requirement
- robust-score recomputation and cross pareto tagging are already shared instead of duplicated
- both validation families already depend on the same shared candidate trait contract

## Weaknesses / Failure Modes

- small helper changes here can silently alter both validation families at once
- canonicalization, gating, and pareto logic are mathematically dense enough that drift may not be obvious from UI behavior
- cross-dataset dedupe and path normalization are easy to break on Windows-path edge cases
- robust-score updates can appear strategy-specific even though the shared layer is shaping them

## Regressions To Avoid

- changing candidate-key canonicalization so the same finalist maps differently across reruns
- altering gate defaults or normalization rules without updating validation previews and tests
- breaking cross-dataset dedupe or path normalization
- making cross pareto tagging inconsistent between validation families
- modifying shared robust-score updates without verifying downstream verdict ordering
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because candidate-key and cross-stat payloads already behave like structured backend contracts.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because canonical key generation and persisted cross state depend on stable JSON semantics.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because candidate-key stability and config-style canonicalization rely on deterministic object ordering.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because shared backend helpers must not obscure actual route limitations or silently overclaim GPU-consistent semantics.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because frontend-facing summaries should remain aligned with backend contracts that are exercised through service tests.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Treat `validation_common.rs` as a cross-family contract layer with explicit invariants for path normalization, candidate identity, cross stats, and gate evaluation.
2. Document the candidate-key canonicalization and cross-dataset dedupe rules in one place.
3. Freeze gate semantics and defaults so frontend previews can rely on them.
4. Document robust-score updates and cross pareto tagging as shared backend meaning, not strategy-local quirks.
5. Expand tests around path normalization, candidate canonicalization, gate behavior, and pareto marking.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document shared cross-ticker sanitation and dedupe behavior.
- [x] Document candidate-key canonicalization invariants.
- [x] Document cross-gate semantics and defaults.
- [x] Document how robust-score and cross pareto updates are computed.
- [x] Add or tighten tests for Windows-path normalization and duplicate dataset handling.
- [x] Add or tighten tests for candidate-key stability across equivalent JSON shapes.
- [x] Add or tighten tests for gate and cross-pareto behavior after aggregation.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `validation_common.rs` tests
- run targeted `commands_validation.rs` cross-ticker tests
- run representative frontend validation-service tests that assert cross summaries and row semantics
- manually verify one Double-MA and one composite cross-validation run still surface consistent cross summaries
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- keep shared helper semantics stable unless a coordinated frontend/backend update is made
- prefer additive documentation and tests over behavior changes in this layer
- if gate semantics change, update request-builder preview text and validation docs at the same time
- treat candidate-key stability as compatibility-sensitive behavior
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added direct shared-backend regression coverage in `validation_common.rs` for canonical candidate-key stability across equivalent nested JSON shapes, on top of the existing path normalization, dedupe, gating, and robust-score folding tests. This keeps the cross-family backend contract explicit rather than relying on downstream behavior only.
- Acceptance evidence for closure: `cargo test -p ta_desktop_demo_app validation_common --lib`, `cargo test -p ta_desktop_demo_app commands_validation --lib`, and the frontend validation request/service suites that depend on the same canonicalization behavior.

## Related Docs

- `REPO_Improvement/05_validation/06_val_dm_command_2026-04-09.md`
- `REPO_Improvement/05_validation/07_val_composite_command_2026-04-09.md`
- `REPO_Improvement/05_validation/26_cross_ticker_finalist_logic_2026-04-09.md`
- `REPO_Improvement/05_validation/27_validation_robustness_methods_2026-04-09.md`
