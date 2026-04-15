# Review Packet: `crates/optimizer/src/lib.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/lib.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Compact crate root plus public DTO surface, but it should document objective, mode, and engine contracts more explicitly`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Public-surface-sensitive`, `Crate-root`, `Support-code`

## What This File Owns

`lib.rs` owns the public crate surface for `ta_optimizer`:

- module exposure for `grid`, `mo`, and `stream`: `1-3`
- public enums and traits: `8-31`
- CPU engine adapter and result DTOs: `33-78`

## Classification

- Public surface: yes
- Hot path: indirectly
- CUDA/runtime-critical: no
- Strategy-local: no
  - optimizer crate root and shared DTO layer

## Direct Interactions

### Backend interaction map

- downstream consumers:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_mo.rs`
- upstream dependency:
  - `crates/strategies/src/double_ma/mod.rs`

### Downstream verification impact if changed

- changing enums changes request/ranking semantics across optimizer callers
- changing `BacktestEngine` or `OptimizationResult` changes optimizer integration contracts

## Standards And Research Log

This packet was written after targeted web research focused on public API documentation for crate-root surfaces.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file defines the crate’s public objective, mode, engine, and result contracts
   - Standard informed:
     - crate-root and DTO surface contracts should be documented locally

## Findings

## Docs And Examples

### 1. `ObjectiveKind`, `OptimizationMode`, `BacktestEngine`, and `OptimizationResult` should describe their stability expectations

- Evidence:
  - enums and trait are declared at `8-31`
  - result DTOs live at `46-78`
- Why this matters:
  - these are the primary public types downstream code reads and serializes
- Smallest justified fix:
  - add rustdoc describing each type’s role and compatibility posture

### 2. `CpuEngine` is a convenience adapter and should say so explicitly

- Evidence:
  - `CpuEngine` is defined at `33-44`
- Why this matters:
  - readers otherwise have to inspect the implementation to understand that it is the default shared-strategy adapter
- Smallest justified fix:
  - add a short doc comment clarifying that it delegates to `ta_strategies::double_ma::double_ma_batch_cpu`

## Code-Quality Improvement Register

- Safe now:
  - rustdoc for public types
- Defer:
  - no structural change needed

## Technical-Debt Reduction Register

- Safe now:
  - documentation only
- Do not casually remove:
  - current module exports
  - current DTO fields without coordinating downstream serialization/users

## Performance Opportunity Register

- This file is not a direct performance target

## Recommended Action

`Document only`

The structure is already fine. The real improvement is documenting the public optimizer vocabulary and result surface.

## Exact Local Verification Required If Changed

1. Crate verification:
   - `cargo test -p ta_optimizer -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_optimizer -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve public enums, trait shape, and result DTO fields unless downstream crates are updated in lockstep
