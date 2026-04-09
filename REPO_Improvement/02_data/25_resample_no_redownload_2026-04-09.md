# Resample No Redownload Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the no-redownload derived-timeframe posture so users and future contributors understand when the app reuses loaded base data instead of fetching again.

## Scope Boundaries

In scope:

- base-load reuse for derived timeframes
- 1m-cache-then-resample behavior for Polygon
- data-panel timeframe hints and load-source messaging
- derived-load reuse versus fresh resample status

Out of scope:

- remote provider-side resample APIs
- new cache layers beyond the current loaded-data map
- options workflows

## Current User-Facing Behavior

The app already ships a no-redownload resample posture. `web/src/stores/dataStore.ts` tells the user that higher timeframes are derived “with no extra downloads.” `src-tauri/src/state.rs` resamples from the loaded base dataset in memory. `polygon_download_1m_and_load` and `polygon_top_off_dataset` in `src-tauri/src/commands_data.rs` explicitly fetch/cache `1m` first, then resample to the requested display timeframe in memory. Load-source labels already distinguish `in_memory_base_reuse`, `derived_fresh_resample`, and `derived_in_memory_reuse`.

Current user-facing behavior includes:

- applying a higher timeframe to a loaded base dataset does not trigger another provider download
- Polygon requested timeframes still depend on a cached `1m` base under the hood
- the data panel already explains the current “no extra downloads” posture in its timeframe hint

## Current Code Surface

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/timeframe.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `src-tauri/src/state.rs`
- `crates/app_core/src/timeframe.rs`

## Strengths Already Present

- the app already avoids wasteful repeat downloads for derived timeframe changes
- load-source labels already distinguish reuse from fresh derived computation
- Polygon commands already encode the 1m-first architecture instead of pretending the provider cache stores every requested timeframe
- timeframe hints already communicate the no-redownload posture to users

## Weaknesses / Failure Modes

- the no-redownload rule is important but spread across UI hints, service normalization, command logic, and state reuse
- requested timeframe versus cached base timeframe can still be misunderstood without a dedicated doc
- load-source labels are helpful but not yet frozen as a documented contract
- derived reuse and fresh derived computation are close enough that they can be conflated in future copy changes

## Regressions To Avoid

- re-downloading provider data when a base dataset is already loaded and sufficient
- changing UI copy so users think timeframe changes fetch more provider data than they actually do
- breaking load-source labeling for reuse versus fresh derived computation
- letting provider-side timeframe requests leak into the cached-base contract without review
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Polygon Aggregates / Custom Bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) is relevant because the current architecture deliberately fetches one base cadence and derives larger intervals locally.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because no-redownload behavior depends on one store-owned base dataset source of truth.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because resample correctness still depends on stable chronological base data, not repeated fetches.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document no-redownload behavior as a deliberate product rule, not an implementation accident.
2. Freeze the difference between `in_memory_base_reuse`, `derived_fresh_resample`, and `derived_in_memory_reuse`.
3. Tie Polygon’s 1m-first cache posture directly to the user-facing timeframe hint.
4. Keep service/store copy aligned with backend behavior whenever load-source semantics change.
5. Add targeted checks if any change threatens to re-fetch data unnecessarily.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the no-redownload posture for local CSV and Polygon paths.
- [x] Document the current load-source labels and meanings.
- [x] Document the 1m-first Polygon architecture and requested-timeframe resample behavior.
- [x] Document the current timeframe hint copy in the Data panel.
- [x] Add or tighten tests if load-source semantics or hints change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- run targeted state/timeframe tests for derived reuse behavior
- manually apply multiple timeframes to one loaded dataset after any resample-flow refactor
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current no-redownload posture unless a replacement architecture is intentionally adopted
- treat load-source labels and timeframe hints as user-facing contract
- prefer better docs and tests before changing resample-copy language
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Hardened the no-redownload acceptance evidence in `web/src/stores/dataStore.actions.test.ts`: derived-timeframe application now explicitly proves the “no extra downloads” hint, and switching back to the base timeframe proves the base-hint/reset posture instead of leaving it implicit.
- Added service-level precondition coverage in `web/src/services/dataService.test.ts` so malformed timeframe-change requests fail before any backend invoke, which keeps the no-redownload contract tied to valid base-data reuse instead of permissive request shaping.
- Kept the shipped backend architecture unchanged: `src-tauri/src/commands_data.rs` still follows the 1m-first Polygon load/top-off path and `src-tauri/src/state.rs` remains the authority for in-memory base reuse versus fresh derived resample.
- Acceptance evidence captured during execution: `cargo test -p ta_desktop_demo_app state --lib`, `cargo test -p ta_app_core timeframe`, `npm.cmd --prefix web run test -- src/components/DataWorkspacePanel.test.ts src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/utils/core/datasetMeta.test.ts src/services/historyService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/02_data/24_resample_rules_2026-04-09.md`
- `REPO_Improvement/02_data/13_polygon_request_contract_2026-04-09.md`
- `REPO_Improvement/02_data/20_polygon_dataset_topoff_2026-04-09.md`
