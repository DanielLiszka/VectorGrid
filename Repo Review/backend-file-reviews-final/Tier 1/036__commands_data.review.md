# Final Consolidated Review: `src-tauri/src/commands_data.rs`

- Review date: `2026-04-16`
- Source file: [commands_data.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_data.rs:1>)
- First-pass packet: [commands_data.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_data.rs.review.md:1>)
- Second-pass packet: [commands_data.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_data.rs.review.md:1>)
- Third-pass packet: [commands_data.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_data.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/036__src-tauri__src__commands_data.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with reprioritization`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
  - <https://doc.rust-lang.org/std/path/struct.Path.html#method.starts_with>
  - <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
  - <https://polygon.io/docs/websocket/stocks/aggregates-per-minute>

## Consolidated Judgment

The first and second passes were right to keep this file in `Small surgical refactor`. `commands_data.rs` is a public Tauri/runtime boundary with a few sharp correctness problems, but those problems still cluster around bounded command and event handling rather than architecture churn. The third pass correctly noticed that market-data-root migration is path-sensitive, but it over-weighted canonicalized move-guard hardening as the next fix.

The fourth pass is the best implementation-facing packet because it reprioritizes toward more concrete live defects already visible in the current file: the silent failed-`Pong` live-stream exit, too-permissive live-bar normalization, the destructive fixed-name writability probe, and the stale sample-data lookup. Those are stronger safe-now targets than a broader canonicalization patch. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Fix the failed-`Pong` branch in the Polygon live stream so it emits terminal status and disarms the live runner through the same non-manual interruption path as the other websocket failure branches.
- Tighten `normalize_polygon_live_bar` so it requires vendor-provided `sym`, enforces finite OHLC values, and validates timestamp ordering before emitting `polygon_live_bar` or handing the bar to the live runner.
- Replace the fixed `.vta_write_probe` writability check with a non-destructive create-new or unique-temp probe so `set_market_data_dir_for_base` cannot clobber an existing user file.
- Repair `list_sample_csvs` by reusing the repo/workspace discovery helpers and the real top-level `data/` surface rather than the stale `src/data` assumption.
- If this file is already open for command-boundary cleanup, move the direct `load_price_data` and `resample_price_data` wrappers off the synchronous command path using a Tauri-supported async/state-access pattern.

## Deferred Work

- Defer canonicalized or identity-based market-data-root move-guard comparison. The risk class is real, but `canonicalize` is not a trivial drop-in for first-run or non-existent descendant paths.
- Defer any larger task-center parity redesign for `polygon_top_off_dataset` unless frontend/backend task semantics are being reviewed together.
- Defer Windows dialog modernization.
- Defer broader packaging policy for advisory repo-root assets such as `gpu_speedups.csv`.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test settings_tests -- --nocapture`
- `npm --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/services/executionService.test.ts src/tauri/instrumentation.test.ts src/stores/taskStoreEventThrottle.test.ts src/stores/taskStore.test.ts`
- Add targeted Rust tests for:
  - failed-`Pong` shutdown/disarm behavior
  - live-bar rejection for missing `sym`
  - live-bar rejection for invalid OHLC or timestamp ordering
  - non-destructive writability probing
  - sample-data discovery against the real repo layout

## Regression And Feature-Preservation Constraints

- Preserve current command names and live event names.
- Preserve `polygon_live_bar` field names.
- Do not broaden backend-side path trimming for filesystem inputs.
- If move-guard comparison is hardened later, use canonicalization only for comparison, not as the persisted or displayed user path.
- Preserve cooperative cancellation semantics for Polygon download and top-off flows; `spawn_blocking` still does not provide true abort once running.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `commands_data.rs` in `Small surgical refactor`
- prioritize the live-stream exit path, live-bar validation, non-destructive writability probing, and sample-data discovery ahead of canonicalized move-guard work
- treat synchronous command cleanup as secondary boundary hardening, not as the top bug in the file
- defer broader migration-identity, task-center parity, and packaging-policy redesign
