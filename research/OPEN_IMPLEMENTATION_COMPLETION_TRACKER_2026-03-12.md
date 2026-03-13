# Open Implementation Completion Tracker

Date: March 12, 2026

Purpose: provide one umbrella checklist for the remaining open implementation work across GPU execution, validation coverage, strategy expansion follow-through, profiling-driven optimization, and validation hardening.

This document is a working execution tracker. It does not replace the detailed source documents below. It exists to make the remaining backlog explicit and to define what evidence is required before any item can be marked complete.

## Source Documents

Primary status tracker:

- [GPU_VALIDATION_MIGRATION_PLAN_2026-03-10.md](./GPU_VALIDATION_MIGRATION_PLAN_2026-03-10.md)

Primary architecture / route policy:

- [../AGENTS.md](../AGENTS.md)

Supporting validation-policy tracker:

- [VALIDATION_HARDENING_PLAN_2026-03-10.md](./VALIDATION_HARDENING_PLAN_2026-03-10.md)

Supporting strategy / indicator inventory:

- [STRATEGY_INDICATOR_GPU_COVERAGE_INVENTORY_2026-03-10.md](./STRATEGY_INDICATOR_GPU_COVERAGE_INVENTORY_2026-03-10.md)

Separate demo / benchmark plan:

- [VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_PLAN_2026-03-11.md](./VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_PLAN_2026-03-11.md)

## Closure Rules

Do not mark a task complete unless all applicable evidence exists.

Shared implementation evidence:

- [ ] Code path exists in the intended backend/frontend files.
- [ ] CPU fallback behavior is explicit and tested when relevant.
- [ ] Route metadata is preserved or surfaced where user-visible.
- [ ] The relevant source tracker is updated to match the new state.

Shared Rust verification evidence:

