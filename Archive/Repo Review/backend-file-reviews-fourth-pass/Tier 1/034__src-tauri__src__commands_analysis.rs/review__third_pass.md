# Third-Pass Review: `src-tauri/src/commands_analysis.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_analysis.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_analysis.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_analysis.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

The second pass correctly found the fake-cancel contract mismatch, but this file remained high ROI because it sits on a tracked desktop task surface and all four commands still dispatch CPU-heavy work through unconstrained `spawn_blocking`.

## Third-Pass Research Log

1. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - verify whether unbounded analysis concurrency is a real runtime concern
   - Third-pass constraint informed:
     - CPU-bound blocking work should be bounded instead of left to arbitrary parallel fan-out

2. Tauri calling Rust
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - confirm that the wrapper shape is still acceptable while reassessing task semantics
   - Third-pass constraint informed:
     - command correctness here includes desktop task behavior, not only signature shape

3. Tauri state management
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why consulted in third pass:
     - re-check whether the current candle handoff remains a correctness boundary worth preserving
   - Third-pass constraint informed:
     - snapshot acquisition before `spawn_blocking` is still a must-preserve property

## What Third Pass Sharpened

### 1. The second pass caught fake cancellation, but it underweighted runtime-capacity pressure from unconstrained CPU-bound `spawn_blocking`

- Evidence:
  - all four commands still call `spawn_blocking`
  - none uses `try_begin_run()`
  - none applies a local semaphore or other analysis-only concurrency bound
  - the frontend still tracks these commands and maps them to `cancel_current_run`
- Why this matters:
  - the current issue is not only "cancel does nothing"
  - the desktop can also fan out multiple expensive read-only analyses at once, which can compete with other CPU-bound work
- Practical implication:
  - the smallest safe fix is still surgical:
    - either remove these commands from the tracked/cancelable set, or
    - wire them into a bounded analysis-only execution path with honest cancel semantics

### 2. The right fix is not to force these commands into unrelated run-artifact or shared optimization side effects

- Evidence:
  - this file remains a thin read-only wrapper around candle snapshots plus app-core analysis helpers
- Why this matters:
  - the strongest existing property is that analysis work is off-thread and uses a stable candle snapshot
- Practical implication:
  - any hardening should stay local to analysis work
  - do not "solve" the contract issue by routing these commands through unrelated optimization or artifact persistence paths

## Implementation Priority

- Highest-ROI hardening:
  - make frontend task semantics honest for these four commands
- If backend-side hardening is chosen:
  - add a small analysis-only concurrency bound without changing public payloads

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_analysis -- --nocapture`
- `npm --prefix web run test -- src/services/analysisService.test.ts src/stores/taskStore.lifecycle.test.ts src/stores/taskStore.test.ts`

Also verify manually:

- repeated drilldown/local-stability requests do not starve the desktop with avoidable parallel CPU work
- candle replacement during analysis still does not mutate the in-flight snapshot

## Regression And Feature-Preservation Constraints

- Preserve current command names and payload shapes
- Preserve candle snapshot acquisition before the blocking handoff
- Preserve read-only off-thread execution
- If throttling is added, scope it to analysis work rather than serializing unrelated runs

## Third-Pass Conclusion

`Bucket unchanged, but the real issue is now two-part`

The second-pass cancel-contract finding still stands, but the third pass adds a sharper capacity concern: these tracked analysis commands are also unconstrained CPU-bound `spawn_blocking` work. The best third-pass reading is still `Small surgical refactor`, with honest cancel semantics and bounded analysis concurrency as the safe-now target.
