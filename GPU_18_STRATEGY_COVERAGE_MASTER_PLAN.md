# GPU Strategy Coverage Master Plan

Last updated: 2026-03-05
Status: In Progress (Wave 1 optimization-stage rollout)
Priority order: performance -> correctness -> low tech debt -> UX

## Goal

Reach a total of 25 static strategies (including existing ones), while ensuring full coverage of all indicators in `gpu_speedups.csv` with `speedup >= 2x`.

Notes:
- `speedup >= 5x` remains the recommended "GPU-first core set" for benchmark-mode (default) strategy profiles.
- `2x..5x` indicators are still useful, but some are expensive in absolute CUDA time; they must have tight search caps and/or ship in coverage-mode first.

Success criteria:
- Total strategy count in app strategy selector: `25`.
- Every `>=2x` indicator is used by at least one strategy in optimization (benchmark-mode or coverage-mode).
- New strategies are useful (not synthetic-only) and benchmarked.
- No major architecture bloat (shared pipeline, minimal one-off code paths).

## Recent Optimization Log

- 2026-03-05: attempted two VCC optimization variants:
  - direct dual-sweep (`CudaSqueezeMomentum`) for fast `squeeze` + `momentum`,
  - pointer-based VCC metrics kernel path (skip `pack_rows_u64_ptr_f32` staging).
- Result: pointer-based kernel path was **functionally correct** but showed significant runtime regression on medium/large benchmark workloads, so it was reverted.
- Current safe state:
  - VCC GPU path retains row-pack staging (`pack_rows_u64_ptr_f32`) and the existing metrics kernel path.
  - Added a CPU-vs-GPU sanity test for VCC grid parity:
    - `composite_cpu::tests::vcc_optimization_gpu_kernel_sanity_matches_cpu_small_grid`
  - Existing smoke test still passes:
    - `composite_cpu::tests::vcc_optimization_gpu_kernel_smoke_runs_when_cuda_available`
- Latest verification benchmark (`cargo bench -p ta_app_core --bench vcc_opt --features cuda,cuda-backtest-kernel`):
  - `vcc/gpu_kernel/200k/small/975pairs`: `366.37 ms 394.36 ms 421.67 ms` (no significant change).
  - `vcc/gpu_kernel/200k/medium/2365pairs`: `310.76 ms 355.16 ms 426.14 ms` (no significant change).
  - `vcc/gpu_kernel/200k/large/13385pairs`: `337.05 ms 441.15 ms 628.71 ms` (no significant change).

## Current Baseline (7 Existing Strategies)

From current app strategy options:
- `S01` `DoubleMa` (existing)
- `S02` `Asr` (existing)
- `S03` `CompositeCpu` (existing)
- `S04` `CompositeTrend` (existing)
- `S05` `RegimeHybrid` (existing)
- `S06` `BreakoutMomentum` (existing)
- `S07` `CompositeMo` (existing)

## Target Portfolio (25 Total)

Keep existing 7, add 18 new:
- `S08` Volatility Compression Continuation (`VCC`)
- `S09` Volatility Expansion Mean Reversion (`VEMR`)
- `S10` Slope + Efficiency Trend (`SET`)
- `S11` MA Ribbon Regime Trend (`MARRT`)
- `S12` Momentum Triple Stack (`MTS`)
- `S13` Oscillator Reversion Confluence (`ORC`)
- `S14` Volume Flow Confirmation Trend (`VFCT`)
- `S15` Filtered Regression Channel Trend (`FRCT`)
- `S16` Volatility/Tail-Risk Regime Switcher (`VTRS`)
- `S17` Cycle Filter Reversal (`CFR`)
- `S18` Risk Overlay Channel Breakout (`ROCB`)
- `S19` Turtle Donchian Breakout (`TDB`)
- `S20` Alligator Trend Activation (`ATA`)
- `S21` Adaptive Baseline MACZ Trend (`ABMT`)
- `S22` Ultimate-Laguerre Reversion (`ULR`)
- `S23` Mod God Mode Momentum Switch (`MGMS`)
- `S24` MESA Cycle Swing (`MCS`)
- `S25` Nadaraya-Watson Envelope Reversion (`NWER`)

## Web Research Synthesis (Strategy Viability)

Research was run in parallel across trend, mean-reversion, volume/flow, and cycle-filter domains, then normalized to this app's constraints:
- single-series OHLCV-first backtests,
- GPU-batchable signals,
- low-branch rule logic,
- shared validation/meta pipeline.

High-confidence conclusions:
- Trend-following and time-series momentum remain the most stable core for generalization (academic support is strongest).
- Mean-reversion works best when regime-gated (low trend strength / non-expansion states).
- Volume/flow indicators are best as confirmation/risk filters rather than standalone alpha in bar-only data.
- Cycle filters (Ehlers-style) are viable but must use warmup/hysteresis to avoid unstable switching.
- The MA-variant universe should be covered via one profile-driven "MA ribbon family" strategy, not many one-off strategy implementations.

## Final Recommended 18 New Strategies (To Reach 25)

These 18 are selected to be:
- GPU-friendly (batchable boolean rule logic + `gpu_speedups.csv` indicators),
- practically useful as standalone strategies (not just "coverage fillers"),
- distinct enough that users can learn from them (no 18 variants of the same crossover).

### `S08` VCC (Volatility Compression Continuation)
- Archetype: squeeze-release continuation (breakout after volatility contraction).
- Signals:
  - Compression: `ttm_squeeze` OR `bollinger_bands_width` below a low percentile.
  - Direction: `squeeze_momentum` sign and/or `ttm_trend` state.
  - Optional expansion gate: `mass` rising above a threshold (volatility expansion confirmation).
- Entry:
  - Long when compression ends and momentum is positive and `ttm_trend` is bullish.
  - Short symmetric when compression ends and momentum is negative and `ttm_trend` is bearish.
- Exit (pick one to start, keep it simple):
  - Momentum flips sign, OR
  - trailing stop overlay from `S18` primitives (ATR/DevStop/FVG stop).
- Initial parameter bounds (optimize):
  - `bb_len` 10..40, `bb_k` 1.5..3.0
  - `kc_len` 10..40, `kc_mult` 1.0..2.5
  - `mom_len` 5..50
  - `bbw_percentile` 5..25 (if using bandwidth percentile)
- References:
  - TTM Squeeze (official ToS/TTM study docs): https://tlc.thinkorswim.com/center/reference/Tech-Indicators/studies-library/T-U/TTM-Squeeze
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/ttm_squeeze/ , https://vectoralpha.dev/projects/ta/indicators/squeeze_momentum/
  - Bollinger/Keltner (rule background): https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/bollinger-bands , https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/keltner-channels

