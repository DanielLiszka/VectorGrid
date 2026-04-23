use ta_optimizer::OptimizationMode;

use crate::double_ma::{GpuExecutionPolicy, OptimizationModeResolved};

#[cfg(test)]
static GPU_KERNEL_EXECUTION_AVAILABLE_OVERRIDE: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(0);

#[cfg(feature = "cuda")]
fn gpu_kernel_execution_available() -> bool {
    #[cfg(test)]
    match GPU_KERNEL_EXECUTION_AVAILABLE_OVERRIDE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => return false,
        2 => return true,
        _ => {}
    }
    vector_ta::cuda::cuda_available()
}

#[cfg(not(feature = "cuda"))]
fn gpu_kernel_execution_available() -> bool {
    #[cfg(test)]
    match GPU_KERNEL_EXECUTION_AVAILABLE_OVERRIDE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => return false,
        2 => return true,
        _ => {}
    }
    false
}

const AUTO_MODE_LARGE_SEARCH_COMBO_THRESHOLD: usize = 500_000;

pub(crate) struct ModeSelectionContext<'a> {
    pub requested_mode: OptimizationMode,
    pub include_all: bool,
    pub heatmap_bins: usize,
    pub export_all_csv_path: Option<&'a str>,
    pub total_combos: usize,
    pub gpu_execution: GpuExecutionPolicy,
    pub gpu_kernel_viable: bool,
    pub gpu_hybrid_viable: bool,
}

pub(crate) fn resolve_optimization_mode(
    ctx: &ModeSelectionContext<'_>,
) -> OptimizationModeResolved {
    match ctx.requested_mode {
        OptimizationMode::Grid => OptimizationModeResolved::Grid,
        OptimizationMode::CoarseToFine => OptimizationModeResolved::CoarseToFine,
        OptimizationMode::GpuVdps => OptimizationModeResolved::GpuVdps,
        OptimizationMode::Auto => {
            let exact_grid_outputs_requested =
                ctx.include_all || ctx.export_all_csv_path.is_some() || ctx.heatmap_bins > 0;
            if exact_grid_outputs_requested {
                return OptimizationModeResolved::Grid;
            }

            let large_search = ctx.total_combos > AUTO_MODE_LARGE_SEARCH_COMBO_THRESHOLD;
            let gpu_route_viable = (ctx.gpu_kernel_viable && gpu_kernel_execution_available())
                || ctx.gpu_hybrid_viable;
            if large_search
                && gpu_route_viable
                && matches!(
                    ctx.gpu_execution,
                    GpuExecutionPolicy::PreferGpuResident | GpuExecutionPolicy::StrictGpuResident
                )
            {
                return OptimizationModeResolved::GpuVdps;
            }

            if large_search {
                OptimizationModeResolved::CoarseToFine
            } else {
                OptimizationModeResolved::Grid
            }
        }
    }
}

