# Validation Context Compatibility Plan

Last updated: 2026-04-08

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

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces that the backend command contract remains the final compatibility authority.
- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html) is relevant because validation context is assembled from shared workflow state and hydrated artifacts.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting a canonical validation-context payload.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant as an authoritative reminder that time-ordered validation requires explicit context and cannot rely on generic dataset assumptions.

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

## Checklist

- [ ] Define the canonical validation-context tuple and nullability rules.
- [ ] Document per-command strategy-kind compatibility requirements.
- [ ] Align frontend preview/dock copy with Rust error semantics.
- [ ] Define thin-metadata and legacy-artifact handling rules.
- [ ] Surface explicit compatibility status in the validation dock.
- [ ] Add tests for mismatched dataset path, timeframe, and strategy kind.
- [ ] Add tests for incomplete artifact metadata and replay edge cases.
- [ ] Preserve current valid workflows while tightening invalid-context detection.

## Verification Plan

- run `npm test -- src/services/validationRequestBuilder.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- run validation workspace/dock component tests
- run targeted Rust validation command tests

## Rollout / Fallback / Compatibility Notes

- keep Rust-side command validation authoritative
- treat richer frontend compatibility signaling as additive, not as a replacement for backend checks
- if legacy artifacts are partially compatible, surface that state explicitly instead of silently guessing

## Related Docs

- `REPO_Improvement/05_validation/19_validation_request_builder_DATE.md`
- `REPO_Improvement/05_validation/20_validation_service_DATE.md`
- `REPO_Improvement/05_validation/22_validation_context_docks_DATE.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_DATE.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
