# Final Consolidated Review: `crates/app_core/src/prescan_predictive.rs`

- Review date: `2026-04-16`
- Source file: [prescan_predictive.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/prescan_predictive.rs:1>)
- First-pass packet: [prescan_predictive.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/prescan_predictive.rs.review.md:1>)
- Second-pass packet: [prescan_predictive.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/prescan_predictive.rs.review.md:1>)
- Third-pass packet: [prescan_predictive.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/prescan_predictive.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/032__crates__app_core__src__prescan_predictive.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with narrowing`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://doc.rust-lang.org/std/vec/struct.Vec.html>

## Consolidated Judgment

The first three passes were correct on the big decision: this file is a bounded, crate-private rescue heuristic and should stay in `Small surgical refactor`. The fourth pass improves the review by narrowing attention to concrete semantic edges instead of overemphasizing stricter error behavior or stronger typing.

No wrong `review_fourth_pass.md` is present in this folder. The fourth-pass packet matches the live code and is the right basis for the final review.

## Safe-Now Priority

- Document the real scoring contract where the function is defined:
  - MAs are built from `ma_source`,
  - the target remains next close log return,
  - `long_only` skips bearish bars from the denominator rather than averaging them as zero,
  - time downsampling is approximate rather than a strict `MAX_TIME_STEPS` ceiling.
- Add focused tests for duplicate and unsorted period domains on the common `period_stride <= 1`, `len <= MAX_SAMPLED_PERIODS` path.
  - Today those inputs can pass through without sort/dedup and can therefore emit duplicate `(fast, slow)` pairs.
- If behavior changes, prefer the narrowest uniqueness fix:
  - dedup sampled domains before scoring, or
  - dedup the final emitted `(fast, slow)` output.
- Strengthen tests around the actual ranking contract instead of only asserting lexicographic ordering on one happy-path fixture.

## Deferred Work

- Do not change `Ok(Vec::new())` into broader `Err` behavior in this pass.
- Do not change the approximate time-step cap to strict `div_ceil` style logic without treating it as a scoring and throughput change.
- Do not retune the denominator semantics or the target-return series in a local cleanup pass.
- Do not spend the next pass on type-wrapper churn before the live heuristic semantics are pinned in docs/tests.

## Verification If Changed

- `cargo test -p ta_app_core prescan_predictive::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu_prescan_predictive_rescue_can_expand_candidates -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_supports_multiple_ma_pairs -- --nocapture`
- If time-step or scoring behavior changes:
  - relevant composite optimization workflow or CPU perf lane

## Regression And Feature-Preservation Constraints

- Preserve chronology-respecting next-step evaluation.
- Preserve the file’s role as an additive rescue heuristic rather than a hard-failure gate or heavyweight optimizer.
- Treat duplicate-seed handling as a local hardening problem, not a reason to redesign the heuristic.
- Be explicit if caller-normalized period domains are assumed; otherwise enforce uniqueness locally.

## Final Conclusion

Keep `prescan_predictive.rs` in `Small surgical refactor`, treat the fourth pass as valid, and focus the next pass on documented semantics plus duplicate-domain coverage and uniqueness enforcement. Error-model changes, strict time-cap math, and heuristic redesign remain deferred.
