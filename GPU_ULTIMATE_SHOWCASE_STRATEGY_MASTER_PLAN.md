# GPU Ultimate Showcase Strategy Master Plan

Last updated: 2026-03-04
Status: Draft v1 (research pass added)
Priority: Practical usefulness over raw speed

## Objective

Design one final, high-utility strategy for this app that:
- is meaningfully different from current fixed strategies,
- can run through the GPU-capable backend path,
- uses indicators that are confirmed GPU-faster in `gpu_speedups.csv`,
- remains interpretable and practical for real workflow usage.

This document includes an initial web research pass; links and defaults are preserved here so this plan remains useful after context compaction.

## ASR GPU Profiling Notes (2026-03-05)

Workload profiled:
- `asr/gpu_route/200k/small` (`200,000` candles, `975` valid fast<slow combos)

Nsight capture reports:
- `target/nsight/asr_gpu_route_200k_small_basic.ncu-rep` (baseline)
- `target/nsight/asr_gpu_route_200k_small_basic_block64.ncu-rep` (after launch tuning)

Primary hotspots (baseline):
- `asr_backtest_metrics_f32`: ~`123.20 ms`
- `ttm_squeeze_batch_f32`: ~`96.44 ms`
- `squeeze_momentum_batch_f32_opt`: ~`85.28 ms`
- `smi_precompute_shared_f32`: ~`22.92 ms`

Optimization pass kept:
- In `crates/app_core/src/asr_gpu_kernel.rs`, ASR metrics kernel launch now uses an adaptive block-size heuristic:
  - `<= 2048 combos -> 64`
  - `<= 8192 combos -> 128`
  - `> 8192 combos -> 256`

Observed impact from Nsight (same workload):
- `asr_backtest_metrics_f32`: `123.20 ms -> 118.05 ms`
- `ttm_squeeze_batch_f32`: `96.44 ms -> 93.49 ms`
- `squeeze_momentum_batch_f32_opt`: `85.28 ms -> 83.60 ms`

Criterion snapshot after pass:
- `asr/gpu_route/200k/small`: `[390.91 ms 468.58 ms 554.31 ms]`
- Variance remains high on this workstation; directional improvement is visible but not statistically strong in Criterion.

## Why Another Strategy

Current strategies already cover:
- classic MA crossover,
- trend-quality gating,
- breakout momentum gating,
- regime-hybrid behavior,
- composite MO sampling.

What is still missing is a practical regime-switching strategy that can adapt between:
- trend continuation when expansion is real,
- mean reversion when momentum quality is weak,
while keeping indicator choice inside the GPU-faster inventory.

## Proposed Strategy Concept (Initial)

Working name: `Adaptive Squeeze Regime (ASR)`

Core idea:
- Use volatility compression + release to detect transition points.
- Use trend-quality measures to decide whether to follow the move or fade extremes.
- Use robust exits and risk controls that remain interpretable.

### Candidate Signal Stack (GPU-Fast Inventory)

Primary indicators (v1 core):
- `ttm_squeeze` (compression/release state)
- `squeeze_momentum` (directional expansion quality)
- `linearreg_angle` (trend direction and slope confidence)
- `zscore` (distance from local equilibrium)
- `kaufmanstop` (adaptive exit/stop)

Secondary indicators (v1 optional):
- `bollinger_bands_width`
- `keltner`
- `er` (efficiency ratio)
- `willr`
- `roc` / `rocr`

### Strategy Behavior (Initial Rules Sketch)

Mode A: Expansion trend-follow
- Enter long when squeeze releases and momentum + trend-angle confirm upside.
- Enter short when squeeze releases and momentum + trend-angle confirm downside.

Mode B: Mean-reversion fallback
- When trend quality is weak, trade z-score extremes back toward equilibrium.

Exit policy
- Primary: `kaufmanstop` protective logic.
- Secondary: momentum fade, opposite signal, or max bars in trade.

Risk policy
- Use existing app controls: fees, spread, slippage, stop-loss/take-profit, cooldown, persistence.

## Practical Usefulness Goals

Success is measured by usefulness, not just speed:
- Stable behavior across multiple market regimes.
- Low parameter fragility (not hypersensitive to tiny changes).
- Interpretable rule flow for user trust and explainability.
- Easy to tune in UI without requiring expert quant background.

## Feasibility Notes (Current Codebase)

- Keep existing strategies unchanged and available.
- Add this as an additional fixed strategy profile first.
- Reuse existing optimization pipeline and result surfaces.
- Reuse existing GPU routing policy and capability checks.
- Avoid custom kernel sprawl unless profiling proves it necessary.

