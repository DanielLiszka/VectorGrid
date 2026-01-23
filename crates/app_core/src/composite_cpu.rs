use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use ta_optimizer::stream::StreamAggregator;
use ta_optimizer::{ObjectiveKind, OptimizationMode, OptimizationResult};
use ta_strategies::double_ma::{DoubleMaCurves, DoubleMaParams, Metrics, StrategyConfig};

use crate::double_ma::{
    compute_halftrend_trend, compute_log_returns_f64, cpu_tile_sizes, estimate_pairs_fast_lt_slow,
    expand_u16_range, first_finite_idx, lookup_ma_info, ma_matrix_cpu_f64,
    resolve_alma_params_for_side, validate_ma_kind, validate_strategy_cfg,
    BackendOptimizationResult, BackendUsed, DoubleMaParamsResolved, HalfTrendRegimeFilter,
    OptimizationModeResolved,
};
use crate::progress::ProgressSink;

#[derive(Clone, Debug, Deserialize)]
pub struct MaczGateParams {
    #[serde(default = "default_macz_fast_length")]
    pub fast_length: u16,
    #[serde(default = "default_macz_slow_length")]
    pub slow_length: u16,
    #[serde(default = "default_macz_signal_length")]
    pub signal_length: u16,
    #[serde(default = "default_macz_lengthz")]
    pub lengthz: u16,
    #[serde(default = "default_macz_length_stdev")]
    pub length_stdev: u16,
    #[serde(default = "default_macz_a")]
    pub a: f64,
    #[serde(default = "default_macz_b")]
    pub b: f64,
    #[serde(default)]
    pub use_lag: bool,
    #[serde(default = "default_macz_gamma")]
    pub gamma: f64,
    #[serde(default)]
    pub use_volume: bool,
}

fn default_macz_fast_length() -> u16 {
    12
}
fn default_macz_slow_length() -> u16 {
    25
}
fn default_macz_signal_length() -> u16 {
    9
}
fn default_macz_lengthz() -> u16 {
    20
}
fn default_macz_length_stdev() -> u16 {
    25
}
fn default_macz_a() -> f64 {
    1.0
}
fn default_macz_b() -> f64 {
    1.0
}
fn default_macz_gamma() -> f64 {
    0.02
}

