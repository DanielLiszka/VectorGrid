#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    DeviceLaunchCaps, KernelLaunchCaps, LaunchShape1d,
};
use crate::options_expression::GpuFlatScoreInputs;
use cust::context::{Context, CurrentContext};
use cust::device::Device;
use cust::device::DeviceAttribute;
use cust::function::Function;
use cust::launch;
use cust::memory::{mem_get_info, CopyDestination, DeviceBuffer, DevicePointer};
use cust::module::{Module, ModuleJitOption, OptLevel};
use cust::prelude::{CudaFlags, Stream, StreamFlags};
use std::collections::BTreeMap;
use std::mem::size_of;
use std::sync::{Arc, Mutex, OnceLock};

const SCORE_KERNEL_NAME: &str = "options_expression_score_split_f64";
const SCORE_COMBINED_KERNEL_NAME: &str = "options_expression_score_search_holdout_stress_f64";
const GPU_INPUT_CACHE_MAX_ENTRIES: usize = 16;
const GPU_TUPLE_CHUNK_BUDGET_CAP_BYTES: u64 = 64 * 1024 * 1024;
const GPU_TUPLE_CHUNK_BUDGET_FLOOR_BYTES: u64 = 4 * 1024 * 1024;
const GPU_TUPLE_CHUNK_BUDGET_TOTAL_MEMORY_DIVISOR: u64 = 16;
const GPU_TUPLE_CHUNK_MEMORY_HEADROOM_BYTES: u64 = 256 * 1024 * 1024;
const GPU_TUPLE_CHUNK_HARD_CAP_TUPLES: usize = 8_192;

fn device_compute_capability(device: Device) -> Option<(u32, u32)> {
    let major = device
        .get_attribute(DeviceAttribute::ComputeCapabilityMajor)
        .ok()
        .and_then(|value| u32::try_from(value).ok())?;
    let minor = device
        .get_attribute(DeviceAttribute::ComputeCapabilityMinor)
        .ok()
        .and_then(|value| u32::try_from(value).ok())?;
    Some((major, minor))
}

