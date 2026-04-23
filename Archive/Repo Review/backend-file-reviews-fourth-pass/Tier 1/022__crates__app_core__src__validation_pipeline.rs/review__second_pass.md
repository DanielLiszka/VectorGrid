# Second-Pass Review: `crates/app_core/src/validation_pipeline.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_pipeline.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_pipeline.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as methodology-heavy validation orchestration where recommendation mapping, statistical defaults, and candidate collection all have broad downstream impact.

## Stale Check

The live source still owns:

- run verdict construction
- CSCV, SPA, and walk-forward defaults
- parallel candidate collection

The first-pass packet remains current.

## Second-Pass Research Log

1. Bailey/Lopez de Prado DSR and PBO references
   - Sources:
     - <https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf>
     - Lopez de Prado / DSR material already used in first pass
   - Why consulted in second pass:
     - re-check the methodology surface behind validation defaults and verdicts
   - Additional constraint informed:
     - statistical floors and clamps remain methodology choices, not cleanup noise

2. Hansen SPA paper
   - Source: official SPA methodology reference already used in first-pass review context
   - Why consulted in second pass:
     - reassess bootstrap/significance defaults
   - Additional constraint informed:
     - SPA sizing remains policy-bearing

3. Rayon `ParallelIterator`
   - Source: <https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html>
   - Why consulted in second pass:
     - re-check collection-order semantics
   - Additional constraint informed:
     - candidate collection should not be assumed stable unless explicitly sorted later

## Delta vs First Pass

### 1. `build_run_verdict()` still compresses several distinct failure modes into `NoCandidateSurvived`

- Why this matters:
  - top-level recommendation semantics are coarser than the detailed flags beneath them
- Practical implication:
  - preserve the defer bucket rather than trying to patch verdict mapping locally

### 2. `collect_candidates_parallel()` still inherits Rayon order semantics

- Why this matters:
  - later consumers should not assume stable collection order unless they sort
- Practical implication:
  - keep ordering assumptions explicit and tested

## Verification If Changed

- `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`
- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve current statistical defaults unless methodology is intentionally changed
- Preserve deterministic test-backed behavior
- Do not casually alter recommendation mapping or candidate-collection semantics

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly clarifies that run-verdict mapping is still coarser than the detailed flags imply and that parallel candidate collection order should not be assumed stable by accident.
