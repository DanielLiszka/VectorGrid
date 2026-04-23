# Final Consolidated Review: `crates/app_core/src/dynamic_indicator_adapter.rs`

- Review date: `2026-04-15`
- Source file: [dynamic_indicator_adapter.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/dynamic_indicator_adapter.rs:1>)
- First-pass packet: [dynamic_indicator_adapter.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/dynamic_indicator_adapter.rs.review.md:1>)
- Second-pass packet: [dynamic_indicator_adapter.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/dynamic_indicator_adapter.rs.review.md:1>)
- Third-pass packet: [dynamic_indicator_adapter.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/dynamic_indicator_adapter.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/016__crates__app_core__src__dynamic_indicator_adapter.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/nightly/std/macro.thread_local.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>

## Consolidated Judgment

The fourth-pass packet is valid and should control implementation planning. The live code confirms its strongest point: `PreparedCudaDatasetSignature` is only `len` plus base-vector pointers, and cache rebuild is driven only by that signature. In a repo where loaded datasets can mutate in memory and live bars mutate datasets by `data_id`, that is a plausible stale-buffer correctness hazard, not merely a naming/documentation issue. The fourth pass also correctly narrows the public-surface claim: the unchecked/direct CUDA helper is still sharp, but it is not part of the crate’s flattened re-export surface even though the module itself remains public.

No wrong `review_fourth_pass.md` is present in this folder. The only substantive strengthening needed is to keep the stale-cache issue classified honestly as a deferred invalidation-contract redesign rather than a “small patch fully solves it” item.

## Safe-Now Priority

- Add rustdoc for the module-public sharp edges:
  - `PreparedCudaIndicatorInputs`
  - `compute_dynamic_indicator_cuda`
  - `compute_dynamic_indicator_cuda_with_inputs`
  - `compute_dynamic_indicator_cuda_from_device_matrix_row`
  - `compute_dynamic_indicator_cuda_direct_with_inputs`
  - `take_prepared_cuda_dispatch_stats()`
- Preserve more diagnostic truth when fallback also fails by keeping both device-path and host-path error context rather than only the fallback error text.
- Add tests for:
  - current-thread and reset-on-read semantics of prepared CUDA dispatch stats
  - fallback accounting and error-context preservation
  - device-matrix-row out-of-range rejection
  - any indicator/source case where `first_valid_for_input_kind()` could differ from plain OHLC readiness, if such a descriptor/source combination exists in the current catalog
- If the team wants to reduce casual discovery of the unchecked helper without a SemVer-visible break, prefer `#[doc(hidden)]` and blunt rustdoc over immediate visibility narrowing.

## Deferred Work

- Do not claim the cache invalidation issue is solved unless the invalidation signal becomes stronger than pointer-and-length identity.
- Do not add full-series hashing or other heavy hot-path scans as a quick fix.
- Do not unify the strict-device and fallback-permissive CUDA helper families in the same patch.
- Do not convert validated prepared-input fallback into a hard error without auditing downstream runtime callers first.

## Verification If Changed

- `cargo test -p ta_app_core dynamic_indicator_adapter -- --nocapture`
- `cargo test -p ta_app_core cpu_adapter_computes_single_output_ma_like_indicator -- --nocapture`
- Existing CUDA prepared-input parity tests
- Add and run focused tests for:
  - fallback-stat accounting
  - preserved device error context on dual failure
  - stats helper current-thread/reset-on-read behavior
  - any changed cache invalidation behavior using in-place candle mutation that preserves allocation shape

## Regression And Feature-Preservation Constraints

- Preserve the selective-upload prepared-input design.
- Preserve the validated-vs-unchecked split unless a broader API redesign is intentional.
- Keep fallback observability at least as strong as today.
- Treat any invalidation change as both correctness-sensitive and performance-sensitive.

## Final Conclusion

The implementation-facing conclusion is:

- keep `dynamic_indicator_adapter.rs` in `Small surgical refactor`
- treat the fourth pass as valid and stronger than the third pass
- spend the safe-now budget on contract docs, fallback diagnostics, and targeted tests
- keep the prepared-input invalidation redesign explicitly deferred until the runtime can provide a stronger dataset-revision contract
