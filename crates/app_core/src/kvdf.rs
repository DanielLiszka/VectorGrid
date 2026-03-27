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
pub struct KvdfStrategyParams {
    #[serde(default = "default_kvdf_keltner_period")]
    pub keltner_period: u16,
    #[serde(default = "default_kvdf_keltner_mult")]
    pub keltner_mult: f64,
    #[serde(default = "default_kvdf_vwap_anchor")]
    pub vwap_anchor: String,
    #[serde(default = "default_kvdf_fvg_lookback")]
    pub fvg_lookback: u16,
    #[serde(default = "default_kvdf_devstop_period")]
    pub devstop_period: u16,
    #[serde(default = "default_kvdf_devstop_mult")]
    pub devstop_mult: f64,
    #[serde(default = "default_kvdf_dti_period")]
    pub dti_period: u16,
    #[serde(default = "default_kvdf_dti_threshold")]
    pub dti_threshold: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct KvdfResolvedPeriods {
    pub keltner: u16,
    pub fvg_lookback: u16,
    pub fvg_smoothing: u16,
    pub devstop: u16,
    pub dti_r: u16,
    pub dti_s: u16,
    pub dti_u: u16,
}

#[derive(Clone, Debug)]
pub struct KvdfSignals {
    pub keltner_upper: Vec<f64>,
    pub keltner_middle: Vec<f64>,
    pub keltner_lower: Vec<f64>,
    pub vwap: Vec<f64>,
    pub dti: Vec<f64>,
    pub devstop_long: Vec<f64>,
    pub devstop_short: Vec<f64>,
    pub fvg_upper_ts: Vec<f64>,
    pub fvg_lower_ts: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct KvdfSlowSignals {
    pub keltner_upper: Vec<f64>,
    pub keltner_middle: Vec<f64>,
    pub keltner_lower: Vec<f64>,
    pub devstop_long: Vec<f64>,
    pub devstop_short: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct KvdfFastSignals {
    pub fvg_upper_ts: Vec<f64>,
    pub fvg_lower_ts: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct KvdfSharedSignals {
    pub vwap: Vec<f64>,
    pub first_valid: usize,
}

fn default_kvdf_keltner_period() -> u16 {
    20
}

fn default_kvdf_keltner_mult() -> f64 {
    1.5
}

fn default_kvdf_vwap_anchor() -> String {
    "session".to_string()
}

fn default_kvdf_fvg_lookback() -> u16 {
    20
}

fn default_kvdf_devstop_period() -> u16 {
    14
}

fn default_kvdf_devstop_mult() -> f64 {
    2.0
}

fn default_kvdf_dti_period() -> u16 {
    14
}

fn default_kvdf_dti_threshold() -> f64 {
    20.0
}

impl Default for KvdfStrategyParams {
    fn default() -> Self {
        Self {
            keltner_period: default_kvdf_keltner_period(),
            keltner_mult: default_kvdf_keltner_mult(),
            vwap_anchor: default_kvdf_vwap_anchor(),
            fvg_lookback: default_kvdf_fvg_lookback(),
            devstop_period: default_kvdf_devstop_period(),
            devstop_mult: default_kvdf_devstop_mult(),
            dti_period: default_kvdf_dti_period(),
            dti_threshold: default_kvdf_dti_threshold(),
        }
    }
}

pub fn validate_kvdf_params(cfg: &KvdfStrategyParams) -> Result<(), String> {
    if cfg.keltner_period == 0 {
        return Err("KVDF keltner_period must be >= 1.".to_string());
    }
    if !cfg.keltner_mult.is_finite() || cfg.keltner_mult <= 0.0 {
        return Err("KVDF keltner_mult must be > 0.".to_string());
    }
    if cfg.vwap_anchor.trim().is_empty() {
        return Err("KVDF vwap_anchor must not be empty.".to_string());
    }
    if cfg.fvg_lookback == 0 {
        return Err("KVDF fvg_lookback must be >= 1.".to_string());
    }
    if cfg.devstop_period == 0 {
        return Err("KVDF devstop_period must be >= 1.".to_string());
    }
    if !cfg.devstop_mult.is_finite() || cfg.devstop_mult <= 0.0 {
        return Err("KVDF devstop_mult must be > 0.".to_string());
    }
    if cfg.dti_period == 0 {
        return Err("KVDF dti_period must be >= 1.".to_string());
    }
    if !cfg.dti_threshold.is_finite() || cfg.dti_threshold <= 0.0 {
        return Err("KVDF dti_threshold must be finite and > 0.".to_string());
    }
    Ok(())
}

#[inline]
pub fn normalized_kvdf_dti_threshold(cfg: &KvdfStrategyParams) -> f64 {
    cfg.dti_threshold.abs()
}

pub fn normalize_kvdf_vwap_anchor(anchor: &str) -> String {
    let anchor = anchor.trim().to_ascii_lowercase();
    match anchor.as_str() {
        "" | "session" => "1d".to_string(),
        other => other.to_string(),
    }
}

pub fn resolve_kvdf_periods(
    cfg: &KvdfStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> KvdfResolvedPeriods {
    let dti_r = slow_len.max(cfg.dti_period).max(2);
    let dti_s = fast_len.max((dti_r / 2).max(2));
    let dti_u = fast_len.max((dti_s / 2).max(2));
    KvdfResolvedPeriods {
        keltner: slow_len.max(cfg.keltner_period).max(2),
        fvg_lookback: fast_len.max(cfg.fvg_lookback).max(2),
        fvg_smoothing: fast_len.max((cfg.fvg_lookback / 2).max(3)).max(3),
        devstop: slow_len.max(cfg.devstop_period).max(2),
        dti_r,
        dti_s,
        dti_u,
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
fn cuda_series_to_f64(out: DynamicIndicatorCudaComputeOutput) -> Result<Vec<f64>, String> {
    match out.series {
        DynamicIndicatorCudaSeries::HostF32(v) => Ok(v.into_iter().map(|x| x as f64).collect()),
        DynamicIndicatorCudaSeries::DeviceF32(_) => {
            Err("KVDF expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn backend_prefers_gpu(backend: &Backend) -> bool {
    !matches!(backend, Backend::CpuOnly)
}

fn resolve_output_id(indicator_id: &str, preferred_ids: &[&str]) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("KVDF indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;
    if descriptor.outputs.is_empty() {
        return Err(format!(
            "KVDF indicator '{indicator_id}' has no declared outputs"
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
        format!("KVDF indicator '{indicator_id}' is not registered for dynamic dispatch")
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

pub fn compute_kvdf_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KvdfResolvedPeriods,
    cfg: &KvdfStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KvdfSignals, String> {
    let slow = compute_kvdf_slow_signals(
        candles,
        ma_source,
        backend,
        periods,
        cfg,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let shared = compute_kvdf_shared_signals(
        candles,
        ma_source,
        backend,
        cfg,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let dti = compute_kvdf_dti_signal(
        candles,
        ma_source,
        backend,
        periods,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let fast = compute_kvdf_fast_signals(
        candles,
        ma_source,
        backend,
        periods,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let dti_first_valid = first_finite_idx_local(&dti)
        .ok_or_else(|| "KVDF DTI signal is all non-finite".to_string())?;
    let first_valid = slow
        .first_valid
        .max(shared.first_valid)
        .max(dti_first_valid)
        .max(fast.first_valid);

    Ok(KvdfSignals {
        keltner_upper: slow.keltner_upper,
        keltner_middle: slow.keltner_middle,
        keltner_lower: slow.keltner_lower,
        vwap: shared.vwap,
        dti,
        devstop_long: slow.devstop_long,
        devstop_short: slow.devstop_short,
        fvg_upper_ts: fast.fvg_upper_ts,
        fvg_lower_ts: fast.fvg_lower_ts,
        first_valid,
    })
}

pub fn compute_kvdf_slow_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KvdfResolvedPeriods,
    cfg: &KvdfStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KvdfSlowSignals, String> {
    let keltner_upper_output = resolve_output_id("keltner", &["upper_band", "upperband", "upper"])?;
    let keltner_middle_output =
        resolve_output_id("keltner", &["middle_band", "middleband", "middle"])?;
    let keltner_lower_output = resolve_output_id("keltner", &["lower_band", "lowerband", "lower"])?;
    let devstop_output = resolve_output_id("devstop", &["value"])?;

    let keltner_params = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.keltner)),
        },
        IndicatorParamKV {
            key: "multiplier".to_string(),
            value: IndicatorParamValue::Float(cfg.keltner_mult),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString("ema".to_string()),
        },
    ];
    let devstop_long_params = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.devstop)),
        },
        IndicatorParamKV {
            key: "mult".to_string(),
            value: IndicatorParamValue::Float(cfg.devstop_mult),
        },
        IndicatorParamKV {
            key: "devtype".to_string(),
            value: IndicatorParamValue::Int(0),
        },
        IndicatorParamKV {
            key: "direction".to_string(),
            value: IndicatorParamValue::EnumString("long".to_string()),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString("sma".to_string()),
        },
    ];
    let devstop_short_params = vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.devstop)),
        },
        IndicatorParamKV {
            key: "mult".to_string(),
            value: IndicatorParamValue::Float(cfg.devstop_mult),
        },
        IndicatorParamKV {
            key: "devtype".to_string(),
            value: IndicatorParamValue::Int(0),
        },
        IndicatorParamKV {
            key: "direction".to_string(),
            value: IndicatorParamValue::EnumString("short".to_string()),
        },
        IndicatorParamKV {
            key: "ma_type".to_string(),
            value: IndicatorParamValue::EnumString("sma".to_string()),
        },
    ];

    let keltner_upper = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "keltner",
        keltner_upper_output.as_str(),
        keltner_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let keltner_middle = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "keltner",
        keltner_middle_output.as_str(),
        keltner_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let keltner_lower = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "keltner",
        keltner_lower_output.as_str(),
        keltner_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let devstop_long = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "devstop",
        devstop_output.as_str(),
        devstop_long_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let devstop_short = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "devstop",
        devstop_output.as_str(),
        devstop_short_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let first_valid = first_finite_idx_local(&keltner_upper)
        .zip(first_finite_idx_local(&keltner_middle))
        .zip(first_finite_idx_local(&keltner_lower))
        .zip(first_finite_idx_local(&devstop_long))
        .zip(first_finite_idx_local(&devstop_short))
        .map(|((((a, b), c), d), e)| a.max(b).max(c).max(d).max(e))
        .ok_or_else(|| "KVDF slow signals are all non-finite".to_string())?;

    Ok(KvdfSlowSignals {
        keltner_upper,
        keltner_middle,
        keltner_lower,
        devstop_long,
        devstop_short,
        first_valid,
    })
}

pub fn compute_kvdf_shared_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &KvdfStrategyParams,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KvdfSharedSignals, String> {
    let vwap_output = resolve_output_id("vwap", &["value", "vwap"])?;
    let vwap_params = vec![IndicatorParamKV {
        key: "anchor".to_string(),
        value: IndicatorParamValue::EnumString(normalize_kvdf_vwap_anchor(&cfg.vwap_anchor)),
    }];
    let vwap = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "vwap",
        vwap_output.as_str(),
        vwap_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let first_valid = first_finite_idx_local(&vwap)
        .ok_or_else(|| "KVDF VWAP signal is all non-finite".to_string())?;
    Ok(KvdfSharedSignals { vwap, first_valid })
}

pub fn compute_kvdf_dti_signal(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KvdfResolvedPeriods,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    let dti_output = resolve_output_id("dti", &["value", "dti"])?;
    let dti_params = vec![
        IndicatorParamKV {
            key: "r".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.dti_r)),
        },
        IndicatorParamKV {
            key: "s".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.dti_s)),
        },
        IndicatorParamKV {
            key: "u".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.dti_u)),
        },
    ];
    compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "dti",
        dti_output.as_str(),
        dti_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )
}

