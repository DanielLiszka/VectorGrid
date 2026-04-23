# Final Consolidated Review: `src-tauri/src/commands_analysis.rs`

- Review date: `2026-04-15`
- Source file: [commands_analysis.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_analysis.rs:1>)
- First-pass packet: [commands_analysis.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_analysis.rs.review.md:1>)
- Second-pass packet: [commands_analysis.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_analysis.rs.review.md:1>)
- Third-pass packet: [commands_analysis.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_analysis.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/034__src-tauri__src__commands_analysis.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Partially valid`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://v2.tauri.app/develop/state-management/>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>

## Consolidated Judgment

The first pass was too permissive. The second and third passes correctly found that the frontend currently presents these commands as tracked and cancelable while the backend wrappers never participate in the shared run/cancel model. The fourth pass is only partially correct because it reacts against the wrong alternative. It is right that forcing these commands into the optimization/validation run guard would be the wrong local fix. It is wrong to conclude that the file should therefore be accepted as-is.

The right final reading is cross-layer and surgical: this file should keep its current candle-snapshot-before-`spawn_blocking` pattern, but the desktop task semantics around these commands should stop promising cancellation they do not implement. That is stronger than "accept as-is" and safer than retrofitting unrelated run-artifact infrastructure into a thin read-only analysis wrapper.

No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Make the task semantics honest for these four commands:
  - remove the misleading cancel affordance in the frontend, or
  - keep them tracked but explicitly non-cancelable
- Preserve the current `get_candles(...)` before `spawn_blocking(...)` ordering so in-flight analysis keeps using a stable snapshot.
- Add narrow wrapper regression tests if this file is touched at all.

## Deferred Work

- Defer any true cancelable analysis design until there is a scoped analysis-specific executor or cooperative cancel model.
- Defer analysis-specific concurrency throttling unless there is runtime evidence that the current behavior is causing real desktop pressure.
- Defer repo-wide typed Tauri-error cleanup from this file.

## Verification If Changed

- If only frontend task semantics change:
  - `npm --prefix web run test -- src/services/analysisService.test.ts src/stores/taskStore.test.ts src/stores/taskStore.lifecycle.test.ts`
- If the Rust wrapper changes:
  - `cargo test --workspace`
- No performance gate is justified from this file alone unless execution policy changes materially.

## Regression And Feature-Preservation Constraints

- Preserve the current state-snapshot semantics.
- Keep analysis work off the main thread.
- Do not force these commands into the shared optimization/validation run guard just to reuse cancellation plumbing.
- Do not imply true cancellability while the implementation still uses started `spawn_blocking` tasks without cooperative cancellation.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- classify `commands_analysis.rs` as `Small surgical refactor`
- treat the real issue as a frontend/backend task-semantics mismatch, not a backend-only run-guard bug
- preserve the current snapshot-before-handoff structure
- defer any real cancelable-analysis architecture or executor policy work until it is scoped explicitly
