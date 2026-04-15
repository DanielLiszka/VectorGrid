# Final Consolidated Review: `crates/app_core/src/gpu_backtest_kernel.rs`

- Review date: `2026-04-14`
- Source file: [gpu_backtest_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs:1>)
- First-pass packet: [gpu_backtest_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/gpu_backtest_kernel.rs.review.md:1>)
- Second-pass packet: [gpu_backtest_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/gpu_backtest_kernel.rs.review.md:1>)
- Third-pass packet: [gpu_backtest_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/gpu_backtest_kernel.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/008__crates__app_core__src__gpu_backtest_kernel.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
  - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/>

## Consolidated Judgment

All four passes converge on the same bucket, and the fourth pass is the best implementation-facing packet for this file. The live risk is not that the kernel runtime needs structural redesign right now. The live risk is that a few Rust-side uploaded-batch contracts remain looser than the executor path they eventually feed, which can turn malformed geometry or stale assumptions into opaque runtime failures. The safe-now work is therefore contract hardening and clearer diagnostics around the uploaded dynamic path, not stream/layout churn or scratch-retention cleanup.

## Safe-Now Priority

- Tighten the uploaded dynamic-batch geometry contract in [upload_double_ma_dynamic_program_batch](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs:2610>):
  - reject `first_valid >= lr.len()`
  - reject uploaded MA matrix widths that do not match the return-series length
  - reject period or combo counts that later overflow the kernel-facing `i32` argument shape
- Document the real helper contract instead of implying more than the current code does. The uploaded batch is device-specific, upload does not freeze strategy semantics, and the current uploaded execution path still routes through the fixed executor family rather than a fully general uploaded predicate-program kernel.
- Preserve and improve loader/runtime diagnostics rather than collapsing them into generic errors.
- Add negative tests for invalid uploaded geometry before attempting any compaction or scratch reuse changes.

## Deferred Work

- Keep uploaded-path compaction, scratch retention changes, and stream/pinned-memory tuning deferred until they are benchmark-backed.
- Treat any attempt to change uploaded dynamic-handle semantics or broaden the runtime contract as design work, not as cleanup.

## Verification If Changed

- `cargo test -p ta_app_core gpu_backtest_kernel -- --nocapture`
- Rerun representative `double_ma` GPU route tests that exercise uploaded batch preparation and fallback reporting
- If any runtime or staging behavior changes, run the GPU perf gate before trusting the result

## Regression And Feature-Preservation Constraints

- Tightening validation can reject callers that currently rely on accidental permissiveness, so negative tests should lead the change.
- Do not narrow the public helper story so aggressively that current probe/runtime callers lose compatibility without an explicit audit.
- Treat stream, pinned-buffer, launch-shape, and scratch-retention edits as performance-sensitive GPU-runtime work.
- Do not mix contract hardening with combo-row decode or duplicate-combo semantics changes.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `gpu_backtest_kernel.rs` at `Small surgical refactor`
- harden uploaded geometry validation and helper documentation now
- keep runtime/layout/perf work out of scope for this implementation pass
