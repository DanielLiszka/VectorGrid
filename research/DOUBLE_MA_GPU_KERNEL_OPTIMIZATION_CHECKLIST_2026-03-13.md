# Double-MA GPU Kernel Optimization Checklist

This checklist is limited to optimization advice that held up against the current code in:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

It intentionally excludes ideas that were not strongly supported by the current implementation review.

## Scope

- Focus on the existing Double-MA family GPU-native kernel route.
- Preserve current optimization semantics unless explicitly noted.
- Prefer structural reductions in wasted work and memory traffic before occupancy tuning.
- Use benchmark-first, patch-second, verify-third workflow for every optimization target.

## Execution Protocol

- [ ] Before touching a target, run its baseline benchmark if one exists.
- [ ] Record the exact Criterion filter, mean/median time, GPU model, and relevant env vars.
- [ ] Make only the smallest change set needed for that target.
- [ ] Run targeted unit tests for the affected kernel/function path.
- [ ] Re-run the same benchmark with the same env settings.
- [ ] Reject the change if it regresses correctness or if the measured gain is not worth the added complexity.

## Repo Commands For This Checklist

### Baseline GPU-kernel benchmark

- [ ] Run:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [ ] Optional perf-gate wrapper:

```powershell
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate -BenchFilter "double_ma/gpu_kernel/sma_sma/200k/small"
```

### Strategy-profile GPU-kernel benchmark

- [ ] Run when the change may affect non-plain Double-MA profiles:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_strategy/sma_sma/200k"
```

### Hybrid benchmark

- [ ] Run only for hybrid staging / transfer work:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_hybrid_kernel/sma_sma/200k/small"
```

### Targeted GPU-kernel tests

- [ ] Pair-map / include-all / heatmap parity:

```powershell
cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel -- --nocapture
```

- [ ] Top-k v2 vs legacy parity:

```powershell
cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture
```

- [ ] Global top-k device merge parity:

```powershell
cargo test -p ta_app_core gpu_kernel_global_topk_matches_host_merge_path --features cuda,cuda-backtest-kernel -- --nocapture
```

- [ ] Batched vs unbatched parity:

```powershell
cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params --features cuda,cuda-backtest-kernel -- --nocapture
```

- [ ] Export path regression:

```powershell
cargo test -p ta_app_core gpu_kernel_export_all_csv_writes_one_row_per_pair --features cuda,cuda-backtest-kernel -- --nocapture
```

## 1. Compact Valid-Pair Handling End-to-End

- [ ] Confirm current valid-pair map behavior in `build_valid_pair_index_map_i32(...)` at `crates/app_core/src/gpu_backtest_kernel.rs`.
- [ ] Keep the existing compact valid-pair launch path for the main backtest kernel as the baseline integration path.
- [ ] Extend compact valid-pair handling into the GPU reduction stages instead of scanning dense `Pf * Ps` tiles.
- [ ] Add `pair_index_map` and `pair_count` support to `heatmap_update_scores_tm_f32(...)` in `crates/app_core/src/kernels/double_crossover.cu`.
- [ ] Add `pair_index_map` and `pair_count` support to `select_topk_from_pairs_tm_f32(...)` in `crates/app_core/src/kernels/double_crossover.cu`.
- [ ] Add `pair_index_map` and `pair_count` support to `select_topk_from_pairs_tm_f32_v2(...)` in `crates/app_core/src/kernels/double_crossover.cu`.
- [ ] Update the corresponding reduction-kernel launch sites in `crates/app_core/src/gpu_backtest_kernel.rs` to pass compact pair metadata.
- [ ] Keep dense metric storage only where downstream compatibility actually requires it.

### Execution Checklist

- [ ] Run baseline benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [ ] Apply compact valid-pair propagation to:
- [ ] `heatmap_update_scores_tm_f32(...)`
- [ ] `select_topk_from_pairs_tm_f32(...)`
- [ ] `select_topk_from_pairs_tm_f32_v2(...)`
- [ ] corresponding launch sites in `gpu_backtest_kernel.rs`
- [ ] Run tests:

```powershell
cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_global_topk_matches_host_merge_path --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params --features cuda,cuda-backtest-kernel -- --nocapture
```

- [ ] Re-run the same benchmark and compare against baseline.

## 2. Dense Host-Side Tile Cleanup

- [ ] Remove dense rectangular scanning with `if fast_len >= slow_len { continue; }` from `optimize_double_ma_cpu_tiled(...)` in `crates/app_core/src/double_ma.rs`.
- [ ] Replace the tiled CPU hot loop with iteration over a prebuilt valid-pair worklist.
- [ ] Remove the same dense rectangular scan pattern from `optimize_double_ma_gpu_sweep_tiled(...)` in `crates/app_core/src/double_ma.rs`.
- [ ] Reuse the same valid-pair worklist concept in both `include_all` and non-`include_all` branches where practical.
- [ ] Re-check progress accounting so processed counts still reflect valid `fast < slow` pairs only.

