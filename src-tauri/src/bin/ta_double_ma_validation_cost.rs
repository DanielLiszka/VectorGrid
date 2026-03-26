use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use ta_app_core::composite_validate::{
    ReoptWalkForwardConfigOptional, SpaBenchmarkKind, SpaConfigOptional,
};
use ta_app_core::{
    read_candles_from_csv_compat, Backend, CrossTickerConfig, CrossTickerDataset, CscvPboConfig,
    DoubleMaRequest, DoubleMaValidationParams, DoubleMaValidationRequest, IterativeSimConfig,
    PerturbConfig, TradeBootstrapConfig, ValidationArtifactContext, ValidationEngine,
    WalkForwardConfigOptional,
};
use ta_desktop_demo_app::commands::apply_cross_ticker_validation_double_ma_headless;
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
        let parts: Vec<&str> = s.trim().split(':').collect();
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
#[command(name = "ta_double_ma_validation_cost")]
#[command(about = "Measure Double-MA validation module costs against optimization")]
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

    #[arg(long, value_enum, default_value_t = BackendArg::Cpu)]
    backend: BackendArg,

    #[arg(long, default_value = "alma")]
    fast_ma: String,

    #[arg(long, default_value = "alma")]
    slow_ma: String,

    #[arg(long, default_value = "close")]
    ma_source: String,

    #[arg(long, default_value = "9:21:3")]
    fast_period: U32Range,

    #[arg(long, default_value = "30:90:10")]
    slow_period: U32Range,

    #[arg(long, value_enum, default_value_t = ObjectiveArg::Sharpe)]
    objective: ObjectiveArg,

    #[arg(long, default_value_t = 24)]
    optimize_top_k: usize,

    #[arg(long, default_value_t = 12)]
    validate_max_candidates: usize,

    #[arg(long, default_value_t = 64)]
    heatmap_bins: u16,

    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(long, default_value_t = 0.85)]
    alma_offset: f64,

    #[arg(long, default_value_t = 6.0)]
    alma_sigma: f64,

    #[arg(long, default_value_t = 12_000)]
    wf_window_bars: usize,

    #[arg(long, default_value_t = 3_000)]
    wf_step_bars: usize,

    #[arg(long, default_value_t = 8)]
    reopt_top_n_candidates: usize,

    #[arg(long, default_value_t = 8_000)]
    reopt_window_bars: usize,

    #[arg(long, default_value_t = 2_000)]
    reopt_step_bars: usize,

    #[arg(long, default_value_t = 8)]
    spa_slices: usize,

    #[arg(long, default_value_t = 128)]
    spa_bootstrap_samples: usize,

    #[arg(long, default_value_t = 3)]
    spa_block_length: usize,

    #[arg(long, default_value_t = 32)]
    perturb_iterations: usize,

    #[arg(long, default_value_t = -0.10)]
    perturb_pct_min: f64,

    #[arg(long, default_value_t = 0.10)]
    perturb_pct_max: f64,

    #[arg(long, default_value_t = 0.05)]
    perturb_flip_bool_prob: f64,

    #[arg(long, default_value_t = 128)]
    bootstrap_sims: usize,

    #[arg(long)]
    bootstrap_stationary_mean_block_len: Option<usize>,

    #[arg(long, value_enum, default_value_t = BootstrapModeArg::Compare)]
    bootstrap_mode: BootstrapModeArg,

    #[arg(long, default_value_t = 8)]
    cscv_slices: usize,

    #[arg(long, default_value_t = 2_048)]
    cscv_max_combinations: usize,

    #[arg(long, default_value_t = 0.0)]
    cscv_threshold: f64,

    #[arg(long, default_value_t = 4)]
    cross_max_datasets: usize,

    #[arg(long, default_value_t = 1)]
    repeats: usize,
}

#[derive(Clone)]
struct ValidationCaseSpec {
    label: &'static str,
    walk_forward: bool,
    cscv_pbo: bool,
    spa: bool,
    reopt_walk_forward: bool,
    perturb: bool,
    trade_bootstrap: bool,
    cross_ticker_preserved_stack: bool,
}

