I reviewed double_ma.rs and double_crossover.cu. The biggest wins here look structural, not micro-level: reduce useless pair work, improve warp coherence, and cut extra host↔device and metrics-memory traffic. On CUDA, coalesced global access, lower global-memory traffic, and lower warp divergence are usually the first-order levers.

Compact valid (fast, slow) pairs once, then use that compact list everywhere.
In the Rust CPU path, both the serial include_all branch and the Rayon branch still iterate rectangular tiles and skip fast_len >= slow_len inside the hot loop. The same pattern effectively survives on the GPU side too: the CUDA evaluation, heatmap, and top-k logic still reason in dense Pf_tile * Ps_tile space and re-check validity. From direct review of double_crossover.cu, that shows up in the main evaluator (2062–2093), heatmap (3049–3084), and top-k kernels (3093–3260). I would build a compact valid_pairs / pair_index_map once per tile, sorted and reused by CPU eval, GPU eval, heatmap, and top-k. That removes wasted work and also gives you better memory behavior and less branch waste.

Make your warp-dedup path actually pay off by ordering pairs for it.
Your CUDA file already has a nice fast-load helper using __match_any_sync + __shfl_sync to deduplicate the fast-MA load across lanes that share the same pf_local (double_crossover.cu:1741–1759). But that only helps when neighboring threads in a warp are assigned pairs with the same fast-period index. Right now the main kernel walks pair_local linearly and only optionally remaps through pair_index_map (double_crossover.cu:2064–2075, 2133–2136). So I would sort the compact pair map by pf_local first, then ps_local, or even launch per-fast-row batches for larger tiles. That should improve both cache/coalescing and dedup hit rate.

Bucket pairs by similar start_t / warmup so warps don’t stall on mixed workloads.
The kernel computes start_t per pair and, in the lockstep path, still loops from a common earlier bound and skips until the lane becomes active (double_crossover.cu:2096–2100, 2123–2129, 2193–2198). If one warp mixes short-warmup and long-warmup pairs, some lanes will spend cycles doing nothing. After compaction, I’d bucket or sort by start_t as a second key. That is a practical way to reduce control-flow divergence without changing the math.

Fuse evaluation and top-k for the common case.
Right now you write metrics for every pair and then launch separate top-k kernels that reread those metrics and sort block candidates with CUB (double_crossover.cu:3093–3260). When include_all=false, heatmap is off, and CSV export is off, I would keep a block-local top-k during evaluation and emit only candidate pairs. That saves a full global write/read round trip for the “optimize only” path. CUDA guidance generally favors reducing global-memory traffic before chasing smaller instruction tweaks.

Change heatmap from global atomicMax per pair to a two-stage reduction.
Your heatmap kernel does a global atomicMaxFloat for every valid pair (double_crossover.cu:3017–3084). For modest bin counts, that can become a contention point. A good next step is per-block shared-memory accumulation followed by one global merge pass. I would not do this before pair compaction, but I would do it before occupancy micro-tuning.

Fix the Rust hybrid staging path: it still pays extra f64→f32 conversion cost, and you are not timing the upload.
ma_matrix_cpu_f32_staging still does scalar f64→f32 copies in the batch path, and in the fallback path it materializes a full Vec<f64> and then collects to Vec<f32>. Then in optimize_double_ma_gpu_hybrid_kernel_phase1, those staged matrices are prepared, but gpu_indicator_prep_ms and h2d_upload_ms are still recorded as zero. That means the hybrid telemetry is probably under-reporting a real bottleneck. My order of attack would be: generate f32 natively where possible, stage into pinned host buffers, then use async H2D copies so you can overlap transfer and compute where the pipeline permits. NVIDIA’s async copy path explicitly depends on pinned memory.

Keep the current dispatcher shape in Rust; optimize the hot custom paths, not the routing.
eval_metrics_f64_with_exit_mode is already doing something sensible: it routes Kaufman/overlay/regime cases to custom evaluators and keeps simple cases on the faster library implementations. I would not spend time refactoring that dispatcher first. The better play is to optimize the custom paths that a profiler says dominate, especially the overlay/regime variants.

