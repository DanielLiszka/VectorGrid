# Review Packet: `crates/strategies/src/records.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/records.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Compact and test-covered, but not fully review-ready because the parallel-vector invariant and per-column scan semantics are not documented clearly enough`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Support-code`, `Semantics-sensitive`

## What This File Owns

`records.rs` owns the shared trade-record transport and aggregation helpers:

- `TradeRecord` DTO and conversion helpers: `6-51`
- `TradeRecords` SoA container and aggregation helpers: `53-229`
- file-local tests: `230-345`

## Classification

- Public surface: yes
- Hot path: no
- CUDA/runtime-critical: no
- Strategy-local: support code shared across strategy consumers

## Direct Interactions

### Backend interaction map

- upstream dependency:
  - `crates/strategies/src/double_ma/mod.rs`
- downstream export:
  - `crates/app_core/src/lib.rs`

### Downstream verification impact if changed

- changing record layout changes serialized trade payload expectations
- changing per-column aggregation semantics changes trade summary calculations used by downstream consumers

## Standards And Research Log

This packet was written after targeted web research focused on public DTO documentation and maintaining explicit invariants.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file exposes serializable trade DTOs and container helpers
   - Standard informed:
     - public invariants should be documented where users can violate them

## Findings

## Contract Clarity

### 1. `TradeRecords` depends on a parallel-vector invariant that is only enforced by `validate`

- Evidence:
  - the struct stores seven parallel vectors: `46-51`
  - invariant checking lives in `validate`: `71-90`
- Why this matters:
  - callers mutating or deserializing the struct need to know that equal-length fields are the core invariant
- Smallest justified fix:
  - add rustdoc explaining the invariant and when `validate` should be called

## Docs And Examples

### 2. Per-column scans are intentional and should be described as such

- Evidence:
  - `to_trades_for_col` and `trade_stats_for_col` scan the full record arrays: `128-229`
- Why this matters:
  - a future cleanup pass might add indexing or ordering assumptions that are not part of the current contract
- Smallest justified fix:
  - document that the helpers are linear scans over SoA storage

## Tests And Missing Verification

### 3. The file has good targeted tests and should stay conservative

- Evidence:
  - tests cover validation mismatches, per-column scoping, round-trip behavior, and overflow rejection: `230-345`
- Why this matters:
  - the current implementation is clear; the review value is contract clarity, not redesign
- Smallest justified fix:
  - add docs, keep behavior unchanged

## Code-Quality Improvement Register

- Safe now:
  - document the parallel-vector invariant
  - document linear per-column scan behavior
- Defer:
  - no structural change needed

## Technical-Debt Reduction Register

- Safe now:
  - rustdoc only
- Do not casually remove:
  - overflow-checked `try_from_trade`
  - `validate`
  - per-column aggregation helpers

## Performance Opportunity Register

- This file is not the right place to chase performance unless record volumes prove it necessary

## Recommended Action

`Small surgical refactor`

The useful improvement is explicit invariant documentation on the public types. The current implementation and tests are otherwise in good shape.

## Exact Local Verification Required If Changed

1. Crate verification:
   - `cargo test -p ta_strategies -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_strategies -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve the current record field layout and overflow behavior unless downstream serialization expectations are updated
- Preserve per-column aggregation semantics so downstream trade summaries remain stable
