# Review Packet: `src-tauri/src/commands_optimize.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_optimize.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Functionally sound, but not yet clean enough to treat as a finished public runtime boundary`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Cancellation-semantics-sensitive`, `Frontend-contract-sensitive`, `Shared-runtime-infrastructure`

## What This File Owns

`commands_optimize.rs` is the Tauri-side optimization boundary for the main optimization workflows.

This file owns:

- public Tauri commands for:
  - background cancellation: `src-tauri/src/commands_optimize.rs:10-14`
  - foreground cancellation aliasing: `src-tauri/src/commands_optimize.rs:16-25`
  - Double-MA optimization dispatch: `src-tauri/src/commands_optimize.rs:82-107`
  - Composite CPU optimization dispatch: `src-tauri/src/commands_optimize.rs:109-134`
  - Composite MO optimization dispatch: `src-tauri/src/commands_optimize.rs:266-290`
- the Tauri progress-event bridges:
  - `TauriProgressSink`: `src-tauri/src/commands_optimize.rs:28-36`
  - `OffsetProgressSink`: `src-tauri/src/commands_optimize.rs:38-56`
- product policy for optimization-mode normalization:
  - `normalize_product_double_ma_optimization_mode`: `src-tauri/src/commands_optimize.rs:58-74`
  - `normalize_product_composite_cpu_optimization_mode`: `src-tauri/src/commands_optimize.rs:76-80`

This file does not own:

- optimization algorithms, scoring, or route implementation
- loaded-candle storage ownership
- frontend command selection, progress UI, or cancellation UX

Those concerns are delegated to:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `src-tauri/src/state.rs`
- `web/src/services/optimizeService.ts`
- `web/src/stores/taskStore.ts`

## Classification

- Public surface: yes
  - commands are re-exported from `src-tauri/src/commands.rs:109-112`
  - commands are registered in `src-tauri/src/lib.rs:120-123` and `src-tauri/src/lib.rs:183-184`
- Hot path: no
  - this file is request orchestration, not the optimization engine itself
- Runtime-critical: moderately
  - it controls the run mutex, cancel token access, and progress-event bridge for user-visible long-running work
- Benchmark-sensitive: low
  - the heavy work is delegated to app-core and already moved off-thread
- CUDA/runtime-critical: indirectly
  - this file is not CUDA code, but it fronts CPU/GPU/hybrid optimization routes that the frontend treats as canonical backend operations
- Strategy-local: no
  - it is a shared optimization entrypoint for multiple strategy families
- Low-risk support code only: no
  - it owns public IPC contracts and shared runtime helpers

## Direct Interactions

### Backend owners this file depends on

- shared run-slot and cancel-token ownership:
  - `src-tauri/src/state.rs:408-420`
  - `src-tauri/src/state.rs:559-575`
