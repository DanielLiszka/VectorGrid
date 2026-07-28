use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;
use ta_app_core::{
    options_types::{
        OptionsExpressionEventStatus, OptionsExpressionValidationDatasetPolicy,
        OptionsExpressionValidationPlan, OptionsExpressionValidationRequest,
        OptionsExpressionValidationTupleSelection,
    },
    OptionsExpressionBackendPreference, OptionsExpressionRunRequest,
};
use ta_desktop_demo_app::commands::{
    analyze_options_expression_feasibility_blocking_headless,
    analyze_options_expression_validation_feasibility_blocking_headless,
    inspect_options_expression_event_path_blocking_headless,
    inspect_options_expression_tuple_blocking_headless,
    list_options_expression_artifacts_blocking_headless,
    load_options_expression_artifact_blocking_headless, run_options_expression_blocking_headless,
    save_options_csv_blocking_headless, validate_options_expression_artifact_blocking_headless,
    OptionsExpressionReplayBuildTimingBreakdown, SaveOptionsCsvRequest,
};

#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Phase {
    Run,
    Recover,
}

#[derive(Parser, Debug)]
#[command(name = "ta_options_expression_lifecycle")]
#[command(about = "Exercise the saved options-expression lifecycle without opening the desktop UI")]
struct Cli {
    #[arg(long)]
    app_local_data_dir: PathBuf,

    #[arg(
        long,
        help = "Saved options-expression artifact used as the request seed in run mode or recovered directly in recover mode"
    )]
    artifact: PathBuf,

    #[arg(long, value_enum)]
    phase: Phase,

    #[arg(long)]
    export_csv: PathBuf,

    #[arg(long)]
    out_json: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct RunStageSummary {
    source_artifact_path: String,
    created_artifact_path: String,
    backend_used: String,
    tuple_count: usize,
}

#[derive(Debug, Serialize)]
struct ValidationStageSummary {
    feasibility_selected_tuple_count: usize,
    validation_artifact_path: String,
    validated_tuple_count: usize,
    acceptance_verdict: String,
}

#[derive(Debug, Serialize)]
struct InspectionStageSummary {
    tuple_index: usize,
    tuple_count: usize,
    total_events: usize,
    executed_events: usize,
    candidate_missing_events: usize,
    outcome_missing_events: usize,
    event_index: usize,
    event_id: String,
    event_status: String,
    event_path_point_count: usize,
}

#[derive(Debug, Serialize)]
struct LifecycleOutput {
    status: &'static str,
    phase: String,
    app_local_data_dir: String,
    source_artifact_path: String,
    active_artifact_path: String,
    replay_build_timing: Option<OptionsExpressionReplayBuildTimingBreakdown>,
    compiled_tuple_count: usize,
    run: Option<RunStageSummary>,
    listed_artifact_count: usize,
    loaded_tuple_count: usize,
    inspection: InspectionStageSummary,
    validation: ValidationStageSummary,
    export_path: String,
    export_bytes: usize,
    checks: Vec<&'static str>,
}

fn truncate_axis<T>(values: &mut Vec<T>, limit: usize) {
    if values.len() > limit {
        values.truncate(limit);
    }
}

fn bounded_cpu_request(mut request: OptionsExpressionRunRequest) -> OptionsExpressionRunRequest {
    request.backend_preference = OptionsExpressionBackendPreference::CpuReference;
    truncate_axis(&mut request.delay_minutes, 2);
    truncate_axis(&mut request.dte_buckets, 2);
    truncate_axis(&mut request.moneyness_buckets, 2);
    truncate_axis(&mut request.hold_minutes, 2);
    truncate_axis(&mut request.target_return_buckets, 1);
    truncate_axis(&mut request.stop_return_buckets, 1);
    request.top_k = request.top_k.clamp(1, 5);
    request.sealed_finalist_count = request.sealed_finalist_count.clamp(1, request.top_k.max(1));
    request
}

