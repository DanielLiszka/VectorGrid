use clap::Parser;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use ta_app_core::{OptionsExpressionArtifactV1, OptionsExpressionTupleResult};
use ta_desktop_demo_app::commands::{
    list_options_expression_artifacts_blocking_headless,
    load_options_expression_artifact_blocking_headless, OptionsExpressionArtifactSummary,
};
use tauri::Manager;

#[derive(Parser, Debug)]
#[command(name = "ta_options_expression_compare")]
#[command(about = "Compare two saved options-expression artifacts")]
struct Cli {
    #[arg(long)]
    left: Option<PathBuf>,

    #[arg(long)]
    right: Option<PathBuf>,

    #[arg(long)]
    app_local_data_dir: Option<PathBuf>,

    #[arg(long)]
    out_json: Option<PathBuf>,

    #[arg(long)]
    parent_artifact_id: Option<String>,

    #[arg(long)]
    root_symbol: Option<String>,

    #[arg(long)]
    options_dataset_id: Option<String>,

    #[arg(long, default_value_t = 10)]
    match_limit: usize,

    #[arg(long, default_value_t = false)]
    list_artifacts: bool,
}

#[derive(Debug, Serialize)]
struct ArtifactCompareIdentity {
    path: String,
    ts: String,
    parent_artifact_id: String,
    root_symbol: String,
    options_dataset_id: String,
    backend_used: String,
    backend_fallback_reason: Option<String>,
    price_semantics: String,
    tuple_count: usize,
    total_events: usize,
    search_events: usize,
    holdout_events: usize,
    sealed_test_events: usize,
    best_robust_score: f64,
    best_selection_expectancy_per_premium: f64,
}

#[derive(Debug, Serialize)]
struct TupleMetricsSnapshot {
    tuple_key: String,
    tuple_index: usize,
    robust_score: f64,
    selection_expectancy_per_premium: f64,
    holdout_expectancy_per_premium: Option<f64>,
    sealed_expectancy_per_premium: Option<f64>,
    stress_expectancy_per_premium: Option<f64>,
    sealed_stress_expectancy_per_premium: Option<f64>,
    harsher_stress_expectancy_per_premium: Option<f64>,
    sealed_harsher_stress_expectancy_per_premium: Option<f64>,
    extreme_stress_expectancy_per_premium: Option<f64>,
    sealed_extreme_stress_expectancy_per_premium: Option<f64>,
    selection_valid_trades: usize,
    holdout_valid_trades: Option<usize>,
    sealed_valid_trades: Option<usize>,
    stress_valid_trades: Option<usize>,
    sealed_stress_valid_trades: Option<usize>,
    harsher_stress_valid_trades: Option<usize>,
    sealed_harsher_stress_valid_trades: Option<usize>,
    extreme_stress_valid_trades: Option<usize>,
    sealed_extreme_stress_valid_trades: Option<usize>,
}

#[derive(Debug, Serialize)]
struct MatchedTupleComparison {
    tuple_key: String,
    left: TupleMetricsSnapshot,
    right: TupleMetricsSnapshot,
    robust_score_delta: f64,
    selection_expectancy_delta: f64,
    holdout_expectancy_delta: Option<f64>,
    sealed_expectancy_delta: Option<f64>,
    stress_expectancy_delta: Option<f64>,
    sealed_stress_expectancy_delta: Option<f64>,
    harsher_stress_expectancy_delta: Option<f64>,
    sealed_harsher_stress_expectancy_delta: Option<f64>,
    extreme_stress_expectancy_delta: Option<f64>,
    sealed_extreme_stress_expectancy_delta: Option<f64>,
}

#[derive(Debug, Serialize)]
struct OptionsExpressionCompareOutput {
    app_local_data_dir: String,
    left: ArtifactCompareIdentity,
    right: ArtifactCompareIdentity,
    same_parent_artifact_id: bool,
    same_root_symbol: bool,
    same_options_dataset_id: bool,
    matched_tuple_count: usize,
    left_only_tuple_count: usize,
    right_only_tuple_count: usize,
    top_left_tuple: Option<TupleMetricsSnapshot>,
    top_right_tuple: Option<TupleMetricsSnapshot>,
    matched_tuple_deltas: Vec<MatchedTupleComparison>,
}

