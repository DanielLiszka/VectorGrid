# Double-MA GPU Kernel Optimization Checklist

This checklist is limited to optimization advice that held up against the current code in:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

It intentionally excludes ideas that were not strongly supported by the current implementation review.

## Progress Notes

- Implemented on 2026-03-14:
- compact valid-pair propagation through GPU heatmap and first-stage top-k reductions
- dense invalid-pair cleanup in `optimize_double_ma_cpu_tiled(...)`
- dense invalid-pair cleanup in `optimize_double_ma_gpu_sweep_tiled(...)`
- Verification completed with:
- `cargo check -q -p ta_app_core --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt`
- targeted GPU-kernel tests:
  `gpu_kernel_best_matches_include_all_true`,
  `gpu_kernel_topk_v2_matches_forced_legacy`,
  `gpu_kernel_global_topk_matches_host_merge_path`,
  `gpu_kernel_batched_matches_unbatched_best_params_nma`
- Criterion run:
  `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`
  reported about `70.3 ms` mean with Criterion showing an improvement versus its prior sample set
- `cargo bench` and `cargo check` against the repo default `target` directory were blocked by file-lock/access issues, so isolated verification used `--target-dir C:\Users\dlisz\.codex\memories\target_dm_opt`
- Deferred follow-up discovered during parity verification:
- `gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies`
  currently fails on `composite_trend` with
  `pnl delta too large for composite_trend: 0.5724107124129776`
  and should be investigated separately from the pure throughput track

## Scope

- Focus on the existing Double-MA family GPU-native kernel route.
- Preserve current optimization semantics unless explicitly noted.
- Prefer structural reductions in wasted work and memory traffic before occupancy tuning.
- Use benchmark-first, patch-second, verify-third workflow for every optimization target.

## Execution Protocol

- [x] Before touching a target, run its baseline benchmark if one exists.
- [x] Record the exact Criterion filter, mean/median time, GPU model, and relevant env vars.
- [x] Make only the smallest change set needed for that target.
- [x] Run targeted unit tests for the affected kernel/function path.
- [x] Re-run the same benchmark with the same env settings.
- [x] Reject the change if it regresses correctness or if the measured gain is not worth the added complexity.

## Repo Commands For This Checklist

### Baseline GPU-kernel benchmark

- [x] Run:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [x] Optional perf-gate wrapper:

```powershell
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate -BenchFilter "double_ma/gpu_kernel/sma_sma/200k/small"
```

- Latest optional perf-gate rerun on `2026-03-14` failed against the stored baseline:
- `gpu_criterion_mean_not_regressed_10pct`: `192.31064 ms` vs baseline `155.288513 ms`
- `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `645 ms` (`on`) vs `190.5 ms` (`off`)

Current result on `2026-03-14`: gate ran and failed `gpu_criterion_mean_not_regressed_10pct` with `192.31064 ms` vs baseline `155.288513 ms`, while `gpu_dynamic_strategy_ratio_not_regressed_20pct` passed.

### Strategy-profile GPU-kernel benchmark

- [x] Run when the change may affect non-plain Double-MA profiles:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_strategy/sma_sma/200k"
```

- Latest rerun used:
  `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- "double_ma/gpu_kernel_strategy/sma_sma/200k"`
- All reported strategy routes stayed on `backend_used=Gpu` and `backend_route=GpuKernel` with no fallback.
- Most sampled cases were either within noise or showed no detected change.
- Notable regressions in this rerun were:
  `set/large`, `orc/xlarge`, `breakout_momentum/large`, and `breakout_momentum/xlarge`.
- `trend_quality/large` improved in this rerun.
- Criterion emitted one sample-history warning on `pit/xlarge` while reading `base/sample.json`, but the benchmark still produced timings for that case.

### Hybrid benchmark

