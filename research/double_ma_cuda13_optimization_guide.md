
# Double MA CUDA 13+ Optimization Guide
## Target: `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs` on RTX 4090

**Context**

- Reported benchmark: **76.919 ms** for `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`.
- Reported ALMA throughput: **250 million ALMA datapoints in 3.15 ms** on RTX 4090.
- That strongly suggests **MA generation is already very fast** and the remaining headroom is mostly in the **backtest kernel** and the **selection/search path**, not in ALMA itself.

This guide is based on a review of your current CUDA kernel and Rust launcher:

- `double_crossover.cu`
- `gpu_backtest_kernel.rs`

It is also written with **CUDA 13+** and **Ada / sm_89** in mind. I have not run Nsight Compute on your exact benchmark, so the recommendations below are **ranked by expected ROI**, not by measured speedup on your machine.

---

## 1. Executive summary

If I were optimizing this benchmark for maximum performance, I would do the work in this order:

1. **Split optimization into a cheap search pass and a tiny finalist pass.**
   - Right now the optimizer path still hardwires the **full 7-metric backtest** during search.
   - For most objectives, that means every pair-bar pays for logic that the objective does not use.
   - This is the single most likely source of large remaining speedup.

2. **Precompute simple returns (`ret1`) and remove `expm1f(lr_t)` from the per-pair, per-bar stats path.**
   - Your hot loop still does `step_r = pos * expm1f(lr_t)` in the stats path.
   - That puts a transcendental in the inner loop.
   - The clean fix is: keep log returns for PnL / drawdown / risk logic, but add a simple-return buffer for Sharpe / volatility stats.

3. **Retune the launch shape for the backtest family and add `__launch_bounds__` once block size is settled.**
   - The Rust launcher already estimates register-limited occupancy, but then still forces a **minimum block size of 256 or 512** for large work.
   - On Ada, register pressure is still a first-class limiter.
   - For this kernel family I would benchmark **128 and 256 first** and treat 512 as opt-in, not default.

4. **Replace the flat pair mapping with a warp-per-fast-row mapping in the lockstep-M7 path.**
   - Your current kernel already has good ideas (`valid_pair_map`, lockstep time, specialization, warp dedup path), but the hot path for this benchmark is likely the **lockstep M7 family**, not the separate `fast_warp_dedup` kernel.
   - A warp-per-fast-row mapping is the next structural rewrite with the highest ceiling.

5. **Only after that, try shared-memory time tiling with `memcpy_async` / `cp.async`.**
   - If Nsight Compute shows the kernel is still memory-stall dominated after the steps above, stage `lr`, `ret1`, and `regime` in shared memory time tiles first.
   - I would not start with MA-plane tiling.

6. **Then do the small but real micro-optimizations.**
   - Replace 64-bit accumulators that do not need to be 64-bit.
   - Remove `llabs`.
   - Consider compact metric output when `pair_index_map` is active.
   - Add vectorized `float4` stores when using padded M7 stride 8.
   - Consider specialized variants for `trade_on_next_bar`, `long_only`, and `commission == 0`.

---

## 2. What your code already does well

Before talking about changes, it is worth calling out what is already good in the current implementation.

### 2.1 The launcher/runtime side is already quite mature

Your Rust side already has several strong performance-oriented pieces:

- **Resident device buffers** and persistent scratch allocation.
- **`sm_89` cubin loading** with PTX fallback.
- **Cached valid-pair compaction** (`valid_pair_map`) instead of brute-force dense pair space.
- **Device-side top-k selection** and optional global top-k row reduction.
- **Heatmap accumulation on device**.
- **Specialized dispatch paths** for:
  - lockstep time,
  - M7,
  - regime mode specialization,
  - risk/no-risk specialization,
  - epsilon-zero specialization,
  - padded M7 metric stride.
- **Pipeline timing instrumentation** on the Rust side.

That is why I would *not* spend the next round on host orchestration first. The remaining headroom is probably not in “make Rust upload less” or “use graphs more”. It is mostly in the actual pair-bar loop.

### 2.2 The kernel already has the correct broad shape

Inside `double_crossover.cu`, you already have:

- time-major MA matrices,
- a lockstep-time variant,
- a specialized M7 family,
- a warp-dedup fast-row helper,
- CUB-based block top-k reducers,
- pair compaction support.

This is not a naive baseline kernel anymore. The next wins come from **reducing work inside the inner loop**, not from discovering a missing obvious optimization.

---

## 3. Why the backtest body is still the main target

The core of the current hot path is the templated backtest body:

- `double_cross_backtest_tm_flex_f32_body` around **`double_crossover.cu` ~3300-3589**
- `compute_log_returns_f32` around **`double_crossover.cu` ~3077-3086**
- `ld_tm_fast_warp_dedup` around **`double_crossover.cu` ~3030-3048**
- objective selection kernels around **`double_crossover.cu` ~4213+**
- launcher-side dispatch and block-size selection in **`gpu_backtest_kernel.rs` ~387-518, ~6365-6415**

For your benchmark size, the total pair-bar work is enormous. If you assume roughly:

- `58,300` valid pairs
- `~200,000` bars
- `~400` bars of warmup / first-valid offset

then the effective pair-bar count is roughly:

```text
58,300 * (200,000 - 400) = 11,636,680,000 pair-bars
```

That is the quantity dominating the runtime.

The consequence is simple:

- anything that saves **a few instructions per pair-bar** matters,
- anything that saves **an entire metric path per pair-bar** matters a lot,
- anything that only saves a few microseconds of host overhead probably does not move this benchmark much.

---

## 4. Highest-probability win: split the search pass from the finalist pass

## 4.1 What the current optimizer is doing

In the optimization path, the launcher still fixes:

```rust
let metrics_count: usize = METRICS_COUNT;
```

in `optimize_double_ma_gpu_kernel_impl` (around `gpu_backtest_kernel.rs` ~5473), where `METRICS_COUNT = 7`.

That means the search pass still computes the full M7 payload:

1. PnL
2. trades
3. max drawdown
4. mean
5. std
6. exposure
7. net exposure

and then the selectors compute the objective score from that buffer.

That is correct functionally, but expensive.

## 4.2 Why this matters so much

The backtest body does conditional work based on `need_trades`, `need_max_dd`, `need_stats`, `need_std`, and `need_expo`. But in the current optimizer path:

- `metrics_count` is fixed at 7,
- the M7 specialized kernels are favored for large workloads,
- so the compiler is effectively optimizing around **all metrics being required**.

In other words: **every pair-bar pays for all metric machinery during search**, even though the objective usually only needs a subset.

For example:

- **PnL objective** does **not** need Welford stats, std, exposure, or net exposure.
- **Max drawdown objective** does **not** need Welford stats or exposures.
- **Sharpe objective** needs stats, but does **not** need drawdown or exposures.
- **Volatility objective** needs std, but not drawdown or exposures.
- **Calmar objective** needs PnL and drawdown, but not exposure.

That is a lot of avoidable work across **11.6 billion pair-bars**.

## 4.3 The right architecture

Use **two passes**:

### Pass A: cheap search pass

Compute **only** what the objective and active constraints need.

Output either:

- a compact score row (`score + constraint fields`), or
- a compact objective-specific metric row.

Do top-k selection from that.

### Pass B: finalist pass

Recompute full M7 **only for the surviving finalists** (for example top-k or top-k plus a small safety margin).

This is the pass that produces the final `Metrics` payload you expose.

## 4.4 Why this is better than just lowering `M`

A very important subtlety:

changing runtime `M` from 7 to 1 is **not enough**.

