use clap::{Parser, ValueEnum};
use serde::Serialize;
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use ta_app_core::{
    parse_timeframe_label, read_candles_from_csv_compat, resample_candles,
    run_double_ma_optimization_blocking_with_candles, Backend, DoubleMaRequest,
};
use ta_desktop_demo_app::{commands::save_run_artifact, data_cache};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use tauri::Manager;

#[derive(Clone, Copy, Debug)]
struct U32Range {
    start: u32,
    end: u32,
    step: u32,
}

impl FromStr for U32Range {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = raw.trim().split(':').collect();
        match parts.len() {
            1 => {
                let value = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self {
                    start: value,
                    end: value,
                    step: 0,
                })
            }
            2 => {
                let start = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                let end = parts[1].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self {
                    start,
                    end,
                    step: 1,
                })
            }
            3 => {
                let start = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                let end = parts[1].parse::<u32>().map_err(|e| e.to_string())?;
                let step = parts[2].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self { start, end, step })
            }
            _ => Err("range must be N or start:end or start:end:step".to_string()),
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
enum BackendArg {
    Cpu,
    Auto,
    GpuKernel,
    GpuHybridKernel,
}

#[derive(ValueEnum, Clone, Debug)]
enum ObjectiveArg {
    Pnl,
    Sharpe,
    MaxDrawdown,
}

#[derive(Parser, Debug)]
#[command(name = "ta_options_parent_artifact")]
#[command(
    about = "Run a bounded DoubleMa optimization on a local CSV and save a normal run artifact for options-expression follow-up"
)]
struct Cli {
    #[arg(long)]
    csv: PathBuf,

    #[arg(long)]
    symbol: Option<String>,

    #[arg(long, default_value = "1m")]
    timeframe: String,

    #[arg(long, value_enum, default_value_t = BackendArg::Cpu)]
    backend: BackendArg,

    #[arg(long, default_value = "sma")]
    fast_ma: String,

    #[arg(long, default_value = "ema")]
    slow_ma: String,

    #[arg(long, default_value = "close")]
    ma_source: String,

    #[arg(long, default_value = "5:40:5")]
    fast_range: U32Range,

    #[arg(long, default_value = "20:100:10")]
    slow_range: U32Range,

    #[arg(long, value_enum, default_value_t = ObjectiveArg::Sharpe)]
    objective: ObjectiveArg,

    #[arg(long, default_value_t = 10)]
    top_k: usize,

    #[arg(long, default_value_t = 0)]
    heatmap_bins: u16,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    long_only: bool,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    allow_flip: bool,

    #[arg(long)]
    app_local_data_dir: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct CliOutput {
    app_local_data_dir: String,
    artifact_path: String,
    artifact_bytes: usize,
    csv_path: String,
    symbol: String,
    timeframe: String,
    backend_requested: String,
    runtime_ms: u64,
    num_combos: u64,
    best_fast_len: u16,
    best_slow_len: u16,
    best_fast_ma: String,
    best_slow_ma: String,
}

fn backend_from_arg(value: &BackendArg) -> Backend {
    match value {
        BackendArg::Cpu => Backend::CpuOnly,
        BackendArg::Auto => Backend::Auto { device_id: 0 },
        BackendArg::GpuKernel => Backend::GpuKernel { device_id: 0 },
        BackendArg::GpuHybridKernel => Backend::GpuHybridKernel { device_id: 0 },
    }
}

fn objective_from_arg(value: &ObjectiveArg) -> ObjectiveKind {
    match value {
        ObjectiveArg::Pnl => ObjectiveKind::Pnl,
        ObjectiveArg::Sharpe => ObjectiveKind::Sharpe,
        ObjectiveArg::MaxDrawdown => ObjectiveKind::MaxDrawdown,
    }
}

fn derive_symbol(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "OPTIONS_PARENT".to_string())
}

fn config_fingerprint(cli: &Cli, csv_path: &Path, source_fp: &str) -> Result<String, String> {
    let fingerprint_source = serde_json::to_string(&json!({
        "csv_path": csv_path,
        "timeframe": cli.timeframe,
        "backend": format!("{:?}", cli.backend),
        "fast_ma": cli.fast_ma,
        "slow_ma": cli.slow_ma,
        "ma_source": cli.ma_source,
        "fast_range": {
            "start": cli.fast_range.start,
            "end": cli.fast_range.end,
            "step": cli.fast_range.step,
        },
        "slow_range": {
            "start": cli.slow_range.start,
            "end": cli.slow_range.end,
            "step": cli.slow_range.step,
        },
        "objective": format!("{:?}", cli.objective),
        "top_k": cli.top_k,
        "heatmap_bins": cli.heatmap_bins,
        "long_only": cli.long_only,
        "allow_flip": cli.allow_flip,
        "source_fp": source_fp,
    }))
    .map_err(|e| e.to_string())?;
    let mut hasher = DefaultHasher::new();
    fingerprint_source.hash(&mut hasher);
    Ok(format!("cfg1-headless-phase05-{:016x}", hasher.finish()))
}

