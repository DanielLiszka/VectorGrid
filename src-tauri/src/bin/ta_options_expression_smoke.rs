use clap::Parser;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use ta_app_core::{
    analyze_options_expression_feasibility_blocking, inspect_options_expression_tuple_blocking,
    run_options_expression_blocking, ImportedOptionsDataset, OptionType, OptionsContractSeries,
    OptionsDatasetManifestV1, OptionsEvaluationSplit, OptionsExitTemplateFamily,
    OptionsExpressionBackendPreference, OptionsExpressionFeasibilityReport,
    OptionsExpressionTupleInspectionResult, OptionsExpressionTupleResult, OptionsFillModelFamily,
    OptionsMoneynessBucket, OptionsPriceRow, OptionsPriceSemantics, OptionsSignalEvent,
    OptionsSignalEventManifestV1, OptionsSplitAssignmentMode, OptionsStaticFilterBundle,
    SignalSide,
};

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;

#[derive(Parser, Debug)]
#[command(name = "ta_options_expression_smoke")]
#[command(about = "Synthetic smoke runner for the options-expression backend")]
struct Cli {
    #[arg(long)]
    out_json: Option<PathBuf>,

    #[arg(long, default_value_t = 12)]
    events: usize,

    #[arg(long, default_value_t = 180)]
    rows: usize,

    #[arg(long, default_value = "auto")]
    backend_preference: String,

    #[arg(long, default_value_t = false)]
    compare_backends: bool,

    #[arg(long, default_value_t = 1)]
    iterations: usize,
}

#[derive(Debug, Serialize)]
struct SmokeOutput {
    request: ta_app_core::OptionsExpressionRunRequest,
    feasibility: OptionsExpressionFeasibilityReport,
    top_tuple: OptionsExpressionTupleResult,
    inspection: SmokeInspectionSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend_comparison: Option<SmokeBackendComparison>,
}

#[derive(Debug, Serialize)]
struct SmokeInspectionSummary {
    tuple_index: usize,
    total_events: usize,
    executed_events: usize,
    candidate_missing_events: usize,
    outcome_missing_events: usize,
    first_row: Option<ta_app_core::OptionsExpressionEventDetailRow>,
}

#[derive(Debug, Serialize)]
struct BackendRunSummary {
    backend_preference: String,
    backend_used: String,
    iterations: usize,
    duration_ms_samples: Vec<u64>,
    median_duration_ms: u64,
    tuple_count: usize,
    top_robust_score: Option<f64>,
    top_selection_expectancy_per_premium: Option<f64>,
}

#[derive(Debug, Serialize)]
struct SmokeBackendComparison {
    accelerated_backend_preference: String,
    cpu_reference: BackendRunSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    gpu_or_auto: Option<BackendRunSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wall_speedup_vs_cpu: Option<f64>,
}

fn minute_ts(base: i64, minute_index: usize) -> i64 {
    base + (minute_index as i64) * MINUTE_MS
}

fn make_underlier_path(rows: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(rows);
    let mut px = 100.0_f64;
    for idx in 0..rows {
        let drift = if idx < rows / 2 { 0.0009 } else { -0.0006 };
        let wave = ((idx as f64) * 0.095).sin() * 0.0022;
        px *= 1.0 + drift + wave;
        out.push(px.max(1.0));
    }
    out
}

fn option_mark_price(
    side: OptionType,
    strike: f64,
    underlying: f64,
    minute_index: usize,
    rows: usize,
) -> f64 {
    let intrinsic = match side {
        OptionType::Call => (underlying - strike).max(0.0),
        OptionType::Put => (strike - underlying).max(0.0),
    };
    let time_decay = 0.95 - 0.45 * (minute_index as f64 / rows.max(1) as f64);
    let wave = (((minute_index as f64) * 0.17) + strike * 0.03).sin().abs() * 0.08;
    (intrinsic * 0.62 + time_decay + wave).max(0.05)
}

fn make_contract_rows(
    base_ts: i64,
    underlying_path: &[f64],
    side: OptionType,
    strike: f64,
) -> Vec<OptionsPriceRow> {
    underlying_path
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, underlying)| {
            let close = option_mark_price(side, strike, underlying, idx, underlying_path.len());
            let high = close * 1.10;
            let low = close * 0.90;
            OptionsPriceRow {
                timestamp_ms: minute_ts(base_ts, idx),
                open: close * 0.98,
                high,
                low,
                close,
                underlying_price: underlying,
                bid: None,
                ask: None,
                mid: None,
                volume: Some(500.0 + (idx % 11) as f64 * 25.0),
                open_interest: Some(2_500.0),
                quote_age_ms: None,
                session_id: Some("session_a".to_string()),
            }
        })
        .collect()
}

