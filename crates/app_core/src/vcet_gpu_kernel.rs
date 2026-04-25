#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::composite_gpu_metric_decode::{
    decode_non_double_ma_gpu_metrics_row, NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE,
};
use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    load_double_crossover_module, DeviceLaunchCaps, KernelLaunchCaps,
};
use crate::indicator_catalog::IndicatorInputKind;
use crate::standalone_gpu_runtime::{
    checked_i32, ensure_device_buffer_len, ensure_host_f32_len, slice_signature, CloseSeriesKey,
    ResidentCloseReturns, UploadSignature,
};
use crate::vcet::{
    compute_vcet_chandelier_signal_cuda_device, compute_vcet_start_t,
    compute_vcet_vidya_signal_cuda_device, resolve_vcet_periods, VcetOwnedDeviceMatrix,
    VcetStrategyParams,
};
use crate::PreparedCudaIndicatorInputs;
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
use ta_strategies::double_ma::{DoubleMaParams, Metrics, StrategyConfig};
use vector_ta::utilities::data_loader::Candles;

const VCET_METRICS_STRIDE: usize = NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE;

const STRAT_LONG_ONLY: u32 = 1u32 << 0;
const STRAT_NO_FLIP: u32 = 1u32 << 1;
const STRAT_TRADE_ON_NEXT_BAR: u32 = 1u32 << 2;
const STRAT_SIGNED_EXPOSURE: u32 = 1u32 << 4;

pub struct VcetGpuDeviceSignalMatrices<'a> {
    pub close_f32: &'a [f32],
    pub close_device_ptr: u64,
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

pub struct VcetGpuOwnedSignalTables {
    cols: usize,
    vidya_rows_ptr: Vec<u64>,
    ce_long_rows_ptr: Vec<u64>,
    ce_short_rows_ptr: Vec<u64>,
    vidya_rows_keepalive: Vec<VcetOwnedDeviceMatrix>,
    ce_long_rows_keepalive: Vec<VcetOwnedDeviceMatrix>,
    ce_short_rows_keepalive: Vec<VcetOwnedDeviceMatrix>,
}

impl VcetGpuOwnedSignalTables {
    pub fn from_owned_rows(
        cols: usize,
        vidya_rows_keepalive: Vec<VcetOwnedDeviceMatrix>,
        ce_long_rows_keepalive: Vec<VcetOwnedDeviceMatrix>,
        ce_short_rows_keepalive: Vec<VcetOwnedDeviceMatrix>,
    ) -> Result<Self, String> {
        if cols == 0 {
            return Err("VCET GPU owned signal tables require non-zero cols".to_string());
        }
        if ce_long_rows_keepalive.len() != ce_short_rows_keepalive.len() {
            return Err("VCET GPU chandelier owner row-count mismatch".to_string());
        }

        fn collect_row_ptrs(
            rows: &[VcetOwnedDeviceMatrix],
            expected_cols: usize,
            label: &str,
        ) -> Result<Vec<u64>, String> {
            if rows.is_empty() {
                return Err(format!("VCET GPU {label} owner rows cannot be empty"));
            }

            let mut ptrs = Vec::with_capacity(rows.len());
            for (idx, row) in rows.iter().enumerate() {
                if row.rows != 1 {
                    return Err(format!(
                        "VCET GPU {label} row[{idx}] must have rows=1, got {}",
                        row.rows
                    ));
                }
                if row.cols != expected_cols {
                    return Err(format!(
                        "VCET GPU {label} row[{idx}] cols mismatch: expected {expected_cols}, got {}",
                        row.cols
                    ));
                }
                ptrs.push(row.device_ptr());
            }
            Ok(ptrs)
        }

        let vidya_rows_ptr = collect_row_ptrs(&vidya_rows_keepalive, cols, "VIDYA")?;
        let ce_long_rows_ptr = collect_row_ptrs(&ce_long_rows_keepalive, cols, "CE long")?;
        let ce_short_rows_ptr = collect_row_ptrs(&ce_short_rows_keepalive, cols, "CE short")?;

        Ok(Self {
            cols,
            vidya_rows_ptr,
            ce_long_rows_ptr,
            ce_short_rows_ptr,
            vidya_rows_keepalive,
            ce_long_rows_keepalive,
            ce_short_rows_keepalive,
        })
    }

