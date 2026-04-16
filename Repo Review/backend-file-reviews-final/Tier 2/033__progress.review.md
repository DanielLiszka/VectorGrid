# Final Consolidated Review: `crates/app_core/src/progress.rs`

- Review date: `2026-04-16`
- Source file: [progress.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/progress.rs:1>)
- First-pass packet: [progress.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/progress.rs.review.md:1>)
- Second-pass packet: [progress.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/progress.rs.review.md:1>)
- Third-pass packet: [progress.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/progress.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/033__crates__app_core__src__progress.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with narrowing`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/cargo/reference/semver.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://v2.tauri.app/develop/calling-frontend/>
  - <https://doc.rust-lang.org/std/primitive.usize.html>

## Consolidated Judgment

The first three passes were correct that this file is a small but compatibility-sensitive coordination seam. The fourth pass is the right final review because it narrows the safe work to docs and a tiny forwarding test instead of overselling type reshaping on an all-public, IPC-adjacent surface.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and should govern future work here.

## Safe-Now Priority

- Add rustdoc that states the actual contract:
  - `emit_double_ma_progress()` is a thin forwarder and a silent no-op when `sink` is `None`;
  - the helper forwards `processed_pairs`, `total_pairs`, and `phase` unchanged;
  - caller-side code owns monotonicity, ordering, and `processed_pairs <= total_pairs` discipline;
  - sink panics and blocking behavior are not caught or isolated here;
  - `with_double_ma_campaign_vram_prefetch_request()` is an optional synchronous capability hook, not basic progress reporting;
  - `on_double_ma_campaign_vram_prefetch_feedback()` is a cold-path callback.
- If the file is touched beyond docs, add one tiny local unit test that pins:
  - `None` is a no-op;
  - a present sink receives the payload unchanged.

## Deferred Work

- Do not convert `phase` to an enum in this bucket.
- Do not add fields to `DoubleMaProgressPayload`, variants to `DoubleMaCampaignVramPrefetchFeedback`, or change `ProgressSink` method signatures in this pass.
- Do not split the trait, introduce typed request/feedback DTOs, or redesign the Tauri transport in `progress.rs`.
- Do not change counter widths just because `usize` is pointer-sized; that is a longer-horizon IPC/wire-shape decision.

## Verification If Changed

- If docs only: local rustdoc plus one tiny forwarding/no-op unit test if added
- If behavior or payload shape changes:
  - `cargo test -p ta_app_core cpu_dynamic_strategy_progress_parity_matches_legacy_results -- --nocapture`
  - `cargo test -p ta_app_core shared_validation_core_progress_phase_names_and_monotonic_counts -- --nocapture`
  - campaign-prefetch command tests in `ta_desktop_demo_app`
  - frontend progress aggregation and throttling tests identified in `REPO_MAP.md`

## Regression And Feature-Preservation Constraints

- Preserve current field names and phase-string vocabulary in a small pass.
- Keep the helper thin; do not turn it into a validating, panic-catching, or transport-heavy layer.
- Treat this surface as IPC-adjacent public API first and a cleanup target second.
- Remember that Tauri events are intended for small streamed payloads, not higher-throughput transport redesign experiments inside this file.

## Final Conclusion

Keep `progress.rs` in `Small surgical refactor`, treat the fourth pass as valid, and narrow implementation work to explicit contract docs plus at most a tiny forwarding test. Trait reshaping, DTO redesign, and transport changes remain deferred.
