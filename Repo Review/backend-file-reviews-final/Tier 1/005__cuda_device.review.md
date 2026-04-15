# Final Consolidated Review: `crates/app_core/src/cuda_device.rs`

- Review date: `2026-04-14`
- Source file: [cuda_device.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cuda_device.rs:1>)
- First-pass packet: [cuda_device.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/cuda_device.rs.review.md:1>)
- Second-pass packet: [review__second_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cuda_device.rs.review.md:1>)
- Third-pass packet: [review__third_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cuda_device.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/005__crates__app_core__src__cuda_device.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>

## Consolidated Judgment

The fourth pass is the authoritative packet for this file. It keeps the correct bucket and correctly narrows the safe-now work to documentation and tests instead of turning cache retry, reset, or unification into a casual cleanup. The live source still exposes two independently sticky caches through [resolve_preferred_cuda_device_id](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cuda_device.rs:12>) and [preferred_cuda_device](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cuda_device.rs:49>), and the lossy helper [preferred_cuda_device_id](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cuda_device.rs:81>) remains a real boundary worth documenting honestly.

## Safe-Now Priority

- Add rustdoc that makes four things explicit:
  - process-lifetime sticky caching
  - CPU-build `Ok(None)` behavior
  - the current “largest total VRAM wins, first enumerated device wins ties” policy
  - the lossy nature of [preferred_cuda_device_id](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cuda_device.rs:81>)
- Add targeted tests that lock the current non-CUDA stub behavior and the current two-cache model, because the file currently exposes two independent sticky truths: cached ID and cached metadata.
- If behavior work is reopened later, redesign the preferred-device story as one authoritative state model rather than trying to patch `OnceLock` piecemeal for retries or resets.

## Deferred Work

- Keep retry/reset/cache-unification changes deferred.
- Keep heuristic or policy changes to device selection deferred unless a compatibility review is explicitly in scope.

## Verification If Changed

- If only docs, comments, or behavior-preserving tests change: `cargo test -p ta_app_core`
- If cache semantics or selection policy change:
  - rerun `cargo test -p ta_app_core`
  - manually smoke the Tauri-side GPU-status surface
  - verify both a CPU-only build and a CUDA-enabled host
  - on multi-GPU hosts, do a targeted device-selection smoke before trusting benchmarks

## Regression And Feature-Preservation Constraints

- Treat any cache-semantics change as user-visible runtime behavior, not as a no-op refactor.
- Remember that selection changes can redirect work to a different GPU and can change context/module-loading behavior.
- Do not casually rename or narrow the lossy helper without reviewing downstream callers that currently treat `None` as an optional-GPU boundary.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `cuda_device.rs` at `Small surgical refactor`
- add precise docs and tests around the current two-cache, sticky-selection model
- defer any retry/reset/cache-unification work until it is designed as a real behavior change
