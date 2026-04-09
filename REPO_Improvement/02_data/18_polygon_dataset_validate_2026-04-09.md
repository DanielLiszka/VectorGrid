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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

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
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document validate preconditions and error cases.
- [ ] Document every integrity metric refreshed by validation.
- [ ] Document row-count and min/max timestamp refresh behavior.
- [ ] Document frontend validate action and post-validate selection behavior.
- [ ] Add or tighten tests if validation semantics or response shape changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted Polygon backend validate tests
- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- manually validate one cached dataset and inspect updated metadata after any validate-flow change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve validate as non-destructive unless a new explicit mode is introduced
- prefer clearer UI copy before changing backend semantics
- keep manifest-backed preconditions explicit
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/19_polygon_dataset_repair_2026-04-09.md`
- `REPO_Improvement/02_data/15_polygon_manifest_schema_2026-04-09.md`
- `REPO_Improvement/02_data/16_polygon_dataset_listing_2026-04-09.md`
