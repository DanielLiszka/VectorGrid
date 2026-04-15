# Review Packet: `crates/app_core/src/double_ma_vdps.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_vdps.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Cohesive internal helper module, but not fully review-ready as-is because it hides sorted-slice preconditions, overshoots refill budgets in three seed-injection helpers, and drops requested score_mode semantics while pruning partial rung results`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Internal-helper-surface`, `Benchmark-sensitive`, `Semantics-sensitive`, `Telemetry-sensitive`

## What This File Owns

`double_ma_vdps.rs` owns the crate-local helper logic for the `GpuVdps` optimization mode: rung targeting, period decimation/windowing, local frontier selection, candidate reranking, seed-pair refill heuristics, nearest-period snapping, and VDPS telemetry aggregation.

It owns:

- rung targeting and period selection helpers:
  - `vdps_target_pairs`: `crates/app_core/src/double_ma_vdps.rs:14-25`
  - `vdps_decimated_periods`: `27-44`
  - `vdps_period_window_slice`: `46-55`
  - `vdps_local_anchor_axes`: `101-157`
- candidate reranking and partial merge:
  - `vdps_rank_candidates`: `57-99`
  - `vdps_merge_partial_results`: `159-198`
- seed-pair generation helpers:
  - `vdps_inject_random_seed_pairs`: `200-224`
  - `vdps_nearest_period`: `226-245`
  - `vdps_inject_elite_mutation_pairs`: `247-283`
  - `vdps_inject_differential_seed_pairs`: `285-327`
- telemetry aggregation:
  - `merge_vdps_telemetry`: `329-377`

This file does not own:

- request validation and `VdpsResolvedConfig` policy:
  - `crates/app_core/src/double_ma.rs:1542-1652`
- mode gating:
  - `crates/app_core/src/double_ma_mode.rs:68-76`
- final result packaging:
  - `crates/app_core/src/double_ma_result.rs:15-35`
  - `225-242`

## Classification

- Public surface: no
  - crate-private module and `pub(crate)` helpers only
- Hot path: moderate
  - these helpers shape the frontier and survivor set for `GpuVdps`
- CUDA/runtime-critical: indirect
  - affects how many pairs later rungs and GPU refill stages will evaluate
- Strategy-local: yes
  - Double-MA `GpuVdps` helper layer
- Low-risk support code: no
  - logic bugs here can quietly alter candidate budgets and ranking fidelity

## Direct Interactions

### Backend interaction map

- imported by the main Double-MA optimization hub:
  - `crates/app_core/src/double_ma.rs:31-34`
- used directly in the `GpuVdps` rung loop:
  - `crates/app_core/src/double_ma.rs:6388-7036`
- relies on scoring helpers from the main Double-MA module:
  - `effective_score`
  - `estimate_pairs_fast_lt_slow`
  - `DoubleMaScoreMode`
  - `VdpsTelemetry`
  - `crates/app_core/src/double_ma_vdps.rs:8-10`
- telemetry is later surfaced through result shaping:
  - `crates/app_core/src/double_ma_result.rs:15-35`
  - `225-242`

### Downstream verification impact if changed

- refill helper changes affect:
  - survivor set size
  - later-rung workload
  - benchmark throughput and fidelity
- partial-merge ranking changes affect:
  - which candidates survive into later rungs
  - final best/top results under non-default score modes
- telemetry merge changes affect:
  - result diagnostics
  - quality-check interpretation

## Standards And Research Log

This packet was written after targeted web research focused on API clarity for internal helper boundaries, documentation expectations for hidden preconditions, Rust test-organization guidance, slice-partition preconditions for binary-partition helpers, and CUDA best-practices framing for benchmark-sensitive frontier-size changes.

### Sources consulted

1. Rust `slice::partition_point` documentation
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point>
   - Why it was consulted:
     - this file uses `partition_point` in `vdps_period_window_slice` and `vdps_nearest_period`
   - Standard informed:
     - the slice must be partitioned/sorted for the predicate; otherwise the result is unspecified

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this module is a dense internal helper boundary with several hidden invariants
   - Standard informed:
     - helper behavior and preconditions should stay explicit and narrowly owned

3. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - sorted-slice and ranking-contract assumptions are currently undocumented
   - Standard informed:
     - helper modules should document non-obvious invariants that callers must preserve

4. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - some VDPS helpers have coverage, but key contract edges do not
   - Standard informed:
     - algorithmic helper bugs should be pinned with focused tests close to the owning logic

5. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - frontier-size inflation and ranking drift in a GPU-assisted search mode are benchmark-sensitive
   - Standard informed:
     - workload-shaping changes should be measured, not waved through as “small cleanup”

## Findings

## Correctness And Safety

### 1. The three refill helpers do not actually honor `target_count`

