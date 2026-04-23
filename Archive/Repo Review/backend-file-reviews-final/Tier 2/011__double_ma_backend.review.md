# Final Consolidated Review: `crates/app_core/src/double_ma_backend.rs`

- Review date: `2026-04-15`
- Source file: [double_ma_backend.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_backend.rs:1>)
- First-pass packet: [double_ma_backend.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/double_ma_backend.rs.review.md:1>)
- Second-pass packet: [double_ma_backend.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_backend.rs.review.md:1>)
- Third-pass packet: [double_ma_backend.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_backend.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/011__crates__app_core__src__double_ma_backend.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Safe-now hardening`
- External research anchors:
  - <https://doc.rust-lang.org/std/env/fn.var.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth-pass packet correctly upgrades this file out of `Document only`. The live source proves that `build_strategy_capability_report_for_request` is not dead policy mirror code; it is active preflight logic used by DynamicDoubleMa GPU selection. That makes `cpu_dynamic_supported: true` a behavior-bearing claim, not a harmless placeholder.

The fourth-pass packet is valid as written. The main safe-now target is behavioral truth: do not advertise CPU fallback availability more broadly than the rest of the backend actually supports. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Stop hardcoding `cpu_dynamic_supported: true` in `build_strategy_capability_report_for_request`.
- Compute CPU fallback viability from the same MA CPU-single support rule already used by backend resolution, or move the fallback decision outward so it can use that rule directly.
- Add a targeted regression proving that dynamic GPU preflight only offers CPU fallback when the selected MA pair truly supports CPU-single execution.
- Remove the stale `#[allow(dead_code)]` once the helper remains live.
- Add a cheap boundary test for `top_k = 256` versus `257`.

## Deferred Work

- Do not rename route, capability, or residency labels casually; they already feed serialized results.
- Do not rework the broader residency model in this file alone.
- Do not change the `VECTORBT_ENABLE_GPU_HYBRID_KERNEL` parsing contract in this packet.
- Do not spend review budget removing `_objective` / `_score_mode` placeholders unless a broader caller cleanup is already happening.

## Verification If Changed

- `cargo test -p ta_app_core double_ma_backend -- --nocapture`
- `cargo test -p ta_app_core kernel_preflight_allows_sortino_but_still_flags_large_topk -- --nocapture`
- Re-run the downstream DynamicDoubleMa GPU preflight coverage in `double_ma.rs` if CPU-fallback truth changes.

## Regression And Feature-Preservation Constraints

- Preserve current route-label vocabulary unless all consumers are updated together.
- Preserve the current hybrid env-toggle contract unless a deliberate breaking policy change is approved.
- Preserve the `top_k <= 256` kernel gate unless runtime evidence moves with it.
- Tighten CPU-fallback truth without becoming broader or more pessimistic than the existing backend resolver.

## Final Conclusion

The implementation-facing conclusion is:

- upgrade `double_ma_backend.rs` from `Document only` to `Safe-now hardening`
- treat the fourth pass as the authoritative packet because it correctly identifies a live behavior-bearing truth gap
- fix CPU-fallback truth, remove the stale dead-code suppression, and add focused preflight tests
- defer residency-model cleanup, label/schema churn, and wider policy changes
