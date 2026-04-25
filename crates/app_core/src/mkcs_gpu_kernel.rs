#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module, DeviceLaunchCaps, KernelLaunchCaps,
};
use crate::mkcs::MkcsStrategyParams;
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

const MKCS_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE;
const MKCS_FLAG_LONG_ONLY: u32 = 1u32 << 0;
const MKCS_FLAG_DISABLE_FLIP: u32 = 1u32 << 1;
const MKCS_FLAG_TRADE_NEXT_BAR: u32 = 1u32 << 2;
const MKCS_FLAG_SIGNED_EXPOSURE: u32 = 1u32 << 4;

pub struct MkcsGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],

    pub close_device_ptr: u64,
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

#[derive(Default)]
struct MkcsKernelScratch {
    d_combo_stddev: Option<DeviceBuffer<i32>>,
    d_combo_breakout: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    host_metrics: Vec<f32>,
    combo_stddev_sig: Option<UploadSignature>,
    combo_breakout_sig: Option<UploadSignature>,
    combo_start_sig: Option<UploadSignature>,
    combo_end_sig: Option<UploadSignature>,
}

impl MkcsKernelScratch {
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
enum MkcsKernelKey {
    ComputeReturns,
    Backtest,
}

struct MkcsCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    resident_series: Option<ResidentCloseReturns>,
    scratch: MkcsKernelScratch,
    kernel_caps: RefCell<HashMap<MkcsKernelKey, KernelLaunchCaps>>,
}

impl MkcsCudaRuntime {
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
            scratch: MkcsKernelScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for(&self, key: MkcsKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

thread_local! {
    static MKCS_GPU_RUNTIME: RefCell<HashMap<u32, MkcsCudaRuntime>> = RefCell::new(HashMap::new());
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
            .ok_or_else(|| format!("MKCS GPU {label} scratch missing"))?
            .index(..values.len());
        prefix.copy_from(values).map_err(|e| e.to_string())?;
        *sig_slot = Some(sig);
    }
    Ok(sig)
}

fn validate_inputs(
    signals: &MkcsGpuDeviceSignalMatrices<'_>,
    combos: &MkcsGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("MKCS GPU kernel requires non-empty series".to_string());
    }
    let n_i32 = checked_i32("MKCS GPU cols", n)?;
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
    if signals.stddev_stride < n || signals.breakout_stride < n {
        return Err("MKCS GPU stride is smaller than cols".to_string());
    }

    checked_i32("MKCS GPU stddev row count", signals.stddev_rows)?;
    checked_i32("MKCS GPU stddev stride", signals.stddev_stride)?;
    checked_i32("MKCS GPU breakout row count", signals.breakout_rows)?;
    checked_i32("MKCS GPU breakout stride", signals.breakout_stride)?;

