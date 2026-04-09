# Polygon Dataset Repair Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the cached-Polygon repair path so de-duplication, gap repair, row limits, and manifest refresh behavior remain safe and well-documented.

## Scope Boundaries

In scope:

- repair command behavior for cached Polygon datasets
- sort/dedup and best-effort gap-refetch behavior
- repair size limits and failure posture
- manifest refresh after repair

Out of scope:

- generic CSV repair outside the Polygon cache
- full re-download policy beyond the current repair path
- options workflows

## Current User-Facing Behavior

`polygon_repair_dataset` already performs a richer, potentially content-mutating flow than validate. Backend repair requires an existing manifest entry and file, parses the CSV into bars, enforces a hard `MAX_REPAIR_ROWS = 2_000_000` guard, sorts by timestamp, de-duplicates by timestamp, and then may attempt best-effort refetch of gappy days if an API key is available and the gap-day count stays within the bounded repair window. It then overwrites the CSV, refreshes row count and time range, stamps `last_updated_utc_ms`, and writes integrity back with duplicates and out-of-order counts reset to zero.

Current user-facing behavior includes:

- repair is stronger and riskier than validate, because it can rewrite the cached CSV
- repair refuses oversized datasets instead of consuming unbounded memory
- repair can improve same-day gaps but only on a bounded best-effort basis

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

## Strengths Already Present

- repair is already separated cleanly from validation
- hard row limits already prevent unsafe in-memory repair on very large datasets
- dedup/sort semantics already provide a deterministic normalization path
- manifest metadata refresh is already part of repair completion

## Weaknesses / Failure Modes

- repair is one of the highest-risk mutable data paths and is currently documented only in code
- best-effort gap refetch depends on API-key availability and bounded day counts, which is subtle
- repair resets some integrity counts after rewrite, which is correct but easy to misread without documentation
- the user-facing difference between validate and repair is still easy to blur in the UI

## Regressions To Avoid

- removing the repair row-limit guard
- rewriting the file without refreshing manifest metadata and integrity
- silently broadening gap-refetch behavior into an unbounded network repair path
- making repair look non-mutating in docs or UI when it actually rewrites the CSV
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Polygon Aggregates / Custom Bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) is relevant because repair may refetch missing same-day slices through the existing aggregate API path.
- [MDN Retry-After](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Retry-After) is relevant because repair’s network fallback should stay compatible with the downloader’s rate-limit posture if the implementation evolves.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because repair still depends on stable CSV row parsing even when it later rewrites the file.

## Proposed Improvement Approach

1. Document repair as the mutable corrective path and validate as the non-mutating diagnostic path.
2. Freeze the current sort, dedup, gap-refetch, and row-limit behavior.
3. Document what integrity fields mean before and after repair.
4. Clarify when repair should tell the user to delete and re-download instead.
5. Keep repair bounded and explicit rather than silently widening its responsibilities.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document repair preconditions and hard limits.
- [x] Document sort and de-dup behavior explicitly.
- [x] Document best-effort gap-refetch conditions and bounds.
- [x] Document post-repair manifest updates and integrity semantics.
- [x] Add or tighten tests if repair behavior or limits change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted Polygon backend repair tests
- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- perform one manual validate-before/repair/validate-after pass on a controlled cached dataset if repair logic changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep repair opt-in and visibly distinct from validate
- preserve hard limits and bounded network behavior unless new safeguards are added first
- prefer clearer UI wording before extending repair behavior
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added backend precondition coverage in `src-tauri/src/polygon.rs` so repair now explicitly proves the manifest-membership requirement instead of only exercising the successful duplicate/order-fix path.
- Added service/store regression coverage in `web/src/services/dataService.test.ts` and `web/src/stores/dataStore.actions.test.ts` for repair-path guardrails: empty-path rejection remains immediate, the selected cached dataset stays pinned after repair, and refreshed metadata continues to flow back into cache summary and selected-dataset text.
- Preserved the current bounded repair semantics. This slice did not widen row-limit or gap-refetch behavior; it tightened test coverage around the existing mutable path so future contributors cannot accidentally soften those safeguards.
- Acceptance evidence captured during execution: `cargo test -p ta_desktop_demo_app polygon --lib`, `npm.cmd --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/02_data/18_polygon_dataset_validate_2026-04-09.md`
- `REPO_Improvement/02_data/20_polygon_dataset_topoff_2026-04-09.md`
- `REPO_Improvement/02_data/30_data_repair_replay_risk_2026-04-09.md`
