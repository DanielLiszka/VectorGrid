# `double_crossover.cu` / `double_crossover.sm_89.cubin` performance and correctness review

## Scope and limitations

I reviewed:

- `double_crossover.cu`
- `double_crossover.sm_89.cubin`

This review focuses on the **single-pass double-MA backtest path**, especially **17-row and 22-row metric output modes**.

Important limitation: I could not do full SASS disassembly in this environment because `nvdisasm` / `cuobjdump` are not installed. The cubin review is therefore based on:

- ELF section sizes
- cubin kernel metadata inferred from section headers
- source-to-cubin correlation

That is still enough to identify the main optimization opportunities, but not enough to make instruction-level scheduling claims.

---

## Executive summary

### Bottom line

Yes: there is still meaningful optimization headroom for the full-row single-pass backtest path.

For **M17/M22**, the biggest missed optimization is still the lack of **true fixed-contract specializations** analogous to the existing optimized M10 kernels — but after a final third pass, I think there are **three additional gaps** that matter almost as much:

1. the **routing matrix is incomplete** (`lockstep + fast_warp_dedup` is missing in the generic path, and the `windowed` path only has a generic baseline wrapper)
2. the full-row kernels still carry too much **runtime flag logic** (`trade_on_next_bar`, `signed_exposure`, `long_only`, `no_flip`)
3. the backtest kernel does not currently emit **direct score / feasibility side outputs**, so later kernels re-read full metric rows and recompute values you already had live

The highest-value changes are:

1. **Add true fixed M17 and M22 kernels** for common mode combinations.
2. **Fill the missing route combinations**, especially generic `LOCKSTEP_TIME + FAST_WARP_DEDUP` and optimized `windowed` variants.
3. **Promote common flags to compile-time contracts** (`TRADE_ON_NEXT_BAR`, `STRAT_SIGNED_EXPOSURE`, and optionally `LONG_ONLY` / `NO_FLIP`).
4. **Reduce hot-loop overhead** with pointer-walking, lockstep warp-uniform broadcasts, and pair maps that are warp-friendly for dedup.
5. **Emit `score_out` / `feasible_out` directly from the backtest finalizer** when ranking follows immediately.
6. **Fix the reduced-row decode bugs and the final-pass contract issues** before trusting all configurations.

### What I would do first

If I were optimizing this codebase for an RTX 4090, CUDA 13.2, and the full-row backtest path, my first implementation pass would be:

1. Add **M17** and **M22** fixed-row kernels for:
   - non-lockstep, regime off, no risk
   - lockstep, regime off, no risk
   - optional `eps0`, `nocomm`, and `eps0+nocomm` variants if those are common
2. Add the missing **generic combined wrapper**:
   - `double_cross_backtest_tm_flex_f32_lockstep_time_fast_warp_dedup`
3. If `eval_start/eval_end` is common, mirror the same route matrix for the **windowed** path.
4. Create one **fixed-flag M17/M22 branch** with:
   - `TRADE_ON_NEXT_BAR=false`
   - `STRAT_SIGNED_EXPOSURE=false`
   - optional `LONG_ONLY=true` if that is common in production
5. Convert the hot loop to **pointer-walking** and add **lockstep warp-uniform broadcasts** for `lr`, `ret1`, and regime values.
6. Add optional **`score_out` / `feasible_out`** outputs so ranking can skip recomputation.
7. Fix:
   - the **`M < 15` vs `M < 17` calmar decode mismatch**
   - the **`use_max_dd` reduced-row bug**
   - the **`max_abs_net_exposure` vs `STRAT_SIGNED_EXPOSURE` contract bug**
   - host validation for **`metrics_out_stride >= M`**

---

## Cubin findings

### Backtest kernels in the cubin

From the cubin section metadata, the hot backtest kernels look like this:

| kernel | text size | regs/thread (inferred) | approx occupancy @ 128 threads | comment |
|---|---:|---:|---:|---|
| `double_cross_backtest_tm_flex_f32` | 25,472 B | 64 | 32 warps / 67% | generic baseline |
| `double_cross_backtest_tm_flex_f32_windowed` | 25,600 B | 64 | 32 warps / 67% | generic + eval window |
| `double_cross_backtest_tm_flex_f32_fast_warp_dedup` | 26,240 B | 64 | 32 warps / 67% | generic + warp dedup |
| `double_cross_backtest_tm_flex_f32_lockstep_time` | 25,856 B | 64 | 32 warps / 67% | generic + lockstep |
| `double_cross_backtest_tm_flex_f32_m7` | 23,936 B | 63 | 32 warps / 67% | `FULL_M7` alone helps only a little |
| `double_cross_backtest_tm_flex_f32_lockstep_time_m7` | 25,344 B | 71 | 28 warps / 58% | lockstep M7 is heavier |
| `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk` | 11,008 B | 48 | 40 warps / 83% | very strong specialization result |
| `..._m10_regime_off_norisk_nocomm` | 9,472 B | 45 | 44 warps / 92% | even leaner |
| `..._m10_regime_off_norisk_eps0` | 10,112 B | 47 | 40 warps / 83% | lean |
| `..._m10_regime_off_norisk_eps0_nocomm` | 9,344 B | 45 | 44 warps / 92% | leanest visible path |

Notes:

- The register counts above are **inferred from cubin section metadata**, not from `nvdisasm` output.
- The occupancy estimate assumes Ada SM89 limits: 48 warps/SM and 64K 32-bit registers/SM.
- I do **not** see static shared-memory usage on the backtest kernels.

### Why this matters for M17/M22

The important comparison is not generic vs M7. The important comparison is:

- **generic dynamic-contract kernels**: ~64 regs/thread
- **fixed-contract, fixed-mode kernels**: ~45–48 regs/thread

That tells us the big gain is not “a little more specialization.” It tells us the big gain is:

- fixed metric row width
- fixed regime mode
- fixed risk mode
- fixed commission mode when applicable
- fixed epsilon mode when applicable

You already proved that this strategy works with the M10 lockstep kernels (`double_crossover.cu:4472-4650`).

You have **not** yet applied that same strategy to **M17 or M22**.

---

## Why M17/M22 are different from M10

The hot loop derives its runtime work mask from `M` and `metrics_out_stride`:

- `double_crossover.cu:3474-3482`
- `double_crossover.cu:3589-3591`
- `double_crossover.cu:3979-4030`

In the generic path:

- `need_trades`, `need_max_dd`, `need_stats`, `need_std`, `need_expo`, `need_downside_dev`, `need_ulcer`, `need_omega`, `need_kappa_tau`
- `need_side_means`, `need_underwater`, `need_trade_metrics`
- the final metric stores

all depend on runtime `M` and/or runtime `metrics_out_stride`.

For **M17** and **M22**, those booleans are effectively fixed:

### M17 implies

- `need_side_means = true`
- `need_underwater = true`
- `need_trade_metrics = false`

