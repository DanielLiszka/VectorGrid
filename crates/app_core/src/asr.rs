use crate::double_ma::Backend;
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorComputeOutput, DynamicIndicatorSeries,
};
#[cfg(feature = "cuda")]
use crate::indicator_catalog::IndicatorInputKind;
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, IndicatorParamKV, IndicatorParamValue,
};
use crate::iterative_sim::{IntrabarRule, IterativeSimConfig};
use serde::{Deserialize, Serialize};
#[cfg(feature = "cuda")]
use std::cell::RefCell;
use ta_strategies::double_ma::{DoubleMaCurves, Metrics, StrategyConfig, Trade};
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_direct_with_inputs, compute_dynamic_indicator_cuda_with_inputs,
    DynamicIndicatorCudaSeries, PreparedCudaIndicatorInputs,
};
#[cfg(feature = "cuda")]
use vector_ta::cuda::{
    er_wrapper::DeviceArrayF32Er, CudaEr, CudaKaufmanstop, CudaLinearregAngle, CudaSqueezeMomentum,
    CudaTtmSqueeze, CudaZscore, DeviceArrayF32,
};
#[cfg(feature = "cuda")]
use vector_ta::indicators::dispatch::DeviceMatrixF32;
#[cfg(feature = "cuda")]
use vector_ta::indicators::{
    er::ErBatchRange, linearreg_angle::Linearreg_angleBatchRange,
    squeeze_momentum::SqueezeMomentumBatchRange, ttm_squeeze::TtmSqueezeBatchRange,
    zscore::ZscoreBatchRange,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AsrEntryMode {
    Trend,
    MeanReversion,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AsrStrategyParams {
    #[serde(default = "default_true")]
    pub map_fast_to_squeeze_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_trend_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_er_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_z_period: bool,

    #[serde(default = "default_asr_squeeze_period")]
    pub squeeze_period: u16,
    #[serde(default = "default_asr_squeeze_bb_mult")]
    pub squeeze_bb_mult: f64,
    #[serde(default = "default_asr_squeeze_kc_mult")]
    pub squeeze_kc_mult: f64,

    #[serde(default = "default_asr_trend_period")]
    pub trend_period: u16,
    #[serde(default = "default_asr_angle_deg_min")]
    pub angle_deg_min: f64,
    #[serde(default = "default_asr_angle_deg_mr")]
    pub angle_deg_mr: f64,

    #[serde(default = "default_true")]
    pub use_er_filter: bool,
    #[serde(default = "default_asr_er_period")]
    pub er_period: u16,
    #[serde(default = "default_asr_er_min")]
    pub er_min: f64,
    #[serde(default = "default_asr_er_mr")]
    pub er_mr: f64,

    #[serde(default = "default_asr_z_period")]
    pub z_period: u16,
    #[serde(default = "default_asr_z_ma_type")]
    pub z_ma_type: String,
    #[serde(default = "default_asr_z_nbdev")]
    pub z_nbdev: f64,
    #[serde(default = "default_asr_z_devtype")]
    pub z_devtype: u8,
    #[serde(default = "default_asr_z_entry")]
    pub z_entry: f64,
    #[serde(default = "default_asr_z_exit")]
    pub z_exit: f64,

    #[serde(default = "default_asr_confirm_bars")]
    pub confirm_bars: u16,
    #[serde(default = "default_asr_mom_fade_bars")]
    pub mom_fade_bars: u16,
    #[serde(default = "default_asr_max_trade_bars")]
    pub max_trade_bars: u16,
    #[serde(default = "default_asr_max_mr_trade_bars")]
    pub max_mr_trade_bars: u16,
}

fn default_true() -> bool {
    true
}

fn default_asr_squeeze_period() -> u16 {
    20
}

fn default_asr_squeeze_bb_mult() -> f64 {
    2.0
}

fn default_asr_squeeze_kc_mult() -> f64 {
    1.5
}

fn default_asr_trend_period() -> u16 {
    14
}

fn default_asr_angle_deg_min() -> f64 {
    10.0
}

fn default_asr_angle_deg_mr() -> f64 {
    6.0
}

fn default_asr_er_period() -> u16 {
    20
}

fn default_asr_er_min() -> f64 {
    0.30
}

fn default_asr_er_mr() -> f64 {
    0.25
}

fn default_asr_z_period() -> u16 {
    14
}

fn default_asr_z_ma_type() -> String {
    "sma".to_string()
}

fn default_asr_z_nbdev() -> f64 {
    1.0
}

fn default_asr_z_devtype() -> u8 {
    0
}

fn default_asr_z_entry() -> f64 {
    2.0
}

fn default_asr_z_exit() -> f64 {
    0.5
}

fn default_asr_confirm_bars() -> u16 {
    0
}

fn default_asr_mom_fade_bars() -> u16 {
    2
}

fn default_asr_max_trade_bars() -> u16 {
    80
}

fn default_asr_max_mr_trade_bars() -> u16 {
    30
}

impl Default for AsrStrategyParams {
    fn default() -> Self {
        Self {
            map_fast_to_squeeze_period: true,
            map_slow_to_trend_period: true,
            map_slow_to_er_period: true,
            map_slow_to_z_period: true,
            squeeze_period: default_asr_squeeze_period(),
            squeeze_bb_mult: default_asr_squeeze_bb_mult(),
            squeeze_kc_mult: default_asr_squeeze_kc_mult(),
            trend_period: default_asr_trend_period(),
            angle_deg_min: default_asr_angle_deg_min(),
            angle_deg_mr: default_asr_angle_deg_mr(),
            use_er_filter: true,
            er_period: default_asr_er_period(),
            er_min: default_asr_er_min(),
            er_mr: default_asr_er_mr(),
            z_period: default_asr_z_period(),
            z_ma_type: default_asr_z_ma_type(),
            z_nbdev: default_asr_z_nbdev(),
            z_devtype: default_asr_z_devtype(),
            z_entry: default_asr_z_entry(),
            z_exit: default_asr_z_exit(),
            confirm_bars: default_asr_confirm_bars(),
            mom_fade_bars: default_asr_mom_fade_bars(),
            max_trade_bars: default_asr_max_trade_bars(),
            max_mr_trade_bars: default_asr_max_mr_trade_bars(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AsrResolvedPeriods {
    pub squeeze: u16,
    pub trend: u16,
    pub er: u16,
    pub z: u16,
}

#[derive(Clone, Debug)]
pub struct AsrFastSignals {
    pub squeeze_state: Vec<f64>,
    pub momentum: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct AsrSlowSignals {
    pub angle: Vec<f64>,
    pub er: Vec<f64>,
    pub zscore: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct AsrKaufmanStops {
    pub long: Vec<f64>,
    pub short: Vec<f64>,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug)]
pub struct AsrFastSignalsDevice {
    pub squeeze_state: DeviceMatrixF32,
    pub momentum: DeviceMatrixF32,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug)]
pub struct AsrSlowSignalsDevice {
    pub angle: DeviceMatrixF32,
    pub er: DeviceMatrixF32,
    pub zscore: DeviceMatrixF32,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
enum AsrDeviceMatrixOwner {
    Dispatch(DeviceMatrixF32),
    Raw(DeviceArrayF32),
    Er(DeviceArrayF32Er),
}

#[cfg(feature = "cuda")]
pub struct AsrOwnedDeviceMatrix {
    pub device_ptr: u64,
    pub rows: usize,
    pub cols: usize,
    owner: AsrDeviceMatrixOwner,
}

#[cfg(feature = "cuda")]
impl std::fmt::Debug for AsrOwnedDeviceMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsrOwnedDeviceMatrix")
            .field("device_ptr", &self.device_ptr)
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .finish()
    }
}

#[cfg(feature = "cuda")]
impl AsrOwnedDeviceMatrix {
    fn from_dispatch(m: DeviceMatrixF32) -> Self {
        let device_ptr = m.device_ptr;
        let rows = m.rows;
        let cols = m.cols;
        Self {
            device_ptr,
            rows,
            cols,
            owner: AsrDeviceMatrixOwner::Dispatch(m),
        }
    }

    fn from_raw(m: DeviceArrayF32) -> Self {
        let device_ptr = m.device_ptr();
        let rows = m.rows;
        let cols = m.cols;
        Self {
            device_ptr,
            rows,
            cols,
            owner: AsrDeviceMatrixOwner::Raw(m),
        }
    }

    fn from_er(m: DeviceArrayF32Er) -> Self {
        let device_ptr = m.device_ptr();
        let rows = m.rows;
        let cols = m.cols;
        Self {
            device_ptr,
            rows,
            cols,
            owner: AsrDeviceMatrixOwner::Er(m),
        }
    }

    fn keepalive_tag(&self) -> u8 {
        match self.owner {
            AsrDeviceMatrixOwner::Dispatch(_) => 0,
            AsrDeviceMatrixOwner::Raw(_) => 1,
            AsrDeviceMatrixOwner::Er(_) => 2,
        }
    }
}

#[cfg(feature = "cuda")]
pub struct AsrKaufmanStopsDevice {
    pub long: AsrOwnedDeviceMatrix,
    pub short: AsrOwnedDeviceMatrix,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
pub struct AsrPeriodSweepDevice {
    pub matrix: AsrOwnedDeviceMatrix,
    pub period_start: u16,
    pub period_end: u16,
    pub period_step: u16,
}

#[cfg(feature = "cuda")]
impl AsrPeriodSweepDevice {
    pub fn row_index_for_period(&self, period: u16) -> Option<usize> {
        if self.period_step == 0 || period < self.period_start || period > self.period_end {
            return None;
        }
        let span = period.saturating_sub(self.period_start);
        let step = self.period_step;
        if span % step != 0 {
            return None;
        }
        let idx = usize::from(span / step);
        (idx < self.matrix.rows).then_some(idx)
    }

    pub fn row_ptr_for_period(&self, period: u16) -> Option<u64> {
        let _ = self.matrix.keepalive_tag();
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

pub fn validate_asr_params(cfg: &AsrStrategyParams) -> Result<(), String> {
    if cfg.squeeze_period == 0 {
        return Err("ASR squeeze period must be > 0".to_string());
    }
    if cfg.trend_period == 0 {
        return Err("ASR trend period must be > 0".to_string());
    }
    if cfg.er_period == 0 {
        return Err("ASR ER period must be > 0".to_string());
    }
    if cfg.z_period == 0 {
        return Err("ASR z-score period must be > 0".to_string());
    }
    if !cfg.squeeze_bb_mult.is_finite() || cfg.squeeze_bb_mult <= 0.0 {
        return Err("ASR BB multiplier must be finite and > 0".to_string());
    }
    if !cfg.squeeze_kc_mult.is_finite() || cfg.squeeze_kc_mult <= 0.0 {
        return Err("ASR KC multiplier must be finite and > 0".to_string());
    }
    if !cfg.angle_deg_min.is_finite() || cfg.angle_deg_min < 0.0 {
        return Err("ASR angle_deg_min must be finite and >= 0".to_string());
    }
    if !cfg.angle_deg_mr.is_finite() || cfg.angle_deg_mr < 0.0 {
        return Err("ASR angle_deg_mr must be finite and >= 0".to_string());
    }
    if !cfg.er_min.is_finite() || cfg.er_min < 0.0 {
        return Err("ASR er_min must be finite and >= 0".to_string());
    }
    if !cfg.er_mr.is_finite() || cfg.er_mr < 0.0 {
        return Err("ASR er_mr must be finite and >= 0".to_string());
    }
    if !cfg.z_entry.is_finite() || cfg.z_entry < 0.0 {
        return Err("ASR z_entry must be finite and >= 0".to_string());
    }
    if !cfg.z_exit.is_finite() || cfg.z_exit < 0.0 {
        return Err("ASR z_exit must be finite and >= 0".to_string());
    }
    if cfg.z_exit > cfg.z_entry {
        return Err("ASR z_exit must be <= z_entry".to_string());
    }
    if cfg.mom_fade_bars == 0 {
        return Err("ASR mom_fade_bars must be > 0".to_string());
    }
    if cfg.max_trade_bars == 0 {
        return Err("ASR max_trade_bars must be > 0".to_string());
    }
    if cfg.max_mr_trade_bars == 0 {
        return Err("ASR max_mr_trade_bars must be > 0".to_string());
    }
    if cfg.z_ma_type.trim().is_empty() {
        return Err("ASR z_ma_type cannot be empty".to_string());
    }
    Ok(())
}

#[cfg(feature = "cuda")]
thread_local! {
    static ASR_ROUTE_DIAGNOSTICS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[cfg(feature = "cuda")]
pub(crate) fn clear_asr_route_diagnostics() {
    ASR_ROUTE_DIAGNOSTICS.with(|cell| cell.borrow_mut().clear());
}

#[cfg(feature = "cuda")]
pub(crate) fn take_asr_route_diagnostics() -> Option<Vec<String>> {
    ASR_ROUTE_DIAGNOSTICS.with(|cell| {
        let mut diagnostics = cell.borrow_mut();
        if diagnostics.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut *diagnostics))
        }
    })
}

#[cfg(not(feature = "cuda"))]
pub(crate) fn clear_asr_route_diagnostics() {}

#[cfg(not(feature = "cuda"))]
pub(crate) fn take_asr_route_diagnostics() -> Option<Vec<String>> {
    None
}

pub fn resolve_asr_periods(
    cfg: &AsrStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> AsrResolvedPeriods {
    AsrResolvedPeriods {
        squeeze: if cfg.map_fast_to_squeeze_period {
            fast_len.max(1)
        } else {
            cfg.squeeze_period.max(1)
        },
        trend: if cfg.map_slow_to_trend_period {
            slow_len.max(1)
        } else {
            cfg.trend_period.max(1)
        },
        er: if cfg.map_slow_to_er_period {
            slow_len.max(1)
        } else {
            cfg.er_period.max(1)
        },
        z: if cfg.map_slow_to_z_period {
            slow_len.max(1)
        } else {
            cfg.z_period.max(1)
        },
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
            Err("ASR expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn cuda_series_to_device_f32(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<(DeviceMatrixF32, Option<usize>), String> {
    match out.series {
        DynamicIndicatorCudaSeries::DeviceF32(v) => Ok((v, out.warmup)),
        DynamicIndicatorCudaSeries::HostF32(_) => {
            Err("ASR expected CUDA DeviceF32 output".to_string())
        }
    }
}

fn backend_prefers_gpu(backend: &Backend) -> bool {
    !matches!(backend, Backend::CpuOnly)
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
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("ASR indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    let _prefer_gpu = backend_prefers_gpu(backend)
        && descriptor.prefers_gpu_by_speedup
        && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);

    #[cfg(feature = "cuda")]
    {
        if _prefer_gpu {
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

    let cpu = compute_dynamic_indicator_cpu(
        candles,
        ma_source,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
    )
    .map_err(|e| e.to_string())?;
    Ok(series_to_f64(cpu))
}

#[cfg(feature = "cuda")]
fn compute_indicator_f64_cuda_host(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
) -> Result<Vec<f64>, String> {
    let out = compute_dynamic_indicator_cuda_with_inputs(
        prepared_inputs,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
    )
    .map_err(|e| e.to_string())?;
    cuda_series_to_f64(out)
}

#[cfg(feature = "cuda")]
fn compute_indicator_f32_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
) -> Result<(DeviceMatrixF32, Option<usize>), String> {
    let out = compute_dynamic_indicator_cuda_with_inputs(
        prepared_inputs,
        indicator_id,
        Some(output_id),
        params,
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| e.to_string())?;
    cuda_series_to_device_f32(out)
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
) -> Result<AsrPeriodSweepDevice, String> {
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
        format!("ASR indicator '{indicator_id}' is not registered for dynamic dispatch")
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
    let (matrix, _warmup) = cuda_series_to_device_f32(out)?;
    Ok(AsrPeriodSweepDevice {
        matrix: AsrOwnedDeviceMatrix::from_dispatch(matrix),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_asr_ttm_squeeze_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let sweep = TtmSqueezeBatchRange {
        length: (usize::from(start), usize::from(end), usize::from(step)),
        bb_mult: (cfg.squeeze_bb_mult, cfg.squeeze_bb_mult, 0.0),
        kc_high: (1.0, 1.0, 0.0),
        kc_mid: (cfg.squeeze_kc_mult, cfg.squeeze_kc_mult, 0.0),
        kc_low: (2.0, 2.0, 0.0),
    };
    let wrapper = CudaTtmSqueeze::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.hlc_first_valid();
    let (_momentum, squeeze) = wrapper
        .ttm_squeeze_batch_dev_from_device_inputs(
            prepared_inputs.high_device().buffer(),
            prepared_inputs.low_device().buffer(),
            prepared_inputs.close_device().buffer(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| {
            wrapper.ttm_squeeze_batch_dev(
                prepared_inputs.high(),
                prepared_inputs.low(),
                prepared_inputs.close(),
                &sweep,
            )
        })
        .map_err(|e| e.to_string())?;

    Ok(AsrPeriodSweepDevice {
        matrix: AsrOwnedDeviceMatrix::from_raw(squeeze),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_asr_squeeze_momentum_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let sweep = SqueezeMomentumBatchRange {
        length_bb: (usize::from(start), usize::from(start), 0),
        mult_bb: (cfg.squeeze_bb_mult, cfg.squeeze_bb_mult, 0.0),
        length_kc: (usize::from(start), usize::from(end), usize::from(step)),
        mult_kc: (cfg.squeeze_kc_mult, cfg.squeeze_kc_mult, 0.0),
    };
    let wrapper = CudaSqueezeMomentum::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.hlc_first_valid();
    let (_squeeze, momentum, _signal) = wrapper
        .squeeze_momentum_batch_dev_from_device_inputs(
            prepared_inputs.high_device().buffer(),
            prepared_inputs.low_device().buffer(),
            prepared_inputs.close_device().buffer(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| {
            wrapper.squeeze_momentum_batch_dev(
                prepared_inputs.high(),
                prepared_inputs.low(),
                prepared_inputs.close(),
                &sweep,
            )
        })
        .map_err(|e| e.to_string())?;

    Ok(AsrPeriodSweepDevice {
        matrix: AsrOwnedDeviceMatrix::from_raw(momentum),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_asr_linearreg_angle_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let sweep = Linearreg_angleBatchRange {
        period: (usize::from(start), usize::from(end), usize::from(step)),
    };
    let wrapper = CudaLinearregAngle::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.source_first_valid();
    let matrix = wrapper
        .linearreg_angle_batch_dev_from_device_prices(
            prepared_inputs.source_device().buffer(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| wrapper.linearreg_angle_batch_dev(prepared_inputs.source(), &sweep))
        .map_err(|e| e.to_string())?;

    Ok(AsrPeriodSweepDevice {
        matrix: AsrOwnedDeviceMatrix::from_raw(matrix),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn asr_zscore_direct_ma_type_supported(ma_type: &str) -> bool {
    ma_type.eq_ignore_ascii_case("sma") || ma_type.eq_ignore_ascii_case("ema")
}

#[cfg(feature = "cuda")]
fn compute_asr_er_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let sweep = ErBatchRange {
        period: (usize::from(start), usize::from(end), usize::from(step)),
    };
    let wrapper = CudaEr::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.source_first_valid();
    let matrix = wrapper
        .er_batch_dev_from_device_prices(
            prepared_inputs.source_device().buffer(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| wrapper.er_batch_dev(prepared_inputs.source(), &sweep))
        .map_err(|e| e.to_string())?;

    Ok(AsrPeriodSweepDevice {
        matrix: AsrOwnedDeviceMatrix::from_er(matrix),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_asr_zscore_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let sweep = ZscoreBatchRange {
        period: (usize::from(start), usize::from(end), usize::from(step)),
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

    Ok(AsrPeriodSweepDevice {
        matrix: AsrOwnedDeviceMatrix::from_raw(matrix),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_indicator_f64_cuda_host_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: Option<&str>,
    input_kind: IndicatorInputKind,
    params: &[IndicatorParamKV],
) -> Result<Vec<f64>, String> {
    let out = compute_dynamic_indicator_cuda_direct_with_inputs(
        prepared_inputs,
        indicator_id,
        output_id,
        input_kind,
        params,
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
    )
    .map_err(|e| e.to_string())?;
    cuda_series_to_f64(out)
}

#[cfg(feature = "cuda")]
fn compute_indicator_f32_cuda_device_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: Option<&str>,
    input_kind: IndicatorInputKind,
    params: &[IndicatorParamKV],
) -> Result<(DeviceMatrixF32, Option<usize>), String> {
    let out = compute_dynamic_indicator_cuda_direct_with_inputs(
        prepared_inputs,
        indicator_id,
        output_id,
        input_kind,
        params,
        Kernel::Auto,
        vector_ta::indicators::dispatch::CudaOutputTarget::DeviceF32,
    )
    .map_err(|e| e.to_string())?;
    cuda_series_to_device_f32(out)
}

fn first_finite_idx_local(values: &[f64]) -> Option<usize> {
    values.iter().position(|v| v.is_finite())
}

#[cfg(feature = "cuda")]
pub fn compute_asr_fast_signals_cuda_host(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    squeeze_period: u16,
) -> Result<AsrFastSignals, String> {
    let squeeze_period_i64 = i64::from(squeeze_period.max(1));
    let ttm_params = vec![
        IndicatorParamKV {
            key: "length".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "bb_mult".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
    ];
    let squeeze_momentum_params = vec![
        IndicatorParamKV {
            key: "length_bb".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "mult_bb".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
        IndicatorParamKV {
            key: "length_kc".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "mult_kc".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_kc_mult),
        },
    ];

    let squeeze_state =
        compute_indicator_f64_cuda_host(prepared_inputs, "ttm_squeeze", "squeeze", &ttm_params)?;
    let momentum = compute_indicator_f64_cuda_host(
        prepared_inputs,
        "squeeze_momentum",
        "momentum",
        &squeeze_momentum_params,
    )?;

    let first_valid = first_finite_idx_local(&squeeze_state)
        .zip(first_finite_idx_local(&momentum))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "ASR fast signals are all non-finite".to_string())?;

    Ok(AsrFastSignals {
        squeeze_state,
        momentum,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_asr_slow_signals_cuda_host(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    trend_period: u16,
    er_period: u16,
    z_period: u16,
) -> Result<AsrSlowSignals, String> {
    let trend_params = vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(trend_period.max(1))),
    }];
    let er_params = vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(er_period.max(1))),
    }];
    let z_params = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(i64::from(z_period.max(1))),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(cfg.z_ma_type.clone()),
        },
        IndicatorParamKV {
            key: "nbdev".to_string(),
            value: IndicatorParamValue::Float(cfg.z_nbdev),
        },
        IndicatorParamKV {
            key: "devtype".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.z_devtype)),
        },
    ];

    let angle = compute_indicator_f64_cuda_host(
        prepared_inputs,
        "linearreg_angle",
        "value",
        &trend_params,
    )?;
    let er = compute_indicator_f64_cuda_host(prepared_inputs, "er", "value", &er_params)?;
    let zscore = compute_indicator_f64_cuda_host(prepared_inputs, "zscore", "value", &z_params)?;

    let first_valid = first_finite_idx_local(&angle)
        .zip(first_finite_idx_local(&er))
        .zip(first_finite_idx_local(&zscore))
        .map(|((a, b), c)| a.max(b).max(c))
        .ok_or_else(|| "ASR slow signals are all non-finite".to_string())?;

    Ok(AsrSlowSignals {
        angle,
        er,
        zscore,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_asr_kaufman_stops_cuda_host(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    strategy: &StrategyConfig,
) -> Result<AsrKaufmanStops, String> {
    let params_long = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.kaufman_stop_period().max(1) as i64),
        },
        IndicatorParamKV {
            key: "mult".to_string(),
            value: IndicatorParamValue::Float(strategy.kaufman_stop_mult()),
        },
        IndicatorParamKV {
            key: "direction".to_string(),
            value: IndicatorParamValue::EnumString("long".to_string()),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(strategy.kaufman_stop_ma_type().to_string()),
        },
    ];
    let params_short = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.kaufman_stop_period().max(1) as i64),
        },
        IndicatorParamKV {
            key: "mult".to_string(),
            value: IndicatorParamValue::Float(strategy.kaufman_stop_mult()),
        },
        IndicatorParamKV {
            key: "direction".to_string(),
            value: IndicatorParamValue::EnumString("short".to_string()),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(strategy.kaufman_stop_ma_type().to_string()),
        },
    ];

    let output_id = find_dynamic_indicator_descriptor("kaufmanstop")
        .and_then(|d| d.outputs.first())
        .map(|o| o.id.as_str());

    let direct_compute = |params: &[IndicatorParamKV]| -> Result<Vec<f64>, String> {
        let candidates = [
            IndicatorInputKind::HighLow,
            IndicatorInputKind::Candles,
            IndicatorInputKind::Ohlc,
        ];
        let mut last_err: Option<String> = None;
        for kind in candidates {
            match compute_indicator_f64_cuda_host_direct(
                prepared_inputs,
                "kaufmanstop",
                output_id,
                kind,
                params,
            ) {
                Ok(v) => return Ok(v),
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            "ASR kaufman-stop CUDA dispatch failed for all input kinds".to_string()
        }))
    };

    let long = if output_id.is_some() {
        compute_indicator_f64_cuda_host(
            prepared_inputs,
            "kaufmanstop",
            output_id.unwrap_or("value"),
            &params_long,
        )
        .or_else(|_| direct_compute(&params_long))?
    } else {
        direct_compute(&params_long)?
    };
    let short = if output_id.is_some() {
        compute_indicator_f64_cuda_host(
            prepared_inputs,
            "kaufmanstop",
            output_id.unwrap_or("value"),
            &params_short,
        )
        .or_else(|_| direct_compute(&params_short))?
    } else {
        direct_compute(&params_short)?
    };
    if long.len() != short.len() {
        return Err("ASR kaufman-stop output length mismatch".to_string());
    }

    let first_valid = first_finite_idx_local(&long)
        .zip(first_finite_idx_local(&short))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "ASR kaufman-stop series are all non-finite".to_string())?;

    Ok(AsrKaufmanStops {
        long,
        short,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_asr_fast_signals_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    squeeze_period: u16,
) -> Result<AsrFastSignalsDevice, String> {
    let squeeze_period_i64 = i64::from(squeeze_period.max(1));
    let ttm_params = vec![
        IndicatorParamKV {
            key: "length".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "bb_mult".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
    ];
    let squeeze_momentum_params = vec![
        IndicatorParamKV {
            key: "length_bb".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "mult_bb".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
        IndicatorParamKV {
            key: "length_kc".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "mult_kc".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_kc_mult),
        },
    ];

    let (squeeze_state, squeeze_warmup) =
        compute_indicator_f32_cuda_device(prepared_inputs, "ttm_squeeze", "squeeze", &ttm_params)?;
    let (momentum, momentum_warmup) = compute_indicator_f32_cuda_device(
        prepared_inputs,
        "squeeze_momentum",
        "momentum",
        &squeeze_momentum_params,
    )?;

    if squeeze_state.rows != momentum.rows || squeeze_state.cols != momentum.cols {
        return Err("ASR fast CUDA device output shape mismatch".to_string());
    }
    let first_valid = squeeze_warmup
        .unwrap_or(0)
        .max(momentum_warmup.unwrap_or(0));

    Ok(AsrFastSignalsDevice {
        squeeze_state,
        momentum,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_asr_slow_signals_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    trend_period: u16,
    er_period: u16,
    z_period: u16,
) -> Result<AsrSlowSignalsDevice, String> {
    let trend_params = vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(trend_period.max(1))),
    }];
    let er_params = vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(er_period.max(1))),
    }];
    let z_params = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(i64::from(z_period.max(1))),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(cfg.z_ma_type.clone()),
        },
        IndicatorParamKV {
            key: "nbdev".to_string(),
            value: IndicatorParamValue::Float(cfg.z_nbdev),
        },
        IndicatorParamKV {
            key: "devtype".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.z_devtype)),
        },
    ];

    let (angle, angle_warmup) = compute_indicator_f32_cuda_device(
        prepared_inputs,
        "linearreg_angle",
        "value",
        &trend_params,
    )?;
    let (er, er_warmup) =
        compute_indicator_f32_cuda_device(prepared_inputs, "er", "value", &er_params)?;
    let (zscore, z_warmup) =
        compute_indicator_f32_cuda_device(prepared_inputs, "zscore", "value", &z_params)?;

    if angle.rows != er.rows
        || angle.cols != er.cols
        || angle.rows != zscore.rows
        || angle.cols != zscore.cols
    {
        return Err("ASR slow CUDA device output shape mismatch".to_string());
    }
    let first_valid = angle_warmup
        .unwrap_or(0)
        .max(er_warmup.unwrap_or(0))
        .max(z_warmup.unwrap_or(0));

    Ok(AsrSlowSignalsDevice {
        angle,
        er,
        zscore,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_asr_kaufman_stops_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    strategy: &StrategyConfig,
    device_id: u32,
) -> Result<AsrKaufmanStopsDevice, String> {
    let params_long = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.kaufman_stop_period().max(1) as i64),
        },
        IndicatorParamKV {
            key: "mult".to_string(),
            value: IndicatorParamValue::Float(strategy.kaufman_stop_mult()),
        },
        IndicatorParamKV {
            key: "direction".to_string(),
            value: IndicatorParamValue::EnumString("long".to_string()),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(strategy.kaufman_stop_ma_type().to_string()),
        },
    ];
    let params_short = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(strategy.kaufman_stop_period().max(1) as i64),
        },
        IndicatorParamKV {
            key: "mult".to_string(),
            value: IndicatorParamValue::Float(strategy.kaufman_stop_mult()),
        },
        IndicatorParamKV {
            key: "direction".to_string(),
            value: IndicatorParamValue::EnumString("short".to_string()),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(strategy.kaufman_stop_ma_type().to_string()),
        },
    ];

    let sweep_for_direction =
        |direction: &str| vector_ta::indicators::kaufmanstop::KaufmanstopBatchRange {
            period: (
                strategy.kaufman_stop_period().max(1) as usize,
                strategy.kaufman_stop_period().max(1) as usize,
                0,
            ),
            mult: (
                strategy.kaufman_stop_mult(),
                strategy.kaufman_stop_mult(),
                0.0,
            ),
            direction: (direction.to_string(), direction.to_string(), 0.0),
            ma_type: (
                strategy.kaufman_stop_ma_type().to_string(),
                strategy.kaufman_stop_ma_type().to_string(),
                0.0,
            ),
        };

    let direct_compute = |direction: &str| -> Result<AsrOwnedDeviceMatrix, String> {
        let wrapper = CudaKaufmanstop::new(device_id as usize).map_err(|e| e.to_string())?;
        let first_valid = prepared_inputs.high_low_first_valid();
        let sweep = sweep_for_direction(direction);
        let matrix = wrapper
            .kaufmanstop_batch_dev_from_device_inputs(
                prepared_inputs.high_device().buffer(),
                prepared_inputs.low_device().buffer(),
                prepared_inputs.len(),
                first_valid,
                &sweep,
            )
            .map(|(matrix, _)| AsrOwnedDeviceMatrix::from_raw(matrix))
            .or_else(|_| {
                let params = if direction.eq_ignore_ascii_case("long") {
                    &params_long
                } else {
                    &params_short
                };
                let output_id = find_dynamic_indicator_descriptor("kaufmanstop")
                    .and_then(|d| d.outputs.first())
                    .map(|o| o.id.as_str());
                if let Some(output_id) = output_id {
                    compute_indicator_f32_cuda_device(
                        prepared_inputs,
                        "kaufmanstop",
                        output_id,
                        params,
                    )
                    .map(|(matrix, _warmup)| AsrOwnedDeviceMatrix::from_dispatch(matrix))
                } else {
                    let candidates = [
                        IndicatorInputKind::HighLow,
                        IndicatorInputKind::Candles,
                        IndicatorInputKind::Ohlc,
                    ];
                    let mut last_err: Option<String> = None;
                    for kind in candidates {
                        match compute_indicator_f32_cuda_device_direct(
                            prepared_inputs,
                            "kaufmanstop",
                            output_id,
                            kind,
                            params,
                        ) {
                            Ok((matrix, _warmup)) => {
                                return Ok(AsrOwnedDeviceMatrix::from_dispatch(matrix));
                            }
                            Err(err) => {
                                last_err = Some(err);
                            }
                        }
                    }
                    Err(last_err.unwrap_or_else(|| {
                        "ASR kaufman-stop CUDA device dispatch failed for all input kinds"
                            .to_string()
                    }))
                }
            })?;
        Ok(matrix)
    };

    let long = direct_compute("long")?;
    let short = direct_compute("short")?;

    if long.rows != short.rows || long.cols != short.cols {
        return Err("ASR kaufman-stop CUDA device shape mismatch".to_string());
    }
    let first_valid = prepared_inputs.high_low_first_valid()
        + usize::from(strategy.kaufman_stop_period().saturating_sub(1));

    Ok(AsrKaufmanStopsDevice {
        long,
        short,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_asr_ttm_squeeze_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let sweep = compute_asr_ttm_squeeze_sweep_cuda_direct(
        prepared_inputs,
        cfg,
        period_start,
        period_end,
        period_step,
        device_id,
    )?;
    let expected_rows = usize::from(
        (sweep
            .period_end
            .saturating_sub(sweep.period_start)
            .saturating_div(sweep.period_step))
        .saturating_add(1),
    );
    if sweep.matrix.rows != expected_rows {
        return Err(format!(
            "ASR ttm_squeeze sweep rows mismatch: expected {}, got {}",
            expected_rows, sweep.matrix.rows
        ));
    }
    Ok(sweep)
}

#[cfg(feature = "cuda")]
pub fn compute_asr_squeeze_momentum_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let sweep = compute_asr_squeeze_momentum_sweep_cuda_direct(
        prepared_inputs,
        cfg,
        period_start,
        period_end,
        period_step,
        device_id,
    )?;
    let expected_rows = usize::from(
        (sweep
            .period_end
            .saturating_sub(sweep.period_start)
            .saturating_div(sweep.period_step))
        .saturating_add(1),
    );
    if sweep.matrix.rows != expected_rows {
        return Err(format!(
            "ASR squeeze_momentum sweep rows mismatch: expected {}, got {}",
            expected_rows, sweep.matrix.rows
        ));
    }
    Ok(sweep)
}

#[cfg(feature = "cuda")]
pub fn compute_asr_linearreg_angle_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let sweep = compute_asr_linearreg_angle_sweep_cuda_direct(
        prepared_inputs,
        period_start,
        period_end,
        period_step,
        device_id,
    )?;
    let expected_rows = usize::from(
        (sweep
            .period_end
            .saturating_sub(sweep.period_start)
            .saturating_div(sweep.period_step))
        .saturating_add(1),
    );
    if sweep.matrix.rows != expected_rows {
        return Err(format!(
            "ASR linearreg_angle sweep rows mismatch: expected {}, got {}",
            expected_rows, sweep.matrix.rows
        ));
    }
    Ok(sweep)
}

#[cfg(feature = "cuda")]
pub fn compute_asr_er_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let sweep = compute_asr_er_sweep_cuda_direct(
        prepared_inputs,
        period_start,
        period_end,
        period_step,
        device_id,
    )?;
    let expected_rows = usize::from(
        (sweep
            .period_end
            .saturating_sub(sweep.period_start)
            .saturating_div(sweep.period_step))
        .saturating_add(1),
    );
    if sweep.matrix.rows != expected_rows {
        return Err(format!(
            "ASR er sweep rows mismatch: expected {}, got {}",
            expected_rows, sweep.matrix.rows
        ));
    }
    Ok(sweep)
}

#[cfg(feature = "cuda")]
pub fn compute_asr_zscore_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &AsrStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<AsrPeriodSweepDevice, String> {
    let sweep = if cfg.z_devtype == 0 && asr_zscore_direct_ma_type_supported(&cfg.z_ma_type) {
        compute_asr_zscore_sweep_cuda_direct(
            prepared_inputs,
            cfg,
            period_start,
            period_end,
            period_step,
            device_id,
        )?
    } else {
        let base_params = vec![
            IndicatorParamKV {
                key: "ma_type".to_string(),
                value: IndicatorParamValue::EnumString(cfg.z_ma_type.clone()),
            },
            IndicatorParamKV {
                key: "nbdev".to_string(),
                value: IndicatorParamValue::Float(cfg.z_nbdev),
            },
            IndicatorParamKV {
                key: "devtype".to_string(),
                value: IndicatorParamValue::Int(i64::from(cfg.z_devtype)),
            },
        ];
        compute_indicator_period_sweep_cuda_device(
            prepared_inputs,
            "zscore",
            "value",
            "period",
            period_start,
            period_end,
            period_step,
            base_params.as_slice(),
        )?
    };
    let expected_rows = usize::from(
        (sweep
            .period_end
            .saturating_sub(sweep.period_start)
            .saturating_div(sweep.period_step))
        .saturating_add(1),
    );
    if sweep.matrix.rows != expected_rows {
        return Err(format!(
            "ASR zscore sweep rows mismatch: expected {}, got {}",
            expected_rows, sweep.matrix.rows
        ));
    }
    Ok(sweep)
}

pub fn compute_asr_fast_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &AsrStrategyParams,
    squeeze_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<AsrFastSignals, String> {
    let squeeze_period_i64 = i64::from(squeeze_period.max(1));

    let ttm_params = vec![
        IndicatorParamKV {
            key: "length".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "bb_mult".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
    ];

    let squeeze_momentum_params = vec![
        IndicatorParamKV {
            key: "length_bb".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "mult_bb".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
        IndicatorParamKV {
            key: "length_kc".to_string(),
            value: IndicatorParamValue::Int(squeeze_period_i64),
        },
        IndicatorParamKV {
            key: "mult_kc".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_kc_mult),
        },
    ];

    let squeeze_state = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "ttm_squeeze",
        "squeeze",
        &ttm_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let momentum = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "squeeze_momentum",
        "momentum",
        &squeeze_momentum_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let first_valid = first_finite_idx_local(&squeeze_state)
        .zip(first_finite_idx_local(&momentum))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "ASR fast signals are all non-finite".to_string())?;

    Ok(AsrFastSignals {
        squeeze_state,
        momentum,
        first_valid,
    })
}

pub fn compute_asr_slow_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &AsrStrategyParams,
    trend_period: u16,
    er_period: u16,
    z_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<AsrSlowSignals, String> {
    let trend_params = vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(trend_period.max(1))),
    }];
    let er_params = vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(er_period.max(1))),
    }];
    let z_params = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(i64::from(z_period.max(1))),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString(cfg.z_ma_type.clone()),
        },
        IndicatorParamKV {
            key: "nbdev".to_string(),
            value: IndicatorParamValue::Float(cfg.z_nbdev),
        },
        IndicatorParamKV {
            key: "devtype".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.z_devtype)),
        },
    ];

    let angle = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "linearreg_angle",
        "value",
        &trend_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let er = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "er",
        "value",
        &er_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let zscore = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "zscore",
        "value",
        &z_params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let first_valid = first_finite_idx_local(&angle)
        .zip(first_finite_idx_local(&er))
        .zip(first_finite_idx_local(&zscore))
        .map(|((a, b), c)| a.max(b).max(c))
        .ok_or_else(|| "ASR slow signals are all non-finite".to_string())?;

    Ok(AsrSlowSignals {
        angle,
        er,
        zscore,
        first_valid,
    })
}

