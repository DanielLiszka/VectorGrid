use crate::{polygon, settings};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const SCHEMA_VERSION: u32 = 1;
const POLYGON_SETTINGS_FILE: &str = "polygon_settings.json";
const ALPACA_KEYRING_API_KEY_USERNAME: &str = "alpaca_execution_api_key";
const ALPACA_KEYRING_API_SECRET_USERNAME: &str = "alpaca_execution_api_secret";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReleaseVariant {
    Cpu,
    GpuMa,
    GpuKernel,
}

impl ReleaseVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::GpuMa => "gpu-ma",
            Self::GpuKernel => "gpu-kernel",
        }
    }

    pub fn requires_cuda(self) -> bool {
        !matches!(self, Self::Cpu)
    }

    pub fn requires_kernel(self) -> bool {
        matches!(self, Self::GpuKernel)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReleaseReadinessOptions {
    #[serde(default)]
    pub require_packaged_smoke: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseReadinessReport {
    pub schema_version: u32,
    pub variant: String,
    pub supported: bool,
    pub repo_root: String,
    pub app_identifier: String,
    pub app_local_data_dir: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    pub gpu: ReleaseGpuReadiness,
    pub resources: ReleaseResourceReadiness,
    pub tooling: ReleaseToolingReadiness,
    pub provider: ReleaseProviderReadiness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseGpuReadiness {
    pub built_with_cuda: bool,
    pub built_with_cuda_backtest_kernel: bool,
    pub cuda_available: bool,
    pub nvidia_smi_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_device_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_device_memory_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_device_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseResourceReadiness {
    pub src_tauri_config_exists: bool,
    pub root_tauri_config_exists: bool,
    pub src_tauri_windows_config_exists: bool,
    pub root_tauri_windows_config_exists: bool,
    pub market_data_root_dir: String,
    pub polygon_cache_dir: String,
    pub polygon_settings_fallback_file_exists: bool,
    pub catboost_bundle_resource_exists: bool,
    pub windows_bundle_catboost_resource_configured: bool,
    pub windows_bundle_nsis_target_configured: bool,
    pub windows_bundle_webview_install_mode: String,
    pub windows_bundle_installer_icon_exists: bool,
    pub double_crossover_prebuilt_ptx_exists: bool,
    pub options_expression_prebuilt_ptx_exists: bool,
    pub release_binary_exists: bool,
    pub release_bundle_dir_exists: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_binary_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_bundle_dir_path: Option<String>,
    #[serde(default)]
    pub release_bundle_artifact_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseToolingReadiness {
    pub cargo_available: bool,
    pub npm_available: bool,
    pub cargo_tauri_available: bool,
    pub tauri_driver_available: bool,
    pub edge_driver_available: bool,
    pub signtool_available: bool,
    pub makensis_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tauri_driver_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_driver_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signtool_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub makensis_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseProviderReadiness {
    pub alpaca: ReleaseAlpacaReadiness,
    pub polygon: ReleasePolygonReadiness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAlpacaReadiness {
    pub remember_available: bool,
    pub has_api_key: bool,
    pub has_api_secret: bool,
    pub has_credentials: bool,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasePolygonReadiness {
    pub has_api_key: bool,
    pub live_stream_market_scope: String,
    pub delayed_stream_default: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

pub fn collect_release_readiness(
    repo_root: &Path,
    app_identifier: &str,
    app_local_data_dir: &Path,
    variant: ReleaseVariant,
    options: &ReleaseReadinessOptions,
) -> Result<ReleaseReadinessReport, String> {
    let market_root = settings::resolve_market_data_root_dir(app_local_data_dir)?;
    let polygon_cache_dir = polygon::polygon_cache_dir(&market_root);
    let polygon_info = polygon::get_cache_info(app_identifier, &polygon_cache_dir)?;
    let gpu = gpu_readiness();
    let resources = resource_readiness(repo_root, &market_root, &polygon_cache_dir);
    let tooling = tooling_readiness(repo_root);
    let provider = provider_readiness(app_identifier, &polygon_info);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut notes = vec![
        "Release readiness is local-machine specific and does not replace runtime QA.".to_string(),
        "Live execution remains a supervised, operator-driven surface rather than a broad unattended production lane.".to_string(),
    ];

    if !resources.src_tauri_config_exists || !resources.root_tauri_config_exists {
        errors
            .push("Missing Tauri config file(s); release packaging cannot be trusted.".to_string());
    }
    if !resources.src_tauri_windows_config_exists || !resources.root_tauri_windows_config_exists {
        errors.push(
            "Missing Windows Tauri bundle-resource config mirror; packaged native resources cannot be trusted."
                .to_string(),
        );
    }
    if !resources.catboost_bundle_resource_exists {
        errors.push(
            "Missing CatBoost native bundle resource at src-tauri/resources/native/catboostmodel.dll."
                .to_string(),
        );
    }
    if !resources.windows_bundle_catboost_resource_configured {
        errors.push(
            "Windows Tauri bundle config must include CatBoost native DLL as native/catboostmodel.dll."
                .to_string(),
        );
    }
    if !resources.windows_bundle_nsis_target_configured {
        errors.push(
            "Windows Tauri bundle config must include the nsis target for setup .exe releases."
                .to_string(),
        );
    }
    if resources
        .windows_bundle_webview_install_mode
        .trim()
        .is_empty()
    {
        errors.push(
            "Windows Tauri bundle config must explicitly declare bundle.windows.webviewInstallMode."
                .to_string(),
        );
    }
    if !resources.windows_bundle_installer_icon_exists {
        errors.push(
            "Windows Tauri bundle config references no existing NSIS installer icon.".to_string(),
        );
    }
    if variant.requires_kernel() && !resources.double_crossover_prebuilt_ptx_exists {
        errors.push(
            "GPU-kernel variant requires crates/app_core/src/kernels/double_crossover.prebuilt.ptx."
                .to_string(),
        );
    }
    if variant.requires_kernel() && !resources.options_expression_prebuilt_ptx_exists {
        errors.push(
            "GPU-kernel variant requires crates/app_core/src/kernels/options_expression.prebuilt.ptx."
                .to_string(),
        );
    }

    if !tooling.cargo_available {
        errors.push("cargo is not available on PATH.".to_string());
    }
    if !tooling.npm_available {
        errors.push("npm is not available on PATH.".to_string());
    }
    if !tooling.cargo_tauri_available {
        errors.push("cargo tauri is unavailable; packaged release builds cannot run.".to_string());
    }
    if !tooling.signtool_available {
        warnings.push(
            "signtool.exe is not available on PATH; public Authenticode signing will need a configured custom signing hook or Windows SDK SignTool."
                .to_string(),
        );
    }
    if !tooling.makensis_available {
        notes.push(
            "makensis.exe is not available on PATH; the public release gate will still verify whether cargo tauri can produce the configured NSIS bundle."
                .to_string(),
        );
    }
    if options.require_packaged_smoke && !tooling.tauri_driver_available {
        errors.push("tauri-driver is unavailable; packaged smoke cannot run.".to_string());
    }
    if options.require_packaged_smoke && !tooling.edge_driver_available {
        errors.push(
            "msedgedriver is unavailable; packaged smoke cannot run on this machine.".to_string(),
        );
    }

    if variant.requires_cuda() && !gpu.built_with_cuda {
        errors.push(
            "Requested GPU release readiness using a binary that was not compiled with the cuda feature."
                .to_string(),
        );
    }
    if variant.requires_kernel() && !gpu.built_with_cuda_backtest_kernel {
        errors.push(
            "Requested GPU-kernel release readiness using a binary that was not compiled with the cuda-backtest-kernel feature."
                .to_string(),
        );
    }
    if variant.requires_cuda() && !gpu.cuda_available {
        errors.push("CUDA is not available to the current runtime.".to_string());
    }
    if variant.requires_cuda() && gpu.auto_device_name.is_none() {
        errors.push(
            gpu.auto_device_error
                .clone()
                .unwrap_or_else(|| "No preferred CUDA device was resolved.".to_string()),
        );
    } else if !variant.requires_cuda() && gpu.auto_device_error.is_some() {
        warnings.push(format!(
            "GPU diagnostics reported an auto-device issue that does not block CPU releases: {}",
            gpu.auto_device_error.as_deref().unwrap_or("unknown error")
        ));
    }

    if !provider.alpaca.remember_available {
        warnings.push(
            "Secure Alpaca credential storage is unavailable on this machine; live broker workflows will require explicit overrides."
                .to_string(),
        );
    }
    if !provider.polygon.has_api_key {
        warnings.push(
            "Polygon API key is not currently configured; Polygon download and live-stream workflows will remain unavailable until one is saved."
                .to_string(),
        );
    }
    if resources.polygon_settings_fallback_file_exists {
        warnings.push(
            "Polygon secure storage fallback file is present; review local secret-handling posture before public release work."
                .to_string(),
        );
    }
    if !resources.release_bundle_dir_exists {
        notes.push(
            "No release bundle directory exists yet; bundle-level packaged smoke will stay in raw release-binary mode until cargo tauri build emits target/release/bundle."
                .to_string(),
        );
    }
    if !resources.release_binary_exists {
        notes.push(
            "No release executable exists yet; packaged smoke will need a completed release build first."
                .to_string(),
        );
    }
    notes.push(
        "Polygon live candle streaming currently supports market=stocks only, and delayed streaming is the default."
            .to_string(),
    );

    Ok(ReleaseReadinessReport {
        schema_version: SCHEMA_VERSION,
        variant: variant.as_str().to_string(),
        supported: errors.is_empty(),
        repo_root: repo_root.to_string_lossy().to_string(),
        app_identifier: app_identifier.to_string(),
        app_local_data_dir: app_local_data_dir.to_string_lossy().to_string(),
        errors,
        warnings,
        notes,
        gpu,
        resources,
        tooling,
        provider,
    })
}

fn gpu_readiness() -> ReleaseGpuReadiness {
    let (auto_device_name, auto_device_memory_bytes, auto_device_error) =
        match ta_app_core::preferred_cuda_device() {
            Ok(Some(device)) => (Some(device.name), Some(device.total_memory_bytes), None),
            Ok(None) => (None, None, None),
            Err(error) => (None, None, Some(error)),
        };

    ReleaseGpuReadiness {
        built_with_cuda: cfg!(feature = "cuda"),
        built_with_cuda_backtest_kernel: cfg!(feature = "cuda-backtest-kernel"),
        cuda_available: cuda_available_now(),
        nvidia_smi_available: find_command_path("nvidia-smi", None).is_some(),
        auto_device_name,
        auto_device_memory_bytes,
        auto_device_error,
    }
}

#[cfg(feature = "cuda")]
fn cuda_available_now() -> bool {
    vector_ta::cuda::cuda_available()
}

#[cfg(not(feature = "cuda"))]
fn cuda_available_now() -> bool {
    false
}

fn resource_readiness(
    repo_root: &Path,
    market_root: &Path,
    polygon_cache_dir: &Path,
) -> ReleaseResourceReadiness {
    let src_tauri = repo_root.join("src-tauri");
    let windows_bundle = windows_bundle_readiness(&src_tauri);
    let release_paths = resolve_release_build_paths(repo_root);
    ReleaseResourceReadiness {
        src_tauri_config_exists: src_tauri.join("tauri.conf.json").is_file(),
        root_tauri_config_exists: repo_root.join("tauri.conf.json").is_file(),
        src_tauri_windows_config_exists: src_tauri.join("tauri.windows.conf.json").is_file(),
        root_tauri_windows_config_exists: repo_root.join("tauri.windows.conf.json").is_file(),
        market_data_root_dir: market_root.to_string_lossy().to_string(),
        polygon_cache_dir: polygon_cache_dir.to_string_lossy().to_string(),
        polygon_settings_fallback_file_exists: polygon_cache_dir
            .join(POLYGON_SETTINGS_FILE)
            .is_file(),
        catboost_bundle_resource_exists: src_tauri
            .join("resources")
            .join("native")
            .join("catboostmodel.dll")
            .is_file(),
        windows_bundle_catboost_resource_configured: windows_bundle.catboost_resource_configured,
        windows_bundle_nsis_target_configured: windows_bundle.nsis_target_configured,
        windows_bundle_webview_install_mode: windows_bundle.webview_install_mode,
        windows_bundle_installer_icon_exists: windows_bundle.installer_icon_exists,
        double_crossover_prebuilt_ptx_exists: repo_root
            .join("crates")
            .join("app_core")
            .join("src")
            .join("kernels")
            .join("double_crossover.prebuilt.ptx")
            .is_file(),
        options_expression_prebuilt_ptx_exists: repo_root
            .join("crates")
            .join("app_core")
            .join("src")
            .join("kernels")
            .join("options_expression.prebuilt.ptx")
            .is_file(),
        release_binary_exists: release_paths.release_binary_path.is_some(),
        release_bundle_dir_exists: release_paths.release_bundle_dir_path.is_some(),
        release_binary_path: release_paths
            .release_binary_path
            .map(|path| path.to_string_lossy().to_string()),
        release_bundle_dir_path: release_paths
            .release_bundle_dir_path
            .map(|path| path.to_string_lossy().to_string()),
        release_bundle_artifact_count: release_paths.release_bundle_artifact_count,
    }
}

#[derive(Debug, Clone, Default)]
struct WindowsBundleReadiness {
    catboost_resource_configured: bool,
    nsis_target_configured: bool,
    webview_install_mode: String,
    installer_icon_exists: bool,
}

fn windows_bundle_readiness(src_tauri: &Path) -> WindowsBundleReadiness {
    let config_path = src_tauri.join("tauri.windows.conf.json");
    let Ok(raw) = std::fs::read_to_string(&config_path) else {
        return WindowsBundleReadiness::default();
    };
    let Ok(config) = serde_json::from_str::<Value>(&raw) else {
        return WindowsBundleReadiness::default();
    };
    let Some(bundle) = config.get("bundle") else {
        return WindowsBundleReadiness::default();
    };
    let resources = bundle.get("resources");
    let catboost_path = src_tauri
        .join("resources")
        .join("native")
        .join("catboostmodel.dll");
    let catboost_resource_configured = resources
        .is_some_and(|value| bundle_resources_include_catboost(src_tauri, value, &catboost_path));
    let nsis_target_configured = bundle.get("targets").is_some_and(|targets| {
        targets
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("nsis")))
            || targets.as_str() == Some("nsis")
    });
    let webview_install_mode = bundle
        .pointer("/windows/webviewInstallMode/type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let installer_icon_exists = bundle
        .pointer("/windows/nsis/installerIcon")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| src_tauri.join(value).is_file())
        .unwrap_or(false);

    WindowsBundleReadiness {
        catboost_resource_configured,
        nsis_target_configured,
        webview_install_mode,
        installer_icon_exists,
    }
}

fn bundle_resources_include_catboost(
    src_tauri: &Path,
    resources: &Value,
    catboost_path: &Path,
) -> bool {
    if let Some(map) = resources.as_object() {
        return map.iter().any(|(source, target)| {
            resolve_config_relative(src_tauri, source) == catboost_path
                && target.as_str().is_some_and(|value| {
                    normalize_resource_path(value) == "native/catboostmodel.dll"
                })
        });
    }
    if let Some(items) = resources.as_array() {
        return items.iter().any(|item| {
            item.as_str()
                .is_some_and(|source| resolve_config_relative(src_tauri, source) == catboost_path)
        });
    }
    false
}

fn resolve_config_relative(config_dir: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        config_dir.join(path)
    }
}

fn normalize_resource_path(value: &str) -> String {
    value.trim().replace('\\', "/")
}

#[derive(Debug, Clone, Default)]
struct ReleaseBuildPaths {
    release_binary_path: Option<PathBuf>,
    release_bundle_dir_path: Option<PathBuf>,
    release_bundle_artifact_count: usize,
}

fn resolve_release_build_paths(repo_root: &Path) -> ReleaseBuildPaths {
    let release_roots = [
        repo_root.join("src-tauri").join("target").join("release"),
        repo_root.join("target").join("release"),
    ];
    let release_binary_path = release_roots
        .iter()
        .map(|root| root.join("ta_desktop_demo_app.exe"))
        .find(|path| path.is_file());
    let release_bundle_dir_path = release_roots
        .iter()
        .map(|root| root.join("bundle"))
        .find(|path| path.is_dir());
    let release_bundle_artifact_count = release_bundle_dir_path
        .as_ref()
        .map(|bundle_root| count_release_bundle_artifacts(bundle_root))
        .unwrap_or(0);

    ReleaseBuildPaths {
        release_binary_path,
        release_bundle_dir_path,
        release_bundle_artifact_count,
    }
}

fn count_release_bundle_artifacts(bundle_root: &Path) -> usize {
    std::fs::read_dir(bundle_root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|subdir| {
            std::fs::read_dir(subdir)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .count()
        })
        .sum()
}

fn tooling_readiness(repo_root: &Path) -> ReleaseToolingReadiness {
    let cargo_path = find_command_path("cargo", None);
    let npm_path = find_command_path("npm", None);
    let tauri_driver_path = find_command_path("tauri-driver", None);
    let signtool_path = find_command_path("signtool", None);
    let makensis_path = find_command_path("makensis", None);
    let edge_driver_path = find_command_path(
        "msedgedriver",
        Some(&[
            repo_root.join("msedgedriver.exe"),
            repo_root.join("web").join("msedgedriver.exe"),
        ]),
    );

    ReleaseToolingReadiness {
        cargo_available: cargo_path.is_some(),
        npm_available: npm_path.is_some(),
        cargo_tauri_available: cargo_tauri_available(),
        tauri_driver_available: tauri_driver_path.is_some(),
        edge_driver_available: edge_driver_path.is_some(),
        signtool_available: signtool_path.is_some(),
        makensis_available: makensis_path.is_some(),
        cargo_path: cargo_path.map(|path| path.to_string_lossy().to_string()),
        npm_path: npm_path.map(|path| path.to_string_lossy().to_string()),
        tauri_driver_path: tauri_driver_path.map(|path| path.to_string_lossy().to_string()),
        edge_driver_path: edge_driver_path.map(|path| path.to_string_lossy().to_string()),
        signtool_path: signtool_path.map(|path| path.to_string_lossy().to_string()),
        makensis_path: makensis_path.map(|path| path.to_string_lossy().to_string()),
    }
}

fn provider_readiness(
    app_identifier: &str,
    polygon_info: &polygon::PolygonCacheInfo,
) -> ReleaseProviderReadiness {
    ReleaseProviderReadiness {
        alpaca: alpaca_readiness(app_identifier),
        polygon: ReleasePolygonReadiness {
            has_api_key: polygon_info.has_api_key,
            live_stream_market_scope: "stocks-only".to_string(),
            delayed_stream_default: true,
            notes: vec![
                "Polygon live streaming is currently scoped to stocks.".to_string(),
                "Delayed Polygon live streaming is the default.".to_string(),
            ],
        },
    }
}

fn alpaca_readiness(app_identifier: &str) -> ReleaseAlpacaReadiness {
    let Ok((key_entry, secret_entry)) = alpaca_keyring_entries(app_identifier) else {
        return ReleaseAlpacaReadiness {
            remember_available: false,
            has_api_key: false,
            has_api_secret: false,
            has_credentials: false,
            state: "unavailable".to_string(),
            storage_backend: None,
            message: Some("Secure credential storage is unavailable on this system.".to_string()),
        };
    };
    let has_api_key = read_keyring_trimmed(&key_entry).is_some();
    let has_api_secret = read_keyring_trimmed(&secret_entry).is_some();
    let state = if has_api_key && has_api_secret {
        "stored"
    } else if has_api_key || has_api_secret {
        "incomplete"
    } else {
        "missing"
    };
    ReleaseAlpacaReadiness {
        remember_available: true,
        has_api_key,
        has_api_secret,
        has_credentials: has_api_key && has_api_secret,
        state: state.to_string(),
        storage_backend: Some("keyring".to_string()),
        message: None,
    }
}

fn alpaca_keyring_service_name(app_identifier: &str) -> String {
    let id = app_identifier.trim();
    if id.is_empty() {
        "vectorgrid.alpaca.execution".to_string()
    } else {
        format!("{id}.alpaca.execution")
    }
}

fn alpaca_keyring_entries(
    app_identifier: &str,
) -> Result<(keyring::Entry, keyring::Entry), String> {
    let service = alpaca_keyring_service_name(app_identifier);
    let key_entry = keyring::Entry::new(&service, ALPACA_KEYRING_API_KEY_USERNAME)
        .map_err(|e| format!("failed to initialize Alpaca API key secure storage: {e}"))?;
    let secret_entry = keyring::Entry::new(&service, ALPACA_KEYRING_API_SECRET_USERNAME)
        .map_err(|e| format!("failed to initialize Alpaca API secret secure storage: {e}"))?;
    Ok((key_entry, secret_entry))
}

fn read_keyring_trimmed(entry: &keyring::Entry) -> Option<String> {
    entry.get_password().ok().and_then(|raw| {
        let trimmed = raw.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn cargo_tauri_available() -> bool {
    Command::new("cargo")
        .args(["tauri", "--version"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn find_command_path(command: &str, fallback_paths: Option<&[PathBuf]>) -> Option<PathBuf> {
    if let Some(paths) = fallback_paths {
        for path in paths {
            if path.is_file() {
                return Some(path.clone());
            }
        }
    }

    let path_var = env::var_os("PATH")?;
    let mut exts = vec![String::new()];
    if cfg!(windows) {
        let pathext = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        exts = pathext
            .split(';')
            .map(|ext| ext.trim().to_ascii_lowercase())
            .filter(|ext| !ext.is_empty())
            .collect();
        exts.insert(0, String::new());
    }

    for dir in env::split_paths(&path_var) {
        for ext in &exts {
            let candidate = if ext.is_empty() || command.contains('.') {
                dir.join(command)
            } else {
                dir.join(format!("{command}{ext}"))
            };
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        collect_release_readiness, resolve_release_build_paths, ReleaseReadinessOptions,
        ReleaseVariant,
    };
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root parent")
            .to_path_buf()
    }

    #[test]
    fn release_variant_strings_match_expected_contract() {
        assert_eq!(ReleaseVariant::Cpu.as_str(), "cpu");
        assert_eq!(ReleaseVariant::GpuMa.as_str(), "gpu-ma");
        assert_eq!(ReleaseVariant::GpuKernel.as_str(), "gpu-kernel");
    }

    #[test]
    fn release_readiness_detects_repo_local_resources() {
        let temp_dir = std::env::temp_dir().join("vectorgrid_release_readiness_test");
        std::fs::create_dir_all(&temp_dir).expect("temp app data dir");
        let report = collect_release_readiness(
            &repo_root(),
            "dev.vectoralpha.vectorgrid.test.release",
            &temp_dir,
            ReleaseVariant::Cpu,
            &ReleaseReadinessOptions::default(),
        )
        .expect("collect readiness");

        assert!(report.resources.src_tauri_config_exists);
        assert!(report.resources.root_tauri_config_exists);
        assert!(report.resources.catboost_bundle_resource_exists);
        assert!(report.resources.windows_bundle_catboost_resource_configured);
        assert!(report.resources.windows_bundle_nsis_target_configured);
        assert_eq!(
            report.resources.windows_bundle_webview_install_mode,
            "downloadBootstrapper"
        );
        assert!(report.resources.windows_bundle_installer_icon_exists);
        assert!(report.resources.double_crossover_prebuilt_ptx_exists);
        assert_eq!(
            report.resources.release_binary_exists,
            report.resources.release_binary_path.is_some()
        );
        assert_eq!(
            report.resources.release_bundle_dir_exists,
            report.resources.release_bundle_dir_path.is_some()
        );
    }

    #[test]
    fn resolve_release_build_paths_accepts_workspace_target_layout() {
        let temp_root = std::env::temp_dir()
            .join("vectorgrid_release_readiness_workspace_target")
            .join(format!("{}", std::process::id()));
        let src_tauri_release = temp_root.join("src-tauri").join("target").join("release");
        let workspace_release = temp_root.join("target").join("release");
        std::fs::create_dir_all(&src_tauri_release).expect("src-tauri target release");
        std::fs::create_dir_all(workspace_release.join("bundle").join("nsis"))
            .expect("workspace bundle dir");
        std::fs::write(
            workspace_release.join("ta_desktop_demo_app.exe"),
            b"vectorgrid-binary",
        )
        .expect("workspace release binary");
        std::fs::write(
            workspace_release
                .join("bundle")
                .join("nsis")
                .join("VectorGrid.exe"),
            b"bundle-artifact",
        )
        .expect("bundle artifact");

        let resolved = resolve_release_build_paths(&temp_root);
        assert_eq!(
            resolved.release_binary_path,
            Some(workspace_release.join("ta_desktop_demo_app.exe"))
        );
        assert_eq!(
            resolved.release_bundle_dir_path,
            Some(workspace_release.join("bundle"))
        );
        assert_eq!(resolved.release_bundle_artifact_count, 1);

        let _ = std::fs::remove_dir_all(temp_root);
    }
}
