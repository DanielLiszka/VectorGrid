# Final Consolidated Review: `crates/app_core/src/carm_gpu_kernel.rs`

- Review date: `2026-04-15`
- Source file: [carm_gpu_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/carm_gpu_kernel.rs:1>)
- First-pass packet: [carm_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/carm_gpu_kernel.rs.review.md:1>)
- Second-pass packet: [carm_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/carm_gpu_kernel.rs.review.md:1>)
- Third-pass packet: [carm_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/carm_gpu_kernel.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/003__crates__app_core__src__carm_gpu_kernel.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
  - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/>

## Consolidated Judgment

This file remains a strategy-local CUDA wrapper, and the earlier passes were right not to escalate it into a redesign. The real remaining work is still bounded to wrapper contracts and validation, not strategy-math or runtime-architecture churn.

The fourth pass is the best implementation-facing packet because it identifies the most concrete live invariant that remains unenforced: combo cardinality can exceed `mixed_rows`, which lets the kernel zero-fill unreachable rows instead of failing early on the Rust side. The packet also correctly keeps the public raw-pointer and cache-identity concerns in the documentation-and-validation lane rather than treating them as a same-pass redesign. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Extend [validate_inputs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/carm_gpu_kernel.rs:168>) to reject `combo_count > mixed_rows` before launch.
- Add a direct regression test for that combo-to-row cardinality invariant.
- Document the public raw-pointer expectations on [CarmGpuDeviceSignalMatrices](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/carm_gpu_kernel.rs:33>) and [CarmGpuComboBatch](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/carm_gpu_kernel.rs:51>).
- Clarify that `close_f32` remains part of validation and cache identity even when `close_device_ptr` is supplied, because resident close/log-return reuse is still keyed from the host logical series path.

## Deferred Work

- Defer stronger content-based cache identity or content-version reuse checks.
- Defer typed device-handle redesign or narrowing of the public raw-pointer surface.
- Defer broader cross-thread or cross-runtime cache-policy changes.
- Defer any attempt to genericize this wrapper with sibling GPU strategy wrappers.

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm_gpu_kernel -- --nocapture`
- Add a focused test for `combo_count > mixed_rows`.
- Re-run representative CARM GPU-kernel smoke or parity tests after any validation-only patch.
- Keep one regression proving that valid current callers still reuse resident close/log-return buffers correctly.

## Regression And Feature-Preservation Constraints

- Preserve existing kernel-launch behavior for valid signal matrices and combo batches.
- Do not change cache reuse policy in the same patch as combo-count hardening.
- Keep public wrapper types stable while tightening only validation and docs.
- Avoid introducing device-provenance-heavy checks or runtime churn alongside the simple shape fix.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `carm_gpu_kernel.rs` in `Small surgical refactor`
- add the missing `combo_count <= mixed_rows` guard, a matching regression test, and better wrapper contract docs
- preserve current cache and launch behavior for valid inputs
- defer cache redesign, typed device-handle work, and broader runtime cleanup
