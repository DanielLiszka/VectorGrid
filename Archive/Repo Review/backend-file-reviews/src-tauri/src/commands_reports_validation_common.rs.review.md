# Review Packet: `src-tauri/src/commands_reports_validation_common.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_validation_common.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Functionally rich, but too large and semantically overloaded to count as stable common export infrastructure without stronger contract tests and some decomposition planning`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Semantics-sensitive`, `Export-contract-sensitive`, `Benchmark-note-sensitive`, `Support-module`

## What This File Owns

`commands_reports_validation_common.rs` is the canonical transformer from raw validation JSON into the backend export model used by validation report outputs.

This file owns:

- the export-side data model:
  - `ExportRunVerdict`: `src-tauri/src/commands_reports_validation_common.rs:11-28`
  - `ExportSpaSummary`: `src-tauri/src/commands_reports_validation_common.rs:31-40`
  - `ExportReoptSummary`: `src-tauri/src/commands_reports_validation_common.rs:43-60`
  - `ExportCscvSummary`: `src-tauri/src/commands_reports_validation_common.rs:63-73`
  - `ExportCrossSummary`: `src-tauri/src/commands_reports_validation_common.rs:76-82`
  - `ExportRankingStabilitySummary`: `src-tauri/src/commands_reports_validation_common.rs:85-95`
  - `ExportRegimeBucketRow`: `src-tauri/src/commands_reports_validation_common.rs:98-111`
  - `ExportCandidateRow`: `src-tauri/src/commands_reports_validation_common.rs:114-181`
  - `ValidationExportModel`: `src-tauri/src/commands_reports_validation_common.rs:184-226`
- raw `serde_json::Value` parsing and normalization helpers:
  - `opt_f64`, `opt_u64`, `normalize_text`, `join_string_array`, `scalar_text`, `summarize_object`, etc.
- candidate-row extraction and approval/compatibility logic:
  - `legacy_shortlist_flags`: `src-tauri/src/commands_reports_validation_common.rs:554-633`
  - `final_approved`: `src-tauri/src/commands_reports_validation_common.rs:635-648`
  - `approval_label_and_reason`: `src-tauri/src/commands_reports_validation_common.rs:650-696`
  - `build_rows`: `src-tauri/src/commands_reports_validation_common.rs:2311-2518`
- derived export analyses:
  - plateau analysis: `src-tauri/src/commands_reports_validation_common.rs:1019-1316`
  - ranking stability: `src-tauri/src/commands_reports_validation_common.rs:1320-1515`
- expected-route / cross-ticker / module-summary narration:
  - `build_expected_execution_summary`: `src-tauri/src/commands_reports_validation_common.rs:1927-2245`
  - related route-description helpers above it
- end-to-end model assembly from report or validation result:
  - `build_model_from_report`: `src-tauri/src/commands_reports_validation_common.rs:2581-2635`
  - `build_model_from_validation_res`: `src-tauri/src/commands_reports_validation_common.rs:2637-2647`
  - `build_model_from_validation_res_with_context`: `src-tauri/src/commands_reports_validation_common.rs:2649-2725`
- a narrow test set focused mainly on expected route-summary wording:
  - `src-tauri/src/commands_reports_validation_common.rs:2733-2923`

This file does not own:

- workbook rendering
  - `src-tauri/src/commands_reports_validation_xlsx.rs`
- CSV serialization
  - `src-tauri/src/commands_reports_csv.rs`
- primary validation execution
  - the result JSON is produced elsewhere in backend/runtime code

## Classification

- Public surface: no
  - this is an internal support module
- Hot path: no
  - it runs when building export models, not in optimization or validation execution loops
- Runtime-critical: moderate to high
  - validation report XLSX and CSV depend on it for schema interpretation and human-readable summaries
- CUDA/runtime-critical: no
- Strategy-local: no
  - it spans multiple strategy families and route policies
- Low-risk support code only: no
  - it materially shapes user-visible exported content and route-policy explanations

