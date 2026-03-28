use std::collections::HashMap;

use ta_optimizer::OptimizationResult;
use ta_strategies::double_ma::{DoubleMaParams, Metrics};

use crate::double_ma::{
    BackendOptimizationResult, BackendUsed, DoubleMaParamsResolved, GpuKernelTimingMs,
    GpuResidencySummary, HybridTimingMs, IndicatorRoutePlanSummary, OptimizationModeResolved,
    PairObservationScoreContextResolved, ShortlistSliceCandidateDiagnosticsResolved,
    ShortlistSliceDiagnostics, ShortlistSliceDiagnosticsResolved,
    ShortlistSliceModeDiagnosticsResolved, ShortlistSliceSnapshotResolved, VdpsTelemetry,
};
use crate::matrix_index::DoubleMaFastLtSlowIndex;

pub(crate) struct BackendResultContext<'a> {
    pub mode_used: OptimizationModeResolved,
    pub backend_used: BackendUsed,
    pub fast_periods: &'a [u16],
    pub slow_periods: &'a [u16],
    pub fast_ma_label: &'a str,
    pub slow_ma_label: &'a str,
    pub fast_param_variants: &'a [Option<HashMap<String, f64>>],
    pub slow_param_variants: &'a [Option<HashMap<String, f64>>],
    pub num_combos: usize,
    pub num_candles: usize,
    pub runtime_ms: u64,
    pub vdps: Option<VdpsTelemetry>,
    pub hybrid_timing_ms: Option<HybridTimingMs>,
    pub gpu_kernel_timing_ms: Option<GpuKernelTimingMs>,
    pub backend_route: Option<String>,
    pub backend_fallback_reason: Option<String>,
    pub route_diagnostics: Option<Vec<String>>,
    pub indicator_route_plan: Option<IndicatorRoutePlanSummary>,
    pub residency: Option<GpuResidencySummary>,
    pub followup_cache_key: Option<String>,
    pub shortlist_slice_diagnostics: Option<ShortlistSliceDiagnostics>,
    pub score_observation_context: Option<PairObservationScoreContextResolved>,
}

fn resolve_params(
    params: &DoubleMaParams,
    fast_ma_label: &str,
    slow_ma_label: &str,
    fast_param_variants: &[Option<HashMap<String, f64>>],
    slow_param_variants: &[Option<HashMap<String, f64>>],
) -> DoubleMaParamsResolved {
    let fast_ma_params = fast_param_variants
        .get(params.fast_ma_id as usize)
        .cloned()
        .unwrap_or(None);
    let slow_ma_params = slow_param_variants
        .get(params.slow_ma_id as usize)
        .cloned()
        .unwrap_or(None);

    DoubleMaParamsResolved {
        fast_len: params.fast_len,
        slow_len: params.slow_len,
        fast_ma_type: fast_ma_label.to_string(),
        slow_ma_type: slow_ma_label.to_string(),
        fast_ma_params,
        slow_ma_params,
    }
}

fn build_grid_matrix_index(
    mode_used: OptimizationModeResolved,
    fast_periods: &[u16],
    slow_periods: &[u16],
    best_params: &DoubleMaParamsResolved,
    top: &[(DoubleMaParamsResolved, Metrics)],
) -> (
    Option<DoubleMaFastLtSlowIndex>,
    Option<u32>,
    Option<Vec<u32>>,
) {
    if !matches!(mode_used, OptimizationModeResolved::Grid) {
        return (None, None, None);
    }

    match DoubleMaFastLtSlowIndex::new(fast_periods.to_vec(), slow_periods.to_vec()) {
        Ok(idx) => {
            let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
            let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
            for (params, _) in top {
                match idx.col_id(params.fast_len, params.slow_len) {
                    Some(col) => top_cols.push(col),
                    None => {
                        top_cols.clear();
                        break;
                    }
                }
            }
            let top_cols = if top_cols.is_empty() && !top.is_empty() {
                None
            } else {
                Some(top_cols)
            };
            (Some(idx), best_col, top_cols)
        }
        Err(_) => (None, None, None),
    }
}

