# Final Consolidated Review: `crates/app_core/src/strategy_gpu_runtime.rs`

- Review date: `2026-04-15`
- Source file: [strategy_gpu_runtime.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_gpu_runtime.rs:1>)
- First-pass packet: [strategy_gpu_runtime.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/strategy_gpu_runtime.rs.review.md:1>)
- Second-pass packet: [strategy_gpu_runtime.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_gpu_runtime.rs.review.md:1>)
- Third-pass packet: [strategy_gpu_runtime.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_gpu_runtime.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/020__crates__app_core__src__strategy_gpu_runtime.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
  - <https://doc.rust-lang.org/std/macro.thread_local.html>
  - <https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html>
  - <https://serde.rs/field-attrs.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth pass is the best packet for this file, but the framing should be normalized. The live code supports defensive hardening on a public, serde-capable boundary more strongly than it supports a claim of a known active bug on a production serialized-ingest path.

The sharper safe-now issues are:

- under-validated incoming `gpu_program_abi` identity
- miss counters whose names overpromise under races
- reset/stats helpers whose scope is easier to overread than the implementation

That still fits a small patch. The right changes are cache-owner hardening and diagnostic honesty, not a redesign of cache ownership or fingerprint production.

## Safe-Now Priority

- Add a local validation gate before cache-key construction and upload for incoming `gpu_program_abi` payloads:
  - reject empty `compiled_plan_fingerprint_hex`
  - reject empty `executable_fingerprint_hex`
  - reject non-current `abi_version` values
- If both `gpu_program` and `gpu_program_abi` are present, consider a small consistency check rather than trusting the pair blindly.
- Keep the current double-checked locking shape, but make miss-counter semantics honest:
  - either increment misses only on actual vacant insertion after the second lock
  - or document/rename them as cold-path/build-attempt counters
- Clarify in docs and final code comments that:
  - compiled-bundle and executable caches are process-global
  - uploaded executables are current-thread local
  - `reset_gpu_strategy_executable_cache()` does not clear uploads across other threads
- If stats/reset reporting is touched, prefer additive poisoned/unknown-state reporting over silently presenting a clean empty cache.
- Treat the upload probe as a partial integrity check, not a full upload-verification story.

## Deferred Work

- Defer stable cross-release fingerprint redesign to `strategy_plan.rs` and `strategy_gpu_program.rs`.
- Defer cross-thread upload-cache invalidation semantics.
- Defer broader `StrategyHandle` ownership/persistence redesign.
- Defer widening the upload probe until the kernel-side probe ABI is reviewed as a shared contract.

## Verification If Changed

- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`
- If CUDA is available and upload logic changes:
  - `cargo test -p ta_app_core strategy_gpu_runtime::tests::uploaded_executable_is_available_after_cache_build --features cuda,cuda-backtest-kernel -- --nocapture --test-threads=1`

## Regression And Feature-Preservation Constraints

- Preserve current behavior for fresh compiler-produced handles.
- Do not implement an in-place “upgrade” or “repair” path for stale ABI payloads inside this cache owner.
- Do not move compilation or upload work under the cache mutex just to make counters cleaner.
- Do not broaden reset semantics to imply cross-thread upload invalidation unless the design is explicitly widened.
- Do not change fingerprint generation in this file alone; if stable digests are required, the producers must move with it.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `strategy_gpu_runtime.rs` at `Small surgical refactor`
- harden incoming ABI identity first
- make miss-counter and cache-scope semantics honest second
- treat poisoned-lock reporting as a smaller additive cleanup after those two items
- defer stable fingerprinting and broader ownership/reset redesign to coordinated cross-file work
