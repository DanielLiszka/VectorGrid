# Second-Pass Review: `crates/app_core/src/composite_validate.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_validate.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/composite_validate.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this validation owner as mostly sound but broad, with route-diagnostic completeness and capability gating as the most plausible bounded cleanup areas.

## Stale Check

The live source still owns:

- composite-family validation execution
- route diagnostics
- CPU/GPU CSCV-PBO and SPA behavior

The first-pass packet remains current.

## Second-Pass Research Log

1. Bailey et al. PBO / CSCV paper
   - Source: <https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf>
   - Why consulted in second pass:
     - verify the methodology surface the file is enforcing
   - Additional constraint informed:
     - methodology cleanup should reuse the strong existing parity coverage already present here

2. NVIDIA multi-GPU guide
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/03-advanced/multi-gpu-systems.html>
   - Why consulted in second pass:
     - ground backend-specific capability gating
   - Additional constraint informed:
     - route-sensitive capability differences should stay explicit rather than hidden behind global claims

## Delta vs First Pass

### 1. Route diagnostics are structurally incomplete because several families are still omitted

- Why this matters:
  - diagnostics coverage itself is currently family-biased
- Practical implication:
  - still a good bounded cleanup target if this file is touched

### 2. `cross_ticker` support remains route-sensitive, not globally unsupported

- Why this matters:
  - the fixed-candidate GPU path hard-errors while CPU validation still owns the feature
- Practical implication:
  - preserve explicit route-sensitive capability reporting

## Verification If Changed

- rerun existing fixed-candidate CPU/GPU parity tests
- add a family-coverage test for route diagnostics
- add a targeted GPU `cross_ticker` rejection test

## Regression And Feature-Preservation Constraints

- Preserve current CPU/GPU parity behavior for CSCV/PBO and SPA outputs
- Preserve explicit GPU `cross_ticker` rejection until the route truly supports it end to end
- Reuse existing parity harnesses rather than inventing a parallel methodology path

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that route diagnostics are still structurally incomplete and that `cross_ticker` remains a route-specific capability boundary, not a universal yes/no feature.
