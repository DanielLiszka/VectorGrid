#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::asr::AsrStrategyParams;
use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_METRICS_STRIDE,
};
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

pub struct AsrGpuSignalMatrices<'a> {
    pub close_f64: &'a [f64],
    pub cols: usize,
    pub fast_rows: usize,
    pub slow_rows: usize,
    pub fast_squeeze_row_major: &'a [f32],
    pub fast_momentum_row_major: &'a [f32],
    pub slow_angle_row_major: &'a [f32],
    pub slow_er_row_major: &'a [f32],
    pub slow_z_row_major: &'a [f32],
    pub kaufman_long: &'a [f32],
    pub kaufman_short: &'a [f32],
}

pub struct AsrGpuDeviceSignalMatrices<'a> {
    pub close_f64: &'a [f64],
    pub cols: usize,
    pub fast_rows: usize,
    pub slow_rows: usize,
    pub fast_squeeze_rows: &'a [u64],
    pub fast_momentum_rows: &'a [u64],
    pub slow_angle_rows: &'a [u64],
    pub slow_er_rows: &'a [u64],
    pub slow_z_rows: &'a [u64],
    pub kaufman_long_row: u64,
    pub kaufman_short_row: u64,
}

pub struct AsrGpuComboBatch<'a> {
    pub fast_row_index: &'a [i32],
    pub slow_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
}

struct AsrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
}

impl AsrCudaRuntime {
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
    static ASR_GPU_RUNTIME: RefCell<HashMap<u32, AsrCudaRuntime>> = RefCell::new(HashMap::new());
}

#[inline]
fn asr_eval_block_x(combo_count: usize) -> u32 {
    if combo_count <= 2_048 {
        64
    } else if combo_count <= 8_192 {
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

fn validate_inputs(
    signals: &AsrGpuSignalMatrices<'_>,
    combos: &AsrGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("ASR GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f64.len() != n {
        return Err("ASR GPU close length mismatch".to_string());
    }
    if signals.kaufman_long.len() != n || signals.kaufman_short.len() != n {
        return Err("ASR GPU kaufman length mismatch".to_string());
    }
    let fast_expected = signals
        .fast_rows
        .checked_mul(n)
        .ok_or_else(|| "ASR GPU fast matrix overflow".to_string())?;
    let slow_expected = signals
        .slow_rows
        .checked_mul(n)
        .ok_or_else(|| "ASR GPU slow matrix overflow".to_string())?;
    if signals.fast_squeeze_row_major.len() != fast_expected
        || signals.fast_momentum_row_major.len() != fast_expected
    {
        return Err("ASR GPU fast matrix length mismatch".to_string());
    }
    if signals.slow_angle_row_major.len() != slow_expected
        || signals.slow_er_row_major.len() != slow_expected
        || signals.slow_z_row_major.len() != slow_expected
    {
        return Err("ASR GPU slow matrix length mismatch".to_string());
    }

    let combos_len = combos.fast_row_index.len();
    if combos.slow_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("ASR GPU combo vector length mismatch".to_string());
    }
    Ok(())
}

fn validate_inputs_device_rows(
    signals: &AsrGpuDeviceSignalMatrices<'_>,
    combos: &AsrGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("ASR GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f64.len() != n {
        return Err("ASR GPU close length mismatch".to_string());
    }
    if signals.fast_squeeze_rows.len() != signals.fast_rows
        || signals.fast_momentum_rows.len() != signals.fast_rows
    {
        return Err("ASR GPU fast row-pointer length mismatch".to_string());
    }
    if signals.slow_angle_rows.len() != signals.slow_rows
        || signals.slow_er_rows.len() != signals.slow_rows
        || signals.slow_z_rows.len() != signals.slow_rows
    {
        return Err("ASR GPU slow row-pointer length mismatch".to_string());
    }
    if signals.kaufman_long_row == 0 || signals.kaufman_short_row == 0 {
        return Err("ASR GPU kaufman row pointer is null".to_string());
    }
    let combos_len = combos.fast_row_index.len();
    if combos.slow_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("ASR GPU combo vector length mismatch".to_string());
    }
    Ok(())
}

