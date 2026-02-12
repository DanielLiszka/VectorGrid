use crate::composite_validate::{
    CrossTickerConfig, CrossTickerStats, CrossTickerValidationSummary, PerturbConfig, PerturbStats,
    TradeBootstrapConfig, TradeBootstrapStats, WalkForwardConfigOptional, WalkForwardStats,
};
use crate::cscv_pbo::{
    compute_cscv_pbo_from_slices, CscvCandidatePboStats, CscvPboConfig, CscvPboResult,
    SliceReturnStats,
};
use crate::double_ma::{
    compute_halftrend_trend, compute_log_returns_f64, first_finite_idx, ma_matrix_cpu_f64,
    resolve_alma_params_for_side, validate_ma_kind, validate_strategy_cfg, HalfTrendRegimeFilter,
};
use crate::iterative_sim::{
    simulate_iterative_from_signals, IterativeMetricsDelta, IterativeSimConfig, OhlcSlice,
    ValidationEngine,
};
use crate::progress::ProgressSink;
use crate::stats_common::{median, median_u32, quantile, worst_max, worst_min, worst_min_u32};
use crate::validation_pipeline::ValidationPostCandidate;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    pub candidates: Vec<DoubleMaValidationParams>,
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
    #[serde(default)]
    pub validation_engine: ValidationEngine,
    #[serde(default)]
    pub iterative: IterativeSimConfig,
    #[serde(default)]
    pub min_psr: Option<f64>,
    #[serde(default)]
    pub min_dsr: Option<f64>,
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

impl ValidationPostCandidate for DoubleMaValidationCandidateResult {
    fn in_sample_metrics(&self) -> &Metrics {
        &self.in_sample
    }

    fn iterative_in_sample_metrics(&self) -> Option<&Metrics> {
        self.iterative_in_sample.as_ref()
    }

    fn walk_forward_sharpe_maxdd(&self) -> Option<(f64, f64)> {
        self.walk_forward
            .as_ref()
            .map(|w| (w.sharpe_median, w.max_dd_worst))
    }

