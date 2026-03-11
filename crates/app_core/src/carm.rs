use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorComputeOutput, DynamicIndicatorSeries,
};
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, IndicatorParamKV, IndicatorParamValue,
};
use serde::{Deserialize, Serialize};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CarmStrategyParams {
    #[serde(default = "default_carm_coppock_long_period")]
    pub coppock_long_period: u16,
    #[serde(default = "default_carm_coppock_short_period")]
    pub coppock_short_period: u16,
    #[serde(default = "default_carm_coppock_wma_period")]
    pub coppock_wma_period: u16,
    #[serde(default = "default_carm_ao_fast_period")]
    pub ao_fast_period: u16,
    #[serde(default = "default_carm_ao_slow_period")]
    pub ao_slow_period: u16,
    #[serde(default = "default_carm_rocr_period")]
    pub rocr_period: u16,
    #[serde(default = "default_carm_apo_fast_period")]
    pub apo_fast_period: u16,
    #[serde(default = "default_carm_apo_slow_period")]
    pub apo_slow_period: u16,
    #[serde(default = "default_carm_otto_period")]
    pub otto_period: u16,
    #[serde(default = "default_carm_otto_threshold")]
    pub otto_threshold: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct CarmResolvedPeriods {
    pub coppock_short: u16,
    pub coppock_long: u16,
    pub coppock_wma: u16,
    pub ao_fast: u16,
    pub ao_slow: u16,
    pub rocr: u16,
    pub apo_fast: u16,
    pub apo_slow: u16,
    pub otto: u16,
}

#[derive(Clone, Debug)]
pub struct CarmSignals {
    pub coppock: Vec<f64>,
    pub ao: Vec<f64>,
    pub rocr: Vec<f64>,
    pub apo: Vec<f64>,
    pub otto_hott: Vec<f64>,
    pub otto_lott: Vec<f64>,
    pub first_valid: usize,
}

fn default_carm_coppock_long_period() -> u16 {
    14
}

fn default_carm_coppock_short_period() -> u16 {
    11
}

fn default_carm_coppock_wma_period() -> u16 {
    10
}

fn default_carm_ao_fast_period() -> u16 {
    5
}

fn default_carm_ao_slow_period() -> u16 {
    34
}

fn default_carm_rocr_period() -> u16 {
    10
}

fn default_carm_apo_fast_period() -> u16 {
    12
}

fn default_carm_apo_slow_period() -> u16 {
    26
}

fn default_carm_otto_period() -> u16 {
    20
}

fn default_carm_otto_threshold() -> f64 {
    0.0
}

impl Default for CarmStrategyParams {
    fn default() -> Self {
        Self {
            coppock_long_period: default_carm_coppock_long_period(),
            coppock_short_period: default_carm_coppock_short_period(),
            coppock_wma_period: default_carm_coppock_wma_period(),
            ao_fast_period: default_carm_ao_fast_period(),
            ao_slow_period: default_carm_ao_slow_period(),
            rocr_period: default_carm_rocr_period(),
            apo_fast_period: default_carm_apo_fast_period(),
            apo_slow_period: default_carm_apo_slow_period(),
            otto_period: default_carm_otto_period(),
            otto_threshold: default_carm_otto_threshold(),
        }
    }
}

pub fn validate_carm_params(cfg: &CarmStrategyParams) -> Result<(), String> {
    if cfg.coppock_short_period == 0 || cfg.coppock_long_period == 0 || cfg.coppock_wma_period == 0
    {
        return Err("CARM Coppock periods must be >= 1.".to_string());
    }
    if cfg.coppock_short_period >= cfg.coppock_long_period {
        return Err("CARM coppock_short_period must be < coppock_long_period.".to_string());
    }
    if cfg.ao_fast_period == 0 || cfg.ao_slow_period == 0 {
        return Err("CARM AO periods must be >= 1.".to_string());
    }
    if cfg.ao_fast_period >= cfg.ao_slow_period {
        return Err("CARM ao_fast_period must be < ao_slow_period.".to_string());
    }
    if cfg.rocr_period == 0 {
        return Err("CARM rocr_period must be >= 1.".to_string());
    }
    if cfg.apo_fast_period == 0 || cfg.apo_slow_period == 0 {
        return Err("CARM APO periods must be >= 1.".to_string());
    }
    if cfg.apo_fast_period >= cfg.apo_slow_period {
        return Err("CARM apo_fast_period must be < apo_slow_period.".to_string());
    }
    if cfg.otto_period == 0 {
        return Err("CARM otto_period must be >= 1.".to_string());
    }
    if !cfg.otto_threshold.is_finite() {
        return Err("CARM otto_threshold must be finite.".to_string());
    }
    Ok(())
}

pub fn resolve_carm_periods(
    cfg: &CarmStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> CarmResolvedPeriods {
    CarmResolvedPeriods {
        coppock_short: fast_len.max(cfg.coppock_short_period).max(1),
        coppock_long: slow_len
            .max(cfg.coppock_long_period)
            .max(fast_len.saturating_add(1)),
        coppock_wma: fast_len.max(cfg.coppock_wma_period).max(1),
        ao_fast: fast_len.max(cfg.ao_fast_period).max(1),
        ao_slow: slow_len
            .max(cfg.ao_slow_period)
            .max(fast_len.saturating_add(1)),
        rocr: fast_len.max(cfg.rocr_period).max(1),
        apo_fast: fast_len.max(cfg.apo_fast_period).max(1),
        apo_slow: slow_len
            .max(cfg.apo_slow_period)
            .max(fast_len.saturating_add(1)),
        otto: slow_len.max(cfg.otto_period).max(1),
    }
}

fn first_finite_idx_local(values: &[f64]) -> Option<usize> {
    values.iter().position(|v| v.is_finite())
}

fn series_to_f64(out: DynamicIndicatorComputeOutput) -> Vec<f64> {
    match out.series {
        DynamicIndicatorSeries::F64(v) => v,
        DynamicIndicatorSeries::I32(v) => v.into_iter().map(|x| x as f64).collect(),
        DynamicIndicatorSeries::Bool(v) => {
            v.into_iter().map(|x| if x { 1.0 } else { 0.0 }).collect()
        }
    }
}

fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("CARM indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "CARM indicator '{indicator_id}' has no declared outputs"
        ));
    }

    for preferred in preferred_ids {
        if let Some(found) = descriptor
            .outputs
            .iter()
            .find(|out| out.id.eq_ignore_ascii_case(preferred))
        {
            return Ok(found.id.clone());
        }
    }

    let preferred_lower: Vec<String> = preferred_ids
        .iter()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    for out in &descriptor.outputs {
        let out_id = out.id.to_ascii_lowercase();
        if preferred_lower
            .iter()
            .any(|p| out_id.contains(p) || p.contains(out_id.as_str()))
        {
            return Ok(out.id.clone());
        }
    }

    Ok(descriptor.outputs[0].id.clone())
}