fn dataset_meta_value(
    csv_path: &Path,
    symbol: &str,
    timeframe: &str,
    source_identity: data_cache::SourceIdentity,
    source_fingerprint: &str,
    resample_applied: bool,
) -> serde_json::Value {
    let base_timeframe = "1m";
    json!({
        "schema_version": 2,
        "csv_path": csv_path.to_string_lossy().to_string(),
        "label": symbol,
        "symbol": symbol,
        "timeframe": timeframe,
        "base_timeframe": base_timeframe,
        "derived_timeframe": timeframe,
        "resample_applied": resample_applied,
        "source_size_bytes": source_identity.size,
        "source_mtime_utc_ms": source_identity.mtime_utc_ms,
        "source_content_fingerprint": source_fingerprint,
    })
}

fn build_request(cli: &Cli, symbol: &str) -> Result<DoubleMaRequest, String> {
    let strategy = StrategyConfig {
        long_only: cli.long_only,
        allow_flip: cli.allow_flip,
        ..StrategyConfig::default()
    };

    Ok(DoubleMaRequest {
        backend: backend_from_arg(&cli.backend),
        data_id: format!("options_parent_{symbol}_{}", cli.timeframe),
        fast_range: (
            cli.fast_range.start,
            cli.fast_range.end,
            cli.fast_range.step,
        ),
        slow_range: (
            cli.slow_range.start,
            cli.slow_range.end,
            cli.slow_range.step,
        ),
        fast_ma_types: vec![cli.fast_ma.clone()],
        slow_ma_types: vec![cli.slow_ma.clone()],
        ma_source: cli.ma_source.clone(),
        export_all_csv_path: None,
        fast_ma_params: None,
        slow_ma_params: None,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy,
        halftrend_filter: None,
        objective: objective_from_arg(&cli.objective),
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(cli.top_k),
        include_all: Some(false),
        heatmap_bins: Some(cli.heatmap_bins),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        include_trade_metrics: false,
        metric_contract: None,
        hurdle_tau: None,
    })
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    let app_handle = app.handle().clone();
    let resolved_app_local_data_dir = if let Some(explicit) = cli.app_local_data_dir.clone() {
        explicit
    } else {
        app_handle
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
    };

    let csv_path = std::fs::canonicalize(&cli.csv).unwrap_or_else(|_| cli.csv.clone());
    let symbol = cli
        .symbol
        .clone()
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| derive_symbol(&csv_path));
    let source_identity = data_cache::identity_for_source(&csv_path)?;
    let source_fingerprint = data_cache::content_fingerprint_for_source(&csv_path)?;

    let mut candles = read_candles_from_csv_compat(csv_path.to_string_lossy().as_ref())?;
    let timeframe = cli.timeframe.trim().to_string();
    let resample_applied = !timeframe.eq_ignore_ascii_case("1m");
    if resample_applied {
        let spec = parse_timeframe_label(&timeframe)?;
        candles = resample_candles(&candles, spec.interval_ms)?;
    }

    let req = build_request(&cli, &symbol)?;
    let cancel = AtomicBool::new(false);
    let result =
        run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)?;

    let ts = chrono::Utc::now().to_rfc3339();
    let config_fingerprint = config_fingerprint(&cli, &csv_path, &source_fingerprint)?;
    let mut run_req_value = serde_json::to_value(&req).map_err(|e| e.to_string())?;
    if let Some(object) = run_req_value.as_object_mut() {
        object.insert(
            "strategy_kind".to_string(),
            serde_json::Value::String("DoubleMa".to_string()),
        );
    }
    let artifact = json!({
        "version": 1,
        "kind": "vectorgrid_run_artifact",
        "ts": ts,
        "config_fingerprint": config_fingerprint,
        "run": {
            "req": run_req_value,
            "res": serde_json::to_value(&result).map_err(|e| e.to_string())?,
        },
        "report_xlsx_path": "",
        "data": {
            "csv_path": csv_path.to_string_lossy().to_string(),
        },
        "dataset_meta": dataset_meta_value(
            &csv_path,
            &symbol,
            &timeframe,
            source_identity,
            &source_fingerprint,
            resample_applied,
        ),
        "tags": ["options-parent", "phase05"],
    });

    let saved = tauri::async_runtime::block_on(save_run_artifact(artifact, None, app_handle))?;
    let output = CliOutput {
        app_local_data_dir: resolved_app_local_data_dir.to_string_lossy().to_string(),
        artifact_path: saved.path,
        artifact_bytes: saved.bytes,
        csv_path: csv_path.to_string_lossy().to_string(),
        symbol,
        timeframe,
        backend_requested: format!("{:?}", cli.backend),
        runtime_ms: result.runtime_ms,
        num_combos: result.num_combos as u64,
        best_fast_len: result.best_params.fast_len,
        best_slow_len: result.best_params.slow_len,
        best_fast_ma: result.best_params.fast_ma_type,
        best_slow_ma: result.best_params.slow_ma_type,
    };
    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(())
}
