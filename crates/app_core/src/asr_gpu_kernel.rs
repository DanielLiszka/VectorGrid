#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::asr::AsrStrategyParams;
use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE,
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

pub struct AsrGpuSignalMatrices<'a> {
    pub close_f64: &'a [f64],
    pub close_device_ptr: u64,
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
    pub close_device_ptr: u64,
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

const ASR_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE;

#[derive(Default)]
struct AsrKernelScratch {
    d_fast_sq_rows: Option<DeviceBuffer<u64>>,
    d_fast_mom_rows: Option<DeviceBuffer<u64>>,
    d_slow_ang_rows: Option<DeviceBuffer<u64>>,
    d_slow_er_rows: Option<DeviceBuffer<u64>>,
    d_slow_z_rows: Option<DeviceBuffer<u64>>,
    d_fast_sq_packed: Option<DeviceBuffer<f32>>,
    d_fast_mom_packed: Option<DeviceBuffer<f32>>,
    d_slow_ang_packed: Option<DeviceBuffer<f32>>,
    d_slow_er_packed: Option<DeviceBuffer<f32>>,
    d_slow_z_packed: Option<DeviceBuffer<f32>>,
    d_combo_fast: Option<DeviceBuffer<i32>>,
    d_combo_slow: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    host_metrics: Vec<f32>,
    fast_sq_rows_sig: Option<UploadSignature>,
    fast_mom_rows_sig: Option<UploadSignature>,
    slow_ang_rows_sig: Option<UploadSignature>,
    slow_er_rows_sig: Option<UploadSignature>,
    slow_z_rows_sig: Option<UploadSignature>,
    fast_sq_packed_sig: Option<(UploadSignature, usize)>,
    fast_mom_packed_sig: Option<(UploadSignature, usize)>,
    slow_ang_packed_sig: Option<(UploadSignature, usize)>,
    slow_er_packed_sig: Option<(UploadSignature, usize)>,
    slow_z_packed_sig: Option<(UploadSignature, usize)>,
    combo_fast_sig: Option<UploadSignature>,
    combo_slow_sig: Option<UploadSignature>,
    combo_start_sig: Option<UploadSignature>,
    combo_end_sig: Option<UploadSignature>,
}

impl AsrKernelScratch {
    fn ensure_len_u64(
        slot: &mut Option<DeviceBuffer<u64>>,
        required: usize,
        stream: &Stream,
    ) -> Result<(), String> {
        ensure_device_buffer_len(slot, required, stream)
    }

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
enum AsrKernelKey {
    ComputeReturns,
    PackRows,
    Backtest,
}

struct AsrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    resident_series: Option<ResidentCloseReturns>,
    scratch: AsrKernelScratch,
    kernel_caps: RefCell<HashMap<AsrKernelKey, KernelLaunchCaps>>,
}

impl AsrCudaRuntime {
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
            scratch: AsrKernelScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for(&self, key: AsrKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

thread_local! {
    static ASR_GPU_RUNTIME: RefCell<HashMap<u32, AsrCudaRuntime>> = RefCell::new(HashMap::new());
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
            .ok_or_else(|| format!("ASR GPU {label} scratch missing"))?
            .index(..values.len());
        prefix.copy_from(values).map_err(|e| e.to_string())?;
        *sig_slot = Some(sig);
    }
    Ok(sig)
}

fn validate_inputs(
    signals: &AsrGpuSignalMatrices<'_>,
    combos: &AsrGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("ASR GPU kernel requires non-empty series".to_string());
    }
    let n_i32 = checked_i32("ASR GPU cols", n)?;
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
    checked_i32("ASR GPU fast row count", signals.fast_rows)?;
    checked_i32("ASR GPU slow row count", signals.slow_rows)?;

