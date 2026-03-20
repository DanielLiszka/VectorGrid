#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module,
};
use crate::mkcs::MkcsStrategyParams;
use cust::context::{Context, CurrentContext};
use cust::device::Device;
use cust::memory::CopyDestination;
use cust::stream::{Stream, StreamFlags};
use cust::{memory::DeviceBuffer, module::Module, CudaFlags};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;
use ta_strategies::double_ma::{Metrics, StrategyConfig};

const MKCS_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_METRICS_STRIDE;

pub struct MkcsGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub cols: usize,
    pub stddev_row_major_ptr: u64,
    pub stddev_rows: usize,
    pub stddev_stride: usize,
    pub upper_row_major_ptr: u64,
    pub lower_row_major_ptr: u64,
    pub breakout_rows: usize,
    pub breakout_stride: usize,
}

pub struct MkcsGpuComboBatch<'a> {
    pub stddev_row_index: &'a [i32],
    pub breakout_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
}

struct MkcsCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
}

impl MkcsCudaRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let context = Arc::new(Context::new(device).map_err(|e| e.to_string())?);

        let module = load_double_crossover_module(device)?;

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
    static MKCS_GPU_RUNTIME: RefCell<HashMap<u32, MkcsCudaRuntime>> = RefCell::new(HashMap::new());
}

fn validate_inputs(
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("MKCS GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f32.len() != n {
        return Err("MKCS GPU close length mismatch".to_string());
    }
    if signals.stddev_row_major_ptr == 0
        || signals.upper_row_major_ptr == 0
        || signals.lower_row_major_ptr == 0
    {
        return Err("MKCS GPU signal pointer is null".to_string());
    }
    if signals.stddev_rows == 0
        || signals.stddev_stride == 0
        || signals.breakout_rows == 0
        || signals.breakout_stride == 0
    {
        return Err("MKCS GPU signal matrix shape is invalid".to_string());
    }

    let combos_len = combos.stddev_row_index.len();
    if combos.breakout_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("MKCS GPU combo vector length mismatch".to_string());
    }
    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut MkcsCudaRuntime,
    device_id: u32,
    d_close: &DeviceBuffer<f32>,
    d_lr: &mut DeviceBuffer<f32>,
    len: usize,
) -> Result<(), String> {
    if len == 0 {
        return Ok(());
    }

    let kernel = rt
        .module
        .get_function("compute_log_returns_f32")
        .map_err(|e| e.to_string())?;
    let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
    let device_caps = detect_device_launch_caps(device);
    let launch = choose_launch_shape_1d_from_candidates(
        len,
        device_caps,
        detect_kernel_launch_caps(&kernel, device_caps),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = d_close.as_device_ptr().as_raw();
    let mut len_i =
        i32::try_from(len).map_err(|_| "MKCS GPU close length exceeds i32 range".to_string())?;
    let mut lr_ptr = d_lr.as_device_ptr().as_raw();

    unsafe {
        let args: &mut [*mut c_void] = &mut [
            &mut close_ptr as *mut _ as *mut c_void,
            &mut len_i as *mut _ as *mut c_void,
            &mut lr_ptr as *mut _ as *mut c_void,
        ];
        rt.stream
            .launch(
                &kernel,
                (launch.grid_x.max(1), 1, 1),
                (launch.block_x.max(1), 1, 1),
                0,
                args,
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn decode_metrics_row(row: &[f32]) -> Metrics {
    decode_non_double_ma_gpu_metrics_row(row)
}

pub fn eval_mkcs_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    mkcs: &MkcsStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.stddev_row_index.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }
    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "MKCS GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_len = combo_count
        .checked_mul(MKCS_METRICS_STRIDE)
        .ok_or_else(|| "MKCS GPU metrics buffer overflow".to_string())?;

    MKCS_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, MkcsCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "MKCS GPU runtime missing".to_string())?;
        rt.set_current()?;

        let d_close = DeviceBuffer::from_slice(signals.close_f32).map_err(|e| e.to_string())?;
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(signals.cols) }
            .map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, device_id, &d_close, &mut d_lr, signals.cols)?;

        let d_combo_stddev =
            DeviceBuffer::from_slice(combos.stddev_row_index).map_err(|e| e.to_string())?;
        let d_combo_breakout =
            DeviceBuffer::from_slice(combos.breakout_row_index).map_err(|e| e.to_string())?;
        let d_combo_start = DeviceBuffer::from_slice(combos.start_t).map_err(|e| e.to_string())?;
        let d_combo_end = DeviceBuffer::from_slice(combos.end_t).map_err(|e| e.to_string())?;
        let d_out =
            unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }.map_err(|e| e.to_string())?;

        let kernel = rt
            .module
            .get_function("mkcs_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let device_caps = detect_device_launch_caps(device);
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            device_caps,
            detect_kernel_launch_caps(&kernel, device_caps),
            &[32u32, 64, 128, 256],
        );

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
        let mut stddev_ptr = signals.stddev_row_major_ptr;
        let mut stddev_rows_i = signals.stddev_rows as i32;
        let mut stddev_stride_i = signals.stddev_stride as i32;
        let mut upper_ptr = signals.upper_row_major_ptr;
        let mut lower_ptr = signals.lower_row_major_ptr;
        let mut breakout_rows_i = signals.breakout_rows as i32;
        let mut breakout_stride_i = signals.breakout_stride as i32;
        let mut combo_stddev_ptr = d_combo_stddev.as_device_ptr().as_raw();
        let mut combo_breakout_ptr = d_combo_breakout.as_device_ptr().as_raw();
        let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
        let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
        let mut combo_count_i = combo_count as i32;
        let mut t_i = signals.cols as i32;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut compression_threshold_f = mkcs.compression_threshold as f32;
        let mut reexpansion_threshold_f = mkcs.reexpansion_threshold as f32;
        let mut out_stride_i = MKCS_METRICS_STRIDE as i32;
        let mut out_ptr = d_out.as_device_ptr().as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut stddev_ptr as *mut _ as *mut c_void,
                &mut stddev_rows_i as *mut _ as *mut c_void,
                &mut stddev_stride_i as *mut _ as *mut c_void,
                &mut upper_ptr as *mut _ as *mut c_void,
                &mut lower_ptr as *mut _ as *mut c_void,
                &mut breakout_rows_i as *mut _ as *mut c_void,
                &mut breakout_stride_i as *mut _ as *mut c_void,
                &mut combo_stddev_ptr as *mut _ as *mut c_void,
                &mut combo_breakout_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut compression_threshold_f as *mut _ as *mut c_void,
                &mut reexpansion_threshold_f as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(
                    &kernel,
                    (launch.grid_x.max(1), 1, 1),
                    (launch.block_x.max(1), 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        rt.stream.synchronize().map_err(|e| e.to_string())?;
        let mut host_metrics = vec![0.0f32; out_len];
        d_out
            .copy_to(host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out: Vec<Metrics> = Vec::with_capacity(combo_count);
        for row in host_metrics.chunks_exact(MKCS_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}
