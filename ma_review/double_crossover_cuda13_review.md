# Review of `double_crossover.cu` for CUDA 13.x and related performance opportunities

Date: 2026-03-26

## Scope

This is a static code review of `/mnt/data/double_crossover.cu` with line-level references into the file.
I could not compile or profile the file in this environment because `nvcc` is not installed here, so the recommendations below are based on structure, dataflow, kernel shape, and current CUDA 13.x capabilities rather than measured kernel counters.

Line references below use the form `double_crossover.cu:<start>-<end>`.

## Executive summary

Your file is already well into “advanced CUDA” territory:

- A large amount of work is fused into single-pass device kernels, especially in `double_cross_backtest_tm_flex_f32_body` (`double_crossover.cu:3413-4014`).
- You already use warp-level cooperation via `__match_any_sync` and `__shfl_sync` in `ld_tm_fast_warp_dedup` (`double_crossover.cu:3090-3108`).
- You already keep objective scoring, constraints, heatmap updates, and shortlist formation on the GPU (`double_crossover.cu:4671-4850`, `4901-4968`, `4975-5700`).
- The specialized `__launch_bounds__(128)` entry points around the double-cross kernels (`double_crossover.cu:4064-4626`) are exactly the kind of surface where CUDA 13.0’s shared-memory register spilling is worth testing.[^smemspill]

After a second pass through the file, the strongest recommendations are still:

1. **A/B test CUDA 13.0 shared-memory register spilling on the double-cross wrappers first.**
2. **Benchmark CUDA 13.2 `cub::DeviceTopK` against the row-shortlist selectors, not against the fully fused pair-select kernels.**
3. **Do not rewrite this file around CUDA Tile right now.** CUDA Tile in CUDA 13.1/13.2 is currently Tile IR + cuTile Python; NVIDIA explicitly says C++ support is planned for a future release, so there is no direct CUDA C++ drop-in path for this `.cu` file today.[^tile13]
4. **A second-pass opportunity I did miss earlier:** for the double-cross kernels, `ret1` is only used to derive `step_r`, and you can compute that from `lr_t` in-kernel with `expm1f`. That gives you a real chance to delete the extra `ret1` array and the `compute_log_and_simple_returns_f32` preprocessing kernel entirely (`double_crossover.cu:3156-3179`, `3698-3737`, `3875-3915`).

---

## What the file already does well

### 1) Single-pass fused backtesting with compile-time specialization

The central helper, `double_cross_backtest_tm_flex_f32_body`, is a fused one-pass walk over time with a substantial amount of optional metric accumulation (`double_crossover.cu:3413-4014`).

It already combines:

- signal formation,
- trade accounting,
- stop/take-profit handling,
- drawdown / ulcer logic,
- return moments,
- downside metrics,
- Omega / Kappa-tau,
- underwater duration,
- per-side mean returns,
- trade summary metrics.

That is exactly the right high-level shape for a GPU search kernel when host orchestration and extra global-memory passes are expensive.

The compile-time specialization matrix at `double_crossover.cu:4064-4626` is also strong. You already have specialized entry points for:

- plain,
- windowed,
- fast-warp-dedup,
- lockstep,
- M7,
- regime-off / regime-on / hybrid,
- no-risk variants,
- epsilon-specialized variants.

That is a good sign: you are not trying to solve everything with one generic runtime-branch-heavy kernel.

### 2) Sensible warp-level cooperation

`ld_tm_fast_warp_dedup` (`double_crossover.cu:3090-3108`) is a real warp-level optimization, not just a stylistic flourish. It uses `__match_any_sync` to find lanes that need the same fast row and a single `__shfl_sync` to broadcast the result. Given the pair ordering, this is a good idea.

### 3) Device-side scoring and pruning

You already do objective evaluation and constraints on-device:

- `objective_score_f32` and `objective_score_vemr_f32` (`double_crossover.cu:4671-4762`)
- `passes_constraints_f32` (`double_crossover.cu:4764-4850`)
- `heatmap_update_scores_tm_f32` (`double_crossover.cu:4901-4968`)
- top-k selectors using `cub::BlockRadixSort` (`double_crossover.cu:4975-5700`)

That means the file is already following the high-value GPU pattern of “score / filter / compact on device, and only surface the small shortlist.”

---

## Highest-value CUDA 13.x opportunities

### 1) Best immediate CUDA 13.x experiment: shared-memory register spilling

**Why it fits this file**

The double-cross body carries a lot of live scalar state per thread (`double_crossover.cu:3553-3583`), and it is wrapped by explicit `__launch_bounds__(128)` kernels (`double_crossover.cu:4064-4626`). The hot kernels do not use dynamic shared memory, and the double-cross backtest kernels themselves do not allocate their own shared-memory scratchpad.

That is exactly the profile where CUDA 13.0’s opt-in shared-memory spilling is worth testing: register-heavy kernels with stable launch configuration and spare shared memory.[^smemspill]

**Where to test first**

I would test it first on the specialized M7 / lockstep entry points, especially:

- `double_cross_backtest_tm_flex_f32_m7` (`double_crossover.cu:4239`)
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7` (`double_crossover.cu:4282`)
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0` (`double_crossover.cu:4411`)
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0` (`double_crossover.cu:4626`)

Those are the versions most likely to be on the hot path and most likely to have the largest live-state footprint.

**How to test**

Add the pragma inside each wrapper:

```cpp
extern "C" __global__
__launch_bounds__(128)
void double_cross_backtest_tm_flex_f32_m7(/* ... */) {
    asm volatile(".pragma \"enable_smem_spilling\";");
    double_cross_backtest_tm_flex_f32_body<false, false, true>(/* ... */);
}
```

Then compare:

- `ptxas -v` spill loads / spill stores,
- static shared-memory usage,
- achieved occupancy,
- eligible warps per cycle,
- local-memory transactions,
- branch efficiency / stall reasons.

**Important caveat**

I would not start by enabling this on the `BlockRadixSort` selectors, because those kernels already consume static shared memory for temp storage (`double_crossover.cu:5033-5034`, `5112-5113`, `5212-5213`, `5319-5320`, `5427-5428`, `5473-5474`, `5541-5542`, `5613-5614`, `5654-5655`, `5699-5700`). The best fit is the register-heavy backtest kernels, not the sort kernels.

**File-wide extension**

If the experiment works on double-cross, the same idea may be worth testing on other backtest-style kernels in this file, such as:

- `strategy_program_double_ma_backtest_f32` (`double_crossover.cu:310-638`)
- `strategy_program_double_ma_constrained_tm_f32_m7` (`double_crossover.cu:640-792`)
- `strategy_program_double_ma_backtest_tm_f32_m7` (`double_crossover.cu:794-1052`)
- `asr_backtest_metrics_f32` (`double_crossover.cu:5920-6258`)
- `vcc_backtest_metrics_f32` (`double_crossover.cu:6261-6632`)
- `vemr_backtest_metrics_f32` (`double_crossover.cu:6635-7005`)

But for those, I would first check whether adding `__launch_bounds__` is warranted, because the CUDA 13.0 spill feature is easiest to reason about on kernels with explicit launch-shape constraints.[^smemspill]

### 2) Strong CUDA 13.2 benchmark candidate: `cub::DeviceTopK`

CUDA 13.2 / CCCL 3.2 adds `cub::DeviceTopK`, which NVIDIA says can deliver up to 5x speedups over full radix sort when K is small and you do not need to sort the entire input.[^topk132]

**Best candidate locations in your file**

I would benchmark it first against the **row-shortlist** selectors:

- `select_topk_metric_rows_f32` (`double_crossover.cu:5398-5439`)
- `select_topk_metric_rows_from_candidates_f32` (`double_crossover.cu:5440-5485`)
- `select_topk_metric_rows_f32_v2` (`double_crossover.cu:5486-5553`)
- `select_topk_metric_rows_from_candidates_f32_v2` (`double_crossover.cu:5554-5624`)
- `select_topk_vemr_metric_rows_f32` (`double_crossover.cu:5626-5665`)
- `select_topk_vemr_metric_rows_from_candidates_f32` (`double_crossover.cu:5667-5710`)

These are structurally closer to “select top K rows from a scored set.”

**Where I would _not_ replace the current code first**

I would **not** start by replacing:

- `select_topk_from_pairs_tm_f32` (`double_crossover.cu:4975-5045`)
- `select_topk_from_candidates_tm_f32` (`double_crossover.cu:5046-5124`)
- `select_topk_from_pairs_tm_f32_v2` (`double_crossover.cu:5125-5224`)
- `select_topk_from_candidates_tm_f32_v2` (`double_crossover.cu:5225-5331`)

Those kernels are not “just sort/select.” They are also doing row mapping, pair validation, constraint checks, and objective scoring. That fusion is valuable. Replacing them wholesale with a more generic device-wide top-k would likely force you to materialize more intermediates and could lose some of the current pipeline efficiency.

So the likely best use of `DeviceTopK` here is to collapse the **final shortlist formation** rather than the fully fused pair-scoring path.

### 3) CUDA Tile: checked, but not actionable for this C++ file today

I specifically re-checked CUDA Tile because it is one of the headline CUDA 13.x additions.

The conclusion for this file is still: **do not rewrite around it yet**.

Why:

- NVIDIA’s CUDA 13.1 launch is Tile IR + cuTile Python, not a mature CUDA C++ tile authoring model.[^tile13]
- NVIDIA’s own 13.1 post says a C++ implementation is planned for a future release, which means it is **not** the intended path for this `.cu` file today.[^tile13]
- Your hot kernels are not Tensor-Core / block-cooperative tile math kernels anyway. They are register-heavy per-thread state machines walking time series.

There are two places where a tile-flavored rewrite is conceptually imaginable:

- `transpose_row_to_tm` (`double_crossover.cu:3116-3129`)
- possibly some row-packing / staging helpers

But even there, the practical high-ROI fix is still a classic tiled shared-memory transpose, not CUDA Tile.

---

## Second-pass opportunities I missed earlier (not necessarily CUDA 13-specific)

### 4) High-value missed opportunity: remove the separate `ret1` pass

This is the most important non-13.x opportunity I missed on the first review.

`compute_log_and_simple_returns_f32` (`double_crossover.cu:3156-3179`) produces both:

- `lr_out[t] = logf(ratio)`
- `ret1_out[t] = ratio - 1.0f`

But in the double-cross path, `ret1` is only used here:

- `double_crossover.cu:3699`
- `double_crossover.cu:3876`

Specifically:

```cpp
const float step_r = (pos == 0) ? 0.0f : (float)pos * ret1[t];
```

Because `ret1[t] = expm1f(lr_t)`, you can compute the same value in the hot kernel from the already-loaded `lr_t`:

```cpp
const float step_r =
    (pos_bar == 0) ? 0.0f : (float)pos_bar * expm1f(lr_t);
