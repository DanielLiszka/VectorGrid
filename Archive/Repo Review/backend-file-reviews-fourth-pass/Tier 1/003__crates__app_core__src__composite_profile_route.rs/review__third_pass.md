# Third-Pass Review: `crates/app_core/src/composite_profile_route.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_profile_route.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/composite_profile_route.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_profile_route.rs.review.md`
- Second-pass action bucket: `Document only`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

The second pass downgraded this file to documentation-only policy logic. Another pass was warranted because the labels emitted here are not just docs; they are part of the exported route-string contract.

## Third-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in third pass:
     - re-check whether these helpers are merely labels or effectively part of a stable output contract
   - Third-pass constraint informed:
     - labels embedded into artifacts and tests are compatibility behavior, not only documentation

## What Third Pass Sharpened

### 1. These strings are not just policy labels; they already participate in exported `backend_route` behavior

- Evidence:
  - runtime-label helpers emit strings like `CompositeTrendCpu`, `RegimeHybridCpu`, and `BreakoutMomentumCpu`
  - related route strings are embedded downstream into exported route/report surfaces and asserted in tests
- Why this matters:
  - renaming or repurposing these values is no longer just label cleanup
  - it can break artifact, test, UI, or telemetry consumers that interpret route strings
- Practical implication:
  - this file should no longer be treated as `Document only`
  - the right target is small additive clarification between:
    - profile/runtime label
    - GPU target label

## Implementation Priority

- Highest-ROI hardening:
  - clarify and stabilize the distinction between profile label and GPU target label without renaming existing strings casually

## Verification If Changed

- rerun existing precedence and label tests
- rerun representative route-export tests that assert `backend_route` strings

## Regression And Feature-Preservation Constraints

- Preserve current precedence mapping for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`
- Do not rename emitted strings in place without auditing artifact, frontend, and test consumers
- Prefer additive clarification over mutating the existing string family blindly

## Third-Pass Conclusion

`Second-pass bucket changed`

The third pass upgrades `composite_profile_route.rs` from `Document only` to `Small surgical refactor`. The issue is not only label clarity anymore; these helpers already shape exported route strings, which makes them part of the live compatibility contract.
