use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use ta_app_core::{Backend, DoubleMaRequest};
use ta_desktop_demo_app::state::AppState;
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;

#[derive(Clone, Copy, Debug)]
struct U32Range {
    start: u32,
    end: u32,
    step: u32,
}

impl FromStr for U32Range {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            1 => {
                let v = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self {
                    start: v,
                    end: v,
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
    GpuKernel,
    GpuHybridKernel,
    Auto,
}

#[derive(ValueEnum, Clone, Debug)]
enum ObjectiveArg {
    Pnl,
    Sharpe,
    MaxDrawdown,
}

#[derive(Parser, Debug)]
#[command(name = "ta_app_path_faceoff")]
struct Cli {
    #[arg(long)]
    csv: PathBuf,

    #[arg(long)]
    output_json: PathBuf,

    #[arg(long)]
    cache_dir: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = BackendArg::GpuKernel)]
    backend: BackendArg,

    #[arg(long, default_value = "alma")]
    fast_ma: String,

    #[arg(long, default_value = "alma")]
    slow_ma: String,

    #[arg(long, default_value = "close")]
    ma_source: String,

    #[arg(long)]
    timeframe: Option<String>,

    #[arg(long, default_value = "5:200:1")]
    fast_period: U32Range,

    #[arg(long, default_value = "10:400:1")]
    slow_period: U32Range,

    #[arg(long, value_enum, default_value_t = ObjectiveArg::Sharpe)]
    objective: ObjectiveArg,

    #[arg(long, default_value_t = 10)]
    top_k: usize,

    #[arg(long, default_value_t = 0)]
    heatmap_bins: u16,

    #[arg(long, default_value_t = 0.85)]
    alma_offset: f64,

    #[arg(long, default_value_t = 6.0)]
    alma_sigma: f64,

    #[arg(long, default_value_t = 1)]
    warmup_trials: usize,

    #[arg(long, default_value_t = 5)]
    measured_trials: usize,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    long_only: bool,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    allow_flip: bool,
}

#[derive(Debug, Serialize)]
struct TrialArtifact {
    trial_index: usize,
    runtime_ms_wall: f64,
    result: ta_app_core::BackendOptimizationResult,
}

#[derive(Debug, Serialize)]
struct FaceoffDatasetArtifact {
    csv_path: String,
    cache_dir: String,
    load_ms: f64,
    resample_ms: Option<f64>,
    data_id: String,
    version: u64,
    bars: usize,
}

#[derive(Debug, Serialize)]
struct AppPathFaceoffArtifact {
    tool: &'static str,
    mode: &'static str,
    backend: String,
    dataset: FaceoffDatasetArtifact,
    warmup_trials: Vec<TrialArtifact>,
    measured_trials: Vec<TrialArtifact>,
    generated_at_utc: String,
}

fn backend_from_arg(arg: BackendArg) -> Backend {
    match arg {
        BackendArg::Cpu => Backend::CpuOnly,
        BackendArg::GpuKernel => Backend::GpuKernel { device_id: 0 },
        BackendArg::GpuHybridKernel => Backend::GpuHybridKernel { device_id: 0 },
        BackendArg::Auto => Backend::Auto { device_id: 0 },
    }
}

fn objective_from_arg(arg: ObjectiveArg) -> ObjectiveKind {
    match arg {
        ObjectiveArg::Pnl => ObjectiveKind::Pnl,
        ObjectiveArg::Sharpe => ObjectiveKind::Sharpe,
        ObjectiveArg::MaxDrawdown => ObjectiveKind::MaxDrawdown,
    }
}

fn alma_params(offset: f64, sigma: f64, ma_name: &str) -> Option<HashMap<String, f64>> {
    if !ma_name.trim().eq_ignore_ascii_case("alma") {
        return None;
    }
    let mut params = HashMap::new();
    params.insert("offset".to_string(), offset);
    params.insert("sigma".to_string(), sigma);
    Some(params)
}

