# Double-MA GPU Finalist Recompute Optimization Plan

## Scope

This document turns the earlier analysis into a concrete implementation plan. It is grounded in the uploaded prompt guide and the current Rust/CUDA code. The focus is the performance penalty of the **top-50 finalist recompute** that happens after the reduced first-pass search.

Primary files referenced here:

- `DOUBLE_MA_SEARCH_PRODUCT_GAP_REVIEW_2026-03-22.md`
- `double_ma_opt.rs`
- `gpu_backtest_kernel.rs`
- `double_crossover.cu`
- `double_ma.rs`
- `stream.rs`

---

## Executive summary

The code and the math point to four practical conclusions:

1. **The benchmark gap is not a pure “top-50 recompute tax.”**  
   The benchmark pair bundles heatmap and finalist recompute together. In `double_ma_opt.rs:602-645`, `search_only` disables both heatmap and finalist recompute, while `end_to_end_product` enables both.

2. **The current finalist recompute is already pair-correct, and it already has some small-K specialization.**  
   The recompute path deduplicates finalists and runs only exact finalists through `eval_double_ma_batch_gpu_kernel_impl()` (`gpu_backtest_kernel.rs:5530-5611`). The exact-combo executor already has a low-pair M7 fast path (`gpu_backtest_kernel.rs:5262-5281`).

3. **The most concrete near-term win is to reuse pass-1 device state in pass 2.**  
   Today both the search path and the finalist path recompute `d_lr` and `d_ret1` from `d_close` over the full series (`gpu_backtest_kernel.rs:6395-6416` and `4797-4817`). That is avoidable.

4. **The best higher-level A/B test is not “go straight to full 10 metrics.”**  
   The strongest no-replay experiment is a **9-row product-reduced search row**, not 10. The current GPU product contract can reconstruct `pnl`, `max_dd`, `sharpe`, `sortino`, `calmar`, `volatility`, `exposure`, `net_exposure`, `downside_deviation`, `ulcer_index`, and `martin_ratio` from a reduced row with:
   - log-equity,
   - trades,
   - max-log-drawdown,
   - mean,
   - std,
   - exposure,
   - net exposure,
   - downside deviation,
   - ulcer index.

   `martin_ratio` can be derived on CPU from `pnl / ulcer_index` with the same clamp logic as `finalize_risk_metrics()` in `double_ma.rs:487-569`.  
   This makes **9** the minimal emitted row width for current GPU product parity.  
   Important nuance: the current GPU decode path already leaves `omega = 0.0`, even for full metric rows (`gpu_backtest_kernel.rs:724-742`, `8764-8781`), so this preserves the **current GPU product contract**, not an idealized future Omega implementation.

---

## What the current code already does correctly

Before changing anything, it is important to note what is **not** wrong.

### 1) The recompute path does not rerun the full original grid

The finalist recompute wrapper:

- deduplicates finalists (`gpu_backtest_kernel.rs:5545-5564`),
- builds unique fast/slow finalist period subsets (`5566-5577`),
- then runs only the exact finalist combinations through `eval_double_ma_batch_gpu_kernel_impl()` (`5579-5597`).

That means the pass-2 problem is **not** “it reruns too many pairs.”

### 2) The exact-combo path already has a low-pair M7 bias

Inside the exact-combo executor, the code explicitly recognizes that full-metric finalist reruns are tiny in pair count and allows a recompute-only low-pair M7 fast path (`gpu_backtest_kernel.rs:5262-5281`).

So “force M7 on small finalist reruns” is **already implemented**.

### 3) The exact-combo path already copies dense rows directly

The exact-combo executor does **not** use `pack_metric_rows_for_host_copy()` for pass 2. It synchronizes, copies the tile output directly to host, and decodes rows (`gpu_backtest_kernel.rs:5486-5505`).

That means `pack_metric_rows_for_host_copy()` is a **search-finalize concern**, not the main pass-2 concern. The search path uses it when pulling top-k rows from device-side global buffers (`gpu_backtest_kernel.rs:8124-8149`).

### 4) The host `StreamAggregator` is not the main bottleneck

`stream.rs:1-220` is a CPU heap/best/heatmap aggregator. In the GPU route it only sees selected rows/final results, not all 58,300 candidates. That matches the guide’s conclusion that it is not the main explanation for the large gap.

---

## Ground truth from the benchmark and kernel contract

### Benchmark split is bundled

In `double_ma_opt.rs:602-645`:

- `base_req` sets:
  - `objective = Sharpe`
  - `top_k = 50`
  - `include_all = false`
  - `heatmap_bins = 64`