- [ ] `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- [ ] relevant targeted `cargo test` coverage for the changed strategy / validation path
- [ ] `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel` when command-layer or frontend-facing Rust surfaces change

Shared frontend verification evidence:

- [ ] `npm --prefix ./web run typecheck` when frontend capability/request surfaces change
- [ ] relevant `vitest` coverage when request builder / capability summaries change

Shared profiling evidence for performance tasks:

- [ ] benchmark harness exists or is updated
- [ ] route metadata is visible in benchmark output
- [ ] Nsight artifact path is recorded in the detailed tracker
- [ ] optimization conclusion is written down, even if the conclusion is "do not optimize further"

## Current Snapshot

Already materially complete:

- `DoubleMa` GPU optimization and validation
- `CompositeTrend`, `BreakoutMomentum`, `RegimeHybrid` routed GPU / hybrid optimization and validation on the `DoubleMa` family
- composite fixed-candidate GPU validation for `ASR`, `ROCB`, `VEMR`, `MKCS`, and parity-gated `VCC`
- bounded base `CompositeCpu` `GpuKernel` optimization and fixed-candidate validation
- GPU-native optimization track for `KQSR`, `CARM`, `KVDF`, `MKCS` v1, and `CDMC`
- representative parity tests and benchmark / Nsight evidence for those GPU-native optimization routes

Still open at umbrella level:

- deferred `MKCS` indicator expansion
- remaining validation-hardening tasks
- targeted upstream staging / indicator bottlenecks found by Nsight
- optional public benchmark/demo execution work

## Phase U1: First-Wave Indicator End-to-End Coverage

- [x] Verify all selected first-wave indicators remain GPU-executable end to end on the current `vector-ta` version.
  Current implementation note:
  - strategy-local CUDA smoke coverage now exists in source for:
    - `KQSR`
    - `CARM`
    - `KVDF`
    - `CDMC`
    - `MKCS` v1
  - verification passed in this session with:
    - `cargo test -p ta_app_core gpu_smoke_runs_with_ --features cuda,cuda-backtest-kernel`
  Indicators to explicitly verify:
  - `KQSR`: `kdj`, `srsi`, `qqe`, `ift_rsi`, `prb`
  - `CARM`: `coppock`, `ao`, `rocr`, `apo`, `otto`
  - `KVDF`: `keltner`, `vwap`, `dti`, `devstop`, `fvg_trailing_stop`
  - `CDMC`: `dpo`, `correlation_cycle`, `mama`, `decycler`
  - `MKCS` v1: `stddev`, `donchian`
  Completion evidence:
  - [x] adapter or strategy-local smoke tests for each required output surface
  - [x] explicit note recorded for any remaining host-assembled matrix staging
  - [x] [GPU_VALIDATION_MIGRATION_PLAN_2026-03-10.md](./GPU_VALIDATION_MIGRATION_PLAN_2026-03-10.md) updated if any indicator status changed
  Remaining staging note:
  - `KQSR`, `CARM`, and `KVDF` still use strategy-local compact host row-major uploads for some split or mixed indicator surfaces before device-side scoring.
  - `MKCS` v1 continues to use parity-safe host-row staging for its `stddev` and `donchian` sweeps before GPU metric evaluation.
  - `CDMC` keeps its sweep/device outputs VRAM-resident in the strategy-local validation path.

## Phase U2: GPU Validation for GPU-Native Expansion Strategies

Goal: bring validation-stage GPU execution to the new GPU-native strategies instead of stopping at optimization-only GPU execution.

Current implementation note:
- source now carries GPU validation routing for `KQSR`, `CARM`, `KVDF`, and `CDMC` across:
  - in-sample
  - `walk_forward`
  - `CSCV/PBO`
  - `SPA`
  - `reopt_walk_forward`
  - `trade_bootstrap`
  - `perturb`
  - `cross_ticker` via backend-preserving command-layer reruns
- targeted source tests were added for advanced validation smoke coverage in `crates/app_core/src/composite_validate.rs`
- command-layer source smoke coverage exists in `src-tauri/src/commands_validation.rs`
- verification passed in this session with:
  - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
  - `cargo test -p ta_app_core validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel`
  - `cargo test -p ta_app_core advanced_routes_smoke --features cuda,cuda-backtest-kernel`
  - `cargo test -p ta_desktop_demo_app cross_ticker_ --features cuda,cuda-backtest-kernel`
  - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
  - `npm --prefix ./web run typecheck`
  - `npm --prefix ./web run test -- validationRequestBuilder.test.ts`

- [x] Implement validation-stage GPU support for `KQSR`.
  Target validation surface:
  - [x] in-sample
  - [x] `walk_forward`
  - [x] `CSCV/PBO`
  - [x] `SPA`
  - [x] `reopt_walk_forward`
  - [x] `trade_bootstrap`
  - [x] `perturb`
  - [x] `cross_ticker` via backend-preserving command-layer reruns
  Completion evidence:
  - [x] targeted validation tests
  - [x] command-layer smoke coverage if `cross_ticker` is implemented
  - [x] route metadata captured

- [x] Implement validation-stage GPU support for `CARM`.
  Target validation surface:
  - [x] in-sample
  - [x] `walk_forward`
  - [x] `CSCV/PBO`
  - [x] `SPA`
  - [x] `reopt_walk_forward`
  - [x] `trade_bootstrap`
  - [x] `perturb`
  - [x] `cross_ticker` via backend-preserving command-layer reruns
  Completion evidence:
  - [x] targeted validation tests
  - [x] command-layer smoke coverage if `cross_ticker` is implemented
  - [x] route metadata captured

- [x] Implement validation-stage GPU support for `KVDF`.
  Target validation surface:
  - [x] in-sample
  - [x] `walk_forward`
  - [x] `CSCV/PBO`
  - [x] `SPA`
  - [x] `reopt_walk_forward`
  - [x] `trade_bootstrap`
  - [x] `perturb`
  - [x] `cross_ticker` via backend-preserving command-layer reruns
  Completion evidence:
  - [x] targeted validation tests
  - [x] command-layer smoke coverage if `cross_ticker` is implemented
  - [x] route metadata captured

- [x] Implement validation-stage GPU support for `CDMC`.
  Target validation surface:
  - [x] in-sample
  - [x] `walk_forward`
  - [x] `CSCV/PBO`
  - [x] `SPA`
  - [x] `reopt_walk_forward`
  - [x] `trade_bootstrap`
  - [x] `perturb`
  - [x] `cross_ticker` via backend-preserving command-layer reruns
  Completion evidence:
  - [x] targeted validation tests
  - [x] command-layer smoke coverage if `cross_ticker` is implemented
  - [x] route metadata captured

## Phase U3: Composite / Validation Runtime Gaps

- [x] Resolve base `CompositeCpu` `cross_ticker` GPU validation.
  Current state note:
  - command-layer `cross_ticker` reruns already preserve the requested composite backend by rebuilding per-dataset validation requests with `cross_ticker: None` and rerunning `run_composite_validation_blocking_with_candles`
  Completion evidence:
  - [x] backend route implemented via backend-preserving command-layer reruns
  - [x] command-layer smoke coverage exists in `src-tauri/src/commands_validation.rs`
  - [x] tracker note updated

- [ ] Decide whether to widen bounded base composite GPU execution further or freeze the current bounded scope.
  Decision should cover:
  - [ ] leave current `HardGate` bounded route as final
  - [ ] or widen beyond current bounded optimization / validation contract
  Completion evidence:
  - [ ] decision written into the primary tracker
  - [ ] any resulting implementation work broken out into follow-up items

- [ ] Verify parity of hard validation gates for `CompositeMo`-backed validation path.
  Current source:
  - open item from [VALIDATION_HARDENING_PLAN_2026-03-10.md](./VALIDATION_HARDENING_PLAN_2026-03-10.md)
  Completion evidence:
  - [ ] parity tests
  - [ ] tracker updated

## Phase U4: MKCS Planned Extension

Current reality:

- `MKCS` v1 is implemented and GPU-backed
- planned extension indicators remain open:
  - `mass`
  - `kurtosis`
  - `correl_hl`

- [x] Decide whether the original `MKCS` extension remains in scope.
  Completion evidence:
  - [x] explicit decision recorded in the primary tracker
  Decision:
  - the current execution-completion scope is the shipped `MKCS` v1 (`stddev` + `donchian`) only
  - the deferred `mass` / `kurtosis` / `correl_hl` extension is explicitly out of scope for this GPU validation completion pass unless product scope is widened later

- [ ] If the product later re-opens the broader `MKCS` concept, implement the deferred `MKCS` extension indicators as a separate follow-up.
  Implementation checklist:
  - [ ] CPU baseline for `mass`
  - [ ] CPU baseline for `kurtosis`
  - [ ] CPU baseline for `correl_hl`
  - [ ] GPU staging for the deferred `MKCS` signals
  - [ ] `GpuKernel` optimization integration
  - [ ] parity tests
  - [ ] benchmark / route evidence

## Phase U5: Validation Hardening Remaining Tasks

Open items inherited from [VALIDATION_HARDENING_PLAN_2026-03-10.md](./VALIDATION_HARDENING_PLAN_2026-03-10.md):

- [ ] Add minimum local-stability score gate to hard validation gates.

- [ ] Finish benchmark policy decision.
  Remaining sub-item:
  - [ ] strategy-family baseline

- [ ] Extend SPA scope from validated finalists to the full tried-candidate universe when practical.

- [ ] Evaluate whether White Reality Check is still needed after SPA.

- [ ] Evaluate whether Model Confidence Set should be added for survivor-set output.

- [ ] Evaluate whether FDR control should be added for large rule universes.

- [ ] Add snapshot tests for validation result schema changes.

- [ ] Add one end-to-end workflow regression that fails when overfit candidates survive incorrectly.

## Phase U6: Profiling-Driven Backend Follow-Up

These are not all guaranteed implementation items. They started as the evidence-backed optimization queue after the March 12 benchmark-mode Nsight pass and now include the post-change profiling closeout for the implemented reductions.

- current implementation note:
  - the repo-local optimization pass is now implemented for all four remaining GPU-native follow-up items.
  - targeted verification passed in this session with:
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core kvdf_ --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core carm_ --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core kqsr_ --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core cdmc_ --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_desktop_demo_app cross_ticker_ --features cuda,cuda-backtest-kernel`
  - post-change profiling artifacts were captured in this session:
    - Nsight Systems reports:
      - `target/nsight/kqsr_gpu_small_post_u6.nsys-rep`
      - `target/nsight/carm_gpu_small_post_u6.nsys-rep`
      - `target/nsight/kvdf_gpu_small_post_u6.nsys-rep`
      - `target/nsight/cdmc_gpu_small_post_u6.nsys-rep`
    - Nsight Compute reports:
      - `target/nsight/kqsr_gpu_small_post_u6_ncu.ncu-rep`
      - `target/nsight/carm_gpu_small_post_u6_ncu.ncu-rep`
      - `target/nsight/kvdf_gpu_small_post_u6_ncu.ncu-rep`
      - `target/nsight/cdmc_gpu_small_post_u6_ncu2.ncu-rep`
  - environment note:
    - `nsys stats --report cuda_gpu_kern_sum` was not usable on the post-U6 `.nsys-rep` captures in this environment because the CLI reported no CUDA kernel data for those files, so the post-change hotspot summary below is taken from Nsight Compute imports instead.
  - perf-gate note:
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` failed against the stored baseline, but that gate benchmarks `double_ma/gpu_kernel/sma_sma/200k/small` in `scripts/run_gpu_perf_gate.ps1`, not the `gpu_native_expansion_opt` harness used by `KQSR`, `CARM`, `KVDF`, and `CDMC`.
    - the generated summary lives at `target/gpu_perf_summary.json`; treat it as a separate `DoubleMa` benchmark investigation rather than evidence of a regression in the U6 strategy-local changes.

- [x] `CDMC`: investigate and, if viable, optimize `mama_inv_dp_f32`.
  Reason:
  - current best single concentrated hotspot among the new GPU-native strategies
  Completion evidence:
  - [x] repo-local duplicate `MAMA/FAMA` computation removed in `cdmc.rs`, `composite_cpu.rs`, and `composite_validate.rs`
  - [x] backend notes recorded in primary tracker
  Current implementation note:
  - `CDMC` no longer asks the CUDA dispatch path to compute `mama` and `fama` separately for optimization/validation staging.
  - `cdmc.rs` now computes the `MAMA/FAMA` pair in one `CudaMama` pass and reuses both device buffers.
  - the shared `compute_cdmc_signals(...)` helper now computes `MAMA/FAMA` once per request instead of running two identical passes.
  - post-change Nsight Compute summary from `target/nsight/cdmc_gpu_small_post_u6_ncu2.ncu-rep`:
    - `correlation_cycle_batch_f32_ria`: `1644.65 us` total (`62.76%`)
    - `mama_inv_dp_f32`: `391.71 us` total (`14.95%`)
    - `correlation_cycle_state_batch_f32`: `306.02 us` total (`11.68%`)
    - `cdmc_backtest_metrics_f32`: `186.16 us` total (`7.10%`)
  - conclusion:
    - the duplicated repo-local `MAMA/FAMA` staging work is removed and `mama_inv_dp_f32` is no longer the dominant device-side hotspot in the captured post-change run.

- [x] `CARM`: investigate and, if viable, reduce `OTTO` staging cost.
  Reason:
  - `otto_batch_f32` dominates the current benchmark trace
  Completion evidence:
  - [x] repo-local `OTTO` staging reduction implemented in `carm_gpu_kernel.rs`, `composite_cpu.rs`, `composite_validate.rs`, and `double_crossover.cu`
  - [x] backend notes recorded in primary tracker
  Current implementation note:
  - `CARM` now dedupes identical `OTTO` parameter tuples before upload and threads an explicit combo-side `otto_row_index` into the kernel path.
  - mixed `Coppock` / `AO` / `APO` rows remain combo-shaped, but `OTTO` no longer pays one row per combo when its parameters repeat.
  - post-change Nsight Compute summary from `target/nsight/carm_gpu_small_post_u6_ncu.ncu-rep`:
    - `otto_batch_f32`: `758.35 us` total (`62.31%`)
    - `ao_build_prefix_dsf_serial_f32`: `209.31 us` total (`17.20%`)
    - `rocr_batch_f32`: `77.43 us` total (`6.36%`)
    - `coppock_batch_time_parallel_f32`: `55.53 us` total (`4.56%`)
  - conclusion:
    - the repo-local dedupe is in place, but `OTTO` remains the primary device-side hotspot, so any further speed win here is likely upstream indicator work rather than another scorer-path change.

- [x] `KVDF`: investigate upstream indicator-side reductions around `DTI`, `DevStop`, and `FVG` staging.
  Reason:
  - scorer loop is no longer the primary bottleneck
  Completion evidence:
  - [x] repo-local `DTI` staging reduction implemented in `kvdf_gpu_kernel.rs`, `composite_cpu.rs`, `composite_validate.rs`, and `double_crossover.cu`
  - [x] backend notes recorded in primary tracker
  Current implementation note:
  - the repo-local reduction pass is now implemented for the `DTI` portion of the staging path.
  - `kvdf_gpu_kernel.rs` and `double_crossover.cu` now accept an explicit combo-side `DTI` row index instead of forcing one `DTI` row per combo.
  - `composite_cpu.rs` and `composite_validate.rs` now dedupe identical `(dti_r, dti_s, dti_u)` triples before computing/uploading `DTI` rows.
  - for the current benchmark-grid shape in `crates/app_core/benches/gpu_native_expansion_opt.rs` (`fast 5..20`, `slow 20..80`), that reduces the `DTI` row set from `975` combos to `540` unique `DTI` rows, or about `44.6%` fewer `DTI` rows to compute/upload.
  - post-change Nsight Compute summary from `target/nsight/kvdf_gpu_small_post_u6_ncu.ncu-rep`:
    - `fvg_trailing_stop_batch_small_shmem_f32`: `4967.95 us` total (`76.07%`)
    - `devstop_batch_grouped_f32`: `1057.76 us` total (`16.20%`)
    - `devstop_build_range_prefixes_f32`: `281.51 us` total (`4.31%`)
    - `atr_batch_unified_f32`: `89.09 us` total (`1.36%`)
  - conclusion:
    - `DTI` no longer appears in the captured top-duration kernel set; the remaining device-side time is now dominated by `FVG` and `DevStop`.

- [x] `KQSR`: investigate whether staging / transfer volume can be reduced further.
  Reason:
  - current trace is still dominated by `PRB`, `sRSI`, and `QQE` staging rather than final scoring
  Completion evidence:
  - [x] repo-local staging reduction implemented in `kqsr.rs`, `composite_cpu.rs`, and `composite_validate.rs`
  - [x] backend notes recorded in primary tracker
  Current implementation note:
  - `KQSR` now splits reusable fast-side and slow-side signal staging and keeps those GPU-route staging buffers in `f32` form.
  - the route no longer materializes intermediate `Vec<f64>` rows only to immediately re-convert them before upload.
  - this reduces host allocation/conversion overhead without changing device-side metrics semantics.
  - post-change Nsight Compute summary from `target/nsight/kqsr_gpu_small_post_u6_ncu.ncu-rep`:
    - `srsi_build_rsi_f32`: `879.75 us` total (`71.11%`)
    - `willr_build_sparse_level_f32`: `211.59 us` total (`17.10%`)
    - `ift_rsi_batch_f32`: `49.56 us` total (`4.01%`)
    - `srsi_fk_batch_f32`: `22.52 us` total (`1.82%`)
  - conclusion:
    - the device-side mix is still dominated by `sRSI`; the implemented `f32` host-staging reduction is a host allocation / conversion win and is not expected to move the device-only Nsight Compute distribution very much.

## Phase U7: Optional Benchmark / Demo Work

These items are useful, but they are not blockers for the core GPU backend / validation roadmap.

- [x] Freeze the exact VectorGrid vs `vectorbt` faceoff scenario parameters.
- [x] Freeze the dataset used by both tools.
- [x] Add or verify an app-side reproducible runner for the scenario.
- [x] Build the equivalent `vectorbt` exact-grid script.
- [x] Confirm result parity or document any semantic mismatch.
- [x] Run repeated timed trials and record median / min.
- [ ] Capture the side-by-side optimization run.
- [ ] Capture the result-parity sanity check.
- [x] Write a short benchmark summary table for publication or presentation.

Completion evidence:

- [x] frozen scenario file added at `scripts/vectorbt_double_ma_faceoff_scenario.json`
- [x] shared deterministic dataset generator added at `scripts/generate_double_ma_faceoff_dataset.py`
- [x] repeated faceoff runner added at `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- [x] VectorBT-side exact-grid script updated at `scripts/vectorbt_double_ma_faceoff.py`
- [x] Python environment lock snapshot recorded at `scripts/requirements-vectorbt-faceoff.lock.txt`
- [x] repeated-trial summary written to `target/vectorbt_vs_vectorgrid_faceoff/summary.json`
- [x] repeated-trial Markdown table written to `target/vectorbt_vs_vectorgrid_faceoff/summary.md`
- [x] app benchmark anchor written to `target/vectorgrid_double_ma_faceoff.json`
- [x] capture runbook written to `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_CAPTURE_RUNBOOK_2026-03-12.md`

