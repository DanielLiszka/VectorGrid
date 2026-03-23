#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use cust::context::CurrentContext;
use cust::device::DeviceAttribute;
use cust::function::{Function, FunctionAttribute};
use cust::memory::{mem_get_info, AsyncCopyDestination, DeviceBuffer, LockedBuffer};
use cust::module::Module;
use cust::module::{ModuleJitOption, OptLevel};
use cust::prelude::*;
use cust::stream::{Stream, StreamFlags};
use cust::sys as cuda_sys;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use ta_optimizer::stream::StreamAggregator;
use ta_optimizer::{ObjectiveKind, OptimizationHeatmap, OptimizationResult};
use ta_strategies::double_ma::{DoubleMaParams, Metrics, StrategyConfig};
use vector_ta::cuda::CudaDeviceSliceF32Ref;
use vector_ta::utilities::data_loader::{source_type, Candles};

use crate::alloc_policy::{alloc_uninit_copy, alloc_uninit_f32, resize_uninit_f32};
use crate::double_ma::{metrics_pass_constraints, DoubleMaConstraints};
use crate::progress::ProgressSink;
use crate::strategy_gpu_program::{
    CompiledGpuPredicateProgramAbi, GpuConditionAbi, GpuConditionGroupAbi, GpuIndicatorSlotAbi,
    GpuSignalBindingsAbi,
};
use crate::strategy_gpu_runtime::{
    get_or_build_gpu_strategy_executable, with_uploaded_gpu_strategy_executable,
    GpuStrategyExecutableCacheKey, StrategyHandle,
};
use crate::vram_ma::{supports_vram_kernel_ma, VramMaComputer, VramMaInputs};

const STRAT_LONG_ONLY: u32 = 1u32 << 0;
#[allow(dead_code)]
const STRAT_NO_FLIP: u32 = 1u32 << 1;
const STRAT_TRADE_ON_NEXT_BAR: u32 = 1u32 << 2;
const STRAT_ENFORCE_FAST_LT_SLOW: u32 = 1u32 << 3;
#[allow(dead_code)]
const STRAT_SIGNED_EXPOSURE: u32 = 1u32 << 4;
const STRAT_STOP_LOSS: u32 = 1u32 << 5;
const STRAT_TAKE_PROFIT: u32 = 1u32 << 6;

#[cfg(test)]
thread_local! {
    static GPU_TEST_LOCK_DEPTH: Cell<usize> = const { Cell::new(0) };
    static GPU_TEST_LOCK_GUARD: RefCell<Option<std::sync::MutexGuard<'static, ()>>> =
        const { RefCell::new(None) };
}

#[cfg(test)]
pub(crate) struct GpuTestLockGuard;

#[cfg(test)]
impl Drop for GpuTestLockGuard {
    fn drop(&mut self) {
        GPU_TEST_LOCK_DEPTH.with(|depth| {
            let current = depth.get();
            debug_assert!(current > 0, "gpu test lock depth underflow");
            if current <= 1 {
                GPU_TEST_LOCK_GUARD.with(|slot| {
                    slot.borrow_mut().take();
                });
                depth.set(0);
            } else {
                depth.set(current - 1);
            }
        });
    }
}

#[cfg(test)]
pub(crate) fn lock_gpu_test() -> GpuTestLockGuard {
    static GPU_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    GPU_TEST_LOCK_DEPTH.with(|depth| {
        let current = depth.get();
        if current == 0 {
            let guard = GPU_TEST_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            GPU_TEST_LOCK_GUARD.with(|slot| {
                *slot.borrow_mut() = Some(guard);
            });
        }
        depth.set(current + 1);
    });
    GpuTestLockGuard
}

#[cfg(test)]
pub(crate) fn lock_test_env() -> std::sync::MutexGuard<'static, ()> {
    static TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
