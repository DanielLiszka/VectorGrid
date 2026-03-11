use crate::progress::ProgressSink;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use crate::composite_validate::{PerturbConfig, TradeBootstrapConfig, WalkForwardConfigOptional};
use crate::iterative_sim::{IterativeSimConfig, ValidationEngine};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaScoreWeights {
    #[serde(default = "default_w_p10")]
    pub w_p10: f64,
    #[serde(default = "default_w_median")]
    pub w_median: f64,
    #[serde(default = "default_w_cross")]
    pub w_cross: f64,
    #[serde(default = "default_w_pass_rate")]
    pub w_pass_rate: f64,
    #[serde(default = "default_w_dd_p90")]
    pub w_dd_p90: f64,
    #[serde(default = "default_w_psr")]
    pub w_psr: f64,
    #[serde(default = "default_w_dsr")]
    pub w_dsr: f64,
    #[serde(default = "default_w_tail_gap")]
    pub w_tail_gap: f64,
    #[serde(default = "default_w_holdout_pass_rate")]
    pub w_holdout_pass_rate: f64,
    #[serde(default = "default_w_holdout_p10")]
    pub w_holdout_p10: f64,
    #[serde(default = "default_w_holdout_dd_p90")]
    pub w_holdout_dd_p90: f64,
}

fn default_w_p10() -> f64 {
    1.0
}
fn default_w_median() -> f64 {
    0.25
}
fn default_w_cross() -> f64 {
    0.25
}
fn default_w_pass_rate() -> f64 {
    0.10
}
fn default_w_dd_p90() -> f64 {
    0.75
}
fn default_w_psr() -> f64 {
    0.0
}
fn default_w_dsr() -> f64 {
    0.0
}
fn default_w_tail_gap() -> f64 {
    0.20
}
fn default_w_holdout_pass_rate() -> f64 {
    0.30
}
fn default_w_holdout_p10() -> f64 {
    1.00
}
fn default_w_holdout_dd_p90() -> f64 {
    0.75
}

impl Default for MetaScoreWeights {
    fn default() -> Self {
        Self {
            w_p10: default_w_p10(),
            w_median: default_w_median(),
            w_cross: default_w_cross(),
            w_pass_rate: default_w_pass_rate(),
            w_dd_p90: default_w_dd_p90(),
            w_psr: default_w_psr(),
            w_dsr: default_w_dsr(),
            w_tail_gap: default_w_tail_gap(),
            w_holdout_pass_rate: default_w_holdout_pass_rate(),
            w_holdout_p10: default_w_holdout_p10(),
            w_holdout_dd_p90: default_w_holdout_dd_p90(),
        }
    }
}

fn default_top_k() -> usize {
    25
}

fn default_min_samples() -> usize {
    30
}

fn default_strict_shortlist() -> bool {
    true
}

fn default_allow_shortlist_fallback() -> bool {
    false
}

fn default_eval_holdout() -> bool {
    true
}