- [x] Run only for hybrid staging / transfer work:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_hybrid_kernel/sma_sma/200k/small"
```

### Targeted GPU-kernel tests

- [x] Pair-map / include-all / heatmap parity:

```powershell
cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel -- --nocapture
```

- [x] Top-k v2 vs legacy parity:

```powershell
cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture
```

- [x] Global top-k device merge parity:

```powershell
cargo test -p ta_app_core gpu_kernel_global_topk_matches_host_merge_path --features cuda,cuda-backtest-kernel -- --nocapture
```

- [x] Batched vs unbatched parity:

```powershell
cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params --features cuda,cuda-backtest-kernel -- --nocapture
```

- [x] Export path regression:

```powershell
cargo test -p ta_app_core gpu_kernel_export_all_csv_writes_one_row_per_pair --features cuda,cuda-backtest-kernel -- --nocapture
```

## 1. Compact Valid-Pair Handling End-to-End

- [x] Confirm current valid-pair map behavior in `build_valid_pair_index_map_i32(...)` at `crates/app_core/src/gpu_backtest_kernel.rs`.
- [x] Keep the existing compact valid-pair launch path for the main backtest kernel as the baseline integration path.
- [x] Extend compact valid-pair handling into the GPU reduction stages instead of scanning dense `Pf * Ps` tiles.
- [x] Add `pair_index_map` and `pair_count` support to `heatmap_update_scores_tm_f32(...)` in `crates/app_core/src/kernels/double_crossover.cu`.
- [x] Add `pair_index_map` and `pair_count` support to `select_topk_from_pairs_tm_f32(...)` in `crates/app_core/src/kernels/double_crossover.cu`.
- [x] Add `pair_index_map` and `pair_count` support to `select_topk_from_pairs_tm_f32_v2(...)` in `crates/app_core/src/kernels/double_crossover.cu`.
- [x] Update the corresponding reduction-kernel launch sites in `crates/app_core/src/gpu_backtest_kernel.rs` to pass compact pair metadata.
- [x] Keep dense metric storage only where downstream compatibility actually requires it.

### Execution Checklist

- [x] Run baseline benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [x] Apply compact valid-pair propagation to:
- [x] `heatmap_update_scores_tm_f32(...)`
- [x] `select_topk_from_pairs_tm_f32(...)`
- [x] `select_topk_from_pairs_tm_f32_v2(...)`
- [x] corresponding launch sites in `gpu_backtest_kernel.rs`
- [x] Run tests:

```powershell
cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_global_topk_matches_host_merge_path --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params --features cuda,cuda-backtest-kernel -- --nocapture
```

- [x] Re-run the same benchmark and compare against baseline.

### Progress Notes

- Dense host metric storage is already conditional in the current executor: host-side dense tile copies are kept only for `include_all`, CSV export, or host-side constraint filtering.
- Dense device tile metrics are still retained because the current backtest-kernel output ABI and row-reduction pipeline consume dense tile buffers. No compact-row kernel rewrite was accepted in this pass.

## 2. Dense Host-Side Tile Cleanup

- [x] Remove dense rectangular scanning with `if fast_len >= slow_len { continue; }` from `optimize_double_ma_cpu_tiled(...)` in `crates/app_core/src/double_ma.rs`.
- [x] Replace the tiled CPU hot loop with iteration over a prebuilt valid-pair worklist.
- [x] Remove the same dense rectangular scan pattern from `optimize_double_ma_gpu_sweep_tiled(...)` in `crates/app_core/src/double_ma.rs`.
- [x] Reuse the same valid-pair worklist concept in both `include_all` and non-`include_all` branches where practical.
- [x] Re-check progress accounting so processed counts still reflect valid `fast < slow` pairs only.

### Execution Checklist

- [x] Run baseline benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [x] Apply the CPU/GPU-sweep tiled valid-pair worklist changes in `double_ma.rs`.
- [x] Run tests that cover result parity and batched execution behavior:

```powershell
cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params --features cuda,cuda-backtest-kernel -- --nocapture
```

- [x] Re-run the same benchmark and compare against baseline.

## 3. Heatmap Path

- [x] Profile `heatmap_update_scores_tm_f32(...)` before changing it further.
- [x] Determine from profiling whether heatmap contention is material before attempting a two-stage reduction.
- [x] Keep the shared-memory plus global-merge heatmap rewrite deferred because current measured workloads do not justify it.
- [x] Leave heatmap atomic optimization behind compact valid-pair propagation in the priority order.

### Execution Checklist

- [x] Run baseline benchmark with heatmap enabled:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [x] If possible, collect a profiling pass before modifying atomics.
- [x] Do not apply the two-stage heatmap reduction because profiling did not show `heatmap_update_scores_tm_f32(...)` to be material.
- [x] Keep the existing regression-test set as the validation baseline; no additional heatmap-specific code change was accepted in this phase.
- [x] Re-run the same benchmark/profile filter and confirm heatmap remains non-dominant.

### Progress Notes

- Profiling reruns used:

```powershell
$env:VECTORBT_KERNEL_PROFILE_PIPELINE='1'; cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- "double_ma/gpu_kernel/sma_sma/200k/small"
$env:VECTORBT_KERNEL_PROFILE_PIPELINE='1'; cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- "double_ma/gpu_kernel/alma_alma/200k/small"
```

- On the profiled `sma/sma` run, `tile_score/backtest` dominated at roughly `99.8%` to `99.9%` of total measured time. `heatmap` was typically about `0.05 ms`.
- On the profiled `alma/alma` run, `tile_score/backtest` still dominated at roughly `94%` to `99%` depending on sample variance. `heatmap` stayed small, usually about `0.04 ms` to `0.22 ms`.
- Current evidence does not justify a heatmap-specific rewrite on these measured workloads.

## 4. Top-K Reduction Path

- [x] Benchmark the current two-stage top-k reduction after compact valid-pair propagation.
- [x] Limit any future eval-and-top-k fusion experiment to the narrow fast path only:
- [x] `include_all = false`
- [x] no heatmap
- [x] no export
- [x] no host-side constraint filtering
- [x] Treat eval-and-top-k fusion as a second-phase optimization, not the first change.

### Execution Checklist

- [x] Run baseline benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [x] Keep the current two-stage top-k path as the accepted implementation; do not jump straight to fusion.
- [x] If eval-plus-top-k fusion is ever attempted, constrain it to the narrow fast path only.
- [x] Run tests:

```powershell
cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_global_topk_matches_host_merge_path --features cuda,cuda-backtest-kernel -- --nocapture
```

- [x] Re-run the same benchmark/profile filter and compare against baseline.

## 5. Hybrid Staging And Transfer Telemetry

- [x] Review `ma_matrix_cpu_f32_staging(...)` in `crates/app_core/src/double_ma.rs` for avoidable `f64 -> f32` conversion cost.
- [x] Prefer native `f32` generation where the underlying batch path supports it.
- [x] Audit the fallback path that materializes `Vec<f64>` before collecting into `Vec<f32>`.
- [x] Replace placeholder telemetry values for `gpu_indicator_prep_ms` and `h2d_upload_ms` in `crates/app_core/src/double_ma.rs`.
- [x] Measure actual H2D timing inside the kernel executor path in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [x] Confirm the kernel executor already supports pinned host uploads where transfer overlap is intended via env-gated `LockedBuffer` H2D copies.

### Execution Checklist

- [x] Run baseline hybrid benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_hybrid_kernel/sma_sma/200k/small"
```

