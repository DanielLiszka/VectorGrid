# Final Consolidated Review: `crates/app_core/src/kqsr_gpu_kernel.rs`

- Review date: `2026-04-15`
- Source file: [kqsr_gpu_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/kqsr_gpu_kernel.rs:1>)
- First-pass packet: [kqsr_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/kqsr_gpu_kernel.rs.review.md:1>)
- Second-pass packet: [kqsr_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kqsr_gpu_kernel.rs.review.md:1>)
- Third-pass packet: [kqsr_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kqsr_gpu_kernel.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/020__crates__app_core__src__kqsr_gpu_kernel.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/cust/latest/i686-unknown-linux-gnu/cust/context/index.html>
  - <https://doc.rust-lang.org/nightly/std/macro.thread_local.html>
  - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/>

## Consolidated Judgment

The fourth-pass packet is valid and is the best implementation guide for this file. The live source confirms every key sharpening it makes: the safe public entrypoint still accepts raw `u64` device pointers, still changes the calling thread’s current CUDA context via `rt.set_current()`, still requires a host `close_f32` mirror even when a device close pointer is supplied, still treats `close_device_ptr == 0` as a special host-upload fallback, and still short-circuits empty combo batches before unsupported-feature rejection. The crate root also confirms that this module is part of a broader public pattern of strategy-local GPU wrappers, which makes one-off visibility narrowing a larger policy change than the third pass suggested.

No wrong `review_fourth_pass.md` is present in this folder. The extra strengthening needed is to call out that assertion-only hardening cannot solve raw-pointer provenance, so safe-now work should stay in documentation, tests, and testability helpers.

## Safe-Now Priority

- Add rustdoc to the public structs and entrypoint that makes the existing contract explicit:
  - raw device-pointer inputs
  - current-context side effect on the calling thread
  - `close_device_ptr == 0` host-upload fallback
  - required host `close_f32` mirror even in device-pointer mode
  - resident close/log-return cache identity assumptions
- Add targeted tests for:
  - `close_device_ptr == 0` being accepted while genuinely required signal pointers are rejected
  - combo-vector length mismatch
  - unsupported-feature rejection with a non-empty combo batch
  - empty-combo success, if that ordering is intentional
- If this file is going to gain cache/runtime tests, add a file-local `#[cfg(test)]` reset helper for `KQSR_GPU_RUNTIME` and serialize those tests with the existing GPU test lock.

## Deferred Work

- Do not narrow visibility or convert the public function to `unsafe` in a one-file patch.
- Do not redesign the raw-pointer transport surface in the same change; that is the real way to enforce provenance, but it is not small.
- Do not change context restoration, stream topology, or host-terminal decode behavior without explicit benchmark and interop review.

## Verification If Changed

- `cargo test -p ta_app_core validate_inputs_rejects_ --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core checked_i32_rejects_overflow --features cuda,cuda-backtest-kernel -- --nocapture`
- Add and run targeted tests for:
  - `close_device_ptr == 0`
  - combo-length mismatch
  - unsupported-feature rejection with non-empty combos
  - empty-combo success if it remains part of the contract
- If runtime or pointer-transport behavior changes, also run:
  - `cargo test -p ta_app_core kqsr_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kqsr_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kqsr_validation_gpu_kernel_reports_route_diagnostics --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kqsr_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve the kernel ABI surface and current metric decode stride.
- Preserve the `close_device_ptr == 0` fallback unless the API is being intentionally redesigned.
- Do not reorder the empty-combo and unsupported-feature checks casually; both orderings are observable.
- Preserve thread-local runtime reuse, resident close/log-return caching, and upload scratch reuse unless they are being re-measured deliberately.

## Final Conclusion

The implementation-facing conclusion is:

- keep `kqsr_gpu_kernel.rs` in `Small surgical refactor`
- treat the fourth pass as valid and more complete than the third pass
- keep safe-now work limited to contract docs, targeted tests, and optional test-reset support
- defer visibility changes, `unsafe` signature changes, transport redesign, and runtime-topology rewrites
