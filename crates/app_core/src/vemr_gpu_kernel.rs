#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
};
use crate::vemr::VemrStrategyParams;
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
use ta_optimizer::ObjectiveKind;
use ta_strategies::double_ma::{Metrics, StrategyConfig};

const VEMR_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_METRICS_STRIDE;
const TOPK_BLOCK_ITEMS: usize = 512;
const VEMR_DEVICE_TOPK_LIMIT: usize = 256;

pub struct VemrGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
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

struct VemrCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
}

impl VemrCudaRuntime {
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

    let combos_len = combos.z_row_index.len();
    if combos.bb_row_index.len() != combos_len
        || combos.atr_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("VEMR GPU combo vector length mismatch".to_string());
    }
    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut VemrCudaRuntime,
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
        i32::try_from(len).map_err(|_| "VEMR GPU close length exceeds i32 range".to_string())?;
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
    d_combo_tags: &DeviceBuffer<i32>,
    d_combo_dummy: &DeviceBuffer<i32>,
    d_metrics: &DeviceBuffer<f32>,
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

    let objective_i = objective_to_vemr_kernel_id(objective);
    let rows_i = i32::try_from(combo_count)
        .map_err(|_| "VEMR GPU combo_count exceeds i32 range".to_string())?;
    let metrics_i = i32::try_from(VEMR_METRICS_STRIDE)
        .map_err(|_| "VEMR GPU metrics stride exceeds i32 range".to_string())?;
    let topk_i =
        i32::try_from(topk_select).map_err(|_| "VEMR GPU top_k exceeds i32 range".to_string())?;

    let blocks1 = (combo_count + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
    let cand_len1 = blocks1.saturating_mul(topk_select).max(topk_select);
    let d_cand_a =
        unsafe { DeviceBuffer::<i32>::uninitialized(cand_len1) }.map_err(|e| e.to_string())?;
    let d_cand_b =
        unsafe { DeviceBuffer::<i32>::uninitialized(cand_len1) }.map_err(|e| e.to_string())?;
    let d_top_combo =
        unsafe { DeviceBuffer::<i32>::uninitialized(topk_select) }.map_err(|e| e.to_string())?;
    let d_top_dummy =
        unsafe { DeviceBuffer::<i32>::uninitialized(topk_select) }.map_err(|e| e.to_string())?;
    let d_top_metrics = unsafe {
        DeviceBuffer::<f32>::uninitialized(topk_select.saturating_mul(VEMR_METRICS_STRIDE))
    }
    .map_err(|e| e.to_string())?;

    unsafe {
        let mut metrics_ptr = d_metrics.as_device_ptr().as_raw();
        let mut rows_i_mut = rows_i;
        let mut metrics_i_mut = metrics_i;
        let mut objective_i_mut = objective_i;
        let mut topk_i_mut = topk_i;
        let mut out_ptr = d_cand_a.as_device_ptr().as_raw();

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
            let mut metrics_ptr = d_metrics.as_device_ptr().as_raw();
            let mut rows_i_mut = rows_i;
            let mut metrics_i_mut = metrics_i;
            let mut objective_i_mut = objective_i;
            let mut topk_i_mut = topk_i;
            let mut in_ptr = if in_is_a {
                d_cand_a.as_device_ptr().as_raw()
            } else {
                d_cand_b.as_device_ptr().as_raw()
            };
            let mut in_len_i = i32::try_from(in_len)
                .map_err(|_| "VEMR GPU candidate length exceeds i32 range".to_string())?;
            let mut out_ptr = if in_is_a {
                d_cand_b.as_device_ptr().as_raw()
            } else {
                d_cand_a.as_device_ptr().as_raw()
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
        let mut metrics_ptr = d_metrics.as_device_ptr().as_raw();
        let mut rows_i_mut = rows_i;
        let mut metrics_i_mut = metrics_i;
        let mut combo_tags_ptr = d_combo_tags.as_device_ptr().as_raw();
        let mut combo_dummy_ptr = d_combo_dummy.as_device_ptr().as_raw();
        let mut top_ptr = if in_is_a {
            d_cand_a.as_device_ptr().as_raw()
        } else {
            d_cand_b.as_device_ptr().as_raw()
        };
        let mut topk_i_mut = topk_i;
        let mut out_combo_ptr = d_top_combo.as_device_ptr().as_raw();
        let mut out_dummy_ptr = d_top_dummy.as_device_ptr().as_raw();
        let mut out_metrics_ptr = d_top_metrics.as_device_ptr().as_raw();

        let grid_x = ((topk_select as u32) + 255) / 256;
        let args: &mut [*mut c_void] = &mut [
            &mut metrics_ptr as *mut _ as *mut c_void,
            &mut rows_i_mut as *mut _ as *mut c_void,
            &mut metrics_i_mut as *mut _ as *mut c_void,
            &mut combo_tags_ptr as *mut _ as *mut c_void,
            &mut combo_dummy_ptr as *mut _ as *mut c_void,
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

    let mut host_combo_tags = vec![0i32; topk_select];
    let mut host_top_metrics = vec![0.0f32; topk_select.saturating_mul(VEMR_METRICS_STRIDE)];
    d_top_combo
        .copy_to(&mut host_combo_tags)
        .map_err(|e| e.to_string())?;
    d_top_metrics
        .copy_to(&mut host_top_metrics)
        .map_err(|e| e.to_string())?;

    let mut top_rows = Vec::with_capacity(topk_select);
    for rank in 0..topk_select {
        let combo_tag = host_combo_tags[rank];
        if combo_tag <= 0 {
            continue;
        }
        let combo_index = usize::try_from(combo_tag - 1)
            .map_err(|_| "VEMR GPU combo index underflow".to_string())?;
        let base = rank.saturating_mul(VEMR_METRICS_STRIDE);
        let end = base.saturating_add(VEMR_METRICS_STRIDE);
        if end > host_top_metrics.len() {
            break;
        }
        top_rows.push(VemrGpuTopMetricRow {
            combo_index,
            metrics: decode_vemr_metrics_row(&host_top_metrics[base..end]),
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

        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(signals.cols) }
            .map_err(|e| e.to_string())?;
        let d_combo_z = DeviceBuffer::from_slice(combos.z_row_index).map_err(|e| e.to_string())?;
        let d_combo_bb =
            DeviceBuffer::from_slice(combos.bb_row_index).map_err(|e| e.to_string())?;
        let d_combo_atr =
            DeviceBuffer::from_slice(combos.atr_row_index).map_err(|e| e.to_string())?;
        let d_combo_start = DeviceBuffer::from_slice(combos.start_t).map_err(|e| e.to_string())?;
        let d_combo_end = DeviceBuffer::from_slice(combos.end_t).map_err(|e| e.to_string())?;
        let d_out =
            unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }.map_err(|e| e.to_string())?;

        let d_close = DeviceBuffer::from_slice(signals.close_f32).map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, device_id, &d_close, &mut d_lr, signals.cols)?;

        let kernel = rt
            .module
            .get_function("vemr_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let device_caps = detect_device_launch_caps(device);
        let kernel_caps = detect_kernel_launch_caps(&kernel, device_caps);
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            device_caps,
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

        let mut close_ptr = d_close.as_device_ptr().as_raw();
        let mut lr_ptr = d_lr.as_device_ptr().as_raw();
        let mut zscore_ptr = signals.zscore_row_major_ptr;
        let mut bb_upper_ptr = signals.bb_upper_row_major_ptr;
        let mut bb_middle_ptr = signals.bb_middle_row_major_ptr;
        let mut bb_lower_ptr = signals.bb_lower_row_major_ptr;
        let mut atr_ptr = signals.atr_row_major_ptr;
        let mut combo_z_ptr = d_combo_z.as_device_ptr().as_raw();
        let mut combo_bb_ptr = d_combo_bb.as_device_ptr().as_raw();
        let mut combo_atr_ptr = d_combo_atr.as_device_ptr().as_raw();
        let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
        let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
        let mut out_ptr = d_out.as_device_ptr().as_raw();

        let mut t_i = i32::try_from(signals.cols)
            .map_err(|_| "VEMR GPU cols exceed i32 range".to_string())?;
        let mut z_rows_i = i32::try_from(signals.zscore_rows)
            .map_err(|_| "VEMR GPU zscore rows exceed i32 range".to_string())?;
        let mut z_stride_i = i32::try_from(signals.zscore_stride)
            .map_err(|_| "VEMR GPU zscore stride exceed i32 range".to_string())?;
        let mut bb_rows_i = i32::try_from(signals.bb_rows)
            .map_err(|_| "VEMR GPU bb rows exceed i32 range".to_string())?;
        let mut bb_stride_i = i32::try_from(signals.bb_stride)
            .map_err(|_| "VEMR GPU bb stride exceed i32 range".to_string())?;
        let mut atr_rows_i = i32::try_from(signals.atr_rows)
            .map_err(|_| "VEMR GPU atr rows exceed i32 range".to_string())?;
        let mut atr_stride_i = i32::try_from(signals.atr_stride)
            .map_err(|_| "VEMR GPU atr stride exceed i32 range".to_string())?;
        let mut combo_count_i = i32::try_from(combo_count)
            .map_err(|_| "VEMR GPU combo_count exceed i32 range".to_string())?;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut z_entry_f = vemr.z_entry as f32;
        let mut z_exit_f = vemr.z_exit as f32;
        let mut atr_stop_mult_f = vemr.atr_stop_mult as f32;
        let mut require_band_touch_i = i32::from(vemr.require_band_touch);
        let mut confirm_bars_i = i32::from(vemr.confirm_bars);
        let mut max_trade_bars_i = i32::from(vemr.max_trade_bars);
        let mut out_stride_i = out_stride as i32;

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
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
            let combo_tags: Vec<i32> = (0..combo_count)
                .map(|idx| {
                    i32::try_from(idx + 1)
                        .map_err(|_| "VEMR GPU combo index exceeds i32 range".to_string())
                })
                .collect::<Result<_, _>>()?;
            let d_combo_tags =
                DeviceBuffer::from_slice(combo_tags.as_slice()).map_err(|e| e.to_string())?;
            let top_rows = reduce_vemr_topk_on_device(
                rt,
                &d_combo_tags,
                &d_combo_start,
                &d_out,
                combo_count,
                objective,
                topk_select,
            )?;
            return Ok(VemrGpuEvalResult::TopK(top_rows));
        }

        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut host_out = vec![0.0f32; out_len];
        d_out.copy_to(&mut host_out).map_err(|e| e.to_string())?;

        let mut metrics = Vec::with_capacity(combo_count);
        for i in 0..combo_count {
            let base = i * out_stride;
            metrics.push(decode_vemr_metrics_row(
                &host_out[base..base + VEMR_METRICS_STRIDE],
            ));
        }
        Ok(VemrGpuEvalResult::Full(metrics))
    })
}
