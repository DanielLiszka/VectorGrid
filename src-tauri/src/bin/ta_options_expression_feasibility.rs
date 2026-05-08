use clap::Parser;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;
use ta_app_core::options_types::OptionsExpressionFeasibilityEventCoverage;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use ta_app_core::preferred_cuda_device_id_result;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use ta_app_core::prewarm_options_expression_gpu_runtime;
use ta_app_core::{
    scan_options_dataset_manifests, OptionsCandidateInvalidReason, OptionsDatasetManifestV1,
    OptionsExitTemplateFamily, OptionsExpressionBackendPreference, OptionsExpressionDiagnostics,
    OptionsExpressionRunRequest, OptionsExpressionTupleInspectionResult,
    OptionsExpressionTupleResult, OptionsFillModelFamily, OptionsMoneynessBucket,
    OptionsSplitAssignmentMode, OptionsStaticFilterBundle,
};
use ta_desktop_demo_app::commands::{
    analyze_options_expression_feasibility_blocking_headless,
    import_options_dataset_blocking_headless, inspect_options_expression_tuple_blocking_headless,
    run_options_expression_blocking_headless, AnalyzeOptionsExpressionFeasibilityResponse,
    OptionsDatasetImportRequest, RunOptionsExpressionResponse,
};
use ta_desktop_demo_app::settings;
use tauri::Manager;

#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;
#[path = "../bin_support/benchmark_lock.rs"]
mod benchmark_lock;

#[derive(Parser, Debug)]
#[command(name = "ta_options_expression_feasibility")]
#[command(
    about = "Headless Phase 0.5 feasibility preview for Polygon options-expression artifacts"
)]
struct Cli {
    #[arg(long)]
    artifact: Option<PathBuf>,

    #[arg(long)]
    dataset_id: Option<String>,

    #[arg(
        long,
        help = "Import a normalized Polygon options CSV into the local dataset store before running the preview"
    )]
    import_csv: Option<PathBuf>,

    #[arg(long, help = "Root symbol for the imported Polygon dataset")]
    import_root_symbol: Option<String>,

    #[arg(long)]
    app_local_data_dir: Option<PathBuf>,

    #[arg(long)]
    out_json: Option<PathBuf>,

    #[arg(long)]
    out_markdown: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    run_full: bool,

    #[arg(long, default_value_t = false)]
    inspect_top: bool,

    #[arg(long, default_value_t = 0)]
    warm_repeats: usize,

    #[arg(long, default_value_t = false)]
    compare_backends: bool,

    #[arg(long, default_value_t = false)]
    prewarm_gpu_runtime: bool,

    #[arg(long, default_value_t = 1)]
    benchmark_iterations: usize,

    #[arg(long, default_value = "0,1,5")]
    delay_minutes: String,

    #[arg(long, default_value = "1,3,5")]
    dte_buckets: String,

    #[arg(long, default_value = "atm,otm_1,otm_2,itm_1")]
    moneyness_buckets: String,

    #[arg(long, default_value = "5,15,30,60")]
    hold_minutes: String,

    #[arg(long, default_value = "0.25,0.5,1")]
    target_return_buckets: String,

    #[arg(long, default_value = "0.25,0.5")]
    stop_return_buckets: String,

    #[arg(long, default_value = "fixed_hold_or_target_stop")]
    exit_template: String,

    #[arg(long, default_value = "aggregate_research")]
    fill_model: String,

    #[arg(long, default_value = "auto")]
    backend_preference: String,

    #[arg(long, default_value_t = 25)]
    top_k: usize,

    #[arg(long, default_value_t = 5)]
    sealed_finalist_count: usize,

    #[arg(long)]
    min_valid_trades: Option<usize>,

    #[arg(long, default_value_t = 0.6)]
    split_ratio_search: f64,

    #[arg(long, default_value_t = 0.2)]
    split_ratio_holdout: f64,

    #[arg(long, default_value = "event_chronological")]
    split_assignment_mode: String,

    #[arg(long, default_value_t = 5)]
    max_entry_lag_minutes: u32,

    #[arg(long)]
    max_premium: Option<f64>,

    #[arg(long)]
    min_premium: Option<f64>,

    #[arg(long)]
    min_volume: Option<f64>,

    #[arg(long)]
    min_open_interest: Option<f64>,

    #[arg(long, default_value_t = false)]
    list_datasets: bool,
}

#[derive(Debug, Serialize)]
struct FeasibilityHarnessOutput {
    app_local_data_dir: String,
    artifact_path: String,
    options_dataset_id: String,
    request_split_assignment_mode: String,
    dataset_manifest: OptionsDatasetManifestV1,
    response: AnalyzeOptionsExpressionFeasibilityResponse,
    phase05_gates: Phase05GateEvaluation,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    recommendations: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    warm_runs: Vec<WarmPreviewSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend_comparison: Option<FeasibilityBackendComparison>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_summary: Option<RunSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_tuple_inspection: Option<OptionsExpressionTupleInspectionResult>,
}

#[derive(Debug, Serialize)]
struct RunSummary {
    artifact_path: String,
    backend_used: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend_fallback_reason: Option<String>,
    tuple_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_tuple: Option<OptionsExpressionTupleResult>,
}