## Direct Interactions

### Backend interaction map

- workbook consumer:
  - `src-tauri/src/commands_reports_validation_xlsx.rs:2`
  - `src-tauri/src/commands_reports_validation_xlsx.rs:2297`
- CSV consumer:
  - `src-tauri/src/commands_reports_csv.rs:1`
  - `src-tauri/src/commands_reports_csv.rs:21-23`
- wrapper/report assembly:
  - via `src-tauri/src/commands_reports.rs`
- raw validation/meta source context that this file interprets:
  - `src-tauri/src/commands_meta.rs` and the validation-producing backend paths indirectly feed the JSON this file parses

### Frontend contract anchors

- validation export flows depend on the workbook/CSV generated from this model:
  - `web/src/services/validationService.ts`
- frontend also carries overlapping regime/route/category summary logic instead of consuming this model directly:
  - regime summary helpers around `web/src/services/validationService.ts:1212-1269`
  - execution-category and route-note logic around `web/src/services/validationService.ts:1275-1388`
- command-shape coverage:
  - `web/src/tauri/commands.test.ts`
- validation export/store flows:
  - `web/src/services/validationService.test.ts`
  - `web/src/stores/validationStore.actions.test.ts`

### Test anchors

- direct route-summary tests:
  - `src-tauri/src/commands_reports_validation_common.rs:2735-2923`
- indirect workbook validation tests:
  - `src-tauri/src/commands_reports_validation_xlsx.rs:2607-2685`
- indirect report/CSV tests:
  - `src-tauri/src/commands_reports.rs`
  - `src-tauri/src/commands_reports_csv.rs`

## Standards And Research Log

This packet was written after a targeted external review focused on JSON-pointer semantics, raw-JSON access patterns, floating-point ordering behavior, and common Rust API contract guidance.

### Sources consulted

1. `serde_json::Value`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
   - Why it was consulted:
     - this file depends heavily on `Value`, `pointer`, `as_*`, and object traversal rather than typed DTOs
   - Standard informed:
     - raw `Value` access is flexible but weakly typed, so missing-field behavior must be intentionally documented and tested

2. RFC 6901 JSON Pointer
   - Source: <https://datatracker.ietf.org/doc/html/rfc6901>
   - Why it was consulted:
     - this file uses a large number of hard-coded JSON pointer strings
   - Standard informed:
     - JSON Pointer resolution failures are explicit error conditions at the spec level; applications must deliberately define how those failures degrade

3. Rust `f64::total_cmp`
   - Source: <https://doc.rust-lang.org/std/primitive.f64.html>
   - Why it was consulted:
     - this file sorts floating-point values for medians, rankings, and derived summaries
   - Standard informed:
     - `total_cmp` is the correct deterministic ordering primitive for `f64` sorting in the presence of edge floating-point values

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file is effectively a shared internal API with non-trivial failure and contract behavior
   - Standard informed:
     - internal shared helpers still need explicit failure semantics, naming clarity, and representative verification

## Findings

## Correctness And Safety

### 1. Fallback regime-support labels do not align with what the XLSX renderer and frontend expect

- Evidence:
  - backend fallback derivation can emit `trend_ok` and `mixed`:
    - `src-tauri/src/commands_reports_validation_common.rs:289-301`
  - XLSX display mapping only recognizes:
    - `broad`, `trend_only`, `range_only`, `vol_selective`, `weak`
    - `src-tauri/src/commands_reports_validation_xlsx.rs:169-177`
  - frontend fallback derivation uses:
    - `broad`, `trend_only`, `range_only`, `vol_selective`, `weak`
    - `web/src/services/validationService.ts:1212-1224`
  - current integration test already tolerates the mismatch by accepting `Broad` or `Trend OK` or `Mixed`:
    - `src-tauri/tests/validation_xlsx_report_tests.rs:413-425`
- Why it matters:
  - when artifact-level `support_label` is absent, this common layer can generate labels that the renderer treats as `not run`
  - backend and frontend summaries can disagree for the same artifact
