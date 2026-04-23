# Third-Pass Review: `src-tauri/src/commands_campaign.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_campaign.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_campaign.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_campaign.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged, but runtime rationale sharpened`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a long-lived campaign orchestration surface still looked like the right framing under fresh research. The third pass widened the focus from general orchestration complexity to checkpoint durability, because this file keeps rewriting the same campaign summary throughout a long-running flow.

## Third-Pass Research Log

1. Rust fs::write semantics
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why consulted in third pass:
     - confirm how the current summary writes behave on replacement
   - Third-pass constraint informed:
     - plain whole-file overwrite should not be treated as crash-safe checkpointing

2. Tokio blocking-task guidance
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - recheck the orchestration limits of the current blocking campaign path
   - Third-pass constraint informed:
     - long-lived campaign orchestration should be conservative about what state it treats as durable during execution

3. Tauri command boundary guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - reconfirm that campaign state and resume semantics are part of the desktop contract
   - Third-pass constraint informed:
     - resume and checkpoint behavior should be treated as product behavior, not incidental implementation detail

## What Third Pass Sharpened

### 1. Checkpoint durability is the sharper risk than the second-pass packet emphasized
- Evidence:
  - save_campaign_summary_blocking rewrites the same summary path with std::fs::write during a long-running campaign.
  - The summary file is later used as the durable campaign checkpoint and resume anchor.
- Why this matters:
  - A crash or interruption can leave the canonical campaign summary truncated or inconsistent.
  - That is stronger than a generic durability caveat because it directly affects resumability for an in-flight campaign.
- Practical implication:
  - Keep the existing bucket, but treat atomic rewrite semantics for the summary path as the first safe-now hardening target.

### 2. Resume behavior should stay coupled to durable artifact truth, not optimistic in-memory progress
- Why this matters:
  - This file orchestrates work that is much longer-lived than a typical command wrapper.
- Practical implication:
  - Avoid broad orchestration cleanup before the summary-write path is made honest about durability.

## Implementation Priority

- First hardening step: make campaign summary rewrites atomic or clearly staged before replacement.
- Only after durability is fixed should broader orchestration cleanup be considered.

## Verification If Changed

- cargo test -p ta_desktop_demo_app commands_campaign -- --nocapture
- manual interruption and resume smoke that kills a campaign between summary rewrites

## Regression And Feature-Preservation Constraints

- Preserve current campaign summary schema and resume behavior unless the desktop flow is updated together.
- Do not imply durable checkpoints when the underlying write path is still plain overwrite.
- Avoid campaign-loop refactors that make progress reporting harder to correlate with persisted state.

## Third-Pass Conclusion

`Bucket unchanged, but durability scope sharpened`

commands_campaign.rs stays in Larger refactor but defer. The third pass mainly sharpens the risk from general orchestration complexity to a concrete resume and checkpoint hazard: canonical campaign summaries are still replaced with plain whole-file writes during long-running work.