Parallelize include_all, or at least feed Rayon only valid pairs.
The non-include_all path already uses try_fold / try_reduce, which is a solid baseline. Rayon’s try_reduce is designed to stop the rest of the work as soon as possible on an Err/None, so the pattern itself is fine. The bigger issue is that you still map a dense pf * ps tile and bail out for invalid pairs inside the worker.
For include_all, the branch is still serial. If include_all matters for your workload, I’d move it to thread-local row buffers plus a final concat/sort.

Be deliberate about which strategy features you chase on GPU.
Your hybrid path already marks some combinations as kernel blockers: Kaufman-stop exit, execution overlays, and some objectives like Sortino/Omega. That’s a hint about where the easy GPU wins are: optimize the clean cross/no-overlay path first, because that path is already structurally closest to the kernel.

Do occupancy/launch tuning after the structural fixes.
Once the pair set is compacted and ordered, then tune block size and consider __launch_bounds__. NVIDIA’s guidance is that low occupancy hurts latency hiding, but more occupancy is not automatically faster; tune it with measurement rather than assumptions.

One optional experiment: use CUB warp loads on the read-heavy path.
CUB’s WarpLoad supports vectorized and transpose-based warp load strategies for coalescing. After you fix pair ordering, that could be worth trying on the slow-MA side or other read-heavy sections where the current scalar loads are the limiter.

What I’d do first, in order:

Build a compact valid-pair list and reuse it end-to-end.

Sort that list by fast_period, then by start_t bucket.

Fuse eval + top-k when include_all/heatmap/export are off.

Make hybrid staging native-f32 + pinned, and actually time H2D.

Only then tune block size, launch bounds, and heatmap atomics.

I’d implement the main change end to end: compact valid (fast, slow) pairs once per tile, use that compact worklist in double_ma.rs, and feed a sorted dense-pair map into the CUDA heatmap/top-k path in double_crossover.cu. Right now the CPU and GPU-sweep Rust paths still walk dense pf * ps rectangles and skip invalid pairs inside the hot loops, while the main CUDA evaluator already has pair_index_map / pair_count in its interface and launch args. double_ma.rs also forwards the kernel path into gpu_backtest_kernel.rs, so that launcher is the extra Rust touchpoint for the CUDA route. NVIDIA’s guidance lines up with this: minimize redundant global-memory access, keep accesses coherent/coalesced, and reduce warp divergence.

Use these as references while patching: double_ma.rs and double_crossover.cu.

1) Add these helpers to double_ma.rs

Put these near estimate_pairs_fast_lt_slow(...):

#[derive(Clone, Copy, Debug)]
struct ValidPairWork {
    fast_idx: usize,
    slow_idx: usize,
    fast_len: u16,
    slow_len: u16,
}

#[inline]
fn build_valid_pair_work(f_chunk: &[u16], s_chunk: &[u16]) -> Vec<ValidPairWork> {
    let mut out = Vec::with_capacity(estimate_pairs_fast_lt_slow(f_chunk, s_chunk));
    for (fast_idx, &fast_len) in f_chunk.iter().enumerate() {
        for (slow_idx, &slow_len) in s_chunk.iter().enumerate() {
            if fast_len < slow_len {
                out.push(ValidPairWork {
                    fast_idx,
                    slow_idx,
                    fast_len,
                    slow_len,
                });
            }
        }
    }
    out
}

