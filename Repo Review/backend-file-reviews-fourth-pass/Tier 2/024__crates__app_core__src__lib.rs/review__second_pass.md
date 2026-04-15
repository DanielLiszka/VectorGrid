# Second-Pass Review: `crates/app_core/src/lib.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/lib.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/lib.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated the crate root as compatibility-sensitive because it publishes both root re-exports and nested public modules.

## Stale Check

The live source still exports:

- flat root re-exports
- still-public nested modules
- CUDA-gated and CPU-only surfaces side by side

The first-pass packet remains current.

## Second-Pass Research Log

1. Cargo features
   - Source: <https://doc.rust-lang.org/cargo/reference/features.html>
   - Why consulted in second pass:
     - ground CPU/CUDA export expectations
   - Additional constraint informed:
     - feature-gated symbol availability remains part of the crate-root contract

2. Rust reference on conditional compilation
   - Source: <https://doc.rust-lang.org/reference/conditional-compilation.html>
   - Why consulted in second pass:
     - verify how feature-gated exports should be interpreted
   - Additional constraint informed:
     - CPU-only and CUDA-enabled builds still publish meaningfully different surfaces by design

3. Rust reference on visibility and privacy
   - Source: <https://doc.rust-lang.org/reference/visibility-and-privacy.html>
   - Why consulted in second pass:
     - reassess public-surface implications of facade cleanup
   - Additional constraint informed:
     - root and nested exports together still form a compatibility-sensitive surface

## Delta vs First Pass

### 1. The crate root is effectively publishing two API styles at once

- Why this matters:
  - cleanup can break downstream imports even if one path still exists
- Practical implication:
  - the safest improvement remains clearer documentation and visibility intent, not pruning

### 2. Several exports still look infrastructure-internal rather than intentionally stable

- Practical implication:
  - if visibility is revisited, do it as explicit public-surface policy rather than opportunistic cleanup

## Verification If Changed

- `cargo test -p ta_app_core --lib --no-run`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel --lib --no-run`

## Regression And Feature-Preservation Constraints

- Preserve existing downstream imports through both root re-exports and nested modules
- Preserve CPU-only builds plus CUDA-gated symbols
- Treat crate-root cleanup as library-compatibility work

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the crate root still publishes two overlapping public API styles, which makes facade pruning riskier than it first appears.
