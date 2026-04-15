# Second-Pass Review: `crates/app_core/src/strategy_gpu_program.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_gpu_program.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_gpu_program.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as ABI- and cache-contract-sensitive because layout, fingerprints, and `DeviceCopy` safety all matter to GPU execution.

## Stale Check

The live source still owns:

- serialized program/executable shapes
- ABI version and schema version constants
- `DeviceCopy`-bearing GPU data layouts

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust reference on type layout
   - Source: <https://doc.rust-lang.org/reference/type-layout.html>
   - Why consulted in second pass:
     - verify `#[repr(C)]` layout guarantees for the CUDA-facing structs
   - Additional constraint informed:
     - enum encodings and field order remain part of the live ABI contract

2. `cust::memory::DeviceCopy`
   - Source: <https://docs.rs/cust/latest/cust/memory/trait.DeviceCopy.html>
   - Why consulted in second pass:
     - re-check POD-only safety expectations
   - Additional constraint informed:
     - the current `unsafe impl DeviceCopy` blocks still deserve adjacent proof comments

3. `DefaultHasher`
   - Source: <https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html>
   - Why consulted in second pass:
     - verify fingerprint stability expectations
   - Additional constraint informed:
     - the exposed fingerprint hex values are not stable across releases and should be treated as runtime-local/cache-local identifiers

## Delta vs First Pass

### 1. Fingerprints are more contract-sensitive than the first pass emphasized because they are serialized and exposed, but still come from `DefaultHasher`

- Why this matters:
  - callers may over-read them as stable identifiers
- Practical implication:
  - keep them as local cache identity unless a stable fingerprint contract is intentionally introduced

### 2. This file needs a clearer local "when to bump schema vs ABI" rule

- Practical implication:
  - version constants remain the main mutation point for both compatibility layers

## Verification If Changed

- `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
- a representative GPU predicate runtime path

## Regression And Feature-Preservation Constraints

- Preserve enum encodings and ABI field order
- Preserve current fingerprint composition unless all consumers are reviewed together
- Do not change schema or ABI versioning casually

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the exposed fingerprints are runtime-local identities, not stable cross-release fingerprints, and that schema-vs-ABI versioning rules should stay explicit here.
