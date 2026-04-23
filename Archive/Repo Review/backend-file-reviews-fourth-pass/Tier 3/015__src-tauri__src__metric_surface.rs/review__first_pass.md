# Review Packet: `src-tauri/src/metric_surface.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/metric_surface.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful compatibility bridge for optimization metric-surface metadata, but not yet fully hardened because fallback classification can under-report non-legacy surfaces for older artifacts and load paths do not normalize stale or missing surface metadata`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Artifact-contract-sensitive`, `Backward-compat-sensitive`, `Meta-analysis-sensitive`, `History/report-sensitive`

## What This File Owns

`metric_surface.rs` owns the desktop crate's optimization metric-surface sidecar schema and the compatibility logic that derives that schema from saved run artifacts.

This file owns:

- canonical field-name lists for the two current non-legacy optimization metric surfaces:
  - `CORE17_FIELDS`: `src-tauri/src/metric_surface.rs:5-25`
  - `TRADE22_FIELDS`: `27-49`
- the local enum and summary DTO used to classify artifacts:
  - `OptimizationMetricSurfaceKind`: `53-66`
  - `ArtifactMetricSurfaceSummary`: `69-95`
- artifact JSON inspection helpers used to infer the effective metric surface from saved run metadata:
  - request/route helpers: `113-206`
  - strategy/route fallback policy: `208-275`
- projection of `ta_strategies::double_ma::Metrics` into canonical `core17` and `trade22` row order:
  - `278-337`
- sidecar payload construction for persisted artifacts:
  - `416-463`
- a large compatibility test matrix covering multiple strategy kinds, stale-schema backfill, and field-order preservation:
  - `466-1497`

This file does not own:

- the authoritative runtime emission of `metric_surface_kind`
  - that is produced by app-core optimization paths such as `crates/app_core/src/double_ma.rs:349-395`
- artifact persistence and hydration
  - that lives in `src-tauri/src/run_artifacts.rs`
- meta warning generation
  - that lives in `src-tauri/src/commands_meta.rs:2321-2383`
- report export or frontend rendering
  - that lives in `src-tauri/src/commands_reports.rs:192-228`
  - and `web/src/services/historyService.ts:538-552`, `1350-1390`

## Classification

- Public surface: indirect only
  - this module is `pub(crate)`, but it defines persisted artifact schema consumed across backend, reports, and frontend history/detail views
- Hot path: no
  - it runs on artifact save/read/report/meta paths, not in the optimization kernel loops
- Runtime-critical: moderate
  - incorrect classification skews artifact compatibility, meta warnings, and saved-run UX
- CUDA/runtime-critical: indirect only
  - it reasons about GPU route metadata, but does not run CUDA work itself
- Strategy-local: no
  - this is cross-strategy compatibility glue
- Low-risk support code: no
  - computationally light, but semantics-sensitive

## Direct Interactions

### Backend interaction map

- save-time artifact backfill:
  - `src-tauri/src/run_artifacts.rs:1131-1134`
- saved-artifact hydration and detail responses currently clone stored metadata only:
  - `src-tauri/src/run_artifacts.rs:1775`
  - `src-tauri/src/run_artifacts.rs:1803`
- meta warning and summary generation rederive effective surfaces through this module:
  - `src-tauri/src/commands_meta.rs:2321-2383`
- validation report export copies whatever `optimization_metric_surface` is already persisted:
  - `src-tauri/src/commands_reports.rs:192-228`
- field-order report tests depend on this builder:
  - `src-tauri/src/commands_reports.rs:3606-3620`

### Runtime source-of-truth anchors

- app-core determines runtime `metric_surface_kind` in the optimization result:
  - `crates/app_core/src/double_ma.rs:349-395`
- optimization result transport carries that field into the desktop layer:
  - `crates/app_core/src/double_ma_result.rs:15-39`
  - `crates/app_core/src/double_ma_result.rs:225-254`
- non-exact routes in app-core still emit non-legacy surfaces when appropriate:
  - `crates/app_core/src/double_ma.rs:388-395`
  - `crates/app_core/src/double_ma.rs:16060-16064`
  - `crates/app_core/src/double_ma.rs:19679-19683`

### Frontend interaction map

- history/detail rendering reads only persisted metric-surface metadata:
  - `web/src/services/historyService.ts:538-552`
  - `web/src/services/historyService.ts:1355-1387`
