# Review Packet: `crates/app_core/src/ml_signal.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/ml_signal.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Feature-complete and locally well covered, but not fully public-review ready because dataset sidecar artifacts can be published before the main CSV completes, duplicate score timestamps silently resolve as last-write-wins, score-scope semantics can be inferred from the first populated row or even the filename without validating consistency, and model-bundle listing fails closed on the first malformed manifest`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Workflow-critical`, `Semantics-sensitive`, `Benchmark-sensitive`, `Artifact-contract`

## What This File Owns

`ml_signal.rs` owns the backend workflow for ML-signal dataset export, score-cache ingestion, policy validation and sweep execution, and model-bundle manifest inspection.

It owns:

- public score-space, score-scope, dataset-export, score-cache, policy, validation, sweep, and model-bundle DTOs:
  - `MlSignalScoreSpace`: `crates/app_core/src/ml_signal.rs:75-79`
  - `MlSignalScoreScope`: `82-87`
  - dataset export DTOs:
    - `90-201`
  - score-cache DTOs:
    - `204-233`
    - `445-471`
  - policy / validation / sweep DTOs:
    - `234-443`
  - model-bundle manifest/status DTOs:
    - `474-507`
- feature-engineering helpers and frame construction:
  - `log_returns_from_close`: `548-560`
  - indicator-param helpers:
    - `562-584`
  - pattern and series extraction:
    - `586-649`
  - `feature_frame`: `680-772`
- formatting/stat/output-path utilities:
  - `774-1008`
- dataset export:
  - `assign_ml_signal_ticker_holdout`: `937-945`
  - `build_ml_signal_split_windows`: `948-994`
  - `export_ml_signal_dataset_with_candles`: `1010-1337`
- score CSV parsing, score-cache build, and score-cache load:
  - CSV parse helpers:
    - `1391-1583`
  - `build_ml_signal_score_cache_from_csv`: `1585-1707`
  - `load_score_cache`: `1714-1815`
- policy targeting, overlay normalization, validation, and sweep:
  - `score_target`: `1817-1830`
  - overlay helpers:
    - `1834-1999`
  - `validate_ml_signal_candidates`: `2002-2079`
  - axis expansion and contract validation:
    - `2082-2180`
  - public validation/sweep entry points:
    - `2182-2317`
- model-bundle manifest read/classify/list helpers:
  - `2319-2421`
- file-local tests:
  - `2425-3387`

This file does not own:

- Tauri command orchestration and app-data model directory resolution:
  - `src-tauri/src/commands_ml_signal.rs`
- generic dynamic-indicator execution:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
- iterative execution metrics:
  - `crates/app_core/src/iterative_sim.rs`
- generic validation scoring and verdict logic:
  - `crates/app_core/src/validation_pipeline.rs`

## Classification

- Public surface: yes
  - the module is public at `crates/app_core/src/lib.rs:47`
  - a broad set of DTOs and entry points are re-exported at `crates/app_core/src/lib.rs:201-213`
- Hot path: moderate batch hot path
  - not a per-bar runtime kernel, but dataset export, score-cache build, and sweep validation all contain long-running batch loops
- CUDA/runtime-critical: no
- Strategy-local: yes
  - this file is specific to the `MlSignal` backend workflow
- Low-risk support code: no
  - it produces user-facing artifacts, validation results, and model-bundle status

## Direct Interactions

### Backend interaction map

- crate export surface:
  - `crates/app_core/src/lib.rs:47`
  - `201-213`
- Tauri command boundary:
  - `src-tauri/src/commands_ml_signal.rs:96-201`
- campaign exclusion route:
  - `src-tauri/src/commands_campaign.rs:613-614`
- feature-engineering dependency:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
    - `feature_frame` uses `compute_dynamic_indicator_cpu` at `crates/app_core/src/ml_signal.rs:685-757`
- validation/robust-score dependency:
  - `crates/app_core/src/validation_pipeline.rs`
    - imported at `crates/app_core/src/ml_signal.rs:5-9`
- iterative-sim dependency:
  - `crates/app_core/src/iterative_sim.rs`
    - imported at `crates/app_core/src/ml_signal.rs:4`
    - used in `validate_ml_signal_candidates`: `2036-2056`
- validation artifact context dependency:
  - `crates/app_core/src/validation_shared.rs`
    - imported at `crates/app_core/src/ml_signal.rs:10`
