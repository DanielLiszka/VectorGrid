#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::cdmc::CdmcStrategyParams;
use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module, DeviceLaunchCaps, KernelLaunchCaps,
};
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

const CDMC_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_METRICS_STRIDE;
const CDMC_FLAG_LONG_ONLY: u32 = 1u32 << 0;
const CDMC_FLAG_DISABLE_FLIP: u32 = 1u32 << 1;
const CDMC_FLAG_TRADE_NEXT_BAR: u32 = 1u32 << 2;
const CDMC_FLAG_SIGNED_EXPOSURE: u32 = 1u32 << 4;

pub struct CdmcGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub close_device_ptr: u64,
    pub cols: usize,
    pub dpo_row_major_ptr: u64,
    pub dpo_rows: usize,
    pub dpo_stride: usize,
    pub cycle_angle_row_major_ptr: u64,
    pub cycle_state_row_major_ptr: u64,
    pub cycle_rows: usize,
    pub cycle_stride: usize,
    pub mama_ptr: u64,
    pub fama_ptr: u64,
    pub decycler_row_major_ptr: u64,
    pub decycler_rows: usize,
    pub decycler_stride: usize,
}

pub struct CdmcGpuComboBatch<'a> {
    pub dpo_row_index: &'a [i32],
    pub cycle_row_index: &'a [i32],
    pub decycler_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
}

#[derive(Default)]
struct CdmcKernelScratch {
    d_combo_dpo: Option<DeviceBuffer<i32>>,
    d_combo_cycle: Option<DeviceBuffer<i32>>,
    d_combo_decycler: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    host_metrics: Vec<f32>,
    combo_dpo_sig: Option<UploadSignature>,
    combo_cycle_sig: Option<UploadSignature>,
    combo_decycler_sig: Option<UploadSignature>,
    combo_start_sig: Option<UploadSignature>,
    combo_end_sig: Option<UploadSignature>,
}

impl CdmcKernelScratch {
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
enum CdmcKernelKey {
    ComputeReturns,
    Backtest,
}

struct CdmcCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    resident_series: Option<ResidentCloseReturns>,
    scratch: CdmcKernelScratch,
    kernel_caps: RefCell<HashMap<CdmcKernelKey, KernelLaunchCaps>>,
}

impl CdmcCudaRuntime {
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
            scratch: CdmcKernelScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for(&self, key: CdmcKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

thread_local! {
    static CDMC_GPU_RUNTIME: RefCell<HashMap<u32, CdmcCudaRuntime>> = RefCell::new(HashMap::new());
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
            .ok_or_else(|| format!("CDMC GPU {label} scratch missing"))?
            .index(..values.len());
        prefix.copy_from(values).map_err(|e| e.to_string())?;
        *sig_slot = Some(sig);
    }
    Ok(sig)
}

fn validate_inputs(
    signals: &CdmcGpuDeviceSignalMatrices<'_>,
    combos: &CdmcGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("CDMC GPU kernel requires non-empty series".to_string());
    }
    let n_i32 = checked_i32("CDMC GPU cols", n)?;
    if signals.close_f32.len() != n {
        return Err("CDMC GPU close length mismatch".to_string());
    }
    if signals.dpo_row_major_ptr == 0
        || signals.cycle_angle_row_major_ptr == 0
        || signals.cycle_state_row_major_ptr == 0
        || signals.mama_ptr == 0
        || signals.fama_ptr == 0
        || signals.decycler_row_major_ptr == 0
    {
        return Err("CDMC GPU signal pointer is null".to_string());
    }
    if signals.dpo_rows == 0
        || signals.dpo_stride == 0
        || signals.cycle_rows == 0
        || signals.cycle_stride == 0
        || signals.decycler_rows == 0
        || signals.decycler_stride == 0
    {
        return Err("CDMC GPU signal matrix shape is invalid".to_string());
    }
    if signals.dpo_stride < n || signals.cycle_stride < n || signals.decycler_stride < n {
        return Err("CDMC GPU stride is smaller than cols".to_string());
    }

