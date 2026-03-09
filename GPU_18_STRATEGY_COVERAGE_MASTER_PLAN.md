# GPU Strategy Coverage Master Plan

Last updated: 2026-03-09
Status: Re-scoped for practical GPU-only future additions
Priority order: performance -> correctness -> practical utility -> low tech debt -> UX

## Scope Reset

This document now governs future strategy additions only.

The previous 25-strategy / `>=2x` indicator-coverage target is no longer the active product direction.

New committed future strategies must:
- be practical and interpretable in real-world use,
- rely on static GPU-friendly implementations only,
- use core indicators with measured `gpu_speedups.csv` speedup `>= 5x`,
- avoid dependence on dynamic indicator generation.

Non-goals for this version:
- full `>=2x` indicator coverage,
- padding the strategy selector with low-confidence or research-only profiles,
- building strategies whose core edge depends on indicators slower than the `>=5x` cutoff,
- carrying forward strategy concepts that are mainly useful as exploratory research notes.

## Goal

Add a small committed set of future GPU-focused strategies on top of the current baseline:
- `S10` `SET`
- `S11` `MARRT`
- `S12` `MTS`
- `S13` `ORC`
- `S14` `VFCT`
- `S18` `ROCB`

Target outcome:
- current baseline remains available,
- future committed additions are reduced to 6 practical GPU-focused strategies,
- all committed future strategies use only `>=5x` core indicators,
- each committed future strategy reaches optimization, validation, and meta wiring before it is considered done.

Success criteria:
- Total strategy count in app selector after committed rollout: `15`.
- Every committed future strategy has a GPU path, parity/sanity tests, and benchmark evidence.
- Every committed future strategy is practically interpretable by a user without relying on exotic research-only logic.
- No future strategy is kept if it fails either:
  - correctness/parity gates, or
  - meaningful GPU speed improvement on medium+ search sizes.

## Current Baseline (9 Existing Strategies)

From current app strategy options:
- `S01` `DoubleMa`
- `S02` `Asr`
- `S03` `CompositeCpu`
- `S04` `CompositeTrend`
- `S05` `RegimeHybrid`
- `S06` `BreakoutMomentum`
- `S07` `CompositeMo`
- `S08` `VCC`
- `S09` `VEMR`

## Committed Future Strategies (6)

### `S10` SET (Slope + Efficiency Trend)
- Archetype: directional trend-follow with explicit anti-chop gating.
- Core indicators:
  - `linearreg_angle`
  - `er`
  - `chop`
- Why it stays:
  - strong trend/regime intuition,
  - all core indicators are comfortably above the `>=5x` threshold,
  - simpler and more robust than most cycle/filter alternatives.

### `S11` MARRT (MA Ribbon Regime Trend)
- Archetype: MA alignment / expansion trend participation.
- Core indicators:
  - fixed ribbon templates built from a small MA whitelist
- Allowed MA whitelist for v1 core:
  - `sma`
  - `ema`
  - `alma`
  - `wma`
  - `zlema`
  - `vwma`
- Why it stays:
  - practical trend family with clear user intuition,
  - covers useful fast CUDA MA variants without exploding into many one-off MA strategies.

### `S12` MTS (Momentum Triple Stack)
- Archetype: multi-horizon momentum continuation.
- Core indicators:
  - `roc`
  - `mom`
  - `ppo`
- Why it stays:
  - practical continuation logic,
  - strong measured GPU speedups,
  - no need for the more exotic optional momentum indicators in v1.

### `S13` ORC (Oscillator Reversion Confluence)
- Archetype: range-regime mean reversion using classic oscillators.
- Core indicators:
  - `rsi`
  - `stochf`
  - `willr`
- Why it stays:
  - practical and familiar,
  - useful complement to trend/breakout families,
  - keeps mean-reversion logic grounded in interpretable signals.

### `S14` VFCT (Volume Flow Confirmation Trend)
- Archetype: trend participation only when price and volume agree.
- Core indicators:
  - `vwap` or `vwma`
  - `obv`
  - `mfi`
- Why it stays:
  - practical if OHLCV is present,
  - volume/flow is kept in the role where it is most useful: confirmation rather than standalone alpha.
- Constraint:
  - must be disabled when volume data is absent.

### `S18` ROCB (Risk Overlay Channel Breakout)
- Archetype: simple breakout with explicit volatility-aware risk.
- Core indicators:
  - raw N-bar price channel breakout from OHLC
  - `atr`
- v1 exit policy:
  - close back inside channel, or
  - ATR-based stop / max-bars exit
- Why it stays:
  - practical and easy to understand,
  - avoids low-speedup ADX/DI/Donchian/chandelier dependencies in the old `TDB` concept,
  - gives the roadmap one clean breakout family without duplicating nearby ideas.

## Deferred / Removed From Committed Scope

These concepts are kept only as research notes for later reconsideration. They are not part of the committed implementation roadmap.

### Deferred despite `>=5x` core indicators
- `S15` `FRCT`
  - interesting, but more exotic and less obviously useful than the committed set.
- `S16` `VTRS`
  - better treated as a reusable regime gate than as a standalone product strategy.
- `S17` `CFR`
  - cycle-filter logic is higher-risk for overfitting and less user-transparent than the committed set.

### Dropped because core indicators fail the `>=5x` policy or are too weak for current product scope
- `S19` `TDB`
  - core depends on `donchian`, `di`, `adxr`, `chandelier_exit`, all below the new cutoff.
