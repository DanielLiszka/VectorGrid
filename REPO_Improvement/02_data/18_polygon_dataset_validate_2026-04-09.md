# Polygon Dataset Validate Plan

Last updated: 2026-04-09

## Purpose

Define the current cached-Polygon validation contract so integrity scans, manifest updates, and frontend refresh behavior stay explicit and non-destructive.

## Scope Boundaries

In scope:

- validate command behavior for cached Polygon datasets
- integrity-scan outputs and manifest updates
- frontend validation action and post-validate refresh behavior
- row-count and min/max timestamp refresh semantics

Out of scope:

- repair logic that rewrites CSV content
- non-Polygon CSV validation
- options workflows

## Current User-Facing Behavior

`polygon_validate_dataset` already performs a real, non-destructive integrity pass. Backend validation requires that the CSV file exists and that the dataset is already present in the Polygon manifest. It uses the manifest timeframe to scan the CSV for row count, min/max timestamps, parse errors, duplicates, out-of-order rows, and same-day gaps over the threshold, then writes those refreshed values back into the manifest and returns the updated `PolygonDatasetMeta`.

Current user-facing behavior includes:

- validate is only meaningful for cached Polygon datasets already known to the manifest
- validation refreshes integrity metadata without rewriting the CSV itself
- the Data panel can re-select the validated dataset and show updated metadata after the call returns

## Current Code Surface

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/components/DataWorkspacePanel.test.ts`

## Strengths Already Present

- validation is already non-destructive and separate from repair
- manifest metadata already updates in place with refreshed counts and integrity
- frontend flows already expose validate as a user action on selected cached datasets
- timeframe-specific gap thresholds already come from manifest-backed dataset info

## Weaknesses / Failure Modes

- validate depends on manifest membership, which is important but not obvious to users without a doc
- integrity semantics are backend-rich and only partially surfaced in UI copy
- the contract between validation results and selected-dataset refresh behavior is not yet frozen in writing
- there is no dedicated doc that says validate refreshes metadata rather than content

## Regressions To Avoid

- rewriting CSV content in the validate path
- allowing validate to succeed on datasets not present in the manifest without an explicit contract change
- failing to refresh row count and min/max timestamps after validation
- breaking frontend re-selection and metadata refresh after validate completes

## External Research Summary

- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because validation still depends on stable CSV row parsing rules.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because validation returns refreshed dataset metadata rather than an arbitrary blob.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because chronological integrity is meaningful only when later consumers assume ordered time series behavior.

## Proposed Improvement Approach

1. Document validate as a metadata-refresh operation, not a repair operation.
2. Freeze the prerequisite that the dataset already exists in the manifest and on disk.
3. Document every integrity field refreshed by validation.
4. Tie frontend selection refresh behavior to the returned dataset metadata explicitly.
5. Keep validate separate from repair even if the UI shows both side by side.

## Checklist

- [ ] Document validate preconditions and error cases.
- [ ] Document every integrity metric refreshed by validation.
- [ ] Document row-count and min/max timestamp refresh behavior.
- [ ] Document frontend validate action and post-validate selection behavior.
- [ ] Add or tighten tests if validation semantics or response shape changes.

## Verification Plan

- run targeted Polygon backend validate tests
- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`
- manually validate one cached dataset and inspect updated metadata after any validate-flow change

## Rollout / Fallback / Compatibility Notes

- preserve validate as non-destructive unless a new explicit mode is introduced
- prefer clearer UI copy before changing backend semantics
- keep manifest-backed preconditions explicit

## Related Docs

- `REPO_Improvement/02_data/19_polygon_dataset_repair_2026-04-09.md`
- `REPO_Improvement/02_data/15_polygon_manifest_schema_2026-04-09.md`
- `REPO_Improvement/02_data/16_polygon_dataset_listing_2026-04-09.md`
