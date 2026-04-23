# Third-Pass Review: `crates/app_core/src/vemr.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vemr.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vemr.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vemr.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a strategy-owner CPU policy surface still looked like the right framing under fresh research.

## Third-Pass Research Log

1. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether these strategy-local policy surfaces should validate or simplify more aggressively
   - Third-pass constraint informed:
     - strategy defaults and selector semantics should remain explicit and test-backed, not casually normalized away

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in third pass:
     - reconfirm how strongly defaulted config behavior should be treated as active contract
   - Third-pass constraint informed:
     - defaulted strategy config still behaves like product policy once callers depend on it

3. Serde container attributes
   - Source: <https://serde.rs/container-attrs>
   - Why consulted in third pass:
     - check whether stricter config parsing would be safe at this layer
   - Third-pass constraint informed:
     - tightening config semantics here is a compatibility decision that needs coordinated rollout

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; this file still reads as product policy more than cleanup
- Why this matters:
  - The second-pass call of `Bucket unchanged` still fits.
  - Defaults, selector meanings, route notes, and fail-open behavior remain the compatibility-sensitive parts of this surface.
- Practical implication:
  - Keep the current bucket and avoid broad simplification unless CPU/GPU parity and route-truthfulness tests move with it.

### 2. Future cleanup has to be parity-aware
- Why this matters:
  - These files sit close to strategy-local behavior that is mirrored or constrained by GPU and validation routes.
- Practical implication:
  - Any cleanup should be paired with representative parity and route-truthfulness coverage rather than isolated refactor.

## Implementation Priority

- Prioritize parity and route-truthfulness tests over abstraction cleanup.
- Treat defaults and selector behavior as intentional product policy until a wider redesign is approved.

## Verification If Changed

- cargo test -p ta_app_core vemr -- --nocapture
- representative CPU/GPU parity or route-selection tests for the owning strategy family

## Regression And Feature-Preservation Constraints

- Preserve current defaults, selector meanings, and route-diagnostic semantics.
- Do not tighten fail-open behavior without coordinated product and parity review.
- Keep strategy-local organization unless a wider backend redesign is underway.

## Third-Pass Conclusion

`Bucket unchanged`

vemr.rs stays in `Larger refactor but defer`. The third pass mainly reinforces that this file is a strategy-policy surface, so compatibility and parity matter more than cleanup symmetry.