pub fn compute_asr_kaufman_stops(
    candles: &Candles,
    strategy: &StrategyConfig,
) -> Result<AsrKaufmanStops, String> {
    let params_long = vector_ta::indicators::kaufmanstop::KaufmanstopParams {
        period: Some(strategy.kaufman_stop_period()),
        mult: Some(strategy.kaufman_stop_mult()),
        direction: Some("long".to_string()),
        ma_type: Some(strategy.kaufman_stop_ma_type().to_string()),
    };
    let params_short = vector_ta::indicators::kaufmanstop::KaufmanstopParams {
        period: Some(strategy.kaufman_stop_period()),
        mult: Some(strategy.kaufman_stop_mult()),
        direction: Some("short".to_string()),
        ma_type: Some(strategy.kaufman_stop_ma_type().to_string()),
    };

    let long = vector_ta::indicators::kaufmanstop::kaufmanstop(
        &vector_ta::indicators::kaufmanstop::KaufmanstopInput::from_candles(candles, params_long),
    )
    .map_err(|e| format!("ASR kaufman-stop long failed: {e}"))?
    .values;
    let short = vector_ta::indicators::kaufmanstop::kaufmanstop(
        &vector_ta::indicators::kaufmanstop::KaufmanstopInput::from_candles(candles, params_short),
    )
    .map_err(|e| format!("ASR kaufman-stop short failed: {e}"))?
    .values;

    if long.len() != short.len() {
        return Err("ASR kaufman-stop output length mismatch".to_string());
    }

    let first_valid = first_finite_idx_local(&long)
        .zip(first_finite_idx_local(&short))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "ASR kaufman-stop series are all non-finite".to_string())?;

    Ok(AsrKaufmanStops {
        long,
        short,
        first_valid,
    })
}