fn compute_indicator_f64(
    candles: &Candles,
    ma_source: &str,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
) -> Result<Vec<f64>, String> {
    let out = compute_dynamic_indicator_cpu(
        candles,
        ma_source,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
    )
    .map_err(|e| e.to_string())?;
    Ok(series_to_f64(out))
}

pub fn compute_carm_signals(
    candles: &Candles,
    ma_source: &str,
    periods: CarmResolvedPeriods,
    cfg: &CarmStrategyParams,
) -> Result<CarmSignals, String> {
    let coppock_output = resolve_output_id("coppock", &["value"])?;
    let ao_output = resolve_output_id("ao", &["value"])?;
    let rocr_output = resolve_output_id("rocr", &["value"])?;
    let apo_output = resolve_output_id("apo", &["value"])?;
    let otto_hott_output = resolve_output_id("otto", &["hott"])?;
    let otto_lott_output = resolve_output_id("otto", &["lott"])?;

    let coppock_params = vec![
        IndicatorParamKV {
            key: "short_roc_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.coppock_short)),
        },
        IndicatorParamKV {
            key: "long_roc_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.coppock_long)),
        },
        IndicatorParamKV {
            key: "ma_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.coppock_wma)),
        },
    ];
    let ao_params = vec![
        IndicatorParamKV {
            key: "short_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.ao_fast)),
        },
        IndicatorParamKV {
            key: "long_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.ao_slow)),
        },
    ];
    let rocr_params = vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(periods.rocr)),
    }];
    let apo_params = vec![
        IndicatorParamKV {
            key: "short_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.apo_fast)),
        },
        IndicatorParamKV {
            key: "long_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.apo_slow)),
        },
    ];
    let otto_params = vec![
        IndicatorParamKV {
            key: "ott_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.otto)),
        },
        IndicatorParamKV {
            key: "ott_percent".to_string(),
            value: IndicatorParamValue::Float(cfg.otto_threshold.abs().max(0.6)),
        },
        IndicatorParamKV {
            key: "fast_vidya_length".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.coppock_short.max(2))),
        },
        IndicatorParamKV {
            key: "slow_vidya_length".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.ao_slow.max(periods.otto))),
        },
        IndicatorParamKV {
            key: "correcting_constant".to_string(),
            value: IndicatorParamValue::Float(100000.0),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString("VAR".to_string()),
        },
    ];

    let coppock = compute_indicator_f64(
        candles,
        ma_source,
        "coppock",
        coppock_output.as_str(),
        coppock_params.as_slice(),
    )?;
    let ao = compute_indicator_f64(
        candles,
        ma_source,
        "ao",
        ao_output.as_str(),
        ao_params.as_slice(),
    )?;
    let rocr = compute_indicator_f64(
        candles,
        ma_source,
        "rocr",
        rocr_output.as_str(),
        rocr_params.as_slice(),
    )?;
    let apo = compute_indicator_f64(
        candles,
        ma_source,
        "apo",
        apo_output.as_str(),
        apo_params.as_slice(),
    )?;
    let otto_hott = compute_indicator_f64(
        candles,
        ma_source,
        "otto",
        otto_hott_output.as_str(),
        otto_params.as_slice(),
    )?;
    let otto_lott = compute_indicator_f64(
        candles,
        ma_source,
        "otto",
        otto_lott_output.as_str(),
        otto_params.as_slice(),
    )?;

    let first_valid = first_finite_idx_local(&coppock)
        .zip(first_finite_idx_local(&ao))
        .zip(first_finite_idx_local(&rocr))
        .zip(first_finite_idx_local(&apo))
        .zip(first_finite_idx_local(&otto_hott))
        .zip(first_finite_idx_local(&otto_lott))
        .map(|(((((a, b), c), d), e), f)| a.max(b).max(c).max(d).max(e).max(f))
        .ok_or_else(|| "CARM signals are all non-finite".to_string())?;

    Ok(CarmSignals {
        coppock,
        ao,
        rocr,
        apo,
        otto_hott,
        otto_lott,
        first_valid,
    })
}

