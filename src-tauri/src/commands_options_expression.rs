use crate::data_cache;
use crate::settings;
use crate::state::AppState;
use serde::Serialize;
use serde_json::Value;
use ta_app_core::{
    analyze_options_expression_feasibility_blocking, build_options_signal_manifest_from_double_ma,
    compute_double_ma_drilldown_blocking_with_candles, dataset_manifest_path,
    inspect_options_expression_tuple_blocking, load_imported_options_dataset_from_csv,
    parse_timeframe_label, read_options_dataset_manifest, resample_candles,
    run_options_expression_blocking, DoubleMaDrilldownRequest, DoubleMaParamsResolved,
    DoubleMaRequest, DoubleMaValidationParams, ImportedOptionsDatasetCsvBuildRequest,
    OptionsExpressionArtifactV1, OptionsExpressionFeasibilityReport, OptionsExpressionRunRequest,
    OptionsExpressionRunResult, OptionsExpressionTupleInspectionResult, OptionsPriceSemantics,
    OPTIONS_EXPRESSION_ARTIFACT_VERSION,
};
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
pub struct RunOptionsExpressionResponse {
    pub artifact_path: String,
    pub artifact: OptionsExpressionArtifactV1,
    pub result: OptionsExpressionRunResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeOptionsExpressionFeasibilityResponse {
    pub replay_build_ms: u64,
    pub total_preview_ms: u64,
    pub report: OptionsExpressionFeasibilityReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptionsExpressionArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub parent_artifact_id: String,
    pub root_symbol: String,
    pub options_dataset_id: String,
    pub backend_used: String,
    pub price_semantics: OptionsPriceSemantics,
    pub tuple_count: usize,
    pub best_robust_score: f64,
    pub best_selection_expectancy_per_premium: f64,
    pub has_holdout_metrics: bool,
    pub has_sealed_evaluation_metrics: bool,
    pub has_stress_metrics: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InspectOptionsExpressionTupleRequest {
    pub artifact_path: String,
    pub tuple_index: usize,
}

fn non_empty_string(v: Option<&Value>) -> Option<String> {
    v.and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_run_artifact_kind(kind: &str) -> bool {
    matches!(
        kind.trim(),
        "vectorbt_run_artifact" | "vectorgrid_run_artifact"
    )
}

fn resolve_run_artifact_kind(value: &Value) -> String {
    non_empty_string(value.get("kind")).unwrap_or_default()
}

fn resolve_strategy_kind(value: &Value) -> String {
    non_empty_string(value.pointer("/run/req/strategy_kind"))
        .or_else(|| {
            non_empty_string(value.pointer("/validation/req/validation_context/strategy_kind"))
        })
        .or_else(|| non_empty_string(value.pointer("/validation/req/strategy_kind")))
        .unwrap_or_default()
}

fn resolve_csv_path(value: &Value) -> String {
    non_empty_string(value.get("csv_path"))
        .or_else(|| non_empty_string(value.pointer("/data/csv_path")))
        .or_else(|| non_empty_string(value.pointer("/dataset_meta/csv_path")))
        .or_else(|| non_empty_string(value.pointer("/validation/req/validation_context/csv_path")))
        .unwrap_or_default()
}

fn resolve_timeframe(value: &Value) -> String {
    non_empty_string(value.pointer("/dataset_meta/timeframe"))
        .or_else(|| non_empty_string(value.pointer("/validation/req/validation_context/timeframe")))
        .unwrap_or_default()
}

fn resolve_adjustment_mode(value: &Value) -> String {
    non_empty_string(value.pointer("/dataset_meta/adjustment_mode"))
        .unwrap_or_else(|| "raw".to_string())
}

fn normalize_symbol_root(raw: &str) -> String {
    raw.trim()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .to_ascii_uppercase()
}

fn derive_symbol_root(csv_path: &str) -> String {
    let stem = std::path::Path::new(csv_path)
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

fn resolve_symbol_root(value: &Value, csv_path: &str) -> String {
    [
        "/dataset_meta/display_symbol",
        "/dataset_meta/symbol",
        "/data/display_symbol",
        "/data/symbol",
        "/run/req/symbol",
    ]
    .iter()
    .find_map(|pointer| non_empty_string(value.pointer(pointer)))
    .map(|value| normalize_symbol_root(&value))
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| derive_symbol_root(csv_path))
}

fn build_source_dataset_identity_hash(
    csv_path: &std::path::Path,
    source_identity: data_cache::SourceIdentity,
) -> String {
    format!(
        "{}::{}::{}",
        csv_path.to_string_lossy().to_ascii_lowercase(),
        source_identity.size,
        source_identity.mtime_utc_ms
    )
}

fn build_dataset_hash(source_content_fingerprint: &str, timeframe: &str) -> String {
    format!("{source_content_fingerprint}::{timeframe}")
}

fn options_datasets_dir(
    app_local_data_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let market_root = settings::resolve_market_data_root_dir(app_local_data_dir)?;
    let dir = market_root.join("options_research").join("datasets");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn runs_dir(app_local_data_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let dir = app_local_data_dir.join("runs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn ensure_path_is_within_dir(path: &std::path::Path, dir: &std::path::Path) -> Result<(), String> {
    let canonical_dir = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let canonical_path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !canonical_path.starts_with(&canonical_dir) {
        return Err(format!(
            "path {} is outside the allowed directory {}",
            path.display(),
            dir.display()
        ));
    }
    Ok(())
}

fn sanitize_dataset_id_component(raw: &str) -> String {
    let out: String = raw
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    out.trim_matches('_').to_string()
}

fn sanitize_dataset_id(raw: &str) -> Result<String, String> {
    let sanitized = sanitize_dataset_id_component(raw);
    if sanitized.is_empty() {
        return Err("options_dataset_id resolved to an empty value".to_string());
    }
    Ok(sanitized)
}

fn build_import_request_from_manifest(
    manifest: &ta_app_core::OptionsDatasetManifestV1,
    dataset_path: &std::path::Path,
) -> ImportedOptionsDatasetCsvBuildRequest {
    ImportedOptionsDatasetCsvBuildRequest {
        options_dataset_id: manifest.options_dataset_id.clone(),
        dataset_path: dataset_path.to_string_lossy().to_string(),
        root_symbol: Some(manifest.root_symbol.clone()),
        provider_name: manifest.provider_name.clone(),
        provider_source_surface: manifest.provider_source_surface.clone(),
        provider_entitlement_profile: manifest.provider_entitlement_profile.clone(),
        quote_resolution: manifest.quote_resolution.clone(),
        price_semantics: manifest.price_semantics,
        calendar_id: manifest.calendar_id.clone(),
        timezone: manifest.timezone.clone(),
        normalization_version: manifest.normalization_version,
        quote_semantics_version: manifest.quote_semantics_version,
        corporate_action_mapping_version: manifest.corporate_action_mapping_version,
        contract_reference_source: manifest.contract_reference_source.clone(),
        contract_reference_as_of_utc: manifest.contract_reference_as_of_utc.clone(),
        underlier_reference_source: manifest.underlier_reference_source.clone(),
        aggregate_price_field: manifest.aggregate_price_field.clone(),
    }
}

fn dataset_rows_path_from_manifest_path(
    manifest_path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let dataset_dir = manifest_path.parent().ok_or_else(|| {
        format!(
            "options dataset manifest {} is missing a parent directory",
            manifest_path.display()
        )
    })?;
    let rows_path = dataset_dir.join("rows.csv");
    if !rows_path.is_file() {
        return Err(format!(
            "options dataset rows file does not exist: {}",
            rows_path.display()
        ));
    }
    ensure_path_is_within_dir(&rows_path, dataset_dir)?;
    Ok(rows_path)
}

fn suggest_options_expression_artifact_path(
    app_local_data_dir: &std::path::Path,
    ts: &str,
) -> Result<std::path::PathBuf, String> {
    let runs_dir = runs_dir(app_local_data_dir)?;
    std::fs::create_dir_all(&runs_dir).map_err(|e| e.to_string())?;
    let safe_ts = super::sanitize_filename_component(ts);
    let base_name = format!("vectorgrid_options_expression_{safe_ts}");
    let mut path = runs_dir.join(format!("{base_name}.json"));
    if !path.exists() {
        return Ok(path);
    }
    for suffix in 2..=999u32 {
        let candidate = runs_dir.join(format!("{base_name}_{suffix}.json"));
        if !candidate.exists() {
            path = candidate;
            break;
        }
    }
    Ok(path)
}

fn read_artifact_value(path: &std::path::Path) -> Result<Value, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("failed to read saved artifact {}: {e}", path.display()))?;
    serde_json::from_slice::<Value>(&bytes)
        .map_err(|e| format!("failed to parse saved artifact {}: {e}", path.display()))
}

fn load_replayed_candles(
    app_local_data_dir: &std::path::Path,
    artifact: &Value,
    csv_path: &str,
    timeframe: &str,
) -> Result<
    (
        vector_ta::utilities::data_loader::Candles,
        data_cache::SourceIdentity,
        String,
    ),
    String,
> {
    let csv_pathbuf = std::path::Path::new(csv_path);
    let source_identity = data_cache::identity_for_source(csv_pathbuf)?;
    let source_content_fingerprint = data_cache::content_fingerprint_for_source(csv_pathbuf)?;
    let cache_dir = data_cache::app_data_cache_dir(app_local_data_dir);
    let mut candles =
        data_cache::load_or_build_with_identity(csv_pathbuf, &cache_dir, source_identity)?.candles;

    let resample_applied = artifact
        .pointer("/dataset_meta/resample_applied")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let base_timeframe = non_empty_string(artifact.pointer("/dataset_meta/base_timeframe"));
    let derived_timeframe = non_empty_string(artifact.pointer("/dataset_meta/derived_timeframe"))
        .or_else(|| non_empty_string(artifact.pointer("/dataset_meta/timeframe")));

    if !timeframe.trim().is_empty()
        && (resample_applied || base_timeframe.as_deref() != Some(timeframe))
    {
        let spec = parse_timeframe_label(timeframe)?;
        candles = resample_candles(&candles, spec.interval_ms)?;
    } else if let (Some(base), Some(derived)) =
        (base_timeframe.as_deref(), derived_timeframe.as_deref())
    {
        if base != derived {
            let spec = parse_timeframe_label(derived)?;
            candles = resample_candles(&candles, spec.interval_ms)?;
        }
    }

    Ok((candles, source_identity, source_content_fingerprint))
}

fn choose_drilldown_request(
    artifact: &Value,
    parent_candidate_index: Option<usize>,
) -> Result<(DoubleMaDrilldownRequest, bool), String> {
    let run_req_value = artifact
        .pointer("/run/req")
        .cloned()
        .ok_or_else(|| "saved run artifact is missing run.req".to_string())?;
    let run_req: DoubleMaRequest = serde_json::from_value(run_req_value)
        .map_err(|e| format!("failed to deserialize DoubleMa run request from artifact: {e}"))?;

    let candidate_params = if let Some(index) = parent_candidate_index {
        Some(
            artifact
            .pointer(&format!("/validation/res/candidates/{index}/params"))
            .cloned()
            .ok_or_else(|| {
                format!("requested parent_candidate_index {index} is not present in validation results")
            })?,
        )
    } else {
        artifact
            .pointer("/validation/res/candidates/0/params")
            .cloned()
    };

    let (params, strategy, halftrend_filter) = if let Some(candidate_value) = candidate_params {
        let candidate: DoubleMaValidationParams =
            serde_json::from_value(candidate_value).map_err(|e| {
                format!("failed to deserialize validation candidate from artifact: {e}")
            })?;
        (
            DoubleMaParamsResolved {
                fast_len: candidate.fast_len,
                slow_len: candidate.slow_len,
                fast_ma_type: candidate.fast_ma_type.clone(),
                slow_ma_type: candidate.slow_ma_type.clone(),
                fast_ma_params: candidate.fast_ma_params.clone(),
                slow_ma_params: candidate.slow_ma_params.clone(),
            },
            candidate.strategy,
            candidate.halftrend_filter,
        )
    } else {
        let params_value = artifact
            .pointer("/run/res/best_params")
            .cloned()
            .ok_or_else(|| {
                "saved run artifact is missing run.res.best_params and validation candidates"
                    .to_string()
            })?;
        let params: DoubleMaParamsResolved = serde_json::from_value(params_value)
            .map_err(|e| format!("failed to deserialize best_params from artifact: {e}"))?;
        (
            params,
            run_req.strategy.clone(),
            run_req.halftrend_filter.clone(),
        )
    };

    Ok((
        DoubleMaDrilldownRequest {
            data_id: "artifact_replay".to_string(),
            backend_hint: None,
            followup_cache_key: None,
            params: params.clone(),
            ma_source: run_req.ma_source.clone(),
            fast_ma_params: params.fast_ma_params.clone(),
            slow_ma_params: params.slow_ma_params.clone(),
            strategy: strategy.clone(),
            halftrend_filter,
            bins: Some(256),
        },
        strategy.trade_on_next_bar,
    ))
}

fn build_options_expression_artifact(
    artifact_path: &std::path::Path,
    parent_artifact_id: String,
    options_dataset_hash: String,
    request: OptionsExpressionRunRequest,
    result: &OptionsExpressionRunResult,
) -> OptionsExpressionArtifactV1 {
    OptionsExpressionArtifactV1 {
        version: OPTIONS_EXPRESSION_ARTIFACT_VERSION,
        kind: "vectorgrid_options_expression_v1".to_string(),
        ts: chrono::Utc::now().to_rfc3339(),
        artifact_path: artifact_path.to_string_lossy().to_string(),
        parent_artifact_id,
        root_symbol: result.diagnostics.root_symbol.clone(),
        options_dataset_id: result.diagnostics.options_dataset_id.clone(),
        options_dataset_hash,
        backend_used: result.backend_used.clone(),
        price_semantics: result.price_semantics,
        request,
        diagnostics: result.diagnostics.clone(),
        staging: result.staging.clone(),
        selection_metrics: result.ranked_results.clone(),
    }
}

struct OptionsExpressionReplayInputs {
    parent_artifact_id: String,
    normalized_req: OptionsExpressionRunRequest,
    signal_manifest: ta_app_core::OptionsSignalEventManifestV1,
    dataset: ta_app_core::ImportedOptionsDataset,
    options_dataset_hash: String,
}

fn build_options_expression_replay_inputs(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsExpressionReplayInputs, String> {
    let runs_dir = runs_dir(app_local_data_dir)?;
    let parent_artifact_path = std::path::Path::new(req.parent_artifact_path.trim());
    if !parent_artifact_path.is_file() {
        return Err(format!(
            "parent artifact does not exist: {}",
            parent_artifact_path.display()
        ));
    }
    ensure_path_is_within_dir(parent_artifact_path, &runs_dir)?;
    let artifact = read_artifact_value(parent_artifact_path)?;
    let kind = resolve_run_artifact_kind(&artifact);
    if !is_run_artifact_kind(&kind) {
        return Err(format!(
            "options expression parent must be a saved run artifact, found kind '{}'",
            kind
        ));
    }
    let strategy_kind = resolve_strategy_kind(&artifact);
    if strategy_kind.trim() != "DoubleMa" {
        return Err(format!(
            "options expression v1 currently supports DoubleMa parent artifacts only, found '{}'",
            strategy_kind
        ));
    }
    let csv_path = resolve_csv_path(&artifact);
    if csv_path.trim().is_empty() {
        return Err("parent artifact is missing dataset CSV path".to_string());
    }
    let timeframe = resolve_timeframe(&artifact);
    let adjustment_mode = resolve_adjustment_mode(&artifact);
    let (candles, source_identity, source_content_fingerprint) =
        load_replayed_candles(app_local_data_dir, &artifact, &csv_path, &timeframe)?;
    let parent_artifact_id = parent_artifact_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("parent_artifact")
        .to_string();
    let source_dataset_identity_hash =
        build_source_dataset_identity_hash(std::path::Path::new(&csv_path), source_identity);
    let dataset_hash = build_dataset_hash(&source_content_fingerprint, &timeframe);
    let symbol_root = resolve_symbol_root(&artifact, &csv_path);
    let (drilldown_req, trade_on_next_bar) =
        choose_drilldown_request(&artifact, req.parent_candidate_index)?;
    let curves = compute_double_ma_drilldown_blocking_with_candles(drilldown_req, &candles)?;
    let signal_manifest = build_options_signal_manifest_from_double_ma(
        &parent_artifact_id,
        &kind,
        &source_dataset_identity_hash,
        &source_content_fingerprint,
        &dataset_hash,
        &timeframe,
        None,
        &adjustment_mode,
        &symbol_root,
        trade_on_next_bar,
        &candles,
        &curves,
    )?;

    let datasets_dir = options_datasets_dir(app_local_data_dir)?;
    let dataset_id = sanitize_dataset_id(req.options_dataset_id.trim())?;
    let manifest_path = dataset_manifest_path(&datasets_dir, &dataset_id);
    if !manifest_path.is_file() {
        return Err(format!("options dataset '{}' is not imported", dataset_id));
    }
    let dataset_manifest = read_options_dataset_manifest(&manifest_path)?;
    if matches!(
        dataset_manifest.price_semantics,
        OptionsPriceSemantics::MinuteAggregate
    ) && dataset_manifest
        .aggregate_price_field
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| !value.eq_ignore_ascii_case("close"))
        .unwrap_or(false)
    {
        return Err(
            "minute-aggregate options datasets currently require aggregate_price_field='close'"
                .to_string(),
        );
    }
    let dataset_rows_path = dataset_rows_path_from_manifest_path(&manifest_path)?;
    let options_dataset_hash = data_cache::content_fingerprint_for_source(&dataset_rows_path)?;
    let dataset = load_imported_options_dataset_from_csv(&build_import_request_from_manifest(
        &dataset_manifest,
        &dataset_rows_path,
    ))?;
    let mut normalized_req = req;
    normalized_req.options_dataset_id = dataset_id;

    Ok(OptionsExpressionReplayInputs {
        parent_artifact_id,
        normalized_req,
        signal_manifest,
        dataset,
        options_dataset_hash,
    })
}

fn run_options_expression_blocking_impl(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<RunOptionsExpressionResponse, String> {
    let replay = build_options_expression_replay_inputs(req, app_local_data_dir)?;
    let result = run_options_expression_blocking(
        replay.normalized_req.clone(),
        &replay.signal_manifest,
        &replay.dataset,
    )?;
    let ts = chrono::Utc::now().to_rfc3339();
    let artifact_path = suggest_options_expression_artifact_path(app_local_data_dir, &ts)?;
    let artifact = build_options_expression_artifact(
        &artifact_path,
        replay.parent_artifact_id,
        replay.options_dataset_hash,
        replay.normalized_req,
        &result,
    );
    let bytes = serde_json::to_vec_pretty(&artifact)
        .map_err(|e| format!("failed to serialize options expression artifact: {e}"))?;
    std::fs::write(&artifact_path, bytes).map_err(|e| {
        format!(
            "failed to write options expression artifact {}: {e}",
            artifact_path.display()
        )
    })?;

    Ok(RunOptionsExpressionResponse {
        artifact_path: artifact_path.to_string_lossy().to_string(),
        artifact,
        result,
    })
}

fn analyze_options_expression_feasibility_blocking_impl(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<AnalyzeOptionsExpressionFeasibilityResponse, String> {
    let preview_start = std::time::Instant::now();
    let replay_start = std::time::Instant::now();
    let replay = build_options_expression_replay_inputs(req, app_local_data_dir)?;
    let replay_build_ms = replay_start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let report = analyze_options_expression_feasibility_blocking(
        replay.normalized_req,
        &replay.signal_manifest,
        &replay.dataset,
    )?;
    let total_preview_ms = preview_start
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;

    Ok(AnalyzeOptionsExpressionFeasibilityResponse {
        replay_build_ms,
        total_preview_ms,
        report,
    })
}

#[tauri::command]
pub async fn run_options_expression(
    req: OptionsExpressionRunRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunOptionsExpressionResponse, String> {
    let _run_guard = state.try_begin_run()?;
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        run_options_expression_blocking_impl(req, &app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn analyze_options_expression_feasibility(
    req: OptionsExpressionRunRequest,
    app: tauri::AppHandle,
) -> Result<AnalyzeOptionsExpressionFeasibilityResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        analyze_options_expression_feasibility_blocking_impl(req, &app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_options_expression_artifacts(
    app: tauri::AppHandle,
) -> Result<Vec<OptionsExpressionArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let runs_dir = runs_dir(&app_local_data_dir)?;
        if !runs_dir.exists() {
            return Ok(Vec::new());
        }
        let mut out: Vec<OptionsExpressionArtifactSummary> = Vec::new();
        for entry in std::fs::read_dir(&runs_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            if !file_name.starts_with("vectorgrid_options_expression_") {
                continue;
            }
            let bytes = entry.metadata().map_err(|e| e.to_string())?.len();
            let Ok(value) = read_artifact_value(&path) else {
                continue;
            };
            let Ok(artifact) = serde_json::from_value::<OptionsExpressionArtifactV1>(value) else {
                continue;
            };
            if artifact.kind.trim() != "vectorgrid_options_expression_v1" {
                continue;
            }
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
            let has_holdout_metrics = artifact
                .selection_metrics
                .iter()
                .any(|entry| entry.holdout_metrics.is_some());
            let has_sealed_evaluation_metrics = artifact
                .selection_metrics
                .iter()
                .any(|entry| entry.sealed_evaluation_metrics.is_some());
            let has_stress_metrics = artifact
                .selection_metrics
                .iter()
                .any(|entry| entry.stress_metrics.is_some());
            out.push(OptionsExpressionArtifactSummary {
                ts: artifact.ts.clone(),
                path: path.to_string_lossy().to_string(),
                bytes,
                parent_artifact_id: artifact.parent_artifact_id.clone(),
                root_symbol: artifact.root_symbol.clone(),
                options_dataset_id: artifact.options_dataset_id.clone(),
                backend_used: artifact.backend_used.clone(),
                price_semantics: artifact.price_semantics,
                tuple_count: artifact.selection_metrics.len(),
                best_robust_score,
                best_selection_expectancy_per_premium,
                has_holdout_metrics,
                has_sealed_evaluation_metrics,
                has_stress_metrics,
            });
        }
        out.sort_by(|a, b| b.ts.cmp(&a.ts));
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_options_expression_artifact(
    path: String,
    app: tauri::AppHandle,
) -> Result<OptionsExpressionArtifactV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let artifact_path = std::path::Path::new(path.trim());
        if !artifact_path.is_file() {
            return Err(format!(
                "options expression artifact does not exist: {}",
                artifact_path.display()
            ));
        }
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let runs_dir = runs_dir(&app_local_data_dir)?;
        ensure_path_is_within_dir(artifact_path, &runs_dir)?;
        let value = read_artifact_value(artifact_path)?;
        let artifact =
            serde_json::from_value::<OptionsExpressionArtifactV1>(value).map_err(|e| {
                format!(
                    "failed to deserialize options expression artifact {}: {e}",
                    artifact_path.display()
                )
            })?;
        if artifact.kind.trim() != "vectorgrid_options_expression_v1" {
            return Err(format!(
                "unsupported options expression artifact kind: {}",
                artifact.kind
            ));
        }
        Ok(artifact)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn inspect_options_expression_tuple(
    req: InspectOptionsExpressionTupleRequest,
    app: tauri::AppHandle,
) -> Result<OptionsExpressionTupleInspectionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let artifact_path = std::path::Path::new(req.artifact_path.trim());
        if !artifact_path.is_file() {
            return Err(format!(
                "options expression artifact does not exist: {}",
                artifact_path.display()
            ));
        }
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let runs_dir = runs_dir(&app_local_data_dir)?;
        ensure_path_is_within_dir(artifact_path, &runs_dir)?;
        let value = read_artifact_value(artifact_path)?;
        let artifact =
            serde_json::from_value::<OptionsExpressionArtifactV1>(value).map_err(|e| {
                format!(
                    "failed to deserialize options expression artifact {}: {e}",
                    artifact_path.display()
                )
            })?;
        if artifact.kind.trim() != "vectorgrid_options_expression_v1" {
            return Err(format!(
                "unsupported options expression artifact kind: {}",
                artifact.kind
            ));
        }
        let expected_tuple = artifact
            .selection_metrics
            .get(req.tuple_index)
            .ok_or_else(|| {
                format!(
                    "tuple_index {} is out of range for saved artifact with {} tuple{}",
                    req.tuple_index,
                    artifact.selection_metrics.len(),
                    if artifact.selection_metrics.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                )
            })?
            .tuple
            .clone();
        let replay =
            build_options_expression_replay_inputs(artifact.request.clone(), &app_local_data_dir)?;
        let inspection = inspect_options_expression_tuple_blocking(
            replay.normalized_req,
            req.tuple_index,
            &replay.signal_manifest,
            &replay.dataset,
        )?;
        if inspection.tuple_result.tuple != expected_tuple {
            return Err(format!(
                "recomputed tuple at index {} does not match the saved artifact ordering",
                req.tuple_index
            ));
        }
        Ok(inspection)
    })
    .await
    .map_err(|e| e.to_string())?
}
