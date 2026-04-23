use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::sync::OnceLock;

use libloading::Library;

const NVTX_ENV: &str = "VECTORBT_ENABLE_NVTX";
const NVTX_VERSION: u16 = 3;
const NVTX_COLOR_ARGB: i32 = 1;
const NVTX_MESSAGE_TYPE_ASCII: i32 = 1;

const ENGINE_DOMAIN_NAME: &[u8] = b"engine\0";
const KERNEL_DOMAIN_NAME: &[u8] = b"kernel\0";

type NvtxDomainHandle = *mut c_void;

type NvtxDomainCreateAFn = unsafe extern "system" fn(*const c_char) -> NvtxDomainHandle;
type NvtxDomainRangePushExFn =
    unsafe extern "system" fn(NvtxDomainHandle, *const NvtxEventAttributes) -> c_int;
type NvtxDomainRangePopFn = unsafe extern "system" fn(NvtxDomainHandle) -> c_int;
type NvtxRangePushAFn = unsafe extern "system" fn(*const c_char) -> c_int;
type NvtxRangePushExFn = unsafe extern "system" fn(*const NvtxEventAttributes) -> c_int;
type NvtxRangePopFn = unsafe extern "system" fn() -> c_int;

#[derive(Clone, Copy)]
pub(crate) enum Domain {
    Engine,
    Kernel,
}

pub(crate) mod color {
    pub(crate) const STEADY_ITERATION: u32 = 0xFF4E79A7;
    pub(crate) const PREPARE_INPUTS: u32 = 0xFFF28E2B;
    pub(crate) const BACKEND_EXECUTE: u32 = 0xFFE15759;
    pub(crate) const BUILD_RESULT: u32 = 0xFF76B7B2;
    pub(crate) const HYBRID_PLAN: u32 = 0xFF59A14F;
    pub(crate) const HYBRID_PREPARE: u32 = 0xFFEDC948;
    pub(crate) const HYBRID_TRANSFER: u32 = 0xFFB07AA1;
    pub(crate) const HYBRID_KERNEL: u32 = 0xFFFF9DA7;
    pub(crate) const FALLBACK: u32 = 0xFF9C755F;
    pub(crate) const INDICATOR_CHAIN: u32 = 0xFFBAB0AC;
    pub(crate) const BACKTEST_CORE: u32 = 0xFF86BCB6;
    pub(crate) const TOPK_SELECT: u32 = 0xFFF1A340;
    pub(crate) const COPY_OUT: u32 = 0xFF998EC3;
}

#[derive(Clone, Copy)]
struct NvtxApi {
    domain_create_a: Option<NvtxDomainCreateAFn>,
    domain_range_push_ex: Option<NvtxDomainRangePushExFn>,
    domain_range_pop: Option<NvtxDomainRangePopFn>,
    range_push_a: NvtxRangePushAFn,
    range_push_ex: NvtxRangePushExFn,
    range_pop: NvtxRangePopFn,
}

#[repr(C)]
union NvtxPayloadValue {
    ull_value: u64,
    ll_value: i64,
    d_value: f64,
    ui_value: u32,
    i_value: i32,
    f_value: f32,
}

#[repr(C)]
union NvtxMessageValue {
    ascii: *const c_char,
    unicode: *const u16,
    registered: *mut c_void,
}

#[repr(C)]
struct NvtxEventAttributes {
    version: u16,
    size: u16,
    category: u32,
    color_type: i32,
    color: u32,
    payload_type: i32,
    reserved0: i32,
    payload: NvtxPayloadValue,
    message_type: i32,
    message: NvtxMessageValue,
}

impl NvtxEventAttributes {
    fn ascii(name: &CStr, color: u32) -> Self {
        Self {
            version: NVTX_VERSION,
            size: std::mem::size_of::<Self>() as u16,
            category: 0,
            color_type: NVTX_COLOR_ARGB,
            color,
            payload_type: 0,
            reserved0: 0,
            payload: NvtxPayloadValue { ull_value: 0 },
            message_type: NVTX_MESSAGE_TYPE_ASCII,
            message: NvtxMessageValue {
                ascii: name.as_ptr(),
            },
        }
    }
}