#[derive(Debug, Serialize)]
struct WarmPreviewSummary {
    iteration: usize,
    replay_build_ms: u64,
    total_preview_ms: u64,
    best_available_backend: String,
    auto_backend_recommended_backend: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    auto_backend_reasons: Vec<String>,
    structural_cache_hits: usize,
    structural_cache_misses: usize,
    summary_cache_hits: usize,
    summary_cache_misses: usize,
    gpu_memory_budget_source: String,
    gpu_outcome_tensor_fits_in_memory: bool,
    gpu_chunk_plan_fits_in_memory: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    gpu_plan_notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PreviewBenchmarkSummary {
    backend_preference: String,
    best_available_backend: String,
    auto_backend_recommended_backend: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    auto_backend_reasons: Vec<String>,
    iterations: usize,
    wall_ms_samples: Vec<u64>,
    preview_ms_samples: Vec<u64>,
    estimated_backend_preview_ms_samples: Vec<u64>,
    ranking_ms_samples: Vec<u64>,
    ranking_us_samples: Vec<u64>,
    median_wall_ms: u64,
    median_preview_ms: u64,
    median_estimated_backend_preview_ms: u64,
    median_ranking_ms: u64,
    median_ranking_us: u64,
    candidate_density: f64,
    invalid_slot_rate: f64,
    gpu_memory_budget_source: String,
    gpu_outcome_tensor_fits_in_memory: bool,
    gpu_chunk_plan_fits_in_memory: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    gpu_plan_notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RunBenchmarkSummary {
    backend_preference: String,
    backend_used: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend_fallback_reason: Option<String>,
    iterations: usize,
    wall_ms_samples: Vec<u64>,
    median_wall_ms: u64,
    tuple_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_robust_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_selection_expectancy_per_premium: Option<f64>,
}

#[derive(Debug, Serialize)]
struct FeasibilityBackendComparison {
    accelerated_backend_preference: String,
    cpu_reference_preview: PreviewBenchmarkSummary,
    accelerated_preview: PreviewBenchmarkSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_backend_preview_wall_speedup_vs_cpu: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_wall_speedup_vs_cpu: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hybrid_ranking_speedup_vs_cpu: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scorer_only_speedup_vs_cpu: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_reference_run: Option<RunBenchmarkSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accelerated_run: Option<RunBenchmarkSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    full_run_wall_speedup_vs_cpu: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Phase05GateEvaluation {
    valid_slot_density_gate_min: f64,
    valid_slot_density_observed: f64,
    valid_slot_density_pass: bool,
    invalid_slot_rate_gate_max: f64,
    invalid_slot_rate_observed: f64,
    invalid_slot_rate_pass: bool,
    cache_reuse_gate_min: f64,
    structural_cache_reuse_observed: Option<f64>,
    structural_cache_reuse_pass: Option<bool>,
    summary_cache_reuse_observed: Option<f64>,
    summary_cache_reuse_pass: Option<bool>,
    gpu_speedup_gate_min: f64,
    gpu_speedup_observed: Option<f64>,
    gpu_speedup_pass: Option<bool>,
    scorer_share_of_preview_observed: Option<f64>,
    scorer_backend_used_for_share: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    notes: Vec<String>,
}

fn serialize_scalar_to_string<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(|raw| raw.to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn cache_reuse_ratio(hits: usize, misses: usize) -> Option<f64> {
    let total = hits + misses;
    if total == 0 {
        None
    } else {
        Some(hits as f64 / total as f64)
    }
}

fn median_u64(samples: &[u64]) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    Some(sorted[sorted.len() / 2])
}

fn observed_speedup(cpu_median_ms: Option<u64>, accelerated_median_ms: Option<u64>) -> Option<f64> {
    match (cpu_median_ms, accelerated_median_ms) {
        (Some(cpu), Some(accelerated)) if cpu > 0 && accelerated > 0 => {
            Some(cpu as f64 / accelerated as f64)
        }
        _ => None,
    }
}

fn format_millis_from_us(us: u64) -> String {
    if us == 0 {
        "0.000".to_string()
    } else {
        format!("{:.3}", us as f64 / 1_000.0)
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn maybe_prewarm_gpu_runtime(enabled: bool) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    match preferred_cuda_device_id_result()? {
        Some(device_id) => prewarm_options_expression_gpu_runtime(device_id),
        None => Ok(()),
    }
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn maybe_prewarm_gpu_runtime(_enabled: bool) -> Result<(), String> {
    Ok(())
}

fn accelerated_backend_target(
    requested: OptionsExpressionBackendPreference,
) -> OptionsExpressionBackendPreference {
    match requested {
        OptionsExpressionBackendPreference::CpuReference => {
            OptionsExpressionBackendPreference::Auto
        }
        other => other,
    }
}

fn benchmark_preview(
    mut request: OptionsExpressionRunRequest,
    app_local_data_dir: &Path,
    backend_preference: OptionsExpressionBackendPreference,
    iterations: usize,
) -> Result<PreviewBenchmarkSummary, String> {
    request.backend_preference = backend_preference;
    let mut wall_ms_samples = Vec::with_capacity(iterations);
    let mut preview_ms_samples = Vec::with_capacity(iterations);
    let mut estimated_backend_preview_ms_samples = Vec::with_capacity(iterations);
    let mut ranking_ms_samples = Vec::with_capacity(iterations);
    let mut ranking_us_samples = Vec::with_capacity(iterations);
    let mut last_response: Option<AnalyzeOptionsExpressionFeasibilityResponse> = None;
    for _ in 0..iterations {
        let start = Instant::now();
        let response = analyze_options_expression_feasibility_blocking_headless(
            request.clone(),
            app_local_data_dir,
        )?;
        let wall_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let ranking_us = if response.report.best_available_backend.contains("gpu") {
            response
                .report
                .timings
                .gpu_ranking_us
                .unwrap_or(response.report.timings.cpu_ranking_us)
        } else {
            response.report.timings.cpu_ranking_us
        };
        let estimated_backend_preview_us = if response.report.best_available_backend.contains("gpu")
            && response.report.timings.gpu_ranking_us.is_some()
        {
            response
                .total_preview_ms
                .saturating_mul(1_000)
                .saturating_sub(response.report.timings.cpu_ranking_us)
        } else {
            response.total_preview_ms.saturating_mul(1_000)
        };
        let ranking_ms = ranking_us / 1_000;
        wall_ms_samples.push(wall_ms);
        preview_ms_samples.push(response.total_preview_ms);
        estimated_backend_preview_ms_samples.push(estimated_backend_preview_us / 1_000);
        ranking_ms_samples.push(ranking_ms);
        ranking_us_samples.push(ranking_us);
        last_response = Some(response);
    }
    let last_response =
        last_response.ok_or_else(|| "preview benchmark ran zero iterations".to_string())?;
    Ok(PreviewBenchmarkSummary {
        backend_preference: format!("{backend_preference:?}"),
        best_available_backend: last_response.report.best_available_backend.clone(),
        auto_backend_recommended_backend: last_response
            .report
            .auto_backend_decision
            .recommended_backend
            .clone(),
        auto_backend_reasons: last_response.report.auto_backend_decision.reasons.clone(),
        iterations,
        median_wall_ms: median_u64(&wall_ms_samples).unwrap_or(0),
        median_preview_ms: median_u64(&preview_ms_samples).unwrap_or(0),
        median_estimated_backend_preview_ms: median_u64(&estimated_backend_preview_ms_samples)
            .unwrap_or(0),
        median_ranking_ms: median_u64(&ranking_ms_samples).unwrap_or(0),
        median_ranking_us: median_u64(&ranking_us_samples).unwrap_or(0),
        wall_ms_samples,
        preview_ms_samples,
        estimated_backend_preview_ms_samples,
        ranking_ms_samples,
        ranking_us_samples,
        candidate_density: last_response.report.candidate_density,
        invalid_slot_rate: last_response.report.invalid_slot_rate,
        gpu_memory_budget_source: last_response
            .report
            .cache_stats
            .gpu_memory_budget_source
            .clone(),
        gpu_outcome_tensor_fits_in_memory: last_response
            .report
            .cache_stats
            .gpu_outcome_tensor_fits_in_memory,
        gpu_chunk_plan_fits_in_memory: last_response
            .report
            .cache_stats
            .gpu_chunk_plan_fits_in_memory,
        gpu_plan_notes: last_response.report.cache_stats.gpu_plan_notes.clone(),
    })
}

fn benchmark_run(
    mut request: OptionsExpressionRunRequest,
    app_local_data_dir: &Path,
    backend_preference: OptionsExpressionBackendPreference,
    iterations: usize,
) -> Result<RunBenchmarkSummary, String> {
    request.backend_preference = backend_preference;
    let mut wall_ms_samples = Vec::with_capacity(iterations);
    let mut last_response: Option<RunOptionsExpressionResponse> = None;
    for _ in 0..iterations {
        let start = Instant::now();
        let response =
            run_options_expression_blocking_headless(request.clone(), app_local_data_dir)?;
        let wall_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        wall_ms_samples.push(wall_ms);
        last_response = Some(response);
    }
    let last_response =
        last_response.ok_or_else(|| "run benchmark ran zero iterations".to_string())?;
    Ok(RunBenchmarkSummary {
        backend_preference: format!("{backend_preference:?}"),
        backend_used: last_response.result.backend_used.clone(),
        backend_fallback_reason: last_response.result.backend_fallback_reason.clone(),
        iterations,
        median_wall_ms: median_u64(&wall_ms_samples).unwrap_or(0),
        wall_ms_samples,
        tuple_count: last_response.result.tuple_count,
        top_robust_score: last_response
            .result
            .ranked_results
            .first()
            .map(|row| row.robust_score),
        top_selection_expectancy_per_premium: last_response
            .result
            .ranked_results
            .first()
            .map(|row| row.selection_metrics.expectancy_per_premium),
    })
}

fn build_phase05_gate_evaluation(
    response: &AnalyzeOptionsExpressionFeasibilityResponse,
    warm_runs: &[WarmPreviewSummary],
    backend_comparison: Option<&FeasibilityBackendComparison>,
) -> Phase05GateEvaluation {
    const VALID_SLOT_DENSITY_GATE_MIN: f64 = 0.25;
    const INVALID_SLOT_RATE_GATE_MAX: f64 = 0.75;
    const CACHE_REUSE_GATE_MIN: f64 = 0.90;

    const GPU_SPEEDUP_GATE_MIN: f64 = 2.75;

    let structural_cache_reuse_observed = warm_runs
        .last()
        .and_then(|run| cache_reuse_ratio(run.structural_cache_hits, run.structural_cache_misses));
    let summary_cache_reuse_observed = warm_runs
        .last()
        .and_then(|run| cache_reuse_ratio(run.summary_cache_hits, run.summary_cache_misses));
    let structural_cache_reuse_pass =
        structural_cache_reuse_observed.map(|value| value >= CACHE_REUSE_GATE_MIN);
    let summary_cache_reuse_pass =
        summary_cache_reuse_observed.map(|value| value >= CACHE_REUSE_GATE_MIN);

    let gpu_speedup_observed = backend_comparison
        .and_then(|comparison| comparison.hybrid_ranking_speedup_vs_cpu)
        .or_else(|| backend_comparison.and_then(|comparison| comparison.scorer_only_speedup_vs_cpu))
        .or(response.report.timings.gpu_speedup_vs_cpu);
    let gpu_speedup_pass = gpu_speedup_observed.map(|value| value >= GPU_SPEEDUP_GATE_MIN);

    let scorer_backend_used_for_share = if response.report.best_available_backend.contains("gpu") {
        response.report.best_available_backend.trim().to_string()
    } else {
        "cpu_reference".to_string()
    };
    let scorer_rank_ms = if scorer_backend_used_for_share.contains("gpu") {
        response.report.timings.gpu_ranking_ms
    } else {
        Some(response.report.timings.cpu_ranking_ms)
    };
    let scorer_share_of_preview_observed = scorer_rank_ms.and_then(|rank_ms| {
        if response.total_preview_ms == 0 {
            None
        } else {
            Some(rank_ms as f64 / response.total_preview_ms as f64)
        }
    });

    let mut notes = Vec::new();
    if warm_runs.is_empty() {
        notes.push(
            "run with --warm-repeats >= 1 to measure the documented immediate-rerun cache reuse gates"
                .to_string(),
        );
    }
    if gpu_speedup_observed.is_none() {
        notes.push(
            "GPU speedup is unavailable on this run; use a CUDA build plus imported options data to measure the product-promotion gate"
                .to_string(),
        );
    } else if backend_comparison.is_none() {
        notes.push(
            "GPU speedup currently comes from the single-preview hybrid ranking timing; rerun with --compare-backends for repeated hybrid-ranking medians before treating the Phase 0.5 gate as closed"
                .to_string(),
        );
    }
    if scorer_share_of_preview_observed.is_none() {
        notes.push(
            "scorer share of preview time could not be estimated because rank timing was unavailable"
                .to_string(),
        );
    }

    Phase05GateEvaluation {
        valid_slot_density_gate_min: VALID_SLOT_DENSITY_GATE_MIN,
        valid_slot_density_observed: response.report.candidate_density,
        valid_slot_density_pass: response.report.candidate_density >= VALID_SLOT_DENSITY_GATE_MIN,
        invalid_slot_rate_gate_max: INVALID_SLOT_RATE_GATE_MAX,
        invalid_slot_rate_observed: response.report.invalid_slot_rate,
        invalid_slot_rate_pass: response.report.invalid_slot_rate <= INVALID_SLOT_RATE_GATE_MAX,
        cache_reuse_gate_min: CACHE_REUSE_GATE_MIN,
        structural_cache_reuse_observed,
        structural_cache_reuse_pass,
        summary_cache_reuse_observed,
        summary_cache_reuse_pass,
        gpu_speedup_gate_min: GPU_SPEEDUP_GATE_MIN,
        gpu_speedup_observed,
        gpu_speedup_pass,
        scorer_share_of_preview_observed,
        scorer_backend_used_for_share,
        notes,
    }
}

fn push_unique_note(out: &mut Vec<String>, note: impl Into<String>) {
    let note = note.into();
    if !out.iter().any(|existing| existing == &note) {
        out.push(note);
    }
}

fn push_event_overlap_recommendations(
    out: &mut Vec<String>,
    diagnostics: &OptionsExpressionDiagnostics,
    event_coverage: &OptionsExpressionFeasibilityEventCoverage,
    contract_not_eligible_share: Option<f64>,
) {
    let search_missing =
        diagnostics.search_events > 0 && event_coverage.search_events_with_candidates == 0;
    let holdout_missing =
        diagnostics.holdout_events > 0 && event_coverage.holdout_events_with_candidates == 0;
    let sealed_has_overlap =
        diagnostics.sealed_test_events > 0 && event_coverage.sealed_test_events_with_candidates > 0;
    let contract_not_eligible_share = contract_not_eligible_share.unwrap_or(0.0);

    if !(search_missing || holdout_missing) || contract_not_eligible_share < 0.50 {
        return;
    }

    if search_missing && holdout_missing && sealed_has_overlap {
        push_unique_note(
            out,
            format!(
                "`contract_not_eligible_as_of_event` accounts for {:.1}% of invalid slots, and the current dataset only overlaps sealed/test events. Match the provider download window to the parent artifact's search/holdout dates before treating this slice as representative.",
                contract_not_eligible_share * 100.0
            ),
        );
        return;
    }

    push_unique_note(
        out,
        format!(
            "`contract_not_eligible_as_of_event` accounts for {:.1}% of invalid slots. The downloaded contract universe is not lining up with the parent artifact's tradable event window; recheck the provider date range, DTE window, and strike band before widening the GPU tuple grid.",
            contract_not_eligible_share * 100.0
        ),
    );
}

fn build_quality_recommendations(
    output: &AnalyzeOptionsExpressionFeasibilityResponse,
    phase05_gates: &Phase05GateEvaluation,
    backend_comparison: Option<&FeasibilityBackendComparison>,
) -> Vec<String> {
    let report = &output.report;
    let mut notes = Vec::new();

    if !phase05_gates.valid_slot_density_pass || !phase05_gates.invalid_slot_rate_pass {
        push_unique_note(
            &mut notes,
            format!(
                "The current representative grid is still quality-bound rather than scorer-bound: candidate density is {:.1}% and invalid-slot rate is {:.1}%. Narrow the search space before promoting GPU as the default backend.",
                report.candidate_density * 100.0,
                report.invalid_slot_rate * 100.0
            ),
        );
    }

    let total_invalid = report.invalid_slot_count.max(1) as f64;
    let mut contract_not_eligible_share = None;
    for reason in &report.invalid_reasons {
        let share = reason.count as f64 / total_invalid;
        if share < 0.10 {
            continue;
        }
        if matches!(
            reason.reason,
            OptionsCandidateInvalidReason::ContractNotEligibleAsOfEvent
        ) {
            contract_not_eligible_share = Some(share);
        }
        match reason.reason {
            OptionsCandidateInvalidReason::NoExpirationInBucket => push_unique_note(
                &mut notes,
                format!(
                    "`no_expiration_in_bucket` accounts for {:.1}% of invalid slots. Reduce the default DTE bucket spread for this dataset, or download a wider expiration window before treating the large-grid result as representative.",
                    share * 100.0
                ),
            ),
            OptionsCandidateInvalidReason::NoStrikeInTolerance => push_unique_note(
                &mut notes,
                format!(
                    "`no_strike_in_tolerance` accounts for {:.1}% of invalid slots. Re-center the strike window closer to spot or relax the moneyness selector tolerance before widening the tuple grid.",
                    share * 100.0
                ),
            ),
            OptionsCandidateInvalidReason::EntryPastCutoff => push_unique_note(
                &mut notes,
                format!(
                    "`entry_past_cutoff` accounts for {:.1}% of invalid slots. Trim late delay buckets, lower max entry lag, or avoid using late-session parent signals for this options profile.",
                    share * 100.0
                ),
            ),
            OptionsCandidateInvalidReason::NoPriceRowAfterEntry
            | OptionsCandidateInvalidReason::StaleOrMissingPath
            | OptionsCandidateInvalidReason::InsufficientFuturePath => push_unique_note(
                &mut notes,
                format!(
                    "`{:?}` accounts for {:.1}% of invalid slots. Check dataset coverage continuity before widening the search grid.",
                    reason.reason,
                    share * 100.0
                ),
            ),
            _ => {}
        }
    }
    push_event_overlap_recommendations(
        &mut notes,
        &report.diagnostics,
        &report.event_coverage,
        contract_not_eligible_share,
    );

    if let Some(comparison) = backend_comparison {
        if let Some(speedup) = comparison.hybrid_ranking_speedup_vs_cpu {
            if speedup >= phase05_gates.gpu_speedup_gate_min {
                push_unique_note(
                    &mut notes,
                    format!(
                        "GPU hybrid ranking is now materially faster on this workload ({speedup:.2}x median vs CPU). The next blocker is grid quality, not scorer throughput."
                    ),
                );
            }
        }
        if let Some(speedup) = comparison.estimated_backend_preview_wall_speedup_vs_cpu {
            if speedup > 1.0 {
                push_unique_note(
                    &mut notes,
                    format!(
                        "Estimated backend-only preview wall time now favors the accelerated path ({speedup:.2}x vs CPU), but the large-grid defaults still need denser candidate coverage before product promotion."
                    ),
                );
            }
        }
    }

    notes
}

#[cfg(test)]
mod tests {
    use super::push_event_overlap_recommendations;
    use ta_app_core::options_types::OptionsExpressionFeasibilityEventCoverage;
    use ta_app_core::OptionsExpressionDiagnostics;

    fn sample_diagnostics() -> OptionsExpressionDiagnostics {
        OptionsExpressionDiagnostics {
            parent_artifact_path: "artifact.json".to_string(),
            options_dataset_id: "dataset".to_string(),
            root_symbol: "QQQ".to_string(),
            total_events: 100,
            bullish_events: 100,
            bearish_events: 0,
            search_events: 60,
            holdout_events: 20,
            sealed_test_events: 20,
            dataset_contracts: 25,
            dataset_rows: 1000,
            coverage_start: None,
            coverage_end: None,
        }
    }

    #[test]
    fn overlap_recommendation_flags_search_holdout_mismatch_when_only_sealed_has_candidates() {
        let diagnostics = sample_diagnostics();
        let coverage = OptionsExpressionFeasibilityEventCoverage {
            total_events_with_candidates: 5,
            search_events_with_candidates: 0,
            holdout_events_with_candidates: 0,
            sealed_test_events_with_candidates: 5,
            total_events_with_baseline_outcomes: 5,
            search_events_with_baseline_outcomes: 0,
            holdout_events_with_baseline_outcomes: 0,
            sealed_test_events_with_baseline_outcomes: 5,
            total_events_with_conservative_outcomes: 5,
            search_events_with_conservative_outcomes: 0,
            holdout_events_with_conservative_outcomes: 0,
            sealed_test_events_with_conservative_outcomes: 5,
            total_events_with_harsher_outcomes: 5,
            search_events_with_harsher_outcomes: 0,
            holdout_events_with_harsher_outcomes: 0,
            sealed_test_events_with_harsher_outcomes: 5,
        };
        let mut notes = Vec::new();
        push_event_overlap_recommendations(&mut notes, &diagnostics, &coverage, Some(0.92));
        assert!(notes
            .iter()
            .any(|note| note.contains("only overlaps sealed/test events")));
    }

    #[test]
    fn overlap_recommendation_stays_quiet_when_contract_eligibility_is_not_dominant() {
        let diagnostics = sample_diagnostics();
        let coverage = OptionsExpressionFeasibilityEventCoverage::default();
        let mut notes = Vec::new();
        push_event_overlap_recommendations(&mut notes, &diagnostics, &coverage, Some(0.25));
        assert!(notes.is_empty());
    }
}

fn build_backend_comparison(
    request: &OptionsExpressionRunRequest,
    app_local_data_dir: &Path,
    iterations: usize,
    include_full_run: bool,
) -> Result<FeasibilityBackendComparison, String> {
    let accelerated_backend_preference = accelerated_backend_target(request.backend_preference);
    let cpu_reference_preview = benchmark_preview(
        request.clone(),
        app_local_data_dir,
        OptionsExpressionBackendPreference::CpuReference,
        iterations,
    )?;
    let accelerated_preview = benchmark_preview(
        request.clone(),
        app_local_data_dir,
        accelerated_backend_preference,
        iterations,
    )?;
    let cpu_reference_run = if include_full_run {
        Some(benchmark_run(
            request.clone(),
            app_local_data_dir,
            OptionsExpressionBackendPreference::CpuReference,
            iterations,
        )?)
    } else {
        None
    };
    let accelerated_run = if include_full_run {
        Some(benchmark_run(
            request.clone(),
            app_local_data_dir,
            accelerated_backend_preference,
            iterations,
        )?)
    } else {
        None
    };

    let mut notes = Vec::new();
    if accelerated_preview.best_available_backend == cpu_reference_preview.best_available_backend {
        notes.push(
            "accelerated backend comparison resolved to the same backend as cpu_reference; verify a CUDA build/runtime is available before treating the speedup as meaningful"
                .to_string(),
        );
    }
    if accelerated_backend_preference == OptionsExpressionBackendPreference::Auto {
        notes.push(
            "accelerated backend preference used auto-selection so the preview/full-run target reflects the best available backend on this machine"
                .to_string(),
        );
    }
    let hybrid_ranking_speedup_vs_cpu = observed_speedup(
        Some(cpu_reference_preview.median_ranking_us),
        Some(accelerated_preview.median_ranking_us),
    );
    if let Some(speedup) = hybrid_ranking_speedup_vs_cpu {
        if speedup <= 1.0 {
            notes.push(
                "repeated hybrid-ranking medians still favor cpu_reference for this workload; do not treat the accelerated route as the recommended backend yet"
                    .to_string(),
            );
        }
    }

    let estimated_backend_preview_wall_speedup_vs_cpu = observed_speedup(
        Some(cpu_reference_preview.median_estimated_backend_preview_ms),
        Some(accelerated_preview.median_estimated_backend_preview_ms),
    );

    Ok(FeasibilityBackendComparison {
        accelerated_backend_preference: format!("{accelerated_backend_preference:?}"),
        estimated_backend_preview_wall_speedup_vs_cpu,
        preview_wall_speedup_vs_cpu: estimated_backend_preview_wall_speedup_vs_cpu,
        hybrid_ranking_speedup_vs_cpu,
        scorer_only_speedup_vs_cpu: hybrid_ranking_speedup_vs_cpu,
        full_run_wall_speedup_vs_cpu: observed_speedup(
            cpu_reference_run
                .as_ref()
                .map(|summary| summary.median_wall_ms),
            accelerated_run
                .as_ref()
                .map(|summary| summary.median_wall_ms),
        ),
        cpu_reference_preview,
        accelerated_preview,
        cpu_reference_run,
        accelerated_run,
        notes,
    })
}

fn render_markdown_summary(output: &FeasibilityHarnessOutput) -> String {
    let mut out = String::new();
    let _ = writeln!(&mut out, "# Options Expression Phase 0.5 Summary");
    let _ = writeln!(&mut out);
    let _ = writeln!(&mut out, "- Artifact: `{}`", output.artifact_path);
    let _ = writeln!(
        &mut out,
        "- Split assignment: `{}`",
        output.request_split_assignment_mode
    );
    let _ = writeln!(
        &mut out,
        "- Dataset: `{}` (`{}` / {:?})",
        output.dataset_manifest.options_dataset_id,
        output.dataset_manifest.root_symbol,
        output.dataset_manifest.price_semantics
    );
    let _ = writeln!(
        &mut out,
        "- Coverage: `{}` -> `{}`",
        output.dataset_manifest.coverage_start, output.dataset_manifest.coverage_end
    );
    let _ = writeln!(
        &mut out,
        "- Preview backend: `{}`",
        output.response.report.best_available_backend
    );
    if !output
        .response
        .report
        .auto_backend_decision
        .recommended_backend
        .trim()
        .is_empty()
    {
        let _ = writeln!(
            &mut out,
            "- Auto recommendation: `{}`",
            output
                .response
                .report
                .auto_backend_decision
                .recommended_backend
        );
    }
    if !output
        .response
        .report
        .cache_stats
        .gpu_memory_budget_source
        .trim()
        .is_empty()
    {
        let _ = writeln!(
            &mut out,
            "- GPU planner: `{}` / outcome {} / chunk {}",
            output.response.report.cache_stats.gpu_memory_budget_source,
            if output
                .response
                .report
                .cache_stats
                .gpu_outcome_tensor_fits_in_memory
            {
                "fits"
            } else {
                "does not fit"
            },
            if output
                .response
                .report
                .cache_stats
                .gpu_chunk_plan_fits_in_memory
            {
                "fits"
            } else {
                "does not fit"
            }
        );
    }
    if let Some(replay_timing) = output.response.replay_build_timing.as_ref() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Replay/build timing");
        let _ = writeln!(
            &mut out,
            "- Total replay/build: {}",
            format_millis_from_us(replay_timing.total_us)
        );
        let _ = writeln!(
            &mut out,
            "- Parent artifact validation: {}",
            format_millis_from_us(replay_timing.parent_artifact_validation_us)
        );
        let _ = writeln!(
            &mut out,
            "- Source resolution: {}",
            format_millis_from_us(replay_timing.source_resolution_us)
        );
        let _ = writeln!(
            &mut out,
            "- Parent-signal process cache lookup: {} (hit: `{}`)",
            format_millis_from_us(replay_timing.parent_signal_process_cache_lookup_us),
            replay_timing.parent_signal_process_cache_hit
        );
        let _ = writeln!(
            &mut out,
            "- Persisted signal-manifest cache lookup: {} (hit: `{}`)",
            format_millis_from_us(replay_timing.persisted_signal_manifest_cache_lookup_us),
            replay_timing.persisted_signal_manifest_cache_hit
        );
        let _ = writeln!(
            &mut out,
            "- Candle replay load/resample: {}",
            format_millis_from_us(replay_timing.candle_replay_load_us)
        );
        let _ = writeln!(
            &mut out,
            "- Parent replay + candidate detail: {}",
            format_millis_from_us(replay_timing.parent_replay_and_drilldown_us)
        );
        let _ = writeln!(
            &mut out,
            "- Signal-manifest materialization: {}",
            format_millis_from_us(replay_timing.signal_manifest_materialization_us)
        );
        let _ = writeln!(
            &mut out,
            "- Options dataset manifest validation: {}",
            format_millis_from_us(replay_timing.options_dataset_manifest_validation_us)
        );
        let _ = writeln!(
            &mut out,
            "- Options dataset process cache lookup: {} (hit: `{}`)",
            format_millis_from_us(replay_timing.options_dataset_process_cache_lookup_us),
            replay_timing.options_dataset_process_cache_hit
        );
        let _ = writeln!(
            &mut out,
            "- Persisted options dataset cache lookup: {} (hit: `{}`)",
            format_millis_from_us(replay_timing.options_dataset_persisted_cache_lookup_us),
            replay_timing.options_dataset_persisted_cache_hit
        );
        let _ = writeln!(
            &mut out,
            "- Options dataset CSV hash/load: {}",
            format_millis_from_us(replay_timing.options_dataset_csv_load_us)
        );
        let _ = writeln!(
            &mut out,
            "- Request normalization: {}",
            format_millis_from_us(replay_timing.request_normalization_us)
        );
    }
    let _ = writeln!(&mut out);
    let _ = writeln!(&mut out, "## Phase 0.5 gates");
    let _ = writeln!(
        &mut out,
        "- Candidate density: {:.4} (gate >= {:.4}) {}",
        output.phase05_gates.valid_slot_density_observed,
        output.phase05_gates.valid_slot_density_gate_min,
        if output.phase05_gates.valid_slot_density_pass {
            "PASS"
        } else {
            "FAIL"
        }
    );
    let _ = writeln!(
        &mut out,
        "- Invalid slot rate: {:.4} (gate <= {:.4}) {}",
        output.phase05_gates.invalid_slot_rate_observed,
        output.phase05_gates.invalid_slot_rate_gate_max,
        if output.phase05_gates.invalid_slot_rate_pass {
            "PASS"
        } else {
            "FAIL"
        }
    );
    let structural_cache = output
        .phase05_gates
        .structural_cache_reuse_observed
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "n/a".to_string());
    let summary_cache = output
        .phase05_gates
        .summary_cache_reuse_observed
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "n/a".to_string());
    let _ = writeln!(
        &mut out,
        "- Structural cache reuse: {} (gate >= {:.4})",
        structural_cache, output.phase05_gates.cache_reuse_gate_min
    );
    let _ = writeln!(
        &mut out,
        "- Summary cache reuse: {} (gate >= {:.4})",
        summary_cache, output.phase05_gates.cache_reuse_gate_min
    );
    let gpu_speedup = output
        .phase05_gates
        .gpu_speedup_observed
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "n/a".to_string());
    let _ = writeln!(
        &mut out,
        "- GPU hybrid-ranking speedup vs CPU: {} (gate >= {:.4})",
        gpu_speedup, output.phase05_gates.gpu_speedup_gate_min
    );
    if !output.phase05_gates.notes.is_empty() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "Notes:");
        for note in &output.phase05_gates.notes {
            let _ = writeln!(&mut out, "- {}", note);
        }
    }
    if let Some(comparison) = output.backend_comparison.as_ref() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Backend comparison");
        let _ = writeln!(
            &mut out,
            "- CPU preview median wall ms: {} (estimated backend-only `{}` ms)",
            comparison.cpu_reference_preview.median_wall_ms,
            comparison
                .cpu_reference_preview
                .median_estimated_backend_preview_ms
        );
        let _ = writeln!(
            &mut out,
            "- CPU hybrid-ranking median ms: {}",
            format_millis_from_us(comparison.cpu_reference_preview.median_ranking_us)
        );
        let _ = writeln!(
            &mut out,
            "- Accelerated preview median wall ms: {} (estimated backend-only `{}` ms) (`{}` / `{}`)",
            comparison.accelerated_preview.median_wall_ms,
            comparison.accelerated_preview.median_estimated_backend_preview_ms,
            comparison.accelerated_backend_preference,
            comparison.accelerated_preview.best_available_backend
        );
        let _ = writeln!(
            &mut out,
            "- Accelerated hybrid-ranking median ms: {} (`{}`)",
            format_millis_from_us(comparison.accelerated_preview.median_ranking_us),
            comparison.accelerated_preview.best_available_backend
        );
        if !comparison
            .accelerated_preview
            .auto_backend_recommended_backend
            .trim()
            .is_empty()
        {
            let _ = writeln!(
                &mut out,
                "- Accelerated auto recommendation: `{}`",
                comparison
                    .accelerated_preview
                    .auto_backend_recommended_backend
            );
        }
        let preview_speedup = comparison
            .estimated_backend_preview_wall_speedup_vs_cpu
            .or(comparison.preview_wall_speedup_vs_cpu)
            .map(|value| format!("{value:.4}"))
            .unwrap_or_else(|| "n/a".to_string());
        let _ = writeln!(
            &mut out,
            "- Estimated backend preview wall speedup vs CPU: {}",
            preview_speedup
        );
        let scorer_speedup = comparison
            .hybrid_ranking_speedup_vs_cpu
            .or(comparison.scorer_only_speedup_vs_cpu)
            .map(|value| format!("{value:.4}"))
            .unwrap_or_else(|| "n/a".to_string());
        let _ = writeln!(
            &mut out,
            "- Hybrid-ranking speedup vs CPU: {}",
            scorer_speedup
        );
        if let (Some(cpu_run), Some(accelerated_run)) = (
            comparison.cpu_reference_run.as_ref(),
            comparison.accelerated_run.as_ref(),
        ) {
            let _ = writeln!(
                &mut out,
                "- CPU full-run median wall ms: {}",
                cpu_run.median_wall_ms
            );
            let _ = writeln!(
                &mut out,
                "- Accelerated full-run median wall ms: {} (`{}`)",
                accelerated_run.median_wall_ms, accelerated_run.backend_used
            );
            if let Some(reason) = accelerated_run.backend_fallback_reason.as_ref() {
                if !reason.trim().is_empty() {
                    let _ = writeln!(&mut out, "- Accelerated full-run fallback: `{}`", reason);
                }
            }
            let full_speedup = comparison
                .full_run_wall_speedup_vs_cpu
                .map(|value| format!("{value:.4}"))
                .unwrap_or_else(|| "n/a".to_string());
            let _ = writeln!(&mut out, "- Full-run wall speedup vs CPU: {}", full_speedup);
        }
        if !comparison.notes.is_empty() {
            let _ = writeln!(&mut out);
            let _ = writeln!(&mut out, "Comparison notes:");
            for note in &comparison.notes {
                let _ = writeln!(&mut out, "- {}", note);
            }
        }
        if !comparison.accelerated_preview.gpu_plan_notes.is_empty() {
            let _ = writeln!(&mut out);
            let _ = writeln!(&mut out, "Accelerated planner notes:");
            for note in &comparison.accelerated_preview.gpu_plan_notes {
                let _ = writeln!(&mut out, "- {}", note);
            }
        }
    }
    if !output.recommendations.is_empty() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Recommendations");
        for note in &output.recommendations {
            let _ = writeln!(&mut out, "- {}", note);
        }
    }
    if let Some(run_summary) = output.run_summary.as_ref() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Full run");
        let _ = writeln!(&mut out, "- Artifact: `{}`", run_summary.artifact_path);
        let _ = writeln!(&mut out, "- Backend used: `{}`", run_summary.backend_used);
        if let Some(reason) = run_summary.backend_fallback_reason.as_ref() {
            if !reason.trim().is_empty() {
                let _ = writeln!(&mut out, "- Backend fallback: `{}`", reason);
            }
        }
        let _ = writeln!(&mut out, "- Tuple count: `{}`", run_summary.tuple_count);
        if let Some(top_tuple) = run_summary.top_tuple.as_ref() {
            let _ = writeln!(
                &mut out,
                "- Top tuple robust score: `{:.6}`",
                top_tuple.robust_score
            );
            let _ = writeln!(
                &mut out,
                "- Top tuple selection EV / premium: `{:.6}`",
                top_tuple.selection_metrics.expectancy_per_premium
            );
        }
    }
    out
}

