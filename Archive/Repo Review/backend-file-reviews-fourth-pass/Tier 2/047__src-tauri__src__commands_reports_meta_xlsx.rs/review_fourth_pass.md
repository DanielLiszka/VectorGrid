# Fourth-Pass Review: src-tauri/src/commands_reports_meta_xlsx.rs

Review date: `2026-04-16`  
Source file: `src-tauri/src/commands_reports_meta_xlsx.rs`  
First-pass packet: `review__first_pass.md`  
Second-pass packet: `review__second_pass.md`  
Third-pass packet: `review__third_pass.md`  
Third-pass action bucket: `Small surgical refactor`  
Fourth-pass action bucket: `Small surgical refactor`  
Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass packet was directionally right about one important thing: this file is compatibility-sensitive because it emits a user-visible workbook, not an internal throwaway blob. However, the packet drifted into adjacent concerns that the live source does not actually own. This module is a workbook-byte builder with a narrow contract: sheet names, cell values, chart references, placeholder behavior, and error posture. It does **not** choose the report path, persist artifacts, or control frontend hydration order.

That matters because the safest next steps depend on the file’s real blast radius. The live source still looks like a `Small surgical refactor`, but the sharpest work is narrower than the current packet implied: fix the local median bug without widening the helper blast radius, harden workbook-contract tests, and avoid opportunistic format cleanups that would create regressions without solving a clear bug.

## Fourth-Pass Research Log

I reopened the live source first, then the current third-pass packet, and only used the earlier packets to understand where the review had drifted.

I then checked the local command/context files to validate ownership and coupling:

- `context__02__src-tauri__src__commands.rs:132-135` shows that `quantile` is re-exported from the reports surface, which means a “fix quantile globally” recommendation is broader than a one-file cleanup.
- `context__01__src-tauri__src__lib.rs:121-138` shows this workbook builder is behind report/meta commands rather than being a direct Tauri command itself.
- The live source confirms the real contract surface is in-memory workbook construction via `build_meta_report_xlsx_bytes(...) -> Result<Vec<u8>, String>` and `Workbook::save_to_buffer()` at `src-tauri/src/commands_reports_meta_xlsx.rs:5-8,640-642`.

I also rechecked external docs only where they sharpened the boundary of a safe recommendation:

- official `rust_xlsxwriter` workbook/worksheet/chart docs, especially `save_to_buffer()`, blank-cell support, chart insertion, and current NaN/Infinity handling;
- official Microsoft Excel median semantics for even-sized samples.

Those sources mainly reinforced two points: first, `save_to_buffer()` is a normal in-memory export path rather than a smell by itself; second, Excel median semantics really do require averaging the middle two values for even-sized numeric samples.

## What Fourth Pass Sharpened

### Bucket assessment

The third-pass bucket should **stand**.

Nothing in the live source justifies reclassifying this into a redesign or a no-op. The file still sits in the narrow middle ground the earlier packets were circling: it is not a hot path, GPU path, or ABI surface, but it is still semantics-sensitive because workbook output is user-visible and persisted elsewhere.

### What the current third-pass packet got right

The current packet was right to keep this out of performance triage and to treat visible workbook schema as compatibility-sensitive. That is the correct posture for sheet names, placeholder rows, chart presence, and sparse-data behavior.

It was also right that changes here should preserve current behavior unless a real correctness issue is clear. The median issue is such a correctness issue. Most of the other candidate changes are not.

### What it missed or underweighted

#### 1. The median fix should be **local**, not a generic `quantile()` rewrite

This is the biggest thing the current packet still missed.

The live file computes the Asset Matrix value via `median_train_pnl_pct_by_dataset()` and that helper currently delegates to `quantile(&vs, 0.5)` after sorting at `src-tauri/src/commands_reports_meta_xlsx.rs:645-656`. The local `quantile()` implementation at `src-tauri/src/commands_reports_meta_xlsx.rs:659-669` is not a true statistical median for even-sized samples.

But the sharper fourth-pass finding is that `quantile` is not safely “just local report glue.” The reports surface re-exports it in `context__02__src-tauri__src__commands.rs:132-135`. That means changing `quantile()` semantics in place could affect other report code that was not included in this bundle.

So the safe-now fix is narrower than the earlier packet suggested:

- keep `quantile()` semantics unchanged unless the wider reports surface is audited;
- fix the Asset Matrix by adding a dedicated median path for `median_train_pnl_pct_by_dataset()`.

That gives the workbook the correct value without silently changing every other caller that may rely on the current rounded-index quantile behavior.

#### 2. The Asset Matrix currently keys on display label, not stable identity

The current packet did not call out a real edge case in the live code.

The “By Asset (Train)” sheet explicitly exports both `Dataset` and `Dataset key` in the header and rows at `src-tauri/src/commands_reports_meta_xlsx.rs:304-320,346-350`. But the Asset Matrix builds its column set from `tr.dataset` only at `src-tauri/src/commands_reports_meta_xlsx.rs:421-425`, and `median_train_pnl_pct_by_dataset()` also groups by `r.dataset.clone()` only at `src-tauri/src/commands_reports_meta_xlsx.rs:645-649`.

