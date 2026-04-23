# Review Packet: `crates/app_core/src/indicator_catalog.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/indicator_catalog.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and largely sound public metadata/validation surface, but not fully public-review ready because the public schema advertises parameter step metadata that is not enforced or documented as UI-only, duplicate-schema drift is detected later than necessary, and MA-policy-derived GPU hint fields are not described clearly enough for downstream consumers`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Strategy-compile-critical`, `Semantics-sensitive`

## What This File Owns

`indicator_catalog.rs` is the Rust-side canonical metadata and validation layer for dynamic indicators.

It owns:

- serde-facing DTOs for indicator params, outputs, input kinds, and normalized descriptors:
  - `IndicatorParamKind`, `IndicatorParamValue`, `IndicatorValueType`, `IndicatorInputKind`: `5-35`
  - `IndicatorParamDescriptor`, `IndicatorOutputDescriptor`, `IndicatorParamKV`, `IndicatorDescriptor`: `37-89`
- projection from VectorTA registry metadata into app-core descriptors:
  - `normalize_dynamic_indicator_descriptor`: `135-194`
- cached listing and case-insensitive lookup:
  - `list_dynamic_indicator_descriptors`: `310-315`
  - `DYNAMIC_INDICATOR_DESCRIPTORS_CACHE`: `318`
  - `list_dynamic_indicator_descriptors_cached`: `320-324`
  - `find_dynamic_indicator_descriptor`: `326-336`
- parameter normalization / validation:
  - `normalize_and_validate_indicator_params`: `338-441`
- output-ref normalization / validation:
  - `normalize_and_validate_indicator_output_ref`: `443-493`
- module-local tests:
  - `500+`

This file does not execute indicators. It defines the contract that compile-time and runtime consumers rely on before they call CPU or CUDA indicator execution.

## Classification

- Public surface: yes
  - re-exported from `crates/app_core/src/lib.rs:174-179`
  - exposed through Tauri command wiring in `src-tauri/src/commands_data.rs:85-87`
- Hot path: no
  - listing and validation run on compile/dispatch paths rather than numeric inner loops
- CUDA/runtime-critical: indirectly
  - catalog validation and GPU-policy hints directly influence CPU vs GPU route choice for dynamic indicators
- Strategy-local: no
  - shared infrastructure consumed by IR validation, plan compilation, adapter dispatch, and strategy-local descriptor lookups
- Low-risk support code: moderate
  - low computational cost, but high contract importance

## Direct Interactions

### Backend interaction map

- registry source of truth:
  - `vector_ta::indicators::registry` via `135-194` and `310-315`
- MA-policy-derived GPU hints:
  - `crates/app_core/src/ma_policy.rs:4-76`, `142-162`
  - consumed in this file at `188-189`
- dynamic IR validation:
  - `crates/app_core/src/strategy_ir.rs:375-391`
- dynamic plan route selection:
  - `crates/app_core/src/strategy_plan.rs:137-150`, `217-226`
- runtime adapter validation:
  - `crates/app_core/src/dynamic_indicator_adapter.rs:191-209`, `1162-1187`, `1230-1247`
- Tauri export path:
  - `src-tauri/src/commands_data.rs:85-87`
- additional descriptor consumers:
  - representative callers include `double_ma.rs:9367-9403`, `strategy_dynamic.rs:183-192`, `composite_cpu.rs:2113-2148`, `asr.rs:1076-1078`

### Downstream verification impact if changed

- DTO shape changes affect:
  - Tauri command payloads
  - frontend indicator catalog normalization
  - any external consumer of re-exported descriptors/validation helpers
- validation behavior changes affect:
  - dynamic strategy IR validation
  - plan compilation and route selection
  - CPU/CUDA indicator adapter dispatch

## Standards And Research Log