- workflow artifact store preserves the hydrated blob as-is:
  - `web/src/stores/workflowArtifactsStore.ts:318-333`
- validation-service payload threading preserves the same saved artifact metadata:
  - `web/src/services/validationService.ts:194`
  - `web/src/services/validationService.ts:560-561`
  - `web/src/services/validationService.ts:712`

## Standards And Research Log

This packet was written after targeted external review focused on JSON-sidecar schema handling, JSON Pointer semantics, Rust API contract clarity, and verification expectations for compatibility code.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines a persisted compatibility schema and stringly-typed classification contract
   - Standard informed:
     - API-facing artifacts should preserve clarity, validate invariants, and avoid undocumented drift-prone contracts

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - this file serializes a sidecar artifact with optional fields and omission semantics
   - Standard informed:
     - schema omission rules should be deliberate and compatible with versioning expectations

3. `serde_json::Value` JSON Pointer documentation
   - Source: <https://docs.rs/serde_json/latest/serde_json/enum.Value.html>
   - Why it was consulted:
     - this module uses JSON Pointer heavily to traverse saved artifacts
   - Standard informed:
     - `Value::pointer` returns `None` for missing paths, so fallback inference needs to treat path absence explicitly rather than assuming semantic equivalence

4. Rust Book: Writing Automated Tests
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why it was consulted:
     - this file is compatibility-heavy and depends on regression tests more than local type safety
   - Standard informed:
     - compatibility code should have direct tests for intended behavior, especially around stale-format and fallback paths that the type system cannot prove

## Findings

## Correctness And Safety

### 1. Fallback rederivation is stricter than the backend's own metric-surface contract, so older artifacts can be downgraded to `legacy`

- Evidence:
  - sidecar fallback only grants `core17` to:
    - `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`, `CompositeCpu` unconditionally: `208-219`
    - `CompositeTrend` and `BreakoutMomentum` only on exact GPU-kernel routes: `212-214`
    - selected strategy kinds only on CPU routes: `215-217`
  - trade-tail fallback is even narrower:
    - `222-230`
  - `RegimeHybrid` is allowed for exact GPU-kernel inference only:
    - `233-247`
  - app-core itself emits non-legacy metric surfaces on non-exact routes:
    - default non-exact runtime result returns `trade22` or `core17`: `crates/app_core/src/double_ma.rs:388-395`
    - `RegimeHybrid` CPU trade22 coverage: `16060-16064`
    - `RegimeHybrid` GPU full17 coverage: `19679-19683`
- Why it matters:
  - old or externally moved artifacts that lack current `optimization_metric_surface` metadata can be reclassified more conservatively than the real runtime contract
  - that misclassification then flows into:
    - `summarize_artifact_metric_surfaces`: `387-407`
    - meta warnings and summaries: `src-tauri/src/commands_meta.rs:2321-2383`
- Practical consequence:
  - meta-selection diagnostics can report mixed or legacy corpora for artifacts that were valid non-legacy runs under app-core semantics
- Smallest justified fix:
  - align fallback policy with the runtime source of truth for non-exact routes, especially `RegimeHybrid`
  - if broad fallback is intentionally conservative, document that policy explicitly and stop using it for compatibility diagnostics that imply objective runtime truth

### 2. Read/report/frontend flows do not normalize missing or stale metric-surface metadata on load

- Evidence:
  - save-time backfill exists:
    - `src-tauri/src/run_artifacts.rs:1131-1134`
  - hydration/detail paths clone only the stored blob:
    - `src-tauri/src/run_artifacts.rs:1775`
    - `src-tauri/src/run_artifacts.rs:1803`
  - report export copies only stored metadata:
    - `src-tauri/src/commands_reports.rs:202-207`
  - frontend history rendering trusts the hydrated blob:
    - `web/src/services/historyService.ts:538-552`
    - `web/src/services/historyService.ts:1355-1387`
- Why it matters:
  - the same stale or pre-schema artifact can be:
    - rederived as `core17` or `trade22` inside `commands_meta`
    - but still show no metric surface, or stale metric surface, in saved-run detail/history/report paths
- Review judgment:
  - this is a real backward-compatibility inconsistency between meta analysis and artifact presentation
- Smallest justified fix:
  - normalize `optimization_metric_surface` during hydration/detail/report build when the stored blob is absent or stale
  - keep save-time backfill, but do not rely on it as the only compatibility repair mechanism

## API And Naming

