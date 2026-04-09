# Polygon Download And Load Plan

Last updated: 2026-04-09

## Purpose

Define the combined download-plus-load contract for Polygon datasets.

## Scope Boundaries

In scope:

- download request normalization
- chunking, cache use, and manifest updates
- `polygon_download_1m_and_load`
- post-download base-load semantics
- frontend normalization and UX expectations

Out of scope:

- options datasets
- standalone validate/repair/top-off docs outside the immediate download+load flow

## Current User-Facing Behavior

Users can request a Polygon dataset download, optionally reuse cache, and receive both the downloaded CSV path and normalized loaded dataset summaries. The current flow also carries market, symbol, timeframe, adjusted, and integrity metadata through the cache manifest.

## Current Code Surface

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- backend Polygon and state tests
- manual dataset download / cache reuse checks

## Strengths Already Present

- the backend already has explicit request structs, manifest storage, key handling, and cache metadata
- the frontend already normalizes download-and-load responses into `PolygonDownloadAndLoadSummary`
- the implementation already distinguishes cache-only use and base-load results

## Weaknesses / Failure Modes

- request semantics, chunking, and post-download load behavior are not documented in one contract
- timezone/session expectations vary by market
- download success and load success can be conflated if not described clearly

## Regressions To Avoid

- changing limit/chunking behavior without updating UX/status messaging
- losing `usedCacheOnly`, `downloadedBars`, or base-load information
- breaking manifest identity or adjusted/timeframe persistence
- making cached and freshly downloaded paths indistinguishable
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Polygon Custom Bars (Stocks)](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) documents the range endpoint, default adjustment behavior, `limit` defaults/maxima, and empty-interval behavior.
- [Polygon Custom Bars (Crypto)](https://polygon.io/docs/rest/crypto/aggregates/custom-bars/) and [Polygon Custom Bars (Forex)](https://polygon.io/docs/rest/forex/aggregates/custom-bars/) show that market-specific aggregate semantics and sessions differ by market family.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because download+load is exposed as a command contract rather than an internal helper.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Define the request contract and normalized defaults.
2. Define cache-only reuse vs network fetch behavior.
3. Define the boundary between download success and load success.
4. Define the shape of returned metadata and how it maps into the loaded dataset state.
5. Define UX expectations for cache-hit, freshly downloaded, and partially degraded outcomes.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document required request fields and valid market/timeframe combinations.
- [ ] Document default `adjusted` behavior and how it is persisted.
- [ ] Document chunking and request-limit behavior.
- [ ] Document when `usedCacheOnly` may be true.
- [ ] Document the difference between `baseLoad` and final `load`.
- [ ] Document manifest updates and returned dataset metadata.
- [ ] Document failure handling for bad keys, partial coverage, and empty aggregate windows.
- [ ] Add or tighten tests for cache-only and fresh-download paths.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted data-service tests
- run backend Polygon/state tests
- manually verify one cache reuse path and one fresh download path
- confirm metadata survives into loaded dataset context and artifacts
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target

## Rollout / Fallback / Compatibility Notes

- keep existing manifest identity and dataset labels stable
- preserve current response fields even if internal flow changes
- prefer clearer status semantics over new UI surface area
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/11_polygon_root_settings_2026-04-09.md`
- `REPO_Improvement/02_data/15_polygon_manifest_schema_2026-04-09.md`
- `REPO_Improvement/02_data/21_polygon_selection_ux_2026-04-09.md`
