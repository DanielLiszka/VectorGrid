# Final Consolidated Review: `src-tauri/src/commands_reports_api.rs`

- Review date: `2026-04-16`
- Source file: [commands_reports_api.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_api.rs:1>)
- First-pass packet: [commands_reports_api.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_reports_api.rs.review.md:1>)
- Second-pass packet: [commands_reports_api.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_api.rs.review.md:1>)
- Third-pass packet: [commands_reports_api.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_api.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/042__src-tauri__src__commands_reports_api.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with scope correction`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://v2.tauri.app/develop/calling-rust/#returning-array-buffers>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>

## Consolidated Judgment

The third-pass bucket should stand. This file is still intentionally thin, and nothing in the live source justifies a redesign bucket. The third pass was right that `open_path` and `reveal_path` are public capability surfaces and that validation should remain centralized. The fourth pass improves the final read by correcting scope: this file is also a persistence-sensitive and transport-sensitive IPC boundary, not only a path-launch boundary.

That matters because the strongest file-local work is not structural wrapper churn. It is contract clarity and focused negative-case hardening around the artifact-backed save path. `save_report_xlsx_from_artifact` can mutate saved artifact state through `report_xlsx_path`, and its DTO already encodes partial-success semantics through `artifact_updated` and `artifact_update_warning`. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Add command-level contract documentation or rustdoc explaining:
  - `build_report_xlsx` returns base64-encoded XLSX content as a fallback transport
  - `build_validation_shortlist_csv` returns CSV text, not a saved path
  - `save_report_xlsx_from_artifact` may update persisted artifact metadata and may surface a non-fatal warning
  - `open_reports_dir` reveals the app-local reports directory
  - `open_path` and `reveal_path` are shared shell helpers despite their current module home
- Optionally prefix join-failure strings with command context if wrapper diagnostics are being touched anyway.
- If the adjacent blocking owner is already being edited, add unsupported-artifact negative coverage for artifact-backed save and, if practical, one targeted partial-success test for `artifact_update_warning`.

## Deferred Work

- Defer switching the fallback binary transport to `tauri::ipc::Response`.
- Defer converting the raw `Value` payloads to typed DTOs in isolation.
- Defer relocating `open_path` / `reveal_path` just for module tidiness.
- Defer `tauri-plugin-opener` adoption as a deliberate cross-cutting redesign, not as a local cleanup.
- Defer `SaveReportXlsxResponse` ownership churn unless adjacent report owners are already moving.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`
- If frontend invocation semantics move:
  - `npm --prefix ./web run test -- src/services/validationService.test.ts src/stores/commandActionStore.test.ts src/tauri/commands.test.ts`
- Manual desktop smoke is only needed if shell-launch behavior or path policy changes.

## Regression And Feature-Preservation Constraints

- Preserve current command names and return shapes unless frontend consumers are updated in lockstep.
- Preserve the persistence contract of `save_report_xlsx_from_artifact`, including `artifact_updated`, `artifact_update_warning`, and `report_xlsx_path` write-back behavior.
- Keep path validation centralized in the shared launcher/report owner path.
- Do not silently change `open_reports_dir` from reveal-style behavior to a different UX.
- Treat `build_report_xlsx` as transport-sensitive IPC, not as an internal helper.
- Keep `spawn_blocking` usage bounded and wrapper-local.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `commands_reports_api.rs` in `Small surgical refactor`
- prioritize explicit contract documentation and focused negative-case coverage
- keep path validation centralized and preserve current partial-success persistence semantics
- defer transport redesign, DTO typing, module relocation, and opener-plugin adoption
