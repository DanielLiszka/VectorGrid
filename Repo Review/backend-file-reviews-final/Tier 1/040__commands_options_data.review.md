# Final Consolidated Review: `src-tauri/src/commands_options_data.rs`

- Review date: `2026-04-16`
- Source file: [commands_options_data.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs:1>)
- First-pass packet: [commands_options_data.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_options_data.rs.review.md:1>)
- Second-pass packet: [commands_options_data.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_options_data.rs.review.md:1>)
- Third-pass packet: [commands_options_data.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_options_data.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/040__src-tauri__src__commands_options_data.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
  - <https://www.rfc-editor.org/rfc/rfc9110#field.retry-after>
  - <https://doc.rust-lang.org/std/fs/fn.create_dir_all.html>

## Consolidated Judgment

The third-pass bucket should stand. This file is still a public persistence and IPC boundary whose remaining issues are real but still local enough to address surgically. The third pass was right that read-only dataset commands should not mutate the store root and that orphaned dataset directories can create retry pain. The fourth pass is stronger because it shows that the sharper immediate issues are elsewhere: the default Polygon download dataset ID can collide across materially different filtered requests, `contract_type = Both` can still admit provider values the writer rejects late, and `Retry-After` handling is both under-parsed and unbounded inside `spawn_blocking`.

The fourth pass also correctly narrowed one earlier recommendation. Treating "manifest exists" as the only collision authority is too aggressive without a staging or ownership story. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Strengthen the default Polygon download dataset-ID policy so distinct filtered requests do not silently collide under the same root/from/to-only default. Requiring explicit IDs for materially filtered requests is acceptable if needed.
- Reject or explicitly skip unsupported provider `contract_type` values before aggregate fetches begin.
- Reorder Polygon preflight so API key and options-slice viability are proven before underlying stock-series work starts.
- Support both standard `Retry-After` forms and clamp the effective sleep to an explicit maximum.
- Keep the read-only helper split in scope so list/load commands do not create the datasets root just to answer read requests.
- Preserve the earlier low-regression validations:
  - explicit dataset-ID validation
  - `strike_min <= strike_max` preflight
  - one-time `contract_reference_as_of_utc` normalization
  - import metadata validation before copy

## Deferred Work

- Defer a staging-directory plus atomic-handoff redesign.
- Defer manifest-scan best-effort warning policy.
- Defer provider-client extraction for testability.
- Defer any concurrency increase for per-contract download fan-out.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_options_data -- --nocapture`
- Add targeted tests for:
  - default dataset-ID differentiation across filtered requests
  - provider `contract_type = "other"` handling under `Both`
  - integer and HTTP-date `Retry-After` parsing plus clamp behavior
  - early `strike_min > strike_max` rejection
  - read-only list/load behavior when the datasets root is absent
  - the chosen orphan-directory collision behavior

## Regression And Feature-Preservation Constraints

- Preserve the on-disk store layout under `options_research/datasets/<dataset_id>` unless a real migration is in scope.
- Preserve manifest compatibility for downstream options-expression consumers.
- Treat dataset IDs, manifest metadata, and command DTOs as public persistence contracts.
- Do not silently auto-reuse or auto-delete pre-existing final-path dataset directories without an ownership or staging model.
- Preserve current provider alias normalization and `aggregate_price_field = "close"`.
- Do not casually parallelize the per-contract download loop.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `commands_options_data.rs` in `Small surgical refactor`
- prioritize default-ID collision hardening, unsupported-contract filtering, preflight ordering, and bounded `Retry-After` handling
- keep read-only-helper cleanup and earlier low-regression validations in the safe-now set
- defer staging/atomic handoff and broader provider/testability redesign
