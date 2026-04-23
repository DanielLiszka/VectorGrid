use std::collections::HashMap;

use ta_optimizer::OptimizationResult;
use ta_strategies::double_ma::{DoubleMaParams, Metrics};

use crate::double_ma::{
    BackendOptimizationResult, BackendUsed, DoubleMaParamsResolved, GpuKernelTimingMs,
    GpuResidencySummary, HybridTimingMs, IndicatorRoutePlanSummary, OptimizationModeResolved,
    OptimizationSearchSemanticsSummary, PairObservationScoreContextResolved,
    ShortlistSliceCandidateDiagnosticsResolved, ShortlistSliceDiagnostics,
    ShortlistSliceDiagnosticsResolved, ShortlistSliceModeDiagnosticsResolved,
    ShortlistSliceSnapshotResolved, ShortlistUnionTelemetry, VdpsTelemetry,
};
use crate::matrix_index::DoubleMaFastLtSlowIndex;

pub(crate) struct BackendResultContext<'a> {
    pub mode_used: OptimizationModeResolved,
    pub exhaustive_output_requested: bool,
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
    pub metric_surface_kind: Option<String>,
    pub shortlist_union_telemetry: Option<ShortlistUnionTelemetry>,
}

const EXACT_EXHAUSTIVE_WORKLOAD_FAMILY: &str = "double_ma_exact_grid_full_output";
const EXACT_REDUCED_OUTPUT_WORKLOAD_FAMILY: &str = "double_ma_exact_grid_reduced_output";
const COARSE_TO_FINE_WORKLOAD_FAMILY: &str = "double_ma_coarse_to_fine";
const GPU_VDPS_WORKLOAD_FAMILY: &str = "double_ma_gpu_vdps";
const EXACT_EXHAUSTIVE_CONTRACT_SUMMARY: &str =
    "Exact exhaustive search over the full requested candidate space.";
const EXACT_REDUCED_OUTPUT_CONTRACT_SUMMARY: &str =
    "Same exact candidate scores, same tie order, and same retained-output semantics on the retained candidate set.";
const HEURISTIC_REDUCED_OUTPUT_CONTRACT_SUMMARY: &str =
    "Heuristic reduced-output mode; audit with best-score regret, top-k overlap, exact winner rank, holdout regret, and speedup versus exact before promotion.";

fn build_optimization_search_semantics_summary(
    mode_used: &OptimizationModeResolved,
    exhaustive_output_requested: bool,
) -> OptimizationSearchSemanticsSummary {
    match mode_used {
        OptimizationModeResolved::Grid => {
            if exhaustive_output_requested {
                OptimizationSearchSemanticsSummary {
                    approx_mode: "exact_exhaustive".to_string(),
                    workload_family: EXACT_EXHAUSTIVE_WORKLOAD_FAMILY.to_string(),
                    contract_summary: EXACT_EXHAUSTIVE_CONTRACT_SUMMARY.to_string(),
                    declared_domain: None,
                    base_ranker: None,
                    rerank_operator: None,
                    heatmap_semantics: None,
                }
            } else {
                OptimizationSearchSemanticsSummary {
                    approx_mode: "exact_reduced_output".to_string(),
                    workload_family: EXACT_REDUCED_OUTPUT_WORKLOAD_FAMILY.to_string(),
                    contract_summary: EXACT_REDUCED_OUTPUT_CONTRACT_SUMMARY.to_string(),
                    declared_domain: None,
                    base_ranker: None,
                    rerank_operator: None,
                    heatmap_semantics: None,
                }
            }
        }
        OptimizationModeResolved::CoarseToFine => OptimizationSearchSemanticsSummary {
            approx_mode: "heuristic_reduced_output".to_string(),
            workload_family: COARSE_TO_FINE_WORKLOAD_FAMILY.to_string(),
            contract_summary: HEURISTIC_REDUCED_OUTPUT_CONTRACT_SUMMARY.to_string(),
            declared_domain: None,
            base_ranker: None,
            rerank_operator: None,
            heatmap_semantics: None,
        },
        OptimizationModeResolved::GpuVdps => OptimizationSearchSemanticsSummary {
            approx_mode: "heuristic_reduced_output".to_string(),
            workload_family: GPU_VDPS_WORKLOAD_FAMILY.to_string(),
            contract_summary: HEURISTIC_REDUCED_OUTPUT_CONTRACT_SUMMARY.to_string(),
            declared_domain: None,
            base_ranker: None,
            rerank_operator: None,
            heatmap_semantics: None,
        },
    }
}