This packet was written after targeted web research focused on public Rust DTO/API design, documentation of validation behavior, naming clarity for exported capability flags, `OnceLock` cache semantics, and test posture for public normalization helpers.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines public DTOs and public validation helpers
   - Standard informed:
     - public APIs should make contracts, naming, and failure behavior explicit

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - exported capability fields and validation behavior are visible outside the module
   - Standard informed:
     - public items should document what is guaranteed, what is normalized, and what can fail

3. Rust API Guidelines, Naming
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - the file exposes several capability-hint field names whose meanings are slightly transformed from underlying registry names
   - Standard informed:
     - names should reflect actual semantics and avoid ambiguous generality where possible

4. `std::sync::OnceLock` documentation
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - the cached descriptor list is a global immutable `OnceLock<Vec<_>>`
   - Standard informed:
     - caching here is appropriate as long as the data is immutable and startup-time registry capture is intended

5. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this module returns rich but stringly-typed validation errors
   - Standard informed:
     - string errors are tolerable for thin boundary helpers, but the contract should still be deliberate and stable enough for callers

6. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - this file’s public validation helpers are small and should have focused unit tests for failure cases
   - Standard informed:
     - boundary behavior should be pinned directly where the validation logic lives

## Findings

## Correctness And Safety

### 1. The public schema exposes `step`, but validation never enforces it

- Evidence:
  - `IndicatorParamDescriptor` publicly carries `step: Option<f64>`:
    - `crates/app_core/src/indicator_catalog.rs:44-54`
  - registry projection preserves `p.step`:
    - `159-161`
  - numeric validation checks finiteness and min/max bounds, but not step granularity:
    - `196-215`
    - `217-303`
  - parameter normalization path does not add any step enforcement:
    - `338-441`
- Why it matters:
  - callers can reasonably infer that `step` is part of the validation contract because it is present in the public schema next to `min` and `max`
  - today the field functions as metadata only, but that is not documented
- Smallest justified fix:
  - either:
    - enforce step granularity during numeric normalization
  - or:
    - document clearly that `step` is UI guidance only and intentionally not part of Rust-side validation
- Review judgment:
  - medium correctness/contract issue
  - semantics-sensitive if enforcement is added, but the current ambiguity should not remain implicit

### 2. Duplicate-schema-key drift is detected later than necessary

- Evidence:
  - duplicate-schema-key detection happens only after required/default processing and final sorting:
    - `crates/app_core/src/indicator_catalog.rs:377-438`
  - the actual duplicate-schema check is:
    - `433-438`
- Why it matters:
  - if registry metadata ever drifts into duplicate normalized parameter keys, callers can receive a missing/default-related error before the intended schema-drift error
- Smallest justified fix:
  - pre-scan the descriptor schema for duplicate normalized keys before processing user/default values
- Review judgment:
  - low-to-medium correctness / diagnostics issue
  - safe-now

## API And Naming

### 1. GPU capability hint fields are useful, but the file does not document that some are MA-policy-derived rather than raw registry truth

- Evidence:
  - `supports_cuda_sweep` is populated from registry `supports_cuda_batch`:
    - `crates/app_core/src/indicator_catalog.rs:186`
  - `supports_gpu_kernel` and `prefers_gpu_by_speedup` are derived from `ma_policy` rather than raw registry data:
    - `188-189`
- Why it matters:
  - downstream callers may read these fields as canonical registry capabilities rather than app-core route-policy hints
  - that distinction matters in plan compilation:
    - `crates/app_core/src/strategy_plan.rs:137-150`
- Smallest justified fix:
  - add doc comments clarifying which fields are registry-backed and which are app-core policy hints
- Review judgment:
  - medium API/documentation issue
  - safe-now

### 2. The exported DTOs are stable enough to use, but they would benefit from more explicit contract comments

- Evidence:
  - public DTOs and helper functions are exported without field-level or function-level rustdoc:
    - `37-89`
    - `310-336`
    - `338-493`
- Why it matters:
  - these DTOs cross the Tauri/app-core boundary and feed frontend normalization
  - silent semantic assumptions accumulate when fields like `requires_candles`, `supports_gpu_kernel`, `prefers_gpu_by_speedup`, and `output_count` are not described where they are defined
