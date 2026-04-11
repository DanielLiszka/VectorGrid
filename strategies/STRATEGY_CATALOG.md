# Strategy Catalog

Last updated: 2026-04-11

## Purpose

This document is the current human-readable catalog of the non-options strategy surfaces exposed by the application.

It is intended to answer:

- what each strategy essentially is
- which indicators or signal ingredients it uses
- what extra candle fields it explicitly requires
- which optimize/validation route family it uses today
- whether it currently participates in live execution

This is a product-and-implementation catalog, not a profitability claim.

## Scope

In scope:

- the shipped non-options strategy kinds listed in `REPO_MAP.md`
- special strategy-like workflows that appear in the product surface (`CompositeMo`, `MlSignal`)
- current optimize, validation, and live-execution posture

Out of scope:

- options-expression workflow details
- speculative or research-only future strategy families
- parameter-by-parameter optimization ranges

## How To Read This

- `Optimize route` describes the current product-facing optimization route, not a promise that every downstream validation step uses the same backend.
- `Validation posture` describes the current fixed-candidate validation behavior or the most important caveat.
- `Explicit extra data requirements` only lists requirements that are called out explicitly by the current code/docs. If a cell is blank, do not assume the strategy is close-only; it means there is no extra special-case note beyond the underlying indicators and selected MA source.
- `Live execution` reflects the current shipped live-runner support. Today that support is limited to `DoubleMa` and `CompositeCpu`.

## Source Of Truth

This catalog is derived from the current shipped repo surfaces:

- `REPO_MAP.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_cpu.rs`
- strategy-local owners such as `crates/app_core/src/asr.rs`
- `crates/app_core/src/strategy_presets.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

When this document disagrees with code, trust the code-facing routing docs and strategy-local owners.

## Double-MA Family

These strategies either are the base Double-MA engine or reuse the Double-MA execution family with additional regime/profile logic.

| Kind | What it is | Main indicators / signal ingredients | Explicit extra data requirements | Optimize route | Validation posture | Live execution |
| --- | --- | --- | --- | --- | --- | --- |
| `DoubleMa` | Core exact-grid moving-average crossover family and main benchmark/reference surface | Fast/slow MA pair from the supported MA catalog, optional `halftrend_filter` overlay |  | `Auto`, `CpuOnly`, `GpuOnly`, `GpuHybridKernel`, `GpuKernel` | Fixed-candidate validation can use CPU or GPU main route; `GpuOnly` validation falls back to CPU | Yes |
| `DynamicDoubleMa` | Constrained builder/preview surface that compiles supported static profiles into a runnable Double-MA plan | `strategy_ir` over supported indicator/profile kinds, then normal Double-MA execution | Depends on the selected compiled profile | Previewed against strategy-plan backend, then routed back through normal Double-MA execution | No separate validator; resolves into normal runtime flow after preview | No |
| `Pit` | Pressure-impulse trend profile layered onto the MA core | EFI, VOSC, CFO | `OHLCV` | Double-MA family backends | Double-like GPU-main validation path | No |
| `Set` | Slope/efficiency trend profile layered onto the MA core | Linear-reg angle, efficiency ratio, chop | `high/low` | Double-MA family backends | Double-like GPU-main validation path | No |
| `Mts` | Momentum-stack profile layered onto the MA core | ROC, momentum, PPO |  | Double-MA family backends | Double-like GPU-main validation path | No |
| `Orc` | Oscillator-reversion profile layered onto the MA core | RSI, stochastic fast K/D, Williams %R | `high/low` | Double-MA family backends | Double-like GPU-main validation path | No |
| `Marrt` | Thin Double-MA variant used mostly as a preset shape rather than a heavily differentiated signal family | Same MA-crossover core, generally with `long_only=false` posture |  | Double-MA family backends | Double-like GPU-main validation path | No |
| `Vfct` | Volume-flow confirmation profile layered onto the MA core | OBV smoothing plus MFI confirmation | `high/low/volume` | Double-MA family backends | Double-like GPU-main validation path | No |
| `CompositeTrend` | Trend-quality regime profile that reuses the Double-MA family instead of owning a separate composite stack | Linear-reg angle, Bollinger Band width, optional ROC confirmation, optional ADX confirmation |  | Routed through Double-MA; `GpuKernel` without ADX, `GpuHybridKernel` with ADX, CPU fallback | Treated as double-like fixed-candidate validation | No |
| `RegimeHybrid` | Trend-quality regime plus hybrid enter/exit behavior; routed through Double-MA | Trend-quality regime plus ADX enter/exit and mean-reversion thresholds |  | Routed through Double-MA; current target is `GpuHybridKernel`, with CPU fallback | Treated as double-like fixed-candidate validation | No |
| `BreakoutMomentum` | Breakout-and-momentum regime profile that reuses the Double-MA family | Bollinger breakout, MACD, CMO, ADOSC, optional ADX | `high/low/volume` | Routed through Double-MA; `GpuKernel` without ADX, `GpuHybridKernel` with ADX, CPU fallback | Treated as double-like fixed-candidate validation | No |

### Double-MA Family Notes

- `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` are routed Double-MA-family members. They are not `CompositeCpu` strategies.
- `Marrt` keeps a narrower MA whitelist in the current product path: `sma`, `ema`, `alma`, `wma`, `zlema`, `vwma`.
- `Vfct` currently keeps `vwma`-only trend MAs in the routed profile path.
- Exact-grid `DoubleMa` is still the truth anchor for optimization fidelity and any cheaper-route comparisons.

### Primary Owners

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/double_ma_strategy.rs`

