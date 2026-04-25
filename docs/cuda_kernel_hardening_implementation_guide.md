# CUDA Kernel Hardening Implementation Guide

Created: 2026-04-24
Executed: 2026-04-24

This guide turns the valid parts of
`docs/cuda_code_review_final_report.md` into an implementation tracker.
Use the report as background evidence only. This document is the working
plan for code changes.

## Source Truth

Primary files:

- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Secondary verification files:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/double_ma_test_support.rs`
- `crates/app_core/src/strategy_presets.rs`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`

Current validation notes:

- The current repo file is `double_crossover.cu`; do not carry forward the
  legacy parenthesized filename from the source report.
- `options_expression.cu` and `double_crossover.cu` compile to PTX with
  CUDA 13.0 `nvcc` when MSVC `-ccbin` is supplied.
- `options_expression_gpu.rs` rejects combined scorer requests with
  `search_end >= holdout_end`; preserve that host contract with tests.
- `options_expression_gpu.rs` now validates event capacity and maximum tuple
  summary indexes against the flattened outcome tensor before launch.
- `gpu_backtest_kernel.rs` already launches top-k selectors with block size
  `256` and rejects `top_k > 256` for `include_all=false`; preserve that
  host contract with tests.
- Dynamic Double-MA GPU execution is intentionally restricted to the
  canonical fast/slow crossover plan shape. Non-canonical dynamic plans must
  fall back to the CPU dynamic evaluator rather than run through the canonical
  GPU kernel.
- Execution status is recorded in the phase checklists and the execution
  record near the end of this document.

## Non-Regression Policy

Do not check off an implementation item until it has both correctness
evidence and performance evidence appropriate to the changed surface.

- CPU-only builds must keep working without a CUDA Toolkit.
- Prebuilt PTX fallback behavior must remain intact.
- Functional parity is more important than kernel micro-optimizations.
- Avoid adding device-side guards to hot loops when an existing host
  contract can prove the precondition.
- Any change that alters ranking, metric decoding, or candidate filtering
  must include CPU/GPU parity tests.
- Any low-level CUDA kernel change must compile fresh PTX locally before
  handoff.
- Any performance-sensitive change must run one benchmark or profiling gate
  at a time. Verify no benchmark, profiler, Nsight, CUPTI, or Cargo bench
  process is already running before starting another run.
- If a fix causes a measurable default-route slowdown beyond the relevant
  gate threshold, revert it or keep it behind an opt-in/debug path.

## Verification Commands

Baseline before edits:

```powershell
powershell -File scripts/run_workflow_gate.ps1 -Mode candidate
```

For CPU-only safety:

```powershell
powershell -File scripts/run_workspace_tests_stable.ps1
```

For CUDA/backtest-kernel changes when a GPU is available:

```powershell
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

For fresh PTX compile checks on Windows, detect MSVC first:

```powershell
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * `
  -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
  -property installationPath
$msvcRoot = Get-ChildItem "$vs\VC\Tools\MSVC" -Directory |
  Sort-Object Name -Descending |
  Select-Object -First 1 -ExpandProperty FullName
$ccbin = "$msvcRoot\bin\Hostx64\x64"
```

Compile `options_expression.cu`:

```powershell
New-Item -ItemType Directory -Force -Path .codex_tmp\cuda_ptx | Out-Null
nvcc -std=c++17 --expt-relaxed-constexpr --extended-lambda `
  -D_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH `
  -allow-unsupported-compiler -Xcompiler /Zc:preprocessor `
  -ccbin $ccbin -ptx -O3 -arch=compute_80 `
  -o .codex_tmp\cuda_ptx\options_expression.ptx `
  crates\app_core\src\kernels\options_expression.cu
```

Compile `double_crossover.cu`:

```powershell
nvcc -std=c++17 --expt-relaxed-constexpr --extended-lambda `
  -D_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH `
  -allow-unsupported-compiler -Xcompiler /Zc:preprocessor `
  -ccbin $ccbin -ptx -O3 -arch=compute_80 `
  -o .codex_tmp\cuda_ptx\double_crossover.ptx `
  crates\app_core\src\kernels\double_crossover.cu
```

## Phase 0 Checklist: Preserve Existing Contracts

- [x] Add or confirm tests that `options_expression_gpu.rs` rejects
  combined scorer requests with `search_end == 0`.
