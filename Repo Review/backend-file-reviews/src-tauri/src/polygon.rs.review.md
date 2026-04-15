# Review Packet: `src-tauri/src/polygon.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/polygon.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally valuable Polygon cache/download module, but not yet accept-as-is because cache-only reads still require an API key and manifest persistence is not crash-safe`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Network-sensitive`, `Cache-integrity-sensitive`, `Filesystem-persistence-sensitive`, `Options-underlier-sensitive`

## What This File Owns

`polygon.rs` owns the backend support layer for Polygon/Massive-backed historical aggregate caching, repair, and credential lookup.

This file owns:

- cache directory and manifest plumbing:
  - `polygon_cache_dir`: `src-tauri/src/polygon.rs:172-174`
  - manifest path/load/save: `180-420`
- API-key resolution, storage, and migration:
  - keyring service naming: `238-240`
  - keyring migration: `318-334`
  - set/clear/read key: `336-392`
- dataset cache planning:
  - `DownloadCachePlan`: `122-126`
  - cache-plan calculation: `431-458`
- HTTP aggregate fetch and retry logic:
  - `fetch_chunk`: `553-641`
- dataset validation and repair:
  - `validate_cached_dataset`: `840-900`
  - `repair_cached_dataset`: `902-1027`
- manifest lookup by CSV path and top-off request shaping:
  - `find_manifest_dataset_by_csv_path`: `1029-1048`
  - `build_top_off_request_for_dataset`: `1050-1090`
- the main cache population entrypoint:
  - `download_1m_to_cache`: `1099-1277`
- direct file-local tests for cache, manifest, key fallback, and repair behavior:
  - `1279-1699`

This file does not own:

- the Tauri command surface
  - that lives in `src-tauri/src/commands_data.rs`
- Polygon live WebSocket streaming
  - that lives in `src-tauri/src/commands_data.rs:568-1116`
- options option-chain download logic
  - that lives in `src-tauri/src/commands_options_data.rs`
- canonical market-data root resolution
  - that lives in `src-tauri/src/settings.rs` and command callers

## Classification

- Public surface: indirect only
  - internal module, but its behavior is surfaced through Tauri data-download and options-import commands
- Hot path: no
  - it is network/cache support code, not trading-loop compute
- Runtime-critical: high
  - failures block historical data acquisition, top-off, validation, and options underlier bootstrapping
- CUDA/runtime-critical: no
- Strategy-local: no
  - cross-cutting data-ingestion support
- Low-risk support code: no
  - relatively straightforward logic, but operationally sensitive

## Direct Interactions

### Backend interaction map

- Tauri cache info / key management commands call this module directly:
  - `src-tauri/src/commands_data.rs:545-565`
- dataset validate/repair commands route through this module using `spawn_blocking`:
  - `src-tauri/src/commands_data.rs:1123-1145`
- historical download/top-off commands route through this module using `spawn_blocking`:
  - `src-tauri/src/commands_data.rs:1183-1228`
  - `src-tauri/src/commands_data.rs:1236-1278`
- options dataset import depends on this module for the canonical underlier 1m cache:
  - `src-tauri/src/commands_options_data.rs:831-850`

### Caller contract anchors

- the main desktop data command intentionally forces 1m as canonical storage and derives larger timeframes later:
  - `src-tauri/src/commands_data.rs:1196-1207`
- the options importer also hardcodes a 1m underlying fetch:
  - `src-tauri/src/commands_options_data.rs:833-843`

## Standards And Research Log

This packet was written after targeted external review focused on Polygon aggregate semantics, blocking HTTP client guidance, secure-store behavior, and filesystem replacement semantics.

### Sources consulted

1. Massive/Polygon custom bars endpoint documentation
   - Source: <https://massive.com/docs/rest/stocks/aggregates/custom-bars>
   - Why it was consulted:
     - this file builds aggregate-bar requests and assumes date-window behavior and returned-bar semantics
   - Standard informed:
     - request shaping, aggregate-window expectations, and support-code correctness around range fetching

2. Massive knowledge-base article on aggregate-bar construction
   - Source: <https://massive.com/knowledge-base/article/how-does-massive-create-the-open-high-low-close-volume-aggregate-bars>
   - Why it was consulted:
     - this file validates/repairs missing or gappy bar sequences
   - Standard informed:
     - absence of bars and aggregate construction are data-provider semantics, not always local corruption

3. `reqwest::blocking` module documentation
   - Source: <https://docs.rs/reqwest/latest/reqwest/blocking/>
   - Why it was consulted:
     - this module uses blocking HTTP and is called from Tauri `spawn_blocking` wrappers
   - Standard informed:
     - blocking client calls should not run directly inside async runtimes
     - client reuse is preferred when making multiple requests

4. `std::fs::rename` documentation
   - Source: <https://doc.rust-lang.org/std/fs/fn.rename.html>
   - Why it was consulted:
     - manifest persistence currently uses temp-file plus delete-plus-rename replacement
   - Standard informed:
     - rename semantics and replacement behavior should be handled carefully when crash safety matters

