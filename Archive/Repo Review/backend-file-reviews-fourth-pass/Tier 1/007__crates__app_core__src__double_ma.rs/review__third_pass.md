# Third-Pass Review: `crates/app_core/src/double_ma.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

This file remains one of the central runtime owners in the repo. The second-pass packet was still useful, but another pass was warranted because explicit backend intent and follow-up request fidelity are both easy to overstate from the public DTO shapes.

## Third-Pass Research Log

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in third pass:
     - reassess whether the public request/result shapes are more misleading than the second pass stated
   - Third-pass constraint informed:
     - explicit caller intent should not be silently weakened by helper normalization

2. Rust `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in third pass:
     - keep follow-up cache ownership and lifecycle assumptions grounded
   - Third-pass constraint informed:
     - follow-up hardening should preserve local cache ownership rather than widening stale-data scope

## What Third Pass Sharpened

### 1. Explicit non-`Auto` GPU device choice is not just vulnerable to accidental overwrite; it is unconditionally rewritten too early

- Evidence:
  - `with_preferred_cuda_device()` rewrites `device_id` for all GPU-bearing backend variants, not only `Auto`
  - that rewrite still happens before route resolution
- Why this matters:
  - explicit caller-selected GPU affinity can be lost even when the request did not ask for automatic device choice
- Practical implication:
  - this remains the strongest bounded bug candidate in the file

### 2. The follow-up request shapes are less self-sufficient than `DoubleMaParamsResolved` suggests

- Evidence:
  - follow-up paths still consult top-level MA params and request-side fields beyond `params: DoubleMaParamsResolved`
  - cached backend hints can still influence the follow-up execution path
- Why this matters:
  - the public follow-up DTOs imply that the resolved params are nearly self-contained when they are not
- Practical implication:
  - the third-pass packet should now frame this as a real API fidelity issue, not only a generic "footgun"

## Implementation Priority

- Highest-ROI hardening:
  - preserve explicit non-`Auto` device choices
- Next bounded improvement:
  - make follow-up request semantics more truthful relative to `DoubleMaParamsResolved`

## Verification If Changed

- targeted `double_ma.rs` tests for explicit device preservation
- targeted follow-up round-trip tests built from `DoubleMaParamsResolved`
- `cargo test -p ta_app_core double_ma -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve explicit non-`Auto` GPU `device_id` selections
- Preserve current follow-up cache ownership model
- Strengthen follow-up request fidelity without widening stale-cache risk
- Preserve public DTO compatibility unless a broader API revision is intentional

## Third-Pass Conclusion

`Bucket unchanged, but the contract is sharper`

`double_ma.rs` still fits `Small surgical refactor`, but the third pass clarifies the two best targets. Explicit GPU device selection is being overwritten more aggressively than the second pass implied, and the follow-up DTOs still promise more self-sufficiency than they actually provide.
