use clap::{Parser, ValueEnum};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;

use ta_app_core::composite_validate::{
    ReoptWalkForwardConfigOptional, SpaBenchmarkKind, SpaConfigOptional,
};
use ta_app_core::{
    read_candles_from_csv_compat, Backend, CrossTickerConfig, CrossTickerDataset, CscvPboConfig,
    DoubleMaRequest, DoubleMaValidationParams, DoubleMaValidationRequest, PerturbConfig,
    TradeBootstrapConfig, ValidationArtifactContext, WalkForwardConfigOptional,
};
use ta_desktop_demo_app::commands::{
    apply_cross_ticker_validation_double_ma_headless, build_report_xlsx_bytes_headless,
};
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
    Auto,
    Cpu,
    GpuSweep,
    GpuHybridKernel,
    GpuKernel,
}

#[derive(ValueEnum, Clone, Debug)]
enum ObjectiveArg {
    Pnl,
    Sharpe,
    MaxDrawdown,
}

#[derive(ValueEnum, Clone, Debug)]
enum BootstrapModeArg {
    Compare,
    IidOnly,
}

#[derive(ValueEnum, Clone, Debug)]
enum CrossModeArg {
    Lightweight,
    PreservedFinalists,
}

#[derive(Parser, Debug)]
#[command(name = "ta_headless_validate")]
#[command(about = "Headless optimize -> validate -> XLSX pipeline for Double MA")]
struct Cli {
    #[arg(long)]
    csv: PathBuf,

    #[arg(long = "cross-csv")]
    cross_csv: Vec<PathBuf>,

    #[arg(long, value_enum, default_value_t = CrossModeArg::PreservedFinalists)]
    cross_mode: CrossModeArg,

    #[arg(long, default_value = "_write_test/headless_cache")]
    cache_dir: PathBuf,

    #[arg(long)]
    out_json: Option<PathBuf>,

    #[arg(long)]
    out_xlsx: Option<PathBuf>,

    #[arg(long, default_value_t = 24)]
    xlsx_max_rows: usize,

    #[arg(long, value_enum, default_value_t = BackendArg::Cpu)]
    backend: BackendArg,

    #[arg(long, default_value = "sma")]
    fast_ma: String,

    #[arg(long, default_value = "sma")]
    slow_ma: String,

    #[arg(long, default_value = "close")]
    ma_source: String,

    #[arg(long, default_value = "5:50:1")]
    fast_period: U32Range,

    #[arg(long, default_value = "20:200:5")]
    slow_period: U32Range,

    #[arg(long, value_enum, default_value_t = ObjectiveArg::Sharpe)]
    objective: ObjectiveArg,

    #[arg(long, default_value_t = 120)]
    optimize_top_k: usize,

    #[arg(long, default_value_t = 64)]
    heatmap_bins: u16,

