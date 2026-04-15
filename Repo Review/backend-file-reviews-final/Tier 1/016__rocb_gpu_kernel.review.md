# Final Consolidated Review: `crates/app_core/src/rocb_gpu_kernel.rs`

- Review date: `2026-04-15`
- Source file: [rocb_gpu_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/rocb_gpu_kernel.rs:1>)
- First-pass packet: [rocb_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/rocb_gpu_kernel.rs.review.md:1>)
- Second-pass packet: [rocb_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/rocb_gpu_kernel.rs.review.md:1>)
- Third-pass packet: [rocb_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/rocb_gpu_kernel.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/016__crates__app_core__src__rocb_gpu_kernel.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Benchmark-sensitive; do not touch yet`
- External research anchors:
  - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
  - <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
  - <https://docs.rs/cust/latest/cust/context/struct.Context.html>
  - <https://docs.rs/cust/latest/cust/context/struct.CurrentContext.html>
  - <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
  - <https://doc.rust-lang.org/std/macro.thread_local.html>

## Consolidated Judgment

All four passes converge on the same bucket, and the fourth pass gives the clearest reason: this is not just a strategy-local GPU wrapper, it is also a shared decoder/module/ABI boundary. `ROCB_METRICS_STRIDE`, `decode_non_double_ma_gpu_metrics_row`, and `load_double_crossover_module` tie this file to contracts that live outside the file itself.

The correct near-term posture is to keep the hot path frozen. If this file is touched before a benchmark-backed redesign, the only proportionate scope is contract clarification and host-only validation coverage.

## Safe-Now Priority

- Keep behavior unchanged.
- If the file is touched at all, limit the work to:
  - rustdoc / inline comments for raw pointer, stride, and context assumptions
  - safety comments around the `unsafe` launch and uninitialized-device-buffer assumptions
  - host-only `validate_inputs()` test expansion
- Document explicitly that:
  - raw device pointers must be valid under the same device/current-context regime
  - `channel_stride` and `atr_stride` are element strides
  - `close_f32` still participates in cache identity even when `close_device_ptr != 0`
- Add host-only validation coverage for:
  - null or missing pointer cases
  - close-length mismatch
  - combo-length mismatch
  - invalid zero or too-small strides
  - invalid shape combinations

## Deferred Work

- Defer any change to:
  - shared module loading
  - metrics stride / decoder coupling
  - launch shape
  - scratch-buffer reuse policy
  - resident close/log-return caching
  - feature support or normalization flags
- Defer any attempt to genericize the strategy-local wrapper or rework the device-pointer ownership model without parity and perf evidence.

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel rocb_gpu_kernel::tests:: -- --nocapture`
- If behavior or caller-visible validation changes:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`
- If anything beyond docs/tests changes in the live path:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression And Feature-Preservation Constraints

- Do not remove or weaken `CurrentContext::set_current()`.
- Do not change `ROCB_METRICS_STRIDE` semantics or the shared decoder coupling casually.
- Do not alter scratch reuse, resident-series reuse, or upload-skipping logic without perf validation.
- Do not widen supported ROCB GPU feature coverage at this layer unless kernel semantics are extended and revalidated.
- Treat this file as GPU ABI and parity sensitive first, readability-sensitive second.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `rocb_gpu_kernel.rs` at `Benchmark-sensitive; do not touch yet`
- if touched at all, restrict changes to contract documentation and host-only validation tests
- defer loader, ABI, cache, and launch-shape changes until someone is prepared to revalidate performance and ROCB CPU/GPU parity together
