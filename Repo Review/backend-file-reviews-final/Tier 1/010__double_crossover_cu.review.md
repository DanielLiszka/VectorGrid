# Final Consolidated Review: `crates/app_core/src/kernels/double_crossover.cu`

- Review date: `2026-04-14`
- Source file: [double_crossover.cu](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/kernels/double_crossover.cu:1>)
- First-pass packet: [double_crossover.cu.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/kernels/double_crossover.cu.review.md:1>)
- Second-pass packet: [double_crossover.cu.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kernels/double_crossover.cu.review.md:1>)
- Third-pass packet: [double_crossover.cu.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kernels/double_crossover.cu.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/010__crates__app_core__src__kernels__double_crossover.cu/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Benchmark-sensitive; do not touch yet`
- External research anchors:
  - <https://docs.nvidia.com/cuda/cuda-c-programming-guide/>

## Consolidated Judgment

All four passes agree on the right bucket, and the fourth pass gives the clearest implementation guidance. The kernel hot path itself should stay frozen. The safer hardening work is on the Rust side of the boundary: probe enforcement, specialization inventory checks, and fallback regression coverage. The current dynamic-strategy GPU path still behaves like a constrained v1 lowering contract, so the worst possible mistake here would be to perform “cleanup” in the `.cu` file and silently push production traffic onto slower fallback routes.

## Safe-Now Priority

- Keep the kernel ABI layout, numeric encodings, specialization symbols, and row-contract store helpers stable.
- Prioritize Rust-side probe enforcement and regression coverage before any `.cu` edits.
- Treat the current dynamic-strategy GPU path as a constrained v1 lowering contract unless and until the runtime truly launches the dedicated strategy-program kernel end to end.
- If regression hardening is needed, add or tighten specialization/fallback tests rather than editing the kernel body.

## Deferred Work

- Keep packed row-store, launch-shape, CUB top-k, and specialization-family edits deferred behind explicit benchmark budgeting.
- Treat widening operand-source support or rewiring the dedicated strategy-program path as redesign work, not as bounded cleanup.

## Verification If Changed

- If anything around dispatch/probe behavior changes, rerun the relevant Rust-side GPU route tests first
- If the `.cu` file itself changes, run the GPU perf gate and compare shortlist/ranking behavior before trusting the change

## Regression And Feature-Preservation Constraints

- Renaming or losing specialization symbols can fail open into slower fallback dispatch rather than fail hard.
- Touching packed row-store helpers, launch shape, or top-k regions can change occupancy, register pressure, memory traffic, and shortlist behavior.
- Avoid translation-unit restructuring or kernel-family reshaping as part of a cleanup pass.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `double_crossover.cu` at `Benchmark-sensitive; do not touch yet`
- focus any near-term hardening on Rust-side probe and fallback coverage
- leave the kernel hot path frozen unless a benchmark-backed redesign is explicitly in scope