### 1. The compatibility contract is stringly typed and duplicated across strategy names, route names, and route diagnostics

- Evidence:
  - strategy strings are matched inline: `208-247`
  - route classification is based on substring matching:
    - `102-111`
    - `194-206`
  - exact GPU metric counts are parsed from a diagnostic string:
    - `168-179`
- Why it matters:
  - adding a strategy, renaming a route label, or changing a diagnostic message in app-core can silently age this compatibility layer
  - the file is not the runtime source of truth, yet it embeds a second policy table
- Smallest justified fix:
  - document the fallback contract as intentionally derived-from-runtime rather than canonical
  - where practical, prefer consuming explicit persisted/runtime `metric_surface_kind` over route-name heuristics

### 2. The `mean` field in the sidecar row is reconstructed, not carried directly from the metrics struct

- Evidence:
  - `metrics_mean` is defined as `metrics.sharpe * metrics.volatility`: `278-280`
  - the row name published to consumers is `mean`: `11`
- Why it matters:
  - the derivation is reasonable today, but the field meaning is not documented locally
  - if app-core ever changes the metric interpretation without a direct `mean` field, this sidecar can drift silently
- Smallest justified fix:
  - add a short comment documenting that `mean` is reconstructed from `sharpe * volatility` because `Metrics` does not carry a separate `mean` field

## Error Handling

### 1. Malformed metric rows or sidecar serialization failures are silently swallowed

- Evidence:
  - row parsing suppresses all `serde_json::from_value` errors:
    - `410-413`
  - sidecar serialization suppresses all `serde_json::to_value` errors:
    - `461-463`
  - save-time insertion simply skips the sidecar when builder returns `None`:
    - `src-tauri/src/run_artifacts.rs:1131-1134`
- Why it matters:
  - malformed artifacts degrade into absent metric-surface rows or absent sidecar metadata with no diagnostic breadcrumb
  - because this file exists largely for compatibility repair, silent loss of repairability is especially hard to notice
- Smallest justified fix:
  - keep the non-fatal fallback behavior
  - add a log or structured warning path when parsing/serialization fails so stale or damaged artifacts can be diagnosed

## Module Boundaries And Technical Debt

### 1. Current-schema validation is too weak for a contract-oriented compatibility module

- Evidence:
  - `has_current_metric_surface_metadata` checks only `schema_version == 2`: `368-373`
  - `artifact_metric_surface_kind` then trusts the blob enough to avoid incrementing `missing_metadata`, even if `surface` is malformed, and falls back silently only for classification:
    - `375-384`
  - summary accounting increments `missing_metadata` only when the schema version check fails:
    - `391-404`
- Why it matters:
  - an artifact can present current-version metadata while still being semantically malformed:
    - missing or invalid `surface`
    - mismatched `fields`
    - inconsistent row lengths
  - `commands_meta` warnings can therefore overstate metadata health
- Smallest justified fix:
  - strengthen "current metadata" validation to include at least:
    - valid `surface`
    - expected field count for non-legacy surfaces when `fields` is present
  - optionally treat malformed current-schema blobs as `missing_metadata` for warning purposes

### 2. The sidecar carries `best_metrics_row` and `top_metric_rows`, but the rest of the repo currently appears to care mostly about `surface` and `fields`

- Evidence:
  - this file defines and fills those row payloads: `99-107`, `424-457`
  - hydration/detail/report/frontend references found in this review pass were to `optimization_metric_surface` generally, or to `surface`/`fields` specifically:
    - `src-tauri/src/run_artifacts.rs:1775`, `1803`
    - `src-tauri/src/commands_reports.rs:207`, `3612-3620`
    - `web/src/services/historyService.ts:538-552`, `1355-1387`
- Why it matters:
  - these rows are useful future-proofing hooks, but they currently add schema surface and maintenance burden with limited proven consumption
- Review judgment:
  - this is useful debt, not cleanup fodder
  - do not remove these payloads casually
- Smallest justified fix:
  - document the intended future/reporting use of row payloads, or add downstream tests that exercise them if they are already relied upon by product plans

## Docs And Examples

### 1. The file needs a short module-level note explaining when fallback inference is authoritative and when it is only a compatibility best-effort

- Missing documentation topics:
  - explicit persisted `optimization_metric_surface` with current schema should be preferred when valid
  - runtime `metric_surface_kind` is the real source of truth
  - fallback strategy/route heuristics exist only for backward compatibility with older artifacts

