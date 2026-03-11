use crate::strategy_ir::StrategyProfileKind;
use ta_strategies::double_ma::StrategyConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompositeProfileGpuTarget {
    CpuOnly,
    GpuKernel,
    GpuHybridKernel,
}

pub(crate) fn composite_profile_from_strategy(
    strategy: &StrategyConfig,
) -> StrategyProfileKind {
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
pub(crate) fn composite_profile_runtime_label(
    strategy: &StrategyConfig,
) -> &'static str {
    match composite_profile_from_strategy(strategy) {
        StrategyProfileKind::CompositeCpu => "CompositeCpu",
        StrategyProfileKind::CompositeTrend => "CompositeTrendCpu",
        StrategyProfileKind::RegimeHybrid => "RegimeHybridCpu",
        StrategyProfileKind::BreakoutMomentum => "BreakoutMomentumCpu",
        StrategyProfileKind::DoubleMa => "CompositeCpu",
    }
}

pub(crate) fn composite_profile_gpu_target_label(
    strategy: &StrategyConfig,
) -> &'static str {
    match composite_profile_gpu_target(strategy) {
        CompositeProfileGpuTarget::CpuOnly => "CpuOnly",
        CompositeProfileGpuTarget::GpuKernel => "GpuKernel",
        CompositeProfileGpuTarget::GpuHybridKernel => "GpuHybridKernel",
    }
}

pub(crate) fn composite_profile_gpu_target(
    strategy: &StrategyConfig,
) -> CompositeProfileGpuTarget {
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
        assert_eq!(composite_profile_runtime_label(&strategy), "CompositeTrendCpu");
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
        assert_eq!(composite_profile_runtime_label(&strategy), "RegimeHybridCpu");
    }
}