## Composite / Advanced Family

These strategies use the `CompositeCpuRequest` product surface and keep their own strategy-local execution ownership instead of routing through the Double-MA family.

| Kind | What it is | Main indicators / signal ingredients | Explicit extra data requirements | Optimize route | Validation posture | Live execution |
| --- | --- | --- | --- | --- | --- | --- |
| `CompositeCpu` | Broad composite strategy family with its own request/result surface, analysis path, and route diagnostics | Fast/slow MA core plus HalfTrend, MACZ, OTT, and `CompositeSignalMode` |  | CPU and `GpuKernel` routes exist | GPU validation needs uniform HalfTrend params across finalists; otherwise it can fall back to CPU | Yes |
| `Asr` | Squeeze-and-regime strategy with Kaufman-style overlays | TTM squeeze, squeeze momentum, linear-reg angle, ER filter, z-score, Kaufman overlays |  | CPU and `GpuKernel` | GPU route exists, but `Auto` currently prefers CPU on measured workloads | No |
| `Vcc` | Squeeze/trend strategy with optional acceleration gating | TTM squeeze, squeeze momentum, TTM trend, optional momentum-acceleration gating |  | CPU and `GpuKernel` | Mixed validation route; CPU-reference signal prep with exact CPU-series metrics | No |
| `Vemr` | Mean-reversion/risk-overlay strategy | z-score, Bollinger bands, ATR stop |  | CPU and `GpuKernel` | GPU route exists, but `Auto` currently prefers CPU | No |
| `Rocb` | Channel-breakout strategy with ATR risk overlay | Donchian/channel breakout plus ATR risk overlay |  | CPU and `GpuKernel` | GPU route exists, but `Auto` currently prefers CPU | No |
| `Vcet` | Trend-state plus trailing-exit strategy | VIDYA trend-state plus Chandelier Exit |  | CPU and `GpuKernel` | Main fixed-candidate GPU route exists; walk-forward, reopt walk-forward, perturb, and bootstrap remain CPU-tail work | No |
| `Mkcs` | Compression-breakout strategy | Standard-deviation compression plus Donchian breakout |  | CPU and `GpuKernel` | GPU route exists, but `Auto` currently prefers CPU | No |
| `Kqsr` | Oscillator-stack strategy | KDJ, stochastic RSI, IFT-RSI, QQE, PRB | `OHLC` | CPU and `GpuKernel` | GPU route exists, but `Auto` currently prefers CPU | No |
| `Carm` | Multi-oscillator composite with OTTO HOTT/LOTT logic | Coppock, AO, ROCR, APO, OTTO HOTT/LOTT |  | CPU and `GpuKernel` | Mixed validation route; `Auto` currently prefers CPU | No |
| `Kvdf` | Channel/volume/deviation strategy with trailing-stop logic | Keltner, VWAP, DTI, DevStop, FVG trailing-stop logic | `high/low/close/volume` | CPU and `GpuKernel` | GPU route exists, but `Auto` currently prefers CPU | No |
| `Cdmc` | Cycle/decycler strategy | DPO, correlation cycle, MAMA/FAMA, decycler, phase threshold |  | CPU and `GpuKernel` | Mixed validation route; current measured policy is more GPU-favorable than `Carm` | No |

