# Review Packet: `src-tauri/src/commands_execution.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_execution.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Important and mostly coherent, but not yet strong enough to accept without follow-up on live-order safety`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Public-surface-sensitive`, `Runtime-critical`, `Semantics-sensitive`, `Broker-integrity-sensitive`

## What This File Owns

`commands_execution.rs` is the Tauri-side live-execution and broker-control boundary for the desktop app.

This file owns:

- public Tauri command DTOs and response shapes for:
  - one-shot signal execution: `src-tauri/src/commands_execution.rs:42-64`, `src-tauri/src/commands_execution.rs:261-279`, `src-tauri/src/commands_execution.rs:1658-1699`
  - auto-runner lifecycle: `src-tauri/src/commands_execution.rs:289-342`, `src-tauri/src/commands_execution.rs:1701-1848`
  - broker review, credential storage, and safety actions: `src-tauri/src/commands_execution.rs:66-122`, `src-tauri/src/commands_execution.rs:124-259`, `src-tauri/src/commands_execution.rs:2095-2537`
- strategy-to-signal bridging for the currently supported live surface:
  - `LiveStrategySpec::DoubleMa`
  - `LiveStrategySpec::CompositeCpu`
  - bridge entry points and normalization at `src-tauri/src/commands_execution.rs:31-39`, `src-tauri/src/commands_execution.rs:808-852`, `src-tauri/src/commands_execution.rs:1534-1655`
- live-runner runtime state, cancellation marker, generation gating, and queued-bar coalescing:
  - `src-tauri/src/commands_execution.rs:344-560`
  - `src-tauri/src/commands_execution.rs:1185-1207`
  - `src-tauri/src/commands_execution.rs:1856-2093`
- live-session artifact/event ownership:
  - event emission and artifact mirroring at `src-tauri/src/commands_execution.rs:610-646`
  - seed/finalize helpers at `src-tauri/src/commands_execution.rs:648-806`
- Alpaca HTTP transport, credential resolution, snapshot normalization, and order payload shaping:
  - `src-tauri/src/commands_execution.rs:854-1465`

This file does not own:

- the underlying strategy math or backtest drilldown implementations
- Polygon websocket ingestion and live-bar source-of-truth ownership
- run-artifact mutation internals
- frontend execution UX and runtime subscription handling

Those concerns are delegated to:

- `crates/app_core/src/lib.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`

## Classification

- Public surface: yes
  - commands are re-exported from `src-tauri/src/commands.rs:79-83`
  - commands are registered in `src-tauri/src/lib.rs:105-114`
- Hot path: mixed
  - credential and snapshot commands are operator-driven control paths
  - `on_live_execution_bar` is a real runtime hot path because every accepted live bar can trigger dataset mutation, strategy execution, and broker submission
- Runtime-critical: yes
  - this file controls live automation arming, stop/halt handling, broker order submission, and recovery-review signaling
- Benchmark-sensitive: partially
  - most issues here are semantics and safety issues, not micro-performance issues
  - bar-trigger handling is latency-sensitive enough that fixes should avoid adding extra REST round trips per bar
- CUDA/runtime-critical: runtime-critical, not CUDA-critical
- Strategy-local: partially
  - it is strategy-aware for the current live-execution surface, but it is not a strategy-local implementation file
- Low-risk support code only: no

## Direct Interactions

### Backend owners this file depends on

- app state and live dataset mutation:
  - `src-tauri/src/state.rs`
- live-bar source and interruption control:
  - `src-tauri/src/commands_data.rs:712-787`
- live-session artifact mutation:
  - `src-tauri/src/run_artifacts.rs:2469-2557`
