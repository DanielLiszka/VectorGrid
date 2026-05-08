use clap::Parser;
use std::path::PathBuf;
use ta_desktop_demo_app::release_readiness::{
    collect_release_readiness, ReleaseReadinessOptions, ReleaseVariant,
};
use tauri::Manager;

#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;

#[derive(Parser, Debug)]
#[command(name = "ta_release_readiness")]
#[command(about = "Report local release-readiness facts for a VectorGrid release variant")]
struct Cli {
    #[arg(long, default_value = "cpu")]
    variant: String,

    #[arg(long)]
    app_local_data_dir: Option<PathBuf>,

    #[arg(long)]
    json: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    require_packaged_smoke: bool,

    #[arg(long, default_value_t = false)]
    require_public_signing: bool,
}

fn parse_release_variant(raw: &str) -> Result<ReleaseVariant, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "cpu" => Ok(ReleaseVariant::Cpu),
        "gpu-ma" => Ok(ReleaseVariant::GpuMa),
        "gpu-kernel" => Ok(ReleaseVariant::GpuKernel),
        other => Err(format!(
            "unknown variant '{other}' (expected cpu, gpu-ma, or gpu-kernel)"
        )),
    }
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let variant = parse_release_variant(&cli.variant)?;
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "failed to resolve repo root".to_string())?
        .to_path_buf();
    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    let app_handle = app.handle().clone();
    let app_local_data_dir = if let Some(explicit) = cli.app_local_data_dir {
        explicit
    } else {
        app_handle
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
    };
    let report = collect_release_readiness(
        &repo_root,
        &app.config().identifier,
        &app_local_data_dir,
        variant,
        &ReleaseReadinessOptions {
            require_packaged_smoke: cli.require_packaged_smoke,
            require_public_signing: cli.require_public_signing,
        },
    )?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    if let Some(path) = cli.json {
        atomic_output::write_str(&path, &json)?;
    }
    println!("{json}");
    Ok(())
}
