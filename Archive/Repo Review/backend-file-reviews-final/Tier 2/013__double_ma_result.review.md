# Final Consolidated Review: `crates/app_core/src/double_ma_result.rs`

- Review date: `2026-04-15`
- Source file: [double_ma_result.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_result.rs:1>)
- First-pass packet: [double_ma_result.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/double_ma_result.rs.review.md:1>)
- Second-pass packet: [double_ma_result.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_result.rs.review.md:1>)
- Third-pass packet: [double_ma_result.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_result.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/013__crates__app_core__src__double_ma_result.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/macro.debug_assert_eq.html>
  - <https://serde.rs/field-attrs.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

All four passes keep the right bucket, and the fourth pass is the best implementation guide because it replaces generic serde/default framing with the actual invariant seams this file owns: duplicated count sources, projected grid metadata, and silent param-resolution fallback.

The fourth-pass packet is valid as written. The key is to keep the hardening development-facing and non-breaking: assertions, comments, and targeted tests are justified; runtime behavior and serialized shape changes are not. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Add `debug_assert_eq!` checks that `ctx.num_combos == result.num_combos` and `ctx.num_candles == result.num_candles` before final DTO assembly.
- Add development-time invariant checks or tests for out-of-range MA param ids and impossible Grid-mode matrix lookup omissions.
- Document that `matrix_index`, `best_col`, and `top_cols` are period-grid projection metadata, not unique candidate identifiers when MA param sweeps merge variants.
- Add tests that pin the projected-column behavior under param sweeps and the current tolerant omission path.

## Deferred Work

- Do not turn tolerant omission paths into hard runtime errors in this pass.
- Do not overload `route_diagnostics` with result-finalizer invariant notes.
- Do not redesign the matrix identity surface to carry variant-aware ids without auditing all consumers.
- Do not pursue payload deduplication or IPC-schema redesign as part of this packet.

## Verification If Changed

- `cargo test -p ta_app_core double_ma_result -- --nocapture`
- `cargo test -p ta_app_core cpu_ma_param_sweep_increases_combo_count_and_reports_params -- --nocapture`
- Re-run one follow-up or validation consumer that reads resolved MA params if param-resolution behavior changes.

## Regression And Feature-Preservation Constraints

- Preserve current serialized field presence and omission behavior for `BackendOptimizationResult` and `DoubleMaParamsResolved`.
- Treat `best_col` and `top_cols` as projected heatmap metadata unless a versioned contract change says otherwise.
- Do not convert late-stage result shaping into a new user-visible error surface without coordinated caller review.
- Keep performance work out of scope; the current concerns are invariant truth and consumer interpretation.

## Final Conclusion

The implementation-facing conclusion is:

- keep `double_ma_result.rs` in `Small surgical refactor`
- treat the fourth pass as the authoritative packet because it identifies the real invariant seams
- add debug assertions, targeted tests, and projection-semantics comments now
- defer runtime errors, variant-aware matrix identity, and payload/schema redesign
