#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

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

    let combos_len = combos.fast_row_index.len();
    if combos.trend_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.metric_start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("VCC GPU combo vector length mismatch".to_string());
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
    let mut rows_i =
        i32::try_from(rows).map_err(|_| "VCC GPU rows exceed i32 range".to_string())?;
    let mut cols_i =
        i32::try_from(cols).map_err(|_| "VCC GPU cols exceed i32 range".to_string())?;
    let mut out_ptr = out_row_major.as_device_ptr().as_raw();

    let block_x = 256u32;
    let grid_x = ((total as u32) + block_x - 1) / block_x;
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

pub fn eval_vcc_metrics_gpu_kernel_device_rows_detailed(
    device_id: u32,
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcc: &VccStrategyParams,
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

    let out_stride = 10usize;
    let out_len = combo_count
        .checked_mul(out_stride)
        .ok_or_else(|| "VCC GPU metrics buffer overflow".to_string())?;
    let slice_out_len = combo_count
        .checked_mul(2)
        .ok_or_else(|| "VCC GPU slice metrics buffer overflow".to_string())?;

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

        let mut t_i = signals.cols as i32;
        let mut fast_rows_i = signals.fast_rows as i32;
        let mut fast_stride_i = signals.cols as i32;
        let mut trend_rows_i = signals.trend_rows as i32;
        let mut trend_stride_i = signals.cols as i32;
        let mut combo_count_i = combo_count as i32;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut require_mom_accel_i = i32::from(vcc.require_momentum_accel);
        let mut confirm_bars_i = i32::from(vcc.confirm_bars);
        let mut max_trade_bars_i = i32::from(vcc.max_trade_bars);
        let mut out_stride_i = out_stride as i32;

        let block_x = vcc_eval_block_x(combo_count);
        let grid_x = ((combo_count as u32) + block_x - 1) / block_x;
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
        let mut host_slice_out = vec![0.0f64; slice_out_len];
        d_slice_out
            .copy_to(&mut host_slice_out)
            .map_err(|e| e.to_string())?;

        let mut metrics = Vec::with_capacity(combo_count);
        for i in 0..combo_count {
            let base = i * out_stride;
            let pnl = host_out[base + 0] as f64;
            let sharpe = host_out[base + 1] as f64;
            let sortino = host_out[base + 2] as f64;
            let calmar = host_out[base + 3] as f64;
            let omega = host_out[base + 4] as f64;
            let volatility = host_out[base + 5] as f64;
            let max_dd = host_out[base + 6] as f64;
            let trades_f = host_out[base + 7];
            let exposure = host_out[base + 8] as f64;
            let net_exposure = host_out[base + 9] as f64;
            let slice_base = i * 2;
            let step_return_sum = host_slice_out[slice_base];
            let step_return_sumsq = host_slice_out[slice_base + 1];

            metrics.push(VccGpuMetricEval {
                metrics: Metrics {
                    pnl: if pnl.is_finite() { pnl } else { 0.0 },
                    sharpe: if sharpe.is_finite() { sharpe } else { 0.0 },
                    sortino: if sortino.is_finite() { sortino } else { 0.0 },
                    downside_deviation: 0.0,
                    calmar: if calmar.is_finite() { calmar } else { 0.0 },
                    omega: if omega.is_finite() { omega } else { 0.0 },
                    volatility: if volatility.is_finite() {
                        volatility.max(0.0)
                    } else {
                        0.0
                    },
                    ulcer_index: 0.0,
                    martin_ratio: 0.0,
                    max_dd: if max_dd.is_finite() {
                        max_dd.max(0.0)
                    } else {
                        0.0
                    },
                    trades: if trades_f.is_finite() && trades_f >= 0.0 {
                        trades_f as u32
                    } else {
                        0
                    },
                    exposure: if exposure.is_finite() { exposure } else { 0.0 },
                    net_exposure: if net_exposure.is_finite() {
                        net_exposure
                    } else {
                        0.0
                    },
                },
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

pub fn eval_vcc_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &VccGpuDeviceSignalMatrices<'_>,
    combos: &VccGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcc: &VccStrategyParams,
) -> Result<Vec<Metrics>, String> {
    eval_vcc_metrics_gpu_kernel_device_rows_detailed(device_id, signals, combos, strategy, vcc)
        .map(|rows| rows.into_iter().map(|row| row.metrics).collect())
}