- `S20` `ATA`
  - `alligator` and `adxr` do not meet the new cutoff.
- `S21` `ABMT`
  - `vidya` / `macz` do not meet the new cutoff.
- `S22` `ULR`
  - `lrsi`, `ultosc`, and `qstick` do not meet the new cutoff.
- `S23` `MGMS`
  - `mod_god_mode` does not meet the new cutoff.
- `S24` `MCS`
  - `msw` / `dec_osc` do not meet the new cutoff.
- `S25` `NWER`
  - `nadaraya_watson_envelope` does not meet the new cutoff and remains expensive / niche.

## Strategy Design Rules

Rules that apply to all committed future strategies:
- Static GPU-focused implementation only.
- No dynamic indicator generation dependency.
- Core indicators must have measured `gpu_speedups.csv` speedup `>= 5x`.
- Optional modules are out of scope for v1 unless they also satisfy the same cutoff and materially improve the strategy.
- Prefer simple deterministic exits:
  - opposite signal,
  - midline reclaim,
  - close back inside breakout channel,
  - ATR-based stop,
  - max-bars exit.
- Avoid exotic overlay stacking in v1.
- Volume-dependent strategies must be explicitly gated on OHLCV datasets.
- Search spaces must be capped so the strategy remains benchmarkable and explainable.

## V1 Freeze Spec (Implementation Contract)

Purpose: keep the committed roadmap small, stable, and testable.

### Required Modules and Search Caps

| Strategy | Required modules (v1 core) | v1 search cap |
|---|---|---|
| `S10` `SET` | `linearreg_angle`, `er`, `chop` | `<= 80k` |
| `S11` `MARRT` | fixed ribbon templates + one MA type from v1 whitelist | `<= 100k` |
| `S12` `MTS` | `roc`, `mom`, `ppo` | `<= 70k` |
| `S13` `ORC` | `rsi`, `stochf`, `willr` 2-of-3 confluence | `<= 60k` |
| `S14` `VFCT` | `vwap`/`vwma`, `obv`, `mfi` | `<= 60k` |
| `S18` `ROCB` | raw channel breakout + `atr` | `<= 80k` |

### Parameter Cardinality Constraints

- `S11 MARRT`
  - optimize ribbon template ID, not arbitrary per-line periods,
  - MA type candidate set stays inside the v1 whitelist,
  - fast template count `<= 6`,
  - slow template count `<= 6`.
- `S13 ORC`
  - fixed 3-oscillator confluence in v1,
  - require 2-of-3 agreement,
  - no optional oscillator expansion in v1.
- `S14 VFCT`
  - if volume is absent, strategy is unavailable rather than silently degraded.
- `S18 ROCB`
  - use raw channel breakout + ATR risk only in v1,
  - do not add secondary risk overlays in the committed scope.

## Core Indicator Policy

Committed future strategies may use only these core future indicators:
- `linearreg_angle`
- `er`
- `chop`
- `sma`
- `ema`
- `alma`
- `wma`
- `zlema`
- `vwma`
- `roc`
- `mom`
- `ppo`
- `rsi`
- `stochf`
- `willr`
- `vwap`
- `obv`
- `mfi`
- `atr`

Notes:
- Raw OHLC breakout logic is permitted for `S18` and is not treated as a violation of the indicator cutoff policy.
- Indicators slower than `5x` are not part of the committed future roadmap even if they remain interesting research ideas.

## Architecture Constraints

- Keep one shared strategy execution pipeline for:
  - optimization,
  - validation,
  - meta-analysis.
- Strategy definitions should remain profile-driven where possible, but without dynamic indicator generation as a dependency.
- Reuse existing GPU routing and benchmark harness patterns.
- No per-strategy CUDA kernel forks unless profiling proves clear ROI.
- Future committed strategies should be GPU-first by design, not CPU-first with a GPU afterthought.

## Performance Guardrails

- Primary backend for committed future strategies: GPU path required.
- Keep data VRAM-resident whenever possible; minimize host-device transfers.
- For each committed future strategy, require benchmark rows at minimum:
  - 200k candles / small search
  - 200k candles / medium search
  - 200k candles / large search
- Acceptance gate per strategy:
  - no correctness regression vs CPU reference on golden datasets,
  - meaningful speedup over `CpuOnly` on at least medium+ search sizes,
  - if not faster in practice, strategy is deferred.

## Rollout Order

Recommended implementation order:
- Wave 1: `S10`, `S11`, `S12`
- Wave 2: `S13`, `S18`
- Wave 3: `S14`

Rationale:
- Wave 1 covers the strongest trend/momentum families first.
- `S18` stays early because breakout logic is practical and broadly useful.
- `S14` is last because it depends on volume-backed datasets.

## Per-Strategy Implementation Checklist

This checklist is required for every committed future strategy before it is considered complete:

- [ ] GPU backend path implemented and routed from optimize request.
- [ ] CPU reference path exists for parity and validation.
- [ ] Unit tests:
  - [ ] optimization smoke test (CPU reference),
  - [ ] optimization smoke test (GPU, CUDA-gated),
  - [ ] CPU-vs-GPU small-grid sanity/parity test (CUDA-gated).
- [ ] Benchmark setup:
  - [ ] dedicated benchmark target in `crates/app_core/benches`,
  - [ ] GPU benchmark group for `200k` candles with small/medium/large search sizes,
  - [ ] latest measured timings logged in this document.
