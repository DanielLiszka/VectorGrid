# Final Consolidated Review: `crates/app_core/src/composite_gpu_metric_decode.rs`

- Review date: `2026-04-15`
- Source file: [composite_gpu_metric_decode.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_gpu_metric_decode.rs:1>)
- First-pass packet: [composite_gpu_metric_decode.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/composite_gpu_metric_decode.rs.review.md:1>)
- Second-pass packet: [composite_gpu_metric_decode.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_gpu_metric_decode.rs.review.md:1>)
- Third-pass packet: [composite_gpu_metric_decode.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_gpu_metric_decode.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/007__crates__app_core__src__composite_gpu_metric_decode.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/primitive.slice.html>
  - <https://doc.rust-lang.org/std/macro.debug_assert_eq.html>
  - <https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast>
  - <https://docs.rs/cust/latest/cust/>

## Consolidated Judgment

The bucket remains right, but the packet history needs one explicit correction: the third-pass packet was substantively mis-targeted and appears to describe a different class of file. The fourth-pass packet is valuable because it reanchors the review in the live decoder contract instead of cache/planner/runtime concepts that this file does not even contain.

The live file is a tiny, feature-gated GPU metric-row decoder. The real safe-now work is low-risk contract clarification and debug/test hardening around the row-shape semantics already encoded in the code. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Document the real row contract in [decode_non_double_ma_gpu_metrics_row](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_gpu_metric_decode.rs:31>): a 12-field required prefix plus a 7-field optional tail, not a strict `len == 19` runtime requirement.
- Add debug/test-only assertions around impossible or unexpected row shapes so drift is visible during development without changing runtime policy.
- Add tests that lock the current optional-tail zero-default behavior and the current placeholder-zero treatment of trade-distribution fields.
- Keep host-side recomputation limited to what the decoder already does today, including Martin-ratio derivation.

## Deferred Work

- Defer converting the decoder to `Result<Metrics, _>` in isolation.
- Defer changing the runtime contract to strict `row.len() == 19` unless producer and consumer are changed together.
- Defer changing float-to-`u32` coercion semantics here alone.
- Defer broader host-side recomputation or normalization changes that would create drift relative to [gpu_backtest_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs:1>).

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_gpu_metric_decode -- --nocapture`
- Add direct tests for required-prefix handling, optional-tail defaulting, Martin-ratio derivation, and placeholder-zero trade fields.
- Re-run sibling non-double-MA GPU wrapper decode tests if the row contract comment or assertions are adjusted.

## Regression And Feature-Preservation Constraints

- Preserve current runtime behavior for rows with the required 12-field prefix.
- Do not silently hard-fail shorter rows that currently rely on optional-tail defaulting.
- Do not broaden decode semantics beyond the current host-side normalization contract.
- Keep this helper aligned with the nearby generic decoder family unless there is a coordinated cross-decoder redesign.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `composite_gpu_metric_decode.rs` in `Small surgical refactor`
- treat the fourth pass as the corrective packet that supersedes a mis-targeted third pass
- document the real prefix-plus-optional-tail row contract and harden it in debug/tests only
- defer runtime error-surface changes, stricter row-length policy, and decode-family semantic rewrites
