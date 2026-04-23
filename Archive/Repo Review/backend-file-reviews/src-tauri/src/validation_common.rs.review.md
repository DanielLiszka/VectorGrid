# Review Packet: `src-tauri/src/validation_common.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/validation_common.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful shared validation-support module, but not yet accept-as-is because one path-normalization helper encodes Windows-biased identity semantics and several statistical/key-canonicalization contracts are still implicit`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Validation-pipeline-sensitive`, `Cross-ticker-sensitive`, `Semantics-sensitive`, `Cross-platform-sensitive`

## What This File Owns

`validation_common.rs` owns the reusable support layer for cross-ticker validation aggregation in the Tauri backend.

This file owns:

- the cross-ticker accumulation container:
  - `CrossTickerAcc`: `src-tauri/src/validation_common.rs:11-28`
- the candidate abstraction shared across validation result types:
  - `CrossTickerCandidate`: `31-42`
  - impls for:
    - `CompositeValidationCandidateResult`: `49-87`
    - `DoubleMaValidationCandidateResult`: `89-127`
- cross-ticker config sanitation and path-key normalization:
  - `normalize_path_key`: `45-47`
  - `sanitize_cross_ticker_config`: `129-158`
- gate evaluation for finished cross-ticker stats:
  - `cross_ticker_passes_gate`: `160-189`
- dataset labeling for reports/summary metadata:
  - `dataset_label`: `191-201`
- canonical candidate key generation for parameter matching:
  - `canonicalize_json_value`: `203-219`
  - `candidate_key_canonical`: `221-224`
- metric accumulation and summary statistics:
  - `push_metrics`: `226-234`
  - `quantile`, `median`, `worst_min`, `worst_max`, `median_u32_sorted`, `worst_min_u32`: `236-294`
- final cross-ticker stats attachment, Pareto marking, and ranking:
  - `finalize_cross_ticker_candidates`: `296-380`

This file does not own:

- the end-to-end cross-ticker validation orchestration
  - that lives in `src-tauri/src/commands_validation.rs:217-335`
- the candidate result types themselves
  - those live in `crates/app_core/src/composite_validate.rs` and `crates/app_core/src/double_ma_validate.rs`
- report rendering of cross-ticker summaries
  - that lives in `src-tauri/src/commands_reports_validation_common.rs`

## Classification

- Public surface: internal shared backend API
- Hot path: moderate
  - it is on the cross-ticker validation path, but not on per-bar execution loops
- Runtime-critical: moderate to high
  - bugs here can reorder finalists, drop datasets, or mis-gate candidates
- CUDA/runtime-critical: indirect only
  - validation flows may later exercise CPU/GPU backends, but this file is orchestration/support logic
- Strategy-local: no
- Low-risk support code: not fully
  - it is shared validation policy, not incidental glue

## Direct Interactions

### Backend interaction map

- canonical command-layer consumer:
  - `src-tauri/src/commands_validation.rs:151-165`
  - `src-tauri/src/commands_validation.rs:243-330`
- candidate types imported from app-core:
  - `ta_app_core::composite_validate::CompositeValidationCandidateResult`
  - `ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult`
- similar path-normalization logic is duplicated elsewhere instead of reusing this module:
  - `src-tauri/src/commands_ml_signal.rs:31-33`
  - `src-tauri/src/commands_campaign.rs:648-650`
- similar canonical-JSON logic also exists in app-core:
  - `crates/app_core/src/validation_shared.rs:445-477`

### Downstream verification impact if changed

- any change to `normalize_path_key` or `sanitize_cross_ticker_config` requires verification in:
  - `commands_validation`
  - any artifact-context path matching that depends on this helper
- any change to `candidate_key_canonical` or `finalize_cross_ticker_candidates` requires verification in:
  - cross-ticker candidate matching
  - finalist ordering
  - report summaries consuming `CrossTickerStats`

### Test anchors

- local unit coverage:
  - `src-tauri/src/validation_common.rs:391-601`
- cross-summary/report tests that incidentally exercise this module:
  - `src-tauri/src/commands_reports_validation_common.rs`

## Standards And Research Log

This packet was written after targeted external review focused on cross-platform path identity, canonical JSON serialization for stable candidate keys, and float-ordering behavior for deterministic ranking.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes shared internal APIs that encode validation policy across multiple modules
   - Standard informed:
     - reusable helpers should keep semantics explicit, especially around naming and hidden policy