- [ ] Frontend wiring:
  - [ ] strategy selector option,
  - [ ] full parameter controls in Vue UI,
  - [ ] store state + defaults + snapshot restore wiring,
  - [ ] optimize request builder payload serialization for all strategy params,
  - [ ] optimize service auto-build route and command dispatch coverage,
  - [ ] frontend tests for request builder + service route.
- [ ] Validation stage integrated.
- [ ] Meta stage integrated.

## Current Per-Strategy Status

`S08` VCC:
- [x] CPU backend
- [x] GPU backend
- [x] Unit tests (CPU smoke, GPU smoke, CPU-vs-GPU sanity)
- [x] GPU benchmark target + benchmark group
- [x] Frontend optimize wiring with parameters
- [x] Frontend tests
- [ ] Validation/meta integration remains to be fully normalized under the narrowed roadmap

`S09` VEMR:
- [x] CPU backend
- [x] GPU backend
- [x] Unit tests (CPU smoke, GPU smoke, CPU-vs-GPU sanity)
- [x] GPU benchmark target + benchmark group
- [x] Frontend optimize wiring with parameters
- [x] Frontend tests
- [x] Cross-stage wiring repaired after the earlier optimization-only rollout gap

Committed future strategies:
- [x] `S10` `SET`
- [x] `S11` `MARRT`
- [x] `S12` `MTS`
- [x] `S13` `ORC`
- [x] `S14` `VFCT`
- [x] `S18` `ROCB`

Implementation rule for the remaining roadmap strategies:
- [x] Restrict roadmap-strategy backend targets to `CpuOnly` and VRAM-resident `GpuKernel`.
- [x] Do not add `GpuOnly`/legacy sweep-specific strategy branches for `S10`, `S11`, `S12`, `S13`, `S14`, or `S18`.
- [x] Reuse existing optimizer families where practical:
  - `S11` / `S14`: existing double-MA optimize / validate / holdout stack.
  - `S10` / `S12` / `S13`: double-MA stack plus device-native regime filters unless profiling proves a generic predicate kernel is ready.
  - `S18`: prefer a reusable breakout/risk regime layer before considering a bespoke optimizer.
- [x] Treat `GpuHybridKernel` as an application-wide capability, not a roadmap-strategy design target.

Per-strategy closeout checklist for the remaining roadmap:
- [x] `S10` `SET`
  - [x] CPU path
  - [x] `GpuKernel` path with device-native regime build
  - [x] optimize wiring
  - [x] validation wiring
  - [x] meta / holdout wiring
  - [x] frontend selector / request normalization
  - [x] targeted tests
  - [x] benchmark target
- [x] `S11` `MARRT`
  - [x] CPU path
  - [x] `GpuKernel` path
  - [x] optimize wiring
  - [x] validation wiring
  - [x] meta / holdout wiring
  - [x] frontend selector / request normalization
  - [x] targeted tests polished beyond first-pass smoke
  - [x] benchmark target
- [x] `S12` `MTS`
  - [x] CPU path
  - [x] `GpuKernel` path with device-native regime build
  - [x] optimize wiring
  - [x] validation wiring
  - [x] meta / holdout wiring
  - [x] frontend selector / request normalization
  - [x] targeted tests
  - [x] benchmark target
- [x] `S13` `ORC`
  - [x] CPU path
  - [x] `GpuKernel` path with device-native regime build
  - [x] optimize wiring
  - [x] validation wiring
  - [x] meta / holdout wiring
  - [x] frontend selector / request normalization
  - [x] targeted tests
  - [x] benchmark target
- [x] `S14` `VFCT`
  - [x] CPU path
  - [x] `GpuKernel` path with device-native regime build
  - [x] optimize wiring
  - [x] validation wiring
  - [x] meta / holdout wiring
  - [x] frontend selector / request normalization
  - [x] targeted correctness tests
  - [x] benchmark target
- [x] `S18` `ROCB`
  - [x] CPU path
  - [x] `GpuKernel` path with device-native breakout/risk build
  - [x] optimize wiring
  - [x] validation wiring
  - [x] meta / holdout wiring
  - [x] frontend selector / request normalization
  - [x] full parameter controls in Vue UI + store
  - [x] targeted request / Tauri normalization tests
  - [x] benchmark target

## Near-Full `>=5x` CUDA Coverage Expansion

Excluded from this pass by product choice:
- [x] `wclprice`
- [x] `medprice`
- [x] `kurtosis`
- [x] `correl_hl`
- [x] `correlation_cycle`

Second-wave strategy set for near-full practical `>=5x` CUDA indicator coverage:
- [ ] `S26` `PIT` Pressure Impulse Trend
  - core indicators: `efi`, `bop`, `vosc`, `acosc`, `cfo`
  - backend targets: `CpuOnly`, VRAM-resident `GpuKernel`
  - [x] CPU regime path
  - [x] `GpuKernel` path with device-native regime build
  - [x] optimize wiring
  - [x] validation wiring
  - [x] meta / holdout wiring
  - [x] frontend selector / request normalization
  - [x] parameter controls
  - [x] targeted tests
  - [x] benchmark target
