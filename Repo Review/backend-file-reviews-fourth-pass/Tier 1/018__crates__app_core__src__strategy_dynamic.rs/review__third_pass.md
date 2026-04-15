# Third-Pass Review: `crates/app_core/src/strategy_dynamic.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_dynamic.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_dynamic.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_dynamic.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a runtime/planner/cache surface whose public meaning is narrower than its internal policy still looked like the right framing under fresh research.

## Third-Pass Research Log

1. Rust DefaultHasher docs
   - Source: <https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html>
   - Why consulted in third pass:
     - recheck whether hash-derived fingerprints in this area should be treated as stable identity
   - Third-pass constraint informed:
     - hashes built on DefaultHasher should be treated as local heuristics, not durable public IDs

2. Rust OnceLock docs
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in third pass:
     - reassess how process-lifetime caches and sticky initialization should be described
   - Third-pass constraint informed:
     - process-lifetime caches need honest naming and diagnostics because they can outlive transient conditions

3. Rust thread_local docs
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in third pass:
     - ground the scope discussion around thread-local caches and diagnostics
   - Third-pass constraint informed:
     - thread-local state should not be reported as if it were process-global runtime truth

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; the stronger read is cache/runtime policy versus stable public contract
- Why this matters:
  - The second-pass call of `Bucket unchanged` still fits after reopening the file.
  - These files still look more sensitive in naming, diagnostics, and cache semantics than in structure.
- Practical implication:
  - Keep the current bucket and avoid structural rewrites unless telemetry, cache consumers, and downstream expectations are being revalidated together.

### 2. Hash- and cache-derived identities remain local heuristics
- Why this matters:
  - This area still mixes internal caching policy with surfaces that can look more durable than they really are.
- Practical implication:
  - Document or test the heuristic nature of these identities before expanding their use.

## Implementation Priority

- Prefer honest naming, clearer diagnostics, and targeted cache-behavior tests over wider refactor.
- Keep cache identity and runtime policy local unless a wider compatibility plan exists.

## Verification If Changed

- cargo test -p ta_app_core strategy_dynamic -- --nocapture
- representative cache/reset/telemetry tests for the owning runtime path

## Regression And Feature-Preservation Constraints

- Do not treat hash-derived fingerprints as stable cross-process public identity.
- Keep cache scope and diagnostic truthfulness aligned.
- Avoid planner/runtime rewrites without retesting downstream telemetry and consumer assumptions.

## Third-Pass Conclusion

`Bucket unchanged`

strategy_dynamic.rs stays in `Larger refactor but defer`. The third pass mainly sharpens the distinction between internal runtime policy and stable public contract.
