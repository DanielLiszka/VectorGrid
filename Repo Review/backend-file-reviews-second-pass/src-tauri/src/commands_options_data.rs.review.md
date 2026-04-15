# Second-Pass Review: `src-tauri/src/commands_options_data.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_options_data.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_options_data.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a contract-heavy options dataset boundary and flagged bounded validation, provider coupling, and filesystem layout sensitivity as reasons to keep fixes small and precise.

## Stale Check

The live source still owns the same sensitive responsibilities:

- provider alias normalization and defaults
- dataset-id derivation and layout under `options_research/datasets/<dataset_id>`
- CSV import and provider download orchestration

The first-pass concerns remain current.

## Second-Pass Research Log

1. Reqwest blocking module
   - Source: <https://docs.rs/reqwest/latest/reqwest/blocking/index.html>
   - Why consulted in second pass:
     - verify the intended placement of the blocking client in this ingestion-oriented module
   - Additional constraint informed:
     - the current bounded blocking posture is still appropriate for provider download work

2. Reqwest blocking `ClientBuilder`
   - Source: <https://docs.rs/reqwest/latest/reqwest/blocking/struct.ClientBuilder.html>
   - Why consulted in second pass:
     - tighten timeout and user-agent expectations around provider traffic
   - Additional constraint informed:
     - explicit timeout posture remains part of the provider contract, not cleanup noise

3. RFC 9110 `Retry-After`
   - Source: <https://www.rfc-editor.org/rfc/rfc9110.html#field.retry-after>
   - Why consulted in second pass:
     - confirm whether the retry header can legitimately be an HTTP date instead of seconds
   - Additional constraint informed:
     - parsing only integer delay values is incomplete

4. Polygon options contracts reference
   - Source: <https://polygon.io/docs/rest/options/contracts/all-contracts>
   - Why consulted in second pass:
     - verify provider-side contract discovery assumptions
   - Additional constraint informed:
     - current alias normalization and contract-discovery defaults are real workflow policy

## Delta vs First Pass

## Stronger Retry Finding

### 1. `Retry-After` handling is narrower than the HTTP standard allows

- Evidence:
  - RFC 9110 permits either `delay-seconds` or `HTTP-date`
- Why this matters:
  - integer-only parsing can miss legitimate backoff instructions from upstream
- Practical implication:
  - if the retry helper is touched, supporting both standard forms is the correct bounded hardening step

## New Read-Only Boundary Finding

### 2. `list_options_datasets` is not truly read-only because the datasets-dir helper eagerly creates the store root

- Evidence:
  - the directory helper still uses `create_dir_all`
- Why this matters:
  - list-style commands should not mutate the store just because the user inspected it
- Practical implication:
  - this is a low-regression cleanup target, not a redesign reason

## First-Pass Direction Still Holds

### 3. The provider-specific defaults are product policy, not generic cleanup noise

- Evidence:
  - alias normalization to `massive`, minute aggregate defaults, and `aggregate_price_field = "close"` still shape the public workflow
- Practical implication:
  - preserve those defaults unless the frontend and stored manifests move together

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_options_data -- --nocapture`

Add targeted coverage for:

- `Retry-After` HTTP-date handling
- invalid dataset IDs
- early rejection of `strike_min > strike_max`
- read-only list behavior when the dataset root does not exist yet

## Regression And Feature-Preservation Constraints

- Preserve dataset layout under `options_research/datasets/<dataset_id>`
- Preserve manifest compatibility with `commands_options_expression.rs`
- Preserve provider aliases and current root-symbol normalization
- Do not change import/download output shape without frontend coordination

## Second-Pass Conclusion

`Bucket unchanged, but retry parsing and read-only semantics need sharper treatment`

The first-pass `Small surgical refactor` bucket remains correct. The second pass adds two clearer bounded fixes: `Retry-After` should handle both standard forms, and dataset listing should not create the store root just to answer a read-only query.
