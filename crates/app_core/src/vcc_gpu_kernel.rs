#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_METRICS_STRIDE,
};
use crate::standalone_gpu_runtime::checked_i32;
use crate::vcc::VccStrategyParams;
use cust::context::{Context, CurrentContext};
use cust::device::Device;
use cust::memory::CopyDestination;
use cust::module::ModuleJitOption;
use cust::stream::{Stream, StreamFlags};
use cust::{memory::DeviceBuffer, module::Module, CudaFlags};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;
use ta_strategies::double_ma::{Metrics, StrategyConfig};

pub struct VccGpuDeviceSignalMatrices<'a> {
    pub close_f64: &'a [f64],
    pub cols: usize,
    pub fast_rows: usize,
    pub trend_rows: usize,
    pub fast_squeeze_rows: &'a [u64],
    pub fast_squeeze_host_row_major: Option<&'a [f32]>,
    pub fast_momentum_rows: &'a [u64],
    pub fast_momentum_host_row_major: Option<&'a [f32]>,
    pub trend_rows_ptr: &'a [u64],
    pub trend_host_row_major: Option<&'a [f32]>,
}

pub struct VccGpuComboBatch<'a> {
    pub fast_row_index: &'a [i32],
    pub trend_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub metric_start_t: &'a [i32],
    pub end_t: &'a [i32],
}

#[derive(Clone, Debug)]
pub struct VccGpuMetricEval {
    pub metrics: Metrics,
    pub step_return_sum: f64,
    pub step_return_sumsq: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VccGpuEvalMode {
    MetricsOnly,
    Detailed,
}

struct VccCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
}

impl VccCudaRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let context = Arc::new(Context::new(device).map_err(|e| e.to_string())?);

        let ptx: &str = include_str!(concat!(env!("OUT_DIR"), "/double_crossover.ptx"));
        let jit_opts = &[
            ModuleJitOption::DetermineTargetFromContext,
            ModuleJitOption::OptLevel(cust::module::OptLevel::O2),
        ];
        let module = match Module::from_ptx(ptx, jit_opts) {
            Ok(m) => m,
            Err(_) => Module::from_ptx(ptx, &[ModuleJitOption::DetermineTargetFromContext])
                .or_else(|_| Module::from_ptx(ptx, &[]))
                .map_err(|e| e.to_string())?,
        };

        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;

        Ok(Self {
            context,
            module,
            stream,
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }
}

thread_local! {
    static VCC_GPU_RUNTIME: RefCell<HashMap<u32, VccCudaRuntime>> = RefCell::new(HashMap::new());
}

#[inline]
fn vcc_eval_block_x(combo_count: usize) -> u32 {
    // One thread owns one combo and walks the full time series, so smaller blocks at
    // low/medium combo counts help keep more SMs busy by increasing grid block count.
    if combo_count <= 4_096 {
        32
    } else if combo_count <= 16_384 {
        64
    } else if combo_count <= 65_536 {
        128
    } else {
        256
    }
}

#[inline]
fn compute_log_returns_f32(close_f64: &[f64]) -> Vec<f32> {
    let mut out = vec![0.0f32; close_f64.len()];
    for t in 1..close_f64.len() {
        let p = close_f64[t];
        let pm = close_f64[t - 1];
        out[t] = if p.is_finite() && pm.is_finite() && p > 0.0 && pm > 0.0 {
            (p.ln() - pm.ln()) as f32
        } else {
            0.0
        };
    }
    out
}

#[inline]
fn compute_log_returns_f64(close_f64: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0f64; close_f64.len()];
    for t in 1..close_f64.len() {
        let p = close_f64[t];
        let pm = close_f64[t - 1];
        out[t] = if p.is_finite() && pm.is_finite() && p > 0.0 && pm > 0.0 {
            p.ln() - pm.ln()
        } else {
            0.0
        };
    }
    out
}

