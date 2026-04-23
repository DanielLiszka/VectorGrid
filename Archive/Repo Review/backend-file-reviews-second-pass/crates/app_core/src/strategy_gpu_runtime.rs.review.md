# Second-Pass Review: `crates/app_core/src/strategy_gpu_runtime.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_gpu_runtime.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_gpu_runtime.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as stateful runtime infrastructure where cache scope and upload ownership need clearer documentation more than structural rewrites.

## Stale Check

The live source still owns:

- fingerprint-based executable reuse
- per-thread upload caches
- reset helpers for the GPU strategy executable cache

The first-pass packet remains current.

## Second-Pass Research Log

1. `OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - re-check process-lifetime lazy cache semantics
   - Additional constraint informed:
     - unbounded cache lifetime remains a real contract, not implementation detail

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in second pass:
     - ground current-thread upload-cache scope
   - Additional constraint informed:
     - reset helpers and upload ownership are narrower in scope than their names can imply

3. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess stateful side-effect documentation needs
   - Additional constraint informed:
     - `get_or_build_gpu_strategy_executable()` still has hidden upload side effects even on some cache hits

## Delta vs First Pass

### 1. `reset_gpu_strategy_executable_cache()` still overstates its reach because it only clears the current thread’s upload cache

- Why this matters:
  - callers can easily assume process-global reset when the scope is narrower

### 2. `get_or_build_gpu_strategy_executable()` is more than a metadata lookup

- Why this matters:
  - cache hits can still trigger per-thread upload side effects
- Practical implication:
  - documentation and naming clarity remain the best bounded fixes

## Verification If Changed

- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
- one dynamic GPU execution path that reuses uploads on the same thread

## Regression And Feature-Preservation Constraints

- Preserve fingerprint-based reuse
- Preserve current per-thread upload ownership
- Do not narrow or widen cache/reset scope without explicit tests and docs

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that cache reset scope is narrower than the helper name suggests and that executable lookup still has stateful upload side effects.
