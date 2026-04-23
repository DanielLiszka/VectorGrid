# Final Consolidated Review: `src-tauri/src/commands_reports_meta_xlsx.rs`

- Review date: `2026-04-16`
- Source file: [commands_reports_meta_xlsx.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_meta_xlsx.rs:1>)
- First-pass packet: [commands_reports_meta_xlsx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_reports_meta_xlsx.rs.review.md:1>)
- Second-pass packet: [commands_reports_meta_xlsx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_meta_xlsx.rs.review.md:1>)
- Third-pass packet: [commands_reports_meta_xlsx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_meta_xlsx.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/047__src-tauri__src__commands_reports_meta_xlsx.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/index.html>
  - <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/struct.Worksheet.html>
  - <https://support.microsoft.com/en-us/office/median-function-d0916313-4753-414c-8537-ce85bdd967d2>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It keeps the right bucket while correcting the review scope: this file is a workbook-byte builder, not an artifact-persistence or frontend-hydration owner.

The most important live issue is local statistics correctness. [`median_train_pnl_pct_by_dataset()`](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_meta_xlsx.rs:645>) currently routes through a rounded-index `quantile()` helper, so even-sized samples do not yield a true median. The safest fix is local to the Asset Matrix path, not a broader rewrite of shared `quantile()` semantics. The file also has a real dataset-identity edge: the "By Asset (Train)" sheet writes both `dataset` and `dataset_key`, but the Asset Matrix currently buckets only on `dataset`, which can collapse distinct sources if display labels collide. The rest of the file remains compatibility-sensitive workbook output: visible placeholders, deterministic ordering, chart presence, and ratio-vs-percent presentation should move only with tests.

## Safe-Now Priority

- Fix the Asset Matrix median locally inside `median_train_pnl_pct_by_dataset()` without globally changing shared `quantile()` behavior.
- Add focused regression coverage for:
  - even-sized median behavior,
  - duplicate `dataset` labels with distinct `dataset_key` values,
  - no-train and no-holdout branches,
  - chart-part presence beyond the current shallow checks.
- Decide the duplicate-label assumption explicitly:
  - either document that duplicate `dataset` display labels cannot happen,
  - or disambiguate only on collision.
- Add a short contract note that this module owns workbook bytes, visible sheet structure, and deterministic ordering, but not artifact persistence.

## Deferred Work

- Do not change visible `"-"` placeholders to blanks in this pass.
- Do not broadly flip ratio-vs-percent presentation without workbook-level review.
- Do not turn `serde_json::to_string(...).unwrap_or_default()` into a new hard-fail path without stronger evidence.
- Do not treat artifact save ordering as a local obligation of this file.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_meta_xlsx -- --nocapture`
- If workbook semantics change, also rerun adjacent report/meta export smoke coverage
- Manual workbook-open smoke if visible matrix values, placeholders, or chart layout change

## Regression And Feature-Preservation Constraints

- Preserve current sheet names and persisted `report_xlsx_path` behavior.
- Preserve deterministic workbook ordering.
- Avoid global quantile behavior changes when a local median fix is sufficient.
- Treat placeholder policy and presentation-format changes as visible workbook semantics, not cleanup.

## Final Conclusion

Keep `commands_reports_meta_xlsx.rs` in `Small surgical refactor`. The right implementation target is a local median fix plus stronger workbook-contract tests, while broader formatting and persistence-adjacent changes remain deferred.