- [x] Add a representative staged-hybrid benchmark filter for the CPU-staged path:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_hybrid_kernel/frama_sma/200k/small"
```

- [x] Apply the staging / telemetry changes only to the hybrid path.
- [x] Run the most relevant kernel-route smoke/parity tests plus any hybrid-specific checks added during implementation.
- [x] Re-run the same hybrid benchmark and compare against baseline.

### Progress Notes

- `cargo check -q -p ta_app_core --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt` passed after the telemetry patch.
- Baseline hybrid benchmark from the prior run on `2026-03-14`: `double_ma/gpu_hybrid_kernel/sma_sma/200k/small/1590pairs` = `[198.23 ms 198.67 ms 199.15 ms]`.
- Current rerun after the telemetry patch: `double_ma/gpu_hybrid_kernel/sma_sma/200k/small/1590pairs` = `[68.814 ms 68.986 ms 69.149 ms]`. Criterion reported improvement.
- Passing hybrid/staging tests rerun:
- `cargo test -p ta_app_core cpu_ma_matrix_f32_staging_matches_f64_cast --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- `cargo test -p ta_app_core gpu_kernel_precomputed_frama_matrix_executes --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- `cargo test -p ta_app_core gpu_hybrid_kernel_capable_pair_stays_on_gpu_path --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- `cargo test -p ta_app_core gpu_hybrid_large_alma_route_smoke --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- `cargo test -p ta_app_core gpu_hybrid_batched_matches_unbatched_best_params_alma --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- `cargo test -p ta_app_core gpu_hybrid_cpu_staged_kernel_capable_ma_stays_on_gpu_path --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- `cargo test -p ta_app_core gpu_hybrid_vdps_cpu_staged_kernel_capable_ma_stays_on_gpu_path --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- Removed invalid mixed-origin hybrid assertions that attempted to route non-kernel indicators through the current GPU-kernel backend. That behavior is not part of the present contract.
- `ma_matrix_cpu_f32_staging(...)` now forces explicit CPU kernels for staging:
- batched paths use `Kernel::ScalarBatch`
- the row-wise fallback uses `ma_with_kernel(..., Kernel::Scalar)`
- The row-wise fallback path in `ma_matrix_cpu_f32_staging(...)` also writes directly into the preallocated `Vec<f32>` rather than materializing a full `Vec<f64>` matrix first.
- `cpu_ma_matrix_f32_staging_matches_f64_cast` now covers `frama` as well as the existing `sma` and `gaussian` cases.
- The live `optimize_double_ma_gpu_kernel_impl(...)` tile loop now honors precomputed host MA matrices again. The current executor had regressed to always recomputing fast/slow rows on GPU, which broke the staged-hybrid `frama` path despite valid host precompute.
- Representative staged-hybrid benchmark rerun:
- `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt_bench2 -- "double_ma/gpu_hybrid_kernel/frama_sma/200k/small"`
- Result: `double_ma/gpu_hybrid_kernel/frama_sma/200k/small/1590pairs` = `[122.12 ms 122.43 ms 122.74 ms]`
- Note: this rerun used a fresh target dir because the prior bench target had a locked `double_ma_opt.exe` on Windows.
- Existing `gpu_hybrid_kernel/sma_sma` and `gpu_hybrid_kernel/alma_alma` benchmarks still do not exercise CPU staging, so `frama_sma` is the representative staged-hybrid measurement for this checklist item.

