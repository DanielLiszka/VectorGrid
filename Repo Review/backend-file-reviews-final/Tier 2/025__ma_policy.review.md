# Final Consolidated Review: `crates/app_core/src/ma_policy.rs`

- Review date: `2026-04-16`
- Source file: [ma_policy.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/ma_policy.rs:1>)
- First-pass packet: [ma_policy.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/ma_policy.rs.review.md:1>)
- Second-pass packet: [ma_policy.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ma_policy.rs.review.md:1>)
- Third-pass packet: [ma_policy.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/ma_policy.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/025__crates__app_core__src__ma_policy.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/reference/input-format.html>
  - <https://doc.rust-lang.org/std/macro.include_str.html>
  - <https://www.rfc-editor.org/rfc/rfc4180.html>

## Consolidated Judgment

The fourth-pass packet is the best implementation guide for this file. The earlier passes correctly kept the bucket small, but the fourth pass sharpened the real safe-now issues: inconsistent normalization across the public helpers, fragile first-row CSV header detection, and a public “GPU preferred” bit whose semantics are broader than direct kernel-route eligibility.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live source.

## Safe-Now Priority

- Normalize helper inputs consistently:
  - trim in `supports_gpu_kernel_ma`
  - use the normalized identifier on both branches of `prefers_gpu_kernel_ma`, including the static fallback branch
- Harden `parse_gpu_speedup_ids()` without changing the policy model:
  - inspect the first non-empty parsed row instead of raw `line_idx == 0`
  - compare the parsed first field to `indicator_id`
  - strip a leading BOM from that first field before the comparison
- Update docs and/or helper rustdoc so downstream callers understand that:
  - `gpu_preferred_by_speedup` is an advisory preference bit
  - it is not identical to “kernel-route eligible”
  - non-kernel-capable IDs can still be speedup-preferred without producing `HybridRouteTier::GpuKernel`
  - partition outputs are normalized and deterministically ordered
- Add invariant tests for:
  - whitespace/case normalization consistency
  - BOM / quoted-header / leading-blank-line parsing cases
  - preferred-list subset and supported-list registry membership
  - non-kernel speedup-preferred IDs still routing to `CpuStaged`
- Update the stale `vector-ta 0.1.2` comment so the provenance story is current.

## Deferred Work

- Do not rename or remove `prefers_cpu_by_speedup_table` in the same patch unless a consumer audit says it is truly internal.
- Do not narrow public visibility of the route structs or planners as part of the same change.
- Do not add a broad “partial table fallback to static list” behavior that could hide a broken embedded artifact.
- Do not prioritize new registry caching over the semantic fixes above.

## Verification If Changed

- `cargo test -p ta_app_core ma_policy::tests:: -- --nocapture`
- `cargo test -p ta_app_core capability_report_marks_gpu_supported_for_plain_sma_request -- --nocapture`
- If route-selection semantics change:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel double_ma_validation_gpu_kernel_supported_full_stack_preserves_route -- --nocapture`
  - `cargo test --workspace`

## Regression And Feature-Preservation Constraints

- Preserve current route decisions for known kernel-capable IDs unless a deliberate policy change is being made.
- Preserve deterministic partition ordering and dedup behavior.
- Treat helper renames and visibility narrowing as compatibility work.
- Do not turn a parser hardening pass into a hidden policy-source switch.

## Final Conclusion

Keep `ma_policy.rs` in `Small surgical refactor`, treat the fourth pass as valid, and implement the normalization/header-parsing/docs/tests lane first. Public helper renames, visibility changes, and broader policy-source redesign remain deferred.
