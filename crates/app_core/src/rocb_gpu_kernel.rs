#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module, DeviceLaunchCaps, KernelLaunchCaps,
};
use crate::rocb::RocbStrategyParams;
use crate::standalone_gpu_runtime::{
    checked_i32, ensure_device_buffer_len, ensure_host_f32_len, slice_signature, CloseSeriesKey,
    ResidentCloseReturns, UploadSignature,
};
use cust::context::{Context, CurrentContext};
use cust::device::Device;
use cust::function::Function;
use cust::memory::CopyDestination;
use cust::stream::{Stream, StreamFlags};
use cust::{memory::DeviceBuffer, module::Module, CudaFlags};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;
use ta_strategies::double_ma::{Metrics, StrategyConfig};

const ROCB_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE;

pub struct RocbGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub close_device_ptr: u64,
    pub cols: usize,
    pub upper_row_major_ptr: u64,
    pub lower_row_major_ptr: u64,
    pub channel_rows: usize,
    pub channel_stride: usize,
    pub atr_row_major_ptr: u64,
    pub atr_rows: usize,
    pub atr_stride: usize,
}

pub struct RocbGpuComboBatch<'a> {
    pub channel_row_index: &'a [i32],
    pub atr_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
}

#[derive(Default)]
struct RocbKernelScratch {
    d_combo_channel: Option<DeviceBuffer<i32>>,
    d_combo_atr: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    host_metrics: Vec<f32>,
    channel_sig: Option<UploadSignature>,
    atr_sig: Option<UploadSignature>,
}

impl RocbKernelScratch {
    fn ensure_len_i32(
        slot: &mut Option<DeviceBuffer<i32>>,
        required: usize,
        stream: &Stream,
    ) -> Result<(), String> {
        ensure_device_buffer_len(slot, required, stream)
    }

    fn ensure_len_f32(
        slot: &mut Option<DeviceBuffer<f32>>,
        required: usize,
        stream: &Stream,
    ) -> Result<(), String> {
        ensure_device_buffer_len(slot, required, stream)
    }

    fn ensure_host_metrics(&mut self, required: usize) {
        ensure_host_f32_len(&mut self.host_metrics, required);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RocbKernelKey {
    ComputeReturns,
    Backtest,
}

struct RocbCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    resident_series: Option<ResidentCloseReturns>,
    scratch: RocbKernelScratch,
    kernel_caps: RefCell<HashMap<RocbKernelKey, KernelLaunchCaps>>,
}

impl RocbCudaRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let launch_caps = detect_device_launch_caps(device);
        let context = Arc::new(Context::new(device).map_err(|e| e.to_string())?);
        let module = load_double_crossover_module(device)?;
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;

