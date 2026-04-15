# Second-Pass Review: `crates/app_core/src/ma_policy.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/ma_policy.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/ma_policy.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as small but policy-bearing because its generated speedup and hybrid-capability tables directly influence route decisions.

## Stale Check

The live source still:

- embeds CSV-derived policy data
- classifies hybrid capability dynamically
- treats absent speedup entries as CPU-preferred

The first-pass packet remains current.

## Second-Pass Research Log

1. `OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - assess a low-risk caching opportunity
   - Additional constraint informed:
     - `classify_hybrid_capability` still looks like a good `OnceLock` candidate without semantic risk

2. `include_str!`
   - Source: <https://doc.rust-lang.org/std/macro.include_str.html>
   - Why consulted in second pass:
     - ground the embedded-CSV provenance and parsing review
   - Additional constraint informed:
     - embedded policy data remains a stable build-time contract

3. RFC 4180
   - Source: <https://www.rfc-editor.org/rfc/rfc4180>
   - Why consulted in second pass:
     - verify embedded CSV parsing expectations
   - Additional constraint informed:
     - deterministic parsing and ordering remain important here

4. `BTreeSet`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BTreeSet.html>
   - Why consulted in second pass:
     - confirm deterministic ordering guarantees
   - Additional constraint informed:
     - current deterministic ordering remains a must-preserve behavior

## Delta vs First Pass

### 1. `prefers_cpu_by_speedup_table` still overstates what it does

- Why this matters:
  - "not present in CSV" still means CPU-preferred, and that behavior is test-locked
- Practical implication:
  - the main improvement remains naming and documentation, not logic churn

### 2. The embedded comment still cites an outdated `vector-ta` version

- Practical implication:
  - provenance clarity remains a good small cleanup target if the file is touched

## Verification If Changed

- `cargo test -p ta_app_core ma_policy::tests:: -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve deterministic capability ordering
- Do not silently change current route decisions for known MA IDs
- Preserve embedded policy-table behavior unless regenerated intentionally

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the current naming still overstates what the speedup table means, while the real behavior remains a deterministic product-policy contract.