## Web Research Summary (Squeeze + Breakout + Exits)

Key takeaways for a squeeze-driven, interpretable strategy:
- Squeeze state is typically defined as **Bollinger Bands inside Keltner Channels** (volatility compression), and is considered a **setup**, not a directional signal.
- A common trigger is the **“release/firing” transition** (squeeze turns off after being on), because that corresponds to volatility expansion and often precedes directional moves.
- Direction is commonly chosen using a **momentum histogram** (the “Squeeze Momentum” style indicator): long bias when momentum is above zero and strengthening; short bias when below zero and weakening.
- False breakouts and chop are common; typical mitigations are requiring momentum acceleration, requiring close confirmation, and adding a trend-quality filter.
- Exits for breakout/squeeze systems are frequently volatility-based trailing stops (Chandelier Exit / ATR trailing stop / Kaufman-style volatility stop) and/or time-based exits when there is no follow-through.

Reference links (preserved as plain text):
```text
TTM Squeeze basics / usage:
https://toslc.thinkorswim.com/center/reference/Tech-Indicators/studies-library/T-U/TTM-Squeeze
https://www.simplertrading.com/trading-education/tutorials/squeeze-indicator
https://www.simplertrading.com/blog/trading-tips-strategies/how-to-use-the-ttm-squeeze

Squeeze Momentum (LazyBear-style) behavior / parameters:
https://www.tradingview.com/script/nqQ1DT5a-Squeeze-Momentum-Indicator-LazyBear/
https://indie-script.github.io/indicators/Squeeze%20Momentum/

Stops / exits:
https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/chandelier-exit
https://www.tradingview.com/support/solutions/43000773013-chandelier-exit/
https://www.incrediblecharts.com/indicators/atr_average_true_range_trailing_stops.php
https://thepatternsite.com/stops.html
```

Typical parameter defaults commonly cited in platform guides and scripts:
- TTM Squeeze: `length = 20`, `BB mult = 2.0`, `KC mult = 1.5`.
- Squeeze Momentum: `BB length = 20`, `BB mult = 2.0`, `KC length = 20`, `KC mult = 1.5` (KC range often based on True Range).
- Chandelier Exit: common defaults around `(length = 22, ATR mult = 3.0)`.
- Kaufman-style volatility stop: commonly cited defaults around `(period = 22, mult = 2.0)` (implementation details vary across sources/platforms).

## Candidate Strategy Shortlist (GPU-Fast Feasible)

Goal: pick a strategy that is both (1) useful and interpretable, and (2) showcases GPU-accelerated indicator compute (per `gpu_speedups.csv`).

Shortlist (high-level scoring, 1–5 where higher is better):

| Candidate | What it is | Interpretability | Usefulness | GPU showcase | Distinct from current presets | Implementation risk |
|---|---|---:|---:|---:|---:|---:|
| **Adaptive Squeeze Regime (ASR)** | Squeeze release + momentum + trend filter, with MR fallback | 4 | 4 | 5 | 4 | 3 |
| Donchian Breakout + Chandelier Exit | Price-channel breakout with ATR trailing stop | 5 | 4 | 2 | 3 | 2 |
| VWAP Deviation Fade | Intraday mean-reversion around VWAP | 4 | 3 | 3 | 5 | 4 |

Decision:
- **ASR remains the top pick** for this app’s “showcase” strategy because it leverages several of the strongest GPU wins (`ttm_squeeze`, `linearreg_angle`, `zscore`, `kaufmanstop`, `squeeze_momentum`) while still being explainable as “compression → release; follow momentum if trend-quality is real; otherwise fade extremes.”

## ASR v1: Concrete Rule Spec (Draft)

This section defines a concrete v1 so implementation can proceed without ambiguity. Treat values here as sensible defaults that should be validated via the app’s workflow and robustness checks.

### Indicators (Core)

- `ttm_squeeze` (compression state; used to detect squeeze on/off and releases)
- `squeeze_momentum` (directional bias and follow-through quality)
- `linearreg_angle` (trend direction + strength filter)
- `er` (efficiency ratio; optional trend-quality confirmation)
- `zscore` (mean-reversion fallback signal)
- `kaufmanstop` (primary stop / trailing exit)

### Derived Features

