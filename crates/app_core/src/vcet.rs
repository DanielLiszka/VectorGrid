use crate::double_ma::Backend;
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorComputeOutput, DynamicIndicatorSeries,
};
#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_direct_with_inputs, DynamicIndicatorCudaSeries,
    PreparedCudaIndicatorInputs,
};
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, IndicatorInputKind, IndicatorParamKV, IndicatorParamValue,
};
#[cfg(feature = "cuda")]
use cust::memory::DeviceBuffer;
use serde::{Deserialize, Serialize};
#[cfg(all(test, feature = "cuda"))]
use std::cell::Cell;
#[cfg(feature = "cuda")]
use std::cell::RefCell;
#[cfg(feature = "cuda")]
use std::sync::Arc;
use std::sync::OnceLock;
use ta_strategies::double_ma::StrategyConfig;
#[cfg(feature = "cuda")]
use vector_ta::cuda::moving_averages::alma_wrapper::DeviceArrayF32;
#[cfg(feature = "cuda")]
use vector_ta::cuda::CudaChandelierExit;
#[cfg(feature = "cuda")]
use vector_ta::indicators::dispatch::DeviceMatrixF32;
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VcetStrategyParams {
    #[serde(default = "default_true")]
    pub map_fast_to_vidya_short_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_vidya_long_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_ce_period: bool,
    #[serde(default = "default_vcet_vidya_short_period")]
    pub vidya_short_period: u16,
    #[serde(default = "default_vcet_vidya_long_period")]
    pub vidya_long_period: u16,
    #[serde(default = "default_vcet_vidya_alpha")]
    pub vidya_alpha: f64,
    #[serde(default = "default_vcet_ce_period")]
    pub ce_period: u16,
    #[serde(default = "default_vcet_ce_mult")]
    pub ce_mult: f64,
    #[serde(default = "default_true")]
    pub ce_use_close: bool,
    #[serde(default = "default_true")]
    pub require_vidya_slope: bool,
    #[serde(default = "default_vcet_max_trade_bars")]
    pub max_trade_bars: u16,
}

fn default_true() -> bool {
    true
}

fn default_vcet_vidya_short_period() -> u16 {
    6
}

fn default_vcet_vidya_long_period() -> u16 {
    24
}

fn default_vcet_vidya_alpha() -> f64 {
    0.2
}

fn default_vcet_ce_period() -> u16 {
    22
}

fn default_vcet_ce_mult() -> f64 {
    3.0
}

fn default_vcet_max_trade_bars() -> u16 {
    120
}

#[cfg(feature = "cuda")]
thread_local! {
    static VCET_ROUTE_DIAGNOSTICS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    #[cfg(test)]
    static FORCE_VCET_CUDA_CPU_FALLBACK: Cell<bool> = const { Cell::new(false) };
    #[cfg(test)]
    static FORCE_VCET_VIDYA_HOST_STAGE_FALLBACK: Cell<bool> = const { Cell::new(false) };
}

#[cfg(feature = "cuda")]
fn record_vcet_route_diagnostic(message: String) {
    VCET_ROUTE_DIAGNOSTICS.with(|cell| {
        let mut diagnostics = cell.borrow_mut();
        if diagnostics.iter().any(|entry| entry == &message) {
            return;
        }
        diagnostics.push(message);
    });
}

#[cfg(feature = "cuda")]
pub(crate) fn clear_vcet_route_diagnostics() {
    VCET_ROUTE_DIAGNOSTICS.with(|cell| cell.borrow_mut().clear());
}

#[cfg(feature = "cuda")]
pub(crate) fn take_vcet_route_diagnostics() -> Option<Vec<String>> {
    VCET_ROUTE_DIAGNOSTICS.with(|cell| {
        let mut diagnostics = cell.borrow_mut();
        if diagnostics.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut *diagnostics))
        }
    })
}

#[cfg(not(feature = "cuda"))]
pub(crate) fn clear_vcet_route_diagnostics() {}

#[cfg(not(feature = "cuda"))]
pub(crate) fn take_vcet_route_diagnostics() -> Option<Vec<String>> {
    None
}

#[cfg(all(test, feature = "cuda"))]
pub(crate) fn set_force_vcet_cuda_cpu_fallback_for_test(enabled: bool) {
    FORCE_VCET_CUDA_CPU_FALLBACK.with(|cell| cell.set(enabled));
}