#[derive(Serialize)]
struct CostReport {
    csv_path: String,
    backend_requested: String,
    total_bars: usize,
    optimization: OptimizationSummary,
    validation_cases: Vec<ValidationCaseSummary>,
    module_feasibility: Vec<ModuleFeasibilitySummary>,
}

#[derive(Serialize)]
struct OptimizationSummary {
    wall_ms: u64,
    runtime_ms: u64,
    backend_used: String,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    top_candidates: usize,
}

#[derive(Serialize)]
struct ValidationCaseSummary {
    label: String,
    status: String,
    repeats: usize,
    avg_wall_ms: u64,
    avg_main_validation_wall_ms: u64,
    avg_cross_ticker_wall_ms: u64,
    avg_reported_runtime_ms: u64,
    versus_optimization_ratio: f64,
    cheapness_band: String,
    backend_used: Option<String>,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    candidate_count: Option<usize>,
    approved_count: Option<usize>,
    rejected_count: Option<usize>,
    run_verdict_approved_count: Option<usize>,
    run_verdict_total_count: Option<usize>,
    run_verdict_status: Option<String>,
    execution_profile: Option<ValidationExecutionProfileSummary>,
    error: Option<String>,
}

#[derive(Serialize)]
struct ModuleFeasibilitySummary {
    module: String,
    current_execution_mode: String,
    avg_wall_ms: Option<u64>,
    versus_optimization_ratio: Option<f64>,
    cheapness_band: Option<String>,
    gpu_native_feasible_now: bool,
    blocker: Option<String>,
    next_step: String,
}

#[derive(Serialize)]
struct ValidationExecutionProfileSummary {
    requested_candidates: usize,
    unique_candidates: usize,
    unique_fast_periods: usize,
    unique_slow_periods: usize,
    evaluated_pairs_on_device: Option<usize>,
    expansion_ratio: Option<f64>,
    avg_gpu_range_eval_calls: Option<u64>,
    avg_gpu_range_count: Option<u64>,
    avg_aux_finalist_analytics_wall_ms: Option<u64>,
    avg_cpu_tail_wall_ms: Option<u64>,
    avg_gpu_result_materialization_wall_ms: Option<u64>,
}

struct ValidationCaseAggregate {
    total_wall_ms: u128,
    total_main_wall_ms: u128,
    total_cross_wall_ms: u128,
    total_reported_runtime_ms: u128,
    repeats: usize,
    last_backend_used: Option<String>,
    last_backend_route: Option<String>,
    last_backend_fallback_reason: Option<String>,
    last_candidate_count: Option<usize>,
    last_approved_count: Option<usize>,
    last_rejected_count: Option<usize>,
    last_run_verdict_approved_count: Option<usize>,
    last_run_verdict_total_count: Option<usize>,
    last_run_verdict_status: Option<String>,
    last_requested_candidates: Option<usize>,
    last_unique_candidates: Option<usize>,
    last_unique_fast_periods: Option<usize>,
    last_unique_slow_periods: Option<usize>,
    last_evaluated_pairs_on_device: Option<usize>,
    last_expansion_ratio: Option<f64>,
    total_gpu_range_eval_calls: u128,
    gpu_range_eval_calls_samples: usize,
    total_gpu_range_count: u128,
    gpu_range_count_samples: usize,
    total_aux_finalist_analytics_wall_ms: u128,
    aux_finalist_analytics_wall_ms_samples: usize,
    total_cpu_tail_wall_ms: u128,
    cpu_tail_wall_ms_samples: usize,
    total_gpu_result_materialization_wall_ms: u128,
    gpu_result_materialization_wall_ms_samples: usize,
    last_error: Option<String>,
}

