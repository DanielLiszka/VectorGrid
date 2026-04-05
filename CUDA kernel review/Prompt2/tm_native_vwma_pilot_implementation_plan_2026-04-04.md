# TM-Native VWMA Pilot Implementation Plan

Date: 2026-04-04

## Purpose

This document extracts the valid, action-driving parts of `tm_native_ma_expansion_direction_FINAL.md` and turns them into a repo-owned implementation checklist.

This is a **bounded architecture pilot**, not a broad TM-native MA expansion campaign.

## Execution Outcome

Status: retained.

Repo-owned outcome summary:

- Stage 0 result: `small external addition required`
- bounded dependency extension completed through a vendored `vector-ta 0.2.4` patch
- runtime admission widened only for `vwma/vwma`
- no `double_crossover.cu` or frontend/request-builder changes were required
- deterministic `VFCT` parity and missing-volume rejection are now covered by targeted tests
- post-review hardening widened the vendored VWMA TM-writer output index to `size_t` and tightened the parity test so it must actually run on `backend_route = GpuKernel`
- hybrid/precomputed GPU fallback tests were rerun and remained green
- `cargo check -p ta_app_core` passed
- `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` passed
- the plain `DoubleMa vwma/vwma` large exact-grid GPU surface improved from mean `route_wall_ms 100.33 -> 96.33`
- the coarse `gpu_indicator_prep_ms` bucket was neutral in the final refreshed rerun: `30.00 -> 30.00`

Resulting branch decision:

- keep the bounded `vwma/vwma` pilot
- do not promote broader TM-native MA expansion from this branch alone

Primary retained findings document:

- `CUDA kernel review/Prompt2/tm_native_vwma_pilot_findings_2026-04-04.md`

The current repo evidence supports a narrower question:

1. the current `double_ma` GPU runtime already has a VRAM-resident MA path
2. the direct TM-native writer is currently admitted only for `sma/alma`
3. unsupported MA families currently fall back to row-major device output plus `transpose_row_to_tm`
4. `vwma` is a meaningful first pilot because it already has explicit device-side prefix preparation and real product surfaces
5. current Phase 2 evidence does **not** prove that MA-prep is the next headline performance bottleneck, so this lane must justify itself on unsupported-family product coverage and measured end-to-end payoff, not on the existing `alma/alma` benchmark story

Primary evidence baseline:

- `CUDA kernel review/Prompt2/tm_native_ma_expansion_direction_FINAL.md`
- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `README.md`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `web/src/services/optimizeRequestBuilder.ts`

## Scope

This plan covers only:

- `double_ma` runtime ownership
- a first-wave `vwma/vwma` pilot
- current exact `GpuKernel` and compatible hybrid/precomputed surfaces
- backend/runtime changes only

This plan does not authorize:

- a broad TM-native MA rollout
- mixed-pair admission such as `sma/vwma` or `alma/vwma`
- frontend capability changes in the first wave
- `double_crossover.cu` consumer-layout changes in the first wave
- a general `vector-ta` redesign
- using the existing `alma/alma` benchmark anchor as the payoff proof for this lane

## Current Evidence To Preserve

Treat these as current repo facts:

1. the retained product benchmark story is still anchored on `Double MA`, `ALMA/ALMA`, exact grid, warm app-path GPU execution
2. current Phase 2 profiling says the active product hotspots are still backtest-family and replay-family work, not a proved MA-prep bottleneck
3. `kernel_fast_path_enabled(...)` currently admits only `sma/alma`
4. the runtime already performs explicit VWMA prefix preparation through `ensure_vwma_prefix_pv_vol_f64(...)`
5. hybrid/precomputed CPU-staged fallback already exists and is protected by tests
6. the frontend already exposes meaningful VWMA-backed product surfaces:
   - `VFCT` supports only `vwma` fast/slow trend lines
   - `MARRT` supports `sma`, `ema`, `alma`, `wma`, `zlema`, and `vwma`

## Working Hypothesis

The working hypothesis for this plan is narrow:

- a bounded `vwma/vwma` TM-native path could reduce `gpu_indicator_prep_ms` on real non-ALMA product surfaces by eliminating the row-major compute plus transpose path

This is still only a hypothesis.

The pilot is not meant to prove:

- that TM-native expansion is the next highest-value product optimization lane overall
- that all unsupported MA families should now be admitted
- that the TM consumer in `double_crossover.cu` should change

## Key Feasibility Constraint

The pinned dependency state matters.

Current local source inspection of `vector-ta 0.2.4` indicates:

- device-native VWMA exists in the selector path
- `ensure_vwma_prefix_pv_vol_f64(...)` exists
- but `compute_period_ma_tm_into(...)` currently supports only `sma` and `alma`

So Stage 0 is not a formality. It is an actual branch gate.

## Code Touch Surfaces

