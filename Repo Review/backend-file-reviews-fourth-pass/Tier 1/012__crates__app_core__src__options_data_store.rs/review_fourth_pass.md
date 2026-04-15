# Fourth-Pass Review: `crates/app_core/src/options_data_store.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/options_data_store.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass packet is directionally right, but it is still too centered on provenance language around duplicate headers, root overrides, and duplicate timestamps. The live file is more important than that framing suggests. It owns the public CSV-import request DTO, the normalized import path, and the manifest read/write/scan helpers, and those helpers are re-exported from `app_core` as public API (`options_data_store.rs:11-29, 291-489`; `lib.rs:49, 215-217`).

That matters because downstream options-expression code trusts the imported dataset contract rather than re-validating it. It filters contract series by `root_symbol`, rejects datasets whose manifest root does not match the signal manifest root, and surfaces `row_count`, `contract_count`, `coverage_start`, and `coverage_end` in diagnostics (`options_expression.rs:437-447, 2508-2517, 2743-2746`). It also relies on per-contract `expiration_timestamp_ms`, `strike`, and `multiplier` during candidate selection and pricing (`options_expression.rs:966-1005, 1030-1088`).

So the right fourth-pass question was not “are the third-pass findings still real?” They are. The better question was “are those the highest-value safe-now fixes?” The answer is only partly yes. The current third-pass bucket should stand, but the next pass should prioritize contract consistency and persistence hygiene ahead of broader provenance commentary.

## Fourth-Pass Research Log

I re-read the live source file first, then the current third-pass packet, then used the earlier packets only to understand what had already been claimed. I also checked the local type and consumer context in `options_types.rs` and `options_expression.rs` to verify which manifest and contract-series fields downstream code actually trusts.

I then cross-checked the technical claims against primary sources:

- Rust std docs for `BTreeMap::insert`, to verify that duplicate normalized headers really do overwrite earlier entries.
- Rust std docs for `slice::sort_by` and `Vec::dedup_by`, to verify whether the duplicate-timestamp winner is undefined or deterministic.
- Rust std docs for `PathBuf::join`, to verify the consequences of joining unsanitized dataset IDs into a path.
- Rust std docs for `std::fs::write`, to confirm the persistence semantics of the current manifest write helper.
- `csv` crate docs for `ReaderBuilder::flexible`, to verify exactly what tolerance the importer enables.
- `chrono` docs for `DateTime::from_timestamp_millis`, to confirm the date-derivation contract for expiration timestamps.
- Rust API Guidelines material on validation and public documentation, to check whether the right standard here is permissive import tolerance or explicit contract validation.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was correct to keep this file in `Small surgical refactor`. Nothing in the live source suggests a hot-path, GPU, or ABI-heavy redesign. It was also correct that three behaviors are still real and still worth naming explicitly:

- duplicate normalized headers still silently let the last matching column win (`options_data_store.rs:74-106`);
- an explicit `root_symbol` override still relabels rows before any consistency check runs (`options_data_store.rs:173-191, 313-343`);
- per-contract duplicate timestamps are still collapsed after sort (`options_data_store.rs:395-402`).

The third-pass packet was also right not to turn this into a performance review. The main risk here is persisted data-contract drift, not runtime throughput.

### What the current third-pass packet missed or underweighted

The biggest miss is that the importer groups by `contract_id` but never validates that later rows for the same contract ID still agree on the rest of the contract metadata. The first row establishes `root_symbol`, `option_type`, `expiration_date`, `expiration_timestamp_ms`, `strike`, and `multiplier`; later rows only append `OptionsPriceRow` data (`options_data_store.rs:354-366`). If the CSV contains conflicting metadata under one `contract_id`, the importer silently keeps the first series metadata and discards the later disagreement. That is not a cosmetic provenance problem. Downstream options-expression logic actively uses `expiration_timestamp_ms`, `strike`, and `multiplier` during candidate selection and pricing (`options_expression.rs:966-1005, 1030-1088`). This is the highest-value safe-now hardening item, and the third-pass packet did not call it out.

The second miss is manifest version enforcement. `write_options_dataset_manifest` always emits `OPTIONS_DATASET_MANIFEST_VERSION`, but `read_options_dataset_manifest` only deserializes JSON into `OptionsDatasetManifestV1` and never checks whether `manifest.version` is actually supported (`options_data_store.rs:405-427, 435-447`; `options_types.rs:3, 314-347`). That is more important than the current packet suggests because `OptionsDatasetManifestV1` uses multiple serde defaults on deserialize (`options_types.rs:317-347`). In other words, the read path is more permissive than the type name “V1” implies. The packet cited Serde defaults generally, but it never landed the concrete problem: versioned manifest I/O without a version gate is only half a versioning story.

