# Codex implementation prompt — bundle 15 live execution / Polygon streaming / runtime-control responsiveness

You are implementing a **real code change**, not writing another review.

Your task is to **fully implement the live-execution / Polygon-streaming / runtime-control improvements** described in these guidance documents:

1. `gpt54_feature_review_map.md` — **section 15**: `## 15. Live execution, Polygon streaming, and runtime-control responsiveness`
2. `live_execution_polygon_runtime_control_improvement_plan.md`
3. `live_execution_polygon_runtime_control_addendum.md`

If these docs are attached rather than present in the repo, treat the attached copies as authoritative. If they are present in the workspace, read them before editing.

## Precedence and decision rules

Use this precedence order when resolving details:

1. `live_execution_polygon_runtime_control_addendum.md` (supplements/corrects the original plan)
2. `live_execution_polygon_runtime_control_improvement_plan.md`
3. section 15 of `gpt54_feature_review_map.md`

If an exact code shape from the plan does not fit the repo as-is, implement the **smallest functionally equivalent, parity-safe version** that preserves the intended behavior.

## Primary goal

Improve **performance, optimization, and correctness** on the bundle-15 surface **without removing features or breaking functionality**.

Specifically:

- make stop/restart behavior authoritative enough for real user control
- reduce avoidable broker and runtime-control latency
- eliminate state drift between commands, events, and actual runtime state
- keep stop/status controls responsive while other actions are in flight
- tighten symbol/dataset correctness and live-stream/runner coupling
- preserve current feature surface and current trading workflow semantics

## Hard scope guard

Stay tightly within the bundle-15 surface. Do **not** drift into:

- new trading features
- new strategy logic
- new UI panels or workflow surfaces
- broad architecture rewrites
- speculative broker-feature expansion
- optimizer/GPU redesign
- broad Polygon cache/download work beyond what is directly needed for this live-stream/runtime bundle

Prefer **small, parity-safe, incremental changes**.

## Files to modify in the repo

### Primary implementation files