fn summarize_run(response: RunOptionsExpressionResponse) -> RunSummary {
    RunSummary {
        artifact_path: response.artifact_path,
        backend_used: response.result.backend_used,
        backend_fallback_reason: response.result.backend_fallback_reason,
        tuple_count: response.result.tuple_count,
        top_tuple: response.result.ranked_results.first().cloned(),
    }
}

fn parse_u16_list(raw: &str) -> Result<Vec<u16>, String> {
    let out: Vec<u16> = raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .parse::<u16>()
                .map_err(|e| format!("invalid integer '{}': {e}", entry))
        })
        .collect::<Result<_, _>>()?;
    if out.is_empty() {
        return Err("list must not be empty".to_string());
    }
    Ok(out)
}

fn parse_f64_list(raw: &str) -> Result<Vec<f64>, String> {
    let out: Vec<f64> = raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .parse::<f64>()
                .map_err(|e| format!("invalid decimal '{}': {e}", entry))
        })
        .collect::<Result<_, _>>()?;
    if out.is_empty() {
        return Err("list must not be empty".to_string());
    }
    Ok(out)
}

fn parse_moneyness_buckets(raw: &str) -> Result<Vec<OptionsMoneynessBucket>, String> {
    let out: Vec<OptionsMoneynessBucket> = raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| match entry.to_ascii_lowercase().as_str() {
            "atm" => Ok(OptionsMoneynessBucket::Atm),
            "otm1" | "otm_1" => Ok(OptionsMoneynessBucket::Otm1),
            "otm2" | "otm_2" => Ok(OptionsMoneynessBucket::Otm2),
            "itm1" | "itm_1" => Ok(OptionsMoneynessBucket::Itm1),
            other => Err(format!("unsupported moneyness bucket '{}'", other)),
        })
        .collect::<Result<_, _>>()?;
    if out.is_empty() {
        return Err("moneyness bucket list must not be empty".to_string());
    }
    Ok(out)
}

