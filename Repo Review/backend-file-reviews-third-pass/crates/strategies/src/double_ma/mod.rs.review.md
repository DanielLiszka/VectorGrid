# Third-Pass Review: `crates/strategies/src/double_ma/mod.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/double_ma/mod.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/strategies/src/double_ma/mod.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/double_ma/mod.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the foundational double-ma strategy config and metrics surface still looked like the right framing under fresh research. The third pass kept the bucket but sharpened one config-contract issue the second pass underweighted: this large public Deserialize surface defaults many fields and still accepts unknown fields unless callers add external validation.

## Third-Pass Research Log

1. Serde container attributes
   - Source: <https://serde.rs/container-attrs>
   - Why consulted in third pass:
     - recheck how unknown-field handling works for this large public config type
   - Third-pass constraint informed:
     - without deny_unknown_fields, stale or misspelled config keys can deserialize into altered defaults instead of failing fast

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in third pass:
     - ground the review of the file many defaulted config fields
   - Third-pass constraint informed:
     - defaulted config fields are active contract once request builders and stored configs depend on them

3. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - confirm that the right response is still contract clarity, not a local cleanup
   - Third-pass constraint informed:
     - foundational config types should change with deliberate migration, not incidental tightening

## What Third Pass Sharpened

### 1. Unknown config fields can still deserialize into altered defaults without failing fast
- Evidence:
  - StrategyConfig is a large public Deserialize surface with many serde defaulted fields.
- Why this matters:
  - Stale or misspelled non-required keys can be silently ignored, leaving callers with a valid-looking config that carries altered defaults.
- Practical implication:
  - Keep the existing bucket, but make unknown-field policy the sharper third-pass concern.

### 2. This remains larger-than-local contract territory
- Why this matters:
  - Because this config type is foundational across crates, tightening parse semantics is broader than a local helper cleanup.
- Practical implication:
  - If hardened, plan it as a migration with fixtures and caller coordination rather than a one-off refactor.

## Implementation Priority

- Decide explicitly whether unknown-field tolerance should remain part of the contract.
- Add stored-config and request-builder fixtures before tightening serde behavior.

## Verification If Changed

- cargo test -p ta_strategies double_ma -- --nocapture
- representative config round-trip tests with unknown and misspelled fields

## Regression And Feature-Preservation Constraints

- Preserve current valid-field defaults while evaluating unknown-field handling.
- Do not tighten serde behavior without coordinated migration for stored and request-built configs.
- Keep foundational config and metrics shapes stable unless all consumers move together.

## Third-Pass Conclusion

`Bucket unchanged, but config-tolerance sharpened`

double_ma/mod.rs stays in Larger refactor but defer. The third pass mainly sharpens one foundational contract issue: this public config surface still accepts unknown fields into defaulted behavior, which is broader than a local cleanup problem.