fn launch_pack_rows_u64_ptr_f32(
    rt: &mut AsrCudaRuntime,
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
        .map_err(|e| e.to_string())?;
    let total = rows
        .checked_mul(cols)
        .ok_or_else(|| "ASR GPU pack rows overflow".to_string())?;

    let mut row_ptrs_ptr = row_ptrs.as_device_ptr().as_raw();
    let mut rows_i =
        i32::try_from(rows).map_err(|_| "ASR GPU rows exceed i32 range".to_string())?;
    let mut cols_i =
        i32::try_from(cols).map_err(|_| "ASR GPU cols exceed i32 range".to_string())?;
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

pub fn eval_asr_metrics_gpu_kernel(
    device_id: u32,
    signals: &AsrGpuSignalMatrices<'_>,
    combos: &AsrGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    asr: &AsrStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.fast_row_index.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }

    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "ASR GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close_f32: Vec<f32> = signals.close_f64.iter().map(|v| *v as f32).collect();
    let lr_f32 = compute_log_returns_f32(signals.close_f64);

    let out_stride = NON_DOUBLE_MA_GPU_METRICS_STRIDE;
    let out_len = combo_count
        .checked_mul(out_stride)
        .ok_or_else(|| "ASR GPU output length overflow".to_string())?;

    ASR_GPU_RUNTIME.with(|cell| -> Result<Vec<Metrics>, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            let rt = AsrCudaRuntime::new(device_id)?;
            map.insert(device_id, rt);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let d_close = DeviceBuffer::from_slice(&close_f32).map_err(|e| e.to_string())?;
        let d_lr = DeviceBuffer::from_slice(&lr_f32).map_err(|e| e.to_string())?;
        let d_fast_sq =
            DeviceBuffer::from_slice(signals.fast_squeeze_row_major).map_err(|e| e.to_string())?;
        let d_fast_mom =
            DeviceBuffer::from_slice(signals.fast_momentum_row_major).map_err(|e| e.to_string())?;
        let d_slow_ang =
            DeviceBuffer::from_slice(signals.slow_angle_row_major).map_err(|e| e.to_string())?;
        let d_slow_er =
            DeviceBuffer::from_slice(signals.slow_er_row_major).map_err(|e| e.to_string())?;
        let d_slow_z =
            DeviceBuffer::from_slice(signals.slow_z_row_major).map_err(|e| e.to_string())?;
        let d_k_long = DeviceBuffer::from_slice(signals.kaufman_long).map_err(|e| e.to_string())?;
        let d_k_short =
            DeviceBuffer::from_slice(signals.kaufman_short).map_err(|e| e.to_string())?;
        let d_combo_fast =
            DeviceBuffer::from_slice(combos.fast_row_index).map_err(|e| e.to_string())?;
        let d_combo_slow =
            DeviceBuffer::from_slice(combos.slow_row_index).map_err(|e| e.to_string())?;
        let d_combo_start = DeviceBuffer::from_slice(combos.start_t).map_err(|e| e.to_string())?;
        let d_combo_end = DeviceBuffer::from_slice(combos.end_t).map_err(|e| e.to_string())?;
        let d_out =
            unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }.map_err(|e| e.to_string())?;

        let kernel = rt
            .module
            .get_function("asr_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;

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
        flags |= 1u32 << 4; // signed exposure

        let commission = strategy.effective_commission().clamp(0.0, 0.95) as f32;
        let use_er_filter = i32::from(asr.use_er_filter);

        let mut close_ptr = d_close.as_device_ptr().as_raw();
        let mut lr_ptr = d_lr.as_device_ptr().as_raw();
        let mut fast_sq_ptr = d_fast_sq.as_device_ptr().as_raw();
        let mut fast_mom_ptr = d_fast_mom.as_device_ptr().as_raw();
        let mut slow_ang_ptr = d_slow_ang.as_device_ptr().as_raw();
        let mut slow_er_ptr = d_slow_er.as_device_ptr().as_raw();
        let mut slow_z_ptr = d_slow_z.as_device_ptr().as_raw();
        let mut k_long_ptr = d_k_long.as_device_ptr().as_raw();
        let mut k_short_ptr = d_k_short.as_device_ptr().as_raw();
        let mut combo_fast_ptr = d_combo_fast.as_device_ptr().as_raw();
        let mut combo_slow_ptr = d_combo_slow.as_device_ptr().as_raw();
        let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
        let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
        let mut out_ptr = d_out.as_device_ptr().as_raw();

        let mut t_i = signals.cols as i32;
        let mut fast_rows_i = signals.fast_rows as i32;
        let mut fast_stride_i = signals.cols as i32;
        let mut slow_rows_i = signals.slow_rows as i32;
        let mut slow_stride_i = signals.cols as i32;
        let mut combo_count_i = combo_count as i32;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut angle_min_f = asr.angle_deg_min as f32;
        let mut angle_mr_f = asr.angle_deg_mr as f32;
        let mut use_er_filter_i = use_er_filter;
        let mut er_min_f = asr.er_min as f32;
        let mut er_mr_f = asr.er_mr as f32;
        let mut z_entry_f = asr.z_entry as f32;
        let mut z_exit_f = asr.z_exit as f32;
        let mut confirm_bars_i = i32::from(asr.confirm_bars);
        let mut mom_fade_bars_i = i32::from(asr.mom_fade_bars);
        let mut max_trade_bars_i = i32::from(asr.max_trade_bars);
        let mut max_mr_trade_bars_i = i32::from(asr.max_mr_trade_bars);
        let mut out_stride_i = out_stride as i32;

        let block_x = asr_eval_block_x(combo_count);
        let grid_x = ((combo_count as u32) + block_x - 1) / block_x;
        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut fast_sq_ptr as *mut _ as *mut c_void,
                &mut fast_mom_ptr as *mut _ as *mut c_void,
                &mut fast_rows_i as *mut _ as *mut c_void,
                &mut fast_stride_i as *mut _ as *mut c_void,
                &mut slow_ang_ptr as *mut _ as *mut c_void,
                &mut slow_er_ptr as *mut _ as *mut c_void,
                &mut slow_z_ptr as *mut _ as *mut c_void,
                &mut slow_rows_i as *mut _ as *mut c_void,
                &mut slow_stride_i as *mut _ as *mut c_void,
                &mut k_long_ptr as *mut _ as *mut c_void,
                &mut k_short_ptr as *mut _ as *mut c_void,
                &mut combo_fast_ptr as *mut _ as *mut c_void,
                &mut combo_slow_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut angle_min_f as *mut _ as *mut c_void,
                &mut angle_mr_f as *mut _ as *mut c_void,
                &mut use_er_filter_i as *mut _ as *mut c_void,
                &mut er_min_f as *mut _ as *mut c_void,
                &mut er_mr_f as *mut _ as *mut c_void,
                &mut z_entry_f as *mut _ as *mut c_void,
                &mut z_exit_f as *mut _ as *mut c_void,
                &mut confirm_bars_i as *mut _ as *mut c_void,
                &mut mom_fade_bars_i as *mut _ as *mut c_void,
                &mut max_trade_bars_i as *mut _ as *mut c_void,
                &mut max_mr_trade_bars_i as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&kernel, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }

        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut host_out = vec![0.0f32; out_len];
        d_out.copy_to(&mut host_out).map_err(|e| e.to_string())?;

        let mut metrics = Vec::with_capacity(combo_count);
        for i in 0..combo_count {
            let base = i * out_stride;
            metrics.push(decode_non_double_ma_gpu_metrics_row(
                &host_out[base..base + out_stride],
            ));
        }
        Ok(metrics)
    })
}