fn parse_exit_template(raw: &str) -> Result<OptionsExitTemplateFamily, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fixed_hold" => Ok(OptionsExitTemplateFamily::FixedHold),
        "fixed_hold_or_target_stop" => Ok(OptionsExitTemplateFamily::FixedHoldOrTargetStop),
        "end_of_day_or_target_stop" => Ok(OptionsExitTemplateFamily::EndOfDayOrTargetStop),
        "base_exit_or_target_stop" => Ok(OptionsExitTemplateFamily::BaseExitOrTargetStop),
        other => Err(format!("unsupported exit_template '{}'", other)),
    }
}

fn parse_fill_model(raw: &str) -> Result<OptionsFillModelFamily, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "aggregate_research" => Ok(OptionsFillModelFamily::AggregateResearch),
        "quote_conservative" => Ok(OptionsFillModelFamily::QuoteConservative),
        "quote_mid" => Ok(OptionsFillModelFamily::QuoteMid),
        other => Err(format!("unsupported fill_model '{}'", other)),
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

fn runs_dir(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join("runs")
}

fn options_datasets_dir(app_local_data_dir: &Path) -> Result<PathBuf, String> {
    let market_root = settings::resolve_market_data_root_dir(app_local_data_dir)?;
    Ok(market_root.join("options_research").join("datasets"))
}