pub fn compute_kvdf_fast_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    periods: KvdfResolvedPeriods,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<KvdfFastSignals, String> {
    let fvg_upper_ts_output = resolve_output_id("fvg_trailing_stop", &["upper_ts", "upper"])?;
    let fvg_lower_ts_output = resolve_output_id("fvg_trailing_stop", &["lower_ts", "lower"])?;
    let fvg_params = vec![
        IndicatorParamKV {
            key: "unmitigated_fvg_lookback".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.fvg_lookback)),
        },
        IndicatorParamKV {
            key: "smoothing_length".to_string(),
            value: IndicatorParamValue::Int(i64::from(periods.fvg_smoothing)),
        },
        IndicatorParamKV {
            key: "reset_on_cross".to_string(),
            value: IndicatorParamValue::Bool(false),
        },
    ];
    let fvg_upper_ts = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "fvg_trailing_stop",
        fvg_upper_ts_output.as_str(),
        fvg_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let fvg_lower_ts = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "fvg_trailing_stop",
        fvg_lower_ts_output.as_str(),
        fvg_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let first_valid = first_finite_idx_local(&fvg_upper_ts)
        .zip(first_finite_idx_local(&fvg_lower_ts))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "KVDF fast signals are all non-finite".to_string())?;
    Ok(KvdfFastSignals {
        fvg_upper_ts,
        fvg_lower_ts,
        first_valid,
    })
}