5. `keyring` crate documentation
   - Source: <https://docs.rs/keyring/latest/keyring/>
   - Why it was consulted:
     - this module uses cross-platform secure credential storage with file fallback
   - Standard informed:
     - secure-store behavior is platform-dependent and can fail operationally even when the API surface is cross-platform

## Findings

## Correctness And Safety

### 1. Cache-only hits still require an API key even though no network request is needed

- Evidence:
  - API key is required before cache planning:
    - `src-tauri/src/polygon.rs:1117-1118`
  - cache plan is computed only afterward:
    - `1150-1169`
  - full cache hit then exits without HTTP:
    - `1171-1173`
- Why it matters:
  - users with a fully covered cached dataset cannot use it offline unless an API key is still configured
  - this is stricter than the actual runtime behavior requires
- Cross-file impact:
  - desktop download/load command:
    - `src-tauri/src/commands_data.rs:1183-1228`
  - options underlier bootstrap:
    - `src-tauri/src/commands_options_data.rs:831-850`
- Smallest justified fix:
  - move API-key lookup into the branch that actually performs network access
  - keep key validation for `AppendOnly` and `FullRefresh`

### 2. Manifest replacement is not crash-safe because the old manifest is deleted before rename

- Evidence:
  - `save_manifest` writes temp JSON: `410-415`
  - then explicitly deletes the existing manifest: `416-418`
  - only afterward renames temp into place: `419`
- Why it matters:
  - a crash or rename failure between delete and rename can leave the cache with no manifest at all
  - this module is the persistence authority for cached Polygon datasets, so partial replacement is a real durability risk
- Smallest justified fix:
  - switch to a replacement strategy that does not remove the destination first
  - centralize that helper because manifest durability is a module-wide invariant

### 3. Repair integrity metadata mixes pre-repair damage with post-repair file state

- Evidence:
  - parse failures are counted while reading the original damaged CSV: `941`, `953-962`
  - duplicates and ordering are then normalized in-memory: `969-970`
  - manifest integrity after repair stores:
    - original `parse_errors`: `1016-1019`
    - but zeroed `duplicates` and `out_of_order`: `1019-1021`
- Why it matters:
  - the resulting integrity blob is semantically mixed:
    - some fields describe the original broken input
    - others describe the rewritten output
  - that makes downstream interpretation ambiguous
- Test evidence:
  - current test explicitly expects this mixed state:
    - `1366-1372`
- Smallest justified fix:
  - either recompute integrity from the rewritten CSV
  - or rename/document the fields so it is clear which values describe source damage versus repaired output

## API And Naming

### 1. `download_1m_to_cache` is broader than its name suggests

- Evidence:
  - the function accepts arbitrary timeframes through `parse_timeframe(req.timeframe.as_deref())`: `1123-1128`
  - but the main desktop command intentionally forces canonical 1m downloads:
    - `src-tauri/src/commands_data.rs:1196-1207`
  - and the options importer also calls it with `timeframe: Some("1m")`:
    - `src-tauri/src/commands_options_data.rs:833-843`
- Why it matters:
  - the lower-level function is a general aggregate-cache downloader, while the primary callers intentionally treat 1m as the canonical cache lane
  - the current name can mislead maintainers about what is actually generic versus policy-enforced by callers
- Smallest justified fix:
  - either rename/document the function as a general cache downloader
  - or explicitly document that caller policy, not the function itself, freezes the canonical 1m lane

## Error Handling

### 1. Secure-storage fallback is resilient but operationally opaque

- Evidence:
  - keyring migration exits early on entry construction or set failure with no surfaced diagnostic:
    - `318-334`
  - `set_api_key` falls back to local file on keyring set failure:
    - `345-348`
  - `read_api_key` silently ignores keyring read failures and falls back to disk/env:
    - `368-391`
  - `clear_api_key` ignores keyring delete failure:
    - `356-364`
- Why it matters:
  - current behavior is user-friendly in that it preserves fallback behavior
  - it also hides whether secure storage is unavailable, misconfigured, or intermittently failing
- Smallest justified fix:
  - keep the current fallback policy
  - preserve a log-visible diagnostic string or status flag when keyring operations fail

### 2. Retry/backoff classification is reasonable, but the important gap is verification, not local logic

- Evidence:
  - transport failures back off exponentially: `584-600`
  - `429` and `5xx` are retried: `617-631`
  - `Retry-After` is honored when numeric: `617-629`
- Why it matters:
  - this part of the file is stronger than average
  - the remaining risk is that classification changes could regress quietly because there is no focused retry test seam
- Review judgment:
  - not a bug, but a test gap worth tracking

## Module Boundaries And Technical Debt

### 1. The module's strongest save/load guarantees are inconsistent across sub-surfaces

- Evidence:
  - manifest writes use temp-file then delete-plus-rename: `410-419`
  - CSV writes and overwrite logic use their own separate flow later in the file
  - key storage has its own best-effort migration path: `318-392`