fn latest_run_artifact_path(runs_dir: &Path) -> Result<PathBuf, String> {
    let mut latest: Option<(PathBuf, Option<std::time::SystemTime>)> = None;
    for entry in fs::read_dir(runs_dir)
        .map_err(|e| format!("failed to read runs dir {}: {e}", runs_dir.display()))?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file()
            || !path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|name| name.starts_with("vectorgrid_run_") && name.ends_with(".json"))
                .unwrap_or(false)
        {
            continue;
        }

        let modified = entry.metadata().and_then(|meta| meta.modified()).ok();
        match &latest {
            None => latest = Some((path, modified)),
            Some((_, latest_modified)) if modified >= *latest_modified => {
                latest = Some((path, modified));
            }
            _ => {}
        }
    }
    latest
        .map(|(path, _)| path)
        .ok_or_else(|| format!("no saved run artifacts found in {}", runs_dir.display()))
}

fn normalize_symbol_root(raw: &str) -> String {
    raw.trim()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .to_ascii_uppercase()
}

fn derive_symbol_root_from_csv_path(csv_path: &str) -> String {
    let stem = Path::new(csv_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("UNKNOWN");
    let upper = stem.to_ascii_uppercase();
    let stripped = [
        "_1M",
        "_5M",
        "_15M",
        "_30M",
        "_60M",
        "_1H",
        "_4H",
        "_1D",
        "_1W",
        "_DAILY",
        "_RAW",
        "_ADJUSTED",
    ]
    .iter()
    .find_map(|suffix| upper.strip_suffix(suffix))
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .unwrap_or(upper.as_str());
    normalize_symbol_root(stripped)
}

fn infer_run_root_symbol(artifact_path: &Path) -> Result<Option<String>, String> {
    let file = File::open(artifact_path).map_err(|e| {
        format!(
            "failed to read run artifact {}: {e}",
            artifact_path.display()
        )
    })?;
    let value: Value = serde_json::from_reader(BufReader::new(file)).map_err(|e| {
        format!(
            "failed to parse run artifact {}: {e}",
            artifact_path.display()
        )
    })?;
    let pointers = [
        "/dataset_meta/display_symbol",
        "/dataset_meta/symbol",
        "/data/display_symbol",
        "/data/symbol",
        "/run/req/symbol",
    ];
    if let Some(symbol) = pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(|entry| entry.as_str()))
        .map(normalize_symbol_root)
        .filter(|entry| !entry.is_empty())
    {
        return Ok(Some(symbol));
    }
    let csv_path = value
        .get("csv_path")
        .and_then(|entry| entry.as_str())
        .or_else(|| {
            value
                .pointer("/data/csv_path")
                .and_then(|entry| entry.as_str())
        })
        .or_else(|| {
            value
                .pointer("/dataset_meta/csv_path")
                .and_then(|entry| entry.as_str())
        })
        .or_else(|| {
            value
                .pointer("/validation/req/validation_context/csv_path")
                .and_then(|entry| entry.as_str())
        });
    Ok(csv_path
        .map(derive_symbol_root_from_csv_path)
        .filter(|entry| !entry.is_empty()))
}