The third miss is path and identity hygiene. The loader opens `req.dataset_path.trim()` but persists `req.dataset_path.clone()` into the manifest (`options_data_store.rs:294-299, 423-425`). That means a request with leading or trailing whitespace can load successfully but write back a path string that does not match the path that was actually opened. In the same file, `dataset_manifest_path` joins `dataset_id` verbatim (`options_data_store.rs:488-489`). If callers ever pass an empty, absolute, or separator-bearing dataset ID, the helper does not defend itself. I would not overstate this as an already-proven exploit because the call sites were not in this bundle, but as a public helper and public request type this is still a contract weakness that the current packet did not surface.

The fourth miss is the interaction between duplicate-timestamp collapse and quote-semantics validation. The loader counts `rows_with_quote_fields` before any per-contract dedup runs (`options_data_store.rs:322-333`), but duplicate timestamps are dropped later (`options_data_store.rs:395-402`). That means a `top_of_book_quote` dataset can satisfy the “at least one populated quote-side row” check only because of rows that are later discarded. The third-pass packet correctly noticed that duplicate timestamps are silently collapsed, but it underweighted the fact that this already changes semantic acceptance for quote datasets.

A smaller but still real miss is the field-validation asymmetry. Required OHLC and underlying price fields are checked for `>= 0`, but optional numeric fields reuse generic helpers that only enforce finiteness (`options_data_store.rs:116-143, 253-286`). That means negative `bid`, `ask`, `mid`, `volume`, `open_interest`, or `quote_age_ms` can get through the importer today. I would rank this below metadata-consistency and manifest/version issues because a compatibility audit may be needed before tightening historical data acceptance, but it is still a blind spot in the current packet.

### What the current third-pass packet overstates or needs to frame more narrowly

The duplicate-timestamp finding is real, but the packet frames it slightly too loosely. The current winner is not arbitrary. Rust’s stable `sort_by` preserves the initial order of equal elements, and `dedup_by` removes all but the first of consecutive matches. Combined with the importer’s “push rows in file order, then stable sort by timestamp” behavior, the retained duplicate-timestamp row is a deterministic “first seen wins” policy, not an undefined one. That does not make the behavior good, but it changes the safest recommendation. The safe-now move is to pin and document the current policy, and to fix the quote-validation bug described above. A broader change to the winner policy should stay deferred.

The earlier packet lineage also leaned toward “manifest scan should fail open on malformed datasets.” I think the operator problem is real, but implementing that by silently weakening `scan_options_dataset_manifests` itself is too strong. The current function is a strict `Result<Vec<_>, String>` helper (`options_data_store.rs:467-485`). Changing it to skip malformed manifests in place would hide corruption and change behavior for any headless or CLI caller that currently relies on hard failure. If best-effort listing is desired, an additive API or a structured partial-success result is safer than mutating the semantics of the existing helper.

Similarly, any pressure to remove `ReaderBuilder::flexible(true)` should stay narrow. The `csv` docs confirm that `flexible(true)` turns off record-width checking, but this importer is already written to tolerate missing optional cells via `record.get(...).unwrap_or_default()` in the optional parse helpers (`options_data_store.rs:116-150, 302-305`). That makes format tolerance look more deliberate than accidental. Tightening it without provider-fixture review is more likely to create regressions than to produce a clean win.

Finally, the row-count issue should not be “fixed” by silently redefining `manifest.row_count`. Downstream options-expression diagnostics surface `dataset_rows` directly from the manifest (`options_expression.rs:2743-2746`). Reinterpreting `row_count` from “rows seen” to “rows retained” would change persisted semantics and user-visible diagnostics. If the repository wants both numbers, that is a schema design task, not a casual local tweak.

## Implementation Priority

### Safe-now hardening

The current third-pass bucket should stand. The safe-now work is still local and testable, but I would reprioritize it.

First, add same-contract metadata validation in the grouping step. When `contract_id` already exists, compare the incoming row’s authoritative contract metadata against the established series metadata before appending a price row. The safest comparison set is `root_symbol`, `option_type`, `expiration_timestamp_ms`, `strike`, and `multiplier`. Be careful with `expiration_date`: comparing the raw string too strictly could reject benign formatting drift, so normalize or derive before comparing if that field is included.

Second, harden manifest and path hygiene at the public API boundary. Persist the trimmed dataset path that was actually opened, and reject clearly invalid dataset IDs before they are used to build per-dataset paths. This should stay small and conservative: reject empty IDs, absolute paths, and directory separators; do not attempt a larger path-canonicalization scheme in the same change.

