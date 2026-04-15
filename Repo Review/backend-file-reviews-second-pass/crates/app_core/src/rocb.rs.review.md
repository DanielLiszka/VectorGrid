# Second-Pass Review: `crates/app_core/src/rocb.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/rocb.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/rocb.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this strategy file as mostly sound but sensitive because defaults, first-valid handling, and silent desired-position fallback all affect route parity and correctness.

## Stale Check

The live source still:

- uses extensive serde defaults
- depends on first-valid fallback behavior
- silently returns from malformed desired-position buffer writes

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde default fields
   - Source: <https://serde.rs/attr-default.html>
   - Why consulted in second pass:
     - ground the default-sensitive public config surface
   - Additional constraint informed:
     - default changes remain compatibility-sensitive, not local cleanup

2. Rust `OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - re-check one-time descriptor caching semantics
   - Additional constraint informed:
     - current one-time indicator-descriptor caching remains intentional

3. Rust API Guidelines `C-VALIDATE`
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate>
   - Why consulted in second pass:
     - reassess the desired-position and config-validation behavior
   - Additional constraint informed:
     - silent return remains the sharpest local contract weakness

## Delta vs First Pass

### 1. First-valid fallback matters more than the first pass emphasized because it protects parity across CPU and CUDA-adapted indicator paths

### 2. `RocbStrategyParams` remains a persisted/default-sensitive contract

- Practical implication:
  - treat default changes as compatibility changes, not cleanup

## Verification If Changed

- `cargo test -p ta_app_core rocb -- --nocapture`
- representative ROCB paths in `composite_cpu` and `composite_validate`

## Regression And Feature-Preservation Constraints

- Keep current default period mapping between ROCB and fast/slow optimization axes
- Preserve first-valid expected-or-scan resilience
- Do not change desired-position trading semantics without parity checks

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly tightens the importance of first-valid fallback and the compatibility sensitivity of ROCB’s default-heavy config surface.
