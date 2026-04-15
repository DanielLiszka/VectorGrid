# Second-Pass Review: `crates/app_core/src/csv_compat.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/csv_compat.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/csv_compat.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this loader as a compatibility boundary rather than a plain parser wrapper, because header and ragged-row handling are intentionally repo-defined.

## Stale Check

The live source still uses:

- `has_headers(false)`
- `flexible(true)`
- first-non-empty-row probing and repo-local normalization

The first-pass packet remains current.

## Second-Pass Research Log

1. `csv::ReaderBuilder`
   - Source: <https://docs.rs/csv/latest/csv/struct.ReaderBuilder.html>
   - Why consulted in second pass:
     - verify header and ragged-row behavior in the underlying crate
   - Additional constraint informed:
     - the current settings intentionally bypass default csv crate semantics

2. `csv::Reader`
   - Source: <https://docs.rs/csv/latest/csv/struct.Reader.html>
   - Why consulted in second pass:
     - ground how normal header handling and ragged-row errors would behave otherwise
   - Additional constraint informed:
     - repo-local compatibility logic is the real contract owner here

## Delta vs First Pass

### 1. Headerless positional ingestion is deliberate API behavior, not incidental parser configuration

- Why this matters:
  - changing `has_headers(false)` would change accepted legacy shapes
- Practical implication:
  - keep this surface under compatibility discipline

### 2. Flexible ragged-row acceptance is also deliberate contract, not parser leniency by accident

- Practical implication:
  - if compatibility is narrowed, do it explicitly with coverage for legacy uneven-width shapes

## Verification If Changed

- add tests for headerless positional CSV
- add tests for uneven-width rows
- preserve legacy-shape normalization coverage

## Regression And Feature-Preservation Constraints

- Preserve headerless positional ingestion
- Preserve flexible acceptance of legacy uneven-width CSV shapes unless deliberately narrowed
- Keep first-non-empty-row probing behavior stable unless consumers move together

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that both headerless parsing and ragged-row acceptance are active compatibility policy owned by this file.
