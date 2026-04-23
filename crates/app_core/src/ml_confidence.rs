use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

fn is_meta_selection_kind(kind: &str) -> bool {
    matches!(
        kind.trim(),
        "vectorbt_meta_selection" | "vectorgrid_meta_selection"
    )
}

fn default_min_holdout_pass_rate() -> f64 {
    0.60
}

fn default_min_holdout_score_p10() -> f64 {
    0.0
}

fn default_min_holdout_datasets() -> usize {
    30
}

fn default_model_target() -> MlConfidenceTarget {
    MlConfidenceTarget::HoldoutGate
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceScope {
    pub strategy_kind: String,
    pub timeframe: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceLabelConfig {
    #[serde(default = "default_min_holdout_pass_rate")]
    pub min_holdout_pass_rate: f64,
    #[serde(default = "default_min_holdout_score_p10")]
    pub min_holdout_score_p10: f64,
    #[serde(default)]
    pub max_holdout_max_dd_p90: Option<f64>,
    #[serde(default = "default_min_holdout_datasets")]
    pub min_holdout_datasets: usize,
}

impl Default for MlConfidenceLabelConfig {
    fn default() -> Self {
        Self {
            min_holdout_pass_rate: default_min_holdout_pass_rate(),
            min_holdout_score_p10: default_min_holdout_score_p10(),
            max_holdout_max_dd_p90: None,
            min_holdout_datasets: default_min_holdout_datasets(),
        }
    }
}

fn default_l2() -> f64 {
    1.0
}

fn default_learning_rate() -> f64 {
    0.15
}

fn default_max_iter() -> usize {
    300
}

fn default_k_folds() -> usize {
    5
}

fn default_model_family() -> MlConfidenceModelFamily {
    MlConfidenceModelFamily::Auto
}

fn default_gbdt_rounds() -> usize {
    120
}

fn default_gbdt_learning_rate() -> f64 {
    0.08
}

fn default_gbdt_max_bins() -> usize {
    48
}

fn default_gbdt_min_leaf() -> usize {
    8
}

fn default_model_kind() -> MlConfidenceModelKind {
    MlConfidenceModelKind::Logistic
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlConfidenceTarget {
    /// Pick the best target based on available artifacts:
    /// - Prefer `holdout_rows` (per-dataset labels) when present.
    /// - Fall back to `holdout` aggregates otherwise.
    Auto,
    /// Binary label computed from per-group `holdout` aggregates (v1 behavior).
    HoldoutGate,
    /// Binary label per holdout dataset row (`holdout_rows`), using the label thresholds.
    HoldoutDatasetPass,
}

impl Default for MlConfidenceTarget {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlConfidenceModelFamily {
    /// Prefer non-linear boosted stumps when enough labeled rows exist;
    /// fall back to logistic when needed.
    Auto,
    /// Linear baseline model.
    Logistic,
    /// Non-linear boosted tree stumps (tabular-friendly, lightweight).
    GbdtStump,
}

impl Default for MlConfidenceModelFamily {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlConfidenceModelKind {
    Logistic,
    GbdtStump,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MlConfidenceTargetResolved {
    HoldoutGate,
    HoldoutDatasetPass,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceTrainConfig {
    #[serde(default)]
    pub label: MlConfidenceLabelConfig,
    #[serde(default)]
    pub target: MlConfidenceTarget,
    #[serde(default = "default_l2")]
    pub l2: f64,
    #[serde(default = "default_learning_rate")]
    pub learning_rate: f64,
    #[serde(default = "default_max_iter")]
    pub max_iter: usize,
    #[serde(default = "default_k_folds")]
    pub k_folds: usize,
    #[serde(default = "default_model_family")]
    pub model_family: MlConfidenceModelFamily,
    #[serde(default = "default_gbdt_rounds")]
    pub gbdt_rounds: usize,
    #[serde(default = "default_gbdt_learning_rate")]
    pub gbdt_learning_rate: f64,
    #[serde(default = "default_gbdt_max_bins")]
    pub gbdt_max_bins: usize,
    #[serde(default = "default_gbdt_min_leaf")]
    pub gbdt_min_leaf: usize,
}

impl Default for MlConfidenceTrainConfig {
    fn default() -> Self {
        Self {
            label: MlConfidenceLabelConfig::default(),
            target: MlConfidenceTarget::default(),
            l2: default_l2(),
            learning_rate: default_learning_rate(),
            max_iter: default_max_iter(),
            k_folds: default_k_folds(),
            model_family: default_model_family(),
            gbdt_rounds: default_gbdt_rounds(),
            gbdt_learning_rate: default_gbdt_learning_rate(),
            gbdt_max_bins: default_gbdt_max_bins(),
            gbdt_min_leaf: default_gbdt_min_leaf(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceCalibrationBin {
    pub bin: usize,
    pub count: usize,
    pub p_mean: f64,
    pub y_mean: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceTrainMetrics {
    pub rows: usize,
    pub positives: usize,
    pub negatives: usize,
    pub target: MlConfidenceTarget,
    #[serde(default = "default_model_kind")]
    pub model_kind: MlConfidenceModelKind,
    pub train_auc: f64,
    pub train_brier: f64,
    pub baseline_brier: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub train_brier_calibrated: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cv_auc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cv_brier: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cv_brier_calibrated: Option<f64>,
    pub healthy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_artifacts_used: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_meta_ts_used: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calibration: Vec<MlConfidenceCalibrationBin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceCalibration {
    /// Platt scaling on logit(p): p_cal = sigmoid(a * logit(p_raw) + b)
    pub method: String,
    pub a: f64,
    pub b: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceModelMeta {
    pub trained_at_ms: u64,
    pub trained_rows: usize,
    pub trained_positives: usize,
    pub trained_negatives: usize,
    #[serde(default = "default_model_target")]
    pub target: MlConfidenceTarget,
    #[serde(default = "default_model_kind")]
    pub model_kind: MlConfidenceModelKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_key: Option<MlConfidenceCompatibilityKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_artifacts_used: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_meta_ts_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cv_auc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cv_brier: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cv_brier_calibrated: Option<f64>,
    pub baseline_brier: f64,
    pub healthy: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MlConfidenceCompatibilityKey {
    #[serde(default)]
    pub target_family: String,
    #[serde(default)]
    pub train_score_semantics: String,
    #[serde(default)]
    pub grouping_mode: String,
    #[serde(default)]
    pub shortlist_policy: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceModel {
    pub version: u32,
    pub scope: MlConfidenceScope,
    pub feature_names: Vec<String>,
    pub mean: Vec<f64>,
    pub std: Vec<f64>,
    pub weights: Vec<f64>,
    pub bias: f64,
    #[serde(default = "default_model_kind")]
    pub model_kind: MlConfidenceModelKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gbdt: Option<MlConfidenceGbdtModel>,
    pub label: MlConfidenceLabelConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration: Option<MlConfidenceCalibration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MlConfidenceModelMeta>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceModelSummary {
    pub version: u32,
    pub scope: MlConfidenceScope,
    #[serde(default = "default_model_kind")]
    pub model_kind: MlConfidenceModelKind,
    pub label: MlConfidenceLabelConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MlConfidenceModelMeta>,
}

impl From<&MlConfidenceModel> for MlConfidenceModelSummary {
    fn from(model: &MlConfidenceModel) -> Self {
        Self {
            version: model.version,
            scope: model.scope.clone(),
            model_kind: model.model_kind.clone(),
            label: model.label.clone(),
            meta: model.meta.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceGroupFeatures {
    pub samples: u64,
    pub pass_rate: f64,
    pub robust_score_p10: f64,
    pub robust_score_median: f64,
    pub max_dd_p90: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_score_median: Option<f64>,
    pub meta_score: f64,
}

impl MlConfidenceGroupFeatures {
    fn to_feature_vec(&self) -> Option<Vec<f64>> {
        let cross_score_median = self.cross_score_median?;
        let x = vec![
            self.samples as f64,
            self.pass_rate,
            self.robust_score_p10,
            self.robust_score_median,
            self.max_dd_p90,
            cross_score_median,
            self.meta_score,
        ];
        if x.iter().all(|v| v.is_finite()) {
            Some(x)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct MlConfidenceDatasetRow {
    pub group_id: String,
    pub cv_group_id: String,
    pub x: Vec<f64>,
    pub y: f64,
}

#[derive(Clone, Debug)]
pub struct MlConfidencePreparedArtifactRows {
    pub ts: Option<String>,
    pub compatibility_key: MlConfidenceCompatibilityKey,
    pub holdout_gate_rows: Vec<MlConfidenceDatasetRow>,
    pub holdout_dataset_rows: Vec<MlConfidenceDatasetRow>,
}

#[derive(Clone, Debug)]
pub struct MlConfidencePreparedRows {
    pub scope: MlConfidenceScope,
    pub compatibility_key: Option<MlConfidenceCompatibilityKey>,
    pub artifacts: Vec<MlConfidencePreparedArtifactRows>,
    pub has_holdout_dataset_rows: bool,
    pub skipped_incompatible_artifacts: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceTreeStump {
    pub feature_index: usize,
    pub threshold: f64,
    pub left_value: f64,
    pub right_value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceGbdtModel {
    pub base_logit: f64,
    pub learning_rate: f64,
    pub trees: Vec<MlConfidenceTreeStump>,
}

const FEATURE_NAMES: [&str; 7] = [
    "samples",
    "pass_rate",
    "robust_score_p10",
    "robust_score_median",
    "max_dd_p90",
    "cross_score_median",
    "meta_score",
];

type DatasetRow = MlConfidenceDatasetRow;

#[derive(Clone, Debug)]
struct DatasetBuildSummary {
    used_meta: usize,
    newest_meta_ts: Option<String>,
    target: MlConfidenceTargetResolved,
    compatibility_key: Option<MlConfidenceCompatibilityKey>,
}

fn meta_artifact_grouping_mode(meta: &Value) -> String {
    meta.pointer("/req/grouping_mode")
        .and_then(|v| v.as_str())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "legacy_unknown".to_string())
}

fn meta_artifact_shortlist_policy(meta: &Value) -> String {
    let strict = meta
        .pointer("/req/strict_shortlist")
        .and_then(|v| v.as_bool());
    let fallback = meta
        .pointer("/req/allow_shortlist_fallback")
        .and_then(|v| v.as_bool());
    match (strict, fallback) {
        (Some(true), Some(true)) => "strict_with_fallback".to_string(),
        (Some(true), Some(false)) => "strict_no_fallback".to_string(),
        (Some(false), Some(true)) => "permissive_with_fallback".to_string(),
        (Some(false), Some(false)) => "permissive_no_fallback".to_string(),
        _ => "legacy_unknown".to_string(),
    }
}

fn meta_artifact_cv_group_id(meta: &Value, ts: Option<&str>) -> String {
    let payload = serde_json::to_string(meta).unwrap_or_default();
    let fingerprint = fnv1a64(&payload);
    match ts.map(str::trim).filter(|value| !value.is_empty()) {
        Some(ts) => format!("{ts}#{fingerprint:016x}"),
        None => format!("meta-run#{fingerprint:016x}"),
    }
}

fn meta_artifact_target_family(meta: &Value) -> String {
    let has_dataset_rows = meta
        .pointer("/res/groups")
        .and_then(|v| v.as_array())
        .map(|groups| {
            groups.iter().any(|group| {
                group
                    .get("holdout_rows")
                    .and_then(|v| v.as_array())
                    .map(|rows| !rows.is_empty())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    if has_dataset_rows {
        "holdout_dataset_pass_available".to_string()
    } else {
        "holdout_gate_only".to_string()
    }
}

fn meta_artifact_train_score_semantics(meta: &Value) -> String {
    let missing_cross_score = meta
        .pointer("/res/groups")
        .and_then(|v| v.as_array())
        .map(|groups| {
            groups
                .iter()
                .any(|group| group.get("cross_score_median").is_none())
        })
        .unwrap_or(true);
    if missing_cross_score {
        "meta_score_v1_missing_cross_score".to_string()
    } else {
        "meta_score_v1_full".to_string()
    }
}

fn compatibility_key_for_meta_artifact(meta: &Value) -> MlConfidenceCompatibilityKey {
    MlConfidenceCompatibilityKey {
        target_family: meta_artifact_target_family(meta),
        train_score_semantics: meta_artifact_train_score_semantics(meta),
        grouping_mode: meta_artifact_grouping_mode(meta),
        shortlist_policy: meta_artifact_shortlist_policy(meta),
    }
}

fn compatibility_key_label(key: &MlConfidenceCompatibilityKey) -> String {
    format!(
        "target={} | score={} | grouping={} | shortlist={}",
        key.target_family, key.train_score_semantics, key.grouping_mode, key.shortlist_policy
    )
}

fn normalize_timeframe_label(raw: &str) -> String {
    let s0 = raw.trim().to_ascii_lowercase();
    if s0.is_empty() {
        return String::new();
    }
    if let Some(sec_str) = s0.strip_suffix('s') {
        if let Ok(sec) = sec_str.parse::<i64>() {
            let mapped = match sec {
                60 => Some("1m"),
                120 => Some("2m"),
                180 => Some("3m"),
                300 => Some("5m"),
                900 => Some("15m"),
                1800 => Some("30m"),
                3600 => Some("1h"),
                7200 => Some("2h"),
                14_400 => Some("4h"),
                86_400 => Some("1d"),
                _ => None,
            };
            if let Some(lbl) = mapped {
                return lbl.to_string();
            }
        }
    }
    s0
}

fn normalize_ml_confidence_scope(scope: MlConfidenceScope) -> Result<MlConfidenceScope, String> {
    let timeframe = normalize_timeframe_label(&scope.timeframe);
    if timeframe.is_empty() {
        return Err("timeframe is required (e.g. '1m')".to_string());
    }
    let strategy_kind = scope.strategy_kind.trim().to_string();
    if strategy_kind.is_empty() {
        return Err("strategy_kind is required".to_string());
    }
    Ok(MlConfidenceScope {
        strategy_kind,
        timeframe,
    })
}

fn validate_training_rows(rows: &[DatasetRow]) -> Result<(), String> {
    let expected_width = FEATURE_NAMES.len();
    for (idx, row) in rows.iter().enumerate() {
        if row.x.len() != expected_width {
            return Err(format!(
                "row {idx} ({}) has {} features; expected {expected_width} ({})",
                row.group_id,
                row.x.len(),
                FEATURE_NAMES.join(", ")
            ));
        }
        if let Some((feature_index, value)) = row
            .x
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| !value.is_finite())
        {
            return Err(format!(
                "row {idx} ({}) has non-finite feature '{}' at index {feature_index}: {value}",
                row.group_id, FEATURE_NAMES[feature_index]
            ));
        }
        if !row.y.is_finite() {
            return Err(format!(
                "row {idx} ({}) has non-finite target value: {}",
                row.group_id, row.y
            ));
        }
    }
    Ok(())
}

fn extract_meta_scope(meta: &Value) -> Option<(String, String)> {
    let strat = meta
        .pointer("/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| meta.pointer("/res/strategy_kind").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim()
        .to_string();
    let tf_raw = meta
        .pointer("/req/timeframe")
        .and_then(|v| v.as_str())
        .or_else(|| meta.pointer("/res/timeframe").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim()
        .to_string();
    let tf = normalize_timeframe_label(&tf_raw);
    if strat.is_empty() || tf.is_empty() {
        return None;
    }
    Some((strat, tf))
}

pub fn extract_ml_confidence_group_features(group: &Value) -> Option<MlConfidenceGroupFeatures> {
    Some(MlConfidenceGroupFeatures {
        samples: group.get("samples").and_then(|v| v.as_u64())?,
        pass_rate: group.get("pass_rate").and_then(|v| v.as_f64())?,
        robust_score_p10: group.get("robust_score_p10").and_then(|v| v.as_f64())?,
        robust_score_median: group.get("robust_score_median").and_then(|v| v.as_f64())?,
        max_dd_p90: group.get("max_dd_p90").and_then(|v| v.as_f64())?,
        cross_score_median: group.get("cross_score_median").and_then(|v| v.as_f64()),
        meta_score: group.get("meta_score").and_then(|v| v.as_f64())?,
    })
}

fn extract_features_from_meta_group(g: &Value) -> Option<Vec<f64>> {
    extract_ml_confidence_group_features(g)?.to_feature_vec()
}

fn label_from_holdout(h: &Value, cfg: &MlConfidenceLabelConfig) -> Option<f64> {
    let datasets = h.get("datasets").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    if datasets < cfg.min_holdout_datasets {
        return None;
    }

    let pass_rate = h.get("pass_rate").and_then(|v| v.as_f64())?;
    let score_p10 = h.get("score_p10").and_then(|v| v.as_f64())?;
    let max_dd_p90 = h.get("max_dd_p90").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let mut ok = pass_rate >= cfg.min_holdout_pass_rate && score_p10 >= cfg.min_holdout_score_p10;
    if let Some(dd_max) = cfg.max_holdout_max_dd_p90 {
        ok = ok && max_dd_p90 <= dd_max;
    }
    Some(if ok { 1.0 } else { 0.0 })
}

fn label_from_holdout_row(row: &Value, cfg: &MlConfidenceLabelConfig) -> Option<(String, f64)> {
    let dataset = row
        .get("dataset")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    // NOTE: `min_holdout_score_p10` is interpreted as a threshold on the per-dataset median score.
    // This aligns with how the meta holdout aggregate is computed (`score_p10` is the p10 quantile
    // across datasets of each dataset's representative-median score).
    let score_median = row.get("score_median").and_then(|v| v.as_f64())?;
    let max_dd_worst = row
        .get("max_dd_worst")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    if !score_median.is_finite() || !max_dd_worst.is_finite() {
        return None;
    }

    let mut ok = score_median >= cfg.min_holdout_score_p10;
    if let Some(dd_max) = cfg.max_holdout_max_dd_p90 {
        ok = ok && max_dd_worst <= dd_max;
    }

    Some((dataset, if ok { 1.0 } else { 0.0 }))
}

#[derive(Default)]
struct PreparedHistoryCounters {
    skipped_kind: usize,
    skipped_scope: usize,
    skipped_missing: usize,
    skipped_incompatible: usize,
    gate_missing_holdout: usize,
    gate_insufficient_datasets: usize,
    dataset_missing_holdout_rows: usize,
    dataset_insufficient_datasets: usize,
    dataset_skipped_row: usize,
}

fn resolve_target_from_prepared_rows(
    prepared: &MlConfidencePreparedRows,
    cfg_target: MlConfidenceTarget,
    prefix_len: usize,
) -> MlConfidenceTargetResolved {
    match cfg_target {
        MlConfidenceTarget::HoldoutGate => MlConfidenceTargetResolved::HoldoutGate,
        MlConfidenceTarget::HoldoutDatasetPass => MlConfidenceTargetResolved::HoldoutDatasetPass,
        MlConfidenceTarget::Auto => {
            let has_dataset_rows = prepared
                .artifacts
                .iter()
                .take(prefix_len)
                .any(|artifact| !artifact.holdout_dataset_rows.is_empty());
            if has_dataset_rows {
                MlConfidenceTargetResolved::HoldoutDatasetPass
            } else {
                MlConfidenceTargetResolved::HoldoutGate
            }
        }
    }
}

pub fn prepare_ml_confidence_rows_from_meta_artifacts(
    scope: &MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: &MlConfidenceLabelConfig,
) -> MlConfidencePreparedRows {
    let scope_tf = normalize_timeframe_label(&scope.timeframe);
    let scope_strat = scope.strategy_kind.trim();

    let mut artifacts: Vec<MlConfidencePreparedArtifactRows> = Vec::new();
    let mut has_holdout_dataset_rows = false;
    let mut counters = PreparedHistoryCounters::default();
    let mut scoped: Vec<(&Value, Option<String>, MlConfidenceCompatibilityKey)> = Vec::new();

    for meta in meta_artifacts {
        let kind = meta.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if !is_meta_selection_kind(kind) {
            counters.skipped_kind = counters.skipped_kind.saturating_add(1);
            continue;
        }
        let Some((strat, tf)) = extract_meta_scope(meta) else {
            counters.skipped_missing = counters.skipped_missing.saturating_add(1);
            continue;
        };
        if strat != scope_strat || tf != scope_tf {
            counters.skipped_scope = counters.skipped_scope.saturating_add(1);
            continue;
        }
        scoped.push((
            meta,
            meta.get("ts")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            compatibility_key_for_meta_artifact(meta),
        ));
    }

    let active_compatibility_key = scoped
        .iter()
        .max_by(|left, right| left.1.as_deref().cmp(&right.1.as_deref()))
        .map(|(_, _, key)| key.clone());

    for (meta, ts, compatibility_key) in scoped {
        if active_compatibility_key
            .as_ref()
            .map(|active| active != &compatibility_key)
            .unwrap_or(false)
        {
            counters.skipped_incompatible = counters.skipped_incompatible.saturating_add(1);
            continue;
        }

        let groups = meta
            .pointer("/res/groups")
            .and_then(|v| v.as_array())
            .map(|xs| xs.as_slice())
            .unwrap_or(&[]);

        let group_id = ts.clone().unwrap_or_default();
        let cv_group_id = meta_artifact_cv_group_id(meta, ts.as_deref());
        let mut holdout_gate_rows: Vec<DatasetRow> = Vec::new();
        let mut holdout_dataset_rows: Vec<DatasetRow> = Vec::new();
        for g in groups {
            let Some(x) = extract_features_from_meta_group(g) else {
                continue;
            };

            let Some(h) = g.get("holdout") else {
                counters.gate_missing_holdout = counters.gate_missing_holdout.saturating_add(1);
                let Some(holdout_rows) = g.get("holdout_rows").and_then(|v| v.as_array()) else {
                    counters.dataset_missing_holdout_rows =
                        counters.dataset_missing_holdout_rows.saturating_add(1);
                    continue;
                };
                if holdout_rows.len() < cfg.min_holdout_datasets {
                    counters.dataset_insufficient_datasets =
                        counters.dataset_insufficient_datasets.saturating_add(1);
                    continue;
                }
                let mut pushed = 0usize;
                for r in holdout_rows {
                    let Some((ds, y)) = label_from_holdout_row(r, cfg) else {
                        counters.dataset_skipped_row =
                            counters.dataset_skipped_row.saturating_add(1);
                        continue;
                    };
                    holdout_dataset_rows.push(DatasetRow {
                        group_id: ds,
                        cv_group_id: cv_group_id.clone(),
                        x: x.clone(),
                        y,
                    });
                    pushed = pushed.saturating_add(1);
                }
                if pushed > 0 {
                    has_holdout_dataset_rows = true;
                }
                continue;
            };

            if let Some(y) = label_from_holdout(h, cfg) {
                holdout_gate_rows.push(DatasetRow {
                    group_id: group_id.clone(),
                    cv_group_id: cv_group_id.clone(),
                    x: x.clone(),
                    y,
                });
            } else {
                counters.gate_insufficient_datasets =
                    counters.gate_insufficient_datasets.saturating_add(1);
            }

            let Some(holdout_rows) = g.get("holdout_rows").and_then(|v| v.as_array()) else {
                counters.dataset_missing_holdout_rows =
                    counters.dataset_missing_holdout_rows.saturating_add(1);
                continue;
            };
            if holdout_rows.len() < cfg.min_holdout_datasets {
                counters.dataset_insufficient_datasets =
                    counters.dataset_insufficient_datasets.saturating_add(1);
                continue;
            }
            let mut pushed = 0usize;
            for r in holdout_rows {
                let Some((ds, y)) = label_from_holdout_row(r, cfg) else {
                    counters.dataset_skipped_row = counters.dataset_skipped_row.saturating_add(1);
                    continue;
                };
                holdout_dataset_rows.push(DatasetRow {
                    group_id: ds,
                    cv_group_id: cv_group_id.clone(),
                    x: x.clone(),
                    y,
                });
                pushed = pushed.saturating_add(1);
            }
            if pushed > 0 {
                has_holdout_dataset_rows = true;
            }
        }

        artifacts.push(MlConfidencePreparedArtifactRows {
            ts,
            compatibility_key,
            holdout_gate_rows,
            holdout_dataset_rows,
        });
    }

    let mut warnings: Vec<String> = Vec::new();
    if artifacts.is_empty() {
        warnings.push("No matching meta artifacts found for this scope.".to_string());
    }
    if counters.skipped_kind > 0 {
        warnings.push(format!(
            "Skipped {} non-meta JSON file(s).",
            counters.skipped_kind
        ));
    }
    if counters.skipped_scope > 0 {
        warnings.push(format!(
            "Skipped {} meta artifact(s) outside scope.",
            counters.skipped_scope
        ));
    }
    if counters.skipped_missing > 0 {
        warnings.push(format!(
            "Skipped {} meta artifact(s) missing strategy/timeframe.",
            counters.skipped_missing
        ));
    }
    if counters.skipped_incompatible > 0 {
        let key_text = active_compatibility_key
            .as_ref()
            .map(compatibility_key_label)
            .unwrap_or_else(|| "none".to_string());
        warnings.push(format!(
            "Skipped {} meta artifact(s) with incompatible ML-confidence history contracts (active: {}).",
            counters.skipped_incompatible, key_text
        ));
    }

    MlConfidencePreparedRows {
        scope: scope.clone(),
        compatibility_key: active_compatibility_key,
        artifacts,
        has_holdout_dataset_rows,
        skipped_incompatible_artifacts: counters.skipped_incompatible,
        warnings,
    }
}

fn build_dataset_from_prepared_rows(
    prepared: &MlConfidencePreparedRows,
    cfg: &MlConfidenceLabelConfig,
    target: MlConfidenceTargetResolved,
    prefix_len: usize,
) -> (Vec<DatasetRow>, DatasetBuildSummary, Vec<String>) {
    let mut rows: Vec<DatasetRow> = Vec::new();
    let mut used_meta = 0usize;
    let mut newest_meta_ts: Option<String> = None;
    let mut missing_holdout = 0usize;
    let mut insufficient_datasets = 0usize;
    for artifact in prepared.artifacts.iter().take(prefix_len) {
        used_meta = used_meta.saturating_add(1);
        if let Some(ts) = artifact.ts.as_ref() {
            let is_newest = newest_meta_ts
                .as_ref()
                .map(|cur| ts.as_str() > cur.as_str())
                .unwrap_or(true);
            if is_newest {
                newest_meta_ts = Some(ts.clone());
            }
        }

        let source_rows = match target {
            MlConfidenceTargetResolved::HoldoutGate => &artifact.holdout_gate_rows,
            MlConfidenceTargetResolved::HoldoutDatasetPass => &artifact.holdout_dataset_rows,
        };
        if source_rows.is_empty() {
            match target {
                MlConfidenceTargetResolved::HoldoutGate => {
                    missing_holdout = missing_holdout.saturating_add(1);
                }
                MlConfidenceTargetResolved::HoldoutDatasetPass => {
                    insufficient_datasets = insufficient_datasets.saturating_add(1);
                }
            }
            continue;
        }
        rows.extend(source_rows.iter().cloned());
    }

    let mut warnings = prepared.warnings.clone();
    if rows.is_empty() {
        let msg = match target {
            MlConfidenceTargetResolved::HoldoutGate => {
                "No labeled rows found (need meta artifacts with holdout aggregates)."
            }
            MlConfidenceTargetResolved::HoldoutDatasetPass => {
                "No labeled rows found (need meta artifacts with holdout_rows enabled)."
            }
        };
        warnings.push(msg.to_string());
    }
    if missing_holdout > 0 {
        warnings.push(format!(
            "Skipped {missing_holdout} meta artifact(s) without usable holdout aggregates."
        ));
    }
    if insufficient_datasets > 0 {
        warnings.push(format!(
            "Skipped {insufficient_datasets} meta artifact(s) with insufficient holdout datasets (<{}).",
            cfg.min_holdout_datasets
        ));
    }
    (
        rows,
        DatasetBuildSummary {
            used_meta,
            newest_meta_ts,
            target,
            compatibility_key: prepared.compatibility_key.clone(),
        },
        warnings,
    )
}

fn sigmoid(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let ez = z.exp();
        ez / (1.0 + ez)
    }
}

fn logit(p: f64) -> f64 {
    let pp = p.clamp(1e-15, 1.0 - 1e-15);
    (pp / (1.0 - pp)).ln()
}

fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

#[derive(Clone, Debug)]
struct StandardScaler {
    mean: Vec<f64>,
    std: Vec<f64>,
}

impl StandardScaler {
    fn fit(xs: &[Vec<f64>]) -> Self {
        let n = xs.len().max(1) as f64;
        let d = xs.first().map(|v| v.len()).unwrap_or(0);
        let mut mean = vec![0.0; d];
        for x in xs {
            for (j, v) in x.iter().enumerate() {
                mean[j] += *v;
            }
        }
        for m in &mut mean {
            *m /= n;
        }

        let mut var = vec![0.0; d];
        for x in xs {
            for (j, v) in x.iter().enumerate() {
                let dv = *v - mean[j];
                var[j] += dv * dv;
            }
        }
        for v in &mut var {
            *v /= n;
        }
        let mut std: Vec<f64> = var.iter().map(|v| v.sqrt()).collect();
        for s in &mut std {
            if !s.is_finite() || *s < 1e-12 {
                *s = 1.0;
            }
        }
        Self { mean, std }
    }

    fn transform_row_in_place(&self, x: &mut [f64]) {
        for j in 0..x.len() {
            x[j] = (x[j] - self.mean[j]) / self.std[j];
        }
    }
}

fn fit_logistic_regression(
    xs: &[Vec<f64>],
    ys: &[f64],
    l2: f64,
    lr0: f64,
    max_iter: usize,
) -> (Vec<f64>, f64, f64) {
    let n = xs.len().max(1) as f64;
    let d = xs.first().map(|v| v.len()).unwrap_or(0);
    let mut w = vec![0.0; d];
    let mut b = 0.0;
    let mut last_loss = f64::INFINITY;

    for it in 0..max_iter.max(1) {
        let lr = lr0 / (1.0 + (it as f64).sqrt() * 0.25);
        let mut grad_w = vec![0.0; d];
        let mut grad_b = 0.0;

        let mut loss = 0.0;
        for (x, &y) in xs.iter().zip(ys.iter()) {
            let mut z = b;
            for j in 0..d {
                z += w[j] * x[j];
            }
            let p = sigmoid(z).clamp(1e-15, 1.0 - 1e-15);
            let diff = p - y;
            for j in 0..d {
                grad_w[j] += diff * x[j];
            }
            grad_b += diff;
            loss += -(y * p.ln() + (1.0 - y) * (1.0 - p).ln());
        }

        loss /= n;
        let mut l2_term = 0.0;
        for j in 0..d {
            l2_term += w[j] * w[j];
        }
        loss += 0.5 * l2 * l2_term;

        for j in 0..d {
            grad_w[j] = grad_w[j] / n + l2 * w[j];
        }
        grad_b /= n;

        for j in 0..d {
            w[j] -= lr * grad_w[j];
        }
        b -= lr * grad_b;

        if (last_loss - loss).abs() < 1e-9 {
            last_loss = loss;
            break;
        }
        last_loss = loss;
    }

    (w, b, last_loss)
}

fn predict_probs_raw(weights: &[f64], bias: f64, xs: &[Vec<f64>]) -> Vec<f64> {
    let d = weights.len();
    let mut ps = Vec::with_capacity(xs.len());
    for x in xs {
        let mut z = bias;
        for j in 0..d {
            z += weights[j] * x[j];
        }
        ps.push(sigmoid(z));
    }
    ps
}

fn select_threshold_candidates(sorted_values: &[f64], max_bins: usize) -> Vec<f64> {
    if sorted_values.len() < 2 {
        return Vec::new();
    }
    let mut uniq: Vec<f64> = Vec::with_capacity(sorted_values.len());
    for &v in sorted_values {
        if uniq.last().copied() != Some(v) {
            uniq.push(v);
        }
    }
    if uniq.len() < 2 {
        return Vec::new();
    }
    let max_splits = max_bins.clamp(4, 256).min(uniq.len() - 1);
    let mut out: Vec<f64> = Vec::new();
    if uniq.len() - 1 <= max_splits {
        for i in 1..uniq.len() {
            out.push(0.5 * (uniq[i - 1] + uniq[i]));
        }
        return out;
    }
    for c in 1..=max_splits {
        let idx = (c * (uniq.len() - 1)) / (max_splits + 1);
        if idx == 0 || idx >= uniq.len() {
            continue;
        }
        let t = 0.5 * (uniq[idx - 1] + uniq[idx]);
        if out.last().copied() != Some(t) {
            out.push(t);
        }
    }
    out
}

fn fit_best_stump(
    xs: &[Vec<f64>],
    residuals: &[f64],
    max_bins: usize,
    min_leaf: usize,
) -> Option<MlConfidenceTreeStump> {
    if xs.is_empty() || xs.len() != residuals.len() {
        return None;
    }
    let n = xs.len();
    let d = xs[0].len();
    if d == 0 {
        return None;
    }
    let min_leaf = min_leaf.max(1).min((n / 2).max(1));
    let mut best_sse = f64::INFINITY;
    let mut best: Option<MlConfidenceTreeStump> = None;

    for j in 0..d {
        let mut col: Vec<f64> = xs.iter().map(|x| x[j]).collect();
        col.sort_by(|a, b| a.total_cmp(b));
        let thresholds = select_threshold_candidates(&col, max_bins);
        if thresholds.is_empty() {
            continue;
        }
        for &th in &thresholds {
            let mut l_n = 0usize;
            let mut r_n = 0usize;
            let mut l_sum = 0.0;
            let mut r_sum = 0.0;
            for i in 0..n {
                if xs[i][j] <= th {
                    l_n += 1;
                    l_sum += residuals[i];
                } else {
                    r_n += 1;
                    r_sum += residuals[i];
                }
            }
            if l_n < min_leaf || r_n < min_leaf {
                continue;
            }
            let l_mean = l_sum / (l_n as f64);
            let r_mean = r_sum / (r_n as f64);
            let mut sse = 0.0;
            for i in 0..n {
                let pred = if xs[i][j] <= th { l_mean } else { r_mean };
                let e = residuals[i] - pred;
                sse += e * e;
            }
            if !sse.is_finite() {
                continue;
            }
            if sse < best_sse {
                best_sse = sse;
                best = Some(MlConfidenceTreeStump {
                    feature_index: j,
                    threshold: th,
                    left_value: l_mean.clamp(-2.0, 2.0),
                    right_value: r_mean.clamp(-2.0, 2.0),
                });
            }
        }
    }

    best
}

fn predict_prob_gbdt_row(gbdt: &MlConfidenceGbdtModel, x: &[f64]) -> Option<f64> {
    let mut z = gbdt.base_logit;
    for t in &gbdt.trees {
        let v = x.get(t.feature_index).copied()?;
        let leaf = if v <= t.threshold {
            t.left_value
        } else {
            t.right_value
        };
        z += gbdt.learning_rate * leaf;
    }
    Some(sigmoid(z))
}

fn predict_probs_gbdt(gbdt: &MlConfidenceGbdtModel, xs: &[Vec<f64>]) -> Vec<f64> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(predict_prob_gbdt_row(gbdt, x).unwrap_or(0.5));
    }
    out
}

fn fit_gbdt_stumps(
    xs: &[Vec<f64>],
    ys: &[f64],
    rounds: usize,
    learning_rate: f64,
    max_bins: usize,
    min_leaf: usize,
) -> Option<MlConfidenceGbdtModel> {
    if xs.is_empty() || xs.len() != ys.len() {
        return None;
    }
    let pos = ys.iter().filter(|&&y| y > 0.5).count();
    let neg = ys.len().saturating_sub(pos);
    if pos == 0 || neg == 0 {
        return None;
    }
    let base_p = (pos as f64) / (ys.len() as f64);
    let base_logit = logit(base_p);
    let mut margin = vec![base_logit; ys.len()];
    let rounds = rounds.clamp(1, 512);
    let learning_rate = learning_rate.clamp(0.005, 0.5);
    let mut trees: Vec<MlConfidenceTreeStump> = Vec::with_capacity(rounds);

    for _ in 0..rounds {
        let mut residuals: Vec<f64> = Vec::with_capacity(ys.len());
        for i in 0..ys.len() {
            let p = sigmoid(margin[i]);
            residuals.push((ys[i] - p).clamp(-1.0, 1.0));
        }
        let Some(stump) = fit_best_stump(xs, &residuals, max_bins, min_leaf) else {
            break;
        };

        let mut mean_abs_update = 0.0;
        for (i, x) in xs.iter().enumerate() {
            let leaf = if x[stump.feature_index] <= stump.threshold {
                stump.left_value
            } else {
                stump.right_value
            };
            let delta = learning_rate * leaf;
            margin[i] += delta;
            mean_abs_update += delta.abs();
        }
        mean_abs_update /= ys.len().max(1) as f64;
        trees.push(stump);
        if mean_abs_update < 1e-6 {
            break;
        }
    }

    if trees.is_empty() {
        return None;
    }
    Some(MlConfidenceGbdtModel {
        base_logit,
        learning_rate,
        trees,
    })
}

#[derive(Clone, Debug)]
struct TrainedCandidate {
    model_kind: MlConfidenceModelKind,
    mean: Vec<f64>,
    std: Vec<f64>,
    weights: Vec<f64>,
    bias: f64,
    gbdt: Option<MlConfidenceGbdtModel>,
    train_ps: Vec<f64>,
    cv_ps: Vec<f64>,
    cv_ys: Vec<f64>,
    cv_warnings: Vec<String>,
}

fn train_logistic_candidate(
    rows: &[DatasetRow],
    cfg: &MlConfidenceTrainConfig,
) -> TrainedCandidate {
    let xs_raw: Vec<Vec<f64>> = rows.iter().map(|r| r.x.clone()).collect();
    let ys: Vec<f64> = rows.iter().map(|r| r.y).collect();

    let mut cv_ps: Vec<f64> = Vec::new();
    let mut cv_ys: Vec<f64> = Vec::new();
    let mut cv_warnings: Vec<String> = Vec::new();

    let uniq_groups = {
        use std::collections::HashSet;
        let mut s = HashSet::new();
        for r in rows {
            s.insert(r.cv_group_id.clone());
        }
        s.len()
    };
    let k = cfg.k_folds.clamp(2, 10).min(uniq_groups.max(2));
    if uniq_groups >= 2 && k >= 2 {
        for fold in 0..k {
            let mut train_x: Vec<Vec<f64>> = Vec::new();
            let mut train_y: Vec<f64> = Vec::new();
            let mut test_x: Vec<Vec<f64>> = Vec::new();
            let mut test_y: Vec<f64> = Vec::new();

            for (r, x) in rows.iter().zip(xs_raw.iter()) {
                let f = (fnv1a64(&r.cv_group_id) % (k as u64)) as usize;
                if f == fold {
                    test_x.push(x.clone());
                    test_y.push(r.y);
                } else {
                    train_x.push(x.clone());
                    train_y.push(r.y);
                }
            }

            if train_x.is_empty() || test_x.is_empty() {
                continue;
            }

            let scaler = StandardScaler::fit(&train_x);
            let mut train_xs = train_x;
            for x in &mut train_xs {
                scaler.transform_row_in_place(x);
            }
            let mut test_xs = test_x;
            for x in &mut test_xs {
                scaler.transform_row_in_place(x);
            }

            let (w, b, _) = fit_logistic_regression(
                &train_xs,
                &train_y,
                cfg.l2,
                cfg.learning_rate,
                cfg.max_iter,
            );
            let ps = predict_probs_raw(&w, b, &test_xs);
            cv_ps.extend(ps);
            cv_ys.extend(test_y);
        }
        if cv_ps.is_empty() {
            cv_warnings.push("CV produced no predictions.".to_string());
        }
    } else {
        cv_warnings.push("CV skipped (need at least 2 unique meta runs).".to_string());
    }

    let scaler = StandardScaler::fit(&xs_raw);
    let mut xs_all = xs_raw;
    for x in &mut xs_all {
        scaler.transform_row_in_place(x);
    }
    let (weights, bias, _) =
        fit_logistic_regression(&xs_all, &ys, cfg.l2, cfg.learning_rate, cfg.max_iter);
    let train_ps = predict_probs_raw(&weights, bias, &xs_all);

    TrainedCandidate {
        model_kind: MlConfidenceModelKind::Logistic,
        mean: scaler.mean,
        std: scaler.std,
        weights,
        bias,
        gbdt: None,
        train_ps,
        cv_ps,
        cv_ys,
        cv_warnings,
    }
}

fn train_gbdt_candidate(
    rows: &[DatasetRow],
    cfg: &MlConfidenceTrainConfig,
) -> Option<TrainedCandidate> {
    let xs_raw: Vec<Vec<f64>> = rows.iter().map(|r| r.x.clone()).collect();
    let ys: Vec<f64> = rows.iter().map(|r| r.y).collect();
    let mut cv_ps: Vec<f64> = Vec::new();
    let mut cv_ys: Vec<f64> = Vec::new();
    let mut cv_warnings: Vec<String> = Vec::new();

    let uniq_groups = {
        use std::collections::HashSet;
        let mut s = HashSet::new();
        for r in rows {
            s.insert(r.cv_group_id.clone());
        }
        s.len()
    };
    let k = cfg.k_folds.clamp(2, 10).min(uniq_groups.max(2));
    if uniq_groups >= 2 && k >= 2 {
        for fold in 0..k {
            let mut train_x: Vec<Vec<f64>> = Vec::new();
            let mut train_y: Vec<f64> = Vec::new();
            let mut test_x: Vec<Vec<f64>> = Vec::new();
            let mut test_y: Vec<f64> = Vec::new();

            for (r, x) in rows.iter().zip(xs_raw.iter()) {
                let f = (fnv1a64(&r.cv_group_id) % (k as u64)) as usize;
                if f == fold {
                    test_x.push(x.clone());
                    test_y.push(r.y);
                } else {
                    train_x.push(x.clone());
                    train_y.push(r.y);
                }
            }
            if train_x.is_empty() || test_x.is_empty() {
                continue;
            }
            let Some(gbdt) = fit_gbdt_stumps(
                &train_x,
                &train_y,
                cfg.gbdt_rounds,
                cfg.gbdt_learning_rate,
                cfg.gbdt_max_bins,
                cfg.gbdt_min_leaf,
            ) else {
                continue;
            };
            let ps = predict_probs_gbdt(&gbdt, &test_x);
            cv_ps.extend(ps);
            cv_ys.extend(test_y);
        }
        if cv_ps.is_empty() {
            cv_warnings.push(
                "GBDT CV produced no predictions (likely fold class imbalance); will rely on train metrics."
                    .to_string(),
            );
        }
    } else {
        cv_warnings.push("CV skipped (need at least 2 unique meta runs).".to_string());
    }

    let gbdt = fit_gbdt_stumps(
        &xs_raw,
        &ys,
        cfg.gbdt_rounds,
        cfg.gbdt_learning_rate,
        cfg.gbdt_max_bins,
        cfg.gbdt_min_leaf,
    )?;
    let train_ps = predict_probs_gbdt(&gbdt, &xs_raw);

    Some(TrainedCandidate {
        model_kind: MlConfidenceModelKind::GbdtStump,
        mean: Vec::new(),
        std: Vec::new(),
        weights: Vec::new(),
        bias: 0.0,
        gbdt: Some(gbdt),
        train_ps,
        cv_ps,
        cv_ys,
        cv_warnings,
    })
}

fn brier_score(ps: &[f64], ys: &[f64]) -> f64 {
    if ps.is_empty() || ps.len() != ys.len() {
        return 0.0;
    }
    let mut acc = 0.0;
    for (&p, &y) in ps.iter().zip(ys.iter()) {
        let d = p - y;
        acc += d * d;
    }
    acc / (ps.len() as f64)
}

fn baseline_brier(positives: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let p = (positives as f64) / (total as f64);
    p * (1.0 - p)
}

fn fit_platt_scaling_from_probs(ps: &[f64], ys: &[f64]) -> Option<MlConfidenceCalibration> {
    if ps.is_empty() || ps.len() != ys.len() {
        return None;
    }
    let pos = ys.iter().filter(|&&y| y > 0.5).count();
    let neg = ys.len().saturating_sub(pos);
    if pos == 0 || neg == 0 {
        return None;
    }
    let mut xs: Vec<Vec<f64>> = Vec::with_capacity(ps.len());
    for &p in ps {
        xs.push(vec![logit(p)]);
    }
    let ys2: Vec<f64> = ys.to_vec();

    // Lightweight 1D logistic regression (Platt scaling). Very small l2 to keep it stable.
    let (w, b, _) = fit_logistic_regression(&xs, &ys2, 1e-6, 0.2, 200);
    let a = w.first().copied().unwrap_or(1.0);
    if !a.is_finite() || !b.is_finite() || a <= 0.0 {
        return None;
    }
    Some(MlConfidenceCalibration {
        method: "platt_logit".to_string(),
        a,
        b,
    })
}

fn apply_calibration(p_raw: f64, cal: &MlConfidenceCalibration) -> f64 {
    if cal.method != "platt_logit" {
        return p_raw.clamp(0.0, 1.0);
    }
    let z = cal.a * logit(p_raw) + cal.b;
    sigmoid(z)
}

fn auc_roc(ps: &[f64], ys: &[f64]) -> Option<f64> {
    if ps.is_empty() || ps.len() != ys.len() {
        return None;
    }
    let mut pairs: Vec<(f64, f64)> = ps.iter().copied().zip(ys.iter().copied()).collect();
    let pos = ys.iter().filter(|&&y| y > 0.5).count();
    let neg = ys.len().saturating_sub(pos);
    if pos == 0 || neg == 0 {
        return None;
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Compute average ranks for ties, then use rank-sum AUC.
    let mut rank_sum_pos = 0.0;
    let mut i = 0usize;
    let n = pairs.len();
    while i < n {
        let p0 = pairs[i].0;
        let mut j = i + 1;
        while j < n && pairs[j].0 == p0 {
            j += 1;
        }
        // ranks are 1-based
        let rank_lo = (i + 1) as f64;
        let rank_hi = j as f64;
        let rank_avg = (rank_lo + rank_hi) / 2.0;
        let mut pos_in_tie = 0usize;
        for k in i..j {
            if pairs[k].1 > 0.5 {
                pos_in_tie += 1;
            }
        }
        rank_sum_pos += rank_avg * (pos_in_tie as f64);
        i = j;
    }

    let pos_f = pos as f64;
    let neg_f = neg as f64;
    let auc = (rank_sum_pos - pos_f * (pos_f + 1.0) / 2.0) / (pos_f * neg_f);
    Some(auc)
}

fn calibration_bins(ps: &[f64], ys: &[f64], bins: usize) -> Vec<MlConfidenceCalibrationBin> {
    if ps.is_empty() || ps.len() != ys.len() || bins == 0 {
        return Vec::new();
    }
    let mut count = vec![0usize; bins];
    let mut sum_p = vec![0.0; bins];
    let mut sum_y = vec![0.0; bins];
    for (&p, &y) in ps.iter().zip(ys.iter()) {
        let idx = ((p.clamp(0.0, 1.0) * (bins as f64)).floor() as usize).min(bins - 1);
        count[idx] += 1;
        sum_p[idx] += p;
        sum_y[idx] += y;
    }
    let mut out = Vec::new();
    for i in 0..bins {
        if count[i] == 0 {
            continue;
        }
        let n = count[i] as f64;
        out.push(MlConfidenceCalibrationBin {
            bin: i,
            count: count[i],
            p_mean: sum_p[i] / n,
            y_mean: sum_y[i] / n,
        });
    }
    out
}

fn train_ml_confidence_model_from_rows_inner(
    scope: MlConfidenceScope,
    rows: &[DatasetRow],
    cfg: MlConfidenceTrainConfig,
    build: DatasetBuildSummary,
    mut warnings: Vec<String>,
) -> Result<(MlConfidenceModel, MlConfidenceTrainMetrics), String> {
    validate_training_rows(rows)?;

    let target_used = match build.target {
        MlConfidenceTargetResolved::HoldoutGate => MlConfidenceTarget::HoldoutGate,
        MlConfidenceTargetResolved::HoldoutDatasetPass => MlConfidenceTarget::HoldoutDatasetPass,
    };

    let n = rows.len();
    if n < 25 {
        warnings.push("Insufficient labeled rows (need at least 25).".to_string());
        return Err(warnings.join(" "));
    }
    let positives = rows.iter().filter(|r| r.y > 0.5).count();
    let negatives = n.saturating_sub(positives);
    if positives < 5 || negatives < 5 {
        warnings.push(format!(
            "Insufficient class balance (pos={positives}, neg={negatives})."
        ));
        return Err(warnings.join(" "));
    }

    let ys: Vec<f64> = rows.iter().map(|r| r.y).collect();
    let base_brier = baseline_brier(positives, n);
    let unique_groups = {
        use std::collections::HashSet;
        let mut s = HashSet::new();
        for r in rows {
            s.insert(r.cv_group_id.clone());
        }
        s.len()
    };

    let resolved_family = match cfg.model_family.clone() {
        MlConfidenceModelFamily::Auto => {
            if n >= 50 && unique_groups >= 2 {
                MlConfidenceModelFamily::GbdtStump
            } else {
                MlConfidenceModelFamily::Logistic
            }
        }
        x => x,
    };

    let mut family_warnings: Vec<String> = Vec::new();
    let trained = match resolved_family {
        MlConfidenceModelFamily::Logistic => train_logistic_candidate(&rows, &cfg),
        MlConfidenceModelFamily::GbdtStump => match train_gbdt_candidate(&rows, &cfg) {
            Some(c) => c,
            None => {
                family_warnings.push(
                    "GBDT training failed (insufficient split diversity or class balance); falling back to logistic."
                        .to_string(),
                );
                train_logistic_candidate(&rows, &cfg)
            }
        },
        MlConfidenceModelFamily::Auto => unreachable!(),
    };

    let mut cv_auc: Option<f64> = None;
    let mut cv_brier: Option<f64> = None;
    let mut cv_brier_cal: Option<f64> = None;
    if !trained.cv_ps.is_empty() && trained.cv_ps.len() == trained.cv_ys.len() {
        cv_brier = Some(brier_score(&trained.cv_ps, &trained.cv_ys));
    }
    if let Some(auc) = auc_roc(&trained.cv_ps, &trained.cv_ys) {
        cv_auc = Some(auc);
    } else if !trained.cv_ps.is_empty() {
        family_warnings.push("CV AUC unavailable (need both classes in predictions).".to_string());
    }

    let calibration = if !trained.cv_ps.is_empty() && trained.cv_ps.len() == trained.cv_ys.len() {
        if let Some(cal) = fit_platt_scaling_from_probs(&trained.cv_ps, &trained.cv_ys) {
            let ps_cal: Vec<f64> = trained
                .cv_ps
                .iter()
                .copied()
                .map(|p| apply_calibration(p, &cal))
                .collect();
            let b_cal = brier_score(&ps_cal, &trained.cv_ys);
            cv_brier_cal = Some(b_cal);

            if let Some(b_raw) = cv_brier {
                if b_cal <= b_raw {
                    Some(cal)
                } else {
                    cv_brier_cal = None;
                    family_warnings
                        .push("Calibration skipped (did not improve CV Brier score).".to_string());
                    None
                }
            } else {
                Some(cal)
            }
        } else {
            None
        }
    } else {
        None
    };

    warnings.extend(family_warnings);
    warnings.extend(trained.cv_warnings.clone());

    let train_auc = auc_roc(&trained.train_ps, &ys).unwrap_or(0.0);
    let train_brier = brier_score(&trained.train_ps, &ys);
    let train_brier_cal = calibration.as_ref().map(|cal| {
        let ps_cal: Vec<f64> = trained
            .train_ps
            .iter()
            .copied()
            .map(|p| apply_calibration(p, cal))
            .collect();
        brier_score(&ps_cal, &ys)
    });

    let calib_bins = if !trained.cv_ps.is_empty() && trained.cv_ps.len() == trained.cv_ys.len() {
        if let Some(cal) = calibration.as_ref() {
            let ps_cal: Vec<f64> = trained
                .cv_ps
                .iter()
                .copied()
                .map(|p| apply_calibration(p, cal))
                .collect();
            calibration_bins(&ps_cal, &trained.cv_ys, 10)
        } else {
            calibration_bins(&trained.cv_ps, &trained.cv_ys, 10)
        }
    } else if let Some(cal) = calibration.as_ref() {
        let ps_cal: Vec<f64> = trained
            .train_ps
            .iter()
            .copied()
            .map(|p| apply_calibration(p, cal))
            .collect();
        calibration_bins(&ps_cal, &ys, 10)
    } else {
        calibration_bins(&trained.train_ps, &ys, 10)
    };

    let eval_auc = cv_auc.unwrap_or(train_auc);
    let eval_brier = cv_brier_cal.or(cv_brier).unwrap_or(train_brier);
    if cv_auc.is_none() {
        warnings.push(
            "Evaluation evidence used train AUC because cross-validation AUC was unavailable."
                .to_string(),
        );
    }
    if cv_brier_cal.is_none() && cv_brier.is_none() {
        warnings.push(
            "Evaluation evidence used train Brier and calibration bins because cross-validation predictions were unavailable."
                .to_string(),
        );
    } else if cv_brier_cal.is_none() && calibration.is_some() {
        warnings.push(
            "Calibration was retained, but calibrated cross-validation Brier was unavailable; train calibration evidence remains informational only."
                .to_string(),
        );
    }
    let mut healthy = true;
    if eval_auc < 0.55 {
        healthy = false;
        warnings
            .push("Model health: AUC is low; treat ML confidence as untrustworthy.".to_string());
    }
    if eval_brier > base_brier {
        healthy = false;
        warnings.push(
            "Model health: Brier score worse than baseline; treat ML confidence as untrustworthy."
                .to_string(),
        );
    }

    let model = MlConfidenceModel {
        version: 4,
        scope: scope.clone(),
        feature_names: FEATURE_NAMES.iter().map(|s| s.to_string()).collect(),
        mean: trained.mean.clone(),
        std: trained.std.clone(),
        weights: trained.weights.clone(),
        bias: trained.bias,
        model_kind: trained.model_kind.clone(),
        gbdt: trained.gbdt.clone(),
        label: cfg.label,
        calibration: calibration.clone(),
        meta: Some(MlConfidenceModelMeta {
            trained_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
            trained_rows: n,
            trained_positives: positives,
            trained_negatives: negatives,
            target: target_used.clone(),
            model_kind: trained.model_kind.clone(),
            compatibility_key: build.compatibility_key.clone(),
            meta_artifacts_used: Some(build.used_meta),
            newest_meta_ts_used: build.newest_meta_ts.clone(),
            cv_auc,
            cv_brier,
            cv_brier_calibrated: cv_brier_cal,
            baseline_brier: base_brier,
            healthy,
            warnings: warnings.clone(),
        }),
    };

    let metrics = MlConfidenceTrainMetrics {
        rows: n,
        positives,
        negatives,
        target: target_used,
        model_kind: trained.model_kind,
        train_auc,
        train_brier,
        baseline_brier: base_brier,
        train_brier_calibrated: train_brier_cal,
        cv_auc,
        cv_brier,
        cv_brier_calibrated: cv_brier_cal,
        healthy,
        meta_artifacts_used: Some(build.used_meta),
        newest_meta_ts_used: build.newest_meta_ts,
        calibration: calib_bins,
        warnings,
    };

    Ok((model, metrics))
}

pub fn train_ml_confidence_model_from_rows(
    scope: MlConfidenceScope,
    rows: &[MlConfidenceDatasetRow],
    cfg: MlConfidenceTrainConfig,
    target_used: MlConfidenceTarget,
    meta_artifacts_used: usize,
    newest_meta_ts_used: Option<String>,
    warnings: Vec<String>,
) -> Result<(MlConfidenceModel, MlConfidenceTrainMetrics), String> {
    let scope = normalize_ml_confidence_scope(scope)?;

    let build = DatasetBuildSummary {
        used_meta: meta_artifacts_used,
        newest_meta_ts: newest_meta_ts_used,
        target: match target_used {
            MlConfidenceTarget::HoldoutGate => MlConfidenceTargetResolved::HoldoutGate,
            MlConfidenceTarget::HoldoutDatasetPass | MlConfidenceTarget::Auto => {
                MlConfidenceTargetResolved::HoldoutDatasetPass
            }
        },
        compatibility_key: None,
    };
    train_ml_confidence_model_from_rows_inner(scope, rows, cfg, build, warnings)
}

pub fn train_ml_confidence_model_from_prepared_rows(
    scope: MlConfidenceScope,
    prepared: &MlConfidencePreparedRows,
    cfg: MlConfidenceTrainConfig,
    prefix_len: Option<usize>,
) -> Result<(MlConfidenceModel, MlConfidenceTrainMetrics), String> {
    let scope = normalize_ml_confidence_scope(scope)?;
    let prepared_scope = normalize_ml_confidence_scope(prepared.scope.clone())?;
    if scope.strategy_kind != prepared_scope.strategy_kind
        || scope.timeframe != prepared_scope.timeframe
    {
        return Err(format!(
            "prepared rows scope mismatch: requested {}/{} but prepared rows are {}/{}",
            scope.strategy_kind,
            scope.timeframe,
            prepared_scope.strategy_kind,
            prepared_scope.timeframe
        ));
    }
    let effective_prefix = prefix_len
        .unwrap_or(prepared.artifacts.len())
        .min(prepared.artifacts.len());
    let target = resolve_target_from_prepared_rows(prepared, cfg.target.clone(), effective_prefix);
    let (rows, build, warnings) =
        build_dataset_from_prepared_rows(prepared, &cfg.label, target, effective_prefix);
    train_ml_confidence_model_from_rows_inner(scope, &rows, cfg, build, warnings)
}

pub fn train_ml_confidence_model_from_meta_artifacts(
    scope: MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: MlConfidenceTrainConfig,
) -> Result<(MlConfidenceModel, MlConfidenceTrainMetrics), String> {
    let scope = normalize_ml_confidence_scope(scope)?;
    let prepared =
        prepare_ml_confidence_rows_from_meta_artifacts(&scope, meta_artifacts, &cfg.label);
    train_ml_confidence_model_from_prepared_rows(scope, &prepared, cfg, None)
}

pub fn predict_ml_confidence_for_features(
    model: &MlConfidenceModel,
    features: &MlConfidenceGroupFeatures,
) -> Option<f64> {
    let x0 = features.to_feature_vec()?;
    let p_raw = match model.model_kind {
        MlConfidenceModelKind::Logistic => {
            if model.weights.len() != model.mean.len()
                || model.weights.len() != model.std.len()
                || model.weights.is_empty()
                || x0.len() != model.weights.len()
            {
                return None;
            }
            let mut z = model.bias;
            for j in 0..x0.len() {
                let x = (x0[j] - model.mean[j]) / model.std[j];
                z += model.weights[j] * x;
            }
            sigmoid(z)
        }
        MlConfidenceModelKind::GbdtStump => {
            let gbdt = model.gbdt.as_ref()?;
            predict_prob_gbdt_row(gbdt, &x0)?
        }
    };
    if let Some(cal) = model.calibration.as_ref() {
        Some(apply_calibration(p_raw, cal))
    } else {
        Some(p_raw)
    }
}

pub fn predict_ml_confidence_for_group(model: &MlConfidenceModel, group: &Value) -> Option<f64> {
    let features = extract_ml_confidence_group_features(group)?;
    predict_ml_confidence_for_features(model, &features)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn build_manual_rows(rows: usize, groups: usize) -> Vec<MlConfidenceDatasetRow> {
        let group_count = groups.max(1);
        (0..rows)
            .map(|idx| {
                let positive = idx % 2 == 0;
                let group_id = format!("group-{:03}", idx % group_count);
                MlConfidenceDatasetRow {
                    group_id: group_id.clone(),
                    cv_group_id: group_id,
                    x: vec![
                        32.0 + idx as f64,
                        if positive { 0.78 } else { 0.24 },
                        if positive { 0.12 } else { -0.08 },
                        if positive { 0.18 } else { -0.03 },
                        if positive { 0.22 } else { 0.58 },
                        if positive { 0.09 } else { -0.06 },
                        if positive { 0.21 } else { -0.19 },
                    ],
                    y: if positive { 1.0 } else { 0.0 },
                }
            })
            .collect()
    }

    fn mixed_history_scope_cfg_and_artifacts(
    ) -> (MlConfidenceScope, MlConfidenceTrainConfig, Vec<Value>) {
        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "1m".to_string(),
        };

        let mut cfg = MlConfidenceTrainConfig::default();
        cfg.target = MlConfidenceTarget::Auto;
        cfg.label.min_holdout_datasets = 1;
        cfg.label.min_holdout_pass_rate = 0.60;
        cfg.label.min_holdout_score_p10 = 0.0;
        cfg.label.max_holdout_max_dd_p90 = Some(0.60);

        let gate_group = |good: bool, gid: usize| {
            json!({
                "samples": 48,
                "pass_rate": if good { 0.82 } else { 0.28 },
                "robust_score_p10": if good { 0.18 } else { -0.10 },
                "robust_score_median": if good { 0.24 } else { -0.04 },
                "max_dd_p90": if good { 0.22 } else { 0.58 },
                "cross_score_median": if good { 0.08 } else { -0.04 },
                "meta_score": if good { 0.26 } else { -0.16 },
                "key": format!("gate-{gid:03}"),
                "holdout": {
                    "datasets": 12,
                    "pass_rate": if good { 0.80 } else { 0.20 },
                    "score_p10": if good { 0.11 } else { -0.09 },
                    "score_median": if good { 0.18 } else { -0.02 },
                    "sharpe_median": if good { 1.2 } else { -0.3 },
                    "max_dd_p90": if good { 0.24 } else { 0.58 },
                    "pnl_median": if good { 1.4 } else { -0.8 }
                }
            })
        };

        let dataset_group = |good: bool, gid: usize| {
            let mut rows = Vec::new();
            for ds in 0..8usize {
                let pass = if good { ds != 0 } else { ds == 0 };
                rows.push(json!({
                    "dataset": format!("DS{gid:03}_{ds:02}"),
                    "score_median": if pass { 0.22 } else { -0.12 },
                    "score_p10": if pass { 0.10 } else { -0.18 },
                    "sharpe_median": if pass { 1.0 } else { -0.6 },
                    "max_dd_worst": if pass { 0.26 } else { 0.62 },
                    "pnl_median": if pass { 1.1 } else { -0.9 },
                    "pass": pass
                }));
            }
            json!({
                "samples": 52,
                "pass_rate": if good { 0.84 } else { 0.24 },
                "robust_score_p10": if good { 0.20 } else { -0.12 },
                "robust_score_median": if good { 0.27 } else { -0.05 },
                "max_dd_p90": if good { 0.20 } else { 0.60 },
                "cross_score_median": if good { 0.09 } else { -0.06 },
                "meta_score": if good { 0.30 } else { -0.20 },
                "key": format!("rows-{gid:03}"),
                "holdout": {
                    "datasets": 8,
                    "pass_rate": if good { 0.88 } else { 0.12 },
                    "score_p10": if good { 0.10 } else { -0.18 },
                    "score_median": if good { 0.18 } else { -0.05 },
                    "sharpe_median": if good { 1.1 } else { -0.4 },
                    "max_dd_p90": if good { 0.22 } else { 0.55 },
                    "pnl_median": if good { 1.0 } else { -1.0 }
                },
                "holdout_rows": rows
            })
        };

        let gate_only_meta = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-03-01T00:00:00Z",
            "req": { "strategy_kind": "CompositeMo", "timeframe": "1m" },
            "res": {
                "strategy_kind": "CompositeMo",
                "timeframe": "1m",
                "groups": (0..30usize).map(|i| gate_group(i % 2 == 0, i)).collect::<Vec<_>>()
            }
        });
        let dataset_meta = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-03-02T00:00:00Z",
            "req": { "strategy_kind": "CompositeMo", "timeframe": "1m" },
            "res": {
                "strategy_kind": "CompositeMo",
                "timeframe": "1m",
                "groups": (0..10usize).map(|i| dataset_group(i % 2 == 0, i)).collect::<Vec<_>>()
            }
        });

        (scope, cfg, vec![gate_only_meta, dataset_meta])
    }

    #[test]
    fn trains_and_predicts_confidence_from_meta_artifacts() {
        let mk_group = |samples: u64,
                        pass_rate: f64,
                        robust_p10: f64,
                        robust_med: f64,
                        dd_p90: f64,
                        cross_med: f64,
                        meta: f64,
                        hold_pass: f64,
                        hold_p10: f64,
                        hold_dd: f64,
                        hold_datasets: u64| {
            json!({
                "samples": samples,
                "pass_rate": pass_rate,
                "robust_score_p10": robust_p10,
                "robust_score_median": robust_med,
                "max_dd_p90": dd_p90,
                "cross_score_median": cross_med,
                "meta_score": meta,
                "holdout": {
                    "datasets": hold_datasets,
                    "pass_rate": hold_pass,
                    "score_p10": hold_p10,
                    "score_median": hold_p10 + 0.1,
                    "sharpe_median": 1.0,
                    "max_dd_p90": hold_dd,
                    "pnl_median": 1.0
                }
            })
        };

        // Two meta runs, each with a mix of positives and negatives.
        let mut groups1 = Vec::new();
        let mut groups2 = Vec::new();
        for i in 0..30 {
            let good = i % 2 == 0;
            let (hp, hp10) = if good { (0.75, 0.10) } else { (0.30, -0.05) };
            groups1.push(mk_group(
                40,
                0.8,
                if good { 0.2 } else { -0.1 },
                if good { 0.25 } else { -0.05 },
                0.20,
                0.10,
                if good { 0.3 } else { -0.2 },
                hp,
                hp10,
                0.30,
                50,
            ));
            groups2.push(mk_group(
                35,
                0.7,
                if good { 0.18 } else { -0.08 },
                if good { 0.22 } else { -0.04 },
                0.25,
                0.05,
                if good { 0.25 } else { -0.15 },
                hp,
                hp10,
                0.35,
                50,
            ));
        }

        let meta1 = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-01-28T00:00:00Z",
            "req": { "strategy_kind": "CompositeMo", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeMo", "timeframe": "1m", "groups": groups1 }
        });
        let meta2 = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-01-28T00:01:00Z",
            "req": { "strategy_kind": "CompositeMo", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeMo", "timeframe": "1m", "groups": groups2 }
        });

        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "1m".to_string(),
        };
        let cfg = MlConfidenceTrainConfig::default();
        let (model, metrics) =
            train_ml_confidence_model_from_meta_artifacts(scope, &[meta1, meta2], cfg).unwrap();

        assert_eq!(model.scope.timeframe, "1m");
        assert!(metrics.rows >= 50);
        assert!(metrics.train_auc > 0.7);

        // Sanity: "good-looking" group should have higher confidence than "bad-looking".
        let good_g = json!({
            "samples": 40,
            "pass_rate": 0.8,
            "robust_score_p10": 0.2,
            "robust_score_median": 0.25,
            "max_dd_p90": 0.2,
            "cross_score_median": 0.1,
            "meta_score": 0.3
        });
        let bad_g = json!({
            "samples": 40,
            "pass_rate": 0.2,
            "robust_score_p10": -0.1,
            "robust_score_median": -0.05,
            "max_dd_p90": 0.6,
            "cross_score_median": -0.1,
            "meta_score": -0.2
        });

        let p_good = predict_ml_confidence_for_group(&model, &good_g).unwrap();
        let p_bad = predict_ml_confidence_for_group(&model, &bad_g).unwrap();
        assert!(p_good > p_bad);
    }

    #[test]
    fn prepared_rows_match_raw_training_and_feature_prediction() {
        let (scope, cfg, metas) = mixed_history_scope_cfg_and_artifacts();
        let prepared = prepare_ml_confidence_rows_from_meta_artifacts(&scope, &metas, &cfg.label);

        let (raw_model, raw_metrics) =
            train_ml_confidence_model_from_meta_artifacts(scope.clone(), &metas, cfg.clone())
                .unwrap();
        let (prepared_model, prepared_metrics) =
            train_ml_confidence_model_from_prepared_rows(scope, &prepared, cfg, None).unwrap();

        assert_eq!(raw_metrics.target, MlConfidenceTarget::HoldoutDatasetPass);
        assert_eq!(raw_metrics.target, prepared_metrics.target);
        assert_eq!(raw_metrics.rows, prepared_metrics.rows);
        assert_eq!(raw_metrics.positives, prepared_metrics.positives);
        assert_eq!(raw_metrics.negatives, prepared_metrics.negatives);
        assert_eq!(
            raw_metrics.meta_artifacts_used,
            prepared_metrics.meta_artifacts_used
        );
        assert_eq!(
            raw_metrics.newest_meta_ts_used,
            prepared_metrics.newest_meta_ts_used
        );

        let good_group = json!({
            "samples": 52,
            "pass_rate": 0.84,
            "robust_score_p10": 0.20,
            "robust_score_median": 0.27,
            "max_dd_p90": 0.20,
            "cross_score_median": 0.09,
            "meta_score": 0.30
        });
        let features = extract_ml_confidence_group_features(&good_group).unwrap();
        let p_group = predict_ml_confidence_for_group(&raw_model, &good_group).unwrap();
        let p_features = predict_ml_confidence_for_features(&raw_model, &features).unwrap();
        let p_prepared = predict_ml_confidence_for_features(&prepared_model, &features).unwrap();

        assert!((p_group - p_features).abs() < 1e-12);
        assert!((p_group - p_prepared).abs() < 1e-12);
    }

    #[test]
    fn extract_features_preserves_missing_cross_score_median() {
        let group = json!({
            "samples": 40,
            "pass_rate": 0.75,
            "robust_score_p10": 0.10,
            "robust_score_median": 0.18,
            "max_dd_p90": 0.22,
            "meta_score": 0.24
        });

        let features = extract_ml_confidence_group_features(&group).expect("features");
        assert_eq!(features.cross_score_median, None);
        assert!(features.to_feature_vec().is_none());
    }

    #[test]
    fn prepared_rows_skip_incompatible_history_contracts() {
        let (scope, cfg, metas) = mixed_history_scope_cfg_and_artifacts();
        let prepared = prepare_ml_confidence_rows_from_meta_artifacts(&scope, &metas, &cfg.label);
        let compatibility = prepared
            .compatibility_key
            .as_ref()
            .expect("active compatibility key");

        assert_eq!(prepared.artifacts.len(), 1);
        assert_eq!(prepared.skipped_incompatible_artifacts, 1);
        assert_eq!(
            compatibility.target_family,
            "holdout_dataset_pass_available"
        );
        assert_eq!(compatibility.train_score_semantics, "meta_score_v1_full");
        assert_eq!(compatibility.grouping_mode, "legacy_unknown");
        assert_eq!(compatibility.shortlist_policy, "legacy_unknown");
        assert!(prepared
            .warnings
            .iter()
            .any(|warning| warning.contains("incompatible ML-confidence history contracts")));
    }

    #[test]
    fn prepared_row_prefix_auto_target_preserves_meta_usage() {
        let (scope, cfg, metas) = mixed_history_scope_cfg_and_artifacts();
        let prepared = prepare_ml_confidence_rows_from_meta_artifacts(&scope, &metas, &cfg.label);

        let (_gate_model, gate_metrics) = train_ml_confidence_model_from_prepared_rows(
            scope.clone(),
            &prepared,
            cfg.clone(),
            Some(1),
        )
        .unwrap();
        let (_dataset_model, dataset_metrics) =
            train_ml_confidence_model_from_prepared_rows(scope, &prepared, cfg, Some(2)).unwrap();

        assert_eq!(gate_metrics.target, MlConfidenceTarget::HoldoutDatasetPass);
        assert_eq!(gate_metrics.meta_artifacts_used, Some(1));
        assert_eq!(
            gate_metrics.newest_meta_ts_used.as_deref(),
            Some("2026-03-02T00:00:00Z")
        );

        assert_eq!(
            dataset_metrics.target,
            MlConfidenceTarget::HoldoutDatasetPass
        );
        assert_eq!(dataset_metrics.meta_artifacts_used, Some(1));
        assert_eq!(
            dataset_metrics.newest_meta_ts_used.as_deref(),
            Some("2026-03-02T00:00:00Z")
        );
    }

    #[test]
    fn prepared_row_training_persists_compatibility_key_in_model_meta() {
        let (scope, cfg, metas) = mixed_history_scope_cfg_and_artifacts();
        let prepared = prepare_ml_confidence_rows_from_meta_artifacts(&scope, &metas, &cfg.label);
        let (model, _metrics) =
            train_ml_confidence_model_from_prepared_rows(scope, &prepared, cfg, None).unwrap();

        assert_eq!(
            model
                .meta
                .as_ref()
                .and_then(|meta| meta.compatibility_key.as_ref()),
            prepared.compatibility_key.as_ref()
        );
        let serialized = serde_json::to_value(
            model
                .meta
                .as_ref()
                .and_then(|meta| meta.compatibility_key.clone())
                .expect("compatibility key"),
        )
        .expect("serialize compatibility key");
        assert_eq!(
            serialized
                .get("target_family")
                .and_then(|value| value.as_str()),
            Some("holdout_dataset_pass_available")
        );
    }

    #[test]
    fn raw_training_normalizes_equivalent_scope_timeframes() {
        let rows = build_manual_rows(30, 3);
        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "60s".to_string(),
        };
        let (model, metrics) = train_ml_confidence_model_from_rows(
            scope,
            &rows,
            MlConfidenceTrainConfig::default(),
            MlConfidenceTarget::HoldoutGate,
            0,
            None,
            Vec::new(),
        )
        .expect("raw training with equivalent timeframe label");
        assert_eq!(model.scope.timeframe, "1m");
        assert_eq!(metrics.rows, 30);
    }

    #[test]
    fn prepared_rows_scope_mismatch_is_rejected() {
        let (scope, cfg, metas) = mixed_history_scope_cfg_and_artifacts();
        let prepared = prepare_ml_confidence_rows_from_meta_artifacts(&scope, &metas, &cfg.label);
        let mismatch_scope = MlConfidenceScope {
            strategy_kind: "OtherStrategy".to_string(),
            timeframe: "1m".to_string(),
        };
        let err =
            train_ml_confidence_model_from_prepared_rows(mismatch_scope, &prepared, cfg, None)
                .expect_err("prepared scope mismatch should fail");
        assert!(err.contains("prepared rows scope mismatch"));
    }

    #[test]
    fn raw_training_rejects_feature_width_mismatch() {
        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "1m".to_string(),
        };
        let mut rows = build_manual_rows(30, 3);
        rows[0].x.pop();

        let err = train_ml_confidence_model_from_rows(
            scope,
            &rows,
            MlConfidenceTrainConfig::default(),
            MlConfidenceTarget::HoldoutGate,
            0,
            None,
            Vec::new(),
        )
        .expect_err("feature width mismatch should fail");
        assert!(err.contains("expected 7"));
    }

    #[test]
    fn raw_training_rejects_non_finite_feature_values() {
        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "1m".to_string(),
        };
        let mut rows = build_manual_rows(30, 3);
        rows[0].x[2] = f64::INFINITY;

        let err = train_ml_confidence_model_from_rows(
            scope,
            &rows,
            MlConfidenceTrainConfig::default(),
            MlConfidenceTarget::HoldoutGate,
            0,
            None,
            Vec::new(),
        )
        .expect_err("non-finite feature should fail");
        assert!(err.contains("non-finite feature"));
    }

    #[test]
    fn train_only_evidence_warnings_are_explicit_when_cv_is_unavailable() {
        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "1m".to_string(),
        };
        let rows = build_manual_rows(30, 1);
        let (model, metrics) = train_ml_confidence_model_from_rows(
            scope,
            &rows,
            MlConfidenceTrainConfig::default(),
            MlConfidenceTarget::HoldoutGate,
            0,
            None,
            Vec::new(),
        )
        .expect("train-only evidence training");

        assert!(metrics.cv_auc.is_none());
        assert!(metrics.cv_brier.is_none());
        assert!(metrics
            .warnings
            .iter()
            .any(|warning| warning.contains("train AUC")));
        assert!(metrics
            .warnings
            .iter()
            .any(|warning| warning.contains("train Brier and calibration bins")));
        let meta = model.meta.expect("model metadata");
        assert!(meta
            .warnings
            .iter()
            .any(|warning| warning.contains("train AUC")));
        assert!(meta
            .warnings
            .iter()
            .any(|warning| warning.contains("train Brier and calibration bins")));
    }

    #[test]
    fn dataset_pass_training_groups_cv_by_meta_run_instead_of_dataset_id() {
        let mk_group = |gid: usize| {
            let mut rows = Vec::new();
            for ds in 0..6usize {
                let pass = (gid + ds) % 2 == 0;
                rows.push(json!({
                    "dataset": format!("DS{gid:03}_{ds:02}"),
                    "score_median": if pass { 0.18 } else { -0.12 },
                    "score_p10": if pass { 0.08 } else { -0.16 },
                    "sharpe_median": if pass { 0.90 } else { -0.55 },
                    "max_dd_worst": if pass { 0.24 } else { 0.58 },
                    "pnl_median": if pass { 0.80 } else { -0.70 },
                    "pass": pass
                }));
            }
            json!({
                "samples": 44,
                "pass_rate": 0.55,
                "robust_score_p10": 0.02,
                "robust_score_median": 0.08,
                "max_dd_p90": 0.30,
                "cross_score_median": 0.01,
                "meta_score": 0.06,
                "key": format!("rows-{gid:03}"),
                "holdout": {
                    "datasets": rows.len(),
                    "pass_rate": 0.50,
                    "score_p10": 0.00,
                    "score_median": 0.04,
                    "sharpe_median": 0.20,
                    "max_dd_p90": 0.30,
                    "pnl_median": 0.10
                },
                "holdout_rows": rows
            })
        };

        let meta = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-04-01T00:00:00Z",
            "req": { "strategy_kind": "CompositeMo", "timeframe": "1m" },
            "res": {
                "strategy_kind": "CompositeMo",
                "timeframe": "1m",
                "groups": (0..6usize).map(mk_group).collect::<Vec<_>>()
            }
        });
        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "1m".to_string(),
        };
        let mut cfg = MlConfidenceTrainConfig::default();
        cfg.target = MlConfidenceTarget::HoldoutDatasetPass;
        cfg.label.min_holdout_datasets = 1;
        cfg.label.min_holdout_score_p10 = 0.0;

        let prepared =
            prepare_ml_confidence_rows_from_meta_artifacts(&scope, &[meta.clone()], &cfg.label);
        let dataset_rows = &prepared.artifacts[0].holdout_dataset_rows;
        let unique_cv_groups: std::collections::HashSet<_> = dataset_rows
            .iter()
            .map(|row| row.cv_group_id.clone())
            .collect();
        let unique_dataset_ids: std::collections::HashSet<_> = dataset_rows
            .iter()
            .map(|row| row.group_id.clone())
            .collect();
        assert!(dataset_rows.len() >= 25);
        assert_eq!(unique_cv_groups.len(), 1);
        assert!(unique_dataset_ids.len() > 1);

        let (_model, metrics) =
            train_ml_confidence_model_from_meta_artifacts(scope, &[meta], cfg).unwrap();
        assert!(metrics.cv_auc.is_none());
        assert!(metrics
            .warnings
            .iter()
            .any(|warning| warning.contains("CV skipped (need at least 2 unique meta runs).")));
    }

    #[test]
    fn trains_from_holdout_rows_dataset_labels() {
        // Build multiple groups with different feature strength. "Good" groups have more passing
        // holdout datasets; "bad" groups have fewer.
        let mk_group = |good: bool, gid: usize| {
            let mut rows = Vec::new();
            for ds in 0..50usize {
                let pass = if good { ds % 5 != 0 } else { ds % 5 == 0 };
                rows.push(json!({
                    "dataset": format!("DS{ds:03}"),
                    "score_median": if pass { 0.20 } else { -0.10 },
                    "score_p10": if pass { 0.10 } else { -0.20 },
                    "sharpe_median": if pass { 1.0 } else { -0.5 },
                    "max_dd_worst": if pass { 0.25 } else { 0.60 },
                    "pnl_median": if pass { 1.0 } else { -1.0 },
                    "pass": pass
                }));
            }
            json!({
                "samples": 40,
                "pass_rate": if good { 0.8 } else { 0.2 },
                "robust_score_p10": if good { 0.2 } else { -0.1 },
                "robust_score_median": if good { 0.25 } else { -0.05 },
                "max_dd_p90": if good { 0.20 } else { 0.60 },
                "cross_score_median": if good { 0.10 } else { -0.05 },
                "meta_score": if good { 0.3 } else { -0.2 },
                "key": format!("G{gid}"),
                "holdout_rows": rows
            })
        };

        let mut groups = Vec::new();
        for i in 0..6usize {
            let good = i < 3;
            groups.push(mk_group(good, i));
        }

        let meta = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-01-29T12:00:00Z",
            "req": { "strategy_kind": "CompositeMo", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeMo", "timeframe": "1m", "groups": groups }
        });

        let scope = MlConfidenceScope {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: "1m".to_string(),
        };
        let mut cfg = MlConfidenceTrainConfig::default();
        cfg.target = MlConfidenceTarget::HoldoutDatasetPass;
        cfg.label.min_holdout_datasets = 1;
        cfg.label.min_holdout_score_p10 = 0.0;

        let (model, metrics) =
            train_ml_confidence_model_from_meta_artifacts(scope, &[meta], cfg).unwrap();

        assert_eq!(metrics.target, MlConfidenceTarget::HoldoutDatasetPass);
        assert!(metrics.rows >= 6 * 50);

        let good_g = json!({
            "samples": 40,
            "pass_rate": 0.8,
            "robust_score_p10": 0.2,
            "robust_score_median": 0.25,
            "max_dd_p90": 0.20,
            "cross_score_median": 0.10,
            "meta_score": 0.3
        });
        let bad_g = json!({
            "samples": 40,
            "pass_rate": 0.2,
            "robust_score_p10": -0.1,
            "robust_score_median": -0.05,
            "max_dd_p90": 0.60,
            "cross_score_median": -0.05,
            "meta_score": -0.2
        });

        let p_good = predict_ml_confidence_for_group(&model, &good_g).unwrap();
        let p_bad = predict_ml_confidence_for_group(&model, &bad_g).unwrap();
        assert!(p_good > p_bad);
    }

    #[test]
    fn trains_gbdt_when_requested() {
        let mk_group = |pass_rate: f64, p10: f64, dd: f64| {
            json!({
                "samples": 64,
                "pass_rate": pass_rate,
                "robust_score_p10": p10,
                "robust_score_median": p10 + 0.05,
                "max_dd_p90": dd,
                "cross_score_median": p10 * 0.5,
                "meta_score": p10 * 1.2,
                "holdout": {
                    "datasets": 50,
                    "pass_rate": pass_rate,
                    "score_p10": p10,
                    "score_median": p10 + 0.03,
                    "sharpe_median": 1.0,
                    "max_dd_p90": dd,
                    "pnl_median": 1.0
                }
            })
        };
        let mut groups1 = Vec::new();
        let mut groups2 = Vec::new();
        for i in 0..30 {
            let good = i % 2 == 0;
            groups1.push(if good {
                mk_group(0.78, 0.14, 0.22)
            } else {
                mk_group(0.30, -0.10, 0.58)
            });
            groups2.push(if good {
                mk_group(0.74, 0.12, 0.24)
            } else {
                mk_group(0.34, -0.08, 0.55)
            });
        }
        let meta1 = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-01-30T00:00:00Z",
            "req": { "strategy_kind": "CompositeTrend", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeTrend", "timeframe": "1m", "groups": groups1 }
        });
        let meta2 = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-01-30T00:01:00Z",
            "req": { "strategy_kind": "CompositeTrend", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeTrend", "timeframe": "1m", "groups": groups2 }
        });

        let mut cfg = MlConfidenceTrainConfig::default();
        cfg.model_family = MlConfidenceModelFamily::GbdtStump;
        let scope = MlConfidenceScope {
            strategy_kind: "CompositeTrend".to_string(),
            timeframe: "1m".to_string(),
        };
        let (model, metrics) =
            train_ml_confidence_model_from_meta_artifacts(scope, &[meta1, meta2], cfg).unwrap();
        assert_eq!(model.model_kind, MlConfidenceModelKind::GbdtStump);
        assert_eq!(metrics.model_kind, MlConfidenceModelKind::GbdtStump);

        let good_g = json!({
            "samples": 64,
            "pass_rate": 0.79,
            "robust_score_p10": 0.15,
            "robust_score_median": 0.20,
            "max_dd_p90": 0.20,
            "cross_score_median": 0.07,
            "meta_score": 0.24
        });
        let bad_g = json!({
            "samples": 64,
            "pass_rate": 0.28,
            "robust_score_p10": -0.12,
            "robust_score_median": -0.08,
            "max_dd_p90": 0.60,
            "cross_score_median": -0.05,
            "meta_score": -0.18
        });
        let p_good = predict_ml_confidence_for_group(&model, &good_g).unwrap();
        let p_bad = predict_ml_confidence_for_group(&model, &bad_g).unwrap();
        assert!(p_good > p_bad);
    }
}
