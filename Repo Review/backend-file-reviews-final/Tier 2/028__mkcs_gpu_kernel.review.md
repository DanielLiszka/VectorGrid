# Final Consolidated Review: `crates/app_core/src/mkcs_gpu_kernel.rs`

- Review date: `2026-04-16`
- Source file: [mkcs_gpu_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/mkcs_gpu_kernel.rs:1>)
- First-pass packet: [mkcs_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/mkcs_gpu_kernel.rs.review.md:1>)
- Second-pass packet: [mkcs_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/mkcs_gpu_kernel.rs.review.md:1>)
- Third-pass packet: [mkcs_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/mkcs_gpu_kernel.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/028__crates__app_core__src__mkcs_gpu_kernel.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/nomicon/>
  - <https://doc.rust-lang.org/std/macro.thread_local.html>
  - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
  - <https://docs.rs/cust/latest/cust/>

## Consolidated Judgment

The fourth-pass packet is the right implementation-facing review for this file. As with the KVDF wrapper, the live issue is not MKCS math; it is the underdocumented public raw-handle contract. The module is still publicly exported, the evaluator is still a safe public function, and the transport structs still expose raw `u64` device pointers whose real invariants are much stronger than the type surface.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live source and materially improves on the third-pass abstraction-only framing.

## Safe-Now Priority

- Add rustdoc on `MkcsGpuDeviceSignalMatrices`, `MkcsGpuComboBatch`, and `eval_mkcs_metrics_gpu_kernel_device_rows()` that states:
  - nonzero pointers must be valid device buffers for the selected device/context
  - the matrices must be row-major with the advertised row counts and strides
  - `close_device_ptr == 0` triggers host upload from `close_f32`
  - when both host and device close-series inputs are present, callers must keep them coherent and effectively immutable across cache reuse
  - runtime reuse is thread-local rather than process-global
  - overlays, stop-loss, and take-profit are intentionally unsupported on this route
- Add focused tests for:
  - unsupported overlay / stop-loss / take-profit rejection
  - the current empty-combo behavior if the team intends to preserve the early `Ok(Vec::new())` path
  - a host-upload fallback path if it can be exercised without brittle driver coupling
- Keep cache, upload-dedup, synchronization, and launch-ABI behavior unchanged while adding docs/tests.

## Deferred Work

- Do not narrow visibility, convert the evaluator to `unsafe`, or replace the raw transport types in the same patch.
- Do not redesign `CloseSeriesKey`, add explicit cache invalidation, or rework runtime topology in the same batch.
- Do not change synchronization, output materialization, or launch argument order without coordinated CUDA and perf review.

## Verification If Changed

- `cargo test -p ta_app_core mkcs_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core mkcs_gpu_kernel_request_runs_when_cuda_available --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core mkcs_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core mkcs_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- If cache/runtime/sync behavior changes:
  - `cargo test -p ta_desktop_demo_app cross_ticker_mkcs_gpu_kernel_smoke_applies_stats --features cuda,cuda-backtest-kernel -- --nocapture`
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression And Feature-Preservation Constraints

- Preserve host-upload fallback and explicit unsupported-feature rejection.
- Preserve thread-local runtime reuse, resident-series reuse, and upload-signature dedup.
- Do not alter the empty-combo early return unless the intended capability-check semantics are changed deliberately.
- Treat visibility narrowing and API-shape changes as compatibility work.

## Final Conclusion

Keep `mkcs_gpu_kernel.rs` in `Small surgical refactor`, treat the fourth pass as valid, and implement the public-contract docs plus narrow host-side tests first. API-surface narrowing, cache redesign, and runtime topology work remain deferred.