### Composite Family Notes

- `CompositeCpu` is not a routed Double-MA member. It keeps its own route diagnostics, drilldown path, local-stability path, and native Top-K/include-all result surface.
- `CompositeCpu` supports multiple `CompositeSignalMode` behaviors in the current product:
  - `HardGate`
  - `EntryCoreExitOverlay`
  - `TrendPullbackReversion`
- Cluster A families in current repo organization are:
  - `Asr`
  - `Vcc`
  - `Vemr`
  - `Rocb`
  - `Vcet`
- Cluster B families in current repo organization are:
  - `Mkcs`
  - `Kqsr`
  - `Carm`
  - `Kvdf`
  - `Cdmc`

### Primary Owners

- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
- `crates/app_core/src/asr.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/vcet.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/cdmc.rs`

## Special Strategy-Like Workflows

These appear in the product strategy/workflow surface, but they are not normal single-score grid strategies.

| Surface | What it is | Main ingredients | Optimize route | Validation posture | Live execution |
| --- | --- | --- | --- | --- | --- |
| `CompositeMo` | Multi-objective search surface, not a normal Top-K/grid optimizer | Explicit search spaces, objective pairs, constraints, sampler choice (`Nsga2`, `Random`, `Tpe`, `Qmc`), optional stage-0 GPU prescan | Own strategy-local Pareto/sampler/search-space flow; can do bounded exact-grid collapse for tiny discrete spaces | CPU-preferred validation; current GPU fixed-candidate limits are intentionally narrow and tied to plain `CompositeCpu` hard-gate candidates | No |
| `MlSignal` | Separate ML research workflow, not a standard optimizer strategy family | Dataset export, external score-cache import/build, validation, policy sweep | Separate staged workflow, not normal optimize-grid flow | Separate ML validation/policy-sweep workflow with artifact-oriented outputs | No |

### Special Workflow Notes

- `CompositeMo` owns a distinct sampler/search-space contract and should not be read as “CompositeCpu but with more knobs.”
- `MlSignal` intentionally does not populate the standard optimizer result surfaces like heatmaps or export-all grids.

### Primary Owners

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/ml_signal.rs`
- `web/src/services/mlSignalService.ts`

## Related Surface That Is Not A Parent Strategy Kind

The app also has an options-expression workflow, but it is not a parent directional strategy family in the same sense as the strategies above. It is better understood as a separate options-expression/validation workflow layered around saved directional artifacts rather than as one more strategy kind to compare directly against `DoubleMa`, `CompositeCpu`, or `CompositeMo`.

## Current Live-Execution Coverage

Current shipped live execution is intentionally narrower than the full research strategy catalog.

Supported today:

- `DoubleMa`
- `CompositeCpu`

Not currently live-runner supported:

- routed Double-MA profiles such as `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`
- standalone composite families such as `Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet`, `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`
- `CompositeMo`
- `MlSignal`

## Practical Reading Order

If you want the fastest path to understanding how a strategy actually works in code, use this order:

1. Start with `REPO_MAP.md` for the current product-facing matrix.
2. Open the strategy-local owner for the family you care about.
3. Then open the optimize/validation request builders if you need route or UI shaping details.
4. Only then open the larger orchestration files.

Good starting points:

- Double-MA family: `crates/app_core/src/double_ma.rs`
- Dynamic profile compilation: `crates/app_core/src/strategy_dynamic.rs`
- Composite family core: `crates/app_core/src/composite_cpu.rs`
- Strategy presets: `crates/app_core/src/strategy_presets.rs`
- Request shaping: `web/src/services/optimizeRequestBuilder.ts`
- Validation route summaries: `web/src/services/validationRequestBuilder.ts`

## Maintenance Notes

When a strategy is added, removed, renamed, rerouted, or materially changes indicator composition, update this document together with:

- `REPO_MAP.md`
- strategy-local owner files
- request-builder tests and capability-summary tests
- validation route-summary/tests when the validation posture changes
