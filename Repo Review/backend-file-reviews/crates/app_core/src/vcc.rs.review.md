# Review Packet: `crates/app_core/src/vcc.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vcc.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Important strategy-local module, but not fully review-ready because parameter defaults, route-diagnostic behavior, and CPU/GPU sweep contracts are only partially documented`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Strategy-local`, `CPU/GPU-bridge`, `Config-defaults`, `Semantics-sensitive`

## What This File Owns

`vcc.rs` owns the VCC strategy’s parameter contract and signal construction:

- `VccStrategyParams` plus defaults and validation: `33-307`
- period resolution and signal/result structs: `177-283`
- route diagnostics and test-only fallback controls: `100-156`
- CPU indicator computation and CUDA device-sweep helpers: `372-783`
- signal building, start index, and desired-position generation: `785-1041`
- extensive file-local tests: `1042-2216`

## Classification

- Public surface: yes
  - exported through `crates/app_core/src/lib.rs`
- Hot path: partially
  - signal generation is runtime work
- CUDA/runtime-critical: partially
  - both CPU and CUDA-assisted sweep paths live here
- Strategy-local: yes
- Test-sensitive: yes

## Direct Interactions

### Backend interaction map

- optimization/runtime consumers:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`
- GPU-kernel companion:
  - `crates/app_core/src/vcc_gpu_kernel.rs`

### Downstream verification impact if changed

- changing defaults or validation changes request legality and optimization behavior
- changing route diagnostics affects CPU/GPU fallback observability
- changing desired-position semantics affects CPU/GPU parity surfaces

## Standards And Research Log

This packet was written after targeted web research focused on serde default behavior, thread-local state, and CUDA best-practice caution for hybrid paths.

### Sources consulted

1. Serde field defaults
   - Source: <https://serde.rs/attr-default.html>
   - Why it was consulted:
     - `VccStrategyParams` is mostly omission-driven defaults
   - Standard informed:
     - defaults should be documented because they materially change runtime behavior

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why it was consulted:
     - route diagnostics and test-only fallback toggles are thread-local
   - Standard informed:
     - thread-local diagnostics/fallbacks need explicit scope documentation

3. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file mixes CPU paths with CUDA direct/device sweep helpers
   - Standard informed:
     - benchmark-sensitive CPU/GPU routing should not be “simplified” casually

## Findings

## Contract Clarity

### 1. `VccStrategyParams` encodes many behaviorally important defaults and mapping toggles without local documentation

- Evidence:
  - multiple `map_*` toggles and numeric defaults live at `33-99`
  - validation is present, but semantics are not described near the type: `284-307`
- Why this matters:
  - callers crossing the Tauri/backend boundary can omit fields and still get specific strategy behavior
  - maintainers need to know which periods default independently versus which map from fast/slow MA inputs
- Smallest justified fix:
  - add rustdoc to `VccStrategyParams` and `resolve_vcc_periods`

## Module Boundaries

### 2. The file combines config validation, route diagnostics, CPU indicator computation, CUDA sweep support, and desired-position logic

- Evidence:
  - diagnostics/test controls: `100-156`
  - CPU/CUDA indicator helpers: `372-783`
  - position construction: `933-1041`
- Why this matters:
  - this is a large strategy-local module with several distinct responsibilities
  - broad “cleanup” work here would carry parity risk
- Smallest justified fix:
  - document the sub-boundaries now and defer any extraction until parity surfaces are reviewed together

## Diagnostics And Test Controls

### 3. Route diagnostics and forced CUDA fallback are useful, but the thread-local contract is implicit

- Evidence:
  - diagnostics and fallback toggles are thread-local under CUDA builds: `100-156`
- Why this matters:
  - debugging and tests depend on those side channels behaving predictably within a thread
  - future contributors could incorrectly assume process-global visibility
- Smallest justified fix:
  - add comments documenting thread-local scope and intended use

## Tests And Missing Verification

### 4. The file has strong parity-oriented tests, which should be preserved

- Evidence:
  - file-local tests reach well beyond validation-only checks and include CPU/device position and metric parity surfaces: `1042-2216`
- Smallest justified fix:
  - keep these tests intact and treat them as contract locks before any structural change

## Code-Quality Improvement Register

- Safe now:
  - document defaults and mapping rules
  - document thread-local diagnostics/fallback scope
  - document module sub-boundaries
- Defer:
  - splitting CPU/CUDA helpers into separate modules
  - changing routing or sweep behavior

## Technical-Debt Reduction Register

- Safe now:
  - documentation hardening
- Defer:
  - module extraction
- Do not casually remove:
  - route diagnostics
  - parity helpers
  - current desired-position semantics

## Performance Opportunity Register

- Safe structural work:
  - none beyond docs/comments
- Benchmark-sensitive:
  - changes to CUDA device sweeps, signal construction, or CPU/GPU fallback policy

## Recommended Action

`Larger refactor but defer`

The useful short-term work is to document defaults, route diagnostics, and the CPU/GPU split. Structural cleanup should wait until parity coverage is preserved explicitly.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core vcc::tests:: -- --nocapture`
2. Representative composite runtime/validation surfaces:
   - `cargo test -p ta_app_core composite_cpu -- --nocapture`
   - `cargo test -p ta_app_core composite_validate -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core vcc::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change omission-based defaults casually; request behavior depends on them
- Do not narrow or remove thread-local diagnostics without checking composite runtime/validation debugging paths
- Treat CPU/device parity tests as behavior locks, not optional coverage
