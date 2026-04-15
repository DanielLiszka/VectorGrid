# Review Packet: `src-tauri/src/commands_data.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_data.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and mostly coherent, but not yet clean enough to accept as-is`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Runtime-critical`, `Semantics-sensitive`

## What This File Owns

`commands_data.rs` is the Tauri-side data/runtime boundary for the desktop app's dataset and market-data workflows.

This file owns:

- public Tauri commands for dataset load, resample, and clear:
  - `load_price_data`: `src-tauri/src/commands_data.rs:20-29`
  - `resample_price_data`: `src-tauri/src/commands_data.rs:31-37`
  - `clear_loaded_data`: `src-tauri/src/commands_data.rs:72-77`
- catalog and discovery surfaces:
  - `list_moving_averages`: `src-tauri/src/commands_data.rs:80-82`
  - `list_dynamic_indicators`: `src-tauri/src/commands_data.rs:85-87`
  - `list_gpu_speedups`: `src-tauri/src/commands_data.rs:224-228`
  - `list_sample_csvs`: `src-tauri/src/commands_data.rs:263-266`
  - `list_repo_data_csvs`: `src-tauri/src/commands_data.rs:413-431`
- market-data-root config and Polygon cache/API-key management:
  - `get_market_data_dir_info`: `src-tauri/src/commands_data.rs:465-467`
  - `set_market_data_dir`: `src-tauri/src/commands_data.rs:534-541`
  - `clear_market_data_dir`: `src-tauri/src/commands_data.rs:470-473`
  - Polygon cache/API-key commands at `src-tauri/src/commands_data.rs:544-565`
- Polygon live websocket bridge and related event payloads:
  - request/response/status DTOs: `src-tauri/src/commands_data.rs:573-637`
  - status emission: `src-tauri/src/commands_data.rs:676-697`
  - live-bar normalization and websocket handling: `src-tauri/src/commands_data.rs:719-797`
  - start/stop/status commands: `src-tauri/src/commands_data.rs:1003-1119`
- Polygon dataset validation, repair, download, and top-off wrappers:
  - `polygon_validate_dataset`: `src-tauri/src/commands_data.rs:1122-1133`
  - `polygon_repair_dataset`: `src-tauri/src/commands_data.rs:1135-1148`
  - `polygon_download_1m_and_load`: `src-tauri/src/commands_data.rs:1182-1233`
  - `polygon_top_off_dataset`: `src-tauri/src/commands_data.rs:1235-1286`
- Windows-native CSV/file/folder picker commands:
  - `pick_csv_file`: `src-tauri/src/commands_data.rs:1293-1332`
  - `pick_save_csv`: `src-tauri/src/commands_data.rs:1334-1387`
  - `pick_folder`: `src-tauri/src/commands_data.rs:1389-1437`

This file does not own:

- dataset retention, source identity, or derived-dataset invalidation policy
- CSV parse/cache implementation
- Polygon manifest, download, validation, or repair internals
- live execution runner logic
- frontend normalization, task-center rendering, or settings UI

Those concerns are delegated to:

- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- frontend consumers in `web/src/services/*.ts`, `web/src/stores/*.ts`, and `web/src/tauri/instrumentation.ts`

## Classification

- Public surface: yes
  - command registration appears in `src-tauri/src/lib.rs:82-119` and `src-tauri/src/lib.rs:185-187`
  - grouped re-export appears in `src-tauri/src/commands.rs:61-68`
- Hot path: mixed
  - most of the file is command-boundary orchestration
  - the live-bar handler is a real runtime hot path because every accepted Polygon live bar flows through `handle_polygon_ws_text` and `on_live_execution_bar`
- Runtime-critical: yes
  - this file controls loaded-dataset mutation, market-data-root moves, live stream lifecycle, background download exclusivity, and task/event contracts
- Benchmark-sensitive: no formal benchmark gate applies to this file directly
  - however, repo CSV scans and live-bar processing are performance-sensitive enough that unnecessary churn is still risky
- CUDA/runtime-critical: runtime-critical, not CUDA-critical
- Strategy-local: no
- Low-risk support code only: no
  - some subsections are low-risk support code, but the file as a whole is not low-risk

