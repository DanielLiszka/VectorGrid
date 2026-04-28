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
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::c_void;
use std::fs::File;
use std::hash::{Hash, Hasher};
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

use crate::alloc_policy::{
    alloc_uninit_copy, alloc_uninit_f32, resize_uninit_f32, resize_uninit_i32,
};
use crate::double_ma::{
    derived_active_downside_deviation, derived_active_mean, derived_active_sortino,
    derived_directional_bias, derived_downside_share, derived_drawdown_chronicity,
    derived_long_exposure, derived_mean, derived_short_exposure, effective_sample_lcb_10_score,
    effective_score, exact_fee_stress_pnl, metrics_pass_constraints, objective_score,
    score_mode_metrics_count_floor, score_mode_needs_pair_observation_context, score_mode_score_fn,
    DoubleMaConstraints, DoubleMaMetricContract, DoubleMaScoreMode,
    DoubleMaShortlistConfidenceMode, DoubleMaShortlistSliceMode, DoubleMaShortlistUnionMode,
    PairObservationScoreContextResolved, ShortlistSliceBenchmarkRelativeSummary,
    ShortlistSliceCandidateDiagnostics, ShortlistSliceDiagnostics, ShortlistSliceModeDiagnostics,
    ShortlistSliceSnapshot,
};
use crate::progress::ProgressSink;
use crate::strategy_gpu_program::{
    CompiledGpuPredicateProgramAbi, GpuConditionAbi, GpuConditionGroupAbi, GpuIndicatorSlotAbi,
    GpuSignalBindingsAbi,
};
use crate::strategy_gpu_runtime::{
    get_or_build_gpu_strategy_executable, with_uploaded_gpu_strategy_executable,
    GpuStrategyExecutableCacheKey, StrategyHandle,
};
use crate::validation_shared::SpaBenchmarkKind;
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

const PRODUCT_REDUCED_METRICS_COUNT: usize = 10;
const METRICS_COUNT: usize = 17;
const TRADE_METRICS_COUNT: usize = 22;
const EXTRA_SHORTLIST_MAX_HEADS: usize = 3;
const ROW_SCORE_OBJECTIVE_BASE: i32 = 100;
const EXTRA_SHORTLIST_REQUEST_FIELD: &str = "extra_shortlist_objectives";
const SHORTLIST_CONFIDENCE_REQUEST_FIELD: &str = "shortlist_confidence_mode";
const SHORTLIST_UNION_REQUEST_FIELD: &str = "shortlist_union_mode";
const SHORTLIST_SLICE_MODE_REQUEST_FIELD: &str = "shortlist_slice_mode";
const SHORTLIST_SLICE_BENCHMARK_REQUEST_FIELD: &str = "shortlist_slice_benchmark";
const METRIC_IDX_PNL: usize = 0;
const METRIC_IDX_TRADES: usize = 1;
const METRIC_IDX_MAX_DD: usize = 2;
const METRIC_IDX_MEAN: usize = 3;
const METRIC_IDX_STD: usize = 4;
const METRIC_IDX_EXPOSURE: usize = 5;
const METRIC_IDX_NET_EXPOSURE: usize = 6;
const METRIC_IDX_DOWNSIDE_DEVIATION: usize = 7;
const METRIC_IDX_ULCER_INDEX: usize = 8;
const METRIC_IDX_OMEGA: usize = 9;
const METRIC_IDX_DOWNSIDE_FREQUENCY: usize = 10;
const METRIC_IDX_DOWNSIDE_SEVERITY: usize = 11;
const METRIC_IDX_LONG_MEAN_RETURN: usize = 12;
const METRIC_IDX_SHORT_MEAN_RETURN: usize = 13;
const METRIC_IDX_KAPPA_TAU: usize = 14;
const METRIC_IDX_UNDERWATER_FRACTION: usize = 15;
const METRIC_IDX_MAX_UNDERWATER_DURATION_BARS: usize = 16;
const METRIC_IDX_CLOSED_TRADES: usize = 17;
const METRIC_IDX_WINNING_TRADES: usize = 18;
const METRIC_IDX_LOSING_TRADES: usize = 19;
const METRIC_IDX_GROSS_PROFIT: usize = 20;
const METRIC_IDX_GROSS_LOSS: usize = 21;
const TOPK_BLOCK_ITEMS: usize = 512;

const TOPK_BLOCK_THREADS: u32 = 256;
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
    final_agg_finalize: Duration,
    final_finalist_recompute: Duration,
    final_shortlist_union: Duration,
    final_shortlist_confidence: Duration,
    final_shortlist_slice: Duration,
    final_shortlist_slice_replay: Duration,
    final_shortlist_slice_replay_descriptor_prep: Duration,
    final_shortlist_slice_replay_descriptor_upload: Duration,
    final_shortlist_slice_record_assembly: Duration,
    final_shortlist_slice_sort: Duration,
    final_shortlist_slice_diagnostics: Duration,
    final_export_flush: Duration,
    final_cleanup: Duration,
    tile_count: usize,
}

#[derive(Default, Debug)]
struct FinalistReplayTimingProfile {
    returns: Duration,
    fast_ma: Duration,
    slow_ma: Duration,
    pairmap: Duration,
    backtest: Duration,
    d2h_decode: Duration,
    tile_count: usize,
}

#[derive(Default, Debug, Clone, Copy)]
struct ShortlistSliceTimingProfile {
    replay: Duration,
    replay_descriptor_prep: Duration,
    replay_descriptor_upload: Duration,
    record_assembly: Duration,
    sort: Duration,
    diagnostics: Duration,
}

impl FinalistReplayTimingProfile {
    fn total(&self) -> Duration {
        self.returns
            .saturating_add(self.fast_ma)
            .saturating_add(self.slow_ma)
            .saturating_add(self.pairmap)
            .saturating_add(self.backtest)
            .saturating_add(self.d2h_decode)
    }

    fn emit(&self, fast_ma: &str, slow_ma: &str, finalists: usize, ranges: usize) {
        let total = self.total();
        eprintln!(
            "VECTORBT_KERNEL_FINALIST_PROFILE: pair={fast_ma}/{slow_ma} finalists={finalists} ranges={ranges} tiles={} total_ms={:.3}",
            self.tile_count,
            total.as_secs_f64() * 1000.0
        );
        eprintln!(
            "  returns      {:>10.3} ms  ({})",
            self.returns.as_secs_f64() * 1000.0,
            PipelineTimingProfile::pct_str(self.returns, total)
        );
        eprintln!(
            "  fast_ma      {:>10.3} ms  ({})",
            self.fast_ma.as_secs_f64() * 1000.0,
            PipelineTimingProfile::pct_str(self.fast_ma, total)
        );
        eprintln!(
            "  slow_ma      {:>10.3} ms  ({})",
            self.slow_ma.as_secs_f64() * 1000.0,
            PipelineTimingProfile::pct_str(self.slow_ma, total)
        );
        eprintln!(
            "  pairmap      {:>10.3} ms  ({})",
            self.pairmap.as_secs_f64() * 1000.0,
            PipelineTimingProfile::pct_str(self.pairmap, total)
        );
        eprintln!(
            "  backtest     {:>10.3} ms  ({})",
            self.backtest.as_secs_f64() * 1000.0,
            PipelineTimingProfile::pct_str(self.backtest, total)
        );
        eprintln!(
            "  d2h_decode   {:>10.3} ms  ({})",
            self.d2h_decode.as_secs_f64() * 1000.0,
            PipelineTimingProfile::pct_str(self.d2h_decode, total)
        );
    }
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
        let final_other = self
            .final_finalize
            .saturating_sub(self.final_agg_finalize)
            .saturating_sub(self.final_finalist_recompute)
            .saturating_sub(self.final_shortlist_union)
            .saturating_sub(self.final_shortlist_confidence)
            .saturating_sub(self.final_shortlist_slice)
            .saturating_sub(self.final_export_flush)
            .saturating_sub(self.final_cleanup);
        eprintln!(
            "    agg_finalize{:>10.3} ms  ({})",
            self.final_agg_finalize.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_agg_finalize, self.final_finalize)
        );
        eprintln!(
            "    finalist_recompute{:>6.3} ms  ({})",
            self.final_finalist_recompute.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_finalist_recompute, self.final_finalize)
        );
        eprintln!(
            "    shortlist_union{:>9.3} ms  ({})",
            self.final_shortlist_union.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_shortlist_union, self.final_finalize)
        );
        eprintln!(
            "    shortlist_confidence{:>4.3} ms  ({})",
            self.final_shortlist_confidence.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_shortlist_confidence, self.final_finalize)
        );
        eprintln!(
            "    shortlist_slice{:>9.3} ms  ({})",
            self.final_shortlist_slice.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_shortlist_slice, self.final_finalize)
        );
        let shortlist_slice_other = self
            .final_shortlist_slice
            .saturating_sub(self.final_shortlist_slice_replay)
            .saturating_sub(self.final_shortlist_slice_record_assembly)
            .saturating_sub(self.final_shortlist_slice_sort)
            .saturating_sub(self.final_shortlist_slice_diagnostics);
        eprintln!(
            "      replay{:>15.3} ms  ({})",
            self.final_shortlist_slice_replay.as_secs_f64() * 1000.0,
            Self::pct_str(
                self.final_shortlist_slice_replay,
                self.final_shortlist_slice
            )
        );
        let replay_other = self
            .final_shortlist_slice_replay
            .saturating_sub(self.final_shortlist_slice_replay_descriptor_prep)
            .saturating_sub(self.final_shortlist_slice_replay_descriptor_upload);
        eprintln!(
            "        descriptor_prep{:>5.3} ms  ({})",
            self.final_shortlist_slice_replay_descriptor_prep
                .as_secs_f64()
                * 1000.0,
            Self::pct_str(
                self.final_shortlist_slice_replay_descriptor_prep,
                self.final_shortlist_slice_replay
            )
        );
        eprintln!(
            "        descriptor_upload{:>3.3} ms  ({})",
            self.final_shortlist_slice_replay_descriptor_upload
                .as_secs_f64()
                * 1000.0,
            Self::pct_str(
                self.final_shortlist_slice_replay_descriptor_upload,
                self.final_shortlist_slice_replay
            )
        );
        eprintln!(
            "        replay_other{:>8.3} ms  ({})",
            replay_other.as_secs_f64() * 1000.0,
            Self::pct_str(replay_other, self.final_shortlist_slice_replay)
        );
        eprintln!(
            "      record_assembly{:>6.3} ms  ({})",
            self.final_shortlist_slice_record_assembly.as_secs_f64() * 1000.0,
            Self::pct_str(
                self.final_shortlist_slice_record_assembly,
                self.final_shortlist_slice
            )
        );
        eprintln!(
            "      sort{:>17.3} ms  ({})",
            self.final_shortlist_slice_sort.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_shortlist_slice_sort, self.final_shortlist_slice)
        );
        eprintln!(
            "      diagnostics{:>10.3} ms  ({})",
            self.final_shortlist_slice_diagnostics.as_secs_f64() * 1000.0,
            Self::pct_str(
                self.final_shortlist_slice_diagnostics,
                self.final_shortlist_slice
            )
        );
        eprintln!(
            "      slice_other{:>10.3} ms  ({})",
            shortlist_slice_other.as_secs_f64() * 1000.0,
            Self::pct_str(shortlist_slice_other, self.final_shortlist_slice)
        );
        eprintln!(
            "    export_flush{:>12.3} ms  ({})",
            self.final_export_flush.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_export_flush, self.final_finalize)
        );
        eprintln!(
            "    cleanup{:>17.3} ms  ({})",
            self.final_cleanup.as_secs_f64() * 1000.0,
            Self::pct_str(self.final_cleanup, self.final_finalize)
        );
        eprintln!(
            "    final_other{:>13.3} ms  ({})",
            final_other.as_secs_f64() * 1000.0,
            Self::pct_str(final_other, self.final_finalize)
        );
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct KernelExecutorTimingMs {
    pub(crate) runtime_init_ms: u64,
    pub(crate) set_current_ms: u64,
    pub(crate) gpu_indicator_prep_ms: u64,
    pub(crate) h2d_upload_ms: u64,
    pub(crate) launch_submit_us: u64,
    pub(crate) d2h_copy_ms: u64,
    pub(crate) host_decode_ms: u64,
    pub(crate) explicit_sync_ms: u64,
    pub(crate) host_shortlist_materialize_ms: u64,
    pub(crate) host_finalize_ms: u64,
    pub(crate) host_finalize_agg_finalize_ms: u64,
    pub(crate) host_finalize_finalist_recompute_ms: u64,
    pub(crate) host_finalize_shortlist_union_ms: u64,
    pub(crate) host_finalize_shortlist_confidence_ms: u64,
    pub(crate) host_finalize_shortlist_slice_ms: u64,
    pub(crate) host_finalize_shortlist_slice_replay_ms: u64,
    pub(crate) host_finalize_shortlist_slice_replay_descriptor_prep_ms: u64,
    pub(crate) host_finalize_shortlist_slice_replay_descriptor_upload_ms: u64,
    pub(crate) host_finalize_shortlist_slice_record_assembly_ms: u64,
    pub(crate) host_finalize_shortlist_slice_sort_ms: u64,
    pub(crate) host_finalize_shortlist_slice_diagnostics_ms: u64,
    pub(crate) host_finalize_export_flush_ms: u64,
    pub(crate) host_finalize_cleanup_ms: u64,
    pub(crate) graph_capture_ms: u64,
    pub(crate) graph_replay_count: u64,
}

#[inline]
fn sync_stream_timed(stream: &Stream, elapsed: &mut Duration) -> Result<(), String> {
    let t0 = Instant::now();
    stream.synchronize().map_err(|e| e.to_string())?;
    *elapsed = elapsed.saturating_add(t0.elapsed());
    Ok(())
}

#[inline]
fn launch_kernel_timed(
    stream: &Stream,
    function: &Function,
    grid: (u32, u32, u32),
    block: (u32, u32, u32),
    shared_mem_bytes: u32,
    args: &mut [*mut c_void],
    elapsed: &mut Duration,
) -> Result<(), String> {
    let t0 = Instant::now();
    unsafe {
        stream
            .launch(function, grid, block, shared_mem_bytes, args)
            .map_err(|e| e.to_string())?;
    }
    *elapsed = elapsed.saturating_add(t0.elapsed());
    Ok(())
}

#[inline]
fn graph_launch_timed(
    exec: &CudaGraphExecHandle,
    stream: &Stream,
    elapsed: &mut Duration,
) -> Result<(), String> {
    let t0 = Instant::now();
    exec.launch(stream)?;
    *elapsed = elapsed.saturating_add(t0.elapsed());
    Ok(())
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
    hurdle_tau: f32,
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
        hurdle_tau: strategy.hurdle_tau.unwrap_or(0.0) as f32,
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
fn martin_ratio_from_components(pnl: f64, ulcer_index: f64) -> f64 {
    const EPS_DENOM: f64 = 1e-12;
    const CLAMP_MAX: f64 = 1e6;

    let raw = if ulcer_index > EPS_DENOM {
        pnl / ulcer_index
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
fn fixed_metrics_contract_rows(metrics_count: usize, metrics_stride: usize) -> Option<usize> {
    match (metrics_count, metrics_stride) {
        (PRODUCT_REDUCED_METRICS_COUNT, PRODUCT_REDUCED_METRICS_COUNT) => {
            Some(PRODUCT_REDUCED_METRICS_COUNT)
        }
        (METRICS_COUNT, METRICS_COUNT) => Some(METRICS_COUNT),
        (TRADE_METRICS_COUNT, TRADE_METRICS_COUNT) => Some(TRADE_METRICS_COUNT),
        _ => None,
    }
}

#[inline]
fn exact_eval_metrics_count_with_contract(
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
) -> usize {
    match metric_contract {
        Some(DoubleMaMetricContract::Trade22) => TRADE_METRICS_COUNT,
        Some(DoubleMaMetricContract::Full17) => METRICS_COUNT,
        None if include_trade_metrics => TRADE_METRICS_COUNT,
        None => METRICS_COUNT,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixedContractDispatchKind {
    None,
    WindowedM17,
    WindowedM22,
    M17,
    M22,
    LockstepM17,
    LockstepM22,
}

#[inline]
fn choose_fixed_contract_dispatch_kind(
    fixed_metrics_contract: Option<usize>,
    use_windowed_eval: bool,
    use_lockstep_time: bool,
    use_fast_warp_dedup: bool,
    use_m7: bool,
    use_lockstep_m7: bool,
    has_windowed_m17: bool,
    has_windowed_m22: bool,
    has_m17: bool,
    has_m22: bool,
    has_lockstep_m17: bool,
    has_lockstep_m22: bool,
) -> FixedContractDispatchKind {
    if use_windowed_eval {
        return match fixed_metrics_contract {
            Some(METRICS_COUNT) if has_windowed_m17 => FixedContractDispatchKind::WindowedM17,
            Some(TRADE_METRICS_COUNT) if has_windowed_m22 => FixedContractDispatchKind::WindowedM22,
            _ => FixedContractDispatchKind::None,
        };
    }

    match fixed_metrics_contract {
        Some(METRICS_COUNT) => {
            if use_lockstep_time && !use_lockstep_m7 && has_lockstep_m17 {
                FixedContractDispatchKind::LockstepM17
            } else if !use_lockstep_time && !use_fast_warp_dedup && !use_m7 && has_m17 {
                FixedContractDispatchKind::M17
            } else {
                FixedContractDispatchKind::None
            }
        }
        Some(TRADE_METRICS_COUNT) => {
            if use_lockstep_time && !use_lockstep_m7 && has_lockstep_m22 {
                FixedContractDispatchKind::LockstepM22
            } else if !use_lockstep_time && !use_fast_warp_dedup && !use_m7 && has_m22 {
                FixedContractDispatchKind::M22
            } else {
                FixedContractDispatchKind::None
            }
        }
        _ => FixedContractDispatchKind::None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixedContractKernelFlavor {
    Base,
    RegimeOffNoRisk,
    RegimeOffNoRiskNoComm,
    RegimeOffNoRiskEps0,
    RegimeOffNoRiskEps0NoComm,
    RegimeOnNoRisk,
    RegimeOnNoRiskEps0NoComm,
    RegimeHybridNoRisk,
    RegimeHybridNoRiskEps0NoComm,
}

#[inline]
fn choose_fixed_contract_kernel_flavor(
    no_risk_mode: bool,
    use_regime: bool,
    use_regime_hybrid: bool,
    zero_commission_mode: bool,
    eps_zero_mode: bool,
) -> FixedContractKernelFlavor {
    if !no_risk_mode {
        return FixedContractKernelFlavor::Base;
    }
    if use_regime_hybrid {
        return if zero_commission_mode && eps_zero_mode {
            FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm
        } else {
            FixedContractKernelFlavor::RegimeHybridNoRisk
        };
    }
    if use_regime {
        return if zero_commission_mode && eps_zero_mode {
            FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm
        } else {
            FixedContractKernelFlavor::RegimeOnNoRisk
        };
    }
    if zero_commission_mode {
        return if eps_zero_mode {
            FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm
        } else {
            FixedContractKernelFlavor::RegimeOffNoRiskNoComm
        };
    }
    if eps_zero_mode {
        FixedContractKernelFlavor::RegimeOffNoRiskEps0
    } else {
        FixedContractKernelFlavor::RegimeOffNoRisk
    }
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

#[derive(Clone, Copy, Debug, Default)]
struct DeviceConstraintArgs {
    use_min_trades: i32,
    min_trades: f32,
    use_max_dd: i32,
    max_dd: f32,
    use_min_sharpe: i32,
    min_sharpe: f32,
    use_min_sortino: i32,
    min_sortino: f32,
    use_min_calmar: i32,
    min_calmar: f32,
    use_max_volatility: i32,
    max_volatility: f32,
    use_min_exposure: i32,
    min_exposure: f32,
    use_max_exposure: i32,
    max_exposure: f32,
    use_max_abs_net_exposure: i32,
    max_abs_net_exposure: f32,
    use_min_omega: i32,
    min_omega: f32,
    use_max_ulcer: i32,
    max_ulcer: f32,
    use_max_downside_deviation: i32,
    max_downside_deviation: f32,
}

#[inline]
fn min_metrics_count_for_constraints(constraints: Option<&DoubleMaConstraints>) -> usize {
    let Some(cfg) = constraints else {
        return 0;
    };

    let mut metrics_count = 0usize;
    if cfg.min_trades.is_some() {
        metrics_count = metrics_count.max(2);
    }
    if cfg.max_drawdown.is_some() || cfg.min_calmar.is_some() {
        metrics_count = metrics_count.max(3);
    }
    if cfg.min_sharpe.is_some() || cfg.max_volatility.is_some() {
        metrics_count = metrics_count.max(5);
    }
    if cfg.min_exposure.is_some() {
        metrics_count = metrics_count.max(6);
    }
    if cfg.max_exposure.is_some() || cfg.max_abs_net_exposure.is_some() {
        metrics_count = metrics_count.max(7);
    }
    if cfg.min_sortino.is_some() {
        metrics_count = metrics_count.max(8);
    }
    if cfg.max_downside_deviation.is_some() {
        metrics_count = metrics_count.max(8);
    }
    if cfg.max_ulcer.is_some() {
        metrics_count = metrics_count.max(9);
    }
    if cfg.min_omega.is_some() {
        metrics_count = metrics_count.max(METRICS_COUNT);
    }
    metrics_count
}

#[inline]
fn build_device_constraint_args(
    constraints: Option<&DoubleMaConstraints>,
    metrics_count: usize,
) -> DeviceConstraintArgs {
    let Some(cfg) = constraints else {
        return DeviceConstraintArgs::default();
    };

    DeviceConstraintArgs {
        use_min_trades: i32::from(cfg.min_trades.is_some()),
        min_trades: cfg.min_trades.map(|v| v as f32).unwrap_or(0.0),
        use_max_dd: i32::from(cfg.max_drawdown.is_some()),
        max_dd: cfg
            .max_drawdown
            .map(|v| max_drawdown_constraint_to_search_space(v, metrics_count))
            .unwrap_or(f32::INFINITY),
        use_min_sharpe: i32::from(cfg.min_sharpe.is_some()),
        min_sharpe: cfg.min_sharpe.unwrap_or(f32::NEG_INFINITY as f64) as f32,
        use_min_sortino: i32::from(cfg.min_sortino.is_some()),
        min_sortino: cfg.min_sortino.unwrap_or(f32::NEG_INFINITY as f64) as f32,
        use_min_calmar: i32::from(cfg.min_calmar.is_some()),
        min_calmar: cfg.min_calmar.unwrap_or(f32::NEG_INFINITY as f64) as f32,
        use_max_volatility: i32::from(cfg.max_volatility.is_some()),
        max_volatility: cfg.max_volatility.unwrap_or(f32::INFINITY as f64) as f32,
        use_min_exposure: i32::from(cfg.min_exposure.is_some()),
        min_exposure: cfg.min_exposure.unwrap_or(f32::NEG_INFINITY as f64) as f32,
        use_max_exposure: i32::from(cfg.max_exposure.is_some()),
        max_exposure: cfg.max_exposure.unwrap_or(f32::INFINITY as f64) as f32,
        use_max_abs_net_exposure: i32::from(cfg.max_abs_net_exposure.is_some()),
        max_abs_net_exposure: cfg.max_abs_net_exposure.unwrap_or(f32::INFINITY as f64) as f32,
        use_min_omega: i32::from(cfg.min_omega.is_some()),
        min_omega: cfg.min_omega.unwrap_or(f32::NEG_INFINITY as f64) as f32,
        use_max_ulcer: i32::from(cfg.max_ulcer.is_some()),
        max_ulcer: cfg.max_ulcer.unwrap_or(f32::INFINITY as f64) as f32,
        use_max_downside_deviation: i32::from(cfg.max_downside_deviation.is_some()),
        max_downside_deviation: cfg.max_downside_deviation.unwrap_or(f32::INFINITY as f64) as f32,
    }
}

#[inline]
fn decode_tm_flex_metrics_row(
    src: &[f32],
    base: usize,
    metrics_count: usize,
    hurdle_tau: f64,
) -> Metrics {
    const CLAMP_MAX: f64 = 1e6;

    let metric = |idx: usize| -> f64 {
        if metrics_count > idx && base + idx < src.len() {
            src[base + idx] as f64
        } else {
            0.0
        }
    };
    let metric_u32 = |idx: usize| -> u32 {
        let value = metric(idx);
        if value.is_finite() && value >= 0.0 {
            value as u32
        } else {
            0
        }
    };

    let raw_pnl = metric(METRIC_IDX_PNL);
    let raw_max_dd = metric(METRIC_IDX_MAX_DD);
    let mean = metric(METRIC_IDX_MEAN);
    let std = metric(METRIC_IDX_STD);
    let sharpe = if std > 0.0 { mean / std } else { 0.0 };
    let downside_deviation = metric(METRIC_IDX_DOWNSIDE_DEVIATION);
    let mean_excess = mean - hurdle_tau;
    let sortino = if downside_deviation > 0.0 {
        mean_excess / downside_deviation
    } else if mean_excess > 0.0 {
        CLAMP_MAX
    } else if mean_excess < 0.0 {
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
    let ulcer_index = metric(METRIC_IDX_ULCER_INDEX);
    let omega = if metrics_count > METRIC_IDX_OMEGA {
        metric(METRIC_IDX_OMEGA).clamp(0.0, CLAMP_MAX)
    } else {
        0.0
    };
    let kappa_tau = if metrics_count > METRIC_IDX_KAPPA_TAU {
        metric(METRIC_IDX_KAPPA_TAU).clamp(-CLAMP_MAX, CLAMP_MAX)
    } else {
        0.0
    };
    let downside_frequency = if metrics_count > METRIC_IDX_DOWNSIDE_FREQUENCY {
        metric(METRIC_IDX_DOWNSIDE_FREQUENCY).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let downside_severity = if metrics_count > METRIC_IDX_DOWNSIDE_SEVERITY {
        metric(METRIC_IDX_DOWNSIDE_SEVERITY).max(0.0)
    } else {
        0.0
    };
    let underwater_fraction = if metrics_count > METRIC_IDX_UNDERWATER_FRACTION {
        metric(METRIC_IDX_UNDERWATER_FRACTION).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let max_underwater_duration_bars = if metrics_count > METRIC_IDX_MAX_UNDERWATER_DURATION_BARS {
        metric_u32(METRIC_IDX_MAX_UNDERWATER_DURATION_BARS)
    } else {
        0
    };
    let martin_ratio = if metrics_count > METRIC_IDX_ULCER_INDEX {
        martin_ratio_from_components(pnl, ulcer_index)
    } else {
        0.0
    };
    let closed_trades = if metrics_count > METRIC_IDX_CLOSED_TRADES {
        metric_u32(METRIC_IDX_CLOSED_TRADES)
    } else {
        0
    };
    let winning_trades = if metrics_count > METRIC_IDX_WINNING_TRADES {
        metric_u32(METRIC_IDX_WINNING_TRADES)
    } else {
        0
    };
    let losing_trades = if metrics_count > METRIC_IDX_LOSING_TRADES {
        metric_u32(METRIC_IDX_LOSING_TRADES)
    } else {
        0
    };
    let gross_profit = if metrics_count > METRIC_IDX_GROSS_PROFIT {
        metric(METRIC_IDX_GROSS_PROFIT).max(0.0)
    } else {
        0.0
    };
    let gross_loss = if metrics_count > METRIC_IDX_GROSS_LOSS {
        metric(METRIC_IDX_GROSS_LOSS).max(0.0)
    } else {
        0.0
    };

    Metrics {
        pnl,
        sharpe,
        sortino,
        downside_deviation,
        downside_frequency,
        downside_severity,
        calmar: calmar_ratio(pnl, max_dd),
        omega,
        kappa_tau,
        volatility: std,
        ulcer_index,
        martin_ratio,
        max_dd,
        trades: metric_u32(METRIC_IDX_TRADES),
        exposure: metric(METRIC_IDX_EXPOSURE),
        net_exposure: metric(METRIC_IDX_NET_EXPOSURE),
        long_mean_return: metric(METRIC_IDX_LONG_MEAN_RETURN),
        short_mean_return: metric(METRIC_IDX_SHORT_MEAN_RETURN),
        closed_trades,
        winning_trades,
        losing_trades,
        gross_profit,
        gross_loss,
        underwater_fraction,
        max_underwater_duration_bars,
    }
}

#[inline]
fn decode_packed_topk_metric_row(
    src: &[f32],
    row_idx: usize,
    metrics_count: usize,
    hurdle_tau: f64,
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
    let metrics = decode_tm_flex_metrics_row(src, base + 2, metrics_count, hurdle_tau);
    Some((fast_len, slow_len, metrics))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReturnsCacheKey {
    dataset: CandleDatasetSignature,
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
    d_tile_scores: Option<DeviceBuffer<f32>>,

    d_heatmap: Option<DeviceBuffer<f32>>,
    d_heatmap_sum: Option<DeviceBuffer<f32>>,
    d_heatmap_count: Option<DeviceBuffer<i32>>,
    d_cand_a: Option<DeviceBuffer<i32>>,
    d_cand_b: Option<DeviceBuffer<i32>>,
    d_cand_scores_a: Option<DeviceBuffer<f32>>,
    d_cand_scores_b: Option<DeviceBuffer<f32>>,
    d_top_fast: Option<DeviceBuffer<i32>>,
    d_top_slow: Option<DeviceBuffer<i32>>,
    d_top_scores: Option<DeviceBuffer<f32>>,
    d_top_metrics: Option<DeviceBuffer<f32>>,
    d_top_packed: Option<DeviceBuffer<f32>>,
    d_valid_pair_index: Option<DeviceBuffer<i32>>,
    d_valid_pair_fast_period: Option<DeviceBuffer<i32>>,
    d_valid_pair_slow_period: Option<DeviceBuffer<i32>>,
    d_valid_pair_fast_local: Option<DeviceBuffer<i32>>,
    d_valid_pair_slow_local: Option<DeviceBuffer<i32>>,
    d_valid_pair_eval_start: Option<DeviceBuffer<i32>>,
    d_valid_pair_eval_end: Option<DeviceBuffer<i32>>,
    returns_key: Option<ReturnsCacheKey>,
}

#[derive(Default)]
struct HostTransferScratchF32 {
    pageable: Vec<f32>,
    pinned: Option<LockedBuffer<f32>>,
}

impl HostTransferScratchF32 {
    fn ensure_capacity(&mut self, len: usize) -> Result<(), String> {
        if kernel_pinned_d2h_enabled() {
            let needs_realloc = self
                .pinned
                .as_ref()
                .map(|buf| buf.len() < len)
                .unwrap_or(true);
            if needs_realloc {
                self.pinned = Some(
                    unsafe { LockedBuffer::<f32>::uninitialized(len) }
                        .map_err(|e| e.to_string())?,
                );
            }
        } else if self.pageable.capacity() < len {
            self.pageable = alloc_uninit_copy(len);
        }
        Ok(())
    }

    fn copy_device_prefix_to_host<'a>(
        &'a mut self,
        src: &DeviceBuffer<f32>,
        elems: usize,
        stream: &Stream,
    ) -> Result<&'a [f32], String> {
        if elems == 0 {
            return Ok(&[]);
        }

        if kernel_pinned_d2h_enabled() {
            self.ensure_capacity(elems)?;
            let pinned = self
                .pinned
                .as_mut()
                .ok_or_else(|| "missing pinned host transfer scratch".to_string())?;
            let bytes = elems
                .checked_mul(std::mem::size_of::<f32>())
                .ok_or_else(|| "pinned device-to-host copy size overflow".to_string())?;
            cu_result(unsafe {
                cuda_sys::cuMemcpyDtoHAsync_v2(
                    pinned.as_mut_ptr() as *mut c_void,
                    src.as_device_ptr().as_raw(),
                    bytes,
                    stream.as_inner(),
                )
            })?;
            stream.synchronize().map_err(|e| e.to_string())?;
            return Ok(&pinned.as_slice()[..elems]);
        }

        copy_device_prefix_to_host_f32(src, &mut self.pageable, elems, stream)?;
        stream.synchronize().map_err(|e| e.to_string())?;
        Ok(&self.pageable[..elems])
    }

    fn copy_device_prefix_to_host_timed<'a>(
        &'a mut self,
        src: &DeviceBuffer<f32>,
        elems: usize,
        stream: &Stream,
        d2h_copy_elapsed: &mut Duration,
        sync_elapsed: &mut Duration,
    ) -> Result<&'a [f32], String> {
        if elems == 0 {
            return Ok(&[]);
        }

        self.ensure_capacity(elems)?;
        let bytes = elems
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or_else(|| "device-to-host copy size overflow".to_string())?;

        if kernel_pinned_d2h_enabled() {
            let pinned = self
                .pinned
                .as_mut()
                .ok_or_else(|| "missing pinned host transfer scratch".to_string())?;
            let d2h_t0 = Instant::now();
            cu_result(unsafe {
                cuda_sys::cuMemcpyDtoHAsync_v2(
                    pinned.as_mut_ptr() as *mut c_void,
                    src.as_device_ptr().as_raw(),
                    bytes,
                    stream.as_inner(),
                )
            })?;
            *d2h_copy_elapsed = d2h_copy_elapsed.saturating_add(d2h_t0.elapsed());
            sync_stream_timed(stream, sync_elapsed)?;
            return Ok(&pinned.as_slice()[..elems]);
        }

        resize_uninit_f32(&mut self.pageable, elems);
        let d2h_t0 = Instant::now();
        cu_result(unsafe {
            cuda_sys::cuMemcpyDtoHAsync_v2(
                self.pageable.as_mut_ptr() as *mut c_void,
                src.as_device_ptr().as_raw(),
                bytes,
                stream.as_inner(),
            )
        })?;
        *d2h_copy_elapsed = d2h_copy_elapsed.saturating_add(d2h_t0.elapsed());
        sync_stream_timed(stream, sync_elapsed)?;
        Ok(&self.pageable[..elems])
    }
}

#[derive(Default)]
struct HostExactComboScratch {
    host_p_fast: Vec<i32>,
    host_p_slow: Vec<i32>,
    fast_precomputed_host_tile: Vec<f32>,
    slow_precomputed_host_tile: Vec<f32>,
    tile_host_buf: HostTransferScratchF32,
    tile_combo_indices: Vec<usize>,
    tile_pair_index_map: Vec<i32>,
    tile_fast_period_rows: Vec<i32>,
    tile_slow_period_rows: Vec<i32>,
    tile_fast_local_rows: Vec<i32>,
    tile_slow_local_rows: Vec<i32>,
    tile_range_indices: Vec<usize>,
    tile_eval_start_rows: Vec<i32>,
    tile_eval_end_rows: Vec<i32>,
    last_fast_period_upload: Option<PeriodUploadSignature>,
    last_slow_period_upload: Option<PeriodUploadSignature>,
    last_pair_index_upload: Option<PeriodUploadSignature>,
    last_eval_start_upload: Option<PeriodUploadSignature>,
    last_eval_end_upload: Option<PeriodUploadSignature>,
}

impl HostExactComboScratch {
    fn ensure_tile_host_buf_len(&mut self, len: usize) -> Result<(), String> {
        self.tile_host_buf.ensure_capacity(len)
    }
}

fn prepare_batched_windowed_exact_replay_rows(
    host_exact_combo: &mut HostExactComboScratch,
    ranges: &[(usize, usize)],
    n: usize,
) -> Result<(), String> {
    host_exact_combo.tile_range_indices.clear();
    host_exact_combo.tile_eval_start_rows.clear();
    host_exact_combo.tile_eval_end_rows.clear();

    if host_exact_combo.tile_combo_indices.is_empty() {
        return Ok(());
    }

    let base_combo_indices = host_exact_combo.tile_combo_indices.clone();
    let base_pair_index_map = host_exact_combo.tile_pair_index_map.clone();
    let base_fast_period_rows = host_exact_combo.tile_fast_period_rows.clone();
    let base_slow_period_rows = host_exact_combo.tile_slow_period_rows.clone();
    let base_fast_local_rows = host_exact_combo.tile_fast_local_rows.clone();
    let base_slow_local_rows = host_exact_combo.tile_slow_local_rows.clone();
    let base_len = base_combo_indices.len();
    let batched_len = base_len
        .checked_mul(ranges.len())
        .ok_or_else(|| "batched exact replay row count overflow".to_string())?;

    host_exact_combo.tile_combo_indices.clear();
    host_exact_combo.tile_pair_index_map.clear();
    host_exact_combo.tile_fast_period_rows.clear();
    host_exact_combo.tile_slow_period_rows.clear();
    host_exact_combo.tile_fast_local_rows.clear();
    host_exact_combo.tile_slow_local_rows.clear();
    host_exact_combo.tile_range_indices.reserve(batched_len);
    host_exact_combo.tile_eval_start_rows.reserve(batched_len);
    host_exact_combo.tile_eval_end_rows.reserve(batched_len);

    for (range_idx, &(eval_start_t, eval_end_t_raw)) in ranges.iter().enumerate() {
        let eval_start_i = i32::try_from(eval_start_t)
            .map_err(|_| "batched replay eval_start exceeds i32::MAX".to_string())?;
        let eval_end_i = i32::try_from(eval_end_t_raw.min(n))
            .map_err(|_| "batched replay eval_end exceeds i32::MAX".to_string())?;
        host_exact_combo
            .tile_combo_indices
            .extend_from_slice(&base_combo_indices);
        host_exact_combo
            .tile_pair_index_map
            .extend_from_slice(&base_pair_index_map);
        host_exact_combo
            .tile_fast_period_rows
            .extend_from_slice(&base_fast_period_rows);
        host_exact_combo
            .tile_slow_period_rows
            .extend_from_slice(&base_slow_period_rows);
        host_exact_combo
            .tile_fast_local_rows
            .extend_from_slice(&base_fast_local_rows);
        host_exact_combo
            .tile_slow_local_rows
            .extend_from_slice(&base_slow_local_rows);
        host_exact_combo
            .tile_range_indices
            .extend(std::iter::repeat(range_idx).take(base_len));
        host_exact_combo
            .tile_eval_start_rows
            .extend(std::iter::repeat(eval_start_i).take(base_len));
        host_exact_combo
            .tile_eval_end_rows
            .extend(std::iter::repeat(eval_end_i).take(base_len));
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PeriodUploadSignature {
    len: usize,
    hash: u64,
}

#[inline]
fn period_upload_signature(periods: &[i32]) -> PeriodUploadSignature {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    periods.hash(&mut hasher);
    PeriodUploadSignature {
        len: periods.len(),
        hash: hasher.finish(),
    }
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

#[derive(Clone, Copy)]
struct ResolvedValidPairMap {
    index_ptr: u64,
    len: usize,
    fast_period_ptr: u64,
    slow_period_ptr: u64,
    fast_local_ptr: u64,
    slow_local_ptr: u64,
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
        Self::drop_opt_async(&mut self.d_tile_scores, stream)?;

        Self::drop_opt_async(&mut self.d_heatmap, stream)?;
        Self::drop_opt_async(&mut self.d_heatmap_sum, stream)?;
        Self::drop_opt_async(&mut self.d_heatmap_count, stream)?;
        Self::drop_opt_async(&mut self.d_cand_a, stream)?;
        Self::drop_opt_async(&mut self.d_cand_b, stream)?;
        Self::drop_opt_async(&mut self.d_cand_scores_a, stream)?;
        Self::drop_opt_async(&mut self.d_cand_scores_b, stream)?;
        Self::drop_opt_async(&mut self.d_top_fast, stream)?;
        Self::drop_opt_async(&mut self.d_top_slow, stream)?;
        Self::drop_opt_async(&mut self.d_top_scores, stream)?;
        Self::drop_opt_async(&mut self.d_top_metrics, stream)?;
        Self::drop_opt_async(&mut self.d_top_packed, stream)?;
        Self::drop_opt_async(&mut self.d_valid_pair_index, stream)?;
        Self::drop_opt_async(&mut self.d_valid_pair_fast_period, stream)?;
        Self::drop_opt_async(&mut self.d_valid_pair_slow_period, stream)?;
        Self::drop_opt_async(&mut self.d_valid_pair_fast_local, stream)?;
        Self::drop_opt_async(&mut self.d_valid_pair_slow_local, stream)?;
        Self::drop_opt_async(&mut self.d_valid_pair_eval_start, stream)?;
        Self::drop_opt_async(&mut self.d_valid_pair_eval_end, stream)?;
        self.returns_key = None;
        Ok(())
    }
}

struct CudaKernelRuntime {
    context: std::sync::Arc<Context>,
    module: Module,
    stream: Stream,
    staging_stream: Stream,
    ma: VramMaComputer,
    scratch: KernelScratch,
    host_exact_combo_scratch: HostExactComboScratch,
    launch_caps: DeviceLaunchCaps,
    resident_market: Option<ResidentMarketSeriesCache>,
    staged_market: Option<ResidentMarketSeriesCache>,
    valid_pair_maps: HashMap<ValidPairMapSignature, CachedValidPairMap>,
    kernel_caps: RefCell<HashMap<CachedKernelKey, KernelLaunchCaps>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CachedKernelKey {
    ComputeReturns,
    TransposeRowToTm,
    DoubleCrossBacktest,
    DoubleCrossBacktestPruneMaxdd,
    DoubleCrossBacktestM7,
    DoubleCrossBacktestM17,
    DoubleCrossBacktestM17PruneMaxdd,
    DoubleCrossBacktestM22,
    DoubleCrossBacktestM22PruneMaxdd,
    DoubleCrossBacktestLockstep,
    DoubleCrossBacktestLockstepM7,
    DoubleCrossBacktestLockstepM17,
    DoubleCrossBacktestLockstepM22,
    DoubleCrossBacktestFastWarpDedup,
    FillF32,
    FillI32,
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
        let staging_stream =
            Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;

        Ok(Self {
            context,
            module,
            stream,
            staging_stream,
            ma: VramMaComputer::new(device_id),
            scratch: KernelScratch::default(),
            host_exact_combo_scratch: HostExactComboScratch::default(),
            launch_caps,
            resident_market: None,
            staged_market: None,
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

pub fn prepare_cuda_kernel_runtime_for_probe(device_id: u32) -> Result<usize, String> {
    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<usize, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map
            .get_mut(&device_id)
            .ok_or_else(|| format!("missing CUDA kernel runtime for device {device_id}"))?;
        rt.set_current()?;
        Ok(rt.context.as_raw() as usize)
    })
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
) -> Result<Option<ResolvedValidPairMap>, String> {
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
    Ok(Some(ResolvedValidPairMap {
        index_ptr: entry.buffer.as_device_ptr().as_raw(),
        len: entry.len,
        fast_period_ptr: entry.fast_rows.as_device_ptr().as_raw(),
        slow_period_ptr: entry.slow_rows.as_device_ptr().as_raw(),
        fast_local_ptr: 0,
        slow_local_ptr: 0,
    }))
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

#[derive(Clone, Copy, Debug)]
struct EffectiveSampleScoreContext {
    first_valid: usize,
    regime_first_valid: usize,
    eval_start: usize,
    eval_end: usize,
    total_bars: usize,
    trade_on_next_bar: bool,
}

thread_local! {
    static CUDA_KERNEL_RUNTIME: RefCell<HashMap<u32, CudaKernelRuntime>> = RefCell::new(HashMap::new());
    static DYNAMIC_GPU_METRICS_SCRATCH: RefCell<HashMap<(u32, usize), DeviceBuffer<f32>>> = RefCell::new(HashMap::new());
    static LAST_KERNEL_EXECUTOR_TIMING: RefCell<Option<KernelExecutorTimingMs>> = RefCell::new(None);
    static LAST_ROUTE_DIAGNOSTICS: RefCell<Option<Vec<String>>> = RefCell::new(None);
    static LAST_SHORTLIST_SLICE_DIAGNOSTICS: RefCell<Option<ShortlistSliceDiagnostics>> = RefCell::new(None);
    static LAST_SCORE_OBSERVATION_CONTEXT: RefCell<Option<PairObservationScoreContextResolved>> = RefCell::new(None);
    static EFFECTIVE_SAMPLE_SCORE_CONTEXT: RefCell<Option<EffectiveSampleScoreContext>> = RefCell::new(None);
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

fn clear_last_route_diagnostics() {
    LAST_ROUTE_DIAGNOSTICS.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

fn record_last_route_diagnostics(diag: Vec<String>) {
    LAST_ROUTE_DIAGNOSTICS.with(|cell| {
        *cell.borrow_mut() = Some(diag);
    });
}

fn append_last_route_diagnostics(entries: impl IntoIterator<Item = String>) {
    LAST_ROUTE_DIAGNOSTICS.with(|cell| {
        if let Some(diag) = cell.borrow_mut().as_mut() {
            diag.extend(entries);
        }
    });
}

pub(crate) fn take_last_route_diagnostics() -> Option<Vec<String>> {
    LAST_ROUTE_DIAGNOSTICS.with(|cell| cell.borrow_mut().take())
}

fn clear_last_shortlist_slice_diagnostics() {
    LAST_SHORTLIST_SLICE_DIAGNOSTICS.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

fn clear_last_score_observation_context() {
    LAST_SCORE_OBSERVATION_CONTEXT.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub(crate) fn record_last_shortlist_slice_diagnostics(diag: ShortlistSliceDiagnostics) {
    LAST_SHORTLIST_SLICE_DIAGNOSTICS.with(|cell| {
        *cell.borrow_mut() = Some(diag);
    });
}

pub(crate) fn take_last_shortlist_slice_diagnostics() -> Option<ShortlistSliceDiagnostics> {
    LAST_SHORTLIST_SLICE_DIAGNOSTICS.with(|cell| cell.borrow_mut().take())
}

fn record_last_score_observation_context(ctx: PairObservationScoreContextResolved) {
    LAST_SCORE_OBSERVATION_CONTEXT.with(|cell| {
        *cell.borrow_mut() = Some(ctx);
    });
}

pub(crate) fn take_last_score_observation_context() -> Option<PairObservationScoreContextResolved> {
    LAST_SCORE_OBSERVATION_CONTEXT.with(|cell| cell.borrow_mut().take())
}

pub(crate) fn with_effective_sample_score_context<T>(
    ctx: PairObservationScoreContextResolved,
    f: impl FnOnce() -> T,
) -> T {
    record_last_score_observation_context(ctx);
    let _guard = install_effective_sample_score_context(EffectiveSampleScoreContext {
        first_valid: ctx.first_valid,
        regime_first_valid: ctx.regime_first_valid,
        eval_start: ctx.eval_start,
        eval_end: ctx.eval_end,
        total_bars: ctx.total_bars,
        trade_on_next_bar: ctx.trade_on_next_bar,
    });
    f()
}

#[inline]
pub(crate) fn effective_sample_lcb_10_pair_score_for_host(
    params: &DoubleMaParams,
    metrics: &Metrics,
) -> f64 {
    effective_sample_lcb_10_pair_score(params, metrics)
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

fn fill_device_buffer_f32_timed(
    stream: &Stream,
    fill_kernel: &Function,
    device_launch_caps: DeviceLaunchCaps,
    fill_launch_caps: KernelLaunchCaps,
    buffer: &DeviceBuffer<f32>,
    len: usize,
    value: f32,
    launch_submit_elapsed: &mut Duration,
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
        launch_kernel_timed(
            stream,
            fill_kernel,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
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
    pub hurdle_tau: f64,
    pub include_trade_metrics: bool,
    pub metric_contract: Option<DoubleMaMetricContract>,
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn rerank_shortlist_by_slice_robustness_for_host(
    device_id: u32,
    candles: &Candles,
    rows: &[(DoubleMaParams, Metrics)],
    mode: DoubleMaShortlistSliceMode,
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    benchmark: Option<&SpaBenchmarkKind>,
    first_valid: usize,
    eval_start: usize,
    eval_end: usize,
    fast_alma_offset: f64,
    fast_alma_sigma: f64,
    slow_alma_offset: f64,
    slow_alma_sigma: f64,
    hurdle_tau: f64,
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
) -> Result<(Vec<(DoubleMaParams, Metrics)>, ShortlistSliceDiagnostics), String> {
    let mode = kernel_shortlist_slice_mode(Some(mode))?
        .ok_or_else(|| "shortlist_slice_mode is required".to_string())?;
    CUDA_KERNEL_RUNTIME.with(
        |cell| -> Result<(Vec<(DoubleMaParams, Metrics)>, ShortlistSliceDiagnostics), String> {
            let mut map = cell.borrow_mut();
            if !map.contains_key(&device_id) {
                map.insert(device_id, CudaKernelRuntime::new(device_id)?);
            }
            let rt = map.get_mut(&device_id).expect("just inserted");
            rt.set_current()?;
            rerank_shortlist_by_slice_robustness(
                rt,
                candles,
                rows,
                rows.len(),
                rows.len(),
                mode,
                fast_ma_type,
                slow_ma_type,
                ma_source,
                fast_ma_params,
                slow_ma_params,
                None,
                KernelConfig {
                    device_id,
                    fast_alma_offset,
                    fast_alma_sigma,
                    slow_alma_offset,
                    slow_alma_sigma,
                    hurdle_tau,
                    include_trade_metrics,
                    metric_contract,
                },
                None,
                None,
                strategy,
                objective,
                score_mode,
                benchmark,
                first_valid,
                first_valid,
                eval_start,
                eval_end,
                None,
            )
        },
    )
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
    pub(crate) compiled_plan_fingerprint_hex: String,
    pub(crate) executable_fingerprint_hex: String,
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
            compiled_plan_fingerprint_hex: abi.compiled_plan_fingerprint_hex.clone(),
            executable_fingerprint_hex: abi.executable_fingerprint_hex.clone(),
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
    executable_fingerprint_hex: String,
    device_id: u32,
}

impl UploadedDoubleMaDynamicProgramBatch {
    pub fn combo_count(&self) -> usize {
        self.combo_count
    }

    pub(crate) fn first_valid(&self) -> usize {
        self.first_valid
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
    crate::strategy_dynamic::validate_canonical_double_ma_crossover_plan_shape_v1(&handle.plan)?;
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

fn dynamic_program_executable_fingerprint_hex(handle: &StrategyHandle) -> Result<String, String> {
    handle
        .gpu_program_abi
        .as_ref()
        .map(|abi| abi.executable_fingerprint_hex.clone())
        .ok_or_else(|| "strategy handle missing GPU program ABI".to_string())
}

fn validate_uploaded_double_ma_dynamic_program_batch_geometry(
    lr_len: usize,
    stride: usize,
    fast_rows: usize,
    slow_rows: usize,
    combo_count: usize,
    first_valid: usize,
) -> Result<(), String> {
    if first_valid >= lr_len {
        return Err(format!(
            "dynamic gpu batch first_valid {first_valid} must be < lr len {lr_len}"
        ));
    }
    if stride != lr_len {
        return Err(format!(
            "dynamic gpu batch matrix stride {stride} must match lr len {lr_len}"
        ));
    }
    if stride > i32::MAX as usize || lr_len > i32::MAX as usize {
        return Err("dynamic gpu batch dimensions exceed i32 kernel limits".to_string());
    }
    if fast_rows > i32::MAX as usize
        || slow_rows > i32::MAX as usize
        || combo_count > i32::MAX as usize
    {
        return Err(
            "dynamic gpu batch period rows or combo count exceed i32 kernel limits".to_string(),
        );
    }
    let dense_pair_grid = fast_rows
        .checked_mul(slow_rows)
        .ok_or_else(|| "dynamic gpu batch dense pair grid exceeds i32 kernel limits".to_string())?;
    if dense_pair_grid > i32::MAX as usize {
        return Err("dynamic gpu batch dense pair grid exceeds i32 kernel limits".to_string());
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
    handle: &StrategyHandle,
    device_id: u32,
) -> Result<UploadedDoubleMaDynamicProgramBatch, String> {
    validate_double_ma_dynamic_strategy_handle(handle)?;
    let executable_fingerprint_hex = dynamic_program_executable_fingerprint_hex(handle)?;
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
    validate_uploaded_double_ma_dynamic_program_batch_geometry(
        lr.len(),
        stride,
        fast_rows,
        slow_rows,
        combos.len(),
        first_valid,
    )?;
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
                executable_fingerprint_hex,
                device_id,
            })
        },
    )
}

pub fn eval_uploaded_double_ma_dynamic_program_gpu_kernel(
    uploaded_batch: &UploadedDoubleMaDynamicProgramBatch,
    strategy: &StrategyConfig,
    handle: &StrategyHandle,
    metrics_count: usize,
) -> Result<Vec<Metrics>, String> {
    validate_double_ma_dynamic_strategy_handle(handle)?;
    let executable_fingerprint_hex = dynamic_program_executable_fingerprint_hex(handle)?;
    if uploaded_batch.executable_fingerprint_hex != executable_fingerprint_hex {
        return Err(
            "uploaded dynamic gpu batch executable fingerprint does not match strategy handle"
                .to_string(),
        );
    }
    let _ = get_or_build_gpu_strategy_executable(handle, uploaded_batch.device_id)?;
    let key =
        GpuStrategyExecutableCacheKey::new(executable_fingerprint_hex, uploaded_batch.device_id);
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
            let out_stride = metrics_count.clamp(1, TRADE_METRICS_COUNT);
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
            let mut hurdle_tau_f = strategy_params.hurdle_tau;
            let mut stop_log_f = strategy_params.stop_log;
            let mut take_log_f = strategy_params.take_log;
            let mut m_i = out_stride as i32;
            let mut metrics_stride_i = out_stride as i32;
            let mut pair_map_ptr = uploaded_batch.d_pair_index_map.as_device_ptr().as_raw();
            let mut pair_fast_rows_ptr = 0u64;
            let mut pair_slow_rows_ptr = 0u64;
            let mut pair_fast_local_rows_ptr = 0u64;
            let mut pair_slow_local_rows_ptr = 0u64;
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
                    &mut hurdle_tau_f as *mut _ as *mut c_void,
                    &mut stop_log_f as *mut _ as *mut c_void,
                    &mut take_log_f as *mut _ as *mut c_void,
                    &mut flags_u as *mut _ as *mut c_void,
                    &mut m_i as *mut _ as *mut c_void,
                    &mut metrics_stride_i as *mut _ as *mut c_void,
                    &mut pair_map_ptr as *mut _ as *mut c_void,
                    &mut pair_fast_rows_ptr as *mut _ as *mut c_void,
                    &mut pair_slow_rows_ptr as *mut _ as *mut c_void,
                    &mut pair_fast_local_rows_ptr as *mut _ as *mut c_void,
                    &mut pair_slow_local_rows_ptr as *mut _ as *mut c_void,
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
                out_metrics.push(decode_tm_flex_metrics_row(
                    &host_out,
                    base,
                    out_stride,
                    strategy_params.hurdle_tau as f64,
                ));
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
    metrics_count: usize,
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
        handle,
        device_id,
    )?;
    eval_uploaded_double_ma_dynamic_program_gpu_kernel(&uploaded, strategy, handle, metrics_count)
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
            hurdle_tau: 0.0,
            include_trade_metrics: false,
            metric_contract: None,
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

#[cfg(test)]
#[inline]
fn build_valid_pair_local_rows_i32(
    fast_periods: &[u16],
    slow_periods: &[u16],
) -> Result<(Vec<i32>, Vec<i32>), String> {
    let ps = slow_periods.len();
    let mut fast_rows: Vec<i32> = Vec::with_capacity(fast_periods.len().saturating_mul(ps));
    let mut slow_rows: Vec<i32> = Vec::with_capacity(fast_periods.len().saturating_mul(ps));
    for (fi, &fast_len) in fast_periods.iter().enumerate() {
        let fi_i32 = i32::try_from(fi)
            .map_err(|_| "valid pair fast local row exceeds i32::MAX".to_string())?;
        let j0 = slow_periods.partition_point(|&s| s <= fast_len);
        for sj in j0..ps {
            fast_rows.push(fi_i32);
            slow_rows.push(
                i32::try_from(sj)
                    .map_err(|_| "valid pair slow local row exceeds i32::MAX".to_string())?,
            );
        }
    }
    Ok((fast_rows, slow_rows))
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
    let classic_pair = matches!(fast_ma, "sma" | "alma") && matches!(slow_ma, "sma" | "alma");
    let vwma_pilot_pair = fast_ma == "vwma" && slow_ma == "vwma";
    if !(classic_pair || vwma_pilot_pair) {
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
fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
}

#[inline]
fn kernel_async_alloc_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_ASYNC_ALLOC", true)
}

#[inline]
fn kernel_pinned_h2d_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_PINNED_H2D", true)
}

#[inline]
fn kernel_shortlist_slice_diagnostic_row_limit() -> usize {
    env_usize("VECTORBT_KERNEL_SHORTLIST_SLICE_DIAG_ROWS").unwrap_or(8)
}

#[inline]
fn kernel_pinned_d2h_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_PINNED_D2H", true)
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CampaignVramPrefetchOutcome {
    Staged,
    AlreadyResident,
    Skipped(String),
}

#[inline]
fn resident_market_series_estimate_bytes(
    len: usize,
    has_source_override: bool,
    needs_high_low: bool,
    needs_volume: bool,
) -> usize {
    let mut series = 1usize;
    if needs_high_low {
        series = series.saturating_add(2);
    }
    if needs_volume {
        series = series.saturating_add(1);
    }
    if has_source_override {
        series = series.saturating_add(1);
    }
    series
        .saturating_mul(len)
        .saturating_mul(std::mem::size_of::<f32>())
}

#[inline]
fn resident_market_series_cached_bytes(cache: &ResidentMarketSeriesCache) -> usize {
    let mut total = cache.close.len().saturating_mul(std::mem::size_of::<f32>());
    if let Some(high) = cache.high.as_ref() {
        total = total.saturating_add(high.len().saturating_mul(std::mem::size_of::<f32>()));
    }
    if let Some(low) = cache.low.as_ref() {
        total = total.saturating_add(low.len().saturating_mul(std::mem::size_of::<f32>()));
    }
    if let Some(volume) = cache.volume.as_ref() {
        total = total.saturating_add(volume.len().saturating_mul(std::mem::size_of::<f32>()));
    }
    for buffer in cache.source_overrides.values() {
        total = total.saturating_add(buffer.len().saturating_mul(std::mem::size_of::<f32>()));
    }
    total
}

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

fn promote_matching_staged_market_series(
    active_stream: &Stream,
    staging_stream: &Stream,
    resident: &mut Option<ResidentMarketSeriesCache>,
    staged: &mut Option<ResidentMarketSeriesCache>,
    expected_signature: CandleDatasetSignature,
) -> Result<bool, String> {
    let should_promote = staged
        .as_ref()
        .map(|cache| cache.signature == expected_signature)
        .unwrap_or(false);
    if !should_promote {
        return Ok(false);
    }
    staging_stream.synchronize().map_err(|e| e.to_string())?;
    reset_resident_market_cache(active_stream, resident)?;
    *resident = staged.take();
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn stage_resident_market_series_for_next_dataset(
    resident_market: &Option<ResidentMarketSeriesCache>,
    staging_stream: &Stream,
    staged_market: &mut Option<ResidentMarketSeriesCache>,
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
) -> Result<CampaignVramPrefetchOutcome, String> {
    let signature = candle_dataset_signature(candles);
    if resident_market
        .as_ref()
        .map(|cache| cache.signature == signature)
        .unwrap_or(false)
    {
        return Ok(CampaignVramPrefetchOutcome::AlreadyResident);
    }
    if staged_market
        .as_ref()
        .map(|cache| cache.signature == signature)
        .unwrap_or(false)
    {
        return Ok(CampaignVramPrefetchOutcome::AlreadyResident);
    }

    let source_key = normalize_market_source_key(ma_source);
    let has_source_override = needs_ma_prices && source_key != "close";
    let reclaimed_staged_bytes = staged_market
        .as_ref()
        .map(resident_market_series_cached_bytes)
        .unwrap_or(0);
    let budget = vram_budget_bytes().saturating_add(reclaimed_staged_bytes);
    let estimate = resident_market_series_estimate_bytes(
        close.len(),
        has_source_override,
        needs_high_low,
        needs_volume,
    );
    if estimate > budget {
        return Ok(CampaignVramPrefetchOutcome::Skipped(format!(
            "double_ma VRAM staging skipped: estimated resident upload {} MiB exceeds available budget {} MiB",
            estimate / (1024 * 1024),
            budget / (1024 * 1024)
        )));
    }

    ensure_resident_market_series(
        staging_stream,
        staged_market,
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
    Ok(CampaignVramPrefetchOutcome::Staged)
}

fn ensure_log_and_simple_returns(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    d_close: &DeviceBuffer<f32>,
    n: usize,
    dataset_sig: CandleDatasetSignature,
    compute_returns: &Function,
    device_launch_caps: DeviceLaunchCaps,
    compute_returns_launch_caps: KernelLaunchCaps,
) -> Result<(), String> {
    KernelScratch::ensure_len_f32(&mut scratch.d_lr, n, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_ret1, n, bt_stream)?;

    let want = ReturnsCacheKey {
        dataset: dataset_sig,
    };
    if scratch.returns_key == Some(want) {
        return Ok(());
    }

    let d_lr_ptr = scratch
        .d_lr
        .as_ref()
        .ok_or_else(|| "missing cached d_lr buffer".to_string())?
        .as_device_ptr()
        .as_raw();
    let d_ret1_ptr = scratch
        .d_ret1
        .as_ref()
        .ok_or_else(|| "missing cached d_ret1 buffer".to_string())?
        .as_device_ptr()
        .as_raw();
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
            .launch(compute_returns, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
            .map_err(|e| e.to_string())?;
    }

    scratch.returns_key = Some(want);
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
    env_flag_enabled("VECTORBT_KERNEL_GLOBAL_TOPK", true)
}

#[inline]
fn kernel_score_first_topk_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_SCORE_FIRST_TOPK", false)
}

#[inline]
fn kernel_row_score_heads_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_ROW_SCORE_HEADS", false)
}

#[inline]
fn kernel_exact_pruning_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_EXACT_PRUNING", false)
}

#[inline]
fn row_score_mode_objective_index(score_mode: Option<DoubleMaScoreMode>) -> Option<i32> {
    let mode = match score_mode {
        Some(DoubleMaScoreMode::BalancedRiskAdjusted10) => 1,
        Some(DoubleMaScoreMode::AdversarialEnvelope10) => 2,
        Some(DoubleMaScoreMode::SupportShrunk10) => 3,
        Some(DoubleMaScoreMode::ActiveSortino10) => 4,
        Some(DoubleMaScoreMode::OmegaVsSharpe10) => 5,
        Some(DoubleMaScoreMode::RobustMargin10) => 6,
        Some(DoubleMaScoreMode::EffectiveSampleLcb10) | None => return None,
    };
    Some(ROW_SCORE_OBJECTIVE_BASE + mode)
}

#[inline]
fn row_score_heads_preselect_candidate(
    row_score_env_enabled: bool,
    score_mode: Option<DoubleMaScoreMode>,
    include_all: bool,
    export_enabled: bool,
    extra_shortlist_enabled: bool,
    shortlist_confidence_mode: Option<ShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<ShortlistSliceMode>,
    constraints_present: bool,
    eval_range: Option<(usize, usize)>,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    heatmap_bins: usize,
    top_k: usize,
) -> bool {
    row_score_env_enabled
        && row_score_mode_objective_index(score_mode).is_some()
        && !include_all
        && !export_enabled
        && !extra_shortlist_enabled
        && shortlist_confidence_mode.is_none()
        && shortlist_union_mode.is_none()
        && shortlist_slice_mode.is_none()
        && !constraints_present
        && eval_range.is_none()
        && !fast_precomputed_enabled
        && !slow_precomputed_enabled
        && heatmap_bins == 0
        && top_k > 0
}

#[inline]
fn max_drawdown_constraint_to_log_space(max_dd: f64) -> Option<f32> {
    if !max_dd.is_finite() || max_dd >= 1.0 {
        return None;
    }
    if max_dd <= 0.0 {
        return Some(0.0);
    }
    Some((-(1.0 - max_dd).ln()) as f32)
}

#[inline]
fn exact_pruning_max_log_dd_threshold(constraints: Option<&DoubleMaConstraints>) -> Option<f32> {
    constraints
        .and_then(|cfg| cfg.max_drawdown)
        .and_then(max_drawdown_constraint_to_log_space)
        .filter(|v| v.is_finite())
}

fn exact_pruning_blockers(
    exact_pruning_env_enabled: bool,
    global_topk_mode: bool,
    exact_pruning_symbols_available: bool,
    exact_pruning_max_log_dd: Option<f32>,
    constraints_on_device: bool,
    device_include_all: bool,
    export_enabled: bool,
    host_filter_mode: bool,
    eval_range: Option<(usize, usize)>,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    heatmap_bins: usize,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if !exact_pruning_env_enabled {
        blockers.push("env_disabled");
    }
    if !global_topk_mode {
        blockers.push("global_topk_disabled");
    }
    if !exact_pruning_symbols_available {
        blockers.push("exact_pruning_symbols_unavailable");
    }
    if exact_pruning_max_log_dd.is_none() {
        blockers.push("no_prunable_max_drawdown");
    }
    if !constraints_on_device {
        blockers.push("device_constraints_unavailable");
    }
    if device_include_all {
        blockers.push("device_include_all");
    }
    if export_enabled {
        blockers.push("export_enabled");
    }
    if host_filter_mode {
        blockers.push("host_filter_mode");
    }
    if eval_range.is_some() {
        blockers.push("eval_range_set");
    }
    if fast_precomputed_enabled {
        blockers.push("fast_precomputed_enabled");
    }
    if slow_precomputed_enabled {
        blockers.push("slow_precomputed_enabled");
    }
    if heatmap_bins != 0 {
        blockers.push("heatmap_bins_nonzero");
    }
    blockers
}

#[inline]
fn should_use_exact_pruning_mode(
    global_topk_mode: bool,
    exact_pruning_symbols_available: bool,
    exact_pruning_max_log_dd: Option<f32>,
    constraints_on_device: bool,
    device_include_all: bool,
    export_enabled: bool,
    host_filter_mode: bool,
    eval_range: Option<(usize, usize)>,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    heatmap_bins: usize,
) -> bool {
    exact_pruning_blockers(
        kernel_exact_pruning_enabled(),
        global_topk_mode,
        exact_pruning_symbols_available,
        exact_pruning_max_log_dd,
        constraints_on_device,
        device_include_all,
        export_enabled,
        host_filter_mode,
        eval_range,
        fast_precomputed_enabled,
        slow_precomputed_enabled,
        heatmap_bins,
    )
    .is_empty()
}

fn score_first_topk_blockers(
    score_first_env_enabled: bool,
    global_topk_mode: bool,
    score_first_symbols_available: bool,
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_enabled: bool,
    shortlist_confidence_mode: Option<ShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<ShortlistSliceMode>,
    metrics_count: usize,
    constraints_present: bool,
    eval_range: Option<(usize, usize)>,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    heatmap_bins: usize,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if !score_first_env_enabled {
        blockers.push("env_disabled");
    }
    if !global_topk_mode {
        blockers.push("global_topk_disabled");
    }
    if !score_first_symbols_available {
        blockers.push("score_first_symbols_unavailable");
    }
    if score_mode.is_some() {
        blockers.push("score_mode_active");
    }
    if extra_shortlist_enabled {
        blockers.push("extra_shortlist_enabled");
    }
    if shortlist_confidence_mode.is_some() {
        blockers.push("shortlist_confidence_enabled");
    }
    if shortlist_union_mode.is_some() {
        blockers.push("shortlist_union_enabled");
    }
    if shortlist_slice_mode.is_some() {
        blockers.push("shortlist_slice_enabled");
    }
    if metrics_count != METRICS_COUNT && metrics_count != TRADE_METRICS_COUNT {
        blockers.push("metrics_contract_not_full_row");
    }
    if constraints_present {
        blockers.push("constraints_present");
    }
    if eval_range.is_some() {
        blockers.push("eval_range_set");
    }
    if fast_precomputed_enabled {
        blockers.push("fast_precomputed_enabled");
    }
    if slow_precomputed_enabled {
        blockers.push("slow_precomputed_enabled");
    }
    if heatmap_bins != 0 {
        blockers.push("heatmap_bins_nonzero");
    }
    blockers
}

#[inline]
fn should_use_score_first_topk_mode(
    global_topk_mode: bool,
    score_first_symbols_available: bool,
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_enabled: bool,
    shortlist_confidence_mode: Option<ShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<ShortlistSliceMode>,
    metrics_count: usize,
    constraints_present: bool,
    eval_range: Option<(usize, usize)>,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    heatmap_bins: usize,
) -> bool {
    score_first_topk_blockers(
        kernel_score_first_topk_enabled(),
        global_topk_mode,
        score_first_symbols_available,
        score_mode,
        extra_shortlist_enabled,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        metrics_count,
        constraints_present,
        eval_range,
        fast_precomputed_enabled,
        slow_precomputed_enabled,
        heatmap_bins,
    )
    .is_empty()
}

fn row_score_heads_blockers(
    row_score_env_enabled: bool,
    global_topk_mode: bool,
    score_mode: Option<DoubleMaScoreMode>,
    metrics_count: usize,
    include_all: bool,
    export_enabled: bool,
    extra_shortlist_enabled: bool,
    shortlist_confidence_mode: Option<ShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<ShortlistSliceMode>,
    constraints_present: bool,
    eval_range: Option<(usize, usize)>,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    heatmap_bins: usize,
    top_k: usize,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if !row_score_env_enabled {
        blockers.push("env_disabled");
    }
    if !global_topk_mode {
        blockers.push("global_topk_disabled");
    }
    if score_mode.is_none() {
        blockers.push("score_mode_unset");
    } else if row_score_mode_objective_index(score_mode).is_none() {
        blockers.push("score_mode_not_row_only");
    }
    if metrics_count < PRODUCT_REDUCED_METRICS_COUNT {
        blockers.push("metrics_contract_below_product10");
    }
    if include_all {
        blockers.push("include_all");
    }
    if export_enabled {
        blockers.push("export_enabled");
    }
    if extra_shortlist_enabled {
        blockers.push("extra_shortlist_enabled");
    }
    if shortlist_confidence_mode.is_some() {
        blockers.push("shortlist_confidence_enabled");
    }
    if shortlist_union_mode.is_some() {
        blockers.push("shortlist_union_enabled");
    }
    if shortlist_slice_mode.is_some() {
        blockers.push("shortlist_slice_enabled");
    }
    if constraints_present {
        blockers.push("constraints_present");
    }
    if eval_range.is_some() {
        blockers.push("eval_range_set");
    }
    if fast_precomputed_enabled {
        blockers.push("fast_precomputed_enabled");
    }
    if slow_precomputed_enabled {
        blockers.push("slow_precomputed_enabled");
    }
    if heatmap_bins != 0 {
        blockers.push("heatmap_bins_nonzero");
    }
    if top_k == 0 {
        blockers.push("top_k_zero");
    }
    blockers
}

#[inline]
fn should_use_row_score_heads_mode(
    global_topk_mode: bool,
    score_mode: Option<DoubleMaScoreMode>,
    metrics_count: usize,
    include_all: bool,
    export_enabled: bool,
    extra_shortlist_enabled: bool,
    shortlist_confidence_mode: Option<ShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<ShortlistSliceMode>,
    constraints_present: bool,
    eval_range: Option<(usize, usize)>,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    heatmap_bins: usize,
    top_k: usize,
) -> bool {
    row_score_heads_blockers(
        kernel_row_score_heads_enabled(),
        global_topk_mode,
        score_mode,
        metrics_count,
        include_all,
        export_enabled,
        extra_shortlist_enabled,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        constraints_present,
        eval_range,
        fast_precomputed_enabled,
        slow_precomputed_enabled,
        heatmap_bins,
        top_k,
    )
    .is_empty()
}

fn build_gpu_kernel_route_diagnostics(
    device_include_all: bool,
    export_enabled: bool,
    host_filter_mode: bool,
    compact_valid_pair_output: bool,
    global_topk_mode: bool,
    score_first_env_enabled: bool,
    score_first_symbols_available: bool,
    score_first_topk_mode: bool,
    row_score_heads_env_enabled: bool,
    row_score_heads_mode: bool,
    row_score_heads_objective_i: Option<i32>,
    exact_pruning_env_enabled: bool,
    exact_pruning_symbols_available: bool,
    exact_pruning_mode: bool,
    exact_pruning_max_log_dd: Option<f32>,
    metrics_count: usize,
    tile_metrics_stride: usize,
    score_first_blockers: &[&str],
    row_score_heads_blockers: &[&str],
    exact_pruning_blockers: &[&str],
) -> Vec<String> {
    let path = if device_include_all {
        "device_include_all"
    } else if export_enabled {
        "export_all"
    } else if host_filter_mode {
        "host_filter_mode"
    } else if global_topk_mode && row_score_heads_mode {
        "row_first_global_topk_row_score_heads"
    } else if global_topk_mode && exact_pruning_mode {
        "row_first_global_topk_exact_pruning"
    } else if global_topk_mode && score_first_topk_mode {
        "score_first_global_topk"
    } else if global_topk_mode {
        "row_first_global_topk"
    } else {
        "host_merge_topk"
    };

    let score_first_blockers = if score_first_blockers.is_empty() {
        "none".to_string()
    } else {
        score_first_blockers.join(",")
    };
    let row_score_heads_blockers = if row_score_heads_blockers.is_empty() {
        "none".to_string()
    } else {
        row_score_heads_blockers.join(",")
    };
    let exact_pruning_blockers = if exact_pruning_blockers.is_empty() {
        "none".to_string()
    } else {
        exact_pruning_blockers.join(",")
    };
    let exact_pruning_max_log_dd = exact_pruning_max_log_dd
        .map(|v| v.to_string())
        .unwrap_or_else(|| "none".to_string());
    let row_score_heads_objective_i = row_score_heads_objective_i
        .map(|v| v.to_string())
        .unwrap_or_else(|| "none".to_string());

    vec![
        format!("gpu_kernel.path={path}"),
        format!("gpu_kernel.device_include_all={device_include_all}"),
        format!("gpu_kernel.export_enabled={export_enabled}"),
        format!("gpu_kernel.host_filter_mode={host_filter_mode}"),
        format!("gpu_kernel.compact_valid_pair_output={compact_valid_pair_output}"),
        format!("gpu_kernel.global_topk_mode={global_topk_mode}"),
        format!("gpu_kernel.score_first_env_enabled={score_first_env_enabled}"),
        format!("gpu_kernel.score_first_symbols_available={score_first_symbols_available}"),
        format!("gpu_kernel.score_first_mode={score_first_topk_mode}"),
        format!("gpu_kernel.score_first_blockers={score_first_blockers}"),
        format!("gpu_kernel.row_score_heads_env_enabled={row_score_heads_env_enabled}"),
        format!("gpu_kernel.row_score_heads_mode={row_score_heads_mode}"),
        format!("gpu_kernel.row_score_heads_objective_i={row_score_heads_objective_i}"),
        format!("gpu_kernel.row_score_heads_blockers={row_score_heads_blockers}"),
        format!("gpu_kernel.exact_pruning_env_enabled={exact_pruning_env_enabled}"),
        format!("gpu_kernel.exact_pruning_symbols_available={exact_pruning_symbols_available}"),
        format!("gpu_kernel.exact_pruning_mode={exact_pruning_mode}"),
        format!("gpu_kernel.exact_pruning_blockers={exact_pruning_blockers}"),
        format!("gpu_kernel.exact_pruning_max_log_dd={exact_pruning_max_log_dd}"),
        format!("gpu_kernel.metrics_count={metrics_count}"),
        format!("gpu_kernel.tile_metrics_stride={tile_metrics_stride}"),
    ]
}

#[inline]
fn objective_kind_kernel_index(objective: ObjectiveKind) -> i32 {
    match objective {
        ObjectiveKind::Pnl => 0,
        ObjectiveKind::Sharpe => 1,
        ObjectiveKind::MaxDrawdown => 2,
        ObjectiveKind::Calmar => 3,
        ObjectiveKind::Volatility => 4,
        ObjectiveKind::Sortino => 5,
        ObjectiveKind::Omega => 6,
    }
}

fn push_extra_shortlist_objective(
    out: &mut Vec<ObjectiveKind>,
    primary: ObjectiveKind,
    objective: ObjectiveKind,
    source: &str,
) -> Result<(), String> {
    if objective == primary || out.contains(&objective) {
        return Ok(());
    }
    out.push(objective);
    if out.len() > EXTRA_SHORTLIST_MAX_HEADS {
        return Err(format!(
            "{source} supports at most {EXTRA_SHORTLIST_MAX_HEADS} extra objectives"
        ));
    }
    Ok(())
}

fn kernel_extra_shortlist_objectives(
    primary: ObjectiveKind,
    requested: Option<&[ObjectiveKind]>,
) -> Result<Vec<ObjectiveKind>, String> {
    let mut out = Vec::new();
    if let Some(requested) = requested {
        for &objective in requested {
            push_extra_shortlist_objective(
                &mut out,
                primary,
                objective,
                EXTRA_SHORTLIST_REQUEST_FIELD,
            )?;
        }
    }
    Ok(out)
}

struct ExtraShortlistHeadState {
    objective: ObjectiveKind,
    objective_i: i32,
    global_fast_rows: Option<DeviceBuffer<i32>>,
    global_slow_rows: Option<DeviceBuffer<i32>>,
    global_metrics_rows: Option<DeviceBuffer<f32>>,
    global_rows_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShortlistConfidenceMode {
    Psr,
    Dsr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShortlistSliceMode {
    Halves,
    Quartiles,
}

impl ShortlistSliceMode {
    #[inline]
    fn slice_count(self) -> usize {
        match self {
            ShortlistSliceMode::Halves => 2,
            ShortlistSliceMode::Quartiles => 4,
        }
    }

    #[inline]
    fn as_str(self) -> &'static str {
        match self {
            ShortlistSliceMode::Halves => "halves",
            ShortlistSliceMode::Quartiles => "quartiles",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ShortlistSliceOmissionSummary {
    best_label: String,
    best_score: f64,
    worst_label: String,
    worst_score: f64,
    spread: f64,
}

#[derive(Clone, Debug)]
struct ShortlistSliceModeCandidateRecord {
    params: DoubleMaParams,
    full_metrics: Metrics,
    diagnostics: ShortlistSliceModeDiagnostics,
}

#[inline]
fn shortlist_slice_label_prefix(mode: ShortlistSliceMode) -> &'static str {
    match mode {
        ShortlistSliceMode::Halves => "half_",
        ShortlistSliceMode::Quartiles => "q",
    }
}

fn shortlist_slice_labels(mode: ShortlistSliceMode) -> Vec<String> {
    let prefix = shortlist_slice_label_prefix(mode);
    (0..mode.slice_count())
        .map(|idx| format!("{prefix}{}", idx + 1))
        .collect()
}

fn compute_shortlist_slice_omission_summary(
    slice_scores: &[f64],
    labels: &[String],
) -> Result<ShortlistSliceOmissionSummary, String> {
    if slice_scores.len() < 2 || slice_scores.len() != labels.len() {
        return Err("slice omission summary requires >= 2 labeled slice scores".to_string());
    }
    let denom = (slice_scores.len() - 1) as f64;
    let total_score: f64 = slice_scores.iter().copied().sum();
    let mut best_idx = 0usize;
    let mut worst_idx = 0usize;
    let mut best_score = f64::NEG_INFINITY;
    let mut worst_score = f64::INFINITY;
    for (idx, &score) in slice_scores.iter().enumerate() {
        let omission_score = (total_score - score) / denom;
        if omission_score > best_score {
            best_score = omission_score;
            best_idx = idx;
        }
        if omission_score < worst_score {
            worst_score = omission_score;
            worst_idx = idx;
        }
    }
    Ok(ShortlistSliceOmissionSummary {
        best_label: labels[best_idx].clone(),
        best_score,
        worst_label: labels[worst_idx].clone(),
        worst_score,
        spread: best_score - worst_score,
    })
}

#[inline]
fn shortlist_slice_benchmark_label(benchmark: &SpaBenchmarkKind) -> &'static str {
    match benchmark {
        SpaBenchmarkKind::Flat => "flat",
        SpaBenchmarkKind::BuyAndHold => "buy_and_hold",
    }
}

#[inline]
fn shortlist_slice_benchmark_step_return(lr: f64, benchmark: &SpaBenchmarkKind) -> f64 {
    match benchmark {
        SpaBenchmarkKind::Flat => 0.0,
        SpaBenchmarkKind::BuyAndHold => lr.exp_m1(),
    }
}

fn compute_shortlist_slice_benchmark_relative_summary(
    slice_excess_returns: &[f64],
    labels: &[String],
    benchmark: &SpaBenchmarkKind,
) -> Result<ShortlistSliceBenchmarkRelativeSummary, String> {
    if slice_excess_returns.len() < 2 || slice_excess_returns.len() != labels.len() {
        return Err(
            "shortlist benchmark-relative summary requires >= 2 labeled slice returns".to_string(),
        );
    }

    let total_excess: f64 = slice_excess_returns.iter().copied().sum();
    let slice_count = slice_excess_returns.len();
    let beat_rate = slice_excess_returns
        .iter()
        .filter(|&&value| value > 1.0e-12)
        .count() as f64
        / slice_count as f64;
    let mean_excess_return = total_excess / slice_count as f64;
    let best_excess_return = slice_excess_returns
        .iter()
        .copied()
        .max_by(|a, b| a.total_cmp(b))
        .ok_or_else(|| "missing best shortlist benchmark-relative slice".to_string())?;
    let worst_excess_return = slice_excess_returns
        .iter()
        .copied()
        .min_by(|a, b| a.total_cmp(b))
        .ok_or_else(|| "missing worst shortlist benchmark-relative slice".to_string())?;
    let omission = compute_shortlist_slice_omission_summary(slice_excess_returns, labels)?;

    Ok(ShortlistSliceBenchmarkRelativeSummary {
        benchmark: shortlist_slice_benchmark_label(benchmark).to_string(),
        beat_rate,
        mean_excess_return,
        worst_excess_return,
        best_excess_return,
        best_omission_excess_return: omission.best_score,
        worst_omission_excess_return: omission.worst_score,
        omission_excess_spread: omission.spread,
    })
}

#[inline]
fn shortlist_slice_candidate_return_sum(metrics: &Metrics, obs: usize) -> f64 {
    if obs == 0 {
        return 0.0;
    }
    if metrics.volatility > 1.0e-12 {
        return derived_mean(metrics) * obs as f64;
    }

    let base = (1.0 + metrics.pnl).max(0.0);
    let step_return = if base > 0.0 {
        base.powf(1.0 / obs as f64) - 1.0
    } else {
        -1.0
    };
    step_return * obs as f64
}

fn kernel_shortlist_confidence_mode(
    requested: Option<DoubleMaShortlistConfidenceMode>,
) -> Result<Option<ShortlistConfidenceMode>, String> {
    Ok(requested.map(|mode| match mode {
        DoubleMaShortlistConfidenceMode::Psr => ShortlistConfidenceMode::Psr,
        DoubleMaShortlistConfidenceMode::Dsr => ShortlistConfidenceMode::Dsr,
    }))
}

fn kernel_shortlist_union_mode(
    requested: Option<DoubleMaShortlistUnionMode>,
) -> Result<Option<DoubleMaShortlistUnionMode>, String> {
    Ok(requested)
}

fn kernel_shortlist_slice_mode(
    requested: Option<DoubleMaShortlistSliceMode>,
) -> Result<Option<ShortlistSliceMode>, String> {
    Ok(requested.map(|mode| match mode {
        DoubleMaShortlistSliceMode::Halves => ShortlistSliceMode::Halves,
        DoubleMaShortlistSliceMode::Quartiles => ShortlistSliceMode::Quartiles,
    }))
}

fn kernel_shortlist_slice_benchmark(
    requested: Option<SpaBenchmarkKind>,
) -> Result<Option<SpaBenchmarkKind>, String> {
    Ok(requested)
}

#[inline]
fn shortlist_candidate_track_record_bars(
    first_valid: usize,
    regime_first_valid: usize,
    eval_start: usize,
    eval_end: usize,
    total_bars: usize,
    fast_len: u16,
    slow_len: u16,
    trade_on_next_bar: bool,
) -> usize {
    let pair_first_valid = first_valid.saturating_add(fast_len.max(slow_len) as usize - 1);
    let t_valid = pair_first_valid.max(regime_first_valid).max(eval_start);
    let trade_offset = usize::from(trade_on_next_bar);
    let start_t = t_valid.saturating_add(trade_offset).max(eval_start);
    let end_t = total_bars.min(eval_end);
    end_t.saturating_sub(start_t).max(2)
}

struct EffectiveSampleScoreContextGuard {
    prev: Option<EffectiveSampleScoreContext>,
}

impl Drop for EffectiveSampleScoreContextGuard {
    fn drop(&mut self) {
        EFFECTIVE_SAMPLE_SCORE_CONTEXT.with(|cell| {
            *cell.borrow_mut() = self.prev.take();
        });
    }
}

fn install_effective_sample_score_context(
    ctx: EffectiveSampleScoreContext,
) -> EffectiveSampleScoreContextGuard {
    let prev = EFFECTIVE_SAMPLE_SCORE_CONTEXT.with(|cell| {
        let mut slot = cell.borrow_mut();
        let prev = *slot;
        *slot = Some(ctx);
        prev
    });
    EffectiveSampleScoreContextGuard { prev }
}

fn effective_sample_lcb_10_pair_score(params: &DoubleMaParams, metrics: &Metrics) -> f64 {
    EFFECTIVE_SAMPLE_SCORE_CONTEXT.with(|cell| {
        let Some(ctx) = *cell.borrow() else {
            debug_assert!(
                false,
                "effective_sample_lcb_10_pair_score requires installed observation context"
            );
            return effective_sample_lcb_10_score(metrics, 2);
        };
        let n_obs = shortlist_candidate_track_record_bars(
            ctx.first_valid,
            ctx.regime_first_valid,
            ctx.eval_start,
            ctx.eval_end,
            ctx.total_bars,
            params.fast_len,
            params.slow_len,
            ctx.trade_on_next_bar,
        );
        effective_sample_lcb_10_score(metrics, n_obs)
    })
}

fn kernel_effective_score_for_pair(
    params: &DoubleMaParams,
    metrics: &Metrics,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
) -> f64 {
    if matches!(score_mode, Some(DoubleMaScoreMode::EffectiveSampleLcb10)) {
        let score = effective_sample_lcb_10_pair_score(params, metrics);
        if score.is_finite() {
            score
        } else {
            f64::NEG_INFINITY
        }
    } else {
        effective_score(metrics, objective, score_mode)
    }
}

fn shortlist_slice_ranges(
    rows: &[(DoubleMaParams, Metrics)],
    mode: ShortlistSliceMode,
    first_valid: usize,
    regime_first_valid: usize,
    eval_start: usize,
    eval_end: usize,
    total_bars: usize,
    strategy: &StrategyConfig,
) -> Result<Vec<(usize, usize)>, String> {
    if rows.is_empty() {
        return Err("slice robustness requires a non-empty shortlist".to_string());
    }
    let trade_offset = usize::from(strategy.trade_on_next_bar);
    let common_start = rows
        .iter()
        .map(|(params, _)| {
            let pair_first_valid =
                first_valid.saturating_add(params.fast_len.max(params.slow_len) as usize - 1);
            pair_first_valid
                .max(regime_first_valid)
                .max(eval_start)
                .saturating_add(trade_offset)
                .max(eval_start)
        })
        .max()
        .unwrap_or(eval_start);
    let end_t = total_bars.min(eval_end);
    let slice_count = mode.slice_count();
    let span = end_t.saturating_sub(common_start);
    let slice_bars = span / slice_count;
    if slice_bars < 2 {
        return Err(format!(
            "{SHORTLIST_SLICE_MODE_REQUEST_FIELD} requires at least 2 bars per slice after warmup (common_start={common_start}, end={end_t}, slices={slice_count})"
        ));
    }

    let mut ranges = Vec::with_capacity(slice_count);
    for slice_idx in 0..slice_count {
        let start = common_start + slice_idx * slice_bars;
        let end = if slice_idx + 1 == slice_count {
            end_t
        } else {
            common_start + (slice_idx + 1) * slice_bars
        };
        if end <= start {
            return Err(format!(
                "{SHORTLIST_SLICE_MODE_REQUEST_FIELD} produced an empty slice at index {slice_idx}"
            ));
        }
        ranges.push((start, end));
    }
    Ok(ranges)
}

fn rerank_shortlist_by_confidence(
    rows: &[(DoubleMaParams, Metrics)],
    mode: ShortlistConfidenceMode,
    first_valid: usize,
    regime_first_valid: usize,
    eval_start: usize,
    eval_end: usize,
    total_bars: usize,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
) -> Vec<(DoubleMaParams, Metrics)> {
    if rows.is_empty() {
        return Vec::new();
    }

    let sharpe_peers: Vec<f64> = rows
        .iter()
        .map(|(_, metrics)| metrics.sharpe)
        .filter(|value| value.is_finite())
        .collect();
    let (peer_mean, peer_std) = crate::sharpe_stats::mean_std(&sharpe_peers);
    let trials = sharpe_peers.len().max(1);

    let mut ranked: Vec<(DoubleMaParams, Metrics, f64)> = rows
        .iter()
        .cloned()
        .map(|(params, metrics)| {
            let obs = shortlist_candidate_track_record_bars(
                first_valid,
                regime_first_valid,
                eval_start,
                eval_end,
                total_bars,
                params.fast_len,
                params.slow_len,
                strategy.trade_on_next_bar,
            );
            let (psr, dsr) = crate::validation_stats::compute_psr_dsr(
                metrics.sharpe,
                obs,
                trials,
                peer_mean,
                peer_std,
            );
            let confidence = match mode {
                ShortlistConfidenceMode::Psr => psr.unwrap_or(0.0),
                ShortlistConfidenceMode::Dsr => dsr.unwrap_or(0.0),
            };
            (params, metrics, confidence)
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.2.total_cmp(&a.2)
            .then_with(|| {
                let sa = effective_score(&a.1, objective, score_mode);
                let sb = effective_score(&b.1, objective, score_mode);
                sb.total_cmp(&sa)
            })
            .then_with(|| a.1.max_dd.total_cmp(&b.1.max_dd))
    });

    ranked
        .into_iter()
        .map(|(params, metrics, _)| (params, metrics))
        .collect()
}

pub(crate) fn rerank_shortlist_by_confidence_for_host(
    rows: &[(DoubleMaParams, Metrics)],
    mode: DoubleMaShortlistConfidenceMode,
    ctx: PairObservationScoreContextResolved,
    strategy: &StrategyConfig,
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
) -> Vec<(DoubleMaParams, Metrics)> {
    let mode = match mode {
        DoubleMaShortlistConfidenceMode::Psr => ShortlistConfidenceMode::Psr,
        DoubleMaShortlistConfidenceMode::Dsr => ShortlistConfidenceMode::Dsr,
    };
    rerank_shortlist_by_confidence(
        rows,
        mode,
        ctx.first_valid,
        ctx.regime_first_valid,
        ctx.eval_start,
        ctx.eval_end,
        ctx.total_bars,
        strategy,
        objective,
        score_mode,
    )
}

pub(crate) fn shortlist_confidence_value_for_host(
    metrics: &Metrics,
    mode: DoubleMaShortlistConfidenceMode,
    obs: usize,
    peer_mean: f64,
    peer_std: f64,
    trials: usize,
) -> f64 {
    let (psr, dsr) = crate::validation_stats::compute_psr_dsr(
        metrics.sharpe,
        obs.max(2),
        trials.max(1),
        peer_mean,
        peer_std,
    );
    match mode {
        DoubleMaShortlistConfidenceMode::Psr => psr.unwrap_or(0.0),
        DoubleMaShortlistConfidenceMode::Dsr => dsr.unwrap_or(0.0),
    }
}

fn shortlist_union_objectives(
    primary_objective: ObjectiveKind,
    extra_objectives: &[ObjectiveKind],
) -> Vec<ObjectiveKind> {
    let mut objectives = Vec::with_capacity(extra_objectives.len().saturating_add(1));
    objectives.push(primary_objective);
    for &objective in extra_objectives {
        if objective != primary_objective && !objectives.contains(&objective) {
            objectives.push(objective);
        }
    }
    objectives
}

fn shortlist_union_score_rows(
    rows: &[(DoubleMaParams, Metrics)],
    objectives: &[ObjectiveKind],
) -> Vec<Vec<f64>> {
    rows.iter()
        .map(|(_, metrics)| {
            objectives
                .iter()
                .map(|&objective| objective_score(metrics, objective))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn shortlist_union_epsilon(score_rows: &[Vec<f64>]) -> Vec<f64> {
    if score_rows.is_empty() {
        return Vec::new();
    }
    let width = score_rows[0].len();
    let mut mins = vec![f64::INFINITY; width];
    let mut maxs = vec![f64::NEG_INFINITY; width];
    for scores in score_rows {
        for (idx, &score) in scores.iter().enumerate() {
            mins[idx] = mins[idx].min(score);
            maxs[idx] = maxs[idx].max(score);
        }
    }
    mins.iter()
        .zip(maxs.iter())
        .map(|(&min_score, &max_score)| {
            let span = (max_score - min_score).abs();
            let scale = min_score.abs().max(max_score.abs()).max(1.0);
            span * 0.01 + scale * 1.0e-6
        })
        .collect()
}

fn shortlist_union_dominates(a: &[f64], b: &[f64], eps: &[f64]) -> bool {
    let mut strictly_better = false;
    for idx in 0..a.len().min(b.len()).min(eps.len()) {
        if a[idx] + eps[idx] < b[idx] {
            return false;
        }
        if a[idx] > b[idx] + eps[idx] {
            strictly_better = true;
        }
    }
    strictly_better
}

fn shortlist_union_front_ranks(score_rows: &[Vec<f64>], eps: &[f64]) -> Vec<usize> {
    let n = score_rows.len();
    let mut dominates: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut dominated_by_count: Vec<usize> = vec![0; n];
    let mut front_ranks: Vec<usize> = vec![usize::MAX; n];
    let mut current_front: Vec<usize> = Vec::new();

    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            if shortlist_union_dominates(&score_rows[i], &score_rows[j], eps) {
                dominates[i].push(j);
            } else if shortlist_union_dominates(&score_rows[j], &score_rows[i], eps) {
                dominated_by_count[i] = dominated_by_count[i].saturating_add(1);
            }
        }
        if dominated_by_count[i] == 0 {
            front_ranks[i] = 0;
            current_front.push(i);
        }
    }

    let mut depth = 0usize;
    while !current_front.is_empty() {
        let mut next_front: Vec<usize> = Vec::new();
        for &idx in current_front.iter() {
            for &dominated_idx in dominates[idx].iter() {
                dominated_by_count[dominated_idx] =
                    dominated_by_count[dominated_idx].saturating_sub(1);
                if dominated_by_count[dominated_idx] == 0
                    && front_ranks[dominated_idx] == usize::MAX
                {
                    front_ranks[dominated_idx] = depth.saturating_add(1);
                    next_front.push(dominated_idx);
                }
            }
        }
        depth = depth.saturating_add(1);
        current_front = next_front;
    }

    for rank in front_ranks.iter_mut() {
        if *rank == usize::MAX {
            *rank = depth;
        }
    }
    front_ranks
}

fn shortlist_union_rank_rows(score_rows: &[Vec<f64>]) -> Vec<Vec<usize>> {
    if score_rows.is_empty() {
        return Vec::new();
    }
    let height = score_rows.len();
    let width = score_rows[0].len();
    let mut ranks = vec![vec![0usize; width]; height];
    for col in 0..width {
        let mut ordered: Vec<(usize, f64)> = score_rows
            .iter()
            .enumerate()
            .map(|(row_idx, scores)| (row_idx, scores.get(col).copied().unwrap_or(0.0)))
            .collect();
        ordered.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut current_rank = 0usize;
        let mut previous_score: Option<f64> = None;
        for (position, (row_idx, score)) in ordered.into_iter().enumerate() {
            if let Some(previous) = previous_score {
                if score.total_cmp(&previous) != std::cmp::Ordering::Equal {
                    current_rank = position;
                }
            }
            ranks[row_idx][col] = current_rank;
            previous_score = Some(score);
        }
    }
    ranks
}

fn shortlist_union_rank_weights(width: usize) -> Vec<f64> {
    (0..width).map(|idx| 1.0 / (idx as f64 + 1.0)).collect()
}

fn shortlist_union_weighted_rank_penalties(score_rows: &[Vec<f64>]) -> Vec<f64> {
    if score_rows.is_empty() {
        return Vec::new();
    }
    let rank_rows = shortlist_union_rank_rows(score_rows);
    let weights = shortlist_union_rank_weights(score_rows[0].len());
    rank_rows
        .into_iter()
        .map(|row_ranks| {
            row_ranks
                .iter()
                .zip(weights.iter())
                .map(|(&rank, &weight)| rank as f64 * weight)
                .sum::<f64>()
        })
        .collect()
}

pub(crate) fn rerank_shortlist_by_union_mode(
    rows: &[(DoubleMaParams, Metrics)],
    primary_objective: ObjectiveKind,
    extra_objectives: &[ObjectiveKind],
    mode: DoubleMaShortlistUnionMode,
) -> Vec<(DoubleMaParams, Metrics)> {
    if rows.is_empty() {
        return Vec::new();
    }

    let objectives = shortlist_union_objectives(primary_objective, extra_objectives);
    if objectives.len() <= 1 {
        return rows.to_vec();
    }

    let score_rows = shortlist_union_score_rows(rows, objectives.as_slice());
    let eps = if matches!(mode, DoubleMaShortlistUnionMode::EpsilonFront) {
        shortlist_union_epsilon(&score_rows)
    } else {
        vec![0.0; objectives.len()]
    };
    let front_ranks = if matches!(
        mode,
        DoubleMaShortlistUnionMode::Pareto | DoubleMaShortlistUnionMode::EpsilonFront
    ) {
        shortlist_union_front_ranks(&score_rows, &eps)
    } else {
        vec![0usize; rows.len()]
    };
    let weighted_rank_penalties = if matches!(mode, DoubleMaShortlistUnionMode::WeightedRank) {
        shortlist_union_weighted_rank_penalties(&score_rows)
    } else {
        vec![0.0; rows.len()]
    };

    let mut ranked: Vec<(DoubleMaParams, Metrics, Vec<f64>, usize, f64)> = rows
        .iter()
        .cloned()
        .zip(score_rows)
        .zip(front_ranks)
        .zip(weighted_rank_penalties)
        .map(
            |((((params, metrics), scores), front_rank), weighted_rank_penalty)| {
                (params, metrics, scores, front_rank, weighted_rank_penalty)
            },
        )
        .collect();

    ranked.sort_by(|a, b| {
        if matches!(mode, DoubleMaShortlistUnionMode::WeightedRank) {
            let weighted_rank_cmp = a.4.total_cmp(&b.4);
            if weighted_rank_cmp != std::cmp::Ordering::Equal {
                return weighted_rank_cmp;
            }
        }
        if !matches!(mode, DoubleMaShortlistUnionMode::Lexicographic) {
            let front_cmp = a.3.cmp(&b.3);
            if front_cmp != std::cmp::Ordering::Equal {
                return front_cmp;
            }
        }
        for idx in 0..a.2.len().min(b.2.len()) {
            let cmp = b.2[idx].total_cmp(&a.2[idx]);
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        a.1.max_dd.total_cmp(&b.1.max_dd)
    });

    ranked
        .into_iter()
        .map(|(params, metrics, _, _, _)| (params, metrics))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn compute_shortlist_slice_mode_records(
    rt: &mut CudaKernelRuntime,
    candles: &Candles,
    rows: &[(DoubleMaParams, Metrics)],
    mode: ShortlistSliceMode,
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
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    benchmark: Option<&SpaBenchmarkKind>,
    first_valid: usize,
    regime_first_valid: usize,
    eval_start: usize,
    eval_end: usize,
    mut timing: Option<&mut ShortlistSliceTimingProfile>,
) -> Result<Vec<ShortlistSliceModeCandidateRecord>, String> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let compute_t0 = Instant::now();
    let ranges = shortlist_slice_ranges(
        rows,
        mode,
        first_valid,
        regime_first_valid,
        eval_start,
        eval_end,
        candles.close.len(),
        strategy,
    )?;
    let slice_count = ranges.len();
    let slice_labels = shortlist_slice_labels(mode);
    let benchmark_slice_returns = benchmark.map(|benchmark| {
        let lr = crate::double_ma::compute_log_returns_f64(source_type(candles, "close"));
        ranges
            .iter()
            .map(|&(start_bar, end_bar)| {
                lr[start_bar..end_bar]
                    .iter()
                    .copied()
                    .map(|value| shortlist_slice_benchmark_step_return(value, benchmark))
                    .sum::<f64>()
            })
            .collect::<Vec<_>>()
    });
    let combos: Vec<DoubleMaParams> = rows.iter().map(|(params, _)| params.clone()).collect();
    let combo_count = combos.len();

    let mut fast_periods: Vec<u16> = combos.iter().map(|params| params.fast_len).collect();
    fast_periods.sort_unstable();
    fast_periods.dedup();
    let mut slow_periods: Vec<u16> = combos.iter().map(|params| params.slow_len).collect();
    slow_periods.sort_unstable();
    slow_periods.dedup();

    let replay_descriptor_prep_elapsed = compute_t0.elapsed();
    let replay_t0 = Instant::now();
    let range_metrics = eval_double_ma_batch_gpu_kernel_impl(
        rt,
        candles,
        combos.as_slice(),
        fast_periods.as_slice(),
        slow_periods.as_slice(),
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        cfg,
        fast_precomputed,
        slow_precomputed,
        strategy,
        true,
        ranges.as_slice(),
        None,
        true,
        timing.as_deref_mut(),
    )?;
    let replay_elapsed = replay_t0.elapsed();
    if range_metrics.len() != combo_count.saturating_mul(slice_count) {
        return Err(format!(
            "slice robustness expected {} metrics for {} combos across {} slices, got {}",
            combo_count.saturating_mul(slice_count),
            combo_count,
            slice_count,
            range_metrics.len()
        ));
    }

    let mut records = Vec::with_capacity(combo_count);
    for (combo_idx, (params, metrics)) in rows.iter().cloned().enumerate() {
        let mut worst_slice_score = f64::INFINITY;
        let mut positive_slice_count = 0usize;
        let mut mean_slice_score = 0.0f64;
        let mut slice_scores: Vec<f64> = Vec::with_capacity(slice_count);
        let mut slice_excess_returns: Vec<f64> = Vec::with_capacity(slice_count);
        let mut slices = Vec::with_capacity(slice_count);
        for slice_idx in 0..slice_count {
            let slice_metrics = &range_metrics[slice_idx * combo_count + combo_idx];
            let slice_score = effective_score(slice_metrics, objective, score_mode);
            worst_slice_score = worst_slice_score.min(slice_score);
            if slice_score >= 0.0 {
                positive_slice_count += 1;
            }
            mean_slice_score += slice_score;
            slice_scores.push(slice_score);
            let (start_bar, end_bar) = ranges[slice_idx];
            if let Some(benchmark_slice_returns) = benchmark_slice_returns.as_ref() {
                let obs = end_bar.saturating_sub(start_bar);
                let candidate_sum = shortlist_slice_candidate_return_sum(slice_metrics, obs);
                slice_excess_returns.push(candidate_sum - benchmark_slice_returns[slice_idx]);
            }
            slices.push(ShortlistSliceSnapshot {
                label: slice_labels[slice_idx].clone(),
                start_bar,
                end_bar,
                score: slice_score,
                metrics: slice_metrics.clone(),
            });
        }
        mean_slice_score /= slice_count as f64;
        let omission = compute_shortlist_slice_omission_summary(&slice_scores, &slice_labels)?;
        let benchmark_relative = benchmark
            .map(|benchmark| {
                compute_shortlist_slice_benchmark_relative_summary(
                    &slice_excess_returns,
                    &slice_labels,
                    benchmark,
                )
            })
            .transpose()?;
        records.push(ShortlistSliceModeCandidateRecord {
            params,
            full_metrics: metrics,
            diagnostics: ShortlistSliceModeDiagnostics {
                worst_slice_score,
                positive_slice_frac: positive_slice_count as f64 / slice_count as f64,
                mean_slice_score,
                best_omission_label: omission.best_label,
                best_omission_score: omission.best_score,
                worst_omission_label: omission.worst_label,
                worst_omission_score: omission.worst_score,
                omission_score_spread: omission.spread,
                benchmark_relative,
                slices,
            },
        });
    }

    let compute_elapsed = compute_t0.elapsed();
    let record_assembly_elapsed = compute_elapsed
        .saturating_sub(replay_elapsed)
        .saturating_sub(replay_descriptor_prep_elapsed);
    if let Some(timing) = timing.as_deref_mut() {
        timing.replay = timing.replay.saturating_add(replay_elapsed);
        timing.replay_descriptor_prep = timing
            .replay_descriptor_prep
            .saturating_add(replay_descriptor_prep_elapsed);
        timing.record_assembly = timing
            .record_assembly
            .saturating_add(record_assembly_elapsed);
    }

    Ok(records)
}

#[inline]
fn kernel_finalist_recompute_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_FINALIST_RECOMPUTE", true)
}

#[inline]
fn kernel_finalist_profile_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_FINALIST_PROFILE", false)
}

#[inline]
fn kernel_product_reduced_rows_enabled_for_objective(objective: ObjectiveKind) -> bool {
    match env_flag_override("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS") {
        Some(v) => v,
        None => matches!(objective, ObjectiveKind::Sharpe | ObjectiveKind::Volatility),
    }
}

#[inline]
fn kernel_log_dispatch_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_LOG_DISPATCH", false)
}

#[inline]
fn kernel_valid_pair_map_enabled() -> bool {
    env_flag_enabled("VECTORBT_KERNEL_VALID_PAIR_MAP", true)
}

#[inline]
fn kernel_valid_pair_map_max_pairs() -> usize {
    std::env::var("VECTORBT_KERNEL_VALID_PAIR_MAP_MAX_PAIRS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(1_000_000)
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
    if let Some(v) = env_flag_override("VECTORBT_KERNEL_METRICS_PAD8") {
        return v;
    }
    fast_ma == "alma" && slow_ma == "alma"
}

#[inline]
fn kernel_metrics_fixed_stride_override(metrics_count: usize) -> Option<usize> {
    let parse = |name: &str| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|&v| v >= metrics_count)
    };
    match metrics_count {
        METRICS_COUNT => parse("VECTORBT_KERNEL_METRICS_FULL17_STRIDE")
            .or_else(|| parse("VECTORBT_KERNEL_METRICS_FULL_STRIDE")),
        TRADE_METRICS_COUNT => parse("VECTORBT_KERNEL_METRICS_TRADE22_STRIDE"),
        _ => None,
    }
}

#[inline]
fn effective_metrics_stride(
    metrics_count: usize,
    pad8_enabled: bool,
    fixed_stride_override: Option<usize>,
) -> usize {
    if matches!(metrics_count, METRICS_COUNT | TRADE_METRICS_COUNT) {
        if let Some(v) = kernel_metrics_fixed_stride_override(metrics_count) {
            return v;
        }
        if let Some(v) = fixed_stride_override.filter(|&v| v >= metrics_count) {
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
pub(crate) fn search_metrics_count_for_optimization(
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    constraints: Option<&DoubleMaConstraints>,
    include_all: bool,
    export_enabled: bool,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    include_trade_metrics: bool,
) -> usize {
    search_metrics_count_for_optimization_with_contract(
        objective,
        score_mode,
        constraints,
        include_all,
        export_enabled,
        fast_precomputed_enabled,
        slow_precomputed_enabled,
        include_trade_metrics,
        None,
    )
}

#[inline]
pub(crate) fn search_metrics_count_for_optimization_with_contract(
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    constraints: Option<&DoubleMaConstraints>,
    include_all: bool,
    export_enabled: bool,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    include_trade_metrics: bool,
    metric_contract: Option<DoubleMaMetricContract>,
) -> usize {
    if matches!(metric_contract, Some(DoubleMaMetricContract::Trade22)) {
        return TRADE_METRICS_COUNT;
    }
    if matches!(metric_contract, Some(DoubleMaMetricContract::Full17)) {
        return METRICS_COUNT;
    }
    if include_trade_metrics {
        return TRADE_METRICS_COUNT;
    }
    if include_all || export_enabled || fast_precomputed_enabled || slow_precomputed_enabled {
        return METRICS_COUNT;
    }

    let mut metrics_count = match objective {
        ObjectiveKind::Pnl => 1,

        ObjectiveKind::Sharpe | ObjectiveKind::Volatility => 7,
        ObjectiveKind::MaxDrawdown | ObjectiveKind::Calmar => 3,
        ObjectiveKind::Sortino | ObjectiveKind::Omega => METRICS_COUNT,
    };

    metrics_count = metrics_count.max(min_metrics_count_for_constraints(constraints));
    metrics_count = metrics_count.max(score_mode_metrics_count_floor(score_mode));
    if kernel_product_reduced_rows_enabled_for_objective(objective) {
        metrics_count = metrics_count.max(PRODUCT_REDUCED_METRICS_COUNT);
    }

    metrics_count.min(METRICS_COUNT)
}

#[inline]
fn exact_eval_metrics_count(cfg: KernelConfig) -> usize {
    exact_eval_metrics_count_with_contract(cfg.include_trade_metrics, cfg.metric_contract)
}

#[inline]
fn score_first_search_metrics_count_with_exact_contract(
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    constraints: Option<&DoubleMaConstraints>,
    include_all: bool,
    export_enabled: bool,
    fast_precomputed_enabled: bool,
    slow_precomputed_enabled: bool,
    exact_metrics_count: usize,
) -> usize {
    if exact_metrics_count != METRICS_COUNT && exact_metrics_count != TRADE_METRICS_COUNT {
        return exact_metrics_count;
    }
    search_metrics_count_for_optimization_with_contract(
        objective,
        score_mode,
        constraints,
        include_all,
        export_enabled,
        fast_precomputed_enabled,
        slow_precomputed_enabled,
        false,
        None,
    )
}

#[inline]
fn can_decode_current_gpu_product_metrics_without_recompute(metrics_count: usize) -> bool {
    metrics_count >= PRODUCT_REDUCED_METRICS_COUNT
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

fn normalize_topk_select_for_device(
    top_k: usize,
    device_include_all: bool,
) -> Result<usize, String> {
    let topk_select = top_k.max(1);
    if !device_include_all && topk_select > TOPK_BLOCK_THREADS as usize {
        return Err(
            "GPU kernel backend top_k too large (max 256 when include_all=false)".to_string(),
        );
    }
    Ok(topk_select)
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
fn copy_device_prefix_to_host_i32(
    src: &DeviceBuffer<i32>,
    dst: &mut Vec<i32>,
    elems: usize,
    stream: &Stream,
) -> Result<(), String> {
    resize_uninit_i32(dst, elems);
    if elems == 0 {
        return Ok(());
    }
    let bytes = elems
        .checked_mul(std::mem::size_of::<i32>())
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
fn pack_metric_rows_for_host_copy<'a>(
    stream: &Stream,
    scratch: &mut KernelScratch,
    pack_metric_rows: &Function,
    fast_rows_ptr: cuda_sys::CUdeviceptr,
    slow_rows_ptr: cuda_sys::CUdeviceptr,
    metrics_rows_ptr: cuda_sys::CUdeviceptr,
    rows: usize,
    metrics_count: usize,
    metrics_stride: usize,
    host_out: &'a mut HostTransferScratchF32,
    launch_submit_elapsed: &mut Duration,
    d2h_copy_elapsed: &mut Duration,
    sync_elapsed: &mut Duration,
) -> Result<&'a [f32], String> {
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
            let mut metrics_stride_i = metrics_stride as i32;
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
                &mut metrics_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                stream,
                pack_metric_rows,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
        }
    }
    host_out.copy_device_prefix_to_host_timed(
        scratch
            .d_top_packed
            .as_ref()
            .ok_or_else(|| "missing packed top-row buffer".to_string())?,
        packed_len,
        stream,
        d2h_copy_elapsed,
        sync_elapsed,
    )
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

    bytes = bytes.saturating_add(bytes_f32(series_len));
    if !ma_is_close {
        bytes = bytes.saturating_add(bytes_f32(series_len));
    }
    bytes = bytes.saturating_add(bytes_f32(series_len));
    bytes = bytes.saturating_add(bytes_f32(series_len));

    if fast_ma == "vwma" || slow_ma == "vwma" {
        bytes = bytes.saturating_add(bytes_f32(series_len));
        bytes = bytes.saturating_add(bytes_f64(series_len).saturating_mul(2));
    }
    if fast_ma == "frama" || slow_ma == "frama" {
        bytes = bytes.saturating_add(bytes_f32(series_len).saturating_mul(2));
    }

    if fast_ma == "sma" || slow_ma == "sma" {
        bytes = bytes.saturating_add(bytes_f64(series_len.saturating_add(1)));
    }

    if fast_ma == "nma" || slow_ma == "nma" {
        bytes = bytes.saturating_add(bytes_f32(series_len));
    }
    if fast_ma == "edcf" || slow_ma == "edcf" {
        bytes = bytes.saturating_add(bytes_f32(series_len));
    }
    if fast_ma == "cora_wave" || slow_ma == "cora_wave" {
        let p_max = pf.max(ps).max(1);
        bytes = bytes.saturating_add(bytes_f32(p_max.saturating_mul(series_len)));
    }

    let ma_matrix_mult = if kernel_fast_path_enabled(fast_ma, slow_ma) {
        1
    } else {
        2
    };

    bytes = bytes.saturating_add(bytes_i32(pf));
    bytes = bytes.saturating_add(bytes_f32(
        pf.saturating_mul(series_len).saturating_mul(ma_matrix_mult),
    ));
    if fast_ma == "alma" || fast_ma == "cwma" || fast_ma == "cora_wave" {
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(max_fast_period)));
        bytes = bytes.saturating_add(bytes_f32(pf));
    } else if fast_ma == "fwma" || fast_ma == "pwma" {
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(max_fast_period)));
    } else if fast_ma == "srwma" {
        bytes = bytes.saturating_add(bytes_f32(
            pf.saturating_mul(max_fast_period.saturating_sub(1)),
        ));
        bytes = bytes.saturating_add(bytes_f32(pf));
    } else if fast_ma == "ema" || fast_ma == "mwdx" {
        bytes = bytes.saturating_add(bytes_f32(pf));
    } else if fast_ma == "zlema" {
        bytes = bytes.saturating_add(bytes_i32(pf));
        bytes = bytes.saturating_add(bytes_f32(pf));
    } else if fast_ma == "maaq" {
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(2)));
    } else if fast_ma == "jma" {
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(3)));
    } else if fast_ma == "hma" {
        let max_sqrt = ((max_fast_period as f64).sqrt().floor() as usize).max(1);
        bytes = bytes.saturating_add(bytes_f32(pf.saturating_mul(max_sqrt)));
    } else if fast_ma == "vpwma" {
        bytes = bytes.saturating_add(bytes_i32(pf));
        bytes = bytes.saturating_add(bytes_f32(
            pf.saturating_mul(max_fast_period.saturating_sub(1)),
        ));
        bytes = bytes.saturating_add(bytes_f32(pf));
    }
    if matches!(
        fast_ma,
        "jsa" | "smma" | "swma" | "trima" | "fwma" | "pwma" | "srwma"
    ) {
        bytes = bytes.saturating_add(bytes_i32(pf));
    } else if fast_ma == "wilders" {
        bytes = bytes.saturating_add(bytes_i32(pf));
        bytes = bytes.saturating_add(bytes_f32(pf));
    } else if fast_ma == "epma" {
        bytes = bytes.saturating_add(bytes_i32(pf));
    } else if fast_ma == "cora_wave" {
        bytes = bytes.saturating_add(bytes_i32(pf.saturating_mul(2)));
    } else if fast_ma == "ehlers_itrend" {
        bytes = bytes.saturating_add(bytes_i32(pf));
    }
    if fast_ma == "frama" {
        bytes = bytes.saturating_add(bytes_i32(pf).saturating_mul(2));
    }

    bytes = bytes.saturating_add(bytes_i32(ps));
    bytes = bytes.saturating_add(bytes_f32(
        ps.saturating_mul(series_len).saturating_mul(ma_matrix_mult),
    ));
    if slow_ma == "alma" || slow_ma == "cwma" || slow_ma == "cora_wave" {
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(max_slow_period)));
        bytes = bytes.saturating_add(bytes_f32(ps));
    } else if slow_ma == "fwma" || slow_ma == "pwma" {
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(max_slow_period)));
    } else if slow_ma == "srwma" {
        bytes = bytes.saturating_add(bytes_f32(
            ps.saturating_mul(max_slow_period.saturating_sub(1)),
        ));
        bytes = bytes.saturating_add(bytes_f32(ps));
    } else if slow_ma == "ema" || slow_ma == "mwdx" {
        bytes = bytes.saturating_add(bytes_f32(ps));
    } else if slow_ma == "zlema" {
        bytes = bytes.saturating_add(bytes_i32(ps));
        bytes = bytes.saturating_add(bytes_f32(ps));
    } else if slow_ma == "maaq" {
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(2)));
    } else if slow_ma == "jma" {
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(3)));
    } else if slow_ma == "hma" {
        let max_sqrt = ((max_slow_period as f64).sqrt().floor() as usize).max(1);
        bytes = bytes.saturating_add(bytes_f32(ps.saturating_mul(max_sqrt)));
    } else if slow_ma == "vpwma" {
        bytes = bytes.saturating_add(bytes_i32(ps));
        bytes = bytes.saturating_add(bytes_f32(
            ps.saturating_mul(max_slow_period.saturating_sub(1)),
        ));
        bytes = bytes.saturating_add(bytes_f32(ps));
    }
    if matches!(
        slow_ma,
        "jsa" | "smma" | "swma" | "trima" | "fwma" | "pwma" | "srwma"
    ) {
        bytes = bytes.saturating_add(bytes_i32(ps));
    } else if slow_ma == "wilders" {
        bytes = bytes.saturating_add(bytes_i32(ps));
        bytes = bytes.saturating_add(bytes_f32(ps));
    } else if slow_ma == "epma" {
        bytes = bytes.saturating_add(bytes_i32(ps));
    } else if slow_ma == "cora_wave" {
        bytes = bytes.saturating_add(bytes_i32(ps.saturating_mul(2)));
    } else if slow_ma == "ehlers_itrend" {
        bytes = bytes.saturating_add(bytes_i32(ps));
    }
    if slow_ma == "frama" {
        bytes = bytes.saturating_add(bytes_i32(ps).saturating_mul(2));
    }

    let pairs = pf.saturating_mul(ps);
    let tile_metrics_stride = effective_metrics_stride(
        metrics_count,
        kernel_metrics_pad8_enabled_for_pair(fast_ma, slow_ma),
        None,
    );
    bytes = bytes.saturating_add(bytes_f32(pairs.saturating_mul(tile_metrics_stride)));

    if !include_all {
        let topk = topk_select.max(1);
        let blocks = (pairs + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let cand_len = blocks.saturating_mul(topk).max(topk);

        bytes = bytes.saturating_add(bytes_i32(cand_len).saturating_mul(2));
        bytes = bytes.saturating_add(bytes_i32(topk).saturating_mul(2));
        bytes = bytes.saturating_add(bytes_f32(topk.saturating_mul(metrics_count)));
        bytes = bytes.saturating_add(bytes_f32(
            topk.saturating_mul(metrics_count.saturating_add(2)),
        ));

        if heatmap_bins > 0 {
            let heat_len = heatmap_bins.saturating_mul(heatmap_bins);
            bytes = bytes.saturating_add(bytes_f32(heat_len));
        }
    }

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

fn reduce_scored_rows_topk_on_device(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    rows_fast: &DeviceBuffer<i32>,
    rows_slow: &DeviceBuffer<i32>,
    rows_scores: &DeviceBuffer<f32>,
    rows_len: usize,
    topk_select: usize,
    select_rows: &cust::function::Function<'_>,
    select_rows_candidates: &cust::function::Function<'_>,
    gather_rows: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
    if rows_len == 0 || topk_select == 0 {
        return Ok(());
    }

    let blocks1 = (rows_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
    let cand_len1 = blocks1.saturating_mul(topk_select).max(topk_select);
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_a, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_b, cand_len1, bt_stream)?;

    unsafe {
        let mut scores_ptr = rows_scores.as_device_ptr().as_raw();
        let mut rows_i = rows_len as i32;
        let mut topk_i = topk_select as i32;
        let mut out_ptr = scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = blocks1 as u32;
        let args: &mut [*mut c_void] = &mut [
            &mut scores_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut out_ptr as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            select_rows,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    let mut blocks_prev = blocks1;
    let mut in_len = cand_len1;
    let mut in_is_a = true;
    while blocks_prev > 1 {
        let blocks_next = (in_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let out_len = blocks_next.saturating_mul(topk_select).max(topk_select);
        unsafe {
            let mut scores_ptr = rows_scores.as_device_ptr().as_raw();
            let mut rows_i = rows_len as i32;
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
            let args: &mut [*mut c_void] = &mut [
                &mut scores_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut in_ptr as *mut _ as *mut c_void,
                &mut in_len_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_rows_candidates,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
        }
        in_is_a = !in_is_a;
        blocks_prev = blocks_next;
        in_len = out_len;
    }

    unsafe {
        let mut scores_ptr = rows_scores.as_device_ptr().as_raw();
        let mut rows_i = rows_len as i32;
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
        let mut out_scores = scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = ((topk_select as u32) + block_x - 1) / block_x;
        let args: &mut [*mut c_void] = &mut [
            &mut scores_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut fast_ptr as *mut _ as *mut c_void,
            &mut slow_ptr as *mut _ as *mut c_void,
            &mut top_ptr as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut out_f as *mut _ as *mut c_void,
            &mut out_s as *mut _ as *mut c_void,
            &mut out_scores as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            gather_rows,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn merge_current_topk_scores_into_global_head(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    global_fast: &DeviceBuffer<i32>,
    global_slow: &DeviceBuffer<i32>,
    global_scores: &DeviceBuffer<f32>,
    global_rows_len: &mut usize,
    global_rows_cap: usize,
    topk_select: usize,
    select_rows: &cust::function::Function<'_>,
    select_rows_candidates: &cust::function::Function<'_>,
    gather_rows: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
    let row_i32_bytes = topk_select
        .checked_mul(std::mem::size_of::<i32>())
        .ok_or_else(|| "topk row bytes overflow (i32)".to_string())?;
    let row_score_bytes = topk_select
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or_else(|| "topk row bytes overflow (scores)".to_string())?;

    if *global_rows_len == 0 {
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
            global_scores.as_device_ptr().as_raw(),
            scratch
                .d_top_scores
                .as_ref()
                .unwrap()
                .as_device_ptr()
                .as_raw(),
            row_score_bytes,
            bt_stream,
        )?;
        *global_rows_len = topk_select;
        return Ok(());
    }

    if global_rows_len.saturating_add(topk_select) > global_rows_cap {
        return Err(
            "global score-top-k merge buffer overflow; expected capacity for 2 * top_k rows"
                .to_string(),
        );
    }
    let dst_fast = global_fast
        .as_device_ptr()
        .as_raw()
        .saturating_add((*global_rows_len * std::mem::size_of::<i32>()) as u64);
    let dst_slow = global_slow
        .as_device_ptr()
        .as_raw()
        .saturating_add((*global_rows_len * std::mem::size_of::<i32>()) as u64);
    let dst_scores = global_scores
        .as_device_ptr()
        .as_raw()
        .saturating_add((*global_rows_len * std::mem::size_of::<f32>()) as u64);
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
        dst_scores,
        scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw(),
        row_score_bytes,
        bt_stream,
    )?;
    *global_rows_len = global_rows_len.saturating_add(topk_select);

    reduce_scored_rows_topk_on_device(
        bt_stream,
        scratch,
        global_fast,
        global_slow,
        global_scores,
        *global_rows_len,
        topk_select,
        select_rows,
        select_rows_candidates,
        gather_rows,
        launch_submit_elapsed,
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
        global_scores.as_device_ptr().as_raw(),
        scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw(),
        row_score_bytes,
        bt_stream,
    )?;
    *global_rows_len = topk_select;

    Ok(())
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
    metrics_stride: usize,
    objective_i: i32,
    hurdle_tau: f32,
    topk_select: usize,
    constraints_on_device: bool,
    constraint_args: DeviceConstraintArgs,
    select_rows_legacy: &cust::function::Function<'_>,
    select_rows_candidates_legacy: &cust::function::Function<'_>,
    select_rows_v2: Option<&cust::function::Function<'_>>,
    select_rows_candidates_v2: Option<&cust::function::Function<'_>>,
    gather_rows: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
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
        let mut metrics_stride_i = metrics_stride as i32;
        let mut obj_i = objective_i;
        let mut hurdle_tau_i = hurdle_tau;
        let mut topk_i = topk_select as i32;
        let mut out_ptr = scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = blocks1 as u32;
        if constraints_on_device {
            let mut use_min_trades = constraint_args.use_min_trades;
            let mut min_trades = constraint_args.min_trades;
            let mut use_max_dd = constraint_args.use_max_dd;
            let mut max_dd = constraint_args.max_dd;
            let mut use_min_sharpe = constraint_args.use_min_sharpe;
            let mut min_sharpe = constraint_args.min_sharpe;
            let mut use_min_sortino = constraint_args.use_min_sortino;
            let mut min_sortino = constraint_args.min_sortino;
            let mut use_min_calmar = constraint_args.use_min_calmar;
            let mut min_calmar = constraint_args.min_calmar;
            let mut use_max_volatility = constraint_args.use_max_volatility;
            let mut max_volatility = constraint_args.max_volatility;
            let mut use_min_exposure = constraint_args.use_min_exposure;
            let mut min_exposure = constraint_args.min_exposure;
            let mut use_max_exposure = constraint_args.use_max_exposure;
            let mut max_exposure = constraint_args.max_exposure;
            let mut use_max_abs_net_exposure = constraint_args.use_max_abs_net_exposure;
            let mut max_abs_net_exposure = constraint_args.max_abs_net_exposure;
            let mut use_min_omega = constraint_args.use_min_omega;
            let mut min_omega = constraint_args.min_omega;
            let mut use_max_ulcer = constraint_args.use_max_ulcer;
            let mut max_ulcer = constraint_args.max_ulcer;
            let mut use_max_downside_deviation = constraint_args.use_max_downside_deviation;
            let mut max_downside_deviation = constraint_args.max_downside_deviation;
            let args: &mut [*mut c_void] = &mut [
                &mut metrics_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut metrics_stride_i as *mut _ as *mut c_void,
                &mut obj_i as *mut _ as *mut c_void,
                &mut hurdle_tau_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut use_min_trades as *mut _ as *mut c_void,
                &mut min_trades as *mut _ as *mut c_void,
                &mut use_max_dd as *mut _ as *mut c_void,
                &mut max_dd as *mut _ as *mut c_void,
                &mut use_min_sharpe as *mut _ as *mut c_void,
                &mut min_sharpe as *mut _ as *mut c_void,
                &mut use_min_sortino as *mut _ as *mut c_void,
                &mut min_sortino as *mut _ as *mut c_void,
                &mut use_min_calmar as *mut _ as *mut c_void,
                &mut min_calmar as *mut _ as *mut c_void,
                &mut use_max_volatility as *mut _ as *mut c_void,
                &mut max_volatility as *mut _ as *mut c_void,
                &mut use_min_exposure as *mut _ as *mut c_void,
                &mut min_exposure as *mut _ as *mut c_void,
                &mut use_max_exposure as *mut _ as *mut c_void,
                &mut max_exposure as *mut _ as *mut c_void,
                &mut use_max_abs_net_exposure as *mut _ as *mut c_void,
                &mut max_abs_net_exposure as *mut _ as *mut c_void,
                &mut use_min_omega as *mut _ as *mut c_void,
                &mut min_omega as *mut _ as *mut c_void,
                &mut use_max_ulcer as *mut _ as *mut c_void,
                &mut max_ulcer as *mut _ as *mut c_void,
                &mut use_max_downside_deviation as *mut _ as *mut c_void,
                &mut max_downside_deviation as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_rows_v2
                    .ok_or_else(|| "missing select_topk_metric_rows_f32_v2".to_string())?,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
        } else {
            let args: &mut [*mut c_void] = &mut [
                &mut metrics_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut metrics_stride_i as *mut _ as *mut c_void,
                &mut obj_i as *mut _ as *mut c_void,
                &mut hurdle_tau_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_rows_legacy,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
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
            let mut metrics_stride_i = metrics_stride as i32;
            let mut obj_i = objective_i;
            let mut hurdle_tau_i = hurdle_tau;
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
                let mut use_min_trades = constraint_args.use_min_trades;
                let mut min_trades = constraint_args.min_trades;
                let mut use_max_dd = constraint_args.use_max_dd;
                let mut max_dd = constraint_args.max_dd;
                let mut use_min_sharpe = constraint_args.use_min_sharpe;
                let mut min_sharpe = constraint_args.min_sharpe;
                let mut use_min_sortino = constraint_args.use_min_sortino;
                let mut min_sortino = constraint_args.min_sortino;
                let mut use_min_calmar = constraint_args.use_min_calmar;
                let mut min_calmar = constraint_args.min_calmar;
                let mut use_max_volatility = constraint_args.use_max_volatility;
                let mut max_volatility = constraint_args.max_volatility;
                let mut use_min_exposure = constraint_args.use_min_exposure;
                let mut min_exposure = constraint_args.min_exposure;
                let mut use_max_exposure = constraint_args.use_max_exposure;
                let mut max_exposure = constraint_args.max_exposure;
                let mut use_max_abs_net_exposure = constraint_args.use_max_abs_net_exposure;
                let mut max_abs_net_exposure = constraint_args.max_abs_net_exposure;
                let mut use_min_omega = constraint_args.use_min_omega;
                let mut min_omega = constraint_args.min_omega;
                let mut use_max_ulcer = constraint_args.use_max_ulcer;
                let mut max_ulcer = constraint_args.max_ulcer;
                let mut use_max_downside_deviation = constraint_args.use_max_downside_deviation;
                let mut max_downside_deviation = constraint_args.max_downside_deviation;
                let args: &mut [*mut c_void] = &mut [
                    &mut metrics_ptr as *mut _ as *mut c_void,
                    &mut rows_i as *mut _ as *mut c_void,
                    &mut m_i as *mut _ as *mut c_void,
                    &mut metrics_stride_i as *mut _ as *mut c_void,
                    &mut obj_i as *mut _ as *mut c_void,
                    &mut hurdle_tau_i as *mut _ as *mut c_void,
                    &mut topk_i as *mut _ as *mut c_void,
                    &mut in_ptr as *mut _ as *mut c_void,
                    &mut in_len_i as *mut _ as *mut c_void,
                    &mut use_min_trades as *mut _ as *mut c_void,
                    &mut min_trades as *mut _ as *mut c_void,
                    &mut use_max_dd as *mut _ as *mut c_void,
                    &mut max_dd as *mut _ as *mut c_void,
                    &mut use_min_sharpe as *mut _ as *mut c_void,
                    &mut min_sharpe as *mut _ as *mut c_void,
                    &mut use_min_sortino as *mut _ as *mut c_void,
                    &mut min_sortino as *mut _ as *mut c_void,
                    &mut use_min_calmar as *mut _ as *mut c_void,
                    &mut min_calmar as *mut _ as *mut c_void,
                    &mut use_max_volatility as *mut _ as *mut c_void,
                    &mut max_volatility as *mut _ as *mut c_void,
                    &mut use_min_exposure as *mut _ as *mut c_void,
                    &mut min_exposure as *mut _ as *mut c_void,
                    &mut use_max_exposure as *mut _ as *mut c_void,
                    &mut max_exposure as *mut _ as *mut c_void,
                    &mut use_max_abs_net_exposure as *mut _ as *mut c_void,
                    &mut max_abs_net_exposure as *mut _ as *mut c_void,
                    &mut use_min_omega as *mut _ as *mut c_void,
                    &mut min_omega as *mut _ as *mut c_void,
                    &mut use_max_ulcer as *mut _ as *mut c_void,
                    &mut max_ulcer as *mut _ as *mut c_void,
                    &mut use_max_downside_deviation as *mut _ as *mut c_void,
                    &mut max_downside_deviation as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                launch_kernel_timed(
                    bt_stream,
                    select_rows_candidates_v2.ok_or_else(|| {
                        "missing select_topk_metric_rows_from_candidates_f32_v2".to_string()
                    })?,
                    (grid_x, 1, 1),
                    (block_x, 1, 1),
                    0,
                    args,
                    launch_submit_elapsed,
                )?;
            } else {
                let args: &mut [*mut c_void] = &mut [
                    &mut metrics_ptr as *mut _ as *mut c_void,
                    &mut rows_i as *mut _ as *mut c_void,
                    &mut m_i as *mut _ as *mut c_void,
                    &mut metrics_stride_i as *mut _ as *mut c_void,
                    &mut obj_i as *mut _ as *mut c_void,
                    &mut hurdle_tau_i as *mut _ as *mut c_void,
                    &mut topk_i as *mut _ as *mut c_void,
                    &mut in_ptr as *mut _ as *mut c_void,
                    &mut in_len_i as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                launch_kernel_timed(
                    bt_stream,
                    select_rows_candidates_legacy,
                    (grid_x, 1, 1),
                    (block_x, 1, 1),
                    0,
                    args,
                    launch_submit_elapsed,
                )?;
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
        let mut metrics_stride_i = metrics_stride as i32;
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
            &mut metrics_stride_i as *mut _ as *mut c_void,
            &mut fast_ptr as *mut _ as *mut c_void,
            &mut slow_ptr as *mut _ as *mut c_void,
            &mut top_ptr as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut out_f as *mut _ as *mut c_void,
            &mut out_s as *mut _ as *mut c_void,
            &mut out_m as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            gather_rows,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    Ok(())
}

#[allow(dead_code)]
fn score_metric_rows_for_objective_on_device(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    rows: usize,
    metrics_count: usize,
    metrics_stride: usize,
    objective_i: i32,
    hurdle_tau: f32,
    score_metric_rows: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
    if rows == 0 {
        return Ok(());
    }

    KernelScratch::ensure_len_f32(&mut scratch.d_top_scores, rows, bt_stream)?;

    unsafe {
        let mut metrics_ptr = scratch
            .d_top_metrics
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();
        let mut rows_i = rows as i32;
        let mut m_i = metrics_count as i32;
        let mut metrics_stride_i = metrics_stride as i32;
        let mut obj_i = objective_i;
        let mut hurdle_tau_i = hurdle_tau;
        let mut out_ptr = scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();
        let block_x: u32 = 256;
        let grid_x: u32 = ((rows as u32) + block_x - 1) / block_x;
        let args: &mut [*mut c_void] = &mut [
            &mut metrics_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut m_i as *mut _ as *mut c_void,
            &mut metrics_stride_i as *mut _ as *mut c_void,
            &mut obj_i as *mut _ as *mut c_void,
            &mut hurdle_tau_i as *mut _ as *mut c_void,
            &mut out_ptr as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            score_metric_rows,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    Ok(())
}

#[allow(dead_code)]
fn reduce_scored_split_pairs_topk_on_device(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    rows_fast: &DeviceBuffer<i32>,
    rows_slow: &DeviceBuffer<i32>,
    rows_scores: &DeviceBuffer<f32>,
    rows_len: usize,
    topk_select: usize,
    select_scores_legacy: &cust::function::Function<'_>,
    select_scores_candidates_legacy: &cust::function::Function<'_>,
    gather_scored_pairs: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
    if rows_len == 0 || topk_select == 0 {
        return Ok(());
    }

    let blocks1 = (rows_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
    let cand_len1 = blocks1.saturating_mul(topk_select).max(topk_select);
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_a, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_b, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_top_fast, topk_select, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_top_slow, topk_select, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_top_scores, topk_select, bt_stream)?;

    unsafe {
        let mut scores_ptr = rows_scores.as_device_ptr().as_raw();
        let mut rows_i = rows_len as i32;
        let mut topk_i = topk_select as i32;
        let mut out_ptr = scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = blocks1 as u32;
        let args: &mut [*mut c_void] = &mut [
            &mut scores_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut out_ptr as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            select_scores_legacy,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    let mut blocks_prev = blocks1;
    let mut in_len = cand_len1;
    let mut in_is_a = true;
    while blocks_prev > 1 {
        let blocks_next = (in_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let out_len = blocks_next.saturating_mul(topk_select).max(topk_select);
        unsafe {
            let mut scores_ptr = rows_scores.as_device_ptr().as_raw();
            let mut rows_i = rows_len as i32;
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
            let args: &mut [*mut c_void] = &mut [
                &mut scores_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut in_ptr as *mut _ as *mut c_void,
                &mut in_len_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_scores_candidates_legacy,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
        }
        in_is_a = !in_is_a;
        blocks_prev = blocks_next;
        in_len = out_len;
    }

    unsafe {
        let mut fast_ptr = rows_fast.as_device_ptr().as_raw();
        let mut slow_ptr = rows_slow.as_device_ptr().as_raw();
        let mut scores_ptr = rows_scores.as_device_ptr().as_raw();
        let mut rows_i = rows_len as i32;
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
        let mut out_scores = scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = ((topk_select as u32) + block_x - 1) / block_x;
        let args: &mut [*mut c_void] = &mut [
            &mut fast_ptr as *mut _ as *mut c_void,
            &mut slow_ptr as *mut _ as *mut c_void,
            &mut scores_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut top_ptr as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut out_f as *mut _ as *mut c_void,
            &mut out_s as *mut _ as *mut c_void,
            &mut out_scores as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            gather_scored_pairs,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    Ok(())
}

#[allow(dead_code)]
fn merge_current_topk_scored_split_pairs_into_global_head(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    global_fast: &DeviceBuffer<i32>,
    global_slow: &DeviceBuffer<i32>,
    global_scores: &DeviceBuffer<f32>,
    global_rows_len: &mut usize,
    global_rows_cap: usize,
    topk_select: usize,
    select_scores_legacy: &cust::function::Function<'_>,
    select_scores_candidates_legacy: &cust::function::Function<'_>,
    gather_scored_pairs: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
    let row_i32_bytes = topk_select
        .checked_mul(std::mem::size_of::<i32>())
        .ok_or_else(|| "topk scored row bytes overflow (i32)".to_string())?;
    let row_score_bytes = topk_select
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or_else(|| "topk scored row bytes overflow (score)".to_string())?;

    if *global_rows_len == 0 {
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
            global_scores.as_device_ptr().as_raw(),
            scratch
                .d_top_scores
                .as_ref()
                .unwrap()
                .as_device_ptr()
                .as_raw(),
            row_score_bytes,
            bt_stream,
        )?;
        *global_rows_len = topk_select;
        return Ok(());
    }

    if global_rows_len.saturating_add(topk_select) > global_rows_cap {
        return Err(
            "global scored top-k merge buffer overflow; expected capacity for 2 * top_k rows"
                .to_string(),
        );
    }

    let dst_fast = global_fast
        .as_device_ptr()
        .as_raw()
        .saturating_add((*global_rows_len * std::mem::size_of::<i32>()) as u64);
    let dst_slow = global_slow
        .as_device_ptr()
        .as_raw()
        .saturating_add((*global_rows_len * std::mem::size_of::<i32>()) as u64);
    let dst_scores = global_scores
        .as_device_ptr()
        .as_raw()
        .saturating_add((*global_rows_len * std::mem::size_of::<f32>()) as u64);

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
        dst_scores,
        scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw(),
        row_score_bytes,
        bt_stream,
    )?;
    *global_rows_len = global_rows_len.saturating_add(topk_select);

    reduce_scored_split_pairs_topk_on_device(
        bt_stream,
        scratch,
        global_fast,
        global_slow,
        global_scores,
        *global_rows_len,
        topk_select,
        select_scores_legacy,
        select_scores_candidates_legacy,
        gather_scored_pairs,
        launch_submit_elapsed,
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
        global_scores.as_device_ptr().as_raw(),
        scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw(),
        row_score_bytes,
        bt_stream,
    )?;
    *global_rows_len = topk_select;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn merge_current_topk_rows_into_global_head(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    global_fast: &DeviceBuffer<i32>,
    global_slow: &DeviceBuffer<i32>,
    global_metrics: &DeviceBuffer<f32>,
    global_rows_len: &mut usize,
    global_rows_cap: usize,
    metrics_count: usize,
    metrics_stride: usize,
    objective_i: i32,
    hurdle_tau: f32,
    topk_select: usize,
    global_constraints_on_device: bool,
    constraint_args: DeviceConstraintArgs,
    select_topk_rows_legacy: &cust::function::Function<'_>,
    select_topk_rows_candidates_legacy: &cust::function::Function<'_>,
    select_topk_rows_v2: Option<&cust::function::Function<'_>>,
    select_topk_rows_candidates_v2: Option<&cust::function::Function<'_>>,
    gather_topk_rows: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
    let row_i32_bytes = topk_select
        .checked_mul(std::mem::size_of::<i32>())
        .ok_or_else(|| "topk row bytes overflow (i32)".to_string())?;
    let row_metrics_bytes = topk_select
        .checked_mul(metrics_count)
        .and_then(|v| v.checked_mul(std::mem::size_of::<f32>()))
        .ok_or_else(|| "topk row bytes overflow (metrics)".to_string())?;

    if *global_rows_len == 0 {
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
        *global_rows_len = topk_select;
        return Ok(());
    }

    if global_rows_len.saturating_add(topk_select) > global_rows_cap {
        return Err(
            "global top-k merge buffer overflow; expected capacity for 2 * top_k rows".to_string(),
        );
    }
    let dst_fast = global_fast
        .as_device_ptr()
        .as_raw()
        .saturating_add((*global_rows_len * std::mem::size_of::<i32>()) as u64);
    let dst_slow = global_slow
        .as_device_ptr()
        .as_raw()
        .saturating_add((*global_rows_len * std::mem::size_of::<i32>()) as u64);
    let dst_metrics = global_metrics.as_device_ptr().as_raw().saturating_add(
        ((*global_rows_len)
            .saturating_mul(metrics_count)
            .saturating_mul(std::mem::size_of::<f32>())) as u64,
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
    *global_rows_len = global_rows_len.saturating_add(topk_select);

    reduce_metric_rows_topk_on_device(
        bt_stream,
        scratch,
        global_fast,
        global_slow,
        global_metrics,
        *global_rows_len,
        metrics_count,
        metrics_stride,
        objective_i,
        hurdle_tau,
        topk_select,
        global_constraints_on_device,
        constraint_args,
        select_topk_rows_legacy,
        select_topk_rows_candidates_legacy,
        select_topk_rows_v2,
        select_topk_rows_candidates_v2,
        gather_topk_rows,
        launch_submit_elapsed,
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
    *global_rows_len = topk_select;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn select_tile_topk_rows_for_objective(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    pairs: usize,
    launch_pairs: usize,
    metrics_count: usize,
    tile_metrics_stride: usize,
    pf: usize,
    ps: usize,
    topk_select: usize,
    objective_i: i32,
    hurdle_tau: f32,
    compact_valid_pair_output: bool,
    pair_index_map_ptr: u64,
    pair_fast_rows_ptr: u64,
    pair_slow_rows_ptr: u64,
    constraints_on_device: bool,
    constraint_args: DeviceConstraintArgs,
    select_topk_pairs_legacy: &cust::function::Function<'_>,
    select_topk_candidates_legacy: &cust::function::Function<'_>,
    select_topk_pairs_v2: Option<&cust::function::Function<'_>>,
    select_topk_candidates_v2: Option<&cust::function::Function<'_>>,
    gather_topk: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
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
        let mut hurdle_tau_i = hurdle_tau;
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
            let mut use_min_trades = constraint_args.use_min_trades;
            let mut min_trades = constraint_args.min_trades;
            let mut use_max_dd = constraint_args.use_max_dd;
            let mut max_dd = constraint_args.max_dd;
            let mut use_min_sharpe = constraint_args.use_min_sharpe;
            let mut min_sharpe = constraint_args.min_sharpe;
            let mut use_min_sortino = constraint_args.use_min_sortino;
            let mut min_sortino = constraint_args.min_sortino;
            let mut use_min_calmar = constraint_args.use_min_calmar;
            let mut min_calmar = constraint_args.min_calmar;
            let mut use_max_volatility = constraint_args.use_max_volatility;
            let mut max_volatility = constraint_args.max_volatility;
            let mut use_min_exposure = constraint_args.use_min_exposure;
            let mut min_exposure = constraint_args.min_exposure;
            let mut use_max_exposure = constraint_args.use_max_exposure;
            let mut max_exposure = constraint_args.max_exposure;
            let mut use_max_abs_net_exposure = constraint_args.use_max_abs_net_exposure;
            let mut max_abs_net_exposure = constraint_args.max_abs_net_exposure;
            let mut use_min_omega = constraint_args.use_min_omega;
            let mut min_omega = constraint_args.min_omega;
            let mut use_max_ulcer = constraint_args.use_max_ulcer;
            let mut max_ulcer = constraint_args.max_ulcer;
            let mut use_max_downside_deviation = constraint_args.use_max_downside_deviation;
            let mut max_downside_deviation = constraint_args.max_downside_deviation;
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
                &mut hurdle_tau_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut use_min_trades as *mut _ as *mut c_void,
                &mut min_trades as *mut _ as *mut c_void,
                &mut use_max_dd as *mut _ as *mut c_void,
                &mut max_dd as *mut _ as *mut c_void,
                &mut use_min_sharpe as *mut _ as *mut c_void,
                &mut min_sharpe as *mut _ as *mut c_void,
                &mut use_min_sortino as *mut _ as *mut c_void,
                &mut min_sortino as *mut _ as *mut c_void,
                &mut use_min_calmar as *mut _ as *mut c_void,
                &mut min_calmar as *mut _ as *mut c_void,
                &mut use_max_volatility as *mut _ as *mut c_void,
                &mut max_volatility as *mut _ as *mut c_void,
                &mut use_min_exposure as *mut _ as *mut c_void,
                &mut min_exposure as *mut _ as *mut c_void,
                &mut use_max_exposure as *mut _ as *mut c_void,
                &mut max_exposure as *mut _ as *mut c_void,
                &mut use_max_abs_net_exposure as *mut _ as *mut c_void,
                &mut max_abs_net_exposure as *mut _ as *mut c_void,
                &mut use_min_omega as *mut _ as *mut c_void,
                &mut min_omega as *mut _ as *mut c_void,
                &mut use_max_ulcer as *mut _ as *mut c_void,
                &mut max_ulcer as *mut _ as *mut c_void,
                &mut use_max_downside_deviation as *mut _ as *mut c_void,
                &mut max_downside_deviation as *mut _ as *mut c_void,
                &mut pair_map_ptr as *mut _ as *mut c_void,
                &mut pair_count_i as *mut _ as *mut c_void,
                &mut compact_output_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_topk_pairs_v2
                    .ok_or_else(|| "missing select_topk_from_pairs_tm_f32_v2".to_string())?,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
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
                &mut hurdle_tau_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut pair_map_ptr as *mut _ as *mut c_void,
                &mut pair_count_i as *mut _ as *mut c_void,
                &mut compact_output_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_topk_pairs_legacy,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
        }
    }

    let mut blocks_prev = blocks1;
    let mut in_len = cand_len1;
    let mut in_is_a = true;
    while blocks_prev > 1 {
        let blocks_next = (in_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let out_len = blocks_next.saturating_mul(topk_select).max(topk_select);
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
            let mut hurdle_tau_i = hurdle_tau;
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
                let mut use_min_trades = constraint_args.use_min_trades;
                let mut min_trades = constraint_args.min_trades;
                let mut use_max_dd = constraint_args.use_max_dd;
                let mut max_dd = constraint_args.max_dd;
                let mut use_min_sharpe = constraint_args.use_min_sharpe;
                let mut min_sharpe = constraint_args.min_sharpe;
                let mut use_min_sortino = constraint_args.use_min_sortino;
                let mut min_sortino = constraint_args.min_sortino;
                let mut use_min_calmar = constraint_args.use_min_calmar;
                let mut min_calmar = constraint_args.min_calmar;
                let mut use_max_volatility = constraint_args.use_max_volatility;
                let mut max_volatility = constraint_args.max_volatility;
                let mut use_min_exposure = constraint_args.use_min_exposure;
                let mut min_exposure = constraint_args.min_exposure;
                let mut use_max_exposure = constraint_args.use_max_exposure;
                let mut max_exposure = constraint_args.max_exposure;
                let mut use_max_abs_net_exposure = constraint_args.use_max_abs_net_exposure;
                let mut max_abs_net_exposure = constraint_args.max_abs_net_exposure;
                let mut use_min_omega = constraint_args.use_min_omega;
                let mut min_omega = constraint_args.min_omega;
                let mut use_max_ulcer = constraint_args.use_max_ulcer;
                let mut max_ulcer = constraint_args.max_ulcer;
                let mut use_max_downside_deviation = constraint_args.use_max_downside_deviation;
                let mut max_downside_deviation = constraint_args.max_downside_deviation;
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
                    &mut hurdle_tau_i as *mut _ as *mut c_void,
                    &mut topk_i as *mut _ as *mut c_void,
                    &mut in_ptr as *mut _ as *mut c_void,
                    &mut in_len_i as *mut _ as *mut c_void,
                    &mut use_min_trades as *mut _ as *mut c_void,
                    &mut min_trades as *mut _ as *mut c_void,
                    &mut use_max_dd as *mut _ as *mut c_void,
                    &mut max_dd as *mut _ as *mut c_void,
                    &mut use_min_sharpe as *mut _ as *mut c_void,
                    &mut min_sharpe as *mut _ as *mut c_void,
                    &mut use_min_sortino as *mut _ as *mut c_void,
                    &mut min_sortino as *mut _ as *mut c_void,
                    &mut use_min_calmar as *mut _ as *mut c_void,
                    &mut min_calmar as *mut _ as *mut c_void,
                    &mut use_max_volatility as *mut _ as *mut c_void,
                    &mut max_volatility as *mut _ as *mut c_void,
                    &mut use_min_exposure as *mut _ as *mut c_void,
                    &mut min_exposure as *mut _ as *mut c_void,
                    &mut use_max_exposure as *mut _ as *mut c_void,
                    &mut max_exposure as *mut _ as *mut c_void,
                    &mut use_max_abs_net_exposure as *mut _ as *mut c_void,
                    &mut max_abs_net_exposure as *mut _ as *mut c_void,
                    &mut use_min_omega as *mut _ as *mut c_void,
                    &mut min_omega as *mut _ as *mut c_void,
                    &mut use_max_ulcer as *mut _ as *mut c_void,
                    &mut max_ulcer as *mut _ as *mut c_void,
                    &mut use_max_downside_deviation as *mut _ as *mut c_void,
                    &mut max_downside_deviation as *mut _ as *mut c_void,
                    &mut pair_map_ptr as *mut _ as *mut c_void,
                    &mut pair_count_i as *mut _ as *mut c_void,
                    &mut compact_output_i as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                launch_kernel_timed(
                    bt_stream,
                    select_topk_candidates_v2.ok_or_else(|| {
                        "missing select_topk_from_candidates_tm_f32_v2".to_string()
                    })?,
                    (grid_x, 1, 1),
                    (block_x, 1, 1),
                    0,
                    args,
                    launch_submit_elapsed,
                )?;
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
                    &mut hurdle_tau_i as *mut _ as *mut c_void,
                    &mut topk_i as *mut _ as *mut c_void,
                    &mut in_ptr as *mut _ as *mut c_void,
                    &mut in_len_i as *mut _ as *mut c_void,
                    &mut pair_map_ptr as *mut _ as *mut c_void,
                    &mut pair_count_i as *mut _ as *mut c_void,
                    &mut compact_output_i as *mut _ as *mut c_void,
                    &mut out_ptr as *mut _ as *mut c_void,
                ];
                launch_kernel_timed(
                    bt_stream,
                    select_topk_candidates_legacy,
                    (grid_x, 1, 1),
                    (block_x, 1, 1),
                    0,
                    args,
                    launch_submit_elapsed,
                )?;
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
        launch_kernel_timed(
            bt_stream,
            gather_topk,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn select_tile_topk_scores_for_objective(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    pairs: usize,
    launch_pairs: usize,
    metrics_count: usize,
    tile_metrics_stride: usize,
    pf: usize,
    ps: usize,
    topk_select: usize,
    objective_i: i32,
    hurdle_tau: f32,
    compact_valid_pair_output: bool,
    pair_index_map_ptr: u64,
    pair_fast_rows_ptr: u64,
    pair_slow_rows_ptr: u64,
    constraints_on_device: bool,
    constraint_args: DeviceConstraintArgs,
    select_scored_pairs: &cust::function::Function<'_>,
    select_scored_candidates: &cust::function::Function<'_>,
    select_scored_pairs_v2: Option<&cust::function::Function<'_>>,
    gather_scored_pairs: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
    let row_count = if pair_index_map_ptr != 0 && launch_pairs > 0 {
        launch_pairs
    } else {
        pairs
    };
    if row_count == 0 || topk_select == 0 {
        return Ok(());
    }

    let blocks1 = (row_count + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
    let cand_len1 = blocks1.saturating_mul(topk_select).max(topk_select);
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_a, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_b, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_cand_scores_a, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_cand_scores_b, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_top_fast, topk_select, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_top_slow, topk_select, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_top_scores, topk_select, bt_stream)?;

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
        let mut hurdle_tau_i = hurdle_tau;
        let mut topk_i = topk_select as i32;
        let mut pair_map_ptr = pair_index_map_ptr;
        let mut pair_count_i = launch_pairs as i32;
        let mut compact_output_i = if compact_valid_pair_output {
            1i32
        } else {
            0i32
        };
        let mut out_pairs = scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw();
        let mut out_scores = scratch
            .d_cand_scores_a
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();
        let block_x: u32 = 256;
        let grid_x: u32 = blocks1 as u32;
        if constraints_on_device {
            let mut use_min_trades = constraint_args.use_min_trades;
            let mut min_trades = constraint_args.min_trades;
            let mut use_max_dd = constraint_args.use_max_dd;
            let mut max_dd = constraint_args.max_dd;
            let mut use_min_sharpe = constraint_args.use_min_sharpe;
            let mut min_sharpe = constraint_args.min_sharpe;
            let mut use_min_sortino = constraint_args.use_min_sortino;
            let mut min_sortino = constraint_args.min_sortino;
            let mut use_min_calmar = constraint_args.use_min_calmar;
            let mut min_calmar = constraint_args.min_calmar;
            let mut use_max_volatility = constraint_args.use_max_volatility;
            let mut max_volatility = constraint_args.max_volatility;
            let mut use_min_exposure = constraint_args.use_min_exposure;
            let mut min_exposure = constraint_args.min_exposure;
            let mut use_max_exposure = constraint_args.use_max_exposure;
            let mut max_exposure = constraint_args.max_exposure;
            let mut use_max_abs_net_exposure = constraint_args.use_max_abs_net_exposure;
            let mut max_abs_net_exposure = constraint_args.max_abs_net_exposure;
            let mut use_min_omega = constraint_args.use_min_omega;
            let mut min_omega = constraint_args.min_omega;
            let mut use_max_ulcer = constraint_args.use_max_ulcer;
            let mut max_ulcer = constraint_args.max_ulcer;
            let mut use_max_downside_deviation = constraint_args.use_max_downside_deviation;
            let mut max_downside_deviation = constraint_args.max_downside_deviation;
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
                &mut hurdle_tau_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut use_min_trades as *mut _ as *mut c_void,
                &mut min_trades as *mut _ as *mut c_void,
                &mut use_max_dd as *mut _ as *mut c_void,
                &mut max_dd as *mut _ as *mut c_void,
                &mut use_min_sharpe as *mut _ as *mut c_void,
                &mut min_sharpe as *mut _ as *mut c_void,
                &mut use_min_sortino as *mut _ as *mut c_void,
                &mut min_sortino as *mut _ as *mut c_void,
                &mut use_min_calmar as *mut _ as *mut c_void,
                &mut min_calmar as *mut _ as *mut c_void,
                &mut use_max_volatility as *mut _ as *mut c_void,
                &mut max_volatility as *mut _ as *mut c_void,
                &mut use_min_exposure as *mut _ as *mut c_void,
                &mut min_exposure as *mut _ as *mut c_void,
                &mut use_max_exposure as *mut _ as *mut c_void,
                &mut max_exposure as *mut _ as *mut c_void,
                &mut use_max_abs_net_exposure as *mut _ as *mut c_void,
                &mut max_abs_net_exposure as *mut _ as *mut c_void,
                &mut use_min_omega as *mut _ as *mut c_void,
                &mut min_omega as *mut _ as *mut c_void,
                &mut use_max_ulcer as *mut _ as *mut c_void,
                &mut max_ulcer as *mut _ as *mut c_void,
                &mut use_max_downside_deviation as *mut _ as *mut c_void,
                &mut max_downside_deviation as *mut _ as *mut c_void,
                &mut pair_map_ptr as *mut _ as *mut c_void,
                &mut pair_count_i as *mut _ as *mut c_void,
                &mut compact_output_i as *mut _ as *mut c_void,
                &mut out_pairs as *mut _ as *mut c_void,
                &mut out_scores as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_scored_pairs_v2
                    .ok_or_else(|| "missing select_topk_scored_pairs_tm_f32_v2".to_string())?,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
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
                &mut hurdle_tau_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut pair_map_ptr as *mut _ as *mut c_void,
                &mut pair_count_i as *mut _ as *mut c_void,
                &mut compact_output_i as *mut _ as *mut c_void,
                &mut out_pairs as *mut _ as *mut c_void,
                &mut out_scores as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_scored_pairs,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
        }
    }

    let mut blocks_prev = blocks1;
    let mut in_len = cand_len1;
    let mut in_is_a = true;
    while blocks_prev > 1 {
        let blocks_next = (in_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let out_len = blocks_next.saturating_mul(topk_select).max(topk_select);
        unsafe {
            let mut in_pairs = if in_is_a {
                scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw()
            } else {
                scratch.d_cand_b.as_ref().unwrap().as_device_ptr().as_raw()
            };
            let mut in_scores = if in_is_a {
                scratch
                    .d_cand_scores_a
                    .as_ref()
                    .unwrap()
                    .as_device_ptr()
                    .as_raw()
            } else {
                scratch
                    .d_cand_scores_b
                    .as_ref()
                    .unwrap()
                    .as_device_ptr()
                    .as_raw()
            };
            let mut in_len_i = in_len as i32;
            let mut topk_i = topk_select as i32;
            let mut out_pairs = if in_is_a {
                scratch.d_cand_b.as_ref().unwrap().as_device_ptr().as_raw()
            } else {
                scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw()
            };
            let mut out_scores = if in_is_a {
                scratch
                    .d_cand_scores_b
                    .as_ref()
                    .unwrap()
                    .as_device_ptr()
                    .as_raw()
            } else {
                scratch
                    .d_cand_scores_a
                    .as_ref()
                    .unwrap()
                    .as_device_ptr()
                    .as_raw()
            };
            let block_x: u32 = 256;
            let grid_x: u32 = blocks_next as u32;
            let args: &mut [*mut c_void] = &mut [
                &mut in_pairs as *mut _ as *mut c_void,
                &mut in_scores as *mut _ as *mut c_void,
                &mut in_len_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut out_pairs as *mut _ as *mut c_void,
                &mut out_scores as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_scored_candidates,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
        }
        in_is_a = !in_is_a;
        blocks_prev = blocks_next;
        in_len = out_len;
    }

    unsafe {
        let mut pairs_i = pairs as i32;
        let mut pf_i = pf as i32;
        let mut ps_i = ps as i32;
        let mut f_per = scratch.d_fast_p.as_ref().unwrap().as_device_ptr().as_raw();
        let mut s_per = scratch.d_slow_p.as_ref().unwrap().as_device_ptr().as_raw();
        let mut top_pairs = if in_is_a {
            scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw()
        } else {
            scratch.d_cand_b.as_ref().unwrap().as_device_ptr().as_raw()
        };
        let mut top_scores = if in_is_a {
            scratch
                .d_cand_scores_a
                .as_ref()
                .unwrap()
                .as_device_ptr()
                .as_raw()
        } else {
            scratch
                .d_cand_scores_b
                .as_ref()
                .unwrap()
                .as_device_ptr()
                .as_raw()
        };
        let mut topk_i = topk_select as i32;
        let mut pair_map_ptr = pair_index_map_ptr;
        let mut pair_fast_rows = pair_fast_rows_ptr;
        let mut pair_slow_rows = pair_slow_rows_ptr;
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
        let mut out_scores = scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = ((topk_select as u32) + block_x - 1) / block_x;
        let args: &mut [*mut c_void] = &mut [
            &mut pairs_i as *mut _ as *mut c_void,
            &mut pf_i as *mut _ as *mut c_void,
            &mut ps_i as *mut _ as *mut c_void,
            &mut f_per as *mut _ as *mut c_void,
            &mut s_per as *mut _ as *mut c_void,
            &mut top_pairs as *mut _ as *mut c_void,
            &mut top_scores as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut pair_map_ptr as *mut _ as *mut c_void,
            &mut pair_fast_rows as *mut _ as *mut c_void,
            &mut pair_slow_rows as *mut _ as *mut c_void,
            &mut pair_count_i as *mut _ as *mut c_void,
            &mut compact_output_i as *mut _ as *mut c_void,
            &mut out_f as *mut _ as *mut c_void,
            &mut out_s as *mut _ as *mut c_void,
            &mut out_scores as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            gather_scored_pairs,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn select_tile_topk_scalar_scores(
    bt_stream: &Stream,
    scratch: &mut KernelScratch,
    pairs: usize,
    launch_pairs: usize,
    pf: usize,
    ps: usize,
    topk_select: usize,
    compact_valid_pair_output: bool,
    pair_index_map_ptr: u64,
    pair_fast_rows_ptr: u64,
    pair_slow_rows_ptr: u64,
    select_rows: &cust::function::Function<'_>,
    select_rows_candidates: &cust::function::Function<'_>,
    gather_scalar_pairs: &cust::function::Function<'_>,
    launch_submit_elapsed: &mut Duration,
) -> Result<(), String> {
    let row_count = if pair_index_map_ptr != 0 && launch_pairs > 0 {
        launch_pairs
    } else {
        pairs
    };
    if row_count == 0 || topk_select == 0 {
        return Ok(());
    }

    let blocks1 = (row_count + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
    let cand_len1 = blocks1.saturating_mul(topk_select).max(topk_select);
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_a, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_cand_b, cand_len1, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_top_fast, topk_select, bt_stream)?;
    KernelScratch::ensure_len_i32(&mut scratch.d_top_slow, topk_select, bt_stream)?;
    KernelScratch::ensure_len_f32(&mut scratch.d_top_scores, topk_select, bt_stream)?;

    unsafe {
        let mut scores_ptr = scratch
            .d_tile_scores
            .as_ref()
            .ok_or_else(|| "missing shadow score tile buffer".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut rows_i = row_count as i32;
        let mut topk_i = topk_select as i32;
        let mut out_ptr = scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = blocks1 as u32;
        let args: &mut [*mut c_void] = &mut [
            &mut scores_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut out_ptr as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            select_rows,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
    }

    let mut blocks_prev = blocks1;
    let mut in_len = cand_len1;
    let mut in_is_a = true;
    while blocks_prev > 1 {
        let blocks_next = (in_len + TOPK_BLOCK_ITEMS - 1) / TOPK_BLOCK_ITEMS;
        let out_len = blocks_next.saturating_mul(topk_select).max(topk_select);
        unsafe {
            let mut scores_ptr = scratch
                .d_tile_scores
                .as_ref()
                .ok_or_else(|| "missing shadow score tile buffer".to_string())?
                .as_device_ptr()
                .as_raw();
            let mut rows_i = row_count as i32;
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
            let args: &mut [*mut c_void] = &mut [
                &mut scores_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut in_ptr as *mut _ as *mut c_void,
                &mut in_len_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            launch_kernel_timed(
                bt_stream,
                select_rows_candidates,
                (grid_x, 1, 1),
                (block_x, 1, 1),
                0,
                args,
                launch_submit_elapsed,
            )?;
        }
        in_is_a = !in_is_a;
        blocks_prev = blocks_next;
        in_len = out_len;
    }

    unsafe {
        let mut scores_ptr = scratch
            .d_tile_scores
            .as_ref()
            .ok_or_else(|| "missing shadow score tile buffer".to_string())?
            .as_device_ptr()
            .as_raw();
        let mut rows_i = row_count as i32;
        let mut pairs_i = pairs as i32;
        let mut pf_i = pf as i32;
        let mut ps_i = ps as i32;
        let mut f_per = scratch.d_fast_p.as_ref().unwrap().as_device_ptr().as_raw();
        let mut s_per = scratch.d_slow_p.as_ref().unwrap().as_device_ptr().as_raw();
        let mut top_rows = if in_is_a {
            scratch.d_cand_a.as_ref().unwrap().as_device_ptr().as_raw()
        } else {
            scratch.d_cand_b.as_ref().unwrap().as_device_ptr().as_raw()
        };
        let mut topk_i = topk_select as i32;
        let mut pair_map_ptr = pair_index_map_ptr;
        let mut pair_fast_rows = pair_fast_rows_ptr;
        let mut pair_slow_rows = pair_slow_rows_ptr;
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
        let mut out_scores = scratch
            .d_top_scores
            .as_ref()
            .unwrap()
            .as_device_ptr()
            .as_raw();

        let block_x: u32 = 256;
        let grid_x: u32 = ((topk_select as u32) + block_x - 1) / block_x;
        let args: &mut [*mut c_void] = &mut [
            &mut scores_ptr as *mut _ as *mut c_void,
            &mut rows_i as *mut _ as *mut c_void,
            &mut pairs_i as *mut _ as *mut c_void,
            &mut pf_i as *mut _ as *mut c_void,
            &mut ps_i as *mut _ as *mut c_void,
            &mut f_per as *mut _ as *mut c_void,
            &mut s_per as *mut _ as *mut c_void,
            &mut top_rows as *mut _ as *mut c_void,
            &mut topk_i as *mut _ as *mut c_void,
            &mut pair_map_ptr as *mut _ as *mut c_void,
            &mut pair_fast_rows as *mut _ as *mut c_void,
            &mut pair_slow_rows as *mut _ as *mut c_void,
            &mut pair_count_i as *mut _ as *mut c_void,
            &mut compact_output_i as *mut _ as *mut c_void,
            &mut out_f as *mut _ as *mut c_void,
            &mut out_s as *mut _ as *mut c_void,
            &mut out_scores as *mut _ as *mut c_void,
        ];
        launch_kernel_timed(
            bt_stream,
            gather_scalar_pairs,
            (grid_x, 1, 1),
            (block_x, 1, 1),
            0,
            args,
            launch_submit_elapsed,
        )?;
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
            None,
            false,
            None,
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
    metrics_count_override: Option<usize>,
    allow_batched_windowed_ranges: bool,
    mut shortlist_slice_timing: Option<&mut ShortlistSliceTimingProfile>,
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
    let load_bt_fn = |name: &str| {
        bt_module
            .get_function(name)
            .map_err(|e| format!("{name}: {e}"))
    };
    let compute_returns = load_bt_fn("compute_log_and_simple_returns_f32")?;
    let compute_returns_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::ComputeReturns, &compute_returns);
    let use_fast_path = !fast_precomputed_enabled
        && !slow_precomputed_enabled
        && kernel_fast_path_enabled(fast_ma.as_str(), slow_ma.as_str());
    let transpose = load_bt_fn("transpose_row_to_tm")?;
    let backtest = load_bt_fn("double_cross_backtest_tm_flex_f32")?;
    let _backtest_shadow_sharpe = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_shadow_sharpe_score")
        .ok();
    let backtest_windowed = load_bt_fn("double_cross_backtest_tm_flex_f32_windowed")?;
    let backtest_windowed_m17 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m17")
        .ok();
    let backtest_windowed_m17_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk")
        .ok();
    let backtest_windowed_m17_regime_on_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk_eps0_nocomm")
        .ok();
    let backtest_windowed_m17_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk")
        .ok();
    let backtest_windowed_m17_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_windowed_m17_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_windowed_m22 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m22")
        .ok();
    let backtest_windowed_m22_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_windowed_m22_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk")
        .ok();
    let backtest_windowed_m22_regime_on_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk_eps0_nocomm")
        .ok();
    let backtest_windowed_m22_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk")
        .ok();
    let backtest_windowed_m22_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_m7 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m7")
        .ok();
    let backtest_m17 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17")
        .ok();
    let backtest_m17_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_off_norisk")
        .ok();
    let backtest_m17_regime_off_norisk_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_nocomm")
        .ok();
    let backtest_m17_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0")
        .ok();
    let backtest_m17_regime_off_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0_nocomm")
        .ok();
    let backtest_m17_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_on_norisk")
        .ok();
    let backtest_m17_regime_on_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_on_norisk_eps0_nocomm")
        .ok();
    let backtest_m17_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_hybrid_norisk")
        .ok();
    let backtest_m17_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_hybrid_norisk_eps0_nocomm")
        .ok();
    let backtest_m22 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22")
        .ok();
    let backtest_m22_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_off_norisk")
        .ok();
    let backtest_m22_regime_off_norisk_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_nocomm")
        .ok();
    let backtest_m22_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_eps0")
        .ok();
    let backtest_m22_regime_off_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_eps0_nocomm")
        .ok();
    let backtest_m22_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_on_norisk")
        .ok();
    let backtest_m22_regime_on_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_on_norisk_eps0_nocomm")
        .ok();
    let backtest_m22_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_hybrid_norisk")
        .ok();
    let backtest_m22_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_hybrid_norisk_eps0_nocomm")
        .ok();
    let backtest_lockstep = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time")
        .ok();
    let _backtest_lockstep_shadow_sharpe = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_shadow_sharpe_score")
        .ok();
    let backtest_lockstep_m7 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7")
        .ok();
    let backtest_lockstep_m17 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17")
        .ok();
    let backtest_lockstep_m17_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk")
        .ok();
    let backtest_lockstep_m17_regime_off_norisk_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_nocomm",
        )
        .ok();
    let backtest_lockstep_m17_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0")
        .ok();
    let backtest_lockstep_m17_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m17_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_on_norisk")
        .ok();
    let backtest_lockstep_m17_regime_on_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_on_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m17_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_hybrid_norisk")
        .ok();
    let backtest_lockstep_m17_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_hybrid_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m22 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22")
        .ok();
    let backtest_lockstep_m22_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk")
        .ok();
    let backtest_lockstep_m22_regime_off_norisk_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_nocomm",
        )
        .ok();
    let backtest_lockstep_m22_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0")
        .ok();
    let backtest_lockstep_m22_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m22_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_on_norisk")
        .ok();
    let backtest_lockstep_m22_regime_on_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_on_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m22_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_hybrid_norisk")
        .ok();
    let backtest_lockstep_m22_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_hybrid_norisk_eps0_nocomm",
        )
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
    let fill_f32 = load_bt_fn("fill_f32")?;
    let transpose_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::TransposeRowToTm, &transpose);
    let backtest_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktest, &backtest);
    let backtest_m7_launch_caps = backtest_m7
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM7, f));
    let backtest_m17_launch_caps = backtest_m17
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM17, f));
    let backtest_m22_launch_caps = backtest_m22
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM22, f));
    let backtest_lockstep_launch_caps = backtest_lockstep
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstep, f));
    let backtest_lockstep_m7_launch_caps = backtest_lockstep_m7
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstepM7, f));
    let backtest_lockstep_m17_launch_caps = backtest_lockstep_m17
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstepM17, f));
    let backtest_lockstep_m22_launch_caps = backtest_lockstep_m22
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstepM22, f));
    let backtest_fast_launch_caps = backtest_fast_warp
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestFastWarpDedup, f));
    let fill_launch_caps = rt.launch_caps_for_kernel(CachedKernelKey::FillF32, &fill_f32);

    let ma_source_key = normalize_market_source_key(ma_source);
    let ma_is_close = ma_source_key == "close";
    let alloc_ma_buf = needs_ma_prices && !ma_is_close;
    let (resident_market, ma, scratch, host_exact_combo) = (
        &mut rt.resident_market,
        &mut rt.ma,
        &mut rt.scratch,
        &mut rt.host_exact_combo_scratch,
    );
    let dataset_sig = candle_dataset_signature(candles);
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

    let mut replay_profile =
        kernel_finalist_profile_enabled().then(FinalistReplayTimingProfile::default);
    let returns_t0 = replay_profile.as_ref().map(|_| Instant::now());
    ensure_log_and_simple_returns(
        bt_stream,
        scratch,
        d_close,
        n,
        dataset_sig,
        &compute_returns,
        device_launch_caps,
        compute_returns_launch_caps,
    )?;
    if let (Some(t0), Some(p)) = (returns_t0, replay_profile.as_mut()) {
        p.returns = p.returns.saturating_add(t0.elapsed());
    }
    let d_lr_ptr = scratch.d_lr.as_ref().unwrap().as_device_ptr().as_raw();
    let d_ret1_ptr = scratch.d_ret1.as_ref().unwrap().as_device_ptr().as_raw();

    let use_windowed_eval = ranges
        .iter()
        .any(|&(eval_start_t, eval_end_t_raw)| eval_start_t != 0 || eval_end_t_raw.min(n) != n);
    let use_batched_windowed_ranges =
        allow_batched_windowed_ranges && use_windowed_eval && ranges.len() > 1;
    let metrics_count = metrics_count_override.unwrap_or_else(|| exact_eval_metrics_count(cfg));
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
        0,
        true,
        0,
    );
    let is_multi_tile_workload = pf_tile < f_total || ps_tile < s_total;
    let _ = is_multi_tile_workload;
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
    let _tile_score_cap = pf_tile
        .checked_mul(ps_tile)
        .ok_or_else(|| "pf_tile*ps_tile score overflow".to_string())?;

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
    KernelScratch::ensure_len_f32(
        &mut scratch.d_tile_scores,
        pf_tile.saturating_mul(ps_tile).max(1),
        bt_stream,
    )?;

    host_exact_combo.host_p_fast.clear();
    host_exact_combo.host_p_slow.clear();
    host_exact_combo.fast_precomputed_host_tile.clear();
    host_exact_combo.slow_precomputed_host_tile.clear();
    host_exact_combo.tile_combo_indices.clear();
    host_exact_combo.tile_pair_index_map.clear();
    host_exact_combo.tile_fast_period_rows.clear();
    host_exact_combo.tile_slow_period_rows.clear();
    host_exact_combo.tile_fast_local_rows.clear();
    host_exact_combo.tile_slow_local_rows.clear();
    host_exact_combo.tile_range_indices.clear();
    host_exact_combo.tile_eval_start_rows.clear();
    host_exact_combo.tile_eval_end_rows.clear();
    host_exact_combo.ensure_tile_host_buf_len(tile_out_cap)?;

    let strategy_kernel = kernel_strategy_params(strategy);
    let flags = strategy_kernel.flags;
    let commission = strategy_kernel.commission;
    let eps_rel = strategy_kernel.eps_rel;
    let hurdle_tau = strategy.hurdle_tau.unwrap_or(0.0) as f32;
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
        let fast_ma_t0 = replay_profile.as_ref().map(|_| Instant::now());
        let pf = pf_tile.min(f_total - f_start);
        let fast_tm_rows = if pad_tm {
            align_tm_stride(pf.max(1))
        } else {
            pf.max(1)
        };

        host_exact_combo.host_p_fast.clear();
        host_exact_combo.host_p_fast.extend(
            fast_periods[f_start..f_start + pf]
                .iter()
                .map(|&x| x as i32),
        );
        let f_pad = host_exact_combo.host_p_fast.last().copied().unwrap_or(2);
        host_exact_combo.host_p_fast.resize(pf_tile_tm, f_pad);
        let fast_upload_signature = period_upload_signature(&host_exact_combo.host_p_fast);
        if host_exact_combo.last_fast_period_upload != Some(fast_upload_signature) {
            scratch
                .d_fast_p
                .as_mut()
                .unwrap()
                .copy_from(&host_exact_combo.host_p_fast)
                .map_err(|e| e.to_string())?;
            host_exact_combo.last_fast_period_upload = Some(fast_upload_signature);
        }
        if let Some(precomputed_fast) = fast_precomputed {
            pack_precomputed_rows_tm_with_padding(
                precomputed_fast,
                f_start,
                pf,
                fast_tm_rows,
                &mut host_exact_combo.fast_precomputed_host_tile,
            )?;
            scratch
                .d_fast_ma_tm
                .as_mut()
                .ok_or_else(|| {
                    "internal error: missing d_fast_ma_tm for precomputed fast matrix".to_string()
                })?
                .copy_from(&host_exact_combo.fast_precomputed_host_tile)
                .map_err(|e| e.to_string())?;
        } else if use_fast_path {
            ma.compute_period_ma_tm_into(
                fast_ma.as_str(),
                fast_params,
                &vram_inputs,
                n,
                first_valid,
                &host_exact_combo.host_p_fast[..fast_tm_rows],
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
                &host_exact_combo.host_p_fast,
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
        if let (Some(t0), Some(p)) = (fast_ma_t0, replay_profile.as_mut()) {
            p.fast_ma = p.fast_ma.saturating_add(t0.elapsed());
        }

        let mut s_start = 0usize;
        while s_start < s_total {
            let slow_ma_t0 = replay_profile.as_ref().map(|_| Instant::now());
            let ps = ps_tile.min(s_total - s_start);
            let slow_tm_rows = if pad_tm {
                align_tm_stride(ps.max(1))
            } else {
                ps.max(1)
            };

            host_exact_combo.host_p_slow.clear();
            host_exact_combo.host_p_slow.extend(
                slow_periods[s_start..s_start + ps]
                    .iter()
                    .map(|&x| x as i32),
            );
            let s_pad = host_exact_combo.host_p_slow.last().copied().unwrap_or(2);
            host_exact_combo.host_p_slow.resize(ps_tile_tm, s_pad);
            let slow_upload_signature = period_upload_signature(&host_exact_combo.host_p_slow);
            if host_exact_combo.last_slow_period_upload != Some(slow_upload_signature) {
                scratch
                    .d_slow_p
                    .as_mut()
                    .unwrap()
                    .copy_from(&host_exact_combo.host_p_slow)
                    .map_err(|e| e.to_string())?;
                host_exact_combo.last_slow_period_upload = Some(slow_upload_signature);
            }
            if let Some(precomputed_slow) = slow_precomputed {
                pack_precomputed_rows_tm_with_padding(
                    precomputed_slow,
                    s_start,
                    ps,
                    slow_tm_rows,
                    &mut host_exact_combo.slow_precomputed_host_tile,
                )?;
                scratch
                    .d_slow_ma_tm
                    .as_mut()
                    .ok_or_else(|| {
                        "internal error: missing d_slow_ma_tm for precomputed slow matrix"
                            .to_string()
                    })?
                    .copy_from(&host_exact_combo.slow_precomputed_host_tile)
                    .map_err(|e| e.to_string())?;
            } else if use_fast_path {
                ma.compute_period_ma_tm_into(
                    slow_ma.as_str(),
                    slow_params,
                    &vram_inputs,
                    n,
                    first_valid,
                    &host_exact_combo.host_p_slow[..slow_tm_rows],
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
                    &host_exact_combo.host_p_slow,
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
            if let (Some(t0), Some(p)) = (slow_ma_t0, replay_profile.as_mut()) {
                p.slow_ma = p.slow_ma.saturating_add(t0.elapsed());
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

            let pairmap_t0 = replay_profile.as_ref().map(|_| Instant::now());
            host_exact_combo.tile_combo_indices.clear();
            host_exact_combo.tile_pair_index_map.clear();
            host_exact_combo.tile_fast_period_rows.clear();
            host_exact_combo.tile_slow_period_rows.clear();
            host_exact_combo.tile_fast_local_rows.clear();
            host_exact_combo.tile_slow_local_rows.clear();
            #[derive(Clone, Copy)]
            struct ReplayTileRow {
                combo_idx: usize,
                pair_index: i32,
                fast_period: i32,
                slow_period: i32,
                fast_local: i32,
                slow_local: i32,
            }
            let mut replay_tile_rows: Vec<ReplayTileRow> =
                Vec::with_capacity(combo_period_indices.len());
            for (combo_idx, &(global_f, global_s)) in combo_period_indices.iter().enumerate() {
                if global_f >= f_start
                    && global_f < f_start + pf
                    && global_s >= s_start
                    && global_s < s_start + ps
                {
                    let local_f = global_f - f_start;
                    let local_s = global_s - s_start;
                    replay_tile_rows.push(ReplayTileRow {
                        combo_idx,
                        pair_index: i32::try_from(
                            local_f.saturating_mul(ps).saturating_add(local_s),
                        )
                        .map_err(|_| "tile pair index exceeds i32::MAX".to_string())?,
                        fast_period: i32::from(fast_periods[global_f]),
                        slow_period: i32::from(slow_periods[global_s]),
                        fast_local: i32::try_from(local_f)
                            .map_err(|_| "tile fast local row exceeds i32::MAX".to_string())?,
                        slow_local: i32::try_from(local_s)
                            .map_err(|_| "tile slow local row exceeds i32::MAX".to_string())?,
                    });
                }
            }
            replay_tile_rows.sort_unstable_by_key(|row| (row.fast_local, row.slow_local));
            for row in replay_tile_rows {
                host_exact_combo.tile_combo_indices.push(row.combo_idx);
                host_exact_combo.tile_pair_index_map.push(row.pair_index);
                host_exact_combo.tile_fast_period_rows.push(row.fast_period);
                host_exact_combo.tile_slow_period_rows.push(row.slow_period);
                host_exact_combo.tile_fast_local_rows.push(row.fast_local);
                host_exact_combo.tile_slow_local_rows.push(row.slow_local);
            }
            if use_batched_windowed_ranges {
                prepare_batched_windowed_exact_replay_rows(host_exact_combo, ranges, n)?;
            } else {
                host_exact_combo.tile_range_indices.clear();
                host_exact_combo.tile_eval_start_rows.clear();
                host_exact_combo.tile_eval_end_rows.clear();
                host_exact_combo.last_eval_start_upload = None;
                host_exact_combo.last_eval_end_upload = None;
            }
            let launch_pairs = host_exact_combo.tile_combo_indices.len();
            let out_len = launch_pairs
                .checked_mul(tile_metrics_stride)
                .ok_or_else(|| "launch_pairs*metrics overflow".to_string())?;
            KernelScratch::ensure_len_f32(&mut scratch.d_tile, out_len.max(1), bt_stream)?;
            host_exact_combo.ensure_tile_host_buf_len(out_len)?;
            let mut pair_index_map_ptr: u64 = 0;
            let mut pair_fast_rows_ptr: u64 = 0;
            let mut pair_slow_rows_ptr: u64 = 0;
            let mut pair_fast_local_rows_ptr: u64 = 0;
            let mut pair_slow_local_rows_ptr: u64 = 0;
            let mut pair_eval_start_rows_ptr: u64 = 0;
            let mut pair_eval_end_rows_ptr: u64 = 0;
            let replay_descriptor_upload_t0 =
                shortlist_slice_timing.as_ref().map(|_| Instant::now());
            if launch_pairs > 0 {
                let pair_index_upload_signature =
                    period_upload_signature(&host_exact_combo.tile_pair_index_map);
                let eval_start_upload_signature = use_batched_windowed_ranges
                    .then(|| period_upload_signature(&host_exact_combo.tile_eval_start_rows));
                let eval_end_upload_signature = use_batched_windowed_ranges
                    .then(|| period_upload_signature(&host_exact_combo.tile_eval_end_rows));
                let needs_pair_upload = host_exact_combo.last_pair_index_upload
                    != Some(pair_index_upload_signature)
                    || host_exact_combo.last_eval_start_upload != eval_start_upload_signature
                    || host_exact_combo.last_eval_end_upload != eval_end_upload_signature;
                if needs_pair_upload {
                    KernelScratch::ensure_len_i32(
                        &mut scratch.d_valid_pair_index,
                        launch_pairs,
                        bt_stream,
                    )?;
                    scratch
                        .d_valid_pair_index
                        .as_mut()
                        .ok_or_else(|| "missing exact pair index buffer".to_string())?
                        .copy_from(&host_exact_combo.tile_pair_index_map)
                        .map_err(|e| e.to_string())?;
                    KernelScratch::ensure_len_i32(
                        &mut scratch.d_valid_pair_fast_period,
                        launch_pairs,
                        bt_stream,
                    )?;
                    scratch
                        .d_valid_pair_fast_period
                        .as_mut()
                        .ok_or_else(|| "missing exact fast period buffer".to_string())?
                        .copy_from(&host_exact_combo.tile_fast_period_rows)
                        .map_err(|e| e.to_string())?;
                    KernelScratch::ensure_len_i32(
                        &mut scratch.d_valid_pair_slow_period,
                        launch_pairs,
                        bt_stream,
                    )?;
                    scratch
                        .d_valid_pair_slow_period
                        .as_mut()
                        .ok_or_else(|| "missing exact slow period buffer".to_string())?
                        .copy_from(&host_exact_combo.tile_slow_period_rows)
                        .map_err(|e| e.to_string())?;
                    KernelScratch::ensure_len_i32(
                        &mut scratch.d_valid_pair_fast_local,
                        launch_pairs,
                        bt_stream,
                    )?;
                    scratch
                        .d_valid_pair_fast_local
                        .as_mut()
                        .ok_or_else(|| "missing exact fast local row buffer".to_string())?
                        .copy_from(&host_exact_combo.tile_fast_local_rows)
                        .map_err(|e| e.to_string())?;
                    KernelScratch::ensure_len_i32(
                        &mut scratch.d_valid_pair_slow_local,
                        launch_pairs,
                        bt_stream,
                    )?;
                    scratch
                        .d_valid_pair_slow_local
                        .as_mut()
                        .ok_or_else(|| "missing exact slow local row buffer".to_string())?
                        .copy_from(&host_exact_combo.tile_slow_local_rows)
                        .map_err(|e| e.to_string())?;
                    if use_batched_windowed_ranges {
                        KernelScratch::ensure_len_i32(
                            &mut scratch.d_valid_pair_eval_start,
                            launch_pairs,
                            bt_stream,
                        )?;
                        scratch
                            .d_valid_pair_eval_start
                            .as_mut()
                            .ok_or_else(|| "missing exact eval start row buffer".to_string())?
                            .copy_from(&host_exact_combo.tile_eval_start_rows)
                            .map_err(|e| e.to_string())?;
                        KernelScratch::ensure_len_i32(
                            &mut scratch.d_valid_pair_eval_end,
                            launch_pairs,
                            bt_stream,
                        )?;
                        scratch
                            .d_valid_pair_eval_end
                            .as_mut()
                            .ok_or_else(|| "missing exact eval end row buffer".to_string())?
                            .copy_from(&host_exact_combo.tile_eval_end_rows)
                            .map_err(|e| e.to_string())?;
                    }
                    host_exact_combo.last_pair_index_upload = Some(pair_index_upload_signature);
                    host_exact_combo.last_eval_start_upload = eval_start_upload_signature;
                    host_exact_combo.last_eval_end_upload = eval_end_upload_signature;
                }
                pair_index_map_ptr = scratch
                    .d_valid_pair_index
                    .as_ref()
                    .ok_or_else(|| "missing exact pair index buffer".to_string())?
                    .as_device_ptr()
                    .as_raw();
                pair_fast_rows_ptr = scratch
                    .d_valid_pair_fast_period
                    .as_ref()
                    .ok_or_else(|| "missing exact fast period buffer".to_string())?
                    .as_device_ptr()
                    .as_raw();
                pair_slow_rows_ptr = scratch
                    .d_valid_pair_slow_period
                    .as_ref()
                    .ok_or_else(|| "missing exact slow period buffer".to_string())?
                    .as_device_ptr()
                    .as_raw();
                pair_fast_local_rows_ptr = scratch
                    .d_valid_pair_fast_local
                    .as_ref()
                    .ok_or_else(|| "missing exact fast local row buffer".to_string())?
                    .as_device_ptr()
                    .as_raw();
                pair_slow_local_rows_ptr = scratch
                    .d_valid_pair_slow_local
                    .as_ref()
                    .ok_or_else(|| "missing exact slow local row buffer".to_string())?
                    .as_device_ptr()
                    .as_raw();
                if use_batched_windowed_ranges {
                    pair_eval_start_rows_ptr = scratch
                        .d_valid_pair_eval_start
                        .as_ref()
                        .ok_or_else(|| "missing exact eval start row buffer".to_string())?
                        .as_device_ptr()
                        .as_raw();
                    pair_eval_end_rows_ptr = scratch
                        .d_valid_pair_eval_end
                        .as_ref()
                        .ok_or_else(|| "missing exact eval end row buffer".to_string())?
                        .as_device_ptr()
                        .as_raw();
                }
            }
            if let Some(t0) = replay_descriptor_upload_t0 {
                if let Some(timing) = shortlist_slice_timing.as_deref_mut() {
                    timing.replay_descriptor_upload =
                        timing.replay_descriptor_upload.saturating_add(t0.elapsed());
                }
            }
            if let (Some(t0), Some(p)) = (pairmap_t0, replay_profile.as_mut()) {
                p.pairmap = p.pairmap.saturating_add(t0.elapsed());
                if launch_pairs > 0 {
                    p.tile_count = p.tile_count.saturating_add(1);
                }
            }
            let replay_launches: Vec<(usize, usize, usize, bool)> = if use_batched_windowed_ranges {
                vec![(0usize, 0usize, n, true)]
            } else {
                ranges
                    .iter()
                    .enumerate()
                    .map(|(range_idx, &(eval_start_t, eval_end_t_raw))| {
                        (range_idx, eval_start_t, eval_end_t_raw.min(n), false)
                    })
                    .collect()
            };
            for (range_idx, eval_start_t, eval_end_t, batched_launch) in replay_launches {
                if launch_pairs > 0 {
                    let backtest_t0 = replay_profile.as_ref().map(|_| Instant::now());
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
                        let mut hurdle_tau_f = hurdle_tau;
                        let mut stop_log = stop_log;
                        let mut take_log = take_log;
                        let mut flags_u = flags;
                        let mut m_i = metrics_count as i32;
                        let mut metrics_stride_i = tile_metrics_stride as i32;
                        let mut pair_map_ptr = pair_index_map_ptr;
                        let mut pair_fast_rows = pair_fast_rows_ptr;
                        let mut pair_slow_rows = pair_slow_rows_ptr;
                        let mut pair_fast_local_rows = pair_fast_local_rows_ptr;
                        let mut pair_slow_local_rows = pair_slow_local_rows_ptr;
                        let mut pair_eval_start_rows = if batched_launch {
                            pair_eval_start_rows_ptr
                        } else {
                            0
                        };
                        let mut pair_eval_end_rows = if batched_launch {
                            pair_eval_end_rows_ptr
                        } else {
                            0
                        };
                        let mut pair_count_i = launch_pairs as i32;
                        let mut compact_output_i = 1i32;
                        let mut out_ptr = scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut eval_start_i = if batched_launch {
                            0
                        } else {
                            eval_start_t as i32
                        };
                        let mut eval_end_i = if batched_launch {
                            n as i32
                        } else {
                            eval_end_t as i32
                        };
                        let no_risk_mode = (flags_u & (STRAT_STOP_LOSS | STRAT_TAKE_PROFIT)) == 0;
                        let zero_commission_mode = commission <= 0.0f32;
                        let eps_zero_mode = eps_rel <= 0.0f32;
                        let fixed_metrics_contract =
                            fixed_metrics_contract_rows(metrics_count, tile_metrics_stride);
                        let prefer_full_metric_specialization = match fixed_metrics_contract {
                            Some(METRICS_COUNT) => backtest_m17.is_some(),
                            Some(TRADE_METRICS_COUNT) => backtest_m22.is_some(),
                            _ => false,
                        };
                        let prefer_full_metric_lockstep_specialization =
                            match fixed_metrics_contract {
                                Some(METRICS_COUNT) => backtest_lockstep_m17.is_some(),
                                Some(TRADE_METRICS_COUNT) => backtest_lockstep_m22.is_some(),
                                _ => false,
                            };

                        let use_lockstep_time = !use_windowed_eval
                            && kernel_should_use_lockstep_time(
                                launch_pairs,
                                n,
                                max_start_t,
                                backtest_lockstep.is_some(),
                            );
                        let allow_m7_exact = metrics_count == METRICS_COUNT;

                        let force_low_pair_m7 = allow_m7_exact
                            && should_force_low_pair_m7_recompute(
                                prefer_low_pair_m7_dispatch,
                                use_windowed_eval,
                                is_multi_tile_workload,
                                launch_pairs,
                                backtest_m7.is_some(),
                            );
                        let use_m7 = allow_m7_exact
                            && (force_low_pair_m7
                                || (!prefer_full_metric_specialization
                                    && !use_windowed_eval
                                    && kernel_should_use_m7_specialize(
                                        launch_pairs,
                                        n,
                                        max_start_t,
                                        is_multi_tile_workload,
                                        metrics_count,
                                        backtest_m7.is_some(),
                                    )));
                        let use_lockstep_m7 = use_lockstep_time
                            && allow_m7_exact
                            && !is_multi_tile_workload
                            && !prefer_full_metric_lockstep_specialization
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
                        let fixed_contract_dispatch = choose_fixed_contract_dispatch_kind(
                            fixed_metrics_contract,
                            use_windowed_eval,
                            use_lockstep_time,
                            use_fast_warp_dedup,
                            use_m7,
                            use_lockstep_m7,
                            backtest_windowed_m17.is_some(),
                            backtest_windowed_m22.is_some(),
                            backtest_m17.is_some(),
                            backtest_m22.is_some(),
                            backtest_lockstep_m17.is_some(),
                            backtest_lockstep_m22.is_some(),
                        );
                        let use_windowed_m17 =
                            fixed_contract_dispatch == FixedContractDispatchKind::WindowedM17;
                        let use_windowed_m22 =
                            fixed_contract_dispatch == FixedContractDispatchKind::WindowedM22;
                        let use_lockstep_m17 =
                            fixed_contract_dispatch == FixedContractDispatchKind::LockstepM17;
                        let use_lockstep_m22 =
                            fixed_contract_dispatch == FixedContractDispatchKind::LockstepM22;
                        let use_m17 = fixed_contract_dispatch == FixedContractDispatchKind::M17;
                        let use_m22 = fixed_contract_dispatch == FixedContractDispatchKind::M22;
                        let launch_caps =
                            if use_windowed_m17 || use_windowed_m22 || use_windowed_eval {
                                backtest_launch_caps
                            } else if use_lockstep_m7 {
                                backtest_lockstep_m7_launch_caps
                                    .or(backtest_lockstep_launch_caps)
                                    .unwrap_or(backtest_launch_caps)
                            } else if use_lockstep_m17 {
                                backtest_lockstep_m17_launch_caps
                                    .or(backtest_lockstep_launch_caps)
                                    .unwrap_or(backtest_launch_caps)
                            } else if use_lockstep_m22 {
                                backtest_lockstep_m22_launch_caps
                                    .or(backtest_lockstep_launch_caps)
                                    .unwrap_or(backtest_launch_caps)
                            } else if use_lockstep_time {
                                backtest_lockstep_launch_caps.unwrap_or(backtest_launch_caps)
                            } else if use_fast_warp_dedup {
                                backtest_fast_launch_caps.unwrap_or(backtest_launch_caps)
                            } else if use_m17 {
                                backtest_m17_launch_caps.unwrap_or(backtest_launch_caps)
                            } else if use_m22 {
                                backtest_m22_launch_caps.unwrap_or(backtest_launch_caps)
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
                            if use_windowed_m17 {
                                "windowed_m17"
                            } else if use_windowed_m22 {
                                "windowed_m22"
                            } else {
                                "windowed"
                            }
                        } else if use_lockstep_m7 {
                            "lockstep_m7"
                        } else if use_lockstep_m17 {
                            "lockstep_m17"
                        } else if use_lockstep_m22 {
                            "lockstep_m22"
                        } else if use_lockstep_time {
                            "lockstep_time"
                        } else if use_fast_warp_dedup {
                            "fast_warp_dedup"
                        } else if use_m17 {
                            "m17"
                        } else if use_m22 {
                            "m22"
                        } else if use_m7 {
                            "m7"
                        } else {
                            "baseline"
                        };
                        if kernel_log_dispatch_enabled() {
                            let range_log_idx = if batched_launch {
                                -1i32
                            } else {
                                range_idx as i32
                            };
                            eprintln!(
                                "VECTORBT_KERNEL_EVAL_DISPATCH: pair={}/{} range_idx={} batched_windowed_ranges={} range_count={} launch_pairs={} dispatch={} use_m7={} use_m17={} use_m22={} use_lockstep_time={} use_lockstep_m7={} use_lockstep_m17={} use_lockstep_m22={} use_fast_warp={} metrics_count={} tile_metrics_stride={} fixed_metrics_contract={:?}",
                                fast_ma,
                                slow_ma,
                                range_log_idx,
                                batched_launch,
                                if batched_launch { ranges.len() } else { 1usize },
                                launch_pairs,
                                dispatch_label,
                                use_m7,
                                use_m17,
                                use_m22,
                                use_lockstep_time,
                                use_lockstep_m7,
                                use_lockstep_m17,
                                use_lockstep_m22,
                                use_fast_warp_dedup,
                                metrics_count,
                                tile_metrics_stride,
                                fixed_metrics_contract
                            );
                        }
                        let fixed_contract_flavor = choose_fixed_contract_kernel_flavor(
                            no_risk_mode,
                            use_regime_i != 0,
                            use_regime_hybrid_i != 0,
                            zero_commission_mode,
                            eps_zero_mode,
                        );
                        let backtest_fn = if use_windowed_eval {
                            if use_windowed_m17 {
                                match fixed_contract_flavor {
                                    FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm => {
                                        backtest_windowed_m17_regime_off_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeOnNoRisk => {
                                        backtest_windowed_m17_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm => {
                                        backtest_windowed_m17_regime_on_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m17_regime_on_norisk.as_ref())
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeHybridNoRisk => {
                                        backtest_windowed_m17_regime_hybrid_norisk
                                            .as_ref()
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm => {
                                        backtest_windowed_m17_regime_hybrid_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m17_regime_hybrid_norisk.as_ref())
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    _ => backtest_windowed_m17
                                        .as_ref()
                                        .expect("checked windowed m17 kernel availability"),
                                }
                            } else if use_windowed_m22 {
                                match fixed_contract_flavor {
                                    FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm => {
                                        backtest_windowed_m22_regime_off_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeOnNoRisk => {
                                        backtest_windowed_m22_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm => {
                                        backtest_windowed_m22_regime_on_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m22_regime_on_norisk.as_ref())
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeHybridNoRisk => {
                                        backtest_windowed_m22_regime_hybrid_norisk
                                            .as_ref()
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm => {
                                        backtest_windowed_m22_regime_hybrid_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m22_regime_hybrid_norisk.as_ref())
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    _ => backtest_windowed_m22
                                        .as_ref()
                                        .expect("checked windowed m22 kernel availability"),
                                }
                            } else {
                                &backtest_windowed
                            }
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
                        } else if use_lockstep_m17 {
                            if no_risk_mode {
                                if use_regime_hybrid_i != 0 {
                                    if zero_commission_mode && eps_zero_mode {
                                        backtest_lockstep_m17_regime_hybrid_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_lockstep_m17_regime_hybrid_norisk.as_ref())
                                            .or(backtest_lockstep_m17.as_ref())
                                            .expect("checked lockstep m17 kernel availability")
                                    } else {
                                        backtest_lockstep_m17_regime_hybrid_norisk
                                            .as_ref()
                                            .or(backtest_lockstep_m17.as_ref())
                                            .expect("checked lockstep m17 kernel availability")
                                    }
                                } else if use_regime_i != 0 {
                                    if zero_commission_mode && eps_zero_mode {
                                        backtest_lockstep_m17_regime_on_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_lockstep_m17_regime_on_norisk.as_ref())
                                            .or(backtest_lockstep_m17.as_ref())
                                            .expect("checked lockstep m17 kernel availability")
                                    } else {
                                        backtest_lockstep_m17_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_lockstep_m17.as_ref())
                                            .expect("checked lockstep m17 kernel availability")
                                    }
                                } else if zero_commission_mode {
                                    if eps_zero_mode {
                                        backtest_lockstep_m17_regime_off_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_lockstep_m17_regime_off_norisk_eps0
                                                .as_ref())
                                            .or(backtest_lockstep_m17_regime_off_norisk_nocomm
                                                .as_ref())
                                            .or(backtest_lockstep_m17_regime_off_norisk.as_ref())
                                            .or(backtest_lockstep_m17.as_ref())
                                            .expect("checked lockstep m17 kernel availability")
                                    } else {
                                        backtest_lockstep_m17_regime_off_norisk_nocomm
                                            .as_ref()
                                            .or(backtest_lockstep_m17_regime_off_norisk.as_ref())
                                            .or(backtest_lockstep_m17.as_ref())
                                            .expect("checked lockstep m17 kernel availability")
                                    }
                                } else if eps_zero_mode {
                                    backtest_lockstep_m17_regime_off_norisk_eps0
                                        .as_ref()
                                        .or(backtest_lockstep_m17_regime_off_norisk.as_ref())
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                } else {
                                    backtest_lockstep_m17_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                            } else {
                                backtest_lockstep_m17
                                    .as_ref()
                                    .expect("checked lockstep m17 kernel availability")
                            }
                        } else if use_lockstep_m22 {
                            if no_risk_mode {
                                if use_regime_hybrid_i != 0 {
                                    if zero_commission_mode && eps_zero_mode {
                                        backtest_lockstep_m22_regime_hybrid_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_lockstep_m22_regime_hybrid_norisk.as_ref())
                                            .or(backtest_lockstep_m22.as_ref())
                                            .expect("checked lockstep m22 kernel availability")
                                    } else {
                                        backtest_lockstep_m22_regime_hybrid_norisk
                                            .as_ref()
                                            .or(backtest_lockstep_m22.as_ref())
                                            .expect("checked lockstep m22 kernel availability")
                                    }
                                } else if use_regime_i != 0 {
                                    if zero_commission_mode && eps_zero_mode {
                                        backtest_lockstep_m22_regime_on_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_lockstep_m22_regime_on_norisk.as_ref())
                                            .or(backtest_lockstep_m22.as_ref())
                                            .expect("checked lockstep m22 kernel availability")
                                    } else {
                                        backtest_lockstep_m22_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_lockstep_m22.as_ref())
                                            .expect("checked lockstep m22 kernel availability")
                                    }
                                } else if zero_commission_mode {
                                    if eps_zero_mode {
                                        backtest_lockstep_m22_regime_off_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_lockstep_m22_regime_off_norisk_eps0
                                                .as_ref())
                                            .or(backtest_lockstep_m22_regime_off_norisk_nocomm
                                                .as_ref())
                                            .or(backtest_lockstep_m22_regime_off_norisk.as_ref())
                                            .or(backtest_lockstep_m22.as_ref())
                                            .expect("checked lockstep m22 kernel availability")
                                    } else {
                                        backtest_lockstep_m22_regime_off_norisk_nocomm
                                            .as_ref()
                                            .or(backtest_lockstep_m22_regime_off_norisk.as_ref())
                                            .or(backtest_lockstep_m22.as_ref())
                                            .expect("checked lockstep m22 kernel availability")
                                    }
                                } else if eps_zero_mode {
                                    backtest_lockstep_m22_regime_off_norisk_eps0
                                        .as_ref()
                                        .or(backtest_lockstep_m22_regime_off_norisk.as_ref())
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                } else {
                                    backtest_lockstep_m22_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                            } else {
                                backtest_lockstep_m22
                                    .as_ref()
                                    .expect("checked lockstep m22 kernel availability")
                            }
                        } else if use_lockstep_time {
                            backtest_lockstep
                                .as_ref()
                                .expect("checked lockstep kernel availability")
                        } else if use_fast_warp_dedup {
                            backtest_fast_warp
                                .as_ref()
                                .expect("checked fast dedup kernel availability")
                        } else if use_m17 {
                            if no_risk_mode {
                                if use_regime_hybrid_i != 0 {
                                    if zero_commission_mode && eps_zero_mode {
                                        backtest_m17_regime_hybrid_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_m17_regime_hybrid_norisk.as_ref())
                                            .or(backtest_m17.as_ref())
                                            .expect("checked m17 kernel availability")
                                    } else {
                                        backtest_m17_regime_hybrid_norisk
                                            .as_ref()
                                            .or(backtest_m17.as_ref())
                                            .expect("checked m17 kernel availability")
                                    }
                                } else if use_regime_i != 0 {
                                    if zero_commission_mode && eps_zero_mode {
                                        backtest_m17_regime_on_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_m17_regime_on_norisk.as_ref())
                                            .or(backtest_m17.as_ref())
                                            .expect("checked m17 kernel availability")
                                    } else {
                                        backtest_m17_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_m17.as_ref())
                                            .expect("checked m17 kernel availability")
                                    }
                                } else if zero_commission_mode {
                                    if eps_zero_mode {
                                        backtest_m17_regime_off_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_m17_regime_off_norisk_eps0.as_ref())
                                            .or(backtest_m17_regime_off_norisk_nocomm.as_ref())
                                            .or(backtest_m17_regime_off_norisk.as_ref())
                                            .or(backtest_m17.as_ref())
                                            .expect("checked m17 kernel availability")
                                    } else {
                                        backtest_m17_regime_off_norisk_nocomm
                                            .as_ref()
                                            .or(backtest_m17_regime_off_norisk.as_ref())
                                            .or(backtest_m17.as_ref())
                                            .expect("checked m17 kernel availability")
                                    }
                                } else if eps_zero_mode {
                                    backtest_m17_regime_off_norisk_eps0
                                        .as_ref()
                                        .or(backtest_m17_regime_off_norisk.as_ref())
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                } else {
                                    backtest_m17_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                            } else {
                                backtest_m17
                                    .as_ref()
                                    .expect("checked m17 kernel availability")
                            }
                        } else if use_m22 {
                            if no_risk_mode {
                                if use_regime_hybrid_i != 0 {
                                    if zero_commission_mode && eps_zero_mode {
                                        backtest_m22_regime_hybrid_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_m22_regime_hybrid_norisk.as_ref())
                                            .or(backtest_m22.as_ref())
                                            .expect("checked m22 kernel availability")
                                    } else {
                                        backtest_m22_regime_hybrid_norisk
                                            .as_ref()
                                            .or(backtest_m22.as_ref())
                                            .expect("checked m22 kernel availability")
                                    }
                                } else if use_regime_i != 0 {
                                    if zero_commission_mode && eps_zero_mode {
                                        backtest_m22_regime_on_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_m22_regime_on_norisk.as_ref())
                                            .or(backtest_m22.as_ref())
                                            .expect("checked m22 kernel availability")
                                    } else {
                                        backtest_m22_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_m22.as_ref())
                                            .expect("checked m22 kernel availability")
                                    }
                                } else if zero_commission_mode {
                                    if eps_zero_mode {
                                        backtest_m22_regime_off_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_m22_regime_off_norisk_eps0.as_ref())
                                            .or(backtest_m22_regime_off_norisk_nocomm.as_ref())
                                            .or(backtest_m22_regime_off_norisk.as_ref())
                                            .or(backtest_m22.as_ref())
                                            .expect("checked m22 kernel availability")
                                    } else {
                                        backtest_m22_regime_off_norisk_nocomm
                                            .as_ref()
                                            .or(backtest_m22_regime_off_norisk.as_ref())
                                            .or(backtest_m22.as_ref())
                                            .expect("checked m22 kernel availability")
                                    }
                                } else if eps_zero_mode {
                                    backtest_m22_regime_off_norisk_eps0
                                        .as_ref()
                                        .or(backtest_m22_regime_off_norisk.as_ref())
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                } else {
                                    backtest_m22_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                            } else {
                                backtest_m22
                                    .as_ref()
                                    .expect("checked m22 kernel availability")
                            }
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
                            let mut compact_output_i = 1i32;
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
                                &mut pair_eval_start_rows as *mut _ as *mut c_void,
                                &mut pair_eval_end_rows as *mut _ as *mut c_void,
                                &mut commission as *mut _ as *mut c_void,
                                &mut eps_rel as *mut _ as *mut c_void,
                                &mut hurdle_tau_f as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_fast_rows as *mut _ as *mut c_void,
                                &mut pair_slow_rows as *mut _ as *mut c_void,
                                &mut pair_fast_local_rows as *mut _ as *mut c_void,
                                &mut pair_slow_local_rows as *mut _ as *mut c_void,
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
                                &mut hurdle_tau_f as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_fast_rows as *mut _ as *mut c_void,
                                &mut pair_slow_rows as *mut _ as *mut c_void,
                                &mut pair_fast_local_rows as *mut _ as *mut c_void,
                                &mut pair_slow_local_rows as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            bt_stream
                                .launch(backtest_fn, (grid_x, 1, 1), (block_x, 1, 1), 0, args)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    if let (Some(t0), Some(p)) = (backtest_t0, replay_profile.as_mut()) {
                        p.backtest = p.backtest.saturating_add(t0.elapsed());
                    }
                }
                let d2h_t0 = replay_profile.as_ref().map(|_| Instant::now());
                bt_stream.synchronize().map_err(|e| e.to_string())?;

                if launch_pairs > 0 {
                    let host_metrics = host_exact_combo.tile_host_buf.copy_device_prefix_to_host(
                        scratch
                            .d_tile
                            .as_ref()
                            .ok_or_else(|| "missing tile buffer".to_string())?,
                        out_len,
                        bt_stream,
                    )?;
                    if batched_launch {
                        for (row_idx, &combo_idx) in
                            host_exact_combo.tile_combo_indices.iter().enumerate()
                        {
                            let src_base = row_idx * tile_metrics_stride;
                            let range_row_idx = host_exact_combo
                                .tile_range_indices
                                .get(row_idx)
                                .copied()
                                .ok_or_else(|| {
                                    "missing batched replay range index for decoded row".to_string()
                                })?;
                            out_metrics[range_row_idx * combo_count + combo_idx] =
                                decode_tm_flex_metrics_row(
                                    host_metrics,
                                    src_base,
                                    metrics_count,
                                    cfg.hurdle_tau,
                                );
                        }
                    } else {
                        for (row_idx, &combo_idx) in
                            host_exact_combo.tile_combo_indices.iter().enumerate()
                        {
                            let src_base = row_idx * tile_metrics_stride;
                            out_metrics[range_idx * combo_count + combo_idx] =
                                decode_tm_flex_metrics_row(
                                    host_metrics,
                                    src_base,
                                    metrics_count,
                                    cfg.hurdle_tau,
                                );
                        }
                    }
                }
                if let (Some(t0), Some(p)) = (d2h_t0, replay_profile.as_mut()) {
                    p.d2h_decode = p.d2h_decode.saturating_add(t0.elapsed());
                }
            }

            s_start += ps;
        }

        f_start += pf;
    }

    if !kernel_persist_buffers_enabled() {
        ma.clear_cached_constants();
        scratch.clear_async(bt_stream)?;
        host_exact_combo.last_fast_period_upload = None;
        host_exact_combo.last_slow_period_upload = None;
        host_exact_combo.last_pair_index_upload = None;
        host_exact_combo.last_eval_start_upload = None;
        host_exact_combo.last_eval_end_upload = None;
    }
    if let Some(p) = replay_profile.as_ref() {
        p.emit(
            fast_ma.as_str(),
            slow_ma.as_str(),
            combos.len(),
            ranges.len(),
        );
    }

    Ok(out_metrics)
}

#[allow(clippy::too_many_arguments)]
fn rerank_shortlist_by_slice_robustness(
    rt: &mut CudaKernelRuntime,
    candles: &Candles,
    rows: &[(DoubleMaParams, Metrics)],
    return_limit: usize,
    diagnostic_limit: usize,
    mode: ShortlistSliceMode,
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
    objective: ObjectiveKind,
    score_mode: Option<DoubleMaScoreMode>,
    benchmark: Option<&SpaBenchmarkKind>,
    first_valid: usize,
    regime_first_valid: usize,
    eval_start: usize,
    eval_end: usize,
    mut timing: Option<&mut ShortlistSliceTimingProfile>,
) -> Result<(Vec<(DoubleMaParams, Metrics)>, ShortlistSliceDiagnostics), String> {
    if rows.is_empty() {
        return Ok((
            Vec::new(),
            ShortlistSliceDiagnostics {
                ranking_mode: mode.as_str().to_string(),
                neighborhood_robustness: None,
                candidates: Vec::new(),
            },
        ));
    }

    let active_records = compute_shortlist_slice_mode_records(
        rt,
        candles,
        rows,
        mode,
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        cfg,
        fast_precomputed,
        slow_precomputed,
        strategy,
        objective,
        score_mode,
        benchmark,
        first_valid,
        regime_first_valid,
        eval_start,
        eval_end,
        timing.as_deref_mut(),
    )?;
    let mut ranked_active: Vec<(DoubleMaParams, Metrics, ShortlistSliceModeDiagnostics)> =
        active_records
            .into_iter()
            .map(|record| (record.params, record.full_metrics, record.diagnostics))
            .collect();

    let sort_t0 = Instant::now();
    ranked_active.sort_by(|a, b| {
        b.2.worst_slice_score
            .total_cmp(&a.2.worst_slice_score)
            .then_with(|| {
                b.2.worst_omission_score
                    .total_cmp(&a.2.worst_omission_score)
            })
            .then_with(|| b.2.positive_slice_frac.total_cmp(&a.2.positive_slice_frac))
            .then_with(|| {
                a.2.omission_score_spread
                    .total_cmp(&b.2.omission_score_spread)
            })
            .then_with(|| b.2.mean_slice_score.total_cmp(&a.2.mean_slice_score))
            .then_with(|| {
                let sa = effective_score(&a.1, objective, score_mode);
                let sb = effective_score(&b.1, objective, score_mode);
                sb.total_cmp(&sa)
            })
            .then_with(|| a.1.max_dd.total_cmp(&b.1.max_dd))
    });
    let sort_elapsed = sort_t0.elapsed();
    if let Some(timing) = timing.as_deref_mut() {
        timing.sort = timing.sort.saturating_add(sort_elapsed);
    }

    let diag_count = diagnostic_limit
        .min(ranked_active.len())
        .min(kernel_shortlist_slice_diagnostic_row_limit());
    let diag_rows_t0 = Instant::now();
    let diagnostic_rows: Vec<(DoubleMaParams, Metrics)> = ranked_active
        .iter()
        .take(diag_count)
        .map(|(params, metrics, _)| (params.clone(), metrics.clone()))
        .collect();
    let diagnostic_rows_elapsed = diag_rows_t0.elapsed();
    if let Some(timing) = timing.as_deref_mut() {
        timing.diagnostics = timing.diagnostics.saturating_add(diagnostic_rows_elapsed);
    }

    let halves_records = if mode == ShortlistSliceMode::Halves {
        ranked_active
            .iter()
            .take(diag_count)
            .map(
                |(params, metrics, diagnostics)| ShortlistSliceModeCandidateRecord {
                    params: params.clone(),
                    full_metrics: metrics.clone(),
                    diagnostics: diagnostics.clone(),
                },
            )
            .collect()
    } else {
        compute_shortlist_slice_mode_records(
            rt,
            candles,
            diagnostic_rows.as_slice(),
            ShortlistSliceMode::Halves,
            fast_ma_type,
            slow_ma_type,
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter,
            cfg,
            fast_precomputed,
            slow_precomputed,
            strategy,
            objective,
            score_mode,
            benchmark,
            first_valid,
            regime_first_valid,
            eval_start,
            eval_end,
            timing.as_deref_mut(),
        )?
    };
    let quartiles_records = if mode == ShortlistSliceMode::Quartiles {
        ranked_active
            .iter()
            .take(diag_count)
            .map(
                |(params, metrics, diagnostics)| ShortlistSliceModeCandidateRecord {
                    params: params.clone(),
                    full_metrics: metrics.clone(),
                    diagnostics: diagnostics.clone(),
                },
            )
            .collect()
    } else {
        compute_shortlist_slice_mode_records(
            rt,
            candles,
            diagnostic_rows.as_slice(),
            ShortlistSliceMode::Quartiles,
            fast_ma_type,
            slow_ma_type,
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter,
            cfg,
            fast_precomputed,
            slow_precomputed,
            strategy,
            objective,
            score_mode,
            benchmark,
            first_valid,
            regime_first_valid,
            eval_start,
            eval_end,
            timing.as_deref_mut(),
        )?
    };

    let diagnostics_build_t0 = Instant::now();
    let mut candidate_diags: Vec<ShortlistSliceCandidateDiagnostics> =
        Vec::with_capacity(diag_count);
    for ((halves_record, quartiles_record), (params, full_metrics, _)) in halves_records
        .into_iter()
        .zip(quartiles_records.into_iter())
        .zip(ranked_active.iter().take(diag_count))
    {
        let halves_key = (halves_record.params.fast_len, halves_record.params.slow_len);
        let quartiles_key = (
            quartiles_record.params.fast_len,
            quartiles_record.params.slow_len,
        );
        let ranked_key = (params.fast_len, params.slow_len);
        if halves_key != ranked_key {
            return Err(format!(
                "misaligned half-split slice diagnostics: expected {ranked_key:?}, got {halves_key:?}"
            ));
        }
        if quartiles_key != ranked_key {
            return Err(format!(
                "misaligned quartile slice diagnostics: expected {ranked_key:?}, got {quartiles_key:?}"
            ));
        }
        let candidate_diag = ShortlistSliceCandidateDiagnostics {
            params: params.clone(),
            full_metrics: full_metrics.clone(),
            halves: halves_record.diagnostics,
            quartiles: quartiles_record.diagnostics,
        };
        candidate_diags.push(candidate_diag);
    }
    let diagnostics_build_elapsed = diagnostics_build_t0.elapsed();
    if let Some(timing) = timing.as_deref_mut() {
        timing.diagnostics = timing.diagnostics.saturating_add(diagnostics_build_elapsed);
    }

    let reranked_rows = ranked_active
        .iter()
        .take(return_limit.max(1))
        .map(|(params, metrics, _)| (params.clone(), metrics.clone()))
        .collect();
    let diagnostics = ShortlistSliceDiagnostics {
        ranking_mode: mode.as_str().to_string(),
        neighborhood_robustness: None,
        candidates: candidate_diags,
    };
    Ok((reranked_rows, diagnostics))
}

fn exact_eval_metrics_for_params(
    rt: &mut CudaKernelRuntime,
    candles: &Candles,
    params: &[DoubleMaParams],
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
    if params.is_empty() {
        return Ok(Vec::new());
    }

    let mut unique_params: Vec<DoubleMaParams> = Vec::new();
    let mut unique_indices: Vec<usize> = Vec::with_capacity(params.len());
    let mut seen: HashMap<(u16, u16), usize> = HashMap::new();
    for params in params {
        let key = (params.fast_len, params.slow_len);
        let idx = if let Some(&idx) = seen.get(&key) {
            idx
        } else {
            let idx = unique_params.len();
            seen.insert(key, idx);
            unique_params.push(params.clone());
            idx
        };
        unique_indices.push(idx);
    }

    let mut finalist_fast_periods: Vec<u16> =
        unique_params.iter().map(|params| params.fast_len).collect();
    finalist_fast_periods.sort_unstable();
    finalist_fast_periods.dedup();
    let mut finalist_slow_periods: Vec<u16> =
        unique_params.iter().map(|params| params.slow_len).collect();
    finalist_slow_periods.sort_unstable();
    finalist_slow_periods.dedup();

    let ranges = [finalist_recompute_eval_range(
        eval_range,
        candles.close.len(),
    )];
    let recomputed = eval_double_ma_batch_gpu_kernel_impl(
        rt,
        candles,
        unique_params.as_slice(),
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
        ranges.as_slice(),
        None,
        false,
        None,
    )?;
    if recomputed.len() != unique_params.len() {
        return Err(format!(
            "exact metrics recompute returned {} metrics for {} unique params",
            recomputed.len(),
            unique_params.len()
        ));
    }
    Ok(unique_indices
        .into_iter()
        .map(|idx| recomputed[idx].clone())
        .collect())
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
    eval_range: Option<(usize, usize)>,
) -> Result<(), String> {
    let mut finalists: Vec<DoubleMaParams> = Vec::with_capacity(1 + result.top.len());
    finalists.push(result.best_params.clone());
    finalists.extend(result.top.iter().map(|(params, _)| params.clone()));

    let recomputed = exact_eval_metrics_for_params(
        rt,
        candles,
        finalists.as_slice(),
        fast_ma_type,
        slow_ma_type,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        regime_filter,
        cfg,
        strategy,
        eval_range,
    )?;

    result.best_metrics = recomputed[0].clone();
    for ((_, metrics), recomputed_metrics) in result.top.iter_mut().zip(recomputed.iter().skip(1)) {
        *metrics = recomputed_metrics.clone();
    }

    Ok(())
}

#[inline]
fn finalist_recompute_eval_range(
    eval_range: Option<(usize, usize)>,
    total_bars: usize,
) -> (usize, usize) {
    eval_range.unwrap_or((0, total_bars))
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
    score_mode: Option<DoubleMaScoreMode>,
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
        score_mode,
        None,
        None,
        None,
        None,
        None,
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
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
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
        score_mode,
        extra_shortlist_objectives,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        shortlist_slice_benchmark,
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
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
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
        score_mode,
        extra_shortlist_objectives,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        shortlist_slice_benchmark,
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
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
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
    let _nvtx_scope = crate::nvtx::scope("kernel", "kernel_executor_total");
    clear_last_kernel_executor_timing();
    clear_last_route_diagnostics();
    clear_last_shortlist_slice_diagnostics();
    clear_last_score_observation_context();
    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<OptimizationResult, String> {
        let mut map = cell.borrow_mut();
        let mut runtime_init_elapsed = Duration::ZERO;
        if !map.contains_key(&device_id) {
            let _nvtx_scope = crate::nvtx::scope("kernel", "runtime_init");
            let t0 = Instant::now();
            let rt = CudaKernelRuntime::new(device_id)?;
            map.insert(device_id, rt);
            runtime_init_elapsed = t0.elapsed();
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        let set_current_elapsed = {
            let _nvtx_scope = crate::nvtx::scope("kernel", "set_current");
            let set_current_t0 = Instant::now();
            rt.set_current()?;
            set_current_t0.elapsed()
        };

        let (result, mut timing) = optimize_double_ma_gpu_kernel_impl(
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
            score_mode,
            extra_shortlist_objectives,
            shortlist_confidence_mode,
            shortlist_union_mode,
            shortlist_slice_mode,
            shortlist_slice_benchmark,
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
        timing.runtime_init_ms = runtime_init_elapsed.as_millis() as u64;
        timing.set_current_ms = set_current_elapsed.as_millis() as u64;
        record_last_kernel_executor_timing(timing);
        Ok(result)
    })
}

pub fn campaign_stage_plain_double_ma_market(
    device_id: u32,
    candles: &Candles,
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
) -> Result<CampaignVramPrefetchOutcome, String> {
    CUDA_KERNEL_RUNTIME.with(|cell| -> Result<CampaignVramPrefetchOutcome, String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&device_id) {
            map.insert(device_id, CudaKernelRuntime::new(device_id)?);
        }
        let rt = map.get_mut(&device_id).expect("just inserted");
        rt.set_current()?;
        stage_plain_double_ma_market_with_runtime(
            rt,
            candles,
            fast_ma_type,
            slow_ma_type,
            ma_source,
        )
    })
}

fn stage_plain_double_ma_market_with_runtime(
    rt: &mut CudaKernelRuntime,
    candles: &Candles,
    fast_ma_type: &str,
    slow_ma_type: &str,
    ma_source: &str,
) -> Result<CampaignVramPrefetchOutcome, String> {
    let fast_ma = fast_ma_type.trim().to_ascii_lowercase();
    let slow_ma = slow_ma_type.trim().to_ascii_lowercase();
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

    stage_resident_market_series_for_next_dataset(
        &rt.resident_market,
        &rt.staging_stream,
        &mut rt.staged_market,
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
    )
}

fn maybe_stage_campaign_prefetch(
    current_device_id: u32,
    resident_market: &Option<ResidentMarketSeriesCache>,
    staging_stream: &Stream,
    staged_market: &mut Option<ResidentMarketSeriesCache>,
    progress: Option<&dyn ProgressSink>,
) {
    let Some(progress) = progress else { return };
    let mut feedback = None;
    progress.with_double_ma_campaign_vram_prefetch_request(&mut |device_id, candles, fast_ma, slow_ma, ma_source| {
        let stage_started = Instant::now();
        feedback = Some(if device_id != current_device_id {
            crate::progress::DoubleMaCampaignVramPrefetchFeedback::Fatal {
                note: format!(
                    "double_ma VRAM staging disabled: runtime device {} did not match requested device {}",
                    current_device_id, device_id
                ),
                stage_wall_ms: stage_started.elapsed().as_secs_f64() * 1000.0,
            }
        } else {
            let fast_ma = fast_ma.trim().to_ascii_lowercase();
            let slow_ma = slow_ma.trim().to_ascii_lowercase();
            let needs_ma_prices =
                ma_uses_source(fast_ma.as_str()) || ma_uses_source(slow_ma.as_str());
            let needs_volume =
                ma_requires_volume(fast_ma.as_str()) || ma_requires_volume(slow_ma.as_str());
            let needs_high_low = ma_requires_high_low_close(fast_ma.as_str())
                || ma_requires_high_low_close(slow_ma.as_str());

            match validate_candles_for_selected_mas(
                candles,
                needs_ma_prices,
                ma_source,
                needs_volume,
                needs_high_low,
            )
            .and_then(|_| {
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
                stage_resident_market_series_for_next_dataset(
                    resident_market,
                    staging_stream,
                    staged_market,
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
                )
            }) {
                Ok(CampaignVramPrefetchOutcome::Staged) => {
                    crate::progress::DoubleMaCampaignVramPrefetchFeedback::Staged {
                        stage_wall_ms: stage_started.elapsed().as_secs_f64() * 1000.0,
                    }
                }
                Ok(CampaignVramPrefetchOutcome::AlreadyResident) => {
                    crate::progress::DoubleMaCampaignVramPrefetchFeedback::AlreadyResident {
                        stage_wall_ms: stage_started.elapsed().as_secs_f64() * 1000.0,
                    }
                }
                Ok(CampaignVramPrefetchOutcome::Skipped(reason)) => {
                    crate::progress::DoubleMaCampaignVramPrefetchFeedback::Warning {
                        note: reason,
                        stage_wall_ms: stage_started.elapsed().as_secs_f64() * 1000.0,
                    }
                }
                Err(error) => crate::progress::DoubleMaCampaignVramPrefetchFeedback::Fatal {
                    note: format!("double_ma VRAM staging disabled: {error}"),
                    stage_wall_ms: stage_started.elapsed().as_secs_f64() * 1000.0,
                },
            }
        });
    });
    if let Some(feedback) = feedback {
        progress.on_double_ma_campaign_vram_prefetch_feedback(feedback);
    }
}

fn emit_progress_and_maybe_stage_campaign_prefetch(
    current_device_id: u32,
    resident_market: &Option<ResidentMarketSeriesCache>,
    staging_stream: &Stream,
    staged_market: &mut Option<ResidentMarketSeriesCache>,
    progress: Option<&dyn ProgressSink>,
    processed_pairs: usize,
    total_pairs: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed_pairs, total_pairs, phase);
    if phase == "gpu-kernel" && processed_pairs > 0 {
        maybe_stage_campaign_prefetch(
            current_device_id,
            resident_market,
            staging_stream,
            staged_market,
            progress,
        );
    }
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
    score_mode: Option<DoubleMaScoreMode>,
    extra_shortlist_objectives: Option<&[ObjectiveKind]>,
    shortlist_confidence_mode: Option<DoubleMaShortlistConfidenceMode>,
    shortlist_union_mode: Option<DoubleMaShortlistUnionMode>,
    requested_shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
    requested_shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
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
    let _executor_impl_scope = crate::nvtx::scope("kernel", "executor_impl");
    let extra_shortlist_objectives =
        kernel_extra_shortlist_objectives(objective, extra_shortlist_objectives)?;
    let extra_shortlist_enabled = !extra_shortlist_objectives.is_empty();
    let shortlist_confidence_mode = kernel_shortlist_confidence_mode(shortlist_confidence_mode)?;
    let shortlist_union_mode = kernel_shortlist_union_mode(shortlist_union_mode)?;
    let shortlist_slice_mode = kernel_shortlist_slice_mode(requested_shortlist_slice_mode)?;
    let shortlist_slice_benchmark =
        kernel_shortlist_slice_benchmark(requested_shortlist_slice_benchmark)?;
    if extra_shortlist_enabled {
        if top_k == 0 {
            return Err(
                "extra shortlist objectives require top_k > 0 so the union shortlist has a bounded head size"
                    .to_string(),
            );
        }
        if include_all {
            return Err(
                "extra shortlist objectives require include_all=false in the current kernel implementation"
                    .to_string(),
            );
        }
        if score_mode.is_some() {
            return Err(
                "extra shortlist objectives currently require score_mode to be unset".to_string(),
            );
        }
    }
    if shortlist_union_mode.is_some() {
        if !extra_shortlist_enabled {
            return Err(format!(
                "{SHORTLIST_UNION_REQUEST_FIELD} requires {EXTRA_SHORTLIST_REQUEST_FIELD} to be set so the exact union shortlist has multiple heads"
            ));
        }
        if top_k == 0 {
            return Err(format!(
                "{SHORTLIST_UNION_REQUEST_FIELD} requires top_k > 0 so the union shortlist has a bounded head size"
            ));
        }
        if include_all {
            return Err(format!(
                "{SHORTLIST_UNION_REQUEST_FIELD} requires include_all=false in the current kernel implementation"
            ));
        }
        if score_mode.is_some() {
            return Err(format!(
                "{SHORTLIST_UNION_REQUEST_FIELD} currently requires score_mode to be unset"
            ));
        }
    }
    if shortlist_confidence_mode.is_some() {
        if top_k == 0 {
            return Err(format!(
                "{SHORTLIST_CONFIDENCE_REQUEST_FIELD} requires top_k > 0 so the shortlist has a bounded head size"
            ));
        }
        if include_all {
            return Err(format!(
                "{SHORTLIST_CONFIDENCE_REQUEST_FIELD} requires include_all=false in the current kernel implementation"
            ));
        }
        if score_mode.is_some() {
            return Err(format!(
                "{SHORTLIST_CONFIDENCE_REQUEST_FIELD} currently requires score_mode to be unset"
            ));
        }
    }
    if shortlist_slice_mode.is_some() {
        if top_k == 0 {
            return Err(format!(
                "{SHORTLIST_SLICE_MODE_REQUEST_FIELD} requires top_k > 0 so slice robustness can rerank a bounded shortlist"
            ));
        }
        if include_all {
            return Err(format!(
                "{SHORTLIST_SLICE_MODE_REQUEST_FIELD} requires include_all=false in the current kernel implementation"
            ));
        }
    }
    if shortlist_slice_benchmark.is_some() && shortlist_slice_mode.is_none() {
        return Err(format!(
            "{SHORTLIST_SLICE_BENCHMARK_REQUEST_FIELD} requires {SHORTLIST_SLICE_MODE_REQUEST_FIELD} to be set"
        ));
    }
    let export_path = export_all_csv_path.map(str::trim).filter(|s| !s.is_empty());
    let export_enabled = export_path.is_some();
    let row_score_heads_env_enabled = kernel_row_score_heads_enabled();
    let row_score_heads_preselect = row_score_heads_preselect_candidate(
        row_score_heads_env_enabled,
        score_mode,
        include_all,
        export_enabled,
        extra_shortlist_enabled,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        constraints.is_some(),
        eval_range,
        fast_precomputed.is_some(),
        slow_precomputed.is_some(),
        heatmap_bins,
        top_k,
    );
    let device_include_all = include_all || (score_mode.is_some() && !row_score_heads_preselect);
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
    let mut launch_submit_elapsed = Duration::ZERO;
    let mut d2h_copy_elapsed = Duration::ZERO;
    let mut host_decode_elapsed = Duration::ZERO;
    let mut explicit_sync_elapsed = Duration::ZERO;
    let mut host_finalize_elapsed = Duration::ZERO;
    let mut graph_capture_elapsed = Duration::ZERO;
    let mut graph_replay_count = 0u64;

    if export_path.is_some() && include_all {
        return Err(
            "export_all_csv_path requires include_all=false (stream export avoids huge RAM usage)"
                .to_string(),
        );
    }
    if extra_shortlist_enabled && export_path.is_some() {
        return Err(
            "extra shortlist objectives currently require export_all_csv_path to be unset"
                .to_string(),
        );
    }
    if shortlist_confidence_mode.is_some() && export_path.is_some() {
        return Err(format!(
            "{SHORTLIST_CONFIDENCE_REQUEST_FIELD} currently requires export_all_csv_path to be unset"
        ));
    }
    if shortlist_union_mode.is_some() && export_path.is_some() {
        return Err(format!(
            "{SHORTLIST_UNION_REQUEST_FIELD} currently requires export_all_csv_path to be unset"
        ));
    }
    let mut export_writer: Option<BufWriter<File>> = if let Some(path) = export_path {
        let f = File::create(path)
            .map_err(|e| format!("failed to create export file '{path}': {e}"))?;
        let mut w = BufWriter::new(f);
        writeln!(
            w,
            "fast_ma,slow_ma,fast_len,slow_len,pnl,sharpe,max_dd,downside_deviation,downside_frequency,downside_severity,ulcer_index,omega,kappa_tau,martin_ratio,trades,exposure,net_exposure,long_mean_return,short_mean_return,underwater_fraction,max_underwater_duration_bars,closed_trades,winning_trades,losing_trades,gross_profit,gross_loss,win_rate,avg_win,avg_loss,profit_factor,expectancy,active_mean,active_downside_deviation,active_sortino,drawdown_chronicity,downside_share,long_exposure,short_exposure,directional_bias,fee_stress_pnl,score"
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

    let effective_sample_score_context = score_mode_needs_pair_observation_context(score_mode)
        .then_some(EffectiveSampleScoreContext {
            first_valid,
            regime_first_valid,
            eval_start: eval_start_t,
            eval_end: eval_end_t,
            total_bars: n,
            trade_on_next_bar: strategy.trade_on_next_bar,
        });
    let _effective_sample_score_guard =
        effective_sample_score_context.map(install_effective_sample_score_context);
    if let Some(ctx) = effective_sample_score_context {
        record_last_score_observation_context(PairObservationScoreContextResolved {
            first_valid: ctx.first_valid,
            regime_first_valid: ctx.regime_first_valid,
            eval_start: ctx.eval_start,
            eval_end: ctx.eval_end,
            total_bars: ctx.total_bars,
            trade_on_next_bar: ctx.trade_on_next_bar,
        });
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
    if score_mode_needs_pair_observation_context(score_mode) {
        agg = agg.with_pair_score_fn(effective_sample_lcb_10_pair_score);
    } else if let Some(score_fn) = score_mode_score_fn(score_mode) {
        agg = agg.with_score_fn(score_fn);
    }
    if device_include_all {
        if let Some((f_min, f_max, s_min, s_max)) = heatmap_cfg {
            agg = agg.with_heatmap(heatmap_bins, heatmap_bins, f_min, f_max, s_min, s_max);
        }
    }
    let mut processed = 0usize;

    let metrics_count: usize = search_metrics_count_for_optimization_with_contract(
        objective,
        score_mode,
        constraints,
        device_include_all,
        export_enabled,
        fast_precomputed_enabled,
        slow_precomputed_enabled,
        cfg.include_trade_metrics,
        cfg.metric_contract,
    );
    let topk_select = normalize_topk_select_for_device(top_k, device_include_all)?;
    let objective_i = objective_kind_kernel_index(objective);
    let bt_stream = &rt.stream;
    let bt_module = &rt.module;
    let device_launch_caps = rt.launch_caps;
    let compute_returns = bt_module
        .get_function("compute_log_and_simple_returns_f32")
        .map_err(|e| format!("compute_log_and_simple_returns_f32: {e}"))?;
    let compute_returns_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::ComputeReturns, &compute_returns);
    let use_fast_path = kernel_fast_path_enabled(fast_ma.as_str(), slow_ma.as_str())
        && !fast_precomputed_enabled
        && !slow_precomputed_enabled;
    let use_valid_pair_map = kernel_valid_pair_map_enabled();
    let valid_pair_map_max_pairs = kernel_valid_pair_map_max_pairs();
    let load_bt_fn = |name: &str| {
        bt_module
            .get_function(name)
            .map_err(|e| format!("{name}: {e}"))
    };

    let transpose = load_bt_fn("transpose_row_to_tm")?;
    let backtest = load_bt_fn("double_cross_backtest_tm_flex_f32")?;
    let backtest_shadow_sharpe = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_shadow_sharpe_score")
        .ok();
    let backtest_windowed = load_bt_fn("double_cross_backtest_tm_flex_f32_windowed")?;
    let backtest_windowed_m17 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m17")
        .ok();
    let backtest_windowed_m17_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_windowed_m17_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk")
        .ok();
    let backtest_windowed_m17_regime_on_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk_eps0_nocomm")
        .ok();
    let backtest_windowed_m17_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk")
        .ok();
    let backtest_windowed_m17_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_windowed_m22 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m22")
        .ok();
    let backtest_windowed_m22_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_windowed_m22_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk")
        .ok();
    let backtest_windowed_m22_regime_on_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk_eps0_nocomm")
        .ok();
    let backtest_windowed_m22_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk")
        .ok();
    let backtest_windowed_m22_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_m7 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m7")
        .ok();
    let backtest_m17 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17")
        .ok();
    let backtest_m22 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22")
        .ok();
    let backtest_prune_maxdd = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_prune_maxdd")
        .ok();
    let backtest_m17_prune_maxdd = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_prune_maxdd")
        .ok();
    let backtest_m22_prune_maxdd = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_prune_maxdd")
        .ok();
    let backtest_m17_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_off_norisk")
        .ok();
    let backtest_m17_regime_off_norisk_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_nocomm")
        .ok();
    let backtest_m17_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0")
        .ok();
    let backtest_m17_regime_off_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0_nocomm")
        .ok();
    let backtest_m17_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_on_norisk")
        .ok();
    let backtest_m17_regime_on_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_on_norisk_eps0_nocomm")
        .ok();
    let backtest_m17_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_hybrid_norisk")
        .ok();
    let backtest_m17_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m17_regime_hybrid_norisk_eps0_nocomm")
        .ok();
    let backtest_m22_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_off_norisk")
        .ok();
    let backtest_m22_regime_off_norisk_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_nocomm")
        .ok();
    let backtest_m22_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_eps0")
        .ok();
    let backtest_m22_regime_off_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_eps0_nocomm")
        .ok();
    let backtest_m22_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_on_norisk")
        .ok();
    let backtest_m22_regime_on_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_on_norisk_eps0_nocomm")
        .ok();
    let backtest_m22_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_hybrid_norisk")
        .ok();
    let backtest_m22_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_m22_regime_hybrid_norisk_eps0_nocomm")
        .ok();
    let backtest_lockstep = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time")
        .ok();
    let backtest_lockstep_shadow_sharpe = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_shadow_sharpe_score")
        .ok();
    let backtest_lockstep_m7 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m7")
        .ok();
    let backtest_lockstep_m17 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17")
        .ok();
    let backtest_lockstep_m22 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22")
        .ok();
    let backtest_lockstep_m17_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk")
        .ok();
    let backtest_lockstep_m17_regime_off_norisk_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_nocomm",
        )
        .ok();
    let backtest_lockstep_m17_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0")
        .ok();
    let backtest_lockstep_m17_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m17_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_on_norisk")
        .ok();
    let backtest_lockstep_m17_regime_on_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_on_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m17_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_hybrid_norisk")
        .ok();
    let backtest_lockstep_m17_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_hybrid_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m22_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk")
        .ok();
    let backtest_lockstep_m22_regime_off_norisk_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_nocomm",
        )
        .ok();
    let backtest_lockstep_m22_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0")
        .ok();
    let backtest_lockstep_m22_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m22_regime_on_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_on_norisk")
        .ok();
    let backtest_lockstep_m22_regime_on_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_on_norisk_eps0_nocomm",
        )
        .ok();
    let backtest_lockstep_m22_regime_hybrid_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_hybrid_norisk")
        .ok();
    let backtest_lockstep_m22_regime_hybrid_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_hybrid_norisk_eps0_nocomm",
        )
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
    let backtest_lockstep_m10_regime_off_norisk = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk")
        .ok();
    let backtest_lockstep_m10_regime_off_norisk_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_nocomm",
        )
        .ok();
    let backtest_lockstep_m10_regime_off_norisk_eps0 = bt_module
        .get_function("double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_eps0")
        .ok();
    let backtest_lockstep_m10_regime_off_norisk_eps0_nocomm = bt_module
        .get_function(
            "double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_eps0_nocomm",
        )
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
    let fill_f32 = load_bt_fn("fill_f32")?;
    let fill_i32 = load_bt_fn("fill_i32")?;
    let heatmap_update = load_bt_fn("heatmap_update_scores_tm_f32")?;
    let select_topk_pairs_legacy = load_bt_fn("select_topk_from_pairs_tm_f32")?;
    let select_topk_candidates_legacy = load_bt_fn("select_topk_from_candidates_tm_f32")?;
    let select_topk_pairs_v2 = bt_module
        .get_function("select_topk_from_pairs_tm_f32_v2")
        .ok();
    let select_topk_candidates_v2 = bt_module
        .get_function("select_topk_from_candidates_tm_f32_v2")
        .ok();
    let _score_metric_rows = bt_module.get_function("score_metric_rows_f32").ok();

    let select_topk_scored_pairs = bt_module
        .get_function("select_topk_scored_pairs_tm_f32")
        .ok();
    let select_topk_scored_pairs_v2 = bt_module
        .get_function("select_topk_scored_pairs_tm_f32_v2")
        .ok();

    let select_topk_scored_candidates = bt_module
        .get_function("select_topk_scored_candidates_tm_f32")
        .ok();
    let select_topk_scored_rows = bt_module.get_function("select_topk_scored_rows_f32").ok();
    let select_topk_scored_rows_candidates = bt_module
        .get_function("select_topk_scored_rows_from_candidates_f32")
        .ok();
    let gather_topk_scored_rows = bt_module.get_function("gather_topk_scored_rows_f32").ok();
    let select_v2_supported = !kernel_force_legacy_topk_enabled()
        && select_topk_pairs_v2.is_some()
        && select_topk_candidates_v2.is_some();
    let gather_topk = load_bt_fn("gather_topk_pairs_tm_f32")?;
    let pack_metric_rows = load_bt_fn("pack_metric_rows_f32")?;
    let gather_topk_scored_pairs = bt_module
        .get_function("gather_topk_scored_pairs_tm_f32")
        .ok();
    let _gather_topk_scored_scalar_pairs = bt_module
        .get_function("gather_topk_scored_scalar_pairs_tm_f32")
        .ok();
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
    let fill_i32_launch_caps = rt.launch_caps_for_kernel(CachedKernelKey::FillI32, &fill_i32);
    let transpose_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::TransposeRowToTm, &transpose);
    let backtest_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktest, &backtest);
    let _backtest_shadow_launch_caps = backtest_shadow_sharpe
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktest, f));
    let backtest_m7_launch_caps = backtest_m7
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM7, f));
    let backtest_m17_launch_caps = backtest_m17
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM17, f));
    let backtest_m22_launch_caps = backtest_m22
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM22, f));
    let backtest_prune_maxdd_launch_caps = backtest_prune_maxdd
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestPruneMaxdd, f));
    let backtest_m17_prune_maxdd_launch_caps = backtest_m17_prune_maxdd
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM17PruneMaxdd, f));
    let backtest_m22_prune_maxdd_launch_caps = backtest_m22_prune_maxdd
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestM22PruneMaxdd, f));
    let backtest_lockstep_launch_caps = backtest_lockstep
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstep, f));
    let _backtest_lockstep_shadow_launch_caps = backtest_lockstep_shadow_sharpe
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstep, f));
    let backtest_lockstep_m7_launch_caps = backtest_lockstep_m7
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstepM7, f));
    let backtest_lockstep_m17_launch_caps = backtest_lockstep_m17
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstepM17, f));
    let backtest_lockstep_m22_launch_caps = backtest_lockstep_m22
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestLockstepM22, f));
    let backtest_fast_launch_caps = backtest_fast_warp
        .as_ref()
        .map(|f| rt.launch_caps_for_kernel(CachedKernelKey::DoubleCrossBacktestFastWarpDedup, f));
    let heatmap_launch_caps =
        rt.launch_caps_for_kernel(CachedKernelKey::HeatmapUpdateScores, &heatmap_update);

    let ma_source_key = normalize_market_source_key(ma_source);
    let ma_is_close = ma_source_key == "close";
    let alloc_ma_buf = needs_ma_prices && !ma_is_close;
    let (resident_market, staged_market, staging_stream, ma, scratch, valid_pair_maps) = (
        &mut rt.resident_market,
        &mut rt.staged_market,
        &rt.staging_stream,
        &mut rt.ma,
        &mut rt.scratch,
        &mut rt.valid_pair_maps,
    );
    let dataset_sig = candle_dataset_signature(candles);
    let _ = promote_matching_staged_market_series(
        bt_stream,
        staging_stream,
        resident_market,
        staged_market,
        dataset_sig,
    )
    .map_err(|e| format!("promote_matching_staged_market_series: {e}"))?;
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
    )
    .map_err(|e| format!("ensure_resident_market_series: {e}"))?;
    let resident = resident_market
        .as_ref()
        .ok_or_else(|| "resident market cache missing after upload".to_string())?;
    emit_progress_and_maybe_stage_campaign_prefetch(
        cfg.device_id,
        resident_market,
        staging_stream,
        staged_market,
        progress,
        processed,
        total_pairs,
        "gpu-kernel",
    );
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
        let _gpu_indicator_prep_scope = crate::nvtx::scope("kernel", "gpu_indicator_prep");
        let t0 = Instant::now();
        ma.ensure_sma_prefix_f64(d_ma_prices, n, first_valid)
            .map_err(|e| format!("ensure_sma_prefix_f64: {e}"))?;
        gpu_indicator_prep_elapsed = gpu_indicator_prep_elapsed.saturating_add(t0.elapsed());
    }
    if fast_ma == "vwma" || slow_ma == "vwma" {
        let _gpu_indicator_prep_scope = crate::nvtx::scope("kernel", "gpu_indicator_prep");
        let t0 = Instant::now();
        ma.ensure_vwma_prefix_pv_vol_f64(
            d_ma_prices,
            d_volume
                .as_ref()
                .ok_or_else(|| "vwma requires volume".to_string())?,
            n,
            first_valid,
        )
        .map_err(|e| format!("ensure_vwma_prefix_pv_vol_f64: {e}"))?;
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

    ensure_log_and_simple_returns(
        bt_stream,
        scratch,
        d_close,
        n,
        dataset_sig,
        &compute_returns,
        device_launch_caps,
        compute_returns_launch_caps,
    )
    .map_err(|e| format!("ensure_log_and_simple_returns: {e}"))?;
    let d_lr_ptr = scratch.d_lr.as_ref().unwrap().as_device_ptr().as_raw();
    let d_ret1_ptr = scratch.d_ret1.as_ref().unwrap().as_device_ptr().as_raw();

    let use_windowed_eval = eval_range.is_some();
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
        device_include_all,
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
    KernelScratch::ensure_len_f32(
        &mut scratch.d_tile_scores,
        pf_tile.saturating_mul(ps_tile).max(1),
        bt_stream,
    )?;

    let constraints_on_device = constraints.is_some() && !device_include_all && select_v2_supported;
    let host_filter_mode = constraints.is_some() && !device_include_all && !constraints_on_device;
    let compact_valid_pair_output = should_use_compact_valid_pair_output(
        device_include_all,
        export_enabled,
        host_filter_mode,
        metrics_count,
    );
    let row_select_v2_supported = !kernel_force_legacy_topk_enabled()
        && select_topk_rows_v2.is_some()
        && select_topk_rows_candidates_v2.is_some();
    let global_topk_mode = kernel_global_topk_enabled()
        && !device_include_all
        && !export_enabled
        && !host_filter_mode
        && topk_select > 0
        && select_topk_rows_legacy.is_some()
        && select_topk_rows_candidates_legacy.is_some()
        && gather_topk_rows.is_some();

    let score_first_topk_supported = select_topk_scored_pairs.is_some()
        && select_topk_scored_candidates.is_some()
        && select_topk_scored_rows.is_some()
        && select_topk_scored_rows_candidates.is_some()
        && gather_topk_scored_rows.is_some()
        && gather_topk_scored_pairs.is_some();
    let score_first_env_enabled = kernel_score_first_topk_enabled();
    let score_first_blockers = score_first_topk_blockers(
        score_first_env_enabled,
        global_topk_mode,
        score_first_topk_supported,
        score_mode,
        extra_shortlist_enabled,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        metrics_count,
        constraints.is_some(),
        eval_range,
        fast_precomputed.is_some(),
        slow_precomputed.is_some(),
        heatmap_bins,
    );
    let score_first_topk_mode = should_use_score_first_topk_mode(
        global_topk_mode,
        score_first_topk_supported,
        score_mode,
        extra_shortlist_enabled,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        metrics_count,
        constraints.is_some(),
        eval_range,
        fast_precomputed.is_some(),
        slow_precomputed.is_some(),
        heatmap_bins,
    );
    let row_score_heads_objective_i = row_score_mode_objective_index(score_mode);
    let row_score_heads_blocker_list = row_score_heads_blockers(
        row_score_heads_env_enabled,
        global_topk_mode,
        score_mode,
        metrics_count,
        include_all,
        export_enabled,
        extra_shortlist_enabled,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        constraints.is_some(),
        eval_range,
        fast_precomputed.is_some(),
        slow_precomputed.is_some(),
        heatmap_bins,
        top_k,
    );
    let row_score_heads_mode = should_use_row_score_heads_mode(
        global_topk_mode,
        score_mode,
        metrics_count,
        include_all,
        export_enabled,
        extra_shortlist_enabled,
        shortlist_confidence_mode,
        shortlist_union_mode,
        shortlist_slice_mode,
        constraints.is_some(),
        eval_range,
        fast_precomputed.is_some(),
        slow_precomputed.is_some(),
        heatmap_bins,
        top_k,
    );
    let primary_objective_i = if row_score_heads_mode {
        row_score_heads_objective_i.unwrap_or(objective_i)
    } else {
        objective_i
    };
    let exact_pruning_symbols_available = backtest_prune_maxdd.is_some();
    let exact_pruning_env_enabled = kernel_exact_pruning_enabled();
    let exact_pruning_max_log_dd = exact_pruning_max_log_dd_threshold(constraints);
    let exact_pruning_blocker_list = exact_pruning_blockers(
        exact_pruning_env_enabled,
        global_topk_mode,
        exact_pruning_symbols_available,
        exact_pruning_max_log_dd,
        constraints_on_device,
        device_include_all,
        export_enabled,
        host_filter_mode,
        eval_range,
        fast_precomputed.is_some(),
        slow_precomputed.is_some(),
        heatmap_bins,
    );
    let exact_pruning_mode = should_use_exact_pruning_mode(
        global_topk_mode,
        exact_pruning_symbols_available,
        exact_pruning_max_log_dd,
        constraints_on_device,
        device_include_all,
        export_enabled,
        host_filter_mode,
        eval_range,
        fast_precomputed.is_some(),
        slow_precomputed.is_some(),
        heatmap_bins,
    );
    record_last_route_diagnostics(build_gpu_kernel_route_diagnostics(
        device_include_all,
        export_enabled,
        host_filter_mode,
        compact_valid_pair_output,
        global_topk_mode,
        score_first_env_enabled,
        score_first_topk_supported,
        score_first_topk_mode,
        row_score_heads_env_enabled,
        row_score_heads_mode,
        row_score_heads_objective_i,
        exact_pruning_env_enabled,
        exact_pruning_symbols_available,
        exact_pruning_mode,
        exact_pruning_max_log_dd,
        metrics_count,
        tile_metrics_stride,
        &score_first_blockers,
        &row_score_heads_blocker_list,
        &exact_pruning_blocker_list,
    ));
    if row_score_heads_preselect && !row_score_heads_mode {
        let blockers = if row_score_heads_blocker_list.is_empty() {
            "none".to_string()
        } else {
            row_score_heads_blocker_list.join(",")
        };
        return Err(format!(
            "row-score heads were preselected but the GPU row-score route is unavailable ({blockers})"
        ));
    }
    let exact_pruning_counters = if exact_pruning_mode {
        Some(
            DeviceBuffer::from_slice(&[0_u64, 0_u64])
                .map_err(|e| format!("allocate exact-pruning counters: {e}"))?,
        )
    } else {
        None
    };
    let exact_pruning_counters_ptr = exact_pruning_counters
        .as_ref()
        .map(|counters| counters.as_device_ptr().as_raw())
        .unwrap_or(0);
    let global_constraints_on_device =
        constraints.is_some() && global_topk_mode && row_select_v2_supported;
    if extra_shortlist_enabled && !global_topk_mode {
        return Err(
            "extra shortlist objectives currently require the exact global top-k kernel path (disable conflicting host filtering / export overrides)"
                .to_string(),
        );
    }
    if extra_shortlist_enabled && host_filter_mode {
        return Err(
            "extra shortlist objectives currently require device-side filtering; this request forced host_filter_mode"
                .to_string(),
        );
    }
    if shortlist_confidence_mode.is_some() && host_filter_mode {
        return Err(format!(
            "{SHORTLIST_CONFIDENCE_REQUEST_FIELD} currently requires device-side filtering; this request forced host_filter_mode"
        ));
    }
    if shortlist_union_mode.is_some() && host_filter_mode {
        return Err(format!(
            "{SHORTLIST_UNION_REQUEST_FIELD} currently requires device-side filtering; this request forced host_filter_mode"
        ));
    }
    if shortlist_confidence_mode.is_some() && !global_topk_mode {
        return Err(format!(
            "{SHORTLIST_CONFIDENCE_REQUEST_FIELD} currently requires the exact global top-k kernel path (disable include_all / score_mode / export / host filtering overrides)"
        ));
    }
    if shortlist_union_mode.is_some() && !global_topk_mode {
        return Err(format!(
            "{SHORTLIST_UNION_REQUEST_FIELD} currently requires the exact global top-k kernel path (disable include_all / score_mode / export / host filtering overrides)"
        ));
    }
    if shortlist_confidence_mode.is_some()
        && !can_decode_current_gpu_product_metrics_without_recompute(metrics_count)
    {
        return Err(format!(
            "{SHORTLIST_CONFIDENCE_REQUEST_FIELD} currently requires the promoted single-pass product metric rows"
        ));
    }
    let constraint_args = build_device_constraint_args(constraints, metrics_count);
    let mut host_p_fast: Vec<i32> = Vec::with_capacity(pf_tile_tm.max(1));
    let mut host_p_slow: Vec<i32> = Vec::with_capacity(ps_tile_tm.max(1));
    let mut tile_host_buf: Vec<f32> = if device_include_all || export_enabled || host_filter_mode {
        alloc_uninit_copy(tile_out_cap)
    } else {
        Vec::new()
    };
    let mut fast_precomputed_host_tile: Vec<f32> = Vec::new();
    let mut slow_precomputed_host_tile: Vec<f32> = Vec::new();

    let mut global_fast_rows: Option<DeviceBuffer<i32>> = None;
    let mut global_slow_rows: Option<DeviceBuffer<i32>> = None;
    let mut global_score_rows: Option<DeviceBuffer<f32>> = None;
    let mut global_metrics_rows: Option<DeviceBuffer<f32>> = None;
    let mut global_rows_len: usize = 0;
    let global_rows_cap: usize = if global_topk_mode {
        topk_select.saturating_mul(2).max(topk_select)
    } else {
        0
    };
    let mut extra_shortlist_heads: Vec<ExtraShortlistHeadState> = extra_shortlist_objectives
        .iter()
        .copied()
        .map(|head_objective| ExtraShortlistHeadState {
            objective: head_objective,
            objective_i: objective_kind_kernel_index(head_objective),
            global_fast_rows: None,
            global_slow_rows: None,
            global_metrics_rows: None,
            global_rows_len: 0,
        })
        .collect();

    let mut top_packed_host = HostTransferScratchF32::default();

    if !device_include_all {
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
        top_packed_host.ensure_capacity(topk_select * metrics_count.saturating_add(2))?;
        if global_topk_mode {
            KernelScratch::ensure_len_i32(&mut global_fast_rows, global_rows_cap, bt_stream)?;
            KernelScratch::ensure_len_i32(&mut global_slow_rows, global_rows_cap, bt_stream)?;
            if score_first_topk_mode {
                KernelScratch::ensure_len_f32(&mut global_score_rows, global_rows_cap, bt_stream)?;
            } else {
                KernelScratch::ensure_len_f32(
                    &mut global_metrics_rows,
                    global_rows_cap.saturating_mul(metrics_count),
                    bt_stream,
                )?;
                for head in extra_shortlist_heads.iter_mut() {
                    KernelScratch::ensure_len_i32(
                        &mut head.global_fast_rows,
                        global_rows_cap,
                        bt_stream,
                    )?;
                    KernelScratch::ensure_len_i32(
                        &mut head.global_slow_rows,
                        global_rows_cap,
                        bt_stream,
                    )?;
                    KernelScratch::ensure_len_f32(
                        &mut head.global_metrics_rows,
                        global_rows_cap.saturating_mul(metrics_count),
                        bt_stream,
                    )?;
                }
            }
        }

        if heatmap_cfg.is_some() {
            let heatmap_len = heatmap_bins.saturating_mul(heatmap_bins);
            if heatmap_len > 0 {
                KernelScratch::ensure_len_f32(&mut scratch.d_heatmap, heatmap_len, bt_stream)?;
                KernelScratch::ensure_len_f32(&mut scratch.d_heatmap_sum, heatmap_len, bt_stream)?;
                KernelScratch::ensure_len_i32(
                    &mut scratch.d_heatmap_count,
                    heatmap_len,
                    bt_stream,
                )?;
                unsafe {
                    let mut out_ptr = scratch.d_heatmap.as_ref().unwrap().as_device_ptr().as_raw();
                    let mut n_i = heatmap_len as i32;
                    let mut v = f32::NEG_INFINITY;
                    let fill_launch =
                        choose_launch_shape_1d(heatmap_len, device_launch_caps, fill_launch_caps);
                    let block_x: u32 = fill_launch.block_x;
                    let grid_x: u32 = fill_launch.grid_x;
                    let args: &mut [*mut c_void] = &mut [
                        &mut out_ptr as *mut _ as *mut c_void,
                        &mut n_i as *mut _ as *mut c_void,
                        &mut v as *mut _ as *mut c_void,
                    ];
                    launch_kernel_timed(
                        bt_stream,
                        &fill_f32,
                        (grid_x, 1, 1),
                        (block_x, 1, 1),
                        0,
                        args,
                        &mut launch_submit_elapsed,
                    )?;
                }
                unsafe {
                    let mut out_ptr = scratch
                        .d_heatmap_sum
                        .as_ref()
                        .unwrap()
                        .as_device_ptr()
                        .as_raw();
                    let mut n_i = heatmap_len as i32;
                    let mut v = 0.0f32;
                    let fill_launch =
                        choose_launch_shape_1d(heatmap_len, device_launch_caps, fill_launch_caps);
                    let block_x: u32 = fill_launch.block_x;
                    let grid_x: u32 = fill_launch.grid_x;
                    let args: &mut [*mut c_void] = &mut [
                        &mut out_ptr as *mut _ as *mut c_void,
                        &mut n_i as *mut _ as *mut c_void,
                        &mut v as *mut _ as *mut c_void,
                    ];
                    launch_kernel_timed(
                        bt_stream,
                        &fill_f32,
                        (grid_x, 1, 1),
                        (block_x, 1, 1),
                        0,
                        args,
                        &mut launch_submit_elapsed,
                    )?;
                }
                unsafe {
                    let mut out_ptr = scratch
                        .d_heatmap_count
                        .as_ref()
                        .unwrap()
                        .as_device_ptr()
                        .as_raw();
                    let mut n_i = heatmap_len as i32;
                    let mut v = 0i32;
                    let fill_launch = choose_launch_shape_1d(
                        heatmap_len,
                        device_launch_caps,
                        fill_i32_launch_caps,
                    );
                    let block_x: u32 = fill_launch.block_x;
                    let grid_x: u32 = fill_launch.grid_x;
                    let args: &mut [*mut c_void] = &mut [
                        &mut out_ptr as *mut _ as *mut c_void,
                        &mut n_i as *mut _ as *mut c_void,
                        &mut v as *mut _ as *mut c_void,
                    ];
                    launch_kernel_timed(
                        bt_stream,
                        &fill_i32,
                        (grid_x, 1, 1),
                        (block_x, 1, 1),
                        0,
                        args,
                        &mut launch_submit_elapsed,
                    )?;
                }
            }
        }
    }

    let strategy_kernel = kernel_strategy_params(strategy);
    let flags = strategy_kernel.flags;
    let commission = strategy_kernel.commission;
    let eps_rel = strategy_kernel.eps_rel;
    let hurdle_tau = cfg.hurdle_tau as f32;
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

    let use_cuda_graphs = !pipeline_profile_enabled
        && kernel_cuda_graphs_enabled()
        && !device_include_all
        && !host_filter_mode
        && heatmap_cfg.is_none();
    let mut tile_graph_exec: Option<CudaGraphExecHandle> = None;
    let mut tile_graph_shape: Option<(usize, usize, usize, usize, bool, bool)> = None;
    let mut graph_capture_disabled = false;
    let _backtest_core_scope = crate::nvtx::scope("kernel", "backtest_core");

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

        let h2d_t0 = Instant::now();
        {
            let _h2d_upload_scope = crate::nvtx::scope("kernel", "h2d_upload");
            scratch
                .d_fast_p
                .as_mut()
                .unwrap()
                .copy_from(&host_p_fast)
                .map_err(|e| e.to_string())?;
        }
        h2d_upload_elapsed = h2d_upload_elapsed.saturating_add(h2d_t0.elapsed());
        if let Some(precomputed_fast) = fast_precomputed {
            pack_precomputed_rows_tm_with_padding(
                precomputed_fast,
                f_start,
                pf,
                fast_tm_rows,
                &mut fast_precomputed_host_tile,
            )?;
            let _h2d_upload_scope = crate::nvtx::scope("kernel", "h2d_upload");
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
            let _gpu_indicator_prep_scope = crate::nvtx::scope("kernel", "gpu_indicator_prep");
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
                fill_device_buffer_f32_timed(
                    bt_stream,
                    &fill_f32,
                    device_launch_caps,
                    fill_launch_caps,
                    scratch.d_fast_ma.as_ref().unwrap(),
                    pf.saturating_mul(n),
                    f32::NAN,
                    &mut launch_submit_elapsed,
                )?;
                fill_device_buffer_f32_timed(
                    bt_stream,
                    &fill_f32,
                    device_launch_caps,
                    fill_launch_caps,
                    scratch.d_fast_ma_tm.as_ref().unwrap(),
                    fast_tm_rows.saturating_mul(n),
                    f32::NAN,
                    &mut launch_submit_elapsed,
                )?;
            }
            let _gpu_indicator_prep_scope = crate::nvtx::scope("kernel", "gpu_indicator_prep");
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
                launch_kernel_timed(
                    bt_stream,
                    &transpose,
                    (grid_x, 1, 1),
                    (block_x, 1, 1),
                    0,
                    args,
                    &mut launch_submit_elapsed,
                )?;
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

            let h2d_t0 = Instant::now();
            {
                let _h2d_upload_scope = crate::nvtx::scope("kernel", "h2d_upload");
                scratch
                    .d_slow_p
                    .as_mut()
                    .unwrap()
                    .copy_from(&host_p_slow)
                    .map_err(|e| e.to_string())?;
            }
            h2d_upload_elapsed = h2d_upload_elapsed.saturating_add(h2d_t0.elapsed());
            if let Some(precomputed_slow) = slow_precomputed {
                pack_precomputed_rows_tm_with_padding(
                    precomputed_slow,
                    s_start,
                    ps,
                    slow_tm_rows,
                    &mut slow_precomputed_host_tile,
                )?;
                let _h2d_upload_scope = crate::nvtx::scope("kernel", "h2d_upload");
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
                let _gpu_indicator_prep_scope = crate::nvtx::scope("kernel", "gpu_indicator_prep");
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
                    fill_device_buffer_f32_timed(
                        bt_stream,
                        &fill_f32,
                        device_launch_caps,
                        fill_launch_caps,
                        scratch.d_slow_ma.as_ref().unwrap(),
                        ps.saturating_mul(n),
                        f32::NAN,
                        &mut launch_submit_elapsed,
                    )?;
                    fill_device_buffer_f32_timed(
                        bt_stream,
                        &fill_f32,
                        device_launch_caps,
                        fill_launch_caps,
                        scratch.d_slow_ma_tm.as_ref().unwrap(),
                        slow_tm_rows.saturating_mul(n),
                        f32::NAN,
                        &mut launch_submit_elapsed,
                    )?;
                }
                let _gpu_indicator_prep_scope = crate::nvtx::scope("kernel", "gpu_indicator_prep");
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
                    launch_kernel_timed(
                        bt_stream,
                        &transpose,
                        (grid_x, 1, 1),
                        (block_x, 1, 1),
                        0,
                        args,
                        &mut launch_submit_elapsed,
                    )?;
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
            let mut pair_fast_local_rows_ptr: u64 = 0;
            let mut pair_slow_local_rows_ptr: u64 = 0;
            if use_valid_pair_map && pairs <= valid_pair_map_max_pairs {
                let cached_valid_map = ensure_cached_valid_pair_map(
                    valid_pair_maps,
                    &fast_periods[f_start..f_start + pf],
                    &slow_periods[s_start..s_start + ps],
                )?;
                launch_pairs = cached_valid_map.map(|map| map.len).unwrap_or(0);
                if launch_pairs > 0 {
                    pair_index_map_ptr = cached_valid_map
                        .map(|map| map.index_ptr)
                        .unwrap_or_default();
                    pair_fast_rows_ptr = cached_valid_map
                        .map(|map| map.fast_period_ptr)
                        .unwrap_or_default();
                    pair_slow_rows_ptr = cached_valid_map
                        .map(|map| map.slow_period_ptr)
                        .unwrap_or_default();

                    if use_windowed_eval {
                        pair_fast_local_rows_ptr = cached_valid_map
                            .map(|map| map.fast_local_ptr)
                            .unwrap_or_default();
                        pair_slow_local_rows_ptr = cached_valid_map
                            .map(|map| map.slow_local_ptr)
                            .unwrap_or_default();
                    }
                }

                if !compact_valid_pair_output {
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
                        launch_kernel_timed(
                            bt_stream,
                            &fill_f32,
                            (grid_x, 1, 1),
                            (block_x, 1, 1),
                            0,
                            args,
                            &mut launch_submit_elapsed,
                        )?;
                    }
                }
            }

            if let Some(t0) = tile_total_t0 {
                tile_prepare_elapsed = t0.elapsed();
            }

            let mut launch_score_stage = || -> Result<(), String> {
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
                        let mut hurdle_tau_f = hurdle_tau;
                        let mut stop_log = stop_log;
                        let mut take_log = take_log;
                        let mut flags_u = flags;
                        let mut m_i = metrics_count as i32;
                        let mut metrics_stride_i = tile_metrics_stride as i32;
                        let mut pair_map_ptr = pair_index_map_ptr;
                        let mut pair_fast_rows = pair_fast_rows_ptr;
                        let mut pair_slow_rows = pair_slow_rows_ptr;
                        let mut pair_fast_local_rows = pair_fast_local_rows_ptr;
                        let mut pair_slow_local_rows = pair_slow_local_rows_ptr;
                        let mut pair_eval_start_rows = 0u64;
                        let mut pair_eval_end_rows = 0u64;
                        let mut pair_count_i = launch_pairs as i32;
                        let mut compact_output_i = if compact_valid_pair_output {
                            1i32
                        } else {
                            0i32
                        };
                        let mut out_ptr = scratch.d_tile.as_ref().unwrap().as_device_ptr().as_raw();
                        let mut eval_start_i = eval_start_t as i32;
                        let mut eval_end_i = eval_end_t as i32;
                        let no_risk_mode = (flags_u & (STRAT_STOP_LOSS | STRAT_TAKE_PROFIT)) == 0;
                        let zero_commission_mode = commission <= 0.0f32;
                        let eps_zero_mode = eps_rel <= 0.0f32;
                        let fixed_metrics_contract =
                            fixed_metrics_contract_rows(metrics_count, tile_metrics_stride);
                        let prefer_full_metric_specialization = match fixed_metrics_contract {
                            Some(METRICS_COUNT) => backtest_m17.is_some(),
                            Some(TRADE_METRICS_COUNT) => backtest_m22.is_some(),
                            _ => false,
                        };
                        let prefer_full_metric_lockstep_specialization =
                            match fixed_metrics_contract {
                                Some(METRICS_COUNT) => backtest_lockstep_m17.is_some(),
                                Some(TRADE_METRICS_COUNT) => backtest_lockstep_m22.is_some(),
                                _ => false,
                            };

                        let use_lockstep_time = !exact_pruning_mode
                            && !use_windowed_eval
                            && kernel_should_use_lockstep_time(
                                launch_pairs,
                                n,
                                max_start_t,
                                backtest_lockstep.is_some(),
                            );
                        let use_m7 = !exact_pruning_mode
                            && !use_windowed_eval
                            && !prefer_full_metric_specialization
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
                            && !prefer_full_metric_lockstep_specialization
                            && kernel_should_use_m7_specialize(
                                launch_pairs,
                                n,
                                max_start_t,
                                false,
                                metrics_count,
                                backtest_lockstep_m7.is_some(),
                            );
                        let use_lockstep_m10_product = use_lockstep_m7
                            && fixed_metrics_contract == Some(PRODUCT_REDUCED_METRICS_COUNT);
                        let use_fast_warp_dedup = !exact_pruning_mode
                            && !use_windowed_eval
                            && !use_lockstep_time
                            && fast_warp_dedup_enabled
                            && launch_pairs >= fast_warp_dedup_min_pairs
                            && backtest_fast_warp.is_some();
                        let fixed_contract_dispatch = choose_fixed_contract_dispatch_kind(
                            fixed_metrics_contract,
                            use_windowed_eval,
                            use_lockstep_time,
                            use_fast_warp_dedup,
                            use_m7,
                            use_lockstep_m7,
                            backtest_windowed_m17.is_some(),
                            backtest_windowed_m22.is_some(),
                            backtest_m17.is_some(),
                            backtest_m22.is_some(),
                            backtest_lockstep_m17.is_some(),
                            backtest_lockstep_m22.is_some(),
                        );
                        let use_windowed_m17 =
                            fixed_contract_dispatch == FixedContractDispatchKind::WindowedM17;
                        let use_windowed_m22 =
                            fixed_contract_dispatch == FixedContractDispatchKind::WindowedM22;
                        let use_lockstep_m17 =
                            fixed_contract_dispatch == FixedContractDispatchKind::LockstepM17;
                        let use_lockstep_m22 =
                            fixed_contract_dispatch == FixedContractDispatchKind::LockstepM22;
                        let use_m17 = fixed_contract_dispatch == FixedContractDispatchKind::M17;
                        let use_m22 = fixed_contract_dispatch == FixedContractDispatchKind::M22;
                        let launch_caps = if exact_pruning_mode {
                            if use_m17 {
                                backtest_m17_prune_maxdd_launch_caps
                                    .or(backtest_prune_maxdd_launch_caps)
                                    .unwrap_or(backtest_launch_caps)
                            } else if use_m22 {
                                backtest_m22_prune_maxdd_launch_caps
                                    .or(backtest_prune_maxdd_launch_caps)
                                    .unwrap_or(backtest_launch_caps)
                            } else {
                                backtest_prune_maxdd_launch_caps.unwrap_or(backtest_launch_caps)
                            }
                        } else if use_windowed_m17 || use_windowed_m22 || use_windowed_eval {
                            backtest_launch_caps
                        } else if use_lockstep_m7 {
                            backtest_lockstep_m7_launch_caps
                                .or(backtest_lockstep_launch_caps)
                                .unwrap_or(backtest_launch_caps)
                        } else if use_lockstep_m17 {
                            backtest_lockstep_m17_launch_caps
                                .or(backtest_lockstep_launch_caps)
                                .unwrap_or(backtest_launch_caps)
                        } else if use_lockstep_m22 {
                            backtest_lockstep_m22_launch_caps
                                .or(backtest_lockstep_launch_caps)
                                .unwrap_or(backtest_launch_caps)
                        } else if use_lockstep_time {
                            backtest_lockstep_launch_caps.unwrap_or(backtest_launch_caps)
                        } else if use_fast_warp_dedup {
                            backtest_fast_launch_caps.unwrap_or(backtest_launch_caps)
                        } else if use_m17 {
                            backtest_m17_launch_caps.unwrap_or(backtest_launch_caps)
                        } else if use_m22 {
                            backtest_m22_launch_caps.unwrap_or(backtest_launch_caps)
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
                        let dispatch_label = if exact_pruning_mode {
                            if use_m17 {
                                "m17_prune_maxdd"
                            } else if use_m22 {
                                "m22_prune_maxdd"
                            } else {
                                "baseline_prune_maxdd"
                            }
                        } else if use_windowed_eval {
                            if use_windowed_m17 {
                                "windowed_m17"
                            } else if use_windowed_m22 {
                                "windowed_m22"
                            } else {
                                "windowed"
                            }
                        } else if use_lockstep_m7 {
                            "lockstep_m7"
                        } else if use_lockstep_m17 {
                            "lockstep_m17"
                        } else if use_lockstep_m22 {
                            "lockstep_m22"
                        } else if use_lockstep_time {
                            "lockstep_time"
                        } else if use_fast_warp_dedup {
                            "fast_warp_dedup"
                        } else if use_m17 {
                            "m17"
                        } else if use_m22 {
                            "m22"
                        } else if use_m7 {
                            "m7"
                        } else {
                            "baseline"
                        };
                        if kernel_log_dispatch_enabled() {
                            eprintln!(
                                "VECTORBT_KERNEL_DISPATCH: pair={}/{} launch_pairs={} fast_path={} multi_tile={} dispatch={} use_m7={} use_m17={} use_m22={} use_lockstep_time={} use_lockstep_m7={} use_lockstep_m17={} use_lockstep_m22={} use_lockstep_m10_product={} use_fast_warp={} metrics_count={} tile_metrics_stride={} fixed_metrics_contract={:?} decode_without_recompute={}",
                                fast_ma,
                                slow_ma,
                                launch_pairs,
                                use_fast_path,
                                is_multi_tile_workload,
                                dispatch_label,
                                use_m7,
                                use_m17,
                                use_m22,
                                use_lockstep_time,
                                use_lockstep_m7,
                                use_lockstep_m17,
                                use_lockstep_m22,
                                use_lockstep_m10_product,
                                use_fast_warp_dedup,
                                metrics_count,
                                tile_metrics_stride,
                                fixed_metrics_contract,
                                can_decode_current_gpu_product_metrics_without_recompute(
                                    metrics_count
                                )
                            );
                        }
                        let fixed_contract_flavor = choose_fixed_contract_kernel_flavor(
                            no_risk_mode,
                            use_regime_i != 0,
                            use_regime_hybrid_i != 0,
                            zero_commission_mode,
                            eps_zero_mode,
                        );
                        let backtest_fn = if exact_pruning_mode {
                            if use_m17 {
                                backtest_m17_prune_maxdd
                                    .as_ref()
                                    .or(backtest_prune_maxdd.as_ref())
                                    .expect("checked exact-pruning m17 kernel availability")
                            } else if use_m22 {
                                backtest_m22_prune_maxdd
                                    .as_ref()
                                    .or(backtest_prune_maxdd.as_ref())
                                    .expect("checked exact-pruning m22 kernel availability")
                            } else {
                                backtest_prune_maxdd
                                    .as_ref()
                                    .expect("checked exact-pruning kernel availability")
                            }
                        } else if use_windowed_eval {
                            if use_windowed_m17 {
                                match fixed_contract_flavor {
                                    FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm => {
                                        backtest_windowed_m17_regime_off_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeOnNoRisk => {
                                        backtest_windowed_m17_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm => {
                                        backtest_windowed_m17_regime_on_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m17_regime_on_norisk.as_ref())
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeHybridNoRisk => {
                                        backtest_windowed_m17_regime_hybrid_norisk
                                            .as_ref()
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm => {
                                        backtest_windowed_m17_regime_hybrid_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m17_regime_hybrid_norisk.as_ref())
                                            .or(backtest_windowed_m17.as_ref())
                                            .expect("checked windowed m17 kernel availability")
                                    }
                                    _ => backtest_windowed_m17
                                        .as_ref()
                                        .expect("checked windowed m17 kernel availability"),
                                }
                            } else if use_windowed_m22 {
                                match fixed_contract_flavor {
                                    FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm => {
                                        backtest_windowed_m22_regime_off_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeOnNoRisk => {
                                        backtest_windowed_m22_regime_on_norisk
                                            .as_ref()
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm => {
                                        backtest_windowed_m22_regime_on_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m22_regime_on_norisk.as_ref())
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeHybridNoRisk => {
                                        backtest_windowed_m22_regime_hybrid_norisk
                                            .as_ref()
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm => {
                                        backtest_windowed_m22_regime_hybrid_norisk_eps0_nocomm
                                            .as_ref()
                                            .or(backtest_windowed_m22_regime_hybrid_norisk.as_ref())
                                            .or(backtest_windowed_m22.as_ref())
                                            .expect("checked windowed m22 kernel availability")
                                    }
                                    _ => backtest_windowed_m22
                                        .as_ref()
                                        .expect("checked windowed m22 kernel availability"),
                                }
                            } else {
                                &backtest_windowed
                            }
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
                                        if use_lockstep_m10_product {
                                            if zero_commission_mode {
                                                backtest_lockstep_m10_regime_off_norisk_eps0_nocomm
                                                    .as_ref()
                                                    .or(backtest_lockstep_m10_regime_off_norisk_eps0
                                                        .as_ref())
                                                    .or(backtest_lockstep_m10_regime_off_norisk_nocomm
                                                        .as_ref())
                                                    .or(backtest_lockstep_m10_regime_off_norisk
                                                        .as_ref())
                                                    .or(backtest_lockstep_m7_regime_off_norisk_eps0
                                                        .as_ref())
                                                    .or(backtest_lockstep_m7_regime_off_norisk.as_ref())
                                                    .or(backtest_lockstep_m7_regime_off.as_ref())
                                                    .or(backtest_lockstep_m7.as_ref())
                                                    .expect("checked lockstep m7 kernel availability")
                                            } else {
                                                backtest_lockstep_m10_regime_off_norisk_eps0
                                                    .as_ref()
                                                    .or(backtest_lockstep_m10_regime_off_norisk
                                                        .as_ref())
                                                    .or(backtest_lockstep_m7_regime_off_norisk_eps0
                                                        .as_ref())
                                                    .or(backtest_lockstep_m7_regime_off_norisk
                                                        .as_ref())
                                                    .or(backtest_lockstep_m7_regime_off.as_ref())
                                                    .or(backtest_lockstep_m7.as_ref())
                                                    .expect(
                                                        "checked lockstep m7 kernel availability",
                                                    )
                                            }
                                        } else {
                                            backtest_lockstep_m7_regime_off_norisk_eps0
                                                .as_ref()
                                                .or(backtest_lockstep_m7_regime_off_norisk.as_ref())
                                                .or(backtest_lockstep_m7_regime_off.as_ref())
                                                .or(backtest_lockstep_m7.as_ref())
                                                .expect("checked lockstep m7 kernel availability")
                                        }
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
                                    if use_lockstep_m10_product {
                                        if zero_commission_mode {
                                            backtest_lockstep_m10_regime_off_norisk_nocomm
                                                .as_ref()
                                                .or(backtest_lockstep_m10_regime_off_norisk
                                                    .as_ref())
                                                .or(backtest_lockstep_m7_regime_off_norisk.as_ref())
                                                .or(backtest_lockstep_m7_regime_off.as_ref())
                                                .or(backtest_lockstep_m7.as_ref())
                                                .expect("checked lockstep m7 kernel availability")
                                        } else {
                                            backtest_lockstep_m10_regime_off_norisk
                                                .as_ref()
                                                .or(backtest_lockstep_m7_regime_off_norisk.as_ref())
                                                .or(backtest_lockstep_m7_regime_off.as_ref())
                                                .or(backtest_lockstep_m7.as_ref())
                                                .expect("checked lockstep m7 kernel availability")
                                        }
                                    } else {
                                        backtest_lockstep_m7_regime_off_norisk
                                            .as_ref()
                                            .or(backtest_lockstep_m7_regime_off.as_ref())
                                            .or(backtest_lockstep_m7.as_ref())
                                            .expect("checked lockstep m7 kernel availability")
                                    }
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
                        } else if use_lockstep_m17 {
                            match fixed_contract_flavor {
                                FixedContractKernelFlavor::Base => backtest_lockstep_m17
                                    .as_ref()
                                    .expect("checked lockstep m17 kernel availability"),
                                FixedContractKernelFlavor::RegimeOffNoRisk => {
                                    backtest_lockstep_m17_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskNoComm => {
                                    backtest_lockstep_m17_regime_off_norisk_nocomm
                                        .as_ref()
                                        .or(backtest_lockstep_m17_regime_off_norisk.as_ref())
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskEps0 => {
                                    backtest_lockstep_m17_regime_off_norisk_eps0
                                        .as_ref()
                                        .or(backtest_lockstep_m17_regime_off_norisk.as_ref())
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm => {
                                    backtest_lockstep_m17_regime_off_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_lockstep_m17_regime_off_norisk_eps0.as_ref())
                                        .or(backtest_lockstep_m17_regime_off_norisk_nocomm.as_ref())
                                        .or(backtest_lockstep_m17_regime_off_norisk.as_ref())
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOnNoRisk => {
                                    backtest_lockstep_m17_regime_on_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm => {
                                    backtest_lockstep_m17_regime_on_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_lockstep_m17_regime_on_norisk.as_ref())
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeHybridNoRisk => {
                                    backtest_lockstep_m17_regime_hybrid_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm => {
                                    backtest_lockstep_m17_regime_hybrid_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_lockstep_m17_regime_hybrid_norisk.as_ref())
                                        .or(backtest_lockstep_m17.as_ref())
                                        .expect("checked lockstep m17 kernel availability")
                                }
                            }
                        } else if use_lockstep_m22 {
                            match fixed_contract_flavor {
                                FixedContractKernelFlavor::Base => backtest_lockstep_m22
                                    .as_ref()
                                    .expect("checked lockstep m22 kernel availability"),
                                FixedContractKernelFlavor::RegimeOffNoRisk => {
                                    backtest_lockstep_m22_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskNoComm => {
                                    backtest_lockstep_m22_regime_off_norisk_nocomm
                                        .as_ref()
                                        .or(backtest_lockstep_m22_regime_off_norisk.as_ref())
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskEps0 => {
                                    backtest_lockstep_m22_regime_off_norisk_eps0
                                        .as_ref()
                                        .or(backtest_lockstep_m22_regime_off_norisk.as_ref())
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm => {
                                    backtest_lockstep_m22_regime_off_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_lockstep_m22_regime_off_norisk_eps0.as_ref())
                                        .or(backtest_lockstep_m22_regime_off_norisk_nocomm.as_ref())
                                        .or(backtest_lockstep_m22_regime_off_norisk.as_ref())
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOnNoRisk => {
                                    backtest_lockstep_m22_regime_on_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm => {
                                    backtest_lockstep_m22_regime_on_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_lockstep_m22_regime_on_norisk.as_ref())
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeHybridNoRisk => {
                                    backtest_lockstep_m22_regime_hybrid_norisk
                                        .as_ref()
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm => {
                                    backtest_lockstep_m22_regime_hybrid_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_lockstep_m22_regime_hybrid_norisk.as_ref())
                                        .or(backtest_lockstep_m22.as_ref())
                                        .expect("checked lockstep m22 kernel availability")
                                }
                            }
                        } else if use_lockstep_time {
                            backtest_lockstep
                                .as_ref()
                                .expect("checked lockstep kernel availability")
                        } else if use_fast_warp_dedup {
                            backtest_fast_warp
                                .as_ref()
                                .expect("checked fast dedup kernel availability")
                        } else if use_m17 {
                            match fixed_contract_flavor {
                                FixedContractKernelFlavor::Base => backtest_m17
                                    .as_ref()
                                    .expect("checked m17 kernel availability"),
                                FixedContractKernelFlavor::RegimeOffNoRisk => {
                                    backtest_m17_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskNoComm => {
                                    backtest_m17_regime_off_norisk_nocomm
                                        .as_ref()
                                        .or(backtest_m17_regime_off_norisk.as_ref())
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskEps0 => {
                                    backtest_m17_regime_off_norisk_eps0
                                        .as_ref()
                                        .or(backtest_m17_regime_off_norisk.as_ref())
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm => {
                                    backtest_m17_regime_off_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_m17_regime_off_norisk_eps0.as_ref())
                                        .or(backtest_m17_regime_off_norisk_nocomm.as_ref())
                                        .or(backtest_m17_regime_off_norisk.as_ref())
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOnNoRisk => {
                                    backtest_m17_regime_on_norisk
                                        .as_ref()
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm => {
                                    backtest_m17_regime_on_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_m17_regime_on_norisk.as_ref())
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeHybridNoRisk => {
                                    backtest_m17_regime_hybrid_norisk
                                        .as_ref()
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm => {
                                    backtest_m17_regime_hybrid_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_m17_regime_hybrid_norisk.as_ref())
                                        .or(backtest_m17.as_ref())
                                        .expect("checked m17 kernel availability")
                                }
                            }
                        } else if use_m22 {
                            match fixed_contract_flavor {
                                FixedContractKernelFlavor::Base => backtest_m22
                                    .as_ref()
                                    .expect("checked m22 kernel availability"),
                                FixedContractKernelFlavor::RegimeOffNoRisk => {
                                    backtest_m22_regime_off_norisk
                                        .as_ref()
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskNoComm => {
                                    backtest_m22_regime_off_norisk_nocomm
                                        .as_ref()
                                        .or(backtest_m22_regime_off_norisk.as_ref())
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskEps0 => {
                                    backtest_m22_regime_off_norisk_eps0
                                        .as_ref()
                                        .or(backtest_m22_regime_off_norisk.as_ref())
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm => {
                                    backtest_m22_regime_off_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_m22_regime_off_norisk_eps0.as_ref())
                                        .or(backtest_m22_regime_off_norisk_nocomm.as_ref())
                                        .or(backtest_m22_regime_off_norisk.as_ref())
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOnNoRisk => {
                                    backtest_m22_regime_on_norisk
                                        .as_ref()
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm => {
                                    backtest_m22_regime_on_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_m22_regime_on_norisk.as_ref())
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeHybridNoRisk => {
                                    backtest_m22_regime_hybrid_norisk
                                        .as_ref()
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                                FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm => {
                                    backtest_m22_regime_hybrid_norisk_eps0_nocomm
                                        .as_ref()
                                        .or(backtest_m22_regime_hybrid_norisk.as_ref())
                                        .or(backtest_m22.as_ref())
                                        .expect("checked m22 kernel availability")
                                }
                            }
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
                                &mut pair_eval_start_rows as *mut _ as *mut c_void,
                                &mut pair_eval_end_rows as *mut _ as *mut c_void,
                                &mut commission as *mut _ as *mut c_void,
                                &mut eps_rel as *mut _ as *mut c_void,
                                &mut hurdle_tau_f as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_fast_rows as *mut _ as *mut c_void,
                                &mut pair_slow_rows as *mut _ as *mut c_void,
                                &mut pair_fast_local_rows as *mut _ as *mut c_void,
                                &mut pair_slow_local_rows as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            launch_kernel_timed(
                                bt_stream,
                                backtest_fn,
                                (grid_x, 1, 1),
                                (block_x, 1, 1),
                                0,
                                args,
                                &mut launch_submit_elapsed,
                            )?;
                        } else if exact_pruning_mode {
                            let mut prune_max_log_dd =
                                exact_pruning_max_log_dd.unwrap_or(f32::INFINITY);
                            let mut prune_counters_ptr = exact_pruning_counters_ptr;
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
                                &mut hurdle_tau_f as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_fast_rows as *mut _ as *mut c_void,
                                &mut pair_slow_rows as *mut _ as *mut c_void,
                                &mut pair_fast_local_rows as *mut _ as *mut c_void,
                                &mut pair_slow_local_rows as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                                &mut prune_max_log_dd as *mut _ as *mut c_void,
                                &mut prune_counters_ptr as *mut _ as *mut c_void,
                            ];
                            launch_kernel_timed(
                                bt_stream,
                                backtest_fn,
                                (grid_x, 1, 1),
                                (block_x, 1, 1),
                                0,
                                args,
                                &mut launch_submit_elapsed,
                            )?;
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
                                &mut hurdle_tau_f as *mut _ as *mut c_void,
                                &mut stop_log as *mut _ as *mut c_void,
                                &mut take_log as *mut _ as *mut c_void,
                                &mut flags_u as *mut _ as *mut c_void,
                                &mut m_i as *mut _ as *mut c_void,
                                &mut metrics_stride_i as *mut _ as *mut c_void,
                                &mut pair_map_ptr as *mut _ as *mut c_void,
                                &mut pair_fast_rows as *mut _ as *mut c_void,
                                &mut pair_slow_rows as *mut _ as *mut c_void,
                                &mut pair_fast_local_rows as *mut _ as *mut c_void,
                                &mut pair_slow_local_rows as *mut _ as *mut c_void,
                                &mut pair_count_i as *mut _ as *mut c_void,
                                &mut compact_output_i as *mut _ as *mut c_void,
                                &mut out_ptr as *mut _ as *mut c_void,
                            ];
                            launch_kernel_timed(
                                bt_stream,
                                backtest_fn,
                                (grid_x, 1, 1),
                                (block_x, 1, 1),
                                0,
                                args,
                                &mut launch_submit_elapsed,
                            )?;
                        }
                    }
                    if let Some(t0) = backtest_t0 {
                        bt_stream.synchronize().map_err(|e| e.to_string())?;
                        tile_backtest_elapsed
                            .set(tile_backtest_elapsed.get().saturating_add(t0.elapsed()));
                    }
                }

                if let (Some(h), Some(h_sum), Some(h_count), Some((f_min, f_max, s_min, s_max))) = (
                    scratch.d_heatmap.as_ref(),
                    scratch.d_heatmap_sum.as_ref(),
                    scratch.d_heatmap_count.as_ref(),
                    heatmap_cfg,
                ) {
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
                        let mut hurdle_tau_i = hurdle_tau;
                        let mut bins_f = heatmap_bins as i32;
                        let mut bins_s = heatmap_bins as i32;
                        let mut f_min_i = f_min as i32;
                        let mut f_max_i = f_max as i32;
                        let mut s_min_i = s_min as i32;
                        let mut s_max_i = s_max as i32;
                        let mut out_ptr = h.as_device_ptr().as_raw();
                        let mut sum_ptr = h_sum.as_device_ptr().as_raw();
                        let mut count_ptr = h_count.as_device_ptr().as_raw();

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
                            &mut hurdle_tau_i as *mut _ as *mut c_void,
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
                            &mut sum_ptr as *mut _ as *mut c_void,
                            &mut count_ptr as *mut _ as *mut c_void,
                        ];
                        launch_kernel_timed(
                            bt_stream,
                            &heatmap_update,
                            (grid_x, 1, 1),
                            (block_x, 1, 1),
                            0,
                            args,
                            &mut launch_submit_elapsed,
                        )?;
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
                    let _nvtx_scope = crate::nvtx::scope("kernel", "graph_capture");
                    let graph_capture_t0 = Instant::now();
                    match capture_graph_exec(bt_stream, || launch_score_stage()) {
                        Ok(exec) => {
                            graph_capture_elapsed =
                                graph_capture_elapsed.saturating_add(graph_capture_t0.elapsed());
                            tile_graph_shape = Some(shape);
                            tile_graph_exec = Some(exec);
                        }
                        Err(_e) => {
                            graph_capture_elapsed =
                                graph_capture_elapsed.saturating_add(graph_capture_t0.elapsed());
                            graph_capture_disabled = true;
                            let _launch_submit_scope =
                                crate::nvtx::scope("kernel", "launch_submit");
                            launch_score_stage()?;
                        }
                    }
                }
                if !graph_capture_disabled {
                    if let Some(exec) = tile_graph_exec.as_ref() {
                        let _graph_replay_scope = crate::nvtx::scope("kernel", "graph_replay");
                        graph_launch_timed(exec, bt_stream, &mut launch_submit_elapsed)?;
                        graph_replay_count = graph_replay_count.saturating_add(1);
                    }
                }
            } else {
                let _launch_submit_scope = crate::nvtx::scope("kernel", "launch_submit");
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

            if device_include_all {
                let d2h_t0 = Instant::now();
                copy_device_prefix_to_host_f32(
                    scratch.d_tile.as_ref().unwrap(),
                    &mut tile_host_buf,
                    tile_row_count.saturating_mul(tile_metrics_stride),
                    bt_stream,
                )?;
                d2h_copy_elapsed = d2h_copy_elapsed.saturating_add(d2h_t0.elapsed());
                sync_stream_timed(bt_stream, &mut explicit_sync_elapsed)?;

                let decode_t0 = Instant::now();
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
                            cfg.hurdle_tau,
                        );
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
                host_decode_elapsed = host_decode_elapsed.saturating_add(decode_t0.elapsed());
            } else {
                let s_len = slow_chunk.len();
                let mut tile_valid: usize = 0;
                for &f in fast_chunk {
                    let idx = slow_chunk.partition_point(|&s| s <= f);
                    tile_valid = tile_valid.saturating_add(s_len.saturating_sub(idx));
                }

                if host_filter_mode {
                    let d2h_t0 = Instant::now();
                    copy_device_prefix_to_host_f32(
                        scratch.d_tile.as_ref().unwrap(),
                        &mut tile_host_buf,
                        tile_row_count.saturating_mul(tile_metrics_stride),
                        bt_stream,
                    )?;
                    d2h_copy_elapsed = d2h_copy_elapsed.saturating_add(d2h_t0.elapsed());
                    sync_stream_timed(bt_stream, &mut explicit_sync_elapsed)?;

                    let mut exported = 0usize;
                    let decode_t0 = Instant::now();
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
                                cfg.hurdle_tau,
                            );
                            if !metrics_pass_constraints(&metrics, constraints) {
                                return Ok(());
                            }

                            let params = DoubleMaParams {
                                fast_len,
                                slow_len,
                                fast_ma_id: 0,
                                slow_ma_id: 0,
                            };
                            agg.push(params.clone(), metrics.clone());

                            if let Some(w) = export_writer.as_mut() {
                                let score = kernel_effective_score_for_pair(
                                    &params, &metrics, objective, score_mode,
                                );
                                let active_mean = derived_active_mean(&metrics);
                                let active_downside_deviation =
                                    derived_active_downside_deviation(&metrics);
                                let active_sortino = derived_active_sortino(&metrics);
                                let drawdown_chronicity = derived_drawdown_chronicity(&metrics);
                                let downside_share = derived_downside_share(&metrics);
                                let long_exposure = derived_long_exposure(&metrics);
                                let short_exposure = derived_short_exposure(&metrics);
                                let directional_bias = derived_directional_bias(&metrics);
                                let fee_stress_pnl = exact_fee_stress_pnl(&metrics, strategy);
                                let denom =
                                    metrics.winning_trades.saturating_add(metrics.losing_trades);
                                let win_rate = if denom > 0 {
                                    metrics.winning_trades as f64 / denom as f64
                                } else {
                                    0.0
                                };
                                let avg_win = if metrics.winning_trades > 0 {
                                    metrics.gross_profit / metrics.winning_trades as f64
                                } else {
                                    0.0
                                };
                                let avg_loss = if metrics.losing_trades > 0 {
                                    -(metrics.gross_loss / metrics.losing_trades as f64)
                                } else {
                                    0.0
                                };
                                let profit_factor = if metrics.gross_loss > 0.0 {
                                    metrics.gross_profit / metrics.gross_loss
                                } else if metrics.gross_profit > 0.0 {
                                    f64::INFINITY
                                } else {
                                    0.0
                                };
                                let expectancy = if metrics.closed_trades > 0 {
                                    (metrics.gross_profit - metrics.gross_loss)
                                        / metrics.closed_trades as f64
                                } else {
                                    0.0
                                };
                                writeln!(
                                    w,
                                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                                    fast_ma.as_str(),
                                    slow_ma.as_str(),
                                    fast_len,
                                    slow_len,
                                    metrics.pnl,
                                    metrics.sharpe,
                                    metrics.max_dd,
                                    metrics.downside_deviation,
                                    metrics.downside_frequency,
                                    metrics.downside_severity,
                                    metrics.ulcer_index,
                                    metrics.omega,
                                    metrics.kappa_tau,
                                    metrics.martin_ratio,
                                    metrics.trades,
                                    metrics.exposure,
                                    metrics.net_exposure,
                                    metrics.long_mean_return,
                                    metrics.short_mean_return,
                                    metrics.underwater_fraction,
                                    metrics.max_underwater_duration_bars,
                                    metrics.closed_trades,
                                    metrics.winning_trades,
                                    metrics.losing_trades,
                                    metrics.gross_profit,
                                    metrics.gross_loss,
                                    win_rate,
                                    avg_win,
                                    avg_loss,
                                    profit_factor,
                                    expectancy,
                                    active_mean,
                                    active_downside_deviation,
                                    active_sortino,
                                    drawdown_chronicity,
                                    downside_share,
                                    long_exposure,
                                    short_exposure,
                                    directional_bias,
                                    fee_stress_pnl,
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
                    host_decode_elapsed = host_decode_elapsed.saturating_add(decode_t0.elapsed());

                    processed = processed.saturating_add(tile_valid);
                    if cancel.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }
                    emit_progress_and_maybe_stage_campaign_prefetch(
                        cfg.device_id,
                        resident_market,
                        staging_stream,
                        staged_market,
                        progress,
                        processed,
                        total_pairs,
                        "gpu-kernel",
                    );
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

                if score_first_topk_mode {
                    select_tile_topk_scores_for_objective(
                        bt_stream,
                        scratch,
                        pairs,
                        launch_pairs,
                        metrics_count,
                        tile_metrics_stride,
                        pf,
                        ps,
                        topk_select,
                        primary_objective_i,
                        hurdle_tau,
                        compact_valid_pair_output,
                        pair_index_map_ptr,
                        pair_fast_rows_ptr,
                        pair_slow_rows_ptr,
                        constraints_on_device,
                        constraint_args,
                        select_topk_scored_pairs
                            .as_ref()
                            .ok_or_else(|| "missing select_topk_scored_pairs_tm_f32".to_string())?,
                        select_topk_scored_candidates.as_ref().ok_or_else(|| {
                            "missing select_topk_scored_candidates_tm_f32".to_string()
                        })?,
                        select_topk_scored_pairs_v2.as_ref(),
                        gather_topk_scored_pairs
                            .as_ref()
                            .ok_or_else(|| "missing gather_topk_scored_pairs_tm_f32".to_string())?,
                        &mut launch_submit_elapsed,
                    )?;
                } else {
                    select_tile_topk_rows_for_objective(
                        bt_stream,
                        scratch,
                        pairs,
                        launch_pairs,
                        metrics_count,
                        tile_metrics_stride,
                        pf,
                        ps,
                        topk_select,
                        primary_objective_i,
                        hurdle_tau,
                        compact_valid_pair_output,
                        pair_index_map_ptr,
                        pair_fast_rows_ptr,
                        pair_slow_rows_ptr,
                        constraints_on_device,
                        constraint_args,
                        &select_topk_pairs_legacy,
                        &select_topk_candidates_legacy,
                        select_topk_pairs_v2.as_ref(),
                        select_topk_candidates_v2.as_ref(),
                        &gather_topk,
                        &mut launch_submit_elapsed,
                    )?;
                }

                if global_topk_mode {
                    if global_score_rows.is_some() {
                        let global_fast = global_fast_rows
                            .as_ref()
                            .ok_or_else(|| "missing global fast-row buffer".to_string())?;
                        let global_slow = global_slow_rows
                            .as_ref()
                            .ok_or_else(|| "missing global slow-row buffer".to_string())?;
                        let global_scores = global_score_rows
                            .as_ref()
                            .ok_or_else(|| "missing global score-row buffer".to_string())?;

                        merge_current_topk_scores_into_global_head(
                            bt_stream,
                            scratch,
                            global_fast,
                            global_slow,
                            global_scores,
                            &mut global_rows_len,
                            global_rows_cap,
                            topk_select,
                            select_topk_scored_rows
                                .as_ref()
                                .ok_or_else(|| "missing select_topk_scored_rows_f32".to_string())?,
                            select_topk_scored_rows_candidates.as_ref().ok_or_else(|| {
                                "missing select_topk_scored_rows_from_candidates_f32".to_string()
                            })?,
                            gather_topk_scored_rows
                                .as_ref()
                                .ok_or_else(|| "missing gather_topk_scored_rows_f32".to_string())?,
                            &mut launch_submit_elapsed,
                        )?;
                    } else {
                        let global_fast = global_fast_rows
                            .as_ref()
                            .ok_or_else(|| "missing global fast-row buffer".to_string())?;
                        let global_slow = global_slow_rows
                            .as_ref()
                            .ok_or_else(|| "missing global slow-row buffer".to_string())?;
                        let global_metrics = global_metrics_rows
                            .as_ref()
                            .ok_or_else(|| "missing global metrics-row buffer".to_string())?;
                        let select_rows_legacy = select_topk_rows_legacy
                            .as_ref()
                            .ok_or_else(|| "missing select_topk_metric_rows_f32".to_string())?;
                        let select_rows_candidates_legacy =
                            select_topk_rows_candidates_legacy.as_ref().ok_or_else(|| {
                                "missing select_topk_metric_rows_from_candidates_f32".to_string()
                            })?;
                        let gather_rows = gather_topk_rows
                            .as_ref()
                            .ok_or_else(|| "missing gather_topk_metric_rows_f32".to_string())?;

                        merge_current_topk_rows_into_global_head(
                            bt_stream,
                            scratch,
                            global_fast,
                            global_slow,
                            global_metrics,
                            &mut global_rows_len,
                            global_rows_cap,
                            metrics_count,
                            metrics_count,
                            primary_objective_i,
                            hurdle_tau,
                            topk_select,
                            global_constraints_on_device,
                            constraint_args,
                            select_rows_legacy,
                            select_rows_candidates_legacy,
                            select_topk_rows_v2.as_ref(),
                            select_topk_rows_candidates_v2.as_ref(),
                            gather_rows,
                            &mut launch_submit_elapsed,
                        )?;

                        for head in extra_shortlist_heads.iter_mut() {
                            select_tile_topk_rows_for_objective(
                                bt_stream,
                                scratch,
                                pairs,
                                launch_pairs,
                                metrics_count,
                                tile_metrics_stride,
                                pf,
                                ps,
                                topk_select,
                                head.objective_i,
                                hurdle_tau,
                                compact_valid_pair_output,
                                pair_index_map_ptr,
                                pair_fast_rows_ptr,
                                pair_slow_rows_ptr,
                                constraints_on_device,
                                constraint_args,
                                &select_topk_pairs_legacy,
                                &select_topk_candidates_legacy,
                                select_topk_pairs_v2.as_ref(),
                                select_topk_candidates_v2.as_ref(),
                                &gather_topk,
                                &mut launch_submit_elapsed,
                            )?;
                            let head_global_fast = head.global_fast_rows.as_ref().ok_or_else(|| {
                                format!(
                                    "missing global fast-row buffer for extra shortlist objective {:?}",
                                    head.objective
                                )
                            })?;
                            let head_global_slow = head.global_slow_rows.as_ref().ok_or_else(|| {
                                format!(
                                    "missing global slow-row buffer for extra shortlist objective {:?}",
                                    head.objective
                                )
                            })?;
                            let head_global_metrics = head
                                .global_metrics_rows
                                .as_ref()
                                .ok_or_else(|| {
                                    format!(
                                        "missing global metrics-row buffer for extra shortlist objective {:?}",
                                        head.objective
                                    )
                                })?;
                            merge_current_topk_rows_into_global_head(
                                bt_stream,
                                scratch,
                                head_global_fast,
                                head_global_slow,
                                head_global_metrics,
                                &mut head.global_rows_len,
                                global_rows_cap,
                                metrics_count,
                                metrics_count,
                                head.objective_i,
                                hurdle_tau,
                                topk_select,
                                global_constraints_on_device,
                                constraint_args,
                                select_rows_legacy,
                                select_rows_candidates_legacy,
                                select_topk_rows_v2.as_ref(),
                                select_topk_rows_candidates_v2.as_ref(),
                                gather_rows,
                                &mut launch_submit_elapsed,
                            )?;
                        }
                    }
                } else {
                    sync_stream_timed(bt_stream, &mut explicit_sync_elapsed)?;

                    if let Some(w) = export_writer.as_mut() {
                        let d2h_t0 = Instant::now();
                        copy_device_prefix_to_host_f32(
                            scratch.d_tile.as_ref().unwrap(),
                            &mut tile_host_buf,
                            tile_row_count.saturating_mul(tile_metrics_stride),
                            bt_stream,
                        )?;
                        d2h_copy_elapsed = d2h_copy_elapsed.saturating_add(d2h_t0.elapsed());
                        sync_stream_timed(bt_stream, &mut explicit_sync_elapsed)?;

                        let mut exported = 0usize;
                        let decode_t0 = Instant::now();
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
                                    cfg.hurdle_tau,
                                );

                                let params = DoubleMaParams {
                                    fast_len,
                                    slow_len,
                                    fast_ma_id: 0,
                                    slow_ma_id: 0,
                                };
                                let score = kernel_effective_score_for_pair(
                                    &params, &metrics, objective, score_mode,
                                );
                                let active_mean = derived_active_mean(&metrics);
                                let active_downside_deviation =
                                    derived_active_downside_deviation(&metrics);
                                let active_sortino = derived_active_sortino(&metrics);
                                let drawdown_chronicity = derived_drawdown_chronicity(&metrics);
                                let downside_share = derived_downside_share(&metrics);
                                let long_exposure = derived_long_exposure(&metrics);
                                let short_exposure = derived_short_exposure(&metrics);
                                let directional_bias = derived_directional_bias(&metrics);
                                let fee_stress_pnl = exact_fee_stress_pnl(&metrics, strategy);
                                let denom =
                                    metrics.winning_trades.saturating_add(metrics.losing_trades);
                                let win_rate = if denom > 0 {
                                    metrics.winning_trades as f64 / denom as f64
                                } else {
                                    0.0
                                };
                                let avg_win = if metrics.winning_trades > 0 {
                                    metrics.gross_profit / metrics.winning_trades as f64
                                } else {
                                    0.0
                                };
                                let avg_loss = if metrics.losing_trades > 0 {
                                    -(metrics.gross_loss / metrics.losing_trades as f64)
                                } else {
                                    0.0
                                };
                                let profit_factor = if metrics.gross_loss > 0.0 {
                                    metrics.gross_profit / metrics.gross_loss
                                } else if metrics.gross_profit > 0.0 {
                                    f64::INFINITY
                                } else {
                                    0.0
                                };
                                let expectancy = if metrics.closed_trades > 0 {
                                    (metrics.gross_profit - metrics.gross_loss)
                                        / metrics.closed_trades as f64
                                } else {
                                    0.0
                                };

                                writeln!(
                                    w,
                                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                                    fast_ma.as_str(),
                                    slow_ma.as_str(),
                                    fast_len,
                                    slow_len,
                                    metrics.pnl,
                                    metrics.sharpe,
                                    metrics.max_dd,
                                    metrics.downside_deviation,
                                    metrics.downside_frequency,
                                    metrics.downside_severity,
                                    metrics.ulcer_index,
                                    metrics.omega,
                                    metrics.kappa_tau,
                                    metrics.martin_ratio,
                                    metrics.trades,
                                    metrics.exposure,
                                    metrics.net_exposure,
                                    metrics.long_mean_return,
                                    metrics.short_mean_return,
                                    metrics.underwater_fraction,
                                    metrics.max_underwater_duration_bars,
                                    metrics.closed_trades,
                                    metrics.winning_trades,
                                    metrics.losing_trades,
                                    metrics.gross_profit,
                                    metrics.gross_loss,
                                    win_rate,
                                    avg_win,
                                    avg_loss,
                                    profit_factor,
                                    expectancy,
                                    active_mean,
                                    active_downside_deviation,
                                    active_sortino,
                                    drawdown_chronicity,
                                    downside_share,
                                    long_exposure,
                                    short_exposure,
                                    directional_bias,
                                    fee_stress_pnl,
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
                        host_decode_elapsed =
                            host_decode_elapsed.saturating_add(decode_t0.elapsed());
                    }

                    let _sync_and_decode_scope = crate::nvtx::scope("kernel", "sync_and_decode");
                    let top_packed_host_slice = pack_metric_rows_for_host_copy(
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
                        metrics_count,
                        &mut top_packed_host,
                        &mut launch_submit_elapsed,
                        &mut d2h_copy_elapsed,
                        &mut explicit_sync_elapsed,
                    )?;

                    let decode_t0 = Instant::now();
                    for r in 0..topk_select {
                        let Some((fast_len, slow_len, metrics)) = decode_packed_topk_metric_row(
                            top_packed_host_slice,
                            r,
                            metrics_count,
                            cfg.hurdle_tau,
                        ) else {
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
                    host_decode_elapsed = host_decode_elapsed.saturating_add(decode_t0.elapsed());
                }

                processed = processed.saturating_add(tile_valid);
                if cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".to_string());
                }
            }

            emit_progress_and_maybe_stage_campaign_prefetch(
                cfg.device_id,
                resident_market,
                staging_stream,
                staged_market,
                progress,
                processed,
                total_pairs,
                "gpu-kernel",
            );
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

    let sync_and_decode_scope = crate::nvtx::scope("kernel", "sync_and_decode");
    let materialize_shortlist_rows = shortlist_union_mode.is_some()
        || shortlist_confidence_mode.is_some()
        || (shortlist_slice_mode.is_some() && extra_shortlist_enabled);
    let mut host_shortlist_materialize_elapsed = Duration::ZERO;
    let mut materialized_shortlist_rows: Vec<(DoubleMaParams, Metrics)> =
        if materialize_shortlist_rows {
            Vec::with_capacity(
                global_rows_cap
                    .saturating_mul(extra_shortlist_heads.len().saturating_add(1))
                    .max(topk_select),
            )
        } else {
            Vec::new()
        };

    if global_topk_mode
        && !device_include_all
        && global_rows_len > 0
        && global_metrics_rows.is_some()
    {
        let mut shortlisted_pairs: HashSet<(u16, u16)> = HashSet::new();
        let mut push_global_head_rows = |rows_fast: &DeviceBuffer<i32>,
                                         rows_slow: &DeviceBuffer<i32>,
                                         rows_metrics: &DeviceBuffer<f32>|
         -> Result<(), String> {
            let shortlist_materialize_t0 = if materialize_shortlist_rows {
                Some(Instant::now())
            } else {
                None
            };
            let _sync_and_decode_scope = crate::nvtx::scope("kernel", "sync_and_decode");
            let top_packed_host_slice = pack_metric_rows_for_host_copy(
                bt_stream,
                scratch,
                &pack_metric_rows,
                rows_fast.as_device_ptr().as_raw(),
                rows_slow.as_device_ptr().as_raw(),
                rows_metrics.as_device_ptr().as_raw(),
                topk_select,
                metrics_count,
                metrics_count,
                &mut top_packed_host,
                &mut launch_submit_elapsed,
                &mut d2h_copy_elapsed,
                &mut explicit_sync_elapsed,
            )?;

            let decode_t0 = Instant::now();
            for r in 0..topk_select {
                let Some((fast_len, slow_len, metrics)) = decode_packed_topk_metric_row(
                    top_packed_host_slice,
                    r,
                    metrics_count,
                    cfg.hurdle_tau,
                ) else {
                    continue;
                };
                if !metrics_pass_constraints(&metrics, constraints) {
                    continue;
                }
                if !shortlisted_pairs.insert((fast_len, slow_len)) {
                    continue;
                }
                let params = DoubleMaParams {
                    fast_len,
                    slow_len,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                };
                if materialize_shortlist_rows {
                    materialized_shortlist_rows.push((params.clone(), metrics.clone()));
                }
                agg.push(params, metrics);
            }
            host_decode_elapsed = host_decode_elapsed.saturating_add(decode_t0.elapsed());
            if let Some(t0) = shortlist_materialize_t0 {
                host_shortlist_materialize_elapsed =
                    host_shortlist_materialize_elapsed.saturating_add(t0.elapsed());
            }
            Ok(())
        };

        let global_fast = global_fast_rows
            .as_ref()
            .ok_or_else(|| "missing global fast-row buffer".to_string())?;
        let global_slow = global_slow_rows
            .as_ref()
            .ok_or_else(|| "missing global slow-row buffer".to_string())?;
        let global_metrics = global_metrics_rows
            .as_ref()
            .ok_or_else(|| "missing global metrics-row buffer".to_string())?;
        push_global_head_rows(global_fast, global_slow, global_metrics)?;

        for head in extra_shortlist_heads.iter() {
            if head.global_rows_len == 0 {
                continue;
            }
            let head_fast = head.global_fast_rows.as_ref().ok_or_else(|| {
                format!(
                    "missing global fast-row buffer for extra shortlist objective {:?}",
                    head.objective
                )
            })?;
            let head_slow = head.global_slow_rows.as_ref().ok_or_else(|| {
                format!(
                    "missing global slow-row buffer for extra shortlist objective {:?}",
                    head.objective
                )
            })?;
            let head_metrics = head.global_metrics_rows.as_ref().ok_or_else(|| {
                format!(
                    "missing global metrics-row buffer for extra shortlist objective {:?}",
                    head.objective
                )
            })?;
            push_global_head_rows(head_fast, head_slow, head_metrics)?;
        }
    }

    let heatmap_out: Option<OptimizationHeatmap> = if !device_include_all {
        if let (Some(h), Some(h_sum), Some(h_count), Some((f_min, f_max, s_min, s_max))) = (
            scratch.d_heatmap.as_ref(),
            scratch.d_heatmap_sum.as_ref(),
            scratch.d_heatmap_count.as_ref(),
            heatmap_cfg,
        ) {
            let heatmap_len = heatmap_bins.saturating_mul(heatmap_bins);
            if heatmap_len == 0 {
                None
            } else {
                let mut host: Vec<f32> = alloc_uninit_copy(heatmap_len);
                let d2h_t0 = Instant::now();
                h.copy_to(&mut host).map_err(|e| e.to_string())?;
                let mut host_sum: Vec<f32> = alloc_uninit_copy(heatmap_len);
                h_sum.copy_to(&mut host_sum).map_err(|e| e.to_string())?;
                let mut host_count: Vec<i32> = alloc_uninit_copy(heatmap_len);
                h_count
                    .copy_to(&mut host_count)
                    .map_err(|e| e.to_string())?;
                d2h_copy_elapsed = d2h_copy_elapsed.saturating_add(d2h_t0.elapsed());
                let decode_t0 = Instant::now();
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
                let mean_values: Vec<Option<f64>> = host_sum
                    .into_iter()
                    .zip(host_count.iter().copied())
                    .map(|(sum, count)| {
                        if count > 0 {
                            Some(sum as f64 / count as f64)
                        } else {
                            None
                        }
                    })
                    .collect();
                let sample_counts: Vec<u32> = host_count
                    .into_iter()
                    .map(|count| count.max(0) as u32)
                    .collect();
                host_decode_elapsed = host_decode_elapsed.saturating_add(decode_t0.elapsed());
                Some(OptimizationHeatmap {
                    bins_fast: heatmap_bins,
                    bins_slow: heatmap_bins,
                    fast_min: f_min,
                    fast_max: f_max,
                    slow_min: s_min,
                    slow_max: s_max,
                    values,
                    mean_values: Some(mean_values),
                    sample_counts: Some(sample_counts),
                })
            }
        } else {
            None
        }
    } else {
        None
    };

    if global_rows_len > 0 && global_score_rows.is_some() {
        let global_fast = global_fast_rows
            .as_ref()
            .ok_or_else(|| "missing global fast-row buffer".to_string())?;
        let global_slow = global_slow_rows
            .as_ref()
            .ok_or_else(|| "missing global slow-row buffer".to_string())?;

        let d2h_t0 = Instant::now();
        let mut fast_host: Vec<i32> = Vec::new();
        let mut slow_host: Vec<i32> = Vec::new();
        copy_device_prefix_to_host_i32(global_fast, &mut fast_host, global_rows_len, bt_stream)?;
        copy_device_prefix_to_host_i32(global_slow, &mut slow_host, global_rows_len, bt_stream)?;
        d2h_copy_elapsed = d2h_copy_elapsed.saturating_add(d2h_t0.elapsed());
        sync_stream_timed(bt_stream, &mut explicit_sync_elapsed)?;

        let mut finalists: Vec<DoubleMaParams> = Vec::with_capacity(global_rows_len);
        for idx in 0..global_rows_len {
            let fast_len_i = fast_host[idx];
            let slow_len_i = slow_host[idx];
            if fast_len_i <= 0 || slow_len_i <= 0 || fast_len_i >= slow_len_i {
                continue;
            }
            let fast_len = u16::try_from(fast_len_i)
                .map_err(|_| format!("score-first fast period out of range: {fast_len_i}"))?;
            let slow_len = u16::try_from(slow_len_i)
                .map_err(|_| format!("score-first slow period out of range: {slow_len_i}"))?;
            finalists.push(DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            });
        }

        if finalists.is_empty() {
            return Err("scored-candidate exact shortlist produced no valid finalists".to_string());
        }

        let decode_t0 = Instant::now();
        let ranges = [eval_range.unwrap_or((0, candles.close.len()))];
        let replayed_metrics = eval_double_ma_batch_gpu_kernel_impl(
            rt,
            candles,
            finalists.as_slice(),
            fast_periods,
            slow_periods,
            fast_ma.as_str(),
            slow_ma.as_str(),
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter,
            cfg,
            fast_precomputed,
            slow_precomputed,
            strategy,
            false,
            &ranges,
            None,
            false,
            None,
        )?;
        let mut exact_rows: Vec<(DoubleMaParams, Metrics)> = finalists
            .into_iter()
            .zip(replayed_metrics.into_iter())
            .filter(|(_, metrics)| metrics_pass_constraints(metrics, constraints))
            .collect();
        exact_rows.sort_by(|a, b| {
            let sa = kernel_effective_score_for_pair(&a.0, &a.1, objective, score_mode);
            let sb = kernel_effective_score_for_pair(&b.0, &b.1, objective, score_mode);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        for (params, metrics) in exact_rows {
            agg.push(params, metrics);
        }
        host_decode_elapsed = host_decode_elapsed.saturating_add(decode_t0.elapsed());
    }

    drop(sync_and_decode_scope);
    let _host_finalize_scope = crate::nvtx::scope("kernel", "host_finalize");
    let final_finalize_t0 = Instant::now();
    let mut host_finalize_agg_finalize_elapsed = Duration::ZERO;
    let mut host_finalize_finalist_recompute_elapsed = Duration::ZERO;
    let mut host_finalize_shortlist_union_elapsed = Duration::ZERO;
    let mut host_finalize_shortlist_confidence_elapsed = Duration::ZERO;
    let mut host_finalize_shortlist_slice_elapsed = Duration::ZERO;
    let mut shortlist_slice_timing = ShortlistSliceTimingProfile::default();
    let mut host_finalize_export_flush_elapsed = Duration::ZERO;
    let mut host_finalize_cleanup_elapsed = Duration::ZERO;

    let agg_finalize_t0 = Instant::now();
    let mut result = agg
        .finalize()
        .ok_or_else(|| "grid search produced no result".to_string())?;
    host_finalize_agg_finalize_elapsed =
        host_finalize_agg_finalize_elapsed.saturating_add(agg_finalize_t0.elapsed());

    let finalist_recompute_required = metrics_count < METRICS_COUNT
        && !can_decode_current_gpu_product_metrics_without_recompute(metrics_count);
    if !device_include_all
        && finalist_recompute_required
        && !fast_precomputed_enabled
        && !slow_precomputed_enabled
        && kernel_finalist_recompute_enabled()
    {
        let finalist_recompute_t0 = Instant::now();
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
            eval_range,
        )?;
        host_finalize_finalist_recompute_elapsed = host_finalize_finalist_recompute_elapsed
            .saturating_add(finalist_recompute_t0.elapsed());
    }
    if let Some(mode) = shortlist_union_mode {
        if materialized_shortlist_rows.is_empty() {
            return Err(
                "shortlist-union mode produced no shortlist rows after device filtering"
                    .to_string(),
            );
        }
        let top_limit = result.top.len().max(1);
        let shortlist_union_t0 = Instant::now();
        let reranked = rerank_shortlist_by_union_mode(
            &materialized_shortlist_rows,
            objective,
            extra_shortlist_objectives.as_slice(),
            mode,
        );
        host_finalize_shortlist_union_elapsed =
            host_finalize_shortlist_union_elapsed.saturating_add(shortlist_union_t0.elapsed());
        materialized_shortlist_rows = reranked.clone();
        if let Some((best_params, best_metrics)) = reranked.first() {
            result.best_params = best_params.clone();
            result.best_metrics = best_metrics.clone();
            result.top = reranked.into_iter().take(top_limit).collect();
        }
    }
    if let Some(mode) = shortlist_confidence_mode {
        if materialized_shortlist_rows.is_empty() {
            return Err(
                "shortlist-confidence mode produced no shortlist rows after device filtering"
                    .to_string(),
            );
        }
        let top_limit = result.top.len().max(1);
        let shortlist_confidence_t0 = Instant::now();
        let reranked = rerank_shortlist_by_confidence(
            &materialized_shortlist_rows,
            mode,
            first_valid,
            regime_first_valid,
            eval_start_t,
            eval_end_t,
            n,
            strategy,
            objective,
            score_mode,
        );
        host_finalize_shortlist_confidence_elapsed = host_finalize_shortlist_confidence_elapsed
            .saturating_add(shortlist_confidence_t0.elapsed());
        if let Some((best_params, best_metrics)) = reranked.first() {
            result.best_params = best_params.clone();
            result.best_metrics = best_metrics.clone();
            result.top = reranked.into_iter().take(top_limit).collect();
        }
    }
    if let Some(mode) = shortlist_slice_mode {
        let top_limit = result.top.len().max(1);
        let slice_rows: &[(DoubleMaParams, Metrics)] = if materialized_shortlist_rows.is_empty() {
            result.top.as_slice()
        } else {
            materialized_shortlist_rows.as_slice()
        };
        let shortlist_slice_t0 = Instant::now();
        let (reranked, diagnostics) = rerank_shortlist_by_slice_robustness(
            rt,
            candles,
            slice_rows,
            top_limit,
            top_limit,
            mode,
            fast_ma.as_str(),
            slow_ma.as_str(),
            ma_source,
            fast_ma_params,
            slow_ma_params,
            regime_filter,
            cfg,
            fast_precomputed,
            slow_precomputed,
            strategy,
            objective,
            score_mode,
            shortlist_slice_benchmark.as_ref(),
            first_valid,
            regime_first_valid,
            eval_start_t,
            eval_end_t,
            Some(&mut shortlist_slice_timing),
        )?;
        host_finalize_shortlist_slice_elapsed =
            host_finalize_shortlist_slice_elapsed.saturating_add(shortlist_slice_t0.elapsed());
        record_last_shortlist_slice_diagnostics(diagnostics);
        let (best_params, best_metrics) = reranked
            .first()
            .cloned()
            .ok_or_else(|| "shortlist-slice rerank produced no result".to_string())?;
        result.best_params = best_params;
        result.best_metrics = best_metrics;
        result.top = reranked.into_iter().take(top_limit).collect();
    }

    if !device_include_all {
        result.num_combos = total_pairs;
        if heatmap_out.is_some() {
            result.heatmap = heatmap_out;
        }
    }

    if let Some(counters) = exact_pruning_counters.as_ref() {
        let mut host = [0_u64; 2];
        let d2h_t0 = Instant::now();
        counters.copy_to(&mut host).map_err(|e| e.to_string())?;
        d2h_copy_elapsed = d2h_copy_elapsed.saturating_add(d2h_t0.elapsed());
        append_last_route_diagnostics([
            format!("gpu_kernel.exact_pruning_pruned_candidates={}", host[0]),
            format!("gpu_kernel.exact_pruning_skipped_bars={}", host[1]),
        ]);
    }

    if let Some(w) = export_writer.as_mut() {
        let export_flush_t0 = Instant::now();
        w.flush().map_err(|e| e.to_string())?;
        host_finalize_export_flush_elapsed =
            host_finalize_export_flush_elapsed.saturating_add(export_flush_t0.elapsed());
    }

    let cleanup_t0 = Instant::now();
    if !kernel_persist_buffers_enabled() {
        rt.ma.clear_cached_constants();
        rt.scratch.clear_async(&rt.stream)?;
        rt.host_exact_combo_scratch.last_fast_period_upload = None;
        rt.host_exact_combo_scratch.last_slow_period_upload = None;
        rt.host_exact_combo_scratch.last_pair_index_upload = None;
        rt.host_exact_combo_scratch.last_eval_start_upload = None;
        rt.host_exact_combo_scratch.last_eval_end_upload = None;
    }
    host_finalize_cleanup_elapsed =
        host_finalize_cleanup_elapsed.saturating_add(cleanup_t0.elapsed());

    let final_finalize_elapsed = final_finalize_t0.elapsed();
    host_finalize_elapsed = host_finalize_elapsed.saturating_add(final_finalize_elapsed);
    if let Some(p) = pipeline_profile.as_mut() {
        p.final_finalize = p.final_finalize.saturating_add(final_finalize_elapsed);
        p.final_agg_finalize = p
            .final_agg_finalize
            .saturating_add(host_finalize_agg_finalize_elapsed);
        p.final_finalist_recompute = p
            .final_finalist_recompute
            .saturating_add(host_finalize_finalist_recompute_elapsed);
        p.final_shortlist_union = p
            .final_shortlist_union
            .saturating_add(host_finalize_shortlist_union_elapsed);
        p.final_shortlist_confidence = p
            .final_shortlist_confidence
            .saturating_add(host_finalize_shortlist_confidence_elapsed);
        p.final_shortlist_slice = p
            .final_shortlist_slice
            .saturating_add(host_finalize_shortlist_slice_elapsed);
        p.final_shortlist_slice_replay = p
            .final_shortlist_slice_replay
            .saturating_add(shortlist_slice_timing.replay);
        p.final_shortlist_slice_replay_descriptor_prep = p
            .final_shortlist_slice_replay_descriptor_prep
            .saturating_add(shortlist_slice_timing.replay_descriptor_prep);
        p.final_shortlist_slice_replay_descriptor_upload = p
            .final_shortlist_slice_replay_descriptor_upload
            .saturating_add(shortlist_slice_timing.replay_descriptor_upload);
        p.final_shortlist_slice_record_assembly = p
            .final_shortlist_slice_record_assembly
            .saturating_add(shortlist_slice_timing.record_assembly);
        p.final_shortlist_slice_sort = p
            .final_shortlist_slice_sort
            .saturating_add(shortlist_slice_timing.sort);
        p.final_shortlist_slice_diagnostics = p
            .final_shortlist_slice_diagnostics
            .saturating_add(shortlist_slice_timing.diagnostics);
        p.final_export_flush = p
            .final_export_flush
            .saturating_add(host_finalize_export_flush_elapsed);
        p.final_cleanup = p
            .final_cleanup
            .saturating_add(host_finalize_cleanup_elapsed);
        p.emit(
            fast_ma.as_str(),
            slow_ma.as_str(),
            device_include_all,
            topk_select,
            heatmap_bins,
        );
    }

    Ok((
        result,
        KernelExecutorTimingMs {
            runtime_init_ms: 0,
            set_current_ms: 0,
            gpu_indicator_prep_ms: gpu_indicator_prep_elapsed.as_millis() as u64,
            h2d_upload_ms: h2d_upload_elapsed.as_millis() as u64,
            launch_submit_us: launch_submit_elapsed.as_micros() as u64,
            d2h_copy_ms: d2h_copy_elapsed.as_millis() as u64,
            host_decode_ms: host_decode_elapsed.as_millis() as u64,
            explicit_sync_ms: explicit_sync_elapsed.as_millis() as u64,
            host_shortlist_materialize_ms: host_shortlist_materialize_elapsed.as_millis() as u64,
            host_finalize_ms: host_finalize_elapsed.as_millis() as u64,
            host_finalize_agg_finalize_ms: host_finalize_agg_finalize_elapsed.as_millis() as u64,
            host_finalize_finalist_recompute_ms: host_finalize_finalist_recompute_elapsed
                .as_millis() as u64,
            host_finalize_shortlist_union_ms: host_finalize_shortlist_union_elapsed.as_millis()
                as u64,
            host_finalize_shortlist_confidence_ms: host_finalize_shortlist_confidence_elapsed
                .as_millis() as u64,
            host_finalize_shortlist_slice_ms: host_finalize_shortlist_slice_elapsed.as_millis()
                as u64,
            host_finalize_shortlist_slice_replay_ms: shortlist_slice_timing.replay.as_millis()
                as u64,
            host_finalize_shortlist_slice_replay_descriptor_prep_ms: shortlist_slice_timing
                .replay_descriptor_prep
                .as_millis()
                as u64,
            host_finalize_shortlist_slice_replay_descriptor_upload_ms: shortlist_slice_timing
                .replay_descriptor_upload
                .as_millis()
                as u64,
            host_finalize_shortlist_slice_record_assembly_ms: shortlist_slice_timing
                .record_assembly
                .as_millis() as u64,
            host_finalize_shortlist_slice_sort_ms: shortlist_slice_timing.sort.as_millis() as u64,
            host_finalize_shortlist_slice_diagnostics_ms: shortlist_slice_timing
                .diagnostics
                .as_millis() as u64,
            host_finalize_export_flush_ms: host_finalize_export_flush_elapsed.as_millis() as u64,
            host_finalize_cleanup_ms: host_finalize_cleanup_elapsed.as_millis() as u64,
            graph_capture_ms: graph_capture_elapsed.as_millis() as u64,
            graph_replay_count,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        build_device_constraint_args, build_valid_pair_index_map_i32,
        build_valid_pair_local_rows_i32, calmar_ratio,
        can_decode_current_gpu_product_metrics_without_recompute,
        choose_fixed_contract_dispatch_kind, choose_fixed_contract_kernel_flavor,
        compute_shortlist_slice_benchmark_relative_summary, decode_tm_flex_metrics_row,
        effective_metrics_stride, effective_sample_lcb_10_pair_score,
        eval_double_ma_dynamic_program_precomputed_gpu_kernel, exact_pruning_blockers,
        exact_pruning_max_log_dd_threshold, finalist_recompute_eval_range,
        fixed_metrics_contract_rows, install_effective_sample_score_context,
        kernel_async_alloc_enabled, kernel_effective_pair_bars, kernel_exact_pruning_enabled,
        kernel_extra_shortlist_objectives, kernel_pinned_d2h_enabled, kernel_pinned_h2d_enabled,
        kernel_row_score_heads_enabled, kernel_score_first_topk_enabled,
        kernel_shortlist_confidence_mode, kernel_shortlist_slice_benchmark,
        kernel_shortlist_slice_diagnostic_row_limit, kernel_shortlist_slice_mode,
        kernel_shortlist_union_mode, kernel_should_use_lockstep_time,
        kernel_should_use_m7_specialize, kernel_strategy_params,
        max_drawdown_constraint_to_log_space, max_drawdown_constraint_to_search_space,
        normalize_topk_select_for_device, rerank_shortlist_by_confidence,
        rerank_shortlist_by_union_mode, row_score_heads_blockers, row_score_mode_objective_index,
        score_first_topk_blockers, search_metrics_count_for_optimization,
        search_metrics_count_for_optimization_with_contract, search_rows_use_log_space,
        shortlist_candidate_track_record_bars, shortlist_slice_ranges,
        should_force_low_pair_m7_recompute, should_use_compact_valid_pair_output,
        should_use_exact_pruning_mode, should_use_row_score_heads_mode,
        should_use_score_first_topk_mode, upload_double_ma_dynamic_program_batch,
        validate_uploaded_double_ma_dynamic_program_batch_geometry, EffectiveSampleScoreContext,
        FixedContractDispatchKind, FixedContractKernelFlavor, ShortlistConfidenceMode,
        ShortlistSliceMode, METRICS_COUNT, PRODUCT_REDUCED_METRICS_COUNT, ROW_SCORE_OBJECTIVE_BASE,
        STRAT_SIGNED_EXPOSURE, TOPK_BLOCK_THREADS, TRADE_METRICS_COUNT,
    };
    use crate::double_ma::{
        effective_sample_lcb_10_score, objective_score, DoubleMaConstraints,
        DoubleMaMetricContract, DoubleMaScoreMode, DoubleMaShortlistConfidenceMode,
        DoubleMaShortlistSliceMode, DoubleMaShortlistUnionMode,
    };
    use crate::strategy_dynamic::{
        build_dynamic_cpu_context_from_precomputed, compile_dynamic_plan_for_double_ma_config,
        eval_dynamic_plan_metrics_cpu,
    };
    use crate::strategy_gpu_program::{
        compile_gpu_predicate_program, compile_gpu_predicate_program_abi,
        CompiledGpuPredicateProgramAbi, GpuConditionAbi, GpuConditionGroupAbi, GpuIndicatorSlotAbi,
        GpuSignalBindingsAbi, GpuValueOperandAbi,
    };
    use crate::strategy_gpu_runtime::{
        reset_gpu_strategy_executable_cache, StrategyCapabilityReport, StrategyHandle,
    };
    use crate::strategy_ir::{
        ConditionGroupMode, ConditionOp, IndicatorKind, StrategyProfileKind,
        STRATEGY_IR_SCHEMA_VERSION,
    };
    use crate::strategy_plan::{
        CompiledConditionGroup, CompiledConditionOp, CompiledSignalBindings, CompiledStrategyPlan,
        CompiledValueRef, IndicatorComputePlan, IndicatorRouteKind, StrategyPlanBackend,
        STRATEGY_PLAN_SCHEMA_VERSION,
    };
    use crate::validation_shared::SpaBenchmarkKind;
    use cust::memory::{CopyDestination, DeviceBuffer};
    use std::ffi::c_void;
    use ta_optimizer::ObjectiveKind;
    use ta_strategies::double_ma::Metrics;
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

    fn make_oscillating_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        let step = std::f64::consts::TAU / 32.0;
        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64 * 0.005;
            let wave = (i as f64 * step).sin() * 3.0;
            let c = base + wave;
            open.push(c - 0.15);
            high.push(c + 0.75);
            low.push(c - 0.75);
            close.push(c);
            vol.push(8_000.0 + ((i % 29) as f64) * 25.0);
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn metric_row17_for_selector(
        pnl: f32,
        trades: f32,
        max_dd: f32,
        mean: f32,
        std: f32,
        downside_deviation: f32,
        omega: f32,
    ) -> [f32; METRICS_COUNT] {
        let mut row = [0.0f32; METRICS_COUNT];
        row[0] = pnl;
        row[1] = trades;
        row[2] = max_dd;
        row[3] = mean;
        row[4] = std;
        row[5] = 0.50;
        row[6] = 0.10;
        row[7] = downside_deviation;
        row[8] = 0.02;
        row[9] = omega;
        row[10] = 0.20;
        row[11] = 0.05;
        row[12] = 0.03;
        row[13] = -0.01;
        row[14] = 1.10;
        row[15] = 0.25;
        row[16] = 4.0;
        row
    }

    fn poison_objective_field(row: &mut [f32; METRICS_COUNT], objective: i32) {
        match objective {
            0 => row[0] = f32::NAN,
            1 => row[3] = f32::NAN,
            2 => row[2] = f32::NAN,
            3 => row[0] = f32::NAN,
            4 => row[4] = f32::NAN,
            5 => row[7] = f32::NAN,
            6 => row[9] = f32::NAN,
            _ => row[0] = f32::NAN,
        }
    }

    fn flatten_metric_rows(rows: &[[f32; METRICS_COUNT]]) -> Vec<f32> {
        rows.iter().flat_map(|row| row.iter().copied()).collect()
    }

    fn metric_row_objective_score_for_selector_reference(
        row: &[f32; METRICS_COUNT],
        objective: i32,
    ) -> f32 {
        const CLAMP_MAX: f32 = 1.0e6;
        let score = match objective {
            0 => row[0],
            1 => {
                let mean = row[3];
                let std = row[4];
                if mean.is_finite() && std.is_finite() {
                    if std > 0.0 {
                        mean / std
                    } else {
                        0.0
                    }
                } else {
                    f32::NEG_INFINITY
                }
            }
            2 => -row[2],
            3 => {
                let pnl = row[0];
                let max_dd = row[2];
                if pnl.is_finite() && max_dd.is_finite() {
                    if max_dd > 1e-12 {
                        (pnl / max_dd).clamp(-CLAMP_MAX, CLAMP_MAX)
                    } else if pnl > 0.0 {
                        CLAMP_MAX
                    } else if pnl < 0.0 {
                        -CLAMP_MAX
                    } else {
                        0.0
                    }
                } else {
                    f32::NEG_INFINITY
                }
            }
            4 => -row[4],
            5 => {
                let mean = row[3];
                let downside = row[7];
                if mean.is_finite() && downside.is_finite() {
                    if downside > 0.0 {
                        mean / downside
                    } else if mean > 0.0 {
                        CLAMP_MAX
                    } else if mean < 0.0 {
                        -CLAMP_MAX
                    } else {
                        0.0
                    }
                } else {
                    f32::NEG_INFINITY
                }
            }
            6 => row[9],
            _ => f32::NEG_INFINITY,
        };

        if score.is_finite() {
            score
        } else {
            f32::NEG_INFINITY
        }
    }

    fn canonical_metric_row_topk_reference(
        rows: &[[f32; METRICS_COUNT]],
        candidates: Option<&[i32]>,
        objective: i32,
        top_k: usize,
    ) -> Vec<i32> {
        let top_k = top_k.max(1);
        let mut scored = Vec::new();

        match candidates {
            Some(candidates) => {
                for &candidate in candidates {
                    let Ok(row_idx) = usize::try_from(candidate) else {
                        continue;
                    };
                    if row_idx >= rows.len() {
                        continue;
                    }
                    let score = metric_row_objective_score_for_selector_reference(
                        &rows[row_idx],
                        objective,
                    );
                    if score.is_finite() {
                        scored.push((row_idx as i32, score));
                    }
                }
            }
            None => {
                for (row_idx, row) in rows.iter().enumerate() {
                    let score = metric_row_objective_score_for_selector_reference(row, objective);
                    if score.is_finite() {
                        scored.push((row_idx as i32, score));
                    }
                }
            }
        }

        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut out: Vec<i32> = scored
            .into_iter()
            .take(top_k)
            .map(|(row_idx, _)| row_idx)
            .collect();
        out.resize(top_k, -1);
        out
    }

    fn canonical_scored_row_topk_reference(scores: &[f32], top_k: usize) -> Vec<i32> {
        let top_k = top_k.max(1);
        let mut scored: Vec<(i32, f32)> = scores
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(row_idx, score)| score.is_finite().then_some((row_idx as i32, score)))
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut out: Vec<i32> = scored
            .into_iter()
            .take(top_k)
            .map(|(row_idx, _)| row_idx)
            .collect();
        out.resize(top_k, -1);
        out
    }

    fn assert_only_valid_row_selected(label: &str, selected: &[i32]) {
        assert_eq!(
            selected.first().copied(),
            Some(0),
            "{label}: valid row should rank first, selected={selected:?}"
        );
        assert!(
            selected.iter().skip(1).all(|&idx| idx == -1),
            "{label}: unused or invalid slots must stay -1, selected={selected:?}"
        );
    }

    fn launch_metric_row_selector_v2_for_test(
        rt: &super::CudaKernelRuntime,
        metric_rows: &[f32],
        row_count: usize,
        objective: i32,
        top_k: usize,
        constraints: &DoubleMaConstraints,
    ) -> Result<Vec<i32>, String> {
        let selector = rt
            .module
            .get_function("select_topk_metric_rows_f32_v2")
            .map_err(|e| e.to_string())?;
        let d_rows = DeviceBuffer::from_slice(metric_rows).map_err(|e| e.to_string())?;
        let out_init = vec![123_i32; top_k];
        let d_out = DeviceBuffer::from_slice(&out_init).map_err(|e| e.to_string())?;
        let constraint_args = build_device_constraint_args(Some(constraints), METRICS_COUNT);

        unsafe {
            let mut rows_ptr = d_rows.as_device_ptr().as_raw();
            let mut rows_i = row_count as i32;
            let mut m_i = METRICS_COUNT as i32;
            let mut stride_i = METRICS_COUNT as i32;
            let mut objective_i = objective;
            let mut hurdle_tau = 0.0f32;
            let mut topk_i = top_k as i32;
            let mut use_min_trades = constraint_args.use_min_trades;
            let mut min_trades = constraint_args.min_trades;
            let mut use_max_dd = constraint_args.use_max_dd;
            let mut max_dd = constraint_args.max_dd;
            let mut use_min_sharpe = constraint_args.use_min_sharpe;
            let mut min_sharpe = constraint_args.min_sharpe;
            let mut use_min_sortino = constraint_args.use_min_sortino;
            let mut min_sortino = constraint_args.min_sortino;
            let mut use_min_calmar = constraint_args.use_min_calmar;
            let mut min_calmar = constraint_args.min_calmar;
            let mut use_max_volatility = constraint_args.use_max_volatility;
            let mut max_volatility = constraint_args.max_volatility;
            let mut use_min_exposure = constraint_args.use_min_exposure;
            let mut min_exposure = constraint_args.min_exposure;
            let mut use_max_exposure = constraint_args.use_max_exposure;
            let mut max_exposure = constraint_args.max_exposure;
            let mut use_max_abs_net_exposure = constraint_args.use_max_abs_net_exposure;
            let mut max_abs_net_exposure = constraint_args.max_abs_net_exposure;
            let mut use_min_omega = constraint_args.use_min_omega;
            let mut min_omega = constraint_args.min_omega;
            let mut use_max_ulcer = constraint_args.use_max_ulcer;
            let mut max_ulcer = constraint_args.max_ulcer;
            let mut use_max_downside_deviation = constraint_args.use_max_downside_deviation;
            let mut max_downside_deviation = constraint_args.max_downside_deviation;
            let mut out_ptr = d_out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut rows_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut stride_i as *mut _ as *mut c_void,
                &mut objective_i as *mut _ as *mut c_void,
                &mut hurdle_tau as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut use_min_trades as *mut _ as *mut c_void,
                &mut min_trades as *mut _ as *mut c_void,
                &mut use_max_dd as *mut _ as *mut c_void,
                &mut max_dd as *mut _ as *mut c_void,
                &mut use_min_sharpe as *mut _ as *mut c_void,
                &mut min_sharpe as *mut _ as *mut c_void,
                &mut use_min_sortino as *mut _ as *mut c_void,
                &mut min_sortino as *mut _ as *mut c_void,
                &mut use_min_calmar as *mut _ as *mut c_void,
                &mut min_calmar as *mut _ as *mut c_void,
                &mut use_max_volatility as *mut _ as *mut c_void,
                &mut max_volatility as *mut _ as *mut c_void,
                &mut use_min_exposure as *mut _ as *mut c_void,
                &mut min_exposure as *mut _ as *mut c_void,
                &mut use_max_exposure as *mut _ as *mut c_void,
                &mut max_exposure as *mut _ as *mut c_void,
                &mut use_max_abs_net_exposure as *mut _ as *mut c_void,
                &mut max_abs_net_exposure as *mut _ as *mut c_void,
                &mut use_min_omega as *mut _ as *mut c_void,
                &mut min_omega as *mut _ as *mut c_void,
                &mut use_max_ulcer as *mut _ as *mut c_void,
                &mut max_ulcer as *mut _ as *mut c_void,
                &mut use_max_downside_deviation as *mut _ as *mut c_void,
                &mut max_downside_deviation as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&selector, (1, 1, 1), (TOPK_BLOCK_THREADS, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut out = out_init;
        d_out.copy_to(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn launch_metric_row_candidate_selector_v2_for_test(
        rt: &super::CudaKernelRuntime,
        metric_rows: &[f32],
        row_count: usize,
        candidates: &[i32],
        objective: i32,
        top_k: usize,
        constraints: &DoubleMaConstraints,
    ) -> Result<Vec<i32>, String> {
        let selector = rt
            .module
            .get_function("select_topk_metric_rows_from_candidates_f32_v2")
            .map_err(|e| e.to_string())?;
        let d_rows = DeviceBuffer::from_slice(metric_rows).map_err(|e| e.to_string())?;
        let d_candidates = DeviceBuffer::from_slice(candidates).map_err(|e| e.to_string())?;
        let out_init = vec![123_i32; top_k];
        let d_out = DeviceBuffer::from_slice(&out_init).map_err(|e| e.to_string())?;
        let constraint_args = build_device_constraint_args(Some(constraints), METRICS_COUNT);

        unsafe {
            let mut rows_ptr = d_rows.as_device_ptr().as_raw();
            let mut rows_i = row_count as i32;
            let mut m_i = METRICS_COUNT as i32;
            let mut stride_i = METRICS_COUNT as i32;
            let mut objective_i = objective;
            let mut hurdle_tau = 0.0f32;
            let mut topk_i = top_k as i32;
            let mut candidates_ptr = d_candidates.as_device_ptr().as_raw();
            let mut candidates_len_i = candidates.len() as i32;
            let mut use_min_trades = constraint_args.use_min_trades;
            let mut min_trades = constraint_args.min_trades;
            let mut use_max_dd = constraint_args.use_max_dd;
            let mut max_dd = constraint_args.max_dd;
            let mut use_min_sharpe = constraint_args.use_min_sharpe;
            let mut min_sharpe = constraint_args.min_sharpe;
            let mut use_min_sortino = constraint_args.use_min_sortino;
            let mut min_sortino = constraint_args.min_sortino;
            let mut use_min_calmar = constraint_args.use_min_calmar;
            let mut min_calmar = constraint_args.min_calmar;
            let mut use_max_volatility = constraint_args.use_max_volatility;
            let mut max_volatility = constraint_args.max_volatility;
            let mut use_min_exposure = constraint_args.use_min_exposure;
            let mut min_exposure = constraint_args.min_exposure;
            let mut use_max_exposure = constraint_args.use_max_exposure;
            let mut max_exposure = constraint_args.max_exposure;
            let mut use_max_abs_net_exposure = constraint_args.use_max_abs_net_exposure;
            let mut max_abs_net_exposure = constraint_args.max_abs_net_exposure;
            let mut use_min_omega = constraint_args.use_min_omega;
            let mut min_omega = constraint_args.min_omega;
            let mut use_max_ulcer = constraint_args.use_max_ulcer;
            let mut max_ulcer = constraint_args.max_ulcer;
            let mut use_max_downside_deviation = constraint_args.use_max_downside_deviation;
            let mut max_downside_deviation = constraint_args.max_downside_deviation;
            let mut out_ptr = d_out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut rows_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut stride_i as *mut _ as *mut c_void,
                &mut objective_i as *mut _ as *mut c_void,
                &mut hurdle_tau as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut candidates_ptr as *mut _ as *mut c_void,
                &mut candidates_len_i as *mut _ as *mut c_void,
                &mut use_min_trades as *mut _ as *mut c_void,
                &mut min_trades as *mut _ as *mut c_void,
                &mut use_max_dd as *mut _ as *mut c_void,
                &mut max_dd as *mut _ as *mut c_void,
                &mut use_min_sharpe as *mut _ as *mut c_void,
                &mut min_sharpe as *mut _ as *mut c_void,
                &mut use_min_sortino as *mut _ as *mut c_void,
                &mut min_sortino as *mut _ as *mut c_void,
                &mut use_min_calmar as *mut _ as *mut c_void,
                &mut min_calmar as *mut _ as *mut c_void,
                &mut use_max_volatility as *mut _ as *mut c_void,
                &mut max_volatility as *mut _ as *mut c_void,
                &mut use_min_exposure as *mut _ as *mut c_void,
                &mut min_exposure as *mut _ as *mut c_void,
                &mut use_max_exposure as *mut _ as *mut c_void,
                &mut max_exposure as *mut _ as *mut c_void,
                &mut use_max_abs_net_exposure as *mut _ as *mut c_void,
                &mut max_abs_net_exposure as *mut _ as *mut c_void,
                &mut use_min_omega as *mut _ as *mut c_void,
                &mut min_omega as *mut _ as *mut c_void,
                &mut use_max_ulcer as *mut _ as *mut c_void,
                &mut max_ulcer as *mut _ as *mut c_void,
                &mut use_max_downside_deviation as *mut _ as *mut c_void,
                &mut max_downside_deviation as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&selector, (1, 1, 1), (TOPK_BLOCK_THREADS, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut out = out_init;
        d_out.copy_to(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn launch_scored_row_selector_for_test(
        rt: &super::CudaKernelRuntime,
        scores: &[f32],
        top_k: usize,
    ) -> Result<Vec<i32>, String> {
        let selector = rt
            .module
            .get_function("select_topk_scored_rows_f32")
            .map_err(|e| e.to_string())?;
        let d_scores = DeviceBuffer::from_slice(scores).map_err(|e| e.to_string())?;
        let out_init = vec![123_i32; top_k];
        let d_out = DeviceBuffer::from_slice(&out_init).map_err(|e| e.to_string())?;

        unsafe {
            let mut scores_ptr = d_scores.as_device_ptr().as_raw();
            let mut rows_i = scores.len() as i32;
            let mut topk_i = top_k as i32;
            let mut out_ptr = d_out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut scores_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&selector, (1, 1, 1), (TOPK_BLOCK_THREADS, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut out = out_init;
        d_out.copy_to(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn launch_gather_metric_rows_for_test(
        rt: &super::CudaKernelRuntime,
        metric_row: &[f32; METRICS_COUNT],
        top_rows: &[i32],
    ) -> Result<(Vec<i32>, Vec<i32>, Vec<f32>), String> {
        let gather = rt
            .module
            .get_function("gather_topk_metric_rows_f32")
            .map_err(|e| e.to_string())?;
        let d_metrics = DeviceBuffer::from_slice(metric_row).map_err(|e| e.to_string())?;
        let d_fast = DeviceBuffer::from_slice(&[5_i32]).map_err(|e| e.to_string())?;
        let d_slow = DeviceBuffer::from_slice(&[20_i32]).map_err(|e| e.to_string())?;
        let d_top = DeviceBuffer::from_slice(top_rows).map_err(|e| e.to_string())?;
        let fast_init = vec![123_i32; top_rows.len()];
        let slow_init = vec![123_i32; top_rows.len()];
        let metrics_init = vec![123.0f32; top_rows.len() * METRICS_COUNT];
        let d_out_fast = DeviceBuffer::from_slice(&fast_init).map_err(|e| e.to_string())?;
        let d_out_slow = DeviceBuffer::from_slice(&slow_init).map_err(|e| e.to_string())?;
        let d_out_metrics = DeviceBuffer::from_slice(&metrics_init).map_err(|e| e.to_string())?;

        unsafe {
            let mut metrics_ptr = d_metrics.as_device_ptr().as_raw();
            let mut rows_i = 1_i32;
            let mut m_i = METRICS_COUNT as i32;
            let mut stride_i = METRICS_COUNT as i32;
            let mut fast_ptr = d_fast.as_device_ptr().as_raw();
            let mut slow_ptr = d_slow.as_device_ptr().as_raw();
            let mut top_ptr = d_top.as_device_ptr().as_raw();
            let mut topk_i = top_rows.len() as i32;
            let mut out_fast_ptr = d_out_fast.as_device_ptr().as_raw();
            let mut out_slow_ptr = d_out_slow.as_device_ptr().as_raw();
            let mut out_metrics_ptr = d_out_metrics.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut metrics_ptr as *mut _ as *mut c_void,
                &mut rows_i as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut stride_i as *mut _ as *mut c_void,
                &mut fast_ptr as *mut _ as *mut c_void,
                &mut slow_ptr as *mut _ as *mut c_void,
                &mut top_ptr as *mut _ as *mut c_void,
                &mut topk_i as *mut _ as *mut c_void,
                &mut out_fast_ptr as *mut _ as *mut c_void,
                &mut out_slow_ptr as *mut _ as *mut c_void,
                &mut out_metrics_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&gather, (1, 1, 1), (256, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut out_fast = fast_init;
        let mut out_slow = slow_init;
        let mut out_metrics = metrics_init;
        d_out_fast
            .copy_to(&mut out_fast)
            .map_err(|e| e.to_string())?;
        d_out_slow
            .copy_to(&mut out_slow)
            .map_err(|e| e.to_string())?;
        d_out_metrics
            .copy_to(&mut out_metrics)
            .map_err(|e| e.to_string())?;
        Ok((out_fast, out_slow, out_metrics))
    }

    fn launch_flex_m7_case_for_test(
        rt: &super::CudaKernelRuntime,
        commission: f32,
        include_ret1: bool,
        fast: &[f32],
        slow: &[f32],
        lr: &[f32],
    ) -> Result<Vec<f32>, String> {
        let func = rt
            .module
            .get_function("double_cross_backtest_tm_flex_f32_m7")
            .map_err(|e| e.to_string())?;
        let t_len = lr.len();
        if t_len == 0 || fast.len() != t_len || slow.len() != t_len {
            return Err("flex M7 test inputs must be non-empty and equal length".to_string());
        }
        let ret1: Vec<f32> = lr.iter().map(|&v| v.exp_m1()).collect();

        let d_fast = DeviceBuffer::from_slice(fast).map_err(|e| e.to_string())?;
        let d_slow = DeviceBuffer::from_slice(slow).map_err(|e| e.to_string())?;
        let d_lr = DeviceBuffer::from_slice(lr).map_err(|e| e.to_string())?;
        let d_ret1 = if include_ret1 {
            Some(DeviceBuffer::from_slice(&ret1).map_err(|e| e.to_string())?)
        } else {
            None
        };
        let d_fast_periods = DeviceBuffer::from_slice(&[2_i32]).map_err(|e| e.to_string())?;
        let d_slow_periods = DeviceBuffer::from_slice(&[4_i32]).map_err(|e| e.to_string())?;
        let d_pair_map = DeviceBuffer::from_slice(&[0_i32]).map_err(|e| e.to_string())?;
        let out_init = vec![123.0f32; 7];
        let d_out = DeviceBuffer::from_slice(&out_init).map_err(|e| e.to_string())?;

        unsafe {
            let mut fast_ptr = d_fast.as_device_ptr().as_raw();
            let mut slow_ptr = d_slow.as_device_ptr().as_raw();
            let mut lr_ptr = d_lr.as_device_ptr().as_raw();
            let mut ret1_ptr = d_ret1
                .as_ref()
                .map(|buf| buf.as_device_ptr().as_raw())
                .unwrap_or(0);
            let mut t_i = t_len as i32;
            let mut pf_tile_i = 1_i32;
            let mut ps_tile_i = 1_i32;
            let mut fast_stride_i = 1_i32;
            let mut slow_stride_i = 1_i32;
            let mut pf_total_i = 1_i32;
            let mut ps_total_i = 1_i32;
            let mut f_offset_i = 0_i32;
            let mut s_offset_i = 0_i32;
            let mut fast_periods_ptr = d_fast_periods.as_device_ptr().as_raw();
            let mut slow_periods_ptr = d_slow_periods.as_device_ptr().as_raw();
            let mut regime_ptr = 0_u64;
            let mut use_regime_i = 0_i32;
            let mut use_regime_hybrid_i = 0_i32;
            let mut regime_entry = 0.0f32;
            let mut regime_exit = 0.0f32;
            let mut first_valid_i = 0_i32;
            let mut regime_first_valid_i = 0_i32;
            let mut commission_i = commission;
            let mut eps_rel = 0.0f32;
            let mut hurdle_tau = 0.0f32;
            let mut stop_log = 0.0f32;
            let mut take_log = 0.0f32;
            let mut flags = 0_u32;
            let mut m_i = 7_i32;
            let mut metrics_stride_i = 7_i32;
            let mut pair_map_ptr = d_pair_map.as_device_ptr().as_raw();
            let mut compact_fast_periods_ptr = 0_u64;
            let mut compact_slow_periods_ptr = 0_u64;
            let mut compact_fast_local_rows_ptr = 0_u64;
            let mut compact_slow_local_rows_ptr = 0_u64;
            let mut pair_count_i = 1_i32;
            let mut compact_output_i = 0_i32;
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
                &mut f_offset_i as *mut _ as *mut c_void,
                &mut s_offset_i as *mut _ as *mut c_void,
                &mut fast_periods_ptr as *mut _ as *mut c_void,
                &mut slow_periods_ptr as *mut _ as *mut c_void,
                &mut regime_ptr as *mut _ as *mut c_void,
                &mut use_regime_i as *mut _ as *mut c_void,
                &mut use_regime_hybrid_i as *mut _ as *mut c_void,
                &mut regime_entry as *mut _ as *mut c_void,
                &mut regime_exit as *mut _ as *mut c_void,
                &mut first_valid_i as *mut _ as *mut c_void,
                &mut regime_first_valid_i as *mut _ as *mut c_void,
                &mut commission_i as *mut _ as *mut c_void,
                &mut eps_rel as *mut _ as *mut c_void,
                &mut hurdle_tau as *mut _ as *mut c_void,
                &mut stop_log as *mut _ as *mut c_void,
                &mut take_log as *mut _ as *mut c_void,
                &mut flags as *mut _ as *mut c_void,
                &mut m_i as *mut _ as *mut c_void,
                &mut metrics_stride_i as *mut _ as *mut c_void,
                &mut pair_map_ptr as *mut _ as *mut c_void,
                &mut compact_fast_periods_ptr as *mut _ as *mut c_void,
                &mut compact_slow_periods_ptr as *mut _ as *mut c_void,
                &mut compact_fast_local_rows_ptr as *mut _ as *mut c_void,
                &mut compact_slow_local_rows_ptr as *mut _ as *mut c_void,
                &mut pair_count_i as *mut _ as *mut c_void,
                &mut compact_output_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&func, (1, 1, 1), (128, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut out = out_init;
        d_out.copy_to(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn launch_flex_m7_numeric_case_for_test(
        rt: &super::CudaKernelRuntime,
        commission: f32,
        include_ret1: bool,
    ) -> Result<Vec<f32>, String> {
        let t_len = 64usize;
        let mut fast: Vec<f32> = Vec::with_capacity(t_len);
        let mut slow: Vec<f32> = Vec::with_capacity(t_len);
        let mut lr: Vec<f32> = Vec::with_capacity(t_len);
        for t in 0..t_len {
            let high_fast = (8..24).contains(&t) || (40..56).contains(&t);
            fast.push(if high_fast { 2.0 } else { 0.0 });
            slow.push(1.0);
            let wave = match t % 5 {
                0 => 0.018,
                1 => -0.006,
                2 => 0.011,
                3 => -0.004,
                _ => 0.007,
            };
            lr.push(wave);
        }
        launch_flex_m7_case_for_test(rt, commission, include_ret1, &fast, &slow, &lr)
    }

    fn launch_flex_m7_log_equity_case_for_test(
        rt: &super::CudaKernelRuntime,
        step_log_return: f32,
    ) -> Result<Vec<f32>, String> {
        let t_len = 64usize;
        let fast = vec![2.0f32; t_len];
        let slow = vec![1.0f32; t_len];
        let lr = vec![step_log_return; t_len];
        launch_flex_m7_case_for_test(rt, 0.0, false, &fast, &slow, &lr)
    }

    fn launch_strategy_program_tm_m7_negative_count_case_for_test(
        rt: &super::CudaKernelRuntime,
    ) -> Result<Vec<f32>, String> {
        let func = rt
            .module
            .get_function("strategy_program_double_ma_backtest_tm_f32_m7")
            .map_err(|e| e.to_string())?;
        let t_len = 16usize;
        let lr = vec![0.0f32; t_len];
        let fast = vec![1.0f32; t_len];
        let slow = vec![1.0f32; t_len];
        let combo_row = [0_i32];
        let combo_start = [1_i32];
        let combo_end = [(t_len - 1) as i32];
        let indicator_slots = [GpuIndicatorSlotAbi {
            indicator_index: 0,
            route: 1,
        }];
        let condition = GpuConditionAbi {
            op: 0,
            lhs: GpuValueOperandAbi {
                source: 0,
                index: 0,
                constant_bits: 0,
            },
            rhs: GpuValueOperandAbi {
                source: 3,
                index: 0,
                constant_bits: 1.0f32.to_bits() as u64,
            },
        };
        let groups = [GpuConditionGroupAbi {
            mode: 0,
            condition_offset: 0,
            condition_count: 1,
        }];
        let group_indexes = [0_u32];
        let signals = [GpuSignalBindingsAbi {
            long_entry_group: 0,
            long_exit_group: 0,
            short_entry_group: 0,
            short_exit_group: 0,
        }];

        let d_lr = DeviceBuffer::from_slice(&lr).map_err(|e| e.to_string())?;
        let d_fast = DeviceBuffer::from_slice(&fast).map_err(|e| e.to_string())?;
        let d_slow = DeviceBuffer::from_slice(&slow).map_err(|e| e.to_string())?;
        let d_combo_fast = DeviceBuffer::from_slice(&combo_row).map_err(|e| e.to_string())?;
        let d_combo_slow = DeviceBuffer::from_slice(&combo_row).map_err(|e| e.to_string())?;
        let d_combo_start = DeviceBuffer::from_slice(&combo_start).map_err(|e| e.to_string())?;
        let d_combo_end = DeviceBuffer::from_slice(&combo_end).map_err(|e| e.to_string())?;
        let d_indicator_slots =
            DeviceBuffer::from_slice(&indicator_slots).map_err(|e| e.to_string())?;
        let d_conditions = DeviceBuffer::from_slice(&[condition]).map_err(|e| e.to_string())?;
        let d_groups = DeviceBuffer::from_slice(&groups).map_err(|e| e.to_string())?;
        let d_group_indexes =
            DeviceBuffer::from_slice(&group_indexes).map_err(|e| e.to_string())?;
        let d_signals = DeviceBuffer::from_slice(&signals).map_err(|e| e.to_string())?;
        let out_init = vec![123.0f32; 7];
        let d_out = DeviceBuffer::from_slice(&out_init).map_err(|e| e.to_string())?;

        unsafe {
            let mut lr_ptr = d_lr.as_device_ptr().as_raw();
            let mut t_i = t_len as i32;
            let mut fast_ptr = d_fast.as_device_ptr().as_raw();
            let mut fast_stride_i = 1_i32;
            let mut slow_ptr = d_slow.as_device_ptr().as_raw();
            let mut slow_stride_i = 1_i32;
            let mut combo_fast_ptr = d_combo_fast.as_device_ptr().as_raw();
            let mut combo_slow_ptr = d_combo_slow.as_device_ptr().as_raw();
            let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
            let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
            let mut combo_count_i = 1_i32;
            let mut indicator_slots_ptr = d_indicator_slots.as_device_ptr().as_raw();
            let mut indicator_slot_count_i = -1_i32;
            let mut conditions_ptr = d_conditions.as_device_ptr().as_raw();
            let mut condition_count_i = -1_i32;
            let mut groups_ptr = d_groups.as_device_ptr().as_raw();
            let mut group_count_i = -1_i32;
            let mut group_indexes_ptr = d_group_indexes.as_device_ptr().as_raw();
            let mut group_index_count_i = -1_i32;
            let mut signals_ptr = d_signals.as_device_ptr().as_raw();
            let mut flags = 0_u32;
            let mut commission = 0.0f32;
            let mut eps_rel = 0.0f32;
            let mut stop_log = 0.0f32;
            let mut take_log = 0.0f32;
            let mut out_ptr = d_out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut fast_ptr as *mut _ as *mut c_void,
                &mut fast_stride_i as *mut _ as *mut c_void,
                &mut slow_ptr as *mut _ as *mut c_void,
                &mut slow_stride_i as *mut _ as *mut c_void,
                &mut combo_fast_ptr as *mut _ as *mut c_void,
                &mut combo_slow_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut indicator_slots_ptr as *mut _ as *mut c_void,
                &mut indicator_slot_count_i as *mut _ as *mut c_void,
                &mut conditions_ptr as *mut _ as *mut c_void,
                &mut condition_count_i as *mut _ as *mut c_void,
                &mut groups_ptr as *mut _ as *mut c_void,
                &mut group_count_i as *mut _ as *mut c_void,
                &mut group_indexes_ptr as *mut _ as *mut c_void,
                &mut group_index_count_i as *mut _ as *mut c_void,
                &mut signals_ptr as *mut _ as *mut c_void,
                &mut flags as *mut _ as *mut c_void,
                &mut commission as *mut _ as *mut c_void,
                &mut eps_rel as *mut _ as *mut c_void,
                &mut stop_log as *mut _ as *mut c_void,
                &mut take_log as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&func, (1, 1, 1), (128, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut out = out_init;
        d_out.copy_to(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn make_threshold_strategy_plan(strategy: StrategyConfig) -> CompiledStrategyPlan {
        CompiledStrategyPlan {
            plan_schema_version: STRATEGY_PLAN_SCHEMA_VERSION,
            backend: StrategyPlanBackend::GpuUnified,
            profile: StrategyProfileKind::DoubleMa,
            ir_schema_version: STRATEGY_IR_SCHEMA_VERSION,
            ir_fingerprint_hex: "test-threshold-strategy-program".to_string(),
            indicator_plan: vec![
                IndicatorComputePlan {
                    indicator_id: "fast_ma".to_string(),
                    indicator_kind: Some(IndicatorKind::FastMa),
                    source_indicator_id: None,
                    output_id: None,
                    params: Vec::new(),
                    route: IndicatorRouteKind::GpuKernel,
                },
                IndicatorComputePlan {
                    indicator_id: "slow_ma".to_string(),
                    indicator_kind: Some(IndicatorKind::SlowMa),
                    source_indicator_id: None,
                    output_id: None,
                    params: Vec::new(),
                    route: IndicatorRouteKind::GpuKernel,
                },
            ],
            conditions: vec![
                CompiledConditionOp {
                    op: ConditionOp::Gt,
                    lhs: CompiledValueRef::IndicatorIndex { index: 0 },
                    rhs: CompiledValueRef::ConstantBits {
                        bits: 1.0f64.to_bits(),
                    },
                },
                CompiledConditionOp {
                    op: ConditionOp::Le,
                    lhs: CompiledValueRef::IndicatorIndex { index: 0 },
                    rhs: CompiledValueRef::ConstantBits {
                        bits: 1.0f64.to_bits(),
                    },
                },
            ],
            groups: vec![
                CompiledConditionGroup {
                    mode: ConditionGroupMode::All,
                    condition_indexes: vec![0],
                },
                CompiledConditionGroup {
                    mode: ConditionGroupMode::All,
                    condition_indexes: vec![1],
                },
            ],
            signals: CompiledSignalBindings {
                long_entry_group: Some(0),
                long_exit_group: Some(1),
                short_entry_group: None,
                short_exit_group: None,
            },
            strategy,
            halftrend_filter: None,
            halftrend: None,
            macz: None,
            ott: None,
            signal_mode: None,
        }
    }

    fn launch_strategy_program_row_major_threshold_case_for_test(
        rt: &super::CudaKernelRuntime,
        strategy: &StrategyConfig,
        abi: &CompiledGpuPredicateProgramAbi,
        lr: &[f32],
        fast: &[f32],
        slow: &[f32],
        start_t: i32,
        end_t: i32,
    ) -> Result<Vec<f32>, String> {
        let func = rt
            .module
            .get_function("strategy_program_double_ma_backtest_f32")
            .map_err(|e| e.to_string())?;
        assert_eq!(lr.len(), fast.len());
        assert_eq!(lr.len(), slow.len());

        let combo_row = [0_i32];
        let combo_start = [start_t];
        let combo_end = [end_t];
        let signals = [abi.signals];

        let d_lr = DeviceBuffer::from_slice(lr).map_err(|e| e.to_string())?;
        let d_fast = DeviceBuffer::from_slice(fast).map_err(|e| e.to_string())?;
        let d_slow = DeviceBuffer::from_slice(slow).map_err(|e| e.to_string())?;
        let d_combo_fast = DeviceBuffer::from_slice(&combo_row).map_err(|e| e.to_string())?;
        let d_combo_slow = DeviceBuffer::from_slice(&combo_row).map_err(|e| e.to_string())?;
        let d_combo_start = DeviceBuffer::from_slice(&combo_start).map_err(|e| e.to_string())?;
        let d_combo_end = DeviceBuffer::from_slice(&combo_end).map_err(|e| e.to_string())?;
        let d_indicator_slots =
            DeviceBuffer::from_slice(&abi.indicator_slots).map_err(|e| e.to_string())?;
        let d_conditions = DeviceBuffer::from_slice(&abi.conditions).map_err(|e| e.to_string())?;
        let d_groups = DeviceBuffer::from_slice(&abi.groups).map_err(|e| e.to_string())?;
        let d_group_indexes =
            DeviceBuffer::from_slice(&abi.group_condition_indexes).map_err(|e| e.to_string())?;
        let d_signals = DeviceBuffer::from_slice(&signals).map_err(|e| e.to_string())?;
        let out_init = vec![123.0f32; 10];
        let d_out = DeviceBuffer::from_slice(&out_init).map_err(|e| e.to_string())?;
        let params = kernel_strategy_params(strategy);

        unsafe {
            let mut lr_ptr = d_lr.as_device_ptr().as_raw();
            let mut t_i = lr.len() as i32;
            let mut fast_ptr = d_fast.as_device_ptr().as_raw();
            let mut fast_rows_i = 1_i32;
            let mut fast_stride_i = lr.len() as i32;
            let mut slow_ptr = d_slow.as_device_ptr().as_raw();
            let mut slow_rows_i = 1_i32;
            let mut slow_stride_i = lr.len() as i32;
            let mut combo_fast_ptr = d_combo_fast.as_device_ptr().as_raw();
            let mut combo_slow_ptr = d_combo_slow.as_device_ptr().as_raw();
            let mut combo_start_ptr = d_combo_start.as_device_ptr().as_raw();
            let mut combo_end_ptr = d_combo_end.as_device_ptr().as_raw();
            let mut combo_count_i = 1_i32;
            let mut indicator_slots_ptr = d_indicator_slots.as_device_ptr().as_raw();
            let mut indicator_slot_count_i = abi.indicator_slots.len() as i32;
            let mut conditions_ptr = d_conditions.as_device_ptr().as_raw();
            let mut condition_count_i = abi.conditions.len() as i32;
            let mut groups_ptr = d_groups.as_device_ptr().as_raw();
            let mut group_count_i = abi.groups.len() as i32;
            let mut group_indexes_ptr = d_group_indexes.as_device_ptr().as_raw();
            let mut group_index_count_i = abi.group_condition_indexes.len() as i32;
            let mut signals_ptr = d_signals.as_device_ptr().as_raw();
            let mut flags = params.flags;
            let mut commission = params.commission;
            let mut eps_rel = params.eps_rel;
            let mut stop_log = params.stop_log;
            let mut take_log = params.take_log;
            let mut out_stride_i = 10_i32;
            let mut out_ptr = d_out.as_device_ptr().as_raw();
            let args: &mut [*mut c_void] = &mut [
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut fast_ptr as *mut _ as *mut c_void,
                &mut fast_rows_i as *mut _ as *mut c_void,
                &mut fast_stride_i as *mut _ as *mut c_void,
                &mut slow_ptr as *mut _ as *mut c_void,
                &mut slow_rows_i as *mut _ as *mut c_void,
                &mut slow_stride_i as *mut _ as *mut c_void,
                &mut combo_fast_ptr as *mut _ as *mut c_void,
                &mut combo_slow_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut indicator_slots_ptr as *mut _ as *mut c_void,
                &mut indicator_slot_count_i as *mut _ as *mut c_void,
                &mut conditions_ptr as *mut _ as *mut c_void,
                &mut condition_count_i as *mut _ as *mut c_void,
                &mut groups_ptr as *mut _ as *mut c_void,
                &mut group_count_i as *mut _ as *mut c_void,
                &mut group_indexes_ptr as *mut _ as *mut c_void,
                &mut group_index_count_i as *mut _ as *mut c_void,
                &mut signals_ptr as *mut _ as *mut c_void,
                &mut flags as *mut _ as *mut c_void,
                &mut commission as *mut _ as *mut c_void,
                &mut eps_rel as *mut _ as *mut c_void,
                &mut stop_log as *mut _ as *mut c_void,
                &mut take_log as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(&func, (1, 1, 1), (128, 1, 1), 0, args)
                .map_err(|e| e.to_string())?;
        }
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        let mut out = out_init;
        d_out.copy_to(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn assert_f32_slices_close(label: &str, actual: &[f32], expected: &[f32], tol: f32) {
        assert_eq!(actual.len(), expected.len(), "{label}: len mismatch");
        for (idx, (&a, &b)) in actual.iter().zip(expected.iter()).enumerate() {
            assert!(
                a.is_finite() && b.is_finite(),
                "{label}: non-finite value at {idx}: actual={a} expected={b}"
            );
            assert!(
                (a - b).abs() <= tol,
                "{label}: mismatch at {idx}: actual={a} expected={b}"
            );
        }
    }

    #[test]
    fn period_upload_signature_tracks_full_period_contents() {
        let a = super::period_upload_signature(&[5, 10, 10]);
        let b = super::period_upload_signature(&[5, 10, 10]);
        let c = super::period_upload_signature(&[5, 10, 11]);
        let d = super::period_upload_signature(&[5, 10]);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn candle_dataset_signature_tracks_dataset_identity() {
        let candles = make_trending_candles(32);
        let same_a = super::candle_dataset_signature(&candles);
        let same_b = super::candle_dataset_signature(&candles);
        let cloned = candles.clone();
        let clone_sig = super::candle_dataset_signature(&cloned);
        let shorter = make_trending_candles(31);
        let shorter_sig = super::candle_dataset_signature(&shorter);

        assert_eq!(same_a, same_b);
        assert_eq!(same_a.len, clone_sig.len);
        assert_eq!(same_a.field_mask, clone_sig.field_mask);
        assert_ne!(same_a, clone_sig);
        assert_ne!(same_a, shorter_sig);
    }

    #[test]
    fn staged_market_promotes_into_active_resident_cache() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        let _guard = lock_env();
        let current = make_trending_candles(512);
        let next = make_oscillating_candles(512);

        super::CUDA_KERNEL_RUNTIME
            .with(|cell| -> Result<(), String> {
                let mut map = cell.borrow_mut();
                map.clear();
                map.insert(0, super::CudaKernelRuntime::new(0)?);
                let rt = map.get_mut(&0).expect("runtime inserted");
                rt.set_current()?;

                let current_close = source_type(&current, "close");
                super::ensure_resident_market_series(
                    &rt.stream,
                    &mut rt.resident_market,
                    &current,
                    current_close,
                    "close",
                    None,
                    false,
                    false,
                    None,
                    None,
                    false,
                    None,
                )?;

                let next_close = source_type(&next, "close");
                let stage_outcome = super::stage_resident_market_series_for_next_dataset(
                    &rt.resident_market,
                    &rt.staging_stream,
                    &mut rt.staged_market,
                    &next,
                    next_close,
                    "close",
                    None,
                    false,
                    false,
                    None,
                    None,
                    false,
                    None,
                )?;
                assert!(matches!(
                    stage_outcome,
                    super::CampaignVramPrefetchOutcome::Staged
                        | super::CampaignVramPrefetchOutcome::AlreadyResident
                ));

                let next_sig = super::candle_dataset_signature(&next);
                assert_eq!(
                    rt.staged_market.as_ref().map(|cache| cache.signature),
                    Some(next_sig)
                );

                let promoted = super::promote_matching_staged_market_series(
                    &rt.stream,
                    &rt.staging_stream,
                    &mut rt.resident_market,
                    &mut rt.staged_market,
                    next_sig,
                )?;
                assert!(promoted);
                assert_eq!(
                    rt.resident_market.as_ref().map(|cache| cache.signature),
                    Some(next_sig)
                );
                assert!(rt.staged_market.is_none());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn kernel_scratch_clear_async_resets_returns_cache_key() {
        let _guard = lock_env();
        let candles = make_trending_candles(32);
        let mut scratch = super::KernelScratch::default();
        scratch.returns_key = Some(super::ReturnsCacheKey {
            dataset: super::candle_dataset_signature(&candles),
        });

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");
        scratch
            .clear_async(&rt.stream)
            .expect("clear scratch buffers asynchronously");

        assert!(scratch.returns_key.is_none());
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
    fn kernel_score_first_flag_defaults_to_disabled_and_parses_true() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_SCORE_FIRST_TOPK").ok();

        std::env::remove_var("VECTORBT_KERNEL_SCORE_FIRST_TOPK");
        assert!(!kernel_score_first_topk_enabled());

        std::env::set_var("VECTORBT_KERNEL_SCORE_FIRST_TOPK", "0");
        assert!(!kernel_score_first_topk_enabled());

        std::env::set_var("VECTORBT_KERNEL_SCORE_FIRST_TOPK", "1");
        assert!(kernel_score_first_topk_enabled());

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_SCORE_FIRST_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_SCORE_FIRST_TOPK"),
        }
    }

    #[test]
    fn kernel_row_score_heads_flag_defaults_to_disabled_and_parses_true() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_ROW_SCORE_HEADS").ok();

        std::env::remove_var("VECTORBT_KERNEL_ROW_SCORE_HEADS");
        assert!(!kernel_row_score_heads_enabled());

        std::env::set_var("VECTORBT_KERNEL_ROW_SCORE_HEADS", "0");
        assert!(!kernel_row_score_heads_enabled());

        std::env::set_var("VECTORBT_KERNEL_ROW_SCORE_HEADS", "false");
        assert!(!kernel_row_score_heads_enabled());

        std::env::set_var("VECTORBT_KERNEL_ROW_SCORE_HEADS", "1");
        assert!(kernel_row_score_heads_enabled());

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_ROW_SCORE_HEADS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_ROW_SCORE_HEADS"),
        }
    }

    #[test]
    fn kernel_exact_pruning_flag_defaults_to_disabled_and_parses_true() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_EXACT_PRUNING").ok();

        std::env::remove_var("VECTORBT_KERNEL_EXACT_PRUNING");
        assert!(!kernel_exact_pruning_enabled());

        std::env::set_var("VECTORBT_KERNEL_EXACT_PRUNING", "0");
        assert!(!kernel_exact_pruning_enabled());

        std::env::set_var("VECTORBT_KERNEL_EXACT_PRUNING", "false");
        assert!(!kernel_exact_pruning_enabled());

        std::env::set_var("VECTORBT_KERNEL_EXACT_PRUNING", "1");
        assert!(kernel_exact_pruning_enabled());

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_EXACT_PRUNING", v),
            None => std::env::remove_var("VECTORBT_KERNEL_EXACT_PRUNING"),
        }
    }

    #[test]
    fn score_first_topk_mode_requires_narrow_exact_surface() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_SCORE_FIRST_TOPK").ok();
        std::env::set_var("VECTORBT_KERNEL_SCORE_FIRST_TOPK", "1");

        assert!(should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));
        assert!(should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            None,
            None,
            TRADE_METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true, true, None, false, None, None, None, 10, false, None, false, false, 0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            Some(DoubleMaScoreMode::SupportShrunk10),
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            true,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            Some(ShortlistConfidenceMode::Psr),
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            Some(DoubleMaShortlistUnionMode::Pareto),
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            None,
            Some(ShortlistSliceMode::Halves),
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            true,
            None,
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            Some((10, 100)),
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            true,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            true,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            true,
            None,
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            8,
        ));
        assert!(!should_use_score_first_topk_mode(
            false,
            true,
            None,
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));
        assert!(!should_use_score_first_topk_mode(
            true,
            false,
            None,
            false,
            None,
            None,
            None,
            METRICS_COUNT,
            false,
            None,
            false,
            false,
            0,
        ));

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_SCORE_FIRST_TOPK", v),
            None => std::env::remove_var("VECTORBT_KERNEL_SCORE_FIRST_TOPK"),
        }
    }

    #[test]
    fn row_score_heads_mode_requires_row_only_score_mode_and_global_topk() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_ROW_SCORE_HEADS").ok();
        std::env::set_var("VECTORBT_KERNEL_ROW_SCORE_HEADS", "1");

        assert_eq!(
            row_score_mode_objective_index(Some(DoubleMaScoreMode::BalancedRiskAdjusted10)),
            Some(ROW_SCORE_OBJECTIVE_BASE + 1)
        );
        assert_eq!(
            row_score_mode_objective_index(Some(DoubleMaScoreMode::RobustMargin10)),
            Some(ROW_SCORE_OBJECTIVE_BASE + 6)
        );
        assert_eq!(
            row_score_mode_objective_index(Some(DoubleMaScoreMode::EffectiveSampleLcb10)),
            None
        );
        assert!(should_use_row_score_heads_mode(
            true,
            Some(DoubleMaScoreMode::SupportShrunk10),
            PRODUCT_REDUCED_METRICS_COUNT,
            false,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            false,
            0,
            20,
        ));
        assert!(!should_use_row_score_heads_mode(
            true,
            Some(DoubleMaScoreMode::EffectiveSampleLcb10),
            PRODUCT_REDUCED_METRICS_COUNT,
            false,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            false,
            0,
            20,
        ));
        assert!(!should_use_row_score_heads_mode(
            false,
            Some(DoubleMaScoreMode::SupportShrunk10),
            PRODUCT_REDUCED_METRICS_COUNT,
            false,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            false,
            0,
            20,
        ));
        assert!(!should_use_row_score_heads_mode(
            true,
            Some(DoubleMaScoreMode::SupportShrunk10),
            7,
            false,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            false,
            0,
            20,
        ));
        assert!(!should_use_row_score_heads_mode(
            true,
            Some(DoubleMaScoreMode::SupportShrunk10),
            PRODUCT_REDUCED_METRICS_COUNT,
            true,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            false,
            0,
            20,
        ));

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_ROW_SCORE_HEADS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_ROW_SCORE_HEADS"),
        }
    }

    #[test]
    fn exact_pruning_mode_requires_prunable_device_max_drawdown_constraint() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_EXACT_PRUNING").ok();
        std::env::set_var("VECTORBT_KERNEL_EXACT_PRUNING", "1");

        let constraints = DoubleMaConstraints {
            max_drawdown: Some(0.25),
            ..Default::default()
        };
        let threshold = exact_pruning_max_log_dd_threshold(Some(&constraints));
        assert!(threshold.is_some());
        assert!(should_use_exact_pruning_mode(
            true, true, threshold, true, false, false, false, None, false, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, true, None, true, false, false, false, None, false, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, true, threshold, false, false, false, false, None, false, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, true, threshold, true, true, false, false, None, false, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            false, true, threshold, true, false, false, false, None, false, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, false, threshold, true, false, false, false, None, false, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true,
            true,
            threshold,
            true,
            false,
            false,
            false,
            Some((10, 100)),
            false,
            false,
            0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, true, threshold, true, false, true, false, None, false, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, true, threshold, true, false, false, true, None, false, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, true, threshold, true, false, false, false, None, true, false, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, true, threshold, true, false, false, false, None, false, true, 0,
        ));
        assert!(!should_use_exact_pruning_mode(
            true, true, threshold, true, false, false, false, None, false, false, 16,
        ));

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_EXACT_PRUNING", v),
            None => std::env::remove_var("VECTORBT_KERNEL_EXACT_PRUNING"),
        }
    }

    #[test]
    fn exact_pruning_blockers_explain_row_first_fallback() {
        let blockers = exact_pruning_blockers(
            false,
            false,
            false,
            None,
            false,
            true,
            true,
            true,
            Some((100, 200)),
            true,
            true,
            32,
        );

        assert_eq!(
            blockers,
            vec![
                "env_disabled",
                "global_topk_disabled",
                "exact_pruning_symbols_unavailable",
                "no_prunable_max_drawdown",
                "device_constraints_unavailable",
                "device_include_all",
                "export_enabled",
                "host_filter_mode",
                "eval_range_set",
                "fast_precomputed_enabled",
                "slow_precomputed_enabled",
                "heatmap_bins_nonzero",
            ]
        );
    }

    #[test]
    fn score_first_topk_blockers_explain_why_row_first_fallback_is_used() {
        let blockers = score_first_topk_blockers(
            false,
            false,
            false,
            Some(DoubleMaScoreMode::BalancedRiskAdjusted10),
            true,
            Some(ShortlistConfidenceMode::Psr),
            Some(DoubleMaShortlistUnionMode::Pareto),
            Some(ShortlistSliceMode::Halves),
            PRODUCT_REDUCED_METRICS_COUNT,
            true,
            Some((100, 200)),
            true,
            true,
            32,
        );

        assert_eq!(
            blockers,
            vec![
                "env_disabled",
                "global_topk_disabled",
                "score_first_symbols_unavailable",
                "score_mode_active",
                "extra_shortlist_enabled",
                "shortlist_confidence_enabled",
                "shortlist_union_enabled",
                "shortlist_slice_enabled",
                "metrics_contract_not_full_row",
                "constraints_present",
                "eval_range_set",
                "fast_precomputed_enabled",
                "slow_precomputed_enabled",
                "heatmap_bins_nonzero",
            ]
        );
    }

    #[test]
    fn row_score_heads_blockers_explain_why_include_all_fallback_is_used() {
        let blockers = row_score_heads_blockers(
            false,
            false,
            Some(DoubleMaScoreMode::EffectiveSampleLcb10),
            7,
            true,
            true,
            true,
            Some(ShortlistConfidenceMode::Psr),
            Some(DoubleMaShortlistUnionMode::Pareto),
            Some(ShortlistSliceMode::Halves),
            true,
            Some((100, 200)),
            true,
            true,
            32,
            0,
        );

        assert_eq!(
            blockers,
            vec![
                "env_disabled",
                "global_topk_disabled",
                "score_mode_not_row_only",
                "metrics_contract_below_product10",
                "include_all",
                "export_enabled",
                "extra_shortlist_enabled",
                "shortlist_confidence_enabled",
                "shortlist_union_enabled",
                "shortlist_slice_enabled",
                "constraints_present",
                "eval_range_set",
                "fast_precomputed_enabled",
                "slow_precomputed_enabled",
                "heatmap_bins_nonzero",
                "top_k_zero",
            ]
        );
    }

    #[test]
    fn topk_select_contract_caps_global_device_reduction_width() {
        assert_eq!(
            normalize_topk_select_for_device(0, false).expect("top_k zero normalizes to one"),
            1
        );
        assert_eq!(
            normalize_topk_select_for_device(TOPK_BLOCK_THREADS as usize, false)
                .expect("block-width top_k is allowed"),
            TOPK_BLOCK_THREADS as usize
        );

        let err = normalize_topk_select_for_device((TOPK_BLOCK_THREADS as usize) + 1, false)
            .expect_err("top_k beyond block width should fail");
        assert!(err.contains("max 256 when include_all=false"));

        assert_eq!(
            normalize_topk_select_for_device((TOPK_BLOCK_THREADS as usize) + 1, true)
                .expect("include_all/score-mode path bypasses global top-k cap"),
            (TOPK_BLOCK_THREADS as usize) + 1
        );
    }

    #[test]
    fn extra_shortlist_objectives_without_request_is_empty() {
        let parsed =
            kernel_extra_shortlist_objectives(ObjectiveKind::Sharpe, None).expect("no extra heads");
        assert!(
            parsed.is_empty(),
            "unexpected implicit extra heads: {parsed:?}"
        );
    }

    #[test]
    fn extra_shortlist_objectives_request_parses_and_deduplicates() {
        let parsed = kernel_extra_shortlist_objectives(
            ObjectiveKind::Sharpe,
            Some(&[
                ObjectiveKind::Pnl,
                ObjectiveKind::Omega,
                ObjectiveKind::Sharpe,
                ObjectiveKind::Pnl,
            ]),
        )
        .expect("parse request extra heads");
        assert_eq!(
            parsed,
            vec![ObjectiveKind::Pnl, ObjectiveKind::Omega],
            "primary objective should be skipped and duplicates removed"
        );
    }

    #[test]
    fn shortlist_confidence_mode_request_maps_supported_values() {
        assert_eq!(
            kernel_shortlist_confidence_mode(Some(DoubleMaShortlistConfidenceMode::Psr))
                .expect("map psr"),
            Some(ShortlistConfidenceMode::Psr)
        );
        assert_eq!(
            kernel_shortlist_confidence_mode(Some(DoubleMaShortlistConfidenceMode::Dsr))
                .expect("map dsr"),
            Some(ShortlistConfidenceMode::Dsr)
        );
        assert_eq!(
            kernel_shortlist_confidence_mode(None).expect("unset confidence mode"),
            None
        );
    }

    #[test]
    fn shortlist_union_mode_request_maps_supported_values() {
        assert_eq!(
            kernel_shortlist_union_mode(Some(DoubleMaShortlistUnionMode::Lexicographic))
                .expect("map lexicographic"),
            Some(DoubleMaShortlistUnionMode::Lexicographic)
        );
        assert_eq!(
            kernel_shortlist_union_mode(Some(DoubleMaShortlistUnionMode::Pareto))
                .expect("map pareto"),
            Some(DoubleMaShortlistUnionMode::Pareto)
        );
        assert_eq!(
            kernel_shortlist_union_mode(Some(DoubleMaShortlistUnionMode::EpsilonFront))
                .expect("map epsilon"),
            Some(DoubleMaShortlistUnionMode::EpsilonFront)
        );
        assert_eq!(
            kernel_shortlist_union_mode(Some(DoubleMaShortlistUnionMode::WeightedRank))
                .expect("map weighted rank"),
            Some(DoubleMaShortlistUnionMode::WeightedRank)
        );
        assert_eq!(
            kernel_shortlist_union_mode(None).expect("unset union mode"),
            None
        );
    }

    #[test]
    fn shortlist_slice_mode_request_maps_supported_values() {
        assert_eq!(
            kernel_shortlist_slice_mode(Some(DoubleMaShortlistSliceMode::Halves))
                .expect("map halves"),
            Some(ShortlistSliceMode::Halves)
        );
        assert_eq!(
            kernel_shortlist_slice_mode(Some(DoubleMaShortlistSliceMode::Quartiles))
                .expect("map quartiles"),
            Some(ShortlistSliceMode::Quartiles)
        );
        assert_eq!(
            kernel_shortlist_slice_mode(None).expect("unset slice mode"),
            None
        );
    }

    #[test]
    fn shortlist_slice_benchmark_request_maps_supported_values() {
        assert!(matches!(
            kernel_shortlist_slice_benchmark(Some(SpaBenchmarkKind::Flat)).expect("map flat"),
            Some(SpaBenchmarkKind::Flat)
        ));
        assert!(matches!(
            kernel_shortlist_slice_benchmark(Some(SpaBenchmarkKind::BuyAndHold))
                .expect("map buy and hold"),
            Some(SpaBenchmarkKind::BuyAndHold)
        ));
        assert!(kernel_shortlist_slice_benchmark(None)
            .expect("unset slice benchmark")
            .is_none());
    }

    #[test]
    fn shortlist_slice_diagnostic_row_limit_defaults_and_obeys_env_override() {
        let prev = std::env::var("VECTORBT_KERNEL_SHORTLIST_SLICE_DIAG_ROWS").ok();
        std::env::remove_var("VECTORBT_KERNEL_SHORTLIST_SLICE_DIAG_ROWS");
        assert_eq!(kernel_shortlist_slice_diagnostic_row_limit(), 8);

        std::env::set_var("VECTORBT_KERNEL_SHORTLIST_SLICE_DIAG_ROWS", "3");
        assert_eq!(kernel_shortlist_slice_diagnostic_row_limit(), 3);

        std::env::set_var("VECTORBT_KERNEL_SHORTLIST_SLICE_DIAG_ROWS", "not-a-number");
        assert_eq!(kernel_shortlist_slice_diagnostic_row_limit(), 8);

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_SHORTLIST_SLICE_DIAG_ROWS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_SHORTLIST_SLICE_DIAG_ROWS"),
        }
    }

    fn make_union_row(
        fast_len: u16,
        slow_len: u16,
        sharpe: f64,
        pnl: f64,
        omega: f64,
        max_dd: f64,
    ) -> (DoubleMaParams, Metrics) {
        (
            DoubleMaParams {
                fast_len,
                slow_len,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            Metrics {
                sharpe,
                pnl,
                omega,
                max_dd,
                ..Metrics::default()
            },
        )
    }

    #[test]
    fn rerank_shortlist_by_union_mode_lexicographic_uses_objective_priority_order() {
        let rows = vec![
            make_union_row(5, 20, 2.0, 10.0, 1.1, 0.30),
            make_union_row(6, 21, 2.0, 8.0, 3.0, 0.10),
            make_union_row(7, 22, 1.9, 20.0, 2.0, 0.05),
            make_union_row(8, 23, 2.0, 10.0, 1.1, 0.05),
        ];

        let reranked = rerank_shortlist_by_union_mode(
            &rows,
            ObjectiveKind::Sharpe,
            &[ObjectiveKind::Pnl, ObjectiveKind::Omega],
            DoubleMaShortlistUnionMode::Lexicographic,
        );
        let actual_keys: Vec<(u16, u16)> = reranked
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert_eq!(actual_keys, vec![(8, 23), (5, 20), (6, 21), (7, 22)]);
    }

    #[test]
    fn rerank_shortlist_by_union_mode_pareto_prioritizes_front_rank_before_primary_score() {
        let rows = vec![
            make_union_row(5, 20, 1.0, 10.0, 1.0, 0.20),
            make_union_row(6, 21, 2.0, 8.0, 1.0, 0.10),
            make_union_row(7, 22, 1.5, 9.0, 1.0, 0.15),
            make_union_row(8, 23, 0.5, 7.0, 1.0, 0.05),
        ];

        let reranked = rerank_shortlist_by_union_mode(
            &rows,
            ObjectiveKind::Sharpe,
            &[ObjectiveKind::Pnl],
            DoubleMaShortlistUnionMode::Pareto,
        );
        let actual_keys: Vec<(u16, u16)> = reranked
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert_eq!(actual_keys, vec![(6, 21), (7, 22), (5, 20), (8, 23)]);
    }

    #[test]
    fn rerank_shortlist_by_union_mode_epsilon_front_softens_near_dominance() {
        let rows = vec![
            make_union_row(5, 20, 10.0, 10.0, 1.0, 0.20),
            make_union_row(6, 21, 10.001, 10.001, 1.0, 0.10),
            make_union_row(7, 22, 9.0, 20.0, 1.0, 0.05),
        ];

        let strict = rerank_shortlist_by_union_mode(
            &rows,
            ObjectiveKind::Sharpe,
            &[ObjectiveKind::Pnl],
            DoubleMaShortlistUnionMode::Pareto,
        );
        let relaxed = rerank_shortlist_by_union_mode(
            &rows,
            ObjectiveKind::Sharpe,
            &[ObjectiveKind::Pnl],
            DoubleMaShortlistUnionMode::EpsilonFront,
        );
        let strict_keys: Vec<(u16, u16)> = strict
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();
        let relaxed_keys: Vec<(u16, u16)> = relaxed
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert_eq!(strict_keys, vec![(6, 21), (7, 22), (5, 20)]);
        assert_eq!(relaxed_keys, vec![(6, 21), (5, 20), (7, 22)]);
    }

    #[test]
    fn rerank_shortlist_by_union_mode_weighted_rank_balances_objective_ranks() {
        let rows = vec![
            make_union_row(5, 20, 3.0, 1.0, 1.0, 0.20),
            make_union_row(6, 21, 2.0, 3.0, 2.0, 0.10),
            make_union_row(7, 22, 1.0, 2.0, 3.0, 0.05),
        ];

        let reranked = rerank_shortlist_by_union_mode(
            &rows,
            ObjectiveKind::Sharpe,
            &[ObjectiveKind::Pnl, ObjectiveKind::Omega],
            DoubleMaShortlistUnionMode::WeightedRank,
        );
        let actual_keys: Vec<(u16, u16)> = reranked
            .iter()
            .map(|(params, _)| (params.fast_len, params.slow_len))
            .collect();

        assert_eq!(actual_keys, vec![(6, 21), (5, 20), (7, 22)]);
    }

    #[test]
    fn rerank_shortlist_by_union_mode_is_noop_without_distinct_extra_objectives() {
        let rows = vec![
            make_union_row(5, 20, 2.0, 1.0, 1.0, 0.20),
            make_union_row(6, 21, 1.0, 2.0, 1.0, 0.10),
        ];

        let reranked = rerank_shortlist_by_union_mode(
            &rows,
            ObjectiveKind::Sharpe,
            &[ObjectiveKind::Sharpe],
            DoubleMaShortlistUnionMode::Pareto,
        );

        let original_scores: Vec<f64> = rows
            .iter()
            .map(|(_, metrics)| objective_score(metrics, ObjectiveKind::Sharpe))
            .collect();
        let reranked_scores: Vec<f64> = reranked
            .iter()
            .map(|(_, metrics)| objective_score(metrics, ObjectiveKind::Sharpe))
            .collect();

        assert_eq!(reranked.len(), rows.len());
        assert_eq!(reranked_scores, original_scores);
    }

    #[test]
    fn shortlist_slice_omission_summary_matches_expected_scores() {
        let labels = super::shortlist_slice_labels(ShortlistSliceMode::Quartiles);
        let summary =
            super::compute_shortlist_slice_omission_summary(&[2.0, -1.0, 3.0, 0.0], &labels)
                .expect("quartile omission summary");
        assert_eq!(summary.best_label, "q2");
        assert!(approx_eq(summary.best_score, 5.0 / 3.0, 1e-12));
        assert_eq!(summary.worst_label, "q3");
        assert!(approx_eq(summary.worst_score, 1.0 / 3.0, 1e-12));
        assert!(approx_eq(summary.spread, 4.0 / 3.0, 1e-12));
    }

    #[test]
    fn shortlist_slice_omission_summary_requires_slice_local_scores_not_just_aggregate_score() {
        let labels = super::shortlist_slice_labels(ShortlistSliceMode::Quartiles);
        let evenly_split = [4.0, 4.0, 0.0, 0.0];
        let concentrated = [8.0, 0.0, 0.0, 0.0];

        let evenly_mean = evenly_split.iter().copied().sum::<f64>() / evenly_split.len() as f64;
        let concentrated_mean =
            concentrated.iter().copied().sum::<f64>() / concentrated.len() as f64;
        assert!(approx_eq(evenly_mean, concentrated_mean, 1e-12));

        let evenly_summary =
            super::compute_shortlist_slice_omission_summary(&evenly_split, &labels)
                .expect("evenly split omission summary");
        let concentrated_summary =
            super::compute_shortlist_slice_omission_summary(&concentrated, &labels)
                .expect("concentrated omission summary");

        assert_ne!(evenly_summary.best_label, concentrated_summary.best_label);
        assert!(!approx_eq(
            evenly_summary.worst_score,
            concentrated_summary.worst_score,
            1e-12
        ));
        assert!(!approx_eq(
            evenly_summary.spread,
            concentrated_summary.spread,
            1e-12
        ));
    }

    #[test]
    fn shortlist_slice_benchmark_relative_summary_matches_expected_scores() {
        let labels = super::shortlist_slice_labels(ShortlistSliceMode::Quartiles);
        let summary = compute_shortlist_slice_benchmark_relative_summary(
            &[0.04, 0.00, -0.02, -0.03],
            &labels,
            &SpaBenchmarkKind::BuyAndHold,
        )
        .expect("benchmark-relative summary");
        assert_eq!(summary.benchmark, "buy_and_hold");
        assert!(approx_eq(summary.beat_rate, 0.25, 1e-12));
        assert!(approx_eq(summary.mean_excess_return, -0.0025, 1e-12));
        assert!(approx_eq(summary.best_excess_return, 0.04, 1e-12));
        assert!(approx_eq(summary.worst_excess_return, -0.03, 1e-12));
        assert!(approx_eq(
            summary.best_omission_excess_return,
            0.006666666666666667,
            1e-12
        ));
        assert!(approx_eq(
            summary.worst_omission_excess_return,
            -0.016666666666666666,
            1e-12
        ));
        assert!(approx_eq(
            summary.omission_excess_spread,
            0.023333333333333334,
            1e-12
        ));
    }

    #[test]
    fn shortlist_slice_benchmark_relative_summary_requires_slice_local_excess_returns() {
        let labels = super::shortlist_slice_labels(ShortlistSliceMode::Quartiles);
        let distributed = [0.04, 0.04, -0.04, -0.04];
        let concentrated = [
            0.16,
            -0.05333333333333334,
            -0.05333333333333334,
            -0.05333333333333334,
        ];

        let distributed_mean = distributed.iter().copied().sum::<f64>() / distributed.len() as f64;
        let concentrated_mean =
            concentrated.iter().copied().sum::<f64>() / concentrated.len() as f64;
        assert!(approx_eq(distributed_mean, concentrated_mean, 1e-12));

        let distributed_summary = compute_shortlist_slice_benchmark_relative_summary(
            &distributed,
            &labels,
            &SpaBenchmarkKind::BuyAndHold,
        )
        .expect("distributed benchmark-relative summary");
        let concentrated_summary = compute_shortlist_slice_benchmark_relative_summary(
            &concentrated,
            &labels,
            &SpaBenchmarkKind::BuyAndHold,
        )
        .expect("concentrated benchmark-relative summary");

        assert!(!approx_eq(
            distributed_summary.beat_rate,
            concentrated_summary.beat_rate,
            1e-12
        ));
        assert!(!approx_eq(
            distributed_summary.best_omission_excess_return,
            concentrated_summary.best_omission_excess_return,
            1e-12
        ));
        assert!(!approx_eq(
            distributed_summary.worst_omission_excess_return,
            concentrated_summary.worst_omission_excess_return,
            1e-12
        ));
    }

    #[test]
    fn shortlist_candidate_track_record_bars_respects_warmup_and_trade_offset() {
        let obs = shortlist_candidate_track_record_bars(0, 0, 0, 100, 100, 5, 20, false);
        assert_eq!(obs, 81);

        let delayed_obs = shortlist_candidate_track_record_bars(0, 0, 0, 100, 100, 5, 20, true);
        assert_eq!(delayed_obs, 80);

        let windowed_obs = shortlist_candidate_track_record_bars(3, 12, 10, 90, 100, 7, 25, true);
        assert_eq!(windowed_obs, 62);
    }

    #[test]
    fn effective_sample_pair_score_uses_exact_track_record_bars() {
        let params = DoubleMaParams {
            fast_len: 9,
            slow_len: 37,
            fast_ma_id: 0,
            slow_ma_id: 0,
        };
        let metrics = Metrics {
            pnl: 0.18,
            sharpe: 1.24,
            sortino: 1.91,
            downside_deviation: 0.09,
            calmar: 0.94,
            omega: 1.52,
            volatility: 0.17,
            ulcer_index: 0.05,
            martin_ratio: 2.2,
            max_dd: 0.11,
            trades: 28,
            exposure: 0.46,
            net_exposure: 0.02,
            ..Metrics::default()
        };
        let ctx = EffectiveSampleScoreContext {
            first_valid: 4,
            regime_first_valid: 11,
            eval_start: 20,
            eval_end: 420,
            total_bars: 500,
            trade_on_next_bar: true,
        };
        let expected_n = shortlist_candidate_track_record_bars(
            ctx.first_valid,
            ctx.regime_first_valid,
            ctx.eval_start,
            ctx.eval_end,
            ctx.total_bars,
            params.fast_len,
            params.slow_len,
            ctx.trade_on_next_bar,
        );
        let expected = effective_sample_lcb_10_score(&metrics, expected_n);
        let _guard = install_effective_sample_score_context(ctx);
        let score = effective_sample_lcb_10_pair_score(&params, &metrics);
        assert!(
            (score - expected).abs() <= 1e-12,
            "expected effective-sample score to use exact n_obs: expected={expected} got={score}"
        );
    }

    #[test]
    fn shortlist_slice_ranges_use_common_shortlist_start() {
        let rows = vec![
            (
                DoubleMaParams {
                    fast_len: 5,
                    slow_len: 20,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                Metrics::default(),
            ),
            (
                DoubleMaParams {
                    fast_len: 12,
                    slow_len: 40,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                Metrics::default(),
            ),
        ];
        let ranges = shortlist_slice_ranges(
            &rows,
            ShortlistSliceMode::Halves,
            0,
            0,
            0,
            200,
            200,
            &StrategyConfig::default(),
        )
        .expect("slice ranges");
        assert_eq!(ranges, vec![(40, 120), (120, 200)]);
    }

    #[test]
    fn prepare_batched_windowed_exact_replay_rows_preserves_range_major_order() {
        let mut scratch = super::HostExactComboScratch::default();
        scratch.tile_combo_indices = vec![0, 2];
        scratch.tile_pair_index_map = vec![1, 3];
        scratch.tile_fast_period_rows = vec![5, 8];
        scratch.tile_slow_period_rows = vec![10, 13];
        scratch.tile_fast_local_rows = vec![0, 1];
        scratch.tile_slow_local_rows = vec![1, 0];

        super::prepare_batched_windowed_exact_replay_rows(&mut scratch, &[(3, 11), (11, 21)], 18)
            .expect("batched replay rows");

        assert_eq!(scratch.tile_combo_indices, vec![0, 2, 0, 2]);
        assert_eq!(scratch.tile_pair_index_map, vec![1, 3, 1, 3]);
        assert_eq!(scratch.tile_fast_period_rows, vec![5, 8, 5, 8]);
        assert_eq!(scratch.tile_slow_period_rows, vec![10, 13, 10, 13]);
        assert_eq!(scratch.tile_fast_local_rows, vec![0, 1, 0, 1]);
        assert_eq!(scratch.tile_slow_local_rows, vec![1, 0, 1, 0]);
        assert_eq!(scratch.tile_range_indices, vec![0, 0, 1, 1]);
        assert_eq!(scratch.tile_eval_start_rows, vec![3, 3, 11, 11]);
        assert_eq!(scratch.tile_eval_end_rows, vec![11, 11, 18, 18]);
    }

    #[test]
    fn rerank_shortlist_by_confidence_matches_manual_probability_order() {
        let rows = vec![
            (
                DoubleMaParams {
                    fast_len: 5,
                    slow_len: 20,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                Metrics {
                    sharpe: 1.20,
                    max_dd: 0.18,
                    ..Metrics::default()
                },
            ),
            (
                DoubleMaParams {
                    fast_len: 18,
                    slow_len: 24,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                Metrics {
                    sharpe: 1.16,
                    max_dd: 0.10,
                    ..Metrics::default()
                },
            ),
            (
                DoubleMaParams {
                    fast_len: 40,
                    slow_len: 50,
                    fast_ma_id: 0,
                    slow_ma_id: 0,
                },
                Metrics {
                    sharpe: 1.14,
                    max_dd: 0.09,
                    ..Metrics::default()
                },
            ),
        ];
        let strategy = StrategyConfig::default();
        let sharpe_peers: Vec<f64> = rows.iter().map(|(_, metrics)| metrics.sharpe).collect();
        let (peer_mean, peer_std) = crate::sharpe_stats::mean_std(&sharpe_peers);
        for mode in [ShortlistConfidenceMode::Psr, ShortlistConfidenceMode::Dsr] {
            let reranked = rerank_shortlist_by_confidence(
                &rows,
                mode,
                0,
                0,
                0,
                400,
                400,
                &strategy,
                ObjectiveKind::Sharpe,
                None,
            );

            let mut expected = rows.clone();
            expected.sort_by(|a, b| {
                let obs_a = shortlist_candidate_track_record_bars(
                    0,
                    0,
                    0,
                    400,
                    400,
                    a.0.fast_len,
                    a.0.slow_len,
                    strategy.trade_on_next_bar,
                );
                let obs_b = shortlist_candidate_track_record_bars(
                    0,
                    0,
                    0,
                    400,
                    400,
                    b.0.fast_len,
                    b.0.slow_len,
                    strategy.trade_on_next_bar,
                );
                let (psr_a, dsr_a) = crate::validation_stats::compute_psr_dsr(
                    a.1.sharpe,
                    obs_a,
                    sharpe_peers.len(),
                    peer_mean,
                    peer_std,
                );
                let (psr_b, dsr_b) = crate::validation_stats::compute_psr_dsr(
                    b.1.sharpe,
                    obs_b,
                    sharpe_peers.len(),
                    peer_mean,
                    peer_std,
                );
                let score_a = match mode {
                    ShortlistConfidenceMode::Psr => psr_a.unwrap_or(0.0),
                    ShortlistConfidenceMode::Dsr => dsr_a.unwrap_or(0.0),
                };
                let score_b = match mode {
                    ShortlistConfidenceMode::Psr => psr_b.unwrap_or(0.0),
                    ShortlistConfidenceMode::Dsr => dsr_b.unwrap_or(0.0),
                };
                score_b
                    .total_cmp(&score_a)
                    .then_with(|| b.1.sharpe.total_cmp(&a.1.sharpe))
                    .then_with(|| a.1.max_dd.total_cmp(&b.1.max_dd))
            });

            let actual_keys: Vec<(u16, u16)> = reranked
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();
            let expected_keys: Vec<(u16, u16)> = expected
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect();
            assert_eq!(actual_keys, expected_keys, "mode={mode:?}");
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
    fn kernel_d2h_flag_defaults_to_enabled() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_PINNED_D2H").ok();
        std::env::remove_var("VECTORBT_KERNEL_PINNED_D2H");
        assert!(kernel_pinned_d2h_enabled());
        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PINNED_D2H", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PINNED_D2H"),
        }
    }

    #[test]
    fn kernel_d2h_flag_respects_false_tokens() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_PINNED_D2H").ok();

        std::env::set_var("VECTORBT_KERNEL_PINNED_D2H", "0");
        assert!(!kernel_pinned_d2h_enabled());

        std::env::set_var("VECTORBT_KERNEL_PINNED_D2H", "false");
        assert!(!kernel_pinned_d2h_enabled());

        std::env::set_var("VECTORBT_KERNEL_PINNED_D2H", "1");
        assert!(kernel_pinned_d2h_enabled());

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PINNED_D2H", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PINNED_D2H"),
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
    fn kernel_valid_pair_map_cap_defaults_high_enough_for_xlarge_dense_grid() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_VALID_PAIR_MAP_MAX_PAIRS").ok();
        std::env::remove_var("VECTORBT_KERNEL_VALID_PAIR_MAP_MAX_PAIRS");
        assert!(
            super::kernel_valid_pair_map_max_pairs() >= 491_536,
            "default valid-pair cap should cover the xlarge dense grid"
        );
        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_VALID_PAIR_MAP_MAX_PAIRS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_VALID_PAIR_MAP_MAX_PAIRS"),
        }
    }

    #[test]
    fn search_metrics_count_uses_minimal_search_payload_when_allowed() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS").ok();
        let prev_recompute = std::env::var("VECTORBT_KERNEL_FINALIST_RECOMPUTE").ok();
        std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", "0");
        std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", "0");

        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Pnl,
                None,
                None,
                false,
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
                None,
                false,
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
                None,
                false,
                false,
                false,
                false,
                false,
            ),
            7
        );
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Sortino,
                None,
                None,
                false,
                false,
                false,
                false,
                false,
            ),
            super::METRICS_COUNT
        );

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS"),
        }
        match prev_recompute {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE"),
        }
    }

    #[test]
    fn search_metrics_count_falls_back_to_full_metrics_when_required() {
        let constraints = super::DoubleMaConstraints {
            min_trades: Some(10),
            max_drawdown: Some(0.2),
            ..Default::default()
        };
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Pnl,
                None,
                Some(&constraints),
                false,
                false,
                false,
                false,
                false,
            ),
            3
        );
        let exposure = super::DoubleMaConstraints {
            max_abs_net_exposure: Some(0.25),
            ..Default::default()
        };
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Pnl,
                None,
                Some(&exposure),
                false,
                false,
                false,
                false,
                false,
            ),
            7
        );
        let omega = super::DoubleMaConstraints {
            min_omega: Some(1.1),
            ..Default::default()
        };
        assert_eq!(
            super::search_metrics_count_for_optimization(
                super::ObjectiveKind::Pnl,
                None,
                Some(&omega),
                false,
                false,
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
                None,
                true,
                false,
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
                None,
                false,
                true,
                false,
                false,
                false,
            ),
            super::METRICS_COUNT
        );
    }

    #[test]
    fn search_metrics_count_promotes_to_full_rows_for_score_mode() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS").ok();
        let prev_recompute = std::env::var("VECTORBT_KERNEL_FINALIST_RECOMPUTE").ok();
        std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", "0");
        std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", "0");

        for score_mode in [
            super::DoubleMaScoreMode::BalancedRiskAdjusted10,
            super::DoubleMaScoreMode::AdversarialEnvelope10,
            super::DoubleMaScoreMode::SupportShrunk10,
            super::DoubleMaScoreMode::ActiveSortino10,
            super::DoubleMaScoreMode::EffectiveSampleLcb10,
            super::DoubleMaScoreMode::OmegaVsSharpe10,
            super::DoubleMaScoreMode::RobustMargin10,
        ] {
            assert_eq!(
                super::search_metrics_count_for_optimization(
                    super::ObjectiveKind::Sharpe,
                    Some(score_mode),
                    None,
                    false,
                    false,
                    false,
                    false,
                    false,
                ),
                super::PRODUCT_REDUCED_METRICS_COUNT
            );
        }

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS"),
        }
        match prev_recompute {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE"),
        }
    }

    #[test]
    fn exact_eval_metrics_count_prefers_metric_contract_over_trade_flag() {
        assert_eq!(
            super::exact_eval_metrics_count(super::KernelConfig {
                device_id: 0,
                fast_alma_offset: 0.85,
                fast_alma_sigma: 6.0,
                slow_alma_offset: 0.85,
                slow_alma_sigma: 6.0,
                hurdle_tau: 0.0,
                include_trade_metrics: false,
                metric_contract: Some(super::DoubleMaMetricContract::Trade22),
            }),
            super::TRADE_METRICS_COUNT
        );
        assert_eq!(
            super::exact_eval_metrics_count(super::KernelConfig {
                device_id: 0,
                fast_alma_offset: 0.85,
                fast_alma_sigma: 6.0,
                slow_alma_offset: 0.85,
                slow_alma_sigma: 6.0,
                hurdle_tau: 0.0,
                include_trade_metrics: true,
                metric_contract: Some(super::DoubleMaMetricContract::Full17),
            }),
            super::METRICS_COUNT
        );
        assert_eq!(
            super::exact_eval_metrics_count(super::KernelConfig {
                device_id: 0,
                fast_alma_offset: 0.85,
                fast_alma_sigma: 6.0,
                slow_alma_offset: 0.85,
                slow_alma_sigma: 6.0,
                hurdle_tau: 0.0,
                include_trade_metrics: true,
                metric_contract: None,
            }),
            super::TRADE_METRICS_COUNT
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn exact_eval_windowed_trade22_populates_trade_tail_metrics() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let candles = make_oscillating_candles(4096);
        let strategy = StrategyConfig::default();
        let combos = vec![
            DoubleMaParams {
                fast_len: 5,
                slow_len: 20,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            DoubleMaParams {
                fast_len: 6,
                slow_len: 21,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            DoubleMaParams {
                fast_len: 7,
                slow_len: 22,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
            DoubleMaParams {
                fast_len: 8,
                slow_len: 23,
                fast_ma_id: 0,
                slow_ma_id: 0,
            },
        ];
        let cfg = super::KernelConfig {
            device_id: 0,
            fast_alma_offset: 0.85,
            fast_alma_sigma: 6.0,
            slow_alma_offset: 0.85,
            slow_alma_sigma: 6.0,
            hurdle_tau: 0.0,
            include_trade_metrics: true,
            metric_contract: Some(super::DoubleMaMetricContract::Trade22),
        };

        let metrics = super::eval_double_ma_batch_gpu_kernel_with_periods_and_regime(
            &candles,
            combos.as_slice(),
            &[5u16, 6u16, 7u16, 8u16],
            &[20u16, 21u16, 22u16, 23u16],
            "alma",
            "alma",
            "close",
            None,
            None,
            None,
            cfg,
            &strategy,
            Some((512, 3584)),
        )
        .expect("windowed exact gpu eval trade22");

        assert!(
            metrics.iter().any(|row| row.trades > 0),
            "expected at least one trade on windowed eval"
        );
        assert!(
            metrics.iter().any(|row| {
                row.closed_trades > 0
                    && row.winning_trades + row.losing_trades <= row.closed_trades
                    && (row.gross_profit != 0.0 || row.gross_loss != 0.0)
            }),
            "expected trade22 tail metrics to be populated on windowed eval"
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
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS").ok();
        let prev_recompute = std::env::var("VECTORBT_KERNEL_FINALIST_RECOMPUTE").ok();
        std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", "0");
        std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", "0");

        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                None,
                false,
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
                None,
                false,
                false,
                false,
                false,
                false,
            ),
            7
        );
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Volatility,
                None,
                None,
                false,
                false,
                false,
                false,
                false,
            ),
            7
        );
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::MaxDrawdown,
                None,
                None,
                false,
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
                None,
                false,
                false,
                false,
                false,
                false,
            ),
            3
        );

        let min_trades = DoubleMaConstraints {
            min_trades: Some(20),
            ..Default::default()
        };
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                Some(&min_trades),
                false,
                false,
                false,
                false,
                false,
            ),
            2
        );

        let max_dd = DoubleMaConstraints {
            max_drawdown: Some(0.25),
            ..Default::default()
        };
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                Some(&max_dd),
                false,
                false,
                false,
                false,
                false,
            ),
            3
        );

        let exposure = DoubleMaConstraints {
            max_abs_net_exposure: Some(0.30),
            ..Default::default()
        };
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                Some(&exposure),
                false,
                false,
                false,
                false,
                false,
            ),
            7
        );

        let sortino = DoubleMaConstraints {
            min_sortino: Some(1.1),
            ..Default::default()
        };
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                Some(&sortino),
                false,
                false,
                false,
                false,
                false,
            ),
            8
        );

        let omega = DoubleMaConstraints {
            min_omega: Some(1.2),
            ..Default::default()
        };
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                Some(&omega),
                false,
                false,
                false,
                false,
                false,
            ),
            METRICS_COUNT
        );

        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                None,
                true,
                false,
                false,
                false,
                false,
            ),
            METRICS_COUNT
        );

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS"),
        }
        match prev_recompute {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE"),
        }
    }

    #[test]
    fn search_metrics_count_product_reduced_rows_default_and_override() {
        let _guard = lock_env();
        let prev = std::env::var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS").ok();
        let prev_recompute = std::env::var("VECTORBT_KERNEL_FINALIST_RECOMPUTE").ok();

        std::env::remove_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS");
        std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", "0");
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Sharpe,
                None,
                None,
                false,
                false,
                false,
                false,
                false,
            ),
            PRODUCT_REDUCED_METRICS_COUNT
        );
        assert!(can_decode_current_gpu_product_metrics_without_recompute(
            PRODUCT_REDUCED_METRICS_COUNT
        ));

        std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", "0");
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Sharpe,
                None,
                None,
                false,
                false,
                false,
                false,
                false,
            ),
            7
        );

        std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", "1");
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                None,
                false,
                false,
                false,
                false,
                false,
            ),
            PRODUCT_REDUCED_METRICS_COUNT
        );

        match prev {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS", v),
            None => std::env::remove_var("VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS"),
        }
        match prev_recompute {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE", v),
            None => std::env::remove_var("VECTORBT_KERNEL_FINALIST_RECOMPUTE"),
        }
    }

    #[test]
    fn search_metrics_count_metric_contract_override_forces_requested_rows() {
        assert_eq!(
            search_metrics_count_for_optimization_with_contract(
                ObjectiveKind::Sharpe,
                None,
                None,
                false,
                false,
                false,
                false,
                false,
                Some(DoubleMaMetricContract::Full17),
            ),
            METRICS_COUNT
        );
        assert_eq!(
            search_metrics_count_for_optimization_with_contract(
                ObjectiveKind::Sharpe,
                None,
                None,
                false,
                false,
                false,
                false,
                false,
                Some(DoubleMaMetricContract::Trade22),
            ),
            TRADE_METRICS_COUNT
        );
    }

    #[test]
    fn exact_eval_metrics_count_contract_respects_trade22_request() {
        assert_eq!(
            super::exact_eval_metrics_count_with_contract(
                false,
                Some(DoubleMaMetricContract::Full17),
            ),
            METRICS_COUNT
        );
        assert_eq!(
            super::exact_eval_metrics_count_with_contract(
                false,
                Some(DoubleMaMetricContract::Trade22),
            ),
            TRADE_METRICS_COUNT
        );
        assert_eq!(
            super::exact_eval_metrics_count_with_contract(true, None),
            TRADE_METRICS_COUNT
        );
        assert_eq!(
            super::exact_eval_metrics_count_with_contract(false, None),
            METRICS_COUNT
        );
    }

    #[test]
    fn finalist_recompute_eval_range_preserves_windowed_span() {
        assert_eq!(
            finalist_recompute_eval_range(Some((512, 3584)), 4096),
            (512, 3584)
        );
        assert_eq!(finalist_recompute_eval_range(None, 4096), (0, 4096));
    }

    #[test]
    fn search_metrics_count_promotes_to_trade_rows_when_requested() {
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Sharpe,
                None,
                None,
                false,
                false,
                false,
                false,
                true,
            ),
            TRADE_METRICS_COUNT
        );
        assert_eq!(
            search_metrics_count_for_optimization(
                ObjectiveKind::Pnl,
                None,
                None,
                false,
                false,
                false,
                false,
                true,
            ),
            TRADE_METRICS_COUNT
        );
    }

    #[test]
    fn fixed_metrics_contract_rows_require_matching_metric_stride() {
        assert_eq!(
            fixed_metrics_contract_rows(
                PRODUCT_REDUCED_METRICS_COUNT,
                PRODUCT_REDUCED_METRICS_COUNT
            ),
            Some(PRODUCT_REDUCED_METRICS_COUNT)
        );
        assert_eq!(
            fixed_metrics_contract_rows(METRICS_COUNT, METRICS_COUNT),
            Some(METRICS_COUNT)
        );
        assert_eq!(
            fixed_metrics_contract_rows(TRADE_METRICS_COUNT, TRADE_METRICS_COUNT),
            Some(TRADE_METRICS_COUNT)
        );
        assert_eq!(
            fixed_metrics_contract_rows(METRICS_COUNT, TRADE_METRICS_COUNT),
            None
        );
        assert_eq!(
            fixed_metrics_contract_rows(TRADE_METRICS_COUNT, METRICS_COUNT),
            None
        );
        assert_eq!(
            fixed_metrics_contract_rows(PRODUCT_REDUCED_METRICS_COUNT, METRICS_COUNT),
            None
        );
        assert_eq!(fixed_metrics_contract_rows(TRADE_METRICS_COUNT, 24), None);
    }

    #[test]
    fn effective_metrics_stride_accepts_fixed_full_row_override() {
        assert_eq!(effective_metrics_stride(METRICS_COUNT, false, Some(24)), 24);
        assert_eq!(
            effective_metrics_stride(TRADE_METRICS_COUNT, false, Some(24)),
            24
        );
        assert_eq!(
            effective_metrics_stride(TRADE_METRICS_COUNT, false, Some(METRICS_COUNT)),
            TRADE_METRICS_COUNT
        );
    }

    #[test]
    fn fixed_contract_dispatch_selects_specialized_routes_when_symbols_are_present() {
        assert_eq!(
            choose_fixed_contract_dispatch_kind(
                Some(METRICS_COUNT),
                true,
                false,
                false,
                false,
                false,
                true,
                true,
                true,
                true,
                true,
                true,
            ),
            FixedContractDispatchKind::WindowedM17
        );
        assert_eq!(
            choose_fixed_contract_dispatch_kind(
                Some(METRICS_COUNT),
                false,
                true,
                false,
                false,
                false,
                true,
                true,
                true,
                true,
                true,
                true,
            ),
            FixedContractDispatchKind::LockstepM17
        );
        assert_eq!(
            choose_fixed_contract_dispatch_kind(
                Some(TRADE_METRICS_COUNT),
                true,
                false,
                false,
                false,
                false,
                true,
                true,
                true,
                true,
                true,
                true,
            ),
            FixedContractDispatchKind::WindowedM22
        );
        assert_eq!(
            choose_fixed_contract_dispatch_kind(
                Some(TRADE_METRICS_COUNT),
                false,
                false,
                false,
                false,
                false,
                true,
                true,
                true,
                true,
                true,
                true,
            ),
            FixedContractDispatchKind::M22
        );
    }

    #[test]
    fn fixed_contract_dispatch_falls_back_when_matching_symbol_is_unavailable() {
        assert_eq!(
            choose_fixed_contract_dispatch_kind(
                Some(METRICS_COUNT),
                true,
                false,
                false,
                false,
                false,
                false,
                true,
                true,
                true,
                true,
                true,
            ),
            FixedContractDispatchKind::None
        );
        assert_eq!(
            choose_fixed_contract_dispatch_kind(
                Some(METRICS_COUNT),
                false,
                true,
                false,
                false,
                false,
                true,
                true,
                true,
                true,
                false,
                true,
            ),
            FixedContractDispatchKind::None
        );
        assert_eq!(
            choose_fixed_contract_dispatch_kind(
                Some(TRADE_METRICS_COUNT),
                true,
                false,
                false,
                false,
                false,
                true,
                false,
                true,
                true,
                true,
                true,
            ),
            FixedContractDispatchKind::None
        );
        assert_eq!(
            choose_fixed_contract_dispatch_kind(
                Some(TRADE_METRICS_COUNT),
                false,
                false,
                false,
                false,
                false,
                true,
                true,
                true,
                false,
                true,
                false,
            ),
            FixedContractDispatchKind::None
        );
    }

    #[test]
    fn fixed_contract_kernel_flavor_covers_full_row_regime_variants() {
        assert_eq!(
            choose_fixed_contract_kernel_flavor(true, false, false, false, false),
            FixedContractKernelFlavor::RegimeOffNoRisk
        );
        assert_eq!(
            choose_fixed_contract_kernel_flavor(true, false, false, true, true),
            FixedContractKernelFlavor::RegimeOffNoRiskEps0NoComm
        );
        assert_eq!(
            choose_fixed_contract_kernel_flavor(true, true, false, true, true),
            FixedContractKernelFlavor::RegimeOnNoRiskEps0NoComm
        );
        assert_eq!(
            choose_fixed_contract_kernel_flavor(true, true, false, false, false),
            FixedContractKernelFlavor::RegimeOnNoRisk
        );
        assert_eq!(
            choose_fixed_contract_kernel_flavor(true, false, true, false, false),
            FixedContractKernelFlavor::RegimeHybridNoRisk
        );
        assert_eq!(
            choose_fixed_contract_kernel_flavor(true, false, true, true, true),
            FixedContractKernelFlavor::RegimeHybridNoRiskEps0NoComm
        );
        assert_eq!(
            choose_fixed_contract_kernel_flavor(false, true, true, true, true),
            FixedContractKernelFlavor::Base
        );
    }

    #[test]
    fn kernel_strategy_params_always_set_signed_exposure_flag() {
        let base = kernel_strategy_params(&StrategyConfig::default());
        assert_ne!(base.flags & STRAT_SIGNED_EXPOSURE, 0);

        let mut variant = StrategyConfig::default();
        variant.long_only = true;
        variant.allow_flip = false;
        variant.trade_on_next_bar = true;
        let variant = kernel_strategy_params(&variant);
        assert_ne!(variant.flags & STRAT_SIGNED_EXPOSURE, 0);
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
    fn build_valid_pair_index_map_preserves_dense_pf_major_order() {
        let fast = [5u16, 10u16];
        let slow = [6u16, 10u16, 12u16];

        let dense_pairs = build_valid_pair_index_map_i32(&fast, &slow).expect("pair map");
        assert_eq!(dense_pairs, vec![0, 1, 2, 5]);

        let visited_pairs: Vec<(u16, u16)> = dense_pairs
            .iter()
            .map(|&dense| {
                let dense = usize::try_from(dense).expect("non-negative dense pair index");
                let fi = dense / slow.len();
                let sj = dense % slow.len();
                (fast[fi], slow[sj])
            })
            .collect();
        assert_eq!(visited_pairs, vec![(5, 6), (5, 10), (5, 12), (10, 12)]);
    }

    #[test]
    fn build_valid_pair_local_rows_match_compact_pf_major_order() {
        let fast = [5u16, 10u16];
        let slow = [6u16, 10u16, 12u16];

        let (fast_rows, slow_rows) =
            build_valid_pair_local_rows_i32(&fast, &slow).expect("local rows");

        assert_eq!(fast_rows, vec![0, 0, 0, 1]);
        assert_eq!(slow_rows, vec![0, 1, 2, 2]);
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

        let metrics = decode_tm_flex_metrics_row(&row, 0, 5, 0.0);
        assert!((metrics.pnl - pnl).abs() < 1e-6);
        assert_eq!(metrics.trades, 12);
        assert!((metrics.max_dd - max_dd).abs() < 1e-6);
    }

    #[test]
    fn search_rows_use_log_space_keeps_fifteen_and_sixteen_in_search_contract() {
        assert!(search_rows_use_log_space(15));
        assert!(search_rows_use_log_space(16));
        assert!(!search_rows_use_log_space(METRICS_COUNT));
        assert!(!search_rows_use_log_space(TRADE_METRICS_COUNT));
    }

    #[test]
    fn decode_fifteen_and_sixteen_metric_rows_restore_calmar_from_log_space() {
        let pnl = 0.18f64;
        let max_dd = 0.30f64;
        let expected_calmar = calmar_ratio(pnl, max_dd);
        let base_row = vec![
            pnl.ln_1p() as f32,
            9.0f32,
            (-(1.0 - max_dd).ln()) as f32,
            0.05f32,
            0.02f32,
            0.45f32,
            0.12f32,
            0.02f32,
            0.04f32,
            3.5f32,
            0.20f32,
            0.07f32,
            0.06f32,
            -0.03f32,
            1.20f32,
            0.40f32,
        ];

        let metrics_15 = decode_tm_flex_metrics_row(&base_row, 0, 15, 0.0);
        assert!((metrics_15.pnl - pnl).abs() < 1e-6);
        assert!((metrics_15.max_dd - max_dd).abs() < 1e-6);
        assert!((metrics_15.calmar - expected_calmar).abs() < 1e-6);

        let metrics_16 = decode_tm_flex_metrics_row(&base_row, 0, 16, 0.0);
        assert!((metrics_16.pnl - pnl).abs() < 1e-6);
        assert!((metrics_16.max_dd - max_dd).abs() < 1e-6);
        assert!((metrics_16.calmar - expected_calmar).abs() < 1e-6);
    }

    #[test]
    fn decode_full_metric_rows_restore_omega_and_derive_martin_ratio() {
        let row = vec![
            0.15f32, 7.0f32, 0.20f32, 0.06f32, 0.03f32, 0.40f32, 0.10f32, 0.02f32, 0.05f32, 4.0f32,
            0.25f32, 0.08f32, 0.07f32, -0.04f32, 1.75f32, 0.35f32, 12.0f32,
        ];

        let metrics = decode_tm_flex_metrics_row(&row, 0, METRICS_COUNT, 0.0);
        assert!((metrics.sortino - 3.0).abs() < 1e-6);
        assert!((metrics.omega - 4.0).abs() < 1e-6);
        assert!((metrics.martin_ratio - 3.0).abs() < 1e-6);
        assert!((metrics.downside_frequency - 0.25).abs() < 1e-6);
        assert!((metrics.downside_severity - 0.08).abs() < 1e-6);
        assert!((metrics.long_mean_return - 0.07).abs() < 1e-6);
        assert!((metrics.short_mean_return + 0.04).abs() < 1e-6);
        assert!((metrics.kappa_tau - 1.75).abs() < 1e-6);
        assert!((metrics.underwater_fraction - 0.35).abs() < 1e-6);
        assert_eq!(metrics.max_underwater_duration_bars, 12);
    }

    #[test]
    fn decode_trade_metric_rows_restore_trade_distribution_summaries() {
        let row = vec![
            0.15f32, 7.0f32, 0.20f32, 0.06f32, 0.03f32, 0.40f32, 0.10f32, 0.02f32, 0.05f32, 4.0f32,
            0.25f32, 0.08f32, 0.07f32, -0.04f32, 1.75f32, 0.35f32, 12.0f32, 5.0f32, 3.0f32, 2.0f32,
            0.90f32, 0.30f32,
        ];

        let metrics = decode_tm_flex_metrics_row(&row, 0, TRADE_METRICS_COUNT, 0.0);
        assert_eq!(metrics.closed_trades, 5);
        assert_eq!(metrics.winning_trades, 3);
        assert_eq!(metrics.losing_trades, 2);
        assert!((metrics.gross_profit - 0.90).abs() < 1e-6);
        assert!((metrics.gross_loss - 0.30).abs() < 1e-6);
        assert!((metrics.underwater_fraction - 0.35).abs() < 1e-6);
        assert_eq!(metrics.max_underwater_duration_bars, 12);
    }

    #[test]
    fn decode_full_metric_rows_apply_hurdle_tau_to_sortino() {
        let row = vec![
            0.15f32, 7.0f32, 0.20f32, 0.06f32, 0.03f32, 0.40f32, 0.10f32, 0.02f32, 0.05f32, 4.0f32,
        ];

        let metrics = decode_tm_flex_metrics_row(&row, 0, PRODUCT_REDUCED_METRICS_COUNT, 0.01);
        assert!((metrics.sortino - 2.5).abs() < 1e-6);
        assert!((metrics.omega - 4.0).abs() < 1e-6);
    }

    #[test]
    fn decode_product_reduced_rows_restore_omega_and_derive_martin_ratio() {
        let pnl = 0.15f64;
        let max_dd = 0.20f64;
        let ulcer = 0.05f64;
        let row = vec![
            pnl.ln_1p() as f32,
            7.0f32,
            (-(1.0 - max_dd).ln()) as f32,
            0.06f32,
            0.03f32,
            0.40f32,
            0.10f32,
            0.02f32,
            ulcer as f32,
            4.0f32,
        ];

        let metrics = decode_tm_flex_metrics_row(&row, 0, PRODUCT_REDUCED_METRICS_COUNT, 0.0);
        assert!((metrics.pnl - pnl).abs() < 1e-6);
        assert!((metrics.max_dd - max_dd).abs() < 1e-6);
        assert!((metrics.omega - 4.0).abs() < 1e-6);
        assert!((metrics.martin_ratio - 3.0).abs() < 1e-6);
        assert_eq!(metrics.downside_frequency, 0.0);
        assert_eq!(metrics.downside_severity, 0.0);
        assert_eq!(metrics.long_mean_return, 0.0);
        assert_eq!(metrics.short_mean_return, 0.0);
        assert_eq!(metrics.kappa_tau, 0.0);
    }

    #[test]
    fn max_drawdown_constraint_converts_for_reduced_search_rows() {
        let raw = max_drawdown_constraint_to_search_space(0.25, METRICS_COUNT);
        assert!((raw - 0.25).abs() < 1e-6);

        let search = max_drawdown_constraint_to_search_space(0.25, 3);
        let expected = (-(1.0f64 - 0.25).ln()) as f32;
        assert!((search - expected).abs() < 1e-6);
        assert!((max_drawdown_constraint_to_log_space(0.25).unwrap() - expected).abs() < 1e-6);
        assert_eq!(max_drawdown_constraint_to_log_space(0.0), Some(0.0));

        assert!(max_drawdown_constraint_to_search_space(1.0, 3).is_infinite());
        assert_eq!(max_drawdown_constraint_to_log_space(1.0), None);
    }

    #[test]
    fn device_constraint_args_keep_raw_max_drawdown_for_full_metric_contracts() {
        let constraints = DoubleMaConstraints {
            max_drawdown: Some(0.25),
            ..Default::default()
        };

        let reduced =
            build_device_constraint_args(Some(&constraints), PRODUCT_REDUCED_METRICS_COUNT);
        let full17 = build_device_constraint_args(Some(&constraints), METRICS_COUNT);
        let trade22 = build_device_constraint_args(Some(&constraints), TRADE_METRICS_COUNT);

        let expected_reduced = (-(1.0f64 - 0.25).ln()) as f32;
        assert_eq!(reduced.use_max_dd, 1);
        assert!((reduced.max_dd - expected_reduced).abs() < 1e-6);

        assert_eq!(full17.use_max_dd, 1);
        assert!((full17.max_dd - 0.25).abs() < 1e-6);

        assert_eq!(trade22.use_max_dd, 1);
        assert!((trade22.max_dd - 0.25).abs() < 1e-6);
    }

    #[test]
    fn selector_harness_keeps_unused_topk_slots_invalid_across_objectives() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");
        let constraints = DoubleMaConstraints {
            min_trades: Some(2),
            max_drawdown: Some(0.50),
            ..Default::default()
        };
        let candidates = [1_i32, -1, 3, 99, 0, 2];

        for objective in 0..=6 {
            let valid = metric_row17_for_selector(0.25, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5);
            let mut failed_trades =
                metric_row17_for_selector(10.0, 1.0, 0.10, 2.0, 0.01, 0.001, 10.0);
            failed_trades[8] = 0.001;
            let failed_drawdown =
                metric_row17_for_selector(10.0, 3.0, 0.90, 2.0, 0.01, 0.001, 10.0);
            let mut nonfinite = metric_row17_for_selector(10.0, 3.0, 0.10, 2.0, 0.01, 0.001, 10.0);
            poison_objective_field(&mut nonfinite, objective);

            let rows = [valid, failed_trades, failed_drawdown, nonfinite];
            let flat = flatten_metric_rows(&rows);
            let label = format!("objective {objective}");

            let selected = launch_metric_row_selector_v2_for_test(
                &rt,
                &flat,
                rows.len(),
                objective,
                5,
                &constraints,
            )
            .unwrap_or_else(|e| panic!("direct selector failed for {label}: {e}"));
            assert_only_valid_row_selected(&label, &selected);

            let selected = launch_metric_row_candidate_selector_v2_for_test(
                &rt,
                &flat,
                rows.len(),
                &candidates,
                objective,
                5,
                &constraints,
            )
            .unwrap_or_else(|e| panic!("candidate selector failed for {label}: {e}"));
            assert_only_valid_row_selected(&format!("{label} candidates"), &selected);
        }
    }

    #[test]
    fn canonical_metric_row_topk_reference_rejects_non_finite_scores_and_ties_by_row() {
        let rows = [
            metric_row17_for_selector(0.25, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(0.75, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(f32::NAN, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(0.75, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(f32::INFINITY, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(f32::NEG_INFINITY, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(0.10, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
        ];

        let ranked = canonical_metric_row_topk_reference(&rows, None, 0, 7);
        assert_eq!(ranked, vec![1, 3, 0, 6, -1, -1, -1]);

        let candidate_ranked =
            canonical_metric_row_topk_reference(&rows, Some(&[3, -1, 99, 0, 2, 1]), 0, 5);
        assert_eq!(candidate_ranked, vec![1, 3, 0, -1, -1]);
    }

    #[test]
    fn metric_row_selector_v2_matches_reference_for_non_finite_fixture() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");
        let constraints = DoubleMaConstraints::default();
        let rows = [
            metric_row17_for_selector(0.25, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(0.75, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(f32::NAN, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(0.50, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(f32::INFINITY, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
            metric_row17_for_selector(0.10, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5),
        ];
        let flat = flatten_metric_rows(&rows);
        let top_k = 6;

        let selected =
            launch_metric_row_selector_v2_for_test(&rt, &flat, rows.len(), 0, top_k, &constraints)
                .expect("direct selector");
        let expected = canonical_metric_row_topk_reference(&rows, None, 0, top_k);
        assert_eq!(selected, expected);

        let candidates = [5_i32, -1, 3, 99, 0, 2, 4, 1];
        let selected = launch_metric_row_candidate_selector_v2_for_test(
            &rt,
            &flat,
            rows.len(),
            &candidates,
            0,
            top_k,
            &constraints,
        )
        .expect("candidate selector");
        let expected = canonical_metric_row_topk_reference(&rows, Some(&candidates), 0, top_k);
        assert_eq!(selected, expected);
    }

    #[test]
    fn metric_row_global_topk_reduction_matches_reference_across_blocks() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");

        let row_count = super::TOPK_BLOCK_ITEMS + 33;
        let mut rows =
            vec![metric_row17_for_selector(-100.0, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5); row_count];
        let winners = [
            (super::TOPK_BLOCK_ITEMS + 17, 9.0),
            (11, 8.5),
            (super::TOPK_BLOCK_ITEMS + 4, 7.0),
            (271, 6.5),
            (super::TOPK_BLOCK_ITEMS + 32, 6.0),
            (42, 5.5),
            (super::TOPK_BLOCK_ITEMS + 1, 5.0),
            (390, 4.5),
        ];
        for (row_idx, pnl) in winners {
            rows[row_idx][0] = pnl;
        }
        rows[super::TOPK_BLOCK_ITEMS + 2][0] = f32::NAN;
        rows[super::TOPK_BLOCK_ITEMS + 3][0] = f32::INFINITY;

        let flat = flatten_metric_rows(&rows);
        let fast_rows: Vec<i32> = (0..row_count).map(|idx| idx as i32).collect();
        let slow_rows: Vec<i32> = (0..row_count).map(|idx| 10_000 + idx as i32).collect();
        let d_fast = DeviceBuffer::from_slice(&fast_rows).expect("upload fast rows");
        let d_slow = DeviceBuffer::from_slice(&slow_rows).expect("upload slow rows");
        let d_metrics = DeviceBuffer::from_slice(&flat).expect("upload metric rows");
        let mut scratch = super::KernelScratch::default();
        let top_k = winners.len();
        super::KernelScratch::ensure_len_i32(&mut scratch.d_top_fast, top_k, &rt.stream)
            .expect("top fast scratch");
        super::KernelScratch::ensure_len_i32(&mut scratch.d_top_slow, top_k, &rt.stream)
            .expect("top slow scratch");
        super::KernelScratch::ensure_len_f32(
            &mut scratch.d_top_metrics,
            top_k * METRICS_COUNT,
            &rt.stream,
        )
        .expect("top metrics scratch");

        let select_rows_legacy = rt
            .module
            .get_function("select_topk_metric_rows_f32")
            .expect("legacy row selector");
        let select_rows_candidates_legacy = rt
            .module
            .get_function("select_topk_metric_rows_from_candidates_f32")
            .expect("legacy candidate row selector");
        let select_rows_v2 = rt
            .module
            .get_function("select_topk_metric_rows_f32_v2")
            .expect("v2 row selector");
        let select_rows_candidates_v2 = rt
            .module
            .get_function("select_topk_metric_rows_from_candidates_f32_v2")
            .expect("v2 candidate row selector");
        let gather_rows = rt
            .module
            .get_function("gather_topk_metric_rows_f32")
            .expect("gather rows");
        let constraints = DoubleMaConstraints::default();
        let constraint_args = build_device_constraint_args(Some(&constraints), METRICS_COUNT);
        let mut launch_submit_elapsed = std::time::Duration::ZERO;

        super::reduce_metric_rows_topk_on_device(
            &rt.stream,
            &mut scratch,
            &d_fast,
            &d_slow,
            &d_metrics,
            rows.len(),
            METRICS_COUNT,
            METRICS_COUNT,
            0,
            0.0,
            top_k,
            true,
            constraint_args,
            &select_rows_legacy,
            &select_rows_candidates_legacy,
            Some(&select_rows_v2),
            Some(&select_rows_candidates_v2),
            &gather_rows,
            &mut launch_submit_elapsed,
        )
        .expect("global metric row top-k reduction");
        rt.stream
            .synchronize()
            .expect("synchronize top-k reduction");

        let mut actual = vec![-1_i32; top_k];
        scratch
            .d_top_fast
            .as_ref()
            .expect("top fast output")
            .copy_to(&mut actual)
            .expect("copy top fast rows");

        let expected = canonical_metric_row_topk_reference(&rows, None, 0, top_k);
        assert_eq!(actual, expected);
    }

    #[test]
    fn scored_row_selector_and_gather_preserve_invalid_topk_sentinels() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");

        let scores = [0.75, f32::NAN, f32::INFINITY, -0.10, f32::NEG_INFINITY];
        let selected =
            launch_scored_row_selector_for_test(&rt, &scores, 7).expect("scored selector");
        assert_eq!(selected, canonical_scored_row_topk_reference(&scores, 7));

        let metric_row = metric_row17_for_selector(0.25, 3.0, 0.10, 0.04, 0.02, 0.01, 2.5);
        let (fast, slow, metrics) =
            launch_gather_metric_rows_for_test(&rt, &metric_row, &[0, -1, 999])
                .expect("gather metric rows");
        assert_eq!(fast, vec![5, 0, 0]);
        assert_eq!(slow, vec![20, 0, 0]);
        assert_eq!(&metrics[0..METRICS_COUNT], &metric_row);
        for value in &metrics[METRICS_COUNT..] {
            assert!(value.is_nan(), "invalid gathered metrics should be NaN");
        }
    }

    #[test]
    fn flex_m7_numeric_harness_covers_commission_clamp_and_ret1_fallback() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");

        let with_ret1 = launch_flex_m7_numeric_case_for_test(&rt, 0.0, true).expect("ret1 launch");
        let null_ret1 =
            launch_flex_m7_numeric_case_for_test(&rt, 0.0, false).expect("null ret1 launch");
        assert!(
            with_ret1[1] > 0.0,
            "numeric harness should exercise non-zero trades: {with_ret1:?}"
        );
        assert_f32_slices_close("ret1 fallback", &null_ret1, &with_ret1, 1e-6);

        let zero = launch_flex_m7_numeric_case_for_test(&rt, 0.0, true).expect("zero commission");
        let negative =
            launch_flex_m7_numeric_case_for_test(&rt, -0.25, true).expect("negative commission");
        assert_f32_slices_close("negative commission clamp", &negative, &zero, 1e-6);

        let clamp = launch_flex_m7_numeric_case_for_test(&rt, 0.95, true).expect("0.95 commission");
        let one = launch_flex_m7_numeric_case_for_test(&rt, 1.0, true).expect("1.0 commission");
        let oversized =
            launch_flex_m7_numeric_case_for_test(&rt, 2.0, true).expect("oversized commission");
        assert_f32_slices_close("1.0 commission clamp", &one, &clamp, 1e-6);
        assert_f32_slices_close("oversized commission clamp", &oversized, &clamp, 1e-6);
    }

    #[test]
    fn flex_m7_numeric_harness_keeps_extreme_log_equity_rows_finite() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");

        let positive =
            launch_flex_m7_log_equity_case_for_test(&rt, 10.0).expect("positive log-equity launch");
        assert!(
            positive[0] > 88.0,
            "positive reduced M7 row should keep log equity beyond f32 exp overflow threshold: {positive:?}"
        );
        assert!(
            positive.iter().all(|v| v.is_finite()),
            "positive extreme log-equity row should stay finite: {positive:?}"
        );

        let negative = launch_flex_m7_log_equity_case_for_test(&rt, -10.0)
            .expect("negative log-equity launch");
        assert!(
            negative[0] < -88.0,
            "negative reduced M7 row should keep large negative log equity: {negative:?}"
        );
        assert!(
            negative[2] > 88.0,
            "negative reduced M7 row should keep large drawdown in log space: {negative:?}"
        );
        assert!(
            negative.iter().all(|v| v.is_finite()),
            "negative extreme log-equity row should stay finite: {negative:?}"
        );
    }

    #[test]
    fn strategy_program_tm_m7_rejects_negative_abi_counts_without_device_fault() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");

        let metrics = launch_strategy_program_tm_m7_negative_count_case_for_test(&rt)
            .expect("negative ABI count launch");
        assert_f32_slices_close("negative ABI counts", &metrics, &[0.0; 7], 1e-7);
    }

    #[test]
    fn strategy_program_row_major_generic_threshold_matches_cpu_dynamic() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }

        let rt = super::CudaKernelRuntime::new(0).expect("create CUDA runtime");
        rt.set_current().expect("set CUDA context current");

        let len = 96usize;
        let mut lr = Vec::with_capacity(len);
        for t in 0..len {
            let step = match t % 6 {
                0 => 0.0040,
                1 => -0.0015,
                2 => 0.0030,
                3 => -0.0020,
                4 => 0.0010,
                _ => 0.0025,
            };
            lr.push(step);
        }

        let mut fast = vec![0.50f32; len];
        for v in fast.iter_mut().take(40).skip(10) {
            *v = 1.25;
        }
        let slow = vec![10.0f32; len];
        let strategy = StrategyConfig {
            trade_on_next_bar: false,
            long_only: true,
            ..StrategyConfig::default()
        };
        let plan = make_threshold_strategy_plan(strategy);
        let abi = compile_gpu_predicate_program(&plan)
            .map(|program| compile_gpu_predicate_program_abi(&program))
            .expect("compile threshold strategy ABI");

        let gpu = launch_strategy_program_row_major_threshold_case_for_test(
            &rt,
            &plan.strategy,
            &abi,
            &lr,
            &fast,
            &slow,
            1,
            len as i32,
        )
        .expect("row-major strategy program launch");
        assert!(
            gpu.iter().all(|v| v.is_finite()),
            "generic row-major strategy metrics must be finite: {gpu:?}"
        );
        assert!(
            gpu[7] > 0.0,
            "threshold strategy should produce trades, metrics={gpu:?}"
        );

        let lr_f64: Vec<f64> = lr.iter().map(|&v| f64::from(v)).collect();
        let fast_f64: Vec<f64> = fast.iter().map(|&v| f64::from(v)).collect();
        let slow_f64: Vec<f64> = slow.iter().map(|&v| f64::from(v)).collect();
        let mut close = vec![100.0f64; len];
        for t in 1..len {
            close[t] = close[t - 1] * lr_f64[t].exp();
        }
        let context = build_dynamic_cpu_context_from_precomputed(
            &lr_f64, &close, &close, None, None, None, None, None,
        );
        let cpu = eval_dynamic_plan_metrics_cpu(&plan, &context, &fast_f64, &slow_f64, 1, true)
            .expect("cpu dynamic threshold eval");

        let expected = [
            cpu.pnl,
            cpu.sharpe,
            cpu.sortino,
            cpu.calmar,
            cpu.omega,
            cpu.volatility,
            cpu.max_dd,
            f64::from(cpu.trades),
            cpu.exposure,
            cpu.net_exposure,
        ];
        let tolerances = [
            5.0e-5, 5.0e-5, 5.0e-4, 2.0e-3, 1.0e-3, 5.0e-6, 5.0e-5, 1.0e-6, 1.0e-6, 1.0e-6,
        ];
        for (idx, ((&actual, &expected), &tol)) in gpu
            .iter()
            .zip(expected.iter())
            .zip(tolerances.iter())
            .enumerate()
        {
            assert!(
                (f64::from(actual) - expected).abs() <= tol,
                "metric {idx} mismatch: gpu={actual} cpu={expected} tol={tol} row={gpu:?}"
            );
        }
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
            PRODUCT_REDUCED_METRICS_COUNT,
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
            approx_eq(gpu.omega, cpu_metrics.omega, 1e-5),
            "omega gpu={} cpu={}",
            gpu.omega,
            cpu_metrics.omega
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
    fn uploaded_dynamic_double_ma_geometry_rejects_first_valid_past_return_series() {
        let err = validate_uploaded_double_ma_dynamic_program_batch_geometry(32, 32, 2, 2, 3, 32)
            .expect_err("first_valid at lr len should be rejected");
        assert!(err.contains("first_valid 32"));
        assert!(err.contains("lr len 32"));
    }

    #[test]
    fn uploaded_dynamic_double_ma_geometry_rejects_matrix_width_mismatch() {
        let err = validate_uploaded_double_ma_dynamic_program_batch_geometry(32, 31, 2, 2, 3, 4)
            .expect_err("matrix width mismatch should be rejected");
        assert!(err.contains("stride 31"));
        assert!(err.contains("lr len 32"));
    }

    #[test]
    fn uploaded_dynamic_double_ma_geometry_rejects_i32_overflowing_counts() {
        let err = validate_uploaded_double_ma_dynamic_program_batch_geometry(
            64,
            64,
            i32::MAX as usize + 1,
            2,
            3,
            4,
        )
        .expect_err("overflowing period rows should be rejected");
        assert!(err.contains("i32 kernel limits"));

        let err = validate_uploaded_double_ma_dynamic_program_batch_geometry(
            64, 64, 50_000, 50_000, 3, 4,
        )
        .expect_err("overflowing dense pair grid should be rejected");
        assert!(err.contains("dense pair grid"));
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
            &handle,
            0,
        )
        .expect("upload dynamic batch");

        let first = super::eval_uploaded_double_ma_dynamic_program_gpu_kernel(
            &uploaded,
            &strategy,
            &handle,
            PRODUCT_REDUCED_METRICS_COUNT,
        )
        .expect("first launch");
        let second = super::eval_uploaded_double_ma_dynamic_program_gpu_kernel(
            &uploaded,
            &strategy,
            &handle,
            PRODUCT_REDUCED_METRICS_COUNT,
        )
        .expect("second launch");
        assert_eq!(first.len(), second.len());
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.trades, b.trades);
            assert!(approx_eq(a.pnl, b.pnl, 1e-7));
            assert!(approx_eq(a.sharpe, b.sharpe, 1e-7));
        }
    }

    #[test]
    fn uploaded_dynamic_double_ma_batch_rejects_mismatched_handle_executable_shape() {
        let _guard = lock_env();
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        reset_gpu_strategy_executable_cache();

        let candles = make_trending_candles(2048);
        let strategy = StrategyConfig::default();
        let base_plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "sma",
            None,
            None,
            &strategy,
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile base dynamic double-ma plan");
        let base_handle =
            StrategyHandle::from_plan(base_plan, StrategyCapabilityReport::gpu_dynamic_ready())
                .expect("build base dynamic gpu strategy handle");
        let mismatched_plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "alma",
            None,
            None,
            &strategy,
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile mismatched dynamic double-ma plan");
        let mismatched_handle = StrategyHandle::from_plan(
            mismatched_plan,
            StrategyCapabilityReport::gpu_dynamic_ready(),
        )
        .expect("build mismatched dynamic gpu strategy handle");

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
            &base_handle,
            0,
        )
        .expect("upload dynamic batch");

        let err = super::eval_uploaded_double_ma_dynamic_program_gpu_kernel(
            &uploaded,
            &strategy,
            &mismatched_handle,
            PRODUCT_REDUCED_METRICS_COUNT,
        )
        .expect_err("mismatched executable shape should be rejected");
        assert!(err.contains("executable fingerprint"));
    }
}
