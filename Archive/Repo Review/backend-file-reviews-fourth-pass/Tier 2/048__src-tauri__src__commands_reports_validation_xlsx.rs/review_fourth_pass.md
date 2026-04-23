# Fourth-Pass Review: `src-tauri/src/commands_reports_validation_xlsx.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_reports_validation_xlsx.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the third-pass packet was directionally right about one important thing and still too soft on several source-local problems. It was right to treat `commands_reports_validation_xlsx.rs` as a user-visible workbook contract, where sheet names, hyperlinks, placeholder text, and summary wording matter more than code-style cleanup. But the third pass drifted toward adjacent concerns such as CSV/header reasoning and artifact save ordering that this file does not actually own.

The live source still looks like a `Small surgical refactor`, and the current third-pass bucket should stand. The reason it should stand, however, is not “artifact-backed ordering matters as much as file generation.” It should stand because the file has a handful of bounded truthfulness and workbook-integrity issues that can be fixed without redesigning the renderer:

- one concrete dashboard layout bug
- one concrete skipped-vs-not-run status bug
- one weakly specified hyperlink construction path
- several under-tested workbook-contract edges

That is a surgical hardening pass, not a rewrite.

## Fourth-Pass Research Log

This fourth pass re-read the live renderer itself first, especially the helper layer (`61-297`), candidate-sheet builders (`299-872`), dashboard (`874-1974`), distributions (`1976-2215`), entrypoint (`2293-2435`), and direct tests (`2437-2635`).

It then cross-checked the current packet against the earlier packets to separate truly source-backed concerns from concerns that were inherited from adjacent report/export surfaces.

To avoid misframing the file, this pass also checked the command-surface context in `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs`. That confirmed the important boundary: this module is IPC-reachable through the report commands, but the function under review still only builds workbook bytes in memory and returns `Vec<u8>`. It does not own artifact persistence, report-directory save ordering, or run-history hydration.

External API research was limited to the workbook behaviors that directly affect the recommendations:

- `rust_xlsxwriter::worksheet::Worksheet`, especially `write_blank()` and hyperlink documentation
- `rust_xlsxwriter::utility::quote_sheet_name()`
- `rust_xlsxwriter` chart docs, especially the presence of explicit empty-cell handling concepts
- Microsoft Excel guidance on how charts treat empty cells

That external read sharpened two review boundaries. First, blank cells are not the same thing as text placeholders, and `write_blank()` is available if the project ever wants that change. Second, changing blanks in chart source ranges is not a cosmetic formatting tweak because Excel chart behavior for empty cells is configurable and type-sensitive.

One limitation should be called out explicitly. Earlier packets cited `src-tauri/src/commands_reports_validation_common.rs` for some cross-file truthfulness issues, but that file was not included in this review bundle. Those earlier cross-file claims remain plausible, but this fourth pass does not restate them as re-verified facts from source. The findings below are therefore weighted toward issues that are directly visible in the bundled source.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet correctly treated this file as a compatibility-sensitive export surface. That framing should remain. The sheet names, workbook layout, dashboard wording, and placeholder semantics are product behavior, not incidental implementation detail.

It was also right not to chase performance. This module is workbook assembly, not a validation hot path, and it has no GPU ABI or runtime-critical ownership.

### What the current third-pass packet missed or underweighted

The biggest miss is a real dashboard layout defect, not an abstract compatibility concern. In the regime section the dashboard writes a `Slice Audit` row at `73,5` / `73,6` (`1741-1744`), and then immediately reuses row `73,5` for the `Bootstrap Compare` section heading (`1752-1754`). The result is that the row-73 value cell still says `see Audit Data -> Regime Slice Audit`, but the row-73 label cell is overwritten to `Bootstrap Compare`. That is a visible workbook bug, not merely a maintainability issue.

The next concrete miss is module-status truthfulness. The helper `disabled_or_omitted_status()` only checks `actual_disabled_module_summary` and `actual_cross_omitted_summary` (`189-197`), even though the dashboard separately surfaces `actual_skipped_module_summary` (`1381-1386`). The helper is used for missing CSCV, SPA, reopt walk-forward, and cross-ticker sections (`1158-1166`, `1214-1218`, `1283-1291`, `1337-1345`). That means a module can be known to have been skipped and still render in its primary status cell as `not run`. The data needed to distinguish `skipped` already exists in the model summary fields; the helper simply does not use it.

The hyperlink path is also weaker than the current packet suggested. The dashboard writes internal links to `Approved Candidates` and `Audit Data` using unquoted worksheet names with spaces:

- `internal:Approved Candidates!A1` (`1491`)
- `internal:Audit Data!A1` (`1494`)

