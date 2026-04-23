# Review Packet: `crates/app_core/src/ma_policy.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/ma_policy.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally useful and locally well-tested, but not fully public-review ready because route preference semantics depend on an embedded CSV override path with weak fallback validation, several public route-planning types appear to be internal-only, and the file does not document its policy sources or feature-selection semantics`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Policy-critical`, `Semantics-sensitive`, `Support code`, `Low-runtime-risk`

## What This File Owns

`ma_policy.rs` owns moving-average backend policy for the Double-MA family and adjacent metadata surfaces.

It owns:

- the hardcoded GPU-kernel support list:
  - `crates/app_core/src/ma_policy.rs:5-39`
- the built-in GPU-preferred fallback list:
  - `43-76`
- the embedded `gpu_speedups.csv` loading and parse/cache path:
  - `78-133`
- support/preference helpers:
  - `supports_gpu_kernel_ma`: `142-144`
  - `prefers_gpu_kernel_ma`: `147-157`
  - `prefers_gpu_by_speedup_table`: `160-162`
  - `prefers_cpu_by_speedup_table`: `165-167`
- capability and route enums / structs:
  - `HybridCapabilityTier`: `169-175`
  - `HybridRouteTier`: `177-182`
  - `HybridIndicatorRouteDecision`: `184-190`
  - `DoubleMaHybridRoutePlan`: `192-216`
  - `HybridIndicatorPartition`: `218-236`
- capability classification and route planning:
  - `classify_hybrid_capability`: `239-259`
  - `plan_hybrid_indicator_partition`: `261-285`
  - `plan_hybrid_indicator_route`: `288-314`
  - `plan_double_ma_hybrid_route`: `317-322`
  - `plan_double_ma_hybrid_partition`: `325-327`
- file-local tests:
  - `329-475`

This file does not own:

- MA-kind validation:
  - `crates/app_core/src/double_ma_ma_kind.rs`
- strategy validation:
  - `crates/app_core/src/double_ma_strategy.rs`
