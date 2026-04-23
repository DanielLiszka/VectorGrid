# Second-Pass Review: `crates/optimizer/src/mo.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/mo.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/mo.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as the algorithmic optimizer hub whose defaults, duplicate handling, and many responsibilities make safe local cleanup difficult.

## Stale Check

The live source still owns:

- search-space modeling and validation
- NSGA-II, Sobol QMC, and TPE behavior
- duplicate avoidance through exact parameter hashing

The first-pass packet remains current.

## Second-Pass Research Log

1. `sobol` crate docs
   - Source: <https://docs.rs/sobol/latest/sobol/>
   - Why consulted in second pass:
     - re-check the low-discrepancy QMC path
   - Additional constraint informed:
     - the Sobol route remains algorithmic policy, not generic randomness

2. Algorithms for Hyper-Parameter Optimization
   - Source: <https://papers.nips.cc/paper/4443-algorithms-for-hyper-parameter-optimization.pdf>
   - Why consulted in second pass:
     - ground TPE policy context
   - Additional constraint informed:
     - TPE defaults remain algorithmic choices, not incidental constants

3. NSGA-II paper
   - Source: <https://repository.ias.ac.in/83498/1/2-a.pdf>
   - Why consulted in second pass:
     - re-check non-dominated sorting and crowding semantics
   - Additional constraint informed:
     - current frontier behavior remains semantics-sensitive

## Delta vs First Pass

### 1. Search-family defaults are more contract-heavy than the first pass emphasized

- Why this matters:
  - NSGA-II, Sobol, and TPE defaults collectively define optimizer policy

### 2. Duplicate avoidance is intentionally exact-bit-pattern based

- Why this matters:
  - `ParamValue::Float(f.to_bits())` means near-equality is not deduped
- Practical implication:
  - preserve this for reproducibility unless optimizer outputs are intentionally re-baselined

## Verification If Changed

- `cargo test -p ta_optimizer -- --nocapture`
- `cargo test -p ta_app_core composite_mo -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve bounded-grid acceptance rules
- Preserve duplicate-avoidance semantics
- Preserve current heuristic defaults unless optimizer outputs are intentionally re-baselined and documented

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly clarifies that optimizer-family defaults and exact-bit duplicate handling are stronger product contract than they first appeared.