    let combos_len = combos.stddev_row_index.len();
    checked_i32("MKCS GPU combo count", combos_len)?;
    if combos.breakout_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("MKCS GPU combo vector length mismatch".to_string());
    }

    for (idx, ((&stddev_row, &breakout_row), (&start_t, &end_t))) in combos
        .stddev_row_index
        .iter()
        .zip(combos.breakout_row_index.iter())
        .zip(combos.start_t.iter().zip(combos.end_t.iter()))
        .enumerate()
    {
        if stddev_row < 0 || stddev_row >= signals.stddev_rows as i32 {
            return Err(format!(
                "MKCS GPU stddev_row_index[{idx}] out of bounds: {stddev_row} not in [0, {})",
                signals.stddev_rows
            ));
        }
        if breakout_row < 0 || breakout_row >= signals.breakout_rows as i32 {
            return Err(format!(
                "MKCS GPU breakout_row_index[{idx}] out of bounds: {breakout_row} not in [0, {})",
                signals.breakout_rows
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("MKCS GPU time window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "MKCS GPU time window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if start_t >= n_i32 {
            return Err(format!(
                "MKCS GPU time window[{idx}] start_t {start_t} exceeds cols {n_i32}"
            ));
        }
        if end_t > n_i32 {
            return Err(format!(
                "MKCS GPU time window[{idx}] end_t {end_t} exceeds cols {n_i32}"
            ));
        }
    }
    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut MkcsCudaRuntime,
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
        rt.launch_caps_for(MkcsKernelKey::ComputeReturns, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr;
    let mut len_i = checked_i32("MKCS GPU close length", len)?;
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
    rt: &'a mut MkcsCudaRuntime,
    close_f32: &[f32],
    close_device_ptr: u64,
) -> Result<&'a ResidentCloseReturns, String> {
    let key = CloseSeriesKey::from_host_and_device(close_f32, close_device_ptr);
    let needs_refresh = rt
        .resident_series
        .as_ref()
        .map(|resident| resident.key != key)
        .unwrap_or(true);

    if needs_refresh {
        let (close_ptr, d_close_owner) = if close_device_ptr != 0 {
            (close_device_ptr, None)
        } else {
            let d_close = DeviceBuffer::from_slice(close_f32).map_err(|e| e.to_string())?;
            let close_ptr = d_close.as_device_ptr().as_raw();
            (close_ptr, Some(d_close))
        };
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(close_f32.len()) }
            .map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, close_ptr, &mut d_lr, close_f32.len())?;
        rt.resident_series = Some(ResidentCloseReturns {
            key,
            close_ptr,
            d_close_owner,
            d_lr,
        });
    }

    rt.resident_series
        .as_ref()
        .ok_or_else(|| "MKCS resident close/lr cache missing".to_string())
}

#[inline]
fn mkcs_kernel_flags(strategy: &StrategyConfig) -> u32 {
    let mut flags = MKCS_FLAG_SIGNED_EXPOSURE;
    if strategy.long_only {
        flags |= MKCS_FLAG_LONG_ONLY;
    }
    if !strategy.allow_flip {
        flags |= MKCS_FLAG_DISABLE_FLIP;
    }
    if strategy.trade_on_next_bar {
        flags |= MKCS_FLAG_TRADE_NEXT_BAR;
    }
    flags
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

        let (close_ptr_raw, lr_ptr_raw) = {
            let resident =
                ensure_resident_close_lr(rt, signals.close_f32, signals.close_device_ptr)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (resident.close_ptr, resident.d_lr.as_device_ptr().as_raw())
        };

        MkcsKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_stddev, combo_count, &rt.stream)?;
        MkcsKernelScratch::ensure_len_i32(
            &mut rt.scratch.d_combo_breakout,
            combo_count,
            &rt.stream,
        )?;
        MkcsKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        MkcsKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        MkcsKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;

        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_stddev,
            &mut rt.scratch.combo_stddev_sig,
            combos.stddev_row_index,
            &rt.stream,
            "combo stddev",
        )?;
        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_breakout,
            &mut rt.scratch.combo_breakout_sig,
            combos.breakout_row_index,
            &rt.stream,
            "combo breakout",
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
            .get_function("mkcs_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(MkcsKernelKey::Backtest, &kernel),
            &[32u32, 64, 128, 256],
        );

        let mut close_ptr = close_ptr_raw;
        let mut lr_ptr = lr_ptr_raw;
        let mut stddev_ptr = signals.stddev_row_major_ptr;
        let mut stddev_rows_i = checked_i32("MKCS GPU stddev row count", signals.stddev_rows)?;
        let mut stddev_stride_i = checked_i32("MKCS GPU stddev stride", signals.stddev_stride)?;
        let mut upper_ptr = signals.upper_row_major_ptr;
        let mut lower_ptr = signals.lower_row_major_ptr;
        let mut breakout_rows_i =
            checked_i32("MKCS GPU breakout row count", signals.breakout_rows)?;
        let mut breakout_stride_i =
            checked_i32("MKCS GPU breakout stride", signals.breakout_stride)?;
        let mut combo_stddev_ptr = rt
            .scratch
            .d_combo_stddev
            .as_ref()
            .ok_or_else(|| "MKCS combo stddev scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_breakout_ptr = rt
            .scratch
            .d_combo_breakout
            .as_ref()
            .ok_or_else(|| "MKCS combo breakout scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "MKCS combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "MKCS combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_count_i = checked_i32("MKCS GPU combo count", combo_count)?;
        let mut t_i = checked_i32("MKCS GPU cols", signals.cols)?;
        let mut flags_u = mkcs_kernel_flags(strategy);
        let mut commission_f = strategy.effective_commission().clamp(0.0, 0.95) as f32;
        let mut compression_threshold_f = mkcs.compression_threshold as f32;
        let mut reexpansion_threshold_f = mkcs.reexpansion_threshold as f32;
        let mut hurdle_tau_f = strategy.hurdle_tau.unwrap_or(0.0) as f32;
        let mut out_stride_i = checked_i32("MKCS GPU metrics stride", MKCS_METRICS_STRIDE)?;
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "MKCS output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

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
            .ok_or_else(|| "MKCS output scratch missing".to_string())?
            .index(..out_len);
        out_slice
            .copy_to(rt.scratch.host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out = Vec::with_capacity(combo_count);
        for row in rt.scratch.host_metrics[..out_len].chunks_exact(MKCS_METRICS_STRIDE) {
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
    ) -> MkcsGpuDeviceSignalMatrices<'_> {
        MkcsGpuDeviceSignalMatrices {
            close_f32,
            close_device_ptr,
            cols: close_f32.len(),
            stddev_row_major_ptr: 11,
            stddev_rows: 2,
            stddev_stride: close_f32.len(),
            upper_row_major_ptr: 12,
            lower_row_major_ptr: 13,
            breakout_rows: 2,
            breakout_stride: close_f32.len(),
        }
    }

    fn sample_signals(close_f32: &[f32]) -> MkcsGpuDeviceSignalMatrices<'_> {
        sample_signals_with_close_ptr(close_f32, 1)
    }

    fn sample_combo_batch() -> MkcsGpuComboBatch<'static> {
        MkcsGpuComboBatch {
            stddev_row_index: &[0],
            breakout_row_index: &[0],
            start_t: &[1],
            end_t: &[4],
        }
    }

    #[test]
    fn validate_inputs_rejects_stride_smaller_than_cols() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let mut signals = sample_signals(&close);
        signals.stddev_stride = 3;
        let combos = MkcsGpuComboBatch {
            stddev_row_index: &[0],
            breakout_row_index: &[0],
            start_t: &[1],
            end_t: &[4],
        };

        let err = validate_inputs(&signals, &combos).expect_err("stride bounds");
        assert!(err.contains("stride is smaller than cols"));
    }

    #[test]
    fn validate_inputs_rejects_out_of_range_combo_rows() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close);
        let combos = MkcsGpuComboBatch {
            stddev_row_index: &[2],
            breakout_row_index: &[0],
            start_t: &[1],
            end_t: &[4],
        };

        let err = validate_inputs(&signals, &combos).expect_err("row bounds");
        assert!(err.contains("stddev_row_index[0]"));
    }

    #[test]
    fn validate_inputs_rejects_invalid_windows() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close);

        let negative = MkcsGpuComboBatch {
            stddev_row_index: &[0],
            breakout_row_index: &[0],
            start_t: &[-1],
            end_t: &[4],
        };
        let err = validate_inputs(&signals, &negative).expect_err("negative bounds");
        assert!(err.contains("must be non-negative"));

        let empty = MkcsGpuComboBatch {
            stddev_row_index: &[0],
            breakout_row_index: &[0],
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
    fn eval_entry_rejects_overlay_stop_loss_and_take_profit_before_cuda_runtime_work() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
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
            let err = eval_mkcs_metrics_gpu_kernel_device_rows(
                0,
                &signals,
                &combos,
                &strategy,
                &MkcsStrategyParams::default(),
            )
            .expect_err("unsupported features should be rejected before CUDA runtime work");
            assert!(err.contains("without overlays/stop-loss/take-profit"));
        }
    }

    #[test]
    fn eval_entry_returns_empty_for_empty_combo_batch_before_feature_rejection() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals_with_close_ptr(&close, 0);
        let combos = MkcsGpuComboBatch {
            stddev_row_index: &[],
            breakout_row_index: &[],
            start_t: &[],
            end_t: &[],
        };
        let strategy = StrategyConfig {
            stop_loss_pct: Some(0.02),
            ..StrategyConfig::default()
        };

        let metrics = eval_mkcs_metrics_gpu_kernel_device_rows(
            0,
            &signals,
            &combos,
            &strategy,
            &MkcsStrategyParams::default(),
        )
        .expect("empty combo batches intentionally short-circuit");
        assert!(metrics.is_empty());
    }

    #[test]
    fn checked_i32_rejects_overflow() {
        let err = checked_i32("MKCS GPU test value", usize::MAX).expect_err("overflow");
        assert!(err.contains("exceeds i32 range"));
    }
}
