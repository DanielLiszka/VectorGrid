# Tensor Core ALMA Indicator-Prep Experiment Implementation Plan

Date: 2026-04-10

Status: repo-grounded experimental plan only. No Tensor Core ALMA path is approved for the shipped route today.

## Purpose

This document defines the valid experiment hidden inside the imported `tensor_cores/` bundle:

- test whether Tensor Cores can accelerate **ALMA family indicator preparation** enough to justify a niche, opt-in optimization in this repo

This is not a general Tensor Core adoption plan.
It is a narrow experiment plan for one fixed-weight moving-average family on one existing GPU execution path.

The core repo-valid question is:

- can a Tensor Core ALMA implementation beat the current VRAM-native ALMA CUDA path on large ALMA sweeps, while preserving the exact evaluator contract and ranked-result behavior?

If the answer is no, the correct outcome is to keep the existing `vector-ta` ALMA implementation and discard the experiment.

## Why This Idea Is More Repo-Valid Than The RT-Core Bundle

Unlike the RT-core bundle, the Tensor Core idea is pointed at a stage this repo already has and already cares about:

- GPU indicator preparation
- VRAM-resident exact search
- ALMA-heavy fast/slow MA families

Current repo facts:

1. The exact GPU route in `crates/app_core/src/gpu_backtest_kernel.rs` already prepares MA slabs on-device before the backtest kernel runs.
2. `ALMA` is already supported in the current VRAM MA stack through `VramMaComputer` and vendored `vector-ta`.
3. The current ALMA path already has dedicated time-major CUDA kernels in:
   - `vendor/vector-ta-0.2.4/src/cuda/moving_averages/alma_wrapper.rs`
   - `vendor/vector-ta-0.2.4/src/cuda/moving_averages/vram_ma.rs`
4. The exact evaluator already consumes time-major MA outputs via `d_fast_ma_tm` and `d_slow_ma_tm` in `crates/app_core/src/gpu_backtest_kernel.rs`.
5. The imported Tensor Core bundle also targets indicator preparation rather than the lockstep trade-state kernel.

So the valid experiment is:

- replace or augment the current **ALMA indicator-prep implementation** behind the existing VRAM MA interface

The invalid experiment is:

- bolt a separate ALMA stack into `ta_app_core` and bypass the existing `vector-ta` MA pipeline

## Imported Bundle: What Is Valid And What Is Not

Valid aspects from `tensor_cores/`:

- indicator-prep placement instead of backtest-body placement
- ALMA as the first Tensor Core candidate
- large-sweep / large-series gating
- Tensor Core use via GEMM-style batching
- TF32-fast as the first precision mode to test
- the requirement to measure end-to-end exact sweep impact, not just kernel time

Invalid or not-yet-valid aspects from `tensor_cores/`:

- a separate `crates/app_core/src/tensorcore_alma.rs` module as the main integration point
- build wiring that ignores the repo's existing CUDA feature model
- a benchmark binary that imports `app_core` instead of `ta_app_core`
- a separate native build path in `ta_app_core` instead of integrating with vendored `vector-ta`
- the implied row-major output slab contract instead of the current `*_tm` device-output contract
- any assumption that this should land directly in `gpu_backtest_kernel` as a parallel side path

## Current Baseline To Respect

The experiment must be evaluated against the current repo baseline, not against a naive CPU ALMA reference.

Current baseline facts:

1. `vector-ta` already ships ALMA CUDA kernels and a `CudaAlma` wrapper in:
   - `vendor/vector-ta-0.2.4/src/cuda/moving_averages/alma_wrapper.rs`
2. `VramMaComputer::compute_period_ma_tm_into(...)` already routes `alma` through `CudaAlma::alma_batch_device_tm(...)` in:
   - `vendor/vector-ta-0.2.4/src/cuda/moving_averages/vram_ma.rs`
3. The exact GPU path in `crates/app_core/src/gpu_backtest_kernel.rs` already calls:
   - `ma.compute_period_ma_tm_into(...)`
   - and writes into `d_fast_ma_tm` / `d_slow_ma_tm`
4. `vector-ta` already has its own CUDA build and PTX staging flow in:
   - `vendor/vector-ta-0.2.4/build.rs`

Interpretation:

- the experiment must beat the **existing ALMA CUDA path**
- not just a CPU reference
- and not just a synthetic cuBLASLt microbenchmark