fn synthetic_dataset(rows: usize) -> ImportedOptionsDataset {
    let base_ts = 1_700_000_000_000_i64;
    let underlying_path = make_underlier_path(rows.max(80));
    let expiration_short = base_ts + 3 * DAY_MS;
    let expiration_long = base_ts + 5 * DAY_MS;
    let mut contracts = Vec::new();
    for (contract_id, side, strike, expiration_ts, expiration_date) in [
        (
            "SPY_C_ATM_3D",
            OptionType::Call,
            100.0,
            expiration_short,
            "2026-04-08",
        ),
        (
            "SPY_C_OTM_3D",
            OptionType::Call,
            102.0,
            expiration_short,
            "2026-04-08",
        ),
        (
            "SPY_C_ITM_5D",
            OptionType::Call,
            98.0,
            expiration_long,
            "2026-04-10",
        ),
        (
            "SPY_P_ATM_3D",
            OptionType::Put,
            100.0,
            expiration_short,
            "2026-04-08",
        ),
        (
            "SPY_P_OTM_3D",
            OptionType::Put,
            98.0,
            expiration_short,
            "2026-04-08",
        ),
        (
            "SPY_P_ITM_5D",
            OptionType::Put,
            102.0,
            expiration_long,
            "2026-04-10",
        ),
    ] {
        contracts.push(OptionsContractSeries {
            root_symbol: "SPY".to_string(),
            contract_id: contract_id.to_string(),
            option_type: side,
            expiration_date: expiration_date.to_string(),
            expiration_timestamp_ms: expiration_ts,
            strike,
            multiplier: 100.0,
            rows: make_contract_rows(base_ts, &underlying_path, side, strike),
        });
    }

    ImportedOptionsDataset {
        manifest: OptionsDatasetManifestV1 {
            version: 1,
            options_dataset_id: "synthetic_spy".to_string(),
            root_symbol: "SPY".to_string(),
            provider_name: "synthetic".to_string(),
            provider_source_surface: "minute_aggregate".to_string(),
            provider_entitlement_profile: None,
            quote_resolution: "1m".to_string(),
            price_semantics: OptionsPriceSemantics::MinuteAggregate,
            calendar_id: "market_hours_utc".to_string(),
            timezone: "UTC".to_string(),
            coverage_start: base_ts,
            coverage_end: minute_ts(base_ts, underlying_path.len().saturating_sub(1)),
            normalization_version: 1,
            quote_semantics_version: 1,
            corporate_action_mapping_version: 1,
            contract_reference_source: "synthetic".to_string(),
            contract_reference_as_of_utc: None,
            underlier_reference_source: "synthetic_underlier".to_string(),
            dataset_path: "synthetic".to_string(),
            row_count: contracts.iter().map(|series| series.rows.len()).sum(),
            contract_count: contracts.len(),
            aggregate_price_field: Some("close".to_string()),
        },
        contracts,
    }
}