fn validation_cases(include_cross: bool) -> Vec<ValidationCaseSpec> {
    let mut cases = vec![
        ValidationCaseSpec {
            label: "validation_base",
            walk_forward: false,
            cscv_pbo: false,
            spa: false,
            reopt_walk_forward: false,
            perturb: false,
            trade_bootstrap: false,
            cross_ticker_preserved_stack: false,
        },
        ValidationCaseSpec {
            label: "validation_walk_forward",
            walk_forward: true,
            cscv_pbo: false,
            spa: false,
            reopt_walk_forward: false,
            perturb: false,
            trade_bootstrap: false,
            cross_ticker_preserved_stack: false,
        },
        ValidationCaseSpec {
            label: "validation_cscv_pbo",
            walk_forward: false,
            cscv_pbo: true,
            spa: false,
            reopt_walk_forward: false,
            perturb: false,
            trade_bootstrap: false,
            cross_ticker_preserved_stack: false,
        },
        ValidationCaseSpec {
            label: "validation_spa",
            walk_forward: false,
            cscv_pbo: false,
            spa: true,
            reopt_walk_forward: false,
            perturb: false,
            trade_bootstrap: false,
            cross_ticker_preserved_stack: false,
        },
        ValidationCaseSpec {
            label: "validation_reopt_walk_forward",
            walk_forward: false,
            cscv_pbo: false,
            spa: false,
            reopt_walk_forward: true,
            perturb: false,
            trade_bootstrap: false,
            cross_ticker_preserved_stack: false,
        },
        ValidationCaseSpec {
            label: "validation_perturb",
            walk_forward: false,
            cscv_pbo: false,
            spa: false,
            reopt_walk_forward: false,
            perturb: true,
            trade_bootstrap: false,
            cross_ticker_preserved_stack: false,
        },
        ValidationCaseSpec {
            label: "validation_trade_bootstrap",
            walk_forward: false,
            cscv_pbo: false,
            spa: false,
            reopt_walk_forward: false,
            perturb: false,
            trade_bootstrap: true,
            cross_ticker_preserved_stack: false,
        },
        ValidationCaseSpec {
            label: "validation_full_stack_no_cross",
            walk_forward: true,
            cscv_pbo: true,
            spa: true,
            reopt_walk_forward: true,
            perturb: true,
            trade_bootstrap: true,
            cross_ticker_preserved_stack: false,
        },
    ];
    if include_cross {
        cases.push(ValidationCaseSpec {
            label: "validation_cross_ticker_preserved_stack",
            walk_forward: false,
            cscv_pbo: false,
            spa: false,
            reopt_walk_forward: false,
            perturb: false,
            trade_bootstrap: false,
            cross_ticker_preserved_stack: true,
        });
        cases.push(ValidationCaseSpec {
            label: "validation_full_stack_plus_cross_ticker_preserved_stack",
            walk_forward: true,
            cscv_pbo: true,
            spa: true,
            reopt_walk_forward: true,
            perturb: true,
            trade_bootstrap: true,
            cross_ticker_preserved_stack: true,
        });
    }
    cases
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

fn backend_used_label(backend: ta_app_core::BackendUsed) -> &'static str {
    match backend {
        ta_app_core::BackendUsed::Cpu => "Cpu",
        ta_app_core::BackendUsed::Gpu => "Gpu",
    }
}

fn requested_backend_label(backend: &Backend) -> &'static str {
    match backend {
        Backend::CpuOnly => "CpuOnly",
        Backend::GpuKernel { .. } => "GpuKernel",
        Backend::GpuHybridKernel { .. } => "GpuHybridKernel",
        Backend::GpuOnly { .. } => "GpuOnly",
        Backend::Auto { .. } => "Auto",
    }
}

fn to_cross_mode(arg: CrossModeArg) -> ta_app_core::CrossTickerMode {
    match arg {
        CrossModeArg::Lightweight => ta_app_core::CrossTickerMode::Lightweight,
        CrossModeArg::PreservedFinalists => ta_app_core::CrossTickerMode::PreservedFinalists,
    }
}

fn build_validation_candidates(
    top: &[(
        ta_app_core::DoubleMaParamsResolved,
        ta_strategies::double_ma::Metrics,
    )],
    max_candidates: usize,
    strategy: &StrategyConfig,
) -> Vec<DoubleMaValidationParams> {
    let mut out = Vec::new();
    let mut seen: HashSet<(String, u16, String, u16)> = HashSet::new();

    for (p, _) in top {
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
            halftrend_filter: None,
        });
        if out.len() >= max_candidates {
            break;
        }
    }

    out
}

