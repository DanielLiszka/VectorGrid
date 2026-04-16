//! Test-only forwarding helpers for `double_ma.rs`.
//!
//! Canonical preset definitions remain owned by `strategy_presets.rs`; this
//! module only keeps large local tests from importing that module repeatedly.

use ta_strategies::double_ma::StrategyConfig;

pub(super) fn trend_quality_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::trend_quality_strategy_for_kernel()
}

pub(super) fn regime_hybrid_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::regime_hybrid_strategy_for_kernel()
}

pub(super) fn breakout_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::breakout_strategy_for_kernel()
}

pub(super) fn vfct_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::vfct_strategy_for_kernel()
}

pub(super) fn marrt_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::marrt_strategy_for_kernel()
}

pub(super) fn pit_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::pit_strategy_for_kernel()
}

pub(super) fn set_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::set_strategy_for_kernel()
}

pub(super) fn mts_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::mts_strategy_for_kernel()
}

pub(super) fn orc_strategy_for_kernel() -> StrategyConfig {
    crate::strategy_presets::orc_strategy_for_kernel()
}

pub(super) fn double_like_kernel_strategy_cases() -> Vec<(&'static str, StrategyConfig)> {
    crate::strategy_presets::double_like_kernel_strategy_cases()
}