- actual optimizer or validation execution:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/double_ma_validate.rs`
- UI serialization / registry transport:
  - `src-tauri/src/ma_registry.rs`

## Classification

- Public surface: yes
  - the module is publicly exposed from `crates/app_core/src/lib.rs:40`
- Hot path: no
  - this is not a numeric execution path
- CUDA/runtime-critical: no
  - it influences route choice, but it is not runtime/CUDA orchestration code
- Strategy-local: yes
  - it is scoped to moving-average routing policy for Double-MA-family flows and related metadata
- Low-risk support code: no
  - it is support code, but changing it changes backend-selection behavior and UI capability metadata

## Direct Interactions

### Backend interaction map

- crate export:
  - `crates/app_core/src/lib.rs:40`
- Tauri MA registry metadata:
  - `src-tauri/src/ma_registry.rs:24-31`
- indicator catalog backend metadata:
  - `crates/app_core/src/indicator_catalog.rs:183-189`
- Double-MA backend capability/blocker reporting:
  - `crates/app_core/src/double_ma_backend.rs:68-77`
  - `169-180`
- Double-MA optimization route selection:
  - `crates/app_core/src/double_ma.rs:5161-5169`
  - `12391`
  - `12965-12970`
- Double-MA validation route selection:
  - `crates/app_core/src/double_ma_validate.rs:963-979`

### Downstream verification impact if changed

- support/preference boolean changes affect:
  - UI registry metadata in `ma_registry.rs`
  - indicator catalog metadata in `indicator_catalog.rs`
- route-tier or capability changes affect:
  - optimization route selection in `double_ma.rs`
  - validation route selection in `double_ma_validate.rs`
  - capability summaries and fallback-reason surfaces in `double_ma_backend.rs`

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API documentation, lazy static initialization, embedded-file semantics, deterministic ordered-set behavior, and CSV-format expectations relevant to the custom speedup-table parser.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public helpers, enums, route structs, and planners
   - Standard informed:
     - public APIs should be intentional and documented, especially when they encode policy rather than pure computation

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public policy types and helpers currently have no rustdoc
   - Standard informed:
     - public functions and types should explain semantics, not just expose names

3. `OnceLock` documentation
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - the speedup-table parse result is cached in a `OnceLock`
   - Standard informed:
     - lazy one-time initialization is appropriate here, but initialization semantics and failure behavior should remain explicit

4. `include_str!` documentation
   - Source: <https://doc.rust-lang.org/std/macro.include_str.html>
   - Why it was consulted:
     - the speedup CSV is embedded at compile time
   - Standard informed:
     - embedded-file behavior is compile-time/static, which makes parse robustness and drift detection more important because there is no runtime refresh path

5. `BTreeSet` documentation
   - Source: <https://doc.rust-lang.org/std/collections/struct.BTreeSet.html>
   - Why it was consulted:
     - the partition planner uses `BTreeSet` for deduplication and deterministic order
   - Standard informed:
     - ordered iteration is a valid reason to prefer `BTreeSet` here and should be preserved if the implementation is refactored

6. RFC 4180, Common Format and MIME Type for CSV Files
   - Source: <https://www.rfc-editor.org/rfc/rfc4180>
   - Why it was consulted:
     - this file implements a custom CSV parser rather than using a library
   - Standard informed:
     - if a custom parser is kept, it should be clear which subset of CSV it supports and how malformed input is handled

## Findings

## Correctness And Safety

### 1. Any non-empty parsed speedup table becomes authoritative, even if it is partial or malformed

- Evidence:
  - the embedded CSV is loaded at compile time:
    - `crates/app_core/src/ma_policy.rs:78`
  - it is parsed into a set of first-column indicator ids:
    - `109-133`
  - `prefers_gpu_kernel_ma` treats any non-empty parsed set as authoritative and bypasses the built-in fallback list entirely:
    - `147-156`
  - route planning uses that preference signal to decide `GpuKernel` versus `CpuStaged` for kernel-capable MAs:
    - `291-299`
  - downstream route selection depends on those decisions:
    - `crates/app_core/src/double_ma.rs:5161-5169`
    - `12965-12970`
    - `crates/app_core/src/double_ma_validate.rs:963-979`
- Why it matters:
  - a truncated or partially malformed embedded table would silently change backend-preference behavior without falling back to the curated static list
  - this is a policy-correctness issue, not just a documentation issue, because it can alter route choice and UI preference reporting
- Smallest justified fix:
  - keep the embedded-table design
  - add explicit validation invariants before using the parsed set as authoritative
  - if the parsed table fails those invariants, fall back to `GPU_KERNEL_PREFERRED_MAS` instead of silently using a partial set
- Review judgment:
  - medium-high correctness/policy issue
  - safe-now if implemented as validation + fallback, not policy redesign

## API And Naming

### 1. `prefers_cpu_by_speedup_table` overstates “not GPU-preferred” as “CPU-preferred”

- Evidence:
  - the function is just logical negation:
    - `crates/app_core/src/ma_policy.rs:165-167`
  - it returns `true` even for unknown ids, and the file-local tests lock that behavior in:
    - `426-428`
  - there are no in-repo consumers outside the file itself and its tests:
    - search results show only:
      - `crates/app_core/src/ma_policy.rs:165`
      - `427`
- Why it matters:
  - unsupported or unmeasured is not the same thing as affirmatively CPU-preferred
  - the current name invites misuse if a future caller treats it as stronger evidence than the implementation actually provides
- Smallest justified fix:
  - strongest low-churn option:
    - make it private or remove it if no external caller needs it
  - if it must remain:
    - rename or document it as “not GPU-preferred by current speedup table”
- Review judgment:
  - medium API/naming issue
  - safe-now if there is no crate-external dependency

### 2. The public surface is broader than current consumer usage appears to require

- Evidence:
  - the module itself is public:
    - `crates/app_core/src/lib.rs:40`
  - the following policy types and planners are public:
    - `crates/app_core/src/ma_policy.rs:169-327`
  - in-repo consumers of those route-tier types and planners are internal:
    - `crates/app_core/src/double_ma_backend.rs`
    - `crates/app_core/src/double_ma.rs`
    - `crates/app_core/src/double_ma_validate.rs`
  - Tauri/UI-facing consumers only appear to need the boolean helpers:
    - `src-tauri/src/ma_registry.rs:24-31`
    - `crates/app_core/src/indicator_catalog.rs:183-189`
- Why it matters:
  - a broad public surface creates unnecessary compatibility burden around policy structs that look internal to app-core orchestration
- Smallest justified fix:
  - audit whether crate-external callers actually use the route-tier enums/structs
  - if not, narrow those items to `pub(crate)` and keep only the intentionally public helpers public
- Review judgment:
  - medium API-surface issue
  - semantics-sensitive if external users already depend on the current shape

## Error Handling

### 1. The file has no error channel for malformed speedup-table content, so policy degradation is silent

- Evidence:
  - `parse_gpu_speedup_ids` always returns a set and never reports malformed rows:
    - `crates/app_core/src/ma_policy.rs:109-130`
  - `gpu_speedup_indicator_ids` caches that result permanently:
    - `132-134`
  - `prefers_gpu_kernel_ma` only distinguishes empty versus non-empty parsed results:
    - `152-156`
- Why it matters:
  - with an embedded static file, silent policy degradation is harder to notice than runtime parse failure
- Smallest justified fix:
  - add parse/result validation at init time and keep the current zero-error outward API
  - this does not require introducing `Result` into public helper signatures
- Review judgment:
  - medium error-handling/robustness issue

## Module Boundaries And Technical Debt

### 1. Route policy is split across three static sources plus the live registry, which creates drift risk

- Evidence:
  - hardcoded kernel-support list:
    - `crates/app_core/src/ma_policy.rs:5-39`
  - hardcoded preferred fallback list:
    - `43-76`
  - embedded speedup CSV:
    - `78-133`
  - live registry capability lookup:
    - `244-246`
- Why it matters:
  - the file is maintaining multiple sources of truth:
    - kernel capability
    - preference
    - registry-discovered CPU/CUDA support
  - these are all reasonable inputs, but without explicit drift checks they can diverge quietly over time
- Smallest justified fix:
  - keep the current design
  - add one or two drift-detection tests that validate policy invariants against the embedded table and registry shape
- Review judgment:
  - medium technical-debt issue

### 2. The top-level support-list comment is stale relative to the actual workspace dependency

- Evidence:
  - the comment says:
    - `Kernel-capable MA ids aligned with vector-ta 0.1.2 VRAM kernel support.`
    - `crates/app_core/src/ma_policy.rs:4`
  - the workspace currently depends on `vector-ta = "0.2.4"`:
    - `crates/app_core/Cargo.toml:20`
    - `Cargo.toml:11`
    - `crates/strategies/Cargo.toml:9`
    - `crates/optimizer/Cargo.toml:9`
    - `src-tauri/Cargo.toml:25`
- Why it matters:
  - this comment is part of the maintenance story for a manually curated policy table
  - a stale anchor version makes drift harder to detect during future updates
- Smallest justified fix:
  - update or remove the version-specific comment
  - if the list is manually curated, say so plainly and describe the expected update workflow
- Review judgment:
  - low-severity but objective maintenance issue

## Docs And Examples

### 1. Public policy helpers and route types are undocumented

- Evidence:
  - public items without rustdoc include:
    - `supports_gpu_kernel_ma`: `142`
    - `prefers_gpu_kernel_ma`: `147`
    - `HybridCapabilityTier`: `169`
    - `HybridRouteTier`: `178`
    - `HybridIndicatorRouteDecision`: `185`
    - `DoubleMaHybridRoutePlan`: `193`
    - `HybridIndicatorPartition`: `219`
    - `classify_hybrid_capability`: `239`
    - `plan_hybrid_indicator_route`: `288`
    - `plan_double_ma_hybrid_route`: `317`
- Why it matters:
  - the file encodes non-obvious policy semantics such as:
    - what “CpuStaged” means
    - when the speedup table overrides the static preferred list
    - how unknown ids are treated
    - the difference between capability and selected route
- Smallest justified fix:
  - add rustdoc to the public helpers and route types
  - document that `CpuStaged` is a route choice, not proof of CPU-only capability

## Tests And Missing Verification

### 1. File-local policy coverage is good, but the risky fallback semantics are not directly pinned down

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core ma_policy::tests:: -- --nocapture`
      - passed
      - included:
        - `classify_unknown_ma_as_unsupported`
        - `partition_categories_match_capability_classifier`
        - `hybrid_partition_is_deterministic_and_deduplicated`
        - `speedup_table_routing_marks_unknown_id_as_cpu_preferred`
        - `unified_route_uses_gpu_for_gpu_preferred_kernel_capable_ma`
        - `unified_route_uses_cpu_for_unknown_indicator`
        - `kernel_support_and_preference_sets_are_distinct`
        - `speedup_table_routing_marks_known_ids_as_gpu_preferred`
        - `unified_route_mixed_selection_has_distinct_gpu_and_cpu_staged_paths`
    - downstream consumer verification:
      - `cargo test -p ta_app_core capability_report_marks_gpu_supported_for_plain_sma_request -- --nocapture`
        - passed
  - current tests do not directly cover:
    - partial-but-non-empty CSV fallback behavior
    - malformed quoted rows
    - drift between policy tables and documented dependency/version expectations
