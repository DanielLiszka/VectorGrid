# Second-Pass Review: `src-tauri/src/commands_data.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_data.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_data.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already found three material concerns:

- `list_sample_csvs` is stale against the live repo layout
- `load_price_data` and `resample_price_data` remain synchronous command wrappers around heavier state work
- the live-stream and dataset command surface still has semantics-sensitive observability and cancellation gaps

## Stale Check

Those issues still stand in the live source:

- the sample-data lookup still points at an outdated repo-relative location
- `load_price_data` and `resample_price_data` are still synchronous Tauri commands
- background download/top-off still rely on cooperative cancellation around blocking helpers

The second pass did not change the bucket, but it did find two more concrete parity issues in the same surface.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - tighten the async-command recommendation around borrowed `State<'_, T>` and heavier state-loading paths
   - Additional constraint informed:
     - the heavier filesystem/state mutation commands can move off the synchronous command path without changing public command names

2. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - refine the treatment of background Polygon download and top-off cancellation
   - Additional constraint informed:
     - once the blocking helper starts, wrapper-level abort is not possible; responsiveness depends on cooperative polling inside the blocking helper

3. Rust `std::fs::create_dir_all`
   - Source: <https://doc.rust-lang.org/std/fs/fn.create_dir_all.html>
   - Why consulted in second pass:
     - verify whether "small" market-directory helpers are actually cheap metadata operations
   - Additional constraint informed:
     - directory setup and probe writes in market-directory commands are real filesystem mutation work

4. Tauri event guidance
   - Source: <https://v2.tauri.app/develop/calling-frontend/>
   - Why consulted in second pass:
     - validate how strongly the review should treat `polygon_live_status` and `polygon_live_bar` payloads as public contracts
   - Additional constraint informed:
     - the emitted payloads remain cross-layer compatibility surface, not local implementation detail

## Delta vs First Pass

## New Command-Path Finding

### 1. The "move heavy state work off the synchronous command path" concern is broader than just load and resample

- Evidence:
  - `set_market_data_dir` still performs directory creation, probe writes, directory scanning, and optional cache migration on the synchronous command path
- Why this matters:
  - this is still user-triggered filesystem work, not a trivial in-memory setting toggle
- Practical implication:
  - if this file is touched for command-boundary cleanup, treat the market-directory mutation path as part of the same review set

## New Cancellation-Parity Finding

### 2. `polygon_top_off_dataset` still shares the background download slot and cancel token, but the frontend task model does not track it like the main download command

- Evidence:
  - both workflows use the same background-task slot and cooperative cancellation model
  - the frontend task store tracks/cancels the main download path, not the top-off path, with the same fidelity
- Why this matters:
  - the UX contract is currently inconsistent across two backend flows that share the same cancellation mechanics
- Practical implication:
  - the safest direction is parity:
    - either track top-off with the same task-center/cancel surface,
    - or intentionally scope both workflows differently

## First-Pass Findings Still Stand

### 3. The original bucket still fits

- the stale sample-data lookup is still a real defect
- the synchronous `load_price_data` / `resample_price_data` issue is still justified
- the second pass did not uncover a reason to inflate this into broader redesign work

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test settings_tests -- --nocapture`
- `npm --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/services/executionService.test.ts src/tauri/instrumentation.test.ts src/stores/taskStoreEventThrottle.test.ts src/stores/taskStore.test.ts`

If live-stream or background-task behavior changes, also run a manual desktop smoke pass for:

- start live candles
- verify `polygon_live_status` transitions
- verify accepted live bars still reach the live runner path
- verify `polygon_top_off_dataset` task visibility and cancellation semantics

## Regression And Feature-Preservation Constraints

- Preserve current public command names and event names
- Preserve helper-level cooperative cancellation for download/top-off paths
- Preserve frontend-compatible live event payload semantics unless updated in lockstep
- Fix `list_sample_csvs` without narrowing the current sample-data UX

## Second-Pass Conclusion

`Bucket unchanged, but the synchronous-command and cancellation-parity edges are sharper`

The first-pass `Small surgical refactor` bucket remains correct. The second pass clarifies that the command-boundary issue also covers `set_market_data_dir`, and that `polygon_top_off_dataset` still has a real frontend tracking/cancellation parity gap relative to the main download flow.