/// Dense-pair ids sorted to improve CUDA warp locality:
/// 1) same fast row together (helps your fast-row warp dedup path)
/// 2) shorter warmups together
/// 3) then slow row
#[inline]
fn build_dense_pair_index_map_sorted(f_chunk: &[u16], s_chunk: &[u16]) -> Vec<i32> {
    let ps = s_chunk.len();
    let mut out: Vec<(usize, usize, usize, i32)> =
        Vec::with_capacity(estimate_pairs_fast_lt_slow(f_chunk, s_chunk));

    for (fast_idx, &fast_len) in f_chunk.iter().enumerate() {
        for (slow_idx, &slow_len) in s_chunk.iter().enumerate() {
            if fast_len < slow_len {
                let dense_pair = (fast_idx * ps + slow_idx) as i32;
                let warmup_key = (fast_len as usize).max(slow_len as usize);
                out.push((fast_idx, warmup_key, slow_idx, dense_pair));
            }
        }
    }

    out.sort_unstable_by_key(|&(fast_idx, warmup_key, slow_idx, _)| {
        (fast_idx, warmup_key, slow_idx)
    });

    out.into_iter()
        .map(|(_, _, _, dense_pair)| dense_pair)
        .collect()
}
2) Replace the hot-loop block in optimize_double_ma_cpu_tiled

Inside optimize_double_ma_cpu_tiled, replace the entire current inner block:

if include_all {
    ...
} else {
    ...
}

