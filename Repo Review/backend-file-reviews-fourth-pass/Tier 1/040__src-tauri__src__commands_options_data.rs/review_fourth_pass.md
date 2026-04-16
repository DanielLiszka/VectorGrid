# Fourth-Pass Review: src-tauri/src/commands_options_data.rs

- Review date: `2026-04-15`
- Source file: `src-tauri/src/commands_options_data.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_options_data.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_options_data.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_options_data.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file still deserves a fourth pass because the third-pass bucket is directionally correct, but the third-pass packet narrowed too aggressively around only two behaviors: read-only commands that mutate the datasets root, and orphaned dataset directories that can block retries. Those are real issues, and the third-pass packet was right to keep the file in a conservative bucket. But the live source still contains several smaller, sharper contract problems that are more immediate than a broad collision-policy rewrite.

The bucket should **stand**. This is still a `Small surgical refactor` file, not a redesign file. The reasons are unchanged: this module is a public Tauri IPC boundary, writes durable dataset state under the managed options store, and feeds downstream options-expression workflows through persisted manifests. That makes “small, explicit, test-backed” the right posture.

What changed on fourth pass is the packet, not the bucket. The third-pass packet correctly identified two real behaviors, but it missed a more important default naming collision on the Polygon download path, missed a provider-contract mismatch around rare `contract_type = "other"` responses, underweighted preflight ordering costs, and recommended one collision fix that is broader than the code safely justifies.

## Fourth-Pass Research Log

1. Re-read the live source first, especially `derive_polygon_download_dataset_id` (lines 334-346), `send_json_with_retry` (lines 505-559), the Polygon download flow (lines 781-977), and the read/list/load command surface (lines 1089-1145).
2. Re-read the current third-pass packet and prior packet history to see what was being carried forward versus what had been dropped.
3. Re-checked the downstream options-expression consumer. In local context, `parse_contract_reference_timestamp_ms` in `crates/app_core/src/options_expression.rs` only accepts RFC 3339 values and silently falls back to `None` on parse failure, which matters for how severe the `contract_reference_as_of_utc` inconsistency really is.
4. Re-checked official Tauri command guidance and Tokio blocking-task guidance. Tauri’s docs describe async commands as the preferred boundary for heavy work, and Tokio’s `spawn_blocking` docs explicitly note that blocking tasks run on a blocking pool and cannot be aborted once running.
5. Re-checked RFC 9110 `Retry-After` semantics. The standard still allows either `delay-seconds` or `HTTP-date`.
6. Re-checked the current Massive options contracts documentation. It still documents that `contract_type` can be `"put"`, `"call"`, or in rare cases `"other"`.

## What Fourth Pass Sharpened

### 1. The third-pass bucket should stay, and the packet was right about two things

The third-pass packet was correct that `list_options_datasets` and `load_options_dataset_manifest` are not actually read-only today because both reach `options_datasets_dir`, which unconditionally calls `create_dir_all` at lines 349-355 before scanning or loading at lines 1093-1119. That is still worth hardening.

It was also correct that the current directory-exists collision checks at lines 823-828 and 989-994 can turn a crash-leaked or interrupted partial dataset directory into a false “already exists” outcome.

Those findings remain valid.

### 2. The larger missed edge case is the default Polygon download dataset ID, not just orphaned directories

The current default Polygon download ID is derived only from `root_symbol`, `from`, and `to` at lines 334-346. But the request surface also includes `contract_type`, expiration filters, strike filters, `max_dte_days`, and `contract_reference_as_of_utc` at lines 147-171. Those fields can materially change which contracts are selected and therefore change the persisted dataset contents.

That means two legitimate download requests for the same root and date range can collide by default even when they are intentionally different datasets. A `call`-only slice and a `put`-only slice for the same root/from/to currently resolve to the same default dataset ID. The third-pass packet focused on orphaned directories, but the live code also has a deterministic naming collision for perfectly healthy, user-intended distinct downloads.

This is a stronger reason to revisit the download-side ID policy than the crash-orphan case alone.

### 3. `contract_type = Both` currently accepts provider values that the writer cannot serialize

`OptionContractTypeFilter::matches_contract` returns `true` for `Both` regardless of the raw provider value at lines 253-259. Later, `write_polygon_options_contract_rows` calls `normalized_row_value`, which only accepts `call` and `put` at lines 262-269 and 729. The Massive contracts docs also say the provider can return `contract_type = "other"` in rare cases.

So the current flow can do all of the following in one run:

- select an `"other"` contract under the default `Both` path,
- fetch its aggregate bars,
- and then fail late while writing the normalized CSV because the serializer does not support that contract type.

That is not a redesign issue. It is a contract mismatch inside this file. The safe-now fix is to reject or skip unsupported provider contract types during selection, before aggregate fetches start.

### 4. The preflight ordering still does too much work before proving the request is viable

The third-pass packet underweighted how much work happens before the command proves it can succeed.

Today the Polygon flow resolves the market root, downloads or reuses the underlying stock 1m dataset, opens and parses the underlying CSV, and only then reads the options API key and discovers/contracts-selects the option universe (lines 831-899). That means failures such as:

- missing current API credentials,
- zero contracts after contract discovery,
- and `max_contracts` overflow,

can all happen **after** the command has already paid cache and file-I/O costs for the underlying path.

This is a smaller, safer hardening target than provider parallelism or retry-policy redesign. The command should prove it has a viable options slice before it downloads or parses the supporting underlying series.

### 5. The retry problem is not only HTTP-date parsing; it is also unbounded wait inside `spawn_blocking`

The earlier packets were right that `send_json_with_retry` only parses integer `Retry-After` values at lines 535-540 even though RFC 9110 allows both `delay-seconds` and `HTTP-date`. But that is not the full problem.

The code also lets `Retry-After` determine `wait_ms` without a client-side upper bound at lines 545-547. Because this work is wrapped in `spawn_blocking` for the Tauri commands at lines 1081-1084 and 1138-1141, a very large `Retry-After` can park a blocking worker for an arbitrarily long time. Tokio’s own docs explicitly note that `spawn_blocking` tasks cannot be aborted once they are running.

So the safe-now retry fix is not “support HTTP-date” in isolation. It is “support both header forms **and** clamp the effective sleep to an explicit maximum that matches product expectations.”

### 6. One third-pass recommendation is too strong as written

The third-pass packet said to “treat collision detection as manifest-backed dataset exists rather than directory exists.” That diagnosis is understandable, but the recommendation is too broad for a safe-now change.

Right now, if a pre-existing final-path directory is treated as reusable merely because it lacks `manifest.json`, the code will happily enter that directory, create or truncate `rows.csv`, and on later failure may call `remove_dir_all` against that final path (download path at lines 901-975, import path at lines 1004-1065). Without a staging directory or an ownership marker, “no manifest” is not strong enough proof that the directory is safe to reuse or safe to delete.

So the collision issue is real, but the safe-now response is narrower:

- either return a more precise “incomplete/orphaned dataset directory exists” error,
- or introduce a staging/rename pattern before making final-path reuse automatic.

Blindly switching to manifest-only collision checks is too aggressive for this public persistence boundary.

### 7. The first-pass packet still had valid items that the third-pass packet effectively underweighted

Several earlier findings still look correct against the live source and should not disappear just because the third pass narrowed the story:

- explicit download dataset IDs still bypass `sanitize_dataset_id` and only use `sanitize_id_component` through `derive_polygon_download_dataset_id` (lines 334-346),
- `contract_reference_as_of_utc` is still parsed one way for live contract lookup (lines 852-860) and persisted another way through `format_contract_reference_as_of_utc` or direct import metadata (lines 399-407 and 1053),
- `strike_min > strike_max` is still not rejected before provider work starts,
- and import-side metadata validation still happens after the file copy begins at lines 1004-1015.

The nuance is severity. For example, the `contract_reference_as_of_utc` inconsistency is primarily a provenance and downstream diagnostics problem, not a GPU/runtime correctness issue, because the current options-expression consumer falls back to `None` on invalid RFC 3339 values rather than crashing.

## Implementation Priority

### Safe-now hardening

Keep the bucket as `Small surgical refactor`, but update the packet to prioritize these bounded changes:

1. **Fix the default Polygon download ID policy.** Either include the contract-selection-affecting filters in the derived ID, or require an explicit `options_dataset_id` when non-default filters or `contract_reference_as_of_utc` are supplied. The current root/from/to-only default is too weak for a persistence key.
2. **Reject or skip unsupported provider contract types before aggregate fetches.** Do not let `contract_type = Both` admit values that the writer later rejects. If the implementation chooses to skip rare unsupported contract types, keep the response accounting explicit so `contracts_selected` and `skipped_contracts` remain interpretable.
3. **Reorder Polygon preflight so options viability is checked before underlying data work.** Read the API key and discover/filter/limit-check contracts before downloading and parsing the underlying 1m stock series.
4. **Harden `Retry-After` handling in two dimensions.** Support both standard header forms and clamp the effective wait to an explicit maximum. Preserve the current retry-count posture.
5. **Keep the read-only helper split on the table.** `list_options_datasets` and `load_options_dataset_manifest` should not have to mutate disk merely to answer a read.
6. **Keep the earlier low-regression validations alive.** Explicit download dataset ID validation, `strike_min <= strike_max` preflight, one-time `contract_reference_as_of_utc` normalization, and import metadata validation before copy still look justified.

### Larger redesign still deferred

These are reasonable, but they are not safe-now defaults for this file:

1. **Staging directory plus atomic handoff.** This is the clean fix for crash-leaked final-path directories, but it is a larger persistence-flow change than the third-pass packet acknowledged.
2. **Manifest-scan best-effort behavior with per-dataset warnings.** Valuable, but it crosses into app-core coordination and user-facing diagnostics design.
3. **Provider client extraction for testability.** Helpful if this area keeps growing, but not required to land the safe-now fixes above.
4. **Any concurrency increase for per-contract aggregate fetches.** Still measurement-first and provider-quota-sensitive.

## Verification If Changed

At minimum:

- `cargo test -p ta_desktop_demo_app commands_options_data -- --nocapture`

Add or update focused tests for:

1. `derive_polygon_download_dataset_id` behavior so distinct filtered requests for the same root/from/to do not silently collide by default.
2. provider `contract_type = "other"` behavior under `Both`, proving the command fails early and clearly or skips explicitly before aggregate fetches.
3. `Retry-After` parsing for both integer and HTTP-date values, plus a cap test proving an extreme header does not create an unbounded sleep contract.
4. early rejection of `strike_min > strike_max`.
5. read-only list/load behavior when the datasets root does not yet exist.
6. the collision-path behavior chosen for orphaned directories, especially if any cleanup automation is added.

If the preflight ordering is changed, add a narrow helper-level test around the new preflight stage rather than trying to force a full live-provider integration test into this file.

## Regression And Feature-Preservation Constraints

- Preserve the on-disk store layout under `options_research/datasets/<dataset_id>` unless the change is explicitly staged as a migration.
- Preserve manifest schema compatibility with downstream options-expression consumers.
- Treat `options_dataset_id`, manifest metadata, and command DTOs as public IPC and persistence contracts.
- Do not silently auto-reuse or auto-delete pre-existing final-path dataset directories without an ownership or staging story.
- Preserve current provider alias normalization and the minute-aggregate `aggregate_price_field = "close"` contract.
- Do not casually parallelize the per-contract download loop.
- GPU ABI risk is effectively none here, but persistence and validation-diagnostics risk is real because this file decides what downstream workflows see as dataset identity and provenance.
- If unsupported provider contract types are skipped rather than rejected, keep response counters and user-visible semantics explicit so saved manifests and UI summaries do not become misleading.

## Fourth-Pass Conclusion

The current third-pass **bucket should stand**: `src-tauri/src/commands_options_data.rs` is still a `Small surgical refactor` file.

The third-pass packet got two important things right: the read-only command surface is not truly read-only, and orphaned final-path directories can create retry pain. But it also narrowed too much and proposed one fix too broadly.

The live source still has a more important default-ID collision on the Polygon download path, a real provider-contract mismatch around rare `contract_type = "other"` responses, avoidable preflight ordering costs, and a retry helper that is risky not just because it ignores HTTP-date `Retry-After`, but because it can sleep unboundedly inside non-abortable blocking work.

So the packet should be revised, not escalated. Keep the bucket. Tighten the safe-now changes. Defer staging/atomic handoff and broader provider/testability refactors until they can be landed as explicit persistence-flow work rather than folded into a nominally small cleanup.
