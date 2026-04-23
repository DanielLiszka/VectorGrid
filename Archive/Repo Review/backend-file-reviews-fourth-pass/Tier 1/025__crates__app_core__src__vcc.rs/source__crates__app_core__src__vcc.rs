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
#[cfg(all(test, feature = "cuda"))]
use std::cell::Cell;
#[cfg(feature = "cuda")]
use std::cell::RefCell;
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

#[cfg(feature = "cuda")]
thread_local! {
    static VCC_ROUTE_DIAGNOSTICS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    #[cfg(test)]
    static FORCE_VCC_CUDA_CPU_FALLBACK: Cell<bool> = const { Cell::new(false) };
}

#[cfg(feature = "cuda")]
fn record_vcc_route_diagnostic(message: String) {
    VCC_ROUTE_DIAGNOSTICS.with(|cell| {
        let mut diagnostics = cell.borrow_mut();
        if diagnostics.iter().any(|entry| entry == &message) {
            return;
        }
        diagnostics.push(message);
    });
}

#[cfg(feature = "cuda")]
pub(crate) fn clear_vcc_route_diagnostics() {
    VCC_ROUTE_DIAGNOSTICS.with(|cell| cell.borrow_mut().clear());
}

#[cfg(feature = "cuda")]
pub(crate) fn take_vcc_route_diagnostics() -> Option<Vec<String>> {
    VCC_ROUTE_DIAGNOSTICS.with(|cell| {
        let mut diagnostics = cell.borrow_mut();
        if diagnostics.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut *diagnostics))
        }
    })
}

#[cfg(not(feature = "cuda"))]
pub(crate) fn clear_vcc_route_diagnostics() {}

#[cfg(not(feature = "cuda"))]
pub(crate) fn take_vcc_route_diagnostics() -> Option<Vec<String>> {
    None
}

#[cfg(all(test, feature = "cuda"))]
pub(crate) fn set_force_vcc_cuda_cpu_fallback_for_test(enabled: bool) {
    FORCE_VCC_CUDA_CPU_FALLBACK.with(|cell| cell.set(enabled));
}

#[cfg(all(test, feature = "cuda"))]
fn force_vcc_cuda_cpu_fallback_for_test() -> bool {
    FORCE_VCC_CUDA_CPU_FALLBACK.with(|cell| cell.get())
}

