# Third-Pass Review: `crates/strategies/src/composite_cpu/mod.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/composite_cpu/mod.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/strategies/src/composite_cpu/mod.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/composite_cpu/mod.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a shared strategy contract surface still looked like the right framing under fresh research.

## Third-Pass Research Log

1. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - recheck whether these shared strategy surfaces should become stricter or stay compatibility-oriented
   - Third-pass constraint informed:
     - shared strategy contracts should move deliberately because multiple crates consume them

2. Serde container attributes
   - Source: <https://serde.rs/container-attrs>
   - Why consulted in third pass:
     - confirm how much of the config and metrics behavior is already serde-visible contract
   - Third-pass constraint informed:
     - tightening or renaming strategy config behavior needs coordinated caller changes

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in third pass:
     - ground default/omission semantics for strategy-shared types
   - Third-pass constraint informed:
     - defaulted fields and omitted values are active behavior once request builders depend on them

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; this file remains a shared strategy contract rather than a cleanup target
- Why this matters:
  - The second-pass call of `Bucket unchanged` still fits.
  - The live risk is still permissive normalization or zeroed fallback behavior acting as intentional library contract.
- Practical implication:
  - Keep the existing bucket and avoid strictness changes unless all consuming crates move together.

### 2. Compatibility tests are higher ROI than refactor symmetry
- Why this matters:
  - Shared strategy files are consumed widely enough that “cleanup” can easily become contract drift.
- Practical implication:
  - Lead with round-trip and behavioral fixtures before representation changes.

## Implementation Priority

- Prefer contract fixtures and serde-visible regression tests over refactor symmetry.
- Keep shared strategy behavior stable until a broader migration is planned.

## Verification If Changed

- cargo test -p ta_strategies composite_cpu -- --nocapture
- representative config/metrics round-trip tests across consuming crates

## Regression And Feature-Preservation Constraints

- Preserve shared strategy config and metrics behavior unless all consumers move together.
- Do not tighten permissive behavior without an explicit migration plan.
- Keep crate-root exports stable until downstream code is updated.

## Third-Pass Conclusion

`Bucket unchanged`

mod.rs stays in `Larger refactor but defer`. The third pass mainly confirms that this is shared contract territory, so compatibility coverage matters more than cleanup symmetry.
