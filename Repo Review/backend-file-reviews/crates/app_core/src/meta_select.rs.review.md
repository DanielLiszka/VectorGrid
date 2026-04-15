# Review Packet: `crates/app_core/src/meta_select.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/meta_select.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally useful and backed by good local tests, but not fully public-review ready because the public request/result surface advertises holdout-related controls that this file does not itself apply, the train-selection stage and holdout-augmentation stage are split across files without strong contract documentation, and the benchmarked grouping/canonicalization path has benchmark-sensitive allocation behavior that should not be changed casually`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Workflow-critical`, `Benchmark-sensitive`, `Semantics-sensitive`, `Support code`

## What This File Owns

`meta_select.rs` owns the train-side meta-selection stage for selecting, grouping, scoring, and summarizing validation artifacts before optional holdout augmentation is applied elsewhere.

It owns:

- public request/config surface for meta selection:
  - `MetaScoreWeights`: `crates/app_core/src/meta_select.rs:16-80`
  - `MetaGroupingMode`: `101-116`
  - `MetaHoldoutMode`: `118-144`
  - `MetaHoldoutValidationConfig`: `150-161`
  - `MetaClusterConfig`: `170-189`
  - `MetaSelectionRequest`: `192-239`
- public result surface and holdout/result row types:
  - `HoldoutAggregate`: `242-250`
  - `HoldoutSummary`: `252-256`
  - `MetaCandidateAggregate`: `258-294`
  - `HoldoutDatasetRow`: `297-324`
  - `MetaSelectionResult`: `327-338`
- evidence-flag policy:
  - `compute_meta_group_evidence_flags`: `367-411`
  - `refresh_meta_group_evidence_flags`: `413-415`
- the main train-artifact scan, shortlist, grouping, scoring, and summary path:
  - `meta_select_from_artifacts`: `426-786`
- progress emission for the selector:
  - `emit_progress`: `788-797`
- artifact filtering and normalization helpers:
  - `artifact_matches_request`: `828-880`
  - MA/context extraction helpers: `883-1002`
- grouping/canonicalization and clustered-key generation:
  - `meta_group_key`: `1004-1018`
  - `meta_group_key_with_context`: `1020-1044`
  - `meta_group_key_for_request`: `1046-1062`
  - `meta_group_key_for_request_with_context`: `1064-1086`
  - value quantization helpers: `1088-1166`
  - `canonical_json`: `1435-1476`
- shortlist and representative selection logic:
  - `shortlist_candidates`: `1168-1313`
  - validation-verdict compatibility helper:
    - `artifact_backend_validation_approved`: `1320-1360`
  - `pick_representatives`: `1386-1433`
- file-local tests:
  - `1478-2114`

This file does not own:

- holdout execution and holdout-result augmentation:
  - `src-tauri/src/commands_meta.rs:2121`
  - `2556-2641`
  - `2647-3102`
- report rendering of meta results:
  - `src-tauri/src/commands_reports.rs:46`
  - `src-tauri/src/commands_reports_meta_xlsx.rs:7`
- generic progress payload transport:
  - `crates/app_core/src/progress.rs:20-42`

## Classification

- Public surface: yes
  - the public API is re-exported from `crates/app_core/src/lib.rs:183-186`
- Hot path: yes, but as a workflow path rather than a numeric inner loop
  - this file scans artifacts, groups candidates, sorts rows, and is benchmarked by `crates/app_core/benches/meta_select_workflow.rs`
- CUDA/runtime-critical: no
  - it is not a device/runtime path
- Strategy-local: partially
  - it is generic meta-selection infrastructure, but it is currently wired into Double-MA-family and adjacent meta workflows
- Low-risk support code: no
  - it is support code with meaningful public-contract and workflow-selection consequences

## Direct Interactions

### Backend interaction map

- public re-export surface:
  - `crates/app_core/src/lib.rs:183-186`
- Tauri command boundary imports:
  - `src-tauri/src/commands.rs:161`
  - `src-tauri/src/commands_meta.rs:14`
- train-side meta selection entrypoint:
  - `src-tauri/src/commands_meta.rs:2121`
- holdout augmentation and evidence refresh:
  - `src-tauri/src/commands_meta.rs:2556-2641`
  - `2647-3102`
  - `2594`
- report/export consumers:
  - `src-tauri/src/commands_reports.rs:46`
  - `src-tauri/src/commands_reports_meta_xlsx.rs:7`
