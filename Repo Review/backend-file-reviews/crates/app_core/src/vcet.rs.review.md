# Review Packet: `crates/app_core/src/vcet.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vcet.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful strategy-local module, but not fully review-ready because default mapping rules, route diagnostics, and CPU/CUDA staging behavior are not sufficiently documented`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Strategy-local`, `CPU/GPU-bridge`, `Config-defaults`, `Semantics-sensitive`

## What This File Owns

`vcet.rs` owns VCET parameter/default handling and signal construction:

- `VcetStrategyParams` plus defaults/validation: `33-313`
- route diagnostics and test-only fallbacks: `87-160`
- indicator-id cache and signal builders: `199-753`
- start index and desired-position logic: `754-877`
- file-local tests: `878-934`

## Classification

- Public surface: yes
  - exported from `crates/app_core/src/lib.rs`
- Hot path: partially
- CUDA/runtime-critical: partially
  - CPU and CUDA-facing helpers coexist
- Strategy-local: yes

## Direct Interactions

### Backend interaction map

- main consumers:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`
- GPU-kernel companion:
  - `crates/app_core/src/vcet_gpu_kernel.rs`

### Downstream verification impact if changed

- changing defaults or mapping rules changes request behavior
- changing desired-position semantics changes CPU/GPU parity
- changing route diagnostics affects fallback observability

## Standards And Research Log

This packet was written after targeted web research focused on serde defaults, thread-local diagnostics, and CUDA-support caution.

### Sources consulted

1. Serde field defaults
   - Source: <https://serde.rs/attr-default.html>
   - Why it was consulted:
     - `VcetStrategyParams` is omission-driven
   - Standard informed:
     - default mapping and numeric parameters should be documented

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why it was consulted:
     - route diagnostics and test fallbacks are thread-local
   - Standard informed:
     - diagnostic/fallback scope should be explicit

3. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file stages CPU/CUDA signal paths that feed kernel execution
   - Standard informed:
     - staging logic should be preserved unless measured otherwise

## Findings

## Contract Clarity

### 1. `VcetStrategyParams` needs local documentation for its mapping toggles and omission-based defaults

- Evidence:
  - defaults and toggles live at `33-86`
  - validation begins at `285`
- Why this matters:
  - callers need to know which periods derive from fast/slow MA ranges versus standalone defaults
- Smallest justified fix:
  - add rustdoc to the params type and `resolve_vcet_periods`

## Diagnostics And Fallbacks

### 2. Thread-local route diagnostics and host-stage fallback controls are useful but undocumented

- Evidence:
  - diagnostics and forced fallback knobs live at `87-160`
- Why this matters:
  - these side channels are part of how CPU/CUDA route behavior is tested and debugged
- Smallest justified fix:
  - document that the controls are thread-local and test-oriented

## Technical Debt

### 3. The file mixes config validation, indicator-ID caching, signal construction, and desired-position rules

- Evidence:
  - indicator-id cache via `OnceLock`: `199-456`
  - signal builders and CUDA helpers: `488-753`
  - desired-position logic: `775-877`
- Why this matters:
  - the file is still understandable, but a future refactor should split by responsibility rather than by “CPU vs GPU” alone
- Smallest justified fix:
  - document the internal ownership split now; defer extraction

## Tests And Missing Verification

### 4. Default-build tests are focused but narrow

- Evidence:
  - passed tests cover period mapping and desired-position behavior: `878-934`
- Missing coverage:
  - most CUDA-specific behavior is not exercised in this default-build review pass
- Smallest justified fix:
  - preserve current tests and treat CUDA-facing changes conservatively

## Code-Quality Improvement Register

- Safe now:
  - document defaults/mapping rules
  - document thread-local diagnostics/fallback scope
  - document internal ownership split
- Defer:
  - structural file split
  - route/staging changes

## Technical-Debt Reduction Register

- Safe now:
  - documentation/comments
- Defer:
  - module extraction
- Do not casually remove:
  - thread-local diagnostics
  - indicator-id cache
  - desired-position semantics

## Performance Opportunity Register

- Safe structural work:
  - none beyond docs/comments
- Benchmark-sensitive:
  - changing CPU/CUDA staging behavior

## Recommended Action

`Small surgical refactor`

The useful work is contract documentation, not behavioral change. VCET’s defaults and fallback controls should be explicit before anyone attempts deeper cleanup.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core vcet::tests:: -- --nocapture`
2. Representative composite consumers:
   - `cargo test -p ta_app_core composite_cpu -- --nocapture`
   - `cargo test -p ta_app_core composite_validate -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core vcet::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change omission-based defaults or mapping toggles casually
- Do not remove thread-local diagnostics/fallback knobs without checking current CUDA debug/test use
- Preserve existing desired-position semantics unless CPU/GPU parity is revalidated
