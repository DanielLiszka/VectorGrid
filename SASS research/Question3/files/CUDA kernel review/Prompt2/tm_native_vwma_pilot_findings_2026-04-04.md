# TM-Native VWMA Pilot Findings

Date: 2026-04-04

## Purpose

This memo records the retained outcome of the bounded Prompt2 implementation branch:

- add TM-native VWMA output support without broad `vector-ta` redesign
- admit only the `vwma/vwma` fast-path pilot in the repo runtime
- prove parity against the existing row-major-plus-transpose fallback
- verify current fallback and benchmark sentinels remain healthy

## Branch Decision

Retain the branch.

The pilot cleared the bounded gate for a backend-local `vwma/vwma` admission change. It did **not** justify broader TM-native MA expansion.

## Stage 0 Result

Stage 0 outcome: `small external addition required`.

Current pinned `vector-ta 0.2.4` did not already expose TM-native VWMA output through `compute_period_ma_tm_into(...)`. The missing capability was added surgically in the vendored workspace patch rather than by redesigning the selector/runtime surface.

Repo-owned wiring:

- [Cargo.toml](C:/Rust%20Projects/ta_desktop_demo/Cargo.toml)
- [Cargo.lock](C:/Rust%20Projects/ta_desktop_demo/Cargo.lock)

Vendored dependency surfaces:

- [vwma_kernel.cu](C:/Rust%20Projects/ta_desktop_demo/vendor/vector-ta-0.2.4/kernels/cuda/moving_averages/vwma_kernel.cu)
- [vwma_wrapper.rs](C:/Rust%20Projects/ta_desktop_demo/vendor/vector-ta-0.2.4/src/cuda/moving_averages/vwma_wrapper.rs)
- [vram_ma.rs](C:/Rust%20Projects/ta_desktop_demo/vendor/vector-ta-0.2.4/src/cuda/moving_averages/vram_ma.rs)

The vendored patch adds:

- a TM-layout VWMA batch kernel writer
- a TM-layout VWMA device wrapper
- a `"vwma"` arm for `compute_period_ma_tm_into(...)`

The change stayed VWMA-only.

Post-review hardening kept the same bounded scope:

- the TM-output VWMA kernel writer now uses `size_t` indexing for TM output addressing so large `time_len * total_combos` products do not silently overflow a 32-bit store index
- the deterministic `VFCT` parity test now asserts that the retained pilot actually executes on `backend_route = GpuKernel`

## Retained Repo Changes

Runtime admission stayed narrow:

- [gpu_backtest_kernel.rs](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs)

`kernel_fast_path_enabled(...)` still admits only:

- the existing `sma/alma` family pair
- the new bounded `vwma/vwma` pilot pair

No broader per-side capability framework was added.

Fallback/parity ownership:

- [double_ma.rs](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs)

Added tests:

- `gpu_vfct_vwma_fast_path_matches_fallback_small_grid`
- `gpu_vfct_vwma_fast_path_rejects_missing_volume`

No first-wave changes were made to:

- [double_crossover.cu](C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/double_crossover.cu)
- frontend/request-builder capability files

## Verification

### CPU / build sanity

Passed:

- `cargo check -p ta_app_core`

This confirmed the vendored `vector-ta` patch does not break the CPU-only build expectation for `ta_app_core`.

### Targeted correctness tests

