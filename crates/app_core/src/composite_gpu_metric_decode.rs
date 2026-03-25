#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use ta_strategies::double_ma::Metrics;

pub(crate) const NON_DOUBLE_MA_GPU_METRICS_STRIDE: usize = 12;

const CLAMP_MAX: f64 = 1.0e6;
const EPS_DENOM: f64 = 1.0e-12;

fn clamp_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn derive_martin_ratio(pnl: f64, ulcer_index: f64) -> f64 {
    let martin = if ulcer_index > EPS_DENOM {
        pnl / ulcer_index
    } else if pnl > 0.0 {
        CLAMP_MAX
    } else if pnl < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    };
    martin.clamp(-CLAMP_MAX, CLAMP_MAX)
}

pub(crate) fn decode_non_double_ma_gpu_metrics_row(row: &[f32]) -> Metrics {
    let pnl = row[0] as f64;
    let sharpe = row[1] as f64;
    let sortino = row[2] as f64;
    let calmar = row[3] as f64;
    let omega = row[4] as f64;
    let volatility = row[5] as f64;
    let max_dd = row[6] as f64;
    let trades_f = row[7];
    let exposure = row[8] as f64;
    let net_exposure = row[9] as f64;
    let downside_deviation = row[10] as f64;
    let ulcer_index = row[11] as f64;

    let pnl = if pnl.is_finite() { pnl } else { 0.0 };
    let ulcer_index = clamp_non_negative(ulcer_index);

    Metrics {
        pnl,
        sharpe: if sharpe.is_finite() { sharpe } else { 0.0 },
        sortino: if sortino.is_finite() { sortino } else { 0.0 },
        downside_deviation: clamp_non_negative(downside_deviation),
        downside_frequency: 0.0,
        downside_severity: 0.0,
        calmar: if calmar.is_finite() { calmar } else { 0.0 },
        omega: if omega.is_finite() { omega } else { 0.0 },
        kappa_tau: 0.0,
        volatility: clamp_non_negative(volatility),
        ulcer_index,
        martin_ratio: derive_martin_ratio(pnl, ulcer_index),
        max_dd: clamp_non_negative(max_dd),
        trades: if trades_f.is_finite() && trades_f >= 0.0 {
            trades_f as u32
        } else {
            0
        },
        exposure: if exposure.is_finite() { exposure } else { 0.0 },
        net_exposure: if net_exposure.is_finite() {
            net_exposure
        } else {
            0.0
        },
        long_mean_return: 0.0,
        short_mean_return: 0.0,
        closed_trades: 0,
        winning_trades: 0,
        losing_trades: 0,
        gross_profit: 0.0,
        gross_loss: 0.0,
        underwater_fraction: 0.0,
        max_underwater_duration_bars: 0,
    }
}

