# Review Packet: `crates/strategies/src/double_ma/mod.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/double_ma/mod.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Core foundational module with strong test coverage, but not fully review-ready because the public config/default contracts and the internal responsibility split are too large and too implicit`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Public-surface-sensitive`, `Hot-path`, `Foundational`, `Semantics-sensitive`

## What This File Owns

`double_ma/mod.rs` is the foundational shared-strategy module for the workspace:

- public DTOs and risk metrics surface: `10-349`
- `StrategyConfig` defaults and helpers: `350-1339`
- curve generation and matrix prep helpers: `1340-2220`
- single/batch evaluation entry points and CPU engines: `2233-4228`
- file-local tests: `4229-4315`

## Classification

- Public surface: yes
  - widely consumed by `ta_app_core` and `ta_optimizer`
- Hot path: yes
- CUDA/runtime-critical: indirectly
  - CPU semantics anchor GPU parity
- Strategy-local: no
  - foundational shared strategy crate

## Direct Interactions

### Backend interaction map

- main consumers:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/optimizer/src/lib.rs`
  - `crates/optimizer/src/grid.rs`
  - `crates/optimizer/src/stream.rs`
- sibling support surface:
  - `crates/strategies/src/records.rs`
  - `crates/strategies/src/composite_cpu/mod.rs`

### Downstream verification impact if changed

- changing DTO or default semantics affects nearly every optimizer and validation workflow
- changing metric finalization or evaluation logic affects ranking, reports, and CPU/GPU parity
- changing builder or batch behavior affects both strategy and optimizer crates

## Standards And Research Log

This packet was written after targeted web research focused on public API documentation and parallel-evaluation behavior.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file defines most of the public shared-strategy surface
   - Standard informed:
     - omission defaults and foundational DTO semantics should be documented locally

2. `rayon::iter::ParallelIterator`
   - Source: <https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html>
   - Why it was consulted:
     - the module uses Rayon for batch evaluation work
   - Standard informed:
     - parallel batch behavior should be treated as part of performance-sensitive semantics

## Findings

## Contract Clarity

### 1. `StrategyConfig` is too important to remain mostly discoverable only by reading default helper functions

- Evidence:
  - dozens of default helpers appear from `355-619`
  - `StrategyConfig` is declared at `624` and defaulted at `781`
- Why this matters:
  - many downstream workflows depend on omission-based config behavior
- Smallest justified fix:
  - add high-level rustdoc describing the major config families and that defaults are compatibility-sensitive

### 2. `Metrics` and `finalize_risk_metrics` form a core contract for downstream ranking/reporting

- Evidence:
  - `Metrics`: `39-107`
  - `finalize_risk_metrics`: `129-325`
- Why this matters:
  - app-core and optimizer consumers treat these fields as stable output semantics
- Smallest justified fix:
  - document that new fields and default values must preserve serialization and ranking compatibility

## Module Boundaries And Technical Debt

### 3. The file has grown into a foundational monolith spanning DTOs, config, metrics, curves, builders, and batch engines

- Evidence:
  - public DTO/config surface dominates the first third of the file
  - evaluation engines and matrix prep dominate the middle/lower sections through `4228`
- Why this matters:
  - a cleaner split exists, but changing it casually would create repo-wide churn
- Smallest justified fix:
  - document the internal ownership split now; defer extraction to a dedicated foundational refactor pass

## Performance Concerns

### 4. The batch CPU path is a hot path and should not be refactored for aesthetics

- Evidence:
  - Rayon-backed evaluation is imported at the top
  - `double_ma_batch_cpu` and batch builder logic live at `2178-2461`
- Why this matters:
  - small structural edits can alter throughput, ordering assumptions, or allocation behavior across app-core and optimizer paths
- Smallest justified fix:
  - preserve current hot-path structure unless measurement is part of the task

## Tests And Missing Verification

### 5. Local tests are focused and should remain the first guardrail for any shared-strategy changes

- Evidence:
  - tests cover costs, TP/SL behavior, derived metrics, grid expansion, and batch builder behavior: `4229-4315`
- Why this matters:
  - the file is too central for speculative cleanup
- Smallest justified fix:
  - keep changes small and verify app-core consumers when any public contract changes

## Code-Quality Improvement Register

- Safe now:
  - document `StrategyConfig` families and omission defaults
  - document `Metrics` compatibility expectations
  - add a high-level ownership map comment at the top
- Coordination required:
  - splitting config/metrics/evaluation into separate modules
  - changing batch evaluation structure
- Defer:
  - broad foundational refactor

## Technical-Debt Reduction Register

- Safe now:
  - rustdoc and top-level section comments
- Defer:
  - module extraction
- Do not casually remove:
  - compatibility defaults
  - metric fields consumed downstream
  - CPU batch entry points used across crates

## Performance Opportunity Register

- Safe structural work:
  - comments only
- Benchmark-sensitive:
  - changing Rayon-backed batch logic
  - restructuring matrix prep or evaluation loops

## Recommended Action

`Larger refactor but defer`

This file needs better documentation more urgently than it needs a split. The right refactor is possible, but it should happen only as a dedicated foundational pass with broad downstream verification.

## Exact Local Verification Required If Changed

1. Crate verification:
   - `cargo test -p ta_strategies -- --nocapture`
2. Optimizer consumers:
   - `cargo test -p ta_optimizer -- --nocapture`
3. Representative app-core consumers:
   - `cargo test -p ta_app_core double_ma -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_strategies -- --nocapture`
  - `cargo test -p ta_optimizer -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve `StrategyConfig` omission defaults unless compatibility is intentionally changed
- Preserve current metric field semantics because ranking, reporting, and parity flows depend on them
- Treat hot-path refactors as benchmark-sensitive even when the code shape looks simplifiable
