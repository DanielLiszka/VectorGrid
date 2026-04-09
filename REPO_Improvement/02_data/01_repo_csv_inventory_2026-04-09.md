# Repo CSV Inventory Plan

Last updated: 2026-04-09

## Purpose

Define the current repository-level CSV inventory contract so repo-data discovery, identity enrichment, and downstream meta-workflow consumers can be improved without breaking existing listing behavior.

## Scope Boundaries

In scope:

- `list_repo_data_csvs` command contract
- inventory shape for `csv_paths`, optional `csv_entries`, and `truncated`
- repo-data listing used by meta/research helpers
- identity enrichment through source size and mtime when requested

Out of scope:

- generic filesystem indexing outside the repo data surface
- options datasets
- new ingestion backends beyond the current repo and Polygon lanes

## Current User-Facing Behavior

The repo already exposes a real CSV inventory command. `src-tauri/src/commands_data.rs` provides `list_repo_data_csvs(max_files, recursive, include_identity)` and returns a `RepoDataCsvList` containing an optional root `dir`, an ordered `csv_paths` list, optional `csv_entries` carrying source size and mtime, plus a `truncated` flag. The listing is already consumed outside the Data panel by `web/src/services/metaService.ts`, which uses repo CSV inventory and Polygon cache inventory together when building research/meta flows.

Current shipped behavior includes:

- bounded file-count clamps rather than unbounded crawling
- optional recursive enumeration
- optional identity enrichment instead of always computing it
- stable sorted output before returning

## Current Code Surface

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`

## Current Verification Surface

- `src-tauri/src/commands_data.rs` helper tests for repo inventory and discovery
- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `command-surface registration in `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs``

## Strengths Already Present

- inventory shape already distinguishes between lightweight path-only listing and heavier identity enrichment
- results are already sorted and explicitly marked truncated when capped
- the command is already useful outside the main data panel, which raises its value as shared infrastructure
- identity enrichment already reuses `data_cache::identity_for_source` instead of inventing a second source-identity path

## Weaknesses / Failure Modes

- the contract is richer than the current written docs, so downstream consumers could drift
- the split between `csv_paths` and `csv_entries` can become inconsistent if not frozen explicitly
- repo inventory behavior is currently more meta-oriented than user-visible in the main Data workspace
- the command’s traversal and cap policy are not yet recorded in one implementation doc

## Regressions To Avoid

- silently dropping `csv_entries` when `include_identity` is true
- changing sorting or truncation semantics without updating consumers
- making inventory enumeration unbounded or much heavier by default
- breaking shared consumers in meta flows while refactoring data commands
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because repo inventory ultimately exists to surface CSV data sources under an explicit file-format assumption.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because `RepoDataCsvList` behaves like a versioned response contract with optional enrichment fields.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because inventory consumers depend on a stable invoke payload and response shape across the frontend/backend boundary.

## Proposed Improvement Approach

1. Document the full `RepoDataCsvList` response contract, including when `csv_entries` is populated and how `truncated` should be interpreted.
2. Freeze sorting, cap, and enrichment semantics so shared consumers can rely on them.
3. Record where repo CSV inventory is currently consumed and keep its scope non-options only.
4. Clarify the difference between repo-data inventory and sample-CSV listing so the two surfaces do not blur together.
5. Tie any future refactors back to the same identity-for-source helper rather than duplicating file-identity logic.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- extracted `build_repo_data_csv_list(...)` in `src-tauri/src/commands_data.rs` so the repo inventory contract can be tested directly without going through the async Tauri command wrapper
- added direct Rust coverage proving `csv_paths`, `csv_entries`, `dir`, and `truncated` stay aligned when `include_identity` is toggled
- preserved the current meta-service contract that effectively relies on object-based `csv_entries` rows when `include_identity=true`
- kept path-only listing intact for lighter callers while freezing the richer identity-enriched lane for meta/research consumers

## Checklist

- [x] Document every field in `RepoDataCsvList` and `RepoDataCsvEntry`.
- [x] Document `include_identity`, `recursive`, and `max_files` semantics.
- [x] Document sorting and truncation behavior.
- [x] List current frontend consumers of repo CSV inventory.
- [x] Add or tighten tests if inventory shape or shared consumer expectations change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted meta-service tests that cover repo CSV inventory consumption
- manually call the command once with and without identity enrichment after any contract change
- verify command registration stays aligned in `commands.rs` and `lib.rs`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve field names unless a compatibility migration is written
- prefer additive enrichment over removing existing lightweight path listings
- keep repo CSV inventory separated from options-specific dataset concepts
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/02_repo_csv_discovery_2026-04-09.md`
- `REPO_Improvement/02_data/27_dataset_meta_schema_2026-04-08.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
