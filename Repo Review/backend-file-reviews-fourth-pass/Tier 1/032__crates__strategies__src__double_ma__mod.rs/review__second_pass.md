# Second-Pass Review: `crates/strategies/src/double_ma/mod.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/double_ma/mod.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/strategies/src/double_ma/mod.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as the foundational shared strategy module for the workspace, with too many public DTO/default and hot-path responsibilities to justify casual local cleanup.

## Stale Check

The live source still owns:

- foundational public DTOs and risk metrics
- `StrategyConfig` defaults and helper surface
- batch and single evaluation engines

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess omission-default and foundational DTO documentation expectations
   - Additional constraint informed:
     - `StrategyConfig` omission defaults remain one of the most important public contracts in the shared strategy surface

2. Rayon `ParallelIterator`
   - Source: <https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html>
   - Why consulted in second pass:
     - re-check how much batch evaluation semantics depend on Rayon behavior
   - Additional constraint informed:
     - parallel batch behavior remains part of performance-sensitive shared semantics and should not be casually reworked

## Delta vs First Pass

### 1. `StrategyConfig` omission defaults remain even more central than the first pass emphasized because they flow into nearly every optimizer and validation path

### 2. Parallel batch behavior still deserves to be treated as semantics-sensitive infrastructure, not just speed optimization

- Practical implication:
  - the defer bucket remains the right posture; safe immediate work is still documentation and contract clarification

## Verification If Changed

- `cargo test -p ta_strategies -- --nocapture`
- representative downstream `double_ma`, `composite_cpu`, and optimizer flows in `ta_app_core` and `ta_optimizer`

## Regression And Feature-Preservation Constraints

- Preserve current `StrategyConfig` omission/default behavior unless a broader migration is planned
- Preserve current metric finalization and batch-evaluation semantics
- Do not split responsibilities here without cross-crate parity review

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly reinforces that `StrategyConfig` defaults and shared batch-evaluation behavior remain foundational cross-crate contract, which keeps structural cleanup safely deferred.
