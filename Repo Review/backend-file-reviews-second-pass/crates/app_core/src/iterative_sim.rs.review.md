# Second-Pass Review: `crates/app_core/src/iterative_sim.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/iterative_sim.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/iterative_sim.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already saw result-shape and error-shape awkwardness here, but kept the bucket small pending a deeper look at how much of that awkwardness is active public contract.

## Stale Check

The live source still:

- returns `Metrics`
- collapses invalid input to `Metrics::default()`
- uses default-heavy public config, including `IntrabarRule::Pessimistic`

The second pass found enough public-surface pressure to raise the bucket.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - re-check public config/result/error expectations
   - Additional constraint informed:
     - default-to-zero metrics on invalid input overpromises result validity

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground predictable public behavior review
   - Additional constraint informed:
     - both error shape and result shape want a clearer contract than the current API provides

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - verify the impact of the default-heavy config surface
   - Additional constraint informed:
     - defaults here are product policy, not neutral convenience

## Delta vs First Pass

### 1. The public surface now looks overpromising in two directions at once: error handling and result validity

- Evidence:
  - invalid input still collapses into `Metrics::default()`
  - many structurally zero fields still look authoritative when returned
- Why this matters:
  - callers cannot easily distinguish "real zero metrics" from "invalid input gave me fallback metrics"
- Practical implication:
  - the bucket should move to `Larger refactor but defer`

### 2. `IntrabarRule::Pessimistic` being the default makes this file product-policy-bearing, not just utility code

- Practical implication:
  - future redesign should preserve semantics and ideally add a clearer `Result`-returning sibling rather than silently repurposing the current path

## Verification If Changed

- `cargo test -p ta_app_core iterative_ -- --nocapture`
- higher-level iterative validation delta tests

## Regression And Feature-Preservation Constraints

- Preserve intrabar semantics, cooldown behavior, and persistence behavior
- Preserve current validation-engine parity checks
- Do not silently reinterpret default-zero metrics as stronger contract than they are

## Second-Pass Conclusion

`First-pass bucket changed`

The second pass upgrades `iterative_sim.rs` from `Small surgical refactor` to `Larger refactor but defer`. The reason is no longer just style or docs debt: the current public surface overpromises both result validity and error semantics.
