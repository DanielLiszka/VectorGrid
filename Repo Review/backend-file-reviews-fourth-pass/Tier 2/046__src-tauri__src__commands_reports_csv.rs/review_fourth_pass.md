# Fourth-Pass Review: `src-tauri/src/commands_reports_csv.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_reports_csv.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_csv.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_csv.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_csv.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file warranted a fourth pass because the current third-pass packet kept the right bucket, but not the right focus.

The live source is still a very small serializer boundary: it calls `build_model_from_validation_res`, chooses `shortlist_rows` or `all_rows`, emits a fixed 28-column header, serializes rows with a hand-written `format!`, and returns one in-memory CSV string (`src-tauri/src/commands_reports_csv.rs:18-95`). It does **not** build XLSX workbooks, manage sheet names, control placeholder cells, persist artifacts, or own artifact save ordering. Those concerns live elsewhere in the reports and artifact surfaces. The command is user-visible because `build_validation_shortlist_csv` is exported through the reports API and registered on the Tauri command surface (`src-tauri/src/commands.rs:136-138`, `src-tauri/src/lib.rs:126-127`), but that does not make workbook-layout or artifact-retention concerns local to this file.

So the main reason for the fourth pass was to challenge the third-pass framing, narrow the review back to what the live file actually owns, and decide whether the bucket should really remain small and surgical.

## Fourth-Pass Research Log

I re-read the live source first, then the prior packets, and used local repo-context files only to validate ownership boundaries.

1. **Live source and tests**
   - `csv_escape` still handles commas, quotes, LF, and CR by quoting and doubling quotes (`src-tauri/src/commands_reports_csv.rs:4-10`).
   - `build_validation_shortlist_csv_text` still builds the model with a hard cap of `Some(4096)`, prefers `shortlist_rows`, falls back to `all_rows`, errors on zero rows, and joins output with `\n` (`src-tauri/src/commands_reports_csv.rs:21-30`, `33-95`).
   - The direct tests still rely on `csv.lines()` and a handwritten single-record parser (`src-tauri/src/commands_reports_csv.rs:103-130`, `168-174`, `206-208`).

2. **Prior packet comparison**
   - The first pass correctly identified the real blind spots: newline-bearing field coverage, missing negative tests, missing shortlist-preference coverage, and the contract sensitivity of a manual serializer.
   - The third pass was directionally right that this is a user-visible compatibility surface, but it drifted into XLSX and artifact-centric guidance that the live file does not own.

3. **Local contract check**
   - The Tauri surface confirms this is an exported report command, so output compatibility matters (`src-tauri/src/commands.rs:136-138`, `src-tauri/src/lib.rs:126-127`).
   - The same context split also shows artifact operations are owned elsewhere, not here (`src-tauri/src/commands.rs:44-51`, `136-138`; `src-tauri/src/run_artifacts.rs` context file).