2. `std::fs::canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why it was consulted:
     - this file currently uses a string-normalization helper as a path-identity policy
   - Standard informed:
     - stable file identity is stronger when derived from normalized filesystem identity rather than Windows-specific text rewriting

3. `Path::file_stem`
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html#method.file_stem>
   - Why it was consulted:
     - `dataset_label` falls back to filename-derived labels
   - Standard informed:
     - filename-based label fallback is reasonable, but should stay intentionally presentation-only

4. `serde_json::to_value`
   - Source: <https://docs.rs/serde_json/latest/serde_json/fn.to_value.html>
   - Why it was consulted:
     - `candidate_key_canonical` derives stable keys from serialized params
   - Standard informed:
     - canonical key generation depends on a predictable JSON value transform and should stay aligned across layers

5. `f64::total_cmp`
   - Source: <https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp>
   - Why it was consulted:
     - this file sorts floating-point vectors and final candidate rankings using total ordering
   - Standard informed:
     - deterministic float sorting is correct here, but any ranking-semantics change should remain explicit and tested

## Findings

## Correctness And Safety

### 1. `normalize_path_key` is Windows-biased and can collapse distinct paths on case-sensitive platforms

- Evidence:
  - helper lowercases all paths and rewrites `/` to `\`:
    - `src-tauri/src/validation_common.rs:45-47`
  - it is used to dedupe configured datasets:
    - `129-134`
  - it is also used to compare artifact-context path vs currently loaded dataset path:
    - `src-tauri/src/commands_validation.rs:151-165`
- Why it matters:
  - on Windows this is often convenient
  - on case-sensitive filesystems, two distinct files differing only by case can be treated as the same dataset
  - the same issue can let a mismatched dataset path appear compatible during validation-context checks
- Smallest justified fix:
  - make normalization platform-aware
  - on Windows, case-folding may remain acceptable
  - on Unix-like platforms, avoid unconditional lowercasing and prefer canonical or component-based normalization when the file exists
- Review judgment:
  - objective cross-platform correctness issue

## API And Naming

### 1. `median` does not implement the arithmetic median for even-sized inputs, and that semantics is currently undocumented

- Evidence:
  - `median` delegates to `quantile(sorted, 0.5)`:
    - `src-tauri/src/validation_common.rs:257-259`
  - `quantile` rounds the index rather than averaging the middle pair:
    - `236-255`
- Why it matters:
  - for even-sized dataset sets, this chooses the upper-middle element rather than the average of the two middle values
  - that may be intentional, but the function name reads like standard median semantics
  - these values directly feed:
    - `sharpe_median`
    - `max_dd_median`
    - `pnl_median`
    - final `score`
- Smallest justified fix:
  - either document the current nearest-rank semantics clearly
  - or change to a true arithmetic median only with explicit regression review because it will alter cross-ticker ranking and gating behavior
- Review judgment:
  - semantics-sensitive; do not change casually

## Error Handling

### 1. Serialization failures are flattened into plain strings, which is acceptable here but hides which candidate/key path failed

- Evidence:
  - `candidate_key_canonical` maps both `to_value` and `to_string` errors with `.to_string()`:
    - `src-tauri/src/validation_common.rs:221-224`
- Why it matters:
  - the command layer already uses `String` errors, so this is not inherently wrong
  - but when candidate-key generation fails, the resulting error loses higher-level context about which stage or candidate triggered it
- Smallest justified fix:
  - wrap the current string with operation context, for example `"failed to canonicalize validation params: ..."`
- Review judgment:
  - low severity, but easy local diagnostics improvement

## Module Boundaries And Technical Debt

### 1. Canonical JSON key generation is duplicated across Tauri and app-core layers

- Evidence:
  - local canonicalization:
    - `src-tauri/src/validation_common.rs:203-224`
  - app-core already has the same conceptual helper:
    - `crates/app_core/src/validation_shared.rs:445-477`
- Why it matters:
  - this is a drift risk
  - if one side changes canonicalization rules and the other side does not, cross-layer candidate matching can diverge silently
- Smallest justified fix:
  - converge on one canonical helper
  - if layer boundaries make immediate reuse awkward, add a parity test proving both implementations remain equivalent
- Review judgment:
  - real shared-logic debt, but still small enough to address surgically

### 2. Path-normalization logic is also duplicated in other Tauri command modules

- Evidence:
  - duplicate helpers in:
    - `src-tauri/src/commands_ml_signal.rs:31-33`
    - `src-tauri/src/commands_campaign.rs:648-650`
- Why it matters:
  - if `validation_common` is corrected for platform semantics but the other copies are not, repo behavior will diverge by workflow
- Smallest justified fix:
  - either centralize this policy
  - or create a small shared path-identity utility with explicit platform behavior
- Review judgment:
  - cross-file debt that should be coordinated, not fixed in isolation without checking the sibling modules

## Docs And Examples

### 1. The module would benefit from local comments on policy-sensitive helpers

- Missing documentation topics:
  - whether `normalize_path_key` is intended to be Windows-only compatibility logic or true path identity
  - whether `median` is intended as nearest-rank or arithmetic median
  - whether `candidate_key_canonical` must remain byte-for-byte aligned with app-core canonicalization

## Tests And Missing Verification

### 1. Existing local coverage is good and currently green

- Verified in this review pass:
  - `cargo test -p ta_desktop_demo_app validation_common -- --nocapture`
    - passed `13` relevant tests, including local unit tests and dependent validation-report tests
- Existing strengths:
  - gate clamping:
    - `src-tauri/src/validation_common.rs:391-416`
  - normalized path dedupe behavior as currently implemented:
    - `419-445`
  - gate enforcement:
    - `448-469`
  - robust-score folding and ranking:
    - `526-571`
  - canonical JSON key stability for nested object key order:
    - `574-601`

### 2. Important policy edges are not yet explicitly tested

- Missing tests found in this review pass:
  - platform-sensitive path normalization behavior
  - parity between `validation_common::candidate_key_canonical` and `ta_app_core::validation_shared` canonicalization
  - explicit even-cardinality median behavior
  - tie behavior for Pareto marking and final ranking when multiple candidates have identical stats

### 3. Exact local verification required if changed

- `cargo test -p ta_desktop_demo_app validation_common -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
- if path-normalization behavior changes:
  - `cargo test -p ta_desktop_demo_app commands_ml_signal -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands_campaign -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful micro-optimization work is justified today

- Evidence:
  - this file is mostly vector sorting and summary construction over finalist-sized candidate sets
- Review judgment:
  - clarity and semantic stability matter more than allocation churn here

### 2. The Pareto pass is quadratic, but changing it is benchmark-sensitive rather than obviously necessary

- Evidence:
  - Pareto dominance checks use a nested candidate loop:
    - `src-tauri/src/validation_common.rs:356-377`
- Why it matters:
  - this may be fine for current finalist counts
  - a more complex optimization is not justified without evidence that candidate counts make this hot
- Review judgment:
  - benchmark-sensitive; do not touch yet

## Code-Quality Improvement Register

- Safe now:
  - make path normalization platform-aware
  - add operation context to canonicalization errors
  - document median semantics
  - add a parity test for canonical JSON key generation across layers
- Defer:
  - larger reuse/consolidation work across `commands_validation`, `commands_ml_signal`, and `commands_campaign` unless done deliberately as one bounded cleanup

## Technical-Debt Reduction Register

- Safe to reduce now:
  - Windows-only assumptions in `normalize_path_key`
  - undocumented median semantics
  - canonical JSON helper drift risk
- Defer because it spans multiple files:
  - centralizing all path-normalization helpers into one repo-wide utility
  - cross-layer canonical JSON reuse if it requires app-core surface changes
- Useful debt that should not be “cleaned up” casually:
  - deterministic float ordering via `total_cmp`
  - simple local stats aggregation over generic abstraction layers

## Performance Opportunity Register

- Safe structural improvement:
  - none urgent
- Benchmark-sensitive or semantics-sensitive:
  - changing Pareto algorithm complexity
  - changing median/quantile semantics
  - changing candidate-key generation format

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - preserve current cross-ticker candidate matching and gating behavior unless a semantics change is explicitly intended
  - preserve stable deterministic ranking order for equal inputs
- Regression risk if changed carelessly:
  - candidates can be mismatched across datasets
  - datasets can be deduped incorrectly
  - cross-ticker gating can silently change pass/fail outcomes
  - report summaries can drift from validation ordering
- Safe change posture:
  - prefer platform-aware path fixes, parity tests, and documentation over algorithm churn
