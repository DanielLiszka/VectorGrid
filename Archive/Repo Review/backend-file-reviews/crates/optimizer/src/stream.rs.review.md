# Review Packet: `crates/optimizer/src/stream.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/stream.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Clear and well tested, but not fully review-ready because merge compatibility rules and heap-order assumptions are not documented at the API boundary`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Optimizer-core`, `Semantics-sensitive`, `Ordering-sensitive`

## What This File Owns

`stream.rs` owns the incremental optimizer result aggregator:

- local score helper and scored entry types: `6-160`
- `StreamAggregator` behavior, merging, and finalization: `162-401`
- file-local tests: `403-521`

## Classification

- Public surface: yes
- Hot path: moderately
- CUDA/runtime-critical: no
- Strategy-local: no
  - shared optimizer support

## Direct Interactions

### Backend interaction map

- sibling support:
  - `crates/optimizer/src/grid.rs`
  - `crates/optimizer/src/lib.rs`
- downstream consumers:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/gpu_backtest_kernel.rs`

### Downstream verification impact if changed

- changing merge behavior changes combined best/top-k outputs in parallelized workflows
- changing score normalization changes ordering and finalist selection
- changing heatmap merge/finalize behavior changes reporting outputs

## Standards And Research Log

This packet was written after targeted web research focused on heap ordering and stable ranking semantics.

### Sources consulted

1. `std::collections::BinaryHeap`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html>
   - Why it was consulted:
     - this file uses `BinaryHeap<Reverse<ScoredCombo>>` as its top-k core
   - Standard informed:
     - ordering assumptions are part of correctness, not an implementation footnote

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - merge compatibility rules are observable API semantics
   - Standard informed:
     - silent no-op behavior should be documented when it is intentional

## Findings

## Contract Clarity

### 1. `merge` silently returns on incompatible aggregators, and that behavior needs explicit documentation

- Evidence:
  - `merge` returns early on mismatched objective, candle count, score function identity, pair-score identity, and `top_k`: `299-352`
- Why this matters:
  - callers could reasonably expect an error or debug signal when merges are dropped
- Smallest justified fix:
  - document the silent no-op contract; optionally add debug assertions in a future pass if desired

### 2. Function-pointer identity is part of merge compatibility, but readers have to infer that from implementation details

- Evidence:
  - `score_fn` and `pair_score_fn` are compared by function pointer address in `merge`: `307-318`
- Why this matters:
  - custom scoring users need to know that semantically equivalent closures are not supported here
- Smallest justified fix:
  - document the compatibility rule near the public builder methods or `merge`

## Ordering And Data-Structure Concerns

### 3. The top-k heap relies on stable `ScoredCombo` ordering rules and should remain intentionally simple

- Evidence:
  - `ScoredCombo` ordering is based on `score.total_cmp`: `142-160`
  - heap maintenance lives at `243-352`
- Why this matters:
  - changing ordering rules changes finalist ranking behavior directly
- Smallest justified fix:
  - document that the heap tracks score order only and that non-finite scores are normalized before insertion

## Tests And Missing Verification

### 4. Local tests are strong and already cover the tricky parts

- Evidence:
  - tests cover custom ranking, merge behavior, heatmap statistics, include-all ordering, and non-finite pair scoring: `403-521`
- Why this matters:
  - the review value is clarifying API behavior, not altering the implementation
- Smallest justified fix:
  - keep tests as-is and add comments

## Code-Quality Improvement Register

- Safe now:
  - document `merge` as a compatibility-checked no-op on mismatch
  - document function-pointer identity expectations
  - document heap-order contract
- Defer:
  - changing `merge` to return `Result`

## Technical-Debt Reduction Register

- Safe now:
  - comments only
- Defer:
  - API-shape changes to `merge`
- Do not casually remove:
  - score normalization
  - heatmap merge semantics
  - current heap ordering rules

## Performance Opportunity Register

- Safe structural work:
  - comments only
- Benchmark-sensitive:
  - changing heap maintenance strategy
  - changing merge/all-storage behavior in hot caller loops

## Recommended Action

`Small surgical refactor`

The implementation is already solid. The useful improvement is explicit documentation of merge compatibility and ordering rules so callers do not have to reverse-engineer them from tests.

## Exact Local Verification Required If Changed

1. Crate verification:
   - `cargo test -p ta_optimizer -- --nocapture`
2. Representative app-core consumers:
   - `cargo test -p ta_app_core double_ma -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_optimizer -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve `merge` no-op compatibility behavior unless callers are updated intentionally
- Preserve heap ordering and score normalization semantics so best/top-k outputs remain stable
- Preserve heatmap merge/finalize behavior unless downstream report expectations are revalidated
