# Second-Pass Review: `crates/app_core/src/options_data_store.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_data_store.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_data_store.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a persisted dataset-contract owner where CSV normalization, manifest semantics, and grouped series construction all matter more than raw compute cost.

## Stale Check

The live source still owns:

- header normalization and lookup
- normalized CSV ingest
- manifest read/write/scan behavior

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `BTreeMap`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BTreeMap.html>
   - Why consulted in second pass:
     - verify duplicate-key replacement semantics and key-order behavior in header normalization
   - Additional constraint informed:
     - duplicate normalized headers still silently overwrite earlier entries when inserted into the map

2. `csv::ReaderBuilder`
   - Source: <https://docs.rs/csv/latest/csv/struct.ReaderBuilder.html>
   - Why consulted in second pass:
     - re-check what the parser itself treats as header behavior and uneven-record handling
   - Additional constraint informed:
     - current header shadowing is repo-local normalization behavior, not something the CSV crate is deciding for us

## Delta vs First Pass

### 1. Duplicate normalized headers still shadow earlier columns silently

- Evidence:
  - header lookup uses a map keyed by normalized header name, and later inserts replace earlier values
- Why this matters:
  - the selected column can change silently if two headers normalize to the same key
- Practical implication:
  - this remains a good bounded hardening target because it affects import correctness before any row parsing begins

### 2. An explicit `root_symbol` override suppresses mixed-root detection rather than merely validating against it

- Evidence:
  - each parsed row is normalized through the override before root-symbol consistency is checked
- Why this matters:
  - a mixed-root CSV can be made to look consistent under an explicit override
- Practical implication:
  - this is a stronger contract issue than the first pass made explicit

## Verification If Changed

- `cargo test -p ta_app_core options_data_store::tests:: -- --nocapture`
- re-run Tauri-side options dataset import/load and options-expression replay tests if manifest or root-symbol semantics change

## Regression And Feature-Preservation Constraints

- Preserve current normalized CSV import contract unless a migration story exists
- Preserve manifest compatibility with the options-expression workflow
- Do not tighten duplicate-header or root-symbol behavior without explicit regression tests

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens two import-contract edges: duplicate normalized headers still shadow earlier columns silently, and explicit `root_symbol` override still suppresses mixed-root detection instead of checking it.