fn normalize_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_ascii_uppercase())
}

fn tuple_key(row: &OptionsExpressionTupleResult) -> String {
    serde_json::to_string(&row.tuple).unwrap_or_else(|_| format!("{:?}", row.tuple))
}

fn tuple_snapshot(row: &OptionsExpressionTupleResult, tuple_index: usize) -> TupleMetricsSnapshot {
    TupleMetricsSnapshot {
        tuple_key: tuple_key(row),
        tuple_index,
        robust_score: row.robust_score,
        selection_expectancy_per_premium: row.selection_metrics.expectancy_per_premium,
        holdout_expectancy_per_premium: row
            .holdout_metrics
            .as_ref()
            .map(|metrics| metrics.expectancy_per_premium),
        sealed_expectancy_per_premium: row
            .sealed_evaluation_metrics
            .as_ref()
            .map(|metrics| metrics.expectancy_per_premium),
        stress_expectancy_per_premium: row
            .stress_metrics
            .as_ref()
            .map(|metrics| metrics.expectancy_per_premium),
        sealed_stress_expectancy_per_premium: row
            .sealed_stress_metrics
            .as_ref()
            .map(|metrics| metrics.expectancy_per_premium),
        harsher_stress_expectancy_per_premium: row
            .harsher_stress_metrics
            .as_ref()
            .map(|metrics| metrics.expectancy_per_premium),
        sealed_harsher_stress_expectancy_per_premium: row
            .sealed_harsher_stress_metrics
            .as_ref()
            .map(|metrics| metrics.expectancy_per_premium),
        extreme_stress_expectancy_per_premium: row
            .extreme_stress_metrics
            .as_ref()
            .map(|metrics| metrics.expectancy_per_premium),
        sealed_extreme_stress_expectancy_per_premium: row
            .sealed_extreme_stress_metrics
            .as_ref()
            .map(|metrics| metrics.expectancy_per_premium),
        selection_valid_trades: row.selection_metrics.valid_trades,
        holdout_valid_trades: row
            .holdout_metrics
            .as_ref()
            .map(|metrics| metrics.valid_trades),
        sealed_valid_trades: row
            .sealed_evaluation_metrics
            .as_ref()
            .map(|metrics| metrics.valid_trades),
        stress_valid_trades: row
            .stress_metrics
            .as_ref()
            .map(|metrics| metrics.valid_trades),
        sealed_stress_valid_trades: row
            .sealed_stress_metrics
            .as_ref()
            .map(|metrics| metrics.valid_trades),
        harsher_stress_valid_trades: row
            .harsher_stress_metrics
            .as_ref()
            .map(|metrics| metrics.valid_trades),
        sealed_harsher_stress_valid_trades: row
            .sealed_harsher_stress_metrics
            .as_ref()
            .map(|metrics| metrics.valid_trades),
        extreme_stress_valid_trades: row
            .extreme_stress_metrics
            .as_ref()
            .map(|metrics| metrics.valid_trades),
        sealed_extreme_stress_valid_trades: row
            .sealed_extreme_stress_metrics
            .as_ref()
            .map(|metrics| metrics.valid_trades),
    }
}

fn artifact_identity(
    path: &Path,
    artifact: &OptionsExpressionArtifactV1,
) -> ArtifactCompareIdentity {
    let best_robust_score = artifact
        .selection_metrics
        .iter()
        .map(|entry| entry.robust_score)
        .fold(0.0_f64, f64::max);
    let best_selection_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .map(|entry| entry.selection_metrics.expectancy_per_premium)
        .fold(0.0_f64, f64::max);
    ArtifactCompareIdentity {
        path: path.to_string_lossy().to_string(),
        ts: artifact.ts.clone(),
        parent_artifact_id: artifact.parent_artifact_id.clone(),
        root_symbol: artifact.root_symbol.clone(),
        options_dataset_id: artifact.options_dataset_id.clone(),
        backend_used: artifact.backend_used.clone(),
        backend_fallback_reason: artifact.backend_fallback_reason.clone(),
        price_semantics: format!("{:?}", artifact.price_semantics),
        tuple_count: artifact.selection_metrics.len(),
        total_events: artifact.diagnostics.total_events,
        search_events: artifact.diagnostics.search_events,
        holdout_events: artifact.diagnostics.holdout_events,
        sealed_test_events: artifact.diagnostics.sealed_test_events,
        best_robust_score,
        best_selection_expectancy_per_premium,
    }
}