### Execution Checklist

- [ ] Run baseline benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [ ] Apply the CPU/GPU-sweep tiled valid-pair worklist changes in `double_ma.rs`.
- [ ] Run tests that cover result parity and batched execution behavior:

```powershell
cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params --features cuda,cuda-backtest-kernel -- --nocapture
```

- [ ] Re-run the same benchmark and compare against baseline.

## 3. Heatmap Path

- [ ] Profile `heatmap_update_scores_tm_f32(...)` before changing it further.
- [ ] If heatmap contention is material, prototype a two-stage reduction:
- [ ] Stage 1: shared-memory per-block accumulation.
- [ ] Stage 2: global merge/update.
- [ ] Only pursue heatmap atomic optimization after compact valid-pair support is in place.

### Execution Checklist

- [ ] Run baseline benchmark with heatmap enabled:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [ ] If possible, collect a profiling pass before modifying atomics.
- [ ] Apply the two-stage heatmap reduction only if profiling shows `heatmap_update_scores_tm_f32(...)` is material.
- [ ] Run tests:

```powershell
cargo test -p ta_app_core gpu_kernel_best_matches_include_all_true --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params --features cuda,cuda-backtest-kernel -- --nocapture
```

- [ ] Re-run the same benchmark and compare against baseline.

## 4. Top-K Reduction Path

- [ ] Benchmark the current two-stage top-k reduction after compact valid-pair propagation.
- [ ] Only consider eval-and-top-k fusion for the narrow fast path:
- [ ] `include_all = false`
- [ ] no heatmap
- [ ] no export
- [ ] no host-side constraint filtering
- [ ] Treat eval-and-top-k fusion as a second-phase optimization, not the first change.

### Execution Checklist

- [ ] Run baseline benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small"
```

- [ ] First optimize the existing two-stage top-k path before attempting any fusion.
- [ ] If eval-plus-top-k fusion is attempted, constrain it to the narrow fast path only.
- [ ] Run tests:

```powershell
cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_global_topk_matches_host_merge_path --features cuda,cuda-backtest-kernel -- --nocapture
```

- [ ] Re-run the same benchmark and compare against baseline.

## 5. Hybrid Staging And Transfer Telemetry

- [ ] Review `ma_matrix_cpu_f32_staging(...)` in `crates/app_core/src/double_ma.rs` for avoidable `f64 -> f32` conversion cost.
- [ ] Prefer native `f32` generation where the underlying batch path supports it.
- [ ] Audit the fallback path that materializes `Vec<f64>` before collecting into `Vec<f32>`.
- [ ] Replace placeholder telemetry values for `gpu_indicator_prep_ms` and `h2d_upload_ms` in `crates/app_core/src/double_ma.rs`.
- [ ] Measure actual H2D timing inside the kernel executor path in `crates/app_core/src/gpu_backtest_kernel.rs`.
- [ ] Use pinned host buffers where transfer overlap is intended.

### Execution Checklist

- [ ] Run baseline hybrid benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_hybrid_kernel/sma_sma/200k/small"
```

- [ ] Apply the staging / telemetry changes only to the hybrid path.
- [ ] Run the most relevant kernel-route smoke/parity tests plus any hybrid-specific checks added during implementation.
- [ ] Re-run the same hybrid benchmark and compare against baseline.

## 6. Measurement Gate Before Micro-Tuning

- [ ] Re-profile after compact valid-pair propagation and hybrid-staging fixes.
- [ ] Check whether backtest, heatmap, top-k, or finalize phases are now dominant.
- [ ] Only then evaluate:
- [ ] block-size changes
- [ ] launch-bounds tuning
- [ ] warp-load experiments
- [ ] additional pair ordering experiments

### Execution Checklist

- [ ] Do not enter this phase unless at least one earlier structural change shows measurable benefit.
- [ ] Use the same benchmark filter before and after every micro-tuning change.
- [ ] Prefer one-variable-at-a-time tuning so regressions are attributable.
- [ ] Re-run the targeted GPU-kernel tests after each accepted tuning change.

## 7. Explicit Non-Priorities

- [ ] Do not treat pair reordering for warp dedup as a first-pass optimization.
- [ ] Do not assume warmup/start-time bucketing is a proven win without profiling.
- [ ] Do not start with a full eval-plus-top-k kernel fusion rewrite.
- [ ] Do not spend time on occupancy tuning before removing known dense invalid-pair work.

## Suggested Order

- [ ] Step 1: propagate compact valid-pair handling through heatmap and top-k.
- [ ] Step 2: remove dense invalid-pair iteration from tiled CPU and GPU-sweep Rust paths.
- [ ] Step 3: fix hybrid staging and transfer telemetry.
- [ ] Step 4: profile again.
- [ ] Step 5: only then decide whether heatmap atomics, fusion, or launch tuning are justified.
