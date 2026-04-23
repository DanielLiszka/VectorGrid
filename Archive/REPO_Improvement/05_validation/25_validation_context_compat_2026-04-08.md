# Validation Context Compatibility Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the validation-context compatibility contract so non-options validation requests, docks, and backend commands agree on when a candidate can be validated against a given dataset, timeframe, and strategy family.

## Scope Boundaries

In scope:

- validation context tuple and compatibility rules
- builder/service/backend alignment
- validation dock truthfulness
- legacy or thin-metadata artifact handling

Out of scope:

- options validation surfaces
- workbook/report presentation quality
- redesigning the entire validation workflow

## Current User-Facing Behavior

Users can open validation with context hydrated from current workflow state or artifacts. The validation dock presents summary information, but compatibility is not always surfaced as an explicit pass/fail state. Some requests can be built from incomplete metadata and only fail later when Rust checks required fields or supported strategy kinds.

## Current Code Surface

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/components/ValidationContextDock.vue`
- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationRequestBuilder.test.ts`

## Current Verification Surface

- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`
- validation workspace component tests
- Rust validation command tests

## Strengths Already Present

- validation requests already go through a dedicated builder/service path
- the UI already exposes a validation-context dock instead of hiding provenance entirely
- Rust handlers already guard key strategy and dataset/timeframe assumptions
- artifact hydration already carries meaningful context into validation flows

## Weaknesses / Failure Modes

- the canonical validation-context tuple is not clearly documented
- builder output can contain thin or null metadata that only fails later
- strategy-kind allowlists are authoritative in Rust while the frontend remains partly advisory
- the dock shows metadata but not always explicit compatibility status
- legacy artifacts with incomplete metadata remain risky to hydrate

## Regressions To Avoid

- blocking currently valid validation flows with overly strict frontend assumptions
- letting incompatible contexts advance farther before failing
- weakening Rust-side validation authority
- making artifact replay less reliable for existing saved runs
- removing helpful dock context in the name of simplification
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces that the backend command contract remains the final compatibility authority.
- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html) is relevant because validation context is assembled from shared workflow state and hydrated artifacts.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting a canonical validation-context payload.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant as an authoritative reminder that time-ordered validation requires explicit context and cannot rely on generic dataset assumptions.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Define the canonical validation-context tuple:
   - strategy kind
   - dataset identity/path
   - timeframe
   - source artifact or current-run provenance
   - any additional strategy-family requirements
2. Align builder, service, dock, and Rust command boundaries to that tuple.
3. Surface compatibility status explicitly in the dock instead of relying on inferred metadata quality.
4. Document how thin/legacy artifacts are handled, including when the user should be warned, blocked, or allowed to continue with degraded confidence.
5. Expand tests around mismatched dataset paths, timeframes, strategy kinds, and incomplete artifact metadata.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define the canonical validation-context tuple and nullability rules.
- [x] Document per-command strategy-kind compatibility requirements.
- [x] Align frontend preview/dock copy with Rust error semantics.
- [x] Define thin-metadata and legacy-artifact handling rules.
- [x] Surface explicit compatibility status in the validation dock.
- [x] Add tests for mismatched dataset path, timeframe, and strategy kind.
- [x] Add tests for incomplete artifact metadata and replay edge cases.
- [x] Preserve current valid workflows while tightening invalid-context detection.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/services/validationRequestBuilder.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- run validation workspace/dock component tests
- run targeted Rust validation command tests
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- keep Rust-side command validation authoritative
- treat richer frontend compatibility signaling as additive, not as a replacement for backend checks
- if legacy artifacts are partially compatible, surface that state explicitly instead of silently guessing
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added a shared frontend compatibility preview in `web/src/services/validationService.ts` that evaluates dataset-path, timeframe, and strategy-kind alignment from the saved run artifact without weakening the existing Rust-side command checks.
- Surfaced that preview in `web/src/components/ValidationContextDock.vue` as an explicit compatibility row with state-specific styling and tooltip detail, so the dock now calls out `compatible`, `warning`, `incompatible`, and `missing` states instead of leaving users to infer status from raw metadata alone.
- Added regression coverage in `web/src/services/validationService.test.ts` and `web/src/components/ValidationContextDock.test.ts` for matching, mismatched, and thin-metadata artifact cases. Closure evidence: `npm.cmd --prefix web run test -- src/components/ValidationContextDock.test.ts src/services/validationService.test.ts src/components/ValidationWorkspacePanel.test.ts src/services/validationRequestBuilder.test.ts src/stores/validationStore.actions.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/22_validation_context_docks_2026-04-08.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`

## Execution Notes

- Completed as an additive frontend/service hardening slice without changing backend authority or blocking existing valid validation flows.
- Consolidated `web/src/services/validationService.ts` to one canonical compatibility-preview helper and wired it into `web/src/components/ValidationContextDock.vue` so the dock now surfaces explicit `compatible`, `warning`, `incompatible`, or `missing` state instead of only metadata hints.
- Kept Rust validation authority unchanged in `src-tauri/src/commands_validation.rs`; the dock preview remains advisory and mirrors the same dataset/timeframe/strategy checks already enforced before validation runs.
- Added targeted service and dock coverage for compatible, incompatible, thin-metadata, and saved-validation strategy-mismatch scenarios in `web/src/services/validationService.test.ts` and `web/src/components/ValidationContextDock.test.ts`.
- Verified with:
  - `npm.cmd --prefix web run test -- src/services/validationService.test.ts src/components/ValidationContextDock.test.ts src/components/ValidationWorkspacePanel.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - `cargo check -p ta_desktop_demo_app`