- Evidence:
  - random refill:
    - `crates/app_core/src/double_ma_vdps.rs:200-224`
  - elite mutation refill:
    - `247-283`
  - differential refill:
    - `285-327`
  - each loop compares:
    - `seen.len() < current.len().saturating_add(target_count)`
  - but `current.len()` grows every time a new pair is pushed, so the target keeps moving upward
  - these helpers feed directly into the rung survivor/refill path:
    - `crates/app_core/src/double_ma.rs:6722-6839`
- Why it matters:
  - refill budgets can overshoot
  - intermediate seed sets can exceed the intended survivor/refill count
  - later rungs can evaluate more frontier than the configured heuristic budget requested
- Smallest justified fix:
  - freeze the goal length before the loop, for example:
    - `let goal_len = seen.len().saturating_add(target_count);`
  - compare against that fixed goal instead of recomputing from the growing `current.len()`
  - add focused tests that prove each refill helper stops at the requested count
- Review judgment:
  - real correctness and performance issue
  - benchmark-sensitive and semantics-sensitive

### 2. `vdps_merge_partial_results` drops `score_mode` semantics while pruning partial rung results

- Evidence:
  - helper signature does not accept `score_mode`:
    - `crates/app_core/src/double_ma_vdps.rs:159-164`
  - it reranks with:
    - `vdps_rank_candidates(..., objective, None, ...)`: `177`
  - it also recomputes `best_score` and `top_scores` with:
    - `effective_score(..., objective, None)`: `180-184`
  - caller-side `GpuVdps` execution is using the requested `score_mode` elsewhere:
    - `crates/app_core/src/double_ma.rs:5117`
    - `5535-5613`
  - the merged partial result is then used to build the survivor seed set:
    - `6697-6719`
- Why it matters:
  - for non-default `score_mode`, candidates can be truncated according to the plain objective rather than the requested ranking contract
  - once truncated, later `vdps_rank_candidates(..., score_mode, ...)` calls cannot recover candidates that were already dropped from the merged partial set
- Smallest justified fix:
  - thread `score_mode` into `vdps_merge_partial_results`
  - update its test to cover a non-`None` score mode
- Review judgment:
  - high-value algorithmic correctness issue
  - semantics-sensitive and benchmark-sensitive

## API And Naming

### 1. The helper names are mostly clear, but important sorted-slice preconditions are implicit

- Evidence:
  - `vdps_period_window_slice` uses `partition_point`:
    - `crates/app_core/src/double_ma_vdps.rs:46-55`
  - `vdps_nearest_period` also uses `partition_point`:
    - `226-245`
  - Rust’s docs state the slice is assumed to be partitioned/sorted for the predicate; otherwise results are unspecified
- Why it matters:
  - callers must preserve sorted ascending period arrays for these helpers to be meaningful
  - that contract is currently only knowable by reading the implementation plus the standard-library docs
- Smallest justified fix:
  - add concise doc comments stating that `periods` must be sorted ascending
- Review judgment:
  - low-severity API clarity issue

## Error Handling

### 1. Telemetry aggregation silently merges incompatible shapes

- Evidence:
  - `merge_vdps_telemetry` merges rung stats positionally if the rung counts match:
    - `crates/app_core/src/double_ma_vdps.rs:353-370`
  - it does not verify that `cur.rungs` and `inc.rungs` actually contain the same fractions/order
  - if lengths differ, it appends `rung_stats` but does not update `cur.rungs`:
    - `371-372`
  - this aggregate is later surfaced through the result payload:
    - `crates/app_core/src/double_ma.rs:7405`
    - `crates/app_core/src/double_ma_result.rs:225-242`
- Why it matters:
  - execution output is not corrupted, but telemetry integrity is brittle
  - a future mismatch in rung plans can yield misleading aggregate diagnostics
- Smallest justified fix:
  - only merge telemetry when the rung plans match exactly
  - otherwise keep telemetry separate or clear aggregate-only fields explicitly
- Review judgment:
  - medium-severity telemetry/diagnostics issue

## Module Boundaries And Technical Debt

### 1. The file carries a small dead-code smell via an unused import

- Evidence:
  - `objective_score` is imported but not used:
    - `crates/app_core/src/double_ma_vdps.rs:8-10`
  - current test/build output also reports that warning
- Why it matters:
  - low severity, but it is noise in a dense helper module where signal matters
- Smallest justified fix:
  - remove the unused import
- Review judgment:
  - safe-now cleanup

## Docs And Examples

### 1. The module should explain what “VDPS” means and what invariants its helpers assume

- Why it matters:
  - this file is cohesive, but opaque to a reviewer who opens it cold
  - sorted-slice and heuristic-budget assumptions are not obvious from the module boundary
- Smallest justified fix:
  - add a short module-level comment covering:
    - what `GpuVdps` is in repo terms
    - that period slices must be sorted ascending
    - that these helpers are benchmark-sensitive and feed survivor selection

## Tests And Missing Verification

### 1. Existing tests cover some helpers, but the highest-risk cases are still missing

