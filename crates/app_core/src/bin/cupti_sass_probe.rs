#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn main() {
    eprintln!("cupti_sass_probe requires --features cuda,cuda-backtest-kernel");
    std::process::exit(1);
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[path = "../bin_support/benchmark_lock.rs"]
mod benchmark_lock;

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
mod real {
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::fs;
    use std::mem::{offset_of, size_of};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use cust::device::Device;
    use cust::sys as cuda;
    use libloading::Library;
    use serde::Serialize;
    use serde_json::json;
    use ta_app_core::double_ma::{DoubleMaMetricContract, DoubleMaShortlistSliceMode};
    use ta_app_core::gpu_backtest_kernel::prepare_cuda_kernel_runtime_for_probe;
    use ta_app_core::{
        run_double_ma_optimization_blocking_with_candles_and_eval_range, Backend, BackendUsed,
        DoubleMaRequest, SpaBenchmarkKind,
    };
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::StrategyConfig;
    use vector_ta::utilities::data_loader::Candles;

    const DEFAULT_METRIC: &str = "smsp__sass_inst_executed";
    const CUPTI_PROFILER_SASS_METRICS: i32 = 2;
    const CUPTI_PROFILER_CONFIGURATION_SUPPORTED: i32 = 3;
    const CUPTI_SASS_METRICS_OUTPUT_GRANULARITY_GPU: u8 = 0;

    #[derive(Clone, Copy)]
    enum WorkloadKind {
        Large,
        Xlarge,
    }

    #[derive(Clone, Copy)]
    enum CandleShape {
        Auto,
        Synth,
        Trending,
        Choppy,
        Breakout,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum CommandMode {
        Inventory,
        Capture,
    }

    #[derive(Clone, Serialize)]
    struct InventoryMetric {
        metric_id: u64,
        metric_name: String,
        metric_description: String,
    }

    #[derive(Serialize)]
    struct InventoryOutput {
        run_id: String,
        capture_status: String,
        gpu_name: String,
        driver_version: i32,
        cuda_path: String,
        cupti_dll_path: String,
        chip_name: String,
        device_supported: bool,
        support_flags: BTreeMap<String, i32>,
        metric_count: usize,
        metrics: Vec<InventoryMetric>,
        notes: Vec<String>,
    }

    #[derive(Serialize)]
    struct SurfaceSpec {
        strategy: String,
        workload: String,
        fast_ma: String,
        slow_ma: String,
        metric_contract: String,
        shortlist_slice_mode: String,
        shortlist_slice_benchmark: String,
        candle_shape: String,
        device_id: u32,
    }

    #[derive(Serialize)]
    struct RuntimeSummary {
        backend_used: String,
        backend_route: String,
        runtime_ms: u64,
        wall_ms: u64,
        best_fast_len: u16,
        best_slow_len: u16,
    }

    #[derive(Serialize)]
    struct PcMetricRecord {
        pc_offset: u32,
        values: BTreeMap<String, u64>,
    }

    #[derive(Serialize)]
    struct KernelMetricSummary {
        function_name: String,
        cubin_crc: u32,
        function_index: u32,
        record_count: usize,
        metric_totals: BTreeMap<String, u64>,
        pc_records: Vec<PcMetricRecord>,
    }

    #[derive(Serialize)]
    struct CaptureOutput {
        run_id: String,
        gpu_name: String,
        driver_version: i32,
        cuda_path: String,
        cupti_dll_path: String,
        chip_name: String,
        target_surface: SurfaceSpec,
        target_kernel_filter: String,
        metric_inventory_used: Vec<String>,
        capture_status: String,
        support_flags: BTreeMap<String, i32>,
        runtime_summary: RuntimeSummary,
        observed_kernel_names: Vec<String>,
        matched_kernel_names: Vec<String>,
        matched_instruction_records: usize,
        region_metrics: Vec<KernelMetricSummary>,
        notes: Vec<String>,
    }

    #[derive(Clone)]
    struct ProbeConfig {
        mode: CommandMode,
        device_id: u32,
        workload: WorkloadKind,
        metric_contract: Option<DoubleMaMetricContract>,
        shortlist_slice_mode: Option<DoubleMaShortlistSliceMode>,
        shortlist_slice_benchmark: Option<SpaBenchmarkKind>,
        candle_shape: CandleShape,
        metrics: Vec<String>,
        kernel_filter: String,
        output_dir: PathBuf,
        enable_lazy_patching: bool,
        include_local_paths: bool,
        include_command_line: bool,
    }

    #[derive(Clone)]
    struct SupportStatus {
        overall: i32,
        architecture: i32,
        sli: i32,
        vgpu: i32,
        confidential_compute: i32,
        cmp: i32,
        wsl: i32,
        sku: i32,
    }

    #[derive(Clone)]
    struct MetricCatalog {
        chip_name: String,
        metrics: Vec<InventoryMetric>,
        by_name: BTreeMap<String, u64>,
    }

    #[repr(C)]
    struct CUpti_Profiler_Initialize_Params {
        struct_size: usize,
        p_priv: *mut c_void,
    }

    #[repr(C)]
    struct CUpti_Profiler_DeviceSupported_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        cu_device: cuda::CUdevice,
        is_supported: i32,
        architecture: i32,
        sli: i32,
        v_gpu: i32,
        confidential_compute: i32,
        cmp: i32,
        wsl: i32,
        api: i32,
        sku: i32,
    }

    #[repr(C)]
    struct CUpti_Device_GetChipName_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        device_index: usize,
        p_chip_name: *const c_char,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CUpti_SassMetrics_MetricDetails {
        metric_id: u64,
        p_metric_name: *const c_char,
        p_metric_description: *const c_char,
    }

