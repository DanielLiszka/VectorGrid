use super::*;
use serde_json::json;
use std::path::Path;
use tauri::Manager;

#[derive(Debug)]
pub(crate) struct SealedCpuVcetCampaignSourceFixture {
    pub(crate) summary_json_path: String,
    pub(crate) selection_csv_path: String,
    pub(crate) final_test_csv_path: String,
    pub(crate) candidate_ids: Vec<String>,
}

fn cpu_validation_result(plan: &CampaignVcetFixedCandidatePlan, dataset_index: usize) -> Value {
    let candidates = plan
        .candidates
        .iter()
        .enumerate()
        .map(|(candidate_index, params)| {
            let pnl = 2.0 + dataset_index as f64 + candidate_index as f64;
            json!({
                "params": params,
                "in_sample": {
                    "pnl": pnl,
                    "sharpe": pnl / 10.0,
                    "max_dd": 0.10,
                    "trades": 20 + candidate_index as u32,
                    "exposure": 0.50,
                    "net_exposure": 0.25,
                },
                "validation_verdict": { "passed": true },
                "robust_score": pnl / 10.0 - 0.20,
                "robust_pareto": candidate_index + 1 == plan.candidates.len(),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "requested_backend": "CpuOnly",
        "backend_used": "Cpu",
        "backend_route": "CpuOnly",
        "backend_fallback_reason": null,
        "validation_residency_tier": "cpu",
        "candidates": candidates,
    })
}

/// Builds the exact app-owned source bundle consumed by temporal campaign
/// preparation while avoiding a second, unrelated optimization run in these
/// lifecycle tests. All manifest, artifact, admission, and hash surfaces are
/// produced and preflighted through the production campaign builders.
pub(crate) fn build_sealed_cpu_vcet_campaign_source<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    dataset_paths: [&Path; 2],
) -> Result<SealedCpuVcetCampaignSourceFixture, String> {
    let mut request = ResearchCampaignRequest {
        strategy_kind: "Vcet".to_string(),
        datasets: dataset_paths
            .iter()
            .enumerate()
            .map(|(index, path)| ResearchCampaignDatasetInput {
                csv_path: path.to_string_lossy().to_string(),
                label: Some(format!("VCET_TEMPORAL_{}", index + 1)),
                timeframe: Some("1m".to_string()),
                ..ResearchCampaignDatasetInput::default()
            })
            .collect(),
        timeframe_override: Some("1m".to_string()),
        tags: vec!["vcet-temporal-lifecycle".to_string()],
        holdout_pct: 50,
        holdout_tag: Some("vcet-temporal-final".to_string()),
        skip_existing: false,
        auto_meta: false,
        auto_stage5: false,
        execution_mode: ResearchCampaignExecutionMode::EvaluateCandidates,
        optimization_template: json!({
            "strategy_kind": "Vcet",
            "data_id": "vcet-temporal-lifecycle-template",
            "backend": { "backend": "CpuOnly" },
            "fast_range": [8, 12, 4],
            "slow_range": [32, 48, 16],
            "fast_ma_types": ["vidya"],
            "slow_ma_types": ["vidya"],
            "ma_source": "close",
            "fast_ma_params": null,
            "slow_ma_params": null,
            "strategy": ta_strategies::double_ma::StrategyConfig::default(),
            "halftrend": {
                "amplitude": 2,
                "channel_deviation": 2.0,
                "atr_period": 100
            },
            "vcet": {
                "map_fast_to_vidya_short_period": true,
                "map_slow_to_vidya_long_period": true,
                "map_slow_to_ce_period": false,
                "vidya_short_period": 8,
                "vidya_long_period": 32,
                "vidya_alpha": 0.2,
                "ce_period": 20,
                "ce_mult": 3.0,
                "ce_use_close": true,
                "require_vidya_slope": true,
                "max_trade_bars": 4
            },
            "objective": "Sharpe",
            "mode": "Exact",
            "top_k": 2,
            "include_all": false,
            "heatmap_bins": 8
        }),
        validation_config: ResearchCampaignValidationConfig {
            max_candidates: 2,
            fixed_candidates: vec![
                ResearchCampaignVcetFixedCandidate {
                    fast_len: 8,
                    slow_len: 32,
                },
                ResearchCampaignVcetFixedCandidate {
                    fast_len: 12,
                    slow_len: 48,
                },
            ],
            require_gpu_kernel: false,
            cross_universe_admission: ResearchCampaignCrossUniverseAdmissionPolicy {
                min_profitable_share_requested: 0.0,
                min_hard_gate_pass_share_requested: 0.0,
                min_pnl_p10: -1_000_000.0,
                min_sharpe_p10: -1_000_000.0,
            },
            ..ResearchCampaignValidationConfig::default()
        },
        meta_request: None,
        stage5_request: None,
        preload_policy: ResearchCampaignPreloadPolicy::HostOnly,
    };

    request.datasets = normalize_campaign_datasets(request.datasets)?;
    refresh_campaign_dataset_source_identities(&mut request.datasets)?;
    validate_holdout_tag_isolation(
        &request.tags,
        &request.datasets,
        request.holdout_pct,
        request.holdout_tag.as_deref(),
    )?;

    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    let cache_dir = data_cache::app_data_cache_dir(&base);
    let mut prepared_datasets = Vec::with_capacity(request.datasets.len());
    let mut seen_source_fingerprints = HashMap::new();
    for input in request.datasets.iter().cloned() {
        let prepared =
            prepare_campaign_dataset(input, request.timeframe_override.as_deref(), &cache_dir)?;
        record_unique_prepared_campaign_source(&prepared, &mut seen_source_fingerprints)?;
        prepared_datasets.push(prepared);
    }
    request.datasets = prepared_datasets
        .iter()
        .map(|prepared| prepared.input.clone())
        .collect();

    let plan = build_vcet_fixed_candidate_plan(
        &request,
        "Vcet",
        ResearchCampaignExecutionMode::EvaluateCandidates,
    )?
    .ok_or_else(|| "test fixture did not produce a VCET candidate plan".to_string())?;
    let manifest = build_campaign_manifest(
        &request,
        "Vcet",
        Some("1m"),
        &request.tags,
        request.holdout_tag.as_deref(),
        ResearchCampaignExecutionMode::EvaluateCandidates,
    );
    let manifest_hash = manifest.manifest_hash.clone();
    let dataset_paths = request
        .datasets
        .iter()
        .map(|dataset| dataset.csv_path.clone())
        .collect::<Vec<_>>();
    let (_, holdout_paths) =
        split_holdout_paths(&dataset_paths, request.holdout_pct, &request.tags[0]);
    let cohort_tag = campaign_manifest_cohort_tag(&manifest_hash);
    let mut items = Vec::with_capacity(prepared_datasets.len());

    for (index, prepared) in prepared_datasets.iter().enumerate() {
        let is_holdout = holdout_paths.contains(&prepared.input.csv_path);
        let data_id = make_campaign_data_id(index, &prepared.input.csv_path, &prepared.timeframe);
        let run_request =
            patch_optimization_template(&request.optimization_template, "Vcet", &data_id)?;
        let validation_request = build_vcet_fixed_candidate_validation_request_value(
            &data_id,
            &run_request,
            &prepared.dataset_meta,
            &request.validation_config,
            &plan,
        );
        let validation_response = cpu_validation_result(&plan, index);
        let fixed_candidate_result = parse_vcet_fixed_candidate_dataset_result(
            &validation_response,
            &plan,
            request.validation_config.require_gpu_kernel,
        )?;
        let mut tags = campaign_artifact_tags(
            &request.tags,
            &prepared.label,
            is_holdout,
            request.holdout_tag.as_deref(),
        );
        tags.push(cohort_tag.clone());
        let tags = dedup_tags(tags);
        let base_fingerprint = run_artifacts::compute_config_fingerprint_from_artifact(
            &artifact_stub(&run_request, Some(&validation_request)),
        );
        let fingerprint = campaign_artifact_config_fingerprint(
            &base_fingerprint,
            &tags,
            is_holdout,
            request.holdout_tag.as_deref(),
        );
        let artifact = build_run_artifact_value(
            &format!("2026-08-02T00:00:{index:02}Z"),
            &fingerprint,
            &manifest_hash,
            is_holdout,
            request.holdout_tag.as_deref(),
            &prepared.input.csv_path,
            &prepared.dataset_meta,
            &tags,
            &run_request,
            &vcet_fixed_candidate_run_result_value(&plan),
            Some(&validation_request),
            Some(&validation_response),
        );
        let saved = save_campaign_run_artifact_blocking(
            artifact,
            app.clone(),
            None,
            "Vcet",
            ResearchCampaignExecutionMode::EvaluateCandidates,
        )?;
        items.push(ResearchCampaignItemSummary {
            item_id: Some(campaign_item_id(
                &manifest_hash,
                index,
                &prepared.input.csv_path,
                &prepared.timeframe,
            )),
            csv_path: prepared.input.csv_path.clone(),
            label: prepared.label.clone(),
            timeframe: prepared.timeframe.clone(),
            is_holdout,
            status: "ok".to_string(),
            dataset_identity: Some(dataset_identity_summary(prepared)),
            evidence_state: Some(CampaignEvidenceState::Complete),
            resume_state: Some(CampaignResumeState::Complete),
            reuse_decision: None,
            artifact_path: Some(saved.path),
            artifact_sha256: None,
            report_xlsx_path: None,
            error: None,
            diagnostics: None,
            fixed_candidate_result: Some(fixed_candidate_result),
        });
    }

    seal_vcet_campaign_artifact_hashes(&mut items, app)?;
    let counts = ResearchCampaignCounts {
        total: items.len(),
        optimized: 0,
        validated: items.len(),
        skipped: 0,
        failed: 0,
        cancelled: false,
    };
    let mut summary = build_campaign_summary(
        "2026-08-02T00:01:00Z",
        "completed",
        "Vcet",
        Some("1m".to_string()),
        &request,
        ResearchCampaignExecutionMode::EvaluateCandidates,
        &request.tags,
        request.holdout_tag.clone(),
        &counts,
        &items,
        None,
        None,
        &[],
        1,
    );
    let summary_value = serde_json::to_value(&summary)
        .map_err(|error| format!("encode test campaign summary: {error}"))?;
    validate_campaign_summary_artifact_value(&summary_value)?;
    let candidate_source = build_vcet_temporal_campaign_candidate_source(&summary)?;

    let contract = build_vcet_fixed_candidate_artifact_contract(&summary)?;
    let mut training_artifacts = Vec::new();
    for item in &items {
        let path = item
            .artifact_path
            .as_ref()
            .ok_or_else(|| "test source item is missing its artifact path".to_string())?;
        let (artifact, _) = run_artifacts::load_artifact_value_checked_with_sha256_blocking(
            path.clone(),
            app.clone(),
        )?;
        validate_vcet_fixed_candidate_artifact_contract(&artifact, &contract)?;
        validate_vcet_fixed_candidate_artifact_request_contract(&artifact, &contract)?;
        if !item.is_holdout {
            training_artifacts.push(artifact);
        }
    }
    validate_vcet_fixed_candidate_training_evidence(&summary, training_artifacts.iter())?;

    let selection_csv_path = items
        .iter()
        .find(|item| !item.is_holdout)
        .map(|item| item.csv_path.clone())
        .ok_or_else(|| "test source fixture has no selection dataset".to_string())?;
    let final_test_csv_path = items
        .iter()
        .find(|item| item.is_holdout)
        .map(|item| item.csv_path.clone())
        .ok_or_else(|| "test source fixture has no final-test dataset".to_string())?;
    let candidate_ids = candidate_source
        .sealed_candidate_manifest
        .payload
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id.clone())
        .collect();
    let (summary_json_path, _) =
        save_campaign_summary_checkpoint_blocking(&mut summary, &mut items, None, app, None)?;

    Ok(SealedCpuVcetCampaignSourceFixture {
        summary_json_path,
        selection_csv_path,
        final_test_csv_path,
        candidate_ids,
    })
}