pub fn compute_asr_start_t(
    first_valid_close: usize,
    first_valid_source: usize,
    first_valid_fast: usize,
    first_valid_slow: usize,
    first_valid_kaufman: usize,
    max_period: usize,
    trade_on_next_bar: bool,
) -> usize {
    let mut first_valid = first_valid_close.max(first_valid_source);
    first_valid = first_valid.max(first_valid_fast);
    first_valid = first_valid.max(first_valid_slow);
    first_valid = first_valid.max(first_valid_kaufman);

    let t_valid = first_valid.saturating_add(max_period.max(1).saturating_sub(1));
    if trade_on_next_bar {
        t_valid.saturating_add(1)
    } else {
        t_valid
    }
}

pub fn build_asr_desired_positions(
    close: &[f64],
    fast: &AsrFastSignals,
    slow: &AsrSlowSignals,
    kaufman: &AsrKaufmanStops,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &AsrStrategyParams,
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
    let mut mode: Option<AsrEntryMode> = None;
    let mut bars_in_trade: usize = 0;
    let mut mom_fade_count: usize = 0;
    let mut release_confirm_left: usize = 0;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let squeeze_now = fast.squeeze_state.get(sig_t).copied().unwrap_or(f64::NAN);
        let squeeze_prev = fast.squeeze_state.get(prev_t).copied().unwrap_or(f64::NAN);
        let mom_now = fast.momentum.get(sig_t).copied().unwrap_or(f64::NAN);
        let mom_prev = fast.momentum.get(prev_t).copied().unwrap_or(f64::NAN);
        let mom_delta = if mom_now.is_finite() && mom_prev.is_finite() {
            mom_now - mom_prev
        } else {
            f64::NAN
        };

        let ang_now = slow.angle.get(sig_t).copied().unwrap_or(f64::NAN);
        let er_now = slow.er.get(sig_t).copied().unwrap_or(f64::NAN);
        let z_now = slow.zscore.get(sig_t).copied().unwrap_or(f64::NAN);

        let squeeze_on_now = squeeze_now.is_finite() && squeeze_now > 0.5;
        let squeeze_on_prev = squeeze_prev.is_finite() && squeeze_prev > 0.5;
        let squeeze_release = squeeze_on_prev && !squeeze_on_now;

        if squeeze_release {
            release_confirm_left = usize::from(cfg.confirm_bars);
        } else if release_confirm_left > 0 {
            release_confirm_left = release_confirm_left.saturating_sub(1);
        }
        let release_confirmed = squeeze_release || release_confirm_left == 0;

        let trend_ok_long = ang_now.is_finite()
            && ang_now >= cfg.angle_deg_min
            && (!cfg.use_er_filter || (er_now.is_finite() && er_now >= cfg.er_min));
        let trend_ok_short = ang_now.is_finite()
            && ang_now <= -cfg.angle_deg_min
            && (!cfg.use_er_filter || (er_now.is_finite() && er_now >= cfg.er_min));

        let trend_weak = ang_now.is_finite()
            && ang_now.abs() <= cfg.angle_deg_mr
            && (!cfg.use_er_filter || (er_now.is_finite() && er_now <= cfg.er_mr));

        if pos == 0 {
            bars_in_trade = 0;
            mom_fade_count = 0;

            let mut next_pos = 0i32;
            let mut next_mode: Option<AsrEntryMode> = None;

            if release_confirmed {
                if trend_ok_long
                    && mom_now.is_finite()
                    && mom_now > 0.0
                    && mom_delta.is_finite()
                    && mom_delta > 0.0
                {
                    next_pos = 1;
                    next_mode = Some(AsrEntryMode::Trend);
                } else if !strategy.long_only
                    && trend_ok_short
                    && mom_now.is_finite()
                    && mom_now < 0.0
                    && mom_delta.is_finite()
                    && mom_delta < 0.0
                {
                    next_pos = -1;
                    next_mode = Some(AsrEntryMode::Trend);
                }
            }

            if next_pos == 0 && trend_weak && z_now.is_finite() {
                if z_now <= -cfg.z_entry {
                    next_pos = 1;
                    next_mode = Some(AsrEntryMode::MeanReversion);
                } else if !strategy.long_only && z_now >= cfg.z_entry {
                    next_pos = -1;
                    next_mode = Some(AsrEntryMode::MeanReversion);
                }
            }

            if strategy.long_only && next_pos < 0 {
                next_pos = 0;
                next_mode = None;
            }

            pos = next_pos;
            mode = next_mode;
            if pos != 0 {
                bars_in_trade = 1;
            }
        } else {
            let mut exit_now = false;

            let close_now = close[t];
            if pos > 0 {
                let stop = kaufman.long.get(sig_t).copied().unwrap_or(f64::NAN);
                if close_now.is_finite() && stop.is_finite() && close_now <= stop {
                    exit_now = true;
                }
            } else {
                let stop = kaufman.short.get(sig_t).copied().unwrap_or(f64::NAN);
                if close_now.is_finite() && stop.is_finite() && close_now >= stop {
                    exit_now = true;
                }
            }

            if !exit_now {
                match mode {
                    Some(AsrEntryMode::Trend) => {
                        let fading = if pos > 0 {
                            mom_delta.is_finite() && mom_delta < 0.0
                        } else {
                            mom_delta.is_finite() && mom_delta > 0.0
                        };
                        if fading {
                            mom_fade_count = mom_fade_count.saturating_add(1);
                        } else {
                            mom_fade_count = 0;
                        }
                        if mom_fade_count >= usize::from(cfg.mom_fade_bars) {
                            exit_now = true;
                        }
                        if bars_in_trade >= usize::from(cfg.max_trade_bars) {
                            exit_now = true;
                        }
                    }
                    Some(AsrEntryMode::MeanReversion) => {
                        if z_now.is_finite() && z_now.abs() <= cfg.z_exit {
                            exit_now = true;
                        }
                        if bars_in_trade >= usize::from(cfg.max_mr_trade_bars) {
                            exit_now = true;
                        }
                    }
                    None => {}
                }
            }

            if exit_now {
                pos = 0;
                mode = None;
                bars_in_trade = 0;
                mom_fade_count = 0;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        }

        desired[t] = pos;
    }

    desired
}

