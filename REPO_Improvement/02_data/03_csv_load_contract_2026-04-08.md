# CSV Load Contract Plan

Last updated: 2026-04-08

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

## Checklist

- [ ] Document accepted named-header aliases and positional layouts.
- [ ] Document required vs optional candle fields.
- [ ] Document timestamp parsing expectations and failure cases.
- [ ] Document response fields returned to the frontend.
- [ ] Document `loadSource` values and when each is used.
- [ ] Document cache-hit vs in-memory-reuse semantics.
- [ ] Add or tighten tests for representative CSV variants and malformed cases.
- [ ] Verify artifact/dataset metadata consumers still receive the same core fields.

## Verification Plan

- run `npm test -- src/services/dataService.test.ts`
- run targeted data-store tests
- run relevant Rust tests for CSV compatibility and loaded dataset state
- manually verify at least one named-header CSV and one positional CSV

## Rollout / Fallback / Compatibility Notes

- keep existing CSV compatibility behavior unless a break is explicitly approved
- preserve current response fields while tightening semantics
- if stricter validation is added later, expose it as clearer errors, not silent rejection

## Related Docs

- `REPO_Improvement/02_data/04_csv_header_compat_DATE.md`
- `REPO_Improvement/02_data/07_data_cache_sources_DATE.md`
- `REPO_Improvement/02_data/27_dataset_meta_schema_DATE.md`
