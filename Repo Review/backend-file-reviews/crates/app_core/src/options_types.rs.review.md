# Review Packet: `crates/app_core/src/options_types.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_types.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and already covered by focused serde/verdict tests, but not fully public-review ready because this file is the canonical persisted contract for the options-expression workflow while still relying on permissive deserialize defaults, stringly typed external discriminators, and almost no inline contract documentation`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Persistence-critical`, `Options-workflow`, `Semantics-sensitive`, `Low-compute`, `Tauri-boundary-adjacent`

## What This File Owns

`options_types.rs` owns the canonical serialized contract for the options-expression backend workflow. It defines the schema that dataset import, signal-manifest materialization, options-expression execution, artifact persistence, replay cache loading, and inspection surfaces all share.

It owns:

- versioned persisted schema constants:
  - `OPTIONS_DATASET_MANIFEST_VERSION`, `OPTIONS_SIGNAL_MANIFEST_VERSION`, `OPTIONS_EXPRESSION_ARTIFACT_VERSION`: `crates/app_core/src/options_types.rs:3-5`
- default policy helpers used during serde deserialization:
  - `7-104`
- options-expression enums and domain labels:
  - `OptionsPriceSemantics` through `OptionsCandidateInvalidReason`: `104-236`
- dataset and signal manifest DTOs:
  - `OptionsDatasetManifestV1`: `313-349`
  - `OptionsPriceRow`: `351-372`
  - `OptionsContractSeries`: `375-385`
  - `ImportedOptionsDataset`: `388-391`
  - `OptionsSignalEvent`: `395-429`
  - `OptionsSignalEventManifestV1`: `433-451`
- tuple/result/inspection/report DTOs:
  - `OptionsTupleSpec`: `455-468`
  - `OptionsExpressionMetrics`: `470-503`
  - `OptionsExpressionTupleResult`: `518-614`
  - inspection/diagnostics payloads: `637-749`
  - feasibility/report payloads: `753-1037`
  - run request/result and persisted artifact payloads: `1039-1123`
- file-local serde and verdict regression tests:
  - `1127-1398`

This file does not own:

- dataset-building logic:
  - `crates/app_core/src/options_data_store.rs`
- signal-manifest materialization policy:
  - `crates/app_core/src/options_signal_manifest.rs`
- request validation, tuple compilation, split assignment, or backend execution:
  - `crates/app_core/src/options_expression.rs`
- artifact replay and desktop command transport:
  - `src-tauri/src/commands_options_expression.rs`

## Classification

- Public surface: yes
  - the types are re-exported from `ta_app_core` at `crates/app_core/src/lib.rs:230-241`
- Hot path: no
  - this file mostly defines schema and light helper methods
- CUDA/runtime-critical: no direct runtime ownership
  - it influences runtime semantics through request/result contracts, but it does not own CUDA execution
- Strategy-local: yes
  - it is specific to the options-expression workflow
- Low-risk support code: no
  - compute cost is low, but schema drift here has high downstream blast radius across persistence, replay, and Tauri IPC

## Direct Interactions

### Backend interaction map

- crate export surface:
  - `crates/app_core/src/lib.rs:230-241`
- dataset import/build producers:
  - `crates/app_core/src/options_data_store.rs:291-433`
- signal-manifest producers:
  - `crates/app_core/src/options_signal_manifest.rs:46-262`
- primary engine consumer and request validator:
  - `crates/app_core/src/options_expression.rs:1755-2043`
  - `5223-5755`
- Tauri replay, artifact persistence, and inspection consumers:
  - `src-tauri/src/commands_options_expression.rs:1073-1136`
  - `1394-1716`
  - `2453-2479`
  - `3311-3387`
- replay/artifact parsing downstream:
  - `src-tauri/src/run_artifacts.rs:2146`
- bench and smoke-fixture construction:
  - `crates/app_core/benches/options_expression_bench_support.rs:76-254`
  - `src-tauri/src/bin/ta_options_expression_smoke.rs:150-397`

### Downstream verification impact if changed

- changing request defaults or tuple invariants affects:
  - options-expression feasibility analysis
  - ranking/exactification behavior
  - replayed artifact determinism
- changing persisted manifest or artifact field semantics affects:
  - replay cache load/upgrade paths
  - Tauri artifact inspection and provenance checks
  - old JSON/binary payload compatibility
- changing backend labels or verdict semantics affects:
  - `options_expression.rs`
  - `commands_options_expression.rs`
  - persisted artifacts already written to disk

## Standards And Research Log

