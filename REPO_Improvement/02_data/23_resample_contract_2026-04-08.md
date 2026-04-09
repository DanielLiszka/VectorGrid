# Resample Contract Plan

Last updated: 2026-04-09

## Purpose

Define the current non-options resample contract so derived dataset creation, caching, artifact metadata, and downstream replay behavior stay aligned with the actual repo logic.

## Scope Boundaries

In scope:

- `resample_price_data`
- `AppState::resample_loaded_data`
- resample metadata in artifacts and reports
- derived dataset reuse and invalidation behavior

Out of scope:

- options resampling behavior
- changing the OHLCV resampling algorithm itself
- adding new timeframe syntaxes

## Current User-Facing Behavior

Users can load base data and then request a different timeframe, which creates a derived dataset rather than mutating the base load in place. The app already distinguishes fresh derived resamples in the data layer, reuses an active derived dataset when the same base/timeframe is requested again, and invalidates derived datasets when the base data changes. Persisted artifacts and reports also already carry base/derived timeframe metadata and resample identity fields.

## Current Code Surface

- `src-tauri/src/commands_data.rs`
  - exposes `resample_price_data` and current load/resample command behavior
- `src-tauri/src/state.rs`
  - owns `resample_loaded_data`, derived dataset caching, reuse, and invalidation after base-data changes
- `crates/app_core/src/timeframe.rs`
  - owns timeframe parsing and the underlying candle resampling rules
- `src-tauri/src/run_artifacts.rs`
  - normalizes and persists `base_timeframe`, `derived_timeframe`, `resample_applied`, `resample_rule`, and `resample_anchor`
- `src-tauri/src/commands_reports.rs`
  - carries resample metadata into report payloads
- `web/src/services/dataService.ts`
  - invokes resampling and reflects resample load-source state back to the frontend
- `web/src/stores/dataStore.ts`
  - stores current load-source labels including `derived_fresh_resample`
- `web/src/utils/core/datasetMeta.ts`
  - normalizes base/derived timeframe and resample metadata on the frontend

## Current Verification Surface

- `src-tauri/src/state.rs` tests
- `crates/app_core/src/timeframe.rs` tests
- `web/src/services/dataService.test.ts`
- `web/src/services/historyService.test.ts`
- `src-tauri/src/run_artifacts.rs` tests

## Strengths Already Present

- base and derived datasets are already modeled distinctly instead of being conflated
- active derived datasets are reused when possible instead of being rebuilt every time
- derived datasets are invalidated when the base dataset changes
- artifacts and reports already preserve resample identity fields

## Weaknesses / Failure Modes

- load-time resample behavior, runtime derived-dataset reuse, and persisted resample metadata are easy to document inconsistently
- resample identity is partly runtime state and partly artifact metadata, which invites drift
- users may see the current timeframe without understanding whether it is base or derived unless the contract is explicit
- replay and report flows can quietly lose fidelity if base/derived timeframe semantics are not treated as part of the contract

## Regressions To Avoid

- mutating base data in place instead of keeping derived datasets distinct
- breaking current derived-dataset reuse semantics
- failing to invalidate derived datasets after base changes
- dropping `base_timeframe`, `derived_timeframe`, or `resample_applied` from persisted metadata
- changing resample rule/anchor semantics without updating artifacts and reports
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because resample identity is already part of persisted `dataset_meta`
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259)
  - relevant because the resample contract survives across JSON artifacts and hydration payloads
- [scikit-learn: TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because derived timeframe changes still operate on ordered time-series data and should preserve chronology semantics
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because resampling is a command-mediated boundary crossing from the UI to Rust state
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the resample contract from command call to derived dataset reuse.
2. Separate three things explicitly:
   - base dataset identity
   - derived dataset identity
   - persisted resample metadata
3. Document current reuse and invalidation behavior so replay and history assumptions stay correct.
4. Treat resample metadata as part of reproducibility, not just UI decoration.
5. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document `resample_price_data` request and response behavior.
- [ ] Document how `AppState::resample_loaded_data` creates or reuses derived datasets.
- [ ] Document when derived datasets are invalidated.
- [ ] Document current timeframe parsing and chronology requirements.
- [ ] Document `base_timeframe`, `derived_timeframe`, `resample_applied`, `resample_rule`, and `resample_anchor`.
- [ ] Document frontend load-source behavior for fresh derived resamples.
- [ ] Add or tighten tests if reuse, invalidation, or metadata behavior changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted tests in `src-tauri/src/state.rs`
- run targeted tests in `crates/app_core/src/timeframe.rs`
- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current derived-dataset reuse and invalidation semantics unless a broader data-cache redesign is intentional
- preserve persisted resample metadata across artifacts and reports
- if resample policy changes, update both runtime behavior and metadata contracts in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
