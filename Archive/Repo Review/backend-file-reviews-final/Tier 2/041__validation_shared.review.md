# Final Consolidated Review: `crates/app_core/src/validation_shared.rs`

- Review date: `2026-04-16`
- Source file: [validation_shared.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_shared.rs:1>)
- First-pass packet: [validation_shared.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/validation_shared.rs.review.md:1>)
- Second-pass packet: [validation_shared.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_shared.rs.review.md:1>)
- Third-pass packet: [validation_shared.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_shared.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/041__crates__app_core__src__validation_shared.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://serde.rs/field-attrs.html>
  - <https://serde.rs/container-attrs.html>
  - <https://www.rfc-editor.org/rfc/rfc8785>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It identifies the real high-value hardening target more precisely than the earlier packets did.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and should govern the final review.

This file is still a compatibility-sensitive DTO and helper surface, but the most important immediate risk is not abstract serde cleanup. It is the duplicated default-path contract around [`CrossTickerGateConfig`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_shared.rs:340>): omitting the whole nested `gate` object uses `Default`, while `gate: {}` uses field-level serde defaults. Those paths are manually kept aligned today and can drift. The next tier of risk is cross-file sentinel behavior completed in `validation_pipeline.rs`, especially `PerturbConfig` zero-default meaning "disabled" and `TradeBootstrapConfig::stationary_mean_block_len = Some(0)` meaning IID-only mode.

## Safe-Now Priority

- Add a regression proving that deserializing an empty `CrossTickerGateConfig` produces the same values as `CrossTickerGateConfig::default()`.
- Add targeted sentinel/default tests spanning this file and `validation_pipeline.rs`:
  - `PerturbConfig` all-zero omission posture,
  - `TradeBootstrapConfig::stationary_mean_block_len = Some(0)`,
  - representative walk-forward and SPA low-value sanitization behavior.
- Add a few representative JSON-shape and canonical-key invariants instead of blanket round-trip expectations:
  - one sparse DTO shape,
  - one omitted-optional stats shape,
  - nested-object vs array-order behavior for `canonical_json_key()`.

## Deferred Work

- Do not add blanket `deny_unknown_fields` here in this pass.
- Do not normalize null-vs-omitted serialization shape broadly.
- Do not replace `canonical_json_key()` with a different canonicalization or hashing scheme without auditing every caller.
- Do not force serialize-only result types into `Deserialize` just to satisfy generic round-trip testing advice.

## Verification If Changed

- `cargo test -p ta_app_core validation_shared::tests:: -- --nocapture`
- `cargo test -p ta_app_core validation_pipeline -- --nocapture`
- If any serde-visible shape changes, also rerun representative validation callers before merge.

## Regression And Feature-Preservation Constraints

- Keep `missing gate` and `gate: {}` behavior equivalent unless an intentional contract change says otherwise.
- Preserve current sentinel semantics for `PerturbConfig` and `TradeBootstrapConfig`.
- Treat sparse JSON shape as compatibility-sensitive for artifacts and IPC payloads.
- Keep this file out of GPU-ABI discussions; the risk here is validation semantics and payload compatibility, not device layout.

## Final Conclusion

Keep `validation_shared.rs` in `Small surgical refactor`. The immediate implementation target is a small set of equivalence and sentinel tests plus concise documentation of the non-obvious contract edges. Broader serde tightening and DTO reshaping remain deferred.
