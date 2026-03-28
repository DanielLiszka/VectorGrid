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
    find_dynamic_indicator_descriptor, IndicatorInputKind, IndicatorParamKV, IndicatorParamValue,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use ta_strategies::double_ma::StrategyConfig;
#[cfg(feature = "cuda")]
use vector_ta::cuda::{
    atr_wrapper::DeviceArrayF32Atr, bollinger_bands_wrapper::DeviceArrayF32Bb, CudaAtr,
    CudaBollingerBands, CudaZscore, DeviceArrayF32,
};
#[cfg(feature = "cuda")]
use vector_ta::indicators::dispatch::DeviceMatrixF32;
#[cfg(feature = "cuda")]
use vector_ta::indicators::{
    atr::AtrBatchRange, bollinger_bands::BollingerBandsBatchRange, zscore::ZscoreBatchRange,
};
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VemrStrategyParams {
    #[serde(default = "default_true")]
    pub map_fast_to_z_period: bool,
    #[serde(default = "default_true")]
    pub map_fast_to_bb_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_atr_period: bool,

    #[serde(default = "default_vemr_z_period")]
    pub z_period: u16,
    #[serde(default = "default_vemr_z_ma_type")]
    pub z_ma_type: String,
    #[serde(default = "default_vemr_z_nbdev")]
    pub z_nbdev: f64,
    #[serde(default = "default_vemr_z_devtype")]
    pub z_devtype: u8,
    #[serde(default = "default_vemr_z_entry")]
    pub z_entry: f64,
    #[serde(default = "default_vemr_z_exit")]
    pub z_exit: f64,

    #[serde(default = "default_vemr_bb_period")]
    pub bb_period: u16,
    #[serde(default = "default_vemr_bb_mult")]
    pub bb_mult: f64,
    #[serde(default = "default_true")]
    pub require_band_touch: bool,

    #[serde(default = "default_vemr_atr_period")]
    pub atr_period: u16,
    #[serde(default = "default_vemr_atr_stop_mult")]
    pub atr_stop_mult: f64,

    #[serde(default = "default_vemr_confirm_bars")]
    pub confirm_bars: u16,
    #[serde(default = "default_vemr_max_trade_bars")]
    pub max_trade_bars: u16,
}

fn default_true() -> bool {
    true
}

fn default_vemr_z_period() -> u16 {
    20
}

fn default_vemr_z_ma_type() -> String {
    "sma".to_string()
}

fn default_vemr_z_nbdev() -> f64 {
    1.0
}

fn default_vemr_z_devtype() -> u8 {
    0
}

fn default_vemr_z_entry() -> f64 {
    2.0
}

fn default_vemr_z_exit() -> f64 {
    0.5
}

fn default_vemr_bb_period() -> u16 {
    20
}

fn default_vemr_bb_mult() -> f64 {
    2.0
}

fn default_vemr_atr_period() -> u16 {
    14
}

fn default_vemr_atr_stop_mult() -> f64 {
    2.0
}

fn default_vemr_confirm_bars() -> u16 {
    0
}

fn default_vemr_max_trade_bars() -> u16 {
    80
}

