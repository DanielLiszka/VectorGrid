# Second-Pass Review: `src-tauri/src/validation_common.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/validation_common.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/validation_common.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this helper file as semantics-sensitive because candidate identity, ranking order, and path normalization all feed shared validation behavior.

## Stale Check

The live source still owns:

- candidate-key canonicalization
- robust ranking helpers
- path-key normalization

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `Path`
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html>
   - Why consulted in second pass:
     - revisit whether current path-key shaping should be read as true path identity
   - Additional constraint informed:
     - `normalize_path_key()` remains a Windows-compat text transform, not a cross-platform identity rule

2. `serde_json::Value`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
   - Why consulted in second pass:
     - verify the semantics of candidate-key canonical shaping
   - Additional constraint informed:
     - object-key order is normalized, array order intentionally is not

3. `f64::total_cmp`
   - Source: <https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp>
   - Why consulted in second pass:
     - confirm deterministic float ordering in ranking
   - Additional constraint informed:
     - current final ordering is deliberately stable given the present score semantics

## Delta vs First Pass

### 1. Array order in `candidate_key_canonical()` remains part of candidate identity by design

- Why this matters:
  - "sort arrays for convenience" would change key identity and downstream grouping
- Practical implication:
  - preserve that asymmetry unless candidate identity policy intentionally changes

### 2. Nearest-rank median semantics matter more than they first appeared because ranking uses these stats in final ordering

- Why this matters:
  - even small statistical changes can alter deterministic candidate order
- Practical implication:
  - treat any median or ranking-stat cleanup as semantics-sensitive

## Verification If Changed

- `cargo test -p ta_desktop_demo_app validation_common -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
- ML-signal and campaign command suites if path normalization changes

## Regression And Feature-Preservation Constraints

- Preserve stable candidate-key generation
- Preserve deterministic ranking unless semantics intentionally change
- Do not overstate `normalize_path_key()` as true path identity

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that candidate-key array order and nearest-rank median behavior are both active semantics, not incidental helper details.
