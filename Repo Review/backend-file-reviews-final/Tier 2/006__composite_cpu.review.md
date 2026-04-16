# Final Consolidated Review: `crates/app_core/src/composite_cpu.rs`

- Review date: `2026-04-15`
- Source file: [composite_cpu.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_cpu.rs:1>)
- First-pass packet: [composite_cpu.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/composite_cpu.rs.review.md:1>)
- Second-pass packet: [composite_cpu.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_cpu.rs.review.md:1>)
- Third-pass packet: [composite_cpu.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_cpu.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/006__crates__app_core__src__composite_cpu.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://serde.rs/field-attrs.html>
  - <https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html>

## Consolidated Judgment

The earlier passes were right to keep this file in the smallest bucket despite its size. `composite_cpu.rs` is still a public request/result owner, route-policy owner, GPU-assisted helper owner, and analysis/drilldown/local-stability owner. That makes broad cleanup far riskier than targeted guardrail fixes.

The fourth pass correctly reranks the live work. The strongest safe-now issues are not broad route-truthfulness debates. They are a real local-stability base-point hole, missing guardrails on the base drilldown path, and some cheap request-shape rejections that still happen later than they need to. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Force the exact base pair into local-stability neighborhoods even when `step` skips zero, and add targeted tests proving `summary.base` does not silently fall back to `0.0` for valid requests.
- Align base drilldown with the stricter strategy-local drilldown surfaces by enforcing `fast_len < slow_len` and clamping `bins`.
- Move obviously cheap request-shape rejections earlier on the base optimization path where that can be done without changing intended error precedence.
- Add focused regression coverage for non-unit local-stability steps that would otherwise skip the base pair.

## Deferred Work

- Defer rewriting `backend_route` semantics; the split between `backend_route`, `backend_used`, and diagnostics is already pinned by live tests.
- Defer making [resolve_output_id](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_cpu.rs:2113>) exact-only.
- Defer broader analysis-backend semantics cleanup or the host-precompute-heavy GPU route redesign.
- Defer decomposition or service extraction of this file.

## Verification If Changed

- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- Add regressions for local-stability neighborhoods where `step` skips zero.
- Add regressions for base drilldown rejecting invalid MA ordering and clamping bins.
- Re-run representative route-diagnostics and GPU/CPU parity tests after any early-validation adjustment.

## Regression And Feature-Preservation Constraints

- Preserve current route-label semantics and diagnostics contracts.
- Do not widen scope into route-policy relabeling while fixing local-stability and drilldown guardrails.
- Keep valid request behavior and performance-sensitive hot paths intact except where a missing base-point or invalid public input is being corrected.
- Avoid exact-only output-id matching unless the indicator-catalog contract is frozen first.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `composite_cpu.rs` in `Small surgical refactor`
- prioritize the local-stability base-point fix, base drilldown guardrails, and earlier cheap validation
- preserve current route-label and diagnostics semantics
- defer route-policy rewrites, output-id hardening, and broader decomposition