- Missing tests I would want before changing this file:
  - a test that a malformed or insufficient parsed table falls back to the built-in preferred list
  - a test that public support/preference booleans exposed via `ma_registry` remain consistent for representative ids
  - a drift-detection test around the manually curated support list

## Performance, Allocation, And CUDA Concerns

### 1. Capability classification linearly scans the MA registry on every call

- Evidence:
  - `classify_hybrid_capability` does:
    - `vector_ta::indicators::moving_averages::registry::list_moving_averages()`
    - then `.iter().find(...)`
    - `crates/app_core/src/ma_policy.rs:244-246`
  - partitioning and route planning layer on top of that classifier:
    - `261-327`
  - downstream orchestration calls these helpers repeatedly:
    - `crates/app_core/src/double_ma.rs:5161-5169`
    - `12391`
    - `12965-12970`
    - `crates/app_core/src/double_ma_validate.rs:963-979`
- Why it matters:
  - this is not a major runtime hotspot, but it is repeated policy work in orchestration and metadata generation
- Smallest justified fix:
  - cache the registry capability view with `OnceLock` the same way the speedup-table ids are cached
  - keep deterministic `BTreeSet` behavior for partition ordering if refactored
- Review judgment:
  - low-medium performance / tech-debt issue
  - safe-now if kept local and behavior-preserving