    #[arg(long, default_value_t = 32)]
    validate_max_candidates: usize,

    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(long, default_value_t = false)]
    wf_enable: bool,

    #[arg(long, default_value_t = 20_000)]
    wf_window_bars: usize,

    #[arg(long, default_value_t = 20_000)]
    wf_step_bars: usize,

    #[arg(long, default_value_t = false)]
    reopt_wf_enable: bool,

    #[arg(long, default_value_t = 8)]
    reopt_top_n_candidates: usize,

    #[arg(long, default_value_t = 8_000)]
    reopt_window_bars: usize,

    #[arg(long, default_value_t = 2_000)]
    reopt_step_bars: usize,

    #[arg(long, default_value_t = false)]
    spa_enable: bool,

    #[arg(long, default_value_t = 8)]
    spa_slices: usize,

    #[arg(long, default_value_t = 128)]
    spa_bootstrap_samples: usize,

    #[arg(long, default_value_t = 3)]
    spa_block_length: usize,

    #[arg(long, default_value_t = false)]
    perturb_enable: bool,

    #[arg(long, default_value_t = 128)]
    perturb_iterations: usize,

    #[arg(long, default_value_t = -0.10)]
    perturb_pct_min: f64,

    #[arg(long, default_value_t = 0.10)]
    perturb_pct_max: f64,

    #[arg(long, default_value_t = 0.05)]
    perturb_flip_bool_prob: f64,

    #[arg(long)]
    min_psr: Option<f64>,

    #[arg(long)]
    min_dsr: Option<f64>,

    #[arg(long)]
    min_cscv_oos_rank_frac: Option<f64>,

    #[arg(long)]
    min_cscv_oos_metric: Option<f64>,

    #[arg(long)]
    max_pbo: Option<f64>,

    #[arg(long, default_value_t = false)]
    bootstrap_enable: bool,

    #[arg(long, default_value_t = 500)]
    bootstrap_sims: usize,

    #[arg(long)]
    bootstrap_stationary_mean_block_len: Option<usize>,

    #[arg(long, value_enum, default_value_t = BootstrapModeArg::Compare)]
    bootstrap_mode: BootstrapModeArg,

    #[arg(long, default_value_t = false)]
    cscv_enable: bool,

    #[arg(long, default_value_t = 16)]
    cscv_slices: usize,

    #[arg(long, default_value_t = 50_000)]
    cscv_max_combinations: usize,

    #[arg(long, default_value_t = 0.0)]
    cscv_threshold: f64,

    #[arg(long, default_value_t = 8)]
    cross_max_datasets: usize,

    #[arg(long, default_value_t = 0.85)]
    alma_offset: f64,

    #[arg(long, default_value_t = 6.0)]
    alma_sigma: f64,

    #[arg(long, default_value_t = 0)]
    halftrend_amplitude: u32,

    #[arg(long, default_value_t = 2.0)]
    halftrend_channel_deviation: f64,

    #[arg(long, default_value_t = 100)]
    halftrend_atr_period: u32,
}

fn to_backend(arg: BackendArg) -> Backend {
    match arg {
        BackendArg::Auto => Backend::Auto { device_id: 0 },
        BackendArg::Cpu => Backend::CpuOnly,
        BackendArg::GpuSweep => Backend::GpuOnly { device_id: 0 },
        BackendArg::GpuHybridKernel => Backend::GpuHybridKernel { device_id: 0 },
        BackendArg::GpuKernel => Backend::GpuKernel { device_id: 0 },
    }
}

fn to_objective(arg: ObjectiveArg) -> ObjectiveKind {
    match arg {
        ObjectiveArg::Pnl => ObjectiveKind::Pnl,
        ObjectiveArg::Sharpe => ObjectiveKind::Sharpe,
        ObjectiveArg::MaxDrawdown => ObjectiveKind::MaxDrawdown,
    }
}

fn to_cross_mode(arg: CrossModeArg) -> ta_app_core::CrossTickerMode {
    match arg {
        CrossModeArg::Lightweight => ta_app_core::CrossTickerMode::Lightweight,
        CrossModeArg::PreservedFinalists => ta_app_core::CrossTickerMode::PreservedFinalists,
    }
}

fn alma_params_if_needed(ma: &str, offset: f64, sigma: f64) -> Option<HashMap<String, f64>> {
    if ma.trim().eq_ignore_ascii_case("alma") {
        let mut p: HashMap<String, f64> = HashMap::new();
        p.insert("offset".to_string(), offset);
        p.insert("sigma".to_string(), sigma);
        Some(p)
    } else {
        None
    }
}

fn report_ts_path_fragment(ts: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.format("%Y%m%d_%H%M%S").to_string())
        .unwrap_or_else(|_| {
            ts.chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                .collect()
        })
}

fn make_default_xlsx_path(csv_path: &Path, report_ts: &str) -> PathBuf {
    let stem = csv_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("dataset");
    let ts = report_ts_path_fragment(report_ts);
    PathBuf::from("_write_test")
        .join("headless_reports")
        .join(format!("{stem}_report_{ts}.xlsx"))
}

