# `double_crossover` static performance findings and concrete patch plan

## Scope

This document consolidates the repo-grounded static findings from:

- `double_crossover.cu`
- `double_crossover_sm89_cuobjdump_2026-04-01.sass.txt`
- `double_crossover_sm89_cuobjdump_symbols_2026-04-01.txt`
- `double_crossover_sm89_nvdisasm_symbols_2026-04-01.txt`
- `gpu_backtest_kernel.rs` (runtime/orchestration context)
- `double_crossover_second_pass_static_counts.txt`

This is intentionally **not** generic CUDA advice. It is a code-and-cubin-specific patch plan for the `double_ma` route. It is also static-only: anything that depends on runtime mix, cache hit rate, or actual memory throughput still needs Nsight Compute later.

---

## Direct conclusion

The remaining headroom still looks mostly **source/layout and reduction/dataflow related**, not like a broad `sm_89` compiler failure.

The strongest evidence is in the exact `m17` / `m22` families. The source already contains a packed `N == 22` store path in `store_metrics_row_fixed_f32` (source lines 3485-3566), and the emitted SASS shows that packed stores do appear in the `windowed_m22` family. But the non-windowed `m17` / `m22` families and the lockstep `m17` / `m22` families consistently call `double_cross_backtest_tm_flex_f32_body` **without** `FIXED_M` and often discard `metrics_out_stride`, so the hottest exact wrappers are structurally preventing the best write path from being used.

The second major issue is the reduction path. The raw row selectors recompute `objective_score_f32` inline, the `v2` selectors also call `passes_constraints_f32` and then compute the objective again, and the runtime still drives `reduce_metric_rows_topk_on_device(...)` through the legacy row selector family. Meanwhile, the source already has a smaller score-array path (`score_metric_rows_f32`, `select_topk_scored_rows_f32`, `select_topk_scored_rows_from_candidates_f32`).

The pair path has the same pattern: `select_topk_scored_pairs_tm_f32*` are **not** actually pre-scored; they still compute the objective inside the selector. The real score-array path is `score_metric_rows_f32` + `select_topk_scored_candidates_tm_f32` + `gather_topk_scored_scalar_pairs_tm_f32`.

The only codegen issue specific enough to keep alive is narrow: **if you fix the exact-wrapper specialization and give `m22` a consistently aligned padded stride and the compiler still refuses to emit packed stores, then investigate codegen**. Right now the better explanation is still the source contract, not the compiler.

---

## What the source is trying to do

### Core backtest body

`double_cross_backtest_tm_flex_f32_body` (source lines 3569-4359) is the common execution engine. It walks bars, computes returns / drawdown / trade statistics / exposure metrics, and writes either:

- a full metrics row,
- or a score-only row for special score-only variants.

It also supports:

- dense pair mapping,
- compact row mapping,
- regime filtering,
- multiple risk/eps/commission specialization modes,
- lockstep time,
- windowed evaluation,
- optional warp-dedup loads for fast-series accesses.

### Exact fixed-row output helpers

`store_metrics_row_fixed_f32` (source lines 3485-3566) already contains an explicit packed path for `N == 22`:

- aligned path: `float4` stores for the first 20 floats + `float2` tail,
- fallback path: `float2` stores,
- otherwise scalar stores for other widths.

`zero_metrics_row_fixed_f32` (source line 3474 onward) writes out a full zero row for invalid combinations.

### Row reduction path

The row reducer family is:

- `score_metric_rows_f32` (8834-8848)
- `select_topk_scored_rows_f32` (8851-8886)
- `select_topk_scored_rows_from_candidates_f32` (8889-8928)
- `gather_topk_scored_rows_f32` (8931-8955)
- `select_topk_metric_rows_f32` (8958-8997)
- `select_topk_metric_rows_from_candidates_f32` (9000-9043)
- `select_topk_metric_rows_f32_v2` (9046-9111)
- `select_topk_metric_rows_from_candidates_f32_v2` (9114-9183)
- `gather_topk_metric_rows_f32` (9272-9298)

The important split is:

- **scored-row path**: objective is computed once into a score array;
- **raw-row path**: objective is recomputed inside the selector.

### Pair reduction path

The pair reducer family is:

