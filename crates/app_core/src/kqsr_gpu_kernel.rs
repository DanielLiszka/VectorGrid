#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module, DeviceLaunchCaps, KernelLaunchCaps,
};
use crate::kqsr::KqsrStrategyParams;
use crate::standalone_gpu_runtime::{
    checked_i32, ensure_device_buffer_len, ensure_host_f32_len, slice_signature, CloseSeriesKey,
    ResidentCloseReturns, UploadSignature,
};
use cust::context::{Context, CurrentContext};
use cust::device::Device;
use cust::function::Function;
use cust::memory::{CopyDestination, DeviceCopy};
use cust::stream::{Stream, StreamFlags};
use cust::{memory::DeviceBuffer, module::Module, CudaFlags};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;
use ta_strategies::double_ma::{Metrics, StrategyConfig};

const KQSR_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE;
const KQSR_FLAG_LONG_ONLY: u32 = 1u32 << 0;
const KQSR_FLAG_DISABLE_FLIP: u32 = 1u32 << 1;
const KQSR_FLAG_TRADE_NEXT_BAR: u32 = 1u32 << 2;
const KQSR_FLAG_SIGNED_EXPOSURE: u32 = 1u32 << 4;

pub struct KqsrGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub close_device_ptr: u64,
    pub cols: usize,
    pub j_row_major_ptr: u64,
    pub fast_rows: usize,
    pub fast_stride: usize,
    pub srsi_k_row_major_ptr: u64,
    pub srsi_d_row_major_ptr: u64,
    pub ift_rsi_row_major_ptr: u64,
    pub qqe_fast_row_major_ptr: u64,
    pub qqe_slow_row_major_ptr: u64,
    pub slow_rows: usize,
    pub slow_stride: usize,
    pub prb_upper_row_major_ptr: u64,
    pub prb_lower_row_major_ptr: u64,
}

pub struct KqsrGpuComboBatch<'a> {
    pub fast_row_index: &'a [i32],
    pub slow_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
}

#[derive(Default)]
struct KqsrKernelScratch {
    d_combo_fast: Option<DeviceBuffer<i32>>,
    d_combo_slow: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    host_metrics: Vec<f32>,
    combo_fast_sig: Option<UploadSignature>,
    combo_slow_sig: Option<UploadSignature>,
    combo_start_sig: Option<UploadSignature>,
    combo_end_sig: Option<UploadSignature>,
}

impl KqsrKernelScratch {
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
enum KqsrKernelKey {
    ComputeReturns,
    Backtest,
}

struct KqsrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    resident_series: Option<ResidentCloseReturns>,
    scratch: KqsrKernelScratch,
    kernel_caps: RefCell<HashMap<KqsrKernelKey, KernelLaunchCaps>>,
}

impl KqsrCudaRuntime {
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
            scratch: KqsrKernelScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for(&self, key: KqsrKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

thread_local! {
    static KQSR_GPU_RUNTIME: RefCell<HashMap<u32, KqsrCudaRuntime>> = RefCell::new(HashMap::new());
}

fn ensure_uploaded_slice<T: DeviceCopy + std::hash::Hash>(
    slot: &mut Option<DeviceBuffer<T>>,
    sig_slot: &mut Option<UploadSignature>,
    values: &[T],
    stream: &Stream,
    label: &str,
) -> Result<UploadSignature, String> {
    ensure_device_buffer_len(slot, values.len(), stream)?;
    let sig = slice_signature(values);
    if *sig_slot != Some(sig) {
        let mut prefix = slot
            .as_mut()
            .ok_or_else(|| format!("KQSR GPU {label} scratch missing"))?
            .index(..values.len());
        prefix.copy_from(values).map_err(|e| e.to_string())?;
        *sig_slot = Some(sig);
    }
    Ok(sig)
}

fn validate_inputs(
    signals: &KqsrGpuDeviceSignalMatrices<'_>,
    combos: &KqsrGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("KQSR GPU kernel requires non-empty series".to_string());
    }
    let n_i32 = checked_i32("KQSR GPU cols", n)?;
    if signals.close_f32.len() != n {
        return Err("KQSR GPU close length mismatch".to_string());
    }
    if signals.j_row_major_ptr == 0
        || signals.srsi_k_row_major_ptr == 0
        || signals.srsi_d_row_major_ptr == 0
        || signals.ift_rsi_row_major_ptr == 0
        || signals.qqe_fast_row_major_ptr == 0
        || signals.qqe_slow_row_major_ptr == 0
        || signals.prb_upper_row_major_ptr == 0
        || signals.prb_lower_row_major_ptr == 0
    {
        return Err("KQSR GPU signal pointer is null".to_string());
    }
    if signals.fast_rows == 0
        || signals.fast_stride == 0
        || signals.slow_rows == 0
        || signals.slow_stride == 0
    {
        return Err("KQSR GPU signal matrix shape is invalid".to_string());
    }
    if signals.fast_stride < n || signals.slow_stride < n {
        return Err("KQSR GPU stride is smaller than cols".to_string());
    }

    let fast_rows_i32 = checked_i32("KQSR GPU fast row count", signals.fast_rows)?;
    checked_i32("KQSR GPU fast stride", signals.fast_stride)?;
    let slow_rows_i32 = checked_i32("KQSR GPU slow row count", signals.slow_rows)?;
    checked_i32("KQSR GPU slow stride", signals.slow_stride)?;

    let combos_len = combos.fast_row_index.len();
    checked_i32("KQSR GPU combo count", combos_len)?;
    if combos.slow_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("KQSR GPU combo vector length mismatch".to_string());
    }