with this:

            let valid_pairs = build_valid_pair_work(f_chunk, s_chunk);
            let valid_pair_count = valid_pairs.len();

            if include_all {
                for pair in &valid_pairs {
                    let fast_row =
                        &fast_mat[pair.fast_idx * cols..(pair.fast_idx + 1) * cols];
                    let slow_row =
                        &slow_mat[pair.slow_idx * cols..(pair.slow_idx + 1) * cols];

                    let max_p = (pair.fast_len as usize)
                        .max(pair.slow_len as usize)
                        .max(1);
                    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                    let start_t = t_valid.saturating_add(trade_offset);

                    let metrics = if let Some((plan, ctx)) = dynamic_plan_ctx.as_ref() {
                        let dynamic_metrics =
                            crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu(
                                plan,
                                ctx,
                                fast_row,
                                slow_row,
                                start_t,
                                true,
                            )?;
                        if dynamic_shadow_enabled {
                            let sampled = dynamic_shadow_sampled
                                .fetch_add(1, Ordering::Relaxed)
                                .saturating_add(1);
                            if sampled % dynamic_shadow_every == 0 {
                                let static_metrics = eval_metrics_f64_with_exit_mode(
                                    &lr,
                                    close,
                                    fast_row,
                                    slow_row,
                                    halftrend_trend,
                                    start_t,
                                    strategy,
                                    kaufman_stops.as_ref(),
                                    true,
                                );
                                if !crate::strategy_dynamic::metrics_shadow_equivalent(
                                    &dynamic_metrics,
                                    &static_metrics,
                                ) {
                                    dynamic_shadow_mismatches.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                        dynamic_metrics
                    } else {
                        eval_metrics_f64_with_exit_mode(
                            &lr,
                            close,
                            fast_row,
                            slow_row,
                            halftrend_trend,
                            start_t,
                            strategy,
                            kaufman_stops.as_ref(),
                            true,
                        )
                    };

                    if metrics_pass_constraints(&metrics, constraints) {
                        agg.push(
                            DoubleMaParams {
                                fast_len: pair.fast_len,
                                slow_len: pair.slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            },
                            metrics,
                        );
                    }

                    processed = processed.saturating_add(1);
                    if (processed & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }
                }
            } else {
                let compute_sharpe = objective_needs_return_stats(objective);

                let init = || {
                    let mut local = StreamAggregator::new(objective, top_k, false, n);
                    if let Some((f_min, f_max, s_min, s_max)) = heatmap_cfg {
                        local = local.with_heatmap(
                            heatmap_bins,
                            heatmap_bins,
                            f_min,
                            f_max,
                            s_min,
                            s_max,
                        );
                    }
                    local
                };

                let tile = valid_pairs
                    .par_iter()
                    .try_fold(init, |mut local, pair| {
                        if cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }

                        let fast_row =
                            &fast_mat[pair.fast_idx * cols..(pair.fast_idx + 1) * cols];
                        let slow_row =
                            &slow_mat[pair.slow_idx * cols..(pair.slow_idx + 1) * cols];

                        let max_p = (pair.fast_len as usize)
                            .max(pair.slow_len as usize)
                            .max(1);
                        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                        let start_t = t_valid.saturating_add(trade_offset);

                        let metrics = if let Some((plan, ctx)) = dynamic_plan_ctx.as_ref() {
                            let dynamic_metrics =
                                crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu(
                                    plan,
                                    ctx,
                                    fast_row,
                                    slow_row,
                                    start_t,
                                    compute_sharpe,
                                )?;
                            if dynamic_shadow_enabled {
                                let sampled = dynamic_shadow_sampled
                                    .fetch_add(1, Ordering::Relaxed)
                                    .saturating_add(1);
                                if sampled % dynamic_shadow_every == 0 {
                                    let static_metrics = eval_metrics_f64_with_exit_mode(
                                        &lr,
                                        close,
                                        fast_row,
                                        slow_row,
                                        halftrend_trend,
                                        start_t,
                                        strategy,
                                        kaufman_stops.as_ref(),
                                        compute_sharpe,
                                    );
                                    if !crate::strategy_dynamic::metrics_shadow_equivalent(
                                        &dynamic_metrics,
                                        &static_metrics,
                                    ) {
                                        dynamic_shadow_mismatches.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                            dynamic_metrics
                        } else {
                            eval_metrics_f64_with_exit_mode(
                                &lr,
                                close,
                                fast_row,
                                slow_row,
                                halftrend_trend,
                                start_t,
                                strategy,
                                kaufman_stops.as_ref(),
                                compute_sharpe,
                            )
                        };

                        if metrics_pass_constraints(&metrics, constraints) {
                            local.push(
                                DoubleMaParams {
                                    fast_len: pair.fast_len,
                                    slow_len: pair.slow_len,
                                    fast_ma_id: 0,
                                    slow_ma_id: 0,
                                },
                                metrics,
                            );
                        }

                        Ok(local)
                    })
                    .try_reduce(init, |mut a, b| {
                        a.merge(b);
                        Ok(a)
                    })?;

                processed = processed.saturating_add(valid_pair_count);
                agg.merge(tile);
            }

That removes the dense pf * ps scan and the per-iteration if fast_len >= slow_len { continue; } in both branches. optimize_double_ma_cpu_tiled is one of the places still doing that today.

3) Replace the hot-loop block in optimize_double_ma_gpu_sweep_tiled

Do the same replacement pattern in optimize_double_ma_gpu_sweep_tiled. Replace its current inner:

if include_all {
    ...
} else {
    ...
}

with:

            let valid_pairs = build_valid_pair_work(f_chunk, s_chunk);
            let valid_pair_count = valid_pairs.len();

            if include_all {
                for pair in &valid_pairs {
                    let fast_row =
                        &fast_mat[pair.fast_idx * cols..(pair.fast_idx + 1) * cols];
                    let slow_row =
                        &slow_mat[pair.slow_idx * cols..(pair.slow_idx + 1) * cols];

                    let max_p = (pair.fast_len as usize)
                        .max(pair.slow_len as usize)
                        .max(1);
                    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                    let start_t = t_valid.saturating_add(trade_offset);

                    let metrics = eval_metrics_f32_with_exit_mode(
                        &lr,
                        close,
                        fast_row,
                        slow_row,
                        halftrend_trend,
                        start_t,
                        strategy,
                        kaufman_stops.as_ref(),
                        true,
                    );

                    if metrics_pass_constraints(&metrics, constraints) {
                        agg.push(
                            DoubleMaParams {
                                fast_len: pair.fast_len,
                                slow_len: pair.slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            },
                            metrics,
                        );
                    }

                    processed = processed.saturating_add(1);
                    if (processed & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }
                }
            } else {
                let compute_sharpe = objective_needs_return_stats(objective);

                let init = || {
                    let mut local = StreamAggregator::new(objective, top_k, false, n);
                    if let Some((f_min, f_max, s_min, s_max)) = heatmap_cfg {
                        local = local.with_heatmap(
                            heatmap_bins,
                            heatmap_bins,
                            f_min,
                            f_max,
                            s_min,
                            s_max,
                        );
                    }
                    local
                };

                let tile = valid_pairs
                    .par_iter()
                    .try_fold(init, |mut local, pair| {
                        if cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }

                        let fast_row =
                            &fast_mat[pair.fast_idx * cols..(pair.fast_idx + 1) * cols];
                        let slow_row =
                            &slow_mat[pair.slow_idx * cols..(pair.slow_idx + 1) * cols];

                        let max_p = (pair.fast_len as usize)
                            .max(pair.slow_len as usize)
                            .max(1);
                        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                        let start_t = t_valid.saturating_add(trade_offset);

                        let metrics = eval_metrics_f32_with_exit_mode(
                            &lr,
                            close,
                            fast_row,
                            slow_row,
                            halftrend_trend,
                            start_t,
                            strategy,
                            kaufman_stops.as_ref(),
                            compute_sharpe,
                        );

                        if metrics_pass_constraints(&metrics, constraints) {
                            local.push(
                                DoubleMaParams {
                                    fast_len: pair.fast_len,
                                    slow_len: pair.slow_len,
                                    fast_ma_id: 0,
                                    slow_ma_id: 0,
                                },
                                metrics,
                            );
                        }

                        Ok(local)
                    })
                    .try_reduce(init, |mut a, b| {
                        a.merge(b);
                        Ok(a)
                    })?;

                processed = processed.saturating_add(valid_pair_count);
                agg.merge(tile);
            }

That path also still does the dense rectangle + skip today.

4) Kernel-launcher Rust integration for the CUDA path

This part is not in the uploaded double_ma.rs; it lives in gpu_backtest_kernel.rs, because double_ma.rs forwards the actual kernel execution into that module. The main backtest kernel launch already passes pair_map_ptr and pair_count_i, so you only need to populate them per tile instead of leaving them empty.

Add this per tile, right before the backtest launch where pair_map_ptr / pair_count_i are prepared:

let pair_index_map_host = build_dense_pair_index_map_sorted(
    &fast_periods[f_start..f_start + pf],
    &slow_periods[s_start..s_start + ps],
);

let compact_pair_count = pair_index_map_host.len();
let launch_pairs = if compact_pair_count > 0 {
    compact_pair_count
} else {
    pf.saturating_mul(ps)
};

// Add this scratch buffer if you do not already have one:
// d_pair_index_map: Option<...i32 device buffer...>

let pair_index_map_ptr = if compact_pair_count > 0 {
    KernelScratch::ensure_len_i32(
        &mut scratch.d_pair_index_map,
        compact_pair_count,
        bt_stream,
    )?;
    scratch
        .d_pair_index_map
        .as_mut()
        .unwrap()
        .copy_from(&pair_index_map_host)
        .map_err(|e| e.to_string())?;
    scratch
        .d_pair_index_map
        .as_ref()
        .unwrap()
        .as_device_ptr()
        .as_raw()
} else {
    0
};

Then keep using the already-existing launch args:

let mut pair_map_ptr = pair_index_map_ptr;
let mut pair_count_i = if compact_pair_count > 0 {
    compact_pair_count as i32
} else {
    0
};

Two important caller rules:

For the backtest kernel, launch_pairs should be the compact count when you have a map.

For heatmap/top-k kernels, keep pairs = pf * ps as the dense metric-row count, and pass the compact count separately as pair_count.

5) Add this helper to double_crossover.cu

Put it near ld_tm_fast_warp_dedup(...):

static __device__ __forceinline__ int resolve_dense_pair_idx(
    int compact_pair,
    int dense_pairs,
    const int* __restrict__ pair_index_map)
{
    const int dense_pair =
        (pair_index_map != nullptr) ? pair_index_map[compact_pair] : compact_pair;

    return (dense_pair >= 0 && dense_pair < dense_pairs) ? dense_pair : -1;
}
6) Replace heatmap_update_scores_tm_f32

