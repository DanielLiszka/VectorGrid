# Review Packet: `crates/app_core/src/asr.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/asr.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Substantial strategy-local ASR engine with broad CPU/GPU coverage, but not yet accept-as-is because z-score parameter validation is incomplete, release-confirm semantics are inconsistent across execution paths, and preferred-GPU fallback can become silent`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Strategy-local`, `CPU/GPU-parity-sensitive`, `Semantics-sensitive`, `Metrics-sensitive`

## What This File Owns

`asr.rs` owns the Adaptive Squeeze Regime strategy support layer in `ta_app_core`, including parameter schema, CPU/GPU indicator assembly, desired-position generation, and final metrics/curve simulation from desired positions.

This file owns:

- the public ASR parameter surface:
  - `AsrStrategyParams`: `crates/app_core/src/asr.rs:44-98`
  - re-exported from app-core root:
    - `crates/app_core/src/lib.rs:120`
- resolved period mapping from fast/slow MA lengths:
  - `resolve_asr_periods`: `462-489`
- CPU/GPU indicator series conversion and route helpers:
  - `compute_indicator_f64`: `529-575`
  - CUDA direct/sweep helpers: `577-913`
- fast and slow ASR signal construction:
  - `compute_asr_fast_signals*`: `920-967`, `1145-1201`, `1598-1669`
  - `compute_asr_slow_signals*`: `977-1029`, `1204-1267`, `1671-1751`
- Kaufman-stop construction:
  - CPU and CUDA variants: `1035-1138`, `1270-1408`, `1753-1795`
- warmup/start index derivation:
  - `compute_asr_start_t`: `1797-1817`
- desired-position state machine:
  - `build_asr_desired_positions`: `1819-1996`
- execution overlay and intrabar fill simulation:
  - `ExecutionOverlay*`, `fill_price_from_intrabar`, `simulate_from_desired`: `1998-2725`
- public metrics/curve evaluation entry points:
  - `eval_asr_metrics_from_desired`: `2727-2745`
  - `eval_asr_metrics_window_from_desired`: `2747-2766`
  - `eval_asr_curves_from_desired`: `2768-2804`

This file does not own:

- composite optimization/validation orchestration
  - that lives in `composite_cpu.rs` and `composite_validate.rs`
- GPU kernel execution of finished ASR signal matrices
  - that lives in `asr_gpu_kernel.rs`

## Classification

- Public surface: yes
  - `AsrStrategyParams` is re-exported from `lib.rs`
- Hot path: yes
  - signal generation and desired-position building feed optimization/validation workflows
- Runtime-critical: high
  - mistakes here change candidate ranking and final metrics
- CUDA/runtime-critical: high
  - the file owns CPU/GPU path selection and direct-sweep helpers
- Strategy-local: yes
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- composite CPU optimization/drilldown paths call into this module for full ASR evaluation:
  - `crates/app_core/src/composite_cpu.rs:663-725`
  - many additional optimization/validation entry points reuse the same helpers
- composite validation reuses ASR metrics and signal builders:
  - `crates/app_core/src/composite_validate.rs:815`
  - `crates/app_core/src/composite_validate.rs:10237-10310`
- ASR GPU kernel support consumes `AsrStrategyParams` and ASR device matrices:
  - `crates/app_core/src/asr_gpu_kernel.rs:3`
  - `crates/app_core/src/asr_gpu_kernel.rs:602`
  - `crates/app_core/src/asr_gpu_kernel.rs:846`
- dynamic strategy execution also reuses the final metrics path:
  - `crates/app_core/src/strategy_dynamic.rs:1628`

### Downstream verification impact if changed

- any change to parameter validation affects:
  - composite CPU optimization
  - composite validation
  - dynamic strategy evaluation
- any change to release-confirm semantics affects:
  - CPU desired-position generation
  - GPU parity expectations
  - candidate ranking and validation outputs
- any change to GPU fallback diagnostics affects:
  - ASR route reporting in composite CPU/validation workflows

### Test anchors

- direct local test:
  - `crates/app_core/src/asr.rs:2861-2902`
- dependent smoke tests:
  - `composite_cpu` and `composite_validate` ASR smoke tests surfaced by `cargo test -p ta_app_core asr`