- validation/workflow integration:
  - `crates/app_core/src/double_ma_validate.rs:4065-4066`
  - `9303`
- benchmark coverage:
  - `crates/app_core/benches/meta_select_workflow.rs:6-7`
  - `167-252`
  - `crates/app_core/benches/e2e_workflow_ml.rs:9-16`
  - `451-474`

### Downstream verification impact if changed

- changing request/result fields affects:
  - `commands_meta.rs`
  - `commands_reports.rs`
  - `commands_reports_meta_xlsx.rs`
  - `src-tauri/src/commands.rs`
- changing grouping-key or canonicalization semantics affects:
  - stored/reported meta-group keys
  - ML/meta workflow comparisons in benches and regression harnesses
- changing shortlist or verdict filtering affects:
  - train candidate inclusion
  - downstream holdout selection and report contents

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API contracts, Serde request/result semantics, canonical JSON expectations, stable sorting of floating-point-derived summaries, and collection behavior relevant to the benchmarked grouping path.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public request/result types and helper functions that are consumed outside the module
   - Standard informed:
     - public surfaces should be intentional, documented, and avoid misleading knobs or contract drift

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the file exports public types and helpers with non-obvious two-phase semantics
   - Standard informed:
     - public APIs should explain behavior, stage ownership, and compatibility assumptions, not just compile

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - `MetaSelectionRequest` relies heavily on `#[serde(default)]` and result shapes use optional/skip-serializing semantics
   - Standard informed:
     - defaults and omitted fields are part of the wire contract and should not silently become no-op controls without documentation

4. RFC 8785, JSON Canonicalization Scheme
   - Source: <https://www.rfc-editor.org/rfc/rfc8785>
   - Why it was consulted:
     - this file uses `canonical_json` to form stable grouping and deduplication keys
   - Standard informed:
     - canonicalization semantics are contract-sensitive; if the repo keeps a local canonicalization helper, its determinism should be preserved and changes should be treated as compatibility-sensitive

5. Rust slice sorting documentation
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by>
   - Why it was consulted:
     - the file sorts representative params and train rows and uses floating-point ordering
   - Standard informed:
     - stable sorting and `total_cmp`-style total ordering matter when ranking/group summaries depend on reproducible ordering

6. Rust `HashMap` documentation
   - Source: <https://doc.rust-lang.org/std/collections/struct.HashMap.html>
   - Why it was consulted:
     - the selector builds grouped accumulators and dedup structures across artifact scans
   - Standard informed:
     - collection-heavy workflow code can be improved locally, but allocation/layout changes should be justified by profiling when the file is already benchmarked

## Findings

## Correctness And Safety

### 1. The public request/result surface advertises holdout behavior that this file does not itself implement

- Evidence:
  - `MetaSelectionRequest` exposes holdout-oriented controls:
    - `holdout_tag`: `crates/app_core/src/meta_select.rs:199`
    - `eval_holdout`: `200-201`
    - `holdout_eval_top_n`: `206-209`
    - `holdout_eval_explore_n`: `210-213`
    - `holdout_eval_rank_by`: `214-216`
    - `holdout_mode`: `218`
    - `holdout_validation`: `219-220`
  - result types expose holdout payloads:
    - `MetaCandidateAggregate.holdout`: `287`
    - `MetaCandidateAggregate.holdout_rows`: `293`
    - `MetaSelectionResult.holdout`: `337`
  - the train-side selector always emits `holdout: None`:
    - group-level: `714`
    - result-level: `784`
  - holdout augmentation happens later in `commands_meta.rs` after `meta_select_from_artifacts` returns:
    - `src-tauri/src/commands_meta.rs:2121`
    - `2556-2641`
    - `2647-3102`
- Why it matters:
  - direct callers of `meta_select_from_artifacts` can reasonably assume the holdout-related request knobs affect this function, because they live in the same public request type and the result type already includes holdout fields
  - that is contract drift rather than a crash bug, but it is still an objective correctness/readiness issue for a public API
- Smallest justified fix:
  - document explicitly that this file performs train-side aggregation only and that holdout population is a second-stage augmentation performed by `commands_meta.rs`
  - do not remove or narrow the fields casually, because downstream consumers already depend on the shared request/result shapes
  - if the project wants stronger contracts later, split stage-specific request/result types in a coordinated pass