pub(crate) fn reset_test_runtime() {
    CUDA_KERNEL_RUNTIME.with(|cell| {
        cell.borrow_mut().clear();
    });
    DYNAMIC_GPU_METRICS_SCRATCH.with(|cell| {
        cell.borrow_mut().clear();
    });
    LAST_KERNEL_EXECUTOR_TIMING.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

#[cfg(test)]
thread_local! {
    static TEST_VRAM_BUDGET_MB: Cell<Option<usize>> = const { Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn with_test_vram_budget_mb<T>(budget_mb: Option<usize>, f: impl FnOnce() -> T) -> T {
    let prev = TEST_VRAM_BUDGET_MB.with(|cell| cell.replace(budget_mb));
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    TEST_VRAM_BUDGET_MB.with(|cell| cell.set(prev));
    match out {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

const BASE_METRICS_COUNT: usize = 7;
const METRICS_COUNT: usize = 10;
const METRIC_IDX_PNL: usize = 0;
const METRIC_IDX_TRADES: usize = 1;
const METRIC_IDX_MAX_DD: usize = 2;
const METRIC_IDX_MEAN: usize = 3;
const METRIC_IDX_STD: usize = 4;
const METRIC_IDX_EXPOSURE: usize = 5;
const METRIC_IDX_NET_EXPOSURE: usize = 6;
const METRIC_IDX_DOWNSIDE_DEVIATION: usize = 7;
const METRIC_IDX_ULCER_INDEX: usize = 8;
const METRIC_IDX_MARTIN_RATIO: usize = 9;
const TOPK_BLOCK_ITEMS: usize = 512;

#[derive(Clone, Copy, Debug)]
pub(crate) struct DeviceLaunchCaps {
    pub(crate) sm_count: u32,
    pub(crate) warp_size: u32,
    pub(crate) max_threads_per_block: u32,
    pub(crate) max_threads_per_sm: u32,
    pub(crate) max_registers_per_sm: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct KernelLaunchCaps {
    pub(crate) max_threads_per_block: u32,
    pub(crate) regs_per_thread: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LaunchShape1d {
    pub(crate) grid_x: u32,
    pub(crate) block_x: u32,
}

#[derive(Default, Debug)]
struct PipelineTimingProfile {
    host_prep: Duration,
    tile_prepare: Duration,
    tile_score: Duration,
    tile_backtest: Duration,
    tile_heatmap: Duration,
    tile_finalize: Duration,
    final_finalize: Duration,
    tile_count: usize,
}

impl PipelineTimingProfile {
    fn total(&self) -> Duration {
        self.host_prep
            .saturating_add(self.tile_prepare)
            .saturating_add(self.tile_score)
            .saturating_add(self.tile_finalize)
            .saturating_add(self.final_finalize)
    }

    fn pct_str(part: Duration, total: Duration) -> String {
        let t = total.as_secs_f64();
        if t <= 0.0 {
            return "0.0%".to_string();
        }
        format!("{:.1}%", (part.as_secs_f64() / t) * 100.0)
    }

    fn emit(
        &self,
        fast_ma: &str,
        slow_ma: &str,
        include_all: bool,
        topk: usize,
        heatmap_bins: usize,
    ) {
        let total = self.total();
        eprintln!(
            "VECTORBT_KERNEL_PIPELINE_PROFILE: pair={fast_ma}/{slow_ma} include_all={include_all} topk={topk} heatmap_bins={heatmap_bins} tiles={} total_ms={:.3}",
            self.tile_count,
            total.as_secs_f64() * 1000.0
        );
        eprintln!(
            "  host_prep     {:>10.3} ms  ({})",
            self.host_prep.as_secs_f64() * 1000.0,
            Self::pct_str(self.host_prep, total)
        );
        eprintln!(
            "  tile_prepare  {:>10.3} ms  ({})",
            self.tile_prepare.as_secs_f64() * 1000.0,
            Self::pct_str(self.tile_prepare, total)
        );
        eprintln!(
            "  tile_score    {:>10.3} ms  ({})",
            self.tile_score.as_secs_f64() * 1000.0,
            Self::pct_str(self.tile_score, total)
        );
        let tile_score_other = self
            .tile_score
            .saturating_sub(self.tile_backtest)
            .saturating_sub(self.tile_heatmap);
        eprintln!(
            "    backtest    {:>10.3} ms  ({})",
            self.tile_backtest.as_secs_f64() * 1000.0,
            Self::pct_str(self.tile_backtest, self.tile_score)
        );
        eprintln!(
            "    heatmap     {:>10.3} ms  ({})",
            self.tile_heatmap.as_secs_f64() * 1000.0,
            Self::pct_str(self.tile_heatmap, self.tile_score)
        );
        eprintln!(
            "    score_other {:>10.3} ms  ({})",
            tile_score_other.as_secs_f64() * 1000.0,
            Self::pct_str(tile_score_other, self.tile_score)
        );
        eprintln!(
            "  tile_finalize {:>10.3} ms  ({})",
            self.tile_finalize.as_secs_f64() * 1000.0,
            Self::pct_str(self.tile_finalize, total)
        );
        eprintln!(
            "  final_finalize{:>10.3} ms  ({})",
            self.final_finalize.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_finalize, total)
        );
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct KernelExecutorTimingMs {
    pub(crate) gpu_indicator_prep_ms: u64,
    pub(crate) h2d_upload_ms: u64,
}

#[inline]
fn device_attr_u32(device: Device, attr: DeviceAttribute, fallback: u32) -> u32 {
    device
        .get_attribute(attr)
        .ok()
        .and_then(|v| u32::try_from(v).ok())
        .filter(|&v| v > 0)
        .unwrap_or(fallback)
}

#[inline]
pub(crate) fn detect_device_launch_caps(device: Device) -> DeviceLaunchCaps {
    let warp_size = device_attr_u32(device, DeviceAttribute::WarpSize, 32).max(1);
    let max_threads_per_block = device_attr_u32(
        device,
        DeviceAttribute::MaxThreadsPerBlock,
        warp_size.saturating_mul(8),
    )
    .max(warp_size);
    let max_threads_per_sm = device_attr_u32(
        device,
        DeviceAttribute::MaxThreadsPerMultiprocessor,
        warp_size.saturating_mul(16),
    )
    .max(warp_size);
    let max_registers_per_sm = device_attr_u32(
        device,
        DeviceAttribute::MaxRegistersPerMultiprocessor,
        65_536,
    );

    DeviceLaunchCaps {
        sm_count: device_attr_u32(device, DeviceAttribute::MultiprocessorCount, 1).max(1),
        warp_size,
        max_threads_per_block,
        max_threads_per_sm,
        max_registers_per_sm,
    }
}

#[inline]
pub(crate) fn detect_kernel_launch_caps(
    func: &Function<'_>,
    device_caps: DeviceLaunchCaps,
) -> KernelLaunchCaps {
    let max_threads_per_block = func
        .get_attribute(FunctionAttribute::MaxThreadsPerBlock)
        .ok()
        .and_then(|v| u32::try_from(v).ok())
        .filter(|&v| v > 0)
        .unwrap_or(device_caps.max_threads_per_block)
        .min(device_caps.max_threads_per_block)
        .max(device_caps.warp_size);
    let regs_per_thread = func
        .get_attribute(FunctionAttribute::NumRegisters)
        .ok()
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0);
    KernelLaunchCaps {
        max_threads_per_block,
        regs_per_thread,
    }
}

#[inline]
fn kernel_target_waves() -> f32 {
    std::env::var("VECTORBT_KERNEL_TARGET_WAVES")
        .ok()
        .and_then(|v| v.trim().parse::<f32>().ok())
        .map(|v| v.clamp(0.5, 8.0))
        .unwrap_or(1.5)
}

#[inline]
fn fixed_block_override() -> Option<u32> {
    std::env::var("VECTORBT_KERNEL_BLOCK_X")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|&v| v > 0)
}

#[inline]
fn div_ceil_u32(n: u32, d: u32) -> u32 {
    (n + d - 1) / d
}

#[inline]
fn align_tm_stride(rows: usize) -> usize {
    const WARP: usize = 32;
    if rows == 0 {
        0
    } else {
        ((rows + (WARP - 1)) / WARP) * WARP
    }
}

#[inline]
fn normalize_block_x(raw: u32, warp: u32, max_block: u32) -> u32 {
    let warp = warp.max(1);
    let max_block = max_block.max(warp);
    let mut block = raw.clamp(warp, max_block);
    block -= block % warp;
    if block == 0 {
        warp
    } else {
        block
    }
}

#[inline]
fn estimated_active_blocks_per_sm(
    device_caps: DeviceLaunchCaps,
    kernel_caps: KernelLaunchCaps,
    block_x: u32,
) -> u32 {
    if block_x == 0 {
        return 1;
    }
    let by_threads = (device_caps.max_threads_per_sm / block_x).max(1);
    let by_regs = if kernel_caps.regs_per_thread > 0 && device_caps.max_registers_per_sm > 0 {
        let regs_per_block = kernel_caps.regs_per_thread.saturating_mul(block_x);
        if regs_per_block > 0 {
            (device_caps.max_registers_per_sm / regs_per_block).max(1)
        } else {
            by_threads
        }
    } else {
        by_threads
    };
    by_threads.min(by_regs).max(1)
}

#[inline]
pub(crate) fn choose_launch_shape_1d_from_candidates(
    total_items: usize,
    device_caps: DeviceLaunchCaps,
    kernel_caps: KernelLaunchCaps,
    candidates: &[u32],
) -> LaunchShape1d {
    if total_items == 0 {
        return LaunchShape1d {
            grid_x: 1,
            block_x: device_caps.warp_size.max(1),
        };
    }

    let total_u32 = total_items.min(u32::MAX as usize) as u32;
    let warp = device_caps.warp_size.max(1);
    let max_block = device_caps
        .max_threads_per_block
        .min(kernel_caps.max_threads_per_block)
        .max(warp);

    if let Some(raw) = fixed_block_override() {
        let block_x = normalize_block_x(raw, warp, max_block);
        return LaunchShape1d {
            grid_x: div_ceil_u32(total_u32, block_x),
            block_x,
        };
    }

    let mut candidates: Vec<u32> = candidates
        .iter()
        .copied()
        .map(|b| normalize_block_x(b, warp, max_block))
        .filter(|&b| b >= warp && b <= max_block)
        .collect();
    candidates.sort_unstable();
    candidates.dedup();
    if candidates.is_empty() {
        candidates.push(normalize_block_x(max_block, warp, max_block));
    }

    let target_waves = kernel_target_waves();
    let tiny_work_items = device_caps.sm_count.saturating_mul(warp).saturating_mul(32);
    let large_work_items = device_caps.sm_count.saturating_mul(2048);
    let mut best: Option<(f32, u32)> = None;
    for block_x in candidates {
        let grid_x = div_ceil_u32(total_u32, block_x);
        let active_blocks = estimated_active_blocks_per_sm(device_caps, kernel_caps, block_x);
        let launch_capacity = device_caps
            .sm_count
            .max(1)
            .saturating_mul(active_blocks.max(1));
        let waves = (grid_x as f32) / (launch_capacity.max(1) as f32);
        let occ = ((active_blocks.saturating_mul(block_x)) as f32
            / (device_caps.max_threads_per_sm.max(1) as f32))
            .clamp(0.0, 1.0);
        let waves_score = (waves / target_waves).clamp(0.0, 1.0);
        let mut score = (waves_score * 0.70) + (occ * 0.30);
        if waves >= target_waves {
            let over = ((waves / target_waves) - 1.0).clamp(0.0, 1.0);
            score += (block_x as f32 / max_block as f32) * (0.05 * over);
        } else if total_u32 <= tiny_work_items {
            score += ((max_block - block_x) as f32 / max_block as f32) * 0.02;
        }
        if total_u32 >= large_work_items {
            score += (block_x as f32 / max_block as f32) * 0.05;
        }

        match best {
            Some((best_score, best_block)) => {
                if score > best_score || (score == best_score && block_x > best_block) {
                    best = Some((score, block_x));
                }
            }
            None => best = Some((score, block_x)),
        }
    }

    let block_x = best.map(|(_, b)| b).unwrap_or(warp);
    LaunchShape1d {
        grid_x: div_ceil_u32(total_u32, block_x),
        block_x,
    }
}

#[inline]
fn choose_launch_shape_1d(
    total_items: usize,
    device_caps: DeviceLaunchCaps,
    kernel_caps: KernelLaunchCaps,
) -> LaunchShape1d {
    choose_launch_shape_1d_from_candidates(
        total_items,
        device_caps,
        kernel_caps,
        &[64u32, 128, 256, 512],
    )
}

#[inline]
fn choose_backtest_launch_shape_1d(
    total_items: usize,
    device_caps: DeviceLaunchCaps,
    kernel_caps: KernelLaunchCaps,
) -> LaunchShape1d {
    choose_launch_shape_1d_from_candidates(
        total_items,
        device_caps,
        kernel_caps,
        // Backtest kernels are sensitive to block shape on mid-sized ALMA workloads.
        &[64u32, 96, 128, 160, 192, 224, 256, 320, 384, 512],
    )
}

#[derive(Debug)]
struct CudaGraphExecHandle {
    raw: cuda_sys::CUgraphExec,
}

#[inline]
fn cu_result(res: cuda_sys::CUresult) -> Result<(), String> {
    if res == cuda_sys::CUresult::CUDA_SUCCESS {
        Ok(())
    } else {
        Err(format!("{res:?}"))
    }
}

impl CudaGraphExecHandle {
    #[inline]
    fn launch(&self, stream: &Stream) -> Result<(), String> {
        cu_result(unsafe { cuda_sys::cuGraphLaunch(self.raw, stream.as_inner()) })
    }
}

impl Drop for CudaGraphExecHandle {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        unsafe {
            let _ = cuda_sys::cuGraphExecDestroy(self.raw);
        }
        self.raw = std::ptr::null_mut();
    }
}

fn capture_graph_exec(
    stream: &Stream,
    record: impl FnOnce() -> Result<(), String>,
) -> Result<CudaGraphExecHandle, String> {
    cu_result(unsafe {
        cuda_sys::cuStreamBeginCapture_v2(
            stream.as_inner(),
            cuda_sys::CUstreamCaptureMode_enum::CU_STREAM_CAPTURE_MODE_GLOBAL,
        )
    })?;

    if let Err(e) = record() {
        let mut graph: cuda_sys::CUgraph = std::ptr::null_mut();
        let _ = unsafe { cuda_sys::cuStreamEndCapture(stream.as_inner(), &mut graph as *mut _) };
        if !graph.is_null() {
            let _ = unsafe { cuda_sys::cuGraphDestroy(graph) };
        }
        return Err(e);
    }

    let mut graph = std::mem::MaybeUninit::<cuda_sys::CUgraph>::uninit();
    cu_result(unsafe { cuda_sys::cuStreamEndCapture(stream.as_inner(), graph.as_mut_ptr()) })?;
    let graph = unsafe { graph.assume_init() };

    let mut graph_exec = std::mem::MaybeUninit::<cuda_sys::CUgraphExec>::uninit();
    let inst = unsafe {
        cuda_sys::cuGraphInstantiate_v2(
            graph_exec.as_mut_ptr(),
            graph,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };
    let _ = unsafe { cuda_sys::cuGraphDestroy(graph) };
    cu_result(inst)?;

    Ok(CudaGraphExecHandle {
        raw: unsafe { graph_exec.assume_init() },
    })
}

#[derive(Clone, Copy)]
struct KernelStrategyParams {
    flags: u32,
    commission: f32,
    eps_rel: f32,
    stop_log: f32,
    take_log: f32,
}

#[inline]
fn kernel_strategy_params(strategy: &StrategyConfig) -> KernelStrategyParams {
    let mut flags = STRAT_ENFORCE_FAST_LT_SLOW | STRAT_SIGNED_EXPOSURE;
    if strategy.long_only {
        flags |= STRAT_LONG_ONLY;
    }
    if !strategy.allow_flip {
        flags |= STRAT_NO_FLIP;
    }
    if strategy.trade_on_next_bar {
        flags |= STRAT_TRADE_ON_NEXT_BAR;
    }

    let stop_loss_pct = strategy.stop_loss_pct();
    let take_profit_pct = strategy.take_profit_pct();
    if stop_loss_pct.is_some() {
        flags |= STRAT_STOP_LOSS;
    }
    if take_profit_pct.is_some() {
        flags |= STRAT_TAKE_PROFIT;
    }

    KernelStrategyParams {
        flags,
        commission: strategy.effective_commission() as f32,
        eps_rel: strategy.eps_rel as f32,
        stop_log: stop_loss_pct
            .map(|pct| (1.0 - pct).ln() as f32)
            .unwrap_or(0.0),
        take_log: take_profit_pct
            .map(|pct| (1.0 + pct).ln() as f32)
            .unwrap_or(0.0),
    }
}

fn calmar_ratio(pnl: f64, max_dd: f64) -> f64 {
    const EPS_DENOM: f64 = 1e-12;
    const CLAMP_MAX: f64 = 1e6;

    let raw = if max_dd > EPS_DENOM {
        pnl / max_dd
    } else if pnl > 0.0 {
        CLAMP_MAX
    } else if pnl < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    };
    raw.clamp(-CLAMP_MAX, CLAMP_MAX)
}

#[inline]
fn search_rows_use_log_space(metrics_count: usize) -> bool {
    metrics_count < METRICS_COUNT
}

#[inline]
fn search_log_pnl_to_pnl(log_pnl: f64) -> f64 {
    log_pnl.exp_m1()
}

#[inline]
fn search_log_max_dd_to_max_dd(max_log_dd: f64) -> f64 {
    if max_log_dd > 0.0 {
        -(-max_log_dd).exp_m1()
    } else {
        0.0
    }
}

#[inline]
fn max_drawdown_constraint_to_search_space(max_dd: f64, metrics_count: usize) -> f32 {
    if !search_rows_use_log_space(metrics_count) {
        return max_dd as f32;
    }
    if !max_dd.is_finite() {
        return f32::INFINITY;
    }
    if max_dd <= 0.0 {
        return 0.0;
    }
    if max_dd >= 1.0 {
        return f32::INFINITY;
    }
    (-(1.0 - max_dd).ln()) as f32
}

#[inline]
fn decode_tm_flex_metrics_row(src: &[f32], base: usize, metrics_count: usize) -> Metrics {
    const CLAMP_MAX: f64 = 1e6;

    let metric = |idx: usize| -> f64 {
        if metrics_count > idx && base + idx < src.len() {
            src[base + idx] as f64
        } else {
            0.0
        }
    };

    let raw_pnl = metric(METRIC_IDX_PNL);
    let trades = metric(METRIC_IDX_TRADES);
    let raw_max_dd = metric(METRIC_IDX_MAX_DD);
    let mean = metric(METRIC_IDX_MEAN);
    let std = metric(METRIC_IDX_STD);
    let sharpe = if std > 0.0 { mean / std } else { 0.0 };
    let downside_deviation = metric(METRIC_IDX_DOWNSIDE_DEVIATION);
    let sortino = if downside_deviation > 0.0 {
        mean / downside_deviation
    } else if mean > 0.0 {
        CLAMP_MAX
    } else if mean < 0.0 {
        -CLAMP_MAX
    } else {
        0.0
    }
    .clamp(-CLAMP_MAX, CLAMP_MAX);
    let pnl = if search_rows_use_log_space(metrics_count) {
        search_log_pnl_to_pnl(raw_pnl)
    } else {
        raw_pnl
    };
    let max_dd = if search_rows_use_log_space(metrics_count) {
        search_log_max_dd_to_max_dd(raw_max_dd)
    } else {
        raw_max_dd
    };

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        calmar: calmar_ratio(pnl, max_dd),
        omega: 0.0,
        volatility: std,
        ulcer_index: metric(METRIC_IDX_ULCER_INDEX),
        martin_ratio: metric(METRIC_IDX_MARTIN_RATIO),
        max_dd,
        trades: if trades.is_finite() && trades >= 0.0 {
            trades as u32
        } else {
            0
        },
        exposure: metric(METRIC_IDX_EXPOSURE),
        net_exposure: metric(METRIC_IDX_NET_EXPOSURE),
    }
}

#[inline]
fn decode_packed_topk_metric_row(
    src: &[f32],
    row_idx: usize,
    metrics_count: usize,
) -> Option<(u16, u16, Metrics)> {
    let row_stride = metrics_count.saturating_add(2);
    let base = row_idx.checked_mul(row_stride)?;
    if base + row_stride > src.len() {
        return None;
    }
    let fast_len_i = src[base] as i32;
    let slow_len_i = src[base + 1] as i32;
    if fast_len_i <= 0 || slow_len_i <= 0 {
        return None;
    }
    let fast_len = u16::try_from(fast_len_i).ok()?;
    let slow_len = u16::try_from(slow_len_i).ok()?;
    if fast_len >= slow_len {
        return None;
    }
    let metrics = decode_tm_flex_metrics_row(src, base + 2, metrics_count);
    Some((fast_len, slow_len, metrics))
}

#[derive(Default)]
struct KernelScratch {
    d_lr: Option<DeviceBuffer<f32>>,
    d_ret1: Option<DeviceBuffer<f32>>,
    d_fast_p: Option<DeviceBuffer<i32>>,
    d_fast_ma: Option<DeviceBuffer<f32>>,
    d_fast_ma_tm: Option<DeviceBuffer<f32>>,
    d_slow_p: Option<DeviceBuffer<i32>>,
    d_slow_ma: Option<DeviceBuffer<f32>>,
    d_slow_ma_tm: Option<DeviceBuffer<f32>>,
    d_tile: Option<DeviceBuffer<f32>>,

    d_heatmap: Option<DeviceBuffer<f32>>,
    d_cand_a: Option<DeviceBuffer<i32>>,
    d_cand_b: Option<DeviceBuffer<i32>>,
    d_top_fast: Option<DeviceBuffer<i32>>,
    d_top_slow: Option<DeviceBuffer<i32>>,
    d_top_metrics: Option<DeviceBuffer<f32>>,
    d_top_packed: Option<DeviceBuffer<f32>>,
    d_valid_pair_index: Option<DeviceBuffer<i32>>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct PeriodSliceSignature {
    len: usize,
    first: u16,
    second: u16,
    last: u16,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ValidPairMapSignature {
    fast: PeriodSliceSignature,
    slow: PeriodSliceSignature,
}

struct CachedValidPairMap {
    buffer: DeviceBuffer<i32>,
    fast_rows: DeviceBuffer<i32>,
    slow_rows: DeviceBuffer<i32>,
    len: usize,
}

impl KernelScratch {
    fn drop_opt_async<T: cust::memory::DeviceCopy>(
        buf: &mut Option<DeviceBuffer<T>>,
        stream: &Stream,
    ) -> Result<(), String> {
        if let Some(old) = buf.take() {
            old.drop_async(stream).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn ensure_len_f32(
        buf: &mut Option<DeviceBuffer<f32>>,
        len: usize,
        stream: &Stream,
    ) -> Result<(), String> {
        if buf.as_ref().map(|b| b.len()) != Some(len) {
            Self::drop_opt_async(buf, stream)?;
            let next = if kernel_async_alloc_enabled() {
                unsafe { DeviceBuffer::<f32>::uninitialized_async(len, stream) }
            } else {
                unsafe { DeviceBuffer::<f32>::uninitialized(len) }
            }
            .map_err(|e| e.to_string())?;
            *buf = Some(next);
        }
        Ok(())
    }

    fn ensure_len_i32(
        buf: &mut Option<DeviceBuffer<i32>>,
        len: usize,
        stream: &Stream,
    ) -> Result<(), String> {
        if buf.as_ref().map(|b| b.len()) != Some(len) {
            Self::drop_opt_async(buf, stream)?;
            let next = if kernel_async_alloc_enabled() {
                unsafe { DeviceBuffer::<i32>::uninitialized_async(len, stream) }
            } else {
                unsafe { DeviceBuffer::<i32>::uninitialized(len) }
            }
            .map_err(|e| e.to_string())?;
            *buf = Some(next);
        }
        Ok(())
    }

    fn clear_async(&mut self, stream: &Stream) -> Result<(), String> {
        Self::drop_opt_async(&mut self.d_lr, stream)?;
        Self::drop_opt_async(&mut self.d_ret1, stream)?;
        Self::drop_opt_async(&mut self.d_fast_p, stream)?;
        Self::drop_opt_async(&mut self.d_fast_ma, stream)?;
        Self::drop_opt_async(&mut self.d_fast_ma_tm, stream)?;
        Self::drop_opt_async(&mut self.d_slow_p, stream)?;
        Self::drop_opt_async(&mut self.d_slow_ma, stream)?;
        Self::drop_opt_async(&mut self.d_slow_ma_tm, stream)?;
        Self::drop_opt_async(&mut self.d_tile, stream)?;

        Self::drop_opt_async(&mut self.d_heatmap, stream)?;
        Self::drop_opt_async(&mut self.d_cand_a, stream)?;
        Self::drop_opt_async(&mut self.d_cand_b, stream)?;
        Self::drop_opt_async(&mut self.d_top_fast, stream)?;
        Self::drop_opt_async(&mut self.d_top_slow, stream)?;
        Self::drop_opt_async(&mut self.d_top_metrics, stream)?;
        Self::drop_opt_async(&mut self.d_top_packed, stream)?;
        Self::drop_opt_async(&mut self.d_valid_pair_index, stream)?;
        Ok(())
    }
}

struct CudaKernelRuntime {
    context: std::sync::Arc<Context>,
    module: Module,
    stream: Stream,
    ma: VramMaComputer,
    scratch: KernelScratch,
    launch_caps: DeviceLaunchCaps,
    resident_market: Option<ResidentMarketSeriesCache>,
    valid_pair_maps: HashMap<ValidPairMapSignature, CachedValidPairMap>,
    kernel_caps: RefCell<HashMap<CachedKernelKey, KernelLaunchCaps>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CachedKernelKey {
    ComputeReturns,
    TransposeRowToTm,
    DoubleCrossBacktest,
    DoubleCrossBacktestM7,
    DoubleCrossBacktestLockstep,
    DoubleCrossBacktestLockstepM7,
    DoubleCrossBacktestFastWarpDedup,
    FillF32,
    HeatmapUpdateScores,
    BuildVfctRegime,
    BuildPitRegime,
    BuildSetRegime,
    BuildMtsRegime,
    BuildOrcRegime,
}

impl CudaKernelRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let launch_caps = detect_device_launch_caps(device);
        let context = std::sync::Arc::new(Context::new(device).map_err(|e| e.to_string())?);
        let module = load_double_crossover_module(device)?;
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;

        Ok(Self {
            context,
            module,
            stream,
            ma: VramMaComputer::new(device_id),
            scratch: KernelScratch::default(),
            launch_caps,
            resident_market: None,
            valid_pair_maps: HashMap::new(),
            kernel_caps: RefCell::new(HashMap::new()),
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(self.context.as_ref()).map_err(|e| e.to_string())
    }

    fn launch_caps_for_kernel(&self, key: CachedKernelKey, func: &Function) -> KernelLaunchCaps {
        if let Some(cached) = self.kernel_caps.borrow().get(&key) {
            return *cached;
        }
        let caps = detect_kernel_launch_caps(func, self.launch_caps);
        self.kernel_caps.borrow_mut().insert(key, caps);
        caps
    }
}

pub(crate) fn load_double_crossover_module(device: Device) -> Result<Module, String> {
    if device_compute_capability(device) == Some((8, 9)) {
        let cubin: &[u8] =
            include_bytes!(concat!(env!("OUT_DIR"), "/double_crossover.sm_89.cubin"));
        if !cubin.is_empty() {
            if let Ok(module) = Module::from_cubin(cubin, &[]) {
                return Ok(module);
            }
        }
    }

    let ptx: &str = include_str!(concat!(env!("OUT_DIR"), "/double_crossover.ptx"));
    let jit_opts = &[
        ModuleJitOption::DetermineTargetFromContext,
        ModuleJitOption::OptLevel(OptLevel::O2),
    ];
    match Module::from_ptx(ptx, jit_opts) {
        Ok(m) => Ok(m),
        Err(_) => {
            if let Ok(m) = Module::from_ptx(ptx, &[ModuleJitOption::DetermineTargetFromContext]) {
                Ok(m)
            } else {
                Module::from_ptx(ptx, &[]).map_err(|e| e.to_string())
            }
        }
    }
}

#[inline]
fn period_slice_signature(periods: &[u16]) -> Option<PeriodSliceSignature> {
    let first = *periods.first()?;
    let second = periods.get(1).copied().unwrap_or(first);
    let last = *periods.last().unwrap_or(&first);
    Some(PeriodSliceSignature {
        len: periods.len(),
        first,
        second,
        last,
    })
}

fn ensure_cached_valid_pair_map(
    valid_pair_maps: &mut HashMap<ValidPairMapSignature, CachedValidPairMap>,
    fast_periods: &[u16],
    slow_periods: &[u16],
) -> Result<Option<(u64, usize, u64, u64)>, String> {
    let Some(fast_sig) = period_slice_signature(fast_periods) else {
        return Ok(None);
    };
    let Some(slow_sig) = period_slice_signature(slow_periods) else {
        return Ok(None);
    };
    let key = ValidPairMapSignature {
        fast: fast_sig,
        slow: slow_sig,
    };

    if !valid_pair_maps.contains_key(&key) {
        let valid_map = build_valid_pair_index_map_i32(fast_periods, slow_periods)?;
        let (fast_rows_host, slow_rows_host) =
            build_valid_pair_period_rows_i32(fast_periods, slow_periods);
        let len = valid_map.len();
        let buffer = DeviceBuffer::from_slice(&valid_map).map_err(|e| e.to_string())?;
        let fast_rows = DeviceBuffer::from_slice(&fast_rows_host).map_err(|e| e.to_string())?;
        let slow_rows = DeviceBuffer::from_slice(&slow_rows_host).map_err(|e| e.to_string())?;
        valid_pair_maps.insert(
            key,
            CachedValidPairMap {
                buffer,
                fast_rows,
                slow_rows,
                len,
            },
        );
    }

    let entry = valid_pair_maps
        .get(&key)
        .ok_or_else(|| "cached valid pair map missing after insertion".to_string())?;
    Ok(Some((
        entry.buffer.as_device_ptr().as_raw(),
        entry.len,
        entry.fast_rows.as_device_ptr().as_raw(),
        entry.slow_rows.as_device_ptr().as_raw(),
    )))
}

fn device_compute_capability(device: Device) -> Option<(u32, u32)> {
    let major = device
        .get_attribute(DeviceAttribute::ComputeCapabilityMajor)
        .ok()
        .and_then(|v| u32::try_from(v).ok())?;
    let minor = device
        .get_attribute(DeviceAttribute::ComputeCapabilityMinor)
        .ok()
        .and_then(|v| u32::try_from(v).ok())?;
    Some((major, minor))
}

thread_local! {
    static CUDA_KERNEL_RUNTIME: RefCell<HashMap<u32, CudaKernelRuntime>> = RefCell::new(HashMap::new());
    static DYNAMIC_GPU_METRICS_SCRATCH: RefCell<HashMap<(u32, usize), DeviceBuffer<f32>>> = RefCell::new(HashMap::new());
    static LAST_KERNEL_EXECUTOR_TIMING: RefCell<Option<KernelExecutorTimingMs>> = RefCell::new(None);
}

fn clear_last_kernel_executor_timing() {
    LAST_KERNEL_EXECUTOR_TIMING.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

fn record_last_kernel_executor_timing(timing: KernelExecutorTimingMs) {
    LAST_KERNEL_EXECUTOR_TIMING.with(|cell| {
        *cell.borrow_mut() = Some(timing);
    });
}

pub(crate) fn take_last_kernel_executor_timing() -> Option<KernelExecutorTimingMs> {
    LAST_KERNEL_EXECUTOR_TIMING.with(|cell| cell.borrow_mut().take())
}

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed_pairs: usize,
    total_pairs: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed_pairs, total_pairs, phase);
}

fn fill_device_buffer_f32(
    stream: &Stream,
    fill_kernel: &Function,
    device_launch_caps: DeviceLaunchCaps,
    fill_launch_caps: KernelLaunchCaps,
    buffer: &DeviceBuffer<f32>,
    len: usize,
    value: f32,
) -> Result<(), String> {
    if len == 0 {
        return Ok(());
    }

    unsafe {
        let mut out_ptr = buffer.as_device_ptr().as_raw();
        let mut n_i = len as i32;
        let mut v = value;
        let launch = choose_launch_shape_1d(len, device_launch_caps, fill_launch_caps);
        let block_x: u32 = launch.block_x;
        let grid_x: u32 = launch.grid_x;
        let args: &mut [*mut c_void] = &mut [
            &mut out_ptr as *mut _ as *mut c_void,
            &mut n_i as *mut _ as *mut c_void,
            &mut v as *mut _ as *mut c_void,
        ];
        stream
            .launch(fill_kernel, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct KernelConfig {
    pub device_id: u32,
    pub fast_alma_offset: f64,
    pub fast_alma_sigma: f64,
    pub slow_alma_offset: f64,
    pub slow_alma_sigma: f64,
}

pub struct PrecomputedHostMaMatrix<'a> {
    pub periods: &'a [u16],
    pub cols: usize,
    pub values_row_major: &'a [f32],
}

pub(crate) struct DeviceRegimeFilter {
    buffer: DeviceBuffer<f32>,
    scratch_f32: Vec<DeviceBuffer<f32>>,
    scratch_dispatch_f32: Vec<vector_ta::indicators::dispatch::DeviceMatrixF32>,
    len: usize,
    first_valid: usize,
    device_id: u32,
}

impl DeviceRegimeFilter {
    #[inline]
    pub(crate) fn with_scratch_buffers(mut self, scratch_f32: Vec<DeviceBuffer<f32>>) -> Self {
        self.scratch_f32 = scratch_f32;
        self
    }

    #[inline]
    pub(crate) fn with_dispatch_matrices(
        mut self,
        scratch_dispatch_f32: Vec<vector_ta::indicators::dispatch::DeviceMatrixF32>,
    ) -> Self {
        self.scratch_dispatch_f32 = scratch_dispatch_f32;
        self
    }

    #[inline]
    fn as_device_ptr_raw(&self) -> u64 {
        self.buffer.as_device_ptr().as_raw()
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn first_valid(&self) -> usize {
        self.first_valid
    }

    #[inline]
    fn device_id(&self) -> u32 {
        self.device_id
    }
}

pub(crate) fn upload_device_series_f32(
    device_id: u32,
    values: &[f32],
) -> Result<(CudaDeviceSliceF32Ref, DeviceBuffer<f32>), String> {
    if values.is_empty() {
        return Err("device upload requires a non-empty series".to_string());
    }

    CUDA_KERNEL_RUNTIME.with(
        |cell| -> Result<(CudaDeviceSliceF32Ref, DeviceBuffer<f32>), String> {
            let mut map = cell.borrow_mut();
            if !map.contains_key(&device_id) {
                map.insert(device_id, CudaKernelRuntime::new(device_id)?);
            }
            let rt = map.get_mut(&device_id).expect("just inserted");
            rt.set_current()?;

            let buffer = DeviceBuffer::from_slice(values).map_err(|e| e.to_string())?;
            let slice = unsafe {
                CudaDeviceSliceF32Ref::from_raw_parts(
                    buffer.as_device_ptr().as_raw(),
                    values.len(),
                    device_id,
                )
            }
            .map_err(|e| e.to_string())?;
            Ok((slice, buffer))
        },
    )
}

pub(crate) fn upload_host_regime_filter(
    device_id: u32,
    regime: &[f32],
    first_valid: usize,
) -> Result<DeviceRegimeFilter, String> {
    let len = regime.len();
    if len == 0 {
        return Err("device regime upload requires a non-empty series".to_string());
    }
    if first_valid > len {
        return Err(format!(
            "device regime first_valid {first_valid} exceeds input len {len}"
        ));
    }

    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<DeviceRegimeFilter, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;
        let buffer = DeviceBuffer::from_slice(regime).map_err(|e| e.to_string())?;
        Ok(DeviceRegimeFilter {
            buffer,
            scratch_f32: Vec::new(),
            scratch_dispatch_f32: Vec::new(),
            len,
            first_valid,
            device_id,
        })
    })
}

#[allow(dead_code)]
pub(crate) struct UploadedGpuStrategyProgram {
    pub(crate) d_indicator_slots: DeviceBuffer<GpuIndicatorSlotAbi>,
    pub(crate) d_conditions: DeviceBuffer<GpuConditionAbi>,
    pub(crate) d_groups: DeviceBuffer<GpuConditionGroupAbi>,
    pub(crate) d_group_condition_indexes: DeviceBuffer<u32>,
    pub(crate) d_signals: DeviceBuffer<GpuSignalBindingsAbi>,
    pub(crate) abi_version: u32,
    pub(crate) plan_fingerprint_hex: String,
    pub(crate) device_id: u32,
}

impl UploadedGpuStrategyProgram {
    pub(crate) fn indicator_slots_ptr_raw(&self) -> u64 {
        self.d_indicator_slots.as_device_ptr().as_raw()
    }

    pub(crate) fn conditions_ptr_raw(&self) -> u64 {
        self.d_conditions.as_device_ptr().as_raw()
    }

    pub(crate) fn groups_ptr_raw(&self) -> u64 {
        self.d_groups.as_device_ptr().as_raw()
    }

    pub(crate) fn group_condition_indexes_ptr_raw(&self) -> u64 {
        self.d_group_condition_indexes.as_device_ptr().as_raw()
    }

    pub(crate) fn signals_ptr_raw(&self) -> u64 {
        self.d_signals.as_device_ptr().as_raw()
    }
}

pub(crate) fn upload_gpu_strategy_program_abi(
    device_id: u32,
    abi: &CompiledGpuPredicateProgramAbi,
) -> Result<UploadedGpuStrategyProgram, String> {
    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<UploadedGpuStrategyProgram, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let d_indicator_slots =
            DeviceBuffer::from_slice(&abi.indicator_slots).map_err(|e| e.to_string())?;
        let d_conditions = DeviceBuffer::from_slice(&abi.conditions).map_err(|e| e.to_string())?;
        let d_groups = DeviceBuffer::from_slice(&abi.groups).map_err(|e| e.to_string())?;
        let d_group_condition_indexes =
            DeviceBuffer::from_slice(&abi.group_condition_indexes).map_err(|e| e.to_string())?;
        let d_signals = DeviceBuffer::from_slice(std::slice::from_ref(&abi.signals))
            .map_err(|e| e.to_string())?;

        Ok(UploadedGpuStrategyProgram {
            d_indicator_slots,
            d_conditions,
            d_groups,
            d_group_condition_indexes,
            d_signals,
            abi_version: abi.abi_version,
            plan_fingerprint_hex: abi.plan_fingerprint_hex.clone(),
            device_id,
        })
    })
}

pub(crate) fn probe_uploaded_gpu_strategy_program(
    uploaded: &UploadedGpuStrategyProgram,
) -> Result<[u32; 7], String> {
    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<[u32; 7], String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&uploaded.device_id) {
            map.insert(
                uploaded.device_id,
                CudaKernelRuntime::new(uploaded.device_id)?,
            );
        }
        let rt = map.get_mut(&uploaded.device_id).expect("just inserted");
        rt.set_current()?;
        let func = rt
            .module
            .get_function("strategy_program_probe_f32")
            .map_err(|e| e.to_string())?;

        let mut out = [0u32; 7];
        let d_out = DeviceBuffer::from_slice(&out).map_err(|e| e.to_string())?;
        let mut slots_ptr = uploaded.indicator_slots_ptr_raw();
        let mut slot_count = uploaded.d_indicator_slots.len() as i32;
        let mut conditions_ptr = uploaded.conditions_ptr_raw();
        let mut condition_count = uploaded.d_conditions.len() as i32;
        let mut groups_ptr = uploaded.groups_ptr_raw();
        let mut group_count = uploaded.d_groups.len() as i32;
        let mut group_idx_ptr = uploaded.group_condition_indexes_ptr_raw();
        let mut group_idx_count = uploaded.d_group_condition_indexes.len() as i32;
        let mut signals_ptr = uploaded.signals_ptr_raw();
        let mut abi_version = uploaded.abi_version;
        let mut out_ptr = d_out.as_device_ptr().as_raw();
        let args: &mut [*mut c_void] = &mut [
            &mut slots_ptr as *mut _ as *mut c_void,
            &mut slot_count as *mut _ as *mut c_void,
            &mut conditions_ptr as *mut _ as *mut c_void,
            &mut condition_count as *mut _ as *mut c_void,
            &mut groups_ptr as *mut _ as *mut c_void,
            &mut group_count as *mut _ as *mut c_void,
            &mut group_idx_ptr as *mut _ as *mut c_void,
            &mut group_idx_count as *mut _ as *mut c_void,
            &mut signals_ptr as *mut _ as *mut c_void,
            &mut abi_version as *mut _ as *mut c_void,
            &mut out_ptr as *mut _ as *mut c_void,
        ];
        unsafe {
            rt.stream
                .launch(&func, (1, 1, 1), (1, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        rt.stream.synchronize().map_err(|e| e.to_string())?;
        d_out.copy_to(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    })
}

pub struct UploadedDoubleMaDynamicProgramBatch {
    d_lr: DeviceBuffer<f32>,
    d_ret1: DeviceBuffer<f32>,
    d_fast_ma_tm: DeviceBuffer<f32>,
    d_slow_ma_tm: DeviceBuffer<f32>,
    d_fast_periods: DeviceBuffer<i32>,
    d_slow_periods: DeviceBuffer<i32>,
    d_pair_index_map: DeviceBuffer<i32>,
    combo_dense_pair_idx: Vec<usize>,
    combo_count: usize,
    first_valid: usize,
    fast_tm_stride: usize,
    slow_tm_stride: usize,
    device_id: u32,
}

impl UploadedDoubleMaDynamicProgramBatch {
    pub fn combo_count(&self) -> usize {
        self.combo_count
    }
}

fn transpose_row_major_to_tm_f32(row_major: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let mut out = alloc_uninit_f32(row_major.len());
    for row in 0..rows {
        let row_off = row * cols;
        for col in 0..cols {
            out[col * rows + row] = row_major[row_off + col];
        }
    }
    out
}

fn simple_returns_from_log_returns_f32(lr: &[f32]) -> Vec<f32> {
    let mut out = alloc_uninit_f32(lr.len());
    for (dst, &v) in out.iter_mut().zip(lr.iter()) {
        *dst = v.exp_m1();
    }
    out
}

fn validate_double_ma_dynamic_strategy_handle(handle: &StrategyHandle) -> Result<(), String> {
    if handle.plan.backend != crate::strategy_plan::StrategyPlanBackend::GpuUnified {
        return Err("dynamic gpu kernel requires a GpuUnified strategy plan".to_string());
    }
    if handle.plan.profile != crate::strategy_ir::StrategyProfileKind::DoubleMa {
        return Err("dynamic gpu kernel currently supports only the DoubleMa profile".to_string());
    }
    if handle.plan.halftrend_filter.is_some() {
        return Err(
            "dynamic gpu kernel v1 does not support halftrend/regime-gated DoubleMa plans"
                .to_string(),
        );
    }
    if handle.plan.strategy.exit_mode != ta_strategies::double_ma::ExitMode::Cross {
        return Err(
            "dynamic gpu kernel v1 supports only plain crossover exit mode for DoubleMa"
                .to_string(),
        );
    }
    let program = handle
        .gpu_program
        .as_ref()
        .ok_or_else(|| "strategy handle is missing a compiled gpu predicate program".to_string())?;
    if program.indicator_slots.len() != 2 {
        return Err(format!(
            "dynamic gpu kernel v1 expects exactly 2 indicator slots, got {}",
            program.indicator_slots.len()
        ));
    }
    for slot in &program.indicator_slots {
        if slot.indicator_index > 1 {
            return Err(format!(
                "dynamic gpu kernel v1 only supports fast/slow slots, found indicator slot index {}",
                slot.indicator_index
            ));
        }
    }
    if let Some(abi) = handle.gpu_program_abi.as_ref() {
        for condition in &abi.conditions {
            let lhs_supported = matches!(condition.lhs.source, 0 | 3);
            let rhs_supported = matches!(condition.rhs.source, 0 | 3);
            if !lhs_supported || !rhs_supported {
                return Err(
                    "dynamic gpu kernel v1 supports only indicator and constant operands"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

pub fn upload_double_ma_dynamic_program_batch(
    lr: &[f32],
    fast_row_major: &[f32],
    slow_row_major: &[f32],
    stride: usize,
    fast_periods: &[u16],
    slow_periods: &[u16],
    combos: &[DoubleMaParams],
    first_valid: usize,
    _strategy: &StrategyConfig,
    device_id: u32,
) -> Result<UploadedDoubleMaDynamicProgramBatch, String> {
    if lr.len() < 2 {
        return Err("dynamic gpu batch upload requires at least 2 lr values".to_string());
    }
    if stride == 0 {
        return Err("dynamic gpu batch upload requires a non-zero stride".to_string());
    }
    if combos.is_empty() {
        return Err("dynamic gpu batch upload requires at least one combo".to_string());
    }
    if !fast_row_major.len().is_multiple_of(stride) {
        return Err("fast row-major matrix length must be a multiple of stride".to_string());
    }
    if !slow_row_major.len().is_multiple_of(stride) {
        return Err("slow row-major matrix length must be a multiple of stride".to_string());
    }
    let fast_rows = fast_row_major.len() / stride;
    let slow_rows = slow_row_major.len() / stride;
    if fast_rows != fast_periods.len() {
        return Err(format!(
            "fast row-major matrix rows ({fast_rows}) do not match fast period count ({})",
            fast_periods.len()
        ));
    }
    if slow_rows != slow_periods.len() {
        return Err(format!(
            "slow row-major matrix rows ({slow_rows}) do not match slow period count ({})",
            slow_periods.len()
        ));
    }
    if stride > i32::MAX as usize || lr.len() > i32::MAX as usize {
        return Err("dynamic gpu batch dimensions exceed i32 kernel limits".to_string());
    }
    let fast_tm = transpose_row_major_to_tm_f32(fast_row_major, fast_rows, stride);
    let slow_tm = transpose_row_major_to_tm_f32(slow_row_major, slow_rows, stride);
    let ret1 = simple_returns_from_log_returns_f32(lr);

    let mut fast_idx: HashMap<u16, usize> = HashMap::with_capacity(fast_periods.len());
    for (i, &period) in fast_periods.iter().enumerate() {
        fast_idx.insert(period, i);
    }
    let mut slow_idx: HashMap<u16, usize> = HashMap::with_capacity(slow_periods.len());
    for (i, &period) in slow_periods.iter().enumerate() {
        slow_idx.insert(period, i);
    }

    let fast_periods_i32: Vec<i32> = fast_periods.iter().map(|&p| p as i32).collect();
    let slow_periods_i32: Vec<i32> = slow_periods.iter().map(|&p| p as i32).collect();
    let mut combo_dense_pair_idx = Vec::with_capacity(combos.len());
    for combo in combos {
        let fast_row = *fast_idx
            .get(&combo.fast_len)
            .ok_or_else(|| format!("fast_len {} not in fast_periods", combo.fast_len))?;
        let slow_row = *slow_idx
            .get(&combo.slow_len)
            .ok_or_else(|| format!("slow_len {} not in slow_periods", combo.slow_len))?;
        let dense_pair_idx = fast_row
            .checked_mul(slow_rows)
            .and_then(|v| v.checked_add(slow_row))
            .ok_or_else(|| "dynamic gpu dense pair index overflow".to_string())?;
        combo_dense_pair_idx.push(dense_pair_idx);
    }

    let pair_index_map_i32: Vec<i32> = combo_dense_pair_idx
        .iter()
        .map(|&idx| {
            i32::try_from(idx).map_err(|_| "dynamic gpu dense pair index exceeds i32".to_string())
        })
        .collect::<Result<_, _>>()?;

    CUDA_KERNEL_RUNTIME.with(
        |cell| -> Result<UploadedDoubleMaDynamicProgramBatch, String> {
            let mut map = cell.borrow_mut();
            if !map.contains_key(&device_id) {
                map.insert(device_id, CudaKernelRuntime::new(device_id)?);
            }
            let rt = map.get_mut(&device_id).expect("just inserted");
            rt.set_current()?;

            Ok(UploadedDoubleMaDynamicProgramBatch {
                d_lr: DeviceBuffer::from_slice(lr).map_err(|e| e.to_string())?,
                d_ret1: DeviceBuffer::from_slice(&ret1).map_err(|e| e.to_string())?,
                d_fast_ma_tm: DeviceBuffer::from_slice(&fast_tm).map_err(|e| e.to_string())?,
                d_slow_ma_tm: DeviceBuffer::from_slice(&slow_tm).map_err(|e| e.to_string())?,
                d_fast_periods: DeviceBuffer::from_slice(&fast_periods_i32)
                    .map_err(|e| e.to_string())?,
                d_slow_periods: DeviceBuffer::from_slice(&slow_periods_i32)
                    .map_err(|e| e.to_string())?,
                d_pair_index_map: DeviceBuffer::from_slice(&pair_index_map_i32)
                    .map_err(|e| e.to_string())?,
                combo_dense_pair_idx,
                combo_count: combos.len(),
                first_valid,
                fast_tm_stride: fast_rows,
                slow_tm_stride: slow_rows,
                device_id,
            })
        },
    )
}

pub fn eval_uploaded_double_ma_dynamic_program_gpu_kernel(
    uploaded_batch: &UploadedDoubleMaDynamicProgramBatch,
    strategy: &StrategyConfig,
    handle: &StrategyHandle,
) -> Result<Vec<Metrics>, String> {
    validate_double_ma_dynamic_strategy_handle(handle)?;
    let _ = get_or_build_gpu_strategy_executable(handle, uploaded_batch.device_id)?;
    let key = GpuStrategyExecutableCacheKey::new(
        handle.plan.ir_fingerprint_hex.clone(),
        uploaded_batch.device_id,
    );
    let strategy_params = kernel_strategy_params(strategy);

    with_uploaded_gpu_strategy_executable(&key, |_uploaded_program| {
        CUDA_KERNEL_RUNTIME.with(|cell| -> Result<Vec<Metrics>, String> {
            let mut map = cell.borrow_mut();
            if !map.contains_key(&uploaded_batch.device_id) {
                map.insert(
                    uploaded_batch.device_id,
                    CudaKernelRuntime::new(uploaded_batch.device_id)?,
                );
            }
            let rt = map
                .get_mut(&uploaded_batch.device_id)
                .expect("just inserted");
            rt.set_current()?;

            let func = rt
                .module
                .get_function("double_cross_backtest_tm_flex_f32_m7")
                .map_err(|e| e.to_string())?;
            let launch_caps =
                rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM7, &func);
            let launch = choose_backtest_launch_shape_1d(
                uploaded_batch.combo_count,
                rt.launch_caps,
                launch_caps,
            );
            let out_stride = METRICS_COUNT;
            let out_len = uploaded_batch
                .fast_tm_stride
                .checked_mul(uploaded_batch.slow_tm_stride)
                .and_then(|v| v.checked_mul(out_stride))
                .ok_or_else(|| "dynamic gpu metrics output length overflow".to_string())?;
            let scratch_key = (uploaded_batch.device_id, out_len);

            let mut lr_ptr = uploaded_batch.d_lr.as_device_ptr().as_raw();
            let mut ret1_ptr = uploaded_batch.d_ret1.as_device_ptr().as_raw();
            let mut t_i = uploaded_batch.d_lr.len() as i32;
            let mut fast_ptr = uploaded_batch.d_fast_ma_tm.as_device_ptr().as_raw();
            let mut fast_stride_i = uploaded_batch.fast_tm_stride as i32;
            let mut slow_ptr = uploaded_batch.d_slow_ma_tm.as_device_ptr().as_raw();
            let mut slow_stride_i = uploaded_batch.slow_tm_stride as i32;
            let mut pf_tile_i = uploaded_batch.fast_tm_stride as i32;
            let mut ps_tile_i = uploaded_batch.slow_tm_stride as i32;
            let mut pf_total_i = uploaded_batch.fast_tm_stride as i32;
            let mut ps_total_i = uploaded_batch.slow_tm_stride as i32;
            let mut f_off_i = 0i32;
            let mut s_off_i = 0i32;
            let mut fast_periods_ptr = uploaded_batch.d_fast_periods.as_device_ptr().as_raw();
            let mut slow_periods_ptr = uploaded_batch.d_slow_periods.as_device_ptr().as_raw();
            let mut regime_ptr = std::ptr::null_mut::<f32>();
            let mut use_regime_i = 0i32;
            let mut use_regime_hybrid_i = 0i32;
            let mut regime_entry_f = 0.0f32;
            let mut regime_exit_f = 0.0f32;
            let mut first_valid_i = uploaded_batch.first_valid as i32;
            let mut regime_first_valid_i = uploaded_batch.first_valid as i32;
            let mut flags_u = strategy_params.flags;
            let mut commission_f = strategy_params.commission;
            let mut eps_rel_f = strategy_params.eps_rel;
            let mut stop_log_f = strategy_params.stop_log;
            let mut take_log_f = strategy_params.take_log;
            let mut m_i = out_stride as i32;
            let mut metrics_stride_i = out_stride as i32;
            let mut pair_map_ptr = uploaded_batch.d_pair_index_map.as_device_ptr().as_raw();
            let mut pair_count_i = uploaded_batch.combo_count as i32;
            let mut compact_output_i = 0i32;
            let mut host_out = alloc_uninit_f32(out_len);
            DYNAMIC_GPU_METRICS_SCRATCH.with(|scratch| -> Result<(), String> {
                let mut scratch = scratch.borrow_mut();
                if !scratch.contains_key(&scratch_key) {
                    scratch.insert(
                        scratch_key,
                        unsafe { DeviceBuffer::<f32>::uninitialized(out_len) }
                            .map_err(|e| e.to_string())?,
                    );
                }
                let d_out = scratch.get_mut(&scratch_key).ok_or_else(|| {
                    "dynamic gpu scratch buffer missing after insertion".to_string()
                })?;
                let mut out_ptr = d_out.as_device_ptr().as_raw();
                let args: &mut [*mut c_void] = &mut [
                    &mut fast_ptr as *mut _ as *mut c_void,
                    &mut slow_ptr as *mut _ as *mut c_void,
                    &mut lr_ptr as *mut _ as *mut c_void,
                    &mut ret1_ptr as *mut _ as *mut c_void,
                    &mut t_i as *mut _ as *mut c_void,
                    &mut pf_tile_i as *mut _ as *mut c_void,
                    &mut ps_tile_i as *mut _ as *mut c_void,
                    &mut fast_stride_i as *mut _ as *mut c_void,
                    &mut slow_stride_i as *mut _ as *mut c_void,
                    &mut pf_total_i as *mut _ as *mut c_void,
                    &mut ps_total_i as *mut _ as *mut c_void,
                    &mut f_off_i as *mut _ as *mut c_void,
                    &mut s_off_i as *mut _ as *mut c_void,
                    &mut fast_periods_ptr as *mut _ as *mut c_void,
                    &mut slow_periods_ptr as *mut _ as *mut c_void,
                    &mut regime_ptr as *mut _ as *mut c_void,
                    &mut use_regime_i as *mut _ as *mut c_void,
                    &mut use_regime_hybrid_i as *mut _ as *mut c_void,
                    &mut regime_entry_f as *mut _ as *mut c_void,
                    &mut regime_exit_f as *mut _ as *mut c_void,
                    &mut first_valid_i as *mut _ as *mut c_void,
                    &mut regime_first_valid_i as *mut _ as *mut c_void,
                    &mut commission_f as *mut _ as *mut c_void,
                    &mut eps_rel_f as *mut _ as *mut c_void,
                    &mut stop_log_f as *mut _ as *mut c_void,
                    &mut take_log_f as *mut _ as *mut c_void,
                    &mut flags_u as *mut _ as *mut c_void,
                    &mut m_i as *mut _ as *mut c_void,
                    &mut metrics_stride_i as *mut _ as *mut c_void,
                    &mut pair_map_ptr as *mut _ as *mut c_void,
                    &mut pair_count_i as *mut _ as *mut c_void,
                    &mut compact_output_i as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                unsafe {
                    rt.stream
                        .launch(
                            &func,
                            (launch.grid_x, 1, 1),
                            (launch.block_x, 1, 1),
                            0,
                            args,
                        )
                        .map_err(|e| e.to_string())?;
                }
                rt.stream.synchronize().map_err(|e| e.to_string())?;
                d_out.copy_to(&mut host_out).map_err(|e| e.to_string())?;
                Ok(())
            })?;
            let mut out_metrics = Vec::with_capacity(uploaded_batch.combo_count);
            for &dense_pair_idx in &uploaded_batch.combo_dense_pair_idx {
                let base = dense_pair_idx * out_stride;
                out_metrics.push(decode_tm_flex_metrics_row(&host_out, base, out_stride));
            }
            Ok(out_metrics)
        })
    })
    .ok_or_else(|| "uploaded gpu strategy executable missing from thread-local cache".to_string())?
}

pub fn eval_double_ma_dynamic_program_precomputed_gpu_kernel(
    lr: &[f32],
    fast_row_major: &[f32],
    slow_row_major: &[f32],
    stride: usize,
    fast_periods: &[u16],
    slow_periods: &[u16],
    combos: &[DoubleMaParams],
    first_valid: usize,
    strategy: &StrategyConfig,
    handle: &StrategyHandle,
    device_id: u32,
) -> Result<Vec<Metrics>, String> {
    let uploaded = upload_double_ma_dynamic_program_batch(
        lr,
        fast_row_major,
        slow_row_major,
        stride,
        fast_periods,
        slow_periods,
        combos,
        first_valid,
        strategy,
        device_id,
    )?;
    eval_uploaded_double_ma_dynamic_program_gpu_kernel(&uploaded, strategy, handle)
}

#[derive(Clone, Copy)]
pub(crate) enum RegimeFilterInput<'a> {
    HostF64(&'a [f64]),
    DeviceF32(&'a DeviceRegimeFilter),
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            device_id: 0,
            fast_alma_offset: 0.85,
            fast_alma_sigma: 6.0,
            slow_alma_offset: 0.85,
            slow_alma_sigma: 6.0,
        }
    }
}

pub(crate) fn build_vfct_regime_filter(
    device_id: u32,
    obv: CudaDeviceSliceF32Ref,
    obv_smooth: CudaDeviceSliceF32Ref,
    mfi: CudaDeviceSliceF32Ref,
    first_valid: usize,
    mfi_long_min: f32,
    mfi_short_max: f32,
    confirm_bars: usize,
) -> Result<DeviceRegimeFilter, String> {
    let len = obv.len();
    if len == 0 {
        return Err("vfct device regime build requires a non-empty series".to_string());
    }
    if obv_smooth.len() != len || mfi.len() != len {
        return Err("vfct device regime input length mismatch".to_string());
    }
    if obv.device_id() != device_id
        || obv_smooth.device_id() != device_id
        || mfi.device_id() != device_id
    {
        return Err(format!(
            "vfct device regime input device mismatch (expected device_id={device_id})"
        ));
    }
    if confirm_bars == 0 {
        return Err("vfct confirm_bars must be > 0".to_string());
    }
    if first_valid > len {
        return Err(format!(
            "vfct first_valid {first_valid} exceeds input len {len}"
        ));
    }

    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<DeviceRegimeFilter, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let regime_kernel = rt
            .module
            .get_function("build_vfct_regime_f32")
            .map_err(|e| e.to_string())?;
        let launch_caps =
            rt.launch_caps_for_kernel(CachedKernelKey::BuildVfctRegime, &regime_kernel);
        let launch = choose_launch_shape_1d(len, rt.launch_caps, launch_caps);
        let out = if kernel_async_alloc_enabled() {
            unsafe { DeviceBuffer::<f32>::uninitialized_async(len, &rt.stream) }
        } else {
            unsafe { DeviceBuffer::<f32>::uninitialized(len) }
        }
        .map_err(|e| e.to_string())?;

        unsafe {
            let mut obv_ptr = obv.device_ptr();
            let mut obv_smooth_ptr = obv_smooth.device_ptr();
            let mut mfi_ptr = mfi.device_ptr();
            let mut t_i = len as i32;
            let mut first_valid_i = first_valid as i32;
            let mut long_min = mfi_long_min;
            let mut short_max = mfi_short_max;
            let mut confirm_bars_i = confirm_bars as i32;
            let mut out_ptr = out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut obv_ptr as *mut _ as *mut c_void,
                &mut obv_smooth_ptr as *mut _ as *mut c_void,
                &mut mfi_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut first_valid_i as *mut _ as *mut c_void,
                &mut long_min as *mut _ as *mut c_void,
                &mut short_max as *mut _ as *mut c_void,
                &mut confirm_bars_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(
                    &regime_kernel,
                    (launch.grid_x, 1, 1),
                    (launch.block_x, 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        Ok(DeviceRegimeFilter {
            buffer: out,
            scratch_f32: Vec::new(),
            scratch_dispatch_f32: Vec::new(),
            len,
            first_valid,
            device_id,
        })
    })
}

pub(crate) fn build_pit_regime_filter(
    device_id: u32,
    efi: CudaDeviceSliceF32Ref,
    bop: CudaDeviceSliceF32Ref,
    vosc: CudaDeviceSliceF32Ref,
    acosc: CudaDeviceSliceF32Ref,
    cfo: CudaDeviceSliceF32Ref,
    first_valid: usize,
    confirm_bars: usize,
) -> Result<DeviceRegimeFilter, String> {
    let len = efi.len();
    if len == 0 {
        return Err("pit device regime build requires a non-empty series".to_string());
    }
    if bop.len() != len || vosc.len() != len || acosc.len() != len || cfo.len() != len {
        return Err("pit device regime input length mismatch".to_string());
    }
    if efi.device_id() != device_id
        || bop.device_id() != device_id
        || vosc.device_id() != device_id
        || acosc.device_id() != device_id
        || cfo.device_id() != device_id
    {
        return Err(format!(
            "pit device regime input device mismatch (expected device_id={device_id})"
        ));
    }
    if confirm_bars == 0 {
        return Err("pit confirm_bars must be > 0".to_string());
    }
    if first_valid > len {
        return Err(format!(
            "pit first_valid {first_valid} exceeds input len {len}"
        ));
    }

    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<DeviceRegimeFilter, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let regime_kernel = rt
            .module
            .get_function("build_pit_regime_f32")
            .map_err(|e| e.to_string())?;
        let launch_caps =
            rt.launch_caps_for_kernel(CachedKernelKey::BuildPitRegime, &regime_kernel);
        let launch = choose_launch_shape_1d(len, rt.launch_caps, launch_caps);
        let out = if kernel_async_alloc_enabled() {
            unsafe { DeviceBuffer::<f32>::uninitialized_async(len, &rt.stream) }
        } else {
            unsafe { DeviceBuffer::<f32>::uninitialized(len) }
        }
        .map_err(|e| e.to_string())?;

        unsafe {
            let mut efi_ptr = efi.device_ptr();
            let mut bop_ptr = bop.device_ptr();
            let mut vosc_ptr = vosc.device_ptr();
            let mut acosc_ptr = acosc.device_ptr();
            let mut cfo_ptr = cfo.device_ptr();
            let mut t_i = len as i32;
            let mut first_valid_i = first_valid as i32;
            let mut confirm_bars_i = confirm_bars as i32;
            let mut out_ptr = out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut efi_ptr as *mut _ as *mut c_void,
                &mut bop_ptr as *mut _ as *mut c_void,
                &mut vosc_ptr as *mut _ as *mut c_void,
                &mut acosc_ptr as *mut _ as *mut c_void,
                &mut cfo_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut first_valid_i as *mut _ as *mut c_void,
                &mut confirm_bars_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(
                    &regime_kernel,
                    (launch.grid_x, 1, 1),
                    (launch.block_x, 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        Ok(DeviceRegimeFilter {
            buffer: out,
            scratch_f32: Vec::new(),
            scratch_dispatch_f32: Vec::new(),
            len,
            first_valid,
            device_id,
        })
    })
}

pub(crate) fn build_set_regime_filter(
    device_id: u32,
    angle: CudaDeviceSliceF32Ref,
    er: CudaDeviceSliceF32Ref,
    chop: CudaDeviceSliceF32Ref,
    first_valid: usize,
    angle_min_deg: f32,
    er_min: f32,
    chop_max: f32,
) -> Result<DeviceRegimeFilter, String> {
    let len = angle.len();
    if len == 0 {
        return Err("set device regime build requires a non-empty series".to_string());
    }
    if er.len() != len || chop.len() != len {
        return Err("set device regime input length mismatch".to_string());
    }
    if angle.device_id() != device_id
        || er.device_id() != device_id
        || chop.device_id() != device_id
    {
        return Err(format!(
            "set device regime input device mismatch (expected device_id={device_id})"
        ));
    }
    if first_valid > len {
        return Err(format!(
            "set first_valid {first_valid} exceeds input len {len}"
        ));
    }

    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<DeviceRegimeFilter, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let regime_kernel = rt
            .module
            .get_function("build_set_regime_f32")
            .map_err(|e| e.to_string())?;
        let launch_caps =
            rt.launch_caps_for_kernel(CachedKernelKey::BuildSetRegime, &regime_kernel);
        let launch = choose_launch_shape_1d(len, rt.launch_caps, launch_caps);
        let out = if kernel_async_alloc_enabled() {
            unsafe { DeviceBuffer::<f32>::uninitialized_async(len, &rt.stream) }
        } else {
            unsafe { DeviceBuffer::<f32>::uninitialized(len) }
        }
        .map_err(|e| e.to_string())?;

        unsafe {
            let mut angle_ptr = angle.device_ptr();
            let mut er_ptr = er.device_ptr();
            let mut chop_ptr = chop.device_ptr();
            let mut t_i = len as i32;
            let mut first_valid_i = first_valid as i32;
            let mut angle_min = angle_min_deg;
            let mut er_min_f = er_min;
            let mut chop_max_f = chop_max;
            let mut out_ptr = out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut angle_ptr as *mut _ as *mut c_void,
                &mut er_ptr as *mut _ as *mut c_void,
                &mut chop_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut first_valid_i as *mut _ as *mut c_void,
                &mut angle_min as *mut _ as *mut c_void,
                &mut er_min_f as *mut _ as *mut c_void,
                &mut chop_max_f as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(
                    &regime_kernel,
                    (launch.grid_x, 1, 1),
                    (launch.block_x, 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        Ok(DeviceRegimeFilter {
            buffer: out,
            scratch_f32: Vec::new(),
            scratch_dispatch_f32: Vec::new(),
            len,
            first_valid,
            device_id,
        })
    })
}

pub(crate) fn build_mts_regime_filter(
    device_id: u32,
    roc: CudaDeviceSliceF32Ref,
    mom: CudaDeviceSliceF32Ref,
    ppo: CudaDeviceSliceF32Ref,
    first_valid: usize,
) -> Result<DeviceRegimeFilter, String> {
    let len = roc.len();
    if len == 0 {
        return Err("mts device regime build requires a non-empty series".to_string());
    }
    if mom.len() != len || ppo.len() != len {
        return Err("mts device regime input length mismatch".to_string());
    }
    if roc.device_id() != device_id || mom.device_id() != device_id || ppo.device_id() != device_id
    {
        return Err(format!(
            "mts device regime input device mismatch (expected device_id={device_id})"
        ));
    }
    if first_valid > len {
        return Err(format!(
            "mts first_valid {first_valid} exceeds input len {len}"
        ));
    }

    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<DeviceRegimeFilter, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let regime_kernel = rt
            .module
            .get_function("build_mts_regime_f32")
            .map_err(|e| e.to_string())?;
        let launch_caps =
            rt.launch_caps_for_kernel(CachedKernelKey::BuildMtsRegime, &regime_kernel);
        let launch = choose_launch_shape_1d(len, rt.launch_caps, launch_caps);
        let out = if kernel_async_alloc_enabled() {
            unsafe { DeviceBuffer::<f32>::uninitialized_async(len, &rt.stream) }
        } else {
            unsafe { DeviceBuffer::<f32>::uninitialized(len) }
        }
        .map_err(|e| e.to_string())?;

        unsafe {
            let mut roc_ptr = roc.device_ptr();
            let mut mom_ptr = mom.device_ptr();
            let mut ppo_ptr = ppo.device_ptr();
            let mut t_i = len as i32;
            let mut first_valid_i = first_valid as i32;
            let mut out_ptr = out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut roc_ptr as *mut _ as *mut c_void,
                &mut mom_ptr as *mut _ as *mut c_void,
                &mut ppo_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut first_valid_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(
                    &regime_kernel,
                    (launch.grid_x, 1, 1),
                    (launch.block_x, 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        Ok(DeviceRegimeFilter {
            buffer: out,
            scratch_f32: Vec::new(),
            scratch_dispatch_f32: Vec::new(),
            len,
            first_valid,
            device_id,
        })
    })
}

pub(crate) fn build_orc_regime_filter(
    device_id: u32,
    rsi: CudaDeviceSliceF32Ref,
    stoch_k: CudaDeviceSliceF32Ref,
    willr: CudaDeviceSliceF32Ref,
    first_valid: usize,
) -> Result<DeviceRegimeFilter, String> {
    let len = rsi.len();
    if len == 0 {
        return Err("orc device regime build requires a non-empty series".to_string());
    }
    if stoch_k.len() != len || willr.len() != len {
        return Err("orc device regime input length mismatch".to_string());
    }
    if rsi.device_id() != device_id
        || stoch_k.device_id() != device_id
        || willr.device_id() != device_id
    {
        return Err(format!(
            "orc device regime input device mismatch (expected device_id={device_id})"
        ));
    }
    if first_valid > len {
        return Err(format!(
            "orc first_valid {first_valid} exceeds input len {len}"
        ));
    }

    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<DeviceRegimeFilter, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let regime_kernel = rt
            .module
            .get_function("build_orc_regime_f32")
            .map_err(|e| e.to_string())?;
        let launch_caps =
            rt.launch_caps_for_kernel(CachedKernelKey::BuildOrcRegime, &regime_kernel);
        let launch = choose_launch_shape_1d(len, rt.launch_caps, launch_caps);
        let out = if kernel_async_alloc_enabled() {
            unsafe { DeviceBuffer::<f32>::uninitialized_async(len, &rt.stream) }
        } else {
            unsafe { DeviceBuffer::<f32>::uninitialized(len) }
        }
        .map_err(|e| e.to_string())?;

        unsafe {
            let mut rsi_ptr = rsi.device_ptr();
            let mut stoch_ptr = stoch_k.device_ptr();
            let mut willr_ptr = willr.device_ptr();
            let mut t_i = len as i32;
            let mut first_valid_i = first_valid as i32;
            let mut out_ptr = out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut rsi_ptr as *mut _ as *mut c_void,
                &mut stoch_ptr as *mut _ as *mut c_void,
                &mut willr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut first_valid_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(
                    &regime_kernel,
                    (launch.grid_x, 1, 1),
                    (launch.block_x, 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        Ok(DeviceRegimeFilter {
            buffer: out,
            scratch_f32: Vec::new(),
            scratch_dispatch_f32: Vec::new(),
            len,
            first_valid,
            device_id,
        })
    })
}

#[cfg(test)]
pub(crate) fn download_device_regime_filter(
    device_id: u32,
    regime: &DeviceRegimeFilter,
) -> Result<Vec<f32>, String> {
    let _gpu_guard = lock_gpu_test();
    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<Vec<f32>, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;
        let mut host = alloc_uninit_f32(regime.len);
        regime
            .buffer
            .copy_to(host.as_mut_slice())
            .map_err(|e| e.to_string())?;
        Ok(host)
    })
}

#[inline]
fn validate_precomputed_host_matrix(
    matrix: &PrecomputedHostMaMatrix<'_>,
    expected_periods: &[u16],
    expected_cols: usize,
    side: &str,
) -> Result<(), String> {
    if matrix.cols != expected_cols {
        return Err(format!(
            "precomputed {side} cols mismatch: got {}, expected {expected_cols}",
            matrix.cols
        ));
    }
    if matrix.periods != expected_periods {
        return Err(format!(
            "precomputed {side} periods do not match requested period grid"
        ));
    }
    let expected_len = expected_periods
        .len()
        .checked_mul(expected_cols)
        .ok_or_else(|| format!("precomputed {side} matrix length overflow"))?;
    if matrix.values_row_major.len() != expected_len {
        return Err(format!(
            "precomputed {side} matrix len mismatch: got {}, expected {expected_len}",
            matrix.values_row_major.len()
        ));
    }
    Ok(())
}

fn pack_precomputed_rows_tm_with_padding(
    src: &PrecomputedHostMaMatrix<'_>,
    row_start: usize,
    row_count: usize,
    padded_rows: usize,
    dst: &mut Vec<f32>,
) -> Result<(), String> {
    if row_count == 0 || padded_rows == 0 {
        return Err(
            "precomputed row pack requires non-empty row_count and padded_rows".to_string(),
        );
    }
    if row_start
        .checked_add(row_count)
        .map_or(true, |end| end > src.periods.len())
    {
        return Err("precomputed row pack out of bounds".to_string());
    }
    let cols = src.cols;
    let total = padded_rows
        .checked_mul(cols)
        .ok_or_else(|| "precomputed TM padded buffer overflow".to_string())?;
    resize_uninit_f32(dst, total);

    let base_row_start = row_start
        .checked_mul(cols)
        .ok_or_else(|| "precomputed TM source start overflow".to_string())?;
    for col in 0..cols {
        let dst_col_off = col
            .checked_mul(padded_rows)
            .ok_or_else(|| "precomputed TM destination overflow".to_string())?;
        let mut src_idx = base_row_start + col;
        for row in 0..row_count {
            dst[dst_col_off + row] = src.values_row_major[src_idx];
            src_idx += cols;
        }
        if padded_rows > row_count {
            let last = dst[dst_col_off + row_count - 1];
            for row in row_count..padded_rows {
                dst[dst_col_off + row] = last;
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn remap_precomputed_rows_for_periods(
    src: &PrecomputedHostMaMatrix<'_>,
    selected_periods: &[u16],
    side: &str,
) -> Result<Vec<f32>, String> {
    let cols = src.cols;
    let mut out = Vec::with_capacity(
        selected_periods
            .len()
            .checked_mul(cols)
            .ok_or_else(|| format!("precomputed {side} remap overflow"))?,
    );
    for &period in selected_periods {
        let row = src
            .periods
            .iter()
            .position(|&p| p == period)
            .ok_or_else(|| {
                format!("precomputed {side} matrix missing period {period} for recompute subset")
            })?;
        let start = row
            .checked_mul(cols)
            .ok_or_else(|| format!("precomputed {side} remap start overflow"))?;
        let end = start
            .checked_add(cols)
            .ok_or_else(|| format!("precomputed {side} remap end overflow"))?;
        out.extend_from_slice(&src.values_row_major[start..end]);
    }
    Ok(out)
}

fn expand_u16_range((start, end, step): (u32, u32, u32)) -> Result<Vec<u16>, String> {
    let push_checked = |out: &mut Vec<u16>, v: u32| -> Result<(), String> {
        if v > u16::MAX as u32 {
            return Err(format!("period {v} exceeds u16::MAX"));
        }
        out.push(v as u16);
        Ok(())
    };

    if step == 0 || start == end {
        let mut out = Vec::with_capacity(1);
        push_checked(&mut out, start)?;
        return Ok(out);
    }

    let step = step.max(1);
    let mut out = Vec::new();
    if start <= end {
        let mut v = start;
        loop {
            push_checked(&mut out, v)?;
            if v == end {
                break;
            }
            match v.checked_add(step) {
                Some(next) if next > v && next <= end => v = next,
                _ => break,
            }
        }
    } else {
        let mut v = start;
        loop {
            push_checked(&mut out, v)?;
            if v == end {
                break;
            }
            let next = v.saturating_sub(step);
            if next == v || next < end {
                break;
            }
            v = next;
        }
    }
    if out.is_empty() {
        return Err("empty period range".to_string());
    }
    Ok(out)
}

#[inline]
fn build_valid_pair_index_map_i32(
    fast_periods: &[u16],
    slow_periods: &[u16],
) -> Result<Vec<i32>, String> {
    let ps = slow_periods.len();
    if ps == 0 || fast_periods.is_empty() {
        return Ok(Vec::new());
    }
    let mut out: Vec<i32> = Vec::with_capacity(fast_periods.len().saturating_mul(ps));
    for (fi, &f) in fast_periods.iter().enumerate() {
        let j0 = slow_periods.partition_point(|&s| s <= f);
        for sj in j0..ps {
            let idx = fi
                .checked_mul(ps)
                .and_then(|v| v.checked_add(sj))
                .ok_or_else(|| "valid pair index overflow".to_string())?;
            if idx > i32::MAX as usize {
                return Err("valid pair index exceeds i32::MAX".to_string());
            }
            out.push(idx as i32);
        }
    }
    Ok(out)
}

#[inline]
fn build_valid_pair_period_rows_i32(
    fast_periods: &[u16],
    slow_periods: &[u16],
) -> (Vec<i32>, Vec<i32>) {
    let ps = slow_periods.len();
    let mut fast_rows: Vec<i32> = Vec::with_capacity(fast_periods.len().saturating_mul(ps));
    let mut slow_rows: Vec<i32> = Vec::with_capacity(fast_periods.len().saturating_mul(ps));
    for &fast_len in fast_periods {
        let j0 = slow_periods.partition_point(|&s| s <= fast_len);
        for &slow_len in &slow_periods[j0..] {
            fast_rows.push(i32::from(fast_len));
            slow_rows.push(i32::from(slow_len));
        }
    }
    (fast_rows, slow_rows)
}

#[inline]
fn for_each_tile_metric_row(
    fast_chunk: &[u16],
    slow_chunk: &[u16],
    compact_rows: bool,
    mut f: impl FnMut(usize, u16, u16) -> Result<(), String>,
) -> Result<(), String> {
    if compact_rows {
        let valid_pairs = build_valid_pair_index_map_i32(fast_chunk, slow_chunk)?;
        let ps = slow_chunk.len();
        for (row_idx, dense_pair_i32) in valid_pairs.into_iter().enumerate() {
            let dense_pair = usize::try_from(dense_pair_i32)
                .map_err(|_| "negative dense pair index in valid pair map".to_string())?;
            let i = dense_pair / ps;
            let j = dense_pair % ps;
            f(row_idx, fast_chunk[i], slow_chunk[j])?;
        }
        Ok(())
    } else {
        let ps = slow_chunk.len();
        for (i, &fast_len) in fast_chunk.iter().enumerate() {
            for (j, &slow_len) in slow_chunk.iter().enumerate() {
                if fast_len >= slow_len {
                    continue;
                }
                f(i * ps + j, fast_len, slow_len)?;
            }
        }
        Ok(())
    }
}

#[inline]
fn first_finite_idx(series: &[f64]) -> Option<usize> {
    series.iter().position(|v| v.is_finite())
}

#[inline]
fn ma_uses_source(ma: &str) -> bool {
    ma != "frama"
}

#[inline]
fn ma_requires_volume(ma: &str) -> bool {
    ma == "vwma"
}

#[inline]
fn ma_requires_high_low_close(ma: &str) -> bool {
    ma == "frama"
}

#[inline]
fn validate_candles_for_selected_mas(
    candles: &Candles,
    needs_ma_prices: bool,
    ma_source: &str,
    needs_volume: bool,
    needs_high_low: bool,
) -> Result<(), String> {
    if !candles.fields.close {
        return Err("CSV is missing required 'close' data".to_string());
    }
    if needs_volume && !candles.fields.volume {
        return Err("Selected MA requires 'volume' but CSV has no volume column".to_string());
    }
    if needs_high_low && (!candles.fields.high || !candles.fields.low) {
        return Err(
            "Selected MA requires 'high'/'low' but CSV is missing those columns".to_string(),
        );
    }

    if needs_ma_prices {
        let src = ma_source.trim().to_ascii_lowercase();
        match src.as_str() {
            "close" => {}
            "open" => {
                if !candles.fields.open {
                    return Err("ma_source=open requires an 'open' column in the CSV".to_string());
                }
            }
            "high" => {
                if !candles.fields.high {
                    return Err("ma_source=high requires a 'high' column in the CSV".to_string());
                }
            }
            "low" => {
                if !candles.fields.low {
                    return Err("ma_source=low requires a 'low' column in the CSV".to_string());
                }
            }
            "hl2" => {
                if !candles.fields.high || !candles.fields.low {
                    return Err(
                        "ma_source=hl2 requires 'high'/'low' columns in the CSV".to_string()
                    );
                }
            }
            "hlc3" => {
                if !candles.fields.high || !candles.fields.low || !candles.fields.close {
                    return Err(
                        "ma_source=hlc3 requires 'high'/'low'/'close' columns in the CSV"
                            .to_string(),
                    );
                }
            }
            "hlcc" | "hlcc4" => {
                if !candles.fields.high || !candles.fields.low || !candles.fields.close {
                    return Err(
                        "ma_source=hlcc requires 'high'/'low'/'close' columns in the CSV"
                            .to_string(),
                    );
                }
            }
            "ohlc4" => {
                if !candles.fields.open
                    || !candles.fields.high
                    || !candles.fields.low
                    || !candles.fields.close
                {
                    return Err(
                        "ma_source=ohlc4 requires 'open'/'high'/'low'/'close' columns in the CSV"
                            .to_string(),
                    );
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn first_valid_idx_required(
    close: &[f64],
    ma_prices: Option<&[f64]>,
    volume: Option<&[f64]>,
    high: Option<&[f64]>,
    low: Option<&[f64]>,
    start_at: usize,
) -> Option<usize> {
    let n = close.len();
    let start_at = start_at.min(n);
    for i in start_at..n {
        if !close[i].is_finite() {
            continue;
        }
        if let Some(p) = ma_prices {
            if !p[i].is_finite() {
                continue;
            }
        }
        if let Some(v) = volume {
            if !v[i].is_finite() {
                continue;
            }
        }
        if let Some(h) = high {
            if !h[i].is_finite() {
                continue;
            }
        }
        if let Some(l) = low {
            if !l[i].is_finite() {
                continue;
            }
        }
        return Some(i);
    }
    None
}

fn vram_budget_bytes() -> usize {
    let headroom = 512usize * 1024 * 1024;
    let min_budget = 64usize * 1024 * 1024;

    #[cfg(test)]
    if let Some(mb) = TEST_VRAM_BUDGET_MB.with(|cell| cell.get()) {
        let mut bytes = mb.saturating_mul(1024).saturating_mul(1024);
        if bytes > headroom {
            bytes = bytes.saturating_sub(headroom);
        } else {
            bytes = bytes / 2;
        }
        return bytes.max(min_budget);
    }

    match std::env::var("VECTORBT_KERNEL_VRAM_BUDGET_MB")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    {
        Some(mb) => {
            let mut bytes = mb.saturating_mul(1024).saturating_mul(1024);
            if bytes > headroom {
                bytes = bytes.saturating_sub(headroom);
            } else {
                bytes = bytes / 2;
            }
            bytes.max(min_budget)
        }
        None => mem_get_info()
            .ok()
            .map(|(free, _total)| {
                let usable = free.saturating_sub(headroom);
                if usable == 0 {
                    let half = free.saturating_mul(50).saturating_div(100);
                    return half.max(min_budget.min(free));
                }
                let pct = 85usize;
                let budget = usable.saturating_mul(pct).saturating_div(100);
                let floor = (256usize * 1024 * 1024).min(usable);
                budget.max(floor).max(min_budget.min(usable)).min(usable)
            })
            .unwrap_or(256usize * 1024 * 1024),
    }
}

#[inline]
fn kernel_persist_buffers_enabled() -> bool {
    match std::env::var("VECTORBT_KERNEL_PERSIST_BUFFERS") {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false"))
        }
        Err(_) => true,
    }
}

#[inline]
fn kernel_fast_path_enabled(fast_ma: &str, slow_ma: &str) -> bool {
    if !matches!(fast_ma, "sma" | "alma") || !matches!(slow_ma, "sma" | "alma") {
        return false;
    }
    match std::env::var("VECTORBT_KERNEL_FAST_PATH") {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false"))
        }
        Err(_) => true,
    }
}

#[inline]
fn env_flag_enabled(name: &str, default_enabled: bool) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false"))
        }
        Err(_) => default_enabled,
    }
}

#[inline]
fn env_flag_override(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|v| {
        let v = v.trim();
        !(v == "0" || v.eq_ignore_ascii_case("false"))
    })
}

#[inline]
fn kernel_async_alloc_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_ASYNC_ALLOC", true)
}

#[inline]
fn kernel_pinned_h2d_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_PINNED_H2D", true)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CandleDatasetSignature {
    len: usize,
    timestamp_ptr: usize,
    open_ptr: usize,
    high_ptr: usize,
    low_ptr: usize,
    close_ptr: usize,
    volume_ptr: usize,
    field_mask: u8,
}

#[derive(Debug)]
struct ResidentMarketSeriesCache {
    signature: CandleDatasetSignature,
    close: DeviceBuffer<f32>,
    high: Option<DeviceBuffer<f32>>,
    low: Option<DeviceBuffer<f32>>,
    volume: Option<DeviceBuffer<f32>>,
    source_overrides: HashMap<String, DeviceBuffer<f32>>,
}

const MARKET_FIELD_OPEN: u8 = 1 << 0;
const MARKET_FIELD_HIGH: u8 = 1 << 1;
const MARKET_FIELD_LOW: u8 = 1 << 2;
const MARKET_FIELD_CLOSE: u8 = 1 << 3;
const MARKET_FIELD_VOLUME: u8 = 1 << 4;

#[inline]
fn candle_dataset_signature(candles: &Candles) -> CandleDatasetSignature {
    let mut field_mask = 0u8;
    if candles.fields.open {
        field_mask |= MARKET_FIELD_OPEN;
    }
    if candles.fields.high {
        field_mask |= MARKET_FIELD_HIGH;
    }
    if candles.fields.low {
        field_mask |= MARKET_FIELD_LOW;
    }
    if candles.fields.close {
        field_mask |= MARKET_FIELD_CLOSE;
    }
    if candles.fields.volume {
        field_mask |= MARKET_FIELD_VOLUME;
    }

    CandleDatasetSignature {
        len: candles.close.len(),
        timestamp_ptr: candles.timestamp.as_ptr() as usize,
        open_ptr: candles.open.as_ptr() as usize,
        high_ptr: candles.high.as_ptr() as usize,
        low_ptr: candles.low.as_ptr() as usize,
        close_ptr: candles.close.as_ptr() as usize,
        volume_ptr: candles.volume.as_ptr() as usize,
        field_mask,
    }
}

#[inline]
fn normalize_market_source_key(ma_source: &str) -> String {
    let trimmed = ma_source.trim();
    if trimmed.is_empty() {
        return "close".to_string();
    }
    trimmed.to_ascii_lowercase()
}

fn reset_resident_market_cache(
    stream: &Stream,
    resident: &mut Option<ResidentMarketSeriesCache>,
) -> Result<(), String> {
    if resident.is_some() {
        stream.synchronize().map_err(|e| e.to_string())?;
        *resident = None;
    }
    Ok(())
}

fn upload_resident_series_f64(stream: &Stream, src: &[f64]) -> Result<DeviceBuffer<f32>, String> {
    let use_pinned = kernel_pinned_h2d_enabled();
    let mut guard = AsyncPinnedUploadGuard::new(stream);
    let buf = upload_f32_source_to_device(stream, HostF32Source::F64(src), use_pinned, &mut guard)?;
    guard.finalize()?;
    Ok(buf)
}

fn upload_ephemeral_series_f32(stream: &Stream, src: &[f32]) -> Result<DeviceBuffer<f32>, String> {
    let use_pinned = kernel_pinned_h2d_enabled();
    let mut guard = AsyncPinnedUploadGuard::new(stream);
    let buf = upload_f32_source_to_device(stream, HostF32Source::F32(src), use_pinned, &mut guard)?;
    guard.finalize()?;
    Ok(buf)
}

struct ResidentUploadBatch<'a> {
    stream: &'a Stream,
    guard: AsyncPinnedUploadGuard<'a>,
    use_pinned: bool,
}

impl<'a> ResidentUploadBatch<'a> {
    #[inline]
    fn new(stream: &'a Stream) -> Self {
        Self {
            stream,
            guard: AsyncPinnedUploadGuard::new(stream),
            use_pinned: kernel_pinned_h2d_enabled(),
        }
    }

    #[inline]
    fn upload_f64(&mut self, src: &[f64]) -> Result<DeviceBuffer<f32>, String> {
        upload_f32_source_to_device(
            self.stream,
            HostF32Source::F64(src),
            self.use_pinned,
            &mut self.guard,
        )
    }

    #[inline]
    fn finish(self) -> Result<(), String> {
        self.guard.finalize()
    }
}

fn ensure_resident_market_series(
    stream: &Stream,
    resident: &mut Option<ResidentMarketSeriesCache>,
    candles: &Candles,
    close: &[f64],
    ma_source: &str,
    ma_prices: Option<&[f64]>,
    needs_ma_prices: bool,
    needs_high_low: bool,
    high: Option<&[f64]>,
    low: Option<&[f64]>,
    needs_volume: bool,
    volume: Option<&[f64]>,
) -> Result<(), String> {
    let signature = candle_dataset_signature(candles);
    let source_key = normalize_market_source_key(ma_source);

    let should_reset = resident
        .as_ref()
        .map(|cache| cache.signature != signature)
        .unwrap_or(true);
    if should_reset {
        reset_resident_market_cache(stream, resident)?;
        let mut batch = ResidentUploadBatch::new(stream);
        let mut cache = ResidentMarketSeriesCache {
            signature,
            close: batch.upload_f64(close)?,
            high: None,
            low: None,
            volume: None,
            source_overrides: HashMap::new(),
        };

        if needs_high_low {
            cache.high = Some(batch.upload_f64(
                high.ok_or_else(|| "high prices required for resident upload".to_string())?,
            )?);
            cache.low = Some(batch.upload_f64(
                low.ok_or_else(|| "low prices required for resident upload".to_string())?,
            )?);
        }

        if needs_volume {
            cache.volume = Some(batch.upload_f64(
                volume.ok_or_else(|| "volume prices required for resident upload".to_string())?,
            )?);
        }

        if needs_ma_prices && source_key != "close" {
            let uploaded =
                batch
                    .upload_f64(ma_prices.ok_or_else(|| {
                        format!("ma prices required for ma_source={source_key}")
                    })?)?;
            cache.source_overrides.insert(source_key.clone(), uploaded);
        }

        batch.finish()?;
        *resident = Some(cache);
    }

    let cache = resident
        .as_mut()
        .ok_or_else(|| "resident market cache unavailable".to_string())?;

    let mut batch = ResidentUploadBatch::new(stream);

    if needs_high_low {
        if cache.high.is_none() {
            cache.high = Some(batch.upload_f64(
                high.ok_or_else(|| "high prices required for resident upload".to_string())?,
            )?);
        }
        if cache.low.is_none() {
            cache.low = Some(batch.upload_f64(
                low.ok_or_else(|| "low prices required for resident upload".to_string())?,
            )?);
        }
    }

    if needs_volume && cache.volume.is_none() {
        cache.volume = Some(batch.upload_f64(
            volume.ok_or_else(|| "volume prices required for resident upload".to_string())?,
        )?);
    }

    if needs_ma_prices && source_key != "close" && !cache.source_overrides.contains_key(&source_key)
    {
        let uploaded = batch.upload_f64(
            ma_prices.ok_or_else(|| format!("ma prices required for ma_source={source_key}"))?,
        )?;
        cache.source_overrides.insert(source_key, uploaded);
    }

    batch.finish()?;
    Ok(())
}

struct AsyncPinnedUploadGuard<'a> {
    stream: &'a Stream,
    pinned: Vec<LockedBuffer<f32>>,
    finalized: bool,
}

impl<'a> AsyncPinnedUploadGuard<'a> {
    #[inline]
    fn new(stream: &'a Stream) -> Self {
        Self {
            stream,
            pinned: Vec::new(),
            finalized: false,
        }
    }

    #[inline]
    fn push(&mut self, buf: LockedBuffer<f32>) {
        self.pinned.push(buf);
    }

    #[inline]
    fn finalize(mut self) -> Result<(), String> {
        if !self.pinned.is_empty() {
            self.stream.synchronize().map_err(|e| e.to_string())?;
        }
        self.finalized = true;
        Ok(())
    }
}

impl Drop for AsyncPinnedUploadGuard<'_> {
    fn drop(&mut self) {
        if !self.finalized && !self.pinned.is_empty() {
            let _ = self.stream.synchronize();
        }
    }
}

enum HostF32Source<'a> {
    F32(&'a [f32]),
    F64(&'a [f64]),
}

impl HostF32Source<'_> {
    #[inline]
    fn len(&self) -> usize {
        match self {
            HostF32Source::F32(src) => src.len(),
            HostF32Source::F64(src) => src.len(),
        }
    }
}

#[inline]
fn cast_f64_slice_to_f32_vec(src: &[f64]) -> Vec<f32> {
    let mut out = alloc_uninit_f32(src.len());
    for (dst, &v) in out.iter_mut().zip(src.iter()) {
        *dst = v as f32;
    }
    out
}

#[inline]
fn upload_f32_source_to_device(
    stream: &Stream,
    src: HostF32Source<'_>,
    use_pinned: bool,
    guard: &mut AsyncPinnedUploadGuard<'_>,
) -> Result<DeviceBuffer<f32>, String> {
    let len = src.len();
    if use_pinned {
        let mut locked =
            unsafe { LockedBuffer::<f32>::uninitialized(len) }.map_err(|e| e.to_string())?;
        match src {
            HostF32Source::F32(slice) => {
                locked.as_mut_slice().copy_from_slice(slice);
            }
            HostF32Source::F64(slice) => {
                for (dst, &v) in locked.as_mut_slice().iter_mut().zip(slice.iter()) {
                    *dst = v as f32;
                }
            }
        }
        if len > 0 {
            let mut dev = unsafe { DeviceBuffer::<f32>::uninitialized_async(len, stream) }
                .map_err(|e| e.to_string())?;
            unsafe { dev.async_copy_from(locked.as_ref(), stream) }.map_err(|e| e.to_string())?;
            guard.push(locked);
            return Ok(dev);
        }
        // Empty buffers are valid and should avoid async copy submission.
        return DeviceBuffer::from_slice(&[]).map_err(|e| e.to_string());
    }

    match src {
        HostF32Source::F32(slice) => DeviceBuffer::from_slice(slice).map_err(|e| e.to_string()),
        HostF32Source::F64(slice) => {
            let tmp = cast_f64_slice_to_f32_vec(slice);
            DeviceBuffer::from_slice(&tmp).map_err(|e| e.to_string())
        }
    }
}

#[inline]
fn kernel_cuda_graphs_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_USE_CUDA_GRAPHS", false)
}

#[inline]
fn kernel_pipeline_profile_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_PROFILE_PIPELINE", false)
}

#[inline]
fn kernel_force_legacy_topk_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_FORCE_LEGACY_TOPK", false)
}

#[inline]
fn kernel_global_topk_enabled() -> bool {
    // Keep top-k reductions on device by default to avoid per-tile D2H traffic.
    env_flag_enabled("VECTORBT_KERNEL_GLOBAL_TOPK", true)
}

#[inline]
fn kernel_finalist_recompute_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_FINALIST_RECOMPUTE", true)
}

#[inline]
fn kernel_log_dispatch_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_LOG_DISPATCH", false)
}

#[inline]
fn kernel_valid_pair_map_enabled() -> bool {
    // Compact dense Pf*Ps launch space into valid fast<slow pairs only.
    env_flag_enabled("VECTORBT_KERNEL_VALID_PAIR_MAP", true)
}

#[inline]
fn kernel_valid_pair_map_max_pairs() -> usize {
    std::env::var("VECTORBT_KERNEL_VALID_PAIR_MAP_MAX_PAIRS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(150_000)
}

#[inline]
fn kernel_tm_padding_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_TM_PAD", false)
}

#[inline]
fn kernel_fast_warp_dedup_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_FAST_WARP_DEDUP", false)
}

#[inline]
fn kernel_fast_warp_dedup_min_pairs() -> usize {
    std::env::var("VECTORBT_KERNEL_FAST_WARP_DEDUP_MIN_PAIRS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(150_000)
}

#[inline]
fn kernel_lockstep_time_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_LOCKSTEP_TIME", true)
}

#[inline]
fn kernel_lockstep_time_min_pairs() -> usize {
    std::env::var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIRS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(150_000)
}

#[inline]
fn kernel_lockstep_time_min_pair_bars() -> Option<u64> {
    match std::env::var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIR_BARS") {
        Ok(v) => v.trim().parse::<u64>().ok().filter(|&v| v > 0),
        Err(_) => Some(5_000_000_000),
    }
}

#[inline]
fn kernel_should_use_lockstep_time(
    pair_count: usize,
    series_len: usize,
    first_valid: usize,
    kernel_available: bool,
) -> bool {
    if !kernel_lockstep_time_enabled() || !kernel_available {
        return false;
    }

    if pair_count >= kernel_lockstep_time_min_pairs() {
        return true;
    }

    kernel_lockstep_time_min_pair_bars()
        .map(|min_pair_bars| {
            kernel_effective_pair_bars(pair_count, series_len, first_valid) >= min_pair_bars
        })
        .unwrap_or(false)
}

#[inline]
fn kernel_m7_specialize_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_M7_SPECIALIZE", true)
}

#[inline]
fn kernel_m7_specialize_min_pairs() -> usize {
    std::env::var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIRS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(300_000)
}

#[inline]
fn kernel_m7_specialize_min_pair_bars() -> Option<u64> {
    match std::env::var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIR_BARS") {
        Ok(v) => v.trim().parse::<u64>().ok().filter(|&v| v > 0),
        Err(_) => Some(5_000_000_000),
    }
}

#[inline]
fn kernel_effective_pair_bars(pair_count: usize, series_len: usize, first_valid: usize) -> u64 {
    let effective_bars = series_len.saturating_sub(first_valid).max(1);
    (pair_count as u64).saturating_mul(effective_bars as u64)
}

#[inline]
fn kernel_should_use_m7_specialize(
    pair_count: usize,
    series_len: usize,
    first_valid: usize,
    is_multi_tile_workload: bool,
    metrics_count: usize,
    kernel_available: bool,
) -> bool {
    if !kernel_m7_specialize_enabled()
        || is_multi_tile_workload
        || metrics_count < BASE_METRICS_COUNT
        || !kernel_available
    {
        return false;
    }

    if pair_count >= kernel_m7_specialize_min_pairs() {
        return true;
    }

    kernel_m7_specialize_min_pair_bars()
        .map(|min_pair_bars| {
            kernel_effective_pair_bars(pair_count, series_len, first_valid) >= min_pair_bars
        })
        .unwrap_or(false)
}

#[inline]
fn should_force_low_pair_m7_recompute(
    prefer_low_pair_m7_dispatch: bool,
    use_windowed_eval: bool,
    is_multi_tile_workload: bool,
    launch_pairs: usize,
    kernel_available: bool,
) -> bool {
    prefer_low_pair_m7_dispatch
        && kernel_m7_specialize_enabled()
        && !use_windowed_eval
        && !is_multi_tile_workload
        && launch_pairs > 0
        && launch_pairs <= 128
        && kernel_available
}

#[inline]
fn kernel_lockstep_m7_regime_mode_specialize_enabled_for_pair(
    fast_ma: &str,
    slow_ma: &str,
) -> bool {
    if let Some(v) = env_flag_override("VECTORBT_KERNEL_LOCKSTEP_M7_REGIME_MODE_SPECIALIZE") {
        return v;
    }
    fast_ma == "alma" && slow_ma == "alma"
}

#[inline]
fn kernel_lockstep_m7_norisk_specialize_enabled_for_pair(_fast_ma: &str, _slow_ma: &str) -> bool {
    if let Some(v) = env_flag_override("VECTORBT_KERNEL_LOCKSTEP_M7_NORISK_SPECIALIZE") {
        return v;
    }
    true
}

#[inline]
fn kernel_lockstep_m7_eps0_specialize_enabled_for_pair(fast_ma: &str, slow_ma: &str) -> bool {
    if let Some(v) = env_flag_override("VECTORBT_KERNEL_LOCKSTEP_M7_EPS0_SPECIALIZE") {
        return v;
    }
    fast_ma == "alma" && slow_ma == "alma"
}

fn kernel_metrics_pad8_enabled_for_pair(fast_ma: &str, slow_ma: &str) -> bool {
    // Pad legacy 7-float rows to 8 floats to improve alignment/coalescing in tile buffers.
    // Wider rows should use their natural stride.
    // Default only for ALMA/ALMA (current proven win); env override still wins.
    if let Some(v) = env_flag_override("VECTORBT_KERNEL_METRICS_PAD8") {
        return v;
    }
    fast_ma == "alma" && slow_ma == "alma"
}

#[inline]
fn kernel_metrics_full_stride_override() -> Option<usize> {
    std::env::var("VECTORBT_KERNEL_METRICS_FULL_STRIDE")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&v| v >= METRICS_COUNT)
}

#[inline]
fn effective_metrics_stride(
    metrics_count: usize,
    pad8_enabled: bool,
    full_stride_override: Option<usize>,
) -> usize {
    if metrics_count == METRICS_COUNT {
        if let Some(v) = kernel_metrics_full_stride_override() {
            return v;
        }
        if let Some(v) = full_stride_override {
            return v;
        }
    }
    if pad8_enabled && metrics_count == BASE_METRICS_COUNT {
        8
    } else {
        metrics_count
    }
}

#[inline]
fn search_metrics_count_for_optimization(
    objective: ObjectiveKind,
    constraints: Option<&DoubleMaConstraints>,
    include_all: bool,
    export_enabled: bool,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
) -> usize {
    if include_all || export_enabled || fast_precomputed_enabled || slow_precomputed_enabled {
        return METRICS_COUNT;
    }

    let mut metrics_count = match objective {
        ObjectiveKind::Pnl => 1,
        ObjectiveKind::Sharpe | ObjectiveKind::Volatility => 5,
        ObjectiveKind::MaxDrawdown | ObjectiveKind::Calmar => 3,
        ObjectiveKind::Sortino | ObjectiveKind::Omega => METRICS_COUNT,
    };

    if constraints.and_then(|c| c.min_trades).is_some() {
        metrics_count = metrics_count.max(2);
    }
    if constraints.and_then(|c| c.max_drawdown).is_some() {
        metrics_count = metrics_count.max(3);
    }

    metrics_count.min(METRICS_COUNT)
}

#[inline]
fn should_use_compact_valid_pair_output(
    _include_all: bool,
    _export_enabled: bool,
    _host_filter_mode: bool,
    _metrics_count: usize,
) -> bool {
    true
}

#[inline]
fn cu_memcpy_d2d_async(
    dst: cuda_sys::CUdeviceptr,
    src: cuda_sys::CUdeviceptr,
    bytes: usize,
    stream: &Stream,
) -> Result<(), String> {
    if bytes == 0 {
        return Ok(());
    }
    cu_result(unsafe { cuda_sys::cuMemcpyDtoDAsync_v2(dst, src, bytes, stream.as_inner()) })
}

#[inline]
fn copy_device_prefix_to_host_f32(
    src: &DeviceBuffer<f32>,
    dst: &mut Vec<f32>,
    elems: usize,
    stream: &Stream,
) -> Result<(), String> {
    resize_uninit_f32(dst, elems);
    if elems == 0 {
        return Ok(());
    }
    let bytes = elems
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or_else(|| "device-to-host copy size overflow".to_string())?;
    cu_result(unsafe {
        cuda_sys::cuMemcpyDtoHAsync_v2(
            dst.as_mut_ptr() as *mut c_void,
            src.as_device_ptr().as_raw(),
            bytes,
            stream.as_inner(),
        )
    })
}

#[inline]
fn pack_metric_rows_for_host_copy(
    stream: &Stream,
    scratch: &mut KernelScratch,
    pack_metric_rows: &Function,
    fast_rows_ptr: cuda_sys::CUdeviceptr,
    slow_rows_ptr: cuda_sys::CUdeviceptr,
    metrics_rows_ptr: cuda_sys::CUdeviceptr,
    rows: usize,
    metrics_count: usize,
    host_out: &mut Vec<f32>,
) -> Result<(), String> {
    let packed_stride = metrics_count.saturating_add(2);
    let packed_len = rows.saturating_mul(packed_stride);
    KernelScratch::ensure_len_f32(&mut scratch.d_top_packed, packed_len, stream)?;
    if rows > 0 {
        unsafe {
            let mut fast_ptr = fast_rows_ptr;
            let mut slow_ptr = slow_rows_ptr;
            let mut metrics_ptr = metrics_rows_ptr;
            let mut rows_i = rows as i32;
            let mut m_i = metrics_count as i32;
            let mut out_ptr = scratch
                .d_top_packed
                .as_ref()
                .ok_or_else(|| "missing packed top-row buffer".to_string())?
                .as_device_ptr()
                .as_raw();
            let block_x: u32 = 256;
            let grid_x: u32 = ((rows as u32) + block_x - 1) / block_x;
            let args: &mut [*mut c_void] = &mut [
                &mut fast_ptr as *mut _ as *mut c_void,
                &mut slow_ptr as *mut _ as *mut c_void,
                &mut metrics_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            stream
                .launch(pack_metric_rows, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
    }
    copy_device_prefix_to_host_f32(
        scratch
            .d_top_packed
            .as_ref()
            .ok_or_else(|| "missing packed top-row buffer".to_string())?,
        host_out,
        packed_len,
        stream,
    )?;
    stream.synchronize().map_err(|e| e.to_string())
}

#[inline]
fn scaled_vdps_counts(
    sample_total: usize,
    elite_target: usize,
    diff_target: usize,
    rand_target: usize,
) -> (usize, usize, usize) {
    let target_total = elite_target
        .saturating_add(diff_target)
        .saturating_add(rand_target);
    if sample_total == 0 || target_total == 0 {
        return (0, 0, 0);
    }
    let ratio = sample_total as f64 / (target_total as f64);
    let mut elite = ((elite_target as f64) * ratio).round() as usize;
    let mut diff = ((diff_target as f64) * ratio).round() as usize;
    let mut rand = ((rand_target as f64) * ratio).round() as usize;
    let assigned = elite.saturating_add(diff).saturating_add(rand);
    if assigned > sample_total {
        let overflow = assigned.saturating_sub(sample_total);
        let take_rand = overflow.min(rand);
        rand = rand.saturating_sub(take_rand);
        let rem = overflow.saturating_sub(take_rand);
        let take_diff = rem.min(diff);
        diff = diff.saturating_sub(take_diff);
        let rem2 = rem.saturating_sub(take_diff);
        elite = elite.saturating_sub(rem2.min(elite));
    } else if assigned < sample_total {
        rand = rand.saturating_add(sample_total.saturating_sub(assigned));
    }
    (elite, diff, rand)
}

pub fn vdps_generate_seed_pairs_gpu_kernel(
    device_id: u32,
    elite_seeds: &[(u16, u16)],
    fast_periods: &[u16],
    slow_periods: &[u16],
    elite_target: usize,
    diff_target: usize,
    rand_target: usize,
    fast_step: u16,
    slow_step: u16,
    seed: u64,
) -> Result<Option<Vec<(u16, u16)>>, String> {
    let target_total = elite_target
        .saturating_add(diff_target)
        .saturating_add(rand_target);
    if target_total == 0 || fast_periods.is_empty() || slow_periods.is_empty() {
        return Ok(Some(Vec::new()));
    }

    // Oversample to offset duplicate/invalid pairs from nearest-period projection.
    let sample_total = target_total.saturating_mul(4).max(target_total).min(16_384);
    let (elite_count, diff_count, rand_count) =
        scaled_vdps_counts(sample_total, elite_target, diff_target, rand_target);

    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<Option<Vec<(u16, u16)>>, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            let rt = CudaKernelRuntime::new(device_id)?;
            map.insert(device_id, rt);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let mut elite_fast_h: Vec<i32> = Vec::with_capacity(elite_seeds.len().max(1));
        let mut elite_slow_h: Vec<i32> = Vec::with_capacity(elite_seeds.len().max(1));
        for &(f, s) in elite_seeds.iter() {
            elite_fast_h.push(f as i32);
            elite_slow_h.push(s as i32);
        }
        let elite_len = elite_fast_h.len() as i32;
        if elite_fast_h.is_empty() {
            elite_fast_h.push(0);
            elite_slow_h.push(0);
        }
        let fast_h: Vec<i32> = fast_periods.iter().map(|&p| p as i32).collect();
        let slow_h: Vec<i32> = slow_periods.iter().map(|&p| p as i32).collect();

        let d_elite_fast = DeviceBuffer::from_slice(&elite_fast_h).map_err(|e| e.to_string())?;
        let d_elite_slow = DeviceBuffer::from_slice(&elite_slow_h).map_err(|e| e.to_string())?;
        let d_fast = DeviceBuffer::from_slice(&fast_h).map_err(|e| e.to_string())?;
        let d_slow = DeviceBuffer::from_slice(&slow_h).map_err(|e| e.to_string())?;
        let d_out_fast = unsafe { DeviceBuffer::<i32>::uninitialized(sample_total) }
            .map_err(|e| e.to_string())?;
        let d_out_slow = unsafe { DeviceBuffer::<i32>::uninitialized(sample_total) }
            .map_err(|e| e.to_string())?;

        let kernel = match rt.module.get_function("vdps_generate_seed_pairs_tm_i32") {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };
        let stream = &rt.stream;
        unsafe {
            let mut elite_fast_ptr = d_elite_fast.as_device_ptr().as_raw();
            let mut elite_slow_ptr = d_elite_slow.as_device_ptr().as_raw();
            let mut elite_len_i = elite_len;
            let mut fast_ptr = d_fast.as_device_ptr().as_raw();
            let mut fast_len_i = fast_h.len() as i32;
            let mut slow_ptr = d_slow.as_device_ptr().as_raw();
            let mut slow_len_i = slow_h.len() as i32;
            let mut elite_count_i = elite_count as i32;
            let mut diff_count_i = diff_count as i32;
            let mut rand_count_i = rand_count as i32;
            let mut fast_step_i = fast_step.max(1) as i32;
            let mut slow_step_i = slow_step.max(1) as i32;
            let mut seed_u64 = seed;
            let mut out_len_i = sample_total as i32;
            let mut out_fast_ptr = d_out_fast.as_device_ptr().as_raw();
            let mut out_slow_ptr = d_out_slow.as_device_ptr().as_raw();

            let block_x: u32 = 256;
            let grid_x: u32 = ((sample_total as u32) + block_x - 1) / block_x;
            let args: &mut [*mut c_void] = &mut [
                &mut elite_fast_ptr as *mut _ as *mut c_void,
                &mut elite_slow_ptr as *mut _ as *mut c_void,
                &mut elite_len_i as *mut _ as *mut c_void,
                &mut fast_ptr as *mut _ as *mut c_void,
                &mut fast_len_i as *mut _ as *mut c_void,
                &mut slow_ptr as *mut _ as *mut c_void,
                &mut slow_len_i as *mut _ as *mut c_void,
                &mut elite_count_i as *mut _ as *mut c_void,
                &mut diff_count_i as *mut _ as *mut c_void,
                &mut rand_count_i as *mut _ as *mut c_void,
                &mut fast_step_i as *mut _ as *mut c_void,
                &mut slow_step_i as *mut _ as *mut c_void,
                &mut seed_u64 as *mut _ as *mut c_void,
                &mut out_len_i as *mut _ as *mut c_void,
                &mut out_fast_ptr as *mut _ as *mut c_void,
                &mut out_slow_ptr as *mut _ as *mut c_void,
            ];
            stream
                .launch(&kernel, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        stream.synchronize().map_err(|e| e.to_string())?;

        let mut out_fast_h: Vec<i32> = alloc_uninit_copy(sample_total);
        let mut out_slow_h: Vec<i32> = alloc_uninit_copy(sample_total);
        d_out_fast
            .copy_to(&mut out_fast_h)
            .map_err(|e| e.to_string())?;
        d_out_slow
            .copy_to(&mut out_slow_h)
            .map_err(|e| e.to_string())?;

        let mut out: Vec<(u16, u16)> = Vec::with_capacity(target_total);
        let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
        for i in 0..sample_total {
            let f = out_fast_h[i];
            let s = out_slow_h[i];
            if f <= 0 || s <= 0 || f >= s || f > (u16::MAX as i32) || s > (u16::MAX as i32) {
                continue;
            }
            let pair = (f as u16, s as u16);
            if seen.insert(pair) {
                out.push(pair);
                if out.len() >= target_total {
                    break;
                }
            }
        }
        Ok(Some(out))
    })
}

fn estimate_tile_bytes(
    pf: usize,
    ps: usize,
    series_len: usize,
    max_fast_period: usize,
    max_slow_period: usize,
    fast_ma: &str,
    slow_ma: &str,
    ma_is_close: bool,
    metrics_count: usize,
    topk_select: usize,
    include_all: bool,
    heatmap_bins: usize,
) -> usize {
    let bytes_f32 = |n: usize| n.saturating_mul(std::mem::size_of::<f32>());
    let bytes_f64 = |n: usize| n.saturating_mul(std::mem::size_of::<f64>());
    let bytes_i32 = |n: usize| n.saturating_mul(std::mem::size_of::<i32>());

    let mut bytes = 0usize;

    // Input series and shared scratch.
    bytes = bytes.saturating_add(bytes_f32(series_len)); // d_close
    if !ma_is_close {
        bytes = bytes.saturating_add(bytes_f32(series_len)); // d_ma_buf
    }
    bytes = bytes.saturating_add(bytes_f32(series_len)); // d_lr
    bytes = bytes.saturating_add(bytes_f32(series_len)); // d_ret1

    if fast_ma == "vwma" || slow_ma == "vwma" {
        bytes = bytes.saturating_add(bytes_f32(series_len)); // d_volume
        bytes = bytes.saturating_add(bytes_f64(series_len).saturating_mul(2)); // pv_prefix + vol_prefix
    }
    if fast_ma == "frama" || slow_ma == "frama" {
        bytes = bytes.saturating_add(bytes_f32(series_len).saturating_mul(2)); // d_high + d_low
    }

    // SMA prefix (shared across sides).
    if fast_ma == "sma" || slow_ma == "sma" {
        bytes = bytes.saturating_add(bytes_f64(series_len.saturating_add(1)));
    }

    // Shared scratch buffers (can be reused across fast/slow computation).
    if fast_ma == "nma" || slow_ma == "nma" {
        bytes = bytes.saturating_add(bytes_f32(series_len)); // abs_diffs
    }
    if fast_ma == "edcf" || slow_ma == "edcf" {
        bytes = bytes.saturating_add(bytes_f32(series_len)); // dist scratch
    }
    if fast_ma == "cora_wave" || slow_ma == "cora_wave" {
        let p_max = pf.max(ps).max(1);
        bytes = bytes.saturating_add(bytes_f32(p_max.saturating_mul(series_len)));
        // y tmp (for smoothing)
    }

    let ma_matrix_mult = if kernel_fast_path_enabled(fast_ma, slow_ma) {
        1
    } else {
        2
    };

    // Fast MA buffers.
    bytes = bytes.saturating_add(bytes_i32(pf)); // periods
    bytes = bytes.saturating_add(bytes_f32(
        pf.saturating_mul(series_len).saturating_mul(ma_matrix_mult),
    )); // ma(+ma_tm) or ma_tm-only
    if fast_ma == "alma" || fast_ma == "cwma" || fast_ma == "cora_wave" {
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(max_fast_period))); // weights
        bytes = bytes.saturating_add(bytes_f32(pf)); // inv norms
    } else if fast_ma == "fwma" || fast_ma == "pwma" {
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(max_fast_period)));
    // weights
    } else if fast_ma == "srwma" {
        bytes = bytes.saturating_add(bytes_f32(
            pf.saturating_mul(max_fast_period.saturating_sub(1)),
        )); // weights
        bytes = bytes.saturating_add(bytes_f32(pf)); // inv norms
    } else if fast_ma == "ema" || fast_ma == "mwdx" {
        bytes = bytes.saturating_add(bytes_f32(pf)); // alphas/factors
    } else if fast_ma == "zlema" {
        bytes = bytes.saturating_add(bytes_i32(pf)); // lags
        bytes = bytes.saturating_add(bytes_f32(pf)); // alphas
    } else if fast_ma == "maaq" {
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(2))); // fast_sc + slow_sc
    } else if fast_ma == "jma" {
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(3))); // alpha + (1-beta) + phase_ratio
    } else if fast_ma == "hma" {
        let max_sqrt = ((max_fast_period as f64).sqrt().floor() as usize).max(1);
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(max_sqrt))); // ring scratch
    } else if fast_ma == "vpwma" {
        bytes = bytes.saturating_add(bytes_i32(pf)); // win lengths
        bytes = bytes.saturating_add(bytes_f32(
            pf.saturating_mul(max_fast_period.saturating_sub(1)),
        )); // weights
        bytes = bytes.saturating_add(bytes_f32(pf)); // inv norms
    }
    if matches!(
        fast_ma,
        "jsa" | "smma" | "swma" | "trima" | "fwma" | "pwma" | "srwma"
    ) {
        bytes = bytes.saturating_add(bytes_i32(pf)); // warm indices
    } else if fast_ma == "wilders" {
        bytes = bytes.saturating_add(bytes_i32(pf)); // warm indices
        bytes = bytes.saturating_add(bytes_f32(pf)); // alphas
    } else if fast_ma == "epma" {
        bytes = bytes.saturating_add(bytes_i32(pf)); // offsets
    } else if fast_ma == "cora_wave" {
        bytes = bytes.saturating_add(bytes_i32(pf.saturating_mul(2))); // smooth_periods + warm0s
    } else if fast_ma == "ehlers_itrend" {
        bytes = bytes.saturating_add(bytes_i32(pf)); // warmups
    }
    if fast_ma == "frama" {
        bytes = bytes.saturating_add(bytes_i32(pf).saturating_mul(2)); // scs + fcs
    }

    // Slow MA buffers.
    bytes = bytes.saturating_add(bytes_i32(ps)); // periods
    bytes = bytes.saturating_add(bytes_f32(
        ps.saturating_mul(series_len).saturating_mul(ma_matrix_mult),
    )); // ma(+ma_tm) or ma_tm-only
    if slow_ma == "alma" || slow_ma == "cwma" || slow_ma == "cora_wave" {
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(max_slow_period))); // weights
        bytes = bytes.saturating_add(bytes_f32(ps)); // inv norms
    } else if slow_ma == "fwma" || slow_ma == "pwma" {
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(max_slow_period)));
    // weights
    } else if slow_ma == "srwma" {
        bytes = bytes.saturating_add(bytes_f32(
            ps.saturating_mul(max_slow_period.saturating_sub(1)),
        )); // weights
        bytes = bytes.saturating_add(bytes_f32(ps)); // inv norms
    } else if slow_ma == "ema" || slow_ma == "mwdx" {
        bytes = bytes.saturating_add(bytes_f32(ps)); // alphas/factors
    } else if slow_ma == "zlema" {
        bytes = bytes.saturating_add(bytes_i32(ps)); // lags
        bytes = bytes.saturating_add(bytes_f32(ps)); // alphas
    } else if slow_ma == "maaq" {
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(2))); // fast_sc + slow_sc
    } else if slow_ma == "jma" {
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(3))); // alpha + (1-beta) + phase_ratio
    } else if slow_ma == "hma" {
        let max_sqrt = ((max_slow_period as f64).sqrt().floor() as usize).max(1);
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(max_sqrt))); // ring scratch
    } else if slow_ma == "vpwma" {
        bytes = bytes.saturating_add(bytes_i32(ps)); // win lengths
        bytes = bytes.saturating_add(bytes_f32(
            ps.saturating_mul(max_slow_period.saturating_sub(1)),
        )); // weights
        bytes = bytes.saturating_add(bytes_f32(ps)); // inv norms
    }
    if matches!(
        slow_ma,
        "jsa" | "smma" | "swma" | "trima" | "fwma" | "pwma" | "srwma"
    ) {
        bytes = bytes.saturating_add(bytes_i32(ps)); // warm indices
    } else if slow_ma == "wilders" {
        bytes = bytes.saturating_add(bytes_i32(ps)); // warm indices
        bytes = bytes.saturating_add(bytes_f32(ps)); // alphas
    } else if slow_ma == "epma" {
        bytes = bytes.saturating_add(bytes_i32(ps)); // offsets
    } else if slow_ma == "cora_wave" {
        bytes = bytes.saturating_add(bytes_i32(ps.saturating_mul(2))); // smooth_periods + warm0s
    } else if slow_ma == "ehlers_itrend" {
        bytes = bytes.saturating_add(bytes_i32(ps)); // warmups
    }
    if slow_ma == "frama" {
        bytes = bytes.saturating_add(bytes_i32(ps).saturating_mul(2)); // scs + fcs
    }

    // Per-pair metrics tile (optionally padded to improve store/read coalescing).
    let pairs = pf.saturating_mul(ps);
    let tile_metrics_stride = effective_metrics_stride(
        metrics_count,
        kernel_metrics_pad8_enabled_for_pair(fast_ma, slow_ma),
        None,
    );
    bytes = bytes.saturating_add(bytes_f32(pairs.saturating_mul(tile_metrics_stride)));

    // Optional reductions (used only when include_all=false).
    if !include_all {
        let topk = topk_select.max(1);
        let blocks = (pairs + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let cand_len = blocks.saturating_mul(topk).max(topk);

        bytes = bytes.saturating_add(bytes_i32(cand_len).saturating_mul(2)); // cand_a + cand_b
        bytes = bytes.saturating_add(bytes_i32(topk).saturating_mul(2)); // top_fast + top_slow
        bytes = bytes.saturating_add(bytes_f32(topk.saturating_mul(metrics_count))); // top_metrics
        bytes = bytes.saturating_add(bytes_f32(
            topk.saturating_mul(metrics_count.saturating_add(2)),
        )); // packed top rows

        if heatmap_bins > 0 {
            let heat_len = heatmap_bins.saturating_mul(heatmap_bins);
            bytes = bytes.saturating_add(bytes_f32(heat_len)); // heatmap
        }
    }

    // Small fudge factor for driver allocations / fragmentation.
    bytes.saturating_add(64usize * 1024 * 1024)
}