- [ ] `S27` `AFT` Adaptive Filter Trend
  - core indicators: `highpass`, `highpass_2_pole`, `decycler`, `mama`, `sama`, `otto`
  - backend targets: `CpuOnly`, VRAM-resident `GpuKernel`
  - [ ] CPU regime path
  - [ ] `GpuKernel` path with device-native regime build
  - [ ] optimize wiring
  - [ ] validation wiring
  - [ ] meta / holdout wiring
  - [ ] frontend selector / request normalization
  - [ ] parameter controls
  - [ ] targeted tests
  - [ ] benchmark target
- [ ] `S28` `OSR2` Oscillator Stack Reversion 2
  - core indicators: `qqe`, `srsi`, `ift_rsi`, `kdj`, `prb`
  - backend targets: `CpuOnly`, VRAM-resident `GpuKernel`
  - [ ] CPU regime path
  - [ ] `GpuKernel` path with device-native regime build
  - [ ] optimize wiring
  - [ ] validation wiring
  - [ ] meta / holdout wiring
  - [ ] frontend selector / request normalization
  - [ ] parameter controls
  - [ ] targeted tests
  - [ ] benchmark target
- [ ] `S29` `DMR` Detrended Momentum Rotation
  - core indicators: `ao`, `apo`, `coppock`, `dpo`, `rocr`, `dti`
  - backend targets: `CpuOnly`, VRAM-resident `GpuKernel`
  - [ ] CPU regime path
  - [ ] `GpuKernel` path with device-native regime build
  - [ ] optimize wiring
  - [ ] validation wiring
  - [ ] meta / holdout wiring
  - [ ] frontend selector / request normalization
  - [ ] parameter controls
  - [ ] targeted tests
  - [ ] benchmark target
- [ ] `S30` `VRE` Volatility Risk Envelope
  - core indicators: `keltner`, `stddev`, `var`, `deviation`, `devstop`, `fvg_trailing_stop`
  - backend targets: `CpuOnly`, VRAM-resident `GpuKernel`
  - [ ] CPU regime path
  - [ ] `GpuKernel` path with device-native regime build
  - [ ] optimize wiring
  - [ ] validation wiring
  - [ ] meta / holdout wiring
  - [ ] frontend selector / request normalization
  - [ ] parameter controls
  - [ ] targeted tests
  - [ ] benchmark target

Implementation rule for the near-full coverage expansion:
- [ ] Keep the same two-path model per strategy: `CpuOnly` or VRAM-resident `GpuKernel`.
- [ ] Reuse the existing double-like optimize / validate / holdout stack whenever the strategy reduces to a directional regime buffer plus MA cross execution.
- [ ] Prefer shared device-native regime builders and compose kernels over bespoke end-to-end optimizer forks.
- [ ] Only add a new custom optimizer family if the strategy state machine cannot be expressed as a reusable regime/risk layer.

- 2026-03-09: implemented `S26` (`PIT`) as the first second-wave near-full `>=5x` CUDA coverage strategy on the shared double-like stack.
  - Added `pressure_impulse_trend_enable` plus `pit_*` params to `ta_strategies::double_ma::StrategyConfig`.
  - Added CPU and VRAM-resident device-native regime builders in `crates/app_core/src/double_ma.rs` using `efi`, `bop`, `vosc`, `acosc`, and `cfo`.
  - Added the CUDA compose kernel `build_pit_regime_f32` in `crates/app_core/src/kernels/double_crossover.cu` and the Rust launcher in `crates/app_core/src/gpu_backtest_kernel.rs`.
  - Wired `Pit` through optimize / validation / meta normalization in the frontend and Tauri layers without introducing a new optimizer family.
  - Added targeted Rust and TS coverage plus benchmark registration in `crates/app_core/benches/double_ma_opt.rs`.
  - Follow-up closeout: added dedicated Vue parameter controls in `web/src/components/StrategyWorkspacePanel.vue`, store defaults/snapshot wiring in `web/src/stores/strategyStore.ts`, and Pinia-to-request bindings in `web/src/services/optimizeService.ts` so PIT no longer relies on request-builder defaults during normal UI-driven runs.

## Implementation Phases Checklist

### Phase 0: Freeze Scope
- [ ] Freeze the 6 committed future strategy IDs.
- [ ] Confirm current 9-strategy baseline in UI + backend enums/normalization.
- [ ] Freeze `>=5x` core-indicator policy.
- [ ] Freeze the static-GPU-only constraint for future additions.

### Phase 1: Strategy Specs
- [ ] Freeze concrete entry/exit/risk rules for `S10`, `S11`, `S12`, `S13`, `S14`, `S18`.
- [ ] Freeze parameter bounds/defaults.
- [ ] Freeze search-space limits.
- [ ] Freeze v1 MA whitelist for `S11`.
- [ ] Freeze volume requirement behavior for `S14`.

### Phase 2: Backend Integration
- [ ] Add static strategy profile registry entries for the 6 committed future strategies.
- [ ] Wire optimization execution for each.
- [ ] Add strategy-specific serialization compatibility tests.

### Phase 3: Frontend Integration
- [ ] Add strategy selector entries for the 6 committed future strategies.
- [ ] Add parameter controls for each.
- [ ] Add strategy descriptions and intended use hints.

### Phase 4: Benchmarks and Gates
- [ ] Add benchmark targets for the 6 committed future strategies.
- [ ] Run CPU and GPU benchmark matrix.
- [ ] Record baseline vs candidate timings.
- [ ] Defer any strategy that fails performance/correctness gates.