## Direct Interactions

### Backend owners this file delegates to

- dataset load/resample/invalidate:
  - `src-tauri/src/state.rs:192-347`
- loaded dataset context and live stream slot/status state:
  - `src-tauri/src/state.rs:577-717`
- app-local cache/identity logic:
  - `src-tauri/src/data_cache.rs`
- market-data-root persistence:
  - `src-tauri/src/settings.rs:36-177`
- Polygon cache/API-key/download/validate/repair internals:
  - `src-tauri/src/polygon.rs`
- live execution runner filtering and interruption handling:
  - `src-tauri/src/commands_execution.rs:718-742`
  - `src-tauri/src/commands_execution.rs:1185-1197`
  - `src-tauri/src/commands_execution.rs:1842-1866`

### Frontend contract anchors

- data workspace owner:
  - `web/src/services/dataService.ts:370-606`
- live execution owner:
  - `web/src/services/executionService.ts:1365-1373`
  - `web/src/services/executionService.ts:1759-1875`
- moving-average catalog:
  - `web/src/services/movingAverageCatalogService.ts:149`
  - `web/src/services/optimizeService.ts:1722`
- dynamic indicator catalog:
  - `web/src/services/dynamicIndicatorCatalogService.ts:264`
- GPU speedup catalog:
  - `web/src/services/gpuSpeedupsService.ts:60`
- batch/meta dataset scan:
  - `web/src/services/metaService.ts:1734-1758`
- frontend command-name registry:
  - `web/src/tauri/commands.ts:2-20`
  - `web/src/tauri/commands.ts:37-39`
  - `web/src/tauri/commands.ts:105-107`

### REPO_MAP contract anchors

- command-module ownership:
  - `REPO_MAP.md:558-559`
- live event channel ownership:
  - `REPO_MAP.md:620-623`
- load-price-data progress expectations:
  - `REPO_MAP.md:838-839`
- data workspace notes:
  - `REPO_MAP.md:1535-1541`
- Polygon cache/live-stream notes:
  - `REPO_MAP.md:1556-1560`

## Standards and Research Log

This packet was written only after a targeted research pass against official or primary sources relevant to Tauri command boundaries, blocking work, event contracts, Polygon websocket behavior, and native Windows dialogs.

### Sources consulted

1. Tauri command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file exports a large public command surface
     - some commands are lightweight wrappers, while others are clearly blocking CSV/cache operations
   - Standard informed:
     - async commands are preferred for heavy work
     - non-async commands run on the main thread
     - `Result<_, _>` remains the practical async workaround when borrowed types such as `State<'_, T>` are involved

2. Tokio `spawn_blocking` documentation
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why it was consulted:
     - this file already uses `spawn_blocking` for repo scans, dataset validation/repair, and Polygon downloads
   - Standard informed:
     - blocking work belongs off the async/UI-sensitive path
     - `spawn_blocking` tasks are cooperative for cancellation but cannot be aborted once started

3. Tauri frontend event guidance
   - Source: <https://v2.tauri.app/develop/calling-frontend/>
   - Why it was consulted:
     - this file emits `polygon_live_status` and `polygon_live_bar`
   - Standard informed:
     - event payloads should be explicit, serializable, and stable across the frontend/backend boundary

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines several public DTOs and user-facing command names
   - Standard informed:
     - keep contracts predictable, unsurprising, and easy to test

5. Rust API Guidelines naming guidance
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - command names and DTO field choices are part of a cross-layer contract
   - Standard informed:
     - avoid churn unless a change materially improves clarity or correctness

6. Rust Book error handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this file mostly forwards fallible runtime work as `Result<_, String>`
   - Standard informed:
     - preserve actionable failure information and avoid introducing new opaque failure modes

7. Rust Book testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why it was consulted:
     - this file already has strong helper tests, but the review needed to identify real contract gaps
   - Standard informed:
     - tests should cover actual behavior boundaries, not just helpers

8. Clippy documentation
   - Source: <https://doc.rust-lang.org/clippy/>
   - Why it was consulted:
     - this module should stay boundary-simple enough that correctness and complexity lint pressure remains meaningful
   - Standard informed:
     - avoid avoidable suspiciousness and overly clever local helper logic

