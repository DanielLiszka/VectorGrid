# Second-Pass Review: `crates/app_core/src/double_ma_vdps.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_vdps.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_vdps.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as benchmark-sensitive frontier logic where refill counts, merge semantics, and telemetry shape are more important than structural elegance.

## Stale Check

The live source still owns:

- partial-result merging
- refill injectors
- telemetry merge and rung-plan handling

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust slice `partition_point`
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point>
   - Why consulted in second pass:
     - re-check sorted-slice assumptions in period helpers
   - Additional constraint informed:
     - sorted ascending period arrays remain a caller invariant worth keeping explicit

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground explicit helper-contract review
   - Additional constraint informed:
     - refill-count behavior and merge semantics remain public enough to warrant focused tests

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep frontier/budget changes benchmark-sensitive
   - Additional constraint informed:
     - refill and merge cleanup should not bypass performance evidence

## Delta vs First Pass

### 1. The refill helpers do appear to overshoot their intended budgets because the loop target moves with `current.len()`

- Why this matters:
  - this is a real budget bug, not only awkward implementation style
- Practical implication:
  - still a good bounded hardening target if touched

### 2. `vdps_merge_partial_results` still risks pruning under the wrong ranking contract when non-`None` `score_mode` is dropped

- Practical implication:
  - preserve requested `score_mode` semantics through partial-result merging

## Verification If Changed

- `cargo test -p ta_app_core vdps_merge_partial_results -- --nocapture`
- add focused tests for exact refill-count stopping in all three injectors
- add telemetry-merge rejection tests when rung plans differ

## Regression And Feature-Preservation Constraints

- Keep sorted ascending period arrays as a caller invariant
- Preserve requested `score_mode` semantics through partial-result merging
- Keep refill counts and telemetry behavior stable unless explicitly hardened with matching tests and benchmarks

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly strengthens two earlier suspicions into concrete risks: refill helpers still appear to overshoot budgets, and partial-result merging can still discard the intended score mode.
