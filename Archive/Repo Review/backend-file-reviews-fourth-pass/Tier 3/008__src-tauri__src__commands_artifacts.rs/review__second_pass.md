# Second-Pass Review: `src-tauri/src/commands_artifacts.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_artifacts.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_artifacts.rs.review.md`
- First-pass action bucket: `Accept as-is`
- Second-pass action bucket: `Accept as-is`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass concluded that `commands_artifacts.rs` was a thin persistence-boundary wrapper that correctly delegated to `run_artifacts.rs`, kept blocking filesystem work off the Tauri command path, and should remain accepted as-is.

## Stale Check

The live source still aligns with the first-pass packet:

- the command surface is still a uniform delegation layer
- all filesystem-heavy commands still use `spawn_blocking`
- path policy and containment checks still live in `run_artifacts.rs`, not here

No material source drift was found.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - reconfirm that these command wrappers still follow the intended async command shape
   - Additional constraint informed:
     - there is still no reason to collapse this layer into synchronous command handlers

2. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - clarify blocking-pool and abort behavior for artifact-library scans, hydration, and replay loading
   - Additional constraint informed:
     - these wrappers must stay bounded; they should not accrete long-lived background behavior under the guise of convenience

## Delta vs First Pass

## Refined Boundary Constraint

### 1. The wrapper must continue to avoid inventing its own path or persistence policy

- Evidence:
  - every command still delegates directly into `run_artifacts::*_blocking`
- Why this strengthens the first pass:
  - the second pass confirms the wrapper remains correct only because path containment, runs-root policy, and replay shaping are centralized elsewhere
- Practical implication:
  - even well-intentioned pre-validation or path normalization in this file would risk divergence from the real owner

## Refined Runtime Constraint

### 2. `spawn_blocking` remains appropriate here, but only for bounded file operations

- Evidence:
  - every load/list/save/delete wrapper still uses `spawn_blocking`
- Why this strengthens the first pass:
  - Tokio documents that started blocking tasks cannot be aborted and consume blocking-pool capacity until completion
- Practical implication:
  - if artifact-library scans or replay hydration ever become persistent/background workflows, they should move to a dedicated worker design instead of expanding this command layer

## No Material New Defect

### 3. No new file-local bug or missing contract break was surfaced

- The wrapper remains intentionally repetitive and readable.
- The second pass did not uncover a stronger local reason to change the first-pass bucket.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_artifacts -- --nocapture`
- `npm --prefix web run test -- src/services/historyService.test.ts src/services/metaService.test.ts src/services/optionsExpressionService.test.ts src/services/settingsService.test.ts`

If any command contract changes:

- reverify `web/src/tauri/commands.ts`

## Regression And Feature-Preservation Constraints

- Preserve the current command names and payload shapes
- Preserve delegation into `run_artifacts.rs`
- Preserve the off-thread filesystem boundary with `spawn_blocking`
- Do not duplicate path-policy logic in this wrapper

## Second-Pass Conclusion

`No material delta from first pass`

The first-pass `Accept as-is` bucket still stands. The useful second-pass improvement is a stronger statement of what must not move into this wrapper: path policy, artifact semantics, and any long-lived persistence workflow behavior.
