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

#[derive(Clone)]
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

fn resolve_strategy_kind(value: &Value) -> String {
    non_empty_string(value.pointer("/run/req/strategy_kind"))
        .or_else(|| {
            non_empty_string(value.pointer("/validation/req/validation_context/strategy_kind"))
        })
        .or_else(|| non_empty_string(value.pointer("/validation/req/strategy_kind")))
        .unwrap_or_default()
}

pub fn supported_options_parent_strategy_list() -> &'static str {
    "DoubleMa, CompositeTrend, RegimeHybrid, and BreakoutMomentum"
}

pub fn supported_options_parent_family_from_artifact(
    artifact: &Value,
) -> Option<OptionsSupportedParentFamily> {
    supported_options_parent_family_from_kind(&resolve_strategy_kind(artifact))
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
    let family = supported_options_parent_family_from_artifact(artifact).ok_or_else(|| {
        format!(
            "options expression currently supports {} parent artifacts only",
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
}