- app-core optimization entrypoints:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_mo.rs`
- shared progress payload contract:
  - `crates/app_core/src/progress.rs:20-42`

### Backend modules that already depend on helpers from this file

- command re-export surface:
  - `src-tauri/src/commands.rs:109-112`
- validation reuses both sinks:
  - `src-tauri/src/commands_validation.rs:1-5`
  - `src-tauri/src/commands_validation.rs:296-307`
  - `src-tauri/src/commands_validation.rs:377`
  - `src-tauri/src/commands_validation.rs:612`
- meta uses `TauriProgressSink`:
  - `src-tauri/src/commands_meta.rs:4`
  - `src-tauri/src/commands_meta.rs:2014`
  - `src-tauri/src/commands_meta.rs:2652`
- campaign routing imports optimization normalization policy and `TauriProgressSink`:
  - `src-tauri/src/commands_campaign.rs:5-8`
  - `src-tauri/src/commands_campaign.rs:1065-1088`
  - `src-tauri/src/commands_campaign.rs:2179`

### Frontend contract anchors

- typed command allowlist:
  - `web/src/tauri/commands.ts:40-43`
  - `web/src/tauri/commands.ts:103-104`
- command selection and invoke shape:
  - `web/src/services/optimizeService.ts:1591-1614`
  - `web/src/services/optimizeService.ts:2833-2891`
  - `web/src/services/optimizeService.ts:2980-2982`
- run-control legacy fallback:
  - `web/src/services/runControlService.ts:27-53`
- task tracking, success toasts, and cancel mapping:
  - `web/src/stores/taskStore.ts:205-227`
  - `web/src/stores/taskStore.ts:242-260`
  - `web/src/stores/taskStore.ts:262-280`
- event listener and progress UI:
  - `web/src/tauri/instrumentation.ts:29-35`
  - `web/src/tauri/instrumentation.ts:169-172`
  - `web/src/stores/taskStore.ts:918-955`
- store-level entrypoints:
  - `web/src/stores/optimizeStore.ts:1073-1114`
  - `web/src/stores/analysisStore.ts:67-75`

### REPO_MAP anchors

- event surface:
  - `REPO_MAP.md:618`
  - `REPO_MAP.md:826`
  - `REPO_MAP.md:834`

## Standards and Research Log

This packet was written after a targeted official-source review focused on Tauri command/event/state behavior, Tokio blocking-task semantics, and Rust public API/error-handling expectations for an IPC-facing runtime boundary.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file is a direct Tauri command boundary and returns public IPC payloads
   - Standard informed:
     - command signatures and return/error behavior are public contracts, not private implementation details

2. Tauri calling the frontend from Rust
   - Source: <https://v2.tauri.app/develop/calling-frontend/>
   - Why it was consulted:
     - the file emits `double_ma_progress` events for optimize, validation, meta, and campaign consumers
   - Standard informed:
     - Tauri events are appropriate for small progress payloads, but they are not high-throughput or strongly typed transport
     - silent event-delivery failure is operationally risky

3. Tauri state management
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why it was consulted:
     - all long-running optimize commands depend on `AppState` and shared cancel/run-slot ownership
   - Standard informed:
     - shared state semantics should match the concurrency and cancellation contract exposed to the frontend

4. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why it was consulted:
     - every optimization command delegates heavy work through `spawn_blocking`
   - Standard informed:
     - once a blocking task has started, cancellation is cooperative rather than a hard abort
     - public cancel surfaces should not imply stronger guarantees than the delegated runtime can provide

5. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - the file exposes public runtime commands and shared helpers reused outside the optimize subsystem
   - Standard informed:
     - public behavior should be predictable, discoverable, and not surprising to downstream consumers

6. Rust Book error handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this module flattens runtime failures into `String` and currently drops event-emission errors entirely
   - Standard informed:
     - failures should either be handled intentionally or surfaced in a way that preserves useful diagnostic value

## Applicable Current Standards

- Tauri boundary standard:
  - optimization work should stay off the command thread
  - command DTOs, event names, and event payload shape are public frontend contracts
- Shared-run standard:
  - all optimize commands should participate consistently in the same run slot and cancel flow
- Event-transport standard:
  - `double_ma_progress` is suitable for small progress/status payloads, not large streaming payloads
  - event-delivery failure should be observable
- API predictability standard:
  - public commands should not silently imply stronger cancellation or execution-mode guarantees than they actually provide
- Product-policy standard:
  - intentional request normalization is acceptable, but externally visible coercion should be documented and verifiable

## Findings

### Correctness / Safety

#### 1. No high-confidence wrapper-local correctness bug is evident, but cancellation is only cooperative

Objective observation:

- all three long-running commands dispatch through `spawn_blocking`:
  - `src-tauri/src/commands_optimize.rs:95-106`
  - `src-tauri/src/commands_optimize.rs:122-133`
  - `src-tauri/src/commands_optimize.rs:278-289`
- the run-state cancel surface only flips an atomic flag:
  - `src-tauri/src/state.rs:569-575`

Impact:

- the file does not offer hard task interruption; cancellation depends on app-core code polling the token
- that is acceptable, but it must be understood as cooperative cancellation

Recommended action:

- do not treat this as a correctness bug
- document the cooperative nature of cancellation wherever the command contract is described or tested

Sensitivity:

- semantics-sensitive

### API and Naming

#### 2. `cancel_double_ma_optimization` is now a legacy alias with broader semantics than its name suggests

Objective issue:

- `cancel_double_ma_optimization` and `cancel_current_run` are identical:
  - `src-tauri/src/commands_optimize.rs:16-25`
- the frontend still carries a fallback path that tries the old command when `cancel_current_run` fails:
  - `web/src/services/runControlService.ts:27-53`
- task routing now maps optimize commands to `cancel_current_run`, not the legacy name:
  - `web/src/stores/taskStore.ts:262-265`

Impact:

- the old name implies Double-MA-specific cancellation, but it actually requests cancellation for any active foreground run
- this preserves compatibility but leaves avoidable public IPC debt and semantic ambiguity in place

Smallest justified fix:

- keep the alias for compatibility today
- explicitly mark it as legacy in code comments, tests, and tracker notes
- remove it only after frontend fallback and any external callers are retired

Sensitivity:

- semantics-sensitive
- coordination required

#### 3. Optimization mode coercion is intentional but insufficiently visible at the public boundary

Objective issue:

- Double-MA requests are coerced back to `Grid` when grid-only outputs are requested:
  - `src-tauri/src/commands_optimize.rs:58-74`
- Composite CPU requests are always coerced to `Grid`:
  - `src-tauri/src/commands_optimize.rs:76-80`
- campaign routing reuses the same normalization policy:
  - `src-tauri/src/commands_campaign.rs:1065-1088`
- the current in-file tests prove the coercion behavior is intentional:
  - `src-tauri/src/commands_optimize.rs:187-263`

Impact:

- callers can submit one mode and get execution under another effective mode without the public contract making that obvious
- this makes route debugging, artifact interpretation, and future API changes harder

Smallest justified fix:

- document the coercion rules clearly in the packet, code comments, or command-facing docs
- if stronger runtime visibility is wanted later, return or record the effective mode rather than changing the coercion behavior

Sensitivity:

- semantics-sensitive

### Error Handling

#### 4. Progress-event emission failures are silently discarded

Objective issue:

- both sinks ignore `app.emit(...)` failures:
  - `src-tauri/src/commands_optimize.rs:32-35`
  - `src-tauri/src/commands_optimize.rs:45-54`
- the UI depends on the `double_ma_progress` stream:
  - `web/src/tauri/instrumentation.ts:169-172`
  - `web/src/stores/taskStore.ts:918-955`

Impact:

- if event delivery breaks, compute can continue while the desktop UI appears stalled or non-responsive
- diagnosis becomes harder because the failure is swallowed at the boundary

Smallest justified fix:

- log or record the first emit failure per run without altering optimization success/failure semantics
- keep optimization execution independent from progress UI transport

Sensitivity:

- safe-now

### Module Boundaries / Tech Debt

#### 5. Shared progress bridges and normalization policy live in an optimize-specific module even though multiple subsystems depend on them

Objective issue:

- `TauriProgressSink` and `OffsetProgressSink` are re-exported from `commands.rs` and reused by validation and meta paths:
  - `src-tauri/src/commands.rs:112`
  - `src-tauri/src/commands_validation.rs:1-5`
  - `src-tauri/src/commands_meta.rs:4`
- campaign logic reuses the optimization normalization helpers directly:
  - `src-tauri/src/commands_campaign.rs:5-8`
  - `src-tauri/src/commands_campaign.rs:1065-1088`

Impact:

- unrelated review and maintenance work on validation, meta, and campaign code now depends on a module named for optimize-only ownership
- this is avoidable coupling and makes ownership boundaries less obvious than they should be

Smallest justified fix:

- move the shared Tauri progress bridge and any clearly cross-cutting normalization policy into a neutral module
- keep optimize-command orchestration in this file

Sensitivity:

- safe-now
- coordination required

### Docs / Examples

#### 6. The file lacks local explanation of two important product behaviors

Objective issue:

- there is no local documentation explaining:
  - why `double_ma_progress` remains the shared event name for non-Double-MA work
  - why product policy coerces some requested optimization modes to `Grid`

Impact:

- maintainers have to rediscover cross-layer product assumptions from tests and frontend code
- review and debugging become slower than necessary

Smallest justified fix:

- add short comments above the normalization helpers and progress sink, or document the behavior in adjacent review/ownership docs

Sensitivity:

- document-only

### Tests / Missing Verification

#### 7. Existing tests cover normalization and cancel-flag wiring, but not real optimize command execution through the Tauri boundary

Objective issue:

- local tests only exercise normalization helpers:
  - `src-tauri/src/commands_optimize.rs:136-264`
- command wiring coverage only verifies cancel intent flags:
  - `src-tauri/tests/command_runtime_wiring.rs:12-39`
- there is no Rust-side test for:
  - successful optimize dispatch through `try_begin_run`, `get_candles`, and `spawn_blocking`
  - `double_ma_progress` payload compatibility
  - in-flight cancellation responsiveness
  - public response serialization for the three optimize commands

Current verification observed:

- `cargo test -p ta_desktop_demo_app commands_optimize -- --nocapture`
  - passed `4` tests
- `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`
  - passed `3` tests

Impact:

- the current coverage is enough to guard local normalization behavior and cancel-flag flipping
- it is not enough to prove that the public optimize IPC surface behaves correctly end to end

Smallest justified fix:

- add targeted Rust-side tests for:
  - command dispatch with a loaded `data_id`
  - progress event emission shape
  - run-lock contention
  - cancellation expectations for in-flight work

Sensitivity:

- safe-now

### Perf / Allocation / CUDA Concerns

#### 8. There is no meaningful file-local performance work to do here

Observation:

- the wrapper already moves heavy work off-thread with `spawn_blocking`
- candle data is retrieved once and forwarded by shared ownership:
  - `src-tauri/src/commands_optimize.rs:89-106`
  - `src-tauri/src/commands_optimize.rs:116-133`
  - `src-tauri/src/commands_optimize.rs:272-289`
  - `src-tauri/src/state.rs:408-420`
- the event payload is small enough to be an appropriate Tauri event payload

Recommended action:

- do not spend time micro-optimizing this file
- prioritize observability, contract clarity, and missing verification instead

## Recommended Action

- Overall packet disposition: `Small surgical refactor`

Safe-now changes:

- stop silently discarding progress-event emission failures
- add targeted Rust-side tests for public optimize command behavior
- move clearly shared progress-bridge helpers out of the optimize-specific module if the refactor stays narrow and mechanical

Document-only changes:

- make cooperative cancellation explicit
- document effective-mode coercion rules
- mark `cancel_double_ma_optimization` as a legacy compatibility alias

Larger refactor but defer:

- any removal of the legacy cancel alias
- any change to mode-coercion policy itself
- any rename of the `double_ma_progress` event channel

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_optimize -- --nocapture`
2. `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`
3. Add and run targeted Rust tests for:
   - optimize command dispatch with loaded candle state
   - progress payload/event compatibility
   - run-lock contention
   - cancellation responsiveness expectations