### 2. The current allocation profile is acceptable because the expensive work is one-time or low-frequency

- Evidence:
  - CSV parsing allocates strings and a `BTreeSet`, but it runs once via `OnceLock`:
    - `78-133`
  - partition output allocates owned `String`s intentionally for stable, deduplicated summaries:
    - `219-285`
- Review judgment:
  - no urgent allocation problem
  - avoid replacing readable deterministic behavior with overly clever micro-optimizations

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for policy helpers and route types
  - validate the parsed speedup set before treating it as authoritative
  - update the stale `vector-ta` version comment
- Coordination required:
  - narrowing public route types/functions
  - renaming `prefers_cpu_by_speedup_table`
- Defer until later pass:
  - larger policy-source unification work

## Technical-Debt Reduction Register

- Safe to reduce now:
  - undocumented route semantics
  - weak CSV-authority fallback semantics
  - stale dependency/version comment
- Coordination required:
  - public API narrowing
  - renaming or removing public aliases
- Useful debt that should not be “cleaned up” casually:
  - deterministic ordering via `BTreeSet`
  - embedded static speedup-table design, if the project intentionally wants no runtime dependency on benchmark artifacts

## Performance Opportunity Register

- Safe structural improvements:
  - cache registry capability lookup
- Benchmark-sensitive candidates:
  - none urgent
- Not the right place to chase performance without evidence:
  - replacing deterministic structures or readable policy code purely for micro-optimizations

## Observability And Diagnostics Gaps

- the file does not document which policy source wins when the embedded CSV is present
- the file does not document that `CpuStaged` is a routing choice rather than a capability tier
- the file does not document unknown/unmeasured-id semantics clearly

## Implementation Staging Guidance

- Safe now:
  - docs
  - parsed-table validation
  - version-comment cleanup
  - cached registry lookup if done behavior-preservingly
- Coordination required:
  - public API narrowing
  - alias renaming/removal
- Defer until later pass:
  - broader redesign of route-policy data ownership

## Change Blast Radius

- Local only:
  - docs
  - stale comment cleanup
  - internal cache addition
- Same subsystem:
  - support/preference semantics change affects `double_ma`, `double_ma_validate`, `double_ma_backend`, `indicator_catalog`, and `ma_registry`
- Cross-backend/runtime:
  - route-policy changes alter backend selection and fallback diagnostics for the Double-MA family

## Open Questions Or Assumptions

- assumption:
  - the embedded speedup CSV is intended to be the authoritative preference source when it is well-formed
- open question:
  - whether any crate-external consumer depends on the currently public route-tier structs and planners
- open question:
  - whether unknown or unmeasured ids should continue to be described as CPU-preferred, or merely as not GPU-preferred

## Recommended Action

`Small surgical refactor`

The right next pass is:

- harden the parsed-table fallback semantics
- document the policy contract
- narrow or rename the least-defensible public API pieces if no external consumer needs them

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core ma_policy::tests:: -- --nocapture`
- `cargo test -p ta_app_core capability_report_marks_gpu_supported_for_plain_sma_request -- --nocapture`
- `cargo test -p ta_app_core`
- if route-selection semantics change:
  - `cargo test -p ta_app_core double_ma_validation_gpu_kernel_supported_full_stack_preserves_route --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test --workspace`

## Regression Risk And Feature-Preservation Constraints

- do not silently change GPU-preferred versus CPU-staged policy without checking downstream route diagnostics and UI metadata
- do not remove deterministic ordering from partition outputs without verifying any consumer assumptions about stable display or summary order
- do not narrow public route types/functions until crate-external usage has been checked
