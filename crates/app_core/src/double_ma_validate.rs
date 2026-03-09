use crate::composite_validate::{
    CrossTickerConfig, CrossTickerStats, CrossTickerValidationSummary, PerturbConfig, PerturbStats,
    TradeBootstrapConfig, TradeBootstrapStats, ValidationArtifactContext,
    WalkForwardConfigOptional, WalkForwardStats,
};
use crate::cscv_pbo::{CscvCandidatePboStats, CscvPboConfig, CscvPboResult, SliceReturnStats};
use crate::double_ma::{
    compute_combined_regime_filter, compute_log_returns_f64, first_finite_idx, ma_matrix_cpu_f64,
    resolve_alma_params_for_side, validate_ma_kind, validate_strategy_cfg, HalfTrendRegimeFilter,
};
use crate::iterative_sim::{IterativeMetricsDelta, IterativeSimConfig, ValidationEngine};
use crate::progress::ProgressSink;
use crate::validation_pipeline::ValidationPostCandidate;

use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
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

    fn set_cscv_pbo(&mut self, v: Option<CscvCandidatePboStats>) {
        self.cscv_pbo = v;
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
    let min_psr = crate::validation_stats::sanitize_probability_gate(req.min_psr);
    let min_dsr = crate::validation_stats::sanitize_probability_gate(req.min_dsr);

    let (out, cscv_pbo) = crate::validation_core::run_validation_candidates(
        &req.candidates,
        seed,
        cancel,
        progress,
        "double_ma_validate",
        "double_ma_validate_cscv_prep",
        min_psr,
        min_dsr,
        cscv_cfg.as_ref(),
        candles.close.len(),
        |_, p, rng| {
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
                let sims = cfg.simulations.min(50_000);
                let trades = trades_for_series(&series);
                Some(crate::validation_pipeline::compute_trade_bootstrap_stats(
                    &trades, sims, rng,
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
            let robust_score = crate::validation_pipeline::robust_score_from_metrics(
                robust_base,
                walk_forward
                    .as_ref()
                    .map(|w| (w.sharpe_median, w.max_dd_worst)),
            );

            Ok((
                DoubleMaValidationCandidateResult {
                    params: p.clone(),
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
        },
        |cand, cscv_start_t, slice_bars, slices| {
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
            validation_context: None,
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
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates: candidates.clone(),
                walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 17,
                max_candidates: 32,
                validation_engine,
                iterative: IterativeSimConfig {
                    slippage_vol_mult: 0.0,
                    ..IterativeSimConfig::default()
                },
                min_psr: None,
                min_dsr: None,
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

        assert!(gated.candidates.len() <= baseline.candidates.len());
        for cand in &gated.candidates {
            assert!(cand.psr.unwrap_or(0.0) >= 0.80);
            assert!(cand.dsr.unwrap_or(0.0) >= 0.60);
        }
    }

    #[test]
    fn double_ma_validation_rejects_invalid_candidate_params() {
        let candles = make_trending_candles(2048);
        let cancel = AtomicBool::new(false);

        let req = DoubleMaValidationRequest {
            data_id: "invalid".to_string(),
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
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 1,
            validation_engine: ValidationEngine::Vectorized,
            iterative: IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
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
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                candidates,
                walk_forward: None,
                perturb: None,
                trade_bootstrap: None,
                cross_ticker: None,
                cscv_pbo: None,
                seed: 7,
                max_candidates: 16,
                validation_engine: ValidationEngine::Vectorized,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
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
                "kind": "vectorbt_run_artifact",
                "ts": "ct_0001",
                "tags": ["ct-e2e"],
                "run": { "req": { "strategy_kind": "CompositeTrend", "ma_source": "close" }, "res": {} },
                "dataset_meta": { "label": "asset_a", "timeframe": "1m", "csv_path": "asset_a.csv" },
                "validation": { "req": { "ma_source": "close" }, "res": val_a }
            }),
            json!({
                "kind": "vectorbt_run_artifact",
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