fn select_dataset_manifest(
    manifests: &[OptionsDatasetManifestV1],
    dataset_id: Option<&str>,
    root_hint: Option<&str>,
) -> Result<OptionsDatasetManifestV1, String> {
    if let Some(explicit) = dataset_id.map(str::trim).filter(|value| !value.is_empty()) {
        return manifests
            .iter()
            .find(|entry| entry.options_dataset_id == explicit)
            .cloned()
            .ok_or_else(|| format!("Polygon options dataset '{}' is not imported", explicit));
    }

    if let Some(root) = root_hint
        .map(normalize_symbol_root)
        .filter(|value| !value.is_empty())
    {
        let matches: Vec<_> = manifests
            .iter()
            .filter(|entry| normalize_symbol_root(&entry.root_symbol) == root)
            .cloned()
            .collect();
        if matches.len() == 1 {
            return Ok(matches[0].clone());
        }
        if matches.len() > 1 {
            return Err(format!(
                "multiple imported Polygon options datasets match root '{}'; pass --dataset-id explicitly",
                root
            ));
        }
    }

    if manifests.len() == 1 {
        return Ok(manifests[0].clone());
    }

    Err(
        "unable to select a Polygon options dataset automatically; pass --dataset-id explicitly"
            .to_string(),
    )
}