- representative bench surfaces:
  - `crates/app_core/benches/ml_signal_workflow.rs:103-231`
  - `crates/app_core/benches/ml_signal_validation.rs:129-218`

### Downstream verification impact if changed

- changing dataset-export DTOs or artifact paths affects:
  - `src-tauri/src/commands_ml_signal.rs`
  - any external workflow that consumes exported CSV/schema/manifest triplets
- changing score-cache parsing or manifest semantics affects:
  - validation and sweep entry points in this file
  - any score-cache artifacts stored on disk
- changing policy-target or overlay semantics affects:
  - `simulate_iterative_from_signals` consumers
  - benchmark baselines in `ml_signal_validation.rs` and `ml_signal_workflow.rs`
- changing model-bundle listing/classification affects:
  - `src-tauri/src/commands_ml_signal.rs:164-201`

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API expectations, Serde compatibility, artifact-writing behavior, CSV writer behavior, time-series split conventions, and partial-selection performance guidance.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exports a broad public DTO and function surface
   - Standard informed:
     - public APIs should keep contracts intentional, documented, and meaningfully constrained

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the file exposes many public request/result/helper types with no rustdoc contract sections
   - Standard informed:
     - public items should have examples, and fallible functions should document `Errors` / `Panics` / safety considerations where relevant

3. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - the public manifest/request structs rely heavily on `#[serde(default)]` and `skip_serializing_if`
   - Standard informed:
     - omitted/defaulted fields are part of the wire-compatibility contract and should be treated intentionally

4. Rust Book, Recoverable Errors With `Result`
   - Source: <https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html>
   - Why it was consulted:
     - this file performs substantial file I/O and CSV/JSON parsing behind `Result<_, String>` boundaries
   - Standard informed:
     - recoverable file/parse errors should preserve clear, caller-actionable context

5. `serde_json::to_writer_pretty`
   - Source: <https://docs.rs/serde_json/latest/serde_json/fn.to_writer_pretty.html>
   - Why it was consulted:
     - this file emits feature schema, split manifest, export manifest, and score-cache JSON artifacts
   - Standard informed:
     - JSON artifact writing is streaming I/O and should be staged carefully if partial publication would be misleading

6. `std::fs::rename`
   - Source: <https://doc.rust-lang.org/std/fs/fn.rename.html>
   - Why it was consulted:
     - the file currently writes final artifact paths directly instead of using a staged publish flow
   - Standard informed:
     - a temp-file-plus-rename pattern is the standard small fix when artifact publication should appear atomic to callers

7. `csv::Writer` documentation
   - Source: <https://docs.rs/csv/latest/csv/struct.Writer.html>
   - Why it was consulted:
     - dataset export writes CSV via `csv::WriterBuilder`
   - Standard informed:
     - the CSV writer is already buffered, so wrapping it in an extra `BufWriter` is unnecessary I/O layering

8. scikit-learn `TimeSeriesSplit`
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html>
   - Why it was consulted:
     - this file generates walk-forward style split windows for exported datasets
   - Standard informed:
     - split-window semantics should be documented clearly as time-ordered, expanding/rolling windows rather than treated as generic train/test partitions

9. Rust slice selection APIs
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.select_nth_unstable_by>
   - Why it was consulted:
     - the sweep path fully sorts all candidates before truncating to `top_k`
   - Standard informed:
     - when only the top subset is needed, partial selection is the standard library primitive to consider before paying full-sort cost

## Findings

## Correctness And Safety

### 1. Dataset sidecar artifacts are written before the main CSV completes, so failures can leave a misleading partial export set

- Evidence:
  - feature schema is written first:
    - `crates/app_core/src/ml_signal.rs:1042-1043`
  - split manifest is also written before the dataset CSV exists:
    - `1050-1055`
  - only after that does the file create and stream the dataset CSV:
    - `1065-1069`
    - row loop through `1293`
  - the export manifest is published only after flush:
    - `1319-1320`
- Why it matters:
  - cancellation or a row-write failure can leave schema/split sidecars on disk without a valid dataset CSV
  - that makes the artifact directory look more complete than it actually is
- Smallest justified fix:
  - keep the current artifact shapes and filenames
  - but write schema/split/data artifacts to temp paths first and `rename` them into place after successful flush
  - at minimum, avoid publishing sidecars before the main CSV is known-good
- Review judgment:
  - medium-high correctness / artifact-integrity issue

### 2. Duplicate score timestamps are silently resolved as last-write-wins

