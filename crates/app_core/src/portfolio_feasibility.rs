use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_MAX_CANDIDATES: usize = 50;
const HARD_MAX_CANDIDATES: usize = 250;
const SUPPORTED_OPTIONS_FAMILIES: &[&str] = &["long_single", "debit_vertical", "long_pair"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioEvidenceStatus {
    FeasibilityOnly,
    InsufficientCovarianceEvidence,
    ReplayRequired,
    ReplayParityPassed,
    ReplayParityFailed,
    ArtifactProvenanceBlocked,
    UnsupportedCandidateFamily,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioSourceRole {
    ValidationArtifact,
    MetaSelectionArtifact,
    OptionsExpressionArtifact,
    MlSignalArtifact,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioAssetClass {
    Equity,
    Options,
    Mixed,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioReplayEvidenceStatus {
    MissingReplayMatrix,
    ReplayRequired,
    ReplayMatrixPresent,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioFeasibilityRequestV1 {
    #[serde(default)]
    pub source_artifacts: Vec<Value>,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,
    #[serde(default)]
    pub require_approved_validation: bool,
    #[serde(default)]
    pub require_replay_matrix: bool,
    #[serde(default)]
    pub require_source_provenance: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioFeasibilityResultV1 {
    pub version: u32,
    pub status: PortfolioEvidenceStatus,
    pub source_artifacts_seen: usize,
    pub eligible_candidates: Vec<PortfolioCandidateEvidenceV1>,
    pub rejected_candidates: Vec<PortfolioRejectedCandidateV1>,
    pub missing_evidence: Vec<PortfolioMissingEvidenceV1>,
    pub warnings: Vec<String>,
    pub next_required_step: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioCandidateEvidenceV1 {
    pub candidate_id: String,
    pub source_artifact_index: usize,
    pub source_role: PortfolioSourceRole,
    pub strategy_kind: String,
    pub symbol_root: Option<String>,
    pub timeframe: Option<String>,
    pub params_hash: Option<String>,
    pub asset_class: PortfolioAssetClass,
    pub admission: PortfolioAdmissionEvidenceV1,
    pub summary_metrics: PortfolioSummaryMetricsV1,
    pub replay_status: PortfolioReplayEvidenceStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PortfolioAdmissionEvidenceV1 {
    pub validation_passed: Option<bool>,
    pub run_approved: Option<bool>,
    pub final_recommendation: Option<String>,
    pub psr: Option<f64>,
    pub dsr: Option<f64>,
    pub pbo: Option<f64>,
    pub cross_ticker_score: Option<f64>,
    pub robust_score: Option<f64>,
    pub options_validation_verdict: Option<String>,
    pub options_non_promotable: Option<bool>,
    pub options_structure_family: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PortfolioSummaryMetricsV1 {
    pub pnl: Option<f64>,
    pub sharpe: Option<f64>,
    pub sortino: Option<f64>,
    pub calmar: Option<f64>,
    pub omega: Option<f64>,
    pub volatility: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub trades: Option<u32>,
    pub exposure: Option<f64>,
    pub net_exposure: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioRejectedCandidateV1 {
    pub candidate_id: String,
    pub source_artifact_index: usize,
    pub source_role: PortfolioSourceRole,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioMissingEvidenceV1 {
    pub source_artifact_index: Option<usize>,
    pub candidate_id: Option<String>,
    pub evidence_kind: String,
    pub message: String,
}

#[derive(Clone, Debug)]
struct CandidateDraft {
    value: Value,
    source_artifact_index: usize,
    source_role: PortfolioSourceRole,
    artifact_has_replay_matrix: bool,
}

pub fn portfolio_feasibility_from_artifacts(
    req: PortfolioFeasibilityRequestV1,
) -> PortfolioFeasibilityResultV1 {
    let source_artifacts_seen = req.source_artifacts.len();
    let max_candidates = normalize_max_candidates(req.max_candidates);
    let mut eligible_candidates = Vec::new();
    let mut rejected_candidates = Vec::new();
    let mut missing_evidence = Vec::new();
    let mut warnings = Vec::new();

    if req.source_artifacts.is_empty() {
        missing_evidence.push(PortfolioMissingEvidenceV1 {
            source_artifact_index: None,
            candidate_id: None,
            evidence_kind: "source_artifacts".to_string(),
            message: "No source artifacts were provided for portfolio feasibility analysis."
                .to_string(),
        });
    }

    for (source_artifact_index, artifact) in req.source_artifacts.iter().enumerate() {
        let role = detect_source_role(artifact);
        let artifact_has_source_provenance = has_source_provenance(artifact);
        let artifact_has_replay_matrix = has_replay_matrix(artifact);

        if !artifact_has_source_provenance {
            let message = "Source artifact has no durable hash or fingerprint; portfolio evidence remains provenance-limited.".to_string();
            missing_evidence.push(PortfolioMissingEvidenceV1 {
                source_artifact_index: Some(source_artifact_index),
                candidate_id: None,
                evidence_kind: "source_provenance".to_string(),
                message: message.clone(),
            });
            warnings.push(message);
        }

        let drafts = extract_candidate_drafts(
            artifact,
            source_artifact_index,
            role.clone(),
            artifact_has_replay_matrix,
        );

        if role == PortfolioSourceRole::Unknown {
            missing_evidence.push(PortfolioMissingEvidenceV1 {
                source_artifact_index: Some(source_artifact_index),
                candidate_id: None,
                evidence_kind: "supported_artifact_kind".to_string(),
                message:
                    "Artifact shape is not recognized as validation, meta-selection, options, or ML signal evidence."
                        .to_string(),
            });
        }

        if drafts.is_empty() {
            missing_evidence.push(PortfolioMissingEvidenceV1 {
                source_artifact_index: Some(source_artifact_index),
                candidate_id: None,
                evidence_kind: "candidate_rows".to_string(),
                message: "No candidate rows could be normalized from this source artifact."
                    .to_string(),
            });
        }

        for draft in drafts {
            if eligible_candidates
                .len()
                .saturating_add(rejected_candidates.len())
                >= max_candidates
            {
                warnings.push(format!(
                    "Candidate scan stopped at max_candidates={max_candidates}; remaining source rows were ignored."
                ));
                break;
            }

            let candidate = normalize_candidate(draft);
            let mut rejection_reason =
                reject_reason_for_candidate(&candidate, &req, artifact_has_source_provenance);
            if rejection_reason.is_none()
                && req.require_replay_matrix
                && candidate.replay_status != PortfolioReplayEvidenceStatus::ReplayMatrixPresent
            {
                rejection_reason =
                    Some("replay matrix required but not present for this candidate".to_string());
            }

            if let Some(reason) = rejection_reason {
                rejected_candidates.push(PortfolioRejectedCandidateV1 {
                    candidate_id: candidate.candidate_id,
                    source_artifact_index: candidate.source_artifact_index,
                    source_role: candidate.source_role,
                    reason,
                });
            } else {
                if candidate.replay_status == PortfolioReplayEvidenceStatus::MissingReplayMatrix {
                    missing_evidence.push(PortfolioMissingEvidenceV1 {
                        source_artifact_index: Some(candidate.source_artifact_index),
                        candidate_id: Some(candidate.candidate_id.clone()),
                        evidence_kind: "co_timed_replay_matrix".to_string(),
                        message: "Candidate has admission evidence but no co-timed return/exposure matrix.".to_string(),
                    });
                }
                eligible_candidates.push(candidate);
            }
        }
    }

    let status = resolve_status(
        &req,
        &eligible_candidates,
        &rejected_candidates,
        &missing_evidence,
    );
    let next_required_step = next_required_step(&status);

    PortfolioFeasibilityResultV1 {
        version: SCHEMA_VERSION,
        status,
        source_artifacts_seen,
        eligible_candidates,
        rejected_candidates,
        missing_evidence,
        warnings: dedupe_strings(warnings),
        next_required_step,
    }
}

fn default_max_candidates() -> usize {
    DEFAULT_MAX_CANDIDATES
}

fn normalize_max_candidates(value: usize) -> usize {
    if value == 0 {
        DEFAULT_MAX_CANDIDATES
    } else {
        value.min(HARD_MAX_CANDIDATES)
    }
}

fn detect_source_role(value: &Value) -> PortfolioSourceRole {
    let kind = first_text(
        value,
        &[
            "artifact_kind",
            "kind",
            "type",
            "artifact_type",
            "schema",
            "schema_version",
            "run_kind",
        ],
    )
    .unwrap_or_default()
    .to_ascii_lowercase();

    if contains_any(
        &kind,
        &["options_expression", "options-expression", "option"],
    ) {
        return PortfolioSourceRole::OptionsExpressionArtifact;
    }
    if contains_any(&kind, &["meta_selection", "meta-select", "meta"]) {
        return PortfolioSourceRole::MetaSelectionArtifact;
    }
    if contains_any(&kind, &["ml_signal", "ml-signal"]) {
        return PortfolioSourceRole::MlSignalArtifact;
    }
    if contains_any(&kind, &["validation", "double_ma", "composite"]) {
        return PortfolioSourceRole::ValidationArtifact;
    }

    if value.get("tuple_results").is_some()
        || value.get("selected_tuples").is_some()
        || value.get("options_dataset_id").is_some()
    {
        return PortfolioSourceRole::OptionsExpressionArtifact;
    }
    if value.get("groups").is_some()
        || value.get("meta_groups").is_some()
        || value.get("meta_selection").is_some()
    {
        return PortfolioSourceRole::MetaSelectionArtifact;
    }
    if value.get("candidates").is_some()
        || value.get("finalists").is_some()
        || value.get("shortlist").is_some()
    {
        return PortfolioSourceRole::ValidationArtifact;
    }

    PortfolioSourceRole::Unknown
}

fn extract_candidate_drafts(
    artifact: &Value,
    source_artifact_index: usize,
    role: PortfolioSourceRole,
    artifact_has_replay_matrix: bool,
) -> Vec<CandidateDraft> {
    let mut values = Vec::new();
    match role {
        PortfolioSourceRole::OptionsExpressionArtifact => {
            collect_array_paths(
                artifact,
                &[
                    "tuple_results",
                    "selected_tuples",
                    "tuples",
                    "results",
                    "candidates",
                    "rows",
                ],
                &mut values,
            );
        }
        PortfolioSourceRole::MetaSelectionArtifact => {
            collect_array_paths(
                artifact,
                &[
                    "groups",
                    "meta_groups",
                    "clusters",
                    "selected_groups",
                    "candidates",
                    "rows",
                ],
                &mut values,
            );
        }
        PortfolioSourceRole::ValidationArtifact
        | PortfolioSourceRole::MlSignalArtifact
        | PortfolioSourceRole::Unknown => {
            collect_array_paths(
                artifact,
                &[
                    "candidates",
                    "finalists",
                    "shortlist",
                    "top",
                    "results",
                    "rows",
                    "validation_results",
                    "selected_candidates",
                ],
                &mut values,
            );
        }
    }

    values
        .into_iter()
        .map(|value| CandidateDraft {
            value,
            source_artifact_index,
            source_role: role.clone(),
            artifact_has_replay_matrix,
        })
        .collect()
}

fn collect_array_paths(value: &Value, keys: &[&str], out: &mut Vec<Value>) {
    for key in keys {
        if let Some(items) = value.get(*key).and_then(Value::as_array) {
            out.extend(items.iter().cloned());
        }
    }
    if let Some(obj) = value.as_object() {
        for nested in obj.values() {
            if nested.is_object() {
                collect_array_paths(nested, keys, out);
            }
        }
    }
}

fn normalize_candidate(draft: CandidateDraft) -> PortfolioCandidateEvidenceV1 {
    let candidate_source = representative_candidate_value(&draft.value).unwrap_or(&draft.value);
    let params_hash = canonical_candidate_params_hash(candidate_source);
    let candidate_id = first_text(
        candidate_source,
        &[
            "candidate_id",
            "id",
            "tuple_id",
            "params_hash",
            "config_fingerprint",
            "fingerprint",
        ],
    )
    .or_else(|| Some(format!("portfolio_candidate_{}", params_hash)));
    let admission = admission_evidence(candidate_source, draft.source_role.clone());
    let asset_class = infer_asset_class(candidate_source, draft.source_role.clone());
    let replay_status = if draft.artifact_has_replay_matrix || has_replay_matrix(candidate_source) {
        PortfolioReplayEvidenceStatus::ReplayMatrixPresent
    } else if draft.source_role == PortfolioSourceRole::MetaSelectionArtifact
        && representative_candidate_value(&draft.value).is_none()
    {
        PortfolioReplayEvidenceStatus::NotApplicable
    } else {
        PortfolioReplayEvidenceStatus::MissingReplayMatrix
    };

    PortfolioCandidateEvidenceV1 {
        candidate_id: candidate_id.unwrap_or_else(|| format!("portfolio_candidate_{params_hash}")),
        source_artifact_index: draft.source_artifact_index,
        source_role: draft.source_role.clone(),
        strategy_kind: infer_strategy_kind(candidate_source, draft.source_role),
        symbol_root: infer_symbol_root(candidate_source),
        timeframe: infer_timeframe(candidate_source),
        params_hash: Some(params_hash),
        asset_class,
        admission,
        summary_metrics: summary_metrics(candidate_source),
        replay_status,
    }
}

fn representative_candidate_value(value: &Value) -> Option<&Value> {
    for key in [
        "representative_candidate",
        "representative",
        "best_candidate",
        "selected_candidate",
    ] {
        if let Some(candidate) = value.get(key).filter(|v| v.is_object()) {
            return Some(candidate);
        }
    }
    None
}

fn admission_evidence(value: &Value, role: PortfolioSourceRole) -> PortfolioAdmissionEvidenceV1 {
    let final_recommendation = first_text(
        value,
        &[
            "final_recommendation",
            "recommendation",
            "run_verdict",
            "verdict",
            "status",
        ],
    );
    let options_validation_verdict = first_text(
        value,
        &[
            "options_validation_verdict",
            "validation_verdict",
            "verdict",
        ],
    );
    let options_structure_family = normalize_family(
        first_text(
            value,
            &["structure_family", "family", "options_structure_family"],
        )
        .as_deref(),
    );
    let options_non_promotable = first_bool(
        value,
        &[
            "options_non_promotable",
            "non_promotable",
            "non_promotable_candidate",
        ],
    )
    .or_else(|| first_bool(value, &["promotable"]).map(|value| !value));
    let validation_passed = first_bool(
        value,
        &["validation_passed", "passed", "approved", "run_approved"],
    )
    .or_else(|| verdict_passed(final_recommendation.as_deref()))
    .or_else(|| verdict_passed(options_validation_verdict.as_deref()));

    PortfolioAdmissionEvidenceV1 {
        validation_passed,
        run_approved: first_bool(value, &["run_approved", "approved"]),
        final_recommendation,
        psr: first_number(value, &["psr", "probabilistic_sharpe_ratio"]),
        dsr: first_number(value, &["dsr", "deflated_sharpe_ratio"]),
        pbo: first_number(value, &["pbo", "cscv_pbo"]),
        cross_ticker_score: first_number(value, &["cross_ticker_score", "cross_score"]),
        robust_score: first_number(value, &["robust_score", "meta_score"]),
        options_validation_verdict: if role == PortfolioSourceRole::OptionsExpressionArtifact {
            options_validation_verdict
        } else {
            None
        },
        options_non_promotable,
        options_structure_family: if role == PortfolioSourceRole::OptionsExpressionArtifact {
            options_structure_family
        } else {
            None
        },
    }
}

fn summary_metrics(value: &Value) -> PortfolioSummaryMetricsV1 {
    let metrics = nested_object(value, &["metrics", "summary_metrics", "validation_metrics"])
        .unwrap_or(value);
    PortfolioSummaryMetricsV1 {
        pnl: first_number(metrics, &["pnl", "total_pnl", "net_pnl"]),
        sharpe: first_number(metrics, &["sharpe", "sharpe_ratio"]),
        sortino: first_number(metrics, &["sortino", "sortino_ratio"]),
        calmar: first_number(metrics, &["calmar", "calmar_ratio"]),
        omega: first_number(metrics, &["omega", "omega_ratio"]),
        volatility: first_number(metrics, &["volatility", "vol", "annualized_volatility"]),
        max_drawdown: first_number(metrics, &["max_drawdown", "max_dd", "drawdown"]),
        trades: first_u32(metrics, &["trades", "trade_count", "num_trades"]),
        exposure: first_number(metrics, &["exposure", "gross_exposure", "exposure_pct"]),
        net_exposure: first_number(metrics, &["net_exposure", "net_exposure_pct"]),
    }
}

fn reject_reason_for_candidate(
    candidate: &PortfolioCandidateEvidenceV1,
    req: &PortfolioFeasibilityRequestV1,
    artifact_has_source_provenance: bool,
) -> Option<String> {
    if req.require_source_provenance && !artifact_has_source_provenance {
        return Some("source artifact lacks a durable hash or fingerprint".to_string());
    }
    if candidate.source_role == PortfolioSourceRole::Unknown {
        return Some("unsupported artifact kind".to_string());
    }
    if candidate.source_role == PortfolioSourceRole::MetaSelectionArtifact
        && candidate.replay_status == PortfolioReplayEvidenceStatus::NotApplicable
    {
        return Some(
            "meta-selection cohort has no representative candidate for portfolio admission"
                .to_string(),
        );
    }
    if req.require_approved_validation && candidate.admission.validation_passed != Some(true) {
        return Some("approved validation evidence is required but not present".to_string());
    }
    if candidate.admission.validation_passed == Some(false) {
        return Some("candidate validation verdict is failed or rejected".to_string());
    }
    if candidate.source_role == PortfolioSourceRole::OptionsExpressionArtifact {
        if candidate.admission.options_non_promotable == Some(true) {
            return Some("options candidate is marked non-promotable".to_string());
        }
        let family = candidate.admission.options_structure_family.clone();
        if family
            .as_deref()
            .is_some_and(|family| !SUPPORTED_OPTIONS_FAMILIES.contains(&family))
        {
            return Some(format!(
                "unsupported options structure family: {}",
                family.unwrap()
            ));
        }
    }
    None
}

fn resolve_status(
    req: &PortfolioFeasibilityRequestV1,
    eligible: &[PortfolioCandidateEvidenceV1],
    rejected: &[PortfolioRejectedCandidateV1],
    missing: &[PortfolioMissingEvidenceV1],
) -> PortfolioEvidenceStatus {
    if req.require_source_provenance
        && missing
            .iter()
            .any(|row| row.evidence_kind == "source_provenance")
    {
        return PortfolioEvidenceStatus::ArtifactProvenanceBlocked;
    }
    if eligible.is_empty()
        && rejected
            .iter()
            .any(|row| row.reason.contains("unsupported"))
    {
        return PortfolioEvidenceStatus::UnsupportedCandidateFamily;
    }
    if eligible.is_empty() {
        return PortfolioEvidenceStatus::InsufficientCovarianceEvidence;
    }
    if eligible
        .iter()
        .any(|row| row.replay_status != PortfolioReplayEvidenceStatus::ReplayMatrixPresent)
    {
        return PortfolioEvidenceStatus::InsufficientCovarianceEvidence;
    }
    PortfolioEvidenceStatus::ReplayRequired
}

fn next_required_step(status: &PortfolioEvidenceStatus) -> String {
    match status {
        PortfolioEvidenceStatus::ArtifactProvenanceBlocked => {
            "Provide source artifacts with durable hashes or run fingerprints before portfolio research."
        }
        PortfolioEvidenceStatus::UnsupportedCandidateFamily => {
            "Provide validation, meta-selection, options-expression, or ML-signal artifacts with supported candidate rows."
        }
        PortfolioEvidenceStatus::InsufficientCovarianceEvidence => {
            "Implement or provide a co-timed return/exposure replay matrix and parity checks before any allocation."
        }
        PortfolioEvidenceStatus::ReplayRequired => {
            "Run replay parity verification before enabling covariance diagnostics or allocation research."
        }
        PortfolioEvidenceStatus::ReplayParityPassed => {
            "Covariance diagnostics may be considered, but allocation remains research-only."
        }
        PortfolioEvidenceStatus::ReplayParityFailed => {
            "Fix replay drift before using portfolio evidence."
        }
        PortfolioEvidenceStatus::FeasibilityOnly => {
            "Review feasibility diagnostics; no allocation output is available."
        }
    }
    .to_string()
}

fn has_source_provenance(value: &Value) -> bool {
    first_text(
        value,
        &[
            "source_hash",
            "artifact_hash",
            "artifact_sha256",
            "source_run_config_fingerprint",
            "config_fingerprint",
            "dataset_hash",
            "provenance_hash",
        ],
    )
    .is_some()
        || nested_object(value, &["provenance", "artifact_provenance"])
            .and_then(|nested| {
                first_text(
                    nested,
                    &["hash", "sha256", "source_hash", "config_fingerprint"],
                )
            })
            .is_some()
}

fn has_replay_matrix(value: &Value) -> bool {
    [
        "co_timed_return_matrix",
        "return_matrix",
        "replay_matrix",
        "portfolio_replay_matrix",
    ]
    .iter()
    .any(|key| value.get(*key).is_some())
}

fn infer_strategy_kind(value: &Value, role: PortfolioSourceRole) -> String {
    first_text(
        value,
        &[
            "strategy_kind",
            "strategy",
            "family",
            "profile",
            "route_family",
            "source_strategy",
        ],
    )
    .unwrap_or_else(|| match role {
        PortfolioSourceRole::OptionsExpressionArtifact => "options_expression".to_string(),
        PortfolioSourceRole::MetaSelectionArtifact => "meta_selection".to_string(),
        PortfolioSourceRole::MlSignalArtifact => "ml_signal".to_string(),
        PortfolioSourceRole::ValidationArtifact => "validation_candidate".to_string(),
        PortfolioSourceRole::Unknown => "unknown".to_string(),
    })
}

fn infer_asset_class(value: &Value, role: PortfolioSourceRole) -> PortfolioAssetClass {
    let text = first_text(value, &["asset_class", "market", "instrument_type", "kind"])
        .unwrap_or_default()
        .to_ascii_lowercase();
    if role == PortfolioSourceRole::OptionsExpressionArtifact
        || contains_any(&text, &["option", "options"])
    {
        return PortfolioAssetClass::Options;
    }
    if contains_any(&text, &["stock", "equity", "equities"]) {
        return PortfolioAssetClass::Equity;
    }
    if contains_any(&text, &["mixed", "multi_asset"]) {
        return PortfolioAssetClass::Mixed;
    }
    PortfolioAssetClass::Unknown
}

fn infer_symbol_root(value: &Value) -> Option<String> {
    first_text(
        value,
        &[
            "symbol_root",
            "root",
            "symbol",
            "ticker",
            "polygon_ticker",
            "underlying",
            "underlying_symbol",
        ],
    )
    .map(|symbol| {
        let raw = symbol.trim().to_ascii_uppercase();
        raw.split(|ch: char| !ch.is_ascii_alphanumeric())
            .find(|part| !part.is_empty())
            .unwrap_or(raw.as_str())
            .to_string()
    })
}

fn infer_timeframe(value: &Value) -> Option<String> {
    first_text(value, &["timeframe", "bar_timeframe", "tf", "interval"])
}

fn verdict_passed(value: Option<&str>) -> Option<bool> {
    let value = value?.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    if contains_any(
        &value,
        &[
            "fail",
            "failed",
            "reject",
            "rejected",
            "non_promotable",
            "blocked",
        ],
    ) {
        return Some(false);
    }
    if contains_any(
        &value,
        &["pass", "passed", "approved", "promote", "accepted", "valid"],
    ) {
        return Some(true);
    }
    None
}

fn canonical_candidate_params_hash(value: &Value) -> String {
    let canonical = canonicalize_value(value);
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    hex_lower(&digest[..])
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_value).collect()),
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = Map::new();
            for key in keys {
                if let Some(value) = map.get(&key) {
                    out.insert(key, canonicalize_value(value));
                }
            }
            Value::Object(out)
        }
        _ => value.clone(),
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn first_text(value: &Value, keys: &[&str]) -> Option<String> {
    find_first(value, keys).and_then(|value| match value {
        Value::String(text) => {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_string())
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    })
}

fn first_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    find_first(value, keys).and_then(|value| match value {
        Value::Bool(value) => Some(*value),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "pass" | "passed" | "approved" => Some(true),
            "false" | "no" | "fail" | "failed" | "rejected" => Some(false),
            _ => None,
        },
        _ => None,
    })
}

fn first_number(value: &Value, keys: &[&str]) -> Option<f64> {
    find_first(value, keys).and_then(|value| match value {
        Value::Number(number) => number.as_f64().filter(|value| value.is_finite()),
        Value::String(text) => text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite()),
        _ => None,
    })
}

fn first_u32(value: &Value, keys: &[&str]) -> Option<u32> {
    find_first(value, keys).and_then(|value| match value {
        Value::Number(number) => number.as_u64().and_then(|value| u32::try_from(value).ok()),
        Value::String(text) => text.trim().parse::<u32>().ok(),
        _ => None,
    })
}

fn nested_object<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    find_first(value, keys).filter(|value| value.is_object())
}

fn find_first<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    for key in keys {
        if let Some(found) = find_key_case_insensitive(value, key) {
            return Some(found);
        }
    }
    None
}

fn find_key_case_insensitive<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let obj = value.as_object()?;
    if let Some(value) = obj.get(key) {
        return Some(value);
    }
    let needle = normalize_key(key);
    obj.iter()
        .find(|(candidate, _)| normalize_key(candidate) == needle)
        .map(|(_, value)| value)
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_family(value: Option<&str>) -> Option<String> {
    value.map(|value| {
        let mut out = String::new();
        for (idx, ch) in value.trim().chars().enumerate() {
            if ch.is_ascii_uppercase() && idx > 0 {
                out.push('_');
            }
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
            } else if !out.ends_with('_') {
                out.push('_');
            }
        }
        out.trim_matches('_').to_string()
    })
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if value.trim().is_empty() || out.contains(&value) {
            continue;
        }
        out.push(value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serde_round_trip_preserves_request_and_response_contract() {
        let req = PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({"artifact_kind": "validation", "candidates": []})],
            max_candidates: 10,
            require_approved_validation: true,
            require_replay_matrix: false,
            require_source_provenance: true,
        };
        let encoded = serde_json::to_string(&req).expect("serialize request");
        let decoded: PortfolioFeasibilityRequestV1 =
            serde_json::from_str(&encoded).expect("deserialize request");
        assert_eq!(decoded.max_candidates, 10);
        assert!(decoded.require_source_provenance);

        let result = portfolio_feasibility_from_artifacts(decoded);
        let encoded = serde_json::to_string(&result).expect("serialize result");
        let decoded: PortfolioFeasibilityResultV1 =
            serde_json::from_str(&encoded).expect("deserialize result");
        assert_eq!(decoded.version, SCHEMA_VERSION);
        assert_eq!(decoded.source_artifacts_seen, 1);
    }

    #[test]
    fn unknown_artifact_reports_missing_evidence_without_panicking() {
        let result = portfolio_feasibility_from_artifacts(PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({"not": "a portfolio artifact"})],
            ..test_request()
        });

        assert_eq!(
            result.status,
            PortfolioEvidenceStatus::InsufficientCovarianceEvidence
        );
        assert!(result
            .missing_evidence
            .iter()
            .any(|row| row.evidence_kind == "supported_artifact_kind"));
    }

    #[test]
    fn validation_candidate_is_admitted_but_missing_replay_blocks_allocation() {
        let result = portfolio_feasibility_from_artifacts(PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({
                "artifact_kind": "double_ma_validation",
                "artifact_hash": "hash-1",
                "candidates": [{
                    "candidate_id": "aapl-fast-slow",
                    "strategy_kind": "double_ma",
                    "symbol": "AAPL",
                    "timeframe": "1m",
                    "validation_passed": true,
                    "metrics": {"sharpe": 1.25, "max_drawdown": -0.12, "trades": 42}
                }]
            })],
            require_approved_validation: true,
            ..test_request()
        });

        assert_eq!(
            result.status,
            PortfolioEvidenceStatus::InsufficientCovarianceEvidence
        );
        assert_eq!(result.eligible_candidates.len(), 1);
        assert!(result.rejected_candidates.is_empty());
        assert_eq!(
            result.eligible_candidates[0].symbol_root.as_deref(),
            Some("AAPL")
        );
        assert_eq!(
            result.eligible_candidates[0].summary_metrics.trades,
            Some(42)
        );
        assert!(result
            .missing_evidence
            .iter()
            .any(|row| row.evidence_kind == "co_timed_replay_matrix"));
        let serialized = serde_json::to_value(&result).expect("serialize result");
        assert!(serialized.get("target_weights").is_none());
        assert!(serialized.get("allocation_frontier").is_none());
    }

    #[test]
    fn optimization_top_rows_are_admitted_from_run_artifacts() {
        let result = portfolio_feasibility_from_artifacts(PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({
                "artifact_kind": "validation_artifact_bundle",
                "config_fingerprint": "cfg-123",
                "run_res": {
                    "top": [{
                        "candidate_id": "top-row-1",
                        "strategy_kind": "double_ma",
                        "symbol": "SPY",
                        "timeframe": "5m",
                        "validation_passed": true,
                        "metrics": {"sharpe": 1.7, "trades": 18}
                    }]
                }
            })],
            require_approved_validation: true,
            ..test_request()
        });

        assert_eq!(result.source_artifacts_seen, 1);
        assert_eq!(result.eligible_candidates.len(), 1);
        assert_eq!(result.eligible_candidates[0].candidate_id, "top-row-1");
        assert_eq!(
            result.eligible_candidates[0].source_role,
            PortfolioSourceRole::ValidationArtifact
        );
        assert_eq!(
            result.eligible_candidates[0].symbol_root.as_deref(),
            Some("SPY")
        );
    }

    #[test]
    fn failed_validation_candidate_is_rejected() {
        let result = portfolio_feasibility_from_artifacts(PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({
                "artifact_kind": "validation",
                "artifact_hash": "hash-1",
                "candidates": [{"candidate_id": "bad", "run_verdict": "failed"}]
            })],
            require_approved_validation: true,
            ..test_request()
        });

        assert!(result.eligible_candidates.is_empty());
        assert_eq!(result.rejected_candidates.len(), 1);
        assert!(result.rejected_candidates[0].reason.contains("approved"));
    }

    #[test]
    fn options_non_promotable_and_unsupported_structures_are_rejected() {
        let result = portfolio_feasibility_from_artifacts(PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({
                "artifact_kind": "options_expression",
                "artifact_hash": "hash-1",
                "tuple_results": [
                    {
                        "tuple_id": "long-ok",
                        "structure_family": "LongSingle",
                        "validation_verdict": "passed",
                        "non_promotable": false,
                        "underlying_symbol": "MSFT"
                    },
                    {
                        "tuple_id": "wide-no",
                        "structure_family": "DebitVertical",
                        "validation_verdict": "passed",
                        "non_promotable": true
                    },
                    {
                        "tuple_id": "credit-no",
                        "structure_family": "CreditSpread",
                        "validation_verdict": "passed"
                    }
                ]
            })],
            ..test_request()
        });

        assert_eq!(result.eligible_candidates.len(), 1);
        assert_eq!(result.rejected_candidates.len(), 2);
        assert_eq!(
            result.eligible_candidates[0].asset_class,
            PortfolioAssetClass::Options
        );
    }

    #[test]
    fn source_provenance_can_be_required_as_a_blocking_gate() {
        let result = portfolio_feasibility_from_artifacts(PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({
                "artifact_kind": "validation",
                "candidates": [{"candidate_id": "candidate-1", "validation_passed": true}]
            })],
            require_source_provenance: true,
            ..test_request()
        });

        assert_eq!(
            result.status,
            PortfolioEvidenceStatus::ArtifactProvenanceBlocked
        );
        assert!(result
            .rejected_candidates
            .iter()
            .any(|row| row.reason.contains("durable hash")));
    }

    #[test]
    fn meta_group_without_representative_is_rejected_as_cohort_only() {
        let result = portfolio_feasibility_from_artifacts(PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({
                "artifact_kind": "meta_selection",
                "artifact_hash": "hash-1",
                "groups": [{"group_id": "cluster-1", "meta_score": 0.9}]
            })],
            ..test_request()
        });

        assert!(result.eligible_candidates.is_empty());
        assert_eq!(result.rejected_candidates.len(), 1);
        assert!(result.rejected_candidates[0]
            .reason
            .contains("representative"));
    }

    fn test_request() -> PortfolioFeasibilityRequestV1 {
        PortfolioFeasibilityRequestV1 {
            source_artifacts: Vec::new(),
            max_candidates: DEFAULT_MAX_CANDIDATES,
            require_approved_validation: false,
            require_replay_matrix: false,
            require_source_provenance: false,
        }
    }
}