fn make_validation_request(
    cli: &Cli,
    case: &ValidationCaseSpec,
    backend: Backend,
    candidates: Vec<DoubleMaValidationParams>,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    validation_context: ValidationArtifactContext,
) -> DoubleMaValidationRequest {
    DoubleMaValidationRequest {
        data_id: cli
            .csv
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("dataset")
            .to_string(),
        backend,
        ma_source: cli.ma_source.clone(),
        fast_ma_params,
        slow_ma_params,
        candidates,
        walk_forward: case.walk_forward.then_some(WalkForwardConfigOptional {
            window_bars: cli.wf_window_bars,
            step_bars: cli.wf_step_bars,
        }),
        reopt_walk_forward: case
            .reopt_walk_forward
            .then_some(ReoptWalkForwardConfigOptional {
                window_bars: cli.reopt_window_bars,
                step_bars: cli.reopt_step_bars,
                top_n_candidates: cli.reopt_top_n_candidates,
            }),
        spa: case.spa.then_some(SpaConfigOptional {
            slices: cli.spa_slices,
            bootstrap_samples: cli.spa_bootstrap_samples,
            block_length: cli.spa_block_length,
            benchmark: SpaBenchmarkKind::BuyAndHold,
        }),
        perturb: case.perturb.then_some(PerturbConfig {
            iterations: cli.perturb_iterations,
            pct_min: cli.perturb_pct_min,
            pct_max: cli.perturb_pct_max,
            flip_bool_prob: cli.perturb_flip_bool_prob,
        }),
        trade_bootstrap: case.trade_bootstrap.then_some(TradeBootstrapConfig {
            simulations: cli.bootstrap_sims,
            stationary_mean_block_len: match cli.bootstrap_mode {
                BootstrapModeArg::Compare => cli.bootstrap_stationary_mean_block_len,
                BootstrapModeArg::IidOnly => Some(0),
            },
        }),
        cross_ticker: None,
        cscv_pbo: case.cscv_pbo.then_some(CscvPboConfig {
            slices: cli.cscv_slices,
            max_combinations: cli.cscv_max_combinations,
            threshold: cli.cscv_threshold,
        }),
        seed: cli.seed,
        max_candidates: cli.validate_max_candidates,
        validation_engine: ValidationEngine::Vectorized,
        iterative: IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: Some(validation_context),
    }
}

fn cheapness_band(ratio: f64) -> String {
    if ratio <= 0.25 {
        "cheap".to_string()
    } else if ratio <= 1.0 {
        "comparable".to_string()
    } else if ratio <= 3.0 {
        "expensive".to_string()
    } else {
        "very_expensive".to_string()
    }
}

fn summarize_aux_module_feasibility(
    summaries: &[ValidationCaseSummary],
) -> Vec<ModuleFeasibilitySummary> {
    let perturb_case = summaries
        .iter()
        .find(|case| case.label == "validation_perturb");
    let bootstrap_case = summaries
        .iter()
        .find(|case| case.label == "validation_trade_bootstrap");

    vec![
        ModuleFeasibilitySummary {
            module: "perturb".to_string(),
            current_execution_mode: "GPU main route + CPU tail".to_string(),
            avg_wall_ms: perturb_case.map(|case| case.avg_wall_ms),
            versus_optimization_ratio: perturb_case.map(|case| case.versus_optimization_ratio),
            cheapness_band: perturb_case.map(|case| case.cheapness_band.clone()),
            gpu_native_feasible_now: false,
            blocker: Some(
                "Current fixed-candidate GPU batching requires a uniform StrategyConfig, while perturb mutates StrategyConfig per iteration.".to_string(),
            ),
            next_step:
                "Add a GPU validation path that can batch non-uniform perturbed candidates or explicitly keep the CPU tail and document the measured cost."
                    .to_string(),
        },
        ModuleFeasibilitySummary {
            module: "trade_bootstrap".to_string(),
            current_execution_mode: "GPU main route + CPU tail".to_string(),
            avg_wall_ms: bootstrap_case.map(|case| case.avg_wall_ms),
            versus_optimization_ratio: bootstrap_case.map(|case| case.versus_optimization_ratio),
            cheapness_band: bootstrap_case.map(|case| case.cheapness_band.clone()),
            gpu_native_feasible_now: false,
            blocker: Some(
                "Current fixed-candidate GPU validation returns aggregate metrics only and does not expose the closed-trade sequence required by bootstrap resampling.".to_string(),
            ),
            next_step:
                "Either extend the GPU validation backend to surface trade sequences or keep the CPU bootstrap tail if the measured cost remains acceptable."
                    .to_string(),
        },
    ]
}

