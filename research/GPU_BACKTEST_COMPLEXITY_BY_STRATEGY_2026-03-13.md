# GPU Backtest Complexity By Strategy

## Goal

Formulate the algorithmic complexity of optimization and validation for every implemented backtest strategy under a GPU-executed path, with a second lens for the hypothetical case where no strategy-local parameter is fixed and every parameter is ranged.

This note is meant to answer one practical question:

- when does a more specialized CUDA optimization/validation path actually become justified instead of a simpler GPU brute-force / exhaustive GPU sweep?

## Important scope note

There are two materially different search-space shapes in this repo:

1. **Current product shape**
   Most strategies still optimize only the shared `fast_len x slow_len` grid, with `fast < slow`. Double-like strategies can additionally sweep fast/slow MA internal parameters (`fast_ma_param_sweeps`, `slow_ma_param_sweeps`), but most strategy-local knobs are currently fixed request config, not candidate-space axes.
2. **Hypothetical full-range shape**
   Per your request, I also model the case where every strategy-local parameter has its own range and nothing is fixed. That is the case where CUDA starts to make sense much more often.

The biggest current-state conclusion is simple:

- today, many strategies are still effectively `O(N * P)` optimizers, where `P` is just the valid fast/slow pair count, so a simple GPU brute-force implementation can remain very competitive
- the strongest CUDA case appears once the search space becomes `O(N * P * R_strategy)` with a large strategy-local multiplier `R_strategy`, or when validation multiplies fixed-candidate reruns across windows, slices, bootstraps, and datasets

## Notation

- `N`: candle count
- `F`: number of fast lengths in `fast_range`
- `S`: number of slow lengths in `slow_range`
- `P`: valid MA-length pairs, `|{(f,s): f < s}|`
- `Vf`, `Vs`: fast/slow MA-parameter variant counts from `fast_ma_param_sweeps` and `slow_ma_param_sweeps`
- `V = Vf * Vs`
- `K`: validation candidate count after shortlist / `max_candidates`
- `W`: walk-forward window count
- `Rw`: reopt walk-forward window count
- `Fw`: reopt finalist count
- `Z`: CSCV/SPA slice count
- `Comb(Z)`: CSCV combinations, `min(binomial(Z, Z/2), max_combinations)`
- `B`: SPA bootstrap samples
- `Ppert`: perturb iterations
- `Q`: trade-bootstrap simulations
- `D`: cross-ticker dataset count
- `T`: closed trades in one candidate
- `R[...]`: Cartesian product of the range cardinalities of the listed parameters
- `Pair[...]`: Cartesian product with an ordering constraint, for example `Pair[macd_fast_period, macd_slow_period]` means valid `(fast, slow)` assignments with `fast < slow`
- `U_*`: number of unique parameter values actually present in the fixed validation candidate set for a given sweep family

For readability, I factor out shared execution/risk knobs into one term:

- `R_exec`: shared `StrategyConfig` search multiplier if you also range common execution settings such as costs, stop-loss, take-profit, cooldown, persistence, exit mode, Kaufman stop settings, `long_only`, `allow_flip`, and `eps_rel`

If you truly range those knobs too, multiply every optimization candidate count below by `R_exec`.

## Reading the formulas correctly

The hypothetical full-range products in this note should be read as **upper-bound Cartesian products before implementation-specific reductions**.

Two reductions matter in this repo:

- some strategy structs include mapping flags such as `map_fast_to_*` or `map_slow_to_*`; when those are enabled, an explicit local period may become derived from `fast_len` or `slow_len` instead of acting as an independent dimension
- several strategies clamp or derive local periods from `fast_len` and `slow_len`, so distinct raw parameter tuples can collapse to the same effective runtime candidate

So the full-range formulas below answer the product question "what if every knob were independently ranged?" rather than the narrower implementation question "how many distinct effective candidates does the current derived-period code actually realize?"

## Optimization

### General GPU cost model

For exact-grid GPU optimization, the useful decomposition is:

`T_opt(strategy) = O(N * (Precompute_strategy + C_strategy))`

Where:

- `Precompute_strategy` is the sum of independent indicator sweep rows built once on device or staged for the kernel path
- `C_strategy` is the full candidate count actually scored by the GPU backtest evaluator

For simple strategies, `Precompute_strategy` is small and `C_strategy` is only `P` or `P * V`.
For richer strategies, `Precompute_strategy` is still linear in `N`, but `C_strategy` becomes `P * R_strategy`, which is where CUDA becomes compelling.

### Current-state search-space reality

Today, the product builder mostly produces these candidate counts:

- `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`, `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`: `C_now = P * V`
- `Asr`, `Vcc`, `Vemr`, `Rocb`, `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`, `CompositeCpu`: `C_now = P`
- `CompositeMo`: sampler-budget driven, not an exact full-grid product path

That explains why simple double-MA GPU brute force can still win in practice.

### Double-like family

All of these route through the double-MA optimizer in the current product path. The intended GPU route is:

- `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`: `GpuKernel` when eligible, otherwise hybrid / sweep fallback
- `CompositeTrend`: `GpuKernel` with ADX off, `GpuHybridKernel` with ADX on
- `RegimeHybrid`: `GpuHybridKernel`
- `BreakoutMomentum`: `GpuKernel` with ADX off, `GpuHybridKernel` with ADX on

Their generic optimization cost is:

`T = O(N * (F * Vf + S * Vs + G_strategy + C_full))`

Where `G_strategy` is the one-time regime/filter sweep work listed below.

| Strategy | Current `C_now` | Hypothetical full-range `C_full` | Strategy-local GPU staging term `G_strategy` |
| --- | --- | --- | --- |
| `DoubleMa` | `P * V` | `P * V * R_exec` | none beyond MA staging |
| `Pit` | `P * V` | `P * V * R[pit_efi_period, pit_vosc_short_period, pit_vosc_long_period, pit_cfo_period, pit_cfo_scalar, pit_confirm_bars] * R_exec` | EFI + VOSC + CFO |
| `Set` | `P * V` | `P * V * R[set_linearreg_period, set_angle_min_deg, set_er_period, set_er_min, set_chop_period, set_chop_max] * R_exec` | linear-reg angle + ER + CHOP |
| `Mts` | `P * V` | `P * V * R[mts_roc_period, mts_mom_period] * Pair[mts_ppo_fast_period, mts_ppo_slow_period] * R_exec` | ROC + MOM + PPO |
| `Orc` | `P * V` | `P * V * R[orc_rsi_period, orc_stoch_fastk_period, orc_stoch_fastd_period, orc_willr_period] * R_exec` | RSI + Stoch + WillR |
| `Marrt` | `P * V` | `P * V * R_exec` | no extra strategy-local indicator family |
| `Vfct` | `P * V` | `P * V * R[volume_flow_obv_smooth_period, volume_flow_mfi_period, volume_flow_mfi_long_min, volume_flow_mfi_short_max, volume_flow_confirm_bars] * R_exec` | OBV + OBV EMA + MFI |
| `CompositeTrend` | `P * V` | `P * V * R[trend_quality_linearreg_period, trend_quality_angle_min_deg, trend_quality_bbw_period, trend_quality_bbw_mult, trend_quality_bbw_min, trend_quality_roc_period, trend_quality_use_roc, trend_quality_use_adx, trend_quality_adx_period, trend_quality_adx_min] * R_exec` | linear-reg angle + BB width + optional ROC + optional ADX |
| `RegimeHybrid` | `P * V` | `P * V * R[trend_quality_linearreg_period, trend_quality_angle_min_deg, trend_quality_bbw_period, trend_quality_bbw_mult, trend_quality_bbw_min, trend_quality_roc_period, trend_quality_use_roc, regime_hybrid_adx_period, regime_hybrid_adx_enter, regime_hybrid_adx_exit, regime_hybrid_reversion_entry_rel, regime_hybrid_reversion_exit_rel] * R_exec` | trend-quality regime + ADX hysteresis + range-state logic |
| `BreakoutMomentum` | `P * V` | `P * V * R[breakout_bbands_period, breakout_bbands_mult, breakout_confirm_bars, breakout_macd_signal_period, breakout_cmo_period, breakout_cmo_min_abs, breakout_use_adx, breakout_adx_period, breakout_adx_min] * Pair[breakout_macd_fast_period, breakout_macd_slow_period] * Pair[breakout_adosc_short_period, breakout_adosc_long_period] * R_exec` | Bollinger + MACD + CMO + ADOSC + optional ADX |

### Composite / specialized family

These route through `run_composite_cpu_optimization`, but the strategies with dedicated GPU kernels are the specialized families `Asr` through `Cdmc`. `CompositeCpu` itself also has a base-composite GPU kernel route. `CompositeMo` is separate and sampler-driven.

Their generic optimization cost is:

`T = O(N * (Precompute_strategy + C_full))`

For the current builder, `C_now = P` for every strategy in this section except `CompositeMo`.

