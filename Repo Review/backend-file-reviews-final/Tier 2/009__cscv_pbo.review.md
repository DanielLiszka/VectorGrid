# Final Consolidated Review: `crates/app_core/src/cscv_pbo.rs`

- Review date: `2026-04-15`
- Source file: [cscv_pbo.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cscv_pbo.rs:1>)
- First-pass packet: [cscv_pbo.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/cscv_pbo.rs.review.md:1>)
- Second-pass packet: [cscv_pbo.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cscv_pbo.rs.review.md:1>)
- Third-pass packet: [cscv_pbo.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cscv_pbo.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/009__crates__app_core__src__cscv_pbo.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with narrowing`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://carmamaths.org/jon/backtest2.pdf>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>

## Consolidated Judgment

The fourth-pass packet is the right final anchor for this file because it corrects the two biggest review-history problems at once: the first pass overstated the `r / N` versus `r / (N + 1)` issue as a clear local bug, while the third pass underweighted the still-live contract and methodology edges. The bucket does not need to change, but the final guidance does.

The correct reading is that this file needs documentation, contract tests, and small observability cleanup now. Methodology rewrites should stay deferred unless the owning validation/report surfaces move together. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Document the live public contract for slice sanitization, skip semantics, deterministic best-effort sampling, tie handling, quantile behavior, and the current Sharpe-from-`sum`/`sumsq` score model.
- Add focused tests that pin the current convention for odd-`N` rank normalization, even-length `p50` quantiles, skipped-path empty candidate vectors, zero-variance score fallback, and deterministic sampling.
- Decouple the hardcoded `"composite_validate_cscv"` progress label from this shared helper.
- Document that `max_combinations = 0` is currently coerced to at least one sampled combination.

## Deferred Work

- Do not switch from `r / N` to `r / (N + 1)` as a local cleanup; that is a validation-methodology change with downstream gate/report impact.
- Do not change quantile method or zero-variance score semantics without coordinated downstream review.
- Do not change skipped-path vector shape unless consumers are audited for schema and positional expectations.
- Do not rewrite the sampling algorithm merely to guarantee exact sample count in this pass.

## Verification If Changed

- `cargo test -p ta_app_core cscv_pbo -- --nocapture`
- Re-run the owning CSCV/PBO validation coverage if any methodology semantics move beyond docs/tests.
- Recheck downstream consumers of `oos_rank_frac_p50`, `oos_metric_p50`, and `lambda_*` before changing normalization, quantiles, or skipped-path tuple shape.

## Regression And Feature-Preservation Constraints

- Preserve deterministic seeded sampling unless a methodology change is explicitly approved.
- Preserve current tie, rank, and quantile behavior unless downstream thresholds and fixtures move together.
- Treat skipped-path vector shape as a real public contract, not an incidental implementation detail.
- Keep performance work out of scope; the live risk is semantics and reporting, not throughput.

## Final Conclusion

The implementation-facing conclusion is:

- keep `cscv_pbo.rs` in `Small surgical refactor`
- treat the fourth pass as the authoritative packet because it corrects both an over-strong first-pass bug claim and an under-strong third-pass dismissal
- land docs, targeted edge-case tests, and careful progress-label decoupling now
- defer rank-normalization, quantile-method, zero-variance-score, and skip-shape changes until the validation/report contract is intentionally revised