### `S09` VEMR (Volatility Expansion Mean Reversion)
- Archetype: fade large dislocations back toward equilibrium in non-trending regimes.
- Signals:
  - Dislocation: `zscore` of price vs rolling mean (or of returns) below/above threshold.
  - Band context: `bollinger_bands` distance and `minmax` stretch.
  - Risk: `atr` to size stops and gate "too-hot" volatility.
- Entry:
  - Long when `zscore <= -z_entry` and price is at/below lower band.
  - Short when `zscore >= +z_entry` and price is at/above upper band.
- Exit:
  - Mean reversion target: `zscore` crosses 0 OR price reaches band midline.
  - Stop: ATR-multiple or `S18` trailing stop primitives.
- Initial parameter bounds (optimize):
  - `z_len` 10..100, `z_entry` 1.0..3.5, `z_exit` 0.0..1.0
  - `bb_len` 10..60, `bb_k` 1.5..3.0
  - `atr_len` 7..50, `atr_stop_mult` 1.0..5.0
- References:
  - Bollinger (bands + mean reversion context): https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/bollinger-bands
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/zscore/ , https://vectoralpha.dev/projects/ta/indicators/minmax/ , https://vectoralpha.dev/projects/ta/indicators/bollinger_bands/

### `S10` SET (Slope + Efficiency Trend)
- Archetype: trend-follow only when the path is directional (high efficiency) and slope confirms.
- Signals:
  - Slope proxy: `linearreg_angle` above/below threshold.
  - "Noise vs direction": `er` (efficiency ratio) above threshold.
  - Regime gate: `chop` below threshold.
  - Optional confirmation: `dti`, `coppock`, `ttm_trend`.
- Entry:
  - Long when slope positive AND `er >= er_min` AND `chop <= chop_max`.
  - Short symmetric.
- Exit:
  - Slope flips, OR `er` falls below threshold, OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - `linreg_len` 10..80, `angle_min` 5..45 (degrees, depending on normalization)
  - `er_len` 10..80, `er_min` 0.15..0.60
  - `chop_len` 10..60, `chop_max` 40..70
  - Optional: `dti_len` 7..50, `coppock_fast/slow` small bounded sets
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/linearreg_angle/ , https://vectoralpha.dev/projects/ta/indicators/er/ , https://vectoralpha.dev/projects/ta/indicators/chop/
  - Coppock overview (historical momentum indicator): https://www.fidelity.com/bin-public/060_www_fidelity_com/documents/IndicatorsReferenceGuide.pdf

### `S11` MARRT (MA Ribbon Regime Trend)
- Archetype: MA ribbon alignment (GMMA-style) for trend participation + regime filtering.
- Signals:
  - Ribbon alignment: fast MA set above slow MA set for longs (reverse for shorts).
  - Ribbon expansion: spacing between ribbon groups increasing (trend strengthening).
  - Optional gate: `er` or `chop` (reuse `S10` primitives) to avoid chop.
- Entry:
  - Long when ribbon aligned bullish AND expansion condition holds.
  - Short symmetric.
- Exit:
  - Ribbon de-aligns OR fast group crosses slow group OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - `ma_type` from the fast-GPU MA catalog (all `S11`-owned MAs in the checklist).
  - `fast_periods` fixed sets (e.g. GMMA-like) or optimized scalar+step.
  - `slow_periods` fixed sets (e.g. GMMA-like) or optimized scalar+step.
- References:
  - GMMA (ribbon background): https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/guppy-multiple-moving-averages-gmma

### `S12` MTS (Momentum Triple Stack)
- Archetype: momentum continuation gated by multiple time-horizon confirmations.
- Signals:
  - Short momentum: `roc`/`mom`
  - Medium momentum: `ppo`/`apo`
  - Trend-impulse confirmation: `ao`/`acosc` (and optionally `cfo`/`cmo`)
- Entry:
  - Long when all selected momentum components are positive (or above thresholds).
  - Short symmetric.
- Exit:
  - Any required component flips negative OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - `roc_len` 5..80, `mom_len` 5..80
  - `ppo_fast/slow` small bounded sets around common defaults (avoid huge grid blowup)
  - `ao_fast/slow` small bounded sets (or fixed for first implementation)
- References:
  - Time-series momentum (academic framing; map to indicatorized momentum): https://pages.stern.nyu.edu/~lpederse/papers/TimeSeriesMomentum.pdf

### `S13` ORC (Oscillator Reversion Confluence)
- Archetype: range-regime mean reversion using multi-oscillator confluence.
- Signals (choose 2..4 for first shipping version; keep rest optional):
  - Oversold/overbought: `rsi`, `stochf`, `willr`, `kdj`, `srsi`, `ift_rsi`
  - Confirmation/trigger: `qqe`, `bop`, `dpo`
- Entry:
  - Long when multiple oscillators are oversold AND at least one trigger confirms reversal.
  - Short when multiple oscillators are overbought AND trigger confirms.
- Exit:
  - Oscillators mean-revert to midline, OR ATR/DevStop risk exit.
- Initial parameter bounds (optimize):
  - `rsi_len` 7..50, `rsi_os` 10..40, `rsi_ob` 60..90
  - `stoch_len` 7..50, `willr_len` 7..50
  - Keep KDJ/QQE/IFT as optional modules behind a "complexity toggle".
- References:
  - RSI + Stoch + Williams %R background: https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/relative-strength-index-rsi , https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/stochastic-oscillator-fast-slow-and-full , https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/williams-r

### `S14` VFCT (Volume Flow Confirmation Trend)
- Archetype: trend participation only when volume/flow confirms the move.
- Signals:
  - Volume flow: `obv` slope, `efi`, `mfi` bias, `vosc` regime.
  - Price reference: `vwap` and/or `vwma` alignment (price above VWAP for longs).
- Entry:
  - Long when price is above VWAP/VWMA AND at least one volume-flow confirmation is bullish.
  - Short symmetric.
- Exit:
  - Price crosses back under VWAP/VWMA OR confirmations flip OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - `mfi_len` 7..50, `efi_len` 2..50, `vosc_fast/slow` bounded sets
  - `vwap_len` (if rolling) 10..200, `vwma_len` 10..200
- References:
  - OBV/MFI/VWAP background: https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/on-balance-volume-obv , https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/money-flow-index-mfi , https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/volume-weighted-average-price-vwap

### `S15` FRCT (Filtered Regression Channel Trend)
- Archetype: trend + pullback continuation inside a smooth channel (regression + filtering).
- Signals:
  - Channel: `prb` (polynomial regression bands) as a dynamic trend channel.
  - Filter: `lpc` (low-pass channel) to suppress noise and gate entries.
  - Adaptive tracker: `otto` (optimized trend tracker oscillator) to confirm direction.
  - Optional: `cora_wave` as a smoother/confirmation and `medprice`/`wclprice` as alternate inputs.
- Entry:
  - Long when `otto` bullish AND price is above `lpc` centerline AND price reclaims `prb` midline after pullback.
  - Short symmetric.