| Strategy | Current `C_now` | Hypothetical full-range `C_full` | GPU precompute shape |
| --- | --- | --- | --- |
| `CompositeCpu` | `P` | `P * R[halftrend_amplitude, halftrend_channel_deviation, halftrend_atr_period, macz_fast_length, macz_slow_length, macz_signal_length, macz_lengthz, macz_length_stdev, macz_a, macz_b, macz_use_lag, macz_gamma, macz_use_volume, ott_period, ott_percent, ott_ma_type, signal_mode] * R_exec` | `O(N * (F + S + P))` plus HalfTrend + MACZ + OTT |
| `Asr` | `P` | `P * R[map_fast_to_squeeze_period, map_slow_to_trend_period, map_slow_to_er_period, map_slow_to_z_period, squeeze_period, squeeze_bb_mult, squeeze_kc_mult, trend_period, angle_deg_min, angle_deg_mr, use_er_filter, er_period, er_min, er_mr, z_period, z_ma_type, z_nbdev, z_devtype, z_entry, z_exit, confirm_bars, mom_fade_bars, max_trade_bars, max_mr_trade_bars] * R_exec` | `O(N * (2U_squeeze + 3U_slow + 1))` |
| `Vcc` | `P` | `P * R[map_fast_to_squeeze_period, map_fast_to_momentum_period, map_slow_to_trend_period, squeeze_period, momentum_period, momentum_bb_period, trend_period, squeeze_bb_mult, squeeze_kc_mult, confirm_bars, require_momentum_accel, max_trade_bars] * R_exec` | `O(N * (2U_fast + U_trend))` |
| `Vemr` | `P` | `P * R[map_fast_to_z_period, map_fast_to_bb_period, map_slow_to_atr_period, z_period, z_ma_type, z_nbdev, z_devtype, z_entry, z_exit, bb_period, bb_mult, require_band_touch, atr_period, atr_stop_mult, confirm_bars, max_trade_bars] * R_exec` | `O(N * (4U_fast + U_atr))` |
| `Rocb` | `P` | `P * R[map_fast_to_channel_period, map_slow_to_atr_period, channel_period, atr_period, atr_stop_mult, max_trade_bars] * R_exec` | `O(N * (2U_channel + U_atr))` |
| `Mkcs` | `P` | `P * R[map_fast_to_stddev_period, map_slow_to_breakout_lookback, stddev_period, breakout_lookback, compression_threshold, reexpansion_threshold] * R_exec` | `O(N * (U_stddev + 2U_breakout))` |
| `Kqsr` | `P` | `P * R[kdj_period, srsi_period, srsi_smooth, ift_rsi_period, qqe_period, oversold, overbought, prb_period, prb_threshold] * R_exec` | roughly `O(N * (4F + 4S))` in the current GPU staging |
| `Carm` | `P` | `P * R[coppock_long_period, coppock_short_period, coppock_wma_period, ao_fast_period, ao_slow_period, rocr_period, apo_fast_period, apo_slow_period, otto_period, otto_threshold] * R_exec` | effectively `O(N * P)` staging already, then `O(N * P)` eval |
| `Kvdf` | `P` | `P * R[keltner_period, keltner_mult, vwap_anchor, fvg_lookback, devstop_period, devstop_mult, dti_period, dti_threshold] * R_exec` | `O(N * (1 + F + S + U_dti))` |
| `Cdmc` | `P` | `P * R[dpo_period, correlation_cycle_period, mama_fast_limit, mama_slow_limit, decycler_period, phase_threshold] * R_exec` | `O(N * (U_dpo + 2U_cycle + U_decycler + 1))` |

### CompositeMo

`CompositeMo` is not a pure exhaustive GPU grid in the current product path. It uses its own sampler plus optional GPU prescan. So its useful complexity model is:

- conceptual exhaustive search space: `R_mo_total = Π_j |range_j|`
- actual runtime today: `T_mo = O(N * (P_prescan + M))`

Where:

- `P_prescan` is the optional GPU prescan work
- `M` is the actual number of sampled candidates the CompositeMo search procedure evaluates

So `CompositeMo` is not the best place to compare simple exhaustive GPU search versus a more specialized CUDA search directly, because the search algorithm is budgeted rather than full Cartesian enumeration.

### Optimization conclusion

If the search space stays close to `P` or `P * V`, CUDA is helpful but not automatically dominant. That is the current state for much of this codebase.

CUDA starts making much more sense when at least one of these is true:

- `P` is already very large because the fast/slow grid is wide
- strategy-local knobs are also ranged, so candidate count becomes `P * R_strategy`
- the strategy has multiple independent indicator sweep families, so staging and scoring can both stay massively parallel on device
- the strategy is validation-heavy and the same shortlisted candidates are rerun many times