fn load_options_expression_module(device: Device) -> Result<Module, String> {
    if device_compute_capability(device) == Some((8, 9)) {
        let cubin: &[u8] =
            include_bytes!(concat!(env!("OUT_DIR"), "/options_expression.sm_89.cubin"));
        if !cubin.is_empty() {
            if let Ok(module) = Module::from_cubin(cubin, &[]) {
                return Ok(module);
            }
        }
    }

    let ptx = include_str!(concat!(env!("OUT_DIR"), "/options_expression.ptx"));
    let jit_opts = &[
        ModuleJitOption::DetermineTargetFromContext,
        ModuleJitOption::OptLevel(OptLevel::O2),
    ];
    Module::from_ptx(ptx, jit_opts)
        .or_else(|_| Module::from_ptx(ptx, &[ModuleJitOption::DetermineTargetFromContext]))
        .or_else(|_| Module::from_ptx(ptx, &[]))
        .map_err(|e| e.to_string())
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct OptionsGpuTupleAggregate {
    pub(crate) valid_trades: u32,
    pub(crate) wins: u32,
    pub(crate) total_pnl: f32,
    pub(crate) total_premium: f32,
    pub(crate) sum_return_on_premium: f32,
    pub(crate) sumsq_return_on_premium: f32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OptionsGpuRuntimeCacheUsage {
    pub(crate) shared_input_cache_hits: usize,
    pub(crate) shared_input_cache_misses: usize,
    pub(crate) outcome_input_cache_hits: usize,
    pub(crate) outcome_input_cache_misses: usize,
    pub(crate) output_buffer_reuses: usize,
    pub(crate) output_buffer_allocations: usize,
    pub(crate) tuple_chunk_count: usize,
    pub(crate) tuple_chunk_size: usize,
    pub(crate) kernel_launch_count: usize,
    pub(crate) gpu_tuple_budget_bytes: usize,
    pub(crate) gpu_outcome_tensor_bytes: usize,
    pub(crate) gpu_shared_input_bytes_per_chunk: usize,
    pub(crate) gpu_output_bytes_per_chunk: usize,
    pub(crate) gpu_total_device_bytes_per_chunk: usize,
    pub(crate) gpu_device_total_memory_bytes: usize,
    pub(crate) gpu_device_free_memory_bytes: usize,
    pub(crate) gpu_device_usable_memory_bytes: usize,
    pub(crate) gpu_memory_budget_source: String,
    pub(crate) gpu_outcome_tensor_fits_in_memory: bool,
    pub(crate) gpu_chunk_plan_fits_in_memory: bool,
    pub(crate) gpu_plan_notes: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OptionsGpuTensorRun {
    pub(crate) aggregates_by_range: Vec<Vec<OptionsGpuTupleAggregate>>,
    pub(crate) cache_usage: OptionsGpuRuntimeCacheUsage,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OptionsGpuCombinedTensorRun {
    pub(crate) search_aggregates: Vec<OptionsGpuTupleAggregate>,
    pub(crate) holdout_aggregates: Vec<OptionsGpuTupleAggregate>,
    pub(crate) conservative_holdout_aggregates: Vec<OptionsGpuTupleAggregate>,
    pub(crate) harsher_holdout_aggregates: Vec<OptionsGpuTupleAggregate>,
    pub(crate) robust_scores: Vec<f64>,
    pub(crate) cache_usage: OptionsGpuRuntimeCacheUsage,
}

struct UploadedSharedInputs {
    d_tuple_candidate_offsets: DeviceBuffer<u32>,
    d_tuple_summary_offsets: DeviceBuffer<u32>,
    tuple_count: usize,
    tuple_count_u32: u32,
    candidate_slot_stride_u32: u32,
    summary_stride_u32: u32,
}

struct UploadedOutcomeInputs {
    d_outcome_valid: DeviceBuffer<u8>,
    d_outcome_pnl: DeviceBuffer<f32>,
    d_outcome_premium: DeviceBuffer<f32>,
}

struct ReusableOutputBuffers {
    d_out_valid: DeviceBuffer<i32>,
    d_out_wins: DeviceBuffer<i32>,
    d_out_total_pnl: DeviceBuffer<f32>,
    d_out_total_premium: DeviceBuffer<f32>,
    d_out_sum_return_on_premium: DeviceBuffer<f32>,
    d_out_sumsq_return_on_premium: DeviceBuffer<f32>,
    host_valid: Vec<i32>,
    host_wins: Vec<i32>,
    host_total_pnl: Vec<f32>,
    host_total_premium: Vec<f32>,
    host_sum_return_on_premium: Vec<f32>,
    host_sumsq_return_on_premium: Vec<f32>,
}

struct CombinedReusableOutputBuffers {
    search: ReusableOutputBuffers,
    holdout: ReusableOutputBuffers,
    conservative_holdout: ReusableOutputBuffers,
    harsher_holdout: ReusableOutputBuffers,
    d_out_robust_score: DeviceBuffer<f64>,
    host_robust_score: Vec<f64>,
}

#[derive(Clone, Copy)]
struct SharedKernelLaunchArgs {
    tuple_count: usize,
    tuple_count_u32: u32,
    candidate_slot_stride_u32: u32,
    summary_stride_u32: u32,
    tuple_candidate_offsets_ptr: DevicePointer<u32>,
    tuple_summary_offsets_ptr: DevicePointer<u32>,
}

#[derive(Clone, Copy)]
struct OutcomeKernelLaunchArgs {
    outcome_valid_ptr: DevicePointer<u8>,
    outcome_pnl_ptr: DevicePointer<f32>,
    outcome_premium_ptr: DevicePointer<f32>,
}

#[derive(Clone, Debug, Default)]
struct OptionsGpuChunkPlan {
    tuple_chunk_size: usize,
    tuple_chunk_count: usize,
    gpu_tuple_budget_bytes: usize,
    gpu_outcome_tensor_bytes: usize,
    gpu_shared_input_bytes_per_chunk: usize,
    gpu_output_bytes_per_chunk: usize,
    gpu_total_device_bytes_per_chunk: usize,
    gpu_device_total_memory_bytes: usize,
    gpu_device_free_memory_bytes: usize,
    gpu_device_usable_memory_bytes: usize,
    gpu_memory_budget_source: String,
    gpu_outcome_tensor_fits_in_memory: bool,
    gpu_chunk_plan_fits_in_memory: bool,
    gpu_plan_notes: Vec<String>,
}

fn per_tuple_shared_input_device_bytes() -> usize {
    size_of::<u32>() * 2
}

fn per_tuple_output_device_bytes() -> usize {
    size_of::<i32>() * 2 + size_of::<f32>() * 4
}

fn outcome_tensor_device_bytes(inputs: &GpuFlatScoreInputs) -> usize {
    inputs.outcome_valid.len() * size_of::<u8>()
        + inputs.outcome_pnl.len() * size_of::<f32>()
        + inputs.outcome_premium.len() * size_of::<f32>()
}

fn combined_outcome_tensor_device_bytes(inputs: &[&GpuFlatScoreInputs]) -> usize {
    inputs
        .iter()
        .map(|lane_inputs| outcome_tensor_device_bytes(lane_inputs))
        .sum()
}

fn usable_memory_bytes_from_free(free_memory_bytes: u64) -> u64 {
    let usable = free_memory_bytes.saturating_sub(GPU_TUPLE_CHUNK_MEMORY_HEADROOM_BYTES);
    if usable > 0 {
        return usable;
    }
    free_memory_bytes / 2
}

fn tuple_budget_bytes(
    total_memory_bytes: u64,
    usable_memory_bytes: Option<u64>,
) -> (usize, String) {
    let fraction = total_memory_bytes / GPU_TUPLE_CHUNK_BUDGET_TOTAL_MEMORY_DIVISOR;
    let clamped = fraction
        .max(GPU_TUPLE_CHUNK_BUDGET_FLOOR_BYTES)
        .min(GPU_TUPLE_CHUNK_BUDGET_CAP_BYTES);
    let total_budget = usize::try_from(clamped).unwrap_or(usize::MAX);
    match usable_memory_bytes {
        Some(usable) => {
            let usable_budget = usize::try_from(usable).unwrap_or(usize::MAX);
            (
                total_budget.min(usable_budget),
                "free_memory_headroom".to_string(),
            )
        }
        None => (total_budget, "total_memory_fallback".to_string()),
    }
}

fn configured_tuple_chunk_cap() -> usize {
    std::env::var("VECTORGRID_OPTIONS_GPU_TUPLE_CHUNK_CAP")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(GPU_TUPLE_CHUNK_HARD_CAP_TUPLES)
        .max(1)
}

impl OptionsGpuChunkPlan {
    fn for_layout(
        tuple_count: usize,
        total_memory_bytes: u64,
        free_memory_bytes: Option<u64>,
        gpu_outcome_tensor_bytes: usize,
        shared_input_lane_count: usize,
        aggregate_lane_count: usize,
        extra_output_bytes_per_tuple: usize,
    ) -> Result<Self, String> {
        if tuple_count == 0 {
            return Ok(Self::default());
        }

        let per_tuple_device_bytes = per_tuple_shared_input_device_bytes()
            .saturating_mul(shared_input_lane_count)
            .saturating_add(per_tuple_output_device_bytes().saturating_mul(aggregate_lane_count))
            .saturating_add(extra_output_bytes_per_tuple);
        let mut gpu_plan_notes = Vec::new();
        let gpu_device_free_memory_bytes = free_memory_bytes
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let gpu_device_usable_memory_bytes = free_memory_bytes
            .map(usable_memory_bytes_from_free)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let (mut tuple_budget, gpu_memory_budget_source) = tuple_budget_bytes(
            total_memory_bytes,
            free_memory_bytes.map(usable_memory_bytes_from_free),
        );
        if free_memory_bytes.is_none() {
            gpu_plan_notes.push(
                "GPU free-memory probe was unavailable; tuple chunk planning fell back to total device memory heuristics."
                    .to_string(),
            );
        }
        let gpu_outcome_tensor_fits_in_memory = free_memory_bytes
            .map(|free| gpu_outcome_tensor_bytes <= usable_memory_bytes_from_free(free) as usize)
            .unwrap_or(true);
        if !gpu_outcome_tensor_fits_in_memory {
            let free_bytes = gpu_device_free_memory_bytes.max(1);
            let usable_bytes = gpu_device_usable_memory_bytes.max(1);
            return Err(format!(
                "GPU options-expression outcome tensor requires {} bytes, but only {} bytes are currently free ({} usable after headroom).",
                gpu_outcome_tensor_bytes, free_bytes, usable_bytes
            ));
        }
        if let Some(free) = free_memory_bytes {
            let usable = usable_memory_bytes_from_free(free);
            let remaining = usize::try_from(usable).unwrap_or(usize::MAX);
            tuple_budget = tuple_budget.min(remaining.saturating_sub(gpu_outcome_tensor_bytes));
        }
        if tuple_budget < per_tuple_device_bytes {
            return Err(format!(
                "GPU options-expression tuple chunk budget {} bytes cannot fit even one tuple working set of {} bytes after accounting for the outcome tensor.",
                tuple_budget, per_tuple_device_bytes
            ));
        }
        let tuple_chunk_size = (tuple_budget / per_tuple_device_bytes)
            .max(1)
            .min(configured_tuple_chunk_cap())
            .min(tuple_count);
        let tuple_chunk_count = tuple_count.div_ceil(tuple_chunk_size);
        let gpu_shared_input_bytes_per_chunk = tuple_chunk_size
            .saturating_mul(per_tuple_shared_input_device_bytes())
            .saturating_mul(shared_input_lane_count);
        let gpu_output_bytes_per_chunk = tuple_chunk_size.saturating_mul(
            per_tuple_output_device_bytes()
                .saturating_mul(aggregate_lane_count)
                .saturating_add(extra_output_bytes_per_tuple),
        );
        let gpu_total_device_bytes_per_chunk = gpu_outcome_tensor_bytes
            .saturating_add(gpu_shared_input_bytes_per_chunk)
            .saturating_add(gpu_output_bytes_per_chunk);
        let gpu_chunk_plan_fits_in_memory = free_memory_bytes
            .map(|free| {
                gpu_total_device_bytes_per_chunk <= usable_memory_bytes_from_free(free) as usize
            })
            .unwrap_or(true);
        if !gpu_chunk_plan_fits_in_memory {
            return Err(format!(
                "GPU options-expression chunk plan requires {} bytes per launch, but the current usable device budget is {} bytes after headroom.",
                gpu_total_device_bytes_per_chunk, gpu_device_usable_memory_bytes
            ));
        }

        Ok(Self {
            tuple_chunk_size,
            tuple_chunk_count,
            gpu_tuple_budget_bytes: tuple_budget,
            gpu_outcome_tensor_bytes,
            gpu_shared_input_bytes_per_chunk,
            gpu_output_bytes_per_chunk,
            gpu_total_device_bytes_per_chunk,
            gpu_device_total_memory_bytes: usize::try_from(total_memory_bytes)
                .unwrap_or(usize::MAX),
            gpu_device_free_memory_bytes,
            gpu_device_usable_memory_bytes,
            gpu_memory_budget_source,
            gpu_outcome_tensor_fits_in_memory,
            gpu_chunk_plan_fits_in_memory,
            gpu_plan_notes,
        })
    }

    fn for_inputs(
        tuple_count: usize,
        total_memory_bytes: u64,
        free_memory_bytes: Option<u64>,
        inputs: &GpuFlatScoreInputs,
    ) -> Result<Self, String> {
        Self::for_layout(
            tuple_count,
            total_memory_bytes,
            free_memory_bytes,
            outcome_tensor_device_bytes(inputs),
            1,
            1,
            0,
        )
    }

    fn for_combined_inputs(
        tuple_count: usize,
        total_memory_bytes: u64,
        free_memory_bytes: Option<u64>,
        baseline_inputs: &GpuFlatScoreInputs,
        conservative_inputs: &GpuFlatScoreInputs,
        harsher_inputs: &GpuFlatScoreInputs,
    ) -> Result<Self, String> {
        Self::for_layout(
            tuple_count,
            total_memory_bytes,
            free_memory_bytes,
            combined_outcome_tensor_device_bytes(&[
                baseline_inputs,
                conservative_inputs,
                harsher_inputs,
            ]),
            3,
            4,
            size_of::<f64>(),
        )
    }
}

impl UploadedSharedInputs {
    fn upload_chunk(
        candidate_slot_stride: usize,
        summary_stride: usize,
        tuple_candidate_offsets: &[u32],
        tuple_summary_offsets: &[u32],
        tuple_start: usize,
        tuple_end: usize,
    ) -> Result<Self, String> {
        if tuple_candidate_offsets.len() != tuple_summary_offsets.len() {
            return Err("GPU options-expression tuple offset lengths do not match".to_string());
        }
        let candidate_offsets = tuple_candidate_offsets
            .get(tuple_start..tuple_end)
            .ok_or_else(|| "GPU options-expression tuple chunk is out of bounds".to_string())?;
        let summary_offsets = tuple_summary_offsets
            .get(tuple_start..tuple_end)
            .ok_or_else(|| "GPU options-expression tuple chunk is out of bounds".to_string())?;
        if candidate_offsets.len() != summary_offsets.len() {
            return Err("GPU options-expression tuple chunk lengths do not match".to_string());
        }
        let tuple_count = candidate_offsets.len();
        let tuple_count_u32 =
            u32::try_from(tuple_count).map_err(|_| "tuple_count exceeds u32::MAX".to_string())?;
        let candidate_slot_stride_u32 = u32::try_from(candidate_slot_stride)
            .map_err(|_| "candidate_slot_stride exceeds u32::MAX".to_string())?;
        let summary_stride_u32 = u32::try_from(summary_stride)
            .map_err(|_| "summary_stride exceeds u32::MAX".to_string())?;

        Ok(Self {
            d_tuple_candidate_offsets: DeviceBuffer::from_slice(candidate_offsets)
                .map_err(|e| e.to_string())?,
            d_tuple_summary_offsets: DeviceBuffer::from_slice(summary_offsets)
                .map_err(|e| e.to_string())?,
            tuple_count,
            tuple_count_u32,
            candidate_slot_stride_u32,
            summary_stride_u32,
        })
    }
}

impl UploadedOutcomeInputs {
    fn upload(
        outcome_valid: &[u8],
        outcome_pnl: &[f32],
        outcome_premium: &[f32],
    ) -> Result<Self, String> {
        if outcome_valid.len() != outcome_pnl.len() || outcome_valid.len() != outcome_premium.len()
        {
            return Err("GPU options-expression outcome tensor lengths do not match".to_string());
        }

        Ok(Self {
            d_outcome_valid: DeviceBuffer::from_slice(outcome_valid).map_err(|e| e.to_string())?,
            d_outcome_pnl: DeviceBuffer::from_slice(outcome_pnl).map_err(|e| e.to_string())?,
            d_outcome_premium: DeviceBuffer::from_slice(outcome_premium)
                .map_err(|e| e.to_string())?,
        })
    }
}

impl ReusableOutputBuffers {
    fn new(tuple_count: usize) -> Result<Self, String> {
        Ok(Self {
            // Safety: the score kernel writes every output index in `[0, tuple_count)` before any
            // device-to-host copy, so there is no observable read of uninitialized device memory.
            d_out_valid: unsafe { DeviceBuffer::<i32>::uninitialized(tuple_count) }
                .map_err(|e| e.to_string())?,
            // Safety: same full-write guarantee as `d_out_valid`.
            d_out_wins: unsafe { DeviceBuffer::<i32>::uninitialized(tuple_count) }
                .map_err(|e| e.to_string())?,
            // Safety: same full-write guarantee as `d_out_valid`.
            d_out_total_pnl: unsafe { DeviceBuffer::<f32>::uninitialized(tuple_count) }
                .map_err(|e| e.to_string())?,
            // Safety: same full-write guarantee as `d_out_valid`.
            d_out_total_premium: unsafe { DeviceBuffer::<f32>::uninitialized(tuple_count) }
                .map_err(|e| e.to_string())?,
            // Safety: same full-write guarantee as `d_out_valid`.
            d_out_sum_return_on_premium: unsafe { DeviceBuffer::<f32>::uninitialized(tuple_count) }
                .map_err(|e| e.to_string())?,
            // Safety: same full-write guarantee as `d_out_valid`.
            d_out_sumsq_return_on_premium: unsafe {
                DeviceBuffer::<f32>::uninitialized(tuple_count)
            }
            .map_err(|e| e.to_string())?,
            host_valid: vec![0i32; tuple_count],
            host_wins: vec![0i32; tuple_count],
            host_total_pnl: vec![0.0f32; tuple_count],
            host_total_premium: vec![0.0f32; tuple_count],
            host_sum_return_on_premium: vec![0.0f32; tuple_count],
            host_sumsq_return_on_premium: vec![0.0f32; tuple_count],
        })
    }

    fn copy_to_host(&mut self) -> Result<(), String> {
        self.d_out_valid
            .copy_to(&mut self.host_valid)
            .map_err(|e| e.to_string())?;
        self.d_out_wins
            .copy_to(&mut self.host_wins)
            .map_err(|e| e.to_string())?;
        self.d_out_total_pnl
            .copy_to(&mut self.host_total_pnl)
            .map_err(|e| e.to_string())?;
        self.d_out_total_premium
            .copy_to(&mut self.host_total_premium)
            .map_err(|e| e.to_string())?;
        self.d_out_sum_return_on_premium
            .copy_to(&mut self.host_sum_return_on_premium)
            .map_err(|e| e.to_string())?;
        self.d_out_sumsq_return_on_premium
            .copy_to(&mut self.host_sumsq_return_on_premium)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn aggregates_prefix(&self, tuple_count: usize) -> Vec<OptionsGpuTupleAggregate> {
        (0..tuple_count.min(self.host_valid.len()))
            .map(|index| OptionsGpuTupleAggregate {
                valid_trades: self.host_valid[index].max(0) as u32,
                wins: self.host_wins[index].max(0) as u32,
                total_pnl: self.host_total_pnl[index],
                total_premium: self.host_total_premium[index],
                sum_return_on_premium: self.host_sum_return_on_premium[index],
                sumsq_return_on_premium: self.host_sumsq_return_on_premium[index],
            })
            .collect()
    }

    fn tuple_count(&self) -> usize {
        self.host_valid.len()
    }
}

impl CombinedReusableOutputBuffers {
    fn new(tuple_count: usize) -> Result<Self, String> {
        Ok(Self {
            search: ReusableOutputBuffers::new(tuple_count)?,
            holdout: ReusableOutputBuffers::new(tuple_count)?,
            conservative_holdout: ReusableOutputBuffers::new(tuple_count)?,
            harsher_holdout: ReusableOutputBuffers::new(tuple_count)?,
            // Safety: the combined score kernel writes every score element before any copy.
            d_out_robust_score: unsafe { DeviceBuffer::<f64>::uninitialized(tuple_count) }
                .map_err(|e| e.to_string())?,
            host_robust_score: vec![0.0f64; tuple_count],
        })
    }

    fn copy_to_host(&mut self) -> Result<(), String> {
        self.search.copy_to_host()?;
        self.holdout.copy_to_host()?;
        self.conservative_holdout.copy_to_host()?;
        self.harsher_holdout.copy_to_host()?;
        self.d_out_robust_score
            .copy_to(&mut self.host_robust_score)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn tuple_count(&self) -> usize {
        self.search.tuple_count()
    }

    fn robust_scores_prefix(&self, tuple_count: usize) -> Vec<f64> {
        self.host_robust_score[..tuple_count.min(self.host_robust_score.len())].to_vec()
    }
}

struct OptionsExpressionGpuRuntime {
    context: Context,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    device_total_memory_bytes: u64,
    kernel_caps: BTreeMap<&'static str, KernelLaunchCaps>,
    shared_inputs_cache: BTreeMap<String, UploadedSharedInputs>,
    outcome_inputs_cache: BTreeMap<String, UploadedOutcomeInputs>,
    reusable_output_buffers: Option<ReusableOutputBuffers>,
    reusable_combined_output_buffers: Option<CombinedReusableOutputBuffers>,
}

impl OptionsExpressionGpuRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let device_total_memory_bytes = device.total_memory().map_err(|e| e.to_string())? as u64;
        let launch_caps = detect_device_launch_caps(device);
        let context = Context::new(device).map_err(|e| e.to_string())?;
        CurrentContext::set_current(&context).map_err(|e| e.to_string())?;
        let module = load_options_expression_module(device)?;
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;

        Ok(Self {
            context,
            module,
            stream,
            launch_caps,
            device_total_memory_bytes,
            kernel_caps: BTreeMap::new(),
            shared_inputs_cache: BTreeMap::new(),
            outcome_inputs_cache: BTreeMap::new(),
            reusable_output_buffers: None,
            reusable_combined_output_buffers: None,
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(&self.context).map_err(|e| e.to_string())
    }

    fn kernel_caps(&mut self, kernel_name: &'static str) -> Result<KernelLaunchCaps, String> {
        if let Some(cached) = self.kernel_caps.get(kernel_name).copied() {
            return Ok(cached);
        }
        let func = self
            .module
            .get_function(kernel_name)
            .map_err(|e| e.to_string())?;
        let caps = detect_kernel_launch_caps(&func, self.launch_caps);
        self.kernel_caps.insert(kernel_name, caps);
        Ok(caps)
    }

    fn ensure_shared_inputs_chunk(
        &mut self,
        inputs: &GpuFlatScoreInputs,
        tuple_start: usize,
        tuple_end: usize,
        cache_usage: &mut OptionsGpuRuntimeCacheUsage,
    ) -> Result<String, String> {
        let chunk_key = format!("{}|tuple={tuple_start}:{tuple_end}", inputs.shared_key);
        if self.shared_inputs_cache.contains_key(&chunk_key) {
            cache_usage.shared_input_cache_hits += 1;
            return Ok(chunk_key);
        }
        cache_usage.shared_input_cache_misses += 1;
        let uploaded = UploadedSharedInputs::upload_chunk(
            inputs.candidate_slot_stride,
            inputs.summary_stride,
            &inputs.tuple_candidate_offsets,
            &inputs.tuple_summary_offsets,
            tuple_start,
            tuple_end,
        )?;
        evict_oldest_if_needed(&mut self.shared_inputs_cache, GPU_INPUT_CACHE_MAX_ENTRIES);
        self.shared_inputs_cache.insert(chunk_key.clone(), uploaded);
        Ok(chunk_key)
    }

    fn ensure_outcome_inputs(
        &mut self,
        inputs: &GpuFlatScoreInputs,
        cache_usage: &mut OptionsGpuRuntimeCacheUsage,
    ) -> Result<(), String> {
        if self.outcome_inputs_cache.contains_key(&inputs.outcome_key) {
            cache_usage.outcome_input_cache_hits += 1;
            return Ok(());
        }
        cache_usage.outcome_input_cache_misses += 1;
        let uploaded = UploadedOutcomeInputs::upload(
            &inputs.outcome_valid,
            &inputs.outcome_pnl,
            &inputs.outcome_premium,
        )?;
        evict_oldest_if_needed(&mut self.outcome_inputs_cache, GPU_INPUT_CACHE_MAX_ENTRIES);
        self.outcome_inputs_cache
            .insert(inputs.outcome_key.clone(), uploaded);
        Ok(())
    }

    fn ensure_output_buffers(
        &mut self,
        tuple_count: usize,
        cache_usage: &mut OptionsGpuRuntimeCacheUsage,
    ) -> Result<(), String> {
        let needs_allocation = self
            .reusable_output_buffers
            .as_ref()
            .map(|buffers| buffers.tuple_count() < tuple_count)
            .unwrap_or(true);
        if needs_allocation {
            self.reusable_output_buffers = Some(ReusableOutputBuffers::new(tuple_count)?);
            cache_usage.output_buffer_allocations += 1;
        } else {
            cache_usage.output_buffer_reuses += 1;
        }
        Ok(())
    }

    fn ensure_combined_output_buffers(
        &mut self,
        tuple_count: usize,
        cache_usage: &mut OptionsGpuRuntimeCacheUsage,
    ) -> Result<(), String> {
        let needs_allocation = self
            .reusable_combined_output_buffers
            .as_ref()
            .map(|buffers| buffers.tuple_count() < tuple_count)
            .unwrap_or(true);
        if needs_allocation {
            self.reusable_combined_output_buffers =
                Some(CombinedReusableOutputBuffers::new(tuple_count)?);
            cache_usage.output_buffer_allocations += 1;
        } else {
            cache_usage.output_buffer_reuses += 1;
        }
        Ok(())
    }

    fn score_ranges(
        &mut self,
        inputs: &GpuFlatScoreInputs,
        event_ranges: &[(usize, usize)],
    ) -> Result<OptionsGpuTensorRun, String> {
        self.set_current()?;
        let mut cache_usage = OptionsGpuRuntimeCacheUsage::default();
        if inputs.tuple_candidate_offsets.len() != inputs.tuple_summary_offsets.len() {
            return Err("GPU options-expression tuple offset lengths do not match".to_string());
        }
        let tuple_count = inputs.tuple_candidate_offsets.len();
        let free_memory_bytes = mem_get_info().ok().map(|(free, _total)| free as u64);
        let chunk_plan = OptionsGpuChunkPlan::for_inputs(
            tuple_count,
            self.device_total_memory_bytes,
            free_memory_bytes,
            inputs,
        )?;
        cache_usage.tuple_chunk_count = chunk_plan.tuple_chunk_count;
        cache_usage.tuple_chunk_size = chunk_plan.tuple_chunk_size;
        cache_usage.gpu_tuple_budget_bytes = chunk_plan.gpu_tuple_budget_bytes;
        cache_usage.gpu_outcome_tensor_bytes = chunk_plan.gpu_outcome_tensor_bytes;
        cache_usage.gpu_shared_input_bytes_per_chunk = chunk_plan.gpu_shared_input_bytes_per_chunk;
        cache_usage.gpu_output_bytes_per_chunk = chunk_plan.gpu_output_bytes_per_chunk;
        cache_usage.gpu_total_device_bytes_per_chunk = chunk_plan.gpu_total_device_bytes_per_chunk;
        cache_usage.gpu_device_total_memory_bytes = chunk_plan.gpu_device_total_memory_bytes;
        cache_usage.gpu_device_free_memory_bytes = chunk_plan.gpu_device_free_memory_bytes;
        cache_usage.gpu_device_usable_memory_bytes = chunk_plan.gpu_device_usable_memory_bytes;
        cache_usage.gpu_memory_budget_source = chunk_plan.gpu_memory_budget_source.clone();
        cache_usage.gpu_outcome_tensor_fits_in_memory =
            chunk_plan.gpu_outcome_tensor_fits_in_memory;
        cache_usage.gpu_chunk_plan_fits_in_memory = chunk_plan.gpu_chunk_plan_fits_in_memory;
        cache_usage.gpu_plan_notes = chunk_plan.gpu_plan_notes.clone();
        self.ensure_outcome_inputs(inputs, &mut cache_usage)?;
        let kernel_caps = self.kernel_caps(SCORE_KERNEL_NAME)?;

        let outcome_args = self
            .outcome_inputs_cache
            .get(&inputs.outcome_key)
            .map(|outcomes| OutcomeKernelLaunchArgs {
                outcome_valid_ptr: outcomes.d_outcome_valid.as_device_ptr(),
                outcome_pnl_ptr: outcomes.d_outcome_pnl.as_device_ptr(),
                outcome_premium_ptr: outcomes.d_outcome_premium.as_device_ptr(),
            })
            .ok_or_else(|| "missing cached GPU outcome inputs".to_string())?;
        let mut aggregates_by_range = Vec::with_capacity(event_ranges.len());
        for &(event_start, event_end) in event_ranges {
            let event_start_u32 = u32::try_from(event_start)
                .map_err(|_| "event_start exceeds u32::MAX".to_string())?;
            let event_end_u32 =
                u32::try_from(event_end).map_err(|_| "event_end exceeds u32::MAX".to_string())?;
            let mut range_aggregates = Vec::with_capacity(tuple_count);
            for chunk_index in 0..chunk_plan.tuple_chunk_count.max(1) {
                let tuple_start = chunk_index.saturating_mul(chunk_plan.tuple_chunk_size);
                let tuple_end = (tuple_start + chunk_plan.tuple_chunk_size).min(tuple_count);
                let shared_key = self.ensure_shared_inputs_chunk(
                    inputs,
                    tuple_start,
                    tuple_end,
                    &mut cache_usage,
                )?;
                let shared_args = self
                    .shared_inputs_cache
                    .get(&shared_key)
                    .map(|shared| SharedKernelLaunchArgs {
                        tuple_count: shared.tuple_count,
                        tuple_count_u32: shared.tuple_count_u32,
                        candidate_slot_stride_u32: shared.candidate_slot_stride_u32,
                        summary_stride_u32: shared.summary_stride_u32,
                        tuple_candidate_offsets_ptr: shared
                            .d_tuple_candidate_offsets
                            .as_device_ptr(),
                        tuple_summary_offsets_ptr: shared.d_tuple_summary_offsets.as_device_ptr(),
                    })
                    .ok_or_else(|| "missing cached GPU shared inputs".to_string())?;
                let launch = choose_launch_shape_1d_from_candidates(
                    shared_args.tuple_count,
                    self.launch_caps,
                    kernel_caps,
                    &[64u32, 128, 256],
                );
                self.ensure_output_buffers(shared_args.tuple_count, &mut cache_usage)?;
                let mut outputs = self
                    .reusable_output_buffers
                    .take()
                    .ok_or_else(|| "missing reusable GPU output buffers".to_string())?;
                let kernel = self
                    .module
                    .get_function(SCORE_KERNEL_NAME)
                    .map_err(|e| e.to_string())?;
                cache_usage.kernel_launch_count += 1;
                let chunk_aggregates = launch_score_range(
                    &self.stream,
                    &kernel,
                    launch,
                    shared_args,
                    outcome_args,
                    event_start_u32,
                    event_end_u32,
                    &mut outputs,
                )?;
                self.reusable_output_buffers = Some(outputs);
                range_aggregates.extend(chunk_aggregates);
            }
            aggregates_by_range.push(range_aggregates);
        }
        Ok(OptionsGpuTensorRun {
            aggregates_by_range,
            cache_usage,
        })
    }

    fn score_search_holdout_stress(
        &mut self,
        search_end: usize,
        holdout_end: usize,
        baseline_inputs: &GpuFlatScoreInputs,
        conservative_inputs: &GpuFlatScoreInputs,
        harsher_inputs: &GpuFlatScoreInputs,
    ) -> Result<OptionsGpuCombinedTensorRun, String> {
        self.set_current()?;
        if baseline_inputs.tuple_candidate_offsets.len()
            != baseline_inputs.tuple_summary_offsets.len()
            || conservative_inputs.tuple_candidate_offsets.len()
                != conservative_inputs.tuple_summary_offsets.len()
            || harsher_inputs.tuple_candidate_offsets.len()
                != harsher_inputs.tuple_summary_offsets.len()
        {
            return Err("GPU options-expression tuple offset lengths do not match".to_string());
        }
        let tuple_count = baseline_inputs.tuple_candidate_offsets.len();
        if conservative_inputs.tuple_candidate_offsets.len() != tuple_count
            || harsher_inputs.tuple_candidate_offsets.len() != tuple_count
        {
            return Err("GPU options-expression combined scorer requires identical tuple counts across lanes".to_string());
        }
        let search_end_u32 =
            u32::try_from(search_end).map_err(|_| "search_end exceeds u32::MAX".to_string())?;
        let holdout_end_u32 =
            u32::try_from(holdout_end).map_err(|_| "holdout_end exceeds u32::MAX".to_string())?;
        let free_memory_bytes = mem_get_info().ok().map(|(free, _total)| free as u64);
        let chunk_plan = OptionsGpuChunkPlan::for_combined_inputs(
            tuple_count,
            self.device_total_memory_bytes,
            free_memory_bytes,
            baseline_inputs,
            conservative_inputs,
            harsher_inputs,
        )?;
        let mut cache_usage = OptionsGpuRuntimeCacheUsage {
            tuple_chunk_count: chunk_plan.tuple_chunk_count,
            tuple_chunk_size: chunk_plan.tuple_chunk_size,
            gpu_tuple_budget_bytes: chunk_plan.gpu_tuple_budget_bytes,
            gpu_outcome_tensor_bytes: chunk_plan.gpu_outcome_tensor_bytes,
            gpu_shared_input_bytes_per_chunk: chunk_plan.gpu_shared_input_bytes_per_chunk,
            gpu_output_bytes_per_chunk: chunk_plan.gpu_output_bytes_per_chunk,
            gpu_total_device_bytes_per_chunk: chunk_plan.gpu_total_device_bytes_per_chunk,
            gpu_device_total_memory_bytes: chunk_plan.gpu_device_total_memory_bytes,
            gpu_device_free_memory_bytes: chunk_plan.gpu_device_free_memory_bytes,
            gpu_device_usable_memory_bytes: chunk_plan.gpu_device_usable_memory_bytes,
            gpu_memory_budget_source: chunk_plan.gpu_memory_budget_source.clone(),
            gpu_outcome_tensor_fits_in_memory: chunk_plan.gpu_outcome_tensor_fits_in_memory,
            gpu_chunk_plan_fits_in_memory: chunk_plan.gpu_chunk_plan_fits_in_memory,
            gpu_plan_notes: chunk_plan.gpu_plan_notes.clone(),
            ..OptionsGpuRuntimeCacheUsage::default()
        };

        self.ensure_outcome_inputs(baseline_inputs, &mut cache_usage)?;
        self.ensure_outcome_inputs(conservative_inputs, &mut cache_usage)?;
        self.ensure_outcome_inputs(harsher_inputs, &mut cache_usage)?;
        let kernel_caps = self.kernel_caps(SCORE_COMBINED_KERNEL_NAME)?;

        let baseline_outcomes = self
            .outcome_inputs_cache
            .get(&baseline_inputs.outcome_key)
            .map(|outcomes| OutcomeKernelLaunchArgs {
                outcome_valid_ptr: outcomes.d_outcome_valid.as_device_ptr(),
                outcome_pnl_ptr: outcomes.d_outcome_pnl.as_device_ptr(),
                outcome_premium_ptr: outcomes.d_outcome_premium.as_device_ptr(),
            })
            .ok_or_else(|| "missing cached GPU baseline outcome inputs".to_string())?;
        let conservative_outcomes = self
            .outcome_inputs_cache
            .get(&conservative_inputs.outcome_key)
            .map(|outcomes| OutcomeKernelLaunchArgs {
                outcome_valid_ptr: outcomes.d_outcome_valid.as_device_ptr(),
                outcome_pnl_ptr: outcomes.d_outcome_pnl.as_device_ptr(),
                outcome_premium_ptr: outcomes.d_outcome_premium.as_device_ptr(),
            })
            .ok_or_else(|| "missing cached GPU conservative outcome inputs".to_string())?;
        let harsher_outcomes = self
            .outcome_inputs_cache
            .get(&harsher_inputs.outcome_key)
            .map(|outcomes| OutcomeKernelLaunchArgs {
                outcome_valid_ptr: outcomes.d_outcome_valid.as_device_ptr(),
                outcome_pnl_ptr: outcomes.d_outcome_pnl.as_device_ptr(),
                outcome_premium_ptr: outcomes.d_outcome_premium.as_device_ptr(),
            })
            .ok_or_else(|| "missing cached GPU harsher outcome inputs".to_string())?;

        let mut search_aggregates = Vec::with_capacity(tuple_count);
        let mut holdout_aggregates = Vec::with_capacity(tuple_count);
        let mut conservative_holdout_aggregates = Vec::with_capacity(tuple_count);
        let mut harsher_holdout_aggregates = Vec::with_capacity(tuple_count);
        let mut robust_scores = Vec::with_capacity(tuple_count);

        for chunk_index in 0..chunk_plan.tuple_chunk_count.max(1) {
            let tuple_start = chunk_index.saturating_mul(chunk_plan.tuple_chunk_size);
            let tuple_end = (tuple_start + chunk_plan.tuple_chunk_size).min(tuple_count);
            let baseline_shared_key = self.ensure_shared_inputs_chunk(
                baseline_inputs,
                tuple_start,
                tuple_end,
                &mut cache_usage,
            )?;
            let conservative_shared_key = self.ensure_shared_inputs_chunk(
                conservative_inputs,
                tuple_start,
                tuple_end,
                &mut cache_usage,
            )?;
            let harsher_shared_key = self.ensure_shared_inputs_chunk(
                harsher_inputs,
                tuple_start,
                tuple_end,
                &mut cache_usage,
            )?;
            let baseline_shared = self
                .shared_inputs_cache
                .get(&baseline_shared_key)
                .map(|shared| SharedKernelLaunchArgs {
                    tuple_count: shared.tuple_count,
                    tuple_count_u32: shared.tuple_count_u32,
                    candidate_slot_stride_u32: shared.candidate_slot_stride_u32,
                    summary_stride_u32: shared.summary_stride_u32,
                    tuple_candidate_offsets_ptr: shared.d_tuple_candidate_offsets.as_device_ptr(),
                    tuple_summary_offsets_ptr: shared.d_tuple_summary_offsets.as_device_ptr(),
                })
                .ok_or_else(|| "missing cached GPU baseline shared inputs".to_string())?;
            let conservative_shared = self
                .shared_inputs_cache
                .get(&conservative_shared_key)
                .map(|shared| SharedKernelLaunchArgs {
                    tuple_count: shared.tuple_count,
                    tuple_count_u32: shared.tuple_count_u32,
                    candidate_slot_stride_u32: shared.candidate_slot_stride_u32,
                    summary_stride_u32: shared.summary_stride_u32,
                    tuple_candidate_offsets_ptr: shared.d_tuple_candidate_offsets.as_device_ptr(),
                    tuple_summary_offsets_ptr: shared.d_tuple_summary_offsets.as_device_ptr(),
                })
                .ok_or_else(|| "missing cached GPU conservative shared inputs".to_string())?;
            let harsher_shared = self
                .shared_inputs_cache
                .get(&harsher_shared_key)
                .map(|shared| SharedKernelLaunchArgs {
                    tuple_count: shared.tuple_count,
                    tuple_count_u32: shared.tuple_count_u32,
                    candidate_slot_stride_u32: shared.candidate_slot_stride_u32,
                    summary_stride_u32: shared.summary_stride_u32,
                    tuple_candidate_offsets_ptr: shared.d_tuple_candidate_offsets.as_device_ptr(),
                    tuple_summary_offsets_ptr: shared.d_tuple_summary_offsets.as_device_ptr(),
                })
                .ok_or_else(|| "missing cached GPU harsher shared inputs".to_string())?;
            let launch = choose_launch_shape_1d_from_candidates(
                baseline_shared.tuple_count,
                self.launch_caps,
                kernel_caps,
                &[64u32, 128, 256],
            );
            self.ensure_combined_output_buffers(baseline_shared.tuple_count, &mut cache_usage)?;
            let mut outputs = self
                .reusable_combined_output_buffers
                .take()
                .ok_or_else(|| "missing reusable GPU combined output buffers".to_string())?;
            let kernel = self
                .module
                .get_function(SCORE_COMBINED_KERNEL_NAME)
                .map_err(|e| e.to_string())?;
            cache_usage.kernel_launch_count += 1;
            let chunk_aggregates = launch_score_search_holdout_stress(
                &self.stream,
                &kernel,
                launch,
                search_end_u32,
                holdout_end_u32,
                baseline_shared,
                baseline_outcomes,
                conservative_shared,
                conservative_outcomes,
                harsher_shared,
                harsher_outcomes,
                &mut outputs,
            )?;
            self.reusable_combined_output_buffers = Some(outputs);
            search_aggregates.extend(chunk_aggregates.search_aggregates);
            holdout_aggregates.extend(chunk_aggregates.holdout_aggregates);
            conservative_holdout_aggregates
                .extend(chunk_aggregates.conservative_holdout_aggregates);
            harsher_holdout_aggregates.extend(chunk_aggregates.harsher_holdout_aggregates);
            robust_scores.extend(chunk_aggregates.robust_scores);
        }

        Ok(OptionsGpuCombinedTensorRun {
            search_aggregates,
            holdout_aggregates,
            conservative_holdout_aggregates,
            harsher_holdout_aggregates,
            robust_scores,
            cache_usage,
        })
    }

    fn inspect_plan(
        &mut self,
        inputs: &GpuFlatScoreInputs,
    ) -> Result<OptionsGpuRuntimeCacheUsage, String> {
        self.set_current()?;
        if inputs.tuple_candidate_offsets.len() != inputs.tuple_summary_offsets.len() {
            return Err("GPU options-expression tuple offset lengths do not match".to_string());
        }
        let tuple_count = inputs.tuple_candidate_offsets.len();
        let free_memory_bytes = mem_get_info().ok().map(|(free, _total)| free as u64);
        let chunk_plan = OptionsGpuChunkPlan::for_inputs(
            tuple_count,
            self.device_total_memory_bytes,
            free_memory_bytes,
            inputs,
        )?;
        Ok(OptionsGpuRuntimeCacheUsage {
            tuple_chunk_count: chunk_plan.tuple_chunk_count,
            tuple_chunk_size: chunk_plan.tuple_chunk_size,
            gpu_tuple_budget_bytes: chunk_plan.gpu_tuple_budget_bytes,
            gpu_outcome_tensor_bytes: chunk_plan.gpu_outcome_tensor_bytes,
            gpu_shared_input_bytes_per_chunk: chunk_plan.gpu_shared_input_bytes_per_chunk,
            gpu_output_bytes_per_chunk: chunk_plan.gpu_output_bytes_per_chunk,
            gpu_total_device_bytes_per_chunk: chunk_plan.gpu_total_device_bytes_per_chunk,
            gpu_device_total_memory_bytes: chunk_plan.gpu_device_total_memory_bytes,
            gpu_device_free_memory_bytes: chunk_plan.gpu_device_free_memory_bytes,
            gpu_device_usable_memory_bytes: chunk_plan.gpu_device_usable_memory_bytes,
            gpu_memory_budget_source: chunk_plan.gpu_memory_budget_source,
            gpu_outcome_tensor_fits_in_memory: chunk_plan.gpu_outcome_tensor_fits_in_memory,
            gpu_chunk_plan_fits_in_memory: chunk_plan.gpu_chunk_plan_fits_in_memory,
            gpu_plan_notes: chunk_plan.gpu_plan_notes,
            ..OptionsGpuRuntimeCacheUsage::default()
        })
    }

    fn inspect_combined_plan(
        &mut self,
        baseline_inputs: &GpuFlatScoreInputs,
        conservative_inputs: &GpuFlatScoreInputs,
        harsher_inputs: &GpuFlatScoreInputs,
    ) -> Result<OptionsGpuRuntimeCacheUsage, String> {
        self.set_current()?;
        let tuple_count = baseline_inputs.tuple_candidate_offsets.len();
        if conservative_inputs.tuple_candidate_offsets.len() != tuple_count
            || harsher_inputs.tuple_candidate_offsets.len() != tuple_count
        {
            return Err("GPU options-expression combined scorer requires identical tuple counts across lanes".to_string());
        }
        let free_memory_bytes = mem_get_info().ok().map(|(free, _total)| free as u64);
        let chunk_plan = OptionsGpuChunkPlan::for_combined_inputs(
            tuple_count,
            self.device_total_memory_bytes,
            free_memory_bytes,
            baseline_inputs,
            conservative_inputs,
            harsher_inputs,
        )?;
        Ok(OptionsGpuRuntimeCacheUsage {
            tuple_chunk_count: chunk_plan.tuple_chunk_count,
            tuple_chunk_size: chunk_plan.tuple_chunk_size,
            gpu_tuple_budget_bytes: chunk_plan.gpu_tuple_budget_bytes,
            gpu_outcome_tensor_bytes: chunk_plan.gpu_outcome_tensor_bytes,
            gpu_shared_input_bytes_per_chunk: chunk_plan.gpu_shared_input_bytes_per_chunk,
            gpu_output_bytes_per_chunk: chunk_plan.gpu_output_bytes_per_chunk,
            gpu_total_device_bytes_per_chunk: chunk_plan.gpu_total_device_bytes_per_chunk,
            gpu_device_total_memory_bytes: chunk_plan.gpu_device_total_memory_bytes,
            gpu_device_free_memory_bytes: chunk_plan.gpu_device_free_memory_bytes,
            gpu_device_usable_memory_bytes: chunk_plan.gpu_device_usable_memory_bytes,
            gpu_memory_budget_source: chunk_plan.gpu_memory_budget_source,
            gpu_outcome_tensor_fits_in_memory: chunk_plan.gpu_outcome_tensor_fits_in_memory,
            gpu_chunk_plan_fits_in_memory: chunk_plan.gpu_chunk_plan_fits_in_memory,
            gpu_plan_notes: chunk_plan.gpu_plan_notes,
            ..OptionsGpuRuntimeCacheUsage::default()
        })
    }
}

fn evict_oldest_if_needed<T>(cache: &mut BTreeMap<String, T>, max_entries: usize) {
    while cache.len() >= max_entries {
        let Some(oldest_key) = cache.keys().next().cloned() else {
            break;
        };
        cache.remove(&oldest_key);
    }
}

fn options_expression_gpu_runtime_store(
) -> &'static Mutex<BTreeMap<u32, Arc<Mutex<OptionsExpressionGpuRuntime>>>> {
    static STORE: OnceLock<Mutex<BTreeMap<u32, Arc<Mutex<OptionsExpressionGpuRuntime>>>>> =
        OnceLock::new();
    STORE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

#[cfg(test)]
fn options_expression_gpu_runtime_store_len() -> usize {
    options_expression_gpu_runtime_store()
        .lock()
        .expect("options expression gpu runtime store poisoned")
        .len()
}

fn with_runtime<T>(
    device_id: u32,
    f: impl FnOnce(&mut OptionsExpressionGpuRuntime) -> Result<T, String>,
) -> Result<T, String> {
    let runtime = {
        let mut runtimes = options_expression_gpu_runtime_store()
            .lock()
            .expect("options expression gpu runtime store poisoned");
        if let Some(runtime) = runtimes.get(&device_id) {
            Arc::clone(runtime)
        } else {
            let runtime = Arc::new(Mutex::new(OptionsExpressionGpuRuntime::new(device_id)?));
            runtimes.insert(device_id, Arc::clone(&runtime));
            runtime
        }
    };
    let mut runtime = runtime
        .lock()
        .expect("options expression gpu runtime poisoned");
    runtime.set_current()?;
    f(&mut runtime)
}

pub fn prewarm_options_expression_gpu_runtime(device_id: u32) -> Result<(), String> {
    with_runtime(device_id, |runtime| {
        runtime.set_current()?;
        let _ = runtime.kernel_caps(SCORE_KERNEL_NAME)?;
        let _ = runtime.kernel_caps(SCORE_COMBINED_KERNEL_NAME)?;
        Ok(())
    })
}

fn launch_score_range(
    stream: &Stream,
    kernel: &Function<'_>,
    launch: LaunchShape1d,
    shared: SharedKernelLaunchArgs,
    outcomes: OutcomeKernelLaunchArgs,
    event_start_u32: u32,
    event_end_u32: u32,
    outputs: &mut ReusableOutputBuffers,
) -> Result<Vec<OptionsGpuTupleAggregate>, String> {
    unsafe {
        launch!(kernel<<<(launch.grid_x.max(1), 1, 1), (launch.block_x.max(1), 1, 1), 0, stream>>>(
            event_start_u32,
            event_end_u32,
            shared.candidate_slot_stride_u32,
            shared.summary_stride_u32,
            shared.tuple_candidate_offsets_ptr,
            shared.tuple_summary_offsets_ptr,
            outcomes.outcome_valid_ptr,
            outcomes.outcome_pnl_ptr,
            outcomes.outcome_premium_ptr,
            shared.tuple_count_u32,
            outputs.d_out_valid.as_device_ptr(),
            outputs.d_out_wins.as_device_ptr(),
            outputs.d_out_total_pnl.as_device_ptr(),
            outputs.d_out_total_premium.as_device_ptr(),
            outputs.d_out_sum_return_on_premium.as_device_ptr(),
            outputs.d_out_sumsq_return_on_premium.as_device_ptr()
        ))
        .map_err(|e| e.to_string())?;
    }

    stream.synchronize().map_err(|e| e.to_string())?;
    outputs.copy_to_host()?;
    Ok(outputs.aggregates_prefix(shared.tuple_count))
}

fn launch_score_search_holdout_stress_into_buffers(
    stream: &Stream,
    kernel: &Function<'_>,
    launch: LaunchShape1d,
    search_end_u32: u32,
    holdout_end_u32: u32,
    baseline_shared: SharedKernelLaunchArgs,
    baseline_outcomes: OutcomeKernelLaunchArgs,
    conservative_shared: SharedKernelLaunchArgs,
    conservative_outcomes: OutcomeKernelLaunchArgs,
    harsher_shared: SharedKernelLaunchArgs,
    harsher_outcomes: OutcomeKernelLaunchArgs,
    outputs: &mut CombinedReusableOutputBuffers,
) -> Result<(), String> {
    unsafe {
        launch!(kernel<<<(launch.grid_x.max(1), 1, 1), (launch.block_x.max(1), 1, 1), 0, stream>>>(
            search_end_u32,
            holdout_end_u32,
            baseline_shared.candidate_slot_stride_u32,
            baseline_shared.summary_stride_u32,
            baseline_shared.tuple_candidate_offsets_ptr,
            baseline_shared.tuple_summary_offsets_ptr,
            baseline_outcomes.outcome_valid_ptr,
            baseline_outcomes.outcome_pnl_ptr,
            baseline_outcomes.outcome_premium_ptr,
            conservative_shared.candidate_slot_stride_u32,
            conservative_shared.summary_stride_u32,
            conservative_shared.tuple_candidate_offsets_ptr,
            conservative_shared.tuple_summary_offsets_ptr,
            conservative_outcomes.outcome_valid_ptr,
            conservative_outcomes.outcome_pnl_ptr,
            conservative_outcomes.outcome_premium_ptr,
            harsher_shared.candidate_slot_stride_u32,
            harsher_shared.summary_stride_u32,
            harsher_shared.tuple_candidate_offsets_ptr,
            harsher_shared.tuple_summary_offsets_ptr,
            harsher_outcomes.outcome_valid_ptr,
            harsher_outcomes.outcome_pnl_ptr,
            harsher_outcomes.outcome_premium_ptr,
            baseline_shared.tuple_count_u32,
            outputs.search.d_out_valid.as_device_ptr(),
            outputs.search.d_out_wins.as_device_ptr(),
            outputs.search.d_out_total_pnl.as_device_ptr(),
            outputs.search.d_out_total_premium.as_device_ptr(),
            outputs.search.d_out_sum_return_on_premium.as_device_ptr(),
            outputs.search.d_out_sumsq_return_on_premium.as_device_ptr(),
            outputs.holdout.d_out_valid.as_device_ptr(),
            outputs.holdout.d_out_wins.as_device_ptr(),
            outputs.holdout.d_out_total_pnl.as_device_ptr(),
            outputs.holdout.d_out_total_premium.as_device_ptr(),
            outputs.holdout.d_out_sum_return_on_premium.as_device_ptr(),
            outputs.holdout.d_out_sumsq_return_on_premium.as_device_ptr(),
            outputs.conservative_holdout.d_out_valid.as_device_ptr(),
            outputs.conservative_holdout.d_out_wins.as_device_ptr(),
            outputs.conservative_holdout.d_out_total_pnl.as_device_ptr(),
            outputs.conservative_holdout.d_out_total_premium.as_device_ptr(),
            outputs.conservative_holdout.d_out_sum_return_on_premium.as_device_ptr(),
            outputs.conservative_holdout.d_out_sumsq_return_on_premium.as_device_ptr(),
            outputs.harsher_holdout.d_out_valid.as_device_ptr(),
            outputs.harsher_holdout.d_out_wins.as_device_ptr(),
            outputs.harsher_holdout.d_out_total_pnl.as_device_ptr(),
            outputs.harsher_holdout.d_out_total_premium.as_device_ptr(),
            outputs.harsher_holdout.d_out_sum_return_on_premium.as_device_ptr(),
            outputs.harsher_holdout.d_out_sumsq_return_on_premium.as_device_ptr(),
            outputs.d_out_robust_score.as_device_ptr()
        ))
        .map_err(|e| e.to_string())?;
    }

    stream.synchronize().map_err(|e| e.to_string())
}

fn launch_score_search_holdout_stress(
    stream: &Stream,
    kernel: &Function<'_>,
    launch: LaunchShape1d,
    search_end_u32: u32,
    holdout_end_u32: u32,
    baseline_shared: SharedKernelLaunchArgs,
    baseline_outcomes: OutcomeKernelLaunchArgs,
    conservative_shared: SharedKernelLaunchArgs,
    conservative_outcomes: OutcomeKernelLaunchArgs,
    harsher_shared: SharedKernelLaunchArgs,
    harsher_outcomes: OutcomeKernelLaunchArgs,
    outputs: &mut CombinedReusableOutputBuffers,
) -> Result<OptionsGpuCombinedTensorRun, String> {
    launch_score_search_holdout_stress_into_buffers(
        stream,
        kernel,
        launch,
        search_end_u32,
        holdout_end_u32,
        baseline_shared,
        baseline_outcomes,
        conservative_shared,
        conservative_outcomes,
        harsher_shared,
        harsher_outcomes,
        outputs,
    )?;
    outputs.copy_to_host()?;
    Ok(OptionsGpuCombinedTensorRun {
        search_aggregates: outputs
            .search
            .aggregates_prefix(baseline_shared.tuple_count),
        holdout_aggregates: outputs
            .holdout
            .aggregates_prefix(baseline_shared.tuple_count),
        conservative_holdout_aggregates: outputs
            .conservative_holdout
            .aggregates_prefix(baseline_shared.tuple_count),
        harsher_holdout_aggregates: outputs
            .harsher_holdout
            .aggregates_prefix(baseline_shared.tuple_count),
        robust_scores: outputs.robust_scores_prefix(baseline_shared.tuple_count),
        cache_usage: OptionsGpuRuntimeCacheUsage::default(),
    })
}

pub(crate) fn score_options_expression_tensors_gpu_range(
    device_id: u32,
    event_start: usize,
    event_end: usize,
    inputs: &GpuFlatScoreInputs,
) -> Result<OptionsGpuTensorRun, String> {
    score_options_expression_tensors_gpu_multi_range(device_id, &[(event_start, event_end)], inputs)
}

pub(crate) fn score_options_expression_tensors_gpu_multi_range(
    device_id: u32,
    event_ranges: &[(usize, usize)],
    inputs: &GpuFlatScoreInputs,
) -> Result<OptionsGpuTensorRun, String> {
    if event_ranges.is_empty() {
        return Ok(OptionsGpuTensorRun::default());
    }

    with_runtime(device_id, |runtime| {
        runtime.score_ranges(inputs, event_ranges)
    })
}

pub(crate) fn score_options_expression_tensors_gpu_search_holdout_stress(
    device_id: u32,
    search_end: usize,
    holdout_end: usize,
    baseline_inputs: &GpuFlatScoreInputs,
    conservative_inputs: &GpuFlatScoreInputs,
    harsher_inputs: &GpuFlatScoreInputs,
) -> Result<OptionsGpuCombinedTensorRun, String> {
    with_runtime(device_id, |runtime| {
        runtime.score_search_holdout_stress(
            search_end,
            holdout_end,
            baseline_inputs,
            conservative_inputs,
            harsher_inputs,
        )
    })
}

pub(crate) fn inspect_options_expression_gpu_plan(
    device_id: u32,
    inputs: &GpuFlatScoreInputs,
) -> Result<OptionsGpuRuntimeCacheUsage, String> {
    with_runtime(device_id, |runtime| runtime.inspect_plan(inputs))
}

pub(crate) fn inspect_options_expression_gpu_combined_plan(
    device_id: u32,
    baseline_inputs: &GpuFlatScoreInputs,
    conservative_inputs: &GpuFlatScoreInputs,
    harsher_inputs: &GpuFlatScoreInputs,
) -> Result<OptionsGpuRuntimeCacheUsage, String> {
    with_runtime(device_id, |runtime| {
        runtime.inspect_combined_plan(baseline_inputs, conservative_inputs, harsher_inputs)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::lock_test_env;

    fn sample_inputs(tuple_count: usize) -> GpuFlatScoreInputs {
        GpuFlatScoreInputs {
            tuple_candidate_offsets: (0..tuple_count).map(|value| value as u32).collect(),
            tuple_summary_offsets: (0..tuple_count).map(|value| value as u32).collect(),
            outcome_valid: vec![1, 1, 1, 1],
            outcome_pnl: vec![1.0, 1.0, 1.0, 1.0],
            outcome_premium: vec![1.0, 1.0, 1.0, 1.0],
            candidate_slot_stride: 1,
            summary_stride: 1,
            shared_key: "shared".to_string(),
            outcome_key: "outcome".to_string(),
        }
    }

    #[test]
    fn chunk_plan_caps_large_tuple_batches() {
        let inputs = sample_inputs(25_000);
        let plan = OptionsGpuChunkPlan::for_inputs(
            25_000,
            24 * 1024 * 1024 * 1024,
            Some(24 * 1024 * 1024 * 1024),
            &inputs,
        )
        .expect("plan");

        assert!(plan.tuple_chunk_size <= configured_tuple_chunk_cap());
        assert!(plan.tuple_chunk_count >= 4);
        assert!(plan.gpu_total_device_bytes_per_chunk >= plan.gpu_outcome_tensor_bytes);
    }

    #[test]
    fn chunk_plan_keeps_small_tuple_batches_single_chunk() {
        let inputs = sample_inputs(128);
        let plan = OptionsGpuChunkPlan::for_inputs(
            128,
            8 * 1024 * 1024 * 1024,
            Some(8 * 1024 * 1024 * 1024),
            &inputs,
        )
        .expect("plan");

        assert_eq!(plan.tuple_chunk_size, 128);
        assert_eq!(plan.tuple_chunk_count, 1);
    }

    #[test]
    fn chunk_plan_respects_free_memory_without_minimum_tuple_floor() {
        let inputs = sample_inputs(5_000);
        let plan =
            OptionsGpuChunkPlan::for_inputs(5_000, 8 * 1024 * 1024 * 1024, Some(4_096), &inputs)
                .expect("plan");

        assert!(plan.tuple_chunk_size < 512);
        assert!(plan.gpu_chunk_plan_fits_in_memory);
        assert_eq!(plan.gpu_memory_budget_source, "free_memory_headroom");
    }

    #[test]
    fn chunk_plan_errors_when_outcome_tensor_cannot_fit_current_free_memory() {
        let mut inputs = sample_inputs(128);
        inputs.outcome_valid = vec![1; 16_384];
        inputs.outcome_pnl = vec![1.0; 16_384];
        inputs.outcome_premium = vec![1.0; 16_384];

        let err =
            OptionsGpuChunkPlan::for_inputs(128, 8 * 1024 * 1024 * 1024, Some(1_024), &inputs)
                .expect_err("expected free-memory fit failure");
        assert!(err.contains("outcome tensor"));
    }

    #[test]
    fn output_buffers_reuse_larger_allocations_for_smaller_chunks() {
        if cust::init(CudaFlags::empty()).is_err() {
            return;
        }
        let Ok(device) = Device::get_device(0) else {
            return;
        };
        let Ok(context) = Context::new(device) else {
            return;
        };
        if CurrentContext::set_current(&context).is_err() {
            return;
        }

        let mut buffers = ReusableOutputBuffers::new(16).expect("buffers");
        for index in 0..buffers.host_valid.len() {
            buffers.host_valid[index] = index as i32;
            buffers.host_wins[index] = index as i32;
            buffers.host_total_pnl[index] = index as f32;
            buffers.host_total_premium[index] = index as f32;
            buffers.host_sum_return_on_premium[index] = index as f32 * 0.5;
            buffers.host_sumsq_return_on_premium[index] = index as f32 * 1.5;
        }

        let prefix = buffers.aggregates_prefix(4);
        assert_eq!(prefix.len(), 4);
        assert_eq!(prefix[3].valid_trades, 3);
        assert_eq!(prefix[3].wins, 3);
        assert_eq!(prefix[3].sum_return_on_premium, 1.5);
        assert_eq!(prefix[3].sumsq_return_on_premium, 4.5);
    }

    #[test]
    fn per_tuple_output_device_bytes_reflects_widened_layout() {
        assert_eq!(per_tuple_output_device_bytes(), 24);
    }

    #[test]
    fn prewarm_runtime_reuses_process_wide_entry_across_threads() {
        let _env = lock_test_env();
        if cust::init(CudaFlags::empty()).is_err() {
            return;
        }
        let Ok(device_count) = Device::num_devices() else {
            return;
        };
        if device_count == 0 {
            return;
        }

        let before = options_expression_gpu_runtime_store_len();
        prewarm_options_expression_gpu_runtime(0).expect("main-thread prewarm");
        let after_main_thread = options_expression_gpu_runtime_store_len();
        assert!(after_main_thread >= before);

        std::thread::spawn(|| prewarm_options_expression_gpu_runtime(0))
            .join()
            .expect("worker thread join")
            .expect("worker-thread prewarm");
        let after_worker_thread = options_expression_gpu_runtime_store_len();

        assert_eq!(after_worker_thread, after_main_thread);
    }
}