- `select_topk_from_pairs_tm_f32` (8074-8143)
- `select_topk_from_candidates_tm_f32` (8146-8222)
- `select_topk_from_pairs_tm_f32_v2` (8225-8322)
- `select_topk_from_candidates_tm_f32_v2` (8325-8429)
- `gather_topk_pairs_tm_f32` (8432-8495)
- `select_topk_scored_pairs_tm_f32` (8498-8569)
- `select_topk_scored_pairs_tm_f32_v2` (8572-8671)
- `select_topk_scored_candidates_tm_f32` (8674-8712)
- `gather_topk_scored_pairs_tm_f32` (8715-8772)
- `gather_topk_scored_scalar_pairs_tm_f32` (8775-8831)

The misleading part is the names: `select_topk_scored_pairs_tm_f32*` still compute the objective from `metrics`, so they are not the true score-array path.

---

## Most important evidence

### 1) Exact wrapper specialization / stride matrix

| Wrapper | body<> template | FIXED_M in template? | metrics_out_stride | store mix in SASS |
|---|---|---:|---|---|
| `double_cross_backtest_tm_flex_f32_m17` | `false,false,false` | no | ignored | scalar only |
| `double_cross_backtest_tm_flex_f32_m22` | `false,false,false` | no | ignored | scalar only |
| `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk` | `false,false,false,0,0` | no | ignored | scalar only |
| `double_cross_backtest_tm_flex_f32_m22_regime_off_norisk` | `false,false,false,0,0` | no | ignored | scalar only |
| `double_cross_backtest_tm_flex_f32_lockstep_time_m17` | `false,true,false` | no | ignored | scalar only |
| `double_cross_backtest_tm_flex_f32_lockstep_time_m22` | `false,true,false` | no | ignored | scalar only |
| `double_cross_backtest_tm_flex_f32_windowed_m17` | `false,false,false` | no | honored | scalar only |
| `double_cross_backtest_tm_flex_f32_windowed_m22` | `false,false,false,-1,-1,-1,-1,22,true` | yes | honored | mixed 128/64/scalar |


Interpretation:

- The non-windowed exact families do **not** instantiate `FIXED_M` even though they hardcode `M = 17` or `M = 22`.
- The non-windowed and lockstep exact families also discard the caller-provided `metrics_out_stride` and force a dense row stride of `17` or `22`.
- `windowed_m22` is the one clear counterexample: it already instantiates `FIXED_M = 22` and honors `metrics_out_stride`.
- `windowed_m17` is inconsistent with `windowed_m22`: it still omits `FIXED_M`.

### 2) Store-width evidence from the emitted SASS

| Function | STG.E.128 | STG.E.64 | STG.E (scalar) |
|---|---:|---:|---:|
| `double_cross_backtest_tm_flex_f32_m17` | 0 | 0 | 51 |
| `double_cross_backtest_tm_flex_f32_m22` | 0 | 0 | 66 |
| `double_cross_backtest_tm_flex_f32_lockstep_time_m17` | 0 | 0 | 51 |
| `double_cross_backtest_tm_flex_f32_lockstep_time_m22` | 0 | 0 | 66 |
| `double_cross_backtest_tm_flex_f32_windowed_m17` | 0 | 0 | 51 |
| `double_cross_backtest_tm_flex_f32_windowed_m22` | 5 | 12 | 44 |
| `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm` | 5 | 12 | 44 |


Interpretation:

- The packed-store path is real. It is not hypothetical; it already shows up in the emitted cubin for `windowed_m22`.
- The hot non-windowed and lockstep `m22` families are still fully scalar in the attached SASS.
- `m17` currently has no equivalent packed path at all.

### 3) Reduction/output static size comparison

