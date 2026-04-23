# Final Consolidated Review: `crates/app_core/src/strategy_dynamic.rs`

- Review date: `2026-04-15`
- Source file: [strategy_dynamic.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_dynamic.rs:1>)
- First-pass packet: [strategy_dynamic.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/strategy_dynamic.rs.review.md:1>)
- Second-pass packet: [strategy_dynamic.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_dynamic.rs.review.md:1>)
- Third-pass packet: [strategy_dynamic.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_dynamic.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/018__crates__app_core__src__strategy_dynamic.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Safe-now hardening; larger redesign still deferred`
- External research anchors:
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
  - <https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>

## Consolidated Judgment

The fourth pass correctly changes the bucket. This file still carries real structural debt, but the next move should not be “wait for a future rewrite.” There are bounded hardening items available now that improve contract accuracy and failure behavior without reopening the whole planner/runtime split.

What needs normalization is scope. The strongest issues are concrete and current: preview identity naming overstates what it identifies, the product-cap gate is broader than runnable GPU-kernel eligibility, and a fallible public path still contains an internal `expect(...)`. The slot-order issue should remain framed as a proof/documentation task until upstream validation guarantees are explicitly checked.

## Safe-Now Priority

- Clarify that `DynamicDoubleMaStrategyIrPreview.plan_fingerprint_hex` is currently IR identity, not resolved compiled-plan identity. Prefer additive clarification or an added field over an in-place rename.
- Remove the hidden panic edge from `prepare_runnable_dynamic_double_ma_plan(...)`. If a resolved GPU preview unexpectedly lacks a GPU handle, return `Err(...)` rather than reconstructing and `expect`ing.
- Separate product-surface caps from runnable GPU-kernel eligibility in docs, tests, and error text. The broad gate and the narrow `validate_dynamic_double_ma_gpu_handle_v1(...)` contract are not the same thing.
- Add parity coverage between `eval_dynamic_plan_metrics_cpu_with_candles(...)` and `eval_dynamic_plan_metrics_cpu(...)` before changing evaluator behavior.
- Treat the fast/slow slot-order issue as a contract-proof task:
  - either document/assert the upstream guarantee
  - or switch the seeded-entry logic to resolve by kind/name after parity is established
- Document current diagnostic-truth limits where useful:
  - planned route versus actual fallback route
  - shadow-equivalence checks covering only a subset of metrics

## Deferred Work

- Defer module extraction and broader cleanup across preview, staging, telemetry, and evaluation ownership.
- Defer unifying the two Double-MA CPU evaluators into one engine until parity and failure-semantics tests are in place.
- Defer redesign of the staging progress callback to report actual fallback route/cause unless the public callback contract is intentionally widened.
- Defer broader product/backend-policy reshaping until frontend preview, command, and request-builder expectations are updated together.

## Verification If Changed

- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`
- If evaluator semantics change, add direct parity tests between the two Double-MA CPU entrypoints on the same plan and candles.

## Regression And Feature-Preservation Constraints

- Do not rename or reinterpret `plan_fingerprint_hex` in place.
- Do not broaden or reorder product-cap enforcement casually; it is part of the current preview contract.
- Preserve the lazy CPU fallback and route-aware staging structure unless a broader redesign is explicitly in scope.
- Treat any change from silent falsey behavior toward hard evaluator errors as a behavioral change requiring targeted verification.
- Do not claim the slot-order issue is a confirmed bug until the upstream IR/order guarantee is checked.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- move `strategy_dynamic.rs` from `Larger refactor but defer` to `Safe-now hardening; larger redesign still deferred`
- prioritize contract-accuracy fixes and test-first hardening now
- keep the broader planner/runtime/evaluator redesign deferred until parity and consumer contracts are explicit
