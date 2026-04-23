# Final Consolidated Review: `src-tauri/src/commands_meta.rs`

- Review date: `2026-04-16`
- Source file: [commands_meta.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_meta.rs:1>)
- First-pass packet: [commands_meta.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_meta.rs.review.md:1>)
- Second-pass packet: [commands_meta.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_meta.rs.review.md:1>)
- Third-pass packet: [commands_meta.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_meta.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/038__src-tauri__src__commands_meta.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with reprioritization`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://v2.tauri.app/develop/state-management/>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
  - <https://doc.rust-lang.org/std/fs/fn.rename.html>
  - <https://doc.rust-lang.org/std/fs/fn.remove_file.html>

## Consolidated Judgment

The earlier packets were right to keep this file in `Small surgical refactor`, and the fourth pass does not change that bucket. What it changes is priority. The sharpest live issue is no longer generic task-contract hygiene around Meta or ML-confidence commands. It is that offline policy evaluation currently lets holdout-derived `final_score` influence the same evaluation flow whose label is also computed from holdout data, and even the `meta_only` arm can still consult `ml_confidence` as a tie-breaker.

The other bounded findings remain real: `ml_confidence_model_status` still mutates disk through a create-if-needed helper, model writes still target the final path directly, unreadable artifacts can disappear silently from scans, and zero-usable-dataset holdout runs can still report success too quietly. The safe-now cancellation fix, however, is contract honesty, not a pretend backend cancellation retrofit that the current `ta_app_core::ml_confidence` APIs do not support. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- In offline policy evaluation, build candidate ranking from raw saved `meta_score`, not from `group_rank_score` / holdout-influenced `final_score`.
- Split or parameterize the tie-break logic so `meta_only` stays non-ML even on ties.
- Make `ml_confidence_model_status` truly read-only by separating path construction from directory creation.
- Replace direct-final-path model writes with temp-write-plus-rename.
- Surface warnings for unreadable-artifact skips and zero-usable-dataset holdout runs.
- If the helper is touched, make `suggest_meta_report_path` fail after suffix-space exhaustion instead of reusing the last existing candidate.

## Deferred Work

- Defer true cooperative cancellation for Stage 4, Stage 5, manual ML training, and ML policy evaluation until app-core training/evaluation loops gain explicit cancel plumbing.
- Defer scope-local model-mutation locking.
- Defer any `model_healthy` tri-state or Stage 4 / Stage 5 artifact-family contract migration.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- Add targeted tests for:
  - policy evaluation using raw `meta_score` instead of holdout-adjusted `final_score`
  - `meta_only` tie regions not consulting `ml_confidence`
  - status-path read-only behavior
  - atomic/parseable model replacement
  - suffix-space exhaustion in `suggest_meta_report_path`

## Regression And Feature-Preservation Constraints

- Do not globally replace `group_rank_score` with raw `meta_score`; the purity fix belongs only to offline policy evaluation.
- Do not advertise real cancellation for ML `spawn_blocking` workflows unless backend cooperative-cancel plumbing actually exists.
- Do not reuse the app-global run mutex for model CRUD as a small cleanup.
- Do not change `MetaStage4Result.model_healthy` from `bool` in isolation.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `commands_meta.rs` in `Small surgical refactor`
- prioritize policy-evaluation integrity and meta-only arm purity first
- keep status-path and atomic-write hardening in the same safe-now set
- narrow the cancellation recommendation to contract honesty, with backend cancellation retrofit deferred