pub(crate) fn validate_optimization_mode(
    mode_used: &OptimizationModeResolved,
    include_all: bool,
    heatmap_bins: usize,
    export_all_csv_path: Option<&str>,
) -> Result<(), String> {
    if matches!(mode_used, OptimizationModeResolved::CoarseToFine) {
        if include_all {
            return Err("CoarseToFine mode requires include_all=false".to_string());
        }
        if heatmap_bins > 0 {
            return Err("CoarseToFine mode currently requires heatmap_bins=0".to_string());
        }
        if export_all_csv_path.is_some() {
            return Err("CoarseToFine mode cannot be used with export_all_csv_path".to_string());
        }
    }
    if matches!(mode_used, OptimizationModeResolved::GpuVdps) {
        if include_all {
            return Err("GpuVdps mode requires include_all=false".to_string());
        }
        if heatmap_bins > 0 {
            return Err("GpuVdps mode currently requires heatmap_bins=0".to_string());
        }
        if export_all_csv_path.is_some() {
            return Err("GpuVdps mode does not support export_all_csv_path".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_optimization_mode, validate_optimization_mode, ModeSelectionContext,
        AUTO_MODE_LARGE_SEARCH_COMBO_THRESHOLD,
    };
    use crate::double_ma::{GpuExecutionPolicy, OptimizationModeResolved};
    use ta_optimizer::OptimizationMode;

    struct GpuKernelExecutionAvailabilityGuard;

    impl Drop for GpuKernelExecutionAvailabilityGuard {
        fn drop(&mut self) {
            super::GPU_KERNEL_EXECUTION_AVAILABLE_OVERRIDE
                .store(0, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn force_gpu_kernel_execution_available(enabled: bool) -> GpuKernelExecutionAvailabilityGuard {
        super::GPU_KERNEL_EXECUTION_AVAILABLE_OVERRIDE.store(
            if enabled { 2 } else { 1 },
            std::sync::atomic::Ordering::Relaxed,
        );
        GpuKernelExecutionAvailabilityGuard
    }

    fn base_ctx() -> ModeSelectionContext<'static> {
        ModeSelectionContext {
            requested_mode: OptimizationMode::Auto,
            include_all: false,
            heatmap_bins: 0,
            export_all_csv_path: None,
            total_combos: 750_000,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            gpu_kernel_viable: false,
            gpu_hybrid_viable: false,
        }
    }

    #[test]
    fn auto_mode_only_adapts_when_grid_outputs_are_disabled() {
        let mut ctx = base_ctx();
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::CoarseToFine
        ));

        ctx.include_all = true;
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::Grid
        ));

        ctx.include_all = false;
        ctx.heatmap_bins = 64;
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::Grid
        ));
    }

    #[test]
    fn auto_mode_prefers_vdps_for_gpu_resident_large_runs_when_route_is_viable() {
        let _guard = force_gpu_kernel_execution_available(true);
        let mut ctx = base_ctx();
        ctx.gpu_execution = GpuExecutionPolicy::StrictGpuResident;
        ctx.gpu_hybrid_viable = true;
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::GpuVdps
        ));
    }

    #[test]
    fn auto_mode_resident_request_requires_executable_gpu_route_for_vdps() {
        let _guard = force_gpu_kernel_execution_available(false);
        let mut ctx = base_ctx();
        ctx.gpu_execution = GpuExecutionPolicy::StrictGpuResident;
        ctx.gpu_kernel_viable = true;
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::CoarseToFine
        ));
    }

    #[test]
    fn auto_mode_large_search_threshold_is_strictly_greater_than_500k() {
        let _guard = force_gpu_kernel_execution_available(true);
        let mut ctx = base_ctx();
        ctx.gpu_execution = GpuExecutionPolicy::StrictGpuResident;
        ctx.gpu_hybrid_viable = true;
        ctx.total_combos = AUTO_MODE_LARGE_SEARCH_COMBO_THRESHOLD;
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::Grid
        ));

        ctx.total_combos = AUTO_MODE_LARGE_SEARCH_COMBO_THRESHOLD + 1;
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::GpuVdps
        ));
    }

    #[test]
    fn auto_mode_best_effort_does_not_promote_to_vdps() {
        let _guard = force_gpu_kernel_execution_available(true);
        let mut ctx = base_ctx();
        ctx.gpu_execution = GpuExecutionPolicy::BestEffort;
        ctx.gpu_hybrid_viable = true;
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::CoarseToFine
        ));
    }

    #[test]
    fn mode_validation_rejects_export_all_csv_for_gpu_vdps() {
        let err = validate_optimization_mode(
            &OptimizationModeResolved::GpuVdps,
            false,
            0,
            Some("export.csv"),
        )
        .expect_err("GpuVdps export_all backstop must fail");
        assert!(err.contains("export_all_csv_path"));
    }

    #[test]
    fn mode_validation_rejects_incompatible_outputs() {
        assert!(validate_optimization_mode(
            &OptimizationModeResolved::CoarseToFine,
            false,
            0,
            None
        )
        .is_ok());
        assert!(
            validate_optimization_mode(&OptimizationModeResolved::CoarseToFine, true, 0, None)
                .is_err()
        );
        assert!(validate_optimization_mode(
            &OptimizationModeResolved::CoarseToFine,
            false,
            8,
            None
        )
        .is_err());
        assert!(validate_optimization_mode(
            &OptimizationModeResolved::CoarseToFine,
            false,
            0,
            Some("out.csv")
        )
        .is_err());
        assert!(
            validate_optimization_mode(&OptimizationModeResolved::GpuVdps, true, 0, None).is_err()
        );
        assert!(
            validate_optimization_mode(&OptimizationModeResolved::GpuVdps, false, 64, None)
                .is_err()
        );
        assert!(validate_optimization_mode(
            &OptimizationModeResolved::GpuVdps,
            false,
            0,
            Some("out.csv")
        )
        .is_err());
    }
}
