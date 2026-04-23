# RT-Core Barrier Validation Experiment Implementation Plan

Date: 2026-04-10

Status: repo-grounded experimental plan only. No RT-core integration is approved for the shipped path.

## Purpose

This document defines the RT-core experiments that are currently worth considering in this repository:

- primary target: test whether RTX RT cores can accelerate **risk-enabled iterative validation** enough to justify a niche, opt-in feature in this repo
- secondary follow-on: test whether the same traversal machinery can help **moving channel/band first-exit scans** in standalone strategy loops such as ROCB and VEMR

This is intentionally **not** a general OptiX adoption plan.
The imported material in `rt_cores/` is treated as external inspiration, not as a valid drop-in patch.

To count as viable, the experiment must have a real home in the current product:

- fixed-candidate validation on shared OHLC history
- iterative validation with `stop_loss_pct` and/or `take_profit_pct`
- repeated candidate batches on the same dataset or resample
- optional later reuse in meta heavy-holdout reruns that already force CPU validation

If it cannot clear that bar, it should remain an abandoned experiment even if a synthetic microbenchmark looks good.

## Why The Iterative Barrier Lane Is Still The Best Target Surface

Current repo facts:

1. The current RT-core patch bundle points at `gpu_backtest_kernel`, but that is not the valid integration surface for this repo.
2. Double-MA GPU fixed-candidate validation currently supports only the vectorized validation engine in `crates/app_core/src/double_ma_validate.rs`.
3. Composite GPU fixed-candidate validation also supports only the vectorized validation engine in `crates/app_core/src/composite_validate.rs`.
4. GPU composite validation currently rejects overlays and stop-loss / take-profit shapes in `crates/app_core/src/composite_validate.rs`.
5. The actual stop-loss / take-profit intrabar logic for iterative validation lives on the CPU in:
   - `crates/app_core/src/iterative_sim.rs`
   - `crates/app_core/src/asr.rs`
6. Validation-side iterative execution is already wired through `crate::validation_pipeline::maybe_run_iterative_validation(...)` in `crates/app_core/src/validation_pipeline.rs`.
7. Meta heavy-holdout reruns already have CPU validation call sites in `src-tauri/src/commands_meta.rs`.

That means the only repo-valid RT-core question is:

- can RT cores help the **CPU validation fallback / iterative-risk lane**, where many candidate validations reuse the same candle history?

It is not:

- can RT cores be bolted onto the current generic GPU kernel optimizer path?

There is one additional RT-shaped idea worth acknowledging, but it is weaker than the primary target:

- ROCB, VEMR, and related standalone strategy loops already perform first-exit checks against moving channel, band, or ATR-derived barriers in:
  - `crates/app_core/src/rocb.rs`
  - `crates/app_core/src/vemr.rs`
  - `crates/app_core/src/mkcs.rs`
- those loops are real "earliest exit against a changing boundary" problems
- but they rank lower because:
  - the barriers move every bar, so scene reuse is harder to amortize cleanly
  - exit logic depends on more state than plain stop/take reachability, including flips and regime-specific entry/exit rules
  - the repo already ships dedicated CUDA kernels for these families in:
    - `crates/app_core/src/rocb_gpu_kernel.rs`
    - `crates/app_core/src/vemr_gpu_kernel.rs`
    - `crates/app_core/src/mkcs_gpu_kernel.rs`

So this guide keeps the iterative-risk lane as the primary experiment and treats moving-barrier exits only as an optional later follow-on.

## Imported Patch Bundle: What To Preserve And What To Reject

Useful ideas from `rt_cores/`:

- a dataset-scoped reusable barrier scene
- a batched earliest-hit query interface
- treating RT traversal as a broad-phase accelerator, not the authority on tie-break semantics

Things that are not repo-valid as written:

- integration into `gpu_backtest_kernel`
- environment-variable-only build activation
- per-call scene rebuilds
- hardcoded device `0`
- `f32`-only assumptions without repo-level parity proof
- a benchmark binary that is not wired to this repo's actual crate names or workload binaries

This plan keeps the first group and rejects the second.

