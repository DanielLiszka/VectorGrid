#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::alloc_policy::{alloc_f64_with_prefix_uninit_tail, resize_uninit_f64};
use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module, DeviceLaunchCaps, KernelLaunchCaps,
};
use crate::standalone_gpu_runtime::{
    checked_i32, ensure_device_buffer_len, ensure_host_f32_len, f64_slice_content_hash,
    slice_signature, CloseSeriesKey, UploadSignature,
};
use crate::vcc::VccStrategyParams;
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

pub struct VccGpuDeviceSignalMatrices<'a> {
    pub close_f64: &'a [f64],
    pub close_f32: &'a [f32],
    pub close_device_ptr: u64,
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

#[derive(Default)]
struct VccKernelScratch {
    d_fast_sq_rows: Option<DeviceBuffer<u64>>,
    d_fast_mom_rows: Option<DeviceBuffer<u64>>,
    d_trend_rows: Option<DeviceBuffer<u64>>,
    d_fast_sq: Option<DeviceBuffer<f32>>,
    d_fast_mom: Option<DeviceBuffer<f32>>,
    d_trend: Option<DeviceBuffer<f32>>,
    d_combo_fast: Option<DeviceBuffer<i32>>,
    d_combo_trend: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_metric_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    d_slice_out: Option<DeviceBuffer<f64>>,
    host_out: Vec<f32>,
    host_slice_out: Vec<f64>,
    fast_sq_rows_sig: Option<UploadSignature>,
    fast_mom_rows_sig: Option<UploadSignature>,
    trend_rows_sig: Option<UploadSignature>,
    fast_sq_sig: Option<UploadSignature>,
    fast_mom_sig: Option<UploadSignature>,
    trend_sig: Option<UploadSignature>,
    combo_fast_sig: Option<UploadSignature>,
    combo_trend_sig: Option<UploadSignature>,
    combo_start_sig: Option<UploadSignature>,
    combo_metric_start_sig: Option<UploadSignature>,
    combo_end_sig: Option<UploadSignature>,
}

impl VccKernelScratch {
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

    fn ensure_len_f64(
        slot: &mut Option<DeviceBuffer<f64>>,
        required: usize,
        stream: &Stream,
    ) -> Result<(), String> {
        ensure_device_buffer_len(slot, required, stream)
    }

    fn ensure_host_out(&mut self, required: usize) {
        ensure_host_f32_len(&mut self.host_out, required);
    }