pub struct RangeGuard {
    domain: Option<NvtxDomainHandle>,
    pushed_default_range: bool,
    default_name: Option<CString>,
}

impl RangeGuard {
    fn disabled() -> Self {
        Self {
            domain: None,
            pushed_default_range: false,
            default_name: None,
        }
    }
}

impl Drop for RangeGuard {
    fn drop(&mut self) {
        let Some(api) = api() else {
            return;
        };
        if let Some(domain) = self.domain {
            if let Some(domain_range_pop) = api.domain_range_pop {
                unsafe {
                    domain_range_pop(domain);
                }
            }
        }
        if self.pushed_default_range {
            unsafe {
                (api.range_pop)();
            }
        }
    }
}

pub(crate) fn engine_range(name: &str, color: u32) -> RangeGuard {
    range(Domain::Engine, name, color)
}

pub(crate) fn kernel_range(name: &str, color: u32) -> RangeGuard {
    range(Domain::Kernel, name, color)
}

pub fn scope(domain: &str, name: &str) -> RangeGuard {
    let color = default_color(name);
    match domain {
        "engine" => engine_range(name, color),
        "kernel" => kernel_range(name, color),
        _ => RangeGuard::disabled(),
    }
}

fn range(domain: Domain, name: &str, color: u32) -> RangeGuard {
    if !env_flag_enabled(NVTX_ENV, false) {
        return RangeGuard::disabled();
    }
    let Some(api) = api() else {
        return RangeGuard::disabled();
    };
    let domain_handle = None;
    let can_use_domain_api = false;
    let pushed_default_range = true;
    let default_name = if pushed_default_range {
        fallback_default_range_name(domain, name)
    } else {
        None
    };
    let default_name_c = match default_name {
        Some(ref value) => match CString::new(value.as_str()) {
            Ok(v) => Some(v),
            Err(_) => return RangeGuard::disabled(),
        },
        None => None,
    };
    let domain_name_c = if can_use_domain_api {
        match CString::new(name) {
            Ok(v) => Some(v),
            Err(_) => return RangeGuard::disabled(),
        }
    } else {
        None
    };
    unsafe {
        if let Some(ref default_name_c) = default_name_c {
            let _ = color;
            (api.range_push_a)(default_name_c.as_ptr());
        }
        if let (Some(domain_handle), Some(domain_range_push_ex)) =
            (domain_handle, api.domain_range_push_ex)
        {
            let domain_name_c = domain_name_c
                .as_ref()
                .expect("domain name must exist when domain API is enabled");
            let domain_attrs = NvtxEventAttributes::ascii(domain_name_c.as_c_str(), color);
            domain_range_push_ex(domain_handle, &domain_attrs);
        }
    }
    RangeGuard {
        domain: if can_use_domain_api {
            domain_handle
        } else {
            None
        },
        pushed_default_range,
        default_name: default_name_c,
    }
}

fn should_mirror_default_range(domain: Domain, name: &str) -> bool {
    matches!(domain, Domain::Engine)
        && matches!(
            name,
            "steady_iteration"
                | "double_ma_optimization"
                | "double_ma_opt_total"
                | "tauri_double_ma_request"
                | "options_expression_run_total"
                | "options_expression_feasibility_total"
                | "tauri_options_expression_request"
                | "tauri_options_expression_feasibility_request"
        )
}

fn domain_handle(domain: Domain) -> Option<NvtxDomainHandle> {
    match domain {
        Domain::Engine => ENGINE_DOMAIN
            .get_or_init(|| create_domain(ENGINE_DOMAIN_NAME))
            .map(|handle| handle as NvtxDomainHandle),
        Domain::Kernel => KERNEL_DOMAIN
            .get_or_init(|| create_domain(KERNEL_DOMAIN_NAME))
            .map(|handle| handle as NvtxDomainHandle),
    }
}

fn create_domain(name: &[u8]) -> Option<usize> {
    let api = api()?;
    let domain_create_a = api.domain_create_a?;
    let ptr = unsafe { domain_create_a(name.as_ptr().cast()) };
    if ptr.is_null() {
        None
    } else {
        Some(ptr as usize)
    }
}