This packet was written after targeted web research focused on Rust public-API discipline for schema files, Serde contract strictness/default behavior, enum wire-format guidance, and the documentation bar expected for public persisted DTOs.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file is part of the re-exported public contract surface
   - Standard informed:
     - public APIs should make validation, future-proofing, and documentation deliberate

2. Rust API Guidelines type-safety guidance
   - Source: <https://rust-lang.github.io/api-guidelines/type-safety.html#newtypes-provide-static-distinctions-c-newtype>
   - Why it was consulted:
     - several externally meaningful discriminators are still raw `String`
   - Standard informed:
     - closed or semantically distinct identifiers should prefer typed representations over ad hoc strings when practical

3. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the file exposes many public DTOs and helper methods with no rustdoc
   - Standard informed:
     - public items should document usage, error semantics, and examples where compatibility-sensitive behavior exists

4. Rust API Guidelines interoperability guidance, `C-GOOD-ERR`
   - Source: <https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err>
   - Why it was consulted:
     - this schema influences how downstream code can distinguish contract failures versus fallback/default behavior
   - Standard informed:
     - meaningful, typed contracts are preferable to overly collapsed stringly semantics

5. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - this file heavily uses `#[serde(default)]` and `skip_serializing_if`
   - Standard informed:
     - field defaults are powerful compatibility tools, but they deliberately change deserialize behavior and should be applied only where that behavior is intended

6. Serde container attributes
   - Source: <https://serde.rs/container-attrs.html>
   - Why it was consulted:
     - major persisted DTOs currently accept unknown fields because they do not use strict container attributes such as `deny_unknown_fields`
   - Standard informed:
     - container-level strictness is an explicit contract choice and should be deliberate for persisted schema

7. Serde enum representations
   - Source: <https://serde.rs/enum-representations.html>
   - Why it was consulted:
     - backend and artifact identity values are modeled as raw strings even though they come from small closed sets
   - Standard informed:
     - enum-backed wire formats are preferable when a protocol field is conceptually closed and versioned

## Findings

## Correctness And Safety

### 1. Several required persisted collections silently deserialize to empty state

- Evidence:
  - `ImportedOptionsDataset.contracts` uses `#[serde(default)]`:
    - `crates/app_core/src/options_types.rs:390-391`
  - `OptionsSignalEventManifestV1.events` uses `#[serde(default)]`:
    - `450-451`
  - `OptionsExpressionEventPathInspectionResult.points` uses `#[serde(default)]`:
    - `730-731`
  - `OptionsExpressionRunResult.ranked_results` uses `#[serde(default)]`:
    - `1096-1097`
  - reason vectors and similar summaries also default to empty:
    - `909-929`
    - `989-990`
- Why this matters:
  - malformed, truncated, or drifted payloads can deserialize into valid-looking but semantically incomplete objects rather than failing fast
  - that is especially risky for replay caches and persisted artifacts
- Smallest justified fix:
  - keep empty defaults only where backward compatibility truly requires them
  - for core persisted payloads, either remove the default from required collections in the next schema revision or add explicit loader-level validation that rejects unexpectedly empty mandatory data

### 2. `validation_verdict` is persisted state even though it is recomputed from other fields

- Evidence:
  - the field is stored directly on `OptionsExpressionTupleResult`:
    - `543-545`
  - helper logic recomputes it from validation metrics:
    - `564-613`
  - the main engine refreshes persisted verdicts after ranking:
    - `crates/app_core/src/options_expression.rs:2908-2913`
  - the Tauri side refreshes them again after artifact load:
    - `src-tauri/src/commands_options_expression.rs:1131-1134`
- Why this matters:
  - the DTO has two sources of truth
  - hand-authored fixtures, legacy artifacts, or stale caches can carry a verdict that no longer matches the metrics payload
- Smallest justified fix:
  - document `validation_verdict` as derived data and require a refresh-on-load policy everywhere it is consumed
  - longer-term, consider removing the stored field from the next schema version and deriving it on demand

## API And Naming

### 3. Multiple externally meaningful discriminators are still plain `String`

- Evidence:
  - provenance and run-kind fields:
    - `OptionsSignalEvent.parent_run_kind`: `398`
    - `OptionsSignalEvent.adjustment_mode`: `405`
    - `OptionsSignalEventManifestV1.parent_run_kind`: `437`
    - `OptionsSignalEventManifestV1.adjustment_mode`: `444`
  - backend-selection/reporting fields:
    - `OptionsExpressionAutoBackendDecision.recommended_backend`: `987`
    - `OptionsExpressionFeasibilityReport.best_available_backend`: `998`
    - `OptionsExpressionRunResult.backend_used`: `1083`
    - `OptionsExpressionArtifactV1.kind`: `1103`
    - `OptionsExpressionArtifactV1.backend_used`: `1110`
  - those values are produced elsewhere as string literals:
    - `crates/app_core/src/options_expression.rs:2297-2300`
    - `2363-2365`
    - `3809-3835`
    - `5533-5555`