- `squeeze_on[t]`: squeeze state at bar `t` (interpretation: BB inside KC). If the underlying indicator encodes multi-level squeeze states, treat “on” as the strictest compression state by default.
- `squeeze_release[t] = squeeze_on[t-1] && !squeeze_on[t]`
- `mom[t] = squeeze_momentum[t]`
- `mom_delta[t] = mom[t] - mom[t-1]`
- `ang[t] = linearreg_angle[t]`
- `z[t] = zscore[t]`

### Regime / Mode Selection

ASR has two modes:

Mode A (Expansion trend-follow):
- Active when `squeeze_release[t]` is true and trend-quality is “good enough.”

Mode B (Mean-reversion fallback):
- Active when trend-quality is weak and `z[t]` is extreme.

Trend-quality heuristic (v1):
- `trend_ok_long[t] = ang[t] >= angle_deg_min && er[t] >= er_min` (if `er` disabled, use only `ang`)
- `trend_ok_short[t] = ang[t] <= -angle_deg_min && er[t] >= er_min`
- `trend_weak[t] = abs(ang[t]) <= angle_deg_mr` (and if `er` is enabled: `er[t] <= er_mr`)

### Entries

Mode A (Expansion trend-follow):
- Long entry at `t` if:
  - `squeeze_release[t]`
  - `trend_ok_long[t]`
  - `mom[t] > 0.0` and `mom_delta[t] > 0` (momentum is bullish and strengthening)
- Short entry at `t` if:
  - `squeeze_release[t]`
  - `trend_ok_short[t]`
  - `mom[t] < 0.0` and `mom_delta[t] < 0` (momentum is bearish and weakening)

Mode B (Mean-reversion fallback):
- Long entry at `t` if:
  - `trend_weak[t]`
  - `z[t] <= -z_entry`
- Short entry at `t` if:
  - `trend_weak[t]`
  - `z[t] >= z_entry`

Optional v1 anti-chop gates (start as OFF by default; evaluate ROI before enabling):
- Confirmation bar: require 1 additional bar after `squeeze_release` before entry.
- Cooldown: enforce `cooldown_bars` after exit before re-entry.

### Exits

Primary exit (all modes):
- Use `kaufmanstop` as the protective stop:
  - Long: exit when `close[t] <= kaufman_long_stop[t]`
  - Short: exit when `close[t] >= kaufman_short_stop[t]`

Secondary exits (Mode A):
- Momentum fade: exit long if `mom_delta[t] < 0` for `mom_fade_bars` consecutive bars (short symmetric).
- Time stop: exit if `bars_in_trade >= max_trade_bars`.

Secondary exits (Mode B):
- Mean reversion: exit when `abs(z[t]) <= z_exit` (return toward equilibrium).
- Time stop: exit after `max_mr_trade_bars` regardless (avoid getting stuck against persistent trends).

### Parameter Defaults + Suggested Optimization Bounds (v1)

Keep degrees of freedom low in v1. Prefer tuning 2–4 parameters while leaving the rest at defaults.

Squeeze:
- `length`: default `20`, bounds `10..50`
- `bb_mult`: default `2.0`, bounds `1.5..2.5`
- `kc_mult`: default `1.5`, bounds `1.0..2.0`

Trend filter:
- `linearreg_angle_period`: default `14`, bounds `7..50`
- `angle_deg_min`: default `10`, bounds `3..25`
- `er_period`: default `20`, bounds `10..50`
- `er_min`: default `0.30`, bounds `0.10..0.60`
- `er_mr`: default `0.25`, bounds `0.05..0.50`
- `angle_deg_mr`: default `6`, bounds `2..20`

Mean reversion:
- `z_period`: default `14` (or align with squeeze length), bounds `10..50`
- `z_entry`: default `2.0`, bounds `1.0..3.0`
- `z_exit`: default `0.5`, bounds `0.0..1.0`

Stops:
- `kaufman_stop_period`: default `22`, bounds `10..40`
- `kaufman_stop_mult`: default `2.0`, bounds `1.25..3.25`

Control:
- `cooldown_bars`: default `0`, bounds `0..10`
- `mom_fade_bars`: default `2`, bounds `1..10`
- `max_trade_bars`: default `80`, bounds `20..200`
- `max_mr_trade_bars`: default `30`, bounds `10..100`

### UI/UX Notes (Showcase-Ready)