- Smallest justified fix:
  - align fallback label generation to the same canonical label set used by the renderer and frontend
  - add a direct test that proves the fallback labels survive end to end
- Sensitivity:
  - `Safe now`

### 2. Missing or mismatched schema fields frequently collapse into plausible-looking zeroes or blanks instead of surfacing drift

- Evidence:
  - only `validation_res.candidates` is a hard error in `build_rows`: `src-tauri/src/commands_reports_validation_common.rs:2311-2315`
  - many core metrics default missing values to `0.0` or empty strings:
    - `pnl`: `2352`
    - `sharpe`: `2353`
    - `robust_score`: `2411`
    - `max_dd`: `2422`
    - `trades`: `2423`
    - fallback string summaries across `2649-2725`
- Research relevance:
  - RFC 6901 treats pointer-resolution failures as explicit error conditions; this file intentionally suppresses many of them
- Why it matters:
  - artifact/schema drift can become a “valid-looking” export row instead of an actionable error or explicit missing-data marker
- Review judgment:
  - this is an objective maintainability/correctness risk, though not every missing field should become fatal because compatibility mode is a real supported behavior
- Smallest justified fix:
  - identify a small set of critical fields that should produce explicit warnings when missing in non-compatibility mode
  - preserve permissive fallback behavior for legacy artifacts where truly necessary

### 3. Plateau analysis silently disappears on serialization/parse failures because parameters are round-tripped through JSON strings

- Evidence:
  - `build_rows` serializes params into `params_json` with fallback `{}`:
    - `src-tauri/src/commands_reports_validation_common.rs:2336-2337`
  - `plateau_stats_for_row` reparses `row.params_json` and uses `.ok()?`:
    - `src-tauri/src/commands_reports_validation_common.rs:1219-1221`
- Why it matters:
  - a failure in either direction suppresses plateau enrichment instead of surfacing a detectable report-generation problem
  - this is unnecessary self-inflicted fragility because the data started as structured JSON
- Smallest justified fix:
  - keep a structured params representation alongside `params_json`, or defer JSON-string generation to the rendering layer

### 4. Route-note strings embed benchmark assertions that can go stale independently of the code

- Evidence:
  - benchmark-claim strings are hard-coded in `describe_composite_route_detail`:
    - `src-tauri/src/commands_reports_validation_common.rs:1824-1864`
- Why it matters:
  - these are user-visible claims about CPU/GPU relative performance and route posture
  - they are temporally unstable by nature
- Review judgment:
  - this is not a runtime bug, but it is a correctness risk in exported narrative
- Smallest justified fix:
  - require a benchmark-evidence refresh before changing these strings
  - document that these strings are benchmark-sensitive policy text, not timeless facts

## API And Naming

### 1. `commands_reports_validation_common.rs` is far broader than a typical “common” helper module

- Evidence:
  - the file is `2923` lines long and combines:
    - DTO definitions
    - raw JSON parsing
    - approval policy
    - plateau analytics
    - ranking stability
    - expected-route narration
    - shortlist selection
    - compatibility behavior
- Why it matters:
  - the module boundary no longer communicates what must be changed together
  - the current name hides that this file is effectively the export policy engine
- Review judgment:
  - this is real structural debt

### 2. Several helpers are named generically even though they encode product policy

- Examples:
  - `build_expected_execution_summary`: `1927-2245`
  - `shortlist_rows`: `2548-2579`
  - `approval_label_and_reason`: `650-696`
- Why it matters:
  - these are not generic transforms; they encode workflow-specific policy and exported wording
- Smallest justified fix:
  - document them as policy helpers even if names stay unchanged

## Error Handling

### 1. The file has a very narrow hard-error boundary and a very broad silent-fallback boundary

- Evidence:
  - hard failure on missing candidates: `2311-2315`
  - broad `unwrap_or`, `unwrap_or_else`, `ok()?`, and empty-summary defaults throughout:
    - `2336-2343`
    - `2594-2628`
    - `2693-2706`