### M22 implies

- `need_side_means = true`
- `need_underwater = true`
- `need_trade_metrics = true`

That means a specialized M17/M22 kernel can eliminate a large amount of runtime control flow and dead code.

This matters because for the full-row path the hot loop is already doing substantial work per bar. Extra dynamic branching and unused writer logic are exactly the kind of things that push a kernel from the high-50-register range into the low/mid-60s.

### Practical occupancy thresholds on Ada at 128 threads/block

For this kernel family, shaving even a few registers matters:

| regs/thread | active blocks/SM | active warps/SM | occupancy |
|---:|---:|---:|---:|
| 64 | 8 | 32 | 67% |
| 56 | 9 | 36 | 75% |
| 51 | 10 | 40 | 83% |
| 45 | 11 | 44 | 92% |
| 42 | 12 | 48 | 100% |

So an M17/M22 specialization does **not** need to drop all the way to M10 levels to be worth it. Even getting from ~64 regs to the mid-50s would be meaningful.

---

## Highest-priority performance recommendations

## 1. Add fixed M17 and M22 specializations

### Why

This is the single biggest missing optimization for your stated target workload.

You already have specialized M10 wrappers that hard-wire:

- `M=10`
- `metrics_out_stride=10`
- `REGIME_MODE=0`
- `RISK_MODE=0`
- optional `EPS_MODE=0`
- optional `COMMISSION_MODE=0`

See:

- `double_crossover.cu:4472-4650`

Those wrappers are much smaller and leaner in the cubin.

### What is missing

You do **not** currently have analogous wrappers for:

- `M=17`
- `M=22`

That is the main hole for single-pass full-row performance.

### Minimal-change implementation

A minimal version can follow your existing M10 wrapper pattern almost exactly.

Example: add a **lockstep M17 regime-off no-risk** wrapper:

```cpp
extern "C" __global__
__launch_bounds__(128)
void double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    const float* __restrict__ ret1,
    int T,
    int Pf_tile, int Ps_tile,
    int fast_tm_stride, int slow_tm_stride,
    int Pf_total, int Ps_total,
    int f_offset, int s_offset,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel,
    int first_valid,
    int regime_first_valid,
    float commission,
    float eps_rel,
    float hurdle_tau,
    float stop_log,
    float take_log,
    unsigned int flags,
    int M,
    int metrics_out_stride,
    const int* __restrict__ pair_index_map,
    int pair_count,
    int compact_output,
    float* __restrict__ metrics_out)
{
    (void)M;
    (void)metrics_out_stride;
    double_cross_backtest_tm_flex_f32_body<true, true, true, 0, 0>(
        fast_ma_T, slow_ma_T, lr, ret1,
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride,
        Pf_total, Ps_total, f_offset, s_offset,
        fast_periods, slow_periods, regime_filter,
        use_regime, use_regime_hybrid,
        regime_range_entry_rel, regime_range_exit_rel,
        first_valid, regime_first_valid,
        commission, eps_rel, hurdle_tau,
        stop_log, take_log,
        flags,
        17, 17,
        pair_index_map, pair_count, compact_output,
        metrics_out);
}
```

And similarly for M22:

```cpp
... flags, 22, 22, pair_index_map, pair_count, compact_output, metrics_out
```

### Stronger implementation (better for codegen)

A stronger version is to refactor the body to make fixed `M` a **template parameter**, rather than relying on constant propagation from the wrapper call.

Suggested refactor shape:

```cpp
template <
    int FIXED_M,
    bool FAST_WARP_DEDUP,
    bool LOCKSTEP_TIME,
    int REGIME_MODE = -1,
    int RISK_MODE = -1,
    int EPS_MODE = -1,
    int COMMISSION_MODE = -1>
static __device__ __forceinline__ void double_cross_backtest_tm_flex_f32_body_fixed(...)
{
    constexpr int METRICS_STRIDE = FIXED_M;
    constexpr bool NEED_TRADES         = FIXED_M > 1;
    constexpr bool NEED_MAX_DD         = FIXED_M > 2;
    constexpr bool NEED_STATS          = FIXED_M > 3;
    constexpr bool NEED_STD            = FIXED_M > 4;
    constexpr bool NEED_EXPO           = FIXED_M > 5;
    constexpr bool NEED_DOWNSIDE_DEV   = FIXED_M > 7;
    constexpr bool NEED_ULCER          = FIXED_M > 8;
    constexpr bool NEED_OMEGA          = FIXED_M > 9;
    constexpr bool NEED_KAPPA_TAU      = FIXED_M > 14;
    constexpr bool NEED_SIDE_MEANS     = FIXED_M > 12;
    constexpr bool NEED_UNDERWATER     = FIXED_M > 16;
    constexpr bool NEED_TRADE_METRICS  = FIXED_M > 21;
    ...
}
```

This gives the compiler the strongest possible signal that M17 and M22 are fully fixed contracts.

### Which wrappers to add first

I would add these first:

#### Non-lockstep

- `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk`
- `double_cross_backtest_tm_flex_f32_m22_regime_off_norisk`
- `..._nocomm`
- `..._eps0`
- `..._eps0_nocomm`

#### Lockstep

- `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk`
- `..._nocomm`
- `..._eps0`
- `..._eps0_nocomm`

### Why both lockstep and non-lockstep matter

Right now the big fixed-contract win only exists in the **lockstep M10** family.

If your common M17/M22 work does not take the lockstep path, you fall back to the heavier generic kernels:

- `double_crossover.cu:4082-4254`

That is why I would implement both families, not just lockstep.

---

## 2. Keep `FAST_WARP_DEDUP` only when pair order preserves `pf` locality

### Why

The fast-row dedup helper is here:

- `double_crossover.cu:3108-3127`

It uses `__match_any_sync()` to detect lanes in the warp that share the same `pf_local`, then loads the fast MA row once and broadcasts it.

That only pays off when adjacent lanes tend to share the same `pf_local`.

Your dense mapping does exactly that:

- `double_crossover.cu:3497-3505`

```cpp
const int pf_local = dense_pair / Ps_tile;
const int ps_local = dense_pair % Ps_tile;
```

That means lanes walk `ps_local` fastest, so a warp often shares one `pf_local` across many lanes.

### The catch

If `pair_index_map` is compacted or reordered without preserving dense/pf-major order, `FAST_WARP_DEDUP` can lose its benefit or even become overhead.

### Recommendation

For M17/M22 specialized routing, make `FAST_WARP_DEDUP` depend on pair-map ordering quality:

- **Use the dedup specialization** when `pair_index_map == nullptr` or when the pair map preserves dense/pf-major ordering.
- **Use a non-dedup specialization** when the pair map is compacted in a way that scrambles `pf_local` adjacency.

### Concrete host-side rule

If you compact valid pairs, emit them in increasing `dense_pair` order.

That preserves:

- fast-row locality
- fast-warp dedup usefulness
- row-major compact output

If you cannot preserve that ordering, benchmark a non-dedup specialized variant.

### Implementation note

The existing M10 specializations are inconsistent here:

- `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk` uses `FAST_WARP_DEDUP=true` (`double_crossover.cu:4506`)
- the `nocomm` / `eps0` M10 variants use `FAST_WARP_DEDUP=false` (`double_crossover.cu:4552`, `4597`, `4643`)

That suggests you should **benchmark both choices** for M17/M22 rather than hard-code one assumption everywhere.

---

## 3. Add lockstep warp-uniform broadcasts for `lr`, `ret1`, and regime values

### Why

In the lockstep path, all active lanes are iterating the same `t` value. That means these loads are warp-uniform:

- `lr[t]`
- `ret1[t]`
- `regime_filter[t_sig]`

Relevant code locations:

- no-risk path load of `lr[t]`: `double_crossover.cu:3687-3692`
- no-risk path load of `ret1[t]`: `double_crossover.cu:3716-3724`
- risk path load of `lr[t]`: `double_crossover.cu:3843-3850`
- risk path load of `ret1[t]`: `double_crossover.cu:3893-3901`
- regime load happens inside the sign helpers:
  - `double_crossover.cu:2958`
  - `double_crossover.cu:3013`
  - `double_crossover.cu:3036`

### Why this matters more for M17/M22

For M17/M22:

- `lr[t]` is always needed
- `ret1[t]` is always needed
- drawdown/stats/omega/downside/ulcer logic is always active

So these warp-uniform loads are hit every iteration.

### Concrete implementation

Add a small helper:

```cpp
static __device__ __forceinline__ float warp_uniform_ld_f32(
    const float* __restrict__ a,
    int idx)
{
    const unsigned mask = __activemask();
    const int lane = (int)(threadIdx.x & 31);
    const int leader = __ffs((int)mask) - 1;
    float v = 0.0f;
    if (lane == leader) {
        v = ld_ro_1d(a, (size_t)idx);
    }
    return __shfl_sync(mask, v, leader);
}
```

Then in the lockstep path:

```cpp
const float lr_t = LOCKSTEP_TIME
    ? warp_uniform_ld_f32(lr, t)
    : ld_ro_1d(lr, (size_t)t);

const float ret1_t = LOCKSTEP_TIME
    ? warp_uniform_ld_f32(ret1, t)
    : ld_ro_1d(ret1, (size_t)t);
```

Use `ret1_t` in the stats block instead of reloading.

### Regime value broadcast

For the regime helpers, the cleanest implementation is to add an overload that accepts a preloaded regime value:

```cpp
template <int REGIME_MODE>
static __device__ __forceinline__ int desired_sign_mode_with_rv_f32(
    float f,
    float s,
    float eps,
    unsigned int flags,
    float rv,
    int use_regime,
    int use_regime_hybrid,
    float regime_range_entry_rel,
    float regime_range_exit_rel);
```

Then the lockstep loop can do:

```cpp
const float rv = (regime_filter != nullptr)
    ? warp_uniform_ld_f32(regime_filter, t_sig)
    : NAN;
```

and pass `rv` into the sign helper.

### Expected effect

This is not likely to beat M17/M22 specialization in importance, but it is a good second-wave optimization for the lockstep kernels.

---

## 4. Make compact valid-pair maps the default for ranked/search routes

### Why

Invalid pair handling currently does a full zero-write path when `fast >= slow` and the flag is enabled:

- `double_crossover.cu:3515-3538`

There is a second large zero-write path when `start_t >= end_t`:

- `double_crossover.cu:3547-3570`

For M17 and especially M22, that is a lot of wasted memory traffic.

### Why this matters more for 17/22 rows

Each invalid row zeroes:

- 17 floats = 68 bytes for M17
- 22 floats = 88 bytes for M22

If you evaluate a dense `Pf_tile * Ps_tile` surface and enforce `fast < slow`, you can easily throw away ~half the pair surface while still paying for those stores.

### Recommendation

For search/ranking workflows, generate a compact `pair_index_map` containing **only valid pairs**, in dense/pf-major order, and launch with:

- `pair_index_map != nullptr`
- `pair_count = valid_count`
- `compact_output = 1`

The kernel is already set up for this:

- `double_crossover.cu:3491-3513`

### Host-side generation rule

Generate the compact pair list in **ascending `dense_pair` order**.

That gives you three things at once:

1. No wasted invalid-row writes
2. Good `FAST_WARP_DEDUP` behavior
3. Stable compact row indexing

---

## 5. A/B test `__launch_bounds__(128,2)` or a per-kernel register cap

### Why

All the backtest wrappers currently use:

- `__launch_bounds__(128)`

See:

- `double_crossover.cu:4082-4864`

The official CUDA Programming Guide explicitly says the two-argument form can improve performance in some cases, and that the best `minBlocksPerMultiprocessor` must be found by analysis/measurement.

### Concrete test plan

Try these on a branch, kernel by kernel:

- current: `__launch_bounds__(128)`
- test: `__launch_bounds__(128,2)`

I would test first on:

- `double_cross_backtest_tm_flex_f32`
- `double_cross_backtest_tm_flex_f32_fast_warp_dedup`
- `double_cross_backtest_tm_flex_f32_lockstep_time`
- new M17/M22 specializations

### Why `minBlocksPerMultiprocessor=2`

It is a reasonable first experiment because your current 64-reg generic kernels already allow 8 resident 128-thread blocks on Ada, so this is not forcing an unrealistically high occupancy target. It simply gives the compiler a stronger hint about acceptable register usage.

### Follow-up experiment

If launch-bounds tuning is inconclusive, create a second branch that replaces `__launch_bounds__` with a per-kernel register cap, for example:

```cpp
__global__ __maxnreg__(56)
void double_cross_backtest_tm_flex_f32(...)
```

Then compare:

- runtime
- any spills / local memory growth
- achieved occupancy

I would try:

- `56`
- `60`

on the generic kernels first.

### Important caution

Do **not** combine `__launch_bounds__` and `__maxnreg__` on the same kernel. The compiler does not allow that.

---

## 6. Pack invariant scalar parameters into a `__grid_constant__` params struct

### Why

The wrappers have very long parameter lists:

- `double_crossover.cu:4082-4864`

CUDA 13.2 supports `__grid_constant__`, which prevents the compiler from creating per-thread copies of a kernel parameter object.

This is worth testing here because the backtest kernels take many invariant scalar parameters:

- sizes
- offsets
- flags
- thresholds
- `M`
- `metrics_out_stride`
- `pair_count`
- `compact_output`

### Suggested refactor