The `rust_xlsxwriter` hyperlink docs and `quote_sheet_name()` utility describe sheet names with spaces as requiring single quotes in Excel-style references. I would not call the current code a confirmed broken-link bug without opening the generated workbook, because the library may normalize more than the docs promise. But it is at minimum a weakly specified assumption, and it becomes more important because those `write_url_with_text()` calls are also currently best-effort (`let _ = ...`). The safer narrow fix is to quote the internal targets and add a regression test, not to broadly propagate every ignored worksheet-layout call.

The file also contains an internal workbook-consistency problem around regime bucket display. The renderer already has a regime bucket display adapter in `display_regime_bucket_label()` (`148-158`), but candidate rows and the dashboard write `row.regime_worst_bucket` raw (`586-596`, `1691-1699`), and the side audit table in `Audit Data` writes `bucket.label` raw (`738-740`). Only the standalone `Regime Audit` sheet uses the display adapter (`831-835`). That means the same workbook can show `high_vol` in one place and `High Vol` in another. This is source-local inconsistency; no cross-file evidence is needed.

Relatedly, the workbook currently has two regime-audit surfaces that do not obviously follow the same contract. The `Audit Data` side-table skips zero-bar buckets (`723-725`), while the standalone `Regime Audit` sheet writes every bucket row it sees (`814-859`). That may be intentional, but the current workbook does not explain the distinction, and the third-pass packet did not flag it. This is not automatically a safe-now behavior change, but it is an unresolved contract question inside the file.

Another underweighted edge is the run-level rejection story. The direct test `validation_report_empties_approved_sheet_when_run_verdict_rejects_final_approval()` proves that the `Approved Candidates` sheet can be intentionally empty when the run verdict rejects final approval (`2540-2600`). At the same time, the dashboard’s `Approved` count comes from `run_verdict.approved_candidate_count` when present (`975-977`), and the distributions sheet counts approval labels from `model.all_rows` (`233-268`, `2016-2027`). That means the workbook can legitimately show an empty approved sheet while still showing non-zero approved counts elsewhere. That may be correct if the workbook is distinguishing candidate-level approval from final run-level approval, but the current wording does not make that distinction explicit. The third-pass packet did not surface this semantic edge at all.

Finally, the direct tests are weaker than the packet implied. The current module tests do not assert that the `Regime Audit` sheet exists by name; they only assert workbook names for five tabs and a raw `sheet5.xml` entry (`2529-2536`). That `sheet5.xml` assertion is not a semantic contract. It would not catch a missing `Regime Audit` sheet, and it ties the test to workbook internals instead of the user-visible workbook surface. The tests also do not pin the dashboard row-73 layout, hyperlink targets, or the run-rejected dashboard/approved-sheet relationship.

### What the current third-pass packet overstated or weakly supported

The third-pass packet imported the wrong neighboring concerns. CSV/header-order reasoning is not the strongest lens for this file; it is an XLSX renderer and does not use the CSV writer APIs at all. Likewise, artifact save ordering and hydration are downstream report-surface concerns, not something `build_validation_report_xlsx()` itself owns. The entrypoint simply builds a `Workbook`, writes sheets, and returns `save_to_buffer()` bytes (`2293-2435`).

That matters because it changes what the safest next step is. The safest next step is not “prioritize artifact save ordering tests instead of workbook cleanup.” The safest next step is to fix the source-local workbook issues and add workbook-level regression checks.

The earlier broad concern about ignored worksheet errors also needs narrowing. Propagating every `set_column_width()` or `set_freeze_panes()` failure would be a coarse change with little practical upside because those are constant, in-range layout calls. The more defensible hardening target is the hyperlink writes, where the current strings are not in canonical quoted form for sheet names with spaces and where failures would directly degrade navigation.

The optional-placeholder discussion was also still too optimistic. Earlier packets treated `not run` / `n/a` mainly as a mixed-type column issue. In this file it is riskier than that because some of the same optional columns feed charts in `Distributions` (`2089-2092`, `2163-2210`) and summary metrics in the dashboard (`1558-1564`, `1775-1787`, `1822-1834`). Moving from text placeholders to blanks would therefore change not just sort/filter behavior but chart input semantics as well. Since the workbook does not currently set an explicit chart empty-cell policy, that should stay deferred.

## Implementation Priority

### Safe-now hardening

First, fix the dashboard row collision. That is the clearest source-visible bug in the file, and it is fully local. Either move the `Bootstrap Compare` heading down one row or move the slice-audit note up, but stop reusing row `73` for two different section labels.

Second, teach the absent-module status helper about `skipped`. The current renderer already carries `actual_skipped_module_summary`; the helper just ignores it. Returning `skipped` when the summary says so is a truthfulness fix, not a redesign.