#[cfg(all(feature = "cuda", not(test)))]
fn force_vcc_cuda_cpu_fallback_for_test() -> bool {
    false
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VccMomentumRoundtripParity {
    pub sign_mismatch_count: usize,
    pub delta_sign_mismatch_count: usize,
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

fn sign_i8(v: f64) -> i8 {
    if !v.is_finite() {
        0
    } else if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

pub fn vcc_momentum_roundtrip_parity(momentum: &[f64]) -> VccMomentumRoundtripParity {
    let roundtrip: Vec<f64> = momentum.iter().map(|v| (*v as f32) as f64).collect();
    let mut stats = VccMomentumRoundtripParity::default();
    for (src, rt) in momentum.iter().zip(roundtrip.iter()) {
        if sign_i8(*src) != sign_i8(*rt) {
            stats.sign_mismatch_count += 1;
        }
    }
    for idx in 1..momentum.len() {
        let src_delta = momentum[idx] - momentum[idx - 1];
        let rt_delta = roundtrip[idx] - roundtrip[idx - 1];
        if sign_i8(src_delta) != sign_i8(rt_delta) {
            stats.delta_sign_mismatch_count += 1;
        }
    }
    stats
}

pub fn vcc_momentum_roundtrip_is_fp32_stable(momentum: &[f64]) -> bool {
    let parity = vcc_momentum_roundtrip_parity(momentum);
    parity.sign_mismatch_count == 0 && parity.delta_sign_mismatch_count == 0
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

struct IndicatorSeriesF64 {
    values: Vec<f64>,
    warmup: Option<usize>,
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

#[cfg(all(test, feature = "cuda", feature = "cuda-backtest-kernel"))]
fn compute_indicator_f64(
    candles: &Candles,
    ma_source: &str,
    _backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    Ok(compute_indicator_f64_with_warmup(
        candles,
        ma_source,
        _backend,
        indicator_id,
        output_id,
        params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?
    .values)
}

fn compute_indicator_f64_with_warmup(
    candles: &Candles,
    ma_source: &str,
    _backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<IndicatorSeriesF64, String> {
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
                if force_vcc_cuda_cpu_fallback_for_test() {
                    record_vcc_route_diagnostic(format!(
                        "VCC indicator {indicator_id}/{output_id} CUDA helper fell back to CPU: forced test fallback"
                    ));
                } else {
                    let out = compute_dynamic_indicator_cuda_with_inputs(
                        prepared,
                        indicator_id,
                        Some(output_id),
                        params,
                        Kernel::Auto,
                        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                    );
                    match out {
                        Ok(values) => {
                            let warmup = values.warmup;
                            return Ok(IndicatorSeriesF64 {
                                values: cuda_series_to_f64(values)?,
                                warmup,
                            });
                        }
                        Err(err) => {
                            record_vcc_route_diagnostic(format!(
                                "VCC indicator {indicator_id}/{output_id} CUDA helper fell back to CPU: {err}"
                            ));
                        }
                    }
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
    let warmup = cpu.warmup;
    Ok(IndicatorSeriesF64 {
        values: series_to_f64(cpu),
        warmup,
    })
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
    let squeeze_sweep = compute_vcc_ttm_squeeze_sweep_cuda_direct(
        prepared_inputs,
        cfg,
        squeeze_period_start,
        squeeze_period_end,
        period_step,
        device_id,
    )?;
    let momentum_sweep = compute_vcc_squeeze_momentum_sweep_cuda_direct(
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
    let squeeze_params = vec![
        IndicatorParamKV {
            key: "length".to_string(),
            value: IndicatorParamValue::Int(i64::from(squeeze_period.max(1))),
        },
        IndicatorParamKV {
            key: "bb_mult".to_string(),
            value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
        },
    ];
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

    let squeeze_state = compute_indicator_f64_with_warmup(
        candles,
        ma_source,
        backend,
        "ttm_squeeze",
        "squeeze",
        squeeze_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let momentum = compute_indicator_f64_with_warmup(
        candles,
        ma_source,
        backend,
        "squeeze_momentum",
        "momentum",
        momentum_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    if squeeze_state.values.len() != momentum.values.len() {
        return Err("VCC fast signal series length mismatch".to_string());
    }

    let squeeze_first_valid = squeeze_state
        .warmup
        .or_else(|| first_finite_idx_local(&squeeze_state.values))
        .ok_or_else(|| "VCC squeeze-state series is all non-finite".to_string())?;
    let momentum_first_valid = momentum
        .warmup
        .or_else(|| first_finite_idx_local(&momentum.values))
        .ok_or_else(|| "VCC momentum series is all non-finite".to_string())?;
    let first_valid = squeeze_first_valid.max(momentum_first_valid);

    Ok(VccFastSignals {
        squeeze_state: squeeze_state.values,
        momentum: momentum.values,
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

    let trend_state = compute_indicator_f64_with_warmup(
        candles,
        ma_source,
        backend,
        "ttm_trend",
        "value",
        trend_params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    let first_valid = trend_state
        .warmup
        .or_else(|| first_finite_idx_local(&trend_state.values))
        .ok_or_else(|| "VCC trend signal series are all non-finite".to_string())?;

    Ok(VccTrendSignals {
        trend_state: trend_state.values,
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

pub fn compute_vcc_fast_first_valid(
    cfg: &VccStrategyParams,
    squeeze_period: u16,
    momentum_period: u16,
) -> usize {
    usize::from(
        cfg.momentum_bb_period
            .max(1)
            .max(squeeze_period.max(1))
            .max(momentum_period.max(1))
            .saturating_sub(1),
    )
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
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    use crate::asr::eval_asr_metrics_from_desired;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    use crate::vcc_gpu_kernel::{
        eval_vcc_metrics_gpu_kernel_device_rows, VccGpuComboBatch, VccGpuDeviceSignalMatrices,
    };
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    use cust::memory::CopyDestination;
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

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn make_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

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

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn make_monotonic_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.5);
            vol.push(1_000.0);
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn vcc_trend_signals_record_route_diagnostics_on_cuda_cpu_fallback() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        struct ResetVccCpuFallback;

        impl Drop for ResetVccCpuFallback {
            fn drop(&mut self) {
                set_force_vcc_cuda_cpu_fallback_for_test(false);
                clear_vcc_route_diagnostics();
            }
        }

        clear_vcc_route_diagnostics();
        set_force_vcc_cuda_cpu_fallback_for_test(true);
        let _reset = ResetVccCpuFallback;

        let candles = make_trending_candles(512);
        let prepared_cuda_inputs =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0)
                .expect("prepared cuda inputs");

        let signals = compute_vcc_trend_signals(
            &candles,
            "close",
            &Backend::GpuKernel { device_id: 0 },
            21,
            Some(&prepared_cuda_inputs),
        )
        .expect("vcc trend signals with forced cpu fallback");
        assert_eq!(signals.trend_state.len(), candles.close.len());

        let route_diagnostics = take_vcc_route_diagnostics().unwrap_or_default();
        assert!(
            route_diagnostics.iter().any(|entry| {
                entry.contains("VCC indicator ttm_trend/value CUDA helper fell back to CPU")
                    && entry.contains("forced test fallback")
            }),
            "expected VCC helper fallback diagnostic, got {:?}",
            route_diagnostics
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_signal_diagnostic_cpu_vs_cuda_host_series() {
        fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
            a.iter()
                .zip(b.iter())
                .filter_map(|(x, y)| {
                    if x.is_finite() && y.is_finite() {
                        Some((x - y).abs())
                    } else {
                        None
                    }
                })
                .fold(0.0, f64::max)
        }

        let candles = make_trending_candles(4096);
        let cfg = VccStrategyParams::default();
        let periods = resolve_vcc_periods(&cfg, 8, 24);
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();

        let fast_cpu = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            &cfg,
            periods.squeeze,
            periods.momentum,
            Some(&prepared),
        )
        .unwrap();
        let fast_gpu = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::GpuKernel { device_id: 0 },
            &cfg,
            periods.squeeze,
            periods.momentum,
            Some(&prepared),
        )
        .unwrap();
        let trend_cpu = compute_vcc_trend_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            periods.trend,
            Some(&prepared),
        )
        .unwrap();
        let trend_gpu = compute_vcc_trend_signals(
            &candles,
            "close",
            &Backend::GpuKernel { device_id: 0 },
            periods.trend,
            Some(&prepared),
        )
        .unwrap();

        let sq_sm_cpu = compute_indicator_f64(
            &candles,
            "close",
            &Backend::CpuOnly,
            "squeeze_momentum",
            "squeeze",
            &[
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
                    value: IndicatorParamValue::Int(i64::from(periods.squeeze.max(1))),
                },
                IndicatorParamKV {
                    key: "mult_kc".to_string(),
                    value: IndicatorParamValue::Float(cfg.squeeze_kc_mult),
                },
            ],
            Some(&prepared),
        )
        .unwrap();
        let sq_ttm_cpu = compute_indicator_f64(
            &candles,
            "close",
            &Backend::CpuOnly,
            "ttm_squeeze",
            "squeeze",
            &[
                IndicatorParamKV {
                    key: "length".to_string(),
                    value: IndicatorParamValue::Int(i64::from(periods.squeeze.max(1))),
                },
                IndicatorParamKV {
                    key: "bb_mult".to_string(),
                    value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
                },
            ],
            Some(&prepared),
        )
        .unwrap();
        let sq_sm_gpu = compute_indicator_f64(
            &candles,
            "close",
            &Backend::GpuKernel { device_id: 0 },
            "squeeze_momentum",
            "squeeze",
            &[
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
                    value: IndicatorParamValue::Int(i64::from(periods.squeeze.max(1))),
                },
                IndicatorParamKV {
                    key: "mult_kc".to_string(),
                    value: IndicatorParamValue::Float(cfg.squeeze_kc_mult),
                },
            ],
            Some(&prepared),
        )
        .unwrap();
        let sq_ttm_gpu = compute_indicator_f64(
            &candles,
            "close",
            &Backend::GpuKernel { device_id: 0 },
            "ttm_squeeze",
            "squeeze",
            &[
                IndicatorParamKV {
                    key: "length".to_string(),
                    value: IndicatorParamValue::Int(i64::from(periods.squeeze.max(1))),
                },
                IndicatorParamKV {
                    key: "bb_mult".to_string(),
                    value: IndicatorParamValue::Float(cfg.squeeze_bb_mult),
                },
            ],
            Some(&prepared),
        )
        .unwrap();

        eprintln!(
            "vcc diag: fast_first_valid cpu={} gpu={} trend_first_valid cpu={} gpu={}",
            fast_cpu.first_valid,
            fast_gpu.first_valid,
            trend_cpu.first_valid,
            trend_gpu.first_valid
        );
        eprintln!(
            "vcc diag: max abs diff squeeze={} momentum={} trend={}",
            max_abs_diff(&fast_cpu.squeeze_state, &fast_gpu.squeeze_state),
            max_abs_diff(&fast_cpu.momentum, &fast_gpu.momentum),
            max_abs_diff(&trend_cpu.trend_state, &trend_gpu.trend_state)
        );
        eprintln!(
            "vcc diag: cpu squeeze impl gap sm_vs_ttm={}",
            max_abs_diff(&sq_sm_cpu, &sq_ttm_cpu)
        );
        eprintln!(
            "vcc diag: squeeze impl gaps cpu_sm_vs_gpu_sm={} cpu_ttm_vs_gpu_ttm={} cpu_sm_vs_gpu_ttm={} cpu_ttm_vs_gpu_sm={}",
            max_abs_diff(&sq_sm_cpu, &sq_sm_gpu),
            max_abs_diff(&sq_ttm_cpu, &sq_ttm_gpu),
            max_abs_diff(&sq_sm_cpu, &sq_ttm_gpu),
            max_abs_diff(&sq_ttm_cpu, &sq_sm_gpu)
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_signal_diagnostic_device_sweeps_vs_cpu_series() {
        fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
            a.iter()
                .zip(b.iter())
                .filter_map(|(x, y)| {
                    if x.is_finite() && y.is_finite() {
                        Some((x - y).abs())
                    } else {
                        None
                    }
                })
                .fold(0.0, f64::max)
        }

        fn sweep_row_to_host(sweep: &VccPeriodSweepDevice) -> Vec<f64> {
            let mut out = vec![0.0f32; sweep.matrix.rows * sweep.matrix.cols];
            match &sweep.matrix.owner {
                VccDeviceMatrixOwner::Raw(raw) => raw.buf.copy_to(&mut out).unwrap(),
                VccDeviceMatrixOwner::Dispatch(_) => panic!("unexpected dispatch owner"),
            }
            out.into_iter().map(|x| x as f64).collect()
        }

        let candles = make_trending_candles(4096);
        let cfg = VccStrategyParams::default();
        let periods = resolve_vcc_periods(&cfg, 8, 24);
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();

        let fast_cpu = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            &cfg,
            periods.squeeze,
            periods.momentum,
            Some(&prepared),
        )
        .unwrap();
        let trend_cpu = compute_vcc_trend_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            periods.trend,
            Some(&prepared),
        )
        .unwrap();

        let squeeze_sweep = compute_vcc_ttm_squeeze_sweep_cuda_device(
            &prepared,
            &cfg,
            periods.squeeze,
            periods.squeeze,
            1,
            0,
        )
        .unwrap();
        let momentum_sweep = compute_vcc_squeeze_momentum_sweep_cuda_device(
            &prepared,
            &cfg,
            periods.momentum,
            periods.momentum,
            1,
            0,
        )
        .unwrap();
        let trend_sweep =
            compute_vcc_ttm_trend_sweep_cuda_device(&prepared, periods.trend, periods.trend, 1, 0)
                .unwrap();

        let squeeze_dev = sweep_row_to_host(&squeeze_sweep);
        let momentum_dev = sweep_row_to_host(&momentum_sweep);
        let trend_dev = sweep_row_to_host(&trend_sweep);

        eprintln!(
            "vcc device diag: diff squeeze={} momentum={} trend={}",
            max_abs_diff(&fast_cpu.squeeze_state, &squeeze_dev),
            max_abs_diff(&fast_cpu.momentum, &momentum_dev),
            max_abs_diff(&trend_cpu.trend_state, &trend_dev)
        );
        let sign_mismatch = fast_cpu
            .momentum
            .iter()
            .zip(momentum_dev.iter())
            .filter(|(a, b)| a.is_finite() && b.is_finite() && a.signum() != b.signum())
            .count();
        let near_zero_sign_mismatch = fast_cpu
            .momentum
            .iter()
            .zip(momentum_dev.iter())
            .filter(|(a, b)| {
                a.is_finite()
                    && b.is_finite()
                    && a.signum() != b.signum()
                    && a.abs().max(b.abs()) < 0.01
            })
            .count();
        eprintln!(
            "vcc device diag: momentum sign mismatches={} near_zero(<0.01)={}",
            sign_mismatch, near_zero_sign_mismatch
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_signal_diagnostic_multirow_sweeps_vs_cpu_series() {
        fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
            a.iter()
                .zip(b.iter())
                .filter_map(|(x, y)| {
                    if x.is_finite() && y.is_finite() {
                        Some((x - y).abs())
                    } else {
                        None
                    }
                })
                .fold(0.0, f64::max)
        }

        fn sweep_row_to_host(sweep: &VccPeriodSweepDevice, period: u16) -> Vec<f64> {
            let mut out = vec![0.0f32; sweep.matrix.rows * sweep.matrix.cols];
            match &sweep.matrix.owner {
                VccDeviceMatrixOwner::Raw(raw) => raw.buf.copy_to(&mut out).unwrap(),
                VccDeviceMatrixOwner::Dispatch(_) => panic!("unexpected dispatch owner"),
            }
            let row = sweep.row_index_for_period(period).unwrap();
            let cols = sweep.matrix.cols;
            out[(row * cols)..((row + 1) * cols)]
                .iter()
                .map(|x| *x as f64)
                .collect()
        }

        let candles = make_trending_candles(4096);
        let cfg = VccStrategyParams::default();
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();

        let fast_cpu = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            &cfg,
            8,
            8,
            Some(&prepared),
        )
        .unwrap();
        let trend_cpu =
            compute_vcc_trend_signals(&candles, "close", &Backend::CpuOnly, 24, Some(&prepared))
                .unwrap();

        let (sq_sweep, mo_sweep) =
            compute_vcc_fast_dual_sweep_cuda_device(&prepared, &cfg, 8, 12, 8, 12, 1, 0).unwrap();
        let tr_sweep = compute_vcc_ttm_trend_sweep_cuda_device(&prepared, 24, 36, 1, 0).unwrap();

        let squeeze_dev = sweep_row_to_host(&sq_sweep, 8);
        let momentum_dev = sweep_row_to_host(&mo_sweep, 8);
        let trend_dev = sweep_row_to_host(&tr_sweep, 24);

        eprintln!(
            "vcc multi-row diag: diff squeeze={} momentum={} trend={}",
            max_abs_diff(&fast_cpu.squeeze_state, &squeeze_dev),
            max_abs_diff(&fast_cpu.momentum, &momentum_dev),
            max_abs_diff(&trend_cpu.trend_state, &trend_dev)
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_signal_diagnostic_short_combo_sweeps_vs_cpu_series() {
        fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
            a.iter()
                .zip(b.iter())
                .filter_map(|(x, y)| {
                    if x.is_finite() && y.is_finite() {
                        Some((x - y).abs())
                    } else {
                        None
                    }
                })
                .fold(0.0, f64::max)
        }

        fn sweep_row_to_host(sweep: &VccPeriodSweepDevice, period: u16) -> Vec<f64> {
            let mut out = vec![0.0f32; sweep.matrix.rows * sweep.matrix.cols];
            match &sweep.matrix.owner {
                VccDeviceMatrixOwner::Raw(raw) => raw.buf.copy_to(&mut out).unwrap(),
                VccDeviceMatrixOwner::Dispatch(_) => panic!("unexpected dispatch owner"),
            }
            let row = sweep.row_index_for_period(period).unwrap();
            let cols = sweep.matrix.cols;
            out[(row * cols)..((row + 1) * cols)]
                .iter()
                .map(|x| *x as f64)
                .collect()
        }

        let candles = make_monotonic_trending_candles(3000);
        let cfg = VccStrategyParams::default();
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();

        let fast_cpu = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            &cfg,
            5,
            5,
            Some(&prepared),
        )
        .unwrap();
        let trend_cpu =
            compute_vcc_trend_signals(&candles, "close", &Backend::CpuOnly, 12, Some(&prepared))
                .unwrap();

        let (sq_sweep, mo_sweep) =
            compute_vcc_fast_dual_sweep_cuda_device(&prepared, &cfg, 5, 7, 5, 7, 1, 0).unwrap();
        let tr_sweep = compute_vcc_ttm_trend_sweep_cuda_device(&prepared, 12, 16, 1, 0).unwrap();

        let squeeze_dev = sweep_row_to_host(&sq_sweep, 5);
        let momentum_dev = sweep_row_to_host(&mo_sweep, 5);
        let trend_dev = sweep_row_to_host(&tr_sweep, 12);

        let sign_mismatch = fast_cpu
            .momentum
            .iter()
            .zip(momentum_dev.iter())
            .filter(|(a, b)| a.is_finite() && b.is_finite() && a.signum() != b.signum())
            .count();
        let delta_sign_mismatch = fast_cpu
            .momentum
            .windows(2)
            .zip(momentum_dev.windows(2))
            .filter(|(cpu, gpu)| {
                let cpu_delta = cpu[1] - cpu[0];
                let gpu_delta = gpu[1] - gpu[0];
                cpu_delta.is_finite()
                    && gpu_delta.is_finite()
                    && cpu_delta.signum() != gpu_delta.signum()
            })
            .count();

        eprintln!(
            "vcc short diag: diff squeeze={} momentum={} trend={} sign_mismatch={} delta_sign_mismatch={}",
            max_abs_diff(&fast_cpu.squeeze_state, &squeeze_dev),
            max_abs_diff(&fast_cpu.momentum, &momentum_dev),
            max_abs_diff(&trend_cpu.trend_state, &trend_dev),
            sign_mismatch,
            delta_sign_mismatch
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_metrics_diagnostic_short_combo_cpu_vs_gpu_kernel() {
        fn sweep_row_to_host(sweep: &VccPeriodSweepDevice, period: u16) -> Vec<f64> {
            let mut out = vec![0.0f32; sweep.matrix.rows * sweep.matrix.cols];
            match &sweep.matrix.owner {
                VccDeviceMatrixOwner::Raw(raw) => raw.buf.copy_to(&mut out).unwrap(),
                VccDeviceMatrixOwner::Dispatch(_) => panic!("unexpected dispatch owner"),
            }
            let row = sweep.row_index_for_period(period).unwrap();
            let cols = sweep.matrix.cols;
            out[(row * cols)..((row + 1) * cols)]
                .iter()
                .map(|x| *x as f64)
                .collect()
        }

        let candles = make_monotonic_trending_candles(3000);
        let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
        let lr = crate::double_ma::compute_log_returns_f64(close);
        let strategy = StrategyConfig::default();
        let cfg = VccStrategyParams::default();
        let periods = resolve_vcc_periods(&cfg, 5, 12);
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();

        let fast_cpu = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            &cfg,
            periods.squeeze,
            periods.momentum,
            Some(&prepared),
        )
        .unwrap();
        let trend_cpu = compute_vcc_trend_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            periods.trend,
            Some(&prepared),
        )
        .unwrap();
        let start_t = compute_vcc_start_t(
            0,
            0,
            fast_cpu.first_valid,
            trend_cpu.first_valid,
            usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1),
            strategy.trade_on_next_bar,
        );
        let desired_cpu =
            build_vcc_desired_positions(close, &fast_cpu, &trend_cpu, start_t, &strategy, &cfg);
        let metrics_cpu =
            eval_asr_metrics_from_desired(&candles, &lr, &desired_cpu, start_t, &strategy);

        let (sq_sweep, mo_sweep) =
            compute_vcc_fast_dual_sweep_cuda_device(&prepared, &cfg, 5, 7, 5, 7, 1, 0).unwrap();
        let tr_sweep = compute_vcc_ttm_trend_sweep_cuda_device(&prepared, 12, 16, 1, 0).unwrap();
        let squeeze_dev = sweep_row_to_host(&sq_sweep, 5);
        let momentum_dev = sweep_row_to_host(&mo_sweep, 5);
        let trend_dev = sweep_row_to_host(&tr_sweep, 12);
        let fast_dev = VccFastSignals {
            squeeze_state: squeeze_dev,
            momentum: momentum_dev,
            first_valid: fast_cpu.first_valid,
        };
        let trend_dev = VccTrendSignals {
            trend_state: trend_dev,
            first_valid: trend_cpu.first_valid,
        };
        let desired_dev =
            build_vcc_desired_positions(close, &fast_dev, &trend_dev, start_t, &strategy, &cfg);
        let metrics_dev_cpu =
            eval_asr_metrics_from_desired(&candles, &lr, &desired_dev, start_t, &strategy);

        let fast_squeeze_row = sq_sweep.row_ptr_for_period(5).unwrap();
        let fast_momentum_row = mo_sweep.row_ptr_for_period(5).unwrap();
        let trend_row = tr_sweep.row_ptr_for_period(12).unwrap();
        let signals = VccGpuDeviceSignalMatrices {
            close_f64: close,
            close_f32: prepared.close(),
            close_device_ptr: prepared.close_device().device_ptr(),
            cols: close.len(),
            fast_rows: 1,
            trend_rows: 1,
            fast_squeeze_rows: &[fast_squeeze_row],
            fast_squeeze_host_row_major: None,
            fast_momentum_rows: &[fast_momentum_row],
            fast_momentum_host_row_major: None,
            trend_rows_ptr: &[trend_row],
            trend_host_row_major: None,
        };
        let combo_start = [start_t as i32];
        let combo_end = [close.len() as i32];
        let combos = VccGpuComboBatch {
            fast_row_index: &[0],
            trend_row_index: &[0],
            start_t: &combo_start,
            metric_start_t: &combo_start,
            end_t: &combo_end,
        };
        let metrics_gpu =
            eval_vcc_metrics_gpu_kernel_device_rows(0, &signals, &combos, &strategy, &cfg).unwrap();

        eprintln!(
            "vcc metric diag: cpu pnl={} dev_cpu pnl={} gpu pnl={} cpu trades={} dev_cpu trades={} gpu trades={}",
            metrics_cpu.pnl,
            metrics_dev_cpu.pnl,
            metrics_gpu[0].pnl,
            metrics_cpu.trades,
            metrics_dev_cpu.trades,
            metrics_gpu[0].trades
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_metrics_diagnostic_validation_batch_cpu_vs_gpu_kernel() {
        let candles = make_trending_candles(4096);
        let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
        let lr = crate::double_ma::compute_log_returns_f64(close);
        let strategy = StrategyConfig::default();
        let cfg = VccStrategyParams::default();
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();
        let combos_spec = [(8u16, 24u16), (12u16, 36u16)];

        let mut cpu_metrics = Vec::new();
        let mut fast_rows = Vec::new();
        let mut fast_mom_rows = Vec::new();
        let mut trend_rows = Vec::new();
        let mut combo_fast_idx = Vec::new();
        let mut combo_trend_idx = Vec::new();
        let mut combo_start = Vec::new();

        let (sq_sweep, mo_sweep) =
            compute_vcc_fast_dual_sweep_cuda_device(&prepared, &cfg, 8, 12, 8, 12, 1, 0).unwrap();
        let tr_sweep = compute_vcc_ttm_trend_sweep_cuda_device(&prepared, 24, 36, 1, 0).unwrap();

        for (i, (fast_len, slow_len)) in combos_spec.iter().copied().enumerate() {
            let periods = resolve_vcc_periods(&cfg, fast_len, slow_len);
            let fast_cpu = compute_vcc_fast_signals(
                &candles,
                "close",
                &Backend::CpuOnly,
                &cfg,
                periods.squeeze,
                periods.momentum,
                Some(&prepared),
            )
            .unwrap();
            let trend_cpu = compute_vcc_trend_signals(
                &candles,
                "close",
                &Backend::CpuOnly,
                periods.trend,
                Some(&prepared),
            )
            .unwrap();
            let start_t = compute_vcc_start_t(
                0,
                0,
                fast_cpu.first_valid,
                trend_cpu.first_valid,
                usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1),
                strategy.trade_on_next_bar,
            );
            let desired =
                build_vcc_desired_positions(close, &fast_cpu, &trend_cpu, start_t, &strategy, &cfg);
            cpu_metrics.push(eval_asr_metrics_from_desired(
                &candles, &lr, &desired, start_t, &strategy,
            ));

            fast_rows.push(sq_sweep.row_ptr_for_period(periods.squeeze).unwrap());
            fast_mom_rows.push(mo_sweep.row_ptr_for_period(periods.momentum).unwrap());
            trend_rows.push(tr_sweep.row_ptr_for_period(periods.trend).unwrap());
            combo_fast_idx.push(i as i32);
            combo_trend_idx.push(i as i32);
            combo_start.push(start_t as i32);
        }

        let signals = VccGpuDeviceSignalMatrices {
            close_f64: close,
            close_f32: prepared.close(),
            close_device_ptr: prepared.close_device().device_ptr(),
            cols: close.len(),
            fast_rows: fast_rows.len(),
            trend_rows: trend_rows.len(),
            fast_squeeze_rows: &fast_rows,
            fast_squeeze_host_row_major: None,
            fast_momentum_rows: &fast_mom_rows,
            fast_momentum_host_row_major: None,
            trend_rows_ptr: &trend_rows,
            trend_host_row_major: None,
        };
        let combo_end = vec![close.len() as i32; combo_start.len()];
        let combos = VccGpuComboBatch {
            fast_row_index: &combo_fast_idx,
            trend_row_index: &combo_trend_idx,
            start_t: &combo_start,
            metric_start_t: &combo_start,
            end_t: &combo_end,
        };
        let gpu_metrics =
            eval_vcc_metrics_gpu_kernel_device_rows(0, &signals, &combos, &strategy, &cfg).unwrap();

        for (idx, (cpu, gpu)) in cpu_metrics.iter().zip(gpu_metrics.iter()).enumerate() {
            eprintln!(
                "vcc batch diag idx={} cpu pnl={} gpu pnl={} cpu trades={} gpu trades={}",
                idx, cpu.pnl, gpu.pnl, cpu.trades, gpu.trades
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_metrics_diagnostic_optimizer_shape_single_combo() {
        let candles = make_monotonic_trending_candles(3000);
        let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
        let lr = crate::double_ma::compute_log_returns_f64(close);
        let strategy = StrategyConfig::default();
        let cfg = VccStrategyParams::default();
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();

        let fast_cpu = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            &cfg,
            5,
            5,
            Some(&prepared),
        )
        .unwrap();
        let trend_cpu =
            compute_vcc_trend_signals(&candles, "close", &Backend::CpuOnly, 12, Some(&prepared))
                .unwrap();
        let start_t = compute_vcc_start_t(
            0,
            0,
            fast_cpu.first_valid,
            trend_cpu.first_valid,
            12,
            strategy.trade_on_next_bar,
        );
        let desired =
            build_vcc_desired_positions(close, &fast_cpu, &trend_cpu, start_t, &strategy, &cfg);
        let metrics_cpu =
            eval_asr_metrics_from_desired(&candles, &lr, &desired, start_t, &strategy);

        let (sq_sweep, mo_sweep) =
            compute_vcc_fast_dual_sweep_cuda_device(&prepared, &cfg, 5, 7, 5, 7, 1, 0).unwrap();
        let tr_sweep = compute_vcc_ttm_trend_sweep_cuda_device(&prepared, 12, 16, 1, 0).unwrap();

        let fast_squeeze_rows = vec![
            sq_sweep.row_ptr_for_period(5).unwrap(),
            sq_sweep.row_ptr_for_period(6).unwrap(),
            sq_sweep.row_ptr_for_period(7).unwrap(),
        ];
        let fast_momentum_rows = vec![
            mo_sweep.row_ptr_for_period(5).unwrap(),
            mo_sweep.row_ptr_for_period(6).unwrap(),
            mo_sweep.row_ptr_for_period(7).unwrap(),
        ];
        let trend_rows_ptr = vec![
            tr_sweep.row_ptr_for_period(12).unwrap(),
            tr_sweep.row_ptr_for_period(13).unwrap(),
            tr_sweep.row_ptr_for_period(14).unwrap(),
            tr_sweep.row_ptr_for_period(15).unwrap(),
            tr_sweep.row_ptr_for_period(16).unwrap(),
        ];

        let signals = VccGpuDeviceSignalMatrices {
            close_f64: close,
            close_f32: prepared.close(),
            close_device_ptr: prepared.close_device().device_ptr(),
            cols: close.len(),
            fast_rows: fast_squeeze_rows.len(),
            trend_rows: trend_rows_ptr.len(),
            fast_squeeze_rows: &fast_squeeze_rows,
            fast_squeeze_host_row_major: None,
            fast_momentum_rows: &fast_momentum_rows,
            fast_momentum_host_row_major: None,
            trend_rows_ptr: &trend_rows_ptr,
            trend_host_row_major: None,
        };
        let combo_start = [start_t as i32];
        let combo_end = [close.len() as i32];
        let combos = VccGpuComboBatch {
            fast_row_index: &[0],
            trend_row_index: &[0],
            start_t: &combo_start,
            metric_start_t: &combo_start,
            end_t: &combo_end,
        };
        let metrics_gpu =
            eval_vcc_metrics_gpu_kernel_device_rows(0, &signals, &combos, &strategy, &cfg).unwrap();

        eprintln!(
            "vcc optimizer-shape diag: cpu pnl={} gpu pnl={} cpu trades={} gpu trades={}",
            metrics_cpu.pnl, metrics_gpu[0].pnl, metrics_cpu.trades, metrics_gpu[0].trades
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_metrics_diagnostic_optimizer_shape_single_combo_fast6() {
        fn sweep_row_to_host(sweep: &VccPeriodSweepDevice, period: u16) -> Vec<f64> {
            let mut out = vec![0.0f32; sweep.matrix.rows * sweep.matrix.cols];
            match &sweep.matrix.owner {
                VccDeviceMatrixOwner::Raw(raw) => raw.buf.copy_to(&mut out).unwrap(),
                VccDeviceMatrixOwner::Dispatch(_) => panic!("unexpected dispatch owner"),
            }
            let row = sweep.row_index_for_period(period).unwrap();
            let cols = sweep.matrix.cols;
            out[(row * cols)..((row + 1) * cols)]
                .iter()
                .map(|x| *x as f64)
                .collect()
        }
        fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
            a.iter()
                .zip(b.iter())
                .filter_map(|(x, y)| (x.is_finite() && y.is_finite()).then_some((x - y).abs()))
                .fold(0.0, f64::max)
        }

        let candles = make_monotonic_trending_candles(3000);
        let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
        let lr = crate::double_ma::compute_log_returns_f64(close);
        let strategy = StrategyConfig::default();
        let cfg = VccStrategyParams::default();
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();

        let fast_cpu = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::CpuOnly,
            &cfg,
            6,
            6,
            Some(&prepared),
        )
        .unwrap();
        let trend_cpu =
            compute_vcc_trend_signals(&candles, "close", &Backend::CpuOnly, 12, Some(&prepared))
                .unwrap();
        let start_t = compute_vcc_start_t(
            0,
            0,
            fast_cpu.first_valid,
            trend_cpu.first_valid,
            12,
            strategy.trade_on_next_bar,
        );
        let desired =
            build_vcc_desired_positions(close, &fast_cpu, &trend_cpu, start_t, &strategy, &cfg);
        let metrics_cpu =
            eval_asr_metrics_from_desired(&candles, &lr, &desired, start_t, &strategy);
        let fast_gpu_host = compute_vcc_fast_signals(
            &candles,
            "close",
            &Backend::Auto { device_id: 0 },
            &cfg,
            6,
            6,
            Some(&prepared),
        )
        .unwrap();

        let (sq_sweep, mo_sweep) =
            compute_vcc_fast_dual_sweep_cuda_device(&prepared, &cfg, 5, 7, 5, 7, 1, 0).unwrap();
        let tr_sweep = compute_vcc_ttm_trend_sweep_cuda_device(&prepared, 12, 16, 1, 0).unwrap();

        let fast_squeeze_rows = vec![
            sq_sweep.row_ptr_for_period(5).unwrap(),
            sq_sweep.row_ptr_for_period(6).unwrap(),
            sq_sweep.row_ptr_for_period(7).unwrap(),
        ];
        let fast_momentum_rows = vec![
            mo_sweep.row_ptr_for_period(5).unwrap(),
            mo_sweep.row_ptr_for_period(6).unwrap(),
            mo_sweep.row_ptr_for_period(7).unwrap(),
        ];
        let trend_rows_ptr = vec![
            tr_sweep.row_ptr_for_period(12).unwrap(),
            tr_sweep.row_ptr_for_period(13).unwrap(),
            tr_sweep.row_ptr_for_period(14).unwrap(),
            tr_sweep.row_ptr_for_period(15).unwrap(),
            tr_sweep.row_ptr_for_period(16).unwrap(),
        ];

        let signals = VccGpuDeviceSignalMatrices {
            close_f64: close,
            close_f32: prepared.close(),
            close_device_ptr: prepared.close_device().device_ptr(),
            cols: close.len(),
            fast_rows: fast_squeeze_rows.len(),
            trend_rows: trend_rows_ptr.len(),
            fast_squeeze_rows: &fast_squeeze_rows,
            fast_squeeze_host_row_major: None,
            fast_momentum_rows: &fast_momentum_rows,
            fast_momentum_host_row_major: None,
            trend_rows_ptr: &trend_rows_ptr,
            trend_host_row_major: None,
        };
        let fast_dev = VccFastSignals {
            squeeze_state: sweep_row_to_host(&sq_sweep, 6),
            momentum: sweep_row_to_host(&mo_sweep, 6),
            first_valid: fast_cpu.first_valid,
        };
        let trend_dev = VccTrendSignals {
            trend_state: sweep_row_to_host(&tr_sweep, 12),
            first_valid: trend_cpu.first_valid,
        };
        let desired_dev =
            build_vcc_desired_positions(close, &fast_dev, &trend_dev, start_t, &strategy, &cfg);
        let metrics_dev_cpu =
            eval_asr_metrics_from_desired(&candles, &lr, &desired_dev, start_t, &strategy);
        let combo_start = [start_t as i32];
        let combo_end = [close.len() as i32];
        let combos = VccGpuComboBatch {
            fast_row_index: &[1],
            trend_row_index: &[0],
            start_t: &combo_start,
            metric_start_t: &combo_start,
            end_t: &combo_end,
        };
        let metrics_gpu =
            eval_vcc_metrics_gpu_kernel_device_rows(0, &signals, &combos, &strategy, &cfg).unwrap();

        eprintln!(
            "vcc optimizer-shape fast6 diag: start_t={} fast_first_valid={} trend_first_valid={} squeeze_diff={} momentum_diff={} host_gpu_momentum_diff={} trend_diff={} cpu pnl={} dev_cpu pnl={} gpu pnl={} cpu trades={} dev_cpu trades={} gpu trades={}",
            start_t,
            fast_cpu.first_valid,
            trend_cpu.first_valid,
            max_abs_diff(&fast_cpu.squeeze_state, &fast_dev.squeeze_state),
            max_abs_diff(&fast_cpu.momentum, &fast_dev.momentum),
            max_abs_diff(&fast_cpu.momentum, &fast_gpu_host.momentum),
            max_abs_diff(&trend_cpu.trend_state, &trend_dev.trend_state),
            metrics_cpu.pnl,
            metrics_dev_cpu.pnl,
            metrics_gpu[0].pnl,
            metrics_cpu.trades,
            metrics_dev_cpu.trades,
            metrics_gpu[0].trades
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    #[ignore = "diagnostic"]
    fn vcc_metrics_diagnostic_validation_batch_monotonic_with_cpu_momentum() {
        fn sweep_row_to_host(sweep: &VccPeriodSweepDevice, period: u16) -> Vec<f64> {
            let mut out = vec![0.0f32; sweep.matrix.rows * sweep.matrix.cols];
            match &sweep.matrix.owner {
                VccDeviceMatrixOwner::Raw(raw) => raw.buf.copy_to(&mut out).unwrap(),
                VccDeviceMatrixOwner::Dispatch(_) => panic!("unexpected dispatch owner"),
            }
            let row = sweep.row_index_for_period(period).unwrap();
            let cols = sweep.matrix.cols;
            out[(row * cols)..((row + 1) * cols)]
                .iter()
                .map(|x| *x as f64)
                .collect()
        }
        fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
            a.iter()
                .zip(b.iter())
                .filter_map(|(x, y)| (x.is_finite() && y.is_finite()).then_some((x - y).abs()))
                .fold(0.0, f64::max)
        }

        let candles = {
            let len = 4096usize;
            let mut ts = Vec::with_capacity(len);
            let mut open = Vec::with_capacity(len);
            let mut high = Vec::with_capacity(len);
            let mut low = Vec::with_capacity(len);
            let mut close = Vec::with_capacity(len);
            let mut vol: Vec<f64> = Vec::with_capacity(len);
            for i in 0..len {
                ts.push(i as i64);
                let base = 100.0 + i as f64 * 0.1;
                open.push(base);
                high.push(base + 1.0);
                low.push(base - 1.0);
                close.push(base + 0.25);
                vol.push(1_000.0);
            }
            Candles::new(ts, open, high, low, close, vol)
        };
        let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
        let lr = crate::double_ma::compute_log_returns_f64(close);
        let strategy = StrategyConfig::default();
        let cfg = VccStrategyParams::default();
        let prepared =
            PreparedCudaIndicatorInputs::from_candles_on_device(&candles, "close", 0).unwrap();
        let combos_spec = [(8u16, 24u16), (12u16, 36u16)];

        let mut cpu_metrics = Vec::new();
        let mut fast_rows = Vec::new();
        let mut trend_rows = Vec::new();
        let mut combo_fast_idx = Vec::new();
        let mut combo_trend_idx = Vec::new();
        let mut combo_start = Vec::new();
        let mut host_momentum_row_major = Vec::new();

        let (sq_sweep, mo_sweep) =
            compute_vcc_fast_dual_sweep_cuda_device(&prepared, &cfg, 8, 12, 8, 12, 1, 0).unwrap();
        let tr_sweep = compute_vcc_ttm_trend_sweep_cuda_device(&prepared, 24, 36, 12, 0).unwrap();

        for (i, (fast_len, slow_len)) in combos_spec.iter().copied().enumerate() {
            let periods = resolve_vcc_periods(&cfg, fast_len, slow_len);
            let fast_cpu = compute_vcc_fast_signals(
                &candles,
                "close",
                &Backend::CpuOnly,
                &cfg,
                periods.squeeze,
                periods.momentum,
                Some(&prepared),
            )
            .unwrap();
            let trend_cpu = compute_vcc_trend_signals(
                &candles,
                "close",
                &Backend::CpuOnly,
                periods.trend,
                Some(&prepared),
            )
            .unwrap();
            let start_t = compute_vcc_start_t(
                0,
                0,
                fast_cpu.first_valid,
                trend_cpu.first_valid,
                usize::from(periods.squeeze.max(periods.momentum).max(periods.trend)).max(1),
                strategy.trade_on_next_bar,
            );
            let desired =
                build_vcc_desired_positions(close, &fast_cpu, &trend_cpu, start_t, &strategy, &cfg);
            cpu_metrics.push(eval_asr_metrics_from_desired(
                &candles, &lr, &desired, start_t, &strategy,
            ));

            let fast_roundtrip = VccFastSignals {
                squeeze_state: fast_cpu.squeeze_state.clone(),
                momentum: fast_cpu
                    .momentum
                    .iter()
                    .map(|v| (*v as f32) as f64)
                    .collect(),
                first_valid: fast_cpu.first_valid,
            };
            let desired_roundtrip = build_vcc_desired_positions(
                close,
                &fast_roundtrip,
                &trend_cpu,
                start_t,
                &strategy,
                &cfg,
            );
            let roundtrip_metrics = eval_asr_metrics_from_desired(
                &candles,
                &lr,
                &desired_roundtrip,
                start_t,
                &strategy,
            );

            let sq_dev = sweep_row_to_host(&sq_sweep, periods.squeeze);
            let tr_dev = sweep_row_to_host(&tr_sweep, periods.trend);
            eprintln!(
                "vcc monotonic validation diag combo=({fast_len},{slow_len}) squeeze_diff={} trend_diff={} roundtrip pnl={} roundtrip trades={}",
                max_abs_diff(&fast_cpu.squeeze_state, &sq_dev),
                max_abs_diff(&trend_cpu.trend_state, &tr_dev),
                roundtrip_metrics.pnl,
                roundtrip_metrics.trades,
            );

            fast_rows.push(sq_sweep.row_ptr_for_period(periods.squeeze).unwrap());
            trend_rows.push(tr_sweep.row_ptr_for_period(periods.trend).unwrap());
            combo_fast_idx.push(i as i32);
            combo_trend_idx.push(i as i32);
            combo_start.push(start_t as i32);
            host_momentum_row_major.extend(fast_cpu.momentum.iter().map(|v| *v as f32));
            let _ = mo_sweep.row_ptr_for_period(periods.momentum).unwrap();
        }

        let signals = VccGpuDeviceSignalMatrices {
            close_f64: close,
            close_f32: prepared.close(),
            close_device_ptr: prepared.close_device().device_ptr(),
            cols: close.len(),
            fast_rows: fast_rows.len(),
            trend_rows: trend_rows.len(),
            fast_squeeze_rows: &fast_rows,
            fast_squeeze_host_row_major: None,
            fast_momentum_rows: &[],
            fast_momentum_host_row_major: Some(&host_momentum_row_major),
            trend_rows_ptr: &trend_rows,
            trend_host_row_major: None,
        };
        let combo_end = vec![close.len() as i32; combo_start.len()];
        let combos = VccGpuComboBatch {
            fast_row_index: &combo_fast_idx,
            trend_row_index: &combo_trend_idx,
            start_t: &combo_start,
            metric_start_t: &combo_start,
            end_t: &combo_end,
        };
        let gpu_metrics =
            eval_vcc_metrics_gpu_kernel_device_rows(0, &signals, &combos, &strategy, &cfg).unwrap();

        for (idx, (cpu, gpu)) in cpu_metrics.iter().zip(gpu_metrics.iter()).enumerate() {
            eprintln!(
                "vcc monotonic validation metrics idx={} cpu pnl={} gpu pnl={} cpu trades={} gpu trades={}",
                idx, cpu.pnl, gpu.pnl, cpu.trades, gpu.trades
            );
        }
    }
}