If two different inputs can share the same human-readable dataset label while differing in `dataset_key`, the matrix silently collapses them into one column and one median bucket. That is a real workbook-correctness risk, not merely a layout preference.

I would not “fix” this by blanket-switching headers to `dataset_key`; that would be a visible contract change and would make the workbook much noisier. But the collision risk is real and underweighted. The safer approach is to add an explicit duplicate-label fixture first, then either:

- document that duplicate dataset labels are impossible by upstream contract, or
- lazily disambiguate only when a collision actually occurs.

#### 3. The format mismatch is broader than the packet’s pass-rate example

The earlier packet focused on pass-rate formatting inconsistency, which is valid, but incomplete.

The workbook also labels values as percentages while writing raw fractional numbers with `0.000000` formatting in at least two other visible places:

- `By Asset (Train)` uses the header `PnL %` at `src-tauri/src/commands_reports_meta_xlsx.rs:304-320` but writes `tr.pnl_pct` with `num6_fmt` at `src-tauri/src/commands_reports_meta_xlsx.rs:355-360`.
- `Asset Matrix` titles the sheet `Per-asset PnL% matrix (median by dataset)` at `src-tauri/src/commands_reports_meta_xlsx.rs:411-417` but writes the matrix values with `num6_fmt` at `src-tauri/src/commands_reports_meta_xlsx.rs:472-475`.

That does not prove the code is numerically wrong. It does show that the workbook has a broader ratio-vs-percent presentation ambiguity than the current packet acknowledged. Because this is visible contract surface, the correct response is not to ad hoc flip individual columns to `pct_fmt`; it is to choose one reporting rule and apply it deliberately.

#### 4. The deterministic-ordering story is a current strength and should be preserved

The third-pass packet talked about compatibility at a high level but did not call out an existing strength in the live source: the file already uses `BTreeSet` and `BTreeMap` at `src-tauri/src/commands_reports_meta_xlsx.rs:2,421-425,645-656`. That helps keep workbook ordering stable across runs.

If the matrix-keying logic changes, that deterministic ordering should be preserved. It is part of what makes workbook diffs and regression tests tractable.

#### 5. The test gap is more specific than “needs deeper coverage”

The live tests at `src-tauri/src/commands_reports_meta_xlsx.rs:791-983` are still very shallow. The current packet was right about that, but it could have been much sharper about exactly what is missing.

The current tests do **not** lock:

- even-sized median behavior in the Asset Matrix;
- the empty/sparse branches for no groups, no train rows, no holdout rows, and empty histograms;
- duplicate dataset-label behavior versus `dataset_key`;
- all inserted charts, even though the workbook can emit two summary charts plus four distribution charts at `src-tauri/src/commands_reports_meta_xlsx.rs:255-283,634-637`.

The existing chart assertions only check `chart1.xml` and `chart2.xml` at `src-tauri/src/commands_reports_meta_xlsx.rs:980-981`. That is well below the contract this file actually owns.

### What the current packet overstated or weakly supported

#### 1. Artifact save ordering and hydration are outside this file’s ownership

This was the clearest scope drift.

The current packet’s emphasis on artifact-backed export ordering and save-order expectations is not well supported by the live source. This module does not pick paths, write files, or persist `report_xlsx_path`. It returns workbook bytes. The surrounding meta/report commands own persistence behavior.

That does **not** mean end-to-end smoke is useless. It does mean the fourth-pass review should stop attributing artifact-save ordering to this file. The real regression risk here is simpler: if this builder starts failing or silently changes workbook structure, downstream persistence surfaces will receive different bytes or an error.

#### 2. CSV/header-order guidance was imported from the wrong neighboring surface

The current third-pass research log leaned on CSV writer guidance and header-order framing that belong to adjacent exporters, not this XLSX builder. This file has no CSV serialization path. That is a sign the packet was generalizing from the reports area instead of staying anchored on the live source.

#### 3. Hard-failing on `serde_json::to_string(...).unwrap_or_default()` is not a clearly justified safe-now change

The earlier packet treated the three `unwrap_or_default()` call sites at `src-tauri/src/commands_reports_meta_xlsx.rs:162,341,529` as a meaningful diagnostics gap. I do not think the evidence is strong enough to elevate that into a hard-failure recommendation.

In this surface, `params` already behaves like JSON-shaped data. A serialization failure is much less likely than a normal workbook-format regression, and converting that low-probability edge into a new report-generation error path could reduce robustness more than it helps.

If someone wants to improve this area, the conservative move is a visible sentinel string on failure, not a newly propagated export error. Even that is lower priority than the median and test issues.

#### 4. The fixed merge-range cleanup is cosmetic, not a fourth-pass priority