fn synthetic_manifest(events: usize, rows: usize) -> OptionsSignalEventManifestV1 {
    let base_ts = 1_700_000_000_000_i64;
    let underlying_path = make_underlier_path(rows.max(80));
    let event_count = events.max(6);
    let max_index = underlying_path.len().saturating_sub(6);
    let stride = (max_index / event_count.max(1)).max(5);
    let mut out = Vec::with_capacity(event_count);
    for idx in 0..event_count {
        let row_index = (5 + idx * stride).min(max_index);
        let side = if idx % 2 == 0 {
            SignalSide::Bullish
        } else {
            SignalSide::Bearish
        };
        out.push(OptionsSignalEvent {
            event_id: format!("evt_{idx:02}"),
            parent_artifact_id: "synthetic_parent".to_string(),
            parent_run_kind: "vectorgrid_run_artifact".to_string(),
            source_dataset_identity_hash: "synthetic_source".to_string(),
            source_content_fingerprint: "synthetic_fp".to_string(),
            dataset_hash: "synthetic_dataset_hash".to_string(),
            timeframe: "1m".to_string(),
            resample_spec_hash: None,
            adjustment_mode: "raw".to_string(),
            symbol_root: "SPY".to_string(),
            side,
            decision_timestamp: minute_ts(base_ts, row_index.saturating_sub(1)),
            earliest_tradable_timestamp: minute_ts(base_ts, row_index),
            decision_bar_index: row_index.saturating_sub(1),
            earliest_tradable_bar_index: row_index,
            raw_underlier_price: underlying_path[row_index],
            adjusted_underlier_price: None,
            base_entry_timestamp: Some(minute_ts(base_ts, row_index)),
            base_exit_timestamp: Some(minute_ts(base_ts, (row_index + 12).min(max_index))),
            base_exit_bar_index: Some((row_index + 12).min(max_index)),
            fold: OptionsEvaluationSplit::Search,
            session_id: Some("session_a".to_string()),
            regime_tag: Some(if idx % 3 == 0 { "trend" } else { "range" }.to_string()),
            confidence_tag: Some(if idx % 2 == 0 { "high" } else { "medium" }.to_string()),
            calendar_id: "market_hours_utc".to_string(),
        });
    }

    OptionsSignalEventManifestV1 {
        version: 1,
        manifest_id: "synthetic_manifest".to_string(),
        parent_artifact_id: "synthetic_parent".to_string(),
        parent_run_kind: "vectorgrid_run_artifact".to_string(),
        source_dataset_identity_hash: "synthetic_source".to_string(),
        source_content_fingerprint: "synthetic_fp".to_string(),
        dataset_hash: "synthetic_dataset_hash".to_string(),
        timeframe: "1m".to_string(),
        resample_spec_hash: None,
        adjustment_mode: "raw".to_string(),
        symbol_root: "SPY".to_string(),
        calendar_id: "market_hours_utc".to_string(),
        runtime_dataset_id: None,
        events: out,
    }
}

fn parse_backend_preference(raw: &str) -> Result<OptionsExpressionBackendPreference, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(OptionsExpressionBackendPreference::Auto),
        "cpu_reference" | "cpu" => Ok(OptionsExpressionBackendPreference::CpuReference),
        "gpu_compiled_tensor" | "gpu" => Ok(OptionsExpressionBackendPreference::GpuCompiledTensor),
        other => Err(format!("unsupported backend_preference '{}'", other)),
    }
}

fn smoke_request(
    backend_preference: OptionsExpressionBackendPreference,
) -> ta_app_core::OptionsExpressionRunRequest {
    ta_app_core::OptionsExpressionRunRequest {
        parent_artifact_path: "synthetic://options_expression_smoke".to_string(),
        parent_candidate_index: None,
        options_dataset_id: "synthetic_spy".to_string(),
        structure_family: ta_app_core::options_types::OptionsStructureFamily::LongSingle,
        backend_preference,
        objective_config: Default::default(),
        delay_minutes: vec![0, 1, 5],
        dte_buckets: vec![1, 3, 5],
        moneyness_buckets: vec![
            OptionsMoneynessBucket::Atm,
            OptionsMoneynessBucket::Otm1,
            OptionsMoneynessBucket::Itm1,
        ],
        hold_minutes: vec![5, 15, 30],
        target_return_buckets: vec![0.25, 0.5],
        stop_return_buckets: vec![0.25, 0.5],
        exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
        fill_model: OptionsFillModelFamily::AggregateResearch,
        filters: OptionsStaticFilterBundle {
            max_entry_lag_minutes: Some(5),
            max_premium: Some(500.0),
            min_premium: Some(10.0),
            min_volume: Some(100.0),
            min_open_interest: Some(500.0),
            max_quote_age_ms: None,
            max_spread_bps: None,
        },
        top_k: 10,
        min_valid_trades: Some(1),
        split_ratio_search: 0.6,
        split_ratio_holdout: 0.2,
        split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
        sealed_finalist_count: 3,
    }
}

fn inspection_summary(
    inspection: OptionsExpressionTupleInspectionResult,
) -> SmokeInspectionSummary {
    SmokeInspectionSummary {
        tuple_index: inspection.tuple_index,
        total_events: inspection.total_events,
        executed_events: inspection.executed_events,
        candidate_missing_events: inspection.candidate_missing_events,
        outcome_missing_events: inspection.outcome_missing_events,
        first_row: inspection.rows.into_iter().next(),
    }
}