- strategy drilldown entry points:
  - `crates/app_core/src/lib.rs:103-106`
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`

### Frontend contract anchors

- command allow-list:
  - `web/src/tauri/commands.ts:25-33`
- execution service command callers:
  - `web/src/services/executionService.ts:1752-2081`
- execution store event/status consumers:
  - `web/src/stores/executionStore.ts:581-636`
  - `web/src/stores/executionStore.ts:1592-1642`

### Runtime contract anchors

- event name:
  - `src-tauri/src/commands_execution.rs:281`
- emitted runner status vocabulary:
  - `started`
  - `stopped`
  - `queued_latest`
  - `triggered`
  - `signal_processed`
  - `order_submitted`
  - `error`
  - implementation anchor: `src-tauri/src/commands_execution.rs:610-646`, `src-tauri/src/commands_execution.rs:1782-1822`, `src-tauri/src/commands_execution.rs:1894-2085`
- live-bar bridge from Polygon:
  - `src-tauri/src/commands_data.rs:783-792`

## Standards and Research Log

This packet was written after a targeted official-source research pass focused on Tauri command/runtime semantics, Tokio blocking-task cancellation behavior, and Alpaca order-state expectations.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file exports a broad public command surface and mixes sync and async commands
   - Standard informed:
     - heavy work should not stay on the UI-sensitive command path
     - async command return types and IPC contracts should stay explicit

2. Tauri state-management guidance
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why it was consulted:
     - this file uses global runtime slots, `State<'_, AppState>`, and `AppHandle::state()` access across async and spawned work
   - Standard informed:
     - shared mutable state should be locked narrowly and not rely on wishful concurrency assumptions

3. Tokio task documentation
   - Source: <https://docs.rs/tokio/latest/tokio/task/>
   - Why it was consulted:
     - the runner relies on `spawn_blocking` plus a custom abort marker for cancellation
   - Standard informed:
     - spawned async tasks cancel only at yield points
     - `spawn_blocking` work cannot be force-aborted once it has started running

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines a long-lived public IPC and event contract
   - Standard informed:
     - keep public response shapes stable, unsurprising, and testable

5. Rust Book error handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this file often maps operational failures into `Result<_, String>`
   - Standard informed:
     - operator-visible failures should preserve the distinction between broker failure and audit/logging failure

6. Rust Book testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why it was consulted:
     - the packet needed to distinguish helper coverage from real runtime- or IPC-level verification
   - Standard informed:
     - tests should cover behavior boundaries, not only local pure helpers

7. Alpaca order documentation
   - Source: <https://docs.alpaca.markets/docs/orders-at-alpaca>
   - Why it was consulted:
     - this file submits market and bracket orders and filters open-like states locally
   - Standard informed:
     - order lifecycle includes states such as `new`, `accepted`, `pending_new`, `partially_filled`, `pending_cancel`, and `pending_replace`
     - terminal state assumptions must stay aligned with broker semantics

8. Alpaca websocket streaming documentation
   - Source: <https://docs.alpaca.markets/docs/websocket-streaming>
   - Why it was consulted:
     - the current runner infers safety from position polling alone
   - Standard informed:
     - Alpaca provides `trade_updates` so clients can track order changes in near real time
     - relying only on position polling is weaker than tracking order lifecycle directly

9. Alpaca positions documentation
   - Source: <https://docs.alpaca.markets/docs/working-with-positions>
   - Why it was consulted:
     - this file uses `/v2/positions/{symbol}` as the runner's current-state input
   - Standard informed:
     - position endpoints describe current holdings, not outstanding open-order intent

## Applicable Current Standards

- Tauri command-boundary standard:
  - blocking broker I/O and strategy execution should remain off the async/UI-sensitive path
  - public command names and DTO shapes are frontend contracts and should not churn casually
- Runtime-state standard:
  - live-runner state transitions must be internally coherent under stop, restart, and interruption
  - shared state checks that guard broker mutations should be treated as concurrency-sensitive
- Broker-integrity standard:
  - live order submission must not rely solely on current filled position when open orders may still be working
  - stop/halt semantics should not imply stronger safety than the implementation can actually provide
- Artifact/audit standard:
  - audit artifact failures should not be allowed to blur the truth about whether a broker order was already submitted
