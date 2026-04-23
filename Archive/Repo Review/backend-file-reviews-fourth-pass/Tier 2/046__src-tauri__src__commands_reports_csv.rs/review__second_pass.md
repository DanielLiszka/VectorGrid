# Second-Pass Review: `src-tauri/src/commands_reports_csv.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_csv.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_csv.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this exporter as mostly sound but contract-sensitive because it owns a fixed header layout, shortlist semantics, and hand-rolled CSV shaping.

## Stale Check

The live source still has the same essential behavior:

- fixed 28-column header
- shortlist-first then all-rows fallback
- `params_json` emitted as JSON text in a single CSV field

No source drift weakened the first-pass packet.

## Second-Pass Research Log

1. RFC 4180
   - Source: <https://datatracker.ietf.org/doc/html/rfc4180>
   - Why consulted in second pass:
     - verify the baseline CSV contract and line/field expectations
   - Additional constraint informed:
     - the current emitter makes valid CSV, but newline and spreadsheet behavior are separate questions

2. Rust `csv::Writer`
   - Source: <https://docs.rs/csv/latest/csv/struct.Writer.html>
   - Why consulted in second pass:
     - compare the current manual emitter with a library-backed alternative
   - Additional constraint informed:
     - the current implementation is acceptable, but it is consciously choosing a custom contract surface

3. OWASP CSV Injection
   - Source: <https://owasp.org/www-community/attacks/CSV_Injection>
   - Why consulted in second pass:
     - separate CSV syntax correctness from spreadsheet formula-execution risk
   - Additional constraint informed:
     - spreadsheet safety would require an additional hardening choice if exported text becomes user-controlled or provider-derived

## Delta vs First Pass

## Refined Safety Finding

### 1. CSV syntax safety and spreadsheet safety are separate, and the current escaper only addresses the first one

- Evidence:
  - quoted CSV output does not by itself neutralize spreadsheet formula execution
- Why this matters:
  - if future exported text becomes more user-supplied or market-derived, spreadsheet-opening risk becomes relevant
- Practical implication:
  - any hardening here should be explicit and regression-tested because it would change visible cell contents

## Refined Output-Contract Finding

### 2. The file is deliberately a shortlist-oriented export surface, not a raw full-candidate dump

- Evidence:
  - the hard-coded `Some(4096)` model cap remains in place
- Why this matters:
  - treating this exporter as a general dump would misread the product contract
- Practical implication:
  - preserve shortlist-first behavior unless the consuming UX changes

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_csv -- --nocapture`

Add targeted coverage for:

- multiline-field records
- header order preservation
- spreadsheet-injection hardening, if added

## Regression And Feature-Preservation Constraints

- Keep the 28-column header order unless consumers move with it
- Preserve shortlist-first then all-rows fallback behavior
- Preserve `params_json` as JSON text in one CSV field

## Second-Pass Conclusion

`Bucket unchanged, but the export contract is more specific than "just CSV"`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens two points: the current file solves CSV syntax, not spreadsheet execution risk, and the exporter is intentionally shortlist-oriented rather than a full raw-candidate dump.