fn validation_request(artifact_path: &str) -> OptionsExpressionValidationRequest {
    OptionsExpressionValidationRequest {
        artifact_path: artifact_path.to_string(),
        options_dataset_id: String::new(),
        dataset_policy: OptionsExpressionValidationDatasetPolicy::RequireExactReplay,
        tuple_selection: OptionsExpressionValidationTupleSelection {
            include_sealed_finalists: false,
            include_robust_leader: false,
            include_holdout_leader: false,
            tuple_indexes: vec![0],
        },
        validation_plan: OptionsExpressionValidationPlan::default(),
        fill_model_override: None,
        min_valid_trades: None,
    }
}

fn csv_escape(value: impl AsRef<str>) -> String {
    let value = value.as_ref();
    if value.contains(',') || value.contains('"') || value.contains('\r') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn build_export_csv(artifact: &ta_app_core::OptionsExpressionArtifactV1) -> Result<String, String> {
    let mut out =
        String::from("tuple_index,tuple,robust_score,validation_verdict,selection_valid_trades\n");
    for (tuple_index, row) in artifact.selection_metrics.iter().enumerate() {
        let tuple = serde_json::to_string(&row.tuple).map_err(|e| e.to_string())?;
        let verdict = serde_json::to_string(&row.validation_verdict)
            .map_err(|e| e.to_string())?
            .trim_matches('"')
            .to_string();
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            tuple_index,
            csv_escape(tuple),
            row.robust_score,
            csv_escape(verdict),
            row.selection_metrics.valid_trades
        ));
    }
    Ok(out)
}

