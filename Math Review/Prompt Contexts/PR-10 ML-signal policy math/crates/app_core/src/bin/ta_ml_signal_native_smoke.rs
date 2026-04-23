use clap::Parser;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};
use ta_app_core::ml_signal::MlSignalInferenceBackendPreference;
use ta_app_core::{
    build_ml_signal_score_cache_from_csv, read_candles_from_csv_compat, read_ml_signal_manifest,
    MlSignalScoreCacheBuildRequest,
};

#[derive(Parser, Debug)]
#[command(name = "ta_ml_signal_native_smoke")]
#[command(about = "Headless MlSignal native CatBoost score-cache smoke runner")]
struct Cli {
    #[arg(long)]
    csv_path: PathBuf,

    #[arg(long)]
    bundle_manifest: PathBuf,

    #[arg(long)]
    score_cache_out: Option<PathBuf>,

    #[arg(long)]
    out_json: Option<PathBuf>,

    #[arg(long)]
    native_library_path: Option<PathBuf>,

    #[arg(long, default_value = "native_cpu")]
    backend_preference: String,

    #[arg(long)]
    gpu_device_id: Option<i32>,

    #[arg(long)]
    data_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct SmokeOutput {
    csv_path: String,
    bundle_manifest: String,
    backend_preference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_library_path: Option<String>,
    result: ta_app_core::MlSignalScoreCacheBuildResult,
}

fn parse_backend_preference(raw: &str) -> Result<MlSignalInferenceBackendPreference, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(MlSignalInferenceBackendPreference::Auto),
        "native_cuda" | "native-cuda" | "cuda" => {
            Ok(MlSignalInferenceBackendPreference::NativeCuda)
        }
        "native_cpu" | "native-cpu" | "cpu" => Ok(MlSignalInferenceBackendPreference::NativeCpu),
        "score_cache_only" | "score-cache-only" | "score_cache" | "score-cache" => Err(
            "backend_preference 'score_cache_only' is incompatible with the native smoke harness; use auto, native_cuda, or native_cpu".to_string(),
        ),
        other => Err(format!(
            "unsupported backend_preference '{other}'; expected auto, native_cuda, or native_cpu"
        )),
    }
}

fn require_file(path: &std::path::Path, label: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!(
            "{label} '{}' does not exist or is not a file",
            path.display()
        ))
    }
}

fn ensure_parent_dir(path: &std::path::Path, label: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "create parent directory for {label} '{}': {e}",
            path.display()
        )
    })
}

fn default_score_cache_out() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("ta_ml_signal_native_smoke_{nonce}.json"))
}

fn default_data_id(csv_path: &PathBuf) -> String {
    csv_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("ml_signal_native_smoke")
        .to_string()
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let backend_preference = parse_backend_preference(&cli.backend_preference)?;
    require_file(&cli.csv_path, "csv_path")?;
    require_file(&cli.bundle_manifest, "bundle_manifest")?;
    if let Some(path) = cli.native_library_path.as_ref() {
        require_file(path, "native_library_path")?;
    }
    let candles = read_candles_from_csv_compat(
        cli.csv_path
            .to_str()
            .ok_or_else(|| "csv_path contains invalid UTF-8".to_string())?,
    )?;
    let manifest = read_ml_signal_manifest(&cli.bundle_manifest)?;
    let output_path = cli.score_cache_out.unwrap_or_else(default_score_cache_out);
    ensure_parent_dir(&output_path, "score_cache_out")?;
    let native_library_path = cli
        .native_library_path
        .as_ref()
        .map(|value| value.to_string_lossy().to_string());
    let data_id = cli
        .data_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_data_id(&cli.csv_path));
    let req = MlSignalScoreCacheBuildRequest {
        data_id,
        input_path: String::new(),
        output_path: output_path.to_string_lossy().to_string(),
        model_bundle_id: Some(manifest.bundle_id.clone()),
        feature_schema_path: None,
        feature_schema_hash: None,
        export_manifest_path: None,
        source_bundle_manifest_path: Some(cli.bundle_manifest.to_string_lossy().to_string()),
        calibration_state: None,
        threshold_policy: None,
        producer_kind: None,
        score_contract_version: None,
        native_backend_preference: backend_preference,
        native_library_path: native_library_path.clone(),
        native_gpu_device_id: cli.gpu_device_id,
    };
    let cancel = AtomicBool::new(false);
    let result = build_ml_signal_score_cache_from_csv(req, &candles, &cancel)?;
    let output = SmokeOutput {
        csv_path: cli.csv_path.to_string_lossy().to_string(),
        bundle_manifest: cli.bundle_manifest.to_string_lossy().to_string(),
        backend_preference: cli.backend_preference,
        native_library_path,
        result,
    };
    let rendered = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("serialize smoke output: {e}"))?;
    if let Some(path) = cli.out_json {
        ensure_parent_dir(&path, "out_json")?;
        fs::write(&path, rendered.as_bytes())
            .map_err(|e| format!("write smoke output '{}': {e}", path.display()))?;
    }
    println!("{rendered}");
    Ok(())
}
