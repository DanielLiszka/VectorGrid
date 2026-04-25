use crate::double_ma::Backend;
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorComputeOutput, DynamicIndicatorSeries,
};
#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_direct_with_inputs, compute_dynamic_indicator_cuda_with_inputs,
    DynamicIndicatorCudaSeries, PreparedCudaIndicatorInputs,
};
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, IndicatorParamKV, IndicatorParamValue,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use ta_strategies::double_ma::StrategyConfig;
#[cfg(feature = "cuda")]
use vector_ta::indicators::dispatch::DeviceMatrixF32;
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RocbStrategyParams {
    #[serde(default = "default_true")]
    pub map_fast_to_channel_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_atr_period: bool,
    #[serde(default = "default_rocb_channel_period")]
    pub channel_period: u16,
    #[serde(default = "default_rocb_atr_period")]
    pub atr_period: u16,
    #[serde(default = "default_rocb_atr_stop_mult")]
    pub atr_stop_mult: f64,
    #[serde(default = "default_rocb_max_trade_bars")]
    pub max_trade_bars: u16,
}

fn default_true() -> bool {
    true
}

fn default_rocb_channel_period() -> u16 {
    20
}

fn default_rocb_atr_period() -> u16 {
    14
}

fn default_rocb_atr_stop_mult() -> f64 {
    2.0
}

fn default_rocb_max_trade_bars() -> u16 {
    80
}

