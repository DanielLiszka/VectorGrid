# Review Packet: `src-tauri/src/state.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/state.rs`
- Packet status: `Reviewed`
- Public-review readiness: `High-leverage runtime state hub with solid test coverage, but not yet accept-as-is because timestamp normalization has a real historical-data edge case and several mutation/reuse contracts remain implicit`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Runtime-state-sensitive`, `Live-data-sensitive`, `Cache-identity-sensitive`, `Semantics-sensitive`

## What This File Owns

`state.rs` owns the desktop backend's in-memory runtime state for loaded candle datasets, derived-resample residency, live-bar mutation, single-run exclusivity, and background/live-stream task state.

This file owns:

- the core app-managed state container:
  - `AppState`: `src-tauri/src/state.rs:120-129`
- dataset load, normalization, and in-memory base reuse:
  - `load_price_data`: `209-288`
  - `find_matching_loaded_base`: `720-772`
- derived resample cache, versioning, and invalidation:
  - `resample_loaded_data`: `290-405`
  - `invalidate_dataset`: adjacent runtime state helpers in the same module
- live-bar upsert into resident datasets:
  - `upsert_live_bar`: `432-557`
- single-run gate used by long-running command flows:
  - `try_begin_run`, `cancel_token`, `request_cancel`: `559-575`
  - `RunGuard`: `162-169`
- live candle stream slot and observable status:
  - `replace_live_candle_stream`: `577-607`
  - `stop_live_candle_stream`: `609-637`
  - `live_candle_stream_info`: `639-692`
  - `record_live_candle_stream_status`: `694-717`
- dataset retention and dependency-aware eviction:
  - `descendant_ids_locked`: `825-850`
  - `remove_dataset_tree_locked`: `852-864`
  - `expanded_protect_ids_locked`: `866-884`
  - `eviction_candidate_locked`: `886-910`
  - `enforce_dataset_retention_locked`: `912-952`
- background task exclusivity for non-run backend jobs:
  - `BackgroundTaskState`: `1413-1455`
- timestamp normalization heuristics for loaded data:
  - `timestamp_uses_millis`: `1508-1510`
  - `timestamps_look_like_seconds`: `1512-1521`
  - `normalize_timestamps_to_millis`: `1523-1533`
  - `infer_timeframe_label`: `1535-1581`

This file does not own:

- the Tauri command surface that exposes these operations
  - those live across `src-tauri/src/commands_*.rs`
- the durable on-disk cache build/load policy
  - that lives in `src-tauri/src/data_cache.rs`
- optimization, validation, or execution algorithms
  - those live in `src-tauri/src/commands_*.rs` and `crates/app_core/src/**`

## Classification

- Public surface: internal/backend-only, but effectively canonical runtime surface
- Hot path: moderate
  - load, resample, and live-bar paths are user-visible and repeatedly exercised
- Runtime-critical: high
  - this file controls whether datasets are reused, invalidated, or mutated correctly
- CUDA/runtime-critical: indirectly yes
  - CUDA and validation flows depend on the run guard and resident data being correct
- Strategy-local: no
- Low-risk support code: no
  - this is shared runtime coordination state

## Direct Interactions

### Backend interaction map

- command-layer load/resample/clear entry points:
  - `src-tauri/src/commands_data.rs:21-28`
  - `src-tauri/src/commands_data.rs:33-37`
  - `src-tauri/src/commands_data.rs:49-60`
  - `src-tauri/src/commands_data.rs:1220-1225`
- live Polygon stream startup depends on this file's live-stream slots and dataset residency:
  - `src-tauri/src/commands_data.rs:1004-1014`
- live execution mutates the active dataset through `upsert_live_bar` and treats only `Err` as failure:
  - `src-tauri/src/commands_execution.rs:1932-1947`
- options-expression runtime uses the single-run gate:
  - `src-tauri/src/commands_options_expression.rs:1933-1945`
- other long-running command families rely on the same exclusivity contract:
  - `src-tauri/src/commands_optimize.rs:89`, `116`, `272`
  - `src-tauri/src/commands_validation.rs:347`, `584`
  - `src-tauri/src/commands_ml_signal.rs:100`, `116`, `132`, `150`
  - `src-tauri/src/commands_meta.rs:1881`, `1893`, `1905`, `1938`
  - `src-tauri/src/commands_campaign.rs:3752`
- data identity and load/build behavior are delegated to the disk-cache layer:
  - `src-tauri/src/data_cache.rs`

### Downstream verification impact if changed

- any change to base reuse or timestamp normalization requires sanity checks in:
  - `commands_data`
  - `commands_execution`
  - any flow that depends on `get_candles` and `get_loaded_data_context`
- any change to run/background guards requires sanity checks in:
  - optimize
  - validation
  - ML signal
  - options expression
  - campaign/meta runs

### Test anchors

- normalization and residency behavior:
  - `src-tauri/src/state.rs:1030-1271`
- retention behavior:
  - `src-tauri/src/state.rs:1272-1401`
- live-stream state:
  - `src-tauri/src/state.rs:1597-1674`
- run/background guards:
  - `src-tauri/src/state.rs:1681-1719`

## Standards And Research Log

This packet was written after targeted external review focused on Tauri managed state expectations, canonical path identity, clone-on-write mutation semantics, and atomic-ordering constraints before proposing any state-layer changes.

### Sources consulted

1. Tauri state management
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why it was consulted:
     - this file is the repo's central managed runtime state for commands and background tasks
   - Standard informed:
     - shared app-managed state should make ownership and cross-command mutation boundaries explicit

2. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file sits directly behind command-layer calls and its error strings cross the IPC boundary
   - Standard informed:
     - command-facing state helpers should preserve actionable errors and avoid ambiguous silent no-ops

3. `std::fs::canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why it was consulted:
     - base dataset reuse currently depends on raw path-string equality
   - Standard informed:
     - stable path identity should prefer normalized absolute form when aliasing would otherwise duplicate state

4. `Arc::make_mut`
   - Source: <https://doc.rust-lang.org/std/sync/struct.Arc.html#method.make_mut>
   - Why it was consulted:
     - live-bar writes mutate resident datasets through clone-on-write semantics
   - Standard informed:
     - `Arc::make_mut` is the correct local tool here, and changes to this mutation model should preserve clone-on-write guarantees

5. `std::sync::atomic::Ordering`
   - Source: <https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html>
   - Why it was consulted:
     - run/background guards use `AtomicBool` with `Ordering::SeqCst`
   - Standard informed:
     - any attempt to relax ordering is concurrency-sensitive and should not be treated as a casual cleanup

6. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines reusable backend runtime APIs consumed across many command modules
   - Standard informed:
     - internal shared APIs should keep invariants and outcomes explicit, especially around mutation and reuse behavior

## Findings

## Correctness And Safety

### 1. Pre-1980 millisecond timestamps can be misclassified as seconds and multiplied by `1_000`

- Evidence:
  - timestamp heuristics define `1980-01-01` as the cut line:
    - `src-tauri/src/state.rs:1474-1475`
  - values with `max_abs < MIN_EPOCH_MILLIS` but `max_abs >= MIN_EPOCH_SECONDS` are treated as seconds:
    - `1512-1521`
  - normalization then rewrites the full series in place:
    - `1523-1533`
- Why it matters:
  - this is an objective heuristic bug for valid historical millisecond data before 1980
  - the current product may mostly use modern equities data, but the heuristic itself is broader than that product assumption
- Smallest justified fix:
  - tighten `timestamps_look_like_seconds` so pre-1980 millisecond data is not classified as seconds solely on magnitude
  - keep delta-based detection, but require stronger evidence before rewriting
  - add a regression test with legitimate pre-1980 millisecond timestamps

### 2. `upsert_live_bar` silently drops unseen out-of-order bars

- Evidence:
  - if the incoming bar is older than the current tail and no exact timestamp match is found, `idx` becomes `usize::MAX`:
    - `src-tauri/src/state.rs:509-518`
  - that path returns `mutated = false`, does not invalidate descendants, and still returns `Ok(())`:
    - `539-556`
  - the live execution runner reacts only to `Err`:
    - `src-tauri/src/commands_execution.rs:1932-1947`
- Why it matters:
  - a stale or out-of-order live bar can be dropped with no observable signal at the command/runtime boundary
  - this is not necessarily the wrong policy, but it is an implicit policy with silent behavior
- Smallest justified fix:
  - return an explicit outcome enum or status describing `appended`, `updated_existing`, or `ignored_stale`
  - if the public signature should remain unchanged, at minimum emit an explicit log/diagnostic when a stale unmatched bar is ignored
  - add a regression test covering exact-match overwrite vs unmatched late-bar ignore

## API And Naming

### 1. Base-data reuse is semantically keyed by identity, but the implementation also requires exact raw path equality

- Evidence:
  - reuse requires both `ctx.csv_path == path` and matching `source_identity`:
    - `src-tauri/src/state.rs:736-738`
  - callers pass the path string through from command inputs:
    - `src-tauri/src/commands_data.rs:21-28`
- Why it matters:
  - the effective contract is stricter than the conceptual contract
  - equivalent spellings of the same file on Windows can miss reuse and create duplicate resident datasets
- Smallest justified fix:
  - normalize or canonicalize the stored identity key once during base load
  - preserve the original path string only as display metadata if the UI needs it
- Review note:
  - this is both an API-shape and correctness concern because callers naturally assume file identity, not string-spelling identity

### 2. Mutation outcomes are currently implicit in a `Result<(), String>` API

- Evidence:
  - `upsert_live_bar` has three real outcomes:
    - append
    - in-place overwrite
    - ignored unmatched stale bar
  - only hard validation failures become `Err`
- Why it matters:
  - command callers cannot distinguish a successful mutation from a deliberate no-op
- Smallest justified fix:
  - introduce a small local outcome type if this behavior needs to become externally observable
  - otherwise document the no-op path explicitly in the function contract

## Error Handling

### 1. Lock poisoning and runtime-state failures are flattened into generic string errors

- Evidence:
  - repeated patterns such as:
    - `"failed to lock candles map"`: `src-tauri/src/state.rs:237`, `303`, `364`, `459`, `730`
    - `"failed to lock loaded-data context map"`: `258`, `388`, `734`
    - `"failed to lock live stream state"`: `583`, `613`, `643`
    - `"failed to lock live stream status"`: `690`, `709`
- Why it matters:
  - Tauri commands do need serializable errors, so `String` is normal here
  - however, the current diagnostics lose which operation failed and whether the problem came from poisoning or a specific state transition
- Smallest justified fix:
  - keep `String` at the IPC edge if desired, but enrich the messages with operation context
  - avoid a large custom error refactor unless multiple command files are touched together
- Review judgment:
  - real diagnostics debt, but lower priority than the timestamp and silent-no-op issues

## Module Boundaries And Technical Debt

### 1. This file intentionally centralizes multiple kinds of runtime state, but some contracts should be made more explicit instead of further generalized

- Evidence:
  - one module currently owns:
    - dataset residency
    - derived invalidation/versioning
    - live-stream lifecycle
    - run/background gates
    - timestamp normalization heuristics
- Why it matters:
  - broad decomposition for its own sake would increase cross-file coordination cost
  - the better next step is to document and test the existing boundaries, not to prematurely split the module
- Smallest justified fix:
  - document the dataset identity contract
  - document the live-bar no-op policy
  - document why guard atomics intentionally remain conservative

### 2. Retention behavior is deliberate and reasonably careful; avoid “cleanup” that removes its dependency protections

- Evidence:
  - protected IDs are expanded to include ancestors:
    - `src-tauri/src/state.rs:866-884`
  - eviction prefers unshared `Arc` entries before shared ones:
    - `886-910`
  - retention removes full dataset trees instead of leaving orphans:
    - `852-864`, `912-952`
- Why it matters:
  - this is useful technical debt, not accidental complexity
  - oversimplifying it would risk invalid derived-cache behavior
- Review judgment:
  - keep this design unless profiling or correctness evidence says otherwise

## Docs And Examples

### 1. Several important contracts are only encoded in behavior and tests today

- Missing documentation topics:
  - path identity rules for in-memory base reuse
  - live-bar behavior when a late unmatched bar arrives
  - why run/background guards use conservative atomic ordering
  - timestamp normalization assumptions and supported historical-data expectations

## Tests And Missing Verification

### 1. Existing coverage is strong and currently green

- Verified in this review pass:
  - `cargo test -p ta_desktop_demo_app state_ -- --nocapture`
    - passed `14` tests
- Existing strengths:
  - second-to-millisecond load normalization:
    - `src-tauri/src/state.rs:1030-1059`
  - load/resample invalidation/versioning:
    - `1061-1156`
  - in-memory reuse and access-sequence refresh:
    - `1158-1271`
  - retention limits and dependency protection:
    - `1272-1401`
  - live-stream status lifecycle:
    - `1597-1674`
  - run/background guard semantics:
    - `1681-1719`

### 2. Important edge-case verification is still missing

- Missing tests found in this review pass:
  - legitimate pre-1980 millisecond timestamps should not be multiplied by `1_000`
  - alternate path spellings for the same file should not create duplicate in-memory base datasets if identity normalization is added
  - `upsert_live_bar` should have explicit test coverage for:
    - append-at-tail
    - overwrite-existing timestamp
    - ignore-unmatched-stale-bar

### 3. Exact local verification required if changed

- `cargo test -p ta_desktop_demo_app state_ -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_execution -- --nocapture`
- if run/background guard semantics change:
  - `cargo test -p ta_desktop_demo_app commands_optimize -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_ml_signal -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. No immediate throughput rewrite is justified here

- Evidence:
  - the main objective issues are semantic, not allocation-driven
  - the current use of `Arc::make_mut` is the right local clone-on-write tool for rare live-bar mutation
- Review judgment:
  - do not treat this file as a generic “optimize the state map” target without contention evidence

### 2. Atomic ordering changes would be benchmark-sensitive and correctness-sensitive

- Evidence:
  - both run and background guards use `Ordering::SeqCst`:
    - `src-tauri/src/state.rs:168`, `560`, `563`, `1424`, `1439`, `1442`, `1453`
- Why it matters:
  - relaxed ordering could be safe, but only with a deliberate concurrency proof
- Review judgment:
  - `benchmark-sensitive; do not touch yet` for memory-order tuning

## Code-Quality Improvement Register

- Safe now:
  - tighten the timestamp normalization heuristic
  - make the live-bar stale/no-op outcome explicit
  - normalize dataset identity keys for in-memory base reuse
  - improve lock-failure messages with operation context
- Defer:
  - broad module splitting
  - generic runtime-state abstractions shared across unrelated state domains

## Technical-Debt Reduction Register

- Safe to reduce now:
  - implicit timestamp rewrite assumptions
  - implicit live-bar no-op behavior
  - path-spelling-sensitive reuse logic
- Defer because it spans many files:
  - replacing all stringified runtime errors with a structured cross-command error system
  - large-scale state-module decomposition
- Useful debt that should not be “cleaned up” casually:
  - dependency-aware retention rules
  - conservative guard atomics
  - clone-on-write live-bar mutation

## Performance Opportunity Register

- Safe structural improvement:
  - identity-key normalization can reduce duplicate residency and redundant reload/resample churn
- Benchmark-sensitive or semantics-sensitive:
  - atomic ordering relaxation
  - changing retention candidate selection heuristics
  - changing clone-on-write mutation strategy for resident candles

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - preserve current load, resample, and invalidate behavior
  - preserve live execution support for tail append and exact-timestamp overwrite
  - preserve single-run exclusivity across optimize/validation/ML/options/meta/campaign flows
- Regression risk if changed carelessly:
  - loaded dataset identity can fork unexpectedly
  - derived caches can become stale or orphaned
  - live execution can silently diverge from expected data mutation behavior
  - concurrent run protection can weaken
- Safe change posture:
  - prefer narrow contract-clarifying fixes and regression tests over structural rewrites
