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
