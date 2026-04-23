# Final Consolidated Review: `src-tauri/src/commands_reports_validation_common.rs`

- Review date: `2026-04-15`
- Source file: [commands_reports_validation_common.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_validation_common.rs:1>)
- First-pass packet: [commands_reports_validation_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_reports_validation_common.rs.review.md:1>)
- Second-pass packet: [commands_reports_validation_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_validation_common.rs.review.md:1>)
- Third-pass packet: [commands_reports_validation_common.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_validation_common.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/044__src-tauri__src__commands_reports_validation_common.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Targeted hardening now; larger redesign still deferred`
- External research anchors:
  - <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
  - <https://docs.rs/serde_json/latest/serde_json/value/>
  - <https://doc.rust-lang.org/stable/std/primitive.slice.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The first three passes were correct to defer any broad redesign here. This file is a large report-model assembly surface with mixed-schema compatibility logic, sorting policy, nested row projections, and report-shape expectations that other layers likely already depend on.

The fourth pass is the first packet that identifies enough bounded correctness work to justify action without reopening the broader architecture. Two issues stand out: nested `regime_bucket_rows` presentation metadata can drift out of sync with the final outer-row ordering and approval state, and `parse_run_verdict()` can treat a partially populated object-shaped verdict as an authoritative rejection because missing booleans default to `false`. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Re-synchronize nested `regime_bucket_rows` presentation metadata after the final outer ordering is established and after any run-level rejection rewrite changes the outer approval state.
- Tighten `parse_run_verdict()` so placeholder or partially populated object-shaped verdicts are treated as absent or incomplete rather than silently becoming a hard rejection.
- Add targeted tests for:
  - nested regime-bucket ranking and approval metadata after sorting
  - nested metadata after run-level rejection rewrites
  - `run_verdict` values that are missing `passed` and `approved`
  - mixed legacy/current report rows where only some inputs carry newer verdict fields
- Keep the immediate patch local to metadata synchronization and verdict parsing. Do not mix it with schema redesign or cross-report ordering changes.

## Deferred Work

- Defer mixed-schema compatibility redesign while `compatibility_mode` remains a run-wide concept and row completeness is still heterogeneous.
- Defer broader shortlist unification and regime-support canonicalization.
- Defer plateau-ordering policy changes until the team decides whether plateau enrichment is presentation-only or ranking-relevant.
- Defer report-model extraction or typed DTO cleanup until characterization coverage is stronger.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app -- --nocapture`
- Add focused tests covering nested bucket metadata before and after outer-row rejection rewrites.
- Add parsing tests for absent, partial, and fully populated `run_verdict` objects.
- Re-run any report-export or validation-report smoke tests that consume these assembled rows to confirm JSON shape stability.

## Regression And Feature-Preservation Constraints

- Preserve the current public report JSON shape unless a dedicated schema migration is approved.
- Do not change top-level row ordering rules in the same patch as nested-metadata synchronization.
- Do not reinterpret legacy rows as rejected solely because newer verdict fields are absent.
- Keep mixed-schema compatibility behavior stable except where it is necessary to stop false rejection or stale nested labels.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- move `commands_reports_validation_common.rs` to `Targeted hardening now; larger redesign still deferred`
- fix nested regime-bucket metadata drift and partial-object `run_verdict` rejection now
- add characterization coverage around sort/rewrite interactions and mixed-schema verdict parsing
- defer compatibility redesign, plateau-policy changes, and report-model extraction