fn merge_case_result(
    agg: &mut ValidationCaseAggregate,
    wall_ms: u64,
    main_wall_ms: u64,
    cross_wall_ms: u64,
    reported_runtime_ms: u64,
    backend_used: Option<String>,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    candidate_count: Option<usize>,
    approved_count: Option<usize>,
    rejected_count: Option<usize>,
    run_verdict_approved_count: Option<usize>,
    run_verdict_total_count: Option<usize>,
    run_verdict_status: Option<String>,
    execution_profile: Option<&ta_app_core::ValidationExecutionProfile>,
    error: Option<String>,
) {
    agg.total_wall_ms += u128::from(wall_ms);
    agg.total_main_wall_ms += u128::from(main_wall_ms);
    agg.total_cross_wall_ms += u128::from(cross_wall_ms);
    agg.total_reported_runtime_ms += u128::from(reported_runtime_ms);
    agg.last_backend_used = backend_used;
    agg.last_backend_route = backend_route;
    agg.last_backend_fallback_reason = backend_fallback_reason;
    agg.last_candidate_count = candidate_count;
    agg.last_approved_count = approved_count;
    agg.last_rejected_count = rejected_count;
    agg.last_run_verdict_approved_count = run_verdict_approved_count;
    agg.last_run_verdict_total_count = run_verdict_total_count;
    agg.last_run_verdict_status = run_verdict_status;
    if let Some(profile) = execution_profile {
        agg.last_requested_candidates = Some(profile.requested_candidates);
        agg.last_unique_candidates = Some(profile.unique_candidates);
        agg.last_unique_fast_periods = Some(profile.unique_fast_periods);
        agg.last_unique_slow_periods = Some(profile.unique_slow_periods);
        agg.last_evaluated_pairs_on_device = profile.evaluated_pairs_on_device;
        agg.last_expansion_ratio = profile.expansion_ratio;
        if let Some(value) = profile.gpu_range_eval_calls {
            agg.total_gpu_range_eval_calls += value as u128;
            agg.gpu_range_eval_calls_samples += 1;
        }
        if let Some(value) = profile.gpu_range_count {
            agg.total_gpu_range_count += value as u128;
            agg.gpu_range_count_samples += 1;
        }
        if let Some(value) = profile.aux_finalist_analytics_wall_ms {
            agg.total_aux_finalist_analytics_wall_ms += value as u128;
            agg.aux_finalist_analytics_wall_ms_samples += 1;
        }
        if let Some(value) = profile.cpu_tail_wall_ms {
            agg.total_cpu_tail_wall_ms += value as u128;
            agg.cpu_tail_wall_ms_samples += 1;
        }
        if let Some(value) = profile.gpu_result_materialization_wall_ms {
            agg.total_gpu_result_materialization_wall_ms += value as u128;
            agg.gpu_result_materialization_wall_ms_samples += 1;
        }
    }
    agg.last_error = error;
}

