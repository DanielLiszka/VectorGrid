# Data Cache State Backend Plan

Last updated: 2026-04-09

## Purpose

Plan improvements around backend cache ownership and state boundaries so disk parse cache, in-memory dataset state, and Polygon cache behavior remain understandable, testable, and non-contradictory.

## Scope Boundaries

In scope:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/settings.rs`
- frontend normalization of backend load metadata in `web/src/services/dataService.ts`

Out of scope:

- redesigning the Polygon download product
- changing artifact provenance everywhere in one pass
- options-related cache surfaces

## Current User-Facing Behavior

The app already uses multiple caching layers with different purposes. Generic CSV parsing uses `.vta` sidecars or app-local fallback cache files. Loaded datasets are also retained in memory through `AppState`. Polygon datasets additionally live under the configured market-data root with their own manifest and maintenance commands.

From the user’s perspective, these layers surface through load-source text and refresh behavior. A CSV can load from binary cache, parse and then write cache, skip writing cache, or reuse an already loaded in-memory base dataset. Resampled datasets likewise distinguish fresh derived work from derived in-memory reuse. Polygon datasets still reload through the generic CSV loader after download, repair, or top-off, so a single file can participate in both the Polygon cache and the generic parse-cache system.

This means the backend already has a real authority split, but the repo does not yet have a dedicated plan that freezes those authority boundaries for future contributors.

## Current Code Surface

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`

## Current Verification Surface

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- the repo already distinguishes disk parse cache from in-memory dataset reuse
- sidecar-before-fallback parse-cache precedence is already implemented
- cache writes are best-effort rather than load-blocking
- `LoadSource` already provides a meaningful frontend-facing contract for backend load paths
- Polygon storage already has its own manifest and root-management logic separate from `.vta` parse cache

## Weaknesses / Failure Modes

- cache authority is real in code but still under-documented as a contract
- frontend consumers can easily over-infer backend cache behavior from simplified load-source strings
- `content_fingerprint_for_source()` exists but is not part of the active reuse contract, which creates ambiguity if contributors assume it already is
- Polygon datasets participate in both market-data-root management and generic parse-cache loading, which is easy to misunderstand
- corrupt-cache and write-failure edge cases are not fully documented as part of the cache contract

## Regressions To Avoid

- conflating `market_data_root_dir` with generic `.vta` cache ownership
- changing `LoadSource` semantics without synchronized frontend and test updates
- losing sidecar-before-fallback precedence
- turning cache-write failures into load failures
- treating content fingerprints as active reuse keys before the rest of the plumbing exists
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because all backend cache and load metadata eventually cross the Tauri command boundary into the frontend contract.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because the backend cache/state split depends on explicit long-lived backend authority rather than frontend inference.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because `LoadSource`, cache info summaries, and dataset metadata behave like serialized contracts even when not expressed as formal schemas.
- [Rust `Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) is relevant because in-memory dataset retention and reuse rely on shared ownership semantics that should not be confused with durable caching.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the three separate authorities explicitly: disk parse cache, in-memory dataset state, and Polygon cache.
2. Freeze the current precedence rules for sidecar cache, fallback cache, parse, and best-effort writes.
3. Document what `LoadSource` does and does not promise to the frontend.
4. Record that content fingerprinting exists today as a future-strengthening mechanism rather than an active cache-key rule.
5. Add missing test expectations for corrupt cache rejection, fallback-cache hits, and partial cache-write behavior if those gaps remain.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the separate ownership of `.vta` parse cache, `AppState` dataset state, and Polygon cache.
- [ ] Document the current cache precedence and failure-tolerance rules.
- [ ] Document every `LoadSource` value and what frontend consumers may infer from it.
- [ ] Document that content fingerprinting is available but not yet the active reuse contract.
- [ ] Add or tighten backend tests for corrupt-cache, fallback-hit, and write-failure scenarios.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app data_cache`
- run `cargo test -p ta_desktop_demo_app state`
- run `cargo test -p ta_desktop_demo_app commands_data`
- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current `LoadSource` values until all frontend consumers are updated together
- prefer additive metadata over silently redefining existing cache-family meanings
- keep cache writes best-effort and non-fatal unless the product explicitly changes that policy
- treat stronger provenance as a future additive layer, not a hidden reinterpretation of current state
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/30_data_repair_replay_risk_2026-04-09.md`
- `REPO_Improvement/02_data/19_polygon_dataset_repair_2026-04-09.md`
- `REPO_Improvement/02_data/20_polygon_dataset_topoff_2026-04-09.md`
- `REPO_Improvement/02_data/34_dataset_memory_retention_2026-04-09.md`