#[cfg(all(test, feature = "cuda"))]
pub(crate) fn set_force_vcet_vidya_host_stage_fallback_for_test(enabled: bool) {
    FORCE_VCET_VIDYA_HOST_STAGE_FALLBACK.with(|cell| cell.set(enabled));
}

#[cfg(all(test, feature = "cuda"))]
fn force_vcet_cuda_cpu_fallback_for_test() -> bool {
    FORCE_VCET_CUDA_CPU_FALLBACK.with(|cell| cell.get())
}

#[cfg(all(feature = "cuda", not(test)))]
fn force_vcet_cuda_cpu_fallback_for_test() -> bool {
    false
}

#[cfg(all(test, feature = "cuda"))]
fn force_vcet_vidya_host_stage_fallback_for_test() -> bool {
    FORCE_VCET_VIDYA_HOST_STAGE_FALLBACK.with(|cell| cell.get())
}

#[cfg(all(feature = "cuda", not(test)))]
fn force_vcet_vidya_host_stage_fallback_for_test() -> bool {
    false
}

impl Default for VcetStrategyParams {
    fn default() -> Self {
        Self {
            map_fast_to_vidya_short_period: default_true(),
            map_slow_to_vidya_long_period: default_true(),
            map_slow_to_ce_period: default_true(),
            vidya_short_period: default_vcet_vidya_short_period(),
            vidya_long_period: default_vcet_vidya_long_period(),
            vidya_alpha: default_vcet_vidya_alpha(),
            ce_period: default_vcet_ce_period(),
            ce_mult: default_vcet_ce_mult(),
            ce_use_close: default_true(),
            require_vidya_slope: default_true(),
            max_trade_bars: default_vcet_max_trade_bars(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VcetResolvedPeriods {
    pub vidya_short: u16,
    pub vidya_long: u16,
    pub ce: u16,
}

#[derive(Clone, Debug)]
pub struct VcetVidyaSignals {
    pub values: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct VcetChandelierSignals {
    pub long_stop: Vec<f64>,
    pub short_stop: Vec<f64>,
    pub first_valid: usize,
}

struct VcetIndicatorIds {
    vidya_short_key: String,
    vidya_long_key: String,
    vidya_alpha_key: String,
    vidya_output: String,
    vidya_input_kind: IndicatorInputKind,
    chandelier_period_key: String,
    chandelier_mult_key: String,
    chandelier_use_close_key: String,
    chandelier_long_output: String,
    chandelier_short_output: String,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
enum VcetDeviceMatrixOwner {
    Dispatch(DeviceMatrixF32),
    Buffer(DeviceBuffer<f32>),
    SharedBuffer(Arc<DeviceArrayF32>),
}

#[cfg(feature = "cuda")]
pub struct VcetOwnedDeviceMatrix {
    pub rows: usize,
    pub cols: usize,
    device_ptr: u64,
    owner: VcetDeviceMatrixOwner,
}

#[cfg(feature = "cuda")]
impl VcetOwnedDeviceMatrix {
    fn from_dispatch(matrix: DeviceMatrixF32) -> Self {
        Self {
            rows: matrix.rows,
            cols: matrix.cols,
            device_ptr: matrix.device_ptr,
            owner: VcetDeviceMatrixOwner::Dispatch(matrix),
        }
    }

    fn from_host_row(values: Vec<f32>) -> Result<Self, String> {
        let cols = values.len();
        let buf = DeviceBuffer::from_slice(values.as_slice()).map_err(|e| e.to_string())?;
        let device_ptr = buf.as_device_ptr().as_raw() as u64;
        Ok(Self {
            rows: 1,
            cols,
            device_ptr,
            owner: VcetDeviceMatrixOwner::Buffer(buf),
        })
    }

    fn from_shared_row(owner: Arc<DeviceArrayF32>, row_index: usize) -> Result<Self, String> {
        if owner.cols == 0 {
            return Err("VCET shared CUDA row has zero columns".to_string());
        }
        if owner.rows <= row_index {
            return Err(format!(
                "VCET shared CUDA row index out of bounds: row_index={} rows={}",
                row_index, owner.rows
            ));
        }
        let elem_offset = row_index
            .checked_mul(owner.cols)
            .ok_or_else(|| "VCET shared CUDA row offset overflow".to_string())?;
        let byte_offset = elem_offset
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or_else(|| "VCET shared CUDA row byte offset overflow".to_string())?;
        let device_ptr = owner
            .device_ptr()
            .checked_add(byte_offset as u64)
            .ok_or_else(|| "VCET shared CUDA row device pointer overflow".to_string())?;
        Ok(Self {
            rows: 1,
            cols: owner.cols,
            device_ptr,
            owner: VcetDeviceMatrixOwner::SharedBuffer(owner),
        })
    }

    pub fn device_ptr(&self) -> u64 {
        match &self.owner {
            VcetDeviceMatrixOwner::Dispatch(_) => {}
            VcetDeviceMatrixOwner::Buffer(_) => {}
            VcetDeviceMatrixOwner::SharedBuffer(_) => {}
        }
        self.device_ptr
    }
}

pub fn validate_vcet_params(cfg: &VcetStrategyParams) -> Result<(), String> {
    if cfg.vidya_short_period == 0 {
        return Err("VCET vidya_short_period must be > 0".to_string());
    }
    if cfg.vidya_long_period == 0 {
        return Err("VCET vidya_long_period must be > 0".to_string());
    }
    if cfg.vidya_long_period <= cfg.vidya_short_period
        && !cfg.map_fast_to_vidya_short_period
        && !cfg.map_slow_to_vidya_long_period
    {
        return Err("VCET vidya_long_period must be > vidya_short_period".to_string());
    }
    if !cfg.vidya_alpha.is_finite() || cfg.vidya_alpha <= 0.0 {
        return Err("VCET vidya_alpha must be finite and > 0".to_string());
    }
    if cfg.ce_period == 0 {
        return Err("VCET ce_period must be > 0".to_string());
    }
    if !cfg.ce_mult.is_finite() || cfg.ce_mult <= 0.0 {
        return Err("VCET ce_mult must be finite and > 0".to_string());
    }
    if cfg.max_trade_bars == 0 {
        return Err("VCET max_trade_bars must be > 0".to_string());
    }
    Ok(())
}

pub fn resolve_vcet_periods(
    cfg: &VcetStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> VcetResolvedPeriods {
    let vidya_short = if cfg.map_fast_to_vidya_short_period {
        fast_len.max(1)
    } else {
        cfg.vidya_short_period.max(1)
    };
    let raw_long = if cfg.map_slow_to_vidya_long_period {
        slow_len.max(1)
    } else {
        cfg.vidya_long_period.max(1)
    };
    let vidya_long = raw_long.max(vidya_short.saturating_add(1));
    let ce = if cfg.map_slow_to_ce_period {
        slow_len.max(1)
    } else {
        cfg.ce_period.max(1)
    };
    VcetResolvedPeriods {
        vidya_short,
        vidya_long,
        ce,
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
            Err("VCET expected CUDA HostF32 output".to_string())
        }
    }
}

fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("VCET indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "VCET indicator '{indicator_id}' has no declared outputs"
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
        format!("VCET indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.params.is_empty() {
        return Err(format!(
            "VCET indicator '{indicator_id}' has no declared parameters"
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
        "VCET indicator '{indicator_id}' parameter key mismatch; available keys: {}",
        descriptor
            .params
            .iter()
            .map(|p| p.key.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn vcet_indicator_ids() -> Result<&'static VcetIndicatorIds, String> {
    static IDS: OnceLock<Result<VcetIndicatorIds, String>> = OnceLock::new();

    IDS.get_or_init(|| {
        let vidya_descriptor = find_dynamic_indicator_descriptor("vidya").ok_or_else(|| {
            "VCET indicator 'vidya' is not registered for dynamic dispatch".to_string()
        })?;
        Ok(VcetIndicatorIds {
            vidya_short_key: resolve_param_key("vidya", &["short_period", "short", "fast"])?,
            vidya_long_key: resolve_param_key("vidya", &["long_period", "long", "slow"])?,
            vidya_alpha_key: resolve_param_key("vidya", &["alpha"])?,
            vidya_output: resolve_output_id("vidya", &["values", "value", "vidya"])?,
            vidya_input_kind: vidya_descriptor.input_kind,
            chandelier_period_key: resolve_param_key(
                "chandelier_exit",
                &["period", "length", "timeperiod"],
            )?,
            chandelier_mult_key: resolve_param_key("chandelier_exit", &["mult", "multiplier"])?,
            chandelier_use_close_key: resolve_param_key(
                "chandelier_exit",
                &["use_close", "close"],
            )?,
            chandelier_long_output: resolve_output_id("chandelier_exit", &["long_stop", "long"])?,
            chandelier_short_output: resolve_output_id(
                "chandelier_exit",
                &["short_stop", "short"],
            )?,
        })
    })
    .as_ref()
    .map_err(Clone::clone)
}

fn compute_indicator_f64(
    candles: &Candles,
    ma_source: &str,
    _backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    #[cfg(feature = "cuda")]
    {
        if !matches!(_backend, Backend::CpuOnly) {
            if let Some(inputs) = prepared_cuda_inputs {
                if force_vcet_cuda_cpu_fallback_for_test() {
                    record_vcet_route_diagnostic(format!(
                        "VCET indicator {indicator_id}/{output_id} CUDA helper fell back to CPU: forced test fallback"
                    ));
                } else {
                    let descriptor =
                        find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
                            format!(
                                "VCET indicator '{indicator_id}' is not registered for dynamic dispatch"
                            )
                        })?;
                    let out = compute_dynamic_indicator_cuda_direct_with_inputs(
                        inputs,
                        indicator_id,
                        Some(output_id),
                        descriptor.input_kind,
                        params,
                        Kernel::Auto,
                        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                    );
                    match out {
                        Ok(values) => return cuda_series_to_f64(values),
                        Err(err) => {
                            record_vcet_route_diagnostic(format!(
                                "VCET indicator {indicator_id}/{output_id} CUDA helper fell back to CPU: {err}"
                            ));
                        }
                    }
                }
            } else {
                record_vcet_route_diagnostic(format!(
                    "VCET indicator {indicator_id}/{output_id} CUDA helper fell back to CPU: prepared CUDA inputs unavailable"
                ));
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

#[cfg(feature = "cuda")]
fn cuda_series_to_device_f32(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<VcetOwnedDeviceMatrix, String> {
    match out.series {
        DynamicIndicatorCudaSeries::DeviceF32(v) => Ok(VcetOwnedDeviceMatrix::from_dispatch(v)),
        DynamicIndicatorCudaSeries::HostF32(_) => {
            Err("VCET expected CUDA DeviceF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn cuda_series_to_host_device_f32(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<VcetOwnedDeviceMatrix, String> {
    match out.series {
        DynamicIndicatorCudaSeries::HostF32(v) => VcetOwnedDeviceMatrix::from_host_row(v),
        DynamicIndicatorCudaSeries::DeviceF32(v) => Ok(VcetOwnedDeviceMatrix::from_dispatch(v)),
    }
}

pub fn compute_vcet_vidya_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: VcetResolvedPeriods,
    alpha: f64,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<VcetVidyaSignals, String> {
    let ids = vcet_indicator_ids()?;
    let params = vec![
        IndicatorParamKV {
            key: ids.vidya_short_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.vidya_short)),
        },
        IndicatorParamKV {
            key: ids.vidya_long_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.vidya_long)),
        },
        IndicatorParamKV {
            key: ids.vidya_alpha_key.clone(),
            value: IndicatorParamValue::Float(alpha),
        },
    ];
    let values = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "vidya",
        ids.vidya_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let first_valid = first_finite_idx_local(&values)
        .ok_or_else(|| "VCET VIDYA output is all non-finite".to_string())?;
    Ok(VcetVidyaSignals {
        values,
        first_valid,
    })
}

pub fn compute_vcet_chandelier_signals(
    candles: &Candles,
    backend: &Backend,
    period: u16,
    mult: f64,
    use_close: bool,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<VcetChandelierSignals, String> {
    let ids = vcet_indicator_ids()?;
    let params = vec![
        IndicatorParamKV {
            key: ids.chandelier_period_key.clone(),
            value: IndicatorParamValue::Int(i64::from(period)),
        },
        IndicatorParamKV {
            key: ids.chandelier_mult_key.clone(),
            value: IndicatorParamValue::Float(mult),
        },
        IndicatorParamKV {
            key: ids.chandelier_use_close_key.clone(),
            value: IndicatorParamValue::Bool(use_close),
        },
    ];
    let long_stop = compute_indicator_f64(
        candles,
        "close",
        backend,
        "chandelier_exit",
        ids.chandelier_long_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let short_stop = compute_indicator_f64(
        candles,
        "close",
        backend,
        "chandelier_exit",
        ids.chandelier_short_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    if long_stop.len() != short_stop.len() {
        return Err("VCET chandelier output length mismatch".to_string());
    }
    let first_valid = first_finite_idx_local(&long_stop)
        .zip(first_finite_idx_local(&short_stop))
        .map(|(a, b)| a.min(b))
        .or_else(|| first_finite_idx_local(&long_stop))
        .or_else(|| first_finite_idx_local(&short_stop))
        .ok_or_else(|| "VCET chandelier outputs are all non-finite".to_string())?;
    Ok(VcetChandelierSignals {
        long_stop,
        short_stop,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_vcet_vidya_signal_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    periods: VcetResolvedPeriods,
    alpha: f64,
) -> Result<VcetOwnedDeviceMatrix, String> {
    let ids = vcet_indicator_ids()?;
    let params = vec![
        IndicatorParamKV {
            key: ids.vidya_short_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.vidya_short)),
        },
        IndicatorParamKV {
            key: ids.vidya_long_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.vidya_long)),
        },
        IndicatorParamKV {
            key: ids.vidya_alpha_key.clone(),
            value: IndicatorParamValue::Float(alpha),
        },
    ];
    let out = if force_vcet_vidya_host_stage_fallback_for_test() {
        Err("forced test fallback".to_string())
    } else {
        compute_dynamic_indicator_cuda_direct_with_inputs(
            prepared_inputs,
            "vidya",
            Some(ids.vidya_output.as_str()),
            ids.vidya_input_kind,
            params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::DeviceF32,
        )
        .map_err(|e| e.to_string())
    };
    match out {
        Ok(values) => cuda_series_to_device_f32(values),
        Err(err) => {
            record_vcet_route_diagnostic(format!(
                "VCET VIDYA device-output CUDA path fell back to host-staged CUDA upload: {err}"
            ));
            let host_out = compute_dynamic_indicator_cuda_direct_with_inputs(
                prepared_inputs,
                "vidya",
                Some(ids.vidya_output.as_str()),
                ids.vidya_input_kind,
                params.as_slice(),
                Kernel::Auto,
                vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
            )
            .map_err(|e| e.to_string())?;
            cuda_series_to_host_device_f32(host_out)
        }
    }
}

#[cfg(feature = "cuda")]
pub fn compute_vcet_chandelier_signal_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period: u16,
    mult: f64,
    use_close: bool,
) -> Result<(VcetOwnedDeviceMatrix, VcetOwnedDeviceMatrix), String> {
    let device_id = prepared_inputs.source_device().device_id();
    let cuda = CudaChandelierExit::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.hlc_first_valid();
    let periods = [i32::from(period.max(1))];
    let mults = [mult as f32];
    let matrix = cuda
        .chandelier_exit_batch_from_device_dev(
            prepared_inputs.high_device().buffer(),
            prepared_inputs.low_device().buffer(),
            prepared_inputs.close_device().buffer(),
            prepared_inputs.len(),
            first_valid,
            periods.as_slice(),
            mults.as_slice(),
            use_close,
        )
        .map_err(|e| e.to_string())?;
    let shared = Arc::new(matrix);
    let long_matrix = VcetOwnedDeviceMatrix::from_shared_row(shared.clone(), 0)?;
    let short_matrix = VcetOwnedDeviceMatrix::from_shared_row(shared, 1)?;
    Ok((long_matrix, short_matrix))
}

pub fn compute_vcet_start_t(
    first_valid_close: usize,
    first_valid_high: usize,
    first_valid_low: usize,
    first_valid_vidya: usize,
    first_valid_ce: usize,
    trade_on_next_bar: bool,
) -> usize {
    let first_valid = first_valid_close
        .max(first_valid_high)
        .max(first_valid_low)
        .max(first_valid_vidya)
        .max(first_valid_ce)
        .max(1);
    if trade_on_next_bar {
        first_valid.saturating_add(1)
    } else {
        first_valid
    }
}

pub fn build_vcet_desired_positions(
    close: &[f64],
    vidya: &VcetVidyaSignals,
    ce: &VcetChandelierSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &VcetStrategyParams,
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
    let max_trade_bars = usize::from(cfg.max_trade_bars.max(1));
    let mut pos: i32 = 0;
    let mut bars_in_trade: usize = 0;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close.get(sig_t).copied().unwrap_or(f64::NAN);
        let vidya_now = vidya.values.get(sig_t).copied().unwrap_or(f64::NAN);
        let vidya_prev = vidya.values.get(prev_t).copied().unwrap_or(f64::NAN);
        let ce_long = ce.long_stop.get(sig_t).copied().unwrap_or(f64::NAN);
        let ce_short = ce.short_stop.get(sig_t).copied().unwrap_or(f64::NAN);

        let slope_up = !cfg.require_vidya_slope
            || (vidya_now.is_finite() && vidya_prev.is_finite() && vidya_now >= vidya_prev);
        let slope_down = !cfg.require_vidya_slope
            || (vidya_now.is_finite() && vidya_prev.is_finite() && vidya_now <= vidya_prev);

        let long_signal = close_now.is_finite()
            && vidya_now.is_finite()
            && ce_long.is_finite()
            && close_now >= vidya_now
            && close_now >= ce_long
            && slope_up;
        let short_signal = close_now.is_finite()
            && vidya_now.is_finite()
            && ce_short.is_finite()
            && close_now <= vidya_now
            && close_now <= ce_short
            && slope_down;

        if pos == 0 {
            bars_in_trade = 0;
            if long_signal {
                pos = 1;
                bars_in_trade = 1;
            } else if !strategy.long_only && short_signal {
                pos = -1;
                bars_in_trade = 1;
            }
        } else if pos > 0 {
            let mut exit_now = !long_signal || bars_in_trade >= max_trade_bars;
            if !strategy.long_only && short_signal {
                if strategy.allow_flip {
                    pos = -1;
                    bars_in_trade = 1;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }
            if exit_now {
                pos = 0;
                bars_in_trade = 0;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        } else {
            let mut exit_now = !short_signal || bars_in_trade >= max_trade_bars;
            if long_signal {
                if strategy.allow_flip {
                    pos = 1;
                    bars_in_trade = 1;
                    desired[t] = pos;
                    continue;
                }
                exit_now = true;
            }
            if exit_now {
                pos = 0;
                bars_in_trade = 0;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        }

        desired[t] = pos;
    }

    desired
}

#[cfg(test)]
mod tests {
    use super::*;
    use ta_strategies::double_ma::StrategyConfig;

    #[test]
    fn resolve_vcet_periods_maps_fast_slow() {
        let cfg = VcetStrategyParams::default();
        let resolved = resolve_vcet_periods(&cfg, 8, 34);
        assert_eq!(resolved.vidya_short, 8);
        assert_eq!(resolved.vidya_long, 34);
        assert_eq!(resolved.ce, 34);
    }

    #[test]
    fn resolve_vcet_periods_respects_mixed_mapping_and_normalizes_long_period() {
        let cfg = VcetStrategyParams {
            map_fast_to_vidya_short_period: true,
            map_slow_to_vidya_long_period: false,
            map_slow_to_ce_period: false,
            vidya_long_period: 4,
            ce_period: 15,
            ..VcetStrategyParams::default()
        };

        let resolved = resolve_vcet_periods(&cfg, 8, 34);
        assert_eq!(resolved.vidya_short, 8);
        assert_eq!(resolved.vidya_long, 9);
        assert_eq!(resolved.ce, 15);
    }

    #[test]
    fn compute_vcet_start_t_applies_trade_on_next_bar_offset() {
        assert_eq!(compute_vcet_start_t(0, 3, 1, 2, 4, false), 4);
        assert_eq!(compute_vcet_start_t(0, 3, 1, 2, 4, true), 5);
    }

    #[test]
    fn build_vcet_desired_positions_respects_long_only_even_when_short_signals_fire() {
        let close = vec![10.0, 9.5, 9.2, 9.0];
        let vidya = VcetVidyaSignals {
            values: vec![10.0, 10.0, 9.9, 9.8],
            first_valid: 1,
        };
        let ce = VcetChandelierSignals {
            long_stop: vec![f64::NAN, 9.0, 8.9, 8.8],
            short_stop: vec![f64::NAN, 9.8, 9.7, 9.6],
            first_valid: 1,
        };

        let desired = build_vcet_desired_positions(
            &close,
            &vidya,
            &ce,
            1,
            &StrategyConfig {
                long_only: true,
                allow_flip: true,
                trade_on_next_bar: false,
                ..StrategyConfig::default()
            },
            &VcetStrategyParams {
                require_vidya_slope: false,
                ..VcetStrategyParams::default()
            },
        );

        assert_eq!(desired, vec![0, 0, 0, 0]);
    }

    #[test]
    fn build_vcet_desired_positions_exits_without_flipping_when_allow_flip_is_disabled() {
        let close = vec![10.0, 11.0, 9.0, 8.8];
        let vidya = VcetVidyaSignals {
            values: vec![10.0, 10.5, 10.2, 10.0],
            first_valid: 1,
        };
        let ce = VcetChandelierSignals {
            long_stop: vec![f64::NAN, 10.0, 8.5, 8.4],
            short_stop: vec![f64::NAN, 12.0, 9.5, 9.2],
            first_valid: 1,
        };

        let desired = build_vcet_desired_positions(
            &close,
            &vidya,
            &ce,
            1,
            &StrategyConfig {
                long_only: false,
                allow_flip: false,
                trade_on_next_bar: false,
                ..StrategyConfig::default()
            },
            &VcetStrategyParams {
                require_vidya_slope: false,
                ..VcetStrategyParams::default()
            },
        );

        assert_eq!(desired[1], 1);
        assert_eq!(desired[2], 0);
        assert_eq!(desired[3], -1);
    }

    #[test]
    fn build_vcet_desired_positions_exits_when_max_trade_bars_is_reached() {
        let close = vec![10.0, 11.0, 11.2, 11.1, 11.0];
        let vidya = VcetVidyaSignals {
            values: vec![10.0, 10.5, 10.6, 10.7, 10.8],
            first_valid: 1,
        };
        let ce = VcetChandelierSignals {
            long_stop: vec![f64::NAN, 10.1, 10.2, 10.3, 10.4],
            short_stop: vec![f64::NAN, 12.0, 12.0, 12.0, 12.0],
            first_valid: 1,
        };

        let desired = build_vcet_desired_positions(
            &close,
            &vidya,
            &ce,
            1,
            &StrategyConfig {
                long_only: false,
                allow_flip: true,
                trade_on_next_bar: false,
                ..StrategyConfig::default()
            },
            &VcetStrategyParams {
                require_vidya_slope: false,
                max_trade_bars: 2,
                ..VcetStrategyParams::default()
            },
        );

        assert_eq!(desired[1], 1);
        assert_eq!(desired[2], 1);
        assert_eq!(desired[3], 0);
    }

    #[test]
    fn build_vcet_desired_positions_exits_on_signal_loss_and_flips_after_short_confirms() {
        let close = vec![10.0, 10.5, 11.0, 11.4, 11.8, 11.1, 10.7, 10.2];
        let vidya = VcetVidyaSignals {
            values: vec![9.8, 10.0, 10.4, 10.8, 11.0, 11.0, 10.9, 10.7],
            first_valid: 1,
        };
        let ce = VcetChandelierSignals {
            long_stop: vec![
                f64::NAN,
                9.9,
                10.2,
                10.6,
                10.9,
                f64::NAN,
                f64::NAN,
                f64::NAN,
            ],
            short_stop: vec![
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                11.2,
                11.0,
                10.8,
            ],
            first_valid: 1,
        };
        let desired = build_vcet_desired_positions(
            &close,
            &vidya,
            &ce,
            1,
            &StrategyConfig {
                long_only: false,
                allow_flip: true,
                trade_on_next_bar: false,
                ..StrategyConfig::default()
            },
            &VcetStrategyParams::default(),
        );

        assert_eq!(desired[1], 1);
        assert_eq!(desired[4], 1);
        assert_eq!(desired[5], 0);
        assert_eq!(desired[6], -1);
    }
}
