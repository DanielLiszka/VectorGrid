use ta_strategies::double_ma::StrategyConfig;

fn require_positive_raw_u16(name: &str, value: u16) -> Result<(), String> {
    if value == 0 {
        Err(format!("{name} must be > 0"))
    } else {
        Ok(())
    }
}

fn require_positive_finite(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        Err(format!("{name} must be finite and > 0"))
    } else {
        Ok(())
    }
}

// This module owns boundary validation for caller-supplied configs. Reject
// invalid raw fields here; `StrategyConfig` accessors still own internal
// normalization/repair for already-admitted configs and older artifacts.
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
        require_positive_raw_u16("kaufman_stop_period", cfg.kaufman_stop_period)?;
        require_positive_finite("kaufman_stop_mult", cfg.kaufman_stop_mult)?;
        if cfg.kaufman_stop_ma_type().trim().is_empty() {
            return Err("kaufman_stop_ma_type cannot be empty".to_string());
        }
    }

    if cfg.uses_trend_quality_filter() || cfg.uses_regime_hybrid_filter() {
        require_positive_raw_u16(
            "trend_quality_linearreg_period",
            cfg.trend_quality_linearreg_period,
        )?;
        require_positive_finite(
            "trend_quality_angle_min_deg",
            cfg.trend_quality_angle_min_deg,
        )?;
        require_positive_raw_u16("trend_quality_bbw_period", cfg.trend_quality_bbw_period)?;
        require_positive_finite("trend_quality_bbw_mult", cfg.trend_quality_bbw_mult)?;
        let bbw_min = cfg.trend_quality_bbw_min;
        if !bbw_min.is_finite() || bbw_min < 0.0 {
            return Err("trend_quality_bbw_min must be finite and >= 0".to_string());
        }
        if cfg.trend_quality_use_roc && cfg.trend_quality_roc_period == 0 {
            return Err(
                "trend_quality_roc_period must be > 0 when ROC confirmation is enabled".to_string(),
            );
        }
        if cfg.trend_quality_use_adx {
            require_positive_raw_u16("trend_quality_adx_period", cfg.trend_quality_adx_period)?;
            require_positive_finite("trend_quality_adx_min", cfg.trend_quality_adx_min)?;
        }
    }

    if cfg.uses_breakout_momentum_filter() {
        require_positive_raw_u16("breakout_bbands_period", cfg.breakout_bbands_period)?;
        require_positive_finite("breakout_bbands_mult", cfg.breakout_bbands_mult)?;
        require_positive_raw_u16("breakout_confirm_bars", cfg.breakout_confirm_bars)?;
        let macd_fast = cfg.breakout_macd_fast_period;
        let macd_slow = cfg.breakout_macd_slow_period;
        let macd_signal = cfg.breakout_macd_signal_period;
        if macd_fast == 0 || macd_slow == 0 || macd_signal == 0 {
            return Err("breakout MACD periods must be > 0 (fast/slow/signal)".to_string());
        }
        if macd_fast >= macd_slow {
            return Err("breakout MACD requires fast period < slow period".to_string());
        }
        require_positive_raw_u16("breakout_cmo_period", cfg.breakout_cmo_period)?;
        let cmo_min_abs = cfg.breakout_cmo_min_abs;
        if !cmo_min_abs.is_finite() || cmo_min_abs < 0.0 {
            return Err("breakout_cmo_min_abs must be finite and >= 0".to_string());
        }
        let adosc_short = cfg.breakout_adosc_short_period;
        let adosc_long = cfg.breakout_adosc_long_period;
        if adosc_short == 0 || adosc_long == 0 {
            return Err("breakout ADOSC periods must be > 0".to_string());
        }
        if adosc_short >= adosc_long {
            return Err("breakout ADOSC requires short period < long period".to_string());
        }
        if cfg.breakout_use_adx {
            require_positive_raw_u16("breakout_adx_period", cfg.breakout_adx_period)?;
            require_positive_finite("breakout_adx_min", cfg.breakout_adx_min)?;
        }
    }

    if cfg.uses_volume_flow_confirm_filter() {
        require_positive_raw_u16(
            "volume_flow_obv_smooth_period",
            cfg.volume_flow_obv_smooth_period,
        )?;
        require_positive_raw_u16("volume_flow_mfi_period", cfg.volume_flow_mfi_period)?;
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
        require_positive_raw_u16("volume_flow_confirm_bars", cfg.volume_flow_confirm_bars)?;
    }

    if cfg.uses_pressure_impulse_trend_filter() {
        require_positive_raw_u16("pit_efi_period", cfg.pit_efi_period)?;
        require_positive_raw_u16("pit_vosc_short_period", cfg.pit_vosc_short_period)?;
        require_positive_raw_u16("pit_vosc_long_period", cfg.pit_vosc_long_period)?;
        if cfg.pit_vosc_short_period >= cfg.pit_vosc_long_period {
            return Err("pit_vosc_short_period must be < pit_vosc_long_period".to_string());
        }
        require_positive_raw_u16("pit_cfo_period", cfg.pit_cfo_period)?;
        require_positive_finite("pit_cfo_scalar", cfg.pit_cfo_scalar)?;
        require_positive_raw_u16("pit_confirm_bars", cfg.pit_confirm_bars)?;
    }

    if cfg.uses_slope_efficiency_trend_filter() {
        require_positive_raw_u16("set_linearreg_period", cfg.set_linearreg_period)?;
        require_positive_finite("set_angle_min_deg", cfg.set_angle_min_deg)?;
        require_positive_raw_u16("set_er_period", cfg.set_er_period)?;
        let er_min = cfg.set_er_min();
        if !er_min.is_finite() || !(0.0..=1.0).contains(&er_min) {
            return Err("set_er_min must be finite and in [0, 1]".to_string());
        }
        require_positive_raw_u16("set_chop_period", cfg.set_chop_period)?;
        let chop_max = cfg.set_chop_max();
        if !chop_max.is_finite() || !(0.0..=100.0).contains(&chop_max) {
            return Err("set_chop_max must be finite and in [0, 100]".to_string());
        }
    }

    if cfg.uses_momentum_triple_stack_filter() {
        require_positive_raw_u16("mts_roc_period", cfg.mts_roc_period)?;
        require_positive_raw_u16("mts_mom_period", cfg.mts_mom_period)?;
        require_positive_raw_u16("mts_ppo_fast_period", cfg.mts_ppo_fast_period)?;
        require_positive_raw_u16("mts_ppo_slow_period", cfg.mts_ppo_slow_period)?;
        if cfg.mts_ppo_fast_period >= cfg.mts_ppo_slow_period {
            return Err("mts_ppo_fast_period must be < mts_ppo_slow_period".to_string());
        }
    }

    if cfg.uses_oscillator_reversion_confluence_filter() {
        require_positive_raw_u16("orc_rsi_period", cfg.orc_rsi_period)?;
        require_positive_raw_u16("orc_stoch_fastk_period", cfg.orc_stoch_fastk_period)?;
        require_positive_raw_u16("orc_stoch_fastd_period", cfg.orc_stoch_fastd_period)?;
        require_positive_raw_u16("orc_willr_period", cfg.orc_willr_period)?;
    }

    if cfg.uses_regime_hybrid_filter() {
        require_positive_raw_u16("regime_hybrid_adx_period", cfg.regime_hybrid_adx_period)?;
        let adx_enter = cfg.regime_hybrid_adx_enter;
        if !adx_enter.is_finite() || adx_enter <= 0.0 {
            return Err("regime_hybrid_adx_enter must be finite and > 0".to_string());
        }
        let adx_exit = cfg.regime_hybrid_adx_exit;
        if !adx_exit.is_finite() || adx_exit <= 0.0 || adx_exit > adx_enter {
            return Err(
                "regime_hybrid_adx_exit must be finite, > 0, and <= regime_hybrid_adx_enter"
                    .to_string(),
            );
        }
        let rev_entry = cfg.regime_hybrid_reversion_entry_rel;
        if !rev_entry.is_finite() || rev_entry <= 0.0 {
            return Err("regime_hybrid_reversion_entry_rel must be finite and > 0".to_string());
        }
        let rev_exit = cfg.regime_hybrid_reversion_exit_rel;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ta_strategies::double_ma::ExitMode;

    #[test]
    fn validate_strategy_cfg_rejects_zero_kaufman_stop_period_raw_value() {
        let cfg = StrategyConfig {
            exit_mode: ExitMode::KaufmanStop,
            kaufman_stop_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero Kaufman stop period must fail");
        assert!(err.contains("kaufman_stop_period"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_kaufman_stop_multiplier_raw_value() {
        let cfg = StrategyConfig {
            exit_mode: ExitMode::KaufmanStop,
            kaufman_stop_mult: 0.0,
            ..StrategyConfig::default()
        };

        let err =
            validate_strategy_cfg(&cfg).expect_err("raw zero Kaufman stop multiplier must fail");
        assert!(err.contains("kaufman_stop_mult"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_trend_quality_linearreg_period_raw_value() {
        let cfg = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_linearreg_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero trend-quality period must fail");
        assert!(err.contains("trend_quality_linearreg_period"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_trend_quality_angle_raw_value() {
        let cfg = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_angle_min_deg: 0.0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero trend-quality angle must fail");
        assert!(err.contains("trend_quality_angle_min_deg"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_breakout_bbands_period_raw_value() {
        let cfg = StrategyConfig {
            breakout_momentum_enable: true,
            breakout_bbands_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero breakout period must fail");
        assert!(err.contains("breakout_bbands_period"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_volume_flow_period_raw_value() {
        let cfg = StrategyConfig {
            volume_flow_confirm_enable: true,
            volume_flow_obv_smooth_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero volume-flow period must fail");
        assert!(err.contains("volume_flow_obv_smooth_period"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_pit_efi_period_raw_value() {
        let cfg = StrategyConfig {
            pressure_impulse_trend_enable: true,
            pit_efi_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero PIT EFI period must fail");
        assert!(err.contains("pit_efi_period"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_set_linearreg_period_raw_value() {
        let cfg = StrategyConfig {
            slope_efficiency_trend_enable: true,
            set_linearreg_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero SET period must fail");
        assert!(err.contains("set_linearreg_period"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_mts_roc_period_raw_value() {
        let cfg = StrategyConfig {
            momentum_triple_stack_enable: true,
            mts_roc_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero MTS period must fail");
        assert!(err.contains("mts_roc_period"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_orc_rsi_period_raw_value() {
        let cfg = StrategyConfig {
            oscillator_reversion_confluence_enable: true,
            orc_rsi_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero ORC period must fail");
        assert!(err.contains("orc_rsi_period"));
    }

    #[test]
    fn validate_strategy_cfg_rejects_zero_regime_hybrid_adx_period_raw_value() {
        let cfg = StrategyConfig {
            regime_hybrid_enable: true,
            regime_hybrid_adx_period: 0,
            ..StrategyConfig::default()
        };

        let err = validate_strategy_cfg(&cfg).expect_err("raw zero regime-hybrid period must fail");
        assert!(err.contains("regime_hybrid_adx_period"));
    }

    #[test]
    fn strategy_accessors_still_repair_zero_values_after_boundary_rejection() {
        let cfg = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_linearreg_period: 0,
            ..StrategyConfig::default()
        };

        assert!(validate_strategy_cfg(&cfg).is_err());
        assert_eq!(cfg.trend_quality_linearreg_period(), 1);
    }
}