- Why it matters:
  - the model builder often cannot distinguish “field intentionally absent”, “legacy artifact”, and “schema unexpectedly changed”
- Smallest justified fix:
  - introduce explicit warning accumulation for missing critical sections rather than turning every failure into a default value

### 2. Compatibility mode is useful but under-signaled in the common layer itself

- Evidence:
  - compatibility mode is inferred broadly at `2329`
  - it changes approval behavior through `approval_label_and_reason`: `650-696`
  - workbook consumers later surface it:
    - `src-tauri/src/commands_reports_validation_xlsx.rs:906-924`
- Why it matters:
  - the common layer is already making compatibility-policy decisions that affect approval labels and shortlist composition
- Smallest justified fix:
  - document the exact compatibility-mode trigger and its consequences here, not only in the workbook layer

## Module Boundaries And Technical Debt

### 1. This file mixes parsing, analytics, export DTO assembly, and presentation-policy narration

- Evidence:
  - raw parsers live next to presentation-oriented route notes and warning strings
  - plateau and ranking analytics live next to output-label logic
- Why it matters:
  - changes to one behavior require a reviewer to reason about too many unrelated concerns
- Review judgment:
  - the technically correct long-term fix is decomposition, but that should be a planned follow-up, not casual churn

### 2. The model builder encodes a JSON-string roundtrip that should not exist in shared core export logic

- Evidence:
  - serialize at `2336-2337`
  - parse back at `1219`
- Why it matters:
  - duplicated transformations increase drift and hide failure modes
- Smallest justified fix:
  - store structured params once and render string form at the final export boundary

### 3. Product-policy wording is embedded in backend code and tightly coupled to report outputs

- Evidence:
  - large blocks of user-facing wording in `build_expected_execution_summary` and `describe_composite_route_detail`
  - direct tests mainly pin wording rather than parsed-model structure: `2735-2923`
- Why it matters:
  - wording changes become semantics-sensitive report changes
  - backend policy drift can occur without broad detection if only a few phrases are tested

## Docs And Examples

### 1. The module lacks a contract note for its input schema and degradation rules

- Missing documentation topics:
  - which JSON sections are mandatory vs optional
  - which fields default to zero/blank
  - compatibility-mode trigger and effect
  - shortlist precedence order
  - which route-note strings are benchmark-sensitive
- Why it matters:
  - this file is already the de facto specification for export-model assembly

## Tests And Missing Verification

### 1. Direct tests are too narrow for the amount of policy encoded here

- Direct coverage currently focuses on eight expected-route summary cases:
  - `src-tauri/src/commands_reports_validation_common.rs:2735-2923`
- Important gaps:
  - `build_rows` field extraction and defaulting behavior
  - compatibility-mode activation and approval relabeling
  - `run_level_rejection_reason`
  - plateau analysis and plateau-source selection
  - ranking stability calculations
  - missing-`validation.req` behavior
  - JSON-roundtrip failure behavior around `params_json`
- Why it matters:
  - most of the file’s complexity is presently unguarded by direct tests

### 2. Indirect coverage exists, but it does not replace direct contract tests

- Indirect consumers:
  - validation workbook tests
  - shortlist CSV tests
  - frontend validation export tests
- Limitation:
  - those tests mostly confirm downstream rendering and orchestration, not the common model builder’s edge semantics

## Performance, Allocation, And CUDA Concerns

### 1. This is not the right place to optimize for throughput first

- Evidence:
  - main work is JSON traversal, string formatting, cloning, and export-only analytics
  - repeated cloning happens for approved/shortlist rows and plateau/ranking enrichment:
    - `1299-1316`
    - `2674-2725`
- Review judgment:
  - acceptable for export-time work
  - correctness, documentation, and test coverage are higher priority than micro-optimizing these allocations

### 2. The benchmark-sensitive concern here is narrative accuracy, not raw runtime

