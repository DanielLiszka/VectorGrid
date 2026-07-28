use crate::{polygon, settings};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

const SCHEMA_VERSION: u32 = 1;
const POLYGON_SETTINGS_FILE: &str = "polygon_settings.json";
const ALPACA_KEYRING_API_KEY_USERNAME: &str = "alpaca_execution_api_key";
const ALPACA_KEYRING_API_SECRET_USERNAME: &str = "alpaca_execution_api_secret";
const GPU_KERNEL_SHIPPING_PTX_ISA_VERSION: &str = "9.2";
const GPU_KERNEL_SHIPPING_PTX_TARGET: &str = "sm_89";
const GPU_KERNEL_MINIMUM_COMPUTE_CAPABILITY_MAJOR: u32 = 8;
const GPU_KERNEL_MINIMUM_COMPUTE_CAPABILITY_MINOR: u32 = 9;
const CATBOOST_REVIEWED_VERSION: &str = "1.2.10";
const CATBOOST_REVIEWED_TAG_COMMIT: &str = "b1bd2a6d77219e82a1acfcedfccb8e6f6c1ee084";
const CATBOOST_REVIEWED_SIZE_BYTES: u64 = 16_438_272;
const CATBOOST_REVIEWED_SHA256: &str =
    "668d8867940a0832accaad6004e249f11ecc48eb63eb1d65551cfdd37c1939ad";
