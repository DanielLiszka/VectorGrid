#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module, DeviceLaunchCaps, KernelLaunchCaps,
};
use crate::kvdf::{normalized_kvdf_dti_threshold, KvdfStrategyParams};
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

const KVDF_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE;
const FLAG_LONG_ONLY: u32 = 1 << 0;
const FLAG_DISABLE_FLIP: u32 = 1 << 1;
const FLAG_TRADE_ON_NEXT_BAR: u32 = 1 << 2;
const FLAG_ENABLE_STRATEGY_LOGIC: u32 = 1 << 4;

pub struct KvdfGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],

    pub close_device_ptr: u64,
    pub cols: usize,
    pub dti_rows: usize,
    pub dti_stride: usize,

    pub keltner_upper_row_major_ptr: u64,
    pub keltner_middle_row_major_ptr: u64,
    pub keltner_lower_row_major_ptr: u64,
    pub vwap_row_major_ptr: u64,
    pub dti_row_major_ptr: u64,
    pub slow_rows: usize,
    pub slow_stride: usize,
    pub devstop_long_row_major_ptr: u64,
    pub devstop_short_row_major_ptr: u64,
    pub fvg_upper_ts_row_major_ptr: u64,
    pub fvg_lower_ts_row_major_ptr: u64,
    pub fast_rows: usize,
    pub fast_stride: usize,
}

pub struct KvdfGpuComboBatch<'a> {
    pub fast_row_index: &'a [i32],
    pub slow_row_index: &'a [i32],
    pub dti_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
}

#[derive(Default)]
struct KvdfKernelScratch {
    d_combo_fast: Option<DeviceBuffer<i32>>,
    d_combo_slow: Option<DeviceBuffer<i32>>,
    d_combo_dti: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    host_metrics: Vec<f32>,
    fast_sig: Option<UploadSignature>,
    slow_sig: Option<UploadSignature>,
    dti_sig: Option<UploadSignature>,
    start_sig: Option<UploadSignature>,
    end_sig: Option<UploadSignature>,
}

impl KvdfKernelScratch {
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
enum KvdfKernelKey {
    ComputeReturns,
    Backtest,
}

struct KvdfCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    resident_series: Option<ResidentCloseReturns>,
    scratch: KvdfKernelScratch,
    kernel_caps: RefCell<HashMap<KvdfKernelKey, KernelLaunchCaps>>,
}

impl KvdfCudaRuntime {
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
            scratch: KvdfKernelScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for(&self, key: KvdfKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

thread_local! {
    static KVDF_GPU_RUNTIME: RefCell<HashMap<u32, KvdfCudaRuntime>> = RefCell::new(HashMap::new());
}

fn validate_inputs(
    signals: &KvdfGpuDeviceSignalMatrices<'_>,
    combos: &KvdfGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("KVDF GPU kernel requires non-empty series".to_string());
    }
    let n_i32 = checked_i32("KVDF GPU cols", n)?;
    if signals.close_f32.len() != n {
        return Err("KVDF GPU close length mismatch".to_string());
    }
    if signals.dti_rows == 0
        || signals.dti_stride == 0
        || signals.fast_rows == 0
        || signals.fast_stride == 0
        || signals.slow_rows == 0
        || signals.slow_stride == 0
    {
        return Err("KVDF GPU signal matrix shape is invalid".to_string());
    }
    checked_i32("KVDF GPU DTI row count", signals.dti_rows)?;
    checked_i32("KVDF GPU DTI stride", signals.dti_stride)?;
    checked_i32("KVDF GPU slow row count", signals.slow_rows)?;
    checked_i32("KVDF GPU slow stride", signals.slow_stride)?;
    checked_i32("KVDF GPU fast row count", signals.fast_rows)?;
    checked_i32("KVDF GPU fast stride", signals.fast_stride)?;
    if signals.dti_stride < n || signals.slow_stride < n || signals.fast_stride < n {
        return Err("KVDF GPU signal stride is smaller than series length".to_string());
    }
    if signals.keltner_upper_row_major_ptr == 0
        || signals.keltner_middle_row_major_ptr == 0
        || signals.keltner_lower_row_major_ptr == 0
        || signals.vwap_row_major_ptr == 0
        || signals.dti_row_major_ptr == 0
        || signals.devstop_long_row_major_ptr == 0
        || signals.devstop_short_row_major_ptr == 0
        || signals.fvg_upper_ts_row_major_ptr == 0
        || signals.fvg_lower_ts_row_major_ptr == 0
    {
        return Err("KVDF GPU signal pointer is null".to_string());
    }

    let combos_len = combos.fast_row_index.len();
    checked_i32("KVDF GPU combo count", combos_len)?;
    if combos.slow_row_index.len() != combos_len
        || combos.dti_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("KVDF GPU combo vector length mismatch".to_string());
    }

