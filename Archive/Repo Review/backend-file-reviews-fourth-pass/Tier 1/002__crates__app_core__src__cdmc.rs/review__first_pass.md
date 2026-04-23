# Review Packet: `crates/app_core/src/cdmc.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cdmc.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Broad CDMC strategy-local engine with useful CPU/GPU helpers, but not yet accept-as-is because the strategy layer appears to reinterpret correlation_cycle state semantics, phase_threshold admits values outside its effective domain, and preferred-GPU fallback remains silent`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Strategy-local`, `CPU/GPU-parity-sensitive`, `Semantics-sensitive`, `Metrics-sensitive`

## What This File Owns

`cdmc.rs` owns the CDMC strategy support layer in `ta_app_core`, including its public parameter surface, indicator-id resolution, CPU/GPU-backed signal builders, device-period sweep helpers used by the GPU optimization route, and the CPU desired-position state machine.

This file owns:

- the public CDMC parameter surface:
  - `CdmcStrategyParams`: `crates/app_core/src/cdmc.rs:30-42`
  - re-exported from app-core root:
    - `crates/app_core/src/lib.rs:122`
- validation and resolved-period policy:
  - `validate_cdmc_params`: `192-215`
  - `resolve_cdmc_periods`: `217-226`
- cached indicator id/parameter-key resolution:
  - `cdmc_indicator_ids`: `263-285`
  - `resolve_output_id`: `287-322`
  - `resolve_param_key`: `324-359`
- CUDA device-matrix and sweep helpers:
  - `CdmcDeviceMatrix`: `77-113`
  - `CdmcPeriodSweepDevice`: `116-141`
  - DPO / correlation-cycle / decycler sweeps:
    - `441-521`
  - device MAMA/FAMA builder:
    - `523-628`
- CPU/GPU-backed indicator builders:
  - `compute_indicator_f64`: `630-657`
  - `compute_mama_pair_f64`: `659-706`
  - `compute_decycler_f64`: `708-758`
- combined signal assembly:
  - `compute_cdmc_signals`: `761-850`
- warmup/start index derivation:
  - `compute_cdmc_start_t`: `852-863`
- CPU desired-position state machine:
  - `build_cdmc_desired_positions`: `865-984`
- local tests:
  - `1008-1078`

This file does not own:

- CDMC GPU metrics-kernel execution
  - that lives in `cdmc_gpu_kernel.rs` and `kernels/double_crossover.cu`
- composite optimization and validation orchestration
  - that lives in `composite_cpu.rs` and `composite_validate.rs`

## Classification

- Public surface: yes
  - `CdmcStrategyParams` is re-exported from `lib.rs`
- Hot path: yes
  - signal generation and desired-position logic feed both optimization and validation
- Runtime-critical: high
  - mistakes here alter regime classification, entry logic, and candidate ranking
- CUDA/runtime-critical: medium-high
  - the file owns CPU/GPU signal route selection and device sweep preparation
- Strategy-local: yes
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- composite CPU optimization and eval routes reuse this module:
  - `crates/app_core/src/composite_cpu.rs:45-53`
  - `crates/app_core/src/composite_cpu.rs:1118-1128`
  - `crates/app_core/src/composite_cpu.rs:9977-10075`
  - `crates/app_core/src/composite_cpu.rs:10417-10433`
- composite validation reuses CDMC signal generation and desired-position construction:
  - `crates/app_core/src/composite_validate.rs:30-32`
  - `crates/app_core/src/composite_validate.rs:8852-9159`
  - `crates/app_core/src/composite_validate.rs:10960-11002`
- CDMC GPU metrics-kernel routes consume the same signal semantics and `phase_threshold` policy:
  - `crates/app_core/src/cdmc_gpu_kernel.rs:525-526`
  - `crates/app_core/src/kernels/double_crossover.cu:2258-2291`
- underlying indicator semantics come from VectorTA indicators:
  - correlation cycle
  - MAMA/FAMA
  - decycler

### Downstream verification impact if changed

- any change to `phase_threshold` validation affects:
  - optimization request acceptance
  - validation request acceptance
  - GPU kernel phase gating
- any change to `cycle_state` interpretation affects:
  - CPU desired-position behavior
  - GPU-kernel parity in `cdmc_gpu_kernel.rs`
  - optimization and validation outcome ranking
- any change to output-id resolution affects:
  - both CPU and GPU sweep/device helper correctness

### Test anchors

- direct local tests:
  - `crates/app_core/src/cdmc.rs:1008-1078`
- higher-level parity and route tests:
  - `crates/app_core/src/composite_cpu.rs:18037-18345`
  - `crates/app_core/src/composite_validate.rs:23586-25324`

## Standards And Research Log

This packet was written after targeted external review focused on public config predictability, serde-defaulted contract clarity, correlation-cycle state semantics, and CUDA best-practice expectations for device-sweep helpers.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines a public strategy config and several widely reused strategy helpers
   - Standard informed:
     - public APIs should be predictable and should not silently reinterpret caller-supplied configuration

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - `CdmcStrategyParams` is a serde-defaulted request/config surface
   - Standard informed:
     - defaulted fields should still correspond to a clear validated value domain

