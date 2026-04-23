# Fourth-Pass Review: `src-tauri/src/polygon.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/polygon.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file needed a fourth pass because the current third-pass packet reviewed the wrong abstraction.

The live source is not a thin Tauri IPC wrapper. It is the concrete backend owner for Polygon cache planning, manifest persistence, credential fallback, HTTP aggregate fetching, validation, repair, and top-off behavior across roughly the whole file surface (`src-tauri/src/polygon.rs:15-1277`). The actual public Tauri command boundary is one layer up in the command registry (`src-tauri/src/lib.rs:82-100`) and command re-export surface (`src-tauri/src/commands.rs:61-72`). That means the third-pass packet’s emphasis on IPC naming, runtime authority, and wrapper drift was directionally cautious, but materially off-center for this source file.

The current third-pass bucket should **stand**, but for a different reason than the third-pass packet gave. This is still `Small surgical refactor` code because the highest-confidence fixes are narrow and local. The packet itself, however, should not stand as written, because it under-reviewed the operational behavior that actually matters here.

## Fourth-Pass Research Log

This pass re-read the live source first, then used the bundle context only where it changed the review outcome.

1. **Live-source re-read**
   - Re-opened `src-tauri/src/polygon.rs` directly, plus the third-pass packet.
   - Used `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs` only to verify whether this file is actually the public IPC seam. It is not.

2. **Filesystem durability and replacement semantics**
   - Re-checked the official Rust docs for `std::fs::rename` and `std::fs::File::sync_all`.
   - This matters because the earlier packet correctly identified a manifest replacement risk, but its implied fix was too loose to call cross-platform-safe by itself.

3. **Blocking HTTP and caller-layer cancellation**
   - Re-checked the official `reqwest::blocking` and `tokio::task::spawn_blocking` docs.
   - This confirmed that the third-pass packet’s blocking/cancellation concerns are real, but they live mostly in the command layer that calls this file, not in `polygon.rs` itself.

4. **Provider-side aggregate semantics**
   - Re-checked Massive/Polygon aggregate documentation and the missing-aggregates knowledge-base guidance.
   - This matters for two reasons:
     - same-day gaps are not always corruption;
     - the endpoint accepts either dates **or millisecond timestamps**, which means this file’s date-only shaping is a local simplification, not a hard provider constraint.

5. **Environment note**
   - This bundle did not include the full repo/workspace layout needed to honestly claim fresh `cargo test` execution here, so the verification section below is prescriptive rather than reported-as-run.

## What Fourth Pass Sharpened

### Bucket assessment

The third-pass action bucket should **remain `Small surgical refactor`**.

That is not because this file is a thin wrapper. It is because the best next changes are still bounded and regression-resistant:

- make cache-only hits truly read-only;
- stop preserving stale integrity metadata after writes;
- tighten validation semantics so they match the actual row contract;
- harden same-day intraday top-off behavior without redesigning the whole cache model.

A larger redesign is still deferrable, but the third-pass packet under-described why.

### What the current third-pass packet got right

The third-pass packet got three things right.

First, it was right **not** to recommend broad wrapper consolidation or command-surface churn. There is no evidence in the live source that this file wants a structural rewrite.

Second, it was right that `spawn_blocking`/cancellation semantics matter for this workflow family. That concern just belongs primarily to the command layer that invokes this module, not to the module owner itself.

Third, it was right that conservative contract hardening is the right posture. This file is network-sensitive, cache-integrity-sensitive, and options-underlier-sensitive support code, so correctness and truthfulness matter more than cleanup aesthetics.

### What the third-pass packet missed or underweighted

#### 1. The file-owner classification is wrong

The packet’s center of gravity is “public IPC seam,” but the live file is the backend owner beneath that seam.