    let combos_len = combos.fast_row_index.len();
    checked_i32("ASR GPU combo count", combos_len)?;
    if combos.slow_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("ASR GPU combo vector length mismatch".to_string());
    }
    for (idx, ((&fast_row, &slow_row), (&start_t, &end_t))) in combos
        .fast_row_index
        .iter()
        .zip(combos.slow_row_index.iter())
        .zip(combos.start_t.iter().zip(combos.end_t.iter()))
        .enumerate()
    {
        if fast_row < 0 || fast_row >= signals.fast_rows as i32 {
            return Err(format!(
                "ASR GPU fast_row_index[{idx}] out of bounds: {fast_row} not in [0, {})",
                signals.fast_rows
            ));
        }
        if slow_row < 0 || slow_row >= signals.slow_rows as i32 {
            return Err(format!(
                "ASR GPU slow_row_index[{idx}] out of bounds: {slow_row} not in [0, {})",
                signals.slow_rows
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("ASR GPU time window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "ASR GPU time window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if start_t >= n_i32 {
            return Err(format!(
                "ASR GPU time window[{idx}] start_t {start_t} exceeds cols {n_i32}"
            ));
        }
        if end_t > n_i32 {
            return Err(format!(
                "ASR GPU time window[{idx}] end_t {end_t} exceeds cols {n_i32}"
            ));
        }
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
    let n_i32 = checked_i32("ASR GPU cols", n)?;
    if signals.close_f64.len() != n {
        return Err("ASR GPU close length mismatch".to_string());
    }
    checked_i32("ASR GPU fast row count", signals.fast_rows)?;
    checked_i32("ASR GPU slow row count", signals.slow_rows)?;
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
    checked_i32("ASR GPU combo count", combos_len)?;
    if combos.slow_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("ASR GPU combo vector length mismatch".to_string());
    }
    for (idx, ((&fast_row, &slow_row), (&start_t, &end_t))) in combos
        .fast_row_index
        .iter()
        .zip(combos.slow_row_index.iter())
        .zip(combos.start_t.iter().zip(combos.end_t.iter()))
        .enumerate()
    {
        if fast_row < 0 || fast_row >= signals.fast_rows as i32 {
            return Err(format!(
                "ASR GPU fast_row_index[{idx}] out of bounds: {fast_row} not in [0, {})",
                signals.fast_rows
            ));
        }
        if slow_row < 0 || slow_row >= signals.slow_rows as i32 {
            return Err(format!(
                "ASR GPU slow_row_index[{idx}] out of bounds: {slow_row} not in [0, {})",
                signals.slow_rows
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("ASR GPU time window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "ASR GPU time window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if start_t >= n_i32 {
            return Err(format!(
                "ASR GPU time window[{idx}] start_t {start_t} exceeds cols {n_i32}"
            ));
        }
        if end_t > n_i32 {
            return Err(format!(
                "ASR GPU time window[{idx}] end_t {end_t} exceeds cols {n_i32}"
            ));
        }
    }
    Ok(())
}

#[inline]
fn should_compact_rows(used_rows: usize, total_rows: usize) -> bool {
    total_rows > 0 && used_rows < total_rows && (used_rows as f64 / total_rows as f64) <= 0.85
}

fn collect_used_rows_sorted(
    combo_rows: &[i32],
    total_rows: usize,
    used_rows: &mut Vec<usize>,
) -> Result<(), String> {
    used_rows.clear();
    used_rows.reserve(combo_rows.len());
    for &row in combo_rows {
        if row < 0 || row as usize >= total_rows {
            return Err(format!(
                "ASR GPU combo row index out of bounds: {row} not in [0, {total_rows})"
            ));
        }
        used_rows.push(row as usize);
    }
    used_rows.sort_unstable();
    used_rows.dedup();
    Ok(())
}

fn remap_compacted_combo_rows(
    combo_rows: &[i32],
    total_rows: usize,
    used_rows: &[usize],
    inverse_rows: &mut Vec<i32>,
    remapped_combo_rows: &mut Vec<i32>,
) -> Result<(), String> {
    if inverse_rows.len() < total_rows {
        inverse_rows.resize(total_rows, -1);
    } else {
        inverse_rows[..total_rows].fill(-1);
    }

    for (new_idx, &old_idx) in used_rows.iter().enumerate() {
        inverse_rows[old_idx] = checked_i32("ASR GPU compact row index", new_idx)?;
    }

    remapped_combo_rows.clear();
    remapped_combo_rows.reserve(combo_rows.len());
    for &row in combo_rows {
        remapped_combo_rows.push(inverse_rows[row as usize]);
    }
    Ok(())
}

fn gather_selected_row_ptrs(all_rows: &[u64], selected_rows: &[usize], out: &mut Vec<u64>) {
    out.clear();
    out.reserve(selected_rows.len());
    for &row in selected_rows {
        out.push(all_rows[row]);
    }
}

fn prepare_compacted_row_plan<'a>(
    combo_rows: &'a [i32],
    total_rows: usize,
    used_rows: &'a mut Vec<usize>,
    inverse_rows: &mut Vec<i32>,
    remapped_combo_rows: &'a mut Vec<i32>,
) -> Result<(usize, Option<&'a [usize]>, &'a [i32]), String> {
    collect_used_rows_sorted(combo_rows, total_rows, used_rows)?;
    if should_compact_rows(used_rows.len(), total_rows) {
        remap_compacted_combo_rows(
            combo_rows,
            total_rows,
            used_rows.as_slice(),
            inverse_rows,
            remapped_combo_rows,
        )?;
        Ok((
            used_rows.len(),
            Some(used_rows.as_slice()),
            remapped_combo_rows.as_slice(),
        ))
    } else {
        Ok((total_rows, None, combo_rows))
    }
}

fn launch_compute_log_returns_f32(
    rt: &mut AsrCudaRuntime,
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
        rt.launch_caps_for(AsrKernelKey::ComputeReturns, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr;
    let mut len_i = checked_i32("ASR GPU close length", len)?;
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
    rt: &'a mut AsrCudaRuntime,
    close_f64: &[f64],
    close_device_ptr: u64,
) -> Result<&'a ResidentCloseReturns, String> {
    let key = CloseSeriesKey {
        len: close_f64.len(),
        host_ptr: close_f64.as_ptr() as usize,
        device_ptr: close_device_ptr,
    };
    let needs_refresh = rt
        .resident_series
        .as_ref()
        .map(|resident| resident.key != key)
        .unwrap_or(true);
    if needs_refresh {
        let (close_ptr, d_close_owner) = if close_device_ptr != 0 {
            (close_device_ptr, None)
        } else {
            let close_f32: Vec<f32> = close_f64.iter().map(|&value| value as f32).collect();
            let d_close = DeviceBuffer::from_slice(&close_f32).map_err(|e| e.to_string())?;
            let close_ptr = d_close.as_device_ptr().as_raw();
            (close_ptr, Some(d_close))
        };
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(close_f64.len()) }
            .map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, close_ptr, &mut d_lr, close_f64.len())?;
        rt.resident_series = Some(ResidentCloseReturns {
            key,
            close_ptr,
            d_close_owner,
            d_lr,
        });
    }

    rt.resident_series
        .as_ref()
        .ok_or_else(|| "ASR resident close/lr cache missing".to_string())
}

fn launch_pack_rows_u64_ptr_f32(
    rt: &mut AsrCudaRuntime,
    row_ptrs_ptr: u64,
    rows: usize,
    cols: usize,
    out_ptr: u64,
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
    let launch = choose_launch_shape_1d_from_candidates(
        total,
        rt.launch_caps,
        rt.launch_caps_for(AsrKernelKey::PackRows, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut row_ptrs_ptr = row_ptrs_ptr;
    let mut rows_i = checked_i32("ASR GPU pack row count", rows)?;
    let mut cols_i = checked_i32("ASR GPU pack cols", cols)?;
    let mut out_ptr = out_ptr;

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

#[inline]
fn asr_kernel_flags(strategy: &StrategyConfig) -> u32 {
    let mut flags: u32 = 1u32 << 4; // signed exposure
    if strategy.long_only {
        flags |= 1u32 << 0;
    }
    if !strategy.allow_flip {
        flags |= 1u32 << 1;
    }
    if strategy.trade_on_next_bar {
        flags |= 1u32 << 2;
    }
    flags
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

    let out_stride = ASR_METRICS_STRIDE;
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

        let (close_ptr_raw, lr_ptr_raw) = {
            let resident =
                ensure_resident_close_lr(rt, signals.close_f64, signals.close_device_ptr)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (resident.close_ptr, resident.d_lr.as_device_ptr().as_raw())
        };

        AsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_fast, combo_count, &rt.stream)?;
        AsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_slow, combo_count, &rt.stream)?;
        AsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        AsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        AsrKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;

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

        let kernel = rt
            .module
            .get_function("asr_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(AsrKernelKey::Backtest, &kernel),
            &[32u32, 64, 128, 256],
        );

        let mut close_ptr = close_ptr_raw;
        let mut lr_ptr = lr_ptr_raw;
        let mut fast_sq_ptr = d_fast_sq.as_device_ptr().as_raw();
        let mut fast_mom_ptr = d_fast_mom.as_device_ptr().as_raw();
        let mut slow_ang_ptr = d_slow_ang.as_device_ptr().as_raw();
        let mut slow_er_ptr = d_slow_er.as_device_ptr().as_raw();
        let mut slow_z_ptr = d_slow_z.as_device_ptr().as_raw();
        let mut k_long_ptr = d_k_long.as_device_ptr().as_raw();
        let mut k_short_ptr = d_k_short.as_device_ptr().as_raw();
        let mut combo_fast_ptr = rt
            .scratch
            .d_combo_fast
            .as_ref()
            .ok_or_else(|| "ASR combo fast scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_slow_ptr = rt
            .scratch
            .d_combo_slow
            .as_ref()
            .ok_or_else(|| "ASR combo slow scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "ASR combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "ASR combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "ASR output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

        let mut t_i = checked_i32("ASR GPU cols", signals.cols)?;
        let mut fast_rows_i = checked_i32("ASR GPU fast row count", signals.fast_rows)?;
        let mut fast_stride_i = checked_i32("ASR GPU fast stride", signals.cols)?;
        let mut slow_rows_i = checked_i32("ASR GPU slow row count", signals.slow_rows)?;
        let mut slow_stride_i = checked_i32("ASR GPU slow stride", signals.cols)?;
        let mut combo_count_i = checked_i32("ASR GPU combo count", combo_count)?;
        let mut flags_u = asr_kernel_flags(strategy);
        let mut commission_f = strategy.effective_commission().clamp(0.0, 0.95) as f32;
        let mut angle_min_f = asr.angle_deg_min as f32;
        let mut angle_mr_f = asr.angle_deg_mr as f32;
        let mut use_er_filter_i = i32::from(asr.use_er_filter);
        let mut er_min_f = asr.er_min as f32;
        let mut er_mr_f = asr.er_mr as f32;
        let mut z_entry_f = asr.z_entry as f32;
        let mut z_exit_f = asr.z_exit as f32;
        let mut confirm_bars_i = i32::from(asr.confirm_bars);
        let mut mom_fade_bars_i = i32::from(asr.mom_fade_bars);
        let mut max_trade_bars_i = i32::from(asr.max_trade_bars);
        let mut max_mr_trade_bars_i = i32::from(asr.max_mr_trade_bars);
        let mut hurdle_tau_f = strategy.hurdle_tau.unwrap_or(0.0) as f32;
        let mut out_stride_i = checked_i32("ASR GPU metrics stride", out_stride)?;

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
            .ok_or_else(|| "ASR output scratch missing".to_string())?
            .index(..out_len);
        out_slice
            .copy_to(rt.scratch.host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut metrics = Vec::with_capacity(combo_count);
        for i in 0..combo_count {
            let base = i * out_stride;
            metrics.push(decode_non_double_ma_gpu_metrics_row(
                &rt.scratch.host_metrics[base..base + out_stride],
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

    let out_stride = ASR_METRICS_STRIDE;
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

        let (close_ptr_raw, lr_ptr_raw) = {
            let resident =
                ensure_resident_close_lr(rt, signals.close_f64, signals.close_device_ptr)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (resident.close_ptr, resident.d_lr.as_device_ptr().as_raw())
        };

        let mut fast_used_rows = Vec::new();
        let mut fast_inverse_rows = Vec::new();
        let mut fast_remapped_combo_rows = Vec::new();
        let (fast_rows_used, fast_selected_rows, fast_combo_rows_upload) =
            prepare_compacted_row_plan(
                combos.fast_row_index,
                signals.fast_rows,
                &mut fast_used_rows,
                &mut fast_inverse_rows,
                &mut fast_remapped_combo_rows,
            )?;
        let fast_mat_len = fast_rows_used
            .checked_mul(signals.cols)
            .ok_or_else(|| "ASR GPU compact fast matrix overflow".to_string())?;

        let mut slow_used_rows = Vec::new();
        let mut slow_inverse_rows = Vec::new();
        let mut slow_remapped_combo_rows = Vec::new();
        let (slow_rows_used, slow_selected_rows, slow_combo_rows_upload) =
            prepare_compacted_row_plan(
                combos.slow_row_index,
                signals.slow_rows,
                &mut slow_used_rows,
                &mut slow_inverse_rows,
                &mut slow_remapped_combo_rows,
            )?;
        let slow_mat_len = slow_rows_used
            .checked_mul(signals.cols)
            .ok_or_else(|| "ASR GPU compact slow matrix overflow".to_string())?;

        let mut selected_row_ptrs: Vec<u64> = Vec::new();

        AsrKernelScratch::ensure_len_u64(
            &mut rt.scratch.d_fast_sq_rows,
            fast_rows_used,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_u64(
            &mut rt.scratch.d_fast_mom_rows,
            fast_rows_used,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_u64(
            &mut rt.scratch.d_slow_ang_rows,
            slow_rows_used,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_u64(
            &mut rt.scratch.d_slow_er_rows,
            slow_rows_used,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_u64(
            &mut rt.scratch.d_slow_z_rows,
            slow_rows_used,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_f32(
            &mut rt.scratch.d_fast_sq_packed,
            fast_mat_len,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_f32(
            &mut rt.scratch.d_fast_mom_packed,
            fast_mat_len,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_f32(
            &mut rt.scratch.d_slow_ang_packed,
            slow_mat_len,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_f32(
            &mut rt.scratch.d_slow_er_packed,
            slow_mat_len,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_f32(
            &mut rt.scratch.d_slow_z_packed,
            slow_mat_len,
            &rt.stream,
        )?;
        AsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_fast, combo_count, &rt.stream)?;
        AsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_slow, combo_count, &rt.stream)?;
        AsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        AsrKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        AsrKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;

        let fast_sq_rows_sig = ensure_uploaded_slice(
            &mut rt.scratch.d_fast_sq_rows,
            &mut rt.scratch.fast_sq_rows_sig,
            if let Some(selected_rows) = fast_selected_rows {
                gather_selected_row_ptrs(
                    signals.fast_squeeze_rows,
                    selected_rows,
                    &mut selected_row_ptrs,
                );
                selected_row_ptrs.as_slice()
            } else {
                signals.fast_squeeze_rows
            },
            &rt.stream,
            "fast squeeze rows",
        )?;
        let fast_mom_rows_sig = ensure_uploaded_slice(
            &mut rt.scratch.d_fast_mom_rows,
            &mut rt.scratch.fast_mom_rows_sig,
            if let Some(selected_rows) = fast_selected_rows {
                gather_selected_row_ptrs(
                    signals.fast_momentum_rows,
                    selected_rows,
                    &mut selected_row_ptrs,
                );
                selected_row_ptrs.as_slice()
            } else {
                signals.fast_momentum_rows
            },
            &rt.stream,
            "fast momentum rows",
        )?;
        let slow_ang_rows_sig = ensure_uploaded_slice(
            &mut rt.scratch.d_slow_ang_rows,
            &mut rt.scratch.slow_ang_rows_sig,
            if let Some(selected_rows) = slow_selected_rows {
                gather_selected_row_ptrs(
                    signals.slow_angle_rows,
                    selected_rows,
                    &mut selected_row_ptrs,
                );
                selected_row_ptrs.as_slice()
            } else {
                signals.slow_angle_rows
            },
            &rt.stream,
            "slow angle rows",
        )?;
        let slow_er_rows_sig = ensure_uploaded_slice(
            &mut rt.scratch.d_slow_er_rows,
            &mut rt.scratch.slow_er_rows_sig,
            if let Some(selected_rows) = slow_selected_rows {
                gather_selected_row_ptrs(
                    signals.slow_er_rows,
                    selected_rows,
                    &mut selected_row_ptrs,
                );
                selected_row_ptrs.as_slice()
            } else {
                signals.slow_er_rows
            },
            &rt.stream,
            "slow ER rows",
        )?;
        let slow_z_rows_sig = ensure_uploaded_slice(
            &mut rt.scratch.d_slow_z_rows,
            &mut rt.scratch.slow_z_rows_sig,
            if let Some(selected_rows) = slow_selected_rows {
                gather_selected_row_ptrs(
                    signals.slow_z_rows,
                    selected_rows,
                    &mut selected_row_ptrs,
                );
                selected_row_ptrs.as_slice()
            } else {
                signals.slow_z_rows
            },
            &rt.stream,
            "slow z-score rows",
        )?;

        let fast_sq_pack_sig = (fast_sq_rows_sig, signals.cols);
        if rt.scratch.fast_sq_packed_sig != Some(fast_sq_pack_sig) {
            let row_ptrs_ptr = rt
                .scratch
                .d_fast_sq_rows
                .as_ref()
                .ok_or_else(|| "ASR fast squeeze row scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            let out_ptr = rt
                .scratch
                .d_fast_sq_packed
                .as_ref()
                .ok_or_else(|| "ASR fast squeeze packed scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            launch_pack_rows_u64_ptr_f32(rt, row_ptrs_ptr, fast_rows_used, signals.cols, out_ptr)?;
            rt.scratch.fast_sq_packed_sig = Some(fast_sq_pack_sig);
        }

        let fast_mom_pack_sig = (fast_mom_rows_sig, signals.cols);
        if rt.scratch.fast_mom_packed_sig != Some(fast_mom_pack_sig) {
            let row_ptrs_ptr = rt
                .scratch
                .d_fast_mom_rows
                .as_ref()
                .ok_or_else(|| "ASR fast momentum row scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            let out_ptr = rt
                .scratch
                .d_fast_mom_packed
                .as_ref()
                .ok_or_else(|| "ASR fast momentum packed scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            launch_pack_rows_u64_ptr_f32(rt, row_ptrs_ptr, fast_rows_used, signals.cols, out_ptr)?;
            rt.scratch.fast_mom_packed_sig = Some(fast_mom_pack_sig);
        }

        let slow_ang_pack_sig = (slow_ang_rows_sig, signals.cols);
        if rt.scratch.slow_ang_packed_sig != Some(slow_ang_pack_sig) {
            let row_ptrs_ptr = rt
                .scratch
                .d_slow_ang_rows
                .as_ref()
                .ok_or_else(|| "ASR slow angle row scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            let out_ptr = rt
                .scratch
                .d_slow_ang_packed
                .as_ref()
                .ok_or_else(|| "ASR slow angle packed scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            launch_pack_rows_u64_ptr_f32(rt, row_ptrs_ptr, slow_rows_used, signals.cols, out_ptr)?;
            rt.scratch.slow_ang_packed_sig = Some(slow_ang_pack_sig);
        }

        let slow_er_pack_sig = (slow_er_rows_sig, signals.cols);
        if rt.scratch.slow_er_packed_sig != Some(slow_er_pack_sig) {
            let row_ptrs_ptr = rt
                .scratch
                .d_slow_er_rows
                .as_ref()
                .ok_or_else(|| "ASR slow ER row scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            let out_ptr = rt
                .scratch
                .d_slow_er_packed
                .as_ref()
                .ok_or_else(|| "ASR slow ER packed scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            launch_pack_rows_u64_ptr_f32(rt, row_ptrs_ptr, slow_rows_used, signals.cols, out_ptr)?;
            rt.scratch.slow_er_packed_sig = Some(slow_er_pack_sig);
        }

        let slow_z_pack_sig = (slow_z_rows_sig, signals.cols);
        if rt.scratch.slow_z_packed_sig != Some(slow_z_pack_sig) {
            let row_ptrs_ptr = rt
                .scratch
                .d_slow_z_rows
                .as_ref()
                .ok_or_else(|| "ASR slow z-score row scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            let out_ptr = rt
                .scratch
                .d_slow_z_packed
                .as_ref()
                .ok_or_else(|| "ASR slow z-score packed scratch missing".to_string())?
                .as_device_ptr()
                .as_raw();
            launch_pack_rows_u64_ptr_f32(rt, row_ptrs_ptr, slow_rows_used, signals.cols, out_ptr)?;
            rt.scratch.slow_z_packed_sig = Some(slow_z_pack_sig);
        }

        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_fast,
            &mut rt.scratch.combo_fast_sig,
            fast_combo_rows_upload,
            &rt.stream,
            "combo fast",
        )?;
        ensure_uploaded_slice(
            &mut rt.scratch.d_combo_slow,
            &mut rt.scratch.combo_slow_sig,
            slow_combo_rows_upload,
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
            .get_function("asr_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(AsrKernelKey::Backtest, &kernel),
            &[32u32, 64, 128, 256],
        );

        let mut close_ptr = close_ptr_raw;
        let mut lr_ptr = lr_ptr_raw;
        let mut fast_sq_ptr = rt
            .scratch
            .d_fast_sq_packed
            .as_ref()
            .ok_or_else(|| "ASR fast squeeze packed scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut fast_mom_ptr = rt
            .scratch
            .d_fast_mom_packed
            .as_ref()
            .ok_or_else(|| "ASR fast momentum packed scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut slow_ang_ptr = rt
            .scratch
            .d_slow_ang_packed
            .as_ref()
            .ok_or_else(|| "ASR slow angle packed scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut slow_er_ptr = rt
            .scratch
            .d_slow_er_packed
            .as_ref()
            .ok_or_else(|| "ASR slow ER packed scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut slow_z_ptr = rt
            .scratch
            .d_slow_z_packed
            .as_ref()
            .ok_or_else(|| "ASR slow z-score packed scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut k_long_ptr = signals.kaufman_long_row;
        let mut k_short_ptr = signals.kaufman_short_row;
        let mut combo_fast_ptr = rt
            .scratch
            .d_combo_fast
            .as_ref()
            .ok_or_else(|| "ASR combo fast scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_slow_ptr = rt
            .scratch
            .d_combo_slow
            .as_ref()
            .ok_or_else(|| "ASR combo slow scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "ASR combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "ASR combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "ASR output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

        let mut t_i = checked_i32("ASR GPU cols", signals.cols)?;
        let mut fast_rows_i = checked_i32("ASR GPU fast row count", signals.fast_rows)?;
        let mut fast_stride_i = checked_i32("ASR GPU fast stride", signals.cols)?;
        let mut slow_rows_i = checked_i32("ASR GPU slow row count", signals.slow_rows)?;
        let mut slow_stride_i = checked_i32("ASR GPU slow stride", signals.cols)?;
        let mut combo_count_i = checked_i32("ASR GPU combo count", combo_count)?;
        let mut flags_u = asr_kernel_flags(strategy);
        let mut commission_f = strategy.effective_commission().clamp(0.0, 0.95) as f32;
        let mut angle_min_f = asr.angle_deg_min as f32;
        let mut angle_mr_f = asr.angle_deg_mr as f32;
        let mut use_er_filter_i = i32::from(asr.use_er_filter);
        let mut er_min_f = asr.er_min as f32;
        let mut er_mr_f = asr.er_mr as f32;
        let mut z_entry_f = asr.z_entry as f32;
        let mut z_exit_f = asr.z_exit as f32;
        let mut confirm_bars_i = i32::from(asr.confirm_bars);
        let mut mom_fade_bars_i = i32::from(asr.mom_fade_bars);
        let mut max_trade_bars_i = i32::from(asr.max_trade_bars);
        let mut max_mr_trade_bars_i = i32::from(asr.max_mr_trade_bars);
        let mut hurdle_tau_f = strategy.hurdle_tau.unwrap_or(0.0) as f32;
        let mut out_stride_i = checked_i32("ASR GPU metrics stride", out_stride)?;

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
            .ok_or_else(|| "ASR output scratch missing".to_string())?
            .index(..out_len);
        out_slice
            .copy_to(rt.scratch.host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut metrics = Vec::with_capacity(combo_count);
        for i in 0..combo_count {
            let base = i * out_stride;
            metrics.push(decode_non_double_ma_gpu_metrics_row(
                &rt.scratch.host_metrics[base..base + out_stride],
            ));
        }
        Ok(metrics)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device_signals<'a>(
        close_f64: &'a [f64],
        fast_rows: &'a [u64],
        slow_rows: &'a [u64],
    ) -> AsrGpuDeviceSignalMatrices<'a> {
        AsrGpuDeviceSignalMatrices {
            close_f64,
            close_device_ptr: 1,
            cols: close_f64.len(),
            fast_rows: fast_rows.len(),
            slow_rows: slow_rows.len(),
            fast_squeeze_rows: fast_rows,
            fast_momentum_rows: fast_rows,
            slow_angle_rows: slow_rows,
            slow_er_rows: slow_rows,
            slow_z_rows: slow_rows,
            kaufman_long_row: 11,
            kaufman_short_row: 12,
        }
    }

    #[test]
    fn validate_inputs_device_rows_rejects_out_of_range_combo_rows() {
        let close = [100.0, 101.0, 102.0, 103.0];
        let fast_rows = [21u64, 22];
        let slow_rows = [31u64, 32];
        let signals = sample_device_signals(&close, &fast_rows, &slow_rows);
        let combos = AsrGpuComboBatch {
            fast_row_index: &[2],
            slow_row_index: &[0],
            start_t: &[1],
            end_t: &[4],
        };

        let err = validate_inputs_device_rows(&signals, &combos).expect_err("row bounds");
        assert!(err.contains("fast_row_index[0]"));
    }

    #[test]
    fn validate_inputs_device_rows_rejects_invalid_windows() {
        let close = [100.0, 101.0, 102.0, 103.0];
        let fast_rows = [21u64];
        let slow_rows = [31u64];
        let signals = sample_device_signals(&close, &fast_rows, &slow_rows);

        let negative = AsrGpuComboBatch {
            fast_row_index: &[0],
            slow_row_index: &[0],
            start_t: &[-1],
            end_t: &[4],
        };
        let err = validate_inputs_device_rows(&signals, &negative).expect_err("negative bounds");
        assert!(err.contains("must be non-negative"));

        let empty = AsrGpuComboBatch {
            fast_row_index: &[0],
            slow_row_index: &[0],
            start_t: &[3],
            end_t: &[3],
        };
        let err = validate_inputs_device_rows(&signals, &empty).expect_err("empty window");
        assert!(err.contains("requires start_t < end_t"));
    }

    #[test]
    fn checked_i32_rejects_overflow() {
        let err = checked_i32("ASR GPU test value", usize::MAX).expect_err("overflow");
        assert!(err.contains("exceeds i32 range"));
    }

    #[test]
    fn prepare_compacted_row_plan_remaps_sparse_rows() {
        let mut used_rows = Vec::new();
        let mut inverse_rows = Vec::new();
        let mut remapped = Vec::new();
        let combos = [7, 2, 7, 4];

        let (row_count, selected_rows, combo_rows) = prepare_compacted_row_plan(
            &combos,
            10,
            &mut used_rows,
            &mut inverse_rows,
            &mut remapped,
        )
        .expect("compact sparse rows");

        assert_eq!(row_count, 3);
        assert_eq!(selected_rows.expect("selected rows"), &[2, 4, 7]);
        assert_eq!(combo_rows, &[2, 0, 2, 1]);
    }

    #[test]
    fn prepare_compacted_row_plan_skips_near_full_coverage() {
        let mut used_rows = Vec::new();
        let mut inverse_rows = Vec::new();
        let mut remapped = Vec::new();
        let combos = [0, 1, 2, 3, 4, 5, 6];

        let (row_count, selected_rows, combo_rows) = prepare_compacted_row_plan(
            &combos,
            8,
            &mut used_rows,
            &mut inverse_rows,
            &mut remapped,
        )
        .expect("skip dense compaction");

        assert_eq!(row_count, 8);
        assert!(selected_rows.is_none());
        assert_eq!(combo_rows, combos);
    }
}