- Event-contract standard:
  - `live_execution_runner_event` status strings and payload fields are a cross-layer public contract and must remain stable unless Rust and frontend change together

## Findings

### Correctness / Safety

#### 1. Stop or replace is not a hard barrier against an in-flight broker order

Objective issue:

- the stop path cancels the abort handle and removes the slot, but it does not wait for the in-flight worker to finish:
  - `src-tauri/src/commands_execution.rs:1796-1808`
- the blocking execution path checks the abort handle only at a few phase boundaries:
  - `src-tauri/src/commands_execution.rs:1541-1547`
  - `src-tauri/src/commands_execution.rs:1566-1568`
  - `src-tauri/src/commands_execution.rs:1606-1608`
  - `src-tauri/src/commands_execution.rs:1626-1628`
- after the final abort check, the code still calls `submit_alpaca_order`:
  - `src-tauri/src/commands_execution.rs:1630-1636`
- the runner completion path uses a generation guard only for state bookkeeping and event emission:
  - `src-tauri/src/commands_execution.rs:2017-2025`
- Tokio's own documentation states that `spawn_blocking` work cannot be force-aborted once it has started running

Impact:

- an operator can stop or supersede the runner and still have a stale in-flight run submit a broker order after the stop request
- the generation guard then suppresses the normal runner bookkeeping for that stale execution, which makes the behavior harder to audit

Smallest justified fix:

- do not claim stop as a hard broker-safety barrier until it is actually one
- introduce an explicit `stop_requested` or `disarmed_at_generation` fence that is re-checked immediately before `submit_alpaca_order`
- if the team wants true post-stop safety, the larger fix is a two-phase runner architecture where broker submission requires a final state check performed outside the stale blocking closure

Sensitivity:

- semantics-sensitive
- do not patch casually without a dedicated race-focused test plan

#### 2. The runner computes delta from filled position only and ignores outstanding working orders

Objective issue:

- current runner logic fetches only `/positions/{symbol}`:
  - `src-tauri/src/commands_execution.rs:1558-1565`
  - `src-tauri/src/commands_execution.rs:1100-1129`
- it computes `delta_qty = target_position_qty - current_position_qty`:
  - `src-tauri/src/commands_execution.rs:1571-1587`
- it submits a fresh broker order without checking for existing open orders for the symbol:
  - `src-tauri/src/commands_execution.rs:1615-1636`
- the file already has open-order helpers and an open-like status vocabulary, but only uses them for review and safety actions:
  - `src-tauri/src/commands_execution.rs:1209-1372`
  - `src-tauri/src/commands_execution.rs:2344-2460`
- order payloads do not include a stable `client_order_id`:
  - opening order payload at `src-tauri/src/commands_execution.rs:1414-1465`
  - non-opening payload at `src-tauri/src/commands_execution.rs:1615-1621`
- Alpaca's docs describe open-order lifecycle states and a `trade_updates` stream specifically for order changes

Impact:

- if an earlier order is still `new`, `accepted`, `pending_new`, `partially_filled`, `pending_cancel`, or otherwise working, the next bar can compute the same delta again and stack exposure
- this is a real live-trading risk, not style debt

Smallest justified fix:

- safe-now improvement:
  - document clearly that the current runner is position-based, not order-state-based
  - add a temporary gate that refuses new broker submission while there are open orders for the symbol
- larger but preferred fix:
  - track broker order state explicitly, ideally using `client_order_id` plus broker order updates, instead of relying only on filled position

Sensitivity:

- semantics-sensitive
- broker-behavior-sensitive
- do not merge without paper-account validation

#### 3. Safety-action precheck is not atomic with respect to runner start

Objective issue:

- broker safety actions rely on a point-in-time precheck:
  - `src-tauri/src/commands_execution.rs:1476-1487`
  - used by `cancel_live_execution_open_orders` at `src-tauri/src/commands_execution.rs:2349`
  - used by `flatten_live_execution_position` at `src-tauri/src/commands_execution.rs:2412`
