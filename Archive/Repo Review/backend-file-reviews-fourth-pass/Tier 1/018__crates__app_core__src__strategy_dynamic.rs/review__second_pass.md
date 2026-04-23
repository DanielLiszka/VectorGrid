# Second-Pass Review: `crates/app_core/src/strategy_dynamic.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_dynamic.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_dynamic.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a broad dynamic-strategy owner whose output-shape enforcement, artifact logging, and shared global state make safe local cleanup difficult.

## Stale Check

The live source still:

- appends artifacts best-effort through env-driven paths
- enforces the current GPU-v1 output contract across CPU staging
- relies on process-global env handling and shared counters in tests

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - ground lazy compile and context initialization behavior
   - Additional constraint informed:
     - lazy initialization remains deliberate, not accidental complexity

2. Rust `OpenOptions`
   - Source: <https://doc.rust-lang.org/nightly/std/fs/struct.OpenOptions.html>
   - Why consulted in second pass:
     - verify append/create artifact semantics for env-driven logging
   - Additional constraint informed:
     - current artifact append is best-effort observability, not semantic workflow ownership

3. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess behavior-heavy helper contracts
   - Additional constraint informed:
     - the safe immediate win remains documentation and contract clarification

4. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - avoid casual broadening of CPU/GPU staging work
   - Additional constraint informed:
     - current staging and fallback behavior remains performance-sensitive

## Delta vs First Pass

### 1. The env-driven artifact path is best-effort observability tooling, not part of the semantic execution contract

- Why this matters:
  - append failures log to stderr and do not fail the strategy path
- Practical implication:
  - preserve that reading unless artifact logging is intentionally upgraded

### 2. The CPU staging path is deliberately enforcing the GPU-v1 output contract across routes

- Why this matters:
  - the current validator name may look odd, but the behavior is product contract, not accidental reuse
- Practical implication:
  - preserve the defer bucket

## Verification If Changed

- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
- the same suite with `--test-threads=1`
- representative `commands_dynamic` and `strategy_gpu_runtime` coverage

## Regression And Feature-Preservation Constraints

- Keep lazy CPU fallback behavior for GPU-resolved plans
- Preserve current preview fallback semantics
- Keep artifact append best-effort unless the contract is intentionally upgraded
- Do not broaden dynamic output shapes without coordinated route-policy review

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly clarifies that env-driven artifact logging is observability tooling, while the current output-shape enforcement is a deliberate cross-route product contract.
