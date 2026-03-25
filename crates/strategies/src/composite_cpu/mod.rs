use crate::double_ma::{DoubleMaCurves, Metrics, MinMaxBins, StrategyConfig, Trade, TradeStats};
use serde::{Deserialize, Serialize};

fn clamp_bins(requested: usize, n_steps: usize) -> usize {
    if n_steps == 0 {
        return 0;
    }
    let req = requested.clamp(1, 8192);
    req.min(n_steps)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositeSignalMode {
    #[default]
    HardGate,
    EntryCoreExitOverlay,
    TrendPullbackReversion,
}

#[inline(always)]
pub fn composite_desired_position(
    fast: f64,
    slow: f64,
    halftrend: f64,
    macz_hist: f64,
    ott_price: f64,
    ott_line: f64,
    current_pos: i32,
    cfg: &StrategyConfig,
    signal_mode: CompositeSignalMode,
) -> i32 {
    let eps = if cfg.eps_rel > 0.0 {
        cfg.eps_rel * 1.0_f64.max(slow.abs())
    } else {
        0.0
    };
    let mut core_sign: i32 = (fast > slow + eps) as i32 - (fast < slow - eps) as i32;
    if cfg.long_only && core_sign < 0 {
        core_sign = 0;
    }

    if !halftrend.is_finite() {
        core_sign = 0;
    } else if halftrend < 0.5 {
        core_sign = core_sign.max(0);
    } else {
        core_sign = core_sign.min(0);
    }

    match signal_mode {
        CompositeSignalMode::HardGate => {
            let mut sign = core_sign;
            if !macz_hist.is_finite() {
                sign = 0;
            } else if macz_hist > 0.0 {
                sign = sign.max(0);
            } else if macz_hist < 0.0 {
                sign = sign.min(0);
            } else {
                sign = 0;
            }

            if !ott_price.is_finite() || !ott_line.is_finite() {
                sign = 0;
            } else if ott_price > ott_line {
                sign = sign.max(0);
            } else if ott_price < ott_line {
                sign = sign.min(0);
            } else {
                sign = 0;
            }
            sign
        }
        CompositeSignalMode::EntryCoreExitOverlay => {
            if core_sign == 0 {
                return 0;
            }
            let pos = current_pos.clamp(-1, 1);

            // Entry is driven by the MA + HalfTrend core. MACZ/OTT are used as protective exits.
            if pos > 0 && core_sign > 0 {
                if !macz_hist.is_finite() || !ott_price.is_finite() || !ott_line.is_finite() {
                    0
                } else if macz_hist < 0.0 || ott_price < ott_line {
                    0
                } else {
                    1
                }
            } else if pos < 0 && core_sign < 0 {
                if !macz_hist.is_finite() || !ott_price.is_finite() || !ott_line.is_finite() {
                    0
                } else if macz_hist > 0.0 || ott_price > ott_line {
                    0
                } else {
                    -1
                }
            } else {
                core_sign
            }
        }
        CompositeSignalMode::TrendPullbackReversion => {
            if core_sign == 0 {
                return 0;
            }
            let pos = current_pos.clamp(-1, 1);
            let price_eps = if cfg.eps_rel > 0.0 {
                cfg.eps_rel * 1.0_f64.max(ott_price.abs())
            } else {
                0.0
            };

            // Treat MA trend + HalfTrend regime as the directional bias; only enter on pullbacks.
            // MACZ is used as pullback/reversion timing, and OTT line as a protective regime break.
            let pullback_long = macz_hist.is_finite()
                && macz_hist <= 0.0
                && ott_price.is_finite()
                && fast.is_finite()
                && slow.is_finite()
                && ott_price <= fast + price_eps
                && ott_price >= slow - price_eps
                && ott_line.is_finite()
                && ott_price >= ott_line - price_eps;

            let pullback_short = macz_hist.is_finite()
                && macz_hist >= 0.0
                && ott_price.is_finite()
                && fast.is_finite()
                && slow.is_finite()
                && ott_price >= fast - price_eps
                && ott_price <= slow + price_eps
                && ott_line.is_finite()
                && ott_price <= ott_line + price_eps;

            if pos == 0 {
                if core_sign > 0 {
                    if pullback_long {
                        1
                    } else {
                        0
                    }
                } else if core_sign < 0 {
                    if pullback_short {
                        -1
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else if pos > 0 {
                if core_sign <= 0 {
                    return 0;
                }
                if !ott_price.is_finite()
                    || !ott_line.is_finite()
                    || ott_price < ott_line - price_eps
                {
                    return 0;
                }
                let reverted = (macz_hist.is_finite() && macz_hist >= 0.0)
                    && (ott_price.is_finite() && fast.is_finite() && ott_price >= fast - price_eps);
                if reverted {
                    0
                } else {
                    1
                }
            } else {
                if core_sign >= 0 {
                    return 0;
                }
                if !ott_price.is_finite()
                    || !ott_line.is_finite()
                    || ott_price > ott_line + price_eps
                {
                    return 0;
                }
                let reverted = (macz_hist.is_finite() && macz_hist <= 0.0)
                    && (ott_price.is_finite() && fast.is_finite() && ott_price <= fast + price_eps);
                if reverted {
                    0
                } else {
                    -1
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ExecutionOverlayParams {
    cooldown_bars: usize,
    persistence_bars: usize,
}

impl ExecutionOverlayParams {
    #[inline]
    fn from_cfg(cfg: &StrategyConfig) -> Self {
        Self {
            cooldown_bars: cfg.cooldown_bars(),
            persistence_bars: cfg.signal_persistence_bars().max(1),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ExecutionOverlayState {
    cooldown_left: usize,
    pending_sign: i32,
    pending_count: usize,
}

#[inline]
fn overlay_on_bar_start(state: &mut ExecutionOverlayState) {
    if state.cooldown_left > 0 {
        state.cooldown_left = state.cooldown_left.saturating_sub(1);
    }
}

#[inline]
fn overlay_apply_signal_persistence(
    raw_sign: i32,
    current_pos: i32,
    params: &ExecutionOverlayParams,
    state: &mut ExecutionOverlayState,
) -> i32 {
    if params.persistence_bars <= 1 {
        if raw_sign == 0 {
            state.pending_sign = 0;
            state.pending_count = 0;
        } else {
            state.pending_sign = raw_sign;
            state.pending_count = 1;
        }
        return raw_sign;
    }

    if raw_sign == 0 {
        state.pending_sign = 0;
        state.pending_count = 0;
        return 0;
    }

    if raw_sign == state.pending_sign {
        state.pending_count = state.pending_count.saturating_add(1);
    } else {
        state.pending_sign = raw_sign;
        state.pending_count = 1;
    }

    if state.pending_count >= params.persistence_bars {
        raw_sign
    } else if current_pos != 0 {
        current_pos
    } else {
        0
    }
}

#[inline]
fn overlay_apply_entry_cooldown(sign: i32, current_pos: i32, state: &ExecutionOverlayState) -> i32 {
    if current_pos == 0 && sign != 0 && state.cooldown_left > 0 {
        0
    } else {
        sign
    }
}

#[inline]
fn overlay_on_trade_close(state: &mut ExecutionOverlayState, params: &ExecutionOverlayParams) {
    if params.cooldown_bars > 0 {
        state.cooldown_left = params.cooldown_bars;
    }
}

pub fn eval_composite_metrics_from_log_returns_and_mas_f64(
    lr: &[f64],
    ott_prices: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_line: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
        lr,
        ott_prices,
        fast,
        slow,
        halftrend_trend,
        macz_hist,
        ott_line,
        start_t,
        cfg,
        CompositeSignalMode::HardGate,
    )
}

pub fn eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
    lr: &[f64],
    ott_prices: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_line: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    signal_mode: CompositeSignalMode,
) -> Metrics {
    eval_composite_metrics_from_log_returns_and_mas_f64_impl::<true>(
        lr,
        ott_prices,
        fast,
        slow,
        halftrend_trend,
        macz_hist,
        ott_line,
        start_t,
        cfg,
        signal_mode,
    )
}

pub fn eval_composite_metrics_from_log_returns_and_mas_f64_fast(
    lr: &[f64],
    ott_prices: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_line: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_composite_metrics_from_log_returns_and_mas_f64_fast_with_mode(
        lr,
        ott_prices,
        fast,
        slow,
        halftrend_trend,
        macz_hist,
        ott_line,
        start_t,
        cfg,
        CompositeSignalMode::HardGate,
    )
}

pub fn eval_composite_metrics_from_log_returns_and_mas_f64_fast_with_mode(
    lr: &[f64],
    ott_prices: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_line: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    signal_mode: CompositeSignalMode,
) -> Metrics {
    eval_composite_metrics_from_log_returns_and_mas_f64_impl::<false>(
        lr,
        ott_prices,
        fast,
        slow,
        halftrend_trend,
        macz_hist,
        ott_line,
        start_t,
        cfg,
        signal_mode,
    )
}

fn eval_composite_metrics_from_log_returns_and_mas_f64_impl<const COMPUTE_SHARPE: bool>(
    lr: &[f64],
    ott_prices: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_line: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    signal_mode: CompositeSignalMode,
) -> Metrics {
    let n_total = lr.len();
    if n_total < 2
        || fast.len() != n_total
        || slow.len() != n_total
        || halftrend_trend.len() != n_total
        || macz_hist.len() != n_total
        || ott_line.len() != n_total
        || ott_prices.len() != n_total
    {
        return Metrics {
            pnl: 0.0,
            sharpe: 0.0,
            sortino: 0.0,
            downside_deviation: 0.0,
            calmar: 0.0,
            omega: 0.0,
            kappa_tau: 0.0,
            volatility: 0.0,
            ulcer_index: 0.0,
            martin_ratio: 0.0,
            max_dd: 0.0,
            trades: 0,
            exposure: 0.0,
            net_exposure: 0.0,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            ..Metrics::default()
        };
    }

    if start_t >= n_total {
        return Metrics {
            pnl: 0.0,
            sharpe: 0.0,
            sortino: 0.0,
            downside_deviation: 0.0,
            calmar: 0.0,
            omega: 0.0,
            kappa_tau: 0.0,
            volatility: 0.0,
            ulcer_index: 0.0,
            martin_ratio: 0.0,
            max_dd: 0.0,
            trades: 0,
            exposure: 0.0,
            net_exposure: 0.0,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            ..Metrics::default()
        };
    }

    let commission = cfg.effective_commission();
    if !(0.0..1.0).contains(&commission) && commission != 0.0 {
        return Metrics {
            pnl: 0.0,
            sharpe: 0.0,
            sortino: 0.0,
            downside_deviation: 0.0,
            calmar: 0.0,
            omega: 0.0,
            kappa_tau: 0.0,
            volatility: 0.0,
            ulcer_index: 0.0,
            martin_ratio: 0.0,
            max_dd: 0.0,
            trades: 0,
            exposure: 0.0,
            net_exposure: 0.0,
            downside_frequency: 0.0,
            downside_severity: 0.0,
            long_mean_return: 0.0,
            short_mean_return: 0.0,
            ..Metrics::default()
        };
    }

    let obs: u64 = (n_total - start_t) as u64;

    let log_comm = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let has_commission = commission > 0.0;
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
    let overlays = ExecutionOverlayParams::from_cfg(cfg);
    let risk_enabled = stop_log.is_some() || take_log.is_some();
    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    let mut pos: i32 = 0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;
    let mut max_log_dd = 0.0_f64;
    let mut ulcer_sum_sq = 0.0_f64;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;
    let mut trade_log_ret = 0.0_f64;
    let mut overlay_state = ExecutionOverlayState::default();

    for t in start_t..n_total {
        overlay_on_bar_start(&mut overlay_state);
        let t_sig = t.saturating_sub(trade_offset);
        let raw_sign = composite_desired_position(
            fast[t_sig],
            slow[t_sig],
            halftrend_trend[t_sig],
            macz_hist[t_sig],
            ott_prices[t_sig],
            ott_line[t_sig],
            pos,
            cfg,
            signal_mode,
        );
        let persisted_sign =
            overlay_apply_signal_persistence(raw_sign, pos, &overlays, &mut overlay_state);
        let sign = overlay_apply_entry_cooldown(persisted_sign, pos, &overlay_state);

        if sign != pos {
            if pos == 0 || sign == 0 {
                if has_commission {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = if sign == 0 { 0 } else { sign };
                if pos == 0 {
                    trade_log_ret = 0.0;
                    overlay_on_trade_close(&mut overlay_state, &overlays);
                } else {
                    trade_log_ret = if has_commission { log_comm } else { 0.0 };
                }
            } else if !cfg.allow_flip {
                if has_commission {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                trade_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            } else {
                if has_commission {
                    log_eq += 2.0 * log_comm;
                }
                trades = trades.saturating_add(2);
                pos = sign;
                trade_log_ret = if has_commission { log_comm } else { 0.0 };
            }
        }

        let pos_bar = pos;
        let lr_t = lr[t];
        let step_log_ret = (pos_bar as f64) * lr_t;
        log_eq += step_log_ret;

        if risk_enabled && pos_bar != 0 {
            trade_log_ret += step_log_ret;
            let hit_stop = stop_log.is_some_and(|th| trade_log_ret <= th);
            let hit_take = take_log.is_some_and(|th| trade_log_ret >= th);
            if hit_stop || hit_take {
                if has_commission {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                trade_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }
        let dd = if dd_log > 0.0 {
            -(-dd_log).exp_m1()
        } else {
            0.0
        };
        ulcer_sum_sq += dd * dd;

        if COMPUTE_SHARPE {
            let step_r = if pos_bar == 0 {
                0.0
            } else {
                (pos_bar as f64) * lr_t.exp_m1()
            };
            count += 1;
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
        }

        sum_abs_pos += (pos_bar as i64).abs();
        sum_pos += pos_bar as i64;
    }

    const EPS_DENOM: f64 = 1e-12;
    const CLAMP_MAX: f64 = 1e6;

    let (sharpe, volatility, sortino, downside_deviation, omega) = if COMPUTE_SHARPE {
        let variance = if count > 1 {
            m2 / (count as f64 - 1.0)
        } else {
            0.0
        };
        let std = variance.sqrt();
        let sharpe = if std > 0.0 { mean / std } else { 0.0 };
        let volatility = std;

        let denom = (count as f64).max(1.0);
        let downside_deviation = (sum_neg_sq / denom).sqrt();
        let sortino_raw = if downside_deviation > 0.0 {
            mean / downside_deviation
        } else if mean > 0.0 {
            CLAMP_MAX
        } else if mean < 0.0 {
            -CLAMP_MAX
        } else {
            0.0
        };
        let sortino = sortino_raw.clamp(-CLAMP_MAX, CLAMP_MAX);

        let omega_raw = (sum_pos_r + EPS_DENOM) / (sum_neg_abs_r + EPS_DENOM);
        let omega = omega_raw.clamp(0.0, CLAMP_MAX);

        (sharpe, volatility, sortino, downside_deviation, omega)
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    };
    let denom = if COMPUTE_SHARPE { count } else { obs };
    let exposure = if denom > 0 {
        sum_abs_pos as f64 / denom as f64
    } else {
        0.0
    };
    let net_exposure = if denom > 0 {
        sum_pos as f64 / denom as f64
    } else {
        0.0
    };
    let max_dd = if max_log_dd > 0.0 {
        -(-max_log_dd).exp_m1()
    } else {
        0.0
    };
    let ulcer_index = (ulcer_sum_sq / (obs as f64).max(1.0)).sqrt();

    let pnl = log_eq.exp() - 1.0;
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
    let martin_raw = if ulcer_index > EPS_DENOM {
        pnl / ulcer_index
    } else if pnl > 0.0 {
        CLAMP_MAX
    } else if pnl < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    };
    let martin_ratio = martin_raw.clamp(-CLAMP_MAX, CLAMP_MAX);

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        downside_frequency: 0.0,
        downside_severity: 0.0,
        calmar,
        omega,
        kappa_tau: 0.0,
        volatility,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
        exposure,
        net_exposure,
        long_mean_return: 0.0,
        short_mean_return: 0.0,
        ..Metrics::default()
    }
}

pub fn eval_composite_curves_from_log_returns_and_mas_f64(
    lr: &[f64],
    ott_prices: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_line: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    bins: usize,
) -> DoubleMaCurves {
    eval_composite_curves_from_log_returns_and_mas_f64_with_mode(
        lr,
        ott_prices,
        fast,
        slow,
        halftrend_trend,
        macz_hist,
        ott_line,
        start_t,
        cfg,
        bins,
        CompositeSignalMode::HardGate,
    )
}

pub fn eval_composite_curves_from_log_returns_and_mas_f64_with_mode(
    lr: &[f64],
    ott_prices: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    macz_hist: &[f64],
    ott_line: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    bins: usize,
    signal_mode: CompositeSignalMode,
) -> DoubleMaCurves {
    let n_total = lr.len();
    if n_total < 2
        || fast.len() != n_total
        || slow.len() != n_total
        || halftrend_trend.len() != n_total
        || macz_hist.len() != n_total
        || ott_line.len() != n_total
        || ott_prices.len() != n_total
        || start_t >= n_total
    {
        return DoubleMaCurves {
            equity: MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: Vec::new(),
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
    }

    let commission = cfg.effective_commission();
    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let log_comm = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let has_commission = commission > 0.0;
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
    let overlays = ExecutionOverlayParams::from_cfg(cfg);
    let risk_enabled = stop_log.is_some() || take_log.is_some();

    let n_steps = n_total.saturating_sub(start_t);
    let bins = clamp_bins(bins, n_steps);
    if bins == 0 {
        return DoubleMaCurves {
            equity: MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: MinMaxBins {
                start_t,
                end_t: n_total,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: Vec::new(),
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
    }

    let mut eq_min: Vec<f64> = vec![f64::INFINITY; bins];
    let mut eq_max: Vec<f64> = vec![f64::NEG_INFINITY; bins];
    let mut dd_min: Vec<f64> = vec![f64::INFINITY; bins];
    let mut dd_max: Vec<f64> = vec![f64::NEG_INFINITY; bins];

    let mut pos: i32 = 0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;

    let mut trades: Vec<Trade> = Vec::new();
    let mut cur_entry_t: usize = 0;
    let mut cur_dir: i32 = 0;
    let mut cur_log_ret: f64 = 0.0;
    let mut overlay_state = ExecutionOverlayState::default();

    for t in start_t..n_total {
        overlay_on_bar_start(&mut overlay_state);
        let t_sig = t.saturating_sub(trade_offset);
        let raw_sign = composite_desired_position(
            fast[t_sig],
            slow[t_sig],
            halftrend_trend[t_sig],
            macz_hist[t_sig],
            ott_prices[t_sig],
            ott_line[t_sig],
            pos,
            cfg,
            signal_mode,
        );
        let persisted_sign =
            overlay_apply_signal_persistence(raw_sign, pos, &overlays, &mut overlay_state);
        let sign = overlay_apply_entry_cooldown(persisted_sign, pos, &overlay_state);

        if sign != pos {
            if pos == 0 || sign == 0 {
                if has_commission {
                    log_eq += log_comm;
                }
                if pos != 0 && cur_dir != 0 {
                    if has_commission {
                        cur_log_ret += log_comm;
                    }
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: cur_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                    cur_dir = 0;
                    cur_log_ret = 0.0;
                    overlay_on_trade_close(&mut overlay_state, &overlays);
                } else if pos == 0 && sign != 0 {
                    cur_entry_t = t;
                    cur_dir = sign;
                    cur_log_ret = if has_commission { log_comm } else { 0.0 };
                }
                pos = if sign == 0 { 0 } else { sign };
            } else if !cfg.allow_flip {
                if has_commission {
                    log_eq += log_comm;
                }
                if cur_dir != 0 {
                    if has_commission {
                        cur_log_ret += log_comm;
                    }
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: cur_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                    cur_dir = 0;
                    cur_log_ret = 0.0;
                    overlay_on_trade_close(&mut overlay_state, &overlays);
                }
                pos = 0;
            } else {
                if has_commission {
                    log_eq += 2.0 * log_comm;
                }
                if cur_dir != 0 {
                    if has_commission {
                        cur_log_ret += log_comm;
                    }
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: cur_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                }
                cur_entry_t = t;
                cur_dir = sign;
                cur_log_ret = if has_commission { log_comm } else { 0.0 };
                pos = sign;
            }
        }

        log_eq += (pos as f64) * lr[t];
        if cur_dir != 0 {
            cur_log_ret += (pos as f64) * lr[t];
        }
        if risk_enabled && cur_dir != 0 {
            let hit_stop = stop_log.is_some_and(|th| cur_log_ret <= th);
            let hit_take = take_log.is_some_and(|th| cur_log_ret >= th);
            if hit_stop || hit_take {
                if has_commission {
                    log_eq += log_comm;
                    cur_log_ret += log_comm;
                }
                trades.push(Trade {
                    entry_t: cur_entry_t,
                    exit_t: t,
                    direction: cur_dir,
                    pnl: cur_log_ret.exp() - 1.0,
                    bars: t.saturating_sub(cur_entry_t) as u32,
                    open: false,
                });
                cur_dir = 0;
                cur_log_ret = 0.0;
                pos = 0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }
        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;

        let eq = log_eq.exp();
        let dd = if dd_log > 0.0 {
            -(-dd_log).exp_m1()
        } else {
            0.0
        };

        let i = t - start_t;
        let b = (i.saturating_mul(bins)) / n_steps.max(1);
        let b = b.min(bins - 1);

        if eq < eq_min[b] {
            eq_min[b] = eq;
        }
        if eq > eq_max[b] {
            eq_max[b] = eq;
        }
        if dd < dd_min[b] {
            dd_min[b] = dd;
        }
        if dd > dd_max[b] {
            dd_max[b] = dd;
        }
    }

    let mut eq_mm: Vec<f32> = Vec::with_capacity(bins.saturating_mul(2));
    let mut dd_mm: Vec<f32> = Vec::with_capacity(bins.saturating_mul(2));
    for i in 0..bins {
        let (a, b) = (eq_min[i], eq_max[i]);
        if a.is_finite() && b.is_finite() {
            eq_mm.push(a as f32);
            eq_mm.push(b as f32);
        } else {
            eq_mm.push(f32::NAN);
            eq_mm.push(f32::NAN);
        }

        let (a, b) = (dd_min[i], dd_max[i]);
        if a.is_finite() && b.is_finite() {
            dd_mm.push(a as f32);
            dd_mm.push(b as f32);
        } else {
            dd_mm.push(f32::NAN);
            dd_mm.push(f32::NAN);
        }
    }

    if cur_dir != 0 {
        let exit_t = n_total.saturating_sub(1);
        trades.push(Trade {
            entry_t: cur_entry_t,
            exit_t,
            direction: cur_dir,
            pnl: cur_log_ret.exp() - 1.0,
            bars: exit_t.saturating_sub(cur_entry_t) as u32,
            open: true,
        });
    }

    let trade_stats: Option<TradeStats> = {
        let mut open_trades: usize = 0;
        let mut closed_trades: usize = 0;
        let mut wins: usize = 0;
        let mut losses: usize = 0;
        let mut sum_win: f64 = 0.0;
        let mut sum_loss_abs: f64 = 0.0;
        let mut sum_win_count: usize = 0;
        let mut sum_loss_count: usize = 0;
        let mut sum_all: f64 = 0.0;

        for tr in trades.iter() {
            if tr.open {
                open_trades += 1;
                continue;
            }
            closed_trades += 1;
            let r = tr.pnl;
            sum_all += r;
            if r > 0.0 {
                wins += 1;
                sum_win += r;
                sum_win_count += 1;
            } else if r < 0.0 {
                losses += 1;
                sum_loss_abs += -r;
                sum_loss_count += 1;
            }
        }

        if closed_trades == 0 && open_trades == 0 {
            None
        } else {
            let denom = wins + losses;
            let win_rate = if denom > 0 {
                wins as f64 / denom as f64
            } else {
                0.0
            };
            let avg_win = if sum_win_count > 0 {
                sum_win / sum_win_count as f64
            } else {
                0.0
            };
            let avg_loss = if sum_loss_count > 0 {
                -(sum_loss_abs / sum_loss_count as f64)
            } else {
                0.0
            };
            let profit_factor = if sum_loss_abs > 0.0 {
                sum_win / sum_loss_abs
            } else if sum_win > 0.0 {
                f64::INFINITY
            } else {
                0.0
            };
            let expectancy = if closed_trades > 0 {
                sum_all / closed_trades as f64
            } else {
                0.0
            };

            Some(TradeStats {
                closed_trades,
                open_trades,
                wins,
                losses,
                win_rate,
                avg_win,
                avg_loss,
                profit_factor,
                expectancy,
            })
        }
    };

    DoubleMaCurves {
        equity: MinMaxBins {
            start_t,
            end_t: n_total,
            bins,
            minmax: eq_mm,
        },
        drawdown: MinMaxBins {
            start_t,
            end_t: n_total,
            bins,
            minmax: dd_mm,
        },
        trades,
        trade_stats,
        indicator_bars: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compute_log_returns(prices: &[f64]) -> Vec<f64> {
        let mut lr = Vec::with_capacity(prices.len());
        if prices.is_empty() {
            return lr;
        }
        lr.push(0.0);
        for w in prices.windows(2) {
            lr.push((w[1] / w[0]).ln());
        }
        lr
    }

    #[test]
    fn entry_core_exit_overlay_enters_on_core_when_flat() {
        let cfg = StrategyConfig::default();
        let sign = composite_desired_position(
            11.0,
            10.0,
            0.0,  // HalfTrend bullish branch
            -1.0, // MACZ bearish (should not block fresh entry)
            99.0, // OTT bearish (should not block fresh entry)
            100.0,
            0,
            &cfg,
            CompositeSignalMode::EntryCoreExitOverlay,
        );
        assert_eq!(sign, 1);
    }

    #[test]
    fn entry_core_exit_overlay_exits_long_on_overlay_bearish() {
        let cfg = StrategyConfig::default();
        let sign = composite_desired_position(
            11.0,
            10.0,
            0.0, // HalfTrend bullish
            -1.0,
            99.0,
            100.0,
            1, // currently long
            &cfg,
            CompositeSignalMode::EntryCoreExitOverlay,
        );
        assert_eq!(sign, 0);
    }

    #[test]
    fn hard_gate_mode_keeps_existing_behavior() {
        let cfg = StrategyConfig::default();
        let sign = composite_desired_position(
            11.0,
            10.0,
            0.0,  // HalfTrend bullish
            -1.0, // MACZ bearish veto
            99.0, // OTT bearish veto
            100.0,
            0,
            &cfg,
            CompositeSignalMode::HardGate,
        );
        assert_eq!(sign, 0);
    }

    #[test]
    fn metrics_wrapper_matches_hard_gate_mode() {
        let lr = vec![0.0, 0.01, -0.01, 0.02, -0.005, 0.01];
        let fast = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let slow = vec![2.0, 2.0, 2.0, 3.0, 4.0, 5.0];
        let ht = vec![0.0; lr.len()];
        let macz = vec![1.0; lr.len()];
        let ott_line = vec![1.0; lr.len()];
        let ott_prices = vec![2.0; lr.len()];
        let cfg = StrategyConfig {
            trade_on_next_bar: false,
            ..StrategyConfig::default()
        };

        let a = eval_composite_metrics_from_log_returns_and_mas_f64(
            &lr,
            &ott_prices,
            &fast,
            &slow,
            &ht,
            &macz,
            &ott_line,
            1,
            &cfg,
        );
        let b = eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
            &lr,
            &ott_prices,
            &fast,
            &slow,
            &ht,
            &macz,
            &ott_line,
            1,
            &cfg,
            CompositeSignalMode::HardGate,
        );

        assert!((a.pnl - b.pnl).abs() < 1e-12);
        assert!((a.sharpe - b.sharpe).abs() < 1e-12);
        assert!((a.sortino - b.sortino).abs() < 1e-12);
        assert!((a.calmar - b.calmar).abs() < 1e-12);
        assert!((a.omega - b.omega).abs() < 1e-12);
        assert!((a.volatility - b.volatility).abs() < 1e-12);
        assert!((a.max_dd - b.max_dd).abs() < 1e-12);
        assert_eq!(a.trades, b.trades);
        assert!((a.exposure - b.exposure).abs() < 1e-12);
        assert!((a.net_exposure - b.net_exposure).abs() < 1e-12);
    }

    #[test]
    fn trend_pullback_reversion_enters_long_on_bull_pullback() {
        let cfg = StrategyConfig::default();
        let sign = composite_desired_position(
            110.0,
            100.0,
            0.0, // HalfTrend bullish
            -0.4,
            105.0, // below fast, above slow (pullback)
            103.0, // above OTT line
            0,
            &cfg,
            CompositeSignalMode::TrendPullbackReversion,
        );
        assert_eq!(sign, 1);
    }

    #[test]
    fn trend_pullback_reversion_stays_flat_without_pullback() {
        let cfg = StrategyConfig::default();
        let sign = composite_desired_position(
            110.0,
            100.0,
            0.0,   // HalfTrend bullish
            0.6,   // not a pullback
            111.0, // above fast
            103.0,
            0,
            &cfg,
            CompositeSignalMode::TrendPullbackReversion,
        );
        assert_eq!(sign, 0);
    }

    #[test]
    fn trend_pullback_reversion_exits_long_on_reversion() {
        let cfg = StrategyConfig::default();
        let sign = composite_desired_position(
            110.0,
            100.0,
            0.0,   // HalfTrend bullish
            0.3,   // reverted momentum
            111.0, // above fast => reversion complete
            104.0,
            1,
            &cfg,
            CompositeSignalMode::TrendPullbackReversion,
        );
        assert_eq!(sign, 0);
    }

    #[test]
    fn tp_sl_increases_trade_count_when_signals_stay_long() {
        let n = 256usize;
        let prices: Vec<f64> = (0..n)
            .map(|i| {
                if i < 128 {
                    100.0 + i as f64 * 0.20
                } else {
                    125.6 - (i - 128) as f64 * 0.30
                }
            })
            .collect();
        let lr = compute_log_returns(&prices);
        let fast: Vec<f64> = vec![2.0; n];
        let slow: Vec<f64> = vec![1.0; n];
        let ht: Vec<f64> = vec![0.0; n];
        let macz: Vec<f64> = vec![1.0; n];
        let ott_line: Vec<f64> = vec![1.0; n];
        let ott_prices: Vec<f64> = vec![2.0; n];

        let base = StrategyConfig {
            commission: 0.0,
            fee_bps: 1.0,
            spread_bps: 0.0,
            slippage_bps: 0.0,
            ..StrategyConfig::default()
        };
        let risk = StrategyConfig {
            stop_loss_pct: Some(0.02),
            take_profit_pct: Some(0.02),
            ..base.clone()
        };

        let m0 = eval_composite_metrics_from_log_returns_and_mas_f64(
            &lr,
            &ott_prices,
            &fast,
            &slow,
            &ht,
            &macz,
            &ott_line,
            1,
            &base,
        );
        let m1 = eval_composite_metrics_from_log_returns_and_mas_f64(
            &lr,
            &ott_prices,
            &fast,
            &slow,
            &ht,
            &macz,
            &ott_line,
            1,
            &risk,
        );

        assert!(m1.trades > m0.trades);
    }

    #[test]
    fn curves_emit_closed_trades_when_tp_sl_hits() {
        let n = 128usize;
        let prices: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.30).collect();
        let lr = compute_log_returns(&prices);
        let fast: Vec<f64> = vec![2.0; n];
        let slow: Vec<f64> = vec![1.0; n];
        let ht: Vec<f64> = vec![0.0; n];
        let macz: Vec<f64> = vec![1.0; n];
        let ott_line: Vec<f64> = vec![1.0; n];
        let ott_prices: Vec<f64> = vec![2.0; n];

        let cfg = StrategyConfig {
            commission: 0.0,
            fee_bps: 1.0,
            spread_bps: 0.0,
            slippage_bps: 0.0,
            stop_loss_pct: None,
            take_profit_pct: Some(0.015),
            ..StrategyConfig::default()
        };

        let curves = eval_composite_curves_from_log_returns_and_mas_f64(
            &lr,
            &ott_prices,
            &fast,
            &slow,
            &ht,
            &macz,
            &ott_line,
            1,
            &cfg,
            64,
        );
        let closed = curves.trades.iter().filter(|t| !t.open).count();
        assert!(closed > 0);
    }
}