- Exit:
  - Leave when `otto` flips OR price closes outside `prb` channel opposite direction OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - `prb_len` 20..200, `prb_degree` small bounded set (start with 1..3)
  - `lpc_len` 10..200
  - `otto_len` 10..200, `otto_sensitivity` small bounded set
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/prb/ , https://vectoralpha.dev/projects/ta/indicators/lpc/ , https://vectoralpha.dev/projects/ta/indicators/otto/ , https://vectoralpha.dev/projects/ta/indicators/cora_wave/

### `S16` VTRS (Volatility/Tail-Risk Regime Switcher)
- Archetype: tail-risk/volatility gating around a simple trend core (keeps users out of unstable regimes).
- Signals:
  - Realized volatility proxy: `var` and/or `deviation`.
  - Tail-risk proxy: `kurtosis`.
  - Intrabar structure: `correl_hl` (used as an additional stability/structure gate).
  - Trend baseline: `buff_averages` (smoother baseline for directional bias).
- Entry:
  - Long when baseline trend is up (price above baseline) AND volatility below threshold AND kurtosis below threshold.
  - Short symmetric.
- Exit:
  - If volatility or kurtosis breaches thresholds OR baseline flips OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - `vol_len` 20..400, `vol_max` dataset-relative (optimize quantile-based thresholds)
  - `kurt_len` 50..500, `kurt_max` 3.0..12.0 (distribution-dependent)
  - `buff_len` 10..200 (depending on indicator semantics)
- References:
  - Volatility-managed portfolios (academic motivation for vol gating): https://people.stern.nyu.edu/lpederse/papers/VolatilityManagedPortfolios.pdf
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/var/ , https://vectoralpha.dev/projects/ta/indicators/kurtosis/ , https://vectoralpha.dev/projects/ta/indicators/correl_hl/ , https://vectoralpha.dev/projects/ta/indicators/buff_averages/

### `S17` CFR (Cycle Filter Reversal)
- Archetype: cycle-regime mean reversion with Ehlers-style filters (avoid trend logic in cycle state).
- Signals:
  - Cycle/noise filtering: `highpass`, `highpass_2_pole`, `decycler`.
  - Regime estimate: `correlation_cycle`.
  - Optional: `mama`/`emd` for additional adaptive smoothing and decomposition.
- Entry:
  - Only trade when `correlation_cycle` indicates cycle-like regime.
  - Long when filtered cycle component turns up from low; short symmetric.
- Exit:
  - Cycle component turns down from high OR regime changes OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - `filter_len` 10..100, `cycle_gate` small bounded thresholds
  - Keep `emd`/`mama` optional to avoid over-parameterization early.
- References:
  - Ehlers filter background: https://c.mql5.com/forextsd/forum/118/predictiveindicators.pdf
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/highpass/ , https://vectoralpha.dev/projects/ta/indicators/decycler/ , https://vectoralpha.dev/projects/ta/indicators/correlation_cycle/

### `S18` ROCB (Risk Overlay Channel Breakout)
- Archetype: channel breakout (Donchian/price channel style) with explicit risk overlays.
- Signals:
  - Breakout: N-bar channel break (computed from raw OHLC; no extra indicator required).
  - Risk/exit overlays: `atr`, `devstop`, `fvg_trailing_stop`.
- Entry:
  - Long when close breaks above upper channel AND risk gate passes (optional ATR floor).
  - Short symmetric on lower channel break.
- Exit:
  - Primary: `devstop` or `fvg_trailing_stop` hit.
  - Secondary: channel breakout failure (close back inside channel).
- Initial parameter bounds (optimize):
  - `channel_len` 10..200
  - `atr_len` 7..50, `atr_floor` dataset-relative (optional)
  - `devstop_len` 5..100, `devstop_mult` 1.0..6.0
  - `fvg_len` 5..100, `fvg_sensitivity` bounded set
- References:
  - DevStop (primary source): https://kaseco.com/papers/DevStops.PDF
  - Donchian/price channel background: https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/price-channels
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/devstop/ , https://vectoralpha.dev/projects/ta/indicators/fvg_trailing_stop/ , https://vectoralpha.dev/projects/ta/indicators/atr/

### `S19` TDB (Turtle Donchian Breakout)
- Archetype: classic Turtle-style trend breakout with explicit trend-strength gating and standardized risk exits.
- Signals:
  - Entry breakout: `donchian` upper/lower channel break.
  - Trend-strength gate: `di` (DI+ vs DI-) and `adxr` above threshold.
  - Optional regime gate: `aroonosc` above/below threshold to avoid low-signal chop.
  - Exit overlay: `chandelier_exit` (ATR-based) as primary trailing stop.
- Entry:
  - Long when close breaks above Donchian upper AND `di+ > di-` AND `adxr >= adxr_min`.
  - Short when close breaks below Donchian lower AND `di- > di+` AND `adxr >= adxr_min`.
- Exit:
  - `chandelier_exit` stop hit OR close crosses opposite Donchian channel.
- Initial parameter bounds (optimize):
  - `donchian_len` 10..200
  - `di_len` 7..50, `adxr_len` 7..50, `adxr_min` 10..40
  - Optional: `aroon_len` 7..50, `aroon_min` 0..60
  - `chandelier_period` 10..50, `chandelier_mult` 1.5..5.0
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/donchian/ , https://vectoralpha.dev/projects/ta/indicators/di/ , https://vectoralpha.dev/projects/ta/indicators/adxr/ , https://vectoralpha.dev/projects/ta/indicators/aroonosc/ , https://vectoralpha.dev/projects/ta/indicators/chandelier_exit/

### `S20` ATA (Alligator Trend Activation)
- Archetype: Bill Williams Alligator "sleep -> awaken" trend participation, with trend-strength gating.
- Signals:
  - Direction and structure: `alligator` (jaw/teeth/lips ordering).
  - Trend-strength gate: `adxr` (or `di` as secondary gate).
  - Optional confirmation: `halftrend` state alignment (coverage-mode first; expensive in absolute CUDA time).
- Entry:
  - Long when `lips > teeth > jaw` AND spacing is expanding AND close is above lips.
  - Short when `lips < teeth < jaw` AND spacing is expanding AND close is below lips.
- Exit:
  - Alligator ordering breaks OR spacing contracts materially OR trailing stop overlay (reuse `S18` primitives).
- Initial parameter bounds (optimize):
  - `jaw_len` 8..21, `teeth_len` 5..13, `lips_len` 3..10
  - Offsets: small bounded sets around defaults (`jaw_off` 5..13, `teeth_off` 3..8, `lips_off` 2..5)
  - `adxr_len` 7..50, `adxr_min` 10..40
  - Optional: `halftrend_amplitude` 1..6, `halftrend_atr_period` 20..200, `halftrend_channel_dev` 0.5..4.0
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/alligator/ , https://vectoralpha.dev/projects/ta/indicators/halftrend/ , https://vectoralpha.dev/projects/ta/indicators/adxr/

