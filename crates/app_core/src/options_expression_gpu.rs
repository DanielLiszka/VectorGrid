#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::gpu_backtest_kernel::{
    choose_launch_shape_1d_from_candidates, detect_device_launch_caps, detect_kernel_launch_caps,
    DeviceLaunchCaps, KernelLaunchCaps, LaunchShape1d,
};
use crate::options_expression::GpuFlatScoreInputs;
use cust::context::{Context, CurrentContext};
use cust::device::Device;
use cust::function::Function;
use cust::launch;
use cust::memory::{mem_get_info, CopyDestination, DeviceBuffer, DevicePointer};
use cust::module::{Module, ModuleJitOption, OptLevel};
use cust::prelude::{CudaFlags, Stream, StreamFlags};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::mem::size_of;

const SCORE_KERNEL_NAME: &str = "options_expression_score_split_f64";
const GPU_INPUT_CACHE_MAX_ENTRIES: usize = 16;
const GPU_TUPLE_CHUNK_BUDGET_CAP_BYTES: u64 = 64 * 1024 * 1024;
const GPU_TUPLE_CHUNK_BUDGET_FLOOR_BYTES: u64 = 4 * 1024 * 1024;
const GPU_TUPLE_CHUNK_BUDGET_TOTAL_MEMORY_DIVISOR: u64 = 16;
const GPU_TUPLE_CHUNK_MEMORY_HEADROOM_BYTES: u64 = 256 * 1024 * 1024;
const GPU_TUPLE_CHUNK_HARD_CAP_TUPLES: usize = 8_192;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct OptionsGpuTupleAggregate {
    pub(crate) valid_trades: u32,
    pub(crate) wins: u32,
    pub(crate) total_pnl: f32,
    pub(crate) total_premium: f32,
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
    host_valid: Vec<i32>,
    host_wins: Vec<i32>,
    host_total_pnl: Vec<f32>,
    host_total_premium: Vec<f32>,
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
    size_of::<i32>() * 2 + size_of::<f32>() * 2
}

fn outcome_tensor_device_bytes(inputs: &GpuFlatScoreInputs) -> usize {
    inputs.outcome_valid.len() * size_of::<u8>()
        + inputs.outcome_pnl.len() * size_of::<f32>()
        + inputs.outcome_premium.len() * size_of::<f32>()
}

fn usable_memory_bytes_from_free(free_memory_bytes: u64) -> u64 {
    let usable = free_memory_bytes.saturating_sub(GPU_TUPLE_CHUNK_MEMORY_HEADROOM_BYTES);
    if usable > 0 {
        return usable;
    }
    free_memory_bytes / 2
}

