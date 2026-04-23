# Final Consolidated Review: `crates/strategies/src/double_ma/mod.rs`

- Review date: `2026-04-15`
- Source file: [mod.rs](</C:/Rust Projects/ta_desktop_demo/crates/strategies/src/double_ma/mod.rs:1>)
- First-pass packet: [mod.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/strategies/src/double_ma/mod.rs.review.md:1>)
- Second-pass packet: [mod.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/strategies/src/double_ma/mod.rs.review.md:1>)
- Third-pass packet: [mod.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/strategies/src/double_ma/mod.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/032__crates__strategies__src__double_ma__mod.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Targeted hardening now; larger redesign still deferred`
- External research anchors:
  - <https://serde.rs/attr-default.html>
  - <https://serde.rs/container-attrs.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The first three passes were right to keep the broad redesign deferred. This file is a foundational strategy module with a large serde-facing config surface, multiple evaluator families, and many downstream consumers. The fourth pass correctly identifies one concrete hardening defect that is strong enough to justify action now without reopening the module architecture.

That defect is the reduced halftrend f64 path. The reduced wrappers intentionally disable drawdown-risk computation, but the underlying f64 halftrend implementation still computes drawdown and ulcer accumulation unconditionally. The plain f64 path and the f32 halftrend path already gate that work correctly. That makes the reduced f64 branch the right safe-now target. The fourth pass is also right that earlier review packets overstated local test coverage.

No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Gate drawdown-risk work correctly in the reduced halftrend f64 path so `COMPUTE_DRAWDOWN_RISK = false` actually removes the extra work and metrics accumulation.
- Add focused tests that pin reduced-vs-rich halftrend metric-surface behavior and prove the intended gating difference.
- Add narrow rustdoc on the helper/builder surfaces where the current behavior is easy to misread:
  - `DoubleMaBatchBuilder::run_cpu`
  - the legacy type-mapping helper
  - the typed evaluation path

## Deferred Work

- Defer splitting the file by evaluator family or responsibility.
- Defer tighter serde-policy changes for `StrategyConfig`.
- Defer public API reshaping or stricter request validation.

## Verification If Changed

- `cargo test -p ta_strategies -- --nocapture`
- `cargo test -p ta_optimizer -- --nocapture`
- `cargo test -p ta_app_core double_ma -- --nocapture`
- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- If evaluator hot-loop behavior changes beyond the intended gating fix, add a CPU perf/workflow smoke before rollout.

## Regression And Feature-Preservation Constraints

- Preserve existing `StrategyConfig` field names, defaults, and serde shape unless compatibility is being changed deliberately.
- Preserve evaluator semantics outside the intended reduced-path drawdown-risk gating fix.
- Do not combine the safe-now fix with unrelated module cleanup, config tightening, or ranking changes.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- move `double_ma/mod.rs` to `Targeted hardening now; larger redesign still deferred`
- fix the reduced halftrend f64 drawdown-risk gate and add narrow regression coverage around it
- keep helper rustdoc and local test coverage improvements additive
- defer file split, serde tightening, and broader public-API cleanup until a dedicated double-MA architecture pass