3. VectorTA Correlation Cycle documentation
   - Source: <https://vectoralpha.dev/projects/ta/indicators/correlation_cycle/>
   - Why it was consulted:
     - this file relies on both `angle` and `state` outputs from the indicator and then builds regime logic from them
   - Standard informed:
     - the indicator documents `state` as a trend-versus-cycle classifier derived from angle stability, not as a direct bullish-vs-bearish regime selector

4. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file creates device-resident sweep helpers and a device MAMA/FAMA pair for the GPU optimization route
   - Standard informed:
     - device-path reuse is valuable, but hidden host recomputation and fallback behavior should be treated carefully

## Findings

## Correctness And Safety

### 1. CDMC appears to interpret `correlation_cycle.state` as a trend-vs-reversion selector, but the underlying indicator defines it as a trend-vs-cycling classifier

- Evidence:
  - the Correlation Cycle docs describe the market state flag as distinguishing trending versus cycling behavior:
    - <https://vectoralpha.dev/projects/ta/indicators/correlation_cycle/>
  - the underlying implementation emits `state = 0.0` when angle changes exceed the threshold and `state = +/-1.0` only when angle changes remain stable enough to pass the threshold:
    - `vendor/vector-ta-0.2.4/src/indicators/correlation_cycle.rs:690-698`
  - CDMC then treats non-negative state as `trend_regime` and negative state as `reversion_regime`:
    - `crates/app_core/src/cdmc.rs:891-893`
  - the CUDA kernel mirrors the same interpretation:
    - `crates/app_core/src/kernels/double_crossover.cu:2276-2291`
- Why it matters:
  - the underlying indicator's `state` looks like:
    - `0.0` for trending / unstable-angle behavior
    - `+/-1.0` for stable cycling behavior, with sign reflecting angle sign
  - CDMC instead routes `state >= 0.0` into its "trend" branch and only `state < 0.0` into its "reversion" branch
  - that means the strategy may be making regime decisions on a semantic mapping that does not match the upstream indicator contract
- Smallest justified fix:
  - do not change this blindly
  - first add targeted tests that lock the intended product interpretation of `cycle_state`
  - then review CPU and GPU parity together before changing the regime mapping
- Review judgment:
  - semantics-sensitive and potentially high impact

## API And Naming

### 1. `phase_threshold` is validated too loosely for its actual contract because both CPU and GPU paths silently normalize it with `abs()`

- Evidence:
  - validation only requires finiteness:
    - `crates/app_core/src/cdmc.rs:211-213`
  - CPU desired-position logic immediately normalizes with `abs()`:
    - `crates/app_core/src/cdmc.rs:883`
  - GPU kernel wrapper and CUDA kernel do the same:
    - `crates/app_core/src/cdmc_gpu_kernel.rs:525-526`
    - `crates/app_core/src/kernels/double_crossover.cu:2258`
- Why it matters:
  - negative values are accepted at the public config surface, but they do not have independent semantics
  - callers cannot tell from validation alone that the effective domain is non-negative
- Smallest justified fix:
  - require `phase_threshold >= 0.0` and finite in `validate_cdmc_params`
  - add a direct validation test for negative values
- Review judgment:
  - safe-now contract hardening

## Error Handling

### 1. Preferred-GPU indicator attempts still fail silently and fall back to CPU without preserving the reason

- Evidence:
  - dynamic indicator helper prefers GPU and returns CUDA output when available:
    - `crates/app_core/src/cdmc.rs:631-647`
  - if that CUDA call fails, the function simply continues into CPU computation with no diagnostic:
    - `648-657`
  - the same silent fallback pattern exists in the MAMA and decycler helpers:
    - `677-697`
    - `722-744`
- Why it matters:
  - unexpected CPU fallback becomes hard to explain during performance review
  - CUDA-path regressions can disappear into normal successful execution
- Smallest justified fix:
  - preserve the current fallback policy, but surface a route note or first failure reason for diagnostics
- Review judgment:
  - safe-now diagnostic improvement

## Module Boundaries And Technical Debt

### 1. `resolve_output_id` fails open to the first declared output, and the resolved ids are cached for the lifetime of the process

- Evidence:
  - `resolve_output_id` falls back to the first output when no preferred match is found:
    - `crates/app_core/src/cdmc.rs:287-322`
  - those resolved ids are cached inside `OnceLock`:
    - `263-285`
- Why it matters:
  - if indicator metadata drifts, CDMC can silently bind the wrong series rather than failing where the contract broke
  - because the result is cached, that silent mismatch persists for the rest of the process lifetime
- Smallest justified fix:
  - make output selection fail closed when no preferred output is found
  - add a focused unit test for expected output-id resolution
- Review judgment:
  - safe-now hardening

### 2. Some validation plumbing still carries CUDA-prepared inputs through paths that are explicitly forced to CPU-only execution

