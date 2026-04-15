//! Composite-profile policy classification for the Double-MA family.
//!
//! This module classifies the intended composite profile and GPU target policy
//! from a [`StrategyConfig`]. It does not own realized per-stage backend
//! execution, runtime fallback behavior, or validation routing.
//!
//! Precedence is fixed and intentional:
//! `BreakoutMomentum > RegimeHybrid > CompositeTrend > CompositeCpu`.

use crate::strategy_ir::StrategyProfileKind;
use ta_strategies::double_ma::StrategyConfig;

/// Intended GPU target policy for the selected composite profile.
///
/// This is a target-policy classification, not a report of the backend route
/// ultimately realized by downstream execution stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompositeProfileGpuTarget {
    CpuOnly,
    GpuKernel,
    GpuHybridKernel,
}

/// Classifies the composite profile implied by the current strategy flags.
///
/// The precedence is fixed and preserved exactly:
/// `BreakoutMomentum > RegimeHybrid > CompositeTrend > CompositeCpu`.
pub(crate) fn composite_profile_from_strategy(strategy: &StrategyConfig) -> StrategyProfileKind {
    if strategy.breakout_momentum_enable {
        StrategyProfileKind::BreakoutMomentum
    } else if strategy.regime_hybrid_enable {
        StrategyProfileKind::RegimeHybrid
    } else if strategy.trend_quality_enable {
        StrategyProfileKind::CompositeTrend
    } else {
        StrategyProfileKind::CompositeCpu
    }
}

#[cfg_attr(not(feature = "cuda"), allow(dead_code))]
/// Returns the stable runtime-family label for the classified composite profile.
///
/// This label is part of the current product-facing route vocabulary. It names
/// the runtime family, not the realized backend execution path for a specific
/// stage.
pub(crate) fn composite_profile_runtime_label(strategy: &StrategyConfig) -> &'static str {
    match composite_profile_from_strategy(strategy) {
        StrategyProfileKind::CompositeCpu => "CompositeCpu",
        StrategyProfileKind::CompositeTrend => "CompositeTrendCpu",
        StrategyProfileKind::RegimeHybrid => "RegimeHybridCpu",
        StrategyProfileKind::BreakoutMomentum => "BreakoutMomentumCpu",
        StrategyProfileKind::DoubleMa => "CompositeCpu",
    }
}

/// Returns the string label for the intended GPU target policy.
///
/// This is the label form of [`composite_profile_gpu_target`], used to expose
/// target-policy classification without changing the underlying string
/// contract.
pub(crate) fn composite_profile_gpu_target_label(strategy: &StrategyConfig) -> &'static str {
    match composite_profile_gpu_target(strategy) {
        CompositeProfileGpuTarget::CpuOnly => "CpuOnly",
        CompositeProfileGpuTarget::GpuKernel => "GpuKernel",
        CompositeProfileGpuTarget::GpuHybridKernel => "GpuHybridKernel",
    }
}

