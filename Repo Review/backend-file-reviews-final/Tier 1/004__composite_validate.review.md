# Final Consolidated Review: `crates/app_core/src/composite_validate.rs`

- Review date: `2026-04-14`
- Source file: [composite_validate.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_validate.rs:1>)
- First-pass packet: [composite_validate.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/composite_validate.rs.review.md:1>)
- Second-pass packet: [review__second_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_validate.rs.review.md:1>)
- Third-pass packet: [review__third_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_validate.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/004__crates__app_core__src__composite_validate.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html>

## Consolidated Judgment

The third pass correctly noticed that the public DTO surface is broader than the implementation this file actually honors, and that route-diagnostic handling is biased toward the first candidate family. What it got wrong was the bucket escalation. The fourth pass is substantively right that the file still admits bounded hardening inside this module: replace production panic paths, make route-diagnostic aggregation deterministic, and document the actual local contract. Normalize that conclusion by avoiding any claim that mixed-family CPU batches or wrapper-owned fields are definitively intended forever; the safe final stance is narrower: do not newly reject them without an explicit compatibility review.

## Safe-Now Priority

- Replace the production GPU reopt `panic!` paths with ordinary completeness checks plus `Err(...)` before entering `compute_reopt_walk_forward_stats`. The live panic sites remain real, including the block near [1455](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_validate.rs:1455>).
- Make route-diagnostic handling deterministic across all currently diagnostic-capable families instead of keying it off the first candidate family only. The relevant clear/take helpers are near [327](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_validate.rs:327>) and [342](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_validate.rs:342>).
- Add rustdoc that states the actual core contract for request fields and normalization behavior, especially on the public DTO surface around [9584](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_validate.rs:9584>) and the main entrypoint handling around [12278](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_validate.rs:12278>).
- Add one focused mixed-family route-diagnostic regression test and one truncation-order regression test.

## Deferred Work

- Keep DTO split and field-ownership cleanup deferred until it is handled as coordinated IPC and artifact-compatibility work.
- Do not newly reject mixed-family CPU batches, `cross_ticker`, or wrapper-owned fields at the app-core boundary without explicit compatibility review.

## Verification If Changed

- `cargo test -p ta_app_core composite_validate -- --nocapture`
- Rerun the route/fallback tests in this file that assert `backend_route` and fallback behavior
- If GPU reopt handling changes, rerun the relevant composite GPU validation lanes and compare the route and stats outputs before and after the change

## Regression And Feature-Preservation Constraints

- Panic-to-error conversion must happen before sparse-but-valid intermediate states become indistinguishable from real failure cases.
- Route-diagnostic aggregation must use deterministic ordering to avoid artifact churn.
- Do not reinterpret wrapper-owned fields as newly hard-invalid here.
- Preserve existing successful-run semantics while removing the production panic surfaces.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `composite_validate.rs` at `Small surgical refactor`
- land bounded hardening inside this file now
- defer DTO ownership cleanup and any contract narrowing until wrapper and artifact consumers are in scope