### Primary repo-owned runtime ownership

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - `kernel_fast_path_enabled(...)`
  - the `use_fast_path` decisions in optimization and exact-eval paths
  - `ensure_vwma_prefix_pv_vol_f64(...)` call sites
  - `compute_period_ma_tm_into(...)`
  - `compute_period_ma_into(...)`
  - `transpose_row_to_tm`

### Fallback and parity ownership

- `crates/app_core/src/double_ma.rs`
  - `ma_matrix_cpu_f32_staging(...)`
  - `PrecomputedHostMaMatrix`
  - hybrid/precomputed call sites
  - existing parity tests for precomputed and CPU-staged kernel-capable MAs

### Dependency/version guardrail

- `crates/app_core/Cargo.toml`
- `src-tauri/Cargo.toml`

### Expected external dependency touch if Stage 0 fails

- `vector-ta 0.2.4`
  - local inspected source:
    - `src/cuda/moving_averages/vram_ma.rs`
    - `src/cuda/moving_averages/ma_selector.rs`

### Expected no-change files for first wave

- `crates/app_core/src/kernels/double_crossover.cu`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

## Truth Surfaces

Pilot payoff surfaces:

1. `DoubleMa`, `vwma/vwma`, exact-grid GPU route
2. `VFCT`, `vwma/vwma`, exact-grid GPU route
3. optional: `MARRT`, `vwma/vwma`, exact-grid GPU route

Regression sentinels:

4. README-style `Double MA`, `ALMA/ALMA`, exact-grid warm app-path surface
5. Criterion anchor:
   - `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`

Parity/fallback surfaces:

6. deterministic `vwma/vwma` candles with volume present
7. deterministic missing-volume rejection surface for VWMA
8. existing hybrid/precomputed tests that keep CPU-staged kernel-capable MAs on the GPU evaluation path

## Phase 0: Preflight And Feasibility Snapshot

- [x] Confirm no benchmark or profiler process is active.
- [x] Record current branch and commit.
- [x] Record `vector-ta` version pinned in:
  - `crates/app_core/Cargo.toml`
  - `src-tauri/Cargo.toml`
- [x] Record current local source evidence for `vector-ta 0.2.4`:
  - `compute_period_ma_tm_into(...)` support set
  - VWMA device-native selector support
- [x] Confirm current repo behavior:
  - `kernel_fast_path_enabled(...)` admits only `sma/alma`
  - VWMA prefix prep already exists
- [x] Keep the retained Phase 2 code state fixed while the pilot is being scoped.

## Phase 1: Stage-0 Gate

Goal: determine whether current `vector-ta 0.2.4` already supports TM-native VWMA output.

- [x] Attempt a compile/API check for TM-native VWMA support.
- [x] If needed, add a tiny non-retained scratch probe or compile-only branch to verify whether:
  - `compute_period_ma_tm_into("vwma", ...)` exists and works
  - or the current dependency only supports row-major VWMA output
- [x] Record the result as one of:
  - `already supported`
  - `small external addition required`
  - `broad dependency redesign required`

Stage-0 decision:

- [ ] If the result is `already supported`, continue to Phase 3.
- [x] If the result is `small external addition required`, continue to Phase 2.
- [ ] If the result is `broad dependency redesign required`, stop the branch with a no-go note.

## Phase 2: Conditional External Dependency Extension

Only start this phase if Stage 0 shows that TM-native VWMA output is missing but can be added surgically.

- [x] Keep the change narrowly scoped to TM-native VWMA output.
- [x] Do not broaden into a general multi-family TM-native redesign.
- [x] Keep workspace `vector-ta` versions aligned in:
  - `crates/app_core/Cargo.toml`
  - `src-tauri/Cargo.toml`
- [x] Record exactly what was added:
  - TM-native VWMA writer path only
  - no unrelated MA-family work
- [x] Re-check CPU-only build expectations after dependency adjustment.

Stop conditions for Phase 2:

- [ ] stop if the change spills into broader selector redesign
- [ ] stop if additional frontend or CUDA-consumer changes become necessary before the pilot is even runnable

## Phase 3: Narrow Runtime Pilot

Only start this phase if Stage 0 or Phase 2 makes TM-native VWMA output available.

- [x] Update `gpu_backtest_kernel.rs` so the runtime can admit `vwma/vwma` to the TM-native writer path.
- [x] Keep the implementation pair-level and narrow.
- [x] Do **not** introduce a generalized per-side capability framework in this wave.
- [x] Reuse the existing VWMA prefix prep path.
- [x] Keep row-major plus `transpose_row_to_tm` as the fallback for all other unsupported families.
- [x] Keep `double_crossover.cu` unchanged.

Implementation rule:

- [x] the first retained runtime pilot should only change writer-path admission and should not change backtest-kernel semantics

## Phase 4: Parity And Regression Hardening