    fn cscv_oos_metric_p50(&self) -> Option<f64> {
        self.cscv_pbo.as_ref().and_then(|c| c.oos_metric_p50)
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

    fn robust_score(&self) -> f64 {
        self.robust_score
    }

    fn set_robust_score(&mut self, v: f64) {
        self.robust_score = v;
    }

    fn set_robust_pareto(&mut self, v: bool) {
        self.robust_pareto = v;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DoubleMaValidationResult {
    pub data_id: String,
    pub runtime_ms: u64,
    pub candidates: Vec<DoubleMaValidationCandidateResult>,
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

#[derive(Clone)]
struct CandidateSeries<'a> {
    n: usize,
    lr: &'a [f64],
    fast: Vec<f64>,
    slow: Vec<f64>,
    halftrend_trend: Option<Vec<f64>>,
    start_t: usize,
    cfg: StrategyConfig,
}

fn build_candidate_series<'a>(
    candles: &vector_ta::utilities::data_loader::Candles,
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

    let halftrend_trend: Option<Vec<f64>> = match params.halftrend_filter.as_ref() {
        Some(f) => Some(compute_halftrend_trend(candles, f)?),
        None => None,
    };

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
    let mut start_t = t_sig0.saturating_add(trade_offset).max(base_first_valid);

    if let Some(trend) = halftrend_trend.as_ref() {
        if trend.len() != n {
            return Err("HalfTrend trend series length mismatch".to_string());
        }
        let fv_ht = first_finite_idx(trend)
            .ok_or_else(|| "halftrend trend series all non-finite".to_string())?;
        start_t = start_t.max(fv_ht);
    }

    if start_t >= n {
        return Err("not enough candles after warmup".to_string());
    }

    Ok(CandidateSeries {
        n,
        lr,
        fast: fast_mat,
        slow: slow_mat,
        halftrend_trend,
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
    let fast = &series.fast[..end_t];
    let slow = &series.slow[..end_t];

    if let Some(ht) = series.halftrend_trend.as_ref() {
        let ht2 = &ht[..end_t];
        ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter(
            lr,
            fast,
            slow,
            ht2,
            start_t,
            &series.cfg,
        )
    } else {
        ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f64(
            lr,
            fast,
            slow,
            start_t,
            &series.cfg,
        )
    }
}

fn trades_for_series(series: &CandidateSeries<'_>) -> Vec<Trade> {
    if let Some(ht) = series.halftrend_trend.as_ref() {
        ta_strategies::double_ma::eval_double_ma_curves_from_log_returns_and_mas_f64_with_halftrend_filter(
            series.lr,
            &series.fast,
            &series.slow,
            ht,
            series.start_t,
            &series.cfg,
            1,
        )
        .trades
    } else {
        ta_strategies::double_ma::eval_double_ma_curves_from_log_returns_and_mas_f64(
            series.lr,
            &series.fast,
            &series.slow,
            series.start_t,
            &series.cfg,
            1,
        )
        .trades
    }
}

fn desired_positions_for_series(
    series: &CandidateSeries<'_>,
    start_t: usize,
    end_t: usize,
) -> Vec<i32> {
    let n = series.n;
    let end_t = end_t.min(n);
    let mut out: Vec<i32> = vec![0; n];
    if start_t >= end_t || n == 0 {
        return out;
    }

    let trade_offset = if series.cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    for t in start_t..end_t {
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

        out[t] = sign;
    }
    out
}

fn sanitize_cscv_slices(slices: usize) -> usize {
    let s = slices.clamp(4, 32);
    if (s & 1) == 0 {
        s
    } else {
        s.saturating_sub(1).max(4)
    }
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

fn trade_bootstrap_stats(
    trades: &[Trade],
    simulations: usize,
    rng: &mut StdRng,
) -> TradeBootstrapStats {
    let closed: Vec<f64> = trades.iter().filter(|t| !t.open).map(|t| t.pnl).collect();
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

    let processed = AtomicUsize::new(0);
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
    let min_psr = crate::validation_stats::sanitize_probability_gate(req.min_psr);
    let min_dsr = crate::validation_stats::sanitize_probability_gate(req.min_dsr);

    let tmp: Vec<(DoubleMaValidationCandidateResult, usize, usize)> = req
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

            let (iterative_in_sample, iterative_delta) =
                if validation_engine == ValidationEngine::Iterative {
                    let desired = desired_positions_for_series(&series, series.start_t, series.n);
                    let iterative = simulate_iterative_from_signals(
                        series.lr,
                        OhlcSlice {
                            open: &candles.open,
                            high: &candles.high,
                            low: &candles.low,
                            close: &candles.close,
                        },
                        &desired,
                        series.start_t,
                        &series.cfg,
                        &iterative_cfg,
                    );
                    let delta = IterativeMetricsDelta::from_pair(&in_sample, &iterative);
                    (Some(iterative), Some(delta))
                } else {
                    (None, None)
                };

            let robust_base = iterative_in_sample.as_ref().unwrap_or(&in_sample);
            let robust_score = crate::validation_pipeline::robust_score_from_metrics(
                robust_base,
                walk_forward.as_ref().map(|w| (w.sharpe_median, w.max_dd_worst)),
            );

            let done = processed.fetch_add(1, Ordering::Relaxed).saturating_add(1);
            if (done & 0x3) == 0 || done == total {
                emit_progress(progress, done, total, "double_ma_validate");
            }

            Ok((
                DoubleMaValidationCandidateResult {
                    params: p2,
                    in_sample,
                    iterative_in_sample,
                    iterative_delta,
                    psr: None,
                    dsr: None,
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
                series.n.saturating_sub(series_start_t),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut out: Vec<DoubleMaValidationCandidateResult> = Vec::with_capacity(tmp.len());
    let mut start_ts: Vec<usize> = Vec::with_capacity(tmp.len());
    let mut sample_obs: Vec<usize> = Vec::with_capacity(tmp.len());
    for (cand, st, obs) in tmp {
        out.push(cand);
        start_ts.push(st);
        sample_obs.push(obs.max(2));
    }
    let common_start_t = start_ts.into_iter().max().unwrap_or(0);

    crate::validation_pipeline::compute_probability_metrics_and_gate(
        &mut out,
        &sample_obs,
        min_psr,
        min_dsr,
    );
    crate::validation_pipeline::mark_robust_pareto(&mut out);

    let mut cscv_pbo: Option<CscvPboResult> = None;
    if let Some(cfg) = cscv_cfg.as_ref() {
        let slices = sanitize_cscv_slices(cfg.slices);
        let slice_bars = candles.close.len().saturating_sub(common_start_t) / slices;
        if slice_bars >= 2 && out.len() >= 2 {
            emit_progress(progress, 0, out.len(), "double_ma_validate_cscv_prep");
            let mut by_cand: Vec<Vec<SliceReturnStats>> = Vec::with_capacity(out.len());
            for (i, cand) in out.iter().enumerate() {
                if cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".to_string());
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
                by_cand.push(slice_return_stats_for_series(
                    &series,
                    common_start_t,
                    slice_bars,
                    slices,
                ));
                let done = i.saturating_add(1);
                if (done & 0x7) == 0 || done == out.len() {
                    emit_progress(progress, done, out.len(), "double_ma_validate_cscv_prep");
                }
            }

            let (res, cand_stats) =
                compute_cscv_pbo_from_slices(&by_cand, slice_bars, cfg, seed, cancel, progress)?;
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
                skipped_reason: Some(
                    "CSCV/PBO skipped (not enough bars after warmup).".to_string(),
                ),
            });
        }
    }

    crate::validation_pipeline::fold_cscv_into_robust_and_sort(&mut out);

    let runtime_ms = start.elapsed().as_millis() as u64;
    Ok(DoubleMaValidationResult {
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
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 2000,
                step_bars: 1000,
            }),
            perturb: Some(PerturbConfig {
                iterations: 3,
                pct_min: 0.01,
                pct_max: 0.02,
                flip_bool_prob: 0.02,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 50 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
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
        assert!(c.robust_score.is_finite());
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
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![a, b],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 8,
                max_combinations: 1000,
                threshold: 0.0,
            }),
            seed: 1,
            max_candidates: 32,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
        };

        let res = run_double_ma_validation_blocking_with_candles(req, &candles, &cancel, None)
            .expect("validation run");
        assert_eq!(res.candidates.len(), 2);
        assert!(res.cscv_pbo.is_some());
        assert!(res.candidates[0].cscv_pbo.is_some());
        assert!(res.candidates[1].cscv_pbo.is_some());
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
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 9,
            max_candidates: 32,
            validation_engine: ValidationEngine::Iterative,
            iterative: IterativeSimConfig {
                slippage_vol_mult: 0.25,
                ..IterativeSimConfig::default()
            },
            min_psr: None,
            min_dsr: None,
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
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base],
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 1024,
                step_bars: 512,
            }),
            perturb: Some(PerturbConfig {
                iterations: 8,
                pct_min: 0.01,
                pct_max: 0.02,
                flip_bool_prob: 0.01,
            }),
            trade_bootstrap: Some(TradeBootstrapConfig { simulations: 64 }),
            cross_ticker: None,
            cscv_pbo: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
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
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![mk(8, 24), mk(12, 36), mk(20, 60)],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 77,
            max_candidates: 16,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
        };

        let baseline =
            run_double_ma_validation_blocking_with_candles(base_req.clone(), &candles, &cancel, None)
                .expect("baseline run");

        let mut gated_req = base_req;
        gated_req.min_psr = Some(0.80);
        gated_req.min_dsr = Some(0.60);
        let gated =
            run_double_ma_validation_blocking_with_candles(gated_req, &candles, &cancel, None)
                .expect("gated run");

        assert!(gated.candidates.len() <= baseline.candidates.len());
        for cand in &gated.candidates {
            assert!(cand.psr.unwrap_or(0.0) >= 0.80);
            assert!(cand.dsr.unwrap_or(0.0) >= 0.60);
        }
    }
}