    fn ensure_host_slice_out(&mut self, required: usize) {
        resize_uninit_f64(&mut self.host_slice_out, required);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum VccKernelKey {
    ComputeReturns,
    PackRows,
    Backtest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VccCloseSeriesKey {
    f32_key: CloseSeriesKey,
    f64_content_hash: u64,
}

impl VccCloseSeriesKey {
    fn from_signals(signals: &VccGpuDeviceSignalMatrices<'_>) -> Self {
        Self {
            f32_key: CloseSeriesKey::from_host_and_device(
                signals.close_f32,
                signals.close_device_ptr,
            ),
            f64_content_hash: f64_slice_content_hash(signals.close_f64),
        }
    }
}

struct VccResidentSeries {
    key: VccCloseSeriesKey,
    close_ptr: u64,
    d_close_owner: Option<DeviceBuffer<f32>>,
    d_lr: DeviceBuffer<f32>,
    d_lr64: DeviceBuffer<f64>,
}

struct VccCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    resident_series: Option<VccResidentSeries>,
    scratch: VccKernelScratch,
    kernel_caps: RefCell<HashMap<VccKernelKey, KernelLaunchCaps>>,
}

impl VccCudaRuntime {
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
            scratch: VccKernelScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for(&self, key: VccKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

thread_local! {
    static VCC_GPU_RUNTIME: RefCell<HashMap<u32, VccCudaRuntime>> = RefCell::new(HashMap::new());
}

#[inline]
fn compute_log_returns_f64(close_f64: &[f64]) -> Vec<f64> {
    let mut out = alloc_f64_with_prefix_uninit_tail(close_f64.len(), 1, 0.0);
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

#[inline]
fn f32_upload_signature(values: &[f32]) -> UploadSignature {
    let bits = unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u32, values.len()) };
    slice_signature(bits)
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
            .ok_or_else(|| format!("VCC GPU {label} scratch missing"))?
            .index(..values.len());
        prefix.copy_from(values).map_err(|e| e.to_string())?;
        *sig_slot = Some(sig);
    }
    Ok(sig)
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
    if signals.close_f32.len() != n {
        return Err("VCC GPU close_f32 length mismatch".to_string());
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
    row_ptrs_ptr: u64,
    rows: usize,
    cols: usize,
    out_row_major_ptr: u64,
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

    let mut row_ptrs_ptr = row_ptrs_ptr;
    let mut rows_i = checked_i32("VCC GPU pack row count", rows)?;
    let mut cols_i = checked_i32("VCC GPU pack cols", cols)?;
    let mut out_ptr = out_row_major_ptr;

    let launch = choose_launch_shape_1d_from_candidates(
        total,
        rt.launch_caps,
        rt.launch_caps_for(VccKernelKey::PackRows, &kernel),
        &[64u32, 128, 256, 512],
    );
    unsafe {
        let args: &mut [*mut c_void] = &mut [
            &mut row_ptrs_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut cols_i as *mut _ as *mut c_void,
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
    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut VccCudaRuntime,
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
        rt.launch_caps_for(VccKernelKey::ComputeReturns, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr;
    let mut len_i = checked_i32("VCC GPU close length", len)?;
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

fn ensure_resident_close_returns<'a>(
    rt: &'a mut VccCudaRuntime,
    signals: &VccGpuDeviceSignalMatrices<'_>,
) -> Result<&'a VccResidentSeries, String> {
    let key = VccCloseSeriesKey::from_signals(signals);
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
        let host_lr64 = compute_log_returns_f64(signals.close_f64);
        let d_lr64 = DeviceBuffer::from_slice(&host_lr64).map_err(|e| e.to_string())?;
        rt.resident_series = Some(VccResidentSeries {
            key,
            close_ptr,
            d_close_owner,
            d_lr,
            d_lr64,
        });
    }

    rt.resident_series
        .as_ref()
        .ok_or_else(|| "VCC resident close/lr cache missing".to_string())
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

    let out_stride = NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE;
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

        let (close_ptr_raw, lr_ptr_raw, lr64_ptr_raw) = {
            let resident = ensure_resident_close_returns(rt, signals)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (
                resident.close_ptr,
                resident.d_lr.as_device_ptr().as_raw(),
                resident.d_lr64.as_device_ptr().as_raw(),
            )
        };

        let fast_mat_len = signals
            .fast_rows
            .checked_mul(signals.cols)
            .ok_or_else(|| "VCC GPU fast matrix overflow".to_string())?;
        let trend_mat_len = signals
            .trend_rows
            .checked_mul(signals.cols)
            .ok_or_else(|| "VCC GPU trend matrix overflow".to_string())?;

        VccKernelScratch::ensure_len_f32(&mut rt.scratch.d_fast_sq, fast_mat_len, &rt.stream)?;
        VccKernelScratch::ensure_len_f32(&mut rt.scratch.d_fast_mom, fast_mat_len, &rt.stream)?;
        VccKernelScratch::ensure_len_f32(&mut rt.scratch.d_trend, trend_mat_len, &rt.stream)?;
        VccKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_fast, combo_count, &rt.stream)?;
        VccKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_trend, combo_count, &rt.stream)?;
        VccKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        VccKernelScratch::ensure_len_i32(
            &mut rt.scratch.d_combo_metric_start,
            combo_count,
            &rt.stream,
        )?;
        VccKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        VccKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;
        VccKernelScratch::ensure_len_f64(&mut rt.scratch.d_slice_out, slice_out_len, &rt.stream)?;

        if let Some(host_rows) = signals.fast_squeeze_host_row_major {
            let sig = f32_upload_signature(host_rows);
            if rt.scratch.fast_sq_sig != Some(sig) {
                let mut fast_sq_slice = rt
                    .scratch
                    .d_fast_sq
                    .as_mut()
                    .ok_or_else(|| "VCC fast squeeze scratch missing".to_string())?
                    .index(..fast_mat_len);
                fast_sq_slice
                    .copy_from(host_rows)
                    .map_err(|e| e.to_string())?;
                rt.scratch.fast_sq_sig = Some(sig);
                rt.scratch.fast_sq_rows_sig = None;
            }
        } else {
            ensure_uploaded_slice(
                &mut rt.scratch.d_fast_sq_rows,
                &mut rt.scratch.fast_sq_rows_sig,
                signals.fast_squeeze_rows,
                &rt.stream,
                "fast squeeze row",
            )?;
            let row_ptrs = rt
                .scratch
                .d_fast_sq_rows
                .as_ref()
                .ok_or_else(|| "VCC fast squeeze row scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            let out = rt
                .scratch
                .d_fast_sq
                .as_mut()
                .ok_or_else(|| "VCC fast squeeze scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            launch_pack_rows_u64_ptr_f32(rt, row_ptrs, signals.fast_rows, signals.cols, out)?;
            rt.scratch.fast_sq_sig = None;
        }

        if let Some(host_rows) = signals.fast_momentum_host_row_major {
            let sig = f32_upload_signature(host_rows);
            if rt.scratch.fast_mom_sig != Some(sig) {
                let mut fast_mom_slice = rt
                    .scratch
                    .d_fast_mom
                    .as_mut()
                    .ok_or_else(|| "VCC fast momentum scratch missing".to_string())?
                    .index(..fast_mat_len);
                fast_mom_slice
                    .copy_from(host_rows)
                    .map_err(|e| e.to_string())?;
                rt.scratch.fast_mom_sig = Some(sig);
                rt.scratch.fast_mom_rows_sig = None;
            }
        } else {
            ensure_uploaded_slice(
                &mut rt.scratch.d_fast_mom_rows,
                &mut rt.scratch.fast_mom_rows_sig,
                signals.fast_momentum_rows,
                &rt.stream,
                "fast momentum row",
            )?;
            let row_ptrs = rt
                .scratch
                .d_fast_mom_rows
                .as_ref()
                .ok_or_else(|| "VCC fast momentum row scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            let out = rt
                .scratch
                .d_fast_mom
                .as_mut()
                .ok_or_else(|| "VCC fast momentum scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            launch_pack_rows_u64_ptr_f32(rt, row_ptrs, signals.fast_rows, signals.cols, out)?;
            rt.scratch.fast_mom_sig = None;
        }

        if let Some(host_rows) = signals.trend_host_row_major {
            let sig = f32_upload_signature(host_rows);
            if rt.scratch.trend_sig != Some(sig) {
                let mut trend_slice = rt
                    .scratch
                    .d_trend
                    .as_mut()
                    .ok_or_else(|| "VCC trend scratch missing".to_string())?
                    .index(..trend_mat_len);
                trend_slice
                    .copy_from(host_rows)
                    .map_err(|e| e.to_string())?;
                rt.scratch.trend_sig = Some(sig);
                rt.scratch.trend_rows_sig = None;
            }
        } else {
            ensure_uploaded_slice(
                &mut rt.scratch.d_trend_rows,
                &mut rt.scratch.trend_rows_sig,
                signals.trend_rows_ptr,
                &rt.stream,
                "trend row",
            )?;
            let row_ptrs = rt
                .scratch
                .d_trend_rows
                .as_ref()
                .ok_or_else(|| "VCC trend row scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            let out = rt
                .scratch
                .d_trend
                .as_mut()
                .ok_or_else(|| "VCC trend scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            launch_pack_rows_u64_ptr_f32(rt, row_ptrs, signals.trend_rows, signals.cols, out)?;
            rt.scratch.trend_sig = None;
        }

        let combo_fast_sig = slice_signature(combos.fast_row_index);
        if rt.scratch.combo_fast_sig != Some(combo_fast_sig) {
            let mut combo_fast_slice = rt
                .scratch
                .d_combo_fast
                .as_mut()
                .ok_or_else(|| "VCC combo fast scratch missing".to_string())?
                .index(..combo_count);
            combo_fast_slice
                .copy_from(combos.fast_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_fast_sig = Some(combo_fast_sig);
        }

        let combo_trend_sig = slice_signature(combos.trend_row_index);
        if rt.scratch.combo_trend_sig != Some(combo_trend_sig) {
            let mut combo_trend_slice = rt
                .scratch
                .d_combo_trend
                .as_mut()
                .ok_or_else(|| "VCC combo trend scratch missing".to_string())?
                .index(..combo_count);
            combo_trend_slice
                .copy_from(combos.trend_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_trend_sig = Some(combo_trend_sig);
        }

        let combo_start_sig = slice_signature(combos.start_t);
        if rt.scratch.combo_start_sig != Some(combo_start_sig) {
            let mut combo_start_slice = rt
                .scratch
                .d_combo_start
                .as_mut()
                .ok_or_else(|| "VCC combo start scratch missing".to_string())?
                .index(..combo_count);
            combo_start_slice
                .copy_from(combos.start_t)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_start_sig = Some(combo_start_sig);
        }

        let combo_metric_start_sig = slice_signature(combos.metric_start_t);
        if rt.scratch.combo_metric_start_sig != Some(combo_metric_start_sig) {
            let mut combo_metric_start_slice = rt
                .scratch
                .d_combo_metric_start
                .as_mut()
                .ok_or_else(|| "VCC combo metric_start scratch missing".to_string())?
                .index(..combo_count);
            combo_metric_start_slice
                .copy_from(combos.metric_start_t)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_metric_start_sig = Some(combo_metric_start_sig);
        }

        let combo_end_sig = slice_signature(combos.end_t);
        if rt.scratch.combo_end_sig != Some(combo_end_sig) {
            let mut combo_end_slice = rt
                .scratch
                .d_combo_end
                .as_mut()
                .ok_or_else(|| "VCC combo end scratch missing".to_string())?
                .index(..combo_count);
            combo_end_slice
                .copy_from(combos.end_t)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_end_sig = Some(combo_end_sig);
        }

        let kernel = rt
            .module
            .get_function("vcc_backtest_metrics_f32")
            .map_err(|e| format!("VCC GPU missing vcc_backtest_metrics_f32 kernel: {e}"))?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(VccKernelKey::Backtest, &kernel),
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
        let mut lr64_ptr = lr64_ptr_raw;
        let mut fast_sq_ptr = rt
            .scratch
            .d_fast_sq
            .as_ref()
            .ok_or_else(|| "VCC fast squeeze scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut fast_mom_ptr = rt
            .scratch
            .d_fast_mom
            .as_ref()
            .ok_or_else(|| "VCC fast momentum scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut trend_ptr = rt
            .scratch
            .d_trend
            .as_ref()
            .ok_or_else(|| "VCC trend scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_fast_ptr = rt
            .scratch
            .d_combo_fast
            .as_ref()
            .ok_or_else(|| "VCC combo fast scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_trend_ptr = rt
            .scratch
            .d_combo_trend
            .as_ref()
            .ok_or_else(|| "VCC combo trend scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "VCC combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_metric_start_ptr = rt
            .scratch
            .d_combo_metric_start
            .as_ref()
            .ok_or_else(|| "VCC combo metric_start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "VCC combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "VCC output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut slice_out_ptr = rt
            .scratch
            .d_slice_out
            .as_ref()
            .ok_or_else(|| "VCC slice output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

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
        let mut hurdle_tau_f = strategy.hurdle_tau.unwrap_or(0.0) as f32;
        let mut out_stride_i = checked_i32("VCC GPU metrics stride", out_stride)?;

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
                &mut hurdle_tau_f as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
                &mut slice_out_ptr as *mut _ as *mut c_void,
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

        rt.scratch.ensure_host_out(out_len);
        let d_out_prefix = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "VCC output scratch missing".to_string())?
            .index(..out_len);
        d_out_prefix
            .copy_to(rt.scratch.host_out.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let host_slice_out = if need_slice_readback {
            rt.scratch.ensure_host_slice_out(slice_out_len);
            let d_slice_out_prefix = rt
                .scratch
                .d_slice_out
                .as_ref()
                .ok_or_else(|| "VCC slice output scratch missing".to_string())?
                .index(..slice_out_len);
            d_slice_out_prefix
                .copy_to(rt.scratch.host_slice_out.as_mut_slice())
                .map_err(|e| e.to_string())?;
            Some(rt.scratch.host_slice_out.as_slice())
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
                metrics: decode_non_double_ma_gpu_metrics_row(
                    &rt.scratch.host_out[base..base + out_stride],
                ),
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

    fn sample_signals<'a>(
        close_f64: &'a [f64],
        close_f32: &'a [f32],
    ) -> VccGpuDeviceSignalMatrices<'a> {
        const FAST_PTRS: [u64; 2] = [11, 12];
        const MOM_PTRS: [u64; 2] = [13, 14];
        const TREND_PTRS: [u64; 2] = [15, 16];
        VccGpuDeviceSignalMatrices {
            close_f64,
            close_f32,
            close_device_ptr: 17,
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
        let close_f32 = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close, &close_f32);
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
        let close_f32 = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close, &close_f32);

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
        let close_f32 = [100.0f32, 101.0, 102.0, 103.0];
        let bad_fast = [1.0f32; 3];
        let signals = VccGpuDeviceSignalMatrices {
            close_f64: &close,
            close_f32: &close_f32,
            close_device_ptr: 17,
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

    #[test]
    fn resident_close_key_tracks_f64_content_even_when_f32_content_matches() {
        let close_f32 = [100.0f32, 100.0, 100.0, 100.0];
        let close_a = [100.00000001f64, 100.00000002, 100.00000003, 100.00000004];
        let close_b = [100.00000004f64, 100.00000003, 100.00000002, 100.00000001];
        assert_eq!(
            close_a
                .iter()
                .map(|value| *value as f32)
                .collect::<Vec<_>>(),
            close_b
                .iter()
                .map(|value| *value as f32)
                .collect::<Vec<_>>()
        );

        let signals_a = sample_signals(&close_a, &close_f32);
        let signals_b = sample_signals(&close_b, &close_f32);

        assert_ne!(
            VccCloseSeriesKey::from_signals(&signals_a),
            VccCloseSeriesKey::from_signals(&signals_b)
        );
    }

    #[test]
    fn validate_inputs_rejects_close_f32_length_mismatch() {
        let close = [100.0f64, 101.0, 102.0, 103.0];
        let close_f32 = [100.0f32, 101.0, 102.0];
        let signals = VccGpuDeviceSignalMatrices {
            close_f64: &close,
            close_f32: &close_f32,
            close_device_ptr: 17,
            cols: close.len(),
            fast_rows: 2,
            trend_rows: 2,
            fast_squeeze_rows: &[11, 12],
            fast_squeeze_host_row_major: None,
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

        let err = validate_inputs_device_rows(&signals, &combos).expect_err("close_f32 mismatch");
        assert!(err.contains("close_f32 length mismatch"));
    }

    #[test]
    fn compute_log_returns_f64_preserves_zero_prefix_and_invalid_rows() {
        assert!(compute_log_returns_f64(&[]).is_empty());
        assert_eq!(compute_log_returns_f64(&[100.0]), vec![0.0]);

        let out = compute_log_returns_f64(&[100.0, 110.0, f64::NAN, 90.0, 0.0, 95.0]);
        assert_eq!(out[0], 0.0);
        assert!((out[1] - (110.0f64 / 100.0).ln()).abs() <= f64::EPSILON);
        assert_eq!(out[2], 0.0);
        assert_eq!(out[3], 0.0);
        assert_eq!(out[4], 0.0);
        assert_eq!(out[5], 0.0);
    }
}