fn debug_assert_result_alignment(result: &OptimizationResult, ctx: &BackendResultContext<'_>) {
    debug_assert_eq!(
        result.num_combos, ctx.num_combos,
        "backend result num_combos must align with context metadata"
    );
    debug_assert_eq!(
        result.num_candles, ctx.num_candles,
        "backend result num_candles must align with context metadata"
    );
    if let Some(top_scores) = result.top_scores.as_ref() {
        debug_assert_eq!(
            top_scores.len(),
            result.top.len(),
            "top_scores length must align with projected top rows"
        );
    }
    if let Some(all_scores) = result.all_scores.as_ref() {
        let all_rows = result
            .all
            .as_ref()
            .expect("all_scores must only be present when all rows are present");
        debug_assert_eq!(
            all_scores.len(),
            all_rows.len(),
            "all_scores length must align with projected all rows"
        );
    }
}

fn resolve_params(
    params: &DoubleMaParams,
    fast_ma_label: &str,
    slow_ma_label: &str,
    fast_param_variants: &[Option<HashMap<String, f64>>],
    slow_param_variants: &[Option<HashMap<String, f64>>],
) -> DoubleMaParamsResolved {
    debug_assert!(
        (params.fast_ma_id as usize) < fast_param_variants.len(),
        "fast_ma_id {} exceeds fast_param_variants length {}",
        params.fast_ma_id,
        fast_param_variants.len()
    );
    debug_assert!(
        (params.slow_ma_id as usize) < slow_param_variants.len(),
        "slow_ma_id {} exceeds slow_param_variants length {}",
        params.slow_ma_id,
        slow_param_variants.len()
    );

    // Preserve existing serialized compatibility for stale cached artifacts or
    // externally constructed params by collapsing out-of-range variant ids to
    // `None` instead of changing the wire shape.
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
            debug_assert!(
                best_col.is_some(),
                "grid-mode best params ({}, {}) must project into matrix axes",
                best_params.fast_len,
                best_params.slow_len
            );
            let mut top_cols: Vec<u32> = Vec::with_capacity(top.len());
            for (params, _) in top {
                match idx.col_id(params.fast_len, params.slow_len) {
                    Some(col) => top_cols.push(col),
                    None => {
                        // Keep this projection all-or-nothing: partial `top_cols`
                        // would desynchronize consumers that treat the vector as a
                        // positional overlay for `top`.
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
            debug_assert!(
                top_cols
                    .as_ref()
                    .map_or(true, |cols| cols.len() == top.len()),
                "top_cols must either project every top row or be omitted"
            );
            (Some(idx), best_col, top_cols)
        }
        Err(_) => (None, None, None),
    }
}

pub(crate) fn build_backend_optimization_result(
    result: OptimizationResult,
    ctx: BackendResultContext<'_>,
) -> BackendOptimizationResult {
    debug_assert_result_alignment(&result, &ctx);
    let mode_used = ctx.mode_used.clone();
    let search_semantics = build_optimization_search_semantics_summary(
        &ctx.mode_used,
        ctx.exhaustive_output_requested,
    );
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
        metric_surface_kind: ctx.metric_surface_kind,
        shortlist_union_telemetry: ctx.shortlist_union_telemetry,
        search_semantics: Some(search_semantics),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::double_ma::GpuExecutionPolicy;

    fn base_result() -> OptimizationResult {
        OptimizationResult {
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
        }
    }

    #[test]
    fn build_backend_result_preserves_score_observation_context() {
        let result = base_result();
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
                exhaustive_output_requested: false,
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
                metric_surface_kind: None,
                shortlist_union_telemetry: None,
            },
        );

        assert_eq!(built.score_observation_context, Some(expected));
    }

    #[test]
    fn build_backend_result_preserves_gpu_kernel_timing() {
        let result = base_result();
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
                exhaustive_output_requested: false,
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
                metric_surface_kind: None,
                shortlist_union_telemetry: None,
            },
        );

        assert_eq!(built.gpu_kernel_timing_ms, Some(expected));
    }

    #[test]
    fn build_backend_result_preserves_metric_surface_kind() {
        let result = base_result();
        let built = build_backend_optimization_result(
            result,
            BackendResultContext {
                mode_used: OptimizationModeResolved::Grid,
                exhaustive_output_requested: false,
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
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
                metric_surface_kind: Some("trade22".to_string()),
                shortlist_union_telemetry: None,
            },
        );

        assert_eq!(built.metric_surface_kind.as_deref(), Some("trade22"));
    }

    #[test]
    fn build_backend_result_labels_exact_grid_output_contracts() {
        let reduced = build_backend_optimization_result(
            base_result(),
            BackendResultContext {
                mode_used: OptimizationModeResolved::Grid,
                exhaustive_output_requested: false,
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
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
                metric_surface_kind: None,
                shortlist_union_telemetry: None,
            },
        );
        let exhaustive = build_backend_optimization_result(
            base_result(),
            BackendResultContext {
                mode_used: OptimizationModeResolved::Grid,
                exhaustive_output_requested: true,
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
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
                metric_surface_kind: None,
                shortlist_union_telemetry: None,
            },
        );

        assert_eq!(
            reduced
                .search_semantics
                .as_ref()
                .map(|value| value.approx_mode.as_str()),
            Some("exact_reduced_output")
        );
        assert_eq!(
            reduced
                .search_semantics
                .as_ref()
                .map(|value| value.workload_family.as_str()),
            Some(EXACT_REDUCED_OUTPUT_WORKLOAD_FAMILY)
        );
        assert_eq!(
            exhaustive
                .search_semantics
                .as_ref()
                .map(|value| value.approx_mode.as_str()),
            Some("exact_exhaustive")
        );
        assert_eq!(
            exhaustive
                .search_semantics
                .as_ref()
                .map(|value| value.workload_family.as_str()),
            Some(EXACT_EXHAUSTIVE_WORKLOAD_FAMILY)
        );
    }

    #[test]
    fn build_backend_result_labels_heuristic_modes_for_audit_workloads() {
        let coarse = build_backend_optimization_result(
            base_result(),
            BackendResultContext {
                mode_used: OptimizationModeResolved::CoarseToFine,
                exhaustive_output_requested: false,
                backend_used: BackendUsed::Cpu,
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
                backend_route: Some("CpuOnly".to_string()),
                backend_fallback_reason: None,
                route_diagnostics: None,
                indicator_route_plan: None,
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
                metric_surface_kind: None,
                shortlist_union_telemetry: None,
            },
        );
        let vdps = build_backend_optimization_result(
            base_result(),
            BackendResultContext {
                mode_used: OptimizationModeResolved::GpuVdps,
                exhaustive_output_requested: false,
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
                backend_route: Some("GpuVdps".to_string()),
                backend_fallback_reason: None,
                route_diagnostics: None,
                indicator_route_plan: None,
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
                metric_surface_kind: None,
                shortlist_union_telemetry: None,
            },
        );

        for (value, expected_workload) in [
            (coarse, COARSE_TO_FINE_WORKLOAD_FAMILY),
            (vdps, GPU_VDPS_WORKLOAD_FAMILY),
        ] {
            assert_eq!(
                value
                    .search_semantics
                    .as_ref()
                    .map(|entry| entry.approx_mode.as_str()),
                Some("heuristic_reduced_output")
            );
            assert_eq!(
                value
                    .search_semantics
                    .as_ref()
                    .map(|entry| entry.workload_family.as_str()),
                Some(expected_workload)
            );
            assert!(
                value
                    .search_semantics
                    .as_ref()
                    .is_some_and(|entry| entry.contract_summary.contains("best-score regret")),
                "expected heuristic audit contract summary"
            );
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "fast_ma_id")]
    fn resolve_params_debug_asserts_when_variant_ids_are_out_of_bounds() {
        let _ = resolve_params(
            &DoubleMaParams {
                fast_len: 8,
                slow_len: 34,
                fast_ma_id: 3,
                slow_ma_id: 4,
            },
            "alma",
            "ema",
            &[Some(HashMap::from([("offset".to_string(), 0.85)]))],
            &[Some(HashMap::from([("alpha".to_string(), 2.0)]))],
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn resolve_params_keeps_wire_shape_when_variant_ids_are_out_of_bounds() {
        let params = resolve_params(
            &DoubleMaParams {
                fast_len: 8,
                slow_len: 34,
                fast_ma_id: 3,
                slow_ma_id: 4,
            },
            "alma",
            "ema",
            &[Some(HashMap::from([("offset".to_string(), 0.85)]))],
            &[Some(HashMap::from([("alpha".to_string(), 2.0)]))],
        );

        assert_eq!(params.fast_len, 8);
        assert_eq!(params.slow_len, 34);
        assert_eq!(params.fast_ma_type, "alma");
        assert_eq!(params.slow_ma_type, "ema");
        assert_eq!(params.fast_ma_params, None);
        assert_eq!(params.slow_ma_params, None);
    }

    #[test]
    fn build_grid_matrix_index_omits_projection_for_non_grid_modes() {
        let best_params = DoubleMaParamsResolved {
            fast_len: 5,
            slow_len: 20,
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
        };
        let top = vec![(best_params.clone(), Metrics::default())];

        let (matrix_index, best_col, top_cols) = build_grid_matrix_index(
            OptimizationModeResolved::CoarseToFine,
            &[5, 10],
            &[20, 30],
            &best_params,
            &top,
        );

        assert!(matrix_index.is_none());
        assert!(best_col.is_none());
        assert!(top_cols.is_none());
    }

    #[test]
    fn build_grid_matrix_index_requires_full_top_projection() {
        let best_params = DoubleMaParamsResolved {
            fast_len: 5,
            slow_len: 20,
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
        };
        let off_axis = DoubleMaParamsResolved {
            fast_len: 10,
            slow_len: 25,
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
        };
        let top = vec![
            (best_params.clone(), Metrics::default()),
            (off_axis, Metrics::default()),
        ];

        let (matrix_index, best_col, top_cols) = build_grid_matrix_index(
            OptimizationModeResolved::Grid,
            &[5, 10],
            &[20, 30],
            &best_params,
            &top,
        );

        assert!(matrix_index.is_some());
        assert_eq!(best_col, Some(0));
        assert_eq!(top_cols, None);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "num_combos")]
    fn build_backend_result_debug_asserts_on_count_mismatch() {
        let result = OptimizationResult {
            num_combos: 2,
            ..base_result()
        };

        let _ = build_backend_optimization_result(
            result,
            BackendResultContext {
                mode_used: OptimizationModeResolved::Grid,
                exhaustive_output_requested: false,
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
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
                metric_surface_kind: None,
                shortlist_union_telemetry: None,
            },
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "top_scores")]
    fn build_backend_result_debug_asserts_on_top_score_mismatch() {
        let result = OptimizationResult {
            top: vec![(
                DoubleMaParams {
                    fast_len: 5,
                    slow_len: 20,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                Metrics::default(),
            )],
            top_scores: Some(vec![1.0, 2.0]),
            ..base_result()
        };

        let _ = build_backend_optimization_result(
            result,
            BackendResultContext {
                mode_used: OptimizationModeResolved::Grid,
                exhaustive_output_requested: false,
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
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
                metric_surface_kind: None,
                shortlist_union_telemetry: None,
            },
        );
    }
}