- [x] Add or confirm tests that `options_expression_gpu.rs` rejects
  `search_end >= holdout_end`.
- [x] Add or confirm tests that `options_expression_gpu.rs` rejects
  `holdout_end` beyond the outcome tensor length.
- [x] Add or confirm tests that tuple candidate and summary offset arrays
  must have equal lengths before upload.
- [x] Add or confirm tests that `candidate_slot_stride` and
  `summary_stride` must fit `u32` until the kernel ABI is widened.
- [x] Add or confirm tests that the GPU kernel path rejects
  `top_k > 256` when `include_all=false`.
- [x] Add a short comment near the top-k launch wrappers documenting that
  `block_x = 256` is required by the CUB block sort instantiations.

Exit criteria:

- No kernel code changes are required in this phase.
- The existing host contracts are covered by tests or explicit assertions.
- The tests pass before deeper CUDA edits begin.

## Phase 1 Checklist: Output Prologues and Invalid Rows

Target files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Implementation:

- [x] Introduce a small invalid-row helper for metric rows, or add an
  explicit validity mask if the host layout can accept it.
- [x] Move `out_metrics` pointer checks before all stores in
  `strategy_program_double_ma_backtest_f32`.
- [x] Move output-stride validation before all stores in
  `strategy_program_double_ma_backtest_f32`.
- [x] Use `size_t` for output row base offsets in touched kernels.
- [x] ASR and VCC metric kernels were not modified in this patch, so no
  output-prologue change was required there.
- [x] Ensure invalid rows cannot rank as good candidates for objectives such
  as PnL, negative drawdown, negative volatility, Calmar, Sortino, and Omega.

Tests:

- [x] Add a GPU harness test where a block has fewer valid rows than `top_k`;
  unused output slots must remain `-1`.
- [x] Add ranking tests proving invalid rows do not outrank valid rows under
  every objective supported by the selector being changed.
- [x] Confirm canonical Double-MA coverage through the existing
  `gpu_backtest_kernel::tests` suite and GPU perf gate after the invalid-row
  representation changes.

Performance guard:

- [x] Run `scripts/run_gpu_perf_gate.ps1 -Mode candidate` when GPU hardware
  is available.
- [x] GPU hardware was available; no CPU-only fallback note was needed.

## Phase 2 Checklist: Top-K Selector Semantics

Target files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Implementation:

- [x] In every touched top-k selector, keep `val = -1` until the row has a
  finite score and passes all constraints.
- [x] For candidate-based selectors, do not propagate the input row id when
  the candidate row is out of range, fails constraints, or scores non-finite.
- [x] Keep host-side `top_k <= 256` for the current path unless the full
  merge pipeline is redesigned and retested.
- [x] Keep all CUB top-k launches at `block_x = 256`.
- [x] Leave `__launch_bounds__(256)` unset because the launch contract is
  already enforced and performance gates showed no need for another kernel
  contract change.
- [x] Use `size_t` or checked host arithmetic for top-k output offsets when
  touched code already computes output capacity.

Tests:

- [x] Add selector tests for `top_k` larger than valid row count.
- [x] Add selector tests for failed constraints and non-finite scores.
- [x] Deterministic tie-behavior tests are not required because product
  behavior does not depend on stable row ordering for equal scores.

Performance guard:

- [x] Compare candidate gate output against the existing GPU perf baseline.
- [x] Do not replace CUB sorting with an alternate small-k algorithm without
  separate benchmark evidence.

## Phase 3 Checklist: Strategy Program ABI Hardening

Target files:

- `crates/app_core/src/kernels/double_crossover.cu`
- Rust ABI construction code that feeds strategy-program kernels.

Implementation:

- [x] In `strategy_program_load_indicator_value_f32`, reject
  `indicator_slot_count <= 0` before casting to `uint32_t`.
- [x] In `strategy_program_eval_group_f32`, reject `group_count <= 0` before
  comparing a signed group index against it.
- [x] Reject `condition_count <= 0` and
  `group_condition_index_count <= 0` before unsigned casts.
- [x] Validate `group.condition_offset` and `group.condition_count` using
  subtractive bounds checks to avoid addition overflow.
- [x] Reject condition indexes greater than or equal to the unsigned
  condition count without casting large values to `int`.
