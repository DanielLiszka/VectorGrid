use ta_optimizer::OptimizationMode;

use crate::double_ma::{GpuExecutionPolicy, OptimizationModeResolved};

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

pub(crate) fn resolve_optimization_mode(ctx: &ModeSelectionContext<'_>) -> OptimizationModeResolved {
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

            let large_search = ctx.total_combos > 500_000;
            let gpu_route_viable = ctx.gpu_kernel_viable || ctx.gpu_hybrid_viable;
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
    use super::{resolve_optimization_mode, validate_optimization_mode, ModeSelectionContext};
    use crate::double_ma::{GpuExecutionPolicy, OptimizationModeResolved};
    use ta_optimizer::OptimizationMode;

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
        let mut ctx = base_ctx();
        ctx.gpu_execution = GpuExecutionPolicy::StrictGpuResident;
        ctx.gpu_hybrid_viable = true;
        assert!(matches!(
            resolve_optimization_mode(&ctx),
            OptimizationModeResolved::GpuVdps
        ));
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
        assert!(
            validate_optimization_mode(&OptimizationModeResolved::GpuVdps, false, 64, None)
                .is_err()
        );
    }
}