The strongest optimization-side CUDA candidates under the full-range assumption are:

- `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`
- `Asr`, `Vemr`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`
- `CompositeCpu` if HalfTrend, MACZ, and OTT are all genuinely ranged

The weakest optimization-side CUDA case remains:

- plain `DoubleMa` or `Marrt` with narrow length ranges and no MA-param sweeps

## Validation

### General GPU validation model

Validation is a fixed-candidate rerun problem, not a full search-space problem. The right decomposition is:

`T_val(strategy) = O(N * (Precompute_val(strategy, K) + K)) + StageMultipliers`

Where `Precompute_val(strategy, K)` depends on the number of unique parameter values present in the `K` shortlisted candidates, not on the original optimization grid.

Backend caveat:

- double-like validation has an implemented GPU fixed-candidate path for `GpuKernel` and also accepts `GpuHybridKernel`, but can still fall back to CPU for unsupported features such as direct `cross_ticker`
- composite validation has an implemented GPU fixed-candidate path for `GpuKernel`, but not for `GpuHybridKernel`; unsupported backend/feature combinations fall back to CPU

### Shared stage multipliers

These apply to both `double_ma_validate` and `composite_validate`.

| Stage | Asymptotic cost |
| --- | --- |
| Base fixed-candidate evaluation | `O(N * (Precompute_val + K))` |
| Optional iterative rerun | `+ O(N * K)` |
| Walk-forward | `+ O(N * K * W)` |
| Reopt walk-forward | `+ O(N * Fw * Rw)` |
| CSCV slice prep | `+ O(N * K * Z)` |
| CSCV combinatorics | `+ O(Comb(Z) * K * Z)` |
| SPA slice prep | `+ O(N * K * Z)` |
| SPA bootstrap | `+ O(B * K * Z)` |
| Perturbation | `+ O(N * K * Ppert)` |
| Trade bootstrap | `+ O(K * Q * T)` |
| Cross-ticker | multiply rerun cost by `D` |

Two route-specific notes matter:

- double-like cross-ticker can forward more of the heavy validation stack in preserved-finalists mode
- composite cross-ticker currently reruns base-only validation on extra datasets

### Validation routes by strategy

Double-like validation route:

- `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`, `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`

Composite validation route:

- `CompositeCpu`, `CompositeMo`, `Asr`, `Vcc`, `Vemr`, `Rocb`, `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`

### Per-strategy base validation cost

All rows below assume a GPU-executed fixed-candidate validation path. `U_*` means unique values present across the shortlisted candidate set.

| Strategy | Validation route | Base GPU precompute term | Base validation cost |
| --- | --- | --- | --- |
| `DoubleMa` | double-like | `U_fast + U_slow + U_kaufman_stop` | `O(N * (U_fast + U_slow + U_kaufman_stop + K))` |
| `Pit` | double-like | `U_fast + U_slow + U_efi + U_vosc + U_cfo + U_kaufman_stop` | `O(N * (... + K))` |
| `Set` | double-like | `U_fast + U_slow + U_linearreg + U_er + U_chop + U_kaufman_stop` | `O(N * (... + K))` |
| `Mts` | double-like | `U_fast + U_slow + U_roc + U_mom + U_ppo + U_kaufman_stop` | `O(N * (... + K))` |
| `Orc` | double-like | `U_fast + U_slow + U_rsi + U_stoch + U_willr + U_kaufman_stop` | `O(N * (... + K))` |
| `Marrt` | double-like | `U_fast + U_slow + U_kaufman_stop` | `O(N * (U_fast + U_slow + U_kaufman_stop + K))` |
| `Vfct` | double-like | `U_fast + U_slow + U_obv + U_mfi + U_kaufman_stop` | `O(N * (... + K))` |
| `CompositeTrend` | double-like | `U_fast + U_slow + U_linearreg + U_bbw + U_roc + U_adx + U_kaufman_stop` | `O(N * (... + K))` |
| `RegimeHybrid` | double-like | `U_fast + U_slow + U_trend_quality + U_adx + U_range_state + U_kaufman_stop` | `O(N * (... + K))` |
| `BreakoutMomentum` | double-like | `U_fast + U_slow + U_bb + U_macd + U_cmo + U_adosc + U_adx + U_kaufman_stop` | `O(N * (... + K))` |
| `CompositeCpu` | composite | `U_fast + U_slow + U_halftrend + U_macz + U_ott` | `O(N * (U_fast + U_slow + U_halftrend + U_macz + U_ott + K))` |
| `CompositeMo` | composite | same base-composite families as `CompositeCpu` for fixed-candidate validation | `O(N * (U_fast + U_slow + U_halftrend + U_macz + U_ott + K))` |
| `Asr` | composite | squeeze + angle + ER + z-score + Kaufman-stop unique sweeps | `O(N * (U_asr_signals + K))` |
| `Vcc` | composite | squeeze + momentum + trend unique sweeps | `O(N * (U_vcc_signals + K))` |
| `Vemr` | composite | z-score + Bollinger + ATR unique sweeps | `O(N * (U_vemr_signals + K))` |
| `Rocb` | composite | Donchian + ATR unique sweeps | `O(N * (U_rocb_signals + K))` |
| `Mkcs` | composite | stddev + Donchian breakout unique sweeps | `O(N * (U_mkcs_signals + K))` |
| `Kqsr` | composite | KDJ + SRSI + IFT-RSI + QQE + percentile-range unique sweeps | `O(N * (U_kqsr_signals + K))` |
| `Carm` | composite | Coppock + AO + ROCR + APO + OTTO unique sweeps | `O(N * (U_carm_signals + K))` |
| `Kvdf` | composite | VWAP + FVG + Keltner + DevStop + DTI unique sweeps | `O(N * (U_kvdf_signals + K))` |
| `Cdmc` | composite | DPO + correlation-cycle + MAMA/FAMA + decycler unique sweeps | `O(N * (U_cdmc_signals + K))` |

### Validation-side conclusion

CUDA becomes easier to justify in validation than in optimization once any of these are true:

- `K` is moderately large and validation enables walk-forward, CSCV, SPA, or perturbation
- the same candidate set is rerun across many datasets (`D`)
- the strategy has expensive signal families that can be swept once and reused across many reruns

The strongest validation-side CUDA cases are:

- any strategy with `K * W` or `K * Z` large
- `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`
- `Asr`, `Vemr`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`

