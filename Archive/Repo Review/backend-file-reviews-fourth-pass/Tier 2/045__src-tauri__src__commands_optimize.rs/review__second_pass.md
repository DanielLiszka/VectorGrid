# Second-Pass Review: `src-tauri/src/commands_optimize.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_optimize.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_optimize.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a sensitive optimization command boundary because it owns shared run-slot participation, progress-event forwarding, and product-mode normalization for the main optimize flows.

## Stale Check

The live source still has the same core shape:

- shared cancel aliases remain split between foreground and background task paths
- progress still flows through `double_ma_progress`
- optimization-mode normalization is still enforced at this command boundary before app-core execution

No drift weakened the first-pass packet.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - reconfirm that this file is a public IPC boundary, not just internal wrapper glue
   - Additional constraint informed:
     - command-level event names and DTOs remain part of the public contract

2. Tauri frontend event guidance
   - Source: <https://v2.tauri.app/develop/calling-frontend/>
   - Why consulted in second pass:
     - verify how strongly to treat progress payload shape and event naming
   - Additional constraint informed:
     - `double_ma_progress` payload fields are shared cross-feature semantics, not optimize-local implementation detail

3. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - clarify how much cancellation this wrapper can really promise
   - Additional constraint informed:
     - cancellation remains cooperative only; the wrapper must not imply hard-stop semantics

## Delta vs First Pass

## Refined Progress-Contract Finding

### 1. `double_ma_progress` is a shared event contract and `phase` is one of the must-preserve semantics

- Evidence:
  - the same sink is reused outside the main optimize command path
- Why this matters:
  - frontend progress UI and related flows do not treat these payloads as opaque blobs
- Practical implication:
  - `processed_pairs`, `total_pairs`, and especially `phase` should be treated as compatibility-sensitive fields if this area is changed

## Refined Cancellation-Model Finding

### 2. `cancel_background_task` and `cancel_current_run` must stay distinct because they target different state owners

- Evidence:
  - one path targets `BackgroundTaskState`, the other targets the shared run state
- Why this matters:
  - collapsing them for "cleanup" would blur two different ownership models
- Practical implication:
  - keep both commands until the frontend no longer needs the compatibility aliasing and the state model is intentionally redesigned

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_optimize -- --nocapture`
- manual desktop smoke covering:
  - optimization start
  - progress updates
  - cancellation for both optimize and composite flows

## Regression And Feature-Preservation Constraints

- Preserve the `double_ma_progress` event name and payload shape
- Preserve current mode-normalization policy for grid-only outputs
- Keep `cancel_double_ma_optimization` as a compatibility alias until frontend fallback removal is coordinated
- Do not imply hard-stop cancellation semantics the backend does not actually provide

## Second-Pass Conclusion

`Bucket unchanged, but the event and cancellation contracts are sharper`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly tightens two points: `double_ma_progress` is a shared compatibility surface, and the foreground/background cancel commands should not be merged casually because they govern different state domains.