Keep it user-friendly while preserving auditability:
- Present ASR as a card/preset with badges: `Squeeze`, `Regime`, `Trend/MR`, `GPU-Accelerated Indicators`.
- Progressive disclosure: show only 3–4 key knobs by default (e.g., squeeze length, angle threshold, z-entry, stop multiplier).
- Show a “Strategy spec” view: the exact entry/exit logic and defaults (auditable).
- Show live run-state hints during backtests: “Squeeze ON”, “Squeeze RELEASE”, “Mode: Trend”, “Mode: Mean-Reversion”.

## In-Place Plan Validation (Codebase Reality Check)

This section validates whether the current architecture can absorb ASR (and an alternate candidate) without deep refactors.

Validated integration points:
- Strategy profile enum currently supports five fixed profiles only (`DoubleMa`, `CompositeCpu`, `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`), so a new fixed strategy requires adding a new profile variant in IR and request routing.
- Static profile builder and program mapping are centralized in `build_static_strategy_ir(...)`, which is the correct insertion point for a new fixed strategy graph.
- Frontend strategy picker currently exposes options in one place (`web/src/stores/optimizeStore.ts`), so adding a new preset is low-risk from UI wiring perspective.
- The compiled plan route logic already supports descriptor-driven indicator routing (`Cpu`, `GpuSingle`, `GpuKernel`) and defaults to GPU only when speedup + capability checks pass.
- Dynamic indicator descriptors are pulled from VectorTA registry and include per-indicator capabilities (`supports_cpu_batch`, `supports_cuda_single`, `supports_cuda_sweep`, outputs, params), which is exactly what ASR/alternate need for low-tech-debt indicator wiring.
- Existing strategy-dynamic compute path already computes descriptor-driven indicators with cache/reuse and route-specific execution, so we do not need one-off bespoke dispatcher wiring for each ASR indicator.

Primary risk findings:
- `Medium`: Adding a new fixed strategy profile requires touching both backend and meta workflows where strategy kind allow-lists are explicit (holdout + validation paths currently enumerate known kinds).
- `Medium`: If any chosen indicator output is non-scalar or multi-output and we do not pin `output_id`, runtime ambiguity can occur. Mitigation: explicitly define `output_id` for all multi-output indicators in strategy IR.
- `Low`: Route behavior depends on embedded `gpu_speedups.csv`; updating that file changes auto-routing behavior. Mitigation: log route decisions in run diagnostics and freeze benchmark CSV per release.

Files confirmed as integration anchors:
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `web/src/stores/optimizeStore.ts`
- `src-tauri/src/commands_meta.rs`

## Alternate Candidate (Distinct from ASR): ADET

Working name: `Adaptive Donchian Efficiency Trend (ADET)`

Why this alternate exists:
- ADET is a trend-breakout continuation strategy, not a squeeze-regime-switch strategy.
- It provides a clean contrast to ASR: no mean-reversion fallback, no squeeze dependency, fewer regime transitions.
- It uses multiple GPU-faster indicators from `gpu_speedups.csv`: `donchian`, `chop`, `er`, `rocr`, `chandelier_exit` (optionally `atr`).

### ADET v1 Core Logic

Indicators:
- `donchian` (breakout structure: upper/lower channel)
- `chop` (trendability filter: avoid choppy/range phases)
- `er` (directional efficiency filter)
- `rocr` (momentum confirmation ratio around 1.0)
- `chandelier_exit` (volatility-adjusted trailing exit)

Derived states:
- `trendable[t] = chop[t] <= chop_max`
- `efficient[t] = er[t] >= er_min`
- `mom_up[t] = rocr[t] >= rocr_long_min`
- `mom_down[t] = rocr[t] <= rocr_short_max`
- `breakout_up[t] = close[t] > donchian_upper[t]`
- `breakout_down[t] = close[t] < donchian_lower[t]`

Entries:
- Long entry if all true:
  - `breakout_up[t]`
  - `trendable[t]`
  - `efficient[t]`
  - `mom_up[t]`
- Short entry if all true:
  - `breakout_down[t]`
  - `trendable[t]`
  - `efficient[t]`
  - `mom_down[t]`

Exits:
- Primary stop/trail:
  - Long exit when `close[t] <= chandelier_long[t]`
  - Short exit when `close[t] >= chandelier_short[t]`
- Secondary controls:
  - Time stop: `bars_in_trade >= max_trade_bars`
  - Cooldown after exit: `cooldown_bars`

### ADET Defaults and Bounds (v1)

Breakout:
- `donchian_period`: default `20`, bounds `10..100`

Trendability:
- `chop_period`: default `14`, bounds `10..50`
- `chop_max`: default `38.2`, bounds `30.0..50.0`