- Evidence:
  - existing targeted runs succeeded:
    - `cargo test -p ta_app_core vdps_merge_partial_results -- --nocapture`
    - result: `1` test passed
    - `cargo test -p ta_app_core gpu_vdps -- --nocapture`
    - result: `2` tests passed
  - existing helper tests in `double_ma.rs` include:
    - `vdps_local_anchor_axes_include_center_and_trim_candidates`: `crates/app_core/src/double_ma.rs:20857-20869`
    - `vdps_merge_partial_results_keeps_global_best`: `20871-20945`
  - I did not find focused coverage for:
    - refill helpers honoring `target_count`
    - `vdps_merge_partial_results` preserving non-`None` `score_mode`
    - mismatched telemetry merge behavior
- Smallest justified fix:
  - add those three focused tests

## Performance, Allocation, And CUDA Concerns

### 1. The refill-budget bug is the main performance concern in this file

- Evidence:
  - overshoot can inflate `next_seed_pairs` before later rung evaluation:
    - `crates/app_core/src/double_ma.rs:6722-6839`
  - this mode has benchmark surface in:
    - `crates/app_core/benches/double_ma_opt.rs:2026-2200`
- Why it matters:
  - more seeds can mean more CPU-side prep and more candidate work in later rungs
- Smallest justified fix:
  - fix the goal-length bug first, then benchmark representative VDPS paths before and after
- Review judgment:
  - benchmark-sensitive

### 2. Secondary perf costs exist, but they are smaller than the refill bug

- Evidence:
  - repeated `BTreeSet` allocations in all refill helpers:
    - `200-224`
    - `247-283`
    - `285-327`
  - repeated sort/dedup in `vdps_decimated_periods`:
    - `27-44`
- Review judgment:
  - do not touch these first
  - fix correctness/contract issues before micro-optimizing helper allocations

## Code-Quality Improvement Register

- Safe now:
  - remove the unused `objective_score` import
  - document sorted-slice preconditions
  - harden telemetry merging against mismatched rung plans
  - add focused tests for refill caps and score-mode-preserving merges
- Coordination required:
  - thread `score_mode` through partial-result merging and call sites
  - benchmark the refill-budget fix because it changes survivor/frontier size
- Defer until later pass:
  - micro-optimizing container choices or sort/dedup patterns

## Technical-Debt Reduction Register

- Safe to reduce now:
  - hidden preconditions
  - unused import noise
  - brittle telemetry merge behavior
- Coordination required:
  - `score_mode` preservation through rung merges
  - refill-budget semantics
- Useful debt that should not be “cleaned up” casually:
  - the modular separation between VDPS helper logic and the larger `double_ma.rs` runtime

## Performance Opportunity Register

- Safe structural improvements:
  - none beyond correcting the refill-budget bug
- Benchmark-sensitive candidates:
  - refill-budget correction
  - score-mode-preserving partial merge if it changes survivor composition
- Not the right place to chase performance:
  - low-level allocation tuning before the rung budget/ranking contract is corrected

## Observability And Diagnostics Gaps

- telemetry merge does not make incompatible rung plans explicit
- the file does not explain that some helpers require sorted ascending inputs

## Implementation Staging Guidance

- Safe now:
  - docs
  - tests
  - telemetry-merge hardening
  - unused import cleanup
- Coordination required:
  - fixed refill-budget logic
  - `score_mode` threading into partial-result merge
- Defer until later pass:
  - micro-optimizations after benchmarks

## Change Blast Radius

- Local only:
  - docs, tests, unused import cleanup
- Same subsystem:
  - rung survivor selection
  - VDPS telemetry output
- Cross-backend/runtime:
  - `double_ma.rs` rung loop and any downstream analysis of VDPS telemetry

## Open Questions Or Assumptions

- assumption:
  - caller period slices are intended to remain sorted ascending everywhere this module is used
- open question:
  - whether the repo wants telemetry merge to be strict-and-drop on mismatch or to preserve multiple incompatible telemetry blocks separately

## Recommended Action

`Small surgical refactor`

The fixes are narrow, but they are not trivial cleanup:

- cap refill helpers against a fixed target
- preserve `score_mode` when merging and truncating partial rung results
- harden telemetry merge behavior
- add focused regression tests before touching any perf-adjacent helper internals

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core vdps_merge_partial_results -- --nocapture`
- `cargo test -p ta_app_core gpu_vdps -- --nocapture`
- add and run focused tests for:
  - refill helpers honoring `target_count`
  - non-`None` `score_mode` through `vdps_merge_partial_results`
  - mismatched telemetry merge behavior
- if refill or survivor semantics change, run the representative VDPS benchmark surface before and after:
  - `crates/app_core/benches/double_ma_opt.rs` VDPS scenarios

## Regression Risk And Feature-Preservation Constraints

- do not narrow or remove `GpuVdps`; fix the helper contracts while preserving the mode itself
- expect the refill-budget fix to change survivor/frontier size, so treat it as semantics-sensitive and benchmark-sensitive
- if `score_mode` is threaded through partial merges, verify that final best/top results improve in fidelity without breaking existing default-score behavior
