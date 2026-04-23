# Final Consolidated Review: `crates/app_core/src/standalone_gpu_runtime.rs`

- Review date: `2026-04-16`
- Source file: [standalone_gpu_runtime.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/standalone_gpu_runtime.rs:1>)
- First-pass packet: [standalone_gpu_runtime.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/standalone_gpu_runtime.rs.review.md:1>)
- Second-pass packet: [standalone_gpu_runtime.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/standalone_gpu_runtime.rs.review.md:1>)
- Third-pass packet: [standalone_gpu_runtime.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/standalone_gpu_runtime.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/036__crates__app_core__src__standalone_gpu_runtime.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/cust/latest/src/cust/memory/device/device_buffer.rs.html>
  - <https://docs.rs/cust/latest/src/cust/memory/device/device_slice.rs.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The first three passes were right to keep this file small and to avoid turning it into a runtime-policy rewrite. The fourth pass is also directionally correct, and it is the first pass that puts the emphasis in the right place: this file is mainly about local helper honesty, not cache topology.

No wrong `review_fourth_pass.md` is present in this folder. The fourth-pass packet matches the live file and should govern the final review.

The strongest current issue is the mismatch between the helpers' fallible signatures and their actual panic surface. [`copy_host_to_device_prefix()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/standalone_gpu_runtime.rs:73>) and [`copy_device_to_host_prefix()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/standalone_gpu_runtime.rs:81>) slice the device buffer before copying, so an oversized prefix can panic before returning `Err`. The next real issue is contract visibility: [`ensure_host_f32_len()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/standalone_gpu_runtime.rs:89>) and the device-side grow helper both rely on overwrite-before-read semantics, but the file does not state that locally. Finally, [`CloseSeriesKey`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/standalone_gpu_runtime.rs:10>) is an identity key, not a content key, so cache reuse is only sound while callers treat the underlying series and device pointer as immutable for the cache lifetime or explicitly invalidate on reuse.

## Safe-Now Priority

- Add explicit prefix-length guards in both prefix-copy helpers before any `buf.index(..values.len())` call so oversize requests return `Err(...)` instead of panicking.
- Add concise local contract comments on the host and device grow helpers stating that newly grown regions are uninitialized and must be fully overwritten before any read.
- Add a short contract comment on `CloseSeriesKey` clarifying that it is allocation identity, not content identity, and therefore depends on caller-side immutability or invalidation.

## Deferred Work

- Do not replace `CloseSeriesKey` with content hashing in this pass.
- Do not switch this helper to async allocation or otherwise change stream-order assumptions here.
- Do not reorder the reallocation path in `ensure_device_buffer_len()` without treating that as a deliberate runtime behavior change.

## Verification If Changed

- Re-run the owning CUDA adapter coverage under `--features cuda,cuda-backtest-kernel`.
- Prefer dependent adapter tests over a file-name test filter; this file has no meaningful local test module of its own.
- If the copy helpers change, include a targeted regression covering oversize-prefix error behavior.

## Regression And Feature-Preservation Constraints

- Preserve grow-and-reuse behavior unless a broader runtime review is being done.
- Preserve identity-based cache keys by default; do not introduce O(n) content hashing on the hot reuse path casually.
- Keep success-path changes allocation-free and branch-light.
- Treat this file as runtime correctness and helper-contract code, not as a planner, telemetry, or persistence seam.

## Final Conclusion

Keep `standalone_gpu_runtime.rs` in `Small surgical refactor`. The implementation target is narrow: make the prefix helpers honestly fallible, document the uninitialized scratch contract, and document the immutability assumption behind `CloseSeriesKey`. Larger cache, allocation, or runtime-architecture changes remain deferred.
