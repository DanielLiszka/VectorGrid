use clap::Parser;
use ta_app_core::{
    export_ml_signal_dataset_with_candles, read_candles_from_csv_compat,
    MlSignalDatasetExportRequest,
};

#[derive(Parser, Debug)]
#[command(name = "ta_ml_signal_export_dataset")]
#[command(about = "Headless MlSignal dataset export runner")]
struct Cli {
    #[arg(long)]
    csv_path: String,

    #[arg(long)]
    output_path: String,

    #[arg(long)]
    data_id: Option<String>,

    #[arg(long, default_value = "close")]
    ma_source: String,

    #[arg(long, default_value_t = 10)]
    horizon_bars: usize,

    #[arg(long, default_value_t = 12.5)]
    round_trip_cost_bps: f64,
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let candles = read_candles_from_csv_compat(&cli.csv_path)?;
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result = export_ml_signal_dataset_with_candles(
        MlSignalDatasetExportRequest {
            data_id: cli
                .data_id
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| cli.csv_path.clone()),
            output_path: cli.output_path,
            ma_source: cli.ma_source,
            horizon_bars: cli.horizon_bars,
            round_trip_cost_bps: cli.round_trip_cost_bps,
            split_plan: None,
            ticker_holdout: None,
        },
        &candles,
        &cancel,
    )?;
    let rendered = serde_json::to_string_pretty(&result)
        .map_err(|e| format!("serialize export result: {e}"))?;
    println!("{rendered}");
    Ok(())
}
