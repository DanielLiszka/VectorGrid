# Third-Pass Review: `crates/app_core/src/options_data_store.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_data_store.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_data_store.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_data_store.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the imported options dataset normalization owner still looked like the right framing under fresh research. The third pass sharpened the packet around three concrete data-shape behaviors: normalized duplicate headers overwrite earlier columns, explicit root override can relabel mixed-root input, and duplicate timestamps are silently collapsed per contract.

## Third-Pass Research Log

1. CSV ReaderBuilder docs
   - Source: <https://docs.rs/csv/latest/csv/struct.ReaderBuilder.html>
   - Why consulted in third pass:
     - ground the header and row parsing discussion in the actual CSV tooling layer
   - Third-pass constraint informed:
     - header interpretation and tolerant row parsing should remain explicit because they shape imported dataset truth

2. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether the sharper issue is documentation-only or contract-level
   - Third-pass constraint informed:
     - data-import helpers should not silently rewrite semantically meaningful input without tests and clear policy

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in third pass:
     - reconfirm how manifest and data-shape defaults interact with imported datasets
   - Third-pass constraint informed:
     - defaulting or overwriting imported fields is part of dataset provenance once manifests are emitted

## What Third Pass Sharpened

### 1. Normalized duplicate headers silently let later columns win
- Evidence:
  - header_index inserts normalized header names into a map and later entries overwrite earlier ones.
- Why this matters:
  - That can silently reinterpret an input file with conflicting normalized headers instead of rejecting it.
- Practical implication:
  - Keep the existing bucket, but treat duplicate normalized headers as provenance-sensitive input policy rather than benign tolerance.

### 2. Explicit root override can relabel mixed-root input before consistency checks
- Evidence:
  - parse_row replaces each row root with the caller-supplied root symbol before the dataset consistency check runs.
- Why this matters:
  - Mixed-root input can therefore be silently coerced into one root label rather than rejected at raw-row time.
- Practical implication:
  - If hardened, make root-override policy explicit and test whether mixed-root input should fail fast.

### 3. Per-contract duplicate timestamps are silently collapsed after sorting
- Evidence:
  - Rows are sorted and deduplicated by timestamp within each contract series.
- Why this matters:
  - That is a real data-loss policy, not just storage cleanup.
- Practical implication:
  - Keep the current bucket, but document and fixture this collapse behavior if it remains intentional.

## Implementation Priority

- First hardening step: make duplicate-header and root-override policy explicit in tests and docs.
- Second hardening step: decide whether duplicate timestamp collapse should stay silent or become a surfaced warning or error.

## Verification If Changed

- cargo test -p ta_app_core options_data_store -- --nocapture
- representative import fixtures with duplicate normalized headers, mixed-root rows, and duplicate timestamps

## Regression And Feature-Preservation Constraints

- Preserve current manifest schema and contract-series output shape unless consumers move together.
- Do not silently change import tolerance rules without explicit fixture coverage.
- Keep provenance-sensitive normalization behavior documented if it remains permissive.

## Third-Pass Conclusion

`Bucket unchanged, but provenance loss sharpened`

options_data_store.rs stays in Small surgical refactor. The third pass mainly sharpens the provenance risks: normalized duplicate headers silently overwrite earlier columns, explicit root override can relabel mixed-root input, and per-contract duplicate timestamps are still collapsed without surfacing the loss.
