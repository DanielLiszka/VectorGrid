use crate::double_ma::Backend;
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorComputeOutput, DynamicIndicatorSeries,
};
#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_direct_with_inputs, compute_dynamic_indicator_cuda_with_inputs,
    DynamicIndicatorCudaComputeOutput, DynamicIndicatorCudaSeries, PreparedCudaIndicatorInputs,
};
#[cfg(feature = "cuda")]
use crate::indicator_catalog::IndicatorInputKind;
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, IndicatorParamKV, IndicatorParamValue,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use ta_strategies::double_ma::StrategyConfig;
#[cfg(feature = "cuda")]
use vector_ta::cuda::moving_averages::{CudaMama, DeviceArrayF32};
use vector_ta::indicators::decycler::DecyclerBuilder;
#[cfg(feature = "cuda")]
use vector_ta::indicators::dispatch::DeviceMatrixF32;
#[cfg(feature = "cuda")]
use vector_ta::indicators::moving_averages::mama::MamaBatchRange;
use vector_ta::indicators::moving_averages::mama::MamaBuilder;
use vector_ta::utilities::data_loader::{source_type, Candles};
use vector_ta::utilities::enums::Kernel;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CdmcStrategyParams {
    #[serde(default = "default_cdmc_dpo_period")]
    pub dpo_period: u16,
    #[serde(default = "default_cdmc_correlation_cycle_period")]
    pub correlation_cycle_period: u16,
    #[serde(default = "default_cdmc_mama_fast_limit")]
    pub mama_fast_limit: f64,
    #[serde(default = "default_cdmc_mama_slow_limit")]
    pub mama_slow_limit: f64,
    #[serde(default = "default_cdmc_decycler_period")]
    pub decycler_period: u16,
    #[serde(default = "default_cdmc_phase_threshold")]
    pub phase_threshold: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct CdmcResolvedPeriods {
    pub dpo: u16,
    pub correlation_cycle: u16,
    pub decycler: u16,
}

#[derive(Clone, Debug)]
pub struct CdmcSignals {
    pub dpo: Vec<f64>,
    pub cycle_angle: Vec<f64>,
    pub cycle_state: Vec<f64>,
    pub mama: Vec<f64>,
    pub fama: Vec<f64>,
    pub decycler: Vec<f64>,
    pub first_valid: usize,
}

struct CdmcIndicatorIds {
    dpo_period_key: String,
    dpo_output: String,
    correlation_cycle_period_key: String,
    correlation_cycle_angle_output: String,
    correlation_cycle_state_output: String,
    #[cfg(feature = "cuda")]
    decycler_period_key: String,
    #[cfg(feature = "cuda")]
    decycler_output: String,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
enum CdmcDeviceMatrixOwner {
    Dispatch(DeviceMatrixF32),
    CudaMama(DeviceArrayF32),
}

#[cfg(feature = "cuda")]
pub struct CdmcDeviceMatrix {
    pub device_ptr: u64,
    pub rows: usize,
    pub cols: usize,
    pub device_id: u32,
    _owner: CdmcDeviceMatrixOwner,
}

#[cfg(feature = "cuda")]
impl CdmcDeviceMatrix {
    fn from_dispatch(owner: DeviceMatrixF32) -> Self {
        Self {
            device_ptr: owner.device_ptr,
            rows: owner.rows,
            cols: owner.cols,
            device_id: owner.device_id,
            _owner: CdmcDeviceMatrixOwner::Dispatch(owner),
        }
    }

    fn from_cuda_mama(owner: DeviceArrayF32, device_id: u32) -> Self {
        Self {
            device_ptr: owner.device_ptr(),
            rows: owner.rows,
            cols: owner.cols,
            device_id,
            _owner: CdmcDeviceMatrixOwner::CudaMama(owner),
        }
    }

    #[allow(dead_code)]
    fn keepalive_tag(&self) -> u8 {
        match self._owner {
            CdmcDeviceMatrixOwner::Dispatch(ref owner) => {
                let _ = owner.rows;
                1
            }
            CdmcDeviceMatrixOwner::CudaMama(ref owner) => {
                let _ = owner.len();
                2
            }
        }
    }
}

#[cfg(feature = "cuda")]
pub struct CdmcPeriodSweepDevice {
    pub matrix: CdmcDeviceMatrix,
    pub period_start: u16,
    pub period_end: u16,
    pub period_step: u16,
}

#[cfg(feature = "cuda")]
impl CdmcPeriodSweepDevice {
    /// Returns the device-row index for an exact period inside the current sweep.
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

    /// Returns the current conservative warmup estimate used by existing sweep consumers.
    ///
    /// This helper is intentionally documented as inexact. DPO, Correlation
    /// Cycle `angle`, Correlation Cycle `state`, and Decycler do not share the
    /// same exact warmup contract, so this compatibility helper should not be
    /// treated as a precise output-validity boundary for every sweep family.
    pub fn first_valid_for_period(&self, period: u16) -> usize {
        usize::from(period.saturating_sub(1))
    }
}

fn default_cdmc_dpo_period() -> u16 {
    20
}

fn default_cdmc_correlation_cycle_period() -> u16 {
    20
}

fn default_cdmc_mama_fast_limit() -> f64 {
    0.5
}

fn default_cdmc_mama_slow_limit() -> f64 {
    0.05
}

fn default_cdmc_decycler_period() -> u16 {
    20
}

fn default_cdmc_phase_threshold() -> f64 {
    0.0
}

impl Default for CdmcStrategyParams {
    fn default() -> Self {
        Self {
            dpo_period: default_cdmc_dpo_period(),
            correlation_cycle_period: default_cdmc_correlation_cycle_period(),
            mama_fast_limit: default_cdmc_mama_fast_limit(),
            mama_slow_limit: default_cdmc_mama_slow_limit(),
            decycler_period: default_cdmc_decycler_period(),
            phase_threshold: default_cdmc_phase_threshold(),
        }
    }
}

pub fn validate_cdmc_params(cfg: &CdmcStrategyParams) -> Result<(), String> {
    if cfg.dpo_period == 0 {
        return Err("CDMC dpo_period must be >= 1.".to_string());
    }
    if cfg.correlation_cycle_period == 0 {
        return Err("CDMC correlation_cycle_period must be >= 1.".to_string());
    }
    if !cfg.mama_fast_limit.is_finite() || cfg.mama_fast_limit <= 0.0 || cfg.mama_fast_limit > 1.0 {
        return Err("CDMC mama_fast_limit must be in (0, 1].".to_string());
    }
    if !cfg.mama_slow_limit.is_finite() || cfg.mama_slow_limit <= 0.0 || cfg.mama_slow_limit > 1.0 {
        return Err("CDMC mama_slow_limit must be in (0, 1].".to_string());
    }
    if cfg.mama_slow_limit > cfg.mama_fast_limit {
        return Err("CDMC mama_slow_limit must be <= mama_fast_limit.".to_string());
    }
    if cfg.decycler_period == 0 {
        return Err("CDMC decycler_period must be >= 1.".to_string());
    }
    if !cfg.phase_threshold.is_finite() {
        return Err("CDMC phase_threshold must be finite.".to_string());
    }
    Ok(())
}

pub fn resolve_cdmc_periods(
    cfg: &CdmcStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> CdmcResolvedPeriods {
    CdmcResolvedPeriods {
        dpo: fast_len.max(cfg.dpo_period).max(2),
        correlation_cycle: slow_len.max(cfg.correlation_cycle_period).max(2),
        decycler: slow_len.max(cfg.decycler_period).max(2),
    }
}

fn first_finite_idx_local(values: &[f64]) -> Option<usize> {
    values.iter().position(|v| v.is_finite())
}

fn max_first_valid_idx(series: &[&[f64]]) -> Option<usize> {
    series
        .iter()
        .filter_map(|values| first_finite_idx_local(values))
        .max()
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
fn cuda_series_to_f64(out: DynamicIndicatorCudaComputeOutput) -> Result<Vec<f64>, String> {
    match out.series {
        DynamicIndicatorCudaSeries::HostF32(v) => Ok(v.into_iter().map(|x| x as f64).collect()),
        DynamicIndicatorCudaSeries::DeviceF32(_) => {
            Err("CDMC expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn backend_prefers_gpu(backend: &Backend) -> bool {
    !matches!(backend, Backend::CpuOnly)
}

#[cfg(feature = "cuda")]
fn note_cdmc_gpu_fallback(label: &str, err: &dyn std::fmt::Display) {
    static DPO_WARNED: OnceLock<()> = OnceLock::new();
    static CORRELATION_CYCLE_WARNED: OnceLock<()> = OnceLock::new();
    static MAMA_WARNED: OnceLock<()> = OnceLock::new();
    static DECYCLER_WARNED: OnceLock<()> = OnceLock::new();

    let warned = match label {
        "dpo" => &DPO_WARNED,
        "correlation_cycle" => &CORRELATION_CYCLE_WARNED,
        "mama_pair" => &MAMA_WARNED,
        "decycler" => &DECYCLER_WARNED,
        _ => return,
    };

    if warned.set(()).is_ok() {
        eprintln!("CDMC GPU-preferred path fell back to CPU for {label}: {err}");
    }
}

fn cdmc_indicator_ids() -> Result<&'static CdmcIndicatorIds, String> {
    static IDS: OnceLock<Result<CdmcIndicatorIds, String>> = OnceLock::new();
    IDS.get_or_init(|| {
        Ok(CdmcIndicatorIds {
            dpo_period_key: resolve_param_key("dpo", &["period", "length", "timeperiod"])?,
            dpo_output: resolve_output_id("dpo", &["value", "dpo"])?,
            correlation_cycle_period_key: resolve_param_key(
                "correlation_cycle",
                &["period", "length", "timeperiod"],
            )?,
            correlation_cycle_angle_output: resolve_output_id("correlation_cycle", &["angle"])?,
            correlation_cycle_state_output: resolve_output_id("correlation_cycle", &["state"])?,
            #[cfg(feature = "cuda")]
            decycler_period_key: resolve_param_key("decycler", &["hp_period", "period", "length"])?,
            #[cfg(feature = "cuda")]
            decycler_output: resolve_output_id("decycler", &["value", "decycler"])?,
        })
    })
    .as_ref()
    .map_err(|err| err.clone())
}

fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("CDMC indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "CDMC indicator '{indicator_id}' has no declared outputs"
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

    if descriptor.outputs.len() == 1 {
        return Ok(descriptor.outputs[0].id.clone());
    }

    Err(format!(
        "CDMC indicator '{indicator_id}' output mismatch; available outputs: {}",
        descriptor
            .outputs
            .iter()
            .map(|out| out.id.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn resolve_param_key(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("CDMC indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.params.is_empty() {
        return Err(format!(
            "CDMC indicator '{indicator_id}' has no declared parameters"
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
        "CDMC indicator '{indicator_id}' parameter key mismatch; available keys: {}",
        descriptor
            .params
            .iter()
            .map(|p| p.key.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(feature = "cuda")]
fn cuda_series_to_device_f32(
    out: DynamicIndicatorCudaComputeOutput,
) -> Result<CdmcDeviceMatrix, String> {
    match out.series {
        DynamicIndicatorCudaSeries::DeviceF32(v) => Ok(CdmcDeviceMatrix::from_dispatch(v)),
        DynamicIndicatorCudaSeries::HostF32(_) => {
            Err("CDMC expected CUDA DeviceF32 output".to_string())
        }
    }
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
) -> Result<CdmcPeriodSweepDevice, String> {
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
        format!("CDMC indicator '{indicator_id}' is not registered for dynamic dispatch")
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

    Ok(CdmcPeriodSweepDevice {
        matrix,
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_cdmc_dpo_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> Result<CdmcPeriodSweepDevice, String> {
    let ids = cdmc_indicator_ids()?;
    compute_indicator_period_sweep_cuda_device(
        prepared_inputs,
        "dpo",
        ids.dpo_output.as_str(),
        ids.dpo_period_key.as_str(),
        period_start,
        period_end,
        period_step,
        &[],
    )
}

#[cfg(feature = "cuda")]
pub fn compute_cdmc_correlation_cycle_output_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    output_preferred_ids: &[&str],
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> Result<CdmcPeriodSweepDevice, String> {
    let ids = cdmc_indicator_ids()?;
    let output_id = if output_preferred_ids
        .iter()
        .any(|id| id.eq_ignore_ascii_case("angle"))
    {
        ids.correlation_cycle_angle_output.clone()
    } else if output_preferred_ids
        .iter()
        .any(|id| id.eq_ignore_ascii_case("state"))
    {
        ids.correlation_cycle_state_output.clone()
    } else {
        resolve_output_id("correlation_cycle", output_preferred_ids)?
    };
    let base_params = [IndicatorParamKV {
        key: "threshold".to_string(),
        value: IndicatorParamValue::Float(9.0),
    }];
    compute_indicator_period_sweep_cuda_device(
        prepared_inputs,
        "correlation_cycle",
        output_id.as_str(),
        ids.correlation_cycle_period_key.as_str(),
        period_start,
        period_end,
        period_step,
        &base_params,
    )
}

#[cfg(feature = "cuda")]
pub fn compute_cdmc_decycler_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> Result<CdmcPeriodSweepDevice, String> {
    let ids = cdmc_indicator_ids()?;
    let base_params = [IndicatorParamKV {
        key: "k".to_string(),
        value: IndicatorParamValue::Float(0.707),
    }];
    compute_indicator_period_sweep_cuda_device(
        prepared_inputs,
        "decycler",
        ids.decycler_output.as_str(),
        ids.decycler_period_key.as_str(),
        period_start,
        period_end,
        period_step,
        &base_params,
    )
}

#[cfg(feature = "cuda")]
pub fn compute_cdmc_mama_pair_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &CdmcStrategyParams,
) -> Result<((CdmcDeviceMatrix, CdmcDeviceMatrix), usize), String> {
    let prices = prepared_inputs.source();
    let first_valid = prepared_inputs.first_valid_for_input_kind(IndicatorInputKind::Slice);
    let valid = prices.len().saturating_sub(first_valid);
    if valid < 10 {
        return Err(format!(
            "CDMC MAMA requires at least 10 valid samples after first_valid; have {valid}"
        ));
    }

    let device_id = prepared_inputs.source_device().device_id();
    let cuda = CudaMama::new(device_id as usize).map_err(|e| e.to_string())?;
    let sweep = MamaBatchRange {
        fast_limit: (cfg.mama_fast_limit, cfg.mama_fast_limit, 0.0),
        slow_limit: (cfg.mama_slow_limit, cfg.mama_slow_limit, 0.0),
    };
    let mut d_inv_dp = unsafe { cust::memory::DeviceBuffer::<f32>::uninitialized(prices.len()) }
        .map_err(|e| e.to_string())?;
    let d_fast_limits = cust::memory::DeviceBuffer::from_slice(&[cfg.mama_fast_limit as f32])
        .map_err(|e| e.to_string())?;
    let d_slow_limits = cust::memory::DeviceBuffer::from_slice(&[cfg.mama_slow_limit as f32])
        .map_err(|e| e.to_string())?;
    let mut d_mama = unsafe { cust::memory::DeviceBuffer::<f32>::uninitialized(prices.len()) }
        .map_err(|e| e.to_string())?;
    let mut d_fama = unsafe { cust::memory::DeviceBuffer::<f32>::uninitialized(prices.len()) }
        .map_err(|e| e.to_string())?;

    cuda.mama_inv_dp_device(
        prepared_inputs.source_device().buffer(),
        prices.len(),
        first_valid,
        &mut d_inv_dp,
    )
    .map_err(|e| e.to_string())?;
    cuda.mama_batch_from_inv_dp_device(
        prepared_inputs.source_device().buffer(),
        &d_inv_dp,
        &d_fast_limits,
        &d_slow_limits,
        prices.len(),
        1,
        first_valid,
        &mut d_mama,
        &mut d_fama,
    )
    .map_err(|e| e.to_string())?;

    let mut host_mama = vec![f32::NAN; prices.len()];
    let mut host_fama = vec![f32::NAN; prices.len()];
    cuda.mama_batch_into_host_f32(
        prepared_inputs.source(),
        &sweep,
        &mut host_mama,
        &mut host_fama,
    )
    .map_err(|e| e.to_string())?;
    let mama_first_valid = host_mama.iter().position(|value| value.is_finite());
    let fama_first_valid = host_fama.iter().position(|value| value.is_finite());
    let first_output_valid = mama_first_valid
        .zip(fama_first_valid)
        .map(|(mama, fama)| mama.max(fama))
        .ok_or_else(|| "CDMC MAMA/FAMA outputs are all non-finite".to_string())?;

    Ok((
        (
            CdmcDeviceMatrix::from_cuda_mama(
                DeviceArrayF32 {
                    buf: d_mama,
                    rows: 1,
                    cols: prices.len(),
                },
                device_id,
            ),
            CdmcDeviceMatrix::from_cuda_mama(
                DeviceArrayF32 {
                    buf: d_fama,
                    rows: 1,
                    cols: prices.len(),
                },
                device_id,
            ),
        ),
        first_output_valid,
    ))
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
        format!("CDMC indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    #[cfg(feature = "cuda")]
    {
        let prefer_gpu = backend_prefers_gpu(backend)
            && matches!(
                indicator_id,
                "dpo" | "correlation_cycle" | "mama" | "decycler"
            )
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
                match out {
                    Ok(values) => return cuda_series_to_f64(values),
                    Err(err) => note_cdmc_gpu_fallback(indicator_id, &err),
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

fn compute_mama_pair_f64(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &CdmcStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    #[cfg(not(feature = "cuda"))]
    let _ = backend;
    #[cfg(feature = "cuda")]
    {
        if backend_prefers_gpu(backend) {
            if let Some(prepared) = prepared_cuda_inputs {
                let cuda = CudaMama::new(prepared.source_device().device_id() as usize)
                    .map_err(|e| e.to_string())?;
                let sweep = MamaBatchRange {
                    fast_limit: (cfg.mama_fast_limit, cfg.mama_fast_limit, 0.0),
                    slow_limit: (cfg.mama_slow_limit, cfg.mama_slow_limit, 0.0),
                };
                let n = prepared.source().len();
                let mut mama = vec![f32::NAN; n];
                let mut fama = vec![f32::NAN; n];
                match cuda.mama_batch_into_host_f32(prepared.source(), &sweep, &mut mama, &mut fama)
                {
                    Ok(_) => {
                        return Ok((
                            mama.into_iter().map(|v| v as f64).collect(),
                            fama.into_iter().map(|v| v as f64).collect(),
                        ));
                    }
                    Err(err) => note_cdmc_gpu_fallback("mama_pair", &err),
                }
            }
        }
    }

    let price = source_type(candles, ma_source);
    let out = MamaBuilder::new()
        .fast_limit(cfg.mama_fast_limit)
        .slow_limit(cfg.mama_slow_limit)
        .kernel(Kernel::Auto)
        .apply_slice(price)
        .map_err(|e| e.to_string())?;
    Ok((out.mama_values, out.fama_values))
}

fn compute_decycler_f64(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: CdmcResolvedPeriods,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    #[cfg(not(feature = "cuda"))]
    let _ = backend;
    #[cfg(feature = "cuda")]
    {
        if backend_prefers_gpu(backend) {
            if let Some(prepared) = prepared_cuda_inputs {
                let ids = cdmc_indicator_ids()?;
                let params = vec![
                    IndicatorParamKV {
                        key: ids.decycler_period_key.clone(),
                        value: IndicatorParamValue::Int(i64::from(periods.decycler)),
                    },
                    IndicatorParamKV {
                        key: "k".to_string(),
                        value: IndicatorParamValue::Float(0.707),
                    },
                ];
                let out = compute_dynamic_indicator_cuda_direct_with_inputs(
                    prepared,
                    "decycler",
                    Some(ids.decycler_output.as_str()),
                    IndicatorInputKind::Slice,
                    params.as_slice(),
                    Kernel::Auto,
                    vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                );
                match out {
                    Ok(values) => return cuda_series_to_f64(values),
                    Err(err) => note_cdmc_gpu_fallback("decycler", &err),
                }
            }
        }
    }

    let price = source_type(candles, ma_source);
    DecyclerBuilder::new()
        .hp_period(periods.decycler as usize)
        .k(0.707)
        .kernel(Kernel::Auto)
        .apply_slice(price)
        .map_err(|e| e.to_string())
        .map(|out| out.values)
}

pub fn compute_cdmc_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: CdmcResolvedPeriods,
    cfg: &CdmcStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<CdmcSignals, String> {
    let ids = cdmc_indicator_ids()?;
    let dpo_params = vec![IndicatorParamKV {
        key: ids.dpo_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(periods.dpo)),
    }];
    let cycle_params = vec![
        IndicatorParamKV {
            key: ids.correlation_cycle_period_key.clone(),
            value: IndicatorParamValue::Int(i64::from(periods.correlation_cycle)),
        },
        IndicatorParamKV {
            key: "threshold".to_string(),
            value: IndicatorParamValue::Float(9.0),
        },
    ];
    let dpo = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "dpo",
        ids.dpo_output.as_str(),
        dpo_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let cycle_angle = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "correlation_cycle",
        ids.correlation_cycle_angle_output.as_str(),
        cycle_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let cycle_state = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "correlation_cycle",
        ids.correlation_cycle_state_output.as_str(),
        cycle_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let (mama, fama) = compute_mama_pair_f64(
        candles,
        ma_source,
        backend,
        cfg,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let decycler = compute_decycler_f64(
        candles,
        ma_source,
        backend,
        periods,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let first_valid = max_first_valid_idx(&[
        dpo.as_slice(),
        cycle_angle.as_slice(),
        cycle_state.as_slice(),
        mama.as_slice(),
        fama.as_slice(),
        decycler.as_slice(),
    ])
    .ok_or_else(|| "CDMC signals are all non-finite".to_string())?;

    Ok(CdmcSignals {
        dpo,
        cycle_angle,
        cycle_state,
        mama,
        fama,
        decycler,
        first_valid,
    })
}

pub fn compute_cdmc_start_t(
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

pub fn build_cdmc_desired_positions(
    close: &[f64],
    signals: &CdmcSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &CdmcStrategyParams,
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
    let phase_gate = cfg.phase_threshold.abs();
    let mut pos = 0i32;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let dpo_now = signals.dpo.get(sig_t).copied().unwrap_or(f64::NAN);
        let dpo_prev = signals.dpo.get(prev_t).copied().unwrap_or(f64::NAN);
        let angle_now = signals.cycle_angle.get(sig_t).copied().unwrap_or(f64::NAN);
        let state_now = signals.cycle_state.get(sig_t).copied().unwrap_or(f64::NAN);
        let mama_now = signals.mama.get(sig_t).copied().unwrap_or(f64::NAN);
        let fama_now = signals.fama.get(sig_t).copied().unwrap_or(f64::NAN);
        let decycler_now = signals.decycler.get(sig_t).copied().unwrap_or(f64::NAN);
        let decycler_prev = signals.decycler.get(prev_t).copied().unwrap_or(f64::NAN);

        let bullish_phase = angle_now.is_finite() && angle_now >= phase_gate;
        let bearish_phase = angle_now.is_finite() && angle_now <= -phase_gate;
        let trend_regime = !state_now.is_finite() || state_now >= 0.0;
        let reversion_regime = state_now.is_finite() && state_now < 0.0;
        let trend_up = mama_now.is_finite() && fama_now.is_finite() && mama_now > fama_now;
        let trend_down = mama_now.is_finite() && fama_now.is_finite() && mama_now < fama_now;
        let decycler_rising =
            decycler_now.is_finite() && decycler_prev.is_finite() && decycler_now > decycler_prev;
        let decycler_falling =
            decycler_now.is_finite() && decycler_prev.is_finite() && decycler_now < decycler_prev;
        let dpo_cross_up =
            dpo_now.is_finite() && dpo_prev.is_finite() && dpo_prev <= 0.0 && dpo_now > 0.0;
        let dpo_cross_down =
            dpo_now.is_finite() && dpo_prev.is_finite() && dpo_prev >= 0.0 && dpo_now < 0.0;

        let long_entry = (trend_regime
            && trend_up
            && decycler_rising
            && dpo_now.is_finite()
            && dpo_now > 0.0
            && bullish_phase)
            || (reversion_regime && trend_up && dpo_cross_up && bullish_phase);
        let short_entry = !strategy.long_only
            && ((trend_regime
                && trend_down
                && decycler_falling
                && dpo_now.is_finite()
                && dpo_now < 0.0
                && bearish_phase)
                || (reversion_regime && trend_down && dpo_cross_down && bearish_phase));

        if pos == 0 {
            if long_entry {
                pos = 1;
            } else if short_entry {
                pos = -1;
            }
        } else if pos > 0 {
            let opposing_votes = usize::from(trend_down)
                + usize::from(decycler_falling)
                + usize::from(dpo_now.is_finite() && dpo_now < 0.0)
                + usize::from(bearish_phase);
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
            let opposing_votes = usize::from(trend_up)
                + usize::from(decycler_rising)
                + usize::from(dpo_now.is_finite() && dpo_now > 0.0)
                + usize::from(bullish_phase);
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
    #[cfg(feature = "cuda")]
    use crate::dynamic_indicator_adapter::PreparedCudaIndicatorInputs;
    #[cfg(feature = "cuda")]
    use vector_ta::cuda::cuda_available;
    use vector_ta::utilities::data_loader::Candles;

    fn make_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64 * 0.05;
            open.push(base);
            high.push(base + 0.75);
            low.push(base - 0.75);
            close.push(base + ((i % 11) as f64 - 5.0) * 0.01);
            vol.push(10_000.0 + ((i % 17) as f64) * 10.0);
        }

        Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn validate_cdmc_params_rejects_invalid_values() {
        let mut cfg = CdmcStrategyParams::default();
        cfg.mama_fast_limit = 0.0;
        assert!(validate_cdmc_params(&cfg).is_err());

        let mut cfg = CdmcStrategyParams::default();
        cfg.mama_slow_limit = 0.6;
        assert!(validate_cdmc_params(&cfg).is_err());
    }

    #[test]
    fn build_cdmc_desired_positions_enters_and_exits_on_cycle_shift() {
        let close = vec![100.0, 100.4, 101.2, 102.0, 101.4, 100.5, 99.8];
        let signals = CdmcSignals {
            dpo: vec![f64::NAN, -0.3, 0.2, 0.7, 0.4, -0.2, -0.6],
            cycle_angle: vec![f64::NAN, -5.0, 8.0, 12.0, 6.0, -8.0, -14.0],
            cycle_state: vec![f64::NAN, -1.0, -1.0, 1.0, 1.0, 1.0, -1.0],
            mama: vec![f64::NAN, 99.8, 100.9, 101.8, 101.7, 100.7, 99.9],
            fama: vec![f64::NAN, 100.0, 100.5, 101.0, 101.3, 101.0, 100.4],
            decycler: vec![f64::NAN, 99.9, 100.4, 101.0, 101.2, 100.8, 100.1],
            first_valid: 1,
        };
        let mut strategy = StrategyConfig::default();
        strategy.trade_on_next_bar = false;
        strategy.allow_flip = false;
        let cfg = CdmcStrategyParams::default();

        let desired = build_cdmc_desired_positions(&close, &signals, 1, &strategy, &cfg);
        assert_eq!(desired[2], 1);
        assert_eq!(desired[5], 0);
    }

    #[test]
    fn resolve_cdmc_periods_normalizes_against_strategy_lengths_and_floor() {
        let cfg = CdmcStrategyParams {
            dpo_period: 1,
            correlation_cycle_period: 1,
            decycler_period: 1,
            ..CdmcStrategyParams::default()
        };
        let periods = resolve_cdmc_periods(&cfg, 12, 34);
        assert_eq!(periods.dpo, 12);
        assert_eq!(periods.correlation_cycle, 34);
        assert_eq!(periods.decycler, 34);
    }

    #[test]
    fn phase_threshold_is_effectively_magnitude_only() {
        let close = vec![100.0, 100.4, 101.2, 102.0, 101.4, 100.5, 99.8];
        let signals = CdmcSignals {
            dpo: vec![f64::NAN, -0.3, 0.2, 0.7, 0.4, -0.2, -0.6],
            cycle_angle: vec![f64::NAN, -5.0, 8.0, 12.0, 6.0, -8.0, -14.0],
            cycle_state: vec![f64::NAN, -1.0, -1.0, 1.0, 1.0, 1.0, -1.0],
            mama: vec![f64::NAN, 99.8, 100.9, 101.8, 101.7, 100.7, 99.9],
            fama: vec![f64::NAN, 100.0, 100.5, 101.0, 101.3, 101.0, 100.4],
            decycler: vec![f64::NAN, 99.9, 100.4, 101.0, 101.2, 100.8, 100.1],
            first_valid: 1,
        };
        let strategy = StrategyConfig::default();
        let positive = build_cdmc_desired_positions(
            &close,
            &signals,
            1,
            &strategy,
            &CdmcStrategyParams {
                phase_threshold: 8.0,
                ..CdmcStrategyParams::default()
            },
        );
        let negative = build_cdmc_desired_positions(
            &close,
            &signals,
            1,
            &strategy,
            &CdmcStrategyParams {
                phase_threshold: -8.0,
                ..CdmcStrategyParams::default()
            },
        );
        assert_eq!(positive, negative);
    }

    #[test]
    fn resolve_output_id_rejects_unresolved_multi_output_indicators() {
        let err = resolve_output_id("correlation_cycle", &["does_not_exist"])
            .expect_err("multi-output mismatch should not silently bind the first output");
        assert!(err.contains("available outputs"));
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compute_cdmc_signals_gpu_smoke_runs_with_dynamic_mama_and_decycler() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(512);
        let prepared = PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
            &candles,
            "close",
            0,
            &[crate::indicator_catalog::IndicatorInputKind::Slice],
        )
        .expect("cuda inputs");
        let cfg = CdmcStrategyParams::default();
        let periods = resolve_cdmc_periods(&cfg, 12, 34);

        let signals = compute_cdmc_signals(
            &candles,
            "close",
            &Backend::GpuOnly { device_id: 0 },
            periods,
            &cfg,
            Some(&prepared),
        )
        .expect("cdmc gpu signals");

        assert_eq!(signals.mama.len(), candles.close.len());
        assert_eq!(signals.fama.len(), candles.close.len());
        assert_eq!(signals.decycler.len(), candles.close.len());
        assert!(signals.first_valid < candles.close.len());
        assert!(signals.mama.iter().any(|v| v.is_finite()));
        assert!(signals.fama.iter().any(|v| v.is_finite()));
        assert!(signals.decycler.iter().any(|v| v.is_finite()));
    }
}
