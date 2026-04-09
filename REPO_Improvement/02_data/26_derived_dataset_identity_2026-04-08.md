# Derived Dataset Identity Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for derived dataset identity so resampled or otherwise derived timeframe datasets remain explicitly tied to their base source, cache, and artifact metadata instead of masquerading as original raw loads.

## Scope Boundaries

In scope:

- base-versus-derived dataset identity in frontend state
- resample-derived dataset identity in artifacts
- no-redownload expectations for derived timeframes
- derived dataset reuse and invalidation semantics

Out of scope:

- raw CSV parsing
- unrelated ML artifact identity
- options datasets

## Current User-Facing Behavior

The repo already distinguishes base dataset identity from current active derived identity. Current evidence of that distinction includes:

- `baseDataId` versus current `dataId`
- `baseDataMeta` versus current timeframe/context fields
- `baseCsvPath` versus active `csvPath`
- `baseDatasetLabel` / `baseDatasetContext`
- dataset metadata fields:
  - `timeframe`
  - `base_timeframe`
  - `derived_timeframe`
  - `resample_applied`
  - `resample_rule`
  - `resample_anchor`

The data store already tells the user that higher timeframes are derived without extra downloads, and artifact builders already preserve derived/base identity fields.

## Current Code Surface

- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`
- `web/src/utils/core/datasetMeta.ts`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- `src-tauri/src/state.rs` derived-cache reuse tests
- `src-tauri/src/run_artifacts.rs` dataset-meta normalization tests
- data-service and validation-service tests that consume derived-timeframe metadata

## Strengths Already Present

- derived/base identity fields already exist
- no-redownload messaging is already user-visible
- artifact metadata already carries derived/base/resample fields
- backend state already has reuse/invalidation behavior for derived datasets

## Weaknesses / Failure Modes

- derived dataset identity is still spread across frontend hints, backend state, and artifact meta
- it is easy for one surface to collapse back to a single `timeframe` field and hide derivation
- current derived identity rules are not yet described as a stable contract
- future derived transformations could overload the same fields without careful documentation

## Regressions To Avoid

- presenting a derived dataset as if it were the raw/base file
- dropping `base_timeframe` or `derived_timeframe` from artifacts
- reusing a stale derived dataset after the base changes
- losing the “no extra downloads” guarantee for derived timeframes

## External Research Summary

- [PROV-Overview](https://www.w3.org/TR/prov-overview/)
  - relevant because derived datasets are provenance relationships, not just alternate labels
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because derived time-series identity still needs to preserve temporal correctness and lineage
- [Custom Bars (OHLC) | Massive/Polygon](https://massive.com/docs/rest/stocks/aggregates/custom-bars)
  - relevant because higher timeframe bars are naturally derived entities built from a base series
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because derived/base identity fields already behave like a schema contract

## Proposed Improvement Approach

1. Define a derived-dataset identity model explicitly:
   - base identity
   - derived identity
   - transformation metadata
   - invalidation relationship
2. Document which fields belong to the base source and which belong to the currently active derived dataset.
3. Preserve the current no-redownload guarantee as a documented product contract for derived timeframes.
4. Make artifact builders and shell context consumers use the same derived/base identity rules.
5. Add tests around derived reuse, base replacement, and artifact round-trip preservation.

## Checklist

- [ ] Document base-versus-derived dataset identity fields and meanings.
- [ ] Document current no-redownload expectations for derived timeframes.
- [ ] Document reuse and invalidation semantics for derived datasets.
- [ ] Document how derived identity is preserved in artifacts and reports.
- [ ] Add tests for base replacement invalidating prior derived identity.
- [ ] Add tests proving artifacts preserve base and derived timeframe fields distinctly.
- [ ] Add tests that shell context stays aligned with artifact/meta identity for derived datasets.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app state`
- run `cargo test -p ta_desktop_demo_app run_artifacts`
- run targeted data/validation frontend tests that consume derived-timeframe context
- manually verify a base dataset load followed by a derived-timeframe switch and artifact save

## Rollout / Fallback / Compatibility Notes

- preserve the current field inventory and meanings first
- add new derivation metadata rather than mutating current base/derived fields silently
- keep raw/base identity authoritative even when a derived timeframe is active

## Related Docs

- `REPO_Improvement/02_data/23_resample_contract_2026-04-08.md`
- `REPO_Improvement/02_data/10_dataset_context_surface_2026-04-08.md`
- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`

