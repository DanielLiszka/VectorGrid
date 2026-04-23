# Final Consolidated Review: `crates/app_core/src/double_ma_validate.rs`

- Review date: `2026-04-14`
- Source file: [double_ma_validate.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_validate.rs:1>)
- First-pass packet: [double_ma_validate.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/double_ma_validate.rs.review.md:1>)
- Second-pass packet: [review__second_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_validate.rs.review.md:1>)
- Third-pass packet: [review__third_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_validate.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/006__crates__app_core__src__double_ma_validate.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Larger refactor but defer`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>

## Consolidated Judgment

The third pass had the right architectural diagnosis: this file exposes a broader public contract than app-core itself fully owns, and a real DTO and field-ownership cleanup remains larger compatibility work. The fourth pass correctly pressure-tested that conclusion and kept the defer bucket while recovering a smaller, local hardening slice. That slice is the right safe-now output: document the semantics the file really owns and add direct tests around them, while keeping cross-layer contract cleanup deferred.

## Safe-Now Priority

- Add rustdoc on the public request/result DTOs and entrypoints clarifying what app-core owns versus what wrapper layers still own, including `validation_context`, cross-ticker execution/population, and timing/reporting semantics.
- Add targeted tests for candidate truncation and execution-profile meaning. The file still normalizes `max_candidates == 0` at [3434](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_validate.rs:3434>), and `ValidationExecutionProfile.requested_candidates` is derived from the post-truncation slice.
- Add direct tests that make finalist-only auxiliary analytics explicit. Optional result fields such as `regime_slice_robustness`, `friction_stress`, `entry_margin_fragility`, `underwater_duration`, `checkpoint_robustness`, and `benchmark_slice_summary` are not populated for every candidate.
- Keep the current tested `cross_ticker` GPU-blocker semantics explicit rather than implied. The blocker remains at [130](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_validate.rs:130>) and the direct fallback test remains an important contract pin.

## Deferred Work

- Keep DTO cleanup and field-ownership cleanup deferred until they are handled as coordinated IPC and artifact compatibility work.
- Do not reinterpret or rename `requested_candidates` in place.
- Do not change `cross_ticker` GPU-blocker behavior or broaden finalist-only analytics to every candidate as part of the safe-now cleanup.

## Verification If Changed

- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- Rerun the direct GPU fallback and `cross_ticker` tests in this file
- If DTO docs or serialization-facing behavior changes, compare representative JSON or artifact outputs before and after the change

## Regression And Feature-Preservation Constraints

- Keep any `requested_candidates` clarification additive rather than mutating the existing field contract.
- Treat `cross_ticker` GPU-blocker behavior as test-pinned. Changing it is route-policy work, not documentation cleanup.
- Keep finalist-only aux analytics behavior stable unless CPU cost and result semantics are deliberately reopened together.
- Do not convert wrapper-owned fields into new hard errors inside app-core without coordinated wrapper changes.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `double_ma_validate.rs` at `Larger refactor but defer`
- land the local docs-and-tests hardening slice now
- keep DTO cleanup and contract narrowing out of scope for a one-file implementation pass
