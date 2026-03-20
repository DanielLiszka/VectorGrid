use ta_optimizer::ObjectiveKind;
use ta_strategies::double_ma::StrategyConfig;

use crate::double_ma::{
    kernel_regime_filters_enabled, Backend, GpuExecutionPolicy, IndicatorRouteDecisionSummary,
    IndicatorRoutePlanSummary,
};
use crate::strategy_gpu_runtime::StrategyCapabilityReport;

#[cfg(feature = "cuda")]
use vector_ta::cuda::cuda_available;

#[inline]
pub(crate) fn backend_kind_label(backend: &Backend) -> &'static str {
    match backend {
        Backend::Auto { .. } => "Auto",
        Backend::CpuOnly => "CpuOnly",
        Backend::GpuOnly { .. } => "GpuOnly",
        Backend::GpuHybridKernel { .. } => "GpuHybridKernel",
        Backend::GpuKernel { .. } => "GpuKernel",
    }
}

#[inline]
pub(crate) fn hybrid_backend_enabled() -> bool {
    match std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL") {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false"))
        }
        Err(_) => true,
    }
}

fn gpu_kernel_topk_supported(top_k: usize, include_all: bool) -> bool {
    if include_all {
        return true;
    }
    let topk_select = if top_k == 0 { 1 } else { top_k };
    topk_select <= 256
}

pub(crate) fn gpu_kernel_blockers_for_request(
    fast_ma: &str,
    slow_ma: &str,
    strategy: &StrategyConfig,
    regime_filter_present: bool,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if strategy.uses_kaufman_stop_exit() {
        blockers.push("kaufman-stop exit is not supported by GPU kernel path".to_string());
    }
    if strategy.uses_execution_overlays() {
        blockers.push(
            "execution overlays (cooldown/persistence) are not supported by GPU kernel path"
                .to_string(),
        );
    }
    if regime_filter_present && !kernel_regime_filters_enabled() {
        blockers
            .push("regime filter provided while kernel regime filters are disabled".to_string());
    }
    if matches!(objective, ObjectiveKind::Sortino | ObjectiveKind::Omega) {
        blockers.push(format!(
            "objective {objective:?} is not supported by GPU kernel path"
        ));
    }
    if !gpu_kernel_topk_supported(top_k, include_all) {
        blockers.push(
            "GPU kernel backend top_k too large (max 256 when include_all=false)".to_string(),
        );
    }
    #[cfg(feature = "cuda")]
    {
        if !cuda_available() {
            blockers.push("CUDA runtime is unavailable".to_string());
        }
    }
    #[cfg(not(feature = "cuda"))]
    {
        blockers.push("CUDA feature is not enabled in this build".to_string());
    }
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        if !crate::vram_ma::supports_vram_kernel_ma(fast_ma) {
            blockers.push(format!(
                "fast MA '{fast_ma}' is not supported by GPU kernel path"
            ));
        }
        if !crate::vram_ma::supports_vram_kernel_ma(slow_ma) {
            blockers.push(format!(
                "slow MA '{slow_ma}' is not supported by GPU kernel path"
            ));
        }
    }
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    {
        let _ = (fast_ma, slow_ma);
        blockers.push("GPU kernel backend is not enabled in this build".to_string());
    }
    blockers
}

#[allow(dead_code)]
pub(crate) fn build_strategy_capability_report_for_request(
    fast_ma: &str,
    slow_ma: &str,
    strategy: &StrategyConfig,
    regime_filter_present: bool,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
) -> StrategyCapabilityReport {
    let reasons = gpu_kernel_blockers_for_request(
        fast_ma,
        slow_ma,
        strategy,
        regime_filter_present,
        objective,
        top_k,
        include_all,
    );
    let gpu_supported = reasons.is_empty();
    StrategyCapabilityReport {
        gpu_static_supported: gpu_supported,
        gpu_dynamic_supported: gpu_supported,
        cpu_dynamic_supported: true,
        reasons,
    }
}

fn map_hybrid_capability_label(cap: crate::ma_policy::HybridCapabilityTier) -> &'static str {
    match cap {
        crate::ma_policy::HybridCapabilityTier::GpuKernel => "gpu_kernel",
        crate::ma_policy::HybridCapabilityTier::CudaOnly => "cuda_only",
        crate::ma_policy::HybridCapabilityTier::CpuOnly => "cpu_only",
        crate::ma_policy::HybridCapabilityTier::Unsupported => "unsupported",
    }
}

fn map_hybrid_route_label(route: crate::ma_policy::HybridRouteTier) -> &'static str {
    match route {
        crate::ma_policy::HybridRouteTier::GpuKernel => "gpu_kernel",
        crate::ma_policy::HybridRouteTier::CpuStaged => "cpu_staged",
        crate::ma_policy::HybridRouteTier::Unsupported => "unsupported",
    }
}

fn indicator_staging_reason(
    decision: &crate::ma_policy::HybridIndicatorRouteDecision,
) -> Option<&'static str> {
    match decision.route {
        crate::ma_policy::HybridRouteTier::GpuKernel => None,
        crate::ma_policy::HybridRouteTier::CpuStaged => match decision.capability {
            crate::ma_policy::HybridCapabilityTier::GpuKernel => {
                if decision.gpu_preferred_by_speedup {
                    None
                } else {
                    Some("cpu_preferred_by_speedup")
                }
            }
            crate::ma_policy::HybridCapabilityTier::CudaOnly => {
                Some("kernel_unsupported_cuda_only")
            }
            crate::ma_policy::HybridCapabilityTier::CpuOnly => Some("kernel_unsupported_cpu_only"),
            crate::ma_policy::HybridCapabilityTier::Unsupported => Some("unsupported"),
        },
        crate::ma_policy::HybridRouteTier::Unsupported => Some("unsupported"),
    }
}

