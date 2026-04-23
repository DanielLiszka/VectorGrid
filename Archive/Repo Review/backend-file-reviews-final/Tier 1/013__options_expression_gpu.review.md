# Final Consolidated Review: `crates/app_core/src/options_expression_gpu.rs`

- Review date: `2026-04-14`
- Source file: [options_expression_gpu.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs:1>)
- First-pass packet: [options_expression_gpu.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/options_expression_gpu.rs.review.md:1>)
- Second-pass packet: [options_expression_gpu.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_expression_gpu.rs.review.md:1>)
- Third-pass packet: [options_expression_gpu.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_expression_gpu.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/013__crates__app_core__src__options_expression_gpu.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/cust/latest/cust/context/index.html>
  - <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
  - <https://doc.rust-lang.org/std/sync/struct.Mutex.html>

## Consolidated Judgment

The fourth pass is directionally right: the file stays in `Small surgical refactor`, and the best safe-now work is contract alignment between planning/inspection and the live scorer, plus bounded runtime-state hardening. What needs normalization is the tone around poison recovery and feasibility/accounting changes. The final packet should not imply that we can casually reinterpret exported cache-usage metrics or blindly recover poisoned GPU runtime state. The safe implementation is narrower: align validation with the current scorer contract, fix combined-plan accounting only where shared layout identity is actually proven, and surface runtime-state failures as explicit errors or bounded rebuilds.

## Safe-Now Priority

- Make the feasibility inspector mirror the live scorer’s input checks. `inspect_plan(...)` and `inspect_combined_plan(...)` should reject the same malformed inputs the scorer rejects instead of accepting them and diverging later.
- Add local range and ordering guardrails for `event_start/event_end` and `search_end/holdout_end` before launch preparation.
- Fix combined-plan shared-input accounting only when the three lanes genuinely share the same layout identity, not by assuming `3` shared lanes unconditionally.
- Narrow poison handling to surfaced `Err(...)` or bounded runtime-entry rebuild, rather than `.expect(...)` panics or blind poison recovery.

## Deferred Work

- Keep any broader reinterpretation of exported cache-usage or feasibility metrics deferred.
- Do not broaden runtime cache recovery or planner semantics beyond what the current scorer contract can justify.

## Verification If Changed

- `cargo test -p ta_app_core options_expression_gpu -- --nocapture`
- Rerun GPU planner/inspector parity tests and any combined-plan feasibility tests
- If runtime-state handling changes, rerun representative warm-cache vs cold-cache flows before trusting perf comparisons

## Regression And Feature-Preservation Constraints

- Tightening inspector validation can change feasibility outcomes for malformed-but-currently-accepted inputs, so keep it aligned to the current scorer contract rather than inventing a stricter one.
- Combined-plan accounting changes can alter feasibility stats and route decisions if exported metrics are reinterpreted too aggressively.
- Poison handling is not cosmetic here. Wrong recovery can reuse tainted GPU state, while overly aggressive rebuilds can throw away warm-cache benefits and distort measurements.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `options_expression_gpu.rs` at `Small surgical refactor`
- align inspector validation with the live scorer
- keep runtime-state recovery bounded and explicit