fn default_cluster_representatives() -> usize {
    2
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetaGroupingMode {
    Exact,
    Clustered,
}

impl Default for MetaGroupingMode {
    fn default() -> Self {
        Self::Clustered
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetaHoldoutMode {
    Light,
    Heavy,
}

impl Default for MetaHoldoutMode {
    fn default() -> Self {
        Self::Light
    }
}

fn default_holdout_seed() -> u64 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaHoldoutValidationConfig {
    #[serde(default)]
    pub walk_forward: Option<WalkForwardConfigOptional>,
    #[serde(default)]
    pub perturb: Option<PerturbConfig>,
    #[serde(default)]
    pub trade_bootstrap: Option<TradeBootstrapConfig>,
    #[serde(default = "default_holdout_seed")]
    pub seed: u64,
    #[serde(default)]
    pub validation_engine: ValidationEngine,
    #[serde(default)]
    pub iterative: IterativeSimConfig,
}

fn default_int_bucket() -> i64 {
    5
}

fn default_float_bucket() -> f64 {
    0.25
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaClusterConfig {
    #[serde(default = "default_int_bucket")]
    pub int_bucket: i64,
    #[serde(default = "default_float_bucket")]
    pub float_bucket: f64,
}

impl Default for MetaClusterConfig {
    fn default() -> Self {
        Self {
            int_bucket: default_int_bucket(),
            float_bucket: default_float_bucket(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaSelectionRequest {
    pub strategy_kind: String,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default)]
    pub required_tag: Option<String>,
    #[serde(default)]
    pub holdout_tag: Option<String>,
    #[serde(default = "default_eval_holdout")]
    pub eval_holdout: bool,
    /// When holdout evaluation is enabled, optionally persist per-dataset holdout rows
    /// (useful for future ML improvements, but increases meta JSON size).
    #[serde(default)]
    pub save_holdout_rows: bool,
    /// Optionally evaluate holdout for only a subset of meta groups (leave others unlabeled).
    /// When `None` or `0`, evaluate all groups (default / backwards compatible).
    #[serde(default)]
    pub holdout_eval_top_n: Option<usize>,
    /// Optional exploration budget: evaluate additional groups with highest uncertainty
    /// (only relevant when the ranker is ML-based).
    #[serde(default)]
    pub holdout_eval_explore_n: usize,
    /// Ranking strategy used when selecting holdout groups: "auto" (default), "meta", or "ml".
    #[serde(default)]
    pub holdout_eval_rank_by: Option<String>,
    #[serde(default)]
    pub holdout_mode: MetaHoldoutMode,
    #[serde(default)]
    pub holdout_validation: Option<MetaHoldoutValidationConfig>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_min_samples")]
    pub min_samples: usize,
    #[serde(default = "default_strict_shortlist")]
    pub strict_shortlist: bool,
    /// When true, empty shortlist stages fall back to the broader candidate set.
    /// Disable this for a hard validation gate prior to meta aggregation.
    #[serde(default = "default_allow_shortlist_fallback")]
    pub allow_shortlist_fallback: bool,
    #[serde(default)]
    pub weights: MetaScoreWeights,
    #[serde(default)]
    pub grouping_mode: MetaGroupingMode,
    #[serde(default)]
    pub cluster_config: MetaClusterConfig,
    #[serde(default = "default_cluster_representatives")]
    pub cluster_representatives: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct HoldoutAggregate {
    pub datasets: usize,
    pub pass_rate: f64,
    pub score_median: f64,
    pub score_p10: f64,
    pub sharpe_median: f64,
    pub max_dd_p90: f64,
    pub pnl_median: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct HoldoutSummary {
    pub datasets: usize,
    pub skipped: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct MetaCandidateAggregate {
    pub params: Value,
    pub key: String,
    pub ma_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_context: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_context: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_ma_params: Option<HashMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_ma_params: Option<HashMap<String, f64>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub representatives: Vec<Value>,
    pub samples: usize,
    pub datasets_covered: usize,
    pub pass_rate: f64,
    pub robust_score_median: f64,
    pub robust_score_p10: f64,
    pub tail_gap: f64,
    pub max_dd_p90: f64,
    pub cross_score_median: f64,
    pub psr_median: f64,
    pub dsr_median: f64,
    pub meta_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout: Option<HoldoutAggregate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_flags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub train_rows: Vec<TrainDatasetRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holdout_rows: Vec<HoldoutDatasetRow>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HoldoutDatasetRow {
    pub dataset: String,
    pub score_median: f64,
    pub score_p10: f64,
    pub sharpe_median: f64,
    pub max_dd_worst: f64,
    pub pnl_median: f64,
    pub pass: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TrainDatasetRow {
    pub dataset: String,
    pub dataset_key: String,
    pub robust_score: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub sharpe: f64,
    pub max_dd: f64,
    pub trades: f64,
    pub exposure: f64,
    pub cross_score: f64,
    pub cross_pnl_positive_frac: f64,
    pub psr: f64,
    pub dsr: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MetaSelectionResult {
    pub runtime_ms: u64,
    pub strategy_kind: String,
    pub timeframe: Option<String>,
    pub artifacts_considered: usize,
    pub artifacts_used: usize,
    pub candidates_considered: usize,
    pub groups: Vec<MetaCandidateAggregate>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout: Option<HoldoutSummary>,
}

#[derive(Default)]
struct GroupAcc {
    params: Value,
    ma_source: String,
    feature_context: Option<Value>,
    run_context: Option<Value>,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    robust_scores: Vec<f64>,
    robust_params: Vec<(f64, Value)>,
    max_dds: Vec<f64>,
    cross_scores: Vec<f64>,
    psrs: Vec<f64>,
    dsrs: Vec<f64>,
    dataset_keys: HashSet<String>,
    train_rows: Vec<TrainDatasetRow>,
}

fn compute_meta_group_evidence_flags(group: &MetaCandidateAggregate) -> Vec<String> {
    let mut flags: Vec<String> = Vec::new();

    if group.samples < GROUP_EVIDENCE_MIN_SAMPLES_WARN {
        flags.push(format!(
            "train_samples_lt_{}",
            GROUP_EVIDENCE_MIN_SAMPLES_WARN
        ));
    }
    if group.pass_rate < GROUP_EVIDENCE_MIN_PASS_RATE_WARN {
        flags.push(format!(
            "train_pass_rate_lt_{:.2}",
            GROUP_EVIDENCE_MIN_PASS_RATE_WARN
        ));
    }
    if group.tail_gap > group.robust_score_p10.abs().max(0.25) {
        flags.push("train_tail_gap_wide".to_string());
    }

    if let Some(holdout) = group.holdout.as_ref() {
        if holdout.datasets < GROUP_EVIDENCE_MIN_HOLDOUT_DATASETS_WARN {
            flags.push(format!(
                "holdout_datasets_lt_{}",
                GROUP_EVIDENCE_MIN_HOLDOUT_DATASETS_WARN
            ));
        }
        if holdout.pass_rate < GROUP_EVIDENCE_MIN_HOLDOUT_PASS_RATE_WARN {
            flags.push(format!(
                "holdout_pass_rate_lt_{:.2}",
                GROUP_EVIDENCE_MIN_HOLDOUT_PASS_RATE_WARN
            ));
        }
        if holdout.score_p10 <= 0.0 {
            flags.push("holdout_score_p10_non_positive".to_string());
        }
        if holdout.max_dd_p90 > GROUP_EVIDENCE_MAX_HOLDOUT_DD_P90_WARN {
            flags.push(format!(
                "holdout_dd_p90_gt_{:.2}",
                GROUP_EVIDENCE_MAX_HOLDOUT_DD_P90_WARN
            ));
        }
    }

    flags
}

pub fn refresh_meta_group_evidence_flags(group: &mut MetaCandidateAggregate) {
    group.evidence_flags = compute_meta_group_evidence_flags(group);
}

const GROUP_GATE_MIN_ROBUST_P10: f64 = 0.0;
const GROUP_GATE_MIN_PASS_RATE: f64 = 0.15;
const GROUP_GATE_MAX_DD_P90: f64 = 0.80;
const GROUP_EVIDENCE_MIN_SAMPLES_WARN: usize = 3;
const GROUP_EVIDENCE_MIN_PASS_RATE_WARN: f64 = 0.60;
const GROUP_EVIDENCE_MIN_HOLDOUT_DATASETS_WARN: usize = 3;
const GROUP_EVIDENCE_MIN_HOLDOUT_PASS_RATE_WARN: f64 = 0.50;
const GROUP_EVIDENCE_MAX_HOLDOUT_DD_P90_WARN: f64 = 0.50;

pub fn meta_select_from_artifacts(
    req: &MetaSelectionRequest,
    artifacts: &[Value],
    progress: Option<&dyn ProgressSink>,
) -> MetaSelectionResult {
    let t0 = Instant::now();
    let mut warnings: Vec<String> = Vec::new();
    let mut groups: HashMap<String, GroupAcc> = HashMap::new();

    let mut artifacts_considered = 0usize;
    let mut artifacts_used = 0usize;
    let mut candidates_considered = 0usize;
    let mut artifacts_rejected_by_shortlist = 0usize;
    let mut artifacts_rejected_by_validation_verdict = 0usize;
    let mut datasets_considered: HashSet<String> = HashSet::new();

    for (i, art) in artifacts.iter().enumerate() {
        artifacts_considered += 1;

        if !artifact_matches_request(art, req) {
            continue;
        }

        let candidates = match art
            .pointer("/validation/res/candidates")
            .and_then(|v| v.as_array())
        {
            Some(xs) => xs,
            None => continue,
        };
        if candidates.is_empty() {
            continue;
        }

        if matches!(artifact_backend_validation_approved(art), Some(false)) {
            artifacts_rejected_by_validation_verdict =
                artifacts_rejected_by_validation_verdict.saturating_add(1);
            continue;
        }
        artifacts_used += 1;

        if i % 25 == 0 || i + 1 == artifacts.len() {
            emit_progress(progress, i + 1, artifacts.len(), "meta_scan");
        }

        let cscv = art
            .pointer("/validation/res/cscv_pbo")
            .and_then(|v| v.as_object());
        let cscv_threshold = cscv
            .and_then(|o| o.get("threshold").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let cscv_combos = cscv
            .and_then(|o| o.get("combinations").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let selected = shortlist_candidates(
            candidates,
            req.strict_shortlist,
            if cscv_combos > 0 {
                Some(cscv_threshold)
            } else {
                None
            },
            req.allow_shortlist_fallback,
        );
        if selected.is_empty() {
            artifacts_rejected_by_shortlist = artifacts_rejected_by_shortlist.saturating_add(1);
            continue;
        }
        let dataset_key = artifact_dataset_key(art, i);
        let dataset_label = artifact_dataset_label(art, i);
        let artifact_ts = art
            .get("ts")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        datasets_considered.insert(dataset_key.clone());

        let ma_source = artifact_ma_source(art).unwrap_or_else(|| "close".to_string());
        let fast_ma_params = artifact_ma_params(art, "fast");
        let slow_ma_params = artifact_ma_params(art, "slow");
        let feature_context = artifact_feature_context(art);
        let run_context = artifact_run_context(art);

        let mut seen_keys: HashSet<String> = HashSet::new();
        for cand in selected {
            let params = cand.get("params").unwrap_or(&Value::Null);
            let key = meta_group_key_for_request_with_context(
                req,
                params,
                &ma_source,
                &fast_ma_params,
                &slow_ma_params,
                feature_context.as_ref(),
                run_context.as_ref(),
            );
            if !seen_keys.insert(key.clone()) {
                continue;
            }
            let robust_score = cand
                .get("robust_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let max_dd = cand
                .pointer("/walk_forward/max_dd_worst")
                .and_then(|v| v.as_f64())
                .or_else(|| cand.pointer("/in_sample/max_dd").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            let cross_score = cand
                .pointer("/cross_ticker/score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let psr = cand.get("psr").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let dsr = cand.get("dsr").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let pnl = cand
                .pointer("/in_sample/pnl")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let pnl_pct = cand
                .pointer("/in_sample/pnl_pct")
                .and_then(|v| v.as_f64())
                .or_else(|| {
                    cand.pointer("/in_sample/net_profit_pct")
                        .and_then(|v| v.as_f64())
                })
                .unwrap_or(pnl);
            let sharpe = cand
                .pointer("/in_sample/sharpe")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let in_sample_dd = cand
                .pointer("/in_sample/max_dd")
                .and_then(|v| v.as_f64())
                .unwrap_or(max_dd);
            let trades = cand
                .pointer("/in_sample/trades")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let exposure = cand
                .pointer("/in_sample/exposure")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let cross_pnl_positive_frac = cand
                .pointer("/cross_ticker/pnl_positive_frac")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            candidates_considered += 1;

            let entry = groups.entry(key.clone()).or_insert_with(|| GroupAcc {
                params: params.clone(),
                ma_source: ma_source.clone(),
                feature_context: feature_context.clone(),
                run_context: run_context.clone(),
                fast_ma_params: fast_ma_params.clone(),
                slow_ma_params: slow_ma_params.clone(),
                ..GroupAcc::default()
            });
            entry.robust_scores.push(robust_score);
            entry.robust_params.push((robust_score, params.clone()));
            entry.max_dds.push(max_dd);
            entry.cross_scores.push(cross_score);
            entry.psrs.push(psr);
            entry.dsrs.push(dsr);
            entry.dataset_keys.insert(dataset_key.clone());
            entry.train_rows.push(TrainDatasetRow {
                dataset: dataset_label.clone(),
                dataset_key: dataset_key.clone(),
                robust_score,
                pnl,
                pnl_pct,
                sharpe,
                max_dd: in_sample_dd,
                trades,
                exposure,
                cross_score,
                cross_pnl_positive_frac,
                psr,
                dsr,
                ts: artifact_ts.clone(),
            });
        }
    }

    emit_progress(progress, 1, 1, "meta_aggregate");

    let mut aggregates: Vec<MetaCandidateAggregate> = Vec::with_capacity(groups.len());
    let dataset_universe = datasets_considered.len();
    let mut dropped_gate_robust = 0usize;
    let mut dropped_gate_pass = 0usize;
    let mut dropped_gate_dd = 0usize;
    for (key, mut acc) in groups {
        let samples = acc.robust_scores.len();
        if samples < req.min_samples {
            continue;
        }

        if !acc.robust_params.is_empty() {
            acc.robust_params.sort_by(|a, b| a.0.total_cmp(&b.0));
            let idx = quantile_index(acc.robust_params.len(), 0.5);
            if let Some((_, p)) = acc.robust_params.get(idx) {
                acc.params = p.clone();
            }
        }

        let representatives = if req.grouping_mode == MetaGroupingMode::Exact {
            vec![acc.params.clone()]
        } else {
            pick_representatives(&acc.robust_params, req.cluster_representatives.clamp(1, 3))
        };
        acc.train_rows.sort_by(|a, b| {
            a.dataset
                .cmp(&b.dataset)
                .then_with(|| a.dataset_key.cmp(&b.dataset_key))
                .then_with(|| a.ts.cmp(&b.ts))
        });

        acc.robust_scores.sort_by(|a, b| a.total_cmp(b));
        acc.max_dds.sort_by(|a, b| a.total_cmp(b));
        acc.cross_scores.sort_by(|a, b| a.total_cmp(b));
        acc.psrs.sort_by(|a, b| a.total_cmp(b));
        acc.dsrs.sort_by(|a, b| a.total_cmp(b));

        let robust_score_median = median(&acc.robust_scores);
        let robust_score_p10 = quantile(&acc.robust_scores, 0.10);
        let max_dd_p90 = quantile(&acc.max_dds, 0.90);
        let cross_score_median = median(&acc.cross_scores);
        let psr_median = median(&acc.psrs);
        let dsr_median = median(&acc.dsrs);
        let datasets_covered = acc.dataset_keys.len();

        let pass_rate = if dataset_universe > 0 {
            (datasets_covered as f64) / (dataset_universe as f64)
        } else {
            0.0
        };
        let tail_gap = (robust_score_median - robust_score_p10).max(0.0);

        if robust_score_p10 <= GROUP_GATE_MIN_ROBUST_P10 {
            dropped_gate_robust = dropped_gate_robust.saturating_add(1);
            continue;
        }
        if pass_rate < GROUP_GATE_MIN_PASS_RATE {
            dropped_gate_pass = dropped_gate_pass.saturating_add(1);
            continue;
        }
        if max_dd_p90 > GROUP_GATE_MAX_DD_P90 {
            dropped_gate_dd = dropped_gate_dd.saturating_add(1);
            continue;
        }

        let meta_score = req.weights.w_p10 * robust_score_p10
            + req.weights.w_median * robust_score_median
            + req.weights.w_cross * cross_score_median
            + req.weights.w_pass_rate * pass_rate
            - req.weights.w_dd_p90 * max_dd_p90
            + req.weights.w_psr * psr_median
            + req.weights.w_dsr * dsr_median
            - req.weights.w_tail_gap * tail_gap;

        aggregates.push(MetaCandidateAggregate {
            params: acc.params,
            key,
            ma_source: acc.ma_source,
            feature_context: acc.feature_context,
            run_context: acc.run_context,
            fast_ma_params: acc.fast_ma_params,
            slow_ma_params: acc.slow_ma_params,
            representatives,
            samples,
            datasets_covered,
            pass_rate,
            robust_score_median,
            robust_score_p10,
            tail_gap,
            max_dd_p90,
            cross_score_median,
            psr_median,
            dsr_median,
            meta_score,
            final_score: None,
            holdout: None,
            evidence_flags: Vec::new(),
            train_rows: acc.train_rows,
            holdout_rows: Vec::new(),
        });
    }

    for group in &mut aggregates {
        refresh_meta_group_evidence_flags(group);
    }

    aggregates.sort_by(|a, b| b.meta_score.total_cmp(&a.meta_score));
    if aggregates.len() > req.top_k {
        aggregates.truncate(req.top_k);
    }

    if artifacts_used == 0 {
        warnings.push("No matching run artifacts with validation results were found.".to_string());
    }
    if artifacts_rejected_by_validation_verdict > 0 {
        warnings.push(format!(
            "Validation verdict rejected {} artifact(s): no approved candidate survived backend validation.",
            artifacts_rejected_by_validation_verdict
        ));
    }
    if artifacts_rejected_by_shortlist > 0 && !req.allow_shortlist_fallback {
        warnings.push(format!(
            "Hard shortlist rejected {} artifact(s): no candidates passed strict/CSCV gates.",
            artifacts_rejected_by_shortlist
        ));
    }

    let dropped_total = dropped_gate_robust
        .saturating_add(dropped_gate_pass)
        .saturating_add(dropped_gate_dd);
    if dropped_total > 0 {
        warnings.push(format!(
            "Meta group gates filtered {dropped_total} group(s): robust_p10>{:.3} rejected={} · pass_rate>={:.2} rejected={} · dd_p90<={:.2} rejected={}.",
            GROUP_GATE_MIN_ROBUST_P10,
            dropped_gate_robust,
            GROUP_GATE_MIN_PASS_RATE,
            dropped_gate_pass,
            GROUP_GATE_MAX_DD_P90,
            dropped_gate_dd
        ));
    } else if aggregates.is_empty() {
        warnings.push(format!(
            "No candidates met min_samples={} (matched artifacts={}). Try lowering min_samples.",
            req.min_samples, artifacts_used
        ));
    }
    let groups_with_evidence_flags = aggregates
        .iter()
        .filter(|group| !group.evidence_flags.is_empty())
        .count();
    if groups_with_evidence_flags > 0 {
        warnings.push(format!(
            "Meta evidence warning: {groups_with_evidence_flags} group(s) have thin or noisy evidence."
        ));
    }

    MetaSelectionResult {
        runtime_ms: t0.elapsed().as_millis() as u64,
        strategy_kind: req.strategy_kind.clone(),
        timeframe: req.timeframe.clone(),
        artifacts_considered,
        artifacts_used,
        candidates_considered,
        groups: aggregates,
        warnings,
        holdout: None,
    }
}

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed: usize,
    total: usize,
    phase: &'static str,
) {
    if let Some(p) = progress {
        crate::progress::emit_double_ma_progress(Some(p), processed, total, phase);
    }
}

fn normalize_timeframe_label(raw: &str) -> String {
    let s0 = raw.trim().to_ascii_lowercase();
    if s0.is_empty() {
        return String::new();
    }
    // Back-compat: older artifacts may store inferred timeframes like "120s".
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

fn artifact_matches_request(art: &Value, req: &MetaSelectionRequest) -> bool {
    let kind = art.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    if kind != "vectorbt_run_artifact" {
        return false;
    }

    let strat = art
        .pointer("/run/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| {
            art.pointer("/run/req/strategyKind")
                .and_then(|v| v.as_str())
        })
        .unwrap_or("");
    if strat != req.strategy_kind {
        return false;
    }

    if let Some(tf) = req
        .timeframe
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let tf2 = art
            .pointer("/dataset_meta/timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if normalize_timeframe_label(tf2) != normalize_timeframe_label(tf) {
            return false;
        }
    }

    if let Some(tag) = req
        .required_tag
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let tags = art
            .get("tags")
            .and_then(|v| v.as_array())
            .map_or(&[][..], |xs| xs.as_slice());
        let ok = tags
            .iter()
            .filter_map(|v| v.as_str())
            .any(|t| t.trim() == tag);
        if !ok {
            return false;
        }
    }

    true
}

fn artifact_ma_source(art: &Value) -> Option<String> {
    let v = art
        .pointer("/validation/req/ma_source")
        .and_then(|v| v.as_str())
        .or_else(|| art.pointer("/run/req/ma_source").and_then(|v| v.as_str()))?;
    let s = v.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn artifact_ma_params(art: &Value, side: &str) -> Option<HashMap<String, f64>> {
    let ptr = if side == "slow" {
        "/validation/req/slow_ma_params"
    } else {
        "/validation/req/fast_ma_params"
    };
    let ptr2 = if side == "slow" {
        "/run/req/slow_ma_params"
    } else {
        "/run/req/fast_ma_params"
    };
    let v = art.pointer(ptr).or_else(|| art.pointer(ptr2))?;
    parse_ma_param_map(v)
}

fn normalize_optional_json(v: Option<&Value>) -> Option<Value> {
    match v {
        Some(Value::Null) | None => None,
        Some(x) => Some(x.clone()),
    }
}

fn artifact_feature_context(art: &Value) -> Option<Value> {
    normalize_optional_json(
        art.pointer("/validation/req/feature_context")
            .or_else(|| art.pointer("/run/req/feature_context")),
    )
}

fn artifact_run_context(art: &Value) -> Option<Value> {
    normalize_optional_json(
        art.pointer("/validation/req/run_context")
            .or_else(|| art.pointer("/run/req/run_context")),
    )
}

fn artifact_dataset_key(art: &Value, idx: usize) -> String {
    let csv = art
        .pointer("/data/csv_path")
        .and_then(|v| v.as_str())
        .or_else(|| {
            art.pointer("/dataset_meta/csv_path")
                .and_then(|v| v.as_str())
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(p) = csv {
        return format!("csv:{p}");
    }

    let label = art
        .pointer("/dataset_meta/label")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(lbl) = label {
        return format!("label:{lbl}");
    }

    format!("artifact:{idx}")
}

fn artifact_dataset_label(art: &Value, idx: usize) -> String {
    if let Some(lbl) = art
        .pointer("/dataset_meta/label")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return lbl.to_string();
    }

    if let Some(csv) = art
        .pointer("/data/csv_path")
        .and_then(|v| v.as_str())
        .or_else(|| {
            art.pointer("/dataset_meta/csv_path")
                .and_then(|v| v.as_str())
        })
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if let Some(name) = Path::new(csv).file_name().and_then(|s| s.to_str()) {
            if !name.trim().is_empty() {
                return name.to_string();
            }
        }
    }

    format!("dataset_{}", idx + 1)
}

fn parse_ma_param_map(v: &Value) -> Option<HashMap<String, f64>> {
    let obj = v.as_object()?;
    if obj.is_empty() {
        return None;
    }
    let mut out: HashMap<String, f64> = HashMap::with_capacity(obj.len());
    for (k, v) in obj {
        let x = v.as_f64()?;
        if !x.is_finite() {
            return None;
        }
        out.insert(k.clone(), x);
    }
    Some(out)
}

pub fn meta_group_key(
    params: &Value,
    ma_source: &str,
    fast_ma_params: &Option<HashMap<String, f64>>,
    slow_ma_params: &Option<HashMap<String, f64>>,
) -> String {
    meta_group_key_with_context(
        params,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        None,
        None,
    )
}

pub fn meta_group_key_with_context(
    params: &Value,
    ma_source: &str,
    fast_ma_params: &Option<HashMap<String, f64>>,
    slow_ma_params: &Option<HashMap<String, f64>>,
    feature_context: Option<&Value>,
    run_context: Option<&Value>,
) -> String {
    // Prefer strategy-agnostic context when present; otherwise keep legacy MA-based key shape.
    let v = if feature_context.is_some() || run_context.is_some() {
        serde_json::json!({
            "params": params,
            "feature_context": feature_context,
            "run_context": run_context,
        })
    } else {
        serde_json::json!({
            "params": params,
            "ma_source": ma_source,
            "fast_ma_params": fast_ma_params,
            "slow_ma_params": slow_ma_params,
        })
    };
    canonical_json(&v)
}

pub fn meta_group_key_for_request(
    req: &MetaSelectionRequest,
    params: &Value,
    ma_source: &str,
    fast_ma_params: &Option<HashMap<String, f64>>,
    slow_ma_params: &Option<HashMap<String, f64>>,
) -> String {
    meta_group_key_for_request_with_context(
        req,
        params,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        None,
        None,
    )
}

pub fn meta_group_key_for_request_with_context(
    req: &MetaSelectionRequest,
    params: &Value,
    ma_source: &str,
    fast_ma_params: &Option<HashMap<String, f64>>,
    slow_ma_params: &Option<HashMap<String, f64>>,
    feature_context: Option<&Value>,
    run_context: Option<&Value>,
) -> String {
    if req.grouping_mode == MetaGroupingMode::Exact {
        return meta_group_key_with_context(
            params,
            ma_source,
            fast_ma_params,
            slow_ma_params,
            feature_context,
            run_context,
        );
    }

    let cfg = &req.cluster_config;
    let params_sig = quantize_value(params, cfg);
    let v = if feature_context.is_some() || run_context.is_some() {
        let feature_sig = feature_context
            .map(|x| quantize_value(x, cfg))
            .unwrap_or(Value::Null);
        let run_sig = run_context
            .map(|x| quantize_value(x, cfg))
            .unwrap_or(Value::Null);
        serde_json::json!({
            "params": params_sig,
            "feature_context": feature_sig,
            "run_context": run_sig,
        })
    } else {
        let fast_sig = quantize_value(
            &serde_json::to_value(fast_ma_params).unwrap_or(Value::Null),
            cfg,
        );
        let slow_sig = quantize_value(
            &serde_json::to_value(slow_ma_params).unwrap_or(Value::Null),
            cfg,
        );
        serde_json::json!({
            "params": params_sig,
            "ma_source": ma_source,
            "fast_ma_params": fast_sig,
            "slow_ma_params": slow_sig,
        })
    };
    canonical_json(&v)
}

fn quantize_value(v: &Value, cfg: &MetaClusterConfig) -> Value {
    match v {
        Value::Null => Value::Null,
        Value::Bool(b) => Value::Bool(*b),
        Value::String(s) => Value::String(s.clone()),
        Value::Number(n) => quantize_number(n, cfg),
        Value::Array(xs) => Value::Array(xs.iter().map(|x| quantize_value(x, cfg)).collect()),
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v2) in map {
                out.insert(k.clone(), quantize_value(v2, cfg));
            }
            Value::Object(out)
        }
    }
}

fn quantize_number(n: &serde_json::Number, cfg: &MetaClusterConfig) -> Value {
    if n.is_f64() {
        let Some(x) = n.as_f64() else {
            return Value::Number(n.clone());
        };
        let b = cfg.float_bucket;
        if !b.is_finite() || b <= 0.0 {
            return Value::Number(n.clone());
        }
        let idx = (x / b).floor();
        let idx_i = if idx.is_finite() { idx as i64 } else { 0 };
        return Value::Number(serde_json::Number::from(idx_i));
    }

    let b = cfg.int_bucket.max(1);
    let i = if let Some(i) = n.as_i64() {
        i
    } else if let Some(u) = n.as_u64() {
        if u > i64::MAX as u64 {
            i64::MAX
        } else {
            u as i64
        }
    } else {
        return Value::Number(n.clone());
    };

    if b <= 1 {
        return Value::Number(serde_json::Number::from(i));
    }
    let q = i.div_euclid(b) * b;
    Value::Number(serde_json::Number::from(q))
}

fn shortlist_candidates<'a>(
    candidates: &'a [Value],
    strict: bool,
    cscv_threshold: Option<f64>,
    allow_fallback: bool,
) -> Vec<&'a Value> {
    const CSCV_RANK_FRAC_MIN: f64 = 0.55;
    const CROSS_SCORE_MIN: f64 = 0.0;
    const CROSS_PNL_POS_FRAC_MIN: f64 = 0.50;

    if !strict {
        let base: Vec<&Value> = candidates.iter().collect();
        if cscv_threshold.is_none() {
            return base;
        }
        let threshold = cscv_threshold.unwrap_or(0.0);
        let filtered: Vec<&Value> = base
            .iter()
            .copied()
            .filter(|cand| {
                if let Some(passed) = candidate_backend_validation_pass(cand) {
                    return passed;
                }
                let st = cand.get("cscv_pbo").and_then(|v| v.as_object());
                let sel = st
                    .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if sel <= 0.0 {
                    return false;
                }
                let rank = st
                    .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if rank < CSCV_RANK_FRAC_MIN {
                    return false;
                }
                let metric = st
                    .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(f64::NEG_INFINITY);
                metric >= threshold
            })
            .collect();
        return if filtered.is_empty() {
            if allow_fallback {
                base
            } else {
                Vec::new()
            }
        } else {
            filtered
        };
    }

    let has_cross = candidates
        .iter()
        .any(|c| c.get("cross_ticker").and_then(|v| v.as_object()).is_some());

    let shortlist: Vec<&Value> = candidates
        .iter()
        .filter(|cand| {
            let robust = cand
                .get("robust_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !robust {
                return false;
            }
            if !has_cross {
                return true;
            }
            let cross_pareto = cand
                .get("cross_ticker_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !cross_pareto {
                return false;
            }
            let cross_score = cand
                .pointer("/cross_ticker/score")
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::NEG_INFINITY);
            if !cross_score.is_finite() || cross_score <= CROSS_SCORE_MIN {
                return false;
            }
            let pnl_positive_frac = cand
                .pointer("/cross_ticker/pnl_positive_frac")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            pnl_positive_frac >= CROSS_PNL_POS_FRAC_MIN
        })
        .collect();

    let base: Vec<&Value> = if shortlist.is_empty() {
        if allow_fallback {
            candidates.iter().collect()
        } else {
            Vec::new()
        }
    } else {
        shortlist
    };
    if base.is_empty() {
        return base;
    }

    let Some(threshold) = cscv_threshold else {
        return base;
    };

    let filtered: Vec<&Value> = base
        .iter()
        .copied()
        .filter(|cand| {
            if let Some(passed) = candidate_backend_validation_pass(cand) {
                return passed;
            }
            let st = cand.get("cscv_pbo").and_then(|v| v.as_object());
            let sel = st
                .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            if sel <= 0.0 {
                return false;
            }
            let rank = st
                .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            if rank < CSCV_RANK_FRAC_MIN {
                return false;
            }
            let metric = st
                .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
                .unwrap_or(f64::NEG_INFINITY);
            metric >= threshold
        })
        .collect();

    if filtered.is_empty() {
        if allow_fallback {
            base
        } else {
            Vec::new()
        }
    } else {
        filtered
    }
}

fn candidate_backend_validation_pass(cand: &Value) -> Option<bool> {
    cand.pointer("/validation_verdict/passed")
        .and_then(|v| v.as_bool())
}

fn artifact_backend_validation_approved(art: &Value) -> Option<bool> {
    let verdict = art.pointer("/validation/res/run_verdict")?.as_object()?;

    if let Some(approved) = verdict.get("approved").and_then(|v| v.as_bool()) {
        return Some(approved);
    }

    if let Some(recommendation) = verdict
        .get("final_recommendation")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_ascii_lowercase())
    {
        if matches!(
            recommendation.as_str(),
            "singlecandidatesupported"
                | "approvedwithwarnings"
                | "single_candidate_supported"
                | "approved_with_warnings"
        ) {
            return Some(true);
        }
        if matches!(
            recommendation.as_str(),
            "nocandidatesurvived"
                | "nocandidateapproved"
                | "candidatessurvivedbutsearchnotstatisticallysuperior"
                | "evidencetooshortmintrackrecord"
                | "no_candidate_survived"
                | "no_candidate_approved"
                | "candidates_survived_but_search_not_statistically_superior"
                | "evidence_too_short_min_track_record"
        ) {
            return Some(false);
        }
    }

    verdict
        .get("approved_candidate_count")
        .and_then(|v| v.as_u64())
        .map(|count| count > 0)
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = quantile_index(sorted.len(), qq);
    sorted[idx.min(sorted.len() - 1)]
}

fn median(sorted: &[f64]) -> f64 {
    quantile(sorted, 0.5)
}

fn quantile_index(len: usize, q: f64) -> usize {
    if len <= 1 {
        return 0;
    }
    let qq = q.clamp(0.0, 1.0);
    (qq * (len.saturating_sub(1) as f64)).round() as usize
}

fn pick_representatives(sorted: &[(f64, Value)], max: usize) -> Vec<Value> {
    let max = max.max(1);
    if sorted.is_empty() {
        return Vec::new();
    }

    let mid = quantile_index(sorted.len(), 0.5);

    let mut indices: Vec<usize> = Vec::with_capacity(sorted.len());
    indices.push(mid);
    for d in 1..sorted.len() {
        if mid >= d {
            indices.push(mid - d);
        }
        let hi = mid + d;
        if hi < sorted.len() {
            indices.push(hi);
        }
        if indices.len() >= sorted.len() {
            break;
        }
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<Value> = Vec::new();
    for idx in indices {
        let Some((_, p)) = sorted.get(idx) else {
            continue;
        };
        let sig = canonical_json(p);
        if !seen.insert(sig) {
            continue;
        }
        out.push(p.clone());
        if out.len() >= max {
            break;
        }
    }

    if out.is_empty() {
        out.push(sorted[mid].1.clone());
    }

    out
}

fn canonical_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(xs) => {
            let mut out = String::from("[");
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(x));
            }
            out.push(']');
            out
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            let mut out = String::from("{");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                let v = map.get(*k).unwrap_or(&Value::Null);
                out.push_str(&canonical_json(v));
            }
            out.push('}');
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_json_is_order_invariant_for_objects() {
        let a = json!({"b": 1, "a": 2, "c": {"y": 1, "x": 2}});
        let b = json!({"c": {"x": 2, "y": 1}, "a": 2, "b": 1});
        assert_eq!(canonical_json(&a), canonical_json(&b));
    }

    #[test]
    fn default_shortlist_fallback_is_hard_mode() {
        assert!(!default_allow_shortlist_fallback());
    }

    #[test]
    fn meta_select_groups_and_scores() {
        let params = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": 5,
            "slow_len": 20,
            "halftrend": {"amplitude": 2, "atr_period": 100, "channel_deviation": 2.0},
            "macz": {},
            "ott": {},
            "strategy": {}
        });

        let art1 = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "AAPL", "timeframe": "1m", "csv_path": "aapl.csv" },
            "tags": ["u1"],
            "validation": { "res": { "candidates": [
                { "params": params, "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.20, "trades": 10, "exposure": 0.5 }, "robust_score": 1.1, "robust_pareto": true }
            ]}}
        });

        let art2 = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:01:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "MSFT", "timeframe": "1m", "csv_path": "msft.csv" },
            "tags": ["u1"],
            "validation": { "res": { "candidates": [
                { "params": params, "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.10, "trades": 10, "exposure": 0.5 }, "robust_score": 2.2, "robust_pareto": true }
            ]}}
        });

        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: Some("u1".to_string()),
            holdout_tag: None,
            eval_holdout: true,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 2,
            strict_shortlist: true,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(&req, &[art1, art2], None);
        assert_eq!(res.artifacts_used, 2);
        assert_eq!(res.groups.len(), 1);

        let g = &res.groups[0];
        assert_eq!(g.samples, 2);
        assert!((g.pass_rate - 1.0).abs() < 1e-9);
        assert!(g.meta_score.is_finite());
        assert!(g.robust_score_p10 > 0.0);
        assert_eq!(g.train_rows.len(), 2);
        assert!(g
            .train_rows
            .iter()
            .any(|r| r.dataset.eq_ignore_ascii_case("AAPL")));
        assert!(g
            .train_rows
            .iter()
            .any(|r| r.dataset.eq_ignore_ascii_case("MSFT")));
    }

    #[test]
    fn strict_shortlist_respects_cross_pareto_when_present() {
        let params = json!({"fast_ma_type":"sma","slow_ma_type":"sma","fast_len":5,"slow_len":20,"halftrend":{"amplitude":2,"atr_period":100,"channel_deviation":2.0},"macz":{},"ott":{},"strategy":{}});
        let art = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "AAPL", "timeframe": "1m", "csv_path": "aapl.csv" },
            "tags": [],
            "validation": { "res": { "candidates": [
                { "params": params, "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.20, "trades": 10, "exposure": 0.5 }, "robust_score": 1.1, "robust_pareto": true, "cross_ticker": { "score": 0.5 }, "cross_ticker_pareto": false }
            ]}}
        });

        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: None,
            holdout_tag: None,
            eval_holdout: true,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: true,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(&req, &[art], None);

        // Strict shortlist would exclude, but we intentionally fall back to all candidates
        // to avoid producing an empty meta run.
        assert_eq!(res.groups.len(), 1);
        assert_eq!(res.groups[0].samples, 1);
    }

    #[test]
    fn strict_shortlist_hard_mode_excludes_when_cross_pareto_missing() {
        let params = json!({"fast_ma_type":"sma","slow_ma_type":"sma","fast_len":5,"slow_len":20,"halftrend":{"amplitude":2,"atr_period":100,"channel_deviation":2.0},"macz":{},"ott":{},"strategy":{}});
        let art = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "AAPL", "timeframe": "1m", "csv_path": "aapl.csv" },
            "tags": [],
            "validation": { "res": { "candidates": [
                { "params": params, "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.20, "trades": 10, "exposure": 0.5 }, "robust_score": 1.1, "robust_pareto": true, "cross_ticker": { "score": 0.5 }, "cross_ticker_pareto": false }
            ]}}
        });

        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: None,
            holdout_tag: None,
            eval_holdout: true,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: true,
            allow_shortlist_fallback: false,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(&req, &[art], None);
        assert_eq!(res.groups.len(), 0);
        assert!(res
            .warnings
            .iter()
            .any(|w| w.contains("Hard shortlist rejected")));
    }

    #[test]
    fn strict_shortlist_hard_mode_excludes_when_cross_score_non_positive() {
        let params = json!({"fast_ma_type":"sma","slow_ma_type":"sma","fast_len":5,"slow_len":20,"halftrend":{"amplitude":2,"atr_period":100,"channel_deviation":2.0},"macz":{},"ott":{},"strategy":{}});
        let art = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "AAPL", "timeframe": "1m", "csv_path": "aapl.csv" },
            "tags": [],
            "validation": { "res": { "candidates": [
                {
                    "params": params,
                    "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.20, "trades": 10, "exposure": 0.5 },
                    "robust_score": 1.1,
                    "robust_pareto": true,
                    "cross_ticker": { "score": 0.0, "pnl_positive_frac": 1.0 },
                    "cross_ticker_pareto": true
                }
            ]}}
        });

        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: None,
            holdout_tag: None,
            eval_holdout: true,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: true,
            allow_shortlist_fallback: false,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(&req, &[art], None);
        assert_eq!(res.groups.len(), 0);
        assert!(res
            .warnings
            .iter()
            .any(|w| w.contains("Hard shortlist rejected")));
    }

    #[test]
    fn meta_select_rejects_unapproved_validation_artifacts() {
        let params = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": 5,
            "slow_len": 20,
            "halftrend": {"amplitude": 2, "atr_period": 100, "channel_deviation": 2.0},
            "macz": {},
            "ott": {},
            "strategy": {}
        });

        let art = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "AAPL", "timeframe": "1m", "csv_path": "aapl.csv" },
            "validation": { "res": {
                "run_verdict": {
                    "approved": false,
                    "final_recommendation": "NoCandidateApproved"
                },
                "candidates": [
                    {
                        "params": params,
                        "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.20, "trades": 10, "exposure": 0.5 },
                        "robust_score": 1.1,
                        "robust_pareto": true,
                        "validation_verdict": { "passed": true }
                    }
                ]
            }}
        });

        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: None,
            holdout_tag: None,
            eval_holdout: false,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: true,
            allow_shortlist_fallback: false,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(&req, &[art], None);
        assert_eq!(res.artifacts_used, 0);
        assert!(res.groups.is_empty());
        assert!(res
            .warnings
            .iter()
            .any(|w| w.contains("Validation verdict rejected")));
    }

    #[test]
    fn meta_select_populates_evidence_flags_for_thin_groups() {
        let params = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": 5,
            "slow_len": 20,
            "halftrend": {"amplitude": 2, "atr_period": 100, "channel_deviation": 2.0},
            "macz": {},
            "ott": {},
            "strategy": {}
        });

        let art = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "AAPL", "timeframe": "1m", "csv_path": "aapl.csv" },
            "validation": { "res": {
                "candidates": [
                    {
                        "params": params,
                        "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.20, "trades": 10, "exposure": 0.5 },
                        "robust_score": 1.1,
                        "robust_pareto": true
                    }
                ]
            }}
        });

        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: None,
            holdout_tag: None,
            eval_holdout: false,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: true,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(&req, &[art], None);
        assert_eq!(res.groups.len(), 1);
        assert!(!res.groups[0].evidence_flags.is_empty());
        assert!(res
            .warnings
            .iter()
            .any(|w| w.contains("thin or noisy evidence")));
    }

    #[test]
    fn meta_select_keeps_legacy_validation_artifacts_compatible_without_run_verdict() {
        let params = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": 5,
            "slow_len": 20,
            "halftrend": {"amplitude": 2, "atr_period": 100, "channel_deviation": 2.0},
            "macz": {},
            "ott": {},
            "strategy": {}
        });

        let art = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "AAPL", "timeframe": "1m", "csv_path": "aapl.csv" },
            "validation": { "res": {
                "candidates": [
                    {
                        "params": params,
                        "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.20, "trades": 10, "exposure": 0.5 },
                        "robust_score": 1.1,
                        "robust_pareto": true
                    }
                ]
            }}
        });

        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: None,
            holdout_tag: None,
            eval_holdout: false,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: true,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(&req, &[art], None);
        assert_eq!(res.artifacts_used, 1);
        assert_eq!(res.groups.len(), 1);
        assert!(res
            .warnings
            .iter()
            .all(|w| !w.contains("Validation verdict rejected")));
    }

    #[test]
    fn clustered_grouping_buckets_similar_params() {
        let p1 = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": 6,
            "slow_len": 21,
            "halftrend": {"amplitude": 2, "atr_period": 99, "channel_deviation": 2.10},
            "macz": {},
            "ott": {},
            "strategy": {}
        });
        let p2 = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": 9,
            "slow_len": 24,
            "halftrend": {"amplitude": 3, "atr_period": 97, "channel_deviation": 2.24},
            "macz": {},
            "ott": {},
            "strategy": {}
        });

        let art1 = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "AAPL", "timeframe": "1m", "csv_path": "aapl.csv" },
            "tags": ["u1"],
            "validation": { "res": { "candidates": [
                { "params": p1, "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.20, "trades": 10, "exposure": 0.5 }, "robust_score": 1.1, "robust_pareto": true }
            ]}}
        });

        let art2 = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-01-26T00:01:00Z",
            "run": { "req": { "strategy_kind": "CompositeMo" }, "res": {} },
            "dataset_meta": { "label": "MSFT", "timeframe": "1m", "csv_path": "msft.csv" },
            "tags": ["u1"],
            "validation": { "res": { "candidates": [
                { "params": p2, "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.10, "trades": 10, "exposure": 0.5 }, "robust_score": 2.2, "robust_pareto": true }
            ]}}
        });

        let exact_req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: Some("u1".to_string()),
            holdout_tag: None,
            eval_holdout: true,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: true,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };
        let exact = meta_select_from_artifacts(&exact_req, &[art1.clone(), art2.clone()], None);
        assert_eq!(exact.groups.len(), 2);

        let clustered_req = MetaSelectionRequest {
            grouping_mode: MetaGroupingMode::Clustered,
            cluster_config: MetaClusterConfig {
                int_bucket: 5,
                float_bucket: 0.25,
            },
            ..exact_req
        };
        let clustered = meta_select_from_artifacts(&clustered_req, &[art1, art2], None);

        // With int buckets of 5 and float buckets of 0.25, these params should collapse into a single region.
        assert_eq!(clustered.groups.len(), 1);
        assert_eq!(clustered.groups[0].samples, 2);
        assert!(clustered.groups[0].representatives.len() >= 2);
    }

    #[test]
    fn back_compat_old_artifact_without_generic_context_uses_legacy_ma_grouping() {
        let params = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "sma",
            "fast_len": 5,
            "slow_len": 20
        });
        let mk_art = |label: &str, ma_source: &str| {
            json!({
                "kind": "vectorbt_run_artifact",
                "run": { "req": { "strategy_kind": "CompositeMo", "ma_source": ma_source }, "res": {} },
                "dataset_meta": { "label": label, "timeframe": "1m", "csv_path": format!("{label}.csv") },
                "validation": {
                    "req": { "ma_source": ma_source },
                    "res": { "candidates": [
                        {
                            "params": params,
                            "in_sample": { "pnl": 1.0, "sharpe": 1.8, "max_dd": 0.20, "trades": 10, "exposure": 0.5 },
                            "robust_score": 1.1,
                            "robust_pareto": true
                        }
                    ]}
                }
            })
        };
        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: None,
            holdout_tag: None,
            eval_holdout: false,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: false,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(
            &req,
            &[mk_art("AAPL", "close"), mk_art("MSFT", "hl2")],
            None,
        );
        // Legacy MA context is still part of the grouping key when generic context is absent.
        assert_eq!(res.groups.len(), 2);
    }

    #[test]
    fn forward_compat_generic_context_groups_without_legacy_ma_fields() {
        let params = json!({
            "alpha": 7,
            "beta": 13
        });
        let mk_art = |label: &str| {
            json!({
                "kind": "vectorbt_run_artifact",
                "run": { "req": { "strategy_kind": "CompositeMo", "feature_context": { "family": "generic" }, "run_context": { "variant": "v1" } }, "res": {} },
                "dataset_meta": { "label": label, "timeframe": "1m", "csv_path": format!("{label}.csv") },
                "validation": {
                    "req": { "feature_context": { "family": "generic" }, "run_context": { "variant": "v1" } },
                    "res": { "candidates": [
                        {
                            "params": params,
                            "in_sample": { "pnl": 1.0, "sharpe": 1.8, "max_dd": 0.20, "trades": 10, "exposure": 0.5 },
                            "robust_score": 1.1,
                            "robust_pareto": true
                        }
                    ]}
                }
            })
        };
        let req = MetaSelectionRequest {
            strategy_kind: "CompositeMo".to_string(),
            timeframe: Some("1m".to_string()),
            required_tag: None,
            holdout_tag: None,
            eval_holdout: false,
            save_holdout_rows: false,
            holdout_eval_top_n: None,
            holdout_eval_explore_n: 0,
            holdout_eval_rank_by: None,
            holdout_mode: MetaHoldoutMode::Light,
            holdout_validation: None,
            top_k: 10,
            min_samples: 1,
            strict_shortlist: false,
            allow_shortlist_fallback: true,
            weights: MetaScoreWeights::default(),
            grouping_mode: MetaGroupingMode::Exact,
            cluster_config: MetaClusterConfig::default(),
            cluster_representatives: 2,
        };

        let res = meta_select_from_artifacts(&req, &[mk_art("AAPL"), mk_art("MSFT")], None);
        assert_eq!(res.groups.len(), 1);
        assert!(res.groups[0].feature_context.is_some());
        assert!(res.groups[0].run_context.is_some());
    }
}