### `S21` ABMT (Adaptive Baseline MACZ Trend)
- Archetype: adaptive baseline trend participation with a normalized MACD-style impulse (`macz`) for entry/exit timing.
- Signals:
  - Baseline: `vidya` (adaptive MA) in v1 core.
  - Impulse: `macz` histogram sign / cross.
  - Optional baseline alternatives (coverage-mode): `volume_adjusted_ma`, `epma`, `jma`, `supersmoother`, `edcf`, `maaq`, `mwdx`.
- Entry:
  - Long when close is above baseline AND `macz_hist > 0`.
  - Short when close is below baseline AND `macz_hist < 0`.
- Exit:
  - `macz_hist` flips sign OR close crosses baseline OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - Baseline: `baseline_len` 10..200
  - `vidya_cmo_len` 5..30
  - `macz_fast` 5..20, `macz_slow` 15..60, `macz_signal` 3..20, `macz_zlen` 20..200
  - Coverage-mode: `baseline_type` in `{vidya,volume_adjusted_ma,epma,jma,supersmoother,edcf,maaq,mwdx}`
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/vidya/ , https://vectoralpha.dev/projects/ta/indicators/macz/ , https://vectoralpha.dev/projects/ta/indicators/volume_adjusted_ma/ , https://vectoralpha.dev/projects/ta/indicators/epma/ , https://vectoralpha.dev/projects/ta/indicators/jma/ , https://vectoralpha.dev/projects/ta/indicators/supersmoother/ , https://vectoralpha.dev/projects/ta/indicators/edcf/ , https://vectoralpha.dev/projects/ta/indicators/maaq/ , https://vectoralpha.dev/projects/ta/indicators/mwdx/

### `S22` ULR (Ultimate-Laguerre Reversion)
- Archetype: range-regime mean reversion using adaptive oscillators + intrabar pressure trigger.
- Signals:
  - Primary oscillator confluence: `lrsi` (Laguerre RSI) and `ultosc` (Ultimate Oscillator).
  - Trigger: `qstick` reversal (intrabar pressure).
  - Optional volume confirmation: `adosc` sign / slope.
- Entry:
  - Long when `lrsi <= lrsi_os` AND `ultosc <= ult_os` AND `qstick` turns up.
  - Short when `lrsi >= lrsi_ob` AND `ultosc >= ult_ob` AND `qstick` turns down.
- Exit:
  - Oscillators revert to midline OR `qstick` flips back OR trailing stop overlay.
- Initial parameter bounds (optimize):
  - `lrsi_gamma` 0.1..0.8, `lrsi_os` 0.05..0.35, `lrsi_ob` 0.65..0.95
  - `ult_short` 5..14, `ult_mid` 10..28, `ult_long` 20..56, `ult_os` 20..40, `ult_ob` 60..80
  - `qstick_len` 3..30
  - Optional: `adosc_short` 2..10, `adosc_long` 5..30
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/lrsi/ , https://vectoralpha.dev/projects/ta/indicators/ultosc/ , https://vectoralpha.dev/projects/ta/indicators/qstick/ , https://vectoralpha.dev/projects/ta/indicators/adosc/

### `S23` MGMS (Mod God Mode Momentum Switch)
- Archetype: composite oscillator crossover strategy designed to reduce single-oscillator noise.
- Signals:
  - `mod_god_mode` wavetrend line vs signal line cross (histogram sign as impulse).
  - Optional filter: require histogram magnitude above threshold to avoid micro-chop.
  - Coverage note: `gpu_speedups.csv` contains `chande`; verify whether it is a CMO alias or a separate implementation and map it here if needed.
- Entry:
  - Long when `wt` crosses above `signal` and histogram crosses above 0.
  - Short when `wt` crosses below `signal` and histogram crosses below 0.
- Exit:
  - Cross in the opposite direction OR histogram decays below threshold.
- Initial parameter bounds (optimize):
  - `n1` 10..30, `n2` 3..15, `n3` 2..10
  - `mode` fixed to `tradition_mg` in v1; `use_volume` fixed to true when volume exists.
  - `hist_min` 0.0..20.0 (indicator-scale dependent; use percentile-based in v1.1+)
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/mod_god_mode/

### `S24` MCS (MESA Cycle Swing)
- Archetype: cycle swing trading using Ehlers-style cycle oscillators, gated to avoid trend regimes.
- Signals:
  - Cycle oscillator: `msw` (Mesa Sine Wave) phase/turning points.
  - Detrending: `dec_osc` for mean-zero swing confirmation.
  - Optional: use `supersmoother` as a pre-filter in coverage-mode (already owned by `S21`).
- Entry:
  - Long when MSW cycle turns up from low AND detrended oscillator confirms upward reversal.
  - Short symmetric.
- Exit:
  - MSW cycle turns down from high OR detrended oscillator crosses back through 0.
- Initial parameter bounds (optimize):
  - `msw_len` 5..40
  - `dec_osc_len` 10..60
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/msw/ , https://vectoralpha.dev/projects/ta/indicators/dec_osc/

### `S25` NWER (Nadaraya-Watson Envelope Reversion)
- Archetype: kernel regression envelope mean reversion with strict search caps (very expensive in absolute CUDA time).
- Signals:
  - `nadaraya_watson_envelope` upper/lower envelopes and centerline.
  - Optional trigger (coverage-mode): `qstick` inflection to avoid catching a falling knife.
- Entry:
  - Long when close pierces the lower envelope and then reclaims the lower envelope or centerline.
  - Short when close pierces the upper envelope and then loses the upper envelope or centerline.
- Exit:
  - Return-to-mean: reach centerline OR opposite envelope.
  - Risk: ATR/DevStop overlay (reuse `S18` primitives) if enabled.
- Initial parameter bounds (optimize):
  - `bandwidth` 2..20
  - `multiplier` 1.0..5.0
  - `lookback` 100..2000
- References:
  - VectorTA docs: https://vectoralpha.dev/projects/ta/indicators/nadaraya_watson_envelope/

## V1 Freeze Spec (Implementation Contract)

Purpose: remove ambiguity before coding and keep perf stable by bounding combinatorial search.

Rules that apply to all `S08`..`S25`:
- Exactly one v1 core profile per strategy (required modules only).
- Optional modules are disabled by default and excluded from baseline benchmarks.
- Search-space cap is enforced per strategy (if raw grid exceeds cap, use stratified/pruned sampling with deterministic seed).
- All percentile/quantile thresholds are computed on in-sample only, then frozen for validation/OOS to prevent leakage.

### Required vs Optional Modules (v1)

