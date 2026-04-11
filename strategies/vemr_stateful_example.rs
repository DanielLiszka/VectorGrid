use std::error::Error;
use std::fmt;

/// Stateful VEMR example that turns plain indicator alignment into a small,
/// grid-friendly trade lifecycle:
///
/// Flat -> Armed(Long/Short) -> InPosition -> Cooldown -> Flat
///
/// The design goal is to stay cheap enough for optimizer use:
/// - no dynamic dispatch
/// - no heap allocation inside the loop
/// - only a few comparisons and counters per bar
/// - no extra indicators beyond the existing VEMR ingredients
///
/// This module is intended as a repo-shaped implementation example that can be
/// folded into crates/app_core/src/vemr.rs and later mirrored in the local GPU
/// kernel if desired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VemrSideMode {
    Both,
    LongOnly,
    ShortOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VemrSignalMode {
    /// Existing/simple behavior: enter whenever the raw entry condition is true.
    Classic,
    /// More realistic behavior: first arm on an extreme move, then only enter
    /// after a reclaim / improvement event. This adds hysteresis, setup expiry,
    /// and cooldown while remaining GPU-friendly.
    ArmReclaim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VemrTraceState {
    Flat = 0,
    ArmedLong = 1,
    ArmedShort = 2,
    Long = 3,
    Short = 4,
    Cooldown = 5,
}

#[derive(Debug, Clone, Copy)]
pub struct VemrLifecycleConfig {
    pub signal_mode: VemrSignalMode,
    pub side_mode: VemrSideMode,
    /// Arm only after a more extreme move than the actual entry trigger.
    /// Example: z_arm = 2.50, z_entry = 1.75.
    pub z_arm: f64,
    /// Entry threshold magnitude. Long reclaim requires z >= -z_entry,
    /// short reclaim requires z <= z_entry.
    pub z_entry: f64,
    /// Exit threshold magnitude. Long exits when z >= -z_exit or reaches the
    /// Bollinger mid. Short exits when z <= z_exit or reaches the Bollinger mid.
    pub z_exit: f64,
    pub atr_stop_mult: f64,
    pub require_band_touch: bool,
    /// Number of consecutive trigger bars required after arming.
    pub confirm_bars: usize,
    /// Maximum bars allowed between arm and entry trigger.
    pub setup_expiry_bars: usize,
    /// Bars to stay flat after an exit.
    pub cooldown_bars: usize,
    /// Zero means unlimited hold time.
    pub max_hold_bars: usize,
    pub allow_flip: bool,
    /// If true, shift the resulting desired position series by one bar.
    /// Keep this aligned with the rest of your app's order/fill semantics.
    pub trade_on_next_bar: bool,
}

#[derive(Debug, Clone)]
pub struct VemrLifecycleResult {
    /// Position series after optional next-bar shifting.
    pub desired_positions: Vec<i8>,
    /// Raw state-machine output before optional next-bar shifting.
    pub raw_desired_positions: Vec<i8>,
    /// Useful for debugging, parity tests, and visual overlays.
    pub state_trace: Vec<VemrTraceState>,
    /// Stop price trace for inspection. NaN when flat / armed / cooldown.
    pub stop_trace: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VemrLifecycleError {
    EmptySeries,
    LengthMismatch,
    InvalidThresholdOrder,
    InvalidConfirmBars,
}

impl fmt::Display for VemrLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySeries => write!(f, "input series must not be empty"),
            Self::LengthMismatch => write!(f, "all input series must have the same length"),
            Self::InvalidThresholdOrder => write!(
                f,
                "expected z_arm >= z_entry >= z_exit >= 0 and atr_stop_mult >= 0"
            ),
            Self::InvalidConfirmBars => write!(f, "confirm_bars must be >= 1"),
        }
    }
}

impl Error for VemrLifecycleError {}