fn api() -> Option<NvtxApi> {
    *NVTX_API.get_or_init(|| unsafe { try_load_api() })
}

unsafe fn try_load_api() -> Option<NvtxApi> {
    for candidate in library_candidates() {
        let Ok(lib) = Library::new(candidate) else {
            continue;
        };
        let Some(range_push_a) = load_symbol::<NvtxRangePushAFn>(&lib, b"nvtxRangePushA\0") else {
            continue;
        };
        let Some(range_push_ex) = load_symbol::<NvtxRangePushExFn>(&lib, b"nvtxRangePushEx\0")
        else {
            continue;
        };
        let Some(range_pop) = load_symbol::<NvtxRangePopFn>(&lib, b"nvtxRangePop\0") else {
            continue;
        };
        let domain_create_a = load_symbol::<NvtxDomainCreateAFn>(&lib, b"nvtxDomainCreateA\0");
        let domain_range_push_ex =
            load_symbol::<NvtxDomainRangePushExFn>(&lib, b"nvtxDomainRangePushEx\0");
        let domain_range_pop = load_symbol::<NvtxDomainRangePopFn>(&lib, b"nvtxDomainRangePop\0");
        let has_full_domain_api = domain_create_a.is_some()
            && domain_range_push_ex.is_some()
            && domain_range_pop.is_some();
        std::mem::forget(lib);
        return Some(NvtxApi {
            domain_create_a: if has_full_domain_api {
                domain_create_a
            } else {
                None
            },
            domain_range_push_ex: if has_full_domain_api {
                domain_range_push_ex
            } else {
                None
            },
            domain_range_pop: if has_full_domain_api {
                domain_range_pop
            } else {
                None
            },
            range_push_a,
            range_push_ex,
            range_pop,
        });
    }
    None
}

unsafe fn load_symbol<T: Copy>(lib: &Library, name: &[u8]) -> Option<T> {
    let symbol = lib.get::<T>(name).ok()?;
    Some(*symbol)
}

fn env_flag_enabled(name: &str, default_enabled: bool) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false"))
        }
        Err(_) => default_enabled,
    }
}

fn library_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Some(program_files) = std::env::var_os("ProgramFiles") {
            let path = std::path::PathBuf::from(program_files)
                .join("NVIDIA Corporation")
                .join("NvToolsExt")
                .join("bin")
                .join("x64")
                .join("nvToolsExt64_1.dll");
            candidates.push(path.to_string_lossy().into_owned());
        }
        if let Some(program_w6432) = std::env::var_os("ProgramW6432") {
            let path = std::path::PathBuf::from(program_w6432)
                .join("NVIDIA Corporation")
                .join("NvToolsExt")
                .join("bin")
                .join("x64")
                .join("nvToolsExt64_1.dll");
            candidates.push(path.to_string_lossy().into_owned());
        }
        candidates.push("nvToolsExt64_1.dll".to_string());
        candidates.push("nvToolsExt.dll".to_string());
    }
    #[cfg(target_os = "linux")]
    {
        candidates.push("libnvToolsExt.so.1".to_string());
        candidates.push("libnvToolsExt.so".to_string());
    }
    candidates
}

static NVTX_API: OnceLock<Option<NvtxApi>> = OnceLock::new();
static ENGINE_DOMAIN: OnceLock<Option<usize>> = OnceLock::new();
static KERNEL_DOMAIN: OnceLock<Option<usize>> = OnceLock::new();