4. If command/event contracts or cancel flow change, rerun frontend consumers covering:
   - `web/src/services/optimizeService.test.ts`
   - `web/src/services/runControlService.test.ts`
   - `web/src/tauri/instrumentation.test.ts`
   - `web/src/stores/taskStoreCampaignProgress.test.ts`
5. `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`

## Cross-File Coordination

- `src-tauri/src/state.rs`
  - owns the single-run gate, candle lookup, and cancel token
- `src-tauri/src/commands_validation.rs`
  - depends on `OffsetProgressSink` and `TauriProgressSink`
- `src-tauri/src/commands_meta.rs`
  - depends on `TauriProgressSink`
- `src-tauri/src/commands_campaign.rs`
  - depends on both the normalization policy and `TauriProgressSink`
- `web/src/services/optimizeService.ts`
  - owns command selection and invoke shape
- `web/src/services/runControlService.ts`
  - still preserves the legacy cancel fallback
- `web/src/stores/taskStore.ts`
  - owns progress UI and cancel-command mapping

## Code-Quality and Debt Register

- High-value debt:
  - silent progress-event emission failure
  - lack of end-to-end Rust-side optimize command tests
- Medium-value debt:
  - legacy cancel alias with misleadingly narrow name
  - shared runtime helpers living in an optimize-specific module
