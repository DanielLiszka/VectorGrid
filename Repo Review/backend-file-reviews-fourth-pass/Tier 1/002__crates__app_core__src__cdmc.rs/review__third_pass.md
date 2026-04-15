# Third-Pass Review: `crates/app_core/src/cdmc.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cdmc.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/cdmc.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cdmc.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

The second pass already raised a real strategy-semantics concern, but this file stayed high ROI because it also sits on a backend-selection boundary where GPU preference and CPU fallback semantics can drift silently.

## Third-Pass Research Log

1. VectorTA Correlation Cycle docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/correlation_cycle/>
   - Why consulted in third pass:
     - re-check what the upstream indicator claims about `state` and thresholding
   - Third-pass constraint informed:
     - the repo is layering its own regime policy on top of an upstream trend/cycling classifier

2. VectorTA MAMA docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/mama/>
   - Why consulted in third pass:
     - verify whether the bigger remaining issue is still regime meaning rather than MAMA parameter validation
   - Third-pass constraint informed:
     - MAMA handling still looks less urgent than backend downgrade and regime policy

3. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - check whether silent backend downgrade should be treated as contract debt
   - Third-pass constraint informed:
     - backend selection and fallback behavior should not silently weaken an explicitly preferred route

## What Third Pass Sharpened

### 1. Silent GPU downgrade is now the sharper issue, ahead of the already-known regime-mapping concern

- Evidence:
  - CUDA indicator and MAMA/decycler paths can fall back to CPU on failure
  - the route still treats non-`CpuOnly` backends as GPU-preferring
  - the downgrade does not surface as a strong route note or hard error
- Why this matters:
  - this weakens `GpuOnly` and `GpuKernel` style semantics in a way the caller can miss
- Practical implication:
  - future hardening should split:
    - opportunistic fallback for `Auto` or explicitly hybrid routes
    - strict failure or explicit downgrade signaling for strict GPU routes

### 2. The second-pass regime-semantics concern still stands, but it should now be framed as repo policy layered onto upstream indicator output

- Evidence:
  - `cycle_state` sign still drives trend-vs-reversion behavior
  - non-finite state still defaults to trend
  - the file still hardcodes the Correlation Cycle threshold at `9.0`
- Why this matters:
  - this is not only a possible upstream-contract misread
  - it is also a product-policy choice that needs documentation and parity tests if retained
- Practical implication:
  - do not "fix" this locally without reviewing CPU/GPU parity and strategy expectations together

## Implementation Priority

- Highest-ROI hardening:
  - make strict GPU route downgrade behavior explicit
- Larger semantics-sensitive work still deferred:
  - review and document the repo-specific meaning attached to `cycle_state` and threshold `9.0`

## Verification If Changed

- `cargo test -p ta_app_core cdmc -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

Also verify:

- CPU/GPU parity remains intact
- route metadata or diagnostics clearly show strict GPU downgrade behavior if introduced

## Regression And Feature-Preservation Constraints

- Preserve CPU/GPU numerical parity across `cdmc.rs`, `cdmc_gpu_kernel.rs`, and related validation paths
- Preserve current product capability for hybrid and CPU fallback routes where intentionally supported
- Do not silently reinterpret `cycle_state` or threshold policy without targeted parity review

## Third-Pass Conclusion

`Bucket unchanged, but silent route downgrade is now the top issue`

`cdmc.rs` still belongs in `Larger refactor but defer`, but the third pass changes the ordering of concerns. The highest-ROI problem is no longer only the `state == 0` semantics question; it is also that strict GPU preference can still collapse into CPU execution without a strong caller-visible signal.
