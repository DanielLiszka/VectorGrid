#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::alloc_policy::resize_uninit_f32;
use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module, DeviceLaunchCaps, KernelLaunchCaps,
};
use crate::vemr::VemrStrategyParams;
use cust::context::{Context, CurrentContext};
use cust::device::Device;
use cust::function::Function;
use cust::memory::{CopyDestination, DeviceCopy};
use cust::stream::{Stream, StreamFlags};
use cust::{memory::DeviceBuffer, module::Module, CudaFlags};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use ta_optimizer::ObjectiveKind;
use ta_strategies::double_ma::{Metrics, StrategyConfig};

const VEMR_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_METRICS_STRIDE;
const TOPK_BLOCK_ITEMS: usize = 512;
const VEMR_DEVICE_TOPK_LIMIT: usize = 256;

pub struct VemrGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub close_device_ptr: u64,
    pub cols: usize,
    pub zscore_row_major_ptr: u64,
    pub zscore_rows: usize,
    pub zscore_stride: usize,
    pub bb_upper_row_major_ptr: u64,
    pub bb_middle_row_major_ptr: u64,
    pub bb_lower_row_major_ptr: u64,
    pub bb_rows: usize,
    pub bb_stride: usize,
    pub atr_row_major_ptr: u64,
    pub atr_rows: usize,
    pub atr_stride: usize,
}

pub struct VemrGpuComboBatch<'a> {
    pub z_row_index: &'a [i32],
    pub bb_row_index: &'a [i32],
    pub atr_row_index: &'a [i32],
    pub start_t: &'a [i32],
    pub end_t: &'a [i32],
}

pub struct VemrGpuTopMetricRow {
    pub combo_index: usize,
    pub metrics: Metrics,
}

pub enum VemrGpuEvalResult {
    Full(Vec<Metrics>),
    TopK(Vec<VemrGpuTopMetricRow>),
}