Why:

- your current generic body still contains runtime conditionals like:
  - `const int need_stats = FULL_M7 ? 1 : (M > 3);`
  - `const int need_expo = FULL_M7 ? 1 : (M > 5);`
- if `M` is just a runtime value, you still carry more runtime branching and state than necessary,
- and the compiler cannot fully remove the unused code paths.

So the real win comes from:

- **a separate search-pass kernel family**, or
- **template specializations** where the objective requirements are compile-time known.

## 4.5 Objective-by-objective search requirements

Here is the minimal state I would use for the search pass.

| Objective | Search score | Must track in search pass | Can avoid full M7? | Can stay in log space for ranking? |
|---|---:|---|---|---|
| PnL | `log_eq` | `pos`, `log_eq`, commission transitions, plus constraints if any | yes | **yes** |
| Sharpe | `mean / std` | `pos`, `mean`, `m2`, `n` (and risk state if SL/TP enabled) | yes | n/a |
| Max DD | `-max_log_dd` | `pos`, `log_eq`, `log_peak`, `max_log_dd`, plus constraints if any | yes | **yes** |
| Calmar | `pnl / max_dd` | `pos`, `log_eq`, `log_peak`, `max_log_dd` | yes | **no** |
| Volatility | `-std` | `pos`, `mean`, `m2`, `n` (and risk state if SL/TP enabled) | yes | n/a |

### Important insight: use monotonic log-space scores where possible

You are already tracking log equity internally.

That gives you an extra optimization:

- **PnL ranking** by `pnl = exp(log_eq) - 1` is equivalent to ranking by **`log_eq`** because `exp(x) - 1` is monotonic.
- **Max drawdown ranking** by `-max_dd` is equivalent to ranking by **`-max_log_dd`**, because  
  `max_dd = 1 - exp(-max_log_dd)` is monotonic in `max_log_dd`.

So in the selection pass you can often avoid the final transcendental and just score on:

- `log_eq` for PnL
- `-max_log_dd` for MaxDD

Then only convert finalists to the public metric representation.

### Convert max drawdown constraints once on the host

If the user has a linear drawdown constraint:

```text
max_dd <= d
```

with `0 <= d < 1`, the equivalent log-space constraint is:

```text
max_log_dd <= -log1p(-d)
```

That means the selection pass can stay entirely in log-space for MaxDD objective and MaxDD constraints.

## 4.6 Important semantic note about Sharpe / volatility in your current code

In the current kernel, the stats path uses:

```cpp
const float step_r = (pos_bar == 0) ? 0.0f : (float)pos_bar * expm1f(lr_t);
```

This means the Sharpe / volatility path is based on **raw bar returns**, not explicitly cost-adjusted returns.

Commission updates affect `log_eq`, but **not** `step_r`.

That has two consequences:

1. If this is intentional, your **Sharpe/volatility search pass can be much cheaper**:
   - when stop-loss / take-profit is off,
   - you do not need `log_eq` at all for those objectives,
   - only the Welford state matters.

2. If this is **not** intentional, fix the metric definition first before optimizing around it.

Either way, it is worth making explicit.

## 4.7 Recommended search-pass API shape

A practical host-side representation:

```rust
#[derive(Clone, Copy, Debug)]
struct SearchPassSpec {
    need_trades: bool,
    need_max_dd: bool,
    need_stats: bool,
    need_exposure: bool,
    score_mode: SearchScoreMode,
}

#[derive(Clone, Copy, Debug)]
enum SearchScoreMode {
    LogPnl,
    Sharpe,
    NegLogMaxDd,
    Calmar,
    NegVolatility,
}

fn search_pass_spec(
    objective: ObjectiveKind,
    constraints: Option<&DoubleMaConstraints>,
) -> SearchPassSpec {
    let need_trades = constraints.and_then(|c| c.min_trades).is_some();
    let need_max_dd_constraint = constraints.and_then(|c| c.max_drawdown).is_some();

    match objective {
        ObjectiveKind::Pnl => SearchPassSpec {
            need_trades,
            need_max_dd: need_max_dd_constraint,
            need_stats: false,
            need_exposure: false,
            score_mode: SearchScoreMode::LogPnl,
        },
        ObjectiveKind::Sharpe => SearchPassSpec {
            need_trades,
            need_max_dd: need_max_dd_constraint,
            need_stats: true,
            need_exposure: false,
            score_mode: SearchScoreMode::Sharpe,
        },
        ObjectiveKind::MaxDrawdown => SearchPassSpec {
            need_trades,
            need_max_dd: true,
            need_stats: false,
            need_exposure: false,
            score_mode: SearchScoreMode::NegLogMaxDd,
        },
        ObjectiveKind::Calmar => SearchPassSpec {
            need_trades,
            need_max_dd: true,
            need_stats: false,
            need_exposure: false,
            score_mode: SearchScoreMode::Calmar,
        },
        ObjectiveKind::Volatility => SearchPassSpec {
            need_trades,
            need_max_dd: need_max_dd_constraint,
            need_stats: true,
            need_exposure: false,
            score_mode: SearchScoreMode::NegVolatility,
        },
        _ => unimplemented!(),
    }
}
```

Then dispatch to a dedicated search kernel family rather than forcing M7.

## 4.8 Recommended kernel-side shape

I would not start by building dozens of wrappers. I would build a **small dedicated search family** first:

```cpp
enum ScoreMode : int {
    SCORE_LOG_PNL      = 0,
    SCORE_SHARPE       = 1,
    SCORE_NEG_LOG_DD   = 2,
    SCORE_CALMAR       = 3,
    SCORE_NEG_VOL      = 4,
};

template <
    int SCORE_MODE,
    bool NEED_TRADES,
    bool NEED_MAX_DD,
    bool NEED_STATS,
    int REGIME_MODE = -1,
    int RISK_MODE   = -1,
    int EPS_MODE    = -1,
    bool LOCKSTEP   = false>
static __device__ __forceinline__
void double_cross_search_body(/* ... */, float* out_score, float* out_aux0, float* out_aux1)
{
    // Only keep the state required for the chosen objective and active constraints.
    // No exposure / net exposure here.
    // Final M7 is computed later for finalists only.
}
```

For a first implementation, you can keep the output row small but simple:

- `score`
- `trades` if min-trades constraint is active
- `max_log_dd` or `max_dd` if max-drawdown constraint is active

That is already dramatically cheaper than M7.

## 4.9 Expected effect

This is the change with the largest expected impact because it attacks:

- instruction count,
- register pressure,
- output bandwidth,
- selector bandwidth,
- and the number of live values in the inner loop.

For your benchmark, I would treat this as the **highest-probability major speedup**.

---

## 5. Second major win: precompute simple returns and remove `expm1f` from the inner stats loop

## 5.1 What the current kernel is doing

Inside the hot loop, the stats path currently does:

- around `double_crossover.cu` ~3467:
  ```cpp
  const float step_r = (pos == 0) ? 0.0f : (float)pos * expm1f(lr_t);
  ```

- and around `double_crossover.cu` ~3562:
  ```cpp
  const float step_r = (pos_bar == 0) ? 0.0f : (float)pos_bar * expm1f(lr_t);
  ```

So every pair-bar in the stats path pays for `expm1f`.

That is exactly the kind of operation I want out of the hot loop.

## 5.2 The clean fix

Add a second precomputed return buffer:

- `lr[t]   = log(price[t] / price[t-1])`
- `ret1[t] = price[t] / price[t-1] - 1`

Then use:

- `lr[t]` for:
  - PnL
  - drawdown
  - risk logic (`trade_log_ret`)
- `ret1[t]` for:
  - mean
  - variance
  - Sharpe
  - volatility

That moves the expensive transcendental out of the per-pair inner loop and into a **single one-time preprocessing pass**.

## 5.3 Improve the preprocessing kernel too

Your current preprocessing kernel is:

```cpp
lr_out[t] = (p > 0.0f && pm > 0.0f && isfinite(p) && isfinite(pm))
    ? logf(p) - logf(pm)
    : 0.0f;
```

That uses **two logs per bar**.

A better version is:

- compute the ratio once,
- use one log for `lr`,
- and reuse the same ratio for `ret1`.

### Suggested replacement

```cpp
extern "C" __global__
void compute_log_and_simple_returns_f32(
    const float* __restrict__ prices,
    int T,
    float* __restrict__ lr_out,
    float* __restrict__ ret1_out)
{
    for (int t = blockIdx.x * blockDim.x + threadIdx.x;
         t < T;
         t += blockDim.x * gridDim.x)
    {
        if (t == 0) {
            lr_out[0]   = 0.0f;
            ret1_out[0] = 0.0f;
            continue;
        }

        const float p  = prices[t];
        const float pm = prices[t - 1];

        if (p > 0.0f && pm > 0.0f && isfinite(p) && isfinite(pm)) {
            const float ratio = p / pm;
            lr_out[t]   = logf(ratio);
            ret1_out[t] = ratio - 1.0f;
        } else {
            lr_out[t]   = 0.0f;
            ret1_out[t] = 0.0f;
        }
    }
}
```

## 5.4 Kernel body change

Then the stats path becomes:

```cpp
const float bar_ret = ret1[t];
const float step_r  = (pos_bar == 0) ? 0.0f : (float)pos_bar * bar_ret;

n += 1;
const float delta = step_r - mean;
mean += delta / (float)n;
m2   += delta * (step_r - mean);
```

## 5.5 Why this is so attractive

This change is:

- conceptually simple,
- mathematically aligned with your current logic,
- local to the hot path,
- and synergistic with objective-specific search kernels.

It is especially valuable because:

- PnL / drawdown logic still keeps the precise log-return representation,
- stats logic stops paying for `expm1f`,
- and Sharpe / volatility search kernels can then run on `ret1` only.

## 5.6 Accuracy note

Mathematically:

```text
expm1(log(price_t / price_{t-1})) == price_t / price_{t-1} - 1
```

So this is the right replacement.

Numerically, the exact FP32 bit pattern may differ slightly from `expm1f(logf(ratio))`, but it should be very close. If ranking stability is critical, validate:

- top-1 identity,
- top-k overlap,
- and full metric tolerances on finalists.

---

## 6. Third major win: objective-specific kernels should specialize state, not just output width

This is separate from Section 4 because it is the difference between “good” and “actually fast”.

## 6.1 What to remove by objective

### PnL search kernel

Track only:

- `pos`
- `log_eq`
- transition commissions
- optional `trades`
- optional `max_log_dd` only if needed by constraints

You do **not** need:

- Welford state
- exposure
- net exposure
- final `expf(log_eq) - 1` for ranking

### MaxDD search kernel

Track only:

- `pos`
- `log_eq`
- `log_peak`
- `max_log_dd`
- optional `trades`

You do **not** need:

- Welford state
- exposure
- net exposure
- final `-expm1f(-max_log_dd)` for ranking

### Sharpe search kernel

Track only:

- `pos`
- `mean`
- `m2`
- `n`
- optional risk state if SL/TP enabled
- optional `trades`
- optional `max_log_dd` only if maxDD constraint is active

You do **not** need:

- exposure
- net exposure
- full PnL state when no risk path is active

### Volatility search kernel

Same as Sharpe but output score is just `-std`.

### Calmar search kernel

This one is heavier:

- `pos`
- `log_eq`
- `log_peak`
- `max_log_dd`

It still avoids stats/exposure, so it is cheaper than M7, but not as cheap as PnL or MaxDD.

## 6.2 Why compile-time specialization matters

State that disappears from the kernel tends to help twice:

1. fewer executed instructions
2. fewer live registers

That second effect matters a lot because Ada occupancy is still strongly influenced by register pressure.

---

## 7. Fourth major win: retune launch size and add `__launch_bounds__`

## 7.1 What the launcher is doing now

Your Rust chooser is already doing several good things:

- reads device caps,
- reads per-kernel register count,
- estimates active blocks per SM,
- scores candidates.

But the backtest chooser then overrides that with this minimum-block logic:

```rust
let mut min_block = warp;
if pairs_per_sm >= 1024 {
    min_block = 512;
} else if pairs_per_sm >= 192 {
    min_block = 256;
}
```

from `choose_backtest_launch_shape_1d` (`gpu_backtest_kernel.rs` ~487-518).

So even if the score-based chooser would pick something smaller, large enough work can still get forced upward.

## 7.2 Why I would change that first

For this kernel family, the main trade-off is not “bigger block is always better”.

It is:

- bigger block can increase scheduling efficiency and waves,
- but it also increases the chance that register pressure limits occupancy,
- and this kernel clearly has a lot of live state.

On Ada, the key occupancy facts are still:

- max **48 resident warps** per SM,
- **64K 32-bit registers per SM**,
- unified L1/shared system,
- and explicit `sm_89` compilation is recommended to fully benefit from Ada FP32 throughput.

That means block-size policy is still sensitive to register usage.

## 7.3 My recommendation

### First experiment

Benchmark the backtest family with:

- `VECTORBT_KERNEL_BLOCK_X=128`
- `VECTORBT_KERNEL_BLOCK_X=256`
- `VECTORBT_KERNEL_BLOCK_X=512`

while logging dispatch and collecting ptxas register counts.

I would expect **128 or 256** to be the real contenders.

### Then simplify the chooser

I would make the backtest chooser stop forcing 256/512 floors.

For a first pass:

```rust
#[inline]
fn choose_backtest_launch_shape_1d(
    total_items: usize,
    device_caps: DeviceLaunchCaps,
    kernel_caps: KernelLaunchCaps,
) -> LaunchShape1d {
    choose_launch_shape_1d_from_candidates(
        total_items,
        device_caps,
        kernel_caps,
        &[128u32, 256, 512],
    )
}
```

That is intentionally boring. The point is to let measured register pressure win.

### Then settle on a maximum and add `__launch_bounds__`

Once you decide what the real maximum block size should be for the backtest family, add launch bounds to the hot kernels.

A good conservative first step is the **single-argument** form:

```cpp
extern "C" __global__
__launch_bounds__(256)
void double_cross_backtest_tm_flex_f32_lockstep_time_m7(/* ... */)
{
    // ...
}
```

If profiling shows a clear occupancy target, then try the two-argument form:

```cpp
extern "C" __global__
__launch_bounds__(256, 2)
void double_cross_backtest_tm_flex_f32_lockstep_time_m7(/* ... */)
{
    // ...
}
```

I would treat the two-argument form as a measured optimization, not a default guess.

## 7.4 Why this is better than file-wide `-maxrregcount`

`-maxrregcount` is useful for experiments, but it is file-wide and blunt.

`__launch_bounds__` is:

- per-kernel,
- architecture-aware through `__CUDA_ARCH__` if needed,
- and often the better long-term control mechanism for a family of hot kernels.

## 7.5 Practical compile-time instrumentation

Always keep these build-time signals on while tuning:

```bash
nvcc -O3 -lineinfo -Xptxas=-v ...
```

