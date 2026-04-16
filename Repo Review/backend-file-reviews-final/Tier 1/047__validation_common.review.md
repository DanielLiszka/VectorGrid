# Final Consolidated Review: `src-tauri/src/validation_common.rs`

- Review date: `2026-04-15`
- Source file: [validation_common.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/validation_common.rs:1>)
- First-pass packet: [validation_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/validation_common.rs.review.md:1>)
- Second-pass packet: [validation_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/validation_common.rs.review.md:1>)
- Third-pass packet: [validation_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/validation_common.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/047__src-tauri__src__validation_common.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with scope correction`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/path/struct.Path.html>
  - <https://doc.rust-lang.org/stable/std/primitive.slice.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The first three passes were right to keep this file in the smallest-action bucket. It contains shared cross-ticker helpers, candidate aggregation, and lexical path-key normalization that are used by wrapper-level validation flows. That makes broad identity-model changes risky, but it does not prevent bounded hardening.

The fourth pass correctly identifies `normalize_path_key()` as the strongest live issue and is also right to keep the bucket small. Its one real overreach is the recommendation to add caller-side handling for cross-ticker datasets that sanitize to empty; that gap is already handled upstream in `commands_validation.rs`. The packet belongs to the correct file and is not a misplaced copy. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Make `normalize_path_key()` platform-aware and non-destructive so Unix-style paths are not aliased by unconditional slash rewriting and lowercasing. Keep this lexical, not `canonicalize()`-driven, unless the repo explicitly adopts a stronger filesystem-identity contract.
- Add cheap invariant hardening around `finalize_cross_ticker_candidates()`, such as debug assertions and focused tests for accumulator alignment, missing/empty accumulator handling, and stable equal-score ordering.
- Preserve the current quantile and median-selection behavior for now, but add tests that freeze the current nearest-rank semantics so future cleanup does not accidentally drift from app-core behavior.

## Deferred Work

- Defer repo-wide filesystem identity consolidation and any `canonicalize()`-based path identity policy.
- Defer new tie-break rules or candidate-key ordering changes unless they are reviewed across all validation layers together.
- Defer any additional caller-side empty-sanitize handling in this file because the upstream command layer already exits early when no cross-ticker datasets survive sanitization.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app -- --nocapture`
- Add targeted tests for `normalize_path_key()` on Windows-style and Unix-style inputs, including case-sensitive Unix paths.
- Add invariant tests for `finalize_cross_ticker_candidates()` covering missing accumulators, empty accumulators, and equal-score ordering.
- Add tests that prove current quantile and median-selection behavior stays aligned with the app-core validation path.

## Regression And Feature-Preservation Constraints

- Preserve current cross-ticker candidate scoring and ordering semantics except where explicit invariant hardening requires clarification.
- Do not introduce `canonicalize()` or absolute-path requirements as part of a narrow dedupe-key fix.
- Keep any path-key change focused on preventing cross-platform aliasing, not on redefining dataset identity globally.
- Preserve current nearest-rank quantile behavior unless a repo-wide validation-policy change is approved.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `validation_common.rs` in `Small surgical refactor`
- fix the path-key aliasing risk and add invariant/test hardening around cross-ticker candidate finalization
- preserve current quantile behavior while freezing it with tests
- defer repo-wide filesystem identity redesign and broader candidate-ordering policy changes