- Why this matters:
  - these fields are effectively protocol discriminators
  - typo drift or inconsistent naming propagates into persistence, replay, reporting, and UI consumers
- Smallest justified fix:
  - introduce enums or newtypes for closed sets such as backend labels and artifact kinds
  - if schema migration is not yet justified, at minimum centralize shared constants and document the accepted values

### 4. Omitted request fields deserialize into active execution policy, not just convenience defaults

- Evidence:
  - `OptionsExpressionRunRequest` uses defaults for structure family, backend preference, bucket lists, exit template, fill model, filters, top-k, split ratios, split mode, and sealed finalists:
    - `1039-1077`
  - file-local tests assert this behavior:
    - `1189-1210`
  - downstream request validation and normalization live in the engine, not in the DTO:
    - `crates/app_core/src/options_expression.rs:1755-2043`
- Why this matters:
  - partially specified requests do not mean “unknown”; they mean “apply current code defaults”
  - replayed or hand-authored JSON can therefore shift semantics when defaults change over time
- Smallest justified fix:
  - keep the behavior for backward compatibility, but document it explicitly and add a normalization helper or constructor that makes the implied policy concrete before persistence

## Error Handling

### 5. Major persisted DTOs are permissive on deserialize and do not reject unknown fields

- Evidence:
  - representative persisted/public structs use serde derives but no strict container attributes:
    - `OptionsDatasetManifestV1`: `313-349`
    - `OptionsSignalEventManifestV1`: `433-451`
    - `OptionsExpressionFeasibilityReport`: `994-1037`
    - `OptionsExpressionRunRequest`: `1039-1077`
    - `OptionsExpressionRunResult`: `1081-1098`
    - `OptionsExpressionArtifactV1`: `1101-1123`
  - the same structs also use many defaults, so missing or renamed fields may deserialize successfully rather than surfacing contract drift
- Why this matters:
  - persistence/replay code loses some ability to distinguish “older but supported” from “unexpectedly malformed”
  - this is particularly relevant for cache corruption and hand-edited debug artifacts
- Smallest justified fix:
  - decide per-DTO whether permissive compatibility is intentional
  - for next-version persisted schemas, consider `deny_unknown_fields` or explicit post-deserialize validators on the highest-risk payloads

## Module Boundaries And Technical Debt

### 6. The file combines too many unrelated schema families in one contract hub

- Evidence:
  - dataset import schema: `313-391`
  - signal manifest schema: `395-451`
  - tuple/inspection schema and verdict helpers: `455-749`
  - feasibility/report schema: `753-1037`
  - run/artifact schema: `1039-1123`
  - tests: `1127-1398`
- Why this matters:
  - low compute cost does not mean low maintenance cost
  - unrelated changes to dataset manifests, artifact schema, or inspection DTOs all collide in one file with one blast radius
- Recommended action:
  - defer a split into submodules until after the review campaign unless a concrete change forces it
  - if later split, keep the public re-export surface stable

### 7. Legacy-compatible numeric defaults collapse “missing” and “zero” into the same value

- Evidence:
  - `OptionsExpressionMetrics.mean_return_on_premium` and `return_volatility_on_premium` default to `0.0`:
    - `477-479`
  - file-local compatibility test locks in that behavior:
    - `1299-1320`
- Why this matters:
  - the contract can no longer distinguish a true measured zero from an omitted legacy field
  - that is acceptable for backward compatibility, but it is still semantic debt
- Smallest justified fix:
  - retain current behavior for `V1` compatibility
  - document the meaning clearly, and use `Option<f64>` in a future schema version if “unknown versus zero” becomes operationally important

## Docs And Examples

### 8. The public schema surface has no rustdoc at all

- Evidence:
  - a repo search for `^///` in `crates/app_core/src/options_types.rs` returns no matches
  - the file exposes public enums/structs across `104-1123`
- Why this matters:
  - this is the canonical contract file for persisted JSON/bincode-like payloads and Tauri IPC results
  - callers cannot tell which fields are compatibility shims, which are derived, which are required, or which defaults are semantically active
- Smallest justified fix:
  - add rustdoc to the highest-blast-radius types first:
    - `OptionsDatasetManifestV1`
    - `OptionsSignalEventManifestV1`
    - `OptionsExpressionRunRequest`
    - `OptionsExpressionRunResult`
    - `OptionsExpressionArtifactV1`
    - `OptionsExpressionTupleResult`