Modify these files as needed:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/executionService.ts`
- `web/src/services/runControlService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/tauri/events.ts` (only if needed; optional helper is fine)
- `web/src/services/executionService.test.ts`
- `web/src/services/runControlService.test.ts`

### Adjacent files you are allowed and expected to inspect / modify if required

These are likely needed to finish the implementation cleanly:

- `src-tauri/src/state.rs`
  - for dataset metadata lookup (`data_id -> dataset symbol`)
  - for live stream task registration / replacement semantics
  - for clearing finished websocket-task runtime info
  - for potential `upsert_live_bar(...)` / `get_candles(...)` adjacency
- any Tauri command-registration file if a changed signature or new command must be registered
- the existing execution workspace bootstrap / mount path if runtime event subscriptions need to be initialized from a component or boot file rather than lazily from the store alone
- a new frontend test file such as `web/src/stores/executionStore.test.ts` if you need store-level tests

### Read-only / reference files

Read these, but do **not** spend time rewriting them unless you need a tiny supporting change:

- `src-tauri/src/polygon.rs`
  - use it as a reference for HTTP client builder / timeout / retry posture
  - do not perform a broad rewrite

## What “done” means

The task is only done when:

1. the bundle-15 behavior changes are implemented in code
2. impacted tests are updated
3. new tests are added for the newly enforced correctness/latency behavior
4. any required adjacent state/registration/bootstrap changes are made
5. the final result stays parity-safe and does not fake runtime truth

---

# Mandatory implementation work

Implement **all required / high-priority items** from the original plan **plus** the high-priority additions from the addendum.

## A. Backend runner correctness and stop authority

### Files

- `src-tauri/src/commands_execution.rs`
- possibly `src-tauri/src/state.rs`

### Required changes

1. **Make runner stop/restart authoritative.**
   - Add a runner generation and cancellation/abort handle.
   - Do not rely on symbol-only matching to detect stale work.
   - A stop or same-symbol restart must prevent stale in-flight work from mutating the current runner state.

2. **Cancel prior runtime on replace.**
   - Starting a new runner must cancel the previous one before replacement.

3. **Cancel before stop removal.**
   - `stop_live_execution_runner(...)` must cancel first, then remove/take the runtime.

4. **Add generation-safe completion and stale-task protection.**
   - Any spawned live-bar execution task must verify it still belongs to the active generation before mutating runtime state or emitting completion events.
   - This must fix the “stop + restart same symbol, stale task writes into new runtime” problem.

5. **Add cancellation checks before expensive stages and before broker submit.**
   - The stop path must become authoritative **before the submit boundary**.
   - It is acceptable that an already-sent broker POST cannot be cancelled once on the wire, but you must narrow the remaining window with pre-submit checks and timeouts.

6. **Add one-slot latest-bar coalescing.**
   - When a new live bar arrives while the runner is in flight, do not just drop everything.
   - Keep a single “latest pending bar” and process it after the current run finishes.
   - Preserve the ability to count skipped/coalesced behavior, but avoid unbounded queueing.

7. **Keep the implementation parity-safe.**
   - Do not redesign the live signal engine.
   - Do not implement a full incremental evaluator in this pass.

### Strongly suggested code shape

A functionally equivalent implementation is acceptable, but the intent should match the plan:

- add a runner generation atomic (for example `AtomicU64`)
- add a cancellation handle (for example `Arc<AtomicBool>`)
- add `pending_bar: Option<LiveExecutionRunnerBar>` to the runtime
- use generation + cancel checks in `on_live_execution_bar(...)` and completion paths

If you reuse different names, keep the semantics identical.

## B. Backend dataset/symbol correctness guard

### Files

- `src-tauri/src/commands_execution.rs`
- likely `src-tauri/src/state.rs`
- optionally frontend preflight in `web/src/services/executionService.ts`

### Required changes

1. **Add an authoritative `symbol <-> data_id` guard in the backend.**
   - `execute_strategy_signal_once(...)` must reject cases where the requested broker/live symbol does not match the loaded dataset symbol behind the referenced `strategy.data_id`.
   - `start_live_execution_runner(...)` must do the same.

2. **Expose the minimal adjacent state API needed to look up dataset symbol by `data_id`.**
   - If `AppState` already tracks this metadata, reuse it.
   - If not, add the smallest possible API to surface it.

3. **Keep the backend guard authoritative even if you add a frontend preflight.**
   - Frontend preflight is optional/strongly recommended.
   - Backend enforcement is mandatory.

### Acceptable adjacent API shape

A functionally equivalent API is fine, but a shape like this is desirable:

```rust
pub fn dataset_symbol_for_data_id(&self, data_id: &str) -> Result<Option<String>, String>;
```

Then add a helper in `commands_execution.rs` to compare canonicalized symbol forms and reject mismatch.

## C. Canonical symbol handling (preserve meaningful punctuation)

### Files

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts` (if store-level comparisons are used)
- tests

### Required changes

1. Replace punctuation-stripping symbol normalization with a **canonical symbol helper** that:
   - trims
   - removes whitespace
   - uppercases
   - **preserves meaningful punctuation** such as `.`, `-`, `_`, `/`, `:`

2. Use the same canonicalization semantics across:
   - backend runner/live-symbol comparisons
   - backend snapshot symbol filtering helpers
   - frontend live/runner symbol checks
   - store availability checks

3. Add tests that specifically cover punctuation-bearing symbols.
   - Example: `BRK.B` must **not** collide with `BRKB`.

### Important note

The goal is **not** to broaden symbol support beyond current product scope. The goal is to stop silently collapsing distinct valid identifiers.

## D. Shared timed Alpaca HTTP client

### Files

- `src-tauri/src/commands_execution.rs`

### Required changes

1. Replace repeated `reqwest::blocking::Client::new()` usage with a shared client helper.
2. Use explicit timeouts and connection reuse.
3. Reuse the helper across:
   - one-shot execution
   - account test
   - broker snapshot refresh

### Desired posture

Mirror the general posture already used in `src-tauri/src/polygon.rs`:

- builder-based client creation
- explicit user agent
- explicit connect timeout
- explicit request timeout
- pooling/idle reuse where appropriate

A builder configuration approximately like this is acceptable if it fits the repo:

- connect timeout around 3s
- total request timeout around 12s
- idle pool timeout around 90s

## E. Slim snapshot refresh and symbol-filter correctness

### Files

- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`
- tests

### Required changes

1. Extend the live snapshot request shape with **include flags** so the common orders-refresh path can skip unneeded account work.
   - `include_account`
   - `include_orders`
   - `include_position`

2. Use the single-symbol position endpoint when a symbol is present.
   - Do not always fetch all positions if you only need one symbol.

3. Update frontend `refreshOrders()` to request only what it actually needs.
   - It should not pay for account status if the UI is only showing orders + position.

4. Tighten symbol-filtered order refresh correctness.
   - The current pattern fetches a global recent-order list and then filters client-side by symbol.
   - Do not leave a silent under-return hazard where the desired symbol’s rows may be truncated before filtering.

### Minimum acceptable implementation for order-filter correctness

Implement at least one safe improvement:

- increase scan depth when symbol filtering is active, and/or
- separate scan limit from return limit, and/or
- iterate pages until enough matching rows are found or a hard cap is reached, and/or
- use server-side symbol filtering if the current broker path supports it cleanly

Pick the smallest safe option that fits the existing code. Do **not** leave the issue untouched.

## F. Live-stream lifecycle truthfulness and coupling to runner

### Files

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/commands_execution.rs`
- likely `src-tauri/src/state.rs`
- frontend service/store/tests

### Required changes

1. Emit explicit live-status events for **manual start** and **manual stop**.
   - `started`
   - `stopped`

2. Couple the runner to **manual** live-stream lifecycle transitions:
   - manual live stop should also stop the runner
   - manual replacement of the live stream with a different symbol should also stop the runner

3. Do **not** blindly broaden this into a new lifecycle system.
   - For unexpected websocket `error` / `closed`, it is acceptable in the first pass to keep the runner configured but surface the degraded state truthfully via events/store.
   - Manual stop / manual replacement is the mandatory coupling path.

4. Verify adjacent `state.rs` behavior.
   - Ensure finished/aborted websocket tasks do not leave `polygon_live_candle_stream_status()` returning `active=true` forever.
   - If needed, fix the cleanup in the adjacent state layer.

## G. Websocket startup robustness

### Files

- `src-tauri/src/commands_data.rs`
- likely `src-tauri/src/state.rs`
- tests if feasible

### Required changes

1. Add explicit timeouts around websocket startup stages:
   - `connect_async(...)`
   - auth send
   - subscribe send

2. Abort the spawned websocket task if registration into state fails after spawn.
   - The current command spawns first and registers second.
   - If registration fails, do not leave an orphan background task running.

### Preferred implementation approach

If needed, make the smallest adjacent state API change that returns the task back on failure so the command path can abort it explicitly.

A shape like this is acceptable if it fits cleanly:

```rust
pub fn replace_live_candle_stream(
    &self,
    task: LiveCandleStreamTask,
) -> Result<Option<LiveCandleStreamInfo>, (String, LiveCandleStreamTask)>;
```

A functionally equivalent abort-on-drop guard is also acceptable if it is simpler in the actual repo.

## H. Frontend execution service improvements

### Files

- `web/src/services/executionService.ts`
- tests

### Required changes

1. Parse richer live-stream status metadata.
   - Preserve `stream.symbol`, `stream.delayed`, etc.
   - Do not treat live status as just a boolean.

2. Add a frontend live-stream symbol guard before starting the runner.
   - If live candles are running for `MSFT`, starting a runner for `AAPL` must reject clearly before invoke.
   - This is a fast preflight; backend still remains authoritative.

3. Add a runtime event subscription helper.
   - Subscribe to `polygon_live_status`
   - Subscribe to `live_execution_runner_event`
   - Aggregate cleanup/unsubscribe

4. Update `refreshOrders()` to use the slim snapshot flags.

5. Remove the fake runtime reset from `clearKeys()`.
   - Clearing keys must clear credentials and key-related output only.
   - It must **not** pretend that live candles or the runner are off.

6. Remove the immediate post-submit broker refresh from `runOnce()`.
   - Do not block success on a second broker round-trip.
   - Preserve explicit manual refresh for authoritative broker state.

7. Add optimistic order-row behavior for immediate feedback after order submit.
   - Either from `runOnce()` success or runtime events (or both)
   - Keep it bounded and clearly non-authoritative

