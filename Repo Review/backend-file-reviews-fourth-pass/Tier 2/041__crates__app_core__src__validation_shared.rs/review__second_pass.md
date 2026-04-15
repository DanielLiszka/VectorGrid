# Second-Pass Review: `crates/app_core/src/validation_shared.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_shared.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_shared.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a DTO and helper layer where defaults and canonical-key behavior matter more than implementation size.

## Stale Check

The live source still owns:

- walk-forward, SPA, and cross-ticker config DTOs
- canonical JSON key generation
- auxiliary-work masks like `preserved_finalists()`

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde default-field docs
   - Source: <https://serde.rs/attr-default.html>
   - Why consulted in second pass:
     - verify omission-driven config behavior
   - Additional constraint informed:
     - current defaults remain part of the public validation contract

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground documentation expectations for behavior-heavy DTOs
   - Additional constraint informed:
     - helper policy choices should stay explicit even in a small shared layer

## Delta vs First Pass

### 1. `canonical_json_key()` is deterministic internal-key shaping, not a general canonical-JSON layer

- Why this matters:
  - it sorts object keys recursively but intentionally preserves array order
- Practical implication:
  - keep that narrower meaning explicit

### 2. `ValidationAuxWorkMask::preserved_finalists()` disabling all auxiliary work is a deliberate cost/scope policy

- Practical implication:
  - preserve it unless validation cost policy intentionally changes

## Verification If Changed

- `cargo test -p ta_app_core validation_shared::tests:: -- --nocapture`
- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve existing serde defaults unless callers move together
- Preserve current canonical-key behavior
- Keep preserved-finalist auxiliary-work suppression unless policy changes deliberately

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that omission defaults and canonical-key behavior are still active contract choices, not generic helper trivia.