#[derive(Default)]
struct VemrCudaScratch {
    d_close: Option<DeviceBuffer<f32>>,
    d_lr: Option<DeviceBuffer<f32>>,
    d_combo_z: Option<DeviceBuffer<i32>>,
    d_combo_bb: Option<DeviceBuffer<i32>>,
    d_combo_atr: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    d_combo_tags: Option<DeviceBuffer<i32>>,
    d_topk_stage_a: Option<DeviceBuffer<i32>>,
    d_topk_stage_b: Option<DeviceBuffer<i32>>,
    d_top_combo: Option<DeviceBuffer<i32>>,
    d_top_dummy: Option<DeviceBuffer<i32>>,
    d_top_metrics: Option<DeviceBuffer<f32>>,
    host_out: Vec<f32>,
    host_top_combo_tags: Vec<i32>,
    host_top_metrics: Vec<f32>,
    close_sig: Option<VemrCloseSignature>,
    combo_z_sig: Option<u64>,
    combo_bb_sig: Option<u64>,
    combo_atr_sig: Option<u64>,
    combo_tags_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VemrCloseSignature {
    len: usize,
    host_ptr: usize,
    device_ptr: u64,
}

impl VemrCudaScratch {
    fn ensure_len_f32(buf: &mut Option<DeviceBuffer<f32>>, len: usize) -> Result<(), String> {
        if buf
            .as_ref()
            .map(|existing| existing.len() < len)
            .unwrap_or(true)
        {
            *buf = Some(
                unsafe { DeviceBuffer::<f32>::uninitialized(len) }.map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }

    fn ensure_len_i32(buf: &mut Option<DeviceBuffer<i32>>, len: usize) -> Result<(), String> {
        if buf
            .as_ref()
            .map(|existing| existing.len() < len)
            .unwrap_or(true)
        {
            *buf = Some(
                unsafe { DeviceBuffer::<i32>::uninitialized(len) }.map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }

    fn ensure_host_out_len(&mut self, len: usize) {
        resize_uninit_f32(&mut self.host_out, len);
    }

    fn ensure_host_top_combo_tags_len(&mut self, len: usize) {
        if self.host_top_combo_tags.capacity() < len {
            self.host_top_combo_tags
                .reserve(len - self.host_top_combo_tags.capacity());
        }
        // Safety: the device readback fully overwrites this prefix before any read.
        unsafe { self.host_top_combo_tags.set_len(len) };
    }

    fn ensure_host_top_metrics_len(&mut self, len: usize) {
        resize_uninit_f32(&mut self.host_top_metrics, len);
    }
}

struct VemrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    device_caps: DeviceLaunchCaps,
    scratch: VemrCudaScratch,
    kernel_caps: RefCell<HashMap<VemrKernelKey, KernelLaunchCaps>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum VemrKernelKey {
    ComputeLogReturns,
    EvalMetrics,
    SelectTopkRows,
    SelectTopkRowsFromCandidates,
    GatherTopkRows,
}

impl VemrCudaRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let device_caps = detect_device_launch_caps(device);
        let context = Arc::new(Context::new(device).map_err(|e| e.to_string())?);
        let module = load_double_crossover_module(device)?;

        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;
        Ok(Self {
            context,
            module,
            stream,
            device_caps,
            scratch: VemrCudaScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for_kernel(&self, key: VemrKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.device_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

fn checked_i32(value: usize, label: &str) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("VEMR GPU {label} exceeds i32 range"))
}

fn ensure_fixed_block_kernel_support(
    kernel_name: &str,
    kernel_caps: KernelLaunchCaps,
    block_x: u32,
) -> Result<(), String> {
    if kernel_caps.max_threads_per_block < block_x {
        return Err(format!(
            "VEMR GPU kernel {kernel_name} requires block_x={block_x}, but max_threads_per_block={}",
            kernel_caps.max_threads_per_block
        ));
    }
    Ok(())
}

fn copy_prefix_from_slice<T: DeviceCopy>(
    dst: &mut DeviceBuffer<T>,
    src: &[T],
) -> Result<(), String> {
    let mut prefix = dst.index(..src.len());
    prefix.copy_from(src).map_err(|e| e.to_string())
}

fn copy_prefix_to_slice<T: DeviceCopy>(src: &DeviceBuffer<T>, dst: &mut [T]) -> Result<(), String> {
    let prefix = src.index(..dst.len());
    prefix.copy_to(dst).map_err(|e| e.to_string())
}

thread_local! {
    static VEMR_GPU_RUNTIME: RefCell<HashMap<u32, VemrCudaRuntime>> = RefCell::new(HashMap::new());
}

fn validate_inputs(
    signals: &VemrGpuDeviceSignalMatrices<'_>,
    combos: &VemrGpuComboBatch<'_>,
) -> Result<(), String> {
    let n = signals.cols;
    if n == 0 {
        return Err("VEMR GPU kernel requires non-empty series".to_string());
    }
    checked_i32(n, "cols")?;
    if signals.close_f32.len() != n {
        return Err("VEMR GPU close length mismatch".to_string());
    }
    if signals.zscore_row_major_ptr == 0
        || signals.bb_upper_row_major_ptr == 0
        || signals.bb_middle_row_major_ptr == 0
        || signals.bb_lower_row_major_ptr == 0
        || signals.atr_row_major_ptr == 0
    {
        return Err("VEMR GPU signal pointer is null".to_string());
    }
    if signals.zscore_rows == 0
        || signals.bb_rows == 0
        || signals.atr_rows == 0
        || signals.zscore_stride == 0
        || signals.bb_stride == 0
        || signals.atr_stride == 0
    {
        return Err("VEMR GPU signal matrix shape is invalid".to_string());
    }
    if signals.zscore_stride < n || signals.bb_stride < n || signals.atr_stride < n {
        return Err("VEMR GPU signal stride is smaller than series length".to_string());
    }
    checked_i32(signals.zscore_rows, "zscore rows")?;
    checked_i32(signals.zscore_stride, "zscore stride")?;
    checked_i32(signals.bb_rows, "bb rows")?;
    checked_i32(signals.bb_stride, "bb stride")?;
    checked_i32(signals.atr_rows, "atr rows")?;
    checked_i32(signals.atr_stride, "atr stride")?;

    let combos_len = combos.z_row_index.len();
    if combos.bb_row_index.len() != combos_len
        || combos.atr_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("VEMR GPU combo vector length mismatch".to_string());
    }
    checked_i32(combos_len, "combo count")?;

    for i in 0..combos_len {
        let z_row = usize::try_from(combos.z_row_index[i])
            .map_err(|_| format!("VEMR GPU z_row_index[{i}] is negative"))?;
        if z_row >= signals.zscore_rows {
            return Err(format!(
                "VEMR GPU z_row_index[{i}]={} out of bounds for {} rows",
                combos.z_row_index[i], signals.zscore_rows
            ));
        }

        let bb_row = usize::try_from(combos.bb_row_index[i])
            .map_err(|_| format!("VEMR GPU bb_row_index[{i}] is negative"))?;
        if bb_row >= signals.bb_rows {
            return Err(format!(
                "VEMR GPU bb_row_index[{i}]={} out of bounds for {} rows",
                combos.bb_row_index[i], signals.bb_rows
            ));
        }

        let atr_row = usize::try_from(combos.atr_row_index[i])
            .map_err(|_| format!("VEMR GPU atr_row_index[{i}] is negative"))?;
        if atr_row >= signals.atr_rows {
            return Err(format!(
                "VEMR GPU atr_row_index[{i}]={} out of bounds for {} rows",
                combos.atr_row_index[i], signals.atr_rows
            ));
        }

        let start_t = usize::try_from(combos.start_t[i])
            .map_err(|_| format!("VEMR GPU start_t[{i}] is negative"))?;
        let end_t = usize::try_from(combos.end_t[i])
            .map_err(|_| format!("VEMR GPU end_t[{i}] is negative"))?;
        if start_t >= n {
            return Err(format!(
                "VEMR GPU start_t[{i}]={} exceeds series length {}",
                combos.start_t[i], n
            ));
        }
        if end_t > n {
            return Err(format!(
                "VEMR GPU end_t[{i}]={} exceeds series length {}",
                combos.end_t[i], n
            ));
        }
        if start_t >= end_t {
            return Err(format!(
                "VEMR GPU invalid eval range at combo {i}: start_t={} end_t={}",
                combos.start_t[i], combos.end_t[i]
            ));
        }
    }
    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut VemrCudaRuntime,
    close_ptr: u64,
    lr_ptr: u64,
    len: usize,
) -> Result<(), String> {
    if len == 0 {
        return Ok(());
    }

    let kernel = rt
        .module
        .get_function("compute_log_returns_f32")
        .map_err(|e| e.to_string())?;
    let kernel_caps = rt.launch_caps_for_kernel(VemrKernelKey::ComputeLogReturns, &kernel);
    let launch = choose_launch_shape_1d_from_candidates(
        len,
        rt.device_caps,
        kernel_caps,
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr_mut = close_ptr;
    let mut len_i = checked_i32(len, "close length")?;
    let mut lr_ptr_mut = lr_ptr;

    unsafe {
        let args: &mut [*mut c_void] = &mut [
            &mut close_ptr_mut as *mut _ as *mut c_void,
            &mut len_i as *mut _ as *mut c_void,
            &mut lr_ptr_mut as *mut _ as *mut c_void,
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

fn objective_to_vemr_kernel_id(objective: ObjectiveKind) -> i32 {
    match objective {
        ObjectiveKind::Pnl => 0,
        ObjectiveKind::Sharpe => 1,
        ObjectiveKind::Sortino => 2,
        ObjectiveKind::Calmar => 3,
        ObjectiveKind::Omega => 4,
        ObjectiveKind::Volatility => 5,
        ObjectiveKind::MaxDrawdown => 6,
    }
}

fn objective_score(metrics: &Metrics, objective: ObjectiveKind) -> f64 {
    match objective {
        ObjectiveKind::Sharpe => metrics.sharpe,
        ObjectiveKind::Sortino => metrics.sortino,
        ObjectiveKind::Calmar => metrics.calmar,
        ObjectiveKind::Omega => metrics.omega,
        ObjectiveKind::Volatility => -metrics.volatility,
        ObjectiveKind::Pnl => metrics.pnl,
        ObjectiveKind::MaxDrawdown => -metrics.max_dd,
    }
}

fn decode_vemr_metrics_row(row: &[f32]) -> Metrics {
    decode_non_double_ma_gpu_metrics_row(row)
}

fn reduce_vemr_topk_on_device(
    rt: &mut VemrCudaRuntime,
    scratch: &mut VemrCudaScratch,
    combo_tags_ptr: u64,
    combo_dummy_ptr: u64,
    metrics_ptr_raw: u64,
    combo_count: usize,
    objective: ObjectiveKind,
    topk_select: usize,
) -> Result<Vec<VemrGpuTopMetricRow>, String> {
    let select_rows = rt
        .module
        .get_function("select_topk_vemr_metric_rows_f32")
        .map_err(|e| e.to_string())?;
    let select_rows_candidates = rt
        .module
        .get_function("select_topk_vemr_metric_rows_from_candidates_f32")
        .map_err(|e| e.to_string())?;
    let gather_rows = rt
        .module
        .get_function("gather_topk_metric_rows_f32")
        .map_err(|e| e.to_string())?;
    let select_rows_caps = rt.launch_caps_for_kernel(VemrKernelKey::SelectTopkRows, &select_rows);
    let select_rows_candidates_caps = rt.launch_caps_for_kernel(
        VemrKernelKey::SelectTopkRowsFromCandidates,
        &select_rows_candidates,
    );
    let gather_rows_caps = rt.launch_caps_for_kernel(VemrKernelKey::GatherTopkRows, &gather_rows);
    ensure_fixed_block_kernel_support("select_topk_vemr_metric_rows_f32", select_rows_caps, 256)?;
    ensure_fixed_block_kernel_support(
        "select_topk_vemr_metric_rows_from_candidates_f32",
        select_rows_candidates_caps,
        256,
    )?;
    ensure_fixed_block_kernel_support("gather_topk_metric_rows_f32", gather_rows_caps, 256)?;

    let objective_i = objective_to_vemr_kernel_id(objective);
    let rows_i = checked_i32(combo_count, "combo count")?;
    let metrics_i = checked_i32(VEMR_METRICS_STRIDE, "metrics stride")?;
    let topk_i = checked_i32(topk_select, "top_k")?;

    let blocks1 = (combo_count + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
    let cand_len1 = blocks1.saturating_mul(topk_select).max(topk_select);
    let top_metrics_len = topk_select.saturating_mul(VEMR_METRICS_STRIDE);
    VemrCudaScratch::ensure_len_i32(&mut scratch.d_topk_stage_a, cand_len1)?;
    VemrCudaScratch::ensure_len_i32(&mut scratch.d_topk_stage_b, cand_len1)?;
    VemrCudaScratch::ensure_len_i32(&mut scratch.d_top_combo, topk_select)?;
    VemrCudaScratch::ensure_len_i32(&mut scratch.d_top_dummy, topk_select)?;
    VemrCudaScratch::ensure_len_f32(&mut scratch.d_top_metrics, top_metrics_len)?;

    unsafe {
        let mut metrics_ptr = metrics_ptr_raw;
        let mut rows_i_mut = rows_i;
        let mut metrics_i_mut = metrics_i;
        let mut objective_i_mut = objective_i;
        let mut topk_i_mut = topk_i;
        let mut out_ptr = scratch
            .d_topk_stage_a
            .as_ref()
            .expect("VEMR top-k stage-a scratch")
            .as_device_ptr()
            .as_raw();

        let args: &mut [*mut c_void] = &mut [
            &mut metrics_ptr as *mut _ as *mut c_void,
            &mut rows_i_mut as *mut _ as *mut c_void,
            &mut metrics_i_mut as *mut _ as *mut c_void,
            &mut objective_i_mut as *mut _ as *mut c_void,
            &mut topk_i_mut as *mut _ as *mut c_void,
            &mut out_ptr as *mut _ as *mut c_void,
        ];
        rt.stream
            .launch(
                &select_rows,
                (u32::try_from(blocks1).unwrap_or(u32::MAX).max(1), 1, 1),
                (256, 1, 1),
                0,
                args,
            )
            .map_err(|e| e.to_string())?;
    }

    let mut blocks_prev = blocks1;
    let mut in_len = cand_len1;
    let mut in_is_a = true;
    while blocks_prev > 1 {
        let blocks_next = (in_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let out_len = blocks_next.saturating_mul(topk_select).max(topk_select);

        unsafe {
            let mut metrics_ptr = metrics_ptr_raw;
            let mut rows_i_mut = rows_i;
            let mut metrics_i_mut = metrics_i;
            let mut objective_i_mut = objective_i;
            let mut topk_i_mut = topk_i;
            let mut in_ptr = if in_is_a {
                scratch
                    .d_topk_stage_a
                    .as_ref()
                    .expect("VEMR top-k stage-a scratch")
                    .as_device_ptr()
                    .as_raw()
            } else {
                scratch
                    .d_topk_stage_b
                    .as_ref()
                    .expect("VEMR top-k stage-b scratch")
                    .as_device_ptr()
                    .as_raw()
            };
            let mut in_len_i = i32::try_from(in_len)
                .map_err(|_| "VEMR GPU candidate length exceeds i32 range".to_string())?;
            let mut out_ptr = if in_is_a {
                scratch
                    .d_topk_stage_b
                    .as_ref()
                    .expect("VEMR top-k stage-b scratch")
                    .as_device_ptr()
                    .as_raw()
            } else {
                scratch
                    .d_topk_stage_a
                    .as_ref()
                    .expect("VEMR top-k stage-a scratch")
                    .as_device_ptr()
                    .as_raw()
            };

            let args: &mut [*mut c_void] = &mut [
                &mut metrics_ptr as *mut _ as *mut c_void,
                &mut rows_i_mut as *mut _ as *mut c_void,
                &mut metrics_i_mut as *mut _ as *mut c_void,
                &mut objective_i_mut as *mut _ as *mut c_void,
                &mut topk_i_mut as *mut _ as *mut c_void,
                &mut in_ptr as *mut _ as *mut c_void,
                &mut in_len_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(
                    &select_rows_candidates,
                    (u32::try_from(blocks_next).unwrap_or(u32::MAX).max(1), 1, 1),
                    (256, 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        in_is_a = !in_is_a;
        blocks_prev = blocks_next;
        in_len = out_len;
    }

    unsafe {
        let mut metrics_ptr_mut = metrics_ptr_raw;
        let mut rows_i_mut = rows_i;
        let mut metrics_i_mut = metrics_i;
        let mut combo_tags_ptr_mut = combo_tags_ptr;
        let mut combo_dummy_ptr_mut = combo_dummy_ptr;
        let mut top_ptr = if in_is_a {
            scratch
                .d_topk_stage_a
                .as_ref()
                .expect("VEMR top-k stage-a scratch")
                .as_device_ptr()
                .as_raw()
        } else {
            scratch
                .d_topk_stage_b
                .as_ref()
                .expect("VEMR top-k stage-b scratch")
                .as_device_ptr()
                .as_raw()
        };
        let mut topk_i_mut = topk_i;
        let mut out_combo_ptr = scratch
            .d_top_combo
            .as_ref()
            .expect("VEMR top-k combo scratch")
            .as_device_ptr()
            .as_raw();
        let mut out_dummy_ptr = scratch
            .d_top_dummy
            .as_ref()
            .expect("VEMR top-k dummy scratch")
            .as_device_ptr()
            .as_raw();
        let mut out_metrics_ptr = scratch
            .d_top_metrics
            .as_ref()
            .expect("VEMR top-k metrics scratch")
            .as_device_ptr()
            .as_raw();

        let grid_x = ((topk_select as u32) + 255) / 256;
        let args: &mut [*mut c_void] = &mut [
            &mut metrics_ptr_mut as *mut _ as *mut c_void,
            &mut rows_i_mut as *mut _ as *mut c_void,
            &mut metrics_i_mut as *mut _ as *mut c_void,
            &mut combo_tags_ptr_mut as *mut _ as *mut c_void,
            &mut combo_dummy_ptr_mut as *mut _ as *mut c_void,
            &mut top_ptr as *mut _ as *mut c_void,
            &mut topk_i_mut as *mut _ as *mut c_void,
            &mut out_combo_ptr as *mut _ as *mut c_void,
            &mut out_dummy_ptr as *mut _ as *mut c_void,
            &mut out_metrics_ptr as *mut _ as *mut c_void,
        ];
        rt.stream
            .launch(&gather_rows, (grid_x.max(1), 1, 1), (256, 1, 1), 0, args)
            .map_err(|e| e.to_string())?;
    }

    rt.stream.synchronize().map_err(|e| e.to_string())?;

    scratch.ensure_host_top_combo_tags_len(topk_select);
    scratch.ensure_host_top_metrics_len(top_metrics_len);
    copy_prefix_to_slice(
        scratch
            .d_top_combo
            .as_ref()
            .expect("VEMR top-k combo scratch"),
        scratch.host_top_combo_tags.as_mut_slice(),
    )?;
    copy_prefix_to_slice(
        scratch
            .d_top_metrics
            .as_ref()
            .expect("VEMR top-k metrics scratch"),
        scratch.host_top_metrics.as_mut_slice(),
    )?;

    let mut top_rows = Vec::with_capacity(topk_select);
    for rank in 0..topk_select {
        let combo_tag = scratch.host_top_combo_tags[rank];
        if combo_tag <= 0 {
            continue;
        }
        let combo_index = usize::try_from(combo_tag - 1)
            .map_err(|_| "VEMR GPU combo index underflow".to_string())?;
        let base = rank.saturating_mul(VEMR_METRICS_STRIDE);
        let end = base.saturating_add(VEMR_METRICS_STRIDE);
        if end > scratch.host_top_metrics.len() {
            break;
        }
        top_rows.push(VemrGpuTopMetricRow {
            combo_index,
            metrics: decode_vemr_metrics_row(&scratch.host_top_metrics[base..end]),
        });
    }

    top_rows.sort_by(|a, b| {
        objective_score(&b.metrics, objective).total_cmp(&objective_score(&a.metrics, objective))
    });
    Ok(top_rows)
}

pub fn eval_vemr_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &VemrGpuDeviceSignalMatrices<'_>,
    combos: &VemrGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    vemr: &VemrStrategyParams,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    heatmap_bins: usize,
) -> Result<VemrGpuEvalResult, String> {
    validate_inputs(signals, combos)?;
    let combo_count = combos.z_row_index.len();
    if combo_count == 0 {
        return Ok(VemrGpuEvalResult::Full(Vec::new()));
    }

    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "VEMR GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let out_stride = VEMR_METRICS_STRIDE;
    let out_len = combo_count
        .checked_mul(out_stride)
        .ok_or_else(|| "VEMR GPU metrics buffer overflow".to_string())?;
    let topk_select = top_k.max(1);
    let use_device_topk = !include_all
        && heatmap_bins == 0
        && topk_select < combo_count
        && topk_select <= VEMR_DEVICE_TOPK_LIMIT;

    VEMR_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            let rt = VemrCudaRuntime::new(device_id)?;
            runtimes.insert(device_id, rt);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "VEMR GPU runtime missing".to_string())?;
        rt.set_current()?;
        let mut scratch = std::mem::take(&mut rt.scratch);
        let result = (|| {
            VemrCudaScratch::ensure_len_f32(&mut scratch.d_lr, signals.cols)?;
            VemrCudaScratch::ensure_len_i32(&mut scratch.d_combo_z, combo_count)?;
            VemrCudaScratch::ensure_len_i32(&mut scratch.d_combo_bb, combo_count)?;
            VemrCudaScratch::ensure_len_i32(&mut scratch.d_combo_atr, combo_count)?;
            VemrCudaScratch::ensure_len_i32(&mut scratch.d_combo_start, combo_count)?;
            VemrCudaScratch::ensure_len_i32(&mut scratch.d_combo_end, combo_count)?;
            VemrCudaScratch::ensure_len_f32(&mut scratch.d_out, out_len)?;

            let close_sig = VemrCloseSignature {
                len: signals.cols,
                host_ptr: signals.close_f32.as_ptr() as usize,
                device_ptr: signals.close_device_ptr,
            };
            let close_ptr = if signals.close_device_ptr != 0 {
                signals.close_device_ptr
            } else {
                VemrCudaScratch::ensure_len_f32(&mut scratch.d_close, signals.cols)?;
                let d_close = scratch.d_close.as_mut().expect("VEMR close scratch");
                if scratch.close_sig != Some(close_sig) {
                    copy_prefix_from_slice(d_close, signals.close_f32)?;
                }
                d_close.as_device_ptr().as_raw()
            };
            if scratch.close_sig != Some(close_sig) {
                let lr_ptr = scratch
                    .d_lr
                    .as_ref()
                    .expect("VEMR lr scratch")
                    .as_device_ptr()
                    .as_raw();
                launch_compute_log_returns_f32(rt, close_ptr, lr_ptr, signals.cols)?;
                scratch.close_sig = Some(close_sig);
                if signals.close_device_ptr != 0 {
                    scratch.d_close = None;
                }
            }

            let combo_z_sig = {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                combos.z_row_index.hash(&mut hasher);
                hasher.finish()
            };
            if scratch.combo_z_sig != Some(combo_z_sig) {
                copy_prefix_from_slice(
                    scratch.d_combo_z.as_mut().expect("VEMR combo z scratch"),
                    combos.z_row_index,
                )?;
                scratch.combo_z_sig = Some(combo_z_sig);
            }
            let combo_bb_sig = {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                combos.bb_row_index.hash(&mut hasher);
                hasher.finish()
            };
            if scratch.combo_bb_sig != Some(combo_bb_sig) {
                copy_prefix_from_slice(
                    scratch.d_combo_bb.as_mut().expect("VEMR combo bb scratch"),
                    combos.bb_row_index,
                )?;
                scratch.combo_bb_sig = Some(combo_bb_sig);
            }
            let combo_atr_sig = {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                combos.atr_row_index.hash(&mut hasher);
                hasher.finish()
            };
            if scratch.combo_atr_sig != Some(combo_atr_sig) {
                copy_prefix_from_slice(
                    scratch
                        .d_combo_atr
                        .as_mut()
                        .expect("VEMR combo atr scratch"),
                    combos.atr_row_index,
                )?;
                scratch.combo_atr_sig = Some(combo_atr_sig);
            }
            copy_prefix_from_slice(
                scratch
                    .d_combo_start
                    .as_mut()
                    .expect("VEMR combo start scratch"),
                combos.start_t,
            )?;
            copy_prefix_from_slice(
                scratch
                    .d_combo_end
                    .as_mut()
                    .expect("VEMR combo end scratch"),
                combos.end_t,
            )?;

            let lr_ptr = scratch
                .d_lr
                .as_ref()
                .expect("VEMR lr scratch")
                .as_device_ptr()
                .as_raw();

            let kernel = rt
                .module
                .get_function("vemr_backtest_metrics_f32")
                .map_err(|e| e.to_string())?;
            let kernel_caps = rt.launch_caps_for_kernel(VemrKernelKey::EvalMetrics, &kernel);
            let launch = choose_launch_shape_1d_from_candidates(
                combo_count,
                rt.device_caps,
                kernel_caps,
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

            let mut close_ptr_mut = close_ptr;
            let mut lr_ptr_mut = lr_ptr;
            let mut zscore_ptr = signals.zscore_row_major_ptr;
            let mut bb_upper_ptr = signals.bb_upper_row_major_ptr;
            let mut bb_middle_ptr = signals.bb_middle_row_major_ptr;
            let mut bb_lower_ptr = signals.bb_lower_row_major_ptr;
            let mut atr_ptr = signals.atr_row_major_ptr;
            let mut combo_z_ptr = scratch
                .d_combo_z
                .as_ref()
                .expect("VEMR combo z scratch")
                .as_device_ptr()
                .as_raw();
            let mut combo_bb_ptr = scratch
                .d_combo_bb
                .as_ref()
                .expect("VEMR combo bb scratch")
                .as_device_ptr()
                .as_raw();
            let mut combo_atr_ptr = scratch
                .d_combo_atr
                .as_ref()
                .expect("VEMR combo atr scratch")
                .as_device_ptr()
                .as_raw();
            let mut combo_start_ptr = scratch
                .d_combo_start
                .as_ref()
                .expect("VEMR combo start scratch")
                .as_device_ptr()
                .as_raw();
            let mut combo_end_ptr = scratch
                .d_combo_end
                .as_ref()
                .expect("VEMR combo end scratch")
                .as_device_ptr()
                .as_raw();
            let mut out_ptr = scratch
                .d_out
                .as_ref()
                .expect("VEMR output scratch")
                .as_device_ptr()
                .as_raw();

            let mut t_i = checked_i32(signals.cols, "cols")?;
            let mut z_rows_i = checked_i32(signals.zscore_rows, "zscore rows")?;
            let mut z_stride_i = checked_i32(signals.zscore_stride, "zscore stride")?;
            let mut bb_rows_i = checked_i32(signals.bb_rows, "bb rows")?;
            let mut bb_stride_i = checked_i32(signals.bb_stride, "bb stride")?;
            let mut atr_rows_i = checked_i32(signals.atr_rows, "atr rows")?;
            let mut atr_stride_i = checked_i32(signals.atr_stride, "atr stride")?;
            let mut combo_count_i = checked_i32(combo_count, "combo count")?;
            let mut flags_u = flags;
            let mut commission_f = commission;
            let mut z_entry_f = vemr.z_entry as f32;
            let mut z_exit_f = vemr.z_exit as f32;
            let mut atr_stop_mult_f = vemr.atr_stop_mult as f32;
            let mut require_band_touch_i = i32::from(vemr.require_band_touch);
            let mut confirm_bars_i = i32::from(vemr.confirm_bars);
            let mut max_trade_bars_i = i32::from(vemr.max_trade_bars);
            let mut out_stride_i = checked_i32(out_stride, "metrics stride")?;

            unsafe {
                let args: &mut [*mut c_void] = &mut [
                    &mut close_ptr_mut as *mut _ as *mut c_void,
                    &mut lr_ptr_mut as *mut _ as *mut c_void,
                    &mut t_i as *mut _ as *mut c_void,
                    &mut zscore_ptr as *mut _ as *mut c_void,
                    &mut z_rows_i as *mut _ as *mut c_void,
                    &mut z_stride_i as *mut _ as *mut c_void,
                    &mut bb_upper_ptr as *mut _ as *mut c_void,
                    &mut bb_middle_ptr as *mut _ as *mut c_void,
                    &mut bb_lower_ptr as *mut _ as *mut c_void,
                    &mut bb_rows_i as *mut _ as *mut c_void,
                    &mut bb_stride_i as *mut _ as *mut c_void,
                    &mut atr_ptr as *mut _ as *mut c_void,
                    &mut atr_rows_i as *mut _ as *mut c_void,
                    &mut atr_stride_i as *mut _ as *mut c_void,
                    &mut combo_z_ptr as *mut _ as *mut c_void,
                    &mut combo_bb_ptr as *mut _ as *mut c_void,
                    &mut combo_atr_ptr as *mut _ as *mut c_void,
                    &mut combo_start_ptr as *mut _ as *mut c_void,
                    &mut combo_end_ptr as *mut _ as *mut c_void,
                    &mut combo_count_i as *mut _ as *mut c_void,
                    &mut flags_u as *mut _ as *mut c_void,
                    &mut commission_f as *mut _ as *mut c_void,
                    &mut z_entry_f as *mut _ as *mut c_void,
                    &mut z_exit_f as *mut _ as *mut c_void,
                    &mut atr_stop_mult_f as *mut _ as *mut c_void,
                    &mut require_band_touch_i as *mut _ as *mut c_void,
                    &mut confirm_bars_i as *mut _ as *mut c_void,
                    &mut max_trade_bars_i as *mut _ as *mut c_void,
                    &mut out_stride_i as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                rt.stream
                    .launch(
                        &kernel,
                        (launch.grid_x, 1, 1),
                        (launch.block_x, 1, 1),
                        0,
                        args,
                    )
                    .map_err(|e| e.to_string())?;
            }

            if use_device_topk {
                VemrCudaScratch::ensure_len_i32(&mut scratch.d_combo_tags, combo_count)?;
                if scratch.combo_tags_len < combo_count {
                    let combo_tags: Vec<i32> = (0..combo_count)
                        .map(|idx| checked_i32(idx + 1, "combo index"))
                        .collect::<Result<_, _>>()?;
                    copy_prefix_from_slice(
                        scratch
                            .d_combo_tags
                            .as_mut()
                            .expect("VEMR combo tag scratch"),
                        combo_tags.as_slice(),
                    )?;
                    scratch.combo_tags_len = combo_count;
                }
                let combo_tags_ptr = scratch
                    .d_combo_tags
                    .as_ref()
                    .expect("VEMR combo tag scratch")
                    .as_device_ptr()
                    .as_raw();
                let combo_start_ptr = scratch
                    .d_combo_start
                    .as_ref()
                    .expect("VEMR combo start scratch")
                    .as_device_ptr()
                    .as_raw();
                let out_ptr = scratch
                    .d_out
                    .as_ref()
                    .expect("VEMR output scratch")
                    .as_device_ptr()
                    .as_raw();
                let top_rows = reduce_vemr_topk_on_device(
                    rt,
                    &mut scratch,
                    combo_tags_ptr,
                    combo_start_ptr,
                    out_ptr,
                    combo_count,
                    objective,
                    topk_select,
                )?;
                return Ok(VemrGpuEvalResult::TopK(top_rows));
            }

            rt.stream.synchronize().map_err(|e| e.to_string())?;

            scratch.ensure_host_out_len(out_len);
            let mut host_out = std::mem::take(&mut scratch.host_out);
            copy_prefix_to_slice(
                scratch.d_out.as_ref().expect("VEMR output scratch"),
                &mut host_out[..out_len],
            )?;

            let mut metrics = Vec::with_capacity(combo_count);
            for i in 0..combo_count {
                let base = i * out_stride;
                metrics.push(decode_vemr_metrics_row(
                    &host_out[base..base + VEMR_METRICS_STRIDE],
                ));
            }
            scratch.host_out = host_out;
            Ok(VemrGpuEvalResult::Full(metrics))
        })();
        rt.scratch = scratch;
        result
    })
}

#[cfg(test)]
mod tests {
    use super::{validate_inputs, VemrGpuComboBatch, VemrGpuDeviceSignalMatrices};

    fn valid_signals() -> VemrGpuDeviceSignalMatrices<'static> {
        VemrGpuDeviceSignalMatrices {
            close_f32: &[1.0, 2.0, 3.0, 4.0],
            close_device_ptr: 0,
            cols: 4,
            zscore_row_major_ptr: 0x1000,
            zscore_rows: 2,
            zscore_stride: 4,
            bb_upper_row_major_ptr: 0x2000,
            bb_middle_row_major_ptr: 0x3000,
            bb_lower_row_major_ptr: 0x4000,
            bb_rows: 2,
            bb_stride: 4,
            atr_row_major_ptr: 0x5000,
            atr_rows: 2,
            atr_stride: 4,
        }
    }

    fn valid_combos() -> VemrGpuComboBatch<'static> {
        VemrGpuComboBatch {
            z_row_index: &[0, 1],
            bb_row_index: &[0, 1],
            atr_row_index: &[0, 1],
            start_t: &[1, 2],
            end_t: &[3, 4],
        }
    }

    #[test]
    fn validate_inputs_rejects_out_of_bounds_rows() {
        let signals = valid_signals();
        let combos = VemrGpuComboBatch {
            z_row_index: &[2],
            bb_row_index: &[0],
            atr_row_index: &[0],
            start_t: &[1],
            end_t: &[3],
        };
        let err = validate_inputs(&signals, &combos).expect_err("expected row bounds error");
        assert!(err.contains("out of bounds"));
    }

    #[test]
    fn validate_inputs_rejects_invalid_eval_ranges() {
        let signals = valid_signals();
        let combos = VemrGpuComboBatch {
            start_t: &[3],
            end_t: &[3],
            z_row_index: &[0],
            bb_row_index: &[0],
            atr_row_index: &[0],
        };
        let err = validate_inputs(&signals, &combos).expect_err("expected range error");
        assert!(err.contains("invalid eval range"));
    }

    #[test]
    fn validate_inputs_rejects_short_stride() {
        let mut signals = valid_signals();
        signals.bb_stride = 3;
        let err = validate_inputs(&signals, &valid_combos()).expect_err("expected stride error");
        assert!(err.contains("stride"));
    }
}