- Evidence:
  - validation-series construction passes `prepared_cuda_inputs` into `compute_cdmc_signals` while forcing `Backend::CpuOnly`:
    - `crates/app_core/src/composite_validate.rs:9131-9139`
    - `crates/app_core/src/composite_validate.rs:10981-10990`
  - the CDMC helpers only use those prepared CUDA inputs when `backend_prefers_gpu(backend)` is true:
    - `crates/app_core/src/cdmc.rs:631-647`
    - `crates/app_core/src/cdmc.rs:677-697`
    - `crates/app_core/src/cdmc.rs:722-744`
- Why it matters:
  - this currently behaves as dead plumbing rather than active CPU/GPU cooperation
  - it obscures the actual contract boundary between CPU-only validation series generation and GPU-assisted validation routes
- Smallest justified fix:
  - either stop threading unused CUDA-prepared inputs through CPU-only CDMC validation calls
  - or document clearly that the parameter is intentionally threaded for future route symmetry rather than current execution
- Review judgment:
  - low-risk boundary cleanup; safe-now if done narrowly

## Docs And Examples

### 1. The public config and the regime interpretation logic are effectively undocumented

- Missing documentation topics:
  - what `phase_threshold` means and that its effective domain is non-negative
  - how `cycle_state` is intended to be interpreted by CDMC
  - when preferred-GPU execution may fall back to CPU
  - that correlation-cycle threshold is currently fixed at `9.0` in the strategy wrapper:
    - `crates/app_core/src/cdmc.rs:482-484`
    - `crates/app_core/src/cdmc.rs:778-784`
- Smallest justified fix:
  - add concise field and helper docs describing the intended regime mapping and fallback policy

## Tests And Missing Verification

### 1. Existing default coverage is useful but does not pin the highest-risk semantics

- Verified in this review pass:
  - `cargo test -p ta_app_core cdmc -- --nocapture`
    - passed `4` relevant tests, including local CDMC tests and dependent composite CPU smoke coverage
- Existing strengths:
  - validation coverage for invalid MAMA fast/slow limits:
    - `crates/app_core/src/cdmc.rs:1008-1017`
  - basic desired-position transition coverage:
    - `1020-1038`
  - CUDA smoke entry point exists for signal generation:
    - `1043-1078`

### 2. Important CDMC-specific edges remain untested

- Missing tests found in this review pass:
  - negative `phase_threshold` rejection
  - explicit intended mapping from `correlation_cycle.state` into CDMC regime logic
  - strict output-id resolution behavior
  - observable preferred-GPU fallback diagnostics

### 3. Exact local verification required if changed

- `cargo test -p ta_app_core cdmc -- --nocapture`
- add a direct unit test for negative `phase_threshold`, then run it specifically
- if changing regime semantics or output-id resolution:
  - `cargo test -p ta_app_core composite_cpu::tests::cdmc_optimization_smoke_runs -- --nocapture`
  - `cargo test -p ta_app_core --features cuda compute_cdmc_signals_gpu_smoke_runs_with_dynamic_mama_and_decycler -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_gpu_kernel_major_parity_vs_cpu -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_optimization_gpu_kernel_sanity_matches_cpu_small_grid -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_validation_gpu_kernel_matches_cpu_metrics -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_validation_gpu_kernel_start_t_matches_cpu_series -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. `compute_cdmc_mama_pair_cuda_device` duplicates MAMA work on the host solely to recover `first_output_valid`

- Evidence:
  - the function computes device-resident MAMA/FAMA outputs first:
    - `crates/app_core/src/cdmc.rs:545-573`
  - it then allocates host arrays and reruns `mama_batch_into_host_f32`:
    - `575-591`
  - the host pass is only used to derive `first_output_valid`:
    - `592-598`
- Why it matters:
  - this partially defeats the point of a device-native helper in a performance-sensitive GPU optimization route
  - it adds host allocation and extra compute to the path
- Review stance:
  - this is a real optimization surface, but not the first change to make
  - any attempt to eliminate the host pass should be benchmarked and validated against current first-valid semantics
- Review judgment:
  - benchmark-sensitive; do not touch casually

## Recommended Action

### Accept as-is

- no

### Document only

- document the intended interpretation of `cycle_state`
- document that `phase_threshold` is effectively non-negative
- document preferred-GPU fallback behavior

### Small surgical refactor

- reject negative `phase_threshold` values in `validate_cdmc_params`
- add diagnostics for preferred-GPU fallbacks
- make `resolve_output_id` fail closed when no preferred output is found

### Larger refactor but defer

- re-evaluate and, if necessary, correct the regime mapping from `correlation_cycle.state` across both CPU and GPU implementations

### Benchmark-sensitive; do not touch yet

- do not change the device MAMA first-valid path without measurement

## Safe-Now Vs Defer-Later Summary

### Safe now

- tighten `phase_threshold` validation
- add missing tests for that contract
- add fallback diagnostics
- fail closed on output-id mismatch

### Defer until semantics review

- any change to how CDMC maps `correlation_cycle.state` into trend-vs-reversion behavior
- any change to GPU/host work split in the device MAMA helper
