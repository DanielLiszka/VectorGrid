# Review Packet: `crates/app_core/src/carm.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/carm.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful strategy-local CARM engine with coherent CPU/GPU signal assembly, but not yet accept-as-is because otto_threshold has an inconsistent cross-path contract, the public field is overloaded across two different semantics, and preferred-GPU fallback currently becomes silent`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Strategy-local`, `CPU/GPU-parity-sensitive`, `Semantics-sensitive`, `Metrics-sensitive`

## What This File Owns

`carm.rs` owns the CARM strategy support layer in `ta_app_core`, including its public parameter surface, period resolution, dynamic-indicator orchestration, CPU-side signal synthesis, and desired-position state machine.

This file owns:

- the public CARM parameter surface:
  - `CarmStrategyParams`: `crates/app_core/src/carm.rs:19-40`
  - re-exported from app-core root:
    - `crates/app_core/src/lib.rs:121`
- validation and period resolution:
  - `validate_carm_params`: `crates/app_core/src/carm.rs:173-203`
  - `resolve_carm_periods`: `205-227`
- output selection and indicator dispatch helpers:
  - `resolve_output_id`: `284-320`
  - `compute_indicator_f64`: `322-382`
  - CUDA host-f32 helper:
    - `384-436`
- mixed momentum signal builders:
  - `compute_carm_mixed_signals*`: `438-667`
- ROCR signal builders:
  - `compute_carm_rocr_signal*`: `669-760`
- OTTO signal builders:
  - `compute_carm_otto_signals*`: `762-949`
- combined CARM signal assembly:
  - `compute_carm_signals*`: `952-1061`
- warmup/start index derivation:
  - `compute_carm_start_t`: `1063-1074`
- CPU desired-position state machine:
  - `build_carm_desired_positions`: `1084-1188`
- local tests:
  - `1221-1442`

This file does not own:

- composite optimization and validation orchestration
  - that lives in `composite_cpu.rs` and `composite_validate.rs`
- standalone GPU metrics-kernel execution
  - that lives in `carm_gpu_kernel.rs` and `kernels/double_crossover.cu`

## Classification

- Public surface: yes
  - `CarmStrategyParams` is re-exported from `lib.rs`
- Hot path: yes
  - this file builds the indicator series and desired positions used by optimization and validation
- Runtime-critical: high
  - mistakes here change trade direction, entry timing, and candidate ranking
- CUDA/runtime-critical: medium-high
  - this file decides whether preferred indicator evaluation goes through CUDA or CPU fallback
- Strategy-local: yes
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- composite CPU paths call into this module for CARM signal generation and desired-position construction:
  - `crates/app_core/src/composite_cpu.rs:31-36`
  - `crates/app_core/src/composite_cpu.rs:1056-1066`
  - `crates/app_core/src/composite_cpu.rs:9002-9018`
  - `crates/app_core/src/composite_cpu.rs:9056-9073`
- composite validation reuses the same parameter validation, signal generation, and desired-position helpers:
  - `crates/app_core/src/composite_validate.rs:18-23`
  - `crates/app_core/src/composite_validate.rs:7024-7045`
  - `crates/app_core/src/composite_validate.rs:10840-10882`
- the standalone GPU metrics path consumes the same public parameter contract:
  - `crates/app_core/src/carm_gpu_kernel.rs:3`
  - `crates/app_core/src/carm_gpu_kernel.rs:358-364`
  - `crates/app_core/src/carm_gpu_kernel.rs:493`
- all indicator computation is delegated through the dynamic indicator layer:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
  - `crates/app_core/src/indicator_catalog.rs`

### Downstream verification impact if changed

- any change to `otto_threshold` validation affects:
  - composite CPU optimization entry
  - composite validation entry
  - GPU kernel parameter admission
- any change to OTTO regime semantics affects:
  - CPU desired-position generation in this file
  - GPU metrics parity in `carm_gpu_kernel.rs` and `double_crossover.cu`
