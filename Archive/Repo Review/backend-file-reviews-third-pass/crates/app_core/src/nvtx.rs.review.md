# Third-Pass Review: `crates/app_core/src/nvtx.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/nvtx.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/nvtx.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/nvtx.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a support or instrumentation seam still looked like the right framing under fresh research. The expanded third pass mainly checked whether this file had drifted into a larger production redesign target and found the same answer: the live risk is still helper truthfulness, scope clarity, and keeping support behavior explicit.

## Third-Pass Research Log

1. Rust testing guidance
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why consulted in third pass:
     - reconfirm what shared helpers should optimize for when many tests or workflows depend on them
   - Third-pass constraint informed:
     - support helpers should make setup, scope, and assertions explicit rather than implicitly global

2. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether the file still belongs in support and invariant territory
   - Third-pass constraint informed:
     - support and instrumentation seams should stay honest about scope, lifetime, and helper guarantees

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; this file remains a support seam rather than a redesign target
- Why this matters:
  - The second-pass call of Bucket unchanged still fits.
- Practical implication:
  - Keep the existing bucket and prefer clearer scope, fixture, or instrumentation assumptions over abstraction cleanup.

### 2. Helper scope and truthfulness matter more than convenience
- Why this matters:
  - Support and instrumentation code is easy to over-trust when it hides shared state, publication order, or no-op behavior.
- Practical implication:
  - If this area changes, lead with targeted scope and behavior tests before adding convenience abstraction.

## Implementation Priority

- Keep helper and instrumentation assumptions explicit and test-backed.
- Avoid broad convenience cleanup that hides scope, lifetime, or fixture truth.

## Verification If Changed

- cargo test -p ta_app_core nvtx -- --nocapture
- representative helper or instrumentation tests for the owning flow

## Regression And Feature-Preservation Constraints

- Do not make support helpers silently depend on global mutable state.
- Keep instrumentation and helper behavior explicit enough for callers to reason about.
- Avoid broad support refactors unless the affected test or artifact flows move together.

## Third-Pass Conclusion

`Bucket unchanged`

nvtx.rs stays in Small surgical refactor. The third pass mainly confirms that this file is still a support and instrumentation seam where explicit scope and helper truth matter more than structural cleanup.