### Phase 5: Cross-Stage Wiring
- [ ] Wire validation stage for each committed future strategy.
- [ ] Wire meta-analysis stage for each committed future strategy.
- [ ] Run full workflow gates.
- [ ] Fix cross-stage regressions before declaring the strategy complete.

## Strategy Count Checklist (15 Target)

- [x] `S01` DoubleMa
- [x] `S02` Asr
- [x] `S03` CompositeCpu
- [x] `S04` CompositeTrend
- [x] `S05` RegimeHybrid
- [x] `S06` BreakoutMomentum
- [x] `S07` CompositeMo
- [x] `S08` VCC
- [x] `S09` VEMR
- [x] `S10` SET
- [x] `S11` MARRT
- [x] `S12` MTS
- [x] `S13` ORC
- [x] `S14` VFCT
- [x] `S18` ROCB

Deferred from committed target:
- `S15`
- `S16`
- `S17`
- `S19`
- `S20`
- `S21`
- `S22`
- `S23`
- `S24`
- `S25`

## Recent Optimization Log

- 2026-03-09: closed the remaining roadmap implementation gaps for ROCB controls, benchmark targets, and MARRT smoke coverage.
  - Added explicit `ROCB` parameter controls to the Vue strategy panel and strategy store (`channel_period`, `atr_period`, `atr_stop_mult`, `max_trade_bars`, plus fast/slow mapping toggles), so ROCB no longer relies only on request-builder defaults for tuning.
  - Extended `crates/app_core/benches/double_ma_opt.rs` strategy-profile coverage to include benchmark rows for `S10` (`SET`), `S11` (`MARRT`), `S12` (`MTS`), `S13` (`ORC`), and `S14` (`VFCT`) on the existing VRAM-resident double-MA kernel harness.
  - Added dedicated `crates/app_core/benches/rocb_opt.rs` for `S18` (`ROCB`) CPU and `GpuKernel` benchmark groups, and registered the target in `crates/app_core/Cargo.toml`.
  - Added focused `MARRT` CPU/GPU small-grid smoke tests in `crates/app_core/src/double_ma.rs` so that strategy family is no longer represented only by frontend/validation normalization tests.
  - Benchmark execution and timing capture were not run in this pass; this closes code/config coverage, not the measurement/evidence gate.
- 2026-03-09: implemented `S18 ROCB` on the `CompositeCpu` custom-strategy stack with only `CpuOnly` and VRAM-resident `GpuKernel` targets.
  - Added `crates/app_core/src/rocb.rs` for shared CPU/device Donchian + ATR signal construction and ROCB desired-position logic.
  - Added `crates/app_core/src/rocb_gpu_kernel.rs` plus `rocb_backtest_metrics_f32` in `crates/app_core/src/kernels/double_crossover.cu` for device-resident ROCB backtest scoring.
  - Wired optimize/validation/meta/frontend normalization through the existing composite strategy path.
  - Verified with `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`, `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`, ROCB Tauri normalization tests, and request-builder Vitest coverage.
  - Follow-up pass on 2026-03-09 added the missing benchmark target and explicit frontend controls.
- 2026-03-05: attempted two VCC optimization variants:
  - direct dual-sweep (`CudaSqueezeMomentum`) for fast `squeeze` + `momentum`,
  - pointer-based VCC metrics kernel path (skip `pack_rows_u64_ptr_f32` staging).
- Result: pointer-based path was functionally correct but regressed runtime on medium/large workloads, so it was reverted.
- Current safe VCC state:
  - row-pack staging retained,
  - CPU-vs-GPU sanity test added,
  - latest benchmark remained broadly unchanged.
- 2026-03-05: `S09` (`VEMR`) optimize path was first completed, then a follow-up audit found missing validation / analysis / meta wiring.
- 2026-03-05: that `S09` cross-stage gap was repaired.
- 2026-03-05: VEMR optimize path reduced host/device traffic:
  - `close_f32` consumed directly,
  - log returns derived on device,
  - device-side top-k used when possible.
- Latest superseding VEMR large-case benchmark:
  - `vemr/gpu_kernel/200k/large/13385pairs`: `[104.26 ms 104.99 ms 105.62 ms]`
- 2026-03-08: after upgrading to `vector-ta 0.1.7`, the app-side generic CUDA dispatch path regressed badly because host-dispatch now detours through the new device-core bridge for several indicators.
- 2026-03-08: app-side mitigation landed for GPU optimization hot paths:
  - `PreparedCudaIndicatorInputs` now exposes reusable host-side `f32` slices cleanly,
  - `S09` (`VEMR`) indicator sweeps bypass generic dispatch and call direct CUDA wrappers (`zscore`, `bollinger_bands`, `atr`),
  - `S08` (`VCC`) optimization sweeps bypass generic dispatch and call direct CUDA wrappers (`ttm_squeeze`, `squeeze_momentum`, `ttm_trend`),
  - `S02` (`ASR`) optimization sweeps bypass generic dispatch and call direct CUDA wrappers (`ttm_squeeze`, `squeeze_momentum`, `linearreg_angle`, `er`, `zscore`), with a generic fallback only for unsupported zscore params.