fn tuple_budget_bytes(total_memory_bytes: u64, usable_memory_bytes: Option<u64>) -> (usize, String) {
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
    fn for_inputs(
        tuple_count: usize,
        total_memory_bytes: u64,
        free_memory_bytes: Option<u64>,
        inputs: &GpuFlatScoreInputs,
    ) -> Result<Self, String> {
        if tuple_count == 0 {
            return Ok(Self::default());
        }

        let per_tuple_device_bytes =
            per_tuple_shared_input_device_bytes() + per_tuple_output_device_bytes();
        let gpu_outcome_tensor_bytes = outcome_tensor_device_bytes(inputs);
        let mut gpu_plan_notes = Vec::new();
        let gpu_device_free_memory_bytes = free_memory_bytes
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let gpu_device_usable_memory_bytes = free_memory_bytes
            .map(usable_memory_bytes_from_free)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let (mut tuple_budget, gpu_memory_budget_source) =
            tuple_budget_bytes(total_memory_bytes, free_memory_bytes.map(usable_memory_bytes_from_free));
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
        let gpu_shared_input_bytes_per_chunk =
            tuple_chunk_size * per_tuple_shared_input_device_bytes();
        let gpu_output_bytes_per_chunk = tuple_chunk_size * per_tuple_output_device_bytes();
        let gpu_total_device_bytes_per_chunk = gpu_outcome_tensor_bytes
            .saturating_add(gpu_shared_input_bytes_per_chunk)
            .saturating_add(gpu_output_bytes_per_chunk);
        let gpu_chunk_plan_fits_in_memory = free_memory_bytes
            .map(|free| gpu_total_device_bytes_per_chunk <= usable_memory_bytes_from_free(free) as usize)
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
            gpu_device_total_memory_bytes: usize::try_from(total_memory_bytes).unwrap_or(usize::MAX),
            gpu_device_free_memory_bytes,
            gpu_device_usable_memory_bytes,
            gpu_memory_budget_source,
            gpu_outcome_tensor_fits_in_memory,
            gpu_chunk_plan_fits_in_memory,
            gpu_plan_notes,
        })
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
            host_valid: vec![0i32; tuple_count],
            host_wins: vec![0i32; tuple_count],
            host_total_pnl: vec![0.0f32; tuple_count],
            host_total_premium: vec![0.0f32; tuple_count],
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
        Ok(())
    }

    fn aggregates_prefix(&self, tuple_count: usize) -> Vec<OptionsGpuTupleAggregate> {
        (0..tuple_count.min(self.host_valid.len()))
            .map(|index| OptionsGpuTupleAggregate {
                valid_trades: self.host_valid[index].max(0) as u32,
                wins: self.host_wins[index].max(0) as u32,
                total_pnl: self.host_total_pnl[index],
                total_premium: self.host_total_premium[index],
            })
            .collect()
    }

    fn tuple_count(&self) -> usize {
        self.host_valid.len()
    }
}

struct OptionsExpressionGpuRuntime {
    context: Context,
    module: Module,
    stream: Stream,
    launch_caps: DeviceLaunchCaps,
    device_total_memory_bytes: u64,
    kernel_caps: RefCell<Option<KernelLaunchCaps>>,
    shared_inputs_cache: BTreeMap<String, UploadedSharedInputs>,
    outcome_inputs_cache: BTreeMap<String, UploadedOutcomeInputs>,
    reusable_output_buffers: Option<ReusableOutputBuffers>,
}

