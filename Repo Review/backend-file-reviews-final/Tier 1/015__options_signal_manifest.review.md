# Final Consolidated Review: `crates/app_core/src/options_signal_manifest.rs`

- Review date: `2026-04-15`
- Source file: [options_signal_manifest.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/options_signal_manifest.rs:1>)
- First-pass packet: [options_signal_manifest.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/options_signal_manifest.rs.review.md:1>)
- Second-pass packet: [options_signal_manifest.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_signal_manifest.rs.review.md:1>)
- Third-pass packet: [options_signal_manifest.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_signal_manifest.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/015__crates__app_core__src__options_signal_manifest.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.from_timestamp_millis>
  - <https://serde.rs/field-attrs.html>

## Consolidated Judgment

The fourth pass is the strongest packet and should stand without normalization. Earlier passes correctly flagged that this file is tiny but contract-sensitive because it emits persisted signal-manifest data consumed by options follow-up workflows. The fourth pass sharpened the right near-term issues: closed-trade exit integrity, strategy-neutral error text, and explicit documentation of the current price/session policy.

The highest-value safe-now work is not broad provenance redesign. It is to stop silently emitting structurally valid-looking events for invalid closed-trade exit data and to make the builder’s live behavior easier to audit.

## Safe-Now Priority

- Validate `entry_t` unconditionally rather than relying on edge-bar clamping alone.
- For closed trades, reject `exit_t < entry_t` instead of emitting a nonsensical event.
- For closed trades, require a real `base_exit_timestamp` whenever `base_exit_bar_index` is present. Today a closed trade can degrade into downstream fixed-hold fallback behavior if the timestamp lookup returns `None`.
- Keep the no-signal path as `Err(...)`, but genericize the message so it is not `double_ma`-specific when called through the composite wrappers.
- Document that `raw_underlier_price` is currently sourced from the entry-bar `open` with `close` fallback, and that `session_id` / `calendar_id` currently encode a UTC-day policy rather than a true market-hours resolver.
- Add fixture coverage for:
  - closed trade with missing exit timestamp
  - closed trade with `exit_t < entry_t`
  - open trade preserving absent exit metadata
  - `trade_on_next_bar = false`
  - parity across the four public wrapper functions

## Deferred Work

- Do not change the no-signal modeling policy from `Err(...)` to `Ok(empty_manifest)` in a small patch. That is a broader IPC/product decision.
- Do not try to solve exchange-local session or dataset-provenance threading inside this helper alone.
- Do not silently change the emitted underlier price basis without revalidating downstream candidate-selection parity.

## Verification If Changed

- `cargo test -p ta_app_core options_signal_manifest::tests:: -- --nocapture`
- `cargo test -p ta_app_core options_expression::tests:: -- --nocapture`
- If materialization behavior or Tauri-facing error propagation changes:
  - `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve `event_id` and `manifest_id` formats unless replay/cache invalidation is intentional.
- Do not silently change `decision_timestamp`, `earliest_tradable_timestamp`, or the corresponding bar indexes; those fields participate in downstream identity and replay semantics.
- Do not silently change `raw_underlier_price`; it is operationally significant downstream.
- Keep open-trade exit metadata absent by design.
- Treat `calendar_id` and session behavior as persistence-affecting contract fields, not cosmetic metadata.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `options_signal_manifest.rs` at `Small surgical refactor`
- prioritize closed-trade exit integrity, strategy-neutral error text, and explicit price/session-contract documentation
- defer no-signal policy changes and broader provenance/session redesign until the wider options-expression boundary is being changed intentionally
