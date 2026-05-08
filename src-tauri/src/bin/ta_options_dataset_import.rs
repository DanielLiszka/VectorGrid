use clap::Parser;
use serde::Serialize;
use std::path::PathBuf;
use ta_desktop_demo_app::commands::{
    import_options_dataset_blocking_headless, OptionsDatasetImportRequest,
};
use tauri::Manager;

#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;

#[derive(Parser, Debug)]
#[command(name = "ta_options_dataset_import")]
#[command(about = "Import a Polygon options CSV into the VectorGrid local dataset store")]
struct Cli {
    #[arg(long)]
    csv: PathBuf,

    #[arg(long)]
    app_local_data_dir: Option<PathBuf>,

    #[arg(long)]
    dataset_id: Option<String>,

    #[arg(long)]
    root_symbol: Option<String>,

    #[arg(long, default_value = "polygon")]
    provider_name: String,

    #[arg(long, default_value = "minute_aggregate")]
    provider_source_surface: String,

    #[arg(long)]
    provider_entitlement_profile: Option<String>,

    #[arg(long, default_value = "1m")]
    quote_resolution: String,

    #[arg(long, default_value = "minute_aggregate")]
    price_semantics: String,

    #[arg(long, default_value = "market_hours_utc")]
    calendar_id: String,

    #[arg(long, default_value = "UTC")]
    timezone: String,

    #[arg(long)]
    normalization_version: Option<u32>,

    #[arg(long)]
    quote_semantics_version: Option<u32>,

    #[arg(long)]
    corporate_action_mapping_version: Option<u32>,

    #[arg(long, default_value = "polygon_contract_reference")]
    contract_reference_source: String,

    #[arg(long)]
    contract_reference_as_of_utc: Option<String>,

    #[arg(long, default_value = "underlying_dataset")]
    underlier_reference_source: String,

    #[arg(long)]
    aggregate_price_field: Option<String>,

    #[arg(long)]
    out_json: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct ImportOutput {
    app_local_data_dir: String,
    imported: ta_app_core::OptionsDatasetManifestV1,
}

fn parse_price_semantics(raw: &str) -> Result<ta_app_core::OptionsPriceSemantics, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "minute_aggregate" => Ok(ta_app_core::OptionsPriceSemantics::MinuteAggregate),
        "top_of_book_quote" => Ok(ta_app_core::OptionsPriceSemantics::TopOfBookQuote),
        "derived_mid_snapshot" => Ok(ta_app_core::OptionsPriceSemantics::DerivedMidSnapshot),
        other => Err(format!(
            "unsupported price semantics '{other}' (expected minute_aggregate, top_of_book_quote, or derived_mid_snapshot)"
        )),
    }
}

fn build_request(cli: &Cli) -> Result<OptionsDatasetImportRequest, String> {
    Ok(OptionsDatasetImportRequest {
        csv_path: cli.csv.to_string_lossy().to_string(),
        options_dataset_id: cli.dataset_id.clone(),
        root_symbol: cli.root_symbol.clone(),
        provider_name: cli.provider_name.clone(),
        provider_source_surface: cli.provider_source_surface.clone(),
        provider_entitlement_profile: cli.provider_entitlement_profile.clone(),
        quote_resolution: cli.quote_resolution.clone(),
        price_semantics: parse_price_semantics(&cli.price_semantics)?,
        calendar_id: cli.calendar_id.clone(),
        timezone: cli.timezone.clone(),
        normalization_version: cli.normalization_version,
        quote_semantics_version: cli.quote_semantics_version,
        corporate_action_mapping_version: cli.corporate_action_mapping_version,
        contract_reference_source: cli.contract_reference_source.clone(),
        contract_reference_as_of_utc: cli.contract_reference_as_of_utc.clone(),
        underlier_reference_source: cli.underlier_reference_source.clone(),
        aggregate_price_field: cli.aggregate_price_field.clone(),
    })
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

    let manifest =
        import_options_dataset_blocking_headless(build_request(&cli)?, &app_local_data_dir)?;
    let output = ImportOutput {
        app_local_data_dir: app_local_data_dir.to_string_lossy().to_string(),
        imported: manifest,
    };
    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("failed to serialize import output: {e}"))?;
    if let Some(out_path) = cli.out_json.as_ref() {
        atomic_output::write_str(out_path, &json)
            .map_err(|e| format!("failed to write import JSON {}: {e}", out_path.display()))?;
    }
    println!("{json}");
    Ok(())
}
