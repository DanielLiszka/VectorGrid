use ta_optimizer::OptimizationMode;

use crate::double_ma::OptimizationModeResolved;

pub(crate) fn resolve_optimization_mode(
    requested_mode: OptimizationMode,
    include_all: bool,
    heatmap_bins: usize,
    export_all_csv_path: Option<&str>,
    total_combos: usize,
) -> OptimizationModeResolved {
    match requested_mode {
        OptimizationMode::Grid => OptimizationModeResolved::Grid,
        OptimizationMode::CoarseToFine => OptimizationModeResolved::CoarseToFine,
        OptimizationMode::GpuVdps => OptimizationModeResolved::GpuVdps,
        OptimizationMode::Auto => {
            let can_adapt = !include_all && export_all_csv_path.is_none() && heatmap_bins == 0;
            if can_adapt && total_combos > 500_000 {
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
    use super::{resolve_optimization_mode, validate_optimization_mode};
    use crate::double_ma::OptimizationModeResolved;
    use ta_optimizer::OptimizationMode;

    #[test]
    fn auto_mode_only_adapts_when_grid_outputs_are_disabled() {
        assert!(matches!(
            resolve_optimization_mode(OptimizationMode::Auto, false, 0, None, 750_000),
            OptimizationModeResolved::CoarseToFine
        ));
        assert!(matches!(
            resolve_optimization_mode(OptimizationMode::Auto, true, 0, None, 750_000),
            OptimizationModeResolved::Grid
        ));
        assert!(matches!(
            resolve_optimization_mode(OptimizationMode::Auto, false, 64, None, 750_000),
            OptimizationModeResolved::Grid
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