### 2. The sidecar schema contract is not documented near the struct definition

- Missing documentation topics:
  - expected field order for `core17` and `trade22`
  - `best_metrics_row` and `top_metric_rows` omission semantics
  - how stale schema versions are treated during rederivation

## Tests And Missing Verification

### 1. File-local compatibility coverage is strong but biased toward exact-route and save-time paths

- Verified in this review pass:
  - `cargo test -p ta_desktop_demo_app metric_surface -- --nocapture`
    - passed `42` relevant tests, including:
      - stale-schema rederivation: `1443-1495`
      - exact-route `core17` / `trade22` inference
      - canonical field-order preservation
      - meta-summary counts
      - report and run-artifact threading tests routed through the same filter output
- Existing strengths:
  - direct field-order assertions: `505-565`, `1230-1290`
  - stale-schema fallback coverage: `1443-1495`
  - pareto fallback coverage for `CompositeMo`: `1359-1388`

### 2. Important compatibility gaps remain

- Missing tests found in this review pass:
  - no fallback-compatibility test for `RegimeHybrid` on non-exact routes even though app-core emits `trade22` and `core17` there
  - no hydration/detail normalization test for artifacts missing `optimization_metric_surface`
  - no report-export normalization test for stale or absent sidecar metadata
  - no test that malformed current-schema blobs are surfaced as degraded metadata quality

### 3. If this file changes, verification needs to cover both backend and saved-run UX

- Exact local verification required if changed:
  - `cargo test -p ta_desktop_demo_app metric_surface -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
  - `cargo test -p ta_desktop_demo_app run_artifacts -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`
  - `npm --prefix ./web run test -- src/services/historyService.test.ts src/stores/workflowArtifactsStore.test.ts src/services/validationService.test.ts`

## Performance, Allocation, And CUDA Concerns

### 1. There is no convincing case for micro-optimizing this file today

- Evidence:
  - this module runs on artifact save/read/report/meta paths, not the optimization hot loop
  - current work is small JSON traversal and vector projection
- Review judgment:
  - performance work here should remain subordinate to correctness and compatibility

### 2. The only meaningful performance question is payload size, not local CPU time

- Evidence:
  - the sidecar stores projected rows in addition to `surface` and `fields`: `99-107`, `424-457`
- Safe-now guidance:
  - keep the payload unless there is evidence of artifact-size or hydration-cost pressure
- Measure-first guidance:
  - if saved-run payload size becomes a problem, measure the cost/benefit of carrying `top_metric_rows` before trimming anything
  - do not remove row payloads as "cleanup" without confirming that reports, future review tools, and validation flows do not depend on them

## Code-Quality Improvement Register

- Safe now:
  - align fallback strategy coverage with the runtime source of truth for non-exact surfaces
  - strengthen current-schema validation beyond `schema_version`
  - add non-fatal diagnostics on parse/serialization failure
  - document `mean` reconstruction and fallback-authority limits
- Defer unless there is a wider artifact-schema pass:
  - replacing ad hoc JSON-pointer traversal with a typed intermediate artifact view
  - redesigning the persisted sidecar shape

## Technical-Debt Reduction Register

- Safe to reduce now:
  - backward-compat drift between fallback tables and runtime truth
  - weak malformed-metadata detection
  - absent diagnostics for sidecar build failure
- Defer:
  - broader unification of app-core metric-surface policy and desktop artifact compatibility logic
  - any schema rewrite that would require migration of already-saved artifacts
- Useful debt that should not be "cleaned up" casually:
  - row payload preservation for future reporting/review uses
  - fallback compatibility logic for stale artifacts, even if it remains slightly verbose

## Performance Opportunity Register

- Safe structural improvement:
  - normalize stale or missing sidecar metadata during hydration/detail/report build so downstream views do not diverge from meta analysis
- Benchmark-sensitive or semantics-sensitive:
  - trimming sidecar rows for payload-size reasons
  - reworking the persisted schema or changing the definition/order of published metric fields

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - do not narrow supported saved-artifact compatibility as part of cleanup
  - do not remove `best_metrics_row` / `top_metric_rows` merely because current consumers are sparse
- Regression risk if changed carelessly:
  - saved-run history and detail views can diverge from meta warnings
  - old artifacts can be reclassified differently
  - report exports can lose or misstate the optimization metric surface
- Safe change posture:
  - treat this file as a compatibility layer first
  - prefer additive hardening and better normalization over schema churn