fn make_cross_config(cli: &Cli) -> Option<CrossTickerConfig> {
    if cli.cross_csv.is_empty() || cli.cross_max_datasets == 0 {
        return None;
    }
    Some(CrossTickerConfig {
        datasets: cli
            .cross_csv
            .iter()
            .map(|p| CrossTickerDataset {
                csv_path: p.to_string_lossy().to_string(),
                label: String::new(),
            })
            .collect(),
        max_datasets: cli.cross_max_datasets,
        mode: to_cross_mode(cli.cross_mode.clone()),
        gate: ta_app_core::CrossTickerGateConfig::default(),
    })
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
    if cli.repeats == 0 {
        return Err("repeats must be > 0".to_string());
    }
    if cli.perturb_pct_min > cli.perturb_pct_max {
        return Err("perturb_pct_min must be <= perturb_pct_max".to_string());
    }

    let backend = to_backend(cli.backend.clone());
    let objective = to_objective(cli.objective.clone());
    let fast_ma_params = alma_params_if_needed(&cli.fast_ma, cli.alma_offset, cli.alma_sigma);
    let slow_ma_params = alma_params_if_needed(&cli.slow_ma, cli.alma_offset, cli.alma_sigma);
    let strategy = StrategyConfig::default();
    let cancel = AtomicBool::new(false);

    println!("Loading candles: {}", cli.csv.to_string_lossy());
    let candles = read_candles_from_csv_compat(
        cli.csv
            .to_str()
            .ok_or_else(|| "invalid csv path".to_string())?,
    )?;
    println!("Loaded {} bars", candles.close.len());

    let opt_req = DoubleMaRequest {
        backend: backend.clone(),
        data_id: cli.csv.to_string_lossy().to_string(),
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
        halftrend_filter: None,
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

        hurdle_tau: None,
    };

    println!("Running optimization baseline...");
    let opt_started = Instant::now();
    let opt_res = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        opt_req, &candles, &cancel, None,
    )?;
    let opt_backend_used = backend_used_label(opt_res.backend_used).to_string();
    let opt_wall_ms = opt_started.elapsed().as_millis() as u64;
    println!(
        "Optimization: backend_used={} route={} fallback={} wall_ms={} runtime_ms={} top={}",
        opt_backend_used,
        opt_res.backend_route.as_deref().unwrap_or("<none>"),
        opt_res
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("<none>"),
        opt_wall_ms,
        opt_res.runtime_ms,
        opt_res.top.len()
    );

    let candidates =
        build_validation_candidates(&opt_res.top, cli.validate_max_candidates, &strategy);
    if candidates.is_empty() {
        return Err("no candidates available for validation".to_string());
    }
    let validation_context = ValidationArtifactContext {
        strategy_kind: "DoubleMa".to_string(),
        csv_path: Some(cli.csv.to_string_lossy().to_string()),
        timeframe: None,
        label: None,
    };
    let cross_cfg = make_cross_config(&cli);
    std::fs::create_dir_all(&cli.cache_dir).map_err(|e| e.to_string())?;

    let mut summaries = Vec::new();
    for case in validation_cases(cross_cfg.is_some()) {
        println!("Running case: {}", case.label);
        let mut agg = ValidationCaseAggregate {
            total_wall_ms: 0,
            total_main_wall_ms: 0,
            total_cross_wall_ms: 0,
            total_reported_runtime_ms: 0,
            repeats: cli.repeats,
            last_backend_used: None,
            last_backend_route: None,
            last_backend_fallback_reason: None,
            last_candidate_count: None,
            last_approved_count: None,
            last_rejected_count: None,
            last_run_verdict_approved_count: None,
            last_run_verdict_total_count: None,
            last_run_verdict_status: None,
            last_requested_candidates: None,
            last_unique_candidates: None,
            last_unique_fast_periods: None,
            last_unique_slow_periods: None,
            last_evaluated_pairs_on_device: None,
            last_expansion_ratio: None,
            total_gpu_range_eval_calls: 0,
            gpu_range_eval_calls_samples: 0,
            total_gpu_range_count: 0,
            gpu_range_count_samples: 0,
            total_aux_finalist_analytics_wall_ms: 0,
            aux_finalist_analytics_wall_ms_samples: 0,
            total_cpu_tail_wall_ms: 0,
            cpu_tail_wall_ms_samples: 0,
            total_gpu_result_materialization_wall_ms: 0,
            gpu_result_materialization_wall_ms_samples: 0,
            last_error: None,
        };

        for _ in 0..cli.repeats {
            let req = make_validation_request(
                &cli,
                &case,
                backend.clone(),
                candidates.clone(),
                fast_ma_params.clone(),
                slow_ma_params.clone(),
                validation_context.clone(),
            );
            let cross_walk_forward = req.walk_forward.clone();
            let cross_reopt_walk_forward = req.reopt_walk_forward.clone();
            let cross_spa = req.spa.clone();
            let cross_perturb = req.perturb.clone();
            let cross_trade_bootstrap = req.trade_bootstrap.clone();
            let cross_cscv_pbo = req.cscv_pbo.clone();
            let cross_min_psr = req.min_psr;
            let cross_min_dsr = req.min_dsr;
            let cross_min_cscv_oos_rank_frac = req.min_cscv_oos_rank_frac;
            let cross_min_cscv_oos_metric = req.min_cscv_oos_metric;
            let cross_max_pbo = req.max_pbo;

            let wall_started = Instant::now();
            let main_started = Instant::now();
            let validation = ta_app_core::run_double_ma_validation_blocking_with_candles(
                req, &candles, &cancel, None,
            );
            let main_wall_ms = main_started.elapsed().as_millis() as u64;

            match validation {
                Ok(mut res) => {
                    let mut cross_wall_ms = 0u64;
                    if case.cross_ticker_preserved_stack {
                        if let Some(cfg) = cross_cfg.clone() {
                            let cross_started = Instant::now();
                            let cross_result = apply_cross_ticker_validation_double_ma_headless(
                                &mut res,
                                cfg,
                                &candles,
                                backend.clone(),
                                &cli.ma_source,
                                fast_ma_params.clone(),
                                slow_ma_params.clone(),
                                cross_walk_forward.clone(),
                                cross_reopt_walk_forward.clone(),
                                cross_spa.clone(),
                                cross_perturb.clone(),
                                cross_trade_bootstrap.clone(),
                                cross_cscv_pbo.clone(),
                                cli.seed,
                                &cli.cache_dir,
                                &cancel,
                                ValidationEngine::Vectorized,
                                IterativeSimConfig::default(),
                                cross_min_psr,
                                cross_min_dsr,
                                cross_min_cscv_oos_rank_frac,
                                cross_min_cscv_oos_metric,
                                cross_max_pbo,
                            );
                            cross_wall_ms = cross_started.elapsed().as_millis() as u64;
                            if let Err(err) = cross_result {
                                let wall_ms = wall_started.elapsed().as_millis() as u64;
                                merge_case_result(
                                    &mut agg,
                                    wall_ms,
                                    main_wall_ms,
                                    cross_wall_ms,
                                    res.runtime_ms,
                                    Some(backend_used_label(res.backend_used).to_string()),
                                    res.backend_route.clone(),
                                    res.backend_fallback_reason.clone(),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    res.execution_profile.as_ref(),
                                    Some(err),
                                );
                                continue;
                            }
                        }
                    }

                    let wall_ms = wall_started.elapsed().as_millis() as u64;
                    let candidate_count = res.candidates.len();
                    let approved_count = res
                        .candidates
                        .iter()
                        .filter(|cand| cand.validation_verdict.passed)
                        .count();
                    let rejected_count = candidate_count.saturating_sub(approved_count);
                    let (run_verdict_approved_count, run_verdict_total_count, run_verdict_status) =
                        if case.cross_ticker_preserved_stack {
                            (None, None, None)
                        } else {
                            (
                                Some(res.run_verdict.approved_candidate_count),
                                Some(res.run_verdict.candidates_tested),
                                Some(if res.run_verdict.approved {
                                    "approved".to_string()
                                } else if res.run_verdict.passed {
                                    "passed_not_approved".to_string()
                                } else {
                                    "rejected".to_string()
                                }),
                            )
                        };

                    merge_case_result(
                        &mut agg,
                        wall_ms,
                        main_wall_ms,
                        cross_wall_ms,
                        res.runtime_ms,
                        Some(backend_used_label(res.backend_used).to_string()),
                        res.backend_route.clone(),
                        res.backend_fallback_reason.clone(),
                        Some(candidate_count),
                        Some(approved_count),
                        Some(rejected_count),
                        run_verdict_approved_count,
                        run_verdict_total_count,
                        run_verdict_status,
                        res.execution_profile.as_ref(),
                        None,
                    );
                }
                Err(err) => {
                    let wall_ms = wall_started.elapsed().as_millis() as u64;
                    merge_case_result(
                        &mut agg,
                        wall_ms,
                        main_wall_ms,
                        0,
                        0,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some(err),
                    );
                }
            }
        }

        let avg_wall_ms = (agg.total_wall_ms / agg.repeats as u128) as u64;
        let avg_main_wall_ms = (agg.total_main_wall_ms / agg.repeats as u128) as u64;
        let avg_cross_wall_ms = (agg.total_cross_wall_ms / agg.repeats as u128) as u64;
        let avg_reported_runtime_ms = (agg.total_reported_runtime_ms / agg.repeats as u128) as u64;
        let ratio = if opt_wall_ms > 0 {
            avg_wall_ms as f64 / opt_wall_ms as f64
        } else {
            0.0
        };
        let avg_u64 = |total: u128, samples: usize| -> Option<u64> {
            if samples > 0 {
                Some((total / samples as u128) as u64)
            } else {
                None
            }
        };
        let execution_profile = agg.last_requested_candidates.map(|requested_candidates| {
            ValidationExecutionProfileSummary {
                requested_candidates,
                unique_candidates: agg.last_unique_candidates.unwrap_or(requested_candidates),
                unique_fast_periods: agg.last_unique_fast_periods.unwrap_or(0),
                unique_slow_periods: agg.last_unique_slow_periods.unwrap_or(0),
                evaluated_pairs_on_device: agg.last_evaluated_pairs_on_device,
                expansion_ratio: agg.last_expansion_ratio,
                avg_gpu_range_eval_calls: avg_u64(
                    agg.total_gpu_range_eval_calls,
                    agg.gpu_range_eval_calls_samples,
                ),
                avg_gpu_range_count: avg_u64(
                    agg.total_gpu_range_count,
                    agg.gpu_range_count_samples,
                ),
                avg_aux_finalist_analytics_wall_ms: avg_u64(
                    agg.total_aux_finalist_analytics_wall_ms,
                    agg.aux_finalist_analytics_wall_ms_samples,
                ),
                avg_cpu_tail_wall_ms: avg_u64(
                    agg.total_cpu_tail_wall_ms,
                    agg.cpu_tail_wall_ms_samples,
                ),
                avg_gpu_result_materialization_wall_ms: avg_u64(
                    agg.total_gpu_result_materialization_wall_ms,
                    agg.gpu_result_materialization_wall_ms_samples,
                ),
            }
        });

        println!(
            "  status={} avg_wall_ms={} ratio_vs_opt={:.2} backend={} route={} fallback={} error={}",
            if agg.last_error.is_some() { "error" } else { "ok" },
            avg_wall_ms,
            ratio,
            agg.last_backend_used.as_deref().unwrap_or("<none>"),
            agg.last_backend_route.as_deref().unwrap_or("<none>"),
            agg.last_backend_fallback_reason.as_deref().unwrap_or("<none>"),
            agg.last_error.as_deref().unwrap_or("<none>")
        );

        summaries.push(ValidationCaseSummary {
            label: case.label.to_string(),
            status: if agg.last_error.is_some() {
                "error".to_string()
            } else {
                "ok".to_string()
            },
            repeats: agg.repeats,
            avg_wall_ms,
            avg_main_validation_wall_ms: avg_main_wall_ms,
            avg_cross_ticker_wall_ms: avg_cross_wall_ms,
            avg_reported_runtime_ms,
            versus_optimization_ratio: ratio,
            cheapness_band: cheapness_band(ratio),
            backend_used: agg.last_backend_used,
            backend_route: agg.last_backend_route,
            backend_fallback_reason: agg.last_backend_fallback_reason,
            candidate_count: agg.last_candidate_count,
            approved_count: agg.last_approved_count,
            rejected_count: agg.last_rejected_count,
            run_verdict_approved_count: agg.last_run_verdict_approved_count,
            run_verdict_total_count: agg.last_run_verdict_total_count,
            run_verdict_status: agg.last_run_verdict_status,
            execution_profile,
            error: agg.last_error,
        });
    }

    let module_feasibility = summarize_aux_module_feasibility(&summaries);
    let report = CostReport {
        csv_path: cli.csv.to_string_lossy().to_string(),
        backend_requested: requested_backend_label(&backend).to_string(),
        total_bars: candles.close.len(),
        optimization: OptimizationSummary {
            wall_ms: opt_wall_ms,
            runtime_ms: opt_res.runtime_ms,
            backend_used: opt_backend_used,
            backend_route: opt_res.backend_route,
            backend_fallback_reason: opt_res.backend_fallback_reason,
            top_candidates: candidates.len(),
        },
        validation_cases: summaries,
        module_feasibility,
    };

    let out = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    if let Some(path) = cli.out_json.as_ref() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, out).map_err(|e| e.to_string())?;
        println!("Saved measurement report: {}", path.to_string_lossy());
    } else {
        println!("{out}");
    }

    Ok(())
}
