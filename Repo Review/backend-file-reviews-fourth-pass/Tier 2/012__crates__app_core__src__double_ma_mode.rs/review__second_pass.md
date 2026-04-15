# Second-Pass Review: `crates/app_core/src/double_ma_mode.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_mode.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_mode.rs.review.md`
- First-pass action bucket: `Document only`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as product policy logic whose main need is sharper documentation, not structural change.

## Stale Check

The live source still owns:

- mode forcing for export/output constraints
- `Auto` routing heuristics
- `GpuVdps` promotion thresholds and gating

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - verify expectations for small policy helpers
   - Additional constraint informed:
     - these helpers are policy surface, not generic utilities

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground policy and fallback documentation needs
   - Additional constraint informed:
     - current thresholds and forcing rules should be documented as policy

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep route thresholds benchmark-sensitive rather than cleanup-driven
   - Additional constraint informed:
     - the `500_000` threshold remains a measured policy boundary, not casual tuning fodder

## Delta vs First Pass

### 1. Grid-output forcing is intentionally duplicated across product and app-core boundaries

- Why this matters:
  - policy changes need to move in both places together, not "dedupe" one side blindly

### 2. `Auto` only promotes to `GpuVdps` under a narrower condition set than the file name alone suggests

- Practical implication:
  - keep the current route policy explicit and documented

## Verification If Changed

- `cargo test -p ta_app_core auto_mode_only_adapts_when_grid_outputs_are_disabled -- --nocapture`
- `cargo test -p ta_app_core auto_mode_prefers_vdps_for_gpu_resident_large_runs_when_route_is_viable -- --nocapture`
- add threshold-boundary tests around `500_000` vs `500_001`

## Regression And Feature-Preservation Constraints

- Keep exact-output forcing aligned with `commands_optimize.rs`
- Do not change the large-search threshold without benchmark evidence
- Preserve `GpuVdps` gating on both route viability and GPU-resident preference

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Document only` bucket remains correct. The second pass mainly clarifies that this file is policy-bearing in two directions at once: output forcing is intentionally duplicated across layers, and `Auto` promotion to `GpuVdps` is deliberately narrower than a casual reading suggests.