- Review judgment:
  - medium-high contract-correctness issue
  - semantics-sensitive because the current shared surface is already consumed downstream

## API And Naming

### 1. `artifact_matches_request` ignores `holdout_tag` and other holdout-selection knobs because the request type conflates train and holdout phases

- Evidence:
  - the matcher only checks artifact kind, strategy kind, timeframe, and `required_tag`:
    - `crates/app_core/src/meta_select.rs:828-880`
  - `holdout_tag` exists on the request surface:
    - `199`
  - search results show holdout-selection behavior is implemented in `commands_meta.rs`, not in this matcher:
    - `src-tauri/src/commands_meta.rs:1987-1988`
    - `2139`
    - `2143-2264`
- Why it matters:
  - a shared request type with stage-specific no-op fields is hard to review, hard to reason about, and easy to misuse
  - it also makes it unclear which layer owns artifact tagging/filtering semantics
- Smallest justified fix:
  - document which request fields are train-stage inputs versus holdout-stage inputs
  - if a later pass splits types, keep backward compatibility explicit rather than silently repurposing fields
- Review judgment:
  - medium API/boundary issue

### 2. The generic meta selector reports progress through a Double-MA-named helper

- Evidence:
  - `emit_progress` is a generic wrapper:
    - `crates/app_core/src/meta_select.rs:788-797`
  - it delegates to `crate::progress::emit_double_ma_progress`:
    - `795`
  - `commands_meta.rs` also uses the same Double-MA-named helper for meta phases such as `meta_export`, `meta_save`, and `meta_holdout`:
    - `src-tauri/src/commands_meta.rs:2287-2309`
    - `3084-3087`
- Why it matters:
  - this is not a runtime correctness bug, but it is a naming/ownership leak across subsystems
  - it makes progress semantics look more strategy-specific than they actually are
- Smallest justified fix:
  - add a generic alias/helper name in `progress.rs` and migrate call sites without changing the payload format
  - keep the actual payload shape stable unless UI consumers are audited
- Review judgment:
  - medium-low API/naming debt

## Error Handling

### 1. Artifact mismatches and schema gaps are mostly handled by silent exclusion or default values

- Evidence:
  - unmatched artifacts are silently skipped:
    - `crates/app_core/src/meta_select.rs:445-447`
  - missing candidate arrays are silently skipped:
    - `449-455`
  - empty candidate arrays are silently skipped:
    - `456-458`
  - shortlist filtering and CSCV gating default missing numeric inputs to `0.0` or `NEG_INFINITY`:
    - `474-489`
    - `1191-1207`
    - `1238-1256`
    - `1284-1300`
  - output warnings are aggregate summaries rather than per-reason diagnostics:
    - `730-771`
- Why it matters:
  - the selector is deliberately tolerant of legacy or mixed artifacts, which is useful
  - but when schema drift or malformed artifacts occur, it is difficult to tell whether artifacts were skipped for mismatch, missing validation structure, or failing thresholds
- Smallest justified fix:
  - keep tolerant behavior
  - add additive counters or optional debug diagnostics for the main skip reasons
  - avoid turning this into a hard-fail parser unless product requirements change
- Review judgment:
  - medium observability/error-handling issue

## Module Boundaries And Technical Debt

### 1. The train-selection engine and holdout-augmentation engine are split across files, but they share the same public request/result types

- Evidence:
  - this file owns train aggregation and sets holdout fields to `None`:
    - `crates/app_core/src/meta_select.rs:426-786`
    - `714`
    - `784`
  - `commands_meta.rs` later mutates the same result shape to inject holdout summary, group holdout metrics, holdout rows, and refreshed evidence flags:
    - `src-tauri/src/commands_meta.rs:2556-2641`
    - `2647-3102`
    - `2594`
- Why it matters:
  - the split is functionally reasonable, but the contract boundary is blurry
  - backend ownership is harder to review because stage-specific behavior is encoded as shared types plus post-hoc mutation rather than explicit stage outputs
- Smallest justified fix:
  - document the two-stage flow in both this file and `commands_meta.rs`
  - defer any type split until the command/reporting consumers are audited together
- Review judgment:
  - medium-high module-boundary / tech-debt issue

## Docs And Examples

### 1. The file exports non-trivial public APIs with little or no rustdoc about stage ownership, grouping behavior, or compatibility

