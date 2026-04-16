# Final Consolidated Review: `crates/app_core/src/validation_pipeline.rs`

- Review date: `2026-04-15`
- Source file: [validation_pipeline.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_pipeline.rs:1>)
- First-pass packet: [validation_pipeline.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/validation_pipeline.rs.review.md:1>)
- Second-pass packet: [validation_pipeline.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_pipeline.rs.review.md:1>)
- Third-pass packet: [validation_pipeline.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_pipeline.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/022__crates__app_core__src__validation_pipeline.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Larger refactor but defer`
- External research anchors:
  - <https://doc.rust-lang.org/std/option/enum.Option.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf>

## Consolidated Judgment

The fourth pass is the right final packet for this file. The earlier packets correctly treated `validation_pipeline.rs` as shared validation infrastructure whose real risk is not local control-flow polish, but shared contract semantics. The live file still mixes two candidate identities inside one run summary: the top-ranked candidate derived from `out.first()` and the first approved candidate derived later in the verdict path. That is the strongest reason the file remains in `Larger refactor but defer`.

The fourth pass also sharpened two bounded but real issues that are worth preserving in the final guide. First, SPA `None` still conflates "not requested," "invalid/unsupported," and "cancelled/aborted" states. Second, `sample_obs.get(i).copied().unwrap_or(2).max(2)` silently turns a caller-shape mismatch into a two-observation assumption. Both are real weaknesses, but neither is a clean file-local refactor because the surrounding validation payloads are serialized and consumed across commands, reports, and saved artifacts.

## Safe-Now Priority

- If this file is touched before a broader redesign, prefer characterization tests over semantic edits.
- Add a regression test for the "rank 1 rejected, later candidate approved" path so the current top-ranked-versus-approved mismatch is pinned explicitly.
- Add a regression test for short or mismatched `sample_obs` so the current fallback-to-`2` behavior is visible and deliberate.
- Add narrow contract notes or debug-only assertions around caller-seeded probability metrics and `robust_score` assumptions if the file is already open for nearby work.

## Deferred Work

- Do not redesign `ValidationRunVerdict` as a small patch. Separating top-ranked-candidate summary from approved-candidate summary is a shared serialized-contract change.
- Do not replace SPA `Option` state with a richer status model without auditing Tauri payloads, reports, saved artifacts, and frontend consumers together.
- Do not reinterpret optional walk-forward, CSCV, or SPA statistics locally unless the corresponding consumer semantics are reviewed in the same pass.

## Verification If Changed

- `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`
- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- If verdict fields, serialized shapes, or report-facing semantics change, widen verification to report and artifact consumers before rollout.

## Regression And Feature-Preservation Constraints

- Preserve current validation behavior unless a contract weakness is being fixed deliberately and across all consumers.
- Treat SPA state expansion as shared-contract work, not a local enum cleanup.
- Preserve current robust-score ordering and candidate sorting unless downstream ranking and recommendation semantics are revalidated together.
- Keep the current indexed parallel-collection behavior unless there is a proven ordering bug, not a speculative cleanup instinct.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `validation_pipeline.rs` at `Larger refactor but defer`
- preserve the fourth-pass diagnosis around verdict identity mixing and SPA ambiguity
- allow only narrow characterization tests or debug-only contract notes ahead of any broader redesign
- defer any payload-shape or recommendation-semantics rewrite until shared validation consumers are audited together