Replace the whole kernel with this version:

extern "C" __global__
void heatmap_update_scores_tm_f32(
    const float* __restrict__ metrics,
    int pairs,  // dense metric row count, usually Pf_tile * Ps_tile
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    int objective,
    int bins_fast,
    int bins_slow,
    int fast_min,
    int fast_max,
    int slow_min,
    int slow_max,
    const int* __restrict__ pair_index_map,
    int pair_count,
    float* __restrict__ heatmap_out
) {
    const int dense_pairs = pairs;
    const int total_pairs = (pair_count > 0) ? pair_count : pairs;
    const int stride = blockDim.x * gridDim.x;

    for (int compact_pair = blockIdx.x * blockDim.x + threadIdx.x;
         compact_pair < total_pairs;
         compact_pair += stride)
    {
        const int dense_pair =
            resolve_dense_pair_idx(compact_pair, dense_pairs, pair_index_map);
        if (dense_pair < 0) continue;

        const int pf_local = dense_pair / Ps_tile;
        const int ps_local = dense_pair - pf_local * Ps_tile;
        if (pf_local < 0 || pf_local >= Pf_tile || ps_local < 0 || ps_local >= Ps_tile) {
            continue;
        }

        const int f = fast_periods[pf_local];
        const int s = slow_periods[ps_local];
        if (f >= s) continue;

        const size_t base = (size_t)dense_pair * (size_t)metrics_stride;
        const float score = objective_score_f32(metrics, base, M, objective);
        if (!isfinite(score)) continue;

        const int fb = bin_period_i32(f, fast_min, fast_max, bins_fast);
        const int sb = bin_period_i32(s, slow_min, slow_max, bins_slow);
        const int idx = fb * bins_slow + sb;
        atomicMaxFloat(&heatmap_out[idx], score);
    }
}
7) Replace select_topk_from_pairs_tm_f32