fn validate_inputs_device_rows(
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("VCC GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f64.len() != n {
        return Err("VCC GPU close length mismatch".to_string());
    }
    if let Some(host_rows) = signals.fast_squeeze_host_row_major {
        let expected = signals
            .fast_rows
            .checked_mul(signals.cols)
            .ok_or_else(|| "VCC GPU fast squeeze host matrix overflow".to_string())?;
        if host_rows.len() != expected {
            return Err("VCC GPU fast squeeze host matrix length mismatch".to_string());
        }
    } else if signals.fast_squeeze_rows.len() != signals.fast_rows {
        return Err("VCC GPU fast row-pointer length mismatch".to_string());
    }
    if let Some(host_rows) = signals.fast_momentum_host_row_major {
        let expected = signals
            .fast_rows
            .checked_mul(signals.cols)
            .ok_or_else(|| "VCC GPU fast momentum host matrix overflow".to_string())?;
        if host_rows.len() != expected {
            return Err("VCC GPU fast momentum host matrix length mismatch".to_string());
        }
    } else if signals.fast_momentum_rows.len() != signals.fast_rows {
        return Err("VCC GPU fast row-pointer length mismatch".to_string());
    }
    if let Some(host_rows) = signals.trend_host_row_major {
        let expected = signals
            .trend_rows
            .checked_mul(signals.cols)
            .ok_or_else(|| "VCC GPU trend host matrix overflow".to_string())?;
        if host_rows.len() != expected {
            return Err("VCC GPU trend host matrix length mismatch".to_string());
        }
    } else if signals.trend_rows_ptr.len() != signals.trend_rows {
        return Err("VCC GPU trend row-pointer length mismatch".to_string());
    }

    let cols_i32 = checked_i32("VCC GPU cols", signals.cols)?;
    let fast_rows_i32 = checked_i32("VCC GPU fast row count", signals.fast_rows)?;
    let trend_rows_i32 = checked_i32("VCC GPU trend row count", signals.trend_rows)?;
    let combos_len = combos.fast_row_index.len();
    checked_i32("VCC GPU combo count", combos_len)?;
    if combos.trend_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.metric_start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("VCC GPU combo vector length mismatch".to_string());
    }
    for (idx, (((&fast_row, &trend_row), &start_t), (&metric_start_t, &end_t))) in combos
        .fast_row_index
        .iter()
        .zip(combos.trend_row_index.iter())
        .zip(combos.start_t.iter())
        .zip(combos.metric_start_t.iter().zip(combos.end_t.iter()))
        .enumerate()
    {
        if fast_row < 0 || fast_row >= fast_rows_i32 {
            return Err(format!(
                "VCC GPU fast_row_index[{idx}] out of bounds: {fast_row} not in [0, {fast_rows_i32})"
            ));
        }
        if trend_row < 0 || trend_row >= trend_rows_i32 {
            return Err(format!(
                "VCC GPU trend_row_index[{idx}] out of bounds: {trend_row} not in [0, {trend_rows_i32})"
            ));
        }
        if start_t < 0 || metric_start_t < 0 || end_t < 0 {
            return Err(format!("VCC GPU combo window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "VCC GPU combo window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if metric_start_t < start_t || metric_start_t > end_t {
            return Err(format!(
                "VCC GPU combo window[{idx}] requires start_t <= metric_start_t <= end_t but got start={start_t}, metric_start={metric_start_t}, end={end_t}"
            ));
        }
        if end_t > cols_i32 {
            return Err(format!(
                "VCC GPU combo window[{idx}] end_t {end_t} exceeds cols {cols_i32}"
            ));
        }
    }
    Ok(())
}

fn launch_pack_rows_u64_ptr_f32(
    rt: &mut VccCudaRuntime,
    row_ptrs: &DeviceBuffer<u64>,
    rows: usize,
    cols: usize,
    out_row_major: &mut DeviceBuffer<f32>,
) -> Result<(), String> {
    if rows == 0 || cols == 0 {
        return Ok(());
    }

    let kernel = rt
        .module
        .get_function("pack_rows_u64_ptr_f32")
        .map_err(|e| format!("VCC GPU missing pack_rows_u64_ptr_f32 kernel: {e}"))?;

    let total = rows
        .checked_mul(cols)
        .ok_or_else(|| "VCC GPU pack rows overflow".to_string())?;

    let mut row_ptrs_ptr = row_ptrs.as_device_ptr().as_raw();
    let mut rows_i = checked_i32("VCC GPU pack row count", rows)?;
    let mut cols_i = checked_i32("VCC GPU pack cols", cols)?;
    let mut out_ptr = out_row_major.as_device_ptr().as_raw();

    let block_x = 256u32;
    let total_u32 =
        u32::try_from(total).map_err(|_| "VCC GPU pack workload exceeds u32 range".to_string())?;
    let grid_x = (total_u32 + block_x - 1) / block_x;
    unsafe {
        let args: &mut [*mut c_void] = &mut [
            &mut row_ptrs_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut cols_i as *mut _ as *mut c_void,
            &mut out_ptr as *mut _ as *mut c_void,
        ];
        rt.stream
            .launch(&kernel, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn eval_vcc_metrics_gpu_kernel_device_rows_impl(
    device_id: u32,
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcc: &VccStrategyParams,
    mode: VccGpuEvalMode,
) -> Result<Vec<VccGpuMetricEval>, String> {
    validate_inputs_device_rows(signals, combos)?;
    let combo_count = combos.fast_row_index.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }

    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "VCC GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close_f32: Vec<f32> = signals.close_f64.iter().map(|v| *v as f32).collect();
    let lr_f32 = compute_log_returns_f32(signals.close_f64);
    let lr_f64 = compute_log_returns_f64(signals.close_f64);

    let out_stride = NON_DOUBLE_MA_GPU_METRICS_STRIDE;
    let out_len = combo_count
        .checked_mul(out_stride)
        .ok_or_else(|| "VCC GPU metrics buffer overflow".to_string())?;
    let slice_out_len = combo_count
        .checked_mul(2)
        .ok_or_else(|| "VCC GPU slice metrics buffer overflow".to_string())?;
    let need_slice_readback = matches!(mode, VccGpuEvalMode::Detailed);

    VCC_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            let rt = VccCudaRuntime::new(device_id)?;
            runtimes.insert(device_id, rt);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "VCC GPU runtime missing".to_string())?;

        rt.set_current()?;

        let d_close = DeviceBuffer::from_slice(&close_f32).map_err(|e| e.to_string())?;
        let d_lr = DeviceBuffer::from_slice(&lr_f32).map_err(|e| e.to_string())?;
        let d_lr64 = DeviceBuffer::from_slice(&lr_f64).map_err(|e| e.to_string())?;

        let fast_mat_len = signals
            .fast_rows
            .checked_mul(signals.cols)
            .ok_or_else(|| "VCC GPU fast matrix overflow".to_string())?;
        let trend_mat_len = signals
            .trend_rows
            .checked_mul(signals.cols)
            .ok_or_else(|| "VCC GPU trend matrix overflow".to_string())?;

        let mut d_fast_sq = if let Some(host_rows) = signals.fast_squeeze_host_row_major {
            DeviceBuffer::from_slice(host_rows).map_err(|e| e.to_string())?
        } else {
            unsafe { DeviceBuffer::<f32>::uninitialized(fast_mat_len) }
                .map_err(|e| e.to_string())?
        };
        let mut d_fast_mom = if let Some(host_rows) = signals.fast_momentum_host_row_major {
            DeviceBuffer::from_slice(host_rows).map_err(|e| e.to_string())?
        } else {
            unsafe { DeviceBuffer::<f32>::uninitialized(fast_mat_len) }
                .map_err(|e| e.to_string())?
        };
        let mut d_trend = if let Some(host_rows) = signals.trend_host_row_major {
            DeviceBuffer::from_slice(host_rows).map_err(|e| e.to_string())?
        } else {
            unsafe { DeviceBuffer::<f32>::uninitialized(trend_mat_len) }
                .map_err(|e| e.to_string())?
        };

        if signals.fast_squeeze_host_row_major.is_none() {
            let d_fast_sq_rows =
                DeviceBuffer::from_slice(signals.fast_squeeze_rows).map_err(|e| e.to_string())?;
            launch_pack_rows_u64_ptr_f32(
                rt,
                &d_fast_sq_rows,
                signals.fast_rows,
                signals.cols,
                &mut d_fast_sq,
            )?;
        }
        if signals.fast_momentum_host_row_major.is_none() {
            let d_fast_mom_rows =
                DeviceBuffer::from_slice(signals.fast_momentum_rows).map_err(|e| e.to_string())?;
            launch_pack_rows_u64_ptr_f32(
                rt,
                &d_fast_mom_rows,
                signals.fast_rows,
                signals.cols,
                &mut d_fast_mom,
            )?;
        }
        if signals.trend_host_row_major.is_none() {
            let d_trend_rows =
                DeviceBuffer::from_slice(signals.trend_rows_ptr).map_err(|e| e.to_string())?;
            launch_pack_rows_u64_ptr_f32(
                rt,
                &d_trend_rows,
                signals.trend_rows,
                signals.cols,
                &mut d_trend,
            )?;
        }

        let d_combo_fast =
            DeviceBuffer::from_slice(combos.fast_row_index).map_err(|e| e.to_string())?;
        let d_combo_trend =
            DeviceBuffer::from_slice(combos.trend_row_index).map_err(|e| e.to_string())?;
        let d_combo_start = DeviceBuffer::from_slice(combos.start_t).map_err(|e| e.to_string())?;
        let d_combo_metric_start =
            DeviceBuffer::from_slice(combos.metric_start_t).map_err(|e| e.to_string())?;
        let d_combo_end = DeviceBuffer::from_slice(combos.end_t).map_err(|e| e.to_string())?;
        let d_out =
            unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }.map_err(|e| e.to_string())?;
        let d_slice_out = unsafe { DeviceBuffer::<f64>::uninitialized(slice_out_len) }
            .map_err(|e| e.to_string())?;

        let kernel = rt
            .module
            .get_function("vcc_backtest_metrics_f32")
            .map_err(|e| format!("VCC GPU missing vcc_backtest_metrics_f32 kernel: {e}"))?;

        let mut flags: u32 = 0;
        if strategy.long_only {
            flags |= 1u32 << 0;
        }
        if !strategy.allow_flip {
            flags |= 1u32 << 1;
        }
        if strategy.trade_on_next_bar {
            flags |= 1u32 << 2;
        }
        flags |= 1u32 << 4;

        let commission = strategy.effective_commission().clamp(0.0, 0.95) as f32;

        let mut close_ptr = d_close.as_device_ptr().as_raw();
        let mut lr_ptr = d_lr.as_device_ptr().as_raw();
        let mut lr64_ptr = d_lr64.as_device_ptr().as_raw();
        let mut fast_sq_ptr = d_fast_sq.as_device_ptr().as_raw();
        let mut fast_mom_ptr = d_fast_mom.as_device_ptr().as_raw();
        let mut trend_ptr = d_trend.as_device_ptr().as_raw();
        let mut combo_fast_ptr = d_combo_fast.as_device_ptr().as_raw();
        let mut combo_trend_ptr = d_combo_trend.as_device_ptr().as_raw();
        let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
        let mut combo_metric_start_ptr = d_combo_metric_start.as_device_ptr().as_raw();
        let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
        let mut out_ptr = d_out.as_device_ptr().as_raw();
        let mut slice_out_ptr = d_slice_out.as_device_ptr().as_raw();

        let mut t_i = checked_i32("VCC GPU cols", signals.cols)?;
        let mut fast_rows_i = checked_i32("VCC GPU fast row count", signals.fast_rows)?;
        let mut fast_stride_i = checked_i32("VCC GPU fast stride", signals.cols)?;
        let mut trend_rows_i = checked_i32("VCC GPU trend row count", signals.trend_rows)?;
        let mut trend_stride_i = checked_i32("VCC GPU trend stride", signals.cols)?;
        let mut combo_count_i = checked_i32("VCC GPU combo count", combo_count)?;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut require_mom_accel_i = i32::from(vcc.require_momentum_accel);
        let mut confirm_bars_i = i32::from(vcc.confirm_bars);
        let mut max_trade_bars_i = i32::from(vcc.max_trade_bars);
        let mut out_stride_i = checked_i32("VCC GPU metrics stride", out_stride)?;

        let block_x = vcc_eval_block_x(combo_count);
        let combo_count_u32 = u32::try_from(combo_count)
            .map_err(|_| "VCC GPU combo count exceeds u32 range".to_string())?;
        let grid_x = (combo_count_u32 + block_x - 1) / block_x;
        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut lr64_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut fast_sq_ptr as *mut _ as *mut c_void,
                &mut fast_mom_ptr as *mut _ as *mut c_void,
                &mut fast_rows_i as *mut _ as *mut c_void,
                &mut fast_stride_i as *mut _ as *mut c_void,
                &mut trend_ptr as *mut _ as *mut c_void,
                &mut trend_rows_i as *mut _ as *mut c_void,
                &mut trend_stride_i as *mut _ as *mut c_void,
                &mut combo_fast_ptr as *mut _ as *mut c_void,
                &mut combo_trend_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_metric_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut require_mom_accel_i as *mut _ as *mut c_void,
                &mut confirm_bars_i as *mut _ as *mut c_void,
                &mut max_trade_bars_i as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
                &mut slice_out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&kernel, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }

        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut host_out = vec![0.0f32; out_len];
        d_out.copy_to(&mut host_out).map_err(|e| e.to_string())?;
        let host_slice_out = if need_slice_readback {
            let mut host_slice_out = vec![0.0f64; slice_out_len];
            d_slice_out
                .copy_to(&mut host_slice_out)
                .map_err(|e| e.to_string())?;
            Some(host_slice_out)
        } else {
            None
        };

        let mut metrics = Vec::with_capacity(combo_count);
        for i in 0..combo_count {
            let base = i * out_stride;
            let (step_return_sum, step_return_sumsq) =
                if let Some(slice_out) = host_slice_out.as_ref() {
                    let slice_base = i * 2;
                    (slice_out[slice_base], slice_out[slice_base + 1])
                } else {
                    (0.0, 0.0)
                };

            metrics.push(VccGpuMetricEval {
                metrics: decode_non_double_ma_gpu_metrics_row(&host_out[base..base + out_stride]),
                step_return_sum: if step_return_sum.is_finite() {
                    step_return_sum
                } else {
                    0.0
                },
                step_return_sumsq: if step_return_sumsq.is_finite() {
                    step_return_sumsq.max(0.0)
                } else {
                    0.0
                },
            });
        }
        Ok(metrics)
    })
}

