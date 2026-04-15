# Second-Pass Review: `crates/app_core/src/vram_ma.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vram_ma.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vram_ma.rs.review.md`
- First-pass action bucket: `Document only`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as a tiny public seam whose main importance is feature-gated re-export stability.

## Stale Check

The live source still provides:

- feature-gated public re-exports for CUDA moving-average support

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust reference on conditional compilation
   - Source: <https://doc.rust-lang.org/reference/conditional-compilation.html>
   - Why consulted in second pass:
     - verify the feature-gate contract on this seam
   - Additional constraint informed:
     - the wrapper remains a real public API boundary, not redundant glue

2. Rustdoc re-export guidance
   - Source: <https://doc.rust-lang.org/beta/rustdoc/write-documentation/re-exports.html>
   - Why consulted in second pass:
     - confirm that public re-exports are documented API surface
   - Additional constraint informed:
     - bypassing or removing this wrapper would be a public API shift even if runtime behavior stayed the same

3. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground intent documentation for a very small public seam
   - Additional constraint informed:
     - documentation remains the right scope here

## Delta vs First Pass

### 1. The file is more clearly an app-core public seam than "redundant wrapper" suggests

- Why this matters:
  - feature-gated re-exports are still part of the crate’s public contract
- Practical implication:
  - keep this file in `Document only`

## Verification If Changed

- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_backtest_kernel -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve the current re-export path and feature gate
- Preserve CPU-only builds and current import paths
- Do not bypass this seam without coordinated API review

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Document only` bucket remains correct. The second pass mainly clarifies that this tiny file is still a public API seam because the feature-gated re-exports are user-visible contract.