pub(crate) fn should_cpu_stage_indicator(
    decision: &crate::ma_policy::HybridIndicatorRouteDecision,
    gpu_execution: GpuExecutionPolicy,
) -> bool {
    match decision.route {
        crate::ma_policy::HybridRouteTier::GpuKernel => false,
        crate::ma_policy::HybridRouteTier::CpuStaged => match decision.capability {
            crate::ma_policy::HybridCapabilityTier::GpuKernel => !matches!(
                gpu_execution,
                GpuExecutionPolicy::PreferGpuResident | GpuExecutionPolicy::StrictGpuResident
            ),
            crate::ma_policy::HybridCapabilityTier::CudaOnly
            | crate::ma_policy::HybridCapabilityTier::CpuOnly
            | crate::ma_policy::HybridCapabilityTier::Unsupported => true,
        },
        crate::ma_policy::HybridRouteTier::Unsupported => true,
    }
}

pub(crate) fn build_indicator_route_plan_summary(
    fast_ma: &str,
    slow_ma: &str,
    gpu_execution: GpuExecutionPolicy,
) -> IndicatorRoutePlanSummary {
    let plan = crate::ma_policy::plan_double_ma_hybrid_route(fast_ma, slow_ma);
    let fast_cpu_staged = should_cpu_stage_indicator(&plan.fast, gpu_execution);
    let slow_cpu_staged = should_cpu_stage_indicator(&plan.slow, gpu_execution);
    let fast = IndicatorRouteDecisionSummary {
        indicator_id: plan.fast.indicator_id.clone(),
        capability: map_hybrid_capability_label(plan.fast.capability).to_string(),
        route: map_hybrid_route_label(plan.fast.route).to_string(),
        gpu_preferred_by_speedup: plan.fast.gpu_preferred_by_speedup,
        residency_tier: if fast_cpu_staged {
            "cpu_staged".to_string()
        } else {
            "gpu_kernel".to_string()
        },
        staging_reason: fast_cpu_staged
            .then(|| indicator_staging_reason(&plan.fast))
            .flatten()
            .map(str::to_string),
    };
    let slow = IndicatorRouteDecisionSummary {
        indicator_id: plan.slow.indicator_id.clone(),
        capability: map_hybrid_capability_label(plan.slow.capability).to_string(),
        route: map_hybrid_route_label(plan.slow.route).to_string(),
        gpu_preferred_by_speedup: plan.slow.gpu_preferred_by_speedup,
        residency_tier: if slow_cpu_staged {
            "cpu_staged".to_string()
        } else {
            "gpu_kernel".to_string()
        },
        staging_reason: slow_cpu_staged
            .then(|| indicator_staging_reason(&plan.slow))
            .flatten()
            .map(str::to_string),
    };
    IndicatorRoutePlanSummary { fast, slow }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ta_strategies::double_ma::ExitMode;

    #[test]
    fn capability_report_marks_gpu_supported_for_plain_sma_request() {
        let report = build_strategy_capability_report_for_request(
            "sma",
            "sma",
            &StrategyConfig::default(),
            false,
            ObjectiveKind::Sharpe,
            50,
            false,
        );
        assert!(report.gpu_static_supported);
        assert!(report.gpu_dynamic_supported);
        assert!(report.cpu_dynamic_supported);
        assert!(report.reasons.is_empty());
    }

    #[test]
    fn capability_report_keeps_cpu_dynamic_when_gpu_is_blocked() {
        let report = build_strategy_capability_report_for_request(
            "alma",
            "alma",
            &StrategyConfig::default(),
            false,
            ObjectiveKind::Omega,
            50,
            false,
        );
        assert!(!report.gpu_static_supported);
        assert!(!report.gpu_dynamic_supported);
        assert!(report.cpu_dynamic_supported);
        assert!(!report.reasons.is_empty());
    }

    #[test]
    fn capability_report_blocks_kaufman_stop_for_gpu_dynamic_but_keeps_cpu_dynamic() {
        let mut strategy = StrategyConfig::default();
        strategy.exit_mode = ExitMode::KaufmanStop;

        let report = build_strategy_capability_report_for_request(
            "alma",
            "alma",
            &strategy,
            false,
            ObjectiveKind::Sharpe,
            50,
            false,
        );

        assert!(!report.gpu_static_supported);
        assert!(!report.gpu_dynamic_supported);
        assert!(report.cpu_dynamic_supported);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("kaufman-stop exit")));
    }

    #[test]
    fn capability_report_blocks_execution_overlays_for_gpu_dynamic_but_keeps_cpu_dynamic() {
        let mut strategy = StrategyConfig::default();
        strategy.cooldown_bars = 2;

        let report = build_strategy_capability_report_for_request(
            "alma",
            "alma",
            &strategy,
            false,
            ObjectiveKind::Sharpe,
            50,
            false,
        );

        assert!(!report.gpu_static_supported);
        assert!(!report.gpu_dynamic_supported);
        assert!(report.cpu_dynamic_supported);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("execution overlays")));
    }
}