Passed:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_vfct_vwma_fast_path_matches_fallback_small_grid -- --nocapture --test-threads=1`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_vfct_vwma_fast_path_rejects_missing_volume -- --nocapture --test-threads=1`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_hybrid_cpu_staged_kernel_capable_ma_stays_on_gpu_path -- --nocapture --test-threads=1`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_hybrid_vdps_cpu_staged_kernel_capable_ma_stays_on_gpu_path -- --nocapture --test-threads=1`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_precomputed_frama_matrix_executes -- --nocapture --test-threads=1`

What these prove:

- TM-native VWMA matches the existing fallback result on a deterministic `VFCT` surface
- missing-volume rejection remains intact
- hybrid/precomputed GPU fallback routes remain intact

### Perf gates

Passed:

- `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`

Artifacts:

- [gpu_perf_summary.json](C:/Rust%20Projects/ta_desktop_demo/target/gpu_perf_summary.json)
- [gpu_alma_contract_rows_perf_summary.json](C:/Rust%20Projects/ta_desktop_demo/target/gpu_alma_contract_rows_perf_summary.json)

Current sentinel means:

- `gpu_alma_large`: `70.349136 ms`
- `contract auto`: `65.816467 ms`
- `contract full17`: `69.042700 ms`
- `contract trade22`: `68.783495 ms`

### Full-package test caveat

`cargo test -p ta_app_core --features cuda,cuda-backtest-kernel` remains red in the current repo, but not because of this branch. The retain decision for this pilot therefore rests on:

- `cargo check -p ta_app_core`
- targeted CUDA parity/fallback tests
- the existing authoritative GPU perf gates

## Pilot Measurement

Benchmark artifacts:

- [double_ma_vwma_large_fastpath_off.jsonl](C:/Rust%20Projects/ta_desktop_demo/target/tm_native_vwma_pilot/double_ma_vwma_large_fastpath_off.jsonl)
- [double_ma_vwma_large_fastpath_on.jsonl](C:/Rust%20Projects/ta_desktop_demo/target/tm_native_vwma_pilot/double_ma_vwma_large_fastpath_on.jsonl)

Measured `DoubleMa`, `vwma/vwma`, exact-grid GPU route summary:

- fast path `off`
  - mean `route_wall_ms`: `100.33`
  - mean `wall_ms`: `100.33`
  - mean `gpu_indicator_prep_ms`: `30.00`
- fast path `on`
  - mean `route_wall_ms`: `96.33`
  - mean `wall_ms`: `96.33`
  - mean `gpu_indicator_prep_ms`: `30.00`

Observed movement:

- `route_wall_ms`: `-4.00 ms` (`~4.0%`)
- `gpu_indicator_prep_ms`: `0.00 ms`

Interpretation:

- the bounded `vwma/vwma` route-wall win remained present on the refreshed post-fix rerun
- the coarse `gpu_indicator_prep_ms` bucket did **not** retain a measurable mean reduction in that final rerun, so the original prep-time hypothesis is only partially supported

Both modes stayed on the intended GPU route:

- `backend_route = GpuKernel`
- `backend_used = Gpu`
- `route_diagnostics` stayed on `gpu_kernel.path=row_first_global_topk`

Both modes also produced the same result on the measured surface:

- best params: `fast_len=5`, `slow_len=10`
- best metrics:
  - `pnl = 223.69510113163247`
  - `sharpe = 0.0171482389353086`

## Scope Preserved

The retained branch remained additive:

- no feature/functionality removal
- no `double_crossover.cu` consumer rewrite
- no frontend/request-builder changes
- no broad TM-native MA rollout
- no fallback removal for unsupported families

Fallback behavior remains:

- row-major compute plus `transpose_row_to_tm` for unsupported families
- hybrid/precomputed CPU-staged routes

## Limitations

The measured payoff surface retained here is the plain `DoubleMa vwma/vwma` GPU route. There is no standalone `VFCT` runner benchmark artifact in this branch. `VFCT` coverage is currently provided by the targeted parity/error-path tests rather than by a separate runner timing artifact.

That is sufficient for the bounded retain decision, but not enough to justify broader family rollout.

## Final Recommendation

Keep the branch as:

- a vendored `vector-ta` VWMA TM-output addition
- a repo-local `vwma/vwma` fast-path admission change
- new parity/error-path coverage in `double_ma.rs`

Do **not** broaden this into:

- mixed-pair admission
- EMA/WMA/ZLEMA follow-on work
- per-side capability cleanup
- TM-consumer/kernel-layout changes

This branch proved a bounded `vwma/vwma` win. It did not prove a general TM-native expansion campaign.
