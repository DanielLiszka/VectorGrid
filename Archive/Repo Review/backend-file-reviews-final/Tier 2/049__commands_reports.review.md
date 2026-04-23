# Final Consolidated Review: `src-tauri/src/commands_reports.rs`

- Review date: `2026-04-16`
- Source file: [commands_reports.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports.rs:1>)
- First-pass packet: [commands_reports.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_reports.rs.review.md:1>)
- Second-pass packet: [commands_reports.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports.rs.review.md:1>)
- Third-pass packet: [commands_reports.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/049__src-tauri__src__commands_reports.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://doc.rust-lang.org/std/fs/fn.write.html>
  - <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/struct.Workbook.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It correctly reframes this file as an artifact-persistence coordinator and wrapper surface, not the primary workbook-layout owner for the active product path.

The most important local risk is not workbook formatting. It is the artifact-backed export path in [`save_report_xlsx_from_artifact_blocking()`](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports.rs:219>), which currently re-enters `save_run_artifact_blocking` after writing the workbook and therefore inherits the full artifact-save side effects of `run_artifacts.rs`. The file also has a thinner but real wrapper drift: the public/save wrappers normalize `max_rows = 0` to `None`, while [`build_report_xlsx_bytes_headless()`](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports.rs:66>) forwards `Option<usize>` unchanged even though its comment says it mirrors save behavior.

## Safe-Now Priority

- Validate artifact kind before artifact-backed export proceeds, and keep support aligned with the accepted run-artifact kinds.
- Lock down the `max_rows` contract and either:
  - make the headless helper mirror the save wrappers,
  - or narrow its documentation so the behavior difference is explicit.
- Add direct regression coverage for:
  - unsupported artifact kind,
  - artifact-backed export success,
  - loadable artifact after `report_xlsx_path` save-back.
- Keep the current save-back side effects explicit in comments/tests if they remain intentionally broad.

## Deferred Work

- Do not replace `save_run_artifact_blocking` with a narrower report-path patch helper unless the project explicitly wants to change persistence semantics.
- Do not spend this pass on legacy-builder extraction or naming cleanup.
- Do not treat workbook formatting fixes in adjacent modules as local work for this file.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`
- If `max_rows` behavior moves, also rerun validation-report workbook coverage
- Manual artifact-backed export smoke:
  - export from saved artifact,
  - confirm workbook is written,
  - confirm artifact still loads afterward

## Regression And Feature-Preservation Constraints

- Preserve the validation-first export path that delegates to `commands_reports_validation_xlsx`.
- Preserve current artifact-to-report passthrough behavior unless consumers move together.
- Keep public command names stable in this pass.
- Be explicit about any change that narrows save-back enrichment or pruning behavior.

## Final Conclusion

Keep `commands_reports.rs` in `Small surgical refactor`. The right next move is to tighten artifact-kind and `max_rows` contract handling while preserving the validation-first wrapper structure and making save-back side effects explicit.

