# Review Packet: `crates/app_core/src/vram_ma.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vram_ma.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Tiny feature-gated wrapper, but its purpose as a stable re-export seam should be documented`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Feature-gated`, `CUDA-bridge`, `Support-code`

## What This File Owns

`vram_ma.rs` is a thin feature-gated bridge module:

- CUDA-only module gate: `1`
- re-exports of `VramMaInputs`, `supports_vram_kernel_ma`, and `VramMaComputer`: `3-4`

## Classification

- Public surface: yes
  - exported from `crates/app_core/src/lib.rs`
- Hot path: no
- CUDA/runtime-critical: indirectly
  - it gates access to CUDA moving-average helpers
- Strategy-local: no
  - shared support seam

## Direct Interactions

### Backend interaction map

- main consumers:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- upstream implementation source:
  - `vector_ta::cuda::moving_averages::vram_ma`

### Downstream verification impact if changed

- changing this module changes the stable import path for VRAM-moving-average support checks and helpers
- removing the wrapper forces wider call-site churn with no direct runtime value

## Standards And Research Log

This packet was written after targeted web research focused on conditional compilation and stable module seams.

### Sources consulted

1. Rust Reference: conditional compilation
   - Source: <https://doc.rust-lang.org/reference/conditional-compilation.html>
   - Why it was consulted:
     - this module exists entirely behind feature gates
   - Standard informed:
     - the gating contract should be explicit

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - tiny bridge modules still need to explain why they exist
   - Standard informed:
     - document intent when a wrapper is preserving a public seam

## Findings

## Docs And Examples

### 1. The file’s entire value is the stable feature-gated re-export seam, but that intent is not stated

- Evidence:
  - the file only contains a `cfg` gate and re-exports: `1-4`
- Why this matters:
  - without a comment, future cleanup passes may inline or delete it as “redundant”
- Smallest justified fix:
  - add a short module comment explaining that app-core intentionally re-exports the vector-ta VRAM MA helpers from one stable location

## Technical Debt

### 2. This is useful indirection, not accidental duplication

- Evidence:
  - the module centralizes the CUDA feature gate and import path
- Why this matters:
  - keeping a single wrapper reduces call-site knowledge of the upstream crate layout
- Smallest justified fix:
  - document the seam and leave behavior unchanged

## Code-Quality Improvement Register

- Safe now:
  - add a module-level comment
- Defer:
  - no structural change needed

## Technical-Debt Reduction Register

- Safe now:
  - documentation only
- Do not casually remove:
  - this wrapper module
  - the current feature gate

## Performance Opportunity Register

- This file is not the place to chase performance

## Recommended Action

`Document only`

This wrapper is already doing the right thing. The useful improvement is to document why the wrapper exists so it is not removed in a future cleanup pass.

## Exact Local Verification Required If Changed

1. Build-level verification:
   - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
2. Representative GPU consumer verification:
   - `cargo test -p ta_app_core gpu_backtest_kernel --features cuda,cuda-backtest-kernel -- --nocapture`

### Verification run for this packet

- Not run in this review pass:
  - CUDA-gated build/tests for the VRAM MA seam

## Regression Risk And Feature-Preservation Constraints

- Preserve the current re-export path so shared GPU callers do not need cross-crate import knowledge
- Preserve the existing feature gate so CPU-only builds remain unaffected