## Real Use Or No-Go

This experiment only counts as useful if it improves a real repo workload.

Real candidate use cases:

1. Double-MA exact GPU route
   - large ALMA/ALMA sweeps
   - VRAM-resident exact search
   - large `bars x params` surfaces where indicator prep is non-trivial

2. Composite GPU-capable routes that reuse ALMA MA prep through the same VRAM MA stack

Potentially valid later:

3. any other `vector-ta` ALMA batch user in this repo that already consumes `compute_period_ma_tm_into(...)`

Not enough on its own:

- a benchmark binary that only shows GEMM speed
- a benchmark against CPU ALMA only
- a speedup on indicator prep that does not survive into the end-to-end exact route

If the only win is a microbenchmark win, the experiment fails the repo-fit test.

## Non-Goals

This plan does not authorize:

- replacing all moving averages with Tensor Core variants
- changing the backtest kernel body
- introducing a parallel MA stack beside `vector-ta`
- changing the product route policy
- making cuBLASLt a required dependency for standard CUDA builds
- changing ranked-result semantics for speed
- default-enabling lower-precision ALMA on shipped paths

## Working Hypothesis

The working hypothesis is narrow:

- ALMA is a fixed-weight sliding-window dot product
- on sufficiently large sweeps, Tensor Core GEMM may compute ALMA families faster than the current dedicated ALMA CUDA kernels
- if the output contract and numeric drift are acceptable, that win might reduce indicator-prep time and possibly end-to-end exact sweep time

Counter-hypothesis:

- the current tiled ALMA CUDA kernels in `vector-ta` are already well matched to this workload
- packing + GEMM overhead may erase or exceed any Tensor Core benefit
- the exact evaluator may be dominated elsewhere, making indicator-prep gains irrelevant

This plan assumes neither side is true without measurement.

## Viability Bar

The Tensor Core ALMA experiment should be considered viable only if all of the following are true:

- it integrates behind the existing ALMA VRAM interface rather than beside it
- it preserves the current `*_tm` output contract consumed by the exact evaluator
- CPU-only and normal CUDA builds remain unaffected when the experiment is off
- numeric drift stays inside a predeclared tolerance and does not meaningfully perturb ranking outcomes on target workloads
- at least one real exact-route truth surface shows:
  - at least `20%` indicator-prep improvement
  - and at least `8%` end-to-end exact-route improvement

Immediate no-go conditions:

- the Tensor Core path only wins on a synthetic benchmark
- it requires a second parallel ALMA pipeline in `ta_app_core`
- it cannot preserve the current time-major output contract cleanly
- result drift changes ordering or shortlist composition materially
- end-to-end route time does not improve meaningfully

## Output Contract Constraint

This is the most important technical constraint for the experiment.

The imported bundle talks about slabs like:

- `[rows, params]`

But the live repo does not care about that abstraction.
The live repo cares about the device output contract already consumed by the exact evaluator:

- `compute_period_ma_tm_into(...)`
- `d_fast_ma_tm`
- `d_slow_ma_tm`

Therefore:

- a valid Tensor Core ALMA path must either produce the current `*_tm` layout directly
- or adapt internally before returning to the caller

It must not push a new external row-major layout requirement into `gpu_backtest_kernel`.

## Correct Integration Seam

The correct integration seam is inside vendored `vector-ta`, not as a second ALMA module in `ta_app_core`.

Primary touch surfaces:

- `vendor/vector-ta-0.2.4/src/cuda/moving_averages/alma_wrapper.rs`
- `vendor/vector-ta-0.2.4/src/cuda/moving_averages/vram_ma.rs`
- `vendor/vector-ta-0.2.4/build.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Why:

1. `vector-ta` already owns ALMA CUDA kernels and PTX staging.
2. `VramMaComputer` already owns ALMA caching and dispatch.
3. `ta_app_core` already consumes that interface.

So the experiment should look like:

- optional alternate ALMA backend inside vendored `vector-ta`
- selected only for `alma`
- only on large-enough shapes
- and only when the experiment feature is enabled

## Architecture Direction

If the experiment proceeds, the intended shape is:

1. Add an experiment-gated Tensor Core ALMA backend behind `CudaAlma`
2. Keep `VramMaComputer::compute_period_ma_tm_into(...)` as the dispatch owner
3. Preserve existing ALMA parameter and cache semantics
4. Preserve existing device output layout and synchronization semantics
5. Expose enough route diagnostics to tell whether:
   - current ALMA CUDA path was used
   - or Tensor Core ALMA experiment path was used

Valid design options:

- add a Tensor Core implementation inside `vendor/vector-ta-0.2.4/src/cuda/moving_averages/alma_wrapper.rs`
- add a narrow helper module inside vendored `vector-ta` and call it from `CudaAlma`

Invalid design option:

- add an unrelated `tensorcore_alma.rs` module in `ta_app_core` and manually thread raw pointers through `gpu_backtest_kernel`

## Build And Feature Direction

The imported bundle's build wiring is not valid for this repo as written.

A valid experiment build shape should be:

1. feature-gated
2. off by default
3. nested under the existing CUDA-capable build flow
4. implemented where ALMA PTX/native build ownership already lives

Preferred direction:

- add a vendored `vector-ta` experiment feature such as `cuda-tensorcore-alma-experiment`
- forward it from `ta_app_core` only if needed
- keep standard `cuda` and `cuda-backtest-kernel` flows unaffected when the feature is off

The experiment must not:

- require cuBLASLt for ordinary CPU builds
- require cuBLASLt for ordinary CUDA builds when the experiment is off

## Precision Policy

The only reasonable initial precision mode is:

- TF32-fast first

Why:

- it is the least aggressive Tensor Core precision candidate
- it is more likely than FP16 to preserve ranking stability on price-series work

FP16 is not forbidden forever, but it is explicitly a second-wave experiment only after TF32-fast is measured and bounded.

## Measurement Surfaces

Primary truth surfaces:

1. Double-MA exact GPU route
   - `alma/alma`
   - large or xlarge bar counts
   - parameter counts large enough to trigger the experiment threshold
   - heatmap off first
   - no unrelated blockers or fallback triggers

2. A second Double-MA ALMA-heavy surface with different bars/params to check stability

3. One composite GPU-capable surface that still routes ALMA prep through the same VRAM MA stack

Control surfaces:

1. same workloads with the experiment disabled
2. small ALMA sweeps where the experiment should stay off
3. non-ALMA MA families to confirm no regression or accidental route widening

## Phase 0: Baseline Measurement Gate

Goal: determine whether current ALMA indicator prep is expensive enough to justify a Tensor Core branch.

- [ ] Confirm no benchmark or profiler process is active.
- [ ] Use existing repo runners or add a narrow benchmark driver for ALMA exact-route measurement.
- [ ] Measure the current ALMA CUDA path on at least:
  - [ ] 200k bars / ~250 ALMA params
  - [ ] a second larger or differently tiled shape
- [ ] Record:
  - [ ] indicator-prep time only
  - [ ] end-to-end exact sweep time
  - [ ] route diagnostics
- [ ] Confirm that ALMA prep is large enough to matter relative to full route time.

Phase-0 decision gate:

- [ ] Stop if ALMA prep cost is too small to plausibly move end-to-end route timing.

## Phase 1: Out-Of-Tree Native Spike

Goal: prove the Tensor Core ALMA kernel concept without touching the live repo path.

- [ ] Keep `tensor_cores/` out of the active build.
- [ ] Fix obvious spike-only issues:
  - [ ] crate naming
  - [ ] build assumptions
  - [ ] output-layout assumptions
- [ ] Prove Tensor Core ALMA output numerically matches the current ALMA reference closely enough for sampled checks.
- [ ] Measure pure prep time against the current ALMA CUDA path, not CPU only.

Phase-1 decision gate:

- [ ] Stop if the Tensor Core kernel cannot clearly beat the current ALMA CUDA path on its own stage.

## Phase 2: Vendored VectorTA Scaffolding

Goal: add a clean repo-native experiment seam in the right ownership layer.

- [ ] Add an experiment feature in vendored `vector-ta`.
- [ ] Keep it off by default.
- [ ] Extend the existing `vector-ta` build flow rather than adding a second CUDA build system in `ta_app_core`.
- [ ] Add route diagnostics or debug state to tell which ALMA backend ran.
- [ ] Preserve existing public interfaces where possible.

Phase-2 decision gate:

- [ ] Stop if the Tensor Core path cannot be isolated cleanly behind the current ALMA dispatch surface.

## Phase 3: ALMA Dispatch Prototype

Goal: integrate the experiment only behind the current ALMA VRAM path.

- [ ] Add Tensor Core ALMA dispatch behind `CudaAlma`.
- [ ] Trigger only when:
  - [ ] MA family is `alma`
  - [ ] bar count is large enough
  - [ ] parameter count is large enough
  - [ ] experiment feature is enabled
- [ ] Preserve the current `compute_period_ma_tm_into(...)` contract.
- [ ] Preserve existing ALMA weight/cache semantics where possible.
- [ ] Add parity tests for:
  - [ ] value agreement on sampled outputs
  - [ ] output dimensions and valid-from behavior
  - [ ] no accidental activation on unsupported shapes

Phase-3 decision gate:

- [ ] Promote only if the experiment preserves contract and beats the current ALMA CUDA prep path.

## Phase 4: End-To-End Exact Route Check

Goal: prove the experiment matters to the repo, not just to ALMA math.

- [ ] Measure full exact-route timing on Double-MA ALMA/ALMA truth surfaces.
- [ ] Compare:
  - [ ] experiment off
  - [ ] experiment on
- [ ] Record:
  - [ ] indicator-prep delta
  - [ ] end-to-end route delta
  - [ ] ranking drift
  - [ ] shortlist overlap
- [ ] Reject the experiment if prep wins do not survive into route-level gains.

Phase-4 decision gate:

- [ ] Stop if route-level wins are too small or unstable.

## Phase 5: Composite Follow-On

Goal: confirm there is at least one second consumer in the repo.

- [ ] Test one composite GPU-capable ALMA-heavy surface.
- [ ] Confirm it still routes through the same ALMA dispatch seam.
- [ ] Re-run drift and timing checks.

Phase-5 decision gate:

- [ ] Stop if the Double-MA win does not generalize even to one more real consumer.

## Verification Plan

Required validation categories:

- [ ] sampled numeric parity against the current ALMA CUDA path
- [ ] ranked-result and shortlist stability on real exact-route workloads
- [ ] CPU-only build smoke with the experiment off
- [ ] standard CUDA build smoke with the experiment off
- [ ] experiment build smoke with the feature on
- [ ] targeted `ta_app_core` and/or vendored `vector-ta` tests

Preferred command surfaces:

- [ ] targeted `cargo test -p ta_app_core ...`
- [ ] `cargo test --workspace` after retained integration
- [ ] existing profile runners for exact-route timing
- [ ] repo quality gate for retained cross-file changes

## Implementation Checklist

Use this as the consolidated go/no-go tracker.

- [ ] Measure current ALMA CUDA prep cost before writing integration code.
- [ ] Compare against the current ALMA CUDA path, not CPU only.
- [ ] Keep the experiment behind the existing ALMA VRAM interface.
- [ ] Keep the experiment out of the backtest kernel body.
- [ ] Keep CPU-only and standard CUDA builds unaffected when the experiment is off.
- [ ] Preserve the current `*_tm` output contract.
- [ ] Preserve current `valid_from` semantics.
- [ ] Use TF32-fast first.
- [ ] Prove sampled numeric parity is acceptable.
- [ ] Prove ranking drift stays acceptably low on real truth surfaces.
- [ ] Prove at least one real exact-route wall-time improvement.
- [ ] Reject the experiment if only the microbenchmark wins.
- [ ] Reject the experiment if it requires a second parallel ALMA stack in `ta_app_core`.

## Promotion Policy

This experiment should be promoted only in this order:

1. measure current ALMA cost
2. beat the current ALMA CUDA prep path
3. integrate behind vendored `vector-ta`
4. prove end-to-end exact-route gain
5. optionally widen to one additional real consumer

It should never be promoted in this order:

1. import patch bundle directly
2. add parallel ALMA code in `ta_app_core`
3. default-enable lower-precision ALMA globally

## Final Decision Rule

If the Tensor Core experiment:

- beats the current ALMA CUDA prep implementation
- preserves the exact-route contract
- and produces a real route-level win on large ALMA-heavy GPU workloads

then it is worth retaining as a niche repo optimization.

If it does not do all three, the correct outcome is:

- document the findings
- keep the experiment artifacts
- remove the code

That is still a successful experiment.