pub fn build_vemr_stateful_desired_positions(
    cfg: VemrLifecycleConfig,
    close: &[f64],
    zscore: &[f64],
    bb_lower: &[f64],
    bb_mid: &[f64],
    bb_upper: &[f64],
    atr: &[f64],
) -> Result<VemrLifecycleResult, VemrLifecycleError> {
    validate_inputs(cfg, close, zscore, bb_lower, bb_mid, bb_upper, atr)?;

    let n = close.len();
    let mut raw = vec![0_i8; n];
    let mut trace = vec![VemrTraceState::Flat; n];
    let mut stop_trace = vec![f64::NAN; n];

    let mut state = VemrTraceState::Flat;
    let mut setup_age = 0_usize;
    let mut trigger_streak = 0_usize;
    let mut cooldown_left = 0_usize;
    let mut bars_in_trade = 0_usize;
    let mut entry_price = 0.0_f64;
    let mut stop_price = f64::NAN;

    let has_long = !matches!(cfg.side_mode, VemrSideMode::ShortOnly);
    let has_short = !matches!(cfg.side_mode, VemrSideMode::LongOnly);

    for t in 0..n {
        if cooldown_left > 0 {
            cooldown_left -= 1;
            state = if cooldown_left == 0 {
                VemrTraceState::Flat
            } else {
                VemrTraceState::Cooldown
            };
        }

        let prev_close = if t > 0 { close[t - 1] } else { close[t] };

        let long_arm = has_long
            && zscore[t] <= -cfg.z_arm
            && (!cfg.require_band_touch || close[t] <= bb_lower[t]);
        let short_arm = has_short
            && zscore[t] >= cfg.z_arm
            && (!cfg.require_band_touch || close[t] >= bb_upper[t]);

        let long_reclaim_trigger = has_long
            && zscore[t] >= -cfg.z_entry
            && close[t] > prev_close
            && close[t] > bb_lower[t]
            && close[t] < bb_mid[t];
        let short_reclaim_trigger = has_short
            && zscore[t] <= cfg.z_entry
            && close[t] < prev_close
            && close[t] < bb_upper[t]
            && close[t] > bb_mid[t];

        match cfg.signal_mode {
            VemrSignalMode::Classic => {
                let want_long = long_reclaim_trigger;
                let want_short = short_reclaim_trigger;
                let prior_pos = if t > 0 { raw[t - 1] } else { 0 };
                raw[t] = match (want_long, want_short, prior_pos) {
                    (true, false, _) => 1,
                    (false, true, _) => -1,
                    (true, true, 1) => 1,
                    (true, true, -1) => -1,
                    (true, true, _) => 0,
                    _ => prior_pos,
                };
                trace[t] = if raw[t] > 0 {
                    VemrTraceState::Long
                } else if raw[t] < 0 {
                    VemrTraceState::Short
                } else {
                    VemrTraceState::Flat
                };
                continue;
            }
            VemrSignalMode::ArmReclaim => {
                match state {
                    VemrTraceState::Flat => {
                        raw[t] = 0;
                        stop_price = f64::NAN;
                        if long_arm {
                            state = VemrTraceState::ArmedLong;
                            setup_age = 0;
                            trigger_streak = 0;
                        } else if short_arm {
                            state = VemrTraceState::ArmedShort;
                            setup_age = 0;
                            trigger_streak = 0;
                        }
                    }
                    VemrTraceState::ArmedLong => {
                        raw[t] = 0;
                        stop_price = f64::NAN;
                        setup_age += 1;

                        if short_arm {
                            state = VemrTraceState::ArmedShort;
                            setup_age = 0;
                            trigger_streak = 0;
                        } else if cfg.setup_expiry_bars > 0 && setup_age > cfg.setup_expiry_bars {
                            state = VemrTraceState::Flat;
                            setup_age = 0;
                            trigger_streak = 0;
                        } else if close[t] >= bb_mid[t] || zscore[t] > 0.0 {
                            // Reversion already happened without a clean entry.
                            state = VemrTraceState::Flat;
                            setup_age = 0;
                            trigger_streak = 0;
                        } else {
                            trigger_streak = if long_reclaim_trigger {
                                trigger_streak + 1
                            } else {
                                0
                            };
                            if trigger_streak >= cfg.confirm_bars {
                                state = VemrTraceState::Long;
                                entry_price = close[t];
                                stop_price = entry_price - cfg.atr_stop_mult * atr[t].max(f64::EPSILON);
                                bars_in_trade = 0;
                                raw[t] = 1;
                            }
                        }
                    }
                    VemrTraceState::ArmedShort => {
                        raw[t] = 0;
                        stop_price = f64::NAN;
                        setup_age += 1;

                        if long_arm {
                            state = VemrTraceState::ArmedLong;
                            setup_age = 0;
                            trigger_streak = 0;
                        } else if cfg.setup_expiry_bars > 0 && setup_age > cfg.setup_expiry_bars {
                            state = VemrTraceState::Flat;
                            setup_age = 0;
                            trigger_streak = 0;
                        } else if close[t] <= bb_mid[t] || zscore[t] < 0.0 {
                            state = VemrTraceState::Flat;
                            setup_age = 0;
                            trigger_streak = 0;
                        } else {
                            trigger_streak = if short_reclaim_trigger {
                                trigger_streak + 1
                            } else {
                                0
                            };
                            if trigger_streak >= cfg.confirm_bars {
                                state = VemrTraceState::Short;
                                entry_price = close[t];
                                stop_price = entry_price + cfg.atr_stop_mult * atr[t].max(f64::EPSILON);
                                bars_in_trade = 0;
                                raw[t] = -1;
                            }
                        }
                    }
                    VemrTraceState::Long => {
                        bars_in_trade += 1;
                        let mean_exit = zscore[t] >= -cfg.z_exit || close[t] >= bb_mid[t];
                        let stop_exit = close[t] <= stop_price;
                        let timed_exit = cfg.max_hold_bars > 0 && bars_in_trade >= cfg.max_hold_bars;
                        let flip_arm = cfg.allow_flip && short_arm;

                        if stop_exit || mean_exit || timed_exit || flip_arm {
                            raw[t] = 0;
                            if flip_arm {
                                state = VemrTraceState::ArmedShort;
                                setup_age = 0;
                                trigger_streak = 0;
                                stop_price = f64::NAN;
                            } else if cfg.cooldown_bars > 0 {
                                cooldown_left = cfg.cooldown_bars;
                                state = VemrTraceState::Cooldown;
                                stop_price = f64::NAN;
                            } else {
                                state = VemrTraceState::Flat;
                                stop_price = f64::NAN;
                            }
                        } else {
                            raw[t] = 1;
                        }
                    }
                    VemrTraceState::Short => {
                        bars_in_trade += 1;
                        let mean_exit = zscore[t] <= cfg.z_exit || close[t] <= bb_mid[t];
                        let stop_exit = close[t] >= stop_price;
                        let timed_exit = cfg.max_hold_bars > 0 && bars_in_trade >= cfg.max_hold_bars;
                        let flip_arm = cfg.allow_flip && long_arm;

                        if stop_exit || mean_exit || timed_exit || flip_arm {
                            raw[t] = 0;
                            if flip_arm {
                                state = VemrTraceState::ArmedLong;
                                setup_age = 0;
                                trigger_streak = 0;
                                stop_price = f64::NAN;
                            } else if cfg.cooldown_bars > 0 {
                                cooldown_left = cfg.cooldown_bars;
                                state = VemrTraceState::Cooldown;
                                stop_price = f64::NAN;
                            } else {
                                state = VemrTraceState::Flat;
                                stop_price = f64::NAN;
                            }
                        } else {
                            raw[t] = -1;
                        }
                    }
                    VemrTraceState::Cooldown => {
                        raw[t] = 0;
                        stop_price = f64::NAN;
                    }
                }
            }
        }

        trace[t] = state;
        stop_trace[t] = stop_price;
    }

    let desired_positions = if cfg.trade_on_next_bar {
        shift_one_bar(&raw)
    } else {
        raw.clone()
    };

    Ok(VemrLifecycleResult {
        desired_positions,
        raw_desired_positions: raw,
        state_trace: trace,
        stop_trace,
    })
}