- Evidence:
  - duplicates are only counted:
    - `crates/app_core/src/ml_signal.rs:1609-1618`
  - matched rows are stored in a `HashMap<i64, MlSignalScoreCacheRow>`:
    - `1607`
  - later rows overwrite earlier rows through `matched.insert(...)`:
    - `1623-1632`
  - the result surface reports only the duplicate count, not which row won:
    - `1660-1688`
- Why it matters:
  - score-cache semantics depend on input ordering whenever the CSV contains multiple rows for the same timestamp
  - that is a real semantics decision, not just cleanup detail
- Smallest justified fix:
  - either document last-write-wins explicitly
  - or reject conflicting duplicates while still allowing exact repeats
  - avoid silently changing this to first-write-wins without a compatibility check
- Review judgment:
  - medium semantics-sensitive correctness issue

## API And Naming

### 1. `score_scope` semantics are under-validated and can be inferred from weak signals

- Evidence:
  - the parser only captures the first non-empty `score_scope` cell it sees:
    - `crates/app_core/src/ml_signal.rs:1545-1564`
  - later rows are not checked for consistency
  - when no explicit scope is found, `infer_score_scope` falls back to filename heuristics:
    - `1478-1498`
- Why it matters:
  - a mixed-scope CSV can still produce a single manifest-level `score_scope`
  - filename-based inference is useful as a fallback, but it should not quietly substitute for a validated contract when a scope column exists
- Smallest justified fix:
  - if a `score_scope` column is present, require all populated rows to agree
  - keep filename inference only as a documented fallback when the column is absent
- Review judgment:
  - medium API / artifact-contract issue

### 2. The public surface is broader than the apparent in-repo consumer set

- Evidence:
  - the module is public at `crates/app_core/src/lib.rs:47`
  - many DTOs and helpers are re-exported at `crates/app_core/src/lib.rs:201-213`
  - primary in-repo consumers appear to be:
    - `src-tauri/src/commands_ml_signal.rs:96-201`
    - the bench surfaces under `crates/app_core/benches/`
- Why it matters:
  - every re-exported DTO and helper becomes a compatibility promise, including artifact-shape helpers and manifest inspection functions
  - that raises the cost of later cleanup or narrowing
- Smallest justified fix:
  - audit which items truly need crate-external visibility
  - keep request/result artifact structs public if they are part of the command contract
  - consider narrowing purely internal helpers in a later coordinated pass
- Review judgment:
  - medium public-surface issue

## Error Handling

### 1. Model-bundle listing aborts the whole directory scan on the first malformed manifest

- Evidence:
  - `list_ml_signal_model_bundle_summaries` scans the root directory:
    - `crates/app_core/src/ml_signal.rs:2378-2405`
  - each discovered manifest is classified with `?`:
    - `2406`
  - a parse or classification error therefore aborts the full listing instead of surfacing a per-bundle warning row
- Why it matters:
  - one broken manifest can hide all healthy model bundles from the UI/listing consumer
  - this is especially unfriendly for a directory intended to hold multiple independent bundles
- Smallest justified fix:
  - degrade malformed bundles into a warning summary entry instead of failing the full listing
  - preserve the hard error only for directory-read failures or similarly global problems
- Review judgment:
  - medium error-handling / operational resilience issue

## Module Boundaries And Technical Debt

### 1. This file concentrates too many distinct responsibilities into one public module

- Evidence:
  - public DTOs: `75-507`
  - feature engineering: `548-772`
  - export-path helpers and artifact writing: `876-1337`
  - score-cache parsing/build/load: `1391-1815`
  - policy targeting and overlays: `1817-1999`
  - validation and sweep: `2002-2317`
  - manifest inspection: `2319-2421`
- Why it matters:
  - the module is expensive to review and risky to edit because artifact I/O, validation semantics, and bundle inspection all live together
  - even small changes have a broad local blast radius
- Smallest justified fix:
  - do not split it immediately
  - document seams now, then consider a later split into `export`, `score_cache`, `policy`, and `model_bundle` submodules if maintenance pressure continues
- Review judgment:
  - medium technical-debt issue

### 2. The file carries warning-level dead-code debt that should be cleaned deliberately

- Evidence:
  - boundary compile and bench/test runs emitted dead-code warnings for:
    - `ScorePoint::raw_long` / `raw_short`: `crates/app_core/src/ml_signal.rs:525-529`
    - `stddev`: `782-789`
    - `current_unix_ms`: `1003-1007`
