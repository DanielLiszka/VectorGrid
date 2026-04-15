# Third-Pass Review: `crates/app_core/src/composite_validate.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_validate.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/composite_validate.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_validate.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

The second-pass packet kept this file in the small bucket, but another pass was warranted because the wrapper/core contract split and route-diagnostics behavior are both broader than that rating suggested.

## Third-Pass Research Log

1. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether the visible request fields are semantically stronger than the app-core layer really honors
   - Third-pass constraint informed:
     - public inputs should not be accepted and then effectively delegated away without a clear contract

2. ARCH SPA reference
   - Source: <https://bashtage.github.io/arch/doc/multiple-comparison/generated/arch.bootstrap.SPA.html>
   - Why consulted in third pass:
     - keep inherited SPA ambiguity grounded in the validation surface
   - Third-pass constraint informed:
     - nullable SPA results remain too weak a contract here as well

## What Third Pass Sharpened

### 1. The wrapper/core ownership split is stronger than the second pass stated

- Evidence:
  - the core request still exposes `cross_ticker` and `validation_context`
  - the wrapper validates context, strips `cross_ticker`, reapplies it later, and overwrites `runtime_ms`
- Why this matters:
  - the visible app-core API still implies ownership of semantics that really live at the wrapper layer
- Practical implication:
  - this should no longer be treated as small cleanup

### 2. Route diagnostics are not merely incomplete; they are family-biased in a way mixed-family requests can observe

- Evidence:
  - route-diagnostic clearing and capture still key off the first candidate/family
  - mixed-family CPU requests remain possible
  - GPU preparation still starts from the first family
- Why this matters:
  - mixed-family requests can miss later-family diagnostics or retain stale diagnostics from a previous run
- Practical implication:
  - this pushes the file into `Larger refactor but defer`

### 3. The file also inherits the stronger SPA ambiguity from `validation_pipeline.rs`

- Practical implication:
  - keep any SPA contract correction aligned across validation owners instead of patching only this file

## Implementation Priority

- Highest-value redesign target:
  - separate wrapper-owned request/result semantics from the app-core validation API
- Companion redesign target:
  - make route diagnostics explicit for mixed-family behavior rather than first-family-biased by accident

## Verification If Changed

- rerun existing composite validation CPU/GPU parity tests
- add mixed-family diagnostics coverage
- add targeted SPA-requested versus SPA-skipped tests

## Regression And Feature-Preservation Constraints

- Preserve current uniform-family GPU fast paths and their parity tests
- If mixed-family requests are rejected or normalized, do it explicitly and update caller expectations
- Preserve wrapper-owned cross-ticker filtering and run-verdict rebuilding
- Do not silently change backend-route or diagnostics semantics in artifacts

## Third-Pass Conclusion

`Second-pass bucket changed`

The third pass upgrades `composite_validate.rs` from `Small surgical refactor` to `Larger refactor but defer`. The file’s real problem is no longer just diagnostics completeness; it is that wrapper-owned request semantics and first-family-biased route diagnostics are both broader and more compatibility-sensitive than the second pass rated them.