    pub fn as_signal_matrices<'a>(
        &'a self,
        close_f32: &'a [f32],
        close_device_ptr: u64,
    ) -> VcetGpuDeviceSignalMatrices<'a> {
        let _ = (
            &self.vidya_rows_keepalive,
            &self.ce_long_rows_keepalive,
            &self.ce_short_rows_keepalive,
        );
        VcetGpuDeviceSignalMatrices {
            close_f32,
            close_device_ptr,
            cols: self.cols,
            vidya_rows_ptr: &self.vidya_rows_ptr,
            ce_long_rows_ptr: &self.ce_long_rows_ptr,
            ce_short_rows_ptr: &self.ce_short_rows_ptr,
            vidya_rows: self.vidya_rows_ptr.len(),
            ce_rows: self.ce_long_rows_ptr.len(),
        }
    }
}

pub struct PreparedVcetGpuKernelBatch {
    prepared_device_id: u32,
    prepared_cuda_inputs: PreparedCudaIndicatorInputs,
    signal_tables: VcetGpuOwnedSignalTables,
    combo_vidya_row_idx: Vec<i32>,
    combo_ce_row_idx: Vec<i32>,
    combo_start_t: Vec<i32>,
    combo_end_t: Vec<i32>,
    combo_params: Vec<DoubleMaParams>,
}