impl Default for RocbStrategyParams {
    fn default() -> Self {
        Self {
            map_fast_to_channel_period: default_true(),
            map_slow_to_atr_period: default_true(),
            channel_period: default_rocb_channel_period(),
            atr_period: default_rocb_atr_period(),
            atr_stop_mult: default_rocb_atr_stop_mult(),
            max_trade_bars: default_rocb_max_trade_bars(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RocbResolvedPeriods {
    pub channel: u16,
    pub atr: u16,
}

#[derive(Clone, Debug)]
pub struct RocbBreakoutSignals {
    pub upper: Vec<f64>,
    pub lower: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct RocbRiskSignals {
    pub atr: Vec<f64>,
    pub first_valid: usize,
}

struct RocbIndicatorIds {
    donchian_period_key: String,
    donchian_upper_output: String,
    donchian_lower_output: String,
    atr_period_key: String,
    atr_output: String,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
enum RocbDeviceMatrixOwner {
    Dispatch(DeviceMatrixF32),
}

#[cfg(feature = "cuda")]
pub struct RocbDeviceMatrix {
    pub device_ptr: u64,
    pub rows: usize,
    pub cols: usize,
    pub device_id: u32,
    _owner: RocbDeviceMatrixOwner,
}

#[cfg(feature = "cuda")]
impl RocbDeviceMatrix {
    fn from_dispatch(owner: DeviceMatrixF32) -> Self {
        Self {
            device_ptr: owner.device_ptr,
            rows: owner.rows,
            cols: owner.cols,
            device_id: owner.device_id,
            _owner: RocbDeviceMatrixOwner::Dispatch(owner),
        }
    }
}

#[cfg(feature = "cuda")]
pub struct RocbPeriodSweepDevice {
    pub matrix: RocbDeviceMatrix,
    pub period_start: u16,
    pub period_end: u16,
    pub period_step: u16,
}

#[cfg(feature = "cuda")]
impl RocbPeriodSweepDevice {
    pub fn row_index_for_period(&self, period: u16) -> Option<usize> {
        if self.period_step == 0 || period < self.period_start || period > self.period_end {
            return None;
        }
        let span = period.saturating_sub(self.period_start);
        if span % self.period_step != 0 {
            return None;
        }
        let idx = usize::from(span / self.period_step);
        (idx < self.matrix.rows).then_some(idx)
    }

    pub fn first_valid_for_period(&self, period: u16) -> usize {
        first_valid_from_period(period)
    }
}

pub fn validate_rocb_params(cfg: &RocbStrategyParams) -> Result<(), String> {
    if cfg.channel_period == 0 {
        return Err("ROCB channel_period must be > 0".to_string());
    }
    if cfg.atr_period == 0 {
        return Err("ROCB atr_period must be > 0".to_string());
    }
    if !cfg.atr_stop_mult.is_finite() || cfg.atr_stop_mult <= 0.0 {
        return Err("ROCB atr_stop_mult must be finite and > 0".to_string());
    }
    if cfg.max_trade_bars == 0 {
        return Err("ROCB max_trade_bars must be > 0".to_string());
    }
    Ok(())
}

pub fn resolve_rocb_periods(
    cfg: &RocbStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> RocbResolvedPeriods {
    RocbResolvedPeriods {
        channel: if cfg.map_fast_to_channel_period {
            fast_len.max(1)
        } else {
            cfg.channel_period.max(1)
        },
        atr: if cfg.map_slow_to_atr_period {
            slow_len.max(1)
        } else {
            cfg.atr_period.max(1)
        },
    }
}

fn first_valid_from_period(period: u16) -> usize {
    usize::from(period.saturating_sub(1))
}

fn first_finite_idx_local(values: &[f64]) -> Option<usize> {
    values.iter().position(|v| v.is_finite())
}

fn first_finite_idx_expected_or_scan(values: &[f64], expected: usize) -> Option<usize> {
    if expected < values.len() && values[expected].is_finite() {
        Some(expected)
    } else {
        first_finite_idx_local(values)
    }
}

fn first_joint_finite_idx_expected_or_scan(
    lhs: &[f64],
    rhs: &[f64],
    expected: usize,
) -> Option<usize> {
    if expected < lhs.len()
        && expected < rhs.len()
        && lhs[expected].is_finite()
        && rhs[expected].is_finite()
    {
        Some(expected)
    } else {
        lhs.iter()
            .zip(rhs.iter())
            .position(|(left, right)| left.is_finite() && right.is_finite())
    }
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

#[cfg(feature = "cuda")]
fn cuda_series_to_f64(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<Vec<f64>, String> {
    match out.series {
        DynamicIndicatorCudaSeries::HostF32(v) => Ok(v.into_iter().map(|x| x as f64).collect()),
        DynamicIndicatorCudaSeries::DeviceF32(_) => {
            Err("ROCB expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn cuda_series_to_device_f32(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<RocbDeviceMatrix, String> {
    match out.series {
        DynamicIndicatorCudaSeries::DeviceF32(v) => Ok(RocbDeviceMatrix::from_dispatch(v)),
        DynamicIndicatorCudaSeries::HostF32(_) => {
            Err("ROCB expected CUDA DeviceF32 output".to_string())
        }
    }
}

fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("ROCB indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "ROCB indicator '{indicator_id}' has no declared outputs"
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

fn resolve_distinct_output_ids(
    indicator_id: &str,
    upper_preferred_ids: &[&str],
    lower_preferred_ids: &[&str],
) -> Result<(String, String), String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("ROCB indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.len() < 2 {
        return Err(format!(
            "ROCB indicator '{indicator_id}' must expose at least two distinct outputs; available outputs: {}",
            descriptor
                .outputs
                .iter()
                .map(|output| output.id.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let upper = resolve_output_id(indicator_id, upper_preferred_ids)?;
    let lower = resolve_output_id(indicator_id, lower_preferred_ids)?;
    if upper.eq_ignore_ascii_case(lower.as_str()) {
        return Err(format!(
            "ROCB indicator '{indicator_id}' upper/lower outputs resolved to the same binding '{upper}'; available outputs: {}",
            descriptor
                .outputs
                .iter()
                .map(|output| output.id.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    Ok((upper, lower))
}

fn resolve_param_key(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("ROCB indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.params.is_empty() {
        return Err(format!(
            "ROCB indicator '{indicator_id}' has no declared parameters"
        ));
    }

    for preferred in preferred_ids {
        if let Some(found) = descriptor
            .params
            .iter()
            .find(|param| param.key.eq_ignore_ascii_case(preferred))
        {
            return Ok(found.key.clone());
        }
    }

    let preferred_lower: Vec<String> = preferred_ids
        .iter()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    for param in &descriptor.params {
        let param_id = param.key.to_ascii_lowercase();
        if preferred_lower
            .iter()
            .any(|p| param_id.contains(p) || p.contains(param_id.as_str()))
        {
            return Ok(param.key.clone());
        }
    }

    if descriptor.params.len() == 1 {
        return Ok(descriptor.params[0].key.clone());
    }

    Err(format!(
        "ROCB indicator '{indicator_id}' parameter key mismatch; available keys: {}",
        descriptor
            .params
            .iter()
            .map(|p| p.key.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn rocb_indicator_ids() -> Result<&'static RocbIndicatorIds, String> {
    static IDS: OnceLock<Result<RocbIndicatorIds, String>> = OnceLock::new();

    IDS.get_or_init(|| {
        let (donchian_upper_output, donchian_lower_output) = resolve_distinct_output_ids(
            "donchian",
            &["upper_band", "upperband", "upper", "up", "value"],
            &["lower_band", "lowerband", "lower", "down"],
        )?;
        Ok(RocbIndicatorIds {
            donchian_period_key: resolve_param_key(
                "donchian",
                &["period", "length", "timeperiod"],
            )?,
            donchian_upper_output,
            donchian_lower_output,
            atr_period_key: resolve_param_key("atr", &["period", "length", "timeperiod"])?,
            atr_output: resolve_output_id("atr", &["value", "atr"])?,
        })
    })
    .as_ref()
    .map_err(Clone::clone)
}

#[cfg(feature = "cuda")]
fn rocb_prefers_lower_output(output_preferred_ids: &[&str]) -> bool {
    output_preferred_ids.iter().any(|id| {
        matches!(
            id.to_ascii_lowercase().as_str(),
            "lower_band" | "lowerband" | "lower" | "down"
        )
    })
}

#[cfg(feature = "cuda")]
fn backend_prefers_gpu(backend: &Backend) -> bool {
    !matches!(backend, Backend::CpuOnly)
}

#[cfg(feature = "cuda")]
fn compute_indicator_period_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: &str,
    period_key: &str,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    base_params: &[IndicatorParamKV],
) -> Result<RocbPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);

    let mut params: Vec<IndicatorParamKV> = Vec::with_capacity(base_params.len() + 3);
    params.extend_from_slice(base_params);
    params.push(IndicatorParamKV {
        key: format!("{}_start", period_key),
        value: IndicatorParamValue::Int(i64::from(start)),
    });
    params.push(IndicatorParamKV {
        key: format!("{}_end", period_key),
        value: IndicatorParamValue::Int(i64::from(end)),
    });
    params.push(IndicatorParamKV {
        key: format!("{}_step", period_key),
        value: IndicatorParamValue::Int(i64::from(step)),
    });

    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("ROCB indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    let out = compute_dynamic_indicator_cuda_direct_with_inputs(
        prepared_inputs,
        indicator_id,
        Some(output_id),
        descriptor.input_kind,
        params.as_slice(),
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| e.to_string())?;
    let matrix = cuda_series_to_device_f32(out)?;

    Ok(RocbPeriodSweepDevice {
        matrix,
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_rocb_donchian_output_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    output_preferred_ids: &[&str],
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> Result<RocbPeriodSweepDevice, String> {
    let ids = rocb_indicator_ids()?;
    let output_id = if rocb_prefers_lower_output(output_preferred_ids) {
        ids.donchian_lower_output.as_str()
    } else {
        ids.donchian_upper_output.as_str()
    };
    compute_indicator_period_sweep_cuda_device(
        prepared_inputs,
        "donchian",
        output_id,
        ids.donchian_period_key.as_str(),
        period_start,
        period_end,
        period_step,
        &[],
    )
}

#[cfg(feature = "cuda")]
pub fn compute_rocb_atr_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> Result<RocbPeriodSweepDevice, String> {
    let ids = rocb_indicator_ids()?;
    compute_indicator_period_sweep_cuda_device(
        prepared_inputs,
        "atr",
        ids.atr_output.as_str(),
        ids.atr_period_key.as_str(),
        period_start,
        period_end,
        period_step,
        &[],
    )
}

fn compute_indicator_f64(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    #[cfg(not(feature = "cuda"))]
    let _ = backend;

    #[cfg(feature = "cuda")]
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("ROCB indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    #[cfg(feature = "cuda")]
    {
        let prefer_gpu = backend_prefers_gpu(backend)
            && descriptor.prefers_gpu_by_speedup
            && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);
        if prefer_gpu {
            if let Some(prepared) = prepared_cuda_inputs {
                let out = compute_dynamic_indicator_cuda_with_inputs(
                    prepared,
                    indicator_id,
                    Some(output_id),
                    params,
                    Kernel::Auto,
                    vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                );
                if let Ok(values) = out {
                    return cuda_series_to_f64(values);
                }
            }
        }
    }

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

pub fn compute_rocb_breakout_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    channel_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<RocbBreakoutSignals, String> {
    let ids = rocb_indicator_ids()?;
    let params = [IndicatorParamKV {
        key: ids.donchian_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(channel_period)),
    }];

    let upper = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "donchian",
        ids.donchian_upper_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let lower = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "donchian",
        ids.donchian_lower_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    if upper.len() != lower.len() {
        return Err("ROCB donchian output length mismatch".to_string());
    }

    let first_valid = first_joint_finite_idx_expected_or_scan(
        &upper,
        &lower,
        first_valid_from_period(channel_period),
    )
    .ok_or_else(|| "ROCB donchian outputs are all non-finite".to_string())?;

    Ok(RocbBreakoutSignals {
        upper,
        lower,
        first_valid,
    })
}

pub fn compute_rocb_risk_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    atr_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<RocbRiskSignals, String> {
    let ids = rocb_indicator_ids()?;
    let params = [IndicatorParamKV {
        key: ids.atr_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(atr_period)),
    }];
    let atr = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "atr",
        ids.atr_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let first_valid = first_finite_idx_expected_or_scan(&atr, first_valid_from_period(atr_period))
        .ok_or_else(|| "ROCB ATR output is all non-finite".to_string())?;
    Ok(RocbRiskSignals { atr, first_valid })
}

pub fn compute_rocb_start_t(
    first_valid_close: usize,
    first_valid_high: usize,
    first_valid_low: usize,
    first_valid_breakout: usize,
    first_valid_risk: usize,
    trade_on_next_bar: bool,
) -> usize {
    let first_valid = first_valid_close
        .max(first_valid_high)
        .max(first_valid_low)
        .max(first_valid_breakout)
        .max(first_valid_risk)
        .max(1);
    if trade_on_next_bar {
        first_valid.saturating_add(1)
    } else {
        first_valid
    }
}

pub fn build_rocb_desired_positions_into(
    desired: &mut Vec<i32>,
    close: &[f64],
    breakout: &RocbBreakoutSignals,
    risk: &RocbRiskSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &RocbStrategyParams,
) {
    let n = close.len();
    desired.clear();
    if desired.capacity() < n {
        desired.reserve(n);
    }
    desired.resize(n, 0);
    if start_t >= n {
        return;
    }
    if breakout.upper.len() != n || breakout.lower.len() != n || risk.atr.len() != n {
        return;
    }

    let trade_offset = usize::from(strategy.trade_on_next_bar);
    let long_only = strategy.long_only;
    let allow_flip = strategy.allow_flip;
    let max_trade_bars = usize::from(cfg.max_trade_bars.max(1));
    let atr_stop_mult = cfg.atr_stop_mult;
    let mut pos: i32 = 0;
    let mut bars_in_trade: usize = 0;
    let mut entry_price = f64::NAN;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close[sig_t];
        let upper_prev = breakout.upper[prev_t];
        let lower_prev = breakout.lower[prev_t];
        let atr_now = risk.atr[sig_t];

        let long_breakout =
            close_now.is_finite() && upper_prev.is_finite() && close_now > upper_prev;
        let short_breakout =
            close_now.is_finite() && lower_prev.is_finite() && close_now < lower_prev;

        if pos == 0 {
            bars_in_trade = 0;
            entry_price = f64::NAN;
            if long_breakout {
                pos = 1;
                bars_in_trade = 1;
                entry_price = close_now;
            } else if !long_only && short_breakout {
                pos = -1;
                bars_in_trade = 1;
                entry_price = close_now;
            }
        } else if pos > 0 {
            let stop_dist = if atr_now.is_finite() {
                atr_stop_mult * atr_now
            } else {
                f64::NAN
            };
            let stop_hit = entry_price.is_finite()
                && stop_dist.is_finite()
                && close_now.is_finite()
                && close_now <= entry_price - stop_dist;
            let channel_exit =
                close_now.is_finite() && upper_prev.is_finite() && close_now <= upper_prev;
            let mut exit_now = stop_hit || channel_exit || bars_in_trade >= max_trade_bars;

            if !long_only && short_breakout {
                if allow_flip {
                    pos = -1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }

            if exit_now {
                pos = 0;
                bars_in_trade = 0;
                entry_price = f64::NAN;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        } else {
            let stop_dist = if atr_now.is_finite() {
                atr_stop_mult * atr_now
            } else {
                f64::NAN
            };
            let stop_hit = entry_price.is_finite()
                && stop_dist.is_finite()
                && close_now.is_finite()
                && close_now >= entry_price + stop_dist;
            let channel_exit =
                close_now.is_finite() && lower_prev.is_finite() && close_now >= lower_prev;
            let mut exit_now = stop_hit || channel_exit || bars_in_trade >= max_trade_bars;

            if long_breakout {
                if allow_flip {
                    pos = 1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }

            if exit_now {
                pos = 0;
                bars_in_trade = 0;
                entry_price = f64::NAN;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        }
        desired[t] = pos;
    }
}

pub fn build_rocb_desired_positions(
    close: &[f64],
    breakout: &RocbBreakoutSignals,
    risk: &RocbRiskSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &RocbStrategyParams,
) -> Vec<i32> {
    let mut desired = Vec::with_capacity(close.len());
    build_rocb_desired_positions_into(&mut desired, close, breakout, risk, start_t, strategy, cfg);
    desired
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rocb_params_validate_and_resolve() {
        let cfg = RocbStrategyParams::default();
        validate_rocb_params(&cfg).expect("default ROCB params");
        let periods = resolve_rocb_periods(&cfg, 12, 48);
        assert_eq!(periods.channel, 12);
        assert_eq!(periods.atr, 48);
    }

    #[test]
    fn rocb_desired_positions_breakout_and_exit() {
        let close = vec![100.0, 101.0, 103.0, 104.0, 101.0, 99.0];
        let breakout = RocbBreakoutSignals {
            upper: vec![f64::NAN, 100.5, 102.0, 103.0, 102.5, 101.0],
            lower: vec![f64::NAN, 99.5, 99.0, 100.0, 100.5, 98.0],
            first_valid: 1,
        };
        let risk = RocbRiskSignals {
            atr: vec![f64::NAN, 1.0, 1.0, 1.0, 1.0, 1.0],
            first_valid: 1,
        };
        let mut strategy = StrategyConfig::default();
        strategy.trade_on_next_bar = false;
        let desired = build_rocb_desired_positions(
            close.as_slice(),
            &breakout,
            &risk,
            1,
            &strategy,
            &RocbStrategyParams::default(),
        );
        assert_eq!(desired[2], 1);
        assert_eq!(desired[4], 0);
    }

    #[test]
    fn rocb_desired_positions_into_reuses_buffer_with_zero_prefix() {
        let close = vec![100.0, 101.0, 103.0, 104.0, 101.0, 99.0];
        let breakout = RocbBreakoutSignals {
            upper: vec![f64::NAN, 100.5, 102.0, 103.0, 102.5, 101.0],
            lower: vec![f64::NAN, 99.5, 99.0, 100.0, 100.5, 98.0],
            first_valid: 1,
        };
        let risk = RocbRiskSignals {
            atr: vec![f64::NAN, 1.0, 1.0, 1.0, 1.0, 1.0],
            first_valid: 1,
        };
        let mut strategy = StrategyConfig::default();
        strategy.trade_on_next_bar = false;
        let mut desired = vec![7, 7, 7];

        build_rocb_desired_positions_into(
            &mut desired,
            close.as_slice(),
            &breakout,
            &risk,
            2,
            &strategy,
            &RocbStrategyParams::default(),
        );

        assert_eq!(desired.len(), close.len());
        assert_eq!(&desired[..2], &[0, 0]);
        assert_eq!(desired[2], 1);
        assert_eq!(desired[4], 0);
    }

    #[test]
    fn rocb_desired_positions_into_keeps_full_length_on_shape_mismatch() {
        let close = vec![100.0, 101.0, 103.0, 104.0, 101.0, 99.0];
        let breakout = RocbBreakoutSignals {
            upper: vec![f64::NAN, 100.5, 102.0],
            lower: vec![f64::NAN, 99.5, 99.0],
            first_valid: 1,
        };
        let risk = RocbRiskSignals {
            atr: vec![f64::NAN, 1.0, 1.0],
            first_valid: 1,
        };
        let mut desired = vec![7, 7, 7];

        build_rocb_desired_positions_into(
            &mut desired,
            close.as_slice(),
            &breakout,
            &risk,
            2,
            &StrategyConfig::default(),
            &RocbStrategyParams::default(),
        );

        assert_eq!(desired, vec![0; close.len()]);
    }

    #[test]
    fn rocb_joint_first_valid_scans_for_real_overlap() {
        let lhs = vec![f64::NAN, 10.0, 11.0, 12.0];
        let rhs = vec![f64::NAN, f64::NAN, f64::NAN, 20.0];

        assert_eq!(
            first_joint_finite_idx_expected_or_scan(&lhs, &rhs, 1),
            Some(3)
        );
    }

    #[test]
    fn rocb_first_valid_fast_path_preserves_short_series_none() {
        let values = vec![f64::NAN; 3];
        assert_eq!(first_finite_idx_expected_or_scan(&values, 5), None);
        assert_eq!(
            first_joint_finite_idx_expected_or_scan(&values, &values, 5),
            None
        );
    }

    #[test]
    fn rocb_indicator_ids_resolve_distinct_donchian_outputs() {
        let ids = rocb_indicator_ids().expect("ROCB indicator ids resolve");

        assert!(!ids.donchian_period_key.is_empty());
        assert_ne!(
            ids.donchian_upper_output.to_ascii_lowercase(),
            ids.donchian_lower_output.to_ascii_lowercase()
        );
        assert!(!ids.atr_output.is_empty());
    }
}