| Strategy | Required modules (v1 core) | Optional modules (v1.1+) | v1 search cap |
|---|---|---|---|
| `S08` `VCC` | `ttm_squeeze`, `squeeze_momentum`, `ttm_trend` | `bollinger_bands_width`, `keltner`, `mass` gate | `<= 80k` |
| `S09` `VEMR` | `zscore`, `bollinger_bands`, `atr` stop | `minmax`, `stddev` | `<= 80k` |
| `S10` `SET` | `linearreg_angle`, `er`, `chop` | `dti`, `coppock`, `ttm_trend` | `<= 80k` |
| `S11` `MARRT` | MA ribbon with fixed fast/slow period templates + one `ma_type` | `er/chop` regime gate, dynamic ribbon spacing rule | `<= 120k` |
| `S12` `MTS` | `roc`, `mom`, `ppo` | `apo`, `ao`, `acosc`, `cfo`, `cmo` | `<= 70k` |
| `S13` `ORC` | `rsi`, `stochf`, `willr` 2-of-3 confluence | `srsi`, `ift_rsi`, `kdj`, `qqe`, `bop`, `dpo` | `<= 60k` |
| `S14` `VFCT` | `vwap` (or `vwma`), `obv`, `mfi` | `efi`, `vosc` | `<= 60k` |
| `S15` `FRCT` | `prb`, `lpc`, `otto` | `cora_wave`, `medprice`, `wclprice` alternate source | `<= 70k` |
| `S16` `VTRS` | `var`, `kurtosis`, `buff_averages` | `deviation`, `correl_hl` | `<= 50k` |
| `S17` `CFR` | `highpass`, `decycler`, `correlation_cycle` | `highpass_2_pole`, `mama`, `emd` | `<= 50k` |
| `S18` `ROCB` | channel breakout + `atr` + (`devstop` OR `fvg_trailing_stop`) | dual trailing overlays enabled together | `<= 80k` |
| `S19` `TDB` | `donchian`, `di`, `adxr`, `chandelier_exit` | `aroonosc` | `<= 80k` |
| `S20` `ATA` | `alligator`, `adxr` | `di`, `halftrend` | `<= 60k` |
| `S21` `ABMT` | `vidya`, `macz` | `volume_adjusted_ma`, `epma`, `jma`, `supersmoother`, `edcf`, `maaq`, `mwdx` | `<= 70k` |
| `S22` `ULR` | `lrsi`, `ultosc`, `qstick` | `adosc` | `<= 50k` |
| `S23` `MGMS` | `mod_god_mode` | `chande` (verify alias/semantics) | `<= 30k` |
| `S24` `MCS` | `msw`, `dec_osc` | `supersmoother` pre-filter | `<= 30k` |
| `S25` `NWER` | `nadaraya_watson_envelope` | `qstick` trigger, `atr` overlay | `<= 10k` |

### Parameter Cardinality Constraints (Critical)

These constraints are hard limits for v1 to prevent `S11/S13/S17` grid explosion:
- `S11 MARRT`:
  - `ma_type` candidate set in v1 core: `alma,sma,ema,wma,vwma,vpwma,swma,tema,tilson,zlema`.
  - Fast period set size `<= 6`; slow period set size `<= 6`.
  - Ribbon templates are preset IDs; optimize template ID, not arbitrary per-line periods.
- `S13 ORC`:
  - v1 core is fixed 3-oscillator confluence (`rsi`,`stochf`,`willr`), requiring 2-of-3 agreement.
  - Optional oscillator modules are off in baseline benchmark and correctness gates.
- `S17 CFR`:
  - v1 core uses one filter chain only (`highpass -> decycler -> correlation_cycle gate`).
  - `mama`/`emd` are optional and excluded from v1 baseline.

### `S16` Quantile-Normalized Threshold Policy (Required)

- `vol_max` and `kurt_max` are not global constants; they are dataset-relative quantiles computed from the in-sample split only.
- Optimize quantile parameters directly:
  - `vol_q` in `[0.60, 0.95]` step `0.05`.
  - `kurt_q` in `[0.60, 0.95]` step `0.05`.
- During optimization:
  - convert quantiles to concrete thresholds once per dataset/split.
  - freeze thresholds for validation/OOS to avoid look-ahead bias.
- Optional hysteresis (v1.1+):
  - enter gate at `q_enter`, exit gate at `q_exit` with `q_exit >= q_enter`.

### Coverage-Mode vs Benchmark-Mode

To satisfy both quality and indicator-coverage goals without inflating baseline complexity:
- Benchmark-mode (default): v1 core modules only.
- Coverage-mode (explicit): enables optional modules and extended MA variants to satisfy full `>=2x` indicator coverage checks (and to exercise the `>=5x` core set more broadly).
- Performance reporting must separate both modes to avoid misleading baseline numbers.

## Implementation Order (Recommended)

- Wave 1 (highest ROI / easiest integration): `S08`, `S09`, `S10`, `S11`.
- Wave 2 (signal diversification): `S12`, `S13`, `S14`.
- Wave 3 (specialized profiles): `S15`, `S16`, `S17`, `S18`.
- Wave 4 (>=2x coverage expansion): `S19`, `S20`, `S21`, `S22`, `S23`, `S24`, `S25`.

This order maximizes practical utility early while preserving low tech debt.

## Research Checklist (Completed)

- [x] Trend archetype review (papers + official indicator docs)
- [x] Mean-reversion archetype review (papers + official indicator docs)
- [x] Volume/flow archetype review (official docs + microstructure references)
- [x] Cycle-filter archetype review (Ehlers references + platform docs)
- [x] Consolidated strategy slate aligned with 25-total target
- [x] Mapped indicator-coverage ownership to strategy IDs

## Source Links (Web Research)

Trend / momentum:
- https://pages.stern.nyu.edu/~lpederse/papers/TimeSeriesMomentum.pdf
- https://www.nber.org/papers/w20439
- https://afajof.org/issue/volume-47-issue-5/
- https://mebfaber.com/wp-content/uploads/2016/05/SSRN-id962461.pdf

Mean-reversion / stat-arb:
- https://repec.som.yale.edu/icfpub/publications/2573.pdf
- https://traders.berkeley.edu/papers/Statistical%20arbitrage%20in%20the%20US%20equities%20market.pdf
- https://www.nber.org/papers/w2533

Indicator and rule references (official docs):
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/bollinger-bands
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/bollinger-bandwidth
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/keltner-channels
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/relative-strength-index-rsi
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/stochastic-oscillator-fast-slow-and-full
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/williams-r
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/average-directional-index-adx
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/average-true-range-atr-and-average-true-range-percent-atrp
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/on-balance-volume-obv
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-indicators/money-flow-index-mfi
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/volume-weighted-average-price-vwap
- https://chartschool.stockcharts.com/table-of-contents/technical-indicators-and-overlays/technical-overlays/price-channels
- https://ta-lib.github.io/ta-doc/indicator/ATR.htm
- https://ta-lib.github.io/ta-doc/indicator/LINEARREG_SLOPE.htm