impl PreparedVcetGpuKernelBatch {
    pub fn prepare(
        candles: &Candles,
        ma_source: &str,
        fast_periods: &[u16],
        slow_periods: &[u16],
        strategy: &StrategyConfig,
        vcet: &VcetStrategyParams,
        device_id: u32,
    ) -> Result<Self, String> {
        if candles.close.len() < 2 {
            return Err("VCET GPU batch requires at least 2 candles".to_string());
        }
        if fast_periods.is_empty() || slow_periods.is_empty() {
            return Err("VCET GPU batch requires non-empty period ranges".to_string());
        }

        let n = candles.close.len();
        let fv_close = first_finite_idx_f64(candles.close.as_slice())
            .ok_or_else(|| "close series all non-finite".to_string())?;
        let fv_high = first_finite_idx_f64(candles.high.as_slice())
            .ok_or_else(|| "high series all non-finite".to_string())?;
        let fv_low = first_finite_idx_f64(candles.low.as_slice())
            .ok_or_else(|| "low series all non-finite".to_string())?;
        let prepared_cuda_inputs =
            PreparedCudaIndicatorInputs::from_candles_on_device_for_input_kinds(
                candles,
                ma_source,
                device_id as usize,
                &[IndicatorInputKind::Ohlc],
            )
            .map_err(|e| e.to_string())?;

        let mut vidya_keys: Vec<(u16, u16)> = Vec::new();
        let mut ce_keys: Vec<u16> = Vec::new();
        for &fast_len in fast_periods {
            for &slow_len in slow_periods {
                if fast_len >= slow_len {
                    continue;
                }
                let periods = resolve_vcet_periods(vcet, fast_len, slow_len);
                vidya_keys.push((periods.vidya_short, periods.vidya_long));
                ce_keys.push(periods.ce);
            }
        }
        vidya_keys.sort_unstable();
        vidya_keys.dedup();
        ce_keys.sort_unstable();
        ce_keys.dedup();
        if vidya_keys.is_empty() || ce_keys.is_empty() {
            return Err("VCET GPU batch produced no valid period combinations".to_string());
        }

        let mut vidya_lookup: HashMap<(u16, u16), (i32, usize)> =
            HashMap::with_capacity(vidya_keys.len());
        let mut ce_lookup: HashMap<u16, (i32, usize)> = HashMap::with_capacity(ce_keys.len());
        let mut vidya_keepalive: Vec<VcetOwnedDeviceMatrix> = Vec::with_capacity(vidya_keys.len());
        let mut ce_long_keepalive: Vec<VcetOwnedDeviceMatrix> = Vec::with_capacity(ce_keys.len());
        let mut ce_short_keepalive: Vec<VcetOwnedDeviceMatrix> = Vec::with_capacity(ce_keys.len());

        for &(vidya_short, vidya_long) in &vidya_keys {
            let matrix = compute_vcet_vidya_signal_cuda_device(
                &prepared_cuda_inputs,
                crate::vcet::VcetResolvedPeriods {
                    vidya_short,
                    vidya_long,
                    ce: 1,
                },
                vcet.vidya_alpha,
            )?;
            if matrix.rows != 1 || matrix.cols != n {
                return Err("VCET GPU VIDYA matrix shape mismatch".to_string());
            }
            let idx = i32::try_from(vidya_keepalive.len())
                .map_err(|_| "VCET GPU VIDYA row index exceeds i32 range".to_string())?;
            let first_valid = usize::from(vidya_long.saturating_sub(1));
            vidya_lookup.insert((vidya_short, vidya_long), (idx, first_valid));
            vidya_keepalive.push(matrix);
        }

        for &ce_period in &ce_keys {
            let (long_matrix, short_matrix) = compute_vcet_chandelier_signal_cuda_device(
                &prepared_cuda_inputs,
                ce_period,
                vcet.ce_mult,
                vcet.ce_use_close,
            )?;
            if long_matrix.rows != 1
                || short_matrix.rows != 1
                || long_matrix.cols != n
                || short_matrix.cols != n
            {
                return Err("VCET GPU chandelier matrix shape mismatch".to_string());
            }
            let idx = i32::try_from(ce_long_keepalive.len())
                .map_err(|_| "VCET GPU chandelier row index exceeds i32 range".to_string())?;
            let first_valid = usize::from(ce_period.saturating_sub(1));
            ce_lookup.insert(ce_period, (idx, first_valid));
            ce_long_keepalive.push(long_matrix);
            ce_short_keepalive.push(short_matrix);
        }

        let mut combo_vidya_row_idx: Vec<i32> = Vec::new();
        let mut combo_ce_row_idx: Vec<i32> = Vec::new();
        let mut combo_start_t: Vec<i32> = Vec::new();
        let mut combo_params: Vec<DoubleMaParams> = Vec::new();
        for &fast_len in fast_periods {
            for &slow_len in slow_periods {
                if fast_len >= slow_len {
                    continue;
                }
                let periods = resolve_vcet_periods(vcet, fast_len, slow_len);
                let (vidya_row, vidya_first_valid) = *vidya_lookup
                    .get(&(periods.vidya_short, periods.vidya_long))
                    .ok_or_else(|| "VCET GPU VIDYA row lookup miss".to_string())?;
                let (ce_row, ce_first_valid) = *ce_lookup
                    .get(&periods.ce)
                    .ok_or_else(|| "VCET GPU chandelier row lookup miss".to_string())?;
                let start_t = compute_vcet_start_t(
                    fv_close,
                    fv_high,
                    fv_low,
                    vidya_first_valid,
                    ce_first_valid,
                    strategy.trade_on_next_bar,
                );
                if start_t >= n {
                    continue;
                }

                combo_vidya_row_idx.push(vidya_row);
                combo_ce_row_idx.push(ce_row);
                combo_start_t.push(
                    i32::try_from(start_t)
                        .map_err(|_| "VCET GPU start_t exceeds i32 range".to_string())?,
                );
                combo_params.push(DoubleMaParams {
                    fast_len,
                    slow_len,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                });
            }
        }

        if combo_params.is_empty() {
            return Err("VCET GPU batch produced no valid parameter combinations".to_string());
        }

        let combo_end_t = vec![
            i32::try_from(n)
                .map_err(|_| "VCET end_t exceeds i32 range".to_string())?;
            combo_vidya_row_idx.len()
        ];
        let signal_tables = VcetGpuOwnedSignalTables::from_owned_rows(
            n,
            vidya_keepalive,
            ce_long_keepalive,
            ce_short_keepalive,
        )?;

        Ok(Self {
            prepared_device_id: device_id,
            prepared_cuda_inputs,
            signal_tables,
            combo_vidya_row_idx,
            combo_ce_row_idx,
            combo_start_t,
            combo_end_t,
            combo_params,
        })
    }

    pub fn combo_count(&self) -> usize {
        self.combo_params.len()
    }

    pub fn combo_params(&self) -> &[DoubleMaParams] {
        self.combo_params.as_slice()
    }

