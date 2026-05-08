# Data Artifact Report Performance Audit

Date: 2026-05-06
Status: audit plan

Update 2026-05-07: `scripts/run_support_perf_gate.ps1` now covers deterministic CSV load/cache/resample, idle Polygon live status/stop, options dataset import/manifest scan, report CSV/XLSX generation/persistence, and disposable artifact save/catalog/hydration/tag timing.

## Goal

Measure and optimize non-strategy performance paths that shape perceived speed: data loading, cache operations, artifact save/load, history hydration, and report generation.

## Scope

- data commands and Polygon/cache paths;
- CSV/sample loading;
- resampling;
- artifact save/load/list/repair;
- run hydration;
- report generation and XLSX export;
- options dataset import/load;
- large JSON serialization/deserialization.

## Repo Anchors

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/commands_options_data.rs`
- `crates/app_core/src/options_data_store.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports*.rs`
- `web/src/services/historyService.ts`
- `web/src/services/dataService.ts`
- `web/src/services/optionsExpressionService.ts`

## Performance Risks

| Surface | Risks |
|---|---|
| CSV load | parsing, validation, copying candles, date parsing |
| Polygon cache | manifest scan, repair/top-off diffing, JSON/CSV processing |
| Resampling | repeated aggregation, full-copy output |
| Options dataset import | large chain rows, quote-quality summary, manifest write |
| Artifact list | directory scan, JSON parse for every summary |
| Artifact detail | full JSON parse, hydration model build |
| Save artifact | large JSON serialize, atomic write overhead |
| Repair artifact | repeated parse/write cycles |
| XLSX report | workbook construction, large sheets, style overhead |

## Measurement Cases

### Data

- sample CSV load;
- large CSV load;
- load cached Polygon dataset;
- resample 1m to 5m/15m/daily;
- Polygon validate/repair/top-off dry run.

### Options Data

- import small fixture;
- import medium realistic fixture;
- load manifest;
- list datasets;
- quote-quality summary generation.

### Artifacts

- save optimization artifact;
- save validation artifact;
- save options artifact;
- list run artifacts with 10/100/1000 files;
- load run artifact detail;
- latest hydration.

### Reports

- build validation XLSX;
- build options report/export if present;
- save report to disk;
- open/reveal path overhead should not be benchmarked as compute.

## Instrumentation Fields

Add optional diagnostics where useful:

- file bytes;
- row count;
- parse milliseconds;
- transform milliseconds;
- serialize milliseconds;
- write milliseconds;
- artifact count scanned;
- artifact count parsed;
- report sheet count;
- workbook bytes;
- warnings.

Timing fields are telemetry and must not enter artifact identity.

## Optimization Ideas To Validate

- Cache artifact summaries instead of parsing full files during list.
- Lazy-load artifact detail only on selection.
- Paginate or virtualize history rows.
- Split report model parse from XLSX generation in timings.
- Avoid recomputing quote-quality summaries on every display.
- Precompute dataset manifest identity/quality once during import.
- Avoid cloning full artifact JSON into multiple frontend stores.

## Verification

- targeted Rust tests for artifact/report/data behavior;
- `node scripts/run_output_parity_check.mjs` if report/export shape changes;
- `npm --prefix web run test -- historyService dataService optionsExpressionService`;
- `npm --prefix web run typecheck`;
- workflow gate if data/artifact changes affect normal workflows.

## Regression Watchpoints

- Do not weaken artifact atomic-write safety for speed.
- Do not skip provenance/identity checks.
- Do not change report content for performance without output parity.
- Do not cache stale artifact summaries without invalidation.
- Do not hide file IO errors behind performance fallbacks.

## Definition Of Done

- Data/artifact/report workflows have timing cases.
- Artifact list/detail and report generation bottlenecks are separated.
- Any caching/lazy-loading change preserves replay and report correctness.
