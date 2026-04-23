# Second-Pass Review: `src-tauri/src/commands_reports_validation_common.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_validation_common.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_validation_common.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a broad, compatibility-heavy translation layer for validation artifacts and workbook models, and deferred larger cleanup because permissive legacy handling is part of the product surface.

## Stale Check

The live source still owns:

- legacy and modern artifact normalization
- shortlist/report model shaping
- enrichment and compatibility-mode behavior that directly feeds workbook export

The first-pass packet remains current.

## Second-Pass Research Log

1. `serde_json::Value`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
   - Why consulted in second pass:
     - verify missing-path and pointer behavior in artifact traversal
   - Additional constraint informed:
     - partial modern-schema presence can still trigger compatibility handling in ways that are not purely "legacy artifact" cases

2. RFC 6901
   - Source: <https://datatracker.ietf.org/doc/html/rfc6901>
   - Why consulted in second pass:
     - ground the JSON Pointer behavior behind artifact-path lookups
   - Additional constraint informed:
     - path-miss behavior should be treated as compatibility-sensitive model shaping

3. `rust_xlsxwriter` `Worksheet`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/struct.Worksheet.html>
   - Why consulted in second pass:
     - confirm how directly this model layer feeds visible workbook semantics
   - Additional constraint informed:
     - label and compatibility drift here will surface directly in exports

## Delta vs First Pass

### 1. `compatibility_mode` is broader than true legacy-artifact detection

- Evidence:
  - partial modern-schema presence can still fall into compatibility behavior
- Why this matters:
  - modern-but-incomplete artifacts may be shaped like legacy inputs even when they are not truly old-format
- Practical implication:
  - this strengthens the case for deferring broad cleanup until artifact-version semantics are clarified

### 2. `normalize_regime_support_label()` still trims but does not canonicalize

- Evidence:
  - label drift can still survive through backend, workbook, and frontend surfaces
- Practical implication:
  - this remains a real consistency gap, but still not one that justifies risky broad restructuring by itself

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_validation_common -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_reports_validation_xlsx -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_reports_csv -- --nocapture`
- frontend validation export tests

## Regression And Feature-Preservation Constraints

- Preserve permissive legacy-artifact export
- Preserve shortlist order and approval semantics
- Preserve current compatibility behavior unless artifact-version policy is intentionally redesigned

## Second-Pass Conclusion

`Bucket unchanged, but compatibility-mode scope is broader than the first pass emphasized`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly sharpens that compatibility behavior is not limited to clearly legacy artifacts, which makes casual cleanup riskier rather than safer.