    pub fn eval_metrics(
        &self,
        device_id: u32,
        strategy: &StrategyConfig,
        vcet: &VcetStrategyParams,
    ) -> Result<Vec<Metrics>, String> {
        validate_prepared_device_id(self.prepared_device_id, device_id)?;
        let signals = self.signal_tables.as_signal_matrices(
            self.prepared_cuda_inputs.close(),
            self.prepared_cuda_inputs
                .close_device()
                .buffer()
                .as_device_ptr()
                .as_raw(),
        );
        let combos = VcetGpuComboBatch {
            vidya_row_index: self.combo_vidya_row_idx.as_slice(),
            ce_row_index: self.combo_ce_row_idx.as_slice(),
            start_t: self.combo_start_t.as_slice(),
            end_t: self.combo_end_t.as_slice(),
        };
        eval_vcet_metrics_gpu_kernel_device_rows(device_id, &signals, &combos, strategy, vcet)
    }
}

fn first_finite_idx_f64(values: &[f64]) -> Option<usize> {
    values.iter().position(|value| value.is_finite())
}

fn validate_prepared_device_id(
    prepared_device_id: u32,
    requested_device_id: u32,
) -> Result<(), String> {
    if prepared_device_id != requested_device_id {
        return Err(format!(
            "VCET GPU batch was prepared on device {prepared_device_id} but eval_metrics requested device {requested_device_id}"
        ));
    }
    Ok(())
}

#[derive(Default)]
struct VcetKernelScratch {
    d_vidya_rows: Option<DeviceBuffer<u64>>,
    d_ce_long_rows: Option<DeviceBuffer<u64>>,
    d_ce_short_rows: Option<DeviceBuffer<u64>>,
    d_combo_vidya: Option<DeviceBuffer<i32>>,
    d_combo_ce: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    host_metrics: Vec<f32>,
    vidya_rows_sig: Option<UploadSignature>,
    ce_long_rows_sig: Option<UploadSignature>,
    ce_short_rows_sig: Option<UploadSignature>,
    combo_vidya_sig: Option<UploadSignature>,
    combo_ce_sig: Option<UploadSignature>,
    combo_start_sig: Option<UploadSignature>,
    combo_end_sig: Option<UploadSignature>,
}

impl VcetKernelScratch {
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
enum VcetKernelKey {
    ComputeReturns,
    Backtest,
}

struct VcetCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    resident_series: Option<ResidentCloseReturns>,
    scratch: VcetKernelScratch,
    kernel_caps: RefCell<HashMap<VcetKernelKey, KernelLaunchCaps>>,
}

