use serde::{Deserialize, Serialize};

use ta_strategies::double_ma::{Metrics, StrategyConfig};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationEngine {
    Vectorized,
    Iterative,
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::Vectorized
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntrabarRule {
    Pessimistic,
    Optimistic,
    StopFirst,
    TakeFirst,
}

impl Default for IntrabarRule {
    fn default() -> Self {
        Self::Pessimistic
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IterativeSimConfig {
    #[serde(default)]
    pub stop_loss_pct: Option<f64>,
    #[serde(default)]
    pub take_profit_pct: Option<f64>,
    #[serde(default)]
    pub intrabar_rule: IntrabarRule,
    #[serde(default)]
    pub slippage_vol_mult: f64,
}

impl Default for IterativeSimConfig {
    fn default() -> Self {
        Self {
            stop_loss_pct: None,
            take_profit_pct: None,
            intrabar_rule: IntrabarRule::Pessimistic,
            slippage_vol_mult: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct IterativeMetricsDelta {
    pub pnl: f64,
    pub sharpe: f64,
    pub max_dd: f64,
    pub trades: i64,
    pub exposure: f64,
    pub net_exposure: f64,
}

impl IterativeMetricsDelta {
    pub fn from_pair(vectorized: &Metrics, iterative: &Metrics) -> Self {
        Self {
            pnl: iterative.pnl - vectorized.pnl,
            sharpe: iterative.sharpe - vectorized.sharpe,
            max_dd: iterative.max_dd - vectorized.max_dd,
            trades: iterative.trades as i64 - vectorized.trades as i64,
            exposure: iterative.exposure - vectorized.exposure,
            net_exposure: iterative.net_exposure - vectorized.net_exposure,
        }
    }
}

pub struct OhlcSlice<'a> {
    pub open: &'a [f64],
    pub high: &'a [f64],
    pub low: &'a [f64],
    pub close: &'a [f64],
}

fn clamp_pct(v: Option<f64>) -> Option<f64> {
    let x = v?;
    if !x.is_finite() || x <= 0.0 {
        None
    } else {
        Some(x.min(0.95))
    }
}

fn trade_log_cost(cfg: &StrategyConfig, sim: &IterativeSimConfig, lr_abs: f64) -> f64 {
    let base = cfg.effective_commission();
    let extra = (sim.slippage_vol_mult.max(0.0) * lr_abs.max(0.0)).clamp(0.0, 0.95);
    let c = (base + extra).clamp(0.0, 0.95);
    if c > 0.0 {
        (1.0 - c).ln()
    } else {
        0.0
    }
}

fn fill_price_from_intrabar(
    pos: i32,
    entry_px: f64,
    high: f64,
    low: f64,
    stop_loss_pct: Option<f64>,
    take_profit_pct: Option<f64>,
    rule: IntrabarRule,
) -> Option<f64> {
    if pos == 0 || !entry_px.is_finite() || entry_px <= 0.0 {
        return None;
    }
    if !high.is_finite() || !low.is_finite() {
        return None;
    }

    let stop = clamp_pct(stop_loss_pct);
    let take = clamp_pct(take_profit_pct);

    let (stop_px, take_px, stop_hit, take_hit) = if pos > 0 {
        let stop_px = stop.map(|p| entry_px * (1.0 - p));
        let take_px = take.map(|p| entry_px * (1.0 + p));
        let stop_hit = stop_px.map(|x| low <= x).unwrap_or(false);
        let take_hit = take_px.map(|x| high >= x).unwrap_or(false);
        (stop_px, take_px, stop_hit, take_hit)
    } else {
        let stop_px = stop.map(|p| entry_px * (1.0 + p));
        let take_px = take.map(|p| entry_px * (1.0 - p));
        let stop_hit = stop_px.map(|x| high >= x).unwrap_or(false);
        let take_hit = take_px.map(|x| low <= x).unwrap_or(false);
        (stop_px, take_px, stop_hit, take_hit)
    };

    if !stop_hit && !take_hit {
        return None;
    }

    if stop_hit && !take_hit {
        return stop_px;
    }
    if take_hit && !stop_hit {
        return take_px;
    }

    match rule {
        IntrabarRule::StopFirst => stop_px,
        IntrabarRule::TakeFirst => take_px,
        IntrabarRule::Pessimistic => {
            if pos > 0 {
                Some(stop_px.unwrap_or(entry_px).min(take_px.unwrap_or(entry_px)))
            } else {
                Some(stop_px.unwrap_or(entry_px).max(take_px.unwrap_or(entry_px)))
            }
        }
        IntrabarRule::Optimistic => {
            if pos > 0 {
                Some(stop_px.unwrap_or(entry_px).max(take_px.unwrap_or(entry_px)))
            } else {
                Some(stop_px.unwrap_or(entry_px).min(take_px.unwrap_or(entry_px)))
            }
        }
    }
}

pub fn simulate_iterative_from_signals(
    lr: &[f64],
    ohlc: OhlcSlice<'_>,
    desired_pos: &[i32],
    start_t: usize,
    cfg: &StrategyConfig,
    sim: &IterativeSimConfig,
) -> Metrics {
    let n_total = lr.len();
    if n_total < 2
        || desired_pos.len() != n_total
        || ohlc.open.len() != n_total
        || ohlc.high.len() != n_total
        || ohlc.low.len() != n_total
        || ohlc.close.len() != n_total
        || start_t >= n_total
    {
        return Metrics::default();
    }

    let mut pos: i32 = 0;
    let mut entry_px: f64 = 0.0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;
    let mut max_log_dd = 0.0_f64;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;

    for t in start_t..n_total {
        let mut desired = desired_pos[t].clamp(-1, 1);
        if cfg.long_only && desired < 0 {
            desired = 0;
        }

        let lr_abs = lr[t].abs();
        let log_cost = trade_log_cost(cfg, sim, lr_abs);

        if desired != pos {
            if pos == 0 || desired == 0 {
                if log_cost != 0.0 {
                    log_eq += log_cost;
                }
                trades = trades.saturating_add(1);
                if desired != 0 {
                    entry_px = if t > 0 {
                        ohlc.close[t - 1]
                    } else {
                        ohlc.open[t]
                    };
                } else {
                    entry_px = 0.0;
                }
                pos = desired;
            } else if !cfg.allow_flip {
                if log_cost != 0.0 {
                    log_eq += log_cost;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                entry_px = 0.0;
            } else {
                if log_cost != 0.0 {
                    log_eq += 2.0 * log_cost;
                }
                trades = trades.saturating_add(2);
                pos = desired;
                entry_px = if t > 0 {
                    ohlc.close[t - 1]
                } else {
                    ohlc.open[t]
                };
            }
        }

        let mut step_log = if pos == 0 { 0.0 } else { (pos as f64) * lr[t] };

        if pos != 0 {
            if !entry_px.is_finite() || entry_px <= 0.0 {
                entry_px = if t > 0 {
                    ohlc.close[t - 1]
                } else {
                    ohlc.open[t]
                };
            }
            if let Some(fill_px) = fill_price_from_intrabar(
                pos,
                entry_px,
                ohlc.high[t],
                ohlc.low[t],
                sim.stop_loss_pct,
                sim.take_profit_pct,
                sim.intrabar_rule,
            ) {
                let prev_px = if t > 0 {
                    ohlc.close[t - 1]
                } else {
                    ohlc.open[t]
                };
                if prev_px.is_finite() && prev_px > 0.0 && fill_px.is_finite() && fill_px > 0.0 {
                    let bar_log = (fill_px / prev_px).ln();
                    step_log = if pos > 0 { bar_log } else { -bar_log };
                }
                if log_cost != 0.0 {
                    log_eq += log_cost;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                entry_px = 0.0;
            }
        }

        log_eq += step_log;
        let step_r = step_log.exp_m1();
        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }

        count = count.saturating_add(1);
        let delta = step_r - mean;
        mean += delta / count as f64;
        let delta2 = step_r - mean;
        m2 += delta * delta2;

        if step_r < 0.0 {
            sum_neg_sq += step_r * step_r;
            sum_neg_abs_r += -step_r;
        } else {
            sum_pos_r += step_r;
        }

        sum_abs_pos += (pos as i64).abs();
        sum_pos += pos as i64;
    }

    const EPS_DENOM: f64 = 1e-12;
    const CLAMP_MAX: f64 = 1e6;

    let variance = if count > 1 {
        m2 / (count as f64 - 1.0)
    } else {
        0.0
    };
    let std = variance.sqrt();
    let sharpe = if std > 0.0 { mean / std } else { 0.0 };
    let volatility = std;

    let denom = (count as f64).max(1.0);
    let downside_dev = (sum_neg_sq / denom).sqrt();
    let sortino_raw = if downside_dev > 0.0 {
        mean / downside_dev
    } else if mean > 0.0 {
        CLAMP_MAX
    } else if mean < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    };
    let sortino = sortino_raw.clamp(-CLAMP_MAX, CLAMP_MAX);
    let omega = if sum_neg_abs_r > EPS_DENOM {
        (sum_pos_r / sum_neg_abs_r).clamp(0.0, CLAMP_MAX)
    } else if sum_pos_r > 0.0 {
        CLAMP_MAX
    } else {
        0.0
    };

    let pnl = log_eq.exp() - 1.0;
    let max_dd = 1.0 - (-max_log_dd).exp();
    let calmar_raw = if max_dd > EPS_DENOM {
        pnl / max_dd
    } else if pnl > 0.0 {
        CLAMP_MAX
    } else if pnl < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    };
    let calmar = calmar_raw.clamp(-CLAMP_MAX, CLAMP_MAX);

    let obs = (n_total - start_t) as f64;
    let exposure = if obs > 0.0 {
        (sum_abs_pos as f64) / obs
    } else {
        0.0
    };
    let net_exposure = if obs > 0.0 {
        (sum_pos as f64) / obs
    } else {
        0.0
    };

    Metrics {
        pnl,
        sharpe,
        sortino,
        calmar,
        omega,
        volatility,
        max_dd,
        trades,
        exposure,
        net_exposure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lr_uptrend(n: usize, step: f64) -> Vec<f64> {
        (0..n).map(|_| step).collect()
    }

    fn make_flat_ohlc(n: usize) -> OhlcSlice<'static> {
        let mut open: Vec<f64> = Vec::with_capacity(n);
        let mut high: Vec<f64> = Vec::with_capacity(n);
        let mut low: Vec<f64> = Vec::with_capacity(n);
        let mut close: Vec<f64> = Vec::with_capacity(n);
        let mut px = 100.0;
        for _ in 0..n {
            open.push(px);
            px *= 1.001;
            high.push(px * 1.001);
            low.push(px * 0.999);
            close.push(px);
        }
        let open = Box::leak(open.into_boxed_slice());
        let high = Box::leak(high.into_boxed_slice());
        let low = Box::leak(low.into_boxed_slice());
        let close = Box::leak(close.into_boxed_slice());
        OhlcSlice {
            open,
            high,
            low,
            close,
        }
    }

    #[test]
    fn iterative_slippage_penalizes_pnl() {
        let n = 512usize;
        let lr = make_lr_uptrend(n, 0.0005);
        let mut sig = vec![0i32; n];
        for x in sig.iter_mut().skip(2) {
            *x = 1;
        }

        let cfg = StrategyConfig::default();
        let base = IterativeSimConfig::default();
        let mut slip = IterativeSimConfig::default();
        slip.slippage_vol_mult = 0.5;

        let m0 = simulate_iterative_from_signals(&lr, make_flat_ohlc(n), &sig, 2, &cfg, &base);
        let m1 = simulate_iterative_from_signals(&lr, make_flat_ohlc(n), &sig, 2, &cfg, &slip);
        assert!(m1.pnl <= m0.pnl + 1e-12);
    }
}