| Kernel | inst | LDG | STG | IMAD | FFMA |
|---|---:|---:|---:|---:|---:|
| `score_metric_rows_f32` | 880 | 10 | 1 | 51 | 87 |
| `select_topk_metric_rows_f32` | 1792 | 20 | 2 | 114 | 162 |
| `select_topk_metric_rows_f32_v2` | 2976 | 50 | 2 | 180 | 272 |
| `select_topk_scored_rows_f32` | 656 | 2 | 2 | 73 | 22 |
| `gather_topk_metric_rows_f32` | 640 | 37 | 70 | 52 | 0 |
| `gather_topk_scored_rows_f32` | 368 | 4 | 5 | 33 | 22 |
| `select_topk_from_pairs_tm_f32` | 3200 | 46 | 2 | 190 | 312 |
| `select_topk_from_pairs_tm_f32_v2` | 5584 | 106 | 2 | 331 | 532 |
| `select_topk_scored_pairs_tm_f32` | 3232 | 46 | 4 | 194 | 312 |
| `select_topk_scored_candidates_tm_f32` | 688 | 4 | 4 | 76 | 22 |
| `gather_topk_pairs_tm_f32` | 1600 | 69 | 165 | 105 | 0 |
| `gather_topk_scored_scalar_pairs_tm_f32` | 656 | 9 | 14 | 68 | 42 |


Interpretation:

- The row-scored path is materially lighter than the raw row selectors.
- The `v2` row selector is much larger because it duplicates constraint and score work.
- `select_topk_scored_pairs_tm_f32` is basically as large as `select_topk_from_pairs_tm_f32`, which matches the source: it is not actually pre-scored.
- `gather_topk_pairs_tm_f32` is a major address/write-heavy kernel.
- `gather_topk_metric_rows_f32` is also output-heavy relative to its actual job.

---

## Findings I would act on

## A. Fix the exact wrapper contract first

### What is wrong

The hot exact wrappers around lines 5367-6246 and 6304-7522 mostly do one of these two bad things:

1. call `double_cross_backtest_tm_flex_f32_body<...>` **without** `FIXED_M`, even though they hardcode `M = 17` or `M = 22`;
2. ignore `metrics_out_stride` and hardcode the row stride to `17` or `22`.

That means:

- the compiler cannot fully specialize fixed-width exact writes for the hottest non-windowed families,
- even if the runtime allocates padded rows, the exact wrapper throws that stride away.

### Why this matters

For `m22`, a dense stride of `22` floats means each row advances by 88 bytes. Starting from a 16-byte aligned base pointer, the next row lands at `+8 (mod 16)`, so the aligned packed path only hits intermittently.

For `m17`, there is not even a specialized fast path yet, so the best you can hope for is scalar output unless you add one.

### Concrete change

Apply the same pattern used by `windowed_m22` to the non-windowed and lockstep exact families:

- instantiate `FIXED_M = 17` or `FIXED_M = 22`,
- stop discarding `metrics_out_stride`,
- pass the caller-provided stride through to the body.

#### Representative patch: `double_cross_backtest_tm_flex_f32_m22`

```cpp
// before
(void)M;
(void)metrics_out_stride;
double_cross_backtest_tm_flex_f32_body<false, false, false>(
    /* ... */,
    flags,
    22,
    22,
    pair_index_map,
    compact_fast_periods,
    compact_slow_periods,
    compact_fast_local_rows,
    compact_slow_local_rows,
    pair_count,
    compact_output,
    metrics_out);

// after
(void)M;
const int out_stride = (metrics_out_stride > 0) ? metrics_out_stride : 22;
double_cross_backtest_tm_flex_f32_body<
    false, false, false, -1, -1, -1, -1, 22, false
>(
    /* ... */,
    flags,
    22,
    out_stride,
    pair_index_map,
    compact_fast_periods,
    compact_slow_periods,
    compact_fast_local_rows,
    compact_slow_local_rows,
    pair_count,
    compact_output,
    metrics_out);
```

Apply the same pattern to:

- `double_cross_backtest_tm_flex_f32_m17`
- all non-windowed `m17_*` wrappers
- all non-windowed `m22_*` wrappers
- `double_cross_backtest_tm_flex_f32_lockstep_time_m17`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m22`
- all lockstep `m17_*` wrappers
- all lockstep `m22_*` wrappers
- `double_cross_backtest_tm_flex_f32_windowed_m17` (this one still lacks `FIXED_M`)

### How to make this maintainable

Do **not** hand-edit dozens of wrappers permanently. Introduce a small wrapper macro or a tiny helper template so the `FIXED_M` and stride rules are declared once.

Example pattern:

```cpp
#define LAUNCH_EXACT_FIXED(NAME, LOCKSTEP, REGIME_MODE, RISK_MODE, EPS_MODE, COMM_MODE, FIXED_M, WALK_SERIES) \
extern "C" __global__ __launch_bounds__(128) void NAME(/* existing args */) { \
    (void)M; \
    const int out_stride = (metrics_out_stride > 0) ? metrics_out_stride : FIXED_M; \
    double_cross_backtest_tm_flex_f32_body< \
        false, LOCKSTEP, false, REGIME_MODE, RISK_MODE, EPS_MODE, COMM_MODE, FIXED_M, WALK_SERIES \
    >( \
        /* unchanged args */, \
        flags, FIXED_M, out_stride, \
        pair_index_map, compact_fast_periods, compact_slow_periods, \
        compact_fast_local_rows, compact_slow_local_rows, \
        pair_count, compact_output, metrics_out \
    ); \
}
```

That change alone fixes the exact-wrapper asymmetry that is visible in both source and SASS.

---

## B. Add a real `m17` fixed-width store path

### What is wrong

`store_metrics_row_fixed_f32` only has a packed specialization for `N == 22`. `N == 17` falls through to scalar stores.

### Concrete change

Add a dedicated `N == 17` branch that writes the first 16 values as four `float4` stores and the final value as one scalar. This is only worth doing if the row stride is padded/aligned consistently, so pair it with the exact-wrapper stride fix above.

#### Suggested patch in `store_metrics_row_fixed_f32`

```cpp
if constexpr (N == 17) {
    float* const row = metrics_out + base;
    if ((((uintptr_t)row) & (sizeof(float4) - 1u)) == 0u) {
        float4* const row4 = reinterpret_cast<float4*>(row);
        row4[0] = make_float4(pnl, (float)trades, max_dd, mean);
        row4[1] = make_float4(std, exposure, signed_net_expo, downside_dev);
        row4[2] = make_float4(ulcer_index, omega, downside_frequency, downside_severity);
        row4[3] = make_float4(long_mean_return, short_mean_return, kappa_tau, underwater_fraction);
        row[16] = (float)max_underwater_duration_bars;
        return;
    }

    float2* const row2 = reinterpret_cast<float2*>(row);
    row2[0] = make_float2(pnl, (float)trades);
    row2[1] = make_float2(max_dd, mean);
    row2[2] = make_float2(std, exposure);
    row2[3] = make_float2(signed_net_expo, downside_dev);
    row2[4] = make_float2(ulcer_index, omega);
    row2[5] = make_float2(downside_frequency, downside_severity);
    row2[6] = make_float2(long_mean_return, short_mean_return);
    row2[7] = make_float2(kappa_tau, underwater_fraction);
    row[16] = (float)max_underwater_duration_bars;
    return;
}
```

### Recommended stride for exact rows

If you want this to actually stay aligned row-to-row:

- use **24** for `m22`,
- use **20** or **24** for `m17`.

The repo already computes `tile_metrics_stride` in Rust via `effective_metrics_stride(...)`, so the lowest-churn version is:

- keep using that runtime stride machinery,
- stop discarding `metrics_out_stride` in the exact wrappers.

---

## C. Do not enable padded exact rows until the row reducers understand `metrics_stride`

### New finding that matters

The row-based kernels currently assume **dense `M` stride**, not `metrics_stride`.

Examples from source:

- `score_metric_rows_f32`: `base = i * M`
- `select_topk_metric_rows_f32`: `base = idx * M`
- `select_topk_metric_rows_from_candidates_f32`: `base = row * M`
- `select_topk_metric_rows_f32_v2`: `base = idx * M`
- `gather_topk_metric_rows_f32`: `base_in = row * M`
- `pack_metric_rows_f32`: `base_in = i * M`

So if you fix the exact wrappers to honor padded `metrics_out_stride`, the row reducer path becomes inconsistent unless you also fix these kernels.

### Concrete change

Thread `metrics_stride` through every row kernel that reads metric rows.

#### Minimal patch shape

```cpp
extern "C" __global__
void score_metric_rows_f32(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    int metrics_stride,
    int objective,
    float hurdle_tau,
    float* __restrict__ out_scores
) {
    const int stride = (metrics_stride > 0) ? metrics_stride : M;
    for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < rows; i += blockDim.x * gridDim.x) {
        const size_t base = (size_t)i * (size_t)stride;
        float score = objective_score_f32(metrics_rows, base, M, objective, hurdle_tau);
        if (!isfinite(score)) score = NEG_INF_F;
        out_scores[i] = score;
    }
}
```

Apply the same input/stride pattern to:

- `select_topk_metric_rows_f32`
- `select_topk_metric_rows_from_candidates_f32`
- `select_topk_metric_rows_f32_v2`
- `select_topk_metric_rows_from_candidates_f32_v2`
- `gather_topk_metric_rows_f32`
- `pack_metric_rows_f32`

### Why I recommend this over a repack pass

A separate repack kernel would work, but it adds more memory traffic to a route that is already output-heavy. The cleaner fix is to make the row consumers stride-aware once.


### Required runtime wiring in `gpu_backtest_kernel.rs`

The Rust side already has the right building blocks:

- it computes `metrics_pad8_enabled` and `tile_metrics_stride`,
- it allocates `tile_out_cap` from `pf_tile * ps_tile * tile_metrics_stride`,
- it already has `compact_valid_pair_output` / `tile_rows_compact`,
- and it still routes global row reduction through `reduce_metric_rows_topk_on_device(...)` with the legacy raw row selectors.

That means the runtime changes are concrete, not architectural:

1. when launching exact `m17` / `m22` kernels, pass the existing `tile_metrics_stride` all the way through and stop relying on the wrapper’s current forced dense stride;
2. update the row-kernel launch signatures so row score/select/gather kernels receive `metrics_stride`;
3. in `reduce_metric_rows_topk_on_device(...)`, switch the order from “select raw rows, then gather” to “score rows, select scored rows, then gather final rows”;
4. if a route only needs `(fast, slow, score)`, use the existing score-array pair gather instead of full pair-metric gather.

This is why I recommend fixing the wrapper contract and the row-kernel stride contract together. Doing only one side would leave the pipeline inconsistent.

---

## D. Add a score-once row path for global top-k and stop using raw row selectors by default

### What is wrong

The source already has the scored-row family, but the runtime still drives `reduce_metric_rows_topk_on_device(...)` through:

- `select_topk_metric_rows_f32`
- `select_topk_metric_rows_from_candidates_f32`
- their `v2` variants
- `gather_topk_metric_rows_f32`

Meanwhile `gpu_backtest_kernel.rs` already loads:

- `select_topk_metric_rows_f32`
- `select_topk_metric_rows_from_candidates_f32`
- `select_topk_metric_rows_f32_v2`
- `select_topk_metric_rows_from_candidates_f32_v2`
- `gather_topk_metric_rows_f32`

and the reduction path uses `reduce_metric_rows_topk_on_device(...)` with that legacy row family.

### Concrete change

Use the kernels you already have:

- `score_metric_rows_f32`
- `select_topk_scored_rows_f32`
- `select_topk_scored_rows_from_candidates_f32`

and keep `gather_topk_metric_rows_f32` only for the final row copy.

#### New kernel to add

The cleanest version is a constrained score kernel so the `v2` selectors disappear from the hot path:

```cpp
extern "C" __global__
void score_metric_rows_f32_v2(
    const float* __restrict__ metrics_rows,
    int rows,
    int M,
    int metrics_stride,
    int objective,
    float hurdle_tau,
    int use_min_trades,
    float min_trades,
    int use_max_dd,
    float max_dd,
    int use_min_sharpe,
    float min_sharpe,
    int use_min_sortino,
    float min_sortino,
    int use_min_calmar,
    float min_calmar,
    int use_max_volatility,
    float max_volatility,
    int use_min_exposure,
    float min_exposure,
    int use_max_exposure,
    float max_exposure,
    int use_max_abs_net_exposure,
    float max_abs_net_exposure,
    int use_min_omega,
    float min_omega,
    int use_max_ulcer,
    float max_ulcer,
    int use_max_downside_deviation,
    float max_downside_deviation,
    float* __restrict__ out_scores
) {
    const int stride = (metrics_stride > 0) ? metrics_stride : M;
    for (int i = blockIdx.x * blockDim.x + threadIdx.x; i < rows; i += blockDim.x * gridDim.x) {
        const size_t base = (size_t)i * (size_t)stride;
        float score = NEG_INF_F;
        if (passes_constraints_f32(
                metrics_rows, base, M, hurdle_tau,
                use_min_trades, min_trades,
                use_max_dd, max_dd,
                use_min_sharpe, min_sharpe,
                use_min_sortino, min_sortino,
                use_min_calmar, min_calmar,
                use_max_volatility, max_volatility,
                use_min_exposure, min_exposure,
                use_max_exposure, max_exposure,
                use_max_abs_net_exposure, max_abs_net_exposure,
                use_min_omega, min_omega,
                use_max_ulcer, max_ulcer,
                use_max_downside_deviation, max_downside_deviation)) {
            score = objective_score_f32(metrics_rows, base, M, objective, hurdle_tau);
            if (!isfinite(score)) score = NEG_INF_F;
        }
        out_scores[i] = score;
    }
}
```

#### Runtime sequence to use instead

```rust
// legacy today:
// select_topk_metric_rows_* -> gather_topk_metric_rows_f32