- after the precheck, the functions release the runner lock and then begin broker-side work in `spawn_blocking`:
  - `src-tauri/src/commands_execution.rs:2358-2389`
  - `src-tauri/src/commands_execution.rs:2421-2463`

Impact:

- a new runner can be armed after the precheck but before the broker mutation begins
- the operator-facing error message suggests a stronger exclusion guarantee than the implementation actually enforces

Smallest justified fix:

- recheck runner activity immediately before broker mutation inside the blocking closure, or serialize safety actions and runner start behind a single higher-level execution lock

Sensitivity:

- semantics-sensitive
- safe-now if implemented as a second recheck without changing external behavior

#### 4. Runner start can leave automation armed after the command returns an error

Objective issue:

- the runner runtime is written into the live slot before live-session artifact creation:
  - `src-tauri/src/commands_execution.rs:1756-1764`
  - artifact creation then happens at `src-tauri/src/commands_execution.rs:1766-1777`
- if artifact creation fails, `start_live_execution_runner` returns `Err`, but the slot has already been armed

Impact:

- UI and operator can be told that start failed even though the runner is actually live in memory
- this is a state-coherence bug, especially because the same file uses live-session artifacts as the audit record of arming

Smallest justified fix:

- create the artifact first, or roll back the runner slot on artifact-creation failure before returning the error

Sensitivity:

- safe-now
- strong candidate for the first surgical code change in this file

### API and Naming

#### 5. Internal route and event vocabulary are exposed as stringly typed public contracts

Objective issue:

- the file exposes public string status values over the `live_execution_runner_event` channel:
  - `src-tauri/src/commands_execution.rs:610-646`
- frontend lifecycle logic duplicates those same literals:
  - `web/src/stores/executionStore.ts:581-636`
  - `web/src/stores/executionStore.ts:1592-1642`
- the strategy enum exposes `CompositeCpu` as a public IPC label:
  - `src-tauri/src/commands_execution.rs:31-39`

Impact:

- contract changes are easy to make accidentally and hard to catch without cross-layer tests
- `CompositeCpu` also bakes an implementation-specific route name into a public request surface

Recommended action:

- document the event vocabulary and strategy-kind contract now
- defer any renaming unless Rust and frontend are updated in lockstep

### Error Handling

#### 6. Manual execution can report failure after a successful broker submission

Objective issue:

- `execute_strategy_signal_once` first awaits the blocking execution result:
  - `src-tauri/src/commands_execution.rs:1682-1686`
- it then writes and finalizes a live-session artifact:
  - `src-tauri/src/commands_execution.rs:1687-1697`
  - helper path at `src-tauri/src/commands_execution.rs:745-806`
- any artifact failure is returned as the command error, even if the broker order was already submitted successfully

Impact:

- the operator can receive an apparent execution failure after a successful order placement
- frontend recovery logic in `web/src/services/executionService.ts:2037-2081` treats command success/failure differently, so this can actively mislead the UI

Smallest justified fix:

- preserve broker outcome as primary truth
- if artifact capture fails after successful broker submission, return the successful execution response and attach a warning, or return a response shape that distinguishes `broker_outcome` from `audit_capture_outcome`

Sensitivity:

- safe-now
- high-value correctness fix

### Module Boundaries / Tech Debt

#### 7. Broker transport and runner logic are too tightly coupled to enable realistic failure-path testing

Objective issue:

- HTTP transport uses a process-global `reqwest::blocking::Client`:
  - `src-tauri/src/commands_execution.rs:1081-1098`
- runner logic, broker submission, and artifact mutation are all coordinated directly inside this file
- there is no injectable transport seam for simulating open-order, partial-fill, cancel, or post-submit failure paths

Impact:

- the file has good helper coverage but weak coverage for the highest-risk live paths
- future fixes are more likely to be conservative or under-tested because the current design resists realistic test doubles