```cpp
struct DoubleCrossParams {
    int T;
    int Pf_tile, Ps_tile;
    int fast_tm_stride, slow_tm_stride;
    int Pf_total, Ps_total;
    int f_offset, s_offset;
    int use_regime, use_regime_hybrid;
    float regime_range_entry_rel, regime_range_exit_rel;
    int first_valid, regime_first_valid;
    int eval_start, eval_end;
    float commission, eps_rel, hurdle_tau, stop_log, take_log;
    unsigned int flags;
    int M, metrics_out_stride;
    int pair_count, compact_output;
};

extern "C" __global__
__launch_bounds__(128)
void double_cross_backtest_tm_flex_f32(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    const float* __restrict__ ret1,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    const float* __restrict__ regime_filter,
    const int* __restrict__ pair_index_map,
    const __grid_constant__ DoubleCrossParams p,
    float* __restrict__ metrics_out)
{
    ...
}
```

### Priority

Medium. I would do this **after** fixed M17/M22 specializations and lockstep broadcasts.

---

## 7. If lockstep remains important, bucket pairs by `start_t`

### Why

The lockstep path currently starts at:

- `loop_start = LOCKSTEP_TIME ? first_valid : start_t`

See:

- no-risk: `double_crossover.cu:3605-3611`
- risk: `double_crossover.cu:3760-3767`

That means the lockstep loop still carries this divergence point:

```cpp
if (t < start_t) continue;
```

So if lanes in a warp have a wide spread of `start_t`, part of the warp stays inactive while the earliest-start lanes are already doing work.

### Recommendation

When you build compact pair maps, optionally bucket pairs by coarse `start_t` ranges, while preserving dense/pf-major order within each bucket.

For example:

- bucket width: 8 or 16 bars
- stable sort key: `(start_t_bucket, dense_pair)`

This improves lockstep utilization without sacrificing fast-row locality too much.

### Priority

Medium-low. I would only do this after the easier specialization work.

---

## 8. Optional, larger redesign: metric-major output layout

### Why

The full-row writer stores a row per pair:

- `double_crossover.cu:3979-4030`

That is convenient, but the store pattern is stride-`metrics_stride` across threads.

The later top-k/selection kernels also read row-major metric rows:

- for example `double_crossover.cu:5599-5821`

For M17 and M22, a metric-major layout (`metric x pair`) would produce much more coalesced writes and reads.

### Why I am not making this a top recommendation

Because the backtest kernel’s dominant cost is usually the **time loop**, not the final metric write tail.

So I would only pursue this if:

- your windows are short, or
- your top-k/constraint passes dominate end-to-end time

### Priority

Low for kernel-only tuning. Medium for end-to-end pipeline tuning.

---

## Additional third-pass findings

### 1. Fill the missing routing matrix: combined lockstep + dedup, and optimized `windowed` variants

The current wrapper matrix covers:

- baseline generic: `double_cross_backtest_tm_flex_f32` at `double_crossover.cu:4081-4122`
- baseline `windowed`: `double_cross_backtest_tm_flex_f32_windowed` at `4124-4168`
- generic `FAST_WARP_DEDUP`: `4170-4211`
- generic `LOCKSTEP_TIME`: `4213-4254`

But it does **not** cover:

- a generic **combined** `LOCKSTEP_TIME + FAST_WARP_DEDUP` wrapper
- any optimized `windowed` variants beyond the generic baseline

That means a full-row M17/M22 workload can still miss useful speedups even before exact-M specialization is applied.

### Why this matters

For your target workload, the routing gaps create two practical problems:

1. a dynamic M17/M22 call can choose **lockstep** or **dedup**, but not both, unless it happens to hit one of the narrower specializations
2. a `windowed` run currently loses:
   - warp dedup
   - lockstep
   - and any future exact M17/M22 specialization

### Minimal implementation

Add the missing generic combined wrapper:

```cpp
extern "C" __global__
__launch_bounds__(128)
void double_cross_backtest_tm_flex_f32_lockstep_time_fast_warp_dedup(
    /* same signature as double_cross_backtest_tm_flex_f32 */
){
    double_cross_backtest_tm_flex_f32_body<true, true, false>(
        /* same arguments */
    );
}
```

Then add the same matrix for the `windowed` path:

```cpp
extern "C" __global__
__launch_bounds__(128)
void double_cross_backtest_tm_flex_f32_windowed_lockstep_time_fast_warp_dedup(
    /* same signature as the current windowed wrapper */
){
    double_cross_backtest_tm_flex_f32_body<true, true, false>(
        /* ... */,
        eval_start, eval_end,
        /* ... */
    );
}
```

At minimum, I would add:

- `windowed_fast_warp_dedup`
- `windowed_lockstep_time`
- `windowed_lockstep_time_fast_warp_dedup`
- `windowed_m17_regime_off_norisk`
- `windowed_m22_regime_off_norisk`

### Important nuance from the current M10 specializations

The current M10 wrappers do **not** preserve `FAST_WARP_DEDUP` consistently:

- `double_crossover.cu:4506` uses `double_cross_backtest_tm_flex_f32_body<true, true, true, 0, 0>(...)`
- `4552`, `4597`, and `4643` use `FAST_WARP_DEDUP=false`

So for M17/M22 you should benchmark:

- lockstep only
- lockstep + dedup

instead of assuming dedup always helps.

---

### 2. Move M17/M22 to a true fixed-M / fixed-flag body, not just literal-`M` wrappers

The current generic body still carries runtime variability from:

- `M` / `metrics_out_stride` at `3470-3482` and `3589-3591`
- `TRADE_ON_NEXT_BAR` at `3543`, then `t_sig = t - trade_offset` at `3613` and `3769`
- `STRAT_NO_FLIP` at `3651-3683` and `3807-3839`
- `STRAT_SIGNED_EXPOSURE` at `3990` / `4014`
- `LONG_ONLY` inside the sign helpers at `2951-2955`, `2994-2997`, `3008-3016`, and `3029-3054`

For **M17** and **M22**, most of the `need_*` logic is effectively constant. That means the best M17/M22 version is not just “the same body with `M=17` or `M=22`.” It is a **different body shape**.

### Stronger template shape

```cpp
template<
    int M_FIXED,
    bool FAST_WARP_DEDUP,
    bool LOCKSTEP_TIME,
    bool LONG_ONLY,
    bool TRADE_ON_NEXT_BAR,
    bool SIGNED_EXPOSURE,
    bool NO_FLIP,
    int REGIME_MODE,
    int RISK_MODE,
    int EPS_MODE,
    int COMMISSION_MODE>
static __device__ __forceinline__
void double_cross_backtest_tm_fixed_f32_body(/* same args, minus dynamic M/stride */);
```

### Why these flags are worth specializing

#### `TRADE_ON_NEXT_BAR`

If this is fixed off in a common route:

- `trade_offset` becomes a compile-time `0`
- `t_sig = t - trade_offset` collapses to `t`
- `start_t` calculation simplifies

Relevant lines:

- `double_crossover.cu:3543-3545`
- `3613`
- `3769`