- 2026-03-08 validation / measurements after the mitigation:
  - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel` passed,
  - `cargo test -p ta_app_core vcc_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel` passed,
  - `cargo test -p ta_app_core asr_optimization_gpu_kernel_smoke_runs_when_cuda_available --features cuda,cuda-backtest-kernel` passed,
  - `cargo test -p ta_app_core vemr_optimization_gpu_kernel_topk_matches_cpu_small_grid --features cuda,cuda-backtest-kernel` passed,
  - `vemr/gpu_kernel/200k/large/13385pairs`: `[108.29 ms 110.78 ms 114.68 ms]` (recovered from the `~700 ms` regression band),
  - `vcc/gpu_kernel/200k/large/13385pairs`: `[559.61 ms 682.78 ms 879.71 ms]` (still noisy; no statistically clear recovery yet),
  - `asr/gpu_route/200k/large/13385pairs`: `[749.52 ms 1.0369 s 1.3396 s]` (improved versus the worst post-upgrade run, but still noisy and not clearly back at the old baseline).
- 2026-03-08: app-side CUDA input staging is now dual-resident for the shared dynamic path.
  - `PreparedCudaIndicatorInputs` now uploads `open/high/low/close/volume` plus derived source once into VRAM and exposes both host slices and device buffers/views.
  - `compute_dynamic_indicator_cuda_with_inputs` and `compute_dynamic_indicator_cuda_direct_with_inputs` now try the new `vector-ta` device-native dispatch first (`compute_cuda_device`) with app-side `first_valid` injection, and only fall back to the older host-dispatch bridge if the device-native request fails.
  - strategy sweep hot paths were updated to use the new device-input wrapper entrypoints where available:
    - `S09` (`VEMR`): `zscore_batch_from_device_ptr`, `bollinger_bands_batch_from_device_ptr`, `atr_batch_from_device_ptrs`,
    - `S08` (`VCC`): `ttm_squeeze_batch_dev_from_device_inputs`, `squeeze_momentum_batch_dev_from_device_inputs`, `ttm_trend_batch_dev_from_device_inputs`,
    - `S02` (`ASR`): `ttm_squeeze_batch_dev_from_device_inputs`, `squeeze_momentum_batch_dev_from_device_inputs`, `linearreg_angle_batch_dev_from_device_prices`, `er_batch_dev_from_device_prices`, `zscore_batch_from_device_ptr`, `kaufmanstop_batch_dev_from_device_inputs`.
- 2026-03-08 validation after the device-native app integration:
  - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel` passed,
  - `cargo test -p ta_app_core cuda_adapter_with_prepared_inputs_matches_standard_cuda_adapter --features cuda,cuda-backtest-kernel -- --nocapture` passed,
  - `cargo test -p ta_app_core vemr_optimization_gpu_kernel_topk_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture` passed,
  - `cargo test -p ta_app_core vcc_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture` passed,
  - `cargo test -p ta_app_core asr_optimization_gpu_kernel_smoke_runs_when_cuda_available --features cuda,cuda-backtest-kernel -- --nocapture` passed.
- 2026-03-09: composite validation now reuses the new prepared CUDA input seam for custom strategy candidates where it exists.
  - `run_composite_validation_blocking_with_candles` prepares `PreparedCudaIndicatorInputs` once per dataset.
  - `ASR` and `VEMR` validation candidate builders now use `Backend::Auto { device_id: 0 }` plus prepared CUDA inputs when available, with existing CPU fallback still intact.
  - This closes the biggest post-opt gap between optimization and validation for those two strategies.
  - Meta selection / holdout / ML-confidence stages do not rerun indicator computations; they consume saved validation/meta artifacts, so there is no corresponding `vector-ta` API wiring to add there.