- The benchmark-sensitive parts are the hard-coded route-detail claims, not the execution cost of the model builder itself.

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports.rs`
- `web/src/services/validationService.ts`

### Downstream consumers needing verification

- validation workbook sheets and summary text
- shortlist CSV content and ordering
- frontend validation export flows
- any workflow that relies on approval labels, compatibility mode, or expected route summaries

## Code-Quality Improvement Register

### Safe now

- add direct tests for:
  - `build_rows`
  - compatibility mode
  - missing `validation.req`
  - plateau enrichment
  - ranking stability
- document the JSON schema/degradation contract
- add warning accumulation for missing critical sections in non-compat mode
- remove the params JSON roundtrip from plateau analysis

### Coordination required

- changing approval-label rules
- changing shortlist precedence
- changing route-summary wording
- changing benchmark-note text
- changing how missing numeric fields are represented in the export model

### Defer until later pass

- splitting this module into parser / analytics / policy-summary / DTO layers
- broader typed-DTO migration away from raw `serde_json::Value`

## Technical-Debt Reduction Register

### Debt safe to reduce now

- missing direct tests
- missing contract documentation
- params JSON roundtrip
- missing explicit warnings for critical-field absence

### Debt that spans multiple files

- shared export wording policy across backend and frontend expectations
- route-summary and benchmark-note ownership
- typed schema vs permissive raw JSON across report/export surfaces

### Debt that should not be “cleaned up” casually

- compatibility-mode behavior for legacy artifacts
- shortlist precedence order
- strategy-specific expected-route wording without checking neighboring workbook/CSV/UX behavior

## Performance Opportunity Register

- Safe structural opportunities:
  - remove the serialize-then-parse params roundtrip
  - reduce some clone/rebuild passes if the file is ever split
- Benchmark-sensitive opportunities:
  - none worth prioritizing for runtime speed
- Not the right place to chase performance:
  - yes; report-contract integrity matters more here

## Observability And Diagnostics Gaps

- missing critical JSON sections often degrade silently instead of producing explicit warnings
- plateau analysis can disappear without a visible explanation if params roundtrip fails
- route-note benchmark claims have no embedded provenance or freshness marker

## Open Questions And Assumptions

- assumption:
  - permissive fallback behavior is partly intentional to support older artifacts and compatibility mode
- open question:
  - which fields are important enough to warn on when missing, even in permissive export mode
- open question:
  - should route-detail benchmark claims move to a separately versioned policy source instead of remaining hard-coded strings

## Regression Risk And Feature-Preservation Constraints

- Do not change approval-label or shortlist rules casually; workbook and CSV outputs depend on them.
- Do not change route-summary wording casually; direct tests already pin some phrases and exported report meaning will change.
- Treat the benchmark-note strings as benchmark-sensitive. Updating them without fresh evidence risks making the export less accurate, not more.
- Preserve compatibility-mode support unless a deliberate migration plan exists for older artifacts.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_reports_validation_common -- --nocapture`
2. `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`
3. `cargo test -p ta_desktop_demo_app commands_reports_validation_xlsx -- --nocapture`
4. `cargo test -p ta_desktop_demo_app commands_reports_csv -- --nocapture`
5. `npm --prefix ./web run test -- src/services/validationService.test.ts src/tauri/commands.test.ts src/stores/validationStore.actions.test.ts`
6. Manual validation-export spot-check if route summaries, shortlist logic, or compatibility behavior changes:
   - generate a validation workbook
   - export shortlist CSV
   - verify approval labels, expected route summaries, and shortlist ordering

## Recommended Action

`Larger refactor but defer`

The immediate value is in bounded hardening:

- add missing direct tests
- document schema/degradation rules
- stop silently losing plateau analysis through the params JSON roundtrip
- add explicit warnings for critical-field absence

But the deeper issue is structural: this file is currently a parser, analytics engine, and export-policy narrator all at once. That larger split is justified, but it should be a deliberate follow-up rather than opportunistic churn during routine maintenance.