fn build_request(
    cli: &Cli,
    artifact_path: &Path,
    dataset_id: &str,
) -> Result<OptionsExpressionRunRequest, String> {
    let top_k = cli.top_k.max(1);
    Ok(OptionsExpressionRunRequest {
        parent_artifact_path: artifact_path.to_string_lossy().to_string(),
        parent_candidate_index: None,
        options_dataset_id: dataset_id.to_string(),
        structure_family: ta_app_core::options_types::OptionsStructureFamily::LongSingle,
        backend_preference: parse_backend_preference(&cli.backend_preference)?,
        objective_config: Default::default(),
        delay_minutes: parse_u16_list(&cli.delay_minutes)?,
        dte_buckets: parse_u16_list(&cli.dte_buckets)?,
        moneyness_buckets: parse_moneyness_buckets(&cli.moneyness_buckets)?,
        hold_minutes: parse_u16_list(&cli.hold_minutes)?,
        target_return_buckets: parse_f64_list(&cli.target_return_buckets)?,
        stop_return_buckets: parse_f64_list(&cli.stop_return_buckets)?,
        exit_template: parse_exit_template(&cli.exit_template)?,
        fill_model: parse_fill_model(&cli.fill_model)?,
        filters: OptionsStaticFilterBundle {
            max_entry_lag_minutes: Some(cli.max_entry_lag_minutes),
            max_premium: cli.max_premium,
            min_premium: cli.min_premium,
            min_volume: cli.min_volume,
            min_open_interest: cli.min_open_interest,
            max_quote_age_ms: None,
            max_spread_bps: None,
        },
        top_k,
        min_valid_trades: cli.min_valid_trades,
        split_ratio_search: cli.split_ratio_search,
        split_ratio_holdout: cli.split_ratio_holdout,
        split_assignment_mode: parse_split_assignment_mode(&cli.split_assignment_mode)?,
        sealed_finalist_count: cli.sealed_finalist_count.max(1).min(top_k),
    })
}