fn make_request(cli: &Cli, data_id: &str) -> DoubleMaRequest {
    let mut strategy = StrategyConfig::default();
    strategy.long_only = cli.long_only;
    strategy.allow_flip = cli.allow_flip;

    DoubleMaRequest {
        backend: backend_from_arg(cli.backend.clone()),
        data_id: data_id.to_string(),
        fast_range: (
            cli.fast_period.start,
            cli.fast_period.end,
            cli.fast_period.step,
        ),
        slow_range: (
            cli.slow_period.start,
            cli.slow_period.end,
            cli.slow_period.step,
        ),
        fast_ma_types: vec![cli.fast_ma.clone()],
        slow_ma_types: vec![cli.slow_ma.clone()],
        ma_source: cli.ma_source.clone(),
        export_all_csv_path: None,
        fast_ma_params: alma_params(cli.alma_offset, cli.alma_sigma, &cli.fast_ma),
        slow_ma_params: alma_params(cli.alma_offset, cli.alma_sigma, &cli.slow_ma),
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy,
        halftrend_filter: None,
        objective: objective_from_arg(cli.objective.clone()),
        mode: OptimizationMode::Grid,
        top_k: Some(cli.top_k),
        include_all: Some(false),
        heatmap_bins: Some(cli.heatmap_bins),
        constraints: None,
        vdps: None,
    }
}

fn run_trial(
    req: &DoubleMaRequest,
    state: &AppState,
    data_id: &str,
    trial_index: usize,
) -> Result<TrialArtifact, String> {
    let candles = state.get_candles(data_id)?;
    let cancel = AtomicBool::new(false);
    let started = Instant::now();
    let result = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req.clone(),
        candles.as_ref(),
        &cancel,
        None,
    )?;
    Ok(TrialArtifact {
        trial_index,
        runtime_ms_wall: started.elapsed().as_secs_f64() * 1000.0,
        result,
    })
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let cache_dir = cli
        .cache_dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("ta_desktop_demo_faceoff_cache"));
    std::fs::create_dir_all(&cache_dir).map_err(|e| {
        format!(
            "failed to create cache dir {}: {e}",
            cache_dir.to_string_lossy()
        )
    })?;

    let state = AppState::default();

    let load_started = Instant::now();
    let load = state.load_price_data(
        cli.csv
            .to_str()
            .ok_or_else(|| "invalid csv path".to_string())?,
        &cache_dir,
    )?;
    let load_ms = load_started.elapsed().as_secs_f64() * 1000.0;

    let (data_id, resample_ms) = if let Some(raw_tf) = cli.timeframe.as_deref() {
        let started = Instant::now();
        let derived = state.resample_loaded_data(&load.id, raw_tf)?;
        (derived.id, Some(started.elapsed().as_secs_f64() * 1000.0))
    } else {
        (load.id.clone(), None)
    };

    let version = state.dataset_version(&data_id)?;
    let bars = state.get_candles(&data_id)?.close.len();
    let req = make_request(&cli, &data_id);

    let mut warmup_trials = Vec::with_capacity(cli.warmup_trials);
    for i in 0..cli.warmup_trials {
        warmup_trials.push(run_trial(&req, &state, &data_id, i + 1)?);
    }

    let mut measured_trials = Vec::with_capacity(cli.measured_trials);
    for i in 0..cli.measured_trials {
        measured_trials.push(run_trial(&req, &state, &data_id, i + 1)?);
    }

    let artifact = AppPathFaceoffArtifact {
        tool: "vectorgrid",
        mode: "app-path",
        backend: format!("{:?}", backend_from_arg(cli.backend)),
        dataset: FaceoffDatasetArtifact {
            csv_path: cli.csv.to_string_lossy().to_string(),
            cache_dir: cache_dir.to_string_lossy().to_string(),
            load_ms,
            resample_ms,
            data_id,
            version,
            bars,
        },
        warmup_trials,
        measured_trials,
        generated_at_utc: chrono::Utc::now().to_rfc3339(),
    };

    if let Some(parent) = cli.output_json.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create output dir {}: {e}",
                parent.to_string_lossy()
            )
        })?;
    }
    let json = serde_json::to_string_pretty(&artifact).map_err(|e| e.to_string())?;
    std::fs::write(&cli.output_json, json).map_err(|e| {
        format!(
            "failed to write output json {}: {e}",
            cli.output_json.to_string_lossy()
        )
    })?;
    Ok(())
}
