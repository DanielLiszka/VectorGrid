# Second-Pass Review: `crates/app_core/src/test_support.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/test_support.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/test_support.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as tiny shared test infrastructure where the main risk is global-lock semantics rather than complexity.

## Stale Check

The live source still owns:

- a shared process-global test lock
- `OnceLock`-backed initialization
- poison-sensitive `Mutex` handling

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `Mutex`
   - Source: <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
   - Why consulted in second pass:
     - re-check poisoning semantics and limitations
   - Additional constraint informed:
     - poisoning is advisory and not a soundness guarantee, which makes `expect(\"test env lock poisoned\")` even less attractive as a cascading-failure mechanism

2. `OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - confirm the one-time lazy global-init pattern
   - Additional constraint informed:
     - the global helper nature of this file remains explicit

3. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground intent documentation even for small internal modules
   - Additional constraint informed:
     - this file should say more clearly that it is a shared process-global coordinator

## Delta vs First Pass

### 1. Poison handling is an even weaker justification for cascading test failure than the first pass implied

### 2. The file is clearly shared global test coordination, not a local helper

- Practical implication:
  - documentation and bounded poison handling remain the right small improvements

## Verification If Changed

- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
- compare against a serialized `--test-threads=1` run

## Regression And Feature-Preservation Constraints

- Preserve the single shared test lock unless env-sensitive suites are re-audited
- Preserve current serialized coordination intent
- Do not weaken test isolation while softening poison handling

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that poison semantics are weaker than they look and that this file is deliberately global test coordination.
