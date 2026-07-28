use clap::Parser;
use std::path::PathBuf;
use ta_app_core::{
    export_ml_signal_dataset_with_candles, read_candles_from_csv_compat,
    MlSignalDatasetExportRequest, MlSignalSplitPlan, MlSignalTickerHoldoutConfig,
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

    #[arg(
        long,
        help = "Optional MlSignalSplitPlan JSON. Required for reproducible walk-forward training bundles."
    )]
    split_plan_json: Option<PathBuf>,

    #[arg(long)]
    ticker_holdout_modulus: Option<usize>,

    #[arg(long, default_value_t = 0)]
    ticker_holdout_residue: usize,
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let split_plan = cli
        .split_plan_json
        .as_ref()
        .map(|path| {
            let raw = std::fs::read_to_string(path)
                .map_err(|error| format!("read split plan '{}': {error}", path.display()))?;
            serde_json::from_str::<MlSignalSplitPlan>(&raw)
                .map_err(|error| format!("parse split plan '{}': {error}", path.display()))
        })
        .transpose()?;
    let ticker_holdout = cli
        .ticker_holdout_modulus
        .map(|modulus| {
            if modulus == 0 {
                return Err("ticker_holdout_modulus must be greater than zero".to_string());
            }
            if cli.ticker_holdout_residue >= modulus {
                return Err(format!(
                    "ticker_holdout_residue {} must be smaller than modulus {modulus}",
                    cli.ticker_holdout_residue
                ));
            }
            Ok(MlSignalTickerHoldoutConfig {
                modulus,
                residue: cli.ticker_holdout_residue,
            })
        })
        .transpose()?;
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
            split_plan,
            ticker_holdout,
        },
        &candles,
        &cancel,
    )?;
    let rendered = serde_json::to_string_pretty(&result)
        .map_err(|e| format!("serialize export result: {e}"))?;
    println!("{rendered}");
    Ok(())
}
