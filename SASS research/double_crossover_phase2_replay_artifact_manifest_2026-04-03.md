# `double_crossover` Phase 2 Replay Artifact Manifest

Date: 2026-04-03

## Purpose

Record the Phase A0 preconditions, toolchain state, and retained validation artifacts for the replay-dataflow follow-on branch.

This manifest is the reproducible starting point for the next replay micro-attribution pass. It is intentionally broader than a code-change log so the branch can support product work and research work at the same time.

## Repo State

- repo root: `C:\Rust Projects\ta_desktop_demo`
- recorded commit: `71bf00055125931c2662fbd1882079dac221f08e`
- active branch state: local worktree with retained replay batching and replay-dataflow follow-on planning in progress

## Hardware And Toolchain

- GPU: `NVIDIA GeForce RTX 4090`
- NVIDIA driver: `595.71`
- CUDA compiler: `release 13.0, V13.0.88`
- Nsight Compute CLI: `2026.1.0.0`
- Nsight Systems CLI: not available on `PATH` in the current shell

## Process Hygiene

Checked before continuing Phase A work:

- no active `cargo`, `profile_gpu_dynamic_runner`, `ncu`, `nsys`, `nv-nsight-cu-cli`, or `nsys-ui` processes were found in the current shell session

## Compile Verification

Successful compile pass:

```powershell
cargo build -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel
```

Result:

- pass
- the new replay descriptor prep and replay descriptor upload timing fields compile cleanly through `gpu_backtest_kernel.rs`, `double_ma.rs`, and `profile_gpu_dynamic_runner.rs`

## Targeted Replay-Path Tests

Successful targeted tests:

```powershell
cargo test -p ta_app_core prepare_batched_windowed_exact_replay_rows_preserves_range_major_order --lib --release --features cuda,cuda-backtest-kernel -- --nocapture
```

```powershell
cargo test -p ta_app_core gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics --lib --release --features cuda,cuda-backtest-kernel -- --nocapture
```

```powershell
cargo test -p ta_app_core gpu_kernel_regime_hybrid_trade22_multi_range_eval_preserves_trade_tail_metrics --lib --release --features cuda,cuda-backtest-kernel -- --nocapture
```

Results:

- all three passed
- no replay ordering or `trade22` tail-metric regression was introduced by the new timing-field plumbing

## Retained Branch Inputs

Existing retained replay branch artifacts that remain authoritative for this follow-on:

- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_shortlist_slice_attribution_log_2026-04-03.md`
- `SASS research/double_crossover_phase2_windowed_replay_kernel_study_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_dataflow_architecture_plan_2026-04-03.md`
- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`

## Newly Surfaced Measurement Fields

The replay follow-on branch now has runner-visible or pipeline-visible fields for:

- shortlist replay total
- shortlist replay descriptor prep
- shortlist replay descriptor upload
- shortlist replay record assembly
- shortlist replay sort
- shortlist replay diagnostics

These fields are surfaced through:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

## Next Planned Artifact Outputs

The next active outputs for this branch are:

- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_branch_decision_2026-04-03.md`

Expected supporting run artifacts:

- shortlist-heavy runner JSONL outputs under `target/`
- replay dispatch logs under `target/`
- any targeted profiler outputs under `target/nsight/`

## Open Notes

- The current shell can invoke `ncu` directly, but `nsys` is not available on `PATH`. If a targeted Nsight Systems refresh is needed, use the repo’s existing wrapper path or the known local Nsight Systems installation path rather than assuming `nsys` resolves directly.
- This manifest intentionally does not claim Phase A measurement is complete. It only establishes that the next branch now has a stable compile baseline, targeted test baseline, and a concrete checklist.