    for (idx, ((&fast_row, &slow_row), (&start_t, &end_t))) in combos
        .fast_row_index
        .iter()
        .zip(combos.slow_row_index.iter())
        .zip(combos.start_t.iter().zip(combos.end_t.iter()))
        .enumerate()
    {
        if fast_row < 0 || fast_row >= fast_rows_i32 {
            return Err(format!(
                "KQSR GPU fast_row_index[{idx}] out of bounds: {fast_row} not in [0, {})",
                signals.fast_rows
            ));
        }
        if slow_row < 0 || slow_row >= slow_rows_i32 {
            return Err(format!(
                "KQSR GPU slow_row_index[{idx}] out of bounds: {slow_row} not in [0, {})",
                signals.slow_rows
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("KQSR GPU time window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "KQSR GPU time window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if start_t >= n_i32 {
            return Err(format!(
                "KQSR GPU time window[{idx}] start_t {start_t} exceeds cols {n_i32}"
            ));
        }
        if end_t > n_i32 {
            return Err(format!(
                "KQSR GPU time window[{idx}] end_t {end_t} exceeds cols {n_i32}"
            ));
        }
    }
    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut KqsrCudaRuntime,
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
        .map_err(|e| format!("KQSR get compute_log_returns_f32: {e}"))?;
    let launch = choose_launch_shape_1d_from_candidates(
        len,
        rt.launch_caps,
        rt.launch_caps_for(KqsrKernelKey::ComputeReturns, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr;
    let mut len_i = checked_i32("KQSR GPU close length", len)?;
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
    rt: &'a mut KqsrCudaRuntime,
    signals: &KqsrGpuDeviceSignalMatrices<'_>,
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
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(signals.close_f32.len()) }
            .map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, close_ptr, &mut d_lr, signals.close_f32.len())?;
        rt.resident_series = Some(ResidentCloseReturns {
            key,
            close_ptr,
            d_close_owner,
            d_lr,
        });
    }

    rt.resident_series
        .as_ref()
        .ok_or_else(|| "KQSR resident close/lr cache missing".to_string())
}

#[inline]
fn kqsr_kernel_flags(strategy: &StrategyConfig) -> u32 {
    let mut flags = KQSR_FLAG_SIGNED_EXPOSURE;
    if strategy.long_only {
        flags |= KQSR_FLAG_LONG_ONLY;
    }
    if !strategy.allow_flip {
        flags |= KQSR_FLAG_DISABLE_FLIP;
    }
    if strategy.trade_on_next_bar {
        flags |= KQSR_FLAG_TRADE_NEXT_BAR;
    }
    flags
}

fn decode_metrics_row(row: &[f32]) -> Metrics {
    decode_non_double_ma_gpu_metrics_row(row)
}

pub fn eval_kqsr_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &KqsrGpuDeviceSignalMatrices<'_>,
    combos: &KqsrGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    kqsr: &KqsrStrategyParams,
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
            "KQSR GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_len = combo_count
        .checked_mul(KQSR_METRICS_STRIDE)
        .ok_or_else(|| "KQSR GPU metrics buffer overflow".to_string())?;

    KQSR_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, KqsrCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "KQSR GPU runtime missing".to_string())?;
        rt.set_current()?;

        let (close_ptr_raw, lr_ptr_raw) = {
            let resident = ensure_resident_close_lr(rt, signals)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (resident.close_ptr, resident.d_lr.as_device_ptr().as_raw())
        };

        KqsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_fast, combo_count, &rt.stream)?;
        KqsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_slow, combo_count, &rt.stream)?;
        KqsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        KqsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        KqsrKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;

        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_fast,
            &mut rt.scratch.combo_fast_sig,
            combos.fast_row_index,
            &rt.stream,
            "combo fast",
        )?;
        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_slow,
            &mut rt.scratch.combo_slow_sig,
            combos.slow_row_index,
            &rt.stream,
            "combo slow",
        )?;
        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_start,
            &mut rt.scratch.combo_start_sig,
            combos.start_t,
            &rt.stream,
            "combo start",
        )?;
        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_end,
            &mut rt.scratch.combo_end_sig,
            combos.end_t,
            &rt.stream,
            "combo end",
        )?;

        let kernel = rt
            .module
            .get_function("kqsr_backtest_metrics_f32")
            .map_err(|e| format!("KQSR get kqsr_backtest_metrics_f32: {e}"))?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(KqsrKernelKey::Backtest, &kernel),
            &[32u32, 64, 128, 256],
        );

        let mut close_ptr = close_ptr_raw;
        let mut lr_ptr = lr_ptr_raw;
        let mut t_i = checked_i32("KQSR GPU cols", signals.cols)?;
        let mut j_ptr = signals.j_row_major_ptr;
        let mut fast_rows_i = checked_i32("KQSR GPU fast row count", signals.fast_rows)?;
        let mut fast_stride_i = checked_i32("KQSR GPU fast stride", signals.fast_stride)?;
        let mut srsi_k_ptr = signals.srsi_k_row_major_ptr;
        let mut srsi_d_ptr = signals.srsi_d_row_major_ptr;
        let mut ift_ptr = signals.ift_rsi_row_major_ptr;
        let mut qqe_fast_ptr = signals.qqe_fast_row_major_ptr;
        let mut qqe_slow_ptr = signals.qqe_slow_row_major_ptr;
        let mut slow_rows_i = checked_i32("KQSR GPU slow row count", signals.slow_rows)?;
        let mut slow_stride_i = checked_i32("KQSR GPU slow stride", signals.slow_stride)?;
        let mut prb_upper_ptr = signals.prb_upper_row_major_ptr;
        let mut prb_lower_ptr = signals.prb_lower_row_major_ptr;
        let mut combo_fast_ptr = rt
            .scratch
            .d_combo_fast
            .as_ref()
            .ok_or_else(|| "KQSR combo fast scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_slow_ptr = rt
            .scratch
            .d_combo_slow
            .as_ref()
            .ok_or_else(|| "KQSR combo slow scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "KQSR combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "KQSR combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_count_i = checked_i32("KQSR GPU combo count", combo_count)?;
        let mut flags_u = kqsr_kernel_flags(strategy);
        let mut commission_f = strategy.effective_commission().clamp(0.0, 0.95) as f32;
        let mut oversold_f = kqsr.oversold.clamp(0.0, 100.0) as f32;
        let mut overbought_f = kqsr.overbought.clamp(0.0, 100.0) as f32;
        let mut prb_threshold_f = kqsr.prb_threshold.clamp(0.0, 1.0) as f32;
        let mut hurdle_tau_f = strategy.hurdle_tau.unwrap_or(0.0) as f32;
        let mut out_stride_i = checked_i32("KQSR GPU metrics stride", KQSR_METRICS_STRIDE)?;
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "KQSR output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut j_ptr as *mut _ as *mut c_void,
                &mut fast_rows_i as *mut _ as *mut c_void,
                &mut fast_stride_i as *mut _ as *mut c_void,
                &mut srsi_k_ptr as *mut _ as *mut c_void,
                &mut srsi_d_ptr as *mut _ as *mut c_void,
                &mut ift_ptr as *mut _ as *mut c_void,
                &mut qqe_fast_ptr as *mut _ as *mut c_void,
                &mut qqe_slow_ptr as *mut _ as *mut c_void,
                &mut slow_rows_i as *mut _ as *mut c_void,
                &mut slow_stride_i as *mut _ as *mut c_void,
                &mut prb_upper_ptr as *mut _ as *mut c_void,
                &mut prb_lower_ptr as *mut _ as *mut c_void,
                &mut combo_fast_ptr as *mut _ as *mut c_void,
                &mut combo_slow_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut oversold_f as *mut _ as *mut c_void,
                &mut overbought_f as *mut _ as *mut c_void,
                &mut prb_threshold_f as *mut _ as *mut c_void,
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
        let out_slice = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "KQSR output scratch missing".to_string())?
            .index(..out_len);
        out_slice
            .copy_to(rt.scratch.host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out = Vec::with_capacity(combo_count);
        for row in rt.scratch.host_metrics[..out_len].chunks_exact(KQSR_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_signals_with_close_ptr(
        close_f32: &[f32],
        close_device_ptr: u64,
    ) -> KqsrGpuDeviceSignalMatrices<'_> {
        KqsrGpuDeviceSignalMatrices {
            close_f32,
            close_device_ptr,
            cols: close_f32.len(),
            j_row_major_ptr: 11,
            fast_rows: 2,
            fast_stride: close_f32.len(),
            srsi_k_row_major_ptr: 12,
            srsi_d_row_major_ptr: 13,
            ift_rsi_row_major_ptr: 14,
            qqe_fast_row_major_ptr: 15,
            qqe_slow_row_major_ptr: 16,
            slow_rows: 2,
            slow_stride: close_f32.len(),
            prb_upper_row_major_ptr: 17,
            prb_lower_row_major_ptr: 18,
        }
    }

    fn sample_signals(close_f32: &[f32]) -> KqsrGpuDeviceSignalMatrices<'_> {
        sample_signals_with_close_ptr(close_f32, 1)
    }

    fn sample_combo_batch<'a>() -> KqsrGpuComboBatch<'a> {
        KqsrGpuComboBatch {
            fast_row_index: &[0],
            slow_row_index: &[0],
            start_t: &[1],
            end_t: &[4],
        }
    }

    #[test]
    fn validate_inputs_rejects_stride_smaller_than_cols() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let mut signals = sample_signals(&close);
        signals.fast_stride = 3;
        let combos = sample_combo_batch();

        let err = validate_inputs(&signals, &combos).expect_err("stride bounds");
        assert!(err.contains("stride is smaller than cols"));
    }

    #[test]
    fn validate_inputs_rejects_out_of_range_combo_rows() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close);
        let combos = KqsrGpuComboBatch {
            fast_row_index: &[2],
            slow_row_index: &[0],
            start_t: &[1],
            end_t: &[4],
        };

        let err = validate_inputs(&signals, &combos).expect_err("fast row bounds");
        assert!(err.contains("fast_row_index[0]"));
    }

    #[test]
    fn validate_inputs_rejects_invalid_windows() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close);

        let negative = KqsrGpuComboBatch {
            fast_row_index: &[0],
            slow_row_index: &[0],
            start_t: &[-1],
            end_t: &[4],
        };
        let err = validate_inputs(&signals, &negative).expect_err("negative bounds");
        assert!(err.contains("must be non-negative"));

        let empty = KqsrGpuComboBatch {
            fast_row_index: &[0],
            slow_row_index: &[0],
            start_t: &[3],
            end_t: &[3],
        };
        let err = validate_inputs(&signals, &empty).expect_err("empty window");
        assert!(err.contains("requires start_t < end_t"));
    }

    #[test]
    fn validate_inputs_allows_close_device_ptr_zero_host_upload_fallback() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals_with_close_ptr(&close, 0);
        let combos = sample_combo_batch();

        validate_inputs(&signals, &combos).expect("host-upload fallback should validate");
    }

    #[test]
    fn validate_inputs_rejects_combo_length_mismatch() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close);
        let combos = KqsrGpuComboBatch {
            fast_row_index: &[0, 1],
            slow_row_index: &[0],
            start_t: &[1, 1],
            end_t: &[4, 4],
        };

        let err = validate_inputs(&signals, &combos).expect_err("combo length mismatch");
        assert!(err.contains("combo vector length mismatch"));
    }

    #[test]
    fn eval_entry_rejects_unsupported_features_for_non_empty_combos() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals_with_close_ptr(&close, 0);
        let combos = sample_combo_batch();
        let strategy = StrategyConfig {
            stop_loss_pct: Some(0.02),
            ..StrategyConfig::default()
        };

        let err = eval_kqsr_metrics_gpu_kernel_device_rows(
            0,
            &signals,
            &combos,
            &strategy,
            &KqsrStrategyParams::default(),
        )
        .expect_err("unsupported features should be rejected before CUDA runtime work");
        assert!(err.contains("without overlays/stop-loss/take-profit"));
    }

    #[test]
    fn eval_entry_returns_empty_for_empty_combo_batch_before_feature_rejection() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals_with_close_ptr(&close, 0);
        let combos = KqsrGpuComboBatch {
            fast_row_index: &[],
            slow_row_index: &[],
            start_t: &[],
            end_t: &[],
        };
        let strategy = StrategyConfig {
            stop_loss_pct: Some(0.02),
            ..StrategyConfig::default()
        };

        let metrics = eval_kqsr_metrics_gpu_kernel_device_rows(
            0,
            &signals,
            &combos,
            &strategy,
            &KqsrStrategyParams::default(),
        )
        .expect("empty combo batches intentionally short-circuit");
        assert!(metrics.is_empty());
    }

    #[test]
    fn checked_i32_rejects_overflow() {
        let err = checked_i32("KQSR GPU test value", usize::MAX).expect_err("overflow");
        assert!(err.contains("exceeds i32 range"));
    }
}
