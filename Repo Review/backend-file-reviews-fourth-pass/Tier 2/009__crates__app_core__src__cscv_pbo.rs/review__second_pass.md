# Second-Pass Review: `crates/app_core/src/cscv_pbo.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cscv_pbo.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/cscv_pbo.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as methodology-sensitive code where small-looking normalization or sampling changes can alter statistical meaning.

## Stale Check

The live source still owns:

- slice sanitization
- exhaustive vs sampled mask generation
- PBO-related summary metrics

The first-pass packet remains current.

## Second-Pass Research Log

1. Bailey et al. PBO / CSCV paper
   - Source: <https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf>
   - Why consulted in second pass:
     - verify the core CSCV/PBO definitions behind the implementation
   - Additional constraint informed:
     - equal IS/OOS splits remain a defensible reason for odd-slice normalization

2. Bailey, "Statistical Overfitting and Backtest Performance"
   - Source: <https://www.davidhbailey.com/dhbpapers/overfitting.pdf>
   - Why consulted in second pass:
     - ground the interpretation of performance degradation and false discovery
   - Additional constraint informed:
     - sampled-mask approximations should be described honestly

## Delta vs First Pass

### 1. `sanitize_slices` has a defensible methodology reason, but it still needs that rationale documented

- Why this matters:
  - odd-slice normalization otherwise looks arbitrary
- Practical implication:
  - documentation remains the best bounded improvement

### 2. `sample_k_masks` is an approximation, not a stratified or exhaustive CSCV enumeration

- Why this matters:
  - forced sampling changes the methodological meaning of the output
- Practical implication:
  - keep deterministic seeded behavior and describe the approximation clearly

## Verification If Changed

- extend `cscv_pbo_overfit_pattern_has_expected_phi`
- add odd-slice and sampled-combination cases
- assert exhaustive vs sampled mask counts explicitly

## Regression And Feature-Preservation Constraints

- Preserve deterministic seeded behavior for sampled mask selection
- Preserve current skip-path semantics for too-few candidates / too-short slices unless downstream consumers move with it
- Treat `omega` / `lambda` normalization as methodology-sensitive

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that odd-slice normalization is methodologically defensible, while sampled-mask generation should be treated and documented as an approximation rather than full CSCV enumeration.
