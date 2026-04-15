# Third-Pass Review: `crates/app_core/src/iterative_sim.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/iterative_sim.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/iterative_sim.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/iterative_sim.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

The second pass already raised a public-contract concern, but this file remained high ROI because it feeds other validation surfaces and still returns the shared `Metrics` type as if the result were fully populated and authoritative.

## Third-Pass Research Log

1. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether a partially populated `Metrics` return should be treated as contract debt rather than convenience
   - Third-pass constraint informed:
     - a public result type should not silently overstate completeness

2. Serde defaults and public config surface
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in third pass:
     - re-check how much of the current behavior is encoded into default-heavy public configuration
   - Third-pass constraint informed:
     - `IntrabarRule::Pessimistic` remains product policy, not neutral utility default

## What Third Pass Sharpened

### 1. The larger miss is no longer only invalid-input fallback; successful runs still return a structurally incomplete `Metrics`

- Evidence:
  - valid iterative runs still zero-fill or fabricate a number of shared `Metrics` fields
  - the function returns the same broad metrics type used elsewhere
- Why this matters:
  - callers are encouraged to treat the output as fully authoritative even when many fields are placeholders
- Practical implication:
  - the correct redesign direction is still larger and deferred:
    - either return a narrower iterative-specific result
    - or fully populate the fields this function currently promises

### 2. The `Metrics::default()` invalid-input fallback remains dangerous because downstream callers still treat it as real output

- Evidence:
  - invalid shape or range input still collapses into `Metrics::default()`
  - higher-level validation surfaces still consume this output without a strong invalid-result distinction
- Why this matters:
  - the caller cannot reliably tell "simulation failed contract" from "simulation produced zero-like metrics"
- Practical implication:
  - a safe migration path is still:
    - add a `try_` or `Result`-returning sibling
    - then decide whether the current function should remain as a compatibility wrapper

## Implementation Priority

- Highest-value design correction:
  - stop implying that the shared `Metrics` result is fully populated and authoritative
- Compatibility-first migration:
  - add an explicit fallible sibling before changing current callers

## Verification If Changed

- `cargo test -p ta_app_core iterative_ -- --nocapture`
- targeted validation and ML-signal flows that consume iterative metrics

Add targeted coverage for:

- invalid iterative input producing explicit failure on the new path
- compatibility behavior on the legacy path if retained
- downstream callers continuing to preserve current engine-selection semantics

## Regression And Feature-Preservation Constraints

- Preserve intrabar, cooldown, persistence, and trade-cost semantics
- Preserve current iterative validation behavior until callers migrate intentionally
- Do not silently reinterpret zero/default metrics as stronger contract than they are today

## Third-Pass Conclusion

`Bucket unchanged, but the happy-path contract is weaker than second pass emphasized`

`iterative_sim.rs` still belongs in `Larger refactor but defer`, but the third pass sharpens the message. The biggest problem is not only that invalid input falls back to `Metrics::default()`; it is also that successful runs still return a partially fabricated `Metrics` value that overstates completeness.
