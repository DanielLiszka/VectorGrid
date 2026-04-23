# Third-Pass Review: `crates/app_core/src/options_signal_manifest.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_signal_manifest.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_signal_manifest.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_signal_manifest.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the options signal manifest builder still looked like the right framing under fresh research. The third pass tightened the packet around two contract issues: out-of-range trade indexes are clamped into edge bars instead of failing validation, and the helper hardcodes UTC calendar and session policy rather than preserving dataset provenance.

## Third-Pass Research Log

1. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether index clamping should still be described as a small helper choice
   - Third-pass constraint informed:
     - manifest builders should not silently fabricate provenance-significant positions without explicit policy

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in third pass:
     - ground how much of the manifest shape becomes durable emitted contract
   - Third-pass constraint informed:
     - hardcoded manifest fields should be treated as emitted behavior once downstream consumers depend on them

## What Third Pass Sharpened

### 1. Index clamping can fabricate edge-bar provenance instead of rejecting invalid input
- Evidence:
  - clamp_index turns out-of-range trade indexes into start or end-bar selections.
- Why this matters:
  - That makes invalid source indexing look like a legitimate edge-bar event inside the manifest.
- Practical implication:
  - Keep the existing bucket, but treat this as provenance fabrication risk rather than a harmless convenience helper.

### 2. Calendar and session identity are currently hardcoded policy
- Evidence:
  - The manifest path hardcodes calendar_id = market_hours_utc and derives UTC-date session IDs.
- Why this matters:
  - This bakes calendar policy into emitted manifests instead of preserving source-dataset provenance.
- Practical implication:
  - If hardened, separate dataset provenance from default policy rather than just renaming helper fields.

## Implementation Priority

- Clarify whether out-of-range indexes should remain clamped or become explicit validation failures.
- Make calendar and session provenance policy explicit before expanding manifest consumers.

## Verification If Changed

- cargo test -p ta_app_core options_signal_manifest -- --nocapture
- representative manifest fixtures with out-of-range trade indexes and non-default calendar expectations

## Regression And Feature-Preservation Constraints

- Preserve current manifest schema unless downstream consumers move together.
- Do not silently change index-clamping behavior without fixture coverage.
- Keep calendar and session policy explicit if it remains hardcoded.

## Third-Pass Conclusion

`Bucket unchanged, but provenance policy sharpened`

options_signal_manifest.rs stays in Small surgical refactor. The third pass mainly sharpens two points: out-of-range indexes are still turned into edge-bar events, and emitted manifest calendar and session identity is still hardcoded policy rather than preserved provenance.