- 2026-03-09: `S08` (`VCC`) validation / holdout parity landed.
  - `CompositeParams` and composite validation now support `vcc` candidates end-to-end.
  - `build_vcc_candidate_series(...)` mirrors the `ASR` / `VEMR` validation flow and reuses prepared CUDA inputs when available.
  - frontend validation request building now preserves `Vcc` top-row params instead of falling through the composite branch.
  - meta holdout support now explicitly includes `Vcc` in both backend and frontend support lists.
  - validation passed with:
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core vcc_validation_smoke_runs_and_returns_stats --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app holdout_support_includes_asr_strategy --features cuda,cuda-backtest-kernel -- --nocapture`
    - `npx vitest run src/services/validationRequestBuilder.test.ts src/services/metaRequestBuilder.test.ts`
- 2026-03-09: double-MA `Auto` routing now defaults to the fully device-resident kernel backend whenever the kernel route is valid.
  - The `GpuKernel` double-MA optimization path was already VRAM-resident for supported MAs via `gpu_backtest_kernel.rs` + `vector-ta` `VramMaComputer`.
  - The remaining app-side gap was backend selection: `Auto` could still choose the older host-backed `GpuOnly` sweep backend even when `GpuKernel` was available.
  - `run_double_ma_optimization_blocking_with_candles` now resolves `Auto` to `GpuKernel` whenever `gpu_kernel_blockers_for_request(...)` is empty, leaving explicit `GpuOnly` requests unchanged.
  - verification:
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core auto_backend_prefers_gpu_kernel_when_kernel_route_is_supported --features cuda,cuda-backtest-kernel -- --nocapture`
- 2026-03-09: `S11` (`MARRT`) first-pass implementation landed via the existing double-MA VRAM-resident backend.
  - Chosen as the low-debt first rollout because it can reuse the existing `run_double_ma_optimization` / `run_double_ma_validation` / holdout stack instead of introducing a new bespoke optimizer.
  - Frontend strategy selection, optimize request building, validation request building, analysis, and meta normalization now preserve `strategy_kind = "Marrt"` end-to-end instead of collapsing it back to `DoubleMa`.
  - `MARRT` is treated as a constrained double-MA-derived strategy and routes through the same fully device-resident `GpuKernel` backend when selected MAs support that path.
  - The request builder currently enforces a v1 ribbon MA whitelist to keep the strategy inside the supported/low-debt surface: `sma`, `ema`, `alma`, `wma`, `zlema`, `vwma`.
  - Backend validation and holdout support now recognize `Marrt` as a double-MA validation family strategy.
  - verification:
    - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_desktop_demo_app holdout_support_includes_asr_strategy --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app validation_context_accepts_marrt_for_double_ma_validation --features cuda,cuda-backtest-kernel -- --nocapture`
    - `npx vitest run web/src/services/optimizeRequestBuilder.test.ts web/src/services/validationRequestBuilder.test.ts web/src/services/metaRequestBuilder.test.ts`
- 2026-03-09: shared CUDA indicator staging can now chain device output into a second device-native indicator dispatch without host round-trips.
  - `dynamic_indicator_adapter.rs` now exposes `compute_dynamic_indicator_cuda_from_device_matrix_row(...)` for `Slice`-input indicators.
  - This is the missing app-side primitive for flows like `OBV -> MA smoothing` and other indicator-on-indicator device pipelines needed by planned strategies such as `S14` (`VFCT`).
  - The helper takes a `DeviceMatrixF32` row view plus explicit `first_valid`, calls `compute_cuda_device(...)` directly, and supports both `DeviceF32` and `HostF32` targets.
  - verification:
    - `cargo test -p ta_app_core cuda_adapter_supports_device_slice_chaining --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- 2026-03-09: `S14` (`VFCT`) first-pass implementation landed as a constrained double-MA-derived strategy with a dedicated volume-confirmation regime filter.
  - Chosen as the next low-debt rollout because it can reuse the existing double-MA optimize / validation / holdout stack instead of introducing another bespoke optimizer family.
  - Backend now exposes a real `volume_flow_confirm_enable` strategy profile in `StrategyConfig`, using `OBV` vs smoothed `OBV` plus `MFI` thresholds to build a directional regime filter.
  - `compute_combined_regime_filter(...)` now merges that VFCT volume-flow regime with the existing double-MA filter pipeline, so CPU and `GpuKernel` backtest execution reuse the same optimizer/validation path.
  - Frontend strategy selection, optimize request building, validation request building, analysis, and meta normalization now preserve `strategy_kind = "Vfct"` end-to-end instead of collapsing it back to `DoubleMa`.
  - The current request builder intentionally constrains VFCT to `vwma` fast/slow trend lines in v1 to stay inside the existing VRAM-resident double-MA kernel surface.
  - Validation and holdout support now recognize `Vfct` as a double-MA validation family strategy.
  - verification:
    - `cargo check -p ta_app_core`
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo check -p ta_desktop_demo_app`
    - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core cpu_vfct_small_grid_runs -- --nocapture`
    - `cargo test -p ta_app_core volume_flow_confirmation_regime_produces_directional_windows --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core gpu_vfct_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app validation_context_accepts_vfct_for_double_ma_validation -- --nocapture`
    - `cargo test -p ta_desktop_demo_app holdout_support_includes_asr_strategy -- --nocapture`
    - `npx vitest run web/src/services/optimizeRequestBuilder.test.ts web/src/services/validationRequestBuilder.test.ts web/src/services/metaRequestBuilder.test.ts`
- 2026-03-09: `S14` (`VFCT`) GPU-kernel optimization path is now fully device-native for the regime stage.
  - Added a small CUDA compose kernel `build_vfct_regime_f32` in `double_crossover.cu` to turn device-resident `OBV`, smoothed `OBV`, and `MFI` series into the directional regime buffer consumed by the existing double-MA backtest kernel.
  - `gpu_backtest_kernel.rs` now supports an optional device-resident regime input alongside the legacy host regime upload path, so the optimizer can use a prebuilt VRAM regime buffer without changing the underlying backtest kernel ABI.
  - `double_ma.rs` now skips the old CPU `compute_combined_regime_filter(...)` path for `VFCT` on the explicit `GpuKernel` route, builds `OBV` and `MFI` on device via the new `vector-ta` device API, smooths `OBV` on device through the adapter chaining helper, and launches the regime compose kernel before optimization.
  - CPU and `GpuOnly` fallback routes still use the cached host regime path by design. The `GpuKernel` route, the batched kernel-eval path, and the `GpuHybridKernel` route when it actually takes the kernel branch now switch to the device-native regime builder.
  - verification:
    - `cargo check -p ta_app_core`
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core gpu_vfct_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core vfct_device_regime_matches_cpu_regime_small --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core gpu_kernel_batched_matches_unbatched_best_params_vfct --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core gpu_hybrid_vfct_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
- 2026-03-09: `S10` (`SET`) is now wired as a first-class double-like strategy on the CPU / VRAM-resident `GpuKernel` path.
  - `StrategyConfig` now exposes a dedicated `slope_efficiency_trend_enable` profile with `linearreg_angle`, `er`, and `chop` parameters instead of reusing unrelated trend-quality knobs.
  - CPU optimization/validation reuse the existing double-MA stack through `compute_slope_efficiency_trend_regime(...)`.
  - `GpuKernel` and hybrid-kernel-when-kernel-route-taken now build the `SET` regime entirely on device: `linearreg_angle`, `er`, and `chop` are dispatched through the new `vector-ta` device-native API, then `build_set_regime_f32` composes the directional regime buffer in VRAM before the existing backtest kernel runs.
  - Frontend optimize / validation / meta normalization now preserves `strategy_kind = "Set"` end to end instead of collapsing it back to `DoubleMa`.
  - verification:
    - `cargo check -p ta_app_core`
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo check -p ta_desktop_demo_app`
    - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core cpu_set_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core gpu_set_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core slope_efficiency_trend_regime_produces_directional_windows --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core set_device_regime_matches_cpu_regime_small --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app validation_context_accepts_set_for_double_ma_validation --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app holdout_support_includes_asr_strategy --features cuda,cuda-backtest-kernel -- --nocapture`
    - `npx vitest run web/src/services/optimizeRequestBuilder.test.ts web/src/services/validationRequestBuilder.test.ts web/src/services/metaRequestBuilder.test.ts`