Recommended action:

- keep the public command surface here
- extract a narrow broker adapter and a runner-core executor seam behind traits or function parameters only if needed to unlock the high-risk tests listed below
- this is justified tech-debt reduction, not abstraction for abstraction's sake

### Docs / Examples

#### 8. The file lacks an explicit local contract note for runner semantics

Objective issue:

- there is no local documentation describing:
  - whether stop is best-effort or strict
  - whether runner decisions are position-based or order-aware
  - the exact meaning of `queued_latest`, `signal_processed`, and `order_submitted`

Impact:

- operators and future maintainers can over-trust the current safety posture
- frontend store logic ends up being the de facto documentation of event meaning

Recommended action:

- document the runner contract and event vocabulary in this file or in `REPO_MAP.md`
- this is document-only and safe-now

### Tests / Missing Verification

#### 9. Helper coverage is decent, but the highest-risk runtime paths are not directly tested

Objective issue:

- existing tests cover:
  - credential storage helpers
  - symbol normalization
  - status mapping
  - payload builders
  - generation guard helpers
  - selected precondition helpers
  - anchors: `src-tauri/src/commands_execution.rs:2546-3288`
- what is missing:
  - no direct Rust test for `on_live_execution_bar` success, error, queued-latest replay, or interruption behavior
  - no test that proves stale generations cannot emit a broker order after stop
  - no test for the "artifact write fails after successful broker submission" path
  - no test for "runner start fails after slot is armed" rollback behavior
  - no Tauri mock-runtime integration test for this command family in `src-tauri/tests/command_runtime_wiring.rs`

Impact:

- the file's most dangerous behaviors are the least verified

Recommended action:

- add focused runtime tests before changing live-order sequencing logic
- prefer small targeted tests over broad integration harnesses unless a mock-runtime test is the only way to verify the contract

### Perf / Allocation / CUDA Concerns

#### 10. Performance risk is secondary here, but order-state fixes should avoid per-bar REST amplification

Objective issue:

- current design already coalesces pending bars to the latest timestamp:
  - `src-tauri/src/commands_execution.rs:1192-1203`
  - `src-tauri/src/commands_execution.rs:1880-1890`
- the obvious naive fix for order duplication would be "fetch open orders every bar before submit"

Impact:

- that would reduce correctness risk only by adding more broker latency and network load to a runtime-sensitive path
- it would also make stop/replace races even harder to reason about

Recommended action:

- do not "fix" the order-state issue by adding unconditional extra REST polling on each bar
- if this area is upgraded, prefer an in-memory broker-order state model or explicit broker review cache keyed by symbol and order identity

## Recommended Action

- Overall packet disposition: `Larger refactor but defer`

Safe-now changes:

- prevent false error reporting after successful manual submission when artifact capture fails
- roll back or reorder runner start so a failed start cannot leave the runner armed
- add a second runner-activity check inside broker safety actions
- document current runner semantics and event vocabulary explicitly

Larger refactor, but justified:

- redesign the runner so stop/disarm is a true broker-submission barrier
- make broker submission order-aware instead of position-only
- introduce stable intent correlation such as `client_order_id`

Document-only if code is not being changed yet:

- state clearly that current runner automation is best-effort cancellation plus position-based delta logic, not a full order-state engine

Do not touch yet without a dedicated safety review:

- any fix that adds more live REST calls per bar
- any public renaming of event statuses or `LiveStrategySpec` labels

## Exact Local Verification Required If Changed

If this file is changed, the minimum acceptable verification set is:

1. `cargo test -p ta_desktop_demo_app commands_execution -- --nocapture`
2. Add and run targeted tests for:
   - stop or supersede while a blocking execution is already in progress
   - manual execution success followed by artifact write or finalize failure
   - runner-start failure rollback if live-session artifact creation fails
   - queued-latest replay and `pending_bar` behavior in `on_live_execution_bar`
   - safety-action recheck under concurrent runner start