Replace the whole kernel with this version:

extern "C" __global__
void select_topk_from_pairs_tm_f32(
    const float* __restrict__ metrics,
    int pairs,  // dense metric row count, usually Pf_tile * Ps_tile
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    int objective,
    int topk,
    const int* __restrict__ pair_index_map,
    int pair_count,
    int* __restrict__ out_pair_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    const int dense_pairs = pairs;
    const int total_pairs = (pair_count > 0) ? pair_count : pairs;

    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int compact_idx =
            block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;

        float score = NEG_INF_F;
        int val = -1;

        if (compact_idx < total_pairs) {
            const int dense_pair =
                resolve_dense_pair_idx(compact_idx, dense_pairs, pair_index_map);

            if (dense_pair >= 0) {
                const int pf_local = dense_pair / Ps_tile;
                const int ps_local = dense_pair - pf_local * Ps_tile;

                if (pf_local >= 0 && pf_local < Pf_tile &&
                    ps_local >= 0 && ps_local < Ps_tile)
                {
                    const int f = fast_periods[pf_local];
                    const int s = slow_periods[ps_local];
                    if (f < s) {
                        const size_t base = (size_t)dense_pair * (size_t)metrics_stride;
                        score = objective_score_f32(metrics, base, M, objective);
                        if (!isfinite(score)) score = NEG_INF_F;
                        val = dense_pair; // emit dense ids downstream
                    }
                }
            }
        }

        keys[item] = score;
        vals[item] = val;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_pair_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}
8) Replace select_topk_from_pairs_tm_f32_v2

Replace the whole kernel with this version:

extern "C" __global__
void select_topk_from_pairs_tm_f32_v2(
    const float* __restrict__ metrics,
    int pairs,  // dense metric row count, usually Pf_tile * Ps_tile
    int M,
    int metrics_stride,
    int Pf_tile,
    int Ps_tile,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    int objective,
    int topk,
    int use_min_trades,
    float min_trades,
    int use_max_dd,
    float max_dd,
    const int* __restrict__ pair_index_map,
    int pair_count,
    int* __restrict__ out_pair_idx
) {
    const int block_start = (int)blockIdx.x * (int)TOPK_BLOCK_ITEMS;
    const int dense_pairs = pairs;
    const int total_pairs = (pair_count > 0) ? pair_count : pairs;

    float keys[TOPK_ITEMS_PER_THREAD];
    int vals[TOPK_ITEMS_PER_THREAD];

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int compact_idx =
            block_start + (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;

        float score = NEG_INF_F;
        int val = -1;

        if (compact_idx < total_pairs) {
            const int dense_pair =
                resolve_dense_pair_idx(compact_idx, dense_pairs, pair_index_map);

            if (dense_pair >= 0) {
                const int pf_local = dense_pair / Ps_tile;
                const int ps_local = dense_pair - pf_local * Ps_tile;

                if (pf_local >= 0 && pf_local < Pf_tile &&
                    ps_local >= 0 && ps_local < Ps_tile)
                {
                    const int f = fast_periods[pf_local];
                    const int s = slow_periods[ps_local];
                    if (f < s) {
                        const size_t base = (size_t)dense_pair * (size_t)metrics_stride;
                        if (passes_constraints_f32(
                                metrics,
                                base,
                                M,
                                use_min_trades,
                                min_trades,
                                use_max_dd,
                                max_dd))
                        {
                            score = objective_score_f32(metrics, base, M, objective);
                            if (!isfinite(score)) score = NEG_INF_F;
                            val = dense_pair; // emit dense ids downstream
                        }
                    }
                }
            }
        }

        keys[item] = score;
        vals[item] = val;
    }

    typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
    __shared__ typename BlockSort::TempStorage temp_storage;
    BlockSort(temp_storage).SortDescending(keys, vals);

#pragma unroll
    for (int item = 0; item < TOPK_ITEMS_PER_THREAD; ++item) {
        const int rank = (int)threadIdx.x * TOPK_ITEMS_PER_THREAD + item;
        if (rank < topk) {
            out_pair_idx[(int)blockIdx.x * topk + rank] = vals[item];
        }
    }
}
9) What stays unchanged in double_crossover.cu

You do not need to change these if the two select_topk_from_pairs* kernels now emit dense pair ids:

select_topk_from_candidates_tm_f32

select_topk_from_candidates_tm_f32_v2

gather_topk_pairs_tm_f32

They already expect dense pair ids and decode them as pf_local = pair / Ps_tile; ps_local = pair - pf_local * Ps_tile;.

You also do not have to remove the period_f >= period_s safety guard in the main backtest kernel. Once the compact map only contains valid pairs it becomes redundant, but leaving it in is a harmless debug/safety check.

10) Callsite change summary

For every tile in the CUDA kernel path:

Build pair_index_map_host = build_dense_pair_index_map_sorted(f_chunk, s_chunk).

Upload it once to device.

Use the existing pair_map_ptr / pair_count_i slots for the main backtest launch.

Pass the same map/count into the heatmap and select_topk_from_pairs* launches.

Keep pairs = pf * ps for dense metric storage.

Use pair_count = valid_pair_count only as the scan length.

That gives you the full benefit: CPU path stops burning cycles on invalid pairs, GPU-sweep stops doing it too, and the CUDA kernel side finally reuses the same compact-valid-pair representation across eval, heatmap, and top-k, instead of only in the main evaluator.

After this lands, the next patch I’d do is the hybrid staging path: ma_matrix_cpu_f32_staging plus real H2D timing, because cudaMemcpyAsync only overlaps as intended with pinned/page-locked host buffers.