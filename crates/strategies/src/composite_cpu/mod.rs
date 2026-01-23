use crate::double_ma::{DoubleMaCurves, Metrics, MinMaxBins, StrategyConfig, Trade, TradeStats};

fn clamp_bins(requested: usize, n_steps: usize) -> usize {
    if n_steps == 0 {
        return 0;
    }
    let req = requested.clamp(1, 8192);
    req.min(n_steps)
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
            max_dd: 0.0,
            trades: 0,
            exposure: 0.0,
            net_exposure: 0.0,
        };
    }

    if start_t >= n_total {
        return Metrics {
            pnl: 0.0,
            sharpe: 0.0,
            max_dd: 0.0,
            trades: 0,
            exposure: 0.0,
            net_exposure: 0.0,
        };
    }

    if !(0.0..1.0).contains(&cfg.commission) && cfg.commission != 0.0 {
        return Metrics {
            pnl: 0.0,
            sharpe: 0.0,
            max_dd: 0.0,
            trades: 0,
            exposure: 0.0,
            net_exposure: 0.0,
        };
    }

    let obs: u64 = (n_total - start_t) as u64;

    let log_comm = if cfg.commission > 0.0 {
        (1.0 - cfg.commission).ln()
    } else {
        0.0
    };
    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    let mut pos: i32 = 0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;
    let mut max_log_dd = 0.0_f64;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;

    for t in start_t..n_total {
        let t_sig = t.saturating_sub(trade_offset);
        let f = fast[t_sig];
        let s = slow[t_sig];
        let eps = if cfg.eps_rel > 0.0 {
            cfg.eps_rel * 1.0_f64.max(s.abs())
        } else {
            0.0
        };

        let mut sign: i32 = (f > s + eps) as i32 - (f < s - eps) as i32;
        if cfg.long_only && sign < 0 {
            sign = 0;
        }

        let ht = halftrend_trend[t_sig];
        if !ht.is_finite() {
            sign = 0;
        } else if ht < 0.5 {
            sign = sign.max(0);
        } else {
            sign = sign.min(0);
        }

        let mh = macz_hist[t_sig];
        if !mh.is_finite() {
            sign = 0;
        } else if mh > 0.0 {
            sign = sign.max(0);
        } else if mh < 0.0 {
            sign = sign.min(0);
        } else {
            sign = 0;
        }

        let px = ott_prices[t_sig];
        let ott = ott_line[t_sig];
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
                if cfg.commission > 0.0 {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = if sign == 0 { 0 } else { sign };
            } else if !cfg.allow_flip {
                if cfg.commission > 0.0 {
                    log_eq += log_comm;
                }
                trades = trades.saturating_add(1);
                pos = 0;
            } else {
                if cfg.commission > 0.0 {
                    log_eq += 2.0 * log_comm;
                }
                trades = trades.saturating_add(2);
                pos = sign;
            }
        }

        let lr_t = lr[t];
        log_eq += (pos as f64) * lr_t;
        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }

        if COMPUTE_SHARPE {
            let step_r = if pos == 0 {
                0.0
            } else {
                (pos as f64) * lr_t.exp_m1()
            };
            count += 1;
            let delta = step_r - mean;
            mean += delta / count as f64;
            let delta2 = step_r - mean;
            m2 += delta * delta2;
        }

        sum_abs_pos += (pos as i64).abs();
        sum_pos += pos as i64;
    }

    let sharpe = if COMPUTE_SHARPE {
        let variance = if count > 1 {
            m2 / (count as f64 - 1.0)
        } else {
            0.0
        };
        let std = variance.sqrt();
        if std > 0.0 {
            mean / std
        } else {
            0.0
        }
    } else {
        0.0
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

    Metrics {
        pnl: log_eq.exp() - 1.0,
        sharpe,
        max_dd,
        trades,
        exposure,
        net_exposure,
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
        };
    }

    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let log_comm = if cfg.commission > 0.0 {
        (1.0 - cfg.commission).ln()
    } else {
        0.0
    };

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

    for t in start_t..n_total {
        let t_sig = t.saturating_sub(trade_offset);
        let f = fast[t_sig];
        let s = slow[t_sig];
        let eps = if cfg.eps_rel > 0.0 {
            cfg.eps_rel * 1.0_f64.max(s.abs())
        } else {
            0.0
        };

        let mut sign: i32 = (f > s + eps) as i32 - (f < s - eps) as i32;
        if cfg.long_only && sign < 0 {
            sign = 0;
        }

        let ht = halftrend_trend[t_sig];
        if !ht.is_finite() {
            sign = 0;
        } else if ht < 0.5 {
            sign = sign.max(0);
        } else {
            sign = sign.min(0);
        }

        let mh = macz_hist[t_sig];
        if !mh.is_finite() {
            sign = 0;
        } else if mh > 0.0 {
            sign = sign.max(0);
        } else if mh < 0.0 {
            sign = sign.min(0);
        } else {
            sign = 0;
        }

        let px = ott_prices[t_sig];
        let ott = ott_line[t_sig];
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
                if cfg.commission > 0.0 {
                    log_eq += log_comm;
                }
                if pos != 0 && cur_dir != 0 {
                    if cfg.commission > 0.0 {
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
                } else if pos == 0 && sign != 0 {
                    cur_entry_t = t;
                    cur_dir = sign;
                    cur_log_ret = if cfg.commission > 0.0 { log_comm } else { 0.0 };
                }
                pos = if sign == 0 { 0 } else { sign };
            } else if !cfg.allow_flip {
                if cfg.commission > 0.0 {
                    log_eq += log_comm;
                }
                if cur_dir != 0 {
                    if cfg.commission > 0.0 {
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
                }
                pos = 0;
            } else {
                if cfg.commission > 0.0 {
                    log_eq += 2.0 * log_comm;
                }
                if cur_dir != 0 {
                    if cfg.commission > 0.0 {
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
                cur_log_ret = if cfg.commission > 0.0 { log_comm } else { 0.0 };
                pos = sign;
            }
        }

        log_eq += (pos as f64) * lr[t];
        if cur_dir != 0 {
            cur_log_ret += (pos as f64) * lr[t];
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
    }
}