fn validate_inputs(
    cfg: VemrLifecycleConfig,
    close: &[f64],
    zscore: &[f64],
    bb_lower: &[f64],
    bb_mid: &[f64],
    bb_upper: &[f64],
    atr: &[f64],
) -> Result<(), VemrLifecycleError> {
    let n = close.len();
    if n == 0 {
        return Err(VemrLifecycleError::EmptySeries);
    }
    if zscore.len() != n
        || bb_lower.len() != n
        || bb_mid.len() != n
        || bb_upper.len() != n
        || atr.len() != n
    {
        return Err(VemrLifecycleError::LengthMismatch);
    }
    if cfg.confirm_bars == 0 {
        return Err(VemrLifecycleError::InvalidConfirmBars);
    }
    if cfg.z_arm < cfg.z_entry
        || cfg.z_entry < cfg.z_exit
        || cfg.z_exit < 0.0
        || cfg.atr_stop_mult < 0.0
    {
        return Err(VemrLifecycleError::InvalidThresholdOrder);
    }
    Ok(())
}

fn shift_one_bar(src: &[i8]) -> Vec<i8> {
    if src.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0_i8; src.len()];
    out[0] = 0;
    for i in 1..src.len() {
        out[i] = src[i - 1];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cfg() -> VemrLifecycleConfig {
        VemrLifecycleConfig {
            signal_mode: VemrSignalMode::ArmReclaim,
            side_mode: VemrSideMode::LongOnly,
            z_arm: 2.50,
            z_entry: 1.75,
            z_exit: 0.25,
            atr_stop_mult: 2.0,
            require_band_touch: true,
            confirm_bars: 1,
            setup_expiry_bars: 4,
            cooldown_bars: 2,
            max_hold_bars: 0,
            allow_flip: false,
            trade_on_next_bar: false,
        }
    }

    #[test]
    fn long_lifecycle_requires_arm_then_reclaim() {
        let close = vec![100.0, 96.0, 94.0, 95.0, 97.0, 100.0];
        let z = vec![0.0, -2.7, -2.6, -1.5, -0.4, 0.1];
        let lower = vec![98.0, 97.0, 95.0, 94.5, 95.0, 96.0];
        let mid = vec![100.0; 6];
        let upper = vec![102.0; 6];
        let atr = vec![1.0; 6];

        let out = build_vemr_stateful_desired_positions(base_cfg(), &close, &z, &lower, &mid, &upper, &atr)
            .expect("stateful VEMR should evaluate");

        assert_eq!(out.raw_desired_positions, vec![0, 0, 0, 1, 1, 0]);
        assert_eq!(out.state_trace[1], VemrTraceState::ArmedLong);
        assert_eq!(out.state_trace[3], VemrTraceState::Long);
    }

    #[test]
    fn trade_on_next_bar_shifts_positions() {
        let mut cfg = base_cfg();
        cfg.trade_on_next_bar = true;

        let close = vec![100.0, 96.0, 94.0, 95.0, 97.0, 100.0];
        let z = vec![0.0, -2.7, -2.6, -1.5, -0.4, 0.1];
        let lower = vec![98.0, 97.0, 95.0, 94.5, 95.0, 96.0];
        let mid = vec![100.0; 6];
        let upper = vec![102.0; 6];
        let atr = vec![1.0; 6];

        let out = build_vemr_stateful_desired_positions(cfg, &close, &z, &lower, &mid, &upper, &atr)
            .expect("stateful VEMR should evaluate");

        assert_eq!(out.raw_desired_positions, vec![0, 0, 0, 1, 1, 0]);
        assert_eq!(out.desired_positions, vec![0, 0, 0, 0, 1, 1]);
    }

    #[test]
    fn cooldown_blocks_immediate_reentry() {
        let close = vec![100.0, 96.0, 94.0, 95.0, 100.0, 95.0, 94.0, 96.0, 99.0];
        let z = vec![0.0, -2.7, -2.6, -1.5, 0.1, -2.6, -2.4, -1.4, 0.0];
        let lower = vec![98.0, 97.0, 95.0, 94.5, 96.0, 96.0, 95.0, 94.5, 96.0];
        let mid = vec![100.0; 9];
        let upper = vec![102.0; 9];
        let atr = vec![1.0; 9];

        let out = build_vemr_stateful_desired_positions(base_cfg(), &close, &z, &lower, &mid, &upper, &atr)
            .expect("stateful VEMR should evaluate");

        // First long occurs, exits, then the second oversold patch is ignored by the cooldown.
        assert_eq!(out.raw_desired_positions, vec![0, 0, 0, 1, 0, 0, 0, 0, 0]);
        assert_eq!(out.state_trace[5], VemrTraceState::Cooldown);
    }
}
