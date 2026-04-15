# Second-Pass Review: `crates/app_core/src/indicator_catalog.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/indicator_catalog.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/indicator_catalog.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a DTO and registry projection boundary where snapshot semantics and field meaning matter more than algorithmic complexity.

## Stale Check

The live source still uses:

- `OnceLock` catalog caching
- duplicate-schema-key guarding
- `step` and related UI-oriented descriptor metadata

The first-pass packet remains current.

## Second-Pass Research Log

1. `OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why consulted in second pass:
     - verify immutable snapshot semantics
   - Additional constraint informed:
     - the catalog is still a process-lifetime snapshot after first access

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - ground the public DTO contract review
   - Additional constraint informed:
     - descriptor field semantics remain serialization contract, not only UI hints

3. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess the clarity expectations for public metadata fields
   - Additional constraint informed:
     - `step` still needs sharper documentation because it behaves more like UI metadata than validation data

## Delta vs First Pass

### 1. The catalog is a deliberate process-lifetime snapshot and should stay documented that way

- Why this matters:
  - runtime registry or policy mutation would not be reflected after first access
- Practical implication:
  - preserve the snapshot reading unless hot-reload behavior is intentionally added

### 2. `step` remains the sharpest contract ambiguity

- Practical implication:
  - treat it as documentation debt rather than structural registry risk

## Verification If Changed

- `cargo test -p ta_app_core indicator_catalog -- --nocapture`
- dynamic-plan routing tests
- desktop `list_dynamic_indicators` test

## Regression And Feature-Preservation Constraints

- Preserve current DTO field names
- Preserve case-insensitive lookup behavior
- Preserve process-lifetime snapshot semantics unless intentionally redesigned

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the catalog is intentionally a process-lifetime snapshot and that `step` remains the most ambiguous field contract.
