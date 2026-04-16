# Final Consolidated Review: `src-tauri/src/polygon.rs`

- Review date: `2026-04-16`
- Source file: [polygon.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/polygon.rs:1>)
- First-pass packet: [polygon.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/polygon.rs.review.md:1>)
- Second-pass packet: [polygon.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/polygon.rs.review.md:1>)
- Third-pass packet: [polygon.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/polygon.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/017__src-tauri__src__polygon.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/reqwest/latest/reqwest/blocking/>
  - <https://doc.rust-lang.org/std/fs/fn.rename.html>
  - <https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all>
  - <https://massive.com/docs/rest/stocks/aggregates/custom-bars>

## Consolidated Judgment

The fourth-pass packet is valid and is the strongest current review. [polygon.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/polygon.rs:1>) is the backend owner for cache planning, manifest persistence, fetch/retry behavior, validation, repair, and top-off. It is not a thin IPC seam.

The highest-confidence safe-now issues are concrete and local:

- a `FullHit` cache path still records a fresh [last_updated_utc_ms](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/polygon.rs:1264>) and persists the manifest even though [used_cache_only = true](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/polygon.rs:1172>);
- append/full-refresh writes preserve stale [integrity: cached_meta.and_then(|m| m.integrity)](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/polygon.rs:1267>);
- [scan_csv_integrity](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/polygon.rs:720>) validates timestamps only, while [parse_csv_bar](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/polygon.rs:805>) requires the full six-column row contract;
- same-day intraday top-off can false-hit as complete because coverage and top-off planning are date-granular even though the provider endpoint accepts millisecond timestamps.

The first-pass persistence concern is still real, but the fourth pass correctly narrows it: manifest durability is one issue; the broader CSV/manifest durability story is a larger redesign and should not be oversold in a small patch.

## Safe-Now Priority

- Keep the bucket at `Small surgical refactor`.
- Make `DownloadCachePlan::FullHit` truly read-only:
  - avoid API-key lookup until a network path is needed;
  - skip manifest rewrite and `last_updated_utc_ms` update on full cache hits.
- Clear or recompute integrity whenever bytes change; preserving stale integrity after append/full refresh is a truthfulness bug.
- Tighten validation to the real CSV row contract or explicitly narrow the validation claim if a lighter scan must stay.
- Add a narrow same-day top-off correction so earlier-in-the-day `max_ts` does not suppress a needed trailing fetch.

## Deferred Work

- Defer a larger cross-platform durable-replace helper for both manifest and CSV writes.
- Defer stronger manifest/file coherence redesign.
- Do not turn this pass into a transport-abstraction rewrite just for retry tests.

## Verification If Changed

- Keep the existing local polygon tests green.
- Add targeted regression cases for:
  - cache-only full hit not rewriting freshness metadata;
  - integrity clearing/recomputation after write paths;
  - malformed numeric fields failing validation;
  - same-day intraday top-off not false-hitting as complete.
- Then re-run caller-layer coverage for the command wrappers that invoke this module.

## Regression And Feature-Preservation Constraints

- Preserve the credential fallback chain.
- Do not treat provider-side missing aggregates as automatic corruption.
- Preserve the current canonical underlier storage policy unless commands and UI move together.
- Do not market the write path as crash-safe unless both manifest and CSV durability are actually addressed.
- Keep same-day top-off fixes narrow; do not widen every historical cache reuse into an unnecessary redownload.

## Final Conclusion

The fourth-pass packet is valid, no wrong fourth-pass file is present, and the final bucket remains `Small surgical refactor`. The right implementation target is truthfulness hardening around cache-only hits, integrity metadata, validation semantics, and same-day top-off behavior, while larger durability redesign stays deferred.