fn default_color(name: &str) -> u32 {
    match name {
        "steady_iteration"
        | "double_ma_optimization"
        | "double_ma_opt_total"
        | "tauri_double_ma_request" => color::STEADY_ITERATION,
        "prepare_inputs" | "route_resolve" => color::PREPARE_INPUTS,
        "options_replay_build_total"
        | "options_parent_artifact_validation"
        | "options_source_resolution"
        | "options_parent_signal_process_cache_lookup"
        | "options_persisted_signal_manifest_cache_lookup"
        | "options_candle_replay_load"
        | "options_parent_replay_and_drilldown"
        | "options_signal_manifest_materialization"
        | "options_dataset_manifest_validation"
        | "options_dataset_process_cache_lookup"
        | "options_dataset_persisted_cache_lookup"
        | "options_dataset_csv_load"
        | "options_request_normalization"
        | "options_prepare"
        | "options_tuple_compile"
        | "options_prepare_baseline_candidates"
        | "options_prepare_baseline_exit_summaries"
        | "options_prepare_conservative_candidates"
        | "options_prepare_conservative_exit_summaries"
        | "options_prepare_harsher_candidates"
        | "options_prepare_harsher_exit_summaries" => color::PREPARE_INPUTS,
        "backend_result_build" | "build_result" | "result_finalize" | "host_finalize" => {
            color::BUILD_RESULT
        }
        "gpu_kernel_route" | "gpu_kernel_dispatch" | "backend_execute" => color::BACKEND_EXECUTE,
        "options_expression_run_total"
        | "options_expression_feasibility_total"
        | "tauri_options_expression_request"
        | "tauri_options_expression_feasibility_request"
        | "options_engine_run_total"
        | "options_engine_feasibility_total"
        | "options_gpu_rank_total"
        | "options_gpu_tensor_score"
        | "options_feasibility_cpu_rank"
        | "options_feasibility_gpu_plan_probe"
        | "options_feasibility_gpu_rank_probe"
        | "options_cpu_rank_total" => color::BACKEND_EXECUTE,
        "options_exactify_total"
        | "options_exactify_summary_overwrite"
        | "options_exactify_enrichment"
        | "options_exactify_extreme"
        | "options_sealed_stage_total"
        | "options_sealed_stage_non_extreme"
        | "options_sealed_stage_extreme"
        | "options_result_finalize" => color::BUILD_RESULT,
        "hybrid_plan" => color::HYBRID_PLAN,
        "hybrid_prepare" => color::HYBRID_PREPARE,
        "hybrid_transfer" => color::HYBRID_TRANSFER,
        "hybrid_kernel" => color::HYBRID_KERNEL,
        "hybrid_fallback_gpu_sweep" | "hybrid_fallback_cpu" => color::FALLBACK,
        "indicator_chain" | "gpu_indicator_prep" | "graph_replay" => color::INDICATOR_CHAIN,
        "executor" | "executor_impl" | "kernel_executor_total" | "backtest_core" => {
            color::BACKTEST_CORE
        }
        "topk_select" | "launch_submit" => color::TOPK_SELECT,
        "copy_out" | "h2d_upload" | "sync_and_decode" => color::COPY_OUT,
        _ => color::BACKEND_EXECUTE,
    }
}

fn fallback_default_range_name(domain: Domain, name: &str) -> Option<String> {
    if should_mirror_default_range(domain, name) {
        return Some(name.to_string());
    }
    let prefix = match domain {
        Domain::Engine => "engine",
        Domain::Kernel => "kernel",
    };
    Some(format!("{prefix}::{name}"))
}

#[cfg(test)]
mod tests {
    use super::{fallback_default_range_name, Domain};

    #[test]
    fn fallback_preserves_top_level_engine_capture_names() {
        assert_eq!(
            fallback_default_range_name(Domain::Engine, "steady_iteration").as_deref(),
            Some("steady_iteration")
        );
        assert_eq!(
            fallback_default_range_name(Domain::Engine, "tauri_double_ma_request").as_deref(),
            Some("tauri_double_ma_request")
        );
        assert_eq!(
            fallback_default_range_name(Domain::Engine, "options_expression_feasibility_total")
                .as_deref(),
            Some("options_expression_feasibility_total")
        );
        assert_eq!(
            fallback_default_range_name(Domain::Engine, "tauri_options_expression_request")
                .as_deref(),
            Some("tauri_options_expression_request")
        );
    }

    #[test]
    fn fallback_prefixes_nested_ranges_when_domain_api_is_missing() {
        assert_eq!(
            fallback_default_range_name(Domain::Engine, "prepare_inputs").as_deref(),
            Some("engine::prepare_inputs")
        );
        assert_eq!(
            fallback_default_range_name(Domain::Kernel, "backtest_core").as_deref(),
            Some("kernel::backtest_core")
        );
    }
}