#### `STRAT_SIGNED_EXPOSURE`

When signed exposure is off, the kernel still accumulates `sum_pos`:

- `double_crossover.cu:3744-3746`
- `3921-3923`

but then writes slot 6 as zero:

- `3990`
- `4014`

For a fixed `SIGNED_EXPOSURE=false` contract, remove:

- `sum_pos`
- `net_expo`
- the slot-6 write path

#### `LONG_ONLY`

When `LONG_ONLY=true`, the negative side of the sign logic disappears. That also reduces or removes:

- short-side accumulation
- some state transitions
- some range/regime sign cases

#### `NO_FLIP`

If `NO_FLIP` is common and fixed, the sign-change state machine simplifies because direct long↔short transitions are impossible.

### Recommended practical matrix

I would not generate every combination. I would start with:

- `TRADE_ON_NEXT_BAR=false`
- `STRAT_SIGNED_EXPOSURE=false`

and only then add:

- `LONG_ONLY=true`
- `NO_FLIP=true`

if those dominate your production workload.

---

### 3. Convert the hot loop to pointer-walking

The current loop rebuilds indexed addresses through:

- `ld_tm()` at `3100-3105`
- `ld_tm_fast_warp_dedup()` at `3108-3126`

with hot call sites at:

- `3614-3617`
- `3770-3773`

That means each bar repeatedly rebuilds address expressions of the form:

- `t_sig * fast_stride + pf_local`
- `t_sig * slow_stride + ps_local`
- `lr[t]`
- `ret1[t]`

A pointer-walking version is worth A/B testing, especially in the full-row register-heavy kernels.

### Example

```cpp
const int sig0 = loop_start - trade_offset;

const float* fast_p = fast_ma_T + (size_t)sig0 * fast_stride + pf_local;
const float* slow_p = slow_ma_T + (size_t)sig0 * slow_stride + ps_local;
const float* lr_p   = lr + loop_start;
const float* ret_p  = ret1 + loop_start;

for (int t = loop_start; t < end_t; ++t) {
    const float f = FAST_WARP_DEDUP
        ? ld_fast_ptr_warp_dedup(fast_p, pf_local)
        : *fast_p;
    const float s = *slow_p;
    const float lr_t = *lr_p;
    const float ret1_t = *ret_p;

    // existing body...

    fast_p += fast_stride;
    slow_p += slow_stride;
    ++lr_p;
    ++ret_p;
}
```

I would not assume this always wins — nvcc may already strength-reduce some of the index math — but it is one of the most plausible remaining kernel-local experiments for M17/M22.

### Related micro-optimization that becomes free in a fixed-M body

Because `pos` / `pos_bar` only take values `-1`, `0`, or `1`, these lines:

- `double_crossover.cu:3744-3746`
- `3921-3923`

can become:

```cpp
sum_abs_pos += (pos_bar != 0);
```

instead of:

```cpp
sum_abs_pos += abs(pos_bar);
```

This is small, but it sits in the hot loop.

---

### 4. Use a dedicated full-row M17/M22 loop shape

For **M17** and **M22**, the following are always on:

- max drawdown
- stats / variance
- exposure
- downside deviation
- ulcer
- omega
- kappa tau
- side means
- underwater metrics

And for **M22**, trade-summary metrics are also always on.

That means the full-row specialized loop should stop looking like the dynamic-M body. The point of the specialization is not only to dead-code-eliminate the writer. It is to remove **runtime `need_*` branches inside the loop**.

Relevant dynamic sites:

- `double_crossover.cu:3474-3482`
- `3589-3591`
- `3695-3714`
- `3716-3757`
- `3872-3890`
- `3893-3934`

### Concrete simplification

In an M17/M22 fixed body:

- replace `n` and `obs` with a single `bars` counter
- make the downside / omega / kappa block unconditional
- make the drawdown / ulcer / underwater block unconditional
- keep `closed_trades` / `gross_profit` / `gross_loss` only in the M22 body

Sketch:

```cpp
constexpr bool NEED_TRADE_METRICS = (M_FIXED >= 22);

int bars = 0;
float mean = 0.0f, m2 = 0.0f;
float sum_neg_sq = 0.0f, sum_neg_cu = 0.0f;
float ulcer_sum_sq = 0.0f;
float sum_pos_r = 0.0f, sum_neg_abs_r = 0.0f;
int count_neg = 0;
int sum_abs_pos = 0;
int sum_pos = 0;  // only if SIGNED_EXPOSURE
int long_obs = 0, short_obs = 0;
float long_sum_r = 0.0f, short_sum_r = 0.0f;

for (int t = loop_start; t < end_t; ++t) {
    // signal update / position transition
    // log_eq update
    // drawdown / ulcer / underwater: unconditional
    // stats / downside / omega / kappa / exposure / side means: unconditional
    bars += 1;
}
```

This is the loop shape I would actually target for the “full single-pass does maximum useful work” contract.

---

### 5. Make compact pair maps warp-friendly, not just pf-major

The earlier recommendation was to keep compact `pair_index_map` output in dense / pf-major order. That still stands, but on a third pass I would go one step further:

- sort by `(start_t_bucket, pf_local, ps_local)`
- optionally pad each `pf_local` bucket to a warp boundary if the wasted rows are small

Why this helps:

1. `FAST_WARP_DEDUP` benefits when a warp contains as few distinct `pf_local` values as possible
2. `LOCKSTEP_TIME` benefits when a warp contains similar `start_t` values
3. the combination makes it more likely that a warp reaches a stable active set quickly and stays converged for the long middle of the time loop

Relevant kernel decode lines:

- pair decode: `3497-3504`
- `start_t`: `3541-3545`
- fast-row dedup load sites: `3614-3615`, `3770-3771`

### Practical construction rule

For compact valid-pair maps used by search / ranking routes:

1. compute `start_t` (or a coarse bucket) on the host
2. stable-sort by:
   - `start_t_bucket`
   - `pf_local`
   - `ps_local`
3. if a `pf_local` bucket is large and hot, consider padding it to a multiple of 32 so most warps stay within one fast row

I would only pad when the waste is small relative to the expected time-loop savings.

---

### 6. Emit `score_out` and `feasible_out` directly from the backtest finalizer

This is the biggest end-to-end opportunity that was missing from the original document.

You already compute the raw ingredients for ranking and constraints in the backtest kernel, but the selection path later re-reads rows and recomputes them through:

- `objective_score_f32()` at `4871-4930`
- `passes_constraints_f32()` at `4964-5051`

Those functions are then used repeatedly in the selection kernels:

- `5154-5221`
- `5333-5420`
- `5434-5527`
- `5599-5821`

From the cubin metadata, the biggest pair-selection kernels are large:

- `select_topk_from_pairs_tm_f32_v2`: **44,672 B**
- `select_topk_from_candidates_tm_f32_v2`: **44,544 B**

and their shared-memory sections are `0x2440` bytes = **9,280 B** per block.

