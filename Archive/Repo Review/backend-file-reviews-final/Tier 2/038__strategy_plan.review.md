# Final Consolidated Review: `crates/app_core/src/strategy_plan.rs`

- Review date: `2026-04-16`
- Source file: [strategy_plan.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_plan.rs:1>)
- First-pass packet: [strategy_plan.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/strategy_plan.rs.review.md:1>)
- Second-pass packet: [strategy_plan.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_plan.rs.review.md:1>)
- Third-pass packet: [strategy_plan.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_plan.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/038__crates__app_core__src__strategy_plan.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/collections/hash_map/struct.DefaultHasher.html>
  - <https://doc.rust-lang.org/std/collections/struct.HashMap.html>
  - <https://serde.rs/container-attrs.html>

## Consolidated Judgment

The four passes agree on the bucket, and that part should stand. `strategy_plan.rs` is still a `Small surgical refactor` file. The fourth pass is valid, but it needs one strengthening in the final consolidation: the strongest current issue is not merely `DefaultHasher` stability, and it is not safe to treat fingerprint generation as a free cleanup either.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and should govern the final review.

The live planner trusts route-override values completely once the trimmed slot id matches, while unknown override keys are silently ignored. At the same time, [`fingerprint_ir()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_plan.rs:187>) hashes raw serialized IR bytes, and the IR can contain `HashMap`-backed parameter maps. That creates a real canonicality concern. But changing the emitted IR fingerprint would also change downstream compiled-plan and executable fingerprint strings and the in-memory GPU cache keys built from them. That makes immediate fingerprint rewriting audit-gated, not a no-risk cleanup.

## Safe-Now Priority

- Add or tighten tests that document the current layered identity model:
  - IR fingerprint,
  - compiled-plan fingerprint,
  - executable fingerprint.
- Add tests that pin `route_overrides` behavior explicitly:
  - unknown keys,
  - whitespace-trimmed keys,
  - trusted value-side overrides.
- Add local documentation clarifying that `ir_fingerprint_hex` is an opaque IR-level token, not the sole executable cache identity and not a durable public ID.

## Deferred Work

- Do not change IR fingerprint generation in production until all consumers of emitted fingerprint strings and cache-derived identities are audited.
- Do not hard-fail unknown `route_overrides` keys in this pass without confirming caller behavior.
- Do not alter default route-selection policy here as part of a review cleanup.

## Verification If Changed

- `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
- If fingerprint semantics change after audit, add a regression covering semantically equivalent IR payloads with different map insertion order.

## Regression And Feature-Preservation Constraints

- Preserve the current layered identity model across plan, GPU program, and executable surfaces.
- Treat emitted fingerprint strings as compatibility-visible even if they are not durable public IDs.
- Preserve route policy unless an explicit planner/runtime review is being done.
- Treat trusted override values as a documented escape hatch until a stricter caller contract exists.

## Final Conclusion

Keep `strategy_plan.rs` in `Small surgical refactor`. The right immediate move is to tighten documentation and tests around override semantics and identity layering. Fingerprint canonicalization is the most plausible later code change, but it is audit-gated because it would churn downstream fingerprint and cache behavior.
