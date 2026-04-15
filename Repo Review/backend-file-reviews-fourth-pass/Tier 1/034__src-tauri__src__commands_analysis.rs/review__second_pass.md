# Second-Pass Review: `src-tauri/src/commands_analysis.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_analysis.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_analysis.rs.review.md`
- First-pass action bucket: `Accept as-is`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated `commands_analysis.rs` as a thin, correctly scoped Tauri wrapper around bounded app-core analysis work and accepted it as-is.

## Stale Check

The live source still has the same basic structure:

- four analysis commands
- eager `AppState` candle lookup before the blocking handoff
- `spawn_blocking` for the actual analysis work
- no run guard and no shared cancel token

The structure did not drift, but the second pass found that the task-model implications were understated in the first pass.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - confirm the wrapper is still using an acceptable async-command shape for borrowed state and blocking work
   - Additional constraint informed:
     - the current command form is still fine, but command correctness includes matching frontend task semantics, not only compile-time shape

2. Tauri state-management guidance
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why consulted in second pass:
     - verify what safety is actually provided by the `State<'_, AppState>` and `Arc` snapshot handoff
   - Additional constraint informed:
     - snapshotting candle ownership before `spawn_blocking` is a real correctness property and should be preserved

3. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - determine whether the current frontend cancel affordance is actually supportable by this backend wrapper
   - Additional constraint informed:
     - started blocking tasks cannot be aborted, so any command presented as cancelable needs a real cooperative cancel path, not just UI wiring

## Delta vs First Pass

## New Material Finding

### 1. The frontend currently treats these commands as cancelable tracked work, but the backend wrappers do not participate in the shared run/cancel model

- Evidence:
  - this file still does not call `try_begin_run()` or use the shared cancel token
  - the frontend task store tracks analysis commands and routes cancellation through `cancel_current_run`
- Why this matters:
  - the user-facing contract currently over-promises cancellation semantics that these commands do not implement
- Practical implication:
  - this is no longer an `Accept as-is` boundary
  - the smallest safe fix is either:
    - remove these commands from the tracked/cancelable frontend set, or
    - explicitly wire them into the same shared cooperative-cancel model used by genuinely cancelable long-running commands

## Preserved Good Property

### 2. The lack of a run guard is not itself a stale-snapshot bug

- Evidence:
  - the commands still retrieve candles from `AppState` before the blocking handoff
- Why this matters:
  - the returned `Arc` snapshot insulates in-flight analysis from later dataset invalidation or replacement
- Practical implication:
  - if cancellation semantics are corrected, this snapshot-before-handoff behavior should stay intact

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_analysis -- --nocapture`
- `npm --prefix web run test -- src/services/analysisService.test.ts src/stores/taskStore.test.ts`

If the fix is backend-side cancellation wiring, also verify:

- shared cancel behavior through `cancel_current_run`
- no regression in dataset replacement while analysis is running

## Regression And Feature-Preservation Constraints

- Preserve the current command names and return payload shapes
- Preserve candle snapshot acquisition before the blocking handoff
- Preserve off-thread execution through `spawn_blocking`
- Do not route these read-only analysis commands into artifact persistence or other unrelated shared-run side effects

## Second-Pass Conclusion

`First-pass bucket changed`

The first pass missed a real cross-layer contract issue. `commands_analysis.rs` should move from `Accept as-is` to `Small surgical refactor` because the frontend currently presents these commands as cancelable tracked work, while the backend wrapper does not actually participate in that cancellation model.
