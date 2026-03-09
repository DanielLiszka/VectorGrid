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
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

#[cfg(feature = "cuda")]
use vector_ta::cuda::{CudaSqueezeMomentum, CudaTtmSqueeze, CudaTtmTrend, DeviceArrayF32};
#[cfg(feature = "cuda")]
use vector_ta::indicators::dispatch::DeviceMatrixF32;
#[cfg(feature = "cuda")]
use vector_ta::indicators::{
    squeeze_momentum::SqueezeMomentumBatchRange, ttm_squeeze::TtmSqueezeBatchRange,
    ttm_trend::TtmTrendBatchRange,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VccStrategyParams {
    #[serde(default = "default_true")]
    pub map_fast_to_squeeze_period: bool,
    #[serde(default = "default_true")]
    pub map_fast_to_momentum_period: bool,
    #[serde(default = "default_true")]
    pub map_slow_to_trend_period: bool,

    #[serde(default = "default_vcc_squeeze_period")]
    pub squeeze_period: u16,
    #[serde(default = "default_vcc_momentum_period")]
    pub momentum_period: u16,
    #[serde(default = "default_vcc_momentum_bb_period")]
    pub momentum_bb_period: u16,
    #[serde(default = "default_vcc_trend_period")]
    pub trend_period: u16,

    #[serde(default = "default_vcc_squeeze_bb_mult")]
    pub squeeze_bb_mult: f64,
    #[serde(default = "default_vcc_squeeze_kc_mult")]
    pub squeeze_kc_mult: f64,

    #[serde(default = "default_vcc_confirm_bars")]
    pub confirm_bars: u16,
    #[serde(default = "default_true")]
    pub require_momentum_accel: bool,
    #[serde(default = "default_vcc_max_trade_bars")]
    pub max_trade_bars: u16,
}

fn default_true() -> bool {
    true
}

fn default_vcc_squeeze_period() -> u16 {
    20
}

fn default_vcc_momentum_period() -> u16 {
    20
}

fn default_vcc_momentum_bb_period() -> u16 {
    20
}

fn default_vcc_trend_period() -> u16 {
    6
}

fn default_vcc_squeeze_bb_mult() -> f64 {
    2.0
}

fn default_vcc_squeeze_kc_mult() -> f64 {
    1.5
}

fn default_vcc_confirm_bars() -> u16 {
    0
}

fn default_vcc_max_trade_bars() -> u16 {
    120
}

impl Default for VccStrategyParams {
    fn default() -> Self {
        Self {
            map_fast_to_squeeze_period: default_true(),
            map_fast_to_momentum_period: default_true(),
            map_slow_to_trend_period: default_true(),
            squeeze_period: default_vcc_squeeze_period(),
            momentum_period: default_vcc_momentum_period(),
            momentum_bb_period: default_vcc_momentum_bb_period(),
            trend_period: default_vcc_trend_period(),
            squeeze_bb_mult: default_vcc_squeeze_bb_mult(),
            squeeze_kc_mult: default_vcc_squeeze_kc_mult(),
            confirm_bars: default_vcc_confirm_bars(),
            require_momentum_accel: default_true(),
            max_trade_bars: default_vcc_max_trade_bars(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VccResolvedPeriods {
    pub squeeze: u16,
    pub momentum: u16,
    pub trend: u16,
}

#[derive(Clone, Debug)]
pub struct VccFastSignals {
    pub squeeze_state: Vec<f64>,
    pub momentum: Vec<f64>,
    pub first_valid: usize,
}

#[derive(Clone, Debug)]
pub struct VccTrendSignals {
    pub trend_state: Vec<f64>,
    pub first_valid: usize,
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
enum VccDeviceMatrixOwner {
    Dispatch(DeviceMatrixF32),
    Raw(DeviceArrayF32),
}

#[cfg(feature = "cuda")]
pub struct VccOwnedDeviceMatrix {
    pub device_ptr: u64,
    pub rows: usize,
    pub cols: usize,
    owner: VccDeviceMatrixOwner,
}

#[cfg(feature = "cuda")]
impl VccOwnedDeviceMatrix {
    #[allow(dead_code)]
    fn from_dispatch(matrix: DeviceMatrixF32) -> Self {
        Self {
            device_ptr: matrix.device_ptr,
            rows: matrix.rows,
            cols: matrix.cols,
            owner: VccDeviceMatrixOwner::Dispatch(matrix),
        }
    }

    fn from_raw(matrix: DeviceArrayF32) -> Self {
        Self {
            device_ptr: matrix.device_ptr(),
            rows: matrix.rows,
            cols: matrix.cols,
            owner: VccDeviceMatrixOwner::Raw(matrix),
        }
    }

    fn keepalive_tag(&self) -> u8 {
        match self.owner {
            VccDeviceMatrixOwner::Dispatch(_) => 0,
            VccDeviceMatrixOwner::Raw(_) => 1,
        }
    }
}

#[cfg(feature = "cuda")]
pub struct VccPeriodSweepDevice {
    pub matrix: VccOwnedDeviceMatrix,
    pub period_start: u16,
    pub period_end: u16,
    pub period_step: u16,
}

#[cfg(feature = "cuda")]
impl VccPeriodSweepDevice {
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

pub fn validate_vcc_params(cfg: &VccStrategyParams) -> Result<(), String> {
    if cfg.squeeze_period == 0 {
        return Err("VCC squeeze period must be > 0".to_string());
    }
    if cfg.momentum_period == 0 {
        return Err("VCC momentum period must be > 0".to_string());
    }
    if cfg.momentum_bb_period == 0 {
        return Err("VCC momentum_bb_period must be > 0".to_string());
    }
    if cfg.trend_period == 0 {
        return Err("VCC trend period must be > 0".to_string());
    }
    if !cfg.squeeze_bb_mult.is_finite() || cfg.squeeze_bb_mult <= 0.0 {
        return Err("VCC squeeze_bb_mult must be finite and > 0".to_string());
    }
    if !cfg.squeeze_kc_mult.is_finite() || cfg.squeeze_kc_mult <= 0.0 {
        return Err("VCC squeeze_kc_mult must be finite and > 0".to_string());
    }
    if cfg.max_trade_bars == 0 {
        return Err("VCC max_trade_bars must be > 0".to_string());
    }
    Ok(())
}

pub fn resolve_vcc_periods(
    cfg: &VccStrategyParams,
    fast_len: u16,
    slow_len: u16,
) -> VccResolvedPeriods {
    VccResolvedPeriods {
        squeeze: if cfg.map_fast_to_squeeze_period {
            fast_len.max(1)
        } else {
            cfg.squeeze_period.max(1)
        },
        momentum: if cfg.map_fast_to_momentum_period {
            fast_len.max(1)
        } else {
            cfg.momentum_period.max(1)
        },
        trend: if cfg.map_slow_to_trend_period {
            slow_len.max(1)
        } else {
            cfg.trend_period.max(1)
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
            Err("VCC expected CUDA HostF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
#[allow(dead_code)]
fn cuda_series_to_device_f32(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<VccOwnedDeviceMatrix, String> {
    match out.series {
        DynamicIndicatorCudaSeries::DeviceF32(v) => Ok(VccOwnedDeviceMatrix::from_dispatch(v)),
        DynamicIndicatorCudaSeries::HostF32(_) => {
            Err("VCC expected CUDA DeviceF32 output".to_string())
        }
    }
}

#[cfg(feature = "cuda")]
fn backend_prefers_gpu(backend: &Backend) -> bool {
    !matches!(backend, Backend::CpuOnly)
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
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("VCC indicator '{indicator_id}' is not registered for dynamic dispatch")
    })?;

    #[cfg(feature = "cuda")]
    {
        let prefer_gpu = backend_prefers_gpu(_backend)
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

    let _ = descriptor;
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
#[allow(dead_code)]
fn compute_indicator_period_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: &str,
    period_key: &str,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    base_params: &[IndicatorParamKV],
) -> Result<VccPeriodSweepDevice, String> {
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
        format!("VCC indicator '{indicator_id}' is not registered for dynamic dispatch")
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

    Ok(VccPeriodSweepDevice {
        matrix,
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_vcc_ttm_squeeze_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VccStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VccPeriodSweepDevice, String> {
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

    Ok(VccPeriodSweepDevice {
        matrix: VccOwnedDeviceMatrix::from_raw(squeeze),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_vcc_squeeze_momentum_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VccStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VccPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let sweep = SqueezeMomentumBatchRange {
        length_bb: (
            usize::from(cfg.momentum_bb_period.max(1)),
            usize::from(cfg.momentum_bb_period.max(1)),
            0,
        ),
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

    Ok(VccPeriodSweepDevice {
        matrix: VccOwnedDeviceMatrix::from_raw(momentum),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
fn compute_vcc_ttm_trend_sweep_cuda_direct(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VccPeriodSweepDevice, String> {
    let start = period_start.max(1);
    let end = period_end.max(start);
    let step = period_step.max(1);
    let sweep = TtmTrendBatchRange {
        period: (usize::from(start), usize::from(end), usize::from(step)),
    };
    let wrapper = CudaTtmTrend::new(device_id as usize).map_err(|e| e.to_string())?;
    let first_valid = prepared_inputs.hlc_first_valid();
    let matrix = wrapper
        .ttm_trend_batch_dev_from_device_inputs(
            prepared_inputs.high_device().buffer(),
            prepared_inputs.low_device().buffer(),
            prepared_inputs.close_device().buffer(),
            prepared_inputs.len(),
            first_valid,
            &sweep,
        )
        .or_else(|_| {
            wrapper.ttm_trend_batch_dev(prepared_inputs.source(), prepared_inputs.close(), &sweep)
        })
        .map_err(|e| e.to_string())?;

    Ok(VccPeriodSweepDevice {
        matrix: VccOwnedDeviceMatrix::from_raw(matrix),
        period_start: start,
        period_end: end,
        period_step: step,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_vcc_ttm_squeeze_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VccStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VccPeriodSweepDevice, String> {
    compute_vcc_ttm_squeeze_sweep_cuda_direct(
        prepared_inputs,
        cfg,
        period_start,
        period_end,
        period_step,
        device_id,
    )
}

fn compute_vcc_squeeze_state_series(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &VccStrategyParams,
    squeeze_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    let squeeze_momentum_params = vec![
        IndicatorParamKV {
            key: "length_bb".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.momentum_bb_period.max(1))),
        },
        IndicatorParamKV {
            key: "mult_bb".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
        IndicatorParamKV {
            key: "length_kc".to_string(),
            value: IndicatorParamValue::Int(i64::from(squeeze_period.max(1))),
        },
        IndicatorParamKV {
            key: "mult_kc".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_kc_mult),
        },
    ];

    let legacy_ttm_squeeze_params = vec![
        IndicatorParamKV {
            key: "length".to_string(),
            value: IndicatorParamValue::Int(i64::from(squeeze_period.max(1))),
        },
        IndicatorParamKV {
            key: "bb_mult".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
    ];

    match compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "squeeze_momentum",
        "squeeze",
        squeeze_momentum_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    ) {
        Ok(v) => Ok(v),
        Err(_) => compute_indicator_f64(
            candles,
            ma_source,
            backend,
            "ttm_squeeze",
            "squeeze",
            legacy_ttm_squeeze_params.as_slice(),
            #[cfg(feature = "cuda")]
            prepared_cuda_inputs,
        ),
    }
}

#[cfg(feature = "cuda")]
pub fn compute_vcc_squeeze_momentum_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VccStrategyParams,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VccPeriodSweepDevice, String> {
    compute_vcc_squeeze_momentum_sweep_cuda_direct(
        prepared_inputs,
        cfg,
        period_start,
        period_end,
        period_step,
        device_id,
    )
}

#[cfg(feature = "cuda")]
pub fn compute_vcc_fast_dual_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    cfg: &VccStrategyParams,
    squeeze_period_start: u16,
    squeeze_period_end: u16,
    momentum_period_start: u16,
    momentum_period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<(VccPeriodSweepDevice, VccPeriodSweepDevice), String> {
    let squeeze_sweep = compute_vcc_ttm_squeeze_sweep_cuda_device(
        prepared_inputs,
        cfg,
        squeeze_period_start,
        squeeze_period_end,
        period_step,
        device_id,
    )?;
    let momentum_sweep = compute_vcc_squeeze_momentum_sweep_cuda_device(
        prepared_inputs,
        cfg,
        momentum_period_start,
        momentum_period_end,
        period_step,
        device_id,
    )?;
    Ok((squeeze_sweep, momentum_sweep))
}

#[cfg(feature = "cuda")]
pub fn compute_vcc_ttm_trend_sweep_cuda_device(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    period_start: u16,
    period_end: u16,
    period_step: u16,
    device_id: u32,
) -> Result<VccPeriodSweepDevice, String> {
    compute_vcc_ttm_trend_sweep_cuda_direct(
        prepared_inputs,
        period_start,
        period_end,
        period_step,
        device_id,
    )
}

pub fn compute_vcc_fast_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    cfg: &VccStrategyParams,
    squeeze_period: u16,
    momentum_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<VccFastSignals, String> {
    let momentum_params = vec![
        IndicatorParamKV {
            key: "length_bb".to_string(),
            value: IndicatorParamValue::Int(i64::from(cfg.momentum_bb_period.max(1))),
        },
        IndicatorParamKV {
            key: "mult_bb".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
        IndicatorParamKV {
            key: "length_kc".to_string(),
            value: IndicatorParamValue::Int(i64::from(momentum_period.max(1))),
        },
        IndicatorParamKV {
            key: "mult_kc".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_kc_mult),
        },
    ];

    let squeeze_state = compute_vcc_squeeze_state_series(
        candles,
        ma_source,
        backend,
        cfg,
        squeeze_period,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let momentum = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "squeeze_momentum",
        "momentum",
        momentum_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    if squeeze_state.len() != momentum.len() {
        return Err("VCC fast signal series length mismatch".to_string());
    }

    let first_valid = first_finite_idx_local(&squeeze_state)
        .zip(first_finite_idx_local(&momentum))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "VCC fast signal series are all non-finite".to_string())?;

    Ok(VccFastSignals {
        squeeze_state,
        momentum,
        first_valid,
    })
}

pub fn compute_vcc_trend_signals(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    trend_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<VccTrendSignals, String> {
    let trend_params = vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(i64::from(trend_period.max(1))),
    }];

    let trend_state = compute_indicator_f64(
        candles,
        ma_source,
        backend,
        "ttm_trend",
        "value",
        trend_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let first_valid = first_finite_idx_local(&trend_state)
        .ok_or_else(|| "VCC trend signal series are all non-finite".to_string())?;

    Ok(VccTrendSignals {
        trend_state,
        first_valid,
    })
}

pub fn compute_vcc_start_t(
    first_valid_close: usize,
    first_valid_source: usize,
    first_valid_fast: usize,
    first_valid_trend: usize,
    max_period: usize,
    trade_on_next_bar: bool,
) -> usize {
    let mut first_valid = first_valid_close.max(first_valid_source);
    first_valid = first_valid.max(first_valid_fast);
    first_valid = first_valid.max(first_valid_trend);

    let t_valid = first_valid.saturating_add(max_period.max(1).saturating_sub(1));
    if trade_on_next_bar {
        t_valid.saturating_add(1)
    } else {
        t_valid
    }
}

pub fn build_vcc_desired_positions(
    close: &[f64],
    fast: &VccFastSignals,
    trend: &VccTrendSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &VccStrategyParams,
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
        let trend_now = trend.trend_state.get(sig_t).copied().unwrap_or(f64::NAN);

        let squeeze_on_now = squeeze_now.is_finite() && squeeze_now > 0.5;
        let squeeze_on_prev = squeeze_prev.is_finite() && squeeze_prev > 0.5;
        let squeeze_release = squeeze_on_prev && !squeeze_on_now;

        if squeeze_release {
            release_confirm_left = usize::from(cfg.confirm_bars);
        } else if release_confirm_left > 0 {
            release_confirm_left = release_confirm_left.saturating_sub(1);
        }
        let release_confirmed = squeeze_release || release_confirm_left == 0;

        if pos == 0 {
            bars_in_trade = 0;
            if release_confirmed {
                let bullish_trend = trend_now.is_finite() && trend_now > 0.5;
                let bearish_trend = trend_now.is_finite() && trend_now <= 0.5;
                let long_mom_ok = if cfg.require_momentum_accel {
                    mom_now.is_finite() && mom_now > 0.0 && mom_delta.is_finite() && mom_delta > 0.0
                } else {
                    mom_now.is_finite() && mom_now > 0.0
                };
                let short_mom_ok = if cfg.require_momentum_accel {
                    mom_now.is_finite() && mom_now < 0.0 && mom_delta.is_finite() && mom_delta < 0.0
                } else {
                    mom_now.is_finite() && mom_now < 0.0
                };

                if bullish_trend && long_mom_ok {
                    pos = 1;
                    bars_in_trade = 1;
                } else if !strategy.long_only && bearish_trend && short_mom_ok {
                    pos = -1;
                    bars_in_trade = 1;
                }
            }
        } else {
            let bullish_trend = trend_now.is_finite() && trend_now > 0.5;
            let bearish_trend = trend_now.is_finite() && trend_now <= 0.5;
            let mut exit_now = false;

            if pos > 0 {
                if !mom_now.is_finite() || mom_now <= 0.0 || !bullish_trend {
                    exit_now = true;
                }
            } else if !mom_now.is_finite() || mom_now >= 0.0 || !bearish_trend {
                exit_now = true;
            }

            if bars_in_trade >= usize::from(cfg.max_trade_bars.max(1)) {
                exit_now = true;
            }

            if exit_now {
                pos = 0;
                bars_in_trade = 0;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        }

        if strategy.long_only && pos < 0 {
            desired[t] = 0;
        } else {
            desired[t] = pos;
        }
    }

    desired
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_vcc_periods_maps_fast_and_slow_when_enabled() {
        let cfg = VccStrategyParams::default();
        let p = resolve_vcc_periods(&cfg, 8, 21);
        assert_eq!(p.squeeze, 8);
        assert_eq!(p.momentum, 8);
        assert_eq!(p.trend, 21);
    }

    #[test]
    fn validate_vcc_params_rejects_invalid_values() {
        let mut cfg = VccStrategyParams::default();
        cfg.max_trade_bars = 0;
        let err = validate_vcc_params(&cfg).expect_err("should reject max_trade_bars=0");
        assert!(err.contains("max_trade_bars"));
    }
}