- Why it matters:
  - these warnings are local to the file and add noise to verification output
  - they also suggest earlier semantics or planned hooks that are no longer active
- Smallest justified fix:
  - either remove clearly dead helpers/fields
  - or make the intended future use explicit if the file is deliberately retaining them
- Review judgment:
  - low-medium code-quality / debt issue

## Docs And Examples

### 1. The public export, score-cache, validation, and model-bundle surfaces lack rustdoc contract guidance

- Evidence:
  - representative public items include:
    - `MlSignalDatasetExportRequest`: `169`
    - `MlSignalDatasetExportResult`: `185`
    - `MlSignalScoreCacheBuildRequest`: `204`
    - `MlSignalScoreCacheManifest`: `445`
    - `MlSignalModelBundleManifest`: `474`
    - `MlSignalPolicyParams`: `234`
    - `MlSignalValidationRequest`: `269`
    - `MlSignalPolicySweepRequest`: `298`
    - `assign_ml_signal_ticker_holdout`: `937`
    - `build_ml_signal_split_windows`: `948`
    - `export_ml_signal_dataset_with_candles`: `1010`
    - `build_ml_signal_score_cache_from_csv`: `1585`
    - `run_ml_signal_validation_blocking_with_candles`: `2182`
    - `run_ml_signal_policy_sweep_blocking_with_candles`: `2216`
    - `read_ml_signal_manifest`: `2319`
    - `classify_ml_signal_manifest`: `2335`
    - `list_ml_signal_model_bundle_summaries`: `2378`
- Why it matters:
  - callers need to know:
    - duplicate timestamp policy
    - score-scope inference policy
    - split-window semantics
    - whether raw-margin caches are accepted or only probability caches are valid for threshold-based validation
    - the meaning of model-bundle `state`
- Smallest justified fix:
  - add rustdoc with at least `Errors` sections and one end-to-end example or linked example for the main workflow entry points
  - document the on-disk artifact contract explicitly

## Tests And Missing Verification

### 1. Local file coverage is good, but several artifact-contract edge cases remain unpinned

- Evidence:
  - local test verification run:
    - `cargo test -p ta_app_core ml_signal::tests:: -- --nocapture`
      - passed `17` tests, including:
        - `score_cache_build_and_validation_smoke`
        - `policy_sweep_returns_ranked_candidates`
        - `validation_rejects_raw_only_score_cache`
        - `validation_rejects_raw_only_cache_and_missing_short_side`
        - `build_split_windows_generates_rolls`
        - `ticker_holdout_assignment_is_stable`
  - bench compile verification runs:
    - `cargo bench -p ta_app_core --bench ml_signal_workflow --no-run`
      - passed
    - `cargo bench -p ta_app_core --bench ml_signal_validation --no-run`
      - passed
  - Tauri boundary compile verification run:
    - `cargo test -p ta_desktop_demo_app --no-run`
      - passed
- Missing tests I would want before changing this file:
  - duplicate timestamp policy:
    - exact-repeat versus conflicting-repeat behavior
  - mixed `score_scope` rows:
    - explicit rejection or normalization policy
  - partial artifact cleanup:
    - ensure failed/cancelled export does not leave a misleading sidecar set
  - malformed manifest listing:
    - confirm one bad bundle does not suppress all healthy bundles if listing behavior is relaxed

## Performance, Allocation, And CUDA Concerns

### 1. Policy sweep fully sorts all candidates before truncating to `top_k`

- Evidence:
  - validation produces the full candidate set:
    - `crates/app_core/src/ml_signal.rs:2287-2298`
  - candidates are then fully sorted:
    - `2299-2301`
  - only after the full sort does the code truncate:
    - `2302-2303`
- Why it matters:
  - the sweep budget can expand to `20,000` candidates:
    - `2241-2243`
  - this is not catastrophic at current scales, but it is the main obvious algorithmic hotspot in the sweep reducer
- Smallest justified fix:
  - do not change it blindly
  - if profiling shows sweep reduction matters, replace the full sort with partial selection and then sort only the retained `top_k`
- Review judgment:
  - medium performance issue
  - benchmark-sensitive rather than urgent

### 2. Dataset export double-buffers the CSV writer even though `csv::Writer` already buffers internally

- Evidence:
  - dataset export creates:
    - `WriterBuilder::new().has_headers(true).from_writer(BufWriter::new(file))`
    - `crates/app_core/src/ml_signal.rs:1067-1069`