impl OptionsExpressionGpuRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let device_total_memory_bytes = device.total_memory().map_err(|e| e.to_string())? as u64;
        let launch_caps = detect_device_launch_caps(device);
        let context = Context::new(device).map_err(|e| e.to_string())?;
        CurrentContext::set_current(&context).map_err(|e| e.to_string())?;
        let ptx = include_str!(concat!(env!("OUT_DIR"), "/options_expression.ptx"));
        let module = Module::from_ptx(
            ptx,
            &[
                ModuleJitOption::DetermineTargetFromContext,
                ModuleJitOption::OptLevel(OptLevel::O2),
            ],
        )
        .or_else(|_| Module::from_ptx(ptx, &[ModuleJitOption::DetermineTargetFromContext]))
        .or_else(|_| Module::from_ptx(ptx, &[]))
        .map_err(|e| e.to_string())?;
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;

        Ok(Self {
            context,
            module,
            stream,
            launch_caps,
            device_total_memory_bytes,
            kernel_caps: RefCell::new(None),
            shared_inputs_cache: BTreeMap::new(),
            outcome_inputs_cache: BTreeMap::new(),
            reusable_output_buffers: None,
        })
    }

    fn set_current(&self) -> Result<(), String> {
        CurrentContext::set_current(&self.context).map_err(|e| e.to_string())
    }

    fn kernel_caps(&self) -> Result<KernelLaunchCaps, String> {
        if let Some(cached) = *self.kernel_caps.borrow() {
            return Ok(cached);
        }
        let func = self
            .module
            .get_function(SCORE_KERNEL_NAME)
            .map_err(|e| e.to_string())?;
        let caps = detect_kernel_launch_caps(&func, self.launch_caps);
        *self.kernel_caps.borrow_mut() = Some(caps);
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
        let free_memory_bytes = mem_get_info()
            .ok()
            .map(|(free, _total)| free as u64);
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
        cache_usage.gpu_shared_input_bytes_per_chunk =
            chunk_plan.gpu_shared_input_bytes_per_chunk;
        cache_usage.gpu_output_bytes_per_chunk = chunk_plan.gpu_output_bytes_per_chunk;
        cache_usage.gpu_total_device_bytes_per_chunk =
            chunk_plan.gpu_total_device_bytes_per_chunk;
        cache_usage.gpu_device_total_memory_bytes = chunk_plan.gpu_device_total_memory_bytes;
        cache_usage.gpu_device_free_memory_bytes = chunk_plan.gpu_device_free_memory_bytes;
        cache_usage.gpu_device_usable_memory_bytes = chunk_plan.gpu_device_usable_memory_bytes;
        cache_usage.gpu_memory_budget_source = chunk_plan.gpu_memory_budget_source.clone();
        cache_usage.gpu_outcome_tensor_fits_in_memory =
            chunk_plan.gpu_outcome_tensor_fits_in_memory;
        cache_usage.gpu_chunk_plan_fits_in_memory = chunk_plan.gpu_chunk_plan_fits_in_memory;
        cache_usage.gpu_plan_notes = chunk_plan.gpu_plan_notes.clone();
        self.ensure_outcome_inputs(inputs, &mut cache_usage)?;
        let kernel_caps = self.kernel_caps()?;

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
                let shared_key =
                    self.ensure_shared_inputs_chunk(inputs, tuple_start, tuple_end, &mut cache_usage)?;
                let shared_args = self
                    .shared_inputs_cache
                    .get(&shared_key)
                    .map(|shared| SharedKernelLaunchArgs {
                        tuple_count: shared.tuple_count,
                        tuple_count_u32: shared.tuple_count_u32,
                        candidate_slot_stride_u32: shared.candidate_slot_stride_u32,
                        summary_stride_u32: shared.summary_stride_u32,
                        tuple_candidate_offsets_ptr: shared.d_tuple_candidate_offsets.as_device_ptr(),
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
}

fn evict_oldest_if_needed<T>(cache: &mut BTreeMap<String, T>, max_entries: usize) {
    while cache.len() >= max_entries {
        let Some(oldest_key) = cache.keys().next().cloned() else {
            break;
        };
        cache.remove(&oldest_key);
    }
}

thread_local! {
    static OPTIONS_EXPRESSION_GPU_RUNTIME: RefCell<BTreeMap<u32, OptionsExpressionGpuRuntime>> =
        RefCell::new(BTreeMap::new());
}

fn with_runtime<T>(
    device_id: u32,
    f: impl FnOnce(&mut OptionsExpressionGpuRuntime) -> Result<T, String>,
) -> Result<T, String> {
    OPTIONS_EXPRESSION_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, OptionsExpressionGpuRuntime::new(device_id)?);
        }
        let runtime = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| format!("missing GPU runtime for device {device_id}"))?;
        runtime.set_current()?;
        f(runtime)
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
            outputs.d_out_total_premium.as_device_ptr()
        ))
        .map_err(|e| e.to_string())?;
    }

    stream.synchronize().map_err(|e| e.to_string())?;
    outputs.copy_to_host()?;
    Ok(outputs.aggregates_prefix(shared.tuple_count))
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

pub(crate) fn inspect_options_expression_gpu_plan(
    device_id: u32,
    inputs: &GpuFlatScoreInputs,
) -> Result<OptionsGpuRuntimeCacheUsage, String> {
    with_runtime(device_id, |runtime| runtime.inspect_plan(inputs))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let plan = OptionsGpuChunkPlan::for_inputs(5_000, 8 * 1024 * 1024 * 1024, Some(4_096), &inputs)
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

        let err = OptionsGpuChunkPlan::for_inputs(128, 8 * 1024 * 1024 * 1024, Some(1_024), &inputs)
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
        }

        let prefix = buffers.aggregates_prefix(4);
        assert_eq!(prefix.len(), 4);
        assert_eq!(prefix[3].valid_trades, 3);
        assert_eq!(prefix[3].wins, 3);
    }
}