pub(crate) fn build_backend_optimization_result(
    result: OptimizationResult,
    ctx: BackendResultContext<'_>,
) -> BackendOptimizationResult {
    let mode_used = ctx.mode_used.clone();
    let best_params = resolve_params(
        &result.best_params,
        ctx.fast_ma_label,
        ctx.slow_ma_label,
        ctx.fast_param_variants,
        ctx.slow_param_variants,
    );
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .into_iter()
        .map(|(params, metrics)| {
            (
                resolve_params(
                    &params,
                    ctx.fast_ma_label,
                    ctx.slow_ma_label,
                    ctx.fast_param_variants,
                    ctx.slow_param_variants,
                ),
                metrics,
            )
        })
        .collect();
    let all = result.all.map(|rows| {
        rows.into_iter()
            .map(|(params, metrics)| {
                (
                    resolve_params(
                        &params,
                        ctx.fast_ma_label,
                        ctx.slow_ma_label,
                        ctx.fast_param_variants,
                        ctx.slow_param_variants,
                    ),
                    metrics,
                )
            })
            .collect()
    });
    let (matrix_index, best_col, top_cols) = build_grid_matrix_index(
        mode_used.clone(),
        ctx.fast_periods,
        ctx.slow_periods,
        &best_params,
        &top,
    );
    let shortlist_slice_diagnostics = ctx.shortlist_slice_diagnostics.map(|diag| {
        let candidates = diag
            .candidates
            .into_iter()
            .map(|candidate| ShortlistSliceCandidateDiagnosticsResolved {
                params: resolve_params(
                    &candidate.params,
                    ctx.fast_ma_label,
                    ctx.slow_ma_label,
                    ctx.fast_param_variants,
                    ctx.slow_param_variants,
                ),
                full_metrics: candidate.full_metrics,
                halves: ShortlistSliceModeDiagnosticsResolved {
                    worst_slice_score: candidate.halves.worst_slice_score,
                    positive_slice_frac: candidate.halves.positive_slice_frac,
                    mean_slice_score: candidate.halves.mean_slice_score,
                    best_omission_label: candidate.halves.best_omission_label,
                    best_omission_score: candidate.halves.best_omission_score,
                    worst_omission_label: candidate.halves.worst_omission_label,
                    worst_omission_score: candidate.halves.worst_omission_score,
                    omission_score_spread: candidate.halves.omission_score_spread,
                    benchmark_relative: candidate.halves.benchmark_relative,
                    slices: candidate
                        .halves
                        .slices
                        .into_iter()
                        .map(|slice| ShortlistSliceSnapshotResolved {
                            label: slice.label,
                            start_bar: slice.start_bar,
                            end_bar: slice.end_bar,
                            score: slice.score,
                            metrics: slice.metrics,
                        })
                        .collect(),
                },
                quartiles: ShortlistSliceModeDiagnosticsResolved {
                    worst_slice_score: candidate.quartiles.worst_slice_score,
                    positive_slice_frac: candidate.quartiles.positive_slice_frac,
                    mean_slice_score: candidate.quartiles.mean_slice_score,
                    best_omission_label: candidate.quartiles.best_omission_label,
                    best_omission_score: candidate.quartiles.best_omission_score,
                    worst_omission_label: candidate.quartiles.worst_omission_label,
                    worst_omission_score: candidate.quartiles.worst_omission_score,
                    omission_score_spread: candidate.quartiles.omission_score_spread,
                    benchmark_relative: candidate.quartiles.benchmark_relative,
                    slices: candidate
                        .quartiles
                        .slices
                        .into_iter()
                        .map(|slice| ShortlistSliceSnapshotResolved {
                            label: slice.label,
                            start_bar: slice.start_bar,
                            end_bar: slice.end_bar,
                            score: slice.score,
                            metrics: slice.metrics,
                        })
                        .collect(),
                },
            })
            .collect();
        ShortlistSliceDiagnosticsResolved {
            ranking_mode: diag.ranking_mode,
            neighborhood_robustness: diag.neighborhood_robustness,
            candidates,
        }
    });

    BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        best_score: result.best_score,
        mode_used,
        backend_used: ctx.backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: ctx.num_combos,
        num_candles: ctx.num_candles,
        runtime_ms: ctx.runtime_ms,
        top,
        top_scores: result.top_scores,
        all,
        all_scores: result.all_scores,
        heatmap: result.heatmap,
        vdps: ctx.vdps,
        hybrid_timing_ms: ctx.hybrid_timing_ms,
        gpu_kernel_timing_ms: ctx.gpu_kernel_timing_ms,
        backend_route: ctx.backend_route,
        backend_fallback_reason: ctx.backend_fallback_reason,
        route_diagnostics: ctx.route_diagnostics,
        indicator_route_plan: ctx.indicator_route_plan,
        residency: ctx.residency,
        followup_cache_key: ctx.followup_cache_key,
        shortlist_slice_diagnostics,
        score_observation_context: ctx.score_observation_context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::double_ma::GpuExecutionPolicy;

    #[test]
    fn build_backend_result_preserves_score_observation_context() {
        let result = OptimizationResult {
            best_params: DoubleMaParams {
                fast_len: 5,
                slow_len: 20,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            best_metrics: Metrics::default(),
            best_score: None,
            top: Vec::new(),
            top_scores: None,
            all: None,
            all_scores: None,
            heatmap: None,
            num_combos: 1,
            num_candles: 200,
        };
        let expected = PairObservationScoreContextResolved {
            first_valid: 3,
            regime_first_valid: 7,
            eval_start: 10,
            eval_end: 200,
            total_bars: 200,
            trade_on_next_bar: true,
        };
        let built = build_backend_optimization_result(
            result,
            BackendResultContext {
                mode_used: OptimizationModeResolved::Grid,
                backend_used: BackendUsed::Gpu,
                fast_periods: &[5],
                slow_periods: &[20],
                fast_ma_label: "sma",
                slow_ma_label: "ema",
                fast_param_variants: &[None],
                slow_param_variants: &[None],
                num_combos: 1,
                num_candles: 200,
                runtime_ms: 1,
                vdps: None,
                hybrid_timing_ms: None,
                gpu_kernel_timing_ms: None,
                backend_route: Some("GpuKernel".to_string()),
                backend_fallback_reason: None,
                route_diagnostics: None,
                indicator_route_plan: None,
                residency: Some(GpuResidencySummary {
                    requested_policy: GpuExecutionPolicy::BestEffort,
                    requested_backend: "gpu_kernel".to_string(),
                    resolved_backend: "gpu_kernel".to_string(),
                    fully_gpu_resident: true,
                    cpu_staged_indicators: false,
                    cpu_fallback_used: false,
                    notes: Vec::new(),
                }),
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: Some(expected),
            },
        );

        assert_eq!(built.score_observation_context, Some(expected));
    }

    #[test]
    fn build_backend_result_preserves_gpu_kernel_timing() {
        let result = OptimizationResult {
            best_params: DoubleMaParams {
                fast_len: 5,
                slow_len: 20,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            best_metrics: Metrics::default(),
            best_score: None,
            top: Vec::new(),
            top_scores: None,
            all: None,
            all_scores: None,
            heatmap: None,
            num_combos: 1,
            num_candles: 200,
        };
        let expected = GpuKernelTimingMs {
            route_wall_ms: 17,
            gpu_indicator_prep_ms: Some(4),
            h2d_upload_ms: Some(2),
            ..Default::default()
        };
        let built = build_backend_optimization_result(
            result,
            BackendResultContext {
                mode_used: OptimizationModeResolved::Grid,
                backend_used: BackendUsed::Gpu,
                fast_periods: &[5],
                slow_periods: &[20],
                fast_ma_label: "sma",
                slow_ma_label: "ema",
                fast_param_variants: &[None],
                slow_param_variants: &[None],
                num_combos: 1,
                num_candles: 200,
                runtime_ms: 1,
                vdps: None,
                hybrid_timing_ms: None,
                gpu_kernel_timing_ms: Some(expected.clone()),
                backend_route: Some("GpuKernel".to_string()),
                backend_fallback_reason: None,
                route_diagnostics: None,
                indicator_route_plan: None,
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
            },
        );

        assert_eq!(built.gpu_kernel_timing_ms, Some(expected));
    }
}