// recommended:
score_metric_rows_f32_v2_or_legacy(..., d_scores);
select_topk_scored_rows_f32_or_candidates(..., d_scores, d_top_idx);
gather_topk_metric_rows_f32(..., global_metrics, rows, metrics_count, metrics_stride, global_fast, global_slow, d_top_idx, topk, d_top_fast, d_top_slow, d_top_metrics);
```

Why this is the right change:

- it removes duplicated objective work from selectors,
- it lets the lighter scored selector family do the sorting,
- it makes constraint handling a score-generation concern instead of a selector concern.

---

## E. Stop treating `select_topk_scored_pairs_tm_f32*` as the true score-once pair path

### What is wrong

Despite the names, these kernels still take `metrics`, `M`, `objective`, and `hurdle_tau`, and still call `objective_score_f32(...)` inside the selector loop:

- `select_topk_scored_pairs_tm_f32`
- `select_topk_scored_pairs_tm_f32_v2`

So they are not the true score-array route.

### Concrete change

For pair selection that only needs parameter IDs plus score, use:

1. `score_metric_rows_f32` or `score_metric_rows_f32_v2`
2. `select_topk_scored_candidates_tm_f32`
3. `gather_topk_scored_scalar_pairs_tm_f32`

That is the real pre-scored path already present in the source.

### Runtime consequence

If the route only needs `(fast, slow, score)` and not the full metric row immediately, prefer the scalar-score gather. Only use `gather_topk_pairs_tm_f32` when the full metrics row is truly needed right there.

---

## F. Add fixed-row copy helpers for the gather kernels

### What is wrong

`gather_topk_metric_rows_f32` and `gather_topk_pairs_tm_f32` currently copy metric rows with scalar loops:

```cpp
for (int m = 0; m < M; ++m) out_metrics[base_out + m] = metrics_rows[base_in + m];
```

That is exactly the sort of output-side work the static SASS makes look expensive.

### Concrete change

Add a copy helper parallel to the store helper.

```cpp
template <int N>
static __device__ __forceinline__ void copy_metrics_row_fixed_f32(
    const float* __restrict__ in,
    size_t base_in,
    float* __restrict__ out,
    size_t base_out)
{
    const float* src = in + base_in;
    float* dst = out + base_out;

    if constexpr (N == 22) {
        if ((((uintptr_t)src | (uintptr_t)dst) & (sizeof(float4) - 1u)) == 0u) {
            const float4* src4 = reinterpret_cast<const float4*>(src);
            float4* dst4 = reinterpret_cast<float4*>(dst);
            dst4[0] = src4[0];
            dst4[1] = src4[1];
            dst4[2] = src4[2];
            dst4[3] = src4[3];
            dst4[4] = src4[4];
            reinterpret_cast<float2*>(dst)[10] = reinterpret_cast<const float2*>(src)[10];
            return;
        }
    }

    if constexpr (N == 17) {
        if ((((uintptr_t)src | (uintptr_t)dst) & (sizeof(float4) - 1u)) == 0u) {
            const float4* src4 = reinterpret_cast<const float4*>(src);
            float4* dst4 = reinterpret_cast<float4*>(dst);
            dst4[0] = src4[0];
            dst4[1] = src4[1];
            dst4[2] = src4[2];
            dst4[3] = src4[3];
            dst[16] = src[16];
            return;
        }
    }

#pragma unroll
    for (int i = 0; i < N; ++i) dst[i] = src[i];
}
```

Then use it in:

- `gather_topk_metric_rows_f32`
- `gather_topk_pairs_tm_f32`
- `pack_metric_rows_f32`

#### Example in `gather_topk_metric_rows_f32`

```cpp
const size_t base_in  = (size_t)row * (size_t)metrics_stride;
const size_t base_out = (size_t)i   * (size_t)M;