fn matched_tuple_rows(
    left: &OptionsExpressionArtifactV1,
    right: &OptionsExpressionArtifactV1,
    limit: usize,
) -> Vec<MatchedTupleComparison> {
    let mut right_by_key: BTreeMap<String, (usize, &OptionsExpressionTupleResult)> =
        BTreeMap::new();
    for (index, row) in right.selection_metrics.iter().enumerate() {
        right_by_key.entry(tuple_key(row)).or_insert((index, row));
    }

    let mut matched: Vec<MatchedTupleComparison> = left
        .selection_metrics
        .iter()
        .enumerate()
        .filter_map(|(left_index, left_row)| {
            let key = tuple_key(left_row);
            let (right_index, right_row) = right_by_key.get(&key)?;
            let left_snapshot = tuple_snapshot(left_row, left_index);
            let right_snapshot = tuple_snapshot(right_row, *right_index);
            Some(MatchedTupleComparison {
                tuple_key: key,
                robust_score_delta: right_snapshot.robust_score - left_snapshot.robust_score,
                selection_expectancy_delta: right_snapshot.selection_expectancy_per_premium
                    - left_snapshot.selection_expectancy_per_premium,
                holdout_expectancy_delta: right_snapshot
                    .holdout_expectancy_per_premium
                    .zip(left_snapshot.holdout_expectancy_per_premium)
                    .map(|(rhs, lhs)| rhs - lhs),
                sealed_expectancy_delta: right_snapshot
                    .sealed_expectancy_per_premium
                    .zip(left_snapshot.sealed_expectancy_per_premium)
                    .map(|(rhs, lhs)| rhs - lhs),
                stress_expectancy_delta: right_snapshot
                    .stress_expectancy_per_premium
                    .zip(left_snapshot.stress_expectancy_per_premium)
                    .map(|(rhs, lhs)| rhs - lhs),
                sealed_stress_expectancy_delta: right_snapshot
                    .sealed_stress_expectancy_per_premium
                    .zip(left_snapshot.sealed_stress_expectancy_per_premium)
                    .map(|(rhs, lhs)| rhs - lhs),
                harsher_stress_expectancy_delta: right_snapshot
                    .harsher_stress_expectancy_per_premium
                    .zip(left_snapshot.harsher_stress_expectancy_per_premium)
                    .map(|(rhs, lhs)| rhs - lhs),
                sealed_harsher_stress_expectancy_delta: right_snapshot
                    .sealed_harsher_stress_expectancy_per_premium
                    .zip(left_snapshot.sealed_harsher_stress_expectancy_per_premium)
                    .map(|(rhs, lhs)| rhs - lhs),
                extreme_stress_expectancy_delta: right_snapshot
                    .extreme_stress_expectancy_per_premium
                    .zip(left_snapshot.extreme_stress_expectancy_per_premium)
                    .map(|(rhs, lhs)| rhs - lhs),
                sealed_extreme_stress_expectancy_delta: right_snapshot
                    .sealed_extreme_stress_expectancy_per_premium
                    .zip(left_snapshot.sealed_extreme_stress_expectancy_per_premium)
                    .map(|(rhs, lhs)| rhs - lhs),
                left: left_snapshot,
                right: right_snapshot,
            })
        })
        .collect();

    matched.sort_by(|lhs, rhs| {
        let lhs_rank = lhs
            .left
            .robust_score
            .abs()
            .max(lhs.right.robust_score.abs());
        let rhs_rank = rhs
            .left
            .robust_score
            .abs()
            .max(rhs.right.robust_score.abs());
        rhs_rank
            .partial_cmp(&lhs_rank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matched.truncate(limit.max(1));
    matched
}

fn filtered_summaries(
    summaries: &[OptionsExpressionArtifactSummary],
    parent_artifact_id: Option<&str>,
    root_symbol: Option<&str>,
    options_dataset_id: Option<&str>,
) -> Vec<OptionsExpressionArtifactSummary> {
    let parent_filter = normalize_filter(parent_artifact_id);
    let root_filter = normalize_filter(root_symbol);
    let dataset_filter = normalize_filter(options_dataset_id);
    summaries
        .iter()
        .filter(|entry| {
            parent_filter
                .as_ref()
                .is_none_or(|expected| entry.parent_artifact_id.to_ascii_uppercase() == *expected)
                && root_filter
                    .as_ref()
                    .is_none_or(|expected| entry.root_symbol.to_ascii_uppercase() == *expected)
                && dataset_filter.as_ref().is_none_or(|expected| {
                    entry.options_dataset_id.to_ascii_uppercase() == *expected
                })
        })
        .cloned()
        .collect()
}

fn resolve_left_right_paths(
    cli: &Cli,
    summaries: &[OptionsExpressionArtifactSummary],
) -> Result<(PathBuf, PathBuf), String> {
    let left_path = if let Some(explicit) = cli.left.clone() {
        explicit
    } else {
        summaries
            .first()
            .map(|entry| PathBuf::from(&entry.path))
            .ok_or_else(|| {
                "no saved options-expression artifacts match the requested filters".to_string()
            })?
    };
    let right_path = if let Some(explicit) = cli.right.clone() {
        explicit
    } else {
        summaries
            .iter()
            .map(|entry| PathBuf::from(&entry.path))
            .find(|path| path != &left_path)
            .ok_or_else(|| {
                "need at least two saved options-expression artifacts to compare".to_string()
            })?
    };
    Ok((left_path, right_path))
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    let app_handle = app.handle().clone();
    let app_local_data_dir = if let Some(explicit) = cli.app_local_data_dir.clone() {
        explicit
    } else {
        app_handle
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
    };

    let summaries = list_options_expression_artifacts_blocking_headless(&app_local_data_dir)?;
    let filtered = filtered_summaries(
        &summaries,
        cli.parent_artifact_id.as_deref(),
        cli.root_symbol.as_deref(),
        cli.options_dataset_id.as_deref(),
    );

    if cli.list_artifacts {
        let json = serde_json::to_string_pretty(&filtered).map_err(|e| e.to_string())?;
        println!("{json}");
        return Ok(());
    }

    let (left_path, right_path) = resolve_left_right_paths(&cli, &filtered)?;
    let left_artifact =
        load_options_expression_artifact_blocking_headless(&left_path, &app_local_data_dir)?;
    let right_artifact =
        load_options_expression_artifact_blocking_headless(&right_path, &app_local_data_dir)?;

    let left_map: BTreeMap<String, usize> = left_artifact
        .selection_metrics
        .iter()
        .enumerate()
        .map(|(index, row)| (tuple_key(row), index))
        .collect();
    let right_map: BTreeMap<String, usize> = right_artifact
        .selection_metrics
        .iter()
        .enumerate()
        .map(|(index, row)| (tuple_key(row), index))
        .collect();
    let matched_tuple_count = left_map
        .keys()
        .filter(|key| right_map.contains_key(*key))
        .count();
    let left_only_tuple_count = left_map.len().saturating_sub(matched_tuple_count);
    let right_only_tuple_count = right_map.len().saturating_sub(matched_tuple_count);

    let output = OptionsExpressionCompareOutput {
        app_local_data_dir: app_local_data_dir.to_string_lossy().to_string(),
        same_parent_artifact_id: left_artifact.parent_artifact_id
            == right_artifact.parent_artifact_id,
        same_root_symbol: left_artifact
            .root_symbol
            .eq_ignore_ascii_case(&right_artifact.root_symbol),
        same_options_dataset_id: left_artifact
            .options_dataset_id
            .eq_ignore_ascii_case(&right_artifact.options_dataset_id),
        left: artifact_identity(&left_path, &left_artifact),
        right: artifact_identity(&right_path, &right_artifact),
        matched_tuple_count,
        left_only_tuple_count,
        right_only_tuple_count,
        top_left_tuple: left_artifact
            .selection_metrics
            .first()
            .map(|row| tuple_snapshot(row, 0)),
        top_right_tuple: right_artifact
            .selection_metrics
            .first()
            .map(|row| tuple_snapshot(row, 0)),
        matched_tuple_deltas: matched_tuple_rows(&left_artifact, &right_artifact, cli.match_limit),
    };

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    if let Some(out_path) = cli.out_json.as_ref() {
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(out_path, &json)
            .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
    }
    println!("{json}");
    Ok(())
}