pub fn compute_carm_start_t(
    first_valid_close: usize,
    first_valid_signals: usize,
    trade_on_next_bar: bool,
) -> usize {
    let first_valid = first_valid_close.max(first_valid_signals).max(1);
    if trade_on_next_bar {
        first_valid.saturating_add(1)
    } else {
        first_valid
    }
}

fn otto_regime_spread(close: f64, hott: f64, lott: f64) -> f64 {
    if !close.is_finite() || !hott.is_finite() || !lott.is_finite() {
        return f64::NAN;
    }
    let denom = close.abs().max(1.0e-9);
    (lott - hott) / denom
}

pub fn build_carm_desired_positions(
    close: &[f64],
    signals: &CarmSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &CarmStrategyParams,
) -> Vec<i32> {
    let n = close.len();
    let mut desired = vec![0i32; n];
    if start_t >= n {
        return desired;
    }

    let trade_offset = if strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let mut pos = 0i32;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close.get(sig_t).copied().unwrap_or(f64::NAN);
        let coppock_now = signals.coppock.get(sig_t).copied().unwrap_or(f64::NAN);
        let coppock_prev = signals.coppock.get(prev_t).copied().unwrap_or(f64::NAN);
        let ao_now = signals.ao.get(sig_t).copied().unwrap_or(f64::NAN);
        let rocr_now = signals.rocr.get(sig_t).copied().unwrap_or(f64::NAN);
        let apo_now = signals.apo.get(sig_t).copied().unwrap_or(f64::NAN);
        let hott_now = signals.otto_hott.get(sig_t).copied().unwrap_or(f64::NAN);
        let lott_now = signals.otto_lott.get(sig_t).copied().unwrap_or(f64::NAN);

        let coppock_up = coppock_now.is_finite()
            && coppock_prev.is_finite()
            && coppock_now > 0.0
            && coppock_now > coppock_prev;
        let coppock_down = coppock_now.is_finite()
            && coppock_prev.is_finite()
            && coppock_now < 0.0
            && coppock_now < coppock_prev;
        let ao_up = ao_now.is_finite() && ao_now > 0.0;
        let ao_down = ao_now.is_finite() && ao_now < 0.0;
        let rocr_up = rocr_now.is_finite() && rocr_now > 1.0;
        let rocr_down = rocr_now.is_finite() && rocr_now < 1.0;
        let apo_up = apo_now.is_finite() && apo_now > 0.0;
        let apo_down = apo_now.is_finite() && apo_now < 0.0;
        let otto_spread = otto_regime_spread(close_now, hott_now, lott_now);
        let otto_up = otto_spread.is_finite() && otto_spread >= cfg.otto_threshold;
        let otto_down = otto_spread.is_finite() && otto_spread <= -cfg.otto_threshold;

        let long_entry = coppock_up && ao_up && rocr_up && apo_up && otto_up;
        let short_entry =
            !strategy.long_only && coppock_down && ao_down && rocr_down && apo_down && otto_down;

        if pos == 0 {
            if long_entry {
                pos = 1;
            } else if short_entry {
                pos = -1;
            }
        } else if pos > 0 {
            let opposing_votes = usize::from(coppock_down)
                + usize::from(ao_down)
                + usize::from(rocr_down)
                + usize::from(apo_down)
                + usize::from(otto_down);
            let mut exit_now = opposing_votes >= 2;
            if short_entry {
                if strategy.allow_flip {
                    pos = -1;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }
            if exit_now {
                pos = 0;
            }
        } else {
            let opposing_votes = usize::from(coppock_up)
                + usize::from(ao_up)
                + usize::from(rocr_up)
                + usize::from(apo_up)
                + usize::from(otto_up);
            let mut exit_now = opposing_votes >= 2;
            if long_entry {
                if strategy.allow_flip {
                    pos = 1;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }
            if exit_now {
                pos = 0;
            }
        }

        desired[t] = pos;
    }

    desired
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_carm_params_rejects_invalid_values() {
        let mut cfg = CarmStrategyParams::default();
        cfg.coppock_short_period = 14;
        cfg.coppock_long_period = 14;
        assert!(validate_carm_params(&cfg).is_err());

        let mut cfg = CarmStrategyParams::default();
        cfg.otto_threshold = f64::INFINITY;
        assert!(validate_carm_params(&cfg).is_err());
    }

    #[test]
    fn build_carm_desired_positions_enters_and_exits_on_momentum_reversal() {
        let close = vec![100.0, 101.0, 102.0, 104.0, 105.0, 103.0, 101.0];
        let signals = CarmSignals {
            coppock: vec![f64::NAN, -0.2, 0.4, 0.8, 0.9, -0.1, -0.6],
            ao: vec![f64::NAN, -0.5, 0.2, 0.5, 0.6, -0.1, -0.4],
            rocr: vec![f64::NAN, 0.98, 1.02, 1.05, 1.06, 0.99, 0.96],
            apo: vec![f64::NAN, -0.3, 0.2, 0.4, 0.5, -0.2, -0.5],
            otto_hott: vec![f64::NAN, 100.6, 100.8, 101.5, 102.4, 103.4, 102.2],
            otto_lott: vec![f64::NAN, 100.2, 101.6, 102.8, 103.6, 102.9, 100.8],
            first_valid: 1,
        };
        let mut strategy = StrategyConfig::default();
        strategy.trade_on_next_bar = false;
        strategy.allow_flip = false;
        let cfg = CarmStrategyParams::default();

        let desired = build_carm_desired_positions(&close, &signals, 1, &strategy, &cfg);
        assert_eq!(desired[2], 1);
        assert_eq!(desired[5], 0);
    }
}