## Tests And Missing Verification

### 9. The file-local tests are useful but still leave contract-drift gaps

- Evidence:
  - current tests focus on request defaults, verdict inference, fill-model compatibility, and legacy metric defaults:
    - `1189-1398`
  - downstream Tauri replay tests cover cache and provenance round-trips:
    - `src-tauri/src/commands_options_expression.rs:2453-2479`
    - `3311-3387`
- Missing verification:
  - no file-local test that proves whether missing required collections should be accepted or rejected
  - no file-local test that demonstrates unknown-field behavior for persisted schemas
  - no explicit test for stale persisted `validation_verdict` payloads versus inferred verdict refresh
  - no schema-focused test matrix for backend label drift
- Smallest justified fix:
  - add a narrow serde-contract test set around malformed-but-deserializable payloads before touching schema strictness

## Performance, Allocation, And CUDA Concerns

### 10. This is not a meaningful compute hotspot, so performance churn here should stay secondary

- Evidence:
  - the file is almost entirely DTO declarations and small helper methods
  - no heavy allocation loops, CUDA launches, or algorithmic kernels live here
- Review conclusion:
  - no benchmark-sensitive file-local performance change is justified from this review alone
  - the real performance concern is semantic drift in request/result schema, not raw runtime cost

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc to the public persisted DTOs and derived-verdict helper methods
  - centralize backend label and artifact-kind constants if a schema migration is not yet justified
  - add DTO-local `validate()` or `normalize()` helpers for high-risk request invariants
- Defer:
  - full enum/newtype migration for persisted discriminator fields if it would require broad artifact migration in one step

## Technical-Debt Reduction Register

- Safe now:
  - document which defaults are compatibility shims versus active product policy
  - document `validation_verdict` as derived state
- Defer:
  - splitting the file into multiple schema modules
  - changing legacy metric fields from `f64` to `Option<f64>` until migration strategy is explicit
- Do not casually remove:
  - backward-compatible deserialize defaults that are still needed for old artifacts or test fixtures

## Performance Opportunity Register

- Safe structural work:
  - none material beyond reducing protocol drift and duplicated string-label handling
- Benchmark-sensitive or semantics-sensitive:
  - any schema change that alters what a missing field means
  - any persisted enum/newtype migration that could affect replay compatibility

## Recommended Action

`Small surgical refactor`

This file does not need algorithmic work. It needs contract hardening:

- document the public schema
- reduce stringly typed protocol fields where justified
- make permissive serde behavior explicit instead of accidental
- keep backward-compatible defaults only where they are intentionally part of the persistence contract

## Exact Local Verification Required If Changed

1. File-local contract tests:
   - `cargo test -p ta_app_core options_types::tests:: -- --nocapture`
2. Compile the app-core test surface:
   - `cargo check -p ta_app_core --tests`
3. Tauri-side replay cache and provenance tests that directly consume these DTOs:
   - `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::signal_manifest_replay_cache_round_trips_to_disk -- --nocapture`
   - `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::options_dataset_replay_cache_round_trips_to_disk -- --nocapture`
   - `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::saved_artifact_replay_provenance_accepts_matching_dataset_hash_and_semantics -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core options_types::tests:: -- --nocapture`
  - `cargo check -p ta_app_core --tests`
  - `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::signal_manifest_replay_cache_round_trips_to_disk -- --nocapture`
  - `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::options_dataset_replay_cache_round_trips_to_disk -- --nocapture`
  - `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::saved_artifact_replay_provenance_accepts_matching_dataset_hash_and_semantics -- --nocapture`
- Blocked by unrelated workspace/nightly state when attempted at the package level:
  - `cargo test -p ta_desktop_demo_app commands_options_expression::tests::signal_manifest_replay_cache_round_trips_to_disk -- --nocapture`
  - current unrelated blockers observed during that broader invocation:
    - `src-tauri/src/bin/ta_app_path_faceoff.rs:1036` type mismatch
    - `src-tauri/tests/validation_xlsx_report_tests.rs:3` could not resolve `ta_desktop_demo_app`
    - nightly ICE in `src-tauri/src/bin/ta_double_ma_validation_cost.rs`

## Regression Risk And Feature-Preservation Constraints

- Do not narrow the deserialize contract for persisted artifacts or replay caches without an explicit migration story
- Do not change default request semantics casually; missing-field behavior is part of current product behavior
- Do not rename backend labels or artifact kinds without checking replay code, report generation, and on-disk artifact readers
- Prefer additive hardening and explicit documentation over schema churn
