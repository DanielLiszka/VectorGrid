# Resample Rules Plan

Last updated: 2026-04-09

## Purpose

Define the current timeframe-label and resample contract so derived datasets remain chronological, field-aware, and compatible with current load IDs and cache reuse.

## Scope Boundaries

In scope:

- timeframe-label parsing and canonicalization
- resample preconditions and bucket rules
- derived dataset IDs and version reuse
- field-availability preservation through resample

Out of scope:

- new resample engines
- provider-side server resampling
- options workflows

## Current User-Facing Behavior

The resample contract is already non-trivial. `crates/app_core/src/timeframe.rs` accepts timeframe labels with `s`, `m`, `h`, or `d`, canonicalizes some seconds labels to familiar names, and requires positive intervals. `resample_candles()` enforces ascending timestamps, restarts bucket anchoring at each day boundary, aggregates OHLCV using finite-value presence, and preserves field flags only when a bucket actually yields usable values. `src-tauri/src/state.rs` then assigns derived IDs of the form `${base}@${timeframe}`, reuses existing derived datasets when the parent source version still matches, and returns `derived_in_memory_reuse` or `derived_fresh_resample` accordingly.

Current user-facing behavior includes:

- resample can fail explicitly on unsorted timestamps
- derived IDs are stable and reusable when the base dataset has not changed
- field availability in derived datasets still matters to downstream strategy gating

## Current Code Surface

- `crates/app_core/src/timeframe.rs`
- `src-tauri/src/state.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Current Verification Surface

- `crates/app_core/src/timeframe.rs`
- `src-tauri/src/state.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- timeframe parsing and resample logic are already centralized in dedicated backend code
- derived dataset reuse already depends on base-source version tracking rather than only string IDs
- resample already preserves chronology by requiring ascending timestamps and resetting buckets by day
- derived datasets still carry proper metadata for downstream field-based gating

## Weaknesses / Failure Modes

- session/day anchoring and field-preservation behavior are subtle and not yet frozen in one doc
- the difference between timeframe parsing, resample rules, and reuse rules spans multiple files
- seconds-label canonicalization is useful but easy to forget when changing supported formats
- there is no dedicated implementation doc tying field rules and resample rules together

## Regressions To Avoid

- allowing unsorted timestamps to resample silently
- changing bucket anchoring or aggregation rules without explicit review
- breaking derived-ID stability or source-version reuse checks
- letting derived field flags disagree with actual aggregated data availability

## External Research Summary

- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because downstream consumers assume time-ordered derived datasets.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because resample responses and derived IDs form a stable contract across layers.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because resample correctness still depends on the integrity of the loaded CSV source arrays.

## Proposed Improvement Approach

1. Document timeframe parsing, resample aggregation, and reuse behavior as one derived-dataset contract.
2. Freeze day-boundary anchoring and current OHLCV aggregation rules.
3. Document derived-ID and source-version reuse semantics explicitly.
4. Tie field-preservation behavior back to the CSV field rules doc.
5. Add or tighten tests before changing timeframe parsing or aggregation behavior.

## Checklist

- [ ] Document supported timeframe label formats and canonicalization.
- [ ] Document sorted-timestamp preconditions.
- [ ] Document bucket anchoring and OHLCV aggregation rules.
- [ ] Document derived-ID and reuse behavior.
- [ ] Document field-availability preservation across resample.

## Verification Plan

- run targeted `timeframe.rs` tests
- run targeted `state.rs` tests for derived reuse behavior
- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve explicit failure on broken chronology
- prefer stronger docs and tests before broadening timeframe support
- keep resample backend-owned even if frontend hints get clearer

## Related Docs

- `REPO_Improvement/02_data/25_resample_no_redownload_2026-04-09.md`
- `REPO_Improvement/02_data/05_csv_field_rules_2026-04-09.md`
- `REPO_Improvement/02_data/06_csv_timestamp_rules_2026-04-09.md`
