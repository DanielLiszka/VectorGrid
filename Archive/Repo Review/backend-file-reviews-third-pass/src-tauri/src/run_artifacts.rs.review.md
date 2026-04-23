# Third-Pass Review: `src-tauri/src/run_artifacts.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/run_artifacts.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/run_artifacts.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/run_artifacts.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the canonical artifact persistence owner still looked like the right framing under fresh research. The third pass widened the durability analysis from artifact writes in general to the more specific fact that this file rewrites the same canonical history artifacts with plain whole-file replacement.

## Third-Pass Research Log

1. Rust fs::write semantics
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why consulted in third pass:
     - reconfirm what the current artifact rewrite helpers guarantee
   - Third-pass constraint informed:
     - plain overwrite of canonical history artifacts should not be treated as crash-safe persistence

2. Rust Path semantics
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html>
   - Why consulted in third pass:
     - ground the artifact-path and history-library review
   - Third-pass constraint informed:
     - artifact history depends on both path validation and honest persistence semantics

3. Tauri command boundary guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - reconfirm that these artifacts feed desktop listing, hydration, and replay
   - Third-pass constraint informed:
     - history artifact durability is part of product behavior, not an implementation detail

## What Third Pass Sharpened

### 1. Canonical history artifacts are still rewritten with plain whole-file writes
- Evidence:
  - write_json_pretty, save_run_artifact_blocking, save_batch_summary_blocking, and update_artifact_tags_blocking all end in plain replacement writes.
- Why this matters:
  - A crash or interruption can damage the same artifacts later used for listing, hydration, replay, and tag updates.
  - That is stronger than a generic persistence caveat because it threatens the canonical history library itself.
- Practical implication:
  - Keep the existing bucket, but treat atomic replacement for canonical artifact rewrites as the first safe-now hardening target.

### 2. History integrity matters more here than code-path cleanup
- Why this matters:
  - This file is the long-lived source of truth for multiple desktop flows, so durability beats abstraction tidiness.
- Practical implication:
  - Avoid broad artifact-library refactors before rewrite semantics are tightened.

## Implementation Priority

- First hardening step: move canonical artifact rewrites to atomic replace semantics.
- Then add interruption-oriented tests around save, tag-update, and hydration paths.

## Verification If Changed

- cargo test -p ta_desktop_demo_app run_artifacts -- --nocapture
- manual desktop smoke that interrupts save and tag-update flows and then reloads the artifact library

## Regression And Feature-Preservation Constraints

- Preserve artifact schema and listing and hydration behavior unless the desktop flow changes in lockstep.
- Do not imply crash-safe history updates while using plain whole-file overwrite.
- Keep path validation intact if rewrite mechanics are changed.

## Third-Pass Conclusion

`Bucket unchanged, but canonical rewrite risk sharpened`

run_artifacts.rs stays in Small surgical refactor. The third pass mainly sharpens the risk from general durability concern to a concrete history-integrity issue: canonical run and batch artifacts are still rewritten with plain whole-file replacement.