- Low-value debt:
  - missing local explanation for mode coercion and shared event naming

## Performance Opportunity Register

- Keep:
  - `spawn_blocking` offload for heavy optimize work
  - small event payloads only on `double_ma_progress`
- Improve only if needed:
  - none locally without evidence from profiling or user-visible regressions
- Avoid:
  - micro-optimizing wrapper code
  - changing cancellation/runtime behavior without measurement and contract review

## Implementation Staging

1. Safe now
   - add diagnostics for event-emission failure
   - add missing Rust-side optimize-boundary tests
2. Coordination required
   - move shared progress bridges into a neutral module
   - mark the legacy cancel alias more explicitly across backend and frontend
3. Defer until later pass
   - remove the legacy alias
   - alter coercion policy or rename shared progress events

## Change Blast Radius

- High blast radius:
  - event-name or payload-shape changes for `double_ma_progress`
  - changes to mode-coercion semantics
- Medium blast radius:
  - cancellation-surface changes
  - moving shared helper ownership across command modules
- Low blast radius:
  - adding diagnostics around emit failure
  - adding tests and local documentation

## Observability Gaps

- no signal when progress events fail to emit
- no explicit record of the effective normalized optimization mode at the command boundary
- no test-backed visibility into how quickly in-flight optimize work responds to cancellation

## Open Questions and Assumptions

- Assumption:
  - external callers beyond the current frontend may still exist for `cancel_double_ma_optimization`, which is why it has not yet been removed
- Open question:
  - should the effective optimization mode be returned in command results, or is documentation sufficient?
- Open question:
  - should the shared Tauri progress bridge live beside `commands.rs`, `state.rs`, or a dedicated runtime-bridge module?