The Asset Matrix title merge range is still hard-coded to column 10 at `src-tauri/src/commands_reports_meta_xlsx.rs:411-418` while dataset columns are dynamic. That is real, but it remains cosmetic. It should not be elevated ahead of semantics or contract-test fixes.

## Implementation Priority

### Safe-now hardening

1. **Fix the Asset Matrix median locally.**

   Do not rewrite shared `quantile()` semantics in this pass. Add a dedicated median path in `median_train_pnl_pct_by_dataset()` so even-sized samples use the average of the two middle values. Add an explicit worksheet-level regression fixture that proves a two-value sample such as `[0.01, 0.99]` yields `0.50` rather than `0.99`.

2. **Turn the workbook tests into actual contract tests.**

   Add targeted fixtures for:

   - empty `meta.groups` / no holdout summary;
   - no train rows;
   - no holdout rows;
   - even-sized Asset Matrix median;
   - duplicate `dataset` labels with different `dataset_key` values;
   - full chart emission, preferably by asserting chart/drawing part count or relationships rather than only `chart1.xml`/`chart2.xml`.

   This is the highest-value hardening after the median fix because it prevents future “cleanup” work from accidentally changing workbook semantics.

3. **Decide the dataset-identity assumption explicitly.**

   Before changing the Asset Matrix key, add a collision fixture. If duplicate display labels are impossible by upstream contract, document that assumption next to the matrix builder. If they are possible, disambiguate only on collision so the default human-readable workbook stays stable.

4. **Add a short module-level contract note.**

   Keep it narrow: list the five sheets, note that summary/distribution charts are conditional on available data, and note that `"-"` placeholders are currently intentional visible workbook behavior. That is enough to keep future refactors from rediscovering the contract by accident.

### Larger redesign still deferred

1. **Global report-stat helper unification.**

   Because `quantile` is re-exported through the reports surface, any attempt to standardize quantile/median behavior across report modules should be handled as a broader reports audit, not slipped into this file under a “small cleanup” label.

2. **Cross-workbook ratio/percentage normalization.**

   The pass-rate example from the earlier packet was real, but the broader issue is that this workbook mixes `%` labels with raw fractional formatting. That should be solved by choosing one product rule for report builders, not by flipping one or two cells here and leaving neighboring sheets inconsistent.

3. **Placeholder-policy redesign.**

   The library supports blank cells, so changing `"-"` placeholders is technically possible. It is still a visible contract change. Do that only if workbook consumers actually want blanks and after worksheet-level regression coverage exists.

4. **Streaming / memory-mode redesign.**

   `save_to_buffer()` is appropriate for the current `Vec<u8>` API. If meta workbooks eventually become large enough for measured memory pressure, that is the point to evaluate a broader redesign around file-backed or constant-memory writing. This bundle does not provide evidence that such a redesign is needed now.

## Verification If Changed

Run the local report tests first:

1. `cargo test -p ta_desktop_demo_app commands_reports_meta_xlsx -- --nocapture`

Then run the meta flow that consumes the builder:

2. `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`

If `quantile()` itself is touched rather than the local median path, widen verification beyond this file before landing. The re-export in `context__02__src-tauri__src__commands.rs:132-135` is enough to treat that as a broader reports change.

If any workbook-visible formatting changes are made, do one manual open of a generated Stage 3 workbook and verify:

- sheet names;
- no-data fallback rows;
- Asset Matrix values for the even-sized median fixture;
- duplicate-dataset header behavior, if changed;
- chart count and placement;
- visible ratio/percent rendering on the affected sheets.

No GPU ABI, validation-engine, or perf gate work is justified here unless the change escapes the workbook builder and starts affecting wider report infrastructure.

## Regression And Feature-Preservation Constraints

Keep the current workbook schema stable unless a real bug is being fixed:

- preserve sheet names and title strings;
- preserve deterministic ordering from the current `BTreeSet` / `BTreeMap` usage;
- preserve the current sparse-data fallback rows unless there is an explicit product decision to fail instead of render;
- do not change `quantile()` semantics casually because this file does not prove it is a private helper;
- do not blanket-replace `dataset` with `dataset_key` in workbook headers without first confirming the upstream identity contract;
- do not replace `"-"` placeholders or percentage/ratio formatting ad hoc without worksheet-level assertions and a manual open.

Also keep the error posture conservative. This builder sits on the meta export path, so introducing new hard-failure branches without a clear correctness payoff is more likely to create regressions than to improve code quality.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `Small surgical refactor` is still the right classification.

What the current packet got right was the compatibility-sensitive framing. What it still underweighted was the source-specific shape of the safest changes. The most valuable work is not artifact-order cleanup, CSV-style export generalization, or a broad report-helper refactor. It is a smaller, sharper hardening pass:

- fix the Asset Matrix median **locally**;
- raise tests to workbook-contract level;
- clarify the dataset-identity assumption before changing matrix keying;
- defer visible placeholder and ratio/percentage redesign until there is an explicit contract decision.

In other words: keep the bucket, narrow the scope, and avoid turning one clear local bug into a wider reports-surface regression.