### Better shape for an end-to-end single-pass pipeline

If ranking follows immediately after backtest, add optional side outputs:

```cpp
float* __restrict__ score_out,
unsigned char* __restrict__ feasible_out
```

and emit them at the backtest finalizer:

```cpp
if (score_out) {
    score_out[row_idx] = objective_from_live_state(
        log_eq, max_log_dd, mean, std, downside_dev, omega, hurdle_tau, objective);
}

if (feasible_out) {
    feasible_out[row_idx] = passes_constraints_from_live_state(
        trades, max_log_dd, mean, std, exposure, net_expo,
        downside_dev, ulcer_index, omega, hurdle_tau,
        /* thresholds */) ? 1 : 0;
}
```

This lets the later top-k stage operate on:

- `score_out`
- `feasible_out`
- pair indices

instead of re-deriving the same values from full rows.

You do **not** have to give up full-row output to do this. The cheap version is “keep full metrics, but also emit score/mask.”

---

### 7. Final-pass correctness / contract issues

#### A. `max_abs_net_exposure` is semantically wrong when `STRAT_SIGNED_EXPOSURE` is off

The writer zeroes slot 6 unless the flag is enabled:

- `double_crossover.cu:3990`
- `4014`

But the constraints later read slot 6 unconditionally:

- `5031-5034`

So if `use_max_abs_net_exposure` is active while `STRAT_SIGNED_EXPOSURE` is off, the constraint will always see zero and will incorrectly pass.

### Fix options

The cleanest options are:

1. **Host-side contract**: if `use_max_abs_net_exposure`, require `STRAT_SIGNED_EXPOSURE`
2. **Kernel contract**: always store true net exposure in slot 6 and let presentation hide it if desired
3. **Separate side output**: emit a hidden `net_exposure_out` for constraint/ranking use

I would do option 1 first because it is lowest risk.

#### B. `metrics_out_stride` can underflow the row contract

`metrics_stride` is derived at:

- `double_crossover.cu:3470`

but the writer still uses `M` to decide how many elements to store:

- `4007-4029`

So if `0 < metrics_out_stride < M`, rows can overlap in memory.

I would add a host-side validation:

```cpp
if (metrics_out_stride > 0 && metrics_out_stride < M) {
    // reject configuration
}
```

For fixed M17/M22 wrappers, ignore runtime `metrics_out_stride` and hard-wire it.

#### C. Accuracy cleanup: prefer `expm1f(x)` to `expf(x) - 1.0f`

These two sites currently use `expf(x) - 1.0f`:

- trade summary: `double_crossover.cu:3421`
- final pnl: `3942`

Use:

```cpp
const float trade_ret = expm1f(trade_log_ret);
const float pnl = expm1f(log_eq);
```

This is mostly an **accuracy** improvement, not a speed optimization, but it is a free cleanup and matches the rest of the kernel, which already uses `log1pf()` / `expm1f()` carefully elsewhere.

---

### 8. Lower-priority runtime A/Bs that still fit this kernel family

These are not first-line recommendations, but they fit the observed kernel shape:

1. **`cudaFuncSetCacheConfig` / L1-favoring carveout**
   - the backtest kernels do not appear to use static shared memory
   - an L1-favoring preference is therefore a reasonable A/B test

2. **L2 access-policy window / persisting cache**
   - `lr`, `ret1`, and `regime_filter` are reused across many launches
   - if you launch many kernels over the same arrays, an L2 persistence experiment is reasonable

3. **Nsight Compute baseline sets**
   - if you do one final measurement pass, focus on:
     - achieved occupancy
     - local memory / spills
     - branch divergence
     - L1 / L2 hit rates
     - instruction mix in the backtest vs top-k stages

These are lower priority than fixed M17/M22 bodies and route coverage, but they are compatible with your current architecture.

---

## Accuracy / semantic issues to fix or verify

## 1. Definite bug: calmar decode mismatch (`M < 15` vs `M < 17`)

### Where

Writer uses:

- `double_crossover.cu:3983`
- `double_crossover.cu:4007`

```cpp
const bool search_row = M < 17;
```

Calmar scorer uses:

- `double_crossover.cu:4895`

```cpp
const bool search_row = M < 15;
```

### Why this is wrong

For `M == 15` or `M == 16`, the writer emits:

- slot 0 in log space (`log_eq`)
- slot 2 in log drawdown space (`max_log_dd`)

but calmar decodes them as if they were arithmetic `pnl` and `max_dd`.

That makes calmar wrong for those row widths.

### Fix

Centralize the rule:

```cpp
static __device__ __forceinline__ bool metrics_row_uses_log_space(int M) {
    return M < 17;
}
```

Then use that helper in:

- the writer (`double_crossover.cu:3983`, `4007`)
- `objective_score_f32()` (`double_crossover.cu:4895`)
- any constraint logic that interprets slot 0 or 2 semantically

---

## 2. Definite bug: `use_max_dd` constraint is wrong for reduced rows

### Where

Writer uses log drawdown for reduced rows:

- `double_crossover.cu:3981-3986`
- `double_crossover.cu:4007-4010`

Constraint path reads slot 2 directly:

- `double_crossover.cu:4999-5003`

```cpp
const float dd = metrics[base + 2];
if (!(dd <= max_dd)) return false;
```

### Why this is wrong

When `M < 17`, slot 2 is `max_log_dd`, not arithmetic `max_dd`.

For ranking by drawdown, this is usually okay because the transform is monotonic.

For **thresholding** (`dd <= max_dd_threshold`), it is not okay unless the caller also converts the threshold into log space.

### Fix

Use the same decode helper as calmar:

```cpp
const bool search_row = metrics_row_uses_log_space(M);
const float dd = search_row
    ? -expm1f(-metrics[base + 2])
    : metrics[base + 2];
if (!(dd <= max_dd)) return false;
```

---

## 3. Likely bug / missing implementation: `regime_range_exit_rel` is unused

### Where

The parameter is threaded through the API and wrappers:

- `double_crossover.cu:2949`
- `double_crossover.cu:3027`
- `double_crossover.cu:3450`
- `double_crossover.cu:4098-4864`

But the regime helpers never actually use it in their logic.

### Why this matters

If your intended semantics were entry/exit hysteresis for range mode, the current GPU path is missing that behavior.

### Important implementation note

You cannot implement a true separate exit threshold in the current helper signature because the helper does not know the **current position**.

That means a real fix requires threading position into the regime decision, e.g.:

```cpp
int desired_sign_mode_f32(..., int current_pos, float regime_range_entry_rel, float regime_range_exit_rel)
```

Then the range-mode branch can do something like:

- use `entry_rel` when entering from flat or flipping
- use `exit_rel` when deciding whether to stay in an existing position

### Recommendation

Either:

1. implement proper hysteresis by passing `pos`, or
2. remove the dead parameter from the ABI if the exit threshold is not intended

