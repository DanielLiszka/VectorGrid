use clap::Parser;
use serde::Serialize;
use std::path::PathBuf;
use ta_desktop_demo_app::commands::{
    download_polygon_options_dataset_blocking_headless, PolygonOptionsDatasetDownloadRequest,
};
use tauri::Manager;

#[derive(Parser, Debug)]
#[command(name = "ta_options_dataset_download")]
#[command(
    about = "Download Polygon options contract-reference + 1m aggregate data into the local options dataset store"
)]
struct Cli {
    #[arg(long)]
    root_symbol: String,

    #[arg(long)]
    from: String,

    #[arg(long)]
    to: String,

    #[arg(long)]
    options_dataset_id: Option<String>,

    #[arg(long)]
    contract_type: Option<String>,

    #[arg(long)]
    expiration_date_from: Option<String>,

    #[arg(long)]
    expiration_date_to: Option<String>,

    #[arg(long)]
    max_dte_days: Option<u16>,

    #[arg(long)]
    strike_min: Option<f64>,

    #[arg(long)]
    strike_max: Option<f64>,

    #[arg(long)]
    max_contracts: Option<usize>,

    #[arg(long)]
    provider_entitlement_profile: Option<String>,

    #[arg(long)]
    contract_reference_as_of_utc: Option<String>,

    #[arg(long)]
    app_local_data_dir: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct CliOutput {
    app_local_data_dir: String,
    response: ta_desktop_demo_app::commands::PolygonOptionsDatasetDownloadResponse,
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    let app_handle = app.handle().clone();
    let app_local_data_dir = if let Some(explicit) = cli.app_local_data_dir.clone() {
        explicit
    } else {
        app_handle
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
    };
    let app_identifier = app.config().identifier.clone();
    let response = download_polygon_options_dataset_blocking_headless(
        PolygonOptionsDatasetDownloadRequest {
            root_symbol: cli.root_symbol,
            from: cli.from,
            to: cli.to,
            options_dataset_id: cli.options_dataset_id,
            contract_type: cli.contract_type,
            expiration_date_from: cli.expiration_date_from,
            expiration_date_to: cli.expiration_date_to,
            max_dte_days: cli.max_dte_days,
            strike_min: cli.strike_min,
            strike_max: cli.strike_max,
            max_contracts: cli.max_contracts,
            provider_entitlement_profile: cli.provider_entitlement_profile,
            contract_reference_as_of_utc: cli.contract_reference_as_of_utc,
        },
        &app_identifier,
        &app_local_data_dir,
    )?;
    let output = CliOutput {
        app_local_data_dir: app_local_data_dir.to_string_lossy().to_string(),
        response,
    };
    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(())
}
