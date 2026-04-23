# Third-Pass Review: `crates/app_core/src/validation_pipeline.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_pipeline.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_pipeline.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_pipeline.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

This file remained a high-ROI target because it owns repo-wide validation verdict semantics. The third pass focused on whether the second-pass packet was still too generic about methodology and result-shape risk.

## Third-Pass Research Log

1. Bailey and Lopez de Prado on deflated Sharpe and higher moments
   - Source: <https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf>
   - Why consulted in third pass:
     - reassess whether the current PSR/DSR/MinTRL reporting is methodologically narrower than its labels imply
   - Third-pass constraint informed:
     - using fixed Gaussian moments should be treated as approximation, not full-fidelity probability reporting

2. `arch` multiple-comparison / SPA documentation
   - Source: <https://arch.readthedocs.io/en/latest/_modules/arch/bootstrap/multiple_comparison.html>
   - Why consulted in third pass:
     - compare the repo's SPA naming and behavior to a fuller documented SPA surface
   - Third-pass constraint informed:
     - the current implementation is a narrower stationary-bootstrap significance screen than the label alone suggests

3. Rust error-handling guidance
   - Source: <https://doc.rust-lang.org/stable/book/ch09-00-error-handling.html>
   - Why consulted in third pass:
     - test whether `Option`-based SPA absence is hiding an actual contract failure
   - Third-pass constraint informed:
     - "requested but aborted/invalid" should not collapse to the same state as "not configured"

## What Third Pass Sharpened

### 1. `spa: None` is still too ambiguous because the same state can mean "not configured" or "requested but failed/aborted"

- Evidence:
  - the SPA path can return `None` for cancellation or internal shape mismatch
  - `build_run_verdict` still treats `None` as absence rather than failed requested work
- Why this matters:
  - a run can be approved after a requested SPA path was cancelled or invalid
- Practical implication:
  - the best bounded fix is additive:
    - distinguish `Skipped`, `Completed`, and `Aborted/Invalid` instead of overloading `Option`

### 2. `passed` versus `approved` is already leaking misleading semantics downstream

- Evidence:
  - `approved` can be false while `passed` remains true
  - downstream surfaces already need compensating logic to explain this state
- Why this matters:
  - the current top-level verdict shape looks cleaner than it really is
- Practical implication:
  - any contract cleanup should be additive first, not an in-place semantic rewrite that breaks current consumers

### 3. The statistical labels overstate methodological specificity

- Evidence:
  - PSR/DSR/MinTRL reporting still uses fixed Gaussian moments
  - the file still labels its significance screen as SPA even though the implementation is narrower than the label implies
  - finite bootstrap p-values can still collapse to `0.0`
- Why this matters:
  - this is not mere docs debt
  - it affects how strongly users should trust the statistical outputs
- Practical implication:
  - retain the larger deferred bucket
  - if the current approach is kept, label the approximations and simplified SPA semantics explicitly

### 4. Third-pass correction to the second-pass packet

- The earlier warning about Rayon collection order is not the highest-ROI issue here
- The stronger risks are verdict semantics and statistical-contract fidelity

## Implementation Priority

- Best bounded hardening:
  - stop collapsing requested-but-aborted SPA into plain absence
- Larger deferred work:
  - align verdict fields and statistical labels with what the engine actually computes

## Verification If Changed

- `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`
- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

Add targeted coverage for:

- SPA requested but cancelled/invalid
- additive fields that distinguish hard-gate pass from approval
- any labeling or value changes to PSR/DSR/MinTRL or SPA output

## Regression And Feature-Preservation Constraints

- Preserve current statistical defaults unless methodology is intentionally changed
- Prefer additive contract fields over in-place semantic rewrites
- Do not silently change approval behavior or report labels without explicit migration notes

## Third-Pass Conclusion

`Bucket unchanged, but the verdict contract is weaker than second pass emphasized`

`validation_pipeline.rs` still belongs in `Larger refactor but defer`, but the third pass identifies a sharper safe-now hardening target: requested SPA work should not disappear into plain `None`. Beyond that, the file still needs a more honest mapping between its top-level verdict fields, its statistical labels, and the narrower methodology it actually implements.
