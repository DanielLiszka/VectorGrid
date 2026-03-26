use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use vector_ta::indicators::moving_averages::ma::{ma, MaData};
use vector_ta::indicators::moving_averages::ma_batch::ma_batch;
use vector_ta::utilities::data_loader::{source_type, Candles};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoubleMaParams {
    pub fast_len: u16,
    pub slow_len: u16,

    pub fast_ma_id: u16,

    pub slow_ma_id: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoubleMaBatchRange {
    pub fast_len: (u16, u16, u16),
    pub slow_len: (u16, u16, u16),
    pub fast_ma_types: Vec<String>,
    pub slow_ma_types: Vec<String>,
}

impl Default for DoubleMaBatchRange {
    fn default() -> Self {
        Self {
            fast_len: (9, 9, 0),
            slow_len: (21, 21, 0),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metrics {
    pub pnl: f64,
    pub sharpe: f64,
    /// Sortino ratio, computed on per-bar strategy returns relative to the configured hurdle
    /// threshold (0 by default).
    #[serde(default)]
    pub sortino: f64,
    /// Downside deviation, computed on per-bar strategy returns relative to the configured hurdle
    /// threshold (0 by default).
    #[serde(default)]
    pub downside_deviation: f64,
    /// Fraction of observed bars with negative strategy return.
    #[serde(default)]
    pub downside_frequency: f64,
    /// Mean absolute negative return across negative-return bars only.
    #[serde(default)]
    pub downside_severity: f64,
    /// Calmar ratio, computed as pnl / max_dd (epsilon-guarded).
    #[serde(default)]
    pub calmar: f64,
    /// Omega ratio, computed on per-bar strategy returns relative to the configured hurdle
    /// threshold (0 by default).
    #[serde(default)]
    pub omega: f64,
    /// Kappa-3 ratio, computed from the third lower partial moment of per-bar strategy returns
    /// relative to the configured hurdle threshold (0 by default).
    #[serde(default)]
    pub kappa_tau: f64,
    /// Volatility (std dev) of per-bar strategy returns.
    #[serde(default)]
    pub volatility: f64,
    /// Ulcer index computed from percentage drawdown depth along the equity path.
    #[serde(default)]
    pub ulcer_index: f64,
    /// Martin ratio computed as pnl / ulcer_index (epsilon-guarded).
    #[serde(default)]
    pub martin_ratio: f64,
    pub max_dd: f64,
    pub trades: u32,
    pub exposure: f64,
    pub net_exposure: f64,
    /// Mean per-bar strategy return while long.
    #[serde(default)]
    pub long_mean_return: f64,
    /// Mean per-bar strategy return while short.
    #[serde(default)]
    pub short_mean_return: f64,
    /// Count of closed trades used for exact trade-distribution summaries.
    #[serde(default)]
    pub closed_trades: u32,
    /// Count of winning closed trades.
    #[serde(default)]
    pub winning_trades: u32,
    /// Count of losing closed trades.
    #[serde(default)]
    pub losing_trades: u32,
    /// Sum of positive closed-trade returns.
    #[serde(default)]
    pub gross_profit: f64,
    /// Sum of absolute negative closed-trade returns.
    #[serde(default)]
    pub gross_loss: f64,
    /// Fraction of evaluated bars spent below the running equity peak.
    #[serde(default)]
    pub underwater_fraction: f64,
    /// Longest contiguous underwater run in evaluated bars.
    #[serde(default)]
    pub max_underwater_duration_bars: u32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            pnl: 0.0,
            sharpe: 0.0,
            sortino: 0.0,
            downside_deviation: 0.0,
            downside_frequency: 0.0,
            downside_severity: 0.0,
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
}

#[allow(clippy::type_complexity)]
fn finalize_risk_metrics(
    compute_return_stats: bool,
    compute_drawdown_risk: bool,
    count: u64,
    obs: u64,
    hurdle_tau: f64,
    mean: f64,
    m2: f64,
    sum_neg_sq: f64,
    sum_neg_cu: f64,
    count_neg: u64,
    sum_pos_r: f64,
    sum_neg_abs_r: f64,
    long_obs: u64,
    short_obs: u64,
    long_sum_r: f64,
    short_sum_r: f64,
    ulcer_sum_sq: f64,
    max_log_dd: f64,
    log_eq: f64,
    underwater_bars: u64,
    max_underwater_duration_bars: u64,
) -> (
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    u32,
) {
    const EPS_DENOM: f64 = 1e-12;
    const CLAMP_MAX: f64 = 1e6;

    let (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        long_mean_return,
        short_mean_return,
    ) = if compute_return_stats {
        let variance = if count > 1 {
            m2 / (count as f64 - 1.0)
        } else {
            0.0
        };
        let std = variance.sqrt();
        let sharpe = if std > 0.0 { mean / std } else { 0.0 };
        let denom = (count as f64).max(1.0);
        let downside_deviation = (sum_neg_sq / denom).sqrt();
        let mean_excess = mean - hurdle_tau;
        let sortino_raw = if downside_deviation > 0.0 {
            mean_excess / downside_deviation
        } else if mean_excess > 0.0 {
            CLAMP_MAX
        } else if mean_excess < 0.0 {
            -CLAMP_MAX
        } else {
            0.0
        };
        let sortino = sortino_raw.clamp(-CLAMP_MAX, CLAMP_MAX);
        let omega_raw = (sum_pos_r + EPS_DENOM) / (sum_neg_abs_r + EPS_DENOM);
        let omega = omega_raw.clamp(0.0, CLAMP_MAX);
        let kappa_denom = (sum_neg_cu / denom).cbrt();
        let kappa_raw = if kappa_denom > 0.0 {
            mean_excess / kappa_denom
        } else if mean_excess > 0.0 {
            CLAMP_MAX
        } else if mean_excess < 0.0 {
            -CLAMP_MAX
        } else {
            0.0
        };
        let kappa_tau = kappa_raw.clamp(-CLAMP_MAX, CLAMP_MAX);
        let downside_frequency = count_neg as f64 / (obs as f64).max(1.0);
        let downside_severity = if count_neg > 0 {
            sum_neg_abs_r / count_neg as f64
        } else {
            0.0
        };
        let long_mean_return = if long_obs > 0 {
            long_sum_r / long_obs as f64
        } else {
            0.0
        };
        let short_mean_return = if short_obs > 0 {
            short_sum_r / short_obs as f64
        } else {
            0.0
        };
        (
            sharpe,
            std,
            sortino,
            downside_deviation,
            downside_frequency,
            downside_severity,
            omega,
            kappa_tau,
            long_mean_return,
            short_mean_return,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    };

    let max_dd = if max_log_dd > 0.0 {
        -(-max_log_dd).exp_m1()
    } else {
        0.0
    };
    let pnl = log_eq.exp() - 1.0;
    let (ulcer_index, martin_ratio) = if compute_drawdown_risk {
        let denom = (obs as f64).max(1.0);
        let ulcer_index = (ulcer_sum_sq / denom).sqrt();
        let martin_raw = if ulcer_index > EPS_DENOM {
            pnl / ulcer_index
        } else if pnl > 0.0 {
            CLAMP_MAX
        } else if pnl < 0.0 {
            -CLAMP_MAX
        } else {
            0.0
        };
        (ulcer_index, martin_raw.clamp(-CLAMP_MAX, CLAMP_MAX))
    } else {
        (0.0, 0.0)
    };
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
    let underwater_fraction = if compute_drawdown_risk && obs > 0 {
        underwater_bars as f64 / obs as f64
    } else {
        0.0
    };
    let max_underwater_duration_bars = if compute_drawdown_risk {
        max_underwater_duration_bars.min(u32::MAX as u64) as u32
    } else {
        0
    };

    (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        max_dd,
        ulcer_index,
        martin_ratio,
        calmar,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
    )
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub entry_t: usize,
    pub exit_t: usize,
    pub direction: i32,
    pub pnl: f64,
    pub bars: u32,
    pub open: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeStats {
    pub closed_trades: usize,
    pub open_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub expectancy: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExitMode {
    Cross,
    KaufmanStop,
}

fn default_exit_mode() -> ExitMode {
    ExitMode::Cross
}

fn default_kaufman_stop_period() -> u16 {
    22
}

fn default_kaufman_stop_mult() -> f64 {
    2.0
}

fn default_kaufman_stop_ma_type() -> String {
    "sma".to_string()
}

fn default_trend_quality_enable() -> bool {
    false
}

fn default_trend_quality_linearreg_period() -> u16 {
    34
}

fn default_trend_quality_angle_min_deg() -> f64 {
    2.0
}

fn default_trend_quality_bbw_period() -> u16 {
    20
}

fn default_trend_quality_bbw_mult() -> f64 {
    2.0
}

fn default_trend_quality_bbw_min() -> f64 {
    0.04
}

fn default_trend_quality_roc_period() -> u16 {
    9
}

fn default_trend_quality_use_roc() -> bool {
    true
}

fn default_trend_quality_use_adx() -> bool {
    false
}

fn default_trend_quality_adx_period() -> u16 {
    14
}

fn default_trend_quality_adx_min() -> f64 {
    20.0
}

fn default_regime_hybrid_enable() -> bool {
    false
}

fn default_regime_hybrid_adx_period() -> u16 {
    14
}

fn default_regime_hybrid_adx_enter() -> f64 {
    22.0
}

fn default_regime_hybrid_adx_exit() -> f64 {
    18.0
}

fn default_regime_hybrid_reversion_entry_rel() -> f64 {
    0.0035
}

fn default_regime_hybrid_reversion_exit_rel() -> f64 {
    0.0012
}

fn default_breakout_momentum_enable() -> bool {
    false
}

fn default_breakout_bbands_period() -> u16 {
    20
}

fn default_breakout_bbands_mult() -> f64 {
    2.0
}

fn default_breakout_confirm_bars() -> u16 {
    1
}

fn default_breakout_macd_fast_period() -> u16 {
    12
}

fn default_breakout_macd_slow_period() -> u16 {
    26
}

fn default_breakout_macd_signal_period() -> u16 {
    9
}

fn default_breakout_cmo_period() -> u16 {
    14
}

fn default_breakout_cmo_min_abs() -> f64 {
    20.0
}

fn default_breakout_adosc_short_period() -> u16 {
    3
}

fn default_breakout_adosc_long_period() -> u16 {
    10
}

fn default_breakout_use_adx() -> bool {
    false
}

fn default_breakout_adx_period() -> u16 {
    14
}

fn default_breakout_adx_min() -> f64 {
    20.0
}

fn default_volume_flow_confirm_enable() -> bool {
    false
}

fn default_volume_flow_obv_smooth_period() -> u16 {
    21
}

fn default_volume_flow_mfi_period() -> u16 {
    14
}

fn default_volume_flow_mfi_long_min() -> f64 {
    55.0
}

fn default_volume_flow_mfi_short_max() -> f64 {
    45.0
}

fn default_volume_flow_confirm_bars() -> u16 {
    1
}

fn default_slope_efficiency_trend_enable() -> bool {
    false
}

fn default_set_linearreg_period() -> u16 {
    34
}

fn default_set_angle_min_deg() -> f64 {
    2.0
}

fn default_set_er_period() -> u16 {
    14
}

fn default_set_er_min() -> f64 {
    0.3
}

fn default_set_chop_period() -> u16 {
    14
}

fn default_set_chop_max() -> f64 {
    45.0
}

fn default_momentum_triple_stack_enable() -> bool {
    false
}

fn default_mts_roc_period() -> u16 {
    10
}

fn default_mts_mom_period() -> u16 {
    20
}

fn default_mts_ppo_fast_period() -> u16 {
    12
}

fn default_mts_ppo_slow_period() -> u16 {
    26
}

fn default_oscillator_reversion_confluence_enable() -> bool {
    false
}

fn default_pressure_impulse_trend_enable() -> bool {
    false
}

fn default_pit_efi_period() -> u16 {
    13
}

fn default_pit_vosc_short_period() -> u16 {
    5
}

fn default_pit_vosc_long_period() -> u16 {
    20
}

fn default_pit_cfo_period() -> u16 {
    9
}

fn default_pit_cfo_scalar() -> f64 {
    100.0
}

fn default_pit_confirm_bars() -> u16 {
    2
}

fn default_orc_rsi_period() -> u16 {
    14
}

fn default_orc_stoch_fastk_period() -> u16 {
    5
}

fn default_orc_stoch_fastd_period() -> u16 {
    3
}

fn default_orc_willr_period() -> u16 {
    14
}

fn default_cooldown_bars() -> u16 {
    0
}

fn default_signal_persistence_bars() -> u16 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub long_only: bool,
    pub allow_flip: bool,
    pub trade_on_next_bar: bool,
    #[serde(default)]
    pub cost_preset: Option<String>,
    #[serde(default)]
    pub fee_bps: f64,
    #[serde(default)]
    pub spread_bps: f64,
    #[serde(default)]
    pub slippage_bps: f64,
    #[serde(default)]
    pub stop_loss_pct: Option<f64>,
    #[serde(default)]
    pub take_profit_pct: Option<f64>,
    #[serde(default = "default_exit_mode")]
    pub exit_mode: ExitMode,
    #[serde(default = "default_kaufman_stop_period")]
    pub kaufman_stop_period: u16,
    #[serde(default = "default_kaufman_stop_mult")]
    pub kaufman_stop_mult: f64,
    #[serde(default = "default_kaufman_stop_ma_type")]
    pub kaufman_stop_ma_type: String,
    #[serde(default = "default_trend_quality_enable")]
    pub trend_quality_enable: bool,
    #[serde(default = "default_trend_quality_linearreg_period")]
    pub trend_quality_linearreg_period: u16,
    #[serde(default = "default_trend_quality_angle_min_deg")]
    pub trend_quality_angle_min_deg: f64,
    #[serde(default = "default_trend_quality_bbw_period")]
    pub trend_quality_bbw_period: u16,
    #[serde(default = "default_trend_quality_bbw_mult")]
    pub trend_quality_bbw_mult: f64,
    #[serde(default = "default_trend_quality_bbw_min")]
    pub trend_quality_bbw_min: f64,
    #[serde(default = "default_trend_quality_roc_period")]
    pub trend_quality_roc_period: u16,
    #[serde(default = "default_trend_quality_use_roc")]
    pub trend_quality_use_roc: bool,
    #[serde(default = "default_trend_quality_use_adx")]
    pub trend_quality_use_adx: bool,
    #[serde(default = "default_trend_quality_adx_period")]
    pub trend_quality_adx_period: u16,
    #[serde(default = "default_trend_quality_adx_min")]
    pub trend_quality_adx_min: f64,
    #[serde(default = "default_regime_hybrid_enable")]
    pub regime_hybrid_enable: bool,
    #[serde(default = "default_regime_hybrid_adx_period")]
    pub regime_hybrid_adx_period: u16,
    #[serde(default = "default_regime_hybrid_adx_enter")]
    pub regime_hybrid_adx_enter: f64,
    #[serde(default = "default_regime_hybrid_adx_exit")]
    pub regime_hybrid_adx_exit: f64,
    #[serde(default = "default_regime_hybrid_reversion_entry_rel")]
    pub regime_hybrid_reversion_entry_rel: f64,
    #[serde(default = "default_regime_hybrid_reversion_exit_rel")]
    pub regime_hybrid_reversion_exit_rel: f64,
    #[serde(default = "default_breakout_momentum_enable")]
    pub breakout_momentum_enable: bool,
    #[serde(default = "default_breakout_bbands_period")]
    pub breakout_bbands_period: u16,
    #[serde(default = "default_breakout_bbands_mult")]
    pub breakout_bbands_mult: f64,
    #[serde(default = "default_breakout_confirm_bars")]
    pub breakout_confirm_bars: u16,
    #[serde(default = "default_breakout_macd_fast_period")]
    pub breakout_macd_fast_period: u16,
    #[serde(default = "default_breakout_macd_slow_period")]
    pub breakout_macd_slow_period: u16,
    #[serde(default = "default_breakout_macd_signal_period")]
    pub breakout_macd_signal_period: u16,
    #[serde(default = "default_breakout_cmo_period")]
    pub breakout_cmo_period: u16,
    #[serde(default = "default_breakout_cmo_min_abs")]
    pub breakout_cmo_min_abs: f64,
    #[serde(default = "default_breakout_adosc_short_period")]
    pub breakout_adosc_short_period: u16,
    #[serde(default = "default_breakout_adosc_long_period")]
    pub breakout_adosc_long_period: u16,
    #[serde(default = "default_breakout_use_adx")]
    pub breakout_use_adx: bool,
    #[serde(default = "default_breakout_adx_period")]
    pub breakout_adx_period: u16,
    #[serde(default = "default_breakout_adx_min")]
    pub breakout_adx_min: f64,
    #[serde(default = "default_volume_flow_confirm_enable")]
    pub volume_flow_confirm_enable: bool,
    #[serde(default = "default_volume_flow_obv_smooth_period")]
    pub volume_flow_obv_smooth_period: u16,
    #[serde(default = "default_volume_flow_mfi_period")]
    pub volume_flow_mfi_period: u16,
    #[serde(default = "default_volume_flow_mfi_long_min")]
    pub volume_flow_mfi_long_min: f64,
    #[serde(default = "default_volume_flow_mfi_short_max")]
    pub volume_flow_mfi_short_max: f64,
    #[serde(default = "default_volume_flow_confirm_bars")]
    pub volume_flow_confirm_bars: u16,
    #[serde(default = "default_slope_efficiency_trend_enable")]
    pub slope_efficiency_trend_enable: bool,
    #[serde(default = "default_set_linearreg_period")]
    pub set_linearreg_period: u16,
    #[serde(default = "default_set_angle_min_deg")]
    pub set_angle_min_deg: f64,
    #[serde(default = "default_set_er_period")]
    pub set_er_period: u16,
    #[serde(default = "default_set_er_min")]
    pub set_er_min: f64,
    #[serde(default = "default_set_chop_period")]
    pub set_chop_period: u16,
    #[serde(default = "default_set_chop_max")]
    pub set_chop_max: f64,
    #[serde(default = "default_momentum_triple_stack_enable")]
    pub momentum_triple_stack_enable: bool,
    #[serde(default = "default_mts_roc_period")]
    pub mts_roc_period: u16,
    #[serde(default = "default_mts_mom_period")]
    pub mts_mom_period: u16,
    #[serde(default = "default_mts_ppo_fast_period")]
    pub mts_ppo_fast_period: u16,
    #[serde(default = "default_mts_ppo_slow_period")]
    pub mts_ppo_slow_period: u16,
    #[serde(default = "default_oscillator_reversion_confluence_enable")]
    pub oscillator_reversion_confluence_enable: bool,
    #[serde(default = "default_pressure_impulse_trend_enable")]
    pub pressure_impulse_trend_enable: bool,
    #[serde(default = "default_pit_efi_period")]
    pub pit_efi_period: u16,
    #[serde(default = "default_pit_vosc_short_period")]
    pub pit_vosc_short_period: u16,
    #[serde(default = "default_pit_vosc_long_period")]
    pub pit_vosc_long_period: u16,
    #[serde(default = "default_pit_cfo_period")]
    pub pit_cfo_period: u16,
    #[serde(default = "default_pit_cfo_scalar")]
    pub pit_cfo_scalar: f64,
    #[serde(default = "default_pit_confirm_bars")]
    pub pit_confirm_bars: u16,
    #[serde(default = "default_orc_rsi_period")]
    pub orc_rsi_period: u16,
    #[serde(default = "default_orc_stoch_fastk_period")]
    pub orc_stoch_fastk_period: u16,
    #[serde(default = "default_orc_stoch_fastd_period")]
    pub orc_stoch_fastd_period: u16,
    #[serde(default = "default_orc_willr_period")]
    pub orc_willr_period: u16,
    #[serde(default = "default_cooldown_bars")]
    pub cooldown_bars: u16,
    #[serde(default = "default_signal_persistence_bars")]
    pub signal_persistence_bars: u16,
    /// Compatibility fallback. When positive, this takes precedence over bps fields.
    pub commission: f64,
    pub eps_rel: f64,
    #[serde(default)]
    pub hurdle_tau: Option<f64>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            long_only: true,
            allow_flip: true,
            trade_on_next_bar: true,
            cost_preset: None,
            fee_bps: 0.0,
            spread_bps: 0.0,
            slippage_bps: 0.0,
            stop_loss_pct: None,
            take_profit_pct: None,
            exit_mode: ExitMode::Cross,
            kaufman_stop_period: 22,
            kaufman_stop_mult: 2.0,
            kaufman_stop_ma_type: "sma".to_string(),
            trend_quality_enable: false,
            trend_quality_linearreg_period: 34,
            trend_quality_angle_min_deg: 2.0,
            trend_quality_bbw_period: 20,
            trend_quality_bbw_mult: 2.0,
            trend_quality_bbw_min: 0.04,
            trend_quality_roc_period: 9,
            trend_quality_use_roc: true,
            trend_quality_use_adx: false,
            trend_quality_adx_period: 14,
            trend_quality_adx_min: 20.0,
            regime_hybrid_enable: false,
            regime_hybrid_adx_period: 14,
            regime_hybrid_adx_enter: 22.0,
            regime_hybrid_adx_exit: 18.0,
            regime_hybrid_reversion_entry_rel: 0.0035,
            regime_hybrid_reversion_exit_rel: 0.0012,
            breakout_momentum_enable: false,
            breakout_bbands_period: 20,
            breakout_bbands_mult: 2.0,
            breakout_confirm_bars: 1,
            breakout_macd_fast_period: 12,
            breakout_macd_slow_period: 26,
            breakout_macd_signal_period: 9,
            breakout_cmo_period: 14,
            breakout_cmo_min_abs: 20.0,
            breakout_adosc_short_period: 3,
            breakout_adosc_long_period: 10,
            breakout_use_adx: false,
            breakout_adx_period: 14,
            breakout_adx_min: 20.0,
            volume_flow_confirm_enable: false,
            volume_flow_obv_smooth_period: 21,
            volume_flow_mfi_period: 14,
            volume_flow_mfi_long_min: 55.0,
            volume_flow_mfi_short_max: 45.0,
            volume_flow_confirm_bars: 1,
            slope_efficiency_trend_enable: false,
            set_linearreg_period: 34,
            set_angle_min_deg: 2.0,
            set_er_period: 14,
            set_er_min: 0.3,
            set_chop_period: 14,
            set_chop_max: 45.0,
            momentum_triple_stack_enable: false,
            mts_roc_period: 10,
            mts_mom_period: 20,
            mts_ppo_fast_period: 12,
            mts_ppo_slow_period: 26,
            oscillator_reversion_confluence_enable: false,
            pressure_impulse_trend_enable: false,
            pit_efi_period: 13,
            pit_vosc_short_period: 5,
            pit_vosc_long_period: 20,
            pit_cfo_period: 9,
            pit_cfo_scalar: 100.0,
            pit_confirm_bars: 2,
            orc_rsi_period: 14,
            orc_stoch_fastk_period: 5,
            orc_stoch_fastd_period: 3,
            orc_willr_period: 14,
            cooldown_bars: 0,
            signal_persistence_bars: 1,
            commission: 0.0,
            eps_rel: 0.0,
            hurdle_tau: None,
        }
    }
}

impl StrategyConfig {
    #[inline]
    fn clean_bps(v: f64) -> f64 {
        if v.is_finite() && v >= 0.0 {
            v
        } else {
            0.0
        }
    }

    #[inline]
    pub fn per_side_cost_bps(&self) -> f64 {
        let fee = Self::clean_bps(self.fee_bps);
        let spread = Self::clean_bps(self.spread_bps);
        let slippage = Self::clean_bps(self.slippage_bps);
        fee + slippage + (spread * 0.5)
    }

    #[inline]
    pub fn commission_from_bps(&self) -> f64 {
        self.per_side_cost_bps() / 10_000.0
    }

    #[inline]
    pub fn effective_commission(&self) -> f64 {
        let commission = self.commission;
        if commission.is_finite() && commission > 0.0 {
            commission
        } else {
            self.commission_from_bps()
        }
    }

    #[inline]
    fn clean_risk_pct(v: Option<f64>) -> Option<f64> {
        let x = v?;
        if x.is_finite() && x > 0.0 {
            Some(x.min(0.95))
        } else {
            None
        }
    }

    #[inline]
    pub fn stop_loss_pct(&self) -> Option<f64> {
        Self::clean_risk_pct(self.stop_loss_pct)
    }

    #[inline]
    pub fn take_profit_pct(&self) -> Option<f64> {
        Self::clean_risk_pct(self.take_profit_pct)
    }

    #[inline]
    pub fn cooldown_bars(&self) -> usize {
        self.cooldown_bars as usize
    }

    #[inline]
    pub fn signal_persistence_bars(&self) -> usize {
        self.signal_persistence_bars.max(1) as usize
    }

    #[inline]
    pub fn uses_execution_overlays(&self) -> bool {
        self.cooldown_bars() > 0 || self.signal_persistence_bars() > 1
    }

    #[inline]
    pub fn exit_mode(&self) -> ExitMode {
        self.exit_mode
    }

    #[inline]
    pub fn uses_kaufman_stop_exit(&self) -> bool {
        matches!(self.exit_mode, ExitMode::KaufmanStop)
    }

    #[inline]
    pub fn kaufman_stop_period(&self) -> usize {
        let p = self.kaufman_stop_period.max(1);
        p as usize
    }

    #[inline]
    pub fn kaufman_stop_mult(&self) -> f64 {
        let m = self.kaufman_stop_mult;
        if m.is_finite() && m > 0.0 {
            m
        } else {
            2.0
        }
    }

    #[inline]
    pub fn kaufman_stop_ma_type(&self) -> &str {
        let s = self.kaufman_stop_ma_type.trim();
        if s.is_empty() {
            "sma"
        } else {
            s
        }
    }

    #[inline]
    pub fn uses_trend_quality_filter(&self) -> bool {
        self.trend_quality_enable
    }

    #[inline]
    pub fn trend_quality_linearreg_period(&self) -> usize {
        self.trend_quality_linearreg_period.max(1) as usize
    }

    #[inline]
    pub fn trend_quality_angle_min_deg(&self) -> f64 {
        let x = self.trend_quality_angle_min_deg;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            2.0
        }
    }

    #[inline]
    pub fn trend_quality_bbw_period(&self) -> usize {
        self.trend_quality_bbw_period.max(1) as usize
    }

    #[inline]
    pub fn trend_quality_bbw_mult(&self) -> f64 {
        let x = self.trend_quality_bbw_mult;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            2.0
        }
    }

    #[inline]
    pub fn trend_quality_bbw_min(&self) -> f64 {
        let x = self.trend_quality_bbw_min;
        if x.is_finite() && x >= 0.0 {
            x
        } else {
            0.04
        }
    }

    #[inline]
    pub fn trend_quality_roc_period(&self) -> usize {
        self.trend_quality_roc_period.max(1) as usize
    }

    #[inline]
    pub fn trend_quality_use_roc(&self) -> bool {
        self.trend_quality_use_roc
    }

    #[inline]
    pub fn trend_quality_use_adx(&self) -> bool {
        self.trend_quality_use_adx
    }

    #[inline]
    pub fn trend_quality_adx_period(&self) -> usize {
        self.trend_quality_adx_period.max(1) as usize
    }

    #[inline]
    pub fn trend_quality_adx_min(&self) -> f64 {
        let x = self.trend_quality_adx_min;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            20.0
        }
    }

    #[inline]
    pub fn uses_regime_hybrid_filter(&self) -> bool {
        self.regime_hybrid_enable
    }

    #[inline]
    pub fn regime_hybrid_adx_period(&self) -> usize {
        self.regime_hybrid_adx_period.max(1) as usize
    }

    #[inline]
    pub fn regime_hybrid_adx_enter(&self) -> f64 {
        let x = self.regime_hybrid_adx_enter;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            22.0
        }
    }

    #[inline]
    pub fn regime_hybrid_adx_exit(&self) -> f64 {
        let x = self.regime_hybrid_adx_exit;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            18.0
        }
    }

    #[inline]
    pub fn regime_hybrid_reversion_entry_rel(&self) -> f64 {
        let x = self.regime_hybrid_reversion_entry_rel;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            0.0035
        }
    }

    #[inline]
    pub fn regime_hybrid_reversion_exit_rel(&self) -> f64 {
        let x = self.regime_hybrid_reversion_exit_rel;
        if x.is_finite() && x >= 0.0 {
            x
        } else {
            0.0012
        }
    }

    #[inline]
    pub fn uses_breakout_momentum_filter(&self) -> bool {
        self.breakout_momentum_enable
    }

    #[inline]
    pub fn breakout_bbands_period(&self) -> usize {
        self.breakout_bbands_period.max(1) as usize
    }

    #[inline]
    pub fn breakout_bbands_mult(&self) -> f64 {
        let x = self.breakout_bbands_mult;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            2.0
        }
    }

    #[inline]
    pub fn breakout_confirm_bars(&self) -> usize {
        self.breakout_confirm_bars.max(1) as usize
    }

    #[inline]
    pub fn breakout_macd_fast_period(&self) -> usize {
        self.breakout_macd_fast_period.max(1) as usize
    }

    #[inline]
    pub fn breakout_macd_slow_period(&self) -> usize {
        self.breakout_macd_slow_period.max(1) as usize
    }

    #[inline]
    pub fn breakout_macd_signal_period(&self) -> usize {
        self.breakout_macd_signal_period.max(1) as usize
    }

    #[inline]
    pub fn breakout_cmo_period(&self) -> usize {
        self.breakout_cmo_period.max(1) as usize
    }

    #[inline]
    pub fn breakout_cmo_min_abs(&self) -> f64 {
        let x = self.breakout_cmo_min_abs;
        if x.is_finite() && x >= 0.0 {
            x
        } else {
            20.0
        }
    }

    #[inline]
    pub fn breakout_adosc_short_period(&self) -> usize {
        self.breakout_adosc_short_period.max(1) as usize
    }

    #[inline]
    pub fn breakout_adosc_long_period(&self) -> usize {
        self.breakout_adosc_long_period.max(1) as usize
    }

    #[inline]
    pub fn breakout_use_adx(&self) -> bool {
        self.breakout_use_adx
    }

    #[inline]
    pub fn breakout_adx_period(&self) -> usize {
        self.breakout_adx_period.max(1) as usize
    }

    #[inline]
    pub fn breakout_adx_min(&self) -> f64 {
        let x = self.breakout_adx_min;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            20.0
        }
    }

    #[inline]
    pub fn uses_volume_flow_confirm_filter(&self) -> bool {
        self.volume_flow_confirm_enable
    }

    #[inline]
    pub fn volume_flow_obv_smooth_period(&self) -> usize {
        self.volume_flow_obv_smooth_period.max(1) as usize
    }

    #[inline]
    pub fn volume_flow_mfi_period(&self) -> usize {
        self.volume_flow_mfi_period.max(1) as usize
    }

    #[inline]
    pub fn volume_flow_mfi_long_min(&self) -> f64 {
        let x = self.volume_flow_mfi_long_min;
        if x.is_finite() {
            x.clamp(0.0, 100.0)
        } else {
            55.0
        }
    }

    #[inline]
    pub fn volume_flow_mfi_short_max(&self) -> f64 {
        let x = self.volume_flow_mfi_short_max;
        if x.is_finite() {
            x.clamp(0.0, 100.0)
        } else {
            45.0
        }
    }

    #[inline]
    pub fn volume_flow_confirm_bars(&self) -> usize {
        self.volume_flow_confirm_bars.max(1) as usize
    }

    pub fn uses_slope_efficiency_trend_filter(&self) -> bool {
        self.slope_efficiency_trend_enable
    }

    pub fn set_linearreg_period(&self) -> usize {
        self.set_linearreg_period.max(1) as usize
    }

    pub fn set_angle_min_deg(&self) -> f64 {
        let x = self.set_angle_min_deg;
        if x.is_finite() {
            x
        } else {
            2.0
        }
    }

    pub fn set_er_period(&self) -> usize {
        self.set_er_period.max(1) as usize
    }

    pub fn set_er_min(&self) -> f64 {
        let x = self.set_er_min;
        if x.is_finite() {
            x
        } else {
            0.3
        }
    }

    pub fn set_chop_period(&self) -> usize {
        self.set_chop_period.max(1) as usize
    }

    pub fn set_chop_max(&self) -> f64 {
        let x = self.set_chop_max;
        if x.is_finite() {
            x
        } else {
            45.0
        }
    }

    pub fn uses_momentum_triple_stack_filter(&self) -> bool {
        self.momentum_triple_stack_enable
    }

    pub fn mts_roc_period(&self) -> usize {
        self.mts_roc_period.max(1) as usize
    }

    pub fn mts_mom_period(&self) -> usize {
        self.mts_mom_period.max(1) as usize
    }

    pub fn mts_ppo_fast_period(&self) -> usize {
        self.mts_ppo_fast_period.max(1) as usize
    }

    pub fn mts_ppo_slow_period(&self) -> usize {
        self.mts_ppo_slow_period.max(1) as usize
    }

    pub fn uses_oscillator_reversion_confluence_filter(&self) -> bool {
        self.oscillator_reversion_confluence_enable
    }

    pub fn uses_pressure_impulse_trend_filter(&self) -> bool {
        self.pressure_impulse_trend_enable
    }

    pub fn pit_efi_period(&self) -> usize {
        self.pit_efi_period.max(1) as usize
    }

    pub fn pit_vosc_short_period(&self) -> usize {
        self.pit_vosc_short_period.max(1) as usize
    }

    pub fn pit_vosc_long_period(&self) -> usize {
        let min_long = (self.pit_vosc_short_period() + 1) as u16;
        self.pit_vosc_long_period.max(min_long) as usize
    }

    pub fn pit_cfo_period(&self) -> usize {
        self.pit_cfo_period.max(1) as usize
    }

    pub fn pit_cfo_scalar(&self) -> f64 {
        let x = self.pit_cfo_scalar;
        if x.is_finite() && x > 0.0 {
            x
        } else {
            100.0
        }
    }

    pub fn pit_confirm_bars(&self) -> usize {
        self.pit_confirm_bars.max(1) as usize
    }

    pub fn orc_rsi_period(&self) -> usize {
        self.orc_rsi_period.max(1) as usize
    }

    pub fn orc_stoch_fastk_period(&self) -> usize {
        self.orc_stoch_fastk_period.max(1) as usize
    }

    pub fn orc_stoch_fastd_period(&self) -> usize {
        self.orc_stoch_fastd_period.max(1) as usize
    }

    pub fn orc_willr_period(&self) -> usize {
        self.orc_willr_period.max(1) as usize
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinMaxBins {
    pub start_t: usize,
    pub end_t: usize,
    pub bins: usize,
    pub minmax: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndicatorBar {
    pub time: i64,
    pub from_index: u32,
    pub to_index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoubleMaCurves {
    pub equity: MinMaxBins,
    pub drawdown: MinMaxBins,
    #[serde(default)]
    pub trades: Vec<Trade>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_stats: Option<TradeStats>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indicator_bars: Vec<IndicatorBar>,
}

fn clamp_bins(requested: usize, n_steps: usize) -> usize {
    if n_steps == 0 {
        return 0;
    }
    let req = requested.clamp(1, 8192);
    req.min(n_steps)
}

pub fn eval_double_ma_curves_from_log_returns_and_mas_f64(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    bins: usize,
) -> DoubleMaCurves {
    let n_total = lr.len();
    if n_total < 2 || fast.len() != n_total || slow.len() != n_total || start_t >= n_total {
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

    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let commission = cfg.effective_commission();
    let log_comm = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let has_commission = commission > 0.0;
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
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

pub fn eval_double_ma_curves_from_log_returns_and_mas_f64_with_halftrend_filter(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
    bins: usize,
) -> DoubleMaCurves {
    let n_total = lr.len();
    if n_total < 2
        || fast.len() != n_total
        || slow.len() != n_total
        || halftrend_trend.len() != n_total
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

    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let commission = cfg.effective_commission();
    let log_comm = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let has_commission = commission > 0.0;
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
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
        sign = if !ht.is_finite() {
            0
        } else if ht < 0.5 {
            sign.max(0)
        } else {
            sign.min(0)
        };

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

fn compute_ma_series(candles: &Candles, len: usize, ma_type: &str, source: &str) -> Vec<f64> {
    let n = candles.close.len();
    ma(ma_type, MaData::Candles { candles, source }, len).unwrap_or_else(|_| vec![f64::NAN; n])
}

pub fn expand_grid(range: &DoubleMaBatchRange) -> Vec<DoubleMaParams> {
    let (f_start, f_end, f_step) = range.fast_len;
    let (s_start, s_end, s_step) = range.slow_len;

    let fast_vals: Vec<u16> = if f_step == 0 {
        vec![f_start]
    } else {
        let (lo, hi) = if f_start <= f_end {
            (f_start, f_end)
        } else {
            (f_end, f_start)
        };
        let mut out = Vec::new();
        let mut v = lo;
        loop {
            out.push(v);
            if v == hi {
                break;
            }
            match v.checked_add(f_step) {
                Some(next) if next > v && next <= hi => v = next,
                _ => break,
            }
        }
        out
    };

    let slow_vals: Vec<u16> = if s_step == 0 {
        vec![s_start]
    } else {
        let (lo, hi) = if s_start <= s_end {
            (s_start, s_end)
        } else {
            (s_end, s_start)
        };
        let mut out = Vec::new();
        let mut v = lo;
        loop {
            out.push(v);
            if v == hi {
                break;
            }
            match v.checked_add(s_step) {
                Some(next) if next > v && next <= hi => v = next,
                _ => break,
            }
        }
        out
    };

    let mut combos = Vec::new();
    for &fast_len in &fast_vals {
        for &slow_len in &slow_vals {
            if fast_len >= slow_len {
                continue;
            }
            let max_fast: u16 = if range.fast_ma_types.len() > u16::MAX as usize {
                u16::MAX
            } else {
                range.fast_ma_types.len() as u16
            };
            let max_slow: u16 = if range.slow_ma_types.len() > u16::MAX as usize {
                u16::MAX
            } else {
                range.slow_ma_types.len() as u16
            };
            for fast_ma_id in 0..max_fast {
                for slow_ma_id in 0..max_slow {
                    combos.push(DoubleMaParams {
                        fast_len,
                        slow_len,
                        fast_ma_id,
                        slow_ma_id,
                    });
                }
            }
        }
    }
    combos
}

#[derive(Clone, Debug, Default)]
pub struct DoubleMaBatchBuilder {
    range: DoubleMaBatchRange,
}

impl DoubleMaBatchBuilder {
    pub fn new() -> Self {
        Self {
            range: DoubleMaBatchRange::default(),
        }
    }

    pub fn fast_len_range(mut self, start: u16, end: u16, step: u16) -> Self {
        self.range.fast_len = (start, end, step);
        self
    }

    pub fn slow_len_range(mut self, start: u16, end: u16, step: u16) -> Self {
        self.range.slow_len = (start, end, step);
        self
    }

    pub fn fast_ma_types(mut self, types: Vec<String>) -> Self {
        self.range.fast_ma_types = types;
        self
    }

    pub fn slow_ma_types(mut self, types: Vec<String>) -> Self {
        self.range.slow_ma_types = types;
        self
    }

    pub fn range(&self) -> &DoubleMaBatchRange {
        &self.range
    }

    pub fn into_range(self) -> DoubleMaBatchRange {
        self.range
    }

    pub fn combos(&self) -> Vec<DoubleMaParams> {
        expand_grid(&self.range)
    }

    pub fn run_cpu(&self, candles: &Candles) -> Vec<Metrics> {
        let combos = self.combos();
        double_ma_batch_cpu(
            candles,
            &combos,
            &self.range.fast_ma_types,
            &self.range.slow_ma_types,
            "close",
        )
    }
}

pub fn eval_double_ma_one(candles: &Candles, params: &DoubleMaParams) -> Metrics {
    let prices = source_type(candles, "close");
    let n = prices.len();
    if n < 2 {
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

    let fast = match params.fast_ma_id {
        0 => compute_ma_series(candles, params.fast_len as usize, "sma", "close"),
        1 => compute_ma_series(candles, params.fast_len as usize, "ema", "close"),
        2 => compute_ma_series(candles, params.fast_len as usize, "alma", "close"),
        _ => vec![f64::NAN; n],
    };
    let slow = match params.slow_ma_id {
        0 => compute_ma_series(candles, params.slow_len as usize, "sma", "close"),
        1 => compute_ma_series(candles, params.slow_len as usize, "ema", "close"),
        2 => compute_ma_series(candles, params.slow_len as usize, "alma", "close"),
        _ => vec![f64::NAN; n],
    };

    let first_valid = first_finite_idx(prices).unwrap_or(0);
    eval_double_ma_from_series_cfg(
        prices,
        &fast,
        &slow,
        params.fast_len,
        params.slow_len,
        first_valid,
        &StrategyConfig::default(),
    )
}

pub fn eval_double_ma_one_with_types(
    candles: &Candles,
    params: &DoubleMaParams,
    fast_ma_types: &[String],
    slow_ma_types: &[String],
) -> Metrics {
    let prices = source_type(candles, "close");
    if prices.len() < 2 {
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

    let fast_type = fast_ma_types
        .get(params.fast_ma_id as usize)
        .map(|s| s.as_str());
    let slow_type = slow_ma_types
        .get(params.slow_ma_id as usize)
        .map(|s| s.as_str());
    let (Some(fast_type), Some(slow_type)) = (fast_type, slow_type) else {
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
    };

    let fast = compute_ma_series(candles, params.fast_len as usize, fast_type, "close");
    let slow = compute_ma_series(candles, params.slow_len as usize, slow_type, "close");

    let first_valid = first_finite_idx(prices).unwrap_or(0);
    eval_double_ma_from_series_cfg(
        prices,
        &fast,
        &slow,
        params.fast_len,
        params.slow_len,
        first_valid,
        &StrategyConfig::default(),
    )
}

pub fn double_ma_batch_cpu(
    candles: &Candles,
    combos: &[DoubleMaParams],
    fast_ma_types: &[String],
    slow_ma_types: &[String],
    ma_source: &str,
) -> Vec<Metrics> {
    if combos.is_empty() {
        return Vec::new();
    }

    let prices = source_type(candles, "close");
    let n = prices.len();
    if n < 2 {
        return combos
            .iter()
            .map(|_| Metrics {
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
            })
            .collect();
    }

    let cfg = StrategyConfig::default();
    let lr = compute_log_returns(prices);
    let first_valid = first_finite_idx(prices).unwrap_or(0);
    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };

    let matrices =
        build_ma_matrices_cpu(candles, combos, fast_ma_types, slow_ma_types, n, ma_source);
    let nan_row: Vec<f64> = vec![f64::NAN; n];

    combos
        .par_iter()
        .map(|p| {
            let fast_type = fast_ma_types.get(p.fast_ma_id as usize).map(|s| s.as_str());
            let slow_type = slow_ma_types.get(p.slow_ma_id as usize).map(|s| s.as_str());

            let (Some(fast_type), Some(slow_type)) = (fast_type, slow_type) else {
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
            };

            let fast = matrices
                .get(fast_type)
                .and_then(|m| m.row(p.fast_len, n))
                .unwrap_or(&nan_row);
            let slow = matrices
                .get(slow_type)
                .and_then(|m| m.row(p.slow_len, n))
                .unwrap_or(&nan_row);

            let max_p = (p.fast_len as usize).max(p.slow_len as usize).max(1);
            let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
            let start_t = t_valid.saturating_add(trade_offset);
            eval_double_ma_from_log_returns_and_mas_f64(&lr, fast, slow, start_t, &cfg)
        })
        .collect()
}

#[inline]
fn first_finite_idx(series: &[f64]) -> Option<usize> {
    series.iter().position(|v| v.is_finite())
}

fn compute_log_returns(prices: &[f64]) -> Vec<f64> {
    let n = prices.len();
    let mut out = vec![0.0_f64; n];
    if n < 2 {
        return out;
    }
    for t in 1..n {
        let p = prices[t];
        let pm = prices[t - 1];
        out[t] = if p > 0.0 && pm > 0.0 && p.is_finite() && pm.is_finite() {
            p.ln() - pm.ln()
        } else {
            0.0
        };
    }
    out
}

pub fn eval_double_ma_from_series_cfg(
    prices: &[f64],
    fast: &[f64],
    slow: &[f64],
    fast_period: u16,
    slow_period: u16,
    first_valid: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    if prices.len() < 2 || fast.len() != prices.len() || slow.len() != prices.len() {
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
    let lr = compute_log_returns(prices);
    let trade_offset = if cfg.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let max_p = (fast_period as usize).max(slow_period as usize).max(1);
    let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
    let start_t = t_valid.saturating_add(trade_offset);
    eval_double_ma_from_log_returns_and_mas_f64(&lr, fast, slow, start_t, cfg)
}

pub fn eval_double_ma_from_log_returns_and_mas_f64(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f64_impl::<true, false>(lr, fast, slow, start_t, cfg)
}

pub fn eval_double_ma_from_log_returns_and_mas_f64_rich(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f64_impl::<true, true>(lr, fast, slow, start_t, cfg)
}

pub fn eval_double_ma_from_log_returns_and_mas_f64_fast(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f64_impl::<false, false>(lr, fast, slow, start_t, cfg)
}

pub fn eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_impl::<true, false>(
        lr,
        fast,
        slow,
        halftrend_trend,
        start_t,
        cfg,
    )
}

pub fn eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_rich(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_impl::<true, true>(
        lr,
        fast,
        slow,
        halftrend_trend,
        start_t,
        cfg,
    )
}

pub fn eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_fast(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_impl::<false, false>(
        lr,
        fast,
        slow,
        halftrend_trend,
        start_t,
        cfg,
    )
}

fn eval_double_ma_from_log_returns_and_mas_f64_impl<
    const COMPUTE_SHARPE: bool,
    const COMPUTE_DRAWDOWN_RISK: bool,
>(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    let n_total = lr.len();
    if n_total < 2 || fast.len() != n_total || slow.len() != n_total {
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
    let mut underwater_bars: u32 = 0;
    let mut underwater_run: u32 = 0;
    let mut max_underwater_duration_bars: u32 = 0;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_neg_cu = 0.0_f64;
    let mut count_neg: u64 = 0;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;
    let mut long_obs: u64 = 0;
    let mut short_obs: u64 = 0;
    let mut long_sum_r = 0.0_f64;
    let mut short_sum_r = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;
    let mut trade_log_ret = 0.0_f64;

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

        if sign != pos {
            if pos == 0 || sign == 0 {
                if has_commission {
                    log_eq += log_comm;
                    if risk_enabled && pos != 0 {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(1);
                pos = if sign == 0 { 0 } else { sign };
                if risk_enabled {
                    trade_log_ret = if pos != 0 && has_commission {
                        log_comm
                    } else {
                        0.0
                    };
                }
            } else if !cfg.allow_flip {
                if has_commission {
                    log_eq += log_comm;
                    if risk_enabled {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(1);
                pos = 0;
                if risk_enabled {
                    trade_log_ret = 0.0;
                }
            } else {
                if has_commission {
                    log_eq += 2.0 * log_comm;
                    if risk_enabled {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(2);
                pos = sign;
                if risk_enabled {
                    trade_log_ret = if has_commission { log_comm } else { 0.0 };
                }
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
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }
        if dd_log > 0.0 {
            underwater_bars = underwater_bars.saturating_add(1);
            underwater_run = underwater_run.saturating_add(1);
            max_underwater_duration_bars = max_underwater_duration_bars.max(underwater_run);
        } else {
            underwater_run = 0;
        }
        if COMPUTE_DRAWDOWN_RISK {
            let dd = if dd_log > 0.0 {
                -(-dd_log).exp_m1()
            } else {
                0.0
            };
            ulcer_sum_sq += dd * dd;
        }

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

            let excess_r = step_r - cfg.hurdle_tau.unwrap_or(0.0);
            if excess_r < 0.0 {
                count_neg += 1;
                sum_neg_sq += excess_r * excess_r;
                let neg_mag = -excess_r;
                sum_neg_cu += neg_mag * neg_mag * neg_mag;
                sum_neg_abs_r += neg_mag;
            } else {
                sum_pos_r += excess_r;
            }
            if pos_bar > 0 {
                long_obs += 1;
                long_sum_r += step_r;
            } else if pos_bar < 0 {
                short_obs += 1;
                short_sum_r += step_r;
            }
        }

        sum_abs_pos += (pos_bar as i64).abs();
        sum_pos += pos_bar as i64;
    }

    let (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        max_dd,
        ulcer_index,
        martin_ratio,
        calmar,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
    ) = finalize_risk_metrics(
        COMPUTE_SHARPE,
        COMPUTE_DRAWDOWN_RISK,
        count,
        obs,
        cfg.hurdle_tau.unwrap_or(0.0),
        mean,
        m2,
        sum_neg_sq,
        sum_neg_cu,
        count_neg,
        sum_pos_r,
        sum_neg_abs_r,
        long_obs,
        short_obs,
        long_sum_r,
        short_sum_r,
        ulcer_sum_sq,
        max_log_dd,
        log_eq,
        underwater_bars as u64,
        max_underwater_duration_bars as u64,
    );
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
    let pnl = log_eq.exp() - 1.0;

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        calmar,
        omega,
        kappa_tau,
        volatility,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
        exposure,
        net_exposure,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
        ..Metrics::default()
    }
}

fn eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_impl<
    const COMPUTE_SHARPE: bool,
    const COMPUTE_DRAWDOWN_RISK: bool,
>(
    lr: &[f64],
    fast: &[f64],
    slow: &[f64],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    let n_total = lr.len();
    if n_total < 2
        || fast.len() != n_total
        || slow.len() != n_total
        || halftrend_trend.len() != n_total
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
    let mut underwater_bars: u32 = 0;
    let mut underwater_run: u32 = 0;
    let mut max_underwater_duration_bars: u32 = 0;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_neg_cu = 0.0_f64;
    let mut count_neg: u64 = 0;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;
    let mut long_obs: u64 = 0;
    let mut short_obs: u64 = 0;
    let mut long_sum_r = 0.0_f64;
    let mut short_sum_r = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;
    let mut trade_log_ret = 0.0_f64;

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
        sign = if !ht.is_finite() {
            0
        } else if ht < 0.5 {
            sign.max(0)
        } else {
            sign.min(0)
        };

        if sign != pos {
            if pos == 0 || sign == 0 {
                if has_commission {
                    log_eq += log_comm;
                    if risk_enabled && pos != 0 {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(1);
                pos = if sign == 0 { 0 } else { sign };
                if risk_enabled {
                    trade_log_ret = if pos != 0 && has_commission {
                        log_comm
                    } else {
                        0.0
                    };
                }
            } else if !cfg.allow_flip {
                if has_commission {
                    log_eq += log_comm;
                    if risk_enabled {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(1);
                pos = 0;
                if risk_enabled {
                    trade_log_ret = 0.0;
                }
            } else {
                if has_commission {
                    log_eq += 2.0 * log_comm;
                    if risk_enabled {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(2);
                pos = sign;
                if risk_enabled {
                    trade_log_ret = if has_commission { log_comm } else { 0.0 };
                }
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
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }
        if dd_log > 0.0 {
            underwater_bars = underwater_bars.saturating_add(1);
            underwater_run = underwater_run.saturating_add(1);
            max_underwater_duration_bars = max_underwater_duration_bars.max(underwater_run);
        } else {
            underwater_run = 0;
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

            let excess_r = step_r - cfg.hurdle_tau.unwrap_or(0.0);
            if excess_r < 0.0 {
                count_neg += 1;
                sum_neg_sq += excess_r * excess_r;
                let neg_mag = -excess_r;
                sum_neg_cu += neg_mag * neg_mag * neg_mag;
                sum_neg_abs_r += neg_mag;
            } else {
                sum_pos_r += excess_r;
            }
            if pos_bar > 0 {
                long_obs += 1;
                long_sum_r += step_r;
            } else if pos_bar < 0 {
                short_obs += 1;
                short_sum_r += step_r;
            }
        }

        sum_abs_pos += (pos_bar as i64).abs();
        sum_pos += pos_bar as i64;
    }

    let (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        max_dd,
        ulcer_index,
        martin_ratio,
        calmar,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
    ) = finalize_risk_metrics(
        COMPUTE_SHARPE,
        COMPUTE_DRAWDOWN_RISK,
        count,
        obs,
        cfg.hurdle_tau.unwrap_or(0.0),
        mean,
        m2,
        sum_neg_sq,
        sum_neg_cu,
        count_neg,
        sum_pos_r,
        sum_neg_abs_r,
        long_obs,
        short_obs,
        long_sum_r,
        short_sum_r,
        ulcer_sum_sq,
        max_log_dd,
        log_eq,
        underwater_bars as u64,
        max_underwater_duration_bars as u64,
    );
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
    let pnl = log_eq.exp() - 1.0;

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        calmar,
        omega,
        kappa_tau,
        volatility,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
        exposure,
        net_exposure,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
        ..Metrics::default()
    }
}

pub fn eval_double_ma_from_log_returns_and_mas_f32(
    lr: &[f64],
    fast: &[f32],
    slow: &[f32],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f32_impl::<true, false>(lr, fast, slow, start_t, cfg)
}

pub fn eval_double_ma_from_log_returns_and_mas_f32_rich(
    lr: &[f64],
    fast: &[f32],
    slow: &[f32],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f32_impl::<true, true>(lr, fast, slow, start_t, cfg)
}

pub fn eval_double_ma_from_log_returns_and_mas_f32_fast(
    lr: &[f64],
    fast: &[f32],
    slow: &[f32],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f32_impl::<false, false>(lr, fast, slow, start_t, cfg)
}

pub fn eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter(
    lr: &[f64],
    fast: &[f32],
    slow: &[f32],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter_impl::<true, false>(
        lr,
        fast,
        slow,
        halftrend_trend,
        start_t,
        cfg,
    )
}

pub fn eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter_rich(
    lr: &[f64],
    fast: &[f32],
    slow: &[f32],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter_impl::<true, true>(
        lr,
        fast,
        slow,
        halftrend_trend,
        start_t,
        cfg,
    )
}

pub fn eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter_fast(
    lr: &[f64],
    fast: &[f32],
    slow: &[f32],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter_impl::<false, false>(
        lr,
        fast,
        slow,
        halftrend_trend,
        start_t,
        cfg,
    )
}

fn eval_double_ma_from_log_returns_and_mas_f32_impl<
    const COMPUTE_SHARPE: bool,
    const COMPUTE_DRAWDOWN_RISK: bool,
>(
    lr: &[f64],
    fast: &[f32],
    slow: &[f32],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    let n_total = lr.len();
    if n_total < 2 || fast.len() != n_total || slow.len() != n_total {
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
    let mut underwater_bars: u32 = 0;
    let mut underwater_run: u32 = 0;
    let mut max_underwater_duration_bars: u32 = 0;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_neg_cu = 0.0_f64;
    let mut count_neg: u64 = 0;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;
    let mut long_obs: u64 = 0;
    let mut short_obs: u64 = 0;
    let mut long_sum_r = 0.0_f64;
    let mut short_sum_r = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;
    let mut trade_log_ret = 0.0_f64;

    for t in start_t..n_total {
        let t_sig = t.saturating_sub(trade_offset);
        let f = fast[t_sig] as f64;
        let s = slow[t_sig] as f64;
        let eps = if cfg.eps_rel > 0.0 {
            cfg.eps_rel * 1.0_f64.max(s.abs())
        } else {
            0.0
        };

        let mut sign: i32 = (f > s + eps) as i32 - (f < s - eps) as i32;
        if cfg.long_only && sign < 0 {
            sign = 0;
        }

        if sign != pos {
            if pos == 0 || sign == 0 {
                if has_commission {
                    log_eq += log_comm;
                    if risk_enabled && pos != 0 {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(1);
                pos = if sign == 0 { 0 } else { sign };
                if risk_enabled {
                    trade_log_ret = if pos != 0 && has_commission {
                        log_comm
                    } else {
                        0.0
                    };
                }
            } else if !cfg.allow_flip {
                if has_commission {
                    log_eq += log_comm;
                    if risk_enabled {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(1);
                pos = 0;
                if risk_enabled {
                    trade_log_ret = 0.0;
                }
            } else {
                if has_commission {
                    log_eq += 2.0 * log_comm;
                    if risk_enabled {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(2);
                pos = sign;
                if risk_enabled {
                    trade_log_ret = if has_commission { log_comm } else { 0.0 };
                }
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
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }
        if dd_log > 0.0 {
            underwater_bars = underwater_bars.saturating_add(1);
            underwater_run = underwater_run.saturating_add(1);
            max_underwater_duration_bars = max_underwater_duration_bars.max(underwater_run);
        } else {
            underwater_run = 0;
        }
        if COMPUTE_DRAWDOWN_RISK {
            let dd = if dd_log > 0.0 {
                -(-dd_log).exp_m1()
            } else {
                0.0
            };
            ulcer_sum_sq += dd * dd;
        }

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

            let excess_r = step_r - cfg.hurdle_tau.unwrap_or(0.0);
            if excess_r < 0.0 {
                count_neg += 1;
                sum_neg_sq += excess_r * excess_r;
                let neg_mag = -excess_r;
                sum_neg_cu += neg_mag * neg_mag * neg_mag;
                sum_neg_abs_r += neg_mag;
            } else {
                sum_pos_r += excess_r;
            }
            if pos_bar > 0 {
                long_obs += 1;
                long_sum_r += step_r;
            } else if pos_bar < 0 {
                short_obs += 1;
                short_sum_r += step_r;
            }
        }

        sum_abs_pos += (pos_bar as i64).abs();
        sum_pos += pos_bar as i64;
    }

    let (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        max_dd,
        ulcer_index,
        martin_ratio,
        calmar,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
    ) = finalize_risk_metrics(
        COMPUTE_SHARPE,
        COMPUTE_DRAWDOWN_RISK,
        count,
        obs,
        cfg.hurdle_tau.unwrap_or(0.0),
        mean,
        m2,
        sum_neg_sq,
        sum_neg_cu,
        count_neg,
        sum_pos_r,
        sum_neg_abs_r,
        long_obs,
        short_obs,
        long_sum_r,
        short_sum_r,
        ulcer_sum_sq,
        max_log_dd,
        log_eq,
        underwater_bars as u64,
        max_underwater_duration_bars as u64,
    );
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
    let pnl = log_eq.exp() - 1.0;

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        calmar,
        omega,
        kappa_tau,
        volatility,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
        exposure,
        net_exposure,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
        ..Metrics::default()
    }
}

fn eval_double_ma_from_log_returns_and_mas_f32_with_halftrend_filter_impl<
    const COMPUTE_SHARPE: bool,
    const COMPUTE_DRAWDOWN_RISK: bool,
>(
    lr: &[f64],
    fast: &[f32],
    slow: &[f32],
    halftrend_trend: &[f64],
    start_t: usize,
    cfg: &StrategyConfig,
) -> Metrics {
    let n_total = lr.len();
    if n_total < 2
        || fast.len() != n_total
        || slow.len() != n_total
        || halftrend_trend.len() != n_total
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
    let mut underwater_bars: u32 = 0;
    let mut underwater_run: u32 = 0;
    let mut max_underwater_duration_bars: u32 = 0;
    let mut trades: u32 = 0;

    let mut count: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_neg_cu = 0.0_f64;
    let mut count_neg: u64 = 0;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;
    let mut long_obs: u64 = 0;
    let mut short_obs: u64 = 0;
    let mut long_sum_r = 0.0_f64;
    let mut short_sum_r = 0.0_f64;

    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;
    let mut trade_log_ret = 0.0_f64;

    for t in start_t..n_total {
        let t_sig = t.saturating_sub(trade_offset);
        let f = fast[t_sig] as f64;
        let s = slow[t_sig] as f64;
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
        sign = if !ht.is_finite() {
            0
        } else if ht < 0.5 {
            sign.max(0)
        } else {
            sign.min(0)
        };

        if sign != pos {
            if pos == 0 || sign == 0 {
                if has_commission {
                    log_eq += log_comm;
                    if risk_enabled && pos != 0 {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(1);
                pos = if sign == 0 { 0 } else { sign };
                if risk_enabled {
                    trade_log_ret = if pos != 0 && has_commission {
                        log_comm
                    } else {
                        0.0
                    };
                }
            } else if !cfg.allow_flip {
                if has_commission {
                    log_eq += log_comm;
                    if risk_enabled {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(1);
                pos = 0;
                if risk_enabled {
                    trade_log_ret = 0.0;
                }
            } else {
                if has_commission {
                    log_eq += 2.0 * log_comm;
                    if risk_enabled {
                        trade_log_ret += log_comm;
                    }
                }
                trades = trades.saturating_add(2);
                pos = sign;
                if risk_enabled {
                    trade_log_ret = if has_commission { log_comm } else { 0.0 };
                }
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
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }
        if dd_log > 0.0 {
            underwater_bars = underwater_bars.saturating_add(1);
            underwater_run = underwater_run.saturating_add(1);
            max_underwater_duration_bars = max_underwater_duration_bars.max(underwater_run);
        } else {
            underwater_run = 0;
        }
        if COMPUTE_DRAWDOWN_RISK {
            let dd = if dd_log > 0.0 {
                -(-dd_log).exp_m1()
            } else {
                0.0
            };
            ulcer_sum_sq += dd * dd;
        }

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

            let excess_r = step_r - cfg.hurdle_tau.unwrap_or(0.0);
            if excess_r < 0.0 {
                count_neg += 1;
                sum_neg_sq += excess_r * excess_r;
                let neg_mag = -excess_r;
                sum_neg_cu += neg_mag * neg_mag * neg_mag;
                sum_neg_abs_r += neg_mag;
            } else {
                sum_pos_r += excess_r;
            }
            if pos_bar > 0 {
                long_obs += 1;
                long_sum_r += step_r;
            } else if pos_bar < 0 {
                short_obs += 1;
                short_sum_r += step_r;
            }
        }

        sum_abs_pos += (pos_bar as i64).abs();
        sum_pos += pos_bar as i64;
    }

    let (
        sharpe,
        volatility,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        omega,
        kappa_tau,
        max_dd,
        ulcer_index,
        martin_ratio,
        calmar,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
    ) = finalize_risk_metrics(
        COMPUTE_SHARPE,
        COMPUTE_DRAWDOWN_RISK,
        count,
        obs,
        cfg.hurdle_tau.unwrap_or(0.0),
        mean,
        m2,
        sum_neg_sq,
        sum_neg_cu,
        count_neg,
        sum_pos_r,
        sum_neg_abs_r,
        long_obs,
        short_obs,
        long_sum_r,
        short_sum_r,
        ulcer_sum_sq,
        max_log_dd,
        log_eq,
        underwater_bars as u64,
        max_underwater_duration_bars as u64,
    );
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
    let pnl = log_eq.exp() - 1.0;

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        calmar,
        omega,
        kappa_tau,
        volatility,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades,
        exposure,
        net_exposure,
        long_mean_return,
        short_mean_return,
        underwater_fraction,
        max_underwater_duration_bars,
        ..Metrics::default()
    }
}

#[derive(Clone, Debug)]
struct PeriodMatrixF64 {
    period_start: u16,
    period_end: u16,
    values: Vec<f64>,
}

impl PeriodMatrixF64 {
    fn row(&self, period: u16, cols: usize) -> Option<&[f64]> {
        if period < self.period_start || period > self.period_end {
            return None;
        }
        let row = (period - self.period_start) as usize;
        let offset = row.checked_mul(cols)?;
        let end = offset.checked_add(cols)?;
        self.values.get(offset..end)
    }
}

fn update_minmax<'a>(map: &mut HashMap<&'a str, (u16, u16)>, ma_type: &'a str, period: u16) {
    let entry = map.entry(ma_type).or_insert((period, period));
    if period < entry.0 {
        entry.0 = period;
    }
    if period > entry.1 {
        entry.1 = period;
    }
}

fn build_ma_matrices_cpu<'a>(
    candles: &'a Candles,
    combos: &[DoubleMaParams],
    fast_ma_types: &'a [String],
    slow_ma_types: &'a [String],
    series_len: usize,
    ma_source: &str,
) -> HashMap<&'a str, PeriodMatrixF64> {
    let max_period_supported: u16 = if series_len >= u16::MAX as usize {
        u16::MAX
    } else {
        series_len as u16
    };

    let mut minmax: HashMap<&str, (u16, u16)> = HashMap::new();
    for p in combos {
        if p.fast_len > 0 {
            if let Some(t) = fast_ma_types.get(p.fast_ma_id as usize) {
                update_minmax(&mut minmax, t.as_str(), p.fast_len);
            }
        }
        if p.slow_len > 0 {
            if let Some(t) = slow_ma_types.get(p.slow_ma_id as usize) {
                update_minmax(&mut minmax, t.as_str(), p.slow_len);
            }
        }
    }

    let mut out: HashMap<&str, PeriodMatrixF64> = HashMap::new();
    for (&ma_type, &(min_p, max_p)) in &minmax {
        if min_p == 0 || max_p == 0 {
            continue;
        }

        let start = min_p;
        let end = max_p.min(max_period_supported);
        if start > end {
            continue;
        }

        let n_periods = (end as u32).saturating_sub(start as u32).saturating_add(1) as usize;
        let expected_len = match n_periods.checked_mul(series_len) {
            Some(v) => v,
            None => continue,
        };

        let mut values: Vec<f64> = vec![f64::NAN; expected_len];

        let batch = ma_batch(
            ma_type,
            MaData::Candles {
                candles,
                source: ma_source,
            },
            (start as usize, end as usize, 1),
        );
        match batch {
            Ok(b) if b.cols == series_len => {
                for (src_row, &period) in b.periods.iter().enumerate() {
                    if period < start as usize || period > end as usize {
                        continue;
                    }
                    let dst_row = period - start as usize;
                    let dst_off = match dst_row.checked_mul(series_len) {
                        Some(v) => v,
                        None => continue,
                    };
                    let src_off = match src_row.checked_mul(b.cols) {
                        Some(v) => v,
                        None => continue,
                    };
                    if src_off + series_len > b.values.len() || dst_off + series_len > values.len()
                    {
                        continue;
                    }
                    values[dst_off..dst_off + series_len]
                        .copy_from_slice(&b.values[src_off..src_off + series_len]);
                }
            }
            _ => {
                for (row, period) in (start..=end).enumerate() {
                    let series = compute_ma_series(candles, period as usize, ma_type, ma_source);
                    if series.len() != series_len {
                        continue;
                    }
                    let off = match row.checked_mul(series_len) {
                        Some(v) => v,
                        None => continue,
                    };
                    if off + series_len > values.len() {
                        continue;
                    }
                    values[off..off + series_len].copy_from_slice(&series);
                }
            }
        }

        out.insert(
            ma_type,
            PeriodMatrixF64 {
                period_start: start,
                period_end: end,
                values,
            },
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use vector_ta::utilities::data_loader::Candles;

    fn make_trending_candles(len: usize) -> Candles {
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

        Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn expand_grid_enforces_fast_lt_slow() {
        let range = DoubleMaBatchRange {
            fast_len: (5, 10, 5),
            slow_len: (8, 12, 2),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
        };
        let combos = expand_grid(&range);
        assert!(!combos.is_empty());
        for p in combos {
            assert!(p.fast_len < p.slow_len);
        }
    }

    #[test]
    fn batch_builder_runs_cpu() {
        let candles = make_trending_candles(200);
        let builder = DoubleMaBatchBuilder::new()
            .fast_len_range(5, 5, 0)
            .slow_len_range(20, 20, 0)
            .fast_ma_types(vec!["sma".to_string()])
            .slow_ma_types(vec!["sma".to_string()]);

        let metrics = builder.run_cpu(&candles);
        assert_eq!(metrics.len(), 1);

        let m = &metrics[0];
        assert!(m.max_dd >= 0.0);
    }

    #[test]
    fn effective_commission_uses_bps_when_commission_is_zero() {
        let cfg = StrategyConfig {
            commission: 0.0,
            fee_bps: 2.0,
            spread_bps: 4.0,
            slippage_bps: 1.0,
            ..StrategyConfig::default()
        };
        let expected = (2.0 + 1.0 + 2.0) / 10_000.0;
        assert!((cfg.effective_commission() - expected).abs() < 1e-12);
    }

    #[test]
    fn higher_costs_do_not_improve_pnl_in_same_setup() {
        let n = 256usize;
        let prices: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.25).collect();
        let lr = compute_log_returns(&prices);
        let fast: Vec<f64> = vec![2.0; n];
        let slow: Vec<f64> = vec![1.0; n];

        let zero = StrategyConfig {
            commission: 0.0,
            fee_bps: 0.0,
            spread_bps: 0.0,
            slippage_bps: 0.0,
            ..StrategyConfig::default()
        };
        let medium = StrategyConfig {
            commission: 0.0,
            fee_bps: 0.5,
            spread_bps: 1.0,
            slippage_bps: 0.5,
            ..StrategyConfig::default()
        };
        let high = StrategyConfig {
            commission: 0.0,
            fee_bps: 2.0,
            spread_bps: 4.0,
            slippage_bps: 2.0,
            ..StrategyConfig::default()
        };

        let m0 = eval_double_ma_from_log_returns_and_mas_f64(&lr, &fast, &slow, 1, &zero);
        let m1 = eval_double_ma_from_log_returns_and_mas_f64(&lr, &fast, &slow, 1, &medium);
        let m2 = eval_double_ma_from_log_returns_and_mas_f64(&lr, &fast, &slow, 1, &high);

        assert!(m1.pnl <= m0.pnl + 1e-12);
        assert!(m2.pnl <= m1.pnl + 1e-12);
    }

    #[test]
    fn derived_risk_metrics_are_populated_for_basic_run() {
        let candles = make_trending_candles(200);
        let params = DoubleMaParams {
            fast_len: 5,
            slow_len: 20,
            fast_ma_id: 0,
            slow_ma_id: 0,
        };
        let metrics = eval_double_ma_one(&candles, &params);

        assert!(metrics.downside_deviation.is_finite());
        assert!(metrics.downside_deviation >= 0.0);
        assert!(metrics.downside_frequency.is_finite());
        assert!((0.0..=1.0).contains(&metrics.downside_frequency));
        assert!(metrics.downside_severity.is_finite());
        assert!(metrics.downside_severity >= 0.0);
        assert!(metrics.ulcer_index.is_finite());
        assert!(metrics.ulcer_index >= 0.0);
        assert!(metrics.martin_ratio.is_finite());
        assert!(metrics.kappa_tau.is_finite());
        assert!(metrics.long_mean_return.is_finite());
        assert!(metrics.short_mean_return.is_finite());
        assert!((0.0..=1.0).contains(&metrics.underwater_fraction));
        assert!(metrics.max_underwater_duration_bars <= 200);
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

        let m0 = eval_double_ma_from_log_returns_and_mas_f64(&lr, &fast, &slow, 1, &base);
        let m1 = eval_double_ma_from_log_returns_and_mas_f64(&lr, &fast, &slow, 1, &risk);

        assert!(m1.trades > m0.trades);
        assert!(m1.pnl <= m0.pnl + 1e-12);
    }

    #[test]
    fn curves_emit_closed_trades_when_tp_sl_hits() {
        let n = 128usize;
        let prices: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.30).collect();
        let lr = compute_log_returns(&prices);
        let fast: Vec<f64> = vec![2.0; n];
        let slow: Vec<f64> = vec![1.0; n];

        let cfg = StrategyConfig {
            commission: 0.0,
            fee_bps: 1.0,
            spread_bps: 0.0,
            slippage_bps: 0.0,
            stop_loss_pct: None,
            take_profit_pct: Some(0.015),
            ..StrategyConfig::default()
        };

        let curves =
            eval_double_ma_curves_from_log_returns_and_mas_f64(&lr, &fast, &slow, 1, &cfg, 64);
        let closed = curves.trades.iter().filter(|t| !t.open).count();
        assert!(closed > 0);
    }
}