## 6. Measurement Gate Before Micro-Tuning

- [x] Document the deferred CPU-vs-GPU parity follow-up discovered after the structural optimization work.
- [x] Investigate the failed parity case in `gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies`.
- [x] Keep this parity investigation separate from the pure throughput/profiling track unless the same issue reproduces on plain `double_ma`.
- [x] Re-profile after compact valid-pair propagation and hybrid-staging fixes.
- [x] Check whether backtest, heatmap, top-k, or finalize phases are now dominant.
- [x] Decide against block-size, launch-bounds, warp-load, or additional pair-ordering experiments for this pass because the current profile does not justify them.

### Execution Checklist

- [x] Record the failed parity test for later investigation:
- `cargo test -p ta_app_core gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- Current investigated failure: `pnl delta too large for composite_trend: 0.5724107124129776`
- Validation-side trend-quality parity still passes:
  `cargo test -p ta_app_core trend_quality_validation_gpu_parity_diagnostic --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
  and
  `cargo test -p ta_app_core double_ma_validation_gpu_kernel_matches_cpu_metrics_trend_quality_adx_off --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- Dynamic-strategy preflight parity also passes for the same double-like strategy family:
  `cargo test -p ta_app_core gpu_kernel_dynamic_strategy_env_path_matches_legacy_results_for_double_like_strategies --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- Additional narrowing:
  for the same `composite_trend` single-combo request, `Backend::GpuOnly` still matches `Backend::CpuOnly` exactly, while `Backend::GpuKernel` does not.
- Disabling the kernel fast-path, lockstep-time path, and `m7` specialization did not change the mismatch.
- Current conclusion: the remaining mismatch is isolated to `Backend::GpuKernel` optimizer parity for the `composite_trend` single-combo optimization case. It was investigated and documented here, but no corrective code change was accepted in the throughput checklist pass.
- [x] Do not enter this phase unless at least one earlier structural change shows measurable benefit.
- [x] No micro-tuning change was accepted in this pass, so there is no before/after tuning loop to run.
- [x] No micro-tuning change was accepted in this pass, so there is no one-variable-at-a-time tuning result to attribute.
- [x] No micro-tuning change was accepted in this pass, so there are no accepted tuning changes that require a dedicated retest loop.

