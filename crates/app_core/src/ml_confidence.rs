use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_min_holdout_pass_rate() -> f64 {
    0.60
}

fn default_min_holdout_score_p10() -> f64 {
    0.0
}

fn default_min_holdout_datasets() -> usize {
    30
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlConfidenceTrainConfig {
    #[serde(default)]
    pub label: MlConfidenceLabelConfig,
    #[serde(default = "default_l2")]
    pub l2: f64,
    #[serde(default = "default_learning_rate")]
    pub learning_rate: f64,
    #[serde(default = "default_max_iter")]
    pub max_iter: usize,
    #[serde(default = "default_k_folds")]
    pub k_folds: usize,
}

impl Default for MlConfidenceTrainConfig {
    fn default() -> Self {
        Self {
            label: MlConfidenceLabelConfig::default(),
            l2: default_l2(),
            learning_rate: default_learning_rate(),
            max_iter: default_max_iter(),
            k_folds: default_k_folds(),
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
    pub train_auc: f64,
    pub train_brier: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cv_auc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cv_brier: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calibration: Vec<MlConfidenceCalibrationBin>,
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
    pub label: MlConfidenceLabelConfig,
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
    let cross_median = g.get("cross_score_median").and_then(|v| v.as_f64()).unwrap_or(0.0);
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

fn build_dataset_from_meta_artifacts(
    scope: &MlConfidenceScope,
    meta_artifacts: &[Value],
    cfg: &MlConfidenceLabelConfig,
) -> (Vec<DatasetRow>, Vec<String>) {
    let mut warnings: Vec<String> = Vec::new();
    let scope_tf = normalize_timeframe_label(&scope.timeframe);
    let scope_strat = scope.strategy_kind.trim();

    let mut rows: Vec<DatasetRow> = Vec::new();
    let mut used_meta = 0usize;
    let mut skipped_kind = 0usize;
    let mut skipped_scope = 0usize;
    let mut skipped_missing = 0usize;
    let mut skipped_no_holdout = 0usize;
    let mut skipped_label = 0usize;

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
        let group_id = meta.get("ts").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let groups = meta
            .pointer("/res/groups")
            .and_then(|v| v.as_array())
            .map(|xs| xs.as_slice())
            .unwrap_or(&[]);
        if groups.is_empty() {
            continue;
        }

        used_meta += 1;
        for g in groups {
            let Some(x) = extract_features_from_meta_group(g) else {
                continue;
            };
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
    }

    if used_meta == 0 {
        warnings.push("No matching meta artifacts found for this scope.".to_string());
    }
    if rows.is_empty() {
        warnings.push("No labeled rows found (need meta artifacts with holdout results).".to_string());
    }
    if skipped_kind > 0 {
        warnings.push(format!("Skipped {skipped_kind} non-meta JSON file(s)."));
    }
    if skipped_scope > 0 {
        warnings.push(format!("Skipped {skipped_scope} meta artifact(s) outside scope."));
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

    (rows, warnings)
}

fn sigmoid(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let ez = z.exp();
        ez / (1.0 + ez)
    }
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

    let (rows, mut warnings) = build_dataset_from_meta_artifacts(&scope, meta_artifacts, &cfg.label);

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

    let xs_raw: Vec<Vec<f64>> = rows.iter().map(|r| r.x.clone()).collect();
    let ys: Vec<f64> = rows.iter().map(|r| r.y).collect();

    // CV metrics (out-of-fold) using group_id hashing to avoid leakage across meta runs.
    let mut cv_auc: Option<f64> = None;
    let mut cv_brier: Option<f64> = None;
    let mut cv_ps: Vec<f64> = Vec::new();
    let mut cv_ys: Vec<f64> = Vec::new();
    let mut cv_warnings: Vec<String> = Vec::new();

    let uniq_groups = {
        use std::collections::HashSet;
        let mut s = HashSet::new();
        for r in &rows {
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
            if test_x.is_empty() {
                continue;
            }
            let pos = train_y.iter().filter(|&&y| y > 0.5).count();
            let neg = train_y.len().saturating_sub(pos);
            if pos == 0 || neg == 0 {
                cv_warnings.push(format!(
                    "Skipped CV fold {fold} due to single-class training split."
                ));
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

            let (w, b, _) =
                fit_logistic_regression(&train_xs, &train_y, cfg.l2, cfg.learning_rate, cfg.max_iter);
            let ps = predict_probs_raw(&w, b, &test_xs);

            cv_ps.extend(ps);
            cv_ys.extend(test_y);
        }

        if let Some(auc) = auc_roc(&cv_ps, &cv_ys) {
            cv_auc = Some(auc);
            cv_brier = Some(brier_score(&cv_ps, &cv_ys));
        } else {
            cv_warnings.push("CV AUC unavailable (need both classes in predictions).".to_string());
        }
    } else {
        cv_warnings.push("CV skipped (need at least 2 unique meta runs).".to_string());
    }

    warnings.extend(cv_warnings);

    // Fit final model on all rows.
    let scaler = StandardScaler::fit(&xs_raw);
    let mut xs: Vec<Vec<f64>> = xs_raw;
    for x in &mut xs {
        scaler.transform_row_in_place(x);
    }

    let (weights, bias, _) =
        fit_logistic_regression(&xs, &ys, cfg.l2, cfg.learning_rate, cfg.max_iter);
    let train_ps = predict_probs_raw(&weights, bias, &xs);

    let train_auc = auc_roc(&train_ps, &ys).unwrap_or(0.0);
    let train_brier = brier_score(&train_ps, &ys);

    let calib = if !cv_ps.is_empty() && cv_ps.len() == cv_ys.len() {
        calibration_bins(&cv_ps, &cv_ys, 10)
    } else {
        calibration_bins(&train_ps, &ys, 10)
    };

    let model = MlConfidenceModel {
        version: 1,
        scope: scope.clone(),
        feature_names: FEATURE_NAMES.iter().map(|s| s.to_string()).collect(),
        mean: scaler.mean,
        std: scaler.std,
        weights,
        bias,
        label: cfg.label,
    };

    let metrics = MlConfidenceTrainMetrics {
        rows: n,
        positives,
        negatives,
        train_auc,
        train_brier,
        cv_auc,
        cv_brier,
        calibration: calib,
        warnings,
    };

    Ok((model, metrics))
}

pub fn predict_ml_confidence_for_group(model: &MlConfidenceModel, group: &Value) -> Option<f64> {
    if model.weights.len() != model.mean.len()
        || model.weights.len() != model.std.len()
        || model.weights.is_empty()
    {
        return None;
    }
    let x0 = extract_features_from_meta_group(group)?;
    if x0.len() != model.weights.len() {
        return None;
    }
    let mut z = model.bias;
    for j in 0..x0.len() {
        let x = (x0[j] - model.mean[j]) / model.std[j];
        z += model.weights[j] * x;
    }
    Some(sigmoid(z))
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
}
