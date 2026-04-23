# Final Consolidated Review: `crates/app_core/src/indicator_catalog.rs`

- Review date: `2026-04-15`
- Source file: [indicator_catalog.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/indicator_catalog.rs:1>)
- First-pass packet: [indicator_catalog.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/indicator_catalog.rs.review.md:1>)
- Second-pass packet: [indicator_catalog.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/indicator_catalog.rs.review.md:1>)
- Third-pass packet: [indicator_catalog.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/indicator_catalog.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/017__crates__app_core__src__indicator_catalog.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
  - <https://serde.rs/field-attrs.html>

## Consolidated Judgment

The fourth-pass packet is valid and should be treated as the current implementation guide. The first and second passes correctly identified the main ambiguity around `step` and the cached-vs-uncached catalog distinction, but the fourth pass improves the prioritization: the higher-value gap is descriptor self-consistency, not opportunistic sort cleanup. The live source confirms that normalized descriptors are projected almost verbatim from registry metadata, while later validation focuses mainly on caller-supplied parameter values and output references. There is little file-local coverage today for unique normalized descriptor ids, unique output ids per descriptor, or descriptor defaults that successfully validate against their own schema.

No wrong `review_fourth_pass.md` is present in this folder. The main strengthening needed is to keep `step` firmly in the documentation-first bucket unless the project intentionally tightens request compatibility across consumers.

## Safe-Now Priority

- Document three existing behaviors explicitly:
  - `IndicatorParamDescriptor.step` is currently exposed metadata and is not enforced by request normalization
  - `supports_gpu_kernel` and `prefers_gpu_by_speedup` are app-core route/policy hints layered onto registry data
  - `list_dynamic_indicator_descriptors_cached()` is the process-lifetime snapshot, while `list_dynamic_indicator_descriptors()` is an uncached projection
- Add invariant tests that pin catalog self-consistency before downstream callers use it:
  - unique normalized descriptor ids
  - unique normalized output ids within each descriptor
  - descriptor defaults normalize successfully against their declared schema
  - any declared registry assumptions the project wants to expose publicly, such as non-empty output ids
- If parameter-validation flow is touched, keep duplicate-schema-key detection early and deterministic while minimizing error-message churn.

## Deferred Work

- Do not start enforcing `step` in the same patch unless the wider request/replay contract is being tightened on purpose.
- Do not convert cached descriptor loading into a panic-heavy or broader fallible startup path.
- Do not reshape public DTOs, rename fields, or start a local typed-error migration in this pass.

## Verification If Changed

- `cargo test -p ta_app_core indicator_catalog -- --nocapture`
- `cargo test -p ta_app_core compile_gpu_plan_routes_dynamic_indicator_to_gpu_when_policy_prefers_gpu -- --nocapture`
- `cargo test -p ta_app_core compile_gpu_plan_routes_dynamic_indicator_to_cpu_when_policy_prefers_cpu -- --nocapture`
- `cargo test -p ta_desktop_demo_app list_dynamic_indicators_returns_only_dynamic_strategy_eligible_descriptors -- --nocapture`
- Add and run focused invariant tests for:
  - normalized descriptor-id uniqueness
  - normalized output-id uniqueness
  - default-value self-validity
  - explicit pinned `step` behavior

## Regression And Feature-Preservation Constraints

- Preserve public DTO field names and current case-insensitive lookup behavior.
- Treat `step` enforcement as a compatibility-tightening change, not a documentation tweak.
- Preserve cached-vs-uncached semantics unless they are being intentionally redesigned.
- Minimize validation-order and error-text churn because the public helpers return plain `String` errors.

## Final Conclusion

The implementation-facing conclusion is:

- keep `indicator_catalog.rs` in `Small surgical refactor`
- treat the fourth pass as valid and more precise than the third pass
- prioritize contract docs and self-consistency invariant tests over sort cleanup or field reshaping
- defer `step` enforcement, DTO redesign, typed errors, and broader registry self-validation
