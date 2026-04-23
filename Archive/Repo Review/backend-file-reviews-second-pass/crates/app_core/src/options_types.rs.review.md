# Second-Pass Review: `crates/app_core/src/options_types.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_types.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_types.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as the canonical persisted schema for the options-expression workflow, with permissive defaults and stringly protocol fields as the main debt.

## Stale Check

The live source still owns:

- versioned manifest and artifact DTOs
- default policy helpers used during deserialization
- the shared serialized contract for import, replay, runtime, and reporting

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde container attributes
   - Source: <https://serde.rs/container-attrs>
   - Why consulted in second pass:
     - confirm unknown-field handling and the absence of `deny_unknown_fields`
   - Additional constraint informed:
     - these persisted structs remain intentionally tolerant of unknown extra JSON keys

2. Serde enum representations
   - Source: <https://serde.rs/enum-representations.html>
   - Why consulted in second pass:
     - compare closed-set enum handling with the still-stringly protocol fields
   - Additional constraint informed:
     - backend/artifact discriminators remain good future candidates for typed wire forms

## Delta vs First Pass

### 1. The schema is not only tolerant of missing compatibility fields; it is also tolerant of unexpected extra fields

- Why this matters:
  - this is a deliberate compatibility posture, not just a side effect of serde defaults
- Practical implication:
  - do not tighten deserialize strictness without checking stored artifacts and replay compatibility

### 2. The file already proves typed enum wire forms work well for true closed sets

- Practical implication:
  - future cleanup should target the remaining raw-string discriminators selectively rather than broad schema churn

## Verification If Changed

- `cargo test -p ta_app_core options_types::tests:: -- --nocapture`
- `cargo check -p ta_app_core --tests`
- rerun focused Tauri replay and provenance tests that deserialize these DTOs

## Regression And Feature-Preservation Constraints

- Preserve backward compatibility for existing V1 manifests, replay caches, and artifacts
- Preserve current omitted-field request defaults unless a migration exists
- Do not tighten deserialize strictness on persisted payloads casually

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the current schema is intentionally tolerant of both missing and extra fields, which makes any strictness increase a migration event rather than cleanup.
