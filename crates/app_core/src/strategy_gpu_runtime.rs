use crate::strategy_gpu_program::{
    compile_gpu_predicate_program, compile_gpu_predicate_program_abi, CompiledGpuPredicateProgram,
    CompiledGpuPredicateProgramAbi, STRATEGY_GPU_EXECUTABLE_ABI_VERSION,
};
use crate::strategy_plan::{CompiledStrategyPlan, StrategyPlanBackend};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct StrategyCapabilityReport {
    pub gpu_static_supported: bool,
    pub gpu_dynamic_supported: bool,
    pub cpu_dynamic_supported: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

impl StrategyCapabilityReport {
    pub fn cpu_dynamic_only(reason: impl Into<String>) -> Self {
        Self {
            gpu_static_supported: false,
            gpu_dynamic_supported: false,
            cpu_dynamic_supported: true,
            reasons: vec![reason.into()],
        }
    }

    pub fn gpu_dynamic_ready() -> Self {
        Self {
            gpu_static_supported: false,
            gpu_dynamic_supported: true,
            cpu_dynamic_supported: true,
            reasons: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StrategyHandle {
    pub plan: CompiledStrategyPlan,
    pub capability: StrategyCapabilityReport,
    #[serde(default)]
    pub gpu_program: Option<CompiledGpuPredicateProgram>,
    #[serde(default)]
    pub gpu_program_abi: Option<CompiledGpuPredicateProgramAbi>,
}

impl StrategyHandle {
    pub fn from_plan(
        plan: CompiledStrategyPlan,
        capability: StrategyCapabilityReport,
    ) -> Result<Self, String> {
        let (gpu_program, gpu_program_abi) = if plan.backend == StrategyPlanBackend::GpuUnified
            && capability.gpu_dynamic_supported
        {
            let program = compile_gpu_predicate_program(&plan)?;
            let abi = compile_gpu_predicate_program_abi(&program);
            (Some(program), Some(abi))
        } else {
            (None, None)
        };

        Ok(Self {
            plan,
            capability,
            gpu_program,
            gpu_program_abi,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuStrategyExecutableCacheKey {
    pub plan_fingerprint_hex: String,
    pub device_id: u32,
    pub abi_version: u32,
}

impl GpuStrategyExecutableCacheKey {
    pub fn new(plan_fingerprint_hex: impl Into<String>, device_id: u32) -> Self {
        Self {
            plan_fingerprint_hex: plan_fingerprint_hex.into(),
            device_id,
            abi_version: STRATEGY_GPU_EXECUTABLE_ABI_VERSION,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GpuStrategyExecutable {
    pub plan_fingerprint_hex: String,
    pub device_id: u32,
    pub abi_version: u32,
    pub indicator_slots_len: usize,
    pub conditions_len: usize,
    pub groups_len: usize,
    pub group_condition_indexes_len: usize,
}

impl GpuStrategyExecutable {
    pub fn from_handle(handle: &StrategyHandle, device_id: u32) -> Result<Self, String> {
        let abi = handle
            .gpu_program_abi
            .as_ref()
            .ok_or_else(|| "strategy handle does not contain a GPU program ABI".to_string())?;
        Ok(Self {
            plan_fingerprint_hex: abi.plan_fingerprint_hex.clone(),
            device_id,
            abi_version: abi.abi_version,
            indicator_slots_len: abi.indicator_slots.len(),
            conditions_len: abi.conditions.len(),
            groups_len: abi.groups.len(),
            group_condition_indexes_len: abi.group_condition_indexes.len(),
        })
    }
}

static GPU_STRATEGY_EXECUTABLE_CACHE: OnceLock<
    Mutex<HashMap<GpuStrategyExecutableCacheKey, GpuStrategyExecutable>>,
> = OnceLock::new();
static GPU_STRATEGY_EXECUTABLE_CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static GPU_STRATEGY_EXECUTABLE_CACHE_MISSES: AtomicU64 = AtomicU64::new(0);

#[allow(dead_code)]
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
struct UploadedGpuStrategyExecutableEntry {
    uploaded: crate::gpu_backtest_kernel::UploadedGpuStrategyProgram,
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
thread_local! {
    static GPU_STRATEGY_EXECUTABLE_UPLOAD_CACHE: RefCell<HashMap<GpuStrategyExecutableCacheKey, Rc<UploadedGpuStrategyExecutableEntry>>> =
        RefCell::new(HashMap::new());
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct StrategyExecutableCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
}

pub fn reset_gpu_strategy_executable_cache() {
    if let Some(cache) = GPU_STRATEGY_EXECUTABLE_CACHE.get() {
        if let Ok(mut guard) = cache.lock() {
            guard.clear();
        }
    }
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    GPU_STRATEGY_EXECUTABLE_UPLOAD_CACHE.with(|cell| {
        cell.borrow_mut().clear();
    });
    GPU_STRATEGY_EXECUTABLE_CACHE_HITS.store(0, Ordering::Relaxed);
    GPU_STRATEGY_EXECUTABLE_CACHE_MISSES.store(0, Ordering::Relaxed);
}

pub fn gpu_strategy_executable_cache_stats() -> StrategyExecutableCacheStats {
    let entries = GPU_STRATEGY_EXECUTABLE_CACHE
        .get()
        .and_then(|cache| cache.lock().ok().map(|guard| guard.len()))
        .unwrap_or(0);
    StrategyExecutableCacheStats {
        hits: GPU_STRATEGY_EXECUTABLE_CACHE_HITS.load(Ordering::Relaxed),
        misses: GPU_STRATEGY_EXECUTABLE_CACHE_MISSES.load(Ordering::Relaxed),
        entries,
    }
}

pub fn get_or_build_gpu_strategy_executable(
    handle: &StrategyHandle,
    device_id: u32,
) -> Result<GpuStrategyExecutable, String> {
    let key = GpuStrategyExecutableCacheKey::new(handle.plan.ir_fingerprint_hex.clone(), device_id);
    let cache = GPU_STRATEGY_EXECUTABLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let guard = cache
            .lock()
            .map_err(|_| "gpu strategy executable cache lock poisoned".to_string())?;
        if let Some(existing) = guard.get(&key) {
            GPU_STRATEGY_EXECUTABLE_CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            return Ok(existing.clone());
        }
    }

    let exec = GpuStrategyExecutable::from_handle(handle, device_id)?;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let abi = handle
            .gpu_program_abi
            .as_ref()
            .ok_or_else(|| "strategy handle missing GPU program ABI for upload".to_string())?;
        let key_for_upload = key.clone();
        GPU_STRATEGY_EXECUTABLE_UPLOAD_CACHE.with(|cell| -> Result<(), String> {
            let mut map = cell.borrow_mut();
            if !map.contains_key(&key_for_upload) {
                let uploaded =
                    crate::gpu_backtest_kernel::upload_gpu_strategy_program_abi(device_id, abi)?;
                let probe =
                    crate::gpu_backtest_kernel::probe_uploaded_gpu_strategy_program(&uploaded)?;
                if probe[0] != uploaded.abi_version
                    || probe[1] != uploaded.d_indicator_slots.len() as u32
                    || probe[2] != uploaded.d_conditions.len() as u32
                {
                    return Err(
                        "uploaded gpu strategy program probe returned inconsistent metadata"
                            .to_string(),
                    );
                }
                map.insert(
                    key_for_upload,
                    Rc::new(UploadedGpuStrategyExecutableEntry { uploaded }),
                );
            }
            Ok(())
        })?;
    }
    let mut guard = cache
        .lock()
        .map_err(|_| "gpu strategy executable cache lock poisoned".to_string())?;
    let entry = guard.entry(key).or_insert_with(|| exec.clone()).clone();
    GPU_STRATEGY_EXECUTABLE_CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
    Ok(entry)
}

#[allow(dead_code)]
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) fn with_uploaded_gpu_strategy_executable<T>(
    key: &GpuStrategyExecutableCacheKey,
    f: impl FnOnce(&crate::gpu_backtest_kernel::UploadedGpuStrategyProgram) -> T,
) -> Option<T> {
    GPU_STRATEGY_EXECUTABLE_UPLOAD_CACHE.with(|cell| {
        let map = cell.borrow();
        map.get(key).map(|entry| f(&entry.uploaded))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy_dynamic::compile_dynamic_plan_for_double_ma_config;
    use ta_strategies::double_ma::StrategyConfig;

    #[test]
    fn strategy_handle_builds_gpu_program_and_abi_for_gpu_unified_plan() {
        let plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "sma",
            None,
            None,
            &StrategyConfig::default(),
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile dynamic plan");

        let handle = StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
            .expect("build strategy handle");

        assert!(handle.gpu_program.is_some());
        assert!(handle.gpu_program_abi.is_some());
        let exec = GpuStrategyExecutable::from_handle(&handle, 0).expect("build executable");
        assert_eq!(exec.abi_version, STRATEGY_GPU_EXECUTABLE_ABI_VERSION);
        assert!(exec.indicator_slots_len > 0);
    }

    #[test]
    fn strategy_handle_skips_gpu_program_when_capability_disables_gpu_dynamic() {
        let plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "sma",
            None,
            None,
            &StrategyConfig::default(),
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile dynamic plan");

        let handle = StrategyHandle::from_plan(
            plan,
            StrategyCapabilityReport::cpu_dynamic_only("gpu dynamic disabled"),
        )
        .expect("build strategy handle");

        assert!(handle.gpu_program.is_none());
        assert!(handle.gpu_program_abi.is_none());
    }

    #[test]
    fn executable_cache_reuses_identical_handle_and_device() {
        reset_gpu_strategy_executable_cache();
        let plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "sma",
            None,
            None,
            &StrategyConfig::default(),
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile dynamic plan");
        let handle = StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
            .expect("build strategy handle");

        let first = get_or_build_gpu_strategy_executable(&handle, 0).expect("first build");
        let second = get_or_build_gpu_strategy_executable(&handle, 0).expect("second build");
        let stats = gpu_strategy_executable_cache_stats();
        assert_eq!(first, second);
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn uploaded_executable_is_available_after_cache_build() {
        reset_gpu_strategy_executable_cache();
        let _gpu = crate::gpu_backtest_kernel::lock_gpu_test();
        let plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "sma",
            None,
            None,
            &StrategyConfig::default(),
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile dynamic plan");
        let key = GpuStrategyExecutableCacheKey::new(plan.ir_fingerprint_hex.clone(), 0);
        let handle = StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
            .expect("build strategy handle");

        let exec = get_or_build_gpu_strategy_executable(&handle, 0).expect("build executable");
        let uploaded = with_uploaded_gpu_strategy_executable(&key, |uploaded| {
            (
                uploaded.abi_version,
                uploaded.plan_fingerprint_hex.clone(),
                uploaded.device_id,
                uploaded.indicator_slots_ptr_raw(),
            )
        })
        .expect("uploaded executable should exist");
        assert_eq!(uploaded.0, exec.abi_version);
        assert_eq!(uploaded.1, exec.plan_fingerprint_hex);
        assert_eq!(uploaded.2, exec.device_id);
        assert!(uploaded.3 > 0);
    }
}
