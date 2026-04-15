# Second-Pass Review: `crates/app_core/src/validation_core.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_core.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_core.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this helper as small but policy-bearing because tuple-return conventions and stringly phase labels feed shared validation behavior.

## Stale Check

The live source still:

- accepts generic closure output as `(candidate, start_t, sample_obs)`
- normalizes `common_start_t` and `sample_obs`
- uses stringly `validate_phase` / `cscv_prep_phase` labels

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground documentation of generic closure and tuple contracts
   - Additional constraint informed:
     - tuple ordering and normalization remain stronger contract than the small size suggests

2. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in second pass:
     - reassess validation and normalization rules
   - Additional constraint informed:
     - current `max(...)` and `sample_obs.max(2)` semantics are policy, not incidental arithmetic

## Delta vs First Pass

### 1. The closure-return tuple is more policy-bearing than it first looks because its two `usize`s directly drive normalization

- Why this matters:
  - changing ordering or meaning would ripple into validation math immediately

### 2. Stringly phase inputs remain acceptable only if their vocabulary stays documented and stable

- Practical implication:
  - keep them aligned with the shared progress/reporting surfaces

## Verification If Changed

- `cargo test -p ta_app_core validation_core::tests:: -- --nocapture`
- representative `double_ma_validate` and `composite_validate` suites

## Regression And Feature-Preservation Constraints

- Preserve tuple ordering
- Preserve `common_start_t.max(...)` and `sample_obs.max(2)` normalization
- Keep phase labels stable unless downstream reporting surfaces move together

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that tuple-return ordering and normalization rules here are active policy, not generic helper trivia.