Third, normalize the dashboard’s internal hyperlinks for sheet names with spaces. Converting the current strings to canonical quoted targets such as `internal:'Approved Candidates'!A1` and `internal:'Audit Data'!A1` is a low-risk hardening change because it does not alter sheet names or displayed link text. It also gives the ignored `write_url_with_text()` calls a much better chance of being correct even if they remain best-effort.

Fourth, strengthen the direct tests around workbook semantics instead of relying on implementation-shape sentinels. Add assertions for:

- the `Regime Audit` sheet by name
- dashboard row-73 wording after the layout fix
- the hyperlink targets or at least the hyperlink markup for quoted internal sheet references
- the run-rejected fixture’s intended relationship between the empty approved sheet and the dashboard summary

Fifth, if no visible wording change is desired yet, add a regression test that intentionally documents the current candidate-level-vs-final-approval divergence. That prevents future refactors from “fixing” it in one sheet while silently breaking the cross-sheet contract elsewhere.

### Larger redesign still deferred

The biggest deferred question is whether the workbook should keep two regime-audit surfaces at all. Right now `Audit Data -> Regime Slice Audit` and the standalone `Regime Audit` sheet use different display paths and different filtering. That deserves a deliberate contract decision later, not an opportunistic one-line cleanup.

Optional numeric placeholder redesign is also still deferred. If the project ever wants blanks or `#N/A` instead of `not run` / `n/a`, that change should be made together with an explicit chart-empty-cell decision and manual Excel verification. Treating it as simple cell-type normalization would be regression-prone.

The dashboard’s `Approved` wording in run-level-rejected cases is likewise deferred unless product wants to rename the concept. The underlying count source may be intentional. The safe-now move is to test and document it, not to silently swap it to `approved_rows.len()`.

Finally, renderer decomposition into smaller sheet builders remains optional debt. The file is large, but the current priority is to fix workbook-truthfulness bugs before moving code around.

## Verification If Changed

Run the direct Rust coverage first:

- `cargo test -p ta_desktop_demo_app commands_reports_validation_xlsx -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`

If the hyperlink strings, dashboard wording, or regime-audit presentation changes, open a generated workbook manually and verify four things in Excel:

1. the dashboard shows separate `Slice Audit` and `Bootstrap Compare` section rows
2. the `Approved`, `Shortlist`, `Distributions`, and `Audit Data` links all navigate correctly, especially the sheets with spaces in their names
3. the `Regime Audit` tab is still present and readable
4. the run-level-rejected fixture still shows the intended relationship between `Run Verdict`, the empty `Approved Candidates` sheet, and the summary counts elsewhere

If any change bleeds into the broader report/export service surface, run the frontend/report command coverage that sits nearest to the IPC boundary, but that is secondary to the workbook-level tests because this file’s primary contract is the XLSX output itself.

## Regression And Feature-Preservation Constraints

Preserve the current sheet names and visible tab vocabulary unless a coordinated downstream change is explicitly intended. Internal links, user expectations, and any saved screenshots or docs will depend on those names more than on workbook-internal sheet numbering.

Do not conflate this renderer with persistence or artifact ordering. This file does not own report-directory writes, artifact retention, or run-history hydration. Fixing workbook bugs should not drag in unrelated report-save plumbing.

Do not change optional numeric placeholders as part of the safe-now pass. Because chart source ranges currently include optional metric columns, blank-vs-text policy is a chart-design decision, not just a cell-format decision.

Treat regime bucket normalization as a visible output change. If the workbook is normalized to one display adapter, verify both the candidate sheets and the regime-audit surfaces together.

There is no GPU ABI or hot-path performance risk here. The real regression surface is workbook compatibility: wording, navigation, cross-sheet consistency, and the exact meaning of approval-related summaries.

## Fourth-Pass Conclusion

The current third-pass bucket should stand: `Small surgical refactor` is still the right action bucket.

What the third-pass packet got right is the compatibility framing. What it got wrong was the center of gravity. The file does not primarily need more discussion of CSV semantics or artifact ordering; it needs a tighter source-local hardening pass.

The best next step is narrow and conservative:

- fix the dashboard row-73 overwrite bug
- distinguish `skipped` from generic `not run`
- canonicalize and test the internal sheet links that contain spaces
- strengthen the workbook regression tests around named sheets, row-73 layout, and the run-rejected semantics

The bigger changes should remain deferred. In particular, do not casually change placeholder policy, do not casually unify candidate-level and final-approved counts, and do not casually collapse the two regime-audit surfaces until the workbook’s intended contract is explicit.

That is the strongest fourth-pass conclusion from the live source: the renderer is basically sound, but the remaining work is more concrete than the current third-pass packet suggests and narrower than a redesign.
