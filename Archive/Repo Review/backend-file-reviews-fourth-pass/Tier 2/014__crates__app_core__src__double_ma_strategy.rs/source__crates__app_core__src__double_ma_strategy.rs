use ta_strategies::double_ma::StrategyConfig;

pub(crate) fn validate_strategy_cfg(cfg: &StrategyConfig) -> Result<(), String> {
    if !cfg.trade_on_next_bar {
        return Err(
            "trade_on_next_bar=false is not permitted because same-bar execution produces optimistic and inaccurate benchmark results.".to_string(),
        );
    }

    let cost_fields = [cfg.fee_bps, cfg.spread_bps, cfg.slippage_bps];
    for (k, v) in ["fee_bps", "spread_bps", "slippage_bps"]
        .iter()
        .zip(cost_fields.iter())
    {
        if !v.is_finite() || *v < 0.0 {
            return Err(format!("{k} must be finite and >= 0"));
        }
    }
    let bps_commission = cfg.commission_from_bps();
    let has_bps_costs = cost_fields.iter().any(|v| *v > 0.0);
    if cfg.commission.is_finite()
        && cfg.commission > 0.0
        && has_bps_costs
        && (cfg.commission - bps_commission).abs() > 1e-12
    {
        return Err(
            "commission conflicts with fee_bps/spread_bps/slippage_bps; use one consistent effective commission.".to_string(),
        );
    }

    let effective_commission = cfg.effective_commission();
    if !effective_commission.is_finite()
        || effective_commission < 0.0
        || effective_commission >= 1.0
    {
        return Err("effective commission must be finite and in [0, 1)".to_string());
    }
    if !cfg.eps_rel.is_finite() || cfg.eps_rel < 0.0 {
        return Err("eps_rel must be finite and >= 0".to_string());
    }
    for (k, v) in [
        ("stop_loss_pct", cfg.stop_loss_pct),
        ("take_profit_pct", cfg.take_profit_pct),
    ] {
        if let Some(pct) = v {
            if !pct.is_finite() || pct <= 0.0 || pct >= 1.0 {
                return Err(format!("{k} must be finite and in (0, 1) when set"));
            }
        }
    }

    if cfg.uses_kaufman_stop_exit() {
        if cfg.kaufman_stop_period() == 0 {
            return Err("kaufman_stop_period must be > 0".to_string());
        }
        let mult = cfg.kaufman_stop_mult();
        if !mult.is_finite() || mult <= 0.0 {
            return Err("kaufman_stop_mult must be finite and > 0".to_string());
        }
        if cfg.kaufman_stop_ma_type().trim().is_empty() {
            return Err("kaufman_stop_ma_type cannot be empty".to_string());
        }
    }

    if cfg.uses_trend_quality_filter() || cfg.uses_regime_hybrid_filter() {
        if cfg.trend_quality_linearreg_period() == 0 {
            return Err("trend_quality_linearreg_period must be > 0".to_string());
        }
        let angle_min = cfg.trend_quality_angle_min_deg();
        if !angle_min.is_finite() || angle_min <= 0.0 {
            return Err("trend_quality_angle_min_deg must be finite and > 0".to_string());
        }
        if cfg.trend_quality_bbw_period() == 0 {
            return Err("trend_quality_bbw_period must be > 0".to_string());
        }
        let bbw_mult = cfg.trend_quality_bbw_mult();
        if !bbw_mult.is_finite() || bbw_mult <= 0.0 {
            return Err("trend_quality_bbw_mult must be finite and > 0".to_string());
        }
        let bbw_min = cfg.trend_quality_bbw_min();
        if !bbw_min.is_finite() || bbw_min < 0.0 {
            return Err("trend_quality_bbw_min must be finite and >= 0".to_string());
        }
        if cfg.trend_quality_use_roc() && cfg.trend_quality_roc_period() == 0 {
            return Err(
                "trend_quality_roc_period must be > 0 when ROC confirmation is enabled".to_string(),
            );
        }
        if cfg.trend_quality_use_adx() {
            if cfg.trend_quality_adx_period() == 0 {
                return Err(
                    "trend_quality_adx_period must be > 0 when ADX confirmation is enabled"
                        .to_string(),
                );
            }
            let adx_min = cfg.trend_quality_adx_min();
            if !adx_min.is_finite() || adx_min <= 0.0 {
                return Err(
                    "trend_quality_adx_min must be finite and > 0 when ADX confirmation is enabled"
                        .to_string(),
                );
            }
        }
    }

    if cfg.uses_breakout_momentum_filter() {
        if cfg.breakout_bbands_period() == 0 {
            return Err("breakout_bbands_period must be > 0".to_string());
        }
        let bb_mult = cfg.breakout_bbands_mult();
        if !bb_mult.is_finite() || bb_mult <= 0.0 {
            return Err("breakout_bbands_mult must be finite and > 0".to_string());
        }
        if cfg.breakout_confirm_bars() == 0 {
            return Err("breakout_confirm_bars must be > 0".to_string());
        }
        let macd_fast = cfg.breakout_macd_fast_period();
        let macd_slow = cfg.breakout_macd_slow_period();
        let macd_signal = cfg.breakout_macd_signal_period();
        if macd_fast == 0 || macd_slow == 0 || macd_signal == 0 {
            return Err("breakout MACD periods must be > 0 (fast/slow/signal)".to_string());
        }
        if macd_fast >= macd_slow {
            return Err("breakout MACD requires fast period < slow period".to_string());
        }
        if cfg.breakout_cmo_period() == 0 {
            return Err("breakout_cmo_period must be > 0".to_string());
        }
        let cmo_min_abs = cfg.breakout_cmo_min_abs();
        if !cmo_min_abs.is_finite() || cmo_min_abs < 0.0 {
            return Err("breakout_cmo_min_abs must be finite and >= 0".to_string());
        }
        let adosc_short = cfg.breakout_adosc_short_period();
        let adosc_long = cfg.breakout_adosc_long_period();
        if adosc_short == 0 || adosc_long == 0 {
            return Err("breakout ADOSC periods must be > 0".to_string());
        }
        if adosc_short >= adosc_long {
            return Err("breakout ADOSC requires short period < long period".to_string());
        }
        if cfg.breakout_use_adx() {
            if cfg.breakout_adx_period() == 0 {
                return Err(
                    "breakout_adx_period must be > 0 when breakout ADX is enabled".to_string(),
                );
            }
            let adx_min = cfg.breakout_adx_min();
            if !adx_min.is_finite() || adx_min <= 0.0 {
                return Err(
                    "breakout_adx_min must be finite and > 0 when breakout ADX is enabled"
                        .to_string(),
                );
            }
        }
    }

    if cfg.uses_volume_flow_confirm_filter() {
        if cfg.volume_flow_obv_smooth_period() == 0 {
            return Err("volume_flow_obv_smooth_period must be > 0".to_string());
        }
        if cfg.volume_flow_mfi_period() == 0 {
            return Err("volume_flow_mfi_period must be > 0".to_string());
        }
        let long_min = cfg.volume_flow_mfi_long_min();
        if !long_min.is_finite() || !(0.0..=100.0).contains(&long_min) {
            return Err("volume_flow_mfi_long_min must be finite and in [0, 100]".to_string());
        }
        let short_max = cfg.volume_flow_mfi_short_max();
        if !short_max.is_finite() || !(0.0..=100.0).contains(&short_max) {
            return Err("volume_flow_mfi_short_max must be finite and in [0, 100]".to_string());
        }
        if short_max > long_min {
            return Err(
                "volume_flow_mfi_short_max must be <= volume_flow_mfi_long_min".to_string(),
            );
        }
        if cfg.volume_flow_confirm_bars() == 0 {
            return Err("volume_flow_confirm_bars must be > 0".to_string());
        }
    }

    if cfg.uses_pressure_impulse_trend_filter() {
        if cfg.pit_efi_period() == 0 {
            return Err("pit_efi_period must be > 0".to_string());
        }
        if cfg.pit_vosc_short_period() == 0 {
            return Err("pit_vosc_short_period must be > 0".to_string());
        }
        if cfg.pit_vosc_long_period() == 0 {
            return Err("pit_vosc_long_period must be > 0".to_string());
        }
        if cfg.pit_vosc_short_period() >= cfg.pit_vosc_long_period() {
            return Err("pit_vosc_short_period must be < pit_vosc_long_period".to_string());
        }
        if cfg.pit_cfo_period() == 0 {
            return Err("pit_cfo_period must be > 0".to_string());
        }
        let scalar = cfg.pit_cfo_scalar();
        if !scalar.is_finite() || scalar <= 0.0 {
            return Err("pit_cfo_scalar must be finite and > 0".to_string());
        }
        if cfg.pit_confirm_bars() == 0 {
            return Err("pit_confirm_bars must be > 0".to_string());
        }
    }

    if cfg.uses_slope_efficiency_trend_filter() {
        if cfg.set_linearreg_period() == 0 {
            return Err("set_linearreg_period must be > 0".to_string());
        }
        let angle_min = cfg.set_angle_min_deg();
        if !angle_min.is_finite() || angle_min <= 0.0 {
            return Err("set_angle_min_deg must be finite and > 0".to_string());
        }
        if cfg.set_er_period() == 0 {
            return Err("set_er_period must be > 0".to_string());
        }
        let er_min = cfg.set_er_min();
        if !er_min.is_finite() || !(0.0..=1.0).contains(&er_min) {
            return Err("set_er_min must be finite and in [0, 1]".to_string());
        }
        if cfg.set_chop_period() == 0 {
            return Err("set_chop_period must be > 0".to_string());
        }
        let chop_max = cfg.set_chop_max();
        if !chop_max.is_finite() || !(0.0..=100.0).contains(&chop_max) {
            return Err("set_chop_max must be finite and in [0, 100]".to_string());
        }
    }

    if cfg.uses_momentum_triple_stack_filter() {
        if cfg.mts_roc_period() == 0 {
            return Err("mts_roc_period must be > 0".to_string());
        }
        if cfg.mts_mom_period() == 0 {
            return Err("mts_mom_period must be > 0".to_string());
        }
        if cfg.mts_ppo_fast_period() == 0 {
            return Err("mts_ppo_fast_period must be > 0".to_string());
        }
        if cfg.mts_ppo_slow_period() == 0 {
            return Err("mts_ppo_slow_period must be > 0".to_string());
        }
        if cfg.mts_ppo_fast_period() >= cfg.mts_ppo_slow_period() {
            return Err("mts_ppo_fast_period must be < mts_ppo_slow_period".to_string());
        }
    }

    if cfg.uses_oscillator_reversion_confluence_filter() {
        if cfg.orc_rsi_period() == 0 {
            return Err("orc_rsi_period must be > 0".to_string());
        }
        if cfg.orc_stoch_fastk_period() == 0 {
            return Err("orc_stoch_fastk_period must be > 0".to_string());
        }
        if cfg.orc_stoch_fastd_period() == 0 {
            return Err("orc_stoch_fastd_period must be > 0".to_string());
        }
        if cfg.orc_willr_period() == 0 {
            return Err("orc_willr_period must be > 0".to_string());
        }
    }

    if cfg.uses_regime_hybrid_filter() {
        if cfg.regime_hybrid_adx_period() == 0 {
            return Err("regime_hybrid_adx_period must be > 0".to_string());
        }
        let adx_enter = cfg.regime_hybrid_adx_enter();
        if !adx_enter.is_finite() || adx_enter <= 0.0 {
            return Err("regime_hybrid_adx_enter must be finite and > 0".to_string());
        }
        let adx_exit = cfg.regime_hybrid_adx_exit();
        if !adx_exit.is_finite() || adx_exit <= 0.0 || adx_exit > adx_enter {
            return Err(
                "regime_hybrid_adx_exit must be finite, > 0, and <= regime_hybrid_adx_enter"
                    .to_string(),
            );
        }
        let rev_entry = cfg.regime_hybrid_reversion_entry_rel();
        if !rev_entry.is_finite() || rev_entry <= 0.0 {
            return Err("regime_hybrid_reversion_entry_rel must be finite and > 0".to_string());
        }
        let rev_exit = cfg.regime_hybrid_reversion_exit_rel();
        if !rev_exit.is_finite() || rev_exit < 0.0 || rev_exit > rev_entry {
            return Err(
                "regime_hybrid_reversion_exit_rel must be finite, >= 0, and <= regime_hybrid_reversion_entry_rel".to_string(),
            );
        }
    }
    if cfg.uses_regime_hybrid_filter() && cfg.uses_breakout_momentum_filter() {
        return Err(
            "regime_hybrid_enable is not compatible with breakout_momentum_enable in double-MA flow"
                .to_string(),
        );
    }
    if cfg.uses_breakout_momentum_filter()
        && (cfg.uses_trend_quality_filter()
            || cfg.uses_volume_flow_confirm_filter()
            || cfg.uses_pressure_impulse_trend_filter()
            || cfg.uses_slope_efficiency_trend_filter()
            || cfg.uses_momentum_triple_stack_filter()
            || cfg.uses_oscillator_reversion_confluence_filter())
    {
        return Err(
            "breakout_momentum_enable is not compatible with other double-MA regime profiles"
                .to_string(),
        );
    }
    if cfg.uses_volume_flow_confirm_filter()
        && (cfg.uses_trend_quality_filter()
            || cfg.uses_pressure_impulse_trend_filter()
            || cfg.uses_slope_efficiency_trend_filter()
            || cfg.uses_regime_hybrid_filter()
            || cfg.uses_breakout_momentum_filter())
    {
        return Err(
            "volume_flow_confirm_enable is not compatible with other double-MA regime profiles"
                .to_string(),
        );
    }
    if cfg.uses_pressure_impulse_trend_filter()
        && (cfg.uses_trend_quality_filter()
            || cfg.uses_slope_efficiency_trend_filter()
            || cfg.uses_momentum_triple_stack_filter()
            || cfg.uses_oscillator_reversion_confluence_filter()
            || cfg.uses_regime_hybrid_filter()
            || cfg.uses_breakout_momentum_filter()
            || cfg.uses_volume_flow_confirm_filter())
    {
        return Err(
            "pressure_impulse_trend_enable is not compatible with other double-MA regime profiles"
                .to_string(),
        );
    }
    if cfg.uses_slope_efficiency_trend_filter()
        && (cfg.uses_trend_quality_filter()
            || cfg.uses_pressure_impulse_trend_filter()
            || cfg.uses_momentum_triple_stack_filter()
            || cfg.uses_regime_hybrid_filter()
            || cfg.uses_breakout_momentum_filter()
            || cfg.uses_volume_flow_confirm_filter())
    {
        return Err(
            "slope_efficiency_trend_enable is not compatible with other double-MA regime profiles"
                .to_string(),
        );
    }
    if cfg.uses_momentum_triple_stack_filter()
        && (cfg.uses_trend_quality_filter()
            || cfg.uses_pressure_impulse_trend_filter()
            || cfg.uses_slope_efficiency_trend_filter()
            || cfg.uses_oscillator_reversion_confluence_filter()
            || cfg.uses_regime_hybrid_filter()
            || cfg.uses_breakout_momentum_filter()
            || cfg.uses_volume_flow_confirm_filter())
    {
        return Err(
            "momentum_triple_stack_enable is not compatible with other double-MA regime profiles"
                .to_string(),
        );
    }
    if cfg.uses_oscillator_reversion_confluence_filter()
        && (cfg.uses_trend_quality_filter()
            || cfg.uses_pressure_impulse_trend_filter()
            || cfg.uses_slope_efficiency_trend_filter()
            || cfg.uses_momentum_triple_stack_filter()
            || cfg.uses_regime_hybrid_filter()
            || cfg.uses_breakout_momentum_filter()
            || cfg.uses_volume_flow_confirm_filter())
    {
        return Err(
            "oscillator_reversion_confluence_enable is not compatible with other double-MA regime profiles"
                .to_string(),
        );
    }
    Ok(())
}

pub(crate) fn validate_strategy_cfg_for_optimization(cfg: &StrategyConfig) -> Result<(), String> {
    validate_strategy_cfg(cfg)
}

pub(crate) fn strategy_cfg_for_optimization(cfg: StrategyConfig) -> StrategyConfig {
    cfg
}