### Important constraint

Do not turn `runOnce()` into a new background orchestration flow. Keep the change simple and parity-safe.

## I. Frontend execution store refactor for responsiveness and truthfulness

### Files

- `web/src/stores/executionStore.ts`
- possibly a bootstrap/mount file if subscriptions must be initialized there
- tests

### Required changes

1. Replace the single global `isBusy` lock with **per-action pending state**.
   - `refreshStatuses`
   - `testConnection`
   - `runOnce`
   - `refreshOrders`
   - `startLive`
   - `stopLive`
   - `startRunner`
   - `stopRunner`
   - `clearKeys`

2. Derive control availability from per-action pending state rather than a single all-or-nothing busy gate.

3. Keep these actions responsive even while another action is running:
   - `stopLive`
   - `stopRunner`
   - `refreshStatuses`

4. Track explicit live-stream identity in store state.
   - add `liveSymbol`
   - add `liveDelayed`

5. Tighten `canStartRunner` so it requires:
   - no conflicting runner action in flight
   - `liveActive === true`
   - a non-empty `liveSymbol`
   - `liveSymbol` matches the requested execution symbol under canonical symbol comparison

6. Wire runtime events into the store.
   - `polygon_live_status` updates live flags / hints / live symbol
   - `live_execution_runner_event` updates runner flags / hints / optimistic submitted rows

7. Ensure subscriptions exist before the surface is used.
   - Preferred: use the existing execution workspace/bootstrap path if one already exists.
   - Acceptable fallback: lazily ensure subscriptions from the store/service before status/control actions.

8. Keep runtime truth separate from credential state.
   - `clearKeys()` must not set `liveActive=false` / `runnerActive=false` unless the runtime actually changed.

### Compatibility note

If another part of the app still expects an `isBusy` field, it is acceptable to keep it as a **derived or compatibility field**. But availability logic must no longer behave as one giant global lock.

## J. Runtime-control cancellation responsiveness

### Files

- `web/src/services/runControlService.ts`
- `web/src/services/runControlService.test.ts`

### Required changes

1. Preserve the existing primary/legacy cancellation behavior:
   - `cancel_current_run`
   - fallback to `cancel_double_ma_optimization`

2. Start the background cancellation probe in parallel rather than serially waiting for it.
   - `cancel_background_task` should no longer wait behind slow rejection of the primary path.

3. Update tests so they do **not** depend on strict nth-call ordering if cancellation is now intentionally concurrent.

### Optional small safety improvement

If the repo already has evidence of hanging invokes, you may add a tiny timeout wrapper. Do not invent that if it would broaden scope unnecessarily.

## K. Event helper changes (only if needed)

### Files

- `web/src/tauri/events.ts`

### Required changes

- None if `executionService.ts` handles aggregation cleanly.

### Optional change

- Add a small `listenEvents(...)` bulk helper for multi-subscription teardown if it simplifies the implementation.

Do not spend time on this file unless it meaningfully reduces implementation risk or complexity.

---

# Optional second-pass improvements (implement only after all mandatory items are done, and only if they remain small and safe)

These are good extras, but they must not derail the main implementation.

1. **Combined runtime-status command**
   - single backend command returning live + runner status together
   - reduces dual-invoke round trips and snapshot skew

2. **Merge `upsert_live_bar(...)` and `get_candles(...)` if state internals make it easy**
   - remove one state lookup / lock boundary on the live-bar path

3. **Deterministic `client_order_id` for broker submits**
   - useful correctness hardening against ambiguous retries / near-boundary restarts

4. **Classify terminal runner errors and support suspended health state**
   - optional second-pass truthfulness improvement

5. **Stricter live activity semantics**
   - distinguish `requested` vs `connected/subscribed/streaming` only if it is easy and low-risk

6. **Reconnect/backoff for websocket loop**
   - only after first-pass fixes are stable and measured
   - do not start here

If you implement any optional item, keep the diff small, add tests if practical, and clearly note it in the final summary.

---

# Concrete repository guidance by file

## `src-tauri/src/commands_execution.rs`

Implement at least the following:

- shared timed Alpaca client helper
- runner generation + cancel handle
- cancel-before-stop and cancel-before-replace
- cancellation checks across blocking execution stages and before broker submit
- pending latest-bar coalescing
- generation-safe completion updates
- dataset-symbol guard using adjacent state metadata
- punctuation-preserving canonical symbol helper / symbol matching
- snapshot include flags
- single-symbol position fetch
- improved symbol-filtered order refresh behavior
- helper tests where feasible

If function signatures must change to carry cancellation state, update all call sites.

## `src-tauri/src/commands_data.rs`

Implement at least the following:

- explicit live `started` event in `polygon_start_live_candle_stream(...)`
- explicit live `stopped` event in `polygon_stop_live_candle_stream(...)`
- manual live stop also stops runner
- manual symbol-changing live replacement also stops runner
- websocket connect/auth/subscribe timeouts
- abort spawned websocket task on registration failure
- verify/fix interaction with adjacent state cleanup

## `src-tauri/src/state.rs` (adjacent)

Modify only as needed to support the implementation cleanly:

- expose dataset symbol by `data_id`
- return enough ownership/context on live-stream registration failure to abort spawned tasks safely
- ensure finished live-stream tasks do not leave stale “active” state
- optionally provide an `upsert_live_bar_and_get_candles(...)` helper if it is a trivial improvement

Keep these changes narrow.

## `src-tauri/src/polygon.rs`

Do not do a broad rewrite. Use it as a style/reference file for client builder posture. Only touch it if there is a tiny, clearly justified supporting change.

## `web/src/services/executionService.ts`

Implement at least the following:

- richer live-status parsing
- canonical symbol helper on frontend side
- live-stream symbol preflight in `startRunner()`
- slim snapshot request flags in `refreshOrders()`
- runtime event subscription helper
- truthful `clearKeys()`
- no immediate broker auto-refresh in `runOnce()`
- bounded optimistic order-row path

## `web/src/stores/executionStore.ts`

Implement at least the following:

- per-action pending state
- availability logic derived from pending state and live/runner identity
- `liveSymbol` + `liveDelayed`
- event-driven state reconciliation from runtime events
- responsiveness for stop/status actions while another action is in progress
- truthful separation of runtime state from credential-clearing behavior

## `web/src/services/runControlService.ts`

Implement the primary/legacy cancel path with overlapping background cancel probe, while preserving current semantics when commands are unavailable.

## `web/src/tauri/events.ts`

Only add an optional helper if it makes event subscription teardown simpler. Otherwise leave it alone.

---

# Testing requirements

Update existing tests and add new ones as needed. Do not leave behavior changes untested.

## Required frontend test work

### `web/src/services/executionService.test.ts`

Add/update tests for at least:

1. **Slim snapshot request**
   - `refreshOrders()` sends the new include flags / lean request shape
   - if you changed scan-vs-return semantics, assert the expected request

2. **Runner/live symbol mismatch rejection**
   - `startRunner()` rejects when live stream is active for a different symbol

3. **`runOnce()` no longer immediately re-fetches broker snapshot**
   - success path returns promptly and uses optimistic behavior instead of automatic `refreshOrders()`

4. **`clearKeys()` stays truthful**
   - it clears credentials / output but does not fake runtime shutdown

5. If you add runtime event helpers with visible behavior, add targeted tests where practical.

### `web/src/services/runControlService.test.ts`

Update tests so they validate semantics without depending on strict call order when background cancellation is concurrent.

Cover at least:

1. primary + background cancel both requested when available
2. legacy fallback still works
3. total failure still throws

### New store-level tests

Add a new store test file if needed, for example `web/src/stores/executionStore.test.ts`.

Cover at least:

1. stop/status actions remain available while another action is pending
2. `canStartRunner` requires matching live symbol
3. `polygon_live_status` event updates `liveActive`, `liveSymbol`, and hints
4. `live_execution_runner_event` updates `runnerActive`, hints, and optimistic rows as expected

## Required Rust test work

Add targeted unit/helper tests where feasible in `src-tauri/src/commands_execution.rs` and/or adjacent state tests.

Cover at least:

1. punctuation-preserving symbol matching / canonicalization
2. dataset-symbol mismatch rejection helper logic if unit-testable
3. any small deterministic helper you introduce for pending-bar coalescing or cancellation gating

