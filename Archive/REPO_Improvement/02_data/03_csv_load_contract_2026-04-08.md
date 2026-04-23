# CSV Load Contract Plan

Last updated: 2026-04-09

## Purpose

Define the authoritative contract for local CSV loading and resample-adjacent load results.

## Scope Boundaries

In scope:

- `load_price_data`
- frontend normalization in `dataService`
- CSV header compatibility
- load-source semantics
- returned metadata and error semantics

Out of scope:

- Polygon network download behavior
- options data

## Current User-Facing Behavior

Users can browse to a CSV, load it, and receive normalized dataset metadata plus a `loadSource` value that distinguishes parsed loads, cache hits, and derived/in-memory reuse. The loader supports both named-header CSVs and positional formats.

## Current Code Surface

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/csv_compat.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`

## Current Verification Surface

- `crates/app_core/src/csv_compat.rs` compatibility tests
- `src-tauri/src/commands_data.rs` repo-data helper tests
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `src-tauri` state/data tests

## Strengths Already Present

- the frontend already normalizes response data into a stable summary shape
- the backend exposes explicit `cache_hit`, `cache_path`, and `load_source` fields
- `csv_compat.rs` already supports named and positional layouts

## Weaknesses / Failure Modes

- header compatibility rules are implicit in code rather than documented as a stable contract
- `loadSource` semantics are easy to misread in the UI
- failure modes for missing fields, malformed timestamps, or partial OHLCV data are not described in one place

## Regressions To Avoid

- breaking compatibility with current supported CSV layouts
- changing `loadSource` strings without updating frontend expectations
- losing dataset metadata fields relied on by artifacts and UI state
- changing derived/resample identity rules accidentally during load work
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) remains the baseline CSV reference even though real-world market data often needs compatibility handling beyond the base format.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting explicit response fields and schema/version discipline.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because `load_price_data` is a command boundary with an explicit payload contract.

## Proposed Improvement Approach

1. Define the accepted CSV shapes and header aliases in a doc table.
2. Define metadata guarantees for successful loads.
3. Define `loadSource` semantics as an explicit enum contract, not an implementation detail.
4. Define how missing open/high/low/volume fields affect downstream capability.
5. Define the error taxonomy for malformed files and invalid timestamps.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- replaced manual comma splitting in `crates/app_core/src/csv_compat.rs` with `csv::ReaderBuilder`, using standards-aware parsing while preserving the existing named-header and positional fallback contract
- kept the frontend-visible load result stable: `load_price_data` still feeds the same `loadSource`, metadata, and cache semantics that `dataService.ts` and `dataStore.ts` normalize today
- preserved current failure posture for malformed timestamps and missing required columns, but improved the parser so quoted headers and unused quoted fields containing commas no longer break import compatibility
- revalidated the command-side consumer expectations through `dataService.test.ts`, `dataStore.actions.test.ts`, and the meta-service tests that depend on repo CSV inventory remaining structurally stable

## Checklist

- [x] Document accepted named-header aliases and positional layouts.
- [x] Document required vs optional candle fields.
- [x] Document timestamp parsing expectations and failure cases.
- [x] Document response fields returned to the frontend.
- [x] Document `loadSource` values and when each is used.
- [x] Document cache-hit vs in-memory-reuse semantics.
- [x] Add or tighten tests for representative CSV variants and malformed cases.
- [x] Verify artifact/dataset metadata consumers still receive the same core fields.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/services/dataService.test.ts`
- run targeted data-store tests
- run relevant Rust tests for CSV compatibility and loaded dataset state
- manually verify at least one named-header CSV and one positional CSV
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep existing CSV compatibility behavior unless a break is explicitly approved
- preserve current response fields while tightening semantics
- if stricter validation is added later, expose it as clearer errors, not silent rejection
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/04_csv_header_compat_2026-04-08.md`
- `REPO_Improvement/02_data/07_data_cache_sources_2026-04-08.md`
- `REPO_Improvement/02_data/27_dataset_meta_schema_2026-04-08.md`