---

## 4. Validate `commission` range on the host

### Where

- `double_crossover.cu:3473`

```cpp
const float log_comm = has_commission ? log1pf(-commission) : 0.0f;
```

### Why

This assumes `0 <= commission < 1`.

The earlier strategy-program kernels clamp commission more defensively (`double_crossover.cu:392-393`, `854-855`), but the flex backtest path does not.

### Recommendation

Validate this on the host before kernel launch.

---

## 5. Semantic consistency check: Sharpe ignores `hurdle_tau`

### Where

Sharpe objective:

- `double_crossover.cu:4884-4888`

```cpp
const float mean = metrics[base + 3];
const float std  = metrics[base + 4];
return (std > 0.0f) ? (mean / std) : 0.0f;
```

Sortino does use `hurdle_tau`:

- `double_crossover.cu:4916-4924`

```cpp
const float mean_excess = mean - hurdle_tau;
```

### Recommendation

Verify that this is intentional.

If `hurdle_tau` is meant to be the excess-return hurdle for both Sharpe and Sortino, then Sharpe should likely be:

```cpp
(mean - hurdle_tau) / std
```

I am not calling this a definite bug because the rest of the file consistently computes Sharpe as `mean / volatility`, which may be your chosen definition.

---

## 6. Semantics check: stop/take is applied after the bar return

### Where

Risk path:

- `double_crossover.cu:3843-3869`

The flow is:

1. apply `lr[t]`
2. update `trade_log_ret`
3. test `stop_log` / `take_log`
4. flatten if hit

### Recommendation

This is correct for a close-to-close cumulative-return stop model.

If you intended intrabar stop logic or high/low stop logic, this is not equivalent.

---

## Concrete implementation plan

## A. Centralize reduced-row log-space decoding

Add this helper near the writer/objective code:

```cpp
static __device__ __forceinline__ bool metrics_row_uses_log_space(int M) {
    return M < 17;
}
```

Then change:

- `double_crossover.cu:3983`
- `double_crossover.cu:4007`
- `double_crossover.cu:4895`
- `double_crossover.cu:4999-5003`

Minimal patch shape:

```cpp
const bool search_row = metrics_row_uses_log_space(M);
```

and in constraints:

```cpp
const bool search_row = metrics_row_uses_log_space(M);
const float dd = search_row ? -expm1f(-metrics[base + 2]) : metrics[base + 2];
```

---

## B. Add fixed M17/M22 wrappers now, then refactor to a fixed-M templated body

### Step 1: add wrappers first

This is the fastest path to a measurable result.

Replicate the M10 wrapper pattern from:

- `double_crossover.cu:4472-4650`

for M17 and M22.

### Step 2: if the gains are real, refactor to a fixed-M body

This will reduce maintenance risk and give the compiler even better constants.

---

## C. Add a macro to generate specialized wrappers cleanly

Because the wrapper matrix will grow, I would strongly recommend generating them with a macro.

Example:

```cpp
#define DECLARE_DC_FIXED_M_KERNEL(NAME, FAST_DEDUP, LOCKSTEP, FIXED_M, REGIME_MODE, RISK_MODE, EPS_MODE, COMM_MODE) \
extern "C" __global__ __launch_bounds__(128) void NAME( \
    const float* __restrict__ fast_ma_T, \
    const float* __restrict__ slow_ma_T, \
    const float* __restrict__ lr, \
    const float* __restrict__ ret1, \
    int T, \
    int Pf_tile, int Ps_tile, \
    int fast_tm_stride, int slow_tm_stride, \
    int Pf_total, int Ps_total, \
    int f_offset, int s_offset, \
    const int* __restrict__ fast_periods, \
    const int* __restrict__ slow_periods, \
    const float* __restrict__ regime_filter, \
    int use_regime, \
    int use_regime_hybrid, \
    float regime_range_entry_rel, \
    float regime_range_exit_rel, \
    int first_valid, \
    int regime_first_valid, \
    float commission, \
    float eps_rel, \
    float hurdle_tau, \
    float stop_log, \
    float take_log, \
    unsigned int flags, \
    int M, \
    int metrics_out_stride, \
    const int* __restrict__ pair_index_map, \
    int pair_count, \
    int compact_output, \
    float* __restrict__ metrics_out) \
{ \
    (void)M; \
    (void)metrics_out_stride; \
    if constexpr (COMM_MODE == 0) (void)commission; \
    double_cross_backtest_tm_flex_f32_body<FAST_DEDUP, LOCKSTEP, true, REGIME_MODE, RISK_MODE, EPS_MODE, COMM_MODE>( \
        fast_ma_T, slow_ma_T, lr, ret1, \
        T, Pf_tile, Ps_tile, fast_tm_stride, slow_tm_stride, \
        Pf_total, Ps_total, f_offset, s_offset, \
        fast_periods, slow_periods, regime_filter, \
        use_regime, use_regime_hybrid, regime_range_entry_rel, regime_range_exit_rel, \
        first_valid, regime_first_valid, \
        (COMM_MODE == 0 ? 0.0f : commission), eps_rel, hurdle_tau, stop_log, take_log, \
        flags, FIXED_M, FIXED_M, \
        pair_index_map, pair_count, compact_output, metrics_out); \
}
```

Then instantiate:

```cpp
DECLARE_DC_FIXED_M_KERNEL(double_cross_backtest_tm_flex_f32_m17_regime_off_norisk,
                          true, false, 17, 0, 0, -1, -1)
DECLARE_DC_FIXED_M_KERNEL(double_cross_backtest_tm_flex_f32_m22_regime_off_norisk,
                          true, false, 22, 0, 0, -1, -1)
DECLARE_DC_FIXED_M_KERNEL(double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk,
                          true, true, 17, 0, 0, -1, -1)
DECLARE_DC_FIXED_M_KERNEL(double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk,
                          true, true, 22, 0, 0, -1, -1)
```

and so on for `eps0`, `nocomm`, `eps0_nocomm`.

---

## D. Add a runtime router for common M17/M22 paths

The point of specializations is not to replace the generic kernel. The point is to route the **high-frequency contracts** to lean kernels.

Suggested host-side routing logic:

```cpp
bool no_risk = !(flags & STRAT_STOP_LOSS) && !(flags & STRAT_TAKE_PROFIT);
bool regime_off = !use_regime && !use_regime_hybrid;
bool eps0 = (eps_rel == 0.0f);
bool nocomm = (commission == 0.0f);

if (M == 17 && regime_off && no_risk) {
    if (lockstep) {
        if (eps0 && nocomm) launch lockstep_m17_regime_off_norisk_eps0_nocomm;
        else if (eps0)      launch lockstep_m17_regime_off_norisk_eps0;
        else if (nocomm)    launch lockstep_m17_regime_off_norisk_nocomm;
        else                launch lockstep_m17_regime_off_norisk;
    } else {
        if (eps0 && nocomm) launch m17_regime_off_norisk_eps0_nocomm;
        else if (eps0)      launch m17_regime_off_norisk_eps0;
        else if (nocomm)    launch m17_regime_off_norisk_nocomm;
        else                launch m17_regime_off_norisk;
    }
} else if (M == 22 && regime_off && no_risk) {
    ... same pattern for M22 ...
} else {
    launch generic fallback;
}
```

