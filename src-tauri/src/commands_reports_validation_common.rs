use serde_json::{Map, Value};

const CSCV_RANK_FRAC_MIN: f64 = 0.55;
const ACTIVE_PROFILE_KEYS: [&str; 9] = [
    "asr", "vcc", "vemr", "rocb", "mkcs", "kqsr", "carm", "kvdf", "cdmc",
];
const AUX_PROFILE_KEYS: [&str; 4] = ["halftrend", "halftrend_filter", "macz", "ott"];

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ExportRunVerdict {
    pub passed: bool,
    pub approved: bool,
    pub candidates_tested: usize,
    pub candidates_survived_hard_gates: usize,
    pub approved_candidate_count: usize,
    pub final_recommendation: String,
    pub warning_flags: String,
    pub fail_reasons: String,
    pub first_approved_rank: Option<usize>,
    pub top_candidate_track_record_bars: Option<usize>,
    pub top_candidate_min_track_record_length_bars: Option<usize>,
    pub top_candidate_psr: Option<f64>,
    pub top_candidate_dsr: Option<f64>,
    pub pbo: Option<f64>,
    pub max_pbo: Option<f64>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ExportSpaSummary {
    pub scope: String,
    pub candidate_count: usize,
    pub slice_count: usize,
    pub bootstrap_samples: usize,
    pub p_value: Option<f64>,
    pub passed: Option<bool>,
    pub skipped_reason: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ExportReoptSummary {
    pub windows: usize,
    pub top_n_candidates: usize,
    pub unique_winners: usize,
    pub winner_changes: usize,
    pub dominant_winner_frac: Option<f64>,
    pub winner_param_drift_mean: Option<f64>,
    pub winner_param_drift_max: Option<f64>,
    pub forward_positive_pnl_frac: Option<f64>,
    pub forward_sharpe_median: Option<f64>,
    pub forward_max_dd_worst: Option<f64>,
    pub wfe_pnl_median: Option<f64>,
    pub wfe_pnl_worst: Option<f64>,
    pub wfe_sharpe_median: Option<f64>,
    pub wfe_sharpe_worst: Option<f64>,
    pub wfe_sharpe_std: Option<f64>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ExportCscvSummary {
    pub slices: usize,
    pub slice_bars: usize,
    pub candidates: usize,
    pub combinations: usize,
    pub pbo: Option<f64>,
    pub threshold: Option<f64>,
    pub prob_oos_below_threshold: Option<f64>,
    pub skipped_reason: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ExportCrossSummary {
    pub dataset_count: usize,
    pub mode: String,
    pub dataset_labels: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ExportRankingStabilitySummary {
    pub finalist_k: usize,
    pub stable_finalist_count: usize,
    pub wf_overlap: Option<usize>,
    pub wf_mean_drift: Option<f64>,
    pub cscv_overlap: Option<usize>,
    pub cscv_mean_drift: Option<f64>,
    pub cross_overlap: Option<usize>,
    pub cross_mean_drift: Option<f64>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ExportRegimeBucketRow {
    pub rank: usize,
    pub approval_label: String,
    pub params_summary: String,
    pub label: String,
    pub bars: Option<usize>,
    pub active_frac: Option<f64>,
    pub pnl: Option<f64>,
    pub sharpe: Option<f64>,
    pub max_dd: Option<f64>,
    pub trades: Option<usize>,
    pub stable: Option<bool>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ExportCandidateRow {
    pub rank: usize,
    pub final_approved: bool,
    pub hard_gate_passed: Option<bool>,
    pub legacy_shortlisted: bool,
    pub approval_label: String,
    pub approval_reason: String,
    pub profile_kind: String,
    pub params_summary: String,
    pub params_json: String,
    pub robust_score: f64,
    pub robust_pareto: bool,
    pub psr: Option<f64>,
    pub dsr: Option<f64>,
    pub track_record_bars: Option<usize>,
    pub min_track_record_length_bars: Option<usize>,
    pub pnl: f64,
    pub sharpe: f64,
    pub max_dd: f64,
    pub trades: f64,
    pub exposure: Option<f64>,
    pub wf_sharpe_median: Option<f64>,
    pub wf_dd_worst: Option<f64>,
    pub wf_pnl_efficiency: Option<f64>,
    pub wf_sharpe_efficiency: Option<f64>,
    pub cross_score: Option<f64>,
    pub cross_pareto: bool,
    pub cscv_selected_frac: Option<f64>,
    pub cscv_oos_rank_frac_p50: Option<f64>,
    pub cscv_oos_metric_p50: Option<f64>,
    pub bootstrap_prob_pnl_positive: Option<f64>,
    pub bootstrap_pnl_p05: Option<f64>,
    pub bootstrap_pnl_p50: Option<f64>,
    pub bootstrap_dd_p95: Option<f64>,
    pub bootstrap_stationary_mean_block_len: Option<usize>,
    pub bootstrap_stationary_pnl_p05: Option<f64>,
    pub bootstrap_stationary_dd_p95: Option<f64>,
    pub perturb_pnl_worst: Option<f64>,
    pub perturb_pnl_mean: Option<f64>,
    pub perturb_positive_frac: Option<f64>,
    pub plateau_score: Option<f64>,
    pub plateau_label: String,
    pub plateau_source: String,
    pub plateau_neighbor_count: Option<usize>,
    pub plateau_sharpe_median: Option<f64>,
    pub plateau_max_dd_worst: Option<f64>,
    pub regime_worst_bucket: String,
    pub regime_worst_sharpe: Option<f64>,
    pub regime_worst_max_dd: Option<f64>,
    pub regime_stable_bucket_count: Option<usize>,
    pub regime_stable_bucket_frac: Option<f64>,
    pub regime_fragility_flag: String,
    pub regime_support_label: String,
    pub regime_bucket_rows: Vec<ExportRegimeBucketRow>,
    pub friction_flag: String,
    pub friction_worst_scenario: String,
    pub friction_worst_sharpe: Option<f64>,
    pub friction_worst_max_dd: Option<f64>,
    pub friction_pass_frac: Option<f64>,
    pub friction_max_rank_drift: Option<usize>,
    pub robust_view_rank: Option<usize>,
    pub wf_view_rank: Option<usize>,
    pub cscv_view_rank: Option<usize>,
    pub cross_view_rank: Option<usize>,
    pub max_view_rank_drift: Option<usize>,
    pub warning_flags: String,
    pub fail_reasons: String,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ValidationExportModel {
    pub ts: String,
    pub data_id: String,
    pub strategy_kind: String,
    pub runtime_ms: Option<u64>,
    pub backend_used: String,
    pub backend_route: String,
    pub backend_fallback_reason: String,
    pub actual_aux_tail_summary: String,
    pub expected_route_summary: String,
    pub expected_cross_summary: String,
    pub expected_module_summary: String,
    pub expected_warning_summary: String,
    pub bootstrap_mode_summary: String,
    pub wfe_policy_summary: String,
    pub friction_policy_summary: String,
    pub compatibility_mode: bool,
    pub visible_cap: usize,
    pub min_psr: Option<f64>,
    pub min_dsr: Option<f64>,
    pub min_cscv_oos_rank_frac: Option<f64>,
    pub min_cscv_oos_metric: Option<f64>,
    pub max_pbo: Option<f64>,
    pub run_verdict: Option<ExportRunVerdict>,
    pub spa: Option<ExportSpaSummary>,
    pub reopt_walk_forward: Option<ExportReoptSummary>,
    pub cscv_pbo: Option<ExportCscvSummary>,
    pub cross_ticker: Option<ExportCrossSummary>,
    pub ranking_stability: Option<ExportRankingStabilitySummary>,
    pub all_rows: Vec<ExportCandidateRow>,
    pub approved_rows: Vec<ExportCandidateRow>,
    pub shortlist_rows: Vec<ExportCandidateRow>,
}

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or("").trim().to_string()
}

fn opt_f64(v: &Value, ptr: &str) -> Option<f64> {
    v.pointer(ptr)
        .and_then(|x| x.as_f64())
        .filter(|x| x.is_finite())
}

fn opt_u64(v: &Value, ptr: &str) -> Option<u64> {
    v.pointer(ptr).and_then(|x| x.as_u64())
}

fn derive_friction_flag(pass_frac: Option<f64>, max_rank_drift: Option<usize>) -> String {
    match (pass_frac, max_rank_drift) {
        (Some(pass_frac), Some(max_rank_drift)) if pass_frac < 1.0 || max_rank_drift >= 2 => {
            "fragile".to_string()
        }
        (Some(_), Some(_)) => "stable".to_string(),
        _ => String::new(),
    }
}

fn derive_regime_fragility_flag(
    stable_bucket_count: Option<usize>,
    worst_sharpe: Option<f64>,
    worst_max_dd: Option<f64>,
) -> String {
    match (stable_bucket_count, worst_sharpe, worst_max_dd) {
        (Some(stable), Some(worst_sharpe), Some(worst_max_dd))
            if stable >= 4 && worst_sharpe >= 0.0 && worst_max_dd <= 0.35 =>
        {
            "broad".to_string()
        }
        (Some(stable), Some(worst_sharpe), Some(worst_max_dd))
            if stable >= 3 && worst_sharpe >= -0.10 && worst_max_dd <= 0.45 =>
        {
            "mixed".to_string()
        }
        (Some(stable), Some(worst_sharpe), _) if stable >= 2 && worst_sharpe >= -0.25 => {
            "narrow".to_string()
        }
        (Some(_), Some(_), Some(_)) => "fragile".to_string(),
        _ => String::new(),
    }
}

fn derive_regime_support_label(
    trend_stable: Option<bool>,
    range_stable: Option<bool>,
    stable_bucket_count: Option<usize>,
) -> String {
    match (trend_stable, range_stable, stable_bucket_count) {
        (Some(true), _, _) => "trend_ok".to_string(),
        (Some(false), Some(true), Some(stable)) if stable >= 3 => "mixed".to_string(),
        (Some(false), Some(true), _) => "range_only".to_string(),
        (Some(false), _, Some(stable)) if stable >= 3 => "mixed".to_string(),
        (Some(false), _, _) => "weak".to_string(),
        _ => String::new(),
    }
}

fn bucket_stable(pnl: Option<f64>, sharpe: Option<f64>) -> Option<bool> {
    match (pnl, sharpe) {
        (Some(pnl), Some(sharpe)) if pnl.is_finite() && sharpe.is_finite() => {
            Some(pnl >= 0.0 && sharpe >= 0.0)
        }
        _ => None,
    }
}

fn parse_regime_bucket_rows(cand: &Value) -> Vec<ExportRegimeBucketRow> {
    let Some(regime) = cand.get("regime_slice_robustness").and_then(Value::as_object) else {
        return Vec::new();
    };
    let labels = [
        ("low_vol", "low_vol"),
        ("mid_vol", "mid_vol"),
        ("high_vol", "high_vol"),
        ("trend", "trend"),
        ("range", "range"),
    ];
    labels
        .iter()
        .filter_map(|(key, label)| {
            let bucket = regime.get(*key)?.as_object()?;
            let pnl = bucket.get("pnl").and_then(Value::as_f64).filter(|v| v.is_finite());
            let sharpe = bucket
                .get("sharpe")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite());
            Some(ExportRegimeBucketRow {
                rank: 0,
                approval_label: String::new(),
                params_summary: String::new(),
                label: (*label).to_string(),
                bars: bucket.get("bars").and_then(Value::as_u64).map(|v| v as usize),
                active_frac: bucket
                    .get("active_frac")
                    .and_then(Value::as_f64)
                    .filter(|v| v.is_finite()),
                pnl,
                sharpe,
                max_dd: bucket
                    .get("max_dd")
                    .and_then(Value::as_f64)
                    .filter(|v| v.is_finite()),
                trades: bucket.get("trades").and_then(Value::as_u64).map(|v| v as usize),
                stable: bucket_stable(pnl, sharpe),
            })
        })
        .collect()
}

fn normalize_regime_support_label(label: &str) -> String {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn build_bootstrap_mode_summary(validation_req: Option<&Value>) -> String {
    let Some(trade_bootstrap) = validation_req
        .and_then(|value| value.get("trade_bootstrap"))
        .filter(|value| !value.is_null())
    else {
        return "-".to_string();
    };
    if trade_bootstrap
        .get("stationary_mean_block_len")
        .and_then(Value::as_u64)
        == Some(0)
    {
        return "iid-only closed-trade bootstrap".to_string();
    }
    let stat_block = trade_bootstrap
        .get("stationary_mean_block_len")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "auto".to_string());
    format!("iid + stationary closed-trade comparison (stat block={stat_block})")
}

fn join_string_array(v: Option<&Value>) -> String {
    let Some(xs) = v.and_then(|value| value.as_array()) else {
        return String::new();
    };
    xs.iter()
        .filter_map(|entry| entry.as_str().map(str::trim))
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn scalar_text(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        let trimmed = s.trim();
        if trimmed.len() > 40 {
            format!("{}...", &trimmed[..37])
        } else {
            trimmed.to_string()
        }
    } else if let Some(b) = value.as_bool() {
        if b {
            "true".to_string()
        } else {
            "false".to_string()
        }
    } else if let Some(n) = value.as_i64() {
        n.to_string()
    } else if let Some(n) = value.as_u64() {
        n.to_string()
    } else if let Some(n) = value.as_f64() {
        if n.fract().abs() < 1e-9 {
            format!("{n:.0}")
        } else {
            format!("{n:.4}")
        }
    } else {
        String::new()
    }
}

fn sorted_object_entries(obj: &Map<String, Value>) -> Vec<(&str, &Value)> {
    let mut entries: Vec<(&str, &Value)> = obj.iter().map(|(k, v)| (k.as_str(), v)).collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
}

fn summarize_object(obj: &Map<String, Value>, max_fields: usize) -> String {
    let parts: Vec<String> = sorted_object_entries(obj)
        .into_iter()
        .filter_map(|(key, value)| {
            let rendered = scalar_text(value);
            if rendered.is_empty() {
                None
            } else {
                Some(format!("{key}={rendered}"))
            }
        })
        .take(max_fields)
        .collect();
    parts.join(", ")
}

fn active_profile_kind(params: &Map<String, Value>) -> String {
    for key in ACTIVE_PROFILE_KEYS {
        if params.get(key).and_then(Value::as_object).is_some() {
            return key.to_ascii_uppercase();
        }
    }
    if params.get("halftrend").and_then(Value::as_object).is_some()
        || params
            .get("halftrend_filter")
            .and_then(Value::as_object)
            .is_some()
    {
        return "HALFTREND".to_string();
    }
    if params.get("fast_ma_type").is_some() || params.get("slow_ma_type").is_some() {
        return "DOUBLE_MA".to_string();
    }
    "GENERIC".to_string()
}

fn params_summary(params: &Map<String, Value>) -> String {
    let mut parts: Vec<String> = Vec::new();
    let fast_ma = params
        .get("fast_ma_type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let slow_ma = params
        .get("slow_ma_type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let fast_len = params.get("fast_len").and_then(Value::as_u64).unwrap_or(0);
    let slow_len = params.get("slow_len").and_then(Value::as_u64).unwrap_or(0);
    if !fast_ma.is_empty() || !slow_ma.is_empty() {
        parts.push(format!(
            "{}({}) / {}({})",
            if fast_ma.is_empty() { "?" } else { fast_ma },
            if fast_len == 0 {
                "?".to_string()
            } else {
                fast_len.to_string()
            },
            if slow_ma.is_empty() { "?" } else { slow_ma },
            if slow_len == 0 {
                "?".to_string()
            } else {
                slow_len.to_string()
            }
        ));
    }

    let mut profile_parts: Vec<String> = Vec::new();
    for key in ACTIVE_PROFILE_KEYS {
        if let Some(obj) = params.get(key).and_then(Value::as_object) {
            let rendered = summarize_object(obj, 3);
            if rendered.is_empty() {
                profile_parts.push(key.to_ascii_uppercase());
            } else {
                profile_parts.push(format!("{}({rendered})", key.to_ascii_uppercase()));
            }
        }
    }
    for key in AUX_PROFILE_KEYS {
        if params.get(key).and_then(Value::as_object).is_some() {
            profile_parts.push(key.to_ascii_uppercase());
        }
    }
    if let Some(strategy) = params.get("strategy").and_then(Value::as_object) {
        let rendered = summarize_object(strategy, 3);
        if !rendered.is_empty() {
            profile_parts.push(format!("strategy({rendered})"));
        }
    }
    if !profile_parts.is_empty() {
        parts.push(profile_parts.join(" + "));
    }

    if parts.is_empty() {
        let fallback = summarize_object(params, 4);
        if fallback.is_empty() {
            "params".to_string()
        } else {
            fallback
        }
    } else {
        parts.join(" | ")
    }
}

fn legacy_shortlist_flags(candidates: &[&Value]) -> Vec<bool> {
    let has_cross = candidates.iter().any(|cand| {
        cand.get("cross_ticker")
            .and_then(Value::as_object)
            .is_some()
    });

    let strict: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter_map(|(idx, cand)| {
            let robust = cand
                .get("robust_pareto")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !robust {
                return None;
            }
            if !has_cross {
                return Some(idx);
            }
            cand.get("cross_ticker_pareto")
                .and_then(Value::as_bool)
                .filter(|passed| *passed)
                .map(|_| idx)
        })
        .collect();

    let cscv_threshold = 0.0;
    let cscv_present = candidates
        .iter()
        .any(|cand| cand.get("cscv_pbo").and_then(Value::as_object).is_some());

    let shortlist: Vec<usize> = if cscv_present {
        let filtered: Vec<usize> = strict
            .iter()
            .copied()
            .filter(|idx| {
                let Some(stats) = candidates[*idx].get("cscv_pbo").and_then(Value::as_object)
                else {
                    return false;
                };
                let selected_frac = stats
                    .get("is_selected_frac")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let rank = stats
                    .get("oos_rank_frac_p50")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let metric = stats
                    .get("oos_metric_p50")
                    .and_then(Value::as_f64)
                    .unwrap_or(f64::NEG_INFINITY);
                selected_frac > 0.0 && rank >= CSCV_RANK_FRAC_MIN && metric >= cscv_threshold
            })
            .collect();
        if filtered.is_empty() {
            if strict.is_empty() {
                (0..candidates.len()).collect()
            } else {
                strict.clone()
            }
        } else {
            filtered
        }
    } else if strict.is_empty() {
        (0..candidates.len()).collect()
    } else {
        strict.clone()
    };

    let mut flags = vec![false; candidates.len()];
    for idx in shortlist {
        if let Some(slot) = flags.get_mut(idx) {
            *slot = true;
        }
    }
    flags
}

fn final_approved(
    hard_gate_passed: Option<bool>,
    track_record_bars: Option<usize>,
    min_track_record_length_bars: Option<usize>,
) -> bool {
    let Some(true) = hard_gate_passed else {
        return false;
    };
    let Some(mintrl) = min_track_record_length_bars else {
        return false;
    };
    let observed = track_record_bars.unwrap_or(0).max(2);
    observed >= mintrl
}

fn approval_label_and_reason(
    compatibility_mode: bool,
    approved: bool,
    hard_gate_passed: Option<bool>,
    legacy_shortlisted: bool,
    track_record_bars: Option<usize>,
    min_track_record_length_bars: Option<usize>,
    fail_reasons: &str,
    warning_flags: &str,
) -> (String, String) {
    if compatibility_mode && hard_gate_passed.is_none() {
        if legacy_shortlisted {
            return (
                "compat_shortlist".to_string(),
                "legacy shortlist compatibility mode".to_string(),
            );
        }
        return (
            "compat_context".to_string(),
            "candidate shown for legacy compatibility".to_string(),
        );
    }
    if approved {
        return ("approved".to_string(), "approved".to_string());
    }
    if let Some(false) = hard_gate_passed {
        let reason = if fail_reasons.is_empty() {
            "hard gate failed".to_string()
        } else {
            fail_reasons.to_string()
        };
        return ("rejected".to_string(), reason);
    }
    if let Some(mintrl) = min_track_record_length_bars {
        let observed = track_record_bars.unwrap_or(0).max(2);
        if observed < mintrl {
            return (
                "not_approved".to_string(),
                format!("track record {observed}/{mintrl}"),
            );
        }
    }
    if !warning_flags.is_empty() {
        return ("review".to_string(), warning_flags.to_string());
    }
    ("review".to_string(), "not approved".to_string())
}

fn parse_run_verdict(validation_res: &Value) -> Option<ExportRunVerdict> {
    let verdict = validation_res.get("run_verdict")?.as_object()?;
    Some(ExportRunVerdict {
        passed: verdict
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        approved: verdict
            .get("approved")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        candidates_tested: verdict
            .get("candidates_tested")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        candidates_survived_hard_gates: verdict
            .get("candidates_survived_hard_gates")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        approved_candidate_count: verdict
            .get("approved_candidate_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        final_recommendation: verdict
            .get("final_recommendation")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_string(),
        warning_flags: join_string_array(verdict.get("warning_flags")),
        fail_reasons: join_string_array(verdict.get("fail_reasons")),
        first_approved_rank: verdict
            .get("first_approved_rank")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
        top_candidate_track_record_bars: verdict
            .get("top_candidate_track_record_bars")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
        top_candidate_min_track_record_length_bars: verdict
            .get("top_candidate_min_track_record_length_bars")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
        top_candidate_psr: verdict
            .get("top_candidate_psr")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        top_candidate_dsr: verdict
            .get("top_candidate_dsr")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        pbo: verdict
            .get("pbo")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        max_pbo: verdict
            .get("max_pbo")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
    })
}

fn parse_spa_summary(validation_res: &Value) -> Option<ExportSpaSummary> {
    let spa = validation_res.get("spa")?.as_object()?;
    Some(ExportSpaSummary {
        scope: normalize_text(spa.get("scope").and_then(Value::as_str)),
        candidate_count: spa
            .get("candidate_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        slice_count: spa.get("slice_count").and_then(Value::as_u64).unwrap_or(0) as usize,
        bootstrap_samples: spa
            .get("bootstrap_samples")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        p_value: spa
            .get("p_value")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        passed: spa.get("passed").and_then(Value::as_bool),
        skipped_reason: normalize_text(spa.get("skipped_reason").and_then(Value::as_str)),
    })
}

fn parse_reopt_summary(validation_res: &Value) -> Option<ExportReoptSummary> {
    let stats = validation_res.get("reopt_walk_forward")?.as_object()?;
    Some(ExportReoptSummary {
        windows: stats.get("windows").and_then(Value::as_u64).unwrap_or(0) as usize,
        top_n_candidates: stats
            .get("top_n_candidates")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        unique_winners: stats
            .get("unique_winners")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        winner_changes: stats
            .get("winner_changes")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        dominant_winner_frac: stats
            .get("dominant_winner_frac")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        winner_param_drift_mean: stats
            .get("winner_param_drift_mean")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        winner_param_drift_max: stats
            .get("winner_param_drift_max")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        forward_positive_pnl_frac: stats
            .get("forward_positive_pnl_frac")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        forward_sharpe_median: stats
            .get("forward_sharpe_median")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        forward_max_dd_worst: stats
            .get("forward_max_dd_worst")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        wfe_pnl_median: stats
            .get("window_pnl_efficiency_median")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        wfe_pnl_worst: stats
            .get("window_pnl_efficiency_worst")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        wfe_sharpe_median: stats
            .get("window_sharpe_efficiency_median")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        wfe_sharpe_worst: stats
            .get("window_sharpe_efficiency_worst")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        wfe_sharpe_std: stats
            .get("window_sharpe_efficiency_std")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
    })
}

fn parse_cscv_summary(validation_res: &Value) -> Option<ExportCscvSummary> {
    let cscv = validation_res.get("cscv_pbo")?.as_object()?;
    Some(ExportCscvSummary {
        slices: cscv.get("slices").and_then(Value::as_u64).unwrap_or(0) as usize,
        slice_bars: cscv.get("slice_bars").and_then(Value::as_u64).unwrap_or(0) as usize,
        candidates: cscv.get("candidates").and_then(Value::as_u64).unwrap_or(0) as usize,
        combinations: cscv
            .get("combinations")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        pbo: cscv
            .get("pbo")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        threshold: cscv
            .get("threshold")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        prob_oos_below_threshold: cscv
            .get("prob_oos_below_threshold")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        skipped_reason: normalize_text(cscv.get("skipped_reason").and_then(Value::as_str)),
    })
}

fn parse_cross_summary(validation_res: &Value) -> Option<ExportCrossSummary> {
    let cross = validation_res.get("cross_ticker")?.as_object()?;
    let datasets = cross.get("datasets")?.as_array()?;
    let labels = datasets
        .iter()
        .filter_map(|entry| entry.as_object())
        .filter_map(|entry| {
            let label = normalize_text(entry.get("label").and_then(Value::as_str));
            let path = normalize_text(entry.get("csv_path").and_then(Value::as_str));
            if !label.is_empty() {
                Some(label)
            } else if !path.is_empty() {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    Some(ExportCrossSummary {
        dataset_count: datasets.len(),
        mode: normalize_text(cross.get("mode").and_then(Value::as_str)),
        dataset_labels: labels,
    })
}

fn is_double_like_strategy(strategy_kind: &str) -> bool {
    matches!(
        strategy_kind.trim(),
        "DoubleMa"
            | "Pit"
            | "Set"
            | "Mts"
            | "Orc"
            | "Marrt"
            | "Vfct"
            | "CompositeTrend"
            | "RegimeHybrid"
            | "BreakoutMomentum"
    )
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PlateauGroupKey {
    profile_kind: String,
    fast_ma_type: String,
    slow_ma_type: String,
    fast_ma_params: String,
    slow_ma_params: String,
}

#[derive(Clone, Debug)]
struct PlateauSurfacePoint {
    fast_len: u64,
    slow_len: u64,
    sharpe: f64,
    max_dd: f64,
}

#[derive(Clone, Debug, Default)]
struct PlateauGroupSurface {
    fast_periods: Vec<u64>,
    slow_periods: Vec<u64>,
    points: Vec<PlateauSurfacePoint>,
}

#[derive(Clone, Debug)]
struct PlateauStats {
    score: f64,
    label: String,
    source: String,
    neighbor_count: usize,
    sharpe_median: f64,
    max_dd_worst: f64,
}

fn plateau_group_from_params(
    params: &Map<String, Value>,
    profile_kind_hint: Option<&str>,
) -> Option<(PlateauGroupKey, u64, u64)> {
    let fast_len = params.get("fast_len").and_then(Value::as_u64)?;
    let slow_len = params.get("slow_len").and_then(Value::as_u64)?;
    let fast_ma_type = normalize_text(params.get("fast_ma_type").and_then(Value::as_str));
    let slow_ma_type = normalize_text(params.get("slow_ma_type").and_then(Value::as_str));
    if fast_ma_type.is_empty() || slow_ma_type.is_empty() {
        return None;
    }
    let profile_kind = profile_kind_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| active_profile_kind(params));
    Some((
        PlateauGroupKey {
            profile_kind,
            fast_ma_type,
            slow_ma_type,
            fast_ma_params: serde_json::to_string(
                params.get("fast_ma_params").unwrap_or(&Value::Null),
            )
            .unwrap_or_else(|_| "null".to_string()),
            slow_ma_params: serde_json::to_string(
                params.get("slow_ma_params").unwrap_or(&Value::Null),
            )
            .unwrap_or_else(|_| "null".to_string()),
        },
        fast_len,
        slow_len,
    ))
}

fn parse_plateau_surface_points(
    report: &Value,
) -> (Vec<(PlateauGroupKey, PlateauSurfacePoint)>, String) {
    let source = report
        .pointer("/run/res/all")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .map(|rows| (rows, "full".to_string()))
        .or_else(|| {
            report
                .pointer("/optimization/res/all")
                .and_then(Value::as_array)
                .filter(|rows| !rows.is_empty())
                .map(|rows| (rows, "full".to_string()))
        })
        .or_else(|| {
            report
                .pointer("/run/res/top")
                .and_then(Value::as_array)
                .map(|rows| (rows, "top".to_string()))
        })
        .or_else(|| {
            report
                .pointer("/optimization/res/top")
                .and_then(Value::as_array)
                .map(|rows| (rows, "top".to_string()))
        });
    let Some(entries) = source else {
        return (Vec::new(), String::new());
    };
    let (entries, source_label) = entries;

    (
        entries
            .iter()
            .filter_map(|entry| {
                let tuple = entry.as_array()?;
                let params = tuple.first()?.as_object()?;
                let metrics = tuple.get(1)?.as_object()?;
                let (group, fast_len, slow_len) = plateau_group_from_params(params, None)?;
                let sharpe = metrics.get("sharpe").and_then(Value::as_f64)?;
                let max_dd = metrics.get("max_dd").and_then(Value::as_f64)?;
                if !sharpe.is_finite() || !max_dd.is_finite() {
                    return None;
                }
                Some((
                    group,
                    PlateauSurfacePoint {
                        fast_len,
                        slow_len,
                        sharpe,
                        max_dd,
                    },
                ))
            })
            .collect(),
        source_label,
    )
}

fn build_plateau_surfaces(
    report: &Value,
) -> (
    std::collections::BTreeMap<PlateauGroupKey, PlateauGroupSurface>,
    String,
) {
    use std::collections::{BTreeMap, BTreeSet};

    let mut grouped_points: BTreeMap<PlateauGroupKey, Vec<PlateauSurfacePoint>> = BTreeMap::new();
    let (surface_points, source_label) = parse_plateau_surface_points(report);
    for (group, point) in surface_points {
        grouped_points.entry(group).or_default().push(point);
    }

    let mut surfaces = BTreeMap::new();
    for (group, points) in grouped_points {
        let mut fast_periods: BTreeSet<u64> = BTreeSet::new();
        let mut slow_periods: BTreeSet<u64> = BTreeSet::new();
        for point in &points {
            fast_periods.insert(point.fast_len);
            slow_periods.insert(point.slow_len);
        }
        surfaces.insert(
            group,
            PlateauGroupSurface {
                fast_periods: fast_periods.into_iter().collect(),
                slow_periods: slow_periods.into_iter().collect(),
                points,
            },
        );
    }
    (surfaces, source_label)
}

fn median_f64(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

fn efficiency_ratio(oos_value: Option<f64>, in_sample_value: f64) -> Option<f64> {
    let oos = oos_value?;
    if !oos.is_finite() || !in_sample_value.is_finite() || in_sample_value <= 0.0 {
        return None;
    }
    Some(oos / in_sample_value)
}

fn label_plateau(score: f64, neighbor_count: usize) -> String {
    if neighbor_count >= 5 && score >= 0.78 {
        "Strong".to_string()
    } else if neighbor_count >= 3 && score >= 0.58 {
        "Moderate".to_string()
    } else if score >= 0.38 {
        "Narrow".to_string()
    } else {
        "Fragile".to_string()
    }
}

fn plateau_stats_for_row(
    row: &ExportCandidateRow,
    surfaces: &std::collections::BTreeMap<PlateauGroupKey, PlateauGroupSurface>,
    source_label: &str,
) -> Option<PlateauStats> {
    let params_value: Value = serde_json::from_str(&row.params_json).ok()?;
    let params = params_value.as_object()?;
    let (group, fast_len, slow_len) = plateau_group_from_params(params, Some(&row.profile_kind))?;
    let surface = surfaces.get(&group)?;
    if surface.points.len() < 3 {
        return None;
    }

    let fast_idx = surface
        .fast_periods
        .iter()
        .position(|value| *value == fast_len)?;
    let slow_idx = surface
        .slow_periods
        .iter()
        .position(|value| *value == slow_len)?;
    let fast_lo = fast_idx.saturating_sub(1);
    let fast_hi = (fast_idx + 1).min(surface.fast_periods.len().saturating_sub(1));
    let slow_lo = slow_idx.saturating_sub(1);
    let slow_hi = (slow_idx + 1).min(surface.slow_periods.len().saturating_sub(1));
    let fast_window = &surface.fast_periods[fast_lo..=fast_hi];
    let slow_window = &surface.slow_periods[slow_lo..=slow_hi];

    let neighbors: Vec<&PlateauSurfacePoint> = surface
        .points
        .iter()
        .filter(|point| {
            fast_window.contains(&point.fast_len) && slow_window.contains(&point.slow_len)
        })
        .collect();
    if neighbors.len() < 2 {
        return None;
    }

    let mut sharpe_values: Vec<f64> = neighbors.iter().map(|point| point.sharpe).collect();
    let sharpe_median = median_f64(&mut sharpe_values)?;
    let max_dd_worst = neighbors
        .iter()
        .map(|point| point.max_dd)
        .fold(f64::NEG_INFINITY, f64::max);
    if !max_dd_worst.is_finite() {
        return None;
    }

    let support_strength = (neighbors.len().min(6) as f64) / 6.0;
    let sharpe_stability = if row.sharpe.abs() > 1e-9 {
        (sharpe_median / row.sharpe).clamp(0.0, 1.0)
    } else if sharpe_median > 0.0 {
        1.0
    } else {
        0.0
    };
    let dd_stability = if max_dd_worst.abs() <= 1e-12 {
        1.0
    } else {
        (row.max_dd / max_dd_worst).clamp(0.0, 1.0)
    };
    let score = 0.45 * support_strength + 0.35 * sharpe_stability + 0.20 * dd_stability;

    Some(PlateauStats {
        score,
        label: label_plateau(score, neighbors.len()),
        source: source_label.to_string(),
        neighbor_count: neighbors.len(),
        sharpe_median,
        max_dd_worst,
    })
}

fn apply_plateau_analysis(model: &mut ValidationExportModel, report: &Value) {
    if !is_double_like_strategy(&model.strategy_kind) {
        return;
    }
    let (surfaces, source_label) = build_plateau_surfaces(report);
    if surfaces.is_empty() {
        return;
    }

    let mut changed = false;
    for row in &mut model.all_rows {
        if let Some(stats) = plateau_stats_for_row(row, &surfaces, &source_label) {
            row.plateau_score = Some(stats.score);
            row.plateau_label = stats.label;
            row.plateau_source = stats.source;
            row.plateau_neighbor_count = Some(stats.neighbor_count);
            row.plateau_sharpe_median = Some(stats.sharpe_median);
            row.plateau_max_dd_worst = Some(stats.max_dd_worst);
            changed = true;
        }
    }
    if changed {
        model.approved_rows = model
            .all_rows
            .iter()
            .filter(|row| row.final_approved)
            .cloned()
            .collect();
        model.shortlist_rows = shortlist_rows(&model.all_rows, model.visible_cap);
    }
}

fn rank_map_from_rows<'a, F>(
    rows: &'a [ExportCandidateRow],
    mut include_and_sort: F,
) -> std::collections::BTreeMap<usize, usize>
where
    F: FnMut(&&'a ExportCandidateRow, &&'a ExportCandidateRow) -> std::cmp::Ordering,
{
    use std::collections::BTreeMap;

    let mut ordered: Vec<&ExportCandidateRow> = rows.iter().collect();
    ordered.sort_by(|a, b| include_and_sort(a, b));
    let mut out = BTreeMap::new();
    for (idx, row) in ordered.into_iter().enumerate() {
        out.insert(row.rank, idx + 1);
    }
    out
}

fn summarize_overlap_and_drift(
    finalist_ids: &[usize],
    robust_ranks: &std::collections::BTreeMap<usize, usize>,
    view_ranks: &std::collections::BTreeMap<usize, usize>,
    finalist_k: usize,
) -> Option<(usize, f64)> {
    if view_ranks.is_empty() || finalist_ids.is_empty() {
        return None;
    }

    let overlap = finalist_ids
        .iter()
        .filter(|id| view_ranks.get(id).is_some_and(|rank| *rank <= finalist_k))
        .count();
    let drifts: Vec<f64> = finalist_ids
        .iter()
        .filter_map(|id| {
            let robust_rank = *robust_ranks.get(id)? as i64;
            let view_rank = *view_ranks.get(id)? as i64;
            Some((robust_rank - view_rank).unsigned_abs() as f64)
        })
        .collect();
    if drifts.is_empty() {
        None
    } else {
        Some((overlap, drifts.iter().sum::<f64>() / drifts.len() as f64))
    }
}

fn apply_ranking_stability(model: &mut ValidationExportModel) {
    use std::collections::BTreeMap;

    if !is_double_like_strategy(&model.strategy_kind) || model.all_rows.is_empty() {
        return;
    }

    let robust_ranks = rank_map_from_rows(&model.all_rows, |a, b| {
        b.robust_score
            .total_cmp(&a.robust_score)
            .then_with(|| a.rank.cmp(&b.rank))
    });

    let wf_ranks: BTreeMap<usize, usize> = {
        let mut ordered: Vec<&ExportCandidateRow> = model
            .all_rows
            .iter()
            .filter(|row| row.wf_sharpe_median.is_some())
            .collect();
        ordered.sort_by(|a, b| {
            b.wf_sharpe_median
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&a.wf_sharpe_median.unwrap_or(f64::NEG_INFINITY))
                .then_with(|| a.rank.cmp(&b.rank))
        });
        ordered
            .into_iter()
            .enumerate()
            .map(|(idx, row)| (row.rank, idx + 1))
            .collect()
    };

    let cscv_ranks: BTreeMap<usize, usize> = {
        let mut ordered: Vec<&ExportCandidateRow> = model
            .all_rows
            .iter()
            .filter(|row| {
                row.cscv_oos_rank_frac_p50.is_some()
                    || row.cscv_selected_frac.is_some()
                    || row.cscv_oos_metric_p50.is_some()
            })
            .collect();
        ordered.sort_by(|a, b| {
            b.cscv_oos_rank_frac_p50
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&a.cscv_oos_rank_frac_p50.unwrap_or(f64::NEG_INFINITY))
                .then_with(|| {
                    b.cscv_selected_frac
                        .unwrap_or(f64::NEG_INFINITY)
                        .total_cmp(&a.cscv_selected_frac.unwrap_or(f64::NEG_INFINITY))
                })
                .then_with(|| {
                    b.cscv_oos_metric_p50
                        .unwrap_or(f64::NEG_INFINITY)
                        .total_cmp(&a.cscv_oos_metric_p50.unwrap_or(f64::NEG_INFINITY))
                })
                .then_with(|| a.rank.cmp(&b.rank))
        });
        ordered
            .into_iter()
            .enumerate()
            .map(|(idx, row)| (row.rank, idx + 1))
            .collect()
    };

    let cross_ranks: BTreeMap<usize, usize> = {
        let mut ordered: Vec<&ExportCandidateRow> = model
            .all_rows
            .iter()
            .filter(|row| row.cross_score.is_some())
            .collect();
        ordered.sort_by(|a, b| {
            b.cross_score
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&a.cross_score.unwrap_or(f64::NEG_INFINITY))
                .then_with(|| a.rank.cmp(&b.rank))
        });
        ordered
            .into_iter()
            .enumerate()
            .map(|(idx, row)| (row.rank, idx + 1))
            .collect()
    };

    let finalist_k = model.all_rows.len().min(3);
    let finalist_ids: Vec<usize> = model
        .all_rows
        .iter()
        .filter_map(|row| {
            robust_ranks
                .get(&row.rank)
                .filter(|rank| **rank <= finalist_k)
                .map(|_| row.rank)
        })
        .collect();

    for row in &mut model.all_rows {
        row.robust_view_rank = robust_ranks.get(&row.rank).copied();
        row.wf_view_rank = wf_ranks.get(&row.rank).copied();
        row.cscv_view_rank = cscv_ranks.get(&row.rank).copied();
        row.cross_view_rank = cross_ranks.get(&row.rank).copied();
        let robust_rank = row.robust_view_rank.unwrap_or(0) as i64;
        let mut drifts: Vec<usize> = Vec::new();
        for view_rank in [row.wf_view_rank, row.cscv_view_rank, row.cross_view_rank]
            .into_iter()
            .flatten()
        {
            drifts.push((robust_rank - view_rank as i64).unsigned_abs() as usize);
        }
        row.max_view_rank_drift = drifts.into_iter().max();
    }

    let wf_summary =
        summarize_overlap_and_drift(&finalist_ids, &robust_ranks, &wf_ranks, finalist_k);
    let cscv_summary =
        summarize_overlap_and_drift(&finalist_ids, &robust_ranks, &cscv_ranks, finalist_k);
    let cross_summary =
        summarize_overlap_and_drift(&finalist_ids, &robust_ranks, &cross_ranks, finalist_k);

    let stable_finalist_count = finalist_ids
        .iter()
        .filter(|id| {
            model
                .all_rows
                .iter()
                .find(|row| row.rank == **id)
                .and_then(|row| row.max_view_rank_drift)
                .is_some_and(|drift| drift <= 1)
        })
        .count();

    model.ranking_stability = Some(ExportRankingStabilitySummary {
        finalist_k,
        stable_finalist_count,
        wf_overlap: wf_summary.map(|value| value.0),
        wf_mean_drift: wf_summary.map(|value| value.1),
        cscv_overlap: cscv_summary.map(|value| value.0),
        cscv_mean_drift: cscv_summary.map(|value| value.1),
        cross_overlap: cross_summary.map(|value| value.0),
        cross_mean_drift: cross_summary.map(|value| value.1),
    });

    model.approved_rows = model
        .all_rows
        .iter()
        .filter(|row| row.final_approved)
        .cloned()
        .collect();
    model.shortlist_rows = shortlist_rows(&model.all_rows, model.visible_cap);
}

fn describe_enabled_modules(validation_req: &Value) -> Vec<&'static str> {
    let mut enabled = vec!["base reruns", "hard gates"];
    if validation_req
        .get("walk_forward")
        .is_some_and(|v| !v.is_null())
    {
        enabled.push("walk-forward");
    }
    if validation_req.get("cscv_pbo").is_some_and(|v| !v.is_null()) {
        enabled.push("CSCV/PBO");
    }
    if validation_req.get("spa").is_some_and(|v| !v.is_null()) {
        enabled.push("SPA");
    }
    if validation_req
        .get("reopt_walk_forward")
        .is_some_and(|v| !v.is_null())
    {
        enabled.push("reopt WF");
    }
    enabled
}

fn describe_aux_cpu_tails(validation_req: &Value) -> Vec<String> {
    let mut aux_cpu = Vec::new();
    if validation_req.get("perturb").is_some_and(|v| !v.is_null()) {
        aux_cpu.push("perturb".to_string());
    }
    if let Some(bootstrap) = validation_req
        .get("trade_bootstrap")
        .filter(|v| !v.is_null())
    {
        if bootstrap
            .get("stationary_mean_block_len")
            .and_then(Value::as_u64)
            == Some(0)
        {
            aux_cpu.push("trade bootstrap iid-only".to_string());
            return aux_cpu;
        }
        let stat_block = bootstrap
            .get("stationary_mean_block_len")
            .and_then(Value::as_u64)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "auto".to_string());
        aux_cpu.push(format!(
            "trade bootstrap compare (iid + stationary, stat block={stat_block})"
        ));
    }
    aux_cpu
}

fn build_expected_execution_summary(
    strategy_kind: &str,
    validation_req: Option<&Value>,
) -> (String, String, String, String) {
    let Some(validation_req) = validation_req else {
        return (
            "validation request unavailable".to_string(),
            "not available".to_string(),
            "-".to_string(),
            "Artifact does not include validation.req; expected execution preview unavailable."
                .to_string(),
        );
    };
    let backend = validation_req
        .pointer("/backend/backend")
        .and_then(Value::as_str)
        .unwrap_or("CpuOnly")
        .trim();
    let enabled_main = describe_enabled_modules(validation_req);
    let aux_cpu = describe_aux_cpu_tails(validation_req);

    let mut route_summary = format!("{backend} requested");
    let module_summary: String;
    let mut warning_summary = "none".to_string();
    let is_double_like = is_double_like_strategy(strategy_kind);

    if is_double_like {
        match backend {
            "CpuOnly" => {
                route_summary = "CpuOnly requested | CPU validation route expected".to_string();
                module_summary = format!(
                    "CPU route: {}{}",
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else {
                        format!(" + {}", aux_cpu.join(" + "))
                    }
                );
            }
            "GpuOnly" => {
                route_summary =
                    "GpuOnly requested | expected CpuOnly fallback for fixed-candidate validation"
                        .to_string();
                module_summary = format!(
                    "Expected CPU fallback route: {}{}",
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else {
                        format!(" + {}", aux_cpu.join(" + "))
                    }
                );
                warning_summary = "GpuOnly is not supported for fixed-candidate Double-MA validation; use GpuKernel, GpuHybridKernel, or Auto.".to_string();
            }
            "GpuHybridKernel" => {
                route_summary =
                    "GpuHybridKernel requested | expected GPU main route (GpuKernel-compatible)"
                        .to_string();
                module_summary = format!(
                    "GPU main route: {}{}",
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else {
                        format!(" | auxiliary CPU tails: {}", aux_cpu.join(" + "))
                    }
                );
            }
            "GpuKernel" => {
                route_summary = "GpuKernel requested | expected GPU main route".to_string();
                module_summary = format!(
                    "GPU main route: {}{}",
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else {
                        format!(" | auxiliary CPU tails: {}", aux_cpu.join(" + "))
                    }
                );
            }
            _ => {
                route_summary =
                    "Auto requested | expected GPU main route when available".to_string();
                module_summary = format!(
                    "Expected GPU main route: {}{}",
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else {
                        format!(" | auxiliary CPU tails: {}", aux_cpu.join(" + "))
                    }
                );
            }
        }

        if warning_summary == "none"
            && !aux_cpu.is_empty()
            && backend != "CpuOnly"
            && backend != "GpuOnly"
        {
            warning_summary = format!(
                "Main GPU route is preserved; {} currently run as auxiliary CPU work.",
                aux_cpu.join(" + ")
            );
        }
    } else {
        module_summary = format!(
            "Enabled modules: {}{}",
            enabled_main.join(" + "),
            if aux_cpu.is_empty() {
                String::new()
            } else {
                format!(" + {}", aux_cpu.join(" + "))
            }
        );
        if !aux_cpu.is_empty() && backend != "CpuOnly" {
            warning_summary = format!(
                "{} may not share the same execution route as the main validation pass; confirm post-run route notes.",
                aux_cpu.join(" + ")
            );
        }
    }

    let cross_enabled = validation_req
        .get("cross_ticker")
        .and_then(Value::as_object)
        .is_some();
    let mut cross_summary = "not run".to_string();
    if cross_enabled {
        let datasets = validation_req
            .pointer("/cross_ticker/datasets")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        let mode = validation_req
            .pointer("/cross_ticker/mode")
            .and_then(Value::as_str)
            .unwrap_or("default")
            .trim();
        let base = format!(
            "{} dataset{} | {}",
            datasets,
            if datasets == 1 { "" } else { "s" },
            mode
        );
        if is_double_like && mode == "preserved_finalists" {
            cross_summary = format!("{base} | finalists only");
            let cross_warning = "Preserved-finalists cross reruns keep CSCV/PBO + hard gates; auxiliary cross reruns omit walk-forward, SPA, reopt WF, perturb, and trade bootstrap.";
            warning_summary = if warning_summary == "none" {
                cross_warning.to_string()
            } else {
                format!("{warning_summary} | {cross_warning}")
            };
        } else if is_double_like && mode == "lightweight" {
            cross_summary = format!("{base} | lightweight reruns");
        } else {
            cross_summary = base;
        }
    }

    (
        route_summary,
        cross_summary,
        module_summary,
        if warning_summary.is_empty() {
            "none".to_string()
        } else {
            warning_summary
        },
    )
}

fn resolve_strategy_kind(report: &Value) -> String {
    for ptr in [
        "/run/req/strategy_kind",
        "/validation/req/validation_context/strategy_kind",
        "/validation/req/strategy_kind",
    ] {
        if let Some(kind) = report.pointer(ptr).and_then(Value::as_str) {
            let trimmed = kind.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    "Unknown".to_string()
}

fn sort_rows(rows: &mut [ExportCandidateRow]) {
    rows.sort_by(|a, b| {
        b.final_approved
            .cmp(&a.final_approved)
            .then_with(|| {
                b.hard_gate_passed
                    .unwrap_or(false)
                    .cmp(&a.hard_gate_passed.unwrap_or(false))
            })
            .then_with(|| b.robust_score.total_cmp(&a.robust_score))
            .then_with(|| {
                b.wf_sharpe_efficiency
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&a.wf_sharpe_efficiency.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| {
                b.wf_pnl_efficiency
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&a.wf_pnl_efficiency.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| {
                b.plateau_score
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&a.plateau_score.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| {
                b.cscv_oos_rank_frac_p50
                    .unwrap_or(0.0)
                    .total_cmp(&a.cscv_oos_rank_frac_p50.unwrap_or(0.0))
            })
            .then_with(|| {
                b.cscv_selected_frac
                    .unwrap_or(0.0)
                    .total_cmp(&a.cscv_selected_frac.unwrap_or(0.0))
            })
            .then_with(|| {
                b.cscv_oos_metric_p50
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&a.cscv_oos_metric_p50.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| a.params_summary.cmp(&b.params_summary))
    });
    for (idx, row) in rows.iter_mut().enumerate() {
        row.rank = idx + 1;
    }
}

fn build_rows(validation_res: &Value) -> Result<(Vec<ExportCandidateRow>, bool), String> {
    let candidates = validation_res
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing validation.res.candidates".to_string())?;
    let candidate_refs: Vec<&Value> = candidates.iter().collect();
    let legacy_shortlist = legacy_shortlist_flags(&candidate_refs);
    let has_candidate_verdicts = candidates.iter().any(|cand| {
        cand.get("validation_verdict")
            .and_then(Value::as_object)
            .and_then(|v| v.get("passed"))
            .and_then(Value::as_bool)
            .is_some()
    });
    let has_run_verdict = validation_res
        .get("run_verdict")
        .and_then(Value::as_object)
        .is_some();
    let compatibility_mode = !(has_candidate_verdicts && has_run_verdict);

    let mut rows: Vec<ExportCandidateRow> = candidates
        .iter()
        .enumerate()
        .map(|(idx, cand)| {
            let params_obj = cand.get("params").and_then(Value::as_object);
            let params_json = serde_json::to_string(cand.get("params").unwrap_or(&Value::Null))
                .unwrap_or_else(|_| "{}".to_string());
            let profile_kind = params_obj
                .map(active_profile_kind)
                .unwrap_or_else(|| "GENERIC".to_string());
            let params_summary = params_obj
                .map(params_summary)
                .unwrap_or_else(|| "params".to_string());
            let verdict = cand.get("validation_verdict").and_then(Value::as_object);
            let hard_gate_passed = verdict
                .and_then(|value| value.get("passed"))
                .and_then(Value::as_bool);
            let warning_flags =
                join_string_array(verdict.and_then(|value| value.get("warning_flags")));
            let fail_reasons =
                join_string_array(verdict.and_then(|value| value.get("fail_reasons")));
            let pnl = opt_f64(cand, "/in_sample/pnl").unwrap_or(0.0);
            let sharpe = opt_f64(cand, "/in_sample/sharpe").unwrap_or(0.0);
            let wf_sharpe_median = opt_f64(cand, "/walk_forward/sharpe_median");
            let wf_pnl_median = opt_f64(cand, "/walk_forward/pnl_median");
            let regime_bucket_rows = parse_regime_bucket_rows(cand);
            let regime_stable_bucket_count = opt_u64(
                cand,
                "/regime_slice_robustness/stable_bucket_count",
            )
            .map(|v| v as usize);
            let regime_worst_sharpe = opt_f64(cand, "/regime_slice_robustness/worst_sharpe");
            let regime_worst_max_dd = opt_f64(cand, "/regime_slice_robustness/worst_max_dd");
            let trend_stable = regime_bucket_rows
                .iter()
                .find(|row| row.label == "trend")
                .and_then(|row| row.stable);
            let range_stable = regime_bucket_rows
                .iter()
                .find(|row| row.label == "range")
                .and_then(|row| row.stable);
            let track_record_bars = opt_u64(cand, "/track_record_bars").map(|v| v as usize);
            let min_track_record_length_bars =
                opt_u64(cand, "/min_track_record_length_bars").map(|v| v as usize);
            let approved = if hard_gate_passed.is_some() {
                final_approved(
                    hard_gate_passed,
                    track_record_bars,
                    min_track_record_length_bars,
                )
            } else {
                legacy_shortlist.get(idx).copied().unwrap_or(false)
            };
            let legacy_shortlisted = legacy_shortlist.get(idx).copied().unwrap_or(false);
            let (approval_label, approval_reason) = approval_label_and_reason(
                compatibility_mode,
                approved,
                hard_gate_passed,
                legacy_shortlisted,
                track_record_bars,
                min_track_record_length_bars,
                &fail_reasons,
                &warning_flags,
            );
            let regime_bucket_rows = regime_bucket_rows
                .into_iter()
                .map(|mut row| {
                    row.rank = idx + 1;
                    row.approval_label = approval_label.clone();
                    row.params_summary = params_summary.clone();
                    row
                })
                .collect();
            ExportCandidateRow {
                rank: idx + 1,
                final_approved: approved,
                hard_gate_passed,
                legacy_shortlisted,
                approval_label,
                approval_reason,
                profile_kind,
                params_summary,
                params_json,
                robust_score: opt_f64(cand, "/robust_score").unwrap_or(0.0),
                robust_pareto: cand
                    .get("robust_pareto")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                psr: opt_f64(cand, "/psr"),
                dsr: opt_f64(cand, "/dsr"),
                track_record_bars,
                min_track_record_length_bars,
                pnl,
                sharpe,
                max_dd: opt_f64(cand, "/in_sample/max_dd").unwrap_or(0.0),
                trades: opt_f64(cand, "/in_sample/trades").unwrap_or(0.0),
                exposure: opt_f64(cand, "/in_sample/exposure"),
                wf_sharpe_median,
                wf_dd_worst: opt_f64(cand, "/walk_forward/max_dd_worst"),
                wf_pnl_efficiency: efficiency_ratio(wf_pnl_median, pnl),
                wf_sharpe_efficiency: efficiency_ratio(wf_sharpe_median, sharpe),
                cross_score: opt_f64(cand, "/cross_ticker/score"),
                cross_pareto: cand
                    .get("cross_ticker_pareto")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                cscv_selected_frac: opt_f64(cand, "/cscv_pbo/is_selected_frac"),
                cscv_oos_rank_frac_p50: opt_f64(cand, "/cscv_pbo/oos_rank_frac_p50"),
                cscv_oos_metric_p50: opt_f64(cand, "/cscv_pbo/oos_metric_p50"),
                bootstrap_prob_pnl_positive: opt_f64(cand, "/trade_bootstrap/prob_pnl_positive"),
                bootstrap_pnl_p05: opt_f64(cand, "/trade_bootstrap/pnl_p05"),
                bootstrap_pnl_p50: opt_f64(cand, "/trade_bootstrap/pnl_p50"),
                bootstrap_dd_p95: opt_f64(cand, "/trade_bootstrap/max_dd_p95"),
                bootstrap_stationary_mean_block_len: opt_u64(
                    cand,
                    "/trade_bootstrap/stationary_mean_block_len",
                )
                .map(|v| v as usize),
                bootstrap_stationary_pnl_p05: opt_f64(cand, "/trade_bootstrap/stationary_pnl_p05"),
                bootstrap_stationary_dd_p95: opt_f64(
                    cand,
                    "/trade_bootstrap/stationary_max_dd_p95",
                ),
                perturb_pnl_worst: opt_f64(cand, "/perturb/pnl_worst"),
                perturb_pnl_mean: opt_f64(cand, "/perturb/pnl_mean"),
                perturb_positive_frac: opt_f64(cand, "/perturb/pnl_positive_frac"),
                plateau_score: None,
                plateau_label: String::new(),
                plateau_source: String::new(),
                plateau_neighbor_count: None,
                plateau_sharpe_median: None,
                plateau_max_dd_worst: None,
                regime_worst_bucket: normalize_text(
                    cand.pointer("/regime_slice_robustness/worst_bucket_label")
                        .and_then(Value::as_str),
                ),
                regime_worst_sharpe,
                regime_worst_max_dd,
                regime_stable_bucket_count,
                regime_stable_bucket_frac: opt_f64(
                    cand,
                    "/regime_slice_robustness/stable_bucket_frac",
                ),
                regime_fragility_flag: derive_regime_fragility_flag(
                    regime_stable_bucket_count,
                    regime_worst_sharpe,
                    regime_worst_max_dd,
                ),
                regime_support_label: {
                    let from_artifact = normalize_regime_support_label(
                        cand.pointer("/regime_slice_robustness/support_label")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                    );
                    if from_artifact.is_empty() {
                        derive_regime_support_label(
                            trend_stable,
                            range_stable,
                            regime_stable_bucket_count,
                        )
                    } else {
                        from_artifact
                    }
                },
                regime_bucket_rows,
                friction_flag: derive_friction_flag(
                    opt_f64(cand, "/friction_stress/pass_frac"),
                    opt_u64(cand, "/friction_stress/max_rank_drift").map(|v| v as usize),
                ),
                friction_worst_scenario: normalize_text(
                    cand.pointer("/friction_stress/worst_scenario_label")
                        .and_then(Value::as_str),
                ),
                friction_worst_sharpe: opt_f64(cand, "/friction_stress/worst_sharpe"),
                friction_worst_max_dd: opt_f64(cand, "/friction_stress/worst_max_dd"),
                friction_pass_frac: opt_f64(cand, "/friction_stress/pass_frac"),
                friction_max_rank_drift: opt_u64(cand, "/friction_stress/max_rank_drift")
                    .map(|v| v as usize),
                robust_view_rank: None,
                wf_view_rank: None,
                cscv_view_rank: None,
                cross_view_rank: None,
                max_view_rank_drift: None,
                warning_flags,
                fail_reasons,
            }
        })
        .collect();

    sort_rows(&mut rows);
    Ok((rows, compatibility_mode))
}

pub(super) fn run_level_rejection_reason(run_verdict: &ExportRunVerdict) -> String {
    if !run_verdict.fail_reasons.is_empty() {
        format!("run verdict: {}", run_verdict.fail_reasons)
    } else if !run_verdict.warning_flags.is_empty() {
        format!("run verdict: {}", run_verdict.warning_flags)
    } else if !run_verdict.final_recommendation.is_empty() {
        format!("run verdict: {}", run_verdict.final_recommendation)
    } else {
        "run verdict rejected final approval".to_string()
    }
}

fn shortlist_rows(rows: &[ExportCandidateRow], visible_cap: usize) -> Vec<ExportCandidateRow> {
    use std::collections::HashSet;

    let mut out: Vec<ExportCandidateRow> = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();

    let mut extend_group = |iter: &mut dyn Iterator<Item = &ExportCandidateRow>| {
        for row in iter {
            if out.len() >= visible_cap {
                break;
            }
            if seen.insert(row.rank) {
                out.push(row.clone());
            }
        }
    };

    extend_group(&mut rows.iter().filter(|row| row.final_approved));
    extend_group(
        &mut rows
            .iter()
            .filter(|row| !row.final_approved && row.hard_gate_passed == Some(true)),
    );
    extend_group(
        &mut rows
            .iter()
            .filter(|row| !row.final_approved && row.legacy_shortlisted),
    );
    extend_group(&mut rows.iter());

    out
}

pub(super) fn build_model_from_report(
    report: &Value,
    max_rows: Option<usize>,
) -> Result<ValidationExportModel, String> {
    let validation_res = report
        .pointer("/validation/res")
        .ok_or_else(|| "missing report.validation.res".to_string())?;
    let ts = normalize_text(report.get("ts").and_then(Value::as_str));
    let strategy_kind = resolve_strategy_kind(report);
    let mut model =
        build_model_from_validation_res_with_context(validation_res, max_rows, ts, strategy_kind)?;
    apply_plateau_analysis(&mut model, report);
    if let Some(validation_req) = report.pointer("/validation/req") {
        model.min_psr = opt_f64(validation_req, "/min_psr");
        model.min_dsr = opt_f64(validation_req, "/min_dsr");
        model.min_cscv_oos_rank_frac = opt_f64(validation_req, "/min_cscv_oos_rank_frac");
        model.min_cscv_oos_metric = opt_f64(validation_req, "/min_cscv_oos_metric");
        model.max_pbo = opt_f64(validation_req, "/max_pbo");
        let aux_cpu = describe_aux_cpu_tails(validation_req);
        if model.backend_used.eq_ignore_ascii_case("gpu")
            && !model.backend_route.contains("->CpuOnly")
            && !aux_cpu.is_empty()
        {
            model.actual_aux_tail_summary = aux_cpu.join(" + ");
        }
        let (route, cross, modules, warnings) =
            build_expected_execution_summary(&model.strategy_kind, Some(validation_req));
        model.expected_route_summary = route;
        model.expected_cross_summary = cross;
        model.expected_module_summary = modules;
        model.expected_warning_summary = warnings;
        model.bootstrap_mode_summary = build_bootstrap_mode_summary(Some(validation_req));
        model.wfe_policy_summary =
            "explanatory + tie-break ordering only; no approval gate".to_string();
        model.friction_policy_summary =
            "finalist-only fee/spread/slippage stress; absolute pass uses pnl >= 0 and sharpe >= 0"
                .to_string();
    }
    Ok(model)
}

pub(super) fn build_model_from_validation_res(
    validation_res: &Value,
    max_rows: Option<usize>,
) -> Result<ValidationExportModel, String> {
    build_model_from_validation_res_with_context(
        validation_res,
        max_rows,
        String::new(),
        "Unknown".to_string(),
    )
}

fn build_model_from_validation_res_with_context(
    validation_res: &Value,
    max_rows: Option<usize>,
    ts: String,
    strategy_kind: String,
) -> Result<ValidationExportModel, String> {
    let data_id = normalize_text(validation_res.get("data_id").and_then(Value::as_str));
    let visible_cap = max_rows.unwrap_or(25).clamp(1, 4096);
    let (mut rows, compatibility_mode) = build_rows(validation_res)?;
    let run_verdict = parse_run_verdict(validation_res);
    if let Some(verdict) = run_verdict.as_ref().filter(|verdict| !verdict.approved) {
        let reason = run_level_rejection_reason(verdict);
        for row in rows.iter_mut().filter(|row| row.final_approved) {
            row.final_approved = false;
            row.approval_label = "not_approved".to_string();
            row.approval_reason = reason.clone();
        }
        sort_rows(&mut rows);
    }

    let approved_rows: Vec<ExportCandidateRow> = rows
        .iter()
        .filter(|row| row.final_approved)
        .cloned()
        .collect();
    let shortlist_rows = shortlist_rows(&rows, visible_cap);
    let mut model = ValidationExportModel {
        ts,
        data_id,
        strategy_kind,
        runtime_ms: validation_res.get("runtime_ms").and_then(Value::as_u64),
        backend_used: normalize_text(validation_res.get("backend_used").and_then(Value::as_str)),
        backend_route: normalize_text(validation_res.get("backend_route").and_then(Value::as_str)),
        backend_fallback_reason: normalize_text(
            validation_res
                .get("backend_fallback_reason")
                .and_then(Value::as_str),
        ),
        actual_aux_tail_summary: "-".to_string(),
        expected_route_summary: "validation request unavailable".to_string(),
        expected_cross_summary: "not available".to_string(),
        expected_module_summary: "-".to_string(),
        expected_warning_summary:
            "Artifact does not include validation.req; expected execution preview unavailable."
                .to_string(),
        bootstrap_mode_summary: "-".to_string(),
        wfe_policy_summary: "explanatory + tie-break ordering only; no approval gate".to_string(),
        friction_policy_summary:
            "finalist-only fee/spread/slippage stress; absolute pass uses pnl >= 0 and sharpe >= 0"
                .to_string(),
        compatibility_mode,
        visible_cap,
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        run_verdict,
        spa: parse_spa_summary(validation_res),
        reopt_walk_forward: parse_reopt_summary(validation_res),
        cscv_pbo: parse_cscv_summary(validation_res),
        cross_ticker: parse_cross_summary(validation_res),
        ranking_stability: None,
        all_rows: rows,
        approved_rows,
        shortlist_rows,
    };
    apply_ranking_stability(&mut model);
    Ok(model)
}
