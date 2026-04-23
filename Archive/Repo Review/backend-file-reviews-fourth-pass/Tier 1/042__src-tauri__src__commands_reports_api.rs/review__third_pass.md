# Third-Pass Review: `src-tauri/src/commands_reports_api.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_api.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_api.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_api.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged, but the fallback and shell-helper boundaries are sharper`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the public Tauri shell and open IPC surface for report files still looked like the right framing under fresh research. The third pass re-read this file together with the already-reviewed shared report owner and concluded that the sharper issue is not helper indirection, but that this file is the frontend-invokable capability boundary for open and reveal behavior.

## Third-Pass Research Log

1. Tauri command boundary guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - reconfirm that this file is the public IPC edge for path-opening behavior
   - Third-pass constraint informed:
     - command wrappers around path launchers remain public capability boundaries

2. Tauri runtime authority model
   - Source: <https://v2.tauri.app/security/runtime-authority/>
   - Why consulted in third pass:
     - check how strongly to treat open and reveal commands as authority-bearing behavior
   - Third-pass constraint informed:
     - open and reveal helpers should stay conservative and explicit about what paths they will act on

3. Tokio blocking-task guidance
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - recheck whether the current wrapper shape is materially wrong
   - Third-pass constraint informed:
     - the sharper risk is capability exposure, not the presence of blocking wrappers themselves

## What Third Pass Sharpened

### 1. This file is the public path-opening capability boundary, not just helper glue
- Evidence:
  - open_reports_dir, reveal_path, and open_path are frontend-invokable Tauri commands.
  - Those commands register the already-reviewed shared path-launch helpers with the desktop IPC surface.
- Why this matters:
  - The authority and path-safety concerns of the shared owner are inherited here because this file is the user-facing entry point.
- Practical implication:
  - Keep the existing bucket, but describe this file as a capability boundary so future cleanup does not understate its sensitivity.

### 2. Path-opening semantics should stay tightly aligned with shared validation and launch policy
- Why this matters:
  - Thin command wrappers are easy places for path checks to drift away from the shared owner over time.
- Practical implication:
  - If hardened, keep validation centralized and make command-level tests prove the public surface cannot widen it accidentally.

## Implementation Priority

- Keep path validation centralized and make the IPC surface explicitly inherit that policy.
- Prefer command-registration and path-boundary tests over wrapper consolidation.

## Verification If Changed

- cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture
- manual desktop smoke covering open and reveal flows for valid and rejected paths

## Regression And Feature-Preservation Constraints

- Preserve current command names and path-opening UX until callers move together.
- Do not widen open and reveal behavior beyond the shared owner path policy.
- Avoid duplicating path validation logic at the command edge.

## Third-Pass Conclusion

`Bucket unchanged, but capability boundary sharpened`

commands_reports_api.rs stays in Small surgical refactor. The third pass mainly tightens the framing: this file is the public Tauri capability boundary for path-open and reveal behavior, so its sensitivity comes from authority exposure more than wrapper complexity.
