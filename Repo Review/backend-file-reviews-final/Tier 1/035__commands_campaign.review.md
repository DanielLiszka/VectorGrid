# Final Consolidated Review: `src-tauri/src/commands_campaign.rs`

- Review date: `2026-04-15`
- Source file: [commands_campaign.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_campaign.rs:1>)
- First-pass packet: [commands_campaign.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_campaign.rs.review.md:1>)
- Second-pass packet: [commands_campaign.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_campaign.rs.review.md:1>)
- Third-pass packet: [commands_campaign.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_campaign.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/035__src-tauri__src__commands_campaign.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Targeted hardening now; larger redesign still deferred`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
  - <https://doc.rust-lang.org/std/fs/fn.write.html>
  - <https://doc.rust-lang.org/std/fs/fn.rename.html>
  - <https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all>

## Consolidated Judgment

The first three passes were right to defer any broad split here. This file is still a large orchestration hub with a public command surface, progress/event contracts, checkpointing, and long-lived blocking execution. The fourth pass is also right that defer-only is no longer the strongest conclusion. The live source exposes several bounded orchestration correctness problems that can be fixed safely before any broad redesign.

The highest-value change is not checkpoint durability in isolation. The stronger immediate defects are control-plane mismatches inside the current workflow: `ValidateOnly` silently depends on `skip_existing`, artifact reuse matching leans on optional request hints instead of the freshly prepared dataset identity, and skipped validation items can point at the wrong artifact path. Those are direct correctness and product-surface issues, not architecture polish.

No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Decouple `ValidateOnly` from `skip_existing` so validation-only runs always load and use existing optimization artifacts, or otherwise fail explicitly.
- Match artifact reuse against the prepared dataset identity that the backend just loaded, with request-provided fingerprint data acting only as an optional stricter key.
- Return and record the matched validation artifact path when validation is skipped because a reusable validated artifact already exists.
- Harden campaign summary rewrites with a same-directory temp-write-and-replace flow, and actually populate `checkpoint_save_ms`.
- Make dataset accounting internally consistent by rejecting blank dataset paths up front or normalizing the dataset list once and using it everywhere.

## Deferred Work

- Defer the broad module split and service extraction.
- Defer typed orchestration-error redesign.
- Defer larger telemetry and campaign-summary-state redesign.
- Defer any engine-selection or validation-policy redesign that is unrelated to the bounded orchestration fixes above.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_campaign -- --nocapture`
- `cargo test --workspace`
- `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts src/services/metaServiceCampaignResume.test.ts src/stores/taskStoreCampaignProgress.test.ts src/tauri/instrumentation.test.ts`
- Add targeted Rust tests for:
  - `ValidateOnly` with `skip_existing = false`
  - reuse matching when request identity hints are absent
  - validation-skip summaries recording the validation artifact path
  - checkpoint timing capture
  - blank-path dataset handling
- If the summary writer changes, run an interruption/resume smoke to confirm the checkpoint remains parseable.

## Regression And Feature-Preservation Constraints

- Preserve the public `run_research_campaign` request/response schema and `research_campaign_progress` event shape.
- Preserve cooperative cancellation semantics and do not imply true abortability for the long-lived blocking workflow.
- Keep temp-file replacement in the same directory as the final summary path.
- Do not casually widen `summary.state` semantics without auditing the frontend and resume flows together.
- Preserve current optimization and validation engine selection while touching only orchestration correctness.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- move `commands_campaign.rs` to `Targeted hardening now; larger redesign still deferred`
- fix the bounded orchestration correctness defects before attempting any large split
- treat checkpoint hardening as part of that safe-now work, not the only safe-now work
- defer service extraction, typed-error cleanup, and larger campaign-architecture changes until characterization coverage is stronger
