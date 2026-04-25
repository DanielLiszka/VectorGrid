use crate::strategy_gpu_program::{
    compile_gpu_predicate_program, compile_gpu_predicate_program_abi,
    fingerprint_compiled_strategy_plan_for_gpu_program, CompiledGpuPredicateProgram,
    CompiledGpuPredicateProgramAbi, STRATEGY_GPU_EXECUTABLE_ABI_VERSION,
    STRATEGY_GPU_PROGRAM_SCHEMA_VERSION,
};
use crate::strategy_plan::{CompiledStrategyPlan, StrategyPlanBackend};
use serde::{Deserialize, Serialize};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
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
            let bundle = get_or_build_compiled_gpu_predicate_bundle(&plan)?;
            (Some(bundle.program), Some(bundle.abi))
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

#[derive(Clone, Debug)]
struct CompiledGpuPredicateBundle {
    program: CompiledGpuPredicateProgram,
    abi: CompiledGpuPredicateProgramAbi,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CompiledGpuPredicateBundleCacheKey {
    compiled_plan_fingerprint_hex: String,
    program_schema_version: u16,
    abi_version: u32,
}

impl CompiledGpuPredicateBundleCacheKey {
    fn for_plan(plan: &CompiledStrategyPlan) -> Result<Self, String> {
        Ok(Self {
            compiled_plan_fingerprint_hex: fingerprint_compiled_strategy_plan_for_gpu_program(
                plan,
            )?,
            program_schema_version: STRATEGY_GPU_PROGRAM_SCHEMA_VERSION,
            abi_version: STRATEGY_GPU_EXECUTABLE_ABI_VERSION,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuStrategyExecutableCacheKey {
    pub executable_fingerprint_hex: String,
    pub device_id: u32,
    pub abi_version: u32,
}

impl GpuStrategyExecutableCacheKey {
    pub fn new(executable_fingerprint_hex: impl Into<String>, device_id: u32) -> Self {
        Self {
            executable_fingerprint_hex: executable_fingerprint_hex.into(),
            device_id,
            abi_version: STRATEGY_GPU_EXECUTABLE_ABI_VERSION,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GpuStrategyExecutable {
    pub plan_fingerprint_hex: String,
    pub compiled_plan_fingerprint_hex: String,
    pub executable_fingerprint_hex: String,
    pub device_id: u32,
    pub abi_version: u32,
    pub indicator_slots_len: usize,
    pub conditions_len: usize,
    pub groups_len: usize,
    pub group_condition_indexes_len: usize,
}

impl GpuStrategyExecutable {
    pub fn from_handle(handle: &StrategyHandle, device_id: u32) -> Result<Self, String> {
        let abi = validated_gpu_program_abi(handle)?;
        Ok(Self {
            plan_fingerprint_hex: abi.plan_fingerprint_hex.clone(),
            compiled_plan_fingerprint_hex: abi.compiled_plan_fingerprint_hex.clone(),
            executable_fingerprint_hex: abi.executable_fingerprint_hex.clone(),
            device_id,
            abi_version: abi.abi_version,
            indicator_slots_len: abi.indicator_slots.len(),
            conditions_len: abi.conditions.len(),
            groups_len: abi.groups.len(),
            group_condition_indexes_len: abi.group_condition_indexes.len(),
        })
    }
}

static GPU_STRATEGY_COMPILED_BUNDLE_CACHE: OnceLock<
    Mutex<HashMap<CompiledGpuPredicateBundleCacheKey, CompiledGpuPredicateBundle>>,
> = OnceLock::new();
static GPU_STRATEGY_COMPILED_BUNDLE_CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static GPU_STRATEGY_COMPILED_BUNDLE_CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
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
    pub compiled_bundle_hits: u64,

    pub compiled_bundle_misses: u64,
    pub compiled_bundle_entries: usize,
    pub hits: u64,

    pub misses: u64,
    pub entries: usize,

    pub uploaded_thread_entries: usize,
}

fn ensure_non_empty_fingerprint(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!(
            "strategy handle gpu program ABI has an empty {field}"
        ))
    } else {
        Ok(())
    }
}

fn validated_gpu_program_abi(
    handle: &StrategyHandle,
) -> Result<&CompiledGpuPredicateProgramAbi, String> {
    let abi = handle
        .gpu_program_abi
        .as_ref()
        .ok_or_else(|| "strategy handle does not contain a GPU program ABI".to_string())?;
    if abi.abi_version != STRATEGY_GPU_EXECUTABLE_ABI_VERSION {
        return Err(format!(
            "strategy handle gpu program ABI version {} does not match runtime version {}",
            abi.abi_version, STRATEGY_GPU_EXECUTABLE_ABI_VERSION
        ));
    }
    ensure_non_empty_fingerprint(
        abi.compiled_plan_fingerprint_hex.as_str(),
        "compiled_plan_fingerprint_hex",
    )?;
    ensure_non_empty_fingerprint(
        abi.executable_fingerprint_hex.as_str(),
        "executable_fingerprint_hex",
    )?;

    if let Some(program) = handle.gpu_program.as_ref() {
        if program.plan_fingerprint_hex != abi.plan_fingerprint_hex {
            return Err(
                "strategy handle gpu program fingerprint does not match ABI plan fingerprint"
                    .to_string(),
            );
        }
        if program.compiled_plan_fingerprint_hex != abi.compiled_plan_fingerprint_hex {
            return Err(
                "strategy handle gpu program fingerprint does not match ABI compiled-plan fingerprint"
                    .to_string(),
            );
        }
        if program.executable_fingerprint_hex != abi.executable_fingerprint_hex {
            return Err(
                "strategy handle gpu program fingerprint does not match ABI executable fingerprint"
                    .to_string(),
            );
        }
    }

    Ok(abi)
}

fn get_or_build_compiled_gpu_predicate_bundle(
    plan: &CompiledStrategyPlan,
) -> Result<CompiledGpuPredicateBundle, String> {
    let key = CompiledGpuPredicateBundleCacheKey::for_plan(plan)?;
    let cache = GPU_STRATEGY_COMPILED_BUNDLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let guard = cache
            .lock()
            .map_err(|_| "gpu predicate bundle cache lock poisoned".to_string())?;
        if let Some(existing) = guard.get(&key) {
            GPU_STRATEGY_COMPILED_BUNDLE_CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            return Ok(existing.clone());
        }
    }

    let program = compile_gpu_predicate_program(plan)?;
    let abi = compile_gpu_predicate_program_abi(&program);
    if program.compiled_plan_fingerprint_hex != key.compiled_plan_fingerprint_hex
        || abi.compiled_plan_fingerprint_hex != key.compiled_plan_fingerprint_hex
    {
        return Err(
            "compiled gpu predicate bundle fingerprint does not match cache-key fingerprint"
                .to_string(),
        );
    }
    let bundle = CompiledGpuPredicateBundle { program, abi };
    let mut guard = cache
        .lock()
        .map_err(|_| "gpu predicate bundle cache lock poisoned".to_string())?;
    let entry = match guard.entry(key) {
        std::collections::hash_map::Entry::Occupied(existing) => {
            GPU_STRATEGY_COMPILED_BUNDLE_CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            existing.get().clone()
        }
        std::collections::hash_map::Entry::Vacant(vacant) => {
            GPU_STRATEGY_COMPILED_BUNDLE_CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
            vacant.insert(bundle).clone()
        }
    };
    Ok(entry)
}

pub fn reset_gpu_strategy_executable_cache() {
    if let Some(cache) = GPU_STRATEGY_COMPILED_BUNDLE_CACHE.get() {
        if let Ok(mut guard) = cache.lock() {
            guard.clear();
        }
    }
    if let Some(cache) = GPU_STRATEGY_EXECUTABLE_CACHE.get() {
        if let Ok(mut guard) = cache.lock() {
            guard.clear();
        }
    }
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    GPU_STRATEGY_EXECUTABLE_UPLOAD_CACHE.with(|cell| {
        cell.borrow_mut().clear();
    });
    GPU_STRATEGY_COMPILED_BUNDLE_CACHE_HITS.store(0, Ordering::Relaxed);
    GPU_STRATEGY_COMPILED_BUNDLE_CACHE_MISSES.store(0, Ordering::Relaxed);
    GPU_STRATEGY_EXECUTABLE_CACHE_HITS.store(0, Ordering::Relaxed);
    GPU_STRATEGY_EXECUTABLE_CACHE_MISSES.store(0, Ordering::Relaxed);
}

pub fn gpu_strategy_executable_cache_stats() -> StrategyExecutableCacheStats {
    let compiled_bundle_entries = GPU_STRATEGY_COMPILED_BUNDLE_CACHE
        .get()
        .and_then(|cache| cache.lock().ok().map(|guard| guard.len()))
        .unwrap_or(0);
    let entries = GPU_STRATEGY_EXECUTABLE_CACHE
        .get()
        .and_then(|cache| cache.lock().ok().map(|guard| guard.len()))
        .unwrap_or(0);
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let uploaded_thread_entries =
        GPU_STRATEGY_EXECUTABLE_UPLOAD_CACHE.with(|cell| cell.borrow().len());
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let uploaded_thread_entries = 0usize;
    StrategyExecutableCacheStats {
        compiled_bundle_hits: GPU_STRATEGY_COMPILED_BUNDLE_CACHE_HITS.load(Ordering::Relaxed),
        compiled_bundle_misses: GPU_STRATEGY_COMPILED_BUNDLE_CACHE_MISSES.load(Ordering::Relaxed),
        compiled_bundle_entries,
        hits: GPU_STRATEGY_EXECUTABLE_CACHE_HITS.load(Ordering::Relaxed),
        misses: GPU_STRATEGY_EXECUTABLE_CACHE_MISSES.load(Ordering::Relaxed),
        entries,
        uploaded_thread_entries,
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn ensure_uploaded_gpu_strategy_executable_for_current_thread(
    key: &GpuStrategyExecutableCacheKey,
    device_id: u32,
    handle: &StrategyHandle,
) -> Result<(), String> {
    let abi = validated_gpu_program_abi(handle)?;
    let key_for_upload = key.clone();
    GPU_STRATEGY_EXECUTABLE_UPLOAD_CACHE.with(|cell| -> Result<(), String> {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&key_for_upload) {
            let uploaded =
                crate::gpu_backtest_kernel::upload_gpu_strategy_program_abi(device_id, abi)?;
            let probe = crate::gpu_backtest_kernel::probe_uploaded_gpu_strategy_program(&uploaded)?;
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
    })
}

pub fn get_or_build_gpu_strategy_executable(
    handle: &StrategyHandle,
    device_id: u32,
) -> Result<GpuStrategyExecutable, String> {
    let abi = validated_gpu_program_abi(handle)?;
    let executable_fingerprint_hex = abi.executable_fingerprint_hex.clone();
    let key = GpuStrategyExecutableCacheKey::new(executable_fingerprint_hex, device_id);
    let cache = GPU_STRATEGY_EXECUTABLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let guard = cache
            .lock()
            .map_err(|_| "gpu strategy executable cache lock poisoned".to_string())?;
        if let Some(existing) = guard.get(&key) {
            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            ensure_uploaded_gpu_strategy_executable_for_current_thread(&key, device_id, handle)?;
            GPU_STRATEGY_EXECUTABLE_CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            return Ok(existing.clone());
        }
    }

    let exec = GpuStrategyExecutable::from_handle(handle, device_id)?;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        ensure_uploaded_gpu_strategy_executable_for_current_thread(&key, device_id, handle)?;
    }
    let mut guard = cache
        .lock()
        .map_err(|_| "gpu strategy executable cache lock poisoned".to_string())?;
    let entry = match guard.entry(key) {
        std::collections::hash_map::Entry::Occupied(existing) => {
            GPU_STRATEGY_EXECUTABLE_CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            existing.get().clone()
        }
        std::collections::hash_map::Entry::Vacant(vacant) => {
            GPU_STRATEGY_EXECUTABLE_CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
            vacant.insert(exec).clone()
        }
    };
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
    use std::sync::{Mutex, MutexGuard};
    use ta_strategies::double_ma::StrategyConfig;

    static STRATEGY_EXECUTABLE_CACHE_TEST_GUARD: Mutex<()> = Mutex::new(());

    fn lock_strategy_executable_cache_test() -> MutexGuard<'static, ()> {
        STRATEGY_EXECUTABLE_CACHE_TEST_GUARD
            .lock()
            .expect("strategy executable cache test guard lock poisoned")
    }

    #[test]
    fn strategy_handle_builds_gpu_program_and_abi_for_gpu_unified_plan() {
        let _guard = lock_strategy_executable_cache_test();
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
        let _guard = lock_strategy_executable_cache_test();
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
    fn executable_cache_rejects_empty_compiled_plan_fingerprint_from_abi() {
        let _guard = lock_strategy_executable_cache_test();
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
        let mut handle =
            StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
                .expect("build strategy handle");
        handle
            .gpu_program_abi
            .as_mut()
            .expect("abi expected")
            .compiled_plan_fingerprint_hex
            .clear();

        let err = get_or_build_gpu_strategy_executable(&handle, 0)
            .expect_err("empty compiled-plan fingerprint should fail");
        assert!(err.contains("empty compiled_plan_fingerprint_hex"));

        let stats = gpu_strategy_executable_cache_stats();
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn executable_cache_rejects_non_current_abi_version() {
        let _guard = lock_strategy_executable_cache_test();
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
        let mut handle =
            StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
                .expect("build strategy handle");
        handle
            .gpu_program_abi
            .as_mut()
            .expect("abi expected")
            .abi_version += 1;

        let err = get_or_build_gpu_strategy_executable(&handle, 0)
            .expect_err("abi version mismatch should fail");
        assert!(err.contains("does not match runtime version"));

        let stats = gpu_strategy_executable_cache_stats();
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn executable_cache_rejects_program_and_abi_fingerprint_mismatch() {
        let _guard = lock_strategy_executable_cache_test();
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
        let mut handle =
            StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
                .expect("build strategy handle");
        handle
            .gpu_program_abi
            .as_mut()
            .expect("abi expected")
            .executable_fingerprint_hex
            .push_str("_mismatch");

        let err = get_or_build_gpu_strategy_executable(&handle, 0)
            .expect_err("program/abi fingerprint mismatch should fail");
        assert!(err.contains("does not match ABI executable fingerprint"));
    }

    #[test]
    fn executable_cache_reuses_identical_handle_and_device() {
        let _guard = lock_strategy_executable_cache_test();
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
        assert_eq!(stats.compiled_bundle_hits, 0);
        assert_eq!(stats.compiled_bundle_misses, 1);
        assert_eq!(stats.compiled_bundle_entries, 1);
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn strategy_handle_reuses_compiled_gpu_bundle_for_identical_plan() {
        let _guard = lock_strategy_executable_cache_test();
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

        let first =
            StrategyHandle::from_plan(plan.clone(), StrategyCapabilityReport::gpu_dynamic_ready())
                .expect("first handle");
        let second = StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
            .expect("second handle");
        let stats = gpu_strategy_executable_cache_stats();

        assert_eq!(
            first
                .gpu_program_abi
                .as_ref()
                .map(|abi| abi.executable_fingerprint_hex.as_str()),
            second
                .gpu_program_abi
                .as_ref()
                .map(|abi| abi.executable_fingerprint_hex.as_str())
        );
        assert_eq!(stats.compiled_bundle_entries, 1);
        assert_eq!(stats.compiled_bundle_hits, 1);
        assert_eq!(stats.compiled_bundle_misses, 1);
    }

    #[test]
    fn executable_cache_key_tracks_compiled_gpu_shape_not_only_ir_identity() {
        let _guard = lock_strategy_executable_cache_test();
        reset_gpu_strategy_executable_cache();
        let plan = compile_dynamic_plan_for_double_ma_config(
            "close",
            "sma",
            "alma",
            None,
            None,
            &StrategyConfig::default(),
            None,
            StrategyPlanBackend::GpuUnified,
        )
        .expect("compile dynamic plan");
        let mut overrides = std::collections::HashMap::new();
        overrides.insert(
            plan.indicator_plan
                .first()
                .expect("indicator expected")
                .indicator_id
                .clone(),
            crate::strategy_plan::IndicatorRouteKind::Cpu,
        );
        let ir = crate::strategy_ir::build_static_strategy_ir(
            crate::strategy_ir::StrategyProfileKind::DoubleMa,
            crate::strategy_ir::StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "alma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("build ir");
        let overridden_plan = crate::strategy_plan::compile_strategy_plan(
            &ir,
            StrategyPlanBackend::GpuUnified,
            Some(&overrides),
        )
        .expect("compile overridden plan");
        let base_handle =
            StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
                .expect("base handle");
        let overridden_handle = StrategyHandle::from_plan(
            overridden_plan,
            StrategyCapabilityReport::gpu_dynamic_ready(),
        )
        .expect("overridden handle");
        let base_key = GpuStrategyExecutableCacheKey::new(
            base_handle
                .gpu_program_abi
                .as_ref()
                .expect("base abi")
                .executable_fingerprint_hex
                .clone(),
            0,
        );
        let overridden_key = GpuStrategyExecutableCacheKey::new(
            overridden_handle
                .gpu_program_abi
                .as_ref()
                .expect("overridden abi")
                .executable_fingerprint_hex
                .clone(),
            0,
        );

        assert_ne!(base_key, overridden_key);
    }

    #[test]
    fn reset_gpu_strategy_executable_cache_clears_entries_and_counters() {
        let _guard = lock_strategy_executable_cache_test();
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
        let _exec = get_or_build_gpu_strategy_executable(&handle, 0).expect("build executable");

        let warm_stats = gpu_strategy_executable_cache_stats();
        assert_eq!(warm_stats.compiled_bundle_entries, 1);
        assert_eq!(warm_stats.entries, 1);
        assert!(warm_stats.compiled_bundle_misses >= 1);
        assert!(warm_stats.misses >= 1);

        reset_gpu_strategy_executable_cache();
        let cleared_stats = gpu_strategy_executable_cache_stats();
        assert_eq!(cleared_stats.compiled_bundle_hits, 0);
        assert_eq!(cleared_stats.compiled_bundle_misses, 0);
        assert_eq!(cleared_stats.compiled_bundle_entries, 0);
        assert_eq!(cleared_stats.hits, 0);
        assert_eq!(cleared_stats.misses, 0);
        assert_eq!(cleared_stats.entries, 0);
        assert_eq!(cleared_stats.uploaded_thread_entries, 0);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn uploaded_executable_is_available_after_cache_build() {
        let _guard = lock_strategy_executable_cache_test();
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
        let handle = StrategyHandle::from_plan(plan, StrategyCapabilityReport::gpu_dynamic_ready())
            .expect("build strategy handle");
        let key = GpuStrategyExecutableCacheKey::new(
            handle
                .gpu_program_abi
                .as_ref()
                .expect("abi expected")
                .executable_fingerprint_hex
                .clone(),
            0,
        );

        let exec = get_or_build_gpu_strategy_executable(&handle, 0).expect("build executable");
        let uploaded = with_uploaded_gpu_strategy_executable(&key, |uploaded| {
            (
                uploaded.abi_version,
                uploaded.plan_fingerprint_hex.clone(),
                uploaded.executable_fingerprint_hex.clone(),
                uploaded.device_id,
                uploaded.indicator_slots_ptr_raw(),
            )
        })
        .expect("uploaded executable should exist");
        assert_eq!(uploaded.0, exec.abi_version);
        assert_eq!(uploaded.1, exec.plan_fingerprint_hex);
        assert_eq!(uploaded.2, exec.executable_fingerprint_hex);
        assert_eq!(uploaded.3, exec.device_id);
        assert!(uploaded.4 > 0);
    }
}