- `search_only` mutates `heatmap_bins = 0` (`613-614`) and runs with `VECTORBT_KERNEL_FINALIST_RECOMPUTE=0` (`625-630`).
- `end_to_end_product` keeps `heatmap_bins = 64` and enables finalist recompute (`638-643`).

So the benchmark gap includes:

- heatmap kernel time,
- graph-replay opportunity loss when heatmap is present (`gpu_backtest_kernel.rs:6602-6608`),
- finalist recompute.

### Sharpe search currently emits 5 rows

`search_metrics_count_for_optimization()` (`gpu_backtest_kernel.rs:3350-3377`) returns:

- `1` for `Pnl`
- `5` for `Sharpe` and `Volatility`
- `3` for `MaxDrawdown` and `Calmar`
- `METRICS_COUNT` for `Sortino` and `Omega`

So the ALMA/ALMA benchmark is running a **5-row search pass**.

### Reduced search rows are log-space in slots 0 and 2

The kernel writes reduced rows in log-space when `M < 10` (`double_crossover.cu:3749-3759`):

- slot 0 = `log_eq`
- slot 2 = `max_log_dd`

The Rust decode path restores arithmetic `pnl` and `max_dd` when `metrics_count < METRICS_COUNT` (`gpu_backtest_kernel.rs:648-742`).

This is important because it means a **9-row reduced-search path** still works with the current search-row log-space contract.

---

## Refined cost model

The earlier simple model was directionally correct, but the code suggests a better decomposition.

### Symbols

Use:

- `N` = total candidate pairs
- `K` = finalist count
- `T` = candle count
- `P_f` = unique finalist fast periods
- `P_s` = unique finalist slow periods
- `C_5(T)` = pass-1 per-pair effective cost for current 5-row Sharpe search
- `C_9(T)` = pass-1 per-pair effective cost for a 9-row product-reduced search
- `C_10(T)` = pass-1 per-pair effective cost for full 10-row output
- `C_ret(T)` = cost of generating `d_lr` and `d_ret1`
- `C_ma_f(P_f, T)` = finalist fast-MA preparation cost
- `C_ma_s(P_s, T)` = finalist slow-MA preparation cost
- `C_bt_full(T)` = full-metric finalist backtest cost per pair
- `U(K, T)` = effective finalist replay utilization factor
- `F_r` = fixed finalist orchestration cost excluding the major time-axis kernels
- `H` = heatmap + graph-replay opportunity loss

For the benchmark in the guide:

- `N ≈ 58,300`
- `K ≈ 50`
- `T = 200,000`
- `K / N ≈ 8.6e-4`

### Current paths

Current search-only:

\[
C_{\text{search-only}} = F_s + N \, C_5(T)
\]

Current end-to-end product path:

\[
C_{\text{product,current}} =
F_s + N\,C_5(T)
+ H
+ F_r
+ C_{\text{ret}}(T)
+ C_{\text{ma,f}}(P_f,T)
+ C_{\text{ma,s}}(P_s,T)
+ \frac{K\,C_{\text{bt,full}}(T)}{U(K,T)}
+ C_{\text{copy}}(K)
\]

Naive one-pass full metrics:

\[
C_{\text{product,10}} = F_s' + N\,C_{10}(T) + H
\]

One-and-a-half-pass 9-row product-reduced search (no replay):

\[
C_{\text{product,9}} = F_s + N\,C_9(T) + H
\]

### Break-even conditions

#### Current two-pass beats naive full 10-row one-pass iff