impl Default for VemrStrategyParams {
    fn default() -> Self {
        Self {
            map_fast_to_z_period: default_true(),
            map_fast_to_bb_period: default_true(),
            map_slow_to_atr_period: default_true(),
            z_period: default_vemr_z_period(),
            z_ma_type: default_vemr_z_ma_type(),
            z_nbdev: default_vemr_z_nbdev(),
            z_devtype: default_vemr_z_devtype(),
            z_entry: default_vemr_z_entry(),
            z_exit: default_vemr_z_exit(),
            bb_period: default_vemr_bb_period(),
            bb_mult: default_vemr_bb_mult(),
            require_band_touch: default_true(),
            atr_period: default_vemr_atr_period(),
            atr_stop_mult: default_vemr_atr_stop_mult(),
            confirm_bars: default_vemr_confirm_bars(),
            max_trade_bars: default_vemr_max_trade_bars(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VemrResolvedPeriods {
    pub z: u16,
    pub bb: u16,
    pub atr: u16,
}

#[derive(Clone, Debug)]
pub struct VemrFastSignals {
    pub zscore: Vec<f64>,
    pub bb_upper: Vec<f64>,
    pub bb_middle: Vec<f64>,
    pub bb_lower: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct VemrRiskSignals {
    pub atr: Vec<f64>,
    pub first_valid: usize,
}

struct VemrIndicatorIds {
    zscore_period_key: String,
    zscore_ma_type_key: String,
    zscore_nbdev_key: String,
    zscore_devtype_key: String,
    zscore_output: String,
    zscore_input_kind: IndicatorInputKind,
    bollinger_period_key: String,
    bollinger_devup_key: String,
    bollinger_devdn_key: String,
    bollinger_upper_output: String,
    bollinger_middle_output: String,
    bollinger_lower_output: String,
    atr_period_key: String,
    atr_output: String,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
enum VemrDeviceMatrixOwner {
    Dispatch(DeviceMatrixF32),
    Zscore(DeviceArrayF32),
    Bollinger(DeviceArrayF32Bb),
    Atr(DeviceArrayF32Atr),
}

#[cfg(feature = "cuda")]
pub struct VemrDeviceMatrix {
    pub device_ptr: u64,
    pub rows: usize,
    pub cols: usize,
    pub device_id: u32,
    _owner: VemrDeviceMatrixOwner,
}

#[cfg(feature = "cuda")]
impl VemrDeviceMatrix {
    fn from_dispatch(owner: DeviceMatrixF32) -> Self {
        Self {
            device_ptr: owner.device_ptr,
            rows: owner.rows,
            cols: owner.cols,
            device_id: owner.device_id,
            _owner: VemrDeviceMatrixOwner::Dispatch(owner),
        }
    }

    fn from_zscore(owner: DeviceArrayF32, device_id: u32) -> Self {
        Self {
            device_ptr: owner.device_ptr(),
            rows: owner.rows,
            cols: owner.cols,
            device_id,
            _owner: VemrDeviceMatrixOwner::Zscore(owner),
        }
    }

    fn from_bollinger(owner: DeviceArrayF32Bb) -> Self {
        Self {
            device_ptr: owner.device_ptr(),
            rows: owner.rows,
            cols: owner.cols,
            device_id: owner.device_id,
            _owner: VemrDeviceMatrixOwner::Bollinger(owner),
        }
    }

    fn from_atr(owner: DeviceArrayF32Atr) -> Self {
        Self {
            device_ptr: owner.device_ptr(),
            rows: owner.rows,
            cols: owner.cols,
            device_id: owner.device_id,
            _owner: VemrDeviceMatrixOwner::Atr(owner),
        }
    }
}

#[cfg(feature = "cuda")]
pub struct VemrPeriodSweepDevice {
    pub matrix: VemrDeviceMatrix,
    pub period_start: u16,
    pub period_end: u16,
    pub period_step: u16,
}

#[cfg(feature = "cuda")]
impl VemrPeriodSweepDevice {
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

    pub fn row_ptr_for_period(&self, period: u16) -> Option<u64> {
        let row = self.row_index_for_period(period)?;
        let cols = self.matrix.cols as u64;
        let offset = (row as u64)
            .checked_mul(cols)?
            .checked_mul(std::mem::size_of::<f32>() as u64)?;
        self.matrix.device_ptr.checked_add(offset)
    }

    pub fn first_valid_for_period(&self, period: u16) -> usize {
        usize::from(period.saturating_sub(1))
    }
}

pub fn validate_vemr_params(cfg: &VemrStrategyParams) -> Result<(), String> {
    if cfg.z_period == 0 {
        return Err("VEMR z_period must be > 0".to_string());
    }
    if cfg.bb_period == 0 {
        return Err("VEMR bb_period must be > 0".to_string());
    }
    if cfg.atr_period == 0 {
        return Err("VEMR atr_period must be > 0".to_string());
    }
    if !cfg.z_nbdev.is_finite() || cfg.z_nbdev <= 0.0 {
        return Err("VEMR z_nbdev must be finite and > 0".to_string());
    }
    if !cfg.z_entry.is_finite() || cfg.z_entry <= 0.0 {
        return Err("VEMR z_entry must be finite and > 0".to_string());
    }
    if !cfg.z_exit.is_finite() || cfg.z_exit < 0.0 {
        return Err("VEMR z_exit must be finite and >= 0".to_string());
    }
    if cfg.z_exit > cfg.z_entry {
        return Err("VEMR z_exit must be <= z_entry".to_string());
    }
    if !cfg.bb_mult.is_finite() || cfg.bb_mult <= 0.0 {
        return Err("VEMR bb_mult must be finite and > 0".to_string());
    }
    if !cfg.atr_stop_mult.is_finite() || cfg.atr_stop_mult <= 0.0 {
        return Err("VEMR atr_stop_mult must be finite and > 0".to_string());
    }
    if cfg.max_trade_bars == 0 {
        return Err("VEMR max_trade_bars must be > 0".to_string());
    }
    if cfg.z_ma_type.trim().is_empty() {
        return Err("VEMR z_ma_type cannot be empty".to_string());
    }
    Ok(())
}

pub fn resolve_vemr_periods(
    cfg: &VemrStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> VemrResolvedPeriods {
    VemrResolvedPeriods {
        z: if cfg.map_fast_to_z_period {
            fast_len.max(1)
        } else {
            cfg.z_period.max(1)
        },
        bb: if cfg.map_fast_to_bb_period {
            fast_len.max(1)
        } else {
            cfg.bb_period.max(1)
        },
        atr: if cfg.map_slow_to_atr_period {
            slow_len.max(1)
        } else {
            cfg.atr_period.max(1)
        },
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

#[cfg(feature = "cuda")]
fn cuda_series_to_f64(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<Vec<f64>, String> {
    match out.series {
        DynamicIndicatorCudaSeries::HostF32(v) => Ok(v.into_iter().map(|x| x as f64).collect()),
        DynamicIndicatorCudaSeries::DeviceF32(_) => {
            Err("VEMR expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn cuda_series_to_device_f32(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<VemrDeviceMatrix, String> {
    match out.series {
        DynamicIndicatorCudaSeries::DeviceF32(v) => Ok(VemrDeviceMatrix::from_dispatch(v)),
        DynamicIndicatorCudaSeries::HostF32(_) => {
            Err("VEMR expected CUDA DeviceF32 output".to_string())
        }
    }
}

fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("VEMR indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "VEMR indicator '{indicator_id}' has no declared outputs"
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

fn resolve_param_key(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("VEMR indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.params.is_empty() {
        return Err(format!(
            "VEMR indicator '{indicator_id}' has no declared parameters"
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
        "VEMR indicator '{indicator_id}' parameter key mismatch; available keys: {}",
        descriptor
            .params
            .iter()
            .map(|p| p.key.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn vemr_indicator_ids() -> Result<&'static VemrIndicatorIds, String> {
    static IDS: OnceLock<Result<VemrIndicatorIds, String>> = OnceLock::new();

    IDS.get_or_init(|| {
        let zscore_descriptor = find_dynamic_indicator_descriptor("zscore").ok_or_else(|| {
            "VEMR indicator 'zscore' is not registered for dynamic dispatch".to_string()
        })?;
        Ok(VemrIndicatorIds {
            zscore_period_key: resolve_param_key("zscore", &["period", "length", "timeperiod"])?,
            zscore_ma_type_key: resolve_param_key("zscore", &["ma_type", "matype", "ma"])?,
            zscore_nbdev_key: resolve_param_key(
                "zscore",
                &["nbdev", "stddev", "stdev", "deviation"],
            )?,
            zscore_devtype_key: resolve_param_key(
                "zscore",
                &["devtype", "deviation_type", "dev_type"],
            )?,
            zscore_output: resolve_output_id("zscore", &["value", "zscore"])?,
            zscore_input_kind: zscore_descriptor.input_kind,
            bollinger_period_key: resolve_param_key(
                "bollinger_bands",
                &["period", "length", "timeperiod"],
            )?,
            bollinger_devup_key: resolve_param_key(
                "bollinger_bands",
                &["devup", "upper_mult", "upper"],
            )?,
            bollinger_devdn_key: resolve_param_key(
                "bollinger_bands",
                &["devdn", "lower_mult", "lower"],
            )?,
            bollinger_upper_output: resolve_output_id(
                "bollinger_bands",
                &["upper_band", "upper", "up"],
            )?,
            bollinger_middle_output: resolve_output_id(
                "bollinger_bands",
                &["middle_band", "middle", "mid", "basis"],
            )?,
            bollinger_lower_output: resolve_output_id(
                "bollinger_bands",
                &["lower_band", "lower", "down"],
            )?,
            atr_period_key: resolve_param_key("atr", &["period", "length", "timeperiod"])?,
            atr_output: resolve_output_id("atr", &["value", "atr"])?,
        })
    })
    .as_ref()
    .map_err(Clone::clone)
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
    input_kind: IndicatorInputKind,
    period_key: &str,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    base_params: &[IndicatorParamKV],
) -> Result<VemrPeriodSweepDevice, String> {
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

    let out = compute_dynamic_indicator_cuda_direct_with_inputs(
        prepared_inputs,
        indicator_id,
        Some(output_id),
        input_kind,
        params.as_slice(),
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| e.to_string())?;
    let matrix = cuda_series_to_device_f32(out)?;

    Ok(VemrPeriodSweepDevice {
        matrix,
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_vemr_zscore_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    let sweep = ZscoreBatchRange {
        period: (
            usize::from(period_start.max(1)),
            usize::from(period_end.max(period_start.max(1))),
            usize::from(period_step.max(1)),
        ),
        ma_type: (cfg.z_ma_type.clone(), cfg.z_ma_type.clone(), String::new()),
        nbdev: (cfg.z_nbdev, cfg.z_nbdev, 0.0),
        devtype: (usize::from(cfg.z_devtype), usize::from(cfg.z_devtype), 0),
    };
    let wrapper = CudaZscore::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.source_first_valid();
    let (matrix, _) = wrapper
        .zscore_batch_from_device_ptr(
            prepared_inputs.source_device().buffer().as_device_ptr(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| wrapper.zscore_batch_dev(prepared_inputs.source(), &sweep))
        .map_err(|e| e.to_string())?;

    Ok(VemrPeriodSweepDevice {
        matrix: VemrDeviceMatrix::from_zscore(matrix, device_id),
        period_start: period_start.max(1),
        period_end: period_end.max(period_start.max(1)),
        period_step: period_step.max(1),
    })
}

#[cfg(feature = "cuda")]
fn compute_vemr_bollinger_output_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    output_preferred_ids: &[&str],
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    let sweep = BollingerBandsBatchRange {
        period: (
            usize::from(period_start.max(1)),
            usize::from(period_end.max(period_start.max(1))),
            usize::from(period_step.max(1)),
        ),
        devup: (cfg.bb_mult, cfg.bb_mult, 0.0),
        devdn: (cfg.bb_mult, cfg.bb_mult, 0.0),
        matype: ("sma".to_string(), "sma".to_string(), 0),
        devtype: (0, 0, 0),
    };
    let wrapper = CudaBollingerBands::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.source_first_valid();
    let (upper, middle, lower) = wrapper
        .bollinger_bands_batch_from_device_ptr(
            prepared_inputs.source_device().buffer().as_device_ptr(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| wrapper.bollinger_bands_batch_dev(prepared_inputs.source(), &sweep))
        .map_err(|e| e.to_string())?;

    let matrix = if output_preferred_ids
        .iter()
        .any(|id| ["upper_band", "upper", "up"].contains(id))
    {
        VemrDeviceMatrix::from_bollinger(upper)
    } else if output_preferred_ids
        .iter()
        .any(|id| ["middle_band", "middle", "mid", "basis"].contains(id))
    {
        VemrDeviceMatrix::from_bollinger(middle)
    } else {
        VemrDeviceMatrix::from_bollinger(lower)
    };

    Ok(VemrPeriodSweepDevice {
        matrix,
        period_start: period_start.max(1),
        period_end: period_end.max(period_start.max(1)),
        period_step: period_step.max(1),
    })
}

#[cfg(feature = "cuda")]
fn compute_vemr_atr_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    let sweep = AtrBatchRange {
        length: (
            usize::from(period_start.max(1)),
            usize::from(period_end.max(period_start.max(1))),
            usize::from(period_step.max(1)),
        ),
    };
    let wrapper = CudaAtr::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.hlc_first_valid();
    let matrix = wrapper
        .atr_batch_from_device_ptrs(
            prepared_inputs.high_device().buffer().as_device_ptr(),
            prepared_inputs.low_device().buffer().as_device_ptr(),
            prepared_inputs.close_device().buffer().as_device_ptr(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| {
            wrapper.atr_batch_dev(
                prepared_inputs.high(),
                prepared_inputs.low(),
                prepared_inputs.close(),
                &sweep,
            )
        })
        .map_err(|e| e.to_string())?;

    Ok(VemrPeriodSweepDevice {
        matrix: VemrDeviceMatrix::from_atr(matrix),
        period_start: period_start.max(1),
        period_end: period_end.max(period_start.max(1)),
        period_step: period_step.max(1),
    })
}

#[cfg(feature = "cuda")]
pub fn compute_vemr_zscore_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    let ids = vemr_indicator_ids()?;
    if cfg.z_devtype == 0 && cfg.z_ma_type.eq_ignore_ascii_case("sma") {
        return compute_vemr_zscore_sweep_cuda_direct(
            prepared_inputs,
            cfg,
            period_start,
            period_end,
            period_step,
            device_id,
        );
    }

    let base_params = vec![
        IndicatorParamKV {
            key: ids.zscore_ma_type_key.clone(),
            value: IndicatorParamValue::EnumString(cfg.z_ma_type.clone()),
        },
        IndicatorParamKV {
            key: ids.zscore_nbdev_key.clone(),
            value: IndicatorParamValue::Float(cfg.z_nbdev),
        },
        IndicatorParamKV {
            key: ids.zscore_devtype_key.clone(),
            value: IndicatorParamValue::Int(i64::from(cfg.z_devtype)),
        },
    ];

    compute_indicator_period_sweep_cuda_device(
        prepared_inputs,
        "zscore",
        ids.zscore_output.as_str(),
        ids.zscore_input_kind,
        ids.zscore_period_key.as_str(),
        period_start,
        period_end,
        period_step,
        base_params.as_slice(),
    )
}

#[cfg(feature = "cuda")]
fn compute_vemr_bollinger_output_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    output_preferred_ids: &[&str],
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    compute_vemr_bollinger_output_sweep_cuda_direct(
        prepared_inputs,
        cfg,
        output_preferred_ids,
        period_start,
        period_end,
        period_step,
        device_id,
    )
}

#[cfg(feature = "cuda")]
pub fn compute_vemr_bollinger_upper_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    compute_vemr_bollinger_output_sweep_cuda_device(
        prepared_inputs,
        cfg,
        &["upper_band", "upper", "up"],
        period_start,
        period_end,
        period_step,
        device_id,
    )
}

#[cfg(feature = "cuda")]
pub fn compute_vemr_bollinger_middle_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    compute_vemr_bollinger_output_sweep_cuda_device(
        prepared_inputs,
        cfg,
        &["middle_band", "middle", "mid", "basis"],
        period_start,
        period_end,
        period_step,
        device_id,
    )
}

#[cfg(feature = "cuda")]
pub fn compute_vemr_bollinger_lower_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VemrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    compute_vemr_bollinger_output_sweep_cuda_device(
        prepared_inputs,
        cfg,
        &["lower_band", "lower", "down"],
        period_start,
        period_end,
        period_step,
        device_id,
    )
}

#[cfg(feature = "cuda")]
pub fn compute_vemr_atr_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VemrPeriodSweepDevice, String> {
    compute_vemr_atr_sweep_cuda_direct(
        prepared_inputs,
        period_start,
        period_end,
        period_step,
        device_id,
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
        format!("VEMR indicator '{indicator_id}' is not registered for dynamic dispatch")
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

pub fn compute_vemr_fast_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &VemrStrategyParams,
    z_period: u16,
    bb_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<VemrFastSignals, String> {
    let ids = vemr_indicator_ids()?;

    let z_params = vec![
        IndicatorParamKV {
            key: ids.zscore_period_key.clone(),
            value: IndicatorParamValue::Int(i64::from(z_period)),
        },
        IndicatorParamKV {
            key: ids.zscore_ma_type_key.clone(),
            value: IndicatorParamValue::EnumString(cfg.z_ma_type.clone()),
        },
        IndicatorParamKV {
            key: ids.zscore_nbdev_key.clone(),
            value: IndicatorParamValue::Float(cfg.z_nbdev),
        },
        IndicatorParamKV {
            key: ids.zscore_devtype_key.clone(),
            value: IndicatorParamValue::Int(i64::from(cfg.z_devtype)),
        },
    ];
    let bb_params = vec![
        IndicatorParamKV {
            key: ids.bollinger_period_key.clone(),
            value: IndicatorParamValue::Int(i64::from(bb_period)),
        },
        IndicatorParamKV {
            key: ids.bollinger_devup_key.clone(),
            value: IndicatorParamValue::Float(cfg.bb_mult),
        },
        IndicatorParamKV {
            key: ids.bollinger_devdn_key.clone(),
            value: IndicatorParamValue::Float(cfg.bb_mult),
        },
    ];

    let zscore = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "zscore",
        ids.zscore_output.as_str(),
        &z_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let bb_upper = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "bollinger_bands",
        ids.bollinger_upper_output.as_str(),
        &bb_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let bb_middle = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "bollinger_bands",
        ids.bollinger_middle_output.as_str(),
        &bb_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let bb_lower = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "bollinger_bands",
        ids.bollinger_lower_output.as_str(),
        &bb_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    if zscore.len() != bb_upper.len()
        || zscore.len() != bb_middle.len()
        || zscore.len() != bb_lower.len()
    {
        return Err("VEMR fast signal output length mismatch".to_string());
    }

    let first_valid = first_finite_idx_local(&zscore)
        .zip(first_finite_idx_local(&bb_upper))
        .zip(first_finite_idx_local(&bb_middle))
        .zip(first_finite_idx_local(&bb_lower))
        .map(|(((a, b), c), d)| a.max(b).max(c).max(d))
        .ok_or_else(|| "VEMR fast signal outputs are all non-finite".to_string())?;

    Ok(VemrFastSignals {
        zscore,
        bb_upper,
        bb_middle,
        bb_lower,
        first_valid,
    })
}

pub fn compute_vemr_risk_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    atr_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<VemrRiskSignals, String> {
    let ids = vemr_indicator_ids()?;
    let atr_params = vec![IndicatorParamKV {
        key: ids.atr_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(atr_period)),
    }];
    let atr = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "atr",
        ids.atr_output.as_str(),
        &atr_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let first_valid = first_finite_idx_local(&atr)
        .ok_or_else(|| "VEMR ATR output is all non-finite".to_string())?;

    Ok(VemrRiskSignals { atr, first_valid })
}

pub fn compute_vemr_start_t(
    first_valid_close: usize,
    first_valid_source: usize,
    first_valid_fast: usize,
    first_valid_risk: usize,
    max_period: usize,
    trade_on_next_bar: bool,
) -> usize {
    let mut first_valid = first_valid_close.max(first_valid_source);
    first_valid = first_valid.max(first_valid_fast);
    first_valid = first_valid.max(first_valid_risk);

    let t_valid = first_valid.saturating_add(max_period.max(1).saturating_sub(1));
    if trade_on_next_bar {
        t_valid.saturating_add(1)
    } else {
        t_valid
    }
}

pub fn build_vemr_desired_positions(
    close: &[f64],
    fast: &VemrFastSignals,
    risk: &VemrRiskSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &VemrStrategyParams,
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
    let mut pos: i32 = 0;
    let mut bars_in_trade: usize = 0;
    let mut entry_price = f64::NAN;
    let mut long_setup_count: usize = 0;
    let mut short_setup_count: usize = 0;
    let need_confirm = usize::from(cfg.confirm_bars).saturating_add(1);

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let close_now = close.get(sig_t).copied().unwrap_or(f64::NAN);
        let z_now = fast.zscore.get(sig_t).copied().unwrap_or(f64::NAN);
        let upper_now = fast.bb_upper.get(sig_t).copied().unwrap_or(f64::NAN);
        let middle_now = fast.bb_middle.get(sig_t).copied().unwrap_or(f64::NAN);
        let lower_now = fast.bb_lower.get(sig_t).copied().unwrap_or(f64::NAN);
        let atr_now = risk.atr.get(sig_t).copied().unwrap_or(f64::NAN);

        let long_seed = z_now.is_finite()
            && z_now <= -cfg.z_entry
            && (!cfg.require_band_touch
                || (close_now.is_finite() && lower_now.is_finite() && close_now <= lower_now));
        let short_seed = z_now.is_finite()
            && z_now >= cfg.z_entry
            && (!cfg.require_band_touch
                || (close_now.is_finite() && upper_now.is_finite() && close_now >= upper_now));

        if pos == 0 {
            bars_in_trade = 0;
            entry_price = f64::NAN;

            long_setup_count = if long_seed {
                long_setup_count.saturating_add(1)
            } else {
                0
            };
            short_setup_count = if short_seed {
                short_setup_count.saturating_add(1)
            } else {
                0
            };

            if long_setup_count >= need_confirm {
                pos = 1;
                bars_in_trade = 1;
                entry_price = close_now;
                long_setup_count = 0;
                short_setup_count = 0;
            } else if !strategy.long_only && short_setup_count >= need_confirm {
                pos = -1;
                bars_in_trade = 1;
                entry_price = close_now;
                long_setup_count = 0;
                short_setup_count = 0;
            }
        } else if pos > 0 {
            let stop_dist = if atr_now.is_finite() {
                cfg.atr_stop_mult * atr_now
            } else {
                f64::NAN
            };
            let long_stop_hit = entry_price.is_finite()
                && stop_dist.is_finite()
                && close_now.is_finite()
                && close_now <= entry_price - stop_dist;
            let mean_exit = (z_now.is_finite() && z_now >= -cfg.z_exit)
                || (close_now.is_finite() && middle_now.is_finite() && close_now >= middle_now);

            let mut exit_now = long_stop_hit || mean_exit;
            if !strategy.long_only && short_seed {
                if strategy.allow_flip {
                    pos = -1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    long_setup_count = 0;
                    short_setup_count = 0;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }
            if bars_in_trade >= usize::from(cfg.max_trade_bars.max(1)) {
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
                cfg.atr_stop_mult * atr_now
            } else {
                f64::NAN
            };
            let short_stop_hit = entry_price.is_finite()
                && stop_dist.is_finite()
                && close_now.is_finite()
                && close_now >= entry_price + stop_dist;
            let mean_exit = (z_now.is_finite() && z_now <= cfg.z_exit)
                || (close_now.is_finite() && middle_now.is_finite() && close_now <= middle_now);

            let mut exit_now = short_stop_hit || mean_exit;
            if long_seed {
                if strategy.allow_flip {
                    pos = 1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    long_setup_count = 0;
                    short_setup_count = 0;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }
            if bars_in_trade >= usize::from(cfg.max_trade_bars.max(1)) {
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

        desired[t] = if strategy.long_only && pos < 0 {
            0
        } else {
            pos
        };
    }

    desired
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_vemr_periods_maps_fast_and_slow_when_enabled() {
        let cfg = VemrStrategyParams::default();
        let p = resolve_vemr_periods(&cfg, 8, 21);
        assert_eq!(p.z, 8);
        assert_eq!(p.bb, 8);
        assert_eq!(p.atr, 21);
    }

    #[test]
    fn validate_vemr_params_rejects_invalid_values() {
        let mut cfg = VemrStrategyParams::default();
        cfg.max_trade_bars = 0;
        let err = validate_vemr_params(&cfg).expect_err("should reject max_trade_bars=0");
        assert!(err.contains("max_trade_bars"));
    }
}