- Why it matters:
  - this is low severity, but it adds needless layering in a large streaming write path
  - it is the sort of small safe-now cleanup that improves clarity without changing semantics
- Smallest justified fix:
  - pass `file` directly to `csv::WriterBuilder::from_writer(...)`
  - keep the JSON artifact paths buffered, where the extra writer wrapper is still reasonable
- Review judgment:
  - low performance / allocation cleanliness issue

## Code-Quality Improvement Register

- Safe now:
  - document duplicate timestamp and score-scope semantics
  - stage artifact publication more carefully
  - relax bundle listing so one malformed manifest does not hide the rest
  - remove or justify the local dead-code warnings
  - remove the extra `BufWriter` around the CSV writer
- Coordination required:
  - narrowing the public re-export surface
  - changing duplicate timestamp behavior from the current effective last-write-wins semantics
  - tightening `score_scope` interpretation if stored artifacts already rely on the current fallback behavior
- Defer until later pass:
  - splitting the module into multiple files
  - partial-selection optimization unless sweep profiling says it is worth doing

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing rustdoc
  - partial-artifact publication risk
  - model-bundle listing fail-closed behavior
  - dead-code warning debt
- Coordination required:
  - public surface narrowing
  - duplicate timestamp semantics
  - score-scope normalization contract
- Useful debt that should not be “cleaned up” casually:
  - the shared strategy-local ownership of dataset export, score-cache load, and validation flow, which currently keeps the `MlSignal` workflow coherent

## Performance Opportunity Register

- Safe structural improvements:
  - remove unnecessary CSV double-buffering
- Benchmark-sensitive candidates:
  - partial selection instead of full sort in sweep ranking
  - any change to the candidate-budget cap or ranking order
- Not the right place to chase performance without evidence:
  - broad feature-frame refactors
  - rewrite-level changes to the export/validation pipeline

## Observability And Diagnostics Gaps

- duplicate timestamp handling is only summarized as a count, not as an explicit conflict policy
- export failure/cancellation does not clearly signal whether partial sidecar artifacts were already written
- model-bundle listing does not provide per-bundle degraded status on parse/classification failure

## Implementation Staging Guidance

- Safe now:
  - docs
  - additive tests
  - dead-code cleanup
  - CSV writer cleanup
  - artifact staging/publish cleanup
- Coordination required:
  - public surface narrowing
  - duplicate timestamp policy changes
  - score-scope contract tightening
  - model-listing behavior if frontend logic assumes hard failure today
- Defer until later pass:
  - larger module split
  - top-k reducer optimization unless measurements justify it

## Change Blast Radius

- Local only:
  - rustdoc
  - dead-code cleanup
  - CSV writer cleanup
- Same subsystem:
  - export artifact publication and score-cache behavior changes affect the `MlSignal` workflow end to end
- Cross-backend / frontend-backend contract:
  - public DTO changes affect `src-tauri/src/commands_ml_signal.rs`
  - on-disk artifact contract changes affect any external or persisted workflow consumer

## Open Questions Or Assumptions

- assumption:
  - a `score_scope` column, when present, is intended to be homogeneous across the score CSV
- open question:
  - whether last-write-wins for duplicate timestamps is deliberate so later scoring passes can override earlier rows
- open question:
  - whether bundle listing is expected to be fail-open for operator visibility or fail-closed for strict hygiene

## Recommended Action

`Small surgical refactor`

The right next pass is:

- make the artifact and score-cache contracts explicit
- harden export/listing behavior where the fixes are small and local
- clean the warning-level debt
- defer any public-surface narrowing or ranking-path optimization until the adjacent command and options/runtime packets are complete

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core ml_signal::tests:: -- --nocapture`
- `cargo bench -p ta_app_core --bench ml_signal_workflow --no-run`
- `cargo bench -p ta_app_core --bench ml_signal_validation --no-run`
- `cargo test -p ta_desktop_demo_app --no-run`
- if artifact-listing or public DTO contracts change:
  - `cargo test --workspace --no-run`

## Regression Risk And Feature-Preservation Constraints

- do not silently change duplicate timestamp behavior without deciding which row should win and why
- do not tighten `score_scope` parsing in a way that breaks existing stored CSV workflows without an explicit migration note
- do not remove raw-margin support from the score-cache artifact format unless the broader ML-signal workflow explicitly narrows scope
- do not change validation ranking order or `top_k` semantics casually; treat sweep ordering as benchmark-sensitive
