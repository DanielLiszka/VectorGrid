# Final Consolidated Review: `src-tauri/src/commands_reports_validation_xlsx.rs`

- Review date: `2026-04-16`
- Source file: [commands_reports_validation_xlsx.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_validation_xlsx.rs:1>)
- First-pass packet: [commands_reports_validation_xlsx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_reports_validation_xlsx.rs.review.md:1>)
- Second-pass packet: [commands_reports_validation_xlsx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_validation_xlsx.rs.review.md:1>)
- Third-pass packet: [commands_reports_validation_xlsx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_validation_xlsx.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/048__src-tauri__src__commands_reports_validation_xlsx.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/struct.Worksheet.html>
  - <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/struct.Url.html>
  - <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/utility/fn.quote_sheet_name.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It keeps the correct compatibility-sensitive workbook framing while replacing overly broad CSV/artifact discussion with source-local issues that are actually visible in this renderer.

The clearest live bug is the dashboard row collision at row `73`: the renderer writes the `Slice Audit` label and note there, then overwrites the label cell with `Bootstrap Compare` while leaving the value cell from the prior section in place. The next strongest issue is truthfulness: [`disabled_or_omitted_status()`](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_validation_xlsx.rs:189>) ignores `actual_skipped_module_summary`, so a module can be known to be skipped yet still render as generic `not run` in primary status cells. The internal hyperlinks are also weaker than they should be for sheet names with spaces, and regime-bucket display is currently inconsistent across workbook surfaces.

## Safe-Now Priority

- Fix the row-73 dashboard overwrite so `Slice Audit` and `Bootstrap Compare` have distinct rows.
- Teach the absent-module status helper to surface `skipped` when that summary explicitly says so.
- Canonicalize internal sheet-link targets for names with spaces and add workbook-level regression coverage for those links.
- Strengthen direct tests around:
  - named sheet presence,
  - row-73 dashboard layout,
  - run-rejected approved-sheet behavior,
  - current cross-sheet approval semantics.

## Deferred Work

- Do not change optional numeric placeholder policy in this pass; that can affect charts as well as sorting/filtering.
- Do not casually unify the two regime-audit surfaces until the intended workbook contract is explicit.
- Do not change candidate-level versus final-approved wording without a product decision and workbook review.
- Do not treat artifact save ordering as a local responsibility of this renderer.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_validation_xlsx -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`
- Manual workbook-open smoke if sheet-link targets, dashboard wording, or regime presentation change

## Regression And Feature-Preservation Constraints

- Preserve current sheet names and visible tab vocabulary.
- Preserve current approval-sheet versus run-level verdict semantics unless intentionally changed across the workbook.
- Keep placeholder policy stable unless chart behavior is reviewed at the same time.
- Treat regime-bucket label normalization as visible workbook output, not a silent cleanup.

## Final Conclusion

Keep `commands_reports_validation_xlsx.rs` in `Small surgical refactor`. The best next step is a narrow workbook-truthfulness hardening pass: fix the row-73 collision, surface `skipped` correctly, quote/test internal links, and add direct workbook-contract coverage.

