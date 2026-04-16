# Final Consolidated Review: `crates/app_core/src/strategy_ir.rs`

- Review date: `2026-04-16`
- Source file: [strategy_ir.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_ir.rs:1>)
- First-pass packet: [strategy_ir.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/strategy_ir.rs.review.md:1>)
- Second-pass packet: [strategy_ir.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_ir.rs.review.md:1>)
- Third-pass packet: [strategy_ir.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_ir.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/037__crates__app_core__src__strategy_ir.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://serde.rs/field-attrs.html>
  - <https://serde.rs/container-attrs.html>
  - <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

All four passes converge on the right bucket: this is still `Small surgical refactor`, not a redesign file. The fourth pass is valid and materially sharper than the earlier packets because it recenters the review on the live contract this file actually owns.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and should govern the final review.

The core issue is that [`validate_strategy_ir()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_ir.rs:460>) is a well-formedness validator, not a full runnable-profile validator, while [`build_static_strategy_ir()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_ir.rs:722>) clearly implies a stricter canonical shape. Dynamic indicators get descriptor-backed checks, but `fast_ma`, `slow_ma`, and `ma_source` remain comparatively loose. The serde layer is also intentionally permissive: omission defaults are part of the wire contract, extra JSON fields are currently tolerated, and [`ValueRef::ConstantBits`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_ir.rs:114>) accepts both numeric and decimal-string inputs even though JavaScript number transport is only integer-safe up to `2^53 - 1`.

## Safe-Now Priority

- Add rustdoc on the public contract types and constants clarifying that `validate_strategy_ir()` enforces well-formedness, not the full set of runtime-profile invariants.
- Add targeted tests for the real contract edges:
  - profile/program or profile/execution mismatches,
  - non-canonical casing and whitespace,
  - unknown JSON fields,
  - MA and `ma_source` legality assumptions,
  - `ConstantBits` string-vs-number transport expectations.
- If stricter validation is needed immediately, add a separate stricter helper for constrained callers instead of silently broadening `validate_strategy_ir()` in place.

## Deferred Work

- Do not add blanket `deny_unknown_fields` to the public IR types in this pass.
- Do not change `ConstantBits` serialization format without auditing frontend, Tauri, and persisted-artifact consumers.
- Do not move durable fingerprint or canonical-identity work into this file; that belongs to the planner/runtime identity layer.

## Verification If Changed

- `cargo test -p ta_app_core strategy_ir::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`
- If validation semantics or serialized shape change, also rerun downstream dynamic-strategy caller coverage before handoff.

## Regression And Feature-Preservation Constraints

- Preserve schema-v1 compatibility unless there is an explicit migration plan.
- Preserve omission-driven serde defaults unless caller behavior and artifact compatibility are audited together.
- Keep this file's role honest: wire acceptance, well-formedness, and static-builder shaping. Do not smuggle planner/runtime identity changes into this module.

## Final Conclusion

Keep `strategy_ir.rs` in `Small surgical refactor`. The next implementation should make validator scope, wire leniency, and `ConstantBits` transport constraints explicit, then lock them down with tests. Broad validation tightening, serializer changes, or identity redesign remain deferred.