That gives you:

- register count
- shared memory usage
- constant memory usage

without needing to guess.

If your device code spans multiple translation units or device libraries, CUDA 13+ also supports **device link-time optimization** (`-dlto`, optionally with `--gen-opt-lto`). For a single large `.cu` translation unit the benefit may be small, but it is worth testing if the device-side implementation is split.

---

## 8. Fifth major win: replace flat pair indexing with a warp-per-fast-row mapping

## 8.1 Why the current mapping is good, but not final-form optimal

Right now the kernel does:

- one logical thread per pair,
- optional `pair_index_map` remapping to valid pairs,
- `pf_local = dense_pair / Ps_tile`
- `ps_local = dense_pair % Ps_tile`
- optional fast-row dedup via `__match_any_sync`.

That is already smarter than a naive dense pair loop.

But for your main benchmark, the real hot path is likely:

- **lockstep time**
- **M7 specialized**
- **valid-pair-map active**
- **not** the separate `fast_warp_dedup` kernel

So the next structural win is not “tune the separate warp-dedup path”. It is “make the main lockstep M7 path use a better mapping”.

## 8.2 The better mapping

Map work as:

- **one warp owns one fast row**
- each lane owns one slow row within that fast row
- lanes advance through slow rows in steps of 32

That gives you:

- one fast load per warp per bar,
- contiguous slow loads across lanes,
- no per-iteration `dense_pair / Ps_tile`,
- no per-iteration `__match_any_sync`,
- and better row-locality.

## 8.3 Why this matches the data layout

Your MA matrices are time-major:

```cpp
a[(size_t)t * (size_t)dim + (size_t)i]
```

For a fixed `t`, adjacent `i` are contiguous in memory.

That means a warp whose lanes hold `ps_local = lane + k*32` gets naturally coalesced slow-row loads for each bar.

That is exactly what you want.

## 8.4 Skeleton of the mapping

A simplified sketch:

```cpp
template <int WARPS_PER_BLOCK>
extern "C" __global__
void double_cross_backtest_tm_lockstep_warp_rows_f32(
    const float* __restrict__ fast_ma_T,
    const float* __restrict__ slow_ma_T,
    const float* __restrict__ lr,
    const float* __restrict__ ret1,
    int T,
    int Pf_tile,
    int Ps_tile,
    int fast_stride,
    int slow_stride,
    const int* __restrict__ fast_periods,
    const int* __restrict__ slow_periods,
    /* regime / strategy / output args ... */)
{
    const int lane    = threadIdx.x & 31;
    const int warp_id = threadIdx.x >> 5;
    const int pf_local = blockIdx.x * WARPS_PER_BLOCK + warp_id;

    if (pf_local >= Pf_tile) return;

    const int period_f = fast_periods[pf_local];
    const int pair_first_valid_fast = /* first_valid + period_f - 1 */;

    for (int ps_local = lane; ps_local < Ps_tile; ps_local += 32) {
        const int period_s = slow_periods[ps_local];
        if (period_f >= period_s) continue;

        int pos = 0;
        float log_eq = 0.0f;
        float log_peak = 0.0f;
        float max_log_dd = 0.0f;
        float mean = 0.0f, m2 = 0.0f;
        int n = 0;

        const int start_t = /* max warmup / regime / eval start */;
        for (int t = start_t; t < T; ++t) {
            const int t_sig = /* trade offset applied here */;
            float f = 0.0f;
            if (lane == 0) {
                f = fast_ma_T[(size_t)t_sig * (size_t)fast_stride + (size_t)pf_local];
            }
            f = __shfl_sync(0xffffffff, f, 0);

            const float s =
                slow_ma_T[(size_t)t_sig * (size_t)slow_stride + (size_t)ps_local];

            // strategy logic ...
            // update log_eq, max_log_dd, mean/m2 from lr[t] and ret1[t]
        }

        // store compact result for this (pf_local, ps_local)
    }
}
```

## 8.5 Why I like this more than extending `__match_any_sync`

`ld_tm_fast_warp_dedup()` is clever, but it still means:

- the warp composition can straddle multiple fast rows,
- `__match_any_sync` / `__shfl_sync` happens in the loop,
- and the mapping is still pair-centric rather than row-centric.

Warp-per-fast-row fixes the topology instead of patching around it.

## 8.6 Cost of this rewrite

This is an invasive rewrite.

I would do it **after**:

- search/finalist split,
- `ret1` precompute,
- launch tuning.

But if you want the next “big” kernel-architecture change after those, this is the one.

---

## 9. Sixth major win: only then try shared-memory time tiling

## 9.1 When to do this

Do this only if Nsight Compute says the kernel is still heavily memory-stalled after the previous changes.

If the main limiter is still:

- SFU pressure from `expm1f`
- or register pressure / low occupancy

then shared tiling is not the first fix.

## 9.2 What to tile first

Do **not** start by tiling the MA matrices.

Start by tiling the arrays that are reused by **every pair** in the block:

- `lr`
- `ret1`
- `regime`

These are much better shared-memory candidates because they are shared by all lanes / warps.

## 9.3 `memcpy_async` / `cp.async` path

On compute capability 8.x and later, CUDA provides hardware-accelerated async copies from global to shared memory.

This is exactly the path to use for time tiles.

### Minimal conceptual shape

```cpp
#include <cuda/pipeline>

template <int TILE_T>
__device__ void load_time_tile_async(
    const float* __restrict__ src_lr,
    const float* __restrict__ src_ret1,
    int t0,
    int T,
    float* sh_lr,
    float* sh_ret1)
{
    auto pipe = cuda::make_pipeline();

    for (int i = threadIdx.x; i < TILE_T; i += blockDim.x) {
        if (t0 + i < T) {
            cuda::memcpy_async(&sh_lr[i],   &src_lr[t0 + i],   sizeof(float), pipe);
            cuda::memcpy_async(&sh_ret1[i], &src_ret1[t0 + i], sizeof(float), pipe);
        }
    }

    cuda::pipeline_commit(pipe);
    cuda::pipeline_wait_prior<0>(pipe);
    __syncthreads();
}
```

Then the bar loop becomes a loop over tiles:

- load tile,
- compute on tile,
- load next tile.

## 9.4 Why this can help

If the same `lr[t]` / `ret1[t]` / `regime[t]` values are read by many threads in the block, staging them once into shared can reduce repeated global/L2 traffic.

This is especially attractive if you move to a warp-per-fast-row kernel because the block’s work becomes more structured in time.

## 9.5 Ada shared-memory notes

On Ada:

- L1/shared is unified,
- the shared carveout can be chosen at runtime,
- and blocks can use up to 99 KB shared memory on sm_89 with the appropriate opt-in.

So if a tiled kernel needs more than the 48 KB static default, remember that you may need:

- `cudaFuncSetAttribute(..., cudaFuncAttributeMaxDynamicSharedMemorySize, ...)`
- and possibly a preferred shared-memory carveout.

---

## 10. Seventh win: compact output when `pair_index_map` is active

## 10.1 What happens today

When `valid_pair_map` is active, your backtest kernel iterates only valid pairs (`launch_pairs`), **but** the metric output is still addressed in **dense pair space**:

```cpp
const int dense_pair =
    (pair_index_map != nullptr) ? pair_index_map[pair_local] : pair_local;
...
const size_t base = pair_global * metrics_stride;
```

and the Rust side zero-fills the whole dense tile when pair-map mode is active.

That means:

- the compute loop is compact,
- but the metric buffer is still dense,
- selectors / heatmap still resolve dense pair indices,
- and there is a whole-tile zero fill before compute.

