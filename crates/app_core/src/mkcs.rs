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
use vector_ta::indicators::dispatch::DeviceMatrixF32;
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

/// Serialized MKCS strategy parameters.
///
/// The serde defaults below are part of the persisted-config compatibility
/// contract for saved presets and artifact replays.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MkcsStrategyParams {
    #[serde(default = "default_true")]
    pub map_fast_to_stddev_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_breakout_lookback: bool,
    #[serde(default = "default_mkcs_stddev_period")]
    pub stddev_period: u16,
    #[serde(default = "default_mkcs_breakout_lookback")]
    pub breakout_lookback: u16,
    #[serde(default = "default_mkcs_compression_threshold")]
    pub compression_threshold: f64,
    #[serde(default = "default_mkcs_reexpansion_threshold")]
    pub reexpansion_threshold: f64,
}

fn default_true() -> bool {
    true
}

fn default_mkcs_stddev_period() -> u16 {
    20
}

fn default_mkcs_breakout_lookback() -> u16 {
    20
}

fn default_mkcs_compression_threshold() -> f64 {
    0.01
}

fn default_mkcs_reexpansion_threshold() -> f64 {
    0.03
}

impl Default for MkcsStrategyParams {
    fn default() -> Self {
        Self {
            map_fast_to_stddev_period: default_true(),
            map_slow_to_breakout_lookback: default_true(),
            stddev_period: default_mkcs_stddev_period(),
            breakout_lookback: default_mkcs_breakout_lookback(),
            compression_threshold: default_mkcs_compression_threshold(),
            reexpansion_threshold: default_mkcs_reexpansion_threshold(),
        }
    }
}

/// Concrete stddev / breakout periods after fast/slow mapping is resolved.
#[derive(Clone, Copy, Debug)]
pub struct MkcsResolvedPeriods {
    pub stddev: u16,
    pub breakout: u16,
}

/// Compression-side MKCS series.
///
/// `stddev` is expected to stay aligned with `candles.close`; callers may rely
/// on `first_valid` only after the returned length contract is satisfied.
#[derive(Clone, Debug)]
pub struct MkcsCompressionSignals {
    pub stddev: Vec<f64>,
    pub first_valid: usize,
}

/// Breakout-side MKCS series.
///
/// `upper` and `lower` are expected to stay aligned with each other and with
/// `candles.close`; callers may rely on `first_valid` only after the returned
/// length contract is satisfied.
#[derive(Clone, Debug)]
pub struct MkcsBreakoutSignals {
    pub upper: Vec<f64>,
    pub lower: Vec<f64>,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
/// Device-resident Donchian sweeps kept on-device for downstream consumers.
pub struct MkcsDonchianSweepsDevice {
    pub upper: MkcsPeriodSweepDevice,
    pub lower: MkcsPeriodSweepDevice,
}

struct MkcsIndicatorIds {
    stddev_period_key: String,
    stddev_output: String,
    #[cfg(feature = "cuda")]
    stddev_input_kind: IndicatorInputKind,
    donchian_period_key: String,
    donchian_upper_output: String,
    donchian_lower_output: String,
    #[cfg(feature = "cuda")]
    donchian_input_kind: IndicatorInputKind,
}

#[cfg(feature = "cuda")]
enum MkcsDeviceMatrixOwner {
    Dispatch(DeviceMatrixF32),
}

#[cfg(feature = "cuda")]
/// Keepalive wrapper around a dispatch-owned device matrix.
pub struct MkcsDeviceMatrix {
    pub device_ptr: u64,
    pub rows: usize,
    pub cols: usize,
    pub device_id: u32,
    _owner: MkcsDeviceMatrixOwner,
}

#[cfg(feature = "cuda")]
impl MkcsDeviceMatrix {
    fn from_dispatch(owner: DeviceMatrixF32) -> Self {
        Self {
            device_ptr: owner.device_ptr,
            rows: owner.rows,
            cols: owner.cols,
            device_id: owner.device_id,
            _owner: MkcsDeviceMatrixOwner::Dispatch(owner),
        }
    }

