#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
};
use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_METRICS_STRIDE,
};
use crate::vcet::VcetStrategyParams;
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

const VCET_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_METRICS_STRIDE;

pub struct VcetGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub cols: usize,
    pub vidya_rows_ptr: &'a [u64],
    pub ce_long_rows_ptr: &'a [u64],
    pub ce_short_rows_ptr: &'a [u64],
    pub vidya_rows: usize,
    pub ce_rows: usize,
}

pub struct VcetGpuComboBatch<'a> {
    pub vidya_row_index: &'a [i32],
    pub ce_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
}

struct VcetCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
}

impl VcetCudaRuntime {
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
    static VCET_GPU_RUNTIME: RefCell<HashMap<u32, VcetCudaRuntime>> = RefCell::new(HashMap::new());
}

fn validate_inputs(
    signals: &VcetGpuDeviceSignalMatrices<'_>,
    combos: &VcetGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("VCET GPU kernel requires non-empty series".to_string());
    }
    if signals.close_f32.len() != n {
        return Err("VCET GPU close length mismatch".to_string());
    }
    if signals.vidya_rows == 0
        || signals.ce_rows == 0
        || signals.vidya_rows_ptr.len() != signals.vidya_rows
        || signals.ce_long_rows_ptr.len() != signals.ce_rows
        || signals.ce_short_rows_ptr.len() != signals.ce_rows
    {
        return Err("VCET GPU row-pointer vector length mismatch".to_string());
    }

    let combos_len = combos.vidya_row_index.len();
    if combos.ce_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("VCET GPU combo vector length mismatch".to_string());
    }
    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut VcetCudaRuntime,
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
        i32::try_from(len).map_err(|_| "VCET GPU close length exceeds i32 range".to_string())?;
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

pub fn eval_vcet_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &VcetGpuDeviceSignalMatrices<'_>,
    combos: &VcetGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vcet: &VcetStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.vidya_row_index.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }
    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "VCET GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_len = combo_count
        .checked_mul(VCET_METRICS_STRIDE)
        .ok_or_else(|| "VCET GPU metrics buffer overflow".to_string())?;

    VCET_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, VcetCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "VCET GPU runtime missing".to_string())?;
        rt.set_current()?;

        let d_close = DeviceBuffer::from_slice(signals.close_f32).map_err(|e| e.to_string())?;
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(signals.cols) }
            .map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, device_id, &d_close, &mut d_lr, signals.cols)?;

        let d_vidya_rows =
            DeviceBuffer::from_slice(signals.vidya_rows_ptr).map_err(|e| e.to_string())?;
        let d_ce_long_rows =
            DeviceBuffer::from_slice(signals.ce_long_rows_ptr).map_err(|e| e.to_string())?;
        let d_ce_short_rows =
            DeviceBuffer::from_slice(signals.ce_short_rows_ptr).map_err(|e| e.to_string())?;
        let d_combo_vidya =
            DeviceBuffer::from_slice(combos.vidya_row_index).map_err(|e| e.to_string())?;
        let d_combo_ce =
            DeviceBuffer::from_slice(combos.ce_row_index).map_err(|e| e.to_string())?;
        let d_combo_start = DeviceBuffer::from_slice(combos.start_t).map_err(|e| e.to_string())?;
        let d_combo_end = DeviceBuffer::from_slice(combos.end_t).map_err(|e| e.to_string())?;
        let d_out =
            unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }.map_err(|e| e.to_string())?;

        let kernel = rt
            .module
            .get_function("vcet_backtest_metrics_f32")
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
        let mut t_i = signals.cols as i32;
        let mut vidya_rows_ptr = d_vidya_rows.as_device_ptr().as_raw();
        let mut ce_long_rows_ptr = d_ce_long_rows.as_device_ptr().as_raw();
        let mut ce_short_rows_ptr = d_ce_short_rows.as_device_ptr().as_raw();
        let mut vidya_rows_i = signals.vidya_rows as i32;
        let mut ce_rows_i = signals.ce_rows as i32;
        let mut combo_vidya_ptr = d_combo_vidya.as_device_ptr().as_raw();
        let mut combo_ce_ptr = d_combo_ce.as_device_ptr().as_raw();
        let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
        let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
        let mut combo_count_i = combo_count as i32;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut require_vidya_slope_i = if vcet.require_vidya_slope { 1i32 } else { 0i32 };
        let mut max_trade_bars_i = i32::from(vcet.max_trade_bars.max(1));
        let mut out_stride_i = VCET_METRICS_STRIDE as i32;
        let mut out_ptr = d_out.as_device_ptr().as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut vidya_rows_ptr as *mut _ as *mut c_void,
                &mut ce_long_rows_ptr as *mut _ as *mut c_void,
                &mut ce_short_rows_ptr as *mut _ as *mut c_void,
                &mut vidya_rows_i as *mut _ as *mut c_void,
                &mut ce_rows_i as *mut _ as *mut c_void,
                &mut combo_vidya_ptr as *mut _ as *mut c_void,
                &mut combo_ce_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut require_vidya_slope_i as *mut _ as *mut c_void,
                &mut max_trade_bars_i as *mut _ as *mut c_void,
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
        for row in host_metrics.chunks_exact(VCET_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}
