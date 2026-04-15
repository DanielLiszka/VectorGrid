# Review Packet: `crates/strategies/src/composite_cpu/mod.rs`

- Review date: `2026-04-14`
- Source file: `crates/strategies/src/composite_cpu/mod.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Foundational and test-covered, but not fully review-ready because signal-mode semantics, overlay behavior, and the file’s internal ownership split are still too implicit`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Public-surface-sensitive`, `Hot-path`, `Semantics-sensitive`, `CPU-core`

## What This File Owns

`composite_cpu/mod.rs` is the shared CPU implementation for composite-strategy evaluation:

- signal-mode enum and desired-position logic: `16-191`
- execution overlay params/state helpers: `193-277`
- metrics/curve entry points and main implementation: `278-1118`
- file-local tests: `1119-1418`

## Classification

- Public surface: yes
  - consumed directly by `ta_app_core`
- Hot path: yes
- CUDA/runtime-critical: no
- Strategy-local: shared strategy implementation

## Direct Interactions

### Backend interaction map

- main consumers:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_mo.rs`
  - `crates/app_core/src/composite_validate.rs`
- shared dependency surface:
  - `crates/strategies/src/double_ma/mod.rs`

### Downstream verification impact if changed

- changing `CompositeSignalMode` behavior changes request semantics across optimization and validation
- changing overlay behavior changes trade timing, trade count, and risk metrics
- changing metric-entry wrappers affects app-core parity and report outputs

## Standards And Research Log

This packet was written after targeted web research focused on public contract documentation and preserving shared CPU evaluation behavior.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file exports the signal-mode surface and evaluation entry points
   - Standard informed:
     - public behavior variants should be documented where semantics are product-facing

2. Rust Book testing guidance
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why it was consulted:
     - this module has a focused local test suite that acts as the main semantic guardrail
   - Standard informed:
     - refactors should preserve the current behavior-oriented test coverage

## Findings

## Contract Clarity

### 1. `CompositeSignalMode` is product semantics, not a cosmetic enum, and should say so locally

- Evidence:
  - `CompositeSignalMode` is the exported mode selector at `16-22`
  - the variants materially change entry/exit logic in `composite_desired_position`: `24-191`
- Why this matters:
  - downstream callers in app-core use these variants as workflow configuration, not internal implementation detail
- Smallest justified fix:
  - add rustdoc describing each mode’s entry/exit philosophy

## Technical Debt

### 2. The execution overlay is part of behavior, but it is buried in private helper state

- Evidence:
  - overlay helpers live at `193-277`
  - the main evaluator applies them inside the hot loop: `380-1118`
- Why this matters:
  - cooldown and persistence are behavior contracts that affect trade timing and metric output
- Smallest justified fix:
  - add a short section comment explaining overlay responsibilities and sequencing

### 3. The file currently mixes signal logic, execution overlays, metrics aggregation, curve generation, and tests

- Evidence:
  - signal logic: `24-191`
  - main metrics implementation: `380-721`
  - curve generation: `722-1118`
- Why this matters:
  - there is a real structural split available, but the file is too central to split casually
- Smallest justified fix:
  - document the internal ownership split now; defer extraction

## Tests And Missing Verification

### 4. Local tests are strong and should be treated as semantic contracts

- Evidence:
  - tests cover hard-gate compatibility, overlay behavior, trend-pullback-reversion, TP/SL trade count, and wrapper parity: `1119-1418`
- Why this matters:
  - these tests are the reason contract documentation is more useful right now than structural edits
- Smallest justified fix:
  - preserve the current tests and add comments rather than change behavior

## Code-Quality Improvement Register

- Safe now:
  - document `CompositeSignalMode`
  - document overlay sequencing
  - document the high-level split between signal logic, metrics, and curves
- Coordination required:
  - splitting the file
  - altering signal or overlay semantics
- Defer:
  - broad refactor of the shared composite CPU engine

## Technical-Debt Reduction Register

- Safe now:
  - rustdoc and section comments
- Defer:
  - module extraction
- Do not casually remove:
  - any `CompositeSignalMode` variant
  - overlay helpers
  - compatibility wrappers that preserve existing callers

## Performance Opportunity Register

- Safe structural work:
  - comments only
- Benchmark-sensitive:
  - altering the hot-loop structure
  - changing overlay sequencing or signal evaluation order

## Recommended Action

`Larger refactor but defer`

The file would benefit from a cleaner split, but that split is shared-behavior work with broad blast radius. The safe value now is clearer contract documentation for modes and overlays.

## Exact Local Verification Required If Changed

1. Crate verification:
   - `cargo test -p ta_strategies -- --nocapture`
2. Representative app-core consumers:
   - `cargo test -p ta_app_core composite_cpu -- --nocapture`
   - `cargo test -p ta_app_core composite_validate -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_strategies -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve current `CompositeSignalMode` semantics unless downstream workflow behavior is intentionally changed
- Preserve overlay sequencing because it affects trade timing and final metrics
- Defer structural splitting until a dedicated shared-strategy refactor pass is budgeted
