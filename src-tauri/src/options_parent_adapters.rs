use serde_json::Value;
use ta_app_core::{
    build_options_signal_manifest_from_breakout_momentum,
    build_options_signal_manifest_from_composite_trend,
    build_options_signal_manifest_from_double_ma, build_options_signal_manifest_from_regime_hybrid,
    DoubleMaCurves, DoubleMaDrilldownRequest, DoubleMaParamsResolved, DoubleMaRequest,
    DoubleMaValidationParams, OptionsSignalEventManifestV1,
};
use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OptionsSupportedParentFamily {
    DoubleMa,
    CompositeTrend,
    RegimeHybrid,
    BreakoutMomentum,
}

impl OptionsSupportedParentFamily {
    pub fn strategy_kind(self) -> &'static str {
        match self {
            Self::DoubleMa => "DoubleMa",
            Self::CompositeTrend => "CompositeTrend",
            Self::RegimeHybrid => "RegimeHybrid",
            Self::BreakoutMomentum => "BreakoutMomentum",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SupportedOptionsParentReplay {
    pub family: OptionsSupportedParentFamily,
    pub drilldown_request: DoubleMaDrilldownRequest,
    pub trade_on_next_bar: bool,
}

fn non_empty_string(v: Option<&Value>) -> Option<String> {
    v.and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn resolved_strategy_kind(value: &Value) -> Option<String> {
    non_empty_string(value.pointer("/run/req/strategy_kind"))
        .or_else(|| {
            non_empty_string(value.pointer("/validation/req/validation_context/strategy_kind"))
        })
        .or_else(|| non_empty_string(value.pointer("/validation/req/strategy_kind")))
}

fn resolve_strategy_kind(value: &Value) -> String {
    resolved_strategy_kind(value).unwrap_or_default()
}

pub fn supported_options_parent_strategy_list() -> &'static str {
    "DoubleMa, CompositeTrend, RegimeHybrid, and BreakoutMomentum"
}

/// Kind allowlist prefilter only; replay compatibility is established later.
pub fn supported_options_parent_family_from_artifact(
    artifact: &Value,
) -> Option<OptionsSupportedParentFamily> {
    let strategy_kind = resolve_strategy_kind(artifact);
    if !is_supported_options_parent_strategy_kind(&strategy_kind) {
        return None;
    }
    supported_options_parent_family_from_kind(&strategy_kind)
}

pub fn supported_options_parent_family_from_kind(
    raw: &str,
) -> Option<OptionsSupportedParentFamily> {
    match raw.trim() {
        "DoubleMa" => Some(OptionsSupportedParentFamily::DoubleMa),
        "CompositeTrend" => Some(OptionsSupportedParentFamily::CompositeTrend),
        "RegimeHybrid" => Some(OptionsSupportedParentFamily::RegimeHybrid),
        "BreakoutMomentum" => Some(OptionsSupportedParentFamily::BreakoutMomentum),
        _ => None,
    }
}

pub fn is_supported_options_parent_strategy_kind(raw: &str) -> bool {
    supported_options_parent_family_from_kind(raw).is_some()
}

fn build_double_like_parent_replay(
    family: OptionsSupportedParentFamily,
    artifact: &Value,
    parent_candidate_index: Option<usize>,
) -> Result<SupportedOptionsParentReplay, String> {
    let run_req_value = artifact
        .pointer("/run/req")
        .cloned()
        .ok_or_else(|| "saved run artifact is missing run.req".to_string())?;
    let run_req: DoubleMaRequest = serde_json::from_value(run_req_value).map_err(|e| {
        format!(
            "failed to deserialize supported options parent '{}' run request: {e}",
            family.strategy_kind()
        )
    })?;

    let candidate_params = if let Some(index) = parent_candidate_index {
        // An explicit replay candidate request is fail-closed when absent.
        Some(
            artifact
                .pointer(&format!("/validation/res/candidates/{index}/params"))
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "requested parent_candidate_index {index} is not present in validation results"
                    )
                })?,
        )
    } else {
        artifact
            .pointer("/validation/res/candidates/0/params")
            .cloned()
    };

    let (params, strategy, halftrend_filter) = if let Some(candidate_value) = candidate_params {
        // A malformed candidate payload is an error; do not silently fall back.
        let candidate: DoubleMaValidationParams =
            serde_json::from_value(candidate_value).map_err(|e| {
                format!(
                    "failed to deserialize supported '{}' validation candidate from artifact: {e}",
                    family.strategy_kind()
                )
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
        let params: DoubleMaParamsResolved = serde_json::from_value(params_value).map_err(|e| {
            format!(
                "failed to deserialize supported '{}' best_params from artifact: {e}",
                family.strategy_kind()
            )
        })?;
        (
            params,
            run_req.strategy.clone(),
            run_req.halftrend_filter.clone(),
        )
    };

    Ok(SupportedOptionsParentReplay {
        family,
        drilldown_request: DoubleMaDrilldownRequest {
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
        trade_on_next_bar: strategy.trade_on_next_bar,
    })
}

pub fn build_supported_options_parent_replay(
    artifact: &Value,
    parent_candidate_index: Option<usize>,
) -> Result<SupportedOptionsParentReplay, String> {
    let strategy_kind = resolved_strategy_kind(artifact).ok_or_else(|| {
        "saved run artifact is missing supported options parent strategy-kind metadata".to_string()
    })?;
    if !is_supported_options_parent_strategy_kind(&strategy_kind) {
        return Err(format!(
            "options expression currently supports {} parent artifacts only (found strategy kind '{strategy_kind}')",
            supported_options_parent_strategy_list()
        ));
    }
    let family = supported_options_parent_family_from_kind(&strategy_kind).ok_or_else(|| {
        format!(
            "options expression currently supports {} parent artifacts only (found strategy kind '{strategy_kind}')",
            supported_options_parent_strategy_list()
        )
    })?;

    match family {
        OptionsSupportedParentFamily::DoubleMa => {
            build_double_like_parent_replay(family, artifact, parent_candidate_index)
        }
        OptionsSupportedParentFamily::CompositeTrend => {
            build_double_like_parent_replay(family, artifact, parent_candidate_index)
        }
        OptionsSupportedParentFamily::RegimeHybrid => {
            build_double_like_parent_replay(family, artifact, parent_candidate_index)
        }
        OptionsSupportedParentFamily::BreakoutMomentum => {
            build_double_like_parent_replay(family, artifact, parent_candidate_index)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_supported_options_parent_signal_manifest(
    family: OptionsSupportedParentFamily,
    parent_artifact_id: &str,
    parent_run_kind: &str,
    source_dataset_identity_hash: &str,
    source_content_fingerprint: &str,
    dataset_hash: &str,
    timeframe: &str,
    resample_spec_hash: Option<String>,
    adjustment_mode: &str,
    symbol_root: &str,
    trade_on_next_bar: bool,
    candles: &Candles,
    curves: &DoubleMaCurves,
) -> Result<OptionsSignalEventManifestV1, String> {
    match family {
        OptionsSupportedParentFamily::DoubleMa => build_options_signal_manifest_from_double_ma(
            parent_artifact_id,
            parent_run_kind,
            source_dataset_identity_hash,
            source_content_fingerprint,
            dataset_hash,
            timeframe,
            resample_spec_hash,
            adjustment_mode,
            symbol_root,
            trade_on_next_bar,
            candles,
            curves,
        ),
        OptionsSupportedParentFamily::CompositeTrend => {
            build_options_signal_manifest_from_composite_trend(
                parent_artifact_id,
                parent_run_kind,
                source_dataset_identity_hash,
                source_content_fingerprint,
                dataset_hash,
                timeframe,
                resample_spec_hash,
                adjustment_mode,
                symbol_root,
                trade_on_next_bar,
                candles,
                curves,
            )
        }
        OptionsSupportedParentFamily::RegimeHybrid => {
            build_options_signal_manifest_from_regime_hybrid(
                parent_artifact_id,
                parent_run_kind,
                source_dataset_identity_hash,
                source_content_fingerprint,
                dataset_hash,
                timeframe,
                resample_spec_hash,
                adjustment_mode,
                symbol_root,
                trade_on_next_bar,
                candles,
                curves,
            )
        }
        OptionsSupportedParentFamily::BreakoutMomentum => {
            build_options_signal_manifest_from_breakout_momentum(
                parent_artifact_id,
                parent_run_kind,
                source_dataset_identity_hash,
                source_content_fingerprint,
                dataset_hash,
                timeframe,
                resample_spec_hash,
                adjustment_mode,
                symbol_root,
                trade_on_next_bar,
                candles,
                curves,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use ta_app_core::{double_ma::GpuExecutionPolicy, Backend};
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::StrategyConfig;

    fn sample_run_request() -> DoubleMaRequest {
        DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "artifact_parent".to_string(),
            fast_range: (5, 5, 1),
            slow_range: (20, 20, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            hurdle_tau: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(10),
            include_all: Some(false),
            heatmap_bins: None,
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
        }
    }

    fn sample_candidate(fast_len: u16, trade_on_next_bar: bool) -> DoubleMaValidationParams {
        DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len: fast_len + 10,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig {
                trade_on_next_bar,
                ..StrategyConfig::default()
            },
            halftrend_filter: None,
        }
    }

    fn sample_best_params(fast_len: u16) -> DoubleMaParamsResolved {
        DoubleMaParamsResolved {
            fast_len,
            slow_len: fast_len + 10,
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
        }
    }

    fn artifact_with_replay_payload(
        strategy_kind: Option<&str>,
        candidates: Option<Vec<Value>>,
        best_params: Option<DoubleMaParamsResolved>,
    ) -> Value {
        let mut run_req = serde_json::to_value(sample_run_request()).expect("run request");
        if let Some(strategy_kind) = strategy_kind {
            run_req
                .as_object_mut()
                .expect("request object")
                .insert("strategy_kind".to_string(), json!(strategy_kind));
        }
        let best_params_value =
            best_params.map(|params| serde_json::to_value(params).expect("best params"));
        let candidates_value = candidates.unwrap_or_default();

        json!({
            "run": {
                "req": run_req,
                "res": {
                    "best_params": best_params_value
                }
            },
            "validation": {
                "res": {
                    "candidates": candidates_value
                }
            }
        })
    }

    #[test]
    fn supported_parent_family_mapping_matches_current_surface() {
        assert_eq!(
            supported_options_parent_family_from_kind("DoubleMa"),
            Some(OptionsSupportedParentFamily::DoubleMa)
        );
        assert_eq!(
            supported_options_parent_family_from_kind("CompositeTrend"),
            Some(OptionsSupportedParentFamily::CompositeTrend)
        );
        assert_eq!(
            supported_options_parent_family_from_kind("RegimeHybrid"),
            Some(OptionsSupportedParentFamily::RegimeHybrid)
        );
        assert_eq!(
            supported_options_parent_family_from_kind("BreakoutMomentum"),
            Some(OptionsSupportedParentFamily::BreakoutMomentum)
        );
        assert_eq!(supported_options_parent_family_from_kind("MlSignal"), None);
    }

    #[test]
    fn build_supported_options_parent_replay_rejects_missing_strategy_kind_metadata() {
        let artifact = artifact_with_replay_payload(None, None, Some(sample_best_params(5)));

        let err = build_supported_options_parent_replay(&artifact, None)
            .expect_err("missing strategy kind should fail");
        assert!(err.contains("missing supported options parent strategy-kind metadata"));
    }

    #[test]
    fn build_supported_options_parent_replay_rejects_unsupported_strategy_kind() {
        let artifact =
            artifact_with_replay_payload(Some("MlSignal"), None, Some(sample_best_params(5)));

        let err = build_supported_options_parent_replay(&artifact, None)
            .expect_err("unsupported strategy kind should fail");
        assert!(err.contains("found strategy kind 'MlSignal'"));
    }

    #[test]
    fn build_supported_options_parent_replay_uses_explicit_nonzero_candidate_index() {
        let artifact = artifact_with_replay_payload(
            Some("DoubleMa"),
            Some(vec![
                json!({ "params": serde_json::to_value(sample_candidate(7, false)).expect("candidate 0") }),
                json!({ "params": serde_json::to_value(sample_candidate(11, true)).expect("candidate 1") }),
            ]),
            Some(sample_best_params(5)),
        );

        let replay = build_supported_options_parent_replay(&artifact, Some(1))
            .expect("candidate 1 replay should succeed");

        assert_eq!(replay.drilldown_request.params.fast_len, 11);
        assert!(replay.trade_on_next_bar);
    }

    #[test]
    fn build_supported_options_parent_replay_fails_closed_for_missing_explicit_candidate() {
        let artifact = artifact_with_replay_payload(
            Some("DoubleMa"),
            Some(vec![json!({
                "params": serde_json::to_value(sample_candidate(7, false)).expect("candidate 0")
            })]),
            Some(sample_best_params(5)),
        );

        let err = build_supported_options_parent_replay(&artifact, Some(1))
            .expect_err("missing explicit candidate should fail");
        assert!(err.contains("requested parent_candidate_index 1"));
    }

    #[test]
    fn build_supported_options_parent_replay_falls_back_to_best_params_when_candidates_are_absent()
    {
        let artifact =
            artifact_with_replay_payload(Some("DoubleMa"), None, Some(sample_best_params(5)));

        let replay = build_supported_options_parent_replay(&artifact, None)
            .expect("best_params fallback should succeed");

        assert_eq!(replay.drilldown_request.params.fast_len, 5);
        assert_eq!(replay.drilldown_request.data_id, "artifact_replay");
        assert_eq!(replay.drilldown_request.bins, Some(256));
    }

    #[test]
    fn build_supported_options_parent_replay_fails_closed_for_malformed_candidate_payload() {
        let artifact = artifact_with_replay_payload(
            Some("DoubleMa"),
            Some(vec![json!({
                "params": {
                    "fast_ma_type": "sma",
                    "slow_ma_type": "ema",
                    "fast_len": 7
                }
            })]),
            Some(sample_best_params(5)),
        );

        let err = build_supported_options_parent_replay(&artifact, None)
            .expect_err("malformed candidate should fail");
        assert!(err.contains("validation candidate"));
    }
}