4. **External standards and library behavior**
   - [RFC 4180](https://datatracker.ietf.org/doc/html/rfc4180) documents the commonly followed CSV format: records are line-delimited, each record should have the same field count, fields containing commas, quotes, or line breaks should be quoted, and a final trailing line break is optional.
   - Rust’s [`str::lines`](https://doc.rust-lang.org/std/primitive.str.html#method.lines) splits on `\n` and `\r\n`, which makes it unsuitable as a record parser for valid CSV containing quoted embedded newlines.
   - The Rust [`csv::Writer`](https://docs.rs/csv/latest/csv/struct.Writer.html) writes valid CSV, quotes fields when needed, and checks record widths, but its default terminator is still `\n`, not RFC-style `\r\n`. That means a future writer migration would need explicit terminator pinning if CRLF ever becomes a product requirement.
   - [OWASP CSV Injection](https://owasp.org/www-community/attacks/CSV_Injection) remains the right reference for spreadsheet-formula risk, and its Excel caveat is important: naive escaping policies can still be brittle when files are saved and reopened.

One limitation of this fourth-pass bundle is that `commands_reports_validation_common.rs` was not included directly, so upstream row-shaping details were treated as an external contract rather than re-reviewed line by line. That makes this pass more conservative, not less: it focuses on the serializer and its tests, which *are* fully available.

## What Fourth Pass Sharpened

### The current third-pass bucket should stand

The third-pass **action bucket** should remain `Small surgical refactor`.

Nothing in the live source suggests a larger redesign is warranted. The implementation is small, bounded by the upstream `Some(4096)` cap, not performance-critical, and does not touch persistence, GPU ABI, runtime orchestration, or artifact storage. The local risk is almost entirely **format compatibility and test coverage**, not architectural debt.

### What the current third-pass packet got right

The current packet was right about two important things.

First, this is a user-visible output boundary, not disposable glue. Because `build_validation_shortlist_csv` is on the exported Tauri surface, header order and row semantics are compatibility-sensitive. Second, broad rewrites are not justified here. The live escaper already matches the core CSV quoting cases the file claims to support, and there is no evidence of a runtime performance or safety problem large enough to justify a redesign.

### What the current third-pass packet overstated

The third-pass packet materially overreached when it pivoted to workbook and artifact guidance.

`commands_reports_csv.rs` has no workbook bytes, sheet names, placeholder cells, hyperlinks, save paths, or artifact ordering logic. Its only observable behavior is CSV text generation. Recommending regression coverage around “sheet names, placeholder cells, and save-order expectations” is therefore not wrong at the **reports subsystem** level, but it is too strong and too diffuse for this specific file. It risks sending the next change in the wrong direction.

That overreach also made the third-pass verification advice weaker than it should have been. For this file, the highest-value verification is not XLSX or artifact hydration smoke. It is record-aware CSV parsing against the exact text emitted here.

### What the current third-pass packet missed or underweighted

The most important missed point is that the existing tests are still too sparse to lock the **actual row contract**.

The file writes 28 columns with a long manual `format!` (`src-tauri/src/commands_reports_csv.rs:35-91`), but the current tests only assert the header plus a handful of populated output slots (`src-tauri/src/commands_reports_csv.rs:140-243`). Most optional metric fields are empty in the fixtures, which means a future column-order bug in the middle of the row could slip through if the affected columns serialize as empty strings in the test data. Put differently: the current tests mostly prove that *some* cells serialize correctly, not that the full row layout is stable.

The current packet also underweighted the importance of the file’s existing textual encoding choices. The serializer currently emits:

- `hard_gate_passed` as `true` / `false` or empty string,
- `legacy_shortlisted` as `1` / `0`,
- `cross_pareto` as `1` / `0`, and
- missing optional numeric/stat fields as empty strings.

That asymmetry is visible output contract, not cleanup fodder. A future contributor could easily “normalize” those encodings and accidentally break consumers or golden expectations. The current tests do not explicitly lock those encodings down.

Finally, the earlier packets were a little too permissive about “centralizing helpers locally” as a safe-now improvement. For the tests specifically, centralizing the handwritten parser would mostly centralize the same blind spot. The narrower and safer improvement is to adopt a real CSV parser in tests while leaving the runtime serializer alone unless there is a concrete reason to change it.

## Implementation Priority

### Safe-now hardening

The safe-now work should stay narrow and test-heavy.

1. **Replace `.lines()`-based assertions with record-aware CSV parsing in tests.**  
   This is still the top issue. The live serializer explicitly claims to support quoted fields containing line breaks, but the current tests cannot verify that contract because they split the entire CSV text into lines before parsing it. The safest change is to parse the emitted CSV in tests with the Rust `csv` crate, not with another handwritten helper.

2. **Add one dense row fixture that makes the optional columns visible.**  
   The current fixtures leave too many columns empty. Add a single fixture—either a handcrafted validation payload if the upstream builder allows it, or a captured validation artifact fixture from the reports path—that yields non-empty values for as many of these columns as possible: `hard_gate_passed`, `legacy_shortlisted`, `psr`, `dsr`, `track_record_bars`, `min_track_record_length_bars`, `wf_*`, `cscv_*`, `cross_score`, `cross_pareto`, `bootstrap_prob_pnl_positive`, and `perturb_pnl_mean`. Then assert the full parsed row, not just spot-checks. That is the highest-leverage protection against future header/row drift inside the manual `format!`.

3. **Add direct negative-path coverage.**  
   The first pass was right here. Add a test for an invalid payload that fails model construction and a test for the no-candidate path that returns `validation_res has no candidates`.

4. **Add direct coverage for shortlist preference, not just fallback.**  
   The live serializer chooses `shortlist_rows` when present and only falls back to `all_rows` when the shortlist is empty (`src-tauri/src/commands_reports_csv.rs:21-26`). The current tests only lock the fallback case. Add a fixture that produces both a non-empty shortlist and a divergent all-rows set, or cover that behavior at the nearest test boundary that can induce both collections if this module’s JSON fixture surface is too narrow.

5. **Add a short contract comment above the serializer.**  
   This is low-risk and worthwhile once the tests are stronger. Document that this file intentionally emits a fixed 28-column shortlist CSV, preserves current textual encodings, prefers shortlist rows when available, and should not be casually “cleaned up” without output-compatibility review.

### Larger redesign still deferred

The larger redesign ideas should remain deferred.

1. **Runtime migration to `csv::Writer` is not a safe-now requirement.**  
   It could be reasonable later, but only with output-compatibility fixtures in place first. The current runtime serializer is small and understandable, and a writer migration would change the code shape without solving the main present bug, which is inadequate test coverage. It also would not automatically change the line terminator policy because `csv::Writer` defaults to `\n` as well.

2. **Spreadsheet-formula mitigation should stay deferred pending product policy.**  
   This is a real category of risk for CSV opened in spreadsheet software, but the live file does not clearly ingest high-risk free-form text at the serializer boundary, and OWASP’s Excel caveat makes naive escaping easy to overstate. Any mitigation here would intentionally change visible cell contents and should therefore be treated as a product and compatibility decision, not a cleanup task.

3. **CRLF, BOM, and Excel-specific interoperability tweaks should stay deferred unless a concrete consumer problem exists.**  
   The current file returns one UTF-8 Rust `String` joined with `\n`. That is a visible format choice. It should only change deliberately, with downstream verification, not as an incidental side effect of a library swap.

## Verification If Changed

Because this fourth-pass bundle did not include a runnable full workspace, the commands below are **recommended verification steps**, not rerun claims from this pass.

1. `cargo test -p ta_desktop_demo_app commands_reports_csv -- --nocapture`  
   This remains the highest-signal leaf test lane.

2. `cargo test -p ta_desktop_demo_app validation_shortlist_csv_smoke -- --nocapture`  
   Keep the parent reports smoke because it catches module wiring regressions above the leaf tests.

3. `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture`  
   Re-verify the async Tauri wrapper if the serializer contract or error surface changes.

4. `npm --prefix ./web run test -- src/services/validationService.test.ts`  
   Keep frontend command-orchestration coverage for the download path.

5. `npm --prefix ./web run test -- src/stores/validationStore.actions.test.ts`  
   Light wiring coverage remains worth keeping because the user-visible flow is export-driven.

6. If any visible CSV semantics change, do one manual shortlist export and inspect it in a spreadsheet-aware viewer.  
   That manual smoke is only needed if output content changes, not for test-only hardening.

## Regression And Feature-Preservation Constraints

The regression constraints are narrower than the third-pass packet implied, but they are real.

- Preserve the current **28-column header order** and exact column count unless downstream consumers move together.
- Preserve the current **shortlist-first, all-rows fallback** behavior.
- Preserve `params_json` as one CSV field, not as exploded columns.
- Preserve the current textual encoding choices unless they are changed deliberately and tested: `hard_gate_passed` as `true` / `false` or empty, `legacy_shortlisted` as `1` / `0`, `cross_pareto` as `1` / `0`, and missing optional metrics as empty strings.
- Preserve the current newline policy unless a deliberate format decision is made. Do not assume that swapping in a CSV library is behavior-neutral.
- Do not add spreadsheet-formula sanitization casually. That is a semantics change and may still be brittle depending on spreadsheet behavior.

The main local regression risk is therefore **CSV compatibility drift**, not performance, GPU behavior, persistence, or IPC lifecycle. IPC matters only in the narrow sense that the command is exported; the file itself does not own artifact retention or workbook persistence.

## Fourth-Pass Conclusion

`Small surgical refactor` should **stand** as the current action bucket.

The third-pass packet got the high-level compatibility framing right, but it did not keep the review tightly scoped to the live file. Its workbook and artifact emphasis is too strong for `commands_reports_csv.rs`, while the real underweighted risk is weaker-than-needed protection for the manual row serializer and its visible field encodings.

The safest next move is not a redesign. It is to harden the tests around actual CSV-record behavior and around the full 28-column row contract, then add a small inline contract comment. Runtime writer migration, spreadsheet-formula mitigation, and line-ending policy changes remain deferred until there is either a concrete interoperability problem or a product decision that justifies changing visible output.
