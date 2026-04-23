# Validation Request Builder Plan

Last updated: 2026-04-09

## Purpose

Define the current validation-request builder contract so candidate admission, module enablement, cross-dataset behavior, hard-gate defaults, and validation-context carry-through stay aligned between the frontend and Rust validation commands.

## Scope Boundaries

In scope:

- `web/src/services/validationRequestBuilder.ts`
- validation request shaping and normalization
- admission policy and admission provenance
- module enablement and backend shaping
- validation context and cross-dataset shaping

Out of scope:

- validation execution internals after the request crosses into Rust
- workbook/report generation
- options validation

## Current User-Facing Behavior

The validation UI does not send raw top-table state directly to Rust. Instead, the frontend builds a structured validation payload from the current run request, current run result, candidate rows, gate controls, module toggles, and cross-dataset inputs. The builder already normalizes strategy kind, backend, candidate admission policy, module defaults, cross-ticker mode, and validation context. It also records `admission_provenance`, which is later surfaced in history summaries.

## Current Code Surface

- `web/src/services/validationRequestBuilder.ts`
  - owns validation request shaping, candidate normalization, admission policy selection, module config building, cross-ticker config, and validation-context emission
- `web/src/services/validationRequestBuilder.test.ts`
  - covers double-like requests, composite requests, default admission behavior, scalar-union behavior, and multiple config edge cases
- `web/src/services/validationService.ts`
  - consumes the built request and coordinates command invocation plus result handling
- `src-tauri/src/commands_validation.rs`
  - receives the request and enforces compatibility and runtime validation boundaries
- `src-tauri/src/validation_common.rs`
  - provides shared validation command helpers and compatibility logic
- `web/src/services/historyService.ts`
  - reuses validation request shape and provenance during replay and saved-run workflows

## Current Verification Surface

- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`
- targeted tests in `src-tauri/src/commands_validation.rs`
- `web/src/services/historyService.test.ts`

## Strengths Already Present

- validation request building is centralized and test-covered
- the request already carries explicit `admission_policy` and `admission_provenance`
- validation context is explicit instead of being inferred late from whatever data happens to be loaded
- cross-dataset behavior is already capped and normalized rather than blindly accepting raw user input

## Weaknesses / Failure Modes

- validation request shaping is subtle because it combines current run state, UI toggles, and top-row candidate state
- module enablement can drift from actual backend compatibility if the builder and capability summaries stop matching
- candidate admission policy and provenance are important enough to be user-facing, but easy to treat as implementation detail
- backend compatibility and builder normalization can drift if validation context and strategy-kind normalization are not documented together

## Regressions To Avoid

- changing admission-policy defaults without updating tests and history summaries
- weakening validation-context emission or compatibility checking
- silently broadening cross-ticker dataset admission or mode behavior
- sending candidates that no longer match current strategy-kind expectations
- dropping `admission_provenance` or shrinking it into an undocumented field
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the validation request is a real cross-layer contract with nested optional modules
- [scikit-learn: TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because multiple validation modules are chronology-sensitive and should remain explicit about ordered evaluation assumptions
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the builder emits the exact command payload sent across the Tauri boundary
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259)
  - relevant because admission provenance and validation context are persisted in JSON artifacts and replay paths
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Document the request by subsystem:
   - identity and backend
   - candidate admission
   - module config
   - cross-dataset config
   - hard gates
   - validation context
2. Separate product-policy normalization from backend-required invariants.
3. Make `admission_policy` and `admission_provenance` first-class parts of the contract.
4. Tie candidate shaping, module shaping, and compatibility shaping to the tests that already protect them.
5. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Enumerate required top-level validation request fields.
- [x] Document strategy-kind normalization and backend shaping rules.
- [x] Document candidate-source selection and admission-policy behavior.
- [x] Document `admission_provenance` structure and current meaning.
- [x] Document module config defaults and clamps for walk-forward, reopt, SPA, perturb, bootstrap, CSCV, and cross-ticker.
- [x] Document validation context fields and compatibility expectations.
- [x] Document cross-dataset caps and preserved-finalists behavior.
- [x] Add or tighten tests if request-shaping behavior changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/validationService.test.ts`
- run targeted tests in `src-tauri/src/commands_validation.rs`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve current request field names and admission semantics unless a migration is documented
- preserve validation context and provenance richness rather than simplifying them away
- if builder refactoring happens, keep emitted payload shape stable first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the request-builder contract against the current frontend and backend tests rather than widening the payload shape. The existing builder suite still covers admission policy and provenance, module defaults, cross-dataset shaping, compatibility summaries, and validation-context emission.
- Acceptance evidence for closure: `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts src/services/historyService.test.ts`, plus `cargo test -p ta_desktop_demo_app commands_validation --lib`, `npm --prefix web run lint`, and `npm --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`
- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/02_data/23_resample_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