/// Returns the intended GPU target policy for the classified composite profile.
///
/// This policy describes the preferred target family for the current strategy
/// shape. It does not guarantee that downstream stages will realize that exact
/// backend route at runtime.
pub(crate) fn composite_profile_gpu_target(strategy: &StrategyConfig) -> CompositeProfileGpuTarget {
    match composite_profile_from_strategy(strategy) {
        StrategyProfileKind::CompositeTrend => {
            if strategy.trend_quality_use_adx() {
                CompositeProfileGpuTarget::GpuHybridKernel
            } else {
                CompositeProfileGpuTarget::GpuKernel
            }
        }
        StrategyProfileKind::BreakoutMomentum => {
            if strategy.breakout_use_adx() {
                CompositeProfileGpuTarget::GpuHybridKernel
            } else {
                CompositeProfileGpuTarget::GpuKernel
            }
        }
        StrategyProfileKind::RegimeHybrid => CompositeProfileGpuTarget::GpuHybridKernel,
        StrategyProfileKind::CompositeCpu | StrategyProfileKind::DoubleMa => {
            CompositeProfileGpuTarget::CpuOnly
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composite_trend_without_adx_targets_full_gpu() {
        let mut strategy = StrategyConfig::default();
        strategy.trend_quality_enable = true;
        strategy.trend_quality_use_adx = false;
        assert_eq!(
            composite_profile_from_strategy(&strategy),
            StrategyProfileKind::CompositeTrend
        );
        assert_eq!(
            composite_profile_gpu_target(&strategy),
            CompositeProfileGpuTarget::GpuKernel
        );
        assert_eq!(
            composite_profile_runtime_label(&strategy),
            "CompositeTrendCpu"
        );
    }

    #[test]
    fn composite_trend_with_adx_targets_hybrid() {
        let mut strategy = StrategyConfig::default();
        strategy.trend_quality_enable = true;
        strategy.trend_quality_use_adx = true;
        assert_eq!(
            composite_profile_gpu_target(&strategy),
            CompositeProfileGpuTarget::GpuHybridKernel
        );
    }

    #[test]
    fn breakout_without_adx_targets_full_gpu() {
        let mut strategy = StrategyConfig::default();
        strategy.breakout_momentum_enable = true;
        strategy.breakout_use_adx = false;
        assert_eq!(
            composite_profile_from_strategy(&strategy),
            StrategyProfileKind::BreakoutMomentum
        );
        assert_eq!(
            composite_profile_gpu_target(&strategy),
            CompositeProfileGpuTarget::GpuKernel
        );
        assert_eq!(
            composite_profile_runtime_label(&strategy),
            "BreakoutMomentumCpu"
        );
    }

    #[test]
    fn breakout_with_adx_targets_hybrid() {
        let mut strategy = StrategyConfig::default();
        strategy.breakout_momentum_enable = true;
        strategy.breakout_use_adx = true;
        assert_eq!(
            composite_profile_gpu_target(&strategy),
            CompositeProfileGpuTarget::GpuHybridKernel
        );
    }

    #[test]
    fn regime_hybrid_targets_hybrid() {
        let mut strategy = StrategyConfig::default();
        strategy.trend_quality_enable = true;
        strategy.regime_hybrid_enable = true;
        assert_eq!(
            composite_profile_from_strategy(&strategy),
            StrategyProfileKind::RegimeHybrid
        );
        assert_eq!(
            composite_profile_gpu_target(&strategy),
            CompositeProfileGpuTarget::GpuHybridKernel
        );
        assert_eq!(
            composite_profile_runtime_label(&strategy),
            "RegimeHybridCpu"
        );
    }

    #[test]
    fn breakout_precedence_wins_over_regime_and_trend() {
        let mut strategy = StrategyConfig::default();
        strategy.trend_quality_enable = true;
        strategy.regime_hybrid_enable = true;
        strategy.breakout_momentum_enable = true;
        assert_eq!(
            composite_profile_from_strategy(&strategy),
            StrategyProfileKind::BreakoutMomentum
        );
    }

    #[test]
    fn regime_precedence_wins_over_trend() {
        let mut strategy = StrategyConfig::default();
        strategy.trend_quality_enable = true;
        strategy.regime_hybrid_enable = true;
        assert_eq!(
            composite_profile_from_strategy(&strategy),
            StrategyProfileKind::RegimeHybrid
        );
    }

    #[test]
    fn default_strategy_stays_on_composite_cpu_route() {
        let strategy = StrategyConfig::default();
        assert_eq!(
            composite_profile_from_strategy(&strategy),
            StrategyProfileKind::CompositeCpu
        );
        assert_eq!(
            composite_profile_gpu_target(&strategy),
            CompositeProfileGpuTarget::CpuOnly
        );
        assert_eq!(composite_profile_runtime_label(&strategy), "CompositeCpu");
        assert_eq!(composite_profile_gpu_target_label(&strategy), "CpuOnly");
    }

    #[test]
    fn composite_profile_target_labels_match_current_product_route_contract() {
        let trend_kernel = {
            let mut strategy = StrategyConfig::default();
            strategy.trend_quality_enable = true;
            strategy.trend_quality_use_adx = false;
            strategy
        };
        let trend_hybrid = {
            let mut strategy = StrategyConfig::default();
            strategy.trend_quality_enable = true;
            strategy.trend_quality_use_adx = true;
            strategy
        };
        let breakout_kernel = {
            let mut strategy = StrategyConfig::default();
            strategy.breakout_momentum_enable = true;
            strategy.breakout_use_adx = false;
            strategy
        };
        let breakout_hybrid = {
            let mut strategy = StrategyConfig::default();
            strategy.breakout_momentum_enable = true;
            strategy.breakout_use_adx = true;
            strategy
        };
        let regime_hybrid = {
            let mut strategy = StrategyConfig::default();
            strategy.regime_hybrid_enable = true;
            strategy
        };

        assert_eq!(
            composite_profile_gpu_target_label(&trend_kernel),
            "GpuKernel"
        );
        assert_eq!(
            composite_profile_gpu_target_label(&trend_hybrid),
            "GpuHybridKernel"
        );
        assert_eq!(
            composite_profile_gpu_target_label(&breakout_kernel),
            "GpuKernel"
        );
        assert_eq!(
            composite_profile_gpu_target_label(&breakout_hybrid),
            "GpuHybridKernel"
        );
        assert_eq!(
            composite_profile_gpu_target_label(&regime_hybrid),
            "GpuHybridKernel"
        );
    }
}