fn choose_tiles(
    fast_total: usize,
    slow_total: usize,
    series_len: usize,
    max_fast_period: usize,
    max_slow_period: usize,
    fast_ma: &str,
    slow_ma: &str,
    ma_is_close: bool,
    metrics_count: usize,
    topk_select: usize,
    include_all: bool,
    heatmap_bins: usize,
) -> (usize, usize) {
    let fast_total = fast_total.max(1);
    let slow_total = slow_total.max(1);

    let budget = vram_budget_bytes();

    let full_bytes = estimate_tile_bytes(
        fast_total,
        slow_total,
        series_len,
        max_fast_period,
        max_slow_period,
        fast_ma,
        slow_ma,
        ma_is_close,
        metrics_count,
        topk_select,
        include_all,
        heatmap_bins,
    );
    if full_bytes <= budget {
        if std::env::var("VECTORBT_KERNEL_TILE_DEBUG").ok().as_deref() == Some("1") {
            eprintln!(
                "VECTORBT_KERNEL_TILE_DEBUG: budget={}MiB full={}MiB pf_tile={}/{} ps_tile={}/{} (full fits)",
                budget / (1024 * 1024),
                full_bytes / (1024 * 1024),
                fast_total,
                fast_total,
                slow_total,
                slow_total
            );
        }
        return (fast_total, slow_total);
    }

    let mut best_pf = 1usize;
    let mut best_ps = 1usize;
    let mut best_pairs = 1usize;

    for pf in (1..=fast_total).rev() {
        if pf.saturating_mul(slow_total) <= best_pairs {
            break;
        }

        let mut lo = 1usize;
        let mut hi = slow_total;
        let mut best_here = 1usize;
        while lo <= hi {
            let mid = lo.saturating_add(hi) / 2;
            let bytes = estimate_tile_bytes(
                pf,
                mid,
                series_len,
                max_fast_period,
                max_slow_period,
                fast_ma,
                slow_ma,
                ma_is_close,
                metrics_count,
                topk_select,
                include_all,
                heatmap_bins,
            );
            if bytes <= budget {
                best_here = mid;
                lo = mid.saturating_add(1);
            } else if mid == 0 {
                break;
            } else {
                hi = mid.saturating_sub(1);
            }
        }

        let pairs = pf.saturating_mul(best_here).max(1);
        if pairs > best_pairs {
            best_pairs = pairs;
            best_pf = pf;
            best_ps = best_here;
        }
    }

    if std::env::var("VECTORBT_KERNEL_TILE_DEBUG").ok().as_deref() == Some("1") {
        let chosen_bytes = estimate_tile_bytes(
            best_pf,
            best_ps,
            series_len,
            max_fast_period,
            max_slow_period,
            fast_ma,
            slow_ma,
            ma_is_close,
            metrics_count,
            topk_select,
            include_all,
            heatmap_bins,
        );
        eprintln!(
            "VECTORBT_KERNEL_TILE_DEBUG: budget={}MiB full={}MiB chosen={}MiB pf_tile={}/{} ps_tile={}/{}",
            budget / (1024 * 1024),
            full_bytes / (1024 * 1024),
            chosen_bytes / (1024 * 1024),
            best_pf,
            fast_total,
            best_ps,
            slow_total
        );
    }

    (
        best_pf.max(1).min(fast_total),
        best_ps.max(1).min(slow_total),
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
fn reduce_metric_rows_topk_on_device(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    rows_fast: &DeviceBuffer<i32>,
    rows_slow: &DeviceBuffer<i32>,
    rows_metrics: &DeviceBuffer<f32>,
    rows_len: usize,
    metrics_count: usize,
    objective_i: i32,
    topk_select: usize,
    constraints_on_device: bool,
    constraint_use_min_trades: i32,
    constraint_min_trades: f32,
    constraint_use_max_dd: i32,
    constraint_max_dd: f32,
    select_rows_legacy: &cust::function::Function<'_>,
    select_rows_candidates_legacy: &cust::function::Function<'_>,
    select_rows_v2: Option<&cust::function::Function<'_>>,
    select_rows_candidates_v2: Option<&cust::function::Function<'_>>,
    gather_rows: &cust::function::Function<'_>,
) -> Result<(), String> {
    if rows_len == 0 || topk_select == 0 {
        return Ok(());
    }

    let blocks1 = (rows_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
    let cand_len1 = blocks1.saturating_mul(topk_select).max(topk_select);
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_a, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_b, cand_len1, bt_stream)?;

    unsafe {
        let mut metrics_ptr = rows_metrics.as_device_ptr().as_raw();
        let mut rows_i = rows_len as i32;
        let mut m_i = metrics_count as i32;
        let mut obj_i = objective_i;
        let mut topk_i = topk_select as i32;
        let mut out_ptr = scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = blocks1 as u32;
        if constraints_on_device {
            let mut use_min_trades = constraint_use_min_trades;
            let mut min_trades = constraint_min_trades;
            let mut use_max_dd = constraint_use_max_dd;
            let mut max_dd = constraint_max_dd;
            let args: &mut [*mut c_void] = &mut [
                &mut metrics_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut obj_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut use_min_trades as *mut _ as *mut c_void,
                &mut min_trades as *mut _ as *mut c_void,
                &mut use_max_dd as *mut _ as *mut c_void,
                &mut max_dd as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            bt_stream
                .launch(
                    select_rows_v2
                        .ok_or_else(|| "missing select_topk_metric_rows_f32_v2".to_string())?,
                    (grid_x, 1, 1),
                    (block_x, 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        } else {
            let args: &mut [*mut c_void] = &mut [
                &mut metrics_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut obj_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            bt_stream
                .launch(select_rows_legacy, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
    }

    let mut blocks_prev = blocks1;
    let mut in_len = cand_len1;
    let mut in_is_a = true;
    while blocks_prev > 1 {
        let blocks_next = (in_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let out_len = blocks_next.saturating_mul(topk_select).max(topk_select);
        unsafe {
            let mut metrics_ptr = rows_metrics.as_device_ptr().as_raw();
            let mut rows_i = rows_len as i32;
            let mut m_i = metrics_count as i32;
            let mut obj_i = objective_i;
            let mut topk_i = topk_select as i32;
            let mut in_ptr = if in_is_a {
                scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw()
            } else {
                scratch.d_cand_b.as_ref().unwrap().as_device_ptr().as_raw()
            };
            let mut in_len_i = in_len as i32;
            let mut out_ptr = if in_is_a {
                scratch.d_cand_b.as_ref().unwrap().as_device_ptr().as_raw()
            } else {
                scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw()
            };

            let block_x: u32 = 256;
            let grid_x: u32 = blocks_next as u32;
            if constraints_on_device {
                let mut use_min_trades = constraint_use_min_trades;
                let mut min_trades = constraint_min_trades;
                let mut use_max_dd = constraint_use_max_dd;
                let mut max_dd = constraint_max_dd;
                let args: &mut [*mut c_void] = &mut [
                    &mut metrics_ptr as *mut _ as *mut c_void,
                    &mut rows_i as *mut _ as *mut c_void,
                    &mut m_i as *mut _ as *mut c_void,
                    &mut obj_i as *mut _ as *mut c_void,
                    &mut topk_i as *mut _ as *mut c_void,
                    &mut in_ptr as *mut _ as *mut c_void,
                    &mut in_len_i as *mut _ as *mut c_void,
                    &mut use_min_trades as *mut _ as *mut c_void,
                    &mut min_trades as *mut _ as *mut c_void,
                    &mut use_max_dd as *mut _ as *mut c_void,
                    &mut max_dd as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                bt_stream
                    .launch(
                        select_rows_candidates_v2.ok_or_else(|| {
                            "missing select_topk_metric_rows_from_candidates_f32_v2".to_string()
                        })?,
                        (grid_x, 1, 1),
                        (block_x, 1, 1),
                        0,
                        args,
                    )
                    .map_err(|e| e.to_string())?;
            } else {
                let args: &mut [*mut c_void] = &mut [
                    &mut metrics_ptr as *mut _ as *mut c_void,
                    &mut rows_i as *mut _ as *mut c_void,
                    &mut m_i as *mut _ as *mut c_void,
                    &mut obj_i as *mut _ as *mut c_void,
                    &mut topk_i as *mut _ as *mut c_void,
                    &mut in_ptr as *mut _ as *mut c_void,
                    &mut in_len_i as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                bt_stream
                    .launch(
                        select_rows_candidates_legacy,
                        (grid_x, 1, 1),
                        (block_x, 1, 1),
                        0,
                        args,
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
        in_is_a = !in_is_a;
        blocks_prev = blocks_next;
        in_len = out_len;
    }

    unsafe {
        let mut metrics_ptr = rows_metrics.as_device_ptr().as_raw();
        let mut rows_i = rows_len as i32;
        let mut m_i = metrics_count as i32;
        let mut fast_ptr = rows_fast.as_device_ptr().as_raw();
        let mut slow_ptr = rows_slow.as_device_ptr().as_raw();
        let mut top_ptr = if in_is_a {
            scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw()
        } else {
            scratch.d_cand_b.as_ref().unwrap().as_device_ptr().as_raw()
        };
        let mut topk_i = topk_select as i32;
        let mut out_f = scratch
            .d_top_fast
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();
        let mut out_s = scratch
            .d_top_slow
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();
        let mut out_m = scratch
            .d_top_metrics
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = ((topk_select as u32) + block_x - 1) / block_x;
        let args: &mut [*mut c_void] = &mut [
            &mut metrics_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut m_i as *mut _ as *mut c_void,
            &mut fast_ptr as *mut _ as *mut c_void,
            &mut slow_ptr as *mut _ as *mut c_void,
            &mut top_ptr as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut out_f as *mut _ as *mut c_void,
            &mut out_s as *mut _ as *mut c_void,
            &mut out_m as *mut _ as *mut c_void,
        ];
        bt_stream
            .launch(gather_rows, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn eval_double_ma_batch_gpu_kernel(
    candles: &Candles,
    combos: &[DoubleMaParams],
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    cfg: KernelConfig,
    strategy: &StrategyConfig,
) -> Result<Vec<Metrics>, String> {
    let fast_periods = expand_u16_range(fast_range)?;
    let slow_periods = expand_u16_range(slow_range)?;
    eval_double_ma_batch_gpu_kernel_with_periods_and_regime(
        candles,
        combos,
        fast_periods.as_slice(),
        slow_periods.as_slice(),
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        None,
        cfg,
        strategy,
        None,
    )
}

pub(crate) fn eval_double_ma_batch_gpu_kernel_with_periods_and_regime(
    candles: &Candles,
    combos: &[DoubleMaParams],
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<RegimeFilterInput<'_>>,
    cfg: KernelConfig,
    strategy: &StrategyConfig,
    eval_range: Option<(usize, usize)>,
) -> Result<Vec<Metrics>, String> {
    let ranges = [eval_range.unwrap_or((0, candles.close.len()))];
    let flattened = eval_double_ma_batch_gpu_kernel_multi_range_with_periods_and_regime(
        candles,
        combos,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        cfg,
        strategy,
        &ranges,
    )?;
    if flattened.len() != combos.len() {
        return Err(format!(
            "exact combo batch expected {} metrics for one range, got {}",
            combos.len(),
            flattened.len()
        ));
    }
    Ok(flattened)
}

pub(crate) fn eval_double_ma_batch_gpu_kernel_multi_range_with_periods_and_regime(
    candles: &Candles,
    combos: &[DoubleMaParams],
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<RegimeFilterInput<'_>>,
    cfg: KernelConfig,
    strategy: &StrategyConfig,
    ranges: &[(usize, usize)],
) -> Result<Vec<Metrics>, String> {
    eval_double_ma_batch_gpu_kernel_multi_range_with_precomputed_and_regime(
        candles,
        combos,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        cfg,
        None,
        None,
        strategy,
        ranges,
    )
}

pub(crate) fn eval_double_ma_batch_gpu_kernel_multi_range_with_precomputed_and_regime(
    candles: &Candles,
    combos: &[DoubleMaParams],
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<RegimeFilterInput<'_>>,
    cfg: KernelConfig,
    fast_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    slow_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    strategy: &StrategyConfig,
    ranges: &[(usize, usize)],
) -> Result<Vec<Metrics>, String> {
    let device_id = cfg.device_id;
    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<Vec<Metrics>, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            let rt = CudaKernelRuntime::new(device_id)?;
            map.insert(device_id, rt);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;
        let owned_device_regime =
            if regime_filter.is_none() && strategy.uses_volume_flow_confirm_filter() {
                Some(
                    crate::double_ma::build_volume_flow_confirmation_regime_device(
                        candles, device_id, strategy,
                    )?,
                )
            } else {
                None
            };
        let effective_regime_filter = regime_filter.or_else(|| {
            owned_device_regime
                .as_ref()
                .map(RegimeFilterInput::DeviceF32)
        });
        eval_double_ma_batch_gpu_kernel_impl(
            rt,
            candles,
            combos,
            fast_periods,
            slow_periods,
            fast_ma_type,
            slow_ma_type,
            ma_source,
            fast_ma_params,
            slow_ma_params,
            effective_regime_filter,
            cfg,
            fast_precomputed,
            slow_precomputed,
            strategy,
            false,
            ranges,
        )
    })
}

fn eval_double_ma_batch_gpu_kernel_impl(
    rt: &mut CudaKernelRuntime,
    candles: &Candles,
    combos: &[DoubleMaParams],
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<RegimeFilterInput<'_>>,
    cfg: KernelConfig,
    fast_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    slow_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    strategy: &StrategyConfig,
    prefer_low_pair_m7_dispatch: bool,
    ranges: &[(usize, usize)],
) -> Result<Vec<Metrics>, String> {
    if combos.is_empty() {
        return Err("no parameter combinations".to_string());
    }
    if ranges.is_empty() {
        return Err("no evaluation ranges".to_string());
    }

    let fast_ma = fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = slow_ma_type.trim().to_ascii_lowercase();
    let fast_precomputed_enabled = fast_precomputed.is_some();
    let slow_precomputed_enabled = slow_precomputed.is_some();
    let fast_on_gpu = !fast_precomputed_enabled;
    let slow_on_gpu = !slow_precomputed_enabled;
    let needs_ma_prices = ma_uses_source(fast_ma.as_str()) || ma_uses_source(slow_ma.as_str());
    let needs_volume = ma_requires_volume(fast_ma.as_str()) || ma_requires_volume(slow_ma.as_str());
    let needs_high_low = ma_requires_high_low_close(fast_ma.as_str())
        || ma_requires_high_low_close(slow_ma.as_str());

    validate_candles_for_selected_mas(
        candles,
        needs_ma_prices,
        ma_source,
        needs_volume,
        needs_high_low,
    )?;

    let close = source_type(candles, "close");
    let ma_prices = if needs_ma_prices {
        Some(source_type(candles, ma_source))
    } else {
        None
    };
    let volume = if needs_volume {
        Some(source_type(candles, "volume"))
    } else {
        None
    };
    let high = if needs_high_low {
        Some(source_type(candles, "high"))
    } else {
        None
    };
    let low = if needs_high_low {
        Some(source_type(candles, "low"))
    } else {
        None
    };
    let n = close.len();
    if n < 2 {
        return Err("not enough candles".to_string());
    }
    if ma_prices.map_or(false, |p| p.len() != n) {
        return Err("ma source length mismatch".to_string());
    }
    if volume.map_or(false, |v| v.len() != n) {
        return Err("volume length mismatch".to_string());
    }
    if high.map_or(false, |h| h.len() != n) {
        return Err("high length mismatch".to_string());
    }
    if low.map_or(false, |l| l.len() != n) {
        return Err("low length mismatch".to_string());
    }
    let host_regime_owned = if regime_filter.is_none() {
        crate::double_ma::compute_combined_regime_filter(candles, ma_source, strategy, None)?
    } else {
        None
    };
    let host_regime_filter = match regime_filter {
        Some(RegimeFilterInput::HostF64(values)) => Some(values),
        None => host_regime_owned.as_deref(),
        _ => None,
    };
    let device_regime_filter = match regime_filter {
        Some(RegimeFilterInput::DeviceF32(values)) => Some(values),
        _ => None,
    };
    let regime_f32: Option<Vec<f32>> = if let Some(regime) = host_regime_filter {
        if regime.len() != n {
            return Err("regime trend series length mismatch".to_string());
        }
        Some(regime.iter().map(|&x| x as f32).collect())
    } else {
        None
    };
    if (regime_f32.is_some() || device_regime_filter.is_some())
        && !crate::double_ma::kernel_regime_filters_enabled()
    {
        return Err(
            "GPU kernel regime filters are disabled (set VECTORBT_KERNEL_ENABLE_REGIME_FILTERS=1 to enable)"
                .to_string(),
        );
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let mut source_start_at = fv_close;
    if let Some(p) = ma_prices {
        let fv_ma =
            first_finite_idx(p).ok_or_else(|| "ma source series all non-finite".to_string())?;
        source_start_at = source_start_at.max(fv_ma);
    }
    if let Some(v) = volume {
        let fv_vol =
            first_finite_idx(v).ok_or_else(|| "volume series all non-finite".to_string())?;
        source_start_at = source_start_at.max(fv_vol);
    }
    if let Some(h) = high {
        let fv_high =
            first_finite_idx(h).ok_or_else(|| "high series all non-finite".to_string())?;
        source_start_at = source_start_at.max(fv_high);
    }
    if let Some(l) = low {
        let fv_low = first_finite_idx(l).ok_or_else(|| "low series all non-finite".to_string())?;
        source_start_at = source_start_at.max(fv_low);
    }
    let first_valid =
        first_valid_idx_required(close, ma_prices, volume, high, low, source_start_at)
            .ok_or_else(|| "no index where all required series are finite".to_string())?;
    let regime_first_valid = if let Some(regime) = host_regime_filter {
        first_finite_idx(regime).ok_or_else(|| "regime trend series all non-finite".to_string())?
    } else if let Some(regime) = device_regime_filter {
        if regime.len() != n {
            return Err("device regime trend series length mismatch".to_string());
        }
        if regime.device_id() != cfg.device_id {
            return Err(format!(
                "device regime filter device mismatch (expected {}, got {})",
                cfg.device_id,
                regime.device_id()
            ));
        }
        regime.first_valid()
    } else {
        first_valid
    };
    if !supports_vram_kernel_ma(fast_ma.as_str()) || !supports_vram_kernel_ma(slow_ma.as_str()) {
        return Err(format!(
             "GPU kernel backend does not support the selected MA(s) for VRAM-resident mode (fast='{fast_ma_type}', slow='{slow_ma_type}')"
        ));
    }

    let f_total = fast_periods.len();
    let s_total = slow_periods.len();
    let max_pf = *fast_periods.iter().max().unwrap_or(&1) as usize;
    let max_ps = *slow_periods.iter().max().unwrap_or(&1) as usize;

    if fast_ma == "wma" && fast_periods.iter().any(|&p| p <= 1) {
        return Err("WMA periods must be > 1 for the GPU kernel backend".to_string());
    }
    if slow_ma == "wma" && slow_periods.iter().any(|&p| p <= 1) {
        return Err("WMA periods must be > 1 for the GPU kernel backend".to_string());
    }
    if fast_ma == "vpwma" && fast_periods.iter().any(|&p| p <= 1) {
        return Err("VPWMA periods must be >= 2 for the GPU kernel backend".to_string());
    }
    if slow_ma == "vpwma" && slow_periods.iter().any(|&p| p <= 1) {
        return Err("VPWMA periods must be >= 2 for the GPU kernel backend".to_string());
    }
    if fast_ma == "frama"
        && fast_periods.iter().any(|&p| {
            let win = p as usize;
            let even = if win & 1 == 1 { win + 1 } else { win };
            even > 1024
        })
    {
        return Err("FRAMA evenized window exceeds CUDA limit (1024)".to_string());
    }
    if slow_ma == "frama"
        && slow_periods.iter().any(|&p| {
            let win = p as usize;
            let even = if win & 1 == 1 { win + 1 } else { win };
            even > 1024
        })
    {
        return Err("FRAMA evenized window exceeds CUDA limit (1024)".to_string());
    }

    let max_period = max_pf.max(max_ps);
    let max_start_t = first_valid
        .saturating_add(max_period.saturating_sub(1))
        .max(regime_first_valid);
    if n.saturating_sub(max_start_t) < 1 {
        return Err(format!(
            "not enough valid data after first_valid={} regime_first_valid={} for max period {} (len={})",
            first_valid,
            regime_first_valid,
            max_period,
            n
        ));
    }
    for &(eval_start_t, eval_end_t_raw) in ranges {
        let eval_end_t = eval_end_t_raw.min(n);
        let eval_effective_start = max_start_t.max(eval_start_t);
        if eval_end_t <= eval_effective_start.saturating_add(1) {
            return Err(format!(
                "evaluation window [{eval_start_t}, {eval_end_t}) has insufficient bars after warmup (effective_start={eval_effective_start})"
            ));
        }
    }

    let mut fast_idx: HashMap<u16, usize> = HashMap::with_capacity(f_total);
    for (i, &p) in fast_periods.iter().enumerate() {
        fast_idx.insert(p, i);
    }
    let mut slow_idx: HashMap<u16, usize> = HashMap::with_capacity(s_total);
    for (i, &p) in slow_periods.iter().enumerate() {
        slow_idx.insert(p, i);
    }

    let mut combo_period_indices: Vec<(usize, usize)> = Vec::with_capacity(combos.len());
    for p in combos.iter() {
        let fi = *fast_idx
            .get(&p.fast_len)
            .ok_or_else(|| format!("fast_len {} not in requested fast range", p.fast_len))?;
        let si = *slow_idx
            .get(&p.slow_len)
            .ok_or_else(|| format!("slow_len {} not in requested slow range", p.slow_len))?;
        combo_period_indices.push((fi, si));
    }

    let combo_count = combos.len();
    let mut out_metrics = vec![Metrics::default(); combo_count.saturating_mul(ranges.len())];

    let bt_stream = &rt.stream;
    let bt_module = &rt.module;
    let device_launch_caps = rt.launch_caps;
    let compute_returns = bt_module
        .get_function("compute_log_and_simple_returns_f32")
        .map_err(|e| e.to_string())?;
    let compute_returns_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::ComputeReturns, &compute_returns);
    let use_fast_path = !fast_precomputed_enabled
        && !slow_precomputed_enabled
        && kernel_fast_path_enabled(fast_ma.as_str(), slow_ma.as_str());
    let transpose = bt_module
        .get_function("transpose_row_to_tm")
        .map_err(|e| e.to_string())?;
    let backtest = bt_module
        .get_function("double_cross_backtest_tm_flex_f32")
        .map_err(|e| e.to_string())?;
    let backtest_windowed = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed")
        .map_err(|e| e.to_string())?;
    let backtest_m7 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m7")
        .ok();
    let backtest_lockstep = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time")
        .ok();
    let backtest_lockstep_m7 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7")
        .ok();
    let backtest_lockstep_m7_regime_off = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off")
        .ok();
    let backtest_lockstep_m7_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk")
        .ok();
    let backtest_lockstep_m7_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0")
        .ok();
    let backtest_lockstep_m7_regime_on = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on")
        .ok();
    let backtest_lockstep_m7_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on_norisk")
        .ok();
    let backtest_lockstep_m7_regime_hybrid = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid")
        .ok();
    let backtest_lockstep_m7_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk")
        .ok();
    let backtest_lockstep_m7_regime_hybrid_norisk_eps0 = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0",
        )
        .ok();
    let backtest_fast_warp = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_fast_warp_dedup")
        .ok();
    let fill_f32 = bt_module
        .get_function("fill_f32")
        .map_err(|e| e.to_string())?;
    let transpose_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::TransposeRowToTm, &transpose);
    let backtest_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktest, &backtest);
    let backtest_m7_launch_caps = backtest_m7
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM7, f));
    let backtest_lockstep_launch_caps = backtest_lockstep
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstep, f));
    let backtest_lockstep_m7_launch_caps = backtest_lockstep_m7
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstepM7, f));
    let backtest_fast_launch_caps = backtest_fast_warp
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestFastWarpDedup, f));
    let fill_launch_caps = rt.launch_caps_for_kernel(CachedKernelKey::FillF32, &fill_f32);

    let ma_source_key = normalize_market_source_key(ma_source);
    let ma_is_close = ma_source_key == "close";
    let alloc_ma_buf = needs_ma_prices && !ma_is_close;
    let (resident_market, ma, scratch) = (&mut rt.resident_market, &mut rt.ma, &mut rt.scratch);
    ensure_resident_market_series(
        bt_stream,
        resident_market,
        candles,
        close,
        ma_source,
        ma_prices,
        needs_ma_prices,
        needs_high_low,
        high,
        low,
        needs_volume,
        volume,
    )?;
    let resident = resident_market
        .as_ref()
        .ok_or_else(|| "resident market cache missing after upload".to_string())?;
    let d_close = &resident.close;
    let d_high = if needs_high_low {
        resident.high.as_ref()
    } else {
        None
    };
    let d_low = if needs_high_low {
        resident.low.as_ref()
    } else {
        None
    };
    let d_volume = if needs_volume {
        resident.volume.as_ref()
    } else {
        None
    };
    let d_regime = match regime_f32.as_deref() {
        Some(src) => Some(upload_ephemeral_series_f32(bt_stream, src)?),
        None => None,
    };
    let d_regime_ptr = d_regime
        .as_ref()
        .map(|d| d.as_device_ptr().as_raw())
        .or_else(|| device_regime_filter.map(DeviceRegimeFilter::as_device_ptr_raw));
    let d_ma_prices: &DeviceBuffer<f32> = if alloc_ma_buf {
        resident
            .source_overrides
            .get(&ma_source_key)
            .ok_or_else(|| format!("resident ma_source buffer missing for {ma_source_key}"))?
    } else {
        d_close
    };

    let vram_inputs = VramMaInputs {
        prices: d_ma_prices,
        close: d_close,
        high: d_high,
        low: d_low,
        volume: d_volume,
    };

    let mut fast_params_from_cfg: HashMap<String, f64> = HashMap::new();
    let mut slow_params_from_cfg: HashMap<String, f64> = HashMap::new();
    let fast_params: Option<&HashMap<String, f64>> =
        if fast_ma == "alma" && fast_ma_params.is_none() {
            fast_params_from_cfg.insert("offset".to_string(), cfg.fast_alma_offset);
            fast_params_from_cfg.insert("sigma".to_string(), cfg.fast_alma_sigma);
            Some(&fast_params_from_cfg)
        } else {
            fast_ma_params
        };
    let slow_params: Option<&HashMap<String, f64>> =
        if slow_ma == "alma" && slow_ma_params.is_none() {
            slow_params_from_cfg.insert("offset".to_string(), cfg.slow_alma_offset);
            slow_params_from_cfg.insert("sigma".to_string(), cfg.slow_alma_sigma);
            Some(&slow_params_from_cfg)
        } else {
            slow_ma_params
        };
    if (fast_on_gpu && fast_ma == "sma") || (slow_on_gpu && slow_ma == "sma") {
        ma.ensure_sma_prefix_f64(d_ma_prices, n, first_valid)?;
    }
    if (fast_on_gpu && fast_ma == "vwma") || (slow_on_gpu && slow_ma == "vwma") {
        ma.ensure_vwma_prefix_pv_vol_f64(
            d_ma_prices,
            d_volume
                .as_ref()
                .ok_or_else(|| "vwma requires volume".to_string())?,
            n,
            first_valid,
        )?;
    }

    let lockstep_m7_regime_mode_specialize_enabled =
        kernel_lockstep_m7_regime_mode_specialize_enabled_for_pair(
            fast_ma.as_str(),
            slow_ma.as_str(),
        );
    let lockstep_m7_norisk_specialize_enabled =
        kernel_lockstep_m7_norisk_specialize_enabled_for_pair(fast_ma.as_str(), slow_ma.as_str());
    let lockstep_m7_eps0_specialize_enabled =
        kernel_lockstep_m7_eps0_specialize_enabled_for_pair(fast_ma.as_str(), slow_ma.as_str());
    let fast_warp_dedup_enabled = kernel_fast_warp_dedup_enabled();
    let fast_warp_dedup_min_pairs = kernel_fast_warp_dedup_min_pairs();

    KernelScratch::ensure_len_f32(&mut scratch.d_lr, n, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_ret1, n, bt_stream)?;
    let d_lr_ptr = scratch.d_lr.as_ref().unwrap().as_device_ptr().as_raw();
    let d_ret1_ptr = scratch.d_ret1.as_ref().unwrap().as_device_ptr().as_raw();
    unsafe {
        let mut prices_ptr = d_close.as_device_ptr().as_raw();
        let mut t_i = n as i32;
        let mut lr_out_ptr = d_lr_ptr;
        let mut ret1_out_ptr = d_ret1_ptr;
        let launch = choose_launch_shape_1d(n, device_launch_caps, compute_returns_launch_caps);
        let block_x: u32 = launch.block_x;
        let grid_x: u32 = launch.grid_x;
        let args: &mut [*mut c_void] = &mut [
            &mut prices_ptr as *mut _ as *mut c_void,
            &mut t_i as *mut _ as *mut c_void,
            &mut lr_out_ptr as *mut _ as *mut c_void,
            &mut ret1_out_ptr as *mut _ as *mut c_void,
        ];
        bt_stream
            .launch(&compute_returns, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
            .map_err(|e| e.to_string())?;
    }

    let (pf_tile, ps_tile) = choose_tiles(
        f_total,
        s_total,
        n,
        max_pf,
        max_ps,
        fast_ma.as_str(),
        slow_ma.as_str(),
        !alloc_ma_buf,
        METRICS_COUNT,
        0,
        true,
        0,
    );
    let is_multi_tile_workload = pf_tile < f_total || ps_tile < s_total;
    let _ = is_multi_tile_workload;
    let metrics_pad8_enabled =
        kernel_metrics_pad8_enabled_for_pair(fast_ma.as_str(), slow_ma.as_str());
    let tile_metrics_stride = effective_metrics_stride(METRICS_COUNT, metrics_pad8_enabled, None);

    let pad_tm = kernel_tm_padding_enabled();
    let pf_tile_tm = if pad_tm {
        align_tm_stride(pf_tile.max(1))
    } else {
        pf_tile.max(1)
    };
    let ps_tile_tm = if pad_tm {
        align_tm_stride(ps_tile.max(1))
    } else {
        ps_tile.max(1)
    };

    let fast_ma_cap = pf_tile_tm
        .checked_mul(n)
        .ok_or_else(|| "pf_tile*n overflow".to_string())?;
    let slow_ma_cap = ps_tile_tm
        .checked_mul(n)
        .ok_or_else(|| "ps_tile*n overflow".to_string())?;
    let tile_out_cap = pf_tile
        .checked_mul(ps_tile)
        .and_then(|v| v.checked_mul(tile_metrics_stride))
        .ok_or_else(|| "pf_tile*ps_tile*metrics overflow".to_string())?;

    KernelScratch::ensure_len_i32(&mut scratch.d_fast_p, pf_tile_tm, bt_stream)?;
    if use_fast_path || fast_precomputed.is_some() {
        scratch.d_fast_ma = None;
    } else {
        KernelScratch::ensure_len_f32(&mut scratch.d_fast_ma, fast_ma_cap, bt_stream)?;
    }
    KernelScratch::ensure_len_f32(&mut scratch.d_fast_ma_tm, fast_ma_cap, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_slow_p, ps_tile_tm, bt_stream)?;
    if use_fast_path || slow_precomputed.is_some() {
        scratch.d_slow_ma = None;
    } else {
        KernelScratch::ensure_len_f32(&mut scratch.d_slow_ma, slow_ma_cap, bt_stream)?;
    }
    KernelScratch::ensure_len_f32(&mut scratch.d_slow_ma_tm, slow_ma_cap, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_tile, tile_out_cap, bt_stream)?;

    let mut host_p_fast: Vec<i32> = Vec::with_capacity(pf_tile_tm.max(1));
    let mut host_p_slow: Vec<i32> = Vec::with_capacity(ps_tile_tm.max(1));
    let mut fast_precomputed_host_tile: Vec<f32> = Vec::new();
    let mut slow_precomputed_host_tile: Vec<f32> = Vec::new();
    let mut tile_host_buf: Vec<f32> = alloc_uninit_copy(tile_out_cap);
    let mut tile_combo_indices: Vec<usize> = Vec::new();
    let mut tile_pair_index_map: Vec<i32> = Vec::new();

    let strategy_kernel = kernel_strategy_params(strategy);
    let flags = strategy_kernel.flags;
    let commission = strategy_kernel.commission;
    let eps_rel = strategy_kernel.eps_rel;
    let stop_log = strategy_kernel.stop_log;
    let take_log = strategy_kernel.take_log;
    let regime_use = if d_regime_ptr.is_some() { 1i32 } else { 0i32 };
    let regime_use_hybrid = if d_regime_ptr.is_some() && strategy.uses_regime_hybrid_filter() {
        1i32
    } else {
        0i32
    };
    let regime_entry_rel = if regime_use_hybrid != 0 {
        strategy.regime_hybrid_reversion_entry_rel() as f32
    } else {
        0.0
    };
    let regime_exit_rel = if regime_use_hybrid != 0 {
        strategy.regime_hybrid_reversion_exit_rel() as f32
    } else {
        0.0
    };

    let mut f_start = 0usize;
    while f_start < f_total {
        let pf = pf_tile.min(f_total - f_start);
        let fast_tm_rows = if pad_tm {
            align_tm_stride(pf.max(1))
        } else {
            pf.max(1)
        };

        host_p_fast.clear();
        host_p_fast.extend(
            fast_periods[f_start..f_start + pf]
                .iter()
                .map(|&x| x as i32),
        );
        let f_pad = host_p_fast.last().copied().unwrap_or(2);
        host_p_fast.resize(pf_tile_tm, f_pad);
        // Host period vectors are pageable and mutated per tile; keep this copy
        // synchronous to avoid host-data races with async H2D semantics.
        scratch
            .d_fast_p
            .as_mut()
            .unwrap()
            .copy_from(&host_p_fast)
            .map_err(|e| e.to_string())?;
        if let Some(precomputed_fast) = fast_precomputed {
            pack_precomputed_rows_tm_with_padding(
                precomputed_fast,
                f_start,
                pf,
                fast_tm_rows,
                &mut fast_precomputed_host_tile,
            )?;
            scratch
                .d_fast_ma_tm
                .as_mut()
                .ok_or_else(|| {
                    "internal error: missing d_fast_ma_tm for precomputed fast matrix".to_string()
                })?
                .copy_from(&fast_precomputed_host_tile)
                .map_err(|e| e.to_string())?;
        } else if use_fast_path {
            ma.compute_period_ma_tm_into(
                fast_ma.as_str(),
                fast_params,
                &vram_inputs,
                n,
                first_valid,
                &host_p_fast[..fast_tm_rows],
                scratch.d_fast_p.as_ref().unwrap(),
                scratch.d_fast_ma_tm.as_mut().unwrap(),
            )?;
        } else {
            if fast_ma == "nma" {
                fill_device_buffer_f32(
                    bt_stream,
                    &fill_f32,
                    device_launch_caps,
                    fill_launch_caps,
                    scratch.d_fast_ma.as_ref().unwrap(),
                    pf.saturating_mul(n),
                    f32::NAN,
                )?;
                fill_device_buffer_f32(
                    bt_stream,
                    &fill_f32,
                    device_launch_caps,
                    fill_launch_caps,
                    scratch.d_fast_ma_tm.as_ref().unwrap(),
                    fast_tm_rows.saturating_mul(n),
                    f32::NAN,
                )?;
            }
            ma.compute_period_ma_into(
                fast_ma.as_str(),
                fast_params,
                &vram_inputs,
                n,
                first_valid,
                &host_p_fast,
                scratch.d_fast_p.as_ref().unwrap(),
                scratch.d_fast_ma.as_mut().unwrap(),
            )?;

            unsafe {
                let mut in_ptr = scratch.d_fast_ma.as_ref().unwrap().as_device_ptr().as_raw();
                let mut rows = pf as i32;
                let mut cols = n as i32;
                let mut out_rows = fast_tm_rows as i32;
                let mut out_ptr = scratch
                    .d_fast_ma_tm
                    .as_ref()
                    .unwrap()
                    .as_device_ptr()
                    .as_raw();
                let total = pf.saturating_mul(n);
                let launch =
                    choose_launch_shape_1d(total, device_launch_caps, transpose_launch_caps);
                let block_x: u32 = launch.block_x;
                let grid_x: u32 = launch.grid_x;
                let args: &mut [*mut c_void] = &mut [
                    &mut in_ptr as *mut _ as *mut c_void,
                    &mut rows as *mut _ as *mut c_void,
                    &mut cols as *mut _ as *mut c_void,
                    &mut out_rows as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                bt_stream
                    .launch(&transpose, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                    .map_err(|e| e.to_string())?;
            }
        }

        let mut s_start = 0usize;
        while s_start < s_total {
            let ps = ps_tile.min(s_total - s_start);
            let slow_tm_rows = if pad_tm {
                align_tm_stride(ps.max(1))
            } else {
                ps.max(1)
            };

            host_p_slow.clear();
            host_p_slow.extend(
                slow_periods[s_start..s_start + ps]
                    .iter()
                    .map(|&x| x as i32),
            );
            let s_pad = host_p_slow.last().copied().unwrap_or(2);
            host_p_slow.resize(ps_tile_tm, s_pad);
            // Host period vectors are pageable and mutated per tile; keep this copy
            // synchronous to avoid host-data races with async H2D semantics.
            scratch
                .d_slow_p
                .as_mut()
                .unwrap()
                .copy_from(&host_p_slow)
                .map_err(|e| e.to_string())?;
            if let Some(precomputed_slow) = slow_precomputed {
                pack_precomputed_rows_tm_with_padding(
                    precomputed_slow,
                    s_start,
                    ps,
                    slow_tm_rows,
                    &mut slow_precomputed_host_tile,
                )?;
                scratch
                    .d_slow_ma_tm
                    .as_mut()
                    .ok_or_else(|| {
                        "internal error: missing d_slow_ma_tm for precomputed slow matrix"
                            .to_string()
                    })?
                    .copy_from(&slow_precomputed_host_tile)
                    .map_err(|e| e.to_string())?;
            } else if use_fast_path {
                ma.compute_period_ma_tm_into(
                    slow_ma.as_str(),
                    slow_params,
                    &vram_inputs,
                    n,
                    first_valid,
                    &host_p_slow[..slow_tm_rows],
                    scratch.d_slow_p.as_ref().unwrap(),
                    scratch.d_slow_ma_tm.as_mut().unwrap(),
                )?;
            } else {
                if slow_ma == "nma" {
                    fill_device_buffer_f32(
                        bt_stream,
                        &fill_f32,
                        device_launch_caps,
                        fill_launch_caps,
                        scratch.d_slow_ma.as_ref().unwrap(),
                        ps.saturating_mul(n),
                        f32::NAN,
                    )?;
                    fill_device_buffer_f32(
                        bt_stream,
                        &fill_f32,
                        device_launch_caps,
                        fill_launch_caps,
                        scratch.d_slow_ma_tm.as_ref().unwrap(),
                        slow_tm_rows.saturating_mul(n),
                        f32::NAN,
                    )?;
                }
                ma.compute_period_ma_into(
                    slow_ma.as_str(),
                    slow_params,
                    &vram_inputs,
                    n,
                    first_valid,
                    &host_p_slow,
                    scratch.d_slow_p.as_ref().unwrap(),
                    scratch.d_slow_ma.as_mut().unwrap(),
                )?;

                unsafe {
                    let mut in_ptr = scratch.d_slow_ma.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut rows = ps as i32;
                    let mut cols = n as i32;
                    let mut out_rows = slow_tm_rows as i32;
                    let mut out_ptr = scratch
                        .d_slow_ma_tm
                        .as_ref()
                        .unwrap()
                        .as_device_ptr()
                        .as_raw();
                    let total = ps.saturating_mul(n);
                    let launch =
                        choose_launch_shape_1d(total, device_launch_caps, transpose_launch_caps);
                    let block_x: u32 = launch.block_x;
                    let grid_x: u32 = launch.grid_x;
                    let args: &mut [*mut c_void] = &mut [
                        &mut in_ptr as *mut _ as *mut c_void,
                        &mut rows as *mut _ as *mut c_void,
                        &mut cols as *mut _ as *mut c_void,
                        &mut out_rows as *mut _ as *mut c_void,
                        &mut out_ptr as *mut _ as *mut c_void,
                    ];
                    bt_stream
                        .launch(&transpose, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                        .map_err(|e| e.to_string())?;
                }
            }

            let pairs = pf
                .checked_mul(ps)
                .ok_or_else(|| "pf*ps overflow".to_string())?;
            let out_len = pairs
                .checked_mul(tile_metrics_stride)
                .ok_or_else(|| "pairs*metrics overflow".to_string())?;

            if out_len > tile_out_cap {
                return Err(format!(
                    "tile output len {out_len} exceeds allocated capacity {tile_out_cap}"
                ));
            }

            tile_combo_indices.clear();
            tile_pair_index_map.clear();
            for (combo_idx, &(global_f, global_s)) in combo_period_indices.iter().enumerate() {
                if global_f >= f_start
                    && global_f < f_start + pf
                    && global_s >= s_start
                    && global_s < s_start + ps
                {
                    let local_f = global_f - f_start;
                    let local_s = global_s - s_start;
                    tile_combo_indices.push(combo_idx);
                    tile_pair_index_map.push(
                        i32::try_from(local_f.saturating_mul(ps).saturating_add(local_s))
                            .map_err(|_| "tile pair index exceeds i32::MAX".to_string())?,
                    );
                }
            }
            let launch_pairs = tile_combo_indices.len();
            let out_len = launch_pairs
                .checked_mul(tile_metrics_stride)
                .ok_or_else(|| "launch_pairs*metrics overflow".to_string())?;
            let mut pair_index_map_ptr: u64 = 0;
            for (range_idx, &(eval_start_t, eval_end_t_raw)) in ranges.iter().enumerate() {
                let eval_end_t = eval_end_t_raw.min(n);

                if launch_pairs > 0 {
                    KernelScratch::ensure_len_i32(
                        &mut scratch.d_valid_pair_index,
                        launch_pairs,
                        bt_stream,
                    )?;
                    scratch
                        .d_valid_pair_index
                        .as_mut()
                        .ok_or_else(|| "missing exact pair index buffer".to_string())?
                        .copy_from(&tile_pair_index_map)
                        .map_err(|e| e.to_string())?;
                    pair_index_map_ptr = scratch
                        .d_valid_pair_index
                        .as_ref()
                        .ok_or_else(|| "missing exact pair index buffer".to_string())?
                        .as_device_ptr()
                        .as_raw();
                }

                if launch_pairs > 0 {
                    unsafe {
                        let mut f_ma_tm = scratch
                            .d_fast_ma_tm
                            .as_ref()
                            .unwrap()
                            .as_device_ptr()
                            .as_raw();
                        let mut s_ma_tm = scratch
                            .d_slow_ma_tm
                            .as_ref()
                            .unwrap()
                            .as_device_ptr()
                            .as_raw();
                        let mut lr = d_lr_ptr;
                        let mut ret1 = d_ret1_ptr;
                        let mut t_i = n as i32;
                        let mut pf_tile_i = pf as i32;
                        let mut ps_tile_i = ps as i32;
                        let mut fast_tm_stride_i = fast_tm_rows as i32;
                        let mut slow_tm_stride_i = slow_tm_rows as i32;
                        let mut pf_total_i = pf as i32;
                        let mut ps_total_i = ps as i32;
                        let mut f_off = 0i32;
                        let mut s_off = 0i32;
                        let mut f_per = scratch.d_fast_p.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut s_per = scratch.d_slow_p.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut regime_ptr = d_regime
                            .as_ref()
                            .map(|d| d.as_device_ptr().as_raw())
                            .unwrap_or(0);
                        let mut use_regime_i = regime_use;
                        let mut use_regime_hybrid_i = regime_use_hybrid;
                        let mut regime_entry = regime_entry_rel;
                        let mut regime_exit = regime_exit_rel;
                        let mut fv = first_valid as i32;
                        let mut regime_fv = regime_first_valid as i32;
                        let mut commission = commission;
                        let mut eps_rel = eps_rel;
                        let mut stop_log = stop_log;
                        let mut take_log = take_log;
                        let mut flags_u = flags;
                        let mut m_i = METRICS_COUNT as i32;
                        let mut metrics_stride_i = tile_metrics_stride as i32;
                        let mut pair_map_ptr = pair_index_map_ptr;
                        let mut pair_count_i = launch_pairs as i32;
                        let mut compact_output_i = 1i32;
                        let mut out_ptr = scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut eval_start_i = eval_start_t as i32;
                        let mut eval_end_i = eval_end_t as i32;
                        let use_windowed_eval = eval_start_t != 0 || eval_end_t != n;

                        let use_lockstep_time = !use_windowed_eval
                            && kernel_should_use_lockstep_time(
                                launch_pairs,
                                n,
                                max_start_t,
                                backtest_lockstep.is_some(),
                            );
                        // Full-metric finalist reruns are tiny in pair count but still walk the
                        // entire time axis. The M7 kernel is a much better fit here than the
                        // generic full-metric path, so allow a recompute-only low-pair fast path.
                        let force_low_pair_m7 = should_force_low_pair_m7_recompute(
                            prefer_low_pair_m7_dispatch,
                            use_windowed_eval,
                            is_multi_tile_workload,
                            launch_pairs,
                            backtest_m7.is_some(),
                        );
                        let use_m7 = force_low_pair_m7
                            || (!use_windowed_eval
                                && kernel_should_use_m7_specialize(
                                    launch_pairs,
                                    n,
                                    max_start_t,
                                    is_multi_tile_workload,
                                    METRICS_COUNT,
                                    backtest_m7.is_some(),
                                ));
                        let use_lockstep_m7 = use_lockstep_time
                            && !is_multi_tile_workload
                            && kernel_should_use_m7_specialize(
                                launch_pairs,
                                n,
                                max_start_t,
                                false,
                                METRICS_COUNT,
                                backtest_lockstep_m7.is_some(),
                            );
                        let use_fast_warp_dedup = !use_windowed_eval
                            && !use_lockstep_time
                            && fast_warp_dedup_enabled
                            && launch_pairs >= fast_warp_dedup_min_pairs
                            && backtest_fast_warp.is_some();
                        let launch_caps = if use_windowed_eval {
                            backtest_launch_caps
                        } else if use_lockstep_m7 {
                            backtest_lockstep_m7_launch_caps
                                .or(backtest_lockstep_launch_caps)
                                .unwrap_or(backtest_launch_caps)
                        } else if use_lockstep_time {
                            backtest_lockstep_launch_caps.unwrap_or(backtest_launch_caps)
                        } else if use_fast_warp_dedup {
                            backtest_fast_launch_caps.unwrap_or(backtest_launch_caps)
                        } else if use_m7 {
                            backtest_m7_launch_caps.unwrap_or(backtest_launch_caps)
                        } else {
                            backtest_launch_caps
                        };
                        let launch = choose_backtest_launch_shape_1d(
                            launch_pairs,
                            device_launch_caps,
                            launch_caps,
                        );
                        let no_risk_mode = (flags_u & (STRAT_STOP_LOSS | STRAT_TAKE_PROFIT)) == 0;
                        let eps_zero_mode = eps_rel <= 0.0f32;
                        let backtest_fn = if use_windowed_eval {
                            &backtest_windowed
                        } else if use_lockstep_m7 {
                            if lockstep_m7_norisk_specialize_enabled && no_risk_mode {
                                if lockstep_m7_eps0_specialize_enabled && eps_zero_mode {
                                    if use_regime_hybrid_i != 0 {
                                        backtest_lockstep_m7_regime_hybrid_norisk_eps0
                                            .as_ref()
                                            .or(backtest_lockstep_m7_regime_hybrid_norisk.as_ref())
                                            .or(backtest_lockstep_m7_regime_hybrid.as_ref())
                                            .or(backtest_lockstep_m7.as_ref())
                                            .expect("checked lockstep m7 kernel availability")
                                    } else if use_regime_i != 0 {
                                        backtest_lockstep_m7_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_lockstep_m7_regime_on.as_ref())
                                            .or(backtest_lockstep_m7.as_ref())
                                            .expect("checked lockstep m7 kernel availability")
                                    } else {
                                        backtest_lockstep_m7_regime_off_norisk_eps0
                                            .as_ref()
                                            .or(backtest_lockstep_m7_regime_off_norisk.as_ref())
                                            .or(backtest_lockstep_m7_regime_off.as_ref())
                                            .or(backtest_lockstep_m7.as_ref())
                                            .expect("checked lockstep m7 kernel availability")
                                    }
                                } else if use_regime_hybrid_i != 0 {
                                    backtest_lockstep_m7_regime_hybrid_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m7_regime_hybrid.as_ref())
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                } else if use_regime_i != 0 {
                                    backtest_lockstep_m7_regime_on_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m7_regime_on.as_ref())
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                } else {
                                    backtest_lockstep_m7_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m7_regime_off.as_ref())
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                }
                            } else if lockstep_m7_regime_mode_specialize_enabled {
                                if use_regime_hybrid_i != 0 {
                                    backtest_lockstep_m7_regime_hybrid
                                        .as_ref()
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                } else if use_regime_i != 0 {
                                    backtest_lockstep_m7_regime_on
                                        .as_ref()
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                } else {
                                    backtest_lockstep_m7_regime_off
                                        .as_ref()
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                }
                            } else {
                                backtest_lockstep_m7
                                    .as_ref()
                                    .expect("checked lockstep m7 kernel availability")
                            }
                        } else if use_lockstep_time {
                            backtest_lockstep
                                .as_ref()
                                .expect("checked lockstep kernel availability")
                        } else if use_fast_warp_dedup {
                            backtest_fast_warp
                                .as_ref()
                                .expect("checked fast dedup kernel availability")
                        } else if use_m7 {
                            backtest_m7
                                .as_ref()
                                .expect("checked m7 kernel availability")
                        } else {
                            &backtest
                        };
                        let block_x: u32 = launch.block_x;
                        let grid_x: u32 = launch.grid_x;
                        if use_windowed_eval {
                            let args: &mut [*mut c_void] = &mut [
                                &mut f_ma_tm as *mut _ as *mut c_void,
                                &mut s_ma_tm as *mut _ as *mut c_void,
                                &mut lr as *mut _ as *mut c_void,
                                &mut ret1 as *mut _ as *mut c_void,
                                &mut t_i as *mut _ as *mut c_void,
                                &mut pf_tile_i as *mut _ as *mut c_void,
                                &mut ps_tile_i as *mut _ as *mut c_void,
                                &mut fast_tm_stride_i as *mut _ as *mut c_void,
                                &mut slow_tm_stride_i as *mut _ as *mut c_void,
                                &mut pf_total_i as *mut _ as *mut c_void,
                                &mut ps_total_i as *mut _ as *mut c_void,
                                &mut f_off as *mut _ as *mut c_void,
                                &mut s_off as *mut _ as *mut c_void,
                                &mut f_per as *mut _ as *mut c_void,
                                &mut s_per as *mut _ as *mut c_void,
                                &mut regime_ptr as *mut _ as *mut c_void,
                                &mut use_regime_i as *mut _ as *mut c_void,
                                &mut use_regime_hybrid_i as *mut _ as *mut c_void,
                                &mut regime_entry as *mut _ as *mut c_void,
                                &mut regime_exit as *mut _ as *mut c_void,
                                &mut fv as *mut _ as *mut c_void,
                                &mut regime_fv as *mut _ as *mut c_void,
                                &mut eval_start_i as *mut _ as *mut c_void,
                                &mut eval_end_i as *mut _ as *mut c_void,
                                &mut commission as *mut _ as *mut c_void,
                                &mut eps_rel as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            bt_stream
                                .launch(backtest_fn, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                                .map_err(|e| e.to_string())?;
                        } else {
                            let args: &mut [*mut c_void] = &mut [
                                &mut f_ma_tm as *mut _ as *mut c_void,
                                &mut s_ma_tm as *mut _ as *mut c_void,
                                &mut lr as *mut _ as *mut c_void,
                                &mut ret1 as *mut _ as *mut c_void,
                                &mut t_i as *mut _ as *mut c_void,
                                &mut pf_tile_i as *mut _ as *mut c_void,
                                &mut ps_tile_i as *mut _ as *mut c_void,
                                &mut fast_tm_stride_i as *mut _ as *mut c_void,
                                &mut slow_tm_stride_i as *mut _ as *mut c_void,
                                &mut pf_total_i as *mut _ as *mut c_void,
                                &mut ps_total_i as *mut _ as *mut c_void,
                                &mut f_off as *mut _ as *mut c_void,
                                &mut s_off as *mut _ as *mut c_void,
                                &mut f_per as *mut _ as *mut c_void,
                                &mut s_per as *mut _ as *mut c_void,
                                &mut regime_ptr as *mut _ as *mut c_void,
                                &mut use_regime_i as *mut _ as *mut c_void,
                                &mut use_regime_hybrid_i as *mut _ as *mut c_void,
                                &mut regime_entry as *mut _ as *mut c_void,
                                &mut regime_exit as *mut _ as *mut c_void,
                                &mut fv as *mut _ as *mut c_void,
                                &mut regime_fv as *mut _ as *mut c_void,
                                &mut commission as *mut _ as *mut c_void,
                                &mut eps_rel as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            bt_stream
                                .launch(backtest_fn, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
                bt_stream.synchronize().map_err(|e| e.to_string())?;

                if launch_pairs > 0 {
                    copy_device_prefix_to_host_f32(
                        scratch
                            .d_tile
                            .as_ref()
                            .ok_or_else(|| "missing tile buffer".to_string())?,
                        &mut tile_host_buf,
                        out_len,
                        bt_stream,
                    )?;
                    bt_stream.synchronize().map_err(|e| e.to_string())?;

                    for (row_idx, &combo_idx) in tile_combo_indices.iter().enumerate() {
                        let src_base = row_idx * tile_metrics_stride;
                        out_metrics[range_idx * combo_count + combo_idx] =
                            decode_tm_flex_metrics_row(
                                &tile_host_buf,
                                src_base,
                                tile_metrics_stride,
                            );
                    }
                }
            }

            s_start += ps;
        }

        f_start += pf;
    }

    if !kernel_persist_buffers_enabled() {
        ma.clear_cached_constants();
        scratch.clear_async(bt_stream)?;
    }

    Ok(out_metrics)
}

fn recompute_optimization_result_finalists_with_full_gpu_pass(
    rt: &mut CudaKernelRuntime,
    candles: &Candles,
    result: &mut OptimizationResult,
    _fast_periods: &[u16],
    _slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<RegimeFilterInput<'_>>,
    cfg: KernelConfig,
    strategy: &StrategyConfig,
) -> Result<(), String> {
    let mut finalists: Vec<DoubleMaParams> = Vec::new();
    let mut seen: HashMap<(u16, u16), usize> = HashMap::new();
    let mut top_indices: Vec<usize> = Vec::with_capacity(result.top.len());

    let best_key = (result.best_params.fast_len, result.best_params.slow_len);
    seen.insert(best_key, 0);
    finalists.push(result.best_params.clone());

    for (params, _) in &result.top {
        let key = (params.fast_len, params.slow_len);
        let idx = if let Some(&existing) = seen.get(&key) {
            existing
        } else {
            let next = finalists.len();
            seen.insert(key, next);
            finalists.push(params.clone());
            next
        };
        top_indices.push(idx);
    }

    let finalist_fast_periods: Vec<u16> = finalists
        .iter()
        .map(|params| params.fast_len)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let finalist_slow_periods: Vec<u16> = finalists
        .iter()
        .map(|params| params.slow_len)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let recomputed = eval_double_ma_batch_gpu_kernel_impl(
        rt,
        candles,
        finalists.as_slice(),
        finalist_fast_periods.as_slice(),
        finalist_slow_periods.as_slice(),
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        cfg,
        None,
        None,
        strategy,
        true,
        &[(0, candles.close.len())],
    )?;
    if recomputed.len() != finalists.len() {
        return Err(format!(
            "full finalist recompute returned {} metrics for {} finalists",
            recomputed.len(),
            finalists.len()
        ));
    }

    result.best_metrics = recomputed[0].clone();
    for ((_, metrics), &idx) in result.top.iter_mut().zip(top_indices.iter()) {
        *metrics = recomputed[idx].clone();
    }

    Ok(())
}

pub fn optimize_double_ma_gpu_kernel(
    candles: &Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    cfg: KernelConfig,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    optimize_double_ma_gpu_kernel_in_range(
        candles,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        strategy,
        objective,
        top_k,
        include_all,
        cfg,
        constraints,
        export_all_csv_path,
        total_pairs,
        heatmap_bins,
        None,
        None,
        cancel,
        progress,
    )
}

pub fn optimize_double_ma_gpu_kernel_in_range(
    candles: &Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    cfg: KernelConfig,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    first_valid_override: Option<usize>,
    eval_range: Option<(usize, usize)>,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    optimize_double_ma_gpu_kernel_with_precomputed(
        candles,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        strategy,
        objective,
        top_k,
        include_all,
        cfg,
        None,
        None,
        constraints,
        export_all_csv_path,
        total_pairs,
        heatmap_bins,
        first_valid_override,
        eval_range,
        cancel,
        progress,
    )
}

pub fn optimize_double_ma_gpu_kernel_with_precomputed(
    candles: &Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<&[f64]>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    cfg: KernelConfig,
    fast_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    slow_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    first_valid_override: Option<usize>,
    eval_range: Option<(usize, usize)>,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    optimize_double_ma_gpu_kernel_with_precomputed_and_regime(
        candles,
        fast_periods,
        slow_periods,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter.map(RegimeFilterInput::HostF64),
        strategy,
        objective,
        top_k,
        include_all,
        cfg,
        fast_precomputed,
        slow_precomputed,
        constraints,
        export_all_csv_path,
        total_pairs,
        heatmap_bins,
        first_valid_override,
        eval_range,
        cancel,
        progress,
    )
}

pub(crate) fn optimize_double_ma_gpu_kernel_with_precomputed_and_regime(
    candles: &Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<RegimeFilterInput<'_>>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    cfg: KernelConfig,
    fast_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    slow_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    first_valid_override: Option<usize>,
    eval_range: Option<(usize, usize)>,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<OptimizationResult, String> {
    #[cfg(test)]
    let _gpu_guard = lock_gpu_test();
    let device_id = cfg.device_id;
    clear_last_kernel_executor_timing();
    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<OptimizationResult, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            let rt = CudaKernelRuntime::new(device_id)?;
            map.insert(device_id, rt);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;

        let (result, timing) = optimize_double_ma_gpu_kernel_impl(
            rt,
            candles,
            fast_periods,
            slow_periods,
            fast_ma_type,
            slow_ma_type,
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter,
            strategy,
            objective,
            top_k,
            include_all,
            cfg,
            fast_precomputed,
            slow_precomputed,
            constraints,
            export_all_csv_path,
            total_pairs,
            heatmap_bins,
            first_valid_override,
            eval_range,
            cancel,
            progress,
        )?;
        record_last_kernel_executor_timing(timing);
        Ok(result)
    })
}

fn optimize_double_ma_gpu_kernel_impl(
    rt: &mut CudaKernelRuntime,
    candles: &Candles,
    fast_periods: &[u16],
    slow_periods: &[u16],
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    regime_filter: Option<RegimeFilterInput<'_>>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    cfg: KernelConfig,
    fast_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    slow_precomputed: Option<&PrecomputedHostMaMatrix<'_>>,
    constraints: Option<&DoubleMaConstraints>,
    export_all_csv_path: Option<&str>,
    total_pairs: usize,
    heatmap_bins: usize,
    first_valid_override: Option<usize>,
    eval_range: Option<(usize, usize)>,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<(OptimizationResult, KernelExecutorTimingMs), String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }
    let pipeline_profile_enabled = kernel_pipeline_profile_enabled();
    let mut pipeline_profile = if pipeline_profile_enabled {
        Some(PipelineTimingProfile::default())
    } else {
        None
    };
    let host_prep_t0 = if pipeline_profile_enabled {
        Some(Instant::now())
    } else {
        None
    };
    let mut gpu_indicator_prep_elapsed = Duration::ZERO;
    let mut h2d_upload_elapsed = Duration::ZERO;

    let export_path = export_all_csv_path.map(str::trim).filter(|s| !s.is_empty());
    if export_path.is_some() && include_all {
        return Err(
            "export_all_csv_path requires include_all=false (stream export avoids huge RAM usage)"
                .to_string(),
        );
    }
    let export_enabled = export_path.is_some();
    let mut export_writer: Option<BufWriter<File>> = if let Some(path) = export_path {
        let f = File::create(path)
            .map_err(|e| format!("failed to create export file '{path}': {e}"))?;
        let mut w = BufWriter::new(f);
        writeln!(
            w,
            "fast_ma,slow_ma,fast_len,slow_len,pnl,sharpe,max_dd,downside_deviation,ulcer_index,martin_ratio,trades,exposure,net_exposure,score"
        )
        .map_err(|e| e.to_string())?;
        Some(w)
    } else {
        None
    };

    if fast_periods.is_empty() || slow_periods.is_empty() {
        return Err("empty fast/slow period list".to_string());
    }

    let fast_ma = fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = slow_ma_type.trim().to_ascii_lowercase();
    let fast_precomputed_enabled = fast_precomputed.is_some();
    let slow_precomputed_enabled = slow_precomputed.is_some();
    let fast_on_gpu = !fast_precomputed_enabled;
    let slow_on_gpu = !slow_precomputed_enabled;
    let needs_ma_prices = (fast_on_gpu && ma_uses_source(fast_ma.as_str()))
        || (slow_on_gpu && ma_uses_source(slow_ma.as_str()));
    let needs_volume = (fast_on_gpu && ma_requires_volume(fast_ma.as_str()))
        || (slow_on_gpu && ma_requires_volume(slow_ma.as_str()));
    let needs_high_low = (fast_on_gpu && ma_requires_high_low_close(fast_ma.as_str()))
        || (slow_on_gpu && ma_requires_high_low_close(slow_ma.as_str()));

    validate_candles_for_selected_mas(
        candles,
        needs_ma_prices,
        ma_source,
        needs_volume,
        needs_high_low,
    )?;

    let close = source_type(candles, "close");
    let ma_prices = if needs_ma_prices {
        Some(source_type(candles, ma_source))
    } else {
        None
    };
    let volume = if needs_volume {
        Some(source_type(candles, "volume"))
    } else {
        None
    };
    let high = if needs_high_low {
        Some(source_type(candles, "high"))
    } else {
        None
    };
    let low = if needs_high_low {
        Some(source_type(candles, "low"))
    } else {
        None
    };
    let n = close.len();
    if n < 2 {
        return Err("not enough candles".to_string());
    }
    if let Some(m) = fast_precomputed {
        validate_precomputed_host_matrix(m, fast_periods, n, "fast")?;
    }
    if let Some(m) = slow_precomputed {
        validate_precomputed_host_matrix(m, slow_periods, n, "slow")?;
    }
    if ma_prices.map_or(false, |p| p.len() != n) {
        return Err("ma source length mismatch".to_string());
    }
    if volume.map_or(false, |v| v.len() != n) {
        return Err("volume length mismatch".to_string());
    }
    if high.map_or(false, |h| h.len() != n) {
        return Err("high length mismatch".to_string());
    }
    if low.map_or(false, |l| l.len() != n) {
        return Err("low length mismatch".to_string());
    }
    let host_regime_filter = match regime_filter {
        Some(RegimeFilterInput::HostF64(values)) => Some(values),
        _ => None,
    };
    let device_regime_filter = match regime_filter {
        Some(RegimeFilterInput::DeviceF32(values)) => Some(values),
        _ => None,
    };
    let regime_f32: Option<Vec<f32>> = if let Some(regime) = host_regime_filter {
        if regime.len() != n {
            return Err("regime trend series length mismatch".to_string());
        }
        Some(regime.iter().map(|&x| x as f32).collect())
    } else {
        None
    };
    if (regime_f32.is_some() || device_regime_filter.is_some())
        && !crate::double_ma::kernel_regime_filters_enabled()
    {
        return Err(
            "GPU kernel regime filters are disabled (set VECTORBT_KERNEL_ENABLE_REGIME_FILTERS=1 to enable)"
                .to_string(),
        );
    }

    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let mut start_at = fv_close;
    if let Some(p) = ma_prices {
        let fv_ma =
            first_finite_idx(p).ok_or_else(|| "ma source series all non-finite".to_string())?;
        start_at = start_at.max(fv_ma);
    }
    if let Some(v) = volume {
        let fv_vol =
            first_finite_idx(v).ok_or_else(|| "volume series all non-finite".to_string())?;
        start_at = start_at.max(fv_vol);
    }
    if let Some(h) = high {
        let fv_high =
            first_finite_idx(h).ok_or_else(|| "high series all non-finite".to_string())?;
        start_at = start_at.max(fv_high);
    }
    if let Some(l) = low {
        let fv_low = first_finite_idx(l).ok_or_else(|| "low series all non-finite".to_string())?;
        start_at = start_at.max(fv_low);
    }
    let derived_first_valid =
        first_valid_idx_required(close, ma_prices, volume, high, low, start_at)
            .ok_or_else(|| "no index where all required series are finite".to_string())?;
    let first_valid = if let Some(override_idx) = first_valid_override {
        if override_idx < derived_first_valid {
            return Err(format!(
                "first_valid_override {override_idx} precedes required market first_valid {derived_first_valid}"
            ));
        }
        if override_idx >= n {
            return Err(format!(
                "first_valid_override {override_idx} exceeds series len {n}"
            ));
        }
        override_idx
    } else {
        derived_first_valid
    };
    let regime_first_valid = if let Some(regime) = host_regime_filter {
        first_finite_idx(regime).ok_or_else(|| "regime trend series all non-finite".to_string())?
    } else if let Some(regime) = device_regime_filter {
        if regime.len() != n {
            return Err("device regime trend series length mismatch".to_string());
        }
        if regime.device_id() != cfg.device_id {
            return Err(format!(
                "device regime filter device mismatch (expected {}, got {})",
                cfg.device_id,
                regime.device_id()
            ));
        }
        regime.first_valid()
    } else {
        first_valid
    };
    if fast_on_gpu && !crate::vram_ma::supports_vram_kernel_ma(fast_ma.as_str()) {
        return Err(format!(
            "GPU kernel backend does not support fast='{fast_ma_type}' without precomputed fast matrix"
        ));
    }
    if slow_on_gpu && !crate::vram_ma::supports_vram_kernel_ma(slow_ma.as_str()) {
        return Err(format!(
            "GPU kernel backend does not support slow='{slow_ma_type}' without precomputed slow matrix"
        ));
    }
    if let (Some(t0), Some(p)) = (host_prep_t0, pipeline_profile.as_mut()) {
        p.host_prep = p.host_prep.saturating_add(t0.elapsed());
    }

    let f_total = fast_periods.len();
    let s_total = slow_periods.len();
    let max_pf = *fast_periods.iter().max().unwrap_or(&1) as usize;
    let max_ps = *slow_periods.iter().max().unwrap_or(&1) as usize;

    if fast_ma == "wma" && fast_periods.iter().any(|&p| p <= 1) {
        return Err("WMA periods must be > 1 for the GPU kernel backend".to_string());
    }
    if slow_ma == "wma" && slow_periods.iter().any(|&p| p <= 1) {
        return Err("WMA periods must be > 1 for the GPU kernel backend".to_string());
    }

    let max_period = max_pf.max(max_ps);
    let max_start_t = first_valid
        .saturating_add(max_period.saturating_sub(1))
        .max(regime_first_valid);
    let (eval_start_t, eval_end_t) = eval_range.unwrap_or((0, n));
    let eval_end_t = eval_end_t.min(n);
    let eval_effective_start = max_start_t.max(eval_start_t);
    if n.saturating_sub(max_start_t) < 1 {
        return Err(format!(
            "not enough valid data after first_valid={} regime_first_valid={} for max period {} (len={})",
            first_valid,
            regime_first_valid,
            max_period,
            n
        ));
    }
    if eval_end_t <= eval_effective_start.saturating_add(1) {
        return Err(format!(
            "evaluation window [{eval_start_t}, {eval_end_t}) has insufficient bars after warmup (effective_start={eval_effective_start})"
        ));
    }

    let heatmap_cfg = if heatmap_bins > 0 {
        Some((
            *fast_periods.iter().min().unwrap_or(&0),
            *fast_periods.iter().max().unwrap_or(&0),
            *slow_periods.iter().min().unwrap_or(&0),
            *slow_periods.iter().max().unwrap_or(&0),
        ))
    } else {
        None
    };

    let mut agg = StreamAggregator::new(objective, top_k, include_all, n);
    if include_all {
        if let Some((f_min, f_max, s_min, s_max)) = heatmap_cfg {
            agg = agg.with_heatmap(heatmap_bins, heatmap_bins, f_min, f_max, s_min, s_max);
        }
    }
    let mut processed = 0usize;
    emit_progress(progress, processed, total_pairs, "gpu-kernel");

    let metrics_count: usize = search_metrics_count_for_optimization(
        objective,
        constraints,
        include_all,
        export_enabled,
        fast_precomputed_enabled,
        slow_precomputed_enabled,
    );
    match objective {
        ObjectiveKind::Pnl
        | ObjectiveKind::Sharpe
        | ObjectiveKind::MaxDrawdown
        | ObjectiveKind::Calmar
        | ObjectiveKind::Volatility => {}
        ObjectiveKind::Sortino | ObjectiveKind::Omega => {
            return Err(format!(
                "GPU kernel backend does not support objective {objective:?}"
            ));
        }
    }
    let topk_select: usize = if top_k == 0 { 1 } else { top_k };
    if !include_all && topk_select > 256 {
        return Err(
            "GPU kernel backend top_k too large (max 256 when include_all=false)".to_string(),
        );
    }
    let objective_i: i32 = match objective {
        ObjectiveKind::Pnl => 0,
        ObjectiveKind::Sharpe => 1,
        ObjectiveKind::MaxDrawdown => 2,
        ObjectiveKind::Calmar => 3,
        ObjectiveKind::Volatility => 4,
        ObjectiveKind::Sortino | ObjectiveKind::Omega => {
            return Err(format!(
                "GPU kernel backend does not support objective {objective:?}"
            ));
        }
    };
    let bt_stream = &rt.stream;
    let bt_module = &rt.module;
    let device_launch_caps = rt.launch_caps;
    let compute_returns = bt_module
        .get_function("compute_log_and_simple_returns_f32")
        .map_err(|e| e.to_string())?;
    let compute_returns_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::ComputeReturns, &compute_returns);
    let use_fast_path = kernel_fast_path_enabled(fast_ma.as_str(), slow_ma.as_str())
        && !fast_precomputed_enabled
        && !slow_precomputed_enabled;
    let use_valid_pair_map = kernel_valid_pair_map_enabled();
    let valid_pair_map_max_pairs = kernel_valid_pair_map_max_pairs();

    let transpose = bt_module
        .get_function("transpose_row_to_tm")
        .map_err(|e| e.to_string())?;
    let backtest = bt_module
        .get_function("double_cross_backtest_tm_flex_f32")
        .map_err(|e| e.to_string())?;
    let backtest_windowed = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed")
        .map_err(|e| e.to_string())?;
    let backtest_m7 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m7")
        .ok();
    let backtest_lockstep = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time")
        .ok();
    let backtest_lockstep_m7 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7")
        .ok();
    let backtest_lockstep_m7_regime_off = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off")
        .ok();
    let backtest_lockstep_m7_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk")
        .ok();
    let backtest_lockstep_m7_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0")
        .ok();
    let backtest_lockstep_m7_regime_on = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on")
        .ok();
    let backtest_lockstep_m7_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on_norisk")
        .ok();
    let backtest_lockstep_m7_regime_hybrid = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid")
        .ok();
    let backtest_lockstep_m7_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk")
        .ok();
    let backtest_lockstep_m7_regime_hybrid_norisk_eps0 = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0",
        )
        .ok();
    let backtest_fast_warp = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_fast_warp_dedup")
        .ok();
    let fill_f32 = bt_module
        .get_function("fill_f32")
        .map_err(|e| e.to_string())?;
    let heatmap_update = bt_module
        .get_function("heatmap_update_scores_tm_f32")
        .map_err(|e| e.to_string())?;
    let select_topk_pairs_legacy = bt_module
        .get_function("select_topk_from_pairs_tm_f32")
        .map_err(|e| e.to_string())?;
    let select_topk_candidates_legacy = bt_module
        .get_function("select_topk_from_candidates_tm_f32")
        .map_err(|e| e.to_string())?;
    let select_topk_pairs_v2 = bt_module
        .get_function("select_topk_from_pairs_tm_f32_v2")
        .ok();
    let select_topk_candidates_v2 = bt_module
        .get_function("select_topk_from_candidates_tm_f32_v2")
        .ok();
    let select_v2_supported = !kernel_force_legacy_topk_enabled()
        && select_topk_pairs_v2.is_some()
        && select_topk_candidates_v2.is_some();
    let gather_topk = bt_module
        .get_function("gather_topk_pairs_tm_f32")
        .map_err(|e| e.to_string())?;
    let pack_metric_rows = bt_module
        .get_function("pack_metric_rows_f32")
        .map_err(|e| e.to_string())?;
    let select_topk_rows_legacy = bt_module.get_function("select_topk_metric_rows_f32").ok();
    let select_topk_rows_candidates_legacy = bt_module
        .get_function("select_topk_metric_rows_from_candidates_f32")
        .ok();
    let select_topk_rows_v2 = bt_module
        .get_function("select_topk_metric_rows_f32_v2")
        .ok();
    let select_topk_rows_candidates_v2 = bt_module
        .get_function("select_topk_metric_rows_from_candidates_f32_v2")
        .ok();
    let gather_topk_rows = bt_module.get_function("gather_topk_metric_rows_f32").ok();
    let fill_launch_caps = rt.launch_caps_for_kernel(CachedKernelKey::FillF32, &fill_f32);
    let transpose_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::TransposeRowToTm, &transpose);
    let backtest_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktest, &backtest);
    let backtest_m7_launch_caps = backtest_m7
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM7, f));
    let backtest_lockstep_launch_caps = backtest_lockstep
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstep, f));
    let backtest_lockstep_m7_launch_caps = backtest_lockstep_m7
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstepM7, f));
    let backtest_fast_launch_caps = backtest_fast_warp
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestFastWarpDedup, f));
    let heatmap_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::HeatmapUpdateScores, &heatmap_update);

    let ma_source_key = normalize_market_source_key(ma_source);
    let ma_is_close = ma_source_key == "close";
    let alloc_ma_buf = needs_ma_prices && !ma_is_close;
    let (resident_market, ma, scratch, valid_pair_maps) = (
        &mut rt.resident_market,
        &mut rt.ma,
        &mut rt.scratch,
        &mut rt.valid_pair_maps,
    );
    ensure_resident_market_series(
        bt_stream,
        resident_market,
        candles,
        close,
        ma_source,
        ma_prices,
        needs_ma_prices,
        needs_high_low,
        high,
        low,
        needs_volume,
        volume,
    )?;
    let resident = resident_market
        .as_ref()
        .ok_or_else(|| "resident market cache missing after upload".to_string())?;
    let d_close = &resident.close;
    let d_high = if needs_high_low {
        resident.high.as_ref()
    } else {
        None
    };
    let d_low = if needs_high_low {
        resident.low.as_ref()
    } else {
        None
    };
    let d_volume = if needs_volume {
        resident.volume.as_ref()
    } else {
        None
    };
    let d_regime = match regime_f32.as_deref() {
        Some(src) => Some(upload_ephemeral_series_f32(bt_stream, src)?),
        None => None,
    };
    let d_regime_ptr = d_regime
        .as_ref()
        .map(|d| d.as_device_ptr().as_raw())
        .or_else(|| device_regime_filter.map(DeviceRegimeFilter::as_device_ptr_raw));
    let d_ma_prices: &DeviceBuffer<f32> = if alloc_ma_buf {
        resident
            .source_overrides
            .get(&ma_source_key)
            .ok_or_else(|| format!("resident ma_source buffer missing for {ma_source_key}"))?
    } else {
        d_close
    };

    let vram_inputs = VramMaInputs {
        prices: d_ma_prices,
        close: d_close,
        high: d_high,
        low: d_low,
        volume: d_volume,
    };

    let mut fast_params_from_cfg: HashMap<String, f64> = HashMap::new();
    let mut slow_params_from_cfg: HashMap<String, f64> = HashMap::new();
    let fast_params: Option<&HashMap<String, f64>> =
        if fast_ma == "alma" && fast_ma_params.is_none() {
            fast_params_from_cfg.insert("offset".to_string(), cfg.fast_alma_offset);
            fast_params_from_cfg.insert("sigma".to_string(), cfg.fast_alma_sigma);
            Some(&fast_params_from_cfg)
        } else {
            fast_ma_params
        };
    let slow_params: Option<&HashMap<String, f64>> =
        if slow_ma == "alma" && slow_ma_params.is_none() {
            slow_params_from_cfg.insert("offset".to_string(), cfg.slow_alma_offset);
            slow_params_from_cfg.insert("sigma".to_string(), cfg.slow_alma_sigma);
            Some(&slow_params_from_cfg)
        } else {
            slow_ma_params
        };
    if fast_ma == "sma" || slow_ma == "sma" {
        let t0 = Instant::now();
        ma.ensure_sma_prefix_f64(d_ma_prices, n, first_valid)?;
        gpu_indicator_prep_elapsed = gpu_indicator_prep_elapsed.saturating_add(t0.elapsed());
    }
    if fast_ma == "vwma" || slow_ma == "vwma" {
        let t0 = Instant::now();
        ma.ensure_vwma_prefix_pv_vol_f64(
            d_ma_prices,
            d_volume
                .as_ref()
                .ok_or_else(|| "vwma requires volume".to_string())?,
            n,
            first_valid,
        )?;
        gpu_indicator_prep_elapsed = gpu_indicator_prep_elapsed.saturating_add(t0.elapsed());
    }

    let lockstep_m7_regime_mode_specialize_enabled =
        kernel_lockstep_m7_regime_mode_specialize_enabled_for_pair(
            fast_ma.as_str(),
            slow_ma.as_str(),
        );
    let lockstep_m7_norisk_specialize_enabled =
        kernel_lockstep_m7_norisk_specialize_enabled_for_pair(fast_ma.as_str(), slow_ma.as_str());
    let lockstep_m7_eps0_specialize_enabled =
        kernel_lockstep_m7_eps0_specialize_enabled_for_pair(fast_ma.as_str(), slow_ma.as_str());
    let fast_warp_dedup_enabled = kernel_fast_warp_dedup_enabled();
    let fast_warp_dedup_min_pairs = kernel_fast_warp_dedup_min_pairs();

    KernelScratch::ensure_len_f32(&mut scratch.d_lr, n, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_ret1, n, bt_stream)?;
    let d_lr_ptr = scratch.d_lr.as_ref().unwrap().as_device_ptr().as_raw();
    let d_ret1_ptr = scratch.d_ret1.as_ref().unwrap().as_device_ptr().as_raw();
    unsafe {
        let mut prices_ptr = d_close.as_device_ptr().as_raw();
        let mut t_i = n as i32;
        let mut lr_out_ptr = d_lr_ptr;
        let mut ret1_out_ptr = d_ret1_ptr;
        let launch = choose_launch_shape_1d(n, device_launch_caps, compute_returns_launch_caps);
        let block_x: u32 = launch.block_x;
        let grid_x: u32 = launch.grid_x;
        let args: &mut [*mut c_void] = &mut [
            &mut prices_ptr as *mut _ as *mut c_void,
            &mut t_i as *mut _ as *mut c_void,
            &mut lr_out_ptr as *mut _ as *mut c_void,
            &mut ret1_out_ptr as *mut _ as *mut c_void,
        ];
        bt_stream
            .launch(&compute_returns, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
            .map_err(|e| e.to_string())?;
    }

    let (pf_tile, ps_tile) = choose_tiles(
        f_total,
        s_total,
        n,
        max_pf,
        max_ps,
        fast_ma.as_str(),
        slow_ma.as_str(),
        !alloc_ma_buf,
        metrics_count,
        topk_select,
        include_all,
        heatmap_bins,
    );
    let is_multi_tile_workload = pf_tile < f_total || ps_tile < s_total;
    let metrics_pad8_enabled =
        kernel_metrics_pad8_enabled_for_pair(fast_ma.as_str(), slow_ma.as_str());
    let tile_metrics_stride = effective_metrics_stride(metrics_count, metrics_pad8_enabled, None);

    let pad_tm = kernel_tm_padding_enabled();
    let pf_tile_tm = if pad_tm {
        align_tm_stride(pf_tile.max(1))
    } else {
        pf_tile.max(1)
    };
    let ps_tile_tm = if pad_tm {
        align_tm_stride(ps_tile.max(1))
    } else {
        ps_tile.max(1)
    };

    let fast_ma_cap = pf_tile_tm
        .checked_mul(n)
        .ok_or_else(|| "pf_tile*n overflow".to_string())?;
    let slow_ma_cap = ps_tile_tm
        .checked_mul(n)
        .ok_or_else(|| "ps_tile*n overflow".to_string())?;
    let tile_out_cap = pf_tile
        .checked_mul(ps_tile)
        .and_then(|v| v.checked_mul(tile_metrics_stride))
        .ok_or_else(|| "pf_tile*ps_tile*metrics overflow".to_string())?;

    KernelScratch::ensure_len_i32(&mut scratch.d_fast_p, pf_tile_tm, bt_stream)?;
    if use_fast_path || fast_precomputed.is_some() {
        scratch.d_fast_ma = None;
    } else {
        KernelScratch::ensure_len_f32(&mut scratch.d_fast_ma, fast_ma_cap, bt_stream)?;
    }
    KernelScratch::ensure_len_f32(&mut scratch.d_fast_ma_tm, fast_ma_cap, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_slow_p, ps_tile_tm, bt_stream)?;
    if use_fast_path || slow_precomputed.is_some() {
        scratch.d_slow_ma = None;
    } else {
        KernelScratch::ensure_len_f32(&mut scratch.d_slow_ma, slow_ma_cap, bt_stream)?;
    }
    KernelScratch::ensure_len_f32(&mut scratch.d_slow_ma_tm, slow_ma_cap, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_tile, tile_out_cap, bt_stream)?;

    let mut host_p_fast: Vec<i32> = Vec::with_capacity(pf_tile_tm.max(1));
    let mut host_p_slow: Vec<i32> = Vec::with_capacity(ps_tile_tm.max(1));

    let constraints_on_device = constraints.is_some() && !include_all && select_v2_supported;
    let host_filter_mode = constraints.is_some() && !include_all && !constraints_on_device;
    let compact_valid_pair_output = should_use_compact_valid_pair_output(
        include_all,
        export_enabled,
        host_filter_mode,
        metrics_count,
    );
    let row_select_v2_supported = !kernel_force_legacy_topk_enabled()
        && select_topk_rows_v2.is_some()
        && select_topk_rows_candidates_v2.is_some();
    let global_topk_mode = kernel_global_topk_enabled()
        && !include_all
        && !export_enabled
        && !host_filter_mode
        && topk_select > 0
        && select_topk_rows_legacy.is_some()
        && select_topk_rows_candidates_legacy.is_some()
        && gather_topk_rows.is_some();
    let global_constraints_on_device =
        constraints.is_some() && global_topk_mode && row_select_v2_supported;
    let (constraint_use_min_trades, constraint_min_trades) =
        match constraints.and_then(|c| c.min_trades) {
            Some(v) => (1i32, v as f32),
            None => (0i32, 0.0f32),
        };
    let (constraint_use_max_dd, constraint_max_dd) = match constraints.and_then(|c| c.max_drawdown)
    {
        Some(v) => (
            1i32,
            max_drawdown_constraint_to_search_space(v, metrics_count),
        ),
        None => (0i32, f32::INFINITY),
    };
    let mut tile_host_buf: Vec<f32> = if include_all || export_enabled || host_filter_mode {
        alloc_uninit_copy(tile_out_cap)
    } else {
        Vec::new()
    };
    let mut fast_precomputed_host_tile: Vec<f32> = Vec::new();
    let mut slow_precomputed_host_tile: Vec<f32> = Vec::new();

    let mut global_fast_rows: Option<DeviceBuffer<i32>> = None;
    let mut global_slow_rows: Option<DeviceBuffer<i32>> = None;
    let mut global_metrics_rows: Option<DeviceBuffer<f32>> = None;
    let mut global_rows_len: usize = 0;
    let global_rows_cap: usize = if global_topk_mode {
        topk_select.saturating_mul(2).max(topk_select)
    } else {
        0
    };

    let mut top_packed_host: Vec<f32> = Vec::new();

    if !include_all {
        let max_pairs_tile = pf_tile.saturating_mul(ps_tile).max(1);
        let blocks_max = (max_pairs_tile + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let cand_len_max = blocks_max.saturating_mul(topk_select).max(topk_select);

        KernelScratch::ensure_len_i32(&mut scratch.d_cand_a, cand_len_max, bt_stream)?;
        KernelScratch::ensure_len_i32(&mut scratch.d_cand_b, cand_len_max, bt_stream)?;
        KernelScratch::ensure_len_i32(&mut scratch.d_top_fast, topk_select, bt_stream)?;
        KernelScratch::ensure_len_i32(&mut scratch.d_top_slow, topk_select, bt_stream)?;
        KernelScratch::ensure_len_f32(
            &mut scratch.d_top_metrics,
            topk_select * metrics_count,
            bt_stream,
        )?;
        top_packed_host = alloc_uninit_copy(topk_select * metrics_count.saturating_add(2));
        if global_topk_mode {
            KernelScratch::ensure_len_i32(&mut global_fast_rows, global_rows_cap, bt_stream)?;
            KernelScratch::ensure_len_i32(&mut global_slow_rows, global_rows_cap, bt_stream)?;
            KernelScratch::ensure_len_f32(
                &mut global_metrics_rows,
                global_rows_cap.saturating_mul(metrics_count),
                bt_stream,
            )?;
        }

        if heatmap_cfg.is_some() {
            let heatmap_len = heatmap_bins.saturating_mul(heatmap_bins);
            if heatmap_len > 0 {
                KernelScratch::ensure_len_f32(&mut scratch.d_heatmap, heatmap_len, bt_stream)?;
                unsafe {
                    let mut out_ptr = scratch.d_heatmap.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut n_i = heatmap_len as i32;
                    let mut v = f32::NEG_INFINITY;
                    let block_x: u32 = 256;
                    let grid_x: u32 = ((heatmap_len as u32) + block_x - 1) / block_x;
                    let args: &mut [*mut c_void] = &mut [
                        &mut out_ptr as *mut _ as *mut c_void,
                        &mut n_i as *mut _ as *mut c_void,
                        &mut v as *mut _ as *mut c_void,
                    ];
                    bt_stream
                        .launch(&fill_f32, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    let strategy_kernel = kernel_strategy_params(strategy);
    let flags = strategy_kernel.flags;
    let commission = strategy_kernel.commission;
    let eps_rel = strategy_kernel.eps_rel;
    let stop_log = strategy_kernel.stop_log;
    let take_log = strategy_kernel.take_log;
    let regime_use = if d_regime.is_some() { 1i32 } else { 0i32 };
    let regime_use_hybrid = if d_regime.is_some() && strategy.uses_regime_hybrid_filter() {
        1i32
    } else {
        0i32
    };
    let regime_entry_rel = if regime_use_hybrid != 0 {
        strategy.regime_hybrid_reversion_entry_rel() as f32
    } else {
        0.0
    };
    let regime_exit_rel = if regime_use_hybrid != 0 {
        strategy.regime_hybrid_reversion_exit_rel() as f32
    } else {
        0.0
    };
    // Graph replay is currently limited to non-heatmap runs; heatmap parity can diverge
    // between batched/unbatched when score+heatmap nodes are captured and reused.
    let use_cuda_graphs = !pipeline_profile_enabled
        && kernel_cuda_graphs_enabled()
        && !include_all
        && !host_filter_mode
        && heatmap_cfg.is_none();
    let mut tile_graph_exec: Option<CudaGraphExecHandle> = None;
    let mut tile_graph_shape: Option<(usize, usize, usize, usize, bool, bool)> = None;
    let mut graph_capture_disabled = false;

    let mut f_start = 0usize;
    while f_start < f_total {
        let pf = pf_tile.min(f_total - f_start);
        let fast_tm_rows = if pad_tm {
            align_tm_stride(pf.max(1))
        } else {
            pf.max(1)
        };

        host_p_fast.clear();
        host_p_fast.extend(
            fast_periods[f_start..f_start + pf]
                .iter()
                .map(|&x| x as i32),
        );
        let f_pad = host_p_fast.last().copied().unwrap_or(2);
        host_p_fast.resize(pf_tile_tm, f_pad);
        // Host period vectors are pageable and mutated per tile; keep this copy
        // synchronous to avoid host-data races with async H2D semantics.
        let h2d_t0 = Instant::now();
        scratch
            .d_fast_p
            .as_mut()
            .unwrap()
            .copy_from(&host_p_fast)
            .map_err(|e| e.to_string())?;
        h2d_upload_elapsed = h2d_upload_elapsed.saturating_add(h2d_t0.elapsed());
        if let Some(precomputed_fast) = fast_precomputed {
            pack_precomputed_rows_tm_with_padding(
                precomputed_fast,
                f_start,
                pf,
                fast_tm_rows,
                &mut fast_precomputed_host_tile,
            )?;
            let h2d_t0 = Instant::now();
            scratch
                .d_fast_ma_tm
                .as_mut()
                .ok_or_else(|| {
                    "internal error: missing d_fast_ma_tm for precomputed fast matrix".to_string()
                })?
                .copy_from(&fast_precomputed_host_tile)
                .map_err(|e| e.to_string())?;
            h2d_upload_elapsed = h2d_upload_elapsed.saturating_add(h2d_t0.elapsed());
        } else if use_fast_path {
            let prep_t0 = Instant::now();
            ma.compute_period_ma_tm_into(
                fast_ma.as_str(),
                fast_params,
                &vram_inputs,
                n,
                first_valid,
                &host_p_fast[..fast_tm_rows],
                scratch.d_fast_p.as_ref().unwrap(),
                scratch.d_fast_ma_tm.as_mut().unwrap(),
            )?;
            gpu_indicator_prep_elapsed =
                gpu_indicator_prep_elapsed.saturating_add(prep_t0.elapsed());
        } else {
            if fast_ma == "nma" {
                fill_device_buffer_f32(
                    bt_stream,
                    &fill_f32,
                    device_launch_caps,
                    fill_launch_caps,
                    scratch.d_fast_ma.as_ref().unwrap(),
                    pf.saturating_mul(n),
                    f32::NAN,
                )?;
                fill_device_buffer_f32(
                    bt_stream,
                    &fill_f32,
                    device_launch_caps,
                    fill_launch_caps,
                    scratch.d_fast_ma_tm.as_ref().unwrap(),
                    fast_tm_rows.saturating_mul(n),
                    f32::NAN,
                )?;
            }
            let prep_t0 = Instant::now();
            ma.compute_period_ma_into(
                fast_ma.as_str(),
                fast_params,
                &vram_inputs,
                n,
                first_valid,
                &host_p_fast,
                scratch.d_fast_p.as_ref().unwrap(),
                scratch.d_fast_ma.as_mut().unwrap(),
            )?;

            unsafe {
                let mut in_ptr = scratch.d_fast_ma.as_ref().unwrap().as_device_ptr().as_raw();
                let mut rows = pf as i32;
                let mut cols = n as i32;
                let mut out_rows = fast_tm_rows as i32;
                let mut out_ptr = scratch
                    .d_fast_ma_tm
                    .as_ref()
                    .unwrap()
                    .as_device_ptr()
                    .as_raw();
                let total = pf.saturating_mul(n);
                let launch =
                    choose_launch_shape_1d(total, device_launch_caps, transpose_launch_caps);
                let block_x: u32 = launch.block_x;
                let grid_x: u32 = launch.grid_x;
                let args: &mut [*mut c_void] = &mut [
                    &mut in_ptr as *mut _ as *mut c_void,
                    &mut rows as *mut _ as *mut c_void,
                    &mut cols as *mut _ as *mut c_void,
                    &mut out_rows as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                bt_stream
                    .launch(&transpose, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                    .map_err(|e| e.to_string())?;
            }
            gpu_indicator_prep_elapsed =
                gpu_indicator_prep_elapsed.saturating_add(prep_t0.elapsed());
        }

        let mut s_start = 0usize;
        while s_start < s_total {
            let tile_total_t0 = if pipeline_profile_enabled {
                Some(Instant::now())
            } else {
                None
            };
            let mut tile_prepare_elapsed = Duration::ZERO;
            let mut tile_score_elapsed = Duration::ZERO;
            let tile_backtest_elapsed = Cell::new(Duration::ZERO);
            let tile_heatmap_elapsed = Cell::new(Duration::ZERO);
            let ps = ps_tile.min(s_total - s_start);
            let slow_tm_rows = if pad_tm {
                align_tm_stride(ps.max(1))
            } else {
                ps.max(1)
            };

            host_p_slow.clear();
            host_p_slow.extend(
                slow_periods[s_start..s_start + ps]
                    .iter()
                    .map(|&x| x as i32),
            );
            let s_pad = host_p_slow.last().copied().unwrap_or(2);
            host_p_slow.resize(ps_tile_tm, s_pad);
            // Host period vectors are pageable and mutated per tile; keep this copy
            // synchronous to avoid host-data races with async H2D semantics.
            let h2d_t0 = Instant::now();
            scratch
                .d_slow_p
                .as_mut()
                .unwrap()
                .copy_from(&host_p_slow)
                .map_err(|e| e.to_string())?;
            h2d_upload_elapsed = h2d_upload_elapsed.saturating_add(h2d_t0.elapsed());
            if let Some(precomputed_slow) = slow_precomputed {
                pack_precomputed_rows_tm_with_padding(
                    precomputed_slow,
                    s_start,
                    ps,
                    slow_tm_rows,
                    &mut slow_precomputed_host_tile,
                )?;
                let h2d_t0 = Instant::now();
                scratch
                    .d_slow_ma_tm
                    .as_mut()
                    .ok_or_else(|| {
                        "internal error: missing d_slow_ma_tm for precomputed slow matrix"
                            .to_string()
                    })?
                    .copy_from(&slow_precomputed_host_tile)
                    .map_err(|e| e.to_string())?;
                h2d_upload_elapsed = h2d_upload_elapsed.saturating_add(h2d_t0.elapsed());
            } else if use_fast_path {
                let prep_t0 = Instant::now();
                ma.compute_period_ma_tm_into(
                    slow_ma.as_str(),
                    slow_params,
                    &vram_inputs,
                    n,
                    first_valid,
                    &host_p_slow[..slow_tm_rows],
                    scratch.d_slow_p.as_ref().unwrap(),
                    scratch.d_slow_ma_tm.as_mut().unwrap(),
                )?;
                gpu_indicator_prep_elapsed =
                    gpu_indicator_prep_elapsed.saturating_add(prep_t0.elapsed());
            } else {
                if slow_ma == "nma" {
                    fill_device_buffer_f32(
                        bt_stream,
                        &fill_f32,
                        device_launch_caps,
                        fill_launch_caps,
                        scratch.d_slow_ma.as_ref().unwrap(),
                        ps.saturating_mul(n),
                        f32::NAN,
                    )?;
                    fill_device_buffer_f32(
                        bt_stream,
                        &fill_f32,
                        device_launch_caps,
                        fill_launch_caps,
                        scratch.d_slow_ma_tm.as_ref().unwrap(),
                        slow_tm_rows.saturating_mul(n),
                        f32::NAN,
                    )?;
                }
                let prep_t0 = Instant::now();
                ma.compute_period_ma_into(
                    slow_ma.as_str(),
                    slow_params,
                    &vram_inputs,
                    n,
                    first_valid,
                    &host_p_slow,
                    scratch.d_slow_p.as_ref().unwrap(),
                    scratch.d_slow_ma.as_mut().unwrap(),
                )?;

                unsafe {
                    let mut in_ptr = scratch.d_slow_ma.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut rows = ps as i32;
                    let mut cols = n as i32;
                    let mut out_rows = slow_tm_rows as i32;
                    let mut out_ptr = scratch
                        .d_slow_ma_tm
                        .as_ref()
                        .unwrap()
                        .as_device_ptr()
                        .as_raw();
                    let total = ps.saturating_mul(n);
                    let launch =
                        choose_launch_shape_1d(total, device_launch_caps, transpose_launch_caps);
                    let block_x: u32 = launch.block_x;
                    let grid_x: u32 = launch.grid_x;
                    let args: &mut [*mut c_void] = &mut [
                        &mut in_ptr as *mut _ as *mut c_void,
                        &mut rows as *mut _ as *mut c_void,
                        &mut cols as *mut _ as *mut c_void,
                        &mut out_rows as *mut _ as *mut c_void,
                        &mut out_ptr as *mut _ as *mut c_void,
                    ];
                    bt_stream
                        .launch(&transpose, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                        .map_err(|e| e.to_string())?;
                }
                gpu_indicator_prep_elapsed =
                    gpu_indicator_prep_elapsed.saturating_add(prep_t0.elapsed());
            }

            let pairs = pf
                .checked_mul(ps)
                .ok_or_else(|| "pf*ps overflow".to_string())?;
            let out_len = pairs
                .checked_mul(tile_metrics_stride)
                .ok_or_else(|| "pairs*metrics overflow".to_string())?;

            if out_len > tile_out_cap {
                return Err(format!(
                    "tile output len {out_len} exceeds allocated capacity {tile_out_cap}"
                ));
            }

            let mut launch_pairs = pairs;
            let mut pair_index_map_ptr: u64 = 0;
            let mut pair_fast_rows_ptr: u64 = 0;
            let mut pair_slow_rows_ptr: u64 = 0;
            if use_valid_pair_map && pairs <= valid_pair_map_max_pairs {
                let cached_valid_map = ensure_cached_valid_pair_map(
                    valid_pair_maps,
                    &fast_periods[f_start..f_start + pf],
                    &slow_periods[s_start..s_start + ps],
                )?;
                launch_pairs = cached_valid_map.map(|(_, len, _, _)| len).unwrap_or(0);
                if launch_pairs > 0 {
                    pair_index_map_ptr = cached_valid_map
                        .map(|(ptr, _, _, _)| ptr)
                        .unwrap_or_default();
                    pair_fast_rows_ptr = cached_valid_map
                        .map(|(_, _, fast_ptr, _)| fast_ptr)
                        .unwrap_or_default();
                    pair_slow_rows_ptr = cached_valid_map
                        .map(|(_, _, _, slow_ptr)| slow_ptr)
                        .unwrap_or_default();
                }

                if !compact_valid_pair_output {
                    // Keep output dense (Pf*Ps*M) for downstream kernels by zero-filling
                    // and only computing valid fast<slow pairs.
                    unsafe {
                        let mut out_ptr = scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut n_i = out_len as i32;
                        let mut v = 0.0f32;
                        let launch =
                            choose_launch_shape_1d(out_len, device_launch_caps, fill_launch_caps);
                        let block_x: u32 = launch.block_x;
                        let grid_x: u32 = launch.grid_x;
                        let args: &mut [*mut c_void] = &mut [
                            &mut out_ptr as *mut _ as *mut c_void,
                            &mut n_i as *mut _ as *mut c_void,
                            &mut v as *mut _ as *mut c_void,
                        ];
                        bt_stream
                            .launch(&fill_f32, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }

            if let Some(t0) = tile_total_t0 {
                tile_prepare_elapsed = t0.elapsed();
            }

            let launch_score_stage = || -> Result<(), String> {
                if launch_pairs > 0 {
                    let backtest_t0 = if pipeline_profile_enabled {
                        Some(Instant::now())
                    } else {
                        None
                    };
                    unsafe {
                        let mut f_ma_tm = scratch
                            .d_fast_ma_tm
                            .as_ref()
                            .unwrap()
                            .as_device_ptr()
                            .as_raw();
                        let mut s_ma_tm = scratch
                            .d_slow_ma_tm
                            .as_ref()
                            .unwrap()
                            .as_device_ptr()
                            .as_raw();
                        let mut lr = d_lr_ptr;
                        let mut ret1 = d_ret1_ptr;
                        let mut t_i = n as i32;
                        let mut pf_tile_i = pf as i32;
                        let mut ps_tile_i = ps as i32;
                        let mut fast_tm_stride_i = fast_tm_rows as i32;
                        let mut slow_tm_stride_i = slow_tm_rows as i32;
                        let mut pf_total_i = pf as i32;
                        let mut ps_total_i = ps as i32;
                        let mut f_off = 0i32;
                        let mut s_off = 0i32;
                        let mut f_per = scratch.d_fast_p.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut s_per = scratch.d_slow_p.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut regime_ptr = d_regime_ptr.unwrap_or(0);
                        let mut use_regime_i = regime_use;
                        let mut use_regime_hybrid_i = regime_use_hybrid;
                        let mut regime_entry = regime_entry_rel;
                        let mut regime_exit = regime_exit_rel;
                        let mut fv = first_valid as i32;
                        let mut regime_fv = regime_first_valid as i32;
                        let mut commission = commission;
                        let mut eps_rel = eps_rel;
                        let mut stop_log = stop_log;
                        let mut take_log = take_log;
                        let mut flags_u = flags;
                        let mut m_i = metrics_count as i32;
                        let mut metrics_stride_i = tile_metrics_stride as i32;
                        let mut pair_map_ptr = pair_index_map_ptr;
                        let mut pair_count_i = launch_pairs as i32;
                        let mut compact_output_i = if compact_valid_pair_output {
                            1i32
                        } else {
                            0i32
                        };
                        let mut out_ptr = scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut eval_start_i = eval_start_t as i32;
                        let mut eval_end_i = eval_end_t as i32;
                        let use_windowed_eval = eval_range.is_some();
                        let no_risk_mode = (flags_u & (STRAT_STOP_LOSS | STRAT_TAKE_PROFIT)) == 0;
                        let eps_zero_mode = eps_rel <= 0.0f32;

                        let use_lockstep_time = !use_windowed_eval
                            && kernel_should_use_lockstep_time(
                                launch_pairs,
                                n,
                                max_start_t,
                                backtest_lockstep.is_some(),
                            );
                        let use_m7 = !use_windowed_eval
                            && kernel_should_use_m7_specialize(
                                launch_pairs,
                                n,
                                max_start_t,
                                is_multi_tile_workload,
                                metrics_count,
                                backtest_m7.is_some(),
                            );
                        let use_lockstep_m7 = use_lockstep_time
                            && !is_multi_tile_workload
                            && kernel_should_use_m7_specialize(
                                launch_pairs,
                                n,
                                max_start_t,
                                false,
                                metrics_count,
                                backtest_lockstep_m7.is_some(),
                            );
                        let use_fast_warp_dedup = !use_windowed_eval
                            && !use_lockstep_time
                            && fast_warp_dedup_enabled
                            && launch_pairs >= fast_warp_dedup_min_pairs
                            && backtest_fast_warp.is_some();
                        let launch_caps = if use_windowed_eval {
                            backtest_launch_caps
                        } else if use_lockstep_m7 {
                            backtest_lockstep_m7_launch_caps
                                .or(backtest_lockstep_launch_caps)
                                .unwrap_or(backtest_launch_caps)
                        } else if use_lockstep_time {
                            backtest_lockstep_launch_caps.unwrap_or(backtest_launch_caps)
                        } else if use_fast_warp_dedup {
                            backtest_fast_launch_caps.unwrap_or(backtest_launch_caps)
                        } else if use_m7 {
                            backtest_m7_launch_caps.unwrap_or(backtest_launch_caps)
                        } else {
                            backtest_launch_caps
                        };
                        let launch = choose_backtest_launch_shape_1d(
                            launch_pairs,
                            device_launch_caps,
                            launch_caps,
                        );
                        let dispatch_label = if use_windowed_eval {
                            "windowed"
                        } else if use_lockstep_m7 {
                            "lockstep_m7"
                        } else if use_lockstep_time {
                            "lockstep_time"
                        } else if use_fast_warp_dedup {
                            "fast_warp_dedup"
                        } else if use_m7 {
                            "m7"
                        } else {
                            "baseline"
                        };
                        if kernel_log_dispatch_enabled() {
                            eprintln!(
                                "VECTORBT_KERNEL_DISPATCH: pair={}/{} launch_pairs={} fast_path={} multi_tile={} dispatch={} use_m7={} use_lockstep_time={} use_lockstep_m7={} use_fast_warp={}",
                                fast_ma,
                                slow_ma,
                                launch_pairs,
                                use_fast_path,
                                is_multi_tile_workload,
                                dispatch_label,
                                use_m7,
                                use_lockstep_time,
                                use_lockstep_m7,
                                use_fast_warp_dedup
                            );
                        }
                        let backtest_fn = if use_windowed_eval {
                            &backtest_windowed
                        } else if use_lockstep_m7 {
                            if lockstep_m7_norisk_specialize_enabled && no_risk_mode {
                                if lockstep_m7_eps0_specialize_enabled && eps_zero_mode {
                                    if use_regime_hybrid_i != 0 {
                                        backtest_lockstep_m7_regime_hybrid_norisk_eps0
                                            .as_ref()
                                            .or(backtest_lockstep_m7_regime_hybrid_norisk.as_ref())
                                            .or(backtest_lockstep_m7_regime_hybrid.as_ref())
                                            .or(backtest_lockstep_m7.as_ref())
                                            .expect("checked lockstep m7 kernel availability")
                                    } else if use_regime_i != 0 {
                                        backtest_lockstep_m7_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_lockstep_m7_regime_on.as_ref())
                                            .or(backtest_lockstep_m7.as_ref())
                                            .expect("checked lockstep m7 kernel availability")
                                    } else {
                                        backtest_lockstep_m7_regime_off_norisk_eps0
                                            .as_ref()
                                            .or(backtest_lockstep_m7_regime_off_norisk.as_ref())
                                            .or(backtest_lockstep_m7_regime_off.as_ref())
                                            .or(backtest_lockstep_m7.as_ref())
                                            .expect("checked lockstep m7 kernel availability")
                                    }
                                } else if use_regime_hybrid_i != 0 {
                                    backtest_lockstep_m7_regime_hybrid_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m7_regime_hybrid.as_ref())
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                } else if use_regime_i != 0 {
                                    backtest_lockstep_m7_regime_on_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m7_regime_on.as_ref())
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                } else {
                                    backtest_lockstep_m7_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m7_regime_off.as_ref())
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                }
                            } else if lockstep_m7_regime_mode_specialize_enabled {
                                if use_regime_hybrid_i != 0 {
                                    backtest_lockstep_m7_regime_hybrid
                                        .as_ref()
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                } else if use_regime_i != 0 {
                                    backtest_lockstep_m7_regime_on
                                        .as_ref()
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                } else {
                                    backtest_lockstep_m7_regime_off
                                        .as_ref()
                                        .or(backtest_lockstep_m7.as_ref())
                                        .expect("checked lockstep m7 kernel availability")
                                }
                            } else {
                                backtest_lockstep_m7
                                    .as_ref()
                                    .expect("checked lockstep m7 kernel availability")
                            }
                        } else if use_lockstep_time {
                            backtest_lockstep
                                .as_ref()
                                .expect("checked lockstep kernel availability")
                        } else if use_fast_warp_dedup {
                            backtest_fast_warp
                                .as_ref()
                                .expect("checked fast dedup kernel availability")
                        } else if use_m7 {
                            backtest_m7
                                .as_ref()
                                .expect("checked m7 kernel availability")
                        } else {
                            &backtest
                        };
                        let block_x: u32 = launch.block_x;
                        let grid_x: u32 = launch.grid_x;
                        if use_windowed_eval {
                            let args: &mut [*mut c_void] = &mut [
                                &mut f_ma_tm as *mut _ as *mut c_void,
                                &mut s_ma_tm as *mut _ as *mut c_void,
                                &mut lr as *mut _ as *mut c_void,
                                &mut ret1 as *mut _ as *mut c_void,
                                &mut t_i as *mut _ as *mut c_void,
                                &mut pf_tile_i as *mut _ as *mut c_void,
                                &mut ps_tile_i as *mut _ as *mut c_void,
                                &mut fast_tm_stride_i as *mut _ as *mut c_void,
                                &mut slow_tm_stride_i as *mut _ as *mut c_void,
                                &mut pf_total_i as *mut _ as *mut c_void,
                                &mut ps_total_i as *mut _ as *mut c_void,
                                &mut f_off as *mut _ as *mut c_void,
                                &mut s_off as *mut _ as *mut c_void,
                                &mut f_per as *mut _ as *mut c_void,
                                &mut s_per as *mut _ as *mut c_void,
                                &mut regime_ptr as *mut _ as *mut c_void,
                                &mut use_regime_i as *mut _ as *mut c_void,
                                &mut use_regime_hybrid_i as *mut _ as *mut c_void,
                                &mut regime_entry as *mut _ as *mut c_void,
                                &mut regime_exit as *mut _ as *mut c_void,
                                &mut fv as *mut _ as *mut c_void,
                                &mut regime_fv as *mut _ as *mut c_void,
                                &mut eval_start_i as *mut _ as *mut c_void,
                                &mut eval_end_i as *mut _ as *mut c_void,
                                &mut commission as *mut _ as *mut c_void,
                                &mut eps_rel as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            bt_stream
                                .launch(backtest_fn, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                                .map_err(|e| e.to_string())?;
                        } else {
                            let args: &mut [*mut c_void] = &mut [
                                &mut f_ma_tm as *mut _ as *mut c_void,
                                &mut s_ma_tm as *mut _ as *mut c_void,
                                &mut lr as *mut _ as *mut c_void,
                                &mut ret1 as *mut _ as *mut c_void,
                                &mut t_i as *mut _ as *mut c_void,
                                &mut pf_tile_i as *mut _ as *mut c_void,
                                &mut ps_tile_i as *mut _ as *mut c_void,
                                &mut fast_tm_stride_i as *mut _ as *mut c_void,
                                &mut slow_tm_stride_i as *mut _ as *mut c_void,
                                &mut pf_total_i as *mut _ as *mut c_void,
                                &mut ps_total_i as *mut _ as *mut c_void,
                                &mut f_off as *mut _ as *mut c_void,
                                &mut s_off as *mut _ as *mut c_void,
                                &mut f_per as *mut _ as *mut c_void,
                                &mut s_per as *mut _ as *mut c_void,
                                &mut regime_ptr as *mut _ as *mut c_void,
                                &mut use_regime_i as *mut _ as *mut c_void,
                                &mut use_regime_hybrid_i as *mut _ as *mut c_void,
                                &mut regime_entry as *mut _ as *mut c_void,
                                &mut regime_exit as *mut _ as *mut c_void,
                                &mut fv as *mut _ as *mut c_void,
                                &mut regime_fv as *mut _ as *mut c_void,
                                &mut commission as *mut _ as *mut c_void,
                                &mut eps_rel as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            bt_stream
                                .launch(backtest_fn, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    if let Some(t0) = backtest_t0 {
                        bt_stream.synchronize().map_err(|e| e.to_string())?;
                        tile_backtest_elapsed
                            .set(tile_backtest_elapsed.get().saturating_add(t0.elapsed()));
                    }
                }

                if let (Some(h), Some((f_min, f_max, s_min, s_max))) =
                    (scratch.d_heatmap.as_ref(), heatmap_cfg)
                {
                    let heatmap_t0 = if pipeline_profile_enabled {
                        Some(Instant::now())
                    } else {
                        None
                    };
                    unsafe {
                        let mut metrics_ptr =
                            scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut pairs_i = pairs as i32;
                        let mut m_i = metrics_count as i32;
                        let mut metrics_stride_i = tile_metrics_stride as i32;
                        let mut pf_i = pf as i32;
                        let mut ps_i = ps as i32;
                        let mut f_per = scratch.d_fast_p.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut s_per = scratch.d_slow_p.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut obj_i = objective_i;
                        let mut bins_f = heatmap_bins as i32;
                        let mut bins_s = heatmap_bins as i32;
                        let mut f_min_i = f_min as i32;
                        let mut f_max_i = f_max as i32;
                        let mut s_min_i = s_min as i32;
                        let mut s_max_i = s_max as i32;
                        let mut out_ptr = h.as_device_ptr().as_raw();

                        let mut pair_map_ptr = pair_index_map_ptr;
                        let mut pair_fast_rows = pair_fast_rows_ptr;
                        let mut pair_slow_rows = pair_slow_rows_ptr;
                        let mut pair_count_i = launch_pairs as i32;
                        let mut compact_output_i = if compact_valid_pair_output {
                            1i32
                        } else {
                            0i32
                        };
                        let heatmap_pairs = if pair_map_ptr != 0 && launch_pairs > 0 {
                            launch_pairs
                        } else {
                            pairs
                        };
                        let launch = choose_launch_shape_1d(
                            heatmap_pairs,
                            device_launch_caps,
                            heatmap_launch_caps,
                        );
                        let block_x: u32 = launch.block_x;
                        let grid_x: u32 = launch.grid_x;
                        let args: &mut [*mut c_void] = &mut [
                            &mut metrics_ptr as *mut _ as *mut c_void,
                            &mut pairs_i as *mut _ as *mut c_void,
                            &mut m_i as *mut _ as *mut c_void,
                            &mut metrics_stride_i as *mut _ as *mut c_void,
                            &mut pf_i as *mut _ as *mut c_void,
                            &mut ps_i as *mut _ as *mut c_void,
                            &mut f_per as *mut _ as *mut c_void,
                            &mut s_per as *mut _ as *mut c_void,
                            &mut obj_i as *mut _ as *mut c_void,
                            &mut bins_f as *mut _ as *mut c_void,
                            &mut bins_s as *mut _ as *mut c_void,
                            &mut f_min_i as *mut _ as *mut c_void,
                            &mut f_max_i as *mut _ as *mut c_void,
                            &mut s_min_i as *mut _ as *mut c_void,
                            &mut s_max_i as *mut _ as *mut c_void,
                            &mut pair_map_ptr as *mut _ as *mut c_void,
                            &mut pair_fast_rows as *mut _ as *mut c_void,
                            &mut pair_slow_rows as *mut _ as *mut c_void,
                            &mut pair_count_i as *mut _ as *mut c_void,
                            &mut compact_output_i as *mut _ as *mut c_void,
                            &mut out_ptr as *mut _ as *mut c_void,
                        ];
                        bt_stream
                            .launch(&heatmap_update, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                            .map_err(|e| e.to_string())?;
                    }
                    if let Some(t0) = heatmap_t0 {
                        bt_stream.synchronize().map_err(|e| e.to_string())?;
                        tile_heatmap_elapsed
                            .set(tile_heatmap_elapsed.get().saturating_add(t0.elapsed()));
                    }
                }
                Ok(())
            };

            let tile_score_t0 = if pipeline_profile_enabled {
                Some(Instant::now())
            } else {
                None
            };
            let can_graph_tile = use_cuda_graphs
                && !graph_capture_disabled
                && pf == pf_tile
                && ps == ps_tile
                && !host_filter_mode;
            if can_graph_tile {
                let shape = (
                    pf,
                    ps,
                    pairs,
                    launch_pairs,
                    heatmap_cfg.is_some(),
                    compact_valid_pair_output,
                );
                let reusable = tile_graph_shape.map_or(false, |s| s == shape);
                if !reusable {
                    tile_graph_exec = None;
                    tile_graph_shape = None;
                    match capture_graph_exec(bt_stream, || launch_score_stage()) {
                        Ok(exec) => {
                            tile_graph_shape = Some(shape);
                            tile_graph_exec = Some(exec);
                        }
                        Err(_e) => {
                            graph_capture_disabled = true;
                            launch_score_stage()?;
                        }
                    }
                }
                if !graph_capture_disabled {
                    if let Some(exec) = tile_graph_exec.as_ref() {
                        exec.launch(bt_stream)?;
                    }
                }
            } else {
                launch_score_stage()?;
            }
            if let Some(t0) = tile_score_t0 {
                bt_stream.synchronize().map_err(|e| e.to_string())?;
                tile_score_elapsed = t0.elapsed();
            }

            let fast_chunk = &fast_periods[f_start..f_start + pf];
            let slow_chunk = &slow_periods[s_start..s_start + ps];
            let tile_rows_compact = compact_valid_pair_output && launch_pairs < pairs;
            let tile_row_count = if tile_rows_compact {
                launch_pairs
            } else {
                pairs
            };

            if include_all {
                copy_device_prefix_to_host_f32(
                    scratch.d_tile.as_ref().unwrap(),
                    &mut tile_host_buf,
                    tile_row_count.saturating_mul(tile_metrics_stride),
                    bt_stream,
                )?;
                bt_stream.synchronize().map_err(|e| e.to_string())?;

                for_each_tile_metric_row(
                    fast_chunk,
                    slow_chunk,
                    tile_rows_compact,
                    |row_idx, fast_len, slow_len| {
                        let src_base = row_idx.saturating_mul(tile_metrics_stride);
                        if src_base + metrics_count > tile_host_buf.len() {
                            return Ok(());
                        }
                        let metrics =
                            decode_tm_flex_metrics_row(&tile_host_buf, src_base, metrics_count);
                        if metrics_pass_constraints(&metrics, constraints) {
                            let params = DoubleMaParams {
                                fast_len,
                                slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            };
                            agg.push(params, metrics);
                        }
                        processed = processed.saturating_add(1);
                        if (processed & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                            return Err("cancelled".to_string());
                        }
                        Ok(())
                    },
                )?;
            } else {
                let s_len = slow_chunk.len();
                let mut tile_valid: usize = 0;
                for &f in fast_chunk {
                    let idx = slow_chunk.partition_point(|&s| s <= f);
                    tile_valid = tile_valid.saturating_add(s_len.saturating_sub(idx));
                }

                if host_filter_mode {
                    copy_device_prefix_to_host_f32(
                        scratch.d_tile.as_ref().unwrap(),
                        &mut tile_host_buf,
                        tile_row_count.saturating_mul(tile_metrics_stride),
                        bt_stream,
                    )?;
                    bt_stream.synchronize().map_err(|e| e.to_string())?;

                    let mut exported = 0usize;
                    for_each_tile_metric_row(
                        fast_chunk,
                        slow_chunk,
                        tile_rows_compact,
                        |row_idx, fast_len, slow_len| {
                            let src_base = row_idx.saturating_mul(tile_metrics_stride);
                            if src_base + metrics_count > tile_host_buf.len() {
                                return Ok(());
                            }
                            let metrics =
                                decode_tm_flex_metrics_row(&tile_host_buf, src_base, metrics_count);
                            if !metrics_pass_constraints(&metrics, constraints) {
                                return Ok(());
                            }

                            let params = DoubleMaParams {
                                fast_len,
                                slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            };
                            agg.push(params, metrics.clone());

                            if let Some(w) = export_writer.as_mut() {
                                let score = match objective {
                                    ObjectiveKind::Pnl => metrics.pnl,
                                    ObjectiveKind::Sharpe => metrics.sharpe,
                                    ObjectiveKind::MaxDrawdown => -metrics.max_dd,
                                    ObjectiveKind::Calmar => metrics.calmar,
                                    ObjectiveKind::Volatility => -metrics.volatility,
                                    ObjectiveKind::Sortino | ObjectiveKind::Omega => {
                                        f64::NEG_INFINITY
                                    }
                                };
                                writeln!(
                                    w,
                                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                                    fast_ma.as_str(),
                                    slow_ma.as_str(),
                                    fast_len,
                                    slow_len,
                                    metrics.pnl,
                                    metrics.sharpe,
                                    metrics.max_dd,
                                    metrics.downside_deviation,
                                    metrics.ulcer_index,
                                    metrics.martin_ratio,
                                    metrics.trades,
                                    metrics.exposure,
                                    metrics.net_exposure,
                                    score
                                )
                                .map_err(|e| e.to_string())?;
                                exported = exported.saturating_add(1);
                                if (exported & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                                    return Err("cancelled".to_string());
                                }
                            }
                            Ok(())
                        },
                    )?;

                    processed = processed.saturating_add(tile_valid);
                    if cancel.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }
                    emit_progress(progress, processed, total_pairs, "gpu-kernel");
                    if pipeline_profile_enabled {
                        bt_stream.synchronize().map_err(|e| e.to_string())?;
                    }
                    if let (Some(t0), Some(p)) = (tile_total_t0, pipeline_profile.as_mut()) {
                        let total_elapsed = t0.elapsed();
                        let finalize_elapsed = total_elapsed
                            .saturating_sub(tile_prepare_elapsed)
                            .saturating_sub(tile_score_elapsed);
                        p.tile_prepare = p.tile_prepare.saturating_add(tile_prepare_elapsed);
                        p.tile_score = p.tile_score.saturating_add(tile_score_elapsed);
                        p.tile_backtest =
                            p.tile_backtest.saturating_add(tile_backtest_elapsed.get());
                        p.tile_heatmap = p.tile_heatmap.saturating_add(tile_heatmap_elapsed.get());
                        p.tile_finalize = p.tile_finalize.saturating_add(finalize_elapsed);
                        p.tile_count = p.tile_count.saturating_add(1);
                    }
                    s_start += ps;
                    continue;
                }

                let pair_stage_pairs = if pair_index_map_ptr != 0 && launch_pairs > 0 {
                    launch_pairs
                } else {
                    pairs
                };
                let blocks1 = (pair_stage_pairs + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
                let cand_len1 = blocks1.saturating_mul(topk_select).max(topk_select);
                let cand_a = scratch.d_cand_a.as_mut().unwrap();
                let cand_b = scratch.d_cand_b.as_mut().unwrap();

                unsafe {
                    let mut metrics_ptr = scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut pairs_i = pairs as i32;
                    let mut m_i = metrics_count as i32;
                    let mut metrics_stride_i = tile_metrics_stride as i32;
                    let mut pf_i = pf as i32;
                    let mut ps_i = ps as i32;
                    let mut f_per = scratch.d_fast_p.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut s_per = scratch.d_slow_p.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut obj_i = objective_i;
                    let mut topk_i = topk_select as i32;
                    let mut pair_map_ptr = pair_index_map_ptr;
                    let mut pair_count_i = launch_pairs as i32;
                    let mut compact_output_i = if compact_valid_pair_output {
                        1i32
                    } else {
                        0i32
                    };
                    let mut out_ptr = cand_a.as_device_ptr().as_raw();

                    let block_x: u32 = 256;
                    let grid_x: u32 = blocks1 as u32;
                    if constraints_on_device {
                        let mut use_min_trades = constraint_use_min_trades;
                        let mut min_trades = constraint_min_trades;
                        let mut use_max_dd = constraint_use_max_dd;
                        let mut max_dd = constraint_max_dd;
                        let args: &mut [*mut c_void] = &mut [
                            &mut metrics_ptr as *mut _ as *mut c_void,
                            &mut pairs_i as *mut _ as *mut c_void,
                            &mut m_i as *mut _ as *mut c_void,
                            &mut metrics_stride_i as *mut _ as *mut c_void,
                            &mut pf_i as *mut _ as *mut c_void,
                            &mut ps_i as *mut _ as *mut c_void,
                            &mut f_per as *mut _ as *mut c_void,
                            &mut s_per as *mut _ as *mut c_void,
                            &mut obj_i as *mut _ as *mut c_void,
                            &mut topk_i as *mut _ as *mut c_void,
                            &mut use_min_trades as *mut _ as *mut c_void,
                            &mut min_trades as *mut _ as *mut c_void,
                            &mut use_max_dd as *mut _ as *mut c_void,
                            &mut max_dd as *mut _ as *mut c_void,
                            &mut pair_map_ptr as *mut _ as *mut c_void,
                            &mut pair_count_i as *mut _ as *mut c_void,
                            &mut compact_output_i as *mut _ as *mut c_void,
                            &mut out_ptr as *mut _ as *mut c_void,
                        ];
                        bt_stream
                            .launch(
                                select_topk_pairs_v2.as_ref().ok_or_else(|| {
                                    "missing select_topk_from_pairs_tm_f32_v2".to_string()
                                })?,
                                (grid_x, 1, 1),
                                (block_x, 1, 1),
                                0,
                                args,
                            )
                            .map_err(|e| e.to_string())?;
                    } else {
                        let args: &mut [*mut c_void] = &mut [
                            &mut metrics_ptr as *mut _ as *mut c_void,
                            &mut pairs_i as *mut _ as *mut c_void,
                            &mut m_i as *mut _ as *mut c_void,
                            &mut metrics_stride_i as *mut _ as *mut c_void,
                            &mut pf_i as *mut _ as *mut c_void,
                            &mut ps_i as *mut _ as *mut c_void,
                            &mut f_per as *mut _ as *mut c_void,
                            &mut s_per as *mut _ as *mut c_void,
                            &mut obj_i as *mut _ as *mut c_void,
                            &mut topk_i as *mut _ as *mut c_void,
                            &mut pair_map_ptr as *mut _ as *mut c_void,
                            &mut pair_count_i as *mut _ as *mut c_void,
                            &mut compact_output_i as *mut _ as *mut c_void,
                            &mut out_ptr as *mut _ as *mut c_void,
                        ];
                        bt_stream
                            .launch(
                                &select_topk_pairs_legacy,
                                (grid_x, 1, 1),
                                (block_x, 1, 1),
                                0,
                                args,
                            )
                            .map_err(|e| e.to_string())?;
                    }
                }

                let mut blocks_prev = blocks1;
                let mut in_len = cand_len1;
                let mut in_is_a = true;
                while blocks_prev > 1 {
                    let blocks_next = (in_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
                    let out_len = blocks_next.saturating_mul(topk_select).max(topk_select);
                    unsafe {
                        let mut metrics_ptr =
                            scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut pairs_i = pairs as i32;
                        let mut m_i = metrics_count as i32;
                        let mut metrics_stride_i = tile_metrics_stride as i32;
                        let mut pf_i = pf as i32;
                        let mut ps_i = ps as i32;
                        let mut f_per = scratch.d_fast_p.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut s_per = scratch.d_slow_p.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut obj_i = objective_i;
                        let mut topk_i = topk_select as i32;
                        let mut in_ptr = if in_is_a {
                            cand_a.as_device_ptr().as_raw()
                        } else {
                            cand_b.as_device_ptr().as_raw()
                        };
                        let mut in_len_i = in_len as i32;
                        let mut out_ptr = if in_is_a {
                            cand_b.as_device_ptr().as_raw()
                        } else {
                            cand_a.as_device_ptr().as_raw()
                        };
                        let mut pair_map_ptr = pair_index_map_ptr;
                        let mut pair_count_i = launch_pairs as i32;
                        let mut compact_output_i = if compact_valid_pair_output {
                            1i32
                        } else {
                            0i32
                        };

                        let block_x: u32 = 256;
                        let grid_x: u32 = blocks_next as u32;
                        if constraints_on_device {
                            let mut use_min_trades = constraint_use_min_trades;
                            let mut min_trades = constraint_min_trades;
                            let mut use_max_dd = constraint_use_max_dd;
                            let mut max_dd = constraint_max_dd;
                            let args: &mut [*mut c_void] = &mut [
                                &mut metrics_ptr as *mut _ as *mut c_void,
                                &mut pairs_i as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pf_i as *mut _ as *mut c_void,
                                &mut ps_i as *mut _ as *mut c_void,
                                &mut f_per as *mut _ as *mut c_void,
                                &mut s_per as *mut _ as *mut c_void,
                                &mut obj_i as *mut _ as *mut c_void,
                                &mut topk_i as *mut _ as *mut c_void,
                                &mut in_ptr as *mut _ as *mut c_void,
                                &mut in_len_i as *mut _ as *mut c_void,
                                &mut use_min_trades as *mut _ as *mut c_void,
                                &mut min_trades as *mut _ as *mut c_void,
                                &mut use_max_dd as *mut _ as *mut c_void,
                                &mut max_dd as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            bt_stream
                                .launch(
                                    select_topk_candidates_v2.as_ref().ok_or_else(|| {
                                        "missing select_topk_from_candidates_tm_f32_v2".to_string()
                                    })?,
                                    (grid_x, 1, 1),
                                    (block_x, 1, 1),
                                    0,
                                    args,
                                )
                                .map_err(|e| e.to_string())?;
                        } else {
                            let args: &mut [*mut c_void] = &mut [
                                &mut metrics_ptr as *mut _ as *mut c_void,
                                &mut pairs_i as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pf_i as *mut _ as *mut c_void,
                                &mut ps_i as *mut _ as *mut c_void,
                                &mut f_per as *mut _ as *mut c_void,
                                &mut s_per as *mut _ as *mut c_void,
                                &mut obj_i as *mut _ as *mut c_void,
                                &mut topk_i as *mut _ as *mut c_void,
                                &mut in_ptr as *mut _ as *mut c_void,
                                &mut in_len_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            bt_stream
                                .launch(
                                    &select_topk_candidates_legacy,
                                    (grid_x, 1, 1),
                                    (block_x, 1, 1),
                                    0,
                                    args,
                                )
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    in_is_a = !in_is_a;
                    blocks_prev = blocks_next;
                    in_len = out_len;
                }

                unsafe {
                    let mut metrics_ptr = scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut pairs_i = pairs as i32;
                    let mut m_i = metrics_count as i32;
                    let mut metrics_stride_i = tile_metrics_stride as i32;
                    let mut pf_i = pf as i32;
                    let mut ps_i = ps as i32;
                    let mut f_per = scratch.d_fast_p.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut s_per = scratch.d_slow_p.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut top_ptr = if in_is_a {
                        cand_a.as_device_ptr().as_raw()
                    } else {
                        cand_b.as_device_ptr().as_raw()
                    };
                    let mut topk_i = topk_select as i32;
                    let mut pair_map_ptr = pair_index_map_ptr;
                    let mut pair_count_i = launch_pairs as i32;
                    let mut compact_output_i = if compact_valid_pair_output {
                        1i32
                    } else {
                        0i32
                    };
                    let mut out_f = scratch
                        .d_top_fast
                        .as_ref()
                        .unwrap()
                        .as_device_ptr()
                        .as_raw();
                    let mut out_s = scratch
                        .d_top_slow
                        .as_ref()
                        .unwrap()
                        .as_device_ptr()
                        .as_raw();
                    let mut out_m = scratch
                        .d_top_metrics
                        .as_ref()
                        .unwrap()
                        .as_device_ptr()
                        .as_raw();
                    let mut pair_fast_rows = pair_fast_rows_ptr;
                    let mut pair_slow_rows = pair_slow_rows_ptr;

                    let block_x: u32 = 256;
                    let grid_x: u32 = ((topk_select as u32) + block_x - 1) / block_x;
                    let args: &mut [*mut c_void] = &mut [
                        &mut metrics_ptr as *mut _ as *mut c_void,
                        &mut pairs_i as *mut _ as *mut c_void,
                        &mut m_i as *mut _ as *mut c_void,
                        &mut metrics_stride_i as *mut _ as *mut c_void,
                        &mut pf_i as *mut _ as *mut c_void,
                        &mut ps_i as *mut _ as *mut c_void,
                        &mut f_per as *mut _ as *mut c_void,
                        &mut s_per as *mut _ as *mut c_void,
                        &mut top_ptr as *mut _ as *mut c_void,
                        &mut topk_i as *mut _ as *mut c_void,
                        &mut pair_map_ptr as *mut _ as *mut c_void,
                        &mut pair_fast_rows as *mut _ as *mut c_void,
                        &mut pair_slow_rows as *mut _ as *mut c_void,
                        &mut pair_count_i as *mut _ as *mut c_void,
                        &mut compact_output_i as *mut _ as *mut c_void,
                        &mut out_f as *mut _ as *mut c_void,
                        &mut out_s as *mut _ as *mut c_void,
                        &mut out_m as *mut _ as *mut c_void,
                    ];
                    bt_stream
                        .launch(&gather_topk, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                        .map_err(|e| e.to_string())?;
                }

                if global_topk_mode {
                    let global_fast = global_fast_rows
                        .as_ref()
                        .ok_or_else(|| "missing global fast-row buffer".to_string())?;
                    let global_slow = global_slow_rows
                        .as_ref()
                        .ok_or_else(|| "missing global slow-row buffer".to_string())?;
                    let global_metrics = global_metrics_rows
                        .as_ref()
                        .ok_or_else(|| "missing global metrics-row buffer".to_string())?;
                    let row_i32_bytes = topk_select
                        .checked_mul(std::mem::size_of::<i32>())
                        .ok_or_else(|| "topk row bytes overflow (i32)".to_string())?;
                    let row_metrics_bytes = topk_select
                        .checked_mul(metrics_count)
                        .and_then(|v| v.checked_mul(std::mem::size_of::<f32>()))
                        .ok_or_else(|| "topk row bytes overflow (metrics)".to_string())?;

                    if global_rows_len == 0 {
                        cu_memcpy_d2d_async(
                            global_fast.as_device_ptr().as_raw(),
                            scratch
                                .d_top_fast
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_i32_bytes,
                            bt_stream,
                        )?;
                        cu_memcpy_d2d_async(
                            global_slow.as_device_ptr().as_raw(),
                            scratch
                                .d_top_slow
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_i32_bytes,
                            bt_stream,
                        )?;
                        cu_memcpy_d2d_async(
                            global_metrics.as_device_ptr().as_raw(),
                            scratch
                                .d_top_metrics
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_metrics_bytes,
                            bt_stream,
                        )?;
                        global_rows_len = topk_select;
                    } else {
                        if global_rows_len.saturating_add(topk_select) > global_rows_cap {
                            return Err(
                                "global top-k merge buffer overflow; expected capacity for 2 * top_k rows"
                                    .to_string(),
                            );
                        }
                        let dst_fast = global_fast
                            .as_device_ptr()
                            .as_raw()
                            .saturating_add((global_rows_len * std::mem::size_of::<i32>()) as u64);
                        let dst_slow = global_slow
                            .as_device_ptr()
                            .as_raw()
                            .saturating_add((global_rows_len * std::mem::size_of::<i32>()) as u64);
                        let dst_metrics = global_metrics.as_device_ptr().as_raw().saturating_add(
                            (global_rows_len
                                .saturating_mul(metrics_count)
                                .saturating_mul(std::mem::size_of::<f32>()))
                                as u64,
                        );
                        cu_memcpy_d2d_async(
                            dst_fast,
                            scratch
                                .d_top_fast
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_i32_bytes,
                            bt_stream,
                        )?;
                        cu_memcpy_d2d_async(
                            dst_slow,
                            scratch
                                .d_top_slow
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_i32_bytes,
                            bt_stream,
                        )?;
                        cu_memcpy_d2d_async(
                            dst_metrics,
                            scratch
                                .d_top_metrics
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_metrics_bytes,
                            bt_stream,
                        )?;
                        global_rows_len = global_rows_len.saturating_add(topk_select);

                        reduce_metric_rows_topk_on_device(
                            bt_stream,
                            scratch,
                            global_fast,
                            global_slow,
                            global_metrics,
                            global_rows_len,
                            metrics_count,
                            objective_i,
                            topk_select,
                            global_constraints_on_device,
                            constraint_use_min_trades,
                            constraint_min_trades,
                            constraint_use_max_dd,
                            constraint_max_dd,
                            select_topk_rows_legacy
                                .as_ref()
                                .ok_or_else(|| "missing select_topk_metric_rows_f32".to_string())?,
                            select_topk_rows_candidates_legacy.as_ref().ok_or_else(|| {
                                "missing select_topk_metric_rows_from_candidates_f32".to_string()
                            })?,
                            select_topk_rows_v2.as_ref(),
                            select_topk_rows_candidates_v2.as_ref(),
                            gather_topk_rows
                                .as_ref()
                                .ok_or_else(|| "missing gather_topk_metric_rows_f32".to_string())?,
                        )?;
                        cu_memcpy_d2d_async(
                            global_fast.as_device_ptr().as_raw(),
                            scratch
                                .d_top_fast
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_i32_bytes,
                            bt_stream,
                        )?;
                        cu_memcpy_d2d_async(
                            global_slow.as_device_ptr().as_raw(),
                            scratch
                                .d_top_slow
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_i32_bytes,
                            bt_stream,
                        )?;
                        cu_memcpy_d2d_async(
                            global_metrics.as_device_ptr().as_raw(),
                            scratch
                                .d_top_metrics
                                .as_ref()
                                .unwrap()
                                .as_device_ptr()
                                .as_raw(),
                            row_metrics_bytes,
                            bt_stream,
                        )?;
                        global_rows_len = topk_select;
                    }
                } else {
                    bt_stream.synchronize().map_err(|e| e.to_string())?;

                    if let Some(w) = export_writer.as_mut() {
                        copy_device_prefix_to_host_f32(
                            scratch.d_tile.as_ref().unwrap(),
                            &mut tile_host_buf,
                            tile_row_count.saturating_mul(tile_metrics_stride),
                            bt_stream,
                        )?;
                        bt_stream.synchronize().map_err(|e| e.to_string())?;

                        let mut exported = 0usize;
                        for_each_tile_metric_row(
                            fast_chunk,
                            slow_chunk,
                            tile_rows_compact,
                            |row_idx, fast_len, slow_len| {
                                let src_base = row_idx.saturating_mul(tile_metrics_stride);
                                if src_base + metrics_count > tile_host_buf.len() {
                                    return Ok(());
                                }
                                let metrics = decode_tm_flex_metrics_row(
                                    &tile_host_buf,
                                    src_base,
                                    metrics_count,
                                );

                                let score = match objective {
                                    ObjectiveKind::Pnl => metrics.pnl,
                                    ObjectiveKind::Sharpe => metrics.sharpe,
                                    ObjectiveKind::MaxDrawdown => -metrics.max_dd,
                                    ObjectiveKind::Calmar => metrics.calmar,
                                    ObjectiveKind::Volatility => -metrics.volatility,
                                    ObjectiveKind::Sortino | ObjectiveKind::Omega => {
                                        f64::NEG_INFINITY
                                    }
                                };

                                writeln!(
                                    w,
                                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                                    fast_ma.as_str(),
                                    slow_ma.as_str(),
                                    fast_len,
                                    slow_len,
                                    metrics.pnl,
                                    metrics.sharpe,
                                    metrics.max_dd,
                                    metrics.downside_deviation,
                                    metrics.ulcer_index,
                                    metrics.martin_ratio,
                                    metrics.trades,
                                    metrics.exposure,
                                    metrics.net_exposure,
                                    score
                                )
                                .map_err(|e| e.to_string())?;

                                exported = exported.saturating_add(1);
                                if (exported & 0x1fff) == 0 && cancel.load(Ordering::Relaxed) {
                                    return Err("cancelled".to_string());
                                }
                                Ok(())
                            },
                        )?;
                    }

                    pack_metric_rows_for_host_copy(
                        bt_stream,
                        scratch,
                        &pack_metric_rows,
                        scratch
                            .d_top_fast
                            .as_ref()
                            .unwrap()
                            .as_device_ptr()
                            .as_raw(),
                        scratch
                            .d_top_slow
                            .as_ref()
                            .unwrap()
                            .as_device_ptr()
                            .as_raw(),
                        scratch
                            .d_top_metrics
                            .as_ref()
                            .unwrap()
                            .as_device_ptr()
                            .as_raw(),
                        topk_select,
                        metrics_count,
                        &mut top_packed_host,
                    )?;

                    for r in 0..topk_select {
                        let Some((fast_len, slow_len, metrics)) =
                            decode_packed_topk_metric_row(&top_packed_host, r, metrics_count)
                        else {
                            continue;
                        };
                        if metrics_pass_constraints(&metrics, constraints) {
                            let params = DoubleMaParams {
                                fast_len,
                                slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            };
                            agg.push(params, metrics);
                        }
                    }
                }

                processed = processed.saturating_add(tile_valid);
                if cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".to_string());
                }
            }

            emit_progress(progress, processed, total_pairs, "gpu-kernel");
            if pipeline_profile_enabled {
                bt_stream.synchronize().map_err(|e| e.to_string())?;
            }
            if let (Some(t0), Some(p)) = (tile_total_t0, pipeline_profile.as_mut()) {
                let total_elapsed = t0.elapsed();
                let finalize_elapsed = total_elapsed
                    .saturating_sub(tile_prepare_elapsed)
                    .saturating_sub(tile_score_elapsed);
                p.tile_prepare = p.tile_prepare.saturating_add(tile_prepare_elapsed);
                p.tile_score = p.tile_score.saturating_add(tile_score_elapsed);
                p.tile_backtest = p.tile_backtest.saturating_add(tile_backtest_elapsed.get());
                p.tile_heatmap = p.tile_heatmap.saturating_add(tile_heatmap_elapsed.get());
                p.tile_finalize = p.tile_finalize.saturating_add(finalize_elapsed);
                p.tile_count = p.tile_count.saturating_add(1);
            }

            s_start += ps;
        }

        f_start += pf;
    }

    let final_finalize_t0 = if pipeline_profile_enabled {
        Some(Instant::now())
    } else {
        None
    };

    if global_topk_mode && !include_all && global_rows_len > 0 {
        let global_fast = global_fast_rows
            .as_ref()
            .ok_or_else(|| "missing global fast-row buffer".to_string())?;
        let global_slow = global_slow_rows
            .as_ref()
            .ok_or_else(|| "missing global slow-row buffer".to_string())?;
        let global_metrics = global_metrics_rows
            .as_ref()
            .ok_or_else(|| "missing global metrics-row buffer".to_string())?;
        pack_metric_rows_for_host_copy(
            bt_stream,
            scratch,
            &pack_metric_rows,
            global_fast.as_device_ptr().as_raw(),
            global_slow.as_device_ptr().as_raw(),
            global_metrics.as_device_ptr().as_raw(),
            topk_select,
            metrics_count,
            &mut top_packed_host,
        )?;

        for r in 0..topk_select {
            let Some((fast_len, slow_len, metrics)) =
                decode_packed_topk_metric_row(&top_packed_host, r, metrics_count)
            else {
                continue;
            };
            if metrics_pass_constraints(&metrics, constraints) {
                let params = DoubleMaParams {
                    fast_len,
                    slow_len,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                };
                agg.push(params, metrics);
            }
        }
    }

    let heatmap_out: Option<OptimizationHeatmap> = if !include_all {
        if let (Some(h), Some((f_min, f_max, s_min, s_max))) =
            (scratch.d_heatmap.as_ref(), heatmap_cfg)
        {
            let heatmap_len = heatmap_bins.saturating_mul(heatmap_bins);
            if heatmap_len == 0 {
                None
            } else {
                let mut host: Vec<f32> = alloc_uninit_copy(heatmap_len);
                h.copy_to(&mut host).map_err(|e| e.to_string())?;
                let values: Vec<Option<f64>> = host
                    .into_iter()
                    .map(|v| {
                        if v.is_finite() && v != f32::NEG_INFINITY {
                            Some(v as f64)
                        } else {
                            None
                        }
                    })
                    .collect();
                Some(OptimizationHeatmap {
                    bins_fast: heatmap_bins,
                    bins_slow: heatmap_bins,
                    fast_min: f_min,
                    fast_max: f_max,
                    slow_min: s_min,
                    slow_max: s_max,
                    values,
                })
            }
        } else {
            None
        }
    } else {
        None
    };

    let mut result = agg
        .finalize()
        .ok_or_else(|| "grid search produced no result".to_string())?;

    if !include_all
        && metrics_count < METRICS_COUNT
        && !fast_precomputed_enabled
        && !slow_precomputed_enabled
        && kernel_finalist_recompute_enabled()
    {
        recompute_optimization_result_finalists_with_full_gpu_pass(
            rt,
            candles,
            &mut result,
            fast_periods,
            slow_periods,
            fast_ma.as_str(),
            slow_ma.as_str(),
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter,
            cfg,
            strategy,
        )?;
    }

    if !include_all {
        result.num_combos = total_pairs;
        if heatmap_out.is_some() {
            result.heatmap = heatmap_out;
        }
    }

    if let Some(w) = export_writer.as_mut() {
        w.flush().map_err(|e| e.to_string())?;
    }

    if !kernel_persist_buffers_enabled() {
        rt.ma.clear_cached_constants();
        rt.scratch.clear_async(&rt.stream)?;
    }

    if let (Some(t0), Some(p)) = (final_finalize_t0, pipeline_profile.as_mut()) {
        p.final_finalize = p.final_finalize.saturating_add(t0.elapsed());
        p.emit(
            fast_ma.as_str(),
            slow_ma.as_str(),
            include_all,
            topk_select,
            heatmap_bins,
        );
    }

    Ok((
        result,
        KernelExecutorTimingMs {
            gpu_indicator_prep_ms: gpu_indicator_prep_elapsed.as_millis() as u64,
            h2d_upload_ms: h2d_upload_elapsed.as_millis() as u64,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        decode_tm_flex_metrics_row, eval_double_ma_dynamic_program_precomputed_gpu_kernel,
        kernel_async_alloc_enabled, kernel_effective_pair_bars, kernel_pinned_h2d_enabled,
        kernel_should_use_lockstep_time, kernel_should_use_m7_specialize,
        max_drawdown_constraint_to_search_space, search_metrics_count_for_optimization,
        should_force_low_pair_m7_recompute, should_use_compact_valid_pair_output,
        upload_double_ma_dynamic_program_batch, METRICS_COUNT,
    };
    use crate::double_ma::DoubleMaConstraints;
    use crate::strategy_dynamic::{
        build_dynamic_cpu_context_from_precomputed, compile_dynamic_plan_for_double_ma_config,
        eval_dynamic_plan_metrics_cpu,
    };
    use crate::strategy_gpu_runtime::{
        reset_gpu_strategy_executable_cache, StrategyCapabilityReport, StrategyHandle,
    };
    use crate::strategy_plan::StrategyPlanBackend;
    use ta_optimizer::ObjectiveKind;
    use ta_strategies::double_ma::{DoubleMaParams, StrategyConfig};
    use vector_ta::utilities::data_loader::{source_type, Candles};

    struct TestEnvGuard {
        _env: std::sync::MutexGuard<'static, ()>,
        _gpu: super::GpuTestLockGuard,
    }

    fn lock_env() -> TestEnvGuard {
        let env = super::lock_test_env();
        let gpu = super::lock_gpu_test();
        super::reset_test_runtime();
        crate::dynamic_indicator_adapter::reset_test_cuda_input_cache();
        TestEnvGuard {
            _env: env,
            _gpu: gpu,
        }
    }

    fn make_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64 * 0.05;
            open.push(base);
            high.push(base + 0.75);
            low.push(base - 0.75);
            close.push(base + ((i % 11) as f64 - 5.0) * 0.01);
            vol.push(10_000.0 + ((i % 17) as f64) * 10.0);
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn kernel_h2d_flag_defaults_to_enabled() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_PINNED_H2D").ok();
        std::env::remove_var("VECTORBT_KERNEL_PINNED_H2D");
        assert!(kernel_pinned_h2d_enabled());
        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PINNED_H2D", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PINNED_H2D"),
        }
    }

    #[test]
    fn kernel_h2d_flag_respects_false_tokens() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_PINNED_H2D").ok();

        std::env::set_var("VECTORBT_KERNEL_PINNED_H2D", "0");
        assert!(!kernel_pinned_h2d_enabled());

        std::env::set_var("VECTORBT_KERNEL_PINNED_H2D", "false");
        assert!(!kernel_pinned_h2d_enabled());

        std::env::set_var("VECTORBT_KERNEL_PINNED_H2D", "1");
        assert!(kernel_pinned_h2d_enabled());

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PINNED_H2D", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PINNED_H2D"),
        }
    }

    #[test]
    fn kernel_async_alloc_flag_defaults_to_enabled_and_parses_false() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_ASYNC_ALLOC").ok();

        std::env::remove_var("VECTORBT_KERNEL_ASYNC_ALLOC");
        assert!(kernel_async_alloc_enabled());

        std::env::set_var("VECTORBT_KERNEL_ASYNC_ALLOC", "0");
        assert!(!kernel_async_alloc_enabled());

        std::env::set_var("VECTORBT_KERNEL_ASYNC_ALLOC", "false");
        assert!(!kernel_async_alloc_enabled());

        std::env::set_var("VECTORBT_KERNEL_ASYNC_ALLOC", "1");
        assert!(kernel_async_alloc_enabled());

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_ASYNC_ALLOC", v),
            None => std::env::remove_var("VECTORBT_KERNEL_ASYNC_ALLOC"),
        }
    }

    #[test]
    fn kernel_effective_pair_bars_uses_valid_tail() {
        assert_eq!(
            kernel_effective_pair_bars(58_300, 200_000, 400),
            11_636_680_000
        );
        assert_eq!(kernel_effective_pair_bars(10, 100, 150), 10);
    }

    #[test]
    fn search_metrics_count_uses_minimal_search_payload_when_allowed() {
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Pnl,
                None,
                false,
                false,
                false,
                false,
            ),
            1
        );
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Calmar,
                None,
                false,
                false,
                false,
                false,
            ),
            3
        );
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Sharpe,
                None,
                false,
                false,
                false,
                false,
            ),
            5
        );
    }

    #[test]
    fn search_metrics_count_falls_back_to_full_metrics_when_required() {
        let constraints = super::DoubleMaConstraints {
            min_trades: Some(10),
            max_drawdown: Some(0.2),
        };
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Pnl,
                Some(&constraints),
                false,
                false,
                false,
                false,
            ),
            3
        );
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Pnl,
                None,
                true,
                false,
                false,
                false,
            ),
            super::METRICS_COUNT
        );
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Pnl,
                None,
                false,
                true,
                false,
                false,
            ),
            super::METRICS_COUNT
        );
    }

    #[test]
    fn compact_valid_pair_output_is_enabled_for_all_host_contracts() {
        assert!(super::should_use_compact_valid_pair_output(
            false, false, false, 5
        ));
        assert!(super::should_use_compact_valid_pair_output(
            true, false, false, 5
        ));
        assert!(super::should_use_compact_valid_pair_output(
            false, true, false, 5
        ));
        assert!(super::should_use_compact_valid_pair_output(
            false, false, true, 5
        ));
        assert!(super::should_use_compact_valid_pair_output(
            false,
            false,
            false,
            super::METRICS_COUNT
        ));
    }

    #[test]
    fn kernel_m7_gate_uses_pair_bars_for_large_long_workloads() {
        let _guard = lock_env();
        let prev_pairs = std::env::var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIRS").ok();
        let prev_pair_bars = std::env::var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIR_BARS").ok();
        let prev_enabled = std::env::var("VECTORBT_KERNEL_M7_SPECIALIZE").ok();

        std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE", "1");
        std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIRS", "300000");
        std::env::remove_var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIR_BARS");

        assert!(kernel_should_use_m7_specialize(
            58_300, 200_000, 400, false, 7, true
        ));
        assert!(!kernel_should_use_m7_specialize(
            14_150, 200_000, 400, false, 7, true
        ));
        assert!(!kernel_should_use_m7_specialize(
            58_300, 200_000, 400, true, 7, true
        ));
        assert!(!kernel_should_use_m7_specialize(
            58_300, 200_000, 400, false, 5, true
        ));

        match prev_pairs {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIRS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIRS"),
        }
        match prev_pair_bars {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIR_BARS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIR_BARS"),
        }
        match prev_enabled {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE", v),
            None => std::env::remove_var("VECTORBT_KERNEL_M7_SPECIALIZE"),
        }
    }

    #[test]
    fn low_pair_recompute_m7_force_respects_global_enable_flag() {
        let _guard = lock_env();
        let prev_enabled = std::env::var("VECTORBT_KERNEL_M7_SPECIALIZE").ok();

        std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE", "0");
        assert!(!should_force_low_pair_m7_recompute(
            true, false, false, 50, true
        ));

        std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE", "1");
        assert!(should_force_low_pair_m7_recompute(
            true, false, false, 50, true
        ));

        match prev_enabled {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE", v),
            None => std::env::remove_var("VECTORBT_KERNEL_M7_SPECIALIZE"),
        }
    }

    #[test]
    fn low_pair_recompute_m7_force_requires_small_single_tile_workload() {
        let _guard = lock_env();
        let prev_enabled = std::env::var("VECTORBT_KERNEL_M7_SPECIALIZE").ok();
        std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE", "1");

        assert!(!should_force_low_pair_m7_recompute(
            false, false, false, 50, true
        ));
        assert!(!should_force_low_pair_m7_recompute(
            true, true, false, 50, true
        ));
        assert!(!should_force_low_pair_m7_recompute(
            true, false, true, 50, true
        ));
        assert!(!should_force_low_pair_m7_recompute(
            true, false, false, 0, true
        ));
        assert!(!should_force_low_pair_m7_recompute(
            true, false, false, 129, true
        ));
        assert!(!should_force_low_pair_m7_recompute(
            true, false, false, 50, false
        ));

        match prev_enabled {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_M7_SPECIALIZE", v),
            None => std::env::remove_var("VECTORBT_KERNEL_M7_SPECIALIZE"),
        }
    }

    #[test]
    fn kernel_lockstep_gate_uses_pair_bars_for_large_long_workloads() {
        let _guard = lock_env();
        let prev_pairs = std::env::var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIRS").ok();
        let prev_pair_bars = std::env::var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIR_BARS").ok();
        let prev_enabled = std::env::var("VECTORBT_KERNEL_LOCKSTEP_TIME").ok();

        std::env::set_var("VECTORBT_KERNEL_LOCKSTEP_TIME", "1");
        std::env::set_var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIRS", "150000");
        std::env::remove_var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIR_BARS");

        assert!(kernel_should_use_lockstep_time(58_300, 200_000, 400, true));
        assert!(!kernel_should_use_lockstep_time(14_150, 200_000, 400, true));
        assert!(!kernel_should_use_lockstep_time(
            58_300, 200_000, 400, false
        ));

        std::env::set_var("VECTORBT_KERNEL_LOCKSTEP_TIME", "0");
        assert!(!kernel_should_use_lockstep_time(58_300, 200_000, 400, true));

        match prev_pairs {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIRS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIRS"),
        }
        match prev_pair_bars {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIR_BARS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIR_BARS"),
        }
        match prev_enabled {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_LOCKSTEP_TIME", v),
            None => std::env::remove_var("VECTORBT_KERNEL_LOCKSTEP_TIME"),
        }
    }

    #[test]
    fn search_metrics_count_tracks_objective_and_constraints() {
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                false,
                false,
                false,
                false,
            ),
            1
        );
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Sharpe,
                None,
                false,
                false,
                false,
                false,
            ),
            5
        );
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Volatility,
                None,
                false,
                false,
                false,
                false,
            ),
            5
        );
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::MaxDrawdown,
                None,
                false,
                false,
                false,
                false,
            ),
            3
        );
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Calmar,
                None,
                false,
                false,
                false,
                false,
            ),
            3
        );

        let min_trades = DoubleMaConstraints {
            min_trades: Some(20),
            max_drawdown: None,
        };
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                Some(&min_trades),
                false,
                false,
                false,
                false,
            ),
            2
        );

        let max_dd = DoubleMaConstraints {
            min_trades: None,
            max_drawdown: Some(0.25),
        };
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                Some(&max_dd),
                false,
                false,
                false,
                false,
            ),
            3
        );

        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                true,
                false,
                false,
                false,
            ),
            METRICS_COUNT
        );
    }

    #[test]
    fn compact_valid_pair_output_stays_enabled_with_full_metrics_rows() {
        assert!(should_use_compact_valid_pair_output(false, false, false, 3));
        assert!(should_use_compact_valid_pair_output(true, false, false, 3));
        assert!(should_use_compact_valid_pair_output(false, true, false, 3));
        assert!(should_use_compact_valid_pair_output(false, false, true, 3));
        assert!(should_use_compact_valid_pair_output(
            false,
            false,
            false,
            METRICS_COUNT
        ));
    }

    #[test]
    fn decode_reduced_search_rows_restores_arithmetic_metrics() {
        let pnl = 0.25f64;
        let max_dd = 0.40f64;
        let row = vec![
            pnl.ln_1p() as f32,
            12.0,
            (-(1.0 - max_dd).ln()) as f32,
            0.0,
            0.0,
        ];

        let metrics = decode_tm_flex_metrics_row(&row, 0, 5);
        assert!((metrics.pnl - pnl).abs() < 1e-6);
        assert_eq!(metrics.trades, 12);
        assert!((metrics.max_dd - max_dd).abs() < 1e-6);
    }

    #[test]
    fn decode_full_metric_rows_restores_sortino_from_mean_and_downside_deviation() {
        let row = vec![
            0.15f32, // pnl
            7.0f32,  // trades
            0.20f32, // max_dd
            0.06f32, // mean
            0.03f32, // std
            0.40f32, // exposure
            0.10f32, // net_exposure
            0.02f32, // downside_deviation
            0.05f32, // ulcer_index
            3.0f32,  // martin_ratio
        ];

        let metrics = decode_tm_flex_metrics_row(&row, 0, METRICS_COUNT);
        assert!((metrics.sortino - 3.0).abs() < 1e-6);
        assert_eq!(metrics.omega, 0.0);
    }

    #[test]
    fn max_drawdown_constraint_converts_for_reduced_search_rows() {
        let raw = max_drawdown_constraint_to_search_space(0.25, METRICS_COUNT);
        assert!((raw - 0.25).abs() < 1e-6);

        let search = max_drawdown_constraint_to_search_space(0.25, 3);
        let expected = (-(1.0f64 - 0.25).ln()) as f32;
        assert!((search - expected).abs() < 1e-6);

        assert!(max_drawdown_constraint_to_search_space(1.0, 3).is_infinite());
    }

    #[test]
    fn dynamic_double_ma_program_gpu_kernel_matches_cpu_plain_sma_metrics() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        reset_gpu_strategy_executable_cache();

        let candles = make_trending_candles(4096);
        let strategy = StrategyConfig::default();
        let plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "sma",
            None,
            None,
            &strategy,
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile dynamic double-ma plan");
        let handle = StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
            .expect("build dynamic gpu strategy handle");

        let fast_periods = vec![10u16];
        let slow_periods = vec![30u16];
        let combos = vec![DoubleMaParams {
            fast_len: 10,
            slow_len: 30,
            fast_ma_id: 0,
            slow_ma_id: 0,
        }];

        let close = source_type(&candles, "close");
        let lr = crate::double_ma::compute_log_returns_f64(close);
        let (fast_mat, cols) = crate::double_ma::ma_matrix_cpu_f64(
            &candles,
            "sma",
            "close",
            &fast_periods,
            None,
            0.85,
            6.0,
        )
        .expect("fast matrix");
        let (slow_mat, cols2) = crate::double_ma::ma_matrix_cpu_f64(
            &candles,
            "sma",
            "close",
            &slow_periods,
            None,
            0.85,
            6.0,
        )
        .expect("slow matrix");
        assert_eq!(cols, cols2);
        let lr_f32: Vec<f32> = lr.iter().map(|&v| v as f32).collect();
        let fast_f32: Vec<f32> = fast_mat.iter().map(|&v| v as f32).collect();
        let slow_f32: Vec<f32> = slow_mat.iter().map(|&v| v as f32).collect();

        let gpu_metrics = eval_double_ma_dynamic_program_precomputed_gpu_kernel(
            &lr_f32,
            &fast_f32,
            &slow_f32,
            cols,
            &fast_periods,
            &slow_periods,
            &combos,
            0,
            &strategy,
            &handle,
            0,
        )
        .expect("run gpu dynamic backtest");
        assert_eq!(gpu_metrics.len(), 1);

        let ctx = build_dynamic_cpu_context_from_precomputed(
            &lr, close, close, None, None, None, None, None,
        );
        let start_t = strategy
            .trade_on_next_bar
            .then_some(1usize)
            .unwrap_or(0usize)
            .saturating_add((30usize).saturating_sub(1));
        let cpu_metrics = eval_dynamic_plan_metrics_cpu(
            &handle.plan,
            &ctx,
            &fast_mat[0..cols],
            &slow_mat[0..cols],
            start_t,
            true,
        )
        .expect("run cpu dynamic backtest");

        let gpu = &gpu_metrics[0];
        assert_eq!(gpu.trades, cpu_metrics.trades);
        assert!(
            approx_eq(gpu.pnl, cpu_metrics.pnl, 1e-5),
            "pnl gpu={} cpu={}",
            gpu.pnl,
            cpu_metrics.pnl
        );
        assert!(
            approx_eq(gpu.sharpe, cpu_metrics.sharpe, 1e-5),
            "sharpe gpu={} cpu={}",
            gpu.sharpe,
            cpu_metrics.sharpe
        );
        assert!(
            approx_eq(gpu.downside_deviation, cpu_metrics.downside_deviation, 1e-5),
            "downside gpu={} cpu={}",
            gpu.downside_deviation,
            cpu_metrics.downside_deviation
        );
        assert!(
            approx_eq(gpu.calmar, cpu_metrics.calmar, 1e-2),
            "calmar gpu={} cpu={}",
            gpu.calmar,
            cpu_metrics.calmar
        );
        assert!(
            approx_eq(gpu.ulcer_index, cpu_metrics.ulcer_index, 1e-5),
            "ulcer gpu={} cpu={}",
            gpu.ulcer_index,
            cpu_metrics.ulcer_index
        );
        assert!(
            approx_eq(gpu.martin_ratio, cpu_metrics.martin_ratio, 1e-1),
            "martin gpu={} cpu={}",
            gpu.martin_ratio,
            cpu_metrics.martin_ratio
        );
        assert!(
            approx_eq(gpu.volatility, cpu_metrics.volatility, 1e-5),
            "vol gpu={} cpu={}",
            gpu.volatility,
            cpu_metrics.volatility
        );
        assert!(
            approx_eq(gpu.max_dd, cpu_metrics.max_dd, 1e-5),
            "max_dd gpu={} cpu={}",
            gpu.max_dd,
            cpu_metrics.max_dd
        );
        assert!(
            approx_eq(gpu.exposure, cpu_metrics.exposure, 1e-5),
            "exposure gpu={} cpu={}",
            gpu.exposure,
            cpu_metrics.exposure
        );
        assert!(
            approx_eq(gpu.net_exposure, cpu_metrics.net_exposure, 1e-5),
            "net_exposure gpu={} cpu={}",
            gpu.net_exposure,
            cpu_metrics.net_exposure
        );
    }

    #[test]
    fn uploaded_dynamic_double_ma_batch_relaunches_consistently() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        reset_gpu_strategy_executable_cache();

        let candles = make_trending_candles(2048);
        let strategy = StrategyConfig::default();
        let plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "sma",
            None,
            None,
            &strategy,
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile dynamic double-ma plan");
        let handle = StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
            .expect("build dynamic gpu strategy handle");

        let fast_periods = vec![8u16, 12u16];
        let slow_periods = vec![21u16, 34u16];
        let combos = vec![
            DoubleMaParams {
                fast_len: 8,
                slow_len: 21,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            DoubleMaParams {
                fast_len: 12,
                slow_len: 34,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
        ];

        let close = source_type(&candles, "close");
        let lr = crate::double_ma::compute_log_returns_f64(close);
        let (fast_mat, cols) = crate::double_ma::ma_matrix_cpu_f64(
            &candles,
            "sma",
            "close",
            &fast_periods,
            None,
            0.85,
            6.0,
        )
        .expect("fast matrix");
        let (slow_mat, _) = crate::double_ma::ma_matrix_cpu_f64(
            &candles,
            "sma",
            "close",
            &slow_periods,
            None,
            0.85,
            6.0,
        )
        .expect("slow matrix");

        let uploaded = upload_double_ma_dynamic_program_batch(
            &lr.iter().map(|&v| v as f32).collect::<Vec<_>>(),
            &fast_mat.iter().map(|&v| v as f32).collect::<Vec<_>>(),
            &slow_mat.iter().map(|&v| v as f32).collect::<Vec<_>>(),
            cols,
            &fast_periods,
            &slow_periods,
            &combos,
            0,
            &strategy,
            0,
        )
        .expect("upload dynamic batch");

        let first = super::eval_uploaded_double_ma_dynamic_program_gpu_kernel(
            &uploaded, &strategy, &handle,
        )
        .expect("first launch");
        let second = super::eval_uploaded_double_ma_dynamic_program_gpu_kernel(
            &uploaded, &strategy, &handle,
        )
        .expect("second launch");
        assert_eq!(first.len(), second.len());
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.trades, b.trades);
            assert!(approx_eq(a.pnl, b.pnl, 1e-7));
            assert!(approx_eq(a.sharpe, b.sharpe, 1e-7));
        }
    }
}