- 2026-03-09: `S12` (`MTS`) is now wired as a first-class double-like strategy on the CPU / VRAM-resident `GpuKernel` path.
  - `StrategyConfig` now exposes a dedicated `momentum_triple_stack_enable` profile with `roc`, `mom`, and `ppo` parameters instead of reusing unrelated momentum toggles.
  - CPU optimization/validation reuse the existing double-MA stack through `compute_momentum_triple_stack_regime(...)`.
  - `GpuKernel` and hybrid-kernel-when-kernel-route-taken now build the `MTS` regime entirely on device: `roc`, `mom`, and `ppo` are dispatched through the new `vector-ta` device-native API, then `build_mts_regime_f32` composes the directional regime buffer in VRAM before the existing backtest kernel runs.
  - Frontend optimize / validation / meta normalization now preserves `strategy_kind = "Mts"` end to end instead of collapsing it back to `DoubleMa`.
  - verification:
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core cpu_mts_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core gpu_mts_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core momentum_triple_stack_regime_produces_directional_windows --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core mts_device_regime_matches_cpu_regime_small --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app validation_context_accepts_mts_for_double_ma_validation --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app holdout_support_includes_asr_strategy --features cuda,cuda-backtest-kernel -- --nocapture`
    - `npx vitest run web/src/services/optimizeRequestBuilder.test.ts web/src/services/validationRequestBuilder.test.ts web/src/services/metaRequestBuilder.test.ts`
- 2026-03-09: `S13` (`ORC`) is now wired as a first-class double-like strategy on the CPU / VRAM-resident `GpuKernel` path.
  - `StrategyConfig` now exposes a dedicated `oscillator_reversion_confluence_enable` profile with `rsi`, `stochf`, and `willr` periods, keeping the v1 rule fixed at a 2-of-3 oversold/overbought confluence.
  - CPU optimization/validation reuse the existing double-MA stack through `compute_oscillator_reversion_confluence_regime(...)`.
  - `GpuKernel` and hybrid-kernel-when-kernel-route-taken now build the `ORC` regime entirely on device: `rsi`, `stochf(k)`, and `willr` are dispatched through the new `vector-ta` device-native API, then `build_orc_regime_f32` composes the directional regime buffer in VRAM before the existing backtest kernel runs.
  - Frontend optimize / validation / meta normalization now preserves `strategy_kind = "Orc"` end to end instead of collapsing it back to `DoubleMa`.
  - verification:
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core cpu_orc_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core gpu_orc_small_grid_runs --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core oscillator_reversion_confluence_regime_produces_directional_windows --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_app_core orc_device_regime_matches_cpu_regime_small --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app validation_context_accepts_orc_for_double_ma_validation --features cuda,cuda-backtest-kernel -- --nocapture`
    - `cargo test -p ta_desktop_demo_app holdout_support_includes_asr_strategy --features cuda,cuda-backtest-kernel -- --nocapture`
    - `npx vitest run web/src/services/optimizeRequestBuilder.test.ts web/src/services/validationRequestBuilder.test.ts web/src/services/metaRequestBuilder.test.ts`

## Source Links

Trend / momentum:
- https://pages.stern.nyu.edu/~lpederse/papers/TimeSeriesMomentum.pdf
- https://www.aqr.com/Insights/Research/Journal-Article/A-Century-of-Evidence-on-Trend-Following-Investing

Mean-reversion / stat-arb:
- https://repec.som.yale.edu/icfpub/publications/2573.pdf
- https://traders.berkeley.edu/papers/Statistical%20arbitrage%20in%20the%20US%20equities%20market.pdf

Indicator and rule references:
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/bollinger-bands
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/relative-strength-index-rsi
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/stochastic-oscillator-fast-slow-and-full
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/williams-r
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/on-balance-volume-obv
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/money-flow-index-mfi
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/volume-weighted-average-price-vwap
- https://ta-lib.github.io/ta-doc/indicator/ATR.htm

## Definition of Done

- [ ] Current baseline remains stable.
- [ ] 6 committed future strategies are implemented and benchmarked.
- [ ] All committed future strategies use only `>=5x` core indicators.
- [ ] All committed future strategies work across optimization, validation, and meta without special-case breakage.
- [ ] No major regression to existing strategy performance baselines.