- any change to preferred-GPU fallback behavior affects:
  - optimization throughput
  - route explainability during CUDA-capable runs

### Test anchors

- direct local tests:
  - `crates/app_core/src/carm.rs:1221-1442`
- dependent smoke tests surfaced by the filtered CARM run:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`

## Standards And Research Log

This packet was written after targeted external review focused on public configuration predictability, serde-defaulted contract clarity, OTTO indicator parameter semantics, and CUDA-route expectations for performance-sensitive strategy wrappers.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines a public config struct and several high-impact strategy helpers
   - Standard informed:
     - public APIs should be predictable and should not surprise callers with hidden parameter reinterpretation

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - `CarmStrategyParams` is a serde-driven request/config surface built from per-field defaults
   - Standard informed:
     - defaulted fields should still correspond to a clear validated contract rather than relying on downstream coercion

3. VectorTA OTTO indicator documentation
   - Source: <https://vectoralpha.dev/projects/ta/indicators/otto/>
   - Why it was consulted:
     - this file maps `otto_threshold` into OTTO indicator inputs, so the underlying indicator parameter semantics matter
   - Standard informed:
     - OTTO's documented default `ott_percent` is `0.6`, which helps explain why this file currently forces `.max(0.6)` during indicator construction

4. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file opportunistically routes indicator computation through CUDA and then falls back to CPU
   - Standard informed:
     - GPU-route decisions are performance-sensitive and should remain explicit enough to diagnose unexpected fallback behavior

## Findings

## Correctness And Safety

### 1. `otto_threshold` is validated too loosely and creates a real CPU/GPU contract bug when negative values are admitted

- Evidence:
  - validation only checks finiteness:
    - `crates/app_core/src/carm.rs:199-201`
  - OTTO indicator construction forces the same field positive with `.abs().max(0.6)`:
    - `crates/app_core/src/carm.rs:803`
    - `crates/app_core/src/carm.rs:899`
  - the request surface already allows values below `0.6`:
    - `web/src/services/validationRequestBuilder.test.ts:1134-1164`
  - CPU desired-position gating then uses the raw signed field:
    - `crates/app_core/src/carm.rs:1132-1133`
  - GPU metrics path normalizes the threshold back to absolute value:
    - `crates/app_core/src/carm_gpu_kernel.rs:493`
    - `crates/app_core/src/kernels/double_crossover.cu:2952`
    - `crates/app_core/src/kernels/double_crossover.cu:2986-2987`
  - the underlying CUDA dispatch layer also carries a documented default of `0.6` for `ott_percent`:
    - `vendor/vector-ta-0.2.4/src/indicators/dispatch/cuda.rs:10096-10103`
    - `vendor/vector-ta-0.2.4/src/indicators/dispatch/cuda.rs:19712-19717`
- Why it matters:
  - a negative `otto_threshold` is accepted by validation, but the CPU desired-position logic interprets it as allowing overlapping `otto_up` and `otto_down` conditions
  - the GPU metrics path already normalizes the same field, so the same config can produce different CPU desired positions and GPU-evaluated trade behavior
  - this is an objective cross-path correctness defect, not a stylistic preference
- Smallest justified fix:
  - require `otto_threshold >= 0.0` and finite in `validate_carm_params`
  - add a direct test that negative values are rejected
  - keep the stronger field split deferred until after the immediate correctness hole is closed
- Review judgment:
  - safe-now change

## API And Naming

### 1. `otto_threshold` currently carries two different meanings, and the public surface does not make that coupling legible

- Evidence:
  - public field name suggests a regime threshold:
    - `crates/app_core/src/carm.rs:38-39`
  - the default config value is `0.0`:
    - `crates/app_core/src/carm.rs:152-168`
  - the same field is repurposed as OTTO indicator `ott_percent`, but forced to a minimum of `0.6`:
    - `crates/app_core/src/carm.rs:803`
    - `crates/app_core/src/carm.rs:899`
  - frontend-side request construction already permits `otto_threshold: 0.1`:
    - `web/src/services/validationRequestBuilder.test.ts:1134-1164`
  - the VectorTA OTTO docs list `ott_percent = 0.6` as the documented indicator default:
    - <https://vectoralpha.dev/projects/ta/indicators/otto/>
- Why it matters:
  - the public config surface reads as though one threshold controls one concept
  - in reality it controls:
    - OTTO indicator construction percent
    - OTTO regime-entry spread threshold
  - those two roles already have different effective defaults, so callers cannot infer behavior cleanly from the field name and stored value alone
- Smallest justified fix:
  - document the current dual-role contract explicitly in the struct and packeted downstream docs
  - reject negative values now
- Larger deferred fix:
  - split this into separate fields for OTTO indicator percent and OTTO regime-entry threshold, but only after compatibility review across request builders, saved configs, and GPU paths
- Review judgment:
  - objective contract debt; safe to document now, semantics-sensitive to redesign

## Error Handling

### 1. Preferred-GPU indicator attempts fail silently and degrade the rest of the CARM signal build to CPU-only execution

- Evidence:
  - preferred-GPU execution is attempted in `compute_indicator_f64` when the descriptor supports CUDA:
    - `crates/app_core/src/carm.rs:342-359`
  - on CUDA failure, the error is discarded and only `cuda_failed = true` is recorded:
    - `crates/app_core/src/carm.rs:360-367`
  - the same behavior exists in the host-f32 helper:
    - `crates/app_core/src/carm.rs:401-420`
  - subsequent indicator computations in the same call share the same attempt state:
    - `crates/app_core/src/carm.rs:447-456`
    - `crates/app_core/src/carm.rs:577-586`
    - `crates/app_core/src/carm.rs:772-781`
    - `crates/app_core/src/carm.rs:1014-1042`
- Why it matters:
  - one failing CUDA attempt can silently turn a preferred-GPU path into CPU-only execution for the remainder of the CARM signal build
  - that is operationally useful as a fast-fail mechanism, but currently invisible to callers and performance review
  - hidden fallback makes it harder to explain route regressions or benchmark drift
- Smallest justified fix:
  - preserve the fast-fail policy, but record the first CUDA error string or route note for diagnostics
  - add one targeted CUDA-path test that asserts the fallback is observable when it occurs
- Review judgment:
  - safe-now diagnostic improvement

## Module Boundaries And Technical Debt

### 1. The file is large, but the right boundary work is contract hardening, not broad decomposition

- Evidence:
  - this module keeps the full CARM strategy flow together:
    - params
    - indicator dispatch
    - signal assembly
    - start index computation
    - desired-position generation
- Why it matters:
  - this is strategy-local code, so keeping the whole decision chain visible in one place is useful
  - the real debt is the overloaded OTTO field contract and hidden fallback behavior, not the fact that the module is strategy-local and sizable
- Recommended boundary stance:
  - avoid generic abstraction churn here
  - focus on tightening config semantics and cross-path parity instead

### 2. `resolve_output_id` currently fails open, which makes indicator-catalog drift a runtime behavior risk instead of a hard contract break

- Evidence:
  - exact match falls back to substring heuristics and then to the first declared output:
    - `crates/app_core/src/carm.rs:284-320`
  - that helper is used for every signal surface:
    - `crates/app_core/src/carm.rs:467-469`
    - `crates/app_core/src/carm.rs:698-702`
    - `crates/app_core/src/carm.rs:794-795`
    - `crates/app_core/src/carm.rs:890-891`
- Why it matters:
  - CARM depends on specific semantic outputs such as `value`, `hott`, and `lott`
  - if the indicator catalog drifts, a first-output fallback can quietly bind the wrong series instead of failing where the contract broke
- Smallest justified fix:
  - make CARM output selection fail closed when no preferred output is found
  - add a focused unit test that protects the intended output-id contract
- Review judgment:
  - safe-now hardening

## Docs And Examples

### 1. The public parameter contract and behavior-sensitive decision rules are effectively undocumented

- Missing documentation topics:
  - that `otto_threshold` is currently used both for OTTO indicator construction and regime gating
  - that OTTO indicator construction clamps the effective percent to at least `0.6`
  - that preferred-GPU execution may fall back to CPU for the remainder of the signal build after a failure
  - that short entries remain gated by `StrategyConfig.long_only`
- Smallest justified fix:
  - add struct-field and helper-level doc comments for the public config contract
  - keep examples minimal and focused on invariants rather than large strategy walkthroughs

## Tests And Missing Verification

### 1. Existing coverage is useful but does not directly pin the main contract defect

- Verified in this review pass:
  - `cargo test -p ta_app_core carm -- --nocapture`
    - passed `8` relevant tests, including local CARM tests and dependent composite CPU smoke coverage
- Existing strengths:
  - desired-position behavior coverage for long-only, flip behavior, and trade-on-next-bar:
    - `crates/app_core/src/carm.rs:1233-1389`
  - CUDA smoke entry point exists for signal generation:
    - `crates/app_core/src/carm.rs:1402-1442`

### 2. Important CARM-specific edge cases remain untested

- Missing tests found in this review pass:
  - negative `otto_threshold` rejection
  - explicit CPU/GPU parity protection for OTTO threshold semantics
  - strict output-id resolution behavior for CARM's expected indicator outputs
  - observable route behavior when preferred-GPU indicator execution falls back to CPU
  - documentation-level protection for the current `ott_percent` minimum-clamp contract if that coupling remains

### 3. Exact local verification required if changed

- `cargo test -p ta_app_core carm -- --nocapture`
- add a direct unit test for negative `otto_threshold`, then run that test specifically
- if `otto_threshold` semantics or desired-position rules change:
  - `cargo test -p ta_app_core composite_cpu -- --nocapture`
  - `cargo test -p ta_app_core composite_validate -- --nocapture`
- if CUDA route or fallback behavior changes:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel compute_carm_signals_gpu_smoke_runs_with_all_indicator_surfaces -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. The current fast-fail CUDA attempt policy is reasonable, but it is benchmark-sensitive and should not be reworked casually

- Evidence:
  - once `cuda_failed` is set, later indicator computations stop retrying CUDA for that signal build:
    - `crates/app_core/src/carm.rs:347-367`
    - `crates/app_core/src/carm.rs:407-420`
- Why it matters:
  - this likely avoids repeated failing device setup or kernel dispatch overhead
  - but it also means performance can shift sharply from mixed GPU to all-CPU execution after one failure
- Review stance:
  - keep the fast-fail behavior unless profiling shows it is materially wrong
  - improve observability first
- Review judgment:
  - benchmark-sensitive; do not touch the retry policy without measurement

## Recommended Action

### Accept as-is

- no

### Document only

- document the current dual-role meaning of `otto_threshold` if a field split is deferred
- document the preferred-GPU fast-fail fallback policy

### Small surgical refactor

- reject negative `otto_threshold` values in `validate_carm_params`
- add a direct unit test for that validation rule
- preserve the current fast-fail CUDA behavior, but surface a fallback diagnostic instead of dropping the error silently

### Larger refactor but defer

- split `otto_threshold` into:
  - an OTTO indicator percent field
  - an OTTO regime-entry threshold field

### Benchmark-sensitive; do not touch yet

- do not change CUDA retry/fallback policy, dispatch ordering, or host-vs-device output strategy without profiling

## Safe-Now Vs Defer-Later Summary

### Safe now

- tighten `otto_threshold` validation to reject negative values
- add tests that lock CPU/GPU threshold semantics back to one admitted contract
- add route diagnostics when preferred-GPU evaluation falls back to CPU
- add minimal doc comments clarifying the current contract

### Defer until compatibility review

- any redesign that splits the overloaded `otto_threshold` field into separate public knobs
- any change that alters when CUDA is retried within a signal-build call
