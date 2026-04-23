# Second-Pass Review: `crates/app_core/src/composite_cpu.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_cpu.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/composite_cpu.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a broad but mostly sound execution owner whose main risks were route reporting fidelity and semantics-sensitive execution modes like `include_all`.

## Stale Check

The live source still owns:

- composite strategy execution and reduction
- route metadata reporting
- `include_all` collection behavior

The first-pass packet remains current.

## Second-Pass Research Log

1. Rayon `ParallelIterator`
   - Source: <https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html>
   - Why consulted in second pass:
     - re-check determinism assumptions around parallel reduction
   - Additional constraint informed:
     - non-`include_all` reduction order should not be treated as stable tie-order contract

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground the public request/result contract review
   - Additional constraint informed:
     - route labeling and execution-mode behavior are part of the public surface

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/archive/12.6.2/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - reassess host-precompute-plus-upload base GPU posture and fallback reporting
   - Additional constraint informed:
     - route reporting should stay honest even when helper-level CUDA work falls back

## Delta vs First Pass

### 1. `include_all = true` is even more clearly an execution-mode switch, not only an output-shape toggle

- Why this matters:
  - it changes collection strategy, guards, and serial/parallel behavior
- Practical implication:
  - preserve it as semantics-sensitive runtime policy

### 2. Honest route reporting remains the highest-value bounded hardening

- Why this matters:
  - helper-level CUDA fallback can still leave runs labeled more GPU-assisted than they really were
- Practical implication:
  - if the file is touched, prioritize route-truthfulness over structure cleanup

## Verification If Changed

- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` if hot-path behavior changes
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` if GPU-route behavior changes and hardware is available

## Regression And Feature-Preservation Constraints

- Preserve current strategy coverage and prescan interactions
- Preserve current `include_all` semantics and finalist exactification behavior
- Do not relabel mixed CPU/GPU routes in a way that hides actual execution

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly tightens two points: `include_all` is a real execution-mode switch, and route-report truthfulness is still the most valuable bounded cleanup target.