Do not force huge async integration tests if they are awkward in this repo; prefer small deterministic helper tests plus frontend behavior tests.

---

# Performance / correctness acceptance criteria

Your implementation should satisfy all of these:

1. **Stop authority**
   - stopping the runner cancels future work and prevents stale tasks from mutating current runtime state
   - same-symbol restart cannot be corrupted by stale completion from the old generation

2. **Runtime control responsiveness**
   - `stopLive`, `stopRunner`, and `refreshStatuses` remain usable while another execution action is running

3. **Live/runner correctness**
   - runner cannot be started for a symbol that does not match the current live stream
   - manual live stop or manual live symbol replacement also stops the runner

4. **Dataset correctness**
   - backend rejects `symbol` / `strategy.data_id` mismatch

5. **Snapshot latency reduction**
   - the common order refresh path no longer always fetches unnecessary account data
   - single-symbol position fetch is used when appropriate

6. **Runtime truthfulness**
   - `clearKeys()` no longer lies about live or runner state
   - event-driven store state reflects runtime changes without requiring manual refresh for everything

7. **Order refresh correctness**
   - symbol-filtered order refresh is less vulnerable to truncation-before-filtering

8. **Websocket startup robustness**
   - connect/auth/subscribe stages are timeout-bounded
   - failed task registration does not leak an orphan websocket task

---

# Implementation order

Use this order unless repo realities force a small adjustment:

## Phase 1 — backend correctness first

1. runner generation + cancellation + stale-task protection
2. pending latest-bar coalescing
3. dataset-symbol guard
4. canonical symbol fix

## Phase 2 — live-stream lifecycle truthfulness

1. explicit started/stopped live-status events
2. manual live stop / replacement -> runner stop
3. websocket registration-failure cleanup
4. websocket startup timeouts
5. adjacent `state.rs` cleanup verification/fix

## Phase 3 — broker latency tightening

1. shared timed Alpaca client
2. lean snapshot request flags
3. single-symbol position fetch
4. improved symbol-filtered order refresh behavior

## Phase 4 — frontend responsiveness and truthfulness

1. executionService live status parsing and runner preflight
2. remove `runOnce()` auto-refresh and add optimistic feedback
3. truthful `clearKeys()`
4. executionStore per-action pending state
5. live symbol tracking + availability tightening
6. runtime event subscriptions and event-driven state updates

## Phase 5 — cancellation service and tests

1. parallel best-effort background cancellation in `runControlService.ts`
2. update service tests
3. add store tests
4. add Rust helper tests

## Phase 6 — optional safe extras only if everything above is complete and clean

- combined runtime status command
- `client_order_id`
- `upsert_live_bar_and_get_candles(...)`
- suspended runner health state

---

# Important implementation nuances

1. **Do not broaden `polygon.rs` scope.**
   Use it as a reference for timeout/retry posture only.

2. **Do not build a new event system.**
   Use the existing Tauri event surface already present in the repo.

3. **Do not create a new UI flow.**
   Work within the current execution service/store/runtime-control surface.

4. **Do not hide difficult behavior with fake state.**
   Runtime truthfulness matters more than cosmetically “clean” hints.

5. **Do not turn optional items into a large rewrite.**
   If an optional item starts expanding scope, stop and defer it.

6. **If you add or change command signatures, update registrations/imports/call sites.**
   Search the repo and fix compile errors rather than leaving a partial change.

7. **If there is already an existing subscription-init location in the app shell/workspace, reuse it.**
   Do not duplicate listeners unnecessarily.

8. **If another file already implements part of the required behavior, adapt to that reality instead of forcing the exact pseudocode from the plan.**
   The functional outcome matters more than literal naming.

---

# Final response format

After implementing, respond with:

1. **Summary of what you changed**
   - grouped by backend, frontend service, store, cancellation, tests

2. **Files changed**
   - include every edited file path

3. **Mandatory items completed**
   - checklist form

4. **Optional items completed or deferred**
   - state clearly what you did and why

5. **Tests run**
   - list commands/run targets and outcomes
   - if any tests could not be run, say exactly why

6. **Known follow-ups (only if truly deferred)**
   - keep this short and specific

Do not return only a plan. Make the code changes.
