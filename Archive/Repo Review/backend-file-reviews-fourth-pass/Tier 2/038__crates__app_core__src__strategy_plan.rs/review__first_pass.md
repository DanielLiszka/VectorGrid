# Review Packet: `crates/app_core/src/strategy_plan.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_plan.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful compiler surface, but not fully review-ready because route-selection policy, fingerprint semantics, and route-override behavior are only partially documented`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Planner`, `Routing-policy`, `Serialization-contract`, `Semantics-sensitive`

## What This File Owns

`strategy_plan.rs` owns compilation of validated `StrategyIr` into a route-annotated execution plan:

- plan/backend/route enums and plan structs:
  - `25-107`
- default route selection:
  - `109-165`
- value compilation and fingerprinting:
  - `167-193`
- main planner:
  - `195-317`
- file-local tests:
  - `319-587`

## Classification

- Public surface: effectively yes
  - exported through `crates/app_core/src/lib.rs`
  - consumed by `strategy_dynamic.rs`, `strategy_gpu_program.rs`, and Tauri/runtime-adjacent paths
- Hot path: no
  - planning/setup only
- Contract-critical: yes
  - route assignments and fingerprints feed later runtime behavior
- Low-risk support code: only partially
  - planning is small, but semantics fan out widely

## Direct Interactions

### Backend interaction map

- IR input and validation:
  - `crates/app_core/src/strategy_ir.rs:460-793`
- GPU executable compilation:
  - `crates/app_core/src/strategy_gpu_program.rs:300-432`
- dynamic runtime:
  - `crates/app_core/src/strategy_dynamic.rs:352-700`
- Tauri preview surface:
  - `src-tauri/src/commands_dynamic.rs`

### Downstream verification impact if changed

- changing default-route policy changes whether indicators stay on CPU, `GpuKernel`, or `GpuSingle`
- changing fingerprint semantics changes runtime cache keys and preview fingerprints
- changing override behavior changes whether caller mistakes are surfaced or ignored

## Standards And Research Log

This packet was written after targeted web research focused on documentation standards, default-field semantics, and fingerprint stability.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this planner encodes important behavior not visible from signatures
   - Standard informed:
     - route selection and error behavior should be explicit

2. `std::hash::DefaultHasher`
   - Source: <https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html>
   - Why it was consulted:
     - IR fingerprints are derived with `DefaultHasher`
   - Standard informed:
     - these fingerprints should not be treated as stable cross-build identifiers unless documented and chosen deliberately

3. Serde field defaults
   - Source: <https://serde.rs/attr-default.html>
   - Why it was consulted:
     - `CompiledStrategyPlan` and related structs use defaulted schema/version fields
   - Standard informed:
     - defaulted serialized fields need contract clarity to avoid silent drift

## Findings

## Routing Policy

### 1. The default route-selection policy is behaviorally important, but there is no local summary of the decision rules

- Evidence:
  - legacy kinds map through `default_legacy_route`: `109-119`
  - descriptor-driven dynamic indicators route based on catalog capabilities and speed-preference flags: `130-165`
- Why this matters:
  - this is the planner’s main product-facing policy choice
  - maintainers must currently reconstruct why an indicator becomes `Cpu`, `GpuKernel`, or `GpuSingle`
- Smallest justified fix:
  - add rustdoc describing route precedence:
    - forced `Cpu` for `StrategyPlanBackend::Cpu`
    - descriptor-based dynamic route choice for `GpuUnified`
    - legacy-kind fallback behavior

## Fingerprinting And Stability

### 2. `ir_fingerprint_hex` looks durable, but `DefaultHasher` does not promise stable output across builds

- Evidence:
  - `fingerprint_ir` serializes the IR and hashes it with `DefaultHasher`: `187-193`
  - the result is stored on `CompiledStrategyPlan` as `ir_fingerprint_hex`: `80-95`, `304`
- Why this matters:
  - the planner exposes the fingerprint to downstream runtime code and preview surfaces
  - if maintainers start treating it as a durable external identifier, they will over-assume its stability
- Smallest justified fix:
  - document that the fingerprint is a runtime/build-local identity token unless a stable hash algorithm is intentionally adopted

## Error Handling

### 3. Unknown route-override keys are silently ignored

- Evidence:
  - route overrides are applied only when a key matches a known trimmed indicator id: `225-239`
  - there is no post-check that every override entry was consumed
- Why this matters:
  - a typo in an override map becomes a silent no-op
  - that weakens the planner as a contract boundary and makes debugging route mismatches harder
- Smallest justified fix:
  - validate that all override keys match known indicator ids before returning a plan
  - if keeping today’s permissive behavior, document it explicitly

## Tests And Missing Verification

### 4. Tests cover happy-path planning well, but not silent override-key mistakes

- Evidence:
  - tests cover CPU routes, legacy schema v1 payloads, route overrides, and descriptor-driven dynamic routing: `319-587`
- Missing coverage:
  - no test shows whether extra/unknown override keys should error or be ignored
- Smallest justified fix:
  - add a test locking down override-key behavior before changing it

## Code-Quality Improvement Register

- Safe now:
  - document route-selection rules
  - document fingerprint scope/stability
  - tighten or explicitly document override-key handling
- Defer:
  - changing route heuristics
  - replacing the hash algorithm

## Technical-Debt Reduction Register

- Safe now:
  - local contract documentation
  - override validation hardening
- Defer:
  - planner redesign
  - route-policy reshaping
- Do not casually remove:
  - current legacy route defaults
  - current descriptor-based route selection

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - changing route-selection heuristics

## Recommended Action

`Small surgical refactor`

The planner is already compact and well-tested. The useful work is to make its route policy explicit, prevent silent override mistakes, and stop readers from inferring stronger fingerprint guarantees than the code actually provides.

## Exact Local Verification Required If Changed

1. File-local planner tests:
   - `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`
2. IR validation tests:
   - `cargo test -p ta_app_core strategy_ir::tests:: -- --nocapture`
3. GPU-program consumer tests:
   - `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change default route selection without checking dynamic preview and GPU runtime consumers together
- Do not replace `DefaultHasher` casually; fingerprint consumers and tests depend on current shape
- If you tighten override-key validation, coordinate any callers that currently rely on permissive behavior