fn run() -> Result<(LifecycleOutput, Option<PathBuf>), String> {
    let cli = Cli::parse();
    let source_artifact =
        load_options_expression_artifact_blocking_headless(&cli.artifact, &cli.app_local_data_dir)?;
    if source_artifact.selection_metrics.is_empty() {
        return Err(format!(
            "source options-expression artifact contains no retained tuples: {}",
            cli.artifact.display()
        ));
    }

    let source_artifact_path = cli.artifact.to_string_lossy().to_string();
    let request = match cli.phase {
        Phase::Run => bounded_cpu_request(source_artifact.request.clone()),
        Phase::Recover => source_artifact.request.clone(),
    };
    let feasibility = analyze_options_expression_feasibility_blocking_headless(
        request.clone(),
        &cli.app_local_data_dir,
    )?;
    let (active_artifact_path, run_summary) = match cli.phase {
        Phase::Run => {
            let response =
                run_options_expression_blocking_headless(request, &cli.app_local_data_dir)?;
            let summary = RunStageSummary {
                source_artifact_path: source_artifact_path.clone(),
                created_artifact_path: response.artifact_path.clone(),
                backend_used: response.result.backend_used.clone(),
                tuple_count: response.result.tuple_count,
            };
            (response.artifact_path, Some(summary))
        }
        Phase::Recover => (source_artifact_path.clone(), None),
    };
    let summaries = list_options_expression_artifacts_blocking_headless(&cli.app_local_data_dir)?;
    if !summaries
        .iter()
        .any(|entry| entry.path == active_artifact_path)
    {
        return Err(format!(
            "active options-expression artifact was not returned by list: {active_artifact_path}"
        ));
    }

    let active_path = PathBuf::from(&active_artifact_path);
    let loaded =
        load_options_expression_artifact_blocking_headless(&active_path, &cli.app_local_data_dir)?;
    if loaded.selection_metrics.is_empty() {
        return Err(format!(
            "active options-expression artifact contains no retained tuples: {active_artifact_path}"
        ));
    }
    let inspection = inspect_options_expression_tuple_blocking_headless(
        loaded.request.clone(),
        0,
        &cli.app_local_data_dir,
    )?;
    let inspected_event = inspection
        .rows
        .iter()
        .find(|row| row.status == OptionsExpressionEventStatus::Executed)
        .or_else(|| inspection.rows.first())
        .ok_or_else(|| {
            format!("tuple replay returned no inspectable events for {active_artifact_path}")
        })?;
    let event_path = inspect_options_expression_event_path_blocking_headless(
        loaded.request.clone(),
        0,
        inspected_event.event_index,
        &cli.app_local_data_dir,
    )?;
    if event_path.event_id != inspected_event.event_id {
        return Err(format!(
            "event-path replay drifted from tuple inspection: '{}' != '{}'",
            event_path.event_id, inspected_event.event_id
        ));
    }
    if inspected_event.status == OptionsExpressionEventStatus::Executed
        && event_path.points.is_empty()
    {
        return Err(format!(
            "executed event '{}' returned no replay path points",
            event_path.event_id
        ));
    }

    let validation_request = validation_request(&active_artifact_path);
    let validation_feasibility =
        analyze_options_expression_validation_feasibility_blocking_headless(
            validation_request.clone(),
            &cli.app_local_data_dir,
        )?;
    let validation = validate_options_expression_artifact_blocking_headless(
        validation_request,
        &cli.app_local_data_dir,
    )?;
    if validation.result.source_artifact_path != active_artifact_path {
        return Err(format!(
            "validation source path drifted: '{}' != '{}'",
            validation.result.source_artifact_path, active_artifact_path
        ));
    }

    let export = save_options_csv_blocking_headless(SaveOptionsCsvRequest {
        path: cli.export_csv.to_string_lossy().to_string(),
        csv_text: build_export_csv(&loaded)?,
    })?;
    if !cli.export_csv.is_file() {
        return Err(format!(
            "options-expression CSV export was not created: {}",
            cli.export_csv.display()
        ));
    }

    let event_status = serde_json::to_string(&event_path.row.status).map_err(|e| e.to_string())?;
    let acceptance_verdict = serde_json::to_string(&validation.result.acceptance_summary.verdict)
        .map_err(|e| e.to_string())?;
    let output = LifecycleOutput {
        status: "passed",
        phase: match cli.phase {
            Phase::Run => "run".to_string(),
            Phase::Recover => "recover".to_string(),
        },
        app_local_data_dir: cli.app_local_data_dir.to_string_lossy().to_string(),
        source_artifact_path,
        active_artifact_path,
        replay_build_timing: feasibility.replay_build_timing,
        compiled_tuple_count: feasibility.report.compiled_tuple_count,
        run: run_summary,
        listed_artifact_count: summaries.len(),
        loaded_tuple_count: loaded.selection_metrics.len(),
        inspection: InspectionStageSummary {
            tuple_index: inspection.tuple_index,
            tuple_count: inspection.tuple_count,
            total_events: inspection.total_events,
            executed_events: inspection.executed_events,
            candidate_missing_events: inspection.candidate_missing_events,
            outcome_missing_events: inspection.outcome_missing_events,
            event_index: event_path.event_index,
            event_id: event_path.event_id,
            event_status: event_status.trim_matches('"').to_string(),
            event_path_point_count: event_path.point_count,
        },
        validation: ValidationStageSummary {
            feasibility_selected_tuple_count: validation_feasibility.selected_tuple_count,
            validation_artifact_path: validation.artifact_path,
            validated_tuple_count: validation.result.tuple_results.len(),
            acceptance_verdict: acceptance_verdict.trim_matches('"').to_string(),
        },
        export_path: export.path,
        export_bytes: export.bytes,
        checks: vec![
            "run_or_recover",
            "list",
            "load",
            "tuple_replay",
            "event_path_replay",
            "validation_feasibility",
            "validation",
            "csv_export",
        ],
    };
    Ok((output, cli.out_json))
}

fn main() -> Result<(), String> {
    let (output, out_path) = run()?;
    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    if let Some(out_path) = out_path {
        atomic_output::write_str(&out_path, &json)
            .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
    }
    println!("{json}");
    Ok(())
}
