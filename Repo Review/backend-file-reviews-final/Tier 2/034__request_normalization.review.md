# Final Consolidated Review: `crates/app_core/src/request_normalization.rs`

- Review date: `2026-04-16`
- Source file: [request_normalization.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/request_normalization.rs:1>)
- First-pass packet: [request_normalization.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/request_normalization.rs.review.md:1>)
- Second-pass packet: [request_normalization.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/request_normalization.rs.review.md:1>)
- Third-pass packet: [request_normalization.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/request_normalization.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/034__crates__app_core__src__request_normalization.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid and bucket changed`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://doc.rust-lang.org/std/primitive.str.html>
  - <https://doc.rust-lang.org/std/option/enum.Option.html>

## Consolidated Judgment

The first pass was directionally right that this file is a shared internal contract seam. The second and third passes over-corrected by downgrading it to `Document only`. The fourth pass is correct: the file is still tiny and behavior-preserving changes are still the goal, but the missing invariant coverage is enough to justify `Small surgical refactor`.

No wrong `review_fourth_pass.md` is present in this folder. The fourth-pass packet matches the live file and should replace the third-pass bucket.

## Safe-Now Priority

- Add precise rustdoc on all three helpers:
  - `normalize_optional_path()` trims leading/trailing Unicode whitespace, drops post-trim empties, and is for optional path-like user input rather than filesystem canonicalization;
  - `normalize_topk_include_all()` only applies omission defaults and intentionally preserves mixed states;
  - `normalize_heatmap_bins()` treats `0` as the disabled sentinel and enforces only the upper bound.
- Expand file-local tests to pin the edge states the current tests miss:
  - `normalize_topk_include_all(Some(0), None)`
  - `normalize_topk_include_all(None, Some(true))`
  - `normalize_topk_include_all(Some(0), Some(true))`
  - `normalize_heatmap_bins(None, max)` versus `normalize_heatmap_bins(Some(0), max)`
  - perimeter trim only, with interior whitespace preserved
  - newline-wrapped path-like input

## Deferred Work

- Do not replace `(usize, bool)` with a new struct or enum in this pass.
- Do not broaden this into a repo-wide normalization cleanup.
- Do not switch to `PathBuf`, canonicalization, or typed error plumbing here.
- Do not spend time micro-optimizing `normalize_optional_path()`; semantic stability matters more than tiny allocation wins.

## Verification If Changed

- `cargo test -p ta_app_core request_normalization::tests:: -- --nocapture`
- If behavior, error wording, or path semantics change:
  - representative optimization-path tests that exercise request shaping
  - owning export/report flows that consume normalized path or heatmap settings

## Regression And Feature-Preservation Constraints

- Preserve current sentinel/default semantics unless the owning callers are audited together.
- Do not casually change the heatmap-bounds error string; it may surface upward into commands, logs, or frontend-visible errors.
- Do not broaden the path helper’s scope without making the lossy trim behavior explicit.
- Treat this as a request-semantics seam, not a GPU or performance surface.

## Final Conclusion

Change the final bucket for `request_normalization.rs` from `Document only` to `Small surgical refactor`, treat the fourth pass as valid, and limit the next pass to rustdoc plus missing invariant tests. Representation churn and broader normalization redesign remain deferred.