Efficiency:
- `er_period`: default `20`, bounds `10..50`
- `er_min`: default `0.30`, bounds `0.10..0.70`

Momentum:
- `rocr_period`: default `10`, bounds `5..40`
- `rocr_long_min`: default `1.001`, bounds `1.000..1.020`
- `rocr_short_max`: default `0.999`, bounds `0.980..1.000`

Exit/risk:
- `chandelier_period`: default `22`, bounds `10..55`
- `chandelier_mult`: default `3.0`, bounds `2.0..4.5`
- `max_trade_bars`: default `120`, bounds `20..300`
- `cooldown_bars`: default `0`, bounds `0..10`

### ADET vs ASR (Practical Contrast)

- ASR: better for mixed regimes (compression release + fallback reversion); more moving parts and more mode transitions.
- ADET: cleaner trend-breakout interpretation; likely easier to explain and tune, but may underperform in prolonged sideways mean-reverting conditions.
- Recommendation: keep ASR as primary showcase candidate and ADET as the benchmark alternate for A/B validation before final lock.

## Research Sources for ADET + Validation

```text
Donchian channels:
https://www.tradingview.com/support/solutions/43000502253-donchian-channels-dc/
https://vectoralpha.dev/projects/indicators/functions/donchian/

Choppiness Index:
https://www.tradingview.com/support/solutions/43000501980-choppiness-index-chop/
https://vectoralpha.dev/projects/indicators/functions/chop/

Efficiency Ratio:
https://vectoralpha.dev/projects/indicators/functions/er/

Rate of Change Ratio:
https://vectoralpha.dev/projects/indicators/functions/rocr/

Chandelier Exit:
https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/chandelier-exit
https://vectoralpha.dev/projects/indicators/functions/chandelier_exit/
```

## Initial Implementation Checklist

Phase 0: Scope Freeze
- [ ] Finalize ASR v1 indicator set (core + optional).
- [ ] Freeze entry/exit operator set and mode-switch rules.
- [ ] Freeze parameter bounds and defaults.
- [ ] Freeze first release as an additive strategy only (no replacement).
- [ ] Freeze alternate strategy (ADET) spec for A/B evaluation (no implementation yet).

Phase 1: Backend Contract
- [ ] Add new strategy kind in request normalization + UI options.
- [ ] Add ASR strategy config fields with validation.
- [ ] Add deterministic strategy profile mapping in backend.

Phase 2: Indicator Wiring
- [ ] Wire core ASR indicator computations via existing dispatcher/routing path.
- [ ] Ensure GPU-preferred routing follows `gpu_speedups.csv` policy.
- [ ] Add strict fallback + diagnostics when route changes.

Phase 3: Execution Logic
- [ ] Implement mode-switch logic (trend-follow vs mean-reversion).
- [ ] Implement entry/exit logic with current risk control hooks.
- [ ] Add parity tests for deterministic repeatability.

Phase 4: Validation and UX
- [ ] Add optimize/validate request builder wiring.
- [ ] Add UI controls for ASR params and sensible defaults.
- [ ] Add status hints explaining active mode and reason.

Phase 5: Performance + Quality Gates
- [ ] Add benchmark cases (CPU, GPU unified, GPU-kernel where applicable).
- [ ] Compare ASR against baseline strategies on same datasets.
- [ ] Compare ADET against ASR on the same benchmark matrix.
- [ ] Run workflow gates and regression checks.

## Strategy Evaluation Matrix (To Fill During Implementation)

- Profitability metrics: Sharpe, Sortino, Calmar, PnL, drawdown.
- Robustness metrics: stability under perturbation, cross-window consistency.
- Execution metrics: fill assumptions sensitivity, turnover impact.
- Practical metrics: parameter count, explainability, setup complexity.

## Indicator Universe Source

All indicators listed below are from:
- `gpu_speedups.csv`
- Interpretation: entries represent indicators measured as faster on GPU than CPU in this environment.

## Full Fast-GPU Indicator List (Sorted by Speedup)

