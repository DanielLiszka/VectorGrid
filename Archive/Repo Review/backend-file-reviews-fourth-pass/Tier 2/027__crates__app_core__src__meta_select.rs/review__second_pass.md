# Second-Pass Review: `crates/app_core/src/meta_select.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/meta_select.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/meta_select.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as deterministic and benchmark-sensitive selection logic whose contract is partly split with higher-level orchestration.

## Stale Check

The live source still owns:

- artifact grouping and canonicalization
- selection and holdout-related request/result shaping
- progress emission through Double-MA-named helpers

The first-pass packet remains current.

## Second-Pass Research Log

1. RFC 8785
   - Source: <https://www.rfc-editor.org/rfc/rfc8785>
   - Why consulted in second pass:
     - ground canonical-JSON expectations
   - Additional constraint informed:
     - deterministic canonicalization remains a key behavior to preserve

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - reassess request/result shaping semantics
   - Additional constraint informed:
     - some holdout-related contract remains split across layers

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground request-surface review
   - Additional constraint informed:
     - artifact-match filtering still ignores `holdout_tag`, keeping part of the contract outside this file

## Delta vs First Pass

### 1. Holdout knobs remain only partially consumed here

- Why this matters:
  - `artifact_matches_request` still ignores `holdout_tag`
- Practical implication:
  - the split contract with orchestration is still a real bounded design debt

### 2. Progress emission still leaks Double-MA naming into a meta-selection surface

- Practical implication:
  - naming cleanup remains worthwhile, but still not enough to justify risky structural change

## Verification If Changed

- `cargo test -p ta_app_core meta_select::tests:: -- --nocapture`
- `cargo bench -p ta_app_core --bench meta_select_workflow --no-run`

## Regression And Feature-Preservation Constraints

- Preserve deterministic canonicalization and grouping
- Preserve backward-tolerant artifact loading
- Preserve the current train-then-holdout split contract with Tauri orchestration

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the holdout contract is still split across layers and that progress naming still leaks Double-MA ownership into meta selection.