pub fn compute_kvdf_start_t(
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

pub fn build_kvdf_desired_positions(
    close: &[f64],
    signals: &KvdfSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &KvdfStrategyParams,
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

        let close_now = close.get(sig_t).copied().unwrap_or(f64::NAN);
        let keltner_upper = signals
            .keltner_upper
            .get(sig_t)
            .copied()
            .unwrap_or(f64::NAN);
        let keltner_middle = signals
            .keltner_middle
            .get(sig_t)
            .copied()
            .unwrap_or(f64::NAN);
        let keltner_lower = signals
            .keltner_lower
            .get(sig_t)
            .copied()
            .unwrap_or(f64::NAN);
        let vwap_now = signals.vwap.get(sig_t).copied().unwrap_or(f64::NAN);
        let dti_now = signals.dti.get(sig_t).copied().unwrap_or(f64::NAN);
        let devstop_long = signals.devstop_long.get(sig_t).copied().unwrap_or(f64::NAN);
        let devstop_short = signals
            .devstop_short
            .get(sig_t)
            .copied()
            .unwrap_or(f64::NAN);
        let fvg_upper_ts = signals.fvg_upper_ts.get(sig_t).copied().unwrap_or(f64::NAN);
        let fvg_lower_ts = signals.fvg_lower_ts.get(sig_t).copied().unwrap_or(f64::NAN);

        let breakout_up = close_now.is_finite()
            && keltner_upper.is_finite()
            && vwap_now.is_finite()
            && close_now > keltner_upper
            && close_now > vwap_now;
        let breakout_down = close_now.is_finite()
            && keltner_lower.is_finite()
            && vwap_now.is_finite()
            && close_now < keltner_lower
            && close_now < vwap_now;
        let dti_threshold = normalized_kvdf_dti_threshold(cfg);
        let dti_up = dti_now.is_finite() && dti_now >= dti_threshold;
        let dti_down = dti_now.is_finite() && dti_now <= -dti_threshold;
        let long_stop_ok = !devstop_long.is_finite() || close_now > devstop_long;
        let short_stop_ok = !devstop_short.is_finite() || close_now < devstop_short;
        let long_fvg_ok = !fvg_lower_ts.is_finite() || close_now > fvg_lower_ts;
        let short_fvg_ok = !fvg_upper_ts.is_finite() || close_now < fvg_upper_ts;

        let long_entry = breakout_up && dti_up && long_stop_ok && long_fvg_ok;
        let short_entry =
            !strategy.long_only && breakout_down && dti_down && short_stop_ok && short_fvg_ok;

        if pos == 0 {
            if long_entry {
                pos = 1;
            } else if short_entry {
                pos = -1;
            }
        } else if pos > 0 {
            let hard_exit = (devstop_long.is_finite() && close_now <= devstop_long)
                || (fvg_lower_ts.is_finite() && close_now <= fvg_lower_ts);
            let opposing_votes =
                usize::from(keltner_middle.is_finite() && close_now < keltner_middle)
                    + usize::from(vwap_now.is_finite() && close_now < vwap_now)
                    + usize::from(dti_now.is_finite() && dti_now < 0.0);
            let mut exit_now = hard_exit || opposing_votes >= 2;
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
            let hard_exit = (devstop_short.is_finite() && close_now >= devstop_short)
                || (fvg_upper_ts.is_finite() && close_now >= fvg_upper_ts);
            let opposing_votes =
                usize::from(keltner_middle.is_finite() && close_now > keltner_middle)
                    + usize::from(vwap_now.is_finite() && close_now > vwap_now)
                    + usize::from(dti_now.is_finite() && dti_now > 0.0);
            let mut exit_now = hard_exit || opposing_votes >= 2;
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
    fn validate_kvdf_params_rejects_invalid_values() {
        let mut cfg = KvdfStrategyParams::default();
        cfg.keltner_mult = 0.0;
        assert!(validate_kvdf_params(&cfg).is_err());

        let mut cfg = KvdfStrategyParams::default();
        cfg.vwap_anchor = " ".to_string();
        assert!(validate_kvdf_params(&cfg).is_err());

        let mut cfg = KvdfStrategyParams::default();
        cfg.dti_threshold = 0.0;
        assert!(validate_kvdf_params(&cfg).is_err());

        let mut cfg = KvdfStrategyParams::default();
        cfg.dti_threshold = -20.0;
        assert!(validate_kvdf_params(&cfg).is_err());
    }

    #[test]
    fn build_kvdf_desired_positions_enters_and_exits_on_breakout_and_stop_rules() {
        let close = vec![100.0, 101.0, 103.0, 105.0, 104.0, 101.0, 99.0];
        let signals = KvdfSignals {
            keltner_upper: vec![f64::NAN, 101.5, 102.0, 103.0, 103.5, 103.0, 101.0],
            keltner_middle: vec![f64::NAN, 100.5, 101.2, 102.0, 102.2, 101.8, 100.4],
            keltner_lower: vec![f64::NAN, 99.5, 100.0, 101.0, 101.0, 100.4, 99.4],
            vwap: vec![f64::NAN, 100.8, 101.3, 102.1, 102.5, 101.9, 100.6],
            dti: vec![f64::NAN, 10.0, 24.0, 28.0, 8.0, -5.0, -22.0],
            devstop_long: vec![f64::NAN, 99.5, 100.8, 101.8, 102.2, 101.5, 100.0],
            devstop_short: vec![f64::NAN, 102.2, 103.6, 104.5, 104.0, 102.6, 100.8],
            fvg_upper_ts: vec![
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                102.8,
                100.6,
            ],
            fvg_lower_ts: vec![f64::NAN, 99.2, 100.4, 101.6, 102.0, f64::NAN, f64::NAN],
            first_valid: 1,
        };
        let mut strategy = StrategyConfig::default();
        strategy.trade_on_next_bar = false;
        strategy.allow_flip = false;
        let cfg = KvdfStrategyParams::default();

        let desired = build_kvdf_desired_positions(&close, &signals, 1, &strategy, &cfg);
        assert_eq!(desired[2], 1);
        assert_eq!(desired[5], 0);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn compute_kvdf_signals_gpu_smoke_runs_with_all_indicator_surfaces() {
        if !cuda_available() {
            return;
        }

        let mut candles = make_trending_candles(512);
        // Inject a few explicit three-candle gaps so the FVG surface has finite output rows.
        for i in (8..candles.close.len()).step_by(37) {
            let gap_low = candles.high[i - 2] + 2.0;
            candles.open[i] = gap_low + 0.2;
            candles.low[i] = gap_low;
            candles.high[i] = gap_low + 0.9;
            candles.close[i] = gap_low + 0.5;
        }
        let prepared = PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
            .expect("cuda inputs");
        let cfg = KvdfStrategyParams::default();
        let periods = resolve_kvdf_periods(&cfg, 12, 34);

        let signals = compute_kvdf_signals(
            &candles,
            "close",
            &crate::double_ma::Backend::GpuOnly { device_id: 0 },
            periods,
            &cfg,
            Some(&prepared),
        )
        .expect("kvdf gpu signals");

        assert_eq!(signals.keltner_upper.len(), candles.close.len());
        assert_eq!(signals.keltner_middle.len(), candles.close.len());
        assert_eq!(signals.keltner_lower.len(), candles.close.len());
        assert_eq!(signals.vwap.len(), candles.close.len());
        assert_eq!(signals.dti.len(), candles.close.len());
        assert_eq!(signals.devstop_long.len(), candles.close.len());
        assert_eq!(signals.devstop_short.len(), candles.close.len());
        assert_eq!(signals.fvg_upper_ts.len(), candles.close.len());
        assert_eq!(signals.fvg_lower_ts.len(), candles.close.len());
        assert!(signals.first_valid < candles.close.len());
        assert!(signals.keltner_upper.iter().any(|v| v.is_finite()));
        assert!(signals.vwap.iter().any(|v| v.is_finite()));
        assert!(signals.dti.iter().any(|v| v.is_finite()));
        assert!(signals.devstop_long.iter().any(|v| v.is_finite()));
        assert!(signals
            .fvg_lower_ts
            .iter()
            .any(|v| v.is_finite() || v.is_nan()));
    }
}
