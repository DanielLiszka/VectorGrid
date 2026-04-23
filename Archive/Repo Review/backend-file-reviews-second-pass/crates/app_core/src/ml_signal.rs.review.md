# Second-Pass Review: `crates/app_core/src/ml_signal.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/ml_signal.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/ml_signal.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as feature-complete but contract-heavy because it owns dataset export, score-cache ingestion, validation/sweep behavior, and model-bundle manifest inspection.

## Stale Check

The live source still owns:

- dataset export plus sidecar manifest writes
- score-cache build and manifest persistence
- model-bundle listing and status classification

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `HashMap::insert`
   - Source: <https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.insert>
   - Why consulted in second pass:
     - verify the exact duplicate-timestamp replacement semantics
   - Additional constraint informed:
     - duplicate score rows remain last-write-wins because `insert` updates the old value when the key already exists

2. Rust `std::fs::write`
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why consulted in second pass:
     - ground the sidecar-write ordering review
   - Additional constraint informed:
     - each sidecar write is a full-file publish on its own; there is no grouped atomic export transaction

## Delta vs First Pass

### 1. Dataset export still is not all-or-nothing because the split manifest is published before the main CSV completes

- Evidence:
  - split-manifest write happens earlier in the export path, while the export manifest is written only after the dataset CSV finishes
- Why this matters:
  - a failing export can still leave behind a valid-looking sidecar artifact without the main CSV
- Practical implication:
  - this strengthens the first-pass concern about sidecar publication order

### 2. Duplicate timestamp handling is stronger than "duplicates are counted": the cached score becomes last-write-wins

- Evidence:
  - duplicate timestamps are counted, but insertion into the timestamp map still overwrites the old row
- Why this matters:
  - the manifest reports collision count, but not which row actually won
- Practical implication:
  - still a good bounded hardening target if score-cache import semantics are revisited

## Verification If Changed

- `cargo test -p ta_app_core ml_signal::tests:: -- --nocapture`
- re-run Tauri-side ML-signal bundle and workflow tests if bundle-listing or export semantics change

## Regression And Feature-Preservation Constraints

- Preserve current score-space and score-scope compatibility
- Preserve current validation and sweep semantics
- Do not change sidecar publication or duplicate-resolution behavior without explicit compatibility tests

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly tightens two operational semantics: dataset export still publishes sidecars non-atomically, and duplicate score timestamps still resolve as silent last-write-wins rather than a louder contract decision.
