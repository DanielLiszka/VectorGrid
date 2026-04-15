# Review Packet: `crates/app_core/src/double_ma_mode.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_mode.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Well-bounded policy helper with no proven defect; the main review value is documenting the mode-selection heuristics and treating the large-search threshold as benchmark-sensitive policy`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Internal-policy-surface`, `Benchmark-sensitive`

## What This File Owns

This module owns optimization-mode selection and mode/output compatibility validation for Double-MA optimization.

It owns:

- mode-selection input context:
  - `ModeSelectionContext`: `crates/app_core/src/double_ma_mode.rs:5-14`
- auto/manual mode resolution:
  - `resolve_optimization_mode`: `16-49`
- mode/output compatibility validation:
  - `validate_optimization_mode`: `51-80`

## Classification

- Public surface: no
- Hot path: low
- CUDA/runtime-critical: indirect
  - influences whether GPU Vdps or other execution paths are chosen
- Strategy-local: yes
- Low-risk support code: yes, with benchmark-sensitive heuristics

## Direct Interactions

- consumed from:
  - `crates/app_core/src/double_ma.rs:24-26`
- depends on:
  - `ta_optimizer::OptimizationMode`
  - `GpuExecutionPolicy` and `OptimizationModeResolved` from `double_ma.rs`

## Standards And Research Log

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file encodes policy heuristics that can surprise callers if left implicit
   - Standard informed:
     - policy helpers should remain predictable and well-documented

2. Rust Book, How to Write Tests
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why it was consulted:
     - heuristic branches should be covered with direct tests
   - Standard informed:
     - branch-selection rules should be pinned explicitly

3. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file promotes `Auto` to `GpuVdps` for certain large GPU-resident searches
   - Standard informed:
     - thresholds that alter GPU execution policy should be treated as measurement-backed policy, not casual cleanup targets

## Findings

## Correctness And Safety

### 1. No correctness defect was proven

- Evidence:
  - mode resolution is simple and covered by unit tests:
    - `82-152`

## Module Boundaries And Technical Debt

### 1. The `large_search > 500_000` threshold is a hidden benchmark-policy constant

- Evidence:
  - `large_search` is defined as:
    - `crates/app_core/src/double_ma_mode.rs:30`
  - that threshold directly changes `Auto` behavior between `Grid`, `CoarseToFine`, and `GpuVdps`:
    - `32-46`
- Why it matters:
  - changing this constant can materially change runtime selection and benchmark outcomes
- Smallest justified fix:
  - document the heuristic and treat changes as benchmark-sensitive
- Review judgment:
  - policy debt, not a bug

### 2. The file is clean, but the exact-output forcing rules should be documented

- Evidence:
  - `include_all`, `export_all_csv_path`, or `heatmap_bins > 0` force `Grid`:
    - `24-28`
- Why it matters:
  - this is a meaningful user-facing policy rule encoded in a small helper
- Smallest justified fix:
  - document the rule at module level or in the calling public request docs

## Tests And Missing Verification

### 1. Existing tests are good, but they do not pin the exact large-search boundary

- Evidence:
  - current tests cover branch behavior broadly:
    - `101-151`
  - no direct test around `500_000` versus `500_001`
- Smallest justified fix:
  - add a threshold-boundary test for the large-search heuristic

## Recommended Action

`Document only`

The implementation is sound. The main risk is unspoken heuristic policy, not code quality.

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core auto_mode_only_adapts_when_grid_outputs_are_disabled -- --nocapture`
- `cargo test -p ta_app_core auto_mode_prefers_vdps_for_gpu_resident_large_runs_when_route_is_viable -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- do not change the large-search heuristic without benchmark evidence
- preserve output-compatibility validation unless downstream request surfaces are updated together
