# Strategy And Indicator GPU Coverage Inventory

Date: 2026-03-10

## Scope

This note inventories the currently exposed strategy kinds in the app, the indicator stack each strategy uses, and the current overlap with `gpu_speedups.csv`.

For the GPU coverage section, this note applies a strict filter:

- only indicators in `gpu_speedups.csv` with `speedup >= 5x`
- coverage is counted only if the indicator is:
  - directly referenced by an exposed strategy, or
  - reachable through a valid fast/slow moving-average slot in the app

The exposed strategy list comes from `web/src/stores/optimizeStore.ts`.

## Current Exposed Strategies

The app currently exposes 16 strategy kinds:

- `DoubleMa`
- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`
- `Rocb`
- `Asr`
- `Vcc`
- `Vemr`
- `CompositeCpu`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`
- `CompositeMo`

## Current Execution Support Matrix

This section is about the **current product route**, not theoretical backend potential.

Definitions used here:

- `CPU + GPU kernel`: the exposed strategy can run on CPU or through the app's current unified GPU kernel route
- `CPU + GPU hybrid`: the exposed strategy has a distinct product-level hybrid route today or a strategy-specific GPU route that resolves to hybrid in some active configs
- `CPU only`: the exposed strategy currently executes through the CPU-centered optimization route

### CPU + GPU kernel

These strategies currently support the app's CPU route and a real product-level GPU kernel route:

