# Data Cache Sources Plan

Last updated: 2026-04-08

## Purpose

Plan improvements to the non-options data-cache/source contract so the app accurately explains where data came from, which cache family was used, and what settings or identities actually control reuse.

## Scope Boundaries

In scope:

- CSV binary-cache sources
- in-memory base and derived reuse
- Polygon cache ownership and settings boundaries
- frontend `loadSource` truthfulness and user-facing copy

Out of scope:

- options data caches
- deep dataset-provenance redesign beyond source/load semantics
- benchmarking cache performance in isolation

## Current User-Facing Behavior

The app can load data from multiple cache or reuse paths, including sidecar `.vta` files, app-local fallback cache files, in-memory base reuse, in-memory derived reuse, fresh CSV parse plus cache write, and Polygon-managed cache paths. The frontend presents a simplified `loadSource` label, but some of that meaning is inferred rather than passed through directly.

## Current Code Surface

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/settings.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/SettingsWorkspacePanel.vue`

## Current Verification Surface

- `src-tauri/src/data_cache.rs` tests
- `src-tauri/src/state.rs` tests
- `src-tauri/src/polygon.rs` tests
- `src-tauri/src/commands_data.rs` tests
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- multiple reuse paths already exist, which reduces unnecessary reparsing and resampling
- backend already has an explicit `LoadSource` enum
- cache writes are best-effort and do not hard-block data loading
- Polygon workflows already preserve separate manifest/cache responsibilities

## Weaknesses / Failure Modes

- “cache” currently means several different families with different owners and settings
- frontend load-source copy is partly backend truth and partly fallback inference
- source identity and base-dedup reuse rely mainly on size plus modified time
- there is a content-fingerprint helper, but it is not clearly part of the active contract
- settings copy points users to Polygon cache behavior more clearly than to CSV binary-cache behavior

## Regressions To Avoid

- breaking reuse of sidecar or app-local fallback `.vta` caches
- changing `LoadSource` semantics without frontend/test updates
- implying that `market_data_root_dir` controls CSV binary caches when it does not
- removing safe best-effort cache-write behavior
- making data-source labels less truthful or more generic

## External Research Summary

- [Tauri JavaScript path API](https://v2.tauri.app/reference/javascript/api/namespacepath/) is relevant for clarifying user-facing path ownership and “open folder” behaviors.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) supports keeping backend source truth authoritative instead of reconstructing it in the UI.
- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html) is relevant because current source identity and reuse depend on filesystem metadata.
- [Rust `std::fs::rename`](https://doc.rust-lang.org/std/fs/fn.rename.html) and [Rust `PathBuf::with_extension`](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.with_extension) are relevant to cache-file placement and sidecar/fallback naming behavior.

## Proposed Improvement Approach

1. Define each cache family explicitly:
   - CSV sidecar binary cache
   - app-local fallback binary cache
   - in-memory base reuse
   - in-memory derived reuse
   - Polygon dataset cache/manifest
2. Make backend `LoadSource` authoritative end to end and reduce frontend inference wherever the backend can provide a direct answer.
3. Document which settings affect which cache families, especially the fact that `market_data_root_dir` governs Polygon storage and not generic CSV binary caches.
4. Clarify which identity signals are contractual today and which are merely helper utilities.
5. Expand tests and user-facing copy so all six current load-source outcomes remain visible and understandable.

## Checklist

- [ ] Document every current cache family and its owner.
- [ ] List all current `LoadSource` values and their meanings.
- [ ] Distinguish backend truth from frontend inference in the source-display pipeline.
- [ ] Document the exact scope of `market_data_root_dir`.
- [ ] Document sidecar vs app-local fallback precedence.
- [ ] Clarify the current source-identity contract and any non-contract helpers.
- [ ] Add tests that exercise all current load-source variants.
- [ ] Improve settings/help copy so users know where different cache families live.

## Verification Plan

- run targeted `src-tauri` data-cache, state, Polygon, and commands-data tests
- run `npm test -- src/services/dataService.test.ts`
- run targeted data-store tests
- manually verify source labels for sidecar cache, fallback cache, fresh parse, in-memory reuse, derived reuse, and Polygon flows if semantics change

## Rollout / Fallback / Compatibility Notes

- preserve current cache reuse behavior while improving truthfulness and diagnostics
- if additional source fields are surfaced from Rust, keep the current frontend copy working until the new contract is fully consumed
- keep cache-write failures non-fatal unless the repo explicitly decides otherwise

## Related Docs

- `REPO_Improvement/02_data/03_csv_load_contract_2026-04-08.md`
- `REPO_Improvement/02_data/17_polygon_download_load_2026-04-08.md`
- `REPO_Improvement/02_data/22_dataset_provenance_DATE.md`
- `REPO_Improvement/02_data/23_resample_contract_DATE.md`
- `REPO_Improvement/02_data/33_data_cache_state_backend_DATE.md`