Cycle / filter references:
- https://c.mql5.com/forextsd/forum/118/predictiveindicators.pdf
- https://traders.com/Documentation/FEEDbk_docs/2015/09/TradersTips.html
- https://traders.com/Documentation/FEEDbk_docs/2016/09/TradersTips.html
- https://toslc.thinkorswim.com/center/reference/Tech-Indicators/studies-library/E-F/EhlersHighpassFilter
- https://toslc.thinkorswim.com/center/reference/Tech-Indicators/studies-library/E-F/EhlersRoofingFilter

## Architecture Constraints (To Avoid Tech Debt)

- Keep one shared strategy execution pipeline for:
  - optimization,
  - validation,
  - meta-analysis.
- Strategy definitions should be profile-driven (config/spec structs), not large copy-paste modules.
- Reuse existing indicator dispatch and GPU routing policy:
  - Benchmark-mode: GPU-first only when indicator is in `gpu_speedups.csv` and `speedup >= 5x`.
  - Coverage-mode: allow GPU route down to `speedup >= 2x` when explicitly enabled and search caps are tight.
  - Fallback to CPU route when needed, but avoid hidden mixed-route surprises.
- No per-strategy CUDA kernel forks unless profiling proves clear ROI.
- One benchmark harness template reused by all new strategies.

## Performance Guardrails

- Primary backend for new strategies: `GpuKernel` unified path.
- Keep data VRAM-resident whenever possible; minimize host-device transfers.
- For each new strategy, require benchmark rows at minimum:
  - 200k candles / small search
  - 200k candles / medium search
  - 200k candles / large search
- Acceptance gate per strategy:
  - No correctness regression vs CPU reference on golden datasets.
  - End-to-end strategy speed should be meaningfully better than `CpuOnly` on at least medium+ search spaces.
  - If not faster in practice, strategy remains behind feature flag until optimized.

## Rollout Policy (Optimization-First)

- New-strategy rollout order is intentionally optimization-first:
  - backend optimization path,
  - frontend optimization controls,
  - benchmarks/perf gates.
- Validation and meta-analysis integration is intentionally deferred to the final stage after all new strategies are wired for optimization.
- Rationale:
  - maximize shared function/kernel reuse across strategies,
  - reduce one-off validation/meta code paths,
  - lower long-term technical debt.

## Per-Strategy Implementation Checklist (Mandatory)

This checklist is required for every new strategy (`S08`..`S25`) before it is considered "optimization-stage complete":

- [ ] CPU backend path implemented and routed from optimize request.
- [ ] GPU backend path implemented and routed from optimize request (`GpuKernel` path required).
- [ ] Unit tests:
  - [ ] optimization smoke test (CPU),
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
- [ ] Explicit note whether validation/meta integration is complete or intentionally deferred to Phase 5.

### Current Per-Strategy Status

`S08` VCC:
- [x] CPU backend
- [x] GPU backend
- [x] Unit tests (CPU smoke, GPU smoke, CPU-vs-GPU sanity)
- [x] GPU benchmark target + benchmark group
- [x] Frontend optimize wiring with parameters
- [x] Frontend tests
- [ ] Validation/meta integration (deferred to Phase 5 by policy)

`S09` VEMR:
- [x] CPU backend
- [x] GPU backend
- [x] Unit tests (CPU smoke, GPU smoke, CPU-vs-GPU sanity)
- [x] GPU benchmark target + benchmark group
- [x] Frontend optimize wiring with parameters
- [x] Frontend tests
- [ ] Validation/meta integration (deferred to Phase 5 by policy)

## Implementation Phases Checklist

### Phase 0: Freeze Scope
- [ ] Freeze the 25 strategy IDs and names.
- [ ] Confirm current 7 strategy baseline in UI + backend enums/normalization.
- [ ] Freeze `gpu_speedups.csv` threshold policy (core `>= 5x`, extended `>= 2x`).
- [ ] Freeze no-telemetry requirement for this rollout.

### Phase 1: Strategy Specs (No Code Yet)
- [ ] Write concrete entry/exit/risk rules for `S08`..`S25`.
- [ ] Define parameter bounds/defaults for each new strategy.
- [ ] Define optimization search-space limits per strategy.
- [ ] Define shared reusable signal primitives each strategy can compose.
- [ ] Freeze required vs optional module sets for each strategy (`V1 Freeze Spec`).
- [ ] Freeze benchmark-mode vs coverage-mode execution policy.
- [ ] Freeze `S16` quantile-threshold normalization and no-leakage conversion policy.

### Phase 2: Backend Integration (Optimization-Only)
- [ ] Add static strategy profile registry entries for `S08`..`S25`.
- [ ] Wire optimization execution for each new strategy.
- [ ] Add strategy-specific serialization compatibility tests.

### Phase 3: Frontend Integration
- [ ] Add strategy selector entries for `S08`..`S25`.
- [ ] Add parameter controls (or shared schema-driven controls) for each new strategy.
- [ ] Add strategy descriptions and intended use hints.
- [ ] Add UX warning for indicators not meeting GPU threshold (if user can override).

### Phase 4: Benchmarks and Gates
- [ ] Add benchmark targets for `S08`..`S25`.
- [ ] Run CPU and GPU benchmark matrix.
- [ ] Run optimization-focused workflow/perf gates.
- [ ] Record baseline vs candidate for each strategy.
- [ ] Flag and defer any strategy that fails performance/correctness gates.

### Phase 5: Final Cross-Stage Wiring (Validation + Meta)
- [ ] Freeze optimization-stage implementation for all new strategies (`S08`..`S25`).
- [ ] Wire validation stage for each new strategy using shared primitives.
- [ ] Wire meta-analysis stage for each new strategy using shared primitives.
- [ ] Run full workflow gates (optimization + validation + meta).
- [ ] Fix cross-stage regressions with reuse-first refactors (avoid strategy-specific forks).

## Strategy Count Checklist (25 Target)

- [x] `S01` DoubleMa (existing)
- [x] `S02` Asr (existing)
- [x] `S03` CompositeCpu (existing)
- [x] `S04` CompositeTrend (existing)
- [x] `S05` RegimeHybrid (existing)
- [x] `S06` BreakoutMomentum (existing)
- [x] `S07` CompositeMo (existing)
- [x] `S08` VCC
- [x] `S09` VEMR
- [ ] `S10` SET
- [ ] `S11` MARRT
- [ ] `S12` MTS
- [ ] `S13` ORC
- [ ] `S14` VFCT
- [ ] `S15` FRCT
- [ ] `S16` VTRS
- [ ] `S17` CFR
- [ ] `S18` ROCB
- [ ] `S19` TDB
- [ ] `S20` ATA
- [ ] `S21` ABMT
- [ ] `S22` ULR
- [ ] `S23` MGMS
- [ ] `S24` MCS
- [ ] `S25` NWER

