# Final Consolidated Review: `crates/app_core/src/cdmc_gpu_kernel.rs`

- Review date: `2026-04-15`
- Source file: [cdmc_gpu_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc_gpu_kernel.rs:1>)
- First-pass packet: [cdmc_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/cdmc_gpu_kernel.rs.review.md:1>)
- Second-pass packet: [cdmc_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cdmc_gpu_kernel.rs.review.md:1>)
- Third-pass packet: [cdmc_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cdmc_gpu_kernel.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/005__crates__app_core__src__cdmc_gpu_kernel.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/cust/latest/cust/context/struct.CurrentContext.html>
  - <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

This file remains a narrow strategy-local GPU wrapper, and the earlier passes were right not to push it into a larger redesign. The live remaining work is mostly wrapper contract clarity, targeted negative tests, and documentation around CUDA current-context side effects.

The fourth pass is directionally correct and appropriately conservative. The strongest immediate work here is not semantics or cache redesign. It is making the public raw-pointer and current-context behavior explicit and strengthening the negative-test surface around validations that already exist. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Add contract comments on [CdmcGpuDeviceSignalMatrices](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc_gpu_kernel.rs:33>), [CdmcGpuComboBatch](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc_gpu_kernel.rs:51>), and [eval_cdmc_metrics_gpu_kernel_device_rows](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc_gpu_kernel.rs:370>).
- Document the current-context side effect of [CdmcCudaRuntime::new](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc_gpu_kernel.rs:107>) and [set_current](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc_gpu_kernel.rs:133>).
- Add missing negative tests for null pointers, combo-length mismatch, and the pre-launch overlay/stop-loss/take-profit rejection path already enforced in [eval_cdmc_metrics_gpu_kernel_device_rows](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc_gpu_kernel.rs:382>).

## Deferred Work

- Defer narrowing the public API to validated internal types.
- Defer stronger resident close/log-return cache identity or invalidation redesign.
- Defer any “restore previous CUDA context” policy change; if adopted, it should be handled across sibling wrappers consistently.
- Defer genericization with sibling GPU wrappers.

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_gpu_kernel -- --nocapture`
- Add direct regressions for the missing negative cases noted above.
- Re-run representative CDMC GPU smoke or parity tests after doc/comment/test hardening to confirm no behavioral churn.

## Regression And Feature-Preservation Constraints

- Preserve current launch behavior and current-context setup for valid callers.
- Do not change cache reuse semantics in the same patch as docs/tests.
- Keep the public wrapper shape stable while adding only comments and negative tests.
- Avoid widening scope into runtime-policy cleanup.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `cdmc_gpu_kernel.rs` in `Small surgical refactor`
- focus on wrapper contract docs and the missing negative-test coverage
- preserve current runtime and cache behavior for valid callers
- defer public-surface narrowing, cache redesign, and cross-wrapper context-policy changes