## Standards And Research Log

This packet was written after targeted external review focused on public config-shape guidance, serde default behavior for strategy params, and CUDA best-practice expectations for device/host routing and fallback clarity.

### Sources consulted

1. Rust API Guidelines documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file defines a public strategy-parameter surface and several broadly reused strategy helpers
   - Standard informed:
     - public configuration structs and behavior-sensitive functions should have explicit contracts and invariants

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - `AsrStrategyParams` is driven by serde defaults and external request payloads
   - Standard informed:
     - defaulted strategy parameters should be intentionally validated, not just defaulted

3. `vector-ta` crate docs
   - Source: <https://docs.rs/vector-ta/latest/vector_ta/>
   - Why it was consulted:
     - this file is a thin orchestration layer over vector-ta indicators and CUDA dispatch
   - Standard informed:
     - strategy-local wrappers should preserve clarity about which constraints belong to local policy vs underlying indicator engines

4. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file decides when to use host-vs-device indicator paths and direct-vs-fallback CUDA routes
   - Standard informed:
     - device-route behavior should stay explicit and diagnosable, especially when fallbacks occur

## Findings

## Correctness And Safety

### 1. `validate_asr_params` leaves `z_nbdev` effectively unconstrained even though it directly scales the z-score denominator

- Evidence:
  - validation checks many ASR fields but does not validate `z_nbdev` at all:
    - `crates/app_core/src/asr.rs:377-429`
  - the value is passed straight into z-score computation:
    - `1689-1705`
  - the underlying indicator multiplies sigma by `nbdev`:
    - `vendor/vector-ta-0.2.4/src/indicators/zscore.rs:2669-2672`
- Why it matters:
  - `z_nbdev <= 0` creates degenerate or sign-inverting z-score behavior rather than a sensible strategy parameter
  - this is especially inconsistent with how other strategy families sanitize similar z-score parameters before use
- Smallest justified fix:
  - require `z_nbdev` to be finite and `> 0.0` in `validate_asr_params`
  - add a direct validation test for zero and negative values

### 2. CPU ASR release-confirm logic allows trend entry before any squeeze release has actually occurred

- Evidence:
  - `release_confirm_left` starts at `0`:
    - `crates/app_core/src/asr.rs:1844`
  - `release_confirmed` becomes true whenever `release_confirm_left == 0`, even if no release event has ever happened:
    - `1866-1873`
  - trend entries are then permitted behind `if release_confirmed`:
    - `1893-1912`
- Why it matters:
  - this means `confirm_bars` currently delays after a release, but does not require a release to happen first
  - for a strategy named Adaptive Squeeze Regime, that is at minimum a hidden contract and likely a real logic bug
- Cross-file inconsistency:
  - one CUDA kernel branch already uses a stricter form that requires `confirm_bars_clamped > 0` and `!squeeze_on_now` before treating the release as confirmed:
    - `crates/app_core/src/kernels/double_crossover.cu:10768-10772`
- Smallest justified fix:
  - introduce explicit state tracking for “have observed a qualifying release”
  - then add CPU/GPU parity tests before changing behavior
- Review judgment:
  - semantics-sensitive, but the inconsistency is objective and should not remain undocumented

## API And Naming

### 1. `AsrStrategyParams` is public and heavily defaulted, so its validated contract should be tighter than it is today

- Evidence:
  - public parameter struct:
    - `crates/app_core/src/asr.rs:44-98`
  - re-export:
    - `crates/app_core/src/lib.rs:120`
- Why it matters:
  - downstream code reasonably treats this as the canonical ASR contract
  - validation gaps become surface gaps, not just internal helper issues
- Smallest justified fix:
  - keep the current shape, but make its accepted value domain explicit through validation and doc comments

## Error Handling

### 1. Preferred-GPU indicator execution can fail silently and fall back to CPU without route diagnostics

- Evidence:
  - `compute_indicator_f64` attempts CUDA when preferred:
    - `crates/app_core/src/asr.rs:542-560`
  - if CUDA fails, it just continues into CPU computation:
    - `565-574`
  - no diagnostic is recorded in `ASR_ROUTE_DIAGNOSTICS` on that path
