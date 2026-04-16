# Final Consolidated Review: `src-tauri/src/commands_reports_csv.rs`

- Review date: `2026-04-16`
- Source file: [commands_reports_csv.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_csv.rs:1>)
- First-pass packet: [commands_reports_csv.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_reports_csv.rs.review.md:1>)
- Second-pass packet: [commands_reports_csv.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_csv.rs.review.md:1>)
- Third-pass packet: [commands_reports_csv.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_csv.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/046__src-tauri__src__commands_reports_csv.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://datatracker.ietf.org/doc/html/rfc4180>
  - <https://doc.rust-lang.org/std/primitive.str.html#method.lines>
  - <https://docs.rs/csv/latest/csv/struct.Writer.html>
  - <https://owasp.org/www-community/attacks/CSV_Injection>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It correctly narrows this file back to a CSV serializer boundary instead of an XLSX/artifact/persistence surface.

The live implementation is small and broadly sound, but the tests still under-protect the real contract. [`build_validation_shortlist_csv_text()`](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_csv.rs:18>) emits a fixed 28-column CSV row through a long manual `format!`, while the current tests still split the full output with `str::lines()`. That cannot validate the very newline-bearing CSV records the file claims to support. The file also has several visible encoding choices that should be frozen explicitly: `hard_gate_passed` as `true`/`false` or empty, `legacy_shortlisted` as `1`/`0`, `cross_pareto` as `1`/`0`, and missing optional metrics as empty strings.

## Safe-Now Priority

- Replace `.lines()`-based test assertions with record-aware CSV parsing in tests.
- Add one dense fixture that fills as many optional columns as possible and assert the full parsed row.
- Add negative-path coverage for:
  - invalid payload/model-build failure,
  - zero-candidate failure,
  - shortlist-preferred selection when both shortlist and all-rows are present.
- Add a short contract comment above the serializer documenting the fixed 28-column output and current textual encodings.

## Deferred Work

- Do not migrate the runtime path to `csv::Writer` until output-compatibility fixtures are stronger.
- Do not add spreadsheet-formula mitigation casually; that is a visible semantics change.
- Do not change newline, CRLF, or BOM policy without a concrete interoperability requirement.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_csv -- --nocapture`
- If exported CSV semantics change, rerun report-command and validation-service export coverage
- If visible CSV content changes, do one manual export smoke in a spreadsheet-aware viewer

## Regression And Feature-Preservation Constraints

- Preserve the exact 28-column header order and column count unless downstream consumers move together.
- Preserve shortlist-first, then all-rows fallback behavior.
- Preserve `params_json` as one CSV field.
- Preserve the current textual encoding choices unless a deliberate product decision changes them.

## Final Conclusion

Keep `commands_reports_csv.rs` in `Small surgical refactor`. The highest-value work is test hardening around true CSV-record behavior and full-row contract coverage, not serializer redesign.