- Smallest justified fix:
  - add concise rustdoc to DTOs and public normalization helpers

## Error Handling

### 1. String errors are acceptable here, but the boundary is intentionally stringly-typed and should stay deliberate

- Evidence:
  - parameter normalization and output-ref normalization return `Result<_, String>`:
    - `338-441`
    - `443-493`
  - downstream callers either wrap the strings with more context or map them into typed errors:
    - `crates/app_core/src/strategy_ir.rs:382-391`
    - `crates/app_core/src/dynamic_indicator_adapter.rs:197-207`
- Why it matters:
  - the current approach is workable because this module is mainly a validation facade, but callers may start depending on specific message text if there is no stronger error contract
- Smallest justified fix:
  - do not force a typed-error rewrite now
  - instead, document message intent and avoid needless churn in existing error wording
- Review judgment:
  - acceptable as-is for now
  - document-only unless a wider typed-error campaign is approved

## Module Boundaries And Technical Debt

### 1. The file currently mixes registry projection, exported DTOs, validation, and route-hint decoration

- Evidence:
  - registry projection:
    - `135-194`
  - public DTO definitions:
    - `37-89`
  - validation helpers:
    - `338-493`
  - route-hint decoration:
    - `188-189`
- Why it matters:
  - the file is still manageable, but it owns multiple layers of responsibility
  - strategy-local modules also repeat small descriptor-resolution helpers instead of leaning on richer shared helpers from this file
- Smallest justified fix:
  - safe-now:
    - document the ownership split
  - defer-later:
    - factor repeated output-id / param-key helper logic into shared catalog utilities if repetition keeps growing
- Review judgment:
  - moderate tech-debt issue
  - defer broad refactors

### 2. The final schema-order sort is redundant work

- Evidence:
  - `normalized` is already produced by iterating `descriptor.params` in order:
    - `377-414`
  - it is then re-sorted back into the same order:
    - `418-430`
- Why it matters:
  - this is small cost, not a hot-path problem, but it is unnecessary code and unnecessary work in a public helper
- Smallest justified fix:
  - remove the final sort unless a caller depends on it as defensive normalization against future construction changes
- Review judgment:
  - low-severity code-quality/perf issue
  - safe-now

## Docs And Examples

### 1. The catalog contract needs documentation on what is metadata versus enforced validation

- Evidence:
  - the file currently does not explain:
    - whether `step` is enforced
    - which capability fields are hints versus hard runtime capability
    - whether cached descriptors are intended to reflect live registry mutations after first initialization
- Smallest justified fix:
  - add rustdoc to:
    - `IndicatorDescriptor`
    - `IndicatorParamDescriptor`
    - `list_dynamic_indicator_descriptors_cached`
    - `normalize_and_validate_indicator_params`
    - `normalize_and_validate_indicator_output_ref`

## Tests And Missing Verification