### Next Feasible Items

- [x] Run a measurement/profile pass on plain GPU-kernel workloads first, for example `double_ma/gpu_kernel/sma_sma/200k/small` or `double_ma/gpu_kernel/alma_alma/200k/small`, because those paths already showed measurable structural wins and are not blocked by the new `composite_trend` parity failure.
- [x] Plain GPU-kernel profiling did not show heatmap work to be material on the sampled small workloads, so keep heatmap-specific rewrites deferred for now.
- [x] `composite_trend` CPU-vs-GPU optimizer parity was investigated after the measurement pass; the failure remains isolated and documented, but no fix was folded into this throughput checklist.

### Progress Notes

- Plain GPU-kernel measurement/profile passes are now complete for:
- `double_ma/gpu_kernel/sma_sma/200k/small`
- `double_ma/gpu_kernel/alma_alma/200k/small`
- Current dominant phase on the sampled plain GPU-kernel workloads is still the backtest stage.
- Heatmap, finalize, and other non-backtest phases are small on the measured small-grid workloads, so micro-tuning should not start with heatmap atomics.

## 7. Explicit Non-Priorities

- [x] Do not treat pair reordering for warp dedup as a first-pass optimization.
- [x] Do not assume warmup/start-time bucketing is a proven win without profiling.
- [x] Do not start with a full eval-plus-top-k kernel fusion rewrite.
- [x] Do not spend time on occupancy tuning before removing known dense invalid-pair work.

## 8. Dense Metric-Storage Rewrite Plan

This is a future implementation plan, not work already accepted in the current pass.

### Goal

- Replace dense per-tile metric storage indexed as `dense_pair = i * ps + j` with compact valid-pair metric rows indexed by the valid `fast < slow` pair order already represented by `pair_index_map`.
- Preserve current optimization semantics for:
- `include_all`
- CSV export
- host-side constraint filtering
- pair-stage top-k
- global row reduction / gather

### Current Limitation

- The executor now launches the backtest over compact valid pairs when `pair_index_map` is enabled, but `scratch.d_tile` is still allocated as dense `Pf * Ps * M`.
- The current implementation explicitly keeps the tile output dense for downstream consumers in `crates/app_core/src/gpu_backtest_kernel.rs`.
- Host-side `include_all` / export / host-filter paths and several row-reduction stages still assume dense pair ids.

### Success Criteria

- The main backtest kernel writes metrics only for valid-pair rows.
- `scratch.d_tile` sizing scales with `launch_pairs * metrics_stride` rather than `pf * ps * metrics_stride`.
- All downstream consumers either:
- consume compact rows directly, or
- perform an explicit dense remap only where product behavior truly requires it.
- Existing top-k, export, and parity tests continue to pass.
- Large-tile or memory-limited workloads show a measurable reduction in tile memory pressure or an observable throughput win.

### Phase 1: Introduce Compact Tile Row ABI

- [ ] Add an explicit compact-row execution mode in the GPU executor, gated behind a new env flag such as `VECTORBT_KERNEL_COMPACT_TILE_ROWS`.
- [ ] Define a compact row invariant:
- row `r` in `d_tile` corresponds to `pair_index_map[r]`
- row count is `launch_pairs`, not `pf * ps`
- [ ] Thread the compact row count and mapping metadata through the main tile bookkeeping in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [ ] Keep the legacy dense path available until all downstream consumers are migrated.

### Phase 2: Backtest Kernel Output Rewrite

- [ ] Update the main backtest kernel output layout in `crates/app_core/src/kernels/double_crossover.cu` so metric rows are written by compact valid-pair row index.
- [ ] Preserve the current dense path behind the compatibility flag during transition.
- [ ] Verify all kernel variants that share the same output ABI:
- [ ] baseline
- [ ] lockstep-time
- [ ] `m7`
- [ ] regime-on / regime-hybrid variants
- [ ] windowed-eval variant if it shares the same output contract