    #[repr(C)]
    struct CUpti_SassMetrics_GetNumOfMetrics_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        p_chip_name: *const c_char,
        num_of_metrics: usize,
    }

    #[repr(C)]
    struct CUpti_SassMetrics_GetMetrics_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        p_chip_name: *const c_char,
        num_of_metrics: usize,
        p_metrics_list: *mut CUpti_SassMetrics_MetricDetails,
    }

    #[repr(C)]
    struct CUpti_SassMetrics_GetProperties_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        p_chip_name: *const c_char,
        p_metric_name: *const c_char,
        metric: CUpti_SassMetrics_MetricDetails,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CUpti_SassMetrics_Config {
        metric_id: u64,
        output_granularity: u8,
    }

    #[repr(C)]
    struct CUpti_SassMetricsSetConfig_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        num_of_metric_config: usize,
        p_configs: *mut CUpti_SassMetrics_Config,
        device_index: u32,
    }

    #[repr(C)]
    struct CUpti_SassMetricsUnsetConfig_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        device_index: u32,
    }

    #[repr(C)]
    struct CUpti_SassMetricsEnable_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        ctx: cuda::CUcontext,
        enable_lazy_patching: u8,
    }

    #[repr(C)]
    struct CUpti_SassMetricsDisable_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        ctx: cuda::CUcontext,
        num_of_dropped_records: usize,
    }

    #[repr(C)]
    struct CUpti_SassMetricsGetDataProperties_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        ctx: cuda::CUcontext,
        num_of_patched_instruction_records: usize,
        num_of_instances: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CUpti_SassMetrics_InstanceValue {
        metric_id: u64,
        value: u64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CUpti_SassMetrics_Data {
        struct_size: usize,
        p_priv: *mut c_void,
        cubin_crc: u32,
        function_index: u32,
        function_name: *const c_char,
        pc_offset: u32,
        p_instance_values: *mut CUpti_SassMetrics_InstanceValue,
    }

    #[repr(C)]
    struct CUpti_SassMetricsFlushData_Params {
        struct_size: usize,
        p_priv: *mut c_void,
        ctx: cuda::CUcontext,
        num_of_patched_instruction_records: usize,
        num_of_instances: usize,
        p_metrics_data: *mut CUpti_SassMetrics_Data,
    }

    struct CuptiApi {
        _lib: Library,
        cupti_get_result_string: unsafe extern "system" fn(i32, *mut *const c_char) -> i32,
        cupti_profiler_initialize:
            unsafe extern "system" fn(*mut CUpti_Profiler_Initialize_Params) -> i32,
        cupti_profiler_device_supported:
            unsafe extern "system" fn(*mut CUpti_Profiler_DeviceSupported_Params) -> i32,
        cupti_device_get_chip_name:
            unsafe extern "system" fn(*mut CUpti_Device_GetChipName_Params) -> i32,
        cupti_sass_metrics_get_num_of_metrics:
            unsafe extern "system" fn(*mut CUpti_SassMetrics_GetNumOfMetrics_Params) -> i32,
        cupti_sass_metrics_get_metrics:
            unsafe extern "system" fn(*mut CUpti_SassMetrics_GetMetrics_Params) -> i32,
        cupti_sass_metrics_get_properties:
            unsafe extern "system" fn(*mut CUpti_SassMetrics_GetProperties_Params) -> i32,
        cupti_sass_metrics_set_config:
            unsafe extern "system" fn(*mut CUpti_SassMetricsSetConfig_Params) -> i32,
        cupti_sass_metrics_unset_config:
            unsafe extern "system" fn(*mut CUpti_SassMetricsUnsetConfig_Params) -> i32,
        cupti_sass_metrics_enable:
            unsafe extern "system" fn(*mut CUpti_SassMetricsEnable_Params) -> i32,
        cupti_sass_metrics_disable:
            unsafe extern "system" fn(*mut CUpti_SassMetricsDisable_Params) -> i32,
        cupti_sass_metrics_get_data_properties:
            unsafe extern "system" fn(*mut CUpti_SassMetricsGetDataProperties_Params) -> i32,
        cupti_sass_metrics_flush_data:
            unsafe extern "system" fn(*mut CUpti_SassMetricsFlushData_Params) -> i32,
        cuda_path: String,
        dll_path: PathBuf,
    }

    impl CuptiApi {
        fn load() -> Result<Self, String> {
            let cuda_path = std::env::var("CUDA_PATH")
                .ok()
                .map(PathBuf::from)
                .filter(|p| p.exists())
                .or_else(|| {
                    [
                        r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.2",
                        r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.0",
                        r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.3",
                    ]
                    .iter()
                    .map(PathBuf::from)
                    .find(|p| p.exists())
                })
                .ok_or_else(|| {
                    "CUDA_PATH was not set and no fallback CUDA toolkit path was found".to_string()
                })?;
            let cupti_dir = cuda_path.join("extras").join("CUPTI").join("lib64");
            let dll_path = fs::read_dir(&cupti_dir)
                .map_err(|e| {
                    format!(
                        "failed to read CUPTI lib directory '{}': {e}",
                        cupti_dir.display()
                    )
                })?
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .find(|p| {
                    p.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.starts_with("cupti64_") && name.ends_with(".dll"))
                        .unwrap_or(false)
                })
                .ok_or_else(|| format!("no cupti64_*.dll found under '{}'", cupti_dir.display()))?;

            if let Ok(existing) = std::env::var("PATH") {
                std::env::set_var("PATH", format!("{};{}", cupti_dir.display(), existing));
            }

            let lib = unsafe { Library::new(&dll_path) }
                .map_err(|e| format!("failed to load '{}': {e}", dll_path.display()))?;

            unsafe fn load_symbol<T: Copy>(lib: &Library, name: &[u8]) -> Result<T, String> {
                let symbol = unsafe { lib.get::<T>(name) }.map_err(|e| {
                    format!(
                        "failed to load symbol '{}': {e}",
                        String::from_utf8_lossy(name)
                    )
                })?;
                Ok(*symbol)
            }

            Ok(Self {
                cupti_get_result_string: unsafe { load_symbol(&lib, b"cuptiGetResultString\0")? },
                cupti_profiler_initialize: unsafe {
                    load_symbol(&lib, b"cuptiProfilerInitialize\0")?
                },
                cupti_profiler_device_supported: unsafe {
                    load_symbol(&lib, b"cuptiProfilerDeviceSupported\0")?
                },
                cupti_device_get_chip_name: unsafe {
                    load_symbol(&lib, b"cuptiDeviceGetChipName\0")?
                },
                cupti_sass_metrics_get_num_of_metrics: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsGetNumOfMetrics\0")?
                },
                cupti_sass_metrics_get_metrics: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsGetMetrics\0")?
                },
                cupti_sass_metrics_get_properties: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsGetProperties\0")?
                },
                cupti_sass_metrics_set_config: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsSetConfig\0")?
                },
                cupti_sass_metrics_unset_config: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsUnsetConfig\0")?
                },
                cupti_sass_metrics_enable: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsEnable\0")?
                },
                cupti_sass_metrics_disable: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsDisable\0")?
                },
                cupti_sass_metrics_get_data_properties: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsGetDataProperties\0")?
                },
                cupti_sass_metrics_flush_data: unsafe {
                    load_symbol(&lib, b"cuptiSassMetricsFlushData\0")?
                },
                _lib: lib,
                cuda_path: cuda_path.display().to_string(),
                dll_path,
            })
        }

        fn result_string(&self, code: i32) -> String {
            let mut ptr: *const c_char = std::ptr::null();
            let status = unsafe { (self.cupti_get_result_string)(code, &mut ptr as *mut _) };
            if status == 0 && !ptr.is_null() {
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned()
            } else {
                format!("CUPTI error {code}")
            }
        }

        fn check(&self, code: i32, op: &str) -> Result<(), String> {
            if code == 0 {
                Ok(())
            } else {
                Err(format!("{op} failed: {}", self.result_string(code)))
            }
        }
    }

    struct SassMetricsEnableGuard<'a> {
        api: &'a CuptiApi,
        ctx: cuda::CUcontext,
        disabled: bool,
    }

    impl<'a> SassMetricsEnableGuard<'a> {
        fn enable(
            api: &'a CuptiApi,
            ctx: cuda::CUcontext,
            enable_lazy_patching: bool,
        ) -> Result<Self, String> {
            let mut enable_params = metric_enable_params(ctx, enable_lazy_patching);
            api.check(
                unsafe { (api.cupti_sass_metrics_enable)(&mut enable_params as *mut _) },
                "cuptiSassMetricsEnable",
            )?;
            Ok(Self {
                api,
                ctx,
                disabled: false,
            })
        }

        fn disable(&mut self) -> Result<usize, String> {
            if self.disabled {
                return Ok(0);
            }
            let mut disable_params = metric_disable_params(self.ctx);
            self.api.check(
                unsafe { (self.api.cupti_sass_metrics_disable)(&mut disable_params as *mut _) },
                "cuptiSassMetricsDisable",
            )?;
            self.disabled = true;
            Ok(disable_params.num_of_dropped_records)
        }
    }

    impl Drop for SassMetricsEnableGuard<'_> {
        fn drop(&mut self) {
            if !self.disabled {
                let mut disable_params = metric_disable_params(self.ctx);
                let _ =
                    unsafe { (self.api.cupti_sass_metrics_disable)(&mut disable_params as *mut _) };
                self.disabled = true;
            }
        }
    }

    fn parse_arg_value(flag: &str) -> Option<String> {
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == flag {
                return args.next();
            }
        }
        None
    }

    fn has_flag(flag: &str) -> bool {
        std::env::args().skip(1).any(|arg| arg == flag)
    }

    fn parse_mode() -> Result<CommandMode, String> {
        let raw = parse_arg_value("--mode").unwrap_or_else(|| "inventory".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "inventory" => Ok(CommandMode::Inventory),
            "capture" => Ok(CommandMode::Capture),
            other => Err(format!(
                "unsupported --mode '{other}' (expected: inventory|capture)"
            )),
        }
    }

    fn parse_device_id() -> Result<u32, String> {
        let raw = parse_arg_value("--device-id").unwrap_or_else(|| "0".to_string());
        raw.trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid --device-id '{}': {e}", raw.trim()))
    }

    fn parse_workload_kind() -> Result<WorkloadKind, String> {
        let raw = parse_arg_value("--workload").unwrap_or_else(|| "large".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "large" => Ok(WorkloadKind::Large),
            "xlarge" => Ok(WorkloadKind::Xlarge),
            other => Err(format!(
                "unsupported --workload '{other}' (expected: large|xlarge)"
            )),
        }
    }

    fn parse_candle_shape() -> Result<CandleShape, String> {
        let raw = parse_arg_value("--candle-shape").unwrap_or_else(|| "auto".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(CandleShape::Auto),
            "synth" => Ok(CandleShape::Synth),
            "trending" | "trend" => Ok(CandleShape::Trending),
            "choppy" => Ok(CandleShape::Choppy),
            "breakout" => Ok(CandleShape::Breakout),
            other => Err(format!(
                "unsupported --candle-shape '{other}' (expected: auto|synth|trending|choppy|breakout)"
            )),
        }
    }

    fn parse_metric_contract() -> Result<Option<DoubleMaMetricContract>, String> {
        let Some(raw) = parse_arg_value("--metric-contract") else {
            return Ok(Some(DoubleMaMetricContract::Full17));
        };
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" | "none" => Ok(None),
            "full17" | "full_17" | "full-17" => Ok(Some(DoubleMaMetricContract::Full17)),
            "trade22" | "trade_22" | "trade-22" => Ok(Some(DoubleMaMetricContract::Trade22)),
            other => Err(format!(
                "unsupported --metric-contract '{other}' (expected: auto|full17|trade22)"
            )),
        }
    }

    fn parse_shortlist_slice_mode() -> Result<Option<DoubleMaShortlistSliceMode>, String> {
        let Some(raw) = parse_arg_value("--shortlist-slice-mode") else {
            return Ok(None);
        };
        match raw.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(None),
            "halves" => Ok(Some(DoubleMaShortlistSliceMode::Halves)),
            "quartiles" => Ok(Some(DoubleMaShortlistSliceMode::Quartiles)),
            other => Err(format!(
                "unsupported --shortlist-slice-mode '{other}' (expected: none|halves|quartiles)"
            )),
        }
    }

    fn parse_shortlist_slice_benchmark() -> Result<Option<SpaBenchmarkKind>, String> {
        let Some(raw) = parse_arg_value("--shortlist-slice-benchmark") else {
            return Ok(None);
        };
        match raw.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(None),
            "flat" => Ok(Some(SpaBenchmarkKind::Flat)),
            "buy_and_hold" | "buy-and-hold" | "buyandhold" => {
                Ok(Some(SpaBenchmarkKind::BuyAndHold))
            }
            other => Err(format!(
                "unsupported --shortlist-slice-benchmark '{other}' (expected: none|flat|buy_and_hold)"
            )),
        }
    }

    fn parse_metrics() -> Vec<String> {
        parse_arg_value("--metric")
            .or_else(|| parse_arg_value("--metrics"))
            .map(|raw| {
                raw.split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(|item| item.to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|metrics| !metrics.is_empty())
            .unwrap_or_else(|| vec![DEFAULT_METRIC.to_string()])
    }

    fn parse_kernel_filter() -> String {
        parse_arg_value("--kernel-filter")
            .unwrap_or_else(|| "double_cross_backtest_tm_flex_f32_lockstep_time".to_string())
    }

    fn parse_output_dir() -> PathBuf {
        parse_arg_value("--output-dir")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("output")
                    .join("profiling")
                    .join("cupti_sass_probe")
            })
    }

    fn metric_contract_label(metric_contract: Option<DoubleMaMetricContract>) -> &'static str {
        match metric_contract {
            Some(DoubleMaMetricContract::Full17) => "full17",
            Some(DoubleMaMetricContract::Trade22) => "trade22",
            None => "auto",
        }
    }

    fn shortlist_slice_mode_label(mode: Option<DoubleMaShortlistSliceMode>) -> &'static str {
        match mode {
            Some(DoubleMaShortlistSliceMode::Halves) => "halves",
            Some(DoubleMaShortlistSliceMode::Quartiles) => "quartiles",
            None => "none",
        }
    }

    fn shortlist_slice_benchmark_label(benchmark: Option<&SpaBenchmarkKind>) -> &'static str {
        match benchmark {
            Some(SpaBenchmarkKind::Flat) => "flat",
            Some(SpaBenchmarkKind::BuyAndHold) => "buy_and_hold",
            None => "none",
        }
    }

    fn candle_shape_label(shape: CandleShape) -> &'static str {
        match shape {
            CandleShape::Auto => "auto",
            CandleShape::Synth => "synth",
            CandleShape::Trending => "trending",
            CandleShape::Choppy => "choppy",
            CandleShape::Breakout => "breakout",
        }
    }

    fn workload_label(kind: WorkloadKind) -> &'static str {
        match kind {
            WorkloadKind::Large => "large",
            WorkloadKind::Xlarge => "xlarge",
        }
    }

    fn workload_ranges(kind: WorkloadKind) -> ((u32, u32, u32), (u32, u32, u32)) {
        match kind {
            WorkloadKind::Large => ((5, 200, 1), (10, 400, 1)),
            WorkloadKind::Xlarge => ((5, 500, 1), (10, 1000, 1)),
        }
    }

    fn make_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        let mut price = 100.0f64;
        for i in 0..len {
            let drift = 0.02 + ((i % 31) as f64 * 0.0003);
            let noise = (((i * 17) % 13) as f64 - 6.0) * 0.01;
            let next = (price + drift + noise).max(1.0);
            ts.push(i as i64);
            open.push(price);
            close.push(next);
            high.push(price.max(next) + 0.3);
            low.push(price.min(next) - 0.3);
            vol.push(1_000.0 + ((i * 37) % 200) as f64);
            price = next;
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    fn make_synth_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        let mut price = 100.0f64;
        for i in 0..len {
            let phase = (i as f64) / 37.0;
            let move_step = phase.sin() * 0.8 + phase.cos() * 0.35;
            let next = (price + move_step).max(1.0);
            ts.push(i as i64);
            open.push(price);
            close.push(next);
            high.push(price.max(next) + 0.25);
            low.push(price.min(next) - 0.25);
            vol.push(800.0 + ((i * 19) % 150) as f64);
            price = next;
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    fn make_choppy_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        let mut price = 100.0f64;
        for i in 0..len {
            let move_step = if i % 2 == 0 { 0.8 } else { -0.78 };
            let next = (price + move_step).max(1.0);
            ts.push(i as i64);
            open.push(price);
            close.push(next);
            high.push(price.max(next) + 0.4);
            low.push(price.min(next) - 0.4);
            vol.push(900.0 + ((i * 11) % 180) as f64);
            price = next;
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    fn make_breakout_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        let mut price = 100.0f64;
        for i in 0..len {
            let breakout = if i % 5_000 == 0 { 8.0 } else { 0.03 };
            let retrace = if i % 11 == 0 { -0.4 } else { 0.0 };
            let next = (price + breakout + retrace).max(1.0);
            ts.push(i as i64);
            open.push(price);
            close.push(next);
            high.push(price.max(next) + 0.5);
            low.push(price.min(next) - 0.5);
            vol.push(1_200.0 + ((i * 29) % 250) as f64);
            price = next;
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    fn candles_for_run(shape: CandleShape, len: usize) -> Candles {
        match shape {
            CandleShape::Auto | CandleShape::Trending => make_trending_candles(len),
            CandleShape::Synth => make_synth_candles(len),
            CandleShape::Choppy => make_choppy_candles(len),
            CandleShape::Breakout => make_breakout_candles(len),
        }
    }

    fn make_request(config: &ProbeConfig) -> DoubleMaRequest {
        let include_trade_metrics = matches!(
            config.metric_contract,
            Some(DoubleMaMetricContract::Trade22)
        );
        let (fast_range, slow_range) = workload_ranges(config.workload);
        DoubleMaRequest {
            backend: Backend::GpuKernel {
                device_id: config.device_id,
            },
            data_id: "cupti-sass-probe".to_string(),
            fast_range,
            slow_range,
            fast_ma_types: vec!["alma".to_string()],
            slow_ma_types: vec!["alma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: config.shortlist_slice_mode,
            shortlist_slice_benchmark: config.shortlist_slice_benchmark.clone(),
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(50),
            include_all: Some(false),
            heatmap_bins: Some(64),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: Default::default(),
            persist_followup_cache: false,
            include_trade_metrics,
            metric_contract: config.metric_contract,
            hurdle_tau: None,
        }
    }

    fn build_probe_config() -> Result<ProbeConfig, String> {
        let mode = parse_mode()?;
        let shortlist_slice_mode = parse_shortlist_slice_mode()?;
        let shortlist_slice_benchmark = parse_shortlist_slice_benchmark()?;
        if shortlist_slice_benchmark.is_some() && shortlist_slice_mode.is_none() {
            return Err("--shortlist-slice-benchmark requires --shortlist-slice-mode".to_string());
        }
        Ok(ProbeConfig {
            mode,
            device_id: parse_device_id()?,
            workload: parse_workload_kind()?,
            metric_contract: parse_metric_contract()?,
            shortlist_slice_mode,
            shortlist_slice_benchmark,
            candle_shape: parse_candle_shape()?,
            metrics: parse_metrics(),
            kernel_filter: parse_kernel_filter(),
            output_dir: parse_output_dir(),
            enable_lazy_patching: !has_flag("--disable-lazy-patching"),
            include_local_paths: has_flag("--include-local-paths"),
            include_command_line: has_flag("--include-command-line"),
        })
    }

    fn new_run_id(prefix: &str) -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{prefix}-{ts}")
    }

    fn cstr_to_string(ptr: *const c_char) -> String {
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned()
        }
    }

    fn support_map(status: &SupportStatus) -> BTreeMap<String, i32> {
        BTreeMap::from([
            ("overall".to_string(), status.overall),
            ("architecture".to_string(), status.architecture),
            ("sli".to_string(), status.sli),
            ("vgpu".to_string(), status.vgpu),
            (
                "confidential_compute".to_string(),
                status.confidential_compute,
            ),
            ("cmp".to_string(), status.cmp),
            ("wsl".to_string(), status.wsl),
            ("sku".to_string(), status.sku),
        ])
    }

    fn cupti_profiler_initialize_params() -> CUpti_Profiler_Initialize_Params {
        CUpti_Profiler_Initialize_Params {
            struct_size: offset_of!(CUpti_Profiler_Initialize_Params, p_priv)
                + size_of::<*mut c_void>(),
            p_priv: std::ptr::null_mut(),
        }
    }

    fn device_supported_params(device: cuda::CUdevice) -> CUpti_Profiler_DeviceSupported_Params {
        CUpti_Profiler_DeviceSupported_Params {
            struct_size: offset_of!(CUpti_Profiler_DeviceSupported_Params, sku) + size_of::<i32>(),
            p_priv: std::ptr::null_mut(),
            cu_device: device,
            is_supported: 0,
            architecture: 0,
            sli: 0,
            v_gpu: 0,
            confidential_compute: 0,
            cmp: 0,
            wsl: 0,
            api: CUPTI_PROFILER_SASS_METRICS,
            sku: 0,
        }
    }

    fn device_get_chip_name_params(device_id: u32) -> CUpti_Device_GetChipName_Params {
        CUpti_Device_GetChipName_Params {
            struct_size: offset_of!(CUpti_Device_GetChipName_Params, p_chip_name)
                + size_of::<*const c_char>(),
            p_priv: std::ptr::null_mut(),
            device_index: device_id as usize,
            p_chip_name: std::ptr::null(),
        }
    }

    fn metric_num_params(chip_name: *const c_char) -> CUpti_SassMetrics_GetNumOfMetrics_Params {
        CUpti_SassMetrics_GetNumOfMetrics_Params {
            struct_size: offset_of!(CUpti_SassMetrics_GetNumOfMetrics_Params, num_of_metrics)
                + size_of::<usize>(),
            p_priv: std::ptr::null_mut(),
            p_chip_name: chip_name,
            num_of_metrics: 0,
        }
    }

    fn metric_list_params(
        chip_name: *const c_char,
        metrics: &mut [CUpti_SassMetrics_MetricDetails],
    ) -> CUpti_SassMetrics_GetMetrics_Params {
        CUpti_SassMetrics_GetMetrics_Params {
            struct_size: offset_of!(CUpti_SassMetrics_GetMetrics_Params, p_metrics_list)
                + size_of::<*mut CUpti_SassMetrics_MetricDetails>(),
            p_priv: std::ptr::null_mut(),
            p_chip_name: chip_name,
            num_of_metrics: metrics.len(),
            p_metrics_list: metrics.as_mut_ptr(),
        }
    }

    fn metric_properties_params(
        chip_name: *const c_char,
        metric_name: *const c_char,
    ) -> CUpti_SassMetrics_GetProperties_Params {
        CUpti_SassMetrics_GetProperties_Params {
            struct_size: offset_of!(CUpti_SassMetrics_GetProperties_Params, metric)
                + size_of::<CUpti_SassMetrics_MetricDetails>(),
            p_priv: std::ptr::null_mut(),
            p_chip_name: chip_name,
            p_metric_name: metric_name,
            metric: CUpti_SassMetrics_MetricDetails {
                metric_id: 0,
                p_metric_name: std::ptr::null(),
                p_metric_description: std::ptr::null(),
            },
        }
    }

    fn metric_set_config_params(
        device_id: u32,
        configs: &mut [CUpti_SassMetrics_Config],
    ) -> CUpti_SassMetricsSetConfig_Params {
        CUpti_SassMetricsSetConfig_Params {
            struct_size: offset_of!(CUpti_SassMetricsSetConfig_Params, device_index)
                + size_of::<u32>(),
            p_priv: std::ptr::null_mut(),
            num_of_metric_config: configs.len(),
            p_configs: configs.as_mut_ptr(),
            device_index: device_id,
        }
    }

    fn metric_unset_config_params(device_id: u32) -> CUpti_SassMetricsUnsetConfig_Params {
        CUpti_SassMetricsUnsetConfig_Params {
            struct_size: offset_of!(CUpti_SassMetricsUnsetConfig_Params, device_index)
                + size_of::<u32>(),
            p_priv: std::ptr::null_mut(),
            device_index: device_id,
        }
    }

    fn metric_enable_params(
        ctx: cuda::CUcontext,
        enable_lazy_patching: bool,
    ) -> CUpti_SassMetricsEnable_Params {
        CUpti_SassMetricsEnable_Params {
            struct_size: offset_of!(CUpti_SassMetricsEnable_Params, enable_lazy_patching)
                + size_of::<u8>(),
            p_priv: std::ptr::null_mut(),
            ctx,
            enable_lazy_patching: if enable_lazy_patching { 1 } else { 0 },
        }
    }

    fn metric_disable_params(ctx: cuda::CUcontext) -> CUpti_SassMetricsDisable_Params {
        CUpti_SassMetricsDisable_Params {
            struct_size: offset_of!(CUpti_SassMetricsDisable_Params, num_of_dropped_records)
                + size_of::<usize>(),
            p_priv: std::ptr::null_mut(),
            ctx,
            num_of_dropped_records: 0,
        }
    }

    fn metric_data_properties_params(
        ctx: cuda::CUcontext,
    ) -> CUpti_SassMetricsGetDataProperties_Params {
        CUpti_SassMetricsGetDataProperties_Params {
            struct_size: offset_of!(CUpti_SassMetricsGetDataProperties_Params, num_of_instances)
                + size_of::<usize>(),
            p_priv: std::ptr::null_mut(),
            ctx,
            num_of_patched_instruction_records: 0,
            num_of_instances: 0,
        }
    }

    fn metric_flush_params(
        ctx: cuda::CUcontext,
        data: &mut [CUpti_SassMetrics_Data],
        num_instances: usize,
    ) -> CUpti_SassMetricsFlushData_Params {
        CUpti_SassMetricsFlushData_Params {
            struct_size: offset_of!(CUpti_SassMetricsFlushData_Params, num_of_instances)
                + size_of::<usize>(),
            p_priv: std::ptr::null_mut(),
            ctx,
            num_of_patched_instruction_records: data.len(),
            num_of_instances: num_instances,
            p_metrics_data: data.as_mut_ptr(),
        }
    }

    fn device_metadata(device_id: u32) -> Result<(String, i32, cuda::CUdevice), String> {
        cust::init(cust::CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let name = device.name().map_err(|e| e.to_string())?;
        let mut driver_version = 0i32;
        unsafe {
            let status = cuda::cuDriverGetVersion(&mut driver_version as *mut i32);
            if status != cuda::CUresult::CUDA_SUCCESS {
                return Err(format!("cuDriverGetVersion failed: {status:?}"));
            }
        }
        Ok((name, driver_version, device.as_raw()))
    }

    fn query_support_and_catalog(
        api: &CuptiApi,
        device_id: u32,
        cu_device: cuda::CUdevice,
    ) -> Result<(SupportStatus, MetricCatalog), String> {
        let mut init_params = cupti_profiler_initialize_params();
        api.check(
            unsafe { (api.cupti_profiler_initialize)(&mut init_params as *mut _) },
            "cuptiProfilerInitialize",
        )?;

        let mut support_params = device_supported_params(cu_device);
        api.check(
            unsafe { (api.cupti_profiler_device_supported)(&mut support_params as *mut _) },
            "cuptiProfilerDeviceSupported",
        )?;

        let support = SupportStatus {
            overall: support_params.is_supported,
            architecture: support_params.architecture,
            sli: support_params.sli,
            vgpu: support_params.v_gpu,
            confidential_compute: support_params.confidential_compute,
            cmp: support_params.cmp,
            wsl: support_params.wsl,
            sku: support_params.sku,
        };

        let mut chip_params = device_get_chip_name_params(device_id);
        api.check(
            unsafe { (api.cupti_device_get_chip_name)(&mut chip_params as *mut _) },
            "cuptiDeviceGetChipName",
        )?;
        let chip_name = cstr_to_string(chip_params.p_chip_name);

        let mut num_params = metric_num_params(chip_params.p_chip_name);
        api.check(
            unsafe { (api.cupti_sass_metrics_get_num_of_metrics)(&mut num_params as *mut _) },
            "cuptiSassMetricsGetNumOfMetrics",
        )?;

        let mut raw_metrics = vec![
            CUpti_SassMetrics_MetricDetails {
                metric_id: 0,
                p_metric_name: std::ptr::null(),
                p_metric_description: std::ptr::null(),
            };
            num_params.num_of_metrics
        ];
        let mut list_params = metric_list_params(chip_params.p_chip_name, &mut raw_metrics);
        api.check(
            unsafe { (api.cupti_sass_metrics_get_metrics)(&mut list_params as *mut _) },
            "cuptiSassMetricsGetMetrics",
        )?;

        let metrics = raw_metrics
            .iter()
            .map(|metric| InventoryMetric {
                metric_id: metric.metric_id,
                metric_name: cstr_to_string(metric.p_metric_name),
                metric_description: cstr_to_string(metric.p_metric_description),
            })
            .collect::<Vec<_>>();
        let by_name = metrics
            .iter()
            .map(|metric| (metric.metric_name.clone(), metric.metric_id))
            .collect::<BTreeMap<_, _>>();

        Ok((
            support,
            MetricCatalog {
                chip_name,
                metrics,
                by_name,
            },
        ))
    }

    fn ensure_output_dir(path: &Path) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|e| {
            format!(
                "failed to create output directory '{}': {e}",
                path.display()
            )
        })
    }

    fn write_json_file<T: Serialize>(path: &Path, payload: &T) -> Result<(), String> {
        let text = serde_json::to_string_pretty(payload)
            .map_err(|e| format!("failed to serialize JSON for '{}': {e}", path.display()))?;
        crate::atomic_output::write_str(path, &text)
            .map_err(|e| format!("failed to write '{}': {e}", path.display()))
    }

    fn run_inventory(config: &ProbeConfig) -> Result<PathBuf, String> {
        ensure_output_dir(&config.output_dir)?;
        let api = CuptiApi::load()?;
        let (gpu_name, driver_version, cu_device) = device_metadata(config.device_id)?;
        let (support, catalog) = query_support_and_catalog(&api, config.device_id, cu_device)?;
        let output = InventoryOutput {
            run_id: new_run_id("cupti-inventory"),
            capture_status: if support.overall == CUPTI_PROFILER_CONFIGURATION_SUPPORTED {
                "supported".to_string()
            } else {
                "unsupported".to_string()
            },
            gpu_name,
            driver_version,
            cuda_path: artifact_path_label(Path::new(&api.cuda_path), config.include_local_paths),
            cupti_dll_path: artifact_path_label(&api.dll_path, config.include_local_paths),
            chip_name: catalog.chip_name,
            device_supported: support.overall == CUPTI_PROFILER_CONFIGURATION_SUPPORTED,
            support_flags: support_map(&support),
            metric_count: catalog.metrics.len(),
            metrics: catalog.metrics,
            notes: vec![
                "Inventory-only mode does not require a CUDA context or workload launch."
                    .to_string(),
                "Metric enumeration came from the local CUPTI SASS-metrics API.".to_string(),
            ],
        };
        let out_path = config.output_dir.join("cupti_metric_inventory.json");
        write_json_file(&out_path, &output)?;
        Ok(out_path)
    }

    fn run_surface(config: &ProbeConfig) -> Result<RuntimeSummary, String> {
        std::env::set_var("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "0");
        std::env::set_var("VECTORBT_ENABLE_NVTX", "0");
        std::env::set_var("VECTORBT_KERNEL_USE_CUDA_GRAPHS", "0");

        let candles = candles_for_run(config.candle_shape, 200_000);
        let cancel = AtomicBool::new(false);
        let request = make_request(config);
        let t0 = Instant::now();
        let result = run_double_ma_optimization_blocking_with_candles_and_eval_range(
            request, &candles, &cancel, None, None,
        )?;
        if !matches!(result.backend_used, BackendUsed::Gpu) {
            return Err(format!(
                "unexpected backend_used={:?}; expected GPU",
                result.backend_used
            ));
        }
        Ok(RuntimeSummary {
            backend_used: format!("{:?}", result.backend_used),
            backend_route: result.backend_route.unwrap_or_default(),
            runtime_ms: result.runtime_ms,
            wall_ms: t0.elapsed().as_millis() as u64,
            best_fast_len: result.best_params.fast_len,
            best_slow_len: result.best_params.slow_len,
        })
    }

    fn capture_surface(config: &ProbeConfig) -> Result<PathBuf, String> {
        ensure_output_dir(&config.output_dir)?;
        let api = CuptiApi::load()?;
        let (gpu_name, driver_version, cu_device) = device_metadata(config.device_id)?;
        let (support, catalog) = query_support_and_catalog(&api, config.device_id, cu_device)?;
        if support.overall != CUPTI_PROFILER_CONFIGURATION_SUPPORTED {
            return Err(format!(
                "CUPTI SASS metrics not supported on current device/configuration: {:?}",
                support_map(&support)
            ));
        }

        for metric_name in &config.metrics {
            if !catalog.by_name.contains_key(metric_name) {
                return Err(format!(
                    "requested metric '{}' is not present in inventory",
                    metric_name
                ));
            }
        }

        let chip_name_c = CString::new(catalog.chip_name.clone())
            .map_err(|e| format!("invalid chip name for CString: {e}"))?;
        let mut metric_configs = Vec::with_capacity(config.metrics.len());
        let mut metric_names_used = Vec::with_capacity(config.metrics.len());
        for metric_name in &config.metrics {
            let metric_name_c = CString::new(metric_name.as_str())
                .map_err(|e| format!("invalid metric name '{}': {e}", metric_name))?;
            let mut props = metric_properties_params(chip_name_c.as_ptr(), metric_name_c.as_ptr());
            api.check(
                unsafe { (api.cupti_sass_metrics_get_properties)(&mut props as *mut _) },
                &format!("cuptiSassMetricsGetProperties({metric_name})"),
            )?;
            metric_configs.push(CUpti_SassMetrics_Config {
                metric_id: props.metric.metric_id,
                output_granularity: CUPTI_SASS_METRICS_OUTPUT_GRANULARITY_GPU,
            });
            metric_names_used.push(metric_name.clone());
        }

        let raw_ctx = prepare_cuda_kernel_runtime_for_probe(config.device_id)?;
        let ctx = raw_ctx as cuda::CUcontext;

        let mut set_config = metric_set_config_params(config.device_id, &mut metric_configs);
        api.check(
            unsafe { (api.cupti_sass_metrics_set_config)(&mut set_config as *mut _) },
            "cuptiSassMetricsSetConfig",
        )?;

        let capture_result = (|| -> Result<CaptureOutput, String> {
            let mut enable_guard =
                SassMetricsEnableGuard::enable(&api, ctx, config.enable_lazy_patching)?;

            let runtime_summary = run_surface(config)?;

            let mut data_props = metric_data_properties_params(ctx);
            api.check(
                unsafe { (api.cupti_sass_metrics_get_data_properties)(&mut data_props as *mut _) },
                "cuptiSassMetricsGetDataProperties",
            )?;

            let mut notes = vec![
                "The helper reused the exact-route CUDA runtime context before enabling CUPTI."
                    .to_string(),
                "Only the filtered kernel family is persisted in the output summary.".to_string(),
                format!(
                    "Collected {} patched instruction records with {} metric instance(s) per record.",
                    data_props.num_of_patched_instruction_records, data_props.num_of_instances
                ),
            ];

            if data_props.num_of_patched_instruction_records == 0
                || data_props.num_of_instances == 0
            {
                let dropped_records = enable_guard.disable()?;
                notes.push(
                    "CUPTI reported no patched instruction records for the capture window."
                        .to_string(),
                );
                notes.push(format!(
                    "Dropped records reported at disable: {dropped_records}."
                ));
                return Ok(CaptureOutput {
                    run_id: new_run_id("cupti-capture"),
                    gpu_name,
                    driver_version,
                    cuda_path: artifact_path_label(
                        Path::new(&api.cuda_path),
                        config.include_local_paths,
                    ),
                    cupti_dll_path: artifact_path_label(&api.dll_path, config.include_local_paths),
                    chip_name: catalog.chip_name.clone(),
                    target_surface: SurfaceSpec {
                        strategy: "double_ma".to_string(),
                        workload: workload_label(config.workload).to_string(),
                        fast_ma: "alma".to_string(),
                        slow_ma: "alma".to_string(),
                        metric_contract: metric_contract_label(config.metric_contract).to_string(),
                        shortlist_slice_mode: shortlist_slice_mode_label(
                            config.shortlist_slice_mode,
                        )
                        .to_string(),
                        shortlist_slice_benchmark: shortlist_slice_benchmark_label(
                            config.shortlist_slice_benchmark.as_ref(),
                        )
                        .to_string(),
                        candle_shape: candle_shape_label(config.candle_shape).to_string(),
                        device_id: config.device_id,
                    },
                    target_kernel_filter: config.kernel_filter.clone(),
                    metric_inventory_used: metric_names_used,
                    capture_status: "no_records".to_string(),
                    support_flags: support_map(&support),
                    runtime_summary,
                    observed_kernel_names: Vec::new(),
                    matched_kernel_names: Vec::new(),
                    matched_instruction_records: 0,
                    region_metrics: Vec::new(),
                    notes,
                });
            }

            let mut data_records = vec![
                CUpti_SassMetrics_Data {
                    struct_size: 0,
                    p_priv: std::ptr::null_mut(),
                    cubin_crc: 0,
                    function_index: 0,
                    function_name: std::ptr::null(),
                    pc_offset: 0,
                    p_instance_values: std::ptr::null_mut(),
                };
                data_props.num_of_patched_instruction_records
            ];
            let mut instance_storage = vec![
                vec![
                    CUpti_SassMetrics_InstanceValue {
                        metric_id: 0,
                        value: 0,
                    };
                    data_props.num_of_instances
                ];
                data_props.num_of_patched_instruction_records
            ];
            for (record, values) in data_records.iter_mut().zip(instance_storage.iter_mut()) {
                record.p_instance_values = values.as_mut_ptr();
            }

            let mut flush_params =
                metric_flush_params(ctx, &mut data_records, data_props.num_of_instances);
            api.check(
                unsafe { (api.cupti_sass_metrics_flush_data)(&mut flush_params as *mut _) },
                "cuptiSassMetricsFlushData",
            )?;

            let dropped_records = enable_guard.disable()?;
            notes.push(format!(
                "Dropped records reported at disable: {}.",
                dropped_records
            ));

            let metric_id_to_name = catalog
                .metrics
                .iter()
                .map(|metric| (metric.metric_id, metric.metric_name.clone()))
                .collect::<BTreeMap<_, _>>();
            let mut observed_kernel_names = BTreeSet::new();
            let mut grouped = BTreeMap::<(String, u32, u32), Vec<PcMetricRecord>>::new();
            let mut totals = BTreeMap::<(String, u32, u32), BTreeMap<String, u64>>::new();
            let mut matched_instruction_records = 0usize;

            for (record_index, record) in data_records.iter().enumerate() {
                let function_name = cstr_to_string(record.function_name);
                if function_name.is_empty() {
                    continue;
                }
                observed_kernel_names.insert(function_name.clone());
                if !function_name.contains(&config.kernel_filter) {
                    continue;
                }
                matched_instruction_records += 1;
                let mut values = BTreeMap::new();
                for instance in &instance_storage[record_index] {
                    let metric_name = metric_id_to_name
                        .get(&instance.metric_id)
                        .cloned()
                        .unwrap_or_else(|| format!("metric_{}", instance.metric_id));
                    *values.entry(metric_name.clone()).or_insert(0) += instance.value;
                    *totals
                        .entry((
                            function_name.clone(),
                            record.cubin_crc,
                            record.function_index,
                        ))
                        .or_default()
                        .entry(metric_name)
                        .or_insert(0) += instance.value;
                }
                grouped
                    .entry((function_name, record.cubin_crc, record.function_index))
                    .or_default()
                    .push(PcMetricRecord {
                        pc_offset: record.pc_offset,
                        values,
                    });
            }

            let region_metrics = grouped
                .into_iter()
                .map(
                    |((function_name, cubin_crc, function_index), mut pc_records)| {
                        pc_records.sort_by_key(|record| record.pc_offset);
                        KernelMetricSummary {
                            function_name: function_name.clone(),
                            cubin_crc,
                            function_index,
                            record_count: pc_records.len(),
                            metric_totals: totals
                                .remove(&(function_name, cubin_crc, function_index))
                                .unwrap_or_default(),
                            pc_records,
                        }
                    },
                )
                .collect::<Vec<_>>();

            Ok(CaptureOutput {
                run_id: new_run_id("cupti-capture"),
                gpu_name,
                driver_version,
                cuda_path: artifact_path_label(
                    Path::new(&api.cuda_path),
                    config.include_local_paths,
                ),
                cupti_dll_path: artifact_path_label(&api.dll_path, config.include_local_paths),
                chip_name: catalog.chip_name.clone(),
                target_surface: SurfaceSpec {
                    strategy: "double_ma".to_string(),
                    workload: workload_label(config.workload).to_string(),
                    fast_ma: "alma".to_string(),
                    slow_ma: "alma".to_string(),
                    metric_contract: metric_contract_label(config.metric_contract).to_string(),
                    shortlist_slice_mode: shortlist_slice_mode_label(config.shortlist_slice_mode)
                        .to_string(),
                    shortlist_slice_benchmark: shortlist_slice_benchmark_label(
                        config.shortlist_slice_benchmark.as_ref(),
                    )
                    .to_string(),
                    candle_shape: candle_shape_label(config.candle_shape).to_string(),
                    device_id: config.device_id,
                },
                target_kernel_filter: config.kernel_filter.clone(),
                metric_inventory_used: metric_names_used,
                capture_status: if region_metrics.is_empty() {
                    "no_match".to_string()
                } else {
                    "ok".to_string()
                },
                support_flags: support_map(&support),
                runtime_summary,
                observed_kernel_names: observed_kernel_names.into_iter().collect(),
                matched_kernel_names: region_metrics
                    .iter()
                    .map(|metric| metric.function_name.clone())
                    .collect(),
                matched_instruction_records,
                region_metrics,
                notes,
            })
        })();

        let mut unset = metric_unset_config_params(config.device_id);
        api.check(
            unsafe { (api.cupti_sass_metrics_unset_config)(&mut unset as *mut _) },
            "cuptiSassMetricsUnsetConfig",
        )?;

        let capture = capture_result?;
        let filename = format!(
            "{}_{}_{}_region_metrics.json",
            workload_label(config.workload),
            metric_contract_label(config.metric_contract),
            shortlist_slice_mode_label(config.shortlist_slice_mode)
        );
        let path = config.output_dir.join(filename);
        write_json_file(&path, &capture)?;
        Ok(path)
    }

    fn save_metadata(config: &ProbeConfig, artifact_path: &Path) -> Result<(), String> {
        let command = config
            .include_command_line
            .then(|| std::env::args().collect::<Vec<_>>().join(" "));
        let payload = json!({
            "command": command,
            "artifact_path": artifact_path_label(artifact_path, config.include_local_paths),
            "mode": match config.mode {
                CommandMode::Inventory => "inventory",
                CommandMode::Capture => "capture",
            },
        });
        let path = config.output_dir.join("last_command.json");
        write_json_file(&path, &payload)
    }

    fn artifact_path_label(path: &Path, include_local_paths: bool) -> String {
        if include_local_paths {
            return path.display().to_string();
        }
        let cwd = std::env::current_dir().ok();
        let absolute = path.canonicalize().unwrap_or_else(|_| {
            cwd.as_ref()
                .map_or_else(|| path.to_path_buf(), |base| base.join(path))
        });
        if let Some(cwd) = cwd.and_then(|value| value.canonicalize().ok()) {
            if let Ok(relative) = absolute.strip_prefix(&cwd) {
                return relative.to_string_lossy().replace('\\', "/");
            }
        }
        path.file_name()
            .and_then(|value| value.to_str())
            .map(|name| format!("<external>/{name}"))
            .unwrap_or_else(|| "<external>".to_string())
    }

    pub fn main() {
        let _benchmark_guard =
            crate::benchmark_lock::acquire("CUPTI SASS probe").unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
        let config = build_probe_config().unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });
        let artifact_path = match config.mode {
            CommandMode::Inventory => run_inventory(&config),
            CommandMode::Capture => capture_surface(&config),
        }
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });

        if let Err(e) = save_metadata(&config, &artifact_path) {
            eprintln!("{e}");
            std::process::exit(1);
        }

        println!("{}", artifact_path.display());
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn main() {
    real::main();
}
