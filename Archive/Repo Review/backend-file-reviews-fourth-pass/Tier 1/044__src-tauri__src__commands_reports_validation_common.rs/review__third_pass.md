# Third-Pass Review: `src-tauri/src/commands_reports_validation_common.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_validation_common.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_validation_common.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_validation_common.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

The second pass correctly kept this file in the defer bucket, but another pass was warranted because it shapes the exported workbook model directly and still hides a more concrete bug inside its broader compatibility logic.

## Third-Pass Research Log

1. `serde_json::Value`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
   - Why consulted in third pass:
     - re-check how partially present modern-schema fields can still trip compatibility behavior
   - Third-pass constraint informed:
     - missing nested fields are not the same thing as true legacy artifact identity

2. RFC 6901
   - Source: <https://datatracker.ietf.org/doc/html/rfc6901>
   - Why consulted in third pass:
     - confirm what JSON Pointer misses mean in the artifact-shaping layer
   - Third-pass constraint informed:
     - pointer-miss handling here is compatibility-sensitive workbook model policy

3. `rust_xlsxwriter` `Worksheet`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/struct.Worksheet.html>
   - Why consulted in third pass:
     - keep the export-shaping logic tied to visible workbook semantics
   - Third-pass constraint informed:
     - stale or contradictory labels in the model will surface directly in exported workbooks

## What Third Pass Sharpened

### 1. `compatibility_mode` is still broader than legacy detection, but the stronger problem is that malformed modern artifacts can silently downgrade into compatibility shaping

- Evidence:
  - the file still enables compatibility mode whenever candidate verdicts or run verdicts are incomplete
  - that decision does not require a true legacy artifact marker
- Why this matters:
  - partially modern or malformed artifacts can be exported as if they were intentional compatibility cases
- Practical implication:
  - this keeps the file in the larger deferred bucket because artifact-version semantics are still broader than a local cleanup patch

### 2. There is also a more bounded bug: run-level rejection rewrites outer candidate approval state without updating nested regime-bucket approval labels

- Evidence:
  - regime-bucket rows copy the candidate approval label during row construction
  - later run-level rejection can change `final_approved` and `approval_label` on the outer row
  - the nested `regime_bucket_rows` do not get resynchronized afterward
- Why this matters:
  - the exported workbook model can present contradictory approval semantics for the same candidate after run-level rejection
- Practical implication:
  - this is a real safe-now hardening target inside the larger deferred file

## Implementation Priority

- Highest-value bounded fix:
  - keep nested regime-bucket approval labels synchronized with post-run rejection rewriting
- Broader deferred work:
  - distinguish true legacy mode from malformed or partially modern artifact fallback

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_validation_common -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_reports_validation_xlsx -- --nocapture`
- frontend validation export tests

Add targeted coverage for:

- partially modern artifacts entering compatibility mode
- run-level rejection updating both candidate rows and nested regime-bucket approval labels
- shortlist ordering after post-run rejection rewrites

## Regression And Feature-Preservation Constraints

- Preserve permissive legacy-artifact export behavior unless artifact-version policy is intentionally redesigned
- Preserve shortlist order and run-level rejection semantics
- Do not silently narrow workbook compatibility handling while fixing nested label consistency

## Third-Pass Conclusion

`Bucket unchanged, but there is now one clearer safe-now bug`

The file still belongs in `Larger refactor but defer`, because compatibility mode remains broader than true legacy-artifact detection. But the third pass also surfaces a more concrete bug: post-run rejection can leave nested regime-bucket approval labels stale relative to the rewritten outer candidate rows.
