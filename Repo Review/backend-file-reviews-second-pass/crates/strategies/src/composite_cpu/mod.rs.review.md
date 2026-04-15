# Second-Pass Review: `crates/strategies/src/composite_cpu/mod.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/composite_cpu/mod.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/strategies/src/composite_cpu/mod.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a foundational shared CPU strategy engine whose signal-mode semantics, overlay behavior, and internal ownership split are too important to "clean up" casually.

## Stale Check

The live source still owns:

- `CompositeSignalMode` and desired-position semantics
- execution overlay params and state helpers
- main metrics/curve evaluation entry points

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess public-behavior documentation expectations for exported modes and entry points
   - Additional constraint informed:
     - `CompositeSignalMode` still deserves explicit local semantics docs because it is product-facing config

2. Rust Book testing guidance
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why consulted in second pass:
     - re-evaluate how much of this module’s safety comes from behavior-oriented tests
   - Additional constraint informed:
     - the local test suite remains the main semantic guardrail and supports keeping structural cleanup deferred

## Delta vs First Pass

### 1. `CompositeSignalMode` remains the strongest public-semantics surface and still should not be treated as cosmetic enum cleanup

### 2. The overlay path is still part of behavior, not merely internal state plumbing

- Practical implication:
  - documentation remains the safest immediate improvement, and the broader refactor still belongs in the defer bucket

## Verification If Changed

- `cargo test -p ta_strategies -- --nocapture`
- representative `composite_cpu`, `composite_mo`, and `composite_validate` consumers in `ta_app_core`

## Regression And Feature-Preservation Constraints

- Preserve current `CompositeSignalMode` entry and exit semantics
- Preserve current overlay behavior and resulting trade timing/risk semantics
- Do not split this file structurally without re-running the local behavior suite and downstream parity flows

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly reinforces that `CompositeSignalMode` and overlay behavior are still the dominant public contracts, so safe work remains documentation and clarification rather than structural change.