if (M == 22) {
    copy_metrics_row_fixed_f32<22>(metrics_rows, base_in, out_metrics, base_out);
} else if (M == 17) {
    copy_metrics_row_fixed_f32<17>(metrics_rows, base_in, out_metrics, base_out);
} else {
    for (int m = 0; m < M; ++m) out_metrics[base_out + (size_t)m] = metrics_rows[base_in + (size_t)m];
}
```

This is one of the clearest repo-specific output-path improvements suggested by the artifacts.

---

## G. Constraint duplication should move out of the selector kernels

### What is wrong

`passes_constraints_f32` already calls `objective_score_f32(...)` for:

- sharpe,
- sortino,
- calmar,
- omega.

Then the `v2` selector computes `objective_score_f32(...)` again for ranking.

This is explicit duplication in:

- `select_topk_metric_rows_f32_v2`
- `select_topk_metric_rows_from_candidates_f32_v2`
- `select_topk_from_pairs_tm_f32_v2`
- `select_topk_from_candidates_tm_f32_v2`
- `select_topk_scored_pairs_tm_f32_v2`

### Best fix

The best first move is **not** to micro-optimize `passes_constraints_f32`. It is to move constraint handling into the score generation stage:

- `score_metric_rows_f32_v2` for rows,
- or a score-buffer preparation stage for pair routes.

After that, the selectors only sort scores.

### Secondary cleanup if you stay with the selector design

If you are not ready to switch the runtime to score-once immediately, at least fuse constraint + objective evaluation in one helper:

```cpp
static __device__ __forceinline__ float constrained_objective_score_f32(/* same args */) {
    if (!passes_constraints_f32(/* ... */)) return NEG_INF_F;
    float score = objective_score_f32(/* ... */);
    return isfinite(score) ? score : NEG_INF_F;
}
```

That avoids some repeated call structure and makes the next refactor easier.

### Additional finding: dense invalid-row materialization is still costing writes

Inside `double_cross_backtest_tm_flex_f32_body`, early invalid exits such as:

- `STRAT_ENFORCE_FAST_LT_SLOW` rejecting `period_f >= period_s`,
- or `start_t >= end_t`

still fall back to `zero_metrics_row_fixed_f32<FIXED_M>(...)` for fixed-width rows.

That is not automatically wrong, because some host/export contracts still want a dense row space. But it means the body is still paying a full row write for combinations that will never contend in top-k.

The safest performance version is **not** to stop writing rows unconditionally. It is:

1. keep the current dense-zero behavior for `include_all` / export / any mode that needs stable dense row identity;
2. after the score-once path lands, allow top-k-only modes to treat these rows as `NEG_INF` at score generation time and avoid carrying them forward through the row/pair reduction path;
3. if the valid-pair maps already exclude `fast >= slow`, keep that invariant strong and avoid falling back to dense invalid rows in compact routes.

In other words: this is a real write-side cost, but it becomes worth changing only after the row-score path is already in place.

---

## H. Keep the pair output path compact as long as possible

### What is wrong

`gather_topk_pairs_tm_f32` is extremely address/write heavy in the attached SASS. It spends a lot of work on:

- resolving dense pair IDs,
- deriving `pf_local` / `ps_local`,
- loading fast/slow lengths,
- copying full metric rows.

### Concrete change

For pair routes:

- use `compact_fast_rows` / `compact_slow_rows` whenever available,
- use `gather_topk_scored_scalar_pairs_tm_f32` if you only need `(fast, slow, score)`,
- only fall back to `gather_topk_pairs_tm_f32` when the full metrics row is required.

This is more important than trying to hand-optimize the selector’s CUB staging code.

---

## I. What I would *not* spend time on yet

### 1) Broad PTX/SASS surgery

Not justified from the attached artifacts.

### 2) Selector shared-memory/barrier cleanup

The `LDS` / `STS` / `BAR.SYNC` patterns look like expected CUB block-sort structure, not a bug.

### 3) Slowpath relocation hunting

The `sqrt` and `div` slowpaths on the backtest wrappers and selector/score kernels match the source formulas. They are not automatically a missed optimization.

### 4) fast-warp-dedup as a universal default

`ld_tm_fast_warp_dedup` is reasonable source code, but the `double_cross_backtest_tm_flex_f32_fast_warp_dedup` static body is slightly heavier than the generic wrapper. Keep it as a conditional specialization, not a blanket change.

---

## Small but real follow-up cleanup

These are lower priority, but real:

1. `score_metric_rows_f32` already canonicalizes non-finite scores to `NEG_INF_F`, but:
   - `select_topk_scored_rows_f32`
   - `select_topk_scored_rows_from_candidates_f32`
   - `gather_topk_scored_rows_f32`
   - `gather_topk_scored_pairs_tm_f32`
   - `gather_topk_scored_scalar_pairs_tm_f32`

   still re-check `isfinite(...)`.

   After score generation is the only producer, these re-checks can be removed.

2. `pack_metric_rows_f32` should gain `metrics_stride` once the exact wrappers start honoring padded stride.

3. `windowed_m17` should be made consistent with the specialized windowed `m17` wrappers by instantiating `FIXED_M = 17`.

---

## Ordered implementation plan

## Phase 1 — highest confidence, best ROI

1. Fix the exact-wrapper template/stride contract:
   - add `FIXED_M` to all non-windowed and lockstep `m17` / `m22` wrappers,
   - stop discarding `metrics_out_stride`,
   - fix generic `windowed_m17`.

2. Thread `metrics_stride` through the row consumers:
   - `score_metric_rows_f32`
   - `select_topk_metric_rows_f32`
   - `select_topk_metric_rows_from_candidates_f32`
   - `select_topk_metric_rows_f32_v2`
   - `select_topk_metric_rows_from_candidates_f32_v2`
   - `gather_topk_metric_rows_f32`
   - `pack_metric_rows_f32`

3. Add `score_metric_rows_f32_v2` and switch `reduce_metric_rows_topk_on_device(...)` to:
   - score once,
   - sort scores,
   - gather only the final top rows.

This phase is the one most strongly supported by the static evidence.

## Phase 2 — still high confidence

4. Add the `N == 17` packed store path.
5. Add `copy_metrics_row_fixed_f32<17/22>` and use it in gather/pack kernels.
6. For pair routes that only need scores, switch to the true score-array path:
   - `score_metric_rows_f32(_v2)`
   - `select_topk_scored_candidates_tm_f32`
   - `gather_topk_scored_scalar_pairs_tm_f32`

## Phase 3 — after the above lands

7. Simplify or retire the fake-scored pair selectors:
   - `select_topk_scored_pairs_tm_f32`
   - `select_topk_scored_pairs_tm_f32_v2`

8. Consider tighter invalid-row materialization rules in top-k-only modes.

## Phase 4 — only if profiling still points there

9. Revisit the exact `m22` write path **only if** padded aligned rows plus `FIXED_M` still produce scalar-only stores in the emitted cubin.

---

## What still needs runtime / Nsight evidence

Static analysis cannot settle these questions:

- whether the real bottleneck is still the backtest body or has shifted to row/pair reduction,
- whether `m17` or `m22` dominates the route mix on your actual workloads,
- whether the lockstep families matter enough in production to justify equal effort,
- whether the score-once row path wins end-to-end once extra score-buffer traffic is included,
- whether `fast_warp_dedup` helps on your actual `(t, pf_local)` duplication pattern,
- whether any of the visible secondary `LDL` / `STL` activity in the windowed family is meaningful in live runs.

But none of those uncertainties change the order of operations above. The attached artifacts are already strong enough to justify the Phase 1 and Phase 2 changes.

---

## Bottom line

If I were editing this repo next, I would do the following in order:

1. **Normalize exact `m17` / `m22` wrappers so they instantiate `FIXED_M` and honor `metrics_out_stride`.**
2. **Make the row reducer family stride-aware.**
3. **Move global row top-k to a score-once path.**
4. **Add fixed-width copy/store helpers for `m17` / `m22`.**
5. **Use the real score-array pair path instead of the misleading fake-scored pair selectors.**

That is the most evidence-backed route to better performance in this codebase without jumping prematurely into hand-tuned cubin work.
