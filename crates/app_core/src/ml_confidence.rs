use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Clone, Debug)]
struct DatasetRow {
    group_id: String,
    x: Vec<f64>,
    y: f64,
}

#[derive(Clone, Debug)]
struct DatasetBuildSummary {
    used_meta: usize,
    newest_meta_ts: Option<String>,
    target: MlConfidenceTargetResolved,
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

fn extract_features_from_meta_group(g: &Value) -> Option<Vec<f64>> {
    let samples = g.get("samples").and_then(|v| v.as_u64())? as f64;
    let pass_rate = g.get("pass_rate").and_then(|v| v.as_f64())?;
    let robust_p10 = g.get("robust_score_p10").and_then(|v| v.as_f64())?;
    let robust_median = g.get("robust_score_median").and_then(|v| v.as_f64())?;
    let max_dd_p90 = g.get("max_dd_p90").and_then(|v| v.as_f64())?;
    let cross_median = g
        .get("cross_score_median")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let meta_score = g.get("meta_score").and_then(|v| v.as_f64())?;

    let x = vec![
        samples,
        pass_rate,
        robust_p10,
        robust_median,
        max_dd_p90,
        cross_median,
        meta_score,
    ];
    if x.iter().all(|v| v.is_finite()) {
        Some(x)
    } else {
        None
    }
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

fn scope_has_holdout_rows(
    scope: &MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: &MlConfidenceLabelConfig,
) -> bool {
    let scope_tf = normalize_timeframe_label(&scope.timeframe);
    let scope_strat = scope.strategy_kind.trim();

    for meta in meta_artifacts {
        let kind = meta.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if kind != "vectorbt_meta_selection" {
            continue;
        }
        let Some((strat, tf)) = extract_meta_scope(meta) else {
            continue;
        };
        if strat != scope_strat || tf != scope_tf {
            continue;
        }

        let groups = meta
            .pointer("/res/groups")
            .and_then(|v| v.as_array())
            .map(|xs| xs.as_slice())
            .unwrap_or(&[]);
        for g in groups {
            let holdout_rows = g.get("holdout_rows").and_then(|v| v.as_array());
            let Some(rows) = holdout_rows else {
                continue;
            };
            if rows.len() < cfg.min_holdout_datasets {
                continue;
            }
            if rows
                .iter()
                .any(|r| label_from_holdout_row(r, cfg).is_some())
            {
                return true;
            }
        }
    }

    false
}

fn build_dataset_from_meta_artifacts(
    scope: &MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: &MlConfidenceLabelConfig,
    target: MlConfidenceTargetResolved,
) -> (Vec<DatasetRow>, DatasetBuildSummary, Vec<String>) {
    let mut warnings: Vec<String> = Vec::new();
    let scope_tf = normalize_timeframe_label(&scope.timeframe);
    let scope_strat = scope.strategy_kind.trim();

    let mut rows: Vec<DatasetRow> = Vec::new();
    let mut used_meta = 0usize;
    let mut newest_meta_ts: Option<String> = None;
    let mut skipped_kind = 0usize;
    let mut skipped_scope = 0usize;
    let mut skipped_missing = 0usize;
    let mut skipped_no_holdout = 0usize;
    let mut skipped_label = 0usize;
    let mut skipped_row = 0usize;

    for meta in meta_artifacts {
        let kind = meta.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if kind != "vectorbt_meta_selection" {
            skipped_kind += 1;
            continue;
        }
        let Some((strat, tf)) = extract_meta_scope(meta) else {
            skipped_missing += 1;
            continue;
        };
        if strat != scope_strat || tf != scope_tf {
            skipped_scope += 1;
            continue;
        }
        let group_id = meta
            .get("ts")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let groups = meta
            .pointer("/res/groups")
            .and_then(|v| v.as_array())
            .map(|xs| xs.as_slice())
            .unwrap_or(&[]);
        if groups.is_empty() {
            continue;
        }

        used_meta += 1;
        if let Some(ts) = meta
            .get("ts")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            let is_newest = newest_meta_ts
                .as_ref()
                .map(|cur| ts.as_str() > cur.as_str())
                .unwrap_or(true);
            if is_newest {
                newest_meta_ts = Some(ts);
            }
        }
        for g in groups {
            let Some(x) = extract_features_from_meta_group(g) else {
                continue;
            };

            match target {
                MlConfidenceTargetResolved::HoldoutGate => {
                    let Some(h) = g.get("holdout") else {
                        skipped_no_holdout += 1;
                        continue;
                    };
                    let Some(y) = label_from_holdout(h, cfg) else {
                        skipped_label += 1;
                        continue;
                    };
                    rows.push(DatasetRow {
                        group_id: group_id.clone(),
                        x,
                        y,
                    });
                }
                MlConfidenceTargetResolved::HoldoutDatasetPass => {
                    let Some(holdout_rows) = g.get("holdout_rows").and_then(|v| v.as_array())
                    else {
                        skipped_no_holdout += 1;
                        continue;
                    };
                    if holdout_rows.len() < cfg.min_holdout_datasets {
                        skipped_label += 1;
                        continue;
                    }
                    for r in holdout_rows {
                        let Some((ds, y)) = label_from_holdout_row(r, cfg) else {
                            skipped_row += 1;
                            continue;
                        };
                        rows.push(DatasetRow {
                            group_id: ds,
                            x: x.clone(),
                            y,
                        });
                    }
                }
            }
        }
    }

    if used_meta == 0 {
        warnings.push("No matching meta artifacts found for this scope.".to_string());
    }
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
    if skipped_kind > 0 {
        warnings.push(format!("Skipped {skipped_kind} non-meta JSON file(s)."));
    }
    if skipped_scope > 0 {
        warnings.push(format!(
            "Skipped {skipped_scope} meta artifact(s) outside scope."
        ));
    }
    if skipped_missing > 0 {
        warnings.push(format!(
            "Skipped {skipped_missing} meta artifact(s) missing strategy/timeframe."
        ));
    }
    if skipped_no_holdout > 0 {
        warnings.push(format!(
            "Skipped {skipped_no_holdout} meta group(s) without holdout results."
        ));
    }
    if skipped_label > 0 {
        warnings.push(format!(
            "Skipped {skipped_label} meta group(s) with insufficient holdout datasets (<{}).",
            cfg.min_holdout_datasets
        ));
    }
    if skipped_row > 0 {
        warnings.push(format!(
            "Skipped {skipped_row} holdout row(s) missing required fields."
        ));
    }

    (
        rows,
        DatasetBuildSummary {
            used_meta,
            newest_meta_ts,
            target,
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
            s.insert(r.group_id.clone());
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
                let f = (fnv1a64(&r.group_id) % (k as u64)) as usize;
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
            s.insert(r.group_id.clone());
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
                let f = (fnv1a64(&r.group_id) % (k as u64)) as usize;
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

pub fn train_ml_confidence_model_from_meta_artifacts(
    scope: MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: MlConfidenceTrainConfig,
) -> Result<(MlConfidenceModel, MlConfidenceTrainMetrics), String> {
    let timeframe = normalize_timeframe_label(&scope.timeframe);
    if timeframe.is_empty() {
        return Err("timeframe is required (e.g. '1m')".to_string());
    }
    if scope.strategy_kind.trim().is_empty() {
        return Err("strategy_kind is required".to_string());
    }
    let scope = MlConfidenceScope {
        strategy_kind: scope.strategy_kind.trim().to_string(),
        timeframe,
    };

    let target = match cfg.target {
        MlConfidenceTarget::HoldoutGate => MlConfidenceTargetResolved::HoldoutGate,
        MlConfidenceTarget::HoldoutDatasetPass => MlConfidenceTargetResolved::HoldoutDatasetPass,
        MlConfidenceTarget::Auto => {
            if scope_has_holdout_rows(&scope, meta_artifacts, &cfg.label) {
                MlConfidenceTargetResolved::HoldoutDatasetPass
            } else {
                MlConfidenceTargetResolved::HoldoutGate
            }
        }
    };

    let (rows, build, mut warnings) =
        build_dataset_from_meta_artifacts(&scope, meta_artifacts, &cfg.label, target);
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
        for r in &rows {
            s.insert(r.group_id.clone());
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

pub fn predict_ml_confidence_for_group(model: &MlConfidenceModel, group: &Value) -> Option<f64> {
    let x0 = extract_features_from_meta_group(group)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
            "kind": "vectorbt_meta_selection",
            "ts": "2026-01-28T00:00:00Z",
            "req": { "strategy_kind": "CompositeMo", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeMo", "timeframe": "1m", "groups": groups1 }
        });
        let meta2 = json!({
            "kind": "vectorbt_meta_selection",
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
            "kind": "vectorbt_meta_selection",
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
            "kind": "vectorbt_meta_selection",
            "ts": "2026-01-30T00:00:00Z",
            "req": { "strategy_kind": "CompositeTrend", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeTrend", "timeframe": "1m", "groups": groups1 }
        });
        let meta2 = json!({
            "kind": "vectorbt_meta_selection",
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