fn benchmark_backend(
    mut request: ta_app_core::OptionsExpressionRunRequest,
    manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
    backend_preference: OptionsExpressionBackendPreference,
    iterations: usize,
) -> Result<BackendRunSummary, String> {
    request.backend_preference = backend_preference;
    let mut duration_ms_samples = Vec::with_capacity(iterations);
    let mut result = None;
    for _ in 0..iterations {
        let start = Instant::now();
        let run = run_options_expression_blocking(request.clone(), manifest, dataset)?;
        duration_ms_samples.push(start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
        result = Some(run);
    }
    let result = result.ok_or_else(|| "backend benchmark ran zero iterations".to_string())?;
    let mut sorted = duration_ms_samples.clone();
    sorted.sort_unstable();
    let median_duration_ms = sorted[sorted.len() / 2];
    Ok(BackendRunSummary {
        backend_preference: format!("{backend_preference:?}"),
        backend_used: result.backend_used,
        iterations,
        duration_ms_samples,
        median_duration_ms,
        tuple_count: result.tuple_count,
        top_robust_score: result.ranked_results.first().map(|row| row.robust_score),
        top_selection_expectancy_per_premium: result
            .ranked_results
            .first()
            .map(|row| row.selection_metrics.expectancy_per_premium),
    })
}

fn assert_smoke_invariants(
    report: &OptionsExpressionFeasibilityReport,
    top_tuple: &OptionsExpressionTupleResult,
    inspection: &SmokeInspectionSummary,
) -> Result<(), String> {
    if report.compiled_tuple_count == 0 {
        return Err("smoke run compiled zero tuples".to_string());
    }
    if !(0.0..=1.0).contains(&report.candidate_density) {
        return Err("candidate density is outside [0, 1]".to_string());
    }
    if top_tuple.selection_metrics.valid_trades == 0 {
        return Err("top tuple has zero valid trades".to_string());
    }
    if top_tuple.holdout_metrics.is_none() {
        return Err("top tuple is missing holdout metrics".to_string());
    }
    if top_tuple.sealed_evaluation_metrics.is_none() {
        return Err("top tuple is missing sealed evaluation metrics".to_string());
    }
    if top_tuple.stress_metrics.is_none() {
        return Err("top tuple is missing stress metrics".to_string());
    }
    if inspection.total_events != report.diagnostics.total_events {
        return Err("inspection event count does not match diagnostics".to_string());
    }
    if inspection.executed_events == 0 {
        return Err("inspection executed zero events".to_string());
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let dataset = synthetic_dataset(cli.rows);
    let manifest = synthetic_manifest(cli.events, cli.rows);
    let request = smoke_request(parse_backend_preference(&cli.backend_preference)?);
    let feasibility =
        analyze_options_expression_feasibility_blocking(request.clone(), &manifest, &dataset)?;
    let run = run_options_expression_blocking(request.clone(), &manifest, &dataset)?;
    let top_tuple = run
        .ranked_results
        .first()
        .cloned()
        .ok_or_else(|| "smoke run produced no ranked tuples".to_string())?;
    let inspection =
        inspect_options_expression_tuple_blocking(request.clone(), 0, &manifest, &dataset)?;
    let inspection = inspection_summary(inspection);
    assert_smoke_invariants(&feasibility, &top_tuple, &inspection)?;
    let backend_comparison = if cli.compare_backends {
        let iterations = cli.iterations.max(1);
        let accelerated_backend_preference =
            match parse_backend_preference(&cli.backend_preference)? {
                OptionsExpressionBackendPreference::CpuReference => {
                    OptionsExpressionBackendPreference::Auto
                }
                other => other,
            };
        let cpu_reference = benchmark_backend(
            request.clone(),
            &manifest,
            &dataset,
            OptionsExpressionBackendPreference::CpuReference,
            iterations,
        )?;
        let gpu_or_auto = Some(benchmark_backend(
            request.clone(),
            &manifest,
            &dataset,
            accelerated_backend_preference,
            iterations,
        )?);
        let wall_speedup_vs_cpu = gpu_or_auto.as_ref().and_then(|accelerated| {
            if cpu_reference.median_duration_ms > 0 && accelerated.median_duration_ms > 0 {
                Some(
                    cpu_reference.median_duration_ms as f64 / accelerated.median_duration_ms as f64,
                )
            } else {
                None
            }
        });
        Some(SmokeBackendComparison {
            accelerated_backend_preference: format!("{accelerated_backend_preference:?}"),
            cpu_reference,
            gpu_or_auto,
            wall_speedup_vs_cpu,
        })
    } else {
        None
    };

    let output = SmokeOutput {
        request,
        feasibility,
        top_tuple,
        inspection,
        backend_comparison,
    };
    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    if let Some(path) = cli.out_json.as_ref() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, &json).map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    }
    println!("{json}");
    Ok(())
}