The weakest validation-side CUDA case remains:

- a tiny shortlist of simple `DoubleMa` candidates with base-only validation and no cross-ticker reruns

## Final answer to the motivating question

If you keep the current product search spaces mostly fixed except for fast/slow lengths, then plain GPU brute force can continue to beat or match a more elaborate CUDA optimizer for simple strategies like `DoubleMa`.

A CUDA optimization algorithm starts to make clear sense when at least one of the following becomes true:

- you unfreeze the strategy-local knobs and the candidate count becomes `P * R_strategy`
- you move to indicator-rich strategies whose GPU staging terms are still linear in `N` but whose candidate-product is much larger
- you care about validation throughput, where one optimized shortlist is rerun across windows, slices, bootstraps, and datasets

In other words:

- **today's product reality** mostly favors simple GPU brute force for simple double-MA style searches
- **the full-range hypothetical you asked for** strongly favors CUDA for the richer strategy families and for validation-heavy workflows

## Final Verdict
Pick MKCS.
                                                                                                                     
  It is the best current target if you want to experiment with a CUDA optimization algorithm in this repo because it 
  has:                                                                                                               
                                                                                                                     
  - a dedicated GPU optimization path in composite_cpu.rs:4760 and composite_cpu.rs:4824                             
  - the strongest end-to-end GPU validation coverage, including parity tests for base metrics, walk-forward, CSCV,   
    SPA, reopt walk-forward, trade bootstrap, and perturbation in composite_validate.rs:17878                        
  - more structural complexity than plain DoubleMa, so there is more room for a smarter CUDA search algorithm to beat
    simple exhaustive GPU sweep                                                                                      
                                                                                                                     
  I would rank the practical choices like this:                                                                      
                                                                                                                     
  1. Mkcs: best balance of full GPU support, moderate complexity, and good validation coverage.                      
  2. Rocb or Vemr: good second choices if you want something a bit simpler than Mkcs.                                
  3. Asr: good if you want a richer strategy, but it has more moving parts and a higher debugging burden.            
                                                                                                                     
  I would avoid these as the first target:                                                                           
                                                                                                                     
  - DoubleMa: too simple; the existing exhaustive GPU path is already a hard baseline to beat.                       
  - CompositeTrend, RegimeHybrid, BreakoutMomentum: intended GPU policy exists, but optimization still routes into   
    CPU profile evaluators rather than a true end-to-end GPU optimization path in composite_cpu.rs:10589.            
  - Kqsr, Carm, Kvdf, Cdmc: GPU-backed, but the validation coverage is less complete than Mkcs and the signal stacks 
    are more cumbersome for first-pass algorithm work.      