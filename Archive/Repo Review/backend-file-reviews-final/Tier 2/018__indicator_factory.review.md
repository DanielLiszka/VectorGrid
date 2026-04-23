# Final Consolidated Review: `crates/app_core/src/indicator_factory.rs`

- Review date: `2026-04-15`
- Source file: [indicator_factory.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/indicator_factory.rs:1>)
- First-pass packet: [indicator_factory.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/indicator_factory.rs.review.md:1>)
- Second-pass packet: [indicator_factory.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/indicator_factory.rs.review.md:1>)
- Third-pass packet: [indicator_factory.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/indicator_factory.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/018__crates__app_core__src__indicator_factory.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://rust-lang.github.io/api-guidelines/naming.html>
  - <https://docs.rs/serde_json/latest/serde_json/enum.Value.html>

## Consolidated Judgment

The fourth-pass packet is valid and should supersede the more generic third-pass framing. The live source confirms that this file is a tiny CPU-only wrapper around `double_ma` moving-average helpers, and its real review surface is therefore boundary semantics: `IndicatorMatrixF64` can still be built directly with public fields, `row()` still returns `None` for zero-width matrices, the request/response pair does not echo resolved `ma_params`, and local wrapper coverage remains thin. The fourth pass is right to treat these as contract/documentation/test issues rather than redesign work.

No wrong `review_fourth_pass.md` is present in this folder. The main strengthening needed is to keep future deserialize hardening and response-shape expansion explicitly out of the safe-now bucket, because both would be public-surface changes.

## Safe-Now Priority

- Tighten rustdoc so the wrapper contract is explicit:
  - CPU-only
  - moving-average-only
  - uniform-step period grid only
  - row-major matrix orientation
  - normalized `ma_type` / trimmed `ma_source` output
  - response does not echo resolved `ma_params`
- Add wrapper-local tests for:
  - empty `ma_type`
  - empty `ma_source`
  - empty `periods`
  - duplicate or descending periods
  - single-period success
  - normalization echo behavior
  - zero-width `row()` behavior if that state remains representable
- If any tiny extra hardening is desired, prefer documentation plus tests over behavior churn.

## Deferred Work

- Do not make `IndicatorMatrixF64` fields private in the same patch unless the crate is intentionally accepting a public-shape change.
- Do not add custom/`try_from` deserialization as if it solved the full invariant-leak problem; it only hardens the deserialize path.
- Do not expand the response DTO with `ma_params` or resolved-parameter provenance without treating it as a serialized/public-shape change.
- Do not treat the current derives as proof that this is a production-ready JSON or IPC DTO, especially given non-finite warmup values.

## Verification If Changed

- `cargo test -p ta_app_core indicator_factory::tests:: -- --nocapture`
- `cargo test -p ta_app_core cpu_ma_params_gaussian_poles_changes_output -- --nocapture`
- `cargo test -p ta_app_core cpu_ma_matrix_rowwise_scalar_fallback_fills_skipped_rows_with_nan -- --nocapture`
- Add and run wrapper-local tests for the boundary cases listed above.

## Regression And Feature-Preservation Constraints

- Preserve delegated MA behavior in `double_ma`; this file is a facade, not the owner of MA math.
- Do not change `ma_source` normalization or row semantics opportunistically.
- Treat zero-width row behavior as a deliberate semantics decision if touched.
- Preserve current serialized/public DTO shape unless a compatibility review explicitly says otherwise.

## Final Conclusion

The implementation-facing conclusion is:

- keep `indicator_factory.rs` in `Small surgical refactor`
- treat the fourth pass as valid and more file-accurate than the third pass
- spend the safe-now budget on docs and boundary tests
- defer representation changes, deserialize hardening, response-shape expansion, and future wire-format redesign