9. Polygon Stocks WebSocket minute aggregate documentation
   - Source: <https://polygon.io/docs/websocket/stocks/aggregates-per-minute>
   - Why it was consulted:
     - this file hardcodes `AM.<symbol>` subscription channels and only accepts `AM` live-bar events
   - Standard informed:
     - keep event/channel assumptions aligned with current vendor docs and treat payload-shape changes as contract-sensitive

10. Microsoft `OPENFILENAMEW` documentation
    - Source: <https://learn.microsoft.com/en-us/windows/win32/api/commdlg/ns-commdlg-openfilenamew>
    - Why it was consulted:
      - this file manually wires native open/save dialogs
    - Standard informed:
      - confirm filter/default-extension usage is reasonable and explicit

11. Microsoft `BROWSEINFO` / `SHBrowseForFolder` documentation
    - Source: <https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-browseinfoa>
    - Why it was consulted:
      - this file manually configures the native folder picker using `BIF_EDITBOX` and `BIF_NEWDIALOGSTYLE`
    - Standard informed:
      - folder-dialog behavior is sensitive to COM apartment setup and should not be changed casually

## Applicable Current Standards

- Tauri command-boundary standard:
  - heavy CSV/cache work should not run on the main-thread command path
  - registered command names should remain stable unless the frontend is updated in lockstep
- Event contract standard:
  - live status/bar payload fields should remain explicit and stable
  - live-event behavior must be reviewed alongside `web/src/tauri/instrumentation.ts` and task-store consumers
- Dataset/state standard:
  - loaded dataset identity and derived invalidation remain owned by `AppState`, not by command wrappers
- Polygon runtime standard:
  - cache/API-key/download/repair rules remain owned by `polygon.rs`
  - this wrapper should not fork or duplicate that policy
- Windows dialog standard:
  - keep the current explicit Windows-only posture unless a cross-platform replacement is intentionally introduced

## Findings

### Correctness / Safety

#### 1. `list_sample_csvs` is stale against the current repo layout

Objective issue:

- `sample_csvs_from_root` still looks for hard-coded `src/data/...` sample files at `src-tauri/src/commands_data.rs:238-259`
- `list_sample_csvs` constructs its root as `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")` at `src-tauri/src/commands_data.rs:263-266`
- for this crate, that resolves above the repo root
- in the current checkout, sample CSVs live under top-level `data/`, not `src/data/`

Local evidence:

- `src-tauri` manifest dir resolves to `C:\Rust Projects\ta_desktop_demo\src-tauri`
- current wrapper root resolves to `C:\Rust Projects`
- the resulting candidate path `C:\Rust Projects\src\data\10kCandles.csv` does not exist
- actual repo CSVs are under `C:\Rust Projects\ta_desktop_demo\data\*.csv`

Impact:

- sample CSV quick-loads are effectively broken or stale in this checkout
- current tests do not catch it because they only validate the helper against a synthetic `src/data` tree:
  - `src-tauri/src/commands_data.rs:1720-1737`
  - frontend test coverage around `list_sample_csvs` in `web/src/services/dataService.test.ts:169-227` only mocks the command response

Smallest justified fix:

- resolve the sample root from the actual repo `data/` directory instead of walking above the workspace and assuming `src/data`
- add one wrapper-level test that exercises the real root-resolution behavior, not just the helper candidate order

#### 2. `normalize_live_stock_symbol` performs lossy rewriting instead of strict validation

Objective issue:

- `normalize_live_stock_symbol` trims, uppercases, and then filters out unsupported characters rather than rejecting them:
  - `src-tauri/src/commands_data.rs:645-657`

Why it matters:

- punctuation is already treated as semantically meaningful elsewhere
- local tests explicitly protect `BRK.B` punctuation in live-bar matching:
  - `src-tauri/src/commands_data.rs:1889-1910`
- downstream symbol matching in the runner path is also punctuation-sensitive:
  - `src-tauri/src/commands_execution.rs:1185-1197`

Example risk:

- `BRK/B` becomes `BRKB`
- `AAPL$` becomes `AAPL`

That is a quiet rewrite, not a validation failure.

Smallest justified fix:

- reject unsupported characters instead of silently deleting them
- keep current allowed characters explicit and test invalid examples directly

#### 3. `polygon_validate_dataset` and `polygon_repair_dataset` do not trim `csv_path`

Objective issue:

- `polygon_top_off_dataset` trims `csv_path` before use at `src-tauri/src/commands_data.rs:1248`
- `polygon_validate_dataset` and `polygon_repair_dataset` forward raw `csv_path` directly at `src-tauri/src/commands_data.rs:1122-1148`

Impact:

- direct Tauri callers with padded paths can get avoidable manifest/path misses
- the shipped frontend is insulated because `web/src/services/dataService.ts:590-606` trims first

Smallest justified fix:

- normalize/trim `csv_path` consistently in validate and repair wrappers

### API and Naming

No naming churn is justified.

Assessment:

- command names are already part of a stable frontend contract in `web/src/tauri/commands.ts`
- DTO naming is descriptive enough for current use
- renaming would create migration cost without solving the real issues identified above

Cross-layer contract issue to note:

- Rust emits `polygon_live_bar` with `time_open_unix_ms` / `time_close_unix_ms` fields via `PolygonLiveBarEvent` at `src-tauri/src/commands_data.rs:620-636`
- frontend instrumentation only normalizes `at_unix_ms | time_unix_ms | t | ts` at `web/src/tauri/instrumentation.ts:115-121`
- task-center live-bar entries therefore fall back to receipt time instead of candle time:
  - `web/src/stores/taskStore.ts:1086-1105`

This is a real interaction defect, but the smallest justified fix lives in the frontend normalizer, not in this file.

### Error Handling

The file mostly follows the repo’s current `Result<_, String>` command-boundary pattern and keeps most deep error ownership in delegated modules.

Assessment:

- acceptable for current repo conventions
- not the place to introduce a typed cross-surface error redesign

One important operational note:

- `begin_background_download` plus `polygon.rs` cooperative cancel checks are doing the real work for download/top-off cancellation
- because Tokio `spawn_blocking` tasks cannot be aborted once running, that cooperative pattern should be preserved if these commands are changed

### Module Boundaries / Technical Debt

There is real but manageable boundary debt.

Objective observation:

- `commands_data.rs` is a mixed-scope boundary file at `1744` lines
- it currently mixes:
  - dataset load/resample/clear
  - catalog queries
  - repo/sample CSV discovery
  - market-data-root config
  - Polygon cache/API-key commands
  - live websocket bridge
  - download/repair/validate wrappers
  - Windows-native dialogs
  - tests

Assessment:

- this breadth raises review cost
- but the file is still understandable because most deep logic is delegated out
- a larger split is plausible in the future, but there is no need to force one before the local issues above are fixed

Deferred refactor posture:

- if split later, separate along these natural seams:
  - dataset/catalog commands
  - market-data-root and Polygon maintenance
  - live websocket bridge
  - Windows-native dialogs

### Docs / Examples

Local documentation gaps are modest but real.

The biggest gaps are:

- sample-data lookup assumptions are not documented and are now stale
- the `polygon_live_bar` payload contract is only implicit in Rust structs and frontend normalization code
- Windows-only dialog behavior is explicit in code, but not surfaced in a higher-level module note

Recommendation:

- do not add broad prose-heavy documentation
- do add short module comments or packet references if sample-root assumptions or live-bar payload fields are changed

### Tests / Missing Verification

Positive evidence:

- targeted Rust verification passed:
  - `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
  - result observed locally: `19 passed; 0 failed`
- file-local Rust tests cover:
  - clear/invalidate behavior: `src-tauri/src/commands_data.rs:1532-1568`
  - market-data-root set/clear/move/conflict: `src-tauri/src/commands_data.rs:1571-1693`
  - repo CSV discovery and identity: `src-tauri/src/commands_data.rs:1695-1815`
  - GPU speedup parsing: `src-tauri/src/commands_data.rs:1835-1851`
  - moving-average and dynamic-indicator listing: `src-tauri/src/commands_data.rs:1853-1887`
  - punctuation-preserving live-bar symbol matching: `src-tauri/src/commands_data.rs:1889-1910`

Important gaps:

- no Rust tests directly exercise the live-stream command lifecycle wrappers:
  - stocks-only gating
  - missing/empty API key behavior
  - `replaced_prior` behavior
  - stop/status command wrappers
- current `list_sample_csvs` tests do not model the actual repo layout and therefore miss the stale root bug
- frontend tests do not verify the real `polygon_live_bar` timestamp field names coming from Rust
- meta-service tests stub `list_repo_data_csvs`, but do not strongly assert invoke args or truncation/error hint handling:
  - `web/src/services/metaService.test.ts:305`
  - `web/src/services/metaServiceCampaignStrategy.test.ts:83`
  - `web/src/services/metaServiceCampaignResume.test.ts:115`

### Perf / Allocation / CUDA Concerns

No direct CUDA concerns exist in this file.

The important runtime/performance issues are elsewhere:

#### 1. `load_price_data` and `resample_price_data` are synchronous Tauri commands

Objective issue:

- `load_price_data` is a plain sync `#[tauri::command]` at `src-tauri/src/commands_data.rs:20-29`
- it delegates to `AppState::load_price_data`, which performs source identity lookup, cache load/build, timestamp normalization, metadata build, and in-memory dataset registration:
  - `src-tauri/src/state.rs:209-287`
- `resample_price_data` is also a plain sync command at `src-tauri/src/commands_data.rs:31-37`
- it delegates to `AppState::resample_loaded_data`, which can parse timeframe labels, inspect cache freshness, and resample candles:
  - `src-tauri/src/state.rs:290-347`

Why this matters:

- Tauri’s official docs state that async commands are preferred for heavy work and that commands without `async` run on the main thread
- these two commands are the heaviest synchronous commands in this file

Smallest justified fix:

- convert `load_price_data` and `resample_price_data` into async wrappers that move the blocking work behind `tauri::async_runtime::spawn_blocking`
- keep the command names and response shapes unchanged

#### 2. Other performance-sensitive subpaths that should not be churned casually

- `list_repo_data_csvs` can scan large trees and optionally compute per-file identity metadata:
  - `src-tauri/src/commands_data.rs:331-431`
- live-bar handling emits a Tauri event and feeds the live execution runner on each accepted bar:
  - `src-tauri/src/commands_data.rs:757-797`

These are not benchmark-gated in the same way as app-core kernels, but they are still sensitive enough that “cleanup” changes should stay narrow.

## Recommended Action

Recommended bucket: `Small surgical refactor`

### Safe-now changes

- fix `list_sample_csvs` so it resolves sample data from the current repo layout instead of `CARGO_MANIFEST_DIR/../..` plus `src/data`
- convert `load_price_data` and `resample_price_data` to async `spawn_blocking` wrappers while keeping the public command names and response contracts unchanged
- reject lossy live-symbol rewrites in `normalize_live_stock_symbol`
- trim `csv_path` in `polygon_validate_dataset` and `polygon_repair_dataset`
- add one wrapper-level test for the sample-data root behavior and one strict-invalid-symbol test

### Document-only or cross-layer follow-up

- align `web/src/tauri/instrumentation.ts` with Rust’s real `polygon_live_bar` timestamp field names
- document that the current task-center live-bar timestamps are receipt-time until that frontend fix lands

### Larger refactor but defer

- splitting this module by concern is reasonable, but not the smallest justified move right now

### Benchmark-sensitive or semantics-sensitive changes to avoid casually

- do not rename live event channels or live status vocabulary without coordinated frontend updates
- do not change `polygon_live_bar` payload field names casually
- do not alter background download guard/cancel semantics without preserving cooperative cancellation inside `polygon.rs`
- do not change repo CSV enumeration defaults without checking `metaService` consumers

## Cross-File Coordination

The main coordination surfaces for this file are:

- `src-tauri/src/state.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- `web/src/services/dataService.ts`
- `web/src/services/executionService.ts`
- `web/src/services/metaService.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/stores/taskStore.ts`

Practical implication:

- path, timestamp, live-stream, or background-task changes are not single-file changes in practice
- some fixes are backend-local, but several require a paired frontend update or verification pass

## Code-Quality and Debt Register

High-value quality improvements:

- make path normalization consistent across dataset-maintenance commands
- unify heavy-command async wrapper style
- centralize strict live-symbol validation instead of allowing per-command normalization drift

Debt that is safe to reduce now:

- stale sample-data lookup assumptions
- inconsistent trimming/normalization
- heavy synchronous Tauri command posture

Debt that should be deferred:

- full module split by concern
- Windows-dialog redesign or cross-platform replacement without a broader product decision

## Technical-Debt Reduction Staging

`Phase 1`

- fix `list_sample_csvs`
- reject lossy live-symbol rewriting
- trim `csv_path` consistently in maintenance wrappers

`Phase 2`

- move `load_price_data` and `resample_price_data` behind async `spawn_blocking`
- align frontend timestamp handling for `polygon_live_bar`
- decide whether top-off should receive tracked background-task cancellation parity

`Phase 3`

- split the file by concern if the mixed scope still slows maintenance after the local fixes land

## Performance Opportunity Register

The real performance opportunities here are structural, not algorithmic.

Useful opportunities:

- offload `load_price_data` and `resample_price_data`
- measure repo scan cost for `list_repo_data_csvs`, especially with `include_identity=true`
- keep the per-bar live path narrow and predictable

Guardrails:

- this is not a CUDA optimization target
- do not invent “perf cleanup” in the Windows-dialog or catalog sections
- any repo-scan default changes should be measured against `metaService` usage, not guessed

## Implementation Staging

`Safe now`

- sample-data root fix
- strict symbol validation
- trim consistency
- wrapper-level tests for the real failure modes

`Coordination required`

- `polygon_live_bar` timestamp-field alignment with frontend instrumentation
- background-task cancellation parity decisions
- command async/sync contract changes with frontend verification

`Defer`

- module split by concern
- dialog-surface redesign

## Change Blast Radius

- `Local only`
  - comments
  - helper tests
  - sample-root logic
- `Same subsystem`
  - dataset loading/resample wrappers
  - market-data-root helpers
  - Polygon maintenance wrappers
- `Cross backend/frontend contract`
  - data workspace load/resample UX
  - Polygon maintenance flows
  - live-stream lifecycle
  - live runner integration
  - repo CSV discovery for campaigns/meta
  - task-center event rendering

## Observability Gaps

Notable gaps:

- no explicit scan timing or truncation diagnostics for repo CSV enumeration
- no sample-data lookup diagnostics when the expected curated files are absent
- no first-class live-stream transition trace beyond event text
- no clear background-task cancellation visibility for top-off vs download parity
- no explicit visibility of which timestamp field the frontend is using for live bars

## Open Questions and Assumptions

- Assumption:
  - top-level repo `data/` is the intended current sample-data root
- Open question:
  - should sample-data discovery remain repo-root-only or become configurable?
- Open question:
  - should `polygon_live_bar` canonicalize around candle-open time, candle-close time, or explicit dual-field frontend handling?
- Open question:
  - are the custom Windows dialogs still an intentional long-term platform choice, or just a temporary demo/runtime constraint?

## Exact Local Verification Required If Changed

If this file changes, the minimum verification should be:

- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test settings_tests -- --nocapture`
- `npm --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/services/executionService.test.ts src/tauri/instrumentation.test.ts src/stores/taskStoreEventThrottle.test.ts src/services/movingAverageCatalogService.test.ts src/services/dynamicIndicatorCatalogService.test.ts src/services/gpuSpeedupsService.test.ts src/services/metaService.test.ts`

If live stream or event payload behavior changes, add a manual desktop smoke pass for:

- start live candles
- verify `polygon_live_status` transitions in the execution workspace
- verify accepted live bars still reach the task center and live runner path

## Final Assessment

`commands_data.rs` is professionally useful already because it keeps most deep ownership in `state.rs`, `polygon.rs`, and `settings.rs`, and it has stronger local tests than a typical Tauri wrapper file.

It is not ready to accept as-is, because the current checkout contains at least one real local correctness bug (`list_sample_csvs`), one meaningful runtime-ownership issue (heavy synchronous commands on the Tauri main-thread boundary), and one semantics-sensitive symbol-validation defect. Those are all small enough to fix surgically without narrowing features or destabilizing the broader backend.