    let dpo_rows_i32 = checked_i32("CDMC GPU dpo row count", signals.dpo_rows)?;
    checked_i32("CDMC GPU dpo stride", signals.dpo_stride)?;
    let cycle_rows_i32 = checked_i32("CDMC GPU cycle row count", signals.cycle_rows)?;
    checked_i32("CDMC GPU cycle stride", signals.cycle_stride)?;
    let decycler_rows_i32 = checked_i32("CDMC GPU decycler row count", signals.decycler_rows)?;
    checked_i32("CDMC GPU decycler stride", signals.decycler_stride)?;

    let combos_len = combos.dpo_row_index.len();
    checked_i32("CDMC GPU combo count", combos_len)?;
    if combos.cycle_row_index.len() != combos_len
        || combos.decycler_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("CDMC GPU combo vector length mismatch".to_string());
    }

    for (idx, (((&dpo_row, &cycle_row), &decycler_row), (&start_t, &end_t))) in combos
        .dpo_row_index
        .iter()
        .zip(combos.cycle_row_index.iter())
        .zip(combos.decycler_row_index.iter())
        .zip(combos.start_t.iter().zip(combos.end_t.iter()))
        .enumerate()
    {
        if dpo_row < 0 || dpo_row >= dpo_rows_i32 {
            return Err(format!(
                "CDMC GPU dpo_row_index[{idx}] out of bounds: {dpo_row} not in [0, {})",
                signals.dpo_rows
            ));
        }
        if cycle_row < 0 || cycle_row >= cycle_rows_i32 {
            return Err(format!(
                "CDMC GPU cycle_row_index[{idx}] out of bounds: {cycle_row} not in [0, {})",
                signals.cycle_rows
            ));
        }
        if decycler_row < 0 || decycler_row >= decycler_rows_i32 {
            return Err(format!(
                "CDMC GPU decycler_row_index[{idx}] out of bounds: {decycler_row} not in [0, {})",
                signals.decycler_rows
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("CDMC GPU time window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "CDMC GPU time window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if start_t >= n_i32 {
            return Err(format!(
                "CDMC GPU time window[{idx}] start_t {start_t} exceeds cols {n_i32}"
            ));
        }
        if end_t > n_i32 {
            return Err(format!(
                "CDMC GPU time window[{idx}] end_t {end_t} exceeds cols {n_i32}"
            ));
        }
    }

    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut CdmcCudaRuntime,
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
        .map_err(|e| format!("CDMC get compute_log_returns_f32: {e}"))?;
    let launch = choose_launch_shape_1d_from_candidates(
        len,
        rt.launch_caps,
        rt.launch_caps_for(CdmcKernelKey::ComputeReturns, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr;
    let mut len_i = checked_i32("CDMC GPU close length", len)?;
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
    rt: &'a mut CdmcCudaRuntime,
    signals: &CdmcGpuDeviceSignalMatrices<'_>,
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
        .ok_or_else(|| "CDMC resident close/lr cache missing".to_string())
}

#[inline]
fn cdmc_kernel_flags(strategy: &StrategyConfig) -> u32 {
    let mut flags = CDMC_FLAG_SIGNED_EXPOSURE;
    if strategy.long_only {
        flags |= CDMC_FLAG_LONG_ONLY;
    }
    if !strategy.allow_flip {
        flags |= CDMC_FLAG_DISABLE_FLIP;
    }
    if strategy.trade_on_next_bar {
        flags |= CDMC_FLAG_TRADE_NEXT_BAR;
    }
    flags
}

fn decode_metrics_row(row: &[f32]) -> Metrics {
    decode_non_double_ma_gpu_metrics_row(row)
}

pub fn eval_cdmc_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &CdmcGpuDeviceSignalMatrices<'_>,
    combos: &CdmcGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    cdmc: &CdmcStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.dpo_row_index.len();
    if combo_count == 0 {
        return Ok(Vec::new());
    }
    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "CDMC GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_len = combo_count
        .checked_mul(CDMC_METRICS_STRIDE)
        .ok_or_else(|| "CDMC GPU metrics buffer overflow".to_string())?;

    CDMC_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, CdmcCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "CDMC GPU runtime missing".to_string())?;
        rt.set_current()?;

        let (close_ptr_raw, lr_ptr_raw) = {
            let resident = ensure_resident_close_lr(rt, signals)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (resident.close_ptr, resident.d_lr.as_device_ptr().as_raw())
        };

        CdmcKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_dpo, combo_count, &rt.stream)?;
        CdmcKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_cycle, combo_count, &rt.stream)?;
        CdmcKernelScratch::ensure_len_i32(
            &mut rt.scratch.d_combo_decycler,
            combo_count,
            &rt.stream,
        )?;
        CdmcKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        CdmcKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        CdmcKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;

        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_dpo,
            &mut rt.scratch.combo_dpo_sig,
            combos.dpo_row_index,
            &rt.stream,
            "combo dpo",
        )?;
        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_cycle,
            &mut rt.scratch.combo_cycle_sig,
            combos.cycle_row_index,
            &rt.stream,
            "combo cycle",
        )?;
        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_decycler,
            &mut rt.scratch.combo_decycler_sig,
            combos.decycler_row_index,
            &rt.stream,
            "combo decycler",
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
            .get_function("cdmc_backtest_metrics_f32")
            .map_err(|e| format!("CDMC get cdmc_backtest_metrics_f32: {e}"))?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(CdmcKernelKey::Backtest, &kernel),
            &[32u32, 64, 128, 256],
        );

        let mut close_ptr = close_ptr_raw;
        let mut lr_ptr = lr_ptr_raw;
        let mut t_i = checked_i32("CDMC GPU cols", signals.cols)?;
        let mut dpo_ptr = signals.dpo_row_major_ptr;
        let mut dpo_rows_i = checked_i32("CDMC GPU dpo row count", signals.dpo_rows)?;
        let mut dpo_stride_i = checked_i32("CDMC GPU dpo stride", signals.dpo_stride)?;
        let mut angle_ptr = signals.cycle_angle_row_major_ptr;
        let mut state_ptr = signals.cycle_state_row_major_ptr;
        let mut cycle_rows_i = checked_i32("CDMC GPU cycle row count", signals.cycle_rows)?;
        let mut cycle_stride_i = checked_i32("CDMC GPU cycle stride", signals.cycle_stride)?;
        let mut mama_ptr = signals.mama_ptr;
        let mut fama_ptr = signals.fama_ptr;
        let mut decycler_ptr = signals.decycler_row_major_ptr;
        let mut decycler_rows_i =
            checked_i32("CDMC GPU decycler row count", signals.decycler_rows)?;
        let mut decycler_stride_i =
            checked_i32("CDMC GPU decycler stride", signals.decycler_stride)?;
        let mut combo_dpo_ptr = rt
            .scratch
            .d_combo_dpo
            .as_ref()
            .ok_or_else(|| "CDMC combo dpo scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_cycle_ptr = rt
            .scratch
            .d_combo_cycle
            .as_ref()
            .ok_or_else(|| "CDMC combo cycle scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_decycler_ptr = rt
            .scratch
            .d_combo_decycler
            .as_ref()
            .ok_or_else(|| "CDMC combo decycler scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "CDMC combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "CDMC combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_count_i = checked_i32("CDMC GPU combo count", combo_count)?;
        let mut flags_u = cdmc_kernel_flags(strategy);
        let mut commission_f = strategy.effective_commission().clamp(0.0, 0.95) as f32;
        let mut phase_threshold_f =
            cdmc.phase_threshold.abs().clamp(0.0, f64::from(f32::MAX)) as f32;
        let mut out_stride_i = checked_i32("CDMC GPU metrics stride", CDMC_METRICS_STRIDE)?;
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "CDMC output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut dpo_ptr as *mut _ as *mut c_void,
                &mut dpo_rows_i as *mut _ as *mut c_void,
                &mut dpo_stride_i as *mut _ as *mut c_void,
                &mut angle_ptr as *mut _ as *mut c_void,
                &mut state_ptr as *mut _ as *mut c_void,
                &mut cycle_rows_i as *mut _ as *mut c_void,
                &mut cycle_stride_i as *mut _ as *mut c_void,
                &mut mama_ptr as *mut _ as *mut c_void,
                &mut fama_ptr as *mut _ as *mut c_void,
                &mut decycler_ptr as *mut _ as *mut c_void,
                &mut decycler_rows_i as *mut _ as *mut c_void,
                &mut decycler_stride_i as *mut _ as *mut c_void,
                &mut combo_dpo_ptr as *mut _ as *mut c_void,
                &mut combo_cycle_ptr as *mut _ as *mut c_void,
                &mut combo_decycler_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut phase_threshold_f as *mut _ as *mut c_void,
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
            .ok_or_else(|| "CDMC output scratch missing".to_string())?
            .index(..out_len);
        out_slice
            .copy_to(rt.scratch.host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out = Vec::with_capacity(combo_count);
        for row in rt.scratch.host_metrics[..out_len].chunks_exact(CDMC_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_signals(close_f32: &[f32]) -> CdmcGpuDeviceSignalMatrices<'_> {
        CdmcGpuDeviceSignalMatrices {
            close_f32,
            close_device_ptr: 1,
            cols: close_f32.len(),
            dpo_row_major_ptr: 11,
            dpo_rows: 2,
            dpo_stride: close_f32.len(),
            cycle_angle_row_major_ptr: 12,
            cycle_state_row_major_ptr: 13,
            cycle_rows: 2,
            cycle_stride: close_f32.len(),
            mama_ptr: 14,
            fama_ptr: 15,
            decycler_row_major_ptr: 16,
            decycler_rows: 2,
            decycler_stride: close_f32.len(),
        }
    }

    #[test]
    fn validate_inputs_rejects_stride_smaller_than_cols() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let mut signals = sample_signals(&close);
        signals.cycle_stride = 3;
        let combos = CdmcGpuComboBatch {
            dpo_row_index: &[0],
            cycle_row_index: &[0],
            decycler_row_index: &[0],
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
        let combos = CdmcGpuComboBatch {
            dpo_row_index: &[0],
            cycle_row_index: &[2],
            decycler_row_index: &[0],
            start_t: &[1],
            end_t: &[4],
        };

        let err = validate_inputs(&signals, &combos).expect_err("cycle row bounds");
        assert!(err.contains("cycle_row_index[0]"));
    }

    #[test]
    fn validate_inputs_rejects_invalid_windows() {
        let close = [100.0f32, 101.0, 102.0, 103.0];
        let signals = sample_signals(&close);

        let negative = CdmcGpuComboBatch {
            dpo_row_index: &[0],
            cycle_row_index: &[0],
            decycler_row_index: &[0],
            start_t: &[-1],
            end_t: &[4],
        };
        let err = validate_inputs(&signals, &negative).expect_err("negative bounds");
        assert!(err.contains("must be non-negative"));

        let empty = CdmcGpuComboBatch {
            dpo_row_index: &[0],
            cycle_row_index: &[0],
            decycler_row_index: &[0],
            start_t: &[3],
            end_t: &[3],
        };
        let err = validate_inputs(&signals, &empty).expect_err("empty window");
        assert!(err.contains("requires start_t < end_t"));
    }

    #[test]
    fn checked_i32_rejects_overflow() {
        let err = checked_i32("CDMC GPU overflow", (i32::MAX as usize) + 1)
            .expect_err("overflow should fail");
        assert!(err.contains("exceeds i32 range"));
    }
}