const CATBOOST_REVIEWED_LICENSE_SHA256: &str =
    "a2574dd20dd0af8e0bd25b34b38c1bb11912ff319ee4f41436395df0b1d19309";

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
    #[serde(default)]
    pub require_public_signing: bool,
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
    pub updater: ReleaseUpdaterReadiness,
    pub provider: ReleaseProviderReadiness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseGpuKernelPtxContract {
    pub shipping_ptx_isa_version: String,
    pub shipping_ptx_target: String,
    pub minimum_compute_capability_major: u32,
    pub minimum_compute_capability_minor: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_device_meets_minimum: Option<bool>,
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
    pub auto_device_compute_capability_major: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_device_compute_capability_minor: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_device_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_kernel_ptx_contract: Option<ReleaseGpuKernelPtxContract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double_crossover_module_jit_loaded: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double_crossover_module_jit_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_expression_module_jit_loaded: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_expression_module_jit_error: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catboost_bundle_resource_sha256: Option<String>,
    pub catboost_bundle_resource_matches_reviewed_release: bool,
    pub catboost_provenance_baseline_exists: bool,
    pub catboost_provenance_baseline_matches_reviewed_release: bool,
    pub catboost_license_resource_exists: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catboost_license_resource_sha256: Option<String>,
    pub catboost_license_resource_matches_reviewed_release: bool,
    pub windows_bundle_catboost_resource_configured: bool,
    pub windows_bundle_catboost_license_configured: bool,
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
    #[serde(default)]
    pub edge_driver_override_configured: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_driver_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_driver_major: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webview2_runtime_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webview2_runtime_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webview2_runtime_major: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_browser_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_browser_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_browser_major: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_compatibility_target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_driver_major_compatible: Option<bool>,
    #[serde(default)]
    pub archive_tool_available: bool,
    pub signtool_available: bool,
    pub custom_signing_hook_configured: bool,
    pub windows_certificate_thumbprint_configured: bool,
    pub windows_timestamp_url_configured: bool,
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
    pub archive_tool_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_tool_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signtool_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub makensis_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseUpdaterReadiness {
    pub configured: bool,
    pub active: bool,
    pub endpoint_count: usize,
    pub pubkey_configured: bool,
    pub intentionally_disabled: bool,
    pub status: String,
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
    let updater = updater_readiness(repo_root);
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
    if !resources.catboost_bundle_resource_matches_reviewed_release {
        errors.push(format!(
            "CatBoost native bundle resource must exactly match the reviewed upstream v{CATBOOST_REVIEWED_VERSION} release asset."
        ));
    }
    if !resources.catboost_provenance_baseline_exists
        || !resources.catboost_provenance_baseline_matches_reviewed_release
    {
        errors.push(
            "CatBoost provenance baseline is missing or no longer matches the release-readiness contract."
                .to_string(),
        );
    }
    if !resources.catboost_license_resource_exists
        || !resources.catboost_license_resource_matches_reviewed_release
    {
        errors.push(
            "CatBoost Apache-2.0 license notice is missing or does not match the reviewed upstream tag."
                .to_string(),
        );
    }
    if !resources.windows_bundle_catboost_resource_configured {
        errors.push(
            "Windows Tauri bundle config must include CatBoost native DLL as native/catboostmodel.dll."
                .to_string(),
        );
    }
    if !resources.windows_bundle_catboost_license_configured {
        errors.push(
            "Windows Tauri bundle config must include the CatBoost license as licenses/CatBoost-LICENSE.txt."
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
    let public_signing_ready = tooling.custom_signing_hook_configured
        || (tooling.signtool_available && tooling.windows_certificate_thumbprint_configured);
    if !tooling.signtool_available && !tooling.custom_signing_hook_configured {
        warnings.push(
            "signtool.exe is not available on PATH and no Windows signCommand is configured; public Authenticode signing needs one of those lanes."
                .to_string(),
        );
    }
    if !public_signing_ready {
        warnings.push(
            "Public Authenticode signing is not fully configured; set bundle.windows.signCommand or a certificate thumbprint with SignTool available before publishing installers."
                .to_string(),
        );
    }
    if public_signing_ready && !tooling.windows_timestamp_url_configured {
        warnings.push(
            "Public Authenticode signing has no configured timestamp URL; signed installers may become unverifiable after certificate expiry."
                .to_string(),
        );
    }
    if options.require_public_signing && !public_signing_ready {
        errors.push(
            "Public signing is required, but no complete Authenticode signing lane is configured."
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
    if options.require_packaged_smoke && !tooling.archive_tool_available {
        errors.push(
            "7z/7za is unavailable; exact packaged smoke cannot extract and inspect the staged NSIS payload."
                .to_string(),
        );
    }
    if options.require_packaged_smoke && !tooling.edge_driver_available {
        errors.push(if tooling.edge_driver_override_configured {
            "VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER does not resolve to an existing msedgedriver executable; packaged smoke will not fall back silently."
                .to_string()
        } else {
            "msedgedriver is unavailable; packaged smoke cannot run on this machine.".to_string()
        });
    } else if options.require_packaged_smoke && tooling.edge_driver_version.is_none() {
        errors.push(
            "The resolved msedgedriver version could not be parsed; packaged smoke cannot prove driver/runtime compatibility."
                .to_string(),
        );
    } else if options.require_packaged_smoke && tooling.edge_compatibility_target.is_none() {
        errors.push(
            "No installed WebView2 runtime or Microsoft Edge version could be resolved; packaged smoke cannot prove EdgeDriver compatibility."
                .to_string(),
        );
    } else if options.require_packaged_smoke && tooling.edge_driver_major_compatible != Some(true) {
        errors.push(format!(
            "EdgeDriver major-version compatibility failed: driver {} versus {} {}. Set VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER to a matching driver.",
            tooling.edge_driver_version.as_deref().unwrap_or("unknown"),
            tooling
                .edge_compatibility_target
                .as_deref()
                .unwrap_or("runtime"),
            selected_edge_runtime_version(&tooling).unwrap_or("unknown"),
        ));
    } else if tooling.edge_driver_major_compatible == Some(false) {
        warnings.push(format!(
            "The available EdgeDriver {} does not match the installed {} {} major version; packaged desktop smoke will fail until a compatible driver is selected.",
            tooling.edge_driver_version.as_deref().unwrap_or("unknown"),
            tooling
                .edge_compatibility_target
                .as_deref()
                .unwrap_or("runtime"),
            selected_edge_runtime_version(&tooling).unwrap_or("unknown"),
        ));
    }

    if !updater.configured {
        notes.push(
            "Updater config is absent; public release update delivery is treated as disabled."
                .to_string(),
        );
    } else if updater.intentionally_disabled {
        notes.push("Updater is explicitly disabled in Tauri config; public release update delivery is out of scope for this build.".to_string());
    } else {
        if updater.endpoint_count == 0 {
            errors.push("Updater is active but has no configured endpoints.".to_string());
        }
        if !updater.pubkey_configured {
            errors.push("Updater is active but has no public signing key configured.".to_string());
        }
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
    if variant.requires_kernel() {
        errors.extend(gpu_kernel_readiness_errors(&gpu));
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
        updater,
        provider,
    })
}

fn gpu_readiness() -> ReleaseGpuReadiness {
    let (
        auto_device_id,
        auto_device_name,
        auto_device_memory_bytes,
        auto_device_compute_capability_major,
        auto_device_compute_capability_minor,
        auto_device_error,
    ) = match ta_app_core::preferred_cuda_device() {
        Ok(Some(device)) => (
            Some(device.device_id),
            Some(device.name),
            Some(device.total_memory_bytes),
            Some(device.compute_capability_major),
            Some(device.compute_capability_minor),
            None,
        ),
        Ok(None) => (None, None, None, None, None, None),
        Err(error) => (None, None, None, None, None, Some(error)),
    };
    let auto_device_compute_capability =
        auto_device_compute_capability_major.zip(auto_device_compute_capability_minor);
    let gpu_kernel_ptx_contract = gpu_kernel_ptx_contract_for_build(auto_device_compute_capability);
    let (
        double_crossover_module_jit_loaded,
        double_crossover_module_jit_error,
        options_expression_module_jit_loaded,
        options_expression_module_jit_error,
    ) = gpu_kernel_runtime_prewarm_readiness(
        auto_device_id,
        gpu_kernel_ptx_contract
            .as_ref()
            .and_then(|contract| contract.preferred_device_meets_minimum),
    );

    ReleaseGpuReadiness {
        built_with_cuda: cfg!(feature = "cuda"),
        built_with_cuda_backtest_kernel: cfg!(feature = "cuda-backtest-kernel"),
        cuda_available: cuda_available_now(),
        nvidia_smi_available: find_command_path("nvidia-smi", None).is_some(),
        auto_device_name,
        auto_device_memory_bytes,
        auto_device_compute_capability_major,
        auto_device_compute_capability_minor,
        auto_device_error,
        gpu_kernel_ptx_contract,
        double_crossover_module_jit_loaded,
        double_crossover_module_jit_error,
        options_expression_module_jit_loaded,
        options_expression_module_jit_error,
    }
}

fn compute_capability_meets_gpu_kernel_contract(major: u32, minor: u32) -> bool {
    (major, minor)
        >= (
            GPU_KERNEL_MINIMUM_COMPUTE_CAPABILITY_MAJOR,
            GPU_KERNEL_MINIMUM_COMPUTE_CAPABILITY_MINOR,
        )
}

#[cfg(any(test, all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn shipping_gpu_kernel_ptx_contract(
    preferred_device_compute_capability: Option<(u32, u32)>,
) -> ReleaseGpuKernelPtxContract {
    ReleaseGpuKernelPtxContract {
        shipping_ptx_isa_version: GPU_KERNEL_SHIPPING_PTX_ISA_VERSION.to_string(),
        shipping_ptx_target: GPU_KERNEL_SHIPPING_PTX_TARGET.to_string(),
        minimum_compute_capability_major: GPU_KERNEL_MINIMUM_COMPUTE_CAPABILITY_MAJOR,
        minimum_compute_capability_minor: GPU_KERNEL_MINIMUM_COMPUTE_CAPABILITY_MINOR,
        preferred_device_meets_minimum: preferred_device_compute_capability
            .map(|(major, minor)| compute_capability_meets_gpu_kernel_contract(major, minor)),
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn gpu_kernel_ptx_contract_for_build(
    preferred_device_compute_capability: Option<(u32, u32)>,
) -> Option<ReleaseGpuKernelPtxContract> {
    Some(shipping_gpu_kernel_ptx_contract(
        preferred_device_compute_capability,
    ))
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn gpu_kernel_ptx_contract_for_build(
    _preferred_device_compute_capability: Option<(u32, u32)>,
) -> Option<ReleaseGpuKernelPtxContract> {
    None
}

fn gpu_kernel_readiness_errors(gpu: &ReleaseGpuReadiness) -> Vec<String> {
    let mut errors = Vec::new();
    if !gpu.built_with_cuda_backtest_kernel {
        return errors;
    }

    if gpu.gpu_kernel_ptx_contract.is_none() {
        errors.push(
            "GPU-kernel shipping PTX contract evidence was not recorded by this build.".to_string(),
        );
        return errors;
    }

    let device_compute_capability = gpu
        .auto_device_compute_capability_major
        .zip(gpu.auto_device_compute_capability_minor);
    let Some((major, minor)) = device_compute_capability else {
        if gpu.auto_device_name.is_some() {
            errors.push(format!(
                "Preferred CUDA device compute capability could not be resolved; the gpu-kernel \
                 release requires at least {GPU_KERNEL_SHIPPING_PTX_TARGET} for shipping PTX ISA \
                 {GPU_KERNEL_SHIPPING_PTX_ISA_VERSION}."
            ));
        }
        return errors;
    };

    if !compute_capability_meets_gpu_kernel_contract(major, minor) {
        errors.push(format!(
            "Preferred CUDA device compute capability sm_{major}{minor} is below the gpu-kernel \
             shipping PTX target {GPU_KERNEL_SHIPPING_PTX_TARGET} (PTX ISA \
             {GPU_KERNEL_SHIPPING_PTX_ISA_VERSION})."
        ));
        return errors;
    }

    if gpu.double_crossover_module_jit_loaded != Some(true) {
        errors.push(format!(
            "Embedded double-crossover GPU runtime prewarm failed (module load, required-symbol \
             resolution, context, or stream initialization): {}",
            gpu.double_crossover_module_jit_error
                .as_deref()
                .unwrap_or("probe did not run")
        ));
    }
    if gpu.options_expression_module_jit_loaded != Some(true) {
        errors.push(format!(
            "Embedded options-expression GPU runtime prewarm failed (module load, required-symbol \
             resolution, context, or stream initialization): {}",
            gpu.options_expression_module_jit_error
                .as_deref()
                .unwrap_or("probe did not run")
        ));
    }
    errors
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn gpu_kernel_runtime_prewarm_readiness(
    device_id: Option<u32>,
    device_meets_minimum: Option<bool>,
) -> (Option<bool>, Option<String>, Option<bool>, Option<String>) {
    let Some(device_id) = device_id else {
        return (
            Some(false),
            Some("no preferred CUDA device was resolved".to_string()),
            Some(false),
            Some("no preferred CUDA device was resolved".to_string()),
        );
    };
    if device_meets_minimum != Some(true) {
        let reason = format!(
            "runtime prewarm skipped because the preferred CUDA device does not satisfy the \
             shipping PTX target {GPU_KERNEL_SHIPPING_PTX_TARGET}"
        );
        return (Some(false), Some(reason.clone()), Some(false), Some(reason));
    }

    let double_crossover = ta_app_core::prepare_cuda_kernel_runtime_for_probe(device_id);
    let options_expression = ta_app_core::prewarm_options_expression_gpu_runtime(device_id);
    (
        Some(double_crossover.is_ok()),
        double_crossover.err(),
        Some(options_expression.is_ok()),
        options_expression.err(),
    )
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn gpu_kernel_runtime_prewarm_readiness(
    _device_id: Option<u32>,
    _device_meets_minimum: Option<bool>,
) -> (Option<bool>, Option<String>, Option<bool>, Option<String>) {
    (None, None, None, None)
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
    let catboost_path = src_tauri
        .join("resources")
        .join("native")
        .join("catboostmodel.dll");
    let catboost_sha256 = sha256_file_hex(&catboost_path);
    let catboost_size_bytes = std::fs::metadata(&catboost_path)
        .ok()
        .map(|metadata| metadata.len());
    let catboost_matches_reviewed_release = catboost_sha256
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(CATBOOST_REVIEWED_SHA256))
        && catboost_size_bytes == Some(CATBOOST_REVIEWED_SIZE_BYTES);
    let catboost_license_path = src_tauri
        .join("resources")
        .join("licenses")
        .join("CatBoost-LICENSE.txt");
    let catboost_license_sha256 = sha256_file_hex(&catboost_license_path);
    let catboost_license_matches_reviewed_release = catboost_license_sha256
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(CATBOOST_REVIEWED_LICENSE_SHA256));
    let catboost_provenance_path = repo_root
        .join("scripts")
        .join("baselines")
        .join("catboost-native-v1.2.10.json");
    let catboost_provenance = std::fs::read_to_string(&catboost_provenance_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    let catboost_provenance_matches_reviewed_release =
        catboost_provenance.as_ref().is_some_and(|baseline| {
            baseline.get("version").and_then(Value::as_str) == Some(CATBOOST_REVIEWED_VERSION)
                && baseline.get("tag_commit").and_then(Value::as_str)
                    == Some(CATBOOST_REVIEWED_TAG_COMMIT)
                && baseline.get("asset_size_bytes").and_then(Value::as_u64)
                    == Some(CATBOOST_REVIEWED_SIZE_BYTES)
                && baseline
                    .get("asset_sha256")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(CATBOOST_REVIEWED_SHA256))
                && baseline
                    .get("license_sha256")
                    .and_then(Value::as_str)
                    .is_some_and(|value| {
                        value.eq_ignore_ascii_case(CATBOOST_REVIEWED_LICENSE_SHA256)
                    })
        });
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
        catboost_bundle_resource_exists: catboost_path.is_file(),
        catboost_bundle_resource_sha256: catboost_sha256,
        catboost_bundle_resource_matches_reviewed_release: catboost_matches_reviewed_release,
        catboost_provenance_baseline_exists: catboost_provenance_path.is_file(),
        catboost_provenance_baseline_matches_reviewed_release:
            catboost_provenance_matches_reviewed_release,
        catboost_license_resource_exists: catboost_license_path.is_file(),
        catboost_license_resource_sha256: catboost_license_sha256,
        catboost_license_resource_matches_reviewed_release:
            catboost_license_matches_reviewed_release,
        windows_bundle_catboost_resource_configured: windows_bundle.catboost_resource_configured,
        windows_bundle_catboost_license_configured: windows_bundle
            .catboost_license_resource_configured,
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
    catboost_license_resource_configured: bool,
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
    let catboost_resource_configured = resources.is_some_and(|value| {
        bundle_resources_include_file(src_tauri, value, &catboost_path, "native/catboostmodel.dll")
    });
    let catboost_license_path = src_tauri
        .join("resources")
        .join("licenses")
        .join("CatBoost-LICENSE.txt");
    let catboost_license_resource_configured = resources.is_some_and(|value| {
        bundle_resources_include_file(
            src_tauri,
            value,
            &catboost_license_path,
            "licenses/CatBoost-LICENSE.txt",
        )
    });
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
        catboost_license_resource_configured,
        nsis_target_configured,
        webview_install_mode,
        installer_icon_exists,
    }
}

fn bundle_resources_include_file(
    src_tauri: &Path,
    resources: &Value,
    expected_source_path: &Path,
    expected_target: &str,
) -> bool {
    if let Some(map) = resources.as_object() {
        return map.iter().any(|(source, target)| {
            resolve_config_relative(src_tauri, source) == expected_source_path
                && target
                    .as_str()
                    .is_some_and(|value| normalize_resource_path(value) == expected_target)
        });
    }
    if let Some(items) = resources.as_array() {
        return items.iter().any(|item| {
            item.as_str().is_some_and(|source| {
                resolve_config_relative(src_tauri, source) == expected_source_path
            })
        });
    }
    false
}

fn sha256_file_hex(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).ok()?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Some(format!("{:x}", hasher.finalize()))
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
        repo_root.join("target").join("release"),
        repo_root.join("src-tauri").join("target").join("release"),
    ];
    let selected_release_root = release_roots
        .iter()
        .find(|root| root.join("ta_desktop_demo_app.exe").is_file() || root.join("bundle").is_dir())
        .cloned();
    let release_binary_path = selected_release_root
        .as_ref()
        .map(|root| root.join("ta_desktop_demo_app.exe"))
        .filter(|path| path.is_file());
    let release_bundle_dir_path = selected_release_root
        .as_ref()
        .map(|root| root.join("bundle"))
        .filter(|path| path.is_dir());
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProductVersion {
    value: String,
    components: Vec<u32>,
}

impl ProductVersion {
    fn major(&self) -> u32 {
        self.components[0]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledEdgeRuntime {
    kind: &'static str,
    path: PathBuf,
    version: ProductVersion,
}

fn parse_product_version(raw: &str) -> Option<ProductVersion> {
    raw.split(|character: char| !(character.is_ascii_digit() || character == '.'))
        .filter(|candidate| !candidate.is_empty())
        .find_map(|candidate| {
            let components = candidate
                .split('.')
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            if components.len() < 3 {
                return None;
            }
            Some(ProductVersion {
                value: components
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join("."),
                components,
            })
        })
}

fn command_version(path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(path).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = [stdout.as_ref(), stderr.as_ref()]
        .into_iter()
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join(" | ");
    (!combined.is_empty()).then_some(combined)
}

fn installed_edge_runtime_candidates() -> Vec<InstalledEdgeRuntime> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let program_files_x86 = env::var_os("ProgramFiles(x86)").map(PathBuf::from);
    let program_files = env::var_os("ProgramFiles").map(PathBuf::from);
    let local_app_data = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let mut roots = Vec::new();
    for base in [program_files_x86, program_files, local_app_data]
        .into_iter()
        .flatten()
    {
        roots.push((
            "webview2",
            base.join("Microsoft")
                .join("EdgeWebView")
                .join("Application"),
            "msedgewebview2.exe",
        ));
        roots.push((
            "edge",
            base.join("Microsoft").join("Edge").join("Application"),
            "msedge.exe",
        ));
    }
    roots
        .into_iter()
        .flat_map(|(kind, root, executable)| {
            std::fs::read_dir(root)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
                .filter_map(move |entry| {
                    let version = parse_product_version(&entry.file_name().to_string_lossy())?;
                    let path = entry.path().join(executable);
                    path.is_file().then_some(InstalledEdgeRuntime {
                        kind,
                        path,
                        version,
                    })
                })
        })
        .collect()
}

fn newest_edge_runtime(
    candidates: &[InstalledEdgeRuntime],
    kind: &'static str,
) -> Option<InstalledEdgeRuntime> {
    candidates
        .iter()
        .filter(|candidate| candidate.kind == kind)
        .max_by(|left, right| left.version.components.cmp(&right.version.components))
        .cloned()
}

fn edge_driver_matches_runtime_major(
    driver: Option<&ProductVersion>,
    runtime: Option<&InstalledEdgeRuntime>,
) -> Option<bool> {
    driver
        .zip(runtime)
        .map(|(driver, runtime)| driver.major() == runtime.version.major())
}

fn edge_driver_path(repo_root: &Path) -> (Option<PathBuf>, bool) {
    let override_path = env::var("VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    edge_driver_path_with_override(repo_root, override_path)
}

fn edge_driver_path_with_override(
    repo_root: &Path,
    override_path: Option<PathBuf>,
) -> (Option<PathBuf>, bool) {
    if let Some(override_path) = override_path {
        return (override_path.is_file().then_some(override_path), true);
    }
    (
        find_command_path(
            "msedgedriver",
            Some(&[
                repo_root.join("msedgedriver.exe"),
                repo_root.join("web").join("msedgedriver.exe"),
            ]),
        ),
        false,
    )
}

fn archive_tool_path() -> Option<PathBuf> {
    let mut fallbacks = Vec::new();
    if let Some(program_files) = env::var_os("ProgramFiles") {
        fallbacks.push(PathBuf::from(program_files).join("7-Zip").join("7z.exe"));
    }
    if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
        fallbacks.push(
            PathBuf::from(program_files_x86)
                .join("7-Zip")
                .join("7z.exe"),
        );
    }
    find_command_path("7z", Some(&fallbacks)).or_else(|| find_command_path("7za", None))
}

fn selected_edge_runtime_version(tooling: &ReleaseToolingReadiness) -> Option<&str> {
    match tooling.edge_compatibility_target.as_deref() {
        Some("webview2") => tooling.webview2_runtime_version.as_deref(),
        Some("edge") => tooling.edge_browser_version.as_deref(),
        _ => None,
    }
}

fn tooling_readiness(repo_root: &Path) -> ReleaseToolingReadiness {
    let cargo_path = find_command_path("cargo", None);
    let npm_path = find_command_path("npm", None);
    let tauri_driver_path = find_command_path("tauri-driver", None);
    let signtool_path = find_command_path("signtool", None);
    let makensis_path = find_command_path("makensis", None);
    let signing_config = windows_signing_config(repo_root);
    let (edge_driver_path, edge_driver_override_configured) = edge_driver_path(repo_root);
    let edge_driver_version = edge_driver_path
        .as_deref()
        .and_then(|path| command_version(path, &["--version"]))
        .and_then(|raw| parse_product_version(&raw));
    let runtime_candidates = installed_edge_runtime_candidates();
    let webview2_runtime = newest_edge_runtime(&runtime_candidates, "webview2");
    let edge_browser = newest_edge_runtime(&runtime_candidates, "edge");
    let compatibility_runtime = webview2_runtime.as_ref().or(edge_browser.as_ref());
    let edge_driver_major_compatible =
        edge_driver_matches_runtime_major(edge_driver_version.as_ref(), compatibility_runtime);
    let archive_tool_path = archive_tool_path();
    let archive_tool_version = archive_tool_path
        .as_deref()
        .and_then(|path| command_version(path, &[]));

    ReleaseToolingReadiness {
        cargo_available: cargo_path.is_some(),
        npm_available: npm_path.is_some(),
        cargo_tauri_available: cargo_tauri_available(),
        tauri_driver_available: tauri_driver_path.is_some(),
        edge_driver_available: edge_driver_path.is_some(),
        edge_driver_override_configured,
        edge_driver_version: edge_driver_version
            .as_ref()
            .map(|version| version.value.clone()),
        edge_driver_major: edge_driver_version.as_ref().map(ProductVersion::major),
        webview2_runtime_path: webview2_runtime
            .as_ref()
            .map(|runtime| runtime.path.to_string_lossy().to_string()),
        webview2_runtime_version: webview2_runtime
            .as_ref()
            .map(|runtime| runtime.version.value.clone()),
        webview2_runtime_major: webview2_runtime
            .as_ref()
            .map(|runtime| runtime.version.major()),
        edge_browser_path: edge_browser
            .as_ref()
            .map(|runtime| runtime.path.to_string_lossy().to_string()),
        edge_browser_version: edge_browser
            .as_ref()
            .map(|runtime| runtime.version.value.clone()),
        edge_browser_major: edge_browser.as_ref().map(|runtime| runtime.version.major()),
        edge_compatibility_target: compatibility_runtime.map(|runtime| runtime.kind.to_string()),
        edge_driver_major_compatible,
        archive_tool_available: archive_tool_path.is_some() && archive_tool_version.is_some(),
        signtool_available: signtool_path.is_some(),
        custom_signing_hook_configured: signing_config.sign_command_configured,
        windows_certificate_thumbprint_configured: signing_config.certificate_thumbprint_configured,
        windows_timestamp_url_configured: signing_config.timestamp_url_configured,
        makensis_available: makensis_path.is_some(),
        cargo_path: cargo_path.map(|path| path.to_string_lossy().to_string()),
        npm_path: npm_path.map(|path| path.to_string_lossy().to_string()),
        tauri_driver_path: tauri_driver_path.map(|path| path.to_string_lossy().to_string()),
        edge_driver_path: edge_driver_path.map(|path| path.to_string_lossy().to_string()),
        archive_tool_path: archive_tool_path.map(|path| path.to_string_lossy().to_string()),
        archive_tool_version,
        signtool_path: signtool_path.map(|path| path.to_string_lossy().to_string()),
        makensis_path: makensis_path.map(|path| path.to_string_lossy().to_string()),
    }
}

#[derive(Debug, Clone, Default)]
struct WindowsSigningConfig {
    sign_command_configured: bool,
    certificate_thumbprint_configured: bool,
    timestamp_url_configured: bool,
}

fn windows_signing_config(repo_root: &Path) -> WindowsSigningConfig {
    let config_path = repo_root.join("src-tauri").join("tauri.windows.conf.json");
    let Ok(raw) = std::fs::read_to_string(config_path) else {
        return WindowsSigningConfig::default();
    };
    let Ok(config) = serde_json::from_str::<Value>(&raw) else {
        return WindowsSigningConfig::default();
    };
    let Some(windows) = config.pointer("/bundle/windows") else {
        return WindowsSigningConfig::default();
    };
    WindowsSigningConfig {
        sign_command_configured: non_empty_json_text(windows.get("signCommand")),
        certificate_thumbprint_configured: non_empty_json_text(
            windows.get("certificateThumbprint"),
        ),
        timestamp_url_configured: non_empty_json_text(windows.get("timestampUrl")),
    }
}

fn updater_readiness(repo_root: &Path) -> ReleaseUpdaterReadiness {
    let config_path = repo_root.join("src-tauri").join("tauri.conf.json");
    let Ok(raw) = std::fs::read_to_string(config_path) else {
        return ReleaseUpdaterReadiness {
            configured: false,
            active: false,
            endpoint_count: 0,
            pubkey_configured: false,
            intentionally_disabled: true,
            status: "missing-config".to_string(),
        };
    };
    let Ok(config) = serde_json::from_str::<Value>(&raw) else {
        return ReleaseUpdaterReadiness {
            configured: false,
            active: false,
            endpoint_count: 0,
            pubkey_configured: false,
            intentionally_disabled: true,
            status: "invalid-config".to_string(),
        };
    };
    let Some(updater) = config.pointer("/plugins/updater") else {
        return ReleaseUpdaterReadiness {
            configured: false,
            active: false,
            endpoint_count: 0,
            pubkey_configured: false,
            intentionally_disabled: true,
            status: "missing-updater-config".to_string(),
        };
    };
    let active = updater
        .get("active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let endpoint_count = updater
        .get("endpoints")
        .and_then(Value::as_array)
        .map(|endpoints| {
            endpoints
                .iter()
                .filter(|endpoint| non_empty_json_text(Some(endpoint)))
                .count()
        })
        .unwrap_or(0);
    let pubkey_configured = non_empty_json_text(updater.get("pubkey"));
    let intentionally_disabled = !active;
    let status = if intentionally_disabled {
        "disabled"
    } else if endpoint_count > 0 && pubkey_configured {
        "active-configured"
    } else {
        "active-incomplete"
    };
    ReleaseUpdaterReadiness {
        configured: true,
        active,
        endpoint_count,
        pubkey_configured,
        intentionally_disabled,
        status: status.to_string(),
    }
}

fn non_empty_json_text(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|raw| !raw.trim().is_empty())
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
        collect_release_readiness, edge_driver_matches_runtime_major,
        edge_driver_path_with_override, gpu_kernel_readiness_errors, newest_edge_runtime,
        parse_product_version, resolve_release_build_paths, shipping_gpu_kernel_ptx_contract,
        updater_readiness, windows_signing_config, InstalledEdgeRuntime, ReleaseGpuReadiness,
        ReleaseReadinessOptions, ReleaseVariant,
    };
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root parent")
            .to_path_buf()
    }

    fn gpu_kernel_readiness_fixture(
        compute_capability: Option<(u32, u32)>,
        double_crossover_runtime_ready: bool,
        options_expression_runtime_ready: bool,
    ) -> ReleaseGpuReadiness {
        ReleaseGpuReadiness {
            built_with_cuda: true,
            built_with_cuda_backtest_kernel: true,
            cuda_available: true,
            nvidia_smi_available: true,
            auto_device_name: Some("Fixture CUDA device".to_string()),
            auto_device_memory_bytes: Some(16 * 1024 * 1024 * 1024),
            auto_device_compute_capability_major: compute_capability.map(|value| value.0),
            auto_device_compute_capability_minor: compute_capability.map(|value| value.1),
            auto_device_error: None,
            gpu_kernel_ptx_contract: Some(shipping_gpu_kernel_ptx_contract(compute_capability)),
            double_crossover_module_jit_loaded: Some(double_crossover_runtime_ready),
            double_crossover_module_jit_error: (!double_crossover_runtime_ready)
                .then(|| "fixture double-crossover prewarm failure".to_string()),
            options_expression_module_jit_loaded: Some(options_expression_runtime_ready),
            options_expression_module_jit_error: (!options_expression_runtime_ready)
                .then(|| "fixture options-expression prewarm failure".to_string()),
        }
    }

    #[test]
    fn release_variant_strings_match_expected_contract() {
        assert_eq!(ReleaseVariant::Cpu.as_str(), "cpu");
        assert_eq!(ReleaseVariant::GpuMa.as_str(), "gpu-ma");
        assert_eq!(ReleaseVariant::GpuKernel.as_str(), "gpu-kernel");
    }

    #[test]
    fn product_version_parser_extracts_edge_versions_and_rejects_incomplete_values() {
        let parsed =
            parse_product_version("Microsoft Edge WebDriver 150.0.4078.99 (fixture revision 123)")
                .expect("parse EdgeDriver product version");
        assert_eq!(parsed.value, "150.0.4078.99");
        assert_eq!(parsed.components, vec![150, 0, 4078, 99]);
        assert_eq!(parsed.major(), 150);
        assert!(parse_product_version("Microsoft Edge WebDriver unavailable").is_none());
        assert!(parse_product_version("build 150").is_none());
    }

    #[test]
    fn newest_edge_runtime_uses_numeric_version_ordering() {
        let candidates = vec![
            InstalledEdgeRuntime {
                kind: "webview2",
                path: PathBuf::from("webview-150.0.4078.83"),
                version: parse_product_version("150.0.4078.83").expect("older version"),
            },
            InstalledEdgeRuntime {
                kind: "edge",
                path: PathBuf::from("edge-151.0.1.0"),
                version: parse_product_version("151.0.1.0").expect("Edge version"),
            },
            InstalledEdgeRuntime {
                kind: "webview2",
                path: PathBuf::from("webview-150.0.4078.105"),
                version: parse_product_version("150.0.4078.105").expect("newer version"),
            },
        ];

        let selected =
            newest_edge_runtime(&candidates, "webview2").expect("select newest WebView2");
        assert_eq!(selected.version.value, "150.0.4078.105");
        assert_eq!(selected.path, PathBuf::from("webview-150.0.4078.105"));
    }

    #[test]
    fn edge_driver_compatibility_is_major_version_scoped_and_unknown_fails_unproven() {
        let matching_driver =
            parse_product_version("Microsoft Edge WebDriver 150.0.1.2").expect("matching driver");
        let stale_driver =
            parse_product_version("Microsoft Edge WebDriver 148.0.9.9").expect("stale driver");
        let runtime = InstalledEdgeRuntime {
            kind: "webview2",
            path: PathBuf::from("msedgewebview2.exe"),
            version: parse_product_version("150.9.9999.1").expect("runtime"),
        };

        assert_eq!(
            edge_driver_matches_runtime_major(Some(&matching_driver), Some(&runtime)),
            Some(true)
        );
        assert_eq!(
            edge_driver_matches_runtime_major(Some(&stale_driver), Some(&runtime)),
            Some(false)
        );
        assert_eq!(
            edge_driver_matches_runtime_major(Some(&matching_driver), None),
            None
        );
        assert_eq!(
            edge_driver_matches_runtime_major(None, Some(&runtime)),
            None
        );
    }

    #[test]
    fn explicit_edge_driver_override_never_falls_back_to_repo_or_path_discovery() {
        let unique = format!(
            "vectorgrid-release-readiness-edge-override-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after epoch")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&root).expect("create temporary readiness root");
        std::fs::write(root.join("msedgedriver.exe"), b"repo fallback")
            .expect("write fallback driver fixture");

        let missing_override = root.join("explicit-driver-does-not-exist.exe");
        let (resolved, configured) = edge_driver_path_with_override(&root, Some(missing_override));
        assert!(configured);
        assert!(
            resolved.is_none(),
            "an invalid explicit override must not fall back to the repo or PATH"
        );

        let (fallback, configured) = edge_driver_path_with_override(&root, None);
        assert!(!configured);
        assert_eq!(fallback, Some(root.join("msedgedriver.exe")));
        std::fs::remove_dir_all(root).expect("remove temporary readiness root");
    }

    #[test]
    fn gpu_kernel_gate_rejects_device_below_shipping_ptx_target() {
        let gpu = gpu_kernel_readiness_fixture(Some((8, 6)), false, false);
        let errors = gpu_kernel_readiness_errors(&gpu);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("sm_86"));
        assert!(errors[0].contains("sm_89"));
        assert!(errors[0].contains("PTX ISA 9.2"));
        assert_eq!(
            gpu.gpu_kernel_ptx_contract
                .as_ref()
                .and_then(|contract| contract.preferred_device_meets_minimum),
            Some(false)
        );
    }

    #[test]
    fn gpu_kernel_gate_accepts_minimum_and_newer_compute_capabilities() {
        assert!(gpu_kernel_readiness_errors(&gpu_kernel_readiness_fixture(
            Some((8, 9)),
            true,
            true,
        ))
        .is_empty());
        assert!(gpu_kernel_readiness_errors(&gpu_kernel_readiness_fixture(
            Some((9, 0)),
            true,
            true,
        ))
        .is_empty());
    }

    #[test]
    fn gpu_kernel_gate_reports_runtime_prewarm_without_mislabeling_it_as_jit_only() {
        let errors =
            gpu_kernel_readiness_errors(&gpu_kernel_readiness_fixture(Some((8, 9)), false, true));

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("GPU runtime prewarm failed"));
        assert!(errors[0].contains("required-symbol resolution"));
        assert!(!errors[0].contains("driver-JIT"));
    }

    #[test]
    fn gpu_readiness_serialization_records_contract_and_accepts_legacy_shape() {
        let gpu = gpu_kernel_readiness_fixture(Some((8, 9)), true, true);
        let value = serde_json::to_value(&gpu).expect("serialize GPU readiness");
        assert_eq!(value["auto_device_compute_capability_major"], 8);
        assert_eq!(value["auto_device_compute_capability_minor"], 9);
        assert_eq!(
            value["gpu_kernel_ptx_contract"]["shipping_ptx_isa_version"],
            "9.2"
        );
        assert_eq!(
            value["gpu_kernel_ptx_contract"]["shipping_ptx_target"],
            "sm_89"
        );
        assert_eq!(
            value["gpu_kernel_ptx_contract"]["preferred_device_meets_minimum"],
            true
        );
        assert_eq!(value["double_crossover_module_jit_loaded"], true);

        let legacy = serde_json::json!({
            "built_with_cuda": false,
            "built_with_cuda_backtest_kernel": false,
            "cuda_available": false,
            "nvidia_smi_available": false
        });
        let decoded: ReleaseGpuReadiness =
            serde_json::from_value(legacy).expect("deserialize legacy GPU readiness");
        assert_eq!(decoded.auto_device_compute_capability_major, None);
        assert_eq!(decoded.auto_device_compute_capability_minor, None);
        assert!(decoded.gpu_kernel_ptx_contract.is_none());
        assert_eq!(decoded.double_crossover_module_jit_loaded, None);
    }

    #[test]
    fn gpu_kernel_structured_contract_matches_shipping_ptx_headers() {
        for relative_path in [
            "crates/app_core/src/kernels/double_crossover.prebuilt.ptx",
            "crates/app_core/src/kernels/options_expression.prebuilt.ptx",
        ] {
            let path = repo_root().join(relative_path);
            let file = std::fs::File::open(&path)
                .unwrap_or_else(|error| panic!("open shipping PTX '{}': {error}", path.display()));
            let header = BufReader::new(file)
                .lines()
                .take(16)
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_else(|error| panic!("read shipping PTX '{}': {error}", path.display()))
                .join("\n");
            assert!(
                header.contains(".version 9.2"),
                "shipping PTX '{}' no longer matches the recorded PTX ISA contract",
                path.display()
            );
            assert!(
                header.contains(".target sm_89"),
                "shipping PTX '{}' no longer matches the recorded PTX target contract",
                path.display()
            );
        }
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
        assert!(
            report
                .resources
                .catboost_bundle_resource_matches_reviewed_release
        );
        assert!(report.resources.catboost_provenance_baseline_exists);
        assert!(
            report
                .resources
                .catboost_provenance_baseline_matches_reviewed_release
        );
        assert!(report.resources.catboost_license_resource_exists);
        assert!(
            report
                .resources
                .catboost_license_resource_matches_reviewed_release
        );
        assert!(report.resources.windows_bundle_catboost_resource_configured);
        assert!(report.resources.windows_bundle_catboost_license_configured);
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
        assert!(report.updater.configured);
        assert!(report.updater.intentionally_disabled);
        assert_eq!(report.updater.status, "disabled");
    }

    #[test]
    fn updater_readiness_reports_active_incomplete_config() {
        let temp_root = std::env::temp_dir()
            .join("vectorgrid_release_readiness_updater")
            .join(format!("{}", std::process::id()));
        let src_tauri = temp_root.join("src-tauri");
        std::fs::create_dir_all(&src_tauri).expect("src-tauri");
        std::fs::write(
            src_tauri.join("tauri.conf.json"),
            r#"{
              "plugins": {
                "updater": {
                  "active": true,
                  "endpoints": [],
                  "pubkey": ""
                }
              }
            }"#,
        )
        .expect("tauri config");

        let updater = updater_readiness(&temp_root);
        assert!(updater.configured);
        assert!(updater.active);
        assert_eq!(updater.endpoint_count, 0);
        assert!(!updater.pubkey_configured);
        assert_eq!(updater.status, "active-incomplete");
        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn windows_signing_config_detects_sign_command_and_timestamp() {
        let temp_root = std::env::temp_dir()
            .join("vectorgrid_release_readiness_signing")
            .join(format!("{}", std::process::id()));
        let src_tauri = temp_root.join("src-tauri");
        std::fs::create_dir_all(&src_tauri).expect("src-tauri");
        std::fs::write(
            src_tauri.join("tauri.windows.conf.json"),
            r#"{
              "bundle": {
                "windows": {
                  "signCommand": "C:/sign/vectorgrid-sign.cmd",
                  "timestampUrl": "http://timestamp.example.invalid"
                }
              }
            }"#,
        )
        .expect("windows config");

        let signing = windows_signing_config(&temp_root);
        assert!(signing.sign_command_configured);
        assert!(!signing.certificate_thumbprint_configured);
        assert!(signing.timestamp_url_configured);
        let _ = std::fs::remove_dir_all(temp_root);
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

    #[test]
    fn resolve_release_build_paths_never_mixes_target_roots() {
        let temp_root = std::env::temp_dir()
            .join("vectorgrid_release_readiness_single_target_root")
            .join(format!("{}", std::process::id()));
        let workspace_release = temp_root.join("target").join("release");
        let legacy_release = temp_root.join("src-tauri").join("target").join("release");
        std::fs::create_dir_all(&workspace_release).expect("workspace release dir");
        std::fs::create_dir_all(legacy_release.join("bundle").join("nsis"))
            .expect("legacy bundle dir");
        std::fs::write(
            workspace_release.join("ta_desktop_demo_app.exe"),
            b"workspace-binary",
        )
        .expect("workspace release binary");
        std::fs::write(
            legacy_release
                .join("bundle")
                .join("nsis")
                .join("stale-legacy.exe"),
            b"legacy-bundle",
        )
        .expect("legacy bundle artifact");

        let resolved = resolve_release_build_paths(&temp_root);
        assert_eq!(
            resolved.release_binary_path,
            Some(workspace_release.join("ta_desktop_demo_app.exe"))
        );
        assert_eq!(resolved.release_bundle_dir_path, None);
        assert_eq!(resolved.release_bundle_artifact_count, 0);

        let _ = std::fs::remove_dir_all(temp_root);
    }
}