- Why it matters:
  - this is the repo's Polygon cache authority, so persistence guarantees should be explicit and consistent
- Smallest justified fix:
  - codify a clearer persistence helper policy for:
    - manifest replacement
    - key fallback cleanup
    - repair/write durability expectations

## Docs And Examples

### 1. The module needs a short note explaining when gaps imply corruption versus normal provider behavior

- Missing documentation topics:
  - same-day gaps may reflect provider behavior or market inactivity, not necessarily local corruption
  - repair is best-effort and intentionally bounded by `MAX_REPAIR_ROWS` and `MAX_GAP_REPAIR_DAYS`

### 2. The API-key and cache-plan relationship should be documented locally

- Missing documentation topics:
  - full cache hits should be usable without network
  - append-only and full-refresh paths require credentials

## Tests And Missing Verification

### 1. Existing local coverage is useful and currently green

- Verified in this review pass:
  - `cargo test -p ta_desktop_demo_app polygon_validate_and_repair_detects_and_fixes_duplicates_and_ordering -- --nocapture`
    - passed `1`
  - `cargo test -p ta_desktop_demo_app read_api_key_falls_back_to_env_vars_when_cache_is_empty -- --nocapture`
    - passed `1`
  - `cargo test -p ta_desktop_demo_app plan_download_cache_strategy_distinguishes_hit_append_and_refresh -- --nocapture`
    - passed `1`
  - `cargo test -p ta_desktop_demo_app load_manifest_upgrades_legacy_adjusted_stock_entries -- --nocapture`
    - passed `1`
- Existing strengths:
  - repair/validation semantics: `1292-1386`
  - top-off request shaping: `1388-1437`
  - API-key env and dotenv fallback: `1439-1475`
  - cache-plan distinction: `1517-1554`
  - legacy manifest upgrade: `1556-1579`
  - manifest CSV-path lookup and membership enforcement: `1656-1699`

### 2. Important operational gaps remain

- Missing tests found in this review pass:
  - full cache hit should succeed without an API key
  - append-only refresh should append only newer bars
  - retry classification for transport errors vs `429` vs `5xx` vs hard `4xx`
  - keyring failure paths should preserve fallback behavior while surfacing diagnostics
  - manifest save failure/crash-window behavior is not pinned

### 3. If this file changes, verification should include both direct and caller-level checks

- Exact local verification required if changed:
  - `cargo test -p ta_desktop_demo_app polygon_validate_and_repair_detects_and_fixes_duplicates_and_ordering -- --nocapture`
  - `cargo test -p ta_desktop_demo_app read_api_key_falls_back_to_env_vars_when_cache_is_empty -- --nocapture`
  - `cargo test -p ta_desktop_demo_app plan_download_cache_strategy_distinguishes_hit_append_and_refresh -- --nocapture`
  - `cargo test -p ta_desktop_demo_app load_manifest_upgrades_legacy_adjusted_stock_entries -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_options_data -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. Performance work here should focus on request behavior and I/O patterns, not micro-allocations

- Evidence:
  - dominant cost is network and file I/O
  - local CPU work is modest parsing, deduplication, and manifest bookkeeping
- Review judgment:
  - do not spend effort on minor clone/allocation cleanup unless profiling proves it matters

### 2. The meaningful performance-sensitive area is request pacing and append-only download behavior

- Evidence:
  - retry/backoff logic: `584-631`
  - append-only planning and fetch behavior: `1150-1185`
- Measure-first guidance:
  - any change to retry cadence, request limits, or append-only fetch windows should be validated against provider behavior and real-world download timings

## Code-Quality Improvement Register

- Safe now:
  - allow full cache hits without requiring a key
  - harden manifest replacement semantics
  - clarify repair-integrity meaning
  - surface keyring fallback diagnostics non-fatally
  - document canonical 1m policy versus generic helper scope
- Defer unless operational evidence demands it:
  - reworking retry logic shape
  - larger cache-manifest redesign

## Technical-Debt Reduction Register

- Safe to reduce now:
  - offline-unfriendly key requirement on full cache hits
  - delete-before-rename manifest replacement
  - ambiguity in repaired integrity semantics
- Defer:
  - deep transport abstraction solely for retry testing
  - provider-specific overfitting in gap repair logic
- Useful debt that should not be "cleaned up" casually:
  - best-effort credential fallback to file/env
  - conservative repair limits that prefer re-download over unbounded in-memory repair

## Performance Opportunity Register

- Safe structural improvement:
  - none urgent beyond making full cache hits credential-independent
- Benchmark-sensitive or semantics-sensitive:
  - retry cadence changes
  - API request limit tuning
  - changing canonical 1m cache policy for callers

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - keep offline reuse of already-downloaded data working
  - keep best-effort credential fallback behavior
  - keep options underlier bootstrap compatibility
- Regression risk if changed carelessly:
  - cached datasets can become unreadable or undiscoverable
  - options imports can fail due to underlier download regressions
  - network retries can become too aggressive or too weak
- Safe change posture:
  - prefer small, operationally clear fixes over broad transport or cache rewrites
