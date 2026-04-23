# Review Packet: `crates/app_core/src/vemr.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vemr.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful strategy-local module, but not fully review-ready because omission defaults, route diagnostics, and the CPU/CUDA ownership split are not documented well enough`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Strategy-local`, `CPU/GPU-bridge`, `Config-defaults`, `Semantics-sensitive`

## What This File Owns

`vemr.rs` owns VEMR parameter/default handling plus CPU/CUDA indicator staging:

- parameter defaults and route-diagnostic controls: `31-183`
- period resolution and indicator-id lookup helpers: `380-587`
- CUDA sweep helpers and indicator materialization: `588-922`
- CPU indicator computation and signal assembly: `923-1141`
- start-index and desired-position logic: `1122-1299`
- file-local tests: `1300-1371`

## Classification

- Public surface: yes
  - exported from `crates/app_core/src/lib.rs`
- Hot path: partially
- CUDA/runtime-critical: partially
  - CPU logic and CUDA staging coexist
- Strategy-local: yes

## Direct Interactions

### Backend interaction map

- main optimization/validation consumers:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`
- GPU-kernel companion:
  - `crates/app_core/src/vemr_gpu_kernel.rs`
- dynamic indicator infrastructure:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
  - `crates/app_core/src/indicator_catalog.rs`

### Downstream verification impact if changed

- changing omission defaults or mapping toggles changes request semantics
- changing desired-position rules changes CPU/GPU parity and shortlist ranking
- changing route diagnostics or test-only fallback controls changes observability and test posture

## Standards And Research Log

This packet was written after targeted web research focused on serde defaults, thread-local controls, and strategy-local CPU/CUDA staging.

### Sources consulted

1. Serde field defaults
   - Source: <https://serde.rs/attr-default.html>
   - Why it was consulted:
     - `VemrStrategyParams` is omission-driven
   - Standard informed:
     - default mapping and fallback semantics should be explicit

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why it was consulted:
     - this file stores route diagnostics and test-only fallback controls in thread-local state
   - Standard informed:
     - thread-local scope and intended use should be documented

3. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - the file stages indicator sweeps that feed the dedicated CUDA kernel path
   - Standard informed:
     - staging logic should be preserved unless parity and perf are revalidated

## Findings

## Contract Clarity

### 1. `VemrStrategyParams` exposes important omission-based mapping behavior, but that behavior is not documented locally

- Evidence:
  - defaults and mapping toggles live at `31-123`
  - resolution logic begins at `380`
- Why this matters:
  - callers need to know which user-specified values are honored directly versus derived from fast/slow MA ranges
- Smallest justified fix:
  - add rustdoc to `VemrStrategyParams` and `resolve_vemr_periods`

## Diagnostics And Fallbacks

### 2. Thread-local route diagnostics and forced CPU fallback controls are useful maintenance hooks that are effectively hidden

- Evidence:
  - diagnostics and test-only fallback controls live at `127-183`
- Why this matters:
  - these controls explain why some requests intentionally remain on CPU during tests or degraded runtime paths
- Smallest justified fix:
  - document that the controls are thread-local, test-oriented, and part of the route-observability contract

## Module Boundaries And Technical Debt

### 3. The file currently owns four distinct responsibilities that should not be casually refactored together

- Evidence:
  - config/defaults: `31-183`
  - indicator descriptor caching via `OnceLock`: `531-587`
  - CUDA sweep staging: `588-922`
  - CPU signal and desired-position logic: `923-1299`
- Why this matters:
  - the file is large because it spans configuration, indicator resolution, GPU staging, and actual strategy semantics
- Smallest justified fix:
  - document the split now and defer extraction until a strategy-local refactor pass can preserve CPU/GPU parity

## Docs And Examples

### 4. The `OnceLock` indicator-id cache protects repeated descriptor lookup cost, but readers have to infer its role

- Evidence:
  - cached IDs live at `531-587`
- Why this matters:
  - this is useful debt, not accidental indirection
- Smallest justified fix:
  - add a short comment explaining that the cache stabilizes descriptor/key lookup, not strategy behavior

## Tests And Missing Verification

### 5. Default-build tests cover mapping and desired-position semantics, but not live CUDA sweep execution in this pass

- Evidence:
  - file-local tests begin at `1300`
- Missing coverage:
  - CUDA sweep/device paths are feature-gated and indirectly exercised elsewhere
- Smallest justified fix:
  - keep CUDA-facing changes conservative and verify them through the composite workflow tests before shipping

## Code-Quality Improvement Register

- Safe now:
  - document omission defaults and period mapping
  - document thread-local diagnostics/fallback scope
  - document the `OnceLock` cache role
- Coordination required:
  - extracting CUDA sweep helpers
  - splitting config resolution from signal assembly
- Defer:
  - broader module reorganization

## Technical-Debt Reduction Register

- Safe now:
  - comments and rustdoc
- Defer:
  - responsibility split across multiple files
- Do not casually remove:
  - thread-local diagnostics
  - forced test fallback hooks
  - indicator-id cache
  - current desired-position semantics

## Performance Opportunity Register

- Safe structural work:
  - documentation only
- Benchmark-sensitive:
  - changing CUDA sweep staging
  - changing cache construction or reuse

## Recommended Action

`Larger refactor but defer`

The file would benefit from a cleaner responsibility split, but that split should happen only in a strategy-local pass that validates CPU/GPU parity. The safe work now is documentation and contract clarification.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core vemr::tests:: -- --nocapture`
2. Representative VEMR composite workflow coverage:
   - `cargo test -p ta_app_core composite_cpu -- --nocapture`
   - `cargo test -p ta_app_core composite_validate -- --nocapture`
3. If CUDA-facing behavior changes:
   - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu::tests::vemr_gpu_kernel_supports_standalone_shortlist_extras -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core vemr::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change omission-based defaults or period mapping casually
- Preserve route diagnostics and test-only fallback controls unless equivalent observability remains
- Preserve current desired-position semantics unless CPU/GPU parity and shortlist behavior are revalidated
