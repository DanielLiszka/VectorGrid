# Final Consolidated Review: `src-tauri/src/commands_reports_shared.rs`

- Review date: `2026-04-15`
- Source file: [commands_reports_shared.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_shared.rs:1>)
- First-pass packet: [commands_reports_shared.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_reports_shared.rs.review.md:1>)
- Second-pass packet: [commands_reports_shared.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_shared.rs.review.md:1>)
- Third-pass packet: [commands_reports_shared.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_shared.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/043__src-tauri__src__commands_reports_shared.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with scope correction`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/path/struct.Path.html>
  - <https://doc.rust-lang.org/std/fs/struct.OpenOptions.html>
  - <https://doc.rust-lang.org/std/process/struct.Command.html>
  - <https://doc.rust-lang.org/std/process/index.html>

## Consolidated Judgment

The first three passes were right to keep this file in the smallest-action bucket. This module is mostly shared report-path and desktop-shell plumbing, so broad trust-boundary changes, launcher-policy tightening, or filename-allocation redesigns would be easy to get wrong for little immediate gain.

The fourth pass adds real value, but it is not clean enough to adopt verbatim. It belongs to the correct file, yet it includes prompt-artifact references (`source__...` and `context__...`) and slightly overreaches when it leans toward broader launcher-policy conclusions. The strongest live issues are narrower: `existing_path()` trims before constructing the path, Linux reveal has a one-component relative-path edge, and `suggest_report_path_in_dir()` is advisory rather than collision-proof. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Split `existing_path()` into a whitespace-only rejection check plus exact-path preservation so valid POSIX paths with leading or trailing spaces are not rewritten before validation.
- Harden Linux reveal behavior for one-component relative paths so an empty parent path falls back correctly instead of being treated as a usable directory.
- Add focused tests for `quantile()` edge handling, especially non-finite `q`, empty inputs, and current rounding behavior, while preserving the current semantics unless there is an explicit product decision to change them.
- Treat `suggest_report_path_in_dir()` as an advisory allocator only. If collision-proof behavior is needed, move that guarantee to the eventual writer/reservation point using atomic create semantics rather than trying to make this helper race-free by itself.

## Deferred Work

- Defer any helper-level path allowlisting or broader filesystem trust-boundary redesign.
- Defer Windows process-launch policy changes unless the entire desktop integration surface is reviewed together.
- Defer any attempt to make report-path suggestion itself authoritative for collision safety; that change belongs at the file-creation boundary, not in this helper alone.
- Defer quantile semantic changes beyond edge-case characterization and tests.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app -- --nocapture`
- Add targeted unit tests for `existing_path()`, Linux reveal fallback behavior, and `quantile()` edge cases.
- Add a regression test proving that paths with leading or trailing spaces are either preserved exactly or rejected explicitly, rather than silently normalized.
- If the report-path allocation contract changes, add a test around writer-side atomic create behavior instead of only helper-side probing.

## Regression And Feature-Preservation Constraints

- Preserve accepted report-path behavior for normal Windows and Unix paths.
- Do not break legitimate filenames that contain meaningful whitespace on platforms that allow them.
- Preserve current reveal/open behavior unless a platform-specific failure mode is being fixed explicitly.
- Keep any collision-safety improvement isolated from filename formatting and quantile behavior.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `commands_reports_shared.rs` in `Small surgical refactor`
- fix the path-trimming and Linux relative-parent edges now
- characterize and preserve current `quantile()` behavior unless a separate decision changes it
- move any true collision-proof filename guarantee to the file-creation layer and defer broader launcher-policy redesign
