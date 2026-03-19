use std::collections::HashMap;

use ta_optimizer::OptimizationResult;
use ta_strategies::double_ma::{DoubleMaParams, Metrics};

use crate::double_ma::{
    BackendOptimizationResult, BackendUsed, DoubleMaParamsResolved, GpuResidencySummary,
    HybridTimingMs, IndicatorRoutePlanSummary, OptimizationModeResolved, VdpsTelemetry,
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
    pub backend_route: Option<String>,
    pub backend_fallback_reason: Option<String>,
    pub indicator_route_plan: Option<IndicatorRoutePlanSummary>,
    pub residency: Option<GpuResidencySummary>,
    pub followup_cache_key: Option<String>,
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

    BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used: ctx.backend_used,
        matrix_index,
        best_col,
        top_cols,
        num_combos: ctx.num_combos,
        num_candles: ctx.num_candles,
        runtime_ms: ctx.runtime_ms,
        top,
        all,
        heatmap: result.heatmap,
        vdps: ctx.vdps,
        hybrid_timing_ms: ctx.hybrid_timing_ms,
        backend_route: ctx.backend_route,
        backend_fallback_reason: ctx.backend_fallback_reason,
        indicator_route_plan: ctx.indicator_route_plan,
        residency: ctx.residency,
        followup_cache_key: ctx.followup_cache_key,
    }
}