## 10.2 Why I would change it

For your benchmark, valid pair count is roughly half of dense pair count when fast and slow period grids are the same size.

So compact output can:

- cut tile-buffer size,
- eliminate the dense zero-fill,
- reduce metric-store traffic,
- reduce selector/heatmap metric reads.

## 10.3 Practical implementation

Keep `pair_index_map` only for decoding:

- `compact_pair -> dense_pair -> (pf_local, ps_local)`

But store the output metrics by **compact pair index**.

### Compact-output base

```cpp
const int compact_pair = pair_local;
const int dense_pair =
    (pair_index_map != nullptr) ? pair_index_map[compact_pair] : compact_pair;

const size_t base_out =
    compact_output ? (size_t)compact_pair * (size_t)metrics_stride
                   : (size_t)dense_pair   * (size_t)metrics_stride;
```

Then in selectors / heatmap:

- use `dense_pair` only to recover `pf_local` / `ps_local`,
- but load metrics from `compact_pair * stride`.

That is a localized change with measurable upside.

## 10.4 Why this is not the first thing I would do

It saves bandwidth and memory footprint, but it does **not** remove as much inner-loop work as:

- search/finalist split,
- `ret1`,
- objective-specialized kernels.

So I would do it after those.

---

## 11. Register-pressure and scalar-width cleanup

These are not the first changes, but they are real.

## 11.1 Replace 64-bit counters that do not need to be 64-bit

In the hot loop you currently use:

- `long long sum_abs_pos`
- `long long sum_pos`
- `long long n`

for series lengths around 200k.

That is overkill.

For this benchmark scale:

- `n` fits in `int`
- `sum_abs_pos` fits in `int`
- `sum_pos` fits in `int`

### Suggested replacement

```cpp
int sum_abs_pos = 0;
int sum_pos     = 0;
int n           = 0;
```

and:

```cpp
sum_abs_pos += (pos_bar != 0);
sum_pos     += pos_bar;
```

instead of:

```cpp
sum_abs_pos += llabs((long long)pos_bar);
sum_pos     += (long long)pos_bar;
```

This saves:

- 64-bit arithmetic,
- `llabs`,
- and register width.

## 11.2 Use smaller base/index types where safe

Within a tile, `pf_local`, `ps_local`, `pair`, and `base` usually fit in 32-bit arithmetic.

If you know the tile cannot exceed 32-bit indexed ranges, use `int` for local indexing and only widen at the host/API boundary when needed.

This is not a top-tier change, but it is free performance when the kernel is register-limited.

## 11.3 Vectorize M7 padded stores

You already support padded M7 stride 8 on the host side.

In the FULL_M7 + padded case, write two `float4`s instead of scalar stores.

### Example

```cpp
if constexpr (FULL_M7) {
    if (metrics_stride == 8) {
        float4 a = make_float4(pnl, (float)trades, max_dd, mean);
        float4 b = make_float4(std, exposure,
                               (flags & STRAT_SIGNED_EXPOSURE) ? net_expo : 0.0f,
                               0.0f);
        reinterpret_cast<float4*>(metrics_out + base)[0] = a;
        reinterpret_cast<float4*>(metrics_out + base)[1] = b;
    } else {
        metrics_out[base + 0] = pnl;
        metrics_out[base + 1] = (float)trades;
        metrics_out[base + 2] = max_dd;
        metrics_out[base + 3] = mean;
        metrics_out[base + 4] = std;
        metrics_out[base + 5] = exposure;
        metrics_out[base + 6] = (flags & STRAT_SIGNED_EXPOSURE) ? net_expo : 0.0f;
    }
}
```

Because CUDA allocations are strongly aligned and stride 8 means 32-byte spacing, alignment should typically be favorable.

## 11.4 Remove always-false branches in valid-pair-map mode

When `pair_index_map` is built from valid fast<slow pairs, checks like:

```cpp
if (f >= s) continue;
```

inside selectors / heatmap become redundant.

Likewise the backtest kernel’s fast-vs-slow validity branch is guaranteed false when that map is used.

This is not a giant speedup, but it is a clean specialization if you keep pair-map mode on the hot path.

---

## 12. Specialization opportunities beyond the current regime/risk/eps family

You already specialize:

- regime mode,
- risk mode,
- epsilon-zero mode,
- lockstep,
- M7.

The next flags worth considering are:

- `STRAT_TRADE_ON_NEXT_BAR`
- `STRAT_LONG_ONLY`
- `STRAT_NO_FLIP`
- `commission == 0`

These are all fixed for a launch, but still generate runtime branches inside the loop.

## 12.1 Which of these I would try first

If your benchmark configuration is stable and repetitive, I would try:

1. `trade_on_next_bar` specialization
2. `commission == 0` specialization
3. `long_only` specialization
4. `no_flip` specialization

The goal is not to explode the combinatorial space. The goal is to specialize the **one or two dominant production modes**.

## 12.2 Why this is not above the search/ret1/warp work

Because those changes attack much larger cost centers.

Treat these as “after the big wins”.

---

## 13. What I would not prioritize right now

I would not spend the next round on these first:

### 13.1 CUDA Graphs

You already have graph capture support on the Rust side.

For a benchmark dominated by an 11.6B pair-bar kernel loop, graph launch overhead is not the main problem.

### 13.2 Host upload optimization

You already have:

- resident market series,
- pinned H2D support,
- async allocation,
- persistent scratch,
- reuse of uploaded data.

That is not where the big headroom is likely hiding.

### 13.3 ALMA-side optimization

Given your reported ALMA throughput, this is not the limiting subsystem.

### 13.4 Top-k tuning before shrinking the search payload

Selectors are already device-side and CUB-backed.

I would first make sure they are selecting from a much cheaper search payload rather than trying to outsmart the existing top-k kernels.

---

## 14. How I would profile this with Nsight Compute

## 14.1 First use your built-in instrumentation

Turn on:

```bash
VECTORBT_KERNEL_PROFILE_PIPELINE=1
VECTORBT_KERNEL_LOG_DISPATCH=1
```

That gives you:

- host prep
- tile prepare
- tile score
- backtest
- heatmap
- finalize

If `tile_backtest` dominates, you know the kernel work is the right target before going to Nsight.

## 14.2 Then use Nsight Compute on just the backtest family

Recommended sections:

- **SpeedOfLight**
- **WarpStateStats**
- **SourceCounters**
- **LaunchStats / Occupancy**

These are the fastest path to answering:

- compute-bound or memory-bound?
- issue-limited or occupancy-limited?
- divergent or uniform?
- SFU pressure vs memory stalls vs branch stalls?

### Example CLI pattern

```bash
ncu \
  --target-processes all \
  --kernel-name-base demangled \
  --section SpeedOfLight \
  --section WarpStateStats \
  --section SourceCounters \
  --section LaunchStats \
  --section Occupancy \
  ./your_benchmark_binary
```

## 14.3 What I would look for

### After the current baseline

- register count from ptxas
- achieved occupancy
- waves per SM / tail effects
- branch efficiency
- sampled warp stalls
- whether issue slots are saturated

### After `ret1`

If the current kernel shows strong special-function pressure or generally low issue efficiency in the stats path, `ret1` should improve that.

### After search/finalist split

The instruction mix and register pressure should fall sharply for PnL / MaxDD / Vol objectives.

### After warp-per-fast-row rewrite

Look for:

- better branch efficiency
- better memory throughput efficiency
- fewer stalls tied to divergent execution
- fewer replays / less row-wrap weirdness
- better L1/L2 behavior for slow-row loads

