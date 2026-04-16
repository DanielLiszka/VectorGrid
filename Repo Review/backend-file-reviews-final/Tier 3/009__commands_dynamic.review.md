# Final Consolidated Review: `src-tauri/src/commands_dynamic.rs`

- Review date: `2026-04-16`
- Source file: [commands_dynamic.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_dynamic.rs:1>)
- First-pass packet: [commands_dynamic.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_dynamic.rs.review.md:1>)
- Second-pass packet: [commands_dynamic.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_dynamic.rs.review.md:1>)
- Third-pass packet: [commands_dynamic.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_dynamic.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/009__src-tauri__src__commands_dynamic.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://serde.rs/container-attrs.html>
  - <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER>
  - <https://doc.rust-lang.org/std/primitive.f64.html#method.to_bits>

## Consolidated Judgment

All four passes land in the same practical bucket, and the fourth pass is the strongest packet. The live file is a small but real Tauri IPC seam built around [DynamicDoubleMaStrategyIrJsonPreviewResponse](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_dynamic.rs:9>), [preview_dynamic_double_ma_strategy_ir_json](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_dynamic.rs:21>), [format_dynamic_strategy_ir_json](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_dynamic.rs:50>), and [encode_dynamic_strategy_constant_bits](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_dynamic.rs:55>).

The stable review conclusion is:

- the file does not warrant a redesign;
- the current transport contract is sharper risk than the file size suggests;
- the main issues are still IPC-shape issues, not runtime ownership or cancellation.

The most credible safe-now concerns are:

- the seam mixes text-form IR ingress with structured `StrategyIr` egress and formatter ingress;
- `schema_compatibility` is serialized through `Debug` formatting at [commands_dynamic.rs:43](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_dynamic.rs:43>), which is weaker than an explicit wire literal mapping;
- the preview command’s argument naming remains implicit even though it has multiple snake_case arguments at [commands_dynamic.rs:23](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_dynamic.rs:23>);
- local tests do not yet cover the risky path: constant-bit-carrying IR, formatter behavior, or signed-zero bit preservation.

## Safe-Now Priority

- Keep the bucket at `Small surgical refactor`.
- Treat textual JSON as the canonical cross-IPC representation for dynamic IR. If `response.ir` remains for compatibility, do not treat it as the long-term source of truth for frontend round-tripping.
- Replace `format!("{schema_compatibility:?}")` with an explicit string mapping so the wire value is decoupled from Rust `Debug`.
- Make the preview command’s argument policy explicit if the frontend is intentionally sending snake_case keys; otherwise keep callers aligned to Tauri’s default camelCase expectation and test the real boundary.
- Add direct regression coverage for:
  - constant-bit-carrying IR through preview;
  - `format_dynamic_strategy_ir_json`;
  - `encode_dynamic_strategy_constant_bits(-0.0)`;
  - a command-boundary case that proves the intended argument names.

## Deferred Work

- Defer any bigger decision about whether `StrategyIr` should ever cross this Tauri boundary as a structured object.
- Do not remove `format_dynamic_strategy_ir_json` or `encode_dynamic_strategy_constant_bits` unless current callers are inventoried first.
- Do not change the preview response shape, command names, or error shape casually.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`
- Re-run the frontend dynamic-strategy builder / preview / execute tests that consume these commands.
- If the preview request/response contract changes, also run:
  - `npm --prefix ./web run check:commands`
  - the targeted frontend typecheck and dynamic-strategy tests

## Regression And Feature-Preservation Constraints

- Preserve exact `f64::to_bits()` semantics in [encode_dynamic_strategy_constant_bits](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_dynamic.rs:55>); signed zero and other exact-bit cases matter here.
- Do not silently flip the canonical IR transport between text and structured-object semantics.
- Do not broaden this file into general strategy-planning ownership; it should remain a narrow preview/format/encoding seam.

## Final Conclusion

The fourth-pass packet is valid, no wrong fourth-pass file is present, and the final bucket remains `Small surgical refactor`. The implementation priority is a narrow contract hardening pass: stabilize wire literals, make the preview argument contract explicit, and add the missing risk-path tests without redesigning the command surface.