Third, enforce manifest version support in `read_options_dataset_manifest`. This is a bounded, repo-local hardening step because the writer already emits the current version constant.

Fourth, keep the duplicate-normalized-header rejection from the earlier packets. That remains a clean, low-risk correctness fix.

Fifth, move `top_of_book_quote` sufficiency checks to the retained dataset shape, not the raw pre-dedup row stream. The simplest fix is to dedup first and then count retained quote-populated rows.

Sixth, add focused file-local tests for each of the above. The current tests exercise happy-path aggregate, quote, and derived-mid loads, but they do not pin these contract edges (`options_data_store.rs:536-680`).

### Larger redesign still deferred

Leave manifest-schema redesign deferred. That includes changing the meaning of `row_count`, adding `retained_row_count` or `dropped_duplicate_count`, or introducing richer duplicate-row diagnostics. Those can be worthwhile, but they are persistence-contract changes and will affect user-visible diagnostics.

Leave any “smarter” duplicate-timestamp winner policy deferred. Preferring the row with populated quote fields, preferring the later row, or surfacing a warning per duplicate cluster all change import semantics in ways that should be decided with real provider fixtures, not guessed from this file alone.

Leave best-effort manifest scanning as an additive design task, not an in-place semantic change to the current strict scanner.

Leave manifest atomic-write work deferred. `std::fs::write` clearly replaces the file contents in place, so the current helper is not staged publication, but that remains lower priority than the data-contract issues above.

Leave broad command-layer and loader-layer invariant unification deferred, except for the narrow path/version/metadata checks that are intrinsic to this module’s own public contract.

## Verification If Changed

At minimum, keep the existing targeted verification from the earlier packets:

- `cargo test -p ta_app_core options_data_store::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_options_data::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app --no-run`

If manifest-reader behavior changes, especially version enforcement or dataset-ID/path validation, I would also run `cargo test --workspace --no-run` because the failure surface is public and not limited to the direct importer.

New fixture coverage should be added for:

- duplicate normalized headers;
- explicit root override with a disagreeing CSV root column;
- same `contract_id` with conflicting strike, expiration timestamp, multiplier, or option type;
- `top_of_book_quote` data where only later-dropped duplicate rows carry quote fields;
- manifest version mismatch rejection;
- trimmed dataset-path persistence;
- invalid dataset IDs for `dataset_manifest_path` or any shared validator that feeds it.

If optional numeric validation is tightened, add fixtures that make the new acceptance surface explicit instead of burying the change inside unrelated quote tests.

## Regression And Feature-Preservation Constraints

The main compatibility risk is not performance. It is import acceptance surface.

Do not silently reinterpret `row_count`. That field already feeds downstream diagnostics, so changing its meaning is a persistence and UX change, not just cleanup.

Do not silently change duplicate-timestamp winner policy in the same patch that documents or tests it. The current behavior is deterministic first-seen-wins; if that is not the desired policy, change it explicitly and test it explicitly.

Do not make `scan_options_dataset_manifests` silently best-effort unless the API contract is changed to surface which manifests were skipped. Hiding corruption is worse than inconvenient listing failures for a library helper.

Do not tighten ragged-row tolerance just because `flexible(true)` looks permissive. The importer already behaves as if missing trailing optional fields are allowed. Tightening that requires real dataset audit.

Be careful when enforcing same-contract metadata consistency not to reject merely cosmetic date-string formatting differences. Prefer authoritative numeric fields where possible.

Be careful when enforcing dataset-ID hygiene not to strand already-imported datasets if older IDs were stored with unexpected characters. If such data exists, a migration or compatibility alias may be needed.

There is no meaningful GPU ABI risk in this file, and no urgent hot-path performance issue. The important risks are persistence compatibility, Tauri/CLI error-surface changes, and downstream validation correctness if malformed imports are allowed through.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `Small surgical refactor` is still the right bucket.

The current third-pass packet got the broad direction right. Duplicate normalized headers, explicit root overrides, and duplicate timestamps are still real issues, and the file still does not justify a broader redesign.

But the packet is not yet aimed at the most important next moves. The highest-value safe-now hardening is:

1. same-contract metadata consistency checks;
2. manifest version enforcement;
3. dataset-path and dataset-ID hygiene at the public boundary;
4. quote-semantics validation against retained rows rather than pre-dedup rows;
5. duplicate-header rejection.

The larger redesign items should stay deferred: manifest schema changes around row counts and duplicate diagnostics, smarter duplicate-row winner logic, best-effort manifest scanning, and atomic publication.

So the bucket stays the same, but the priority stack should change. This file is already small enough for a surgical pass; the fourth pass mainly clarifies that the next surgical pass should target contract integrity before it targets broader provenance commentary.
