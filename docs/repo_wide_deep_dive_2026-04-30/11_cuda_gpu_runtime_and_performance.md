# CUDA GPU Runtime And Performance Implementation Plan

## Surface

Primary files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/kernels/*`
- `src-tauri/src/commands_optimize.rs`
- `src-tauri/src/commands_validation.rs`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`

## Current Strengths

- The product GPU path is conventional CUDA execution on Nvidia CUDA cores, not tensor cores.
- Double MA and related profile routes have mature CUDA acceleration and CPU fallback.
- Recent work moved more validation computation to GPU for supported validation modes.
- GPU benchmark scripts and candidate gates already exist.
- CPU-only builds remain an explicit invariant.

## Improvement Plan

### CUDA-01: Route And Residency Policy Consolidation

Risk: Low/medium.

Problem:
GPU capability is described in several places. A route may be correct in Rust but described differently in frontend or docs.

Implementation:

- Add a single checked matrix for:
  - strategy family
  - requested backend
  - ADX on/off
  - optimization route
  - validation route
  - CPU fallback reason
  - residency tier
- Include `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` policy:
  - `CompositeTrend`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback
  - `BreakoutMomentum`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback
  - `RegimeHybrid`: `GpuHybridKernel` in the current ADX-bearing form, `CpuOnly` fallback
- Add frontend tests that compare displayed capability to matrix fixtures.
- Include separate columns for frontend requested backend, normalized requested backend, and Rust-resolved target backend.

Files:

- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`

Verification:

- `cargo test -p ta_app_core composite_profile_route`
- `cargo test -p ta_app_core double_ma_validate`
- `npm --prefix web run test -- src/utils/strategyOptimizationCapabilities.test.ts src/strategies/compositeProfileGpuPolicy.test.ts`

### CUDA-02: Validation Strict Residency Gate

Risk: Medium.

Problem:
GPU validation should be able to run in best-effort public mode and strict evidence mode.

Implementation:

- Reuse the validation residency policy from `03_validation_gpu_and_evidence_pipeline.md`.
- Add a GPU validation perf gate output that records requested route, observed route, CPU tail, and fallbacks.
- Use typed requested policy and observed residency/result fields rather than string-parsed display route values.
- Fail candidate mode only when strict policy is requested.

Files:

- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `web/src/services/validationService.ts`

Verification:

- `cargo test -p ta_app_core double_ma_validate`
- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`

### CUDA-03: Resident Buffer Reuse For Dynamic Indicators

Risk: Medium/high.

Problem:
GPU performance can be lost if derived series or dynamic indicators are repeatedly copied between host and device.

Implementation:

- Inventory current upload/copy points in optimization and validation.
- Reuse VRAM-resident price and derived buffers inside a stage.
- Keep validation stage independent from optimization stage, but avoid redundant copies within validation.
- Add counters for host-to-device bytes, device-to-host bytes, and kernel count.

Files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`

Verification:

- `cargo test -p ta_app_core gpu_backtest_kernel`
- `cargo test -p ta_app_core double_ma_validate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

### CUDA-04: Composite Hybrid Validation Product Gate

Risk: Medium.

Problem:
Composite profile optimization routes are clearer than some validation product-gating around hybrid GPU routes.

Implementation:

- Add product-gated validation support for Composite `GpuHybridKernel` where CPU auxiliary tails remain expected.
- Make route metadata explicit:
  - kernel work
  - CPU auxiliary work
  - unsupported modules
  - fallback reason
- Add tests for ADX on/off route expectations.

Files:

- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationService.ts`

Verification:

- `cargo test -p ta_app_core composite_validate`
- `cargo test -p ta_app_core double_ma_validate`
- `cargo test -p ta_desktop_demo_app --lib commands_validation`

### CUDA-05: PTX And Native Binary Release Manifest

Risk: Low/medium.

Problem:
Release docs and manifest scripts must agree on which PTX/native files are packaged and hashed.

Implementation:

- Generate a release manifest section for:
  - PTX files
  - CUDA feature flags
  - bundled native libraries
  - model/native runtime bundles if enabled
  - file hashes
- Update docs when options PTX requirements change.
- Add a doc-sync check because manifest generation already includes `options_expression.prebuilt.ptx`, and release docs must not claim otherwise.
- Fail public release gate if expected GPU assets are missing in a GPU build.

Files:

- `scripts/generate_release_manifest.ps1`
- `scripts/run_public_release_gate.ps1`
- `docs/RELEASE.md`
- `docs/PUBLIC_RELEASE.md`
- `crates/app_core/src/kernels/*`

Verification:

- `powershell -File scripts/run_public_release_gate.ps1 -Variant gpu-kernel`
- `powershell -File scripts/generate_release_manifest.ps1 -Variant gpu-kernel`

### CUDA-06: GPU Performance Evidence Dashboard

Risk: Low.

Problem:
Benchmark output should be easy to compare across CPU candidate, GPU candidate, and validation workloads.

Implementation:

- Normalize benchmark JSON summaries:
  - dataset
  - rows
  - candidates
  - route
  - backend
  - GPU device
  - transfer time
  - kernel time
  - reduction time
  - CPU tail time
  - total time
- Add a Markdown summary generator for candidate evidence.
- Keep benchmark executions serialized.

Files:

- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `crates/app_core/benches/*`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `crates/app_core/benches/double_ma_opt.rs`

Verification:

- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`

### CUDA-07: Tensor Core And RT Core Research Lanes

Risk: Research/high.

Problem:
Tensor cores and RT cores are not currently production execution paths. They may still be worth prove-or-kill research.

Implementation:

- Keep tensor core work limited to dense linear algebra, ML inference, or matrix-heavy screening candidates where mixed precision is acceptable.
- Keep RT core/OptiX ideas in research docs unless a concrete mapping beats conventional CUDA.
- Require:
  - mathematical equivalence or acceptable approximation error
  - CPU oracle
  - CUDA baseline comparison
  - benchmark showing real benefit
  - release exclusion until proven

Files:

- `docs/gpu/*`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/*`

Verification:

- CPU oracle tests for any candidate algorithm.
- GPU parity tests.
- Serialized benchmark comparison against conventional CUDA.
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1` for options-expression GPU promotion candidates.

## Acceptance Criteria For This Guide

- CPU-only builds compile without a CUDA toolkit.
- GPU route claims include requested backend, observed route, device, transfer time, kernel time, reduction time, and CPU-tail time where available.
- GPU validation strictness is opt-in and uses explicit residency policy fields.
- Options-expression GPU work continues to pass CPU oracle parity before promotion.
- Tensor-core and RT-core research remains excluded from release claims until CPU oracle, CUDA baseline, and benchmark evidence exist.

## Deferred Higher-Risk Work

- Cross-stage VRAM persistence from optimization into validation.
- GPU acceleration for every validation statistic.
- Tensor core or RT core production use without strong evidence.
