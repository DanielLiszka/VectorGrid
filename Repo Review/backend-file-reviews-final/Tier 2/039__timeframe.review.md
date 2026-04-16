# Final Consolidated Review: `crates/app_core/src/timeframe.rs`

- Review date: `2026-04-16`
- Source file: [timeframe.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/timeframe.rs:1>)
- First-pass packet: [timeframe.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/timeframe.rs.review.md:1>)
- Second-pass packet: [timeframe.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/timeframe.rs.review.md:1>)
- Third-pass packet: [timeframe.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/timeframe.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/039__crates__app_core__src__timeframe.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Safe-now hardening`
- External research anchors:
  - <https://pandas.pydata.org/docs/reference/api/pandas.DataFrame.resample.html>
  - <https://docs.pola.rs/api/python/stable/reference/dataframe/api/polars.DataFrame.group_by_dynamic.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth pass correctly upgrades this file out of `Document only`. That is the right final call.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and should govern the final review.

The mismatch is concrete: [`parse_timeframe_label()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/timeframe.rs:27>) accepts fixed intervals above one day, while [`resample_candles()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/timeframe.rs:88>) resets the bucket lattice at each UTC-day boundary. In practice, labels such as `2d` or `36h` are accepted even though the live resampler cannot aggregate across multiple days. That is no longer a documentation-only concern. It is a bounded contract mismatch on a public export surface.

## Safe-Now Priority

- Make `resample_candles()` reject `interval_ms > DAY_MS` with an explicit error stating that the current algorithm is UTC-day-scoped and does not support multi-day-spanning buckets.
- Add rustdoc clarifying the actual public contract:
  - UTC-day reset behavior,
  - first-observed-bar-of-day anchoring,
  - left-edge bucket timestamps,
  - optional-field aggregation from finite values only,
  - field-flag downgrade when no finite output values exist.
- Add tests for:
  - rejecting intervals greater than one day,
  - truncated-day anchoring,
  - field-flag downgrade when source flags are true but all values are non-finite.

## Deferred Work

- Do not implement true multi-day resampling in this pass.
- Do not silently re-anchor to exchange sessions, wall-clock midnight, or a rolling window policy here.
- Do not change duplicate-timestamp policy casually; that is a separate semantics decision.

## Verification If Changed

- `cargo test -p ta_app_core timeframe::tests:: -- --nocapture`
- Add and run the new `> 1d` rejection and truncated-day anchoring regressions before merge.

## Regression And Feature-Preservation Constraints

- Preserve current `120s -> 2m` back-compat normalization.
- Preserve `1d` behavior as UTC-day-scoped, first-observed-bar-of-day, left-edge-labeled resampling unless a broader product decision changes it.
- Treat label normalization changes as compatibility-sensitive because `TimeframeSpec.label` is a public export and may be persisted or compared textually.

## Final Conclusion

Promote `timeframe.rs` to `Safe-now hardening`. The immediate implementation should close the accepted-label / actual-algorithm gap by rejecting `> 1d` resample requests, then document and test the true UTC-day-scoped contract. Broader timeframe-model redesign remains deferred.