- Why it matters:
  - backend routing becomes harder to explain and validate
  - performance regressions or unexpected CPU fallback can disappear into normal execution
- Smallest justified fix:
  - record a short route-diagnostics entry whenever a preferred GPU indicator attempt fails and CPU fallback is used
- Review judgment:
  - moderate operational issue, especially for performance-sensitive review work

## Module Boundaries And Technical Debt

### 1. The file is large but still strategy-local, so the right cleanup target is contract clarity, not broad decomposition

- Evidence:
  - one module currently owns params, CPU/GPU signal builders, position logic, and final simulation
- Why it matters:
  - splitting it aggressively would make strategy-local iteration harder
  - the more valuable work is clarifying invariants and expanding tests around the decision logic
- Review judgment:
  - avoid generic abstraction churn unless multiple strategy modules are being aligned together

## Docs And Examples

### 1. Several behavior-sensitive ASR contracts are still encoded only in code

- Missing documentation topics:
  - whether trend entries require a prior squeeze release
  - valid value range for `z_nbdev`
  - when GPU preference is allowed to fall back silently to CPU
  - which `z_ma_type` / `z_devtype` combinations remain on direct CUDA vs general fallback paths

## Tests And Missing Verification

### 1. Existing coverage is useful but thin relative to the file's surface area

- Verified in this review pass:
  - `cargo test -p ta_app_core asr -- --nocapture`
    - passed `7` relevant tests including the local ASR test and dependent composite smoke tests
- Existing strengths:
  - core17/trade22 metrics emission:
    - `crates/app_core/src/asr.rs:2861-2902`
  - composite CPU and validation smoke coverage for ASR routes:
    - surfaced in the filtered test run

### 2. Important ASR-specific edges remain untested

- Missing tests found in this review pass:
  - direct validation coverage for invalid `z_nbdev`
  - CPU desired-position behavior before the first squeeze release
  - CPU/GPU parity for release-confirm semantics
  - explicit route-diagnostics behavior when a preferred GPU indicator attempt falls back to CPU

### 3. Exact local verification required if changed

- `cargo test -p ta_app_core asr -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- if CUDA routing or device sweeps change:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. The direct CUDA sweep/device path is valuable and should not be flattened into CPU-first logic

- Evidence:
  - this file has dedicated direct CUDA sweep helpers for squeeze, momentum, angle, ER, and z-score:
    - `668-871`
    - `1411-1596`
- Review judgment:
  - preserve the direct-path architecture
  - focus on route clarity and parity, not on removing GPU specialization

### 2. Silent fallback is the main operational perf concern here, not raw algorithmic inefficiency

- Why it matters:
  - unexpected CPU fallback can distort performance conclusions even when the algorithm remains correct
- Review judgment:
  - route diagnostics are a higher-value improvement than local allocation micro-optimizations

## Code-Quality Improvement Register

- Safe now:
  - validate `z_nbdev > 0`
  - document release-confirm intent
  - log preferred-GPU fallback into route diagnostics
- Defer:
  - any broader ASR architecture split
  - any semantics change to trend-entry gating until CPU/GPU parity is explicitly tested

## Technical-Debt Reduction Register

- Safe to reduce now:
  - incomplete ASR parameter validation
  - missing route diagnostics on silent fallback
- Defer because it is semantics-sensitive:
  - changing the release-confirm state machine
- Useful debt that should not be “cleaned up” casually:
  - keeping ASR-specific CPU/GPU orchestration local to the ASR module

## Performance Opportunity Register

- Safe structural improvement:
  - better route diagnostics for unexpected CPU fallback
- Benchmark-sensitive or semantics-sensitive:
  - changing release-confirm entry logic
  - changing direct-vs-general CUDA sweep selection policy

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - preserve current ASR CPU and CUDA route availability
  - preserve metric field coverage and report compatibility
- Regression risk if changed carelessly:
  - candidate ranking can shift
  - CPU/GPU parity can diverge further
  - route reporting can become less trustworthy
- Safe change posture:
  - prefer validation hardening, diagnostics, and parity tests before altering strategy semantics