```

That gives you a real A/B test with two potential wins:

1. remove the extra global memory stream for `ret1`,
2. potentially remove the `compute_log_and_simple_returns_f32` preprocessing kernel entirely if nothing else in the pipeline consumes `ret1`.

This is exactly the kind of “recompute instead of store/load” GPU trade-off that often pays off when memory traffic and orchestration dominate.

**Caveat:** `expm1f` is not free. So I would only keep this if profiling shows that removing the extra memory pass / array beats the extra SFU math in the metric-accumulation path.

But this one is important enough that I would benchmark it early.

### 5) Adaptive warp dedup may still have headroom: you only dedup the fast row today

In the double-cross time loop, you currently dedup the fast row only:

- `double_crossover.cu:3596-3599`
- `double_crossover.cu:3752-3755`

That is clearly good when `Ps_tile` is large, because many lanes in a warp will share the same `pf_local`.

But when `Ps_tile` is small, you can also get repeated `ps_local` values across the warp. In those shapes, the slow-row load may also be worth deduplicating.

So there is a real second-pass opportunity here:

- keep the current fast dedup when `Ps_tile` is large,
- consider slow dedup when `Ps_tile < warpSize`,
- or adaptively dedup whichever axis has fewer unique indices per warp.

I would not force “dedup both sides all the time,” because two `match_any` + broadcast sequences per iteration can add overhead. But **tile-shape-aware dedup** is worth testing.

### 6) `heatmap_update_scores_tm_f32` is a good candidate for warp-aggregated atomics

`heatmap_update_scores_tm_f32` bins each pair and then immediately issues:

- `atomicMaxFloat(&heatmap_out[idx], score)`
- `atomicAdd(&heatmap_sum_out[idx], score)`
- `atomicAdd(&heatmap_count_out[idx], 1)`

at `double_crossover.cu:4957-4965`.

If many lanes in a warp map to the same `(fb, sb)` bin, this can create avoidable atomic contention.

Because you already use `__match_any_sync` elsewhere in the file (`double_crossover.cu:3098`), this kernel is a natural candidate for **warp-aggregated atomics**:

- group lanes by `idx`,
- reduce `max(score)`,
- reduce `sum(score)`,
- reduce `count`,
- have only the leader lane issue the atomics.

This is not a CUDA 13.x addition, but it is exactly the kind of warp-level optimization that fits this codebase.

If you want a higher-level expression, Cooperative Groups is bundled in CUDA 13.2, but this optimization does **not** depend on a new 13.x capability.[^cuda132rn]

### 7) `transpose_row_to_tm` is still a naive transpose

`transpose_row_to_tm` (`double_crossover.cu:3116-3129`) is a straight global-memory transpose with:

- coalesced-ish reads from row-major input,
- strided writes into transposed storage.

If this kernel shows up meaningfully in Nsight Systems / Nsight Compute, I would replace it with a classic shared-memory tiled transpose. That is the right performance fix.

I **would not** wait for CUDA Tile here. Today, the practical C++ path is still the normal shared-memory tiled transpose.

### 8) Split metric families if register pressure remains high

The double-cross body is already specialized, but it still keeps a large optional metric surface inside one helper:

- feature gating and metric flags: `double_crossover.cu:3454-3571`
- final stores up to 22 metrics: `double_crossover.cu:3961-4011`

If the shared-memory spilling experiment still shows a lot of pressure, the next structural step is not “fuse more.” It is to split the body into a few compile-time metric families, for example:

- search row / ranking row,
- extended risk / underwater diagnostics,
- full trade-diagnostics row.

That should reduce live ranges more deterministically than additional micro-optimizations.

---

## Lower-priority code observations

### 9) Selector kernels may recompute the same objective twice

In the v2 selectors, a row can pass through `passes_constraints_f32(...)` and then immediately call `objective_score_f32(...)` again:

- `double_crossover.cu:5181-5182`
- `double_crossover.cu:5198-5199`
- `double_crossover.cu:5286-5287`
- `double_crossover.cu:5306-5307`
- `double_crossover.cu:5531-5532`
- `double_crossover.cu:5603-5604`

If the active objective is also one of the active constraints (for example Sharpe, Sortino, Calmar, or Omega), some work may be duplicated.

This is lower priority than the backtest kernel work, but if selector time becomes visible, a combined “evaluate constraints + cache objective” helper could shave instructions.

### 10) `regime_range_exit_rel` appears to be threaded through but unused

`regime_range_exit_rel` is passed through multiple helpers and wrappers, but I do not see it used in the actual regime-sign logic:

- parameter plumbing: `double_crossover.cu:3432`, `4035`, `4080`, `4123`, `4169`, `4212`, `4255`, `4298`, `4341`, `4384`, `4427`, `4470`, `4513`, `4556`, `4599`, `4642`
- sign logic references: `double_crossover.cu:2940`, `3009`, `3050`

That looks more like dead plumbing or an unfinished hysteresis input than a CUDA opportunity, but it is worth cleaning up because dead parameters make an already-complex kernel surface harder to reason about.

---

## CUDA 13.x features I checked but would not prioritize for this file

### 11) Modern CCCL runtime wrappers

CUDA 13.2 ships CCCL 3.2 with `cuda::stream`, `cuda::event`, `cuda::buffer`, and `cuda::launch`.[^cccl132]

Those are good host-side ergonomics improvements, but this file is essentially all device code and kernels. So they are not a performance patch for `double_crossover.cu` itself.

### 12) `cudaMemcpyWithAttributesAsync`

CUDA 13.2 adds `cudaMemcpyWithAttributesAsync` / `cudaMemcpy3DWithAttributesAsync`.[^cuda132misc]

Useful if your surrounding host runtime is copy-heavy and wants transfer attributes. Not directly actionable inside this file.

### 13) Green contexts / static SM partitioning

CUDA 13.1 exposes green contexts in the runtime API, and also adds MPS static SM partitioning for Ampere and newer GPUs.[^greenctx]

Those are **application-level** features, not kernel-level patches. They matter if your program runs these long backtest kernels concurrently with other latency-sensitive work and you want deterministic SM reservation. They do not suggest a source change inside this `.cu` file.

---

## Upgrade / build notes for CUDA 13.x

### 14) Current toolkit version and compatibility

As of today, the current toolkit is **CUDA 13.2.0**, and CUDA 13.x minor-version compatibility requires an NVIDIA driver branch **R580 or newer**.[^cuda132rn][^archive]

### 15) CCCL / CUB / libcu++ versions

CUDA 13.2 ships:

- Thrust 3.2.0
- CUB 3.2.0
- libcu++ 3.2.0

according to the 13.2 release notes.[^cuda132rn]

### 16) Your CUB include is fine for normal `nvcc` compilation, but be aware of the CUDA 13 header move

This file includes CUB as:

```cpp
#include <cub/block/block_radix_sort.cuh>
```

at `double_crossover.cu:11`.

CUDA 13.0 moved CCCL headers under `include/cccl/`, but NVIDIA says the common case of files compiled with `nvcc` requires no action. The migration note mainly matters if you have host-compiler-only translation units that consume CUB/Thrust headers directly, or if your build system hardcodes include paths.[^ccclmove]

Also note that CCCL 3.0 requires **C++17 or newer**.[^ccclmove]

### 17) New debugging / profiling support worth adopting immediately

Even though it is not a runtime speed feature, CUDA 13.1’s toolchain adds two genuinely useful items for this file:

- `nvcc -fdevice-sanitize=memcheck` for faster, deeper memcheck instrumentation in Compute Sanitizer,[^sanitizer131]
- improved Nsight Compute / Nsight Systems support around CUDA graph nodes and graph-related profiling.[^sanitizer131]

For a large, heavily specialized kernel file like this, that is worth using during optimization work.

### 18) Windows-specific note

If you are on Windows in WDDM mode, CUDA 13.2 + R595 reduces per-context local-memory footprint, which can matter in memory-constrained environments and especially with spill-heavy kernels.[^cuda132misc]

---

## Suggested benchmarking order

1. **Shared-memory spilling on the double-cross M7 / lockstep wrappers**
   - most plausible direct CUDA 13.x performance win.

2. **Delete the `ret1` pass experimentally**
   - compute `step_r` from `lr_t` with `expm1f`,
   - measure whether removing the extra buffer/pass beats the extra math.

3. **Benchmark `cub::DeviceTopK` on the row-shortlist stage**
   - especially `select_topk_metric_rows_*` and `select_topk_vemr_metric_rows_*`.

4. **Add warp-aggregated atomics to `heatmap_update_scores_tm_f32`**
   - only if heatmap updates show up in the profile.

5. **Prototype adaptive slow-row dedup**
   - especially when `Ps_tile < warpSize`.

6. **Only then consider structural refactors**
   - split metric families if register pressure still dominates.

---

## Bottom line

My updated view after the second pass is:

- I did **not** find a hidden “rewrite this around some shiny CUDA 13.x feature” opportunity.
- I **did** confirm that the two best CUDA 13.x experiments remain:
  - **shared-memory register spilling**, and
  - **`cub::DeviceTopK`**.
- I also found one important non-13.x opportunity I missed earlier:
  - **remove the separate `ret1` pass by recomputing it from `lr_t` inside the double-cross kernel**.
- And I found two additional worthwhile code-level tests:
  - **warp-aggregated heatmap atomics**,
  - **adaptive slow-row warp dedup**.

If I were spending engineering time on this file next, that is the order I would use.

## Sources

[^archive]: NVIDIA CUDA Toolkit Archive: https://developer.nvidia.com/cuda-toolkit-archive
[^cuda132rn]: NVIDIA, “CUDA Toolkit 13.2 - Release Notes”: https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html
[^smemspill]: NVIDIA, “How to Improve CUDA Kernel Performance with Shared Memory Register Spilling” and CUDA 13.0 release notes: https://developer.nvidia.com/blog/how-to-improve-cuda-kernel-performance-with-shared-memory-register-spilling/ and https://docs.nvidia.com/cuda/archive/13.0.0/cuda-toolkit-release-notes/index.html
[^tile13]: NVIDIA, “NVIDIA CUDA 13.1 Powers Next-Gen GPU Programming with NVIDIA CUDA Tile and Performance Gains” and “CUDA 13.2 Introduces Enhanced CUDA Tile Support and New Python Features”: https://developer.nvidia.com/blog/nvidia-cuda-13-1-powers-next-gen-gpu-programming-with-nvidia-cuda-tile-and-performance-gains/ and https://developer.nvidia.com/blog/cuda-13-2-introduces-enhanced-cuda-tile-support-and-new-python-features/
[^topk132]: NVIDIA, “CUDA 13.2 Introduces Enhanced CUDA Tile Support and New Python Features” (CCCL 3.2 / `cub::DeviceTopK`): https://developer.nvidia.com/blog/cuda-13-2-introduces-enhanced-cuda-tile-support-and-new-python-features/
[^greenctx]: NVIDIA, “NVIDIA CUDA 13.1 Powers Next-Gen GPU Programming with NVIDIA CUDA Tile and Performance Gains” (green contexts and static SM partitioning): https://developer.nvidia.com/blog/nvidia-cuda-13-1-powers-next-gen-gpu-programming-with-nvidia-cuda-tile-and-performance-gains/
[^sanitizer131]: NVIDIA, “NVIDIA CUDA 13.1 Powers Next-Gen GPU Programming with NVIDIA CUDA Tile and Performance Gains” (Compute Sanitizer compile-time patching and graph profiling): https://developer.nvidia.com/blog/nvidia-cuda-13-1-powers-next-gen-gpu-programming-with-nvidia-cuda-tile-and-performance-gains/
[^cccl132]: NVIDIA, “CUDA 13.2 Introduces Enhanced CUDA Tile Support and New Python Features” (modern CUDA C++ runtime APIs in CCCL 3.2): https://developer.nvidia.com/blog/cuda-13-2-introduces-enhanced-cuda-tile-support-and-new-python-features/
[^cuda132misc]: NVIDIA, “CUDA 13.2 Introduces Enhanced CUDA Tile Support and New Python Features” (`cudaMemcpyWithAttributesAsync`, LMEM note, mempool query APIs): https://developer.nvidia.com/blog/cuda-13-2-introduces-enhanced-cuda-tile-support-and-new-python-features/
[^ccclmove]: NVIDIA, “What’s New and Important in CUDA Toolkit 13.0” (CCCL header move and C++17 requirement): https://developer.nvidia.com/blog/whats-new-and-important-in-cuda-toolkit-13-0/