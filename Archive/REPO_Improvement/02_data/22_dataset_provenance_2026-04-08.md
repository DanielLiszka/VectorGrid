# Dataset Provenance Plan

Last updated: 2026-04-09

## Purpose

Define the current non-options dataset-provenance contract so loaded data, derived data, run artifacts, reports, and history surfaces preserve the dataset identity information the app already computes.

## Scope Boundaries

In scope:

- loaded dataset identity fields
- persisted `dataset_meta`
- source-content fingerprint usage
- report and artifact provenance carry-through
- user-visible provenance summaries already present in history and shell surfaces

Out of scope:

- options dataset provenance
- adding a brand-new provenance model
- purely theoretical provenance standards work disconnected from repo behavior

## Current User-Facing Behavior

The app already exposes dataset provenance in several practical ways. Loaded and resampled data carry timeframe hints and load-source state. Saved run artifacts preserve `dataset_meta`, including timeframe and source-identity information. History and replay flows reuse that metadata to reload data, summarize validation-admission provenance, and warn when datasets or validation context are stale or changed. The user can already benefit from provenance, but the exact contract is spread across data loading, artifact enrichment, and history presentation.

## Current Code Surface

- `src-tauri/src/state.rs`
  - owns loaded-data context and derived/resampled dataset reuse behavior
- `src-tauri/src/commands_data.rs`
  - exposes load and resample commands and current load-source behavior
- `src-tauri/src/run_artifacts.rs`
  - enriches and normalizes `dataset_meta`, including `schema_version`, timeframe identity, resample identity, source size, source mtime, and source content fingerprint
- `src-tauri/src/commands_reports.rs`
  - carries persisted `dataset_meta` into report-building paths
- `web/src/utils/core/datasetMeta.ts`
  - normalizes dataset-meta structures on the frontend
- `web/src/services/historyService.ts`
  - consumes artifact provenance, dataset paths, timeframes, and validation-admission provenance summaries
- `web/src/services/dataService.ts`
  - reflects load-source and resample-related data behavior back to the shell
- `web/src/stores/dataStore.ts`
  - stores load-source labels like `derived_fresh_resample`

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs` tests
- `src-tauri/src/state.rs` tests
- `web/src/services/dataService.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- persisted artifacts already carry `dataset_meta` instead of path-only provenance
- source-content fingerprinting already exists for stronger dataset identity than path alone
- reports already reuse persisted dataset metadata rather than rebuilding a weaker summary
- the history layer already surfaces validation-admission provenance summaries

## Weaknesses / Failure Modes

- provenance is split across runtime loaded-data state, artifact enrichment, and history/report consumers
- path, timeframe, and fingerprint precedence rules are easy to misstate because multiple fallbacks exist
- provenance can appear “complete” from a UI perspective while still omitting source identity fields in some flows
- derived/resampled datasets are operationally important, but the provenance difference between base and derived datasets is easy to blur

## Regressions To Avoid

- dropping `dataset_meta` from saved artifacts or reports
- weakening current source-content fingerprint capture
- reducing provenance to only `csv_path` and `timeframe`
- misdocumenting base versus derived dataset identity
- changing history or replay behavior without preserving current dataset provenance cues
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [W3C PROV Overview](https://www.w3.org/TR/prov-overview/)
  - relevant because this repo already does practical provenance capture and should keep entity/source relationships explicit rather than implicit
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because `dataset_meta` is a structured persisted contract, not just free-form metadata
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV remains the base external dataset format for the non-options workflows
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259)
  - relevant because provenance is ultimately persisted as JSON in run artifacts and hydration payloads

## Proposed Improvement Approach

1. Document provenance by lifecycle stage:
   - load-time dataset context
   - derived/resampled dataset context
   - artifact enrichment
   - report carry-through
   - history/replay consumption
2. Make the current provenance fields explicit, especially source identity and resample identity.
3. Document fallback and precedence rules so replay, hydration, and reports stay aligned.
4. Treat provenance as an existing reproducibility contract, not optional nice-to-have metadata.
5. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Enumerate the current `dataset_meta` fields used by artifacts and reports.
- [x] Document source-identity fields, including size, mtime, and content fingerprint.
- [x] Document timeframe, base-timeframe, and derived-timeframe behavior.
- [x] Document where validation-admission provenance is separate from dataset provenance.
- [x] Document frontend dataset-meta normalization behavior.
- [x] Document how history/replay uses persisted provenance.
- [x] Add or tighten tests if provenance fields or precedence rules change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted tests in `src-tauri/src/run_artifacts.rs`
- run targeted tests in `src-tauri/src/state.rs`
- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target

## Rollout / Fallback / Compatibility Notes

- keep provenance additive and backward-compatible
- preserve current fingerprint and timeframe identity behavior unless a stronger replacement is implemented
- do not narrow provenance to only path-based identity
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added `web/src/utils/core/datasetMeta.test.ts` to pin the frontend artifact-provenance builder to the shipped field inventory: schema version, base-versus-derived timeframe fields, resample identity, and source-content fingerprint handling are now asserted directly.
- Tightened `web/src/services/historyService.test.ts` so hydrated run artifacts keep dataset provenance explicit during replay paths, including `base_timeframe`, `derived_timeframe`, `resample_applied`, and `source_content_fingerprint`.
- Matched the frontend expectation to the backend-normalized contract already implemented in `src-tauri/src/run_artifacts.rs`: even when `resample_applied` is false, persisted metadata still carries the canonical resample rule/anchor when a derived-timeframe field is present.
- Acceptance evidence captured during execution: `cargo test -p ta_desktop_demo_app state --lib`, `cargo test -p ta_desktop_demo_app run_artifacts --lib`, `cargo test -p ta_app_core timeframe`, `npm.cmd --prefix web run test -- src/components/DataWorkspacePanel.test.ts src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/utils/core/datasetMeta.test.ts src/services/historyService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/02_data/23_resample_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
