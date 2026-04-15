# Review Packet: `crates/app_core/src/strategy_ir.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_ir.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Important serialized contract, but not fully review-ready because schema rules, execution-policy invariants, and constant-bit encoding are not sufficiently documented`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Serialized-contract`, `Validation`, `Schema-versioning`, `Semantics-sensitive`

## What This File Owns

`strategy_ir.rs` owns the dynamic-strategy intermediate representation:

- schema/version constants and compatibility helpers:
  - `12-77`
- IR enums and graph structs:
  - `79-237`
- static IR builder input:
  - `239-277`
- validation:
  - `279-470`
- static IR builder:
  - `499-793`
- file-local tests:
  - `798-1109`

It is the main contract between frontend-built dynamic JSON, Tauri preview parsing, and backend planning.

## Classification

- Public surface: yes
  - consumed by `src-tauri/src/commands_dynamic.rs`
  - frontend JSON is shaped toward this schema through `web/src/strategies/dynamicDoubleMaBuilder.ts`
- Hot path: no
  - validation/build-time only
- Contract-critical: yes
- Strategy-local: no
  - shared contract for dynamic strategy planning

## Direct Interactions

### Backend interaction map

- Tauri preview boundary:
  - `src-tauri/src/commands_dynamic.rs:3-42`
- plan compilation:
  - `crates/app_core/src/strategy_plan.rs:195-317`
- dynamic preview/runtime:
  - `crates/app_core/src/strategy_dynamic.rs:352-700`
- frontend IR construction:
  - `web/src/strategies/dynamicDoubleMaBuilder.ts:309-412`

### Downstream verification impact if changed

- changing schema or defaults changes frontend JSON compatibility
- changing validation rules changes what Tauri will accept before planning
- changing builder output changes static profile parity and dynamic preview behavior

## Standards And Research Log

This packet was written after targeted web research focused on serialized field defaults and API documentation expectations.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file defines a serialized contract with many non-obvious fields and defaults
   - Standard informed:
     - public contract types should state field semantics and failure rules explicitly

2. Serde field defaults
   - Source: <https://serde.rs/attr-default.html>
   - Why it was consulted:
     - several fields use `#[serde(default)]` or default-producing helpers
   - Standard informed:
     - defaulted fields should be clearly documented so omitted JSON does not silently surprise callers

3. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why it was consulted:
     - `validate_strategy_ir` is the main argument-validation gate for dynamic IR
   - Standard informed:
     - important invariants should fail early at validation time, not much later in runtime preparation

## Findings

## API And Schema Contracts

### 1. This serialized IR contract has almost no local rustdoc despite being consumed across frontend, Tauri, and backend boundaries

- Evidence:
  - major public enums and structs (`StrategyIr`, `ExecutionPolicy`, `IndicatorNode`, `SignalProgramGraph`) expose no rustdoc
  - Tauri parses user-provided JSON directly into `StrategyIr`: `src-tauri/src/commands_dynamic.rs:28-31`
- Why this matters:
  - schema versioning, defaulted fields, and dynamic indicator descriptors are important user-facing/backend-facing contracts
  - maintainers currently need to infer semantics from tests and builder code
- Smallest justified fix:
  - add rustdoc to the major schema structs and version constants

## Validation

### 2. `validate_strategy_ir` is strong on graph integrity, but shallow on execution-policy invariants

- Evidence:
  - `validate_strategy_ir` checks schema support, MA specs, non-empty `ma_source`, and graph validity: `460-469`
  - it does not validate profile-specific execution requirements such as composite profiles requiring `halftrend`, `macz`, and `ott`
  - those missing values are instead rejected later in `prepare_dynamic_cpu_context`: `crates/app_core/src/strategy_dynamic.rs:765-784`
- Why this matters:
  - invalid IR can deserialize and validate successfully, then fail much later in plan execution or preview-related workflows
  - that increases debugging distance and weakens the value of the IR validator
- Smallest justified fix:
  - extend validation to enforce profile-to-execution-policy coupling, or document clearly that some execution invariants are deferred to later compilation/runtime stages

## Data Encoding

### 3. `ValueRef::ConstantBits` is a legitimate precision-preserving encoding, but the contract is obscure

- Evidence:
  - `ConstantBits` stores raw `u64` bits: `108-118`
  - deserialization accepts either numeric or string forms: `126-145`
- Why this matters:
  - this is a subtle but important schema choice; without docs, it looks arbitrary
  - frontend or tooling maintainers may “simplify” it to plain floats and accidentally change exact-bit behavior
- Smallest justified fix:
  - document why constants are stored as bits and which JSON forms are accepted

## Limits And Compatibility

### 4. The IR size limits are defined clearly in code but not surfaced as part of the contract

- Evidence:
  - `IR_MAX_INDICATORS`, `IR_MAX_CONDITIONS`, `IR_MAX_GROUPS`, `IR_MAX_GROUP_CONDITIONS`: `15-18`
  - validation enforces them in `validate_program`: `288-456`
- Why this matters:
  - these are product limits, not just implementation details
  - callers building IR externally need to know them before they hit validation errors
- Smallest justified fix:
  - document the limits near the constants and/or on `StrategyIr`

## Tests And Missing Verification

### 5. The tests are good on schema compatibility and builder output, but they do not lock in deferred execution-policy validation

- Evidence:
  - tests cover compatibility, schema v1 payloads, dynamic indicator validation, and builder success: `798-1109`
- Missing coverage:
  - explicit proof of which invariants are intentionally deferred beyond `validate_strategy_ir`
- Smallest justified fix:
  - add a targeted test for profile/execution mismatch so the contract is explicit either way

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc to schema structs
  - document `ConstantBits`
  - document size/version limits
  - tighten or explicitly defer execution-policy validation
- Defer:
  - schema redesign
  - removing legacy schema v1 compatibility

## Technical-Debt Reduction Register

- Safe now:
  - contract documentation
  - validation-hardening for profile/execution coupling
- Defer:
  - schema reshaping
  - cross-version migration work
- Do not casually remove:
  - schema v1 support
  - bit-precise constant encoding

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - none

## Recommended Action

`Small surgical refactor`

This is the right place to strengthen the dynamic-strategy contract. The main opportunity is earlier validation and explicit documentation, not structural rework.

## Exact Local Verification Required If Changed

1. File-local schema tests:
   - `cargo test -p ta_app_core strategy_ir::tests:: -- --nocapture`
2. Planner compilation tests:
   - `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`
3. Tauri preview parsing:
   - `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core strategy_ir::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not tighten validation in a way that breaks the currently supported frontend builder payloads without coordinated frontend/Tauri updates
- Do not remove schema v1 compatibility without an explicit migration decision
- Do not replace `ConstantBits` with plain float serialization casually; that would narrow precision semantics
