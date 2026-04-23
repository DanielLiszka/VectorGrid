# Final Consolidated Review: `crates/app_core/src/csv_compat.rs`

- Review date: `2026-04-15`
- Source file: [csv_compat.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/csv_compat.rs:1>)
- First-pass packet: [csv_compat.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/csv_compat.rs.review.md:1>)
- Second-pass packet: [csv_compat.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/csv_compat.rs.review.md:1>)
- Third-pass packet: [csv_compat.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/csv_compat.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/010__crates__app_core__src__csv_compat.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with corrections`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/csv/latest/csv/struct.ReaderBuilder.html>
  - <https://doc.rust-lang.org/std/primitive.str.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth-pass packet is the correct final authority because it fixes the biggest review-history error: headerless positional CSV is not actually supported by the live implementation. The loader always consumes the first non-empty record as header metadata, so headerless files lose their first data row silently.

That correction materially changes the final guidance. This file still belongs in `Small surgical refactor`, but the safe-now work is contract truth, tests, and a few bounded parser hardenings rather than generic "preserve tolerant behavior." No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Update rustdoc to describe the real live contract: a header row is expected, positional fallback only works after that first-row probe, ragged rows are accepted, blank optional cells become `NaN`, and `CandleFieldFlags` mean "any finite value seen in the column."
- Add targeted tests for headerless files, UTF-8 BOM on the first header token, fractional timestamps, large float-form timestamps beyond exact `f64` integer range, explicit `NaN`/`inf` optional cells, and partially populated optional columns.
- Include `file_path` in the open-failure error.
- Strip a leading UTF-8 BOM from the first header token before alias matching.
- If one behavior-tightening change is taken now, prioritize exact timestamp acceptance over broader optional-column policy changes.

## Deferred Work

- Do not add heuristic headerless support in this pass.
- Do not change `CandleFieldFlags` from "any finite value seen" to a stricter completeness contract without auditing consumers.
- Do not reject literal `NaN` / `inf` in optional columns without treating it as a compatibility change.
- Do not redesign dialect detection or duplicate/partial header handling as part of this packet.

## Verification If Changed

- `cargo test -p ta_app_core csv_compat -- --nocapture`
- Re-run workspace data-loading coverage if timestamp or optional-field acceptance rules change.
- Recheck at least one end-to-end ingestion path if the loader starts rejecting inputs that were previously accepted through coercion.

## Regression And Feature-Preservation Constraints

- Preserve valid named-header and positional-fallback behavior for supported legacy inputs.
- Do not repeat the incorrect claim that headerless positional CSV is currently supported.
- Treat timestamp exactness and optional non-finite acceptance as compatibility-sensitive input-contract changes.
- Keep performance work out of scope; the real risk is ingest correctness and downstream field-availability semantics.

## Final Conclusion

The implementation-facing conclusion is:

- keep `csv_compat.rs` in `Small surgical refactor`
- treat the fourth pass as the corrective packet that overturns the earlier headerless-support claim
- land docs, edge-case tests, BOM handling, and better open-error context now
- if one stricter parser change is taken, prioritize exact timestamp acceptance
- defer headerless support, stricter optional-column semantics, and broader dialect redesign
