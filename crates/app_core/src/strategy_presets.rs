use ta_strategies::double_ma::StrategyConfig;

pub fn trend_quality_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.trend_quality_enable = true;
    s.trend_quality_use_adx = false;
    s
}

pub fn regime_hybrid_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.regime_hybrid_enable = true;
    s
}

pub fn breakout_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.breakout_momentum_enable = true;
    s.breakout_use_adx = false;
    s.breakout_bbands_mult = 0.8;
    s.breakout_cmo_min_abs = 0.0;
    s.breakout_macd_fast_period = 5;
    s.breakout_macd_slow_period = 13;
    s.breakout_macd_signal_period = 4;
    s.breakout_adosc_short_period = 2;
    s.breakout_adosc_long_period = 4;
    s
}

pub fn vfct_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.volume_flow_confirm_enable = true;
    s.volume_flow_obv_smooth_period = 8;
    s.volume_flow_mfi_period = 10;
    s.volume_flow_mfi_long_min = 52.0;
    s.volume_flow_mfi_short_max = 48.0;
    s.volume_flow_confirm_bars = 1;
    s
}

pub fn marrt_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s
}

pub fn pit_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.pressure_impulse_trend_enable = true;
    s.pit_efi_period = 8;
    s.pit_vosc_short_period = 4;
    s.pit_vosc_long_period = 9;
    s.pit_cfo_period = 6;
    s.pit_cfo_scalar = 100.0;
    s.pit_confirm_bars = 2;
    s
}

pub fn set_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.slope_efficiency_trend_enable = true;
    s.set_linearreg_period = 16;
    s.set_angle_min_deg = 0.15;
    s.set_er_period = 10;
    s.set_er_min = 0.02;
    s.set_chop_period = 10;
    s.set_chop_max = 90.0;
    s
}

pub fn mts_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.momentum_triple_stack_enable = true;
    s.mts_roc_period = 6;
    s.mts_mom_period = 10;
    s.mts_ppo_fast_period = 4;
    s.mts_ppo_slow_period = 9;
    s
}

pub fn orc_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.oscillator_reversion_confluence_enable = true;
    s.orc_rsi_period = 5;
    s.orc_stoch_fastk_period = 5;
    s.orc_stoch_fastd_period = 3;
    s.orc_willr_period = 5;
    s
}