- `src-tauri/src/polygon.rs` owns credential resolution, cache planning, fetch/retry behavior, manifest load/save, validation, repair, and top-off request shaping (`src-tauri/src/polygon.rs:318-1277`).
- The actual command registry is in `src-tauri/src/lib.rs:82-100` and the command re-export layer is in `src-tauri/src/commands.rs:61-72`.

That misclassification matters because it caused the packet to over-focus on command identity and under-focus on cache truthfulness and persistence behavior.

#### 2. Full cache hits are not actually read-only

A `FullHit` path still mutates the manifest.

- Cache planning returns `FullHit` at `src-tauri/src/polygon.rs:441-446` and the main path sets `used_cache_only = true` at `1171-1173`.
- But the function still rebuilds metadata with `last_updated_utc_ms: now_utc_ms()` and saves the manifest at `1256-1270`.

That creates two real problems the third-pass packet missed:

- a pure cache reuse is recorded like a fresh dataset update;
- a no-op read path still performs extra disk I/O and reopens the manifest replacement failure window.

This is both a correctness issue and a small performance issue. It is also the narrowest, safest place to improve the file immediately.

#### 3. Download writes preserve stale integrity metadata

After append or full refresh, the file still carries forward the old `integrity` blob.

- Append writes happen at `1207-1211`.
- Full refresh rewrites happen at `1241-1247`.
- The saved metadata still uses `integrity: cached_meta.and_then(|m| m.integrity)` at `1267`.

That means a dataset can be rewritten and still advertise integrity results that describe an earlier file revision. For a file that already exposes `validate_cached_dataset` and `repair_cached_dataset`, this is a truthfulness bug, not just a cosmetic metadata choice.

The safest correction is not a redesign: preserve integrity only on true cache-only hits, and clear it when the file bytes changed.

#### 4. `validate_cached_dataset` overpromises what it validates

The current “validate” path only validates timestamps, not full CSV row parseability.

- `scan_csv_integrity` reads the first field as `timestamp` and stops there (`src-tauri/src/polygon.rs:750-760`).
- It never parses `open`, `close`, `high`, `low`, or `volume`.
- By contrast, repair uses `parse_csv_bar`, which requires the full six-column row to parse (`805-813`, `953-962`).

So a row with a valid timestamp and broken numeric fields can pass validation, count toward `row_count`, and keep the dataset looking structurally sound until a later load/repair path disagrees.

That is a stronger missed edge case than anything in the current third-pass packet. The module either needs to tighten validation to the actual row contract or rename/document the current routine as timestamp-order-only scanning.

#### 5. Same-day intraday top-off can false-hit as “complete”

This is the most important new edge case from the fourth pass.

- `build_top_off_request_for_dataset` derives the request from the cached **last date**, not the last timestamp (`src-tauri/src/polygon.rs:1051-1074`).
- `plan_download_cache_strategy` decides `FullHit` at **date** granularity (`441-446`).
- The main download path then short-circuits on `FullHit` (`1171-1173`).

So if the latest cached bar is earlier on the same day, a same-day top-off can be suppressed even though newer bars may exist.

The file comments partially excuse this by saying cache coverage is date-granular because “Polygon aggs API uses dates” (`1155`). That is too forgiving. The official Massive stocks custom-bars docs state that `from` and `to` accept either `YYYY-MM-DD` **or millisecond timestamps**, so the date-only shaping here is a local simplification, not a provider limitation.

That does **not** mean the whole module should immediately switch to timestamp-based fetching. It **does** mean the same-day top-off blind spot is a real local bug/contract weakness, not an unavoidable upstream constraint.

#### 6. Manifest durability is a real issue, but fixing only that surface is narrower than the packet implied

The first-pass manifest finding was real and should remain in scope.

- `save_manifest` writes a temp file, removes the existing manifest, then renames the temp file into place (`src-tauri/src/polygon.rs:410-419`).

But the fourth pass sharpens the conclusion:

- `overwrite_bars` truncates and rewrites the CSV in place (`690-713`);
- that path is used by repair (`1007`) and full refresh (`1243`).