fn build_validation_candidates(
    top: &[(
        ta_app_core::DoubleMaParamsResolved,
        ta_strategies::double_ma::Metrics,
    )],
    max_candidates: usize,
    strategy: &StrategyConfig,
    halftrend_filter: &Option<ta_app_core::double_ma::HalfTrendRegimeFilter>,
) -> Vec<DoubleMaValidationParams> {
    let mut out: Vec<DoubleMaValidationParams> = Vec::new();
    let mut seen: HashSet<(String, u16, String, u16)> = HashSet::new();

    for (p, _) in top.iter() {
        let key = (
            p.fast_ma_type.clone(),
            p.fast_len,
            p.slow_ma_type.clone(),
            p.slow_len,
        );
        if !seen.insert(key) {
            continue;
        }
        out.push(DoubleMaValidationParams {
            fast_ma_type: p.fast_ma_type.clone(),
            slow_ma_type: p.slow_ma_type.clone(),
            fast_len: p.fast_len,
            slow_len: p.slow_len,
            fast_ma_params: p.fast_ma_params.clone(),
            slow_ma_params: p.slow_ma_params.clone(),
            strategy: strategy.clone(),
            halftrend_filter: halftrend_filter.clone(),
        });
        if out.len() >= max_candidates {
            break;
        }
    }
    out
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();

    if !cli.csv.exists() {
        return Err(format!(
            "csv file does not exist: {}",
            cli.csv.to_string_lossy()
        ));
    }
    if cli.validate_max_candidates == 0 {
        return Err("validate_max_candidates must be > 0".to_string());
    }
    if cli.perturb_enable && cli.perturb_pct_min > cli.perturb_pct_max {
        return Err("perturb_pct_min must be <= perturb_pct_max".to_string());
    }

    let backend = to_backend(cli.backend.clone());
    let objective = to_objective(cli.objective);

    let fast_ma_params = alma_params_if_needed(&cli.fast_ma, cli.alma_offset, cli.alma_sigma);
    let slow_ma_params = alma_params_if_needed(&cli.slow_ma, cli.alma_offset, cli.alma_sigma);

    let halftrend_filter = if cli.halftrend_amplitude == 0 {
        None
    } else {
        if cli.halftrend_amplitude > u16::MAX as u32 {
            return Err("halftrend_amplitude exceeds u16::MAX".to_string());
        }
        if cli.halftrend_atr_period == 0 || cli.halftrend_atr_period > u16::MAX as u32 {
            return Err("halftrend_atr_period must be in [1, u16::MAX]".to_string());
        }
        Some(ta_app_core::double_ma::HalfTrendRegimeFilter {
            amplitude: cli.halftrend_amplitude as u16,
            channel_deviation: cli.halftrend_channel_deviation,
            atr_period: cli.halftrend_atr_period as u16,
        })
    };

    let csv_path = cli
        .csv
        .to_str()
        .ok_or_else(|| "invalid csv path".to_string())?;
    println!("Loading candles: {csv_path}");
    let candles = read_candles_from_csv_compat(csv_path)?;
    println!("Loaded {} bars", candles.close.len());

    let strategy = StrategyConfig::default();
    let req = DoubleMaRequest {
        backend,
        data_id: csv_path.to_string(),
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
        fast_ma_params: fast_ma_params.clone(),
        slow_ma_params: slow_ma_params.clone(),
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy: strategy.clone(),
        halftrend_filter: halftrend_filter.clone(),
        objective,
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(cli.optimize_top_k),
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
    };

    let cancel = AtomicBool::new(false);
    println!("Running optimization...");
    let opt_res = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req, &candles, &cancel, None,
    )?;
    println!(
        "Optimization complete: backend={:?} combos={} top={}",
        opt_res.backend_used,
        opt_res.num_combos,
        opt_res.top.len()
    );

    let candidates = build_validation_candidates(
        &opt_res.top,
        cli.validate_max_candidates,
        &strategy,
        &halftrend_filter,
    );
    if candidates.is_empty() {
        return Err("no candidates available for validation".to_string());
    }

    let walk_forward = if cli.wf_enable {
        Some(WalkForwardConfigOptional {
            window_bars: cli.wf_window_bars,
            step_bars: cli.wf_step_bars,
        })
    } else {
        None
    };

    let reopt_walk_forward = if cli.reopt_wf_enable {
        Some(ReoptWalkForwardConfigOptional {
            window_bars: cli.reopt_window_bars,
            step_bars: cli.reopt_step_bars,
            top_n_candidates: cli.reopt_top_n_candidates,
        })
    } else {
        None
    };

    let spa = if cli.spa_enable {
        Some(SpaConfigOptional {
            slices: cli.spa_slices,
            bootstrap_samples: cli.spa_bootstrap_samples,
            block_length: cli.spa_block_length,
            benchmark: SpaBenchmarkKind::BuyAndHold,
        })
    } else {
        None
    };

    let perturb = if cli.perturb_enable {
        Some(PerturbConfig {
            iterations: cli.perturb_iterations,
            pct_min: cli.perturb_pct_min,
            pct_max: cli.perturb_pct_max,
            flip_bool_prob: cli.perturb_flip_bool_prob,
        })
    } else {
        None
    };

    let trade_bootstrap = if cli.bootstrap_enable {
        Some(TradeBootstrapConfig {
            simulations: cli.bootstrap_sims,
            stationary_mean_block_len: match cli.bootstrap_mode {
                BootstrapModeArg::Compare => cli.bootstrap_stationary_mean_block_len,
                BootstrapModeArg::IidOnly => Some(0),
            },
        })
    } else {
        None
    };

    let cscv_pbo = if cli.cscv_enable {
        Some(CscvPboConfig {
            slices: cli.cscv_slices,
            max_combinations: cli.cscv_max_combinations,
            threshold: cli.cscv_threshold,
        })
    } else {
        None
    };

    let data_id = cli
        .csv
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("dataset")
        .to_string();
    let validation_engine = ta_app_core::ValidationEngine::Vectorized;
    let iterative_cfg = ta_app_core::IterativeSimConfig::default();
    let validation_backend = to_backend(cli.backend.clone());
    let strategy_kind = "DoubleMa".to_string();
    let validation_context = Some(ValidationArtifactContext {
        strategy_kind: strategy_kind.clone(),
        csv_path: Some(cli.csv.to_string_lossy().to_string()),
        timeframe: None,
        label: None,
    });

    let val_req = DoubleMaValidationRequest {
        data_id,
        backend: validation_backend.clone(),
        ma_source: cli.ma_source.clone(),
        fast_ma_params: fast_ma_params.clone(),
        slow_ma_params: slow_ma_params.clone(),
        candidates,
        walk_forward: walk_forward.clone(),
        reopt_walk_forward: reopt_walk_forward.clone(),
        spa: spa.clone(),
        perturb: perturb.clone(),
        trade_bootstrap: trade_bootstrap.clone(),
        cross_ticker: None,
        cscv_pbo: cscv_pbo.clone(),
        seed: cli.seed,
        max_candidates: cli.validate_max_candidates,
        validation_engine,
        iterative: iterative_cfg.clone(),
        min_psr: cli.min_psr,
        min_dsr: cli.min_dsr,
        min_cscv_oos_rank_frac: cli.min_cscv_oos_rank_frac,
        min_cscv_oos_metric: cli.min_cscv_oos_metric,
        max_pbo: cli.max_pbo,
        validation_context: validation_context.clone(),
    };

    println!("Running validation...");
    let mut val_res = ta_app_core::run_double_ma_validation_blocking_with_candles(
        val_req, &candles, &cancel, None,
    )?;

    if !cli.cross_csv.is_empty() && cli.cross_max_datasets > 0 {
        std::fs::create_dir_all(&cli.cache_dir).map_err(|e| e.to_string())?;
        let datasets: Vec<CrossTickerDataset> = cli
            .cross_csv
            .iter()
            .map(|p| CrossTickerDataset {
                csv_path: p.to_string_lossy().to_string(),
                label: "".to_string(),
            })
            .collect();
        let cross_cfg = CrossTickerConfig {
            datasets,
            max_datasets: cli.cross_max_datasets,
            mode: to_cross_mode(cli.cross_mode.clone()),
            gate: ta_app_core::CrossTickerGateConfig::default(),
        };
        println!(
            "Applying cross-ticker validation on {} dataset(s)...",
            cli.cross_csv.len()
        );
        apply_cross_ticker_validation_double_ma_headless(
            &mut val_res,
            cross_cfg,
            &candles,
            validation_backend,
            &cli.ma_source,
            fast_ma_params.clone(),
            slow_ma_params.clone(),
            walk_forward.clone(),
            reopt_walk_forward.clone(),
            spa.clone(),
            perturb.clone(),
            trade_bootstrap.clone(),
            cscv_pbo.clone(),
            cli.seed,
            &cli.cache_dir,
            &cancel,
            validation_engine,
            iterative_cfg.clone(),
            cli.min_psr,
            cli.min_dsr,
            cli.min_cscv_oos_rank_frac,
            cli.min_cscv_oos_metric,
            cli.max_pbo,
        )?;
    }

    println!(
        "Validation complete: candidates={} runtime_ms={}",
        val_res.candidates.len(),
        val_res.runtime_ms
    );

    let ts = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let (min_ts, max_ts) = candles
        .timestamp
        .iter()
        .fold((i64::MAX, i64::MIN), |(mn, mx), &v| (mn.min(v), mx.max(v)));
    let report = json!({
         "kind": "vectorgrid_run_artifact",
         "ts": ts,
         "run": {
             "req": {
                 "strategy_kind": strategy_kind,
                 "ma_source": cli.ma_source.clone()
             }
         },
         "data": { "meta": { "min_ts": min_ts, "max_ts": max_ts } },
         "optimization": { "res": serde_json::to_value(&opt_res).map_err(|e| e.to_string())? },
         "validation": {
             "req": {
                 "strategy_kind": "DoubleMa",
                 "ma_source": cli.ma_source.clone(),
                 "validation_context": validation_context,
                 "min_psr": cli.min_psr,
                 "min_dsr": cli.min_dsr,
                 "min_cscv_oos_rank_frac": cli.min_cscv_oos_rank_frac,
                 "min_cscv_oos_metric": cli.min_cscv_oos_metric,
                 "max_pbo": cli.max_pbo
             },
             "res": serde_json::to_value(&val_res).map_err(|e| e.to_string())?
         }
    });

    if let Some(out_json) = cli.out_json.as_ref() {
        if let Some(parent) = out_json.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let txt = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
        std::fs::write(out_json, txt).map_err(|e| e.to_string())?;
        println!("Saved JSON report: {}", out_json.to_string_lossy());
    }

    let out_xlsx = cli
        .out_xlsx
        .clone()
        .unwrap_or_else(|| make_default_xlsx_path(&cli.csv, &ts));
    if let Some(parent) = out_xlsx.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = build_report_xlsx_bytes_headless(&report, Some(cli.xlsx_max_rows.max(1)))?;
    std::fs::write(&out_xlsx, bytes).map_err(|e| e.to_string())?;
    println!("Saved XLSX report: {}", out_xlsx.to_string_lossy());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{make_default_xlsx_path, report_ts_path_fragment};
    use std::path::Path;

    #[test]
    fn report_ts_path_fragment_uses_report_timestamp_shape() {
        assert_eq!(
            report_ts_path_fragment("2026-03-18T16:04:05Z"),
            "20260318_160405"
        );
    }

    #[test]
    fn default_xlsx_path_derives_from_csv_stem_and_report_ts() {
        let path = make_default_xlsx_path(Path::new("C:/data/AAPL.csv"), "2026-03-18T16:04:05Z");
        assert_eq!(
            path,
            Path::new("_write_test")
                .join("headless_reports")
                .join("AAPL_report_20260318_160405.xlsx")
        );
    }
}