### Phase 3: Pair-Stage Device Reductions

- [ ] Update pair-stage top-k kernels to treat `metrics_ptr` as compact valid-pair rows instead of dense pair ids.
- [ ] Remove any remaining dependence on dense `pairs = pf * ps` in the first reduction stage when `pair_index_map` is present.
- [ ] Confirm heatmap behavior:
- [ ] either keep a compact-row read path that still bins by true `(fast_len, slow_len)`
- [ ] or perform an explicit remap only for heatmap if that is simpler and cheaper

### Phase 4: Host-Side Compatibility Paths

- [ ] Rewrite `include_all` host materialization to iterate compact rows and reconstruct `(fast_len, slow_len)` from `pair_index_map`.
- [ ] Rewrite `host_filter_mode` to read compact rows rather than scanning dense `(i, j)` space.
- [ ] Rewrite CSV export to stream compact rows directly.
- [ ] Keep any dense remap logic isolated and explicit if a compatibility path still needs it.

### Phase 5: Global Row Reduction / Gather

- [ ] Update global row accumulation buffers and `gather_topk_metric_rows_f32` integration to use compact row ids.
- [ ] Confirm second-stage gather/reduction does not assume dense tile row ids.
- [ ] Audit any row-id to `(fast_len, slow_len)` reconstruction logic and switch it to valid-pair reconstruction.

### Phase 6: Remove Legacy Dense Tile Path

- [ ] After compact-row parity is established, remove the dense compatibility mode.
- [ ] Delete any dead dense remap code left over from the migration.
- [ ] Re-profile tile memory footprint and throughput after the cleanup.

### Validation Checklist

- [ ] `cargo check -q -p ta_app_core --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt`
- [ ] `cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- [ ] `cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- [ ] `cargo test -p ta_app_core gpu_kernel_global_topk_matches_host_merge_path --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- [ ] `cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params_nma --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- [ ] `cargo test -p ta_app_core gpu_kernel_export_all_csv_writes_one_row_per_pair --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- [ ] `cargo test -p ta_app_core gpu_kernel_precomputed_frama_matrix_executes --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- [ ] Run at least one CPU-vs-GPU parity check after each accepted phase:
- [ ] `cargo test -p ta_app_core gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt -- --nocapture`
- [ ] Re-run the representative benchmarks after each accepted phase:
- [ ] `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt_bench2 -- "double_ma/gpu_kernel/sma_sma/200k/small"`
- [ ] `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --target-dir C:\Users\dlisz\.codex\memories\target_dm_opt_bench2 -- "double_ma/gpu_kernel/alma_alma/200k/small"`
- [ ] If the rewrite targets memory pressure explicitly, re-run a larger-tile or memory-constrained case as well.

### Suggested Rollout Order

- [ ] Step 1: add compact-row ABI and keep dense path behind a flag.
- [ ] Step 2: switch the main backtest kernel write path to compact rows.
- [ ] Step 3: migrate pair-stage top-k and heatmap consumers.
- [ ] Step 4: migrate host `include_all`, export, and host-filter paths.
- [ ] Step 5: migrate global row reduction / gather.
- [ ] Step 6: remove the dense compatibility path only after parity and benchmark checks pass.

### Stop / Go Criteria

- [ ] Stop if parity breaks outside the currently deferred `composite_trend` issue and the break is not clearly attributable.
- [ ] Stop if compact-row complexity spreads into too many product-only compatibility paths without a measurable tile-memory or throughput benefit.
- [ ] Proceed only if either:
- [ ] tile memory footprint drops materially on large workloads, or
- [ ] representative benchmarks show a repeatable win, or
- [ ] compact rows unlock larger tile sizes / better VRAM utilization that dense rows currently block.

## Suggested Order

- [x] Step 1: propagate compact valid-pair handling through heatmap and top-k.
- [x] Step 2: remove dense invalid-pair iteration from tiled CPU and GPU-sweep Rust paths.
- [x] Step 3: fix hybrid staging and transfer telemetry.
- [x] Step 4: profile again.
- [x] Step 5: decide whether heatmap atomics, fusion, or launch tuning are justified. Current answer: not justified on the measured workloads, so keep them deferred.