    for idx in 0..combos_len {
        let fast_row = combos.fast_row_index[idx];
        let slow_row = combos.slow_row_index[idx];
        let dti_row = combos.dti_row_index[idx];
        let start_t = combos.start_t[idx];
        let end_t = combos.end_t[idx];

        if fast_row < 0 || fast_row >= signals.fast_rows as i32 {
            return Err(format!(
                "KVDF GPU fast_row_index[{idx}] out of bounds: {fast_row} not in [0, {})",
                signals.fast_rows
            ));
        }
        if slow_row < 0 || slow_row >= signals.slow_rows as i32 {
            return Err(format!(
                "KVDF GPU slow_row_index[{idx}] out of bounds: {slow_row} not in [0, {})",
                signals.slow_rows
            ));
        }
        if dti_row < 0 || dti_row >= signals.dti_rows as i32 {
            return Err(format!(
                "KVDF GPU dti_row_index[{idx}] out of bounds: {dti_row} not in [0, {})",
                signals.dti_rows
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("KVDF GPU time window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "KVDF GPU time window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if end_t > n_i32 {
            return Err(format!(
                "KVDF GPU time window[{idx}] end_t {end_t} exceeds cols {n_i32}"
            ));
        }
    }

    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut KvdfCudaRuntime,
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
        rt.launch_caps_for(KvdfKernelKey::ComputeReturns, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr;
    let mut len_i = checked_i32("KVDF GPU close length", len)?;
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
    rt: &'a mut KvdfCudaRuntime,
    signals: &KvdfGpuDeviceSignalMatrices<'_>,
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
        .ok_or_else(|| "KVDF resident close/lr cache missing".to_string())
}

#[inline]
fn kvdf_kernel_flags(strategy: &StrategyConfig) -> u32 {
    let mut flags = FLAG_ENABLE_STRATEGY_LOGIC;
    if strategy.long_only {
        flags |= FLAG_LONG_ONLY;
    }
    if !strategy.allow_flip {
        flags |= FLAG_DISABLE_FLIP;
    }
    if strategy.trade_on_next_bar {
        flags |= FLAG_TRADE_ON_NEXT_BAR;
    }
    flags
}

fn decode_metrics_row(row: &[f32]) -> Metrics {
    decode_non_double_ma_gpu_metrics_row(row)
}

pub fn eval_kvdf_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &KvdfGpuDeviceSignalMatrices<'_>,
    combos: &KvdfGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    kvdf: &KvdfStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.start_t.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }
    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "KVDF GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_len = combo_count
        .checked_mul(KVDF_METRICS_STRIDE)
        .ok_or_else(|| "KVDF GPU metrics buffer overflow".to_string())?;

    KVDF_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, KvdfCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "KVDF GPU runtime missing".to_string())?;
        rt.set_current()?;

        let (close_ptr_raw, lr_ptr_raw) = {
            let resident = ensure_resident_close_lr(rt, signals)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (resident.close_ptr, resident.d_lr.as_device_ptr().as_raw())
        };

        KvdfKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_fast, combo_count, &rt.stream)?;
        KvdfKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_slow, combo_count, &rt.stream)?;
        KvdfKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_dti, combo_count, &rt.stream)?;
        KvdfKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        KvdfKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        KvdfKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;

        let fast_sig = slice_signature(combos.fast_row_index);
        if rt.scratch.fast_sig != Some(fast_sig) {
            let mut combo_fast_slice = rt
                .scratch
                .d_combo_fast
                .as_mut()
                .ok_or_else(|| "KVDF combo fast scratch missing".to_string())?
                .index(..combo_count);
            combo_fast_slice
                .copy_from(combos.fast_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.fast_sig = Some(fast_sig);
        }

        let slow_sig = slice_signature(combos.slow_row_index);
        if rt.scratch.slow_sig != Some(slow_sig) {
            let mut combo_slow_slice = rt
                .scratch
                .d_combo_slow
                .as_mut()
                .ok_or_else(|| "KVDF combo slow scratch missing".to_string())?
                .index(..combo_count);
            combo_slow_slice
                .copy_from(combos.slow_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.slow_sig = Some(slow_sig);
        }

        let dti_sig = slice_signature(combos.dti_row_index);
        if rt.scratch.dti_sig != Some(dti_sig) {
            let mut combo_dti_slice = rt
                .scratch
                .d_combo_dti
                .as_mut()
                .ok_or_else(|| "KVDF combo DTI scratch missing".to_string())?
                .index(..combo_count);
            combo_dti_slice
                .copy_from(combos.dti_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.dti_sig = Some(dti_sig);
        }

        let start_sig = slice_signature(combos.start_t);
        if rt.scratch.start_sig != Some(start_sig) {
            let mut combo_start_slice = rt
                .scratch
                .d_combo_start
                .as_mut()
                .ok_or_else(|| "KVDF combo start scratch missing".to_string())?
                .index(..combo_count);
            combo_start_slice
                .copy_from(combos.start_t)
                .map_err(|e| e.to_string())?;
            rt.scratch.start_sig = Some(start_sig);
        }

        let end_sig = slice_signature(combos.end_t);
        if rt.scratch.end_sig != Some(end_sig) {
            let mut combo_end_slice = rt
                .scratch
                .d_combo_end
                .as_mut()
                .ok_or_else(|| "KVDF combo end scratch missing".to_string())?
                .index(..combo_count);
            combo_end_slice
                .copy_from(combos.end_t)
                .map_err(|e| e.to_string())?;
            rt.scratch.end_sig = Some(end_sig);
        }

        let kernel = rt
            .module
            .get_function("kvdf_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(KvdfKernelKey::Backtest, &kernel),
            &[32u32, 64, 128, 256],
        );

        let mut close_ptr = close_ptr_raw;
        let mut lr_ptr = lr_ptr_raw;
        let mut t_i = checked_i32("KVDF GPU cols", signals.cols)?;
        let mut keltner_upper_ptr = signals.keltner_upper_row_major_ptr;
        let mut keltner_middle_ptr = signals.keltner_middle_row_major_ptr;
        let mut keltner_lower_ptr = signals.keltner_lower_row_major_ptr;
        let mut slow_rows_i = checked_i32("KVDF GPU slow row count", signals.slow_rows)?;
        let mut slow_stride_i = checked_i32("KVDF GPU slow stride", signals.slow_stride)?;
        let mut vwap_ptr = signals.vwap_row_major_ptr;
        let mut dti_ptr = signals.dti_row_major_ptr;
        let mut dti_rows_i = checked_i32("KVDF GPU DTI row count", signals.dti_rows)?;
        let mut dti_stride_i = checked_i32("KVDF GPU DTI stride", signals.dti_stride)?;
        let mut devstop_long_ptr = signals.devstop_long_row_major_ptr;
        let mut devstop_short_ptr = signals.devstop_short_row_major_ptr;
        let mut fvg_upper_ptr = signals.fvg_upper_ts_row_major_ptr;
        let mut fvg_lower_ptr = signals.fvg_lower_ts_row_major_ptr;
        let mut fast_rows_i = checked_i32("KVDF GPU fast row count", signals.fast_rows)?;
        let mut fast_stride_i = checked_i32("KVDF GPU fast stride", signals.fast_stride)?;
        let mut combo_fast_ptr = rt
            .scratch
            .d_combo_fast
            .as_ref()
            .ok_or_else(|| "KVDF combo fast scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_slow_ptr = rt
            .scratch
            .d_combo_slow
            .as_ref()
            .ok_or_else(|| "KVDF combo slow scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_dti_ptr = rt
            .scratch
            .d_combo_dti
            .as_ref()
            .ok_or_else(|| "KVDF combo DTI scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "KVDF combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "KVDF combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_count_i = checked_i32("KVDF GPU combo count", combo_count)?;
        let mut flags_u = kvdf_kernel_flags(strategy);
        let mut commission_f = strategy.effective_commission().clamp(0.0, 0.95) as f32;
        let mut dti_threshold_f = normalized_kvdf_dti_threshold(kvdf) as f32;
        let mut hurdle_tau_f = strategy.hurdle_tau.unwrap_or(0.0) as f32;
        let mut out_stride_i = checked_i32("KVDF GPU metrics stride", KVDF_METRICS_STRIDE)?;
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "KVDF output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut keltner_upper_ptr as *mut _ as *mut c_void,
                &mut keltner_middle_ptr as *mut _ as *mut c_void,
                &mut keltner_lower_ptr as *mut _ as *mut c_void,
                &mut slow_rows_i as *mut _ as *mut c_void,
                &mut slow_stride_i as *mut _ as *mut c_void,
                &mut vwap_ptr as *mut _ as *mut c_void,
                &mut dti_ptr as *mut _ as *mut c_void,
                &mut dti_rows_i as *mut _ as *mut c_void,
                &mut dti_stride_i as *mut _ as *mut c_void,
                &mut devstop_long_ptr as *mut _ as *mut c_void,
                &mut devstop_short_ptr as *mut _ as *mut c_void,
                &mut fvg_upper_ptr as *mut _ as *mut c_void,
                &mut fvg_lower_ptr as *mut _ as *mut c_void,
                &mut fast_rows_i as *mut _ as *mut c_void,
                &mut fast_stride_i as *mut _ as *mut c_void,
                &mut combo_fast_ptr as *mut _ as *mut c_void,
                &mut combo_slow_ptr as *mut _ as *mut c_void,
                &mut combo_dti_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut dti_threshold_f as *mut _ as *mut c_void,
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
            .ok_or_else(|| "KVDF output scratch missing".to_string())?
            .index(..out_len);
        d_out_prefix
            .copy_to(rt.scratch.host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out = Vec::with_capacity(combo_count);
        for row in rt.scratch.host_metrics[..out_len].chunks_exact(KVDF_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_signals_with_close_ptr(
        close: &[f32],
        close_device_ptr: u64,
    ) -> KvdfGpuDeviceSignalMatrices<'_> {
        KvdfGpuDeviceSignalMatrices {
            close_f32: close,
            close_device_ptr,
            cols: close.len(),
            dti_rows: 2,
            dti_stride: close.len(),
            keltner_upper_row_major_ptr: 11,
            keltner_middle_row_major_ptr: 12,
            keltner_lower_row_major_ptr: 13,
            vwap_row_major_ptr: 14,
            dti_row_major_ptr: 15,
            slow_rows: 2,
            slow_stride: close.len(),
            devstop_long_row_major_ptr: 16,
            devstop_short_row_major_ptr: 17,
            fvg_upper_ts_row_major_ptr: 18,
            fvg_lower_ts_row_major_ptr: 19,
            fast_rows: 2,
            fast_stride: close.len(),
        }
    }

    fn sample_combo_batch() -> KvdfGpuComboBatch<'static> {
        KvdfGpuComboBatch {
            fast_row_index: &[0],
            slow_row_index: &[0],
            dti_row_index: &[0],
            start_t: &[1],
            end_t: &[6],
        }
    }

    #[test]
    fn validate_inputs_rejects_negative_start_t() {
        let close = vec![1.0f32; 8];
        let signals = sample_signals_with_close_ptr(&close, 7);
        let combos = KvdfGpuComboBatch {
            fast_row_index: &[0],
            slow_row_index: &[0],
            dti_row_index: &[0],
            start_t: &[-1],
            end_t: &[6],
        };

        let err = validate_inputs(&signals, &combos).expect_err("negative start_t");
        assert!(err.contains("non-negative"));
    }

    #[test]
    fn validate_inputs_rejects_stride_smaller_than_cols() {
        let close = vec![1.0f32; 8];
        let mut signals = sample_signals_with_close_ptr(&close, 7);
        signals.dti_stride = 7;
        let combos = KvdfGpuComboBatch {
            fast_row_index: &[0],
            slow_row_index: &[0],
            dti_row_index: &[0],
            start_t: &[1],
            end_t: &[6],
        };

        let err = validate_inputs(&signals, &combos).expect_err("stride smaller than cols");
        assert!(err.contains("stride"));
    }

    #[test]
    fn validate_inputs_rejects_row_index_out_of_bounds() {
        let close = vec![1.0f32; 8];
        let signals = sample_signals_with_close_ptr(&close, 7);
        let combos = KvdfGpuComboBatch {
            fast_row_index: &[2],
            slow_row_index: &[0],
            dti_row_index: &[0],
            start_t: &[1],
            end_t: &[6],
        };

        let err = validate_inputs(&signals, &combos).expect_err("row index out of bounds");
        assert!(err.contains("fast_row_index"));
    }

    #[test]
    fn validate_inputs_allows_close_device_ptr_zero_host_upload_fallback() {
        let close = vec![1.0f32; 8];
        let signals = sample_signals_with_close_ptr(&close, 0);
        let combos = sample_combo_batch();

        validate_inputs(&signals, &combos).expect("host-upload fallback should validate");
    }

    #[test]
    fn validate_inputs_rejects_null_signal_pointer() {
        let close = vec![1.0f32; 8];
        let mut signals = sample_signals_with_close_ptr(&close, 7);
        signals.fvg_lower_ts_row_major_ptr = 0;
        let combos = sample_combo_batch();

        let err = validate_inputs(&signals, &combos).expect_err("null signal pointer");
        assert!(err.contains("signal pointer is null"));
    }

    #[test]
    fn eval_entry_rejects_overlay_stop_loss_and_take_profit_before_cuda_runtime_work() {
        let close = vec![1.0f32; 8];
        let signals = sample_signals_with_close_ptr(&close, 0);
        let combos = sample_combo_batch();
        let strategy_cases = [
            StrategyConfig {
                cooldown_bars: 1,
                ..StrategyConfig::default()
            },
            StrategyConfig {
                stop_loss_pct: Some(0.02),
                ..StrategyConfig::default()
            },
            StrategyConfig {
                take_profit_pct: Some(0.02),
                ..StrategyConfig::default()
            },
        ];

        for strategy in strategy_cases {
            let err = eval_kvdf_metrics_gpu_kernel_device_rows(
                0,
                &signals,
                &combos,
                &strategy,
                &KvdfStrategyParams::default(),
            )
            .expect_err("unsupported features should be rejected before CUDA runtime work");
            assert!(err.contains("without overlays/stop-loss/take-profit"));
        }
    }

    #[test]
    fn eval_entry_returns_empty_for_empty_combo_batch_before_feature_rejection() {
        let close = vec![1.0f32; 8];
        let signals = sample_signals_with_close_ptr(&close, 0);
        let combos = KvdfGpuComboBatch {
            fast_row_index: &[],
            slow_row_index: &[],
            dti_row_index: &[],
            start_t: &[],
            end_t: &[],
        };
        let strategy = StrategyConfig {
            stop_loss_pct: Some(0.02),
            ..StrategyConfig::default()
        };

        let metrics = eval_kvdf_metrics_gpu_kernel_device_rows(
            0,
            &signals,
            &combos,
            &strategy,
            &KvdfStrategyParams::default(),
        )
        .expect("empty combo batches intentionally short-circuit");
        assert!(metrics.is_empty());
    }
}
