# Third-Pass Review: `crates/app_core/src/strategy_presets.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_presets.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_presets.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_presets.rs.review.md`
- Second-pass action bucket: `Document only`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

The second-pass packet treated these presets as curated fixtures and stopped there. Another pass was warranted because these fixtures inherit more from `StrategyConfig::default()` than the current review set was giving them credit for.

## Third-Pass Research Log

1. Rust `Default`
   - Source: <https://doc.rust-lang.org/std/default/trait.Default.html>
   - Why consulted in third pass:
     - judge how much preset stability currently depends on upstream default values rather than explicit local literals
   - Third-pass constraint informed:
     - fixture behavior can drift when `Default` changes even if this file does not

2. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - re-check whether "document only" is enough for a canonical preset catalog used by downstream tests and workflows
   - Third-pass constraint informed:
     - important fixture contracts should be pinned more explicitly than narrative docs alone

## What Third Pass Sharpened

### 1. These presets are not only curated fixtures; many are still default-sensitive fixtures

- Evidence:
  - the preset builders still start from `StrategyConfig::default()`
  - several functions override only a small subset of fields
- Why this matters:
  - a future `Default` change can silently alter benchmark inputs, parity fixtures, and route coverage without this file changing at all
- Practical implication:
  - `Document only` is too weak for a catalog that acts as a canonical preset source

### 2. Local tests still pin only part of the fixture contract

- Evidence:
  - current tests focus on grouped membership and selected route/parity expectations
  - they do not locally pin all salient invariants for the broader preset set
- Why this matters:
  - fixture drift can still leak through while the file's own tests remain green
- Practical implication:
  - the best bounded hardening is explicit invariant tests or more explicit fixture construction

## Implementation Priority

- Highest-ROI hardening:
  - reduce hidden dependency on `StrategyConfig::default()` or pin the inherited invariants with stronger local tests

## Verification If Changed

- `cargo test -p ta_app_core strategy_presets::tests:: -- --nocapture`
- representative route/parity lanes that consume these presets

## Regression And Feature-Preservation Constraints

- Preserve exact preset values and case membership
- Preserve downstream benchmark and test behavior
- If using explicit struct literals or invariant tests, do not change live preset semantics while making them less default-sensitive

## Third-Pass Conclusion

`Second-pass bucket changed`

The third pass upgrades `strategy_presets.rs` from `Document only` to `Small surgical refactor`. The stronger issue is no longer only fixture intent documentation; it is that these canonical presets still inherit enough from `StrategyConfig::default()` that future default drift can silently alter benchmark, route, and parity coverage.