#[derive(Clone, Copy)]
struct ExecutionOverlayParams {
    cooldown_bars: usize,
    persistence_bars: usize,
}

impl ExecutionOverlayParams {
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

#[derive(Clone, Debug)]
struct AsrSimulation {
    metrics: Metrics,
    curves: Option<DoubleMaCurves>,
}

fn simulate_from_desired(
    candles: &Candles,
    lr: &[f64],
    desired_pos: &[i32],
    start_t: usize,
    end_t: Option<usize>,
    cfg: &StrategyConfig,
    sim_cfg: &IterativeSimConfig,
    bins: Option<usize>,
) -> AsrSimulation {
    let n_total = end_t.unwrap_or(lr.len()).min(lr.len());
    let with_curves = bins.unwrap_or(0) > 0;
    if n_total < 2
        || candles.open.len() < n_total
        || candles.high.len() < n_total
        || candles.low.len() < n_total
        || candles.close.len() < n_total
        || desired_pos.len() < n_total
        || start_t >= n_total
    {
        return AsrSimulation {
            metrics: Metrics::default(),
            curves: if with_curves {
                Some(DoubleMaCurves {
                    equity: ta_strategies::double_ma::MinMaxBins {
                        start_t,
                        end_t: n_total,
                        bins: 0,
                        minmax: Vec::new(),
                    },
                    drawdown: ta_strategies::double_ma::MinMaxBins {
                        start_t,
                        end_t: n_total,
                        bins: 0,
                        minmax: Vec::new(),
                    },
                    trades: Vec::new(),
                    trade_stats: None,
                    indicator_bars: Vec::new(),
                })
            } else {
                None
            },
        };
    }

    let stop_loss_pct = sim_cfg.stop_loss_pct.or(cfg.stop_loss_pct());
    let take_profit_pct = sim_cfg.take_profit_pct.or(cfg.take_profit_pct());
    let overlays = ExecutionOverlayParams::from_cfg(cfg);
    let risk_enabled = stop_loss_pct.is_some() || take_profit_pct.is_some();

    let mut pos: i32 = 0;
    let mut entry_px: f64 = 0.0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;
    let mut max_log_dd = 0.0_f64;
    let mut ulcer_sum_sq = 0.0_f64;
    let mut trades_count: u32 = 0;

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
    let mut trade_log_ret: f64 = 0.0;
    let mut underwater_bars: u64 = 0;
    let mut underwater_run: u32 = 0;
    let mut max_underwater_duration_bars: u32 = 0;
    let mut overlay_state = ExecutionOverlayState::default();

    let mut eq_min: Vec<f64> = Vec::new();
    let mut eq_max: Vec<f64> = Vec::new();
    let mut dd_min: Vec<f64> = Vec::new();
    let mut dd_max: Vec<f64> = Vec::new();
    let n_steps = n_total.saturating_sub(start_t);
    let bins = if with_curves {
        bins.unwrap_or(1024).clamp(1, 8192).min(n_steps.max(1))
    } else {
        0
    };
    if with_curves {
        eq_min = vec![f64::INFINITY; bins];
        eq_max = vec![f64::NEG_INFINITY; bins];
        dd_min = vec![f64::INFINITY; bins];
        dd_max = vec![f64::NEG_INFINITY; bins];
    }

    let mut trades: Vec<Trade> = Vec::new();
    let mut cur_entry_t: usize = 0;
    let mut cur_dir: i32 = 0;

    for t in start_t..n_total {
        overlay_on_bar_start(&mut overlay_state);
        let mut desired = desired_pos[t].clamp(-1, 1);
        if cfg.long_only && desired < 0 {
            desired = 0;
        }
        desired = overlay_apply_signal_persistence(desired, pos, &overlays, &mut overlay_state);
        desired = overlay_apply_entry_cooldown(desired, pos, &overlay_state);

        let lr_abs = lr[t].abs();
        let log_cost = trade_log_cost(cfg, sim_cfg, lr_abs);

        if desired != pos {
            if pos == 0 || desired == 0 {
                if log_cost != 0.0 {
                    log_eq += log_cost;
                }
                trades_count = trades_count.saturating_add(1);
                if desired != 0 {
                    entry_px = if t > 0 {
                        candles.close[t - 1]
                    } else {
                        candles.open[t]
                    };
                    trade_log_ret = if log_cost != 0.0 { log_cost } else { 0.0 };
                    cur_entry_t = t;
                    cur_dir = desired;
                } else {
                    entry_px = 0.0;
                    if cur_dir != 0 {
                        trades.push(Trade {
                            entry_t: cur_entry_t,
                            exit_t: t,
                            direction: cur_dir,
                            pnl: trade_log_ret.exp() - 1.0,
                            bars: t.saturating_sub(cur_entry_t) as u32,
                            open: false,
                        });
                    }
                    cur_dir = 0;
                    trade_log_ret = 0.0;
                    overlay_on_trade_close(&mut overlay_state, &overlays);
                }
                pos = desired;
            } else if !cfg.allow_flip {
                if log_cost != 0.0 {
                    log_eq += log_cost;
                }
                trades_count = trades_count.saturating_add(1);
                if cur_dir != 0 {
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: trade_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                }
                pos = 0;
                entry_px = 0.0;
                cur_dir = 0;
                trade_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            } else {
                if log_cost != 0.0 {
                    log_eq += 2.0 * log_cost;
                }
                trades_count = trades_count.saturating_add(2);
                if cur_dir != 0 {
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: trade_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                }
                pos = desired;
                entry_px = if t > 0 {
                    candles.close[t - 1]
                } else {
                    candles.open[t]
                };
                trade_log_ret = if log_cost != 0.0 { log_cost } else { 0.0 };
                cur_entry_t = t;
                cur_dir = desired;
            }
        }

        let mut step_log = if pos == 0 { 0.0 } else { (pos as f64) * lr[t] };
        let pos_bar = pos;

        if pos != 0 {
            if !entry_px.is_finite() || entry_px <= 0.0 {
                entry_px = if t > 0 {
                    candles.close[t - 1]
                } else {
                    candles.open[t]
                };
            }
            if let Some(fill_px) = fill_price_from_intrabar(
                pos,
                entry_px,
                candles.high[t],
                candles.low[t],
                stop_loss_pct,
                take_profit_pct,
                sim_cfg.intrabar_rule,
            ) {
                let prev_px = if t > 0 {
                    candles.close[t - 1]
                } else {
                    candles.open[t]
                };
                if prev_px.is_finite() && prev_px > 0.0 && fill_px.is_finite() && fill_px > 0.0 {
                    let bar_log = (fill_px / prev_px).ln();
                    step_log = if pos > 0 { bar_log } else { -bar_log };
                }
                if log_cost != 0.0 {
                    log_eq += log_cost;
                    trade_log_ret += log_cost;
                }
                trade_log_ret += step_log;
                trades_count = trades_count.saturating_add(1);
                if cur_dir != 0 {
                    trades.push(Trade {
                        entry_t: cur_entry_t,
                        exit_t: t,
                        direction: cur_dir,
                        pnl: trade_log_ret.exp() - 1.0,
                        bars: t.saturating_sub(cur_entry_t) as u32,
                        open: false,
                    });
                }
                pos = 0;
                entry_px = 0.0;
                cur_dir = 0;
                trade_log_ret = 0.0;
                overlay_on_trade_close(&mut overlay_state, &overlays);
            }
        }

        log_eq += step_log;
        if risk_enabled && pos_bar != 0 && pos != 0 {
            trade_log_ret += step_log;
        }

        let step_r = step_log.exp_m1();
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

        if with_curves {
            let eq = log_eq.exp();
            let dd = if dd_log > 0.0 {
                -(-dd_log).exp_m1()
            } else {
                0.0
            };
            let i = t - start_t;
            let b = (i.saturating_mul(bins)) / n_steps.max(1);
            let b = b.min(bins.saturating_sub(1));
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

        count = count.saturating_add(1);
        let delta = step_r - mean;
        mean += delta / count as f64;
        let delta2 = step_r - mean;
        m2 += delta * delta2;

        let excess_r = step_r - cfg.hurdle_tau.unwrap_or(0.0);
        if excess_r < 0.0 {
            count_neg = count_neg.saturating_add(1);
            sum_neg_sq += excess_r * excess_r;
            let neg_mag = -excess_r;
            sum_neg_cu += neg_mag * neg_mag * neg_mag;
            sum_neg_abs_r += neg_mag;
        } else {
            sum_pos_r += excess_r;
        }
        if pos_bar > 0 {
            long_obs = long_obs.saturating_add(1);
            long_sum_r += step_r;
        } else if pos_bar < 0 {
            short_obs = short_obs.saturating_add(1);
            short_sum_r += step_r;
        }

        sum_abs_pos += (pos as i64).abs();
        sum_pos += pos as i64;
    }

    if cur_dir != 0 {
        let exit_t = n_total.saturating_sub(1);
        trades.push(Trade {
            entry_t: cur_entry_t,
            exit_t,
            direction: cur_dir,
            pnl: trade_log_ret.exp() - 1.0,
            bars: exit_t.saturating_sub(cur_entry_t) as u32,
            open: true,
        });
    }

    const EPS_DENOM: f64 = 1e-12;
    let obs_u64 = (n_total - start_t) as u64;
    let (
        sharpe,
        volatility,
        sortino,
        downside_dev,
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
    ) = {
        const CLAMP_MAX: f64 = 1e6;

        let variance = if count > 1 {
            m2 / (count as f64 - 1.0)
        } else {
            0.0
        };
        let std = variance.sqrt();
        let sharpe = if std > 0.0 { mean / std } else { 0.0 };
        let denom = (count as f64).max(1.0);
        let downside_dev = (sum_neg_sq / denom).sqrt();
        let mean_excess = mean - cfg.hurdle_tau.unwrap_or(0.0);
        let sortino_raw = if downside_dev > 0.0 {
            mean_excess / downside_dev
        } else if mean_excess > 0.0 {
            CLAMP_MAX
        } else if mean_excess < 0.0 {
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
        let pnl = log_eq.exp() - 1.0;
        let max_dd = 1.0 - (-max_log_dd).exp();
        let ulcer_index = (ulcer_sum_sq / (obs_u64 as f64).max(1.0)).sqrt();
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
        let downside_frequency = count_neg as f64 / (obs_u64 as f64).max(1.0);
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
        let underwater_fraction = underwater_bars as f64 / (obs_u64 as f64).max(1.0);

        (
            sharpe,
            std,
            sortino,
            downside_dev,
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
    };

    let pnl = log_eq.exp() - 1.0;
    let obs = obs_u64 as f64;
    let exposure = if obs > 0.0 {
        sum_abs_pos as f64 / obs
    } else {
        0.0
    };
    let net_exposure = if obs > 0.0 { sum_pos as f64 / obs } else { 0.0 };

    let metrics = Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation: downside_dev,
        downside_frequency,
        downside_severity,
        calmar,
        omega,
        kappa_tau,
        volatility,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades: trades_count,
        exposure,
        net_exposure,
        long_mean_return,
        short_mean_return,
        closed_trades: trades.iter().filter(|trade| !trade.open).count() as u32,
        winning_trades: trades
            .iter()
            .filter(|trade| !trade.open && trade.pnl > 0.0)
            .count() as u32,
        losing_trades: trades
            .iter()
            .filter(|trade| !trade.open && trade.pnl < 0.0)
            .count() as u32,
        gross_profit: trades
            .iter()
            .filter(|trade| !trade.open && trade.pnl > 0.0)
            .map(|trade| trade.pnl)
            .sum(),
        gross_loss: trades
            .iter()
            .filter(|trade| !trade.open && trade.pnl < 0.0)
            .map(|trade| -trade.pnl)
            .sum(),
        underwater_fraction,
        max_underwater_duration_bars,
    };

    let curves = if with_curves {
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
        Some(DoubleMaCurves {
            equity: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins,
                minmax: eq_mm,
            },
            drawdown: ta_strategies::double_ma::MinMaxBins {
                start_t,
                end_t: n_total,
                bins,
                minmax: dd_mm,
            },
            trades,
            trade_stats: None,
            indicator_bars: Vec::new(),
        })
    } else {
        None
    };

    AsrSimulation { metrics, curves }
}

pub fn eval_asr_metrics_from_desired(
    candles: &Candles,
    lr: &[f64],
    desired_pos: &[i32],
    start_t: usize,
    strategy: &StrategyConfig,
) -> Metrics {
    simulate_from_desired(
        candles,
        lr,
        desired_pos,
        start_t,
        None,
        strategy,
        &IterativeSimConfig::default(),
        None,
    )
    .metrics
}

pub fn eval_asr_metrics_window_from_desired(
    candles: &Candles,
    lr: &[f64],
    desired_pos: &[i32],
    start_t: usize,
    end_t: usize,
    strategy: &StrategyConfig,
) -> Metrics {
    simulate_from_desired(
        candles,
        lr,
        desired_pos,
        start_t,
        Some(end_t),
        strategy,
        &IterativeSimConfig::default(),
        None,
    )
    .metrics
}

pub fn eval_asr_curves_from_desired(
    candles: &Candles,
    lr: &[f64],
    desired_pos: &[i32],
    start_t: usize,
    strategy: &StrategyConfig,
    bins: usize,
) -> DoubleMaCurves {
    simulate_from_desired(
        candles,
        lr,
        desired_pos,
        start_t,
        None,
        strategy,
        &IterativeSimConfig::default(),
        Some(bins),
    )
    .curves
    .unwrap_or(DoubleMaCurves {
        equity: ta_strategies::double_ma::MinMaxBins {
            start_t,
            end_t: lr.len(),
            bins: 0,
            minmax: Vec::new(),
        },
        drawdown: ta_strategies::double_ma::MinMaxBins {
            start_t,
            end_t: lr.len(),
            bins: 0,
            minmax: Vec::new(),
        },
        trades: Vec::new(),
        trade_stats: None,
        indicator_bars: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::eval_asr_metrics_from_desired;
    use ta_strategies::double_ma::StrategyConfig;
    use vector_ta::utilities::data_loader::Candles;

    fn make_wave_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        let mut price: f64 = 100.0;

        for i in 0..len {
            ts.push(i as i64);
            let step = if i < len / 2 {
                if i % 4 == 0 {
                    -0.8
                } else {
                    1.1
                }
            } else if i % 4 == 0 {
                0.7
            } else {
                -1.0
            };
            let o = price;
            price += step;
            let c = price.max(1.0);
            open.push(o);
            close.push(c);
            high.push(o.max(c) + 0.6);
            low.push(o.min(c) - 0.6);
            vol.push(1_000.0 + i as f64);
        }

        Candles::new(ts, open, high, low, close, vol)
    }

    fn close_log_returns(candles: &Candles) -> Vec<f64> {
        let mut lr = vec![0.0; candles.close.len()];
        for i in 1..candles.close.len() {
            let prev = candles.close[i - 1];
            let cur = candles.close[i];
            lr[i] = if prev > 0.0 && cur > 0.0 {
                (cur / prev).ln()
            } else {
                0.0
            };
        }
        lr
    }

    #[test]
    fn asr_metrics_emit_core17_and_trade22_fields_from_desired_positions() {
        let candles = make_wave_candles(96);
        let lr = close_log_returns(&candles);
        let desired_pos: Vec<i32> = (0..candles.close.len())
            .map(|i| {
                if i < 6 {
                    0
                } else if i < 38 {
                    1
                } else if i < 44 {
                    0
                } else if i < 76 {
                    -1
                } else {
                    0
                }
            })
            .collect();

        let metrics = eval_asr_metrics_from_desired(
            &candles,
            &lr,
            &desired_pos,
            1,
            &StrategyConfig::default(),
        );

        assert!(metrics.downside_frequency.is_finite());
        assert!((0.0..=1.0).contains(&metrics.downside_frequency));
        assert!(metrics.downside_frequency > 0.0);
        assert!(metrics.downside_severity.is_finite());
        assert!(metrics.downside_severity > 0.0);
        assert!(metrics.kappa_tau.is_finite());
        assert!(metrics.long_mean_return.is_finite());
        assert!(metrics.long_mean_return.abs() > 0.0);
        assert!(metrics.short_mean_return.is_finite());
        assert!(metrics.short_mean_return.abs() > 0.0);
        assert!((0.0..=1.0).contains(&metrics.underwater_fraction));
        assert!(metrics.max_underwater_duration_bars > 0);
        assert!(metrics.closed_trades >= 2);
        assert!(metrics.winning_trades + metrics.losing_trades <= metrics.closed_trades);
        assert!(metrics.gross_profit >= 0.0);
        assert!(metrics.gross_loss >= 0.0);
    }
}