impl VcetCudaRuntime {
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
            scratch: VcetKernelScratch::default(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for(&self, key: VcetKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
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
    let n_i32 = checked_i32("VCET GPU cols", n)?;

    if signals.vidya_rows == 0 || signals.ce_rows == 0 {
        return Err("VCET GPU row counts must be non-zero".to_string());
    }
    checked_i32("VCET GPU VIDYA row count", signals.vidya_rows)?;
    checked_i32("VCET GPU CE row count", signals.ce_rows)?;
    if signals.vidya_rows_ptr.len() != signals.vidya_rows
        || signals.ce_long_rows_ptr.len() != signals.ce_rows
        || signals.ce_short_rows_ptr.len() != signals.ce_rows
    {
        return Err("VCET GPU row-pointer vector length mismatch".to_string());
    }
    if signals.vidya_rows_ptr.iter().any(|&ptr| ptr == 0)
        || signals.ce_long_rows_ptr.iter().any(|&ptr| ptr == 0)
        || signals.ce_short_rows_ptr.iter().any(|&ptr| ptr == 0)
    {
        return Err("VCET GPU row-pointer vector contains null device pointers".to_string());
    }

    let combos_len = combos.vidya_row_index.len();
    checked_i32("VCET GPU combo count", combos_len)?;
    if combos.ce_row_index.len() != combos_len
        || combos.start_t.len() != combos_len
        || combos.end_t.len() != combos_len
    {
        return Err("VCET GPU combo vector length mismatch".to_string());
    }

    for (idx, ((&vidya_row, &ce_row), (&start_t, &end_t))) in combos
        .vidya_row_index
        .iter()
        .zip(combos.ce_row_index.iter())
        .zip(combos.start_t.iter().zip(combos.end_t.iter()))
        .enumerate()
    {
        if vidya_row < 0 || vidya_row >= signals.vidya_rows as i32 {
            return Err(format!(
                "VCET GPU vidya_row_index[{idx}] out of bounds: {vidya_row} not in [0, {})",
                signals.vidya_rows
            ));
        }
        if ce_row < 0 || ce_row >= signals.ce_rows as i32 {
            return Err(format!(
                "VCET GPU ce_row_index[{idx}] out of bounds: {ce_row} not in [0, {})",
                signals.ce_rows
            ));
        }
        if start_t < 0 || end_t < 0 {
            return Err(format!("VCET GPU time window[{idx}] must be non-negative"));
        }
        if start_t >= end_t {
            return Err(format!(
                "VCET GPU time window[{idx}] requires start_t < end_t but got {start_t} >= {end_t}"
            ));
        }
        if start_t >= n_i32 {
            return Err(format!(
                "VCET GPU time window[{idx}] start_t {start_t} exceeds cols {n_i32}"
            ));
        }
        if end_t > n_i32 {
            return Err(format!(
                "VCET GPU time window[{idx}] end_t {end_t} exceeds cols {n_i32}"
            ));
        }
    }

    Ok(())
}

fn launch_compute_log_returns_f32(
    rt: &mut VcetCudaRuntime,
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
        rt.launch_caps_for(VcetKernelKey::ComputeReturns, &kernel),
        &[64u32, 128, 256, 512],
    );

    let mut close_ptr = close_ptr;
    let mut len_i = checked_i32("VCET GPU close length", len)?;
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
    rt: &'a mut VcetCudaRuntime,
    signals: &VcetGpuDeviceSignalMatrices<'_>,
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
        .ok_or_else(|| "VCET resident close/lr cache missing".to_string())
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

        let (close_ptr_raw, lr_ptr_raw) = {
            let resident = ensure_resident_close_lr(rt, signals)?;
            let _keepalive = resident.d_close_owner.as_ref();
            (resident.close_ptr, resident.d_lr.as_device_ptr().as_raw())
        };

        VcetKernelScratch::ensure_len_u64(
            &mut rt.scratch.d_vidya_rows,
            signals.vidya_rows,
            &rt.stream,
        )?;
        VcetKernelScratch::ensure_len_u64(
            &mut rt.scratch.d_ce_long_rows,
            signals.ce_rows,
            &rt.stream,
        )?;
        VcetKernelScratch::ensure_len_u64(
            &mut rt.scratch.d_ce_short_rows,
            signals.ce_rows,
            &rt.stream,
        )?;
        VcetKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_vidya, combo_count, &rt.stream)?;
        VcetKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_ce, combo_count, &rt.stream)?;
        VcetKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_start, combo_count, &rt.stream)?;
        VcetKernelScratch::ensure_len_i32(&mut rt.scratch.d_combo_end, combo_count, &rt.stream)?;
        VcetKernelScratch::ensure_len_f32(&mut rt.scratch.d_out, out_len, &rt.stream)?;

        let vidya_rows_sig = slice_signature(signals.vidya_rows_ptr);
        if rt.scratch.vidya_rows_sig != Some(vidya_rows_sig) {
            let mut vidya_rows_slice = rt
                .scratch
                .d_vidya_rows
                .as_mut()
                .ok_or_else(|| "VCET VIDYA row scratch missing".to_string())?
                .index(..signals.vidya_rows);
            vidya_rows_slice
                .copy_from(signals.vidya_rows_ptr)
                .map_err(|e| e.to_string())?;
            rt.scratch.vidya_rows_sig = Some(vidya_rows_sig);
        }

        let ce_long_rows_sig = slice_signature(signals.ce_long_rows_ptr);
        if rt.scratch.ce_long_rows_sig != Some(ce_long_rows_sig) {
            let mut ce_long_rows_slice = rt
                .scratch
                .d_ce_long_rows
                .as_mut()
                .ok_or_else(|| "VCET CE-long row scratch missing".to_string())?
                .index(..signals.ce_rows);
            ce_long_rows_slice
                .copy_from(signals.ce_long_rows_ptr)
                .map_err(|e| e.to_string())?;
            rt.scratch.ce_long_rows_sig = Some(ce_long_rows_sig);
        }

        let ce_short_rows_sig = slice_signature(signals.ce_short_rows_ptr);
        if rt.scratch.ce_short_rows_sig != Some(ce_short_rows_sig) {
            let mut ce_short_rows_slice = rt
                .scratch
                .d_ce_short_rows
                .as_mut()
                .ok_or_else(|| "VCET CE-short row scratch missing".to_string())?
                .index(..signals.ce_rows);
            ce_short_rows_slice
                .copy_from(signals.ce_short_rows_ptr)
                .map_err(|e| e.to_string())?;
            rt.scratch.ce_short_rows_sig = Some(ce_short_rows_sig);
        }

        let combo_vidya_sig = slice_signature(combos.vidya_row_index);
        if rt.scratch.combo_vidya_sig != Some(combo_vidya_sig) {
            let mut combo_vidya_slice = rt
                .scratch
                .d_combo_vidya
                .as_mut()
                .ok_or_else(|| "VCET combo VIDYA scratch missing".to_string())?
                .index(..combo_count);
            combo_vidya_slice
                .copy_from(combos.vidya_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_vidya_sig = Some(combo_vidya_sig);
        }

        let combo_ce_sig = slice_signature(combos.ce_row_index);
        if rt.scratch.combo_ce_sig != Some(combo_ce_sig) {
            let mut combo_ce_slice = rt
                .scratch
                .d_combo_ce
                .as_mut()
                .ok_or_else(|| "VCET combo CE scratch missing".to_string())?
                .index(..combo_count);
            combo_ce_slice
                .copy_from(combos.ce_row_index)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_ce_sig = Some(combo_ce_sig);
        }

        let combo_start_sig = slice_signature(combos.start_t);
        if rt.scratch.combo_start_sig != Some(combo_start_sig) {
            let mut combo_start_slice = rt
                .scratch
                .d_combo_start
                .as_mut()
                .ok_or_else(|| "VCET combo start scratch missing".to_string())?
                .index(..combo_count);
            combo_start_slice
                .copy_from(combos.start_t)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_start_sig = Some(combo_start_sig);
        }

        let combo_end_sig = slice_signature(combos.end_t);
        if rt.scratch.combo_end_sig != Some(combo_end_sig) {
            let mut combo_end_slice = rt
                .scratch
                .d_combo_end
                .as_mut()
                .ok_or_else(|| "VCET combo end scratch missing".to_string())?
                .index(..combo_count);
            combo_end_slice
                .copy_from(combos.end_t)
                .map_err(|e| e.to_string())?;
            rt.scratch.combo_end_sig = Some(combo_end_sig);
        }

        let kernel = rt
            .module
            .get_function("vcet_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            rt.launch_caps,
            rt.launch_caps_for(VcetKernelKey::Backtest, &kernel),
            &[32u32, 64, 128, 256],
        );

        let mut flags: u32 = 0;
        if strategy.long_only {
            flags |= STRAT_LONG_ONLY;
        }
        if !strategy.allow_flip {
            flags |= STRAT_NO_FLIP;
        }
        if strategy.trade_on_next_bar {
            flags |= STRAT_TRADE_ON_NEXT_BAR;
        }
        flags |= STRAT_SIGNED_EXPOSURE;

        let commission = strategy.effective_commission().clamp(0.0, 0.95) as f32;

        let mut close_ptr = close_ptr_raw;
        let mut lr_ptr = lr_ptr_raw;
        let mut t_i = checked_i32("VCET GPU cols", signals.cols)?;
        let mut vidya_rows_ptr = rt
            .scratch
            .d_vidya_rows
            .as_ref()
            .ok_or_else(|| "VCET VIDYA row scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut ce_long_rows_ptr = rt
            .scratch
            .d_ce_long_rows
            .as_ref()
            .ok_or_else(|| "VCET CE-long row scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut ce_short_rows_ptr = rt
            .scratch
            .d_ce_short_rows
            .as_ref()
            .ok_or_else(|| "VCET CE-short row scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut vidya_rows_i = checked_i32("VCET GPU VIDYA row count", signals.vidya_rows)?;
        let mut ce_rows_i = checked_i32("VCET GPU CE row count", signals.ce_rows)?;
        let mut combo_vidya_ptr = rt
            .scratch
            .d_combo_vidya
            .as_ref()
            .ok_or_else(|| "VCET combo VIDYA scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_ce_ptr = rt
            .scratch
            .d_combo_ce
            .as_ref()
            .ok_or_else(|| "VCET combo CE scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_start_ptr = rt
            .scratch
            .d_combo_start
            .as_ref()
            .ok_or_else(|| "VCET combo start scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_end_ptr = rt
            .scratch
            .d_combo_end
            .as_ref()
            .ok_or_else(|| "VCET combo end scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut combo_count_i = checked_i32("VCET GPU combo count", combo_count)?;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut require_vidya_slope_i = if vcet.require_vidya_slope { 1i32 } else { 0i32 };
        let mut max_trade_bars_i = i32::from(vcet.max_trade_bars.max(1));
        let mut hurdle_tau_f = strategy.hurdle_tau.unwrap_or(0.0) as f32;
        let mut out_stride_i = checked_i32("VCET GPU metrics stride", VCET_METRICS_STRIDE)?;
        let mut out_ptr = rt
            .scratch
            .d_out
            .as_ref()
            .ok_or_else(|| "VCET output scratch missing".to_string())?
            .as_device_ptr()
            .as_raw();

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
            .ok_or_else(|| "VCET output scratch missing".to_string())?
            .index(..out_len);
        d_out_prefix
            .copy_to(rt.scratch.host_metrics.as_mut_slice())
            .map_err(|e| e.to_string())?;

        let mut out: Vec<Metrics> = Vec::with_capacity(combo_count);
        for row in rt.scratch.host_metrics[..out_len].chunks_exact(VCET_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        validate_inputs, validate_prepared_device_id, VcetGpuComboBatch,
        VcetGpuDeviceSignalMatrices,
    };

    #[test]
    fn validate_inputs_rejects_negative_start_t() {
        let close = vec![1.0f32; 8];
        let vidya_rows = vec![1u64];
        let ce_rows = vec![2u64];
        let signals = VcetGpuDeviceSignalMatrices {
            close_f32: &close,
            close_device_ptr: 3,
            cols: close.len(),
            vidya_rows_ptr: &vidya_rows,
            ce_long_rows_ptr: &ce_rows,
            ce_short_rows_ptr: &ce_rows,
            vidya_rows: vidya_rows.len(),
            ce_rows: ce_rows.len(),
        };
        let combos = VcetGpuComboBatch {
            vidya_row_index: &[0],
            ce_row_index: &[0],
            start_t: &[-1],
            end_t: &[5],
        };

        let err = validate_inputs(&signals, &combos).expect_err("negative start_t");
        assert!(err.contains("non-negative"));
    }

    #[test]
    fn validate_inputs_rejects_start_ge_end() {
        let close = vec![1.0f32; 8];
        let vidya_rows = vec![1u64];
        let ce_rows = vec![2u64];
        let signals = VcetGpuDeviceSignalMatrices {
            close_f32: &close,
            close_device_ptr: 3,
            cols: close.len(),
            vidya_rows_ptr: &vidya_rows,
            ce_long_rows_ptr: &ce_rows,
            ce_short_rows_ptr: &ce_rows,
            vidya_rows: vidya_rows.len(),
            ce_rows: ce_rows.len(),
        };
        let combos = VcetGpuComboBatch {
            vidya_row_index: &[0],
            ce_row_index: &[0],
            start_t: &[6],
            end_t: &[6],
        };

        let err = validate_inputs(&signals, &combos).expect_err("start_t >= end_t");
        assert!(err.contains("start_t < end_t"));
    }

    #[test]
    fn validate_inputs_rejects_row_index_out_of_bounds() {
        let close = vec![1.0f32; 8];
        let vidya_rows = vec![1u64];
        let ce_rows = vec![2u64];
        let signals = VcetGpuDeviceSignalMatrices {
            close_f32: &close,
            close_device_ptr: 3,
            cols: close.len(),
            vidya_rows_ptr: &vidya_rows,
            ce_long_rows_ptr: &ce_rows,
            ce_short_rows_ptr: &ce_rows,
            vidya_rows: vidya_rows.len(),
            ce_rows: ce_rows.len(),
        };
        let combos = VcetGpuComboBatch {
            vidya_row_index: &[1],
            ce_row_index: &[0],
            start_t: &[1],
            end_t: &[6],
        };

        let err = validate_inputs(&signals, &combos).expect_err("row index out of bounds");
        assert!(err.contains("vidya_row_index"));
    }

    #[test]
    fn validate_prepared_device_id_rejects_mismatch() {
        let err =
            validate_prepared_device_id(2, 3).expect_err("mismatched prepared/requested device");
        assert!(err.contains("prepared on device 2"));
        assert!(err.contains("requested device 3"));
    }
}
