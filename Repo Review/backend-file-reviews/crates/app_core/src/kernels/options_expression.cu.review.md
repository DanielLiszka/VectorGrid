# Review Packet: `crates/app_core/src/kernels/options_expression.cu`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kernels/options_expression.cu`
- Packet status: `Reviewed`
- Public-review readiness: `Compact and understandable, but not fully review-ready because premium/validity assumptions and robust-score weighting are not documented in the kernel contract`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `CUDA-kernel-critical`, `Options-workflow`, `Semantics-sensitive`

## What This File Owns

`options_expression.cu` owns the dedicated options-expression split scoring kernels:

- per-split tuple aggregation kernel: `1-110`
- search/holdout/conservative/harsher stress kernel plus robust-score blend: `112-302`

## Classification

- Public surface: indirectly
  - Rust-side launch code depends on kernel names and output layout
- Hot path: yes
- CUDA/runtime-critical: yes
- Strategy-local: yes
  - options-expression workflow only

## Direct Interactions

### Backend interaction map

- main Rust caller:
  - `crates/app_core/src/options_expression_gpu.rs`
- build/compilation surface:
  - `crates/app_core/build.rs`

### Downstream verification impact if changed

- changing output semantics changes ranking and robust-score ordering for options candidates
- changing premium handling changes risk-adjusted metrics and therefore finalist selection
- changing kernel names or argument order breaks Rust launches

## Standards And Research Log

This packet was written after targeted web research focused on CUDA kernel maintenance and keeping score aggregation rules explicit.

### Sources consulted

1. NVIDIA CUDA C Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - this file is a small, handwritten scoring kernel pair with direct Rust launch coupling
   - Standard informed:
     - kernel contracts should be explicit because debugging surface is narrow

2. NVIDIA CUDA C Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - the kernels are simple enough that clarity improvements are safer than structural rewrites
   - Standard informed:
     - preserve existing work partitioning unless measurement says otherwise

## Findings

## Correctness And Safety

### 1. Both kernels divide by `premium` under the assumption that valid outcomes always have positive premium, but the assumption is not stated here

- Evidence:
  - `return_on_premium = pnl / premium` appears in `options_expression_score_split_f64`: `31-41`
  - the same pattern appears in `options_expression_accumulate_lane`: `64-84`
- Why this matters:
  - if upstream validity ever includes zero-premium rows, these kernels can emit infinities or NaNs into downstream ranking
- Smallest justified fix:
  - document the non-zero-premium precondition at the kernel boundary and mirror it in the Rust caller; only add a device-side guard if semantics are explicitly agreed

## Contract Clarity

### 2. The robust-score blend weights are product semantics, not obvious mathematical invariants

- Evidence:
  - the final blend uses `0.45 / 0.25 / 0.15 / 0.15` at `296-301`
- Why this matters:
  - future maintainers need to know this is an intentional ranking heuristic, not a temporary placeholder
- Smallest justified fix:
  - add a short comment describing the blend as the current options robustness policy

## Tests And Missing Verification

### 3. Verification is caller-driven; this file has no kernel-local safety net

- Evidence:
  - the file contains kernels only
  - Rust caller/orchestration lives in `options_expression_gpu.rs`
- Why this matters:
  - any change here must be validated through the end-to-end options GPU flow
- Smallest justified fix:
  - keep changes small and require caller-level regression coverage

## Code-Quality Improvement Register

- Safe now:
  - document the non-zero-premium assumption
  - document the robust-score weight policy
- Coordination required:
  - adding device-side guards if upstream semantics are ambiguous
- Defer:
  - any attempt to merge this file back into the larger CUDA translation unit without a build/perf reason

## Technical-Debt Reduction Register

- Safe now:
  - comments only
- Defer:
  - semantic changes to robust-score weighting
- Do not casually remove:
  - current kernel names
  - current split/holdout output layout

## Performance Opportunity Register

- Safe structural work:
  - comments only
- Benchmark-sensitive:
  - changing thread/block policy
  - adding extra device-side branching around premium checks

## Recommended Action

`Small surgical refactor`

The right change here is explicit contract documentation. The kernels are small enough that a comment-level hardening pass is useful, but score semantics should not change without validating the full options workflow.

## Exact Local Verification Required If Changed

1. Rebuild the CUDA module:
   - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
2. Representative options GPU verification:
   - `cargo test -p ta_app_core options_expression_gpu --features cuda,cuda-backtest-kernel -- --nocapture`
3. End-to-end options workflow verification if ranking semantics change:
   - run the current options expression GPU regression/workflow coverage used by the repo

### Verification run for this packet

- Not run in this review pass:
  - CUDA build/tests for `options_expression.cu`

## Regression Risk And Feature-Preservation Constraints

- Preserve the current kernel argument and output layout unless the Rust caller is updated in lockstep
- Treat zero-premium handling as a semantics-sensitive contract question, not a casual defensive edit
- Preserve the current robust-score weighting unless product intent changes and downstream ranking is reverified
