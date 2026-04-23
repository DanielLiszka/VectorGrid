# Final Consolidated Review: `src-tauri/src/commands_optimize.rs`

- Review date: `2026-04-16`
- Source file: [commands_optimize.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_optimize.rs:1>)
- First-pass packet: [commands_optimize.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_optimize.rs.review.md:1>)
- Second-pass packet: [commands_optimize.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_optimize.rs.review.md:1>)
- Third-pass packet: [commands_optimize.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_optimize.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/045__src-tauri__src__commands_optimize.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://v2.tauri.app/develop/state-management/>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It correctly narrows this file back to what it actually owns: a thin but compatibility-sensitive Tauri IPC seam over optimize flows, shared run exclusivity, and the `double_ma_progress` event bridge.

The earlier packets were right to keep the bucket small, but the safe-now work is mostly contract hardening and regression coverage, not ownership cleanup. The file's real obligations are preserving command names, progress payload shape, the foreground/background cancellation split, and the product-mode normalization behavior at the command boundary. The live source does not justify moving logic around aggressively as long as those contracts remain in force.

## Safe-Now Priority

- Add explicit regression coverage proving early-failure paths still release the run guard correctly.
- Add focused tests around command-boundary mode normalization so Grid-only-output requests keep today's compatibility behavior.
- Keep progress transport failures non-fatal; if observability is added, make it diagnostic-only and low-noise.
- Add a short comment clarifying that `OffsetProgressSink` intentionally rewrites both count offsets and `phase`.

## Deferred Work

- Do not merge `cancel_background_task` with `cancel_current_run`; they govern different state owners.
- Do not deduplicate optimize and app-core progress helpers unless their phase/count behavior is proven identical.
- Do not push normalization into app-core in this pass; that is a broader product-contract change.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_optimize -- --nocapture`
- If shared progress helpers move, also rerun adjacent validation/meta command coverage
- Manual desktop smoke for:
  - optimize start,
  - progress updates,
  - run cancellation,
  - background-task cancellation alias behavior

## Regression And Feature-Preservation Constraints

- Preserve public command names and argument shapes.
- Preserve the `double_ma_progress` event name and payload fields.
- Preserve the distinct state-owner split between run cancellation and background-task cancellation.
- Preserve the current run-guard lifetime and keep transport-layer event emission non-authoritative.

## Final Conclusion

Keep `commands_optimize.rs` in `Small surgical refactor`. The highest-value next work is tighter guard/normalization coverage and careful preservation of IPC/event contracts, not structural cleanup.