\[
N\big(C_{10}(T)-C_5(T)\big)
>
F_r
+ C_{\text{ret}}(T)
+ C_{\text{ma,f}}(P_f,T)
+ C_{\text{ma,s}}(P_s,T)
+ \frac{K\,C_{\text{bt,full}}(T)}{U(K,T)}
+ C_{\text{copy}}(K)
+ (F_s - F_s')
\]

Interpretation:

- The left-hand side pays for widening **all 58,300 pairs**.
- The right-hand side is the current replay tax.
- This is why “just move everything into pass 1” is not the default recommendation.

#### 9-row product-reduced search beats current two-pass iff

\[
N\big(C_9(T)-C_5(T)\big)
>
\text{replay tax}
\]

or equivalently

\[
N\big(C_9(T)-C_5(T)\big)
<
F_r
+ C_{\text{ret}}(T)
+ C_{\text{ma,f}}(P_f,T)
+ C_{\text{ma,s}}(P_s,T)
+ \frac{K\,C_{\text{bt,full}}(T)}{U(K,T)}
+ C_{\text{copy}}(K)
\]

Interpretation:

- This is the real A/B that matters.
- `C_9 - C_5` is the extra cost of turning on:
  - exposure,
  - net exposure,
  - downside deviation,
  - ulcer index
  for **all pairs** in pass 1.
- The replay side includes:
  - full-series returns,
  - finalist MA generation,
  - full-metric finalist backtest,
  - fixed orchestration.

This is why the right experiment is **5-row vs 9-row**, not necessarily **5-row vs 10-row**.

### Small-finalist utilization

If the replay parallelizes mostly over pairs, available parallel work is roughly proportional to `K`, not `N`.

With `K ≈ 50`, pair-parallel replay inherently has much less available concurrency than the search sweep. The current exact-combo path already recognizes this by preferring an M7 specialization for low-pair reruns (`gpu_backtest_kernel.rs:5262-5281`), but the structural issue remains:

- pass 1: many-pair sweep amortizes setup
- pass 2: tiny-pair replay still walks the full time axis

This is why even a mathematically small replay can be disappointing in practice.

---

## Minimal sufficient statistics and minimal emitted row

There are two distinct questions:

1. What is the **internal sufficient-statistics state** needed to reconstruct the current GPU product metrics?
2. What is the **minimal emitted row width** needed so the current Rust decode path can reconstruct the same output without replay?

### Internal sufficient-statistics state

From the kernel body (`double_crossover.cu:3678-3759`) and `finalize_risk_metrics()` (`double_ma.rs:487-569`), an exact per-pair internal state sufficient for current GPU product metrics is:

- `log_eq`
- `trades`
- `max_log_dd`
- `n`
- `mean`
- `m2`
- `obs`
- `sum_neg_sq`
- `ulcer_sum_sq`
- `sum_abs_pos`
- `sum_pos`

From that state:

- `pnl = exp(log_eq) - 1`
- `max_dd = -expm1(-max_log_dd)`
- `std = sqrt(m2 / (n - 1))` if `n > 1`
- `sharpe = mean / std`
- `downside_deviation = sqrt(sum_neg_sq / obs)`
- `sortino = mean / downside_deviation`
- `ulcer_index = sqrt(ulcer_sum_sq / obs)`
- `martin_ratio = pnl / ulcer_index` with clamp logic
- `exposure = sum_abs_pos / n`
- `net_exposure = sum_pos / n`
- `calmar = pnl / max_dd` with clamp logic

### Minimal emitted row for current GPU product parity

You do **not** need to emit all of the raw sufficient-statistics state if you only care about current GPU output parity.

The minimal emitted row is:

\[
S_{\text{row,9}} =
\{
\log eq,\,
trades,\,
max\_log\_dd,\,
mean,\,
std,\,
exposure,\,
net\_exposure,\,
downside\_deviation,\,
ulcer\_index
\}
\]

That is **9 rows**.

Why 9 and not 10?

- `martin_ratio` can be derived on CPU from `pnl` and `ulcer_index`.
- `calmar` is already derived on CPU from `pnl` and `max_dd`.
- `sharpe` and `sortino` are already derived on CPU from `mean/std` and `mean/downside`.
- `omega` is already `0.0` in the current GPU decode path, even at `METRICS_COUNT`.

So the minimal emitted row for current GPU product parity is **9**, not **10**.

---

## What to implement first

### Priority order

1. **Split the benchmark**
2. **Add pass-2 timing instrumentation**
3. **Cache `d_lr` and `d_ret1` across pass 1 and pass 2**
4. **A/B a 9-row product-reduced mode**
5. **Only if replay is still too costly, add a dedicated small-finalist fast path / device-side handoff**

That order gives the best ratio of engineering effort to expected payoff.

---

## Patch set A — split the benchmark cleanly

### Why

Right now the benchmark conflates:

- search cost,
- heatmap cost,
- replay cost.

That prevents good optimization decisions.

### Where

`double_ma_opt.rs:602-645`

### What to add

Create four benchmark variants:

1. `search_only`
2. `search_plus_heatmap`
3. `search_plus_finalist_recompute`
4. `end_to_end_product`

### Concrete sketch

```rust
let base_req = DoubleMaOptRequest {
    // existing fields...
    objective: ObjectiveKind::Sharpe,
    top_k: Some(50),
    include_all: Some(false),
    heatmap_bins: Some(64),
    // ...
};

let mut search_only_req = base_req.clone();
search_only_req.heatmap_bins = Some(0);

let search_plus_heatmap_req = base_req.clone();

let mut search_plus_finalist_req = base_req.clone();
search_plus_finalist_req.heatmap_bins = Some(0);

let end_to_end_req = base_req.clone();

let variants: [(&str, DoubleMaOptRequest, bool); 4] = [
    ("large/{pairs}pairs/search_only", search_only_req, false),
    ("large/{pairs}pairs/search_plus_heatmap", search_plus_heatmap_req, false),
    ("large/{pairs}pairs/search_plus_finalist_recompute", search_plus_finalist_req, true),
    ("large/{pairs}pairs/end_to_end_product", end_to_end_req, true),
];

for (name_tmpl, req0, recompute_enabled) in variants {
    let name = format!(name_tmpl);
    group.bench_function(&name, |b| {
        b.iter(|| {
            let req = req0.clone();
            run_with_finalist_recompute_env(recompute_enabled, || {
                let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                    req, &candles, &cancel, None,
                )
                .expect("gpu kernel run");
            });
        })
    });
}
```

### Result

After this split, you can measure:

- raw search cost,
- heatmap-only tax,
- replay-only tax,
- full bundled product cost.

Without this split, every further optimization result stays ambiguous.

---

## Patch set B — add finalist replay timing instrumentation

### Why

You need hard data on whether pass 2 is dominated by:

- returns generation,
- MA generation,
- backtest kernel,
- sync/D2H.

The code already has a pipeline profile structure for the search path (`gpu_backtest_kernel.rs:173-256`). Mirror that style for replay.

### Where

Add instrumentation inside `eval_double_ma_batch_gpu_kernel_impl()` around:

- returns generation: `4797-4817`
- fast MA generation: `4939-5024`
- slow MA generation: `5056-5142`
- pair-map build/upload: `5158-5200`
- backtest launch: `5203-5483`
- sync/D2H/decode: `5486-5505`

### Concrete structure

```rust
#[derive(Default, Debug)]
struct FinalistReplayTimingProfile {
    returns: Duration,
    fast_ma: Duration,
    slow_ma: Duration,
    pairmap: Duration,
    backtest: Duration,
    d2h_decode: Duration,
    tiles: usize,
}

fn kernel_finalist_profile_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_FINALIST_PROFILE", false)
}
```

Use `Instant::now()` around each section when the env flag is enabled.

### Why this is worth doing

This is the fastest way to confirm whether the best next patch is:

- returns reuse,
- 9-row no-replay mode,
- or a more invasive finalist executor redesign.

---

## Patch set C — cache `d_lr` and `d_ret1` across pass 1 and pass 2

### Why

Both the search path and the finalist path recompute:

- log returns `d_lr`
- simple returns `d_ret1`

from the same resident `d_close` over the full series.

That happens in:

- search path: `gpu_backtest_kernel.rs:6395-6416`
- exact-combo path: `gpu_backtest_kernel.rs:4797-4817`

This is duplicated work inside a single optimization flow.

### Where to patch

#### 1) Extend `KernelScratch`

Current struct is at `gpu_backtest_kernel.rs:770-790`.

Add cache metadata:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReturnsCacheMode {
    LogAndSimple,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReturnsCacheKey {
    dataset: CandleDatasetSignature,
    mode: ReturnsCacheMode,
}

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

    d_heatmap: Option<DeviceBuffer<f32>>,
    d_cand_a: Option<DeviceBuffer<i32>>,
    d_cand_b: Option<DeviceBuffer<i32>>,
    d_top_fast: Option<DeviceBuffer<i32>>,
    d_top_slow: Option<DeviceBuffer<i32>>,
    d_top_metrics: Option<DeviceBuffer<f32>>,
    d_top_packed: Option<DeviceBuffer<f32>>,
    d_valid_pair_index: Option<DeviceBuffer<i32>>,

    returns_key: Option<ReturnsCacheKey>,
}
```

#### 2) Add a helper

Place it near `ensure_resident_market_series()` utilities or near the returns code:

```rust
fn ensure_log_and_simple_returns(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    d_close: &DeviceBuffer<f32>,
    n: usize,
    dataset_sig: CandleDatasetSignature,
    compute_returns: &Function,
    device_launch_caps: DeviceLaunchCaps,
    compute_returns_launch_caps: KernelLaunchCaps,
) -> Result<(), String> {
    KernelScratch::ensure_len_f32(&mut scratch.d_lr, n, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_ret1, n, bt_stream)?;

    let want = ReturnsCacheKey {
        dataset: dataset_sig,
        mode: ReturnsCacheMode::LogAndSimple,
    };

    if scratch.returns_key == Some(want) {
        return Ok(());
    }

    let d_lr_ptr = scratch.d_lr.as_ref().unwrap().as_device_ptr().as_raw();
    let d_ret1_ptr = scratch.d_ret1.as_ref().unwrap().as_device_ptr().as_raw();

    unsafe {
        let mut prices_ptr = d_close.as_device_ptr().as_raw();
        let mut t_i = n as i32;
        let mut lr_out_ptr = d_lr_ptr;
        let mut ret1_out_ptr = d_ret1_ptr;
        let launch = choose_launch_shape_1d(n, device_launch_caps, compute_returns_launch_caps);
        let block_x = launch.block_x;
        let grid_x = launch.grid_x;
        let args: &mut [*mut c_void] = &mut [
            &mut prices_ptr as *mut _ as *mut c_void,
            &mut t_i as *mut _ as *mut c_void,
            &mut lr_out_ptr as *mut _ as *mut c_void,
            &mut ret1_out_ptr as *mut _ as *mut c_void,
        ];
        bt_stream
            .launch(compute_returns, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
            .map_err(|e| e.to_string())?;
    }

    scratch.returns_key = Some(want);
    Ok(())
}
```

#### 3) Replace duplicated returns launches

In the exact-combo executor, replace `gpu_backtest_kernel.rs:4797-4817` with a call to the helper.

In the search path, replace `gpu_backtest_kernel.rs:6395-6416` with a call to the same helper.

Use:

```rust
let dataset_sig = candle_dataset_signature(candles);
ensure_log_and_simple_returns(
    bt_stream,
    scratch,
    d_close,
    n,
    dataset_sig,
    &compute_returns,
    device_launch_caps,
    compute_returns_launch_caps,
)?;
```

#### 4) Invalidate on clear

Update `clear_async()` (`gpu_backtest_kernel.rs:903-922`):

```rust
fn clear_async(&mut self, stream: &Stream) -> Result<(), String> {
    Self::drop_opt_async(&mut self.d_lr, stream)?;
    Self::drop_opt_async(&mut self.d_ret1, stream)?;
    // existing drops...
    Self::drop_opt_async(&mut self.d_valid_pair_index, stream)?;
    self.returns_key = None;
    Ok(())
}
```

### Why this patch is strong

- very low risk,
- zero product-contract change,
- removes one full-series kernel from pass 2,
- works immediately for the current benchmark.

### Tests to add

Near the existing tests in `gpu_backtest_kernel.rs:8628-8795`, add:

1. a unit test for cache hit/miss key equality,
2. an integration-style test that runs:
   - search pass,
   - finalist recompute,
   - then checks replay profile/diagnostic state to confirm no second returns launch when candles are unchanged.

---

## Patch set D — add a 9-row product-reduced search mode

### Why

If you want to eliminate replay without paying for all 10 rows, the best candidate is 9.

This preserves the current GPU product contract for the current metric set because:

- `pnl` is reconstructed from `log_eq`,
- `max_dd` is reconstructed from `max_log_dd`,
- `calmar` is already derived on CPU,
- `martin_ratio` can be derived from `pnl` and `ulcer_index`,
- `sharpe` and `sortino` are already derived from `mean/std` and `mean/downside`.

### Important caveat

This is an **A/B patch**, not an unconditional default.  
It widens pass 1 for **all pairs**, so it must be benchmarked against the actual replay tax.

### Where to patch

#### 1) Add a new constant

Near metric constants (`gpu_backtest_kernel.rs:136-145`):

```rust
const PRODUCT_REDUCED_METRICS_COUNT: usize = 9;
```

#### 2) Add an env gate

```rust
#[inline]
fn kernel_product_reduced_rows_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", false)
}
```

#### 3) Widen `search_metrics_count_for_optimization()`

Current function is at `gpu_backtest_kernel.rs:3350-3377`.

Patch:

```rust
#[inline]
fn search_metrics_count_for_optimization(
    objective: ObjectiveKind,
    constraints: Option<&DoubleMaConstraints>,
    include_all: bool,
    export_enabled: bool,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
) -> usize {
    if include_all || export_enabled || fast_precomputed_enabled || slow_precomputed_enabled {
        return METRICS_COUNT;
    }

    let mut metrics_count = match objective {
        ObjectiveKind::Pnl => 1,
        ObjectiveKind::Sharpe | ObjectiveKind::Volatility => 5,
        ObjectiveKind::MaxDrawdown | ObjectiveKind::Calmar => 3,
        ObjectiveKind::Sortino | ObjectiveKind::Omega => METRICS_COUNT,
    };

    if constraints.and_then(|c| c.min_trades).is_some() {
        metrics_count = metrics_count.max(2);
    }
    if constraints.and_then(|c| c.max_drawdown).is_some() {
        metrics_count = metrics_count.max(3);
    }

    if kernel_product_reduced_rows_enabled() {
        metrics_count = metrics_count.max(PRODUCT_REDUCED_METRICS_COUNT);
    }

    metrics_count.min(METRICS_COUNT)
}
```

#### 4) Derive `martin_ratio` in decode when metric 9 is absent

Current decode is at `gpu_backtest_kernel.rs:685-742`.

Add a helper matching `finalize_risk_metrics()` logic from `double_ma.rs:500-557`:

```rust
#[inline]
fn martin_ratio_from_components(pnl: f64, ulcer_index: f64) -> f64 {
    const EPS_DENOM: f64 = 1e-12;
    const CLAMP_MAX: f64 = 1e6;
    let raw = if ulcer_index > EPS_DENOM {
        pnl / ulcer_index
    } else if pnl > 0.0 {
        CLAMP_MAX
    } else if pnl < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    };
    raw.clamp(-CLAMP_MAX, CLAMP_MAX)
}
```

Then inside `decode_tm_flex_metrics_row()`:

```rust
let ulcer_index = metric(METRIC_IDX_ULCER_INDEX);
let martin_ratio = if metrics_count > METRIC_IDX_MARTIN_RATIO {
    metric(METRIC_IDX_MARTIN_RATIO)
} else if metrics_count > METRIC_IDX_ULCER_INDEX {
    martin_ratio_from_components(pnl, ulcer_index)
} else {
    0.0
};