pub fn eval_vcc_metrics_gpu_kernel_device_rows_detailed(
    device_id: u32,
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcc: &VccStrategyParams,
) -> Result<Vec<VccGpuMetricEval>, String> {
    eval_vcc_metrics_gpu_kernel_device_rows_impl(
        device_id,
        signals,
        combos,
        strategy,
        vcc,
        VccGpuEvalMode::Detailed,
    )
}

pub fn eval_vcc_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcc: &VccStrategyParams,
) -> Result<Vec<Metrics>, String> {
    eval_vcc_metrics_gpu_kernel_device_rows_impl(
        device_id,
        signals,
        combos,
        strategy,
        vcc,
        VccGpuEvalMode::MetricsOnly,
    )
    .map(|rows| rows.into_iter().map(|row| row.metrics).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_signals(close_f64: &[f64]) -> VccGpuDeviceSignalMatrices<'_> {
        const FAST_PTRS: [u64; 2] = [11, 12];
        const MOM_PTRS: [u64; 2] = [13, 14];
        const TREND_PTRS: [u64; 2] = [15, 16];
        VccGpuDeviceSignalMatrices {
            close_f64,
            cols: close_f64.len(),
            fast_rows: 2,
            trend_rows: 2,
            fast_squeeze_rows: &FAST_PTRS,
            fast_squeeze_host_row_major: None,
            fast_momentum_rows: &MOM_PTRS,
            fast_momentum_host_row_major: None,
            trend_rows_ptr: &TREND_PTRS,
            trend_host_row_major: None,
        }
    }

    #[test]
    fn validate_inputs_rejects_out_of_range_combo_rows() {
        let close = [100.0f64, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close);
        let combos = VccGpuComboBatch {
            fast_row_index: &[2],
            trend_row_index: &[0],
            start_t: &[1],
            metric_start_t: &[1],
            end_t: &[4],
        };

        let err = validate_inputs_device_rows(&signals, &combos).expect_err("row bounds");
        assert!(err.contains("fast_row_index[0]"));
    }

    #[test]
    fn validate_inputs_rejects_invalid_windows() {
        let close = [100.0f64, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close);

        let negative = VccGpuComboBatch {
            fast_row_index: &[0],
            trend_row_index: &[0],
            start_t: &[-1],
            metric_start_t: &[0],
            end_t: &[4],
        };
        let err = validate_inputs_device_rows(&signals, &negative).expect_err("negative bounds");
        assert!(err.contains("must be non-negative"));

        let unordered = VccGpuComboBatch {
            fast_row_index: &[0],
            trend_row_index: &[0],
            start_t: &[2],
            metric_start_t: &[1],
            end_t: &[4],
        };
        let err = validate_inputs_device_rows(&signals, &unordered).expect_err("unordered");
        assert!(err.contains("start_t <= metric_start_t <= end_t"));
    }

    #[test]
    fn validate_inputs_rejects_host_matrix_length_mismatch() {
        let close = [100.0f64, 101.0, 102.0, 103.0];
        let bad_fast = [1.0f32; 3];
        let signals = VccGpuDeviceSignalMatrices {
            close_f64: &close,
            cols: close.len(),
            fast_rows: 2,
            trend_rows: 2,
            fast_squeeze_rows: &[],
            fast_squeeze_host_row_major: Some(&bad_fast),
            fast_momentum_rows: &[13, 14],
            fast_momentum_host_row_major: None,
            trend_rows_ptr: &[15, 16],
            trend_host_row_major: None,
        };
        let combos = VccGpuComboBatch {
            fast_row_index: &[0],
            trend_row_index: &[0],
            start_t: &[1],
            metric_start_t: &[1],
            end_t: &[4],
        };

        let err = validate_inputs_device_rows(&signals, &combos).expect_err("host len mismatch");
        assert!(err.contains("fast squeeze host matrix length mismatch"));
    }

    #[test]
    fn checked_i32_rejects_overflow() {
        let err = checked_i32("VCC GPU test value", usize::MAX).expect_err("overflow");
        assert!(err.contains("exceeds i32 range"));
    }
}
