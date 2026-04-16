# Final Consolidated Review: `crates/app_core/src/kvdf_gpu_kernel.rs`

- Review date: `2026-04-16`
- Source file: [kvdf_gpu_kernel.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/kvdf_gpu_kernel.rs:1>)
- First-pass packet: [kvdf_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/kvdf_gpu_kernel.rs.review.md:1>)
- Second-pass packet: [kvdf_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kvdf_gpu_kernel.rs.review.md:1>)
- Third-pass packet: [kvdf_gpu_kernel.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kvdf_gpu_kernel.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/022__crates__app_core__src__kvdf_gpu_kernel.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/nomicon/>
  - <https://doc.rust-lang.org/std/macro.thread_local.html>
  - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>

## Consolidated Judgment

The fourth-pass packet is the right governing review for this file. The first, second, and fourth passes all converge on the same real issue: this module still exposes a safe public evaluator and public transport structs over raw `u64` device pointers whose actual contract is much sharper than the type system expresses. The live source still relies on caller discipline for device affinity, pointer lifetime, row-major layout, and cache coherence, and `validate_inputs()` only proves shape, bounds, and nullness.

No wrong `review_fourth_pass.md` is present in this folder. The file matches the live source and improves materially on the third-pass packet, which stayed too abstract about “ownership/lifetime” without turning that into concrete implementation guidance.

## Safe-Now Priority

- Add rustdoc on `KvdfGpuDeviceSignalMatrices`, `KvdfGpuComboBatch`, and `eval_kvdf_metrics_gpu_kernel_device_rows()` that states:
  - nonzero pointers must be valid device-resident buffers for the selected device/context
  - matrices must be row-major with the advertised row counts and strides
  - `close_device_ptr == 0` triggers host upload from `close_f32`
  - `close_f32` and `close_device_ptr` are both part of the resident-cache contract and must stay coherent and effectively immutable across reuse
  - runtime/cache reuse is thread-local
  - overlays, stop-loss, and take-profit are intentionally unsupported on this route
- Add focused file-local tests for:
  - unsupported overlay / stop-loss / take-profit rejection
  - the documented host-upload fallback path if it can be exercised without brittle runtime coupling
  - the current empty-combo behavior if the team intends to preserve `Ok(Vec::new())` ahead of unsupported-feature rejection
- Keep the current cache and upload-dedup structure intact while adding docs. The reuse logic is intentional and benchmark-sensitive.

## Deferred Work

- Do not narrow the public surface to `pub(crate)`, convert the evaluator to `unsafe`, or replace the raw transports in the same patch. Those are compatibility-facing changes.
- Do not redesign `CloseSeriesKey`, add content hashing, or rework sync/copy behavior without a measured GPU pass.
- Do not touch launch argument order or the CUDA ABI boundary without coordinated review of `double_crossover.cu`.

## Verification If Changed

- `cargo test -p ta_app_core kvdf_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kvdf_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core kvdf_validation_gpu_kernel_walk_forward_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- If transport/runtime semantics change:
  - `cargo test -p ta_app_core kvdf_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kvdf_validation_gpu_kernel_reports_route_diagnostics --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kvdf_validation_gpu_kernel_cscv_matches_cpu --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kvdf_validation_gpu_kernel_reopt_walk_forward_matches_cpu --features cuda,cuda-backtest-kernel -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve resident close/log-return reuse and thread-local per-device runtime reuse.
- Preserve the current host-upload fallback behavior unless the API contract is deliberately redesigned.
- Preserve the current explicit rejection of overlays, stop-loss, and take-profit.
- Do not change cache-key semantics, synchronization behavior, or launch ABI in a docs-first hardening pass.

## Final Conclusion

Keep `kvdf_gpu_kernel.rs` in `Small surgical refactor`, treat the fourth pass as valid, and implement the narrow contract/documentation/tests lane first. Public-surface narrowing and cache/runtime redesign remain real future work, but they are not the safest next step for this batch.