impl Default for MaczGateParams {
    fn default() -> Self {
        Self {
            fast_length: default_macz_fast_length(),
            slow_length: default_macz_slow_length(),
            signal_length: default_macz_signal_length(),
            lengthz: default_macz_lengthz(),
            length_stdev: default_macz_length_stdev(),
            a: default_macz_a(),
            b: default_macz_b(),
            use_lag: false,
            gamma: default_macz_gamma(),
            use_volume: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct OttGateParams {
    #[serde(default = "default_ott_period")]
    pub period: u16,
    #[serde(default = "default_ott_percent")]
    pub percent: f64,
    #[serde(default = "default_ott_ma_type")]
    pub ma_type: String,
}

fn default_ott_period() -> u16 {
    2
}
fn default_ott_percent() -> f64 {
    1.4
}
fn default_ott_ma_type() -> String {
    "VAR".to_string()
}

impl Default for OttGateParams {
    fn default() -> Self {
        Self {
            period: default_ott_period(),
            percent: default_ott_percent(),
            ma_type: default_ott_ma_type(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeCpuRequest {
    pub data_id: String,
    pub fast_range: (u32, u32, u32),
    pub slow_range: (u32, u32, u32),
    pub fast_ma_types: Vec<String>,
    pub slow_ma_types: Vec<String>,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend: HalfTrendRegimeFilter,
    #[serde(default)]
    pub macz: MaczGateParams,
    #[serde(default)]
    pub ott: OttGateParams,
    pub objective: ObjectiveKind,
    pub mode: OptimizationMode,
    pub top_k: Option<usize>,
    pub include_all: Option<bool>,
    pub heatmap_bins: Option<u16>,
}

fn default_ma_source() -> String {
    "close".to_string()
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeCpuDrilldownRequest {
    pub data_id: String,
    pub params: DoubleMaParamsResolved,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub strategy: StrategyConfig,
    pub halftrend: HalfTrendRegimeFilter,
    #[serde(default)]
    pub macz: MaczGateParams,
    #[serde(default)]
    pub ott: OttGateParams,
    pub bins: Option<usize>,
}

pub fn run_composite_cpu_optimization_blocking_with_candles(
    req: CompositeCpuRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<BackendOptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    validate_strategy_cfg(&req.strategy)?;

    let heatmap_bins = req.heatmap_bins.unwrap_or(0) as usize;
    if heatmap_bins > 512 {
        return Err("heatmap_bins too large (max 512)".to_string());
    }

    if matches!(req.mode, OptimizationMode::CoarseToFine) {
        return Err("Composite strategy currently supports mode=Grid only.".to_string());
    }
    let mode_used = OptimizationModeResolved::Grid;

    if req.fast_ma_types.len() != 1 || req.slow_ma_types.len() != 1 {
        return Err("Select exactly one fast MA and one slow MA.".to_string());
    }

    let fast_ma = req.fast_ma_types[0].trim().to_ascii_lowercase();
    let slow_ma = req.slow_ma_types[0].trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("Fast/slow MA type cannot be empty.".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma, req.fast_ma_params.as_ref())?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, req.slow_ma_params.as_ref())?;

    let fast_periods = expand_u16_range(req.fast_range)?;
    let slow_periods = expand_u16_range(req.slow_range)?;
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty period range".to_string());
    }
    let total_pairs = estimate_pairs_fast_lt_slow(&fast_periods, &slow_periods);
    if total_pairs == 0 {
        return Err("no valid parameter combinations (fast must be < slow)".to_string());
    }

    validate_macz_params(&req.macz)?;
    validate_ott_params(&req.ott)?;
    if req.macz.use_volume && !candles.fields.volume {
        return Err("MACZ with volume requires a CSV with volume column.".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let ma_source = req.ma_source;
    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &ma_source);
    if ma_prices.len() != close.len() {
        return Err("ma source length mismatch".to_string());
    }

    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let mut first_valid = fv_close.max(fv_ma);

    let halftrend_trend = compute_halftrend_trend(candles, &req.halftrend)?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ht);

    let macz_hist = compute_macz_hist(candles, &ma_source, &req.macz)?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    let fv_macz =
        first_finite_idx(&macz_hist).ok_or_else(|| "macz series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_macz);

    let ott_line = compute_ott_line(candles, &ma_source, &req.ott)?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }
    let fv_ott =
        first_finite_idx(&ott_line).ok_or_else(|| "ott series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ott);

    let start = std::time::Instant::now();
    let objective = req.objective;
    let top_k = req.top_k.unwrap_or(0);
    let include_all = req.include_all.unwrap_or(false);
    if include_all && total_pairs > 2_000_000 {
        return Err(format!(
            "include_all=true would return {} rows; disable it or reduce ranges.",
            total_pairs
        ));
    }

    let result = optimize_composite_cpu_tiled(
        candles,
        &fast_periods,
        &slow_periods,
        &fast_ma,
        &slow_ma,
        &ma_source,
        req.fast_ma_params.as_ref(),
        req.slow_ma_params.as_ref(),
        &halftrend_trend,
        &macz_hist,
        ma_prices,
        &ott_line,
        first_valid,
        &req.strategy,
        objective,
        top_k,
        include_all,
        fast_alma_offset,
        fast_alma_sigma,
        slow_alma_offset,
        slow_alma_sigma,
        total_pairs,
        heatmap_bins,
        cancel,
        progress,
    )?;

    let runtime_ms = start.elapsed().as_millis() as u64;

    let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_type: fast_ma.clone(),
            slow_ma_type: slow_ma.clone(),
        }
    };

    let best_params = resolve_params(&result.best_params);
    let top: Vec<(DoubleMaParamsResolved, Metrics)> = result
        .top
        .iter()
        .map(|(p, m)| (resolve_params(p), m.clone()))
        .collect();
    let all = result.all.as_ref().map(|rows| {
        rows.iter()
            .map(|(p, m)| (resolve_params(p), m.clone()))
            .collect::<Vec<_>>()
    });
    let heatmap = result.heatmap.clone();

    Ok(BackendOptimizationResult {
        best_params,
        best_metrics: result.best_metrics,
        mode_used,
        backend_used: BackendUsed::Cpu,
        num_combos: result.num_combos,
        num_candles: n,
        runtime_ms,
        top,
        all,
        heatmap,
    })
}

pub fn compute_composite_cpu_drilldown_blocking_with_candles(
    req: CompositeCpuDrilldownRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
) -> Result<DoubleMaCurves, String> {
    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_ma = req.params.fast_ma_type.trim();
    let slow_ma = req.params.slow_ma_type.trim();
    validate_ma_kind(fast_ma)?;
    validate_ma_kind(slow_ma)?;

    let fast_len = req.params.fast_len as usize;
    let slow_len = req.params.slow_len as usize;
    if fast_len == 0 || slow_len == 0 {
        return Err("periods must be > 0".to_string());
    }
    if fast_len > n || slow_len > n {
        return Err(format!(
            "period exceeds series length: fast_len={fast_len} slow_len={slow_len} n={n}"
        ));
    }

    validate_strategy_cfg(&req.strategy)?;
    validate_macz_params(&req.macz)?;
    validate_ott_params(&req.ott)?;
    if req.macz.use_volume && !candles.fields.volume {
        return Err("MACZ with volume requires a CSV with volume column.".to_string());
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
    let mut first_valid = fv_close.max(fv_ma);

    let halftrend_trend = compute_halftrend_trend(candles, &req.halftrend)?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ht);

    let macz_hist = compute_macz_hist(candles, &req.ma_source, &req.macz)?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    let fv_macz =
        first_finite_idx(&macz_hist).ok_or_else(|| "macz series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_macz);

    let ott_line = compute_ott_line(candles, &req.ma_source, &req.ott)?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }
    let fv_ott =
        first_finite_idx(&ott_line).ok_or_else(|| "ott series all non-finite".to_string())?;
    first_valid = first_valid.max(fv_ott);

    let trade_offset = if req.strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let max_p = fast_len.max(slow_len).max(1);
    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
    let start_t = t_valid.saturating_add(trade_offset);
    if start_t >= n {
        return Err("not enough candles after warmup for analysis".to_string());
    }

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", fast_ma, req.fast_ma_params.as_ref())?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", slow_ma, req.slow_ma_params.as_ref())?;

    let fast_periods = [req.params.fast_len];
    let (fast_mat, cols) = ma_matrix_cpu_f64(
        candles,
        fast_ma,
        &req.ma_source,
        &fast_periods,
        req.fast_ma_params.as_ref(),
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    let slow_periods = [req.params.slow_len];
    let (slow_mat, cols2) = ma_matrix_cpu_f64(
        candles,
        slow_ma,
        &req.ma_source,
        &slow_periods,
        req.slow_ma_params.as_ref(),
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    if cols2 != cols {
        return Err("MA matrix column mismatch".to_string());
    }

    let lr = compute_log_returns_f64(close);
    let bins = req.bins.unwrap_or(1024);
    Ok(
        ta_strategies::composite_cpu::eval_composite_curves_from_log_returns_and_mas_f64(
            &lr,
            ma_prices,
            &fast_mat,
            &slow_mat,
            &halftrend_trend,
            &macz_hist,
            &ott_line,
            start_t,
            &req.strategy,
            bins,
        ),
    )
}

fn validate_macz_params(p: &MaczGateParams) -> Result<(), String> {
    if p.fast_length == 0
        || p.slow_length == 0
        || p.signal_length == 0
        || p.lengthz == 0
        || p.length_stdev == 0
    {
        return Err("MACZ lengths must be > 0".to_string());
    }
    if p.fast_length >= p.slow_length {
        return Err("MACZ requires fast_length < slow_length".to_string());
    }
    if !(-2.0..=2.0).contains(&p.a) || !p.a.is_finite() {
        return Err("MACZ a must be finite and in [-2, 2]".to_string());
    }
    if !(-2.0..=2.0).contains(&p.b) || !p.b.is_finite() {
        return Err("MACZ b must be finite and in [-2, 2]".to_string());
    }
    if !(0.0..1.0).contains(&p.gamma) {
        return Err("MACZ gamma must be in [0, 1)".to_string());
    }
    Ok(())
}

fn compute_macz_hist(
    candles: &vector_ta::utilities::data_loader::Candles,
    source: &str,
    p: &MaczGateParams,
) -> Result<Vec<f64>, String> {
    validate_macz_params(p)?;
    let data = vector_ta::utilities::data_loader::source_type(candles, source);

    let params = vector_ta::indicators::macz::MaczParams {
        fast_length: Some(p.fast_length as usize),
        slow_length: Some(p.slow_length as usize),
        signal_length: Some(p.signal_length as usize),
        lengthz: Some(p.lengthz as usize),
        length_stdev: Some(p.length_stdev as usize),
        a: Some(p.a),
        b: Some(p.b),
        use_lag: Some(p.use_lag),
        gamma: Some(p.gamma),
    };

    let input = if p.use_volume && candles.fields.volume {
        vector_ta::indicators::macz::MaczInput::from_slice_with_volume(
            data,
            &candles.volume,
            params,
        )
    } else {
        vector_ta::indicators::macz::MaczInput::from_slice(data, params)
    };

    let out = vector_ta::indicators::macz::macz(&input).map_err(|e| e.to_string())?;
    Ok(out.values)
}

fn validate_ott_params(p: &OttGateParams) -> Result<(), String> {
    if p.period == 0 {
        return Err("OTT period must be > 0".to_string());
    }
    if !p.percent.is_finite() || p.percent < 0.0 {
        return Err("OTT percent must be finite and >= 0".to_string());
    }
    if p.ma_type.trim().is_empty() {
        return Err("OTT ma_type cannot be empty".to_string());
    }
    Ok(())
}

fn compute_ott_line(
    candles: &vector_ta::utilities::data_loader::Candles,
    source: &str,
    p: &OttGateParams,
) -> Result<Vec<f64>, String> {
    validate_ott_params(p)?;
    let data = vector_ta::utilities::data_loader::source_type(candles, source);
    let input = vector_ta::indicators::ott::OttInput::from_slice(
        data,
        vector_ta::indicators::ott::OttParams {
            period: Some(p.period as usize),
            percent: Some(p.percent),
            ma_type: Some(p.ma_type.clone()),
        },
    );
    let out = vector_ta::indicators::ott::ott(&input).map_err(|e| e.to_string())?;
    Ok(out.values)
}

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed_pairs: usize,
    total_pairs: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed_pairs, total_pairs, phase);
}

#[allow(clippy::too_many_arguments)]
fn optimize_composite_cpu_tiled(
    candles: &vector_ta::utilities::data_loader::Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_prices: &[f64],
    ott_line: &[f64],
    first_valid: usize,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    total_pairs: usize,
    heatmap_bins: usize,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let fast_info = lookup_ma_info(fast_ma_type).ok_or_else(|| "unknown fast MA".to_string())?;
    if !fast_info.supports_cpu_single {
        return Err(format!(
            "CPU backend does not support MA '{}'",
            fast_ma_type
        ));
    }
    let slow_info = lookup_ma_info(slow_ma_type).ok_or_else(|| "unknown slow MA".to_string())?;
    if !slow_info.supports_cpu_single {
        return Err(format!(
            "CPU backend does not support MA '{}'",
            slow_ma_type
        ));
    }

    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }
    if ott_prices.len() != n || ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let lr = compute_log_returns_f64(close);
    let trade_offset = if strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    let heatmap_cfg = if heatmap_bins > 0 {
        Some((
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        ))
    } else {
        None
    };
    if heatmap_bins > 0 {
        let (f_min, f_max, s_min, s_max) = heatmap_cfg.unwrap();
        agg = agg.with_heatmap(heatmap_bins, heatmap_bins, f_min, f_max, s_min, s_max);
    }

    let (pf_tile, ps_tile) = cpu_tile_sizes(fast_periods.len(), slow_periods.len(), n);
    let mut processed = 0usize;
    emit_progress(progress, processed, total_pairs, "composite_cpu");

    for f_chunk in fast_periods.chunks(pf_tile) {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let (fast_mat, cols) = ma_matrix_cpu_f64(
            candles,
            fast_ma_type,
            ma_source,
            f_chunk,
            fast_ma_params,
            fast_alma_offset,
            fast_alma_sigma,
        )?;
        for s_chunk in slow_periods.chunks(ps_tile) {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            let (slow_mat, cols2) = ma_matrix_cpu_f64(
                candles,
                slow_ma_type,
                ma_source,
                s_chunk,
                slow_ma_params,
                slow_alma_offset,
                slow_alma_sigma,
            )?;
            if cols2 != cols {
                return Err("MA matrix column mismatch".to_string());
            }

            if include_all {
                for (i, &fast_len) in f_chunk.iter().enumerate() {
                    let fast_row = &fast_mat[i * cols..(i + 1) * cols];
                    for (j, &slow_len) in s_chunk.iter().enumerate() {
                        if fast_len >= slow_len {
                            continue;
                        }
                        let slow_row = &slow_mat[j * cols..(j + 1) * cols];
                        let max_p = (fast_len as usize).max(slow_len as usize).max(1);
                        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                        let start_t = t_valid.saturating_add(trade_offset);
                        let metrics = ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64(
                            &lr,
                            ott_prices,
                            fast_row,
                            slow_row,
                            halftrend_trend,
                            macz_hist,
                            ott_line,
                            start_t,
                            strategy,
                        );
                        agg.push(
                            DoubleMaParams {
                                fast_len,
                                slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            },
                            metrics,
                        );
                        processed = processed.saturating_add(1);
                        if (processed & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }
                    }
                }
            } else {
                let compute_sharpe = objective == ObjectiveKind::Sharpe;
                let pf = f_chunk.len();
                let ps = s_chunk.len();
                let tile_pairs = pf
                    .checked_mul(ps)
                    .ok_or_else(|| "tile pairs overflow".to_string())?;

                let init = || {
                    let mut local = StreamAggregator::new(objective, top_k, false, n);
                    if let Some((f_min, f_max, s_min, s_max)) = heatmap_cfg {
                        local = local.with_heatmap(
                            heatmap_bins,
                            heatmap_bins,
                            f_min,
                            f_max,
                            s_min,
                            s_max,
                        );
                    }
                    local
                };

                let tile = (0..tile_pairs)
                    .into_par_iter()
                    .try_fold(init, |mut local, idx| {
                        if cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }
                        let i = idx / ps;
                        let j = idx - i * ps;
                        let fast_len = f_chunk[i];
                        let slow_len = s_chunk[j];
                        if fast_len >= slow_len {
                            return Ok(local);
                        }

                        let fast_row = &fast_mat[i * cols..(i + 1) * cols];
                        let slow_row = &slow_mat[j * cols..(j + 1) * cols];

                        let max_p = (fast_len as usize).max(slow_len as usize).max(1);
                        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
                        let start_t = t_valid.saturating_add(trade_offset);
                        let metrics = if compute_sharpe {
                            ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64(
                                &lr,
                                ott_prices,
                                fast_row,
                                slow_row,
                                halftrend_trend,
                                macz_hist,
                                ott_line,
                                start_t,
                                strategy,
                            )
                        } else {
                            ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_fast(
                                &lr,
                                ott_prices,
                                fast_row,
                                slow_row,
                                halftrend_trend,
                                macz_hist,
                                ott_line,
                                start_t,
                                strategy,
                            )
                        };
                        local.push(
                            DoubleMaParams {
                                fast_len,
                                slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            },
                            metrics,
                        );
                        Ok(local)
                    })
                    .try_reduce(init, |mut a, b| {
                        a.merge(b);
                        Ok(a)
                    })?;

                processed = processed.saturating_add(tile.num_combos());
                agg.merge(tile);
            }

            emit_progress(progress, processed, total_pairs, "composite_cpu");
        }
    }

    let mut result = agg
        .finalize()
        .ok_or_else(|| "grid search produced no result".to_string())?;

    if objective != ObjectiveKind::Sharpe && !include_all {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let mut fast_cache: std::collections::HashMap<u16, Vec<f64>> =
            std::collections::HashMap::new();
        let mut slow_cache: std::collections::HashMap<u16, Vec<f64>> =
            std::collections::HashMap::new();

        let cache_ma = |cache: &mut std::collections::HashMap<u16, Vec<f64>>,
                        period: u16,
                        ma_type: &str,
                        ma_params: Option<&HashMap<String, f64>>,
                        alma_offset: f64,
                        alma_sigma: f64|
         -> Result<(), String> {
            if cache.contains_key(&period) {
                return Ok(());
            }
            let periods = [period];
            let (values, _) = ma_matrix_cpu_f64(
                candles,
                ma_type,
                ma_source,
                &periods,
                ma_params,
                alma_offset,
                alma_sigma,
            )?;
            cache.insert(period, values);
            Ok(())
        };

        let recompute = |fast_len: u16,
                         slow_len: u16,
                         fast_cache: &mut std::collections::HashMap<u16, Vec<f64>>,
                         slow_cache: &mut std::collections::HashMap<u16, Vec<f64>>|
         -> Result<Metrics, String> {
            cache_ma(
                fast_cache,
                fast_len,
                fast_ma_type,
                fast_ma_params,
                fast_alma_offset,
                fast_alma_sigma,
            )?;
            cache_ma(
                slow_cache,
                slow_len,
                slow_ma_type,
                slow_ma_params,
                slow_alma_offset,
                slow_alma_sigma,
            )?;

            let fast = fast_cache
                .get(&fast_len)
                .ok_or_else(|| "fast MA cache miss".to_string())?;
            let slow = slow_cache
                .get(&slow_len)
                .ok_or_else(|| "slow MA cache miss".to_string())?;

            let max_p = (fast_len as usize).max(slow_len as usize).max(1);
            let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
            let start_t = t_valid.saturating_add(trade_offset);
            Ok(
                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64(
                    &lr,
                    ott_prices,
                    fast,
                    slow,
                    halftrend_trend,
                    macz_hist,
                    ott_line,
                    start_t,
                    strategy,
                ),
            )
        };

        result.best_metrics = recompute(
            result.best_params.fast_len,
            result.best_params.slow_len,
            &mut fast_cache,
            &mut slow_cache,
        )?;

        for (p, m) in result.top.iter_mut() {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            *m = recompute(p.fast_len, p.slow_len, &mut fast_cache, &mut slow_cache)?;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trending_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.5);
            vol.push(1_000.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn composite_cpu_smoke_small_grid_counts_and_shapes() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let req = CompositeCpuRequest {
            data_id: "test".to_string(),
            fast_range: (5, 7, 1),
            slow_range: (10, 11, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(16),
        };

        let res =
            run_composite_cpu_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("composite cpu run");

        assert_eq!(res.num_combos, 6);
        assert!(res.best_params.fast_len < res.best_params.slow_len);
        assert!(res.top.len() <= 5);
        assert!(res.heatmap.is_some());
        assert!(matches!(res.backend_used, BackendUsed::Cpu));
        assert!(matches!(res.mode_used, OptimizationModeResolved::Grid));
    }

    #[test]
    fn composite_cpu_drilldown_smoke_has_bins_and_values() {
        let candles = make_trending_candles(5000);

        let req = CompositeCpuDrilldownRequest {
            data_id: "test".to_string(),
            params: DoubleMaParamsResolved {
                fast_len: 10,
                slow_len: 30,
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
            },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            bins: Some(256),
        };

        let curves = compute_composite_cpu_drilldown_blocking_with_candles(req, &candles)
            .expect("composite drilldown");

        assert!(curves.equity.bins > 0);
        assert_eq!(curves.equity.bins, curves.drawdown.bins);
        assert_eq!(
            curves.equity.minmax.len(),
            curves.equity.bins.saturating_mul(2)
        );
        assert_eq!(
            curves.drawdown.minmax.len(),
            curves.drawdown.bins.saturating_mul(2)
        );

        assert!(curves.equity.minmax.iter().all(|v| v.is_finite()));
        assert!(curves
            .drawdown
            .minmax
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0));
    }
}
