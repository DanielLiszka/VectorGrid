# Final Consolidated Review: `crates/app_core/src/test_support.rs`

- Review date: `2026-04-16`
- Source file: [test_support.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/test_support.rs:1>)
- First-pass packet: [test_support.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/test_support.rs.review.md:1>)
- Second-pass packet: [test_support.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/test_support.rs.review.md:1>)
- Third-pass packet: [test_support.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/test_support.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/005__crates__app_core__src__test_support.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/env/fn.set_var.html>
  - <https://doc.rust-lang.org/std/env/fn.remove_var.html>
  - <https://doc.rust-lang.org/std/sync/struct.Mutex.html#poisoning>
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. The live file is still tiny and test-only, but the best framing is narrower than the first three packets managed: this is a test-time coordination seam around process-global env/state mutation, not just a generic support helper. The neighboring duplicate helper in [gpu_backtest_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs:113>) is part of that story and makes it clear this file is not yet the crate’s singular env-lock owner.

The bucket should stay small. The file does not justify redesign work, but it does justify sharper documentation and verification guidance, and it does not justify blanket poison recovery as an automatic safe-now fix.

## Safe-Now Priority

- Add rustdoc or an internal comment stating that this is a test-only lock for repo-owned process-global env/state mutation.
- Clarify its limits: it reduces interference among known writers, but it does not make environment mutation generally safe under parallel multithreaded test execution.
- If touched, improve the poison panic message for better triage context.
- Do not change poison handling as a drive-by cleanup.

## Deferred Work

- Unify env-lock identity with the sibling helper in `gpu_backtest_kernel.rs`.
- Broader redesign away from raw process-env mutation toward scoped config or restore-on-drop env guards.
- Revisit poison recovery only after guarded state restoration is known to be panic-safe.

## Verification If Changed

- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`
- If CUDA-kernel test helpers are touched too: `cargo test -p ta_app_core --features cuda-backtest-kernel gpu_backtest_kernel::tests:: -- --nocapture --test-threads=1`

## Regression And Feature-Preservation Constraints

- Keep this helper test-only.
- Do not claim that it fully solves `std::env::{set_var, remove_var}` safety on non-Windows multithreaded runs.
- Preserve lock-order assumptions if env-lock unification is attempted later.
- Treat the real risk as test reliability, not runtime correctness or performance.

## Final Conclusion

Keep `test_support.rs` in `Small surgical refactor`. The fourth pass is valid, no wrong fourth-pass file is present, and the best next work is documentation and verification tightening, not behavior change.