If you already have runtime heuristics for lockstep selection, plug into that.

---

## E. Add warp-uniform load helpers in the lockstep path

Patch points:

- `double_crossover.cu:3687-3692`
- `double_crossover.cu:3716-3724`
- `double_crossover.cu:3843-3850`
- `double_crossover.cu:3893-3901`

Minimal change:

```cpp
const float lr_t = LOCKSTEP_TIME ? warp_uniform_ld_f32(lr, t)
                                 : ld_ro_1d(lr, (size_t)t);
const float ret1_t = LOCKSTEP_TIME ? warp_uniform_ld_f32(ret1, t)
                                   : ld_ro_1d(ret1, (size_t)t);
```

Then replace the later loads with `ret1_t`.

---

## F. Make compact pair maps preserve dense/pf-major ordering

This is a host-side change, but it directly affects kernel efficiency.

Pseudo:

```cpp
std::vector<int> pair_index_map;
pair_index_map.reserve(Pf_tile * Ps_tile);
for (int pf = 0; pf < Pf_tile; ++pf) {
    int period_f = fast_periods[f_offset + pf];
    for (int ps = 0; ps < Ps_tile; ++ps) {
        int period_s = slow_periods[s_offset + ps];
        if (period_f < period_s) {
            pair_index_map.push_back(pf * Ps_tile + ps);
        }
    }
}
```

This keeps the warp layout friendly for `FAST_WARP_DEDUP`.

---

## G. Optional packed-returns experiment

Because both `lr[t]` and `ret1[t]` are used in the full-row path every iteration, you may want to test packing them into a single `float2` stream instead of separate arrays.

Current producer:

- `double_crossover.cu:3175-3195`

Potential replacement:

```cpp
struct ReturnPair { float lr; float ret1; };
```

This is an optional experiment, not a first-line recommendation.

---

## Suggested validation and profiling checklist

After each change, check:

1. kernel wall time
2. compiler-reported resource usage
3. achieved occupancy
4. branch divergence
5. local memory / spills
6. L1 / L2 hit rates on the backtest kernels
7. whether the backtest or the top-k stage is now dominant

I would measure these variants first:

- generic M17
- generic M17 `lockstep + fast_warp_dedup`
- specialized non-lockstep M17
- specialized lockstep M17
- specialized lockstep + dedup M17
- generic M22
- generic M22 `lockstep + fast_warp_dedup`
- specialized non-lockstep M22
- specialized lockstep M22
- specialized lockstep + dedup M22

If `windowed` is common, add:

- generic windowed M17/M22
- specialized windowed M17/M22
- specialized windowed `lockstep + dedup`

If ranking follows backtest immediately, also benchmark:

- current path: `backtest -> topk`
- fused path: `backtest(+score_out/+feasible_out) -> topk_on_scores`

For each one, benchmark:

- current launch bounds
- `__launch_bounds__(128,2)`
- optional register-capped branch
- current index-load loop
- pointer-walking loop

---

## Priority ranking

### Highest priority

1. **Add true fixed M17/M22 kernels**
2. **Fill the missing route matrix** (`lockstep + dedup`, and `windowed` optimized paths if used)
3. **Promote common flags to compile-time contracts** (`TRADE_ON_NEXT_BAR`, `STRAT_SIGNED_EXPOSURE`, optionally `LONG_ONLY` / `NO_FLIP`)
4. **Fix reduced-row decode bugs and contract bugs**
5. **Preserve warp-friendly pair-map order** (`start_t` bucket + `pf_local` locality)

### Medium priority

6. **Pointer-walking hot-loop experiment**
7. **Add lockstep warp-uniform broadcasts**
8. **Emit `score_out` / `feasible_out` directly from the backtest finalizer**
9. **A/B `__launch_bounds__(128,2)`**
10. **Try `__grid_constant__` params struct**

### Lower priority / only if profiling points there

11. **Warp-aligned padding for compact pair maps**
12. **Packed `float2` returns**
13. **Metric-major output layout**
14. **Runtime cache / L2 persistence A/Bs**
15. **Broader flag-specialization matrix beyond the common routes**

---

## Final conclusion

For the workload you care about — **single-pass 17-row and 22-row double-MA backtests** — the clearest next optimization is still to extend your existing M10 specialization strategy to **true fixed M17/M22 contracts**.

But after a final third pass, I would no longer describe the remaining headroom as only “add M17/M22 wrappers.” There are four distinct remaining opportunities:

1. **route coverage is incomplete**
   - generic `LOCKSTEP_TIME + FAST_WARP_DEDUP` is missing
   - `windowed` only has a generic baseline wrapper

2. **full-row kernels still carry too much runtime flag logic**
   - `TRADE_ON_NEXT_BAR`
   - `STRAT_SIGNED_EXPOSURE`
   - and possibly `LONG_ONLY` / `NO_FLIP`

3. **the hot loop still has avoidable overhead**
   - repeated index math
   - repeated dynamic `need_*` structure
   - pair maps that can be made more warp-friendly

4. **the pipeline still recomputes ranking work after backtest**
   - objective / constraint scoring can be emitted while the raw state is already live

If you implement only one optimization wave from this document, I would implement:

- true fixed **M17** and **M22** kernels
- both **non-lockstep** and **lockstep**
- add the missing **generic lockstep + dedup** route
- fix **`TRADE_ON_NEXT_BAR=false`** and **`STRAT_SIGNED_EXPOSURE=false`** as compile-time contracts
- optionally emit **`score_out` / `feasible_out`**

That is the most plausible path to a real speedup on RTX 4090 without changing the algorithm or giving up the “do maximum useful work in one pass” design.

---

## References consulted

- NVIDIA Ada GPU Architecture Tuning Guide (current online edition)
  - https://docs.nvidia.com/cuda/ada-tuning-guide/index.html
- NVIDIA CUDA C++ Programming Guide (Release 13.2)
  - https://docs.nvidia.com/cuda/cuda-programming-guide/
- CUDA C++ Programming Guide, C/C++ language extensions
  - `__launch_bounds__`
  - `__maxnreg__`
  - `__grid_constant__`
  - https://docs.nvidia.com/cuda/cuda-programming-guide/05-appendices/cpp-language-extensions.html
- NVIDIA CUDA C++ Best Practices Guide (Release 13.2)
  - coalesced global memory access
  - divergence reduction
  - L2 access window / persistence guidance
  - https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/
- NVIDIA Nsight Compute Profiling Guide
  - https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html

