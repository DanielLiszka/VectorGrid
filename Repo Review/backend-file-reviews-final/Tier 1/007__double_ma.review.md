# Final Consolidated Review: `crates/app_core/src/double_ma.rs`

- Review date: `2026-04-14`
- Source file: [double_ma.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs:1>)
- First-pass packet: [double_ma.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/double_ma.rs.review.md:1>)
- Second-pass packet: [review__second_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma.rs.review.md:1>)
- Third-pass packet: [review__third_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/007__crates__app_core__src__double_ma.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`

## Consolidated Judgment

The third and fourth passes converge on the right shape for this file: keep the bucket small, but narrow the implementation priority to contract-truthfulness and verification hardening. The best next work is not speculative cache redesign. It is preserving explicit caller intent on backend/device normalization, making follow-up DTO behavior truthful, and documenting what the cache-related fields actually mean.

## Safe-Now Priority

- Preserve explicit non-`Auto` GPU device IDs through normalization. The normalization path begins at [with_resolved_preferred_cuda_device](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs:125>) and is used in both optimization and follow-up flows.
- Make follow-up MA-param resolution truthful without changing DTO shape. The file already carries [DoubleMaParamsResolved](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs:441>), so follow-up consumers should prefer top-level MA params when present and only fall back to nested resolved params when they are not.
- Document [persist_followup_cache](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs:322>) and `followup_cache_key` as process-local, best-effort acceleration hints rather than durable persisted artifacts.
- Clarify cache/reset semantics and misleading names. [reset_dynamic_gpu_preflight_cache](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs:51>) clears more than strictly preflight state, and the `oldest_key` naming around `HashMap` eviction should not imply a stronger ordering guarantee than the code can prove.

## Deferred Work

- Keep follow-up cache-identity strengthening deferred unless it is backed by a real dataset-version token or content fingerprint, not a weak heuristic.
- Keep dynamic GPU batch-cache eviction or automatic clearing deferred behind representative GPU perf gates.
- Do not tighten drilldown or other diagnostic APIs just to mirror optimization-time invariants without clear evidence.

## Verification If Changed

- `cargo test -p ta_app_core double_ma -- --nocapture`
- Rerun the exact-route and fallback tests that assert `GpuKernel`, `GpuHybridKernel`, and fallback route strings
- If follow-up cache-key semantics change, add a direct cache miss/recompute regression test and rerun representative drilldown/local-stability flows
- If cache eviction or GPU batch-cache behavior changes, run the relevant perf gate before trusting the cleanup

## Regression And Feature-Preservation Constraints

- Do not change `Backend` serde shape casually; it is an IPC surface.
- Do not “fix” cache identity with a weak heuristic that can create false confidence instead of correctness.
- Treat cache and eviction changes as performance-sensitive GPU-runtime work.
- Preserve explicit caller device intent rather than silently rewriting all GPU-bearing variants toward the preferred device.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `double_ma.rs` at `Small surgical refactor`
- prioritize device-intent preservation, truthful follow-up DTO behavior, and honest cache documentation
- defer cache-identity and eviction redesign until it can be justified with correctness and perf evidence