## Implementation Log

- 2026-03-05: `S08` (`VCC`) backtest optimization backend implemented in `ta_app_core`.
- Scope completed: optimization only (`CompositeCpuRequest.vcc` route) with CPU path + VRAM-resident GPU-kernel path (`vcc_backtest_metrics_f32`) and CPU fallback semantics.
- 2026-03-05: Added kernel-focused benchmark target `crates/app_core/benches/vcc_opt.rs` (`vcc/gpu_kernel/200k`) with `Backend::GpuKernel` only.
- 2026-03-05: Nsight Compute pass (VCC, 200k/large) indicates runtime is dominated by indicator kernels (`ttm_squeeze_batch_f32`, `squeeze_momentum_batch_f32_opt`) while `vcc_backtest_metrics_f32` is secondary. Added a launch-policy tuning pass in `vcc_eval_block_x` (smaller blocks for low/medium combo counts) and enforced GPU-route assertions in `vcc_opt` benchmark.
- 2026-03-05: VCC indicator-path pass: replaced the separate VCC squeeze-state GPU sweep with `squeeze_momentum` output `squeeze` (with fallback to legacy `ttm_squeeze`), and mirrored the same fallback policy in CPU signal construction (`compute_vcc_squeeze_state_series`). Subsequent reruns were high-variance and did not establish a stable broad gain; keep this path as neutral unless later profiled evidence is stronger.
- 2026-03-05: Optimize UI wiring for `S08` (`VCC`) now includes full parameter controls and request mapping (store -> UI -> optimize payload).
- Validation/meta wiring for `S08` is intentionally deferred to final Phase 5 (after optimization-stage rollout across all new strategies).
- 2026-03-05: `S09` (`VEMR`) optimization-stage implementation completed:
  - backend module `crates/app_core/src/vemr.rs`,
  - composite optimizer routing via `CompositeCpuRequest.vemr`,
  - CPU + GPU optimize paths (with CPU fallback semantics),
  - request exclusivity enforcement with `asr`/`vcc`/`vemr`.
- 2026-03-05: Added VEMR tests in `crates/app_core/src/composite_cpu.rs`:
  - `vemr_optimization_smoke_runs`,
  - `vemr_optimization_gpu_kernel_smoke_runs_when_cuda_available`,
  - `vemr_optimization_gpu_kernel_sanity_matches_cpu_small_grid`.
- 2026-03-05: Added VEMR benchmark target `crates/app_core/benches/vemr_opt.rs` (`vemr/gpu_kernel/200k`).
- 2026-03-05: Latest VEMR GPU benchmark run (`cargo bench -p ta_app_core --bench vemr_opt --features cuda,cuda-backtest-kernel`):
  - `vemr/gpu_kernel/200k/small/975pairs`: `[3.6744 s 4.1351 s 4.6382 s]`
  - `vemr/gpu_kernel/200k/medium/2365pairs`: `[6.7880 s 7.1636 s 7.6864 s]`
  - `vemr/gpu_kernel/200k/large/13385pairs`: `[28.488 s 28.798 s 29.039 s]`
- 2026-03-05: VEMR optimize frontend wiring completed:
  - strategy option + Vue store defaults/snapshot state,
  - strategy workspace parameter controls,
  - optimize request builder payload (`vemr`) + validation,
  - optimize service auto-route to composite CPU command,
  - frontend tests updated for request builder and service route.
- 2026-03-05: Follow-up VEMR wiring audit found the optimize path was complete, but validation and analysis were still incomplete:
  - frontend validation builder rejected `VEMR`,
  - backend `CompositeValidationRequest`/`CompositeParams` did not support `vemr`,
  - composite CPU drilldown + local stability returned `not implemented yet` for `VEMR`,
  - analysis/execution builders were not forwarding `vemr`,
  - meta holdout support did not recognize `VEMR`.
- 2026-03-05: Rectified the deferred `S09` cross-stage wiring:
  - backend composite validation now supports `CompositeParams.vemr`,
  - backend composite drilldown + local stability now execute VEMR paths,
  - frontend validation builder now emits VEMR candidates instead of throwing,
  - frontend analysis/execution wiring now forwards `vemr`,
  - frontend + backend meta holdout support now recognizes `VEMR`,
  - targeted verification passed:
    - `cargo check -p ta_app_core`
    - `cargo check -p ta_desktop_demo_app`
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core vemr_`
    - `cargo test -p ta_desktop_demo_app holdout_support_includes_asr_strategy`
    - `npx vitest run src/services/validationRequestBuilder.test.ts src/services/metaRequestBuilder.test.ts`
- 2026-03-05: Reduced the remaining VEMR host/device traffic in the optimize path:
  - VEMR GPU launcher now consumes the prepared `close_f32` series directly instead of re-casting `close_f64`,
  - log returns are now derived on device via `compute_log_returns_f32`,
  - when `include_all=false` and `heatmap_bins=0`, VEMR now does device-side top-k row selection and copies back only the selected metric rows instead of the full combo metric matrix,
  - targeted verification passed:
    - `cargo check -p ta_app_core`
    - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core vemr_ --features cuda,cuda-backtest-kernel`
    - `cargo bench -p ta_app_core --bench vemr_opt --features cuda,cuda-backtest-kernel -- "vemr/gpu_kernel/200k/large/13385pairs" --sample-size 10 --measurement-time 1 --warm-up-time 1`
  - latest large-case benchmark: `vemr/gpu_kernel/200k/large/13385pairs` improved to `[104.26 ms 104.99 ms 105.62 ms]`.
- Residual `S09` backend gap: VEMR is still not fully end-to-end VRAM resident. The backtest stage now uploads only `close_f32` plus combo metadata, derives log returns on device, and can keep top-k selection on device, but the indicator sweep stage is still bounded by `vector-ta`'s host-slice CUDA dispatch API. `include_all=true` and heatmap requests also still require copying the full metric matrix back to the CPU.

## Indicator Coverage Checklist (`speedup >= 2x`)

Coverage target: 109 indicators.
Legend: primary owner strategy ID is initial planned owner for implementation.

