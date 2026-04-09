# CSV Timestamp Rules Plan

Last updated: 2026-04-09

## Purpose

Define the current timestamp-ordering and time-range rules so CSV ingestion, integrity checks, and timeframe derivation preserve chronological correctness without hidden repair assumptions.

## Scope Boundaries

In scope:

- timestamp parsing and chronological assumptions during CSV load
- resample preconditions and chronological error behavior
- Polygon integrity metrics for duplicates, out-of-order rows, and same-day gaps
- time-range and timeframe hints exposed to the UI

Out of scope:

- timezone normalization beyond the existing loader semantics
- options datasets
- new timestamp formats not already handled by the compatibility loader

## Current User-Facing Behavior

Chronology is already part of the shipped data contract. `crates/app_core/src/csv_compat.rs` parses timestamps and builds candle arrays that later feed `src-tauri/src/state.rs`. `crates/app_core/src/timeframe.rs` rejects unsorted timestamps during resampling and anchors derived candles from the loaded base series. `src-tauri/src/polygon.rs` separately scans cached datasets for duplicates, out-of-order rows, and same-day gaps greater than two minutes, then persists those integrity signals back into the Polygon manifest. The frontend surfaces min/max timestamps, timeframe labels, dataset range text, and integrity summaries.

That means timestamp quality already affects real user-facing behavior:

- unsorted data can block resampling
- Polygon cached datasets can be validated and repaired based on integrity findings
- dataset range and timeframe hints come directly from current timestamp metadata

## Current Code Surface

- `crates/app_core/src/csv_compat.rs`
- `crates/app_core/src/timeframe.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `crates/app_core/src/timeframe.rs`
- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- chronological correctness is already enforced at critical points instead of treated as a best-effort hint
- resample code already rejects broken ordering explicitly
- Polygon integrity metrics already distinguish duplicates, out-of-order rows, and gap conditions
- the UI already exposes time-range summaries derived from actual metadata

## Weaknesses / Failure Modes

- timestamp rules are spread across loader, resample, and Polygon integrity paths
- there is no dedicated doc that states which timestamp problems are tolerated, warned, or fatal
- CSV chronology and Polygon chronology are related but currently documented separately only in code
- the relationship between dataset repair and startup restore safety is not yet captured in one place

## Regressions To Avoid

- allowing unsorted data to resample silently
- changing integrity-scan meanings for duplicates/out-of-order rows without updating consumers
- making range labels or timeframe hints disagree with actual loaded timestamps
- repairing timestamp problems implicitly during load without a visible contract change

## External Research Summary

- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because chronological contracts begin with consistent row parsing and field interpretation.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because later validation/holdout logic assumes chronological ordering rather than iid shuffling.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because loaded meta and integrity summaries behave like typed response contracts whose timestamp fields must stay stable.

## Proposed Improvement Approach

1. Document the loader, resample, and Polygon integrity timestamp rules together as one chronology contract.
2. State which timestamp failures are fatal during resample and which are surfaced as repairable integrity findings.
3. Freeze range-text and timeframe derivation semantics in the data store.
4. Tie repair workflows and startup restore safety back to the same chronology assumptions.
5. Keep chronological correctness explicit rather than hiding it behind silent normalization.

## Checklist

- [ ] Document timestamp ordering assumptions during CSV load and resample.
- [ ] Document current duplicate, out-of-order, and same-day-gap integrity meanings.
- [ ] Document how timeframe and range text are computed for the UI.
- [ ] Document which chronology failures block resample versus remain repairable.
- [ ] Add or tighten tests if timestamp semantics or integrity wording changes.

## Verification Plan

- run targeted `timeframe.rs` tests
- run targeted Polygon integrity/repair tests
- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve explicit chronology failures instead of silently coercing bad data
- prefer documenting timestamp semantics before broadening accepted formats
- keep frontend range/timeframe summaries aligned with backend timestamp truths

## Related Docs

- `REPO_Improvement/02_data/05_csv_field_rules_2026-04-09.md`
- `REPO_Improvement/02_data/24_resample_rules_DATE.md`
- `REPO_Improvement/02_data/18_polygon_dataset_validate_DATE.md`