pub fn double_like_kernel_strategy_cases() -> Vec<(&'static str, StrategyConfig)> {
    vec![
        ("composite_trend", trend_quality_strategy_for_kernel()),
        ("regime_hybrid", regime_hybrid_strategy_for_kernel()),
        ("breakout_momentum", breakout_strategy_for_kernel()),
        ("pit", pit_strategy_for_kernel()),
        ("set", set_strategy_for_kernel()),
        ("mts", mts_strategy_for_kernel()),
        ("orc", orc_strategy_for_kernel()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy_dynamic::compile_dynamic_plan_for_double_ma_config;
    use crate::strategy_ir::{
        build_static_strategy_ir, IndicatorKind, StaticStrategyIrInput, StrategyProfileKind,
    };
    use crate::strategy_plan::{compile_strategy_plan, IndicatorRouteKind, StrategyPlanBackend};
    use std::collections::HashSet;

    #[test]
    fn double_like_kernel_strategy_cases_have_unique_non_empty_labels() {
        let cases = double_like_kernel_strategy_cases();
        let mut seen = HashSet::new();

        assert!(!cases.is_empty());
        for (label, _) in cases {
            assert!(!label.trim().is_empty());
            assert!(
                seen.insert(label),
                "duplicate preset label found in double-like kernel cases: {label}"
            );
        }
    }

    #[test]
    fn regime_hybrid_strategy_for_kernel_keeps_expected_default_backed_thresholds() {
        let strategy = regime_hybrid_strategy_for_kernel();

        assert!(!strategy.long_only);
        assert!(strategy.regime_hybrid_enable);
        assert_eq!(strategy.regime_hybrid_adx_period, 14);
        assert_eq!(strategy.regime_hybrid_adx_enter, 22.0);
        assert_eq!(strategy.regime_hybrid_adx_exit, 18.0);
        assert_eq!(strategy.regime_hybrid_reversion_entry_rel, 0.0035);
        assert_eq!(strategy.regime_hybrid_reversion_exit_rel, 0.0012);
    }

    #[test]
    fn vfct_strategy_for_kernel_smoke_covers_direct_preset_surface() {
        let strategy = vfct_strategy_for_kernel();

        assert!(!strategy.long_only);
        assert!(strategy.volume_flow_confirm_enable);
        assert_eq!(strategy.volume_flow_obv_smooth_period, 8);
        assert_eq!(strategy.volume_flow_mfi_period, 10);
        assert_eq!(strategy.volume_flow_mfi_long_min, 52.0);
        assert_eq!(strategy.volume_flow_mfi_short_max, 48.0);
        assert_eq!(strategy.volume_flow_confirm_bars, 1);
    }

    #[test]
    fn marrt_strategy_for_kernel_smoke_preserves_thin_default_shape() {
        let strategy = marrt_strategy_for_kernel();

        assert!(!strategy.long_only);
        assert!(strategy.allow_flip);
        assert!(strategy.trade_on_next_bar);
        assert_eq!(
            strategy.exit_mode,
            ta_strategies::double_ma::ExitMode::Cross
        );
        assert!(!strategy.regime_hybrid_enable);
        assert!(!strategy.breakout_momentum_enable);
    }

    #[test]
    fn double_like_kernel_strategy_cases_compile_with_gpu_kernel_ma_routes() {
        for (label, strategy) in double_like_kernel_strategy_cases() {
            let plan = compile_dynamic_plan_for_double_ma_config(
                "close",
                "sma",
                "alma",
                None,
                None,
                &strategy,
                None,
                StrategyPlanBackend::GpuUnified,
            )
            .unwrap_or_else(|err| panic!("preset '{label}' failed to compile: {err}"));

            assert_eq!(
                plan.backend,
                StrategyPlanBackend::GpuUnified,
                "preset '{label}' should keep the requested GPU backend"
            );
            assert!(
                plan.signals.long_entry_group.is_some() || plan.signals.short_entry_group.is_some(),
                "preset '{label}' should expose at least one entry binding"
            );

            let fast_ma = plan
                .indicator_plan
                .iter()
                .find(|indicator| indicator.indicator_id == "fast_ma")
                .unwrap_or_else(|| panic!("preset '{label}' missing fast_ma slot"));
            let slow_ma = plan
                .indicator_plan
                .iter()
                .find(|indicator| indicator.indicator_id == "slow_ma")
                .unwrap_or_else(|| panic!("preset '{label}' missing slow_ma slot"));

            assert_eq!(
                fast_ma.route,
                IndicatorRouteKind::GpuKernel,
                "preset '{label}' should keep fast_ma on the kernel route"
            );
            assert_eq!(
                slow_ma.route,
                IndicatorRouteKind::GpuKernel,
                "preset '{label}' should keep slow_ma on the kernel route"
            );
        }
    }

    #[test]
    fn representative_composite_presets_compile_to_expected_gpu_indicator_routes() {
        let cases = [
            (
                "composite_trend",
                StrategyProfileKind::CompositeTrend,
                trend_quality_strategy_for_kernel(),
                IndicatorKind::TrendQualityRegime,
            ),
            (
                "regime_hybrid",
                StrategyProfileKind::RegimeHybrid,
                regime_hybrid_strategy_for_kernel(),
                IndicatorKind::TrendQualityRegime,
            ),
            (
                "breakout_momentum",
                StrategyProfileKind::BreakoutMomentum,
                breakout_strategy_for_kernel(),
                IndicatorKind::BreakoutRegime,
            ),
        ];

        for (label, profile, strategy, regime_kind) in cases {
            let ir = build_static_strategy_ir(
                profile,
                StaticStrategyIrInput {
                    ma_source: "close".to_string(),
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "alma".to_string(),
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy,
                    halftrend_filter: None,
                    halftrend: None,
                    macz: None,
                    ott: None,
                    signal_mode: None,
                },
            )
            .unwrap_or_else(|err| panic!("failed to build preset ir for '{label}': {err}"));
            let plan = compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None)
                .unwrap_or_else(|err| panic!("failed to compile preset plan for '{label}': {err}"));

            let fast_ma = plan
                .indicator_plan
                .iter()
                .find(|indicator| indicator.indicator_kind == Some(IndicatorKind::FastMa))
                .unwrap_or_else(|| panic!("preset '{label}' missing FastMa route"));
            let slow_ma = plan
                .indicator_plan
                .iter()
                .find(|indicator| indicator.indicator_kind == Some(IndicatorKind::SlowMa))
                .unwrap_or_else(|| panic!("preset '{label}' missing SlowMa route"));
            let regime = plan
                .indicator_plan
                .iter()
                .find(|indicator| indicator.indicator_kind == Some(regime_kind))
                .unwrap_or_else(|| panic!("preset '{label}' missing regime route"));

            assert_eq!(fast_ma.route, IndicatorRouteKind::GpuKernel);
            assert_eq!(slow_ma.route, IndicatorRouteKind::GpuKernel);
            assert_eq!(regime.route, IndicatorRouteKind::GpuKernel);
        }
    }
}