## Real Use Or No-Go

The experiment is only worth keeping if it proves a real repo use case:

- repeated fixed-candidate validation where many candidates share the same candle history
- iterative validation with risk overlays enabled
- scene reuse across candidate batches or repeated reruns

Examples of potentially valid consumers:

1. `run_double_ma_validation_blocking_with_candles(...)` with:
   - `Backend::CpuOnly` or GPU fallback to CPU
   - `ValidationEngine::Iterative`
   - `stop_loss_pct` and/or `take_profit_pct`

2. `run_composite_validation_blocking_with_candles(...)` with:
   - `Backend::CpuOnly`
   - `ValidationEngine::Iterative`
   - strategy-level risk overlays enabled

3. heavy meta reruns in `src-tauri/src/commands_meta.rs` that already force `Backend::CpuOnly`

4. optional later strategy-local CPU loops with moving barrier exits:
   - ROCB channel or ATR first-exit scans in `crates/app_core/src/rocb.rs`
   - VEMR middle-band or ATR first-exit scans in `crates/app_core/src/vemr.rs`
   - MKCS only if the same helper shape generalizes cleanly after ROCB or VEMR

If the only measurable win is:

- an isolated earliest-hit microbenchmark
- or a synthetic OptiX scene query benchmark

then the experiment fails the repo-fit test.

## Secondary Candidate: Moving-Barrier First-Exit Scans

This is the best non-options RT-core candidate after the primary iterative-risk barrier path.

Repo-grounded shape:

- ROCB uses open-trade state transitions with:
  - ATR stop checks
  - channel re-entry / channel exit checks
  - max-hold checks
  - optional flip behavior
- VEMR uses open-trade state transitions with:
  - ATR stop checks
  - mean reversion / middle-band exits
  - max-hold checks
  - optional flip behavior
- MKCS has related compression plus breakout state, but it is the weakest fit of the three

The defensible RT-core question here is:

- can an RT-assisted earliest-hit helper answer "when does the path first intersect this moving exit boundary?" fast enough to matter for CPU strategy loops or validation-side reruns?

Why this is weaker than the primary target:

- moving barriers change every bar
- some exit conditions are not pure geometric intersections
- strategy-local state machines still own flip, setup, and hold logic
- replacing dedicated CUDA kernels is a much higher bar than accelerating a CPU fallback niche

Therefore:

- ROCB is the first moving-barrier candidate worth testing
- VEMR is second
- MKCS is optional only if the same helper shape already works elsewhere
- this path should not be attempted before the primary iterative-risk experiment is measured

## Non-Goals

This plan does not authorize:

- any change to the shipped optimization route policy
- any change to `gpu_backtest_kernel`
- any frontend or request-surface work beyond hidden experiment plumbing
- making OptiX a normal build requirement
- breaking CPU-only builds
- widening CUDA/PTX packaging for release builds
- replacing the authoritative intrabar tie-break rules
- broad refactors of validation architecture before evidence exists

## Working Hypothesis

The working hypothesis is narrow:

- on risk-enabled iterative validation workloads, a meaningful amount of CPU time may be spent repeatedly scanning future bars to determine the earliest stop-loss or take-profit reachability against the same OHLC history
- if that scan cost is material, then a cached RT-core scene plus batched barrier queries may reduce the cost of that specific subproblem
- if that scan cost is not material, the RT-core direction is not worth carrying forward

This plan therefore has an explicit **measurement gate before implementation**.

## Viability Bar

The experiment should only be considered viable if all of the following are true:

- exact parity is preserved for hit-bar decisions and final metrics on targeted validation surfaces
- CPU-only builds remain unaffected when the experiment feature is off
- the scene is reused per dataset or resample; a rebuild-per-call result does not count
- at least one real repo truth surface shows:
  - at least `25%` improvement in the targeted iterative-risk substage after amortizing scene build cost
  - and at least `10%` end-to-end validation wall-time improvement
- the gain survives on a workload that matters to the repo, not just a synthetic query flood

Immediate no-go conditions:

- risk-enabled iterative validation is only marginally slower than the same workload without risk overlays
- the RT path only wins when the scene build cost is excluded
- parity depends on weakening intrabar semantics
- performance only improves on an isolated microbenchmark

## Truth Surfaces

Primary truth surfaces for this experiment:

1. Double-MA fixed-candidate validation
   - `Backend::CpuOnly`
   - `ValidationEngine::Iterative`
   - `stop_loss_pct` and `take_profit_pct` enabled
   - at least `16` to `32` candidates on the same dataset

2. Composite fixed-candidate validation
   - `Backend::CpuOnly`
   - `ValidationEngine::Iterative`
   - one representative composite family first
   - risk overlays enabled

3. Meta heavy-holdout replay as a secondary consumer
   - only after the base validation path proves useful

Secondary optional truth surfaces for the moving-barrier follow-on:

4. ROCB strategy-local CPU loop
   - `crates/app_core/src/rocb.rs`
   - earliest exit against channel and ATR-derived moving barriers

5. VEMR strategy-local CPU loop
   - `crates/app_core/src/vemr.rs`
   - earliest exit against middle-band and ATR-derived moving barriers

6. MKCS only if needed later
   - `crates/app_core/src/mkcs.rs`
   - not a first follow-on target

Control surfaces:

1. Same validation workloads with `ValidationEngine::Vectorized`
2. Same iterative workloads with no stop-loss / take-profit configured
3. Small-dataset controls where scene build cost should dominate and likely kill the idea

## Semantic Constraints

The RT path may only act as a broad-phase earliest-hit helper.

It must not redefine:

- `IntrabarRule::{Pessimistic, Optimistic, StopFirst, TakeFirst}`
- long vs short stop/take semantics
- entry price rules
- trade close precedence when a signal exit or flip happens before the barrier hit
- overlay behavior such as cooldown and persistence

Authoritative behavior must remain grounded in the existing CPU semantics in:

- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/asr.rs`

The strict semantic rule is:

- RT cores may help answer "what is the earliest future bar that can hit this barrier?"
- CPU logic must still decide "what fill is valid under the repo's current intrabar and exit-precedence rules?"

For the moving-barrier follow-on, the same rule applies:

- RT cores may help answer "what is the earliest bar where this evolving exit boundary could be crossed?"
- strategy-local CPU logic must still decide:
  - flip precedence
  - entry and setup persistence
  - max-hold handling
  - regime-specific exit validity

## Architecture Direction

If the experiment advances past the measurement gate, the implementation shape should be:

1. Feature-gated native helper in `ta_app_core`
2. Dataset-scoped scene cache keyed by:
   - dataset identity
   - timeframe or resample identity
   - low/high content fingerprint or equivalent cache identity
   - selected CUDA device
3. Validation-only integration first
4. Double-MA first, composite second
5. Hidden experiment activation first, never default-on

The intended Rust-side shape is:

- `crates/app_core/src/rtcore_exit_scan.rs`
- `crates/app_core/native/rtcore_barrier/*`
- validation-side helper wiring in:
  - `crates/app_core/src/validation_pipeline.rs`
  - `crates/app_core/src/iterative_sim.rs`
  - `crates/app_core/src/double_ma_validate.rs`
  - later, conditionally, `crates/app_core/src/composite_validate.rs`

The intended build shape is:

- new cargo feature, for example `rtcore-barrier-experiment`
- feature depends on existing CUDA features instead of bypassing them
- `build.rs` only attempts the OptiX native build when the experiment feature is enabled
- `OPTIX_HOME` remains required only for that feature, never for normal CPU or standard CUDA builds

## Phase 0: Baseline Measurement Gate

Goal: prove that the current risk-overlay scan cost is material enough to justify any OptiX work.

- [ ] Confirm no benchmark or profiling process is already active.
- [ ] Use an existing headless validation tool or add a narrow temporary probe binary under `crates/app_core/src/bin/` or `src-tauri/src/bin/`.
- [ ] Measure Double-MA iterative validation on the same dataset:
  - [ ] no risk overlays
  - [ ] stop-loss only
  - [ ] take-profit only
  - [ ] stop-loss + take-profit
- [ ] Repeat on at least one composite validation workload.
- [ ] Record:
  - [ ] candidate count
  - [ ] bars
  - [ ] validation wall time
  - [ ] iterative-only wall time if separately measurable
- [ ] Add narrow instrumentation around the current barrier-hit check path if needed.
- [ ] Abort the RT-core direction immediately if risk-overlay overhead is too small to matter.

Phase-0 decision gate:

- [ ] Proceed only if risk-enabled iterative validation shows clear, material overhead attributable to barrier scanning.

## Phase 1: Out-Of-Tree Native Smoke

Goal: validate the RT-core helper idea without touching active repo behavior.

- [ ] Keep the top-level `rt_cores/` material out of the live build.
- [ ] Build the native helper out-of-tree with explicit OptiX tooling.
- [ ] Fix obvious repo mismatches in the spike only:
  - [ ] crate naming
  - [ ] device selection
  - [ ] scene lifecycle assumptions
- [ ] Prove scene build and batched query correctness on synthetic data.
- [ ] Confirm the helper can return earliest stop and target hit bars separately.
- [ ] Confirm the helper can report "no hit" correctly.

Phase-1 decision gate:

- [ ] Stop if the native helper cannot be made stable or portable enough even as an experiment.

## Phase 2: Repo-Gated Scaffolding

Goal: introduce a compile-time experiment surface without affecting normal builds.

- [ ] Add a new `ta_app_core` feature, for example `rtcore-barrier-experiment`.
- [ ] Keep the feature off by default.
- [ ] Make the build helper run only when:
  - [ ] the experiment feature is enabled
  - [ ] and `OPTIX_HOME` is set
- [ ] Keep normal CPU and standard CUDA builds unaffected.
- [ ] Add the Rust FFI wrapper module under `crates/app_core/src/`.
- [ ] Add native helper sources under `crates/app_core/native/rtcore_barrier/`.
- [ ] Add a hidden experiment runtime flag only after compile-time gating is correct.

Phase-2 decision gate:

- [ ] Stop if the repo cannot support a clean, isolated feature gate.

## Phase 3: Double-MA Validation Prototype

Goal: test the smallest real repo use case first.

- [ ] Restrict the first in-repo prototype to `run_double_ma_validation_blocking_with_candles(...)`.
- [ ] Restrict it further to:
  - [ ] `Backend::CpuOnly`
  - [ ] or GPU fallback to CPU
  - [ ] `ValidationEngine::Iterative`
  - [ ] risk overlays enabled
- [ ] Do not change vectorized validation.
- [ ] Do not touch `gpu_backtest_kernel`.
- [ ] Add a dataset-scoped scene cache.
- [ ] Batch stop and target queries across candidates that share the same dataset.
- [ ] Preserve CPU authority for:
  - [ ] same-bar stop/target tie handling
  - [ ] signal exit precedence
  - [ ] fill-price calculation
- [ ] Add targeted tests proving parity against the current CPU path.

Phase-3 decision gate:

- [ ] Promote only if Double-MA shows a real repo win under the viability bar.

## Phase 4: Composite Validation Follow-On

Goal: extend only if the first real use case wins.

- [ ] Reuse the same validation-only helper shape.
- [ ] Limit the first composite experiment to CPU iterative validation.
- [ ] Do not widen to every composite family at once.
- [ ] Start with one representative family and one plain composite surface.
- [ ] Re-run the same parity and wall-time checks.

Phase-4 decision gate:

- [ ] Stop if the Double-MA win does not generalize at least to one composite workload.

## Phase 5: Meta Consumer Check

Goal: decide whether there is a second real consumer in the repo.

- [ ] Inspect heavy-holdout reruns in `src-tauri/src/commands_meta.rs`.
- [ ] Confirm that repeated grouped candidate validation can reuse the same scene.
- [ ] Measure end-to-end impact with the RT experiment active.
- [ ] Keep this phase optional.

Phase-5 decision gate:

- [ ] Do not widen into meta-specific plumbing unless the base validation experiment already won.

## Phase 6: Optional Moving-Barrier Follow-On

Goal: test whether the same RT traversal idea can help strategy-local moving-barrier exits outside the stop/take validation lane.

- [ ] Attempt this phase only if the primary iterative-risk experiment has already shown a real niche win or at least produced a stable helper shape.
- [ ] Start with `ROCB` in `crates/app_core/src/rocb.rs`.
- [ ] Measure the current CPU cost of the ROCB open-trade exit loop before writing RT integration code.
- [ ] Treat the RT helper as an earliest moving-boundary hit finder only.
- [ ] Keep ROCB CPU logic authoritative for:
  - [ ] flip handling
  - [ ] channel breakout entry rules
  - [ ] max-hold exits
  - [ ] final state transitions
- [ ] Try `VEMR` only if ROCB shows a real gain.
- [ ] Keep `MKCS` explicitly optional; do not fold it in by default.
- [ ] Do not replace or disturb the existing strategy-local GPU kernels in the first retained branch.
- [ ] Measure against the current CPU path first; dedicated GPU-kernel replacement is a separate and higher bar.

Phase-6 decision gate:

- [ ] Stop unless ROCB or VEMR shows a real CPU-path win without forcing a broad strategy-architecture rewrite.

## Verification Plan

Required validation categories:

- [ ] parity tests for hit bars and final metrics
- [ ] CPU-only build smoke with the feature off
- [ ] CUDA build smoke with standard features but RT feature off
- [ ] RT experiment build smoke with the feature on and OptiX present
- [ ] targeted validation regression tests in `ta_app_core`
- [ ] repeated wall-time measurements on real validation workloads

Preferred command surfaces:

- [ ] targeted `cargo test -p ta_app_core ...`
- [ ] `cargo test --workspace` after any retained integration
- [ ] `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests` for broad retained code changes
- [ ] a dedicated headless validation-cost or experiment binary for repeated timing

## Implementation Checklist

Use this as the consolidated go/no-go tracker.

- [ ] Measure current iterative-risk overhead before writing OptiX integration code.
- [ ] Prove the barrier-scan cost is material on at least one real repo workload.
- [ ] Keep the experiment feature-gated and off by default.
- [ ] Keep CPU-only builds unaffected.
- [ ] Keep the experiment out of `gpu_backtest_kernel`.
- [ ] Keep the RT path validation-only in the first retained branch.
- [ ] Use dataset-scoped scene reuse instead of per-call scene rebuilds.
- [ ] Preserve current intrabar semantics exactly.
- [ ] Prove exact parity on targeted validation surfaces.
- [ ] Prove at least one real end-to-end validation win after amortizing scene build cost.
- [ ] If the primary path wins, test ROCB as the first moving-barrier follow-on.
- [ ] Keep VEMR second and MKCS optional.
- [ ] Do not treat dedicated strategy GPU kernels as the first replacement target.
- [ ] Reject the experiment if only the microbenchmark wins.
- [ ] Reject the experiment if the gain depends on semantic drift.
- [ ] Reject the experiment if the implementation cannot stay isolated from standard builds.

## Promotion Policy

This experiment should be promoted only in this order:

1. measurement evidence
2. isolated feature-gated prototype
3. Double-MA validation niche
4. one composite validation niche
5. optional meta consumer
6. optional ROCB moving-barrier follow-on
7. optional VEMR follow-on

It should never be promoted in this order:

1. imported patch bundle
2. generic GPU kernel integration
3. direct replacement of existing strategy-local GPU kernels
4. release-facing default enablement

## Final Decision Rule

If the experiment clears the measurement gate, keeps repo semantics intact, and produces a real validation win on a shared-history candidate batch, it is worth retaining as a niche repo feature.

If the primary path wins and a later ROCB or VEMR moving-barrier test also wins, that should still be treated as an optional follow-on, not as proof that RT cores belong in the general GPU route policy.

If it does not do all three, the correct outcome is:

- document the result
- keep the review artifacts
- remove the code

That is still a successful experiment.