pub fn eval_asr_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &AsrGpuDeviceSignalMatrices<'_>,
    combos: &AsrGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    asr: &AsrStrategyParams,
) -> Result<Vec<Metrics>, String> {
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
            "ASR GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let close_f32: Vec<f32> = signals.close_f64.iter().map(|v| *v as f32).collect();
    let lr_f32 = compute_log_returns_f32(signals.close_f64);

    let out_stride = NON_DOUBLE_MA_GPU_METRICS_STRIDE;
    let out_len = combo_count
        .checked_mul(out_stride)
        .ok_or_else(|| "ASR GPU output length overflow".to_string())?;
    let fast_mat_len = signals
        .fast_rows
        .checked_mul(signals.cols)
        .ok_or_else(|| "ASR GPU fast matrix overflow".to_string())?;
    let slow_mat_len = signals
        .slow_rows
        .checked_mul(signals.cols)
        .ok_or_else(|| "ASR GPU slow matrix overflow".to_string())?;

    ASR_GPU_RUNTIME.with(|cell| -> Result<Vec<Metrics>, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            let rt = AsrCudaRuntime::new(device_id)?;
            map.insert(device_id, rt);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let d_close = DeviceBuffer::from_slice(&close_f32).map_err(|e| e.to_string())?;
        let d_lr = DeviceBuffer::from_slice(&lr_f32).map_err(|e| e.to_string())?;

        let d_fast_sq_rows =
            DeviceBuffer::from_slice(signals.fast_squeeze_rows).map_err(|e| e.to_string())?;
        let d_fast_mom_rows =
            DeviceBuffer::from_slice(signals.fast_momentum_rows).map_err(|e| e.to_string())?;
        let d_slow_ang_rows =
            DeviceBuffer::from_slice(signals.slow_angle_rows).map_err(|e| e.to_string())?;
        let d_slow_er_rows =
            DeviceBuffer::from_slice(signals.slow_er_rows).map_err(|e| e.to_string())?;
        let d_slow_z_rows =
            DeviceBuffer::from_slice(signals.slow_z_rows).map_err(|e| e.to_string())?;

        let mut d_fast_sq = unsafe { DeviceBuffer::<f32>::uninitialized(fast_mat_len) }
            .map_err(|e| e.to_string())?;
        let mut d_fast_mom = unsafe { DeviceBuffer::<f32>::uninitialized(fast_mat_len) }
            .map_err(|e| e.to_string())?;
        let mut d_slow_ang = unsafe { DeviceBuffer::<f32>::uninitialized(slow_mat_len) }
            .map_err(|e| e.to_string())?;
        let mut d_slow_er = unsafe { DeviceBuffer::<f32>::uninitialized(slow_mat_len) }
            .map_err(|e| e.to_string())?;
        let mut d_slow_z = unsafe { DeviceBuffer::<f32>::uninitialized(slow_mat_len) }
            .map_err(|e| e.to_string())?;

        launch_pack_rows_u64_ptr_f32(
            rt,
            &d_fast_sq_rows,
            signals.fast_rows,
            signals.cols,
            &mut d_fast_sq,
        )?;
        launch_pack_rows_u64_ptr_f32(
            rt,
            &d_fast_mom_rows,
            signals.fast_rows,
            signals.cols,
            &mut d_fast_mom,
        )?;
        launch_pack_rows_u64_ptr_f32(
            rt,
            &d_slow_ang_rows,
            signals.slow_rows,
            signals.cols,
            &mut d_slow_ang,
        )?;
        launch_pack_rows_u64_ptr_f32(
            rt,
            &d_slow_er_rows,
            signals.slow_rows,
            signals.cols,
            &mut d_slow_er,
        )?;
        launch_pack_rows_u64_ptr_f32(
            rt,
            &d_slow_z_rows,
            signals.slow_rows,
            signals.cols,
            &mut d_slow_z,
        )?;

        let d_combo_fast =
            DeviceBuffer::from_slice(combos.fast_row_index).map_err(|e| e.to_string())?;
        let d_combo_slow =
            DeviceBuffer::from_slice(combos.slow_row_index).map_err(|e| e.to_string())?;
        let d_combo_start = DeviceBuffer::from_slice(combos.start_t).map_err(|e| e.to_string())?;
        let d_combo_end = DeviceBuffer::from_slice(combos.end_t).map_err(|e| e.to_string())?;
        let d_out =
            unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }.map_err(|e| e.to_string())?;

        let kernel = rt
            .module
            .get_function("asr_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;

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
        flags |= 1u32 << 4; // signed exposure

        let commission = strategy.effective_commission().clamp(0.0, 0.95) as f32;
        let use_er_filter = i32::from(asr.use_er_filter);

        let mut close_ptr = d_close.as_device_ptr().as_raw();
        let mut lr_ptr = d_lr.as_device_ptr().as_raw();
        let mut fast_sq_ptr = d_fast_sq.as_device_ptr().as_raw();
        let mut fast_mom_ptr = d_fast_mom.as_device_ptr().as_raw();
        let mut slow_ang_ptr = d_slow_ang.as_device_ptr().as_raw();
        let mut slow_er_ptr = d_slow_er.as_device_ptr().as_raw();
        let mut slow_z_ptr = d_slow_z.as_device_ptr().as_raw();
        let mut k_long_ptr = signals.kaufman_long_row;
        let mut k_short_ptr = signals.kaufman_short_row;
        let mut combo_fast_ptr = d_combo_fast.as_device_ptr().as_raw();
        let mut combo_slow_ptr = d_combo_slow.as_device_ptr().as_raw();
        let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
        let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
        let mut out_ptr = d_out.as_device_ptr().as_raw();

        let mut t_i = signals.cols as i32;
        let mut fast_rows_i = signals.fast_rows as i32;
        let mut fast_stride_i = signals.cols as i32;
        let mut slow_rows_i = signals.slow_rows as i32;
        let mut slow_stride_i = signals.cols as i32;
        let mut combo_count_i = combo_count as i32;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut angle_min_f = asr.angle_deg_min as f32;
        let mut angle_mr_f = asr.angle_deg_mr as f32;
        let mut use_er_filter_i = use_er_filter;
        let mut er_min_f = asr.er_min as f32;
        let mut er_mr_f = asr.er_mr as f32;
        let mut z_entry_f = asr.z_entry as f32;
        let mut z_exit_f = asr.z_exit as f32;
        let mut confirm_bars_i = i32::from(asr.confirm_bars);
        let mut mom_fade_bars_i = i32::from(asr.mom_fade_bars);
        let mut max_trade_bars_i = i32::from(asr.max_trade_bars);
        let mut max_mr_trade_bars_i = i32::from(asr.max_mr_trade_bars);
        let mut out_stride_i = out_stride as i32;

        let block_x = asr_eval_block_x(combo_count);
        let grid_x = ((combo_count as u32) + block_x - 1) / block_x;
        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut fast_sq_ptr as *mut _ as *mut c_void,
                &mut fast_mom_ptr as *mut _ as *mut c_void,
                &mut fast_rows_i as *mut _ as *mut c_void,
                &mut fast_stride_i as *mut _ as *mut c_void,
                &mut slow_ang_ptr as *mut _ as *mut c_void,
                &mut slow_er_ptr as *mut _ as *mut c_void,
                &mut slow_z_ptr as *mut _ as *mut c_void,
                &mut slow_rows_i as *mut _ as *mut c_void,
                &mut slow_stride_i as *mut _ as *mut c_void,
                &mut k_long_ptr as *mut _ as *mut c_void,
                &mut k_short_ptr as *mut _ as *mut c_void,
                &mut combo_fast_ptr as *mut _ as *mut c_void,
                &mut combo_slow_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut angle_min_f as *mut _ as *mut c_void,
                &mut angle_mr_f as *mut _ as *mut c_void,
                &mut use_er_filter_i as *mut _ as *mut c_void,
                &mut er_min_f as *mut _ as *mut c_void,
                &mut er_mr_f as *mut _ as *mut c_void,
                &mut z_entry_f as *mut _ as *mut c_void,
                &mut z_exit_f as *mut _ as *mut c_void,
                &mut confirm_bars_i as *mut _ as *mut c_void,
                &mut mom_fade_bars_i as *mut _ as *mut c_void,
                &mut max_trade_bars_i as *mut _ as *mut c_void,
                &mut max_mr_trade_bars_i as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&kernel, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }

        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut host_out = vec![0.0f32; out_len];
        d_out.copy_to(&mut host_out).map_err(|e| e.to_string())?;

        let mut metrics = Vec::with_capacity(combo_count);
        for i in 0..combo_count {
            let base = i * out_stride;
            metrics.push(decode_non_double_ma_gpu_metrics_row(
                &host_out[base..base + out_stride],
            ));
        }
        Ok(metrics)
    })
}