fn parse_split_assignment_mode(raw: &str) -> Result<OptionsSplitAssignmentMode, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "event_chronological" | "event" | "events" => {
            Ok(OptionsSplitAssignmentMode::EventChronological)
        }
        "session_chronological" | "session" | "sessions" => {
            Ok(OptionsSplitAssignmentMode::SessionChronological)
        }
        other => Err(format!(
            "split_assignment_mode must be one of: event_chronological, session_chronological (got '{other}')"
        )),
    }
}

fn main() -> Result<(), String> {
    let _benchmark_guard = benchmark_lock::acquire("ta_options_expression_feasibility")?;
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

    let imported_manifest = if let Some(import_csv) = cli.import_csv.as_ref() {
        Some(import_options_dataset_blocking_headless(
            OptionsDatasetImportRequest {
                csv_path: import_csv.to_string_lossy().to_string(),
                options_dataset_id: cli.dataset_id.clone(),
                root_symbol: cli.import_root_symbol.clone(),
                provider_name: "polygon".to_string(),
                provider_source_surface: "minute_aggregate".to_string(),
                provider_entitlement_profile: None,
                quote_resolution: "1m".to_string(),
                price_semantics: ta_app_core::OptionsPriceSemantics::MinuteAggregate,
                calendar_id: "market_hours_utc".to_string(),
                timezone: "UTC".to_string(),
                normalization_version: None,
                quote_semantics_version: None,
                corporate_action_mapping_version: None,
                contract_reference_source: "polygon_contract_reference".to_string(),
                contract_reference_as_of_utc: None,
                underlier_reference_source: "underlying_dataset".to_string(),
                aggregate_price_field: Some("close".to_string()),
            },
            &resolved_app_local_data_dir,
        )?)
    } else {
        None
    };
    let datasets_dir = options_datasets_dir(&resolved_app_local_data_dir)?;
    let manifests = if datasets_dir.is_dir() {
        scan_options_dataset_manifests(&datasets_dir)?
    } else {
        Vec::new()
    };
    if cli.list_datasets {
        let txt = serde_json::to_string_pretty(&manifests).map_err(|e| e.to_string())?;
        println!("{txt}");
        return Ok(());
    }

    let artifact_path = if let Some(path) = cli.artifact.clone() {
        path
    } else {
        latest_run_artifact_path(&runs_dir(&resolved_app_local_data_dir))?
    };
    if !artifact_path.is_file() {
        return Err(format!(
            "run artifact does not exist: {}",
            artifact_path.display()
        ));
    }
    let root_hint = infer_run_root_symbol(&artifact_path)?;
    let dataset_manifest = if let Some(imported) = imported_manifest {
        imported
    } else {
        select_dataset_manifest(&manifests, cli.dataset_id.as_deref(), root_hint.as_deref())?
    };
    let request = build_request(&cli, &artifact_path, &dataset_manifest.options_dataset_id)?;
    let benchmark_iterations = cli.benchmark_iterations.max(1);
    maybe_prewarm_gpu_runtime(cli.prewarm_gpu_runtime)?;
    let response = analyze_options_expression_feasibility_blocking_headless(
        request.clone(),
        &resolved_app_local_data_dir,
    )?;
    let mut warm_runs = Vec::with_capacity(cli.warm_repeats);
    for iteration in 0..cli.warm_repeats {
        let warm = analyze_options_expression_feasibility_blocking_headless(
            request.clone(),
            &resolved_app_local_data_dir,
        )?;
        warm_runs.push(WarmPreviewSummary {
            iteration: iteration + 1,
            replay_build_ms: warm.replay_build_ms,
            total_preview_ms: warm.total_preview_ms,
            best_available_backend: warm.report.best_available_backend.clone(),
            auto_backend_recommended_backend: warm
                .report
                .auto_backend_decision
                .recommended_backend
                .clone(),
            auto_backend_reasons: warm.report.auto_backend_decision.reasons.clone(),
            structural_cache_hits: warm.report.cache_stats.structural_cache_hits,
            structural_cache_misses: warm.report.cache_stats.structural_cache_misses,
            summary_cache_hits: warm.report.cache_stats.summary_cache_hits,
            summary_cache_misses: warm.report.cache_stats.summary_cache_misses,
            gpu_memory_budget_source: warm.report.cache_stats.gpu_memory_budget_source.clone(),
            gpu_outcome_tensor_fits_in_memory: warm
                .report
                .cache_stats
                .gpu_outcome_tensor_fits_in_memory,
            gpu_chunk_plan_fits_in_memory: warm.report.cache_stats.gpu_chunk_plan_fits_in_memory,
            gpu_plan_notes: warm.report.cache_stats.gpu_plan_notes.clone(),
        });
    }
    let backend_comparison = if cli.compare_backends {
        Some(build_backend_comparison(
            &request,
            &resolved_app_local_data_dir,
            benchmark_iterations,
            cli.run_full,
        )?)
    } else {
        None
    };
    let run_summary = if cli.run_full {
        Some(summarize_run(run_options_expression_blocking_headless(
            request.clone(),
            &resolved_app_local_data_dir,
        )?))
    } else {
        None
    };
    let top_tuple_inspection = if cli.inspect_top {
        Some(inspect_options_expression_tuple_blocking_headless(
            request.clone(),
            0,
            &resolved_app_local_data_dir,
        )?)
    } else {
        None
    };
    let phase05_gates =
        build_phase05_gate_evaluation(&response, &warm_runs, backend_comparison.as_ref());
    let recommendations =
        build_quality_recommendations(&response, &phase05_gates, backend_comparison.as_ref());
    let output = FeasibilityHarnessOutput {
        app_local_data_dir: resolved_app_local_data_dir.to_string_lossy().to_string(),
        artifact_path: artifact_path.to_string_lossy().to_string(),
        options_dataset_id: dataset_manifest.options_dataset_id.clone(),
        request_split_assignment_mode: serialize_scalar_to_string(&request.split_assignment_mode),
        dataset_manifest: dataset_manifest.clone(),
        response,
        phase05_gates,
        recommendations,
        warm_runs,
        backend_comparison,
        run_summary,
        top_tuple_inspection,
    };
    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    let markdown = cli
        .out_markdown
        .as_ref()
        .map(|_| render_markdown_summary(&output));

    if let Some(out_path) = cli.out_json.as_ref() {
        atomic_output::write_str(out_path, &json)
            .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
    }
    if let Some(out_path) = cli.out_markdown.as_ref() {
        let markdown = markdown
            .as_ref()
            .ok_or_else(|| "failed to build markdown summary".to_string())?;
        atomic_output::write_str(out_path, markdown)
            .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
    }

    println!("{json}");
    Ok(())
}
