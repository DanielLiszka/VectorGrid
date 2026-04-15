# Second-Pass Review: `src-tauri/src/commands_campaign.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_campaign.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_campaign.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass identified `commands_campaign.rs` as an overly broad orchestration hub with real test and modularity debt, but too central and semantics-sensitive for opportunistic restructuring.

## Stale Check

The live source still matches the first-pass assessment materially:

- the public Tauri entrypoint still wraps the whole campaign in one `spawn_blocking` call
- the main orchestration loop still lives in `run_research_campaign_blocking_with_mode`
- DTOs, preload machinery, admission policy, summary shaping, and follow-up chaining are still concentrated in the same file

No material drift was found that would shrink the first-pass concern.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - reconfirm the appropriateness of the current async command shell around a blocking workflow
   - Additional constraint informed:
     - the public command shell is still acceptable, but the long-running body must remain deliberately off-thread

2. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - the full campaign is launched as a single blocking task, and the first pass did not tighten that runtime consequence precisely enough
   - Additional constraint informed:
     - started `spawn_blocking` tasks cannot be aborted, and long-lived blocking work can consume blocking-pool capacity; this sharpens the runtime risk profile of the current design

3. Rust Book testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why consulted in second pass:
     - validate whether the first pass underweighted orchestration characterization testing
   - Additional constraint informed:
     - for a file this broad, characterization coverage is a practical prerequisite to any later split

## Delta vs First Pass

## Refined Runtime Clarification

### 1. The current command shell is more runtime-sensitive than the first pass stated explicitly

- Evidence:
  - `run_research_campaign` still wraps the entire campaign in one `spawn_blocking` call
- Why this matters:
  - Tokio documents that once the blocking task starts, it cannot actually be aborted
- Practical implication:
  - campaign cancellation is only as responsive as the internal `AtomicBool` checks distributed through the blocking workflow
  - future work should not describe this command as “cancellable” without clarifying that cancellation is cooperative only

## Refined Future Refactor Target

### 2. If this file is ever split, the highest-value boundary is no longer “just smaller files,” but “separate long-lived workflow orchestration from bounded command shell”

- Evidence:
  - the public command is thin, while the long-running orchestration body is the dominant concern
- Why this strengthens the first pass:
  - `spawn_blocking` guidance makes the runtime consequence of the current design more concrete
- Safer decomposition target:
  - keep the Tauri command shell thin
  - extract orchestration policy and stage progression behind a dedicated internal service boundary
  - preserve cooperative cancellation and summary/progress semantics before any DTO churn

## Additional Test Gap Clarification

### 3. The missing coverage is not just “more tests,” but specifically cancellation and progress-state characterization under long-running campaign phases

- Why this matters:
  - because started blocking tasks are not abortable, the only safety net for future refactors is proof that internal checkpoints still observe cancel state and emit summary/progress updates in the right order
- Practical implication:
  - if this file changes, add characterization tests that exercise:
    - cancellation after startup
    - summary state when a mid-loop cancel occurs
    - progress emission ordering around auto-follow-up phases

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_campaign -- --nocapture`
- `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts src/services/metaServiceCampaignResume.test.ts src/stores/taskStoreCampaignProgress.test.ts`

If cancellation or long-running workflow structure changes:

- re-run headless campaign harness coverage under `src-tauri/src/bin/ta_campaign_perf.rs`

## Regression And Feature-Preservation Constraints

- Preserve cooperative cancellation semantics unless the workflow is explicitly redesigned
- Preserve progress payload ordering and summary checkpoint behavior
- Preserve artifact reuse, holdout routing, and follow-up chaining semantics unless revalidated end to end
- Treat any orchestration split as coordination work across frontend, `run_artifacts`, `commands_meta`, and app-core

## Second-Pass Conclusion

`Bucket unchanged, but runtime rationale sharpened`

The first-pass `Larger refactor but defer` bucket remains correct. The main second-pass improvement is a more precise runtime explanation: the long-lived `spawn_blocking` shell and cooperative-only cancellation model make this file even less suitable for casual restructuring than the first pass spelled out.
