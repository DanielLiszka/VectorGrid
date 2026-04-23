# Third-Pass Review: `src-tauri/src/polygon.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/polygon.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/polygon.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/polygon.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a thin but public Tauri IPC boundary still looked like the right framing under fresh research.

## Third-Pass Research Log

1. Tauri v2 command boundary guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - reconfirm that even small command wrappers remain public IPC and DTO surfaces
   - Third-pass constraint informed:
     - event names, command signatures, and error semantics should be treated as compatibility surfaces rather than internal glue

2. Tauri runtime authority model
   - Source: <https://v2.tauri.app/security/runtime-authority/>
   - Why consulted in third pass:
     - recheck how strongly frontend-invokable command seams should be treated as capability boundaries
   - Third-pass constraint informed:
     - wrapper cleanups must not widen authority or blur which module actually owns a sensitive operation

3. Tokio blocking-task guidance
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - verify the cancellation and ownership limits of the current spawn_blocking usage
   - Third-pass constraint informed:
     - wrapper code should not over-promise cancellation or move more orchestration into long-lived blocking work without intent

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; the sharper read is owner drift, not file-local redesign
- Why this matters:
  - This file is thin enough that broad refactor churn would mostly move responsibilities around rather than remove them.
  - The second-pass call of `Bucket unchanged` still fits after reopening the live source.
- Practical implication:
  - Keep the existing bucket and prefer contract tests, narrow naming cleanup, or documentation of command ownership over structural rewrites.

### 2. Public IPC seams should stay honest about state ownership and cancellation scope
- Why this matters:
  - Small wrappers can still regress the app if they collapse distinct state owners, event channels, or background-task boundaries into one “cleanup” path.
- Practical implication:
  - Preserve current command/event identities until frontend and backend consumers are changed together.

## Implementation Priority

- Prefer bounded contract hardening: targeted tests around command registration, error mapping, and state-owner boundaries.
- Avoid broad wrapper consolidation unless the owning state model is being redesigned at the same time.

## Verification If Changed

- cargo test -p ta_desktop_demo_app polygon -- --nocapture
- manual desktop smoke for the command path or IPC seam this file owns

## Regression And Feature-Preservation Constraints

- Preserve registered command names and argument shapes unless frontend callers move together.
- Do not widen runtime authority or silently merge distinct state-owner paths.
- Keep cancellation/event semantics aligned with what the backend actually guarantees.

## Third-Pass Conclusion

`Bucket unchanged`

polygon.rs stays in `Small surgical refactor`. The third pass mostly confirms that the main risk here is public-boundary drift, not a larger file-local rewrite opportunity.
