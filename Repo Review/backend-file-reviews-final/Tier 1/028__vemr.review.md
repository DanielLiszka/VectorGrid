# Final Consolidated Review: `crates/app_core/src/vemr.rs`

- Review date: `2026-04-15`
- Source file: [vemr.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/vemr.rs:1>)
- First-pass packet: [vemr.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/vemr.rs.review.md:1>)
- Second-pass packet: [vemr.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vemr.rs.review.md:1>)
- Third-pass packet: [vemr.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vemr.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/028__crates__app_core__src__vemr.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Targeted hardening now; larger redesign still deferred`
- External research anchors:
  - <https://doc.rust-lang.org/std/macro.debug_assert.html>
  - <https://serde.rs/attr-default.html>
  - <https://serde.rs/container-attrs.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>

## Consolidated Judgment

The fourth-pass packet is the right final packet for this file. The earlier passes were too passive because they overstated test confidence around the most sensitive VEMR semantics. The live test module covers only period mapping, one invalid-parameter case, and one CUDA fallback diagnostic. It does not directly pin `compute_vemr_start_t()` or the desired-position state machine, which is where ranking- and parity-sensitive behavior actually lives.

The bucket should therefore move from pure defer to targeted hardening now. That does not justify a broader architectural rewrite. It means this file has a meaningful, low-risk slice of tests, debug-only invariants, and contract clarifications that can reduce regression risk immediately without changing strategy behavior.

## Safe-Now Priority

- Add pure Rust tests for:
  - `compute_vemr_start_t()`
  - `build_vemr_desired_positions()` across confirmation streaks, `trade_on_next_bar`, `allow_flip`, `long_only`, ATR-stop exit, mean exit, and `max_trade_bars`
- Add regression tests around `vemr_indicator_ids()` binding behavior so fuzzy descriptor resolution stays deliberate.
- Add validator table tests for:
  - `z_exit > z_entry`
  - empty `z_ma_type`
  - mapped-period edge cases where validation and resolution are intentionally not the same thing
- Add narrow debug assertions on signal-length alignment in `build_vemr_desired_positions()`.

## Deferred Work

- Defer stricter typed descriptor binding.
- Defer tighter serialized-field validation for `z_ma_type` and `z_devtype`.
- Defer the broader CPU/CUDA/file split until `vemr.rs`, `vemr_gpu_kernel.rs`, and composite consumers are reviewed together.
- Defer any broader indicator-catalog redesign or route-diagnostic redesign.

## Verification If Changed

- `cargo test -p ta_app_core vemr::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- If any CUDA-facing helper, sweep-row behavior, or companion GPU surface changes, expand verification to the CUDA-enabled VEMR coverage before rollout.

## Regression And Feature-Preservation Constraints

- Preserve current `VemrStrategyParams` defaults, field names, and validation strictness unless request compatibility is being changed deliberately.
- Preserve current desired-position semantics unless CPU/GPU parity and validation outputs are rechecked together.
- Treat `VemrDeviceMatrix`, sweep-row helpers, and start-index math as GPU-companion-facing contracts even when the safe-now work is test-first.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- move `vemr.rs` to `Targeted hardening now; larger redesign still deferred`
- spend the immediate effort on tests, debug-only invariants, and descriptor-binding guardrails
- preserve current serialized-input and state-machine semantics
- defer descriptor-system cleanup, stricter validation, and file-architecture work until the VEMR CPU/GPU pair is reviewed together