    fn keepalive_tag(&self) -> u8 {
        match self._owner {
            MkcsDeviceMatrixOwner::Dispatch(ref owner) => {
                let _ = owner.rows;
                1
            }
        }
    }
}

#[cfg(feature = "cuda")]
/// Row-major device matrix for a normalized inclusive period sweep.
///
/// Rows map to periods from `period_start..=period_end` using `period_step`.
/// Construction validates the returned row count against that normalized
/// cardinality before this carrier is exposed to callers.
pub struct MkcsPeriodSweepDevice {
    pub matrix: MkcsDeviceMatrix,
    pub period_start: u16,
    pub period_end: u16,
    pub period_step: u16,
}

#[cfg(feature = "cuda")]
impl MkcsPeriodSweepDevice {
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

pub fn validate_mkcs_params(cfg: &MkcsStrategyParams) -> Result<(), String> {
    if cfg.stddev_period == 0 {
        return Err("MKCS stddev_period must be > 0".to_string());
    }
    if cfg.breakout_lookback == 0 {
        return Err("MKCS breakout_lookback must be > 0".to_string());
    }
    if !cfg.compression_threshold.is_finite() || cfg.compression_threshold <= 0.0 {
        return Err("MKCS compression_threshold must be finite and > 0".to_string());
    }
    if !cfg.reexpansion_threshold.is_finite() || cfg.reexpansion_threshold <= 0.0 {
        return Err("MKCS reexpansion_threshold must be finite and > 0".to_string());
    }
    if cfg.reexpansion_threshold < cfg.compression_threshold {
        return Err("MKCS reexpansion_threshold must be >= compression_threshold".to_string());
    }
    Ok(())
}

pub fn resolve_mkcs_periods(
    cfg: &MkcsStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> MkcsResolvedPeriods {
    MkcsResolvedPeriods {
        stddev: if cfg.map_fast_to_stddev_period {
            fast_len.max(1)
        } else {
            cfg.stddev_period.max(1)
        },
        breakout: if cfg.map_slow_to_breakout_lookback {
            slow_len.max(1)
        } else {
            cfg.breakout_lookback.max(1)
        },
    }
}

fn first_finite_idx_local(values: &[f64]) -> Option<usize> {
    values.iter().position(|v| v.is_finite())
}

fn validate_indicator_series_len(
    indicator_id: &str,
    output_id: &str,
    values: &[f64],
    expected_len: usize,
) -> Result<(), String> {
    if values.len() == expected_len {
        return Ok(());
    }
    Err(format!(
        "MKCS indicator '{indicator_id}' output '{output_id}' length mismatch: expected {expected_len}, got {}",
        values.len()
    ))
}

fn validate_breakout_signal_lengths(
    upper_output_id: &str,
    upper: &[f64],
    lower_output_id: &str,
    lower: &[f64],
    expected_len: usize,
) -> Result<(), String> {
    if upper.len() != lower.len() {
        return Err(format!(
            "MKCS donchian output length mismatch: upper '{}' has {}, lower '{}' has {}",
            upper_output_id,
            upper.len(),
            lower_output_id,
            lower.len()
        ));
    }
    validate_indicator_series_len("donchian", upper_output_id, upper, expected_len)?;
    validate_indicator_series_len("donchian", lower_output_id, lower, expected_len)?;
    Ok(())
}

fn validate_distinct_output_ids(
    indicator_id: &str,
    upper_output_id: &str,
    lower_output_id: &str,
    available_outputs: &[String],
) -> Result<(), String> {
    if !upper_output_id.eq_ignore_ascii_case(lower_output_id) {
        return Ok(());
    }
    Err(format!(
        "MKCS indicator '{indicator_id}' resolved duplicate output ids for upper/lower binding: '{}' (available outputs: {})",
        upper_output_id,
        available_outputs.join(", ")
    ))
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
            Err("MKCS expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn cuda_series_to_device_f32(
    out: DynamicIndicatorCudaComputeOutput,
) -> Result<MkcsDeviceMatrix, String> {
    match out.series {
        DynamicIndicatorCudaSeries::DeviceF32(v) => Ok(MkcsDeviceMatrix::from_dispatch(v)),
        DynamicIndicatorCudaSeries::HostF32(_) => {
            Err("MKCS expected CUDA DeviceF32 output".to_string())
        }
    }
}

fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("MKCS indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "MKCS indicator '{indicator_id}' has no declared outputs"
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
        format!("MKCS indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.params.is_empty() {
        return Err(format!(
            "MKCS indicator '{indicator_id}' has no declared parameters"
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
        "MKCS indicator '{indicator_id}' parameter key mismatch; available keys: {}",
        descriptor
            .params
            .iter()
            .map(|p| p.key.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn mkcs_indicator_ids() -> Result<&'static MkcsIndicatorIds, String> {
    static IDS: OnceLock<Result<MkcsIndicatorIds, String>> = OnceLock::new();

    // The first descriptor-resolution outcome is intentionally sticky for the
    // process lifetime so registry drift cannot change MKCS bindings mid-run.
    IDS.get_or_init(|| {
        #[cfg(feature = "cuda")]
        let stddev_descriptor = find_dynamic_indicator_descriptor("stddev").ok_or_else(|| {
            "MKCS indicator 'stddev' is not registered for dynamic dispatch".to_string()
        })?;
        let donchian_descriptor =
            find_dynamic_indicator_descriptor("donchian").ok_or_else(|| {
                "MKCS indicator 'donchian' is not registered for dynamic dispatch".to_string()
            })?;
        let donchian_upper_output = resolve_output_id(
            "donchian",
            &["upper_band", "upperband", "upper", "up", "value"],
        )?;
        let donchian_lower_output =
            resolve_output_id("donchian", &["lower_band", "lowerband", "lower", "down"])?;
        validate_distinct_output_ids(
            "donchian",
            donchian_upper_output.as_str(),
            donchian_lower_output.as_str(),
            &donchian_descriptor
                .outputs
                .iter()
                .map(|output| output.id.clone())
                .collect::<Vec<_>>(),
        )?;
        Ok(MkcsIndicatorIds {
            stddev_period_key: resolve_param_key("stddev", &["period", "length", "timeperiod"])?,
            stddev_output: resolve_output_id("stddev", &["value", "stddev", "stdev"])?,
            #[cfg(feature = "cuda")]
            stddev_input_kind: stddev_descriptor.input_kind,
            donchian_period_key: resolve_param_key(
                "donchian",
                &["period", "length", "timeperiod"],
            )?,
            donchian_upper_output,
            donchian_lower_output,
            #[cfg(feature = "cuda")]
            donchian_input_kind: donchian_descriptor.input_kind,
        })
    })
    .as_ref()
    .map_err(Clone::clone)
}

#[cfg(feature = "cuda")]
fn normalized_period_sweep_cardinality(
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> usize {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    usize::from((end - start) / step) + 1
}

#[cfg(feature = "cuda")]
fn backend_prefers_gpu(backend: &Backend) -> bool {
    !matches!(backend, Backend::CpuOnly)
}

#[cfg(feature = "cuda")]
fn select_donchian_output_id<'a>(
    ids: &'a MkcsIndicatorIds,
    output_preferred_ids: &[&str],
) -> &'a str {
    if output_preferred_ids.iter().any(|id| {
        matches!(
            id.to_ascii_lowercase().as_str(),
            "lower_band" | "lowerband" | "lower" | "down"
        )
    }) {
        ids.donchian_lower_output.as_str()
    } else {
        ids.donchian_upper_output.as_str()
    }
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
) -> Result<MkcsPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let mut params: Vec<IndicatorParamKV> = Vec::with_capacity(base_params.len() + 3);
    params.extend_from_slice(base_params);
    params.push(IndicatorParamKV {
        key: format!("{period_key}_start"),
        value: IndicatorParamValue::Int(i64::from(start)),
    });
    params.push(IndicatorParamKV {
        key: format!("{period_key}_end"),
        value: IndicatorParamValue::Int(i64::from(end)),
    });
    params.push(IndicatorParamKV {
        key: format!("{period_key}_step"),
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
    let expected_rows = normalized_period_sweep_cardinality(start, end, step);
    if matrix.rows != expected_rows {
        return Err(format!(
            "MKCS indicator '{indicator_id}' period sweep row count mismatch: expected {expected_rows}, got {} for normalized range [{start}..={end}] step {step}",
            matrix.rows
        ));
    }

    Ok(MkcsPeriodSweepDevice {
        matrix,
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_mkcs_stddev_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> Result<MkcsPeriodSweepDevice, String> {
    let ids = mkcs_indicator_ids()?;
    compute_indicator_period_sweep_cuda_device(
        prepared_inputs,
        "stddev",
        ids.stddev_output.as_str(),
        ids.stddev_input_kind,
        ids.stddev_period_key.as_str(),
        period_start,
        period_end,
        period_step,
        &[],
    )
}

#[cfg(feature = "cuda")]
pub fn compute_mkcs_donchian_sweeps_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> Result<MkcsDonchianSweepsDevice, String> {
    let ids = mkcs_indicator_ids()?;
    Ok(MkcsDonchianSweepsDevice {
        upper: compute_indicator_period_sweep_cuda_device(
            prepared_inputs,
            "donchian",
            ids.donchian_upper_output.as_str(),
            ids.donchian_input_kind,
            ids.donchian_period_key.as_str(),
            period_start,
            period_end,
            period_step,
            &[],
        )?,
        lower: compute_indicator_period_sweep_cuda_device(
            prepared_inputs,
            "donchian",
            ids.donchian_lower_output.as_str(),
            ids.donchian_input_kind,
            ids.donchian_period_key.as_str(),
            period_start,
            period_end,
            period_step,
            &[],
        )?,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_mkcs_donchian_output_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    output_preferred_ids: &[&str],
    period_start: u16,
    period_end: u16,
    period_step: u16,
) -> Result<MkcsPeriodSweepDevice, String> {
    let ids = mkcs_indicator_ids()?;
    let output_id = select_donchian_output_id(ids, output_preferred_ids);
    compute_indicator_period_sweep_cuda_device(
        prepared_inputs,
        "donchian",
        output_id,
        ids.donchian_input_kind,
        ids.donchian_period_key.as_str(),
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
        format!("MKCS indicator '{indicator_id}' is not registered for dynamic dispatch")
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

pub fn compute_mkcs_compression_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    stddev_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<MkcsCompressionSignals, String> {
    let ids = mkcs_indicator_ids()?;
    let params = vec![IndicatorParamKV {
        key: ids.stddev_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(stddev_period)),
    }];
    let stddev = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "stddev",
        ids.stddev_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    validate_indicator_series_len(
        "stddev",
        ids.stddev_output.as_str(),
        &stddev,
        candles.close.len(),
    )?;
    let first_valid = first_finite_idx_local(&stddev)
        .ok_or_else(|| "MKCS stddev output is all non-finite".to_string())?;
    Ok(MkcsCompressionSignals {
        stddev,
        first_valid,
    })
}

pub fn compute_mkcs_breakout_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    breakout_lookback: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<MkcsBreakoutSignals, String> {
    let ids = mkcs_indicator_ids()?;
    let params = vec![IndicatorParamKV {
        key: ids.donchian_period_key.clone(),
        value: IndicatorParamValue::Int(i64::from(breakout_lookback)),
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
    validate_breakout_signal_lengths(
        ids.donchian_upper_output.as_str(),
        &upper,
        ids.donchian_lower_output.as_str(),
        &lower,
        candles.close.len(),
    )?;

    let first_valid = first_finite_idx_local(&upper)
        .zip(first_finite_idx_local(&lower))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "MKCS donchian outputs are all non-finite".to_string())?;

    Ok(MkcsBreakoutSignals {
        upper,
        lower,
        first_valid,
    })
}

pub fn compute_mkcs_start_t(
    first_valid_close: usize,
    first_valid_high: usize,
    first_valid_low: usize,
    first_valid_compression: usize,
    first_valid_breakout: usize,
    trade_on_next_bar: bool,
) -> usize {
    let first_valid = first_valid_close
        .max(first_valid_high)
        .max(first_valid_low)
        .max(first_valid_compression)
        .max(first_valid_breakout)
        .max(1);
    if trade_on_next_bar {
        first_valid.saturating_add(1)
    } else {
        first_valid
    }
}

fn stddev_ratio(stddev: f64, close: f64) -> f64 {
    if !stddev.is_finite() || !close.is_finite() {
        return f64::NAN;
    }
    let denom = close.abs().max(1.0e-12);
    stddev.abs() / denom
}

pub fn build_mkcs_desired_positions(
    close: &[f64],
    compression: &MkcsCompressionSignals,
    breakout: &MkcsBreakoutSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &MkcsStrategyParams,
) -> Vec<i32> {
    let n = close.len();
    let mut desired = vec![0i32; n];
    if start_t >= n {
        return desired;
    }
    if compression.stddev.len() != n || breakout.upper.len() != n || breakout.lower.len() != n {
        return desired;
    }

    let trade_offset = if strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let mut pos: i32 = 0;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close[sig_t];
        let close_prev = close[prev_t];
        let stddev_prev = compression.stddev[prev_t];
        let stddev_now = compression.stddev[sig_t];
        let upper_prev = breakout.upper[prev_t];
        let lower_prev = breakout.lower[prev_t];

        let compressed_prev = {
            let ratio = stddev_ratio(stddev_prev, close_prev);
            ratio.is_finite() && ratio <= cfg.compression_threshold
        };
        let reexpanded_now = {
            let ratio = stddev_ratio(stddev_now, close_now);
            ratio.is_finite() && ratio >= cfg.reexpansion_threshold
        };

        let long_breakout = compressed_prev
            && close_now.is_finite()
            && upper_prev.is_finite()
            && close_now > upper_prev;
        let short_breakout = compressed_prev
            && close_now.is_finite()
            && lower_prev.is_finite()
            && close_now < lower_prev;

        if pos == 0 {
            if long_breakout {
                pos = 1;
            } else if !strategy.long_only && short_breakout {
                pos = -1;
            }
        } else if pos > 0 {
            let mut exit_now = reexpanded_now
                || (close_now.is_finite() && upper_prev.is_finite() && close_now <= upper_prev);

            if !strategy.long_only && short_breakout {
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
            let mut exit_now = reexpanded_now
                || (close_now.is_finite() && lower_prev.is_finite() && close_now >= lower_prev);

            if long_breakout {
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
        let len = len.max(256);
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
    fn validate_mkcs_params_rejects_invalid_values() {
        let mut cfg = MkcsStrategyParams::default();
        cfg.stddev_period = 0;
        assert!(validate_mkcs_params(&cfg).is_err());

        let mut cfg = MkcsStrategyParams::default();
        cfg.reexpansion_threshold = 0.005;
        cfg.compression_threshold = 0.01;
        assert!(validate_mkcs_params(&cfg).is_err());
    }

    #[test]
    fn validate_indicator_series_len_rejects_mismatch() {
        let err = validate_indicator_series_len("stddev", "value", &[1.0, 2.0], 3)
            .expect_err("mismatched series length should fail");
        assert!(err.contains("expected 3, got 2"));
        assert!(err.contains("stddev"));
        assert!(err.contains("value"));
    }

    #[test]
    fn validate_breakout_signal_lengths_rejects_close_length_mismatch() {
        let err = validate_breakout_signal_lengths("upper", &[1.0, 2.0], "lower", &[0.0, 1.0], 3)
            .expect_err("close length mismatch should fail");
        assert!(err.contains("expected 3, got 2"));
    }

    #[test]
    fn validate_breakout_signal_lengths_rejects_upper_lower_mismatch() {
        let err =
            validate_breakout_signal_lengths("upper", &[1.0, 2.0, 3.0], "lower", &[0.0, 1.0], 3)
                .expect_err("upper/lower mismatch should fail");
        assert!(err.contains("upper 'upper' has 3"));
        assert!(err.contains("lower 'lower' has 2"));
    }

    #[test]
    fn validate_distinct_output_ids_rejects_duplicate_binding() {
        let err = validate_distinct_output_ids(
            "donchian",
            "value",
            "value",
            &[
                "value".to_string(),
                "upper".to_string(),
                "lower".to_string(),
            ],
        )
        .expect_err("duplicate upper/lower binding should fail");
        assert!(err.contains("duplicate output ids"));
        assert!(err.contains("value"));
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn normalized_period_sweep_cardinality_normalizes_inputs() {
        assert_eq!(normalized_period_sweep_cardinality(0, 0, 0), 1);
        assert_eq!(normalized_period_sweep_cardinality(5, 2, 0), 1);
        assert_eq!(normalized_period_sweep_cardinality(5, 11, 3), 3);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn select_donchian_output_id_defaults_to_upper_for_unknown_preferences() {
        let ids = mkcs_indicator_ids().expect("mkcs indicator ids");
        assert_eq!(
            select_donchian_output_id(ids, &["unexpected_output_name"]),
            ids.donchian_upper_output.as_str()
        );
        assert_eq!(
            select_donchian_output_id(ids, &["lower"]),
            ids.donchian_lower_output.as_str()
        );
    }

    #[test]
    fn mkcs_indicator_ids_bind_distinct_donchian_outputs() {
        let ids = mkcs_indicator_ids().expect("mkcs indicator ids");
        assert_ne!(
            ids.donchian_upper_output.to_ascii_lowercase(),
            ids.donchian_lower_output.to_ascii_lowercase()
        );
    }

    #[test]
    fn build_mkcs_desired_positions_enters_and_exits_on_breakout_rules() {
        let close = vec![100.0, 100.2, 100.1, 101.5, 101.0, 100.4];
        let compression = MkcsCompressionSignals {
            stddev: vec![f64::NAN, 0.4, 0.3, 0.3, 3.5, 3.5],
            first_valid: 1,
        };
        let breakout = MkcsBreakoutSignals {
            upper: vec![f64::NAN, 100.5, 100.5, 100.6, 101.2, 101.2],
            lower: vec![f64::NAN, 99.5, 99.5, 99.4, 99.0, 99.0],
            first_valid: 1,
        };
        let mut strategy = StrategyConfig::default();
        strategy.trade_on_next_bar = false;
        strategy.allow_flip = true;
        let cfg = MkcsStrategyParams::default();
        let desired =
            build_mkcs_desired_positions(&close, &compression, &breakout, 2, &strategy, &cfg);
        assert_eq!(desired[3], 1);
        assert_eq!(desired[4], 0);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compute_mkcs_signals_gpu_smoke_runs_with_stddev_and_donchian_surfaces() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(512);
        let prepared = PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
            .expect("cuda inputs");
        let cfg = MkcsStrategyParams::default();
        let periods = resolve_mkcs_periods(&cfg, 12, 34);

        let compression = compute_mkcs_compression_signals(
            &candles,
            "close",
            &crate::double_ma::Backend::GpuOnly { device_id: 0 },
            periods.stddev,
            Some(&prepared),
        )
        .expect("mkcs gpu compression");
        let breakout = compute_mkcs_breakout_signals(
            &candles,
            "close",
            &crate::double_ma::Backend::GpuOnly { device_id: 0 },
            periods.breakout,
            Some(&prepared),
        )
        .expect("mkcs gpu breakout");

        assert_eq!(compression.stddev.len(), candles.close.len());
        assert_eq!(breakout.upper.len(), candles.close.len());
        assert_eq!(breakout.lower.len(), candles.close.len());
        assert!(compression.first_valid < candles.close.len());
        assert!(breakout.first_valid < candles.close.len());
        assert!(compression.stddev.iter().any(|v| v.is_finite()));
        assert!(breakout.upper.iter().any(|v| v.is_finite()));
        assert!(breakout.lower.iter().any(|v| v.is_finite()));
    }
}
