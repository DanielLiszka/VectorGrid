# Second-Pass Review: `crates/app_core/src/indicator_factory.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/indicator_factory.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/indicator_factory.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a small but invariant-sensitive matrix-construction helper whose derived deserialization can bypass `try_new()`.

## Stale Check

The live source still:

- derives `Deserialize` on `IndicatorMatrixF64`
- exposes fields publicly enough to bypass `try_new()`
- delegates real indicator behavior elsewhere

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde custom-deserialization guidance
   - Source: <https://serde.rs/impl-deserialize.html>
   - Why consulted in second pass:
     - re-check invariant-preserving deserialization options
   - Additional constraint informed:
     - `try_new()` is still advisory unless deserialization or visibility enforces it

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground the public invariant and small-wrapper review
   - Additional constraint informed:
     - no production callers in-repo lowers the risk of tightening docs or visibility, but not the API concern itself

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - confirm predictable behavior expectations for public types
   - Additional constraint informed:
     - zero-column semantics remain a contract choice, not a convenience detail

## Delta vs First Pass

### 1. The main unresolved problem is still invariant leakage, not runtime behavior

- Why this matters:
  - invalid states can still deserialize or be constructed outside the checked path
- Practical implication:
  - visibility tightening or invariant-preserving deserialization remains the best bounded fix

## Verification If Changed

- `cargo test -p ta_app_core indicator_factory::tests:: -- --nocapture`
- delegated CPU MA matrix tests in `double_ma`

## Regression And Feature-Preservation Constraints

- Preserve current delegated MA behavior in `double_ma`
- Preserve successful constructor behavior unless invariants are intentionally tightened
- Treat visibility changes as public-surface work even if in-repo callers are scarce

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly confirms that invariant leakage through public fields and derived deserialization is still the real issue.