| # | Indicator ID | Indicator Name | Speedup (x) | CPU ms | CUDA ms |
|---:|---|---|---:|---:|---:|
| 1 | `ttm_squeeze` | Ttm Squeeze | 1,744.307453 | 4,633.752750 | 2.656500 |
| 2 | `linearreg_angle` | Linear Regression Angle | 778.342892 | 1,981.427500 | 2.545700 |
| 3 | `zscore` | Z-Score | 510.231181 | 647.330300 | 1.268700 |
| 4 | `kurtosis` | Kurtosis | 507.439930 | 1,393.836000 | 2.746800 |
| 5 | `minmax` | Min/Max | 400.142897 | 2,759.625500 | 6.896600 |
| 6 | `mass` | Mass Index | 396.278927 | 467.490250 | 1.179700 |
| 7 | `buff_averages` | Buff Averages | 313.497474 | 694.929850 | 2.216700 |
| 8 | `kaufmanstop` | Kaufman Stop | 301.958321 | 729.561500 | 2.416100 |
| 9 | `bollinger_bands_width` | Bollinger Bands Width | 273.109644 | 437.521650 | 1.602000 |
| 10 | `correl_hl` | High-Low Correlation | 232.894380 | 754.554500 | 3.239900 |
| 11 | `kdj` | KDJ Indicator | 157.450095 | 4,795.615000 | 30.458000 |
| 12 | `acosc` | Acceleration Oscillator | 124.666643 | 782.632250 | 6.277800 |
| 13 | `srsi` | Stochastic RSI | 122.765155 | 1,563.045950 | 12.732000 |
| 14 | `squeeze_momentum` | Squeeze Momentum | 116.046158 | 420.609300 | 3.624500 |
| 15 | `coppock` | Coppock Curve | 110.629814 | 312.252650 | 2.822500 |
| 16 | `bollinger_bands` | Bollinger Bands | 105.064114 | 361.336500 | 3.439200 |
| 17 | `dpo` | Detrended Price Oscillator | 104.097154 | 115.933000 | 1.113700 |
| 18 | `ttm_trend` | TTM Trend | 103.948504 | 169.862250 | 1.634100 |
| 19 | `ao` | Awesome Oscillator | 103.252599 | 114.228350 | 1.106300 |
| 20 | `rocr` | Rate of Change Ratio | 102.877060 | 114.265550 | 1.110700 |
| 21 | `vosc` | Volume Oscillator | 102.112418 | 113.314150 | 1.109700 |
| 22 | `willr` | Williams %R | 96.294986 | 213.948200 | 2.221800 |
| 23 | `keltner` | Keltner Channel | 94.517368 | 358.088500 | 3.788600 |
| 24 | `er` | Efficiency Ratio | 92.818140 | 109.859550 | 1.183600 |
| 25 | `var` | Variance | 92.468183 | 107.965850 | 1.167600 |
| 26 | `deviation` | Deviation | 87.066516 | 117.348250 | 1.347800 |
| 27 | `bop` | Balance of Power | 85.232337 | 279.996750 | 3.285100 |
| 28 | `medprice` | Median Price | 81.275639 | 269.652250 | 3.317750 |
| 29 | `roc` | Rate of Change | 80.536469 | 113.508100 | 1.409400 |
| 30 | `mom` | Momentum | 80.023544 | 115.730050 | 1.446200 |
| 31 | `jsa` | JSA Moving Average | 75.542905 | 108.812000 | 1.440400 |
| 32 | `wclprice` | Weighted Close Price | 72.714709 | 251.374750 | 3.457000 |
| 33 | `ehma` | Ehma | 54.551946 | 192.759300 | 3.533500 |
| 34 | `vwma` | Volume Weighted Moving Average | 51.697722 | 319.486750 | 6.179900 |
| 35 | `stochf` | Stochastic Fast | 50.438017 | 268.088150 | 5.315200 |
| 36 | `vwap` | Volume Weighted Average Price | 49.346926 | 342.216000 | 6.934900 |
| 37 | `macd` | Moving Average Convergence Divergence | 43.952247 | 896.669800 | 20.401000 |
| 38 | `alma` | Arnaud Legoux Moving Average | 42.918276 | 158.256850 | 3.687400 |
| 39 | `sma` | Simple Moving Average | 34.409440 | 113.240550 | 3.690000 |
| 40 | `uma` | Uma | 29.460277 | 52,088.716250 | 1,768.100000 |
| 41 | `correlation_cycle` | Correlation Cycle | 28.580246 | 613.589300 | 21.469000 |
| 42 | `ppo` | Percentage Price Oscillator | 28.449055 | 276.530500 | 9.720200 |
| 43 | `chop` | Choppiness Index | 27.234018 | 6,347.705000 | 233.080000 |
| 44 | `nma` | Natural Moving Average | 23.253881 | 205.380600 | 8.832100 |
| 45 | `srwma` | Square Root Weighted Moving Average | 22.488198 | 210.174700 | 9.346000 |
| 46 | `ift_rsi` | Inverse Fisher Transform RSI | 21.849038 | 3,378.953750 | 154.650000 |
| 47 | `sqwma` | Square Weighted Moving Average | 21.756438 | 112.202300 | 5.157200 |
| 48 | `cwma` | Centered Weighted Moving Average | 20.362835 | 163.033000 | 8.006400 |
| 49 | `obv` | On Balance Volume | 19.693123 | 251.653500 | 12.778750 |
| 50 | `lpc` | Lpc | 18.589031 | 13,546.012500 | 728.710000 |
| 51 | `mama` | MESA Adaptive Moving Average | 17.902668 | 255.327850 | 14.262000 |
| 52 | `highpass` | Highpass Filter | 17.607106 | 108.841850 | 6.181700 |
| 53 | `wilders` | Wilder's Smoothing | 16.909153 | 110.624750 | 6.542300 |
| 54 | `smma` | Smoothed Moving Average | 16.459656 | 109.522550 | 6.654000 |
| 55 | `sama` | Sama | 16.338968 | 4,875.874750 | 298.420000 |
| 56 | `ema` | Exponential Moving Average | 15.573925 | 107.000650 | 6.870500 |
| 57 | `pwma` | Pascal's Weighted Moving Average | 14.784935 | 186.467600 | 12.612000 |
| 58 | `mfi` | Money Flow Index | 14.505407 | 21.872703 | 1.507900 |
| 59 | `fwma` | Forward Weighted Moving Average | 14.451643 | 137.488600 | 9.513700 |
| 60 | `prb` | Prb | 13.842037 | 467.833150 | 33.798000 |
| 61 | `qqe` | Qqe | 13.507013 | 1,926.910500 | 142.660000 |
| 62 | `efi` | Elder Force Index | 12.353137 | 794.850250 | 64.344000 |
| 63 | `swma` | Symmetrically Weighted Moving Average | 12.033966 | 109.794300 | 9.123700 |
| 64 | `vpwma` | Volume Price Weighted Moving Average | 11.585609 | 160.008850 | 13.811000 |
| 65 | `trima` | Triangular Moving Average | 11.539269 | 107.579450 | 9.322900 |
| 66 | `apo` | Absolute Price Oscillator | 10.828621 | 111.556450 | 10.302000 |
| 67 | `fvg_trailing_stop` | Fvg Trailing Stop | 10.149368 | 7,295.061500 | 718.770000 |
| 68 | `zlema` | Zero Lag Exponential Moving Average | 10.005515 | 114.022850 | 11.396000 |
| 69 | `stddev` | Standard Deviation | 9.921145 | 122.059850 | 12.303000 |
| 70 | `cora_wave` | Compound Ratio Moving Average (CoRa Wave) | 9.634688 | 115.683700 | 12.007000 |
| 71 | `devstop` | Deviation Stop | 9.627396 | 5,354.372500 | 556.160000 |
| 72 | `dti` | Directional Trend Index | 9.218469 | 553.449250 | 60.037000 |
| 73 | `cmo` | Chande Momentum Oscillator | 8.837400 | 114.532700 | 12.960000 |
| 74 | `rsi` | Relative Strength Index | 8.150236 | 122.824050 | 15.070000 |
| 75 | `wma` | Weighted Moving Average | 8.030009 | 114.339300 | 14.239000 |
| 76 | `highpass_2_pole` | Highpass Filter 2-Pole | 7.912270 | 111.135750 | 14.046000 |
| 77 | `emd` | Empirical Mode Decomposition | 7.328032 | 1,272.366250 | 173.630000 |
| 78 | `decycler` | Decycler | 6.480814 | 132.260450 | 20.408000 |
| 79 | `tema` | Triple Exponential Moving Average | 6.298090 | 110.323650 | 17.517000 |
| 80 | `tilson` | Tilson T3 | 5.913421 | 125.045200 | 21.146000 |
| 81 | `otto` | Optimized Trend Tracker Oscillator (OTTO) | 5.738783 | 3,648.603600 | 635.780000 |
| 82 | `atr` | Average True Range | 5.391050 | 113.788900 | 21.107000 |
| 83 | `sinwma` | Sine Weighted Moving Average | 5.163362 | 161.850750 | 31.346000 |
| 84 | `cfo` | Chaikin Flow Oscillator | 5.106181 | 118.754450 | 23.257000 |
| 85 | `alligator` | Alligator | 4.758171 | 989.794750 | 208.020000 |
| 86 | `ultosc` | Ultimate Oscillator | 4.535655 | 16.579635 | 3.655400 |
| 87 | `qstick` | Qstick | 4.360852 | 4.810892 | 1.103200 |
| 88 | `nadaraya_watson_envelope` | Nadaraya Watson Envelope | 4.261737 | 14,439.618750 | 3,388.200000 |
| 89 | `lrsi` | Laguerre RSI | 4.120543 | 932.602500 | 226.330000 |
| 90 | `donchian` | Donchian Channel | 4.075607 | 888.197100 | 217.930000 |
| 91 | `vidya` | Variable Index Dynamic Average | 3.880118 | 114.979550 | 29.633000 |
| 92 | `mod_god_mode` | Mod God Mode | 3.688799 | 5,264.285500 | 1,427.100000 |
| 93 | `chande` | Chande Momentum Oscillator | 3.666523 | 2,963.834000 | 808.350000 |
| 94 | `halftrend` | Halftrend | 3.634824 | 3,246.952000 | 893.290000 |
| 95 | `adosc` | Accumulation/Distribution Oscillator | 3.493097 | 111.506650 | 31.922000 |
| 96 | `epma` | Endpoint Moving Average | 3.409955 | 131.365100 | 38.524000 |
| 97 | `di` | Directional Indicator (+DI/-DI) | 3.368383 | 253.457350 | 75.246000 |
| 98 | `volume_adjusted_ma` | Volume Adjusted Ma | 3.073919 | 3,481.827800 | 1,132.700000 |
| 99 | `macz` | Macz | 2.822876 | 197.584400 | 69.994000 |
| 100 | `chandelier_exit` | Chandelier Exit | 2.631235 | 4,275.757250 | 1,625.000000 |
| 101 | `jma` | Jurik Moving Average | 2.614796 | 107.928300 | 41.276000 |
| 102 | `dec_osc` | Detrended Oscillator | 2.587047 | 550.290750 | 212.710000 |
| 103 | `msw` | Mesa Sine Wave | 2.496171 | 490.223100 | 196.390000 |
| 104 | `adxr` | Average Directional Movement Index Rating | 2.431204 | 743.559500 | 305.840000 |
| 105 | `aroonosc` | Aroon Oscillator | 2.421917 | 192.787050 | 79.601000 |
| 106 | `supersmoother` | SuperSmoother | 2.190849 | 114.366700 | 52.202000 |
| 107 | `edcf` | Ehlers Distance Coefficient Filter | 2.119290 | 107.846450 | 50.888000 |
| 108 | `mwdx` | Modified Weighted Directional Index | 2.112589 | 108.067400 | 51.154000 |
| 109 | `maaq` | Moving Average Adaptive Q | 2.108474 | 111.148200 | 52.715000 |
| 110 | `fisher` | Fisher Transform | 1.823893 | 718.887350 | 394.150000 |
| 111 | `dema` | Double Exponential Moving Average | 1.763182 | 109.259100 | 61.967000 |
| 112 | `medium_ad` | Median Absolute Deviation | 1.683522 | 6,167.076700 | 3,663.200000 |
| 113 | `ott` | Optimized Trend Tracker (OTT) | 1.657530 | 755.464450 | 455.777200 |
| 114 | `mab` | Moving Average Bands | 1.582534 | 1,861.218000 | 1,176.100000 |
| 115 | `tsf` | Time Series Forecast | 1.577150 | 383.089750 | 242.900000 |
| 116 | `aso` | Aso | 1.447015 | 308.141750 | 212.950000 |
| 117 | `stoch` | Stochastic Oscillator | 1.340774 | 6,144.631200 | 4,582.900000 |
| 118 | `hwma` | Hann Window Moving Average | 1.291440 | 124.286900 | 96.239000 |
| 119 | `ehlers_itrend` | Ehlers Instantaneous Trendline | 1.287556 | 478.455950 | 371.600000 |
| 120 | `hma` | Hull Moving Average | 1.270380 | 118.988900 | 93.664000 |
| 121 | `gaussian` | Gaussian Filter | 1.136964 | 115.504200 | 101.590000 |
| 122 | `mean_ad` | Mean Absolute Deviation | 1.028428 | 117.117400 | 113.880000 |
| 123 | `supersmoother_3_pole` | SuperSmoother 3-Pole | 1.007668 | 111.780600 | 110.930000 |

## Notes

- This is the initial creation pass requested now.
- External literature research and strategy evidence synthesis are intentionally deferred to the next pass.