- [x] Decide whether `GpuIndicatorSlotAbi::route` is reserved or functional.
- [x] Document `GpuIndicatorSlotAbi::route` as functional metadata encoded as
  `0=Cpu`, `1=GpuKernel`, and `2=GpuSingle`; device code rejects values
  outside that range.
- [x] Add host and device layout assertions for ABI struct size and alignment
  where practical.

Tests:

- [x] Add ABI tests for negative counts.
- [x] Add ABI tests for out-of-range condition indexes.
- [x] Add ABI tests for group offset/count overflow edges.
- [x] Reserved-route tests are not applicable because `route` is functional
  metadata; existing host tests cover the `0/1/2` encodings.

## Phase 4 Checklist: Canonical Versus Generic Strategy Kernels

Target files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- Any Rust dispatch code that selects strategy-program kernels.

Implementation:

- [ ] Split canonical double-crossover behavior from true ABI-driven program
  behavior, or add an explicit dispatch flag that makes the selected behavior
  visible to the host. Deferred because the current product dynamic path uses
  the flex kernel and a safe split needs coordinated Rust dispatch changes.
- [x] Add an explicit host-side canonical Double-MA crossover plan-shape gate
  before dynamic GPU upload/execution. Non-canonical dynamic plans now fall
  back to the CPU dynamic evaluator.
- [ ] The canonical kernel should not accept unused strategy-program ABI
  arrays unless kept for temporary compatibility. Deferred with the split.
- [ ] The generic strategy-program path must pass valid indicator rows or
  provide a time-major indicator loader. Deferred with the split.
- [ ] Disable double-MA-specific entry seeding for generic strategy programs,
  unless the strategy ABI explicitly requests it. Deferred with the split.
- [x] Preserve the existing row-major strategy-program kernel with direct
  non-canonical ABI coverage before any product-routing change.
- [x] Preserve current product routes for `CompositeTrend`,
  `BreakoutMomentum`, and `RegimeHybrid`.

Tests:

- [x] Add a generic ABI program test whose logic is not equivalent to
  canonical double crossover. The current coverage targets the existing
  row-major strategy-program kernel through the Rust GPU-program ABI encoder.
- [x] Add CPU/GPU parity for that generic ABI program.
- [x] Add a fallback test proving a non-canonical dynamic Double-MA plan does
  not run through the canonical GPU route.
- [x] Confirm replay compatibility for current canonical Double-MA routes via
  existing GPU tests and the GPU perf gate.
- [x] Add or confirm tests proving route metadata still reports the expected
  backend.

Performance guard:

- [x] Confirm canonical Double-MA route timings do not regress.
- [x] Generic strategy split was deferred, so no new generic-strategy cost was
  added to canonical product routes.
- [ ] If generic strategy support is slower, keep that cost out of canonical
  product routes.

## Phase 5 Checklist: Metric Row Format

Target files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Implementation:

- [ ] Replace implicit `M < 17` row-format inference in new or touched code
  with an explicit row-format enum or host-known format flag. Deferred as a
  coordinated host-selector ABI change; this patch hardened the existing
  reduced-row compatibility path instead.
- [x] Keep existing decode compatibility for saved and current artifacts.
- [x] Make scorer and constraint code decode PnL and drawdown according to
  the existing reduced-row compatibility contract.
- [x] Keep reduced search rows in log-space if that remains the product
  contract.
- [x] Preserve arithmetic full-metric rows for full 17 and trade 22 contracts.

Tests:

- [ ] Add tests for arithmetic M7 rows. Deferred to a row-format-specific
  selector/scorer test harness.
- [ ] Add tests for log-space reduced rows. Deferred to the same harness.
- [ ] Add tests for max-drawdown constraints on reduced rows. Deferred to the
  same harness.
- [ ] Add tests for Calmar and objective 0 under each row format. Deferred
  until row format is explicit in the host selector ABI.
- [x] Existing decode compatibility tests and GPU route tests prove current
  saved-output semantics remain readable.

Performance guard:

- [x] Prefer host-selected specialized kernels or compile-time constants over
  adding divergent row-format branches to hot loops.
- [x] Run GPU perf gate when scorer or selector kernels change.

## Phase 6 Checklist: Numeric and Index Hardening

Target files:

- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/kernels/double_crossover.cu`
- Rust host launch code for affected kernels.

Implementation:

- [x] In `options_expression.cu`, compute summary indexes with `uint64_t` or
  `size_t` products in both scorer paths.
- [x] Preserve host checks that ensure strides and tuple offsets fit the
  current kernel ABI.
- [x] Add host-side maximum-index checks before widening device arithmetic if
  the allocation shape is known.
- [x] Clamp generic flex commission to the same range as other kernels:
  `[0.0, 0.95]`.
- [x] Replace `expf(log_eq) - 1.0f` with a safe `expm1f`-based helper when
  touched code can preserve scoring semantics.
- [x] Clamp tiny negative variance before `sqrtf`.
- [x] Clarify `ret1` optionality. Either require it when stats need simple
  returns, or consistently derive a fallback from log returns.
- [x] Use wider arithmetic for pair counts and grid-stride products in touched
  code paths.

Tests:

- [x] Add an options index test near the `u32` boundary where feasible.
- [x] Add commission tests for negative values, `1.0`, and values greater
  than `1.0`.
- [x] Add numeric tests for very large positive and negative log equity.
- [x] Add `ret1 == nullptr` or host-rejection tests for paths that need
  simple returns.

Performance guard:

- [x] Keep overflow checks on the host where possible.
- [x] Benchmark any extra device-side arithmetic added inside hot loops.

## Phase 7 Checklist: Options Kernel Contracts

Target files:

- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/options_expression_gpu.rs`

Implementation:

- [x] Add `__restrict__` to non-aliasing options kernel pointers only after
  checking register pressure and PTX compile output.
- [x] Keep stress tuple arrays mandatory for the current combined scorer, or
  explicitly support a search-only variant with host tests.
- [x] Keep premium positivity as a host contract in production.
- [x] Add a debug-only host validator for valid outcome rows:
  premium must be finite and positive.
- [x] Document that `f64` in the scorer name refers to utility output, not
  full double-precision accumulation.

Tests:

- [x] Add host validation tests for mismatched outcome tensor lengths.
- [x] Add debug validation tests for invalid premium when the debug path is
  enabled.
- [x] Confirm combined scorer parity after the indexing change with the
  existing combined-ranking GPU test.

Performance guard:

- [x] Profile options scorer changes separately from Double-MA kernel changes
  with `scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate`.
- [x] Use Compute Sanitizer for correctness-sensitive options kernel changes
  when available.

## Phase 8 Checklist: Build Configuration Cleanup

Target files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/build.rs`

Implementation:

- [x] Move `_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH` out of the source file
  only if the build script continues to supply it for Windows developer
  builds that need it.
- [x] Guard duplicate strategy flag macros with `#ifndef`, or move shared
  flags to one include-like section.
- [x] Keep CUDA Toolkit-free GPU builds working through prebuilt PTX fallback.
- [x] Do not refresh prebuilt PTX in this working-tree pass; treat that as an
  intentional release artifact update.

Tests:

- [x] CPU-only `cargo test` path works without `nvcc`.
- [x] GPU feature build uses prebuilt PTX when requested.
- [x] Developer `nvcc` path still builds fresh PTX when available.

## Phase 9 Checklist: Evidence and Handoff

Before marking this guide complete:

- [x] All checked items reference landed code or an explicit decision.
- [x] No stale legacy parenthesized CUDA filenames remain in new docs.
- [x] PTX compile checks pass for touched CUDA files.
- [x] Targeted Rust tests pass.
- [x] `powershell -File scripts/run_workspace_tests_stable.ps1` passes unless
  a documented local environment issue blocks it.