- [ ] `acosc` (124.67x) -> owner `S12`
- [ ] `adosc` (3.49x) -> owner `S22`
- [ ] `adxr` (2.43x) -> owner `S19`
- [ ] `alligator` (4.76x) -> owner `S20`
- [ ] `alma` (42.92x) -> owner `S01`
- [ ] `ao` (103.25x) -> owner `S12`
- [ ] `apo` (10.83x) -> owner `S12`
- [ ] `aroonosc` (2.42x) -> owner `S19`
- [ ] `atr` (5.39x) -> owner `S18`
- [ ] `bollinger_bands` (105.06x) -> owner `S09`
- [ ] `bollinger_bands_width` (273.11x) -> owner `S08`
- [ ] `bop` (85.23x) -> owner `S13`
- [ ] `buff_averages` (313.50x) -> owner `S16`
- [ ] `cfo` (5.11x) -> owner `S12`
- [ ] `chande` (3.67x) -> owner `S23`
- [ ] `chandelier_exit` (2.63x) -> owner `S19`
- [ ] `chop` (27.23x) -> owner `S10`
- [ ] `cmo` (8.84x) -> owner `S12`
- [ ] `coppock` (110.63x) -> owner `S10`
- [ ] `cora_wave` (9.63x) -> owner `S15`
- [ ] `correl_hl` (232.89x) -> owner `S16`
- [ ] `correlation_cycle` (28.58x) -> owner `S17`
- [ ] `cwma` (20.36x) -> owner `S11`
- [ ] `dec_osc` (2.59x) -> owner `S24`
- [ ] `decycler` (6.48x) -> owner `S17`
- [ ] `deviation` (87.07x) -> owner `S16`
- [ ] `devstop` (9.63x) -> owner `S18`
- [ ] `di` (3.37x) -> owner `S19`
- [ ] `donchian` (4.08x) -> owner `S19`
- [ ] `dpo` (104.10x) -> owner `S13`
- [ ] `dti` (9.22x) -> owner `S10`
- [ ] `edcf` (2.12x) -> owner `S21`
- [ ] `efi` (12.35x) -> owner `S14`
- [ ] `ehma` (54.55x) -> owner `S11`
- [ ] `ema` (15.57x) -> owner `S11`
- [ ] `emd` (7.33x) -> owner `S17`
- [ ] `epma` (3.41x) -> owner `S21`
- [ ] `er` (92.82x) -> owner `S10`
- [ ] `fvg_trailing_stop` (10.15x) -> owner `S18`
- [ ] `fwma` (14.45x) -> owner `S11`
- [ ] `halftrend` (3.63x) -> owner `S20`
- [ ] `highpass` (17.61x) -> owner `S17`
- [ ] `highpass_2_pole` (7.91x) -> owner `S17`
- [ ] `ift_rsi` (21.85x) -> owner `S13`
- [ ] `jma` (2.61x) -> owner `S21`
- [ ] `jsa` (75.54x) -> owner `S11`
- [ ] `kaufmanstop` (301.96x) -> owner `S02`
- [ ] `kdj` (157.45x) -> owner `S13`
- [ ] `keltner` (94.52x) -> owner `S08`
- [ ] `kurtosis` (507.44x) -> owner `S16`
- [ ] `linearreg_angle` (778.34x) -> owner `S02`
- [ ] `lpc` (18.59x) -> owner `S15`
- [ ] `lrsi` (4.12x) -> owner `S22`
- [ ] `macd` (43.95x) -> owner `S06`
- [ ] `macz` (2.82x) -> owner `S21`
- [ ] `mama` (17.90x) -> owner `S17`
- [ ] `maaq` (2.11x) -> owner `S21`
- [ ] `mass` (396.28x) -> owner `S08`
- [ ] `medprice` (81.28x) -> owner `S15`
- [ ] `mfi` (14.51x) -> owner `S14`
- [ ] `minmax` (400.14x) -> owner `S09`
- [ ] `mom` (80.02x) -> owner `S12`
- [ ] `mod_god_mode` (3.69x) -> owner `S23`
- [ ] `msw` (2.50x) -> owner `S24`
- [ ] `mwdx` (2.11x) -> owner `S21`
- [ ] `nadaraya_watson_envelope` (4.26x) -> owner `S25`
- [ ] `nma` (23.25x) -> owner `S11`
- [ ] `obv` (19.69x) -> owner `S14`
- [ ] `otto` (5.74x) -> owner `S15`
- [ ] `ppo` (28.45x) -> owner `S12`
- [ ] `prb` (13.84x) -> owner `S15`
- [ ] `pwma` (14.78x) -> owner `S11`
- [ ] `qstick` (4.36x) -> owner `S22`
- [ ] `qqe` (13.51x) -> owner `S13`
- [ ] `roc` (80.54x) -> owner `S12`
- [ ] `rocr` (102.88x) -> owner `S12`
- [ ] `rsi` (8.15x) -> owner `S13`
- [ ] `sama` (16.34x) -> owner `S11`
- [ ] `sinwma` (5.16x) -> owner `S11`
- [ ] `sma` (34.41x) -> owner `S01`
- [ ] `smma` (16.46x) -> owner `S11`
- [ ] `squeeze_momentum` (116.05x) -> owner `S02`
- [ ] `sqwma` (21.76x) -> owner `S11`
- [ ] `srsi` (122.77x) -> owner `S13`
- [ ] `srwma` (22.49x) -> owner `S11`
- [ ] `stddev` (9.92x) -> owner `S09`
- [ ] `stochf` (50.44x) -> owner `S13`
- [ ] `supersmoother` (2.19x) -> owner `S21`
- [ ] `swma` (12.03x) -> owner `S11`
- [ ] `tema` (6.30x) -> owner `S11`
- [ ] `tilson` (5.91x) -> owner `S11`
- [ ] `trima` (11.54x) -> owner `S11`
- [ ] `ttm_squeeze` (1744.31x) -> owner `S02`
- [ ] `ttm_trend` (103.95x) -> owner `S10`
- [ ] `uma` (29.46x) -> owner `S11`
- [ ] `ultosc` (4.54x) -> owner `S22`
- [ ] `var` (92.47x) -> owner `S16`
- [ ] `vosc` (102.11x) -> owner `S14`
- [ ] `vidya` (3.88x) -> owner `S21`
- [ ] `volume_adjusted_ma` (3.07x) -> owner `S21`
- [ ] `vpwma` (11.59x) -> owner `S11`
- [ ] `vwap` (49.35x) -> owner `S14`
- [ ] `vwma` (51.70x) -> owner `S14`
- [ ] `wclprice` (72.71x) -> owner `S15`
- [ ] `wilders` (16.91x) -> owner `S11`
- [ ] `willr` (96.29x) -> owner `S13`
- [ ] `wma` (8.03x) -> owner `S11`
- [ ] `zlema` (10.01x) -> owner `S11`
- [ ] `zscore` (510.23x) -> owner `S09`

## Practical Notes

- Existing `CompositeCpu` remains for compatibility, but new strategy work should prioritize GPU-strong indicator sets.
- If one strategy ends up low-utility after benchmarking, replace it with another profile, but keep total count = 25.
- Do not duplicate validation/meta code paths per strategy; strategy profile should plug into the same workflow hooks.

## Definition of Done

- [ ] 25 strategies available in optimize strategy selector.
- [ ] 109/109 indicators (`>=2x`) checked as covered.
- [ ] Validation + meta-analysis run for all 25 without special-case breakage.
- [ ] Benchmarks recorded for all new strategies (CPU and GPU backends).
- [ ] No major regression to existing strategy performance baselines.