- Evidence:
  - public items include:
    - `MetaSelectionRequest`: `crates/app_core/src/meta_select.rs:192`
    - `HoldoutAggregate`: `242`
    - `HoldoutSummary`: `253`
    - `MetaCandidateAggregate`: `259`
    - `HoldoutDatasetRow`: `297`
    - `MetaSelectionResult`: `327`
    - `refresh_meta_group_evidence_flags`: `413`
    - `meta_select_from_artifacts`: `426`
    - `meta_group_key`: `1004`
    - `meta_group_key_with_context`: `1020`
    - `meta_group_key_for_request`: `1046`
    - `meta_group_key_for_request_with_context`: `1064`
  - only field-level comments exist for some holdout options:
    - `202-214`
  - there is no file-level explanation of:
    - train-stage versus holdout-stage ownership
    - legacy MA-grouping fallback versus strategy-agnostic context grouping
    - canonicalization compatibility expectations
- Smallest justified fix:
  - add rustdoc to the public request/result types and entrypoints
  - state clearly that:
    - this file performs train-side selection
    - holdout augmentation is a second stage
    - grouped keys are compatibility-sensitive and should remain deterministic

## Tests And Missing Verification

### 1. File-local coverage is good, but the two-stage contract and diagnostics behavior are not pinned down directly

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core meta_select::tests:: -- --nocapture`
      - passed
      - included:
        - `canonical_json_is_order_invariant_for_objects`
        - `default_shortlist_fallback_is_hard_mode`
        - `pick_representatives_reuses_cached_signatures_for_duplicate_params`
        - `strict_shortlist_respects_cross_pareto_when_present`
        - `strict_shortlist_hard_mode_excludes_when_cross_pareto_missing`
        - `strict_shortlist_hard_mode_excludes_when_cross_score_non_positive`
        - `meta_select_groups_and_scores`
        - `meta_select_populates_evidence_flags_for_thin_groups`
        - `meta_select_rejects_unapproved_validation_artifacts`
        - `meta_select_keeps_legacy_validation_artifacts_compatible_without_run_verdict`
        - `back_compat_old_artifact_without_generic_context_uses_legacy_ma_grouping`
        - `forward_compat_generic_context_groups_without_legacy_ma_fields`
        - `clustered_grouping_buckets_similar_params`
  - benchmark compile verification run:
    - `cargo bench -p ta_app_core --bench meta_select_workflow --no-run`
      - passed
  - existing tests do not directly pin:
    - that holdout-specific request fields are intentionally ignored by `meta_select_from_artifacts`
    - that the two-stage `meta_select.rs` -> `commands_meta.rs` handoff remains contract-compatible if request/result fields change
    - that diagnostic skip counters remain informative if schema tolerance evolves
- Missing tests I would want before changing this file:
  - an integration test in `commands_meta.rs` that asserts the train-stage result is safely augmentable into the final holdout-bearing result shape
  - an explicit compatibility test for holdout-related request fields so they are either documented no-ops here or actively enforced elsewhere

## Performance, Allocation, And CUDA Concerns

### 1. The selector is intentionally allocation-heavy and key-heavy, so performance edits here are benchmark-sensitive

- Evidence:
  - grouping is accumulator-based over `HashMap<String, GroupAcc>`:
    - `crates/app_core/src/meta_select.rs:433`
  - group keys are built from JSON values and context:
    - `512-520`
    - `1020-1044`
  - clustered grouping quantizes JSON values into new owned maps/arrays:
    - `1119-1129`
  - representative dedup uses canonical signatures and `HashSet<String>`:
    - `1409-1422`
  - canonicalization recursively allocates `String`s:
    - `1435-1476`
  - the workflow is explicitly benchmarked:
    - `crates/app_core/benches/meta_select_workflow.rs:167-252`
- Why it matters:
  - this is not a place for speculative micro-optimization
  - key-shape, canonicalization, ordering, or representative-selection changes can alter both throughput and group identity semantics
- Smallest justified fix:
  - no urgent file-local performance rewrite is justified by the current evidence
  - if profiling later shows this as a bottleneck, evaluate allocation reductions under the benchmark harness before changing key/canonicalization behavior
- Review judgment:
  - benchmark-sensitive performance area
  - defer structural performance changes until measured

### 2. Stable ordering and deterministic key generation are more important than squeezing out minor allocation savings

- Evidence:
  - representative params are sorted by `robust_score.total_cmp`:
    - `crates/app_core/src/meta_select.rs:626-630`
  - train rows are explicitly sorted:
    - `643-648`
  - `canonical_json` exists specifically to produce order-invariant keys:
    - `1435-1476`
  - there is a file-local test that locks in object-order invariance:
    - `1484-1488`
- Review judgment:
  - preserve deterministic behavior first
  - treat any change to key formation or ordering as semantics-sensitive, not just performance work

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc describing the two-stage train/holdout contract
  - fix the warning-string encoding artifact at `crates/app_core/src/meta_select.rs:751`
  - add additive skip-reason counters or debug diagnostics
  - add a generic progress helper name without changing payload semantics
- Coordination required:
  - splitting train-stage versus holdout-stage request/result types
  - changing which request fields are honored by `meta_select_from_artifacts`
  - changing group-key semantics or compatibility guarantees
- Defer until later pass:
  - any allocation/canonicalization redesign driven by performance rather than contract clarity

## Technical-Debt Reduction Register

- Safe to reduce now:
  - undocumented two-stage ownership
  - generic-meta versus Double-MA progress naming leakage
  - weak diagnostics around why artifacts were skipped
  - the warning-string encoding artifact
- Coordination required:
  - public request/result reshaping
  - group-key compatibility changes
- Useful debt that should not be “cleaned up” casually:
  - tolerant backward-compat handling for older artifacts
  - deterministic canonicalization and explicit row sorting

## Performance Opportunity Register

- Safe structural improvements:
  - none clearly justified from current evidence
- Benchmark-sensitive candidates:
  - key/canonicalization allocation reductions
  - group-key interning or cached-key redesign
  - alternative grouping structures or order strategies
- Not the right place to chase performance without evidence:
  - replacing deterministic JSON-based keys with a faster but less transparent scheme

## Observability And Diagnostics Gaps

- the file does not explain that holdout augmentation is owned elsewhere
- the file does not count or surface the main artifact-skip reasons separately
- the generic progress phases still ride on a Double-MA-named emitter
- one warning string contains an encoding artifact:
  - `crates/app_core/src/meta_select.rs:751`

## Implementation Staging Guidance

- Safe now:
  - docs
  - additive diagnostics
  - warning-text cleanup
  - generic progress aliasing
- Coordination required:
  - request/result contract reshaping
  - changing matcher semantics for holdout fields
  - any compatibility-affecting group-key change
- Defer until later pass:
  - benchmark-driven allocation redesign

## Change Blast Radius

- Local only:
  - docs
  - warning text
  - additive counters/diagnostics
- Same subsystem:
  - request/result or grouping changes affect `commands_meta.rs`, `commands_reports.rs`, `commands_reports_meta_xlsx.rs`, and `double_ma_validate.rs`
- Cross-backend / frontend-backend contract:
  - public type changes flow through the Tauri command surface and any persisted/reported meta artifacts

## Open Questions Or Assumptions

- assumption:
  - the intended architecture is that `meta_select.rs` performs train-side aggregation only, and `commands_meta.rs` owns holdout execution and result augmentation
- open question:
  - whether any non-Tauri caller depends on the current shared request type and expects holdout fields to be honored by `meta_select_from_artifacts` directly
- open question:
  - whether meta-group keys are already treated as externally stable artifact/report identifiers, or only as internal grouping helpers

## Recommended Action

`Small surgical refactor`

The right next pass is:

- document the two-stage contract clearly
- improve diagnostics and naming without changing semantics
- defer request/result reshaping and benchmark-facing key changes until the adjacent `commands_meta.rs` and reporting surfaces are reviewed together

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core meta_select::tests:: -- --nocapture`
- `cargo bench -p ta_app_core --bench meta_select_workflow --no-run`
- `cargo test -p ta_desktop_demo_app commands_meta::tests:: -- --nocapture`
- if request/result fields or group-key semantics change:
  - `cargo test --workspace --no-run`

## Regression Risk And Feature-Preservation Constraints

- do not remove or repurpose holdout-related request/result fields without checking `commands_meta.rs` and the reporting/export surfaces
- do not change canonical key formation or ordering semantics casually; that is both benchmark-sensitive and compatibility-sensitive
- do not “simplify” tolerant legacy-artifact handling unless a deliberate migration plan exists
