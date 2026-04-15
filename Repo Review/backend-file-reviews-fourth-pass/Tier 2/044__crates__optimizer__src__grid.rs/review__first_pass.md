# Review Packet: `crates/optimizer/src/grid.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/grid.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small and test-covered, but not fully review-ready because its score-normalization contract is intentionally coupled to stream aggregation and that coupling is only visible through tests`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Optimizer-core`, `Semantics-sensitive`, `Scoring-contract`

## What This File Owns

`grid.rs` owns the simple full-batch grid-search aggregator:

- objective scoring helper: `5-20`
- `grid_search` orchestration: `22-89`
- file-local compatibility tests: `91-189`

## Classification

- Public surface: yes
- Hot path: moderately
- CUDA/runtime-critical: no
- Strategy-local: no
  - shared optimizer support

## Direct Interactions

### Backend interaction map

- public dependencies:
  - `crates/optimizer/src/lib.rs`
  - `crates/optimizer/src/stream.rs`
- downstream consumers:
  - `crates/app_core/src/double_ma.rs`

### Downstream verification impact if changed

- changing score normalization changes best/top-k ordering
- changing `all` or `top_scores` semantics affects downstream result consumers and comparison tests

## Standards And Research Log

This packet was written after targeted web research focused on keeping public scoring behavior explicit and stable.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file provides a public optimizer entry point whose semantics are small but important
   - Standard informed:
     - compatibility-sensitive scoring behavior should be documented

## Findings

## Contract Clarity

### 1. Non-finite objective scores are intentionally clamped to `NEG_INFINITY`, and that contract should be stated next to the scorer

- Evidence:
  - `score` maps non-finite values to `f64::NEG_INFINITY`: `5-20`
  - tests explicitly align this behavior with `StreamAggregator`: `108-189`
- Why this matters:
  - ranking compatibility between grid and stream aggregation is part of observable optimizer behavior
- Smallest justified fix:
  - add a comment documenting that `grid.rs` intentionally mirrors stream aggregation for non-finite scores

## Technical Debt

### 2. Score normalization logic is duplicated from the stream path and currently defended only by tests

- Evidence:
  - local `score` helper: `5-20`
  - compatibility tests compare grid and stream outputs: `133-189`
- Why this matters:
  - behavioral drift is easy if one scorer changes and the other does not
- Smallest justified fix:
  - document the coupling now; defer any shared-score abstraction unless it can land without widening blast radius

## Tests And Missing Verification

### 3. Local tests are well targeted and should remain the main guardrail

- Evidence:
  - tests cover non-finite clamping and parity with `StreamAggregator`: `91-189`
- Why this matters:
  - the module is already small; behavior documentation is more useful than structural change
- Smallest justified fix:
  - keep the current tests and add comments

## Code-Quality Improvement Register

- Safe now:
  - comment the score-normalization contract
  - note the intentional parity with stream aggregation
- Defer:
  - factoring scoring into shared infrastructure

## Technical-Debt Reduction Register

- Safe now:
  - comments only
- Do not casually remove:
  - `NEG_INFINITY` clamping
  - current `top_scores` population behavior

## Performance Opportunity Register

- This file is not the right place for meaningful performance work; it is already straightforward

## Recommended Action

`Small surgical refactor`

The useful improvement is comment-level hardening around the scorer contract. The current behavior and tests are otherwise solid.

## Exact Local Verification Required If Changed

1. Crate verification:
   - `cargo test -p ta_optimizer -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_optimizer -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve score normalization parity with `StreamAggregator`
- Preserve current best/top-k/all result semantics unless downstream consumers are updated deliberately
