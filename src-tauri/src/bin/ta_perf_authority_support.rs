use clap::Parser;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use ta_desktop_demo_app::commands::{
    alpaca_get_credential_status, build_report_xlsx_bytes_headless, build_validation_shortlist_csv,
    encode_dynamic_strategy_constant_bits, format_dynamic_strategy_ir_json,
    get_execution_strategy_support_matrix, get_gpu_status, get_local_paper_execution_snapshot,
    import_options_dataset_blocking_headless, list_repo_data_csvs, list_sample_csvs_for_bindings,
    live_execution_runner_status, load_run_artifact_hydration, polygon_live_candle_stream_status,
    polygon_stop_live_candle_stream, preview_dynamic_double_ma_strategy_ir_json,
    rebuild_artifact_catalog_blocking, reset_local_paper_broker, save_report_xlsx_from_artifact,
    save_run_artifact_blocking, update_artifact_tags_blocking,
    GetLocalPaperExecutionSnapshotRequest, OptionsDatasetImportRequest,
};
use ta_desktop_demo_app::data_cache;
use ta_desktop_demo_app::state::{AppState, BackgroundTaskState};
use ta_optimizer::ObjectiveKind;
use ta_strategies::double_ma::StrategyConfig;
use tauri::Manager;

#[derive(Parser, Debug)]
#[command(name = "ta_perf_authority_support")]
#[command(about = "Lightweight performance authority harness for non-credential support surfaces")]
struct Cli {
    #[arg(long)]
    out: PathBuf,

    #[arg(long)]
    work_dir: Option<PathBuf>,

    #[arg(long)]
    csv: Option<PathBuf>,

    #[arg(long, default_value_t = 20_000)]
    csv_rows: usize,

    #[arg(long, default_value_t = 480)]
    options_rows: usize,

    #[arg(long, default_value_t = 256)]
    report_candidates: usize,

    #[arg(long, default_value_t = 24)]
    artifact_count: usize,

    #[arg(long)]
    case: Vec<String>,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    cleanup_app_data: bool,
}

#[derive(Debug, Serialize)]
struct StepSummary {
    id: String,
    wall_ms: f64,
    metrics: Value,
}

#[derive(Debug, Serialize)]
struct CaseSummary {
    id: String,
    timing_boundary: String,
    status: String,
    wall_ms: f64,
    steps: Vec<StepSummary>,
    metrics: Value,
}

#[derive(Debug, Serialize)]
struct SupportPerfSummary {
    schema_version: u32,
    timing_boundary: String,
    timing_boundary_note: String,
    build_profile: String,
    cases_requested: Vec<String>,
    cases: Vec<CaseSummary>,
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    }
}

fn repo_root() -> Result<PathBuf, String> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "failed to resolve repo root".to_string())
}

fn default_work_dir() -> Result<PathBuf, String> {
    Ok(repo_root()?.join("_write_test/perf_authority_support"))
}

fn selected(cases: &[String], name: &str) -> bool {
    cases.is_empty()
        || cases
            .iter()
            .any(|case| case.trim().eq_ignore_ascii_case(name))
}

const SUPPORT_CASES: &[&str] = &[
    "data",
    "options-data",
    "live-status",
    "reports",
    "artifacts",
    "portfolio",
    "dynamic",
    "execution",
];

fn normalize_requested_cases(cases: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for case in cases {
        let normalized = case.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }
        if !SUPPORT_CASES.contains(&normalized.as_str()) {
            return Err(format!(
                "unknown support perf case '{case}'. Known cases: {}",
                SUPPORT_CASES.join(", ")
            ));
        }
        out.push(normalized);
    }
    Ok(out)
}

fn measure_step<T>(
    id: impl Into<String>,
    f: impl FnOnce() -> Result<(T, Value), String>,
) -> Result<(T, StepSummary), String> {
    let id = id.into();
    let started = Instant::now();
    let (value, metrics) = f()?;
    let wall_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok((
        value,
        StepSummary {
            id,
            wall_ms,
            metrics,
        },
    ))
}

