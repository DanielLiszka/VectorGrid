use crate::double_ma::Backend;
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorComputeOutput, DynamicIndicatorSeries,
};
#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_with_inputs, DynamicIndicatorCudaComputeOutput,
    DynamicIndicatorCudaSeries, PreparedCudaIndicatorInputs,
};
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, IndicatorParamKV, IndicatorParamValue,
};
use serde::{Deserialize, Serialize};
#[cfg(all(test, feature = "cuda"))]
use std::cell::Cell;
#[cfg(feature = "cuda")]
use std::cell::RefCell;
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

/// KQSR strategy parameters.
///
/// Current runtime semantics intentionally remain broader than a stricter
/// product-policy envelope:
/// - exits in [`build_kqsr_desired_positions`] still pivot around the fixed
///   neutral `50.0`
/// - [`validate_kqsr_params`] accepts `prb_threshold` anywhere in `[0, 1]`, so
///   values above `0.5` create overlapping long/short PRB acceptance regions
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct KqsrStrategyParams {
    #[serde(default = "default_kqsr_kdj_period")]
    pub kdj_period: u16,
    #[serde(default = "default_kqsr_srsi_period")]
    pub srsi_period: u16,
    #[serde(default = "default_kqsr_srsi_smooth")]
    pub srsi_smooth: u16,
    #[serde(default = "default_kqsr_ift_rsi_period")]
    pub ift_rsi_period: u16,
    #[serde(default = "default_kqsr_qqe_period")]
    pub qqe_period: u16,
    #[serde(default = "default_kqsr_oversold")]
    pub oversold: f64,
    #[serde(default = "default_kqsr_overbought")]
    pub overbought: f64,
    #[serde(default = "default_kqsr_prb_period")]
    pub prb_period: u16,
    #[serde(default = "default_kqsr_prb_threshold")]
    pub prb_threshold: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct KqsrResolvedPeriods {
    pub kdj: u16,
    pub srsi: u16,
    pub ift_rsi: u16,
    pub qqe: u16,
    pub prb: u16,
}

#[derive(Clone, Debug)]
pub struct KqsrSignals {
    pub j: Vec<f64>,
    pub srsi_k: Vec<f64>,
    pub srsi_d: Vec<f64>,
    pub ift_rsi: Vec<f64>,
    pub qqe_fast: Vec<f64>,
    pub qqe_slow: Vec<f64>,
    pub prb_upper: Vec<f64>,
    pub prb_lower: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct KqsrFastSignals {
    pub j: Vec<f64>,
    pub srsi_k: Vec<f64>,
    pub srsi_d: Vec<f64>,
    pub ift_rsi: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct KqsrSlowSignals {
    pub qqe_fast: Vec<f64>,
    pub qqe_slow: Vec<f64>,
    pub prb_upper: Vec<f64>,
    pub prb_lower: Vec<f64>,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug)]
pub struct KqsrFastSignalsF32 {
    pub j: Vec<f32>,
    pub srsi_k: Vec<f32>,
    pub srsi_d: Vec<f32>,
    pub ift_rsi: Vec<f32>,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug)]
pub struct KqsrSlowSignalsF32 {
    pub qqe_fast: Vec<f32>,
    pub qqe_slow: Vec<f32>,
    pub prb_upper: Vec<f32>,
    pub prb_lower: Vec<f32>,
    pub first_valid: usize,
}

fn default_kqsr_kdj_period() -> u16 {
    14
}

fn default_kqsr_srsi_period() -> u16 {
    14
}

fn default_kqsr_srsi_smooth() -> u16 {
    3
}

fn default_kqsr_ift_rsi_period() -> u16 {
    9
}

fn default_kqsr_qqe_period() -> u16 {
    14
}

fn default_kqsr_oversold() -> f64 {
    20.0
}

fn default_kqsr_overbought() -> f64 {
    80.0
}

fn default_kqsr_prb_period() -> u16 {
    20
}

fn default_kqsr_prb_threshold() -> f64 {
    0.2
}

impl Default for KqsrStrategyParams {
    fn default() -> Self {
        Self {
            kdj_period: default_kqsr_kdj_period(),
            srsi_period: default_kqsr_srsi_period(),
            srsi_smooth: default_kqsr_srsi_smooth(),
            ift_rsi_period: default_kqsr_ift_rsi_period(),
            qqe_period: default_kqsr_qqe_period(),
            oversold: default_kqsr_oversold(),
            overbought: default_kqsr_overbought(),
            prb_period: default_kqsr_prb_period(),
            prb_threshold: default_kqsr_prb_threshold(),
        }
    }
}

/// Validates that the persisted/runtime KQSR configuration is structurally
/// usable without tightening historical compatibility.
///
/// This validator does not require `oversold < 50 < overbought` and does not
/// cap `prb_threshold` at `0.5`; callers that want stricter policy should
/// enforce it separately from this compatibility-oriented check.
pub fn validate_kqsr_params(cfg: &KqsrStrategyParams) -> Result<(), String> {
    if cfg.kdj_period == 0 {
        return Err("KQSR kdj_period must be >= 1.".to_string());
    }
    if cfg.srsi_period == 0 {
        return Err("KQSR srsi_period must be >= 1.".to_string());
    }
    if cfg.srsi_smooth == 0 {
        return Err("KQSR srsi_smooth must be >= 1.".to_string());
    }
    if cfg.ift_rsi_period == 0 {
        return Err("KQSR ift_rsi_period must be >= 1.".to_string());
    }
    if cfg.qqe_period == 0 {
        return Err("KQSR qqe_period must be >= 1.".to_string());
    }
    if !cfg.oversold.is_finite() || !cfg.overbought.is_finite() {
        return Err("KQSR oversold/overbought must be finite.".to_string());
    }
    if cfg.oversold < 0.0 || cfg.oversold > 100.0 {
        return Err("KQSR oversold must be in [0, 100].".to_string());
    }
    if cfg.overbought < 0.0 || cfg.overbought > 100.0 {
        return Err("KQSR overbought must be in [0, 100].".to_string());
    }
    if cfg.oversold >= cfg.overbought {
        return Err("KQSR oversold must be < overbought.".to_string());
    }
    if cfg.prb_period == 0 {
        return Err("KQSR prb_period must be >= 1.".to_string());
    }
    if !cfg.prb_threshold.is_finite() || !(0.0..=1.0).contains(&cfg.prb_threshold) {
        return Err("KQSR prb_threshold must be in [0, 1].".to_string());
    }
    Ok(())
}

pub fn resolve_kqsr_periods(
    cfg: &KqsrStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> KqsrResolvedPeriods {
    KqsrResolvedPeriods {
        kdj: fast_len.max(cfg.kdj_period).max(1),
        srsi: fast_len.max(cfg.srsi_period).max(1),
        ift_rsi: fast_len.max(cfg.ift_rsi_period).max(1),
        qqe: slow_len.max(cfg.qqe_period).max(1),
        prb: slow_len.max(cfg.prb_period).max(1),
    }
}

fn first_finite_idx_local(values: &[f64]) -> Option<usize> {
    values.iter().position(|v| v.is_finite())
}

#[cfg(feature = "cuda")]
fn first_finite_idx_local_f32(values: &[f32]) -> Option<usize> {
    values.iter().position(|v| v.is_finite())
}

fn max_first_valid_idx(series: &[&[f64]]) -> Option<usize> {
    series
        .iter()
        .filter_map(|values| first_finite_idx_local(values))
        .max()
}

#[cfg(feature = "cuda")]
fn max_first_valid_idx_f32(series: &[&[f32]]) -> Option<usize> {
    series
        .iter()
        .filter_map(|values| first_finite_idx_local_f32(values))
        .max()
}

fn ensure_indicator_series_len(
    indicator_id: &str,
    output_id: &str,
    actual_len: usize,
    expected_len: usize,
) -> Result<(), String> {
    if actual_len == expected_len {
        Ok(())
    } else {
        Err(format!(
            "KQSR indicator '{indicator_id}/{output_id}' returned {actual_len} values; expected {expected_len}"
        ))
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
fn series_to_f32(out: DynamicIndicatorComputeOutput) -> Vec<f32> {
    match out.series {
        DynamicIndicatorSeries::F64(v) => v.into_iter().map(|x| x as f32).collect(),
        DynamicIndicatorSeries::I32(v) => v.into_iter().map(|x| x as f32).collect(),
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
            Err("KQSR expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn cuda_series_to_host_f32(out: DynamicIndicatorCudaComputeOutput) -> Result<Vec<f32>, String> {
    match out.series {
        DynamicIndicatorCudaSeries::HostF32(v) => Ok(v),
        DynamicIndicatorCudaSeries::DeviceF32(_) => {
            Err("KQSR expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn backend_prefers_gpu(backend: &Backend) -> bool {
    !matches!(backend, Backend::CpuOnly)
}

#[cfg(feature = "cuda")]
thread_local! {
    static KQSR_ROUTE_DIAGNOSTICS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    #[cfg(test)]
    static FORCE_KQSR_CUDA_CPU_FALLBACK: Cell<bool> = const { Cell::new(false) };
}

#[cfg(feature = "cuda")]
fn record_kqsr_route_diagnostic(message: String) {
    KQSR_ROUTE_DIAGNOSTICS.with(|cell| {
        let mut diagnostics = cell.borrow_mut();
        if diagnostics.iter().any(|entry| entry == &message) {
            return;
        }
        diagnostics.push(message);
    });
}

#[cfg(feature = "cuda")]
pub(crate) fn clear_kqsr_route_diagnostics() {
    KQSR_ROUTE_DIAGNOSTICS.with(|cell| cell.borrow_mut().clear());
}

#[cfg(feature = "cuda")]
pub(crate) fn take_kqsr_route_diagnostics() -> Option<Vec<String>> {
    KQSR_ROUTE_DIAGNOSTICS.with(|cell| {
        let mut diagnostics = cell.borrow_mut();
        if diagnostics.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut *diagnostics))
        }
    })
}

#[cfg(not(feature = "cuda"))]
pub(crate) fn clear_kqsr_route_diagnostics() {}

#[cfg(not(feature = "cuda"))]
pub(crate) fn take_kqsr_route_diagnostics() -> Option<Vec<String>> {
    None
}

#[cfg(all(test, feature = "cuda"))]
pub(crate) fn set_force_kqsr_cuda_cpu_fallback_for_test(enabled: bool) {
    FORCE_KQSR_CUDA_CPU_FALLBACK.with(|cell| cell.set(enabled));
}

#[cfg(all(test, feature = "cuda"))]
fn force_kqsr_cuda_cpu_fallback_for_test() -> bool {
    FORCE_KQSR_CUDA_CPU_FALLBACK.with(|cell| cell.get())
}

#[cfg(all(feature = "cuda", not(test)))]
fn force_kqsr_cuda_cpu_fallback_for_test() -> bool {
    false
}

fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("KQSR indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "KQSR indicator '{indicator_id}' has no declared outputs"
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
        format!("KQSR indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    #[cfg(feature = "cuda")]
    {
        let prefer_gpu = backend_prefers_gpu(backend)
            && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);
        if prefer_gpu {
            if let Some(prepared) = prepared_cuda_inputs {
                let out = if force_kqsr_cuda_cpu_fallback_for_test() {
                    Err(crate::dynamic_indicator_adapter::DynamicIndicatorAdapterError::ComputeFailure {
                        indicator_id: indicator_id.to_string(),
                        details: "forced test fallback".to_string(),
                    })
                } else {
                    compute_dynamic_indicator_cuda_with_inputs(
                        prepared,
                        indicator_id,
                        Some(output_id),
                        params,
                        Kernel::Auto,
                        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                    )
                };
                match out {
                    Ok(values) => return cuda_series_to_f64(values),
                    Err(err) => record_kqsr_route_diagnostic(format!(
                        "KQSR indicator {indicator_id}/{output_id} CUDA helper fell back to CPU: {}",
                        err
                    )),
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

#[cfg(feature = "cuda")]
fn compute_indicator_f32(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f32>, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("KQSR indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    let prefer_gpu = backend_prefers_gpu(backend)
        && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);
    if prefer_gpu {
        if let Some(prepared) = prepared_cuda_inputs {
            let out = if force_kqsr_cuda_cpu_fallback_for_test() {
                Err(crate::dynamic_indicator_adapter::DynamicIndicatorAdapterError::ComputeFailure {
                    indicator_id: indicator_id.to_string(),
                    details: "forced test fallback".to_string(),
                })
            } else {
                compute_dynamic_indicator_cuda_with_inputs(
                    prepared,
                    indicator_id,
                    Some(output_id),
                    params,
                    Kernel::Auto,
                    vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                )
            };
            match out {
                Ok(values) => return cuda_series_to_host_f32(values),
                Err(err) => record_kqsr_route_diagnostic(format!(
                    "KQSR indicator {indicator_id}/{output_id} CUDA helper fell back to CPU: {}",
                    err
                )),
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
    Ok(series_to_f32(out))
}

pub fn compute_kqsr_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KqsrResolvedPeriods,
    cfg: &KqsrStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KqsrSignals, String> {
    let fast = compute_kqsr_fast_signals(
        candles,
        ma_source,
        backend,
        periods,
        cfg,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let slow = compute_kqsr_slow_signals(
        candles,
        ma_source,
        backend,
        periods,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let first_valid = fast.first_valid.max(slow.first_valid);

    Ok(KqsrSignals {
        j: fast.j,
        srsi_k: fast.srsi_k,
        srsi_d: fast.srsi_d,
        ift_rsi: fast.ift_rsi,
        qqe_fast: slow.qqe_fast,
        qqe_slow: slow.qqe_slow,
        prb_upper: slow.prb_upper,
        prb_lower: slow.prb_lower,
        first_valid,
    })
}

pub fn compute_kqsr_fast_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KqsrResolvedPeriods,
    cfg: &KqsrStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KqsrFastSignals, String> {
    let kdj_j_output = resolve_output_id("kdj", &["j"])?;
    let srsi_k_output = resolve_output_id("srsi", &["k"])?;
    let srsi_d_output = resolve_output_id("srsi", &["d"])?;
    let ift_rsi_output = resolve_output_id("ift_rsi", &["value", "ift_rsi"])?;

    let kdj_params = vec![
        IndicatorParamKV {
            key: "fast_k_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.kdj)),
        },
        IndicatorParamKV {
            key: "slow_k_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.srsi_smooth.max(1))),
        },
        IndicatorParamKV {
            key: "slow_k_ma_type".to_string(),
            value: IndicatorParamValue::EnumString("sma".to_string()),
        },
        IndicatorParamKV {
            key: "slow_d_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.srsi_smooth.max(1))),
        },
        IndicatorParamKV {
            key: "slow_d_ma_type".to_string(),
            value: IndicatorParamValue::EnumString("sma".to_string()),
        },
    ];
    let srsi_params = vec![
        IndicatorParamKV {
            key: "rsi_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.srsi)),
        },
        IndicatorParamKV {
            key: "stoch_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.srsi)),
        },
        IndicatorParamKV {
            key: "k".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.srsi_smooth.max(1))),
        },
        IndicatorParamKV {
            key: "d".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.srsi_smooth.max(1))),
        },
    ];
    let ift_rsi_params = vec![
        IndicatorParamKV {
            key: "rsi_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.ift_rsi)),
        },
        IndicatorParamKV {
            key: "wma_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.ift_rsi)),
        },
    ];

    let j = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "kdj",
        kdj_j_output.as_str(),
        kdj_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let srsi_k = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "srsi",
        srsi_k_output.as_str(),
        srsi_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let srsi_d = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "srsi",
        srsi_d_output.as_str(),
        srsi_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let ift_rsi = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "ift_rsi",
        ift_rsi_output.as_str(),
        ift_rsi_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let expected_len = candles.close.len();
    ensure_indicator_series_len("kdj", kdj_j_output.as_str(), j.len(), expected_len)?;
    ensure_indicator_series_len("srsi", srsi_k_output.as_str(), srsi_k.len(), expected_len)?;
    ensure_indicator_series_len("srsi", srsi_d_output.as_str(), srsi_d.len(), expected_len)?;
    ensure_indicator_series_len(
        "ift_rsi",
        ift_rsi_output.as_str(),
        ift_rsi.len(),
        expected_len,
    )?;
    let first_valid = max_first_valid_idx(&[
        j.as_slice(),
        srsi_k.as_slice(),
        srsi_d.as_slice(),
        ift_rsi.as_slice(),
    ])
    .ok_or_else(|| "KQSR fast signals are all non-finite".to_string())?;

    Ok(KqsrFastSignals {
        j,
        srsi_k,
        srsi_d,
        ift_rsi,
        first_valid,
    })
}

pub fn compute_kqsr_slow_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KqsrResolvedPeriods,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KqsrSlowSignals, String> {
    let qqe_fast_output = resolve_output_id("qqe", &["fast"])?;
    let qqe_slow_output = resolve_output_id("qqe", &["slow"])?;
    let prb_upper_output = resolve_output_id("prb", &["upper_band", "upper"])?;
    let prb_lower_output = resolve_output_id("prb", &["lower_band", "lower"])?;

    let qqe_params = vec![
        IndicatorParamKV {
            key: "rsi_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.qqe)),
        },
        IndicatorParamKV {
            key: "smoothing_factor".to_string(),
            value: IndicatorParamValue::Int(5),
        },
        IndicatorParamKV {
            key: "fast_factor".to_string(),
            value: IndicatorParamValue::Float(4.236),
        },
    ];
    let prb_params = vec![
        IndicatorParamKV {
            key: "smooth_data".to_string(),
            value: IndicatorParamValue::Bool(true),
        },
        IndicatorParamKV {
            key: "smooth_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.prb)),
        },
        IndicatorParamKV {
            key: "regression_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.prb)),
        },
        IndicatorParamKV {
            key: "polynomial_order".to_string(),
            value: IndicatorParamValue::Int(2),
        },
        IndicatorParamKV {
            key: "regression_offset".to_string(),
            value: IndicatorParamValue::Int(0),
        },
        IndicatorParamKV {
            key: "ndev".to_string(),
            value: IndicatorParamValue::Float(2.0),
        },
        IndicatorParamKV {
            key: "equ_from".to_string(),
            value: IndicatorParamValue::Int(0),
        },
    ];

    let qqe_fast = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "qqe",
        qqe_fast_output.as_str(),
        qqe_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let qqe_slow = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "qqe",
        qqe_slow_output.as_str(),
        qqe_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let prb_upper = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "prb",
        prb_upper_output.as_str(),
        prb_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let prb_lower = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "prb",
        prb_lower_output.as_str(),
        prb_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let expected_len = candles.close.len();
    ensure_indicator_series_len(
        "qqe",
        qqe_fast_output.as_str(),
        qqe_fast.len(),
        expected_len,
    )?;
    ensure_indicator_series_len(
        "qqe",
        qqe_slow_output.as_str(),
        qqe_slow.len(),
        expected_len,
    )?;
    ensure_indicator_series_len(
        "prb",
        prb_upper_output.as_str(),
        prb_upper.len(),
        expected_len,
    )?;
    ensure_indicator_series_len(
        "prb",
        prb_lower_output.as_str(),
        prb_lower.len(),
        expected_len,
    )?;

    let first_valid = max_first_valid_idx(&[
        qqe_fast.as_slice(),
        qqe_slow.as_slice(),
        prb_upper.as_slice(),
        prb_lower.as_slice(),
    ])
    .ok_or_else(|| "KQSR slow signals are all non-finite".to_string())?;

    Ok(KqsrSlowSignals {
        qqe_fast,
        qqe_slow,
        prb_upper,
        prb_lower,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_kqsr_fast_signals_f32(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KqsrResolvedPeriods,
    cfg: &KqsrStrategyParams,
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KqsrFastSignalsF32, String> {
    let kdj_j_output = resolve_output_id("kdj", &["j"])?;
    let srsi_k_output = resolve_output_id("srsi", &["k"])?;
    let srsi_d_output = resolve_output_id("srsi", &["d"])?;
    let ift_rsi_output = resolve_output_id("ift_rsi", &["value", "ift_rsi"])?;

    let kdj_params = vec![
        IndicatorParamKV {
            key: "fast_k_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.kdj)),
        },
        IndicatorParamKV {
            key: "slow_k_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.srsi_smooth.max(1))),
        },
        IndicatorParamKV {
            key: "slow_k_ma_type".to_string(),
            value: IndicatorParamValue::EnumString("sma".to_string()),
        },
        IndicatorParamKV {
            key: "slow_d_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.srsi_smooth.max(1))),
        },
        IndicatorParamKV {
            key: "slow_d_ma_type".to_string(),
            value: IndicatorParamValue::EnumString("sma".to_string()),
        },
    ];
    let srsi_params = vec![
        IndicatorParamKV {
            key: "rsi_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.srsi)),
        },
        IndicatorParamKV {
            key: "stoch_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.srsi)),
        },
        IndicatorParamKV {
            key: "k".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.srsi_smooth.max(1))),
        },
        IndicatorParamKV {
            key: "d".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.srsi_smooth.max(1))),
        },
    ];
    let ift_rsi_params = vec![
        IndicatorParamKV {
            key: "rsi_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.ift_rsi)),
        },
        IndicatorParamKV {
            key: "wma_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.ift_rsi)),
        },
    ];

    let j = compute_indicator_f32(
        candles,
        ma_source,
        backend,
        "kdj",
        kdj_j_output.as_str(),
        kdj_params.as_slice(),
        prepared_cuda_inputs,
    )?;
    let srsi_k = compute_indicator_f32(
        candles,
        ma_source,
        backend,
        "srsi",
        srsi_k_output.as_str(),
        srsi_params.as_slice(),
        prepared_cuda_inputs,
    )?;
    let srsi_d = compute_indicator_f32(
        candles,
        ma_source,
        backend,
        "srsi",
        srsi_d_output.as_str(),
        srsi_params.as_slice(),
        prepared_cuda_inputs,
    )?;
    let ift_rsi = compute_indicator_f32(
        candles,
        ma_source,
        backend,
        "ift_rsi",
        ift_rsi_output.as_str(),
        ift_rsi_params.as_slice(),
        prepared_cuda_inputs,
    )?;
    let expected_len = candles.close.len();
    ensure_indicator_series_len("kdj", kdj_j_output.as_str(), j.len(), expected_len)?;
    ensure_indicator_series_len("srsi", srsi_k_output.as_str(), srsi_k.len(), expected_len)?;
    ensure_indicator_series_len("srsi", srsi_d_output.as_str(), srsi_d.len(), expected_len)?;
    ensure_indicator_series_len(
        "ift_rsi",
        ift_rsi_output.as_str(),
        ift_rsi.len(),
        expected_len,
    )?;
    let first_valid = max_first_valid_idx_f32(&[
        j.as_slice(),
        srsi_k.as_slice(),
        srsi_d.as_slice(),
        ift_rsi.as_slice(),
    ])
    .ok_or_else(|| "KQSR fast signals are all non-finite".to_string())?;

    Ok(KqsrFastSignalsF32 {
        j,
        srsi_k,
        srsi_d,
        ift_rsi,
        first_valid,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_kqsr_slow_signals_f32(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KqsrResolvedPeriods,
    prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KqsrSlowSignalsF32, String> {
    let qqe_fast_output = resolve_output_id("qqe", &["fast"])?;
    let qqe_slow_output = resolve_output_id("qqe", &["slow"])?;
    let prb_upper_output = resolve_output_id("prb", &["upper_band", "upper"])?;
    let prb_lower_output = resolve_output_id("prb", &["lower_band", "lower"])?;

    let qqe_params = vec![
        IndicatorParamKV {
            key: "rsi_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.qqe)),
        },
        IndicatorParamKV {
            key: "smoothing_factor".to_string(),
            value: IndicatorParamValue::Int(5),
        },
        IndicatorParamKV {
            key: "fast_factor".to_string(),
            value: IndicatorParamValue::Float(4.236),
        },
    ];
    let prb_params = vec![
        IndicatorParamKV {
            key: "smooth_data".to_string(),
            value: IndicatorParamValue::Bool(true),
        },
        IndicatorParamKV {
            key: "smooth_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.prb)),
        },
        IndicatorParamKV {
            key: "regression_period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.prb)),
        },
        IndicatorParamKV {
            key: "polynomial_order".to_string(),
            value: IndicatorParamValue::Int(2),
        },
        IndicatorParamKV {
            key: "regression_offset".to_string(),
            value: IndicatorParamValue::Int(0),
        },
        IndicatorParamKV {
            key: "ndev".to_string(),
            value: IndicatorParamValue::Float(2.0),
        },
        IndicatorParamKV {
            key: "equ_from".to_string(),
            value: IndicatorParamValue::Int(0),
        },
    ];

    let qqe_fast = compute_indicator_f32(
        candles,
        ma_source,
        backend,
        "qqe",
        qqe_fast_output.as_str(),
        qqe_params.as_slice(),
        prepared_cuda_inputs,
    )?;
    let qqe_slow = compute_indicator_f32(
        candles,
        ma_source,
        backend,
        "qqe",
        qqe_slow_output.as_str(),
        qqe_params.as_slice(),
        prepared_cuda_inputs,
    )?;
    let prb_upper = compute_indicator_f32(
        candles,
        ma_source,
        backend,
        "prb",
        prb_upper_output.as_str(),
        prb_params.as_slice(),
        prepared_cuda_inputs,
    )?;
    let prb_lower = compute_indicator_f32(
        candles,
        ma_source,
        backend,
        "prb",
        prb_lower_output.as_str(),
        prb_params.as_slice(),
        prepared_cuda_inputs,
    )?;
    let expected_len = candles.close.len();
    ensure_indicator_series_len(
        "qqe",
        qqe_fast_output.as_str(),
        qqe_fast.len(),
        expected_len,
    )?;
    ensure_indicator_series_len(
        "qqe",
        qqe_slow_output.as_str(),
        qqe_slow.len(),
        expected_len,
    )?;
    ensure_indicator_series_len(
        "prb",
        prb_upper_output.as_str(),
        prb_upper.len(),
        expected_len,
    )?;
    ensure_indicator_series_len(
        "prb",
        prb_lower_output.as_str(),
        prb_lower.len(),
        expected_len,
    )?;

    let first_valid = max_first_valid_idx_f32(&[
        qqe_fast.as_slice(),
        qqe_slow.as_slice(),
        prb_upper.as_slice(),
        prb_lower.as_slice(),
    ])
    .ok_or_else(|| "KQSR slow signals are all non-finite".to_string())?;

    Ok(KqsrSlowSignalsF32 {
        qqe_fast,
        qqe_slow,
        prb_upper,
        prb_lower,
        first_valid,
    })
}

/// Computes the first tradeable bar for KQSR evaluation.
///
/// When `trade_on_next_bar` is enabled, a valid terminal outcome is
/// `start_t == close.len()`: the first usable signal may arrive on the last
/// bar, leaving no remaining tradeable bar. Downstream callers should treat
/// that as a legitimate all-flat case rather than as an error by itself.
pub fn compute_kqsr_start_t(
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

fn prb_band_fraction(close: f64, lower: f64, upper: f64) -> f64 {
    if !close.is_finite() || !lower.is_finite() || !upper.is_finite() {
        return f64::NAN;
    }
    let width = upper - lower;
    if !width.is_finite() || width.abs() <= 1.0e-12 {
        return f64::NAN;
    }
    (close - lower) / width
}

/// Builds the desired-position vector for the already-computed KQSR signals.
///
/// Current semantics are intentionally preserved:
/// - long and short exits both pivot around the fixed neutral `50.0`
/// - `prb_threshold > 0.5` creates overlapping PRB acceptance regions for long
///   and short entry checks
/// - returning an all-flat vector is legitimate when `start_t >= close.len()`
///   and no tradeable bar remains
///
/// Malformed signal shapes should be rejected earlier by the fallible signal
/// builders. This helper keeps its flat-output API for compatibility.
pub fn build_kqsr_desired_positions(
    close: &[f64],
    signals: &KqsrSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &KqsrStrategyParams,
) -> Vec<i32> {
    let n = close.len();
    let mut desired = vec![0i32; n];
    if start_t >= n
        || signals.j.len() < n
        || signals.srsi_k.len() < n
        || signals.srsi_d.len() < n
        || signals.ift_rsi.len() < n
        || signals.qqe_fast.len() < n
        || signals.qqe_slow.len() < n
        || signals.prb_upper.len() < n
        || signals.prb_lower.len() < n
    {
        return desired;
    }

    let trade_offset = if strategy.trade_on_next_bar {
        1usize
    } else {
        0usize
    };
    let neutral = 50.0;
    let mut pos = 0i32;
    let j = signals.j.as_slice();
    let srsi_k = signals.srsi_k.as_slice();
    let srsi_d = signals.srsi_d.as_slice();
    let ift_rsi = signals.ift_rsi.as_slice();
    let qqe_fast = signals.qqe_fast.as_slice();
    let qqe_slow = signals.qqe_slow.as_slice();
    let prb_upper = signals.prb_upper.as_slice();
    let prb_lower = signals.prb_lower.as_slice();

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close[sig_t];
        let j_now = j[sig_t];
        let j_prev = j[prev_t];
        let srsi_k_now = srsi_k[sig_t];
        let srsi_k_prev = srsi_k[prev_t];
        let srsi_d_now = srsi_d[sig_t];
        let ift_now = ift_rsi[sig_t];
        let qqe_fast_now = qqe_fast[sig_t];
        let qqe_slow_now = qqe_slow[sig_t];
        let prb_upper_now = prb_upper[sig_t];
        let prb_lower_now = prb_lower[sig_t];

        let prb_frac = prb_band_fraction(close_now, prb_lower_now, prb_upper_now);
        let prb_long_ok = !prb_frac.is_finite() || prb_frac <= cfg.prb_threshold;
        let prb_short_ok = !prb_frac.is_finite() || prb_frac >= (1.0 - cfg.prb_threshold);

        let long_entry = j_prev.is_finite()
            && j_now.is_finite()
            && j_prev < cfg.oversold
            && j_now >= cfg.oversold
            && srsi_k_now.is_finite()
            && srsi_d_now.is_finite()
            && srsi_k_prev.is_finite()
            && srsi_k_now > srsi_d_now
            && srsi_k_now > srsi_k_prev
            && ift_now.is_finite()
            && ift_now > 0.0
            && qqe_fast_now.is_finite()
            && qqe_slow_now.is_finite()
            && qqe_fast_now > qqe_slow_now
            && prb_long_ok;

        let short_entry = !strategy.long_only
            && j_prev.is_finite()
            && j_now.is_finite()
            && j_prev > cfg.overbought
            && j_now <= cfg.overbought
            && srsi_k_now.is_finite()
            && srsi_d_now.is_finite()
            && srsi_k_prev.is_finite()
            && srsi_k_now < srsi_d_now
            && srsi_k_now < srsi_k_prev
            && ift_now.is_finite()
            && ift_now < 0.0
            && qqe_fast_now.is_finite()
            && qqe_slow_now.is_finite()
            && qqe_fast_now < qqe_slow_now
            && prb_short_ok;

        if pos == 0 {
            if long_entry {
                pos = 1;
            } else if short_entry {
                pos = -1;
            }
        } else if pos > 0 {
            let mut exit_now = (qqe_fast_now.is_finite()
                && qqe_slow_now.is_finite()
                && qqe_fast_now < qqe_slow_now)
                || (j_now.is_finite() && j_now >= neutral);
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
            let mut exit_now = (qqe_fast_now.is_finite()
                && qqe_slow_now.is_finite()
                && qqe_fast_now > qqe_slow_now)
                || (j_now.is_finite() && j_now <= neutral);
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
    fn validate_kqsr_params_rejects_invalid_values() {
        let mut cfg = KqsrStrategyParams::default();
        cfg.kdj_period = 0;
        assert!(validate_kqsr_params(&cfg).is_err());

        let mut cfg = KqsrStrategyParams::default();
        cfg.oversold = 90.0;
        cfg.overbought = 80.0;
        assert!(validate_kqsr_params(&cfg).is_err());
    }

    #[test]
    fn build_kqsr_desired_positions_enters_and_exits_on_reversal_rules() {
        let close = vec![100.0, 99.5, 99.8, 100.2, 100.7, 100.4, 100.1];
        let signals = KqsrSignals {
            j: vec![f64::NAN, 10.0, 25.0, 40.0, 55.0, 82.0, 60.0],
            srsi_k: vec![f64::NAN, 12.0, 24.0, 41.0, 58.0, 76.0, 52.0],
            srsi_d: vec![f64::NAN, 18.0, 20.0, 30.0, 49.0, 72.0, 57.0],
            ift_rsi: vec![f64::NAN, -0.4, 0.2, 0.4, 0.5, 0.1, -0.2],
            qqe_fast: vec![f64::NAN, -1.0, 1.0, 1.2, 1.3, -0.2, -0.4],
            qqe_slow: vec![f64::NAN, 0.0, 0.2, 0.4, 0.6, 0.1, 0.0],
            prb_upper: vec![f64::NAN, 103.0, 103.0, 103.0, 103.0, 103.0, 103.0],
            prb_lower: vec![f64::NAN, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0],
            first_valid: 1,
        };
        let mut strategy = StrategyConfig::default();
        strategy.trade_on_next_bar = false;
        strategy.allow_flip = true;
        let cfg = KqsrStrategyParams::default();

        let desired = build_kqsr_desired_positions(&close, &signals, 1, &strategy, &cfg);
        assert_eq!(desired[2], 1);
        assert_eq!(desired[4], 0);
    }

    #[test]
    fn build_kqsr_desired_positions_keeps_legitimate_no_trade_terminal_case_flat() {
        let close = vec![100.0, 100.2, 100.4, 100.6, 100.8];
        let signals = KqsrSignals {
            j: vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, 49.0],
            srsi_k: vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, 49.5],
            srsi_d: vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, 49.5],
            ift_rsi: vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, 0.0],
            qqe_fast: vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, 0.0],
            qqe_slow: vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, 0.0],
            prb_upper: vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, 101.0],
            prb_lower: vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN, 99.0],
            first_valid: 4,
        };
        let mut strategy = StrategyConfig::default();
        strategy.trade_on_next_bar = true;
        let cfg = KqsrStrategyParams::default();

        let start_t = compute_kqsr_start_t(4, signals.first_valid, strategy.trade_on_next_bar);
        assert_eq!(start_t, close.len());

        let desired = build_kqsr_desired_positions(&close, &signals, start_t, &strategy, &cfg);
        assert_eq!(desired, vec![0; close.len()]);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compute_kqsr_signals_gpu_smoke_runs_with_qqe_fast_and_slow() {
        if !cuda_available() {
            return;
        }

        let candles = make_trending_candles(512);
        let prepared = PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
            .expect("cuda inputs");
        let cfg = KqsrStrategyParams::default();
        let periods = resolve_kqsr_periods(&cfg, 12, 34);

        let signals = compute_kqsr_signals(
            &candles,
            "close",
            &Backend::GpuOnly { device_id: 0 },
            periods,
            &cfg,
            Some(&prepared),
        )
        .expect("kqsr gpu signals");

        assert_eq!(signals.qqe_fast.len(), candles.close.len());
        assert_eq!(signals.qqe_slow.len(), candles.close.len());
        assert!(signals.first_valid < candles.close.len());
        assert!(signals.qqe_fast.iter().any(|v| v.is_finite()));
        assert!(signals.qqe_slow.iter().any(|v| v.is_finite()));
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compute_kqsr_signals_records_route_diagnostics_on_cuda_cpu_fallback() {
        if !cuda_available() {
            return;
        }

        struct ResetKqsrCpuFallback;

        impl Drop for ResetKqsrCpuFallback {
            fn drop(&mut self) {
                set_force_kqsr_cuda_cpu_fallback_for_test(false);
                clear_kqsr_route_diagnostics();
            }
        }

        clear_kqsr_route_diagnostics();
        set_force_kqsr_cuda_cpu_fallback_for_test(true);
        let _reset = ResetKqsrCpuFallback;

        let candles = make_trending_candles(512);
        let prepared = PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
            .expect("cuda inputs");
        let cfg = KqsrStrategyParams::default();
        let periods = resolve_kqsr_periods(&cfg, 12, 34);

        let signals = compute_kqsr_signals(
            &candles,
            "close",
            &Backend::GpuOnly { device_id: 0 },
            periods,
            &cfg,
            Some(&prepared),
        )
        .expect("kqsr fallback signals");

        assert_eq!(signals.qqe_fast.len(), candles.close.len());
        let diagnostics = take_kqsr_route_diagnostics().unwrap_or_default();
        assert!(
            diagnostics.iter().any(|entry| {
                entry.contains("KQSR indicator")
                    && entry.contains("CUDA helper fell back to CPU")
                    && entry.contains("forced test fallback")
            }),
            "expected KQSR helper fallback diagnostic, got {:?}",
            diagnostics
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compute_kqsr_fast_signals_f32_cpu_fallback_matches_f64() {
        let candles = make_trending_candles(512);
        let cfg = KqsrStrategyParams::default();
        let periods = resolve_kqsr_periods(&cfg, 12, 34);

        let cpu_f64 =
            compute_kqsr_fast_signals(&candles, "close", &Backend::CpuOnly, periods, &cfg, None)
                .expect("cpu f64 fast signals");
        let cpu_f32 = compute_kqsr_fast_signals_f32(
            &candles,
            "close",
            &Backend::CpuOnly,
            periods,
            &cfg,
            None,
        )
        .expect("cpu f32 fast signals");

        assert_eq!(cpu_f64.first_valid, cpu_f32.first_valid);
        assert_eq!(cpu_f64.j.len(), cpu_f32.j.len());
        for (lhs, rhs) in cpu_f64.j.iter().zip(cpu_f32.j.iter()) {
            assert_eq!(lhs.is_finite(), rhs.is_finite());
            if lhs.is_finite() {
                assert!(((*lhs as f32) - *rhs).abs() <= 1.0e-5);
            }
        }
        for (lhs, rhs) in cpu_f64.srsi_k.iter().zip(cpu_f32.srsi_k.iter()) {
            assert_eq!(lhs.is_finite(), rhs.is_finite());
            if lhs.is_finite() {
                assert!(((*lhs as f32) - *rhs).abs() <= 1.0e-5);
            }
        }
        for (lhs, rhs) in cpu_f64.srsi_d.iter().zip(cpu_f32.srsi_d.iter()) {
            assert_eq!(lhs.is_finite(), rhs.is_finite());
            if lhs.is_finite() {
                assert!(((*lhs as f32) - *rhs).abs() <= 1.0e-5);
            }
        }
        for (lhs, rhs) in cpu_f64.ift_rsi.iter().zip(cpu_f32.ift_rsi.iter()) {
            assert_eq!(lhs.is_finite(), rhs.is_finite());
            if lhs.is_finite() {
                assert!(((*lhs as f32) - *rhs).abs() <= 1.0e-5);
            }
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compute_kqsr_slow_signals_f32_cpu_fallback_matches_f64() {
        let candles = make_trending_candles(512);
        let cfg = KqsrStrategyParams::default();
        let periods = resolve_kqsr_periods(&cfg, 12, 34);

        let cpu_f64 =
            compute_kqsr_slow_signals(&candles, "close", &Backend::CpuOnly, periods, None)
                .expect("cpu f64 slow signals");
        let cpu_f32 =
            compute_kqsr_slow_signals_f32(&candles, "close", &Backend::CpuOnly, periods, None)
                .expect("cpu f32 slow signals");

        assert_eq!(cpu_f64.first_valid, cpu_f32.first_valid);
        assert_eq!(cpu_f64.qqe_fast.len(), cpu_f32.qqe_fast.len());
        for (lhs, rhs) in cpu_f64.qqe_fast.iter().zip(cpu_f32.qqe_fast.iter()) {
            assert_eq!(lhs.is_finite(), rhs.is_finite());
            if lhs.is_finite() {
                assert!(((*lhs as f32) - *rhs).abs() <= 1.0e-5);
            }
        }
        for (lhs, rhs) in cpu_f64.qqe_slow.iter().zip(cpu_f32.qqe_slow.iter()) {
            assert_eq!(lhs.is_finite(), rhs.is_finite());
            if lhs.is_finite() {
                assert!(((*lhs as f32) - *rhs).abs() <= 1.0e-5);
            }
        }
        for (lhs, rhs) in cpu_f64.prb_upper.iter().zip(cpu_f32.prb_upper.iter()) {
            assert_eq!(lhs.is_finite(), rhs.is_finite());
            if lhs.is_finite() {
                assert!(((*lhs as f32) - *rhs).abs() <= 1.0e-5);
            }
        }
        for (lhs, rhs) in cpu_f64.prb_lower.iter().zip(cpu_f32.prb_lower.iter()) {
            assert_eq!(lhs.is_finite(), rhs.is_finite());
            if lhs.is_finite() {
                assert!(((*lhs as f32) - *rhs).abs() <= 1.0e-5);
            }
        }
    }
}
