use crate::composite_validate::{
    CrossTickerConfig, CrossTickerStats, CrossTickerValidationSummary, PerturbConfig, PerturbStats,
    ReoptWalkForwardConfigOptional, ReoptWalkForwardStats, SpaBenchmarkKind, SpaConfigOptional,
    SpaValidationStats, TradeBootstrapConfig, TradeBootstrapStats, ValidationArtifactContext,
    WalkForwardConfigOptional, WalkForwardStats,
};
use crate::cscv_pbo::{CscvCandidatePboStats, CscvPboConfig, CscvPboResult, SliceReturnStats};
use crate::double_ma::{
    compute_combined_regime_filter, compute_log_returns_f64, first_finite_idx, ma_matrix_cpu_f64,
    resolve_alma_params_for_side, validate_ma_kind, validate_strategy_cfg, Backend, BackendUsed,
    HalfTrendRegimeFilter,
};
use crate::iterative_sim::{IterativeMetricsDelta, IterativeSimConfig, ValidationEngine};
use crate::progress::ProgressSink;
use crate::validation_pipeline::{
    build_run_verdict, sanitize_hard_gates, ValidationCandidateVerdict, ValidationPostCandidate,
    ValidationRunVerdict,
};

use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use ta_strategies::double_ma::{Metrics, StrategyConfig, Trade};

fn default_ma_source() -> String {
    "close".to_string()
}

fn default_seed() -> u64 {
    1
}

fn default_max_candidates() -> usize {
    32
}

fn default_validation_backend() -> Backend {
    Backend::CpuOnly
}

fn validation_backend_label(backend: &Backend) -> &'static str {
    match backend {
        Backend::Auto { .. } => "Auto",
        Backend::CpuOnly => "CpuOnly",
        Backend::GpuOnly { .. } => "GpuOnly",
        Backend::GpuHybridKernel { .. } => "GpuHybridKernel",
        Backend::GpuKernel { .. } => "GpuKernel",
    }
}

fn resolve_validation_backend_runtime(
    backend: &Backend,
    gpu_activated: bool,
    fallback_reason: Option<String>,
) -> (BackendUsed, Option<String>, Option<String>) {
    if gpu_activated {
        let route = match backend {
            Backend::Auto { .. } => Some("Auto->GpuKernel".to_string()),
            _ => Some(validation_backend_label(backend).to_string()),
        };
        return (BackendUsed::Gpu, route, None);
    }

    match backend {
        Backend::CpuOnly => (BackendUsed::Cpu, Some("CpuOnly".to_string()), None),
        Backend::Auto { .. } => (
            BackendUsed::Cpu,
            Some("Auto->CpuOnly".to_string()),
            fallback_reason.or_else(|| {
                Some(
                    "GPU validation runtime not implemented yet; using CPU validation.".to_string(),
                )
            }),
        ),
        _ => (
            BackendUsed::Cpu,
            Some(format!("{}->CpuOnly", validation_backend_label(backend))),
            fallback_reason.or_else(|| {
                Some(
                    "GPU validation runtime not implemented yet; using CPU validation.".to_string(),
                )
            }),
        ),
    }
}

#[derive(Clone)]
struct GpuFixedCandidateValidationEval {
    metrics: Vec<Metrics>,
    start_ts: Vec<usize>,
    walk_forwards: Option<Vec<WalkForwardStats>>,
    cscv_slices: Option<HashMap<(u16, u16), Vec<SliceReturnStats>>>,
    spa_slice_sums: Option<HashMap<(u16, u16), Vec<f64>>>,
    backend_used: BackendUsed,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
}