- `DoubleMa`
- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`
- `CompositeTrend`
- `BreakoutMomentum`
- `Asr`
- `Vcc`
- `Vemr`
- `Rocb`

### CPU + GPU hybrid

These strategies currently support the app's CPU route and a distinct hybrid-capable GPU product route:

- `RegimeHybrid`

Important nuance:

- `CompositeTrend` and `BreakoutMomentum` also resolve to `GpuHybridKernel` when ADX is enabled
- this section classifies each strategy by its strongest default GPU route, not every conditional route it can take

### CPU only

These strategies currently execute through CPU-centered optimization routes in the product path:

- `CompositeCpu`
- `CompositeMo`

### Short Ratio

On the current exposed optimization surface:

- `13 / 16` strategy kinds are `CPU + GPU kernel`
- `1 / 16` strategy kinds are exposed as `CPU + GPU hybrid`
- `2 / 16` strategy kinds are currently `CPU only`

## Strategy Inventory

| Strategy | Current optimization route | Indicator stack | Notes |
| --- | --- | --- | --- |
| `DoubleMa` | `DoubleMa` route | `fast_ma`, `slow_ma` | Plain MA crossover. |
| `Pit` | `DoubleMa` route | `fast_ma`, `slow_ma`, `efi`, `bop`, `vosc`, `acosc`, `cfo` | Pressure Impulse Trend. |
| `Set` | `DoubleMa` route | `fast_ma`, `slow_ma`, `linearreg_angle`, `er`, `chop` | Slope + Efficiency Trend. |
| `Mts` | `DoubleMa` route | `fast_ma`, `slow_ma`, `roc`, `mom`, `ppo` | Momentum Triple Stack. |
| `Orc` | `DoubleMa` route | `fast_ma`, `slow_ma`, `rsi`, `stochf`, `willr` | Oscillator Reversion Confluence. |
| `Marrt` | `DoubleMa` route | `fast_ma`, `slow_ma` | MA-only logic with MA whitelist: `sma`, `ema`, `alma`, `wma`, `zlema`, `vwma`. |
| `Vfct` | `DoubleMa` route | `fast_ma`, `slow_ma`, `obv`, `ema(obv)`, `mfi` | Volume Flow Confirmation Trend. Fast/slow MA whitelist is `vwma` only. |
| `CompositeCpu` | `CompositeCpu` route | `fast_ma`, `slow_ma`, `halftrend`, `macz_hist`, `ott_line` | Base composite stack. |
| `CompositeTrend` | `DoubleMa` route | `fast_ma`, `slow_ma`, `linearreg_angle`, `bollinger_bands_width`, optional `roc`, optional `adx` | Trend-quality double-MA overlay. ADX off resolves to `GpuKernel`; ADX on resolves to `GpuHybridKernel`. |
| `RegimeHybrid` | `DoubleMa` route | `fast_ma`, `slow_ma`, `linearreg_angle`, `bollinger_bands_width`, optional `roc`, `adx` | Regime-hybrid double-MA overlay. Current product route targets `GpuHybridKernel`. |
| `BreakoutMomentum` | `DoubleMa` route | `fast_ma`, `slow_ma`, `bollinger_bands`, `macd`, `cmo`, `adosc`, optional `adx` | Breakout-momentum double-MA overlay. ADX off resolves to `GpuKernel`; ADX on resolves to `GpuHybridKernel`. |
| `CompositeMo` | `CompositeMo` route | `fast_ma`, `slow_ma`, `halftrend`, `macz`, `ott` | Multi-objective optimizer over the composite parameter space. |
| `Asr` | `CompositeCpu` route | `ttm_squeeze`, `squeeze_momentum`, `linearreg_angle`, `er`, `zscore`, optional `kaufmanstop` | Adaptive Squeeze Regime. |
| `Vcc` | `CompositeCpu` route | `ttm_squeeze`, `squeeze_momentum`, `ttm_trend` | Volatility Compression Confirmation. |
| `Vemr` | `CompositeCpu` route | `zscore`, `bollinger_bands`, `atr` | Volatility Expansion Mean Reversion. |
| `Rocb` | `CompositeCpu` route | `donchian`, `atr` | Risk Overlay Channel Breakout. |

## Important Coverage Notes

### 1. Moving-average coverage

Most MA-based strategies can use any moving average that passes the app's MA validation rules:

- it must be present in `vector-ta`
- it must be `period_based`
- it must be `single_output`

That constraint comes from `crates/app_core/src/double_ma_ma_kind.rs`.

This means some indicators in `gpu_speedups.csv` count as covered even when they are not hardcoded into a named strategy, because they are selectable as fast/slow MA types.

### 2. Strategy-specific MA restrictions

Two exposed strategies narrow that general MA surface:

- `Marrt`: `sma`, `ema`, `alma`, `wma`, `zlema`, `vwma`
- `Vfct`: `vwma` only

Those restrictions come from `web/src/services/optimizeRequestBuilder.ts`.

### 3. Double-MA overlay caveat

`CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` now use the `DoubleMa` route in the product path. Their shared base is:

- `fast_ma`
- `slow_ma`

They then add strategy-specific trend or breakout overlays on top of that base.

### 4. ADX caveat

`CompositeTrend` and `BreakoutMomentum` optionally depend on `adx`, while `RegimeHybrid` currently carries ADX-bearing logic by default. There is no exact `adx` row in `gpu_speedups.csv`. Related rows such as `di` and `adxr` are noted separately but are not treated here as substitutes for direct `adx` coverage.

## GPU Coverage For Indicators With At Least 5x CUDA Speedup

Legend:

- `Direct strategy use`: hardwired into at least one exposed strategy
- `Configurable MA slot`: not hardwired, but usable as a valid fast/slow MA in MA-based strategies
- `Direct + MA slot`: both of the above
- `Unused by exposed strategies`: no current exposed strategy uses it directly or through valid fast/slow MA slots

| Indicator | CUDA speedup | Coverage status | Used by / note |
| --- | ---: | --- | --- |
| `ttm_squeeze` | 1744.31x | Direct strategy use | ASR, VCC |
| `linearreg_angle` | 778.34x | Direct strategy use | SET, CompositeTrend, RegimeHybrid, ASR |
| `zscore` | 510.23x | Direct strategy use | ASR, VEMR |
| `kurtosis` | 507.44x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `minmax` | 400.14x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `mass` | 396.28x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `buff_averages` | 313.50x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `kaufmanstop` | 301.96x | Direct strategy use | ASR (optional) |
| `bollinger_bands_width` | 273.11x | Direct strategy use | CompositeTrend, RegimeHybrid |
| `correl_hl` | 232.89x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `kdj` | 157.45x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `acosc` | 124.67x | Direct strategy use | PIT |
| `srsi` | 122.77x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `squeeze_momentum` | 116.05x | Direct strategy use | ASR, VCC |
| `coppock` | 110.63x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `bollinger_bands` | 105.06x | Direct strategy use | BreakoutMomentum, VEMR |
| `dpo` | 104.10x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `ttm_trend` | 103.95x | Direct strategy use | VCC |
| `ao` | 103.25x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `rocr` | 102.88x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `vosc` | 102.11x | Direct strategy use | PIT |
| `willr` | 96.29x | Direct strategy use | ORC |
| `keltner` | 94.52x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `er` | 92.82x | Direct strategy use | SET, ASR |
| `var` | 92.47x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `deviation` | 87.07x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `bop` | 85.23x | Direct strategy use | PIT |
| `medprice` | 81.28x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `roc` | 80.54x | Direct strategy use | MTS; optional in CompositeTrend/RegimeHybrid |
| `mom` | 80.02x | Direct strategy use | MTS |
| `jsa` | 75.54x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `wclprice` | 72.71x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `ehma` | 54.55x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `vwma` | 51.70x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `stochf` | 50.44x | Direct strategy use | ORC |
| `vwap` | 49.35x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `macd` | 43.95x | Direct strategy use | BreakoutMomentum |
| `alma` | 42.92x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `uma` | 29.46x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `correlation_cycle` | 28.58x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `ppo` | 28.45x | Direct strategy use | MTS |
| `chop` | 27.23x | Direct strategy use | SET |
| `nma` | 23.25x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `srwma` | 22.49x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `ift_rsi` | 21.85x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `sqwma` | 21.76x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `cwma` | 20.36x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `obv` | 19.69x | Direct strategy use | VFCT |
| `lpc` | 18.59x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `mama` | 17.90x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `highpass` | 17.61x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `wilders` | 16.91x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `smma` | 16.46x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `sama` | 16.34x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `ema` | 15.57x | Direct + MA slot | VFCT OBV smoothing; also selectable as MA. |
| `pwma` | 14.78x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `mfi` | 14.51x | Direct strategy use | VFCT |
| `fwma` | 14.45x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `prb` | 13.84x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `qqe` | 13.51x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `efi` | 12.35x | Direct strategy use | PIT |
| `swma` | 12.03x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `vpwma` | 11.59x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `trima` | 11.54x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `apo` | 10.83x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `fvg_trailing_stop` | 10.15x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `zlema` | 10.01x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `stddev` | 9.92x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `cora_wave` | 9.63x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `devstop` | 9.63x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `dti` | 9.22x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `cmo` | 8.84x | Direct strategy use | BreakoutMomentum |
| `rsi` | 8.15x | Direct strategy use | ORC |
| `wma` | 8.03x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `highpass_2_pole` | 7.91x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `emd` | 7.33x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `decycler` | 6.48x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `tema` | 6.30x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `tilson` | 5.91x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `otto` | 5.74x | Unused by exposed strategies | No current exposed strategy references it directly or through valid fast/slow MA slots. |
| `atr` | 5.39x | Direct strategy use | VEMR, ROCB |
| `sinwma` | 5.16x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |
| `cfo` | 5.11x | Direct strategy use | PIT |
| `sma` | 34.41x | Configurable MA slot | Selectable as fast/slow MA in MA-based strategies, subject to support/whitelists. |

## Indicators With At Least 5x Speedup That Are Not Used By Any Existing Strategy

These indicators currently have at least 5x measured CUDA speedup in `gpu_speedups.csv`, but no exposed strategy uses them directly and they are not reachable through the valid fast/slow MA slots used by the MA-based optimizers:

- `kurtosis`
- `minmax`
- `mass`
- `buff_averages`
- `correl_hl`
- `kdj`
- `srsi`
- `coppock`
- `dpo`
- `ao`
- `rocr`
- `keltner`
- `var`
- `deviation`
- `medprice`
- `wclprice`
- `vwap`
- `uma`
- `correlation_cycle`
- `ift_rsi`
- `lpc`
- `mama`
- `prb`
- `qqe`
- `apo`
- `fvg_trailing_stop`
- `stddev`
- `devstop`
- `dti`
- `emd`
- `decycler`
- `otto`

## Practical Findings

1. The current strategy surface already covers many of the best CUDA winners.
   - This is especially true for `PIT`, `SET`, `MTS`, `ORC`, `VFCT`, `ASR`, `VEMR`, and the MA-based optimizers.

2. `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` are now `DoubleMa`-family GPU-capable strategies, with ADX determining whether they take the kernel or hybrid route.
   - They use several high-speedup indicators directly:
     - `linearreg_angle`
     - `bollinger_bands_width`
     - `roc`
     - `bollinger_bands`
     - `macd`
     - `cmo`
   - But they also depend on:
     - `halftrend`
     - `macz`
     - `ott`
     - optional `adx`
   - Those are the main blockers to treating the full strategy as cleanly GPU-ready from the current `gpu_speedups.csv` perspective.

3. The unused `>=5x` list is large.
   - There is still substantial room to design additional GPU-friendly strategy variants or optional filters using indicators that already show strong measured CUDA speedups.

4. The strongest “reachable but not hardcoded” GPU opportunity remains the MA surface.
   - A large block of `>=5x` indicators are usable today as fast/slow MA choices even though no strategy references them by name.