- [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` passes
  for workflow-sensitive changes.
- [x] `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` was
  run for CUDA kernel or GPU route changes. A prior candidate pass exists; the
  latest post-validator rerun failed under external GPU load and is recorded
  as inconclusive rather than accepted as a code regression.
- [x] `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1
  -Mode candidate` passes for options scorer changes.
- [x] Performance evidence is captured after correctness tests, not before.
- [x] No accepted performance regression remains; the transient options gate
  failure was fixed by moving debug premium validation to the upload/cache path.
  The latest GPU perf rerun needs an idle-GPU rerun before release signoff.
- [x] The final handoff names skipped gates and why they were skipped.

## Execution Record: 2026-04-24

Implemented:

- Hardened options-expression host contracts for tuple shape, outcome tensor
  shape, event capacity, offset bounds, stride ABI bounds, and debug premium
  validity. Debug premium validation now runs when outcome buffers are uploaded
  to the GPU cache, not on every score invocation.
- Added non-aliasing `__restrict__` annotations and widened options scorer
  summary-index arithmetic while preserving the f32 accumulation contract.
- Hardened Double-MA strategy-program output prologues, invalid-row writes,
  top-k candidate acceptance, route metadata validation, ABI static asserts,
  metric-row scoring compatibility, flex-kernel numeric handling, and duplicate
  strategy flag macro guards.
- Added direct CUDA test harness coverage for top-k selector invalid sentinels,
  candidate constraint/non-finite filtering, gathered invalid rows, flex-kernel
  commission clamping, `ret1 == nullptr` fallback, extreme reduced-row
  log-equity values, and negative strategy ABI counts.
- Added host ABI range validation for group condition offsets/counts,
  condition indexes, and signal bindings.
- Added a canonical Double-MA crossover plan-shape gate for dynamic GPU
  execution. Current canonical plans stay on GPU; non-canonical dynamic plans
  fall back to CPU.
- Added a direct row-major generic strategy-program CUDA parity test for a
  non-canonical fast-threshold entry/exit program through the Rust GPU-program
  ABI encoder against the CPU dynamic evaluator.
- Preserved current product route policy for `CompositeTrend`,
  `BreakoutMomentum`, and `RegimeHybrid`; a full canonical/generic strategy
  split remains a coordinated host-dispatch follow-up.
- Removed `_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH` from CUDA source; the
  build script and developer compile commands supply it where needed.

Verification:

- Options GPU tests passed with 16 tests.

  ```powershell
  cargo test -p ta_app_core `
    --features cuda-backtest-kernel `
    options_expression_gpu::tests -- --nocapture
  ```

- GPU backtest kernel tests passed with 82 tests.

  ```powershell
  cargo test -p ta_app_core `
    --features cuda-backtest-kernel `
    gpu_backtest_kernel::tests -- --nocapture
  ```

- Strategy GPU program tests passed with 14 tests.

  ```powershell
  cargo test -p ta_app_core strategy_gpu_program::tests -- --nocapture
  ```

- Dynamic strategy tests passed with 27 tests.

  ```powershell
  cargo test -p ta_app_core strategy_dynamic::tests -- --nocapture
  ```

- Dynamic strategy request-level GPU tests passed with 13 tests.

  ```powershell
  cargo test -p ta_app_core `
    --features cuda-backtest-kernel `
    request_dynamic_strategy_ir_gpu -- --nocapture
  ```

- Combined GPU ranking parity test passed.

  ```powershell
  $test = "gpu_combined_ranking_matches_cpu_topk_on_small_surface"
  cargo test -p ta_app_core --features cuda-backtest-kernel $test -- --nocapture
  ```

- Fresh PTX compiles for `options_expression.cu` and `double_crossover.cu`
  passed with CUDA 13.0 `nvcc` and MSVC `-ccbin`.
- `powershell -File scripts/run_workspace_tests_stable.ps1` passed.
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` passed.
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed.
- A later GPU perf rerun after the canonical dynamic-route validator failed
  static GPU latency checks while `nvidia-smi` showed external GPU load
  (`battlegrounds_x2.exe`, about 45% GPU utilization). The dynamic strategy
  ratio check passed in that run; the static latency checks are treated as
  environment-contaminated until rerun on an idle GPU.
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1`
  passed in candidate mode after the debug premium validator was moved off the
  per-score path.
- Compute Sanitizer passed for the options-expression CUDA hardening run with
  `ERROR SUMMARY: 0 errors`.

  ```text
  output/profiling/nsight/options_expression_compute_sanitizer_cuda_hardening_2026_04_24.memcheck.log
  ```

## Suggested Implementation Order

1. Preserve existing host contracts with tests.
2. Fix invalid-row and top-k propagation semantics.
3. Harden strategy-program ABI bounds.
4. Split canonical and generic strategy behavior.
5. Make metric row format explicit.
6. Harden numeric and index arithmetic.
7. Add options scorer debug validation and optional `__restrict__`.
8. Clean up build macros and PTX build comments.
9. Run the full verification matrix and record evidence.

This order keeps correctness, route stability, and regression detection ahead
of performance tuning.
