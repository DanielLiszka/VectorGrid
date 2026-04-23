# Final Consolidated Review: `crates/app_core/src/matrix_index.rs`

- Review date: `2026-04-16`
- Source file: [matrix_index.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/matrix_index.rs:1>)
- First-pass packet: [matrix_index.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/matrix_index.rs.review.md:1>)
- Second-pass packet: [matrix_index.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/matrix_index.rs.review.md:1>)
- Third-pass packet: [matrix_index.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/matrix_index.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/026__crates__app_core__src__matrix_index.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with narrowing`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/cargo/reference/semver.html>
  - <https://doc.rust-lang.org/std/primitive.slice.html>
  - <https://serde.rs/attributes.html>

## Consolidated Judgment

The fourth-pass packet is the correct final frame for this file because it preserves the earlier invariant concern while correctly narrowing the safe-now lane. The live type still exposes public fields and derived deserialization over non-trivial derived caches, but the tempting fixes from the first pass, such as private-field tightening or validated deserialization redesign, are not compatibility-free in a public module.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file.

## Safe-Now Priority

- Add stronger rustdoc on `DoubleMaFastLtSlowIndex` and its methods that states:
  - `new()` is the authoritative constructor
  - `fast_periods` and `slow_periods` must be strictly increasing and non-empty
  - `FastMajor` ordering is the current stable mapping contract
  - `cols == 0` is a valid constructed state when no `fast < slow` pairs exist
  - `slow_start_idx` and `fast_offsets` are derived caches, not primary authoring inputs
- Add stronger file-local tests:
  - exhaustive or table-driven round-trip checks over small valid grids
  - an explicit zero-column test
  - a proof that `fast_offsets` can validly end with duplicate values when later fast rows have zero valid slow partners
- If the team intentionally treats the current wire shape as a contract, add a valid-instance serde round-trip test. If not, do not accidentally cement the redundant derived-field shape with new tests.

## Deferred Work

- Do not make fields private in the same patch unless a compatibility decision has already been made.
- Do not add `#[non_exhaustive]`, redesign `MatrixOrder`, or change the serialized field set in the same batch.
- Do not switch to validated custom deserialization without deciding how existing payloads or external consumers should behave.

## Verification If Changed

- `cargo test -p ta_app_core matrix_index::tests:: -- --nocapture`
- `cargo test -p ta_app_core`
- If serde-visible behavior changes:
  - add a valid-instance serialize/deserialize round-trip test
  - `cargo test --workspace`

## Regression And Feature-Preservation Constraints

- Preserve exact `FastMajor` column ordering semantics.
- Preserve the valid zero-column case.
- Do not over-tighten invariants in ways that reject valid nondecreasing `fast_offsets` suffixes.
- Treat all public-shape changes as compatibility work, not local cleanup.

## Final Conclusion

Keep `matrix_index.rs` in `Small surgical refactor`, treat the fourth pass as valid, and limit the safe-now lane to tests and docs. Public-shape tightening and validated-deserialization redesign remain deferred compatibility work.