Metrics {
    pnl,
    sharpe,
    sortino,
    downside_deviation,
    calmar: calmar_ratio(pnl, max_dd),
    omega: 0.0,
    volatility: std,
    ulcer_index,
    martin_ratio,
    max_dd,
    trades: if trades.is_finite() && trades >= 0.0 {
        trades as u32
    } else {
        0
    },
    exposure: metric(METRIC_IDX_EXPOSURE),
    net_exposure: metric(METRIC_IDX_NET_EXPOSURE),
}
```

#### 5) Gate replay only when the reduced row is insufficient

Add:

```rust
#[inline]
fn can_decode_current_gpu_product_metrics_without_recompute(metrics_count: usize) -> bool {
    metrics_count >= PRODUCT_REDUCED_METRICS_COUNT
}
```

Patch the replay trigger at `gpu_backtest_kernel.rs:8205-8225`:

```rust
if !include_all
    && metrics_count < METRICS_COUNT
    && !can_decode_current_gpu_product_metrics_without_recompute(metrics_count)
    && !fast_precomputed_enabled
    && !slow_precomputed_enabled
    && kernel_finalist_recompute_enabled()
{
    recompute_optimization_result_finalists_with_full_gpu_pass(...)?;
}
```

### Why 9 is the correct number

The kernel computes optional outputs based on `M` in `double_crossover.cu:3432-3439` and writes them at `3749-3759`:

- `M > 5` turns on exposure
- `M > 7` turns on downside deviation
- `M > 8` turns on ulcer index
- `M > 9` turns on martin ratio

So `M = 9` is the sweet spot:

- enough to reconstruct current GPU product metrics,
- avoids paying for the 10th slot because `martin_ratio` is derivable on CPU.

### Tests to add

Extend the existing test section with:

```rust
#[test]
fn decode_product_reduced_rows_derives_martin_ratio() {
    let pnl = 0.15f64;
    let max_dd = 0.20f64;
    let ulcer = 0.05f64;
    let row = vec![
        pnl.ln_1p() as f32,          // log_eq
        7.0f32,                     // trades
        (-(1.0 - max_dd).ln()) as f32, // max_log_dd
        0.06f32,                    // mean
        0.03f32,                    // std
        0.40f32,                    // exposure
        0.10f32,                    // net_exposure
        0.02f32,                    // downside_deviation
        ulcer as f32,               // ulcer_index
    ];

    let metrics = decode_tm_flex_metrics_row(&row, 0, PRODUCT_REDUCED_METRICS_COUNT);
    assert!((metrics.pnl - pnl).abs() < 1e-6);
    assert!((metrics.max_dd - max_dd).abs() < 1e-6);
    assert!((metrics.martin_ratio - 3.0).abs() < 1e-6);
}
```

And extend `search_metrics_count_tracks_objective_and_constraints()` to verify the env-gated 9-row widening behavior.

### How to benchmark this patch

Run:

- baseline 5-row replay on,
- 9-row reduced rows with replay off,
- same benchmark split, same candles, same `top_k=50`.

Then compare:

- total time,
- best/top-k metric parity,
- replay branch not taken.

---

## Patch set E — small-finalist fast path inside the exact-combo executor

### Why

If returns reuse and 9-row A/B are not enough, the next step is a more compact replay path for tiny finalist sets.

### Important nuance

This should be framed as a **small-finalist fast path**, not a rewrite of the entire exact-combo executor.

The current exact-combo executor already does a lot right:

- exact finalists only,
- direct dense output copy,
- low-pair M7 specialization.

What it still does generically is:

- nested tile loops,
- repeated period upload / MA-prep scaffolding,
- exact pair-map build/upload,
- general launch orchestration.

### Practical trigger

Add a fast path when all of the following hold:

- one evaluation range,
- no precomputed host matrices,
- `finalists.len() <= 128`,
- unique finalist fast periods fit in one tile,
- unique finalist slow periods fit in one tile.

### Implementation strategy

Do **not** start with a brand-new CUDA kernel.  
First add a simplified Rust-side fast path that bypasses the generic outer tile machinery when it is unnecessary.

Concretely:

1. Detect the small-finalist case in `recompute_optimization_result_finalists_with_full_gpu_pass()`.
2. Call a new helper, for example:

```rust
fn eval_double_ma_small_finalists_gpu_kernel_impl(
    rt: &mut CudaKernelRuntime,
    candles: &Candles,
    finalists: &[DoubleMaParams],
    finalist_fast_periods: &[u16],
    finalist_slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<RegimeFilterInput<'_>>,
    cfg: KernelConfig,
    strategy: &StrategyConfig,
) -> Result<Vec<Metrics>, String>
```

3. Inside this helper:
   - reuse `ensure_resident_market_series()`,
   - reuse the new returns-cache helper,
   - compute fast/slow finalist MA tiles exactly once,
   - build pair-index map once,
   - run the existing M7-specialized full-metric kernel selection logic,
   - copy one dense output block back,
   - decode directly.

### What this fast path should **not** try to do first

- It should not try to keep top-k entirely on device yet.
- It should not introduce a brand-new segmented kernel first.
- It should not attempt a broad search-path rewrite.

### Why this is a second-wave patch

It is more code than the returns cache and less certain than the 9-row A/B.  
Do it only if the replay profile still shows meaningful overhead after the lower-risk patches.

---

## Patch set F — longer-term device-side finalist handoff

### Why

Today the flow is:

1. search pass produces top-k rows on device,
2. search finalization packs/decodes them on host (`gpu_backtest_kernel.rs:8124-8149`),
3. `OptimizationResult` is built,
4. finalists are deduplicated on host (`5530-5611`),
5. pass 2 rebuilds exact-combo structures.

That is functionally fine, but it means the replay handoff is **host-mediated**.

### Longer-term direction

Keep the finalist identity data on device and hand it directly to a compact pass-2 executor.

That likely means:

- retain `global_fast_rows`, `global_slow_rows`, `global_metrics_rows` after search finalization,
- deduplicate finalists on device,
- build the exact compact pair list on device,
- launch replay from that compact pair list.

### Why this is not the first patch

This is more invasive because it changes the contract between:

- search finalization,
- `OptimizationResult` construction,
- replay.

Do it only after the benchmark split and replay instrumentation show that the current host-mediated handoff is still worth attacking.

---

## Optional ALMA/ALMA-specific accelerator

This benchmark is ALMA/ALMA. That raises a special opportunity:

- same MA type on both sides,
- same parameter family,
- same source.

In the current exact-combo executor, fast and slow MA tiles are still prepared separately (`gpu_backtest_kernel.rs:4939-5024` and `5056-5142`).

If you later decide to add a more invasive optimization, the strongest benchmark-specific one is:

- compute a shared MA cache for the union of periods,
- add row-remap support in the backtest kernel,
- let fast and slow read from the same underlying MA matrix.

This is **not** the first thing to implement, but for ALMA/ALMA it can reduce duplicated MA work in both pass 1 and pass 2.

---

## What not to optimize first

Do not start with these:

1. `stream.rs`
2. tiny CPU decode cleanups by themselves
3. removing `pack_metric_rows_for_host_copy()` from pass 2  
   (it is not on the pass-2 path)
4. M7 forcing toggles  
   (the code already has a recompute-only low-pair M7 path)

Those are either already handled or they are not the main leverage point.

---

## Validation checklist

For every optimization patch, validate two things:

### 1) Exactness

For the same input candles and request:

- `best_params` unchanged
- `best_metrics` unchanged within floating-point tolerance
- `top_k` identities unchanged
- finalist replay branch skipped only when intended

### 2) Timing

After benchmark split, record all four timings:

- `search_only`
- `search_plus_heatmap`
- `search_plus_finalist_recompute`
- `end_to_end_product`

And for replay-profile-enabled runs, record:

- returns
- fast MA
- slow MA
- pair-map
- backtest
- D2H/decode

This will tell you whether the next patch should target:

- state reuse,
- widened no-replay rows,
- or a deeper finalist executor change.

---

## Recommended implementation order

### Phase 1 — low risk / highest certainty

- add 4-way benchmark split
- add replay timing profile
- add `d_lr` / `d_ret1` reuse

### Phase 2 — strongest A/B

- add `PRODUCT_REDUCED_METRICS_COUNT = 9`
- add env-gated 9-row mode
- derive `martin_ratio` in decode
- gate replay off when 9 rows are present

### Phase 3 — if replay is still too expensive

- add small-finalist one-tile fast path
- keep existing low-pair M7 selection
- avoid generic outer orchestration when only one tiny finalist tile is needed

### Phase 4 — more invasive / longer-term

- device-side finalist handoff
- optional shared-MA cache / row-remap for ALMA/ALMA-like identical MA pairs

---

## My recommendation

If you want the shortest path to a real speedup on the current codebase, do this:

1. **Split the benchmark** so the replay tax stops being conflated with heatmap.
2. **Add replay timing instrumentation** inside `eval_double_ma_batch_gpu_kernel_impl()`.
3. **Cache `d_lr` and `d_ret1`** so pass 2 stops regenerating returns.
4. **A/B a 9-row product-reduced mode** and compare it directly against current replay.
5. **Only then** decide whether you need a dedicated small-finalist fast path.

If I had to bet on the highest ROI combination, it would be:

- returns reuse,
- replay profiling,
- 9-row A/B.

That combination gives you:

- one immediate pass-2 reduction,
- one decisive measurement framework,
- and one mathematically justified route to removing replay entirely when it wins.

---

## File/line references used

- `double_ma_opt.rs:602-645` — bundled benchmark split
- `double_ma_opt.rs:242-250` — `VECTORBT_KERNEL_FINALIST_RECOMPUTE` helper
- `gpu_backtest_kernel.rs:648-742` — reduced/full row decode
- `gpu_backtest_kernel.rs:770-790` — `KernelScratch`
- `gpu_backtest_kernel.rs:867-922` — scratch allocation / clear
- `gpu_backtest_kernel.rs:2772-2828` — `CandleDatasetSignature`
- `gpu_backtest_kernel.rs:2899-2968` — resident market series
- `gpu_backtest_kernel.rs:3350-3377` — search metric count
- `gpu_backtest_kernel.rs:3427-3479` — packed top-k row copy (search path)
- `gpu_backtest_kernel.rs:4365-5528` — exact-combo executor
- `gpu_backtest_kernel.rs:4797-4817` — returns generation in exact-combo path
- `gpu_backtest_kernel.rs:4939-5024` — fast MA prep in exact-combo path
- `gpu_backtest_kernel.rs:5056-5142` — slow MA prep in exact-combo path
- `gpu_backtest_kernel.rs:5158-5200` — exact pair-map build/upload
- `gpu_backtest_kernel.rs:5262-5281` — low-pair M7 recompute path
- `gpu_backtest_kernel.rs:5486-5505` — exact-combo sync / D2H / decode
- `gpu_backtest_kernel.rs:5530-5611` — finalist recompute wrapper
- `gpu_backtest_kernel.rs:6395-6416` — returns generation in search path
- `gpu_backtest_kernel.rs:6478-6555` — global top-k buffers
- `gpu_backtest_kernel.rs:6602-6608` — heatmap disables graph replay
- `gpu_backtest_kernel.rs:8124-8149` — packed search top-k finalization
- `gpu_backtest_kernel.rs:8205-8225` — finalist replay trigger
- `gpu_backtest_kernel.rs:8628-8795` — existing relevant tests
- `double_crossover.cu:3432-3439` — optional metric enable flags
- `double_crossover.cu:3678-3759` — metric accumulation and row writes
- `double_crossover.cu:4404-4442` — objective score for current kernel route
- `double_ma.rs:487-569` — CPU risk finalizer
- `stream.rs:1-220` — host aggregator behavior
