#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use ta_strategies::double_ma::Metrics;

pub(crate) const NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE: usize = 19;

const CLAMP_MAX: f64 = 1.0e6;
const EPS_DENOM: f64 = 1.0e-12;
const NON_DOUBLE_MA_GPU_REQUIRED_PREFIX_LEN: usize = 12;

#[inline]
fn assert_non_double_ma_gpu_metrics_row_shape(row: &[f32]) {
    #[cfg(any(test, debug_assertions))]
    {
        assert!(
            row.len() >= NON_DOUBLE_MA_GPU_REQUIRED_PREFIX_LEN,
            "non-double-MA GPU metrics rows require a 12-field prefix but got len={}",
            row.len()
        );
        assert!(
            row.len() <= NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE,
            "non-double-MA GPU metrics rows support at most 19 fields but got len={}",
            row.len()
        );
    }
}

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

/// Decodes one non-double-MA GPU metrics row.
///
/// Row contract:
/// - required prefix: 12 fields `[pnl, sharpe, sortino, calmar, omega, volatility, max_dd,
///   trades, exposure, net_exposure, downside_deviation, ulcer_index]`
/// - optional tail: up to 7 additional fields `[downside_frequency, downside_severity,
///   long_mean_return, short_mean_return, kappa_tau, underwater_fraction,
///   max_underwater_duration_bars]`
/// - callers may provide only the 12-field prefix; omitted tail fields default to zero
///
/// Trade-distribution fields (`closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`,
/// `gross_loss`) are still placeholder zeros on this GPU path because the kernel row format does
/// not export them yet.
pub(crate) fn decode_non_double_ma_gpu_metrics_row(row: &[f32]) -> Metrics {
    assert_non_double_ma_gpu_metrics_row_shape(row);

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
    let downside_frequency = row.get(12).copied().unwrap_or(0.0) as f64;
    let downside_severity = row.get(13).copied().unwrap_or(0.0) as f64;
    let long_mean_return = row.get(14).copied().unwrap_or(0.0) as f64;
    let short_mean_return = row.get(15).copied().unwrap_or(0.0) as f64;
    let kappa_tau = row.get(16).copied().unwrap_or(0.0) as f64;
    let underwater_fraction = row.get(17).copied().unwrap_or(0.0) as f64;
    let max_underwater_duration_bars_f = row.get(18).copied().unwrap_or(0.0);

    let pnl = if pnl.is_finite() { pnl } else { 0.0 };
    let ulcer_index = clamp_non_negative(ulcer_index);

    Metrics {
        pnl,
        sharpe: if sharpe.is_finite() { sharpe } else { 0.0 },
        sortino: if sortino.is_finite() { sortino } else { 0.0 },
        downside_deviation: clamp_non_negative(downside_deviation),
        downside_frequency: clamp_non_negative(downside_frequency).min(1.0),
        downside_severity: clamp_non_negative(downside_severity),
        calmar: if calmar.is_finite() { calmar } else { 0.0 },
        omega: if omega.is_finite() { omega } else { 0.0 },
        kappa_tau: if kappa_tau.is_finite() {
            kappa_tau
        } else {
            0.0
        },
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
        long_mean_return: if long_mean_return.is_finite() {
            long_mean_return
        } else {
            0.0
        },
        short_mean_return: if short_mean_return.is_finite() {
            short_mean_return
        } else {
            0.0
        },
        closed_trades: 0,
        winning_trades: 0,
        losing_trades: 0,
        gross_profit: 0.0,
        gross_loss: 0.0,
        underwater_fraction: clamp_non_negative(underwater_fraction).min(1.0),
        max_underwater_duration_bars: if max_underwater_duration_bars_f.is_finite()
            && max_underwater_duration_bars_f >= 0.0
        {
            max_underwater_duration_bars_f as u32
        } else {
            0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_prefix_row() -> [f32; NON_DOUBLE_MA_GPU_REQUIRED_PREFIX_LEN] {
        [
            40.0, // pnl
            1.5,  // sharpe
            2.5,  // sortino
            3.5,  // calmar
            1.1,  // omega
            0.4,  // volatility
            0.2,  // max_dd
            7.0,  // trades
            0.8,  // exposure
            0.3,  // net exposure
            0.25, // downside deviation
            5.0,  // ulcer index
        ]
    }

    #[test]
    fn decode_non_double_ma_gpu_metrics_row_defaults_optional_tail_to_zero() {
        let row = sample_prefix_row();
        let metrics = decode_non_double_ma_gpu_metrics_row(&row);

        assert_eq!(metrics.downside_frequency, 0.0);
        assert_eq!(metrics.downside_severity, 0.0);
        assert_eq!(metrics.long_mean_return, 0.0);
        assert_eq!(metrics.short_mean_return, 0.0);
        assert_eq!(metrics.kappa_tau, 0.0);
        assert_eq!(metrics.underwater_fraction, 0.0);
        assert_eq!(metrics.max_underwater_duration_bars, 0);
    }

    #[test]
    fn decode_non_double_ma_gpu_metrics_row_derives_martin_ratio_from_pnl_and_ulcer_index() {
        let row = sample_prefix_row();
        let metrics = decode_non_double_ma_gpu_metrics_row(&row);

        assert_eq!(metrics.pnl, 40.0);
        assert_eq!(metrics.ulcer_index, 5.0);
        assert_eq!(metrics.martin_ratio, 8.0);
    }

    #[test]
    fn decode_non_double_ma_gpu_metrics_row_keeps_trade_distribution_placeholders_zero() {
        let row = [
            40.0, 1.5, 2.5, 3.5, 1.1, 0.4, 0.2, 7.0, 0.8, 0.3, 0.25, 5.0, 0.4, 0.6, 0.01, -0.02,
            1.7, 0.15, 12.0,
        ];
        let metrics = decode_non_double_ma_gpu_metrics_row(&row);

        assert_eq!(metrics.closed_trades, 0);
        assert_eq!(metrics.winning_trades, 0);
        assert_eq!(metrics.losing_trades, 0);
        assert_eq!(metrics.gross_profit, 0.0);
        assert_eq!(metrics.gross_loss, 0.0);
    }

    #[test]
    fn decode_non_double_ma_gpu_metrics_row_asserts_on_short_rows_in_test_builds() {
        let row = [0.0f32; NON_DOUBLE_MA_GPU_REQUIRED_PREFIX_LEN - 1];
        let panic = std::panic::catch_unwind(|| decode_non_double_ma_gpu_metrics_row(&row));
        assert!(
            panic.is_err(),
            "short rows should trip debug/test assertions"
        );
    }

    #[test]
    fn decode_non_double_ma_gpu_metrics_row_asserts_on_rows_longer_than_tail_contract() {
        let row = [0.0f32; NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE + 1];
        let panic = std::panic::catch_unwind(|| decode_non_double_ma_gpu_metrics_row(&row));
        assert!(
            panic.is_err(),
            "rows longer than the 19-field contract should assert"
        );
    }
}