        Ok(Self {
            context,
            module,
            stream,
            launch_caps,
            resident_series: None,
            scratch: RocbKernelScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for(&self, key: RocbKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

thread_local! {
    static ROCB_GPU_RUNTIME: RefCell<HashMap<u32, RocbCudaRuntime>> = RefCell::new(HashMap::new());
}

fn validate_inputs(
    signals: &RocbGpuDeviceSignalMatrices<'_>,
    combos: &RocbGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("ROCB GPU kernel requires non-empty series".to_string());
    }
    let n_i32 = checked_i32("ROCB GPU cols", n)?;
    if signals.close_f32.len() != n {
        return Err("ROCB GPU close length mismatch".to_string());
    }
    if signals.upper_row_major_ptr == 0
        || signals.lower_row_major_ptr == 0
        || signals.atr_row_major_ptr == 0
    {
        return Err("ROCB GPU signal pointer is null".to_string());
    }
    if signals.channel_rows == 0
        || signals.channel_stride == 0
        || signals.atr_rows == 0
        || signals.atr_stride == 0
    {
        return Err("ROCB GPU signal matrix shape is invalid".to_string());
    }
    checked_i32("ROCB GPU channel row count", signals.channel_rows)?;
    checked_i32("ROCB GPU channel stride", signals.channel_stride)?;
    checked_i32("ROCB GPU ATR row count", signals.atr_rows)?;
    checked_i32("ROCB GPU ATR stride", signals.atr_stride)?;
    if signals.channel_stride < n || signals.atr_stride < n {
        return Err("ROCB GPU signal stride is smaller than series length".to_string());
    }

    let combos_len = combos.channel_row_index.len();
    checked_i32("ROCB GPU combo count", combos_len)?;
    if combos.atr_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("ROCB GPU combo vector length mismatch".to_string());
    }

    for (idx, ((&channel_row, &atr_row), (&start_t, &end_t))) in combos
        .channel_row_index
        .iter()
        .zip(combos.atr_row_index.iter())
        .zip(combos.start_t.iter().zip(combos.end_t.iter()))
        .enumerate()
    {
        if channel_row < 0 || channel_row >= signals.channel_rows as i32 {
            return Err(format!(
                "ROCB GPU channel_row_index[{idx}] out of bounds: {channel_row} not in [0, {})",
                signals.channel_rows
            ));
        }
        if atr_row < 0 || atr_row >= signals.atr_rows as i32 {
            return Err(format!(
                "ROCB GPU atr_row_index[{idx}] out of bounds: {atr_row} not in [0, {})",
                signals.atr_rows
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("ROCB GPU time window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "ROCB GPU time window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if end_t > n_i32 {
            return Err(format!(
                "ROCB GPU time window[{idx}] end_t {end_t} exceeds cols {n_i32}"
            ));
        }
    }

    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut RocbCudaRuntime,
    close_ptr: u64,
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
    let launch = choose_launch_shape_1d_from_candidates(
        len,
        rt.launch_caps,
        rt.launch_caps_for(RocbKernelKey::ComputeReturns, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr;
    let mut len_i = checked_i32("ROCB GPU close length", len)?;
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

fn ensure_resident_close_lr<'a>(
    rt: &'a mut RocbCudaRuntime,
    signals: &RocbGpuDeviceSignalMatrices<'_>,
) -> Result<&'a ResidentCloseReturns, String> {
    let key = CloseSeriesKey::from_host_and_device(signals.close_f32, signals.close_device_ptr);
    let needs_refresh = rt
        .resident_series
        .as_ref()
        .map(|resident| resident.key != key)
        .unwrap_or(true);
    if needs_refresh {
        let (close_ptr, d_close_owner) = if signals.close_device_ptr != 0 {
            (signals.close_device_ptr, None)
        } else {
            let d_close = DeviceBuffer::from_slice(signals.close_f32).map_err(|e| e.to_string())?;
            let close_ptr = d_close.as_device_ptr().as_raw();
            (close_ptr, Some(d_close))
        };
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(signals.cols) }
            .map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, close_ptr, &mut d_lr, signals.cols)?;
        rt.resident_series = Some(ResidentCloseReturns {
            key,
            close_ptr,
            d_close_owner,
            d_lr,
        });
    }

    rt.resident_series
        .as_ref()
        .ok_or_else(|| "ROCB resident close/lr cache missing".to_string())
}

fn decode_metrics_row(row: &[f32]) -> Metrics {
    decode_non_double_ma_gpu_metrics_row(row)
}

pub fn eval_rocb_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &RocbGpuDeviceSignalMatrices<'_>,
    combos: &RocbGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    rocb: &RocbStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.channel_row_index.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }
    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "ROCB GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_len = combo_count
        .checked_mul(ROCB_METRICS_STRIDE)
        .ok_or_else(|| "ROCB GPU metrics buffer overflow".to_string())?;

    ROCB_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, RocbCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "ROCB GPU runtime missing".to_string())?;
        rt.set_current()?;

        let (close_ptr_raw, lr_ptr_raw) = {
            let resident = ensure_resident_close_lr(rt, signals)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (resident.close_ptr, resident.d_lr.as_device_ptr().as_raw())
        };

        RocbKernelScratch::ensure_len_i32(
            &mut rt.scratch.d_combo_channel,
            combo_count,
            &rt.stream,
        )?;
        RocbKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_atr, combo_count, &rt.stream)?;
        RocbKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        RocbKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        RocbKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;

        let channel_sig = slice_signature(combos.channel_row_index);
        if rt.scratch.channel_sig != Some(channel_sig) {
            let mut combo_channel_slice = rt
                .scratch
                .d_combo_channel
                .as_mut()
                .ok_or_else(|| "ROCB combo channel scratch missing".to_string())?
                .index(..combo_count);
            combo_channel_slice
                .copy_from(combos.channel_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.channel_sig = Some(channel_sig);
        }

        let atr_sig = slice_signature(combos.atr_row_index);
        if rt.scratch.atr_sig != Some(atr_sig) {
            let mut combo_atr_slice = rt
                .scratch
                .d_combo_atr
                .as_mut()
                .ok_or_else(|| "ROCB combo ATR scratch missing".to_string())?
                .index(..combo_count);
            combo_atr_slice
                .copy_from(combos.atr_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.atr_sig = Some(atr_sig);
        }

        let mut combo_start_slice = rt
            .scratch
            .d_combo_start
            .as_mut()
            .ok_or_else(|| "ROCB combo start scratch missing".to_string())?
            .index(..combo_count);
        combo_start_slice
            .copy_from(combos.start_t)
            .map_err(|e| e.to_string())?;

        let mut combo_end_slice = rt
            .scratch
            .d_combo_end
            .as_mut()
            .ok_or_else(|| "ROCB combo end scratch missing".to_string())?
            .index(..combo_count);
        combo_end_slice
            .copy_from(combos.end_t)
            .map_err(|e| e.to_string())?;

        let kernel = rt
            .module
            .get_function("rocb_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(RocbKernelKey::Backtest, &kernel),
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

        let mut close_ptr = close_ptr_raw;
        let mut lr_ptr = lr_ptr_raw;
        let mut upper_ptr = signals.upper_row_major_ptr;
        let mut lower_ptr = signals.lower_row_major_ptr;
        let mut channel_rows_i = checked_i32("ROCB GPU channel row count", signals.channel_rows)?;
        let mut channel_stride_i = checked_i32("ROCB GPU channel stride", signals.channel_stride)?;
        let mut atr_ptr = signals.atr_row_major_ptr;
        let mut atr_rows_i = checked_i32("ROCB GPU ATR row count", signals.atr_rows)?;
        let mut atr_stride_i = checked_i32("ROCB GPU ATR stride", signals.atr_stride)?;
        let mut combo_channel_ptr = rt
            .scratch
            .d_combo_channel
            .as_ref()
            .ok_or_else(|| "ROCB combo channel scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_atr_ptr = rt
            .scratch
            .d_combo_atr
            .as_ref()
            .ok_or_else(|| "ROCB combo ATR scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "ROCB combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "ROCB combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_count_i = checked_i32("ROCB GPU combo count", combo_count)?;
        let mut t_i = checked_i32("ROCB GPU cols", signals.cols)?;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut atr_stop_mult_f = rocb.atr_stop_mult as f32;
        let mut max_trade_bars_i = i32::from(rocb.max_trade_bars.max(1));
        let mut hurdle_tau_f = strategy.hurdle_tau.unwrap_or(0.0) as f32;
        let mut out_stride_i = checked_i32("ROCB GPU metrics stride", ROCB_METRICS_STRIDE)?;
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "ROCB output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut upper_ptr as *mut _ as *mut c_void,
                &mut lower_ptr as *mut _ as *mut c_void,
                &mut channel_rows_i as *mut _ as *mut c_void,
                &mut channel_stride_i as *mut _ as *mut c_void,
                &mut atr_ptr as *mut _ as *mut c_void,
                &mut atr_rows_i as *mut _ as *mut c_void,
                &mut atr_stride_i as *mut _ as *mut c_void,
                &mut combo_channel_ptr as *mut _ as *mut c_void,
                &mut combo_atr_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut atr_stop_mult_f as *mut _ as *mut c_void,
                &mut max_trade_bars_i as *mut _ as *mut c_void,
                &mut hurdle_tau_f as *mut _ as *mut c_void,
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
        rt.scratch.ensure_host_metrics(out_len);
        let d_out_prefix = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "ROCB output scratch missing".to_string())?
            .index(..out_len);
        d_out_prefix
            .copy_to(rt.scratch.host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out: Vec<Metrics> = Vec::with_capacity(combo_count);
        for row in rt.scratch.host_metrics[..out_len].chunks_exact(ROCB_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}

#[cfg(test)]
mod tests {
    use super::{validate_inputs, RocbGpuComboBatch, RocbGpuDeviceSignalMatrices};

    #[test]
    fn validate_inputs_rejects_negative_start_t() {
        let close = vec![1.0f32; 8];
        let signals = RocbGpuDeviceSignalMatrices {
            close_f32: &close,
            close_device_ptr: 5,
            cols: close.len(),
            upper_row_major_ptr: 11,
            lower_row_major_ptr: 12,
            channel_rows: 2,
            channel_stride: close.len(),
            atr_row_major_ptr: 13,
            atr_rows: 2,
            atr_stride: close.len(),
        };
        let combos = RocbGpuComboBatch {
            channel_row_index: &[0],
            atr_row_index: &[0],
            start_t: &[-1],
            end_t: &[6],
        };

        let err = validate_inputs(&signals, &combos).expect_err("negative start_t");
        assert!(err.contains("non-negative"));
    }

    #[test]
    fn validate_inputs_rejects_row_index_out_of_bounds() {
        let close = vec![1.0f32; 8];
        let signals = RocbGpuDeviceSignalMatrices {
            close_f32: &close,
            close_device_ptr: 5,
            cols: close.len(),
            upper_row_major_ptr: 11,
            lower_row_major_ptr: 12,
            channel_rows: 2,
            channel_stride: close.len(),
            atr_row_major_ptr: 13,
            atr_rows: 2,
            atr_stride: close.len(),
        };
        let combos = RocbGpuComboBatch {
            channel_row_index: &[2],
            atr_row_index: &[0],
            start_t: &[1],
            end_t: &[6],
        };

        let err = validate_inputs(&signals, &combos).expect_err("row index out of bounds");
        assert!(err.contains("channel_row_index"));
    }

    #[test]
    fn validate_inputs_rejects_end_t_past_series() {
        let close = vec![1.0f32; 8];
        let signals = RocbGpuDeviceSignalMatrices {
            close_f32: &close,
            close_device_ptr: 5,
            cols: close.len(),
            upper_row_major_ptr: 11,
            lower_row_major_ptr: 12,
            channel_rows: 2,
            channel_stride: close.len(),
            atr_row_major_ptr: 13,
            atr_rows: 2,
            atr_stride: close.len(),
        };
        let combos = RocbGpuComboBatch {
            channel_row_index: &[0],
            atr_row_index: &[0],
            start_t: &[1],
            end_t: &[9],
        };

        let err = validate_inputs(&signals, &combos).expect_err("end_t too large");
        assert!(err.contains("end_t"));
    }
}