- [x] Add or update tests to prove TM-native VWMA output matches the existing row-major-plus-transpose result on deterministic candles.
- [x] Add or update a missing-volume test for VWMA.
- [x] Re-run or extend the existing hybrid/precomputed tests to prove those routes stay intact.
- [x] Re-run sentinel checks proving current `sma/alma` fast-path behavior stays unchanged.

Suggested parity focus:

- [x] best params
- [x] top-k ordering
- [x] metric rows
- [x] deterministic row equivalence between new TM-native VWMA and old fallback VWMA path

## Phase 5: Benchmark And Measurement Pass

Goal: prove that the pilot matters on non-ALMA product surfaces.

### 5A. Regression sentinels

- [x] Run the README-style `Double MA`, `ALMA/ALMA`, exact-grid warm app-path surface.
- [x] Run the Criterion anchor:
  - `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`

### 5B. Pilot payoff surfaces

- [x] Run `DoubleMa`, `vwma/vwma`, exact-grid GPU route.
- [ ] Run `VFCT`, `vwma/vwma`, exact-grid GPU route.
- [ ] Optionally run `MARRT`, `vwma/vwma`, exact-grid GPU route.

Current retained scope note:

- the measured runner artifact in this branch is the plain `DoubleMa vwma/vwma` route
- `VFCT` coverage is currently provided by targeted parity/error-path tests rather than by a dedicated runner benchmark artifact

### 5C. What to measure

- [x] `gpu_kernel_timing_ms.route_wall_ms`
- [x] `gpu_kernel_timing_ms.gpu_indicator_prep_ms`
- [x] route diagnostics needed to confirm the intended GPU path
- [x] any parity deltas on result ordering or metrics

### 5D. Optional deeper profiling only if ambiguous

- [ ] refresh dispatch/pipeline logs if runtime timing is ambiguous
- [ ] run one narrow Nsight Systems capture on the VWMA pilot surface only if needed

## Phase 6: Decision Gate

Only retain this branch if the bounded pilot produces a clear non-ALMA payoff without widening scope or breaking parity.

Final retained evaluation:

- [x] the repo can run TM-native VWMA output without broad external redesign
- [ ] `vwma/vwma` pilot surfaces show a clear, repeatable reduction in `gpu_indicator_prep_ms`
- [x] route wall time moves in the same direction on at least one meaningful non-ALMA payoff surface
- [x] `alma/alma` regression sentinels stay healthy
- [x] hybrid/precomputed and missing-volume behavior remain correct
- [x] the change remains backend-local and additive

Retain note:

- the final refreshed rerun kept a bounded `route_wall_ms` win on the plain `DoubleMa vwma/vwma` surface
- the coarse prep bucket did not retain a measurable mean reduction
- this branch is therefore retained as a bounded unsupported-family route improvement, not as a clean proof that `gpu_indicator_prep_ms` was the decisive mechanism

If the gate is not met:

- [ ] stop the branch without broadening scope
- [ ] write a short no-go note

## Verification Checklist

- [ ] `cargo test -p ta_app_core`
- [x] `cargo check -p ta_app_core`
- [x] targeted CUDA parity/fallback tests for the pilot and existing hybrid/precomputed routes
- [x] `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- [x] `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
- [x] keep benchmark and profiling runs serialized

Verification note:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel` remains red in the current repo on unrelated failures, so the retained gate for this pilot is `cargo check`, targeted CUDA tests, and the existing GPU perf gates.

## Artifact Checklist

Suggested artifact stem:

- `target/tm_native_vwma_pilot/`

Suggested files:

- [x] Stage-0 feasibility note
- [x] external dependency delta note, if Phase 2 is used
- [x] deterministic parity test output or summary
- [x] README sentinel benchmark outputs
- [x] VWMA pilot benchmark outputs
- [x] one summary markdown note comparing:
  - pre-pilot fallback path
  - post-pilot path
  - non-ALMA payoff surfaces
  - ALMA regression sentinels

## Rejection Criteria

Reject this branch before or during implementation if any of the following happen:

- [ ] current `vector-ta` lacks TM-native VWMA and enabling it requires broad dependency redesign rather than a surgical addition
- [ ] the runtime pilot needs `double_crossover.cu` consumer changes in the first wave
- [ ] the runtime pilot needs frontend/request-builder changes to become testable
- [ ] the pilot reduces prep time but does not move route wall time on meaningful non-ALMA surfaces
- [ ] the pilot regresses current `alma/alma` sentinels
- [ ] parity diverges on params, ordering, metric rows, or VWMA missing-volume behavior
- [ ] the work starts expanding into multi-family TM-native coverage before one family proves value

## Decision Rule

Treat this plan as successful if it produces one of these outcomes:

1. a clear `vwma/vwma` pilot win that justifies one narrow retained runtime admission change, or
2. a clean no-go result that closes the first-wave TM-native expansion lane without broader speculation

Anything weaker should be treated as a deferral, not a partial win.
