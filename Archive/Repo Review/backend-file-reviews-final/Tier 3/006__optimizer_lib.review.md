# Final Consolidated Review: `crates/optimizer/src/lib.rs`

- Review date: `2026-04-16`
- Source file: [lib.rs](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/lib.rs:1>)
- First-pass packet: [lib.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/optimizer/src/lib.rs.review.md:1>)
- Second-pass packet: [lib.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/lib.rs.review.md:1>)
- Third-pass packet: [lib.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/lib.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/006__crates__optimizer__src__lib.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small hardening`
- External research anchors:
  - <https://serde.rs/container-attrs.html>
  - <https://serde.rs/field-attrs.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://doc.rust-lang.org/cargo/reference/semver.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review, with one narrowing caveat: the optional debug-only batch-cardinality assertion on [CpuEngine](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/lib.rs:34>) is lower priority than the packet implies because real consumer enforcement now exists elsewhere. The stronger parts of the fourth pass still hold: this file is primarily a public contract surface for [ObjectiveKind](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/lib.rs:10>), [OptimizationMode](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/lib.rs:21>), [BacktestEngine](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/lib.rs:30>), [OptimizationResult](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/lib.rs:54>), and [OptimizationHeatmap](</C:/Rust Projects/ta_desktop_demo/crates/optimizer/src/lib.rs:73>).

The earlier packets were too passive in treating this as documentation only. The file still does not justify structural API churn, but it does justify explicit contract docs and serde-contract tests because these enums and DTOs already leak across request and artifact boundaries.

## Safe-Now Priority

- Add precise rustdoc for:
  - `ObjectiveKind`
  - `OptimizationMode`
  - `BacktestEngine`
  - `CpuEngine`
  - `OptimizationResult`
  - `OptimizationHeatmap`
- Document actual contract limits, especially that `OptimizationMode` is request vocabulary and not every consumer necessarily honors every variant.
- Add narrow serde contract tests for enum round-trips and sparse optional-field deserialization on the DTOs.
- Treat any debug-only `CpuEngine` cardinality assertion as optional, not mandatory.

## Deferred Work

- Sealing or redesigning `BacktestEngine`.
- Changing enum wire spelling or adding serde renames.
- Replacing tuple-based `top` / `all` entries with named structs.
- Converting the trait to return `Result` or redesigning the adapter surface.

## Verification If Changed

- `cargo test -p ta_optimizer -- --nocapture`
- If serde tests are added, ensure they cover enum round-trip and omitted optional-field cases for `OptimizationResult` and `OptimizationHeatmap`.

## Regression And Feature-Preservation Constraints

- Preserve the serde-visible shape: enum spellings, field names, omission behavior, tuple ordering.
- Do not add `deny_unknown_fields`.
- Do not document `OptimizationMode` as if every caller honors every variant.
- Do not treat `BacktestEngine` as internal-only without a real compatibility review.

## Final Conclusion

Change the final bucket from `Document only` to `Small hardening`. The fourth pass is valid, no wrong fourth-pass file is present, and the right next move is contract documentation and serde-surface tests, not public API redesign.