fn validation_supports_gpu_fixed_candidates(
    req: &DoubleMaValidationRequest,
) -> Result<u32, String> {
    match req.backend {
        Backend::CpuOnly => Err("CPU validation requested".to_string()),
        Backend::GpuHybridKernel { device_id } => Ok(device_id),
        Backend::GpuOnly { .. } => Err(
            "fixed-candidate GPU validation requires GpuKernel-compatible execution".to_string(),
        ),
        Backend::GpuKernel { device_id } | Backend::Auto { device_id } => Ok(device_id),
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn gpu_validation_strategy_blocker(_strategy: &StrategyConfig) -> Option<String> {
    None
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn gpu_validation_feature_blocker(req: &DoubleMaValidationRequest) -> Option<String> {
    if req.cross_ticker.is_some() {
        return Some(
            "GPU fixed-candidate validation does not support cross_ticker yet".to_string(),
        );
    }
    if !matches!(req.validation_engine, ValidationEngine::Vectorized) {
        return Some(
            "GPU fixed-candidate validation currently supports only the vectorized validation engine"
                .to_string(),
        );
    }
    None
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
type GpuFixedMetricMapResult = (
    HashMap<(u16, u16), Metrics>,
    BackendUsed,
    Option<String>,
    Option<String>,
);

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
fn gpu_fixed_candidate_metric_map(
    req: &DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    first_fast_params: Option<&HashMap<String, f64>>,
    first_slow_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    first_strategy: &StrategyConfig,
    first_halftrend: &Option<HalfTrendRegimeFilter>,
    combos: &[ta_strategies::double_ma::DoubleMaParams],
    device_id: u32,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    eval_range: Option<(usize, usize)>,
) -> Result<GpuFixedMetricMapResult, String> {
    match req.backend {
        Backend::GpuHybridKernel { .. } => {
            if let Some(eval_range) = eval_range {
                if !crate::double_ma_backend::hybrid_backend_enabled() {
                    let res = crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel_in_range(
                        candles,
                        fast_periods,
                        slow_periods,
                        fast_ma_type,
                        slow_ma_type,
                        &req.ma_source,
                        first_fast_params,
                        first_slow_params,
                        regime_filter,
                        first_strategy,
                        ta_optimizer::ObjectiveKind::Sharpe,
                        combos.len(),
                        true,
                        crate::gpu_backtest_kernel::KernelConfig {
                            device_id,
                            fast_alma_offset,
                            fast_alma_sigma,
                            slow_alma_offset,
                            slow_alma_sigma,
                        },
                        None,
                        None,
                        combos.len(),
                        0,
                        None,
                        Some(eval_range),
                        &AtomicBool::new(false),
                        None,
                    )?;
                    let mut metric_by_pair: HashMap<(u16, u16), Metrics> =
                        HashMap::with_capacity(combos.len());
                    if let Some(all) = res.all.as_ref() {
                        for (params, metric) in all {
                            metric_by_pair
                                .insert((params.fast_len, params.slow_len), metric.clone());
                        }
                    }
                    return Ok((
                        metric_by_pair,
                        BackendUsed::Gpu,
                        Some("GpuHybridKernel->GpuKernel".to_string()),
                        Some(
                            "GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel."
                                .to_string(),
                        ),
                    ));
                }

                let total_pairs =
                    crate::double_ma::estimate_pairs_fast_lt_slow(fast_periods, slow_periods);
                let (res, _, backend_used, route_note, fallback_reason) =
                    crate::double_ma::optimize_double_ma_gpu_hybrid_kernel_phase1(
                        candles,
                        device_id,
                        fast_periods,
                        slow_periods,
                        fast_ma_type,
                        slow_ma_type,
                        &req.ma_source,
                        first_fast_params,
                        first_slow_params,
                        regime_filter,
                        first_strategy,
                        ta_optimizer::ObjectiveKind::Sharpe,
                        combos.len(),
                        true,
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        None,
                        None,
                        total_pairs,
                        0,
                        &AtomicBool::new(false),
                        None,
                        crate::double_ma::GpuExecutionPolicy::BestEffort,
                        true,
                        Some(eval_range),
                    )?;
                let mut metric_by_pair: HashMap<(u16, u16), Metrics> =
                    HashMap::with_capacity(combos.len());
                if let Some(all) = res.all.as_ref() {
                    for (params, metric) in all {
                        metric_by_pair.insert((params.fast_len, params.slow_len), metric.clone());
                    }
                }
                return Ok((
                    metric_by_pair,
                    backend_used,
                    route_note.or_else(|| Some("GpuHybridKernel".to_string())),
                    fallback_reason,
                ));
            }
            let res = crate::double_ma::run_double_ma_optimization_blocking_with_candles(
                crate::double_ma::DoubleMaRequest {
                    backend: req.backend.clone(),
                    data_id: req.data_id.clone(),
                    fast_range: (
                        u32::from(*fast_periods.first().unwrap_or(&1)),
                        u32::from(*fast_periods.last().unwrap_or(&1)),
                        1,
                    ),
                    slow_range: (
                        u32::from(*slow_periods.first().unwrap_or(&1)),
                        u32::from(*slow_periods.last().unwrap_or(&1)),
                        1,
                    ),
                    fast_ma_types: vec![fast_ma_type.to_string()],
                    slow_ma_types: vec![slow_ma_type.to_string()],
                    ma_source: req.ma_source.clone(),
                    export_all_csv_path: None,
                    fast_ma_params: first_fast_params.cloned(),
                    slow_ma_params: first_slow_params.cloned(),
                    fast_ma_param_sweeps: None,
                    slow_ma_param_sweeps: None,
                    strategy: first_strategy.clone(),
                    halftrend_filter: first_halftrend.clone(),
                    objective: ta_optimizer::ObjectiveKind::Sharpe,
                    mode: ta_optimizer::OptimizationMode::Grid,
                    top_k: Some(combos.len()),
                    include_all: Some(true),
                    heatmap_bins: Some(0),
                    constraints: None,
                    vdps: None,
                    dynamic_strategy_ir: None,
                    gpu_execution: crate::double_ma::GpuExecutionPolicy::BestEffort,
                    persist_followup_cache: false,
                },
                candles,
                &AtomicBool::new(false),
                None,
            )?;
            let mut metric_by_pair: HashMap<(u16, u16), Metrics> =
                HashMap::with_capacity(combos.len());
            if let Some(all) = res.all.as_ref() {
                for (params, metric) in all {
                    metric_by_pair.insert((params.fast_len, params.slow_len), metric.clone());
                }
            }
            Ok((
                metric_by_pair,
                res.backend_used,
                res.backend_route,
                res.backend_fallback_reason,
            ))
        }
        _ => {
            let res = crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel_in_range(
                candles,
                fast_periods,
                slow_periods,
                fast_ma_type,
                slow_ma_type,
                &req.ma_source,
                first_fast_params,
                first_slow_params,
                regime_filter,
                first_strategy,
                ta_optimizer::ObjectiveKind::Sharpe,
                combos.len(),
                true,
                crate::gpu_backtest_kernel::KernelConfig {
                    device_id,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                },
                None,
                None,
                fast_periods.len().saturating_mul(slow_periods.len()),
                0,
                None,
                eval_range,
                &AtomicBool::new(false),
                None,
            )?;
            let mut metric_by_pair: HashMap<(u16, u16), Metrics> =
                HashMap::with_capacity(combos.len());
            if let Some(all) = res.all.as_ref() {
                for (params, metric) in all {
                    metric_by_pair.insert((params.fast_len, params.slow_len), metric.clone());
                }
            }
            Ok((
                metric_by_pair,
                BackendUsed::Gpu,
                Some(match req.backend {
                    Backend::Auto { .. } => "Auto->GpuKernel".to_string(),
                    _ => validation_backend_label(&req.backend).to_string(),
                }),
                None,
            ))
        }
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
fn compute_gpu_walk_forward_stats_for_candidates(
    req: &DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    first_fast_params: Option<&HashMap<String, f64>>,
    first_slow_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    first_strategy: &StrategyConfig,
    first_halftrend: &Option<HalfTrendRegimeFilter>,
    combos: &[ta_strategies::double_ma::DoubleMaParams],
    start_ts: &[usize],
    device_id: u32,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    cfg: &WalkForwardConfigOptional,
) -> Result<Vec<WalkForwardStats>, String> {
    let n = candles.close.len();
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);

    let mut candidate_windows: Vec<Vec<usize>> = Vec::with_capacity(start_ts.len());
    let mut unique_window_starts: BTreeSet<usize> = BTreeSet::new();
    for &start_t in start_ts {
        let mut windows = Vec::new();
        let mut t0 = start_t;
        while t0 + 2 <= n {
            let end = t0.saturating_add(window).min(n);
            if end <= t0 + 1 {
                break;
            }
            windows.push(t0);
            unique_window_starts.insert(t0);
            if end == n {
                break;
            }
            t0 = t0.saturating_add(step);
        }
        candidate_windows.push(windows);
    }

    let mut by_window: HashMap<usize, HashMap<(u16, u16), Metrics>> =
        HashMap::with_capacity(unique_window_starts.len());
    for &t0 in &unique_window_starts {
        let end = t0.saturating_add(window).min(n);
        let (metric_map, _, _, _) = gpu_fixed_candidate_metric_map(
            req,
            candles,
            fast_periods,
            slow_periods,
            fast_ma_type,
            slow_ma_type,
            first_fast_params,
            first_slow_params,
            regime_filter,
            first_strategy,
            first_halftrend,
            combos,
            device_id,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            Some((t0, end)),
        )?;
        by_window.insert(t0, metric_map);
    }

    let mut out = Vec::with_capacity(req.candidates.len());
    for (idx, cand) in req.candidates.iter().enumerate() {
        let mut sharpes = Vec::new();
        let mut dds = Vec::new();
        let mut pnls = Vec::new();
        let mut downside_ratios = Vec::new();
        let mut ulcers = Vec::new();
        let mut martins = Vec::new();
        let mut trades = Vec::new();
        for &t0 in &candidate_windows[idx] {
            let metric = by_window
                .get(&t0)
                .and_then(|m| m.get(&(cand.fast_len, cand.slow_len)))
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "GPU walk-forward validation did not return metrics for pair ({}, {}) at window start {}",
                        cand.fast_len, cand.slow_len, t0
                    )
                })?;
            sharpes.push(metric.sharpe);
            dds.push(metric.max_dd);
            pnls.push(metric.pnl);
            if let Some(ratio) =
                crate::validation_pipeline::downside_vol_ratio_from_metrics(&metric)
            {
                downside_ratios.push(ratio);
            }
            ulcers.push(metric.ulcer_index);
            martins.push(metric.martin_ratio);
            trades.push(metric.trades);
        }
        sharpes.sort_by(|a, b| a.total_cmp(b));
        dds.sort_by(|a, b| a.total_cmp(b));
        pnls.sort_by(|a, b| a.total_cmp(b));
        downside_ratios.sort_by(|a, b| a.total_cmp(b));
        ulcers.sort_by(|a, b| a.total_cmp(b));
        martins.sort_by(|a, b| a.total_cmp(b));
        out.push(WalkForwardStats {
            windows: sharpes.len(),
            sharpe_median: crate::stats_common::median(&sharpes),
            sharpe_worst: crate::stats_common::worst_min(&sharpes),
            max_dd_median: crate::stats_common::median(&dds),
            max_dd_worst: crate::stats_common::worst_max(&dds),
            pnl_median: crate::stats_common::median(&pnls),
            pnl_worst: crate::stats_common::worst_min(&pnls),
            downside_vol_ratio_median: crate::stats_common::median(&downside_ratios),
            downside_vol_ratio_worst: crate::stats_common::worst_max(&downside_ratios),
            ulcer_index_median: crate::stats_common::median(&ulcers),
            ulcer_index_worst: crate::stats_common::worst_max(&ulcers),
            martin_ratio_median: crate::stats_common::median(&martins),
            martin_ratio_worst: crate::stats_common::worst_min(&martins),
            trades_median: crate::stats_common::median_u32(&trades),
            trades_worst: crate::stats_common::worst_min_u32(&trades),
        });
    }
    Ok(out)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn sanitize_gpu_cscv_slices(slices: usize) -> usize {
    let s = slices.clamp(4, 32);
    if (s & 1) == 0 {
        s
    } else {
        s.saturating_sub(1).max(4)
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
fn compute_gpu_cscv_slice_stats_for_candidates(
    req: &DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    first_fast_params: Option<&HashMap<String, f64>>,
    first_slow_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    first_strategy: &StrategyConfig,
    first_halftrend: &Option<HalfTrendRegimeFilter>,
    combos: &[ta_strategies::double_ma::DoubleMaParams],
    start_ts: &[usize],
    device_id: u32,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    cfg: &CscvPboConfig,
) -> Result<Option<HashMap<(u16, u16), Vec<SliceReturnStats>>>, String> {
    let slices = sanitize_gpu_cscv_slices(cfg.slices);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = candles.close.len().saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || req.candidates.len() < 2 {
        return Ok(None);
    }

    let mut out: HashMap<(u16, u16), Vec<SliceReturnStats>> =
        HashMap::with_capacity(req.candidates.len());
    for cand in &req.candidates {
        out.entry((cand.fast_len, cand.slow_len))
            .or_insert_with(|| vec![SliceReturnStats::default(); slices]);
    }

    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(candles.close.len());
        let (metric_map, _, _, _) = gpu_fixed_candidate_metric_map(
            req,
            candles,
            fast_periods,
            slow_periods,
            fast_ma_type,
            slow_ma_type,
            first_fast_params,
            first_slow_params,
            regime_filter,
            first_strategy,
            first_halftrend,
            combos,
            device_id,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            Some((t0, end)),
        )?;
        let nf = slice_bars as f64;
        for (pair, metric) in metric_map {
            if let Some(slice_vec) = out.get_mut(&pair) {
                let mean = metric.sharpe * metric.volatility;
                let sum = mean * nf;
                let variance = metric.volatility * metric.volatility;
                let sumsq = variance * (nf - 1.0) + (sum * sum) / nf;
                slice_vec[slice_idx] = SliceReturnStats { sum, sumsq };
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
fn compute_gpu_spa_slice_sums_for_candidates(
    req: &DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    first_fast_params: Option<&HashMap<String, f64>>,
    first_slow_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    first_strategy: &StrategyConfig,
    first_halftrend: &Option<HalfTrendRegimeFilter>,
    combos: &[ta_strategies::double_ma::DoubleMaParams],
    start_ts: &[usize],
    device_id: u32,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    cfg: &SpaConfigOptional,
) -> Result<Option<HashMap<(u16, u16), Vec<f64>>>, String> {
    let slices = cfg.slices.max(4);
    let common_start_t = start_ts.iter().copied().max().unwrap_or(0);
    let slice_bars = candles.close.len().saturating_sub(common_start_t) / slices;
    if slice_bars < 2 || req.candidates.is_empty() {
        return Ok(None);
    }

    let mut out: HashMap<(u16, u16), Vec<f64>> = HashMap::with_capacity(req.candidates.len());
    for cand in &req.candidates {
        out.entry((cand.fast_len, cand.slow_len))
            .or_insert_with(|| vec![0.0; slices]);
    }

    let nf = slice_bars as f64;
    for slice_idx in 0..slices {
        let t0 = common_start_t + slice_idx * slice_bars;
        let end = t0.saturating_add(slice_bars).min(candles.close.len());
        let (metric_map, _, _, _) = gpu_fixed_candidate_metric_map(
            req,
            candles,
            fast_periods,
            slow_periods,
            fast_ma_type,
            slow_ma_type,
            first_fast_params,
            first_slow_params,
            regime_filter,
            first_strategy,
            first_halftrend,
            combos,
            device_id,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            Some((t0, end)),
        )?;
        for (pair, metric) in metric_map {
            if let Some(slice_vec) = out.get_mut(&pair) {
                slice_vec[slice_idx] = metric.sharpe * metric.volatility * nf;
            }
        }
    }

    Ok(Some(out))
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[allow(clippy::too_many_arguments)]
fn compute_gpu_reopt_walk_forward_stats_for_candidates(
    req: &DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    finalist_params: &[DoubleMaValidationParams],
    finalist_start_ts: &[usize],
    first_fast_params: Option<&HashMap<String, f64>>,
    first_slow_params: Option<&HashMap<String, f64>>,
    cfg: &ReoptWalkForwardConfigOptional,
) -> Result<Option<ReoptWalkForwardStats>, String> {
    if finalist_params.is_empty() || candles.close.len() < 3 {
        return Ok(None);
    }

    let top_n = cfg.top_n_candidates.max(1).min(finalist_params.len());
    let finalists = &finalist_params[..top_n];
    let finalist_start_ts = &finalist_start_ts[..top_n];
    let device_id = validation_supports_gpu_fixed_candidates(req)?;
    let first = &finalists[0];
    let fast_ma_type = first.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma_type = first.slow_ma_type.trim().to_ascii_lowercase();
    let first_fast_effective = first.fast_ma_params.as_ref().or(first_fast_params);
    let first_slow_effective = first.slow_ma_params.as_ref().or(first_slow_params);
    let first_halftrend = &first.halftrend_filter;
    let first_strategy = &first.strategy;

    let mut fast_periods: BTreeSet<u16> = BTreeSet::new();
    let mut slow_periods: BTreeSet<u16> = BTreeSet::new();
    let mut combos = Vec::with_capacity(finalists.len());
    let mut finalist_entries = Vec::with_capacity(finalists.len());
    for (idx, params) in finalists.iter().enumerate() {
        if params.fast_ma_type.trim().to_ascii_lowercase() != fast_ma_type
            || params.slow_ma_type.trim().to_ascii_lowercase() != slow_ma_type
        {
            return Err(
                "GPU reopt_walk_forward currently requires uniform fast/slow MA types".to_string(),
            );
        }
        if !serialized_eq(&params.strategy, first_strategy)? {
            return Err(
                "GPU reopt_walk_forward currently requires a uniform StrategyConfig".to_string(),
            );
        }
        if !serialized_eq(&params.halftrend_filter, first_halftrend)? {
            return Err(
                "GPU reopt_walk_forward currently requires a uniform halftrend filter".to_string(),
            );
        }
        if !serialized_eq(
            &params.fast_ma_params.as_ref().or(first_fast_params),
            &first_fast_effective,
        )? {
            return Err(
                "GPU reopt_walk_forward currently requires uniform fast MA params".to_string(),
            );
        }
        if !serialized_eq(
            &params.slow_ma_params.as_ref().or(first_slow_params),
            &first_slow_effective,
        )? {
            return Err(
                "GPU reopt_walk_forward currently requires uniform slow MA params".to_string(),
            );
        }
        fast_periods.insert(params.fast_len);
        slow_periods.insert(params.slow_len);
        combos.push(ta_strategies::double_ma::DoubleMaParams {
            fast_len: params.fast_len,
            slow_len: params.slow_len,
            fast_ma_id: 0,
            slow_ma_id: 0,
        });
        let key = serde_json::to_string(params).unwrap_or_else(|_| {
            format!(
                "{}:{}:{}:{}",
                params.fast_ma_type, params.fast_len, params.slow_ma_type, params.slow_len
            )
        });
        finalist_entries.push((
            key,
            (params.fast_len, params.slow_len),
            finalist_start_ts[idx],
        ));
    }
    let fast_periods: Vec<u16> = fast_periods.into_iter().collect();
    let slow_periods: Vec<u16> = slow_periods.into_iter().collect();

    let regime_filter = compute_combined_regime_filter(
        candles,
        &req.ma_source,
        first_strategy,
        first_halftrend.as_ref(),
    )?;
    let blockers = crate::double_ma_backend::gpu_kernel_blockers_for_request(
        &fast_ma_type,
        &slow_ma_type,
        first_strategy,
        regime_filter.is_some(),
        ta_optimizer::ObjectiveKind::Sharpe,
        combos.len(),
        true,
    );
    if !blockers.is_empty() {
        return Err(format!(
            "GPU reopt_walk_forward is unavailable. Reasons: {}",
            blockers.join("; ")
        ));
    }

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma_type, first_fast_effective)?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma_type, first_slow_effective)?;

    let n = candles.close.len();
    let common_start_t = finalist_start_ts
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .min(n.saturating_sub(2));
    let window = cfg.window_bars.max(64);
    let step = cfg.step_bars.max(1);
    let mut ranges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut t0 = common_start_t;
    while t0 + 2 < n {
        let train_end = t0.saturating_add(window).min(n);
        if train_end <= t0 + 1 {
            break;
        }
        let test_end = train_end.saturating_add(step).min(n);
        if test_end <= train_end + 1 {
            break;
        }
        ranges.insert((t0, train_end));
        ranges.insert((train_end, test_end));
        if test_end == n {
            break;
        }
        t0 = t0.saturating_add(step);
    }
    if ranges.is_empty() {
        return Ok(None);
    }

    let mut range_metrics: HashMap<(usize, usize), HashMap<(u16, u16), Metrics>> =
        HashMap::with_capacity(ranges.len());
    for (start, end) in ranges {
        let (metric_map, _, _, _) = gpu_fixed_candidate_metric_map(
            req,
            candles,
            &fast_periods,
            &slow_periods,
            &fast_ma_type,
            &slow_ma_type,
            first_fast_effective,
            first_slow_effective,
            regime_filter.as_deref(),
            first_strategy,
            first_halftrend,
            &combos,
            device_id,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            Some((start, end)),
        )?;
        range_metrics.insert((start, end), metric_map);
    }

    crate::validation_pipeline::compute_reopt_walk_forward_stats(
        &finalist_entries,
        n,
        cfg,
        |(_, _, start_t)| *start_t,
        |(_, pair, _), start, end| {
            range_metrics
                .get(&(start, end))
                .and_then(|m| m.get(pair))
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "missing GPU reopt metrics for pair ({}, {}) in range {}..{}",
                        pair.0, pair.1, start, end
                    )
                })
        },
        |(key, _, _)| key.clone(),
    )
    .ok_or_else(|| "GPU reopt_walk_forward did not produce any windows".to_string())
    .map(Some)
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
#[allow(clippy::too_many_arguments)]
fn compute_gpu_reopt_walk_forward_stats_for_candidates(
    _req: &DoubleMaValidationRequest,
    _candles: &vector_ta::utilities::data_loader::Candles,
    _finalist_params: &[DoubleMaValidationParams],
    _finalist_start_ts: &[usize],
    _first_fast_params: Option<&HashMap<String, f64>>,
    _first_slow_params: Option<&HashMap<String, f64>>,
    _cfg: &ReoptWalkForwardConfigOptional,
) -> Result<Option<ReoptWalkForwardStats>, String> {
    Err("GPU reopt_walk_forward requires cuda and cuda-backtest-kernel features".to_string())
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn serialized_eq<T: Serialize>(a: &T, b: &T) -> Result<bool, String> {
    Ok(serde_json::to_string(a).map_err(|e| e.to_string())?
        == serde_json::to_string(b).map_err(|e| e.to_string())?)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn candidate_start_t(
    candles: &vector_ta::utilities::data_loader::Candles,
    ma_source: &str,
    base_first_valid: usize,
    params: &DoubleMaValidationParams,
) -> Result<usize, String> {
    let n = candles.close.len();
    let trade_offset = if params.strategy.trade_on_next_bar {
        1
    } else {
        0
    };
    let ma_warmup = base_first_valid
        .saturating_add(
            (params.fast_len as usize)
                .max(params.slow_len as usize)
                .saturating_sub(1),
        )
        .saturating_add(trade_offset);
    let mut start_t = ma_warmup;
    let halftrend_trend = compute_combined_regime_filter(
        candles,
        ma_source,
        &params.strategy,
        params.halftrend_filter.as_ref(),
    )?;
    if let Some(trend) = halftrend_trend.as_ref() {
        if trend.len() != n {
            return Err("regime trend series length mismatch".to_string());
        }
        let fv_ht = first_finite_idx(trend)
            .ok_or_else(|| "regime trend series all non-finite".to_string())?;
        start_t = start_t.max(fv_ht);
    }
    if let Some(ks) =
        crate::double_ma::compute_kaufman_stop_series(candles, &params.strategy)?.as_ref()
    {
        let fv_k_long = first_finite_idx(&ks.long)
            .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?;
        let fv_k_short = first_finite_idx(&ks.short)
            .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?;
        start_t = start_t.max(fv_k_long.max(fv_k_short));
    }
    Ok(start_t)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn try_prepare_gpu_fixed_candidate_validation(
    req: &DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    base_first_valid: usize,
) -> Result<GpuFixedCandidateValidationEval, String> {
    let device_id = validation_supports_gpu_fixed_candidates(req)?;
    if let Some(reason) = gpu_validation_feature_blocker(req) {
        return Err(reason);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let first = req
        .candidates
        .first()
        .ok_or_else(|| "no candidates to validate".to_string())?;
    let fast_ma_type = first.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma_type = first.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma_type.is_empty() || slow_ma_type.is_empty() {
        return Err("candidate MA types cannot be empty".to_string());
    }

    let first_fast_params = first
        .fast_ma_params
        .as_ref()
        .or(req.fast_ma_params.as_ref());
    let first_slow_params = first
        .slow_ma_params
        .as_ref()
        .or(req.slow_ma_params.as_ref());
    let first_strategy = &first.strategy;
    let first_halftrend = &first.halftrend_filter;
    if let Some(reason) = gpu_validation_strategy_blocker(first_strategy) {
        return Err(reason);
    }

    let mut fast_periods: BTreeSet<u16> = BTreeSet::new();
    let mut slow_periods: BTreeSet<u16> = BTreeSet::new();
    let mut combos = Vec::with_capacity(req.candidates.len());
    let mut start_ts = Vec::with_capacity(req.candidates.len());

    for cand in &req.candidates {
        if cand.fast_ma_type.trim().to_ascii_lowercase() != fast_ma_type
            || cand.slow_ma_type.trim().to_ascii_lowercase() != slow_ma_type
        {
            return Err(
                "GPU fixed-candidate validation currently requires uniform fast/slow MA types"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.strategy, first_strategy)? {
            return Err(
                "GPU fixed-candidate validation currently requires a uniform StrategyConfig"
                    .to_string(),
            );
        }
        if !serialized_eq(&cand.halftrend_filter, first_halftrend)? {
            return Err(
                "GPU fixed-candidate validation currently requires a uniform halftrend filter"
                    .to_string(),
            );
        }
        if !serialized_eq(
            &cand.fast_ma_params.as_ref().or(req.fast_ma_params.as_ref()),
            &first_fast_params,
        )? {
            return Err(
                "GPU fixed-candidate validation currently requires uniform fast MA params"
                    .to_string(),
            );
        }
        if !serialized_eq(
            &cand.slow_ma_params.as_ref().or(req.slow_ma_params.as_ref()),
            &first_slow_params,
        )? {
            return Err(
                "GPU fixed-candidate validation currently requires uniform slow MA params"
                    .to_string(),
            );
        }

        fast_periods.insert(cand.fast_len);
        slow_periods.insert(cand.slow_len);
        combos.push(ta_strategies::double_ma::DoubleMaParams {
            fast_len: cand.fast_len,
            slow_len: cand.slow_len,
            fast_ma_id: 0,
            slow_ma_id: 0,
        });
        start_ts.push(candidate_start_t(
            candles,
            &req.ma_source,
            base_first_valid,
            cand,
        )?);
    }

    let fast_periods: Vec<u16> = fast_periods.into_iter().collect();
    let slow_periods: Vec<u16> = slow_periods.into_iter().collect();
    let regime_filter = compute_combined_regime_filter(
        candles,
        &req.ma_source,
        first_strategy,
        first_halftrend.as_ref(),
    )?;
    let blockers = crate::double_ma_backend::gpu_kernel_blockers_for_request(
        &fast_ma_type,
        &slow_ma_type,
        first_strategy,
        regime_filter.is_some(),
        ta_optimizer::ObjectiveKind::Sharpe,
        combos.len(),
        true,
    );
    if !blockers.is_empty() {
        return Err(format!(
            "GPU fixed-candidate validation is unavailable. Reasons: {}",
            blockers.join("; ")
        ));
    }

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma_type, first_fast_params)?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma_type, first_slow_params)?;
    let metric_by_pair = gpu_fixed_candidate_metric_map(
        req,
        candles,
        &fast_periods,
        &slow_periods,
        &fast_ma_type,
        &slow_ma_type,
        first_fast_params,
        first_slow_params,
        regime_filter.as_deref(),
        first_strategy,
        first_halftrend,
        &combos,
        device_id,
        fast_alma_offset,
        fast_alma_sigma,
        slow_alma_offset,
        slow_alma_sigma,
        None,
    )?;

    let mut ordered_metrics = Vec::with_capacity(req.candidates.len());
    for cand in &req.candidates {
        let metric = metric_by_pair
            .0
            .get(&(cand.fast_len, cand.slow_len))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "GPU fixed-candidate validation did not return metrics for pair ({}, {})",
                    cand.fast_len, cand.slow_len
                )
            })?;
        ordered_metrics.push(metric);
    }

    let walk_forwards = if let Some(walk_cfg) = req.walk_forward.as_ref() {
        Some(compute_gpu_walk_forward_stats_for_candidates(
            req,
            candles,
            &fast_periods,
            &slow_periods,
            &fast_ma_type,
            &slow_ma_type,
            first_fast_params,
            first_slow_params,
            regime_filter.as_deref(),
            first_strategy,
            first_halftrend,
            &combos,
            &start_ts,
            device_id,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            walk_cfg,
        )?)
    } else {
        None
    };
    let cscv_slices = if let Some(cscv_cfg) = req.cscv_pbo.as_ref() {
        compute_gpu_cscv_slice_stats_for_candidates(
            req,
            candles,
            &fast_periods,
            &slow_periods,
            &fast_ma_type,
            &slow_ma_type,
            first_fast_params,
            first_slow_params,
            regime_filter.as_deref(),
            first_strategy,
            first_halftrend,
            &combos,
            &start_ts,
            device_id,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            cscv_cfg,
        )?
    } else {
        None
    };
    let spa_slice_sums = if let Some(spa_cfg) = req.spa.as_ref() {
        compute_gpu_spa_slice_sums_for_candidates(
            req,
            candles,
            &fast_periods,
            &slow_periods,
            &fast_ma_type,
            &slow_ma_type,
            first_fast_params,
            first_slow_params,
            regime_filter.as_deref(),
            first_strategy,
            first_halftrend,
            &combos,
            &start_ts,
            device_id,
            fast_alma_offset,
            fast_alma_sigma,
            slow_alma_offset,
            slow_alma_sigma,
            spa_cfg,
        )?
    } else {
        None
    };

    Ok(GpuFixedCandidateValidationEval {
        metrics: ordered_metrics,
        start_ts,
        walk_forwards,
        cscv_slices,
        spa_slice_sums,
        backend_used: metric_by_pair.1,
        backend_route: metric_by_pair.2,
        backend_fallback_reason: metric_by_pair.3,
    })
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn try_prepare_gpu_fixed_candidate_validation(
    req: &DoubleMaValidationRequest,
    _candles: &vector_ta::utilities::data_loader::Candles,
    _base_first_valid: usize,
) -> Result<GpuFixedCandidateValidationEval, String> {
    let _ = req;
    Err(
        "GPU fixed-candidate validation requires cuda and cuda-backtest-kernel features"
            .to_string(),
    )
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DoubleMaValidationParams {
    pub fast_ma_type: String,
    pub slow_ma_type: String,
    pub fast_len: u16,
    pub slow_len: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_ma_params: Option<HashMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DoubleMaValidationRequest {
    pub data_id: String,
    #[serde(default = "default_validation_backend")]
    pub backend: Backend,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    pub candidates: Vec<DoubleMaValidationParams>,
    #[serde(default)]
    pub walk_forward: Option<WalkForwardConfigOptional>,
    #[serde(default)]
    pub reopt_walk_forward: Option<ReoptWalkForwardConfigOptional>,
    #[serde(default)]
    pub spa: Option<SpaConfigOptional>,
    #[serde(default)]
    pub perturb: Option<PerturbConfig>,
    #[serde(default)]
    pub trade_bootstrap: Option<TradeBootstrapConfig>,
    #[serde(default)]
    pub cross_ticker: Option<CrossTickerConfig>,
    #[serde(default)]
    pub cscv_pbo: Option<CscvPboConfig>,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,
    #[serde(default)]
    pub validation_engine: ValidationEngine,
    #[serde(default)]
    pub iterative: IterativeSimConfig,
    #[serde(default)]
    pub min_psr: Option<f64>,
    #[serde(default)]
    pub min_dsr: Option<f64>,
    #[serde(default)]
    pub min_cscv_oos_rank_frac: Option<f64>,
    #[serde(default)]
    pub min_cscv_oos_metric: Option<f64>,
    #[serde(default)]
    pub max_pbo: Option<f64>,
    #[serde(default)]
    pub validation_context: Option<ValidationArtifactContext>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoubleMaValidationCandidateResult {
    pub params: DoubleMaValidationParams,
    pub in_sample: Metrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterative_in_sample: Option<Metrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterative_delta: Option<IterativeMetricsDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psr: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsr: Option<f64>,
    pub track_record_bars: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_track_record_length_bars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walk_forward: Option<WalkForwardStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perturb: Option<PerturbStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_bootstrap: Option<TradeBootstrapStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regime_slice_robustness: Option<RegimeSliceRobustnessStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friction_stress: Option<FrictionStressStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker: Option<CrossTickerStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker_pareto: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cscv_pbo: Option<CscvCandidatePboStats>,
    #[serde(default)]
    pub validation_verdict: ValidationCandidateVerdict,
    pub robust_score: f64,
    pub robust_pareto: bool,
}

impl ValidationPostCandidate for DoubleMaValidationCandidateResult {
    fn in_sample_metrics(&self) -> &Metrics {
        &self.in_sample
    }

    fn iterative_in_sample_metrics(&self) -> Option<&Metrics> {
        self.iterative_in_sample.as_ref()
    }

    fn walk_forward_stats(&self) -> Option<&WalkForwardStats> {
        self.walk_forward.as_ref()
    }

    fn cscv_oos_metric_p50(&self) -> Option<f64> {
        self.cscv_pbo.as_ref().and_then(|c| c.oos_metric_p50)
    }

    fn cscv_oos_rank_frac_p50(&self) -> Option<f64> {
        self.cscv_pbo.as_ref().and_then(|c| c.oos_rank_frac_p50)
    }

    fn psr(&self) -> Option<f64> {
        self.psr
    }

    fn dsr(&self) -> Option<f64> {
        self.dsr
    }

    fn set_psr(&mut self, v: Option<f64>) {
        self.psr = v;
    }

    fn set_dsr(&mut self, v: Option<f64>) {
        self.dsr = v;
    }

    fn track_record_bars(&self) -> usize {
        self.track_record_bars
    }

    fn min_track_record_length_bars(&self) -> Option<usize> {
        self.min_track_record_length_bars
    }

    fn set_track_record_bars(&mut self, v: usize) {
        self.track_record_bars = v;
    }

    fn set_min_track_record_length_bars(&mut self, v: Option<usize>) {
        self.min_track_record_length_bars = v;
    }

    fn robust_score(&self) -> f64 {
        self.robust_score
    }

    fn set_robust_score(&mut self, v: f64) {
        self.robust_score = v;
    }

    fn set_robust_pareto(&mut self, v: bool) {
        self.robust_pareto = v;
    }

    fn set_cscv_pbo(&mut self, v: Option<CscvCandidatePboStats>) {
        self.cscv_pbo = v;
    }

    fn validation_verdict(&self) -> &ValidationCandidateVerdict {
        &self.validation_verdict
    }

    fn set_validation_verdict(&mut self, v: ValidationCandidateVerdict) {
        self.validation_verdict = v;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DoubleMaValidationResult {
    pub data_id: String,
    pub runtime_ms: u64,
    pub backend_used: BackendUsed,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    pub candidates: Vec<DoubleMaValidationCandidateResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reopt_walk_forward: Option<ReoptWalkForwardStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spa: Option<SpaValidationStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker: Option<CrossTickerValidationSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cscv_pbo: Option<CscvPboResult>,
    pub run_verdict: ValidationRunVerdict,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegimeBucketStats {
    pub bars: usize,
    pub active_frac: f64,
    pub pnl: f64,
    pub sharpe: f64,
    pub downside_deviation: f64,
    pub ulcer_index: f64,
    pub martin_ratio: f64,
    pub max_dd: f64,
    pub trades: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegimeSliceRobustnessStats {
    pub vol_window: usize,
    pub low_vol: RegimeBucketStats,
    pub mid_vol: RegimeBucketStats,
    pub high_vol: RegimeBucketStats,
    pub trend: RegimeBucketStats,
    pub range: RegimeBucketStats,
    pub worst_bucket_label: String,
    pub worst_sharpe: f64,
    pub worst_ulcer_index: f64,
    pub worst_martin_ratio: f64,
    pub worst_max_dd: f64,
    pub stable_bucket_count: usize,
    pub stable_bucket_frac: f64,
    pub trend_pass: bool,
    pub range_pass: bool,
    pub support_label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FrictionStressStats {
    pub scenario_count: usize,
    pub pass_count: usize,
    pub pass_frac: f64,
    pub worst_scenario_label: String,
    pub worst_pnl: f64,
    pub worst_sharpe: f64,
    pub worst_downside_deviation: f64,
    pub worst_ulcer_index: f64,
    pub worst_martin_ratio: f64,
    pub worst_max_dd: f64,
    pub mean_rank: f64,
    pub max_rank_drift: usize,
}

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed: usize,
    total: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed, total, phase);
}

fn double_ma_validation_robust_score(
    in_sample: &Metrics,
    walk_forward: Option<&WalkForwardStats>,
) -> f64 {
    crate::validation_pipeline::robust_score_from_metrics(in_sample, walk_forward)
}

#[derive(Clone)]
struct CandidateSeries<'a> {
    n: usize,
    lr: &'a [f64],
    close: &'a [f64],
    fast: Vec<f64>,
    slow: Vec<f64>,
    halftrend_trend: Option<Vec<f64>>,
    kaufman_stops: Option<crate::double_ma::KaufmanStopSeries>,
    start_t: usize,
    cfg: StrategyConfig,
}

fn build_candidate_series<'a>(
    candles: &'a vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params_req: Option<&HashMap<String, f64>>,
    slow_ma_params_req: Option<&HashMap<String, f64>>,
    params: &DoubleMaValidationParams,
) -> Result<CandidateSeries<'a>, String> {
    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n {
        return Err("log returns length mismatch".to_string());
    }

    validate_strategy_cfg(&params.strategy)?;

    let fast_ma = params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = params.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("fast/slow MA type cannot be empty".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_params = params.fast_ma_params.as_ref().or(fast_ma_params_req);
    let slow_params = params.slow_ma_params.as_ref().or(slow_ma_params_req);

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma, fast_params)?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, slow_params)?;

    let (fast_mat, _) = ma_matrix_cpu_f64(
        candles,
        &fast_ma,
        ma_source,
        &[params.fast_len],
        fast_params,
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    let (slow_mat, _) = ma_matrix_cpu_f64(
        candles,
        &slow_ma,
        ma_source,
        &[params.slow_len],
        slow_params,
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    if fast_mat.len() != n || slow_mat.len() != n {
        return Err("MA series length mismatch".to_string());
    }
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    if close.len() != n {
        return Err("close series length mismatch".to_string());
    }
    let kaufman_stops = crate::double_ma::compute_kaufman_stop_series(candles, &params.strategy)?;
    let mut first_valid_extra = base_first_valid;
    if let Some(ks) = kaufman_stops.as_ref() {
        let fv_k_long = first_finite_idx(&ks.long)
            .ok_or_else(|| "kaufman-stop long series all non-finite".to_string())?;
        let fv_k_short = first_finite_idx(&ks.short)
            .ok_or_else(|| "kaufman-stop short series all non-finite".to_string())?;
        first_valid_extra = first_valid_extra.max(fv_k_long.max(fv_k_short));
    }

    let halftrend_trend: Option<Vec<f64>> = compute_combined_regime_filter(
        candles,
        ma_source,
        &params.strategy,
        params.halftrend_filter.as_ref(),
    )?;

    let trade_offset = if params.strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let t_sig0 = fast_mat
        .iter()
        .zip(slow_mat.iter())
        .position(|(f, s)| f.is_finite() && s.is_finite())
        .unwrap_or(n);
    let mut start_t = t_sig0.saturating_add(trade_offset).max(first_valid_extra);

    if let Some(trend) = halftrend_trend.as_ref() {
        if trend.len() != n {
            return Err("regime trend series length mismatch".to_string());
        }
        let fv_ht = first_finite_idx(trend)
            .ok_or_else(|| "regime trend series all non-finite".to_string())?;
        start_t = start_t.max(fv_ht);
    }

    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    Ok(CandidateSeries {
        n,
        lr,
        close,
        fast: fast_mat,
        slow: slow_mat,
        halftrend_trend,
        kaufman_stops,
        start_t,
        cfg: params.strategy.clone(),
    })
}

fn eval_metrics_window(series: &CandidateSeries<'_>, start_t: usize, end_t: usize) -> Metrics {
    let n = series.n;
    let end_t = end_t.min(n);
    if end_t < 2 || start_t >= end_t {
        return Metrics::default();
    }

    let lr = &series.lr[..end_t];
    let close = &series.close[..end_t];
    let fast = &series.fast[..end_t];
    let slow = &series.slow[..end_t];
    let ht2 = series.halftrend_trend.as_ref().map(|h| &h[..end_t]);
    crate::double_ma::eval_metrics_f64_with_exit_mode(
        lr,
        close,
        fast,
        slow,
        ht2,
        start_t,
        &series.cfg,
        series.kaufman_stops.as_ref(),
        true,
    )
}

fn trades_for_series(series: &CandidateSeries<'_>) -> Vec<Trade> {
    crate::double_ma::eval_curves_f64_with_exit_mode(
        series.lr,
        series.close,
        &series.fast,
        &series.slow,
        series.halftrend_trend.as_deref(),
        series.start_t,
        &series.cfg,
        1,
        series.kaufman_stops.as_ref(),
    )
    .trades
}

fn desired_positions_for_series(
    series: &CandidateSeries<'_>,
    start_t: usize,
    end_t: usize,
) -> Vec<i32> {
    let n = series.n;
    let mut out = crate::double_ma::desired_positions_with_exit_mode(
        series.close,
        &series.fast,
        &series.slow,
        series.halftrend_trend.as_deref(),
        start_t,
        &series.cfg,
        series.kaufman_stops.as_ref(),
    );
    let end_t = end_t.min(n);
    if end_t < n {
        out[end_t..].fill(0);
    }
    out
}

fn slice_return_stats_for_series(
    series: &CandidateSeries<'_>,
    start_t: usize,
    slice_bars: usize,
    slices: usize,
) -> Vec<SliceReturnStats> {
    let mut out = vec![SliceReturnStats::default(); slices];
    if slices == 0 || slice_bars == 0 {
        return out;
    }

    let end_t = start_t
        .saturating_add(slice_bars.saturating_mul(slices))
        .min(series.n);
    if end_t <= start_t + 1 {
        return out;
    }

    let trade_offset = if series.cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let mut pos: i32 = 0;

    for t in start_t..end_t {
        let slice_idx = (t - start_t) / slice_bars;
        if slice_idx >= slices {
            break;
        }
        let t_sig = t.saturating_sub(trade_offset);
        let f = series.fast[t_sig];
        let s = series.slow[t_sig];

        let mut sign: i32 = 0;
        if f.is_finite() && s.is_finite() {
            let eps = if series.cfg.eps_rel > 0.0 {
                series.cfg.eps_rel * 1.0_f64.max(s.abs())
            } else {
                0.0
            };
            sign = (f > s + eps) as i32 - (f < s - eps) as i32;
            if series.cfg.long_only && sign < 0 {
                sign = 0;
            }
        }

        if let Some(ht) = series.halftrend_trend.as_ref() {
            let v = ht[t_sig];
            if !v.is_finite() {
                sign = 0;
            } else if v < 0.5 {
                sign = sign.max(0);
            } else {
                sign = sign.min(0);
            }
        }

        if sign != pos {
            if pos == 0 || sign == 0 {
                pos = if sign == 0 { 0 } else { sign };
            } else if !series.cfg.allow_flip {
                pos = 0;
            } else {
                pos = sign;
            }
        }

        let lr_t = series.lr[t];
        let step_r = if pos == 0 {
            0.0
        } else {
            (pos as f64) * lr_t.exp_m1()
        };
        out[slice_idx].sum += step_r;
        out[slice_idx].sumsq += step_r * step_r;
    }

    out
}

fn slice_return_sums_for_series(
    series: &CandidateSeries<'_>,
    start_t: usize,
    slice_bars: usize,
    slices: usize,
) -> Vec<f64> {
    slice_return_stats_for_series(series, start_t, slice_bars, slices)
        .into_iter()
        .map(|entry| entry.sum)
        .collect()
}

const DEFAULT_REGIME_SLICE_VOL_WINDOW: usize = 48;
const DEFAULT_REGIME_SLICE_FINALISTS: usize = 5;
const DEFAULT_FRICTION_STRESS_FINALISTS: usize = 4;
const DEFAULT_FRICTION_FEE_FLOOR_BPS: f64 = 2.0;
const DEFAULT_FRICTION_SPREAD_FLOOR_BPS: f64 = 4.0;
const DEFAULT_FRICTION_SLIPPAGE_FLOOR_BPS: f64 = 2.0;

fn compute_step_returns_for_series(
    series: &CandidateSeries<'_>,
) -> (Vec<f64>, Vec<bool>, Vec<bool>, Vec<f64>) {
    let mut step_returns = vec![0.0; series.n];
    let mut active_flags = vec![false; series.n];
    let mut trade_entries = vec![false; series.n];
    let mut spread_ratio = vec![f64::NAN; series.n];
    let trade_offset = if series.cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let mut pos: i32 = 0;

    for t in series.start_t..series.n {
        let slice_idx = t.saturating_sub(trade_offset);
        let f = series.fast[slice_idx];
        let s = series.slow[slice_idx];
        let close = series.close[slice_idx].abs().max(1.0e-9);
        if f.is_finite() && s.is_finite() {
            spread_ratio[t] = (f - s).abs() / close;
        }

        let mut sign: i32 = 0;
        if f.is_finite() && s.is_finite() {
            let eps = if series.cfg.eps_rel > 0.0 {
                series.cfg.eps_rel * 1.0_f64.max(s.abs())
            } else {
                0.0
            };
            sign = (f > s + eps) as i32 - (f < s - eps) as i32;
            if series.cfg.long_only && sign < 0 {
                sign = 0;
            }
        }

        if let Some(ht) = series.halftrend_trend.as_ref() {
            let v = ht[slice_idx];
            if !v.is_finite() {
                sign = 0;
            } else if v < 0.5 {
                sign = sign.max(0);
            } else {
                sign = sign.min(0);
            }
        }

        let prev_pos = pos;
        if sign != pos {
            if pos == 0 || sign == 0 {
                pos = if sign == 0 { 0 } else { sign };
            } else if !series.cfg.allow_flip {
                pos = 0;
            } else {
                pos = sign;
            }
        }
        if pos != 0 && pos != prev_pos {
            trade_entries[t] = true;
        }

        if pos != 0 {
            active_flags[t] = true;
            step_returns[t] = (pos as f64) * series.lr[t].exp_m1();
        }
    }

    (step_returns, active_flags, trade_entries, spread_ratio)
}

fn rolling_volatility_tertiles(lr: &[f64], start_t: usize, window: usize) -> Vec<u8> {
    let n = lr.len();
    let mut states = vec![u8::MAX; n];
    if n == 0 || start_t >= n {
        return states;
    }

    let mut prefix = vec![0.0; n + 1];
    let mut prefix_sq = vec![0.0; n + 1];
    for (idx, value) in lr.iter().copied().enumerate() {
        prefix[idx + 1] = prefix[idx] + value;
        prefix_sq[idx + 1] = prefix_sq[idx] + value * value;
    }

    let mut vols: Vec<f64> = Vec::new();
    let mut raw: Vec<f64> = vec![f64::NAN; n];
    for t in start_t..n {
        let begin = t.saturating_add(1).saturating_sub(window.max(1));
        let count = t.saturating_sub(begin).saturating_add(1);
        if count < window.max(1) {
            continue;
        }
        let sum = prefix[t + 1] - prefix[begin];
        let sumsq = prefix_sq[t + 1] - prefix_sq[begin];
        let mean = sum / count as f64;
        let variance = (sumsq / count as f64 - mean * mean).max(0.0);
        let vol = variance.sqrt();
        raw[t] = vol;
        vols.push(vol);
    }
    if vols.is_empty() {
        return states;
    }
    vols.sort_by(|a, b| a.total_cmp(b));
    let low_thr = crate::stats_common::quantile(&vols, 1.0 / 3.0);
    let high_thr = crate::stats_common::quantile(&vols, 2.0 / 3.0);
    for t in start_t..n {
        let vol = raw[t];
        if !vol.is_finite() {
            continue;
        }
        states[t] = if vol <= low_thr {
            0
        } else if vol <= high_thr {
            1
        } else {
            2
        };
    }
    states
}

fn summarize_regime_bucket(
    step_returns: &[f64],
    active_flags: &[bool],
    trade_entries: &[bool],
    membership: &[bool],
) -> RegimeBucketStats {
    let mut bars = 0usize;
    let mut active_bars = 0usize;
    let mut trades = 0usize;
    let mut eq = 1.0_f64;
    let mut peak = 1.0_f64;
    let mut max_dd = 0.0_f64;
    let mut ulcer_sum_sq = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut samples: Vec<f64> = Vec::new();

    for idx in 0..membership.len() {
        if !membership[idx] {
            continue;
        }
        bars += 1;
        if active_flags[idx] {
            active_bars += 1;
        }
        if trade_entries[idx] {
            trades += 1;
        }
        let r = step_returns[idx];
        samples.push(r);
        eq *= (1.0 + r).max(1.0e-7);
        if eq > peak {
            peak = eq;
        }
        let dd = (peak - eq) / peak.max(1.0e-7);
        if dd > max_dd {
            max_dd = dd;
        }
        ulcer_sum_sq += dd * dd;
        if r < 0.0 {
            sum_neg_sq += r * r;
        }
    }

    if bars == 0 {
        return RegimeBucketStats {
            bars: 0,
            active_frac: 0.0,
            pnl: 0.0,
            sharpe: 0.0,
            downside_deviation: 0.0,
            ulcer_index: 0.0,
            martin_ratio: 0.0,
            max_dd: 0.0,
            trades: 0,
        };
    }

    let mean = if samples.is_empty() {
        0.0
    } else {
        samples.iter().copied().sum::<f64>() / samples.len() as f64
    };
    let std = if samples.len() <= 1 {
        0.0
    } else {
        let sumsq = samples
            .iter()
            .map(|value| {
                let d = *value - mean;
                d * d
            })
            .sum::<f64>();
        (sumsq / (samples.len() - 1) as f64).sqrt()
    };
    let sharpe = if std > 0.0 { mean / std } else { 0.0 };
    let downside_deviation = (sum_neg_sq / bars as f64).sqrt();
    let ulcer_index = (ulcer_sum_sq / bars as f64).sqrt();
    let pnl = eq - 1.0;
    let martin_ratio = if ulcer_index > 1.0e-12 {
        (pnl / ulcer_index).clamp(-1.0e6, 1.0e6)
    } else if pnl > 0.0 {
        1.0e6
    } else if pnl < 0.0 {
        -1.0e6
    } else {
        0.0
    };
    RegimeBucketStats {
        bars,
        active_frac: active_bars as f64 / bars as f64,
        pnl,
        sharpe,
        downside_deviation,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
    }
}

fn derive_regime_support_label(
    trend_pass: bool,
    range_pass: bool,
    stable_bucket_count: usize,
) -> String {
    match (trend_pass, range_pass) {
        (true, true) => "broad".to_string(),
        (true, false) => "trend_only".to_string(),
        (false, true) => "range_only".to_string(),
        (false, false) if stable_bucket_count >= 2 => "vol_selective".to_string(),
        _ => "weak".to_string(),
    }
}

fn compute_regime_slice_robustness_stats(
    series: &CandidateSeries<'_>,
) -> Option<RegimeSliceRobustnessStats> {
    if series.n.saturating_sub(series.start_t) < DEFAULT_REGIME_SLICE_VOL_WINDOW.max(24) {
        return None;
    }

    let (step_returns, active_flags, trade_entries, spread_ratio) =
        compute_step_returns_for_series(series);
    let vol_states =
        rolling_volatility_tertiles(series.lr, series.start_t, DEFAULT_REGIME_SLICE_VOL_WINDOW);

    let mut spreads: Vec<f64> = spread_ratio[series.start_t..]
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if spreads.is_empty() {
        return None;
    }
    spreads.sort_by(|a, b| a.total_cmp(b));
    let spread_threshold = crate::stats_common::median(&spreads);

    let mut low_mask = vec![false; series.n];
    let mut mid_mask = vec![false; series.n];
    let mut high_mask = vec![false; series.n];
    let mut trend_mask = vec![false; series.n];
    let mut range_mask = vec![false; series.n];
    for t in series.start_t..series.n {
        match vol_states[t] {
            0 => low_mask[t] = true,
            1 => mid_mask[t] = true,
            2 => high_mask[t] = true,
            _ => {}
        }
        let spread = spread_ratio[t];
        if spread.is_finite() {
            if spread >= spread_threshold {
                trend_mask[t] = true;
            } else {
                range_mask[t] = true;
            }
        }
    }

    let low_vol = summarize_regime_bucket(&step_returns, &active_flags, &trade_entries, &low_mask);
    let mid_vol = summarize_regime_bucket(&step_returns, &active_flags, &trade_entries, &mid_mask);
    let high_vol =
        summarize_regime_bucket(&step_returns, &active_flags, &trade_entries, &high_mask);
    let trend = summarize_regime_bucket(&step_returns, &active_flags, &trade_entries, &trend_mask);
    let range = summarize_regime_bucket(&step_returns, &active_flags, &trade_entries, &range_mask);
    let trend_pass = trend.sharpe >= 0.0 && trend.pnl >= 0.0;
    let range_pass = range.sharpe >= 0.0 && range.pnl >= 0.0;

    let buckets = [
        ("low_vol", &low_vol),
        ("mid_vol", &mid_vol),
        ("high_vol", &high_vol),
        ("trend", &trend),
        ("range", &range),
    ];
    let mut worst_bucket_label = String::new();
    let mut worst_sharpe = f64::INFINITY;
    let mut worst_ulcer_index = 0.0_f64;
    let mut worst_martin_ratio = f64::INFINITY;
    let mut worst_max_dd = 0.0_f64;
    let mut stable_bucket_count = 0usize;
    for (label, stats) in buckets {
        if stats.bars == 0 {
            continue;
        }
        if stats.sharpe < worst_sharpe {
            worst_sharpe = stats.sharpe;
            worst_bucket_label = label.to_string();
        }
        if stats.max_dd > worst_max_dd {
            worst_max_dd = stats.max_dd;
        }
        if stats.ulcer_index > worst_ulcer_index {
            worst_ulcer_index = stats.ulcer_index;
        }
        if stats.martin_ratio < worst_martin_ratio {
            worst_martin_ratio = stats.martin_ratio;
        }
        if stats.sharpe >= 0.0 && stats.pnl >= 0.0 {
            stable_bucket_count += 1;
        }
    }
    if !worst_sharpe.is_finite() {
        return None;
    }
    if !worst_martin_ratio.is_finite() {
        worst_martin_ratio = 0.0;
    }
    let stable_bucket_frac = stable_bucket_count as f64 / 5.0;
    let support_label = derive_regime_support_label(trend_pass, range_pass, stable_bucket_count);

    Some(RegimeSliceRobustnessStats {
        vol_window: DEFAULT_REGIME_SLICE_VOL_WINDOW,
        low_vol,
        mid_vol,
        high_vol,
        trend,
        range,
        worst_bucket_label,
        worst_sharpe,
        worst_ulcer_index,
        worst_martin_ratio,
        worst_max_dd,
        stable_bucket_count,
        stable_bucket_frac,
        trend_pass,
        range_pass,
        support_label,
    })
}

#[derive(Clone, Copy)]
struct FrictionScenarioSpec {
    label: &'static str,
    fee_bps: f64,
    spread_bps: f64,
    slippage_bps: f64,
}

fn friction_stress_scenarios(base: &StrategyConfig) -> [FrictionScenarioSpec; 4] {
    let fee_stress = (base.fee_bps * 2.0)
        .max(base.fee_bps + 1.0)
        .max(DEFAULT_FRICTION_FEE_FLOOR_BPS);
    let spread_stress = (base.spread_bps * 2.0)
        .max(base.spread_bps + 2.0)
        .max(DEFAULT_FRICTION_SPREAD_FLOOR_BPS);
    let slippage_stress = (base.slippage_bps * 2.0)
        .max(base.slippage_bps + 1.0)
        .max(DEFAULT_FRICTION_SLIPPAGE_FLOOR_BPS);
    [
        FrictionScenarioSpec {
            label: "fee_stress",
            fee_bps: fee_stress,
            spread_bps: base.spread_bps,
            slippage_bps: base.slippage_bps,
        },
        FrictionScenarioSpec {
            label: "spread_stress",
            fee_bps: base.fee_bps,
            spread_bps: spread_stress,
            slippage_bps: base.slippage_bps,
        },
        FrictionScenarioSpec {
            label: "slippage_stress",
            fee_bps: base.fee_bps,
            spread_bps: base.spread_bps,
            slippage_bps: slippage_stress,
        },
        FrictionScenarioSpec {
            label: "combined_stress",
            fee_bps: fee_stress,
            spread_bps: spread_stress,
            slippage_bps: slippage_stress,
        },
    ]
}

fn compute_friction_stress_summaries(
    candles: &vector_ta::utilities::data_loader::Candles,
    lr: &[f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params_req: Option<&HashMap<String, f64>>,
    slow_ma_params_req: Option<&HashMap<String, f64>>,
    finalists: &[DoubleMaValidationParams],
) -> Result<Vec<Option<FrictionStressStats>>, String> {
    if finalists.is_empty() {
        return Ok(Vec::new());
    }

    let scenario_labels =
        friction_stress_scenarios(&StrategyConfig::default()).map(|scenario| scenario.label);
    let scenario_count = scenario_labels.len();
    let mut base_ranks = vec![0usize; finalists.len()];
    let mut scenario_ranks: Vec<Vec<usize>> = vec![vec![0; finalists.len()]; scenario_count];
    let mut scenario_metrics: Vec<Vec<Metrics>> =
        vec![Vec::with_capacity(finalists.len()); scenario_count];
    let mut scenario_scores: Vec<Vec<f64>> = vec![vec![0.0; finalists.len()]; scenario_count];

    let sort_scored = |scored: &mut Vec<(usize, Metrics, f64)>| {
        scored.sort_by(|a, b| {
            b.2.total_cmp(&a.2)
                .then_with(|| b.1.sharpe.total_cmp(&a.1.sharpe))
                .then_with(|| a.1.max_dd.total_cmp(&b.1.max_dd))
        });
    };

    let mut base_scored: Vec<(usize, Metrics, f64)> = Vec::with_capacity(finalists.len());
    for (idx, params) in finalists.iter().enumerate() {
        let series = build_candidate_series(
            candles,
            lr,
            base_first_valid,
            ma_source,
            fast_ma_params_req,
            slow_ma_params_req,
            params,
        )?;
        let metrics = eval_metrics_window(&series, series.start_t, series.n);
        let score = double_ma_validation_robust_score(&metrics, None);
        base_scored.push((idx, metrics, score));
    }
    sort_scored(&mut base_scored);
    for (rank_idx, (finalist_idx, _, _)) in base_scored.into_iter().enumerate() {
        base_ranks[finalist_idx] = rank_idx + 1;
    }

    for scenario_idx in 0..scenario_count {
        let mut scored: Vec<(usize, Metrics, f64)> = Vec::with_capacity(finalists.len());
        for (idx, params) in finalists.iter().enumerate() {
            let scenario = friction_stress_scenarios(&params.strategy)[scenario_idx];
            let mut stressed = params.clone();
            stressed.strategy.fee_bps = scenario.fee_bps;
            stressed.strategy.spread_bps = scenario.spread_bps;
            stressed.strategy.slippage_bps = scenario.slippage_bps;
            let series = build_candidate_series(
                candles,
                lr,
                base_first_valid,
                ma_source,
                fast_ma_params_req,
                slow_ma_params_req,
                &stressed,
            )?;
            let metrics = eval_metrics_window(&series, series.start_t, series.n);
            let score = double_ma_validation_robust_score(&metrics, None);
            scored.push((idx, metrics, score));
        }
        sort_scored(&mut scored);
        let mut metrics_by_finalist: Vec<Option<Metrics>> = vec![None; finalists.len()];
        let mut scores_by_finalist = vec![0.0; finalists.len()];
        for (rank_idx, (finalist_idx, metrics, score)) in scored.into_iter().enumerate() {
            scenario_ranks[scenario_idx][finalist_idx] = rank_idx + 1;
            metrics_by_finalist[finalist_idx] = Some(metrics);
            scores_by_finalist[finalist_idx] = score;
        }
        scenario_metrics[scenario_idx] = metrics_by_finalist
            .into_iter()
            .map(|metrics| metrics.expect("friction metrics for each finalist"))
            .collect();
        scenario_scores[scenario_idx] = scores_by_finalist;
    }

    let mut out = Vec::with_capacity(finalists.len());
    for finalist_idx in 0..finalists.len() {
        let mut pass_count = 0usize;
        let mut worst_label = String::new();
        let mut worst_score = f64::INFINITY;
        let mut worst_pnl = 0.0_f64;
        let mut worst_sharpe = 0.0_f64;
        let mut worst_downside_deviation = 0.0_f64;
        let mut worst_ulcer_index = 0.0_f64;
        let mut worst_martin_ratio = 0.0_f64;
        let mut worst_max_dd = 0.0_f64;
        let mut rank_sum = 0usize;
        let mut max_rank_drift = 0usize;
        for scenario_idx in 0..scenario_count {
            let metrics = &scenario_metrics[scenario_idx][finalist_idx];
            if metrics.pnl >= 0.0 && metrics.sharpe >= 0.0 {
                pass_count += 1;
            }
            let scenario_score = scenario_scores[scenario_idx][finalist_idx];
            if scenario_score < worst_score {
                worst_score = scenario_score;
                worst_pnl = metrics.pnl;
                worst_sharpe = metrics.sharpe;
                worst_downside_deviation = metrics.downside_deviation;
                worst_ulcer_index = metrics.ulcer_index;
                worst_martin_ratio = metrics.martin_ratio;
                worst_max_dd = metrics.max_dd;
                worst_label = scenario_labels[scenario_idx].to_string();
            }
            let rank = scenario_ranks[scenario_idx][finalist_idx];
            rank_sum += rank;
            let drift = rank.abs_diff(base_ranks[finalist_idx]);
            if drift > max_rank_drift {
                max_rank_drift = drift;
            }
        }
        out.push(Some(FrictionStressStats {
            scenario_count,
            pass_count,
            pass_frac: pass_count as f64 / scenario_count as f64,
            worst_scenario_label: worst_label,
            worst_pnl,
            worst_sharpe,
            worst_downside_deviation,
            worst_ulcer_index,
            worst_martin_ratio,
            worst_max_dd,
            mean_rank: rank_sum as f64 / scenario_count as f64,
            max_rank_drift,
        }));
    }
    Ok(out)
}

fn benchmark_slice_returns(
    lr: &[f64],
    start_t: usize,
    slice_bars: usize,
    slices: usize,
    benchmark: &SpaBenchmarkKind,
) -> Vec<f64> {
    let mut out = vec![0.0; slices];
    if slices == 0 || slice_bars == 0 {
        return out;
    }
    let end_t = start_t
        .saturating_add(slice_bars.saturating_mul(slices))
        .min(lr.len());
    for t in start_t..end_t {
        let slice_idx = (t - start_t) / slice_bars;
        if slice_idx >= slices {
            break;
        }
        out[slice_idx] += match benchmark {
            SpaBenchmarkKind::Flat => 0.0,
            SpaBenchmarkKind::BuyAndHold => lr[t].exp_m1(),
        };
    }
    out
}

fn repair_params(p: &mut DoubleMaValidationParams) {
    // MA lengths
    if p.fast_len == 0 {
        p.fast_len = 1;
    }
    if p.slow_len == 0 {
        p.slow_len = 1;
    }
    if p.fast_len >= p.slow_len {
        p.slow_len = p.fast_len.saturating_add(1).max(2);
    }

    // Strategy config
    if !p.strategy.fee_bps.is_finite() || p.strategy.fee_bps < 0.0 {
        p.strategy.fee_bps = 0.0;
    }
    if !p.strategy.spread_bps.is_finite() || p.strategy.spread_bps < 0.0 {
        p.strategy.spread_bps = 0.0;
    }
    if !p.strategy.slippage_bps.is_finite() || p.strategy.slippage_bps < 0.0 {
        p.strategy.slippage_bps = 0.0;
    }
    if !p.strategy.commission.is_finite() {
        p.strategy.commission = p.strategy.commission_from_bps();
    }
    p.strategy.commission = p.strategy.commission.clamp(0.0, 0.999_999);
    if !p.strategy.eps_rel.is_finite() || p.strategy.eps_rel < 0.0 {
        p.strategy.eps_rel = 0.0;
    }
    if !p.strategy.trend_quality_angle_min_deg.is_finite()
        || p.strategy.trend_quality_angle_min_deg <= 0.0
    {
        p.strategy.trend_quality_angle_min_deg = 2.0;
    }
    if !p.strategy.trend_quality_bbw_mult.is_finite() || p.strategy.trend_quality_bbw_mult <= 0.0 {
        p.strategy.trend_quality_bbw_mult = 2.0;
    }
    if !p.strategy.trend_quality_bbw_min.is_finite() || p.strategy.trend_quality_bbw_min < 0.0 {
        p.strategy.trend_quality_bbw_min = 0.04;
    }
    if p.strategy.trend_quality_linearreg_period == 0 {
        p.strategy.trend_quality_linearreg_period = 1;
    }
    if p.strategy.trend_quality_bbw_period == 0 {
        p.strategy.trend_quality_bbw_period = 1;
    }
    if p.strategy.trend_quality_roc_period == 0 {
        p.strategy.trend_quality_roc_period = 1;
    }
    if p.strategy.trend_quality_adx_period == 0 {
        p.strategy.trend_quality_adx_period = 1;
    }
    if !p.strategy.trend_quality_adx_min.is_finite() || p.strategy.trend_quality_adx_min <= 0.0 {
        p.strategy.trend_quality_adx_min = 20.0;
    }

    // HalfTrend filter (optional)
    if let Some(f) = p.halftrend_filter.as_mut() {
        if f.amplitude == 0 {
            f.amplitude = 1;
        }
        if f.atr_period == 0 {
            f.atr_period = 1;
        }
        if !f.channel_deviation.is_finite() || f.channel_deviation <= 0.0 {
            f.channel_deviation = 1.0;
        }
    }

    // MA params: drop non-finite values and clamp ALMA.
    let clamp_alma = |ma_type: &str, map: &mut HashMap<String, f64>| {
        if !ma_type.trim().eq_ignore_ascii_case("alma") {
            return;
        }
        let off = map.get("offset").copied().unwrap_or(0.85);
        let sig = map.get("sigma").copied().unwrap_or(6.0);
        map.insert(
            "offset".to_string(),
            if off.is_finite() {
                off.clamp(0.0, 1.0)
            } else {
                0.85
            },
        );
        map.insert(
            "sigma".to_string(),
            if sig.is_finite() && sig > 0.0 {
                sig
            } else {
                6.0
            },
        );
    };

    if let Some(m) = p.fast_ma_params.as_mut() {
        m.retain(|_, v| v.is_finite());
        if m.is_empty() {
            p.fast_ma_params = None;
        } else {
            clamp_alma(&p.fast_ma_type, m);
        }
    }
    if let Some(m) = p.slow_ma_params.as_mut() {
        m.retain(|_, v| v.is_finite());
        if m.is_empty() {
            p.slow_ma_params = None;
        } else {
            clamp_alma(&p.slow_ma_type, m);
        }
    }
}

fn apply_pct_u16(v: u16, mult: f64) -> u16 {
    let x = (v as f64) * mult;
    if !x.is_finite() {
        return v;
    }
    x.round().clamp(1.0, u16::MAX as f64) as u16
}

fn apply_pct_f64(v: f64, mult: f64) -> f64 {
    let x = v * mult;
    if x.is_finite() {
        x
    } else {
        v
    }
}

fn rand_mult(rng: &mut StdRng, pct_min: f64, pct_max: f64) -> f64 {
    let pct = if pct_max > 0.0 {
        rng.gen_range(pct_min..=pct_max)
    } else {
        0.0
    };
    let sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
    1.0 + sign * pct
}

fn perturb_params(
    base: &DoubleMaValidationParams,
    cfg: &PerturbConfig,
    rng: &mut StdRng,
) -> DoubleMaValidationParams {
    let mut p = base.clone();

    let pct_min = cfg.pct_min.abs();
    let pct_max = cfg.pct_max.abs().max(pct_min);
    let flip_bool_prob = cfg.flip_bool_prob.clamp(0.0, 1.0);

    p.fast_len = apply_pct_u16(p.fast_len, rand_mult(rng, pct_min, pct_max));
    p.slow_len = apply_pct_u16(p.slow_len, rand_mult(rng, pct_min, pct_max));

    if let Some(f) = p.halftrend_filter.as_mut() {
        f.amplitude = apply_pct_u16(f.amplitude, rand_mult(rng, pct_min, pct_max));
        f.atr_period = apply_pct_u16(f.atr_period, rand_mult(rng, pct_min, pct_max));
        f.channel_deviation = apply_pct_f64(f.channel_deviation, rand_mult(rng, pct_min, pct_max));
    }

    p.strategy.fee_bps = apply_pct_f64(p.strategy.fee_bps, rand_mult(rng, pct_min, pct_max)).abs();
    p.strategy.spread_bps =
        apply_pct_f64(p.strategy.spread_bps, rand_mult(rng, pct_min, pct_max)).abs();
    p.strategy.slippage_bps =
        apply_pct_f64(p.strategy.slippage_bps, rand_mult(rng, pct_min, pct_max)).abs();
    p.strategy.commission = apply_pct_f64(p.strategy.commission, rand_mult(rng, pct_min, pct_max));
    p.strategy.eps_rel = apply_pct_f64(p.strategy.eps_rel, rand_mult(rng, pct_min, pct_max)).abs();
    if p.strategy.uses_trend_quality_filter() {
        p.strategy.trend_quality_linearreg_period = apply_pct_u16(
            p.strategy.trend_quality_linearreg_period,
            rand_mult(rng, pct_min, pct_max),
        );
        p.strategy.trend_quality_angle_min_deg = apply_pct_f64(
            p.strategy.trend_quality_angle_min_deg,
            rand_mult(rng, pct_min, pct_max),
        )
        .abs();
        p.strategy.trend_quality_bbw_period = apply_pct_u16(
            p.strategy.trend_quality_bbw_period,
            rand_mult(rng, pct_min, pct_max),
        );
        p.strategy.trend_quality_bbw_mult = apply_pct_f64(
            p.strategy.trend_quality_bbw_mult,
            rand_mult(rng, pct_min, pct_max),
        )
        .abs();
        p.strategy.trend_quality_bbw_min = apply_pct_f64(
            p.strategy.trend_quality_bbw_min,
            rand_mult(rng, pct_min, pct_max),
        )
        .abs();
        p.strategy.trend_quality_roc_period = apply_pct_u16(
            p.strategy.trend_quality_roc_period,
            rand_mult(rng, pct_min, pct_max),
        );
        if rng.gen_bool(flip_bool_prob) {
            p.strategy.trend_quality_use_roc = !p.strategy.trend_quality_use_roc;
        }
        p.strategy.trend_quality_adx_period = apply_pct_u16(
            p.strategy.trend_quality_adx_period,
            rand_mult(rng, pct_min, pct_max),
        );
        p.strategy.trend_quality_adx_min = apply_pct_f64(
            p.strategy.trend_quality_adx_min,
            rand_mult(rng, pct_min, pct_max),
        )
        .abs();
        if rng.gen_bool(flip_bool_prob) {
            p.strategy.trend_quality_use_adx = !p.strategy.trend_quality_use_adx;
        }
    }
    if rng.gen_bool(flip_bool_prob) {
        p.strategy.long_only = !p.strategy.long_only;
    }
    if rng.gen_bool(flip_bool_prob) {
        p.strategy.allow_flip = !p.strategy.allow_flip;
    }

    repair_params(&mut p);
    p
}

#[allow(clippy::too_many_arguments)]
fn compute_cpu_perturb_stats_for_candidate(
    candles: &vector_ta::utilities::data_loader::Candles,
    base_first_valid: usize,
    lr: &[f64],
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    base: &DoubleMaValidationParams,
    cfg: &PerturbConfig,
    cancel: &AtomicBool,
    rng: &mut StdRng,
) -> Result<Option<PerturbStats>, String> {
    crate::validation_pipeline::compute_perturb_stats(
        base,
        cfg,
        cancel,
        rng,
        perturb_params,
        |pp| {
            let s2 = build_candidate_series(
                candles,
                lr,
                base_first_valid,
                ma_source,
                fast_ma_params,
                slow_ma_params,
                pp,
            )?;
            Ok(eval_metrics_window(&s2, s2.start_t, s2.n))
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn compute_trade_bootstrap_for_candidate(
    candles: &vector_ta::utilities::data_loader::Candles,
    lr: &[f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    params: &DoubleMaValidationParams,
    cfg: &TradeBootstrapConfig,
    rng: &mut StdRng,
) -> Result<Option<TradeBootstrapStats>, String> {
    if cfg.simulations == 0 {
        return Ok(None);
    }
    let series = build_candidate_series(
        candles,
        lr,
        base_first_valid,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        params,
    )?;
    let trades = trades_for_series(&series);
    Ok(Some(
        crate::validation_pipeline::compute_trade_bootstrap_stats(&trades, cfg, rng),
    ))
}

pub fn run_double_ma_validation_blocking_with_candles(
    mut req: DoubleMaValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<DoubleMaValidationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let start = std::time::Instant::now();

    if req.max_candidates == 0 {
        req.max_candidates = default_max_candidates();
    }
    if req.candidates.len() > req.max_candidates {
        req.candidates.truncate(req.max_candidates);
    }
    if req.candidates.is_empty() {
        return Err("no candidates to validate".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let base_first_valid = fv_close.max(fv_ma);
    let lr = compute_log_returns_f64(close);

    let total = req.candidates.len();
    emit_progress(progress, 0, total, "double_ma_validate");

    let seed = req.seed;
    let walk_cfg = req.walk_forward.clone();
    let perturb_cfg = req.perturb.clone();
    let mc_cfg = req.trade_bootstrap.clone();
    let ma_source = req.ma_source.clone();
    let fast_ma_params = req.fast_ma_params.clone();
    let slow_ma_params = req.slow_ma_params.clone();
    let cscv_cfg = req.cscv_pbo.clone();
    let validation_engine = req.validation_engine;
    let iterative_cfg = req.iterative.clone();
    let reopt_walk_cfg = req.reopt_walk_forward.clone();
    let spa_cfg = req.spa.clone();
    let mut gpu_validation_fallback_reason: Option<String> = None;
    let gpu_fixed_eval = match validation_supports_gpu_fixed_candidates(&req) {
        Ok(_) => {
            match try_prepare_gpu_fixed_candidate_validation(&req, candles, base_first_valid) {
                Ok(eval) => Some(eval),
                Err(err) => {
                    gpu_validation_fallback_reason = Some(err);
                    None
                }
            }
        }
        Err(reason) => {
            if !matches!(req.backend, Backend::CpuOnly) {
                gpu_validation_fallback_reason = Some(reason);
            }
            None
        }
    };
    let hard_gates = sanitize_hard_gates(
        req.min_psr,
        req.min_dsr,
        req.min_cscv_oos_rank_frac,
        req.min_cscv_oos_metric,
        req.max_pbo,
    );

    let (mut out, cscv_pbo) = crate::validation_core::run_validation_candidates(
        &req.candidates,
        seed,
        cancel,
        progress,
        "double_ma_validate",
        "double_ma_validate_cscv_prep",
        &hard_gates,
        cscv_cfg.as_ref(),
        candles.close.len(),
        |idx, p, rng| {
            if let Some(gpu_eval) = gpu_fixed_eval.as_ref() {
                let in_sample = gpu_eval.metrics[idx].clone();
                let series_start_t = gpu_eval.start_ts[idx];
                let walk_forward = gpu_eval
                    .walk_forwards
                    .as_ref()
                    .and_then(|stats| stats.get(idx).cloned());
                let robust_score =
                    double_ma_validation_robust_score(&in_sample, walk_forward.as_ref());
                let perturb = if let Some(cfg) = perturb_cfg.as_ref() {
                    compute_cpu_perturb_stats_for_candidate(
                        candles,
                        base_first_valid,
                        &lr,
                        &ma_source,
                        fast_ma_params.as_ref(),
                        slow_ma_params.as_ref(),
                        p,
                        cfg,
                        cancel,
                        rng,
                    )?
                } else {
                    None
                };
                let trade_bootstrap = if let Some(cfg) = mc_cfg.as_ref() {
                    compute_trade_bootstrap_for_candidate(
                        candles,
                        &lr,
                        base_first_valid,
                        &ma_source,
                        fast_ma_params.as_ref(),
                        slow_ma_params.as_ref(),
                        p,
                        cfg,
                        rng,
                    )?
                } else {
                    None
                };
                return Ok((
                    DoubleMaValidationCandidateResult {
                        params: p.clone(),
                        in_sample,
                        iterative_in_sample: None,
                        iterative_delta: None,
                        psr: None,
                        dsr: None,
                        track_record_bars: 0,
                        min_track_record_length_bars: None,
                        walk_forward,
                        perturb,
                        trade_bootstrap,
                        regime_slice_robustness: None,
                        friction_stress: None,
                        cross_ticker: None,
                        cross_ticker_pareto: None,
                        cscv_pbo: None,
                        validation_verdict: ValidationCandidateVerdict::default(),
                        robust_score,
                        robust_pareto: false,
                    },
                    series_start_t,
                    n.saturating_sub(series_start_t),
                ));
            }
            let series = build_candidate_series(
                candles,
                &lr,
                base_first_valid,
                &ma_source,
                fast_ma_params.as_ref(),
                slow_ma_params.as_ref(),
                p,
            )?;
            let series_start_t = series.start_t;

            let in_sample = eval_metrics_window(&series, series.start_t, series.n);

            let walk_forward = if let Some(cfg) = walk_cfg.as_ref() {
                Some(crate::validation_pipeline::compute_walk_forward_stats(
                    series.n,
                    series.start_t,
                    cfg,
                    |t0, end| eval_metrics_window(&series, t0, end),
                ))
            } else {
                None
            };

            let perturb = if let Some(cfg) = perturb_cfg.as_ref() {
                crate::validation_pipeline::compute_perturb_stats(
                    p,
                    cfg,
                    cancel,
                    rng,
                    perturb_params,
                    |pp| {
                        let s2 = build_candidate_series(
                            candles,
                            &lr,
                            base_first_valid,
                            &ma_source,
                            fast_ma_params.as_ref(),
                            slow_ma_params.as_ref(),
                            pp,
                        )?;
                        Ok(eval_metrics_window(&s2, s2.start_t, s2.n))
                    },
                )?
            } else {
                None
            };

            let trade_bootstrap = if let Some(cfg) = mc_cfg.as_ref() {
                let trades = trades_for_series(&series);
                Some(crate::validation_pipeline::compute_trade_bootstrap_stats(
                    &trades, cfg, rng,
                ))
            } else {
                None
            };

            let (iterative_in_sample, iterative_delta) =
                crate::validation_pipeline::maybe_run_iterative_validation(
                    validation_engine,
                    &in_sample,
                    series.lr,
                    crate::iterative_sim::OhlcSlice {
                        open: &candles.open,
                        high: &candles.high,
                        low: &candles.low,
                        close: &candles.close,
                    },
                    series.start_t,
                    &series.cfg,
                    &iterative_cfg,
                    || desired_positions_for_series(&series, series.start_t, series.n),
                );

            let robust_base = iterative_in_sample.as_ref().unwrap_or(&in_sample);
            let robust_score =
                double_ma_validation_robust_score(robust_base, walk_forward.as_ref());

            Ok((
                DoubleMaValidationCandidateResult {
                    params: p.clone(),
                    in_sample,
                    iterative_in_sample,
                    iterative_delta,
                    psr: None,
                    dsr: None,
                    track_record_bars: 0,
                    min_track_record_length_bars: None,
                    walk_forward,
                    perturb,
                    trade_bootstrap,
                    regime_slice_robustness: None,
                    friction_stress: None,
                    cross_ticker: None,
                    cross_ticker_pareto: None,
                    cscv_pbo: None,
                    validation_verdict: ValidationCandidateVerdict::default(),
                    robust_score,
                    robust_pareto: false,
                },
                series_start_t,
                series.n.saturating_sub(series_start_t),
            ))
        },
        |cand, cscv_start_t, slice_bars, slices| {
            if let Some(gpu_eval) = gpu_fixed_eval.as_ref() {
                if let Some(slice_map) = gpu_eval.cscv_slices.as_ref() {
                    if let Some(slice_stats) =
                        slice_map.get(&(cand.params.fast_len, cand.params.slow_len))
                    {
                        return Ok(slice_stats.clone());
                    }
                }
            }
            let series = build_candidate_series(
                candles,
                &lr,
                base_first_valid,
                &ma_source,
                fast_ma_params.as_ref(),
                slow_ma_params.as_ref(),
                &cand.params,
            )?;
            Ok(slice_return_stats_for_series(
                &series,
                cscv_start_t,
                slice_bars,
                slices,
            ))
        },
    )?;

    let regime_eval_count = DEFAULT_REGIME_SLICE_FINALISTS.min(out.len());
    for cand in out.iter_mut().take(regime_eval_count) {
        let series = build_candidate_series(
            candles,
            &lr,
            base_first_valid,
            &ma_source,
            fast_ma_params.as_ref(),
            slow_ma_params.as_ref(),
            &cand.params,
        )?;
        cand.regime_slice_robustness = compute_regime_slice_robustness_stats(&series);
    }

    let friction_eval_count = DEFAULT_FRICTION_STRESS_FINALISTS.min(out.len());
    if friction_eval_count > 0 {
        let finalist_params: Vec<DoubleMaValidationParams> = out
            .iter()
            .take(friction_eval_count)
            .map(|cand| cand.params.clone())
            .collect();
        let summaries = compute_friction_stress_summaries(
            candles,
            &lr,
            base_first_valid,
            &ma_source,
            fast_ma_params.as_ref(),
            slow_ma_params.as_ref(),
            &finalist_params,
        )?;
        for (cand, summary) in out
            .iter_mut()
            .take(friction_eval_count)
            .zip(summaries.into_iter())
        {
            cand.friction_stress = summary;
        }
    }

    let reopt_walk_forward = if let Some(cfg) = reopt_walk_cfg.as_ref() {
        let finalist_params: Vec<DoubleMaValidationParams> = out
            .iter()
            .take(cfg.top_n_candidates.max(1))
            .map(|cand| cand.params.clone())
            .collect();
        if gpu_fixed_eval.is_some() {
            let finalist_start_ts: Vec<usize> = out
                .iter()
                .take(cfg.top_n_candidates.max(1))
                .map(|cand| candles.close.len().saturating_sub(cand.track_record_bars))
                .collect();
            compute_gpu_reopt_walk_forward_stats_for_candidates(
                &req,
                candles,
                &finalist_params,
                &finalist_start_ts,
                fast_ma_params.as_ref(),
                slow_ma_params.as_ref(),
                cfg,
            )?
        } else {
            let mut finalist_series: Vec<(String, CandidateSeries<'_>)> =
                Vec::with_capacity(finalist_params.len());
            for params in finalist_params {
                let key = serde_json::to_string(&params).unwrap_or_else(|_| {
                    format!(
                        "{}:{}:{}:{}",
                        params.fast_ma_type, params.fast_len, params.slow_ma_type, params.slow_len
                    )
                });
                finalist_series.push((
                    key,
                    build_candidate_series(
                        candles,
                        &lr,
                        base_first_valid,
                        &ma_source,
                        fast_ma_params.as_ref(),
                        slow_ma_params.as_ref(),
                        &params,
                    )?,
                ));
            }
            crate::validation_pipeline::compute_reopt_walk_forward_stats(
                &finalist_series,
                candles.close.len(),
                cfg,
                |(_, series)| series.start_t,
                |(_, series), t0, end| eval_metrics_window(series, t0, end),
                |(key, _)| key.clone(),
            )
        }
    } else {
        None
    };
    let spa = if let Some(cfg) = spa_cfg.as_ref() {
        let common_start_t = out
            .iter()
            .map(|cand| candles.close.len().saturating_sub(cand.track_record_bars))
            .max()
            .unwrap_or(0);
        let slices = cfg.slices.max(4);
        let slice_bars = candles
            .close
            .len()
            .saturating_sub(common_start_t)
            .checked_div(slices)
            .unwrap_or(0);
        if slice_bars >= 2 {
            let benchmark =
                benchmark_slice_returns(&lr, common_start_t, slice_bars, slices, &cfg.benchmark);
            let mut candidate_keys: Vec<String> = Vec::with_capacity(out.len());
            let mut candidate_slice_returns: Vec<Vec<f64>> = Vec::with_capacity(out.len());
            for cand in &out {
                let params = cand.params.clone();
                let key = serde_json::to_string(&params).unwrap_or_else(|_| {
                    format!(
                        "{}:{}:{}:{}",
                        params.fast_ma_type, params.fast_len, params.slow_ma_type, params.slow_len
                    )
                });
                candidate_keys.push(key);
                if let Some(gpu_eval) = gpu_fixed_eval.as_ref() {
                    if let Some(slice_map) = gpu_eval.spa_slice_sums.as_ref() {
                        if let Some(slice_sums) = slice_map.get(&(params.fast_len, params.slow_len))
                        {
                            candidate_slice_returns.push(slice_sums.clone());
                            continue;
                        }
                    }
                }
                let series = build_candidate_series(
                    candles,
                    &lr,
                    base_first_valid,
                    &ma_source,
                    fast_ma_params.as_ref(),
                    slow_ma_params.as_ref(),
                    &params,
                )?;
                candidate_slice_returns.push(slice_return_sums_for_series(
                    &series,
                    common_start_t,
                    slice_bars,
                    slices,
                ));
            }
            crate::validation_pipeline::compute_spa_validation_stats(
                &candidate_keys,
                &candidate_slice_returns,
                &benchmark,
                cfg,
                req.seed,
                cancel,
            )
        } else {
            Some(SpaValidationStats {
                scope: "validation_candidates".to_string(),
                candidate_count: out.len(),
                slice_count: slices,
                bootstrap_samples: cfg.bootstrap_samples.max(1),
                block_length: cfg.block_length.max(1),
                benchmark: cfg.benchmark.clone(),
                best_candidate_key: String::new(),
                best_candidate_mean_diff: 0.0,
                best_candidate_t_stat: 0.0,
                p_value: 1.0,
                passed: false,
                skipped_reason: Some("spa_skipped_not_enough_bars_after_warmup".to_string()),
            })
        }
    } else {
        None
    };

    let runtime_ms = start.elapsed().as_millis() as u64;
    let run_verdict = build_run_verdict(
        &out,
        req.candidates.len(),
        &hard_gates,
        cscv_pbo.as_ref(),
        spa.as_ref(),
    );
    let (backend_used, backend_route, backend_fallback_reason) =
        if let Some(gpu_eval) = gpu_fixed_eval.as_ref() {
            (
                gpu_eval.backend_used.clone(),
                gpu_eval.backend_route.clone(),
                gpu_eval.backend_fallback_reason.clone(),
            )
        } else {
            resolve_validation_backend_runtime(&req.backend, false, gpu_validation_fallback_reason)
        };
    Ok(DoubleMaValidationResult {
        data_id: req.data_id,
        runtime_ms,
        backend_used,
        backend_route,
        backend_fallback_reason,
        candidates: out,
        reopt_walk_forward,
        spa,
        cross_ticker: None,
        cscv_pbo,
        run_verdict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::double_ma::{
        run_double_ma_optimization_blocking_with_candles, Backend, DoubleMaRequest,
    };
    use crate::meta_select::{
        meta_select_from_artifacts, MetaClusterConfig, MetaGroupingMode, MetaHoldoutMode,
        MetaScoreWeights, MetaSelectionRequest,
    };
    use serde_json::json;
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::ExitMode;

    fn make_trending_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        make_trending_candles_with_slope(len, 0.1)
    }

    fn make_trending_candles_with_slope(
        len: usize,
        slope: f64,
    ) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64 * slope;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.25);
            vol.push(1_000.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn make_choppy_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        let mut prev_close: f64 = 100.0;
        for i in 0..len {
            ts.push(i as i64);
            let x = i as f64;
            let wave = (x * 0.11).sin() * 2.5 + (x * 0.037).cos() * 1.4;
            let drift = x * 0.012;
            let c: f64 = 100.0 + drift + wave;
            let o: f64 = prev_close;
            let hi_pad = 0.9 + (x * 0.17).sin().abs() * 0.4;
            let lo_pad = 0.9 + (x * 0.13).cos().abs() * 0.4;
            let h = o.max(c) + hi_pad;
            let l = o.min(c) - lo_pad;
            open.push(o);
            high.push(h);
            low.push(l);
            close.push(c);
            vol.push(1_000.0 + (x * 0.09).sin().abs() * 300.0);
            prev_close = c;
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn make_breakout_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        let up_start = len / 3;
        let down_start = (len * 2) / 3;
        for i in 0..len {
            ts.push(i as i64);
            let base = if i < up_start {
                100.0 + i as f64 * 0.02
            } else if i < down_start {
                102.0 + (i - up_start) as f64 * 0.35
            } else {
                102.0 + (down_start - up_start) as f64 * 0.35 - (i - down_start) as f64 * 0.38
            };
            let n = ((i as f64) / 7.0).sin() * 0.08;
            let o = base + n;
            let c = if i < down_start {
                base + 0.30 + n
            } else {
                base - 0.30 + n
            };
            open.push(o);
            close.push(c);
            high.push(o.max(c) + 0.22);
            low.push(o.min(c) - 0.22);
            let burst = if i < up_start {
                950.0
            } else if i < down_start {
                2_200.0
            } else {
                2_450.0
            };
            vol.push(burst + ((i % 11) as f64) * 35.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn double_ma_validation_smoke_runs_and_returns_stats() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let base = DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
        };

        let req = DoubleMaValidationRequest {
            data_id: "test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 2000,
                step_bars: 1000,
            }),
            reopt_walk_forward: None,
            perturb: Some(PerturbConfig {
                iterations: 3,
                pct_min: 0.01,
                pct_max: 0.02,
                flip_bool_prob: 0.02,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 50,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert_eq!(res.candidates.len(), 1);
        let c = &res.candidates[0];
        assert!(c.in_sample.pnl.is_finite());
        assert!(c.in_sample.sharpe.is_finite());
        assert!(c.in_sample.max_dd.is_finite());
        assert!(c.walk_forward.as_ref().unwrap().windows > 0);
        assert_eq!(c.perturb.as_ref().unwrap().iterations, 3);
        assert_eq!(c.trade_bootstrap.as_ref().unwrap().simulations, 50);
        let bootstrap = c.trade_bootstrap.as_ref().unwrap();
        if bootstrap.trade_count > 0 {
            assert!(bootstrap.stationary_mean_block_len.unwrap_or(0) >= 1);
            assert!(
                bootstrap.stationary_pnl_p05.unwrap_or(0.0)
                    <= bootstrap.stationary_pnl_p50.unwrap_or(0.0)
            );
            assert!(
                bootstrap.stationary_max_dd_p50.unwrap_or(0.0)
                    <= bootstrap.stationary_max_dd_p95.unwrap_or(0.0)
            );
        }
        assert!(c.robust_score.is_finite());
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("CpuOnly"));
        assert!(res.backend_fallback_reason.is_none());
    }

    #[test]
    fn double_ma_validation_populates_regime_slice_robustness_for_top_finalists() {
        let candles = make_choppy_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaValidationRequest {
            data_id: "regime-robustness".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                DoubleMaValidationParams {
                    fast_ma_type: "alma".to_string(),
                    slow_ma_type: "alma".to_string(),
                    fast_len: 9,
                    slow_len: 30,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: StrategyConfig::default(),
                    halftrend_filter: None,
                },
                DoubleMaValidationParams {
                    fast_ma_type: "alma".to_string(),
                    slow_ma_type: "alma".to_string(),
                    fast_len: 12,
                    slow_len: 42,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: StrategyConfig::default(),
                    halftrend_filter: None,
                },
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1500,
                step_bars: 750,
            }),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 7,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert_eq!(res.candidates.len(), 2);
        for cand in &res.candidates {
            let regime = cand
                .regime_slice_robustness
                .as_ref()
                .expect("top finalists should have regime robustness stats");
            assert!(regime.vol_window >= DEFAULT_REGIME_SLICE_VOL_WINDOW);
            assert!(!regime.worst_bucket_label.is_empty());
            assert!(regime.worst_sharpe.is_finite());
            assert!(regime.worst_ulcer_index.is_finite());
            assert!(regime.worst_martin_ratio.is_finite());
            assert!(regime.worst_max_dd.is_finite());
            assert!(regime.stable_bucket_count <= 5);
            assert!((0.0..=1.0).contains(&regime.stable_bucket_frac));
            assert!(!regime.support_label.is_empty());
            assert!(regime.low_vol.downside_deviation.is_finite());
            assert!(regime.low_vol.ulcer_index.is_finite());
            assert!(regime.low_vol.martin_ratio.is_finite());
        }
    }

    #[test]
    fn double_ma_validation_populates_friction_stress_for_top_finalists() {
        let candles = make_choppy_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaValidationRequest {
            data_id: "friction-stress".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                DoubleMaValidationParams {
                    fast_ma_type: "alma".to_string(),
                    slow_ma_type: "alma".to_string(),
                    fast_len: 9,
                    slow_len: 30,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: StrategyConfig::default(),
                    halftrend_filter: None,
                },
                DoubleMaValidationParams {
                    fast_ma_type: "alma".to_string(),
                    slow_ma_type: "alma".to_string(),
                    fast_len: 12,
                    slow_len: 42,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: StrategyConfig::default(),
                    halftrend_filter: None,
                },
            ],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 13,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        for cand in &res.candidates {
            let friction = cand
                .friction_stress
                .as_ref()
                .expect("top finalists should have friction stress stats");
            assert_eq!(friction.scenario_count, 4);
            assert!(friction.pass_frac.is_finite());
            assert!(!friction.worst_scenario_label.is_empty());
            assert!(friction.worst_sharpe.is_finite());
            assert!(friction.worst_downside_deviation.is_finite());
            assert!(friction.worst_ulcer_index.is_finite());
            assert!(friction.worst_martin_ratio.is_finite());
            assert!(friction.worst_max_dd.is_finite());
        }
    }

    #[test]
    fn double_ma_validation_gpu_request_reports_backend_route() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaValidationRequest {
            data_id: "gpu-route".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        {
            assert!(matches!(res.backend_used, BackendUsed::Gpu));
            assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
            assert!(res.backend_fallback_reason.is_none());
        }
        #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
        {
            assert!(matches!(res.backend_used, BackendUsed::Cpu));
            assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
            assert!(res
                .backend_fallback_reason
                .as_deref()
                .unwrap_or_default()
                .contains("requires cuda"));
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_matches_cpu_metrics_plain() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
        ];

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-plain".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu validation");
        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-plain".to_string(),
                backend: Backend::GpuKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!(
                (cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4,
                "pnl mismatch cpu={} gpu={} route={:?}",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl,
                gpu_res.backend_route
            );
            assert!(
                (cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 5e-5,
                "sharpe mismatch cpu={} gpu={} route={:?}",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe,
                gpu_res.backend_route
            );
            assert!(
                (cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 5e-5,
                "max_dd mismatch cpu={} gpu={} route={:?}",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd,
                gpu_res.backend_route
            );
            assert_eq!(
                cpu.track_record_bars, gpu.track_record_bars,
                "track_record mismatch cpu={} gpu={} route={:?}",
                cpu.track_record_bars, gpu.track_record_bars, gpu_res.backend_route
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_direct_cross_ticker_request_falls_back_to_cpu() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaValidationRequest {
            data_id: "gpu-cross-direct".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: Some(CrossTickerConfig {
                datasets: Vec::new(),
                max_datasets: 0,
                mode: crate::composite_validate::CrossTickerMode::Lightweight,
                gate: crate::composite_validate::CrossTickerGateConfig::default(),
            }),
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("cross_ticker"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_iterative_engine_falls_back_to_cpu_with_reason() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaValidationRequest {
            data_id: "gpu-iterative-fallback".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ValidationEngine::Iterative,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("vectorized validation engine"));
        assert!(res.candidates[0].iterative_in_sample.is_some());
        assert!(res.candidates[0].iterative_delta.is_some());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_auto_iterative_engine_falls_back_to_cpu_with_reason() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaValidationRequest {
            data_id: "auto-iterative-fallback".to_string(),
            backend: Backend::Auto { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ValidationEngine::Iterative,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert_eq!(res.backend_route.as_deref(), Some("Auto->CpuOnly"));
        assert!(res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or_default()
            .contains("vectorized validation engine"));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_walk_forward_matches_cpu_plain() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
        ];

        let walk_forward = Some(WalkForwardConfigOptional {
            window_bars: 512,
            step_bars: 256,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-wf".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: walk_forward.clone(),
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu walk_forward validation");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-wf".to_string(),
                backend: Backend::GpuKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu walk_forward validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_wf = cpu.walk_forward.as_ref().expect("cpu walk_forward");
            let gpu_wf = gpu.walk_forward.as_ref().expect("gpu walk_forward");
            assert_eq!(cpu_wf.windows, gpu_wf.windows);
            assert!(
                (cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 5e-2,
                "sharpe_median mismatch cpu={} gpu={}",
                cpu_wf.sharpe_median,
                gpu_wf.sharpe_median
            );
            assert!(
                (cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 5e-2,
                "sharpe_worst mismatch cpu={} gpu={}",
                cpu_wf.sharpe_worst,
                gpu_wf.sharpe_worst
            );
            assert!(
                (cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-5,
                "max_dd_median mismatch cpu={} gpu={}",
                cpu_wf.max_dd_median,
                gpu_wf.max_dd_median
            );
            assert!(
                (cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-5,
                "max_dd_worst mismatch cpu={} gpu={}",
                cpu_wf.max_dd_worst,
                gpu_wf.max_dd_worst
            );
            assert!(
                (cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-3,
                "pnl_median mismatch cpu={} gpu={}",
                cpu_wf.pnl_median,
                gpu_wf.pnl_median
            );
            assert!(
                (cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-3,
                "pnl_worst mismatch cpu={} gpu={}",
                cpu_wf.pnl_worst,
                gpu_wf.pnl_worst
            );
            assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
            assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_walk_forward_matches_cpu_regime_hybrid() {
        let candles = make_choppy_candles(4096);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: false,
            regime_hybrid_enable: true,
            regime_hybrid_adx_period: 14,
            regime_hybrid_adx_enter: 22.0,
            regime_hybrid_adx_exit: 18.0,
            regime_hybrid_reversion_entry_rel: 0.004,
            regime_hybrid_reversion_exit_rel: 0.0015,
            ..StrategyConfig::default()
        };
        let walk_forward = Some(WalkForwardConfigOptional {
            window_bars: 512,
            step_bars: 256,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-hybrid-wf".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![DoubleMaValidationParams {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_len: 10,
                    slow_len: 30,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: strategy.clone(),
                    halftrend_filter: None,
                }],
                walk_forward: walk_forward.clone(),
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu hybrid walk_forward");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-hybrid-wf".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![DoubleMaValidationParams {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_len: 10,
                    slow_len: 30,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: strategy.clone(),
                    halftrend_filter: None,
                }],
                walk_forward,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid walk_forward");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        let cpu_wf = cpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu walk_forward");
        let gpu_wf = gpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu walk_forward");
        assert_eq!(cpu_wf.windows, gpu_wf.windows);
        assert!((cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 5e-2);
        assert!((cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 5e-2);
        assert!((cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-5);
        assert!((cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-5);
        assert!((cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-3);
        assert!((cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-3);
        assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
        assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_matches_cpu_metrics_regime_hybrid() {
        let candles = make_choppy_candles(4096);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: false,
            regime_hybrid_enable: true,
            regime_hybrid_adx_period: 14,
            regime_hybrid_adx_enter: 22.0,
            regime_hybrid_adx_exit: 18.0,
            regime_hybrid_reversion_entry_rel: 0.004,
            regime_hybrid_reversion_exit_rel: 0.0015,
            ..StrategyConfig::default()
        };
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
        ];

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-hybrid".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu validation");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-hybrid".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!(
                (cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4,
                "pnl mismatch cpu={} gpu={} route={:?}",
                cpu.in_sample.pnl,
                gpu.in_sample.pnl,
                gpu_res.backend_route
            );
            assert!(
                (cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 5e-5,
                "sharpe mismatch cpu={} gpu={} route={:?}",
                cpu.in_sample.sharpe,
                gpu.in_sample.sharpe,
                gpu_res.backend_route
            );
            assert!(
                (cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 5e-5,
                "max_dd mismatch cpu={} gpu={} route={:?}",
                cpu.in_sample.max_dd,
                gpu.in_sample.max_dd,
                gpu_res.backend_route
            );
            assert_eq!(
                cpu.track_record_bars, gpu.track_record_bars,
                "track_record mismatch cpu={} gpu={} route={:?}",
                cpu.track_record_bars, gpu.track_record_bars, gpu_res.backend_route
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_matches_cpu_metrics_trend_quality_adx_off() {
        let candles = make_trending_candles_with_slope(4096, 0.07);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_linearreg_period: 34,
            trend_quality_angle_min_deg: 2.0,
            trend_quality_bbw_period: 20,
            trend_quality_bbw_mult: 2.0,
            trend_quality_bbw_min: 0.01,
            trend_quality_roc_period: 9,
            trend_quality_use_roc: true,
            ..StrategyConfig::default()
        };
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
        ];

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-composite-trend".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu validation");
        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-composite-trend".to_string(),
                backend: Backend::GpuKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!((cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4);
            assert!((cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 2e-4);
            assert!((cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-5);
            assert_eq!(cpu.track_record_bars, gpu.track_record_bars);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_matches_cpu_metrics_trend_quality_adx_on() {
        let candles = make_trending_candles_with_slope(4096, 0.07);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_linearreg_period: 34,
            trend_quality_angle_min_deg: 2.0,
            trend_quality_bbw_period: 20,
            trend_quality_bbw_mult: 2.0,
            trend_quality_bbw_min: 0.01,
            trend_quality_roc_period: 9,
            trend_quality_use_roc: true,
            trend_quality_use_adx: true,
            trend_quality_adx_period: 14,
            trend_quality_adx_min: 20.0,
            ..StrategyConfig::default()
        };
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
        ];

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-composite-trend-adx".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu validation");
        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-composite-trend-adx".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!((cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4);
            assert!((cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 2e-4);
            assert!((cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-5);
            assert_eq!(cpu.track_record_bars, gpu.track_record_bars);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_walk_forward_matches_cpu_trend_quality_adx_off() {
        let candles = make_trending_candles_with_slope(4096, 0.07);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_linearreg_period: 34,
            trend_quality_angle_min_deg: 2.0,
            trend_quality_bbw_period: 20,
            trend_quality_bbw_mult: 2.0,
            trend_quality_bbw_min: 0.01,
            trend_quality_roc_period: 9,
            trend_quality_use_roc: true,
            ..StrategyConfig::default()
        };
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
        ];
        let walk_forward = Some(WalkForwardConfigOptional {
            window_bars: 512,
            step_bars: 256,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-composite-trend-wf".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: walk_forward.clone(),
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu walk_forward validation");
        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-composite-trend-wf".to_string(),
                backend: Backend::GpuKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu walk_forward validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_wf = cpu.walk_forward.as_ref().expect("cpu walk_forward");
            let gpu_wf = gpu.walk_forward.as_ref().expect("gpu walk_forward");
            assert_eq!(cpu_wf.windows, gpu_wf.windows);
            assert!((cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 5e-2);
            assert!((cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 5e-2);
            assert!((cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-5);
            assert!((cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-5);
            assert!((cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-3);
            assert!((cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-3);
            assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
            assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_matches_cpu_metrics_breakout_momentum_adx_off() {
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);
        let strategy = crate::strategy_presets::breakout_strategy_for_kernel();
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
        ];

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-breakout".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu validation");
        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-breakout".to_string(),
                backend: Backend::GpuKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!((cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4);
            assert!((cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 2e-5);
            assert!((cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-5);
            assert_eq!(cpu.track_record_bars, gpu.track_record_bars);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_matches_cpu_metrics_breakout_momentum_adx_on() {
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut strategy = crate::strategy_presets::breakout_strategy_for_kernel();
        strategy.breakout_use_adx = true;
        strategy.breakout_adx_period = 14;
        strategy.breakout_adx_min = 20.0;
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
            },
        ];

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-breakout-adx".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu validation");
        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-breakout-adx".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            assert!((cpu.in_sample.pnl - gpu.in_sample.pnl).abs() < 1e-4);
            assert!((cpu.in_sample.sharpe - gpu.in_sample.sharpe).abs() < 2e-4);
            assert!((cpu.in_sample.max_dd - gpu.in_sample.max_dd).abs() < 1e-5);
            assert_eq!(cpu.track_record_bars, gpu.track_record_bars);
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_walk_forward_matches_cpu_trend_quality_adx_on() {
        let candles = make_trending_candles_with_slope(4096, 0.07);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_linearreg_period: 34,
            trend_quality_angle_min_deg: 2.0,
            trend_quality_bbw_period: 20,
            trend_quality_bbw_mult: 2.0,
            trend_quality_bbw_min: 0.01,
            trend_quality_roc_period: 9,
            trend_quality_use_roc: true,
            trend_quality_use_adx: true,
            trend_quality_adx_period: 14,
            trend_quality_adx_min: 20.0,
            ..StrategyConfig::default()
        };
        let walk_forward = Some(WalkForwardConfigOptional {
            window_bars: 512,
            step_bars: 256,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-hybrid-composite-trend-wf".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![DoubleMaValidationParams {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_len: 10,
                    slow_len: 30,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: strategy.clone(),
                    halftrend_filter: None,
                }],
                walk_forward: walk_forward.clone(),
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu hybrid walk_forward");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-hybrid-composite-trend-wf".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![DoubleMaValidationParams {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_len: 10,
                    slow_len: 30,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: strategy.clone(),
                    halftrend_filter: None,
                }],
                walk_forward,
                reopt_walk_forward: None,
                spa: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 1,
                max_candidates: 8,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid walk_forward");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        let cpu_wf = cpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("cpu walk_forward");
        let gpu_wf = gpu_res.candidates[0]
            .walk_forward
            .as_ref()
            .expect("gpu walk_forward");
        assert_eq!(cpu_wf.windows, gpu_wf.windows);
        assert!((cpu_wf.sharpe_median - gpu_wf.sharpe_median).abs() < 5e-2);
        assert!((cpu_wf.sharpe_worst - gpu_wf.sharpe_worst).abs() < 5e-2);
        assert!((cpu_wf.max_dd_median - gpu_wf.max_dd_median).abs() < 1e-5);
        assert!((cpu_wf.max_dd_worst - gpu_wf.max_dd_worst).abs() < 1e-5);
        assert!((cpu_wf.pnl_median - gpu_wf.pnl_median).abs() < 1e-3);
        assert!((cpu_wf.pnl_worst - gpu_wf.pnl_worst).abs() < 1e-3);
        assert_eq!(cpu_wf.trades_median, gpu_wf.trades_median);
        assert_eq!(cpu_wf.trades_worst, gpu_wf.trades_worst);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_cscv_matches_cpu_breakout_momentum_adx_on() {
        let candles = make_breakout_candles(5000);
        let cancel = AtomicBool::new(false);
        let mut strategy = crate::strategy_presets::breakout_strategy_for_kernel();
        strategy.breakout_use_adx = true;
        strategy.breakout_adx_period = 14;
        strategy.breakout_adx_min = 20.0;

        let cscv = Some(CscvPboConfig {
            slices: 8,
            max_combinations: 1000,
            threshold: 0.0,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-hybrid-breakout-cscv".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 10,
                        slow_len: 30,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 14,
                        slow_len: 42,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                ],
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: cscv.clone(),
                spa: None,
                seed: 1,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu hybrid cscv");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-hybrid-breakout-cscv".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 10,
                        slow_len: 30,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 14,
                        slow_len: 42,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                ],
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: cscv,
                spa: None,
                seed: 1,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid cscv");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!(
            (cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-3,
            "cscv pbo mismatch cpu={} gpu={}",
            cpu_cscv.pbo,
            gpu_cscv.pbo
        );
        assert!(
            (cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-3,
            "cscv lambda mismatch cpu={} gpu={}",
            cpu_cscv.lambda_p50,
            gpu_cscv.lambda_p50
        );
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-6);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn breakout_validation_gpu_parity_diagnostic() {
        let candles = make_breakout_candles(5000);
        let strategy = crate::strategy_presets::breakout_strategy_for_kernel();
        let params = DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
        };
        let lr = compute_log_returns_f64(&candles.close);
        let base_first_valid = first_finite_idx(vector_ta::utilities::data_loader::source_type(
            &candles, "close",
        ))
        .expect("base first valid");
        let series = build_candidate_series(
            &candles,
            &lr,
            base_first_valid,
            "close",
            None,
            None,
            &params,
        )
        .expect("cpu series");
        let gpu_like_start_t =
            candidate_start_t(&candles, "close", base_first_valid, &params).expect("gpu start_t");
        let cpu_f64 = eval_metrics_window(&series, series.start_t, series.n);
        let fast_f32: Vec<f32> = series.fast.iter().map(|&v| v as f32).collect();
        let slow_f32: Vec<f32> = series.slow.iter().map(|&v| v as f32).collect();
        let regime = series
            .halftrend_trend
            .as_deref()
            .expect("breakout regime filter");
        let cpu_f32_same_start =
            ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter(
                &lr,
                &fast_f32,
                &slow_f32,
                regime,
                series.start_t,
                &series.cfg,
            );
        let cpu_f32_gpu_start =
            ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter(
                &lr,
                &fast_f32,
                &slow_f32,
                regime,
                gpu_like_start_t,
                &series.cfg,
            );

        let regime = compute_combined_regime_filter(&candles, "close", &strategy, None)
            .expect("regime filter");
        let kernel = crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel(
            &candles,
            &[params.fast_len],
            &[params.slow_len],
            "sma",
            "sma",
            "close",
            None,
            None,
            regime.as_deref(),
            &strategy,
            ta_optimizer::ObjectiveKind::Sharpe,
            1,
            true,
            crate::gpu_backtest_kernel::KernelConfig {
                device_id: 0,
                fast_alma_offset: 0.85,
                fast_alma_sigma: 6.0,
                slow_alma_offset: 0.85,
                slow_alma_sigma: 6.0,
            },
            None,
            None,
            1,
            0,
            &AtomicBool::new(false),
            None,
        )
        .expect("gpu kernel metrics");
        let gpu = kernel
            .all
            .as_ref()
            .and_then(|all| all.first().map(|(_, m)| m.clone()))
            .expect("gpu metric");

        assert_eq!(series.start_t, gpu_like_start_t);
        assert!((cpu_f64.pnl - cpu_f32_same_start.pnl).abs() < 1e-9);
        assert!((cpu_f64.sharpe - cpu_f32_same_start.sharpe).abs() < 1e-9);
        assert!((cpu_f32_gpu_start.pnl - gpu.pnl).abs() < 1e-4);
        assert!((cpu_f32_gpu_start.sharpe - gpu.sharpe).abs() < 2e-5);
        assert!((cpu_f32_gpu_start.max_dd - gpu.max_dd).abs() < 1e-5);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn trend_quality_validation_gpu_parity_diagnostic() {
        let candles = make_trending_candles_with_slope(4096, 0.10);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_linearreg_period: 34,
            trend_quality_angle_min_deg: 2.0,
            trend_quality_bbw_period: 20,
            trend_quality_bbw_mult: 2.0,
            trend_quality_bbw_min: 0.01,
            trend_quality_roc_period: 9,
            trend_quality_use_roc: true,
            ..StrategyConfig::default()
        };
        let opt = run_double_ma_optimization_blocking_with_candles(
            DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: "trend-quality-parity".to_string(),
                fast_range: (5, 12, 1),
                slow_range: (20, 40, 2),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(1),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: crate::double_ma::GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("optimization run");
        let (best, _) = opt.top.first().expect("top candidate");
        let params = DoubleMaValidationParams {
            fast_ma_type: best.fast_ma_type.clone(),
            slow_ma_type: best.slow_ma_type.clone(),
            fast_len: best.fast_len,
            slow_len: best.slow_len,
            fast_ma_params: best.fast_ma_params.clone(),
            slow_ma_params: best.slow_ma_params.clone(),
            strategy: strategy.clone(),
            halftrend_filter: None,
        };

        let lr = compute_log_returns_f64(&candles.close);
        let base_first_valid = first_finite_idx(vector_ta::utilities::data_loader::source_type(
            &candles, "close",
        ))
        .expect("base first valid");
        let series = build_candidate_series(
            &candles,
            &lr,
            base_first_valid,
            "close",
            None,
            None,
            &params,
        )
        .expect("cpu series");
        let gpu_like_start_t =
            candidate_start_t(&candles, "close", base_first_valid, &params).expect("gpu start_t");
        let cpu_f64 = eval_metrics_window(&series, series.start_t, series.n);
        let fast_f32: Vec<f32> = series.fast.iter().map(|&v| v as f32).collect();
        let slow_f32: Vec<f32> = series.slow.iter().map(|&v| v as f32).collect();
        let regime = series
            .halftrend_trend
            .as_deref()
            .expect("trend-quality regime filter");
        let cpu_f32_gpu_start =
            ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter(
                &lr,
                &fast_f32,
                &slow_f32,
                regime,
                gpu_like_start_t,
                &series.cfg,
            );

        let regime = compute_combined_regime_filter(&candles, "close", &strategy, None)
            .expect("regime filter");
        let kernel = crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel(
            &candles,
            &[params.fast_len],
            &[params.slow_len],
            "sma",
            "sma",
            "close",
            None,
            None,
            regime.as_deref(),
            &strategy,
            ta_optimizer::ObjectiveKind::Sharpe,
            1,
            true,
            crate::gpu_backtest_kernel::KernelConfig {
                device_id: 0,
                fast_alma_offset: 0.85,
                fast_alma_sigma: 6.0,
                slow_alma_offset: 0.85,
                slow_alma_sigma: 6.0,
            },
            None,
            None,
            1,
            0,
            &AtomicBool::new(false),
            None,
        )
        .expect("gpu kernel metrics");
        let gpu = kernel
            .all
            .as_ref()
            .and_then(|all| all.first().map(|(_, m)| m.clone()))
            .expect("gpu metric");

        assert_eq!(series.start_t, gpu_like_start_t);
        assert!((cpu_f64.pnl - cpu_f32_gpu_start.pnl).abs() < 1e-9);
        assert!((cpu_f64.sharpe - cpu_f32_gpu_start.sharpe).abs() < 1e-9);
        assert!((cpu_f32_gpu_start.pnl - gpu.pnl).abs() < 1e-4);
        assert!((cpu_f32_gpu_start.sharpe - gpu.sharpe).abs() < 2e-4);
        assert!((cpu_f32_gpu_start.max_dd - gpu.max_dd).abs() < 1e-5);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn composite_trend_optimizer_gpu_kernel_matches_cpu_f32_same_start_diagnostic() {
        let len = 8_000usize;
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        for i in 0..len {
            let base = 100.0 + i as f64;
            ts.push(i as i64);
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.5);
            vol.push(1_000.0);
        }
        let candles =
            vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol);

        let strategy = crate::strategy_presets::trend_quality_strategy_for_kernel();
        let params = DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 9,
            slow_len: 21,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
        };

        let lr = compute_log_returns_f64(&candles.close);
        let base_first_valid = first_finite_idx(vector_ta::utilities::data_loader::source_type(
            &candles, "close",
        ))
        .expect("base first valid");
        let series = build_candidate_series(
            &candles,
            &lr,
            base_first_valid,
            "close",
            None,
            None,
            &params,
        )
        .expect("cpu series");
        let gpu_like_start_t =
            candidate_start_t(&candles, "close", base_first_valid, &params).expect("gpu start_t");
        let fast_f32: Vec<f32> = series.fast.iter().map(|&v| v as f32).collect();
        let slow_f32: Vec<f32> = series.slow.iter().map(|&v| v as f32).collect();
        let regime = series
            .halftrend_trend
            .as_deref()
            .expect("composite-trend regime filter");
        let cpu_f32_gpu_start =
            ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter(
                &lr,
                &fast_f32,
                &slow_f32,
                regime,
                gpu_like_start_t,
                &series.cfg,
            );

        let regime = compute_combined_regime_filter(&candles, "close", &strategy, None)
            .expect("regime filter");
        let kernel = crate::gpu_backtest_kernel::optimize_double_ma_gpu_kernel(
            &candles,
            &[params.fast_len],
            &[params.slow_len],
            "sma",
            "sma",
            "close",
            None,
            None,
            regime.as_deref(),
            &strategy,
            ta_optimizer::ObjectiveKind::Pnl,
            1,
            true,
            crate::gpu_backtest_kernel::KernelConfig {
                device_id: 0,
                fast_alma_offset: 0.85,
                fast_alma_sigma: 6.0,
                slow_alma_offset: 0.85,
                slow_alma_sigma: 6.0,
            },
            None,
            None,
            1,
            0,
            &AtomicBool::new(false),
            None,
        )
        .expect("gpu kernel metrics");
        let gpu = kernel
            .all
            .as_ref()
            .and_then(|all| all.first().map(|(_, m)| m.clone()))
            .expect("gpu metric");

        assert_eq!(series.start_t, gpu_like_start_t);
        assert!((cpu_f32_gpu_start.pnl - gpu.pnl).abs() < 1e-4);
        assert!((cpu_f32_gpu_start.sharpe - gpu.sharpe).abs() < 2e-4);
        assert!((cpu_f32_gpu_start.max_dd - gpu.max_dd).abs() < 1e-5);
    }

    #[test]
    fn fold_cscv_into_robust_score_uses_oos_median_sharpe_as_lower_bound() {
        let st_low = CscvCandidatePboStats {
            is_selected_frac: 0.5,
            oos_rank_frac_p50: Some(0.8),
            oos_metric_p50: Some(0.25),
        };
        assert!(
            (crate::validation_pipeline::fold_cscv_metric(1.0, st_low.oos_metric_p50) - 0.25).abs()
                < 1e-12
        );

        let st_high = CscvCandidatePboStats {
            is_selected_frac: 0.5,
            oos_rank_frac_p50: Some(0.8),
            oos_metric_p50: Some(2.0),
        };
        assert!(
            (crate::validation_pipeline::fold_cscv_metric(1.0, st_high.oos_metric_p50) - 1.0).abs()
                < 1e-12
        );

        let st_none = CscvCandidatePboStats {
            is_selected_frac: 0.0,
            oos_rank_frac_p50: None,
            oos_metric_p50: None,
        };
        assert!(
            (crate::validation_pipeline::fold_cscv_metric(1.0, st_none.oos_metric_p50) - 1.0).abs()
                < 1e-12
        );
        assert!((crate::validation_pipeline::fold_cscv_metric(1.0, None) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn double_ma_validation_cscv_smoke() {
        let candles = make_trending_candles(8192);
        let cancel = AtomicBool::new(false);

        let a = DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: Some(HalfTrendRegimeFilter {
                amplitude: 2,
                atr_period: 100,
                channel_deviation: 2.0,
            }),
        };
        let b = DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 12,
            slow_len: 36,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: a.halftrend_filter.clone(),
        };

        let req = DoubleMaValidationRequest {
            data_id: "test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![a, b],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 8,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            spa: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert_eq!(res.candidates.len(), 2);
        assert!(res.cscv_pbo.is_some());
        assert!(res.candidates[0].cscv_pbo.is_some());
        assert!(res.candidates[1].cscv_pbo.is_some());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_cscv_matches_cpu_plain() {
        let candles = make_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 12,
                slow_len: 36,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
        ];
        let cscv = Some(CscvPboConfig {
            slices: 8,
            max_combinations: 1000,
            threshold: 0.0,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-cscv".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: cscv.clone(),
                spa: None,
                seed: 1,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu cscv validation");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-cscv".to_string(),
                backend: Backend::GpuKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: cscv,
                spa: None,
                seed: 1,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu cscv validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_cscv = cpu_res.cscv_pbo.as_ref().expect("cpu cscv");
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(cpu_cscv.slices, gpu_cscv.slices);
        assert_eq!(cpu_cscv.slice_bars, gpu_cscv.slice_bars);
        assert!(
            (cpu_cscv.pbo - gpu_cscv.pbo).abs() < 1e-3,
            "cscv pbo mismatch cpu={} gpu={}",
            cpu_cscv.pbo,
            gpu_cscv.pbo
        );
        assert!(
            (cpu_cscv.lambda_p50 - gpu_cscv.lambda_p50).abs() < 1e-3,
            "cscv lambda mismatch cpu={} gpu={}",
            cpu_cscv.lambda_p50,
            gpu_cscv.lambda_p50
        );
        assert_eq!(cpu_res.candidates.len(), gpu_res.candidates.len());
        for (cpu, gpu) in cpu_res.candidates.iter().zip(gpu_res.candidates.iter()) {
            let cpu_st = cpu.cscv_pbo.as_ref().expect("cpu candidate cscv");
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!((cpu_st.is_selected_frac - gpu_st.is_selected_frac).abs() < 1e-6);
            assert!(
                (cpu_st.oos_rank_frac_p50.unwrap_or_default()
                    - gpu_st.oos_rank_frac_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
            assert!(
                (cpu_st.oos_metric_p50.unwrap_or_default()
                    - gpu_st.oos_metric_p50.unwrap_or_default())
                .abs()
                    < 1e-3
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_spa_matches_cpu_plain() {
        let candles = make_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 12,
                slow_len: 36,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
        ];
        let spa = Some(SpaConfigOptional {
            slices: 8,
            bootstrap_samples: 128,
            block_length: 3,
            benchmark: SpaBenchmarkKind::BuyAndHold,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-spa".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa: spa.clone(),
                seed: 7,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu spa validation");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-spa".to_string(),
                backend: Backend::GpuKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa,
                seed: 7,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu spa validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_spa = cpu_res.spa.as_ref().expect("cpu spa");
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(cpu_spa.slice_count, gpu_spa.slice_count);
        assert_eq!(cpu_spa.candidate_count, gpu_spa.candidate_count);
        assert_eq!(cpu_spa.best_candidate_key, gpu_spa.best_candidate_key);
        assert!((cpu_spa.best_candidate_mean_diff - gpu_spa.best_candidate_mean_diff).abs() < 1e-3);
        assert!((cpu_spa.best_candidate_t_stat - gpu_spa.best_candidate_t_stat).abs() < 1e-3);
        assert!((cpu_spa.p_value - gpu_spa.p_value).abs() < 1e-9);
        assert_eq!(cpu_spa.passed, gpu_spa.passed);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_perturb_matches_cpu_plain() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| DoubleMaValidationRequest {
            data_id: "gpu-perturb-plain".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: Some(PerturbConfig {
                iterations: 32,
                pct_min: 0.05,
                pct_max: 0.10,
                flip_bool_prob: 0.1,
            }),
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 59,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu perturb validation");
        let gpu_res = run_double_ma_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu perturb validation");

        let cpu_pt = cpu_res.candidates[0].perturb.as_ref().expect("cpu perturb");
        let gpu_pt = gpu_res.candidates[0].perturb.as_ref().expect("gpu perturb");
        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert_eq!(cpu_pt.iterations, gpu_pt.iterations);
        assert!((cpu_pt.sharpe_mean - gpu_pt.sharpe_mean).abs() < 1e-4);
        assert!((cpu_pt.sharpe_worst - gpu_pt.sharpe_worst).abs() < 1e-4);
        assert!((cpu_pt.max_dd_mean - gpu_pt.max_dd_mean).abs() < 1e-4);
        assert!((cpu_pt.max_dd_worst - gpu_pt.max_dd_worst).abs() < 1e-4);
        assert!((cpu_pt.pnl_mean - gpu_pt.pnl_mean).abs() < 1e-4);
        assert!((cpu_pt.pnl_worst - gpu_pt.pnl_worst).abs() < 1e-4);
        assert!((cpu_pt.pnl_positive_frac - gpu_pt.pnl_positive_frac).abs() < 1e-12);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_trade_bootstrap_preserves_gpu_route() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |backend: Backend| DoubleMaValidationRequest {
            data_id: "gpu-bootstrap".to_string(),
            backend,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 11,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            mk(Backend::CpuOnly),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu bootstrap validation");
        let gpu_res = run_double_ma_validation_blocking_with_candles(
            mk(Backend::GpuKernel { device_id: 0 }),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu bootstrap validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(gpu_res.backend_fallback_reason.is_none());

        let cpu_bt = cpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("cpu bootstrap");
        let gpu_bt = gpu_res.candidates[0]
            .trade_bootstrap
            .as_ref()
            .expect("gpu bootstrap");
        assert_eq!(cpu_bt.simulations, gpu_bt.simulations);
        assert_eq!(cpu_bt.trade_count, gpu_bt.trade_count);
        assert!((cpu_bt.prob_pnl_positive - gpu_bt.prob_pnl_positive).abs() < 1e-12);
        assert!((cpu_bt.pnl_p05 - gpu_bt.pnl_p05).abs() < 1e-12);
        assert!((cpu_bt.pnl_p50 - gpu_bt.pnl_p50).abs() < 1e-12);
        assert!((cpu_bt.pnl_p95 - gpu_bt.pnl_p95).abs() < 1e-12);
        assert!((cpu_bt.max_dd_p50 - gpu_bt.max_dd_p50).abs() < 1e-12);
        assert!((cpu_bt.max_dd_p95 - gpu_bt.max_dd_p95).abs() < 1e-12);
        assert_eq!(
            cpu_bt.stationary_mean_block_len,
            gpu_bt.stationary_mean_block_len
        );
        assert!(
            (cpu_bt.stationary_prob_pnl_positive.unwrap_or(0.0)
                - gpu_bt.stationary_prob_pnl_positive.unwrap_or(0.0))
            .abs()
                < 1e-12
        );
        assert!(
            (cpu_bt.stationary_pnl_p05.unwrap_or(0.0) - gpu_bt.stationary_pnl_p05.unwrap_or(0.0))
                .abs()
                < 1e-12
        );
        assert!(
            (cpu_bt.stationary_pnl_p50.unwrap_or(0.0) - gpu_bt.stationary_pnl_p50.unwrap_or(0.0))
                .abs()
                < 1e-12
        );
        assert!(
            (cpu_bt.stationary_pnl_p95.unwrap_or(0.0) - gpu_bt.stationary_pnl_p95.unwrap_or(0.0))
                .abs()
                < 1e-12
        );
        assert!(
            (cpu_bt.stationary_max_dd_p50.unwrap_or(0.0)
                - gpu_bt.stationary_max_dd_p50.unwrap_or(0.0))
            .abs()
                < 1e-12
        );
        assert!(
            (cpu_bt.stationary_max_dd_p95.unwrap_or(0.0)
                - gpu_bt.stationary_max_dd_p95.unwrap_or(0.0))
            .abs()
                < 1e-12
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_supported_full_stack_preserves_route() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);
        let req = DoubleMaValidationRequest {
            data_id: "gpu-full-stack".to_string(),
            backend: Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![
                DoubleMaValidationParams {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_len: 10,
                    slow_len: 30,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: StrategyConfig::default(),
                    halftrend_filter: None,
                },
                DoubleMaValidationParams {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_len: 12,
                    slow_len: 36,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: StrategyConfig::default(),
                    halftrend_filter: None,
                },
            ],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1024,
                step_bars: 512,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 1024,
                step_bars: 512,
                top_n_candidates: 2,
            }),
            perturb: Some(PerturbConfig {
                iterations: 16,
                pct_min: 0.05,
                pct_max: 0.10,
                flip_bool_prob: 0.1,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 32,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 8,
                max_combinations: 256,
                threshold: 0.0,
            }),
            spa: Some(SpaConfigOptional {
                slices: 8,
                bootstrap_samples: 64,
                block_length: 3,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            seed: 17,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("gpu full stack validation");

        assert!(matches!(res.backend_used, BackendUsed::Gpu));
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(res.backend_fallback_reason.is_none());
        assert!(res.spa.is_some());
        assert!(res.cscv_pbo.is_some());
        assert!(res.reopt_walk_forward.is_some());
        assert_eq!(res.candidates.len(), 2);
        for candidate in &res.candidates {
            assert!(candidate.walk_forward.is_some());
            assert!(candidate.perturb.is_some());
            assert!(candidate.trade_bootstrap.is_some());
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_cscv_smoke_regime_hybrid() {
        let candles = make_choppy_candles(4096);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: false,
            regime_hybrid_enable: true,
            regime_hybrid_adx_period: 14,
            regime_hybrid_adx_enter: 22.0,
            regime_hybrid_adx_exit: 18.0,
            regime_hybrid_reversion_entry_rel: 0.004,
            regime_hybrid_reversion_exit_rel: 0.0015,
            ..StrategyConfig::default()
        };

        let cscv = Some(CscvPboConfig {
            slices: 8,
            max_combinations: 1000,
            threshold: 0.0,
        });

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-hybrid-cscv".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 10,
                        slow_len: 30,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 14,
                        slow_len: 42,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                ],
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: cscv,
                spa: None,
                seed: 1,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid cscv");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        let gpu_cscv = gpu_res.cscv_pbo.as_ref().expect("gpu cscv");
        assert_eq!(gpu_cscv.slices, 8);
        assert!(gpu_cscv.slice_bars > 0);
        assert!(gpu_cscv.pbo.is_finite());
        assert!(gpu_cscv.lambda_p50.is_finite());
        for gpu in &gpu_res.candidates {
            let gpu_st = gpu.cscv_pbo.as_ref().expect("gpu candidate cscv");
            assert!(gpu_st.is_selected_frac.is_finite());
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_spa_smoke_regime_hybrid() {
        let candles = make_choppy_candles(4096);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: false,
            regime_hybrid_enable: true,
            regime_hybrid_adx_period: 14,
            regime_hybrid_adx_enter: 22.0,
            regime_hybrid_adx_exit: 18.0,
            regime_hybrid_reversion_entry_rel: 0.004,
            regime_hybrid_reversion_exit_rel: 0.0015,
            ..StrategyConfig::default()
        };

        let spa = Some(SpaConfigOptional {
            slices: 8,
            bootstrap_samples: 64,
            block_length: 3,
            benchmark: SpaBenchmarkKind::BuyAndHold,
        });

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-hybrid-spa".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 10,
                        slow_len: 30,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 14,
                        slow_len: 42,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                ],
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa,
                seed: 3,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid spa");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        let gpu_spa = gpu_res.spa.as_ref().expect("gpu spa");
        assert_eq!(gpu_spa.slice_count, 8);
        assert_eq!(gpu_spa.candidate_count, 2);
        assert!(!gpu_spa.best_candidate_key.is_empty());
        assert!(gpu_spa.best_candidate_mean_diff.is_finite());
        assert!(gpu_spa.best_candidate_t_stat.is_finite());
        assert!(gpu_spa.p_value.is_finite());
    }

    #[test]
    fn double_ma_validation_reopt_walk_forward_smoke() {
        let candles = make_trending_candles(8192);
        let cancel = AtomicBool::new(false);

        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 14,
                slow_len: 42,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
        ];

        let req = DoubleMaValidationRequest {
            data_id: "reopt".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates,
            walk_forward: None,
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 2000,
                step_bars: 1000,
                top_n_candidates: 2,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 3,
            max_candidates: 8,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("reopt validation run");
        let stats = res.reopt_walk_forward.as_ref().expect("reopt stats");
        assert!(stats.windows > 0);
        assert!(stats.unique_winners > 0);
        assert!(stats.dominant_winner_frac.is_finite());
        assert!(stats.forward_positive_pnl_frac.is_finite());
        assert!(stats.winner_param_drift_mean.is_finite());
        assert!(stats.winner_param_drift_max.is_finite());
        assert_eq!(stats.windows, stats.winner_sequence.len());
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_kernel_reopt_walk_forward_matches_cpu_plain() {
        let candles = make_trending_candles(8192);
        let cancel = AtomicBool::new(false);
        let candidates = vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 12,
                slow_len: 36,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
        ];
        let reopt = Some(ReoptWalkForwardConfigOptional {
            window_bars: 2000,
            step_bars: 1000,
            top_n_candidates: 2,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-reopt".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: reopt.clone(),
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa: None,
                seed: 11,
                max_candidates: 16,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu reopt validation");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-reopt".to_string(),
                backend: Backend::GpuKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: reopt,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa: None,
                seed: 11,
                max_candidates: 16,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu reopt validation");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert_eq!(gpu_res.backend_route.as_deref(), Some("GpuKernel"));
        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_eq!(cpu_stats.windows, gpu_stats.windows);
        assert_eq!(cpu_stats.top_n_candidates, gpu_stats.top_n_candidates);
        assert_eq!(cpu_stats.unique_winners, gpu_stats.unique_winners);
        assert_eq!(cpu_stats.winner_changes, gpu_stats.winner_changes);
        assert_eq!(cpu_stats.winner_sequence, gpu_stats.winner_sequence);
        assert!((cpu_stats.dominant_winner_frac - gpu_stats.dominant_winner_frac).abs() < 1e-6);
        assert!(
            (cpu_stats.winner_param_drift_mean - gpu_stats.winner_param_drift_mean).abs() < 1e-6
        );
        assert!((cpu_stats.winner_param_drift_max - gpu_stats.winner_param_drift_max).abs() < 1e-6);
        assert!((cpu_stats.train_score_median - gpu_stats.train_score_median).abs() < 5e-2);
        assert!(
            (cpu_stats.forward_positive_pnl_frac - gpu_stats.forward_positive_pnl_frac).abs()
                < 1e-6
        );
        assert!((cpu_stats.forward_sharpe_median - gpu_stats.forward_sharpe_median).abs() < 5e-2);
        assert!((cpu_stats.forward_sharpe_worst - gpu_stats.forward_sharpe_worst).abs() < 5e-2);
        assert!((cpu_stats.forward_max_dd_median - gpu_stats.forward_max_dd_median).abs() < 1e-5);
        assert!((cpu_stats.forward_max_dd_worst - gpu_stats.forward_max_dd_worst).abs() < 1e-5);
        assert!((cpu_stats.forward_pnl_median - gpu_stats.forward_pnl_median).abs() < 1e-3);
        assert!((cpu_stats.forward_pnl_worst - gpu_stats.forward_pnl_worst).abs() < 1e-3);
        assert_eq!(
            cpu_stats.forward_trades_median,
            gpu_stats.forward_trades_median
        );
        assert_eq!(
            cpu_stats.forward_trades_worst,
            gpu_stats.forward_trades_worst
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn double_ma_validation_gpu_hybrid_reopt_walk_forward_matches_cpu_regime_hybrid() {
        let candles = make_choppy_candles(4096);
        let cancel = AtomicBool::new(false);
        let strategy = StrategyConfig {
            trend_quality_enable: false,
            regime_hybrid_enable: true,
            regime_hybrid_adx_period: 14,
            regime_hybrid_adx_enter: 22.0,
            regime_hybrid_adx_exit: 18.0,
            regime_hybrid_reversion_entry_rel: 0.004,
            regime_hybrid_reversion_exit_rel: 0.0015,
            ..StrategyConfig::default()
        };

        let reopt = Some(ReoptWalkForwardConfigOptional {
            window_bars: 1024,
            step_bars: 512,
            top_n_candidates: 2,
        });

        let cpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "cpu-hybrid-reopt".to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 10,
                        slow_len: 30,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 14,
                        slow_len: 42,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                ],
                walk_forward: None,
                reopt_walk_forward: reopt.clone(),
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa: None,
                seed: 5,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("cpu hybrid reopt");

        let gpu_res = run_double_ma_validation_blocking_with_candles(
            DoubleMaValidationRequest {
                data_id: "gpu-hybrid-reopt".to_string(),
                backend: Backend::GpuHybridKernel { device_id: 0 },
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: vec![
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 10,
                        slow_len: 30,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy: strategy.clone(),
                        halftrend_filter: None,
                    },
                    DoubleMaValidationParams {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                        fast_len: 14,
                        slow_len: 42,
                        fast_ma_params: None,
                        slow_ma_params: None,
                        strategy,
                        halftrend_filter: None,
                    },
                ],
                walk_forward: None,
                reopt_walk_forward: reopt,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa: None,
                seed: 5,
                max_candidates: 32,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            },
            &candles,
            &cancel,
            None,
        )
        .expect("gpu hybrid reopt");

        assert!(matches!(gpu_res.backend_used, BackendUsed::Gpu));
        assert!(gpu_res
            .backend_route
            .as_deref()
            .unwrap_or_default()
            .contains("GpuHybridKernel"));
        let cpu_stats = cpu_res.reopt_walk_forward.as_ref().expect("cpu reopt");
        let gpu_stats = gpu_res.reopt_walk_forward.as_ref().expect("gpu reopt");
        assert_eq!(cpu_stats.windows, gpu_stats.windows);
        assert_eq!(cpu_stats.top_n_candidates, gpu_stats.top_n_candidates);
        assert_eq!(cpu_stats.unique_winners, gpu_stats.unique_winners);
        assert_eq!(cpu_stats.winner_changes, gpu_stats.winner_changes);
        assert_eq!(cpu_stats.winner_sequence, gpu_stats.winner_sequence);
        assert!((cpu_stats.dominant_winner_frac - gpu_stats.dominant_winner_frac).abs() < 1e-6);
        assert!(
            (cpu_stats.winner_param_drift_mean - gpu_stats.winner_param_drift_mean).abs() < 1e-6
        );
        assert!((cpu_stats.winner_param_drift_max - gpu_stats.winner_param_drift_max).abs() < 1e-6);
        assert!((cpu_stats.train_score_median - gpu_stats.train_score_median).abs() < 5e-2);
        assert!(
            (cpu_stats.forward_positive_pnl_frac - gpu_stats.forward_positive_pnl_frac).abs()
                < 1e-6
        );
        assert!((cpu_stats.forward_sharpe_median - gpu_stats.forward_sharpe_median).abs() < 5e-2);
        assert!((cpu_stats.forward_sharpe_worst - gpu_stats.forward_sharpe_worst).abs() < 5e-2);
        assert!((cpu_stats.forward_max_dd_median - gpu_stats.forward_max_dd_median).abs() < 1e-5);
        assert!((cpu_stats.forward_max_dd_worst - gpu_stats.forward_max_dd_worst).abs() < 1e-5);
        assert!((cpu_stats.forward_pnl_median - gpu_stats.forward_pnl_median).abs() < 1e-3);
        assert!((cpu_stats.forward_pnl_worst - gpu_stats.forward_pnl_worst).abs() < 1e-3);
        assert_eq!(
            cpu_stats.forward_trades_median,
            gpu_stats.forward_trades_median
        );
        assert_eq!(
            cpu_stats.forward_trades_worst,
            gpu_stats.forward_trades_worst
        );
    }

    #[test]
    fn double_ma_validation_iterative_engine_emits_delta_fields() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let base = DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
        };

        let req = DoubleMaValidationRequest {
            data_id: "test".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 9,
            max_candidates: 32,
            validation_engine: ValidationEngine::Iterative,
            iterative: IterativeSimConfig {
                slippage_vol_mult: 0.25,
                ..IterativeSimConfig::default()
            },
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        let c = &res.candidates[0];
        assert!(c.iterative_in_sample.is_some());
        let d = c.iterative_delta.as_ref().expect("iterative delta");
        assert!(d.pnl.is_finite());
        assert!(d.sharpe.is_finite());
        assert!(d.max_dd.is_finite());
    }

    #[test]
    fn double_ma_validation_iterative_engine_matches_optimizer_top_candidates() {
        fn run_case(
            label: &str,
            candles: &vector_ta::utilities::data_loader::Candles,
            strategy: StrategyConfig,
            halftrend_filter: Option<HalfTrendRegimeFilter>,
        ) {
            let cancel = AtomicBool::new(false);
            let opt_req = DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: format!("opt-{label}"),
                fast_range: (5, 9, 1),
                slow_range: (18, 28, 2),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: strategy.clone(),
                halftrend_filter: halftrend_filter.clone(),
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(6),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: crate::double_ma::GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
            };

            let opt =
                run_double_ma_optimization_blocking_with_candles(opt_req, candles, &cancel, None)
                    .expect("optimization run");
            assert!(
                !opt.top.is_empty(),
                "optimization returned no candidates for {label}"
            );

            let candidates: Vec<DoubleMaValidationParams> = opt
                .top
                .iter()
                .take(6)
                .map(|(p, _)| DoubleMaValidationParams {
                    fast_ma_type: p.fast_ma_type.clone(),
                    slow_ma_type: p.slow_ma_type.clone(),
                    fast_len: p.fast_len,
                    slow_len: p.slow_len,
                    fast_ma_params: p.fast_ma_params.clone(),
                    slow_ma_params: p.slow_ma_params.clone(),
                    strategy: strategy.clone(),
                    halftrend_filter: halftrend_filter.clone(),
                })
                .collect();

            let mk_req = |validation_engine: ValidationEngine| DoubleMaValidationRequest {
                data_id: format!("opt-vs-iter-{label}"),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa: None,
                seed: 17,
                max_candidates: 32,
                validation_engine,
                iterative: IterativeSimConfig {
                    slippage_vol_mult: 0.0,
                    ..IterativeSimConfig::default()
                },
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            };

            let vec_res = run_double_ma_validation_blocking_with_candles(
                mk_req(ValidationEngine::Vectorized),
                candles,
                &cancel,
                None,
            )
            .expect("vectorized validation run");
            let iter_res = run_double_ma_validation_blocking_with_candles(
                mk_req(ValidationEngine::Iterative),
                candles,
                &cancel,
                None,
            )
            .expect("iterative validation run");

            assert_eq!(vec_res.candidates.len(), iter_res.candidates.len());

            for vec_cand in &vec_res.candidates {
                let iter_cand = iter_res
                    .candidates
                    .iter()
                    .find(|c| {
                        c.params.fast_ma_type == vec_cand.params.fast_ma_type
                            && c.params.slow_ma_type == vec_cand.params.slow_ma_type
                            && c.params.fast_len == vec_cand.params.fast_len
                            && c.params.slow_len == vec_cand.params.slow_len
                            && c.params.fast_ma_params == vec_cand.params.fast_ma_params
                            && c.params.slow_ma_params == vec_cand.params.slow_ma_params
                    })
                    .expect("matching iterative candidate");

                assert!((iter_cand.in_sample.pnl - vec_cand.in_sample.pnl).abs() < 1e-12);
                assert!((iter_cand.in_sample.sharpe - vec_cand.in_sample.sharpe).abs() < 1e-12);
                assert!((iter_cand.in_sample.max_dd - vec_cand.in_sample.max_dd).abs() < 1e-12);

                let iter_m = iter_cand
                    .iterative_in_sample
                    .as_ref()
                    .expect("iterative metrics");
                let delta = iter_cand.iterative_delta.as_ref().expect("iterative delta");

                assert!(iter_m.pnl.is_finite());
                assert!(iter_m.sharpe.is_finite());
                assert!(iter_m.max_dd.is_finite());
                assert!((iter_m.pnl - iter_cand.in_sample.pnl - delta.pnl).abs() < 1e-9);
                assert!((iter_m.sharpe - iter_cand.in_sample.sharpe - delta.sharpe).abs() < 1e-9);
                assert!((iter_m.max_dd - iter_cand.in_sample.max_dd - delta.max_dd).abs() < 1e-9);
            }
        }

        run_case(
            "double-ma-default",
            &make_trending_candles(4096),
            StrategyConfig::default(),
            None,
        );

        run_case(
            "composite-trend",
            &make_trending_candles(4096),
            StrategyConfig {
                trend_quality_enable: true,
                trend_quality_linearreg_period: 34,
                trend_quality_angle_min_deg: 2.0,
                trend_quality_bbw_period: 20,
                trend_quality_bbw_mult: 2.0,
                trend_quality_bbw_min: 0.01,
                trend_quality_roc_period: 9,
                trend_quality_use_roc: true,
                ..StrategyConfig::default()
            },
            None,
        );

        run_case(
            "regime-hybrid",
            &make_choppy_candles(4096),
            StrategyConfig {
                trend_quality_enable: false,
                regime_hybrid_enable: true,
                regime_hybrid_adx_period: 14,
                regime_hybrid_adx_enter: 22.0,
                regime_hybrid_adx_exit: 18.0,
                regime_hybrid_reversion_entry_rel: 0.004,
                regime_hybrid_reversion_exit_rel: 0.0015,
                ..StrategyConfig::default()
            },
            None,
        );

        run_case(
            "kaufman-stop",
            &make_choppy_candles(4096),
            StrategyConfig {
                exit_mode: ExitMode::KaufmanStop,
                kaufman_stop_period: 22,
                kaufman_stop_mult: 2.0,
                kaufman_stop_ma_type: "sma".to_string(),
                ..StrategyConfig::default()
            },
            None,
        );
    }

    #[test]
    fn double_ma_validation_rerun_equivalence_same_seed() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let base = DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 12,
            slow_len: 36,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
        };

        let req = DoubleMaValidationRequest {
            data_id: "det".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1024,
                step_bars: 512,
            }),
            reopt_walk_forward: None,
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: 0.01,
                pct_max: 0.02,
                flip_bool_prob: 0.01,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let a =
            run_double_ma_validation_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .expect("first run");
        let b = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("second run");

        assert_eq!(a.candidates.len(), b.candidates.len());
        for (ca, cb) in a.candidates.iter().zip(b.candidates.iter()) {
            assert_eq!(
                serde_json::to_string(&ca.params).expect("params a"),
                serde_json::to_string(&cb.params).expect("params b")
            );
            assert!((ca.robust_score - cb.robust_score).abs() < 1e-12);
            assert!((ca.in_sample.pnl - cb.in_sample.pnl).abs() < 1e-12);
            assert!((ca.psr.unwrap_or(0.0) - cb.psr.unwrap_or(0.0)).abs() < 1e-12);
            assert!((ca.dsr.unwrap_or(0.0) - cb.dsr.unwrap_or(0.0)).abs() < 1e-12);
        }
    }

    #[test]
    fn double_ma_validation_probability_gates_filter_results() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let mk = |fast_len: u16, slow_len: u16| DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len,
            slow_len,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
        };

        let base_req = DoubleMaValidationRequest {
            data_id: "gate".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(8, 24), mk(12, 36), mk(20, 60)],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let baseline = run_double_ma_validation_blocking_with_candles(
            base_req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("baseline run");

        let mut gated_req = base_req;
        gated_req.min_psr = Some(0.80);
        gated_req.min_dsr = Some(0.60);
        let gated =
            run_double_ma_validation_blocking_with_candles(gated_req, &candles, &cancel, None)
                .expect("gated run");

        assert_eq!(gated.candidates.len(), baseline.candidates.len());
        assert!(
            gated.run_verdict.candidates_survived_hard_gates <= gated.candidates.len(),
            "survivor count must not exceed total candidates"
        );
        for cand in &gated.candidates {
            let passed = cand.validation_verdict.passed;
            let psr_ok = cand.psr.unwrap_or(0.0) >= 0.80;
            let dsr_ok = cand.dsr.unwrap_or(0.0) >= 0.60;
            assert_eq!(passed, psr_ok && dsr_ok);
        }
    }

    #[test]
    fn double_ma_validation_rejects_invalid_candidate_params() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaValidationRequest {
            data_id: "invalid".to_string(),
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 0,
                slow_len: 0,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig {
                    fee_bps: f64::NAN,
                    spread_bps: 0.0,
                    slippage_bps: 0.0,
                    commission: f64::NAN,
                    ..StrategyConfig::default()
                },
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 1,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let err = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("invalid candidates must fail validation");
        assert!(!err.is_empty());
    }

    #[test]
    fn composite_trend_opt_validate_meta_smoke() {
        fn make_trending_candles_with_slope(
            len: usize,
            slope: f64,
        ) -> vector_ta::utilities::data_loader::Candles {
            let mut ts = Vec::with_capacity(len);
            let mut open = Vec::with_capacity(len);
            let mut high = Vec::with_capacity(len);
            let mut low = Vec::with_capacity(len);
            let mut close = Vec::with_capacity(len);
            let mut vol: Vec<f64> = Vec::with_capacity(len);

            for i in 0..len {
                ts.push(i as i64);
                let base = 100.0 + i as f64 * slope;
                open.push(base);
                high.push(base + 1.0);
                low.push(base - 1.0);
                close.push(base + 0.25);
                vol.push(1_000.0);
            }
            vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
        }

        let cancel = AtomicBool::new(false);
        let candles_a = make_trending_candles_with_slope(4096, 0.10);
        let candles_b = make_trending_candles_with_slope(4096, 0.07);

        let strategy = StrategyConfig {
            trend_quality_enable: true,
            trend_quality_linearreg_period: 34,
            trend_quality_angle_min_deg: 2.0,
            trend_quality_bbw_period: 20,
            trend_quality_bbw_mult: 2.0,
            trend_quality_bbw_min: 0.01,
            trend_quality_roc_period: 9,
            trend_quality_use_roc: true,
            ..StrategyConfig::default()
        };

        let run_for = |label: &str, candles: &vector_ta::utilities::data_loader::Candles| {
            let opt_req = DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: label.to_string(),
                fast_range: (5, 12, 1),
                slow_range: (20, 40, 2),
                fast_ma_types: vec!["sma".to_string()],
                slow_ma_types: vec!["sma".to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: None,
                slow_ma_params: None,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: strategy.clone(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(16),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
                dynamic_strategy_ir: None,
                gpu_execution: crate::double_ma::GpuExecutionPolicy::BestEffort,
                persist_followup_cache: false,
            };

            let opt =
                run_double_ma_optimization_blocking_with_candles(opt_req, candles, &cancel, None)
                    .expect("composite-trend optimization run");
            assert!(
                !opt.top.is_empty(),
                "optimization returned no top candidates"
            );

            let candidates: Vec<DoubleMaValidationParams> = opt
                .top
                .iter()
                .take(16)
                .map(|(p, _)| DoubleMaValidationParams {
                    fast_ma_type: p.fast_ma_type.clone(),
                    slow_ma_type: p.slow_ma_type.clone(),
                    fast_len: p.fast_len,
                    slow_len: p.slow_len,
                    fast_ma_params: p.fast_ma_params.clone(),
                    slow_ma_params: p.slow_ma_params.clone(),
                    strategy: strategy.clone(),
                    halftrend_filter: None,
                })
                .collect();

            let val_req = DoubleMaValidationRequest {
                data_id: label.to_string(),
                backend: Backend::CpuOnly,
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                reopt_walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                spa: None,
                seed: 7,
                max_candidates: 16,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                min_cscv_oos_rank_frac: None,
                min_cscv_oos_metric: None,
                max_pbo: None,
                validation_context: None,
            };

            let val =
                run_double_ma_validation_blocking_with_candles(val_req, candles, &cancel, None)
                    .expect("composite-trend validation run");
            assert!(
                !val.candidates.is_empty(),
                "validation returned no candidates"
            );
            val
        };

        let val_a = run_for("asset_a", &candles_a);
        let val_b = run_for("asset_b", &candles_b);

        let artifacts = vec![
            json!({
                "kind": "vectorgrid_run_artifact",
                "ts": "ct_0001",
                "tags": ["ct-e2e"],
                "run": { "req": { "strategy_kind": "CompositeTrend", "ma_source": "close" }, "res": {} },
                "dataset_meta": { "label": "asset_a", "timeframe": "1m", "csv_path": "asset_a.csv" },
                "validation": { "req": { "ma_source": "close" }, "res": val_a }
            }),
            json!({
                "kind": "vectorgrid_run_artifact",
                "ts": "ct_0002",
                "tags": ["ct-e2e"],
                "run": { "req": { "strategy_kind": "CompositeTrend", "ma_source": "close" }, "res": {} },
                "dataset_meta": { "label": "asset_b", "timeframe": "1m", "csv_path": "asset_b.csv" },
                "validation": { "req": { "ma_source": "close" }, "res": val_b }
            }),
        ];

        let meta_req = MetaSelectionRequest {
            strategy_kind: "CompositeTrend".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: Some("ct-e2e".to_string()),
            holdout_tag: None,
            eval_holdout: false,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: false,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 1,
        };

        let meta = meta_select_from_artifacts(&meta_req, &artifacts, None);
        assert!(meta.artifacts_used >= 2);
        assert!(!meta.groups.is_empty());
        assert!(meta.groups[0].meta_score.is_finite());
    }
}