## 14.4 Important Nsight interpretation rule

When reading stall reasons, only obsess over them if the scheduler is **not** already issuing near its limit.

That is the right mental model: stall categories are valuable only when issue throughput says there is real headroom to recover.

---

## 15. Suggested benchmark matrix

Use the same dataset and exact same strategy config for all of these.

## 15.1 Launch tuning

```bash
VECTORBT_KERNEL_BLOCK_X=128
VECTORBT_KERNEL_BLOCK_X=256
VECTORBT_KERNEL_BLOCK_X=512
```

Collect:

- runtime
- ptxas registers
- achieved occupancy
- waves/SM

## 15.2 Existing-path toggles

```bash
VECTORBT_KERNEL_LOCKSTEP_TIME=0/1
VECTORBT_KERNEL_M7_SPECIALIZE=0/1
VECTORBT_KERNEL_TM_PAD=0/1
VECTORBT_KERNEL_FAST_WARP_DEDUP=0/1
VECTORBT_KERNEL_VALID_PAIR_MAP=0/1
VECTORBT_KERNEL_GLOBAL_TOPK=0/1
```

This tells you whether the benchmark is already on the expected hot path.

For your benchmark size, I would expect:

- lockstep time: **on**
- M7 specialization: **on**
- valid-pair-map: **on**
- fast-warp-dedup: probably **not the decisive hot path**

## 15.3 New-path experiments

Once implemented:

- search/finalist split on/off
- `ret1` path on/off
- compact-output pair-map on/off
- warp-per-fast-row on/off

This is the matrix that will actually tell you what the next 10-30% comes from.

---

## 16. Concrete patch suggestions

This section gives you implementation-oriented suggestions rather than just strategy.

---

### 16.1 Add `ret1` scratch to Rust

```rust
#[derive(Default)]
struct KernelScratch {
    d_lr: Option<DeviceBuffer<f32>>,
    d_ret1: Option<DeviceBuffer<f32>>,
    d_fast_p: Option<DeviceBuffer<i32>>,
    d_fast_ma: Option<DeviceBuffer<f32>>,
    d_fast_ma_tm: Option<DeviceBuffer<f32>>,
    d_slow_p: Option<DeviceBuffer<i32>>,
    d_slow_ma: Option<DeviceBuffer<f32>>,
    d_slow_ma_tm: Option<DeviceBuffer<f32>>,
    d_tile: Option<DeviceBuffer<f32>>,
    // ...
}
```

Then allocate/fill it next to `d_lr`.

---

### 16.2 Replace `compute_log_returns_f32`

```cpp
extern "C" __global__
void compute_log_and_simple_returns_f32(
    const float* __restrict__ prices,
    int T,
    float* __restrict__ lr_out,
    float* __restrict__ ret1_out)
{
    for (int t = blockIdx.x * blockDim.x + threadIdx.x;
         t < T;
         t += blockDim.x * gridDim.x)
    {
        if (t == 0) {
            lr_out[0] = 0.0f;
            ret1_out[0] = 0.0f;
            continue;
        }

        const float p  = prices[t];
        const float pm = prices[t - 1];
        if (p > 0.0f && pm > 0.0f && isfinite(p) && isfinite(pm)) {
            const float ratio = p / pm;
            lr_out[t]   = logf(ratio);
            ret1_out[t] = ratio - 1.0f;
        } else {
            lr_out[t]   = 0.0f;
            ret1_out[t] = 0.0f;
        }
    }
}
```

---

### 16.3 Change the stats path in the kernel body

```cpp
int sum_abs_pos = 0;
int sum_pos     = 0;
int n           = 0;
float mean = 0.0f;
float m2   = 0.0f;

// ...

if (need_stats) {
    const float step_r =
        (pos_bar == 0) ? 0.0f : (float)pos_bar * ret1[t];

    ++n;
    const float delta = step_r - mean;
    mean += delta / (float)n;
    m2   += delta * (step_r - mean);

    if (need_expo) {
        sum_abs_pos += (pos_bar != 0);
        sum_pos     += pos_bar;
    }
}
```

---

### 16.4 Remove the minimum block-size floor

```rust
#[inline]
fn choose_backtest_launch_shape_1d(
    total_items: usize,
    device_caps: DeviceLaunchCaps,
    kernel_caps: KernelLaunchCaps,
) -> LaunchShape1d {
    choose_launch_shape_1d_from_candidates(
        total_items,
        device_caps,
        kernel_caps,
        &[128u32, 256, 512],
    )
}
```

Use `VECTORBT_KERNEL_BLOCK_X` to force values during profiling, then settle on the real best policy.

---

### 16.5 Add launch bounds after settling on a max block size

```cpp
extern "C" __global__
__launch_bounds__(256)
void double_cross_backtest_tm_flex_f32_lockstep_time_m7(/* ... */)
{
    double_cross_backtest_tm_flex_f32_body<false, true, true>(/* ... */);
}
```

Later, if profiling supports it:

```cpp
extern "C" __global__
__launch_bounds__(256, 2)
void double_cross_backtest_tm_flex_f32_lockstep_time_m7(/* ... */)
{
    double_cross_backtest_tm_flex_f32_body<false, true, true>(/* ... */);
}
```

---

### 16.6 Create a cheap search-pass kernel family

For the highest-value first family, I would implement these wrappers first:

- `search_logpnl`
- `search_sharpe`
- `search_neglogdd`
- `search_calmar`
- `search_negvol`

These should all write a small row type, not M7.

### Compact row example

```cpp
struct SearchRow {
    float score;
    float aux0;   // trades or max_log_dd if needed
    float aux1;   // spare / second constraint field if needed
};
```

or separate SOA buffers if you prefer.

---

### 16.7 Finalist recompute path

Once top-k is known, launch full M7 only for finalists.

A simple host-side pattern:

```rust
let finalists: Vec<(u16, u16)> = download_topk_pairs(/* ... */)?;
let full_metrics = eval_finalists_full_m7(
    rt,
    candles,
    &finalists,
    fast_ma_type,
    slow_ma_type,
    ma_source,
    fast_ma_params,
    slow_ma_params,
    regime_filter,
    strategy,
    cfg,
)?;
```

If you want to keep everything on device longer, you can also upload finalist dense-pair indices and reuse the existing dynamic/eval path with a small pair map.

---

### 16.8 Compact-output pair-map path

For valid-pair-map mode:

```cpp
const int compact_pair = pair_local;
const int dense_pair =
    (pair_index_map != nullptr) ? pair_index_map[compact_pair] : compact_pair;

const size_t base_metrics =
    compact_output ? (size_t)compact_pair * (size_t)metrics_stride
                   : (size_t)dense_pair   * (size_t)metrics_stride;
```

Then selectors and heatmap can decode:

- `dense_pair -> pf_local / ps_local`

but read:

- `metrics[compact_pair * metrics_stride]`

That removes the dense zero-fill requirement.

---

### 16.9 Warp-per-fast-row kernel family

If you decide to do the structural rewrite, I would create a new dedicated family rather than trying to contort the existing pair-local family:

- `double_cross_backtest_tm_lockstep_rowwarp_m7_*`

Keep the current kernels as a fallback until the new mapping fully passes validation.

---

## 17. Accuracy-risky experiments (optional, gated behind validation)

These are not my first recommendations, but if you are chasing the absolute edge, they are worth isolating behind a benchmark/validation switch.

### 17.1 Approximate math intrinsics

Possibilities:

- `__logf`
- `__expf`
- `rsqrtf` in Sharpe score path
- `--use_fast_math`

These can be real wins, but they are not free semantically.

For a backtest optimizer, I would only keep them if:

- final top-1 identity is stable,
- top-k overlap is stable,
- and the metric drift stays within your acceptance bounds.

### 17.2 More flag specialization

If one or two strategy modes dominate production, specialize:

- `trade_on_next_bar`
- `long_only`
- `no_flip`
- `commission == 0`

This is especially reasonable if you already know the production benchmark mix is narrow.

---

## 18. My recommended implementation order

If the goal is **maximum performance with good engineering payoff**, this is the sequence I would actually follow.

### Stage 1 — safest, highest ROI

1. Add `ret1`
2. remove `expm1f` from stats path
3. benchmark block sizes 128 / 256 / 512
4. simplify chooser
5. add `__launch_bounds__` once the max block size is settled

### Stage 2 — biggest structural payoff

6. split search pass from finalist M7 pass
7. make search kernels objective-specific
8. use log-space score for PnL and MaxDD search
9. convert maxDD constraints to log-space on host for search

### Stage 3 — next kernel rewrite

10. warp-per-fast-row lockstep M7 family
11. compact metric output in pair-map mode

### Stage 4 — only if Nsight says memory is still the limiter

12. shared-memory time tiling for `lr`, `ret1`, `regime`
13. `memcpy_async` / `cp.async`
14. shared carveout / dynamic shared memory tuning

### Stage 5 — cleanup and final polish

15. int32 accumulators
16. remove `llabs`
17. vectorized padded M7 stores
18. optional strategy-flag specialization

---

## 19. Expected ranking of payoff

My expected ranking, before measurement, is:

1. **search/finalist split + objective-specialized search kernels**
2. **`ret1` precompute + remove `expm1f` from stats path**
3. **launch-size retuning + `__launch_bounds__`**
4. **warp-per-fast-row rewrite in lockstep M7 path**
5. **compact output for valid-pair-map mode**
6. **int32 / `llabs` / vector-store cleanup**
7. **shared-memory time tiling**
8. **approximate math / fast-math experiments**

I would be surprised if host-side orchestration or ALMA-side work beats the first four items for this benchmark.

---

## 20. Final advice

If I had to compress the whole guide into one sentence:

> **Make the search pass much cheaper first, then make the hot loop do less transcendental and less register-heavy work, and only then do a bigger mapping rewrite.**

That is where the maximum remaining headroom is most likely hiding.

---

## 20A. Current-code checklist (verified against repo on 2026-03-19)

This checklist keeps only the parts of the guide that still match the current codebase.

Important current-code corrections before using this as a work tracker:

- [x] Treat the optimizer search payload as a **10-slot GPU row**, not the older 7-slot row.
- [x] Preserve the currently emitted extra GPU metrics:
  - `downside_deviation`
  - `ulcer_index`
  - `martin_ratio`
- [x] Keep in mind that the GPU kernel path currently supports optimization objectives:
  - `Pnl`
  - `Sharpe`
  - `MaxDrawdown`
  - `Calmar`
  - `Volatility`
- [x] Keep in mind that the GPU kernel path currently does **not** support optimization objectives:
  - `Sortino`
  - `Omega`
- [x] Preserve final `Metrics` compatibility for validation and UI consumers that already depend on `downside_deviation`, `ulcer_index`, and `martin_ratio`.

### Baseline and profiling

- [x] Re-run the target benchmark and record the current baseline for `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`.
- [x] Capture pipeline timing with `VECTORBT_KERNEL_PROFILE_PIPELINE=1`.
- [x] Capture dispatch decisions with `VECTORBT_KERNEL_LOG_DISPATCH=1`.
- [x] Record whether the benchmark is actually dispatching to `lockstep_m7`.
- [x] Collect ptxas register counts with `-Xptxas=-v`.
- [x] Run Nsight Compute on the backtest kernels before changing kernel structure.

Measured on 2026-03-19:

- The current repo contains two different ALMA 200k / 58,300-pair Criterion benchmark families:
  - older static optimizer benchmark: `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
  - newer full-metrics benchmark: `double_ma/gpu_kernel_pnl_full_metrics/alma_alma/200k/large/58300pairs`
- The 2026-03-19 reruns below are for the newer `gpu_kernel_pnl_full_metrics` workload only. They should not be treated as a direct replacement for the older `gpu_kernel` target named at the top of this guide.
- A fresh rerun of the exact guide target `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs` with `VECTORBT_KERNEL_BLOCK_X=128` reported `173.74 ms .. 174.32 ms` (Criterion mean estimate about `173.71 ms`).
- That means the current code on this machine/config is materially slower than the older `~77-96 ms` notes captured below; the expected `~85 ms` behavior is not what the current repo is delivering.
- Criterion reruns on `double_ma/gpu_kernel_pnl_full_metrics/alma_alma/200k/large/58300pairs` with `VECTORBT_BENCH_KERNEL_MA=alma` reported:
  - `VECTORBT_KERNEL_BLOCK_X=128`: `174.42 ms .. 174.60 ms`
  - `VECTORBT_KERNEL_BLOCK_X=256`: `453.18 ms .. 453.46 ms`
  - `VECTORBT_KERNEL_BLOCK_X=512`: `174.63 ms .. 174.84 ms`
- Repo-local historical notes for the older `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs` target remain materially lower:
  - guide header reported benchmark: `76.919 ms`
  - dynamic execution plan notes: about `83.42 ms`, then about `82.55 ms`, and a fresh same-session rerun at about `96.388 ms`
- Pipeline+dispatch rerun with `VECTORBT_KERNEL_PROFILE_PIPELINE=1`, `VECTORBT_KERNEL_LOG_DISPATCH=1`, and `VECTORBT_KERNEL_BLOCK_X=128` reported `dispatch=lockstep_time`, `use_m7=false`, `use_lockstep_time=true`, `use_lockstep_m7=false`, `use_fast_warp=false`.
- Pipeline timing on the same workload stayed mostly in the `173-175 ms` range, with warmup and occasional outliers up to about `195 ms`.
- Direct `nvcc -Xptxas=-v` against the current CUDA file reported `double_cross_backtest_tm_flex_f32_lockstep_time` at `54` registers, `double_cross_backtest_tm_flex_f32` at `48`, `double_cross_backtest_tm_flex_f32_m7` at `47`, and `double_cross_backtest_tm_flex_f32_lockstep_time_m7` at `53`; only `double_cross_backtest_tm_flex_f32_fast_warp_dedup` showed spills.
- A fresh narrow Nsight capture at `target/nsight/ncu_lockstep_time_128_current.ncu-rep` successfully profiled `double_cross_backtest_tm_flex_f32_lockstep_time`.

### Search-pass reduction

- [x] Replace the current full 10-slot search row with a cheaper search-pass row.
- [x] Keep full metric recomputation for finalists only.
- [x] Design search-pass outputs around the current supported GPU objectives only.
- [x] Use log-space ranking for `Pnl` search where possible.
- [x] Use log-space ranking for `MaxDrawdown` search where possible.
- [x] Convert host-side max drawdown constraints to log-space for the search pass when applicable.
- [x] Ensure the finalist pass still reconstructs the current full `Metrics` surface expected by the app.

### Hot-loop return path

- [x] Add a precomputed simple-return buffer alongside `lr`.
- [x] Replace inner-loop `expm1f(lr_t)` use in the stats path with reads from the simple-return buffer.
- [x] Keep `lr` for PnL, drawdown, and risk-path logic unless measurement proves a better alternative.
- [x] Validate top-1 identity, top-k overlap, and metric tolerances after the return-path change.

### Launch tuning

- [x] Benchmark backtest block sizes at `128`, `256`, and `512`.
- [x] Remove or relax the current 256/512 minimum backtest block-size floor if measurements support it.
- [x] Add `__launch_bounds__` to the hot backtest kernels only after the block-size decision is measured.
- [x] Re-check occupancy and register pressure after launch-shape changes.

Measured block-size sweep on 2026-03-19 for the same ALMA large benchmark:

- `128`: `174.42 ms .. 174.60 ms`
- `256`: `453.18 ms .. 453.46 ms`
- `512`: `174.63 ms .. 174.84 ms`

Current launch-shape evidence:

- `choose_backtest_launch_shape_1d(...)` now allows `128`.
- The hot exported backtest wrappers carry `__launch_bounds__(128)`.
- A direct Nsight capture for `double_cross_backtest_tm_flex_f32_lockstep_time` at block size `(128, 1, 1)` shows `54` registers per thread, register-limited occupancy (`Block Limit Registers = 9`), theoretical occupancy `75%`, and achieved occupancy `29.66%`.

### Pair mapping and output layout

- [x] Keep using `valid_pair_map` for compact launched work.
- [x] Remove dense output zero-fill in valid-pair-map mode by testing a compact metric-output path.
- [x] Ensure selectors and heatmap logic can decode dense pair identity while reading compact metric rows.

### Kernel-structure work

- [ ] Evaluate a warp-per-fast-row lockstep kernel family as a separate implementation, not an in-place contortion of the existing pair-local family.
- [ ] Keep the current lockstep/M7 family as fallback until parity is proven.
- [ ] Only attempt shared-memory tiling after profiling still shows a memory-stall problem after the earlier changes.
- [ ] If shared tiling is attempted, tile `lr`, simple returns, and `regime` before trying to tile MA planes.

### Cleanup and lower-priority work

- [ ] Replace 64-bit loop accumulators with 32-bit types where range guarantees are clear.
- [ ] Remove `llabs` from the hot path if the accumulator-width change makes it unnecessary.
- [ ] Re-evaluate vectorized metric stores only against the current row layout, not the outdated 7-slot assumption.
- [ ] Treat approximate-math or fast-math experiments as gated follow-up work behind metric-parity validation.

### Additional review findings (2026-03-19)

These items came from a code review of the current optimizer path after the search-row / compact-output / finalist-recompute changes landed.

- [ ] Measure and, if warranted, remove the per-tile global-top-k D2D append copies on the device path in `gpu_backtest_kernel.rs`.
- [ ] Measure and, if warranted, remove the second global-top-k reduction over appended rows in `gpu_backtest_kernel.rs`.
- [ ] Replace the normal per-tile three-copy D2H winner transfer (`top_fast`, `top_slow`, `top_metrics`) with a single packed row transfer or a device-side decoded winner layout.
- [ ] Extend compact-output handling so `include_all`, host-side constraint filtering, and CSV/export do not have to pull the full dense tile back to the host.
- [ ] Re-evaluate whether heatmap work should stay in the default optimization benchmark contract; if yes, reduce the extra fill / update / D2H heatmap passes.
- [ ] Reduce repeated `pair_index_map` decode work across downstream compact-row kernels (heatmap, top-k, candidate reduction, gather) by caching or carrying already-decoded pair identity.
- [ ] Rework finalist full-metric recompute to minimize the returned-candidate rerun cost without dropping the full-metric contract for `best` and `top_k`.
- [ ] For precomputed host MA matrices, emit TM layout directly and remove the extra row-major H2D copy plus transpose launch.
- [ ] In `pack_precomputed_rows_with_padding(...)`, stop fully NaN-filling the whole staging buffer when only the padded tail needs explicit fill.
- [ ] In `pack_precomputed_rows_with_padding(...)`, stop cloning the last row with `to_vec()` on every tile; reuse a scratch row or copy directly from the already-packed slice.
- [ ] Replace remaining safe zero-initialized scratch destinations with uninitialized allocation where every element is definitely overwritten before read.
- [ ] Benchmark the exact guide target in both `search_only` and `end_to_end_product` modes so search-engine wins and full-contract wins do not get conflated.

### Buffer sizing notes (2026-03-19)

The current `NaN`-backed buffers are not all the same size class.

- Regime buffers in `double_ma.rs` are typically `vec![f64::NAN; n]`, where `n = candle_count`.
- Single-row placeholders such as `nan_row` are typically `vec![f32::NAN; n]`.
- MA matrix buffers are typically `vec![f32::NAN; rows * n]` or `vec![f64::NAN; rows * n]`, where `rows = number of swept MA periods`.

For the guide target workload (`alma_alma`, `200,000` candles, fast `5:200:1`, slow `10:400:1`):

- `n = 200,000`
- Fast rows = `196`
- Slow rows = `391`
- One `f64` regime buffer = `200,000 * 8` bytes = `1,600,000` bytes (`~1.53 MiB`)
- One `f32` single-row placeholder = `200,000 * 4` bytes = `800,000` bytes (`~0.76 MiB`)
- Fast `f32` matrix = `196 * 200,000 * 4` bytes = `156,800,000` bytes (`~149.54 MiB`)
- Slow `f32` matrix = `391 * 200,000 * 4` bytes = `312,800,000` bytes (`~298.31 MiB`)
- Fast + slow `f32` matrices together = `469,600,000` bytes (`~447.85 MiB`)
- Fast `f64` matrix = `313,600,000` bytes (`~299.07 MiB`)
- Slow `f64` matrix = `625,600,000` bytes (`~596.62 MiB`)
- Fast + slow `f64` matrices together = `939,200,000` bytes (`~895.69 MiB`)

So the regime buffers are candle-sized, but the matrix buffers are much larger than the candle input and scale as `rows * candle_count`. Those matrix-sized `NaN` buffers are the ones worth treating as a separate performance concern.

### Acceptance criteria

- [x] CPU-only builds still work with no CUDA toolchain installed.
- [x] GPU and CPU parity tests cover `downside_deviation`, `ulcer_index`, and `martin_ratio`.
- [x] Exported CSV and any downstream artifact readers remain compatible with the current metric columns.
- [x] Validation/reporting behavior remains stable for robust-score and risk-warning logic that depends on the newer metrics.
- [x] Performance claims are backed by measured benchmark deltas, not only code inspection.

---

## 21. References

These are the NVIDIA references most relevant to the recommendations above.

- **NVIDIA Ada GPU Architecture Tuning Guide**  
  https://docs.nvidia.com/cuda/ada-tuning-guide/index.html

- **CUDA C++ Programming Guide**  
  https://docs.nvidia.com/cuda/cuda-c-programming-guide/

- **CUDA Compiler Driver (NVCC)**  
  https://docs.nvidia.com/cuda/cuda-compiler-driver-nvcc/

- **CUDA C++ Best Practices Guide**  
  https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/

- **Nsight Compute Profiling Guide**  
  https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html

### Specific topics worth rereading

- Ada occupancy, L2, unified L1/shared, explicit `sm_89` compilation
- `__launch_bounds__`
- occupancy calculator APIs
- `memcpy_async` / async copies to shared
- L2 access-policy windows / persisting accesses
- Nsight Compute `SpeedOfLight`, `WarpStateStats`, and `SourceCounters`
- NVCC flags: `-Xptxas=-v`, `-lineinfo`, `-dlto`, `--gen-opt-lto`