### 1. Existing local coverage is solid on smoke cases, but still misses several public-contract edges

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core indicator_catalog -- --nocapture`
      - passed (`10` tests)
    - `cargo test -p ta_desktop_demo_app list_dynamic_indicators_returns_only_dynamic_strategy_eligible_descriptors -- --nocapture`
      - passed
    - `cargo test -p ta_app_core compile_gpu_plan_routes_dynamic_indicator_to_gpu_when_policy_prefers_gpu -- --nocapture`
      - passed
    - `cargo test -p ta_app_core compile_gpu_plan_routes_dynamic_indicator_to_cpu_when_policy_prefers_cpu -- --nocapture`
      - passed
  - current file-local tests cover:
    - descriptor count/shape
    - cached lookup
    - unknown params
    - case-insensitive keys
    - int/float coercion
    - output-ref behavior
    - `500-600`
- Missing tests I would want before changing the file:
  - step-granularity behavior
  - min/max rejection
  - non-finite float rejection
  - duplicate user keys
  - empty parameter keys
  - missing required params
  - duplicate schema-key drift
  - invalid single-output override wording

## Performance, Allocation, And Runtime Concerns

### 1. This file is not a real performance hotspot, so do not over-optimize it

- Evidence:
  - the heavy work happens in compile/runtime callers and indicator execution, not here
  - most operations are descriptor lookups, map/set handling, and schema normalization
- Review judgment:
  - keep performance discussion proportional
  - the only obvious safe cleanup is removing the redundant final sort

### 2. `OnceLock` cache use is appropriate here

- Evidence:
  - immutable cache:
    - `crates/app_core/src/indicator_catalog.rs:318-324`
- Review judgment:
  - this is a strength, not a liability
  - do not destabilize it without a real need for registry hot-reload semantics

## Code-Quality Improvement Register

- Safe now:
  - enforce or explicitly document `step`
  - pre-scan duplicate schema keys
  - remove the redundant final sort
  - add DTO/helper rustdoc
- Coordination required:
  - none large, unless callers already depend on current `step` being metadata-only
- Defer until later pass:
  - extracting richer shared descriptor utility helpers for strategy-local callers

## Technical-Debt Reduction Register

- Safe to reduce now:
  - metadata-vs-validation ambiguity around `step`
  - redundant final sort
  - missing docs for route-policy hint fields
- Coordination required:
  - any decision to make `step` enforcement strict if frontend or saved artifacts currently emit off-step values
- Useful debt that should not be “cleaned up” casually:
  - the `OnceLock` cache
  - string-based validation errors unless the wider caller graph is ready for typed errors

## Performance Opportunity Register

- Safe structural improvements:
  - remove the redundant final sort
- Benchmark-sensitive candidates:
  - none meaningful in this file
- Not the right place to chase performance:
  - broad cache redesign
  - allocator churn work

## Observability And Diagnostics Gaps

- the file does not document whether `step` is enforced
- MA-policy-derived GPU hint fields are not labeled as such
- duplicate-schema drift errors can be masked by earlier required/default flow

## Implementation Staging Guidance

- Safe now:
  - docs
  - step-policy clarification
  - duplicate-schema pre-scan
  - redundant-sort removal
- Coordination required:
  - only if strict `step` enforcement would reject currently accepted inputs
- Defer until later pass:
  - broader shared-helper extraction

## Change Blast Radius

- Local only:
  - rustdoc
  - redundant-sort removal
  - duplicate-schema pre-scan
- Same subsystem:
  - step enforcement/documentation
  - capability-hint documentation
- Cross-backend / frontend-backend contract:
  - DTO field rename or serialized-shape change

## Open Questions Or Assumptions

- assumption:
  - `step` was intended to be part of the public parameter contract rather than a purely frontend decoration field
- open question:
  - whether any existing frontend or artifact replay path relies on accepting off-step numeric values today

## Recommended Action

`Small surgical refactor`

The next pass should:

- decide and document the `step` contract
- tighten duplicate-schema drift handling
- keep the rest of the public surface stable

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core indicator_catalog -- --nocapture`
- `cargo test -p ta_desktop_demo_app list_dynamic_indicators_returns_only_dynamic_strategy_eligible_descriptors -- --nocapture`
- `cargo test -p ta_app_core compile_gpu_plan_routes_dynamic_indicator_to_gpu_when_policy_prefers_gpu -- --nocapture`
- `cargo test -p ta_app_core compile_gpu_plan_routes_dynamic_indicator_to_cpu_when_policy_prefers_cpu -- --nocapture`
- add and run focused tests for:
  - step enforcement or explicit non-enforcement
  - duplicate schema-key drift
  - min/max rejection
  - non-finite floats
  - missing required params

## Regression Risk And Feature-Preservation Constraints

- do not rename or reshape exported DTO fields casually
- if `step` becomes enforced, treat that as a deliberate contract tightening and verify frontend/request-builder compatibility
- preserve the existing case-insensitive lookup and normalization behavior unless a product-level contract change is approved
