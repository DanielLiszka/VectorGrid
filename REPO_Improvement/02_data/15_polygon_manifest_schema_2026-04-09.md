# Polygon Manifest Schema Plan

Last updated: 2026-04-09

## Purpose

Define the current Polygon manifest schema so dataset identity, key naming, integrity fields, and back-compat rules stay stable across downloads, validation, and repair.

## Scope Boundaries

In scope:

- manifest file location and shape
- dataset-key naming rules
- per-dataset metadata and integrity fields
- backward-compatibility upgrades for older manifest entries

Out of scope:

- generic artifact manifests outside Polygon cache
- options manifests
- new manifest storage engines

## Current User-Facing Behavior

`src-tauri/src/polygon.rs` stores manifest state in `polygon_manifest.json` under `polygon_cache/`. The file is a `PolygonCacheManifest` containing a `BTreeMap<String, PolygonDatasetMeta>`. Dataset keys encode market, ticker, and timeframe, with one back-compat wrinkle: unadjusted stocks use `stocks_raw:*`, while adjusted stocks use `stocks:*`, and legacy entries without `adjusted=true` get upgraded by loader-side back-compat logic. `PolygonDatasetMeta` already carries market, display symbol, Polygon ticker, timeframe, CSV path, min/max timestamps, last-updated time, row count, adjusted posture, and optional integrity.

Current behavior also includes:

- manifest-backed dataset listing for the frontend
- validation and repair updating manifest metadata in place
- downloads preserving prior integrity metadata when reusing cached entries rather than always recomputing it immediately

## Current Code Surface

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`

## Current Verification Surface

- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- manifest ownership is already centralized in one backend module
- dataset metadata is already rich enough to drive listing, validation, repair, and selection UX
- key naming already encodes market/ticker/timeframe identity in a stable way
- back-compat logic already exists for older stock manifest entries

## Weaknesses / Failure Modes

- manifest semantics are only lightly documented today even though several UI surfaces depend on them
- key naming and adjusted-stock back-compat are easy to break during cleanup
- integrity can intentionally remain stale or null depending on the path taken, which is subtle without a dedicated doc
- there are no explicit schema round-trip tests that freeze the key and field contract

## Regressions To Avoid

- renaming manifest keys casually
- breaking adjusted-stock back-compat behavior
- assuming every dataset has fresh integrity immediately after download
- moving the manifest path or structure without updating every consumer

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the manifest is a persisted contract with version-like expectations even if it is not yet formally schema-driven.
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259) is relevant because manifest semantics depend on stable JSON field names and null-versus-omitted behavior.
- [Polygon Aggregates / Custom Bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) is relevant because dataset metadata in the manifest reflects the aggregate-request model currently in use.

## Proposed Improvement Approach

1. Document the manifest file location, top-level shape, and dataset-key rules explicitly.
2. Freeze the meaning of every `PolygonDatasetMeta` field and when `integrity` may be null or stale.
3. Document adjusted-stock key naming and legacy-entry upgrade behavior as first-class compatibility rules.
4. Tie listing, validation, repair, and top-off behavior back to the same manifest contract.
5. Add explicit schema or round-trip tests if key naming or field semantics are touched.

## Checklist

- [ ] Document manifest location and top-level map shape.
- [ ] Document dataset-key naming rules, including `stocks_raw` behavior.
- [ ] Document every `PolygonDatasetMeta` field and its meaning.
- [ ] Document when `integrity` is null, reused, refreshed, or overwritten.
- [ ] Add or tighten backend tests if manifest key or field behavior changes.

## Verification Plan

- run targeted Polygon backend tests for download, validate, and repair paths
- add a manifest round-trip or key-shape test before changing manifest behavior
- run frontend data-service/store tests if manifest-backed response shape changes

## Rollout / Fallback / Compatibility Notes

- preserve key names and path location unless a compatibility migration is implemented
- prefer richer docs and tests before attempting manifest simplification
- keep manifest behavior backend-owned even when UI copy improves

## Related Docs

- `REPO_Improvement/02_data/16_polygon_dataset_listing_2026-04-09.md`
- `REPO_Improvement/02_data/18_polygon_dataset_validate_2026-04-09.md`
- `REPO_Improvement/02_data/19_polygon_dataset_repair_2026-04-09.md`