So the module’s persistence story is broader than manifest replacement alone. Fixing the manifest swap is still worthwhile, but it should not be described as if that alone makes the cache durable or crash-safe.

### What the current packet overstated or weakly supported

#### 1. The “thin public IPC boundary” framing is not supported by the live code

This is the biggest overstatement.

The packet presents `polygon.rs` as if it were mostly a command wrapper or DTO boundary. The bundled repo context shows otherwise. The public Tauri boundary is in `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs`; `polygon.rs` is the logic owner behind those commands.

That matters because the right review target is persistence, cache truthfulness, validation semantics, and provider behavior — not primarily command naming or runtime authority wording.

#### 2. “Switch to a replacement strategy that does not remove the destination first” is directionally correct but under-specified

The underlying concern is valid. The implied implementation is not yet review-ready.

Official Rust docs note that `std::fs::rename` replacement behavior differs across platforms, especially on Windows when both paths already exist. So a naive “just rename over the destination” patch is not a safe cross-platform recommendation by itself.

In addition, official Rust docs also note that durability claims require explicit flush/sync handling if you want stronger guarantees than “best effort on close.” So even a temp-file rewrite plus rename is still not a strong crash-safety story if the write path never explicitly syncs.

That means the current packet’s persistence recommendation should be narrowed:

- safe now: remove the delete-before-rename window only if the replacement helper is explicitly chosen for the supported platforms;
- deferred: do not market the module as crash-safe unless the CSV rewrite paths and flush semantics are also hardened.

#### 3. The verification guidance is too generic for the real risks

A broad `cargo test ... polygon` or manual IPC smoke can still miss the file’s biggest truthfulness regressions:

- full-hit cache reuse mutating manifest timestamps;
- stale integrity surviving downloads;
- validation accepting rows whose numeric fields are broken;
- same-day intraday top-off false-hitting as complete.

Those need targeted file-local tests. Caller-level `commands_data` and `commands_options_data` coverage should come **after** those, not instead of them.

## Implementation Priority

### Safe-now hardening

1. **Make `FullHit` truly read-only.**
   - Defer API-key lookup until a network path is actually needed.
   - On `DownloadCachePlan::FullHit`, return without rewriting `last_updated_utc_ms` and without saving the manifest.
   - This is the safest correctness fix in the file because it removes false freshness and unnecessary writes.

2. **Stop preserving stale integrity after file writes.**
   - Keep existing integrity only on a true cache-only return path.
   - If append/full refresh changes bytes, either clear `integrity` to `None` or recompute it immediately.
   - Clearing is the lower-risk default because it avoids a silent lie without forcing extra I/O.

3. **Tighten validation to the real CSV row contract.**
   - Update `scan_csv_integrity` to parse full rows with the same field requirements used by `parse_csv_bar`, or explicitly narrow the function name/documentation if the current lighter scan must stay.
   - The current `validate_cached_dataset` name is stronger than the current implementation.

4. **Add a narrow same-day intraday top-off correction.**
   - The safest first change is top-off-path-specific, not a full cache-plan redesign.
   - For example, avoid letting same-day intraday top-off requests short-circuit as `FullHit` when the dataset’s `max_ts` is earlier than the current day’s trailing bars.
   - Keep the scope tight so this does not accidentally widen request volume for unrelated historical loads.

5. **Document the local simplifications honestly.**
   - `download_1m_to_cache` is more general than its name.
   - date-only request shaping is a local product choice, not a provider requirement.
   - gap repair remains best-effort because provider-side missing aggregates are not always corruption.

### Larger redesign still deferred

1. **Cross-platform durable replace helpers for both manifest and CSV files.**
   - This should be treated as a larger redesign because it needs one coherent policy for temp-file naming, replacement semantics, and flush behavior across Windows and Unix.
   - Do not smuggle this in as a “tiny rename cleanup.”

