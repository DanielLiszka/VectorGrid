# Third-Pass Review: `crates/app_core/src/matrix_index.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/matrix_index.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/matrix_index.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/matrix_index.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a small but active contract/default surface still looked like the right framing under fresh research.

## Third-Pass Research Log

1. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether this small surface should be treated as a validation and contract seam rather than cleanup fodder
   - Third-pass constraint informed:
     - default values, sentinels, and exported shapes should remain explicit and testable

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in third pass:
     - check how omission/default behavior should be treated in these helper and DTO-adjacent surfaces
   - Third-pass constraint informed:
     - default and omission behavior are part of the wire contract once callers depend on them

3. Serde container attributes
   - Source: <https://serde.rs/container-attrs>
   - Why consulted in third pass:
     - confirm how much future compatibility work should focus on serde-facing behavior rather than internal representation
   - Third-pass constraint informed:
     - renaming or tightening container behavior needs coordinated caller changes, not isolated cleanup

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; this file reads more as a compatibility seam than a refactor target
- Why this matters:
  - The second-pass call of `Bucket unchanged` still fits after reopening the live source.
  - The real risk remains changing defaults, enum labels, omission behavior, or exported shape semantics underneath existing callers.
- Practical implication:
  - Keep the current bucket and prefer explicit contract docs and invariant tests before changing representation.

### 2. Invariant coverage is higher value than representation churn
- Why this matters:
  - These files are mostly small enough that refactors would create motion without removing the compatibility sensitivity.
- Practical implication:
  - If changed, lead with focused tests for default behavior, serde behavior, and result-shape compatibility.

## Implementation Priority

- Write or tighten invariant tests for defaults, omission behavior, and exported labels before refactoring.
- Keep structural changes narrowly scoped and compatibility-reviewed.

## Verification If Changed

- cargo test -p ta_app_core matrix_index -- --nocapture
- representative serde/default round-trip or request-contract tests for the owning flow

## Regression And Feature-Preservation Constraints

- Preserve current defaults, sentinels, and exported labels unless all consumers are updated together.
- Avoid broad internal cleanup that changes serde-visible or DTO-visible behavior.
- Keep compatibility notes close to the source if behavior is intentionally non-obvious.

## Third-Pass Conclusion

`Bucket unchanged`

matrix_index.rs stays in `Small surgical refactor`. The third pass mostly confirms that the high-value work here is contract documentation and invariant coverage, not refactor churn.
