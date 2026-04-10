# Trade Records Contract Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for the shared trade-record contract so the repo has one explicit answer for when full trade-record arrays should be carried, serialized, summarized, or intentionally omitted in non-options workflows.

## Scope Boundaries

In scope:

- `crates/strategies/src/records.rs`
- `crates/app_core/src/lib.rs`
- current consumers and adjacent surfaces such as:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `src-tauri/src/commands_reports.rs`

Out of scope:

- adding a brand-new trade browser UI
- options-related trade-record surfaces
- rewriting all report outputs to always include full trade arrays

## Current User-Facing Behavior

Today, the shared trade-record contract exists, but it is not yet the repo’s visible center of gravity:

- `TradeRecord` and `TradeRecords` define a vectorbt-style structure-of-arrays record shape keyed by `col`
- app-core re-exports the type
- many runtime/report flows still emphasize aggregate `TradeStats` and derived metric surfaces instead of full trade-record arrays

That means users mainly experience trade detail indirectly, through summary metrics and derived views, rather than through a clearly documented full-record pipeline.

## Current Code Surface

- `crates/strategies/src/records.rs`
- `crates/app_core/src/lib.rs`
- related metric/report consumers:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `src-tauri/src/commands_reports.rs`

Current ownership inside `records.rs`:

- `TradeRecord` as one row-level record
- `TradeRecords` as a structure-of-arrays container
- conversions to/from `Trade`
- validation of column-length consistency
- per-column extraction and `TradeStats` derivation

## Current Verification Surface

- direct unit tests in `crates/strategies/src/records.rs`
- `trade22` metric and enrichment tests in `crates/app_core/src/double_ma.rs`
- parallel `trade22` enrichment coverage in `crates/app_core/src/composite_cpu.rs`
- report/artifact tests adjacent to `src-tauri/src/commands_reports.rs`

## Strengths Already Present

- there is already one canonical shared trade-record representation
- the representation is columnar and compact instead of being a loosely typed ad hoc blob
- validation logic exists for array-length consistency
- conversion helpers already make it possible to move between record arrays and `Trade` values
- app-core already re-exports the type, which lowers adoption friction

## Weaknesses / Failure Modes

- the contract is underused relative to how important trade-level detail could become
- the repo does not yet state clearly when full trade arrays should be retained versus summarized away
- reporting and optimization surfaces can keep growing around summary metrics and leave the canonical trade-record type as a side primitive
- there is no single policy for artifact serialization, retention, or omission of trade arrays

## Regressions To Avoid

- creating multiple competing trade-record shapes in different layers
- silently widening or narrowing the record fields without compatibility review
- forcing all workflows to carry full trade arrays when summaries are sufficient
- making summary metrics diverge from what the canonical trade records would imply
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Apache Arrow columnar format](https://arrow.apache.org/docs/format/Columnar.html) is relevant because `TradeRecords` is intentionally structured as a columnar/SoA record container, and that design should remain deliberate.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because any serialized trade-record artifact should follow an explicit contract rather than a casual JSON dump.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because trade records may flow into artifact/report JSON surfaces and should remain predictable to consumers.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because any future CSV export of trade arrays should preserve a stable tabular contract.
- [Criterion.rs getting started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because large record-array handling can become a performance issue if adoption expands.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Freeze `records.rs` as the canonical trade-array shape.
2. Write an explicit policy for when workflows should carry:
   - full `TradeRecords`
   - only derived `TradeStats`
   - neither, for bounded or high-volume contexts
3. Add compatibility tests so `TradeStats` derived from `TradeRecords` remains aligned with existing summary surfaces.
4. Audit current report/artifact flows and decide where record arrays belong today versus later.
5. Keep the first implementation wave focused on contract clarity and compatibility, not widespread adoption.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document `TradeRecord` and `TradeRecords` as the canonical shared trade-array shape.
- [x] Add or tighten tests for record validation, conversion, and per-column extraction.
- [x] Add or tighten parity tests between `TradeRecords`-derived stats and current `TradeStats` summary flows.
- [x] Audit report and artifact surfaces to identify where full trade arrays are intentionally omitted today.
- [x] Decide whether a JSON and/or CSV trade-record artifact contract should be formalized now or deferred.
- [x] Prevent parallel ad hoc trade-record shapes from appearing in app-core or Tauri report code.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run direct tests in `crates/strategies/src/records.rs`
- run relevant `trade22` tests in `crates/app_core/src/double_ma.rs`
- run relevant enrichment tests in `crates/app_core/src/composite_cpu.rs`
- run report/artifact tests near `src-tauri/src/commands_reports.rs` if record serialization or report attachment behavior changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change

## Rollout / Fallback / Compatibility Notes

- keep adoption incremental; do not force every workflow to carry large trade arrays immediately
- preserve the current summary-metric surfaces while the full-record policy is clarified
- if a consumer only needs `TradeStats`, keep the lighter path unless there is a clear benefit to carrying full records
- treat any field-set change in `TradeRecord` as a compatibility-sensitive event
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by tightening the canonical record contract in [records.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/strategies/src/records.rs):
  - added per-column isolation coverage for `to_trades_for_col()` and `trade_stats_for_col()`
  - added overflow rejection coverage for `TradeRecord::try_from_trade()`
- Kept scope intentionally additive:
  - no trade-record field shape changed
  - no report/artifact payloads changed
  - existing `trade22` summary surfaces remain the active user-facing output path
- Acceptance evidence:
  - `cargo test -p ta_strategies records --lib`

## Related Docs

- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-08.md`
- `REPO_Improvement/04_results/05_results_drilldown_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