Current implementation note:

- The faceoff scenario is now frozen to `alma_alma`, `200,000` bars, `58,300` valid pairs, objective `Sharpe`, and a shared deterministic CSV dataset.
- The repeated measured run on `2026-03-12` produced median wall-clock times of:
  - app `GpuKernel`: `1,831.014 ms`
  - app CPU: `4,487.878 ms`
  - VectorBT-side CPU workflow: `90,745.661 ms`
- Best parameters matched exactly across app CPU, app GPU kernel, and the VectorBT-side workflow: `fast=5`, `slow=10`.
- Objective values were materially close but not identical, so parity is documented as best-parameter parity with small metric deltas rather than bit-identical metric parity.
- The actual side-by-side MP4 capture remains manual, but the repo now contains the frozen inputs, measured summary, and explicit capture runbook needed to record it without changing the methodology.

## Suggested Execution Order

Recommended next sequence from the current repo state:

1. end-to-end verification of first-wave indicator GPU executability
2. validation-stage GPU support for the new GPU-native strategies
3. base composite `cross_ticker` GPU validation
4. deferred `MKCS` scope decision
5. optional follow-up on the unrelated `DoubleMa` GPU perf-gate baseline drift
6. remaining validation-hardening items

## Closeout Notes

When a task is completed:

- update this umbrella tracker
- update the underlying detailed tracker
- record exact verification commands
- record exact artifact paths for any benchmark / Nsight evidence
- note any intentionally deferred follow-up instead of silently dropping it