2. **Stronger manifest/file coherence.**
   - Today the cache plan trusts manifest min/max metadata heavily.
   - A more robust design would track stronger coherence evidence such as file fingerprinting, explicit row-count recovery, or explicit trailing-day completeness markers.
   - That is useful debt, but it is not required to land the safe-now fixes above.

3. **Transport abstraction solely for retry tests.**
   - The current blocking-client reuse inside one request is reasonable and aligned with the docs.
   - A deeper transport seam should remain deferred unless provider behavior or test pressure makes it worth the added complexity.

## Verification If Changed

Because I did not have a full workspace here to honestly claim fresh execution, this is the recommended verification order rather than a reported test run.

1. **Keep the existing file-local tests green.**
   - `polygon_validate_and_repair_detects_and_fixes_duplicates_and_ordering`
   - `build_top_off_request_uses_cached_dataset_identity_and_now_date`
   - `build_top_off_request_rejects_dataset_older_than_free_plan_window`
   - `read_api_key_falls_back_to_env_vars_when_cache_is_empty`
   - `plan_download_cache_strategy_distinguishes_hit_append_and_refresh`
   - `load_manifest_upgrades_legacy_adjusted_stock_entries`

2. **Add targeted file-local regression tests for the fourth-pass findings.**
   - cache-only full hit succeeds without an API key and does **not** rewrite `last_updated_utc_ms`
   - download append/full-refresh clears or recomputes stale integrity
   - validation flags rows whose numeric fields are malformed even when the timestamp parses
   - same-day intraday top-off does not false-hit as complete when the cached `max_ts` is earlier in the same day

3. **Then run caller-layer coverage.**
   - `commands_data` coverage for download/top-off/validate/repair wrappers
   - `commands_options_data` coverage for options-underlier bootstrap behavior

4. **Avoid broad performance work here.**
   - There is no GPU ABI risk in this file.
   - The meaningful performance win is simply removing unnecessary no-op manifest writes on cache hits.

## Regression And Feature-Preservation Constraints

- Preserve the current best-effort credential fallback chain: keyring -> local fallback file -> environment / `.env` fallback.
- Do **not** convert provider-side missing aggregates into automatic corruption findings. Massive/Polygon can legitimately omit bars when no eligible trades occur.
- Preserve the caller-layer canonical `1m` storage policy unless commands and UI move together. This file is more generic than its name, but the shipped product contract still appears to prefer canonical `1m` underlier storage.
- If you harden persistence, do not claim “crash-safe” behavior unless both manifest replacement **and** CSV rewrite paths are addressed, and the write path’s durability semantics are explicit.
- If you tighten validation to parse full rows, expect some previously “valid” malformed CSVs to start reporting higher parse-error counts. That is a truthfulness change, not necessarily a regression.
- Any same-day top-off fix must avoid accidentally turning every historical cache reuse into a trailing-day redownload. Keep it narrow.
- Options-underlier callers depend on this module’s success/failure contract. Changes that widen failures or change path semantics should be verified against that workflow explicitly.

## Fourth-Pass Conclusion

The current third-pass **bucket should stand**, but the third-pass **packet should not**.

`src-tauri/src/polygon.rs` is not a low-yield IPC seam. It is a high-sensitivity cache/download/repair owner with a handful of concrete, still-missed truthfulness issues:

- cache-only hits still behave like writes;
- integrity metadata can survive after the file changed;
- validation does not validate the full row contract;
- same-day intraday top-off can false-hit as complete;
- persistence hardening is real, but the cross-platform and crash-safety story is narrower and trickier than the current packet implies.

So the correct fourth-pass conclusion is:

- **keep** the `Small surgical refactor` action bucket;
- **replace** the third-pass framing with a file-owner/correctness framing;
- **land** a narrow hardening pass around full-hit behavior, integrity truthfulness, validation semantics, and same-day top-off;
- **defer** broader persistence and cache-model redesign until the repo wants to pay for a real cross-platform durability story.
