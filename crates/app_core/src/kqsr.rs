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
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

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
            let out = compute_dynamic_indicator_cuda_with_inputs(
                prepared,
                indicator_id,
                Some(output_id),
                params,
                Kernel::Auto,
                vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
            );
            if let Ok(values) = out {
                return cuda_series_to_host_f32(values);
            }
        }
    }

    Ok(compute_indicator_f64(
        candles,
        ma_source,
        backend,
        indicator_id,
        output_id,
        params,
        prepared_cuda_inputs,
    )?
    .into_iter()
    .map(|v| v as f32)
    .collect())
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
    let first_valid = first_finite_idx_local(&j)
        .zip(first_finite_idx_local(&srsi_k))
        .zip(first_finite_idx_local(&srsi_d))
        .zip(first_finite_idx_local(&ift_rsi))
        .map(|(((a, b), c), d)| a.max(b).max(c).max(d))
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

    let first_valid = first_finite_idx_local(&qqe_fast)
        .zip(first_finite_idx_local(&qqe_slow))
        .zip(first_finite_idx_local(&prb_upper))
        .zip(first_finite_idx_local(&prb_lower))
        .map(|(((a, b), c), d)| a.max(b).max(c).max(d))
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
    let first_valid = first_finite_idx_local_f32(&j)
        .zip(first_finite_idx_local_f32(&srsi_k))
        .zip(first_finite_idx_local_f32(&srsi_d))
        .zip(first_finite_idx_local_f32(&ift_rsi))
        .map(|(((a, b), c), d)| a.max(b).max(c).max(d))
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

    let first_valid = first_finite_idx_local_f32(&qqe_fast)
        .zip(first_finite_idx_local_f32(&qqe_slow))
        .zip(first_finite_idx_local_f32(&prb_upper))
        .zip(first_finite_idx_local_f32(&prb_lower))
        .map(|(((a, b), c), d)| a.max(b).max(c).max(d))
        .ok_or_else(|| "KQSR slow signals are all non-finite".to_string())?;

    Ok(KqsrSlowSignalsF32 {
        qqe_fast,
        qqe_slow,
        prb_upper,
        prb_lower,
        first_valid,
    })
}

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

pub fn build_kqsr_desired_positions(
    close: &[f64],
    signals: &KqsrSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &KqsrStrategyParams,
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
    let neutral = 50.0;
    let mut pos = 0i32;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close.get(sig_t).copied().unwrap_or(f64::NAN);
        let j_now = signals.j.get(sig_t).copied().unwrap_or(f64::NAN);
        let j_prev = signals.j.get(prev_t).copied().unwrap_or(f64::NAN);
        let srsi_k_now = signals.srsi_k.get(sig_t).copied().unwrap_or(f64::NAN);
        let srsi_k_prev = signals.srsi_k.get(prev_t).copied().unwrap_or(f64::NAN);
        let srsi_d_now = signals.srsi_d.get(sig_t).copied().unwrap_or(f64::NAN);
        let ift_now = signals.ift_rsi.get(sig_t).copied().unwrap_or(f64::NAN);
        let qqe_fast_now = signals.qqe_fast.get(sig_t).copied().unwrap_or(f64::NAN);
        let qqe_slow_now = signals.qqe_slow.get(sig_t).copied().unwrap_or(f64::NAN);
        let prb_upper_now = signals.prb_upper.get(sig_t).copied().unwrap_or(f64::NAN);
        let prb_lower_now = signals.prb_lower.get(sig_t).copied().unwrap_or(f64::NAN);

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
}