3. `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
4. If command wiring or event payloads change, add or update a Tauri mock-runtime wiring test under `src-tauri/tests/`
5. If broker submission sequencing changes, perform a manual paper-account smoke test and confirm:
   - no duplicate order is submitted while a prior order is still working
   - stop or halt does not allow a stale in-flight submission to slip through
   - the UI still reflects runner and recovery-review state correctly

No CPU or GPU benchmark gate is required unless the change also alters strategy evaluation inside the app-core crates.

## Cross-File Coordination

- `src-tauri/src/commands_data.rs`
  - any change to interruption or live-bar sequencing must be reviewed alongside `stop_live_execution_runner_for_interruption` and the Polygon live-bar bridge
- `src-tauri/src/run_artifacts.rs`
  - any change to live-session capture or finalize behavior must preserve artifact sanitization and retention expectations
- `web/src/services/executionService.ts`
  - command success/failure semantics and optimistic order-row handling depend directly on the current response shapes
- `web/src/stores/executionStore.ts`
  - event vocabulary and recovery-review toggles are tightly coupled to current status strings
- `web/src/tauri/commands.ts`
  - command names are explicitly allow-listed and must remain synchronized

## Code-Quality and Debt Register

- High-value debt:
  - runner stop semantics are stronger in UX than in implementation
  - order-state awareness is missing from live automation
  - broker transport is hard to fake in tests
- Medium-value debt:
  - event statuses are stringly typed across Rust and frontend
  - `CompositeCpu` is an implementation-facing public request label
- Low-value debt:
  - local docs are thinner than the operational risk level of the module justifies

## Performance Opportunity Register

- Keep:
  - latest-bar coalescing via `pending_bar`
  - broker I/O inside `spawn_blocking`
- Improve only if needed:
  - if order-state tracking is added, prefer a cached or streamed state model over repeated REST polling
  - if artifact writes prove noisy on the hot path, measure before introducing any batching or async file-write complexity
- Avoid:
  - per-bar order-list refreshes as a first-line correctness fix
  - micro-allocation cleanup that distracts from the actual broker-safety issues

## Implementation Staging

1. Safe-now cleanup
   - fix manual execution post-submit error reporting
   - make runner start rollback coherent
   - add local contract documentation
2. Safety hardening
   - add a second runner-activity fence for safety actions
   - add explicit pre-submit disarm fencing for the runner
3. Order-state upgrade
   - decide whether the team wants open-order polling, broker snapshot cache, or `trade_updates` integration
   - add intent correlation such as `client_order_id`
4. Contract hardening
   - add mock-runtime and failure-path tests so future changes are less risky

## Change Blast Radius

- High blast radius:
  - runner stop/disarm behavior
  - broker submission sequencing
  - event payload/status changes
- Medium blast radius:
  - artifact capture failure semantics
  - runner-start rollback behavior
- Low blast radius:
  - local documentation improvements
  - internal helper extraction that does not change public contracts

## Observability Gaps

- no explicit event that distinguishes:
  - "runner stop requested"
  - "runner fully quiesced"
  - "stale execution completed after stop but was suppressed"
- no broker-side intent correlation field in emitted execution payloads
- no durable warning path for "broker submission succeeded but audit capture failed"
- no explicit telemetry for open-order gating because the runner does not yet do that gating

## Open Questions and Assumptions

- Assumption:
  - current product intent tolerates best-effort runner stop today, but that should be documented rather than implied away
- Assumption:
  - broker review remains operator-driven after one-shot execution, as reflected in `web/src/services/executionService.ts:2051-2079`
- Open question:
  - should the live runner eventually become order-state-aware through Alpaca `trade_updates`, or is a narrower open-order gate sufficient for the product?
- Open question:
  - does the team want `CompositeCpu` to remain the public live-strategy label, or should strategy naming eventually describe product behavior rather than current implementation route?
