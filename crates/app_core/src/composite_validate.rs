use crate::composite_cpu::{compute_macz_hist, compute_ott_line, MaczGateParams, OttGateParams};
use crate::cscv_pbo::{
    compute_cscv_pbo_from_slices, CscvCandidatePboStats, CscvPboConfig, CscvPboResult,
    SliceReturnStats,
};
use crate::double_ma::{
    compute_halftrend_trend, compute_log_returns_f64, first_finite_idx, lookup_ma_info,
    ma_matrix_cpu_f64, resolve_alma_params_for_side, validate_ma_kind, validate_strategy_cfg,
    HalfTrendRegimeFilter,
};
use crate::progress::ProgressSink;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use ta_strategies::double_ma::{Metrics, StrategyConfig, Trade};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompositeParams {
    pub fast_ma_type: String,
    pub slow_ma_type: String,
    pub fast_len: u16,
    pub slow_len: u16,
    pub halftrend: HalfTrendRegimeFilter,
    #[serde(default)]
    pub macz: MaczGateParams,
    #[serde(default)]
    pub ott: OttGateParams,
    #[serde(default)]
    pub strategy: StrategyConfig,
}

fn default_ma_source() -> String {
    "close".to_string()
}

fn default_seed() -> u64 {
    1
}

fn default_max_candidates() -> usize {
    32
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WalkForwardConfig {
    pub window_bars: usize,
    pub step_bars: usize,
}

fn default_walk_forward_window_bars() -> usize {
    20_000
}

fn default_walk_forward_step_bars() -> usize {
    20_000
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WalkForwardConfigOptional {
    #[serde(default = "default_walk_forward_window_bars")]
    pub window_bars: usize,
    #[serde(default = "default_walk_forward_step_bars")]
    pub step_bars: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalkForwardStats {
    pub windows: usize,
    pub sharpe_median: f64,
    pub sharpe_worst: f64,
    pub max_dd_median: f64,
    pub max_dd_worst: f64,
    pub pnl_median: f64,
    pub pnl_worst: f64,
    pub trades_median: u32,
    pub trades_worst: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PerturbConfig {
    #[serde(default)]
    pub iterations: usize,
    #[serde(default)]
    pub pct_min: f64,
    #[serde(default)]
    pub pct_max: f64,
    #[serde(default)]
    pub flip_bool_prob: f64,
}

fn default_mc_sims() -> usize {
    500
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TradeBootstrapConfig {
    #[serde(default = "default_mc_sims")]
    pub simulations: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct TradeBootstrapStats {
    pub simulations: usize,
    pub trade_count: usize,
    pub prob_pnl_positive: f64,
    pub pnl_p05: f64,
    pub pnl_p50: f64,
    pub pnl_p95: f64,
    pub max_dd_p50: f64,
    pub max_dd_p95: f64,
}

fn default_cross_max_datasets() -> usize {
    8
}

#[derive(Clone, Debug, Deserialize)]
pub struct CrossTickerDataset {
    pub csv_path: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CrossTickerConfig {
    #[serde(default)]
    pub datasets: Vec<CrossTickerDataset>,
    #[serde(default = "default_cross_max_datasets")]
    pub max_datasets: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeValidationRequest {
    pub data_id: String,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    pub candidates: Vec<CompositeParams>,
    #[serde(default)]
    pub walk_forward: Option<WalkForwardConfigOptional>,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct PerturbStats {
    pub iterations: usize,
    pub pct_min: f64,
    pub pct_max: f64,
    pub sharpe_mean: f64,
    pub sharpe_worst: f64,
    pub max_dd_mean: f64,
    pub max_dd_worst: f64,
    pub pnl_mean: f64,
    pub pnl_worst: f64,
    pub pnl_positive_frac: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerDatasetMeta {
    pub label: String,
    pub csv_path: String,
    pub bars: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerValidationSummary {
    pub datasets: Vec<CrossTickerDatasetMeta>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerStats {
    pub datasets: usize,
    pub sharpe_median: f64,
    pub sharpe_worst: f64,
    pub max_dd_median: f64,
    pub max_dd_worst: f64,
    pub pnl_median: f64,
    pub pnl_worst: f64,
    pub trades_median: u32,
    pub trades_worst: u32,
    pub pnl_positive_frac: f64,
    pub score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeValidationCandidateResult {
    pub params: CompositeParams,
    pub in_sample: Metrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walk_forward: Option<WalkForwardStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perturb: Option<PerturbStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_bootstrap: Option<TradeBootstrapStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker: Option<CrossTickerStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker_pareto: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cscv_pbo: Option<CscvCandidatePboStats>,
    pub robust_score: f64,
    pub robust_pareto: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeValidationResult {
    pub data_id: String,
    pub runtime_ms: u64,
    pub candidates: Vec<CompositeValidationCandidateResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_ticker: Option<CrossTickerValidationSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cscv_pbo: Option<CscvPboResult>,
}

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed: usize,
    total: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed, total, phase);
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn median(sorted: &[f64]) -> f64 {
    quantile(sorted, 0.5)
}

fn worst_min(xs: &[f64]) -> f64 {
    xs.iter()
        .copied()
        .fold(f64::INFINITY, |a, b| if b < a { b } else { a })
        .if_finite_or(0.0)
}

trait FiniteOr {
    fn if_finite_or(self, fallback: f64) -> f64;
}

impl FiniteOr for f64 {
    fn if_finite_or(self, fallback: f64) -> f64 {
        if self.is_finite() {
            self
        } else {
            fallback
        }
    }
}

fn worst_max(xs: &[f64]) -> f64 {
    xs.iter()
        .copied()
        .fold(f64::NEG_INFINITY, |a, b| if b > a { b } else { a })
        .if_finite_or(0.0)
}

fn median_u32(xs: &[u32]) -> u32 {
    if xs.is_empty() {
        return 0;
    }
    let mut ys = xs.to_vec();
    ys.sort_unstable();
    ys[ys.len() / 2]
}

fn worst_min_u32(xs: &[u32]) -> u32 {
    xs.iter().copied().min().unwrap_or(0)
}

fn worst_max_u32(xs: &[u32]) -> u32 {
    xs.iter().copied().max().unwrap_or(0)
}

struct CandidateSeries<'a> {
    n: usize,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    fast: Vec<f64>,
    slow: Vec<f64>,
    halftrend_trend: Vec<f64>,
    macz_hist: Vec<f64>,
    ott_line: Vec<f64>,
    start_t: usize,
    cfg: StrategyConfig,
}

fn build_candidate_series<'a>(
    candles: &vector_ta::utilities::data_loader::Candles,
    lr: &'a [f64],
    ott_prices: &'a [f64],
    base_first_valid: usize,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    params: &CompositeParams,
) -> Result<CandidateSeries<'a>, String> {
    validate_strategy_cfg(&params.strategy)?;

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }
    if lr.len() != n || ott_prices.len() != n {
        return Err("series length mismatch".to_string());
    }

    let fast_ma = params.fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = params.slow_ma_type.trim().to_ascii_lowercase();
    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Err("fast/slow MA type cannot be empty".to_string());
    }
    validate_ma_kind(&fast_ma)?;
    validate_ma_kind(&slow_ma)?;

    let fast_info = lookup_ma_info(&fast_ma).ok_or_else(|| "unknown fast MA".to_string())?;
    if !fast_info.supports_cpu_single {
        return Err(format!("CPU backend does not support MA '{}'", fast_ma));
    }
    let slow_info = lookup_ma_info(&slow_ma).ok_or_else(|| "unknown slow MA".to_string())?;
    if !slow_info.supports_cpu_single {
        return Err(format!("CPU backend does not support MA '{}'", slow_ma));
    }

    let halftrend_trend = compute_halftrend_trend(candles, &params.halftrend)?;
    if halftrend_trend.len() != n {
        return Err("HalfTrend trend series length mismatch".to_string());
    }

    if params.macz.use_volume && !candles.fields.volume {
        return Err("MACZ with volume requires a CSV with volume column.".to_string());
    }
    let macz_hist = compute_macz_hist(candles, ma_source, &params.macz)?;
    if macz_hist.len() != n {
        return Err("MACZ series length mismatch".to_string());
    }

    let ott_line = compute_ott_line(candles, ma_source, &params.ott)?;
    if ott_line.len() != n {
        return Err("OTT series length mismatch".to_string());
    }

    let (fast_alma_offset, fast_alma_sigma) =
        resolve_alma_params_for_side("fast", &fast_ma, fast_ma_params)?;
    let (slow_alma_offset, slow_alma_sigma) =
        resolve_alma_params_for_side("slow", &slow_ma, slow_ma_params)?;

    let fast_periods = [params.fast_len];
    let (fast_mat, cols) = ma_matrix_cpu_f64(
        candles,
        &fast_ma,
        ma_source,
        &fast_periods,
        fast_ma_params,
        fast_alma_offset,
        fast_alma_sigma,
    )?;
    if cols != n {
        return Err("fast MA series length mismatch".to_string());
    }

    let slow_periods = [params.slow_len];
    let (slow_mat, cols2) = ma_matrix_cpu_f64(
        candles,
        &slow_ma,
        ma_source,
        &slow_periods,
        slow_ma_params,
        slow_alma_offset,
        slow_alma_sigma,
    )?;
    if cols2 != n {
        return Err("slow MA series length mismatch".to_string());
    }

    let fv_fast =
        first_finite_idx(&fast_mat).ok_or_else(|| "fast MA series all non-finite".to_string())?;
    let fv_slow =
        first_finite_idx(&slow_mat).ok_or_else(|| "slow MA series all non-finite".to_string())?;
    let fv_ht = first_finite_idx(&halftrend_trend)
        .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
    let fv_macz = first_finite_idx(&macz_hist)
        .ok_or_else(|| "macz series all non-finite".to_string())?;
    let fv_ott = first_finite_idx(&ott_line).ok_or_else(|| "ott series all non-finite".to_string())?;

    let mut first_valid = base_first_valid;
    first_valid = first_valid.max(fv_fast);
    first_valid = first_valid.max(fv_slow);
    first_valid = first_valid.max(fv_ht);
    first_valid = first_valid.max(fv_macz);
    first_valid = first_valid.max(fv_ott);

    let trade_offset = if params.strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let max_p = (params.fast_len as usize).max(params.slow_len as usize).max(1);
    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
    let start_t = t_valid.saturating_add(trade_offset);
    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    Ok(CandidateSeries {
        n,
        lr,
        ott_prices,
        fast: fast_mat,
        slow: slow_mat,
        halftrend_trend,
        macz_hist,
        ott_line,
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
    let ott_prices = &series.ott_prices[..end_t];
    let fast = &series.fast[..end_t];
    let slow = &series.slow[..end_t];
    let ht = &series.halftrend_trend[..end_t];
    let macz = &series.macz_hist[..end_t];
    let ott_line = &series.ott_line[..end_t];

    ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64(
        lr,
        ott_prices,
        fast,
        slow,
        ht,
        macz,
        ott_line,
        start_t,
        &series.cfg,
    )
}

fn trades_for_series(series: &CandidateSeries<'_>) -> Vec<Trade> {
    let curves = ta_strategies::composite_cpu::eval_composite_curves_from_log_returns_and_mas_f64(
        series.lr,
        series.ott_prices,
        &series.fast,
        &series.slow,
        &series.halftrend_trend,
        &series.macz_hist,
        &series.ott_line,
        series.start_t,
        &series.cfg,
        1,
    );
    curves.trades
}

fn sanitize_cscv_slices(slices: usize) -> usize {
    let s = slices.clamp(4, 32);
    if (s & 1) == 0 { s } else { s.saturating_sub(1).max(4) }
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

    let end_t = start_t.saturating_add(slice_bars.saturating_mul(slices)).min(series.n);
    if end_t <= start_t + 1 {
        return out;
    }

    let trade_offset = if series.cfg.trade_on_next_bar { 1usize } else { 0usize };
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

        let ht = series.halftrend_trend[t_sig];
        if !ht.is_finite() {
            sign = 0;
        } else if ht < 0.5 {
            sign = sign.max(0);
        } else {
            sign = sign.min(0);
        }

        let mh = series.macz_hist[t_sig];
        if !mh.is_finite() {
            sign = 0;
        } else if mh > 0.0 {
            sign = sign.max(0);
        } else if mh < 0.0 {
            sign = sign.min(0);
        } else {
            sign = 0;
        }

        let px = series.ott_prices[t_sig];
        let ott = series.ott_line[t_sig];
        if !px.is_finite() || !ott.is_finite() {
            sign = 0;
        } else if px > ott {
            sign = sign.max(0);
        } else if px < ott {
            sign = sign.min(0);
        } else {
            sign = 0;
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
        let step_r = if pos == 0 { 0.0 } else { (pos as f64) * lr_t.exp_m1() };
        out[slice_idx].sum += step_r;
        out[slice_idx].sumsq += step_r * step_r;
    }

    out
}

fn trade_bootstrap_stats(
    trades: &[Trade],
    simulations: usize,
    rng: &mut StdRng,
) -> TradeBootstrapStats {
    let closed: Vec<f64> = trades
        .iter()
        .filter(|t| !t.open)
        .map(|t| t.pnl)
        .collect();
    let trade_count = closed.len();
    if simulations == 0 || trade_count == 0 {
        return TradeBootstrapStats {
            simulations,
            trade_count,
            prob_pnl_positive: 0.0,
            pnl_p05: 0.0,
            pnl_p50: 0.0,
            pnl_p95: 0.0,
            max_dd_p50: 0.0,
            max_dd_p95: 0.0,
        };
    }

    let mut pnls: Vec<f64> = Vec::with_capacity(simulations);
    let mut dds: Vec<f64> = Vec::with_capacity(simulations);
    let mut pos = 0usize;
    for _ in 0..simulations {
        let mut eq = 1.0_f64;
        let mut peak = 1.0_f64;
        let mut max_dd = 0.0_f64;
        for _ in 0..trade_count {
            let idx = rng.gen_range(0..trade_count);
            let r = (1.0 + closed[idx]).max(0.000_000_1);
            eq *= r;
            if eq > peak {
                peak = eq;
            }
            let dd = (peak - eq) / peak.max(0.000_000_1);
            if dd > max_dd {
                max_dd = dd;
            }
        }
        let pnl = eq - 1.0;
        if pnl > 0.0 {
            pos += 1;
        }
        pnls.push(pnl);
        dds.push(max_dd);
    }
    pnls.sort_by(|a, b| a.total_cmp(b));
    dds.sort_by(|a, b| a.total_cmp(b));
    TradeBootstrapStats {
        simulations,
        trade_count,
        prob_pnl_positive: pos as f64 / simulations as f64,
        pnl_p05: quantile(&pnls, 0.05),
        pnl_p50: quantile(&pnls, 0.5),
        pnl_p95: quantile(&pnls, 0.95),
        max_dd_p50: quantile(&dds, 0.5),
        max_dd_p95: quantile(&dds, 0.95),
    }
}

fn repair_params(p: &mut CompositeParams) {
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

    // HalfTrend
    if p.halftrend.amplitude == 0 {
        p.halftrend.amplitude = 1;
    }
    if p.halftrend.atr_period == 0 {
        p.halftrend.atr_period = 1;
    }
    if !p.halftrend.channel_deviation.is_finite() || p.halftrend.channel_deviation <= 0.0 {
        p.halftrend.channel_deviation = 2.0;
    }

    // MACZ
    if p.macz.fast_length == 0 {
        p.macz.fast_length = 1;
    }
    if p.macz.slow_length == 0 {
        p.macz.slow_length = 2;
    }
    if p.macz.fast_length >= p.macz.slow_length {
        p.macz.slow_length = p.macz.fast_length.saturating_add(1).max(2);
    }
    if p.macz.signal_length == 0 {
        p.macz.signal_length = 1;
    }
    if p.macz.lengthz == 0 {
        p.macz.lengthz = 1;
    }
    if p.macz.length_stdev == 0 {
        p.macz.length_stdev = 1;
    }
    if !p.macz.a.is_finite() {
        p.macz.a = 0.0;
    }
    if !p.macz.b.is_finite() {
        p.macz.b = 0.0;
    }
    p.macz.a = p.macz.a.clamp(-2.0, 2.0);
    p.macz.b = p.macz.b.clamp(-2.0, 2.0);
    if !p.macz.gamma.is_finite() {
        p.macz.gamma = 0.0;
    }
    p.macz.gamma = p.macz.gamma.clamp(0.0, 0.999_999);

    // OTT
    if p.ott.period == 0 {
        p.ott.period = 1;
    }
    if !p.ott.percent.is_finite() || p.ott.percent < 0.0 {
        p.ott.percent = 0.0;
    }
    if p.ott.ma_type.trim().is_empty() {
        p.ott.ma_type = "VAR".to_string();
    }

    // Strategy config
    if !p.strategy.commission.is_finite() {
        p.strategy.commission = 0.0;
    }
    p.strategy.commission = p.strategy.commission.clamp(0.0, 0.999_999);
    if !p.strategy.eps_rel.is_finite() || p.strategy.eps_rel < 0.0 {
        p.strategy.eps_rel = 0.0;
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
    base: &CompositeParams,
    cfg: &PerturbConfig,
    rng: &mut StdRng,
) -> CompositeParams {
    let mut p = base.clone();

    let pct_min = cfg.pct_min.abs();
    let pct_max = cfg.pct_max.abs().max(pct_min);
    let flip_bool_prob = cfg.flip_bool_prob.clamp(0.0, 1.0);

    // MA lengths
    p.fast_len = apply_pct_u16(p.fast_len, rand_mult(rng, pct_min, pct_max));
    p.slow_len = apply_pct_u16(p.slow_len, rand_mult(rng, pct_min, pct_max));

    // HalfTrend
    p.halftrend.amplitude =
        apply_pct_u16(p.halftrend.amplitude, rand_mult(rng, pct_min, pct_max));
    p.halftrend.atr_period =
        apply_pct_u16(p.halftrend.atr_period, rand_mult(rng, pct_min, pct_max));
    p.halftrend.channel_deviation =
        apply_pct_f64(p.halftrend.channel_deviation, rand_mult(rng, pct_min, pct_max));

    // MACZ
    p.macz.fast_length = apply_pct_u16(p.macz.fast_length, rand_mult(rng, pct_min, pct_max));
    p.macz.slow_length = apply_pct_u16(p.macz.slow_length, rand_mult(rng, pct_min, pct_max));
    p.macz.signal_length = apply_pct_u16(p.macz.signal_length, rand_mult(rng, pct_min, pct_max));
    p.macz.lengthz = apply_pct_u16(p.macz.lengthz, rand_mult(rng, pct_min, pct_max));
    p.macz.length_stdev = apply_pct_u16(p.macz.length_stdev, rand_mult(rng, pct_min, pct_max));
    p.macz.a = apply_pct_f64(p.macz.a, rand_mult(rng, pct_min, pct_max));
    p.macz.b = apply_pct_f64(p.macz.b, rand_mult(rng, pct_min, pct_max));
    p.macz.gamma = apply_pct_f64(p.macz.gamma, rand_mult(rng, pct_min, pct_max));
    if rng.gen_bool(flip_bool_prob) {
        p.macz.use_lag = !p.macz.use_lag;
    }
    if rng.gen_bool(flip_bool_prob) {
        p.macz.use_volume = !p.macz.use_volume;
    }

    // OTT
    p.ott.period = apply_pct_u16(p.ott.period, rand_mult(rng, pct_min, pct_max));
    p.ott.percent = apply_pct_f64(p.ott.percent, rand_mult(rng, pct_min, pct_max));
    if rng.gen_bool(flip_bool_prob) {
        // Keep ma_type stable most of the time; flipping is not well-defined here.
    }

    // Strategy config
    p.strategy.commission =
        apply_pct_f64(p.strategy.commission, rand_mult(rng, pct_min, pct_max));
    p.strategy.eps_rel =
        apply_pct_f64(p.strategy.eps_rel, rand_mult(rng, pct_min, pct_max)).abs();
    if rng.gen_bool(flip_bool_prob) {
        p.strategy.long_only = !p.strategy.long_only;
    }
    if rng.gen_bool(flip_bool_prob) {
        p.strategy.allow_flip = !p.strategy.allow_flip;
    }
    if rng.gen_bool(flip_bool_prob) {
        p.strategy.trade_on_next_bar = !p.strategy.trade_on_next_bar;
    }

    repair_params(&mut p);
    p
}

fn robust_score_from_stats(in_sample: &Metrics, wf: Option<&WalkForwardStats>) -> f64 {
    if let Some(w) = wf {
        w.sharpe_median - 2.0 * w.max_dd_worst
    } else {
        in_sample.sharpe - 2.0 * in_sample.max_dd
    }
}

fn fold_cscv_into_robust_score(base: f64, cscv: Option<&CscvCandidatePboStats>) -> f64 {
    let Some(st) = cscv else {
        return base;
    };
    let Some(oos) = st.oos_metric_p50 else {
        return base;
    };
    if oos.is_finite() { base.min(oos) } else { base }
}

pub fn run_composite_validation_blocking_with_candles(
    mut req: CompositeValidationRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<CompositeValidationResult, String> {
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
    let fv_close = first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma = first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let base_first_valid = fv_close.max(fv_ma);
    let lr = compute_log_returns_f64(close);

    let total = req.candidates.len();
    emit_progress(progress, 0, total, "composite_validate");

    let processed = AtomicUsize::new(0);
    let seed = req.seed;
    let walk_cfg = req.walk_forward.clone();
    let perturb_cfg = req.perturb.clone();
    let mc_cfg = req.trade_bootstrap.clone();
    let ma_source = req.ma_source.clone();
    let fast_ma_params = req.fast_ma_params.clone();
    let slow_ma_params = req.slow_ma_params.clone();
    let cscv_cfg = req.cscv_pbo.clone();

    let tmp: Vec<(CompositeValidationCandidateResult, usize)> = req
        .candidates
        .par_iter()
        .enumerate()
        .map(|(i, p)| {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }

            let mut rng = StdRng::seed_from_u64(seed ^ (i as u64).wrapping_mul(0x9e3779b97f4a7c15));

            let mut p2 = p.clone();
            repair_params(&mut p2);

            let series = build_candidate_series(
                candles,
                &lr,
                ma_prices,
                base_first_valid,
                &ma_source,
                fast_ma_params.as_ref(),
                slow_ma_params.as_ref(),
                &p2,
            )?;
            let series_start_t = series.start_t;

            let in_sample = eval_metrics_window(&series, series.start_t, series.n);

            let walk_forward = if let Some(cfg) = walk_cfg.as_ref() {
                let window = cfg.window_bars.max(64);
                let step = cfg.step_bars.max(1);

                let mut sharpes: Vec<f64> = Vec::new();
                let mut dds: Vec<f64> = Vec::new();
                let mut pnls: Vec<f64> = Vec::new();
                let mut trades: Vec<u32> = Vec::new();

                let mut t0 = series.start_t;
                while t0 + 2 <= series.n {
                    let end = t0.saturating_add(window).min(series.n);
                    if end <= t0 + 1 {
                        break;
                    }
                    let m = eval_metrics_window(&series, t0, end);
                    sharpes.push(m.sharpe);
                    dds.push(m.max_dd);
                    pnls.push(m.pnl);
                    trades.push(m.trades);
                    if end == series.n {
                        break;
                    }
                    t0 = t0.saturating_add(step);
                }

                sharpes.sort_by(|a, b| a.total_cmp(b));
                dds.sort_by(|a, b| a.total_cmp(b));
                pnls.sort_by(|a, b| a.total_cmp(b));

                Some(WalkForwardStats {
                    windows: sharpes.len(),
                    sharpe_median: median(&sharpes),
                    sharpe_worst: worst_min(&sharpes),
                    max_dd_median: median(&dds),
                    max_dd_worst: worst_max(&dds),
                    pnl_median: median(&pnls),
                    pnl_worst: worst_min(&pnls),
                    trades_median: median_u32(&trades),
                    trades_worst: worst_min_u32(&trades),
                })
            } else {
                None
            };

            let perturb = if let Some(cfg) = perturb_cfg.as_ref() {
                let iters = cfg.iterations.min(1024);
                if iters == 0 {
                    None
                } else {
                    let mut sharpes: Vec<f64> = Vec::with_capacity(iters);
                    let mut dds: Vec<f64> = Vec::with_capacity(iters);
                    let mut pnls: Vec<f64> = Vec::with_capacity(iters);
                    let mut pnl_pos: usize = 0;

                    for _ in 0..iters {
                        if cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }
                        let pp = perturb_params(&p2, cfg, &mut rng);
                        let s2 = build_candidate_series(
                            candles,
                            &lr,
                            ma_prices,
                            base_first_valid,
                            &ma_source,
                            fast_ma_params.as_ref(),
                            slow_ma_params.as_ref(),
                            &pp,
                        )?;
                        let m = eval_metrics_window(&s2, s2.start_t, s2.n);
                        if m.pnl > 0.0 {
                            pnl_pos += 1;
                        }
                        sharpes.push(m.sharpe);
                        dds.push(m.max_dd);
                        pnls.push(m.pnl);
                    }

                    let mean = |xs: &[f64]| -> f64 {
                        if xs.is_empty() {
                            return 0.0;
                        }
                        let mut sum = 0.0_f64;
                        let mut c = 0usize;
                        for &x in xs {
                            if x.is_finite() {
                                sum += x;
                                c += 1;
                            }
                        }
                        if c > 0 {
                            sum / c as f64
                        } else {
                            0.0
                        }
                    };

                    let sharpe_mean = mean(&sharpes);
                    let dd_mean = mean(&dds);
                    let pnl_mean = mean(&pnls);

                    let sharpe_worst = worst_min(&sharpes);
                    let dd_worst = worst_max(&dds);
                    let pnl_worst = worst_min(&pnls);

                    Some(PerturbStats {
                        iterations: iters,
                        pct_min: cfg.pct_min,
                        pct_max: cfg.pct_max,
                        sharpe_mean,
                        sharpe_worst,
                        max_dd_mean: dd_mean,
                        max_dd_worst: dd_worst,
                        pnl_mean,
                        pnl_worst,
                        pnl_positive_frac: pnl_pos as f64 / iters as f64,
                    })
                }
            } else {
                None
            };

            let trade_bootstrap = if let Some(cfg) = mc_cfg.as_ref() {
                let sims = cfg.simulations.min(50_000);
                let trades = trades_for_series(&series);
                Some(trade_bootstrap_stats(&trades, sims, &mut rng))
            } else {
                None
            };

            let robust_score = robust_score_from_stats(&in_sample, walk_forward.as_ref());

            let done = processed.fetch_add(1, Ordering::Relaxed).saturating_add(1);
            if (done & 0x3) == 0 || done == total {
                emit_progress(progress, done, total, "composite_validate");
            }

            Ok((
                CompositeValidationCandidateResult {
                    params: p2,
                    in_sample,
                    walk_forward,
                    perturb,
                    trade_bootstrap,
                    cross_ticker: None,
                    cross_ticker_pareto: None,
                    cscv_pbo: None,
                    robust_score,
                    robust_pareto: false,
                },
                series_start_t,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut out: Vec<CompositeValidationCandidateResult> = Vec::with_capacity(tmp.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(tmp.len());
    for (cand, st) in tmp {
        out.push(cand);
        start_ts.push(st);
    }
    let common_start_t = start_ts.into_iter().max().unwrap_or(0);

    // Mark robust Pareto front based on (Sharpe median, MaxDD worst) if walk-forward is enabled;
    // otherwise use in-sample (Sharpe, MaxDD).
    for i in 0..out.len() {
        let (xi, yi) = if let Some(w) = out[i].walk_forward.as_ref() {
            (w.sharpe_median, w.max_dd_worst)
        } else {
            (out[i].in_sample.sharpe, out[i].in_sample.max_dd)
        };

        let mut dominated = false;
        for j in 0..out.len() {
            if i == j {
                continue;
            }
            let (xj, yj) = if let Some(w) = out[j].walk_forward.as_ref() {
                (w.sharpe_median, w.max_dd_worst)
            } else {
                (out[j].in_sample.sharpe, out[j].in_sample.max_dd)
            };
            if xj >= xi && yj <= yi && (xj > xi || yj < yi) {
                dominated = true;
                break;
            }
        }
        out[i].robust_pareto = !dominated;
    }

    let mut cscv_pbo: Option<CscvPboResult> = None;
    if let Some(cfg) = cscv_cfg.as_ref() {
        let slices = sanitize_cscv_slices(cfg.slices);
        let slice_bars = candles.close.len().saturating_sub(common_start_t) / slices;
        if slice_bars >= 2 && out.len() >= 2 {
            emit_progress(progress, 0, out.len(), "composite_validate_cscv_prep");
            let mut by_cand: Vec<Vec<SliceReturnStats>> = Vec::with_capacity(out.len());
            for (i, cand) in out.iter().enumerate() {
                if cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".to_string());
                }
                let series = build_candidate_series(
                    candles,
                    &lr,
                    ma_prices,
                    base_first_valid,
                    &ma_source,
                    fast_ma_params.as_ref(),
                    slow_ma_params.as_ref(),
                    &cand.params,
                )?;
                by_cand.push(slice_return_stats_for_series(
                    &series,
                    common_start_t,
                    slice_bars,
                    slices,
                ));
                let done = i.saturating_add(1);
                if (done & 0x7) == 0 || done == out.len() {
                    emit_progress(progress, done, out.len(), "composite_validate_cscv_prep");
                }
            }

            let (res, cand_stats) = compute_cscv_pbo_from_slices(
                &by_cand,
                slice_bars,
                cfg,
                seed,
                cancel,
                progress,
            )?;
            for (cand, st) in out.iter_mut().zip(cand_stats.into_iter()) {
                cand.cscv_pbo = Some(st);
            }
            cscv_pbo = Some(res);
        } else {
            cscv_pbo = Some(CscvPboResult {
                slices,
                slice_bars,
                candidates: out.len(),
                combinations: 0,
                combinations_possible: 0,
                pbo: 0.0,
                lambda_p25: 0.0,
                lambda_p50: 0.0,
                lambda_p75: 0.0,
                degradation_slope: 0.0,
                degradation_intercept: 0.0,
                degradation_r2: 0.0,
                threshold: cfg.threshold,
                prob_oos_below_threshold: 0.0,
                skipped_reason: Some("CSCV/PBO skipped (not enough bars after warmup).".to_string()),
            });
        }
    }

    for cand in out.iter_mut() {
        cand.robust_score = fold_cscv_into_robust_score(cand.robust_score, cand.cscv_pbo.as_ref());
    }

    out.sort_by(|a, b| b.robust_score.total_cmp(&a.robust_score));

    let runtime_ms = start.elapsed().as_millis() as u64;
    Ok(CompositeValidationResult {
        data_id: req.data_id,
        runtime_ms,
        candidates: out,
        cross_ticker: None,
        cscv_pbo,
    })
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
            let base = 100.0 + i as f64 * 0.1;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.25);
            vol.push(1_000.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn composite_validation_smoke_runs_and_returns_stats() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let base = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base.clone()],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1000,
                step_bars: 1000,
            }),
            perturb: Some(PerturbConfig {
                iterations: 3,
                pct_min: 0.01,
                pct_max: 0.01,
                flip_bool_prob: 0.0,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 50 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 32,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");

        assert_eq!(res.candidates.len(), 1);
        let c = &res.candidates[0];
        assert!(c.in_sample.sharpe.is_finite());
        assert!(c.walk_forward.as_ref().unwrap().windows > 0);
        assert_eq!(c.perturb.as_ref().unwrap().iterations, 3);
        assert_eq!(c.trade_bootstrap.as_ref().unwrap().simulations, 50);
        assert!(c.robust_score.is_finite());
    }

    #[test]
    fn fold_cscv_into_robust_score_uses_oos_median_sharpe_as_lower_bound() {
        let st_low = CscvCandidatePboStats {
            is_selected_frac: 0.5,
            oos_rank_frac_p50: Some(0.8),
            oos_metric_p50: Some(0.25),
        };
        assert!((fold_cscv_into_robust_score(1.0, Some(&st_low)) - 0.25).abs() < 1e-12);

        let st_high = CscvCandidatePboStats {
            is_selected_frac: 0.5,
            oos_rank_frac_p50: Some(0.8),
            oos_metric_p50: Some(2.0),
        };
        assert!((fold_cscv_into_robust_score(1.0, Some(&st_high)) - 1.0).abs() < 1e-12);

        let st_none = CscvCandidatePboStats {
            is_selected_frac: 0.0,
            oos_rank_frac_p50: None,
            oos_metric_p50: None,
        };
        assert!((fold_cscv_into_robust_score(1.0, Some(&st_none)) - 1.0).abs() < 1e-12);
        assert!((fold_cscv_into_robust_score(1.0, None) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn composite_validation_repairs_invalid_params_and_caps_candidates() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let bad = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 0,
            slow_len: 0,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 0,
                channel_deviation: -1.0,
                atr_period: 0,
            },
            macz: MaczGateParams {
                fast_length: 0,
                slow_length: 0,
                signal_length: 0,
                lengthz: 0,
                length_stdev: 0,
                a: f64::NAN,
                b: f64::INFINITY,
                gamma: 2.0,
                use_lag: false,
                use_volume: false,
            },
            ott: OttGateParams {
                period: 0,
                percent: -10.0,
                ma_type: "".to_string(),
            },
            strategy: StrategyConfig {
                long_only: true,
                allow_flip: true,
                trade_on_next_bar: true,
                commission: f64::NAN,
                eps_rel: -1.0,
            },
        };

        let ok = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![bad, ok],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 1,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert_eq!(res.candidates.len(), 1);

        let p = &res.candidates[0].params;
        assert!(p.fast_len > 0);
        assert!(p.slow_len > p.fast_len);
        assert!(p.halftrend.amplitude > 0);
        assert!(p.halftrend.atr_period > 0);
        assert!(p.halftrend.channel_deviation.is_finite());
        assert!(p.macz.fast_length > 0);
        assert!(p.macz.slow_length > p.macz.fast_length);
        assert!(p.macz.signal_length > 0);
        assert!(p.macz.lengthz > 0);
        assert!(p.macz.length_stdev > 0);
        assert!(p.macz.a.is_finite());
        assert!(p.macz.b.is_finite());
        assert!((0.0..1.0).contains(&p.macz.gamma));
        assert!(p.ott.period > 0);
        assert!(p.ott.percent.is_finite());
        assert!(!p.ott.ma_type.trim().is_empty());
        assert!(p.strategy.commission.is_finite());
        assert!((0.0..1.0).contains(&p.strategy.commission));
        assert!(p.strategy.eps_rel.is_finite());
        assert!(p.strategy.eps_rel >= 0.0);
    }

    #[test]
    fn composite_validation_trade_bootstrap_quantiles_monotonic() {
        let candles = make_trending_candles(5000);
        let cancel = AtomicBool::new(false);

        let base = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            strategy: StrategyConfig::default(),
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 200 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 32,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");

        let c = &res.candidates[0];
        let mc = c.trade_bootstrap.as_ref().expect("mc");
        assert_eq!(mc.simulations, 200);
        assert!((0.0..=1.0).contains(&mc.prob_pnl_positive));
        assert!(mc.pnl_p05 <= mc.pnl_p50);
        assert!(mc.pnl_p50 <= mc.pnl_p95);
        assert!(mc.max_dd_p50 <= mc.max_dd_p95);
    }

    #[test]
    fn composite_validation_cscv_pbo_runs_and_populates_fields() {
        let candles = make_trending_candles(4096);
        let cancel = AtomicBool::new(false);

        let base = CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            strategy: StrategyConfig::default(),
        };

        let alt = CompositeParams {
            fast_len: 12,
            slow_len: 40,
            ..base.clone()
        };

        let req = CompositeValidationRequest {
            data_id: "test".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base, alt],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(crate::cscv_pbo::CscvPboConfig {
                slices: 4,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            seed: 1,
            max_candidates: 32,
        };

        let res = run_composite_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");

        let cscv = res.cscv_pbo.as_ref().expect("cscv");
        assert_eq!(cscv.combinations, 6);
        assert!(cscv.pbo.is_finite());

        assert_eq!(res.candidates.len(), 2);
        for c in &res.candidates {
            assert!(c.cscv_pbo.is_some());
        }
    }
}