fn write_synthetic_ohlcv_csv(path: &Path, rows: usize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut out = BufWriter::new(file);
    writeln!(out, "timestamp,open,high,low,close,volume").map_err(|e| e.to_string())?;
    let base_ts = 1_700_000_000_000i64;
    for i in 0..rows {
        let ts = base_ts + (i as i64) * 60_000;
        let trend = 100.0 + (i as f64) * 0.001;
        let wave = ((i as f64) * 0.037).sin() * 0.4;
        let close = trend + wave;
        let open = close - 0.05;
        let high = close + 0.15;
        let low = close - 0.20;
        let volume = 10_000.0 + (i % 97) as f64;
        writeln!(
            out,
            "{ts},{open:.6},{high:.6},{low:.6},{close:.6},{volume:.2}"
        )
        .map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

fn write_synthetic_options_csv(path: &Path, rows: usize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut out = BufWriter::new(file);
    writeln!(
        out,
        "root_symbol,contract_id,option_type,expiration_date,expiration_timestamp_ms,strike,multiplier,timestamp_ms,open,high,low,close,underlying_price,bid,ask,mid,volume,open_interest,quote_age_ms,session_id"
    )
    .map_err(|e| e.to_string())?;
    let base_ts = 1_700_000_000_000i64;
    let expiration_ts = base_ts + 14 * 24 * 60 * 60 * 1000;
    let contract_count = 6usize;
    for i in 0..rows {
        let c = i % contract_count;
        let side = if c % 2 == 0 { "call" } else { "put" };
        let strike = 98.0 + (c as f64) * 2.0;
        let ts = base_ts + (i / contract_count) as i64 * 60_000;
        let underlying = 100.0 + ((i / contract_count) as f64 * 0.011).sin();
        let intrinsic = if side == "call" {
            (underlying - strike).max(0.0)
        } else {
            (strike - underlying).max(0.0)
        };
        let close = (intrinsic + 1.0 + (c as f64) * 0.08).max(0.05);
        let bid = (close - 0.03).max(0.01);
        let ask = close + 0.03;
        let mid = (bid + ask) * 0.5;
        writeln!(
            out,
            "SPY,SPY_{side}_{c:03},{side},2023-11-28,{expiration_ts},{strike:.2},100,{ts},{:.4},{:.4},{:.4},{close:.4},{underlying:.4},{bid:.4},{ask:.4},{mid:.4},{},{},250,regular",
            close - 0.02,
            close + 0.04,
            (close - 0.05).max(0.01),
            10 + (i % 13),
            1000 + c
        )
        .map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

fn build_isolated_app() -> Result<tauri::App, String> {
    let unique = format!(
        "VectorGridPerfAuthority{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let mut context = tauri::generate_context!();
    context.config_mut().product_name = Some(unique.clone());
    context.config_mut().identifier = format!(
        "com.vectorgrid.perfauthority.{}",
        unique.to_ascii_lowercase()
    );
    tauri::Builder::default()
        .manage(AppState::default())
        .manage(BackgroundTaskState::default())
        .build(context)
        .map_err(|e| e.to_string())
}

fn validation_candidate(index: usize) -> Value {
    let fast = 5 + (index % 16) as u32;
    let slow = 24 + (index % 48) as u32;
    json!({
        "rank": index + 1,
        "candidate_id": format!("candidate-{index}"),
        "params": {
            "fast_ma_type": "sma",
            "fast_len": fast,
            "slow_ma_type": "ema",
            "slow_len": slow
        },
        "in_sample": {
            "pnl": 1.0 + index as f64 * 0.01,
            "sharpe": 1.2 + index as f64 * 0.001,
            "max_dd": 0.2 + index as f64 * 0.0001,
            "trades": 10 + index as u32,
            "exposure": 0.45,
            "net_exposure": 0.08
        },
        "walk_forward": { "sharpe_median": 1.0, "max_dd_worst": 0.4 },
        "perturb": { "pnl_worst": -0.2, "pnl_mean": 0.1, "pnl_positive_frac": 0.65 },
        "trade_bootstrap": { "prob_pnl_positive": 0.7, "pnl_p50": 0.2, "max_dd_p95": 0.7 },
        "cross_ticker": { "score": 0.4, "sharpe_median": 0.8, "max_dd_worst": 0.5 },
        "robust_score": 1.0 + index as f64 * 0.002,
        "robust_pareto": index % 3 == 0,
        "cross_ticker_pareto": index % 5 == 0,
        "validation_verdict": { "passed": true, "approved": index % 2 == 0 }
    })
}

fn validation_result(candidate_count: usize) -> Value {
    let candidates: Vec<Value> = (0..candidate_count).map(validation_candidate).collect();
    json!({
        "data_id": "support-perf",
        "candidates": candidates,
        "cscv_pbo": { "combinations": 8, "threshold": 0.0 }
    })
}

fn report_value(candidate_count: usize) -> Value {
    json!({
        "version": 1,
        "report_schema_version": 1,
        "kind": "vectorgrid_report",
        "ts": "2026-05-07T00:00:00Z",
        "validation": {
            "res": validation_result(candidate_count)
        }
    })
}

fn run_data_case(work_dir: &Path, cli: &Cli) -> Result<CaseSummary, String> {
    let started = Instant::now();
    let mut steps = Vec::new();
    let csv_path = cli
        .csv
        .clone()
        .unwrap_or_else(|| work_dir.join("data/synthetic_ohlcv.csv"));
    if cli.csv.is_none() {
        write_synthetic_ohlcv_csv(&csv_path, cli.csv_rows)?;
    }
    let app_local_data_dir = work_dir.join("app_local_data");
    let cache_dir = data_cache::app_data_cache_dir(&app_local_data_dir);

    let state = AppState::default();
    let (cold, step) = measure_step("load_csv_or_cache_cold", || {
        let response = state.load_price_data(&csv_path.to_string_lossy(), &cache_dir)?;
        Ok((
            response.clone(),
            json!({
                "rows": response.meta.len,
                "cache_hit": response.cache_hit,
                "load_source": format!("{:?}", response.load_source)
            }),
        ))
    })?;
    steps.push(step);

    let state = AppState::default();
    let (warm, step) = measure_step("load_binary_cache_warm", || {
        let response = state.load_price_data(&csv_path.to_string_lossy(), &cache_dir)?;
        Ok((
            response.clone(),
            json!({
                "rows": response.meta.len,
                "cache_hit": response.cache_hit,
                "load_source": format!("{:?}", response.load_source)
            }),
        ))
    })?;
    steps.push(step);

    let (_, step) = measure_step("resample_5m", || {
        let response = state.resample_loaded_data(&warm.id, "5m")?;
        Ok((
            (),
            json!({
                "rows": response.meta.len,
                "load_source": format!("{:?}", response.load_source)
            }),
        ))
    })?;
    steps.push(step);

    let (_, step) = measure_step("list_sample_csvs", || {
        let samples = list_sample_csvs_for_bindings();
        Ok(((), json!({ "count": samples.len() })))
    })?;
    steps.push(step);

    let (_, step) = measure_step("list_repo_data_csvs", || {
        let list = tauri::async_runtime::block_on(list_repo_data_csvs(
            Some(200),
            Some(false),
            Some(false),
        ))?;
        Ok((
            (),
            json!({ "count": list.csv_paths.len(), "truncated": list.truncated }),
        ))
    })?;
    steps.push(step);

    Ok(CaseSummary {
        id: "data.market_data_ingest_cache".to_string(),
        timing_boundary: "cold_data_path".to_string(),
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        steps,
        metrics: json!({
            "csv_path": csv_path.to_string_lossy(),
            "rows": cold.meta.len,
            "cache_dir": cache_dir.to_string_lossy()
        }),
    })
}

fn run_options_data_case(work_dir: &Path, cli: &Cli) -> Result<CaseSummary, String> {
    let started = Instant::now();
    let mut steps = Vec::new();
    let app_local_data_dir = work_dir.join("options_app_local_data");
    let csv_path = work_dir.join("options/options_dataset.csv");
    write_synthetic_options_csv(&csv_path, cli.options_rows)?;
    let dataset_id = format!("support_perf_options_{}", std::process::id());
    let request = OptionsDatasetImportRequest {
        csv_path: csv_path.to_string_lossy().to_string(),
        options_dataset_id: Some(dataset_id.clone()),
        root_symbol: Some("SPY".to_string()),
        provider_name: "massive".to_string(),
        provider_source_surface: "minute_aggregate".to_string(),
        provider_entitlement_profile: None,
        quote_resolution: "1m".to_string(),
        price_semantics: ta_app_core::OptionsPriceSemantics::MinuteAggregate,
        calendar_id: "UTC".to_string(),
        timezone: "UTC".to_string(),
        normalization_version: Some(1),
        quote_semantics_version: Some(1),
        corporate_action_mapping_version: Some(1),
        contract_reference_source: "massive_contract_reference".to_string(),
        contract_reference_as_of_utc: None,
        underlier_reference_source: "underlying_dataset".to_string(),
        aggregate_price_field: Some("close".to_string()),
    };

    let (manifest, step) = measure_step("import_options_dataset", || {
        let manifest = import_options_dataset_blocking_headless(request, &app_local_data_dir)?;
        Ok((
            manifest.clone(),
            json!({
                "dataset_id": manifest.options_dataset_id,
                "contracts": manifest.contract_count,
                "rows": manifest.row_count
            }),
        ))
    })?;
    steps.push(step);

    let datasets_dir = app_local_data_dir.join("options_research").join("datasets");
    let (_, step) = measure_step("scan_options_manifests", || {
        let manifests = ta_app_core::scan_options_dataset_manifests(&datasets_dir)?;
        Ok(((), json!({ "count": manifests.len() })))
    })?;
    steps.push(step);

    let (_, step) = measure_step("read_options_manifest", || {
        let manifest_path =
            ta_app_core::dataset_manifest_path(&datasets_dir, &manifest.options_dataset_id);
        let loaded = ta_app_core::read_options_dataset_manifest(&manifest_path)?;
        Ok((
            (),
            json!({
                "dataset_id": loaded.options_dataset_id,
                "manifest_path": manifest_path.to_string_lossy()
            }),
        ))
    })?;
    steps.push(step);

    Ok(CaseSummary {
        id: "options_data.dataset_ingest_manifest".to_string(),
        timing_boundary: "cold_data_path".to_string(),
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        steps,
        metrics: json!({
            "csv_path": csv_path.to_string_lossy(),
            "rows": cli.options_rows,
            "app_local_data_dir": app_local_data_dir.to_string_lossy()
        }),
    })
}

fn run_live_status_case(app_handle: tauri::AppHandle) -> Result<CaseSummary, String> {
    let started = Instant::now();
    let mut steps = Vec::new();

    let (status, step) = measure_step("polygon_live_candle_stream_status_idle", || {
        let response = polygon_live_candle_stream_status(app_handle.state::<AppState>())?;
        let metrics = json!({
            "active": response.active,
            "stream_present": response.stream.is_some()
        });
        Ok((response, metrics))
    })?;
    steps.push(step);

    let (stop, step) = measure_step("polygon_stop_live_candle_stream_idle", || {
        let response =
            polygon_stop_live_candle_stream(app_handle.state::<AppState>(), app_handle.clone())?;
        let metrics = json!({
            "stopped": response.stopped,
            "prior_present": response.prior.is_some()
        });
        Ok((response, metrics))
    })?;
    steps.push(step);

    Ok(CaseSummary {
        id: "data.polygon_live_status".to_string(),
        timing_boundary: "tauri_command_path".to_string(),
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        steps,
        metrics: json!({
            "fixture": "no_credential_idle_status",
            "network_stream_started": false,
            "initial_active": status.active,
            "stop_observed_prior": stop.prior.is_some()
        }),
    })
}

fn run_reports_case(cli: &Cli, app_handle: tauri::AppHandle) -> Result<CaseSummary, String> {
    let started = Instant::now();
    let mut steps = Vec::new();
    let report = report_value(cli.report_candidates);
    let validation_res = report
        .pointer("/validation/res")
        .cloned()
        .ok_or_else(|| "missing validation result".to_string())?;

    let (_, step) = measure_step("build_validation_shortlist_csv", || {
        let csv = tauri::async_runtime::block_on(build_validation_shortlist_csv(validation_res))?;
        Ok((
            (),
            json!({ "bytes": csv.len(), "lines": csv.lines().count() }),
        ))
    })?;
    steps.push(step);

    let (_, step) = measure_step("build_report_xlsx_bytes", || {
        let bytes = build_report_xlsx_bytes_headless(&report, Some(cli.report_candidates))?;
        Ok(((), json!({ "bytes": bytes.len() })))
    })?;
    steps.push(step);

    let runs_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("runs");
    fs::create_dir_all(&runs_dir).map_err(|e| e.to_string())?;
    let artifact_path = runs_dir.join("vectorgrid_run_support_perf_report.json");
    fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&json!({
            "kind": "vectorgrid_run_artifact",
            "ts": "2026-05-07T00:00:00Z",
            "dataset_meta": { "csv_path": "support.csv", "symbol": "SPY", "timeframe": "1m" },
            "run": { "req": { "strategy_kind": "DoubleMa" }, "res": { "top": [] } },
            "validation": {
                "req": { "candidates": [] },
                "res": report.pointer("/validation/res").cloned().unwrap_or(Value::Null)
            }
        }))
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let (_, step) = measure_step("save_report_xlsx_from_artifact", || {
        let saved = tauri::async_runtime::block_on(save_report_xlsx_from_artifact(
            artifact_path.to_string_lossy().to_string(),
            Some(cli.report_candidates as u32),
            app_handle,
        ))?;
        Ok((
            (),
            json!({
                "bytes": saved.bytes,
                "artifact_updated": saved.artifact_updated,
                "path": saved.path
            }),
        ))
    })?;
    steps.push(step);

    Ok(CaseSummary {
        id: "reports.export_generation_and_persistence".to_string(),
        timing_boundary: "tauri_command_path".to_string(),
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        steps,
        metrics: json!({ "candidate_count": cli.report_candidates }),
    })
}

fn run_artifacts_case(cli: &Cli, app_handle: tauri::AppHandle) -> Result<CaseSummary, String> {
    let started = Instant::now();
    let mut steps = Vec::new();
    let artifact = json!({
        "kind": "vectorgrid_run_artifact",
        "ts": "2026-05-07T00:00:00Z",
        "dataset_meta": { "csv_path": "support.csv", "symbol": "SPY", "timeframe": "1m" },
        "run": { "req": { "strategy_kind": "DoubleMa" }, "res": { "top": [] } },
        "validation": { "req": { "candidates": [] }, "res": validation_result(8) },
        "tags": ["support-perf"]
    });

    let mut saved_paths = Vec::with_capacity(cli.artifact_count);
    let (_, step) = measure_step("save_run_artifacts", || {
        for i in 0..cli.artifact_count {
            let mut artifact = artifact.clone();
            artifact["ts"] = Value::String(format!("2026-05-07T00:{:02}:00Z", i % 60));
            let saved = save_run_artifact_blocking(artifact, app_handle.clone(), None)?;
            saved_paths.push(saved.path);
        }
        Ok(((), json!({ "saved": saved_paths.len() })))
    })?;
    steps.push(step);

    let (_, step) = measure_step("rebuild_artifact_catalog", || {
        let snapshot = rebuild_artifact_catalog_blocking(app_handle.clone())?;
        Ok(((), json!({ "run_count": snapshot.runs.len() })))
    })?;
    steps.push(step);

    let (_, step) = measure_step("load_run_artifact_hydration", || {
        let first = saved_paths
            .first()
            .cloned()
            .ok_or_else(|| "no saved artifact path".to_string())?;
        let hydration =
            tauri::async_runtime::block_on(load_run_artifact_hydration(first, app_handle.clone()))?;
        Ok((
            (),
            json!({
                "run_ts": hydration.current_run.ts,
                "has_validation": hydration.current_validation.is_some()
            }),
        ))
    })?;
    steps.push(step);

    let (_, step) = measure_step("update_artifact_tags", || {
        let first = saved_paths
            .first()
            .cloned()
            .ok_or_else(|| "no saved artifact path".to_string())?;
        update_artifact_tags_blocking(
            first,
            vec!["support-perf".to_string(), "timed".to_string()],
            app_handle,
        )?;
        Ok(((), json!({ "updated": true })))
    })?;
    steps.push(step);

    Ok(CaseSummary {
        id: "artifacts.library_hydration_replay_repair".to_string(),
        timing_boundary: "tauri_command_path".to_string(),
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        steps,
        metrics: json!({ "artifact_count": cli.artifact_count }),
    })
}

fn run_portfolio_case() -> Result<CaseSummary, String> {
    let started = Instant::now();
    let mut steps = Vec::new();
    let artifacts: Vec<Value> = (0..120)
        .map(|i| {
            json!({
                "artifact_kind": "validation",
                "artifact_hash": format!("hash-{i}"),
                "candidates": [{
                    "candidate_id": format!("candidate-{i}"),
                    "validation_passed": true,
                    "run_approved": i % 2 == 0,
                    "symbol": if i % 2 == 0 { "SPY" } else { "QQQ" },
                    "strategy_kind": "DoubleMa",
                    "pnl": 1.0 + i as f64,
                    "sharpe": 1.2,
                    "max_dd": 0.3
                }]
            })
        })
        .collect();
    let request = ta_app_core::PortfolioFeasibilityRequestV1 {
        source_artifacts: artifacts,
        max_candidates: 100,
        require_approved_validation: true,
        require_replay_matrix: false,
        require_source_provenance: false,
    };
    let (_, step) = measure_step("portfolio_feasibility_from_artifacts", || {
        let result = ta_app_core::portfolio_feasibility_from_artifacts(request);
        Ok((
            (),
            json!({
                "status": format!("{:?}", result.status),
                "eligible": result.eligible_candidates.len(),
                "rejected": result.rejected_candidates.len()
            }),
        ))
    })?;
    steps.push(step);

    Ok(CaseSummary {
        id: "portfolio.feasibility_analysis".to_string(),
        timing_boundary: "engine_call".to_string(),
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        steps,
        metrics: json!({ "source_artifacts": 120 }),
    })
}

fn run_dynamic_case() -> Result<CaseSummary, String> {
    let started = Instant::now();
    let mut steps = Vec::new();
    let ir = ta_app_core::build_static_strategy_ir(
        ta_app_core::StrategyProfileKind::DoubleMa,
        ta_app_core::StaticStrategyIrInput::from_double_ma(
            "close".to_string(),
            "sma".to_string(),
            "ema".to_string(),
            None,
            None,
            StrategyConfig::default(),
            None,
        ),
    )?;
    let ir_json = serde_json::to_string(&ir).map_err(|e| e.to_string())?;

    let (_, step) = measure_step("preview_dynamic_strategy_ir", || {
        let preview = preview_dynamic_double_ma_strategy_ir_json(
            ir_json,
            Some(ta_app_core::StrategyPlanBackend::GpuUnified),
            Some(ObjectiveKind::Sharpe),
            Some(16),
            Some(false),
        )?;
        Ok((
            (),
            json!({
                "schema_compatibility": preview.schema_compatibility,
                "issue_count": preview.preview.issues.len()
            }),
        ))
    })?;
    steps.push(step);

    let (_, step) = measure_step("format_dynamic_strategy_ir", || {
        let pretty = format_dynamic_strategy_ir_json(ir)?;
        Ok(((), json!({ "bytes": pretty.len() })))
    })?;
    steps.push(step);

    let (_, step) = measure_step("encode_constant_bits", || {
        let bits = encode_dynamic_strategy_constant_bits(-0.0)?;
        Ok(((), json!({ "bits": bits.bits })))
    })?;
    steps.push(step);

    let (_, step) = measure_step("get_gpu_status", || {
        let status = tauri::async_runtime::block_on(get_gpu_status())?;
        Ok((
            (),
            json!({
                "built_with_cuda": status.built_with_cuda,
                "cuda_available": status.cuda_available,
                "device_name": status.auto_device_name
            }),
        ))
    })?;
    steps.push(step);

    Ok(CaseSummary {
        id: "system.dynamic_ir_gpu_status".to_string(),
        timing_boundary: "tauri_command_path".to_string(),
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        steps,
        metrics: json!({}),
    })
}

fn run_execution_case(app_handle: tauri::AppHandle) -> Result<CaseSummary, String> {
    let started = Instant::now();
    let mut steps = Vec::new();

    let (runner_status, step) = measure_step("live_execution_runner_status_idle", || {
        let status = live_execution_runner_status()?;
        let metrics = json!({
            "active": status.active,
            "inflight": status.inflight,
            "run_count": status.run_count,
            "error_count": status.error_count
        });
        Ok((status, metrics))
    })?;
    steps.push(step);

    let (support_matrix, step) = measure_step("get_execution_strategy_support_matrix", || {
        let rows = get_execution_strategy_support_matrix();
        let supported = rows.iter().filter(|row| row.supported).count();
        let metrics = json!({
            "rows": rows.len(),
            "supported": supported
        });
        Ok((rows, metrics))
    })?;
    steps.push(step);

    let (credential_status, step) = measure_step("alpaca_get_credential_status", || {
        let status = alpaca_get_credential_status(app_handle.clone());
        let metrics = json!({
            "remember_available": status.remember_available,
            "has_credentials": status.has_credentials,
            "state": format!("{:?}", status.state)
        });
        Ok((status, metrics))
    })?;
    steps.push(step);

    let (reset, step) = measure_step("reset_local_paper_broker", || {
        let response =
            tauri::async_runtime::block_on(reset_local_paper_broker(app_handle.clone()))?;
        let metrics = json!({
            "symbol": response.snapshot.symbol.clone(),
            "orders": response.snapshot.orders.len(),
            "has_position": response.snapshot.position.is_some()
        });
        Ok((response, metrics))
    })?;
    steps.push(step);

    let (snapshot, step) = measure_step("get_local_paper_execution_snapshot", || {
        let response = tauri::async_runtime::block_on(get_local_paper_execution_snapshot(
            GetLocalPaperExecutionSnapshotRequest {
                symbol: "SPY".to_string(),
                limit: Some(50),
            },
            app_handle,
        ))?;
        let metrics = json!({
            "symbol": response.symbol.clone(),
            "orders": response.orders.len(),
            "has_position": response.position.is_some()
        });
        Ok((response, metrics))
    })?;
    steps.push(step);

    Ok(CaseSummary {
        id: "execution.paper_live_policy_broker_surfaces".to_string(),
        timing_boundary: "tauri_command_path".to_string(),
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        steps,
        metrics: json!({
            "fixture": "no_credential_local_paper_and_idle_runner",
            "broker_network_calls": false,
            "runner_active": runner_status.active,
            "support_matrix_rows": support_matrix.len(),
            "credential_state": format!("{:?}", credential_status.state),
            "reset_orders": reset.snapshot.orders.len(),
            "snapshot_orders": snapshot.orders.len()
        }),
    })
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let requested_cases = normalize_requested_cases(&cli.case)?;
    let work_dir = cli.work_dir.clone().unwrap_or(default_work_dir()?);
    fs::create_dir_all(&work_dir).map_err(|e| e.to_string())?;

    let needs_app = selected(&requested_cases, "live-status")
        || selected(&requested_cases, "reports")
        || selected(&requested_cases, "artifacts")
        || selected(&requested_cases, "execution")
        || requested_cases.is_empty();
    let app = if needs_app {
        Some(build_isolated_app()?)
    } else {
        None
    };
    let app_data_dir = app
        .as_ref()
        .map(|app| {
            app.handle()
                .path()
                .app_local_data_dir()
                .map_err(|e| e.to_string())
        })
        .transpose()?;

    let mut cases = Vec::new();
    if selected(&requested_cases, "data") {
        cases.push(run_data_case(&work_dir, &cli)?);
    }
    if selected(&requested_cases, "options-data") {
        cases.push(run_options_data_case(&work_dir, &cli)?);
    }
    if selected(&requested_cases, "live-status") {
        let handle = app
            .as_ref()
            .ok_or_else(|| "live-status case requires app".to_string())?
            .handle()
            .clone();
        cases.push(run_live_status_case(handle)?);
    }
    if selected(&requested_cases, "reports") {
        let handle = app
            .as_ref()
            .ok_or_else(|| "reports case requires app".to_string())?
            .handle()
            .clone();
        cases.push(run_reports_case(&cli, handle)?);
    }
    if selected(&requested_cases, "artifacts") {
        let handle = app
            .as_ref()
            .ok_or_else(|| "artifacts case requires app".to_string())?
            .handle()
            .clone();
        cases.push(run_artifacts_case(&cli, handle)?);
    }
    if selected(&requested_cases, "portfolio") {
        cases.push(run_portfolio_case()?);
    }
    if selected(&requested_cases, "dynamic") {
        cases.push(run_dynamic_case()?);
    }
    if selected(&requested_cases, "execution") {
        let handle = app
            .as_ref()
            .ok_or_else(|| "execution case requires app".to_string())?
            .handle()
            .clone();
        cases.push(run_execution_case(handle)?);
    }

    let summary = SupportPerfSummary {
        schema_version: 1,
        timing_boundary: "support-performance-authority".to_string(),
        timing_boundary_note:
            "Lightweight non-credential support-surface timing; not a replacement for CPU/GPU identity gates."
                .to_string(),
        build_profile: build_profile().to_string(),
        cases_requested: requested_cases,
        cases,
    };

    if let Some(parent) = cli.out.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        &cli.out,
        serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("write {}: {e}", cli.out.display()))?;

    if cli.cleanup_app_data {
        if let Some(dir) = app_data_dir {
            let _ = fs::remove_dir_all(dir);
        }
    }

    println!("Support perf summary written to {}", cli.out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::normalize_requested_cases;

    #[test]
    fn normalize_requested_cases_rejects_unknown_case() {
        let err = normalize_requested_cases(&["unknown".to_string()]).unwrap_err();
        assert!(err.contains("unknown support perf case"));
    }

    #[test]
    fn normalize_requested_cases_keeps_known_cases_lowercase() {
        let cases = normalize_requested_cases(&["DATA".to_string(), " dynamic ".to_string()])
            .expect("known cases should parse");
        assert_eq!(cases, vec!["data".to_string(), "dynamic".to_string()]);
    }
}
